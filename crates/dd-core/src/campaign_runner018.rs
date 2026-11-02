//! Host runtime and public text monitor; the policy never receives this module's save state.
use super::*;
use crate::auto_policy018::{self, Action, Decision, Memory};
use serde::{Deserialize, Serialize};
pub const RUN_PROFILE: &str = "candidate-auto-018";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mode: String,
    pub never_again: String,
    pub seed: u64,
    pub week_limit: u32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: "radiant".into(),
            never_again: "strict".into(),
            seed: 1,
            week_limit: 208,
        }
    }
}
impl Config {
    pub fn definition(&self) -> Result<Definition> {
        if !(2..=208).contains(&self.week_limit) {
            return Err("week limit must be between 2 and 208".into());
        }
        let mut c = dd_data::content::ContentConfig::full().with_campaign018_candidates()?;
        c.mode = self.mode.clone();
        c.never_again = self.never_again.clone();
        c.validate()?;
        Definition::load(
            GameData::load_with_content(&dd_data::game_dir(), c)?,
            CampaignCandidates::documented()?,
        )?
        .with_automation018()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    Running,
    Success,
    Timeout,
    GameOver,
    Error(String),
}
#[derive(Clone)]
enum Actor {
    Opening(Box<crate::opening015::Session>),
    Campaign(Box<Session>),
}
#[derive(Clone, Serialize)]
pub enum View {
    Opening(Box<crate::opening015::Observation>),
    Campaign(Box<Observation>),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Save {
    profile: String,
    definition: String,
    run_id: String,
    config: Config,
    memory: Memory,
    decisions: u64,
    status: Status,
    stage: String,
    snapshot: String,
}
#[derive(Clone)]
pub struct Runtime {
    definition: Definition,
    run_id: String,
    pub config: Config,
    pub memory: Memory,
    pub decisions: u64,
    pub status: Status,
    actor: Actor,
    cached: Option<View>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Record {
    pub step: u64,
    pub action: Value,
    pub reason: String,
    pub before: Value,
    pub after: Value,
    pub status: Status,
}
fn identity(config: &Config, definition: &Definition) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(format!(
        "{RUN_PROFILE};sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(&(config, &definition.id, auto_policy018::VERSION))
                .map_err(|e| e.to_string())?
        )
    ))
}
impl Runtime {
    pub fn new(config: Config) -> Result<Self> {
        let definition = config.definition()?;
        let opening =
            crate::opening015::Session::new(definition.town.opening.clone(), config.seed)?;
        Ok(Self {
            run_id: identity(&config, &definition)?,
            definition,
            config,
            memory: Memory::default(),
            decisions: 0,
            status: Status::Running,
            actor: Actor::Opening(Box::new(opening)),
            cached: None,
        })
    }
    pub fn with_definition(config: Config, definition: Definition) -> Result<Self> {
        if !(2..=208).contains(&config.week_limit)
            || config.mode != definition.data.content.mode
            || config.never_again != definition.data.content.never_again
        {
            return Err("invalid automation run configuration".into());
        }
        if definition.automation.is_none() {
            return Err("automation definition required".into());
        }
        let opening =
            crate::opening015::Session::new(definition.town.opening.clone(), config.seed)?;
        Ok(Self {
            run_id: identity(&config, &definition)?,
            definition,
            config,
            memory: Memory::default(),
            decisions: 0,
            status: Status::Running,
            actor: Actor::Opening(Box::new(opening)),
            cached: None,
        })
    }
    pub fn observe(&mut self) -> Result<View> {
        if let Some(o) = &self.cached {
            return Ok(o.clone());
        }
        let o = match &self.actor {
            Actor::Opening(s) => View::Opening(Box::new(s.observe()?)),
            Actor::Campaign(s) => View::Campaign(Box::new(s.observe()?)),
        };
        self.cached = Some(o.clone());
        Ok(o)
    }
    fn terminal(&mut self, o: &View) -> Result<()> {
        if let View::Campaign(c) = o {
            if let Some(p) = &c.steward020 {
                let settled = c.expedition.is_none()
                    && !c.content019.as_ref().is_some_and(|v| v.farm.is_some())
                    && c.provisions.is_none()
                    && !c.event_choice_pending;
                self.status = if c.failure.is_some() {
                    Status::GameOver
                } else if settled && p.complete && c.week <= self.config.week_limit {
                    Status::Success
                } else if settled && c.week >= self.config.week_limit {
                    Status::Timeout
                } else {
                    Status::Running
                };
                return Ok(());
            }
            let done = auto_policy018::goals(c)?.iter().all(|(_, done)| *done);
            self.status = if done && c.week <= self.config.week_limit {
                Status::Success
            } else if c.failure.is_some() {
                Status::GameOver
            } else if c.expedition.is_none()
                && !c.content019.as_ref().is_some_and(|v| v.farm.is_some())
                && c.provisions.is_none()
                && c.week >= self.config.week_limit
            {
                Status::Timeout
            } else {
                Status::Running
            };
        }
        Ok(())
    }
    pub fn refresh(&mut self) -> Result<Value> {
        let o = self.observe()?;
        self.terminal(&o)?;
        summary(&o)
    }
    pub fn step(&mut self) -> Result<Option<Record>> {
        let mut next = self.clone();
        let result = next.step_inner()?;
        *self = next;
        Ok(result)
    }
    fn step_inner(&mut self) -> Result<Option<Record>> {
        self.step_inner_with_navigation(None)
    }
    fn step_inner_with_navigation(
        &mut self,
        navigation: Option<&mut crate::navigation019::Memory>,
    ) -> Result<Option<Record>> {
        self.step_inner_with_combat(navigation, None)
    }
    fn step_inner_with_combat(
        &mut self,
        navigation: Option<&mut crate::navigation019::Memory>,
        mut combat: Option<&mut CombatChoice021<'_>>,
    ) -> Result<Option<Record>> {
        let before = self.observe()?;
        self.terminal(&before)?;
        if self.status != Status::Running {
            return Ok(None);
        }
        let before_summary = summary(&before)?;
        let policy_view023 = mechanisms023::policy_view(&before);
        let (action, reason) = match (&mut self.actor, policy_view023.as_ref().unwrap_or(&before)) {
            (Actor::Opening(s), View::Opening(o)) => {
                let mut a = crate::opening_policy015::choose(o)?;
                let mut reason = "按可见观察完成老路与首次废墟".to_string();
                if matches!(a, crate::opening015::Action::Fight(_))
                    && let (Some(policy), Some(b)) = (combat.as_mut(), o.battle.as_ref())
                {
                    let mut legal = b.clone();
                    legal.actions.retain(|a| {
                        o.actions
                            .contains(&crate::opening015::Action::Fight(a.action.clone()))
                    });
                    if let Some((choice, why)) = policy(&legal)? {
                        a = crate::opening015::Action::Fight(choice);
                        reason = why;
                    }
                }
                s.act(a.clone())?;
                let json = json!({"Opening":a});
                if s.observe()?.phase == crate::opening015::Phase::Finished {
                    let handoff = s.town_handoff()?;
                    self.actor = Actor::Campaign(Box::new(Session::from_opening(
                        self.definition.clone(),
                        handoff,
                        self.config.seed,
                    )?));
                }
                (json, reason)
            }
            (Actor::Campaign(s), View::Campaign(o)) => {
                let mut next = self.memory.clone();
                let Decision {
                    mut action,
                    mut reason,
                } = if let Some(memory) = navigation {
                    crate::navigation019::choose(o, &mut next, memory)?
                } else {
                    auto_policy018::choose(o, &mut next)?
                };
                let battle = combat
                    .as_ref()
                    .and_then(|_| combat_observation021(o, &action));
                if let (Some(policy), Some(b)) = (combat.as_mut(), battle)
                    && let Some((choice, why)) = policy(&b)?
                {
                    action = match action {
                        Action::Farm(_)
                            if choice == crate::Action::Retreat
                                && o.content019
                                    .as_ref()
                                    .and_then(|c| c.farm.as_ref())
                                    .is_some_and(|f| {
                                        f.actions.contains(
                                            &crate::campaign017::content019::farm::Action::Exit,
                                        )
                                    }) =>
                        {
                            Action::Farm(crate::campaign017::content019::farm::Action::Exit)
                        }
                        Action::Farm(_) => Action::Farm(
                            crate::campaign017::content019::farm::Action::Fight(choice),
                        ),
                        _ => Action::Raid(crate::opening015::Action::Fight(choice)),
                    };
                    reason = why;
                }
                match action.clone() {
                    Action::Town(a) => s.town_action(a)?,
                    Action::Event(a) => s.event_action(a)?,
                    Action::Select(id) => s.select_quest(Some(id))?,
                    Action::Prepare => s.prepare_quest()?,
                    Action::Provision(a) => s.provision_action(a)?,
                    Action::Raid(a) => s.expedition_action(a)?,
                    Action::Farm(a) => s.farm019_action(a)?,
                }
                self.memory = next;
                (
                    serde_json::to_value(action).map_err(|e| e.to_string())?,
                    reason,
                )
            }
            _ => return Err("runtime observation/actor phase mismatch".into()),
        };
        self.decisions = self
            .decisions
            .checked_add(1)
            .ok_or("decision counter overflow")?;
        self.cached = None;
        let after = self.observe()?;
        self.terminal(&after)?;
        Ok(Some(Record {
            step: self.decisions,
            action,
            reason,
            before: before_summary,
            after: summary(&after)?,
            status: self.status.clone(),
        }))
    }
    pub fn identity(&self) -> &str {
        &self.run_id
    }
    pub fn source_root(&self) -> &std::path::Path {
        &self.definition.town.game.root
    }
    pub fn checkpoint(&self) -> Result<String> {
        let (stage, snapshot) = match &self.actor {
            Actor::Opening(s) => ("opening", s.checkpoint()?),
            Actor::Campaign(s) => ("campaign", s.checkpoint()?),
        };
        serde_json::to_string(&Save {
            profile: RUN_PROFILE.into(),
            definition: self.definition.id.clone(),
            run_id: self.run_id.clone(),
            config: self.config.clone(),
            memory: self.memory.clone(),
            decisions: self.decisions,
            status: self.status.clone(),
            stage: stage.into(),
            snapshot,
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str, definition: Definition) -> Result<Self> {
        if text.len() > 96 * 1024 * 1024 {
            return Err("automation snapshot exceeds bound".into());
        }
        let s: Save = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.profile != RUN_PROFILE
            || s.definition != definition.id
            || s.run_id != identity(&s.config, &definition)?
            || s.memory.version != auto_policy018::VERSION
            || s.config.mode != definition.data.content.mode
            || s.config.never_again != definition.data.content.never_again
            || !(2..=208).contains(&s.config.week_limit)
        {
            return Err("automation profile/content/policy/config mismatch".into());
        }
        let actor = match s.stage.as_str() {
            "opening" => Actor::Opening(Box::new(crate::opening015::Session::restore(
                &s.snapshot,
                &definition.town.opening,
            )?)),
            "campaign" => Actor::Campaign(Box::new(Session::restore(&s.snapshot, &definition)?)),
            _ => return Err("unknown automation snapshot stage".into()),
        };
        let mut r = Self {
            run_id: s.run_id,
            definition,
            config: s.config,
            memory: s.memory,
            decisions: s.decisions,
            status: s.status.clone(),
            actor,
            cached: None,
        };
        let o = r.observe()?;
        r.terminal(&o)?;
        if !matches!(s.status, Status::Error(_)) && r.status != s.status {
            return Err("forged automation terminal status".into());
        }
        Ok(r)
    }
    pub fn restore_config(text: &str) -> Result<Self> {
        let s: Save = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let d = s.config.definition()?;
        Self::restore(text, d)
    }
    pub fn mark_error(&mut self, error: String) {
        self.status = Status::Error(error);
    }
}
fn expedition(o: &crate::opening015::Observation) -> Value {
    json!({"party":o.party,"heroes":o.heroes.iter().filter(|h|o.party.contains(&h.guid)).map(|h|json!({"guid":h.guid,"name":h.name,"class":h.class,"xp":h.xp,"hp":h.hp,"max_hp":h.maximum_hp,"stress":h.stress})).collect::<Vec<_>>(),"food":o.inventory.count(&Item::new("provision","")),"occupied_slots":o.inventory.slots.iter().flatten().count(),"slots":o.inventory.slots.len(),"torch":o.torch,"node":o.map.as_ref().map(|m|m.position.clone()),"visited_known":o.map.as_ref().map(|m|m.nodes.iter().filter(|n|n.visited).count()),"known_geometry":o.map.as_ref().map(|m|m.nodes.len()),"battle":o.battle.as_ref().map(|b|json!({"round":b.round,"actor":b.actor,"enemies":b.units.iter().filter(|u|!u.hero&&!u.corpse).map(|u|json!({"class":u.class,"rank":u.rank,"hp":u.hp,"max_hp":u.max_hp,"stunned":u.stunned})).collect::<Vec<_>>()})),"quest_ready":o.quest_ready,"camping":o.camping.is_some()})
}
pub fn summary(o: &View) -> Result<Value> {
    let mut value = match o {
        View::Opening(o) => {
            json!({"phase":"opening","week":0,"gold":o.wallet.get("gold:").copied().unwrap_or(0),"expedition":expedition(o),"goals":[],"opening_completed":o.results.len(),"deaths":o.heroes.iter().filter(|h|!h.alive).count()})
        }
        View::Campaign(o) => {
            json!({"phase":if o.expedition.is_some(){"expedition"}else if o.content019.as_ref().is_some_and(|v|v.farm.is_some()){"farm"}else if o.provisions.is_some(){"provisions"}else{"town"},"week":o.week,"returns":o.returns,"completions":o.completions,"deaths":o.total_deaths,"gold":o.town.wallet.get("gold:").copied().unwrap_or(0),"wallet":o.town.wallet,"party":o.town.party,"heroes":o.town.heroes.iter().filter(|h|o.town.party.contains(&h.guid)).map(|h|json!({"guid":h.guid,"name":h.name,"class":h.class,"level":h.level,"hp":h.stats.hp,"max_hp":h.stats.hp,"stress":h.stress,"weapon":h.weapon,"armour":h.armour,"busy":h.busy})).collect::<Vec<_>>(),"roster":o.town.heroes.iter().filter(|h|h.alive).count(),"roster_capacity":o.town.roster_capacity,"event":o.current_event,"goals":auto_policy018::goals(o)?,"expedition":o.expedition.as_ref().map(|e|{let mut v=expedition(e);for h in v["heroes"].as_array_mut().unwrap(){if let Some(source)=o.town.heroes.iter().find(|source|Some(source.guid)==h["guid"].as_u64()){h["level"]=json!(source.level);}}v}),"quest":o.board.as_ref().and_then(|b|b.quests.iter().find(|q|Some(&q.id)==b.selected.as_ref())).map(|q|json!({"id":q.id,"plot":q.plot,"region":q.dungeon,"kind":q.kind,"difficulty":q.difficulty,"length":q.length}))})
        }
    };
    if let View::Campaign(o) = o
        && let Some(p) = &o.steward020
    {
        value["steward"] = serde_json::to_value(p).map_err(|e| e.to_string())?;
    }
    Ok(value)
}
fn shown(v: &Value) -> String {
    match v {
        Value::Null => "—".into(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
pub fn render(v: &Value, step: u64, status: &Status, action: &str, reason: &str) -> String {
    let state = match status {
        Status::Running => "执行中",
        Status::Success => "目标完成",
        Status::Timeout => "Timeout / 周数到期",
        Status::GameOver => "原难度战役失败",
        Status::Error(_) => "执行错误",
    };
    let phase = match v["phase"].as_str() {
        Some("opening") => {
            if v["opening_completed"] == 0 {
                "老路"
            } else {
                "首次废墟"
            }
        }
        Some("town") => "城镇",
        Some("provisions") => "补给",
        Some("expedition") => "远征",
        Some("farm") => "农庄",
        _ => "—",
    };
    let mut s = format!(
        "DD-SIM 自动战役监控台    第{}周    决策 {}    {}\n{}\n",
        shown(&v["week"]),
        step,
        state,
        "=".repeat(76)
    );
    s += &format!(
        "阶段 {}   金币 {}   死亡 {}",
        phase,
        shown(&v["gold"]),
        shown(&v["deaths"])
    );
    if !v["roster"].is_null() {
        s += &format!(
            "   名册 {}/{}",
            shown(&v["roster"]),
            shown(&v["roster_capacity"])
        );
    }
    s.push('\n');
    s += &format!(
        "任务 {}\n动作 {}\n理由 {}\n",
        shown(&v["quest"]),
        action,
        reason
    );
    let e = &v["expedition"];
    if let Some(heroes) = e["heroes"].as_array().or_else(|| v["heroes"].as_array()) {
        for h in heroes {
            let progress = if !h["level"].is_null() {
                format!("等级 {}", shown(&h["level"]))
            } else {
                format!("经验 {}", shown(&h["xp"]))
            };
            s += &format!(
                "  #{} {}  HP {}/{}  压力 {}  {}\n",
                shown(&h["guid"]),
                shown(&h["class"]),
                shown(&h["hp"]),
                shown(&h["max_hp"]),
                shown(&h["stress"]),
                progress
            );
        }
    }
    if !e.is_null() {
        if !e["node"].is_null() {
            s += &format!(
                "位置 {}  已走/已知格 {}/{}  ",
                shown(&e["node"]),
                shown(&e["visited_known"]),
                shown(&e["known_geometry"])
            );
        }
        s += &format!(
            "食物 {}  背包 {}/{}  光照 {}\n",
            shown(&e["food"]),
            shown(&e["occupied_slots"]),
            shown(&e["slots"]),
            shown(&e["torch"])
        );
        if !e["battle"].is_null() {
            s += &format!("战斗 {}\n", e["battle"]);
        }
    }
    if let Some(p) = v.get("steward") {
        s += &format!(
            "管家任务 {}/{}  Boss {}/{}  职业 {}/{}  主要内容 {}/{}\n",
            shown(&p["achieved"]),
            shown(&p["required"]),
            shown(&p["categories"]["bosses"]["done"]),
            shown(&p["categories"]["bosses"]["total"]),
            shown(&p["categories"]["classes"]["done"]),
            shown(&p["categories"]["classes"]["total"]),
            shown(&p["categories"]["major_content"]["done"]),
            shown(&p["categories"]["major_content"]["total"])
        );
        if p["full_scope"] != true {
            s += "当前为部分内容配置，不判定完整管家任务通关\n";
        }
        if let Some(classes) = p["classes"].as_object().filter(|rows| !rows.is_empty()) {
            s += "尚未达成的职业（前5项；在册等级不替代返程升级成就）：\n";
            for (id, row) in classes
                .iter()
                .filter(|(_, row)| row["earned"] != true)
                .take(5)
            {
                s += &format!(
                    "  {}  普通英雄 {} 名 / 最高等级 {} / 星晶佣兵 {} 名\n",
                    crate::campaign017::goals020::label(&format!("class:{id}")),
                    shown(&row["ordinary_alive"]),
                    shown(&row["ordinary_max_level"]),
                    shown(&row["mercenaries_alive"])
                );
            }
        }
        if let Some(goals) = p["goals"].as_array() {
            s += "未完成目标（前8项，完整明细见进度记录）：\n";
            for g in goals
                .iter()
                .filter(|g| g["done"] != true && g["category"] != "classes")
                .take(8)
            {
                s += &format!(
                    "  [{}] {}\n",
                    if g["done"] == true {
                        "完成"
                    } else {
                        "待完成"
                    },
                    shown(&g["label"])
                );
            }
        }
    } else if let Some(goals) = v["goals"].as_array() {
        if goals.is_empty() {
            s += "开局结束后统计 DD / CC 目标进度\n";
        } else {
            s += "目标：\n";
            for g in goals {
                s += &format!(
                    "  [{}] {}\n",
                    if g[1] == true { "完成" } else { "待完成" },
                    shown(&g[0])
                );
            }
        }
    }
    s
}


#[path = "runner019.rs"]
pub mod content019;
#[path = "runner020.rs"]
pub mod steward020;
/// A combat-only policy receives an owned, filtered observation, never host state.
type CombatChoice021<'a> =
    dyn FnMut(&crate::Observation) -> Result<Option<(crate::Action, String)>> + 'a;
#[path = "runner021.rs"]
pub mod arena021;
#[path = "runner023.rs"]
pub mod mechanisms023;
#[path = "runner024.rs"]
pub mod preparation024;
#[path = "replay022.rs"]
pub mod replay022;
#[path = "runner025.rs"]
pub mod strategy025;

/// Intersect the combat preview with the outer host's public legal actions.
/// Farm Exit is a real legal guaranteed escape, represented here as the policy's
/// Retreat intent and translated back on execution; no combat result is invented.
fn combat_observation021(o: &Observation, action: &Action) -> Option<crate::Observation> {
    match action {
        Action::Raid(crate::opening015::Action::Fight(_)) => {
            let raid = o.expedition.as_ref()?;
            let mut b = raid.battle.clone()?;
            b.actions.retain(|a| {
                raid.actions
                    .contains(&crate::opening015::Action::Fight(a.action.clone()))
            });
            Some(b)
        }
        Action::Farm(crate::campaign017::content019::farm::Action::Fight(_)) => {
            use crate::campaign017::content019::farm::Action as FarmAction;
            let farm = o.content019.as_ref()?.farm.as_ref()?;
            let mut b = farm.battle.clone();
            let exit = farm.actions.contains(&FarmAction::Exit);
            b.actions.retain(|a| {
                farm.actions.contains(&FarmAction::Fight(a.action.clone()))
                    || (exit && a.action == crate::Action::Retreat)
            });
            if exit && !b.actions.iter().any(|a| a.action == crate::Action::Retreat) {
                b.actions.push(crate::LegalAction {
                    action: crate::Action::Retreat,
                    label: "farm_exit".into(),
                    targets: vec![],
                    hit_chance: 1.,
                    damage: [0, 0],
                    heal: [0, 0],
                    stress_heal: 0,
                });
            }
            Some(b)
        }
        _ => None,
    }
}

#[path = "runner026.rs"]
pub mod rollout026;

#[path = "human030.rs"]
pub mod human030;
#[path = "runner027.rs"]
pub mod rollout027;
