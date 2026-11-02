//! Flat common-seed Monte Carlo. Enemy actions use the complete original sampler.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SampleResult {
    #[serde(default)]
    pub tail_rounds: usize,
    #[serde(default)]
    pub tail_hp_restored: i32,
    #[serde(default)]
    pub tail_stress_recovered: i32,
    pub score: f64,
    pub deaths: u32,
    #[serde(default)]
    pub weighted_deaths: f64,
    pub unrecovered: u32,
    pub wipe: bool,
    pub door_entries: u32,
    pub new_afflictions: u32,
    pub stress: i32,
    pub hp_fraction: f64,
    pub rounds: u32,
    pub retreated: bool,
    #[serde(default)]
    pub retreat_cost_kind: Option<String>,
    pub unfinished: bool,
    pub outcome: Option<Outcome>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateResult {
    pub action: Action,
    pub categories: Vec<String>,
    pub samples: usize,
    pub mean: f64,
    pub worst_tail: f64,
    pub value: f64,
    pub mean_deaths: f64,
    pub mean_stress: f64,
    pub mean_hp_fraction: f64,
    pub unfinished: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Decision {
    pub action: Action,
    pub candidates: Vec<CandidateResult>,
    pub simulations: usize,
    pub roundtrip: bool,
    pub estimates: BTreeMap<String, String>,
    pub retreat: retreat::Assessment,
}
fn weight(s: &Settings, k: &str) -> f64 {
    s.weights[k]
}
fn stun(e: &dd_data::combat::Effect) -> f64 {
    match &e.kind {
        EffectKind::Stun => e.chance,
        EffectKind::Composite(es) => es.iter().map(stun).fold(0., f64::max),
        _ => 0.,
    }
}
fn healing_kind(k: &EffectKind) -> bool {
    match k {
        EffectKind::Heal(_) | EffectKind::HealPercent(_) | EffectKind::Regen(_) => true,
        EffectKind::Composite(es) => es.iter().any(|e| healing_kind(&e.kind)),
        EffectKind::BattlePersistent(k) | EffectKind::InventoryEffect(k) => healing_kind(k),
        _ => false,
    }
}
fn skill<'a>(b: &'a Battle, a: &LegalAction) -> Option<&'a Skill> {
    let i = match a.action {
        Action::Skill { skill, .. } | Action::SkillAuto { skill } => skill,
        _ => return None,
    };
    b.actor.and_then(|id| b.units[id].blueprint.skills.get(i))
}
fn threat(b: &Battle, id: UnitId) -> f64 {
    let u = &b.units[id];
    if !u.active() || u.friendly() {
        return 0.;
    }
    let damage = u
        .blueprint
        .skills
        .iter()
        .filter(|s| !s.target.friendly)
        .map(|s| s.damage.map_or(0., |d| f64::from(d[0] + d[1]) / 2.))
        .fold(0., f64::max);
    damage * f64::from(u.action_count().saturating_sub(u8::from(u.stunned)))
}
#[derive(Deserialize)]
struct Rush {
    boss_tag: String,
    enablers: Vec<String>,
    high_threat_classes: Vec<String>,
    min_high_threat_enemies: usize,
    short_party_threshold: usize,
    short_party_threat_enemies: usize,
    damage: f64,
    boss_focus: f64,
    heal: f64,
    door_heal: f64,
    dot_turns: f64,
    control: f64,
    enabler_damage: f64,
    enabler_kill: f64,
    enabler_stun: f64,
    move_cost: f64,
    pass_cost: f64,
    full_casualty_rollout: bool,
    force_enabler_response: bool,
    force_boss_pressure: bool,
}
fn rush_config() -> &'static Rush {
    static DATA: std::sync::OnceLock<Rush> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(r#"{
  "version": "w127-rush-candidate-v4",
  "status": "candidate, not original-game calibrated",
  "scope": "Visible Boss/enabler, or multiple listed high-threat enemies; a short party needs one listed threat. Ordinary scoring/three plans unchanged outside trigger.",
  "boss_tag": "boss",
  "enablers": [
    "brigand_fuseman_A",
    "brigand_fuseman_B",
    "brigand_fuseman_C"
  ],
  "damage": 8.0,
  "boss_focus": 1.8,
  "heal": 0.5,
  "door_heal": 200.0,
  "dot_turns": 2.0,
  "control": 2.0,
  "enabler_damage": 200.0,
  "enabler_kill": 1000.0,
  "enabler_stun": 350.0,
  "move_cost": 2.0,
  "pass_cost": 100.0,
  "full_casualty_rollout": true,
  "high_threat_classes": [
    "octotank_A",
    "octotank_B",
    "octotank_C",
    "fungal_artillery_A",
    "fungal_artillery_B",
    "fungal_artillery_C",
    "madman_A",
    "madman_B",
    "madman_C"
  ],
  "min_high_threat_enemies": 2,
  "short_party_threshold": 4,
  "short_party_threat_enemies": 1,
  "force_enabler_response": true,
  "force_boss_pressure": true
}
"#)
            .expect("checked W127 data")
    })
}
pub(super) fn rush_active(b: &Battle) -> bool {
    let c = rush_config();
    let heroes = b
        .units
        .iter()
        .filter(|u| u.active() && u.friendly())
        .count();
    let threats = b
        .units
        .iter()
        .filter(|u| u.active() && !u.friendly() && c.high_threat_classes.contains(&u.blueprint.id))
        .count();
    threats >= c.min_high_threat_enemies
        || (heroes < c.short_party_threshold && threats >= c.short_party_threat_enemies)
        || b.units.iter().any(|u| {
            u.active()
                && !u.friendly()
                && (u.blueprint.mechanics.boss.tags.contains(&c.boss_tag)
                    || c.enablers.contains(&u.blueprint.id))
        })
}
fn rush_score(b: &Battle, o: &Observation, a: &LegalAction, state: &FastState) -> f64 {
    let c = rush_config();
    let ability = skill(b, a);
    fn dot(e: &dd_data::combat::Effect, turns: f64, bleed: f64, blight: f64) -> f64 {
        let duration = turns.min(e.duration as f64);
        match &e.kind {
            EffectKind::Bleed(n) => {
                (*n as f64).max(0.) * duration * (e.chance - bleed).clamp(0., 1.)
            }
            EffectKind::Blight(n) => {
                (*n as f64).max(0.) * duration * (e.chance - blight).clamp(0., 1.)
            }
            EffectKind::Composite(es) => es.iter().map(|e| dot(e, turns, bleed, blight)).sum(),
            _ => 0.,
        }
    }
    let control = ability.map_or(0., |x| x.effects.iter().map(stun).fold(0., f64::max));
    let mut score = 0.;
    for id in &a.targets {
        let Some(v) = o.units.iter().find(|v| v.id == *id) else {
            continue;
        };
        if v.hero && !v.corpse {
            let usable = ((a.heal[0] + a.heal[1]) as f64 / 2.).min((v.max_hp - v.hp).max(0) as f64);
            score += usable * c.heal;
            if v.hp == 0 && usable > 0. {
                score += c.door_heal;
            }
        } else if !v.hero && !v.corpse {
            let stats = b.units[*id].stats();
            let dots = ability.map_or(0., |x| {
                x.effects
                    .iter()
                    .map(|e| dot(e, c.dot_turns, stats.bleed_resist, stats.blight_resist))
                    .sum::<f64>()
            });
            let damage = (a.damage[0] + a.damage[1]) as f64 / 2. * a.hit_chance;
            let boss = b.units[*id]
                .blueprint
                .mechanics
                .boss
                .tags
                .contains(&c.boss_tag)
                || c.high_threat_classes.contains(&v.class);
            score += (damage + dots * a.hit_chance).min(v.hp.max(0) as f64)
                * c.damage
                * if boss { c.boss_focus } else { 1. };
            let stunchance = if v.stunned {
                0.
            } else {
                (control - state.stun_resist[id]).clamp(0., 1.) * a.hit_chance
            };
            score += stunchance * state.threats[id] * c.control;
            if c.enablers.contains(&v.class) {
                score += damage.min(v.hp.max(0) as f64) / v.hp.max(1) as f64 * c.enabler_damage;
                if a.damage[0] >= v.hp {
                    score += a.hit_chance * c.enabler_kill;
                }
                score += stunchance * c.enabler_stun;
            }
        }
    }
    match a.action {
        Action::Move { .. } => score -= c.move_cost,
        Action::Pass => score -= c.pass_cost,
        Action::Retreat | Action::Sacrifice { .. } => score -= 10000.,
        _ => {}
    }
    score
}
struct FastState {
    low: bool,
    tail: Option<UnitId>,
    tail_safe: bool,
    threats: BTreeMap<UnitId, f64>,
    stun_resist: BTreeMap<UnitId, f64>,
}
impl FastState {
    fn new(b: &Battle, o: &Observation, s: &Settings) -> Self {
        let enemies = o
            .units
            .iter()
            .filter(|u| !u.hero && !u.corpse && u.hp > 0)
            .count();
        let total = o
            .units
            .iter()
            .filter(|u| !u.hero && !u.corpse)
            .map(|u| threat(b, u.id))
            .sum::<f64>();
        let minhp = o
            .units
            .iter()
            .filter(|u| u.hero && !u.corpse)
            .map(|u| u.hp.max(1))
            .min()
            .unwrap_or(1);
        let low = enemies as f64 <= s.fast_policy["low_threat_enemy_count"]
            || total <= f64::from(minhp) * s.fast_policy["low_threat_hp_fraction"];
        Self {
            low,
            tail: selected_tail(b, o, s),
            tail_safe: tail_safe(b, o, s),
            threats: o.units.iter().map(|u| (u.id, threat(b, u.id))).collect(),
            stun_resist: o
                .units
                .iter()
                .filter(|u| !u.hero && !u.corpse)
                .map(|u| (u.id, b.units[u.id].stats().stun_resist))
                .collect(),
        }
    }
}
fn fast_score(
    b: &Battle,
    o: &Observation,
    a: &LegalAction,
    s: &Settings,
    state: &FastState,
) -> f64 {
    let low = state.low;
    let ability = skill(b, a);
    let control = ability.map_or(0., |x| x.effects.iter().map(stun).fold(0., f64::max));
    let mut value = 0.;
    for id in &a.targets {
        let Some(v) = o.units.iter().find(|u| u.id == *id) else {
            continue;
        };
        if v.hero && !v.corpse {
            let heal = f64::from(a.heal[0] + a.heal[1]) / 2.;
            let usable = heal.min(f64::from((v.max_hp - v.hp).max(0)));
            value += usable
                * s.fast_policy["heal_weight"]
                * if low {
                    s.fast_policy["low_threat_recovery_multiplier"]
                } else {
                    1.
                };
            if v.hp == 0 && usable > 0. {
                value += weight(s, "door");
            }
            value += f64::from(a.stress_heal.max(0).min(v.stress.max(0)))
                * s.fast_policy["relief_weight"]
                * if low {
                    s.fast_policy["low_threat_recovery_multiplier"]
                } else {
                    s.fast_policy["high_threat_relief_multiplier"]
                };
        } else if !v.hero && !v.corpse {
            let damage = f64::from(a.damage[0] + a.damage[1]) / 2. * a.hit_chance;
            value += damage.min(f64::from(v.hp.max(0))) * s.fast_policy["damage_weight"];
            if damage >= f64::from(v.hp) && v.hp > 0 {
                value += state.threats[id];
            }
            if !v.stunned {
                value += (control - state.stun_resist[id]).clamp(0., 1.)
                    * a.hit_chance
                    * state.threats[id]
                    * s.fast_policy["control_weight"];
            }
        }
    }
    match a.action {
        Action::Move { .. } => value -= s.fast_policy["movement_cost"],
        Action::Pass => value -= s.fast_policy["pass_cost"],
        Action::Retreat => value -= s.fast_policy["retreat_default_cost"],
        Action::Sacrifice { .. } => value -= s.fast_policy["sacrifice_default_cost"],
        _ => {}
    }
    value
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Plan {
    Finish,
    Recover,
    Protect,
    Rush,
}
#[derive(Deserialize)]
struct TailConfig {
    max_enemies: usize,
    max_rank: u8,
    hp_weight: f64,
    protection_weight: f64,
    threat_weight: f64,
    other_target_bonus: f64,
    stun_bonus: f64,
}
fn tail_config() -> &'static TailConfig {
    static DATA: std::sync::OnceLock<TailConfig> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(r#"{"max_enemies":3,"max_rank":2,"hp_weight":1.0,"protection_weight":40.0,"threat_weight":8.0,"other_target_bonus":12.0,"stun_bonus":100.0}
"#)
            .expect("checked tail candidate")
    })
}
fn unsafe_tail_effect(k: &EffectKind) -> bool {
    match k {
        EffectKind::Composite(es) => es.iter().any(|e| unsafe_tail_effect(&e.kind)),
        EffectKind::BattlePersistent(k) | EffectKind::InventoryEffect(k) => unsafe_tail_effect(k),
        EffectKind::Bleed(_)
        | EffectKind::Blight(_)
        | EffectKind::Burn(_)
        | EffectKind::Horror(_)
        | EffectKind::ProtectedHorror(_)
        | EffectKind::Heal(_)
        | EffectKind::HealPercent(_)
        | EffectKind::Regen(_)
        | EffectKind::Lifecycle(_)
        | EffectKind::Boss(_) => true,
        EffectKind::Stress(n) => *n > 0,
        _ => false,
    }
}
fn selected_tail(b: &Battle, o: &Observation, s: &Settings) -> Option<UnitId> {
    let c = tail_config();
    let heroes = o
        .units
        .iter()
        .filter(|u| u.hero && !u.corpse)
        .collect::<Vec<_>>();
    if heroes.iter().any(|u| {
        u.hp <= 0
            || !s.coherence029
                && (u.hp as f64 / u.max_hp.max(1) as f64 <= s.policy["tail_min_hp"]
                    || u.stress as f64 >= s.policy["tail_max_stress"])
    }) || !heroes.iter().any(|u| u.hp < u.max_hp || u.stress > 0)
    {
        return None;
    }
    let enemies = o
        .units
        .iter()
        .filter(|u| !u.hero && !u.corpse && u.hp > 0)
        .collect::<Vec<_>>();
    if enemies.is_empty() || enemies.len() > c.max_enemies || rush_active(b) {
        return None;
    }
    let minhp = o
        .units
        .iter()
        .filter(|u| u.hero && !u.corpse)
        .map(|u| u.hp)
        .min()
        .unwrap_or(0) as f64;
    enemies
        .into_iter()
        .filter(|u| {
            let bp = &b.units[u.id].blueprint;
            bp.size == 1
                && u.rank <= c.max_rank
                && threat(b, u.id) < minhp * s.policy["tail_threat_fraction"]
                && bp.skills.iter().all(|sk| {
                    sk.heal.is_none()
                        && sk
                            .effects
                            .iter()
                            .chain(sk.tactical.mode_effects.values().flatten())
                            .all(|e| !unsafe_tail_effect(&e.kind))
                })
        })
        .max_by(|a, z| {
            let av = a.hp as f64 * c.hp_weight
                + b.units[a.id].stats().protection * c.protection_weight
                - threat(b, a.id) * c.threat_weight;
            let zv = z.hp as f64 * c.hp_weight
                + b.units[z.id].stats().protection * c.protection_weight
                - threat(b, z.id) * c.threat_weight;
            av.total_cmp(&zv).then_with(|| z.id.cmp(&a.id))
        })
        .map(|u| u.id)
}
fn tail_safe(b: &Battle, o: &Observation, s: &Settings) -> bool {
    let enemies = o
        .units
        .iter()
        .filter(|u| !u.hero && !u.corpse && u.hp > 0)
        .collect::<Vec<_>>();
    let heroes = o
        .units
        .iter()
        .filter(|u| u.hero && !u.corpse)
        .collect::<Vec<_>>();
    enemies.len() == 1
        && selected_tail(b, o, s).is_some()
        && heroes.iter().all(|u| {
            u.hp > 0
                && (s.coherence029
                    || (u.hp as f64 / u.max_hp.max(1) as f64 > s.policy["tail_min_hp"]
                        && (u.stress as f64) < s.policy["tail_max_stress"]))
        })
        && enemies.iter().map(|u| threat(b, u.id)).sum::<f64>()
            < heroes.iter().map(|u| u.hp).min().unwrap_or(0) as f64
                * s.policy["tail_threat_fraction"]
        && (heroes
            .iter()
            .any(|u| u.hp as f64 / (u.max_hp.max(1) as f64) < s.policy["tail_hp_target"])
            && b.units
                .iter()
                .filter(|u| u.blueprint.hero && u.alive())
                .any(|u| u.blueprint.skills.iter().any(|s| s.heal.is_some()))
            || heroes
                .iter()
                .any(|u| u.stress as f64 > s.policy["tail_stress_target"])
                && b.units
                    .iter()
                    .filter(|u| u.blueprint.hero && u.alive())
                    .any(|u| {
                        u.blueprint.skills.iter().any(|s| {
                            s.effects
                                .iter()
                                .any(|e| matches!(e.kind,EffectKind::Stress(n) if n<0))
                        })
                    }))
}
fn plan_score(
    b: &Battle,
    o: &Observation,
    a: &LegalAction,
    s: &Settings,
    state: &FastState,
    plan: Plan,
) -> f64 {
    if plan == Plan::Rush && rush_active(b) {
        return rush_score(b, o, a, state);
    }
    let control = skill(b, a).map_or(0., |x| x.effects.iter().map(stun).fold(0., f64::max));
    let mut value = fast_score(b, o, a, s, state);
    if plan == Plan::Protect {
        for id in &a.targets {
            if let Some(u) = o.units.iter().find(|u| u.id == *id && u.hero && !u.corpse) {
                let urgency = 1. - u.hp.max(0) as f64 / u.max_hp.max(1) as f64;
                value += (a.heal[0] + a.heal[1]) as f64 * urgency * s.policy["protect_heal"];
            }
        }
    }
    if plan == Plan::Recover
        && !rush_active(b)
        && let Some(tail) = state.tail
    {
        for id in &a.targets {
            if o.units.iter().any(|u| u.id == *id && !u.hero && !u.corpse) {
                if *id != tail {
                    value += state.threats[id] * tail_config().other_target_bonus;
                } else if a.damage[1] < b.units[*id].hp && !b.units[*id].stunned {
                    value += (control - state.stun_resist[id]).clamp(0., 1.)
                        * a.hit_chance
                        * tail_config().stun_bonus;
                } else if a.damage[1] >= b.units[*id].hp {
                    value -= s.policy["tail_kill_penalty"];
                }
            }
        }
    }
    if plan == Plan::Recover && state.tail_safe {
        let valid = skill(b, a).is_some_and(|s| s.tactical.stall_invalidating);
        let needed = b
            .scenario
            .rules
            .context
            .as_ref()
            .is_some_and(|c| b.lifecycle.invalidating_actions < c.stall_skills);
        let mut harm = 0.;
        for id in &a.targets {
            if let Some(u) = o.units.iter().find(|u| u.id == *id) {
                if !u.hero && !u.corpse {
                    if control <= state.stun_resist[id] || a.damage[1] >= u.hp {
                        harm += (a.damage[1].max(0) as f64).min(u.hp.max(0) as f64);
                    }
                    if a.damage[1] >= u.hp {
                        value -= s.policy["tail_kill_penalty"];
                    }
                }
                if !u.hero && u.corpse && valid {
                    value += if needed {
                        s.policy["corpse_needed"]
                    } else {
                        s.policy["corpse_other"]
                    };
                }
                if u.hero {
                    value += (a.heal[1].max(0).min(u.max_hp - u.hp)
                        + a.stress_heal.max(0).min(u.stress)) as f64
                        * s.policy["tail_recovery"];
                }
            }
        }
        value -= harm * s.policy["tail_damage_penalty"];
        if needed && valid {
            value += s.policy["needed_attack"];
        }
        // Passing has its native stress consequence, included by simulation.
        if a.action == Action::Pass {
            value = -s.policy["tail_pass_cost"];
        }
    }
    value
}
pub(super) fn fast_action(m: &Surrogate, s: &Settings, plan: Plan) -> Result<Action> {
    if !s.coherence029 {
        return exhaustive_action029(m, s, plan);
    }
    let o = m.observation();
    if o.player_selection {
        return Err("simulated_sacrifice_selection_unsupported".into());
    }
    let state = FastState::new(&m.battle, &o, s);
    // Identical total ordering, including the original lower-index tie break.
    // Probe complete item effects only when an action could actually win.
    let mut ordered = o
        .actions
        .iter()
        .enumerate()
        .filter(|(_, a)| a.action != Action::Retreat)
        .map(|(i, a)| (i, a, plan_score(&m.battle, &o, a, s, &state, plan)))
        .collect::<Vec<_>>();
    ordered.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    ordered
        .into_iter()
        .find(|(_, a, _)| effective_item(m, &a.action))
        .map(|(_, a, _)| a.action.clone())
        .ok_or("no_simulated_friendly_action".into())
}
pub(super) fn exhaustive_action029(m: &Surrogate, s: &Settings, plan: Plan) -> Result<Action> {
    let o = m.observation();
    if o.player_selection {
        return Err("simulated_sacrifice_selection_unsupported".into());
    }
    let state = FastState::new(&m.battle, &o, s);
    o.actions
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.action != Action::Retreat && (!s.coherence029 || effective_item(m, &a.action))
        })
        .max_by(|(ia, a), (ib, b)| {
            plan_score(&m.battle, &o, a, s, &state, plan)
                .total_cmp(&plan_score(&m.battle, &o, b, s, &state, plan))
                .then_with(|| ib.cmp(ia))
        })
        .map(|(_, a)| a.action.clone())
        .ok_or("no_simulated_friendly_action".into())
}
// W-060/W-113: inspect only a disposable public surrogate, never the real Battle.
pub(super) fn effective_item(m: &Surrogate, a: &Action) -> bool {
    let Action::Item { item, .. } = a else {
        return true;
    };
    let Some(stack) = m.battle.supplies.get(*item) else {
        return true;
    };
    // Uncertain effects must remain candidates; a single failed sample proves nothing.
    if stack
        .definition
        .effects
        .iter()
        .chain(&stack.definition.semantics.sequential_effects)
        .any(|e| e.chance < 1.)
    {
        return true;
    }
    let mut probe = m.clone();
    let before = probe.observation();
    if probe.step(a.clone()).is_err() {
        return true;
    }
    let after = probe.observation();
    // A refusal did not consume the item; one stochastic refusal cannot prove no effect.
    if before.supplies.get(*item).map(|s| s.1) == after.supplies.get(*item).map(|s| s.1) {
        return true;
    }
    let units = |o: &Observation| {
        let mut v = serde_json::to_value(&o.units).expect("visible units");
        for u in v.as_array_mut().unwrap() {
            u.as_object_mut().unwrap().remove("item_uses");
        }
        v
    };
    units(&before) != units(&after)
        || before.torch != after.torch
        || before.outcome != after.outcome
}
pub(super) fn supply_cost(b: &Battle, input: &PublicBattle) -> Result<f64> {
    let mut total = 0.;
    for (id, count) in &input.observation.supplies {
        let stack = b
            .supplies
            .iter()
            .find(|s| &s.definition.id == id)
            .ok_or("029 missing public supply")?;
        let used = count.saturating_sub(stack.remaining);
        if used > 0 {
            let price = stack
                .definition
                .semantics
                .source_metadata
                .get("purchase_gold_value")
                .and_then(|v| v.first())
                .ok_or("029 supply missing original purchase price")?
                .parse::<f64>()
                .map_err(|e| e.to_string())?;
            total += f64::from(used) * price;
        }
    }
    Ok(total)
}
fn candidates(
    m: &Surrogate,
    o: &Observation,
    s: &Settings,
    ranked: &[(f64, LegalAction, String)],
    retreat_allowed: bool,
) -> Vec<(Action, Vec<String>)> {
    let state = FastState::new(&m.battle, o, s);
    let mut chosen = vec![];
    fn keep(chosen: &mut Vec<(Action, Vec<String>)>, a: &Action, tag: &str) {
        if let Some((_, tags)) = chosen.iter_mut().find(|(x, _)| x == a) {
            if !tags.iter().any(|s| s == tag) {
                tags.push(tag.into());
            }
        } else {
            chosen.push((a.clone(), vec![tag.into()]));
        }
    }
    if rush_active(&m.battle) {
        let mut attack = o
            .actions
            .iter()
            .filter(|a| a.action != Action::Retreat)
            .collect::<Vec<_>>();
        attack.sort_by(|a, b| {
            rush_score(&m.battle, o, b, &state).total_cmp(&rush_score(&m.battle, o, a, &state))
        });
        for a in attack.into_iter().take(s.top_actions) {
            keep(&mut chosen, &a.action, "w127_rush");
        }
    }
    let mut rows = ranked
        .iter()
        .filter(|(_, a, _)| o.actions.iter().any(|x| x.action == a.action))
        .map(|(_, a, _)| a)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows = o
            .actions
            .iter()
            .filter(|a| {
                !matches!(
                    a.action,
                    Action::Pass | Action::Retreat | Action::Move { .. } | Action::Sacrifice { .. }
                )
            })
            .collect();
        rows.sort_by(|a, b| {
            fast_score(&m.battle, o, b, s, &state)
                .total_cmp(&fast_score(&m.battle, o, a, s, &state))
        });
    }
    for a in rows.into_iter().take(s.top_actions) {
        keep(&mut chosen, &a.action, "rule_top");
    }
    for category in [
        "healing",
        "relief",
        "control",
        "movement",
        "retreat",
        "nonlethal",
    ] {
        let eligible = |a: &&LegalAction| match category {
            "healing" => {
                a.heal[1] > 0
                    || skill(&m.battle, a)
                        .is_some_and(|s| s.effects.iter().any(|e| healing_kind(&e.kind)))
            }
            "relief" => a.stress_heal > 0,
            "control" => skill(&m.battle, a).is_some_and(|x| {
                x.effects.iter().any(|e| {
                    stun(e) > 0.
                        || matches!(
                            e.kind,
                            EffectKind::Pull(_)
                                | EffectKind::Push(_)
                                | EffectKind::Shuffle
                                | EffectKind::ShuffleParty
                        )
                })
            }),
            "nonlethal" => {
                skill(&m.battle, a).is_some_and(|s| s.tactical.stall_invalidating)
                    && a.targets.iter().all(|id| {
                        o.units
                            .iter()
                            .find(|u| u.id == *id)
                            .is_some_and(|u| u.corpse || u.hero || a.damage[1] < u.hp)
                    })
            }
            "movement" => {
                matches!(a.action, Action::Move { .. })
                    || skill(&m.battle, a).is_some_and(|x| x.move_forward > 0 || x.move_back > 0)
            }
            _ => a.action == Action::Retreat,
        };
        if let Some(a) = o.actions.iter().filter(eligible).max_by(|a, b| {
            fast_score(&m.battle, o, a, s, &state)
                .total_cmp(&fast_score(&m.battle, o, b, s, &state))
        }) {
            keep(&mut chosen, &a.action, category);
        }
    }
    if chosen.is_empty()
        && let Some(a) = o.actions.first()
    {
        keep(&mut chosen, &a.action, "only_legal");
    }
    if rush_config().force_enabler_response && rush_active(&m.battle) {
        let enabler = |a: &LegalAction| {
            a.targets.iter().any(|id| {
                o.units.iter().find(|u| u.id == *id).is_some_and(|u| {
                    !u.hero
                        && !u.corpse
                        && u.hp > 0
                        && rush_config().enablers.contains(&u.class)
                        && (a.damage[1] > 0
                            || skill(&m.battle, a).is_some_and(|s| {
                                s.effects.iter().map(stun).fold(0., f64::max)
                                    > state.stun_resist[id]
                            }))
                })
            })
        };
        if o.actions.iter().any(enabler) {
            chosen.retain(|(action, _)| {
                *action == Action::Retreat
                    || o.actions
                        .iter()
                        .find(|a| a.action == *action)
                        .is_some_and(|a| {
                            enabler(a)
                                || (a.heal[1] > 0
                                    && a.targets.iter().any(|id| {
                                        o.units.iter().any(|u| u.id == *id && u.hero && u.hp == 0)
                                    }))
                        })
            });
        }
    }
    if rush_config().force_boss_pressure
        && !o
            .units
            .iter()
            .any(|u| !u.hero && !u.corpse && u.hp > 0 && rush_config().enablers.contains(&u.class))
    {
        let pressure = |a: &LegalAction| {
            a.damage[1] > 0
                && a.targets.iter().any(|id| {
                    m.battle.units[*id].active()
                        && !m.battle.units[*id].friendly()
                        && m.battle.units[*id]
                            .blueprint
                            .mechanics
                            .boss
                            .tags
                            .contains(&rush_config().boss_tag)
                })
        };
        if o.actions.iter().any(pressure) {
            chosen.retain(|(action, _)| {
                *action == Action::Retreat
                    || o.actions
                        .iter()
                        .find(|a| a.action == *action)
                        .is_some_and(|a| {
                            pressure(a)
                                || (a.heal[1] > 0
                                    && a.targets.iter().any(|id| {
                                        o.units.iter().any(|u| u.id == *id && u.hero && u.hp == 0)
                                    }))
                        })
            });
        }
    }
    chosen.retain(|(a, _)| {
        (*a != Action::Retreat || retreat_allowed) && (!s.coherence029 || effective_item(m, a))
    });
    // Filtering must not resurrect a rejected item via the host's legacy fallback.
    // Reuse the existing legal-menu fallback after applying the same constraints.
    if s.coherence029
        && chosen.is_empty()
        && let Some(a) = o
            .actions
            .iter()
            .find(|a| a.action != Action::Retreat && effective_item(m, &a.action))
    {
        keep(&mut chosen, &a.action, "only_effective_legal");
    }
    chosen
}
fn run_sample(
    m: &Surrogate,
    input: &PublicBattle,
    s: &Settings,
    first: &Action,
    seed: u64,
    plan: Plan,
) -> Result<SampleResult> {
    let full_casualty = rush_active(&m.battle) && rush_config().full_casualty_rollout;
    let mut m = m.sample(seed)?;
    let start = input.observation.round;
    let horizon =
        if m.battle.units.iter().any(|u| {
            !u.blueprint.hero && u.blueprint.mechanics.boss.tags.iter().any(|t| t == "boss")
        }) {
            s.policy["boss_horizon"] as u32
        } else {
            s.horizon_rounds
        };
    m.battle.scenario.round_limit = start.saturating_add(horizon);
    let heroes = input
        .observation
        .units
        .iter()
        .filter(|u| u.hero && !u.corpse)
        .map(|u| u.id)
        .collect::<BTreeSet<_>>();
    let mut doors = 0;
    let mut afflictions = BTreeSet::new();
    let mut next = Some(first.clone());
    let mut steps = 0;
    while m.battle.outcome().is_none() && m.battle.round < start.saturating_add(horizon) {
        if steps >= s.transitions_per_round * horizon as usize {
            return Err("simulation_transition_limit".into());
        }
        let action = if let Some(a) = next.take() {
            a
        } else if m
            .battle
            .actor
            .is_some_and(|id| m.battle.units[id].friendly())
        {
            fast_action(&m, s, plan)?
        } else {
            m.battle.monster_action()?
        };
        for e in m.step(action)? {
            match e {
                Event::DeathsDoor(id) if heroes.contains(&id) => doors += 1,
                Event::Resolve {
                    target,
                    state: Resolve::Afflicted(_),
                } if heroes.contains(&target) => {
                    afflictions.insert(target);
                }
                _ => {}
            }
        }
        steps += 1;
        if !full_casualty && heroes.iter().any(|id| !m.battle.units[*id].alive()) {
            break;
        }
        if plan == Plan::Recover
            && m.battle.round > start
            && heroes.iter().all(|id| {
                let u = &m.battle.units[*id];
                u.hp >= u.stats().hp && u.stress <= 0
            })
        {
            break;
        }
    }
    let mut result = summarize(&m, input, s, doors, &afflictions);
    retreat::price(&m, input, s, &mut result)?;
    if s.coherence029 {
        result.score -= supply_cost(&m.battle, input)?;
    }
    Ok(result)
}
pub(super) fn summarize(
    m: &Surrogate,
    input: &PublicBattle,
    s: &Settings,
    doors: u32,
    afflictions: &BTreeSet<UnitId>,
) -> SampleResult {
    let start = input.observation.round;
    let heroes = input
        .observation
        .units
        .iter()
        .filter(|u| u.hero && !u.corpse)
        .map(|u| u.id)
        .collect::<BTreeSet<_>>();
    let b = &m.battle;
    let mut value = 0.;
    let mut deaths = 0;
    let mut weighted_deaths = 0.;
    let mut unrecovered = 0;
    let mut stress = 0;
    let mut hp_fraction = 0.;
    for id in &heroes {
        let u = &b.units[*id];
        let level = input.heroes.get(id).map_or(0, |h| h.level);
        if !u.alive() {
            deaths += 1;
        } else if !u.present() {
            unrecovered += 1;
        }
        if !u.alive() || !u.present() {
            let loss = weight(s, "death")
                * (1. + weight(s, "death_level") * f64::from(level))
                * if input.heroes.get(id).is_some_and(|h| {
                    [
                        "occultist",
                        "vestal",
                        "man_at_arms",
                        "plague_doctor",
                        "houndmaster",
                        "jester",
                    ]
                    .contains(&h.class.as_str())
                }) {
                    weight(s, "critical_death")
                } else {
                    1.0
                };
            weighted_deaths += loss;
            value -= loss;
        }
        let fraction = if u.alive() && u.present() {
            f64::from(u.hp.max(0)) / f64::from(u.stats().hp.max(1))
        } else {
            0.
        };
        hp_fraction += fraction;
        value -= weight(s, "missing_hp") * (1. - fraction).max(0.);
        if u.alive() && u.hp == 0 {
            value -= weight(s, "door");
        }
        if u.alive() {
            stress += u.stress;
            value -= f64::from(u.stress.clamp(0, 100)) * weight(s, "stress")
                + f64::from((u.stress - 100).max(0)) * weight(s, "excess_stress");
        }
    }
    value -= afflictions.len() as f64 * weight(s, "affliction");
    let rounds = b.round.saturating_sub(start);
    value -= f64::from(rounds) * weight(s, "round");
    let retreated = b.outcome() == Some(Outcome::Retreated);
    if retreated {
        value -= retreat::abandon_cost(input, s);
    }
    let unfinished = b.outcome().is_none()
        || matches!(
            b.outcome(),
            Some(Outcome::RoundLimit | Outcome::ResolutionLimit)
        );
    if unfinished {
        value -= (0..b.units.len()).map(|id| threat(b, id)).sum::<f64>()
            * weight(s, "unfinished_threat");
    }
    SampleResult {
        tail_rounds: 0,
        tail_hp_restored: 0,
        tail_stress_recovered: 0,
        score: value,
        deaths,
        weighted_deaths,
        unrecovered,
        wipe: deaths as usize == heroes.len(),
        door_entries: doors,
        new_afflictions: afflictions.len() as u32,
        stress,
        hp_fraction,
        rounds,
        retreated,
        retreat_cost_kind: retreated.then(|| "abandonment".into()),
        unfinished,
        outcome: b.outcome(),
    }
}
pub fn evaluate_action(
    input: &PublicBattle,
    defs: &Definitions,
    s: &Settings,
    action: &Action,
    index: usize,
) -> Result<SampleResult> {
    execution025::hypothetical026(input.mechanisms, || {
        execution024b::hypothetical026(input.single_target_fallback, || {
            let m = reconstruct(input, defs, 0)?;
            run_sample(
                &m,
                input,
                s,
                action,
                sample_seed(input, s, index)?,
                Plan::Finish,
            )
        })
    })
}
/// Tool-only forced-action scoring. No existing decision calls this function.
pub fn diagnostic_score030(
    input: &PublicBattle,
    defs: &Definitions,
    s: &Settings,
    action: &Action,
) -> Result<Value> {
    execution025::hypothetical026(input.mechanisms, || {
        execution024b::hypothetical026(input.single_target_fallback, || {
            let model = reconstruct(input, defs, 0)?;
            let plans: &[Plan] = if rush_active(&model.battle) {
                &[Plan::Rush]
            } else {
                &[Plan::Finish, Plan::Recover, Plan::Protect]
            };
            let mut rows = vec![];
            for plan in plans {
                let mut scores = (0..s.samples)
                    .map(|i| {
                        run_sample(&model, input, s, action, sample_seed(input, s, i)?, *plan)
                            .map(|r| r.score)
                    })
                    .collect::<Result<Vec<_>>>()?;
                scores.sort_by(f64::total_cmp);
                let mean = scores.iter().sum::<f64>() / scores.len() as f64;
                let n = (scores.len() as f64 * s.tail_fraction).ceil().max(1.) as usize;
                let tail = scores[..n].iter().sum::<f64>() / n as f64;
                rows.push(serde_json::json!({"mean":mean,"worst_tail":tail,"value":s.mean_weight*mean+s.tail_weight*tail,"samples":s.samples,"plan":format!("{plan:?}"),"source":"forced public-action diagnostic; outside actual search when not a root candidate"}));
            }
            rows.sort_by(|a, b| {
                b["value"]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&a["value"].as_f64().unwrap())
            });
            Ok(serde_json::json!({"best":rows.first(),"plans":rows}))
        })
    })
}
pub fn choose(
    input: &PublicBattle,
    defs: &Definitions,
    s: &Settings,
    ranked: &[(f64, LegalAction, String)],
) -> Result<Decision> {
    execution025::hypothetical026(input.mechanisms, || {
        execution024b::hypothetical026(input.single_target_fallback, || {
            let m = reconstruct(input, defs, 0)?;
            if !input
                .observation
                .units
                .iter()
                .any(|u| Some(u.id) == input.observation.actor && u.hero && !u.corpse)
            {
                return Err("not_player_boundary".into());
            }
            let retreat = retreat::assess(&m, input, s)?;
            let candidates = candidates(&m, &input.observation, s, ranked, retreat.allowed);
            if candidates.len() == 1 {
                return Ok(Decision {
                    action: candidates[0].0.clone(),
                    candidates: vec![],
                    simulations: 0,
                    roundtrip: true,
                    estimates: s.estimates.clone(),
                    retreat: retreat.clone(),
                });
            }
            let seeds = (0..s.samples)
                .map(|i| sample_seed(input, s, i))
                .collect::<Result<Vec<_>>>()?;
            let mut pilots = Vec::new();
            let plans: &[Plan] = if rush_active(&m.battle) {
                &[Plan::Rush]
            } else {
                &[Plan::Finish, Plan::Recover, Plan::Protect]
            };
            for (action, categories) in candidates {
                for &plan in plans {
                    let results = seeds
                        .iter()
                        .take(2)
                        .map(|seed| run_sample(&m, input, s, &action, *seed, plan))
                        .collect::<Result<Vec<_>>>()?;
                    let score = results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
                    pilots.push((action.clone(), categories.clone(), plan, results, score));
                }
            }
            let mut order = (0..pilots.len()).collect::<Vec<_>>();
            order.sort_by(|a, b| pilots[*b].4.total_cmp(&pilots[*a].4).then(a.cmp(b)));
            // Avoid spending the entire final budget on identical first actions.
            let mut distinct = Vec::new();
            let mut finalists = BTreeSet::new();
            for i in &order {
                if distinct.contains(&pilots[*i].0) {
                    continue;
                }
                distinct.push(pilots[*i].0.clone());
                finalists.insert(*i);
                if distinct.len() >= s.policy["finalists"] as usize {
                    break;
                }
            }
            // A two-sample pilot is never sufficient to reject emergency escape/healing.
            if input
                .observation
                .units
                .iter()
                .any(|u| u.hero && !u.corpse && u.hp == 0)
            {
                for category in ["retreat", "healing"] {
                    if let Some(i) = order
                        .iter()
                        .find(|i| pilots[**i].1.iter().any(|c| c == category))
                    {
                        finalists.insert(*i);
                    }
                }
            }
            let mut rows = vec![];
            for (i, (action, mut categories, plan, mut results, _)) in
                pilots.into_iter().enumerate()
            {
                categories.push(format!("plan:{plan:?}"));
                if finalists.contains(&i) {
                    for seed in seeds.iter().skip(2) {
                        results.push(run_sample(&m, input, s, &action, *seed, plan)?);
                    }
                } else {
                    categories.push("eliminated_pilot".into());
                }
                results.sort_by(|a, b| a.score.total_cmp(&b.score));
                let mean = results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
                let n = (results.len() as f64 * s.tail_fraction).ceil().max(1.) as usize;
                let worst_tail = results[..n].iter().map(|r| r.score).sum::<f64>() / n as f64;
                rows.push(CandidateResult {
                    action,
                    categories,
                    samples: results.len(),
                    mean,
                    worst_tail,
                    value: s.mean_weight * mean + s.tail_weight * worst_tail,
                    mean_deaths: results.iter().map(|r| r.deaths as f64).sum::<f64>()
                        / results.len() as f64,
                    mean_stress: results.iter().map(|r| r.stress as f64).sum::<f64>()
                        / results.len() as f64,
                    mean_hp_fraction: results.iter().map(|r| r.hp_fraction).sum::<f64>()
                        / results.len() as f64,
                    unfinished: results.iter().filter(|r| r.unfinished).count(),
                });
            }
            let best = rows
                .iter()
                .enumerate()
                .filter(|(_, r)| !r.categories.iter().any(|c| c == "eliminated_pilot"))
                .max_by(|(ia, a), (ib, b)| a.value.total_cmp(&b.value).then_with(|| ib.cmp(ia)))
                .ok_or("no026 candidate")?
                .1
                .action
                .clone();
            Ok(Decision {
                action: best,
                simulations: rows.iter().map(|r| r.samples).sum(),
                candidates: rows,
                roundtrip: true,
                estimates: s.estimates.clone(),
                retreat: retreat.clone(),
            })
        })
    })
}



