//! Isolated preparation wrapper. The 023 runner still owns every combat decision.
use super::*;
use crate::preparation024::{Catalog, Context, GROUPS, Profile};
use mechanisms023::Runtime023;
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "candidate-auto-024-v1";
#[derive(Clone, Serialize, Deserialize)]
pub struct LocatedFallback {
    pub step: u64,
    pub week: u32,
    #[serde(flatten)]
    pub event: crate::execution024b::Fallback,
}
#[derive(Clone)]
pub struct Runtime024 {
    pub inner: Runtime023,
    pub context: Context,
    pub hits: BTreeMap<String, u64>,
    pub preparation_memory: crate::preparation024::economy::Memory,
    pub zero_target_fallback: bool,
    pub zero_target_events: Vec<LocatedFallback>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save024 {
    profile: String,
    hash: String,
    enabled: BTreeSet<String>,
    hits: BTreeMap<String, u64>,
    inner: String,
    preparation_memory: crate::preparation024::economy::Memory,
    #[serde(default)]
    zero_target_fallback: bool,
    #[serde(default)]
    zero_target_events: Vec<LocatedFallback>,
}
impl Runtime024 {
    /// Same decisions and transactions, without delta frames or encounter annotations.
    /// The CLI binds checkpoints to their log mode; this flag is not simulator state.
    pub fn step_without_log(&mut self) -> Result<Option<Value>> {
        let previous = self.inner.replay.recording024c;
        self.inner.replay.recording024c = false;
        let result = self.step();
        self.inner.replay.recording024c = previous;
        result
    }
    /// Deterministic, publicly inspectable task shop; never prepares an encounter.
    pub fn public_cart_quote024b(&self, id: &str) -> Result<crate::preparation024::economy::Quote> {
        let Actor::Campaign(s) = &self.inner.replay.inner.inner.inner.actor else {
            return Err("cart quotes require a town".into());
        };
        Ok(crate::preparation024::economy::Quote::from_visible(
            &s.preparation_quote024b(id)?,
        ))
    }
    pub fn new(config: Config, enabled: BTreeSet<String>) -> Result<Self> {
        let inner = Runtime023::new(
            config,
            arena021::Strategy::Rules021,
            dd_data::mechanisms023::Candidates::documented(),
            true,
        )?;
        Self::from_inner(inner, enabled)
    }
    pub fn from_inner(inner: Runtime023, enabled: BTreeSet<String>) -> Result<Self> {
        if enabled.iter().any(|s| !GROUPS.contains(&s.as_str())) {
            return Err("unknown 024 group".into());
        }
        let catalog = Catalog::project(&inner.replay.inner.inner.inner.definition.town.game)?;
        Ok(Self {
            inner,
            context: Context {
                profile: Profile::documented()?,
                enabled,
                catalog,
                role_gaps: BTreeSet::new(),
            },
            hits: BTreeMap::new(),
            preparation_memory: Default::default(),
            zero_target_fallback: true,
            zero_target_events: vec![],
        })
    }
    pub fn checkpoint(&self) -> Result<String> {
        serde_json::to_string(&Save024 {
            profile: PROFILE.into(),
            hash: self.context.profile.binding(&self.context.enabled)?,
            enabled: self.context.enabled.clone(),
            hits: self.hits.clone(),
            inner: self.inner.checkpoint()?,
            preparation_memory: self.preparation_memory.clone(),
            zero_target_fallback: self.zero_target_fallback,
            zero_target_events: self.zero_target_events.clone(),
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str) -> Result<Self> {
        if text.len() > 220 * 1024 * 1024 {
            return Err("024 snapshot exceeds bound".into());
        }
        let s: Save024 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.profile != PROFILE || s.hash != Profile::documented()?.binding(&s.enabled)? {
            return Err("024 profile/data hash mismatch".into());
        }
        let mut r = Self::from_inner(Runtime023::restore(&s.inner)?, s.enabled)?;
        r.hits = s.hits;
        r.preparation_memory = s.preparation_memory;
        r.zero_target_fallback = s.zero_target_fallback;
        r.zero_target_events = s.zero_target_events;
        Ok(r)
    }
    pub fn step(&mut self) -> Result<Option<Value>> {
        let (log, events) =
            crate::execution024b::run(self.zero_target_fallback, || self.step_preparation())?;
        let step = self.inner.replay.inner.inner.inner.decisions;
        let week = log
            .as_ref()
            .and_then(|v| v["before"]["week"].as_u64())
            .unwrap_or(0) as u32;
        self.zero_target_events
            .extend(
                events
                    .into_iter()
                    .map(|event| LocatedFallback { step, week, event }),
            );
        if self.context.on("H")
            && let Some(v) = &log
            && v["action"]["Town"].get("Sell").is_some()
        {
            self.preparation_memory.sales += 1;
            self.preparation_memory.sale_gold += v["after"]["gold"]
                .as_u64()
                .unwrap_or(0)
                .saturating_sub(v["before"]["gold"].as_u64().unwrap_or(0));
        }
        Ok(log)
    }
    fn step_preparation(&mut self) -> Result<Option<Value>> {
        let _p = dd_data::telemetry024c::scope("policy_and_rules");
        if self.inner.replay.inner.inner.inner.status != Status::Running {
            return Ok(None);
        }
        let mut next = {
            let _p = dd_data::telemetry024c::scope("clone_runner024");
            self.clone()
        };
        let mut rules = vec![];
        let view = next.inner.replay.inner.inner.inner.observe()?;
        let pending_guard = serde_json::to_value(&next.inner.guard).map_err(|e| e.to_string())?["abandon_after_retreat"]
            == true;
        let chosen = if next.context.enabled.is_empty() || pending_guard {
            None
        } else if let View::Campaign(o) = &view {
            let in_farm = o.content019.as_ref().is_some_and(|f| f.farm.is_some());
            if o.event_offer.is_some() || in_farm {
                None
            } else if let Some(raid) = &o.expedition {
                // Position maintenance belongs to A; all path and loot choices stay frozen.
                if (next.context.on("A") || next.context.on("B"))
                    && raid.battle.is_none()
                    && raid.camping.is_none()
                {
                    let runtime = &mut next.inner.replay.inner.inner.inner;
                    let mut visible = o.as_ref().clone();
                    visible.expedition.as_mut().unwrap().actions.retain(|a| {
                        !matches!(
                            a,
                            crate::opening015::Action::Reorder(_)
                                | crate::opening015::Action::EquipTrinket { .. }
                                | crate::opening015::Action::UnequipTrinket { .. }
                        )
                    });
                    let d = if let Some(action) =
                        next.context.reorder(&o.town, &raid.party, &raid.actions)
                    {
                        rules.push("P024-14".into());
                        Decision{action:Action::Raid(action),reason:"024:P024-14; restore the prepared skill-based formation using a legal Reorder".into()}
                    } else {
                        crate::navigation019::choose(
                            &visible,
                            &mut runtime.memory,
                            &mut next.inner.replay.inner.inner.navigation,
                        )?
                    };
                    Some(d)
                } else {
                    None
                }
            } else {
                if next.context.on("H") || next.context.on("E") {
                    crate::preparation024::assets::refresh(
                        &next.inner.replay.inner.inner.inner.definition.town.game,
                        o,
                        &mut next.context.catalog.assets,
                    )?;
                }
                if next.context.on("F") {
                    crate::preparation024::estate::refresh(
                        &next.inner.replay.inner.inner.inner.definition.town.game,
                        &o.town,
                        &next.context.profile.estate,
                        &mut next.context.catalog.quirks,
                    )?;
                }
                let runtime = &mut next.inner.replay.inner.inner.inner;
                next.context.catalog.cart_quote = None;
                if next.context.on("H")
                    && o.provisions.is_none()
                    && !o.town.party.is_empty()
                    && let Some(id) = runtime
                        .memory
                        .quest
                        .as_ref()
                        .or_else(|| o.board.as_ref().and_then(|b| b.selected.as_ref()))
                    && o.board
                        .as_ref()
                        .is_some_and(|b| b.quests.iter().any(|q| &q.id == id))
                    && let Actor::Campaign(s) = &runtime.actor
                {
                    next.context.catalog.cart_quote =
                        Some(crate::preparation024::economy::Quote::from_visible(
                            &s.preparation_quote024b(id)?,
                        ));
                }
                let context = next.context.considering(&o.town);
                if context.on("G") && o.provisions.is_none() {
                    crate::preparation024::funnel::observe(
                        &context,
                        o,
                        &runtime.memory.blocked,
                        &mut next.preparation_memory.merchant_funnel,
                    );
                }
                let d = crate::preparation024::policy::choose(
                    o,
                    &mut runtime.memory,
                    &context,
                    &mut next.preparation_memory,
                )?;
                if next.context.on("A") {
                    match &d.action {
                        Action::Town(crate::town016::Action::Party(_)) => {
                            rules.push("P024-14".to_string())
                        }
                        Action::Town(crate::town016::Action::SelectSkills { hero, .. }) => {
                            if let Some(h) = o.town.heroes.iter().find(|h| h.guid == *hero)
                                && let Some(l) = next
                                    .context
                                    .profile
                                    .loadouts
                                    .iter()
                                    .find(|l| l.class_id == h.class)
                            {
                                rules.push(l.id.clone())
                            }
                        }
                        _ => (),
                    }
                }
                if context.on("A2")
                    && let Action::Town(crate::town016::Action::Upgrade {
                        hero: Some(id),
                        tree,
                        ..
                    }) = &d.action
                    && let Some(h) = o.town.heroes.iter().find(|h| h.guid == *id)
                    && context.desired_skills(&h.class).iter().any(|skill| {
                        !h.skills.contains_key(skill) && *tree == format!("{}.{}", h.class, skill)
                    })
                {
                    rules.push(context.profile.learning.id.clone())
                }
                if context.on("B")
                    && matches!(
                        d.action,
                        Action::Town(crate::town016::Action::Party(_))
                            | Action::Select(_)
                            | Action::Prepare
                    )
                {
                    rules.push(context.profile.composition.id.clone())
                }
                if context.on("C")
                    && matches!(
                        d.action,
                        Action::Town(crate::town016::Action::Party(_))
                            | Action::Select(_)
                            | Action::Prepare
                    )
                {
                    rules.push(context.profile.gates.id.clone())
                }
                if context.on("F") && d.reason.starts_with("024:P024-22") {
                    rules.push(context.profile.estate.id.clone());
                }
                if d.reason.starts_with("024:P024-23") {
                    rules.push(context.profile.recovery.id.clone());
                }
                if context.on("G") && d.reason.starts_with("024:P024-21") {
                    rules.push(context.profile.merchant.id.clone());
                }
                if context.on("E")
                    && (d.reason.starts_with("024:P024-20")
                        || matches!(&d.action,Action::Select(id) if o.board.as_ref().is_some_and(|b|b.quests.iter().any(|q|&q.id==id && crate::preparation024::assets::focus_reward(&context,&o.town,q,o.week,&o.town.party)))))
                {
                    rules.push(context.profile.assets.id.clone());
                }
                if context.on("H") && d.reason.starts_with("024:P024-19") {
                    rules.push(context.profile.economy.id.clone());
                }
                if context.on("D") && d.reason.starts_with("024:P024-18") {
                    rules.push(context.profile.provisions.id.clone())
                }
                Some(d)
            }
        } else {
            None
        };
        let log = if let Some(d) = chosen {
            let reason = d.reason;
            let action = d.action;
            let guard = next.inner.guard.clone();
            let candidates = next.inner.candidates;
            next.inner.replay.step_using023(|r|execute(r,action,reason))?.map(|mut v|{v["mechanism023"]=json!({"profile":mechanisms023::PROFILE,"candidates":candidates,"guard_enabled":true,"guard_action":false,"guard":guard});v})
        } else {
            next.inner.step()?
        };
        for id in &rules {
            *next.hits.entry(id.clone()).or_default() += 1
        }
        let log=log.map(|mut v|{v["preparation024"]=json!({"profile":PROFILE,"definition_hash":next.context.profile.binding(&next.context.enabled).unwrap(),"groups":GROUPS.iter().map(|g|(*g,next.context.on(g))).collect::<BTreeMap<_,_>>(),"rule_ids":rules,"hits":next.hits});v});
        *self = next;
        Ok(log)
    }
}
fn execute(r: &mut arena021::Runtime021, action: Action, reason: String) -> Result<Option<Record>> {
    let mut next = r.clone();
    let runtime = &mut next.inner.inner;
    let before = runtime.observe()?;
    runtime.terminal(&before)?;
    if runtime.status != Status::Running {
        return Ok(None);
    }
    let before_summary = summary(&before)?;
    let Actor::Campaign(s) = &mut runtime.actor else {
        return Err("024 preparation requires campaign boundary".into());
    };
    match action.clone() {
        Action::Town(a) => s.town_action(a)?,
        Action::Select(id) => s.select_quest(Some(id))?,
        Action::Prepare => s.prepare_quest()?,
        Action::Provision(a) => s.provision_action(a)?,
        Action::Raid(a) if !matches!(a, crate::opening015::Action::Fight(_)) => {
            s.expedition_action(a)?
        }
        _ => return Err("024 preparation cannot override combat or farm actions".into()),
    }
    runtime.decisions = runtime
        .decisions
        .checked_add(1)
        .ok_or("024 decision overflow")?;
    runtime.cached = None;
    let after = runtime.observe()?;
    runtime.terminal(&after)?;
    let mut record = Record {
        step: runtime.decisions,
        action: serde_json::to_value(action).map_err(|e| e.to_string())?,
        reason,
        before: before_summary,
        after: summary(&after)?,
        status: runtime.status.clone(),
    };
    next.inner.enrich(&mut record.before)?;
    next.inner.enrich(&mut record.after)?;
    next.memory = Default::default();
    *r = next;
    Ok(Some(record))
}
