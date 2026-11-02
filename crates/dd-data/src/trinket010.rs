//! Equipment triggers and their finite budgets. Quest completion is a host phase.
use crate::{
    combat::*,
    library,
    mechanics::{Buff, read_buff},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const COMBAT_TRIGGERS: &[&str] = &[
    "attack_skill",
    "friendly_skill",
    "riposte_skill",
    "riposte_crit",
    "riposte_kill",
    "was_hit",
    "on_dodge",
    "was_killed",
    "was_killed_all_heroes",
    "kill_performer",
    "kill_all_monsters",
    "turn_end",
    "round_end",
    "battle_finished_successfully",
];
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trinket {
    pub noncombat_buffs: Vec<String>,
    pub id: String,
    pub set: Option<String>,
    pub buffs: Vec<Buff>,
    pub effects: BTreeMap<String, Vec<Effect>>,
    pub trigger_limit: Option<u32>,
    pub quest_uses: Option<u32>,
    pub destroy_on_empty: bool,
    pub blocks_trigger: bool,
    pub blocks_quest: bool,
    pub transform_empty: Option<String>,
    pub transform_death: Option<String>,
    pub quest_metadata: serde_json::Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TrinketState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub growth: Vec<Buff>,
    pub definition: Trinket,
    pub remaining: Option<u32>,
    pub maximum: Option<u32>,
    pub quest_remaining: Option<u32>,
    pub destroyed: bool,
    pub generation: u32,
}
impl TrinketState {
    pub fn fresh(definition: Trinket) -> Self {
        Self {
            growth: vec![],
            remaining: definition.trigger_limit,
            maximum: definition.trigger_limit,
            quest_remaining: definition.quest_uses,
            definition,
            destroyed: false,
            generation: 0,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.growth.len() > 10000 {
            return Err("trinket growth bound exceeded".into());
        }
        for buff in &self.growth {
            buff.validate()?;
        }
        if self.maximum.is_some() != self.definition.trigger_limit.is_some()
            || self
                .maximum
                .zip(self.definition.trigger_limit)
                .is_some_and(|(m, n)| m < n)
            || self.generation > 1000000
            || self.remaining.is_some() != self.maximum.is_some()
            || self.maximum.is_some_and(|n| n > 1000000)
            || self.remaining.zip(self.maximum).is_some_and(|(n, m)| n > m)
            || self.quest_remaining.is_some() != self.definition.quest_uses.is_some()
            || self
                .quest_remaining
                .zip(self.definition.quest_uses)
                .is_some_and(|(n, m)| n > m)
        {
            return Err("invalid trinket state/budget".into());
        }
        Ok(())
    }
    pub fn enabled(&self) -> bool {
        !self.destroyed && self.remaining != Some(0) && self.quest_remaining != Some(0)
    }
}
impl GameData {
    pub fn trinket_catalog(&self, roots: &[String]) -> Result<BTreeMap<String, Trinket>> {
        let _profile = crate::telemetry024c::scope("trinket_parse");
        let lib = library::json_library_arc(
            &self.overlay,
            "trinkets/",
            ".entries.trinkets.json",
            "entries",
        )?;
        let mut pending = roots.to_vec();
        let mut catalog = BTreeMap::new();
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            if seen.len() > 10000 {
                return Err("trinket reference graph too large".into());
            }
            let raw = library::unique_json(&lib.0, &id, "trinket")?;
            let o = raw.as_object().ok_or("invalid trinket definition")?;
            for key in o.keys() {
                if !COMBAT_TRIGGERS
                    .iter()
                    .any(|t| key == &format!("{t}_additional_effects"))
                    && ![
                        "id",
                        "buffs",
                        "hero_class_requirements",
                        "rarity",
                        "visual_rarity",
                        "price",
                        "shard",
                        "limit",
                        "origin_dungeon",
                        "set_id",
                        "trigger_limit",
                        "quest_uses",
                        "destroy_on_triggers_exhausted",
                        "blocks_other_slot_trigger_expend",
                        "blocks_other_slot_quest_uses_expend",
                        "transform_on_trigger_limit_exhausted",
                        "transform_on_wearer_killed",
                        "on_quest_complete_additional_effects",
                        "transform_on_quest_complete",
                        "transform_on_quest_complete_chance",
                        "quest_progressive_art",
                        "trigger_progressive_art",
                    ]
                    .contains(&key.as_str())
                {
                    return Err(format!("{id}: unknown trinket field {key}"));
                }
            }
            let integer = |k: &str| {
                raw.get(k)
                    .map(|v| {
                        v.as_u64()
                            .filter(|n| *n <= 1000000)
                            .map(|n| n as u32)
                            .ok_or_else(|| format!("invalid trinket integer {k}"))
                    })
                    .transpose()
            };
            let flag = |k: &str| {
                raw.get(k)
                    .map(|v| {
                        v.as_bool()
                            .ok_or_else(|| format!("invalid trinket flag {k}"))
                    })
                    .transpose()
                    .map(|v| v.unwrap_or(false))
            };
            let name = |k: &str| {
                raw.get(k)
                    .map(|v| {
                        v.as_str()
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .ok_or_else(|| format!("invalid trinket ID {k}"))
                    })
                    .transpose()
            };
            let mut buffs = vec![];
            let mut noncombat_buffs = vec![];
            for value in raw["buffs"].as_array().ok_or("missing trinket Buffs")? {
                let bid = value.as_str().ok_or("invalid trinket Buff ID")?;
                if self.noncombat_buff(bid)? {
                    noncombat_buffs.push(bid.to_owned());
                } else {
                    let mut buff = read_buff(&self.buffs, bid)?;
                    if buff.rule_type == "quest_uses_remaining_above"
                        && buff.rule_data.string.is_empty()
                    {
                        buff.rule_data.string = id.clone();
                    }
                    buffs.push(buff);
                }
            }
            let mut effects = BTreeMap::new();
            for trigger in COMBAT_TRIGGERS {
                if let Some(ids) = raw.get(format!("{trigger}_additional_effects")) {
                    let list = ids
                        .as_array()
                        .ok_or("invalid trinket trigger list")?
                        .iter()
                        .map(|id| self.effect(id.as_str().ok_or("invalid trinket effect ID")?))
                        .collect::<Result<Vec<_>>>()?;
                    effects.insert(trigger.to_string(), list);
                }
            }
            let transform_empty = name("transform_on_trigger_limit_exhausted")?;
            let transform_death = name("transform_on_wearer_killed")?;
            pending.extend(transform_empty.iter().chain(&transform_death).cloned());
            fn references(e: &Effect, refs: &mut Vec<String>) {
                match &e.kind {
                    EffectKind::TrinketOp(TrinketOp::Gain(id)) => refs.push(id.clone()),
                    EffectKind::InventoryEffect(inner) => {
                        let mut e = e.clone();
                        e.kind = *inner.clone();
                        references(&e, refs);
                    }
                    EffectKind::Composite(es) => {
                        for e in es {
                            references(e, refs)
                        }
                    }
                    _ => {}
                }
            }
            for e in effects.values().flatten() {
                references(e, &mut pending);
            }
            let mut quest = serde_json::Map::new();
            for key in [
                "on_quest_complete_additional_effects",
                "transform_on_quest_complete",
                "transform_on_quest_complete_chance",
                "quest_progressive_art",
                "trigger_progressive_art",
            ] {
                if let Some(value) = raw.get(key) {
                    quest.insert(key.to_owned(), value.clone());
                }
            }
            let definition = Trinket {
                id: id.clone(),
                noncombat_buffs,
                set: name("set_id")?,
                buffs,
                effects,
                trigger_limit: integer("trigger_limit")?,
                quest_uses: integer("quest_uses")?,
                destroy_on_empty: flag("destroy_on_triggers_exhausted")?,
                blocks_trigger: flag("blocks_other_slot_trigger_expend")?,
                blocks_quest: flag("blocks_other_slot_quest_uses_expend")?,
                transform_empty,
                transform_death,
                quest_metadata: serde_json::Value::Object(quest),
            };
            if definition.destroy_on_empty && definition.transform_empty.is_some() {
                return Err("ambiguous trinket destruction/transform".into());
            }
            catalog.insert(id, definition);
        }
        Ok(catalog)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrinketOp {
    Destroy,
    Gain(String),
    Increase { min: u32, max: u32 },
}
