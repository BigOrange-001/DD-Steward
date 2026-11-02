use dd_core::{Baseline, Battle, Event, Outcome, PROFILE, Policy, formation_scenario};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
};
use std::time::Instant;

pub fn run(command: &str, args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("formation experiments require --candidate".into());
    }
    let mut config = ContentConfig::fixture();
    let mut selected = false;
    let mut numbers = vec![];
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--content" {
            if selected {
                return Err("duplicate --content".into());
            }
            selected = true;
            i += 1;
            config = match args.get(i).map(String::as_str) {
                Some("fixture") => ContentConfig::fixture(),
                Some("full") => ContentConfig::full(),
                _ => return Err("expected --content fixture|full".into()),
            };
        } else {
            numbers.push(
                args[i]
                    .parse::<u64>()
                    .map_err(|_| "invalid formation experiment integer")?,
            );
        }
        i += 1;
    }
    let tracing = command == "formation-demo";
    if numbers.len() > if tracing { 1 } else { 2 } {
        return Err("too many formation experiment arguments".into());
    }
    let count = if tracing {
        1
    } else {
        *numbers.first().unwrap_or(&1000)
    };
    let seed = *numbers.get(if tracing { 0 } else { 1 }).unwrap_or(&1);
    if !(1..=1_000_000).contains(&count) {
        return Err("battle count must be 1..1000000".into());
    }
    let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
    if let Some(error) = &g.combat_globals_error {
        return Err(error.clone());
    }
    let scenario = formation_scenario(&g)?;
    println!(
        "EXPERIMENTAL {PROFILE}: whole-block formation and single-action body sizes; uncalibrated."
    );
    println!("Content ID: {}", g.overlay.config.id()?);
    println!(
        "Scenario: highwayman/leper/bounty_hunter/vestal (equipment 4, skill level 4) vs skeleton_common_A + templar_ranged_D (size 2) + skeleton_courtier_A; round limit 100."
    );
    println!("initial_hero_stress=0 checkpoint_every_step={tracing}");
    let start = Instant::now();
    let mut policy = Baseline;
    let (mut wins, mut losses, mut draws, mut retreats, mut mutual) = (0, 0, 0, 0, 0);
    let (mut actions, mut moves, mut large_damage, mut large_deaths, mut large_corpses) =
        (0u64, 0, 0, 0, 0);
    for offset in 0..count {
        let mut b = Battle::new(scenario.clone(), seed.wrapping_add(offset))?;
        while b.outcome().is_none() {
            let o = b.observation();
            let hero = o.units.iter().any(|u| Some(u.id) == o.actor && u.hero);
            let events = if hero {
                let action = policy.choose(&o).ok_or("no policy action")?;
                if tracing {
                    println!("round {} actor {:?}: {action:?}", o.round, o.actor);
                }
                b.step(action)?
            } else {
                if tracing {
                    println!("round {} actor {:?}: monster brain", o.round, o.actor);
                }
                b.step_monster()?
            };
            for event in &events {
                match event {
                    Event::Move { .. } => moves += 1,
                    Event::Damage { target: 5, .. } => large_damage += 1,
                    Event::Death(5) => large_deaths += 1,
                    Event::Corpse(5) => large_corpses += 1,
                    _ => {}
                }
            }
            if tracing {
                println!("  {events:?}");
                println!(
                    "  monster_formation={:?}",
                    b.observation()
                        .units
                        .iter()
                        .filter(|u| !u.hero)
                        .map(|u| (u.id, u.occupied_ranks.clone(), u.corpse))
                        .collect::<Vec<_>>()
                );
                if Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)? != b {
                    return Err("checkpoint replay mismatch".into());
                }
            }
            actions += 1;
        }
        match b.outcome().unwrap() {
            Outcome::HeroesWin => wins += 1,
            Outcome::MonstersWin => losses += 1,
            Outcome::RoundLimit => draws += 1,
            Outcome::Retreated => retreats += 1,
            Outcome::MutualDefeat => mutual += 1,
            Outcome::Teleported | Outcome::FlashbackFailed | Outcome::WaveInterlude => {
                return Err("unexpected relocation in legacy scene".into());
            }
            Outcome::ResolutionLimit => return Err("reaction resolution limit".into()),
        }
        if tracing {
            println!(
                "Outcome: {:?}; every-step checkpoint round-trip verified",
                b.outcome()
            );
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "moves={moves} large_damage_events={large_damage} large_death_events={large_deaths} large_corpse_events={large_corpses}"
    );
    println!(
        "battles={count} seed={seed} hero_wins={wins} losses={losses} round_limits={draws} retreats={retreats} mutual_defeats={mutual} actions={actions}"
    );
    println!(
        "elapsed_seconds={elapsed:.6} battles_per_second={:.2} actions_per_second={:.2}",
        count as f64 / elapsed,
        actions as f64 / elapsed
    );
    Ok(())
}

