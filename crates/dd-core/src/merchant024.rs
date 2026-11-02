//! G: three trained escorts with an antiquarian; public preparation only.
use super::{Context, Observation, VisibleHero};
use crate::campaign017::QuestView;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub poverty_gold: u32,
    pub escort_power: BTreeMap<u32, f64>,
    pub escort_gear: BTreeMap<u32, usize>,
    pub escort_skills: BTreeMap<u32, f64>,
    pub candidate_limit: usize,
}
fn available(h: &VisibleHero, q: &QuestView) -> bool {
    h.alive
        && !h.busy
        && !h.missing
        && h.stress < 70
        && h.level >= q.difficulty.saturating_sub(1)
        && q.maximum_resolve_level.is_none_or(|max| h.level <= max)
}
pub fn eligible(ctx: &Context, t: &Observation, q: &QuestView, ids: &[u64]) -> bool {
    let cfg = &ctx.profile.merchant;
    if t.wallet.get("gold:").copied().unwrap_or(0) >= cfg.poverty_gold
        || !super::economy::ordinary(q)
        || ![1, 2].contains(&q.length)
        || ![1, 3].contains(&q.difficulty)
        || ids.len() != 4
    {
        return false;
    }
    let mut merchants = 0;
    for (i, id) in ids.iter().enumerate() {
        let Some(h) = t.heroes.iter().find(|h| h.guid == *id) else {
            return false;
        };
        if !available(h, q) || !super::composition::compatible(t, &ids[..i], h) {
            return false;
        }
        if h.class == "antiquarian" {
            merchants += 1;
        } else if ctx.power(h) < cfg.escort_power[&q.difficulty]
            || h.weapon < cfg.escort_gear[&q.difficulty]
            || h.armour < cfg.escort_gear[&q.difficulty]
            || super::strength(h)[3] < cfg.escort_skills[&q.difficulty]
        {
            return false;
        }
    }
    let e = super::composition::evaluate(ctx, t, q, ids);
    merchants == 1
        && (q.length == 1
            || (e.healing >= ctx.profile.composition.medium_min_heal
                && e.relief >= ctx.profile.composition.medium_min_relief
                && e.control >= ctx.profile.composition.medium_min_control))
}
pub fn party(ctx: &Context, t: &Observation, q: &QuestView) -> Option<Vec<u64>> {
    if t.wallet.get("gold:").copied().unwrap_or(0) >= ctx.profile.merchant.poverty_gold
        || !super::economy::ordinary(q)
        || ![1, 2].contains(&q.length)
        || ![1, 3].contains(&q.difficulty)
    {
        return None;
    }
    let mut escorts = t
        .heroes
        .iter()
        .filter(|h| h.class != "antiquarian" && available(h, q))
        .collect::<Vec<_>>();
    escorts.sort_by(|a, b| {
        ctx.power(b)
            .total_cmp(&ctx.power(a))
            .then_with(|| a.guid.cmp(&b.guid))
    });
    escorts.truncate(ctx.profile.merchant.candidate_limit);
    let mut best: Option<(f64, Vec<u64>)> = None;
    for ant in t
        .heroes
        .iter()
        .filter(|h| h.class == "antiquarian" && available(h, q))
    {
        for a in 0..escorts.len() {
            for b in a + 1..escorts.len() {
                for c in b + 1..escorts.len() {
                    let ids = ctx.arrange_known(
                        t,
                        vec![escorts[a].guid, escorts[b].guid, escorts[c].guid, ant.guid],
                    );
                    if !eligible(ctx, t, q, &ids) {
                        continue;
                    }
                    let e = super::composition::evaluate(ctx, t, q, &ids);
                    let score = e.power * ctx.profile.composition.weights.power
                        + e.healing * ctx.profile.composition.weights.healing
                        + e.relief * ctx.profile.composition.weights.relief
                        + e.control * ctx.profile.composition.weights.control
                        + ids
                            .iter()
                            .enumerate()
                            .filter_map(|(i, id)| {
                                t.heroes
                                    .iter()
                                    .find(|h| h.guid == *id)
                                    .map(|h| ctx.rank_score(h, (i + 1) as u8))
                            })
                            .sum::<f64>();
                    if best.as_ref().is_none_or(|b| score > b.0) {
                        best = Some((score, ids));
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}
