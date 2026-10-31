use super::*;
use crate::special::BossOp;
const OPS: &[&str] = &[
    "capture",
    "immobilize",
    "unimmobilize",
    "performer_rank_target",
    "clear_rank_target",
    "bark",
    "steal_buff_source_type",
    "control",
    "actor_dot",
    "health_damage",
    "instant_shuffle",
    "dotShuffle",
];
pub(super) fn applies(r: &Record) -> bool {
    if text(r, "name")
        .is_ok_and(|n| ["Tongue Hit", "red seed attack heal", "Average Feed Debuff"].contains(&n))
    {
        return true;
    }
    OPS.iter().any(|k| {
        r.param(k).is_some()
            && (*k != "health_damage" || text(r, "damage_source_type").ok() != Some("trinket"))
    }) && text(r, "target").ok() != Some("target_enemy_rank")
}
pub(super) fn parse(g: &GameData, r: &Record, name: &str) -> Result<Effect> {
    let common = [
        "name",
        "target",
        "chance",
        "on_hit",
        "on_miss",
        "apply_once",
        "can_apply_on_death",
        "queue",
        "curio_result_type",
        "duration",
        "apply_with_result",
    ];
    let mut operations =
        if ["Tongue Hit", "red seed attack heal", "Average Feed Debuff"].contains(&name) {
            vec![BossOp::Presentation]
        } else {
            vec![]
        };
    let mut extra = vec![];
    if name == "Average Feed Debuff" {
        if integer(r, "combat_stat_buff")? != 1 {
            return Err("invalid neutral feed buff".into());
        }
        extra.push("combat_stat_buff");
    }

    if r.param("dotShuffle").is_some() {
        extra.push("dotShuffle");
        if integer(r, "dotShuffle")? != 1 {
            return Err("invalid shuffle DoT flag".into());
        }
        operations.push(BossOp::ShuffleDot {
            turns: u32::try_from(integer(r, "duration")?)
                .map_err(|_| "invalid shuffle duration")?,
        });
    }
    if r.param("control").is_some() {
        extra.push("control");
        operations.push(BossOp::Control {
            turns: u32::try_from(integer(r, "control")?).map_err(|_| "invalid control duration")?,
        });
    }
    if r.param("actor_dot").is_some() {
        extra.push("actor_dot");
        operations.push(BossOp::ActorDot(actor_dot(g, text(r, "actor_dot")?)?));
    }
    if r.param("instant_shuffle").is_some() {
        extra.push("instant_shuffle");
        if integer(r, "instant_shuffle")? != 1 {
            return Err("invalid instant shuffle".into());
        }
        operations.push(BossOp::InstantShuffle);
    }
    if r.param("health_damage").is_some() {
        extra.extend([
            "health_damage",
            "damage_type",
            "damage_source_type",
            "damage_source_data",
        ]);
        if r.param("damage_type").is_some()
            && !matches!(text(r, "damage_type")?, "attack" | "additionaleffect")
        {
            return Err("unsupported health damage type".into());
        }
        if r.param("damage_source_type").is_some()
            && !matches!(text(r, "damage_source_type")?, "monster" | "trinket")
        {
            return Err("unsupported health damage source".into());
        }
        if r.param("damage_source_data").is_some() {
            text(r, "damage_source_data")?;
        }
        operations.push(BossOp::Damage(integer(r, "health_damage")?));
    }
    if r.param("capture").is_some() {
        if integer(r, "capture")? != 1 {
            return Err("invalid capture flag".into());
        }
        extra.extend([
            "capture",
            "capture_remove_from_party",
            "virtue_blockable_chance",
        ]);
        let remove = if r.param("capture_remove_from_party").is_some() {
            let n = integer(r, "capture_remove_from_party")?;
            if ![0, 1].contains(&n) {
                return Err("invalid capture remove".into());
            }
            n == 1
        } else {
            false
        };
        operations.push(BossOp::Capture {
            remove,
            virtue_block: r
                .param("virtue_blockable_chance")
                .map(|_| number(r, "virtue_blockable_chance"))
                .transpose()?
                .unwrap_or(0.),
        });
    }
    for (k, enable) in [("immobilize", true), ("unimmobilize", false)] {
        if r.param(k).is_some() {
            extra.push(k);
            if integer(r, k)? != 1 {
                return Err("invalid immobilize flag".into());
            }
            operations.push(BossOp::Immobilize {
                enable,
                duration: r
                    .param("duration")
                    .map(|_| {
                        integer(r, "duration").and_then(|n| {
                            u32::try_from(n).map_err(|_| "invalid immobilize duration".into())
                        })
                    })
                    .transpose()?,
            });
        }
    }
    if r.param("performer_rank_target").is_some() {
        extra.push("performer_rank_target");
        if integer(r, "performer_rank_target")? != 1 {
            return Err("invalid rank target flag".into());
        }
        operations.push(BossOp::RankTarget);
    }
    if r.param("clear_rank_target").is_some() {
        extra.push("clear_rank_target");
        operations.push(BossOp::ClearRanks(
            Target::parse(text(r, "clear_rank_target")?)?.ranks,
        ));
    }
    if r.param("steal_buff_source_type").is_some() {
        extra.push("steal_buff_source_type");
        if text(r, "steal_buff_source_type")? != "bsrc_skill" {
            return Err("unsupported stolen buff source".into());
        }
        operations.push(BossOp::StealSkillBuffs);
    }
    if r.param("bark").is_some() {
        extra.push("bark");
        operations.push(BossOp::Bark(text(r, "bark")?.into()));
    }
    // Bark can accompany a real buff: recursively parse the remaining operation,
    // preserving filters instead of consuming an extra chance roll for that buff.
    let mut remaining = r.clone();
    remaining
        .params
        .retain(|p| !extra.contains(&p.name.as_str()));
    let has_remaining = remaining
        .params
        .iter()
        .any(|p| !common.contains(&p.name.as_str()));
    let mut children = if has_remaining {
        vec![if extended::applies(&remaining) {
            extended::parse(g, &remaining, name)?
        } else {
            g.effect_legacy(&remaining, name)?
        }]
    } else {
        vec![]
    };
    for p in &r.params {
        if !common.contains(&p.name.as_str()) && !extra.contains(&p.name.as_str()) && !has_remaining
        {
            return Err(format!("unknown boss effect field {}", p.name));
        }
    }
    if r.param("apply_with_result").is_some() {
        flag(r, "apply_with_result", false)?;
    }
    flag(r, "queue", false)?;
    let mut stub = r.clone();
    stub.params.retain(|p| {
        common.contains(&p.name.as_str())
            && !["queue", "apply_with_result", "duration"].contains(&p.name.as_str())
    });
    stub.params.push(crate::darkest::Param {
        name: "heal".into(),
        values: vec![crate::darkest::Value {
            text: "0".into(),
            quoted: false,
        }],
    });
    let base = g.effect_legacy(&stub, name)?;
    for op in operations {
        op.validate()?;
        let mut e = base.clone();
        e.kind = if flag(r, "queue", false)? {
            EffectKind::Lifecycle(crate::lifecycle::LifecycleEffect {
                op: crate::lifecycle::LifeOp::Boss(op),
                queued: true,
                requires_kill: false,
            })
        } else {
            EffectKind::Boss(op)
        };
        children.push(e);
    }
    if children.len() == 1 {
        return Ok(children.remove(0));
    }
    if children.is_empty() {
        return Err("empty boss effect".into());
    }
    let mut result = base;
    result.kind = EffectKind::Composite(children);
    Ok(result)
}

fn actor_dot(g: &GameData, id: &str) -> Result<crate::special::ActorDot> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Library {
        entries: Vec<Entry>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry {
        id: String,
        update_duration_type: String,
        duration_elements: Vec<Step>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Step {
        completion_chance: f64,
        completion_effects: Vec<String>,
        increment_effects: Vec<String>,
    }
    let mut entries = vec![];
    for path in g.overlay.combat_files("raid/actor_dot/", ".actor_dot.json") {
        for bytes in g.overlay.read_versions(&path)? {
            let raw: Library =
                serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))?;
            for e in raw.entries.into_iter().filter(|e| e.id == id) {
                if e.update_duration_type != "after_turn_attack" {
                    return Err("unsupported actor DoT update clock".into());
                }
                for name in e
                    .duration_elements
                    .iter()
                    .flat_map(|s| s.completion_effects.iter().chain(&s.increment_effects))
                {
                    if g.effects
                        .records_keyed("effect")
                        .filter(|r| text(r, "name").ok() == Some(name))
                        .any(|r| r.param("actor_dot").is_some())
                    {
                        return Err("recursive actor DoT stage reference".into());
                    }
                }
                let def = crate::special::ActorDot {
                    id: e.id,
                    stages: e
                        .duration_elements
                        .into_iter()
                        .map(|s| {
                            Ok(crate::special::ActorDotStage {
                                chance: s.completion_chance,
                                completion: s
                                    .completion_effects
                                    .iter()
                                    .map(|s| g.effect(s))
                                    .collect::<Result<Vec<_>>>()?,
                                increment: s
                                    .increment_effects
                                    .iter()
                                    .map(|s| g.effect(s))
                                    .collect::<Result<Vec<_>>>()?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                };
                def.validate()?;
                if !entries.contains(&def) {
                    entries.push(def);
                }
            }
        }
    }
    if entries.len() != 1 {
        return Err(format!("missing or ambiguous actor DoT {id}"));
    }
    Ok(entries.remove(0))
}
