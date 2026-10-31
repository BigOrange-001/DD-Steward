use crate::{
    combat::*,
    mechanics::{Buff, read_buff},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Flashback {
    pub id: String,
    pub source: String,
    pub required: Vec<String>,
    pub allow_virtue: bool,
    pub starting: Vec<Effect>,
    pub min_health: f64,
    pub results: Vec<FlashResult>,
    pub completed_camping_buffs: Vec<Buff>,
    pub trigger: Trigger,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    pub prerequisite: String,
    pub chance: f64,
    pub raid_limit: u32,
    pub complete_limit: u32,
    pub display: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlashResult {
    pub success: bool,
    pub target: String,
    pub operation: FlashOp,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum FlashOp {
    HealthFloor(f64),
    StressCap(i32),
    Buff(Buff),
    Reward { kind: String, id: String },
}
impl Flashback {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || self.required.is_empty()
            || !(0. ..=1.).contains(&self.min_health)
            || self.source.is_empty()
        {
            return Err("invalid flashback contract".into());
        }
        if !(0. ..=1.).contains(&self.trigger.chance)
            || self.trigger.raid_limit == 0
            || self.trigger.complete_limit == 0
        {
            return Err("invalid flashback trigger metadata".into());
        }
        for b in &self.completed_camping_buffs {
            b.validate()?;
        }
        for r in &self.results {
            if !["raid", "hero", "required_hero", "not_required_hero"].contains(&r.target.as_str())
            {
                return Err("unknown flashback result target".into());
            }
            match &r.operation {
                FlashOp::HealthFloor(p) if !(0. ..=1.).contains(p) => {
                    return Err("invalid flashback health floor".into());
                }
                FlashOp::StressCap(n) if !(0..200).contains(n) => {
                    return Err("invalid flashback stress cap".into());
                }
                FlashOp::Buff(b) => b.validate()?,
                FlashOp::Reward { kind, id }
                    if !["trinket", "loot"].contains(&kind.as_str()) || id.is_empty() =>
                {
                    return Err("unknown flashback reward".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}
impl GameData {
    pub fn flashback(&self, id: &str) -> Result<Flashback> {
        let path = "raid/flashback/shieldbreaker.flashbacks.json";
        let raw: serde_json::Value =
            serde_json::from_slice(&self.overlay.read(path)?).map_err(|e| e.to_string())?;
        let entries = raw["flashbacks"]
            .as_array()
            .ok_or("missing flashback library")?;
        let defs = entries.iter().filter(|e| e["id"] == id).collect::<Vec<_>>();
        if defs.len() != 1 {
            return Err("missing or ambiguous flashback".into());
        }
        self.parse_flashback(id, defs[0], path)
    }
    pub fn flashback_catalog(&self) -> Result<std::collections::BTreeMap<String, Flashback>> {
        if !self
            .overlay
            .config
            .packages
            .iter()
            .any(|p| p == "shieldbreaker")
        {
            return Err("flashback catalog requires enabled shieldbreaker".into());
        }
        let path = "raid/flashback/shieldbreaker.flashbacks.json";
        let mut rows = std::collections::BTreeMap::new();
        for bytes in self.overlay.read_versions(path)? {
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            crate::campaign017::fields(&value, &["flashbacks"], "flashback library")?;
            for row in value["flashbacks"]
                .as_array()
                .ok_or("missing flashback catalog entries")?
            {
                let id = row["id"].as_str().ok_or("missing flashback catalog ID")?;
                crate::library::insert_json(&mut rows, id.into(), row.clone());
            }
        }
        let mut result = std::collections::BTreeMap::new();
        for id in rows.keys() {
            let row = crate::library::unique_json(&rows, id, "flashback")?;
            result.insert(id.clone(), self.parse_flashback(id, row, path)?);
        }
        for f in result.values() {
            if !f.trigger.prerequisite.is_empty() && !result.contains_key(&f.trigger.prerequisite) {
                return Err("missing flashback prerequisite".into());
            }
        }
        Ok(result)
    }
    fn parse_flashback(&self, id: &str, e: &serde_json::Value, path: &str) -> Result<Flashback> {
        let known = [
            "id",
            "prerequisite",
            "display",
            "chance",
            "start_type",
            "torch_settings",
            "raid_limit",
            "campaign_complete_limit",
            "required_hero_class_ids",
            "required_heroes_can_be_virtued",
            "required_heroes_starting_effects",
            "required_heroes_starting_min_health_percent",
            "failures",
            "results",
            "completed_camping_start_buffs",
        ];
        if e.as_object()
            .unwrap()
            .keys()
            .any(|k| !known.contains(&k.as_str()))
        {
            return Err("unknown flashback field".into());
        }
        if e["torch_settings"] != "flashback" || e["start_type"] != "camp_finish" {
            return Err("unsupported flashback entry mode".into());
        }
        let strings = |key: &str| -> Result<Vec<String>> {
            e[key]
                .as_array()
                .ok_or("missing flashback list")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or("invalid flashback ID".into())
                })
                .collect()
        };
        let failures = e["failures"]
            .as_array()
            .ok_or("missing flashback failures")?;
        if failures.iter().any(|f| {
            f.as_object().is_none_or(|m| m.len() != 2)
                || f["type"] != "deaths_door"
                || f["target_type"] != "required_hero"
        }) {
            return Err("unsupported flashback failure".into());
        }
        let mut results = vec![];
        for r in e["results"].as_array().ok_or("missing flashback results")? {
            if r.as_object().is_none_or(|m| {
                m.keys().any(|k| {
                    ![
                        "is_on_complete",
                        "target_type",
                        "type",
                        "string_data",
                        "number_data",
                    ]
                    .contains(&k.as_str())
                })
            }) {
                return Err("unknown flashback result field".into());
            }
            let number = r["number_data"]
                .as_f64()
                .ok_or("invalid flashback result number")?;
            let word = r["string_data"]
                .as_str()
                .ok_or("invalid flashback result ID")?;
            let operation = match r["type"].as_str().ok_or("missing flashback result type")? {
                "min_health_percent" => FlashOp::HealthFloor(number),
                "max_stress" if number.fract() == 0. => FlashOp::StressCap(number as i32),
                "buff" => {
                    let mut b = read_buff(&self.buffs, word)?;
                    b.duration_type = Some("quest_end".into());
                    FlashOp::Buff(b)
                }
                kind @ ("loot" | "trinket") => FlashOp::Reward {
                    kind: kind.into(),
                    id: word.into(),
                },
                _ => return Err("unsupported flashback result operation".into()),
            };
            results.push(FlashResult {
                success: r["is_on_complete"]
                    .as_bool()
                    .ok_or("invalid flashback result condition")?,
                target: r["target_type"]
                    .as_str()
                    .ok_or("missing flashback target")?
                    .into(),
                operation,
            });
        }
        let definition = Flashback {
            id: id.into(),
            source: path.into(),
            required: strings("required_hero_class_ids")?,
            allow_virtue: e["required_heroes_can_be_virtued"]
                .as_bool()
                .ok_or("invalid flashback virtue permission")?,
            starting: strings("required_heroes_starting_effects")?
                .iter()
                .map(|id| self.effect(id))
                .collect::<Result<Vec<_>>>()?,
            min_health: e["required_heroes_starting_min_health_percent"]
                .as_f64()
                .ok_or("invalid flashback health")?,
            results,
            completed_camping_buffs: strings("completed_camping_start_buffs")?
                .iter()
                .map(|id| read_buff(&self.buffs, id))
                .collect::<Result<Vec<_>>>()?,
            trigger: Trigger {
                prerequisite: e["prerequisite"]
                    .as_str()
                    .ok_or("missing flashback prerequisite")?
                    .into(),
                chance: e["chance"].as_f64().ok_or("invalid flashback chance")?,
                raid_limit: e["raid_limit"]
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("invalid flashback raid limit")?,
                complete_limit: e["campaign_complete_limit"]
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("invalid flashback complete limit")?,
                display: e["display"]
                    .as_str()
                    .ok_or("missing flashback display")?
                    .into(),
            },
        };
        definition.validate()?;
        Ok(definition)
    }
}
