//! W-126. Every probe starts from the public reconstruction, never the real queue.
use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Assessment {
    pub allowed: bool,
    pub door_ids: Vec<UnitId>,
    pub danger_probability: f64,
    pub samples: usize,
    pub scouted_detour: bool,
    pub reason: String,
}
pub fn detour(input: &PublicBattle) -> bool {
    let roaming = input.observation.units.iter().any(|u| {
        !u.hero
            && ["collector", "shambler", "fanatic", "thing"]
                .iter()
                .any(|name| {
                    u.class == *name
                        || ["A", "B", "C"]
                            .iter()
                            .any(|tier| u.class == format!("{name}_{tier}"))
                })
    });
    let Some(map) = &input.rescue.visible_map else {
        return false;
    };
    if !roaming {
        return false;
    }
    let Some(nodes) = map["nodes"].as_array() else {
        return false;
    };
    let Some(blocked) = map["position"].as_str() else {
        return false;
    };
    // A fully revealed cycle around the blocked node proves a bypass. Mere
    // unexplored geometry, a dead-end branch or an unknown gate proves nothing.
    let known = nodes
        .iter()
        .filter(|n| {
            n["id"] != blocked
                && !n["content"].is_null()
                && (n["gate_key"].is_null() || n["opened"] == true)
        })
        .filter_map(|n| n["id"].as_str().map(|id| (id, n)))
        .collect::<BTreeMap<_, _>>();
    let Some(current) = nodes.iter().find(|n| n["id"] == blocked) else {
        return false;
    };
    let exits = current["exits"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|id| known.contains_key(id))
        .collect::<Vec<_>>();
    for start in exits.iter().filter(|id| known[*id]["visited"] == true) {
        let mut seen = BTreeSet::new();
        let mut stack = vec![*start];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if id != *start && exits.contains(&id) {
                return true;
            }
            if let Some(node) = known.get(id) {
                stack.extend(
                    node["exits"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .filter(|id| known.contains_key(id)),
                );
            }
        }
    }
    false
}
pub(crate) fn assess(m: &Surrogate, input: &PublicBattle, s: &Settings) -> Result<Assessment> {
    let mut a = Assessment {
        door_ids: input
            .observation
            .units
            .iter()
            .filter(|u| u.hero && !u.corpse && u.hp == 0)
            .map(|u| u.id)
            .collect(),
        scouted_detour: detour(input),
        ..Default::default()
    };
    if !input
        .observation
        .actions
        .iter()
        .any(|a| a.action == Action::Retreat)
    {
        a.reason = "native_retreat_illegal".into();
        return Ok(a);
    }
    if a.scouted_detour {
        a.allowed = true;
        a.reason = "visible_roaming_boss_scouted_bypass".into();
        return Ok(a);
    }
    if a.door_ids.is_empty() {
        a.reason = "no_hero_at_deaths_door".into();
        return Ok(a);
    }
    a.samples = s.policy["retreat_probe_samples"] as usize;
    let mut danger = 0;
    let mut seed_settings = s.clone();
    seed_settings.salt.push_str(":w126-risk");
    for index in 0..a.samples {
        let mut sample = m.sample(sample_seed(input, &seed_settings, index)?)?;
        let mut pending = a.door_ids.iter().copied().collect::<BTreeSet<_>>();
        let round = sample.battle.round;
        let mut exposed = false;
        for _ in 0..s.transitions_per_round * 2 {
            if pending.is_empty()
                || sample.battle.outcome().is_some()
                || sample.battle.round > round + 1
            {
                break;
            }
            let action = if sample
                .battle
                .actor
                .is_some_and(|id| sample.battle.units[id].friendly())
            {
                decision::fast_action(&sample, s, decision::Plan::Protect)?
            } else {
                sample.battle.monster_action()?
            };
            for e in sample.step(action)? {
                match e {
                    Event::Heal { target, amount } if amount > 0 => {
                        pending.remove(&target);
                    }
                    Event::Damage { target, amount, .. }
                        if amount > 0 && pending.contains(&target) =>
                    {
                        exposed = true
                    }
                    Event::Death(id) if pending.contains(&id) => exposed = true,
                    _ => {}
                }
            }
            if exposed {
                break;
            }
        }
        danger += usize::from(exposed);
    }
    a.danger_probability = danger as f64 / a.samples.max(1) as f64;
    a.allowed = a.danger_probability >= s.policy["retreat_risk_threshold"];
    a.reason = if a.allowed {
        "door_damage_before_public_rescue"
    } else {
        "rescue_precedes_visible_danger"
    }
    .into();
    Ok(a)
}
pub(super) fn abandon_cost(input: &PublicBattle, s: &Settings) -> f64 {
    s.weights[if input.difficulty >= 5 {
        "abandon_5"
    } else if input.difficulty >= 3 {
        "abandon_3"
    } else {
        "abandon_1"
    }]
}
/// Rescue consumes only current visible supplies. Re-fighting resets every
/// visible enemy to full HP and clears its combat damage/statuses.
fn restored(m: &Surrogate, input: &PublicBattle) -> Option<Surrogate> {
    if m.battle
        .scenario
        .rules
        .encounter_contract
        .as_ref()
        .is_some_and(|c| c.retreat_always_from_raid)
    {
        return None;
    }
    let mut retry = m.clone();
    let mut food = input.rescue.food;
    let mut bandages = m
        .battle
        .supplies
        .iter()
        .filter(|s| s.definition.id == "bandage")
        .map(|s| s.remaining)
        .sum::<u32>();
    let mut antidotes = m
        .battle
        .supplies
        .iter()
        .filter(|s| s.definition.id == "antivenom")
        .map(|s| s.remaining)
        .sum::<u32>();
    for (id, u) in retry.battle.units.iter_mut().enumerate() {
        if !u.blueprint.hero {
            *u = Unit::fresh(&u.blueprint);
            continue;
        }
        if !u.alive() {
            return None;
        }
        let mut loss = 0;
        for (bleed, item) in [(true, &mut bandages), (false, &mut antidotes)] {
            let total = u
                .dots
                .iter()
                .filter(|d| {
                    if bleed {
                        matches!(d.kind, EffectKind::Bleed(_))
                    } else {
                        matches!(d.kind, EffectKind::Blight(_))
                    }
                })
                .map(|d| d.amount.max(0) * d.remaining as i32)
                .sum::<i32>();
            if total > 0 && *item > 0 {
                *item -= 1;
            } else {
                loss += total;
            }
        }
        let mut burn = u.burn.max(0);
        while burn > 0 {
            loss += burn;
            burn /= 2;
        }
        let mut bites = input.rescue.food_remaining.get(&id).copied().unwrap_or(0);
        while u.hp < u.stats().hp && food > 0 && bites > 0 && input.rescue.food_heal > 0. {
            u.hp = (u.hp + (u.stats().hp as f64 * input.rescue.food_heal).ceil() as i32)
                .min(u.stats().hp);
            food -= 1;
            bites -= 1;
        }
        if u.hp <= loss || u.stress >= 160 {
            return None;
        }
        u.hp -= loss;
        u.dots.clear();
        u.burn = 0;
        u.stunned = false;
    }
    // Healing/antidotes above cannot be reused inside the retry.
    for supply in &mut retry.battle.supplies {
        if supply.definition.id == "bandage" {
            supply.remaining = bandages;
        }
        if supply.definition.id == "antivenom" {
            supply.remaining = antidotes;
        }
    }
    retry.battle.outcome = None;
    retry.battle.actor = input.observation.actor;
    Some(retry)
}
pub(super) fn price(
    m: &Surrogate,
    input: &PublicBattle,
    s: &Settings,
    result: &mut SampleResult,
) -> Result<()> {
    if !result.retreated {
        return Ok(());
    }
    let forced = m
        .battle
        .scenario
        .rules
        .encounter_contract
        .as_ref()
        .is_some_and(|c| c.retreat_always_from_raid);
    let bypass = detour(input) && !forced;
    let mut discount = bypass;
    if !discount && let Some(retry) = restored(m, input) {
        let mut wins = 0;
        let n = s.policy["retreat_probe_samples"] as usize;
        let mut settings = s.clone();
        settings.salt.push_str(":w124-full-enemy-retry");
        for index in 0..n {
            let mut trial = retry.restart(sample_seed(input, &settings, index)?)?;
            let end = trial.battle.round + s.policy["boss_horizon"] as u32;
            trial.battle.scenario.round_limit = end;
            for _ in 0..s.transitions_per_round * s.policy["boss_horizon"] as usize {
                if trial.battle.outcome().is_some() || trial.battle.round >= end {
                    break;
                }
                let action = if trial
                    .battle
                    .actor
                    .is_some_and(|id| trial.battle.units[id].friendly())
                {
                    decision::fast_action(&trial, s, decision::Plan::Protect)?
                } else {
                    trial.battle.monster_action()?
                };
                trial.step(action)?;
                if trial
                    .battle
                    .units
                    .iter()
                    .any(|u| u.blueprint.hero && !u.alive())
                {
                    break;
                }
            }
            wins += usize::from(
                trial.battle.outcome() == Some(Outcome::HeroesWin)
                    && trial
                        .battle
                        .units
                        .iter()
                        .filter(|u| u.blueprint.hero)
                        .all(Unit::alive),
            );
        }
        discount = wins as f64 / n.max(1) as f64 >= s.policy["retry_win_threshold"];
    }
    if discount {
        result.retreat_cost_kind = Some(
            if bypass {
                "scouted_detour"
            } else {
                "visible_supplies_and_full_enemy_retry"
            }
            .into(),
        );
        result.score += abandon_cost(input, s)
            - s.weights["retreat"]
            - if bypass { s.weights["detour"] } else { 0. };
    }
    Ok(())
}
