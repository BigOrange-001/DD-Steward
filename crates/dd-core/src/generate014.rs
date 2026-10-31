//! Explicit path-growth-budget-v1 candidate; no claim of original topology distribution.
use super::Rng;
use dd_data::{
    combat::Result,
    generation014::{GenerationRecord, HALL_COUNTS, ROOM_COUNTS, Rule},
    maps014::*,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
fn shuffle<T>(rng: &mut Rng, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = rng.below((i + 1) as u64) as usize;
        v.swap(i, j);
    }
}
fn path(rng: &mut Rng, grid: [usize; 2], n: usize) -> Result<Vec<[usize; 2]>> {
    fn extend(
        rng: &mut Rng,
        grid: [usize; 2],
        n: usize,
        path: &mut Vec<[usize; 2]>,
        seen: &mut BTreeSet<[usize; 2]>,
        budget: &mut usize,
    ) -> bool {
        if path.len() == n {
            return true;
        }
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        let [x, y] = *path.last().unwrap();
        let mut next = vec![];
        if x > 0 {
            next.push([x - 1, y]);
        }
        if y > 0 {
            next.push([x, y - 1]);
        }
        if x + 1 < grid[0] {
            next.push([x + 1, y]);
        }
        if y + 1 < grid[1] {
            next.push([x, y + 1]);
        }
        shuffle(rng, &mut next);
        for p in next {
            if !seen.insert(p) {
                continue;
            }
            path.push(p);
            if extend(rng, grid, n, path, seen, budget) {
                return true;
            }
            path.pop();
            seen.remove(&p);
        }
        false
    }
    let p = [
        rng.below(grid[0] as u64) as usize,
        rng.below(grid[1] as u64) as usize,
    ];
    let mut path = vec![p];
    let mut seen = BTreeSet::from([p]);
    if !extend(rng, grid, n, &mut path, &mut seen, &mut 1000000) {
        return Err("candidate map path search exhausted (no reduced map returned)".into());
    }
    Ok(path)
}
fn draw(rng: &mut Rng, r: [u32; 2]) -> u32 {
    r[0] + rng.below(u64::from(r[1] - r[0]) + 1) as u32
}
fn room_dist(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut d = vec![usize::MAX; n];
    d[0] = 0;
    let mut q = std::collections::VecDeque::from([0]);
    while let Some(a) = q.pop_front() {
        for &(x, y) in edges {
            let b = if x == a {
                y
            } else if y == a {
                x
            } else {
                continue;
            };
            if d[b] == usize::MAX {
                d[b] = d[a] + 1;
                q.push_back(b);
            }
        }
    }
    d
}
fn counts(map: &Map) -> BTreeMap<String, u32> {
    let mut c = ROOM_COUNTS
        .iter()
        .chain(HALL_COUNTS)
        .copied()
        .chain([
            "secret_rooms",
            "prisoner_rooms",
            "objective_curios",
            "total_room_battles",
        ])
        .map(|k| (k.to_string(), 0))
        .collect::<BTreeMap<_, u32>>();
    for n in map.nodes.values() {
        let key = if n.metadata["objective_only_curio"].as_bool() == Some(true) {
            "objective_curios"
        } else if n.room {
            if n.hidden {
                if n.content.curio.as_deref() == Some("prisoner") {
                    "prisoner_rooms"
                } else {
                    "secret_rooms"
                }
            } else if n.content.battle.is_some()
                && n.content.curio.as_deref() == Some("pool:treasure")
            {
                "room_guarded_treasure"
            } else if n.content.battle.is_some() && n.content.curio.is_some() {
                "room_guarded_curio"
            } else if n.content.battle.is_some() {
                "room_battle"
            } else if n.content.curio.as_deref() == Some("pool:treasure") {
                "room_treasure"
            } else if n.content.curio.is_some() {
                "room_curio"
            } else {
                continue;
            }
        } else if n.content.battle.is_some() {
            "hallway_battle"
        } else if n.content.curio.is_some() {
            "hallway_curio"
        } else if n.content.trap.is_some() {
            "hallway_trap"
        } else if n.content.obstacle.is_some() {
            "hallway_obstacle"
        } else if n.content.hunger {
            "hallway_hunger"
        } else {
            continue;
        };
        *c.entry(key.into()).or_default() += 1;
    }
    c.insert(
        "total_room_battles".into(),
        map.nodes
            .values()
            .filter(|n| n.room && !n.hidden && n.content.battle.is_some())
            .count() as u32,
    );
    c
}
fn generate_attempt(rule: &Rule, seed: u64, candidates: &Candidates) -> Result<Map> {
    rule.validate()?;
    candidates.validate()?;
    if candidates.algorithm.as_deref() != Some("path-growth-budget-v1") {
        return Err("random map needs explicit candidate algorithm".into());
    }
    let mut rng = Rng(seed);
    let n = rule.goal_rooms;
    let rooms = path(&mut rng, rule.grid, n)?;
    let mut edges = (1..n).map(|i| (i - 1, i)).collect::<Vec<_>>();
    let mut possible = vec![];
    for a in 0..n {
        for b in a + 1..n {
            if rooms[a][0].abs_diff(rooms[b][0]) + rooms[a][1].abs_diff(rooms[b][1]) == 1
                && !edges.contains(&(a, b))
            {
                possible.push((a, b));
            }
        }
    }
    shuffle(&mut rng, &mut possible);
    let mut attempts = 0;
    for e in possible.into_iter().take(rule.corridor_budget) {
        attempts += 1;
        if rng.roll(rule.connectivity) {
            edges.push(e);
            if room_dist(n, &edges).iter().copied().max().unwrap() < rule.min_final_distance {
                edges.pop();
            }
        }
    }
    let distances = room_dist(n, &edges);
    let eligible = distances
        .iter()
        .enumerate()
        .filter(|(_, d)| **d >= rule.min_final_distance)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let final_room = eligible[rng.below(eligible.len() as u64) as usize];
    let mut map = Map {
        id: format!("generated:{}:{seed}", rule.key.id()),
        region: rule.key.dungeon.clone(),
        source: Source {
            path: "scripts/map_generator.darkest".into(),
            layers: rule.sources.iter().flat_map(|s| s.layers.clone()).collect(),
        },
        raw: json!({"rule":rule,"seed":seed,"final_anchor":format!("room{final_room}")}),
        nodes: BTreeMap::new(),
        entrance: "room0".into(),
        goals: BTreeSet::new(),
        fog: rule.key.dungeon == "courtyard",
        persistent: false,
        key_sources: BTreeMap::new(),
        teleport_targets: BTreeMap::new(),
        generation: None,
    };
    for (i, [x, y]) in rooms.iter().copied().enumerate() {
        let amplitude = f64::from(draw(&mut rng, rule.nudge));
        let random = |rng: &mut Rng| ((rng.next() >> 11) as f64 / ((1u64 << 53) as f64)) * 2. - 1.;
        let parent = if i > 0 { rooms[i - 1] } else { rooms[0] };
        let displacement = [x as f64 - parent[0] as f64, y as f64 - parent[1] as f64];
        let centre = [
            (rule.grid[0] - 1) as f64 / 2. - x as f64,
            (rule.grid[1] - 1) as f64 / 2. - y as f64,
        ];
        let mut p = [
            x as f64 * rule.spacing as f64,
            y as f64 * rule.spacing as f64,
        ];
        for axis in 0..2 {
            let bias = centre[axis] * rule.centre_bias + displacement[axis] * rule.neighbour_bias;
            p[axis] += amplitude * (random(&mut rng) + bias) / (1. + bias.abs());
        }
        let id = format!("room{i}");
        map.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                area: id,
                room: true,
                position: p,
                hidden: false,
                tags: BTreeSet::new(),
                content: Content::default(),
                gate: None,
                teleport: None,
                links: vec![],
                metadata: json!({"grid":[x,y]}),
            },
        );
    }
    let mut corridor_nodes = vec![];
    for (i, (a, b)) in edges.iter().copied().enumerate() {
        let [ax, ay] = rooms[a];
        let [bx, by] = rooms[b];
        let distance = (ax.abs_diff(bx) + ay.abs_diff(by)) * rule.spacing;
        let mut previous = format!("room{a}");
        for t in 1..distance {
            let id = format!("corridor{i}:{t}");
            let f = t as f64 / distance as f64;
            let p = [
                (ax as f64 + (bx as f64 - ax as f64) * f) * rule.spacing as f64,
                (ay as f64 + (by as f64 - ay as f64) * f) * rule.spacing as f64,
            ];
            map.nodes.insert(
                id.clone(),
                Node {
                    id: id.clone(),
                    area: format!("corridor{i}"),
                    room: false,
                    position: p,
                    hidden: false,
                    tags: BTreeSet::new(),
                    content: Content::default(),
                    gate: None,
                    teleport: None,
                    links: vec![],
                    metadata: json!({}),
                },
            );
            map.link(&previous, &id, None)?;
            previous = id.clone();
            corridor_nodes.push(id);
        }
        map.link(&previous, &format!("room{b}"), None)?;
    }
    // The source room_* ranges describe the pre-placement phase in this
    // approved candidate. Then total_room_battles fills ordinary empty rooms.
    let mut preplaced = BTreeMap::new();
    let mut room_pool = (1..n).map(|i| format!("room{i}")).collect::<Vec<_>>();
    shuffle(&mut rng, &mut room_pool);
    let mut chosen = None;
    for _ in 0..10000 {
        let values = ROOM_COUNTS
            .iter()
            .map(|k| ((*k).to_string(), draw(&mut rng, rule.counts[*k])))
            .collect::<BTreeMap<_, _>>();
        let seeded = values.values().sum::<u32>();
        let battles =
            values["room_battle"] + values["room_guarded_curio"] + values["room_guarded_treasure"];
        let min = rule.counts["total_room_battles"][0].max(battles);
        if seeded >= n as u32 {
            continue;
        }
        let max = rule.counts["total_room_battles"][1].min((n - 1) as u32 - (seeded - battles));
        if seeded < (n as u32) && min <= max {
            let total = draw(&mut rng, [min, max]);
            chosen = Some((values, total));
            break;
        }
    }
    let (initial, total) = chosen.ok_or("infeasible source room content ranges (not reduced)")?;
    for k in ROOM_COUNTS {
        let amount = initial[*k];
        preplaced.insert((*k).into(), amount);
        for _ in 0..amount {
            let id = room_pool.pop().ok_or("room content capacity exhausted")?;
            let c = &mut map.nodes.get_mut(&id).unwrap().content;
            match *k {
                "room_battle" => c.battle = Some("pool:room".into()),
                "room_guarded_curio" => {
                    c.battle = Some("pool:room".into());
                    c.curio = Some("pool:room_curio".into());
                }
                "room_curio" => c.curio = Some("pool:room_curio".into()),
                "room_guarded_treasure" => {
                    c.battle = Some("pool:room".into());
                    c.curio = Some("pool:treasure".into());
                }
                "room_treasure" => c.curio = Some("pool:treasure".into()),
                _ => unreachable!(),
            }
        }
    }
    let seeded_battles =
        initial["room_battle"] + initial["room_guarded_curio"] + initial["room_guarded_treasure"];
    for _ in seeded_battles..total {
        let id = room_pool
            .pop()
            .ok_or("total room battle capacity exhausted")?;
        map.nodes.get_mut(&id).unwrap().content.battle = Some("pool:room".into());
    }

    shuffle(&mut rng, &mut corridor_nodes);
    let min_required = HALL_COUNTS
        .iter()
        .map(|k| u64::from(rule.counts[*k][0]))
        .sum::<u64>();
    if min_required > corridor_nodes.len() as u64 {
        return Err("sample rejected: corridor capacity below source minimum".into());
    }
    let mut hall = None;
    for _ in 0..10000 {
        let sampled = HALL_COUNTS
            .iter()
            .map(|k| ((*k).to_string(), draw(&mut rng, rule.counts[*k])))
            .collect::<BTreeMap<_, _>>();
        if sampled.values().map(|v| u64::from(*v)).sum::<u64>() <= corridor_nodes.len() as u64 {
            hall = Some(sampled);
            break;
        }
    }
    let hall = hall.ok_or("sample rejected: joint hallway count draw exhausted")?;
    for k in HALL_COUNTS {
        let amount = hall[*k];
        preplaced.insert((*k).into(), amount);
        for _ in 0..amount {
            let id = corridor_nodes
                .pop()
                .ok_or("hallway content accounting error")?;
            let c = &mut map.nodes.get_mut(&id).unwrap().content;
            match *k {
                "hallway_battle" => c.battle = Some("pool:hallway".into()),
                "hallway_curio" => c.curio = Some("pool:hallway_curio".into()),
                "hallway_trap" => c.trap = Some("pool:trap".into()),
                "hallway_obstacle" => c.obstacle = Some("pool:obstacle".into()),
                "hallway_hunger" => c.hunger = true,
                _ => unreachable!(),
            }
        }
    }
    let secrets = draw(&mut rng, rule.counts["secret_rooms"]);
    preplaced.insert("secret_rooms".into(), secrets);
    // Independent Bernoulli candidate, not a replacement for source secret_rooms.
    let prisoner = rule.prisoner.is_some_and(|p| rng.roll(p));
    preplaced.insert("prisoner_rooms".into(), u32::from(prisoner));
    for i in 0..secrets + u32::from(prisoner) {
        let possible = map
            .nodes
            .iter()
            .filter(|(_, n)| !n.room)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if possible.is_empty() {
            return Err("secret room lacks a corridor attachment".into());
        }
        let from = &possible[rng.below(possible.len() as u64) as usize];
        let tag = format!("secret_room_{i}");
        let id = format!("secret{i}");
        let position = map.nodes[from].position;
        let prisoner = i == secrets;
        map.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                area: id.clone(),
                room: true,
                position: [position[0], position[1] + 1.],
                hidden: true,
                tags: BTreeSet::from([tag.clone()]),
                content: Content {
                    curio: Some(if prisoner {
                        "prisoner".into()
                    } else {
                        "pool:secret_treasure".into()
                    }),
                    ..Content::default()
                },
                gate: None,
                teleport: None,
                links: vec![],
                metadata: json!({}),
            },
        );
        map.link(from, &id, Some(tag))?;
    }
    let anchor = format!("room{final_room}");
    match rule.key.quest.as_str() {
        "explore" => map.goals.extend((0..n).map(|i| format!("room{i}"))),
        "cleanse" => map.goals.extend(
            map.nodes
                .iter()
                .filter(|(_, n)| n.room && n.content.battle.is_some())
                .map(|(id, _)| id.clone()),
        ),
        _ => {
            map.goals.insert(anchor);
        }
    }
    let final_counts = counts(&map);
    map.generation = Some(GenerationRecord {
        topology_attempts: 1,
        candidate_seed: seed,
        rule: rule.clone(),
        seed,
        preplaced,
        final_counts,
        room_graph_edges: edges.len(),
        final_distance: distances[final_room],
        corridor_attempts: attempts,
        algorithm: "path-growth-budget-v1".into(),
    });
    map.validate()?;
    validate_generated(&map)?;
    Ok(map)
}
pub fn validate_generated(map: &Map) -> Result<()> {
    let g = map.generation.as_ref().ok_or("not a generated map")?;
    g.rule.validate()?;
    if g.preplaced.get("prisoner_rooms") != g.final_counts.get("prisoner_rooms")
        || g.algorithm != "path-growth-budget-v1"
        || g.final_counts != counts(map)
        || g.corridor_attempts > g.rule.corridor_budget
        || g.final_distance < g.rule.min_final_distance
    {
        return Err("invalid generated map ledger".into());
    }
    if map.nodes.values().filter(|n| n.room && !n.hidden).count() != g.rule.goal_rooms {
        return Err("generated room count mismatch".into());
    }
    for k in ROOM_COUNTS
        .iter()
        .chain(HALL_COUNTS)
        .chain([&"secret_rooms"])
    {
        let n = *g.preplaced.get(*k).ok_or("missing preplacement ledger")?;
        let r = g.rule.counts[*k];
        if !(r[0]..=r[1]).contains(&n) {
            return Err(format!("generated count outside source {k}"));
        }
        if !ROOM_COUNTS.contains(k) && g.final_counts.get(*k).copied().unwrap_or(0) != n {
            return Err("generated non-room content mismatch".into());
        }
    }
    for k in ROOM_COUNTS {
        let actual = g.final_counts[*k];
        let placed = g.preplaced[*k];
        if (actual != placed && *k != "room_battle") || actual < placed {
            return Err("generated room categories differ from placement ledger".into());
        }
    }
    let anchor = map.raw["final_anchor"]
        .as_str()
        .ok_or("missing generated final anchor")?;
    if map.distances(&map.entrance, true).get(anchor).copied()
        != Some(g.final_distance * g.rule.spacing)
        || map.nodes.values().map(|n| n.links.len()).sum::<usize>() / 2
            != g.room_graph_edges + map.nodes.values().filter(|n| !n.room || n.hidden).count()
    {
        return Err("generated graph differs from distance/edge ledger".into());
    }
    let total = g.final_counts["total_room_battles"];
    let r = g.rule.counts["total_room_battles"];
    if !(r[0]..=r[1]).contains(&total) {
        return Err("generated room battles outside source".into());
    }
    Ok(())
}

pub fn refresh_ledger(map: &mut Map) -> Result<()> {
    let final_counts = counts(map);
    map.generation.as_mut().ok_or("not generated")?.final_counts = final_counts;
    validate_generated(map)
}

pub fn generate(rule: &Rule, seed: u64, candidates: &Candidates) -> Result<Map> {
    rule.validate()?;
    candidates.validate()?;
    if candidates.algorithm.as_deref() != Some("path-growth-budget-v1") {
        return Err("random map needs explicit candidate algorithm".into());
    }
    let mut trials = Rng(seed);
    for attempt in 0..256u32 {
        let candidate_seed = if attempt == 0 { seed } else { trials.next() };
        match generate_attempt(rule, candidate_seed, candidates) {
            Ok(mut m) => {
                m.id = format!("generated:{}:{seed}", rule.key.id());
                m.raw["request_seed"] = json!(seed);
                m.raw["topology_attempts"] = json!(attempt + 1);
                let g = m.generation.as_mut().unwrap();
                g.seed = seed;
                g.candidate_seed = candidate_seed;
                g.topology_attempts = attempt + 1;
                return Ok(m);
            }
            Err(e) if e.starts_with("sample rejected:") => continue,
            Err(e) => return Err(e),
        }
    }
    Err("candidate topology sampling exhausted; no source count reduced".into())
}
