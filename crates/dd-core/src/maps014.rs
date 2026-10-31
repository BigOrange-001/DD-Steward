//! Map navigation and progress only. Encounter/curio/trap/obstacle/hunger
//! settlements are explicit trusted-host boundaries, never policy acknowledgements.
use super::Rng;
use dd_data::{
    catalog014::{Catalog, Route},
    combat::{GameData, Result},
    content::ContentConfig,
    maps014::*,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FarmEntry {
    pub profile: String,
    pub content_id: String,
    pub quest: Value,
}
impl FarmEntry {
    pub fn battle_plan(
        &self,
        g: &GameData,
        eligible: BTreeSet<String>,
    ) -> Result<dd_data::farm011::Plan> {
        if self.content_id != g.overlay.config.id()? {
            return Err("farm map entry content mismatch".into());
        }
        g.farm_plan(&self.profile, eligible)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Map(Box<Map>),
    Farm(FarmEntry),
}
fn place_random_goal(map: &mut Map, c: &Catalog, goal: &str) -> Result<()> {
    let g = dd_data::library::unique_json(&c.goal_defs, goal, "random map goal")?;
    match g["type"].as_str().ok_or("missing random goal type")? {
        "kill_monster" => {
            let anchor = map.raw["final_anchor"]
                .as_str()
                .ok_or("random map lacks final anchor")?
                .to_string();
            if map.nodes[&anchor].content.battle.is_none() {
                let other = map
                    .nodes
                    .iter()
                    .find(|(_, n)| n.room && !n.hidden && n.content.battle.is_some())
                    .map(|(id, _)| id.clone())
                    .ok_or("boss rule has no room battle slot")?;
                let a = map.nodes[&anchor].content.clone();
                let b = map.nodes[&other].content.clone();
                map.nodes.get_mut(&anchor).unwrap().content = b;
                map.nodes.get_mut(&other).unwrap().content = a;
            }
            let n = map.nodes.get_mut(&anchor).unwrap();
            n.content.battle = Some(format!("quest:{goal}"));
            n.content.quest = true;
            map.goals.insert(anchor);
        }
        "activate" | "gather" => {
            let curio = g["data"]["curio_name"]
                .as_str()
                .ok_or("random goal lacks curio")?;
            let amount_value = if g["type"].as_str() == Some("gather") {
                &g["data"]["item"]["amount"]
            } else {
                &g["data"]["amount"]
            };
            let amount = amount_value
                .as_u64()
                .filter(|v| *v > 0 && *v <= 256)
                .ok_or("invalid random goal amount")? as usize;
            let mut empty = map
                .nodes
                .iter()
                .filter(|(id, n)| {
                    n.room && !n.hidden && id.as_str() != map.entrance && n.content.is_empty()
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            let seed = map
                .generation
                .as_ref()
                .ok_or("random map lacks generation record")?
                .seed;
            let mut rng = Rng(seed);
            for i in (1..empty.len()).rev() {
                let j = rng.below((i + 1) as u64) as usize;
                empty.swap(i, j);
            }
            if empty.len() < amount {
                return Err("insufficient empty rooms for source quest goals".into());
            }
            for id in empty.into_iter().take(amount) {
                let n = map.nodes.get_mut(&id).unwrap();
                n.content.curio = Some(curio.into());
                n.content.quest = true;
                n.metadata["objective_only_curio"] = json!(true);
                map.goals.insert(id);
            }
        }
        "explore" | "explore_room" => map.goals.extend(
            map.nodes
                .iter()
                .filter(|(_, n)| n.room && !n.hidden)
                .map(|(id, _)| id.clone()),
        ),
        "kill_room" | "battle_room" => map.goals.extend(
            map.nodes
                .iter()
                .filter(|(_, n)| n.room && !n.hidden && n.content.battle.is_some())
                .map(|(id, _)| id.clone()),
        ),
        kind => return Err(format!("unsupported generated map goal {kind}")),
    }
    if !map.raw["goal_sources"].is_object() {
        map.raw["goal_sources"] = json!({});
    }
    map.raw["goal_sources"][goal] = g.clone();
    super::generate014::refresh_ledger(map)?;
    map.validate()
}
pub fn random_entry(
    c: &Catalog,
    key: &dd_data::generation014::Key,
    goals: &[String],
    seed: u64,
) -> Result<Map> {
    if goals.is_empty() {
        return Err("random quest request requires explicit source goals".into());
    }
    let alternatives = c.generated_goals(key)?;
    if key.quest != "kill_boss" && !alternatives.iter().any(|g| g == goals) {
        return Err("goals are not an enabled alternative for this generator".into());
    }
    if key.quest == "kill_boss"
        && !c.quests.values().any(|q| {
            q["quest"]["dungeon"].as_str() == Some(&key.dungeon)
                && q["quest"]["type"].as_str() == Some("kill_boss")
                && q["quest"]["length"].as_u64() == Some(if key.size == "medium" { 2 } else { 3 })
                && q["quest"]["goal_ids"] == json!(goals)
        })
    {
        return Err("boss goals do not match an original plot quest".into());
    }
    generated_entry(c, key, goals, seed)
}
fn generated_entry(
    c: &Catalog,
    key: &dd_data::generation014::Key,
    goals: &[String],
    seed: u64,
) -> Result<Map> {
    let mut map = super::generate014::generate(&c.rule(key)?, seed, &c.candidates)?;
    map.goals.clear();
    for goal in goals {
        place_random_goal(&mut map, c, goal)?;
    }
    Ok(map)
}
pub fn campaign_random_entry(
    c: &Catalog,
    key: &dd_data::generation014::Key,
    goals: &[String],
    seed: u64,
) -> Result<Map> {
    if goals.is_empty() || !c.campaign_goals(key)?.iter().any(|g| g == goals) {
        return Err("goals do not match combined original campaign requirements".into());
    }
    generated_entry(c, key, goals, seed)
}
pub fn quest_entry(c: &Catalog, id: &str, seed: u64) -> Result<Entry> {
    match c.route(id)? {
        Route::Disabled => Err("disabled test quest is not a production entry".into()),
        Route::Unresolved(e) => Err(e),
        Route::Fixed(_) => Ok(Entry::Map(Box::new(c.fixed_for_quest(id)?))),
        Route::Wave(profile) => Ok(Entry::Farm(FarmEntry {
            profile,
            content_id: c.content.id()?,
            quest: dd_data::library::unique_json(&c.quests, id, "farm quest")?.clone(),
        })),
        Route::Random(key) => {
            let quest = dd_data::library::unique_json(&c.quests, id, "random quest")?;
            let goals = serde_json::from_value::<Vec<String>>(quest["quest"]["goal_ids"].clone())
                .map_err(|e| e.to_string())?;
            let mut map = random_entry(c, &key, &goals, seed)?;
            map.raw["task"] = quest.clone();
            Ok(Entry::Map(Box::new(map)))
        }
    }
}
pub fn repeat_entry(c: &Catalog, kind: &str, seed: u64) -> Result<Map> {
    let (rule, goal) = c.repeat_rule(kind)?;
    let mut map = super::generate014::generate(&rule, seed, &c.candidates)?;
    map.goals.clear();
    place_random_goal(&mut map, c, &goal)?;
    map.raw["repeat_task"] = json!(kind);
    Ok(map)
}
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    map: Arc<Map>,
    content: ContentConfig,
    candidates: Candidates,
    id: String,
    gate_groups: BTreeMap<String, (String, BTreeSet<String>)>,
    secret_tags: BTreeSet<String>,
}
impl Definition {
    pub fn new(map: Map, content: ContentConfig, candidates: Candidates) -> Result<Self> {
        content.validate()?;
        candidates.validate()?;
        map.validate()?;
        if map.generation.is_some() {
            super::generate014::validate_generated(&map)?;
        }
        use sha2::{Digest, Sha256};
        let identity = dd_data::maps014::config_identity(&content, &candidates)?;
        let bytes = serde_json::to_vec(&(&map, &identity)).map_err(|e| e.to_string())?;
        let id = format!("{PROFILE};sha256:{:x}", Sha256::digest(bytes));
        let gate_groups = map.gate_groups()?;
        let secret_tags = map
            .nodes
            .values()
            .flat_map(|n| n.links.iter().filter_map(|l| l.hidden.clone()))
            .collect();
        Ok(Self {
            gate_groups,
            secret_tags,
            map: Arc::new(map),
            content,
            candidates,
            id,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn map(&self) -> &Map {
        &self.map
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    Encounter,
    Trap,
    Obstacle,
    Hunger,
    Curio,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub definition_id: String,
    pub node: String,
    pub kind: EventKind,
    pub serial: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Move(String),
    Interact,
    Unlock(String),
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleContent {
    pub battle: bool,
    pub curio: bool,
    pub curio_name: Option<String>,
    pub trap: bool,
    pub obstacle: bool,
    pub hunger: bool,
    pub objective: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleNode {
    pub id: String,
    pub room: bool,
    pub position: [f64; 2],
    pub visited: bool,
    pub content: Option<VisibleContent>,
    pub exits: Vec<String>,
    pub gate_key: Option<String>,
    pub opened: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Observation {
    pub position: String,
    pub nodes: Vec<VisibleNode>,
    pub keys: BTreeMap<String, u32>,
    pub pending: Option<EventKind>,
    pub completed: bool,
    pub actions: Vec<Action>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Progress {
    profile: String,
    definition_id: String,
    candidate_id: String,
    position: String,
    visited: BTreeSet<String>,
    scouted: BTreeSet<String>,
    revealed: BTreeSet<String>,
    opened: BTreeSet<String>,
    keys: BTreeMap<String, u32>,
    settled: BTreeSet<(String, EventKind)>,
    pending: Option<Pending>,
    serial: u64,
    rng: Rng,
    completed: bool,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    concealed: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    layout: BTreeSet<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    definition: Definition,
    state: Progress,
}
#[derive(Debug, Clone, Copy)]
pub enum Resolution {
    Settled,
    Teleport,
    CancelCurio,
}
fn has(n: &Node, k: EventKind) -> bool {
    match k {
        EventKind::Encounter => n.content.battle.is_some(),
        EventKind::Trap => n.content.trap.is_some(),
        EventKind::Obstacle => n.content.obstacle.is_some(),
        EventKind::Hunger => n.content.hunger,
        EventKind::Curio => n.content.curio.is_some(),
    }
}
impl Session {
    pub fn new(definition: Definition, seed: u64, keys: BTreeMap<String, u32>) -> Result<Self> {
        let position = definition.map.entrance.clone();
        let mut s = Self {
            state: Progress {
                profile: PROFILE.into(),
                definition_id: definition.id.clone(),
                candidate_id: dd_data::maps014::config_identity(
                    &definition.content,
                    &definition.candidates,
                )?,
                position: position.clone(),
                visited: BTreeSet::from([position]),
                scouted: BTreeSet::new(),
                revealed: BTreeSet::new(),
                opened: BTreeSet::new(),
                keys,
                settled: BTreeSet::new(),
                pending: None,
                serial: 0,
                rng: Rng(seed),
                completed: false,
                concealed: BTreeSet::new(),
                layout: BTreeSet::new(),
            },
            definition,
        };
        for n in s.definition.map.nodes.values() {
            if n.metadata["static"]["hd_always_accessible"].as_bool() == Some(true) {
                for l in &n.links {
                    if let Some(tag) = &l.hidden {
                        s.state.revealed.insert(tag.clone());
                    }
                }
            }
        }
        s.enter()?;
        s.validate()?;
        Ok(s)
    }
    pub fn definitions(&self) -> Definition {
        self.definition.clone()
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(&self.state).map_err(|e| e.to_string())
    }
    pub fn restore(json: &str, definition: &Definition) -> Result<Self> {
        if json.len() > 16 * 1024 * 1024 {
            return Err("map progress snapshot exceeds size bound".into());
        }
        let state: Progress = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if state.profile != PROFILE
            || state.definition_id != definition.id
            || state.candidate_id
                != dd_data::maps014::config_identity(&definition.content, &definition.candidates)?
        {
            return Err("map snapshot profile/content/candidate/source mismatch".into());
        }
        let s = Self {
            definition: definition.clone(),
            state,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn resume_campaign_map(json: &str, definition: &Definition) -> Result<Self> {
        if !definition.map.persistent {
            return Err("map does not retain campaign progress".into());
        }
        let s = Self::restore(json, definition)?;
        if s.state.completed {
            return Err("completed map cannot resume as an active expedition".into());
        }
        Ok(s)
    }
    /// Host reveal of geometry only; no contents or hidden links are scouted.
    pub(crate) fn reveal_layout(&mut self, ids: &BTreeSet<String>) -> Result<()> {
        self.transaction(|s| {
            if ids
                .iter()
                .any(|id| s.definition.map.nodes.get(id).is_none_or(|n| n.hidden))
            {
                return Err("layout reveal includes unknown/hidden node".into());
            }
            s.state.layout.extend(ids.iter().cloned());
            Ok(())
        })
    }
    pub fn position(&self) -> &str {
        &self.state.position
    }
    /// Trusted finite-inventory owner replaces derived key availability. Standalone
    /// map awards remain unchanged; campaign callers must reconcile actual bag items.
    pub(crate) fn replace_host_keys(&mut self, keys: BTreeMap<String, u32>) -> Result<()> {
        self.transaction(|s| {
            s.state.keys = keys;
            Ok(())
        })
    }
    pub fn keys(&self) -> &BTreeMap<String, u32> {
        &self.state.keys
    }
    pub fn legal_actions(&self) -> Vec<Action> {
        self.actions()
    }
    pub fn opened(&self) -> &BTreeSet<String> {
        &self.state.opened
    }
    pub fn revealed(&self) -> &BTreeSet<String> {
        &self.state.revealed
    }
    pub fn visited(&self) -> &BTreeSet<String> {
        &self.state.visited
    }
    pub fn pending(&self) -> Option<&Pending> {
        self.state.pending.as_ref()
    }
    fn validate(&self) -> Result<()> {
        let s = &self.state;
        let m = &self.definition.map;
        if s.layout
            .iter()
            .any(|id| m.nodes.get(id).is_none_or(|n| n.hidden))
        {
            return Err("invalid map layout memory".into());
        }
        if s.profile != PROFILE
            || s.definition_id != self.definition.id
            || !m.nodes.contains_key(&s.position)
            || !s.visited.contains(&s.position)
            || s.serial > 1000000
            || s.keys.len() > 128
            || s.keys.values().any(|v| *v == 0 || *v > 1000000)
        {
            return Err("invalid map progress".into());
        }
        if s.concealed.contains(&s.position)
            || s.concealed
                .iter()
                .any(|id| !m.nodes.contains_key(id) || s.scouted.contains(id))
        {
            return Err("invalid concealed map content".into());
        }
        for id in s.visited.iter().chain(&s.scouted) {
            if !m.nodes.contains_key(id) {
                return Err("unknown discovered map node".into());
            }
        }
        for id in &s.opened {
            if !m.nodes.get(id).is_some_and(|n| n.gate.is_some()) {
                return Err("opened node is not a map gate".into());
            }
        }
        for (_, nodes) in self.definition.gate_groups.values() {
            if nodes.iter().any(|id| s.opened.contains(id)) && !nodes.is_subset(&s.opened) {
                return Err("partially opened map lock group".into());
            }
        }
        if !s.revealed.is_subset(&self.definition.secret_tags) {
            return Err("unknown revealed hidden door".into());
        }
        for id in &s.scouted {
            let n = &m.nodes[id];
            if n.hidden && !n.tags.iter().any(|t| s.revealed.contains(t)) {
                return Err("scouted hidden room lacks a discovered doorway".into());
            }
        }
        for key in s.keys.keys() {
            if !m.nodes.values().any(|n| n.gate.as_ref() == Some(key))
                && !m.key_sources.values().any(|a| &a.item == key)
            {
                return Err("unknown map key".into());
            }
        }
        for (id, k) in &s.settled {
            if !s.visited.contains(id) || !m.nodes.get(id).is_some_and(|n| has(n, *k)) {
                return Err("invalid settled map event".into());
            }
        }
        let forced = [
            EventKind::Encounter,
            EventKind::Trap,
            EventKind::Obstacle,
            EventKind::Hunger,
        ]
        .into_iter()
        .find(|kind| {
            has(&m.nodes[&s.position], *kind) && !s.settled.contains(&(s.position.clone(), *kind))
        });
        if match &s.pending {
            Some(p) if p.kind == EventKind::Curio => forced.is_some(),
            Some(p) => forced != Some(p.kind),
            None => forced.is_some(),
        } {
            return Err("map progress is not at a settled host/player boundary".into());
        }
        if let Some(p) = &s.pending
            && (p.definition_id != self.definition.id
                || p.serial != s.serial
                || p.node != s.position
                || !has(&m.nodes[&p.node], p.kind)
                || s.settled.contains(&(p.node.clone(), p.kind))
                || s.completed)
        {
            return Err("invalid pending map event".into());
        }
        if m.nodes[&s.position].gate.is_some() && !s.opened.contains(&s.position) {
            return Err("map position is inside a closed gate".into());
        }
        Ok(())
    }
    fn transaction<T>(&mut self, run: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let mut next = self.clone();
        let value = run(&mut next)?;
        next.validate()?;
        *self = next;
        Ok(value)
    }
    fn enter(&mut self) -> Result<()> {
        if self.state.pending.is_some() {
            return Err("unsettled map boundary".into());
        }
        self.state.concealed.remove(&self.state.position);
        self.state.visited.insert(self.state.position.clone());
        let n = &self.definition.map.nodes[&self.state.position];
        for kind in [
            EventKind::Encounter,
            EventKind::Trap,
            EventKind::Obstacle,
            EventKind::Hunger,
        ] {
            if has(n, kind) && !self.state.settled.contains(&(n.id.clone(), kind)) {
                self.set_pending(kind)?;
                break;
            }
        }
        Ok(())
    }
    fn set_pending(&mut self, kind: EventKind) -> Result<()> {
        self.state.serial = self
            .state
            .serial
            .checked_add(1)
            .ok_or("map serial overflow")?;
        self.state.pending = Some(Pending {
            definition_id: self.definition.id.clone(),
            node: self.state.position.clone(),
            kind,
            serial: self.state.serial,
        });
        Ok(())
    }
    fn known_layout(&self, id: &str) -> bool {
        let m = &self.definition.map;
        let n = &m.nodes[id];
        let discovered = self.state.visited.contains(id)
            || self.state.scouted.contains(id)
            || self.state.revealed.iter().any(|tag| n.tags.contains(tag));
        if n.hidden {
            return discovered;
        }
        !m.fog
            || self.state.layout.contains(id)
            || discovered
            || m.nodes[&self.state.position].links.iter().any(|l| {
                l.to == id
                    && l.hidden
                        .as_ref()
                        .is_none_or(|t| self.state.revealed.contains(t))
            })
    }
    fn actions(&self) -> Vec<Action> {
        if self.state.completed || self.state.pending.is_some() {
            return vec![];
        }
        let m = &self.definition.map;
        let n = &m.nodes[&self.state.position];
        let mut actions = vec![];
        for l in &n.links {
            if l.hidden
                .as_ref()
                .is_some_and(|t| !self.state.revealed.contains(t))
            {
                continue;
            }
            let to = &m.nodes[&l.to];
            if let Some(key) = &to.gate
                && !self.state.opened.contains(&to.id)
            {
                if self.state.keys.get(key).is_some_and(|v| *v > 0) {
                    actions.push(Action::Unlock(to.id.clone()));
                }
                continue;
            }
            actions.push(Action::Move(l.to.clone()));
        }
        if has(n, EventKind::Curio)
            && !self
                .state
                .settled
                .contains(&(n.id.clone(), EventKind::Curio))
        {
            actions.push(Action::Interact);
        }
        actions
    }
    pub fn observe(&self) -> Observation {
        let m = &self.definition.map;
        let mut nodes = vec![];
        for (id, n) in &m.nodes {
            if !self.known_layout(id) {
                continue;
            }
            let known = (self.state.visited.contains(id) || self.state.scouted.contains(id))
                && !self.state.concealed.contains(id);
            let content = known.then(|| VisibleContent {
                battle: has(n, EventKind::Encounter)
                    && !self
                        .state
                        .settled
                        .contains(&(id.clone(), EventKind::Encounter)),
                curio: has(n, EventKind::Curio)
                    && !self.state.settled.contains(&(id.clone(), EventKind::Curio)),
                curio_name: if self.state.visited.contains(id) {
                    n.content
                        .curio
                        .as_ref()
                        .filter(|v| !v.starts_with("pool:") && !v.starts_with("hash:"))
                        .cloned()
                } else {
                    None
                },
                trap: has(n, EventKind::Trap)
                    && !self.state.settled.contains(&(id.clone(), EventKind::Trap)),
                obstacle: has(n, EventKind::Obstacle)
                    && !self
                        .state
                        .settled
                        .contains(&(id.clone(), EventKind::Obstacle)),
                hunger: has(n, EventKind::Hunger)
                    && !self
                        .state
                        .settled
                        .contains(&(id.clone(), EventKind::Hunger)),
                objective: m.goals.contains(id),
            });
            let exits = n
                .links
                .iter()
                .filter(|l| {
                    self.known_layout(&l.to)
                        && l.hidden
                            .as_ref()
                            .is_none_or(|t| self.state.revealed.contains(t))
                })
                .map(|l| l.to.clone())
                .collect();
            nodes.push(VisibleNode {
                id: id.clone(),
                room: n.room,
                position: n.position,
                visited: self.state.visited.contains(id),
                content,
                exits,
                gate_key: if self.state.visited.contains(id)
                    || m.nodes[&self.state.position]
                        .links
                        .iter()
                        .any(|l| l.to == *id)
                {
                    n.gate.clone()
                } else {
                    None
                },
                opened: self.state.opened.contains(id),
            });
        }
        Observation {
            position: self.state.position.clone(),
            nodes,
            keys: self.state.keys.clone(),
            pending: self.state.pending.as_ref().map(|p| p.kind),
            completed: self.state.completed,
            actions: self.actions(),
        }
    }
    pub fn act(&mut self, action: Action) -> Result<()> {
        self.transaction(|s| {
            if !s.actions().contains(&action) {
                return Err("illegal map action".into());
            }
            match action {
                Action::Move(to) => {
                    s.state.position = to;
                    s.enter()?;
                }
                Action::Interact => s.set_pending(EventKind::Curio)?,
                Action::Unlock(id) => {
                    let key = s.definition.map.nodes[&id]
                        .gate
                        .as_ref()
                        .ok_or("not a gate")?;
                    let n = s.state.keys.get_mut(key).ok_or("missing map key")?;
                    *n -= 1;
                    if *n == 0 {
                        s.state.keys.remove(key);
                    }
                    let group = s
                        .definition
                        .gate_groups
                        .values()
                        .find(|&(_, nodes)| nodes.contains(&id))
                        .cloned()
                        .ok_or("missing gate group")?;
                    s.state.opened.extend(group.1);
                }
            }
            Ok(())
        })
    }
    /// Host-only result of actual scouting. A policy cannot call this as an action.
    pub fn scout(
        &mut self,
        nodes: &BTreeSet<String>,
        secret_tags: &BTreeSet<String>,
    ) -> Result<()> {
        self.transaction(|s| {
            if s.state.completed
                || s.state.pending.is_some()
                || s.definition.map.raw["task"]["is_scouting_enabled"].as_bool() == Some(false)
            {
                return Err("scouting is disabled for this map".into());
            }
            if nodes
                .iter()
                .any(|id| !s.definition.map.nodes.contains_key(id))
            {
                return Err("unknown scouting node".into());
            }
            let available = s
                .definition
                .map
                .nodes
                .values()
                .flat_map(|n| n.links.iter().filter_map(|l| l.hidden.clone()))
                .collect::<BTreeSet<_>>();
            if !secret_tags.is_subset(&available) {
                return Err("unknown scouting secret tag".into());
            }
            for id in nodes {
                let n = &s.definition.map.nodes[id];
                if n.hidden
                    && !n
                        .tags
                        .iter()
                        .any(|t| s.state.revealed.contains(t) || secret_tags.contains(t))
                {
                    return Err("hidden room needs a revealed secret-door scouting result".into());
                }
            }
            for id in nodes {
                s.state.concealed.remove(id);
            }
            s.state.scouted.extend(nodes.clone());
            s.state.revealed.extend(secret_tags.clone());
            Ok(())
        })
    }
    fn teleport_inner(&mut self, tag: &str) -> Result<()> {
        let targets = self
            .definition
            .map
            .teleport_targets
            .get(tag)
            .ok_or("unknown map teleport group")?
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return Err("empty map teleport group".into());
        }
        let n = self.state.rng.below(targets.len() as u64) as usize;
        self.state.position = targets[n].clone();
        self.enter()
    }
    /// Host-only battle teleport event, matched to original map destination tags.
    pub fn teleport_from_encounter(&mut self, receipt: &Pending, tag: &str) -> Result<()> {
        self.transaction(|s| {
            if s.state.pending.as_ref() != Some(receipt) || receipt.kind != EventKind::Encounter {
                return Err("battle teleport requires the matching encounter receipt".into());
            }
            s.state.pending = None;
            s.teleport_inner(tag)
        })
    }
    /// Expedition combat retreat. The original encounter remains unresolved.
    pub(crate) fn rearm_hunger(&mut self, id: &str) -> Result<()> {
        if self.state.pending.is_some()
            || id == self.state.position
            || !self
                .definition
                .map
                .nodes
                .get(id)
                .is_some_and(|n| n.content.hunger)
        {
            return Err("invalid hunger rearm boundary".into());
        }
        self.state.settled.remove(&(id.into(), EventKind::Hunger));
        self.validate()
    }
    pub(crate) fn remaining(&self, id: &str, kind: EventKind) -> bool {
        self.definition
            .map
            .nodes
            .get(id)
            .is_some_and(|n| has(n, kind))
            && !self.state.settled.contains(&(id.into(), kind))
    }
    /// Trusted expedition content instantiation. Geometry and source identity remain fixed.
    pub(crate) fn replace_contents(
        &mut self,
        values: &BTreeMap<String, dd_data::maps014::Content>,
        epoch: u64,
    ) -> Result<()> {
        if self.state.pending.is_some() || self.state.completed {
            return Err("content replacement requires quiet active map".into());
        }
        let mut map = (*self.definition.map).clone();
        for (id, content) in values {
            let n = map.nodes.get_mut(id).ok_or("unknown replacement node")?;
            if n.room || n.hidden || n.gate.is_some() {
                return Err("runtime corridor content cannot replace special nodes".into());
            }
            n.content = content.clone();
        }
        map.raw["runtime015"] = serde_json::json!({"epoch":epoch});
        let definition = Definition::new(
            map,
            self.definition.content.clone(),
            self.definition.candidates.clone(),
        )?;
        let mut next = self.clone();
        next.state.definition_id = definition.id.clone();
        next.definition = definition;
        for id in values.keys() {
            next.state.settled.retain(|(node, _)| node != id);
            next.state.scouted.remove(id);
            if id != &next.state.position {
                next.state.concealed.insert(id.clone());
            }
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
    fn retreat_reachable(&self, from: &str, to: &str) -> bool {
        if !self.definition.map.nodes.contains_key(to) {
            return false;
        }
        let area = &self.definition.map.nodes[from].area;
        let mut q = std::collections::VecDeque::from([from.to_string()]);
        let mut seen = BTreeSet::new();
        while let Some(id) = q.pop_front() {
            if id == to {
                return true;
            }
            if !seen.insert(id.clone()) {
                continue;
            }
            let n = &self.definition.map.nodes[&id];
            if id != from && n.room {
                continue;
            }
            for l in &n.links {
                if self.state.visited.contains(&l.to)
                    && l.hidden
                        .as_ref()
                        .is_none_or(|tag| self.state.revealed.contains(tag))
                    && (self.definition.map.nodes[&l.to].room
                        || self.definition.map.nodes[&l.to].area == *area)
                {
                    q.push_back(l.to.clone());
                }
            }
        }
        false
    }
    pub fn retreat_from_encounter(&mut self, receipt: &Pending, to: &str) -> Result<()> {
        self.transaction(|s| {
            if s.state.pending.as_ref() != Some(receipt)
                || receipt.kind != EventKind::Encounter
                || !s.state.visited.contains(to)
                || !s.retreat_reachable(&receipt.node, to)
                || s.definition.map.nodes[to].gate.is_some() && !s.state.opened.contains(to)
            {
                return Err("invalid encounter retreat transition".into());
            }
            s.state.pending = None;
            s.state.position = to.into();
            s.enter()
        })
    }
    pub fn teleport(&mut self, tag: &str) -> Result<()> {
        self.transaction(|s| {
            if s.state.pending.is_some() || s.state.completed {
                return Err("teleport requires settled active map".into());
            }
            s.teleport_inner(tag)
        })
    }
    pub fn resume(&mut self, receipt: &Pending, result: Resolution) -> Result<()> {
        self.transaction(|s| {
            if s.state.pending.as_ref() != Some(receipt) {
                return Err("stale/mismatched map host receipt".into());
            }
            match result {
                Resolution::CancelCurio => {
                    if receipt.kind != EventKind::Curio {
                        return Err("only optional curio interaction can be cancelled".into());
                    }
                    s.state.pending = None;
                }
                Resolution::Teleport => {
                    if receipt.kind != EventKind::Obstacle {
                        return Err("only teleport obstacles accept this resolution".into());
                    }
                    let tag = s.definition.map.nodes[&receipt.node]
                        .teleport
                        .clone()
                        .ok_or("obstacle has no teleport group")?;
                    s.state.pending = None;
                    s.teleport_inner(&tag)?;
                }
                Resolution::Settled => {
                    s.state.settled.insert((receipt.node.clone(), receipt.kind));
                    if receipt.kind == EventKind::Curio
                        && let Some(award) = s.definition.map.key_sources.get(&receipt.node)
                    {
                        let n = s.state.keys.entry(award.item.clone()).or_default();
                        *n = n
                            .checked_add(award.amount)
                            .ok_or("map key count overflow")?;
                    }
                    s.state.pending = None;
                    s.enter()?;
                }
            }
            Ok(())
        })
    }
    /// Host mission completion, not a map-layer claim that quest rewards/XP ran.
    pub fn mark_complete(&mut self) -> Result<()> {
        self.transaction(|s| {
            if s.state.pending.is_some() {
                return Err("cannot complete unsettled map".into());
            }
            s.state.completed = true;
            Ok(())
        })
    }
}
