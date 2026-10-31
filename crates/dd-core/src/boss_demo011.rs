use dd_core::{Battle, Event, PROFILE, boss_encounter_battle_in_region, boss_scenario, boss_step};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
};
use std::{collections::BTreeMap, time::Instant};
pub fn run(command: &str, args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("boss experiments require --candidate".into());
    }
    let mut config = ContentConfig::fixture();
    let mut choices = false;
    let mut combat_choices = false;
    let mut mode = None;
    let mut shared_region = None;
    let mut scene = "flesh".to_string();
    let mut variant = "A".to_string();
    let mut numbers = vec![];
    let mut it = args[1..].iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--content" => {
                config = match it.next().map(String::as_str) {
                    Some("fixture") => ContentConfig::fixture(),
                    Some("full") => ContentConfig::full(),
                    _ => return Err("invalid content preset".into()),
                }
            }
            "--boss-candidates" => choices = true,
            "--combat-candidates" => combat_choices = true,
            "--mode" => mode = Some(it.next().ok_or("missing mode")?.clone()),
            "--shared-region" => {
                shared_region = Some(it.next().ok_or("missing shared host region")?.clone())
            }
            "--scene" => scene = it.next().ok_or("missing scene")?.clone(),
            "--variant" => variant = it.next().ok_or("missing variant")?.clone(),
            _ => numbers.push(arg.parse::<u64>().map_err(|_| "invalid boss argument")?),
        }
    }
    if let Some(mode) = mode {
        config.mode = mode;
    }
    if combat_choices {
        config = config.with_enabled_combat_candidates()?;
    } else if choices {
        config = config.with_boss_effect_candidates()?;
    }
    if !["A", "B", "C", "D", "E", "F"].contains(&variant.as_str()) {
        return Err("invalid variant".into());
    }
    let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
    println!("EXPERIMENTAL {PROFILE}: Boss/DLC candidate");
    println!("Content ID: {}", g.overlay.config.id()?);
    if command == "boss-audit" || command == "boss-matrix" {
        let defs = g.boss_encounters()?;
        let mut failures = vec![];
        let mut executed = 0;
        let mut counts = BTreeMap::new();
        let mut runtime = vec![];
        println!("Shared roaming host region: {shared_region:?}");
        let seeds = if command == "boss-audit" {
            vec![1]
        } else if numbers.is_empty() {
            vec![1, 5, 17]
        } else {
            numbers.clone()
        };
        for d in &defs {
            for seed in &seeds {
                let result = boss_encounter_battle_in_region(&g, d, *seed,shared_region.as_deref()).and_then(|mut b| {
                    let mut seen=std::collections::BTreeSet::new();let mut mechanisms=BTreeMap::new();let mut actions=0;
                    seen.extend(b.observation().units.into_iter().filter(|u|!u.hero).map(|u|u.class));
                    if command == "boss-matrix" {
                        for _ in 0..2000 {
                            if b.outcome().is_some() {
                                break;
                            }
                            let mut restored =
                                Battle::load_json_with_content(&b.save_json()?, &g.overlay.config)?;
                            let events = boss_step(&mut b)?;
                            actions+=1;seen.extend(b.observation().units.into_iter().filter(|u|!u.hero).map(|u|u.class));
                            for e in &events{let name=match e{Event::Changed{to,..}=>{seen.insert(to.clone());"change"},Event::SharedHealth{..}=>"shared_health",Event::Captured{..}=>"capture",Event::Released{..}=>"release",Event::Controlled{..}=>"control",Event::ControlReleased{..}=>"control_release",Event::Teleported{..}=>"teleport",Event::BonusAction{..}=>"bonus",Event::Mode{..}=>"mode",_=>continue};*mechanisms.entry(name).or_insert(0)+=1;}
                            if events != boss_step(&mut restored)?
                                || b != restored
                                || b.observation() != restored.observation()
                            {
                                return Err("matrix per-action replay mismatch".into());
                            }
                        }
                        if b.outcome().is_none() {
                            return Err("matrix action bound".into());
                        }
                        *counts
                            .entry(format!("{:?}", b.outcome().unwrap()))
                            .or_insert(0) += 1;
                    }
                    runtime.push(serde_json::json!({"encounter":d.id,"seed":seed,"outcome":format!("{:?}",b.outcome()),"actions":actions,"classes":seen,"mechanisms":mechanisms}));
                    Ok(())
                });
                match result {
                    Ok(()) => executed += 1,
                    Err(error) => failures.push((format!("{} seed={seed}", d.id), error)),
                }
            }
        }
        println!(
            "formations={} seeds={seeds:?} cases={} accepted={} rejected={} outcomes={counts:?}",
            defs.len(),
            defs.len() * seeds.len(),
            executed,
            failures.len()
        );
        for (id, error) in &failures {
            println!("REJECT {id}: {error}");
        }
        std::fs::write(
            "target/011-boss-runtime.json",
            serde_json::to_string_pretty(&runtime).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let manifest = serde_json::to_string_pretty(&defs).map_err(|e| e.to_string())?;
        std::fs::write("target/011-encounters.json", manifest).map_err(|e| e.to_string())?;
        return if failures.is_empty() {
            Ok(())
        } else {
            Err("boss formation acceptance incomplete".into())
        };
    }
    let ids = match scene.as_str() {
        "flesh" => [
            "formless_melee",
            "formless_guard",
            "formless_weak",
            "formless_ranged",
        ]
        .iter()
        .map(|s| format!("{s}_{variant}"))
        .collect::<Vec<_>>(),
        "hag" => vec![
            format!("cauldron_empty_{variant}"),
            format!("hag_{variant}"),
        ],
        "fanatic" => vec![
            format!("pyre_empty_{variant}"),
            format!("fanatic_{variant}"),
        ],
        "crew" => vec![
            format!("drowned_anchor_{variant}"),
            format!("drowned_captain_{variant}"),
        ],
        "siren" => vec![format!("siren_{variant}")],
        "countess" => vec!["countess_D".into()],
        "heart" => vec!["ancestor_heart_D".into()],
        _ => return Err("unknown boss scene".into()),
    };
    let refs = ids.iter().map(String::as_str).collect::<Vec<_>>();
    let demo = command == "boss-demo";
    let n = if demo {
        1
    } else {
        *numbers.first().unwrap_or(&1000)
    };
    if n == 0 || n > 1000000 {
        return Err("invalid boss count".into());
    }
    let seed = *numbers.get(usize::from(!demo)).unwrap_or(&1);
    let mut outcomes = BTreeMap::new();
    let mut actions = 0;
    let mut mechanisms = BTreeMap::new();
    let start = Instant::now();
    let (scenario, catalog) = boss_scenario(
        &g,
        &refs,
        match scene.as_str() {
            "countess" => "courtyard",
            "hag" | "fanatic" => "weald",
            "crew" | "siren" => "cove",
            "heart" => "darkestdungeon",
            _ => "warrens",
        },
    )?;
    for index in 0..n {
        let mut b = Battle::new_encounter(scenario.clone(), seed + index, catalog.clone(), vec![])?;
        while b.outcome().is_none() {
            let o = b.observation();
            let mut restored = if demo {
                Some(Battle::load_json_with_content(
                    &b.save_json()?,
                    &g.overlay.config,
                )?)
            } else {
                None
            };
            let events = boss_step(&mut b)?;
            if demo {
                let restored = restored.as_mut().unwrap();
                let other = boss_step(restored)?;
                if events != other || &b != restored {
                    return Err("boss per-action replay mismatch".into());
                }
                println!("round={} actor={:?}: {events:?}", o.round, o.actor);
            }
            for e in events {
                let kind = match e {
                    Event::Changed { .. } => "change",
                    Event::SharedHealth { .. } => "shared_hp",
                    Event::Captured { .. } => "capture",
                    Event::Released { .. } => "release",
                    Event::Controlled { .. } => "control",
                    Event::ControlReleased { .. } => "control_release",
                    _ => continue,
                };
                *mechanisms.entry(kind).or_insert(0) += 1;
            }
            actions += 1;
        }
        *outcomes
            .entry(format!("{:?}", b.outcome().unwrap()))
            .or_insert(0) += 1;
    }
    let t = start.elapsed().as_secs_f64();
    println!("scene={scene} variants={variant} outcomes={outcomes:?} mechanisms={mechanisms:?}");
    println!(
        "battles={n} seed={seed} actions={actions} elapsed_seconds={t:.6} battles_per_second={:.2}",
        n as f64 / t
    );
    Ok(())
}
