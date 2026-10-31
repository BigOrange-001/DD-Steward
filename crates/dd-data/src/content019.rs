//! Additional typed campaign inputs. Loading these is not an execution proof.
use crate::{
    combat::{GameData, Result},
    town016,
};
use serde::{Deserialize, Serialize};
pub fn provision_additions(
    game: &GameData,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>> {
    let mut sources = std::collections::BTreeMap::new();
    for path in game.overlay.paths.keys().filter(|p| {
        p.starts_with("campaign/provision/")
            && p.ends_with(".json")
            && p.as_str() != "campaign/provision/provision.json"
    }) {
        let value: serde_json::Value = serde_json::from_slice(&game.overlay.read_content019(path)?)
            .map_err(|e| e.to_string())?;
        crate::campaign017::fields(
            &value,
            &[
                "raid_starting_length_inventory_item_lists",
                "default_store_inventory_item_lists",
                "raid_starting_hero_class_item_lists",
            ],
            "019 additive provision source",
        )?;
        for (key, raw) in value.as_object().ok_or("provision source is not object")? {
            let rows = raw.as_array().ok_or("provision additions are not rows")?;
            for row in rows {
                let items = if key == "raid_starting_hero_class_item_lists" {
                    crate::campaign017::fields(
                        row,
                        &["hero_class", "item_lists"],
                        "019 class grant",
                    )?;
                    if !game
                        .hero_ids()
                        .any(|id| Some(id.as_str()) == row["hero_class"].as_str())
                    {
                        return Err("019 grant references unknown/disabled class".into());
                    }
                    row["item_lists"]
                        .as_array()
                        .ok_or("019 class grant items absent")?
                } else {
                    row.as_array().ok_or("019 length provision items absent")?
                };
                for item in items {
                    crate::campaign017::fields(
                        item,
                        &["type", "id", "amount"],
                        "019 provision item",
                    )?;
                    crate::campaign017::uint(item, "amount")?;
                    if item["type"].as_str().is_none() || item["id"].as_str().is_none() {
                        return Err("019 provision item type/ID absent".into());
                    }
                }
            }
        }
        sources.insert(path.clone(), value);
    }
    Ok(sources)
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LostRules {
    pub in_time_and_space_missing_duration: u32,
    pub in_time_and_space_stress: f64,
    pub in_time_and_space_affliction_id: String,
    pub in_time_and_space_affliction_severity: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MercenaryStore {
    pub required_completed_plot_quest: String,
    pub number_of_recruits: u32,
    pub starting_quirks: Vec<String>,
    pub minimum_number_of_positive_quirks: u32,
    pub minimum_number_of_negative_quirks: u32,
    pub starting_resolve_level: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShardStore {
    pub comet_trinkets_to_display: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FarmSources {
    pub lost: LostRules,
    pub mercenaries: MercenaryStore,
    pub shop: ShardStore,
}
impl FarmSources {
    pub fn load(game: &GameData, town: &town016::Data) -> Result<Option<Self>> {
        if town.content != game.overlay.config {
            return Err("content019 source configuration mismatch".into());
        }
        if !game
            .overlay
            .config
            .packages
            .iter()
            .any(|p| p == "color_of_madness")
        {
            return Ok(None);
        }
        let value = Self {
            lost: serde_json::from_slice(&game.overlay.read_content019("shared/com.rules.json")?)
                .map_err(|e| e.to_string())?,
            mercenaries: serde_json::from_value(
                town.store("stage_coach.shard_hero_recruit")?.clone(),
            )
            .map_err(|e| e.to_string())?,
            shop: serde_json::from_value(town.store("nomad_wagon.shard_trinket_supply")?.clone())
                .map_err(|e| e.to_string())?,
        };
        game.validate_farm_fireplace019()?;
        crate::curios013::Curios::load_campaign(&game.overlay)?.farm_fireplace019()?;
        value.validate()?;
        Ok(Some(value))
    }
    pub fn validate(&self) -> Result<()> {
        let l = &self.lost;
        let m = &self.mercenaries;
        if !l.in_time_and_space_stress.is_finite()
            || !(0. ..=200.).contains(&l.in_time_and_space_stress)
            || l.in_time_and_space_affliction_id.is_empty()
            || l.in_time_and_space_stress.fract() != 0.
            || l.in_time_and_space_affliction_severity != 1
            || m.required_completed_plot_quest.is_empty()
            || m.starting_quirks.iter().any(String::is_empty)
            || m.starting_quirks
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != m.starting_quirks.len()
        {
            return Err("invalid content019 farm identity/value".into());
        }
        // Resource limits, not game tuning. All actual amounts come from input.
        if [
            l.in_time_and_space_missing_duration,
            l.in_time_and_space_affliction_severity,
            m.number_of_recruits,
            m.minimum_number_of_positive_quirks,
            m.minimum_number_of_negative_quirks,
            m.starting_resolve_level,
            self.shop.comet_trinkets_to_display,
        ]
        .iter()
        .any(|n| *n > 10000)
        {
            return Err("content019 source resource limit".into());
        }
        Ok(())
    }
}
impl GameData {
    pub fn validate_farm_start019(&self) -> Result<()> {
        let r = self.effect_source("start_endless")?;
        if r.params.iter().any(|p| {
            ![
                "name",
                "target",
                "chance",
                "skips_endless_wave_curio",
                "on_hit",
                "on_miss",
                "queue",
                "curio_result_type",
            ]
            .contains(&p.name.as_str())
        }) || crate::combat::text(r, "target")? != "global"
            || crate::combat::number(r, "chance")? != 1.
            || crate::combat::text(r, "skips_endless_wave_curio")? != "true"
            || crate::combat::text(r, "on_hit")? != "true"
            || crate::combat::text(r, "on_miss")? != "false"
            || crate::combat::text(r, "queue")? != "true"
            || crate::combat::text(r, "curio_result_type")? != "none"
        {
            return Err("unknown farm start source mechanism".into());
        }
        Ok(())
    }
    pub fn validate_farm_fireplace019(&self) -> Result<()> {
        let r = self.effect_source("use_item_firewood")?;
        if r.params.iter().any(|p| {
            ![
                "name",
                "target",
                "use_item_type",
                "use_item_id",
                "chance",
                "on_hit",
                "on_miss",
                "queue",
            ]
            .contains(&p.name.as_str())
        }) || crate::combat::text(r, "target")? != "target"
            || crate::combat::text(r, "use_item_type")? != "supply"
            || crate::combat::text(r, "use_item_id")? != "firewood"
            || crate::combat::number(r, "chance")? != 1.
            || crate::combat::text(r, "on_hit")? != "true"
            || crate::combat::text(r, "on_miss")? != "false"
            || crate::combat::text(r, "queue")? != "true"
        {
            return Err("unknown enabled farm fireplace host effect".into());
        }
        Ok(())
    }
}
impl crate::curios013::Curios {
    pub fn validate_farm_stone019(&self) -> Result<()> {
        let rows = self.source_rows("com_standingstone")?;
        let mut effects = 0;
        for row in rows
            .iter()
            .skip(2)
            .take_while(|r| r.get(2).map(String::as_str) != Some("Item Interactions"))
        {
            if row.get(5).is_some_and(|s| !s.is_empty()) {
                if row.get(4).map(String::as_str) != Some("Effect")
                    || row.get(5).map(String::as_str) != Some("1")
                    || row.get(7).map(String::as_str) != Some("start_endless")
                    || row.iter().skip(8).take(8).any(|s| !s.is_empty())
                {
                    return Err("unknown farm standing stone source".into());
                }
                effects += 1;
            }
        }
        if effects != 1 {
            return Err("missing farm standing stone effect".into());
        }
        let full = rows
            .iter()
            .position(|r| r.get(2).map(String::as_str) == Some("FULL CURIO?"))
            .and_then(|i| rows.get(i + 1))
            .and_then(|r| r.get(2))
            .map(String::as_str);
        let items = rows
            .iter()
            .position(|r| r.get(2).map(String::as_str) == Some("Item Interactions"))
            .ok_or("farm stone item header absent")?;
        if full != Some("Yes")
            || rows
                .iter()
                .skip(items + 1)
                .any(|r| r.get(4).is_some_and(|s| !s.is_empty()))
        {
            return Err("unknown farm stone auto/item behavior".into());
        }
        Ok(())
    }
    pub fn farm_fireplace019(&self) -> Result<crate::field017::Curio> {
        let mut rows = self.source_rows("com_fireplace")?;
        let mut count = 0;
        for row in &mut rows {
            if row.get(4).is_some_and(|v| v == "Effect") && row.get(5).is_some_and(|v| v == "1") {
                if row.get(7).map(String::as_str) != Some("use_item_firewood")
                    || row.iter().skip(8).take(8).any(|v| !v.is_empty())
                {
                    return Err("unknown farm fireplace curio source".into());
                }
                // This exact source has one effect but omits its inner weight.
                // The explicit 019 candidate selects that sole effect.
                row[8] = "1".into();
                count += 1;
            }
        }
        if count != 1 {
            return Err("missing/ambiguous farm fireplace source".into());
        }
        let (full, outcomes, items) =
            crate::curios013::decode_curio_rows("com_fireplace", &rows, |kind, row| {
                crate::curios013::parse_outcome(kind, row).map(crate::field017::ResultKind::Basic)
            })?;
        if !full || outcomes.len() != 1 || !items.is_empty() {
            return Err("unknown farm fireplace outcomes or item behavior".into());
        }
        Ok(crate::field017::Curio {
            id: "com_fireplace".into(),
            full,
            outcomes,
            items,
        })
    }
}
