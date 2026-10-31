use dd_core::{Battle, Outcome, PROFILE, boss_scenario, boss_step};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
};
use std::{collections::BTreeMap, time::Instant};
pub fn run(args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("farm demo requires --candidate".into());
    }
    let mut profile = "plot_color_of_madness_0".to_string();
    let mut seed = 1;
    let mut groups = 4;
    let mut it = args[1..].iter();
    while let Some(k) = it.next() {
        match k.as_str() {
            "--profile" => profile = it.next().ok_or("missing wave profile")?.clone(),
            "--seed" => {
                seed = it
                    .next()
                    .ok_or("missing seed")?
                    .parse::<u64>()
                    .map_err(|_| "invalid seed")?
            }
            "--groups" => {
                groups = it
                    .next()
                    .ok_or("missing group bound")?
                    .parse::<u32>()
                    .map_err(|_| "invalid group bound")?
            }
            _ => return Err("unknown farm demo option".into()),
        }
    }
    if groups == 0 || groups > 100 {
        return Err("farm demo group bound must be 1..100".into());
    }
    let config = ContentConfig::full().with_boss_effect_candidates()?;
    let g = GameData::load_with_content(&dd_data::game_dir(), config.clone())?;
    let p = g.farm_plan(&profile, Default::default())?;
    let catalog = g.entity_catalog(&p.monster_ids())?;
    let (mut s, _) = boss_scenario(&g, &[&p.tables["farm"].mash[0].monsters[0]], "farm")?;
    s.monsters.clear();
    s.round_limit = 1000;
    s.rules.encounter_contract = Some(g.battle_contract(&profile)?);
    let start = Instant::now();
    let mut b = Battle::new_farm(s, p, catalog, seed)?;
    let mut mechanisms = BTreeMap::new();
    let mut actions = 0;
    println!(
        "EXPERIMENTAL {PROFILE}: original Farmstead wave battle; host curio handoff only, no curio effects applied"
    );
    println!("Content ID: {}", config.id()?);
    loop {
        let mut restored = Battle::load_json_with_content(&b.save_json()?, &config)?;
        if b.outcome() == Some(Outcome::WaveInterlude) {
            println!("HANDOFF {:?}", b.farm_interlude().unwrap());
            if b.farm_interlude().unwrap().group >= groups {
                break;
            }
            let events = b.continue_farm()?;
            if events != restored.continue_farm()? || b != restored {
                return Err("farm interlude replay mismatch".into());
            }
            continue;
        }
        if b.outcome().is_some() {
            break;
        }
        let events = boss_step(&mut b)?;
        if events != boss_step(&mut restored)?
            || b != restored
            || b.observation() != restored.observation()
        {
            return Err("farm per-action replay mismatch".into());
        }
        for e in events {
            let name = match e {
                dd_core::Event::Spawned { .. } => "spawn",
                dd_core::Event::Teleported { .. } => "teleport",
                dd_core::Event::FarmInterlude { .. } => "interlude",
                dd_core::Event::FarmWave { .. } => "wave",
                _ => continue,
            };
            *mechanisms.entry(name).or_insert(0) += 1;
        }
        actions += 1;
        if actions > 20000 {
            return Err("farm demo action bound".into());
        }
    }
    println!(
        "profile={profile} seed={seed} actions={actions} outcome={:?} wave={:?} mechanisms={mechanisms:?}",
        b.outcome(),
        b.observation().farm
    );
    println!("elapsed_seconds={:.6}", start.elapsed().as_secs_f64());
    Ok(())
}
