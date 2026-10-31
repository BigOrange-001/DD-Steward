//! Global lighting effects use the existing finite post-skill job queue.
use super::*;
pub(super) fn remove_vampire(r: &Record, name: &str) -> Result<Effect> {
    let keys = [
        "name",
        "target",
        "chance",
        "curio_result_type",
        "remove_vampire",
        "on_hit",
        "on_miss",
        "queue",
    ];
    if r.params.iter().any(|p| !keys.contains(&p.name.as_str()))
        || text(r, "target")? != "performer"
        || integer(r, "remove_vampire")? != 1
    {
        return Err("unsupported vampire cure shape".into());
    }
    flag(r, "queue", false)?;
    Ok(Effect {
        execution: Default::default(),
        name: name.into(),
        kind: EffectKind::RemoveVampire,
        chance: number(r, "chance")?,
        duration: 1,
        target: EffectTarget::Performer,
        on_hit: flag(r, "on_hit", true)?,
        on_miss: flag(r, "on_miss", false)?,
        apply_once: true,
        resist_debuff: false,
        conditions: vec![],
        can_apply_on_death: false,
    })
}
pub(super) fn disease(r: &Record, name: &str) -> Result<Effect> {
    let keys = [
        "name",
        "target",
        "chance",
        "curio_result_type",
        "disease",
        "on_hit",
        "on_miss",
        "crit_doesnt_apply_to_roll",
    ];
    if r.params.iter().any(|p| !keys.contains(&p.name.as_str())) || text(r, "target")? != "target" {
        return Err("unsupported disease shape".into());
    }
    Ok(Effect {
        execution: Default::default(),
        name: name.into(),
        kind: EffectKind::Disease {
            id: text(r, "disease")?.into(),
            crit_bonus: !flag(r, "crit_doesnt_apply_to_roll", false)?,
        },
        chance: number(r, "chance")?,
        duration: 1,
        target: EffectTarget::Target,
        on_hit: flag(r, "on_hit", true)?,
        on_miss: flag(r, "on_miss", false)?,
        apply_once: false,
        resist_debuff: false,
        conditions: vec![],
        can_apply_on_death: false,
    })
}
pub(super) fn rank_burn(r: &Record, name: &str) -> Result<Effect> {
    let keys = [
        "name",
        "item",
        "target",
        "chance",
        "on_hit",
        "on_miss",
        "controlled_burn_amount",
        "controlled_burn_duration",
    ];
    if r.params.iter().any(|p| !keys.contains(&p.name.as_str())) || text(r, "target")? != "target" {
        return Err("unsupported rank burn shape".into());
    }
    let amount = integer(r, "controlled_burn_amount")?;
    let rounds = integer(r, "controlled_burn_duration")?;
    if !(1..=10000).contains(&amount) || !(1..=10000).contains(&rounds) {
        return Err("invalid rank burn".into());
    }
    Ok(Effect {
        execution: Default::default(),
        name: name.into(),
        kind: EffectKind::ControlledBurn {
            amount,
            rounds: rounds as u32,
        },
        chance: number(r, "chance")?,
        duration: 1,
        target: EffectTarget::Target,
        on_hit: flag(r, "on_hit", true)?,
        on_miss: flag(r, "on_miss", false)?,
        apply_once: false,
        resist_debuff: false,
        conditions: vec![],
        can_apply_on_death: false,
    })
}
pub(super) fn special(r: &Record, name: &str) -> Result<Effect> {
    let keys = [
        "name",
        "item",
        "target",
        "chance",
        "on_hit",
        "on_miss",
        "queue",
        "apply_once",
        "can_apply_on_death",
        "curio_result_type",
        "set_mode",
        "steal_buff_stat_type",
    ];
    if r.params.iter().any(|p| !keys.contains(&p.name.as_str())) {
        return Err("unknown class effect field".into());
    }
    let kind = if r.param("set_mode").is_some() {
        let mode = text(r, "set_mode")?;
        if ![
            "human",
            "beast",
            "stance_defensive",
            "stance_aggressive",
            "guard",
            "land",
            "submerged",
            "normal",
            "pawing",
            "onscreen",
            "offscreen",
            "",
        ]
        .contains(&mode)
        {
            return Err("unknown mode".into());
        }
        EffectKind::SetMode(mode.into())
    } else {
        let kind = text(r, "steal_buff_stat_type")?;
        if !["hp_dot_bleed", "hp_dot_poison"].contains(&kind) {
            return Err("unsupported stolen status".into());
        }
        EffectKind::Lifecycle(crate::lifecycle::LifecycleEffect {
            op: crate::lifecycle::LifeOp::StealDots { kind: kind.into() },
            queued: flag(r, "queue", false)?,
            requires_kill: false,
        })
    };
    if r.param("set_mode").is_some() && r.param("steal_buff_stat_type").is_some() {
        return Err("conflicting class operations".into());
    }
    let target = match text(r, "target")? {
        "performer" => EffectTarget::Performer,
        "target" => EffectTarget::Target,
        _ => return Err("unknown class effect target".into()),
    };
    Ok(Effect {
        execution: Default::default(),
        name: name.into(),
        kind,
        chance: number(r, "chance")?,
        duration: 1,
        target,
        on_hit: flag(r, "on_hit", true)?,
        on_miss: flag(r, "on_miss", false)?,
        apply_once: flag(r, "apply_once", false)?,
        can_apply_on_death: flag(r, "can_apply_on_death", false)?,
        resist_debuff: false,
        conditions: vec![],
    })
}
pub(super) fn parse(r: &Record, name: &str) -> Result<Effect> {
    let keys = [
        "name",
        "item",
        "target",
        "chance",
        "torch_increase",
        "torch_decrease",
        "on_hit",
        "on_miss",
        "apply_once",
        "queue",
        "curio_result_type",
        "can_apply_on_death",
    ];
    if let Some(p) = r.params.iter().find(|p| !keys.contains(&p.name.as_str())) {
        return Err(format!(
            "{name}: unsupported lighting effect parameter .{}",
            p.name
        ));
    }
    if !["global", "performer"].contains(&text(r, "target")?)
        || (text(r, "target")? == "global" && !flag(r, "apply_once", false)?)
        || r.param("torch_increase").is_some() == r.param("torch_decrease").is_some()
    {
        return Err("lighting effect requires one global apply-once operation".into());
    }
    if r.param("item").is_some() && integer(r, "item")? != 1 {
        return Err("invalid item effect flag".into());
    }
    let increase = r.param("torch_increase").is_some();
    let amount = integer(
        r,
        if increase {
            "torch_increase"
        } else {
            "torch_decrease"
        },
    )?;
    if !(1..=10000).contains(&amount) {
        return Err("invalid lighting effect amount".into());
    }
    let op = crate::lifecycle::LifeOp::Light {
        amount: if increase { amount } else { -amount },
    };
    op.validate()?;
    Ok(Effect {
        execution: Default::default(),
        name: name.into(),
        kind: EffectKind::Lifecycle(crate::lifecycle::LifecycleEffect {
            op,
            queued: flag(r, "queue", false)?,
            requires_kill: false,
        }),
        chance: number(r, "chance")?,
        duration: 1,
        target: EffectTarget::Performer,
        on_hit: flag(r, "on_hit", true)?,
        on_miss: flag(r, "on_miss", false)?,
        apply_once: true,
        resist_debuff: false,
        conditions: vec![],
        can_apply_on_death: flag(r, "can_apply_on_death", false)?,
    })
}
