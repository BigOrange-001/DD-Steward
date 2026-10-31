//! Enabled map inventory, logical-ID merging and explicit task selection.
use crate::{
    combat::{Result, text},
    generation014::{Key, Rule},
    library,
    maps014::*,
    overlay::Overlay,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Template {
    pub id: String,
    pub source: Source,
    pub bytes: Vec<u8>,
    pub enabled_references: BTreeSet<String>,
    pub disabled_references: BTreeSet<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Route {
    Fixed(String),
    Random(Key),
    Wave(String),
    Disabled,
    Unresolved(String),
}
#[derive(Debug, Clone)]
pub struct Catalog {
    pub content: crate::content::ContentConfig,
    pub candidates: Candidates,
    pub templates: BTreeMap<String, Template>,
    pub rules: BTreeMap<Key, Vec<Rule>>,
    pub quests: BTreeMap<String, Value>,
    pub goal_defs: BTreeMap<String, Value>,
    pub goal_groups: BTreeMap<String, Value>,
    pub props: BTreeMap<u32, String>,
    pub gates: BTreeMap<String, String>,
    pub key_awards: BTreeMap<String, KeyAward>,
    pub named_mashes: BTreeMap<String, Vec<Vec<String>>>,
}
fn source(o: &Overlay, path: &str) -> Source {
    Source {
        path: path.into(),
        layers: o
            .sources(path)
            .iter()
            .map(|i| o.layers[*i].id.clone())
            .collect(),
    }
}
fn csv_props(o: &Overlay) -> Result<BTreeMap<u32, String>> {
    let mut refs = BTreeMap::new();
    for path in o
        .paths
        .keys()
        .filter(|p| p.starts_with("curios/") && p.ends_with("curio_props.csv"))
    {
        for b in o.read_map_versions(path)? {
            for row in csv::ReaderBuilder::new()
                .flexible(true)
                .from_reader(b.as_slice())
                .records()
            {
                let row = row.map_err(|e| e.to_string())?;
                let id = row.get(0).unwrap_or("").trim();
                if id.is_empty() {
                    continue;
                }
                let h = crate::dson013::hash(id);
                if refs.insert(h, id.to_string()).is_some_and(|v| v != id) {
                    return Err(format!("map curio hash collision {h}"));
                }
            }
        }
    }
    Ok(refs)
}
impl Catalog {
    pub fn load(o: &Overlay, candidates: Candidates) -> Result<Self> {
        candidates.validate()?;
        let court = o.config.packages.iter().any(|p| p == "crimson_court");
        if (!candidates.court_key_loot.is_empty() || candidates.court_task_aliases) && !court {
            return Err("court map candidates require enabled crimson_court".into());
        }
        let mut c = Self {
            content: o.config.clone(),
            candidates,
            templates: BTreeMap::new(),
            rules: BTreeMap::new(),
            quests: BTreeMap::new(),
            goal_defs: BTreeMap::new(),
            goal_groups: BTreeMap::new(),
            props: csv_props(o)?,
            gates: BTreeMap::new(),
            key_awards: BTreeMap::new(),
            named_mashes: BTreeMap::new(),
        };
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("maps/") && (p.ends_with(".dm") || p.ends_with(".csv")))
        {
            let id = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("invalid map filename")?
                .to_string();
            if c.templates.contains_key(&id) {
                return Err(format!("ambiguous fixed map id {id}"));
            }
            c.templates.insert(
                id.clone(),
                Template {
                    id,
                    source: source(o, path),
                    bytes: o.read_map(path)?,
                    enabled_references: BTreeSet::new(),
                    disabled_references: BTreeSet::new(),
                },
            );
        }
        if o.paths
            .contains_key("scripts/starting_save/persist.map.json")
        {
            let path = "scripts/starting_save/persist.map.json";
            c.templates.insert(
                "old_road".into(),
                Template {
                    id: "old_road".into(),
                    source: source(o, path),
                    bytes: o.read_map(path)?,
                    enabled_references: BTreeSet::from(["starting_save".into()]),
                    disabled_references: BTreeSet::new(),
                },
            );
        }
        let path = "scripts/map_generator.darkest";
        for i in o.sources(path) {
            let b = o.read_map_source(path, i)?;
            let d = crate::darkest::parse_str(std::str::from_utf8(&b).map_err(|e| e.to_string())?);
            if !d.warnings.is_empty() {
                return Err("invalid map generator syntax".into());
            }
            for r in &d.records {
                let rule = Rule::parse(
                    r,
                    Source {
                        path: path.into(),
                        layers: vec![o.layers[i].id.clone()],
                    },
                )?;
                let list = c.rules.entry(rule.key.clone()).or_default();
                if let Some(old) = list.iter_mut().find(|old| old.raw == rule.raw) {
                    old.sources.extend(rule.sources);
                } else {
                    list.push(rule);
                }
            }
        }
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("campaign/quest/") && p.ends_with(".json"))
        {
            // Only quest definitions and goals are interpreted. Other campaign quest
            // tables are inventoried by Overlay and selected by future campaign hosts.
            if !path.ends_with("plot_quests.json") && !path.ends_with("quest.types.json") {
                continue;
            }
            for b in o.read_map_versions(path)? {
                let raw: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
                if let Some(rows) = raw.get("plot_quests") {
                    for q in rows.as_array().ok_or("invalid plot quest rows")? {
                        let id = q["id"].as_str().ok_or("plot quest missing id")?;
                        library::insert_json(&mut c.quests, id.into(), q.clone());
                    }
                }
                if let Some(types) = raw["types"].as_array() {
                    for ty in types {
                        let id = ty["id"].as_str().ok_or("quest type missing id")?;
                        if let Some(lists) = ty["goal_lists"].as_array() {
                            for list in lists {
                                let dungeon = list["dungeon"]
                                    .as_str()
                                    .ok_or("goal list missing dungeon")?;
                                let goals = list["goals"]
                                    .as_array()
                                    .ok_or("missing quest goal alternatives")?;
                                library::insert_json(
                                    &mut c.goal_groups,
                                    format!("{id}:{dungeon}"),
                                    json!({"groups":goals}),
                                );
                            }
                        }
                    }
                }
                if let Some(rows) = raw.get("goals") {
                    for g in rows.as_array().ok_or("invalid quest goals")? {
                        library::insert_json(
                            &mut c.goal_defs,
                            g["id"].as_str().ok_or("goal missing id")?.into(),
                            g.clone(),
                        );
                    }
                }
            }
        }
        for q in c.quests.values() {
            let q = if let Some(a) = q.as_array() {
                a.first().ok_or("empty quest conflict")?
            } else {
                q
            };
            if let Some(name) = q["quest"]["map_name"].as_str()
                && let Some(t) = c.templates.get_mut(name)
            {
                let id = q["id"].as_str().ok_or("quest id missing")?.into();
                if q["is_enabled"].as_bool() == Some(false) {
                    t.disabled_references.insert(id);
                } else {
                    t.enabled_references.insert(id);
                }
            }
        }
        if court {
            c.load_court_keys(o)?;
        }
        // Mash metadata is read only for binding fixed quest goals to their original
        // encounters, never executing an encounter or choosing an AI outcome here.
        for path in o.combat_files("dungeons/", ".mash.darkest") {
            for b in o.read_versions(&path)? {
                let d =
                    crate::darkest::parse_str(std::str::from_utf8(&b).map_err(|e| e.to_string())?);
                if !d.warnings.is_empty() {
                    return Err("invalid map mash syntax".into());
                }
                for r in d.records.iter().filter(|r| r.key == "named") {
                    let id = text(r, "name")?;
                    let types = r
                        .param("types")
                        .ok_or("named mash lacks types")?
                        .values
                        .iter()
                        .map(|v| v.text.clone())
                        .collect::<Vec<_>>();
                    let rows = c.named_mashes.entry(id.into()).or_default();
                    if !rows.contains(&types) {
                        rows.push(types);
                    }
                }
            }
        }
        Ok(c)
    }
    fn load_court_keys(&mut self, o: &Overlay) -> Result<()> {
        let mut inventory = BTreeSet::new();
        for path in o.paths.keys().filter(|p| {
            p.starts_with("inventory/")
                && (p.ends_with(".quest_item.inventory.items.darkest")
                    || p.ends_with(".quest.inventory.items.darkest"))
        }) {
            for bytes in o.read_map_versions(path)? {
                for r in crate::darkest::parse_str(
                    std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
                )
                .records
                {
                    if r.key == "inventory_item" && text(&r, "type")? == "quest_item" {
                        inventory.insert(text(&r, "id")?.to_string());
                    }
                }
            }
        }
        let path = "props/courtyard/prop_definitions.json";
        let raw: Value = serde_json::from_slice(&o.read_map(path)?).map_err(|e| e.to_string())?;
        for p in raw["props"]
            .as_array()
            .ok_or("missing courtyard door props")?
        {
            let d = &p["default_data"];
            let Some(interaction) = d.get("interaction_type").and_then(Value::as_str) else {
                continue;
            };
            let rows = raw["interaction_types"]
                .as_array()
                .ok_or("missing map interactions")?
                .iter()
                .filter(|r| r["name"].as_str() == Some(interaction))
                .collect::<Vec<_>>();
            if rows.len() != 1 {
                return Err("ambiguous map gate interaction".into());
            }
            let items = rows[0]["item_interaction_table"]
                .as_array()
                .ok_or("missing map gate items")?;
            if items.len() != 1 || items[0]["item_type"].as_str() != Some("quest_item") {
                return Err("unsupported map gate item alternatives".into());
            }
            let results = items[0]["result_type_table"]
                .as_array()
                .ok_or("missing gate result")?;
            if results.len() != 1
                || results[0]["result_type"].as_str() != Some("unlock_door")
                || results[0]["chance"].as_f64() != Some(1.)
            {
                return Err("unsupported nondeterministic map gate".into());
            }
            let defaults = &rows[0]["default_result_type_table"];
            if defaults
                != &serde_json::json!([{"result_type":"door_default","chance":1.0,"result_data_table":[{"chance":1.0,"data":""}]}])
                || results[0]["result_data_table"] != serde_json::json!([{"chance":1.0,"data":""}])
                || d["success_effects"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
            {
                return Err("unsupported map gate effects/data".into());
            }
            let name = p["name"].as_str().ok_or("missing gate prop name")?;
            let item = items[0]["item_name"].as_str().ok_or("missing gate key")?;
            if !inventory.contains(item) {
                return Err(format!("unknown map gate quest item {item}"));
            }
            if self.gates.insert(name.into(), item.into()).is_some() {
                return Err("duplicate gate prop".into());
            }
        }
        if self.candidates.court_key_loot.is_empty() {
            return Ok(());
        }
        let mut tables = BTreeMap::new();
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("loot/") && p.ends_with(".json"))
        {
            for b in o.read_map_versions(path)? {
                let raw: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
                if let Some(rows) = raw["loot_tables"].as_array() {
                    for t in rows {
                        if self
                            .candidates
                            .court_key_loot
                            .values()
                            .any(|v| t["id"].as_str() == Some(v))
                        {
                            let id = t["id"].as_str().unwrap();
                            library::insert_json(&mut tables, id.into(), t.clone());
                        }
                    }
                }
            }
        }
        for (curio, id) in &self.candidates.court_key_loot {
            let t = library::unique_json(&tables, id, "map key loot")?;
            let rows = t["entries"].as_array().ok_or("missing key entries")?;
            if rows.len() != 1
                || rows[0]["type"].as_str() != Some("item")
                || rows[0]["chances"].as_f64() != Some(1.)
                || rows[0]["data"]["type"].as_str() != Some("quest_item")
            {
                return Err("unsupported map key loot distribution".into());
            }
            let data = &rows[0]["data"];
            if !inventory.contains(data["id"].as_str().ok_or("missing key item")?) {
                return Err("unknown key loot inventory item".into());
            }
            self.key_awards.insert(
                curio.clone(),
                KeyAward {
                    item: data["id"].as_str().ok_or("missing key item")?.into(),
                    amount: data["amount"]
                        .as_u64()
                        .filter(|n| *n > 0 && *n <= 1000000)
                        .ok_or("invalid key loot amount")? as u32,
                },
            );
        }
        Ok(())
    }
    pub fn rule(&self, key: &Key) -> Result<Rule> {
        if key.dungeon == "courtyard" && !self.content.packages.iter().any(|p| p == "crimson_court")
        {
            return Err("courtyard generator requires crimson_court".into());
        }
        let versions = self
            .rules
            .get(key)
            .ok_or_else(|| format!("missing generator {}", key.id()))?;
        if versions.len() == 1 {
            return Ok(versions[0].clone());
        }
        if self.candidates.extended_generation_records {
            let ext = versions
                .iter()
                .filter(|r| r.prisoner.is_some())
                .collect::<Vec<_>>();
            if ext.len() == 1 {
                let r = ext[0];
                if versions.iter().all(|v| {
                    v.raw == r.raw
                        || (v.prisoner.is_none()
                            && v.raw.len() + 1 == r.raw.len()
                            && v.raw.iter().all(|(k, value)| r.raw.get(k) == Some(value)))
                }) {
                    return Ok(r.clone());
                }
            }
        }
        Err(format!("ambiguous generator {}", key.id()))
    }
    pub fn fixed(&self, id: &str) -> Result<Map> {
        let t = self
            .templates
            .get(id)
            .ok_or_else(|| format!("missing fixed map {id}"))?;
        let region = if id.starts_with("DD_") {
            "darkestdungeon"
        } else if id.starts_with("cc_") {
            "courtyard"
        } else if id == "town_invasion_0" {
            "town"
        } else if id.starts_with("com_") {
            "farm"
        } else if id == "tutorial_crypts" {
            "crypts"
        } else {
            "weald"
        };
        let mut m = if t.source.path.ends_with(".csv") {
            crate::fixed014::csv_map(
                id,
                region,
                &t.bytes,
                t.source.clone(),
                &self.gates,
                &self.key_awards,
                &self.candidates.csv_curio_choices,
            )?
        } else {
            crate::fixed014::dson(id, region, &t.bytes, t.source.clone(), &self.props)?
        };
        m.persistent = t.enabled_references.iter().any(|qid| {
            self.quests
                .get(qid)
                .is_some_and(|q| q["is_persistent_through_campaign"].as_bool() == Some(true))
        });
        Ok(m)
    }
    pub fn route(&self, id: &str) -> Result<Route> {
        let q = library::unique_json(&self.quests, id, "plot map quest")?;
        if q["is_enabled"].as_bool() == Some(false) {
            return Ok(Route::Disabled);
        }
        let quest = &q["quest"];
        if let Some(map) = quest["map_name"].as_str() {
            return Ok(if self.templates.contains_key(map) {
                Route::Fixed(map.into())
            } else {
                Route::Unresolved(format!("missing referenced map {map}"))
            });
        }
        let kind = quest["type"].as_str().ok_or("missing quest type")?;
        let dungeon = quest["dungeon"].as_str().ok_or("missing quest dungeon")?;
        if kind == "wave" && dungeon == "farm" {
            return Ok(Route::Wave(id.into()));
        }
        let size = match quest["length"].as_u64() {
            Some(1) => "short",
            Some(2) => "medium",
            Some(3) => "long",
            _ => return Ok(Route::Unresolved("unknown random quest length".into())),
        };
        let key = Key {
            dungeon: dungeon.into(),
            size: size.into(),
            quest: kind.into(),
        };
        Ok(if self.rule(&key).is_ok() {
            Route::Random(key)
        } else {
            Route::Unresolved(format!("unresolved quest generator {}", key.id()))
        })
    }
    pub fn fixed_for_quest(&self, id: &str) -> Result<Map> {
        let Route::Fixed(map) = self.route(id)? else {
            return Err("quest is not an enabled fixed map".into());
        };
        let mut m = self.fixed(&map)?;
        let q = library::unique_json(&self.quests, id, "map quest")?;
        self.bind_goals(&mut m, &q["quest"])?;
        m.raw["task"] = q.clone();
        if m.goals.is_empty() && q["quest"]["type"].as_str() != Some("explore") {
            return Err("fixed quest has no resolved goal node".into());
        }
        Ok(m)
    }
    pub fn generated_goals(&self, key: &Key) -> Result<Vec<Vec<String>>> {
        let id = format!("{}:{}", key.quest, key.dungeon);
        if !self.goal_groups.contains_key(&id) {
            return Ok(vec![]);
        }
        let value = library::unique_json(&self.goal_groups, &id, "generator goal alternatives")?;
        serde_json::from_value(value["groups"].clone()).map_err(|e| e.to_string())
    }
    /// Campaign generation combines the shared goals with the region-specific group.
    /// The legacy map probe keeps generated_goals unchanged for its frozen records.
    pub fn campaign_goals(&self, key: &Key) -> Result<Vec<Vec<String>>> {
        combined_goal_groups(&self.goal_groups, &key.quest, &key.dungeon)
    }
    pub fn repeat_rule(&self, kind: &str) -> Result<(Rule, String)> {
        if !self.candidates.court_task_aliases {
            return Err("court repeat quest binding requires explicit map candidates".into());
        }
        let (size, quest, goal) = match kind {
            "farm_bloodcrate" => ("short", "explore", "gather_bloodcrate"),
            "kill_statue" => ("medium", "kill_boss", "kill_statue_C"),
            _ => return Err("unknown courtyard repeat quest".into()),
        };
        Ok((
            self.rule(&Key {
                dungeon: "courtyard".into(),
                size: size.into(),
                quest: quest.into(),
            })?,
            goal.into(),
        ))
    }
    pub fn bind_goals(&self, map: &mut Map, quest: &Value) -> Result<()> {
        let ids = quest["goal_ids"]
            .as_array()
            .ok_or("missing map quest goals")?;
        for goal in ids {
            let id = goal.as_str().ok_or("invalid quest goal id")?;
            let g = library::unique_json(&self.goal_defs, id, "map quest goal")?;
            let ty = g["type"].as_str().ok_or("missing map goal type")?;
            match ty {
                "explore" | "explore_room" => {}
                "activate" | "gather" | "inventory_activate" => {
                    if let Some(curio) = g["data"]["curio_name"].as_str() {
                        if self.candidates.quest_placeholders.contains(&map.id) {
                            let slots = map
                                .nodes
                                .iter()
                                .filter(|(_, n)| {
                                    n.content
                                        .curio
                                        .as_deref()
                                        .is_some_and(|v| v.starts_with("hash:"))
                                        && n.content.code == Some(6)
                                })
                                .map(|(id, _)| id.clone())
                                .collect::<Vec<_>>();
                            let amount = g["data"]["amount"]
                                .as_u64()
                                .ok_or("missing fixed goal amount")?;
                            if slots.len() as u64 != amount {
                                return Err("fixed quest placeholder count mismatch".into());
                            }
                            for id in slots {
                                map.nodes.get_mut(&id).unwrap().content.curio = Some(curio.into());
                            }
                        }
                        for (nid, n) in &mut map.nodes {
                            if n.content.curio.as_deref() == Some(curio) {
                                n.content.quest = true;
                                map.goals.insert(nid.clone());
                            }
                        }
                    }
                }
                "kill_monster" => {
                    let monsters = g["data"]["monster_class_ids"]
                        .as_array()
                        .ok_or("missing kill target ids")?;
                    for (nid, n) in &map.nodes {
                        if let Some(mash) = &n.content.battle
                            && self.named_mashes.get(mash).is_some_and(|rows| {
                                rows.iter().any(|row| {
                                    row.iter()
                                        .any(|m| monsters.iter().any(|v| v.as_str() == Some(m)))
                                })
                            })
                        {
                            map.goals.insert(nid.clone());
                        }
                    }
                }
                "survive" | "survive_waves" => {}
                _ => return Err(format!("unknown map goal type {ty}")),
            }
        }
        if quest["type"].as_str() == Some("kill_boss") {
            for id in map.goals.clone() {
                let n = map.nodes.get_mut(&id).unwrap();
                n.content.quest = true;
                if n.content.battle.is_none() {
                    n.content.battle = Some(format!(
                        "quest:{}",
                        ids.first()
                            .and_then(Value::as_str)
                            .ok_or("missing boss goal")?
                    ));
                }
            }
        }
        map.raw = json!({"template":map.raw,"quest":quest,"goal_definitions":ids.iter().filter_map(|id|id.as_str().and_then(|id|self.goal_defs.get(id))).collect::<Vec<_>>()});
        map.validate()
    }
    pub fn audit(&self) -> Value {
        let refs=self.templates.values().map(|t|json!({"id":t.id,"source":t.source,"role":if !t.enabled_references.is_empty(){"production"}else if !t.disabled_references.is_empty(){"disabled_test"}else{"unreferenced"},"enabled_references":t.enabled_references,"disabled_references":t.disabled_references})).collect::<Vec<_>>();
        let quests = self
            .quests
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    match self.route(id) {
                        Ok(r) => json!(r),
                        Err(e) => json!({"error":e}),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        json!({"content_id":self.content.id().ok(),"candidate_id":config_identity(&self.content,&self.candidates).ok(),"templates":refs,"generator_keys":self.rules.keys().map(Key::id).collect::<Vec<_>>(),"generator_errors":self.rules.keys().filter_map(|k|self.rule(k).err().map(|e|(k.id(),e))).collect::<BTreeMap<_,_>>(),"quests":quests})
    }
}

/// Candidate composition: shared "all" requirements plus region-specific alternatives.
pub fn combined_goal_groups(
    groups: &BTreeMap<String, Value>,
    kind: &str,
    dungeon: &str,
) -> Result<Vec<Vec<String>>> {
    let mut result = vec![vec![]];
    let mut found = false;
    for scope in ["all", dungeon] {
        let key = format!("{kind}:{scope}");
        let Some(_) = groups.get(&key) else { continue };
        found = true;
        let value = library::unique_json(groups, &key, "campaign goal groups")?;
        let options: Vec<Vec<String>> =
            serde_json::from_value(value["groups"].clone()).map_err(|e| e.to_string())?;
        if options.is_empty() {
            return Err("empty source goal group alternatives".into());
        }
        let mut next = vec![];
        for base in &result {
            for option in &options {
                let mut combined = base.clone();
                for id in option {
                    if !combined.contains(id) {
                        combined.push(id.clone());
                    }
                }
                if !next.contains(&combined) {
                    next.push(combined);
                }
            }
        }
        result = next;
    }
    if !found {
        return Err(format!("missing campaign goal groups {kind}:{dungeon}"));
    }
    Ok(result)
}
