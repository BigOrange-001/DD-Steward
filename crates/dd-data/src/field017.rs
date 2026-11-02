//! General expedition curio data. Legacy 013/015 narrow loading remains strict.
use crate::{
    combat::{self, GameData, Result},
    curios013::{self, Curios, Outcome, col, positive},
    darkest::Param,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Curio {
    pub id: String,
    pub full: bool,
    pub outcomes: Vec<(ResultKind, f64)>,
    pub items: BTreeMap<String, ResultKind>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResultKind {
    Basic(Outcome),
    Quirk(Vec<(String, f64)>),
    Purge(Vec<(String, f64)>),
    Disease(Vec<(String, f64)>),
    Scout(Vec<(String, f64)>),
    Summon(Vec<(String, f64)>),
}
fn result(curio: &str, kind: &str, row: &[String]) -> Result<ResultKind> {
    if ["Nothing", "Loot", "Effect"].contains(&kind) {
        return curios013::parse_outcome(kind, row).map(ResultKind::Basic);
    }
    if !["Quirk", "Purge", "Disease", "Scouting", "Summon"].contains(&kind) {
        return Err(format!("unknown expedition curio result {kind}"));
    }
    let mut choices = vec![];
    for index in [7, 10, 13] {
        let id = col(row, index);
        if id.is_empty() || id == "N/A" {
            continue;
        }
        let weight = col(row, index + 1);
        let weight = if weight.is_empty()
            && (kind == "Summon" && ["egg_cluster_1", "infested_egg_cluster_1"].contains(&id)
                || curio == "cc_food_cart" && kind == "Quirk" && id == "stress_eater")
            && [10, 13]
                .iter()
                .all(|index| col(row, *index).is_empty() || col(row, *index) == "N/A")
        {
            1.
        } else {
            positive(weight)?
        };
        if weight > 0. {
            choices.push((id.to_string(), weight));
        }
    }
    if choices.is_empty() {
        return Err("empty expedition curio choices".into());
    }
    for (id, _) in &choices {
        if kind == "Purge" && !["positive", "negative"].contains(&id.as_str()) {
            return Err("unsupported curio purge scope".into());
        }
        if kind == "Scouting" {
            let (distance, scope) = id
                .split_once(" - ")
                .ok_or("invalid curio scouting descriptor")?;
            distance
                .parse::<u32>()
                .map_err(|_| "invalid curio scouting distance")?;
            if ![
                "all",
                "curios",
                "hall_battles",
                "room_battles",
                "obstacles",
                "traps",
                "map",
                "room_treasures",
            ]
            .contains(&scope)
            {
                return Err("unsupported curio scouting scope".into());
            }
        }
    }
    Ok(match kind {
        "Quirk" => ResultKind::Quirk(choices),
        "Purge" => ResultKind::Purge(choices),
        "Disease" => ResultKind::Disease(choices),
        "Scouting" => ResultKind::Scout(choices),
        "Summon" => ResultKind::Summon(choices),
        _ => unreachable!(),
    })
}
impl Curios {
    pub fn load_campaign(o: &crate::overlay::Overlay) -> Result<Self> {
        let _profile = crate::telemetry024c::scope("curios_campaign");
        let view = o.campaign_view()?;
        let court = view.config.packages.iter().any(|p| p == "crimson_court");
        let mut all = Self::load_with_court_loot(&view, court)?;
        if view.config.packages.iter().any(|p| p == "districts") {
            let path = "loot/blueprint_overrides.loot.json";
            let mut selected = None;
            for layer in view.sources(path) {
                if view.layers[layer].id == "package:districts:0" {
                    let raw: serde_json::Value =
                        serde_json::from_slice(&view.read_initial_source(path, layer)?)
                            .map_err(|e| e.to_string())?;
                    let rows = raw["loot_tables"]
                        .as_array()
                        .ok_or("missing district blueprint loot table")?;
                    if rows.len() != 1
                        || rows[0]["id"] != "BLUEPRINT"
                        || rows[0]["difficulty"] != 0
                        || rows[0]["dungeon"] != ""
                        || selected.is_some()
                    {
                        return Err("unknown district blueprint source layout".into());
                    }
                    selected = Some(rows[0].clone());
                    all.loot_choices.insert(
                        "BLUEPRINT:0:".into(),
                        format!("{}/{}", view.layers[layer].id, path),
                    );
                }
            }
            all.loot.insert(
                "BLUEPRINT:0:".into(),
                selected.ok_or("missing enabled district blueprint source")?,
            );
        }
        all.types.clear();
        for path in view
            .paths
            .keys()
            .filter(|p| p.starts_with("curios/") && p.ends_with("curio_type_library.csv"))
        {
            for bytes in view.read_initial_versions(path)? {
                for (id, value) in campaign_type_blocks(&bytes)? {
                    crate::library::insert_json(&mut all.types, id, value);
                }
            }
        }
        let inventory = crate::inventory015::InventoryRules::load(&view, &all)?;
        all.items = inventory.items;
        Ok(all)
    }
    pub fn campaign_curio(&self, id: &str) -> Result<Curio> {
        let rows = self.source_rows(id)?;
        let (full, outcomes, items) =
            curios013::decode_curio_rows(id, &rows, |kind, row| result(id, kind, row))?;
        Ok(Curio {
            id: id.into(),
            full,
            outcomes,
            items,
        })
    }
    pub fn campaign_prop(&self, name: &str) -> Result<(String, Curio)> {
        let raw = crate::library::unique_json(&self.props, name, "campaign curio prop")?;
        let id = raw["row"][2]
            .as_str()
            .ok_or("missing curio type reference")?;
        Ok((name.into(), self.campaign_curio(id)?))
    }
}
impl GameData {
    /// The curio marker is validated here; all actual operations use the combat parser.
    pub fn field_effect(&self, id: &str) -> Result<combat::Effect> {
        let mut record = self.effect_source(id)?.clone();
        if record.param("curio").is_some() {
            if combat::integer(&record, "curio")? != 1 {
                return Err("invalid curio effect marker".into());
            }
            record.params.retain(|p| p.name != "curio");
        }
        let explicit_duration =
            record.param("duration").is_some() || record.param("buff_duration_type").is_some();
        if !explicit_duration
            && record
                .params
                .iter()
                .any(|p| ["buff_ids", "combat_stat_buff"].contains(&p.name.as_str()))
        {
            record.params.push(Param {
                name: "buff_duration_type".into(),
                values: vec![crate::darkest::Value {
                    text: "quest_end".into(),
                    quoted: true,
                }],
            });
        }
        self.effect_from_record(&record, id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Props {
    pub definitions: BTreeMap<String, serde_json::Value>,
    pub interactions: BTreeMap<String, serde_json::Value>,
}
impl Props {
    pub fn load(o: &crate::overlay::Overlay) -> Result<Self> {
        let mut all = Self {
            definitions: BTreeMap::new(),
            interactions: BTreeMap::new(),
        };
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("props/") && p.ends_with("_definitions.json"))
        {
            for bytes in o.read_map_versions(path)? {
                let v: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                crate::campaign017::fields(
                    &v,
                    &["props", "interaction_types"],
                    "expedition prop library",
                )?;
                for (key, target, idkey) in [
                    ("props", &mut all.definitions, "name"),
                    ("interaction_types", &mut all.interactions, "name"),
                ] {
                    if let Some(rows) = v[key].as_array() {
                        for row in rows {
                            let id = row[idkey]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .ok_or("prop library missing name")?;
                            crate::library::insert_json(target, id.into(), row.clone());
                        }
                    }
                }
            }
        }
        Ok(all)
    }
    pub fn resolve(&self, name: &str, difficulty: u32) -> Result<serde_json::Value> {
        self.resolve_inner(name, difficulty, &mut vec![])
    }
    fn resolve_inner(
        &self,
        name: &str,
        difficulty: u32,
        chain: &mut Vec<String>,
    ) -> Result<serde_json::Value> {
        if chain.iter().any(|s| s == name) || chain.len() > 64 {
            return Err("cyclic prop inheritance".into());
        }
        chain.push(name.into());
        let raw = crate::library::unique_json(&self.definitions, name, "expedition prop")?;
        crate::campaign017::fields(
            raw,
            &["name", "default_data", "difficulty_variations"],
            "expedition prop",
        )?;
        let local = raw["default_data"]
            .as_object()
            .ok_or("missing prop defaults")?;
        let mut data = if let Some(parent) = local.get("inherits_from") {
            crate::campaign017::fields(parent, &["prop_type_name"], "prop inheritance")?;
            let parent = parent["prop_type_name"]
                .as_str()
                .ok_or("missing parent prop")?;
            self.resolve_inner(parent, difficulty, chain)?
                .as_object()
                .ok_or("invalid resolved prop")?
                .clone()
        } else {
            serde_json::Map::new()
        };
        for (key, value) in local {
            if key != "inherits_from" {
                data.insert(key.clone(), value.clone());
            }
        }
        if let Some(rows) = raw["difficulty_variations"].as_array() {
            let mut candidates = BTreeMap::new();
            for row in rows {
                let level = crate::campaign017::uint(row, "level")?;
                if candidates.insert(level, row).is_some() {
                    return Err("duplicate prop difficulty variation".into());
                }
            }
            if let Some((_, row)) = candidates.range(..=difficulty).next_back() {
                for (key, value) in row.as_object().ok_or("invalid prop variation")? {
                    if key != "level" {
                        data.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        chain.pop();
        Ok(serde_json::Value::Object(data))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hazard {
    pub id: String,
    pub source: serde_json::Value,
    pub health: f64,
    pub light: i32,
    pub success: Vec<combat::Effect>,
    pub failure: Vec<combat::Effect>,
    pub ambush: Option<String>,
    pub teleport: bool,
    pub interaction: String,
}
impl Hazard {
    pub fn load(g: &GameData, props: &Props, name: &str, difficulty: u32) -> Result<Self> {
        let value = props.resolve(name, difficulty)?;
        crate::campaign017::fields(
            &value,
            &[
                "graphics_file",
                "audio_path",
                "instance_type",
                "default_animation",
                "success_effects",
                "fail_effects",
                "health",
                "torchlight",
                "ancestor_talk",
                "interaction_type",
                "teleport",
                "clear_on_failure",
                "generate_ambush",
            ],
            "expedition hazard",
        )?;
        let health = value["health"]
            .as_f64()
            .filter(|n| n.is_finite() && (-1. ..=1.).contains(n))
            .ok_or("invalid hazard HP ratio")?;
        let light = value["torchlight"]
            .as_f64()
            .filter(|n| n.is_finite() && n.fract() == 0. && (-100. ..=100.).contains(n))
            .ok_or("invalid hazard light delta")? as i32;
        let effects = |key: &str| -> Result<Vec<combat::Effect>> {
            value[key]
                .as_array()
                .ok_or_else(|| format!("missing hazard {key}"))?
                .iter()
                .map(|id| g.field_effect(id.as_str().ok_or("invalid hazard effect ID")?))
                .collect()
        };
        let success = effects("success_effects")?;
        let failure = effects("fail_effects")?;
        let boolean = |key: &str| -> Result<bool> {
            value
                .get(key)
                .map(|v| {
                    v.as_bool()
                        .ok_or_else(|| format!("invalid hazard bool {key}"))
                })
                .transpose()
                .map(|v| v.unwrap_or(false))
        };
        let ambush = value
            .get("generate_ambush")
            .map(|v| {
                v.as_str()
                    .filter(|s| *s == "trap")
                    .map(str::to_string)
                    .ok_or_else(|| "unknown hazard ambush mash selector".to_string())
            })
            .transpose()?;
        let teleport = boolean("teleport")?;
        let interaction = value["interaction_type"].as_str().unwrap_or("").to_string();
        if !["", "shovel_obstacle", "torch_obstacle"].contains(&interaction.as_str()) {
            return Err("unknown hazard interaction".into());
        }
        Ok(Self {
            id: name.into(),
            source: value,
            health,
            light,
            success,
            failure,
            ambush,
            teleport,
            interaction,
        })
    }
}

/// The ID header delimits blocks even when the optional display ordinal is absent.
pub fn campaign_type_blocks(bytes: &[u8]) -> Result<BTreeMap<String, serde_json::Value>> {
    let rows = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(bytes)
        .records()
        .map(|row| {
            row.map(|row| row.iter().map(str::to_string).collect::<Vec<_>>())
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    let starts = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| col(row, 2) == "ID STRING")
        .map(|(i, _)| i.checked_sub(1).ok_or("curio ID header lacks title row"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut starts = starts;
    starts.push(rows.len());
    let mut all = BTreeMap::new();
    for pair in starts.windows(2) {
        let block = &rows[pair[0]..pair[1]];
        let id = col(block.get(2).ok_or("short curio block")?, 2);
        if id.is_empty() {
            return Err("empty expedition curio ID".into());
        }
        crate::library::insert_json(&mut all, id.into(), serde_json::json!({"rows":block}));
    }
    Ok(all)
}

impl crate::combat::GameData {
    pub fn courtyard_walk_rules(&self) -> Result<serde_json::Value> {
        if !self
            .overlay
            .config
            .packages
            .iter()
            .any(|p| p == "crimson_court")
        {
            return Err("courtyard rules require enabled content".into());
        }
        let data = self.raid_entry("raid_rules_override_data_table", "cc")?;
        crate::campaign017::fields(
            &data,
            &["hallway_stress", "darkness"],
            "Courtyard raid rules",
        )?;
        crate::campaign017::fields(
            &data["hallway_stress"],
            &[
                "hallway_per_tile_stress_damage_fwd",
                "hallway_per_tile_stress_damage_chance_fwd",
                "hallway_per_tile_stress_damage_backing_up",
                "hallway_per_tile_stress_damage_chance_backing_up",
            ],
            "Courtyard walking rules",
        )?;
        for (k, v) in data["hallway_stress"]
            .as_object()
            .ok_or("missing Courtyard hallway rules")?
        {
            let n = v
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.)
                .ok_or("invalid Courtyard walking amount")?;
            if k.contains("chance") && n > 1. || !k.contains("chance") && n.fract() != 0. {
                return Err("invalid Courtyard walking rule domain".into());
            }
        }
        Ok(data["hallway_stress"].clone())
    }
}
