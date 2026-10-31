use dd_core::initial013::{Action, Actor, Input, Session};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
    initial013::Position,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Instant;
fn path(input: &Input, from: &Position, to: &Position) -> Result<Vec<Position>> {
    let mut q = VecDeque::from([from.clone()]);
    let mut prev: BTreeMap<Position, Position> = BTreeMap::new();
    let mut seen = BTreeSet::from([from.clone()]);
    while let Some(p) = q.pop_front() {
        if p == *to {
            let mut out = vec![];
            let mut at = to.clone();
            while at != *from {
                out.push(at.clone());
                at = prev.get(&at).ok_or("path predecessor missing")?.clone();
            }
            out.reverse();
            return Ok(out);
        }
        for n in &input.map.cell(&p)?.neighbours {
            if seen.insert(n.clone()) {
                prev.insert(n.clone(), p.clone());
                q.push_back(n.clone());
            }
        }
    }
    Err("no map route".into())
}
fn checked_action(s: &mut Session, a: Action) -> Result<Option<dd_core::initial013::Interaction>> {
    let mut restored = Session::restore_bound(&s.checkpoint()?, &s.definitions())?;
    let x = s.act(a.clone())?;
    let y = restored.act(a)?;
    if x != y || s.checkpoint()? != restored.checkpoint()? || s.observe()? != restored.observe()? {
        return Err("initial action replay mismatch".into());
    }
    Ok(x)
}
pub fn run(args: &[String]) -> Result<()> {
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: initial-audit|initial-probe <mode> <seeds> [--base-content|--court-loot]"
                .into(),
        );
    }
    let command = &args[0];
    let mode = &args[1];
    let seeds = &args[2];
    let flag = args.get(3).map(String::as_str);
    if ![None, Some("--base-content"), Some("--court-loot")].contains(&flag) {
        return Err("unknown initial option".into());
    }
    let count: u64 = seeds.parse().map_err(|_| "invalid initial seed count")?;
    if count == 0 || count > 100000 {
        return Err("invalid initial seed bound".into());
    }
    let mut cfg = if flag == Some("--base-content") {
        ContentConfig::fixture()
    } else {
        ContentConfig::full().with_enabled_combat_candidates()?
    };
    cfg.mode = mode.clone();
    let g = GameData::load_with_content(&dd_data::game_dir(), cfg)?;
    println!(
        "{} mode={} content={}",
        dd_core::initial013::PROFILE,
        mode,
        g.overlay.config.id()?
    );
    println!(
        "Scope: fixed map/curio host probe; encounters, traps, obstacles and hunger acknowledged by test host, not simulated expedition completion. Actors are unequipped level-zero host probes, not the campaign roster."
    );
    let start = Instant::now();
    let mut actions = 0;
    let mut interactions = 0;
    let mut handoffs = 0;
    for id in ["old_road", "tutorial_crypts"] {
        let input = Input::load_with_court_loot(&g, id, flag == Some("--court-loot"))?;
        let definition = dd_core::initial013::Definition::new(input.clone())?;
        println!(
            "definition_id={} candidates={}",
            definition.id(),
            definition.candidate_id()
        );
        let corridors = input
            .map
            .cells
            .iter()
            .map(|c| c.position.area.clone())
            .filter(|a| !input.map.rooms.contains(a))
            .collect::<BTreeSet<_>>();
        println!(
            "{id}: rooms={} corridors={} tiles={} entrance={:?} layers={:?} difficulty={} dungeon={} torch={}",
            input.map.rooms.len(),
            corridors.len(),
            input.map.cells.len(),
            input.map.entrance,
            input.map.source_layers,
            input.difficulty,
            input.dungeon,
            input.torch
        );
        if command == "initial-audit" {
            for c in &input.map.cells {
                let curio = if c.curio_hash != 0 {
                    let (p, t) = input.curios.prop(c.curio_hash)?;
                    format!("{p}/{}", t.id)
                } else {
                    "-".into()
                };
                println!(
                    "  {}:{} content={} curio={} mash_index={} mash_name={:?} exits={:?}",
                    c.position.area,
                    c.position.tile,
                    c.content,
                    curio,
                    c.mash_index,
                    c.mash_name,
                    c.neighbours
                );
            }
            continue;
        }
        if command != "initial-probe" {
            return Err("unknown initial command".into());
        }
        for seed in 1..=count {
            let actors = vec![
                Actor::fresh(&g, "crusader")?,
                Actor::fresh(&g, "highwayman")?,
            ];
            let mut session = Session::new(
                input.clone(),
                actors,
                BTreeMap::from([
                    ("skeleton_key".into(), 3),
                    ("holy_water".into(), 2),
                    ("shovel".into(), 2),
                ]),
                seed,
            )?;
            let mut rewards = 0;
            let mut negative = 0;
            // Host inspection route deliberately has the complete definition; it is NOT a policy.
            for target in input.map.cells.iter().map(|c| c.position.clone()) {
                for to in path(&input, &session.observe()?.position, &target)? {
                    checked_action(&mut session, Action::Move(to))?;
                    actions += 1;
                    if let Some(h) = session.handoff().cloned() {
                        let mut restored =
                            Session::restore_bound(&session.checkpoint()?, &session.definitions())?;
                        session.resume(&h)?;
                        restored.resume(&h)?;
                        if session.checkpoint()? != restored.checkpoint()? {
                            return Err("initial handoff replay mismatch".into());
                        }
                        handoffs += 1;
                    }
                }
                let c = input.map.cell(&target)?;
                if c.curio_hash != 0 {
                    let (_, curio) = input.curios.prop(c.curio_hash)?;
                    let item = if seed % 2 == 0 {
                        curio
                            .items
                            .keys()
                            .find(|id| session.observe().unwrap().inventory.contains_key(*id))
                            .cloned()
                    } else {
                        None
                    };
                    let r = checked_action(&mut session, Action::Interact { actor: 0, item })?
                        .ok_or("missing curio interaction")?;
                    actions += 1;
                    interactions += 1;
                    rewards += r.rewards.len();
                    negative += usize::from(r.effect.is_some());
                }
            }
            println!(
                "{id} seed={seed} visited={} curios={} reward_stacks={} effect_results={} actor_hp_stress={:?}",
                session.observe()?.visited.len(),
                input.map.cells.iter().filter(|c| c.curio_hash != 0).count(),
                rewards,
                negative,
                session
                    .observe()?
                    .actors
                    .iter()
                    .map(|a| (a.hp, a.stress, a.resolve.clone()))
                    .collect::<Vec<_>>()
            );
        }
    }
    println!(
        "seeds_per_level={count} actions={actions} interactions={interactions} external_handoffs={handoffs} replay=equal"
    );
    println!("elapsed_seconds={:.6}", start.elapsed().as_secs_f64());
    Ok(())
}
