//! Host adapter for the independent025 public pipeline. Old runners are unchanged.
use super::*;
use crate::strategy025::{self as strategy, Context, Memory, Sources};
use mechanisms023::Runtime023;
use std::sync::Arc;
#[derive(Clone)]
pub struct Runtime025 {
    pub inner: Runtime023,
    pub profile: Arc<strategy::Profile>,
    pub enabled: BTreeSet<String>,
    pub ordered: bool,
    pub sources: Sources,
    curio_source: Arc<dd_data::curios013::Curios>,
    pub memory: Memory,
    pub zero_target_events: Vec<preparation024::LocatedFallback>,
    pub mechanisms: crate::execution025::Options,
    pub mechanism_events: Vec<Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save025 {
    profile: String,
    hash: String,
    enabled: BTreeSet<String>,
    ordered: bool,
    inner: String,
    memory: Memory,
    zero_target_events: Vec<preparation024::LocatedFallback>,
    #[serde(default)]
    mechanisms: crate::execution025::Options,
    #[serde(default)]
    mechanism_events: Vec<Value>,
}
impl Runtime025 {
    pub fn new(config: Config, groups: &str) -> Result<Self> {
        let profile = Arc::new(strategy::Profile::documented()?);
        let (enabled, ordered) = profile.enabled(groups)?;
        let inner = Runtime023::new(
            config,
            arena021::Strategy::Rules021,
            dd_data::mechanisms023::Candidates::documented(),
            true,
        )?;
        Self::from_inner(inner, profile, enabled, ordered)
    }
    fn from_inner(
        inner: Runtime023,
        profile: Arc<strategy::Profile>,
        enabled: BTreeSet<String>,
        ordered: bool,
    ) -> Result<Self> {
        if enabled
            .iter()
            .any(|g| !profile.groups.iter().any(|x| &x.id == g))
        {
            return Err("unknown025 enabled group".into());
        }
        let curio_source = Arc::new(dd_data::curios013::Curios::load_campaign(
            &inner.replay.inner.inner.inner.definition.town.game.overlay,
        )?);
        let sources = Sources::project(&inner.replay.inner.inner.inner.definition.town.game)?;
        let mechanisms = crate::execution025::Options::candidate(!enabled.is_empty());
        Ok(Self {
            inner,
            profile,
            enabled,
            ordered,
            sources,
            curio_source,
            memory: Default::default(),
            zero_target_events: vec![],
            mechanisms,
            mechanism_events: vec![],
        })
    }
    pub fn checkpoint(&self) -> Result<String> {
        crate::execution025::with_options(self.mechanisms, || self.checkpoint_scoped())
    }
    fn checkpoint_scoped(&self) -> Result<String> {
        serde_json::to_string(&Save025 {
            profile: strategy::PROFILE.into(),
            hash: self.profile.binding(&self.enabled, self.ordered)?,
            enabled: self.enabled.clone(),
            ordered: self.ordered,
            inner: self.inner.checkpoint()?,
            memory: self.memory.clone(),
            zero_target_events: self.zero_target_events.clone(),
            mechanisms: self.mechanisms,
            mechanism_events: self.mechanism_events.clone(),
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str) -> Result<Self> {
        if text.len() > 220 * 1024 * 1024 {
            return Err("025 snapshot exceeds bound".into());
        }
        Self::restore_profile026(text, Arc::new(strategy::Profile::documented()?))
    }
    pub(crate) fn restore_profile026(text: &str, profile: Arc<strategy::Profile>) -> Result<Self> {
        let saved: Save025 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if saved.profile != strategy::PROFILE
            || saved.hash != profile.binding(&saved.enabled, saved.ordered)?
        {
            return Err("025 profile/data binding mismatch".into());
        }
        crate::execution025::with_options(saved.mechanisms, || {
            let mut r = Self::from_inner(
                Runtime023::restore(&saved.inner)?,
                profile,
                saved.enabled,
                saved.ordered,
            )?;
            r.memory = saved.memory;
            r.zero_target_events = saved.zero_target_events;
            r.mechanisms = saved.mechanisms;
            r.mechanism_events = saved.mechanism_events;
            Ok(r)
        })
    }
    pub(crate) fn sources_visible026(&self, view: &View) -> Result<Sources> {
        let mut sources = self.sources.clone();
        if let View::Campaign(o) = view {
            sources.refresh_visible(
                &self.inner.replay.inner.inner.inner.definition.town.game,
                o,
                &self.profile,
                &self.curio_source,
            )?;
        }
        Ok(sources)
    }
    pub fn observation(&mut self) -> Result<View> {
        crate::execution025::with_options(self.mechanisms, || {
            self.inner.replay.inner.inner.inner.observe()
        })
    }
    pub fn visible(&mut self) -> Result<Value> {
        crate::execution025::with_options(self.mechanisms, || self.inner.replay.inner.refresh())
    }
    pub fn frame(&mut self) -> Result<Value> {
        crate::execution025::with_options(self.mechanisms, || self.inner.replay.frame())
    }
    pub fn step(&mut self, recording: bool) -> Result<Option<Value>> {
        crate::execution025::with_options(self.mechanisms, || self.step_scoped(recording, None))
    }
    pub(crate) fn step_selected026(
        &mut self,
        recording: bool,
        proposal: Option<strategy::Proposal>,
    ) -> Result<Option<Value>> {
        crate::execution025::with_options(self.mechanisms, || self.step_scoped(recording, proposal))
    }
    fn step_scoped(
        &mut self,
        recording: bool,
        selected026: Option<strategy::Proposal>,
    ) -> Result<Option<Value>> {
        let mut next = self.clone();
        next.inner.replay.recording024c = recording;
        let before_view = next.inner.replay.inner.inner.inner.observe()?;
        let enabled = !next.enabled.is_empty();
        if enabled && let View::Campaign(o) = &before_view {
            if (next.enabled.contains("income") || next.enabled.contains("growth"))
                && o.expedition.is_none()
                && o.provisions.is_none()
            {
                next.sources.training_heirlooms = next.memory.ledger.heirlooms.clone();
                let key = serde_json::to_vec(&(
                    &o.town.heroes,
                    &o.town.purchased,
                    &o.town.upgrade_quotes,
                    &o.town.buildings,
                    &o.town
                        .actions
                        .iter()
                        .filter(|c| {
                            matches!(
                                c.action,
                                crate::town016::Action::Queue {
                                    treatment: crate::town016::Treatment::Stress,
                                    ..
                                }
                            )
                        })
                        .collect::<Vec<_>>(),
                    &o.current_event,
                    o.week,
                ))
                .map_err(|e| e.to_string())?;
                if key != next.sources.training_key {
                    let Actor::Campaign(host) = &next.inner.replay.inner.inner.inner.actor else {
                        return Err("025 training quote requires campaign".into());
                    };
                    if next.enabled.contains("income") {
                        next.sources.training = Arc::new(host.training_quotes025()?);
                    }
                    if next.enabled.contains("growth") {
                        next.sources.recovery_ranges = host.recovery_ranges025(&o.town)?;
                    }
                    next.sources.training_key = key;
                }
            }
            next.sources.current_boss = if next.memory.returns == Some(o.returns)
                && next.memory.legacy.target_party.iter().all(|id| {
                    o.town
                        .heroes
                        .iter()
                        .any(|h| h.guid == *id && h.alive && !h.busy && !h.missing)
                }) {
                next.memory
                    .state
                    .get("boss_plan")
                    .and_then(Value::as_str)
                    .map(|id| (id.to_string(), next.memory.legacy.target_party.clone()))
            } else {
                None
            };
            next.sources.refresh_visible(
                &next.inner.replay.inner.inner.inner.definition.town.game,
                o,
                &next.profile,
                &next.curio_source,
            )?;
        }
        let mut chosen = None;
        let ((log, events), candidate_events) = crate::execution025::run(next.mechanisms, || {
            crate::execution024b::run(enabled, || {
                if !enabled && selected026.is_none() {
                    return next.inner.step();
                }
                let profile = next.profile.clone();
                let groups = &next.enabled;
                let ordered = next.ordered;
                let sources = &next.sources;
                let memory = &mut next.memory;
                next.inner.step_using025(|r| {
                    let view = r.inner.inner.observe()?;
                    let proposal = if selected026.is_some() {
                        selected026
                    } else if let View::Campaign(o) = &view {
                        if memory.metrics.needs_observe(o, None) {
                            Arc::make_mut(&mut memory.metrics).observe(o, None, &memory.ledger);
                        }
                        memory
                            .legacy
                            .blocked
                            .extend(r.inner.inner.memory.blocked.iter().cloned());
                        let quote = if o.provisions.is_none() && !o.town.party.is_empty() {
                            memory
                                .legacy
                                .quest
                                .as_ref()
                                .and_then(|id| {
                                    o.board
                                        .as_ref()
                                        .and_then(|b| b.quests.iter().find(|q| &q.id == id))
                                })
                                .map(|q| {
                                    let Actor::Campaign(host) = &r.inner.inner.actor else {
                                        return Err("025 quote needs campaign".into());
                                    };
                                    host.preparation_quote024b(&q.id).map(|p| {
                                        crate::preparation024::economy::Quote::from_visible(&p)
                                    })
                                })
                                .transpose()?
                        } else {
                            None
                        };
                        memory.ledger.refresh_planned(
                            o,
                            &profile,
                            quote.as_ref(),
                            groups.contains("exploration") && !sources.mode.ends_with(":dark"),
                        );
                        strategy::policy::choose(
                            o,
                            memory,
                            &Context {
                                profile: &profile,
                                enabled: groups,
                                ordered,
                                sources,
                            },
                        )?
                    } else {
                        None
                    };
                    if let Some(p) = proposal {
                        let selected = memory.legacy.quest.is_some()
                            && match &view {
                                View::Campaign(o) => o.town.party == memory.legacy.target_party,
                                _ => false,
                            };
                        if !memory.ledger.permits(p.purpose, p.cost, selected) {
                            return Err("025 pipeline proposed spending outside its ledger".into());
                        }
                        let result =
                            execute(r, p.action.clone(), format!("025:{}; {}", p.rule, p.stage))?;
                        chosen = Some(p);
                        Ok(result)
                    } else {
                        r.step()
                    }
                })
            })
        })?;
        let step = next.inner.replay.inner.inner.inner.decisions;
        let week = log
            .as_ref()
            .and_then(|v| v["before"]["week"].as_u64())
            .unwrap_or(0) as u32;
        next.mechanism_events.extend(
            candidate_events
                .iter()
                .map(|v| json!({"step":step,"week":week,"event":v})),
        );
        next.zero_target_events.extend(
            events
                .into_iter()
                .map(|event| preparation024::LocatedFallback { step, week, event }),
        );
        if let Some(log) = &log {
            next.memory.usage.record(&before_view, log);
        }
        if let (Some(log), View::Campaign(o)) = (&log, &before_view) {
            if next.enabled.contains("stress") {
                let b = o
                    .expedition
                    .as_ref()
                    .and_then(|r| r.battle.as_ref())
                    .or_else(|| {
                        o.content019
                            .as_ref()
                            .and_then(|c| c.farm.as_ref())
                            .map(|f| &f.battle)
                    });
                if let Some(b) = b {
                    let a = log["action"]["Raid"]
                        .get("Fight")
                        .or_else(|| log["action"]["Farm"].get("Fight"));
                    if let Some(a) = a {
                        strategy::combat::finish(
                            b,
                            &serde_json::from_value(a.clone()).map_err(|e| e.to_string())?,
                            &mut next.memory,
                            &next.sources,
                        );
                    }
                }
            }

            let mut quoted = 0;
            if let Some(p) = &chosen {
                quoted = p.cost;
            } else if let Ok(action) = serde_json::from_value::<Action>(log["action"].clone()) {
                quoted = strategy::price(o, &action)?;
                next.memory.ledger.record(
                    &strategy::Proposal {
                        purpose: strategy::purpose(&action),
                        action,
                        rule: "S025-00".into(),
                        stage: "legacy_audit".into(),
                        cost: quoted,
                    },
                    week,
                );
            }
            let before = log["before"]["gold"].as_u64().unwrap_or(0);
            let after = log["after"]["gold"].as_u64().unwrap_or(before);
            let automatic = before
                .saturating_sub(after)
                .saturating_sub(u64::from(quoted));
            if automatic > 0 {
                *next
                    .memory
                    .ledger
                    .spent
                    .entry("automatic_net_settlement".into())
                    .or_default() += automatic;
                next.memory.ledger.receipts.push(json!({"week":week,"purpose":"automatic_net_settlement","gold":automatic,"rule":"native settlement; observed net only"}));
            }
            if log["action"]["Town"].get("Sell").is_some() {
                let mut sale = next
                    .memory
                    .state
                    .remove("sale")
                    .unwrap_or_else(|| json!({"score":null}));
                sale["week"] = json!(week);
                sale["gold"] = json!(after.saturating_sub(before));
                sale["action"] = log["action"].clone();
                next.memory.ledger.sold.push(sale);
            }
        }
        if let Some(log) = &log {
            strategy::recovery::committed(&mut next.memory, log);
        }
        let log=log.map(|mut log| {
   if enabled {
    let rule=chosen.as_ref().map_or("S025-00",|p|p.rule.as_str());*next.memory.hits.entry(rule.into()).or_default()+=1;
    if let Some(p)=&chosen {next.memory.ledger.record(p,week);}next.memory.stage=chosen.as_ref().map_or("legacy_fallback",|p|p.stage.as_str()).into();
    log["strategy025"]=json!({"profile":strategy::PROFILE,"rule_ids":[rule],"stage":next.memory.stage,"groups":next.enabled,"ordered":next.ordered,"boss_plan":next.memory.state.get("boss_plan"),"boss_fallback":next.memory.state.get("boss_fallback"),"veteran_selection":next.memory.state.get("veteran_last_selection"),"recovery_intent":next.memory.state.get("next_trip_intent"),"mechanism_flags":next.mechanisms,"mechanism_events":candidate_events});
   }
   if next.profile.parameters["coherence029"]==true {log["strategy025"]["training_budget029"]=json!(next.memory.state.get("training_budget029"));}
   log
  });
        let after_view = next.inner.replay.inner.inner.inner.observe()?;
        if let Some(log) = &log {
            next.memory
                .ledger
                .heirlooms
                .record(&before_view, &after_view, log)?;
        }
        if let View::Campaign(o) = after_view
            && next.memory.metrics.needs_observe(&o, log.as_ref())
        {
            Arc::make_mut(&mut next.memory.metrics).observe(&o, log.as_ref(), &next.memory.ledger);
            if next.profile.parameters["coherence029"] == true
                && log
                    .as_ref()
                    .is_some_and(|l| l["action"]["Provision"] == "Embark")
            {
                let cursor = next
                    .memory
                    .state
                    .get("supply_receipt_cursor029")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                let supply = strategy::metrics::trip_supply029(&next.memory.ledger, cursor);
                if let Some(trip) = &mut Arc::make_mut(&mut next.memory.metrics).active {
                    trip["supply_gold"] = json!(supply);
                }
                next.memory.state.insert(
                    "supply_receipt_cursor029".into(),
                    json!(next.memory.ledger.receipts.len()),
                );
            }
        }
        next.inner.replay.recording024c = true;
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
        return Err("025 override requires campaign boundary".into());
    };
    match action.clone() {
        Action::Town(a) => s.town_action(a)?,
        Action::Select(id) => s.select_quest(Some(id))?,
        Action::Prepare => s.prepare_quest()?,
        Action::Provision(a) => s.provision_action(a)?,
        Action::Raid(a) => s.expedition_action(a)?,
        Action::Farm(a) => s.farm019_action(a)?,
        Action::Event(a) => s.event_action(a)?,
    }
    runtime.decisions = runtime
        .decisions
        .checked_add(1)
        .ok_or("025 decision overflow")?;
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
