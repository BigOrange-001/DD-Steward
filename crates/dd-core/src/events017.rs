//! Campaign clock and town-event arbitration. Host state, never policy observation.
use crate::Rng;
use dd_data::{
    campaign017::{Data, array, uint, validate_event},
    combat::Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    pub week: u32,
    pub dead_heroes: u32,
    pub hero_levels: BTreeMap<u32, u32>,
    pub purchases: BTreeSet<(String, String)>,
    pub stored_trinkets: u32,
    pub available_plots: BTreeSet<String>,
    #[serde(default)]
    pub open_event_plots: BTreeSet<String>,
    pub finished_plots: BTreeSet<String>,
    pub infestation: String,
    pub available_districts: u32,
    pub unlocked_buildings: BTreeSet<String>,
}
impl Facts {
    fn eligible(&self, e: &Value) -> Result<bool> {
        validate_event(e)?;
        let r = &e["requirements"];
        if self.week < uint(r, "minimum_week")?
            || self.dead_heroes < uint(r, "dead_heroes")?
            || self.stored_trinkets < uint(r, "trinket_storage_count")?
        {
            return Ok(false);
        }
        for row in array(r, "hero_level_counts")? {
            if self
                .hero_levels
                .get(&uint(row, "level")?)
                .copied()
                .unwrap_or(0)
                < uint(row, "count")?
            {
                return Ok(false);
            }
        }
        for row in array(r, "upgrades_purchased")? {
            if !self.purchases.contains(&(
                row["tree_id"]
                    .as_str()
                    .ok_or("missing required upgrade tree")?
                    .into(),
                row["requirement_code"]
                    .as_str()
                    .ok_or("missing required upgrade code")?
                    .into(),
            )) {
                return Ok(false);
            }
        }
        if r["available_plot_quest"]
            .as_str()
            .is_some_and(|id| !self.available_plots.contains(id))
        {
            return Ok(false);
        }
        if r["building_unlocked"]
            .as_str()
            .is_some_and(|id| !self.unlocked_buildings.contains(id))
        {
            return Ok(false);
        }
        if let Some(rows) = r["finished_plot_quests"].as_array() {
            for id in rows {
                if !self
                    .finished_plots
                    .contains(id.as_str().ok_or("invalid plot requirement")?)
                {
                    return Ok(false);
                }
            }
        }
        if let Some(rows) = r["infestation_sequences"].as_array()
            && !rows
                .iter()
                .any(|id| id.as_str() == Some(self.infestation.as_str()))
        {
            return Ok(false);
        }
        if let Some(n) = r.get("minimum_number_of_district_buildings")
            && self.available_districts
                < n.as_u64()
                    .ok_or("invalid district availability requirement")? as u32
        {
            return Ok(false);
        }
        Ok(true)
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Counter {
    last: Option<u32>,
    missed: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Guarantee {
    pub event: String,
    pub cause: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    history: BTreeMap<String, Counter>,
    quiet_visits: u32,
    pending: VecDeque<Guarantee>,
    causes: BTreeSet<String>,
    item_opportunity: bool,
    last_week: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub event: String,
    pub cause: String,
}
fn priority(v: &Value) -> Result<u32> {
    match v["priority"].as_str().unwrap_or("normal") {
        "normal" => Ok(0),
        "high" => Ok(1),
        "higher" => Ok(2),
        "highest" => Ok(3),
        _ => Err("unsupported event priority".into()),
    }
}
fn weighted(rng: &mut Rng, w: &[f64]) -> Result<usize> {
    let total = w.iter().sum::<f64>();
    if !total.is_finite() || total <= 0. || w.iter().any(|p| !p.is_finite() || *p < 0.) {
        return Err("invalid event weights".into());
    }
    let mut n = (rng.next() >> 11) as f64 / ((1u64 << 53) as f64) * total;
    for (i, p) in w.iter().enumerate() {
        if n < *p {
            return Ok(i);
        }
        n -= p;
    }
    w.iter()
        .rposition(|p| *p > 0.)
        .ok_or("empty event pool".into())
}
impl Runtime {
    pub fn occurred(&self, id: &str) -> bool {
        self.history.get(id).is_some_and(|c| c.last.is_some())
    }
    pub fn queue(&mut self, data: &Data, event: &str, cause: &str) -> Result<()> {
        data.event(event)?;
        if cause.is_empty() || cause.len() > 512 {
            return Err("invalid guaranteed event cause".into());
        }
        if self.causes.contains(cause) {
            return Err("duplicate guaranteed event receipt".into());
        }
        self.causes.insert(cause.into());
        self.pending.push_back(Guarantee {
            event: event.into(),
            cause: cause.into(),
        });
        Ok(())
    }
    pub fn guarantee_opportunity(&mut self) {
        self.item_opportunity = true;
    }
    pub fn validate(&self, data: &Data) -> Result<()> {
        if self.quiet_visits > 1000000 || self.pending.len() > 10000 {
            return Err("event state limit".into());
        }
        for (id, c) in &self.history {
            data.event(id)?;
            if c.last.zip(self.last_week).is_some_and(|(a, b)| a > b) {
                return Err("future event history".into());
            }
        }
        let mut receipts = BTreeSet::new();
        for p in &self.pending {
            data.event(&p.event)?;
            if p.cause.is_empty() || p.cause.len() > 512 {
                return Err("invalid queued event cause".into());
            }
            if !receipts.insert(&p.cause) || !self.causes.contains(&p.cause) {
                return Err("duplicate queued guarantee".into());
            }
        }
        Ok(())
    }
    pub fn validate_at(&self, data: &Data, week: u32) -> Result<()> {
        self.validate(data)?;
        if self.last_week.is_some_and(|w| w > week) {
            return Err("event history is ahead of campaign clock".into());
        }
        Ok(())
    }
    pub(crate) fn select(
        &mut self,
        data: &Data,
        facts: &Facts,
        setting: &str,
        rng: &mut Rng,
    ) -> Result<Option<Selection>> {
        let old = self.clone();
        let old_rng = rng.clone();
        match self.select_inner(data, facts, setting, rng) {
            Ok(v) => Ok(v),
            Err(e) => {
                *self = old;
                *rng = old_rng;
                Err(e)
            }
        }
    }
    fn select_inner(
        &mut self,
        data: &Data,
        f: &Facts,
        setting: &str,
        rng: &mut Rng,
    ) -> Result<Option<Selection>> {
        self.validate(data)?;
        if self.last_week.is_some_and(|w| f.week <= w) {
            return Err("town event week already processed".into());
        }
        let settings =
            dd_data::library::unique_json(&data.settings, setting, "town event setting")?;
        let chance = array(settings, "event_chance_per_town_visits")?;
        let index = (self.quiet_visits as usize).min(chance.len().saturating_sub(1));
        let probability = chance
            .get(index)
            .and_then(Value::as_f64)
            .filter(|p| (0. ..=1.).contains(p))
            .ok_or("invalid town event opportunity probability")?;
        let mut eligible = vec![];
        let mut forced = vec![];
        for id in data.events.keys() {
            let e = data.event(id)?;
            let c = self.history.get(id).cloned().unwrap_or_default();
            let unique = e["is_unique"].as_bool().unwrap_or(false);
            if unique && c.last.is_some() {
                continue;
            }
            let mut blocked_plot = false;
            for effect in array(e, "data")?
                .iter()
                .filter(|v| v["type"] == "plot_quest")
            {
                let id = effect["string_data"]
                    .as_str()
                    .ok_or("missing event plot ID")?;
                let quest =
                    dd_data::library::unique_json(&data.quests, id, "event plot eligibility")?;
                if f.open_event_plots.contains(id)
                    || quest["is_repeatable"] != true && f.finished_plots.contains(id)
                {
                    blocked_plot = true;
                }
            }
            if blocked_plot {
                continue;
            }
            if !f.eligible(e)? {
                continue;
            }
            let cooldown = uint(e, "cooldown")?;
            let cooled = c
                .last
                .is_none_or(|last| last.checked_add(cooldown).is_some_and(|end| f.week > end));
            let p = priority(e)?;
            if p > 0 && cooled {
                forced.push((id.clone(), p));
            }
            if cooled {
                eligible.push(id.clone());
            }
        }
        let mut selected = None;
        if let Some(best) = forced.iter().map(|(_, p)| *p).max() {
            let ids = forced
                .iter()
                .filter(|(_, p)| *p == best)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            let weights = ids
                .iter()
                .map(|id| {
                    let e = data.event(id).unwrap();
                    e["base_chance"].as_f64().unwrap()
                        + e["per_not_rolled_additional_chance"].as_f64().unwrap()
                            * f64::from(self.history.get(id).map_or(0, |c| c.missed))
                })
                .collect::<Vec<_>>();
            let chosen = if weights.iter().sum::<f64>() > 0. {
                weighted(rng, &weights)?
            } else {
                rng.below(ids.len() as u64) as usize
            };
            selected = Some(Selection {
                event: ids[chosen].clone(),
                cause: "eligible_priority_event".into(),
            });
        }
        if selected.is_none() {
            let mut queue_index = None;
            for (i, p) in self.pending.iter().enumerate() {
                let e = data.event(&p.event)?;
                if e["is_unique"] == true
                    && self.history.get(&p.event).is_some_and(|c| c.last.is_some())
                {
                    return Err("guaranteed unique event already consumed".into());
                }
                if f.eligible(e)? {
                    queue_index = Some(i);
                    break;
                }
            }
            if let Some(i) = queue_index {
                let p = self.pending.remove(i).unwrap();
                selected = Some(Selection {
                    event: p.event,
                    cause: p.cause,
                });
            }
        }
        let item = self.item_opportunity;
        self.item_opportunity = false;
        if selected.is_none() {
            let mut pool = vec![];
            for id in &eligible {
                let e = data.event(id)?;
                let c = self.history.get(id).cloned().unwrap_or_default();
                let w = e["base_chance"].as_f64().unwrap()
                    + e["per_not_rolled_additional_chance"].as_f64().unwrap() * f64::from(c.missed);
                if w > 0. {
                    pool.push((id.clone(), priority(e)?, w));
                }
            }
            if !pool.is_empty() && (item || rng.roll(probability)) {
                let best = pool.iter().map(|(_, p, _)| *p).max().unwrap();
                let pool = pool
                    .into_iter()
                    .filter(|(_, p, _)| *p == best)
                    .collect::<Vec<_>>();
                let i = weighted(rng, &pool.iter().map(|(_, _, w)| *w).collect::<Vec<_>>())?;
                selected = Some(Selection {
                    event: pool[i].0.clone(),
                    cause: if item {
                        "quest_item_guaranteed_opportunity"
                    } else {
                        "weekly_opportunity"
                    }
                    .into(),
                });
            }
        }
        for id in &eligible {
            let c = self.history.entry(id.clone()).or_default();
            if selected.as_ref().is_some_and(|s| &s.event == id) {
                c.last = Some(f.week);
                c.missed = 0;
            } else {
                c.missed = c.missed.checked_add(1).ok_or("event growth overflow")?;
            }
        }
        if let Some(s) = &selected {
            let c = self.history.entry(s.event.clone()).or_default();
            c.last = Some(f.week);
            c.missed = 0;
            self.quiet_visits = 0;
        } else {
            self.quiet_visits = self
                .quiet_visits
                .checked_add(1)
                .ok_or("event opportunity counter overflow")?;
        }
        self.last_week = Some(f.week);
        self.validate(data)?;
        Ok(selected)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReturnKind {
    Complete,
    Abandon,
    Regroup,
    Wipe,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReturnReceipt {
    kind: ReturnKind,
    dead: BTreeSet<u64>,
    progression: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    pub week: u32,
    pub returns: u32,
    pub completions: u32,
    pub dead: BTreeSet<u64>,
    receipts: BTreeMap<String, ReturnReceipt>,
    opening_completions: u32,
    opening_dead: BTreeSet<u64>,
}
impl Clock {
    pub fn opening(returns: u32, completions: u32, dead: BTreeSet<u64>) -> Result<Self> {
        if returns != 2 || completions > returns {
            return Err("campaign opening import requires exactly two known returns".into());
        }
        Ok(Self {
            week: returns,
            returns,
            completions,
            opening_dead: dead.clone(),
            dead,
            receipts: BTreeMap::new(),
            opening_completions: completions,
        })
    }
    pub fn total_deaths(&self) -> usize {
        self.opening_dead.len() + self.receipts.values().map(|r| r.dead.len()).sum::<usize>()
    }
    pub fn validate(&self) -> Result<()> {
        let all = self
            .opening_dead
            .iter()
            .chain(self.receipts.values().flat_map(|r| r.dead.iter()))
            .copied()
            .collect::<BTreeSet<_>>();
        if self.dead != all
            || self.receipts.len().checked_add(2) != Some(self.returns as usize)
            || self.opening_completions > 2
            || Some(self.completions)
                != self.opening_completions.checked_add(
                    self.receipts
                        .values()
                        .filter(|r| r.kind == ReturnKind::Complete)
                        .count() as u32,
                )
            || Some(self.week)
                != 2u32.checked_add(self.receipts.values().filter(|r| r.progression).count() as u32)
        {
            return Err("inconsistent campaign return/death ledger".into());
        }
        Ok(())
    }
    pub fn commit_return(
        &mut self,
        receipt: &str,
        kind: ReturnKind,
        new_dead: &BTreeSet<u64>,
        progression: bool,
    ) -> Result<()> {
        if receipt.is_empty() || receipt.len() > 512 || self.receipts.contains_key(receipt) {
            return Err("invalid/duplicate campaign return".into());
        }
        let mut n = self.clone();
        n.returns = n.returns.checked_add(1).ok_or("campaign return overflow")?;
        if progression {
            n.week = n.week.checked_add(1).ok_or("campaign week overflow")?;
        }
        if kind == ReturnKind::Complete {
            n.completions = n
                .completions
                .checked_add(1)
                .ok_or("campaign completion overflow")?;
        }
        n.dead.extend(new_dead);
        n.receipts.insert(
            receipt.into(),
            ReturnReceipt {
                kind,
                dead: new_dead.clone(),
                progression,
            },
        );
        n.validate()?;
        *self = n;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Failure {
    WeekLimit,
    DeathLimit,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub week: u32,
    pub deaths: u32,
}
impl Limits {
    pub fn reached(&self, week: u32, deaths: usize, story_completed: bool) -> Option<Failure> {
        if story_completed {
            None
        } else if deaths >= self.deaths as usize {
            Some(Failure::DeathLimit)
        } else if week >= self.week {
            Some(Failure::WeekLimit)
        } else {
            None
        }
    }
}
