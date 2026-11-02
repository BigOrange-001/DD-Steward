//! Baseline town decisions. Budgets and role preferences are policy choices;
//! source costs, eligibility and all changes remain owned by the town API.
use super::Context;
use crate::town016::{Action, Observation, Treatment, VisibleHero};
fn selected(h: &VisibleHero, ctx: &Context) -> Vec<String> {
    ctx.selected(h)
}
fn affordable(
    o: &Observation,
    cost: &std::collections::BTreeMap<String, u32>,
    reserve: u32,
) -> bool {
    cost.iter().all(|(key, n)| {
        *n <= o
            .wallet
            .get(key)
            .copied()
            .unwrap_or(0)
            .saturating_sub(if key == "gold:" { reserve } else { 0 })
    })
}
fn building(o: &Observation, reserve: u32) -> Option<Action> {
    let level = o
        .heroes
        .iter()
        .filter(|h| o.party.contains(&h.guid))
        .map(|h| h.level)
        .max()
        .unwrap_or(0);
    let mut quotes = o
        .upgrade_quotes
        .iter()
        .filter(|q| {
            let Action::Upgrade {
                hero: None, tree, ..
            } = &q.action
            else {
                return false;
            };
            let bought = o
                .purchased
                .iter()
                .filter(|(owner, id, _)| owner.is_none() && id == tree)
                .count();
            [
                "blacksmith.weapon",
                "blacksmith.armour",
                "guild.skill_levels",
            ]
            .contains(&tree.as_str())
                && bought < level.min(4) as usize
        })
        .collect::<Vec<_>>();
    quotes.sort_by_key(|q| {
        let Action::Upgrade { tree, .. } = &q.action else {
            unreachable!()
        };
        (
            o.purchased
                .iter()
                .filter(|(owner, id, _)| owner.is_none() && id == tree)
                .count(),
            tree.clone(),
        )
    });
    for q in quotes {
        if affordable(o, &q.cost, reserve) && o.actions.iter().any(|c| c.action == q.action) {
            return Some(q.action.clone());
        }
        if q.cost.get("gold:").copied().unwrap_or(0)
            > o.wallet
                .get("gold:")
                .copied()
                .unwrap_or(0)
                .saturating_sub(reserve)
        {
            continue;
        }
        for (currency, needed) in &q.cost {
            if currency == "gold:" || o.wallet.get(currency).copied().unwrap_or(0) >= *needed {
                continue;
            }
            let exchange = o
                .exchanges
                .iter()
                .filter(|x| {
                    let surplus = o
                        .wallet
                        .get(&x.from)
                        .copied()
                        .unwrap_or(0)
                        .saturating_sub(q.cost.get(&x.from).copied().unwrap_or(0));
                    x.to == *currency
                        && x.spend > 0
                        && surplus >= x.spend
                        && o.actions.iter().any(|c| {
                            c.action
                                == Action::Exchange {
                                    index: x.index,
                                    batches: 1,
                                }
                        })
                })
                .max_by_key(|x| {
                    (u128::from(
                        o.wallet
                            .get(&x.from)
                            .copied()
                            .unwrap_or(0)
                            .saturating_sub(q.cost.get(&x.from).copied().unwrap_or(0)),
                    ) * u128::from(x.gain))
                        / u128::from(x.spend)
                });
            if let Some(x) = exchange {
                return Some(Action::Exchange {
                    index: x.index,
                    batches: 1,
                });
            }
        }
    }
    None
}
/// Return one idempotent next decision, or None when ready to select a quest.
pub fn city(o: &Observation, ctx: &Context, party_locked: bool) -> Option<Action> {
    if o.expedition_active {
        return None;
    }
    let proposed = ctx.town_party(o);
    let established = proposed.len() == 4
        && o.heroes
            .iter()
            .filter(|h| proposed.contains(&h.guid))
            .all(|h| {
                h.level >= 1
                    && h.weapon >= 1
                    && h.armour >= 1
                    && h.stress < 60
                    && !matches!(h.resolve, crate::Resolve::Afflicted(_))
            });
    let reserve = if established { 8000 } else { 4500 };
    let alive = o.heroes.iter().filter(|h| h.alive).count();
    if alive < 8 {
        let wanted = o.recruits.iter().filter(|h| h.alive).min_by_key(|h| {
            let count = o
                .heroes
                .iter()
                .filter(|old| old.alive && old.class == h.class)
                .count();
            (
                count,
                if ["vestal", "occultist"].contains(&h.class.as_str()) {
                    0
                } else {
                    1
                },
                h.guid,
            )
        });
        if let Some(h) = wanted
            && let Some(c) = o.actions.iter().find(|c| {
                matches!(c.action,Action::Hire{guid} if guid==h.guid)
                    && affordable(o, &c.cost, reserve)
            })
        {
            return Some(c.action.clone());
        }
    }
    // Rest an available high-stress hero before selecting the next party, while
    // keeping four available people and an expedition budget. Busy heroes are
    // excluded by the ordinary source eligibility checks on the next call.
    let available = o
        .heroes
        .iter()
        .filter(|h| h.alive && !h.busy && !h.missing)
        .count();
    if available > 4 {
        let mut stressed = o
            .heroes
            .iter()
            .filter(|h| h.alive && !h.busy && !h.missing && h.stress >= 60)
            .collect::<Vec<_>>();
        stressed.sort_by_key(|h| std::cmp::Reverse(h.stress));
        for h in stressed {
            let healer = |class: &str| ["vestal", "occultist"].contains(&class);
            if healer(&h.class)
                && o.heroes
                    .iter()
                    .filter(|other| {
                        other.alive && !other.busy && !other.missing && healer(&other.class)
                    })
                    .count()
                    <= 1
            {
                continue;
            }
            if let Some(c)=o.actions.iter().find(|c|matches!(c.action,Action::Queue{hero,treatment:Treatment::Stress,..} if hero==h.guid)&&affordable(o,&c.cost,3000)){return Some(c.action.clone());}
        }
    }
    let party = if party_locked {
        o.party.clone()
    } else {
        ctx.town_party(o)
    };
    if !party.is_empty() && party != o.party {
        return Some(Action::Party(party));
    }
    for h in o.heroes.iter().filter(|h| o.party.contains(&h.guid)) {
        let skills = selected(h, ctx);
        if skills != h.selected {
            return Some(Action::SelectSkills {
                hero: h.guid,
                skills,
            });
        }
    }
    if ctx.on("A2")
        && let Some(action) = ctx.learn_missing(o)
    {
        return Some(action);
    }
    // First acquire essential missing heals and a position-tolerant attack.
    for (class, skill) in [
        ("vestal", "divine_grace"),
        ("vestal", "gods_comfort"),
        ("occultist", "wyrd_reconstruction"),
        ("plague_doctor", "battlefield_medicine"),
        ("highwayman", "duelist_advance"),
    ] {
        if let Some(c) = o.actions.iter().find(|c| {
            let Action::Upgrade {
                hero: Some(id),
                tree,
                ..
            } = &c.action
            else {
                return false;
            };
            o.heroes.iter().any(|h| {
                h.guid == *id
                    && o.party.contains(id)
                    && h.class == class
                    && !h.skills.contains_key(skill)
            }) && tree == &format!("{class}.{skill}")
                && affordable(o, &c.cost, 4500)
        }) {
            return Some(c.action.clone());
        }
    }
    if ctx.on("F")
        && let Some(a) = super::super::estate::building(ctx, o)
    {
        return Some(a);
    }
    if !ctx.on("F")
        && let Some(a) = building(o, reserve)
    {
        return Some(a);
    }
    for kind in ["weapon", "armour", "skill"] {
        if let Some(c) = o.actions.iter().find(|c| {
            let Action::Upgrade {
                hero: Some(id),
                tree,
                ..
            } = &c.action
            else {
                return false;
            };
            let Some(h) = o
                .heroes
                .iter()
                .find(|h| h.guid == *id && o.party.contains(id))
            else {
                return false;
            };
            let name = tree.strip_prefix(&format!("{}.", h.class)).unwrap_or("");
            if kind == "skill" {
                selected(h, ctx).iter().any(|id| id == name)
                    && affordable(o, &c.cost, reserve.max(7000))
            } else {
                name == kind && affordable(o, &c.cost, reserve)
            }
        }) {
            return Some(c.action.clone());
        }
    }
    // E owns reassignment after quest and party selection.
    if ctx.on("E") {
        return None;
    }
    // Source-validated equipment, with no repeated swap cycle.
    for h in o.heroes.iter().filter(|h| o.party.contains(&h.guid)) {
        for slot in 0..2 {
            if h.equipped.iter().any(|(s, _, _)| *s == slot) {
                continue;
            }
            if let Some(c) = o.actions.iter().find(|c| {
                let Action::Equip {
                    hero,
                    instance,
                    slot: s,
                } = c.action
                else {
                    return false;
                };
                hero == h.guid
                    && s == slot
                    && o.trinkets
                        .iter()
                        .any(|(id, _, owner)| *id == instance && owner.is_none())
            }) {
                return Some(c.action.clone());
            }
        }
    }
    None
}
