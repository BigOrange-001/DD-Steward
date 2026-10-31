use super::*;
use crate::trinket010::TrinketOp;
pub(super) fn applies(r: &Record) -> bool {
    [
        "is_trigger_effect",
        "is_in_inventory",
        "can_apply_on_corpse",
        "destroy_trinket",
        "gain_trinket",
        "trigger_limit_minimum_increase",
        "trigger_limit_maximum_increase",
        "health_damage",
        "crit_doesnt_apply_to_roll",
    ]
    .iter()
    .any(|key| r.param(key).is_some())
        || text(r, "target").is_ok_and(|t| {
            [
                "this_trinket",
                "target_enemy_random",
                "target_enemy_rank",
                "target_group",
            ]
            .contains(&t)
        })
}
fn set(r: &mut Record, k: &str, v: &str) {
    r.params.retain(|p| p.name != k);
    r.params.push(crate::darkest::Param {
        name: k.into(),
        values: vec![crate::darkest::Value {
            text: v.into(),
            quoted: false,
        }],
    });
}
pub(super) fn parse(g: &GameData, r: &Record, name: &str) -> Result<Effect> {
    let inventory = flag(r, "is_in_inventory", false)?;
    let corpse = flag(r, "can_apply_on_corpse", false)?;
    flag(r, "is_trigger_effect", false)?;
    let trinket_op = r.param("destroy_trinket").is_some()
        || r.param("gain_trinket").is_some()
        || r.param("trigger_limit_minimum_increase").is_some()
        || r.param("trigger_limit_maximum_increase").is_some();
    let target = if r.param("target").is_none() && trinket_op {
        "this_trinket"
    } else {
        text(r, "target")?
    };
    let out = match target {
        "this_trinket" => EffectTarget::ThisTrinket,
        "target_enemy_random" => EffectTarget::EnemyRandom,
        "target_enemy_rank" => {
            let rank = integer(r, "clear_rank_target")?;
            if !(1..=4).contains(&rank) {
                return Err("invalid rank trigger target".into());
            }
            EffectTarget::EnemyRank(rank as u8)
        }
        "target_group" => EffectTarget::TargetParty,
        "performer" => EffectTarget::Performer,
        "target" => EffectTarget::Target,
        "performer_group_other" => EffectTarget::PartyOther,
        _ => return Err(format!("unsupported trinket effect target {target}")),
    };
    let mut raw = r.clone();
    raw.params.retain(|p| {
        ![
            "is_in_inventory",
            "is_trigger_effect",
            "can_apply_on_corpse",
            "clear_rank_target",
        ]
        .contains(&p.name.as_str())
    });
    set(
        &mut raw,
        "target",
        if matches!(out, EffectTarget::Performer | EffectTarget::ThisTrinket) {
            "performer"
        } else if matches!(out, EffectTarget::PartyOther) {
            "performer_group_other"
        } else {
            "target"
        },
    );
    if corpse {
        set(&mut raw, "can_apply_on_death", "true");
    }
    let mut effect = if trinket_op || r.param("health_damage").is_some() {
        let known = [
            "name",
            "target",
            "chance",
            "curio_result_type",
            "destroy_trinket",
            "gain_trinket",
            "trigger_limit_minimum_increase",
            "trigger_limit_maximum_increase",
            "health_damage",
            "damage_type",
            "damage_source_type",
            "damage_source_data",
            "on_hit",
            "on_miss",
            "queue",
            "apply_once",
            "can_apply_on_death",
        ];
        if raw.params.iter().any(|p| !known.contains(&p.name.as_str())) {
            return Err("unknown item operation field".into());
        }
        let kinds = [
            r.param("destroy_trinket").is_some(),
            r.param("gain_trinket").is_some(),
            r.param("trigger_limit_minimum_increase").is_some()
                || r.param("trigger_limit_maximum_increase").is_some(),
            r.param("health_damage").is_some(),
        ];
        if kinds.iter().filter(|b| **b).count() != 1 {
            return Err("item effect must have one operation".into());
        }
        let kind = if kinds[0] {
            if !matches!(text(r, "destroy_trinket")?, "true" | "1") {
                return Err("invalid destroy flag".into());
            }
            EffectKind::TrinketOp(TrinketOp::Destroy)
        } else if kinds[1] {
            EffectKind::TrinketOp(TrinketOp::Gain(text(r, "gain_trinket")?.to_owned()))
        } else if kinds[2] {
            let min = integer(r, "trigger_limit_minimum_increase")?;
            let max = integer(r, "trigger_limit_maximum_increase")?;
            if min < 0 || min > max || max > 1000000 {
                return Err("invalid trigger limit increase".into());
            }
            EffectKind::TrinketOp(TrinketOp::Increase {
                min: min as u32,
                max: max as u32,
            })
        } else {
            let n = integer(r, "health_damage")?;
            if !(0..=1000000).contains(&n)
                || text(r, "damage_type")? != "additionaleffect"
                || text(r, "damage_source_type")? != "trinket"
            {
                return Err("unsupported item health damage".into());
            }
            text(r, "damage_source_data")?;
            EffectKind::HealthDamage(n)
        };
        flag(r, "queue", false)?;
        Effect {
            execution: Default::default(),
            name: name.into(),
            kind,
            chance: number(r, "chance")?,
            duration: 1,
            target: out.clone(),
            on_hit: flag(r, "on_hit", true)?,
            on_miss: flag(r, "on_miss", false)?,
            apply_once: flag(r, "apply_once", false)?,
            resist_debuff: false,
            conditions: vec![],
            can_apply_on_death: flag(r, "can_apply_on_death", false)?,
        }
    } else {
        if raw.param("crit_doesnt_apply_to_roll").is_some() {
            flag(&raw, "crit_doesnt_apply_to_roll", true)?;
            raw.params.retain(|p| p.name != "crit_doesnt_apply_to_roll");
        }
        if lifecycle_parser::has(&raw) {
            lifecycle_parser::effect(g, &raw, name)?
        } else if extended::applies(&raw) {
            extended::parse(g, &raw, name)?
        } else {
            g.effect_legacy(&raw, name)?
        }
    };
    effect.target = out;
    if inventory {
        effect.kind = EffectKind::InventoryEffect(Box::new(effect.kind));
    }
    Ok(effect)
}
