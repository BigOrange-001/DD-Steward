use crate::combat::Result;
use serde_json::Value;

pub fn validate(entries: &[Value], skills: &[String]) -> Result<()> {
    if entries.len() > 64 {
        return Err("too many bonus initiatives".into());
    }
    for e in entries {
        let o = e.as_object().ok_or("bonus desire must be object")?;
        if o.keys().any(|k| !["type", "data"].contains(&k.as_str())) {
            return Err("unknown bonus desire field".into());
        }
        let kind = e["type"].as_str().ok_or("missing bonus desire type")?;
        if ![
            "guaranteed",
            "ally_actor_class_count",
            "hp_ratio_threshold",
            "last_skill",
            "actor_dot_count",
            "ally_last_damaged",
            "trinket",
            "death",
        ]
        .contains(&kind)
        {
            return Err(format!("unsupported bonus initiative {kind}"));
        }
        let data = e["data"].as_object().ok_or("missing bonus desire data")?;
        for (k, v) in data {
            match k.as_str() {
                "combat_skill_id_override" => {
                    let id = v.as_str().ok_or("invalid bonus skill ID")?;
                    if !id.is_empty() && !skills.iter().any(|s| s == id) {
                        return Err(format!("unknown bonus skill {id}"));
                    }
                }
                "trinket_id" | "actor_dot_entry_id" => {
                    if v.as_str().is_none_or(str::is_empty) {
                        return Err("invalid bonus identity".into());
                    }
                }
                "last_combat_skill_id" => {
                    if v.as_str().is_none_or(str::is_empty) {
                        return Err("invalid last-skill reference".into());
                    }
                }
                "ally_base_class_id" => {
                    if v.as_str().is_none_or(|s| {
                        s.is_empty() || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    }) {
                        return Err("invalid bonus family".into());
                    }
                }
                "count_heroes"
                | "count_monsters"
                | "is_post_turn"
                | "is_pre_turn"
                | "is_round_finish"
                | "is_round_in_progress"
                | "is_round_start"
                | "is_under_threshold"
                | "ignore_if_stun"
                | "is_valid_if_stunned"
                | "is_cooldown_enabled"
                | "decrement_combat_skill_id_override_cooldown" => {
                    if !v.is_boolean() {
                        return Err(format!("invalid bonus boolean {k}"));
                    }
                }
                "health_ratio_threshold"
                | "performer_health_ratio_min"
                | "performer_health_ratio_max" => {
                    if v.as_f64().is_none_or(|x| !(0. ..=1.).contains(&x)) {
                        return Err("invalid bonus health ratio".into());
                    }
                }
                "actor_dot_count_min"
                | "actor_dot_count_max"
                | "heroes_min"
                | "heroes_max"
                | "monsters_min"
                | "monsters_max"
                | "monsters_size_limit"
                | "ally_count_min"
                | "ally_count_max"
                | "ally_actor_class_count" => {
                    if v.as_u64().is_none_or(|x| x > 4) {
                        return Err(format!("invalid bonus count {k}"));
                    }
                }
                "performing_turn_min" | "performing_turn_max" | "round_min" => {
                    if v.as_u64().is_none_or(|x| x > 10000) {
                        return Err("invalid bonus clock".into());
                    }
                }
                _ => return Err(format!("unsupported bonus initiative field {k}")),
            }
        }
        if kind == "actor_dot_count"
            && (!data.contains_key("actor_dot_entry_id")
                || !data.contains_key("actor_dot_count_min")
                || !data.contains_key("actor_dot_count_max"))
        {
            return Err("actor DoT bonus needs ID/count bounds".into());
        }
        if kind == "trinket" && !data.contains_key("trinket_id") {
            return Err("trinket bonus needs ID".into());
        }
        if kind == "ally_last_damaged" && !data.contains_key("ally_base_class_id") {
            return Err("damage reaction needs family".into());
        }
        if kind == "death"
            && data
                .get("combat_skill_id_override")
                .and_then(|v| v.as_str())
                .is_none_or(str::is_empty)
        {
            return Err("death bonus needs explicit skill".into());
        }
        for stem in [
            "heroes",
            "monsters",
            "ally_count",
            "performing_turn",
            "actor_dot_count",
            "performer_health_ratio",
        ] {
            if let (Some(a), Some(b)) = (
                data.get(&format!("{stem}_min")),
                data.get(&format!("{stem}_max")),
            ) && a.as_u64() > b.as_u64()
            {
                return Err("reversed bonus bounds".into());
            }
        }
        if !data.get("is_pre_turn").is_some_and(|v| v == true)
            && !data.get("is_post_turn").is_some_and(|v| v == true)
        {
            return Err("bonus desire needs a phase".into());
        }
        if !["is_round_start", "is_round_in_progress", "is_round_finish"]
            .iter()
            .any(|k| data.get(*k).is_some_and(|v| v == true))
        {
            return Err("bonus desire needs a round phase".into());
        }
        if kind == "ally_actor_class_count" && !data.contains_key("ally_base_class_id") {
            return Err("bonus ally count needs family".into());
        }
        if kind == "hp_ratio_threshold" && !data.contains_key("health_ratio_threshold") {
            return Err("bonus HP condition needs threshold".into());
        }
        if kind == "last_skill" && !data.contains_key("last_combat_skill_id") {
            return Err("bonus last skill needs ID".into());
        }
        if data.get("is_cooldown_enabled") == Some(&Value::Bool(false))
            && data
                .get("combat_skill_id_override")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            return Err("cooldown bypass requires an explicit override skill".into());
        }
        if data.contains_key("is_under_threshold") && !data.contains_key("health_ratio_threshold") {
            return Err("bonus threshold direction requires threshold".into());
        }
        if data.get("ignore_if_stun") == Some(&Value::Bool(true))
            && data.get("is_valid_if_stunned") == Some(&Value::Bool(true))
        {
            return Err("contradictory bonus stun flags".into());
        }
    }
    Ok(())
}
