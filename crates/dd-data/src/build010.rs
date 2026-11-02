//! Explicit battle build inputs. Acquisition, town activities and loot remain external.
use crate::{
    combat::*,
    content::ContentConfig,
    library,
    mechanics::{Buff, read_buff},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Build {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_trinkets: Option<Vec<crate::trinket010::TrinketState>>,
    pub trinket_states: BTreeMap<String, crate::trinket010::TrinketState>,
    pub initial_disease_turns: BTreeMap<String, u32>,
    pub trinkets: Vec<String>,
    pub quirks: Vec<String>,
    pub diseases: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BuildState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quirk_acts023: Vec<crate::mechanisms023::QuirkAct>,
    pub trinkets: Vec<crate::trinket010::TrinketState>,
    pub trinket_catalog: BTreeMap<String, crate::trinket010::Trinket>,
    pub sets: BTreeMap<String, Vec<Buff>>,
    pub disease_buffs: BTreeMap<String, Vec<Buff>>,
    pub disease_tags: BTreeMap<String, Vec<String>>,
    pub noncombat_buffs: Vec<String>,
    pub negative_quirks: usize,
    pub input: Build,
    pub buffs: Vec<Buff>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Disease {
    pub slots: u32,
    pub replaceable: bool,
    pub evolution: Option<Evolution>,
    pub tags: Vec<String>,
    pub item_changes: Vec<DiseaseItemUse>,
    pub id: String,
    pub weight: f64,
    pub buffs: Vec<Buff>,
    pub incompatible: Vec<String>,
    pub noncombat_buffs: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiseaseItemUse {
    pub item: String,
    pub change: Option<String>,
    pub effects: Vec<Effect>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Evolution {
    pub min: u32,
    pub max: u32,
    pub next: Option<String>,
    pub death: bool,
}
impl GameData {
    pub fn noncombat_buff(&self, id: &str) -> Result<bool> {
        let raw = self
            .buffs
            .get(id)
            .ok_or_else(|| format!("missing Buff {id}"))?;
        let outside = |v: &serde_json::Value| {
            v["stat_type"].as_str().is_some_and(|s| {
                [
                    "food_consumption_percent",
                    "starving_damage_percent",
                    "resolve_xp_bonus_percent",
                    "scouting_chance",
                    "shard_reward_percent",
                    "party_surprise_chance",
                    "monsters_surprise_chance",
                    "stress_from_idle_in_town",
                    "upgrade_discount",
                    "shard_consume_percent",
                ]
                .contains(&s)
            }) || (v["stat_type"] == "stress_dmg_received_percent"
                && ["hunger", "camping_eat"].contains(&v["stat_sub_type"].as_str().unwrap_or("")))
                || (v["stat_type"] == "hp_heal_received_percent" && v["stat_sub_type"] == "eat")
                || v["rule_type"]
                    .as_str()
                    .is_some_and(|r| ["in_camp", "in_town", "in_activity"].contains(&r))
        };
        Ok(raw
            .as_array()
            .map_or_else(|| outside(raw), |a| !a.is_empty() && a.iter().all(outside)))
    }
    pub fn quirks(&self) -> Result<BTreeMap<String, serde_json::Value>> {
        let _profile = crate::telemetry024c::scope("quirk_map");
        library::json_library(
            &self.overlay,
            "shared/quirk/",
            "quirk_library.json",
            "quirks",
        )
    }
    pub fn disease_catalog(&self) -> Result<BTreeMap<String, Disease>> {
        let _profile = crate::telemetry024c::scope("disease_parse");
        let mut out = BTreeMap::new();
        for (id, raw) in self.quirks()? {
            let raw = if let Some(variants) = raw.as_array() {
                if variants.iter().any(|r| r["is_disease"] == true) {
                    return Err(format!("ambiguous disease library {id}"));
                } else {
                    continue;
                }
            } else {
                raw
            };
            crate::schema012::quirk_fields(&raw)?;
            if raw["is_disease"] != true {
                continue;
            }
            let weight = raw["random_chance"]
                .as_f64()
                .ok_or("missing disease weight")?;
            if !weight.is_finite() || weight < 0. {
                return Err("invalid disease weight".into());
            }
            let mut buffs = vec![];
            let mut noncombat_buffs = vec![];
            for id in raw["buffs"].as_array().ok_or("missing disease buffs")? {
                let id = id.as_str().ok_or("invalid disease Buff ID")?;
                if self.noncombat_buff(id)? {
                    noncombat_buffs.push(id.to_owned());
                } else {
                    buffs.push(read_buff(&self.buffs, id)?);
                }
            }
            let incompatible = raw["incompatible_quirks"]
                .as_array()
                .ok_or("missing disease incompatibility")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or("invalid incompatible ID".into())
                })
                .collect::<Result<Vec<_>>>()?;
            out.insert(
                id.clone(),
                Disease {
                    slots: raw
                        .get("slot_size")
                        .map(|v| {
                            v.as_u64()
                                .and_then(|n| u32::try_from(n).ok())
                                .ok_or("invalid disease slot size")
                        })
                        .transpose()?
                        .unwrap_or(1),
                    replaceable: raw
                        .get("can_be_replaced_by_new_quirk")
                        .map(|v| v.as_bool().ok_or("invalid disease replacement flag"))
                        .transpose()?
                        .unwrap_or(true),
                    evolution: if raw.get("evolution_duration_min").is_some()
                        || raw.get("evolution_duration_max").is_some()
                    {
                        let min = raw["evolution_duration_min"]
                            .as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .ok_or("missing disease minimum duration")?;
                        let max = raw["evolution_duration_max"]
                            .as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .ok_or("missing disease maximum duration")?;
                        let next = raw
                            .get("evolution_class_id")
                            .map(|v| {
                                v.as_str()
                                    .map(str::to_string)
                                    .ok_or("invalid disease next class")
                            })
                            .transpose()?;
                        let death = raw
                            .get("evolution_causes_death")
                            .map(|v| v.as_bool().ok_or("invalid disease death flag"))
                            .transpose()?
                            .unwrap_or(false);
                        if min == 0 || min > max || max > 10000 || death == next.is_some() {
                            return Err("invalid disease evolution".into());
                        }
                        Some(Evolution {
                            min,
                            max,
                            next,
                            death,
                        })
                    } else {
                        None
                    },
                    tags: raw
                        .get("tags")
                        .map(|v| serde_json::from_value(v.clone()))
                        .transpose()
                        .map_err(|e| e.to_string())?
                        .unwrap_or_default(),
                    item_changes: {
                        let mut uses = vec![];
                        for (field, change) in [
                            ("use_item_changes", true),
                            ("use_item_dungeon_effects", false),
                        ] {
                            if let Some(items) = raw.get(field) {
                                for item in items.as_array().ok_or("invalid disease item table")? {
                                    let o = item.as_object().ok_or("invalid disease item entry")?;
                                    if o.keys().any(|k| {
                                        !["item_type", "item_id", "change_quirk_class_id", "effect"]
                                            .contains(&k.as_str())
                                    }) || !matches!(
                                        item["item_type"].as_str(),
                                        Some("estate" | "supply")
                                    ) {
                                        return Err("unsupported disease item entry".into());
                                    }
                                    uses.push(DiseaseItemUse {
                                        item: item["item_id"]
                                            .as_str()
                                            .ok_or("missing disease item ID")?
                                            .into(),
                                        change: if change {
                                            Some(
                                                item["change_quirk_class_id"]
                                                    .as_str()
                                                    .ok_or("missing disease replacement")?
                                                    .into(),
                                            )
                                        } else {
                                            None
                                        },
                                        effects: if change {
                                            vec![]
                                        } else {
                                            vec![
                                                self.effect(
                                                    item["effect"]
                                                        .as_str()
                                                        .ok_or("missing disease item effect")?,
                                                )?,
                                            ]
                                        },
                                    });
                                }
                            }
                        }
                        uses
                    },
                    id,
                    weight,
                    buffs,
                    incompatible,
                    noncombat_buffs,
                },
            );
        }
        Ok(out)
    }
    pub fn equip_build(&self, mut hero: Blueprint, input: Build) -> Result<Blueprint> {
        if !hero.hero || input.trinkets.len() > 2 {
            return Err("invalid hero build".into());
        }
        if input
            .quirks
            .iter()
            .chain(&input.diseases)
            .any(|id| hero.mechanics.hero.incompatible_quirks.contains(id))
        {
            return Err("hero class forbids this quirk".into());
        }
        if let Some(states) = &input.physical_trinkets
            && (!input.trinket_states.is_empty()
                || states.len() != input.trinkets.len()
                || states
                    .iter()
                    .zip(&input.trinkets)
                    .any(|(s, id)| s.definition.id != *id))
        {
            return Err(
                "physical trinket slots differ from build IDs or legacy map is also present".into(),
            );
        }
        let mut seen = BTreeSet::new();
        for id in &input.trinkets {
            if !seen.insert(id) && input.physical_trinkets.is_none() {
                return Err("duplicate build entry".into());
            }
        }
        for id in input.quirks.iter().chain(&input.diseases) {
            if !seen.insert(id) {
                return Err("duplicate build entry".into());
            }
        }
        let quirks = self.quirks()?;
        if !input.diseases.is_empty() {
            let catalog = self.disease_catalog()?;
            let mut slots = 0;
            for id in &input.diseases {
                let d = catalog.get(id).ok_or("unknown initial disease")?;
                slots += d.slots;
                if d.evolution.is_some()
                    && input
                        .initial_disease_turns
                        .get(id)
                        .is_none_or(|n| *n == 0 || *n > 10000)
                {
                    return Err("missing/invalid initial disease evolution counter".into());
                }
            }
            if slots > self.rules.quirks_max_diseases {
                return Err("initial disease capacity exceeded".into());
            }
        }
        if input
            .initial_disease_turns
            .keys()
            .any(|id| !input.diseases.contains(id))
            || input
                .trinket_states
                .keys()
                .any(|id| !input.trinkets.contains(id))
        {
            return Err("unreferenced initial build state".into());
        }
        let definitions = self.trinket_catalog(&input.trinkets)?;
        let trinkets = library::json_library(
            &self.overlay,
            "trinkets/",
            ".entries.trinkets.json",
            "entries",
        )?;
        let mut buffs = vec![];
        let mut noncombat_buffs = vec![];
        let mut disease_buffs = BTreeMap::<String, Vec<Buff>>::new();
        let mut disease_tags = BTreeMap::new();
        for (ids, disease, trinket) in [
            (&input.trinkets, false, true),
            (&input.quirks, false, false),
            (&input.diseases, true, false),
        ] {
            for id in ids {
                let raw = library::unique_json(
                    if trinket { &trinkets } else { &quirks },
                    id,
                    "build entry",
                )?;
                if !trinket {
                    crate::schema012::quirk_fields(raw)?;
                }
                if !trinket && raw["is_disease"].as_bool() != Some(disease) {
                    return Err("wrong build entry category".into());
                }
                if trinket {
                    let requirements = raw["hero_class_requirements"]
                        .as_array()
                        .ok_or("missing trinket class restrictions")?;
                    if !requirements.is_empty() && !requirements.iter().any(|v| v == &hero.id) {
                        return Err("trinket class restriction".into());
                    }
                    for field in ["combat_start_effects", "on_hit_effects", "on_miss_effects"] {
                        if raw
                            .get(field)
                            .is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
                        {
                            return Err(format!("unsupported trinket trigger {field}"));
                        }
                    }
                } else if raw["incompatible_quirks"].as_array().is_some_and(|a| {
                    a.iter()
                        .any(|v| v.as_str().is_some_and(|v| seen.contains(&v.to_string())))
                }) {
                    return Err("incompatible build quirks".into());
                }
                if trinket {
                    continue;
                }
                let mut selected = vec![];
                for bid in raw["buffs"].as_array().ok_or("missing build Buffs")? {
                    let bid = bid.as_str().ok_or("invalid build Buff ID")?;
                    if !self.noncombat_buff(bid)? {
                        selected.push(read_buff(&self.buffs, bid)?);
                    } else {
                        noncombat_buffs.push(bid.to_owned());
                    }
                }
                if disease {
                    disease_buffs.insert(id.clone(), selected);
                    disease_tags.insert(
                        id.clone(),
                        raw.get("tags")
                            .map(|v| serde_json::from_value::<Vec<String>>(v.clone()))
                            .transpose()
                            .map_err(|e| e.to_string())?
                            .unwrap_or_default(),
                    );
                } else {
                    buffs.extend(selected);
                }
            }
        }
        let physical_set = input.physical_trinkets.as_ref().and_then(|states| {
            let live = states.iter().filter(|t| !t.destroyed).collect::<Vec<_>>();
            if live.len() == 2
                && live[0].definition.set.is_some()
                && live[0].definition.set == live[1].definition.set
            {
                live[0].definition.set.clone()
            } else {
                None
            }
        });
        let mut sets = BTreeMap::new();
        for id in definitions
            .values()
            .filter_map(|d| d.set.as_ref())
            .collect::<BTreeSet<_>>()
        {
            let library =
                library::json_library(&self.overlay, "trinkets/", ".sets.trinkets.json", "sets")?;
            let raw = library::unique_json(&library, id, "trinket set")?;
            let mut values = vec![];
            for id in raw["buffs"].as_array().ok_or("missing set Buffs")? {
                let id = id.as_str().ok_or("invalid set Buff ID")?;
                if self.noncombat_buff(id)? {
                    if input.physical_trinkets.is_none()
                        || physical_set.as_deref() == raw["id"].as_str()
                    {
                        noncombat_buffs.push(id.into());
                    }
                } else {
                    values.push(read_buff(&self.buffs, id)?);
                }
            }
            sets.insert(id.clone(), values);
        }
        let equipped = input
            .trinkets
            .iter()
            .enumerate()
            .map(|(slot, id)| {
                let def = definitions.get(id).ok_or("missing trinket definition")?;
                let state = input
                    .physical_trinkets
                    .as_ref()
                    .map(|v| v[slot].clone())
                    .or_else(|| input.trinket_states.get(id).cloned())
                    .unwrap_or_else(|| crate::trinket010::TrinketState::fresh(def.clone()));
                self.validate_item_growth(&state)?;
                if &state.definition != def {
                    return Err("trinket state definition mismatch".into());
                }
                Ok(state)
            })
            .collect::<Result<Vec<_>>>()?;
        if input.physical_trinkets.is_some() {
            noncombat_buffs.extend(
                equipped
                    .iter()
                    .filter(|t| !t.destroyed)
                    .flat_map(|t| t.definition.noncombat_buffs.clone()),
            );
        }
        for b in &buffs {
            for id in &b.activation.attack_additional_effect_ids {
                hero.mechanics
                    .extra_effects
                    .insert(id.clone(), self.effect(id)?);
            }
        }
        let negative_quirks = input
            .quirks
            .iter()
            .filter(|id| quirks.get(*id).is_some_and(|v| v["is_positive"] == false))
            .count();
        if negative_quirks > self.rules.quirks_max_negative as usize
            || input.quirks.len() - negative_quirks > self.rules.quirks_max_positive as usize
        {
            return Err("build quirk capacity exceeded".into());
        }
        hero.mechanics.build = BuildState {
            quirk_acts023: self.quirk_acts023.values().cloned().collect(),
            trinkets: equipped,
            trinket_catalog: definitions,
            sets,
            disease_buffs,
            disease_tags,
            input,
            buffs,
            negative_quirks,
            noncombat_buffs,
        };
        Ok(hero)
    }
    pub fn battle_context(
        &self,
        region: String,
        surprise: Surprise,
        stall: bool,
    ) -> Result<BattleContext> {
        let raw = self.overlay.json("shared/rules.json")?;
        let get = |k: &str| {
            raw[k]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("missing context rule {k}"))
        };
        let context = BattleContext {
            trinket_inventory: vec![],
            trinket_catalog: BTreeMap::new(),
            quest_uses_remaining: BTreeMap::new(),
            region,
            corridor: false,
            walking_backwards: false,
            history: BTreeSet::new(),
            surprise,
            corpses: true,
            stall,
            stall_extra_size: get("stall_extra_round_size_threshold")?,
            stall_extra_amount: get("stall_extra_round_size_amount")?,
            stall_accelerated: get("stall_round_accelerated_only_increment")?,
            stall_consecutive: get("stall_round_accelerated_only_consecutive_increment")?,
            stall_party: get("stall_party_size_threshold")?,
            stall_enemy: get("stall_enemy_size_threshold")?,
            stall_skills: get("stall_number_of_invalidating_skills_threshold")?,
            stall_stress_round: get("stall_round_stress_effect_threshold")?,
            stall_summon_round: get("stall_round_summon_effect_threshold")?,
            stall_reset: get("stall_round_reset_threshold")?,
            stall_effect: self.effect(
                raw["stall_stress_effect_name"]
                    .as_str()
                    .ok_or("missing stall effect")?,
            )?,
            reinforcements: vec![],
            diseases: self.disease_catalog()?,
            disease_max: get("quirks_max_diseases")?,
            content: self.overlay.config.clone(),
        };
        context.validate()?;
        Ok(context)
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Surprise {
    #[default]
    Neither,
    Heroes,
    Monsters,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BattleContext {
    pub trinket_inventory: Vec<crate::trinket010::TrinketState>,
    pub trinket_catalog: BTreeMap<String, crate::trinket010::Trinket>,
    pub quest_uses_remaining: BTreeMap<String, u32>,
    pub content: ContentConfig,
    pub region: String,
    pub corridor: bool,
    pub walking_backwards: bool,
    pub history: BTreeSet<String>,
    pub surprise: Surprise,
    pub corpses: bool,
    pub stall: bool,
    pub stall_extra_size: u32,
    pub stall_extra_amount: u32,
    pub stall_accelerated: u32,
    pub stall_consecutive: u32,
    pub stall_party: u32,
    pub stall_enemy: u32,
    pub stall_skills: u32,
    pub stall_stress_round: u32,
    pub stall_summon_round: u32,
    pub stall_reset: u32,
    pub stall_effect: Effect,
    pub reinforcements: Vec<Blueprint>,
    pub diseases: BTreeMap<String, Disease>,
    pub disease_max: u32,
}
impl BattleContext {
    pub fn validate(&self) -> Result<()> {
        self.content.validate()?;
        if ![
            "crypts",
            "weald",
            "warrens",
            "cove",
            "darkestdungeon",
            "courtyard",
            "farm",
            "town",
        ]
        .contains(&self.region.as_str())
            || self.disease_max > 100
            || self.stall_reset == 0
            || self.stall_extra_size > 4
            || self.stall_extra_amount > 10000
            || self.stall_accelerated > 10000
            || self.stall_consecutive > 10000
            || self.stall_party > 4
            || self.stall_enemy > 4
            || self.stall_skills > 10000
            || self.stall_stress_round > 10000
            || self.stall_summon_round > 10000
        {
            return Err("invalid battle context".into());
        }
        if self.stall && self.reinforcements.is_empty() {
            return Err("stall requires explicit reinforcement encounter".into());
        }
        for (id, d) in &self.diseases {
            if id != &d.id || !d.weight.is_finite() || d.weight < 0. {
                return Err("invalid disease catalog".into());
            }
            if d.slots == 0 || d.slots > self.disease_max || d.tags.iter().any(|s| s.is_empty()) {
                return Err("invalid disease slots/tags".into());
            }
            if let Some(e) = &d.evolution
                && (e.min == 0
                    || e.min > e.max
                    || e.max > 10000
                    || e.death == e.next.is_some()
                    || e.next
                        .as_ref()
                        .is_some_and(|id| !self.diseases.contains_key(id)))
            {
                return Err("invalid disease evolution dependency".into());
            }
            for item in &d.item_changes {
                if item.item.is_empty()
                    || item
                        .change
                        .as_ref()
                        .is_some_and(|id| !self.diseases.contains_key(id))
                {
                    return Err("invalid disease item transition dependency".into());
                }
            }
            for b in &d.buffs {
                b.validate()?;
            }
        }
        Ok(())
    }
}

