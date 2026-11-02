//! Flat common-seed Monte Carlo. Enemy actions use the complete original sampler.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SampleResult {
    pub score: f64,
    pub deaths: u32,
    pub unrecovered: u32,
    pub wipe: bool,
    pub door_entries: u32,
    pub new_afflictions: u32,
    pub stress: i32,
    pub hp_fraction: f64,
    pub rounds: u32,
    pub retreated: bool,
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
struct FastState {
    low: bool,
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
fn fast_action(m: &Surrogate, s: &Settings) -> Result<Action> {
    let o = m.observation();
    if o.player_selection {
        return Err("simulated_sacrifice_selection_unsupported".into());
    }
    if o.units
        .iter()
        .filter(|u| u.hero && !u.corpse && u.hp == 0)
        .count()
        >= 2
        && o.actions.iter().any(|a| a.action == Action::Retreat)
    {
        return Ok(Action::Retreat);
    }
    let state = FastState::new(&m.battle, &o, s);
    o.actions
        .iter()
        .enumerate()
        .max_by(|(ia, a), (ib, b)| {
            fast_score(&m.battle, &o, a, s, &state)
                .total_cmp(&fast_score(&m.battle, &o, b, s, &state))
                .then_with(|| ib.cmp(ia))
        })
        .map(|(_, a)| a.action.clone())
        .ok_or("no_simulated_friendly_action".into())
}
fn candidates(
    m: &Surrogate,
    o: &Observation,
    s: &Settings,
    ranked: &[(f64, LegalAction, String)],
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
    for category in ["healing", "relief", "control", "movement", "retreat"] {
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
    chosen
}
fn run_sample(
    m: &Surrogate,
    input: &PublicBattle,
    s: &Settings,
    first: &Action,
    seed: u64,
) -> Result<SampleResult> {
    let mut m = m.sample(seed)?;
    let start = input.observation.round;
    m.battle.scenario.round_limit = start.saturating_add(s.horizon_rounds);
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
    while m.battle.outcome().is_none() && m.battle.round < start.saturating_add(s.horizon_rounds) {
        if steps >= s.transitions_per_round * s.horizon_rounds as usize {
            return Err("simulation_transition_limit".into());
        }
        let action = if let Some(a) = next.take() {
            a
        } else if m
            .battle
            .actor
            .is_some_and(|id| m.battle.units[id].friendly())
        {
            fast_action(&m, s)?
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
    }
    Ok(summarize(&m, input, s, doors, &afflictions))
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
            value -= weight(s, "death") * (1. + weight(s, "death_level") * f64::from(level));
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
        value -= weight(s, "retreat");
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
        score: value,
        deaths,
        unrecovered,
        wipe: deaths as usize == heroes.len(),
        door_entries: doors,
        new_afflictions: afflictions.len() as u32,
        stress,
        hp_fraction,
        rounds,
        retreated,
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
            run_sample(&m, input, s, action, sample_seed(input, s, index)?)
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
            let candidates = candidates(&m, &input.observation, s, ranked);
            if candidates.len() == 1 {
                return Ok(Decision {
                    action: candidates[0].0.clone(),
                    candidates: vec![],
                    simulations: 0,
                    roundtrip: true,
                    estimates: s.estimates.clone(),
                });
            }
            let seeds = (0..s.samples)
                .map(|i| sample_seed(input, s, i))
                .collect::<Result<Vec<_>>>()?;
            let mut rows = vec![];
            for (action, categories) in candidates {
                let mut results = seeds
                    .iter()
                    .map(|seed| run_sample(&m, input, s, &action, *seed))
                    .collect::<Result<Vec<_>>>()?;
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
                    mean_deaths: results.iter().map(|r| f64::from(r.deaths)).sum::<f64>()
                        / results.len() as f64,
                    mean_stress: results.iter().map(|r| f64::from(r.stress)).sum::<f64>()
                        / results.len() as f64,
                    mean_hp_fraction: results.iter().map(|r| r.hp_fraction).sum::<f64>()
                        / results.len() as f64,
                    unfinished: results.iter().filter(|r| r.unfinished).count(),
                });
            }
            let best = rows
                .iter()
                .enumerate()
                .max_by(|(ia, a), (ib, b)| a.value.total_cmp(&b.value).then_with(|| ib.cmp(ia)))
                .ok_or("no026 candidate")?
                .1
                .action
                .clone();
            Ok(Decision {
                action: best,
                simulations: rows.len() * s.samples,
                candidates: rows,
                roundtrip: true,
                estimates: s.estimates.clone(),
            })
        })
    })
}


