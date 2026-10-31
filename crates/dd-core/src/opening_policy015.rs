//! Fixed visible-only opening policy, shared by legacy CLI and campaign acceptance.
use crate::{
    Baseline, Policy,
    opening015::{Action, Observation, Phase},
};
use dd_data::{combat::Result, inventory015::Item};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub fn choose(o: &Observation) -> Result<Action> {
    if !o.loot.is_empty() {
        if let Some(a) = o.actions.iter().find(|a| matches!(a, Action::Take { .. })) {
            return Ok(a.clone());
        }
        return Ok(Action::LeaveLoot);
    }
    if o.phase == Phase::Preparation {
        for (kind, id, target) in [
            ("provision", "", 12),
            ("supply", "torch", 12),
            ("supply", "shovel", 2),
            ("supply", "skeleton_key", 2),
            ("supply", "holy_water", 2),
            ("supply", "bandage", 2),
        ] {
            let item = Item::new(kind, id);
            let a = Action::Buy {
                item: item.clone(),
                amount: 1,
            };
            if o.inventory.count(&item) < target && o.actions.contains(&a) {
                return Ok(a);
            }
        }
        return Ok(Action::Embark);
    }
    if let Some(b) = &o.battle {
        let a = Baseline.choose(b).ok_or("battle policy has no action")?;
        return Ok(Action::Fight(a));
    }
    if let Some(a) = o.actions.iter().find(|a| {
        matches!(
            a,
            Action::Obstacle { shovel: true } | Action::Hunger { eat: true }
        )
    }) {
        return Ok(a.clone());
    }
    if let Some(a) = o.actions.iter().find(|a| {
        matches!(
            a,
            Action::Obstacle { shovel: false }
                | Action::Hunger { eat: false }
                | Action::Disarm { .. }
        )
    }) {
        return Ok(a.clone());
    }
    if o.torch < 76
        && let Some(a)=o.actions.iter().find(|a|matches!(a,Action::Use{slot,..}if o.inventory.slots[*slot].as_ref().is_some_and(|s|s.item.id=="torch"))){return Ok(a.clone());}
    let m = o.map.as_ref().ok_or("missing visible active map")?;
    let curio = m
        .nodes
        .iter()
        .find(|n| n.id == m.position)
        .and_then(|n| n.content.as_ref())
        .and_then(|c| c.curio_name.as_deref());
    if let Some(name) = curio {
        let item = match name {
            "locked_strongbox" | "bandits_trapped_chest" => Some("skeleton_key"),
            "holy_fountain" => Some("holy_water"),
            _ => None,
        };
        let safe = !matches!(name, "transcendent_terror" | "bandits_trapped_chest")
            || item.is_some_and(|id| o.inventory.count(&Item::new("supply", id)) > 0);
        if safe
            && let Some(a)=o.actions.iter().find(|a|matches!(a,Action::Interact{item:i,..}if i.as_deref()==item&&i.as_ref().is_none_or(|id|o.inventory.count(&Item::new("supply",id))>0))){return Ok(a.clone());}
    }
    if let Some(a) = o
        .actions
        .iter()
        .find(|a| matches!(a, Action::Leave { complete: true }))
    {
        return Ok(a.clone());
    }
    let graph = m
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n))
        .collect::<BTreeMap<_, _>>();
    let mut todo = VecDeque::from([m.position.clone()]);
    let mut prev = BTreeMap::new();
    let mut seen = BTreeSet::from([m.position.clone()]);
    let mut target = None;
    while let Some(id) = todo.pop_front() {
        if !graph[&id].visited {
            target = Some(id);
            break;
        }
        for to in &graph[&id].exits {
            if seen.insert(to.clone()) {
                prev.insert(to.clone(), id.clone());
                todo.push_back(to.clone());
            }
        }
    }
    let mut target = target.ok_or("visible navigation exhausted before quest ready")?;
    while prev.get(&target).is_some_and(|v| v != &m.position) {
        target = prev[&target].clone();
    }
    let a = Action::Move(target);
    if !o.actions.contains(&a) {
        return Err(format!("visible route not legal: {a:?}"));
    }
    Ok(a)
}
