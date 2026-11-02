//! Observation-only G instrumentation. Nothing here proposes an action.
use super::Context;
use crate::campaign017::Observation;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Visit {
    pub first_gold: u32,
    pub minimum_gold: u32,
    pub poor: bool,
    pub has_antiquarian: bool,
    pub three_escorts: bool,
    pub suitable_quest: bool,
    pub escort_witness: Vec<u64>,
    pub quest_witness: Option<String>,
}
fn strength_triple(ctx: &Context, o: &Observation) -> Option<Vec<u64>> {
    let available =
        |h: &&crate::town016::VisibleHero| h.alive && !h.busy && !h.missing && h.stress < 70;
    let cfg = &ctx.profile.merchant;
    for difficulty in cfg.escort_power.keys() {
        let candidates = o
            .town
            .heroes
            .iter()
            .filter(available)
            .filter(|h| {
                h.class != "antiquarian"
                    && h.level >= difficulty.saturating_sub(1)
                    && ctx.power(h) >= cfg.escort_power[difficulty]
                    && h.weapon >= cfg.escort_gear[difficulty]
                    && h.armour >= cfg.escort_gear[difficulty]
                    && super::strength(h)[3] >= cfg.escort_skills[difficulty]
            })
            .collect::<Vec<_>>();
        for ant in o
            .town
            .heroes
            .iter()
            .filter(available)
            .filter(|h| h.class == "antiquarian" && h.level >= difficulty.saturating_sub(1))
        {
            for a in 0..candidates.len() {
                for b in a + 1..candidates.len() {
                    for c in b + 1..candidates.len() {
                        let mut ids = vec![ant.guid];
                        for h in [candidates[a], candidates[b], candidates[c]] {
                            if super::composition::compatible(&o.town, &ids, h) {
                                ids.push(h.guid);
                            }
                        }
                        if ids.len() == 4 {
                            return Some(ids);
                        }
                    }
                }
            }
        }
    }
    None
}
pub fn observe(
    ctx: &Context,
    o: &Observation,
    blocked: &BTreeSet<String>,
    visits: &mut BTreeMap<u32, Visit>,
) {
    let gold = o.town.wallet.get("gold:").copied().unwrap_or(0);
    let visit = visits.entry(o.returns).or_insert_with(|| Visit {
        first_gold: gold,
        minimum_gold: gold,
        ..Default::default()
    });
    visit.minimum_gold = visit.minimum_gold.min(gold);
    if gold >= ctx.profile.merchant.poverty_gold {
        return;
    }
    visit.poor = true;
    if !o
        .town
        .heroes
        .iter()
        .any(|h| h.alive && h.class == "antiquarian")
    {
        return;
    }
    visit.has_antiquarian = true;
    if !visit.three_escorts {
        if let Some(ids) = strength_triple(ctx, o) {
            visit.three_escorts = true;
            visit.escort_witness = ids;
        } else {
            return;
        }
    }
    if visit.suitable_quest {
        return;
    }
    for q in o.board.as_ref().into_iter().flat_map(|b| &b.quests) {
        if q.execution == "expedition integration pending"
            || blocked.contains(q.plot.as_ref().unwrap_or(&q.id))
            || !q
                .estate_requirements
                .iter()
                .all(|(id, n)| o.town.wallet.get(id).copied().unwrap_or(0) >= *n)
        {
            continue;
        }
        let party = super::policy::party(o, q, false, ctx);
        if super::merchant::eligible(ctx, &o.town, q, &party) {
            visit.suitable_quest = true;
            visit.quest_witness = Some(q.id.clone());
            break;
        }
    }
}
pub fn summary(visits: &BTreeMap<u32, Visit>) -> serde_json::Value {
    serde_json::json!({"poor_returns":visits.values().filter(|v|v.poor).count(),
        "with_antiquarian":visits.values().filter(|v|v.has_antiquarian).count(),
        "with_three_escorts":visits.values().filter(|v|v.three_escorts).count(),
        "with_suitable_quest":visits.values().filter(|v|v.suitable_quest).count(),"visits":visits,
        "basis":"Once per return if reached at any town preparation step; strength-only escort stage is independent of board availability. Final stage also checks actual visible task, level cap, C gates, recovery and estate requirements. No RNG, queue or future map."})
}
