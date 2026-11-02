//! C: visible training gates. Never changes host eligibility or game difficulty.
use super::{Context, Observation, VisibleHero};
use crate::campaign017::QuestView;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tier {
    pub minimum: u32,
    pub gear_floor: u32,
    pub gear_cap: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub tiers: BTreeMap<u32, Tier>,
    pub court_minimum: BTreeMap<u32, u32>,
    pub court_gear: BTreeMap<u32, u32>,
    pub champion_after_week: u32,
    pub apprentice_required_one_level: u32,
    pub allow_all_zero_rebuild_short: bool,
    pub ordinary_regions: Vec<String>,
    pub length_priority: BTreeMap<u32, u32>,
    pub ignored_skills: Vec<String>,
    pub notes: String,
}
impl Settings {
    pub fn validate(&self) -> bool {
        !self.sources.is_empty()
            && self
                .tiers
                .values()
                .all(|t| t.gear_floor <= t.gear_cap && t.gear_cap <= 4)
            && [1, 3, 5, 6].iter().all(|d| self.tiers.contains_key(d))
    }
    pub fn minimum(&self, q: &QuestView) -> u32 {
        if q.plot
            .as_ref()
            .is_some_and(|p| p.starts_with("plot_crimson_court_"))
        {
            self.court_minimum
                .get(&q.difficulty)
                .copied()
                .unwrap_or(u32::MAX)
        } else {
            self.tiers
                .get(&q.difficulty)
                .map_or(u32::MAX, |t| t.minimum)
        }
    }
}
pub fn hero_ready(ctx: &Context, h: &VisibleHero, q: &QuestView) -> bool {
    let cfg = &ctx.profile.gates;
    let Some(t) = cfg.tiers.get(&q.difficulty) else {
        return false;
    };
    let target = if q
        .plot
        .as_ref()
        .is_some_and(|p| p.starts_with("plot_crimson_court_"))
    {
        cfg.court_gear.get(&q.difficulty).copied().unwrap_or(4)
    } else {
        h.level.min(t.gear_cap).max(t.gear_floor)
    };
    let skills = ctx.selected(h);
    let lowest = skills
        .iter()
        .filter(|id| !cfg.ignored_skills.contains(id))
        .filter(|id| {
            ctx.catalog
                .skills
                .get(&h.class)
                .and_then(|c| c.get(*id))
                .is_some_and(|s| s.available_at_start)
        })
        .map(|id| *h.skills.get(id).unwrap_or(&0))
        .min()
        .unwrap_or(0);
    h.level >= cfg.minimum(q)
        && q.maximum_resolve_level.is_none_or(|max| h.level <= max)
        && h.weapon >= target as usize
        && h.armour >= target as usize
        && lowest >= target as i32
}
pub fn complete(
    ctx: &Context,
    t: &Observation,
    q: &QuestView,
    week: u32,
    mut p: Vec<u64>,
) -> Vec<u64> {
    let cfg = &ctx.profile.gates;
    if p.is_empty()
        || (q.length > 1 && p.len() < 4)
        || (q.difficulty == 5 && week < cfg.champion_after_week)
        || p.iter().any(|id| {
            !t.heroes
                .iter()
                .any(|h| h.guid == *id && hero_ready(ctx, h, q))
        })
    {
        return vec![];
    }
    if q.difficulty == 1
        && !q
            .plot
            .as_ref()
            .is_some_and(|s| s.starts_with("plot_crimson_court_"))
        && !p.iter().any(|id| {
            t.heroes
                .iter()
                .any(|h| h.guid == *id && h.level >= cfg.apprentice_required_one_level)
        })
    {
        let mut leaders = t
            .heroes
            .iter()
            .filter(|h| {
                h.alive
                    && !h.busy
                    && !h.missing
                    && h.level >= cfg.apprentice_required_one_level
                    && hero_ready(ctx, h, q)
            })
            .collect::<Vec<_>>();
        leaders.sort_by(|a, b| {
            ctx.power(b)
                .total_cmp(&ctx.power(a))
                .then_with(|| a.guid.cmp(&b.guid))
        });
        for h in leaders {
            let mut slots = (0..p.len()).collect::<Vec<_>>();
            slots.sort_by(|a, b| {
                let old = |i: usize| t.heroes.iter().find(|x| x.guid == p[i]).unwrap();
                usize::from(old(*a).class != h.class)
                    .cmp(&usize::from(old(*b).class != h.class))
                    .then_with(|| ctx.power(old(*a)).total_cmp(&ctx.power(old(*b))))
            });
            for slot in slots {
                let mut other = p.clone();
                other.remove(slot);
                if super::composition::compatible(t, &other, h) {
                    p[slot] = h.guid;
                    return ctx.arrange(t, p);
                }
            }
        }
        if !(cfg.allow_all_zero_rebuild_short
            && q.length == 1
            && cfg.ordinary_regions.contains(&q.dungeon)
            && t.heroes
                .iter()
                .filter(|h| h.alive && !h.busy && !h.missing)
                .all(|h| h.level < cfg.apprentice_required_one_level))
        {
            return vec![];
        }
    }
    p
}
