use dd_core::{
    generate014,
    maps014::{self, Action, Definition, Entry, Resolution, Session},
};
use dd_data::{
    catalog014::{Catalog, Route},
    combat::{GameData, Result},
    content::ContentConfig,
    maps014::{Candidates, Map},
    overlay::Overlay,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Instant;
fn catalog(args: &[String]) -> Result<(Catalog, u64, String)> {
    let mut mode = "radiant".to_string();
    let mut preset = "full";
    let mut count = 1u64;
    let mut candidate = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--candidate" => candidate = true,
            "--mode" | "--content" | "--seeds" => {
                let key = &args[i];
                i += 1;
                let value = args.get(i).ok_or("missing map option value")?;
                match key.as_str() {
                    "--mode" => mode = value.clone(),
                    "--content" => preset = value,
                    "--seeds" => count = value.parse().map_err(|_| "invalid map seed count")?,
                    _ => unreachable!(),
                }
            }
            _ => return Err(format!("unknown map option {}", args[i])),
        }
        i += 1;
    }
    if count == 0 || count > 100000 {
        return Err("invalid map seed bound".into());
    }
    let mut cfg = match preset {
        "full" => ContentConfig::full().with_enabled_combat_candidates()?,
        "base" => ContentConfig::fixture(),
        _ => return Err("map content must be base or full".into()),
    };
    cfg.mode = mode;
    let candidates = if candidate {
        Candidates::for_content(&cfg)?
    } else {
        Candidates::strict()
    };
    let overlay = Overlay::load(&dd_data::game_dir(), cfg)?;
    Ok((Catalog::load(&overlay, candidates)?, count, preset.into()))
}
fn stats(m: &Map) -> Value {
    json!({"nodes":m.nodes.len(),"rooms":m.nodes.values().filter(|n|n.room).count(),"hidden_rooms":m.nodes.values().filter(|n|n.room&&n.hidden).count(),"edges":m.nodes.values().map(|n|n.links.len()).sum::<usize>()/2,"goals":m.goals.len(),"gate_markers":m.nodes.values().filter(|n|n.gate.is_some()).count(),"gate_groups":m.gate_groups().map(|v|v.len()).ok(),"key_sources":m.key_sources.len(),"teleport_groups":m.teleport_targets.len(),"source":m.source,"persistent":m.persistent,"visual_ambiguities":m.nodes.iter().filter(|(_,n)|n.metadata["visual_ambiguities"].as_object().is_some_and(|v|!v.is_empty())).map(|(id,n)|(id.clone(),n.metadata["visual_ambiguities"].clone())).collect::<BTreeMap<_,_>>()})
}
fn fixed_audit(c: &Catalog) -> Value {
    let maps = c
        .templates
        .keys()
        .map(|id| {
            (
                id.clone(),
                match c.fixed(id) {
                    Ok(m) => stats(&m),
                    Err(e) => json!({"error":e}),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let tasks = c
        .quests
        .keys()
        .filter_map(|id| match c.route(id) {
            Ok(Route::Fixed(_)) => Some((
                id.clone(),
                match c.fixed_for_quest(id) {
                    Ok(m) => stats(&m),
                    Err(e) => json!({"error":e}),
                },
            )),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    json!({"catalog":c.audit(),"fixed":maps,"fixed_tasks":tasks})
}
fn checked(s: &mut Session, run: impl Fn(&mut Session) -> Result<()>) -> Result<()> {
    let mut other = Session::restore(&s.checkpoint()?, &s.definitions())?;
    run(s)?;
    run(&mut other)?;
    if s.checkpoint()? != other.checkpoint()? || s.observe() != other.observe() {
        return Err("map replay mismatch".into());
    }
    Ok(())
}
fn drain(s: &mut Session, actions: &mut usize, handoffs: &mut usize) -> Result<()> {
    loop {
        if let Some(p) = s.pending().cloned() {
            checked(s, |s| s.resume(&p, Resolution::Settled))?;
            *handoffs += 1;
            continue;
        }
        if s.legal_actions().contains(&Action::Interact) {
            checked(s, |s| s.act(Action::Interact))?;
            *actions += 1;
            continue;
        }
        break;
    }
    Ok(())
}
fn routes(s: &Session, m: &Map) -> BTreeMap<String, Option<String>> {
    let start = s.position().to_string();
    let mut prev = BTreeMap::from([(start.clone(), None)]);
    let mut q = VecDeque::from([start]);
    while let Some(id) = q.pop_front() {
        for l in &m.nodes[&id].links {
            if l.hidden.as_ref().is_some_and(|t| !s.revealed().contains(t))
                || m.nodes[&l.to].gate.is_some() && !s.opened().contains(&l.to)
            {
                continue;
            }
            if !prev.contains_key(&l.to) {
                prev.insert(l.to.clone(), Some(id.clone()));
                q.push_back(l.to.clone());
            }
        }
    }
    prev
}
fn route_to(s: &Session, to: &str, prev: &BTreeMap<String, Option<String>>) -> Result<Vec<String>> {
    let start = s.position().to_string();
    let mut at = to.to_string();
    let mut path = vec![];
    while at != start {
        path.push(at.clone());
        at = prev
            .get(&at)
            .and_then(Clone::clone)
            .ok_or("map planner path missing")?;
    }
    path.reverse();
    Ok(path)
}
fn probe(c: &Catalog, map: Map, seed: u64) -> Result<Value> {
    let map = std::sync::Arc::new(map);
    let d = Definition::new((*map).clone(), c.content.clone(), c.candidates.clone())?;
    let mut s = Session::new(d, seed, BTreeMap::new())?;
    let mut actions = 0;
    let mut handoffs = 0;
    let plan = map.lock_plan()?;
    let mut iterations = 0;
    loop {
        iterations += 1;
        if iterations > 1000000 {
            return Err("map host probe exceeded bound".into());
        }
        drain(&mut s, &mut actions, &mut handoffs)?;
        let at = s.position().to_string();
        let tags = map.nodes[&at]
            .links
            .iter()
            .filter_map(|l| l.hidden.clone())
            .filter(|t| !s.revealed().contains(t))
            .collect::<BTreeSet<_>>();
        if !tags.is_empty() && map.raw["task"]["is_scouting_enabled"].as_bool() != Some(false) {
            let nodes = map.nodes[&at]
                .links
                .iter()
                .filter(|l| l.hidden.as_ref().is_some_and(|t| tags.contains(t)))
                .map(|l| l.to.clone())
                .collect();
            checked(&mut s, |s| s.scout(&nodes, &tags))?;
        }
        let prev = routes(&s, &map);
        let target = prev.keys().find(|id| !s.visited().contains(*id)).cloned();
        if let Some(to) = target {
            for to in route_to(&s, &to, &prev)? {
                checked(&mut s, |s| s.act(Action::Move(to.clone())))?;
                actions += 1;
                drain(&mut s, &mut actions, &mut handoffs)?;
            }
            continue;
        }
        let unlock = plan
            .iter()
            .find(|id| !s.opened().contains(*id))
            .cloned()
            .or_else(|| {
                map.nodes
                    .iter()
                    .find(|(id, n)| {
                        n.gate
                            .as_ref()
                            .is_some_and(|k| s.keys().get(k).is_some_and(|n| *n > 0))
                            && !s.opened().contains(*id)
                            && n.links.iter().any(|l| prev.contains_key(&l.to))
                    })
                    .map(|(id, _)| id.clone())
            });
        if let Some(gate) = unlock {
            let key = map.nodes[&gate]
                .gate
                .as_ref()
                .ok_or("lock plan references non-gate")?;
            if !s.keys().get(key).is_some_and(|n| *n > 0) {
                return Err("map planner lacks source key for required gate".into());
            }
            let near = map.nodes[&gate]
                .links
                .iter()
                .find(|l| prev.contains_key(&l.to))
                .map(|l| l.to.clone())
                .ok_or("required map gate not reachable")?;
            for to in route_to(&s, &near, &prev)? {
                checked(&mut s, |s| s.act(Action::Move(to.clone())))?;
                actions += 1;
                drain(&mut s, &mut actions, &mut handoffs)?;
            }
            checked(&mut s, |s| s.act(Action::Unlock(gate.clone())))?;
            actions += 1;
            continue;
        }
        break;
    }
    if !map.goals.is_subset(s.visited()) {
        return Err(format!("map probe missed goal in {}", map.id));
    }
    let snapshot = s.checkpoint()?;
    let visited = s.visited().len();
    if map.persistent {
        let restored = Session::resume_campaign_map(&snapshot, &s.definitions())?;
        if restored.checkpoint() != Ok(snapshot.clone()) {
            return Err("persistent map resume mismatch".into());
        }
    }
    let mut teleports = 0;
    for (tag, targets) in &map.teleport_targets {
        checked(&mut s, |s| s.teleport(tag))?;
        if !targets.contains(s.position()) {
            return Err("map teleport selected a foreign room".into());
        }
        teleports += 1;
        drain(&mut s, &mut actions, &mut handoffs)?;
    }
    let mut replay = s.clone();
    checked(&mut replay, Session::mark_complete)?;
    if map.persistent
        && Session::resume_campaign_map(&replay.checkpoint()?, &replay.definitions()).is_ok()
    {
        return Err("completed campaign map incorrectly resumed".into());
    }
    Ok(
        json!({"id":map.id,"seed":seed,"nodes":map.nodes.len(),"visited":visited,"closed_optional_nodes":map.nodes.len()-visited,"goals":map.goals.len(),"all_goals_reached":true,"actions":actions,"external_handoffs":handoffs,"restore":"equal","teleport_groups_exercised":teleports,"persistent":map.persistent}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (c, count, preset) = catalog(args)?;
    let start = Instant::now();
    let command = args.first().ok_or("missing map command")?;
    match command.as_str() {
        "map-audit" => println!(
            "{}",
            serde_json::to_string_pretty(&fixed_audit(&c)).map_err(|e| e.to_string())?
        ),
        "map-matrix" => {
            let mut fixed = vec![];
            for id in c.templates.keys() {
                let m = c.fixed(id)?;
                fixed.push((id.clone(), stats(&m)));
            }
            let mut generated = vec![];
            for key in c.rules.keys() {
                if key.dungeon == "courtyard"
                    && !c.content.packages.iter().any(|p| p == "crimson_court")
                {
                    continue;
                }
                let rule = c.rule(key)?;
                let mut min = BTreeMap::<String, u32>::new();
                let mut max = BTreeMap::<String, u32>::new();
                let mut degrees = BTreeMap::<usize, u64>::new();
                let mut distances = BTreeMap::<usize, u64>::new();
                let mut attempts = BTreeMap::<u32, u64>::new();
                use sha2::{Digest, Sha256};
                let mut digest = Sha256::new();
                for seed in 1..=count {
                    let m = generate014::generate(&rule, seed, &c.candidates)
                        .map_err(|e| format!("{} seed={seed}: {e}", key.id()))?;
                    let g = m.generation.as_ref().unwrap();
                    for (k, v) in &g.final_counts {
                        min.entry(k.clone())
                            .and_modify(|n| *n = (*n).min(*v))
                            .or_insert(*v);
                        max.entry(k.clone())
                            .and_modify(|n| *n = (*n).max(*v))
                            .or_insert(*v);
                    }
                    for n in m.nodes.values().filter(|n| n.room && !n.hidden) {
                        *degrees.entry(n.links.len()).or_default() += 1;
                    }
                    *distances.entry(g.final_distance).or_default() += 1;
                    *attempts.entry(g.topology_attempts).or_default() += 1;
                    digest.update(serde_json::to_vec(&m).map_err(|e| e.to_string())?);
                }
                generated.push(json!({"selector":key.id(),"seeds":count,"source":rule.sources,"source_parameters":rule.raw,"minimum_counts":min,"maximum_counts":max,"room_degree_histogram":degrees,"final_distance_histogram":distances,"topology_attempt_histogram":attempts,"sequence_sha256":format!("{:x}",digest.finalize())}));
            }
            let mut tasks = vec![];
            let mut farm = None;
            for id in c.quests.keys() {
                match c.route(id)? {
                    Route::Disabled => tasks.push(json!({"quest":id,"role":"disabled"})),
                    _ => match maps014::quest_entry(&c, id, 1)? {
                        Entry::Map(m) => tasks.push(json!({"quest":id,"map":stats(&m)})),
                        Entry::Farm(entry) => {
                            if farm.is_none() {
                                farm = Some(GameData::load_with_content(
                                    &dd_data::game_dir(),
                                    c.content.clone(),
                                )?);
                            }
                            let eligible = c
                                .quests
                                .keys()
                                .filter(|id| !matches!(c.route(id), Ok(Route::Disabled)))
                                .cloned()
                                .collect();
                            let plan = entry.battle_plan(farm.as_ref().unwrap(), eligible)?;
                            plan.validate()?;
                            if plan.id != entry.profile {
                                return Err("farm profile handoff mismatch".into());
                            }
                            tasks.push(json!({"quest":id,"farm_profile":plan.id,"existing_wave_plan":"valid"}));
                        }
                    },
                }
            }
            let mut goal_bindings = vec![];
            for key in c.rules.keys() {
                if key.dungeon == "courtyard"
                    && !c.content.packages.iter().any(|p| p == "crimson_court")
                {
                    continue;
                }
                for goals in c.generated_goals(key)? {
                    if goals.is_empty() {
                        continue;
                    }
                    let map = maps014::random_entry(&c, key, &goals, 1)?;
                    goal_bindings
                        .push(json!({"selector":key.id(),"goals":goals,"map":stats(&map)}));
                }
            }
            let repeat = if c.candidates.court_task_aliases {
                ["farm_bloodcrate", "kill_statue"]
                    .iter()
                    .map(|kind| {
                        maps014::repeat_entry(&c, kind, 1)
                            .map(|m| json!({"kind":kind,"map":stats(&m)}))
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![]
            };
            println!("{}",serde_json::to_string_pretty(&json!({"profile":dd_data::maps014::PROFILE,"content":preset,"mode":c.content.mode,"catalog":c.audit(),"fixed":fixed,"generated":generated,"quests":tasks,"repeat":repeat,"goal_bindings":goal_bindings,"failures":0})).map_err(|e|e.to_string())?);
        }
        "map-probe" => {
            let mut rows = vec![];
            for seed in 1..=count {
                for id in c.templates.keys() {
                    rows.push(probe(&c, c.fixed(id)?, seed)?);
                }
                for id in c.quests.keys() {
                    if matches!(c.route(id), Ok(Route::Fixed(_))) {
                        rows.push(probe(&c, c.fixed_for_quest(id)?, seed)?);
                    }
                }
                for key in c.rules.keys() {
                    if key.dungeon == "courtyard"
                        && !c.content.packages.iter().any(|p| p == "crimson_court")
                    {
                        continue;
                    }
                    rows.push(probe(
                        &c,
                        generate014::generate(&c.rule(key)?, seed, &c.candidates)?,
                        seed,
                    )?);
                }
                if c.candidates.court_task_aliases {
                    for kind in ["farm_bloodcrate", "kill_statue"] {
                        rows.push(probe(&c, maps014::repeat_entry(&c, kind, seed)?, seed)?);
                    }
                }
            }
            println!("{}",serde_json::to_string_pretty(&json!({"profile":dd_data::maps014::PROFILE,"mode":c.content.mode,"content":preset,"note":"trusted host geometry/progress probe; encounter and curio settlements acknowledged, not natural expedition completion","seeds":count,"rows":rows,"failures":0})).map_err(|e|e.to_string())?);
        }
        _ => return Err("unknown map command".into()),
    }
    eprintln!("elapsed_seconds={:.6}", start.elapsed().as_secs_f64());
    Ok(())
}
