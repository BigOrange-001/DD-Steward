use super::*;
const OLD_OPS: &[&str] = &[
    "dotStress",
    "clearDotStress",
    "dotBleed",
    "dotPoison",
    "stun",
    "stress",
    "healstress",
    "heal",
    "tag",
    "push",
    "pull",
    "shuffletarget",
    "stealth",
    "unstealth",
];
const BUFF_FIELDS: &[&str] = &[
    "buff_ids",
    "attack_rating_add",
    "defense_rating_add",
    "protection_rating_add",
    "speed_rating_add",
    "crit_chance_add",
    "damage_low_multiply",
    "damage_high_multiply",
];
const NEW_OPS: &[&str] = &[
    "refreshes_skill_uses",
    "dotBurn",
    "dotHpHeal",
    "heal_percent",
    "cure",
    "cure_bleed",
    "cure_poison",
    "cure_burn",
    "unstun",
    "clear_tag",
    "clear_debuff",
    "clear_buff",
    "kill_enemy_types",
    "shuffleparty",
    "guard",
    "clearguard",
    "clearguarding",
    "clearguarded",
    "riposte",
    "clear_riposte",
    "health_damage_blocks",
    "buff_type",
];
const EXTRA: &[&str] = &[
    "buff_amount",
    "swap_source_and_target",
    "riposte_on_hit_chance_add",
    "riposte_validate",
    "riposte_on_miss_chance_add",
    "has_description",
    "dot_irresistible",
    "buff_is_clear_debuff_valid",
    "untag",
    "speed_rating",
    "critical_rating",
];
const META: &[&str] = &[
    "combat_stat_buff",
    "name",
    "item",
    "target",
    "curio_result_type",
    "chance",
    "duration",
    "on_hit",
    "on_miss",
    "queue",
    "can_apply_on_death",
    "apply_once",
    "debuff",
    "keyStatus",
    "monsterType",
    "buff_source_type",
    "buff_duration_type",
    "source_heal_type",
    "can_crit_heal",
    "crit_doesnt_apply_to_roll",
];
pub(super) fn applies(r: &Record) -> bool {
    r.params
        .iter()
        .any(|p| NEW_OPS.contains(&p.name.as_str()) || EXTRA.contains(&p.name.as_str()))
        || OLD_OPS.iter().filter(|op| r.param(op).is_some()).count()
            + usize::from(BUFF_FIELDS.iter().any(|op| r.param(op).is_some()))
            > 1
}
fn set(r: &mut Record, key: &str, value: &str) {
    r.params.retain(|p| p.name != key);
    r.params.push(crate::darkest::Param {
        name: key.into(),
        values: vec![crate::darkest::Value {
            text: value.into(),
            quoted: false,
        }],
    });
}
fn positive(r: &Record, key: &str) -> Result<i32> {
    let n = integer(r, key)?;
    if !(0..=1000000).contains(&n) {
        return Err(format!("invalid effect {key}"));
    }
    Ok(n)
}
pub(super) fn parse(g: &GameData, r: &Record, name: &str) -> Result<Effect> {
    let mut normalized = r.clone();
    if r.param("riposte_validate").is_some() {
        if flag(r, "riposte_validate", false)? {
            return Err("validated-only riposte is unsupported".into());
        }
        normalized.params.retain(|p| p.name != "riposte_validate");
    }
    for (original, canonical) in [
        ("untag", "clear_tag"),
        ("speed_rating", "speed_rating_add"),
        ("critical_rating", "crit_chance_add"),
    ] {
        if normalized.param(original).is_some() && normalized.param(canonical).is_some() {
            return Err(format!("conflicting effect aliases {original}/{canonical}"));
        }
        for p in &mut normalized.params {
            if p.name == original {
                p.name = canonical.into();
            }
        }
    }
    let r = &normalized;
    for p in &r.params {
        if !META.contains(&p.name.as_str())
            && !BUFF_FIELDS.contains(&p.name.as_str())
            && !OLD_OPS.contains(&p.name.as_str())
            && !NEW_OPS.contains(&p.name.as_str())
            && !EXTRA.contains(&p.name.as_str())
        {
            return Err(format!("{name}: unsupported effect parameter .{}", p.name));
        }
    }
    for key in [
        "has_description",
        "dot_irresistible",
        "buff_is_clear_debuff_valid",
        "swap_source_and_target",
        "queue",
    ] {
        if r.param(key).is_some() {
            flag(r, key, false)?;
        }
    }
    if r.param("swap_source_and_target").is_some() && r.param("guard").is_none() {
        return Err("swap_source_and_target requires guard".into());
    }
    if r.param("riposte").is_none()
        && ["riposte_on_hit_chance_add", "riposte_on_miss_chance_add"]
            .iter()
            .any(|k| r.param(k).is_some())
    {
        return Err("riposte chance requires riposte".into());
    }
    let mut meta = r.clone();
    meta.params.retain(|p| META.contains(&p.name.as_str()));
    if meta.param("chance").is_none() {
        set(&mut meta, "chance", "100%");
    }
    let enemy_group = text(r, "target").ok() == Some("target_group");
    if enemy_group {
        set(&mut meta, "target", "target");
    }
    let mut stub = meta.clone();
    stub.params
        .retain(|p| !["keyStatus", "monsterType"].contains(&p.name.as_str()));
    set(&mut stub, "heal", "0");
    let mut result = g.effect_legacy(&stub, name)?;
    for (key, status) in [("keyStatus", true), ("monsterType", false)] {
        if r.param(key).is_some() {
            let c = if status {
                Condition::Status(text(r, key)?.into())
            } else {
                Condition::MonsterType(text(r, key)?.into())
            };
            c.validate()?;
            result.conditions.push(c);
        }
    }
    if r.param("dot_irresistible").is_some()
        && (r.param("dotBurn").is_none() || !flag(r, "dot_irresistible", false)?)
    {
        return Err("only explicitly irresistible Burn is supported".into());
    }
    if enemy_group {
        result.target = EffectTarget::Enemies;
    }
    let duration = r
        .param("duration")
        .map(|_| integer(r, "duration"))
        .transpose()?
        .unwrap_or(1);
    if duration < 0 && text(r, "buff_duration_type").ok() != Some("combat_end") {
        return Err(format!("{name}: unsupported negative effect duration"));
    }
    if (duration <= 0 || duration > 10000)
        && ["guard", "riposte", "dotHpHeal"]
            .iter()
            .any(|k| r.param(k).is_some())
    {
        return Err("invalid persistent tactical duration".into());
    }
    let mut ops = vec![];
    for op in OLD_OPS {
        if r.param(op).is_some() {
            let mut single = meta.clone();
            single.params.push(r.param(op).unwrap().clone());
            let mut e = g.effect_legacy(&single, name)?;
            if enemy_group {
                e.target = EffectTarget::Enemies;
            }
            ops.push(e);
        }
    }
    let mut buffs = vec![];
    if BUFF_FIELDS.iter().any(|op| r.param(op).is_some()) {
        let mut single = meta.clone();
        for p in &r.params {
            if BUFF_FIELDS.contains(&p.name.as_str()) {
                single.params.push(p.clone());
            }
        }
        if duration < 0 {
            single.params.retain(|p| p.name != "duration");
        }
        let e = g.effect_legacy(&single, name)?;
        if let EffectKind::Buffs(list) = e.kind {
            buffs = list;
        }
    }
    if let Some(p) = r.param("buff_type") {
        if p.values.len() != 1 {
            return Err("buff_type requires one type".into());
        }
        buffs.push(Buff {
            activation: Default::default(),
            id: format!("{name}:inline"),
            stat_type: text(r, "buff_type")?.into(),
            stat_sub_type: String::new(),
            amount: number(r, "buff_amount")?,
            duration: if r.param("duration").is_some() && duration > 0 {
                Some(duration as u32)
            } else {
                None
            },
            duration_type: r
                .param("buff_duration_type")
                .map(|_| text(r, "buff_duration_type").map(str::to_string))
                .transpose()?,
            rule_type: "always".into(),
            is_false_rule: false,
            clearable: true,
            force_positive: false,
            rule_data: Default::default(),
        });
    } else if r.param("buff_amount").is_some() {
        return Err("buff_amount without buff_type".into());
    }
    for b in &buffs {
        b.validate()?;
    }
    let make = |kind| {
        let mut e = result.clone();
        e.kind = kind;
        e.duration = duration.max(1) as u32;
        e
    };
    if r.param("riposte").is_some() {
        if integer(r, "riposte")? != 1 {
            return Err("invalid riposte flag".into());
        }
        let hit = number(r, "riposte_on_hit_chance_add")?;
        let miss = number(r, "riposte_on_miss_chance_add")?;
        if !(0. ..=1.).contains(&hit) || !(0. ..=1.).contains(&miss) {
            return Err("invalid riposte trigger chance".into());
        }
        ops.push(make(EffectKind::Riposte { hit, miss, buffs }));
    } else if !buffs.is_empty() {
        let mut e = make(if flag(r, "buff_is_clear_debuff_valid", true)? {
            EffectKind::Buffs(buffs)
        } else {
            EffectKind::ProtectedBuffs(buffs)
        });
        e.resist_debuff =
            flag(r, "debuff", false)? || text(r, "curio_result_type").ok() == Some("negative");
        ops.push(e);
    }
    let any_cure = ["cure", "cure_bleed", "cure_poison", "cure_burn"]
        .iter()
        .any(|k| r.param(k).is_some());
    if any_cure {
        let mut bits = [false; 3];
        for (key, mask) in [
            ("cure", [true, true, false]),
            ("cure_bleed", [true, false, false]),
            ("cure_poison", [false, true, false]),
            ("cure_burn", [false, false, true]),
        ] {
            if r.param(key).is_some() {
                if integer(r, key)? != 1 {
                    return Err(format!("invalid {key}"));
                }
                for i in 0..3 {
                    bits[i] |= mask[i];
                }
            }
        }
        ops.push(make(EffectKind::Cure {
            bleed: bits[0],
            poison: bits[1],
            burn: bits[2],
        }));
    }
    for (key, kind) in [
        ("dotBurn", 0),
        ("dotHpHeal", 1),
        ("heal_percent", 2),
        ("unstun", 3),
        ("clear_tag", 4),
        ("clear_debuff", 5),
        ("clear_buff", 6),
        ("kill_enemy_types", 7),
        ("shuffleparty", 8),
        ("guard", 9),
        ("clear_riposte", 10),
        ("health_damage_blocks", 11),
    ] {
        if r.param(key).is_none() {
            continue;
        }
        let kind = match kind {
            0 => EffectKind::Burn(positive(r, key)?),
            1 => EffectKind::Regen(positive(r, key)?),
            2 => {
                let n = number(r, key)?;
                if !(0. ..=1.).contains(&n) {
                    return Err("invalid heal_percent".into());
                }
                EffectKind::HealPercent(n)
            }
            7 => {
                if text(r, key)? != "corpse" {
                    return Err("only corpse removal supported".into());
                }
                EffectKind::KillCorpses
            }
            8 => {
                if !r.param(key).unwrap().values.is_empty() && integer(r, key)? != 1 {
                    return Err("invalid shuffleparty".into());
                }
                EffectKind::ShuffleParty
            }
            9 => {
                if integer(r, key)? != 1 {
                    return Err("invalid guard".into());
                }
                EffectKind::Guard {
                    reverse: flag(r, "swap_source_and_target", false)?,
                }
            }
            11 => {
                let count = positive(r, key)? as u32;
                if count == 0 || count > 10000 {
                    return Err("invalid block count".into());
                }
                EffectKind::Blocks {
                    count,
                    turns: if r.param("duration").is_some() && duration > 0 {
                        Some(duration as u32)
                    } else {
                        None
                    },
                }
            }
            n => {
                if integer(r, key)? != 1 {
                    return Err(format!("invalid {key}"));
                }
                match n {
                    3 => EffectKind::Unstun,
                    4 => EffectKind::ClearMark,
                    5 => EffectKind::ClearBuffs { positive: false },
                    6 => EffectKind::ClearBuffs { positive: true },
                    10 => EffectKind::ClearRiposte,
                    _ => unreachable!(),
                }
            }
        };
        ops.push(make(kind));
    }
    if ["clearguard", "clearguarding", "clearguarded"]
        .iter()
        .any(|k| r.param(k).is_some())
    {
        for key in ["clearguard", "clearguarding", "clearguarded"] {
            if r.param(key).is_some() && integer(r, key)? != 1 {
                return Err(format!("invalid {key}"));
            }
        }
        ops.push(make(EffectKind::ClearGuard {
            guarding: r.param("clearguard").is_some() || r.param("clearguarding").is_some(),
            guarded: r.param("clearguard").is_some() || r.param("clearguarded").is_some(),
        }));
    }
    if r.param("refreshes_skill_uses").is_some() {
        if !flag(r, "refreshes_skill_uses", false)? {
            return Err("refreshes_skill_uses must be true".into());
        }
        ops.push(make(EffectKind::RefreshSkills));
    }
    if ops.is_empty() {
        return Err(format!("{name}: no supported operation"));
    }
    if !result.conditions.is_empty() && ops.iter().any(|e| !matches!(e.kind, EffectKind::Buffs(_)))
    {
        return Err("conditional composite requires Buff operations".into());
    }
    if ops.len() == 1 {
        return Ok(ops.remove(0));
    }
    result.kind = EffectKind::Composite(ops);
    Ok(result)
}
