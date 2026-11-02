//! B: finite preparation search using public skill cards and visible roster only.
use super::{Context, Observation, VisibleHero};
use crate::campaign017::QuestView;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub name: String,
    pub slots: Vec<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Weights {
    pub damage: f64,
    pub healing: f64,
    pub relief: f64,
    pub control: f64,
    pub guard: f64,
    pub shuffle: f64,
    pub power: f64,
    pub training_cost: f64,
    pub region: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub templates: Vec<Template>,
    pub beam_width: usize,
    pub candidates_per_template_slot: usize,
    pub weights: Weights,
    pub medium_min_heal: f64,
    pub medium_min_relief: f64,
    pub medium_min_control: f64,
    pub region_resists: BTreeMap<String, [f64; 2]>,
    pub region_basis: String,
    pub wealthy_gold: u32,
    pub emergency_short_min_party: usize,
}
impl Settings {
    pub fn validate(&self) -> bool {
        !self.sources.is_empty()
            && self.id.starts_with("P024-")
            && (1..=128).contains(&self.beam_width)
            && (1..=16).contains(&self.candidates_per_template_slot)
            && (1..=4).contains(&self.emergency_short_min_party)
            && self.templates.iter().all(|t| t.slots.len() == 4)
            && self
                .region_resists
                .values()
                .flatten()
                .all(|n| n.is_finite() && *n >= 0.)
    }
}
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Evaluation {
    pub damage: f64,
    pub healing: f64,
    pub relief: f64,
    pub control: f64,
    pub guard: f64,
    pub shuffle: f64,
    pub power: f64,
    pub training_cost: f64,
    pub region: f64,
}
pub fn compatible(t: &Observation, ids: &[u64], h: &VisibleHero) -> bool {
    !ids.contains(&h.guid)
        && ids
            .iter()
            .filter_map(|id| t.heroes.iter().find(|other| other.guid == *id))
            .all(|other| {
                !h.incompatible_party_tags
                    .iter()
                    .any(|s| other.party_tags.contains(s))
                    && !other
                        .incompatible_party_tags
                        .iter()
                        .any(|s| h.party_tags.contains(s))
            })
}
pub fn evaluate(ctx: &Context, t: &Observation, q: &QuestView, ids: &[u64]) -> Evaluation {
    let mut e = Evaluation::default();
    let cfg = &ctx.profile.composition;
    for (slot, id) in ids.iter().enumerate() {
        let Some(h) = t.heroes.iter().find(|h| h.guid == *id) else {
            continue;
        };
        let selected = ctx.selected(h);
        let mut available = 0.;
        for skill in &selected {
            let Some(a) = ctx
                .catalog
                .skills
                .get(&h.class)
                .and_then(|s| s.get(skill))
                .filter(|a| a.available_at_start)
            else {
                continue;
            };
            e.shuffle += a.launch.len() as f64 / 4.;
            if !a.launch.contains(&((slot + 1) as u8)) {
                continue;
            }
            available += 1.;
            e.damage += a.damage;
            if !a.conditional_healing {
                e.healing = e
                    .healing
                    .max(a.healing + a.healing_percent * f64::from(h.stats.hp));
            }
            e.relief = e.relief.max(a.stress_relief);
            e.control += a.control;
            e.guard += if a.guard { 1. } else { 0. };
            if let Some(resists) = cfg.region_resists.get(&q.dungeon) {
                if a.bleed {
                    e.region += 1. - resists[0].clamp(0., 1.)
                }
                if a.blight {
                    e.region += 1. - resists[1].clamp(0., 1.)
                }
            }
        }
        if !selected.is_empty() {
            e.shuffle += available / selected.len() as f64
        }
        e.power += ctx.power(h);
        if t.wallet.get("gold:").copied().unwrap_or(0) < cfg.wealthy_gold {
            e.training_cost+=t.upgrade_quotes.iter().filter(|c|matches!(&c.action,crate::town016::Action::Upgrade{hero:Some(id),..} if *id==h.guid))
                .map(|c|f64::from(c.cost.get("gold:").copied().unwrap_or(0))).sum::<f64>();
        }
    }
    e
}
fn score(ctx: &Context, e: &Evaluation) -> f64 {
    let w = &ctx.profile.composition.weights;
    e.damage * w.damage
        + e.healing * w.healing
        + e.relief * w.relief
        + e.control * w.control
        + e.guard * w.guard
        + e.shuffle * w.shuffle
        + e.power * w.power
        - e.training_cost * w.training_cost
        + e.region * w.region
}
fn qualified(ctx: &Context, q: &QuestView, e: &Evaluation) -> bool {
    let cfg = &ctx.profile.composition;
    q.length == 1
        || (e.healing >= cfg.medium_min_heal
            && e.relief >= cfg.medium_min_relief
            && e.control >= cfg.medium_min_control)
}
fn available(h: &VisibleHero) -> bool {
    h.alive && !h.busy && !h.missing
}
fn template(ctx: &Context, t: &Observation, q: &QuestView, rule: &Template) -> Option<Vec<u64>> {
    fn grow(
        ctx: &Context,
        t: &Observation,
        q: &QuestView,
        rule: &Template,
        ids: &mut Vec<u64>,
        best: &mut Option<(f64, Vec<u64>)>,
    ) {
        if ids.len() == 4 {
            let e = evaluate(ctx, t, q, ids);
            let s = score(ctx, &e);
            if qualified(ctx, q, &e) && best.as_ref().is_none_or(|b| s > b.0) {
                *best = Some((s, ids.clone()))
            }
            return;
        }
        let slot = ids.len();
        let classes = &rule.slots[slot];
        let mut choices = t
            .heroes
            .iter()
            .filter(|h| {
                available(h)
                    && (classes.is_empty() || classes.contains(&h.class))
                    && compatible(t, ids, h)
            })
            .collect::<Vec<_>>();
        choices.sort_by(|a, b| {
            let value = |h: &VisibleHero| {
                ctx.rank_score(h, (slot + 1) as u8)
                    + ctx.power(h) * ctx.profile.composition.weights.power
                    - f64::from(h.stress)
            };
            value(b)
                .total_cmp(&value(a))
                .then_with(|| a.guid.cmp(&b.guid))
        });
        for h in choices
            .into_iter()
            .take(ctx.profile.composition.candidates_per_template_slot)
        {
            ids.push(h.guid);
            grow(ctx, t, q, rule, ids, best);
            ids.pop();
        }
    }
    let mut best = None;
    grow(ctx, t, q, rule, &mut vec![], &mut best);
    best.map(|b| b.1)
}
pub fn party(ctx: &Context, t: &Observation, q: &QuestView) -> Vec<u64> {
    for rule in &ctx.profile.composition.templates {
        if let Some(ids) = template(ctx, t, q, rule) {
            return ids;
        }
    }
    let count = t.heroes.iter().filter(|h| available(h)).count().min(4);
    if count == 0
        || (q.length > 1 && count < 4)
        || count < ctx.profile.composition.emergency_short_min_party
    {
        return vec![];
    }
    let mut beam = vec![vec![]];
    for _ in 0..count {
        let mut next = vec![];
        for p in &beam {
            for h in t
                .heroes
                .iter()
                .filter(|h| available(h) && compatible(t, p, h))
            {
                let mut ids = p.clone();
                ids.push(h.guid);
                next.push(ids)
            }
        }
        next.sort_by(|a, b| {
            score(ctx, &evaluate(ctx, t, q, b))
                .total_cmp(&score(ctx, &evaluate(ctx, t, q, a)))
                .then_with(|| a.cmp(b))
        });
        next.truncate(ctx.profile.composition.beam_width);
        beam = next;
    }
    beam.into_iter()
        .find(|p| qualified(ctx, q, &evaluate(ctx, t, q, p)))
        .unwrap_or_default()
}
