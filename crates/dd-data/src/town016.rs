//! Explicit source boundary for permanent town facilities and assets.
use crate::{
    combat::{GameData, Result},
    library, opening015,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "candidate-town-016";
pub const BUILDINGS: [&str; 8] = [
    "abbey",
    "tavern",
    "sanitarium",
    "stage_coach",
    "guild",
    "blacksmith",
    "camping_trainer",
    "nomad_wagon",
];
pub fn source_path(p: &str) -> bool {
    opening015::source_path(p)
        || p.starts_with("upgrades/") && p.ends_with(".upgrades.json")
        || p.starts_with("campaign/town/") && p.ends_with(".json")
        || p == "campaign/heirloom_exchange/heirloom_exchange.json"
        || p == "campaign/progression/progression.json"
        || p == "scripts/starting_save/persist.upgrades.json"
        || p.starts_with("raid/camping/") && p.ends_with(".camping_skills.json")
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Candidates {
    pub version: String,
    pub source: String,
    pub caretaker_slots: usize,
    pub trinket_slots: usize,
    pub camping_slots: usize,
    pub currency_precision: u64,
    pub orphan_tiers: BTreeSet<String>,
    pub semantics: BTreeMap<String, String>,
}
impl Candidates {
    pub fn documented() -> Result<Self> {
        serde_json::from_str(r###"{
  "version": "town-candidates-016-v1",
  "source": "Original town/estate/roster/upgrade/camping definitions; missing timing and selection algorithms remain explicit candidates pending original executable calibration; https://darkestdungeon.wiki.gg/wiki/Sanitarium; https://darkestdungeon.wiki.gg/wiki/Quirks; https://darkestdungeon.wiki.gg/wiki/Flagellant; https://darkestdungeon.wiki.gg/wiki/Camping; https://darkestdungeon.wiki.gg/wiki/Stage_Coach",
  "caretaker_slots": 1,
  "orphan_tiers": [
    "abbey.meditation:g",
    "abbey.prayer:g",
    "abbey.flagellation:g",
    "tavern.bar:g",
    "tavern.gambling:g",
    "tavern.brothel:g"
  ],
  "semantics": {
    "scope": "permanent_roster_standard_facilities_v1",
    "missing_camp_rank": "known_skill_rank_else_zero_v1",
    "rounding": "currency_ceil_discounts_additive_capped_at_zero_cost_v1",
    "caretaker": "uniform_unlocked_caretaker_friendly_activity_once_each_return_v1",
    "return": "bound_host_mission_receipt_once_no_policy_wait_week_v1",
    "orphan_tiers": "listed_impossible_source_tiers_remain_inactive_audited_v1",
    "slot_limits": "reuse_native_two_trinket_contract_and_explicit_four_camping_candidate_v1",
    "currency_arithmetic": "millionth_fixed_point_source_rates_round_up_gold_quotes_v1",
    "treatment": "original_positive_price_for_lock_or_remove_combined_one_positive_one_negative_v1",
    "market_refresh": "stock_frozen_until_return_class_cards_without_replacement_highest_matching_death_tier_guaranteed_once_v1",
    "activity_weight_buffs": "add_source_modifier_to_matching_outcome_weight_nonnegative_v1",
    "recruit_names": "class_and_stable_guid_no_localization_rendering_v1",
    "failed_scope": "campaign_contagion_mixed_activity_and_farm_special_stores_rejected_v1",
    "negative_quirks": "source_mission_count_threshold_then_source_lock_chance_with_slot_limit_v1",
    "district_enable": "content_config_gates_optional_enabled_fragment_metadata_v1",
    "facility_counter": "all_settled_returns_including_abandon_source_finished_counter_v1",
    "region_progression": "host_marks_progressing_quest_source_length_indexed_xp_then_original_thresholds_v1",
    "exclusive_camping": "flagellant_draw_source_class_plus_shared_count_from_its_unique_pool_only_v1",
    "camping_capacity": "generation_extra_counts_capped_at_distinct_available_skills_v1",
    "generation_events": "source_town_event_dependency_requires_explicit_trusted_host_ack_no_scheduler_v1",
    "guaranteed_skills": "source_generation_guaranteed_included_before_random_skill_draw_v1",
    "activity_lock": "one_extra_settled_return_no_charge_or_second_treatment_v1",
    "go_missing": "unavailable_for_duration_complete_future_returns_not_counting_treatment_return_v1",
    "town_buff_lifetime": "replace_same_id_persist_until_next_participating_return_v1",
    "idle_relief": "max_source_base_and_district_exclude_jobs_missing_departing_v1",
    "curse_week": "source_town_clock_decrement_discard_overshoot_feed_eligible_wasting_from_estate_first_skip_fed_new_stage_tick_v1",
    "assets": "loss_side_effect_chooses_unowned_matching_rarity_death_equipment_only_explicit_physical_recovery_v1",
    "district_interest": "floor_original_rate_times_departure_opening_gold_once_per_bound_return_v1",
    "district_host": "mandatory_typed_ack_for_field_camping_curio_hunger_stacking_provision_loot_battle_helper_applies_light_and_replacements_v1",
    "estate_identity": "opening_receipt_content_definition_and_town_seed_digest_then_monotonic_departure_token_v1",
    "quest_owner": "host_owns_eligibility_objectives_finite_pickup_abandon_stress_and_recovery_no_policy_mission_forgery_v1",
    "item_pricing": "original_price_field_missing_means_no_gold_trade_no_unspecified_trophy_price_exception_v1",
    "experienced_training": "highest_original_class_node_rank_allowed_by_recruit_resolve_level_independent_gear_skill_v1",
    "emergency_recruits": "stock_at_least_original_start_party_size_minus_living_roster_no_extra_refresh_v1",
    "first_classes": "015_consumes_original_first_hero_classes_no_second_forced_pair_in_016_v1",
    "quest_assets": "native_compact_slot_receipt_maps_to_departure_physical_slot_then_living_equipped_successful_quest_transform_once_fresh_budgets_keep_instance_v1",
    "unknown_quest_effects": "nonempty_on_quest_complete_additional_effects_strict_departure_rejection_until_expedition_owner_support_v1"
  },
  "trinket_slots": 2,
  "camping_slots": 4,
  "currency_precision": 1000000
}"###)
            .map_err(|e| e.to_string())
    }
    pub fn validate(&self) -> Result<()> {
        if self != &Self::documented()? {
            return Err("unknown/modified town candidate parameters".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub content: crate::content::ContentConfig,
    pub candidates: Candidates,
    pub base: opening015::Data,
    pub buildings: BTreeMap<String, Value>,
    pub activities: BTreeMap<String, Value>,
    pub stores: BTreeMap<String, Value>,
    pub trees: BTreeMap<String, Value>,
    pub camps: BTreeMap<String, Value>,
    pub districts: BTreeMap<String, Value>,
    pub exchange: Vec<Value>,
    pub initial_upgrades: Value,
    pub camp_threshold: u32,
    pub progression: Value,
    pub sources: BTreeMap<String, Vec<usize>>,
    pub issues: Vec<String>,
}
fn put(target: &mut BTreeMap<String, Value>, id: String, value: Value) {
    library::insert_json(target, id, value);
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| format!("missing town array {key}"))
}
pub fn uint(v: &Value, key: &str) -> Result<u32> {
    v[key]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("invalid town integer {key}"))
}
fn fields(v: &Value, allowed: &[&str], kind: &str) -> Result<()> {
    if v.as_object()
        .ok_or("town record must be object")?
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
    {
        return Err(format!("unknown {kind} field: {v}"));
    }
    Ok(())
}
impl Data {
    pub fn load(g: &GameData, candidates: Candidates) -> Result<Self> {
        candidates.validate()?;
        let base = opening015::Data::load(g, opening015::Candidates::documented()?)?;
        let mut data = Self {
            content: g.overlay.config.clone(),
            candidates,
            base,
            buildings: BTreeMap::new(),
            activities: BTreeMap::new(),
            stores: BTreeMap::new(),
            trees: BTreeMap::new(),
            camps: BTreeMap::new(),
            districts: BTreeMap::new(),
            exchange: vec![],
            initial_upgrades: Value::Null,
            camp_threshold: 0,
            progression: Value::Null,
            sources: BTreeMap::new(),
            issues: vec![],
        };
        let enabled = g.hero_ids().cloned().collect::<BTreeSet<_>>();
        for p in g.overlay.paths.keys() {
            let is_building = p.starts_with("campaign/town/buildings/")
                && p.ends_with(".building.json")
                && p.split('/').nth(3).is_some_and(|b| BUILDINGS.contains(&b));
            let is_tree = p.starts_with("upgrades/building/")
                && p.ends_with(".upgrades.json")
                && p.rsplit('/')
                    .next()
                    .and_then(|x| x.strip_suffix(".upgrades.json"))
                    .is_some_and(|b| BUILDINGS.contains(&b))
                || p.starts_with("upgrades/heroes/")
                    && p.ends_with(".upgrades.json")
                    && p.rsplit('/')
                        .next()
                        .and_then(|x| x.strip_suffix(".upgrades.json"))
                        .is_some_and(|b| enabled.contains(b));
            let is_camp = p.starts_with("raid/camping/") && p.ends_with(".camping_skills.json");
            let is_district = p.starts_with("campaign/town/districts/")
                && p.ends_with(".json")
                && g.overlay.config.packages.iter().any(|p| p == "districts");
            if !(is_building || is_tree || is_camp || is_district) {
                continue;
            }
            data.sources.insert(p.clone(), g.overlay.sources(p));
            for bytes in g.overlay.read_town_versions(p)? {
                let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("{p}: {e}"))?;
                if is_building {
                    fields(
                        &v,
                        &["on_start_town_visit_priority", "requirements", "data"],
                        "building",
                    )?;
                    let id = p.split('/').nth(3).unwrap();
                    for (key, value) in v.as_object().unwrap() {
                        if key == "data" {
                            for (k, value) in value.as_object().ok_or("invalid building data")? {
                                match k.as_str() {
                                    "activities" | "stores" => {
                                        for row in value.as_array().ok_or("invalid town entries")? {
                                            fields(row, &["id", "data"], "facility entry")?;
                                            let key = format!(
                                                "{id}.{}",
                                                row["id"]
                                                    .as_str()
                                                    .ok_or("missing activity/store id")?
                                            );
                                            put(
                                                if k == "activities" {
                                                    &mut data.activities
                                                } else {
                                                    &mut data.stores
                                                },
                                                key,
                                                row["data"].clone(),
                                            );
                                        }
                                    }
                                    "combat_skill_cost_discount_upgrades"
                                    | "equipment_cost_discount_upgrades"
                                    | "camping_skill_cost_discount_upgrades" => put(
                                        &mut data.buildings,
                                        format!("{id}.{k}"),
                                        json!({"value":value}),
                                    ),
                                    _ => return Err(format!("unsupported facility data {id}.{k}")),
                                }
                            }
                        } else {
                            put(
                                &mut data.buildings,
                                format!("{id}.{key}"),
                                json!({"value":value}),
                            );
                        }
                    }
                }
                if is_tree {
                    fields(&v, &["trees"], "upgrade file")?;
                    for tree in array(&v, "trees")? {
                        let id = tree["id"].as_str().ok_or("missing tree id")?;
                        put(&mut data.trees, id.into(), tree.clone());
                    }
                }
                if is_camp {
                    fields(&v, &["configuration", "skills"], "camping skill library")?;
                    if let Some(c) = v.get("configuration") {
                        let threshold = uint(c, "class_specific_number_of_classes_threshold")?;
                        if data.camp_threshold != 0 && data.camp_threshold != threshold {
                            return Err("conflicting camping class threshold".into());
                        }
                        data.camp_threshold = threshold;
                    }
                    for skill in array(&v, "skills")? {
                        let id = skill["id"].as_str().ok_or("missing camp skill id")?;
                        if uint(skill, "level")? != 0 {
                            return Err("unsupported camping skill rank".into());
                        }
                        for class in array(skill, "hero_classes")?
                            .iter()
                            .filter_map(Value::as_str)
                            .filter(|c| enabled.contains(*c))
                        {
                            let mut scoped = skill.clone();
                            scoped["hero_classes"] = json!([class]);
                            put(&mut data.camps, format!("{class}.{id}"), scoped);
                        }
                    }
                }
                if is_district {
                    fields(&v, &["enabled", "buildings"], "district file")?;
                    if v.get("enabled").is_some_and(|v| v != true) {
                        return Err("unexpected disabled enabled-layer district file".into());
                    }
                    for district in array(&v, "buildings")? {
                        fields(
                            district,
                            &["name", "render_data", "currency_cost", "buff_list"],
                            "district",
                        )?;
                        let id = district["name"].as_str().ok_or("missing district id")?;
                        let mut functional = district.clone();
                        functional.as_object_mut().unwrap().remove("render_data");
                        put(&mut data.districts, id.into(), functional);
                    }
                }
            }
        }
        let p = "campaign/heirloom_exchange/heirloom_exchange.json";
        let exchange: Value =
            serde_json::from_slice(&g.overlay.read_town(p)?).map_err(|e| e.to_string())?;
        fields(&exchange, &["exchange_rates"], "exchange")?;
        data.exchange = array(&exchange, "exchange_rates")?.clone();
        data.sources.insert(p.into(), g.overlay.sources(p));
        let p = "scripts/starting_save/persist.upgrades.json";
        data.initial_upgrades =
            serde_json::from_slice(&g.overlay.read_town(p)?).map_err(|e| e.to_string())?;
        data.sources.insert(p.into(), g.overlay.sources(p));
        let p = "campaign/progression/progression.json";
        data.progression =
            serde_json::from_slice(&g.overlay.read_town(p)?).map_err(|e| e.to_string())?;
        data.sources.insert(p.into(), g.overlay.sources(p));
        data.validate()?;
        for activity in data.activities.values() {
            if let Some(rows) = activity["side_effects"]["results"].as_array() {
                for row in rows {
                    for value in array(row, "data")? {
                        match row["type"].as_str() {
                            Some("apply_buff") => {
                                for id in array(value, "buff_library_ids")? {
                                    crate::mechanics::read_buff(
                                        &g.buffs,
                                        id.as_str().ok_or("invalid activity Buff ID")?,
                                    )?;
                                }
                            }
                            Some("add_quirk") => {
                                library::unique_json(
                                    &data.base.quirk_defs,
                                    value["quirk_library_name"]
                                        .as_str()
                                        .ok_or("missing granted quirk")?,
                                    "activity quirk",
                                )?;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(data)
    }
    pub fn tree(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.trees, id, "town upgrade tree")
    }
    pub fn node(&self, tree: &str, code: &str) -> Result<&Value> {
        let rows = array(self.tree(tree)?, "requirements")?;
        let rows = rows
            .iter()
            .filter(|v| v["code"] == code)
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            return Err(format!("missing/duplicate town node {tree}:{code}"));
        }
        Ok(rows[0])
    }
    pub fn building(&self, id: &str, field: &str) -> Result<&Value> {
        Ok(
            &library::unique_json(&self.buildings, &format!("{id}.{field}"), "town facility")?["value"],
        )
    }
    pub fn activity(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.activities, id, "town activity")
    }
    pub fn store(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.stores, id, "town store")
    }
    pub fn camp(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.camps, id, "camp skill")
    }
    pub fn district(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.districts, id, "district")
    }
    fn validate(&mut self) -> Result<()> {
        for b in BUILDINGS {
            fields(
                self.building(b, "requirements")?,
                &["number_of_quests_finished", "highest_dungeon_level"],
                "facility requirements",
            )?;
        }
        for id in self.activities.keys() {
            if ![
                "abbey.meditation",
                "abbey.prayer",
                "abbey.flagellation",
                "tavern.bar",
                "tavern.gambling",
                "tavern.brothel",
                "sanitarium.treatment",
                "sanitarium.disease_treatment",
            ]
            .contains(&id.as_str())
            {
                return Err(format!("unsupported town activity {id}"));
            }
        }
        for (id, raw) in &self.stores {
            match id.as_str() {
                "stage_coach.hero_recruit" => fields(
                    raw,
                    &[
                        "first_hero_classes",
                        "number_of_recruits_upgrades",
                        "roster_size_upgrades",
                        "upgraded_recruits_upgrades",
                    ],
                    "recruit store",
                )?,
                "nomad_wagon.trinket_supply" => fields(
                    raw,
                    &[
                        "number_of_trinkets_upgrades",
                        "rarity_generation_table",
                        "trinket_cost_discount_upgrades",
                        "trinket_sell_value_discount_upgrades",
                    ],
                    "trinket store",
                )?,
                "stage_coach.shard_hero_recruit" | "nomad_wagon.shard_trinket_supply" => self
                    .issues
                    .push(format!("deferred explicit campaign owner required: {id}")),
                _ => return Err(format!("unsupported town store {id}")),
            }
        }
        let initial = self.initial_upgrades["data"]["purchases"]
            .as_object()
            .ok_or("missing starting upgrades")?;
        for (id, row) in initial {
            fields(
                row,
                &[
                    "instance_number",
                    "tree_id",
                    "requirement_code",
                    "is_purchased",
                ],
                "initial purchase",
            )?;
            if row["tree_id"].as_u64() != Some(u64::from(crate::dson013::hash(id)))
                || row["requirement_code"] != "0"
                || row["is_purchased"] != true
            {
                return Err("unsupported original initial upgrade".into());
            }
            let guid = row["instance_number"]
                .as_u64()
                .ok_or("invalid initial upgrade identity")?;
            let hero = self
                .base
                .initial
                .iter()
                .find(|h| h.guid == guid)
                .ok_or("unknown initial purchase hero")?;
            let (class, skill) = id.split_once('.').ok_or("invalid initial skill tree")?;
            let camping=self.base.roster_source["data"]["heroes"][guid.to_string()]["skills"]["selected_camping_skills"].get(skill).is_some();
            if hero.blueprint.id != class
                || (!camping && !hero.blueprint.skills.iter().any(|s| s.id == skill))
            {
                return Err("starting upgrade differs from original roster".into());
            }
        }
        for (id, v) in &self.trees {
            let tree = library::unique_json(&self.trees, id, "town tree")?;
            fields(
                tree,
                &["id", "is_instanced", "tags", "requirements"],
                "upgrade tree",
            )?;
            if !tree["is_instanced"].is_boolean() {
                return Err("missing tree ownership type".into());
            }
            let mut codes = BTreeSet::new();
            for node in array(v, "requirements")? {
                fields(
                    node,
                    &[
                        "code",
                        "currency_cost",
                        "prerequisite_requirements",
                        "prerequisite_resolve_level",
                    ],
                    "upgrade node",
                )?;
                let code = node["code"].as_str().ok_or("missing upgrade code")?;
                if !codes.insert(code) {
                    return Err(format!("duplicate node {id}:{code}"));
                }
                for cost in array(node, "currency_cost")? {
                    fields(cost, &["type", "amount"], "upgrade currency")?;
                    uint(cost, "amount")?;
                    cost["type"].as_str().ok_or("missing currency type")?;
                }
                for req in array(node, "prerequisite_requirements")? {
                    fields(req, &["tree_id", "requirement_code"], "upgrade dependency")?;
                    self.node(
                        req["tree_id"].as_str().ok_or("missing dependency tree")?,
                        req["requirement_code"]
                            .as_str()
                            .ok_or("missing dependency code")?,
                    )?;
                }
            }
        }
        for (id, raw) in &self.activities {
            let v = library::unique_json(&self.activities, id, "activity")?;
            validate_activity(v)?;
            for (key, rows) in v.as_object().ok_or("invalid activity")? {
                if key.ends_with("_upgrades") {
                    for row in rows.as_array().ok_or("invalid activity upgrades")? {
                        if let Some(code) = row["upgrade_requirement_code"].as_str() {
                            let tree = row["upgrade_tree_id"].as_str().unwrap_or(id);
                            if self.node(tree, code).is_err() {
                                let defect = format!("{tree}:{code}");
                                if !self.candidates.orphan_tiers.contains(&defect) {
                                    return Err(format!("unresolved facility tier {defect}"));
                                }
                                self.issues.push(format!(
                                    "explicit unreachable tier {id}.{key}: {defect}"
                                ));
                            }
                        }
                    }
                }
            }
            let _ = raw;
        }
        for id in &self.candidates.orphan_tiers {
            if !self.issues.iter().any(|s| s.ends_with(id)) {
                return Err(format!("unused orphan candidate {id}"));
            }
        }
        for (id, v) in &self.districts {
            let d = library::unique_json(&self.districts, id, "district")?;
            for row in array(d, "buff_list")? {
                let kind = row["type"].as_str().ok_or("missing district effect type")?;
                if ![
                    "DistrictPrestigeBuffData",
                    "DistrictEstateBuffData",
                    "DistrictLightBuffData",
                    "DistrictHeroBuffData",
                    "DistrictSupplyBuffData",
                    "DistrictRosterStressReliefBuffData",
                    "DistrictCurioInteractionBuffData",
                    "DistrictCampingBuffData",
                    "DistrictDisableBuffData",
                    "DistrictStackingBuffData",
                    "DistrictReplacementInventoryEffectBuffData",
                    "DistrictRandomLootBuffData",
                ]
                .contains(&kind)
                {
                    return Err(format!("unsupported district effect {kind}"));
                }
            }
            let _ = v;
        }
        for row in &self.exchange {
            fields(
                row,
                &[
                    "exchange_from_type",
                    "exchange_from_amount",
                    "exchange_to_type",
                    "exchange_to_amount",
                ],
                "exchange rate",
            )?;
            if uint(row, "exchange_from_amount")? == 0
                || uint(row, "exchange_to_amount")? == 0
                || row["exchange_from_type"] == row["exchange_to_type"]
            {
                return Err("invalid exchange ratio".into());
            }
        }
        Ok(())
    }
    pub fn audit(&self) -> Value {
        let quest_asset_metadata = self
            .base
            .curios
            .trinkets
            .iter()
            .filter(|(_, v)| {
                v.get("transform_on_quest_complete").is_some()
                    || v.get("on_quest_complete_additional_effects").is_some()
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<BTreeMap<_, _>>();
        let side_effect_types = self
            .activities
            .values()
            .filter_map(|a| a["side_effects"]["results"].as_array())
            .flatten()
            .filter_map(|r| r["type"].as_str())
            .collect::<BTreeSet<_>>();
        let nodes = self
            .trees
            .values()
            .filter_map(|v| v["requirements"].as_array())
            .map(Vec::len)
            .sum::<usize>();
        json!({"profile":PROFILE,"counts":{"hero_classes":self.base.class_extras.len(),"ordinary_buildings":BUILDINGS.len(),"trees":self.trees.len(),"nodes":nodes,"activities":self.activities.len(),"districts":self.districts.len()},"side_effect_types":side_effect_types,"quest_asset_metadata":quest_asset_metadata,"initial_upgrades":self.initial_upgrades,"progression":self.progression,"content":self.content,"candidates":self.candidates,"buildings":BUILDINGS,"trees":self.trees,"activities":self.activities,"stores":self.stores,"districts":self.districts,"camp_skills":self.camps.keys().collect::<Vec<_>>(),"exchange":self.exchange,"sources":self.sources,"issues":self.issues})
    }
}

impl Data {
    pub fn replacement(
        &self,
        g: &GameData,
        district: &str,
        item: &str,
    ) -> Result<crate::items::Supply> {
        let d = self.district(district)?;
        let effects = array(d, "buff_list")?
            .iter()
            .filter(|r| {
                r["type"] == "DistrictReplacementInventoryEffectBuffData" && r["item_id"] == item
            })
            .collect::<Vec<_>>();
        if effects.len() != 1 {
            return Err("unknown/conflicting district item replacement".into());
        }
        let effect = effects[0]["effect"]
            .as_str()
            .ok_or("missing replacement effect")?;
        g.supply_with_override(item, vec![g.effect(effect)?])
    }
}

/// Validate every enabled activity branch, including zero weights which town buffs can activate.
pub fn validate_activity(v: &Value) -> Result<()> {
    fields(
        v,
        &[
            "affliction_cure_upgrades",
            "cost_upgrades",
            "miscellaneous",
            "requirements",
            "side_effects",
            "slot_upgrades",
            "stress_upgrades",
            "negative_quirk_cost_upgrades",
            "permanent_negative_quirk_cost_upgrades",
            "positive_quirk_cost_upgrades",
            "quirk_treatment_upgrades",
            "disease_quirk_cost_upgrades",
            "disease_quirk_cure_all_chance_upgrades",
        ],
        "activity",
    )?;
    for (key, rows) in v.as_object().ok_or("invalid activity")? {
        if key.ends_with("_upgrades") {
            for row in rows.as_array().ok_or("invalid activity tiers")? {
                let mut allowed = vec!["upgrade_requirement_code", "upgrade_tree_id"];
                match key.as_str() {
                    "slot_upgrades" => allowed.push("number_of_slots"),
                    "stress_upgrades" => allowed.extend(["heal_low", "heal_high"]),
                    "affliction_cure_upgrades"
                    | "quirk_treatment_upgrades"
                    | "disease_quirk_cure_all_chance_upgrades" => allowed.push("chance"),
                    _ => allowed.push("cost_currency"),
                }
                fields(row, &allowed, "activity tier")?;
            }
        }
    }
    if let Some(rows) = v["requirements"].as_array() {
        for row in rows {
            fields(row, &["type", "data"], "activity requirement")?;
            match row["type"].as_str() {
                Some("not_have_quirks") => {
                    fields(
                        &row["data"],
                        &["quirk_library_names"],
                        "activity constraint",
                    )?;
                    array(&row["data"], "quirk_library_names")?;
                }
                Some("is_diseased") => {}
                _ => return Err("unsupported town activity requirement".into()),
            }
        }
    }
    if let Some(m) = v.get("miscellaneous") {
        fields(m, &["caretaker_friendly"], "activity miscellaneous")?;
    }
    if let Some(side) = v.get("side_effects") {
        fields(side, &["chance", "results"], "activity side effects")?;
        let chance = side["chance"]
            .as_f64()
            .ok_or("invalid activity side effect chance")?;
        if !(0. ..=1.).contains(&chance) {
            return Err("invalid activity side effect chance".into());
        }
        for row in array(side, "results")? {
            fields(row, &["chance", "type", "data"], "side effect")?;
            let weight = row["chance"].as_f64().ok_or("missing side effect weight")?;
            if !weight.is_finite() || weight < 0. {
                return Err("invalid side effect weight".into());
            }
            let kind = row["type"].as_str().ok_or("missing side effect type")?;
            let extra = match kind {
                "activity_lock" => vec![],
                "go_missing" => vec!["duration"],
                "add_quirk" => vec!["quirk_library_name"],
                "apply_buff" => vec!["buff_library_ids"],
                "add_currency" | "remove_currency" => vec!["type", "amount"],
                "add_trinket" | "remove_trinket" => vec!["rarity"],
                _ => return Err(format!("unsupported activity side effect {kind}")),
            };
            let data = array(row, "data")?;
            if kind == "activity_lock" && !data.is_empty() {
                return Err("unsupported activity lock payload".into());
            }
            for d in data {
                let mut allowed = extra.clone();
                allowed.push("chance");
                fields(d, &allowed, "side effect payload")?;
                let w = d["chance"].as_f64().ok_or("missing payload weight")?;
                if !w.is_finite() || w < 0. {
                    return Err("invalid side effect payload weight".into());
                }
            }
        }
    }
    Ok(())
}
