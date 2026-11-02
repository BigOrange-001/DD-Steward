//! Versioned map definitions. The host owns definitions and content references;
//! policy observations are built separately by dd-core.
use crate::{combat::Result, content::ContentConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "candidate-map-014";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidates {
    pub csv_curio_choices: BTreeMap<String, String>,
    pub quest_placeholders: BTreeSet<String>,
    pub version: String,
    pub extended_generation_records: bool,
    pub court_task_aliases: bool,
    pub algorithm: Option<String>,
    pub court_key_loot: BTreeMap<String, String>,
}
impl Candidates {
    pub fn strict() -> Self {
        Self {
            csv_curio_choices: BTreeMap::new(),
            quest_placeholders: BTreeSet::new(),
            version: "map-candidates-014-strict".into(),
            extended_generation_records: false,
            court_task_aliases: false,
            algorithm: None,
            court_key_loot: BTreeMap::new(),
        }
    }
    pub fn documented() -> Result<Self> {
        serde_json::from_str(r#"{
  "version": "map-candidates-014-v1",
  "extended_generation_records": true,
  "court_task_aliases": true,
  "algorithm": "path-growth-budget-v1",
  "court_key_loot": {
    "redkey_heirloom_chest": "KEY_1",
    "yellowkey_heirloom_chest": "KEY_2",
    "greenkey_heirloom_chest": "KEY_3",
    "bluekey_heirloom_chest": "KEY_4"
  },
  "csv_curio_choices": {
    "cc_map_countess@36,107": "redkey_heirloom_chest"
  },
  "quest_placeholders": [
    "DD_map2",
    "DD_map3"
  ]
}
"#)
            .map_err(|e| e.to_string())
    }
    pub fn for_content(content: &ContentConfig) -> Result<Self> {
        let mut c = Self::documented()?;
        if !content.packages.iter().any(|p| p == "crimson_court") {
            c.court_task_aliases = false;
            c.court_key_loot.clear();
            c.csv_curio_choices.clear();
        }
        if !content
            .packages
            .iter()
            .any(|p| ["crimson_court", "color_of_madness"].contains(&p.as_str()))
        {
            c.extended_generation_records = false;
        }
        Ok(c)
    }
    pub fn validate(&self) -> Result<()> {
        let documented = Self::documented()?;
        if self.version != documented.version && self.version != "map-candidates-014-strict" {
            return Err("unknown map candidate version".into());
        }
        if self
            .algorithm
            .as_deref()
            .is_some_and(|a| a != "path-growth-budget-v1")
            || self
                .court_key_loot
                .iter()
                .any(|(k, v)| documented.court_key_loot.get(k) != Some(v))
            || self
                .csv_curio_choices
                .iter()
                .any(|(k, v)| documented.csv_curio_choices.get(k) != Some(v))
            || !self
                .quest_placeholders
                .is_subset(&documented.quest_placeholders)
        {
            return Err("unknown map algorithm/key candidate".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: String,
    pub layers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub code: Option<i64>,
    pub battle: Option<String>,
    pub curio: Option<String>,
    pub trap: Option<String>,
    pub obstacle: Option<String>,
    pub hunger: bool,
    pub quest: bool,
}
impl Content {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
            || self
                == &Self {
                    code: Some(0),
                    ..Self::default()
                }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub to: String,
    pub hidden: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub area: String,
    pub room: bool,
    pub position: [f64; 2],
    pub hidden: bool,
    pub tags: BTreeSet<String>,
    pub content: Content,
    pub gate: Option<String>,
    pub teleport: Option<String>,
    pub links: Vec<Link>,
    pub metadata: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KeyAward {
    pub item: String,
    pub amount: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Map {
    pub id: String,
    pub region: String,
    pub source: Source,
    pub raw: Value,
    pub nodes: BTreeMap<String, Node>,
    pub entrance: String,
    pub goals: BTreeSet<String>,
    pub fog: bool,
    pub persistent: bool,
    pub key_sources: BTreeMap<String, KeyAward>,
    pub teleport_targets: BTreeMap<String, BTreeSet<String>>,
    pub generation: Option<crate::generation014::GenerationRecord>,
}
impl Map {
    pub fn link(&mut self, a: &str, b: &str, hidden: Option<String>) -> Result<()> {
        if a == b || !self.nodes.contains_key(a) || !self.nodes.contains_key(b) {
            return Err("invalid/dangling map edge".into());
        }
        for (x, y) in [(a, b), (b, a)] {
            let links = &mut self.nodes.get_mut(x).unwrap().links;
            let l = Link {
                to: y.into(),
                hidden: hidden.clone(),
            };
            if !links.contains(&l) {
                links.push(l);
                links.sort_by(|a, b| (&a.to, &a.hidden).cmp(&(&b.to, &b.hidden)));
            }
        }
        Ok(())
    }
    pub fn distances(&self, start: &str, open_hidden: bool) -> BTreeMap<String, usize> {
        let mut distances = BTreeMap::from([(start.to_string(), 0)]);
        let mut queue = std::collections::VecDeque::from([start.to_string()]);
        while let Some(id) = queue.pop_front() {
            let Some(n) = self.nodes.get(&id) else {
                continue;
            };
            for l in &n.links {
                if !open_hidden && l.hidden.is_some() {
                    continue;
                }
                if !distances.contains_key(&l.to) {
                    distances.insert(l.to.clone(), distances[&id] + 1);
                    queue.push_back(l.to.clone());
                }
            }
        }
        distances
    }
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || self.nodes.is_empty()
            || self.nodes.len() > 65536
            || !self.nodes.contains_key(&self.entrance)
            || !self.nodes[&self.entrance].room
        {
            return Err("invalid map identity/entrance/size".into());
        }
        for (id, n) in &self.nodes {
            if id != &n.id || n.position.iter().any(|v| !v.is_finite()) || n.links.len() > 16 {
                return Err("invalid map node".into());
            }
            let mut seen = BTreeSet::new();
            for l in &n.links {
                if !seen.insert((&l.to, &l.hidden)) {
                    return Err("duplicate map edge".into());
                }
                let other = self.nodes.get(&l.to).ok_or("dangling map edge")?;
                if !other.links.contains(&Link {
                    to: id.clone(),
                    hidden: l.hidden.clone(),
                }) {
                    return Err("one-way map edge".into());
                }
            }
        }
        if self.distances(&self.entrance, true).len() != self.nodes.len() {
            return Err("disconnected map".into());
        }
        if self.goals.iter().any(|id| !self.nodes.contains_key(id)) {
            return Err("invalid map goal".into());
        }
        for nodes in self.teleport_targets.values() {
            if nodes.is_empty()
                || nodes
                    .iter()
                    .any(|id| !self.nodes.get(id).is_some_and(|n| n.room))
            {
                return Err("invalid map teleport destination".into());
            }
        }
        self.key_reachability()?;
        Ok(())
    }
    /// Consumable keys: search map/key/opened-gate states. No free key reuse across
    /// two doors. Metadata-only movement treats host-settled content as traversable.
    pub fn gate_groups(&self) -> Result<BTreeMap<String, (String, BTreeSet<String>)>> {
        let mut groups: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
        for (id, n) in &self.nodes {
            if let Some(key) = &n.gate {
                let room = n.metadata["gate_room"].as_str().unwrap_or(id).to_string();
                let (old, nodes) = groups
                    .entry(room)
                    .or_insert_with(|| (key.clone(), BTreeSet::new()));
                if old != key {
                    return Err("one lock group has conflicting key requirements".into());
                }
                nodes.insert(id.clone());
            }
        }
        Ok(groups)
    }
    pub fn key_reachability(&self) -> Result<()> {
        self.lock_plan().map(|_| ())
    }
    pub fn lock_plan(&self) -> Result<Vec<String>> {
        let gates = self.gate_groups()?.into_values().collect::<Vec<_>>();
        if gates.is_empty() {
            return Ok(vec![]);
        }
        if gates.len() > 16 || self.key_sources.len() > 32 {
            return Err("map lock-search bound exceeded".into());
        }
        let sources = self.key_sources.keys().cloned().collect::<Vec<_>>();
        let mut queue = std::collections::VecDeque::from([(
            0u32,
            0u64,
            BTreeMap::<String, u32>::new(),
            vec![],
        )]);
        let mut seen = BTreeSet::new();
        while let Some((opened, mut collected, mut keys, plan)) = queue.pop_front() {
            if !seen.insert((opened, collected, keys.clone())) {
                continue;
            }
            if seen.len() > 100000 {
                return Err("map lock-search resource bound".into());
            }
            let mut reached = BTreeSet::from([self.entrance.clone()]);
            let mut todo = vec![self.entrance.clone()];
            while let Some(id) = todo.pop() {
                for l in &self.nodes[&id].links {
                    if gates
                        .iter()
                        .enumerate()
                        .any(|(i, (_, nodes))| nodes.contains(&l.to) && opened & (1 << i) == 0)
                    {
                        continue;
                    }
                    if reached.insert(l.to.clone()) {
                        todo.push(l.to.clone());
                    }
                }
            }
            if self.goals.is_subset(&reached) {
                return Ok(plan);
            }
            for (i, id) in sources.iter().enumerate() {
                if reached.contains(id) && collected & (1 << i) == 0 {
                    let a = &self.key_sources[id];
                    let n = keys.entry(a.item.clone()).or_default();
                    *n = n.checked_add(a.amount).ok_or("map key overflow")?;
                    collected |= 1 << i;
                }
            }
            for (i, (key, nodes)) in gates.iter().enumerate() {
                if opened & (1 << i) != 0 || keys.get(key).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let Some(id) = nodes.iter().find(|id| {
                    self.nodes[*id]
                        .links
                        .iter()
                        .any(|l| reached.contains(&l.to))
                }) else {
                    continue;
                };
                let mut next = keys.clone();
                *next.get_mut(key).unwrap() -= 1;
                let mut next_plan = plan.clone();
                next_plan.push(id.clone());
                queue.push_back((opened | (1 << i), collected, next, next_plan));
            }
        }
        Err("quest goal is unreachable with source keys".into())
    }
}
pub fn source_path(path: &str) -> bool {
    (path.starts_with("maps/") && (path.ends_with(".dm") || path.ends_with(".csv")))
        || path == "scripts/map_generator.darkest"
        || path == "scripts/starting_save/persist.map.json"
        || (path.starts_with("campaign/quest/") && path.ends_with(".json"))
        || (path.starts_with("props/") && path.ends_with("_definitions.json"))
        || (path.starts_with("loot/") && path.ends_with(".json"))
        || (path.starts_with("curios/") && path.ends_with(".csv"))
        || (path.starts_with("inventory/")
            && (path.ends_with(".quest.inventory.items.darkest")
                || path.ends_with(".quest_item.inventory.items.darkest")))
        || (path.starts_with("dungeons/") && path.ends_with(".dungeon.json"))
}
pub fn known(v: &Value, keys: &[&str]) -> Result<()> {
    for k in v.as_object().ok_or("expected map data object")?.keys() {
        if !keys.contains(&k.as_str()) {
            return Err(format!("unknown map field {k}"));
        }
    }
    Ok(())
}
pub fn integer(v: &Value) -> Result<i64> {
    v.as_i64().ok_or("invalid map integer".into())
}
pub fn area_id(v: &Value) -> Result<String> {
    if let Some(s) = v.as_str() {
        return Ok(s.into());
    }
    let n = integer(v)?;
    if !(0..=u32::MAX as i64).contains(&n) {
        return Err("invalid encoded area id".into());
    }
    String::from_utf8((n as u32).to_le_bytes().to_vec()).map_err(|e| e.to_string())
}
pub fn config_identity(content: &ContentConfig, candidates: &Candidates) -> Result<String> {
    content.validate()?;
    candidates.validate()?;
    Ok(format!(
        "{};map={}",
        content.id()?,
        serde_json::to_string(candidates).map_err(|e| e.to_string())?
    ))
}
