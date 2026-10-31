use dd_core::{
    Battle, LifecycleBaseline, LifecycleCounts, Outcome, PROFILE, Policy, lifecycle_scenario,
};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
};
use std::time::Instant;

pub fn run(command: &str, args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("lifecycle experiments require --candidate".into());
    }
    let mut config = ContentConfig::fixture();
    let mut scene = "waves";
    let mut flags = std::collections::BTreeSet::new();
    let mut numbers = vec![];
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            flag @ ("--content" | "--scene") => {
                if !flags.insert(flag) {
                    return Err(format!("duplicate {flag}"));
                }
                i += 1;
                let value = args.get(i).ok_or("missing experiment option")?.as_str();
                if flag == "--content" {
                    config = match value {
                        "fixture" => ContentConfig::fixture(),
                        "full" => ContentConfig::full(),
                        _ => return Err("unknown content preset".into()),
                    };
                } else {
                    scene = value;
                }
            }
            value => numbers.push(
                value
                    .parse::<u64>()
                    .map_err(|_| "invalid experiment integer")?,
            ),
        }
        i += 1;
    }
    let tracing = command == "lifecycle-demo";
    if numbers.len() > if tracing { 1 } else { 2 } {
        return Err("too many experiment integers".into());
    }
    let count = if tracing {
        1
    } else {
        *numbers.first().unwrap_or(&1000)
    };
    let seed = *numbers.get(if tracing { 0 } else { 1 }).unwrap_or(&1);
    if !(1..=1000000).contains(&count) {
        return Err("invalid battle count".into());
    }
    let loading = Instant::now();
    let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
    let (scenario, catalog, waves) = lifecycle_scenario(&g, scene)?;
    let load_seconds = loading.elapsed().as_secs_f64();
    println!("EXPERIMENTAL {PROFILE}: lifecycle scheduling and lifecycle candidate; scene={scene}");
    println!("Content ID: {}", g.overlay.config.id()?);
    println!("Original equipment/skill level=4 initial_stress=0 checkpoint_every_step={tracing}");
    let start = Instant::now();
    let mut results = [0u64; 5];
    let mut actions = 0;
    let mut counts = LifecycleCounts::default();
    for offset in 0..count {
        let mut b = Battle::new_encounter(
            scenario.clone(),
            seed.wrapping_add(offset),
            catalog.clone(),
            waves.clone(),
        )?;
        counts.initial(&b.observation());
        if scene == "rally" {
            let actor = scenario.heroes.len() + 1;
            let events = b.apply_encounter_effect(actor, actor, g.effect("On Death Swap")?)?;
            counts.phase(&events);
            if tracing {
                println!(
                    "Host phase uses original On Death Swap on a parsed corpse; full bearer AI/aura is outside this probe: {events:?}"
                );
                if Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)? != b {
                    return Err("phase checkpoint mismatch".into());
                }
            }
        }
        if scene == "transform" {
            let id = scenario.heroes.len();
            let events = b.apply_encounter_effect(id, id, g.effect("xform to farmhand_A")?)?;
            counts.phase(&events);
            if tracing {
                println!(
                    "Host phase uses original xform to farmhand_A; cocoon mode/Boss encounter is not simulated: {events:?}"
                );
                if Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)? != b {
                    return Err("phase checkpoint mismatch".into());
                }
            }
        }
        while b.outcome().is_none() {
            let before = b.observation();
            let hero = before
                .units
                .iter()
                .any(|u| Some(u.id) == before.actor && u.hero);
            let action = if hero {
                Some(
                    LifecycleBaseline
                        .choose(&before)
                        .ok_or("no policy action")?,
                )
            } else {
                None
            };
            let result = if let Some(a) = &action {
                b.step(a.clone())
            } else {
                b.step_monster()
            };
            let events = result.map_err(|error| {
                format!(
                    "{scene} seed={} round={} actor={:?}: {error}",
                    seed.wrapping_add(offset),
                    before.round,
                    before.actor
                )
            })?;
            counts.record(&before, &events);
            if tracing {
                println!(
                    "round={} actor={:?} action={action:?}: {events:?}",
                    before.round, before.actor
                );
                if Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)? != b {
                    return Err("checkpoint mismatch".into());
                }
            }
            actions += 1;
        }
        let index = match b.outcome().unwrap() {
            Outcome::HeroesWin => 0,
            Outcome::MonstersWin => 1,
            Outcome::RoundLimit => 2,
            Outcome::Retreated => 3,
            Outcome::MutualDefeat => 4,
            Outcome::Teleported | Outcome::FlashbackFailed | Outcome::WaveInterlude => {
                return Err("unexpected relocation in legacy scene".into());
            }
            Outcome::ResolutionLimit => return Err("reaction limit".into()),
        };
        results[index] += 1;
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("mechanisms={counts:?}");
    println!(
        "battles={count} seed={seed} outcomes[win,loss,limit,retreat,mutual]={results:?} actions={actions}"
    );
    println!(
        "load_seconds={load_seconds:.6} elapsed_seconds={elapsed:.6} battles_per_second={:.2} actions_per_second={:.2}",
        count as f64 / elapsed,
        actions as f64 / elapsed
    );
    Ok(())
}
