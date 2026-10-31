use dd_core::{Battle, ContextBaseline, Event, Outcome, PROFILE, Policy, context_scenario};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
};
use std::time::Instant;
pub fn run(command: &str, args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("context experiments require --candidate".into());
    }
    let mut numbers = vec![];
    let mut content = None;
    let mut torch_light = None;
    let mut torch_source = None;
    let mut args = args[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--content" => {
                if content.is_some() {
                    return Err("duplicate --content".into());
                }
                content = Some(match args.next().map(String::as_str) {
                    Some("fixture") => ContentConfig::fixture(),
                    Some("full") => ContentConfig::full(),
                    _ => return Err("invalid --content".into()),
                });
            }
            "--torch-light" => {
                if torch_light.is_some() {
                    return Err("duplicate --torch-light".into());
                }
                torch_light = Some(
                    args.next()
                        .ok_or("missing torch amount")?
                        .parse::<i32>()
                        .map_err(|_| "invalid torch amount")?,
                );
            }
            "--torch-source" => {
                if torch_source.is_some() {
                    return Err("duplicate --torch-source".into());
                }
                torch_source = Some(args.next().ok_or("missing torch source")?.clone());
            }
            _ => numbers.push(arg.parse::<u64>().map_err(|_| "invalid context argument")?),
        }
    }
    if torch_light.is_some() != torch_source.is_some() {
        return Err("torch candidate requires amount and source together".into());
    }
    let demo = command == "context-demo";
    if numbers.len() > if demo { 1 } else { 2 } {
        return Err("too many context arguments".into());
    }
    let count = if demo {
        1
    } else {
        *numbers.first().unwrap_or(&1000)
    };
    let seed = *numbers.get(usize::from(!demo)).unwrap_or(&1);
    if !(1..=1000000).contains(&count) {
        return Err("invalid battle count".into());
    }
    let load = Instant::now();
    let config = content.unwrap_or_else(ContentConfig::fixture);
    let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
    let (scene, mut supplies) = context_scenario(&g)?;
    if let (Some(light_amount), Some(source)) = (torch_light, torch_source) {
        let candidate = dd_data::items::HostItemCandidate {
            light_amount,
            source,
        };
        println!(
            "Host item candidate: torch light={} source={} calibration=pending",
            candidate.light_amount, candidate.source
        );
        supplies.push(dd_data::items::SupplyStack {
            definition: g.supply_with_light_candidate("torch", candidate)?,
            remaining: 2,
        });
    }
    let loading = load.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut outcomes = [0u64; 5];
    let mut actions = 0;
    let mut modes = 0;
    let mut lights = 0;
    let mut items = 0;
    let mut diseases = 0;
    println!("EXPERIMENTAL {PROFILE}: class and battle context candidate");
    println!("Content ID: {}", g.overlay.config.id()?);
    for offset in 0..count {
        let mut b =
            Battle::new_with_supplies(scene.clone(), seed.wrapping_add(offset), supplies.clone())?;
        while b.outcome().is_none() {
            let o = b.observation();
            let hero = o.units.iter().any(|u| u.id == o.actor.unwrap() && u.hero);
            let events = if hero {
                let a = ContextBaseline
                    .choose(&o)
                    .ok_or("no context policy action")?;
                b.step(a)
            } else {
                b.step_monster()
            }
            .map_err(|e| {
                format!(
                    "seed={} round={} actor={:?}: {e}",
                    seed.wrapping_add(offset),
                    o.round,
                    o.actor
                )
            })?;
            for e in &events {
                match e {
                    Event::Mode { .. } => modes += 1,
                    Event::Light { .. } => lights += 1,
                    Event::SupplyConsumed { .. } => items += 1,
                    Event::Effect {
                        name,
                        applied: true,
                        ..
                    } if name.to_lowercase().contains("disease") => diseases += 1,
                    _ => {}
                }
            }
            if demo {
                println!("round={} actor={:?}: {events:?}", o.round, o.actor);
                if b != Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)? {
                    return Err("context checkpoint mismatch".into());
                }
            }
            actions += 1;
        }
        let result = b.result()?;
        assert_eq!(Some(result.outcome), b.outcome());
        let index = match result.outcome {
            Outcome::HeroesWin => 0,
            Outcome::MonstersWin => 1,
            Outcome::RoundLimit => 2,
            Outcome::Retreated => 3,
            Outcome::MutualDefeat => 4,
            Outcome::Teleported | Outcome::FlashbackFailed | Outcome::WaveInterlude => {
                return Err("unexpected relocation in legacy scene".into());
            }
            Outcome::ResolutionLimit => return Err("context resolution limit".into()),
        };
        outcomes[index] += 1;
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("mechanisms mode={modes} light={lights} items={items} disease={diseases}");
    println!(
        "battles={count} seed={seed} outcomes[win,loss,limit,retreat,mutual]={outcomes:?} actions={actions}"
    );
    println!(
        "load_seconds={loading:.6} elapsed_seconds={elapsed:.6} battles_per_second={:.2} actions_per_second={:.2}",
        count as f64 / elapsed,
        actions as f64 / elapsed
    );
    Ok(())
}
