use dd_core::{Battle, Event, Outcome, PROFILE, Policy, StealthBaseline, stealth_scenario};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
    items::SupplyStack,
};
use std::time::Instant;

pub fn run(command: &str, args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("stealth experiments require --candidate".into());
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
                    .map_err(|_| "invalid stealth experiment integer")?,
            );
        }
        i += 1;
    }
    let tracing = command == "stealth-demo";
    if numbers.len() > if tracing { 1 } else { 2 } {
        return Err("too many stealth experiment arguments".into());
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
    let scenario = stealth_scenario(&g)?;
    let supplies = if g.overlay.config.packages.len() > 1 {
        ["holy_water", "laudanum"]
            .into_iter()
            .map(|id| {
                Ok(SupplyStack {
                    definition: g.supply(id)?,
                    remaining: 2,
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![]
    };
    println!(
        "EXPERIMENTAL {PROFILE}: original stealth skills and candidate targeting/lifetime rules."
    );
    println!("Content ID: {}", g.overlay.config.id()?);
    println!(
        "Scenario: grave_robber/highwayman/bounty_hunter/vestal vs {}; round limit 100.",
        scenario
            .monsters
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    );
    println!(
        "initial_hero_stress=99 supplies={:?} checkpoint_every_step={tracing}",
        supplies
            .iter()
            .map(|s| (&s.definition.id, s.remaining))
            .collect::<Vec<_>>()
    );
    let start = Instant::now();
    let mut policy = StealthBaseline;
    let (mut wins, mut losses, mut draws, mut retreats, mut mutual) = (0, 0, 0, 0, 0);
    let (mut actions, mut stealth, mut ended, mut consumed, mut horror) = (0u64, 0, 0, 0, 0);
    for offset in 0..count {
        let mut b = Battle::new_with_stress_and_supplies(
            scenario.clone(),
            seed.wrapping_add(offset),
            &[99; 4],
            supplies.clone(),
        )?;
        while b.outcome().is_none() {
            let o = b.observation();
            let hero = o.units.iter().any(|u| Some(u.id) == o.actor && u.hero);
            let events = if hero {
                let a = policy.choose(&o).ok_or("no policy action")?;
                if tracing {
                    println!("round {} actor {:?}: {a:?}", o.round, o.actor);
                }
                b.step(a)?
            } else {
                if tracing {
                    println!("round {} actor {:?}: monster brain", o.round, o.actor);
                }
                b.step_monster()?
            };
            for e in &events {
                match e {
                    Event::Buff { id, .. } if id.ends_with(":stealth") => stealth += 1,
                    Event::StealthEnded { .. } => ended += 1,
                    Event::SupplyConsumed { .. } => consumed += 1,
                    Event::Effect {
                        name,
                        applied: true,
                        ..
                    } if name == "Color Madness Horror" => horror += 1,
                    _ => {}
                }
            }
            if tracing {
                println!("  {events:?}");
                let restored = Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)?;
                if restored != b {
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
        "stealth_applications={stealth} stealth_ended={ended} supplies_consumed={consumed} horror_applications={horror}"
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
