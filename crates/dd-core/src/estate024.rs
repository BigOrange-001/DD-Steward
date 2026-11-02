//! F minimum scope: recruitment, portraits, deeds and fatal negative quirks.
use super::{Context, Observation};
use crate::town016::{Action, Treatment};
use dd_data::{
    combat::{GameData, Result},
    library,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub recruit_tree: String,
    pub recruit_levels: usize,
    pub ordered_trees: Vec<String>,
    pub reserve_gold: u32,
    pub skip_turn_quirks: BTreeSet<String>,
    pub deferred: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuirkCard {
    pub negative: bool,
    pub accuracy: bool,
    pub dot_resistance: bool,
    pub curio: bool,
    pub skip_turn: bool,
}
pub fn refresh(
    game: &GameData,
    t: &Observation,
    cfg: &Settings,
    cards: &mut BTreeMap<String, QuirkCard>,
) -> Result<()> {
    let ids = t
        .heroes
        .iter()
        .flat_map(|h| h.quirks.keys())
        .filter(|id| !cards.contains_key(*id))
        .cloned()
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(());
    }
    let lib = game.quirks()?;
    for id in ids {
        let q = library::unique_json(&lib, &id, "visible quirk")?;
        let mut card = QuirkCard {
            negative: q["is_positive"].as_bool() == Some(false)
                && q["is_disease"].as_bool() != Some(true),
            curio: q["curio_tag_chance"].as_f64().unwrap_or(0.) > 0.,
            skip_turn: cfg.skip_turn_quirks.contains(&id),
            ..Default::default()
        };
        for bid in q["buffs"].as_array().into_iter().flatten() {
            let b = library::unique_json(
                &game.buffs,
                bid.as_str().ok_or("invalid public quirk buff")?,
                "public quirk buff",
            )?;
            if b["amount"].as_f64().unwrap_or(0.) >= 0. {
                continue;
            }
            card.accuracy |=
                b["stat_type"] == "combat_stat_add" && b["stat_sub_type"] == "attack_rating";
            card.dot_resistance |= b["stat_type"] == "resistance"
                && ["bleed", "poison"].iter().any(|s| b["stat_sub_type"] == *s);
        }
        cards.insert(id, card);
    }
    Ok(())
}
pub fn recruit_ready(ctx: &Context, t: &Observation) -> bool {
    t.purchased
        .iter()
        .filter(|(owner, id, _)| owner.is_none() && id == &ctx.profile.estate.recruit_tree)
        .count()
        >= ctx.profile.estate.recruit_levels
}
fn affordable(t: &Observation, c: &crate::town016::Choice, reserve: u32) -> bool {
    c.cost.iter().all(|(key, n)| {
        t.wallet
            .get(key)
            .copied()
            .unwrap_or(0)
            .saturating_sub(if key == "gold:" { reserve } else { 0 })
            >= *n
    })
}
pub fn recruit(ctx: &Context, t: &Observation) -> Option<Action> {
    if recruit_ready(ctx, t) {
        return None;
    }
    t.actions.iter().find(|c|matches!(&c.action,Action::Upgrade{hero:None,tree,..} if tree==&ctx.profile.estate.recruit_tree)&&affordable(t,c,ctx.profile.estate.reserve_gold)).map(|c|c.action.clone())
}
pub fn building(ctx: &Context, t: &Observation) -> Option<Action> {
    let target = t
        .heroes
        .iter()
        .filter(|h| h.alive && t.party.contains(&h.guid))
        .map(|h| h.level)
        .max()
        .unwrap_or(0)
        .clamp(1, 4) as usize;
    for tree in &ctx.profile.estate.ordered_trees {
        let bought = t
            .purchased
            .iter()
            .filter(|(owner, id, _)| owner.is_none() && id == tree)
            .count();
        if bought >= target {
            continue;
        }
        if let Some(c) = t.actions.iter().find(|c| {
            matches!(&c.action,Action::Upgrade{hero:None,tree:id,..} if id==tree)
                && affordable(t, c, ctx.profile.estate.reserve_gold)
        }) {
            return Some(c.action.clone());
        }
    }
    None
}
pub fn treatment(ctx: &Context, t: &Observation) -> Option<Action> {
    if t.heroes
        .iter()
        .filter(|h| h.alive && !h.busy && !h.missing)
        .count()
        <= 4
    {
        return None;
    }
    let healer = |h: &crate::town016::VisibleHero| {
        ctx.selected(h).iter().any(|id| {
            ctx.catalog
                .skills
                .get(&h.class)
                .and_then(|s| s.get(id))
                .is_some_and(|a| {
                    a.available_at_start
                        && !a.conditional_healing
                        && (a.healing > 0. || a.healing_percent > 0.)
                })
        })
    };
    t.actions
        .iter()
        .filter(|c| affordable(t, c, ctx.profile.estate.reserve_gold))
        .find_map(|c| {
            let Action::Queue {
                hero,
                treatment: Treatment::RemoveQuirk(id),
                ..
            } = &c.action
            else {
                return None;
            };
            let h = t
                .heroes
                .iter()
                .find(|h| h.guid == *hero && h.alive && !h.busy && !h.missing)?;
            let card = ctx.catalog.quirks.get(id)?;
            if !card.negative
                || !(card.accuracy || card.dot_resistance || card.curio || card.skip_turn)
            {
                return None;
            }
            if healer(h)
                && !t.heroes.iter().any(|other| {
                    other.guid != *hero
                        && other.alive
                        && !other.busy
                        && !other.missing
                        && healer(other)
                })
            {
                return None;
            }
            Some(c.action.clone())
        })
}
