//! Strict selected camping operations. Values and Buff durations come from the
//! enabled original libraries, not a second set of gameplay constants.
use crate::{
    campaign017::{fields, uint},
    combat::{GameData, Result},
    curios013::Curios,
    library,
    mechanics::Buff,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub class: String,
    pub cost: u32,
    pub limit: u32,
    pub effects: Vec<Effect>,
    pub groups: Vec<Vec<usize>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub selection: String,
    pub requirements: Vec<String>,
    pub code: String,
    pub chance: f64,
    pub operation: Operation,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operation {
    Heal(f64),
    Damage(f64),
    Stress(i32),
    BleedCure,
    BlightCure,
    RecoveryCure,
    DiseaseCure,
    PreventAmbush(f64),
    Torch(i32),
    Buff(Box<Buff>),
    Loot { table: String, count: u32 },
    RefreshUses,
}
fn amount(v: &Value) -> Result<f64> {
    v["amount"]
        .as_f64()
        .filter(|n| n.is_finite() && n.abs() <= 1000000.)
        .ok_or("invalid original camping amount".into())
}
fn integer(v: &Value) -> Result<i32> {
    let n = amount(v)?;
    if n.fract() != 0. {
        return Err("noninteger camping effect amount".into());
    }
    Ok(n as i32)
}
fn ratio(v: &Value) -> Result<f64> {
    let n = amount(v)?;
    if !(0. ..=1.).contains(&n) {
        return Err("camping percentage outside source domain".into());
    }
    Ok(n)
}
pub fn skill(
    game: &GameData,
    curios: &Curios,
    rows: &BTreeMap<String, Value>,
    class: &str,
    id: &str,
    region: &str,
    difficulty: u32,
) -> Result<Skill> {
    game.hero_document(class)?;
    let raw = library::unique_json(rows, &format!("{class}.{id}"), "camping skill")?;
    fields(
        raw,
        &[
            "id",
            "level",
            "cost",
            "use_limit",
            "effects",
            "hero_classes",
            "upgrade_requirements",
        ],
        "camping skill",
    )?;
    if raw["id"] != id
        || raw["hero_classes"] != serde_json::json!([class])
        || uint(raw, "level")? != 0
    {
        return Err("camping source class/level differs".into());
    }
    let mut effects = vec![];
    for v in raw["effects"].as_array().ok_or("missing camping effects")? {
        fields(
            v,
            &[
                "selection",
                "requirements",
                "chance",
                "type",
                "sub_type",
                "amount",
            ],
            "camping effect",
        )?;
        let selection = v["selection"]
            .as_str()
            .filter(|s| ["self", "party", "party_other", "individual"].contains(s))
            .ok_or("unknown camping selection")?
            .to_string();
        let requirements = v["requirements"]
            .as_array()
            .ok_or("missing camping requirements")?
            .iter()
            .map(|r| {
                r.as_str()
                    .filter(|s| {
                        [
                            "religious",
                            "not_religious",
                            "afflicted",
                            "has_deaths_door_recovery_buffs",
                        ]
                        .contains(s)
                    })
                    .map(str::to_string)
                    .ok_or_else(|| "unknown camping requirement".to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        fields(&v["chance"], &["code", "amount"], "camping chance")?;
        let code = v["chance"]["code"]
            .as_str()
            .filter(|s| {
                !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric())
            })
            .ok_or("invalid camping chance group")?
            .to_string();
        let chance = ratio(&v["chance"])?;
        let kind = v["type"].as_str().ok_or("missing camping operation")?;
        let sub = v["sub_type"].as_str().ok_or("missing camping subtype")?;
        if !["buff", "loot"].contains(&kind) && !sub.is_empty() {
            return Err("unexpected camping effect subtype".into());
        }
        let zero = || -> Result<()> {
            if amount(v)? != 0. {
                return Err("unexpected camping flag amount".into());
            }
            Ok(())
        };
        let operation = match kind {
            "stress_heal_amount" => Operation::Stress(-integer(v)?),
            "stress_damage_amount" => Operation::Stress(integer(v)?),
            "health_heal_max_health_percent" => Operation::Heal(ratio(v)?),
            "health_damage_max_health_percent" => Operation::Damage(ratio(v)?),
            "remove_bleeding" => {
                zero()?;
                Operation::BleedCure
            }
            "remove_poison" => {
                zero()?;
                Operation::BlightCure
            }
            "remove_deaths_door_recovery_buffs" => {
                zero()?;
                Operation::RecoveryCure
            }
            "remove_disease" => {
                zero()?;
                Operation::DiseaseCure
            }
            "refresh_camp_skill_uses" => {
                zero()?;
                Operation::RefreshUses
            }
            "reduce_ambush_chance" => Operation::PreventAmbush(ratio(v)?),
            "reduce_torch" => Operation::Torch(-integer(v)?),
            "buff" => {
                let outside = game.noncombat_buff(sub)?;
                let mut b = if outside {
                    crate::mechanics::read_exploration_buff(&game.buffs, sub)?
                } else {
                    crate::mechanics::read_buff(&game.buffs, sub)?
                };
                if !matches!(b.duration_type.as_deref(), Some("combat_end" | "quest_end"))
                    || b.duration.is_some_and(|n| n == 0 || n > 10000)
                    || (b.duration_type.as_deref() == Some("combat_end") && b.duration.is_none())
                {
                    return Err("unsupported camping Buff lifetime".into());
                }
                b.amount = amount(v)?;
                if outside {
                    b.validate_condition()?;
                } else {
                    b.validate()?;
                }
                Operation::Buff(Box::new(b))
            }
            "loot" => {
                let n = uint(v, "amount")?;
                if n == 0 || n > 1000 {
                    return Err("invalid camping loot count".into());
                }
                curios.validate_campaign_draw(sub, i64::from(difficulty), region, &mut vec![])?;
                Operation::Loot {
                    table: sub.into(),
                    count: n,
                }
            }
            other => return Err(format!("unknown camping operation {other}")),
        };
        effects.push(Effect {
            selection,
            requirements,
            code,
            chance,
            operation,
        });
    }
    if effects.is_empty() || uint(raw, "use_limit")? == 0 {
        return Err("empty/unlimited selected camping skill".into());
    }
    let mut groups = BTreeMap::<&str, (f64, &str)>::new();
    for e in &effects {
        let (total, selection) = groups.entry(&e.code).or_insert((0., &e.selection));
        if *selection != e.selection {
            return Err("one camping chance group has mixed target selections".into());
        }
        *total += e.chance;
        if *total > 1. + f64::EPSILON {
            return Err("camping chance alternatives exceed one".into());
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![];
    for (i, e) in effects.iter().enumerate() {
        if let Some(group) = groups.iter_mut().find(|g| effects[g[0]].code == e.code) {
            group.push(i);
        } else {
            groups.push(vec![i]);
        }
    }
    // The original field-dressing rows use two different codes for the two
    // alternative heals. This precise source case is an explicit 017 candidate.
    if id == "field_dressing" {
        let heals = effects
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.operation, Operation::Heal(_)))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if heals.len() != 2
            || effects[heals[0]].selection != effects[heals[1]].selection
            || (effects[heals[0]].chance + effects[heals[1]].chance - 1.).abs() > f64::EPSILON
        {
            return Err("unknown field-dressing probability layout".into());
        }
        let first = groups.iter().position(|g| g.contains(&heals[0])).unwrap();
        let second = groups.iter().position(|g| g.contains(&heals[1])).unwrap();
        if first != second {
            if groups[first].len() != 1 || groups[second].len() != 1 {
                return Err("mixed field-dressing chance groups".into());
            }
            groups[first].push(heals[1]);
            groups.remove(second);
        }
    }
    Ok(Skill {
        id: id.into(),
        class: class.into(),
        cost: uint(raw, "cost")?,
        limit: uint(raw, "use_limit")?,
        effects,
        groups,
    })
}
