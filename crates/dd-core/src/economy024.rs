//! Visible cash planning. Forecasts contain no random loot or shard-to-gold claim.
use super::Context;
use crate::{
    campaign017::{Observation, ProvisionAction, Provisions, QuestView},
    town016::{Action, VisibleHero},
};
use dd_data::{combat::Result, inventory015::Item};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub poverty_gold: u32,
    pub food_floor: u32,
    pub qualified_merchant_overrides_cheap: bool,
    pub protected_items: BTreeSet<String>,
}
/// A deterministic shop quote: current prices/stock and automatic visible gifts.
/// Contains no map, enemies, random loot, host RNG or future event state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Quote {
    pub quest: String,
    pub party: Vec<u64>,
    pub items: BTreeMap<String, (u32, u32, u32)>,
}
impl Quote {
    pub fn from_visible(p: &Provisions) -> Self {
        Self {
            quest: p.quest.clone(),
            party: p.party.clone(),
            items: p
                .store
                .iter()
                .filter_map(|(item, stock, price)| {
                    let gold = p
                        .currencies019
                        .as_ref()
                        .and_then(|v| v.iter().find(|(i, _)| i == item))
                        .is_none_or(|(_, c)| c == "gold:");
                    gold.then_some((
                        if item.kind == "provision" {
                            "food".into()
                        } else {
                            item.id.clone()
                        },
                        (*stock, *price, p.inventory.count(item)),
                    ))
                })
                .collect(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Budget {
    pub returns: u32,
    pub quest: String,
    pub party: Vec<u64>,
    pub gold: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Memory {
    #[serde(default)]
    pub merchant_funnel: BTreeMap<u32, super::funnel::Visit>,
    #[serde(default)]
    pub recovery: super::recovery::Counts,
    #[serde(default)]
    pub selected_stage: Option<usize>,
    #[serde(default)]
    pub budgets: Vec<Budget>,
    #[serde(default)]
    pub sales: u64,
    #[serde(default)]
    pub sale_gold: u64,
    pub last_return: Option<u32>,
    pub rejected_farms: BTreeSet<String>,
    pub unloaded_return: Option<u32>,
}
pub fn budget(ctx: &Context, q: &QuestView, quote: &Quote) -> u32 {
    let targets = if ctx.on("D") {
        ctx.profile.provisions.table(q).map(|row| {
            row.iter()
                .map(|(id, n)| (id.clone(), n[1]))
                .collect::<BTreeMap<_, _>>()
        })
    } else {
        None
    }
    .unwrap_or_else(|| {
        let long = q.length != 1;
        BTreeMap::from([
            (
                "food".into(),
                if long {
                    if q.length == 2 { 24 } else { 48 }
                } else {
                    12
                },
            ),
            (
                "torch".into(),
                if q.dungeon == "courtyard" {
                    4
                } else if long {
                    16
                } else {
                    8
                },
            ),
            ("shovel".into(), if long { 4 } else { 2 }),
            ("bandage".into(), if long { 3 } else { 1 }),
            ("medicinal_herbs".into(), if long { 3 } else { 1 }),
            ("skeleton_key".into(), 2),
            ("antivenom".into(), if q.dungeon == "weald" { 2 } else { 0 }),
        ])
    });
    let cost = targets
        .into_iter()
        .map(|(id, n)| {
            quote.items.get(&id).map_or(0, |(stock, price, gifts)| {
                n.saturating_sub(*gifts).min(*stock).saturating_mul(*price)
            })
        })
        .sum::<u32>();
    cost.saturating_add(if q.dungeon == "farm" {
        ctx.profile.economy.poverty_gold
    } else {
        0
    })
}
pub fn plan(ctx: &Context, o: &Observation, q: &QuestView, m: &mut Memory) -> Option<u32> {
    let quote = ctx
        .catalog
        .cart_quote
        .as_ref()
        .filter(|v| v.quest == q.id && v.party == o.town.party)?;
    let gold = budget(ctx, q, quote);
    let row = Budget {
        returns: o.returns,
        quest: q.id.clone(),
        party: quote.party.clone(),
        gold,
    };
    if m.budgets.last() != Some(&row) {
        m.budgets.push(row);
    }
    Some(gold)
}
impl Memory {
    pub fn visit(&mut self, returns: u32) {
        if self.last_return != Some(returns) {
            self.last_return = Some(returns);
            self.rejected_farms.clear();
            self.selected_stage = None;
        }
    }
}
pub fn poor(ctx: &Context, o: &Observation) -> bool {
    ctx.on("H")
        && o.town
            .wallet
            .get("gold:")
            .copied()
            .unwrap_or(0)
            .saturating_add(o.provisions.as_ref().map_or(0, |p| p.paid))
            < ctx.profile.economy.poverty_gold
}
pub fn ordinary(q: &QuestView) -> bool {
    q.plot.is_none() && ["crypts", "weald", "warrens", "cove"].contains(&q.dungeon.as_str())
}
/// Before any payment, price the old Farm cart from the *visible* source shop.
/// Quest completion is conditional, so its gold and random loot have a zero floor.
pub fn farm_cost(q: &QuestView, p: &Provisions) -> u32 {
    let long = q.length != 1;
    let targets: [(Item, u32); 6] = [
        (
            Item::new("provision", ""),
            if long {
                if q.length == 2 { 24 } else { 48 }
            } else {
                12
            },
        ),
        (Item::new("supply", "torch"), if long { 16 } else { 8 }),
        (Item::new("supply", "shovel"), if long { 4 } else { 2 }),
        (Item::new("supply", "bandage"), if long { 3 } else { 1 }),
        (
            Item::new("supply", "medicinal_herbs"),
            if long { 3 } else { 1 },
        ),
        (Item::new("supply", "skeleton_key"), 2),
    ];
    targets
        .into_iter()
        .map(|(item, target)| {
            let gold = p
                .currencies019
                .as_ref()
                .and_then(|v| v.iter().find(|(i, _)| i == &item))
                .is_none_or(|(_, c)| c == "gold:");
            if !gold {
                return 0;
            }
            p.store
                .iter()
                .find(|(i, _, _)| i == &item)
                .map_or(0, |(_, stock, price)| {
                    target
                        .saturating_sub(p.inventory.count(&item))
                        .min(*stock)
                        .saturating_mul(*price)
                })
        })
        .sum()
}
pub fn affordable_farm(ctx: &Context, o: &Observation, q: &QuestView, p: &Provisions) -> bool {
    o.town
        .wallet
        .get("gold:")
        .copied()
        .unwrap_or(0)
        .saturating_sub(farm_cost(q, p))
        >= ctx.profile.economy.poverty_gold
}
/// Preserve only worn pieces, the named keeps and this trip's planned equipment.
pub fn sell(ctx: &Context, o: &Observation) -> Option<Action> {
    let town = &o.town;
    let keep = super::asset_plan::planned(ctx, town);
    let mut ranked = town
        .trinkets
        .iter()
        .filter_map(|(instance, id, owner)| {
            let card = ctx.catalog.assets.get(id)?;
            let score = town
                .heroes
                .iter()
                .filter(|h| h.alive && town.party.contains(&h.guid))
                .map(|h| super::assets::score(ctx, h, id, card))
                .fold(0f64, f64::max);
            Some((*instance, id, owner, score, card))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| a.3.total_cmp(&b.3).then_with(|| a.0.cmp(&b.0)));
    ranked.into_iter().find_map(|(instance, id, owner, _, _)| {
        let a = Action::Sell { instance };
        (owner.is_none()
            && !keep.contains(&instance)
            && !ctx.profile.economy.protected_items.contains(id)
            && town.actions.iter().any(|c| c.action == a))
        .then_some(a)
    })
}
pub fn cheap_party(ctx: &Context, o: &Observation, q: &QuestView, otherwise: Vec<u64>) -> Vec<u64> {
    if !poor(ctx, o) || !ordinary(q) || q.length != 1 || q.difficulty != 1 {
        return otherwise;
    }
    if ctx.on("G")
        && ctx.profile.economy.qualified_merchant_overrides_cheap
        && super::merchant::eligible(ctx, &o.town, q, &otherwise)
    {
        return otherwise;
    }
    let mut heroes = o
        .town
        .heroes
        .iter()
        .filter(|h| {
            h.alive
                && !h.busy
                && !h.missing
                && h.stress < 70
                && q.maximum_resolve_level.is_none_or(|max| h.level <= max)
                && (!ctx.on("C") || super::gates::hero_ready(ctx, h, q))
        })
        .collect::<Vec<&VisibleHero>>();
    let eligible = ctx.on("C").then(|| {
        let mut t = o.town.clone();
        t.heroes = heroes.iter().map(|h| (*h).clone()).collect();
        t
    });
    heroes.sort_by(|a, b| {
        ctx.power(a)
            .total_cmp(&ctx.power(b))
            .then_with(|| a.guid.cmp(&b.guid))
    });
    let mut chosen = vec![];
    for h in heroes {
        if chosen.iter().any(|id| {
            o.town
                .heroes
                .iter()
                .find(|old| old.guid == *id)
                .is_some_and(|old| {
                    old.party_tags
                        .iter()
                        .any(|t| h.incompatible_party_tags.contains(t))
                        || h.party_tags
                            .iter()
                            .any(|t| old.incompatible_party_tags.contains(t))
                })
        }) {
            continue;
        }
        chosen.push(h.guid);
        if chosen.len() == 4 {
            let chosen = ctx.arrange(&o.town, chosen);
            return if ctx.on("C") {
                super::gates::complete(ctx, eligible.as_ref().unwrap(), q, o.week, chosen)
            } else {
                chosen
            };
        }
    }
    otherwise
}
pub fn provision(
    ctx: &Context,
    o: &Observation,
    q: &QuestView,
    p: &Provisions,
    m: &mut Memory,
) -> Result<Option<ProvisionAction>> {
    if q.dungeon == "farm" && p.paid == 0 && !affordable_farm(ctx, o, q, p) {
        if !p.actions.contains(&ProvisionAction::Cancel) {
            return Err("024 H cannot legally cancel unaffordable Farm cart".into());
        }
        m.rejected_farms.insert(q.id.clone());
        return Ok(Some(ProvisionAction::Cancel));
    }
    let underfunded = m
        .budgets
        .iter()
        .rev()
        .find(|b| b.returns == o.returns && b.quest == q.id && b.party == p.party)
        .map_or_else(
            || poor(ctx, o),
            |b| {
                o.town
                    .wallet
                    .get("gold:")
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(p.paid)
                    < b.gold
            },
        );
    if underfunded {
        let food = Item::new("provision", "");
        if p.inventory.count(&food) < ctx.profile.economy.food_floor {
            let buy = ProvisionAction::Buy {
                item: food,
                amount: 1,
            };
            return Ok(Some(if p.actions.contains(&buy) {
                buy
            } else {
                ProvisionAction::Embark
            }));
        }
        // After the soft food goal, normal D/legacy buying uses any remaining
        // money. An unaffordable target never blocks the legal Embark action.
    }
    Ok(None)
}
