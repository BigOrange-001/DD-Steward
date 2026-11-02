//! Independent026 host. Only public views/frame and original definitions enter policy.
use super::*;
use crate::{rollout026 as mc, strategy025 as s25};
use std::sync::Arc;
use strategy025::Runtime025;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    #[serde(default, skip_serializing_if = "disabled")]
    pub zero_progress_guard: bool,
    pub rollout: bool,
    #[serde(default)]
    pub legacy_rooms: bool,
    pub samples: usize,
}
fn disabled(value: &bool) -> bool {
    !*value
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Audit {
    pub decisions: u64,
    pub simulations: u64,
    pub fallback_reasons: BTreeMap<String, u64>,
    pub captures: Arc<Vec<Value>>,
    #[serde(default)]
    pub retreats: Arc<Vec<Value>>,
    #[serde(default)]
    pub diagnostics: s25::sync026::diagnostics::Audit,
    seen: Arc<BTreeSet<String>>,
}
pub struct Runtime026 {
    pub inner: Runtime025,
    pub options: Options,
    pub audit: Audit,
    pub settings: mc::Settings,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    profile: String,
    options: Options,
    settings: mc::Settings,
    inner: String,
    audit: Audit,
}
pub fn profile(legacy_rooms: bool) -> Result<Arc<s25::Profile>> {
    let mut profile = s25::Profile::documented()?;
    s25::sync026::apply(&mut profile)?;
    if legacy_rooms {
        profile.parameters["sync026"]["field"]["minimum_visited_rooms"] = json!(4);
    }
    Ok(Arc::new(profile))
}
fn groups026(groups: &str) -> (String, bool) {
    let mut rollout = groups == "all" || groups.starts_with("all,");
    let mut old = vec![];
    for g in groups.split(',') {
        match g {
            "rollout" => rollout = true,
            "-rollout" => rollout = false,
            _ => old.push(g),
        }
    }
    if old.is_empty() {
        old.push("none");
    }
    (old.join(","), rollout)
}
impl Runtime026 {
    pub fn new(config: Config, groups: &str) -> Result<Self> {
        let (text, rollout) = groups026(groups);
        let mut inner = Runtime025::new(config, &text)?;
        inner.profile = profile(false)?;
        let guard: Value = serde_json::from_str(r#"{
  "version": "candidate-zero-progress-026-v1",
  "enabled": true,
  "fallback": "existing_legacy_visited_rooms_gate_for_this_sortie_only",
  "source": "Unattended section3 strategy-deadpoint recovery; identical visible field exit recurs on the next return without a week advancing. No engine week rule or normal retreat threshold changes."
}
"#)
            .map_err(|e| e.to_string())?;
        if guard["version"] != "candidate-zero-progress-026-v1" {
            return Err("026 unknown cycle guard version".into());
        }
        let zero_progress_guard = guard["enabled"]
            .as_bool()
            .ok_or("026 missing cycle guard setting")?;
        let mut facts = s25::sync026::field::Facts::project(
            &inner.inner.replay.inner.inner.inner.definition.town.game,
        )?;
        facts.zero_progress_guard = zero_progress_guard;
        inner.sources.field026 = Some(Arc::new(facts));
        let settings = mc::Settings::budgeted()?;
        Ok(Self {
            inner,
            options: Options {
                zero_progress_guard,
                rollout,
                legacy_rooms: false,
                samples: settings.samples,
            },
            settings,
            audit: Audit::default(),
        })
    }
    pub fn checkpoint(&self) -> Result<String> {
        serde_json::to_string(&Save {
            profile: "candidate-rollout-026-v1".into(),
            options: self.options.clone(),
            settings: self.settings.clone(),
            inner: self.inner.checkpoint()?,
            audit: self.audit.clone(),
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str) -> Result<Self> {
        let s: Save = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.profile != "candidate-rollout-026-v1"
            || serde_json::to_value(&s.settings).map_err(|e| e.to_string())?
                != serde_json::to_value(mc::Settings::budgeted()?).map_err(|e| e.to_string())?
            || s.options.samples != s.settings.samples
        {
            return Err("026 settings binding mismatch".into());
        }
        Ok(Self {
            inner: {
                let mut inner =
                    Runtime025::restore_profile026(&s.inner, profile(s.options.legacy_rooms)?)?;
                let mut facts = s25::sync026::field::Facts::project(
                    &inner.inner.replay.inner.inner.inner.definition.town.game,
                )?;
                facts.zero_progress_guard = s.options.zero_progress_guard;
                inner.sources.field026 = Some(Arc::new(facts));
                inner
            },
            options: s.options,
            settings: s.settings,
            audit: s.audit,
        })
    }
    pub fn zero_progress_guard(&mut self, enabled: bool) {
        self.options.zero_progress_guard = enabled;
        if let Some(f) = &mut self.inner.sources.field026 {
            Arc::make_mut(f).zero_progress_guard = enabled;
        }
    }
    pub fn legacy_rooms(&mut self) -> Result<()> {
        self.options.legacy_rooms = true;
        self.inner.profile = profile(true)?;
        Ok(())
    }
    pub fn observation(&mut self) -> Result<View> {
        self.inner.observation()
    }
    pub fn visible(&mut self) -> Result<Value> {
        self.inner.visible()
    }
    pub fn frame(&mut self) -> Result<Value> {
        self.inner.frame()
    }
    fn decide(
        &self,
        o: &crate::Observation,
        frame: &Value,
        view: &View,
    ) -> Result<(mc::PublicBattle, mc::Decision)> {
        let game = &self
            .inner
            .inner
            .replay
            .inner
            .inner
            .inner
            .definition
            .town
            .game;
        let mut frame = frame.clone();
        frame["corridor"] = json!(
            frame["map"]["nodes"]
                .as_array()
                .and_then(|nodes| nodes.iter().find(|n| n["id"] == frame["map"]["position"]))
                .is_some_and(|n| n["room"] == false)
        );
        let raw = json!({"frame":frame,"units":o.units,"round":o.round,"actor":o.actor,"torch":o.torch,"retreat_legal":o.actions.iter().any(|a|a.action==crate::Action::Retreat)});
        let mut input = mc::corpus::input_from_frame(game, &raw, o.units.clone())?;
        input.observation = o.clone();
        input.mechanisms = self.inner.mechanisms;
        input.single_target_fallback = !self.inner.enabled.is_empty();
        let sources = self.inner.sources_visible026(view)?;
        let ranked = s25::combat::ranked026(
            o,
            &self.inner.memory,
            &s25::Context {
                profile: &self.inner.profile,
                sources: &sources,
                enabled: &self.inner.enabled,
                ordered: self.inner.ordered,
            },
        )?;
        let defs = mc::Definitions::project(game, &input)?;
        let d = mc::choose(&input, &defs, &self.settings, &ranked)?;
        Ok((input, d))
    }
    pub fn step(&mut self, recording: bool) -> Result<Option<Value>> {
        let record_retreats = !self.inner.enabled.is_empty()
            && self.inner.profile.parameters["sync026"]["field"]["enabled"] == true;
        let diagnostics = !self.inner.enabled.is_empty()
            && self.inner.profile.parameters["sync026"]["diagnostics"] == true;
        if !self.options.rollout && !record_retreats && !diagnostics {
            return self.inner.step(recording);
        }
        let view = self.observation()?;
        let battle = match &view {
            View::Opening(o) => o.battle.as_ref(),
            View::Campaign(o) => o
                .expedition
                .as_ref()
                .and_then(|r| r.battle.as_ref())
                .or_else(|| {
                    o.content019
                        .as_ref()
                        .and_then(|c| c.farm.as_ref())
                        .map(|f| &f.battle)
                }),
        };
        let bridge = if record_retreats && battle.is_some() {
            Some(self.frame()?)
        } else {
            None
        };
        let mut result = None;
        let mut proposal = None;
        let mut audit = self.audit.clone();
        if let Some(o) = battle.filter(|b| {
            self.options.rollout
                && b.outcome.is_none()
                && (b.player_selection
                    || b.actor.is_some_and(|id| {
                        b.units
                            .iter()
                            .any(|u| u.id == id && u.hero && u.controlled.is_none())
                    }))
        }) {
            let frame = if let Some(f) = &bridge {
                f.clone()
            } else {
                self.frame()?
            };
            match crate::execution025::hypothetical026(self.inner.mechanisms, || {
                crate::execution024b::hypothetical026(!self.inner.enabled.is_empty(), || {
                    self.decide(o, &frame, &view)
                })
            }) {
                Ok((input, d)) => {
                    audit.decisions += 1;
                    audit.simulations += d.simulations as u64;
                    let key = serde_json::to_string(&(
                        &frame["context"],
                        &frame["map"]["position"],
                        o.wave,
                    ))
                    .map_err(|e| e.to_string())?;
                    if audit.captures.len() < 120 && Arc::make_mut(&mut audit.seen).insert(key) {
                        Arc::make_mut(&mut audit.captures).push(json!({"input":input,"coverage":"complete live Observation roundtrip","frame_context":frame["context"]}));
                    }
                    let action = if matches!(&view,View::Campaign(c) if c.content019.as_ref().is_some_and(|c|c.farm.is_some()))
                    {
                        Action::Farm(crate::campaign017::content019::farm::Action::Fight(
                            d.action.clone(),
                        ))
                    } else {
                        Action::Raid(crate::opening015::Action::Fight(d.action.clone()))
                    };
                    proposal = Some(s25::Proposal {
                        action,
                        rule: "S026-01".into(),
                        stage: "visible_flat_rollout".into(),
                        cost: 0,
                        purpose: s25::ledger::Purpose::Training,
                    });
                    result = Some(json!({"decision":d}));
                }
                Err(e) => {
                    *audit.fallback_reasons.entry(e.clone()).or_default() += 1;
                    result = Some(json!({"fallback":e}));
                }
            }
        }
        let log = self.inner.step_selected026(recording, proposal)?;
        if record_retreats
            && let Some(log) = &log
            && let Some(mut v) = s25::sync026::field::retreat_record(&view, log, bridge.as_ref())
        {
            // A public view is independent of the journal verbosity.
            v["after"] = serde_json::to_value(self.observation()?).map_err(|e| e.to_string())?;
            Arc::make_mut(&mut audit.retreats).push(v);
        }
        if diagnostics && let Some(log) = &log {
            let after = self.observation()?;
            audit.diagnostics.observe(
                &view,
                &after,
                log,
                &self.inner.memory,
                &self.inner.profile,
                &self.inner.sources,
            );
        }
        self.audit = audit;
        if !self.options.rollout {
            return Ok(log);
        }
        Ok(log.map(|mut log| {
            log["rollout026"] = json!({"options":self.options,"detail":result});
            log
        }))
    }
}

