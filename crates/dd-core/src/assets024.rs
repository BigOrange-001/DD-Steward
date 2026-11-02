//! Public tooltip projection. Only observed inventory/shop/reward IDs are supplied.
use super::Context;
use crate::town016::VisibleHero;
use dd_data::{
    combat::{GameData, Result},
    library,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Card {
    pub buffs: Vec<serde_json::Value>,
    pub classes: Vec<String>,
    pub special: bool,
}
pub fn project(game: &GameData, id: &str) -> Result<Card> {
    let lib = library::json_library(
        &game.overlay,
        "trinkets/",
        ".entries.trinkets.json",
        "entries",
    )?;
    let raw = library::unique_json(&lib, id, "visible trinket")?;
    let mut c = Card {
        classes: raw["hero_class_requirements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str().map(String::from))
            .collect(),
        special: raw.as_object().is_some_and(|m| {
            m.keys()
                .any(|k| k.contains("effect") || k.starts_with("transform_") || k == "quest_uses")
        }),
        ..Default::default()
    };
    for bid in raw["buffs"].as_array().into_iter().flatten() {
        let id = bid.as_str().ok_or("024 invalid public buff reference")?;
        let b = library::unique_json(&game.buffs, id, "public trinket buff")?.clone();
        let rule = b["rule_type"].as_str().unwrap_or("always");
        let stat = b["stat_type"].as_str().unwrap_or("");
        let sub = b["stat_sub_type"].as_str().unwrap_or("");
        let known = match stat {
            "combat_stat_add" => [
                "attack_rating",
                "crit_chance",
                "defense_rating",
                "speed_rating",
            ]
            .contains(&sub),
            "combat_stat_multiply" => ["damage_low", "damage_high", "max_hp"].contains(&sub),
            "resistance" => ["bleed", "poison", "disease"].contains(&sub),
            "scouting_chance"
            | "stun_chance"
            | "hp_heal_percent"
            | "stress_heal_percent"
            | "move_chance"
            | "debuff_chance"
            | "stress_dmg_received_percent" => true,
            _ => false,
        };
        c.special |= !known || !["always", "lightbelow", "lightabove"].contains(&rule);
        c.buffs.push(b);
    }
    Ok(c)
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub weights: BTreeMap<String, f64>,
    pub planning_light: f64,
    pub stress_penalty_with_relief: f64,
    pub named_bias: BTreeMap<String, BTreeMap<String, f64>>,
    pub focus_reward_priority: u32,
}
fn active(b: &serde_json::Value, light: f64) -> bool {
    let threshold = b["rule_data"]["float"].as_f64().unwrap_or(0.);
    let condition = match b["rule_type"].as_str().unwrap_or("always") {
        "always" => true,
        "lightbelow" => light < threshold,
        "lightabove" => light > threshold,
        _ => return false,
    };
    condition != b["is_false_rule"].as_bool().unwrap_or(false)
}
pub fn score(ctx: &Context, h: &VisibleHero, id: &str, c: &Card) -> f64 {
    if !c.classes.is_empty() && !c.classes.contains(&h.class) {
        return 0.;
    }
    let cfg = &ctx.profile.assets;
    let mut healing = 0f64;
    let mut relief = 0f64;
    let mut control = 0f64;
    let mut damage = 0f64;
    for s in ctx.selected(h) {
        if let Some(a) = ctx
            .catalog
            .skills
            .get(&h.class)
            .and_then(|skills| skills.get(&s))
            .filter(|a| a.available_at_start)
        {
            healing = healing.max(a.healing + a.healing_percent * f64::from(h.stats.hp));
            relief = relief.max(a.stress_relief);
            control = control.max(a.control);
            damage = damage.max(a.damage);
        }
    }
    let w = |key: &str| cfg.weights.get(key).copied().unwrap_or(0.);
    let mean = f64::from(h.stats.damage[0] + h.stats.damage[1]) / 2.;
    let crit_marginal = (f64::from(h.stats.damage[1]) * ctx.catalog.crit_multiplier - mean).max(0.);
    let mut score = 0.;
    for b in c.buffs.iter().filter(|b| active(b, cfg.planning_light)) {
        let amount = b["amount"].as_f64().unwrap_or(0.);
        let stat = b["stat_type"].as_str().unwrap_or("");
        let sub = b["stat_sub_type"].as_str().unwrap_or("");
        let role = match stat {
            "hp_heal_percent" => f64::from(healing > 0.),
            "stress_heal_percent" => f64::from(relief > 0.),
            "stun_chance" => f64::from(control > 0.),
            "stress_dmg_received_percent" if !ctx.role_gaps.contains("stress_relief") => {
                cfg.stress_penalty_with_relief
            }
            _ => 1.,
        };
        score += amount
            * role
            * match (stat, sub) {
                ("combat_stat_add", "crit_chance") => {
                    if damage > 0. {
                        crit_marginal * w("crit_expected_damage")
                    } else {
                        0.
                    }
                }
                ("combat_stat_add", "attack_rating") => w("accuracy"),
                ("combat_stat_add", "defense_rating") => w("dodge"),
                ("combat_stat_add", "speed_rating") => w("speed"),
                ("combat_stat_multiply", "damage_low" | "damage_high") => mean * w("damage") / 2.,
                ("combat_stat_multiply", "max_hp") => f64::from(h.stats.hp) * w("hp"),
                ("resistance", "bleed" | "poison" | "disease") => w("resistance"),
                _ => w(stat),
            };
    }
    if let Some(bias) = cfg.named_bias.get(id) {
        score += bias
            .get(&h.class)
            .or_else(|| bias.get("*"))
            .copied()
            .unwrap_or(0.);
    }
    score
}
pub fn visible_ids(o: &crate::campaign017::Observation) -> Vec<String> {
    o.town
        .trinkets
        .iter()
        .map(|(_, id, _)| id.clone())
        .chain(o.town.wagon.iter().map(|(_, id, _)| id.clone()))
        .chain(
            o.board
                .iter()
                .flat_map(|b| b.quests.iter())
                .flat_map(|q| q.extra_rewards.clone()),
        )
        .collect()
}
pub fn refresh(
    game: &GameData,
    o: &crate::campaign017::Observation,
    cache: &mut BTreeMap<String, Card>,
) -> Result<()> {
    for id in visible_ids(o) {
        if !cache.contains_key(&id) {
            cache.insert(id.clone(), project(game, &id)?);
        }
    }
    Ok(())
}

/// Finite town assignment; no raid equipment, instance creation, or state reset.
pub fn equip(ctx: &Context, t: &crate::town016::Observation) -> Option<crate::town016::Action> {
    use crate::town016::Action;
    t.actions
        .iter()
        .filter_map(|choice| {
            let Action::Equip {
                hero,
                instance,
                slot,
            } = choice.action
            else {
                return None;
            };
            let h = t
                .heroes
                .iter()
                .find(|h| h.guid == hero && t.party.contains(&hero))?;
            if h.equipped.iter().any(|(s, _, _)| *s == slot) {
                return None;
            }
            let (_, id, owner) = t.trinkets.iter().find(|(i, _, _)| *i == instance)?;
            if owner.is_some() || id == "dd_trinket" {
                return None;
            }
            let card = ctx.catalog.assets.get(id)?;
            Some((
                score(ctx, h, id, card),
                hero,
                instance,
                slot,
                choice.action.clone(),
            ))
        })
        .filter(|a| a.0 > 0.)
        .max_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| b.3.cmp(&a.3))
        })
        .map(|a| a.4)
}
/// An entire return boundary is unloaded once. Locked/busy source actions are
/// left to the host; a later return will retry those physical instances.
pub fn unload(
    t: &crate::town016::Observation,
    returns: u32,
    m: &mut super::economy::Memory,
) -> Option<crate::town016::Action> {
    if m.unloaded_return == Some(returns) {
        return None;
    }
    if let Some(c) = t
        .actions
        .iter()
        .find(|c| matches!(c.action, crate::town016::Action::Unequip { .. }))
    {
        return Some(c.action.clone());
    }
    m.unloaded_return = Some(returns);
    None
}
pub fn focus_reward(
    ctx: &Context,
    t: &crate::town016::Observation,
    q: &crate::campaign017::QuestView,
    week: u32,
    party: &[u64],
) -> bool {
    q.kind == "kill_boss"
        && q.extra_rewards.iter().any(|id| id == "focus_ring")
        && party.len() == 4
        && super::gates::complete(ctx, t, q, week, party.to_vec()).as_slice() == party
}
