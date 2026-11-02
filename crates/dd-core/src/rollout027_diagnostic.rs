//! Offline only: stochastic open-loop UCT with full-battle terminal rollouts.
//! Native worlds here are freshly generated diagnostic fixtures, never campaign saves.
use super::*;
use crate::strategy025 as s25;
use std::{
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

#[derive(Default)]
struct Node {
    visits: u32,
    sum: f64,
    children: BTreeMap<String, usize>,
}
struct Policy {
    memory: s25::Memory,
    sources: s25::Sources,
    old: crate::tactics021::Memory,
    gate_retreat: bool,
}
impl Policy {
    fn new(e: &arena::Engine) -> Self {
        Self {
            memory: Default::default(),
            sources: e.sources.clone(),
            old: Default::default(),
            gate_retreat: true,
        }
    }
    fn action(
        &mut self,
        e: &arena::Engine,
        m: &Surrogate,
        input: &PublicBattle,
        s: &Settings,
    ) -> Result<Action> {
        let mut o = m.observation();
        if self.gate_retreat && o.actions.iter().any(|a| a.action == Action::Retreat) {
            let mut current = input.clone();
            current.observation = o.clone();
            if !retreat::assess(m, &current, s)?.allowed {
                o.actions.retain(|a| a.action != Action::Retreat);
            }
        }
        e.diagnostic_rule(
            &o,
            input,
            &mut self.memory,
            &mut self.sources,
            &mut self.old,
        )
    }
}
fn terminal(m: &Surrogate, input: &PublicBattle, s: &Settings, doors: u32) -> SampleResult {
    let afflicted = m
        .battle
        .units
        .iter()
        .enumerate()
        .filter(|(id, u)| {
            matches!(u.resolve, Resolve::Afflicted(_))
                && input
                    .observation
                    .units
                    .iter()
                    .any(|old| old.id == *id && !matches!(old.resolve, Resolve::Afflicted(_)))
        })
        .map(|(id, _)| id)
        .collect();
    decision::summarize(m, input, s, doors, &afflicted)
}
fn choose(
    e: &arena::Engine,
    root: &Surrogate,
    input: &PublicBattle,
    s: &Settings,
    cfg: &Value,
    decision_index: usize,
) -> Result<(Action, Value)> {
    let timer = Instant::now();
    let mut nodes = vec![Node::default()];
    let mut first_actions = BTreeMap::new();
    let mut max_depth = 0;
    let iterations = cfg["iterations"].as_u64().unwrap() as usize;
    let depth_limit = cfg["tree_depth"].as_u64().unwrap() as usize;
    let horizon = cfg["round_horizon"].as_u64().unwrap() as u32;
    for iteration in 0..iterations {
        if iteration >= 24
            && timer.elapsed().as_millis() >= cfg["per_decision_ms"].as_u64().unwrap() as u128
        {
            break;
        }
        let mut m = root.sample(sample_seed(
            input,
            s,
            1000000 + decision_index * iterations + iteration,
        )?)?;
        let end = m.battle.round + horizon;
        m.battle.scenario.round_limit = end;
        let mut policy = Policy::new(e);
        let mut path = vec![0];
        let mut node = 0;
        let mut expanded = false;
        let mut depth = 0;
        let mut doors = 0;
        for _ in 0..horizon as usize * s.transitions_per_round {
            if m.battle.outcome().is_some() || m.battle.round >= end {
                break;
            }
            let action = if m
                .battle
                .actor
                .is_some_and(|id| m.battle.units[id].friendly())
            {
                if !expanded && depth < depth_limit {
                    let mut o = m.observation();
                    let mut current = input.clone();
                    current.observation = o.clone();
                    if !retreat::assess(&m, &current, s)?.allowed {
                        o.actions.retain(|a| a.action != Action::Retreat);
                    }
                    let prior = policy.action(e, &m, &current, s)?;
                    // Bounded diagnosis: omit non-healing consumables unless the rule prior selects them.
                    o.actions.retain(|a| {
                        !matches!(a.action, Action::Item { .. })
                            || a.heal[1] > 0
                            || a.action == prior
                    });
                    o.actions.sort_by_key(|a| usize::from(a.action != prior));
                    let actions = o
                        .actions
                        .iter()
                        .map(|a| {
                            Ok((
                                serde_json::to_string(&a.action).map_err(|e| e.to_string())?,
                                a.action.clone(),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let unvisited = actions
                        .iter()
                        .find(|(key, _)| !nodes[node].children.contains_key(key));
                    let (key, action) = if let Some(a) = unvisited {
                        expanded = true;
                        a.clone()
                    } else {
                        actions
                            .into_iter()
                            .max_by(|(ka, _), (kb, _)| {
                                let score = |key: &String| {
                                    let n = &nodes[nodes[node].children[key]];
                                    n.sum / n.visits.max(1) as f64
                                        + cfg["uct_exploration"].as_f64().unwrap()
                                            * ((nodes[node].visits.max(1) as f64).ln()
                                                / n.visits.max(1) as f64)
                                                .sqrt()
                                };
                                score(ka).total_cmp(&score(kb))
                            })
                            .ok_or("diagnostic tree has no legal action")?
                    };
                    if node == 0 {
                        first_actions.insert(key.clone(), action.clone());
                    }
                    let next = if let Some(next) = nodes[node].children.get(&key) {
                        *next
                    } else {
                        let n = nodes.len();
                        nodes.push(Node::default());
                        nodes[node].children.insert(key, n);
                        n
                    };
                    node = next;
                    path.push(node);
                    depth += 1;
                    max_depth = max_depth.max(depth);
                    action
                } else {
                    policy.action(e, &m, input, s)?
                }
            } else {
                m.battle.monster_action()?
            };
            for event in m.step(action)? {
                if matches!(event, Event::DeathsDoor(id) if input.heroes.contains_key(&id)) {
                    doors += 1;
                }
            }
        }
        let value = terminal(&m, input, s, doors).score / cfg["score_scale"].as_f64().unwrap();
        for id in path {
            nodes[id].visits += 1;
            nodes[id].sum += value;
        }
    }
    let (key, id) = nodes[0]
        .children
        .iter()
        .max_by(|(_, a), (_, b)| {
            let a = &nodes[**a];
            let b = &nodes[**b];
            (a.sum / a.visits.max(1) as f64)
                .total_cmp(&(b.sum / b.visits.max(1) as f64))
                .then(a.visits.cmp(&b.visits))
        })
        .ok_or("diagnostic no completed tree")?;
    let edges=nodes[0].children.iter().map(|(k,id)|serde_json::json!({"action":first_actions[k],"visits":nodes[*id].visits,"mean_score":nodes[*id].sum/nodes[*id].visits.max(1) as f64*cfg["score_scale"].as_f64().unwrap()})).collect::<Vec<_>>();
    Ok((
        first_actions[key].clone(),
        serde_json::json!({"iterations":nodes[0].visits,"nodes":nodes.len(),"max_tree_depth":max_depth,"selected_visits":nodes[*id].visits,"seconds":timer.elapsed().as_secs_f64(),"root_edges":edges}),
    ))
}
fn native_root(
    input: &PublicBattle,
    defs: &Definitions,
    seed: u64,
) -> Result<(Surrogate, PublicBattle)> {
    let mut public = reconstruct(input, defs, 0)?;
    if public
        .battle
        .units
        .iter()
        .any(|u| u.blueprint.hero && !u.alive())
    {
        return Err("diagnostic restart refuses to resurrect a dead hero".into());
    }
    let supplies = std::mem::take(&mut public.battle.supplies);
    let mut m = public.restart(seed)?;
    // Public observations aggregate inventory stacks; Battle::build accepts physical stacks.
    // Restore the exact public aggregate after native entity initialization.
    m.battle.supplies = supplies;
    for _ in 0..100 {
        if m.battle.outcome().is_some()
            || m.battle
                .actor
                .is_some_and(|id| m.battle.units[id].friendly())
        {
            break;
        }
        let action = m.battle.monster_action()?;
        m.step(action)?;
    }
    let mut current = input.clone();
    current.observation = m.observation();
    current.history = Default::default();
    current.retreat_legal = Some(
        current
            .observation
            .actions
            .iter()
            .any(|a| a.action == Action::Retreat),
    );
    Ok((m.diagnostic_current_root(), current))
}
fn probe(
    e: &arena::Engine,
    input: &PublicBattle,
    defs: &Definitions,
    s: &Settings,
) -> Result<Value> {
    let (native, current) = native_root(input, defs, 89127)?;
    let fresh = Definitions::project(&e.game, &current)?;
    let a = reconstruct(&current, &fresh, 0)?;
    let initial_a =
        serde_json::from_str::<Value>(&a.battle.save_json()?).map_err(|e| e.to_string())?;
    let initial_b =
        serde_json::from_str::<Value>(&native.battle.save_json()?).map_err(|e| e.to_string())?;
    let mut divergences = vec![];
    for seed in 0..8 {
        let mut a = a.sample(92000 + seed)?;
        let mut b = native.sample(92000 + seed)?;
        let mut policy = Policy::new(e);
        for step in 0..256 {
            if a.battle.outcome().is_some() || b.battle.outcome().is_some() {
                break;
            }
            let rng_before = (a.battle.rng.0, b.battle.rng.0);
            let (aa, bb) = if b
                .battle
                .actor
                .is_some_and(|id| b.battle.units[id].friendly())
            {
                let action = policy.action(e, &b, &current, s)?;
                (action.clone(), action)
            } else {
                (a.battle.monster_action()?, b.battle.monster_action()?)
            };
            if aa != bb {
                divergences.push(serde_json::json!({"seed":seed,"step":step,"kind":"enemy_action","a":aa,"b":bb}));
                break;
            }
            let ea = a.step(aa)?;
            let eb = b.step(bb)?;
            let oa = a.observation();
            let ob = b.observation();
            if oa != ob {
                divergences.push(serde_json::json!({"seed":seed,"step":step,"kind":"public_state","rng_before":rng_before,"rng_after":[a.battle.rng.0,b.battle.rng.0],"events_a":ea,"events_b":eb,"units_a":oa.units,"units_b":ob.units}));
                break;
            }
        }
    }
    Ok(
        serde_json::json!({"paired_seeds":8,"max_steps":256,"synthetic_initial_a":initial_a,"synthetic_initial_b":initial_b,"divergences":divergences,"initial_public_equal":a.observation()==native.observation()}),
    )
}
fn case(e: &arena::Engine, scene: &Value, out: &Path, cfg: &Value, model: &str) -> Result<Value> {
    let record: Value = serde_json::from_slice(
        &std::fs::read(scene["file"].as_str().ok_or("scene path")?).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let input: PublicBattle =
        serde_json::from_value(record["input"].clone()).map_err(|e| e.to_string())?;
    let defs = Definitions::project(&e.game, &input)?;
    let s = Settings::budgeted()?;
    let mut trials = vec![];
    let start = Instant::now();
    let trial_count = if std::env::var("DIAGNOSTIC_PROBE_ONLY").is_ok() {
        0
    } else {
        cfg["eval_trials"].as_u64().unwrap()
    };
    for trial in 0..trial_count {
        let result = execution025::hypothetical026(input.mechanisms, || {
            execution024b::hypothetical026(input.single_target_fallback, || -> Result<Value> {
                let ordinal = scene["ordinal"].as_u64().unwrap();
                let seed = 77127000 + ordinal * 100 + trial;
                let (mut world, initial) = native_root(&input, &defs, seed)?;
                let mut policy = Policy::new(e);
                policy.gate_retreat = model != "rules";
                let mut first = None;
                let mut trace = vec![];
                let mut decisions = 0;
                let mut searches = 0;
                let mut nodes = 0;
                let mut depth = 0;
                let mut doors = 0;
                let end = world.battle.round + cfg["round_horizon"].as_u64().unwrap() as u32;
                world.battle.scenario.round_limit = end;
                for step in
                    0..s.transitions_per_round * cfg["round_horizon"].as_u64().unwrap() as usize
                {
                    if world.battle.outcome().is_some()
                        || world.battle.round >= end
                        || decisions
                            >= cfg["max_evaluation_decisions"]
                                .as_u64()
                                .unwrap_or(usize::MAX as u64)
                                as usize
                    {
                        break;
                    }
                    let o = world.observation();
                    let action = if world
                        .battle
                        .actor
                        .is_some_and(|id| world.battle.units[id].friendly())
                    {
                        let mut current = initial.clone();
                        current.observation = o.clone();
                        current.retreat_legal =
                            Some(o.actions.iter().any(|a| a.action == Action::Retreat));
                        let (action, search) = if model == "rules" {
                            (policy.action(e, &world, &current, &s)?, Value::Null)
                        } else {
                            let root = if model == "A" {
                                reconstruct(&current, &Definitions::project(&e.game, &current)?, 0)?
                            } else {
                                world.diagnostic_current_root()
                            };
                            choose(e, &root, &current, &s, cfg, decisions)?
                        };
                        if first.is_none() {
                            first = Some(
                                serde_json::json!({"action":action,"search":search,"public_root":o}),
                            );
                        }
                        searches += search["iterations"].as_u64().unwrap_or(0);
                        nodes += search["nodes"].as_u64().unwrap_or(0);
                        depth = depth.max(search["max_tree_depth"].as_u64().unwrap_or(0));
                        decisions += 1;
                        action
                    } else {
                        world.battle.monster_action()?
                    };
                    let events = world.step(action.clone())?;
                    doors += events
                        .iter()
                        .filter(
                            |e| matches!(e, Event::DeathsDoor(id) if input.heroes.contains_key(id)),
                        )
                        .count() as u32;
                    trace.push(serde_json::json!({"step":step,"round":o.round,"actor":o.actor,"action":action,"events":events,"after":world.observation().units.iter().map(|u|serde_json::json!({"id":u.id,"class":u.class,"hero":u.hero,"hp":u.hp,"stress":u.stress,"rank":u.rank})).collect::<Vec<_>>()}));
                }
                Ok(
                    serde_json::json!({"seed":seed,"result":terminal(&world,&initial,&s,doors),"first":first,"decisions":decisions,"tree_iterations":searches,"tree_nodes":nodes,"max_tree_depth":depth,"trace":trace}),
                )
            })
        });
        trials.push(match result {
            Ok(v) => v,
            Err(error) => serde_json::json!({"error":error,"trial":trial}),
        });
        println!("diagnostic {} {} trial {}", scene["id"], model, trial);
    }
    let probe = if model == "A" {
        Some(execution025::hypothetical026(input.mechanisms, || {
            execution024b::hypothetical026(input.single_target_fallback, || {
                probe(e, &input, &defs, &s)
            })
        }))
    } else {
        None
    };
    let value = serde_json::json!({"version":cfg["version"],"scene":scene,"model":model,"config":cfg,"trials":trials,"mechanism_probe":probe,"seconds":start.elapsed().as_secs_f64(),"scope":"Matched fresh native initialized roots derived from frozen formations/current hero state. A rebuilds from each public observation; B clones only this fresh diagnostic world and resamples RNG/initiative for every tree trajectory. Both execute choices in fresh native worlds. No real campaign state/queue/RNG read by a policy. Enemy formation restarts at full health; not the original saved encounter."});
    let path = out.join(format!("{}-{model}.json", scene["id"].as_str().unwrap()));
    std::fs::write(
        &path,
        serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(
        serde_json::json!({"file":path,"scene":scene["id"],"model":model,"seconds":start.elapsed().as_secs_f64()}),
    )
}
pub fn run(manifest: &Path, out: &Path, jobs: usize) -> Result<()> {
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let cfg: Value = serde_json::from_str(r#"{
  "version": "offline-uct-027-v5-door-events",
  "iterations": 96,
  "tree_depth": 12,
  "round_horizon": 30,
  "eval_trials": 2,
  "uct_exploration": 1.4,
  "score_scale": 10000,
  "per_decision_ms": 250,
  "note": "Open-loop stochastic UCT; complete battle rollouts until terminal or explicit round horizon. Native fresh initialized matched public roots; not actual game executable or saved private battle.",
  "max_evaluation_decisions": 96
}
"#)
        .map_err(|e| e.to_string())?;
    let e = arena::Engine::load()?;
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let tasks = manifest["rows"]
        .as_array()
        .ok_or("rows")?
        .iter()
        .flat_map(|s| ["A", "B", "rules"].map(|m| (s.clone(), m)))
        .filter(|(_, m)| {
            std::env::var("DIAGNOSTIC_MODELS")
                .ok()
                .is_none_or(|selected| selected.split(',').any(|x| x == *m))
        })
        .collect::<Vec<_>>();
    let cursor = AtomicUsize::new(0);
    let results = Mutex::new(vec![]);
    let start = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            let e = &e;
            let tasks = &tasks;
            let cursor = &cursor;
            let results = &results;
            let cfg = &cfg;
            scope.spawn(move || {
                loop {
                    let i = cursor.fetch_add(1, Ordering::Relaxed);
                    let Some((scene, model)) = tasks.get(i) else {
                        break;
                    };
                    let path = out.join(format!("{}-{model}.json", scene["id"].as_str().unwrap()));
                    let r = if path.exists() {
                        Ok(serde_json::json!({"file":path,"existing":true}))
                    } else {
                        case(e, scene, out, cfg, model)
                    };
                    results.lock().unwrap().push(match r {
                        Ok(v) => v,
                        Err(error) => {
                            serde_json::json!({"scene":scene["id"],"model":model,"error":error})
                        }
                    });
                }
            });
        }
    });
    std::fs::write(out.join("summary.json"),serde_json::to_vec_pretty(&serde_json::json!({"results":results.into_inner().unwrap(),"seconds":start.elapsed().as_secs_f64(),"config":cfg})).unwrap()).map_err(|e|e.to_string())
}
