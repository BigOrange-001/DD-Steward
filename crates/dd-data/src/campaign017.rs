//! Campaign source index. Cross-file alternatives stay ambiguous until an explicit selection.
use crate::{
    combat::{GameData, Result},
    content::ContentConfig,
    library,
    overlay::Overlay,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "candidate-campaign-017";
pub const EFFECTS: &[&str] = &[
    "embark_party_buff",
    "idle_resolve_level",
    "bonus_recruit",
    "in_activity_buff",
    "activity_lock",
    "activity_cost_change",
    "provision_item_type_cost_change",
    "provision_item_type_amount_change",
    "upgrade_tag_discount",
    "free_activity",
    "dead_recruit",
    "remove_quest_hero_level_restriction",
    "upgrade_tag_free",
    "idle_buff",
    "plot_quest",
    "trinket_retention_add_from_storage",
    "stage_coach_bonus_recruits",
    "remove_vampire",
    "stress_treatment_debuff",
    "districts_unlocked",
    "bonus_currency",
    "shard_buff",
    "quirked_recruit",
    "event_cost",
    "roster_disease_heal",
    "roster_stress_heal",
    "trinket_sell_multiplier",
    "idle_negative_quirk_remove",
    "heirloom_gain_multiplier",
    "nomad_wagon_trinket_discount",
    "bonus_recruit_stage_coach_level",
];
pub fn source_path(p: &str) -> bool {
    crate::town016::source_path(p)
        || p.starts_with("trinkets/") && p.ends_with(".rarities.trinkets.json")
        || p == "campaign/roster/roster.contagion.json"
        || p == "dungeons/_shared/shared.json"
        || p == "scripts/raid_settings.json"
        || p.starts_with("campaign/town_events/")
            && p.ends_with(".json")
            && !p.starts_with("campaign/town_events/arena.")
        || p.starts_with("campaign/progression/") && p.ends_with(".json")
}
pub fn fields(v: &Value, allowed: &[&str], kind: &str) -> Result<()> {
    for k in v
        .as_object()
        .ok_or_else(|| format!("{kind} requires object"))?
        .keys()
    {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("unknown {kind} field {k}"));
        }
    }
    Ok(())
}
pub fn uint(v: &Value, k: &str) -> Result<u32> {
    crate::town016::uint(v, k)
}
pub fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>> {
    v[k].as_array()
        .ok_or_else(|| format!("missing campaign array {k}"))
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Candidates {
    pub version: String,
    pub source: String,
    pub semantics: BTreeMap<String, String>,
    pub parameters: BTreeMap<String, f64>,
    pub protected_award_categories: BTreeSet<String>,
    pub protected_rarities: BTreeSet<String>,
}
impl Candidates {
    pub fn documented() -> Result<Self> {
        serde_json::from_str(r###"{
  "version": "campaign-candidates-017-v1",
  "source": "Enabled original campaign/quest, campaign/town_events and campaign/progression definitions; execution algorithms remain explicit candidates pending original executable calibration.",
  "semantics": {
    "source_merge": "logical_id_functional_json_cosmetic_fields_excluded_conflict_is_error_v1",
    "generation_modifiers": "sum_distinct_effective_source_file_modifiers_v1",
    "week_epoch": "old_road_return_week_one_tutorial_return_week_two_v1",
    "event_chance": "consecutive_visits_without_event_index_capped_last_entry_reset_after_event_v1",
    "event_weights": "eligible_base_plus_growth_per_eligible_missed_visit_v1",
    "event_priority": "eligible_non_normal_priority_forces_opportunity_highest_higher_high_normal_weighted_ties_zero_total_uniform_v1",
    "event_cooldown": "selection_week_plus_source_cooldown_must_be_strictly_less_than_next_week_v1",
    "event_lifetime": "town_visit_and_next_departure_context_until_bound_return_v1",
    "transactions": "single_bound_return_no_policy_wait_no_preview_rng_or_asset_mutation_v1",
    "number_table_conflict": "select_base_count_if_and_only_if_courtyard_alternative_equals_base_plus_separate_courtyard_delta_v1",
    "district_requirement": "count_enabled_available_district_definitions_not_already_built_to_avoid_unlock_deadlock_v1",
    "quest_result_id": "plot_town_event_result_id_is_original_effect_type_not_event_definition_id_v1",
    "rng": "serialized_campaign_town_and_expedition_streams_not_original_seed_sequence_v1",
    "mission_return_types": "complete_abandon_regroup_wipe_independent_of_week_progression_v1",
    "service_free_uses": "consume_only_successful_purchase_never_restore_on_treatment_cancel_v1",
    "revival": "sample_source_count_dead_guids_once_keep_identity_level_quirks_history_reset_equipment_combat_ranks_and_temporary_states_v1",
    "revival_reference": "https://darkestdungeon.wiki.gg/wiki/Graveyard; HP/stress and camping retention remain uncalibrated candidates",
    "event_choice": "paid_or_revival_choices_are_atomic_decline_allowed_dismiss_and_unequip_allowed_without_reroll_v1",
    "event_recruits": "normal_source_class_deck_event_bonus_offers_last_this_visit_paid_quirked_recruit_immediately_joins_after_capacity_check_v1",
    "event_disease_heal": "source_count_random_distinct_diseases_per_living_hero_including_curse_per_Town_Events_wiki_v1",
    "cumulative_deaths": "each_death_occurrence_counts_revival_never_erases_history_v1",
    "town_contagion": "freeze_living_contacts_before_treatment_most_specific_activity_else_building_else_idle_additive_resistance_then_multiply_chance_clamp_ceiling_roster_cap_no_same_week_chain_v1",
    "idle_events": "apply_to_living_non_departing_non_job_non_missing_roster_at_next_return_raise_level_to_source_threshold_capped_source_max_remove_random_negative_including_locked_v1",
    "estate_currency": "map_source_currency_id_to_unique_inventory_kind_id_campaign_import_normalizes_016_alias_only_v1",
    "building_unlock": "campaign_uses_completed_quests_for_source_number_of_quests_finished_independent_from_returns_v1",
    "goal_groups": "cartesian_combine_all_and_region_group_alternatives_preserve_order_dedup_ids_v1",
    "board_sampling": "desired_source_visit_index_capped_last_plus_distinct_dlc_deltas_source_region_cap_guarantee_each_open_region_then_uniform_eligible_region_weighted_source_type_difficulty_weight_by_living_roster_bucket_count_v1",
    "board_rewards": "draw_once_when_board_instance_created_store_source_goal_and_rewards_preview_and_party_changes_never_reroll_v1",
    "board_shortage": "report_source_cap_or_eligibility_shortfall_do_not_invent_new_quest_types_or_counts_v1",
    "curio_summon_singleton": "only_original_egg_cluster_1_and_infested_egg_cluster_1_missing_single_result_weight_is_one_v1",
    "curio_native_effects": "validate_curio_marker_then_use_native_operations_no_battle_or_skill_event_synthetic_wrap_explicit_duration_preserved_missing_buff_expiry_quest_end_v1",
    "campaign_failure": "after_bound_return_at_or_above_effective_mode_week_or_cumulative_death_limit_death_priority_before_town_choices_final_plot_completion_disables_both_v1",
    "clock_ledger": "derive_week_completions_and_cumulative_deaths_from_once_bound_return_kind_progression_and_death_receipts_v1",
    "campaign_encounter_pools": "retain_all_positive_source_room_and_hall_candidates_with_conditions_for_runtime_owner_never_apply_fixed_opening_exclusions_v1",
    "hazard_variations": "source_inheritance_then_greatest_difficulty_variation_not_above_quest_level_single_override_v1",
    "empty_monster_loot": "only_explicit_empty_code_and_empty_count_is_inactive_with_source_note_campaign_only_v1",
    "loot_infestation_context": "same_loot_id_distinct_explicit_infestation_sequence_conditions_are_variants_not_source_order_overrides_same_condition_conflicts_stay_errors_v1",
    "map_binding_seed": "freeze_hidden_seed_once_per_board_instance_keep_active_persistent_instance_unchanged_v1",
    "source_identity": "sha256_enabled_recognized_combat_and_campaign_text_versions_and_content_config_no_executable_media_or_player_saves_v1",
    "named_formations": "source_region_and_difficulty_named_rows_are_weighted_alternatives_exact_duplicates_dedup_same_composition_conflicting_weights_error_v1",
    "prop_pool_types": "select_source_row_by_weight_split_multiple_types_uniformly_within_row_v1",
    "enabled_shared_courtyard_sources": "017_overlay_view_registers_enabled_crimson_court_root_curios_and_package_mode_curios_using_005_resolution_disabled_dlc_never_read_legacy_view_unchanged_v1",
    "unlabeled_quest_curios": "bind_only_when_one_source_curio_goal_and_exact_marked_node_count_preserve_original_csv_marker_in_metadata_otherwise_error_v1",
    "curio_singleton_missing_weight": "only_original_egg_cluster_summons_and_cc_food_cart_stress_eater_single_result_missing_weight_is_one_other_missing_weights_error_v1",
    "district_blueprint_loot": "017_only_select_enabled_districts_blueprint_overrides_exact_BLUEPRINT_0_empty_region_preserve_default_conflict_otherwise_v1",
    "boss_formations": "bind_quest_kill_monster_marker_to_original_positive_boss_miniboss_local_miniboss_rows_matching_goal_monster_classes_retain_complete_composition_v1",
    "generic_executor": "optional_017_owner_reuses_015_field_actions_native_combat_and_inventory_legacy_profiles_and_branches_unchanged_v1",
    "quest_restrictions": "index_resolve_level_threshold_array_by_quest_difficulty_comment_about_columns_not_applicable_to_actual_flat_array_event_exemption_explicit_v1",
    "random_quest_flags": "generated_ordinary_quests_missing_flags_candidate_enable_stall_surprise_scouting_plot_flags_preserved_v1",
    "provisions": "source_length_class_and_goal_grants_occupy_bag_store_prices_frozen_at_preparation_cancel_refunds_only_paid_gold_discards_no_estate_gain_attempt_dependencies_charged_only_on_embark_v1",
    "quest_objective_items": "gather_total_divided_by_exact_source_marked_container_count_activate_single_starting_item_total_divided_by_source_goal_count_require_exact_integer_ratio_v1",
    "objective_completion": "require_actual_room_visit_clear_native_kill_bag_holdings_and_once_item_consumption_ending_is_separate_player_action_no_external_success_boolean_v1",
    "physical_map_keys": "map_layer_auto_awards_are_replaced_from_actual_finite_bag_pickup_not_curio_resolution_unlock_consumes_one_physical_item_per_shared_gate_group_v1",
    "stall_formations": "choose_source_stall_row_once_per_battle_weighted_preserve_native_stall_thresholds_summon_rules_and_no_loot_flag_v1",
    "battle_loot": "on_heroes_win_only_actual_native_kills_with_loot_enabled_use_final_form_original_loot_no_rewards_for_stall_no_loot_summons_retreat_or_wipe_v1",
    "partial_retreat": "retain_concrete_formation_choice_unresolved_encounter_restarts_enemy_hp_corpses_on_reentry_no_partial_loot_keep_actual_kill_objective_receipts_v1",
    "field_party_order": "017_exports_native_field_reaction_movement_order_as_well_as_status_original_015_branch_unchanged_v1",
    "week_progression_gates": "completion_or_any_original_town_progression_goal_activity_checked_before_return_penalty_no_empty_wait_market_treatment_event_board_refresh_survival_has_no_execution_entry_v1",
    "weekly_clock": "017_treatment_absence_and_idle_availability_use_campaign_week_returns_are_separate_legacy_016_uses_visit_v1",
    "return_deaths": "collect_new_deaths_of_previously_living_roster_after_field_and_town_week_effects_once_cumulative_failures_include_town_deaths_v1",
    "quest_reward_region": "017_random_and_plot_board_trinkets_use_original_quest_dungeon_for_origin_dungeon_eligibility_town_wagon_legacy_empty_region_unchanged_v1",
    "camping_probability_groups": "same_original_chance_code_is_one_weighted_alternative_per_target_distinct_codes_independent_except_exact_field_dressing_two_same_selection_heals_sum_one_are_alternatives_source_codes_retained_v1",
    "camping_probabilities_reference": "https://darkestdungeon.wiki.gg/wiki/Arbalest_(Darkest_Dungeon); source Field Dressing weights 0.75/0.25 override summary page 0.5/0.5 description; group interpretation remains uncalibrated",
    "camping_source_scope": "strict_original_enabled_town_camping_library_class_scopes_unknown_operations_requirements_or_mixed_chance_targets_error_v1",
    "camping_phase": "finite_firewood_cleared_non_entrance_room_source_meal_food_cost_ceil_party_sum_refusal_uses_starvation_meal_only_accepted_rations_consumed_skill_points_and_uses_spent_on_attempt_one_companion_excludes_actor_party_other_also_excludes_selected_companion_v1",
    "camping_modifiers": "in_camp_predicate_resolved_only_in_explicit_camp_kernel_food_meals_use_eat_healing_camping_eat_stress_hunger_damage_modifiers_skills_use_actor_healing_and_recipient_recovery_modifiers_v1",
    "camping_buff_lifetime": "source_combat_end_decrement_every_real_native_terminal_encounter_including_retreat_no_decay_on_field_steps_native_tagged_timed_buff_projection_preserves_dispels_and_stacking_owner_tracks_duration_existing_hp_clamps_after_expiry_v1",
    "camping_nightmare": "enabled_region_difficulty_source_named_formations_only_shared_hero_class_progress_one_triggered_nightmare_per_raid_warning_after_meal_replace_ambush_cannot_prevent_native_results_only_loot_finite_bag_trinket_auto_estate_completed_chain_buffs_source_future_camps_v1",
    "camping_ambush": "source_chance_minus_skill_reduction_ordinary_eligible_hall_formations_can_be_ambush_false_excluded_bloodlight_preserved_other_light_source_delta_source_ambush_surprise_v1",
    "district_consumers": "017_consumes_all_016_field_curio_camping_hunger_stacking_supplies_replacement_light_loot_domains_source_passives_use_build_scope_not_temporary_carry_supply_grants_consumed_once_only_on_actual_embark_v1",
    "camp_bonfire": "matching_party_tags_grant_each_original_camp_bonus_once_not_per_matching_hero_points_frozen_for_camp_participants_v1",
    "persistent_world": "same_plot_instance_seed_bound_source_map_choices_rng_curios_goals_fog_keys_doors_prisoners_and_last_position_preserved_party_and_temporary_camp_state_replaced_native_unresolved_enemy_hp_and_corpses_restart_on_reentry_v1",
    "persistent_keys": "only_actual_bag_keys_enter_per_plot_ledger_on_regroup_reenter_finite_provision_bag_never_estate_cross_plot_inventory_unclaimed_discarded_or_whole_wipe_keys_are_lost_no_chest_respawn_or_free_replacement_completed_plot_keys_discarded_v1",
    "persistent_regroup": "source_regroup_stress_no_completion_xp_or_rewards_but_always_one_real_week_even_when_regular_empty_raid_progression_goals_unmet_v1",
    "courtyard_layout": "force_fog_in_017_persistent_court_only_entered_room_reveals_connected_hall_geometry_until_neighbor_rooms_preserve_layout_memory_contents_still_require_visit_or_scout_secret_links_revealed_only_by_critical_scout_within_source_room_hops_v1",
    "courtyard_walking": "original_cc_raid_rules_override_hallway_amount_chance_select_one_living_hero_per_hall_tile_native_stress_modifiers_and_resolve_preserved_v1",
    "prisoner_generation": "source_dungeons_shared_prisoner_spawn_difficulty_weighted_choice_then_uniform_enabled_matching_class_exact_source_quirk_skill_and_camp_ranges_source_rank_tables_once_offer_after_real_return_refresh_discards_unhired_offer_v1",
    "infestation_timing": "draw_source_inclusive_duration_once_on_entry_tick_once_per_advancing_week_current_zero_duration_waits_next_plot_event_dependency_completed_plot_entry_points_supersede_earlier_timers_source_clear_additional_mashes_clears_suppression_v1",
    "infestation_curios": "source_count_by_length_and_activity_after_validated_base_generation_place_without_replacing_existing_curios_original_generation_ledger_preserved_separately_for_derived_map_geometry_unchanged_references_strict_v1",
    "conditional_encounters": "eligible_original_conditional_rows_form_one_absolute_probability_replacement_distribution_plus_none_total_above_one_error_additional_rows_roll_independently_once_at_raid_entry_place_in_distinct_empty_corridors_no_placement_error_v1",
    "fanatic_suppression": "actual_native_flagged_kill_suppresses_all_flagged_variants_in_same_original_monster_family_until_source_sequence_clear_flag_v1",
    "quest_curse_cure": "source_completion_remove_vampire_or_linked_finished_plot_cure_story_effect_cures_all_living_roster_and_returned_heroes_before_town_week_normal_diseases_and_identity_preserved_story_display_later_idempotent_v1",
    "clinic_curse_unlock": "017_false_can_modify_for_contagious_vampire_may_be_overridden_only_when_nonempty_original_required_plot_quests_to_modify_all_completed_other_diseases_and_legacy_016_rules_unchanged_v1",
    "entry_difficulty": "017_once_each_actual_embark_native_raw_entry_stress_from_hero_difficulty_no_fake_battle_no_repeat_per_encounter_v1",
    "curio_scouting": "source_range_uses_room_hops_not_tiles_curio_scouts_do_not_reveal_secret_links_full_scout_clears_previous_partial_content_mask_v1",
    "town_departure_health": "017_living_town_heroes_start_full_current_native_regional_max_hp_after_equipment_training_growth_and_event_bonuses_not_stale_previous_result_hp_v1",
    "physical_equipment_loot": "017_native_victory_dead_equipment_enters_finite_loot_tray_with_guid_slot_origin_native_gains_keep_states_pickup_uses_one_inventory_slot_discard_or_unclaimed_lost_whole_wipe_no_carried_recovery_destroyed_items_not_lost_ledger_no_fresh_copies_v1",
    "event_plot_windows": "source_is_generated_by_event_one_advancing_return_window_is_started_by_event_persistent_until_complete_actual_attempt_not_ignore_source_ignore_upgrades_apply_once_before_town_weekly_markets_next_event_and_board_v1",
    "retention_rarity": "source_base_rarity_order_and_addon_insert_before_partial_constraints_compare_source_minimum_threshold_no_sibling_order_guess_highest_eligible_source_difficulty_only_count_all_lost_including_common_v1",
    "weekly_board_retirement": "on_actual_advanced_return_retire_previous_week_entries_and_selection_before_new_event_choice_preserve_active_persistent_instances_only_generate_new_board_once_after_choice_v1",
    "named_singleton_weight": "only_one_distinct_referenced_named_formation_may_omit_relative_chance_normalize_unit_choice_multiple_alternatives_with_missing_weight_error_v1",
    "plot_quirk_rewards": "source_chance_rows_are_relative_weights_one_row_per_living_departure_hero_apply_its_quirk_ids_after_normal_maturation_before_quest_item_effects_locked_capacity_rules_preserved_v1",
    "retention_rewards": "source_count_lost_physical_instances_uniform_without_replacement_frozen_at_weekly_board_generation_same_states_and_ids_return_once_complete_only_source_stealable_storage_excludes_protected_items_v1",
    "roster_after_building_loss": "017_source_building_downgrade_keeps_existing_roster_up_to_source_maximum_capacity_recruit_and_revive_still_require_current_unlocked_capacity_no_forced_dismissal_v1",
    "protected_assets": "candidate_source_award_categories_trophy_dd_and_enabled_shieldbreaker_rarity_native_field_loss_at_departure_week_town_activity_loss_at_effective_return_week_automatic_return_on_next_advanced_week_same_instance_not_in_stealable_or_recovery_pool_pending_original_calibration_v1",
    "quest_item_completion": "017_only_native_complete_once_after_equipment_return_rewards_and_quirk_maturation_before_town_week_finite_quest_budgets_persist_across_missions_all_enabled_equipped_survivors_charge_once_other_slot_blocker_protects_not_self_growth_on_physical_asset_source_random_quirk_weights_locked_rules_event_opportunity_not_specific_id_v1",
    "native_raid_quirks": "source_monster_loot_order_reversed_first_unowned_singleton_then_uniform_eligible_living_departure_hero_once_per_native_kill_even_if_quest_abandoned_no_award_without_survivor_or_exhausted_pool_auto_locked_no_stat_values_added_v1; https://darkestdungeon.wiki.gg/wiki/The_thing pending original calibration",
    "event_resource_gains": "017_bank_new_heirloom_gains_source_multiplier_and_shard_gains_one_plus_source_bonus_current_visit_scope_gold_and_direct_currency_transfers_refunds_exchange_not_multiplied_v1",
    "nightmare_journal_delivery": "017_exact_source_specific_page_index_with_auto_open_true_and_read_outside_of_dungeon_true_archived_as_direct_reward_on_native_success_no_bag_slot_or_repeat_page_source_completed_result_binding_v1"
  },
  "protected_award_categories": [
    "trophy",
    "dd"
  ],
  "protected_rarities": [
    "shieldbreaker"
  ],
  "parameters": {
    "revival_stress": 0
  }
}"###)
            .map_err(|e| e.to_string())
    }
    pub fn validate(&self) -> Result<()> {
        if self != &Self::documented()? {
            return Err("unknown/modified campaign candidates".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Data {
    #[serde(default)]
    pub rarities: Option<crate::retention017::Rarities>,
    pub content: ContentConfig,
    pub candidates: Candidates,
    pub events: BTreeMap<String, Value>,
    pub settings: BTreeMap<String, Value>,
    pub guarantees: BTreeMap<String, Value>,
    pub tables: BTreeMap<String, Value>,
    pub modifiers: BTreeMap<String, Value>,
    pub quests: BTreeMap<String, Value>,
    pub goals: BTreeMap<String, Value>,
    pub types: BTreeMap<String, Value>,
    pub goal_groups: BTreeMap<String, Value>,
    pub resolved_conflicts: BTreeMap<String, Value>,
    pub town_goal_ids: BTreeSet<String>,
    pub wave_modifiers: BTreeMap<String, Value>,
    #[serde(default)]
    pub prisoner: Value,
    pub infestation: Value,
    pub contagion: Value,
    pub exit: Value,
    pub restrictions: Value,
    pub sources: BTreeMap<String, Vec<String>>,
}
fn put(m: &mut BTreeMap<String, Value>, id: &str, v: Value) {
    library::insert_json(m, id.into(), v)
}
fn leaf(m: &mut BTreeMap<String, Value>, id: &str, v: Value) {
    put(m, id, json!({"value":v}))
}
impl Data {
    pub fn load(g: &GameData, candidates: Candidates) -> Result<Self> {
        Self::load_overlay(&g.overlay, candidates)
    }
    pub fn load_overlay(o: &Overlay, candidates: Candidates) -> Result<Self> {
        candidates.validate()?;
        let mut d = Self {
            rarities: crate::retention017::Rarities::load(o)?,
            content: o.config.clone(),
            candidates,
            events: BTreeMap::new(),
            settings: BTreeMap::new(),
            guarantees: BTreeMap::new(),
            tables: BTreeMap::new(),
            modifiers: BTreeMap::new(),
            quests: BTreeMap::new(),
            goals: BTreeMap::new(),
            types: BTreeMap::new(),
            goal_groups: BTreeMap::new(),
            resolved_conflicts: BTreeMap::new(),
            town_goal_ids: BTreeSet::new(),
            wave_modifiers: BTreeMap::new(),
            prisoner: Value::Null,
            infestation: Value::Null,
            contagion: Value::Null,
            exit: Value::Null,
            restrictions: Value::Null,
            sources: BTreeMap::new(),
        };
        for p in o
            .paths
            .keys()
            .filter(|p| p.starts_with("trinkets/") && p.ends_with(".rarities.trinkets.json"))
        {
            d.sources.insert(
                p.clone(),
                o.sources(p)
                    .iter()
                    .map(|i| o.layers[*i].id.clone())
                    .collect(),
            );
        }
        for p in o.paths.keys().filter(|p| {
            (p.starts_with("campaign/town_events/")
                && !p.starts_with("campaign/town_events/arena.")
                || p.starts_with("campaign/quest/")
                || p.starts_with("campaign/progression/")
                || p.as_str() == "campaign/roster/roster.contagion.json"
                || p.as_str() == "dungeons/_shared/shared.json")
                && p.ends_with(".json")
        }) {
            d.sources.insert(
                p.clone(),
                o.sources(p)
                    .iter()
                    .map(|i| o.layers[*i].id.clone())
                    .collect(),
            );
            for bytes in o.read_campaign_versions(p)? {
                let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("{p}: {e}"))?;
                if p == "dungeons/_shared/shared.json" {
                    fields(&v, &["prisoner_spawn_data"], "shared dungeon spawning")?;
                    fields(
                        &v["prisoner_spawn_data"],
                        &["hero_spawn_difficulty_table"],
                        "prisoner spawn table",
                    )?;
                    merge_single(
                        &mut d.prisoner,
                        &v["prisoner_spawn_data"],
                        "prisoner spawning",
                    )?;
                } else if p.starts_with("campaign/town_events/") {
                    fields(
                        &v,
                        &["events", "settings", "quest_type_event_guarantees"],
                        "town event document",
                    )?;
                    if let Some(rows) = v["events"].as_array() {
                        for raw in rows {
                            validate_event(raw)?;
                            let mut event = raw.clone();
                            for k in [
                                "town_ambience_paramater_ids",
                                "tone",
                                "sprite",
                                "sprite_attachment",
                                "custom_frame_id",
                                "tutorial_ids",
                                "interaction_texture_id",
                                "interaction_click_sound",
                                "interaction_tooltip",
                            ] {
                                event.as_object_mut().unwrap().remove(k);
                            }
                            let id = event["id"].as_str().ok_or("event missing ID")?.to_string();
                            put(&mut d.events, &id, event);
                        }
                    }
                    if let Some(rows) = v["settings"].as_array() {
                        for row in rows {
                            fields(
                                row,
                                &["id", "event_chance_per_town_visits"],
                                "event setting",
                            )?;
                            put(
                                &mut d.settings,
                                row["id"].as_str().ok_or("setting missing ID")?,
                                row.clone(),
                            );
                        }
                    }
                    if let Some(rows) = v["quest_type_event_guarantees"].as_array() {
                        for row in rows {
                            fields(
                                row,
                                &["dungeon_type", "quest_type", "event_id"],
                                "quest event guarantee",
                            )?;
                            put(
                                &mut d.guarantees,
                                &format!(
                                    "{}:{}",
                                    row["dungeon_type"].as_str().ok_or("missing dungeon")?,
                                    row["quest_type"].as_str().ok_or("missing quest type")?
                                ),
                                row.clone(),
                            );
                        }
                    }
                } else if p.starts_with("campaign/quest/") {
                    fields(
                        &v,
                        &[
                            "generation",
                            "plot_quests",
                            "goals",
                            "town_progression_goal_ids",
                            "types",
                            "fail_penalty",
                            "regroup_penalty",
                            "restriction",
                            "modifiers",
                        ],
                        "quest document",
                    )?;
                    if let Some(rows) = v["modifiers"].as_array() {
                        for row in rows {
                            put(
                                &mut d.wave_modifiers,
                                row["id"].as_str().ok_or("missing wave modifier ID")?,
                                row.clone(),
                            );
                        }
                    }
                    if let Some(rows) = v["town_progression_goal_ids"].as_array() {
                        for id in rows {
                            d.town_goal_ids.insert(
                                id.as_str()
                                    .ok_or("invalid town progression goal ID")?
                                    .into(),
                            );
                        }
                    }
                    for (key, target) in [
                        ("plot_quests", &mut d.quests),
                        ("goals", &mut d.goals),
                        ("types", &mut d.types),
                    ] {
                        if let Some(rows) = v[key].as_array() {
                            for row in rows {
                                let id =
                                    row["id"].as_str().ok_or("quest library entry missing ID")?;
                                let mut functional = row.clone();
                                if key == "types" {
                                    if let Some(lists) = row["goal_lists"].as_array() {
                                        for list in lists {
                                            put(
                                                &mut d.goal_groups,
                                                &format!(
                                                    "{id}:{}",
                                                    list["dungeon"]
                                                        .as_str()
                                                        .ok_or("goal group dungeon missing")?
                                                ),
                                                json!({"groups":list["goals"]}),
                                            );
                                        }
                                    }
                                    functional.as_object_mut().unwrap().remove("goal_lists");
                                }
                                put(target, id, functional);
                            }
                        }
                    }
                    if let Some(groups) = v["generation"].as_object() {
                        for (group, values) in groups {
                            fields(
                                &json!({group:values}),
                                &["dungeon", "difficulty", "type", "rewards", "number"],
                                "quest generation group",
                            )?;
                            for (key, value) in
                                values.as_object().ok_or("invalid generation group")?
                            {
                                if key == "comment" {
                                    continue;
                                }
                                let id = format!("{group}.{key}");
                                match key.as_str() {
                                    "generated_dungeons"
                                    | "available_quests_table"
                                    | "heirloom_type_map"
                                    | "heirloom_amount_table"
                                    | "trinket_chance_table" => {
                                        for row in value
                                            .as_array()
                                            .ok_or("invalid keyed generation rows")?
                                        {
                                            let logical = match key.as_str() {
                                                "generated_dungeons" => row["id"].as_str(),
                                                "available_quests_table" | "heirloom_type_map" => {
                                                    row["dungeon"].as_str()
                                                }
                                                "heirloom_amount_table" => row["type"].as_str(),
                                                _ => row["rarity"].as_str(),
                                            }
                                            .ok_or("missing generation logical ID")?;
                                            let suffix = if key == "trinket_chance_table" {
                                                format!(
                                                    "{logical}:{}",
                                                    row["dungeon"].as_str().unwrap_or("*")
                                                )
                                            } else {
                                                logical.into()
                                            };
                                            let mut functional = row.clone();
                                            functional.as_object_mut().unwrap().remove("comment");
                                            put(
                                                &mut d.tables,
                                                &format!("{id}:{suffix}"),
                                                functional,
                                            );
                                        }
                                    }
                                    "number_of_quests_per_town_visit_table_modifier" => {
                                        leaf(&mut d.modifiers, &format!("{p}:{id}"), value.clone())
                                    }
                                    "generated_quests_max_threshold"
                                    | "generated_resolve_level_difficulties"
                                    | "item_table"
                                    | "resolve_xp_table"
                                    | "number_of_quests_per_town_visit_table" => {
                                        leaf(&mut d.tables, &id, value.clone())
                                    }
                                    _ => return Err(format!("unsupported generation table {id}")),
                                }
                            }
                        }
                    }
                    if v.get("fail_penalty").is_some() {
                        merge_single(&mut d.exit, &v, "quest exit penalties")?;
                    }
                    if v.get("restriction").is_some() {
                        merge_single(&mut d.restrictions, &v, "quest restrictions")?;
                    }
                } else if p == "campaign/roster/roster.contagion.json" {
                    fields(&v, &["idle", "building", "activity"], "town contagion")?;
                    for key in ["idle", "building", "activity"] {
                        for row in array(&v, key)? {
                            fields(
                                row,
                                &[
                                    "quirk_tag",
                                    "base_contraction_chance",
                                    "disease_resistance_addition_mod",
                                    "disease_resistance_multiplication_mod",
                                    "contraction_chance_per_infected_hero",
                                    "min_chance",
                                    "max_chance",
                                    "max_contraction_percent",
                                    "roster_infected_limit_percent",
                                ],
                                "town contagion row",
                            )?;
                            if row["quirk_tag"].as_str() != Some("vampire") {
                                return Err("unknown town contagion tag".into());
                            }
                            for k in [
                                "base_contraction_chance",
                                "disease_resistance_addition_mod",
                                "disease_resistance_multiplication_mod",
                                "contraction_chance_per_infected_hero",
                                "min_chance",
                                "max_chance",
                                "max_contraction_percent",
                                "roster_infected_limit_percent",
                            ] {
                                if !row[k].as_f64().is_some_and(f64::is_finite) {
                                    return Err(format!("invalid town contagion number {k}"));
                                }
                            }
                        }
                    }
                    merge_single(&mut d.contagion, &v, "town contagion")?;
                } else if p.ends_with("infestation.progression.json") {
                    fields(&v, &["infestation"], "infestation document")?;
                    merge_single(&mut d.infestation, &v["infestation"], "infestation")?;
                } else if p.ends_with("progression.json") {
                    fields(&v, &["dungeon"], "progression")?;
                    for (k, v) in v["dungeon"].as_object().ok_or("missing progression")? {
                        leaf(&mut d.tables, &format!("progression.{k}"), v.clone());
                    }
                } else {
                    return Err(format!("unsupported campaign source {p}"));
                }
            }
        }
        let key = "number.number_of_quests_per_town_visit_table";
        if d.tables.get(key).is_some_and(Value::is_array) {
            let path = "campaign/quest/number.quest.generation.json";
            let base = o
                .layers
                .iter()
                .position(|l| l.id == "base")
                .ok_or("missing explicit base count source")?;
            let raw: Value = serde_json::from_slice(&o.read_campaign_source(path, base)?)
                .map_err(|e| e.to_string())?;
            let original = &raw["generation"]["number"]["number_of_quests_per_town_visit_table"];
            let modifier=&d.modifiers.get("campaign/quest/crimson_court.quest.generation.json:number.number_of_quests_per_town_visit_table_modifier").ok_or("missing courtyard count delta for explicit selection")?["value"];
            let a = original
                .as_array()
                .ok_or("invalid original quest count table")?;
            let b = modifier
                .as_array()
                .filter(|b| b.len() == a.len())
                .ok_or("invalid courtyard count delta")?;
            let sum = a
                .iter()
                .zip(b)
                .map(|(a, b)| {
                    Ok(json!(
                        a.as_u64()
                            .ok_or("invalid quest count")?
                            .checked_add(b.as_u64().ok_or("invalid quest count delta")?)
                            .ok_or("quest count overflow")?
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            let variants = d.tables[key].as_array().unwrap();
            if variants.len() != 2
                || !variants.contains(&json!({"value":original}))
                || !variants.contains(&json!({"value":sum}))
            {
                return Err(
                    "unrecognized quest-count conflict; explicit candidate does not apply".into(),
                );
            }
            d.resolved_conflicts.insert(key.into(),json!({"alternatives":d.tables[key],"selection":"base_plus_distinct_modifiers","reason":"courtyard full table already includes its separate delta; avoid double count","source_layer":"base"}));
            leaf_replace(&mut d.tables, key, original.clone());
        }
        d.validate_field_campaign_sources()?;
        Ok(d)
    }
    pub fn event(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.events, id, "town event")
    }
    pub fn table(&self, id: &str) -> Result<&Value> {
        library::unique_json(&self.tables, id, "campaign table")
    }
    pub fn value(&self, id: &str) -> Result<&Value> {
        Ok(&self.table(id)?["value"])
    }
    fn validate_field_campaign_sources(&self) -> Result<()> {
        if !self.infestation.is_null() {
            fields(
                &self.infestation,
                &["activity_levels", "sequence_elements"],
                "infestation",
            )?;
            let mut activities = BTreeSet::new();
            for row in array(&self.infestation, "activity_levels")? {
                fields(
                    row,
                    &[
                        "id",
                        "infestation_curio",
                        "short_curio_count",
                        "med_curio_count",
                        "long_curio_count",
                        "dungeons",
                    ],
                    "infestation activity",
                )?;
                let id = row["id"].as_str().ok_or("missing activity ID")?;
                if !["none", "low", "medium", "high"].contains(&id) || !activities.insert(id) {
                    return Err("unknown/duplicate infestation activity".into());
                }
                if row["infestation_curio"].as_str().is_none_or(str::is_empty) {
                    return Err("missing infestation curio".into());
                }
                for key in ["short_curio_count", "med_curio_count", "long_curio_count"] {
                    let a = array(row, key)?;
                    if a.len() != 2
                        || a[0]
                            .as_u64()
                            .zip(a[1].as_u64())
                            .is_none_or(|(lo, hi)| lo > hi || hi > 10000)
                    {
                        return Err("invalid infestation curio count range".into());
                    }
                }
                for region in array(row, "dungeons")? {
                    if !["crypts", "weald", "warrens", "cove"]
                        .iter()
                        .any(|id| region == id)
                    {
                        return Err("unsupported infestation region".into());
                    }
                }
            }
            let mut ids = BTreeSet::new();
            for row in array(&self.infestation, "sequence_elements")? {
                fields(
                    row,
                    &[
                        "id",
                        "plot_quest_dependency",
                        "town_event_dependency",
                        "activity_level",
                        "minimum_duration_week",
                        "maximum_duration_week",
                        "show_ui",
                        "clear_additional_mashes",
                    ],
                    "infestation sequence",
                )?;
                let id = row["id"]
                    .as_str()
                    .ok_or("missing infestation sequence ID")?;
                if !ids.insert(id)
                    || !row["activity_level"]
                        .as_str()
                        .is_some_and(|id| activities.contains(id))
                    || uint(row, "minimum_duration_week")? > uint(row, "maximum_duration_week")?
                {
                    return Err("invalid infestation sequence reference/timing".into());
                }
                for field in ["show_ui", "clear_additional_mashes"] {
                    if !row[field].is_boolean() {
                        return Err("invalid infestation flag".into());
                    }
                }
                for (field, table) in [
                    ("plot_quest_dependency", &self.quests),
                    ("town_event_dependency", &self.events),
                ] {
                    let id = row[field]
                        .as_str()
                        .ok_or("missing infestation dependency")?;
                    if !id.is_empty() {
                        library::unique_json(table, id, "infestation dependency")?;
                    }
                }
            }
        }
        if !self.prisoner.is_null() {
            fields(
                &self.prisoner,
                &["hero_spawn_difficulty_table"],
                "prisoner spawning",
            )?;
            let mut keys = BTreeSet::new();
            for row in array(&self.prisoner, "hero_spawn_difficulty_table")? {
                fields(row, &["key", "data"], "prisoner difficulty")?;
                let key = uint(row, "key")?;
                if ![1, 3, 5, 6].contains(&key) || !keys.insert(key) {
                    return Err("unsupported/duplicate prisoner difficulty".into());
                }
                for choice in array(row, "data")? {
                    fields(choice, &["chance", "value"], "prisoner choice")?;
                    if choice["chance"]
                        .as_f64()
                        .is_none_or(|w| !w.is_finite() || w < 0.)
                    {
                        return Err("invalid prisoner choice weight".into());
                    }
                    let v = &choice["value"];
                    fields(
                        v,
                        &[
                            "level",
                            "hero_type_tags",
                            "starting_positive_quirks",
                            "starting_negative_quirks",
                            "starting_camping_skills",
                            "starting_shared_camping_skills",
                            "starting_combat_skills",
                        ],
                        "prisoner generation",
                    )?;
                    uint(v, "level")?;
                    array(v, "hero_type_tags")?;
                    for k in [
                        "starting_positive_quirks",
                        "starting_negative_quirks",
                        "starting_camping_skills",
                        "starting_shared_camping_skills",
                        "starting_combat_skills",
                    ] {
                        fields(&v[k], &["min", "max"], "prisoner count")?;
                        if uint(&v[k], "min")? > uint(&v[k], "max")? || uint(&v[k], "max")? > 100 {
                            return Err("invalid prisoner count range".into());
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn audit(&self) -> Value {
        let errors = self
            .events
            .keys()
            .filter_map(|id| self.event(id).err().map(|e| (id, e)))
            .collect::<BTreeMap<_, _>>();
        let effects = self
            .events
            .values()
            .filter_map(|v| v["data"].as_array())
            .flatten()
            .filter_map(|v| v["type"].as_str())
            .collect::<BTreeSet<_>>();
        let conflicts = self
            .tables
            .iter()
            .filter(|(_, v)| v.is_array())
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        json!({"profile":PROFILE,"content":self.content,"candidates":self.candidates,"counts":{"events":self.events.len(),"effect_types":effects.len(),"plot_quests":self.quests.len(),"goals":self.goals.len(),"tables":self.tables.len()},"events":self.events,"event_errors":errors,"effect_types":effects,"settings":self.settings,"guarantees":self.guarantees,"tables":self.tables,"table_conflicts":conflicts,"resolved_conflicts":self.resolved_conflicts,"goal_groups":self.goal_groups,"wave_modifiers":self.wave_modifiers,"town_goal_ids":self.town_goal_ids,"modifiers":self.modifiers,"quests":self.quests,"goals":self.goals,"types":self.types,"infestation":self.infestation,"rarities":self.rarities.as_ref().map(|r|r.audit()),"prisoner_spawn":self.prisoner,"contagion":self.contagion,"exit":self.exit,"restrictions":self.restrictions,"sources":self.sources})
    }
}
fn leaf_replace(m: &mut BTreeMap<String, Value>, id: &str, v: Value) {
    m.insert(id.into(), json!({"value":v}));
}
fn merge_single(target: &mut Value, v: &Value, kind: &str) -> Result<()> {
    if !target.is_null() && target != v {
        return Err(format!("ambiguous {kind}"));
    }
    *target = v.clone();
    Ok(())
}
pub fn validate_event(v: &Value) -> Result<()> {
    fields(
        v,
        &[
            "id",
            "base_chance",
            "per_not_rolled_additional_chance",
            "cooldown",
            "requirements",
            "data",
            "is_unique",
            "priority",
            "town_ambience_paramater_ids",
            "tone",
            "sprite",
            "sprite_attachment",
            "custom_frame_id",
            "tutorial_ids",
            "interaction_texture_id",
            "interaction_click_sound",
            "interaction_tooltip",
        ],
        "town event",
    )?;
    v["id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("event needs ID")?;
    for k in ["base_chance", "per_not_rolled_additional_chance"] {
        if !v[k].as_f64().is_some_and(|x| x.is_finite() && x >= 0.) {
            return Err(format!("invalid event weight {k}"));
        }
    }
    if v.get("is_unique").is_some_and(|v| !v.is_boolean()) {
        return Err("invalid event unique flag".into());
    }
    uint(v, "cooldown")?;
    if let Some(p) = v.get("priority")
        && !["normal", "high", "higher", "highest"]
            .contains(&p.as_str().ok_or("invalid event priority")?)
    {
        return Err("unsupported event priority".into());
    }
    fields(
        &v["requirements"],
        &[
            "minimum_week",
            "dead_heroes",
            "hero_level_counts",
            "upgrades_purchased",
            "trinket_storage_count",
            "available_plot_quest",
            "finished_plot_quests",
            "infestation_sequences",
            "minimum_number_of_district_buildings",
            "building_unlocked",
        ],
        "event requirements",
    )?;
    for key in ["minimum_week", "dead_heroes", "trinket_storage_count"] {
        uint(&v["requirements"], key)?;
    }
    for key in ["available_plot_quest", "building_unlocked"] {
        if v["requirements"].get(key).is_some_and(|v| !v.is_string()) {
            return Err(format!("invalid event requirement {key}"));
        }
    }
    for key in [
        "hero_level_counts",
        "upgrades_purchased",
        "finished_plot_quests",
        "infestation_sequences",
    ] {
        if v["requirements"].get(key).is_some_and(|v| !v.is_array()) {
            return Err(format!("invalid event requirement {key}"));
        }
    }
    for key in ["hero_level_counts", "upgrades_purchased"] {
        for row in array(&v["requirements"], key)? {
            if key == "hero_level_counts" {
                fields(row, &["level", "count"], "hero count requirement")?;
                uint(row, "level")?;
                uint(row, "count")?;
            } else {
                fields(row, &["tree_id", "requirement_code"], "upgrade requirement")?;
                for field in ["tree_id", "requirement_code"] {
                    row[field]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or("invalid upgrade requirement identifier")?;
                }
            }
        }
    }
    for key in ["finished_plot_quests", "infestation_sequences"] {
        if let Some(rows) = v["requirements"][key].as_array()
            && rows.iter().any(|v| v.as_str().is_none_or(str::is_empty))
        {
            return Err(format!("invalid event requirement IDs {key}"));
        }
    }
    if v["requirements"]
        .get("minimum_number_of_district_buildings")
        .is_some()
    {
        uint(&v["requirements"], "minimum_number_of_district_buildings")?;
    }
    for row in array(v, "data")? {
        fields(
            row,
            &["type", "string_data", "number_data"],
            "town event effect",
        )?;
        let kind = row["type"].as_str().ok_or("missing event effect type")?;
        if !EFFECTS.contains(&kind) {
            return Err(format!("unsupported town event effect {kind}"));
        }
        row["string_data"]
            .as_str()
            .ok_or("invalid event effect string")?;
        if !row["number_data"].as_f64().is_some_and(f64::is_finite) {
            return Err("invalid event effect amount".into());
        }
    }
    Ok(())
}
