//! Host-only concrete content bindings. These definitions never enter policy observations.
use crate::{Blueprint, Rng};
use dd_data::{
    catalog014::Catalog,
    combat::{GameData, Result, number, text},
    curios013::{Curios, Outcome},
    field017::{Curio, Hazard, Props, ResultKind},
    maps014::Map,
    opening_spawn015::{MonsterLoot, Pools},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NamedFormation {
    pub weight: f64,
    pub monsters: Vec<String>,
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flashback: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Special {
    Quest(Vec<String>),
    Prisoner(Value),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub map: Map,
    pub quest: Value,
    pub seed: u64,
    pub goals: BTreeMap<String, Value>,
    pub curio_names: BTreeMap<String, String>,
    pub curios: BTreeMap<String, Curio>,
    pub effects: BTreeMap<String, dd_data::combat::Effect>,
    pub traps: BTreeMap<String, Hazard>,
    pub obstacles: BTreeMap<String, Hazard>,
    pub special: BTreeMap<String, Special>,
    pub pools: Pools,
    pub fixed_formations: BTreeMap<String, Vec<String>>,
    pub named: BTreeMap<String, Vec<NamedFormation>>,
    pub flashbacks: BTreeMap<String, dd_data::nightmare011::Flashback>,
    pub catalog: BTreeMap<String, Blueprint>,
    pub monster_loot: BTreeMap<String, MonsterLoot>,
}
pub(crate) fn pick(rng: &mut Rng, weights: &[f64]) -> Result<usize> {
    let total = weights.iter().sum::<f64>();
    if !total.is_finite() || total <= 0. || weights.iter().any(|n| !n.is_finite() || *n < 0.) {
        return Err("invalid expedition source weights".into());
    }
    let mut n = (rng.next() >> 11) as f64 / ((1u64 << 53) as f64) * total;
    for (i, w) in weights.iter().enumerate() {
        if n < *w {
            return Ok(i);
        }
        n -= w;
    }
    weights
        .iter()
        .rposition(|n| *n > 0.)
        .ok_or("empty weighted expedition source".into())
}
fn selector(doc: &dd_data::darkest::Document, key: &str, rng: &mut Rng) -> Result<String> {
    let mut names = vec![];
    let mut weights = vec![];
    for row in doc.records_keyed(key) {
        if row
            .params
            .iter()
            .any(|p| !["chance", "types"].contains(&p.name.as_str()))
        {
            return Err(format!("unknown {key} source parameter"));
        }
        let w = number(row, "chance")?;
        let types = row
            .param("types")
            .filter(|p| !p.values.is_empty())
            .ok_or("empty prop type selector")?;
        for value in &types.values {
            names.push(value.text.clone());
            weights.push(w / types.values.len() as f64);
        }
    }
    Ok(names[pick(rng, &weights).map_err(|e| format!("source prop selector {key}: {e}"))?].clone())
}
fn formations(
    g: &GameData,
    region: &str,
    difficulty: u32,
    name: &str,
    boss_targets: Option<&BTreeSet<String>>,
    automation: bool,
) -> Result<Vec<NamedFormation>> {
    let mut result = vec![];
    let mut missing_weight = false;
    for path in g.overlay.combat_files(
        &format!("dungeons/{region}/"),
        &format!(".{difficulty}.mash.darkest"),
    ) {
        let d = dd_data::library::read_document(&g.overlay, &path)?;
        for row in &d.records {
            let selected = if let Some(targets) = boss_targets {
                if !["boss", "miniboss", "localminiboss"].contains(&row.key.as_str()) {
                    false
                } else if let Some(types) = row.param("types") {
                    let direct = types.values.iter().any(|v| targets.contains(&v.text));
                    if !direct && automation {
                        // A source boss may begin in a different form from the kill objective.
                        // Resolve the original lifecycle closure rather than inventing a starter ID.
                        let roots = types
                            .values
                            .iter()
                            .map(|v| v.text.as_str())
                            .collect::<Vec<_>>();
                        let closure = g.entity_catalog(&roots)?;
                        targets.iter().any(|id| closure.contains_key(id))
                    } else {
                        direct
                    }
                } else {
                    false
                }
            } else if let Some(key) = name.strip_prefix("pool:") {
                row.key == key
            } else {
                row.key == "named" && text(row, "name").ok() == Some(name)
            };
            if !selected {
                continue;
            }
            if row.params.iter().any(|p| {
                ![
                    "name",
                    "chance",
                    "types",
                    "background",
                    "loading_screen",
                    "flashback_id",
                ]
                .contains(&p.name.as_str())
            }) {
                return Err(format!("unknown named formation field {path}:{}", row.line));
            }
            let weight = if row.param("chance").is_some() {
                number(row, "chance")?
            } else {
                missing_weight = true;
                // A sole concrete formation needs no relative probability.
                // Reject this normalization if another alternative exists.
                1.
            };
            if !weight.is_finite() || weight < 0. {
                return Err("invalid named formation weight".into());
            }
            let monsters = row
                .param("types")
                .filter(|p| !p.values.is_empty())
                .ok_or("empty named formation")?
                .values
                .iter()
                .map(|v| v.text.clone())
                .collect::<Vec<_>>();
            let flashback = row
                .param("flashback_id")
                .map(|_| text(row, "flashback_id").map(str::to_string))
                .transpose()?;
            if flashback.as_ref().is_some_and(|id| id.is_empty()) {
                return Err("empty named flashback ID".into());
            }
            if result.iter().any(|old: &NamedFormation| {
                old.monsters == monsters && old.weight == weight && old.flashback == flashback
            }) {
                continue;
            }
            if let Some(old) = result
                .iter()
                .find(|old: &&NamedFormation| old.monsters == monsters)
            {
                let exact_dd_stall = automation
                    && region == "darkestdungeon"
                    && name == "pool:stall"
                    && monsters == ["cultist_shrouded_D", "cultist_shrouded_D"]
                    && [1., 2.].contains(&old.weight)
                    && [1., 2.].contains(&weight)
                    && old.flashback == flashback;
                if !exact_dd_stall {
                    return Err(format!(
                        "conflicting named formation weight {region}/{difficulty}/{name}"
                    ));
                }
            }
            result.push(NamedFormation {
                weight,
                monsters,
                sources: vec![path.clone(), format!("line:{}", row.line)],
                flashback,
            });
        }
    }
    if !result.iter().any(|r| r.weight > 0.) {
        return Err(format!(
            "missing named formation {region}/{difficulty}/{name}"
        ));
    }
    if missing_weight && result.len() != 1 {
        return Err("missing relative weights in alternative named formations".into());
    }
    Ok(result)
}
///026 may read the original public stall table, never a bound encounter/map source.
pub(crate) fn rollout_stall026(
    g: &GameData,
    region: &str,
    difficulty: u32,
) -> Result<Vec<NamedFormation>> {
    formations(g, region, difficulty, "pool:stall", None, true)
}
impl Sources {
    pub fn bind(
        g: &GameData,
        catalog: &Catalog,
        map: Map,
        quest: Value,
        seed: u64,
    ) -> Result<Self> {
        Self::bind_inner(g, catalog, map, quest, seed, false)
    }
    pub fn bind_automation018(
        g: &GameData,
        catalog: &Catalog,
        map: Map,
        quest: Value,
        seed: u64,
    ) -> Result<Self> {
        Self::bind_inner(g, catalog, map, quest, seed, true)
    }
    fn bind_inner(
        g: &GameData,
        catalog: &Catalog,
        mut map: Map,
        quest: Value,
        seed: u64,
        automation: bool,
    ) -> Result<Self> {
        if catalog.content != g.overlay.config {
            return Err("expedition source content differs".into());
        }
        if map.persistent {
            map.fog = true;
        }
        map.validate()?;
        if map.generation.is_some() {
            crate::generate014::validate_generated(&map)?;
        }
        let region = quest["dungeon"]
            .as_str()
            .ok_or("missing expedition dungeon")?
            .to_string();
        let difficulty = dd_data::campaign017::uint(&quest, "difficulty")?;
        let library = Curios::load_campaign(&g.overlay)?;
        let props = Props::load(&g.overlay)?;
        let props_bytes = g
            .overlay
            .read_opening(&format!("dungeons/{region}/{region}.props.darkest"))?;
        let props_doc = dd_data::darkest::parse_str(
            std::str::from_utf8(&props_bytes).map_err(|e| e.to_string())?,
        );
        if !props_doc.warnings.is_empty() {
            return Err("invalid expedition prop selector syntax".into());
        }
        let pools = dd_data::opening_spawn015::campaign_load(g, &region, difficulty, &library)?;
        let mut goal_defs = BTreeMap::<String, Value>::new();
        for id in quest["goal_ids"]
            .as_array()
            .ok_or("missing expedition goals")?
        {
            let id = id.as_str().ok_or("invalid goal ID")?;
            goal_defs.insert(
                id.into(),
                dd_data::library::unique_json(&catalog.goal_defs, id, "expedition goal")?.clone(),
            );
        }
        if automation {
            for goal in goal_defs.values() {
                if let Some(table) = goal["data"]["additional_loot"].as_str() {
                    library.validate_campaign_draw(
                        table,
                        i64::from(difficulty),
                        &region,
                        &mut vec![],
                    )?;
                }
            }
        }
        let curio_goals = goal_defs
            .iter()
            .filter(|(_, g)| g["data"]["curio_name"].is_string())
            .collect::<Vec<_>>();
        let markers = map
            .nodes
            .iter()
            .filter(|(_, n)| n.content.quest && n.content.curio.is_none())
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if !markers.is_empty() && !curio_goals.is_empty() {
            if curio_goals.len() != 1 {
                return Err("ambiguous unlabeled quest curio markers".into());
            }
            let (goal_id, goal) = curio_goals[0];
            let amount = if goal["type"] == "gather" {
                dd_data::campaign017::uint(&goal["data"]["item"], "amount")?
            } else {
                dd_data::campaign017::uint(&goal["data"], "amount")?
            };
            if markers.len() != amount as usize {
                return Err("unlabeled quest marker count differs from source goal".into());
            }
            for id in markers {
                let node = map.nodes.get_mut(&id).unwrap();
                node.content.curio = Some(goal["data"]["curio_name"].as_str().unwrap().into());
                node.metadata["quest_marker_binding017"] =
                    serde_json::json!({"goal":goal_id,"original":"quest_curio"});
            }
        }
        let mut s = Self {
            map,
            quest,
            seed,
            goals: BTreeMap::new(),
            curio_names: BTreeMap::new(),
            curios: BTreeMap::new(),
            effects: BTreeMap::new(),
            traps: BTreeMap::new(),
            obstacles: BTreeMap::new(),
            special: BTreeMap::new(),
            catalog: pools.catalog.clone(),
            pools,
            fixed_formations: BTreeMap::new(),
            named: BTreeMap::new(),
            flashbacks: if g
                .overlay
                .config
                .packages
                .iter()
                .any(|p| p == "shieldbreaker")
            {
                g.flashback_catalog()?
            } else {
                BTreeMap::new()
            },
            monster_loot: BTreeMap::new(),
        };
        let mut rng = Rng(seed);
        for id in s.quest["goal_ids"]
            .as_array()
            .ok_or("missing expedition goals")?
        {
            let id = id.as_str().ok_or("invalid source goal ID")?;
            s.goals.insert(
                id.into(),
                dd_data::library::unique_json(&catalog.goal_defs, id, "expedition goal")?.clone(),
            );
        }
        let nodes = s.map.nodes.keys().cloned().collect::<Vec<_>>();
        let mut wanted_named = BTreeSet::new();
        let mut wanted_boss = BTreeMap::new();
        let quirks = g.quirks()?;
        for id in nodes {
            let node = &s.map.nodes[&id];
            if let Some(reference) = &node.content.battle {
                if reference.starts_with("pool:") {
                    if !["pool:room", "pool:hallway"].contains(&reference.as_str()) {
                        return Err(format!("unknown encounter source pool {reference}"));
                    }
                } else if reference.starts_with("quest:") {
                    let goal_id = reference.strip_prefix("quest:").unwrap();
                    let goal = s
                        .goals
                        .get(goal_id)
                        .ok_or("boss marker has no quest goal")?;
                    let targets = goal["data"]["monster_class_ids"]
                        .as_array()
                        .ok_or("boss goal has no monster classes")?
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(str::to_string)
                                .ok_or_else(|| "invalid boss goal monster class".to_string())
                        })
                        .collect::<Result<BTreeSet<_>>>()?;
                    wanted_boss.insert(reference.clone(), targets);
                } else {
                    wanted_named.insert(reference.clone());
                }
            }
            if let Some(reference) = &node.content.curio {
                let name = if reference.starts_with("pool:") {
                    let key = match reference.as_str() {
                        "pool:hallway_curio" => "hall_curios",
                        "pool:room_curio" => "room_curios",
                        "pool:treasure" => "room_treasures",
                        "pool:secret_treasure" => "secret_room_treasures",
                        _ => return Err(format!("unknown curio source pool {reference}")),
                    };
                    let key = if automation
                        && region == "courtyard"
                        && key == "room_curios"
                        && props_doc.records_keyed(key).next().is_none()
                    {
                        // Explicit 018 candidate for the 014 repeat-map alias: use the
                        // existing original Courtyard curio pool, without invented weights.
                        "hall_curios"
                    } else {
                        key
                    };
                    selector(&props_doc, key, &mut rng)?
                } else {
                    reference.clone()
                };
                let quest_goals = s
                    .goals
                    .iter()
                    .filter(|(_, goal)| goal["data"]["curio_name"].as_str() == Some(name.as_str()))
                    .map(|(goal, _)| goal.clone())
                    .collect::<Vec<_>>();
                if !quest_goals.is_empty() {
                    s.special.insert(id.clone(), Special::Quest(quest_goals));
                } else if name == "prisoner" {
                    let definition = props.resolve(&name, difficulty)?;
                    if definition["instance_type"] != "prisoner" {
                        return Err("source prisoner instance type differs".into());
                    }
                    s.special.insert(id.clone(), Special::Prisoner(definition));
                } else {
                    let (_, curio) = library
                        .campaign_prop(&name)
                        .map_err(|e| format!("{id}/{name}: {e}"))?;
                    for outcome in curio
                        .outcomes
                        .iter()
                        .map(|(r, _)| r)
                        .chain(curio.items.values())
                    {
                        match outcome {
                            ResultKind::Basic(Outcome::Effect(rows)) => {
                                for (effect, _) in rows {
                                    s.effects.insert(effect.clone(), g.field_effect(effect)?);
                                }
                            }
                            ResultKind::Basic(Outcome::Loot { draws, .. }) => {
                                for draw in draws {
                                    library.validate_campaign_draw(
                                        &draw.table,
                                        i64::from(difficulty),
                                        &region,
                                        &mut vec![],
                                    )?;
                                }
                            }
                            ResultKind::Quirk(rows) => {
                                for (quirk, _) in rows {
                                    if !["positive", "negative"].contains(&quirk.as_str()) {
                                        let q = dd_data::library::unique_json(
                                            &quirks,
                                            quirk,
                                            "curio quirk",
                                        )?;
                                        if q["is_disease"] != false {
                                            return Err(
                                                "disease referenced as a curio quirk".into()
                                            );
                                        }
                                    }
                                }
                            }
                            ResultKind::Disease(rows) => {
                                for (disease, _) in rows {
                                    if disease == "random" {
                                        continue;
                                    }
                                    if let Ok(q) = dd_data::library::unique_json(
                                        &quirks,
                                        disease,
                                        "curio disease",
                                    ) {
                                        if q["is_disease"] != true {
                                            return Err(
                                                "quirk referenced as a curio disease".into()
                                            );
                                        }
                                    } else {
                                        let effect = g.field_effect(disease)?;
                                        if !matches!(
                                            effect.kind,
                                            dd_data::combat::EffectKind::Disease { .. }
                                        ) {
                                            return Err("non-disease curio disease effect".into());
                                        }
                                        s.effects.insert(disease.clone(), effect);
                                    }
                                }
                            }
                            ResultKind::Summon(rows) => {
                                wanted_named.extend(rows.iter().map(|(name, _)| name.clone()))
                            }
                            ResultKind::Basic(Outcome::Nothing)
                            | ResultKind::Purge(_)
                            | ResultKind::Scout(_) => {}
                        }
                    }
                    s.curios.insert(id.clone(), curio);
                }
                s.curio_names.insert(id.clone(), name);
            }
            for (reference, is_trap) in
                [(&node.content.trap, true), (&node.content.obstacle, false)]
            {
                if let Some(reference) = reference {
                    let name = if reference.starts_with("pool:") {
                        selector(
                            &props_doc,
                            if is_trap { "traps" } else { "obstacles" },
                            &mut rng,
                        )?
                    } else if automation && reference == "index:0" {
                        let key = if is_trap { "traps" } else { "obstacles" };
                        let types = props_doc
                            .records_keyed(key)
                            .filter_map(|r| r.param("types"))
                            .flat_map(|p| p.values.iter().map(|v| v.text.clone()))
                            .collect::<BTreeSet<_>>();
                        if types.len() != 1 {
                            return Err(format!("ambiguous fixed hazard index {reference}: {key}"));
                        }
                        types.into_iter().next().unwrap()
                    } else {
                        reference.clone()
                    };
                    let hazard = Hazard::load(g, &props, &name, difficulty)?;
                    if is_trap {
                        s.traps.insert(id.clone(), hazard);
                    } else {
                        s.obstacles.insert(id.clone(), hazard);
                    }
                }
            }
        }
        for path in g.overlay.combat_files(
            &format!("dungeons/{region}/"),
            &format!(".{difficulty}.mash.darkest"),
        ) {
            let doc = dd_data::library::read_document(&g.overlay, &path)?;
            for row in doc
                .records_keyed("named")
                .filter(|r| r.param("flashback_id").is_some())
            {
                let id = text(row, "flashback_id")?;
                if !s.flashbacks.contains_key(id) {
                    return Err(format!("missing enabled flashback definition {id}"));
                }
                wanted_named.insert(text(row, "name")?.into());
            }
        }
        if s.quest["is_stall_enabled"] == true {
            wanted_named.insert("pool:stall".into());
        }
        for name in wanted_named {
            s.named.insert(
                name.clone(),
                formations(g, &region, difficulty, &name, None, automation)?,
            );
        }
        for (name, targets) in wanted_boss {
            s.named.insert(
                name.clone(),
                formations(g, &region, difficulty, &name, Some(&targets), automation)?,
            );
        }
        let mut roots = s.catalog.keys().cloned().collect::<BTreeSet<_>>();
        for choices in s.named.values() {
            for choice in choices.iter().filter(|c| c.weight > 0.) {
                roots.extend(choice.monsters.clone());
            }
        }
        s.catalog = g.entity_catalog(&roots.iter().map(String::as_str).collect::<Vec<_>>())?;
        for (id, node) in &s.map.nodes {
            if let Some(reference) = &node.content.battle
                && !reference.starts_with("pool:")
            {
                let options = s
                    .named
                    .get(reference)
                    .ok_or("missing fixed formation binding")?;
                let i = pick(
                    &mut rng,
                    &options.iter().map(|r| r.weight).collect::<Vec<_>>(),
                )?;
                s.fixed_formations
                    .insert(id.clone(), options[i].monsters.clone());
            }
        }
        for (id, blueprint) in &s.catalog {
            if blueprint.hero {
                return Err("enemy encounter catalog contains a hero".into());
            }
            s.monster_loot.insert(
                id.clone(),
                dd_data::opening_spawn015::campaign_monster_loot(
                    g, id, difficulty, &region, &library,
                )?,
            );
        }
        for formation in s.fixed_formations.values() {
            let occupied = formation
                .iter()
                .map(|id| u32::from(s.catalog[id].size))
                .sum::<u32>();
            if occupied == 0 || occupied > 4 {
                return Err("source formation exceeds four ranks".into());
            }
        }
        Ok(s)
    }
}

/// Fingerprint only recognized enabled text scopes, never executable/media/player saves.
pub fn fingerprint(g: &GameData) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let overlay = g.overlay.campaign_view()?;
    hash.update(overlay.config.id()?.as_bytes());
    for path in overlay
        .paths
        .keys()
        .filter(|p| dd_data::overlay::combat_path(p) || dd_data::campaign017::source_path(p))
    {
        hash.update((path.len() as u64).to_le_bytes());
        hash.update(path.as_bytes());
        let versions = if dd_data::overlay::combat_path(path) {
            overlay.read_versions(path)?
        } else {
            overlay.read_campaign_versions(path)?
        };
        hash.update((versions.len() as u64).to_le_bytes());
        for bytes in versions {
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

