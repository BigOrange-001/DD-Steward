//! Exact known fields for simple unit records that otherwise only read selected
//! scalar parameters. Complex skills, Boss contracts and lifecycles retain their
//! own strict interpreters. This is a schema check, not an execution claim.
use crate::{combat::Result, darkest::Document};
use std::collections::BTreeSet;
pub(crate) fn unit_fields(d: &Document) -> Result<()> {
    for r in &d.records {
        let fields: &[&str] = match r.key.as_str() {
            "display" => &["size"],
            "stats" => &[
                "hp",
                "def",
                "prot",
                "spd",
                "stun_resist",
                "bleed_resist",
                "poison_resist",
                "debuff_resist",
                "move_resist",
            ],
            "weapon" => &[
                "name",
                "atk",
                "crit",
                "dmg",
                "spd",
                "upgradeRequirementCode",
            ],
            "armour" => &["name", "def", "prot", "hp", "spd", "upgradeRequirementCode"],
            "resistances" => &[
                "stun",
                "bleed",
                "poison",
                "debuff",
                "move",
                "disease",
                "death_blow",
                "trap",
            ],
            "personality" => &["prefskill"],
            "enemy_type" => &["id"],
            "initiative" => &["hide_indicator", "number_of_turns_per_round"],
            "loot" => &["code", "count", "raid_finish_quirk_class_id"],
            "generation" => &[
                "card_chance",
                "is_generation_enabled",
                "number_of_cards_in_deck",
                "number_of_class_specific_camping_skills",
                "number_of_shared_camping_skills",
                "number_of_negative_quirks_max",
                "number_of_negative_quirks_min",
                "number_of_positive_quirks_max",
                "number_of_positive_quirks_min",
                "number_of_random_combat_skills",
                "town_event_dependency",
            ],
            _ => continue,
        };
        let mut seen = BTreeSet::new();
        for p in &r.params {
            if !fields.contains(&p.name.as_str()) {
                return Err(format!("unknown unit field {}.{}", r.key, p.name));
            }
            if !seen.insert(&p.name) {
                return Err(format!("duplicate unit field {}.{}", r.key, p.name));
            }
        }
    }
    Ok(())
}
pub(crate) fn quirk_fields(value: &serde_json::Value) -> Result<()> {
    let object = value
        .as_object()
        .ok_or("quirk definition must be an object")?;
    for key in object.keys() {
        if ![
            "analytics_enabled",
            "buffs",
            "can_be_replaced_by_new_quirk",
            "can_modify_in_activity",
            "can_remove_with_camping_skill",
            "classification",
            "contagious_class_id",
            "curio_tag",
            "curio_tag_chance",
            "evolution_causes_death",
            "evolution_class_id",
            "evolution_duration_max",
            "evolution_duration_min",
            "evolution_town_attempt_use_item_duration_threshold",
            "evolution_town_progression_duration_change",
            "has_fx",
            "id",
            "incompatible_quirks",
            "is_disease",
            "is_explicit_buff_description_valid_in_character_sheet",
            "is_flavor_description_valid_in_tray_icon",
            "is_positive",
            "keep_loot",
            "random_chance",
            "required_plot_quests_to_embark",
            "required_plot_quests_to_modify",
            "roster_limit",
            "show_explicit_buff_description",
            "show_explicit_curio_tag_description",
            "show_flavor_description",
            "slot_size",
            "tags",
            "use_item_changes",
            "use_item_dungeon_effects",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("unknown quirk field {key}"));
        }
    }
    Ok(())
}
