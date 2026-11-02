//! Original hallway pools, including conditions which are evaluated by the opening host.
use crate::{
    combat::{Blueprint, GameData, Result, number, text},
    curios013::{Curios, Draw},
    library,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub raid_finish_quirks: BTreeMap<String, Vec<String>>,
    pub id: String,
    pub weight: f64,
    pub conditional: bool,
    pub additional: bool,
    pub monsters: Vec<String>,
    pub conditions: BTreeMap<String, Vec<String>>,
    pub loot: Vec<Draw>,
    pub eligible_in_opening: bool,
    pub exclusion: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Pools {
    pub source_notes: Vec<String>,
    pub entries: Vec<Entry>,
    pub catalog: BTreeMap<String, Blueprint>,
}
pub fn load(g: &GameData, region: &str, difficulty: u32, curios: &Curios) -> Result<Pools> {
    load_context(g, region, difficulty, curios, false)
}
pub fn campaign_load(
    g: &GameData,
    region: &str,
    difficulty: u32,
    curios: &Curios,
) -> Result<Pools> {
    load_context(g, region, difficulty, curios, true)
}
fn load_context(
    g: &GameData,
    region: &str,
    difficulty: u32,
    curios: &Curios,
    campaign: bool,
) -> Result<Pools> {
    let suffix = format!(".{difficulty}.mash.darkest");
    let mut entries = vec![];
    let mut source_notes = vec![];
    let mut roots = vec![];
    for path in g
        .overlay
        .combat_files(&format!("dungeons/{region}/"), &suffix)
    {
        let d = library::read_document(&g.overlay, &path)?;
        for r in d
            .records
            .iter()
            .filter(|r| r.key == "hall" || campaign && ["room", "trap"].contains(&r.key.as_str()))
        {
            let allowed = [
                "types",
                "chance",
                "inventory_valid_item_percent_range",
                "darkness_range",
                "limit",
                "can_be_ambush",
                "completed_flashback_id",
                "plot_quest_requirement",
                "random_dungeon_roaming_id",
                "infestation_activity_level",
                "min_vampire_heroes",
                "max_vampire_heroes",
                "loading_screen",
                "background",
            ];
            if r.params.iter().any(|p| !allowed.contains(&p.name.as_str())) {
                return Err(format!(
                    "{path}:{} unknown hallway encounter condition",
                    r.line
                ));
            }
            let weight = number(r, "chance")?;
            if weight < 0. || !weight.is_finite() {
                return Err("invalid hallway weight".into());
            }
            let ids = r
                .param("types")
                .ok_or("missing hallway formation")?
                .values
                .iter()
                .map(|v| v.text.clone())
                .collect::<Vec<_>>();
            if ids.is_empty() {
                return Err("empty hallway formation".into());
            }
            let mut conditions = BTreeMap::new();
            for p in &r.params {
                if !["types", "chance", "background", "loading_screen"].contains(&p.name.as_str()) {
                    conditions.insert(
                        p.name.clone(),
                        p.values.iter().map(|v| v.text.clone()).collect::<Vec<_>>(),
                    );
                }
            }
            for (k, v) in &conditions {
                match k.as_str() {
                    "darkness_range" | "inventory_valid_item_percent_range" => {
                        if v.len() != 2 {
                            return Err("invalid encounter interval".into());
                        }
                        let a = v[0].parse::<f64>().map_err(|_| "bad encounter interval")?;
                        let b = v[1].parse::<f64>().map_err(|_| "bad encounter interval")?;
                        if !a.is_finite()
                            || !b.is_finite()
                            || a > b
                            || a < 0.
                            || b > if k == "darkness_range" { 100. } else { 1. }
                        {
                            return Err("invalid encounter range".into());
                        }
                    }
                    "can_be_ambush" => {
                        if v.len() != 1 || !["true", "false"].contains(&v[0].as_str()) {
                            return Err("bad encounter bool".into());
                        }
                    }
                    "limit" | "min_vampire_heroes" | "max_vampire_heroes" => {
                        if v.len() != 1 || v[0].parse::<u32>().is_err() {
                            return Err("bad encounter count".into());
                        }
                    }
                    "completed_flashback_id"
                    | "plot_quest_requirement"
                    | "random_dungeon_roaming_id"
                    | "infestation_activity_level" => {
                        if v.len() != 1 || v[0].is_empty() {
                            return Err("bad encounter context id".into());
                        }
                    }
                    _ => return Err("unknown opening encounter condition".into()),
                }
            }
            // This fixed opening starts before Court/Com quests, infestation and roaming activation.
            // Keep excluded records and their explicit reason instead of deleting their weights.
            let exclusion = if campaign {
                None
            } else {
                conditions
                    .keys()
                    .find(|k| {
                        [
                            "completed_flashback_id",
                            "plot_quest_requirement",
                            "random_dungeon_roaming_id",
                            "infestation_activity_level",
                        ]
                        .contains(&k.as_str())
                    })
                    .map(|k| format!("opening context has no active {k}"))
            };
            let eligible = weight > 0. && exclusion.is_none();
            let mut loot = vec![];
            let mut raid_finish_quirks = BTreeMap::<String, Vec<String>>::new();
            if eligible {
                roots.extend(ids.clone());
                for id in &ids {
                    if campaign {
                        let incoming = campaign_monster_loot(g, id, difficulty, region, curios)?;
                        loot.extend(incoming.draws);
                        if !incoming.raid_finish_quirks.is_empty() {
                            raid_finish_quirks.insert(id.clone(), incoming.raid_finish_quirks);
                        }
                        source_notes.extend(incoming.notes);
                        continue;
                    }
                    let suffix = format!("/{id}/{id}.info.darkest");
                    let path = g
                        .overlay
                        .combat_files("monsters/", ".info.darkest")
                        .into_iter()
                        .find(|p| p.ends_with(&suffix))
                        .ok_or("missing hallway monster source")?;
                    let d = library::read_document(&g.overlay, &path)?;
                    for r in d.records_keyed("loot") {
                        let code = text(r, "code").map_err(|e| format!("{id} {path}: {e}"))?;
                        if code == "NONE" && r.param("count").is_some_and(|p| p.values.is_empty()) {
                            source_notes.push(format!("{id}: explicit NONE loot has empty count; candidate inactive record"));
                            continue;
                        }

                        let count = number(r, "count").map_err(|e| format!("{id} {path}: {e}"))?;
                        if count.fract() != 0. || !(0. ..=1000.).contains(&count) {
                            return Err("invalid hallway loot count".into());
                        }
                        curios.validate_draw(code, i64::from(difficulty), region, &mut vec![])?;
                        loot.push(Draw {
                            table: code.into(),
                            count: count as u32,
                        });
                    }
                }
            }
            entries.push(Entry {
                area: campaign.then(|| r.key.clone()),
                raid_finish_quirks,
                id: format!("{path}:{}", r.line),
                weight,
                conditional: path.contains(".conditional."),
                additional: path.contains(".additional."),
                monsters: ids,
                conditions,
                loot,
                eligible_in_opening: eligible,
                exclusion,
            });
        }
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    roots.sort();
    roots.dedup();
    let refs = roots.iter().map(String::as_str).collect::<Vec<_>>();
    let catalog = g.entity_catalog(&refs)?;
    Ok(Pools {
        entries,
        catalog,
        source_notes,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MonsterLoot {
    pub draws: Vec<Draw>,
    pub raid_finish_quirks: Vec<String>,
    pub notes: Vec<String>,
}
/// Wave encounters pay their source-valued shard/threshold/curio rewards, not
/// ordinary map-battle loot. Keep independent raid-finish quirk obligations.
pub fn wave_monster_rewards023(g: &GameData, id: &str) -> Result<MonsterLoot> {
    if !crate::mechanisms023::enabled(&g.overlay.config, crate::mechanisms023::FIXES) {
        return Err("wave reward protocol requires task 023 identity".into());
    }
    let suffix = format!("/{id}/{id}.info.darkest");
    let path = g
        .overlay
        .combat_files("monsters/", ".info.darkest")
        .into_iter()
        .find(|p| p.ends_with(&suffix))
        .ok_or("missing wave monster reward source")?;
    let d = library::read_document(&g.overlay, &path)?;
    let mut result = MonsterLoot {
        draws: vec![],
        raid_finish_quirks: vec![],
        notes: vec![],
    };
    for row in d.records_keyed("loot") {
        if row.param("raid_finish_quirk_class_id").is_some() {
            if row.params.len() != 1 {
                return Err("mixed wave raid-finish quirk record".into());
            }
            let quirk = text(row, "raid_finish_quirk_class_id")?;
            library::unique_json(&g.quirks()?, quirk, "wave raid-finish quirk")?;
            result.raid_finish_quirks.push(quirk.into());
        } else {
            if row
                .params
                .iter()
                .any(|p| !["code", "count"].contains(&p.name.as_str()))
            {
                return Err("unknown wave monster loot field".into());
            }
            let code = text(row, "code")?;
            if (code.is_empty() || code == "NONE")
                && row.param("count").is_some_and(|p| p.values.is_empty())
            {
                continue;
            }
            let count = number(row, "count")?;
            if count.fract() != 0. || !(0. ..=1000.).contains(&count) {
                return Err("invalid wave ordinary loot metadata".into());
            }
            result.notes.push(format!(
                "{id}: ordinary map loot {code} x{count} does not apply to wave reward protocol"
            ));
        }
    }
    Ok(result)
}
pub fn campaign_monster_loot(
    g: &GameData,
    id: &str,
    difficulty: u32,
    region: &str,
    curios: &Curios,
) -> Result<MonsterLoot> {
    let suffix = format!("/{id}/{id}.info.darkest");
    let path = g
        .overlay
        .combat_files("monsters/", ".info.darkest")
        .into_iter()
        .find(|p| p.ends_with(&suffix))
        .ok_or("missing campaign monster loot source")?;
    let document = library::read_document(&g.overlay, &path)?;
    let mut result = MonsterLoot {
        draws: vec![],
        raid_finish_quirks: vec![],
        notes: vec![],
    };
    for row in document.records_keyed("loot") {
        if row.param("raid_finish_quirk_class_id").is_some() {
            if row.params.len() != 1 {
                return Err(format!("{id}: mixed raid-finish quirk loot fields"));
            }
            let quirk = text(row, "raid_finish_quirk_class_id")?;
            library::unique_json(&g.quirks()?, quirk, "monster raid-finish quirk")?;
            result.raid_finish_quirks.push(quirk.into());
            continue;
        }
        if row
            .params
            .iter()
            .any(|p| !["code", "count"].contains(&p.name.as_str()))
        {
            return Err(format!("{id}: unknown campaign monster loot field"));
        }
        let code = text(row, "code").map_err(|e| format!("{id} {path}: {e}"))?;
        if (code.is_empty() || code == "NONE")
            && row.param("count").is_some_and(|p| p.values.is_empty())
        {
            result.notes.push(format!("{id}: explicit empty/NONE loot with empty count; campaign candidate inactive record"));
            continue;
        }
        let count = number(row, "count").map_err(|e| format!("{id} {path}: {e}"))?;
        if count.fract() != 0. || !(0. ..=1000.).contains(&count) {
            return Err("invalid campaign monster loot count".into());
        }
        curios.validate_campaign_draw(code, i64::from(difficulty), region, &mut vec![])?;
        result.draws.push(Draw {
            table: code.into(),
            count: count as u32,
        });
    }
    Ok(result)
}
