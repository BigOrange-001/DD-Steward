use super::*;
use crate::lifecycle::*;
pub(super) fn has(r: &Record) -> bool {
    r.params.iter().any(|p| {
        [
            "summon_count",
            "set_monster_class_id",
            "set_monster_class_ids",
            "kill",
            "initiative_change",
        ]
        .contains(&p.name.as_str())
    })
}
fn known(r: &Record, allowed: &[&str]) -> Result<()> {
    for p in &r.params {
        if !allowed.contains(&p.name.as_str()) {
            return Err(format!("unsupported lifecycle field {}.{}", r.key, p.name));
        }
    }
    Ok(())
}
fn ids(r: &Record, key: &str) -> Result<Vec<String>> {
    let p = r.param(key).ok_or_else(|| format!("missing {key}"))?;
    let list = p.values.iter().map(|v| v.text.clone()).collect::<Vec<_>>();
    if list.is_empty() {
        return Err("empty lifecycle IDs".into());
    }
    Ok(list)
}
fn list(r: &Record, key: &str) -> Result<Vec<f64>> {
    r.param(key)
        .ok_or_else(|| format!("missing {key}"))?
        .values
        .iter()
        .map(|v| scalar(&v.text))
        .collect()
}
fn choice(
    r: &Record,
    id_key: &str,
    weight_key: &str,
    limit_key: Option<&str>,
) -> Result<Vec<Choice>> {
    let names = ids(r, id_key)?;
    let mut weights = if r.param(weight_key).is_some() {
        list(r, weight_key)?
    } else if names.len() == 1 {
        vec![1.]
    } else {
        return Err("missing lifecycle weights".into());
    };
    if weights.len() == 1 && names.iter().all(|n| n == &names[0]) {
        weights.resize(names.len(), weights[0]);
    }
    if weights.len() != names.len() {
        return Err("lifecycle weights length mismatch".into());
    }
    let limits = limit_key.and_then(|k| r.param(k));
    if limits.is_some_and(|p| p.values.len() != names.len()) {
        return Err("lifecycle limits length mismatch".into());
    }
    names
        .into_iter()
        .zip(weights)
        .enumerate()
        .map(|(i, (class, weight))| {
            let limit = limits
                .map(|p| {
                    p.values[i]
                        .text
                        .parse::<u32>()
                        .map_err(|_| "invalid summon limit".to_string())
                })
                .transpose()?;
            let c = Choice {
                class,
                weight,
                limit,
            };
            c.validate()?;
            Ok(c)
        })
        .collect()
}
fn change(r: &Record) -> Result<ChangeSpec> {
    if r.param("set_monster_class_id").is_some() && r.param("set_monster_class_ids").is_some() {
        return Err("conflicting class selectors".into());
    }
    let choices = choice(
        r,
        if r.param("set_monster_class_id").is_some() {
            "set_monster_class_id"
        } else {
            "set_monster_class_ids"
        },
        "set_monster_class_chances",
        None,
    )?;
    Ok(ChangeSpec {
        choices,
        reset_hp: flag(r, "set_monster_class_reset_hp", false)?,
        reset_buffs: flag(r, "set_monster_class_reset_buffs", false)?,
        clear_initiative: flag(r, "set_monster_class_clear_initative", false)?,
        clear_cooldowns: flag(r, "set_monster_class_clear_monster_brain_cooldowns", true)?,
    })
}
pub(super) fn effect(g: &GameData, r: &Record, name: &str) -> Result<Effect> {
    let common = [
        "name",
        "target",
        "chance",
        "on_hit",
        "on_miss",
        "apply_once",
        "queue",
        "can_apply_on_death",
        "curio_result_type",
        "apply_with_result",
    ];
    let kinds = [
        r.param("summon_count").is_some(),
        r.param("set_monster_class_id").is_some() || r.param("set_monster_class_ids").is_some(),
        r.param("kill").is_some(),
        r.param("initiative_change").is_some(),
    ];
    if kinds.iter().filter(|x| **x).count() != 1 {
        return Err("lifecycle effect requires exactly one operation".into());
    }
    let extra: &[&str] = if kinds[0] {
        &[
            "summon_count",
            "summon_monsters",
            "summon_chances",
            "summon_limits",
            "summon_ranks",
            "summon_can_spawn_loot",
            "summon_does_roll_initiatives",
            "summon_rank_is_previous_monster_class",
            "set_monster_class_reset_scale",
        ]
    } else if kinds[1] {
        &[
            "set_monster_class_id",
            "set_monster_class_ids",
            "set_monster_class_chances",
            "set_monster_class_reset_hp",
            "set_monster_class_reset_buffs",
            "set_monster_class_clear_initative",
            "set_monster_class_clear_monster_brain_cooldowns",
            "set_monster_class_reset_scale",
        ]
    } else if kinds[3] {
        &[
            "initiative_change",
            "bonus_action_next_turn",
            "requires_kill_target",
        ]
    } else {
        &["kill"]
    };
    let allowed = common
        .into_iter()
        .chain(extra.iter().copied())
        .collect::<Vec<_>>();
    known(r, &allowed)?;
    for key in ["apply_with_result", "set_monster_class_reset_scale"] {
        if r.param(key).is_some() {
            flag(r, key, false)?;
        }
    }
    flag(r, "apply_with_result", false)?;
    let op = if kinds[0] {
        let count = integer(r, "summon_count")?;
        if !(1..=4).contains(&count) {
            return Err("invalid summon count".into());
        }
        let ranks = if r.param("summon_ranks").is_some() {
            ids(r, "summon_ranks")?
                .into_iter()
                .map(|s| {
                    if s == "0"
                        && ["Baron Refill Cocoons 1-3", "Baron Refill Cocoons 2-3"].contains(&name)
                        && g.overlay
                            .config
                            .syntax_choices
                            .get("baron_rank_zero")
                            .map(String::as_str)
                            == Some("auto")
                    {
                        Ok(vec![])
                    } else {
                        Target::parse(&s).map(|t| t.ranks)
                    }
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect()
        } else {
            vec![]
        };
        LifeOp::Summon(SummonSpec {
            choices: choice(
                r,
                "summon_monsters",
                "summon_chances",
                Some("summon_limits"),
            )?,
            count: count as u8,
            ranks,
            roll_initiative: if r.param("summon_does_roll_initiatives").is_some() {
                let values = list(r, "summon_does_roll_initiatives")?;
                if values.is_empty()
                    || (values.len() != 1
                        && values.len() != r.param("summon_monsters").unwrap().values.len())
                    || values.iter().any(|n| *n != values[0])
                {
                    return Err("unsupported mixed summon initiative flags".into());
                }
                let n = values[0] as i32;
                if ![0, 1].contains(&n) {
                    return Err("invalid summon initiative flag".into());
                }
                n == 1
            } else {
                false
            },
            replace_corpse: flag(r, "summon_rank_is_previous_monster_class", false)?,
            loot: flag(r, "summon_can_spawn_loot", false)?,
        })
    } else if kinds[1] {
        LifeOp::Change(change(r)?)
    } else if kinds[2] {
        if integer(r, "kill")? != 1 {
            return Err("invalid kill flag".into());
        }
        LifeOp::Kill
    } else {
        LifeOp::Initiative {
            amount: integer(r, "initiative_change")?,
            next_round: flag(r, "bonus_action_next_turn", false)?,
        }
    };
    op.validate()?;
    for id in op.references() {
        g.monster_document(id)?;
    }
    let mut stub = r.clone();
    stub.params.retain(|p| {
        common.contains(&p.name.as_str())
            && !["queue", "apply_with_result"].contains(&p.name.as_str())
    });
    stub.params.push(crate::darkest::Param {
        name: "heal".into(),
        values: vec![crate::darkest::Value {
            text: "0".into(),
            quoted: false,
        }],
    });
    let mut e = g.effect_legacy(&stub, name)?;
    e.kind = EffectKind::Lifecycle(LifecycleEffect {
        op,
        queued: flag(r, "queue", false)?,
        requires_kill: flag(r, "requires_kill_target", false)?,
    });
    Ok(e)
}
pub(super) fn spec(g: &GameData, d: &Document, turns: u8) -> Result<LifeSpec> {
    let mut spec = LifeSpec {
        turns,
        ..LifeSpec::default()
    };
    if turns == 0 {
        spec.role = EntityRole::Construct;
    }
    if d.records_keyed("enemy_type")
        .any(|r| text(r, "id") == Ok("corpse"))
    {
        spec.role = EntityRole::Corpse;
    }
    for r in d.records_keyed("spawn") {
        known(r, &["effects", "wave_effects"])?;
        if r.param("effects").is_some() {
            for id in ids(r, "effects")? {
                spec.spawn.push(g.effect(&id)?);
            }
        }
        if r.param("wave_effects").is_some() {
            for id in ids(r, "wave_effects")? {
                let mut raw = named(&g.effects, "effect", &id)?.clone();
                let family = raw
                    .param("spawn_target_actor_base_class_id")
                    .map(|_| text(&raw, "spawn_target_actor_base_class_id").map(str::to_string))
                    .transpose()?;
                raw.params
                    .retain(|p| p.name != "spawn_target_actor_base_class_id");
                let effect = if extended::applies(&raw) {
                    extended::parse(g, &raw, &id)?
                } else {
                    g.effect_legacy(&raw, &id)?
                };
                spec.wave_spawn.push(SpawnEffect {
                    target_family: family,
                    effect,
                });
            }
        }
    }
    if let Some(r) = d.records_keyed("battle_modifier").next() {
        for (key, hero) in [
            ("living_other_enemy_buffs", false),
            ("living_hero_buff_instance_ids", true),
        ] {
            if let Some(p) = r.param(key) {
                for id in &p.values {
                    let buff = read_buff(&g.buffs, &id.text)?;
                    if hero {
                        spec.aura_heroes.push(buff);
                    } else {
                        spec.aura_allies.push(buff);
                    }
                }
            }
        }
        spec.disable_stall = flag(r, "disable_stall_penalty", false)?;
        spec.accelerate_stall = flag(r, "accelerate_stall_penalty", false)?;
        spec.can_surprise = flag(r, "can_surprise", true)?;
        spec.can_be_surprised = flag(r, "can_be_surprised", true)?;
        spec.always_surprise = flag(r, "always_surprise", false)?;
        spec.always_be_surprised = flag(r, "always_be_surprised", false)?;
        spec.targetable = flag(r, "can_be_hit", true)?;
        spec.random_targetable = flag(r, "can_be_random_target", true)?;
        spec.friendly_targetable = flag(r, "is_valid_friendly_target", true)?;
        spec.can_be_guarded = flag(r, "can_be_guarded", true)?;
        spec.can_be_summon_rank = flag(r, "can_be_summon_rank", false)?;
    }
    if let Some(r) = d.records_keyed("life_time").next() {
        known(r, &["alive_round_limit", "does_check_for_loot"])?;
        let n = integer(r, "alive_round_limit")?;
        if !(1..=10000).contains(&n) {
            return Err("invalid entity lifetime".into());
        }
        spec.lifetime = Some(n as u32);
        spec.loot = flag(r, "does_check_for_loot", false)?;
    }
    if let Some(r) = d.records_keyed("death_class").next()
        && text(r, "type").ok() != Some("corpse")
    {
        known(
            r,
            &[
                "monster_class_id",
                "random_monster_class_ids",
                "random_monster_class_chances",
                "is_valid_on_crit",
                "is_valid_on_bleed_dot",
                "is_valid_on_blight_dot",
                "is_valid_on_burn_dot",
                "reset_scale_anim",
                "on_change_sfx",
                "can_die_from_damage",
                "carry_over_hp_min_percent",
                "change_class_effects",
                "use_previous_monster_hp",
                "clear_monster_brain_cooldowns",
                "type",
            ],
        )?;
        if r.param("type").is_some() && text(r, "type")? != "baron" {
            return Err("unknown non-corpse death class type".into());
        }
        for key in [
            "is_valid_on_bleed_dot",
            "is_valid_on_blight_dot",
            "is_valid_on_burn_dot",
        ] {
            if r.param(key).is_some() && !flag(r, key, false)? {
                return Err("selective death-source replacement belongs to task 011".into());
            }
        }
        for key in ["reset_scale_anim", "on_change_sfx"] {
            if r.param(key).is_some() {
                flag(r, key, false)?;
            }
        }
        spec.death_change = Some(ChangeSpec {
            choices: choice(
                r,
                if r.param("monster_class_id").is_some() {
                    "monster_class_id"
                } else {
                    "random_monster_class_ids"
                },
                "random_monster_class_chances",
                None,
            )?,
            reset_hp: !flag(r, "use_previous_monster_hp", false)?,
            reset_buffs: true,
            clear_initiative: true,
            clear_cooldowns: flag(r, "clear_monster_brain_cooldowns", true)?,
        });
        spec.death_on_crit = flag(r, "is_valid_on_crit", true)?;
    }
    if spec.role == EntityRole::Corpse && spec.lifetime.is_none() {
        return Err("corpse entity requires original lifetime".into());
    }
    if let Some(r) = d.records_keyed("life_link").next() {
        known(r, &["base_class", "does_spawn_loot"])?;
        if r.param("does_spawn_loot").is_some() {
            spec.link_loot = flag(r, "does_spawn_loot", true)?;
        }
    }
    Ok(spec)
}

