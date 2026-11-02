//! Independent026 host. Only public views/frame and original definitions enter policy.
use super::*;
use crate::{rollout027 as mc, strategy025 as s25};
use std::sync::Arc;
use strategy025::Runtime025;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    #[serde(default, skip_serializing_if = "disabled")]
    pub coherence029: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub start: String,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub baseline_goals: BTreeSet<String>,
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
    #[serde(default)]
    pub history: mc::PublicHistory,
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
pub struct Runtime027 {
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
impl Runtime027 {
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
                coherence029: false,
                start: "old-road".into(),
                baseline_goals: BTreeSet::new(),
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
            profile: "candidate-rollout-027-v1".into(),
            options: self.options.clone(),
            settings: self.settings.clone(),
            inner: self.inner.checkpoint()?,
            audit: self.audit.clone(),
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str) -> Result<Self> {
        let s: Save = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.profile != "candidate-rollout-027-v1"
            || serde_json::to_value({
                let mut settings = s.settings.clone();
                settings.coherence029 = false;
                settings
            })
            .map_err(|e| e.to_string())?
                != serde_json::to_value(mc::Settings::budgeted()?).map_err(|e| e.to_string())?
            || s.options.coherence029 != s.settings.coherence029
            || s.options.samples != s.settings.samples
        {
            return Err("026 settings binding mismatch".into());
        }
        Ok(Self {
            inner: {
                let mut inner = Runtime025::restore_profile026(&s.inner, {
                    let mut p = profile(s.options.legacy_rooms)?;
                    if s.options.coherence029 {
                        Arc::make_mut(&mut p).parameters["coherence029"] = json!(true);
                    }
                    p
                })?;
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
    /// Separate candidate; old entrypoints and fully disabled groups remain unchanged.
    pub fn coherence029(&mut self) {
        if !self.inner.enabled.is_empty() {
            self.options.coherence029 = true;
            self.settings.coherence029 = true;
            Arc::make_mut(&mut self.inner.profile).parameters["coherence029"] = json!(true);
        }
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
    pub(super) fn public_input(
        &self,
        o: &crate::Observation,
        frame: &Value,
        view: &View,
    ) -> Result<mc::PublicBattle> {
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
        input.history = self.audit.history.clone();
        input.rescue.visible_map = frame.get("map").filter(|v| !v.is_null()).cloned();
        let rules = game.overlay.json("shared/rules.json")?;
        input.rescue.food_heal = rules["provision_hp_heal"]
            .as_f64()
            .ok_or("027 missing food healing")?;
        input.rescue.food = frame["inventory"]["slots"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|x| x["item"]["kind"] == "provision")
            .filter_map(|x| x["amount"].as_u64())
            .sum::<u64>() as u32;
        let raid = match view {
            View::Opening(r) => Some(r.as_ref()),
            View::Campaign(c) => c.expedition.as_ref(),
        };
        if let Some(raid) = raid {
            for u in o.units.iter().filter(|u| u.hero) {
                let guid = frame["battle_heroes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|x| x["unit_id"].as_u64() == Some(u.id as u64))
                    .and_then(|x| x["guid"].as_u64());
                if let Some(h) = raid.heroes.iter().find(|h| Some(h.guid) == guid) {
                    input.rescue.food_remaining.insert(u.id, h.food_remaining);
                }
            }
            input.rescue.camp = raid.actions.contains(&crate::opening015::Action::Camp(
                crate::expedition017::CampAction::Start,
            ));
            if let Some(meal) = rules["meals_table"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|m| m["stress"].as_i64().is_some_and(|n| n < 0))
            {
                input.rescue.camp_relief = -meal["stress"].as_i64().unwrap_or(0) as i32;
                input.rescue.camp_food = (meal["rations_per"].as_f64().unwrap_or(0.)
                    * raid.party.len() as f64)
                    .ceil() as u32;
            }
        }
        input.mechanisms = self.inner.mechanisms;
        input.single_target_fallback = !self.inner.enabled.is_empty();
        Ok(input)
    }
    fn decide(
        &self,
        o: &crate::Observation,
        frame: &Value,
        view: &View,
    ) -> Result<(mc::PublicBattle, mc::Decision)> {
        let input = self.public_input(o, frame, view)?;
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
        let defs = mc::Definitions::project029(game, &input, self.options.coherence029)?;
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
        let mut tracking = None;
        let mut result = None;
        let mut proposal = None;
        let mut audit = self.audit.clone();
        // W-126 is shared by the rule entry as well as rollout. Only replace an
        // otherwise selected retreat; every other rule score/decision is retained.
        if self.options.coherence029
            && !self.options.rollout
            && matches!(&view, View::Campaign(_))
            && let Some(o) = battle.filter(|b| b.outcome.is_none() && !b.player_selection)
        {
            let frame = self.frame()?;
            let sources = self.inner.sources_visible026(&view)?;
            let ctx = s25::Context {
                profile: &self.inner.profile,
                sources: &sources,
                enabled: &self.inner.enabled,
                ordered: self.inner.ordered,
            };
            let mut memory = self.inner.memory.clone();
            let primary = s25::combat::choose(o, &mut memory, &ctx)?;
            let selected = match primary {
                Some(c) => Some(c),
                None => self.inner.inner.replay.inner.visible_choice029(o)?,
            };
            if selected.is_some_and(|(a, _)| a == crate::Action::Retreat) {
                let allowed = (|| -> Result<bool> {
                    let input = self.public_input(o, &frame, &view)?;
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
                    let defs =
                        mc::Definitions::project029(game, &input, self.options.coherence029)?;
                    let model = mc::reconstruct(&input, &defs, 0)?;
                    Ok(mc::retreat::assess(&model, &input, &self.settings)?.allowed)
                })()
                .unwrap_or(false);
                if !allowed {
                    let mut safe = o.clone();
                    safe.actions.retain(|a| a.action != crate::Action::Retreat);
                    let primary = s25::combat::choose(&safe, &mut memory, &ctx)?;
                    let safe_choice = match primary {
                        Some(c) => Some(c),
                        None => self.inner.inner.replay.inner.visible_choice029(&safe)?,
                    };
                    let safe_choice = safe_choice.or_else(|| {
                        use crate::Policy;
                        crate::Baseline
                            .choose(&safe)
                            .map(|a| (a, "S027-W126-fallback".into()))
                    });
                    if let Some((a, rule)) = safe_choice {
                        proposal = Some(s25::Proposal {
                            action: Action::Raid(crate::opening015::Action::Fight(a)),
                            rule,
                            stage: "W-126_rule_retreat_gate".into(),
                            cost: 0,
                            purpose: s25::ledger::Purpose::Training,
                        });
                    }
                }
            }
        }

        if let Some(o) = battle.filter(|b| {
            b.outcome.is_none()
                && b.actor
                    .is_some_and(|id| b.units.iter().any(|u| u.id == id && u.hero))
        }) {
            let frame = self.frame()?;
            let key =
                serde_json::to_string(&(&frame["context"], &frame["map"]["position"], o.wave))
                    .map_err(|e| e.to_string())?;
            if audit.captures.len() < 120 && Arc::make_mut(&mut audit.seen).insert(key) {
                match self.public_input(o,&frame,&view) {
                    Ok(input)=>Arc::make_mut(&mut audit.captures).push(json!({"input":input,"coverage":"complete live Observation","frame_context":frame["context"]})),
                    Err(error)=>Arc::make_mut(&mut audit.captures).push(json!({"capture_error":error,"frame_context":frame["context"]})),
                }
            }
        }

        // Runtime025's selected-action executor only accepts Campaign actors.
        // Fixed opening sessions retain their existing native opening policy.
        if self.options.rollout && matches!(&view, View::Opening(_)) && battle.is_some() {
            *audit
                .fallback_reasons
                .entry("opening_legacy_policy_campaign_override_unavailable".into())
                .or_default() += 1;
        }
        if let Some(o) = battle.filter(|b| {
            self.options.rollout
                && matches!(&view, View::Campaign(_))
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
                    tracking = Some(input.clone());
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
                    // W-126 also governs the fallback path, without changing old entrypoints.
                    let mut safe_view = o.clone();
                    safe_view
                        .actions
                        .retain(|a| a.action != crate::Action::Retreat);
                    let sources = self.inner.sources_visible026(&view)?;
                    let ranked = s25::combat::ranked026(
                        &safe_view,
                        &self.inner.memory,
                        &s25::Context {
                            profile: &self.inner.profile,
                            sources: &sources,
                            enabled: &self.inner.enabled,
                            ordered: self.inner.ordered,
                        },
                    )?;
                    let safe = ranked
                        .first()
                        .map(|(_, a, _)| a)
                        .or_else(|| safe_view.actions.first())
                        .ok_or("w126_no_nonretreat_action")?;
                    let action = if matches!(&view,View::Campaign(c) if c.content019.as_ref().is_some_and(|c|c.farm.is_some()))
                    {
                        Action::Farm(crate::campaign017::content019::farm::Action::Fight(
                            safe.action.clone(),
                        ))
                    } else {
                        Action::Raid(crate::opening015::Action::Fight(safe.action.clone()))
                    };
                    proposal = Some(s25::Proposal {
                        action,
                        rule: "S027-W126-fallback".into(),
                        stage: "visible_flat_rollout".into(),
                        cost: 0,
                        purpose: s25::ledger::Purpose::Training,
                    });
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
        if let Some(input) = tracking {
            if let Some(log) = &log {
                let value = log["action"]["Raid"]
                    .get("Fight")
                    .or_else(|| log["action"]["Farm"].get("Fight"));
                let after = self.observation()?;
                let after = match &after {
                    View::Opening(o) => o.battle.as_ref(),
                    View::Campaign(o) => o.expedition.as_ref().and_then(|e| e.battle.as_ref()),
                };
                if let (Some(value), Some(after)) = (value, after) {
                    let action =
                        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
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
                    let defs =
                        mc::Definitions::project029(game, &input, self.options.coherence029)?;
                    audit
                        .history
                        .observe(&input.observation, &action, after, &defs);
                } else {
                    audit.history = Default::default();
                }
            }
        } else if battle.is_none() {
            audit.history = Default::default();
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


#[path = "runner027_start.rs"]
mod start;

#[path = "funnel028.rs"]
pub mod funnel028;
