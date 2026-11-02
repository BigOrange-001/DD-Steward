//! Strategy wrapper; the baseline delegates verbatim to the frozen 020 runner.
use super::*;
use crate::tactics021::{self, Coverage, Rules};
use std::sync::Arc;
use steward020::Runtime020;
pub const PROFILE: &str = "candidate-auto-021-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Strategy {
    Baseline,
    Rules021,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save021 {
    profile: String,
    strategy: Strategy,
    rules_hash: Option<String>,
    memory: tactics021::Memory,
    coverage: Coverage,
    inner: String,
}

#[derive(Clone)]
pub struct Runtime021 {
    pub inner: Runtime020,
    pub strategy: Strategy,
    rules: Option<Arc<Rules>>,
    pub memory: tactics021::Memory,
    pub coverage: Coverage,
}
impl Runtime021 {
    pub fn with_definition(
        config: Config,
        definition: Definition,
        strategy: Strategy,
        rules: Option<Rules>,
    ) -> Result<Self> {
        if (strategy == Strategy::Rules021) != rules.is_some() {
            return Err("021 strategy/table mismatch".into());
        }
        Ok(Self {
            inner: Runtime020::with_definition(config, definition)?,
            strategy,
            rules: rules.map(Arc::new),
            memory: Default::default(),
            coverage: Default::default(),
        })
    }
    pub fn rules_for(definition: &Definition) -> Result<Rules> {
        Rules::documented(&definition.town.game)
    }
    /// Read-only access to the frozen fallback, for the candidate shared retreat gate.
    pub(crate) fn visible_choice029(
        &self,
        o: &crate::Observation,
    ) -> Result<Option<(crate::Action, String)>> {
        self.rules
            .as_ref()
            .ok_or("021 rule table absent")?
            .evaluate(o, &self.memory)
            .map(|e| e.choice.map(|c| (c.action, c.rule_id)))
    }
    pub fn step(&mut self) -> Result<Option<Record>> {
        if self.strategy == Strategy::Baseline {
            return self.inner.step();
        }
        let mut next = {
            let _p = dd_data::telemetry024c::scope("clone_runner021");
            self.clone()
        };
        let ongoing = match next.inner.inner.observe()? {
            View::Opening(o) => o.battle.is_some_and(|b| b.outcome.is_none()),
            View::Campaign(o) => {
                o.expedition
                    .as_ref()
                    .and_then(|e| e.battle.as_ref())
                    .is_some_and(|b| b.outcome.is_none())
                    || o.content019
                        .as_ref()
                        .and_then(|c| c.farm.as_ref())
                        .is_some_and(|f| f.battle.outcome.is_none())
            }
        };
        let rules = next.rules.as_ref().ok_or("021 rule table absent")?.clone();
        let mut memory = next.memory.clone();
        let mut evaluation = None;
        let mut policy = |o: &crate::Observation| {
            let result = rules.evaluate(o, &memory)?;
            memory = result.memory;
            let id = result.choice.as_ref().map(|c| c.rule_id.clone());
            evaluation = Some((o.clone(), id));
            Ok(result
                .choice
                .map(|c| (c.action, format!("021:{}", c.rule_id))))
        };
        let mut record = next
            .inner
            .inner
            .step_inner_with_combat(Some(&mut next.inner.navigation), Some(&mut policy))?;
        if let Some(r) = &mut record {
            next.inner.enrich(&mut r.before)?;
            next.inner.enrich(&mut r.after)?;
            if let Some((o, id)) = evaluation {
                let value = r
                    .action
                    .get("Opening")
                    .or_else(|| r.action.get("Raid"))
                    .or_else(|| r.action.get("Farm"))
                    .and_then(|v| v.get("Fight"));
                let action = if r.action["Farm"] == json!("Exit") {
                    crate::Action::Retreat
                } else {
                    serde_json::from_value(
                        value
                            .ok_or("021 combat action missing from record")?
                            .clone(),
                    )
                    .map_err(|e| e.to_string())?
                };
                next.coverage.record(&o, &action, id.as_deref());
                if id.is_none() {
                    r.reason = format!("021:fallback; {}", r.reason);
                }
                next.memory = memory;
            } else if !ongoing {
                next.memory = Default::default();
            }
        }
        *self = next;
        Ok(record)
    }
    pub fn refresh(&mut self) -> Result<Value> {
        self.inner.refresh()
    }
    pub fn rules_hash(&self) -> Option<&str> {
        self.rules.as_ref().map(|r| r.hash())
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.memory.validate()?;
        serde_json::to_string(&Save021 {
            profile: PROFILE.into(),
            strategy: self.strategy,
            rules_hash: self.rules_hash().map(str::to_owned),
            memory: self.memory.clone(),
            coverage: self.coverage.clone(),
            inner: self.inner.checkpoint()?,
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str, definition: Definition, rules: Option<Rules>) -> Result<Self> {
        if text.len() > 160 * 1024 * 1024 {
            return Err("021 snapshot exceeds bound".into());
        }
        let save: Save021 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if save.profile != PROFILE
            || save.rules_hash.as_deref() != rules.as_ref().map(Rules::hash)
            || (save.strategy == Strategy::Rules021) != rules.is_some()
        {
            return Err("021 profile/strategy/rule table hash mismatch".into());
        }
        save.memory.validate()?;
        let inner = Runtime020::restore(&save.inner, definition)?;
        if save.coverage.hits + save.coverage.fallback != save.coverage.decisions
            || save.coverage.decisions > inner.inner.decisions
        {
            return Err("invalid 021 coverage counters".into());
        }
        Ok(Self {
            inner,
            strategy: save.strategy,
            rules: rules.map(Arc::new),
            memory: save.memory,
            coverage: save.coverage,
        })
    }
    pub fn restore_config(text: &str) -> Result<Self> {
        if text.len() > 160 * 1024 * 1024 {
            return Err("021 snapshot exceeds bound".into());
        }
        let save: Save021 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if save.profile != PROFILE {
            return Err("021 profile mismatch".into());
        }
        let inner = Runtime020::restore_config(&save.inner)?;
        let definition = inner.inner.definition.clone();
        let rules = if save.strategy == Strategy::Rules021 {
            Some(Self::rules_for(&definition)?)
        } else {
            None
        };
        Self::restore(text, definition, rules)
    }
    /// Adds public combat context to new arena logs only; legacy records stay byte-identical.
    pub fn diagnostic(&mut self) -> Result<Value> {
        let view = self.inner.inner.observe()?;
        let mut summary = summary(&view)?;
        let battle = match &view {
            View::Opening(o) => o.battle.as_ref(),
            View::Campaign(o) => o
                .expedition
                .as_ref()
                .and_then(|e| e.battle.as_ref())
                .or_else(|| {
                    o.content019
                        .as_ref()
                        .and_then(|c| c.farm.as_ref())
                        .map(|f| &f.battle)
                }),
        };
        if let Some(b) = battle {
            let retreat = crate::opening015::Action::Fight(crate::Action::Retreat);
            let can_retreat = match &view {
                View::Opening(o) => o.actions.contains(&retreat),
                View::Campaign(o) => {
                    o.expedition
                        .as_ref()
                        .is_some_and(|e| e.actions.contains(&retreat))
                        || o.content019
                            .as_ref()
                            .and_then(|c| c.farm.as_ref())
                            .is_some_and(|f| {
                                f.actions
                                    .contains(&crate::campaign017::content019::farm::Action::Exit)
                                    || f.actions.contains(
                                        &crate::campaign017::content019::farm::Action::Fight(
                                            crate::Action::Retreat,
                                        ),
                                    )
                            })
                }
            };
            let combat = json!({"round":b.round,"actor":b.actor,"enemies":b.units.iter()
                .filter(|u|!u.hero&&!u.corpse).collect::<Vec<_>>(),"units":b.units});
            if summary["expedition"].is_null() {
                if let View::Campaign(o) = &view {
                    let heroes = b
                        .units
                        .iter()
                        .filter(|u| u.hero && !u.corpse)
                        .filter_map(|u| {
                            let guid = o.town.party.get(u.id)?;
                            let h = o.town.heroes.iter().find(|h| h.guid == *guid)?;
                            Some(
                                json!({"guid":guid,"class":u.class,"level":h.level,"xp":h.xp,
                            "hp":u.hp,"max_hp":u.max_hp,"stress":u.stress,"rank":u.rank}),
                            )
                        })
                        .collect::<Vec<_>>();
                    summary["expedition"] =
                        json!({"party":o.town.party,"heroes":heroes,"battle":combat,"node":"farm"});
                }
            } else {
                summary["expedition"]["battle"] = combat;
            }
            summary["expedition"]["can_retreat"] = json!(can_retreat);
        }
        Ok(summary)
    }
}
