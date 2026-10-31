//! Battle formations only: no map, quest generation or campaign state.
use crate::{combat::*, library};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub id: String,
    pub region: String,
    pub sources: Vec<(String, u32)>,
    pub kind: String,
    pub monsters: Vec<String>,
    pub background: Option<String>,
    pub flashback: Option<String>,
    pub conditions: Vec<(String, u32, BTreeMap<String, Vec<String>>)>,
}
impl GameData {
    pub fn boss_encounters(&self) -> Result<Vec<Encounter>> {
        let mut bosses = BTreeSet::new();
        for path in self.overlay.combat_files("monsters/", ".info.darkest") {
            let d = library::read_document(&self.overlay, &path)?;
            if d.records_keyed("tag")
                .any(|r| text(r, "id").ok() == Some("boss"))
            {
                bosses.insert(
                    path.split('/')
                        .nth(2)
                        .ok_or("invalid monster path")?
                        .to_string(),
                );
            }
        }
        let mut found = BTreeMap::<(String, Vec<String>), Encounter>::new();
        let mut paths = self.overlay.combat_files("dungeons/", ".mash.darkest");
        paths.extend(self.overlay.combat_files("dungeons/", ".wavemash.darkest"));
        paths.sort();
        for path in paths {
            let d = library::read_document(&self.overlay, &path)?;
            for r in &d.records {
                let ids = r
                    .param("types")
                    .map(|p| p.values.iter().map(|v| v.text.clone()).collect::<Vec<_>>())
                    .unwrap_or_default();
                let nightmare = path.contains("flashback")
                    && r.key == "named"
                    && r.param("flashback_id").is_some();
                if !["boss", "miniboss", "localminiboss"].contains(&r.key.as_str())
                    && !(["named", "room", "hall", "mash"].contains(&r.key.as_str())
                        && ids
                            .iter()
                            .any(|id| bosses.contains(id) || id == "cell_white_D"))
                    && !nightmare
                {
                    continue;
                }
                if ids.is_empty() {
                    return Err(format!("{path}:{} empty boss formation", r.line));
                }
                if r.params.iter().any(|p| {
                    ![
                        "types",
                        "chance",
                        "name",
                        "background",
                        "flashback_id",
                        "inventory_valid_item_percent_range",
                        "darkness_range",
                        "limit",
                        "can_be_ambush",
                        "plot_quest_requirement",
                        "random_dungeon_roaming_id",
                        "infestation_activity_level",
                        "min_vampire_heroes",
                        "max_vampire_heroes",
                        "loading_screen",
                    ]
                    .contains(&p.name.as_str())
                }) {
                    return Err(format!("{path}:{} unsupported encounter field", r.line));
                }
                for p in &r.params {
                    match p.name.as_str() {
                        "inventory_valid_item_percent_range" | "darkness_range" => {
                            if p.values.len() != 2 {
                                return Err("invalid inventory encounter condition".into());
                            }
                            let values = p
                                .values
                                .iter()
                                .map(|v| scalar(&v.text))
                                .collect::<Result<Vec<_>>>()?;
                            if values[0] > values[1]
                                || values.iter().any(|v| {
                                    !(0. ..=if p.name == "darkness_range" { 100. } else { 1. })
                                        .contains(v)
                                })
                            {
                                return Err("invalid inventory encounter range".into());
                            }
                        }
                        "limit" | "min_vampire_heroes" | "max_vampire_heroes" => {
                            let n = number(r, &p.name)?;
                            if n.fract() != 0. || n < 0. || n > 10000. {
                                return Err("invalid encounter count condition".into());
                            }
                        }
                        "can_be_ambush" => {
                            flag(r, &p.name, false)?;
                        }
                        "infestation_activity_level" => {
                            if !["low", "medium", "high"].contains(&text(r, &p.name)?) {
                                return Err("unknown infestation condition".into());
                            }
                        }
                        "plot_quest_requirement"
                        | "random_dungeon_roaming_id"
                        | "loading_screen"
                            if text(r, &p.name)?.is_empty() =>
                        {
                            return Err("empty encounter metadata ID".into());
                        }
                        _ => {}
                    }
                }
                if r.param("chance").is_some() && number(r, "chance")? < 0. {
                    return Err("negative encounter chance".into());
                }
                let kind = if nightmare {
                    text(r, "flashback_id")?
                } else if path.ends_with(".wavemash.darkest") {
                    match r.key.as_str() {
                        "boss" => "wave_boss",
                        "miniboss" => "wave_miniboss",
                        _ => "wave_localminiboss",
                    }
                } else {
                    r.key.as_str()
                }
                .to_string();
                let entry = found
                    .entry((kind.clone(), ids.clone()))
                    .or_insert_with(|| Encounter {
                        id: format!("{}:{}", ids.join("+"), kind),
                        region: path.split('/').nth(1).unwrap_or("").into(),
                        sources: vec![],
                        kind,
                        monsters: ids,
                        flashback: r
                            .param("flashback_id")
                            .and_then(|p| p.values.first())
                            .map(|v| v.text.clone()),
                        conditions: vec![],
                        background: r
                            .param("background")
                            .and_then(|p| p.values.first())
                            .map(|v| v.text.clone()),
                    });
                entry.sources.push((path.clone(), r.line));
                entry.conditions.push((
                    path.clone(),
                    r.line,
                    r.params
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                p.values.iter().map(|v| v.text.clone()).collect(),
                            )
                        })
                        .collect(),
                ));
            }
        }
        Ok(found.into_values().collect())
    }
}
