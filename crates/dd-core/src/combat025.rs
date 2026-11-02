//! Analytic legal-action comparison. No cloned battle, queue, RNG, or hidden wave.
use super::*;
use crate::{Action as Fight, VisibleUnit};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    pub key: String,
    pub round: u32,
    pub wave: usize,
    pub recovery_streak: u32,
}
pub fn document() -> Result<crate::tactics021::Document> {
    let d: crate::tactics021::Document =
        serde_json::from_str(r#"{
  "version": "visible-combat-025-v1",
  "threats": [],
  "safe_tail": [],
  "rules": [
    {
      "id": "S025-30",
      "priority": 30,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Rescue",
      "action": "Heal",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "compare legal rescue, guard and movement",
      "source": "W-005,W-099",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-31",
      "priority": 31,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Cleanse",
      "action": "Cleanse",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "clear lethal visible DOT",
      "source": "W-005,W-068",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-32",
      "priority": 32,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "visible threat reduction; backline only ties",
      "source": "W-004,W-007,W-048,W-098",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-33",
      "priority": 33,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Stun",
      "action": "Stun",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "source control probability candidate",
      "source": "W-004,W-007",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-34",
      "priority": 34,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "class recovery and finisher positions",
      "source": "W-010,W-070,W-071,W-074,W-076",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-35",
      "priority": 35,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Tail",
      "action": "Recover",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "low threat recovery with source-invalidating corpse attacks",
      "source": "W-008,W-011,W-014,W-040,W-055,W-089,W-090",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-36",
      "priority": 36,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "functional boss harm reduction",
      "source": "W-009,W-012,W-102",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-37",
      "priority": 37,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Heal",
      "action": "Heal",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "guard move heal landing comparison",
      "source": "W-005,W-099",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-38",
      "priority": 38,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Retreat",
      "action": "Retreat",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "multiple visible death-door heroes; legal retreat",
      "source": "W-015,W-099,W-105",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-39",
      "priority": 39,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "hostile source damage reduction; exclude performer penalties",
      "source": "W-007,W-009,W-098",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    }
  ]
}
"#)
            .map_err(|e| e.to_string())?;
    let p = Profile::documented()?;
    let mut ids = BTreeSet::new();
    if d.version != "visible-combat-025-v1"
        || d.rules.is_empty()
        || d.rules.iter().any(|r| {
            !p.rules.contains_key(&r.id)
                || !ids.insert(r.id.clone())
                || r.source.is_empty()
                || r.rationale.is_empty()
                || !(0. ..=1.).contains(&r.parameters.hp_fraction)
        })
    {
        return Err("invalid025 tactics document".into());
    }
    Ok(d)
}
fn n(ctx: &Context<'_>, k: &str) -> f64 {
    ctx.profile.parameters["combat"][k].as_f64().unwrap_or(0.)
}
fn pressure(a: &threat::Assessment, ctx: &Context<'_>) -> f64 {
    a.damage * n(ctx, "damage_weight")
        + a.stress * n(ctx, "stress_weight")
        + a.support * n(ctx, "support_weight")
        + a.lethal_risk * n(ctx, "lethal_weight")
}
fn incoming(o: &crate::Observation, h: &VisibleUnit, rank: u8, ctx: &Context<'_>) -> f64 {
    o.units
        .iter()
        .filter(|u| !u.hero && !u.corpse && u.hp > 0)
        .map(|u| {
            ctx.sources.enemies.get(&threat::key(u)).map_or(0., |e| {
                let turns = u32::from(e.turns).saturating_sub(u32::from(u.stunned));
                e.skills
                    .iter()
                    .filter(|s| {
                        s.launch.iter().any(|r| u.occupied_ranks.contains(r))
                            && (rank..rank + h.size).any(|r| s.ranks.contains(&r))
                    })
                    .map(|s| s.damage * f64::from(turns))
                    .fold(0f64, f64::max)
            })
        })
        .sum()
}
pub(super) fn cures_present029(label: &str, bleed: bool, blight: bool) -> bool {
    match label {
        "bandage" => bleed,
        "antivenom" => blight,
        "battlefield_medicine" => bleed || blight,
        _ => false,
    }
}
pub fn choose(
    o: &crate::Observation,
    m: &mut super::Memory,
    ctx: &Context<'_>,
) -> Result<Option<(Fight, String)>> {
    choose_inner(o, m, ctx, None)
}
///026 uses the unchanged rule scores; calling the original choose path has no collector.
pub fn ranked026(
    o: &crate::Observation,
    m: &super::Memory,
    ctx: &Context<'_>,
) -> Result<Vec<(f64, crate::LegalAction, String)>> {
    let mut rows = vec![];
    let mut memory = m.clone();
    choose_inner(o, &mut memory, ctx, Some(&mut rows))?;
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(rows)
}
fn choose_inner(
    o: &crate::Observation,
    m: &mut super::Memory,
    ctx: &Context<'_>,
    mut scores: Option<&mut Vec<(f64, crate::LegalAction, String)>>,
) -> Result<Option<(Fight, String)>> {
    if o.player_selection || o.outcome.is_some() {
        return Ok(None);
    }
    let Some(actor) = o
        .units
        .iter()
        .find(|u| Some(u.id) == o.actor && u.hero && !u.corpse)
    else {
        return Ok(None);
    };
    if !["combat", "stress", "boss"].iter().any(|g| ctx.on(g)) {
        return Ok(None);
    }
    let assessments = threat::evaluate(o, &ctx.sources.enemies)?;
    let living = assessments.len();
    let friends = o
        .units
        .iter()
        .filter(|u| u.hero && !u.corpse)
        .collect::<Vec<_>>();
    if scores.is_none()
        && ctx.on("combat")
        && friends.iter().filter(|h| h.hp == 0).count() >= 2
        && o.actions.iter().any(|a| a.action == Fight::Retreat)
    {
        return Ok(Some((Fight::Retreat, ctx.rule("S025-38")?)));
    }
    let key = o
        .units
        .iter()
        .filter(|u| !u.hero && !u.corpse)
        .map(|u| format!("{}:{}", u.id, u.class))
        .collect::<Vec<_>>()
        .join(";");
    if m.combat.key != key || o.round < m.combat.round || o.wave != m.combat.wave {
        m.combat = Memory {
            key,
            round: o.round,
            wave: o.wave,
            ..Default::default()
        };
    }
    m.combat.round = o.round;
    let minhp = friends.iter().map(|h| h.hp.max(1)).min().unwrap_or(1);
    let tail = ctx.on("stress")
        && living > 0
        && living <= n(ctx, "tail_enemy_limit") as usize
        && o.round
            <= ctx
                .sources
                .tactics
                .rules
                .iter()
                .find(|r| r.id == "S025-35")
                .ok_or("025 missing tail rule")?
                .parameters
                .max_round
        && assessments.iter().map(|a| a.damage).sum::<f64>()
            <= f64::from(minhp) * n(ctx, "tail_hp_fraction")
        && assessments.iter().map(|a| a.stress).sum::<f64>() <= n(ctx, "tail_stress_limit")
        && assessments.iter().all(|a| !a.uncertain && a.support == 0.)
        && o.units
            .iter()
            .filter(|u| !u.hero && !u.corpse && u.hp > 0)
            .all(|u| {
                ctx.sources
                    .enemies
                    .get(&u.class)
                    .is_some_and(|e| !e.accelerate && !e.boss)
            });
    if scores.is_none()
        && ctx.on("combat")
        && ctx.on("boss")
        && actor.class == "vestal"
        && actor.rank == 3
        && ctx.sources.current_boss.is_some()
        && ctx.profile.parameters["sync026"]["boss"]["enabled"] == true
    {
        let critical = friends
            .iter()
            .any(|h| f64::from(h.hp) <= f64::from(h.max_hp) * n(ctx, "self_heal_hp_fraction"));
        let low = assessments.iter().map(|a| a.damage).sum::<f64>()
            <= f64::from(minhp) * n(ctx, "tail_hp_fraction")
            && assessments.iter().map(|a| a.stress).sum::<f64>() <= n(ctx, "tail_stress_limit");
        if (critical || low)
            && let Some(a) = o
                .actions
                .iter()
                .filter(|a| a.heal[1] > 0)
                .max_by(|a, b| {
                    let value = |a: &crate::LegalAction| {
                        a.targets
                            .iter()
                            .filter_map(|id| friends.iter().find(|h| h.id == *id))
                            .map(|h| {
                                f64::from((h.max_hp - h.hp).max(0))
                                    .min(f64::from(a.heal[0] + a.heal[1]) / 2.)
                            })
                            .sum::<f64>()
                    };
                    value(a).total_cmp(&value(b))
                })
                .filter(|a| {
                    a.targets
                        .iter()
                        .any(|id| friends.iter().any(|h| h.id == *id && h.hp < h.max_hp))
                })
        {
            return Ok(Some((a.action.clone(), ctx.rule("S026-02")?)));
        }
        if let Some(a) = o
            .actions
            .iter()
            .filter(|a| a.label == "judgement")
            .max_by(|a, b| {
                let value = |a: &crate::LegalAction| {
                    a.hit_chance * f64::from(a.damage[0] + a.damage[1]) / 2.
                };
                value(a).total_cmp(&value(b))
            })
        {
            return Ok(Some((a.action.clone(), ctx.rule("S026-02")?)));
        }
    }
    let tail_rule = ctx
        .sources
        .tactics
        .rules
        .iter()
        .find(|r| r.id == "S025-35")
        .ok_or("025 missing tail rule")?;
    let mut best = None;
    for a in &o.actions {
        if matches!(
            a.action,
            Fight::Retreat | Fight::Pass | Fight::Sacrifice { .. }
        ) {
            continue;
        }
        let ability = ctx
            .sources
            .cards
            .skills
            .get(&actor.class)
            .and_then(|s| s.get(&a.label));
        let mut score = 0.;
        let mut rule = "S025-32";
        let mut healing = false;
        let mut rescue = false;
        if ctx.on("combat") {
            for id in &a.targets {
                let Some(u) = o.units.iter().find(|u| u.id == *id) else {
                    continue;
                };
                if u.hero && !u.corpse {
                    let heal = if a.heal[1] > 0 {
                        f64::from(a.heal[0] + a.heal[1]) / 2.
                    } else {
                        ability.map_or(0., |s| s.healing + s.healing_percent * f64::from(u.max_hp))
                    };
                    let benefit = heal.min(f64::from((u.max_hp - u.hp).max(0)));
                    if benefit > 0. {
                        healing = true;
                        score += benefit * n(ctx, "heal_weight");
                        rule = "S025-37";
                        if u.hp == 0 {
                            score += n(ctx, "rescue_bonus");
                            rescue = true;
                            rule = "S025-30"
                        }
                    }
                    if ability.is_some_and(|a| a.guard) && u.id != actor.id {
                        let reverse = ctx
                            .sources
                            .action_model
                            .reverse_guards
                            .contains(&(actor.class.clone(), a.label.clone()));
                        let (protected, guarder) = if reverse { (actor, u) } else { (u, actor) };
                        if protected.guarded_by.is_none() {
                            let single = assessments
                                .iter()
                                .filter(|a| !a.aoe && a.targets.contains(&protected.id))
                                .map(|a| a.damage)
                                .sum::<f64>();
                            let improvement = (single / f64::from(protected.hp.max(1))
                                - single / f64::from(guarder.hp.max(1)))
                            .max(0.);
                            score += improvement * n(ctx, "guard_weight");
                            rule = "S025-37";
                        }
                    }
                    if ["bandage", "antivenom", "battlefield_medicine"].contains(&a.label.as_str())
                        && if ctx.profile.parameters["coherence029"] == true {
                            cures_present029(&a.label, !u.bleed.is_empty(), !u.blight.is_empty())
                        } else {
                            !u.bleed.is_empty() || !u.blight.is_empty()
                        }
                        && u.hp
                            <= ((u.bleed.iter().chain(&u.blight).map(|(n, _)| n).sum::<i32>())
                                .max(1))
                    {
                        score += n(ctx, "rescue_bonus");
                        rule = "S025-31";
                        rescue = true;
                    }
                } else if !u.hero && !u.corpse && u.hp > 0 {
                    let threat = assessments
                        .iter()
                        .find(|t| t.unit == u.id)
                        .ok_or("025 threat target missing")?;
                    if threat.dot_lethal {
                        continue;
                    }
                    let damage = f64::from(a.damage[0] + a.damage[1]) / 2. * a.hit_chance;
                    let reduction = (damage / f64::from(u.hp.max(1))).clamp(0., 1.);
                    score += damage
                        + pressure(threat, ctx) * reduction * n(ctx, "threat_reduction_weight");
                    if threat.dot_lethal {
                        score -= n(ctx, "dot_dead_attack_penalty");
                    }
                    if let Some(control) = ctx
                        .sources
                        .action_model
                        .controls
                        .get(&(actor.class.clone(), a.label.clone()))
                    {
                        let enemy = ctx
                            .sources
                            .enemies
                            .get(&threat::key(u))
                            .ok_or("025 control source missing")?;
                        let stun = if u.stunned {
                            0.
                        } else {
                            (control[0] - enemy.stun_resist).clamp(0., 1.)
                        };
                        let movement = (control[1] - enemy.move_resist).clamp(0., 1.);
                        score += pressure(threat, ctx)
                            * stun.max(movement)
                            * a.hit_chance
                            * n(ctx, "control_weight");
                        rule = if stun >= movement {
                            "S025-33"
                        } else {
                            "S025-39"
                        };
                    }
                    if let Some(reduction) = ctx
                        .sources
                        .action_model
                        .damage_reduction
                        .get(&(actor.class.clone(), a.label.clone()))
                    {
                        score += pressure(threat, ctx)
                            * reduction
                            * a.hit_chance
                            * n(ctx, "control_weight");
                        rule = "S025-39";
                    }
                    score += f64::from(u.rank) * n(ctx, "backline_tie");
                }
            }
            let landing = match a.action {
                Fight::Move { rank } => Some(rank),
                Fight::Skill { .. } | Fight::SkillAuto { .. } if !actor.immobilized => ctx
                    .sources
                    .action_model
                    .movement
                    .get(&(actor.class.clone(), a.label.clone()))
                    .map(|m| {
                        let max = o
                            .units
                            .iter()
                            .filter(|h| h.hero && !h.corpse)
                            .map(|h| h.rank + h.size - 1)
                            .max()
                            .unwrap_or(1);
                        actor
                            .rank
                            .saturating_sub(m[0] as u8)
                            .saturating_add(m[1] as u8)
                            .clamp(1, max)
                    }),
                _ => None,
            };
            if let Some(rank) = landing.filter(|r| *r != actor.rank) {
                score += landing_benefit(o, actor, rank, ctx) * n(ctx, "move_weight");
            }
            if a.label == "holy_lance" && actor.rank >= 3 {
                score += n(ctx, "combo_bonus");
                rule = "S025-34";
            }
            if a.label == "heroic_end" {
                if o.round < n(ctx, "finisher_after_round") as u32 {
                    score -= n(ctx, "rescue_bonus");
                } else {
                    score += n(ctx, "combo_bonus");
                }
                rule = "S025-34";
            }
            if a.label == "dirk_stab" && actor.class == "jester" && actor.rank > 2 {
                score += n(ctx, "combo_bonus");
                rule = "S025-34";
            }
            if a.label == "battle_ballad" && o.round <= n(ctx, "early_buff_rounds") as u32 {
                score += n(ctx, "early_buff_bonus");
                rule = "S025-34";
            }
            if a.label == "transform"
                && living >= n(ctx, "transform_enemy_count") as usize
                && friends.iter().all(|h| {
                    h.stress < n(ctx, "transform_safe_stress") as i32 && h.hp * 2 > h.max_hp
                })
            {
                score += n(ctx, "transform_bonus");
                rule = "S025-34";
            }
        }
        if ctx.on("combat")
            && let Some(recovery) = ctx
                .sources
                .self_recovery
                .get(&(actor.class.clone(), a.label.clone()))
        {
            let heal = (recovery[0] + recovery[1] * f64::from(actor.max_hp))
                .min(f64::from((actor.max_hp - actor.hp).max(0)));
            let stress = recovery[2].min(f64::from(actor.stress.max(0)));
            if (f64::from(actor.hp) <= f64::from(actor.max_hp) * n(ctx, "self_heal_hp_fraction")
                || actor.stress >= n(ctx, "transform_safe_stress") as i32)
                && (heal > 0. || stress > 0.)
            {
                score += if a.heal[1] == 0 {
                    heal * n(ctx, "heal_weight")
                } else {
                    0.
                } + if a.stress_heal == 0 { stress } else { 0. };
                healing = true;
                rule = "S025-34";
                if actor.hp == 0 && heal > 0. {
                    score += n(ctx, "rescue_bonus");
                    rescue = true;
                    rule = "S025-30";
                }
            }
        }
        if ctx.on("boss")
            && ctx
                .sources
                .damage_debuff
                .contains(&(actor.class.clone(), a.label.clone()))
            && a.targets.iter().any(|id| {
                o.units.iter().find(|u| u.id == *id).is_some_and(|u| {
                    ctx.sources
                        .enemies
                        .get(&threat::key(u))
                        .is_some_and(|e| e.boss)
                })
            })
        {
            score += n(ctx, "boss_debuff_bonus") * a.hit_chance;
            rule = "S025-36";
        }
        if tail {
            if m.combat.recovery_streak < n(ctx, "recovery_streak_limit") as u32 {
                let recovery = a
                    .targets
                    .iter()
                    .filter_map(|id| o.units.iter().find(|h| h.id == *id && h.hero && !h.corpse))
                    .map(|h| {
                        f64::from(if h.stress >= tail_rule.parameters.recovery_stress {
                            h.stress.min(a.stress_heal.max(0))
                        } else {
                            0
                        }) + f64::from((h.max_hp - h.hp).max(0).min(a.heal[1].max(0)))
                    })
                    .sum::<f64>();
                if recovery > 0. {
                    score += recovery + n(ctx, "tail_recovery_bonus");
                    healing = true;
                    rule = "S025-35";
                }
            }
            if ctx
                .sources
                .invalidating
                .contains(&(actor.class.clone(), a.label.clone()))
                && a.targets
                    .iter()
                    .any(|id| o.units.iter().any(|u| u.id == *id && u.corpse && !u.hero))
            {
                score += n(ctx, "corpse_invalidating_bonus");
                rule = "S025-35";
            }
            if healing
                && !rescue
                && m.combat.recovery_streak >= n(ctx, "recovery_streak_limit") as u32
            {
                continue;
            }
        }
        let row = ctx
            .sources
            .tactics
            .rules
            .iter()
            .find(|r| r.id == rule)
            .ok_or_else(|| format!("025 action rule absent {rule}"))?;
        if !row.classes.is_empty() && !row.classes.contains(&actor.class) {
            continue;
        }
        if !row.skills.is_empty()
            && !row
                .skills
                .iter()
                .any(|s| s.class == actor.class && s.id == a.label)
        {
            continue;
        }
        if !row.encounters.is_empty()
            && !o
                .units
                .iter()
                .any(|u| !u.hero && row.encounters.contains(&u.class))
        {
            continue;
        }
        if rule == "S025-33" && a.hit_chance < row.parameters.min_hit {
            continue;
        }
        if let Some(rows) = &mut scores {
            rows.push((score, a.clone(), rule.into()));
        }
        if score > 0. && best.as_ref().is_none_or(|(s, _, _, _)| score > *s) {
            best = Some((score, a.clone(), rule, healing));
        }
    }
    if let Some((_, a, rule, _)) = best {
        return Ok(Some((a.action, ctx.rule(rule)?)));
    }
    if tail && m.combat.recovery_streak >= n(ctx, "recovery_streak_limit") as u32 {
        if let Some(a) = o
            .actions
            .iter()
            .filter(|a| a.heal[1] == 0 && a.stress_heal == 0 && a.damage[1] > 0)
            .max_by_key(|a| a.damage[1])
        {
            return Ok(Some((a.action.clone(), ctx.rule("S025-35")?)));
        }
        if o.actions.iter().any(|a| a.action == Fight::Pass) {
            return Ok(Some((Fight::Pass, ctx.rule("S025-35")?)));
        }
    }
    Ok(None)
}
pub fn raid(o: &Observation, m: &mut super::Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if let Some(r) = &o.expedition {
        if let Some(b) = &r.battle
            && let Some((a, id)) = choose(b, m, ctx)?
        {
            return proposal(
                o,
                Action::Raid(crate::opening015::Action::Fight(a)),
                &id,
                "combat",
                ctx,
            )
            .map(Some);
        }
    } else if let Some(f) = o.content019.as_ref().and_then(|c| c.farm.as_ref())
        && f.interlude.is_none()
        && f.camping.is_none()
        && f.loot.is_empty()
        && let Some((a, id)) = choose(&f.battle, m, ctx)?
    {
        return proposal(
            o,
            Action::Farm(crate::campaign017::content019::farm::Action::Fight(a)),
            &id,
            "farm_combat",
            ctx,
        )
        .map(Some);
    }
    Ok(None)
}

pub type SkillIds = BTreeSet<(String, String)>;
pub fn project_skills(game: &GameData) -> Result<(SkillIds, SkillIds)> {
    let mut invalidating = BTreeSet::new();
    let mut damage_debuff = BTreeSet::new();
    for class in game.hero_ids() {
        for id in game.hero_skill_ids(class)? {
            let bp = game.hero(class, 0, 0, &[&id])?;
            let s = bp
                .skills
                .iter()
                .find(|s| s.id == id)
                .ok_or("025 missing named hero skill")?;
            if s.tactical.stall_invalidating {
                invalidating.insert((class.clone(), id.clone()));
            }
            if s.effects
                .iter()
                .map(|e| damage_reduction(e, !s.target.friendly))
                .sum::<f64>()
                > 0.
            {
                damage_debuff.insert((class.clone(), id.clone()));
            }
        }
    }
    Ok((invalidating, damage_debuff))
}

/// Audit every actual hero action, including legacy fallbacks and safeguard actions.
pub fn finish(o: &crate::Observation, action: &Fight, m: &mut super::Memory, sources: &Sources) {
    if let Some(a) = o.actions.iter().find(|a| &a.action == action) {
        let recovery = a.heal[1] > 0
            || a.stress_heal > 0
            || o.units
                .iter()
                .find(|u| Some(u.id) == o.actor)
                .is_some_and(|u| {
                    sources
                        .self_recovery
                        .get(&(u.class.clone(), a.label.clone()))
                        .is_some_and(|r| r.iter().any(|n| *n > 0.))
                });
        m.combat.recovery_streak = if recovery {
            m.combat.recovery_streak.saturating_add(1)
        } else {
            0
        };
    }
}

pub type SelfRecovery = BTreeMap<(String, String), [f64; 3]>;
pub fn project_self_recovery(game: &GameData) -> Result<SelfRecovery> {
    fn visit(e: &dd_data::combat::Effect, own: bool, out: &mut [f64; 3]) {
        use dd_data::combat::{EffectKind as E, EffectTarget};
        let own = own || matches!(e.target, EffectTarget::Performer);
        match &e.kind {
            E::Composite(es) => {
                for e in es {
                    visit(e, own, out)
                }
            }
            E::BattlePersistent(k) | E::InventoryEffect(k) => {
                let mut next = e.clone();
                next.kind = *k.clone();
                visit(&next, own, out)
            }
            E::Heal(n) | E::Regen(n) if own => out[0] += f64::from((*n).max(0)),
            E::HealPercent(n) if own => out[1] += n.max(0.),
            E::Stress(n) if own && *n < 0 => out[2] += f64::from(-n),
            _ => (),
        }
    }
    let mut result = BTreeMap::new();
    for class in game.hero_ids() {
        for id in game.hero_skill_ids(class)? {
            let bp = game.hero(class, 0, 0, &[&id])?;
            let skill = bp
                .skills
                .iter()
                .find(|s| s.id == id)
                .ok_or("025 self-recovery skill absent")?;
            let mut values = [0.; 3];
            for e in &skill.effects {
                visit(
                    e,
                    skill.target.friendly && skill.target.ranks.is_empty(),
                    &mut values,
                )
            }
            if values.iter().any(|n| *n > 0.) {
                result.insert((class.clone(), id), values);
            }
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, Default)]
pub struct ActionModel {
    pub movement: BTreeMap<(String, String), [usize; 2]>,
    pub damage_reduction: BTreeMap<(String, String), f64>,
    pub reverse_guards: BTreeSet<(String, String)>,
    pub controls: BTreeMap<(String, String), [f64; 2]>,
}
fn damage_reduction(e: &dd_data::combat::Effect, hostile: bool) -> f64 {
    use dd_data::combat::{EffectKind as E, EffectTarget as T};
    let hostile = match e.target {
        T::Performer | T::ThisTrinket | T::Party | T::PartyOther => false,
        T::EnemyRandom | T::EnemyRank(_) | T::TargetEnemies | T::Enemies => true,
        T::Target | T::TargetParty => hostile,
    };
    match &e.kind {
        E::Composite(es) => es.iter().map(|e| damage_reduction(e, hostile)).sum(),
        E::BattlePersistent(k) | E::InventoryEffect(k) => {
            let mut next = e.clone();
            next.kind = *k.clone();
            damage_reduction(&next, hostile)
        }
        E::Buffs(bs) | E::ProtectedBuffs(bs) | E::InstantBuffs(bs) if hostile => bs
            .iter()
            .filter(|b| {
                b.stat_type == "combat_stat_multiply"
                    && ["damage_low", "damage_high"].contains(&b.stat_sub_type.as_str())
                    && b.amount < 0.
            })
            .map(|b| -b.amount * e.chance / 2.)
            .sum(),
        _ => 0.,
    }
}
pub fn project_action_model(game: &GameData) -> Result<ActionModel> {
    fn reverse(k: &dd_data::combat::EffectKind) -> bool {
        use dd_data::combat::EffectKind as E;
        match k {
            E::Guard { reverse: r } => *r,
            E::Composite(es) => es.iter().any(|e| reverse(&e.kind)),
            E::BattlePersistent(k) | E::InventoryEffect(k) => reverse(k),
            _ => false,
        }
    }
    let mut result = ActionModel::default();
    for class in game.hero_ids() {
        for id in game.hero_skill_ids(class)? {
            let bp = game.hero(class, 0, 0, &[&id])?;
            let s = bp
                .skills
                .iter()
                .find(|s| s.id == id)
                .ok_or("025 action model skill absent")?;
            let key = (class.clone(), id);
            if s.move_forward > 0 || s.move_back > 0 {
                result
                    .movement
                    .insert(key.clone(), [s.move_forward, s.move_back]);
            }
            let reduction = s
                .effects
                .iter()
                .map(|e| damage_reduction(e, !s.target.friendly))
                .sum::<f64>()
                .clamp(0., 1.);
            if reduction > 0. {
                result.damage_reduction.insert(key.clone(), reduction);
            }
            let mut control = [0.; 2];
            for e in &s.effects {
                control_source(e, !s.target.friendly, &mut control)
            }
            if control.iter().any(|p| *p > 0.) {
                result.controls.insert(key.clone(), control);
            }
            if s.effects.iter().any(|e| reverse(&e.kind)) {
                result.reverse_guards.insert(key);
            }
        }
    }
    Ok(result)
}
fn landing_benefit(
    o: &crate::Observation,
    actor: &VisibleUnit,
    rank: u8,
    ctx: &Context<'_>,
) -> f64 {
    o.units
        .iter()
        .filter(|h| h.hero && !h.corpse)
        .map(|h| {
            let landing = if h.id == actor.id {
                rank
            } else if rank < actor.rank && h.rank >= rank && h.rank < actor.rank {
                h.rank.saturating_add(actor.size)
            } else if rank > actor.rank && h.rank > actor.rank && h.rank <= rank {
                h.rank.saturating_sub(actor.size).max(1)
            } else {
                h.rank
            };
            (incoming(o, h, h.rank, ctx) - incoming(o, h, landing, ctx)) / f64::from(h.hp.max(1))
        })
        .sum()
}

fn control_source(e: &dd_data::combat::Effect, hostile: bool, out: &mut [f64; 2]) {
    use dd_data::combat::{EffectKind as E, EffectTarget as T};
    if !e.conditions.is_empty() {
        return;
    }
    let hostile = match e.target {
        T::Performer | T::ThisTrinket | T::Party | T::PartyOther => false,
        T::EnemyRandom | T::EnemyRank(_) | T::TargetEnemies | T::Enemies => true,
        T::Target | T::TargetParty => hostile,
    };
    match &e.kind {
        E::Composite(es) => {
            for e in es {
                control_source(e, hostile, out)
            }
        }
        E::BattlePersistent(k) | E::InventoryEffect(k) => {
            let mut next = e.clone();
            next.kind = *k.clone();
            control_source(&next, hostile, out)
        }
        E::Stun if hostile => out[0] = out[0].max(e.chance),
        E::Pull(_) | E::Push(_) | E::Shuffle | E::ShuffleParty if hostile => {
            out[1] = out[1].max(e.chance)
        }
        _ => (),
    }
}
