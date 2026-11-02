//! Autonomous decisions. Inputs are filtered observations plus serialized policy memory only.
use crate::{
    campaign017::{EventAction, Observation, ProvisionAction, QuestView},
    opening015::Action as RaidAction,
    town016::{self, Action as TownAction},
};
use dd_data::{combat::Result, inventory015::Item};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const VERSION: &str = "visible-policy-018-v2";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    pub version: String,
    pub last_return: u32,
    pub town_steps: u32,
    pub quest: Option<String>,
    pub target_party: Vec<u64>,
    pub blocked: BTreeSet<String>,
    pub moves: u32,
    pub raids: u32,
    pub was_raiding: bool,
}
impl Default for Memory {
    fn default() -> Self {
        Self {
            version: VERSION.into(),
            last_return: 0,
            town_steps: 0,
            quest: None,
            target_party: vec![],
            blocked: BTreeSet::new(),
            moves: 0,
            raids: 0,
            was_raiding: false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    Town(TownAction),
    Event(EventAction),
    Select(String),
    Prepare,
    Provision(ProvisionAction),
    Raid(RaidAction),
    Farm(crate::campaign017::content019::farm::Action),
}
#[derive(Debug, Clone, Serialize)]
pub struct Decision {
    pub action: Action,
    pub reason: String,
}
fn decide(action: Action, reason: &str) -> Decision {
    Decision {
        action,
        reason: reason.into(),
    }
}
pub fn goals(o: &Observation) -> Result<Vec<(String, bool)>> {
    if let Some(p) = &o.steward020 {
        return Ok(p.goals.iter().map(|g| (g.id.clone(), g.done)).collect());
    }
    let a = o
        .automation
        .as_ref()
        .ok_or("018 policy requires explicit automation observation")?;
    let mut goals = vec![];
    for i in 1..=4 {
        let id = format!("plot_darkest_dungeon_{i}");
        let done = a.completed.contains(&id);
        goals.push((id, done));
    }
    for i in 0..=3 {
        let id = format!("plot_crimson_court_{i}");
        let done = a.completed.contains(&id);
        goals.push((id, done));
    }
    for kind in ["farm_bloodcrate", "kill_statue"] {
        goals.push((
            format!("courtyard:{kind}"),
            a.repeat_completions.get(kind).copied().unwrap_or(0) > 0,
        ));
    }
    if o.content019.as_ref().is_some_and(|v| v.farm_enabled) {
        for i in 0..=2 {
            let id = format!("plot_color_of_madness_{i}");
            let done = a.completed.contains(&id);
            goals.push((id, done));
        }
    }
    Ok(goals)
}
fn affordable(o: &town016::Observation, c: &town016::Choice, reserve: u32) -> bool {
    c.cost.iter().all(|(k, n)| {
        *n <= o
            .wallet
            .get(k)
            .copied()
            .unwrap_or(0)
            .saturating_sub(if k == "gold:" { reserve } else { 0 })
    })
}
fn party(o: &Observation, q: &QuestView, relaxed: bool) -> Vec<u64> {
    let mut town = o.town.clone();
    let marked = o
        .automation
        .as_ref()
        .map(|a| a.marked.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let min = if relaxed {
        0
    } else if q
        .plot
        .as_deref()
        .is_some_and(|id| id.starts_with("plot_crimson_court_"))
    {
        if q.difficulty == 1 {
            3
        } else if q.difficulty == 3 {
            4
        } else {
            6
        }
    } else if q.difficulty >= 6 {
        6
    } else {
        q.difficulty.saturating_sub(1)
    };
    town.heroes.retain(|h| {
        h.level >= min
            && !(h.stress >= 100 && matches!(h.resolve, crate::Resolve::Calm))
            && q.maximum_resolve_level.is_none_or(|max| h.level <= max)
            && h.stress < if relaxed { 201 } else { 70 }
            && !(q.dungeon == "darkestdungeon"
                && marked.contains(&h.guid)
                && q.preparation_error
                    .as_deref()
                    .is_some_and(|s| s.contains("never again")))
    });
    if q.dungeon == "darkestdungeon" && o.automation.as_ref().is_some_and(|a| !a.can_reenter) {
        town.heroes.retain(|h| !marked.contains(&h.guid));
    }
    crate::campaign_policy017::party(&town)
}
fn choose_quest(o: &Observation, m: &Memory) -> Option<(QuestView, Vec<u64>)> {
    let board = o.board.as_ref()?;
    let completed = &o.automation.as_ref()?.completed;
    let blood = o.town.wallet.get("estate:the_blood").copied().unwrap_or(0);
    let infected = o
        .town
        .heroes
        .iter()
        .filter(|h| h.alive && h.diseases.iter().any(|id| id.contains("vampire")))
        .count();
    let mut options = board
        .quests
        .iter()
        .filter(|q| {
            q.execution != "expedition integration pending"
                && q.estate_requirements
                    .iter()
                    .all(|(id, n)| o.town.wallet.get(id).copied().unwrap_or(0) >= *n)
                && !m.blocked.contains(q.plot.as_ref().unwrap_or(&q.id))
        })
        .filter_map(|q| {
            let p = party(o, q, false);
            if p.len() < 4 {
                return None;
            }
            let minimum = o
                .town
                .heroes
                .iter()
                .filter(|h| p.contains(&h.guid))
                .map(|h| h.level)
                .min()
                .unwrap_or(0);
            let priority = if q.dungeon == "courtyard"
                && q.kind == "farm_bloodcrate"
                && infected > 0
                && blood < infected as u32 * 3
            {
                0
            } else if q
                .plot
                .as_deref()
                .is_some_and(|id| id.starts_with("plot_darkest_dungeon_"))
                && minimum >= 6
                && o.town
                    .heroes
                    .iter()
                    .filter(|h| p.contains(&h.guid))
                    .all(|h| h.weapon >= 3 && h.armour >= 3)
            {
                1
            } else if q
                .plot
                .as_deref()
                .is_some_and(|id| id.starts_with("plot_crimson_court_"))
                && minimum
                    >= if q.difficulty == 1 {
                        3
                    } else if q.difficulty == 3 {
                        4
                    } else {
                        6
                    }
            {
                2
            } else if q.dungeon == "courtyard"
                && ["farm_bloodcrate", "kill_statue"].contains(&q.kind.as_str())
                && (q.kind == "farm_bloodcrate"
                    || completed.contains(&"plot_crimson_court_3".into()))
                && o.automation
                    .as_ref()
                    .unwrap()
                    .repeat_completions
                    .get(&q.kind)
                    .copied()
                    .unwrap_or(0)
                    == 0
            {
                if q.kind == "farm_bloodcrate" { 1 } else { 3 }
            } else if o.content019.is_some()
                && q.dungeon == "farm"
                && q.plot.as_ref().is_some_and(|id| !completed.contains(id))
            {
                4
            } else if q.plot.is_none()
                && ["crypts", "warrens", "weald", "cove"].contains(&q.dungeon.as_str())
            {
                5
            } else {
                return None;
            };
            Some((
                (
                    priority,
                    q.length,
                    usize::from(q.dungeon != "crypts"),
                    q.difficulty,
                    q.id.clone(),
                ),
                q.clone(),
                p,
            ))
        })
        .collect::<Vec<_>>();
    options.sort_by_key(|v| v.0.clone());
    if let Some((_, q, p)) = options.into_iter().next() {
        return Some((q, p));
    }
    let ordinary = board
        .quests
        .iter()
        .filter(|q| {
            q.plot.is_none()
                && q.execution != "expedition integration pending"
                && ["crypts", "warrens", "weald", "cove"].contains(&q.dungeon.as_str())
        })
        .filter_map(|q| {
            let p = party(o, q, true);
            (!p.is_empty()).then_some((q.clone(), p))
        })
        .min_by_key(|(q, _)| (q.difficulty, q.length, q.id.clone()));
    if ordinary.is_some() {
        return ordinary;
    }
    // Source plot bosses can occupy every ordinary board slot. They are already
    // supported expeditions; retain the earlier preferences before this recovery.
    board
        .quests
        .iter()
        .filter(|q| {
            q.plot.is_some()
                && q.kind == "kill_boss"
                && q.execution == "018 native expedition"
                && ["crypts", "warrens", "weald", "cove"].contains(&q.dungeon.as_str())
                && !m.blocked.contains(q.plot.as_ref().unwrap())
                && q.estate_requirements
                    .iter()
                    .all(|(id, n)| o.town.wallet.get(id).copied().unwrap_or(0) >= *n)
        })
        .filter_map(|q| {
            let p = party(o, q, true);
            (!p.is_empty()).then_some((q.clone(), p))
        })
        .min_by_key(|(q, _)| (q.difficulty, q.length, q.id.clone()))
}
/// Read-only access for the new final recovery layer; the old policy is unchanged.
pub(crate) fn recovery_selection024b(o: &Observation, m: &Memory) -> Option<(QuestView, Vec<u64>)> {
    choose_quest(o, m)
}
fn talismans(o: &Observation, q: &QuestView, p: &[u64]) -> Option<TownAction> {
    if q.plot.as_deref() != Some("plot_darkest_dungeon_2") {
        return None;
    }
    let protected = p
        .iter()
        .filter(|id| {
            o.town
                .heroes
                .iter()
                .any(|h| h.guid == **id && h.class != "vestal")
        })
        .take(3)
        .copied()
        .collect::<Vec<_>>();
    for id in protected {
        let h = o.town.heroes.iter().find(|h| h.guid == id)?;
        if h.equipped.iter().any(|(_, _, item)| item == "dd_trinket") {
            continue;
        }
        if h.equipped.iter().any(|(slot, _, _)| *slot == 0) {
            let a = TownAction::Unequip { hero: id, slot: 0 };
            if o.town.actions.iter().any(|c| c.action == a) {
                return Some(a);
            }
        }
        for (instance, item, owner) in &o.town.trinkets {
            if item != "dd_trinket" {
                continue;
            }
            if let Some((hero, slot)) = owner {
                if !protected_has(&o.town, *hero) {
                    let a = TownAction::Unequip {
                        hero: *hero,
                        slot: *slot,
                    };
                    if o.town.actions.iter().any(|c| c.action == a) {
                        return Some(a);
                    }
                }
            } else {
                let a = TownAction::Equip {
                    hero: id,
                    instance: *instance,
                    slot: 0,
                };
                if o.town.actions.iter().any(|c| c.action == a) {
                    return Some(a);
                }
            }
        }
    }
    None
}
fn protected_has(t: &town016::Observation, id: u64) -> bool {
    t.party.contains(&id) && t.heroes.iter().any(|h| h.guid == id && h.class != "vestal")
}
pub fn choose(o: &Observation, m: &mut Memory) -> Result<Decision> {
    choose_with_city025(o, m, crate::campaign_policy017::city)
}
/// Additive025 adapter: a caller can suppress a city stage it already owns.
/// The original entry passes the unchanged city policy above.
pub(crate) fn choose_with_city025(
    o: &Observation,
    m: &mut Memory,
    city: fn(&crate::town016::Observation) -> Option<TownAction>,
) -> Result<Decision> {
    if m.version != VERSION {
        return Err("unknown autonomous policy memory version".into());
    }
    if m.last_return != o.returns {
        m.last_return = o.returns;
        m.town_steps = 0;
        m.quest = None;
        m.target_party.clear();
        m.moves = 0;
    }
    if let Some(offer) = &o.event_offer {
        let action = if offer.actions.contains(&EventAction::Accept) {
            EventAction::Accept
        } else if let Some(a) = offer
            .actions
            .iter()
            .find(|a| matches!(a, EventAction::Revive(_)))
        {
            a.clone()
        } else {
            EventAction::Decline
        };
        return Ok(decide(Action::Event(action), "处理本周事件的合法选择"));
    }
    if let Some(raid) = &o.expedition {
        if !m.was_raiding {
            m.raids += 1;
            m.moves = 0;
            m.was_raiding = true;
        }
        let food = raid.inventory.count(&Item::new("provision", ""));
        let danger = raid
            .heroes
            .iter()
            .any(|h| h.alive && raid.party.contains(&h.guid) && (h.stress >= 160 || h.hp == 0));
        if raid.battle.is_none()
            && raid.camping.is_none()
            && raid.loot.is_empty()
            && !raid.quest_ready
            && m.moves > 0
            && raid
                .map
                .as_ref()
                .is_some_and(|m| m.nodes.iter().filter(|n| n.room && n.visited).count() >= 4)
            && (danger || food < raid.party.len() as u32)
            && raid
                .actions
                .contains(&RaidAction::Leave { complete: false })
        {
            return Ok(decide(
                Action::Raid(RaidAction::Leave { complete: false }),
                "补给或状态不足，按真实规则撤回",
            ));
        }
        let action = match raid_action(raid) {
            Ok(a) => a,
            Err(e)
                if e == "visible navigation exhausted before quest ready"
                    && raid
                        .actions
                        .contains(&RaidAction::Leave { complete: false }) =>
            {
                if let Some(q) = o
                    .board
                    .as_ref()
                    .and_then(|b| b.quests.iter().find(|q| Some(&q.id) == m.quest.as_ref()))
                {
                    m.blocked.insert(q.plot.clone().unwrap_or(q.id.clone()));
                }
                return Ok(decide(
                    Action::Raid(RaidAction::Leave { complete: false }),
                    "已知路线没有可达目标，暂返并记录受阻任务",
                ));
            }
            Err(e) => return Err(e),
        };
        if matches!(action, RaidAction::Move(_)) {
            m.moves += 1;
        }
        return Ok(decide(
            Action::Raid(action),
            if raid.battle.is_some() {
                "依据可见敌我状态选择战斗动作"
            } else if !raid.loot.is_empty() {
                "处理有限背包与掉落"
            } else {
                "探索已知路线、交互或完成任务"
            },
        ));
    }
    m.was_raiding = false;
    if let Some(p) = &o.provisions {
        let q = o
            .board
            .as_ref()
            .and_then(|b| b.quests.iter().find(|q| q.id == p.quest));
        let length = q.map_or(1, |q| q.length);
        for (kind, id, n) in [
            (
                "provision",
                "",
                if length == 1 {
                    12
                } else if length == 2 {
                    24
                } else {
                    48
                },
            ),
            (
                "supply",
                "torch",
                if q.is_some_and(|q| q.dungeon == "courtyard") {
                    4
                } else {
                    if length == 1 { 8 } else { 16 }
                },
            ),
            ("supply", "shovel", if length == 1 { 2 } else { 4 }),
            (
                "supply",
                "antivenom",
                if q.is_some_and(|q| q.dungeon == "weald") {
                    2
                } else {
                    0
                },
            ),
            ("supply", "bandage", if length == 1 { 1 } else { 3 }),
            ("supply", "medicinal_herbs", if length == 1 { 1 } else { 3 }),
            ("supply", "skeleton_key", 2),
        ] {
            let item = Item::new(kind, id);
            let a = ProvisionAction::Buy {
                item: item.clone(),
                amount: 1,
            };
            if p.inventory.count(&item) < n && p.actions.contains(&a) {
                return Ok(decide(Action::Provision(a), "按任务长度购买有限补给"));
            }
        }
        return Ok(decide(
            Action::Provision(ProvisionAction::Embark),
            "补给完成，正式出征",
        ));
    }
    if o.failure.is_some() {
        return Err("native campaign failure reached".into());
    }
    m.town_steps += 1;
    let desired = 16;
    if o.town.heroes.iter().filter(|h| h.alive).count() < desired {
        if let Some(c) = o
            .town
            .actions
            .iter()
            .filter(|c| matches!(c.action, TownAction::Hire { .. }))
            .min_by_key(|c| {
                if let TownAction::Hire { guid } = c.action {
                    o.town
                        .recruits
                        .iter()
                        .find(|h| h.guid == guid)
                        .map_or((100, 100, 100), |h| {
                            (
                                o.town
                                    .heroes
                                    .iter()
                                    .filter(|x| x.alive && x.class == h.class)
                                    .count(),
                                usize::from(!["vestal", "occultist"].contains(&h.class.as_str())),
                                h.guid as usize,
                            )
                        })
                } else {
                    (100, 100, 100)
                }
            })
        {
            return Ok(decide(
                Action::Town(c.action.clone()),
                "招募并扩充可轮换队伍",
            ));
        }
        if o.town.roster_capacity < desired as u32
            && let Some(c)=o.town.actions.iter().find(|c|matches!(&c.action,TownAction::Upgrade{hero:None,tree,..} if tree.contains("roster"))&&affordable(&o.town,c,3000)){return Ok(decide(Action::Town(c.action.clone()),"提升驿马车名册容量"));}
    }
    if m.quest.is_none()
        && o.town.heroes.iter().filter(|h| h.alive).count() >= o.town.roster_capacity as usize
        && !o.town.recruits.is_empty()
        && let Some(h) = o
            .town
            .heroes
            .iter()
            .filter(|h| {
                h.alive
                    && !h.busy
                    && !h.missing
                    && (h.level <= 1 && h.stress >= 80
                        || h.stress >= 100 && matches!(h.resolve, crate::Resolve::Calm))
            })
            .max_by_key(|h| h.stress)
    {
        let a = TownAction::Dismiss { hero: h.guid };
        if o.town.actions.iter().any(|c| c.action == a) {
            return Ok(decide(
                Action::Town(a),
                "以可招募的新兵替换难以负担治疗的低级高压英雄",
            ));
        }
    }
    if m.quest.is_none()
        && m.town_steps < 200
        && let Some(a) = city(&o.town)
    {
        return Ok(decide(Action::Town(a), "治疗、学习技能、提升装备和建筑"));
    }
    let (q, p) = if let Some(id) = &m.quest {
        if let Some(q) = o
            .board
            .as_ref()
            .and_then(|b| b.quests.iter().find(|q| q.id == *id))
        {
            let p = m.target_party.clone();
            (q.clone(), p)
        } else {
            m.quest = None;
            choose_quest(o, m).ok_or("no executable visible quest or recoverable party")?
        }
    } else {
        choose_quest(o, m).ok_or("no executable visible quest or recoverable party")?
    };
    m.quest = Some(q.id.clone());
    m.target_party = p.clone();
    if !p.is_empty() && p != o.town.party {
        return Ok(decide(
            Action::Town(TownAction::Party(p)),
            "按任务等级、可用状态与职责配队",
        ));
    }
    if let Some(a) = talismans(o, &q, &o.town.party) {
        return Ok(decide(Action::Town(a), "为第二次极暗地牢配置火焰护符"));
    }
    if m.town_steps < 260 {
        let mut visible = o.town.clone();
        visible.heroes.retain(|h| visible.party.contains(&h.guid));
        visible.recruits.clear();
        if q.plot.as_deref() != Some("plot_darkest_dungeon_2") {
            let dd = visible
                .trinkets
                .iter()
                .filter(|(_, id, _)| id == "dd_trinket")
                .map(|(id, _, _)| *id)
                .collect::<BTreeSet<_>>();
            visible.trinkets.retain(|(_, id, _)| id != "dd_trinket");
            visible.actions.retain(
                |c| !matches!(c.action,TownAction::Equip{instance,..} if dd.contains(&instance)),
            );
        }
        if let Some(a) = city(&visible)
            && !matches!(a, TownAction::Party(_) | TownAction::Hire { .. })
        {
            return Ok(decide(Action::Town(a), "强化已选队伍，保留出征预算"));
        }
    }
    if o.board.as_ref().and_then(|b| b.selected.as_ref()) != Some(&q.id) {
        return Ok(decide(
            Action::Select(q.id),
            "选择当前可执行且有成长或目标收益的任务",
        ));
    }
    if !q.preparation_eligible {
        return Err(format!("chosen quest not ready: {:?}", q.preparation_error));
    }
    Ok(decide(Action::Prepare, "冻结任务与队伍，进入补给"))
}

/// Tactics use visible damage/heal previews and public class/health only.
pub fn raid_action(o: &crate::opening015::Observation) -> Result<RaidAction> {
    if let Some(a) = priority_loot(&o.loot, &o.inventory, &o.actions) {
        return Ok(a);
    }
    if let Some(b) = &o.battle {
        let actor = b.units.iter().find(|u| Some(u.id) == b.actor);
        if let Some(actor) = actor
            && (actor.class == "vestal" || actor.class == "jester")
            && actor.rank < 3
            && let Some(a) = b
                .actions
                .iter()
                .filter(|a| matches!(a.action, crate::Action::Move { .. }))
                .min_by_key(|a| match a.action {
                    crate::Action::Move { rank } => rank.abs_diff(3),
                    _ => u8::MAX,
                })
        {
            return Ok(RaidAction::Fight(a.action.clone()));
        }
        if actor.is_some_and(|u| u.class == "jester") {
            if let Some(a) = b
                .actions
                .iter()
                .filter(|a| a.stress_heal > 0)
                .filter(|a| {
                    a.targets
                        .iter()
                        .any(|id| b.units.iter().any(|u| u.id == *id && u.stress >= 35))
                })
                .max_by_key(|a| {
                    a.targets
                        .iter()
                        .filter_map(|id| b.units.iter().find(|u| u.id == *id))
                        .map(|u| u.stress)
                        .max()
                        .unwrap_or(0)
                })
            {
                return Ok(RaidAction::Fight(a.action.clone()));
            }
            if b.round <= 2
                && let Some(a) = b.actions.iter().find(|a| a.label == "battle_ballad")
            {
                return Ok(RaidAction::Fight(a.action.clone()));
            }
        }
    }
    if o.battle.is_none() {
        let score = |ids: &[u64]| -> usize {
            ids.iter()
                .enumerate()
                .map(|(rank, id)| {
                    let class = o
                        .heroes
                        .iter()
                        .find(|h| h.guid == *id)
                        .map(|h| h.class.as_str())
                        .unwrap_or("");
                    let desired = match class {
                        "vestal" | "occultist" => 3,
                        "jester" | "plague_doctor" | "arbalest" | "antiquarian" | "runaway" => 2,
                        "highwayman" | "bounty_hunter" | "houndmaster" | "grave_robber" => 1,
                        _ => 0,
                    };
                    rank.abs_diff(desired.min(ids.len().saturating_sub(1)))
                })
                .sum()
        };
        if let Some(a) = o
            .actions
            .iter()
            .filter(|a| matches!(a,RaidAction::Reorder(ids) if score(ids)<score(&o.party)))
            .min_by_key(|a| {
                if let RaidAction::Reorder(ids) = a {
                    score(ids)
                } else {
                    usize::MAX
                }
            })
        {
            return Ok(a.clone());
        }
    }
    crate::campaign_policy017::choose(o)
}

fn important(i: &Item) -> bool {
    i.kind == "quest_item"
        || i.kind == "estate_currency" && i.id.starts_with("crimson_court_invitation_")
        || i.kind == "estate" && i.id == "the_blood"
}
fn priority_loot(
    loot: &[dd_data::inventory015::Stack],
    bag: &dd_data::inventory015::Bag,
    actions: &[RaidAction],
) -> Option<RaidAction> {
    let (index, _) = loot.iter().enumerate().find(|(_, s)| important(&s.item))?;
    if let Some(a) = actions
        .iter()
        .find(|a| matches!(a,RaidAction::Take{index:i,..}if *i==index))
    {
        return Some(a.clone());
    }
    let discard = bag
        .slots
        .iter()
        .enumerate()
        .filter_map(|(slot, s)| s.as_ref().map(|s| (slot, s)))
        .filter(|(_, s)| !important(&s.item) && s.item.kind != "provision")
        .filter(|(slot, s)| {
            actions.contains(&RaidAction::Discard {
                slot: *slot,
                amount: s.amount,
            })
        })
        .min_by_key(|(_, s)| {
            (
                !["gold", "gem", "journal_page"].contains(&s.item.kind.as_str()),
                s.amount,
            )
        });
    if let Some((slot, s)) = discard {
        return Some(RaidAction::Discard {
            slot,
            amount: s.amount,
        });
    }
    actions
        .contains(&RaidAction::LeaveLoot)
        .then_some(RaidAction::LeaveLoot)
}
