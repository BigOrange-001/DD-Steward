use crate::{combat::Result, mechanics::Desire};
use serde_json::Value;

pub const COUNTS: &[&str] = &[
    "heroes_min",
    "control_count_min",
    "control_count_max",
    "non_actor_dot_heroes_min",
    "non_actor_dot_heroes_max",
    "marked_heroes_min",
    "marked_heroes_max",
    "non_marked_heroes_min",
    "non_stunned_heroes_min",
    "non_deaths_door_heroes_min",
    "virtued_heroes_max",
    "monsters_min",
    "monsters_max",
    "monsters_size_min",
    "monsters_size_max",
    "non_guard_monsters_min",
    "guarded_monsters_min",
    "guarded_monsters_max",
    "target_stress_min",
    "target_stress_heroes_min",
    "performing_turn_min",
    "performing_turn_max",
    "performer_rounds_in_rank_min",
];
pub fn validate_predicates(d: &Desire, target: bool) -> Result<()> {
    let p = &d.data.predicates;
    if !d.data.resistance.is_empty() {
        return Err("unsupported legacy AI resistance field".into());
    }
    if !target
        && (d.data.is_exclusive_desire
            || d.data.is_enemy_target_desire
            || d.data.is_friendly_target_desire
            || !d.data.specific_combat_skill_id.is_empty()
            || d.data.rank.is_some())
    {
        return Err("target fields on skill desire".into());
    }
    if target
        && (!d.data.combat_skill_id.is_empty()
            || d.data.performing_turn.is_some()
            || d.data.hp_ratio_threshold.is_some())
    {
        return Err("skill fields on target desire".into());
    }
    if d.data.performing_turn.is_some() && d.kind != "performing_turn_skill" {
        return Err("inapplicable performing_turn".into());
    }
    if d.data.hp_ratio_threshold.is_some()
        && !["heal_skill", "specific_skill"].contains(&d.kind.as_str())
    {
        return Err("inapplicable heal threshold".into());
    }
    if (d.data.can_target_not_overstressed
        || d.data.can_target_afflicted
        || d.data.can_target_virtued)
        && !target
    {
        return Err("resolve filters require stress_target".into());
    }
    if d.kind == "performer_dot_skill"
        && !p.get("has_bleed").is_some_and(|v| v == &Value::Bool(true))
        && !p.get("has_poison").is_some_and(|v| v == &Value::Bool(true))
    {
        return Err("DoT skill needs at least one enabled status".into());
    }
    if d.kind == "guard_target" && !p.contains_key("is_guarded") && !p.contains_key("is_guarding") {
        return Err("guard_target requires a guard predicate".into());
    }
    for key in p.keys() {
        let only = match key.as_str() {
            "has_all_dots" | "has_bleed" | "has_poison" => Some("performer_dot_skill"),
            "hp_ratio_min" | "hp_ratio_max" => Some("ally_hp_ratio_skill"),
            "effect_key_status" => Some("effect_key_status_skill"),
            "resistance_type_id" => Some("resistance_target"),
            _ => None,
        };
        if only.is_some_and(|kind| kind != d.kind) {
            return Err(format!("inapplicable AI predicate {key} on {}", d.kind));
        }
        if key == "ally_base_class_id"
            && ![
                "ally_alive_skill",
                "ally_dead_skill",
                "ally_hp_ratio_skill",
                "ally_class_target",
                "health_target",
            ]
            .contains(&d.kind.as_str())
        {
            return Err("inapplicable ally family".into());
        }
    }
    if p.contains_key("target_stress_min") && !p.contains_key("target_stress_heroes_min") {
        return Err("target_stress_min requires target_stress_heroes_min".into());
    }
    if d.kind == "performer_dot_skill"
        && !p.contains_key("has_bleed")
        && !p.contains_key("has_poison")
    {
        return Err("performer_dot_skill requires actual DoT predicates".into());
    }
    for (key, value) in p {
        if key == "first_initiative_only" {
            if target || !value.is_boolean() {
                return Err("invalid first initiative predicate".into());
            }
            continue;
        }
        if key == "is_valid_when_corpse_enabled" {
            if target || !value.is_boolean() {
                return Err("invalid corpse rule predicate".into());
            }
            continue;
        }
        if key == "per_round_chance" {
            if target
                || value
                    .as_f64()
                    .is_none_or(|n| !n.is_finite() || !(0. ..=1000000.).contains(&n))
            {
                return Err("invalid per-round chance".into());
            }
            continue;
        }
        if COUNTS.contains(&key.as_str()) {
            let max = if key == "target_stress_min" {
                200
            } else if key.starts_with("perform") {
                10000
            } else {
                4
            };
            if value
                .as_f64()
                .is_none_or(|n| !n.is_finite() || n.fract() != 0. || n < 0. || n > max as f64)
            {
                return Err(format!("invalid AI count {key}"));
            }
            if target {
                return Err(format!("skill predicate {key} on target desire"));
            }
        } else if [
            "hp_ratio_min",
            "hp_ratio_max",
            "performer_health_ratio_min",
            "performer_health_ratio_max",
        ]
        .contains(&key.as_str())
        {
            if value
                .as_f64()
                .is_none_or(|n| !n.is_finite() || !(0. ..=1.).contains(&n))
            {
                return Err(format!("invalid AI ratio {key}"));
            }
            if target {
                return Err(format!("skill ratio {key} on target desire"));
            }
        } else if [
            "has_all_dots",
            "has_bleed",
            "has_poison",
            "is_guarded",
            "is_guarding",
            "can_target_deaths_door",
            "can_target_last_hero",
            "can_target_stunned",
        ]
        .contains(&key.as_str())
        {
            if !value.is_boolean() {
                return Err(format!("invalid AI boolean {key}"));
            }
            if (key.starts_with("can_target")
                && !target
                && d.kind != "fill_ally_captor_empty_skill")
                || (key.starts_with("has_") && target)
            {
                return Err(format!("misplaced AI predicate {key}"));
            }
        } else if key == "valid_ranks" {
            if !target {
                return Err("valid_ranks requires target desire".into());
            }
            rank_values(value)?;
        } else if [
            "ally_base_class_id",
            "effect_key_status",
            "resistance_type_id",
            "actor_dot_entry_id",
            "trinket_id",
        ]
        .contains(&key.as_str())
        {
            let s = value
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("invalid AI string {key}"))?;
            if key == "effect_key_status" {
                crate::combat::Condition::Status(s.into()).validate()?;
            }
            if key == "resistance_type_id"
                && ![
                    "bleed",
                    "poison",
                    "stun",
                    "debuff",
                    "move",
                    "death_blow",
                    "disease",
                ]
                .contains(&s)
            {
                return Err(format!("unsupported AI resistance {s}"));
            }
            if key == "ally_base_class_id"
                && !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err("invalid ally family".into());
            }
        } else {
            return Err(format!("unsupported AI predicate {key}"));
        }
    }
    for (key, v) in p.iter().filter(|(k, _)| k.ends_with("_min")) {
        let max = key.trim_end_matches("_min").to_string() + "_max";
        if let Some(hi) = p.get(&max)
            && v.as_f64() > hi.as_f64()
        {
            return Err(format!("reversed AI bounds {key}"));
        }
    }
    let required = match d.kind.as_str() {
        "effect_key_status_skill" => Some("effect_key_status"),
        "ally_alive_skill" | "ally_dead_skill" | "ally_hp_ratio_skill" | "ally_class_target" => {
            Some("ally_base_class_id")
        }
        "resistance_target" => Some("resistance_type_id"),
        _ => None,
    };
    if let Some(key) = required
        && !p.contains_key(key)
    {
        return Err(format!("{} requires {key}", d.kind));
    }
    Ok(())
}
pub fn number(d: &Desire, key: &str) -> Option<f64> {
    d.data.predicates.get(key).and_then(Value::as_f64)
}
pub fn boolean(d: &Desire, key: &str) -> Option<bool> {
    d.data.predicates.get(key).and_then(Value::as_bool)
}
pub fn string<'a>(d: &'a Desire, key: &str) -> Option<&'a str> {
    d.data.predicates.get(key).and_then(Value::as_str)
}

/// Both original AI array ranks and compact string ranks denote occupied cells.
pub fn rank_values(value: &Value) -> Result<Vec<u8>> {
    let ranks = if let Some(values) = value.as_array() {
        values
            .iter()
            .map(|v| {
                v.as_u64()
                    .and_then(|v| u8::try_from(v).ok())
                    .ok_or_else(|| "invalid AI rank".to_string())
            })
            .collect::<Result<Vec<_>>>()?
    } else if let Some(text) = value.as_str() {
        if !text.bytes().all(|b| (b'1'..=b'4').contains(&b)) {
            return Err("invalid AI rank string".into());
        }
        text.bytes().map(|b| b - b'0').collect()
    } else {
        return Err("AI valid_ranks requires array or rank string".into());
    };
    if ranks.is_empty() || ranks.iter().any(|r| !(1..=4).contains(r)) {
        return Err("invalid AI ranks".into());
    }
    Ok(ranks)
}

