//! Acceptance policy. Only filtered player observations enter this module.
use crate::opening015::{Action, Observation};
use dd_data::{combat::Result, inventory015::Item};
#[path = "campaign_city_policy017.rs"]
mod city;
pub use city::city;
fn progress_item(item: &Item) -> bool {
    item.kind == "quest_item"
        || (item.kind == "estate_currency" && item.id.starts_with("crimson_court_invitation_"))
}

/// A fixed party choice from visible roster health/availability and familiar class roles.
pub fn party(town: &crate::town016::Observation) -> Vec<u64> {
    let preferences: [&[&str]; 4] = [
        &[
            "hellion",
            "leper",
            "crusader",
            "flagellant",
            "man_at_arms",
            "shieldbreaker",
            "duelist",
        ],
        &[
            "highwayman",
            "bounty_hunter",
            "houndmaster",
            "grave_robber",
            "duelist",
        ],
        &[
            "plague_doctor",
            "jester",
            "arbalest",
            "antiquarian",
            "runaway",
            "grave_robber",
        ],
        &["vestal", "occultist"],
    ];
    let mut selected = std::collections::BTreeSet::new();
    let mut party = std::collections::BTreeMap::new();
    for slot in [3, 0, 1, 2] {
        if let Some(h) = town
            .heroes
            .iter()
            .filter(|h| {
                h.alive
                    && !h.missing
                    && !h.busy
                    && !selected.contains(&h.guid)
                    && town
                        .heroes
                        .iter()
                        .filter(|other| selected.contains(&other.guid))
                        .all(|other| {
                            !h.incompatible_party_tags
                                .iter()
                                .any(|t| other.party_tags.contains(t))
                                && !other
                                    .incompatible_party_tags
                                    .iter()
                                    .any(|t| h.party_tags.contains(t))
                        })
            })
            .min_by_key(|h| {
                let role = if preferences[slot].contains(&h.class.as_str()) {
                    0
                } else {
                    100
                };
                (
                    role + h.stress
                        + if matches!(h.resolve, crate::Resolve::Afflicted(_)) {
                            100
                        } else {
                            0
                        },
                    h.guid,
                )
            })
        {
            selected.insert(h.guid);
            party.insert(slot, h.guid);
        }
    }
    party.into_values().collect()
}
/// A prepared veteran party from visible roster data; no quest/hero state is changed.
pub fn trained_party(
    town: &crate::town016::Observation,
    minimum: u32,
    maximum: Option<u32>,
) -> Vec<u64> {
    let mut visible = town.clone();
    visible.heroes.retain(|h| {
        h.level >= minimum
            && maximum.is_none_or(|limit| h.level <= limit)
            && h.stress < 60
            && h.weapon >= 1
            && h.armour >= 1
            && !matches!(h.resolve, crate::Resolve::Afflicted(_))
    });
    party(&visible)
}
pub fn choose(o: &Observation) -> Result<Action> {
    if let Some(b) = &o.battle {
        for a in &b.actions {
            if matches!(a.action, crate::Action::Item { .. })
                && a.targets.iter().any(|id| {
                    b.units.iter().any(|u| {
                        u.id == *id
                            && match a.label.as_str() {
                                "antivenom" => !u.blight.is_empty(),
                                "bandage" => !u.bleed.is_empty(),
                                "laudanum" => !u.horror.is_empty(),
                                _ => false,
                            }
                    })
                })
            {
                return Ok(Action::Fight(a.action.clone()));
            }
        }
        if b.round <= 2
            && let Some(a) = b.actions.iter().find(|a| {
                a.label == "blinding_gas"
                    && a.targets.iter().any(|id| {
                        b.units.iter().any(|u| {
                            u.id == *id
                                && (u.class.starts_with("skeleton_courtier_")
                                    || u.class.starts_with("madman_")
                                    || u.class.starts_with("cultist_witch_"))
                        })
                    })
                    && a.targets
                        .iter()
                        .filter(|id| {
                            b.units
                                .iter()
                                .any(|u| u.id == **id && !u.hero && !u.corpse && !u.stunned)
                        })
                        .count()
                        >= 2
            })
        {
            return Ok(Action::Fight(a.action.clone()));
        }
        // These class skills express healing/cleansing through source effects,
        // rather than the direct-heal preview. Decide from visible injuries.
        let actor = b.units.iter().find(|u| Some(u.id) == b.actor);
        if let Some(actor) = actor {
            if actor.class == "man_at_arms" {
                if let Some(a) = b.actions.iter().find(|a| a.label == "bolster") {
                    return Ok(Action::Fight(a.action.clone()));
                }
                if actor.guarding.is_none_or(|(_, turns)| turns <= 1)
                    && let Some(a) = b.actions.iter().find(|a| {
                        a.label == "defender"
                            && a.targets.iter().any(|id| {
                                b.units.iter().any(|u| {
                                    u.id == *id && u.class == "vestal" && u.guarded_by.is_none()
                                })
                            })
                    })
                {
                    return Ok(Action::Fight(a.action.clone()));
                }
                if actor.riposte.is_none_or(|n| n <= 1)
                    && let Some(a) = b.actions.iter().find(|a| a.label == "retribution")
                {
                    return Ok(Action::Fight(a.action.clone()));
                }
            }
            if actor.class == "runaway"
                && actor.hp * 2 < actor.max_hp
                && actor.regeneration.is_empty()
                && let Some(a) = b.actions.iter().find(|a| a.label == "run_and_hide")
            {
                return Ok(Action::Fight(a.action.clone()));
            }
            if actor.class == "antiquarian" {
                if let Some(a) = b.actions.iter().find(|a| {
                    a.label == "fortifying_vapours"
                        && a.targets
                            .iter()
                            .any(|id| b.units.iter().any(|u| u.id == *id && u.hp == 0))
                }) {
                    return Ok(Action::Fight(a.action.clone()));
                }
                if let Some(a) = b.actions.iter().find(|a| a.label == "invigorating_vapours") {
                    return Ok(Action::Fight(a.action.clone()));
                }
                if b.units.iter().filter(|u| u.hero && !u.corpse).count() > 2
                    && let Some(a) = b.actions.iter().find(|a| a.label == "cower")
                {
                    return Ok(Action::Fight(a.action.clone()));
                }
            }
            if actor.class == "highwayman"
                && actor.riposte.is_none_or(|n| n <= 1)
                && let Some(a) = b.actions.iter().find(|a| a.label == "duelist_advance")
            {
                return Ok(Action::Fight(a.action.clone()));
            }
            if actor.class == "hellion"
                && (actor.hp * 4 < actor.max_hp * 3
                    || !actor.blight.is_empty()
                    || !actor.bleed.is_empty())
                && let Some(a) = b.actions.iter().find(|a| a.label == "adrenaline_rush")
            {
                return Ok(Action::Fight(a.action.clone()));
            }
            if actor.class == "flagellant" {
                if let Some(a) = b.actions.iter().find(|a| {
                    a.label == "redeem"
                        && a.targets
                            .iter()
                            .any(|id| b.units.iter().any(|u| u.id == *id && u.hp * 2 < u.max_hp))
                }) {
                    return Ok(Action::Fight(a.action.clone()));
                }
                if let Some(a) = b.actions.iter().find(|a| a.label == "exsanguinate") {
                    return Ok(Action::Fight(a.action.clone()));
                }
            }
            if actor.class == "plague_doctor"
                && let Some(a) = b.actions.iter().find(|a| {
                    a.label == "battlefield_medicine"
                        && a.targets.iter().any(|id| {
                            b.units.iter().any(|u| {
                                u.id == *id
                                    && (u.hp == 0
                                        || u.bleed
                                            .iter()
                                            .chain(&u.blight)
                                            .map(|(n, t)| n * *t as i32)
                                            .sum::<i32>()
                                            >= 8)
                            })
                        })
                })
            {
                return Ok(Action::Fight(a.action.clone()));
            }
        }
        let score = |a: &crate::LegalAction| -> f64 {
            match a.action {
                crate::Action::Pass => -1000.,
                crate::Action::Retreat => -2000.,
                crate::Action::Move { .. } => -100.,
                crate::Action::Item { .. } => -1500.,
                _ => a
                    .targets
                    .iter()
                    .map(|id| {
                        let u = b.units.iter().find(|u| u.id == *id).unwrap();
                        if a.heal[1] > 0 {
                            let need = u.max_hp - u.hp;
                            f64::from(need.min(a.heal[1]))
                                * if u.hp == 0 {
                                    100.
                                } else if u.hp * 2 < u.max_hp {
                                    3.
                                } else if u.hp * 4 < u.max_hp * 3 {
                                    2.
                                } else {
                                    0.25
                                }
                        } else if a.stress_heal > 0 {
                            f64::from(u.stress.min(a.stress_heal))
                                * if u.stress >= 80 { 1. } else { 0.1 }
                        } else if u.hero {
                            0.
                        } else if u.corpse {
                            1.
                        } else {
                            let expected = f64::from(a.damage[0] + a.damage[1]) / 2. * a.hit_chance;
                            expected
                                + if [
                                    "skeleton_courtier_A",
                                    "skeleton_courtier_B",
                                    "skeleton_courtier_C",
                                    "madman_A",
                                    "madman_B",
                                    "madman_C",
                                    "cultist_witch_A",
                                    "cultist_witch_B",
                                    "cultist_witch_C",
                                ]
                                .contains(&u.class.as_str())
                                {
                                    12. * a.hit_chance
                                } else {
                                    0.
                                }
                                + if a.label == "if_it_bleeds" || a.label == "firefly" {
                                    10. * a.hit_chance
                                } else {
                                    0.
                                }
                                + if a.damage[0] >= u.hp {
                                    20. * a.hit_chance
                                } else {
                                    0.
                                }
                                + if u.rank >= 3 { 3. } else { 0. }
                                + 4. / f64::from(u.hp.max(1))
                        }
                    })
                    .sum(),
            }
        };
        let best = b
            .actions
            .iter()
            .max_by(|a, b| score(a).total_cmp(&score(b)))
            .ok_or("no visible battle action")?;
        if score(best) <= 0.
            && let Some(actor) = b.units.iter().find(|u| Some(u.id) == b.actor)
        {
            let preferred = match actor.class.as_str() {
                "crusader" | "hellion" | "leper" | "flagellant" | "man_at_arms" => 1,
                "plague_doctor" | "vestal" | "arbalest" | "occultist" => 4,
                _ => 2,
            };
            let preferred = preferred.min(
                b.units
                    .iter()
                    .filter(|u| u.hero && !u.corpse)
                    .map(|u| u.rank + u.size - 1)
                    .max()
                    .unwrap_or(preferred),
            );
            if actor.rank != preferred
                && let Some(action) = b
                    .actions
                    .iter()
                    .filter(|a| matches!(a.action, crate::Action::Move { .. }))
                    .min_by_key(|a| {
                        if let crate::Action::Move { rank } = a.action {
                            rank.abs_diff(preferred)
                        } else {
                            u8::MAX
                        }
                    })
            {
                return Ok(Action::Fight(action.action.clone()));
            }
        }
        return Ok(Action::Fight(best.action.clone()));
    }
    if !o.loot.is_empty() {
        if let Some((index, stack)) = o
            .loot
            .iter()
            .enumerate()
            .find(|(_, s)| progress_item(&s.item))
        {
            if let Some(a) = o
                .actions
                .iter()
                .find(|a| matches!(a,Action::Take{index:i,..}if *i==index))
            {
                return Ok(a.clone());
            }
            if let Some((slot, s)) = o.inventory.slots.iter().enumerate().find_map(|(i, s)| {
                s.as_ref()
                    .filter(|s| !progress_item(&s.item))
                    .map(|s| (i, s))
            }) {
                return Ok(Action::Discard {
                    slot,
                    amount: s.amount,
                });
            }
            return Err(format!(
                "finite inventory cannot retain objective {}",
                stack.item.key()
            ));
        }
        return crate::opening_policy015::choose(o);
    }
    if let Some(camp) = &o.camping {
        use crate::expedition017::CampAction;
        use dd_data::camp017::Operation;
        if camp.phase == "meal" {
            for name in ["feast", "full", "half", "none"] {
                let action = Action::Camp(CampAction::Meal(name.into()));
                if o.actions.contains(&action) {
                    return Ok(action);
                }
            }
        }
        let mut best = None;
        for action in &o.actions {
            let Action::Camp(CampAction::Skill {
                hero,
                skill,
                target,
            }) = action
            else {
                continue;
            };
            let Some(source) = camp
                .skills
                .iter()
                .find(|s| s.hero == *hero && s.id == *skill)
            else {
                continue;
            };
            let mut score = 0.;
            for effect in &source.effects {
                let recipients = o.heroes.iter().filter(|h| {
                    h.alive
                        && o.party.contains(&h.guid)
                        && match effect.selection.as_str() {
                            "self" => h.guid == *hero,
                            "individual" => Some(h.guid) == *target,
                            "party_other" => h.guid != *hero && Some(h.guid) != *target,
                            _ => true,
                        }
                });
                for h in recipients {
                    score += effect.chance
                        * match &effect.operation {
                            Operation::Heal(p) => {
                                f64::from(h.maximum_hp - h.hp).min(f64::from(h.maximum_hp) * p)
                            }
                            Operation::Stress(n) if *n < 0 => f64::from(h.stress.min(-n)) * 0.8,
                            Operation::PreventAmbush(p)
                                if !camp.nightmare_warning && camp.ambush_prevention < 1. =>
                            {
                                p * 12.
                            }
                            Operation::Buff(b) if b.amount > 0. => 1.,
                            _ => 0.,
                        };
                }
            }
            score /= f64::from(source.cost.max(1));
            if score > 0. && best.as_ref().is_none_or(|(value, _)| score > *value) {
                best = Some((score, action.clone()));
            }
        }
        return Ok(best.map_or(Action::Camp(CampAction::Rest), |(_, a)| a));
    }
    if o.actions
        .contains(&Action::Camp(crate::expedition017::CampAction::Start))
        && o.heroes
            .iter()
            .any(|h| h.alive && (h.stress >= 50 || h.hp * 2 < h.maximum_hp))
    {
        return Ok(Action::Camp(crate::expedition017::CampAction::Start));
    }
    if o.battle.is_none() {
        if let Some(a) = o
            .actions
            .iter()
            .find(|a| matches!(a, Action::Leave { complete: true }))
        {
            return Ok(a.clone());
        }
        if o.inventory.count(&Item::new("provision", "")) > 12 {
            for h in o
                .heroes
                .iter()
                .filter(|h| h.alive && h.hp * 4 < h.maximum_hp && h.food_remaining > 0)
            {
                if let Some(a)=o.actions.iter().find(|a|matches!(a,Action::Use{hero,slot}if *hero==h.guid && o.inventory.slots[*slot].as_ref().is_some_and(|s|s.item.kind=="provision"))){return Ok(a.clone());}
            }
        }
        if let Some(a) = o.actions.iter().find(|a| matches!(a, Action::Unlock(_))) {
            return Ok(a.clone());
        }
    }
    // Optional unknown curios are left alone. Quest markers and well-known safe
    // containers use the same public interaction controls as a player.
    let mut filtered = o.clone();
    if let Some(m) = &o.map {
        let current = m
            .nodes
            .iter()
            .find(|n| n.id == m.position)
            .and_then(|n| n.content.as_ref());
        if current.is_some_and(|c| {
            !c.objective
                && ![
                    "unlocked_strongbox",
                    "locked_strongbox",
                    "sack",
                    "discarded_pack",
                    "heirloom_chest",
                    "cc_wine_crate",
                    "cc_firewood_crate",
                    "prisoner",
                    "redkey_heirloom_chest",
                    "yellowkey_heirloom_chest",
                    "greenkey_heirloom_chest",
                    "bluekey_heirloom_chest",
                    "trinket_chest",
                    "cc_treasure_chest",
                    "cc_infested_egg_cluster",
                    "cc_food_cart",
                ]
                .contains(&c.curio_name.as_deref().unwrap_or(""))
        }) {
            filtered
                .actions
                .retain(|a| !matches!(a, Action::Interact { .. }));
        }
    }
    if let Some(map) = &mut filtered.map {
        let blocked = map
            .nodes
            .iter()
            .filter(|n| {
                !n.opened
                    && n.gate_key
                        .as_ref()
                        .is_some_and(|key| map.keys.get(key).copied().unwrap_or(0) == 0)
            })
            .map(|n| n.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for node in &mut map.nodes {
            node.exits.retain(|id| !blocked.contains(id));
        }
        let name = map
            .nodes
            .iter()
            .find(|n| n.id == map.position)
            .and_then(|n| n.content.as_ref())
            .and_then(|c| c.curio_name.as_deref());
        let item = match name {
            Some("cc_wine_crate") => Some("shovel"),
            Some("cc_food_cart") => Some("medicinal_herbs"),
            _ => None,
        };
        if let Some(item) = item
            && let Some(action) = o
                .actions
                .iter()
                .find(|a| matches!(a,Action::Interact{item:i,..} if i.as_deref()==Some(item)))
        {
            return Ok(action.clone());
        }
    }
    crate::opening_policy015::choose(&filtered)
}
