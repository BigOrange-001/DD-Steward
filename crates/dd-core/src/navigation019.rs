//! Navigation memory built exclusively from filtered observations. No map/source access.
use crate::{auto_policy018, campaign017, maps014, opening015};
use dd_data::combat::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: &str = "visible-navigation-019-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    pub version: String,
    worlds: BTreeMap<String, Graph>,
    active: Option<String>,
    pub last_obstruction: Option<String>,
}
impl Default for Memory {
    fn default() -> Self {
        Self {
            version: VERSION.into(),
            worlds: BTreeMap::new(),
            active: None,
            last_obstruction: None,
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Graph {
    nodes: BTreeMap<String, Node>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Node {
    room: bool,
    position: [f64; 2],
    visited: bool,
    exits: BTreeSet<String>,
    gate_key: Option<String>,
    opened: bool,
}
impl Memory {
    pub fn validate(&self) -> Result<()> {
        if self.version != VERSION
            || self.worlds.len() > 64
            || self
                .active
                .as_ref()
                .is_some_and(|id| !self.worlds.contains_key(id))
        {
            return Err("invalid 019 navigation identity".into());
        }
        for (id, graph) in &self.worlds {
            if id.is_empty()
                || graph.nodes.len() > 100_000
                || graph.nodes.iter().any(|(id, n)| {
                    id.is_empty()
                        || n.position.iter().any(|n| !n.is_finite())
                        || n.exits.iter().any(|to| !graph.nodes.contains_key(to))
                })
            {
                return Err("invalid 019 observed graph".into());
            }
        }
        Ok(())
    }
    /// The identity is a public quest instance within this runner. Persistent
    /// courtyard instances survive regrouping; generated instances are discarded.
    pub fn observe(
        &mut self,
        world: &str,
        map: &maps014::Observation,
    ) -> Result<maps014::Observation> {
        self.validate()?;
        let mut next = self.clone();
        let graph = next.worlds.entry(world.into()).or_default();
        for n in &map.nodes {
            let old = graph.nodes.entry(n.id.clone()).or_insert_with(|| Node {
                room: n.room,
                position: n.position,
                visited: false,
                exits: BTreeSet::new(),
                gate_key: None,
                opened: false,
            });
            if old.room != n.room || old.position != n.position {
                return Err("observed map geometry changed under navigation identity".into());
            }
            old.visited |= n.visited;
            old.exits.extend(n.exits.iter().cloned());
            if let Some(key) = &n.gate_key {
                if old.gate_key.as_ref().is_some_and(|k| k != key) {
                    return Err("observed gate identity changed".into());
                }
                old.gate_key = Some(key.clone());
            }
            old.opened |= n.opened;
        }
        next.active = Some(world.into());
        next.validate()?;
        let graph = &next.worlds[world];
        let mut view = map.clone();
        view.nodes = graph
            .nodes
            .iter()
            .map(|(id, n)| maps014::VisibleNode {
                id: id.clone(),
                room: n.room,
                position: n.position,
                visited: n.visited,
                // Remember geometry, never reconstruct hidden encounter contents.
                content: map
                    .nodes
                    .iter()
                    .find(|v| v.id == *id)
                    .and_then(|v| v.content.clone()),
                exits: n.exits.iter().cloned().collect(),
                gate_key: n.gate_key.clone(),
                opened: n.opened,
            })
            .collect();
        *self = next;
        Ok(view)
    }
}
pub fn choose(
    o: &campaign017::Observation,
    policy: &mut auto_policy018::Memory,
    memory: &mut Memory,
) -> Result<auto_policy018::Decision> {
    memory.validate()?;
    if let Some(farm) = o.content019.as_ref().and_then(|v| v.farm.as_ref()) {
        if policy.version != auto_policy018::VERSION {
            return Err("unknown autonomous policy memory version".into());
        }
        if !policy.was_raiding {
            policy.raids += 1;
            policy.moves = 0;
            policy.was_raiding = true;
        }
        return Ok(auto_policy018::Decision {
            action: auto_policy018::Action::Farm(farm_action(farm)?),
            reason: "依据可见波次、中场、库存与营地选择农庄动作".into(),
        });
    }
    let mut visible = o.clone();
    if let Some(raid) = &mut visible.expedition
        && let Some(map) = &raid.map
    {
        let board = o
            .board
            .as_ref()
            .ok_or("019 navigation needs public quest identity")?;
        let q = board
            .quests
            .iter()
            .find(|q| Some(&q.id) == board.selected.as_ref())
            .ok_or("019 navigation has no selected visible quest")?;
        let persistent = matches!(
            q.plot.as_deref(),
            Some("plot_crimson_court_1" | "plot_crimson_court_2" | "plot_crimson_court_3")
        );
        let world = format!(
            "{};quest:{};visit:{}",
            raid.content_id,
            q.id,
            if persistent { 0 } else { o.returns }
        );
        // Keep only the active generated map and persistent worlds. This
        // avoids accidentally reusing generated geometry at a later visit.
        memory
            .worlds
            .retain(|id, _| id.ends_with(";visit:0") || id == &world);
        if memory
            .active
            .as_ref()
            .is_some_and(|id| !memory.worlds.contains_key(id))
        {
            memory.active = None;
        }
        raid.map = Some(memory.observe(&world, map)?);
    }
    let decision = auto_policy018::choose(&visible, policy)?;
    if decision.reason == "已知路线没有可达目标，暂返并记录受阻任务" {
        let map = visible.expedition.as_ref().and_then(|r| r.map.as_ref());
        memory.last_obstruction = Some(
            if map.is_some_and(|m| {
                m.nodes.iter().any(|n| {
                    !n.opened
                        && n.gate_key
                            .as_ref()
                            .is_some_and(|k| m.keys.get(k).copied().unwrap_or(0) == 0)
                })
            }) {
                "observed_route_requires_missing_key"
            } else {
                "observed_frontier_exhausted_not_proof_of_no_hidden_route"
            }
            .into(),
        );
    } else if matches!(
        decision.action,
        auto_policy018::Action::Raid(opening015::Action::Move(_))
    ) {
        memory.last_obstruction = None;
    }
    Ok(decision)
}

fn farm_action(
    view: &crate::campaign017::content019::farm::View,
) -> Result<crate::campaign017::content019::farm::Action> {
    use crate::Policy;
    use crate::campaign017::content019::farm::Action;
    if view.actions.contains(&Action::Start) {
        return Ok(Action::Start);
    }
    if let Some(a) = view
        .actions
        .iter()
        .find(|a| matches!(a, Action::Take { .. }))
    {
        return Ok(a.clone());
    }
    if view.actions.contains(&Action::LeaveLoot) {
        return Ok(Action::LeaveLoot);
    }
    if let Some(camp) = &view.camping {
        if camp.phase == "meal" {
            for name in ["feast", "full", "half", "none"] {
                let action = Action::Camp(crate::expedition017::CampAction::Meal(name.into()));
                if view.actions.contains(&action) {
                    return Ok(action);
                }
            }
            return Err("farm camping has no affordable visible meal".into());
        }
        return Ok(Action::Camp(crate::expedition017::CampAction::Rest));
    }
    if view.interlude.is_some() {
        if view.quest == "plot_color_of_madness_2"
            && (view
                .interlude
                .as_ref()
                .is_some_and(|i| i.group >= 5 && i.group_complete)
                || view
                    .battle
                    .units
                    .iter()
                    .any(|u| u.hero && !u.corpse && (u.stress >= 160 || u.hp == 0)))
        {
            return Ok(Action::Exit);
        }
        if let Some(a) = view
            .actions
            .iter()
            .find(|a| matches!(a, Action::Interact { item: None, .. }))
        {
            return Ok(a.clone());
        }
        return Ok(Action::Continue);
    }
    if view.battle.outcome.is_some() {
        return Ok(Action::Exit);
    }
    Ok(Action::Fight(
        crate::Baseline
            .choose(&view.battle)
            .ok_or("no visible farm combat action")?,
    ))
}

