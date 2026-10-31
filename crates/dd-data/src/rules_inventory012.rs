//! Every original global rule is assigned explicitly. Uncertain legacy fields
//! stay unresolved; a field name is never evidence that it is outside combat.
use crate::combat::{Result, Rules};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleUse {
    pub scope: String,
    pub binding: String,
    pub value: Value,
}
pub fn inventory(raw: &Value, rules: &Rules) -> Result<BTreeMap<String, RuleUse>> {
    let parsed = serde_json::to_value(rules).map_err(|e| e.to_string())?;
    let mut rows = BTreeMap::new();
    for (key, value) in raw.as_object().ok_or("global rules must be object")? {
        if [
            "candidates",
            "content",
            "context",
            "lighting",
            "bloodlight",
            "farm",
            "flashback",
            "globals",
            "encounter_contract",
        ]
        .contains(&key.as_str())
        {
            return Err(format!(
                "host-only metadata is not an original game rule: {key}"
            ));
        }
        let (scope, binding) = if [
            "excessCritMult",
            "exessCritMult",
            "excessCritMax",
            "stun_onset_buffs",
            "stun_onset_buff_consecutive_applies",
            "stun_ends_turn",
            "affliction_severity_table",
        ]
        .contains(&key.as_str())
        {
            (
                "validated_inert",
                "Rules::validate rejects active unsupported variants; original source switch/table is inert",
            )
        } else if [
            "deactivation_chance_small",
            "deactivation_chance_med",
            "min_hero_count_for_escape",
        ]
        .contains(&key.as_str())
        {
            if rules.legacy_host_inputs().is_some() {
                (
                    "candidate_host_input",
                    "explicit user-approved outside_combat_v1 ownership candidate; source values in Rules/checkpoint/BattleResult; original trigger unverified",
                )
            } else {
                (
                    "unresolved",
                    "original value retained; explicit legacy_global_boundary choice required to assign host ownership; no execution claim",
                )
            }
        } else if parsed.get(key).is_some() || key == "stun_recovery_buffs" {
            (
                "battle",
                "typed Rules + combat/resolve/trait/item/difficulty interpreter",
            )
        } else if [
            "effectiveDifficultyMin",
            "effectiveDifficultyMax",
            "effectiveDifficultyDungeonStartingStress",
            "effectiveDifficultyStressDmgModifiers",
            "effectiveDifficultyAfflictionOnsetModifiers",
        ]
        .contains(&key.as_str())
        {
            (
                "host_battle_input",
                "hero_at_difficulty; source tables, explicit resolve/quest levels and raid-entry boundary",
            )
        } else if key == "darkness" {
            (
                "battle",
                "GameData::lighting + typed thresholds and combat modifiers",
            )
        } else if [
            "stall_party_size_threshold",
            "stall_enemy_size_threshold",
            "stall_number_of_invalidating_skills_threshold",
            "stall_round_stress_effect_threshold",
            "stall_round_summon_effect_threshold",
            "stall_round_reset_threshold",
            "stall_extra_round_size_threshold",
            "stall_extra_round_size_amount",
            "stall_stress_effect_name",
            "stall_round_accelerated_only_consecutive_increment",
            "stall_round_accelerated_only_increment",
        ]
        .contains(&key.as_str())
        {
            (
                "battle",
                "GameData::battle_context + typed consecutive stalling controller",
            )
        } else if [
            "ambush_camping_base_chance",
            "ambush_torch_reduction",
            "camp_relieve_stress",
            "camp_rest_number_of_hero_threshold",
            "camp_rest_point_threshold",
            "camp_restore_torch",
            "camp_start_camping_points",
            "corridor_return_content",
            "curioTriggerChance_high",
            "curioTriggerChance_low",
            "curioTriggerChance_med",
            "difficulty_trap_base",
            "disease_after_quest_min_chance",
            "disease_after_quest_min_resolve_level",
            "disease_hero_disease_resist_weight",
            "disease_max_chance",
            "dismissed_hero_stress_penalties",
            "dotHallwayStepRatio",
            "effectiveDifficultyBarkStress",
            "hallway_hunger_HPrestore",
            "hallway_hunger_starve_HPdmg",
            "hallway_stress",
            "m_Deceleration",
            "m_ForwardAcceleration",
            "m_InteractionAreaPixelHeight",
            "m_InteractionAreaPixelWidth",
            "m_MaxForwardSpeed",
            "m_MaxReverseSpeed",
            "m_ReverseAcceleration",
            "masterBarkChance",
            "max_number_of_camping_skills",
            "max_provisions_before_full",
            "meals_table",
            "min_available_activity_slots_for_crier",
            "negStress0Failure",
            "negStress0Success",
            "negStress100Failure",
            "negStress100Success",
            "new_game_plus_hero_death_limit",
            "new_game_plus_week_limit",
            "playerCritBarkChance",
            "posStress0Failure",
            "posStress0Success",
            "posStress100Failure",
            "posStress100Success",
            "provision_hp_heal",
            "quirk_chance_to_lock_negative",
            "quirk_negative_locked_after_turn_count",
            "quirks_max_locked_negative",
            "quirks_max_locked_positive",
            "scouting_chance_base",
            "scouting_chance_scout_treasure",
            "scouting_crit_success",
            "scouting_enter_dungeon_quest_item_scout_chance",
            "scouting_enter_dungeon_scout_chance",
            "surprise_ambush_monsters_base_chance",
            "surprise_ambush_party_base_chance",
            "surprise_corridor_monsters_base_chance",
            "surprise_corridor_party_base_chance",
            "surprise_known_corridor_monsters_base_chance",
            "surprise_known_corridor_party_base_chance",
            "surprise_known_room_monsters_base_chance",
            "surprise_known_room_party_base_chance",
            "surprise_max_monsters_surprised_chance",
            "surprise_max_party_surprised_chance",
            "surprise_room_monsters_base_chance",
            "surprise_room_party_base_chance",
            "tile_light_loss",
            "trap_scout_disarm_bonus",
            "trinkets_equipped_warning_dungeon_min_difficulty",
            "trinkets_equipped_warning_min_percent",
        ]
        .contains(&key.as_str())
        {
            (
                "external",
                "explicit exploration/camp/town/campaign/generation/presentation input; battle receives resolved host context",
            )
        } else {
            return Err(format!("unassigned global rule {key}"));
        };
        rows.insert(
            key.clone(),
            RuleUse {
                scope: scope.into(),
                binding: binding.into(),
                value: value.clone(),
            },
        );
    }
    Ok(rows)
}
