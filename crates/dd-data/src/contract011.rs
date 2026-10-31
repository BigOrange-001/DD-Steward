//! Explicit battle/raid handoff flags, without executing quest or campaign progression.
use crate::combat::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub id: String,
    pub source: String,
    pub can_retreat: bool,
    pub raid_retreat_deaths: u32,
    pub retreat_always_from_raid: bool,
    pub death_override: Option<String>,
}
impl Contract {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || self.source.is_empty()
            || self.raid_retreat_deaths > 4
            || self
                .death_override
                .as_ref()
                .is_some_and(|s| s != "in_time_and_space")
        {
            return Err("invalid encounter battle contract".into());
        }
        Ok(())
    }
}
impl GameData {
    pub fn battle_contract(&self, id: &str) -> Result<Contract> {
        let path = if id.starts_with("plot_color_of_madness_") {
            "campaign/quest/com.quest.plot_quests.json"
        } else if id.starts_with("plot_darkest_dungeon_") {
            "campaign/quest/quest.plot_quests.json"
        } else {
            return Err("unknown special battle contract".into());
        };
        let mut found = vec![];
        for index in self.overlay.sources(path) {
            let file = self.overlay.layers[index].root.join(path);
            let raw = std::fs::read(&file).map_err(|e| e.to_string())?;
            let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
            for row in v["plot_quests"]
                .as_array()
                .ok_or("missing special battle contract library")?
            {
                if row["id"] != id {
                    continue;
                }
                let death_override = match row.get("death_override") {
                    None => None,
                    Some(v) => {
                        if v.as_object().is_none_or(|m| m.len() != 1) {
                            return Err("unsupported death override fields".into());
                        }
                        Some(v["status"].as_str().ok_or("unknown death override")?.into())
                    }
                };
                let c = Contract {
                    id: id.into(),
                    source: path.into(),
                    can_retreat: row["can_retreat"]
                        .as_bool()
                        .ok_or("missing retreat permission")?,
                    raid_retreat_deaths: row["retreat_party_kill_count"]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or("invalid raid retreat death count")?,
                    retreat_always_from_raid: row["retreat_always_from_raid"]
                        .as_bool()
                        .ok_or("missing raid retreat boundary")?,
                    death_override,
                };
                c.validate()?;
                if !found.contains(&c) {
                    found.push(c);
                }
            }
        }
        if found.len() != 1 {
            return Err("missing or ambiguous encounter contract".into());
        }
        Ok(found.remove(0))
    }
}
