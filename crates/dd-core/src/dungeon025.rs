//! Visible exploration only. Legal action lists own every state transition.
use super::*;
use crate::opening015::{Action as A, Observation as Raid};
use dd_data::{
    camp017::Operation,
    curios013::Outcome,
    field017::ResultKind,
    inventory015::{Item, Stack},
};
fn n(ctx: &Context<'_>, k: &str) -> f64 {
    ctx.profile.parameters["dungeon"][k].as_f64().unwrap_or(0.)
}
fn emit(
    o: &Observation,
    a: A,
    id: &str,
    stage: &str,
    ctx: &Context<'_>,
) -> Result<Option<Proposal>> {
    if !o.expedition.as_ref().unwrap().actions.contains(&a) {
        return Err(format!(
            "025 dungeon proposed action absent from visible legal controls: {a:?}"
        ));
    }
    proposal(o, Action::Raid(a), id, stage, ctx).map(Some)
}
/// New navigation must not starve the old field actions. Preserve the selected
/// expedition formation; an intentional altar approach keeps its special order.
pub(super) fn move_with_maintenance(
    o: &Observation,
    m: &Memory,
    a: A,
    id: &str,
    stage: &str,
    ctx: &Context<'_>,
    restore_order: bool,
) -> Result<Option<Proposal>> {
    if ctx.on("exploration") && matches!(a, A::Move(_)) {
        let r = o.expedition.as_ref().unwrap();
        if restore_order {
            let target = m
                .legacy
                .target_party
                .iter()
                .copied()
                .filter(|id| {
                    r.party.contains(id) && r.heroes.iter().any(|h| h.guid == *id && h.alive)
                })
                .collect::<Vec<_>>();
            if target.len() == r.party.len() && !target.is_empty() && target != r.party {
                let order = A::Reorder(target);
                if r.actions.contains(&order) {
                    return emit(o, order, "S025-27", "restore_departing_order", ctx);
                }
            } else if m.legacy.target_party.is_empty()
                && let Ok(order @ A::Reorder(_)) = auto_policy018::raid_action(r)
            {
                return emit(o, order, "S025-27", "legacy_field_order", ctx);
            }
        }
        let mut items = r.clone();
        items.actions.retain(|a|matches!(a,A::Use{slot,..} if restore_order || !r.inventory.slots[*slot].as_ref().is_some_and(|s|s.item.id=="torch")));
        if let Ok(item @ A::Use { .. }) = crate::campaign_policy017::choose(&items) {
            return emit(o, item, "S025-27", "legacy_field_item", ctx);
        }
        if r.map
            .as_ref()
            .and_then(|m| m.nodes.iter().find(|n| n.id == m.position))
            .and_then(|n| n.content.as_ref())
            .is_some_and(|c| c.curio && c.objective)
        {
            let mut objective = r.clone();
            objective
                .actions
                .retain(|a| matches!(a, A::Interact { .. }));
            if let Ok(action @ A::Interact { .. }) = crate::campaign_policy017::choose(&objective) {
                return emit(o, action, "S025-27", "legacy_objective_interaction", ctx);
            }
        }
    }
    emit(o, a, id, stage, ctx)
}
fn ready(o: &Raid) -> Vec<&crate::opening015::VisibleHero> {
    o.heroes
        .iter()
        .filter(|h| h.alive && o.party.contains(&h.guid))
        .collect()
}
fn goal_item(i: &Item) -> bool {
    i.kind == "quest_item"
        || i.kind == "firewood"
        || i.kind == "estate_currency" && i.id.contains("invitation")
        || i.id == "the_blood"
}
fn value(s: &Stack, o: &Raid, ctx: &Context<'_>) -> f64 {
    if goal_item(&s.item) {
        return f64::INFINITY;
    } // handled below explicitly by infinite preservation
    let values = &ctx.profile.parameters["dungeon"]["bag_values"];
    let mut v = values[&s.item.kind]
        .as_f64()
        .or_else(|| values["default"].as_f64())
        .unwrap();
    if s.item.kind == "supply" {
        let mut needed = false;
        if let Some(map) = &o.map {
            for node in &map.nodes {
                if let Some(name) = node.content.as_ref().and_then(|c| c.curio_name.as_ref()) {
                    needed |= ctx
                        .sources
                        .curios
                        .get(name)
                        .is_some_and(|c| c.items.contains_key(&s.item.id));
                }
                if node.content.as_ref().is_some_and(|c| c.obstacle) && s.item.id == "shovel" {
                    needed = true
                }
            }
        }
        if needed {
            v *= 2.
        } else if o.quest_ready {
            v *= 0.25
        }
    }
    if s.item.kind == "provision" {
        let unvisited = o
            .map
            .as_ref()
            .map_or(0, |m| m.nodes.iter().filter(|n| !n.visited).count());
        if unvisited > 0 {
            v *= 2.
        }
    }
    v * f64::from(s.amount)
}
fn loot(o: &Raid, ctx: &Context<'_>) -> Option<A> {
    let valid = |a: &A| o.actions.contains(a);
    for (index, s) in o.loot.iter().enumerate() {
        if s.item.kind == "journal_page" {
            continue;
        }
        if let Some(a) = o
            .actions
            .iter()
            .find(|a| matches!(a,A::Take{index:i,..}if *i==index))
        {
            return Some(a.clone());
        }
        let best = o
            .inventory
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, s)| s.as_ref().map(|s| (slot, s)))
            .filter(|(_, s)| {
                !goal_item(&s.item)
                    && !(s.item.kind == "provision"
                        && o.inventory.count(&s.item)
                            <= o.party.len() as u32 * n(ctx, "food_reserve_per_hero") as u32)
            })
            .filter(|(slot, s)| {
                valid(&A::Discard {
                    slot: *slot,
                    amount: s.amount,
                })
            })
            .min_by(|a, b| {
                value(a.1, o, ctx)
                    .total_cmp(&value(b.1, o, ctx))
                    .then_with(|| a.0.cmp(&b.0))
            });
        if let Some((slot, old)) = best
            && (goal_item(&s.item) || value(s, o, ctx) > value(old, o, ctx))
        {
            return Some(A::Discard {
                slot,
                amount: old.amount,
            });
        }
    }
    valid(&A::LeaveLoot).then_some(A::LeaveLoot)
}
pub fn navigate(o: &Raid) -> Option<A> {
    use std::collections::VecDeque;
    let map = o.map.as_ref()?;
    let start = &map.position;
    let mut seen = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([(start.clone(), None::<String>)]);
    let mut targets = vec![];
    while let Some((id, first)) = queue.pop_front() {
        let Some(node) = map.nodes.iter().find(|n| n.id == id) else {
            continue;
        };
        if &id != start
            && (!node.visited
                || node
                    .content
                    .as_ref()
                    .is_some_and(|c| c.curio_name.as_deref() == Some("secret_stash") && c.curio))
            && let Some(next) = first.as_ref()
        {
            targets.push((
                !node
                    .content
                    .as_ref()
                    .is_some_and(|c| c.curio_name.as_deref() == Some("secret_stash")),
                targets.len(),
                next.clone(),
            ))
        }
        for next in &node.exits {
            if map.nodes.iter().find(|n| &n.id == next).is_some_and(|n| {
                n.gate_key
                    .as_ref()
                    .is_none_or(|k| n.opened || map.keys.get(k).copied().unwrap_or(0) > 0)
            }) && seen.insert(next.clone())
            {
                queue.push_back((next.clone(), first.clone().or(Some(next.clone()))));
            }
        }
    }
    targets.sort();
    for (_, _, next) in targets {
        let a = A::Move(next);
        if o.actions.contains(&a) {
            return Some(a);
        }
    }
    None
}
fn current(o: &Raid) -> Option<&str> {
    o.map
        .as_ref()
        .and_then(|m| m.nodes.iter().find(|n| n.id == m.position))
        .and_then(|n| n.content.as_ref())
        .and_then(|c| c.curio_name.as_deref())
}
fn camp(o: &Raid, ctx: &Context<'_>) -> Option<A> {
    use crate::expedition017::CampAction as C;
    if let Some(c) = &o.camping {
        if c.phase == "meal" {
            let healthy = ready(o)
                .iter()
                .all(|h| f64::from(h.hp) >= f64::from(h.maximum_hp) * n(ctx, "camp_healthy_hp"));
            let urgent = ctx.profile.parameters["sync026"]["field"]["enabled"] == true
                && ready(o).iter().any(|h| {
                    i64::from(h.stress)
                        >= ctx.profile.parameters["dungeon"]["legacy_exit"]["stress"]
                            .as_i64()
                            .unwrap_or(160)
                });
            for name in if healthy && !urgent {
                ["half", "full", "feast", "none"]
            } else {
                ["feast", "full", "half", "none"]
            } {
                let a = A::Camp(C::Meal(name.into()));
                if o.actions.contains(&a) {
                    return Some(a);
                }
            }
        }
        let mut best = None;
        for action in &o.actions {
            let A::Camp(C::Skill {
                hero,
                skill,
                target,
            }) = action
            else {
                continue;
            };
            let source = c
                .skills
                .iter()
                .find(|s| s.hero == *hero && s.id == *skill)?;
            let mut score = 0.;
            for e in &source.effects {
                for h in ready(o).into_iter().filter(|h| match e.selection.as_str() {
                    "self" => h.guid == *hero,
                    "individual" => Some(h.guid) == *target,
                    "party_other" => h.guid != *hero,
                    _ => true,
                }) {
                    score += e.chance
                        * match &e.operation {
                            Operation::Heal(p) => {
                                f64::from(h.maximum_hp - h.hp).min(f64::from(h.maximum_hp) * p)
                            }
                            Operation::Stress(v) if *v < 0 => f64::from(h.stress.min(-v)),
                            Operation::PreventAmbush(v)
                                if c.ambush_prevention < 1. && !c.nightmare_warning =>
                            {
                                v * n(ctx, "camp_ambush_weight")
                            }
                            Operation::Buff(b) if b.stat_type == "scouting_chance" => {
                                b.amount * n(ctx, "camp_scout_weight")
                            }
                            Operation::Buff(b)
                                if b.amount < 0.
                                    && b.stat_type == "stress_dmg_received_percent" =>
                            {
                                -b.amount
                                    * n(
                                        ctx,
                                        if c.nightmare_warning {
                                            "camp_nightmare_resist_weight"
                                        } else {
                                            "camp_stress_resist_weight"
                                        },
                                    )
                            }
                            Operation::Buff(b) if b.amount > 0. => 1.,
                            _ => 0.,
                        };
                }
            }
            score /= f64::from(source.cost.max(1));
            if score > 0. && best.as_ref().is_none_or(|(v, _)| score > *v) {
                best = Some((score, action.clone()))
            }
        }
        return best.map(|(_, a)| a).or_else(|| {
            o.actions
                .contains(&A::Camp(C::Rest))
                .then_some(A::Camp(C::Rest))
        });
    }
    let count = o.map.as_ref().map_or(0, |m| {
        m.nodes.iter().filter(|n| n.room && n.visited).count()
    });
    let shield = ready(o).iter().any(|h| h.class == "shieldbreaker");
    let crisis = ready(o).iter().any(|h| {
        h.stress >= n(ctx, "camp_stress") as i32
            || f64::from(h.hp) < f64::from(h.maximum_hp) * n(ctx, "camp_low_hp")
    });
    let start = A::Camp(C::Start);
    (o.actions.contains(&start)
        && (crisis || !shield && count >= n(ctx, "camp_after_rooms") as usize))
        .then_some(start)
}
fn food_swap(o: &Raid, m: &mut Memory, ctx: &Context<'_>) -> Option<A> {
    if !ctx.on("trinkets") {
        return None;
    }
    // One finite three-step transaction: equip a carried HP item, eat, then restore.
    if let Some(v) = m.state.get("food_swap").cloned() {
        let hero = v["hero"].as_u64()?;
        let slot = v["equipment_slot"].as_u64()? as usize;
        let mut step = v["step"].as_u64()?;
        if !o.heroes.iter().any(|h| h.guid == hero && h.alive) {
            m.state.remove("food_swap");
            return None;
        }
        if step == 0 {
            if let Some(a)=o.actions.iter().find(|a|matches!(a,A::Use{hero:h,slot:s}if *h==hero&&o.inventory.slots[*s].as_ref().is_some_and(|s|s.item.kind=="provision"))).cloned(){m.state.get_mut("food_swap").unwrap()["step"]=json!(1);return Some(a)}
            // Another legal phase may have healed the hero before the planned meal.
            step = 1;
            m.state.get_mut("food_swap").unwrap()["step"] = json!(1);
        }
        if step == 1 {
            let a = A::UnequipTrinket {
                hero,
                equipment_slot: slot,
            };
            if o.actions.contains(&a) {
                m.state.get_mut("food_swap").unwrap()["step"] = json!(2);
                return Some(a);
            }
            return None;
        }
        m.state.remove("food_swap");
    }
    for a in &o.actions {
        let A::EquipTrinket {
            hero,
            bag_slot,
            equipment_slot,
        } = a
        else {
            continue;
        };
        let Some(h) = o
            .heroes
            .iter()
            .find(|h| h.guid == *hero && h.alive && h.food_remaining > 0 && h.hp < h.maximum_hp)
        else {
            continue;
        };
        if h.equipped023
            .as_ref()
            .is_some_and(|v| v.iter().any(|e| e.slot == *equipment_slot))
        {
            continue;
        }
        let item = &o.inventory.slots[*bag_slot].as_ref()?.item;
        let Some(card) = ctx.sources.cards.assets.get(&item.id) else {
            continue;
        };
        let multiplier = card
            .buffs
            .iter()
            .filter(|b| {
                b["stat_type"] == "combat_stat_multiply"
                    && b["stat_sub_type"] == "max_hp"
                    && b["rule_type"].as_str().is_none_or(|r| r == "always")
            })
            .filter_map(|b| b["amount"].as_f64())
            .sum::<f64>();
        let boosted = (f64::from(h.maximum_hp) * (1. + multiplier)).ceil() as i32;
        if multiplier > 0.
            && (f64::from(boosted) * 0.05).ceil() > (f64::from(h.maximum_hp) * 0.05).ceil()
            && o.inventory.count(&Item::new("provision", "")) > o.party.len() as u32
        {
            m.state.insert(
                "food_swap".into(),
                json!({"hero":hero,"equipment_slot":equipment_slot,"step":0}),
            );
            return Some(a.clone());
        }
    }
    None
}
pub fn choose(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    let r = o.expedition.as_ref().ok_or("025 dungeon boundary absent")?;
    if r.battle.is_some() {
        return Ok(None);
    }
    let field026 = ctx.profile.parameters["sync026"]["field"]["enabled"] == true;
    if field026 && let Some(p) = super::sync026::field::choose(o, m, ctx)? {
        return Ok(Some(p));
    }
    let legacy = &ctx.profile.parameters["dungeon"]["legacy_exit"];
    if !field026
        && ctx.on("exploration")
        && r.camping.is_none()
        && r.loot.is_empty()
        && !r.quest_ready
        && m.legacy.moves > 0
        && r.map.as_ref().is_some_and(|map| {
            map.nodes.iter().filter(|n| n.room && n.visited).count()
                >= legacy["visited_rooms"].as_u64().unwrap() as usize
        })
        && (r.heroes.iter().any(|h| {
            h.alive
                && r.party.contains(&h.guid)
                && (h.hp == 0 || i64::from(h.stress) >= legacy["stress"].as_i64().unwrap())
        }) || r.inventory.count(&Item::new("provision", "")) < r.party.len() as u32)
        && r.actions.contains(&A::Leave { complete: false })
    {
        let count = m
            .state
            .get("legacy_safety_exits")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        m.state
            .insert("legacy_safety_exits".into(), json!(count + 1));
        return emit(
            o,
            A::Leave { complete: false },
            "S025-27",
            "legacy_safety_exit",
            ctx,
        );
    }
    let champion = o
        .board
        .as_ref()
        .and_then(|b| {
            b.quests
                .iter()
                .find(|q| Some(&q.id) == m.legacy.quest.as_ref())
        })
        .is_some_and(|q| q.difficulty >= 5);
    if ctx.on("income")
        && champion
        && r.quest_ready
        && r.loot.is_empty()
        && r.camping.is_none()
        && r.actions.contains(&A::Leave { complete: true })
    {
        return emit(
            o,
            A::Leave { complete: true },
            "S025-21",
            "champion_objective_return",
            ctx,
        );
    }
    if ctx.on("exploration")
        && !r.loot.is_empty()
        && let Some(a) = loot(r, ctx)
    {
        return emit(o, a, "S025-23", "inventory", ctx);
    }
    if ctx.on("exploration")
        && let Some(a) = camp(r, ctx)
    {
        return emit(o, a, "S025-20", "camp", ctx);
    }
    if r.camping.is_some() || !r.loot.is_empty() {
        return Ok(None);
    }
    if let Some(a) = food_swap(r, m, ctx) {
        return emit(o, a, "S025-25", "food_equipment", ctx);
    }
    let heroes = ready(r);
    let q = o.board.as_ref().and_then(|b| {
        b.quests
            .iter()
            .find(|q| Some(&q.id) == m.legacy.quest.as_ref())
    });
    let difficulty = q.map_or(1, |q| q.difficulty);
    if ctx.on("poverty")
        && !r.quest_ready
        && heroes
            .iter()
            .any(|h| h.hp == 0 || matches!(h.resolve, crate::Resolve::Afflicted(_)))
        && heroes
            .iter()
            .filter(|h| f64::from(h.hp) < f64::from(h.maximum_hp) * n(ctx, "recovery_retreat_hp"))
            .count()
            >= n(ctx, "low_health_members") as usize
        && r.actions.contains(&A::Leave { complete: false })
    {
        return emit(
            o,
            A::Leave { complete: false },
            "S025-24",
            "poverty_recovery_exit",
            ctx,
        );
    }
    if (ctx.on("income") || ctx.on("exploration"))
        && current(r) == Some("secret_stash")
        && let Some(a) = r
            .actions
            .iter()
            .find(|a| matches!(a,A::Interact{item:Some(i),..}if i=="skeleton_key"))
            .or_else(|| r.actions.iter().find(|a| matches!(a, A::Interact { .. })))
    {
        return emit(
            o,
            a.clone(),
            if ctx.on("exploration") {
                "S025-27"
            } else {
                "S025-22"
            },
            "secret_stash",
            ctx,
        );
    }
    if ctx.on("income")
        && let Some(name) = current(r)
        && let Some(card) = ctx.sources.curios.get(name)
    {
        let mut choices = r
            .actions
            .iter()
            .filter_map(|a| {
                let A::Interact { hero, item } = a else {
                    return None;
                };
                let result = item.as_ref().and_then(|i| card.items.get(i));
                let safe = matches!(
                    result,
                    Some(ResultKind::Basic(Outcome::Loot { .. }) | ResultKind::Purge(_))
                );
                let guaranteed = item.is_none()
                    && card.outcomes.iter().all(|(r, _)| {
                        matches!(
                            r,
                            ResultKind::Basic(Outcome::Loot { .. } | Outcome::Nothing)
                        )
                    });
                if !safe && !guaranteed {
                    return None;
                }
                let antiquarian = heroes
                    .iter()
                    .any(|h| h.guid == *hero && h.class == "antiquarian");
                Some((!safe, !antiquarian, *hero, a))
            })
            .collect::<Vec<_>>();
        choices.sort_by_key(|(a, b, c, _)| (*a, *b, *c));
        if let Some((_, _, _, a)) = choices.first() {
            return emit(o, (*a).clone(), "S025-22", "source_curio", ctx);
        }
    }
    if ctx.on("cohorts")
        && difficulty == 1
        && heroes.len() == 4
        && r.torch > 0
        && heroes.iter().all(|h| {
            o.town
                .heroes
                .iter()
                .any(|t| t.guid == h.guid && t.level == n(ctx, "altar_level") as u32)
        })
        && let Some(a) = altar_preparation(o, m, ctx)?
    {
        return move_with_maintenance(o, m, a, "S025-26", "prepared_optional_altar", ctx, false);
    }
    if r.quest_ready && ctx.on("income") {
        let dead = heroes.iter().any(|h| h.hp == 0);
        let afflicted = heroes
            .iter()
            .filter(|h| matches!(h.resolve, crate::Resolve::Afflicted(_)))
            .count();
        let low = heroes.iter().filter(|h| h.hp * 2 < h.maximum_hp).count();
        let known_battle = r.map.as_ref().is_some_and(|map| {
            map.nodes
                .iter()
                .any(|n| !n.visited && n.content.as_ref().is_some_and(|c| c.battle))
        });
        let full = r.inventory.slots.iter().all(Option::is_some);
        let safe = if known_battle {
            !dead
                && afflicted < n(ctx, "afflictions_stop") as usize
                && low < n(ctx, "low_health_members") as usize
        } else {
            !dead
                && afflicted == 0
                && heroes.iter().all(|h| {
                    f64::from(h.hp)
                        >= f64::from(h.maximum_hp)
                            * ctx.profile.modes[&ctx.sources.mode]["unscouted_hp_fraction"]
                                .as_f64()
                                .unwrap()
                })
        };
        if difficulty < 5
            && safe
            && !full
            && let Some(a) = navigate(r)
        {
            return move_with_maintenance(o, m, a, "S025-21", "post_objective_greed", ctx, true);
        }
        if r.actions.contains(&A::Leave { complete: true }) {
            return emit(
                o,
                A::Leave { complete: true },
                "S025-21",
                "objective_return",
                ctx,
            );
        }
    }
    if ctx.on("exploration") && r.map.as_ref().is_some_and(|m| m.pending.is_none()) {
        if let Some(a) = r.actions.iter().find(|a| matches!(a, A::Unlock(_))) {
            return emit(o, a.clone(), "S025-27", "visible_gate", ctx);
        }
        if let Some(a) = navigate(r) {
            return move_with_maintenance(o, m, a, "S025-27", "visible_frontier", ctx, true);
        }
        match auto_policy018::raid_action(r) {
            Err(e)
                if e == "visible navigation exhausted before quest ready"
                    && r.actions.contains(&A::Leave { complete: false }) =>
            {
                if let Some(q) = q {
                    m.legacy
                        .blocked
                        .insert(q.plot.clone().unwrap_or(q.id.clone()));
                }
                let count = m
                    .state
                    .get("legacy_route_exits")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                m.state
                    .insert("legacy_route_exits".into(), json!(count + 1));
                return emit(
                    o,
                    A::Leave { complete: false },
                    "S025-27",
                    "legacy_route_exhausted",
                    ctx,
                );
            }
            Err(e) => return Err(e),
            _ => {}
        }
    }
    Ok(None)
}

pub fn town_camp(
    o: &Observation,
    m: &mut Memory,
    ctx: &Context<'_>,
    learn: bool,
) -> Result<Option<Proposal>> {
    if !ctx.on("exploration") {
        return Ok(None);
    }
    let shield = o
        .town
        .heroes
        .iter()
        .any(|h| m.legacy.target_party.contains(&h.guid) && h.class == "shieldbreaker");
    for h in o
        .town
        .heroes
        .iter()
        .filter(|h| m.legacy.target_party.contains(&h.guid))
    {
        let mut desired = ctx.profile.parameters["dungeon"]["camp_skills"][&h.class]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>();
        if shield {
            desired.insert(
                0,
                ctx.profile.parameters["dungeon"]["shieldbreaker_pep_talk"]
                    .as_str()
                    .unwrap()
                    .into(),
            );
        }
        if learn {
            for id in &desired {
                if h.camping.contains(id) {
                    continue;
                }
                if let Some(c) = o.town.actions.iter().find(|c| {
                    c.action
                        == TownAction::LearnCamp {
                            hero: h.guid,
                            skill: id.clone(),
                        }
                        && m.ledger.permits(
                            ledger::Purpose::Training,
                            c.cost.get("gold:").copied().unwrap_or(0),
                            true,
                        )
                }) {
                    return proposal(
                        o,
                        Action::Town(c.action.clone()),
                        "S025-20",
                        "camp_learning_after_essentials",
                        ctx,
                    )
                    .map(Some);
                }
            }
        } else {
            let mut selected = desired
                .into_iter()
                .filter(|s| h.camping.contains(s))
                .take(3)
                .collect::<Vec<_>>();
            for id in h.selected_camping.iter().chain(&h.camping) {
                if selected.len() < 3 && !selected.contains(id) {
                    selected.push(id.clone())
                }
            }
            if selected != h.selected_camping {
                return proposal(
                    o,
                    Action::Town(TownAction::SelectCamping {
                        hero: h.guid,
                        skills: selected,
                    }),
                    "S025-20",
                    "camp_loadout",
                    ctx,
                )
                .map(Some);
            }
        }
    }
    Ok(None)
}

/// Original core permits reorder only at an idle boundary. Prepare before a known altar;
/// an unscouted altar with no legal reorder is left alone unless already in a worst order.
pub(super) fn altar_preparation(
    o: &Observation,
    m: &mut Memory,
    ctx: &Context<'_>,
) -> Result<Option<A>> {
    let r = o.expedition.as_ref().ok_or("025 altar boundary absent")?;
    if r.heroes
        .iter()
        .filter(|h| r.party.contains(&h.guid))
        .any(|h| h.skills.is_empty())
    {
        return Ok(None);
    }
    let score = |party: &[u64]| {
        party
            .iter()
            .enumerate()
            .map(|(i, id)| {
                r.heroes.iter().find(|h| h.guid == *id).map_or(0, |h| {
                    h.skills
                        .iter()
                        .filter(|s| {
                            ctx.sources
                                .cards
                                .skills
                                .get(&h.class)
                                .and_then(|c| c.get(*s))
                                .is_some_and(|a| {
                                    a.available_at_start && a.launch.contains(&(i as u8 + 1))
                                })
                        })
                        .count()
                })
            })
            .sum::<usize>()
    };
    fn permutations(ids: &mut Vec<u64>, i: usize, out: &mut Vec<Vec<u64>>) {
        if i == ids.len() {
            out.push(ids.clone());
            return;
        }
        for j in i..ids.len() {
            ids.swap(i, j);
            permutations(ids, i + 1, out);
            ids.swap(i, j);
        }
    }
    let mut orders = vec![];
    permutations(&mut r.party.clone(), 0, &mut orders);
    let minimum = orders.iter().map(|p| score(p)).min().unwrap_or(0);
    let worst = orders
        .into_iter()
        .find(|p| score(p) == minimum)
        .unwrap_or_else(|| r.party.clone());
    if current(r) == Some("shamblers_altar") {
        let prepared = m.state.get("altar_ready").is_some_and(|v| {
            v["returns"].as_u64() == Some(u64::from(o.returns))
                && r.map
                    .as_ref()
                    .is_some_and(|map| v["position"] == map.position)
                && v["party"] == json!(r.party)
        });
        if prepared || score(&r.party) == minimum {
            return Ok(r
                .actions
                .iter()
                .find(|a| matches!(a,A::Interact{item:Some(i),..}if i=="torch"))
                .cloned());
        }
        return Ok(None);
    }
    let target = r
        .actions
        .iter()
        .filter_map(|a| if let A::Move(id) = a { Some(id) } else { None })
        .find(|id| {
            r.map.as_ref().is_some_and(|map| {
                map.nodes.iter().any(|n| {
                    &n.id == *id
                        && n.content.as_ref().is_some_and(|c| {
                            c.curio && c.curio_name.as_deref() == Some("shamblers_altar")
                        })
                })
            })
        });
    if let Some(target) = target {
        if score(&r.party) != minimum {
            let a = A::Reorder(worst);
            return Ok(r.actions.contains(&a).then_some(a));
        }
        m.state.insert(
            "altar_ready".into(),
            json!({"returns":o.returns,"position":target,"party":r.party}),
        );
        return Ok(Some(A::Move(target.clone())));
    }
    Ok(None)
}
