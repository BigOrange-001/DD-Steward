//! Human actions cross the same native execution boundary; all presentation uses View.
use super::*;
use crate::{rollout027 as mc, strategy025 as s25};
use rollout027::Runtime027;
use std::sync::Arc;
#[path = "human030_advice.rs"]
pub mod advice;

fn val(v: impl Serialize) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}
fn arena_summary(o: &crate::Observation, input: &mc::PublicBattle) -> Value {
    let heroes=o.units.iter().filter(|u|u.hero&&!u.corpse).map(|u|json!({"guid":u.id+1,"class":u.class,"name":u.class,"level":input.heroes.get(&u.id).map(|h|h.level),"hp":u.hp,"max_hp":u.max_hp,"maximum_hp":u.max_hp,"stress":u.stress})).collect::<Vec<_>>();
    json!({"phase":"arena","deaths":input.heroes.len().saturating_sub(heroes.len()),"heroes":heroes,"expedition":{"heroes":heroes,"party":input.heroes.keys().map(|id|id+1).collect::<Vec<_>>(),"battle":{"round":o.round,"actor":o.actor,"units":o.units,"enemies":o.units.iter().filter(|u|!u.hero&&!u.corpse).collect::<Vec<_>>(),"outcome":o.outcome},"torch":o.torch,"food":input.rescue.food,"quest_ready":false,"camping":false,"can_retreat":o.actions.iter().any(|a|a.action==crate::Action::Retreat),"node":null,"known_geometry":0,"visited_known":0}})
}
pub enum World {
    Campaign(Box<Runtime027>),
    Arena(Box<mc::human030::Arena>),
}
fn advice_source(world: &World) -> Result<Arc<GameData>> {
    let game = match world {
        World::Campaign(r) => &r.inner.inner.replay.inner.inner.inner.definition.town.game,
        World::Arena(a) => &a.game,
    };
    GameData::load_with_content(&game.root, game.overlay.config.clone()).map(Arc::new)
}
pub struct Human {
    pub world: World,
    pub steps: u64,
    pub hints: bool,
    pub notes: Vec<String>,
    pub thinking: f64,
    pub memory: s25::Memory,
    pub memory21: crate::tactics021::Memory,
    pub history: mc::PublicHistory,
    encoder: replay022::Encoder,
    cache: Option<advice::Cache>,
    advice_game: Arc<GameData>,
    prior_hints: Value,
}
impl Human {
    pub fn campaign(config: Config, start: &str, hints: bool) -> Result<Self> {
        let mut r = Runtime027::new(config, "all")?;
        r.coherence029();
        r.test_start(start)?;
        Self::from_world(World::Campaign(Box::new(r)), hints)
    }
    pub fn arena(id: &str, input: mc::PublicBattle, seed: u64, hints: bool) -> Result<Self> {
        Self::from_world(
            World::Arena(Box::new(mc::human030::Arena::new(id, input, seed)?)),
            hints,
        )
    }
    fn from_world(world: World, hints: bool) -> Result<Self> {
        let advice_game = advice_source(&world)?;
        Ok(Self {
            world,
            steps: 0,
            hints,
            notes: vec![],
            thinking: 0.,
            memory: Default::default(),
            memory21: Default::default(),
            history: Default::default(),
            encoder: Default::default(),
            cache: None,
            advice_game,
            prior_hints: Value::Null,
        })
    }
    pub fn game(&self) -> &GameData {
        match &self.world {
            World::Campaign(r) => &r.inner.inner.replay.inner.inner.inner.definition.town.game,
            World::Arena(a) => &a.game,
        }
    }
    pub fn localization_files(&self) -> Vec<String> {
        let overlay = &self.game().overlay;
        let mut files = vec![];
        for (p, indices) in &overlay.paths {
            if !p.starts_with("localization/")
                || !p.ends_with(".string_table.xml")
                || p.contains("arena")
                || p.contains("pvp")
            {
                continue;
            }
            for i in indices {
                let layer = &overlay.layers[*i];
                if layer.package == "base"
                    && !layer.mode
                    && ![
                        "heroes",
                        "miscellaneous",
                        "curios",
                        "names",
                        "party_names",
                        "backertrinkets",
                    ]
                    .iter()
                    .any(|name| p == &format!("localization/{name}.string_table.xml"))
                {
                    continue;
                }
                files.push((*i, layer.root.join(p).to_string_lossy().into_owned()));
            }
        }
        files.sort();
        files.into_iter().map(|(_, p)| p).collect()
    }
    pub fn observation(&mut self) -> Result<Value> {
        match &mut self.world {
            World::Campaign(r) => val(r.observation()?),
            World::Arena(a) => Ok(json!({"Arena":a.observation()})),
        }
    }
    pub fn frame(&mut self) -> Result<Value> {
        match &mut self.world {
            World::Campaign(r) => r.frame(),
            World::Arena(a) => {
                let o = a.observation();
                let roster=o.units.iter().filter(|u|u.hero).map(|u|json!({"guid":u.id+1,"class":u.class,"name":u.class,"hp":u.hp,"stress":u.stress,"level":a.input.heroes.get(&u.id).map(|h|h.level)})).collect::<Vec<_>>();
                Ok(
                    json!({"context":{"phase":"arena","scene":a.id},"map":null,"inventory":{"slots":o.supplies.iter().map(|(id,n)|if *n==0{Value::Null}else{json!({"item":{"kind":"supply","id":id},"amount":n})}).collect::<Vec<_>>()},"loot":[],"item_states":[],"quest":null,"goal_progress":null,"roster":roster,"party_build":a.input.heroes.iter().map(|(id,h)|json!({"guid":id+1,"weapon":h.weapon,"armour":h.armour,"skills":h.skills.iter().cloned().collect::<BTreeMap<_,_>>(),"selected":h.skills.iter().map(|s|&s.0).collect::<Vec<_>>(),"equipped":[]})).collect::<Vec<_>>(),"battle_heroes":o.units.iter().filter(|u|u.hero).map(|u|json!({"unit_id":u.id,"guid":u.id+1})).collect::<Vec<_>>() }),
                )
            }
        }
    }
    pub fn status(&mut self) -> Result<Value> {
        match &mut self.world {
            World::Campaign(r) => {
                r.visible()?;
                val(&r.inner.inner.replay.inner.inner.inner.status)
            }
            World::Arena(a) => val(match a.observation().outcome {
                None => Status::Running,
                Some(crate::Outcome::HeroesWin) => Status::Success,
                Some(
                    crate::Outcome::MonstersWin
                    | crate::Outcome::MutualDefeat
                    | crate::Outcome::FlashbackFailed,
                ) => Status::GameOver,
                Some(_) => Status::Timeout,
            }),
        }
    }
    pub fn public(&mut self) -> Result<Value> {
        let status = self.status()?;
        let observation = self.observation()?;
        let menu = if status == "Running" {
            menu(&observation)?
        } else {
            vec![]
        };
        if status == "Running" {
            self.prepare_advice();
        }
        let hints = self
            .cache
            .as_ref()
            .map_or(json!({"enabled":self.hints}), |c| c.visible(self.hints));
        Ok(
            json!({"protocol":"dd-human-030-v1","step":self.steps+1,"status":status,"observation":observation,"frame":self.frame()?,"actions":menu,"notes":self.notes,"hints":hints}),
        )
    }
    fn prepare_advice(&mut self) {
        if self.cache.is_none() {
            self.cache = Some(advice::prepare(self).unwrap_or_else(|e| advice::Cache {
                rule: json!({"error":e}),
                job: None,
                model_error: None,
                defs: None,
                memory: self.memory.clone(),
                memory21: self.memory21.clone(),
                mc: None,
                worker: None,
                seen_rule: false,
                seen_mc: false,
            }));
        }
        if let Some(c) = &mut self.cache {
            if self.prior_hints["rules_shown"] == true {
                c.seen_rule = true;
                c.rule = self.prior_hints["rules"].clone();
            }
            if self.prior_hints["rollout_shown"] == true {
                c.seen_mc = true;
                c.mc = Some(self.prior_hints["rollout"].clone());
            }
            c.poll(self.hints);
        }
        self.prior_hints = Value::Null;
    }
    /// The interface acknowledges only recommendations actually rendered.
    pub fn seen(&mut self, rules: bool, rollout: bool) {
        if self.hints
            && let Some(c) = &mut self.cache
        {
            c.seen_rule |= rules && c.rule.get("action").is_some();
            c.seen_mc |= rollout && c.mc.as_ref().is_some_and(|m| m.get("action").is_some());
        }
    }
    pub fn wait_advice(&mut self) -> Result<Value> {
        self.prepare_advice();
        if let Some(c) = &mut self.cache
            && let Some(w) = c.worker.take()
        {
            c.mc = Some(
                w.join()
                    .unwrap_or_else(|_| Err("030 advice worker panicked".into()))
                    .unwrap_or_else(|e| json!({"error":e})),
            );
        }
        self.public()
    }
    pub fn native(&self) -> Result<String> {
        match &self.world {
            World::Campaign(r) => {
                let root = &r.inner.inner.replay.inner.inner.inner;
                let (stage, s) = match &root.actor {
                    Actor::Opening(s) => ("opening", s.checkpoint()?),
                    Actor::Campaign(s) => ("campaign", s.checkpoint()?),
                };
                serde_json::to_string(&(stage, s)).map_err(|e| e.to_string())
            }
            World::Arena(a) => a.native(),
        }
    }
    pub fn save(&self) -> Result<Value> {
        Ok(
            json!({"version":"human030-private-v1","world":match &self.world {World::Campaign(r)=>json!({"Campaign":r.checkpoint()?}),World::Arena(a)=>json!({"Arena":a.save()?})},"steps":self.steps,"hints":self.hints,"pending_hints":self.cache.as_ref().map_or(self.prior_hints.clone(),|c|json!({"rules_shown":c.seen_rule,"rollout_shown":c.seen_mc,"rules":c.rule,"rollout":c.mc})),"notes":self.notes,"thinking":self.thinking,"memory":self.memory,"memory21":self.memory21,"history":self.history,"encoder":self.encoder}),
        )
    }
    pub fn restore(v: &Value) -> Result<Self> {
        if v["version"] != "human030-private-v1" {
            return Err("030 save version".into());
        }
        let world = if let Some(s) = v["world"]["Campaign"].as_str() {
            World::Campaign(Box::new(Runtime027::restore(s)?))
        } else {
            World::Arena(Box::new(mc::human030::Arena::restore(
                &v["world"]["Arena"],
            )?))
        };
        let advice_game = advice_source(&world)?;
        Ok(Self {
            world,
            steps: v["steps"].as_u64().ok_or("030 steps")?,
            hints: v["hints"].as_bool().ok_or("030 hints")?,
            notes: serde_json::from_value(v["notes"].clone()).map_err(|e| e.to_string())?,
            thinking: v["thinking"].as_f64().ok_or("030 thinking")?,
            memory: serde_json::from_value(v["memory"].clone()).map_err(|e| e.to_string())?,
            memory21: serde_json::from_value(v["memory21"].clone()).map_err(|e| e.to_string())?,
            history: serde_json::from_value(v["history"].clone()).map_err(|e| e.to_string())?,
            encoder: serde_json::from_value(v["encoder"].clone()).map_err(|e| e.to_string())?,
            cache: None,
            advice_game,
            prior_hints: v.get("pending_hints").cloned().unwrap_or(Value::Null),
        })
    }
    pub fn act(&mut self, action: Value, confirm: bool, thinking: f64) -> Result<Value> {
        if !thinking.is_finite() || thinking < 0. {
            return Err("invalid thinking time".into());
        }
        let public = self.public()?;
        let prior_moves = self.memory.legacy.moves;
        validate(&public, &action)?;
        if irreversible(&action) && !confirm {
            return Err("confirmation_required".into());
        }
        let before_view = match &mut self.world {
            World::Campaign(r) => Some(r.observation()?),
            _ => None,
        };
        let mut log = match &mut self.world {
            World::Campaign(r) => crate::execution025::with_options(r.inner.mechanisms, || {
                crate::execution024b::run(true, || {
                    r.inner.inner.replay.step_using023(|r| execute(r, &action))
                })
                .map(|r| r.0)
            })?
            .ok_or("030 no execution")?,
            World::Arena(a) => {
                let before = a.observation();
                let mut before_summary = arena_summary(&before, &a.input);
                let before_frame = public["frame"].clone();
                let selected: crate::Action =
                    serde_json::from_value(action.clone()).map_err(|e| e.to_string())?;
                let legal = before
                    .actions
                    .iter()
                    .find(|x| x.action == selected)
                    .ok_or("030 illegal arena")?
                    .clone();
                a.act(selected.clone())?;
                let after = a.observation();
                let mut after_summary = arena_summary(&after, &a.input);
                let after_frame = self.frame()?;
                let bp = self.encoder.encode(&before_frame)?;
                let ap = self.encoder.encode(&after_frame)?;
                before_summary["replay"] = bp;
                after_summary["replay"] = ap;
                let mut encounters = vec![];
                if self.steps == 0 {
                    encounters.push(
                        json!({"kind":"start","id":1,"at":"before","boundary":"observed_entry"}),
                    );
                }
                if after.outcome.is_some() {
                    encounters.push(json!({"kind":"end","id":1,"at":"after","outcome":after.outcome,"boundary":"observed_exit"}));
                }
                json!({"schema_version":replay022::SCHEMA,"step":self.steps+1,"action":{"Opening":{"Fight":selected}},"reason":"human","before":before_summary,"after":after_summary,"status":self.status()?,"combat_action":{"actor":before.actor,"skill_id":if matches!(selected,crate::Action::Skill{..}|crate::Action::SkillAuto{..}){Some(&legal.label)}else{None},"selected_target":match selected {crate::Action::Skill{target,..}|crate::Action::Item{target,..}|crate::Action::Sacrifice{target}=>Some(target),_=>None},"targets":legal.targets,"action":selected},"encounters":encounters})
            }
        };
        self.steps += 1;
        log["decision_maker"] = json!("human");
        let mut hints=self.cache.as_ref().map_or(json!({"enabled":self.hints,"shown":false}),|c|{
            json!({"enabled":self.hints,"shown":c.seen_rule||c.seen_mc,"rules_shown":c.seen_rule,"rollout_shown":c.seen_mc,"rules":if c.seen_rule{Some(&c.rule)}else{None},"rollout":if c.seen_mc{c.mc.as_ref()}else{None}})
        });
        hints["followed_rules"] =
            json!(hints["rules_shown"] == true && hints["rules"]["action"] == action);
        hints["followed_rollout"] =
            json!(hints["rollout_shown"] == true && hints["rollout"]["action"] == action);
        let shadow=self.cache.as_ref().map(|c|json!({"rules":c.rule,"job":c.job,"unscored_rollout":if c.job.is_none(){c.mc.as_ref()}else{None},"model_error":c.model_error,"memory":"approximate adviser intentions, updated only on committed human actions"}));
        log["human"] = json!({"action":action,"legal_actions":public["actions"],"notes":self.notes,"thinking_seconds":self.thinking+thinking,"hints":hints,"shadow":shadow});
        if let Some(c) = self.cache.take() {
            self.memory = c.memory;
            self.memory21 = c.memory21;
            let actual = if matches!(self.world, World::Arena(_)) {
                Some(action.clone())
            } else {
                action
                    .get("Opening")
                    .or_else(|| action.get("Raid"))
                    .or_else(|| action.get("Farm"))
                    .and_then(|v| v.get("Fight"))
                    .cloned()
            };
            if let (Some(job), Some(raw)) = (c.job, actual)
                && let Ok(a) = serde_json::from_value::<crate::Action>(raw)
            {
                if let Ok(sources) = s25::Sources::project(self.game()) {
                    s25::combat::finish(&job.input.observation, &a, &mut self.memory, &sources);
                }
                let current = self.observation()?;
                let after = if let Some(b) = current.get("Arena") {
                    Some(b.clone())
                } else {
                    current
                        .get("Opening")
                        .and_then(|o| o.get("battle"))
                        .or_else(|| current["Campaign"]["expedition"].get("battle"))
                        .or_else(|| current["Campaign"]["content019"]["farm"].get("battle"))
                        .cloned()
                };
                if let Some(v) = after
                    && let Ok(o) = serde_json::from_value::<crate::Observation>(v)
                    && let Ok(defs) = mc::Definitions::project029(self.game(), &job.input, true)
                {
                    self.history.observe(&job.input.observation, &a, &o, &defs);
                }
            }
        }
        if let Some(View::Campaign(before)) = before_view.as_ref() {
            if let Ok(a) = serde_json::from_value::<Action>(action.clone()) {
                if let Ok(cost) = s25::price(before, &a) {
                    self.memory.ledger.record(
                        &s25::Proposal {
                            purpose: s25::purpose(&a),
                            action: a.clone(),
                            rule: "human".into(),
                            stage: "human actual trajectory".into(),
                            cost,
                        },
                        before.week,
                    );
                }
                match a {
                    Action::Select(id) => self.memory.legacy.quest = Some(id),
                    Action::Town(crate::town016::Action::Party(ids)) => {
                        self.memory.legacy.target_party = ids
                    }
                    _ => {}
                }
            }
            if let World::Campaign(r) = &mut self.world {
                let after = r.observation()?;
                if let Err(e) =
                    self.memory
                        .ledger
                        .heirlooms
                        .record(before_view.as_ref().unwrap(), &after, &log)
                {
                    log["human"]["shadow"]["memory_error"] = json!(e);
                }
                if let View::Campaign(o) = &after
                    && self.memory.metrics.needs_observe(o, Some(&log))
                {
                    Arc::make_mut(&mut self.memory.metrics).observe(
                        o,
                        Some(&log),
                        &self.memory.ledger,
                    );
                }
            }
        }
        self.memory.legacy.moves = prior_moves
            + u32::from(
                action
                    .get("Raid")
                    .or_else(|| action.get("Opening"))
                    .is_some_and(|v| v.get("Move").is_some()),
            );
        if let Some(v) = &before_view {
            self.memory.usage.record(v, &log);
        }
        s25::recovery::committed(&mut self.memory, &log);
        self.notes.clear();
        self.thinking = 0.;
        Ok(log)
    }
}
fn listed(actions: &Value, wrapper: &str) -> Vec<Value> {
    actions
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| json!({wrapper:a}))
        .collect()
}
pub fn menu(view: &Value) -> Result<Vec<Value>> {
    let mut actions = if let Some(o) = view.get("Arena") {
        o["actions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| a["action"].clone())
            .collect()
    } else if let Some(o) = view.get("Opening") {
        listed(&o["actions"], "Opening")
    } else {
        let o = &view["Campaign"];
        if o["event_offer"].is_object() {
            listed(&o["event_offer"]["actions"], "Event")
        } else if o["provisions"].is_object() {
            listed(&o["provisions"]["actions"], "Provision")
        } else if o["expedition"].is_object() {
            listed(&o["expedition"]["actions"], "Raid")
        } else if o["content019"]["farm"].is_object() {
            listed(&o["content019"]["farm"]["actions"], "Farm")
        } else {
            let mut rows = o["town"]["actions"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|a| json!({"Town":a["action"]}))
                .collect::<Vec<_>>();
            for q in o["board"]["quests"].as_array().into_iter().flatten() {
                rows.push(json!({"Select":q["id"]}));
                if q["id"] == o["board"]["selected"] && q["preparation_eligible"] == true {
                    rows.push(json!("Prepare"));
                }
            }
            rows
        }
    };
    let mut rows = vec![];
    for a in actions.drain(..) {
        if !rows.iter().any(|r: &Value| r["action"] == a) {
            rows.push(json!({"id":rows.len()+1,"action":a,"confirmation":irreversible(&a)}));
        }
    }
    if view["Campaign"]["expedition"].is_null()
        && view["Campaign"]["provisions"].is_null()
        && view["Campaign"]["town"].is_object()
        && view["Campaign"]["event_offer"].is_null()
        && view["Campaign"]["content019"]["farm"].is_null()
    {
        for family in ["Party", "SelectSkills", "SelectCamping"] {
            rows.push(json!({"id":rows.len()+1,"template":family,"domain":view["Campaign"]["town"]["heroes"]}));
        }
    }
    Ok(rows)
}
pub fn irreversible(a: &Value) -> bool {
    match a {
        Value::String(s) => ["Retreat", "Exit", "LeaveLoot"].contains(&s.as_str()),
        Value::Object(o) => o.iter().any(|(k, v)| {
            ["Leave", "Dismiss", "Sacrifice", "Sell", "Discard"].contains(&k.as_str())
                || irreversible(v)
        }),
        _ => false,
    }
}
pub fn validate(public: &Value, action: &Value) -> Result<()> {
    let rows = public["actions"].as_array().ok_or("030 menu")?;
    if rows.iter().any(|a| a.get("action") == Some(action)) {
        return Ok(());
    }
    let town = &action["Town"];
    for row in rows.iter().filter(|r| r.get("template").is_some()) {
        let family = row["template"].as_str().ok_or("030 family")?;
        if let Some(args) = town.get(family) {
            let heroes = row["domain"].as_array().ok_or("030 domain")?;
            if family == "Party" {
                let ids = args.as_array().ok_or("030 party must be an array")?;
                if ids.len() <= 4
                    && ids
                        .iter()
                        .collect::<Vec<_>>()
                        .iter()
                        .enumerate()
                        .all(|(i, x)| !ids[..i].contains(x))
                    && ids.iter().all(|id| {
                        heroes.iter().any(|h| {
                            h["guid"] == *id
                                && h["alive"] == true
                                && h["busy"] == false
                                && h["missing"] == false
                        })
                    })
                {
                    return Ok(());
                }
            } else if let Some(hero) = heroes.iter().find(|h| h["guid"] == args["hero"]) {
                let skills = args["skills"].as_array().ok_or("030 skill array")?;
                if skills.len() <= 4
                    && skills.iter().enumerate().all(|(i, s)| {
                        !skills[..i].contains(s)
                            && if family == "SelectSkills" {
                                s.as_str().is_some_and(|s| hero["skills"].get(s).is_some())
                            } else {
                                hero["camping"].as_array().is_some_and(|x| x.contains(s))
                            }
                    })
                {
                    return Ok(());
                }
            }
        }
    }
    Err("action_not_in_public_menu".into())
}
fn execute(r: &mut arena021::Runtime021, action: &Value) -> Result<Option<Record>> {
    let root = &mut r.inner.inner;
    let before = root.observe()?;
    root.terminal(&before)?;
    if root.status != Status::Running {
        return Ok(None);
    }
    let before_summary = summary(&before)?;
    match &mut root.actor {
        Actor::Opening(s) => {
            let a = serde_json::from_value(action["Opening"].clone()).map_err(|e| e.to_string())?;
            s.act(a)?;
            if s.observe()?.phase == crate::opening015::Phase::Finished {
                let handoff = s.town_handoff()?;
                root.actor = Actor::Campaign(Box::new(Session::from_opening(
                    root.definition.clone(),
                    handoff,
                    root.config.seed,
                )?));
            }
        }
        Actor::Campaign(s) => {
            let a: Action = serde_json::from_value(action.clone()).map_err(|e| e.to_string())?;
            match a {
                Action::Town(a) => s.town_action(a)?,
                Action::Event(a) => s.event_action(a)?,
                Action::Select(id) => s.select_quest(Some(id))?,
                Action::Prepare => s.prepare_quest()?,
                Action::Provision(a) => s.provision_action(a)?,
                Action::Raid(a) => s.expedition_action(a)?,
                Action::Farm(a) => s.farm019_action(a)?,
            }
        }
    }
    root.decisions += 1;
    root.cached = None;
    let after = root.observe()?;
    root.terminal(&after)?;
    let mut record = Record {
        step: root.decisions,
        action: action.clone(),
        reason: "human".into(),
        before: before_summary,
        after: summary(&after)?,
        status: root.status.clone(),
    };
    r.inner.enrich(&mut record.before)?;
    r.inner.enrich(&mut record.after)?;
    Ok(Some(record))
}

