mod auto_demo018;
mod auto_demo020;
mod opening_demo015;
mod town_demo016;
use dd_core::{Baseline, Battle, Outcome, PROFILE, Policy, Scenario};
use dd_data::combat::{GameData, Result};
use dd_data::content::ContentConfig;
use std::{collections::BTreeMap, time::Instant};
mod boss_demo011;
mod closure_demo012;
mod context_demo010;
mod farm_demo011;
mod formation_demo;
mod initial_demo013;
mod lifecycle_demo;
mod map_demo014;
mod stealth_demo;
mod tactical_demo;

fn content_selection(command: &str, args: &[String]) -> Result<(ContentConfig, bool)> {
    if ["audit", "coverage"].contains(&command) {
        if let [flag, preset, choice] = args
            && flag == "--content"
            && preset == "full"
            && choice == "--combat-candidates"
        {
            return Ok((
                ContentConfig::full().with_enabled_combat_candidates()?,
                true,
            ));
        }
        if let [flag, preset, choice] = args
            && flag == "--content"
            && preset == "full"
            && choice == "--boss-candidates"
        {
            return Ok((ContentConfig::full().with_boss_effect_candidates()?, true));
        }
        return match args {
            [] => Ok((ContentConfig::fixture(), false)),
            [flag, preset] if flag == "--content" => match preset.as_str() {
                "fixture" => Ok((ContentConfig::fixture(), true)),
                "full" => Ok((ContentConfig::full(), true)),
                _ => Err(format!("unknown content preset {preset}")),
            },
            _ => Err("usage: audit|coverage [--content fixture|full]".into()),
        };
    }
    if args.iter().any(|a| a == "--content") {
        return Err("combat demos and benchmarks require fixture content".into());
    }
    Ok((ContentConfig::fixture(), false))
}
fn content_report(g: &GameData) -> Result<()> {
    println!("Content ID: {}", g.overlay.config.id()?);
    println!("Content layers: {}", g.overlay.layers.len());
    for layer in &g.overlay.layers {
        println!("  {} -> {}", layer.id, layer.root.display());
    }
    println!(
        "Hero classes={} monster variants={}",
        g.hero_ids().count(),
        g.monster_ids().count()
    );
    println!(
        "Hero class IDs: {}",
        g.hero_ids().cloned().collect::<Vec<_>>().join(", ")
    );
    let effect_ids = g
        .effects
        .records
        .iter()
        .map(|r| dd_data::combat::text(r, "name"))
        .collect::<Result<std::collections::BTreeSet<_>>>()?;
    println!(
        "Library IDs: effects={} Buffs={} Brains={} traits={}",
        effect_ids.len(),
        g.buffs.len(),
        g.brains.len(),
        g.traits.len()
    );
    let non_mode = dd_data::library::effects_with_modes(&g.overlay, false)?;
    let non_mode_ids = non_mode
        .records
        .iter()
        .map(|r| dd_data::combat::text(r, "name"))
        .collect::<Result<std::collections::BTreeSet<_>>>()?;
    println!(
        "Effect IDs excluding mode.effects.darkest: {}",
        non_mode_ids.len()
    );
    let collisions = g.overlay.collisions();
    println!(
        "Same-path collisions: {} (byte comparison only; non-combat contents are not parsed)",
        collisions.len()
    );
    let mut identical_paths = 0;
    let mut unresolved_paths = 0;
    let mut duplicate_sources = 0;
    for (path, sources) in collisions {
        let groups = g.overlay.collision_content_groups(path)?;
        let duplicates = sources.len() - groups.len();
        duplicate_sources += duplicates;
        let status = if groups.len() == 1 {
            identical_paths += 1;
            "identical/deduplicated"
        } else if dd_data::overlay::combat_path(path) {
            "library ID merge"
        } else {
            unresolved_paths += 1;
            "unresolved same-path conflict"
        };
        println!(
            "  {path}: {status}; sources={} distinct_contents={} deduplicated_sources={duplicates}",
            sources.len(),
            groups.len()
        );
        for (index, group) in groups.iter().enumerate() {
            println!(
                "    content {}: {}",
                index + 1,
                group
                    .iter()
                    .map(|i| g.overlay.layers[*i].id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    println!(
        "Same-path content summary: identical_paths={identical_paths} unresolved_noncombat_paths={unresolved_paths} duplicate_sources={duplicate_sources}"
    );
    for (kind, ids) in g.ambiguous_ids() {
        println!(
            "Ambiguous {kind} IDs ({}): {}",
            ids.len(),
            if ids.is_empty() {
                "(none)".into()
            } else {
                ids.join(", ")
            }
        );
    }
    println!(
        "Ignored unknown extensions: total={} {:?}",
        g.overlay.unknown_extensions.values().sum::<usize>(),
        g.overlay.unknown_extensions
    );
    if let Some(error) = &g.combat_globals_error {
        println!("Combat globals unavailable (references rejected): {error}");
    }
    Ok(())
}

fn coverage(g: &GameData) -> Result<()> {
    for heroes in [true, false] {
        let mut total = 0;
        let mut loaded = vec![];
        let mut rejected: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut record = |id: String, result: Result<dd_data::combat::Blueprint>| {
            total += 1;
            match result {
                Ok(_) => loaded.push(id),
                Err(error) => {
                    // Keep only the first diagnostic, without its data ID prefix.
                    let reason = error.split_once(": ").map_or(error.as_str(), |(_, r)| r);
                    rejected.entry(reason.to_owned()).or_default().push(id);
                }
            }
        };
        if heroes {
            for id in g.hero_ids() {
                for skill in g.hero_skill_ids(id)? {
                    record(format!("{id}.{skill}"), g.hero(id, 0, 0, &[&skill]));
                }
            }
        } else {
            for id in g.monster_ids() {
                record(id.clone(), g.monster(id));
            }
        }
        loaded.sort();
        println!(
            "{}: {}/{total} loaded",
            if heroes {
                "Hero level-0 skills"
            } else {
                "Monster variants"
            },
            loaded.len()
        );
        let mut groups = rejected.into_iter().collect::<Vec<_>>();
        groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
        for (reason, examples) in groups {
            println!(
                "  {}: {reason}; examples: {}",
                examples.len(),
                examples
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if !heroes {
            println!("Loaded monster IDs: {}", loaded.join(", "));
        }
    }
    Ok(())
}

fn demo(g: &GameData) -> Result<Scenario> {
    Ok(Scenario {
        heroes: vec![
            g.hero("highwayman", 0, 0, &["wicked_slice", "opened_vein"])?,
            g.hero("crusader", 0, 0, &["stunning_blow", "battle_heal"])?,
            g.hero("jester", 0, 0, &["inspiring_tune"])?,
            g.hero("vestal", 0, 0, &["divine_grace", "gods_comfort"])?,
        ],
        monsters: vec![
            g.monster("skeleton_common_A")?,
            g.monster("skeleton_common_A")?,
            g.monster("skeleton_courtier_A")?,
            g.monster("skeleton_courtier_A")?,
        ],
        rules: g.rules.clone(),
        round_limit: 100,
    })
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    if command == "auto" {
        return auto_demo018::run(&args[1..]);
    }
    if command == "auto020" {
        return auto_demo020::run(&args[1..]);
    }
    if matches!(command, "town-audit" | "town-probe") {
        return town_demo016::run(command, &args[1..]);
    }
    if matches!(command, "opening-audit" | "opening-probe") {
        return opening_demo015::run(command, &args[1..]);
    }
    if matches!(command, "map-audit" | "map-matrix" | "map-probe") {
        return map_demo014::run(&args);
    }
    if matches!(command, "initial-audit" | "initial-probe") {
        return initial_demo013::run(&args);
    }
    if matches!(
        command,
        "config-matrix" | "skill-matrix" | "build-matrix" | "unit-matrix"
    ) {
        return closure_demo012::run(command, &args[1..]);
    }
    if command == "farm-demo" {
        return farm_demo011::run(&args[1..]);
    }
    if matches!(
        command,
        "boss-demo" | "boss-benchmark" | "boss-audit" | "boss-matrix"
    ) {
        return boss_demo011::run(command, &args[1..]);
    }
    if matches!(command, "context-demo" | "context-benchmark") {
        return context_demo010::run(command, &args[1..]);
    }
    if command == "closure" {
        let (config, _) = content_selection("audit", &args[1..])?;
        let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
        let report = dd_data::closure012::inventory(&g)?;
        println!(
            "{}",
            serde_json::to_string(&report).map_err(|e| e.to_string())?
        );
        if !report.unresolved.is_empty() {
            return Err("unresolved combat closure dependencies; see report".into());
        }
        return Ok(());
    }
    if command == "support" {
        let (config, _) = content_selection("audit", &args[1..])?;
        let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
        println!(
            "{}",
            serde_json::to_string(&dd_data::support::inventory(&g)?).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if matches!(command, "formation-demo" | "formation-benchmark") {
        return formation_demo::run(command, &args[1..]);
    }
    if matches!(command, "tactical-demo" | "tactical-benchmark") {
        return tactical_demo::run(command, &args[1..]);
    }
    if matches!(command, "lifecycle-demo" | "lifecycle-benchmark") {
        return lifecycle_demo::run(command, &args[1..]);
    }
    if matches!(command, "stealth-demo" | "stealth-benchmark") {
        return stealth_demo::run(command, &args[1..]);
    }
    if command == "help" {
        println!(
            "dd-core auto --candidate [--seed N] [--mode radiant|base|new_game_plus|bloodmoon] [--weeks 208] [--out DIR] [--resume FILE] [--plain] [--every N] [--delay-ms N]"
        );
        println!(
            "dd-core town-audit|town-probe --candidate --mode <base|radiant|new_game_plus|bloodmoon> --seeds N [--interface-returns N]"
        );
        println!(
            "dd-core context-demo --candidate [seed] [--content fixture|full] [--torch-light <host candidate amount> --torch-source <source>]\ndd-core context-benchmark --candidate [battles] [seed] [--content fixture|full] [--torch-light <host candidate amount> --torch-source <source>]"
        );
        println!(
            "dd-core formation-demo --candidate [seed] [--content fixture|full]\ndd-core formation-benchmark --candidate [battles] [seed] [--content fixture|full]"
        );
        println!(
            "dd-core stealth-demo --candidate [seed] [--content fixture|full]\ndd-core stealth-benchmark --candidate [battles] [seed] [--content fixture|full]"
        );
        println!(
            "dd-core audit [--content fixture|full]\ndd-core coverage [--content fixture|full]\ndd-core demo --candidate [seed]\ndd-core stress-demo --candidate [seed]\ndd-core benchmark --candidate [battles] [seed]\ndd-core stress-benchmark --candidate [battles] [seed]\ndd-core displacement-demo --candidate [seed]\ndd-core displacement-benchmark --candidate [battles] [seed]\nDD_GAME_DIR overrides the base-game data directory. See CALIBRATION.md before interpreting results."
        );
        return Ok(());
    }
    if ![
        "audit",
        "coverage",
        "displacement-demo",
        "displacement-benchmark",
        "demo",
        "stress-demo",
        "benchmark",
        "stress-benchmark",
    ]
    .contains(&command)
    {
        return Err(format!("unknown command {command}"));
    }
    if !["audit", "coverage"].contains(&command)
        && args.get(1).map(String::as_str) != Some("--candidate")
    {
        return Err(
            "experimental combat requires --candidate; not calibrated for formal training".into(),
        );
    }
    let (config, explicit_content) = content_selection(command, &args[1..])?;
    let g = GameData::load_with_content(&dd_data::game_dir(), config)?;
    if explicit_content {
        content_report(&g)?;
    }
    if command == "coverage" {
        return coverage(&g);
    }
    if command == "audit" {
        let mut counts = [0usize; 2];
        let mut files = 0;
        for (index, heroes) in [true, false].into_iter().enumerate() {
            for document in g.info_documents(heroes) {
                counts[index] += document.records.len();
                files += 1;
            }
        }
        println!(
            "Base game data: {}\nParsed {files} info files: {} hero records, {} monster records; {} effect records; {} monster brains.\nBuff ids={} conflicting_ids={} resolve_traits={}.\nThis audits syntax/data loading, not mechanical coverage.",
            g.root.display(),
            counts[0],
            counts[1],
            g.effects.records.len(),
            g.brains.len(),
            g.buffs.len(),
            g.buffs.values().filter(|v| v.is_array()).count(),
            g.rules.globals.as_ref().map_or_else(
                || g.traits
                    .values()
                    .filter(|v| v["is_generated"] == true)
                    .count(),
                |globals| globals.traits.len()
            )
        );
        let mut turn_kinds = vec![];
        let mut combat_reactions = vec![];
        let mut external_reactions = vec![];
        let Some(globals) = g.rules.globals.as_ref() else {
            println!(
                "Trait event validation rejected: {}",
                g.combat_globals_error
                    .as_deref()
                    .unwrap_or("missing definitions")
            );
            return Ok(());
        };
        for t in &globals.traits {
            for a in &t.turn_acts {
                if a.chance > 0. && !turn_kinds.contains(&a.id) {
                    turn_kinds.push(a.id);
                }
            }
            for r in &t.reactions {
                if r.chance > 0. {
                    let list = if r.id.is_combat() {
                        &mut combat_reactions
                    } else {
                        &mut external_reactions
                    };
                    if !list.contains(&r.id) {
                        list.push(r.id);
                    }
                }
            }
        }
        println!(
            "Enabled trait turn kinds={} combat reaction kinds={} outside-combat reaction kinds={} (retained, not emitted by combat).",
            turn_kinds.len(),
            combat_reactions.len(),
            external_reactions.len()
        );
        return Ok(());
    }
    let number = |i: usize, default: u64| -> Result<u64> {
        args.get(i).map_or(Ok(default), |s| {
            s.parse().map_err(|_| format!("invalid integer: {s}"))
        })
    };
    let tracing = matches!(command, "demo" | "stress-demo" | "displacement-demo");
    let displacement = matches!(command, "displacement-demo" | "displacement-benchmark");
    let high_stress = matches!(command, "stress-demo" | "stress-benchmark");
    let count = if tracing { 1 } else { number(2, 10000)? };
    let seed = number(if tracing { 2 } else { 3 }, 1)?;
    if count == 0 || count > 1_000_000 {
        return Err("battle count must be 1..1000000".into());
    }
    let scenario = if displacement {
        dd_core::displacement_scenario(&g)?
    } else {
        demo(&g)?
    };
    let mut wins = 0;
    let mut losses = 0;
    let mut draws = 0;
    let mut actions = 0u64;
    let mut baseline = Baseline;
    let mut stress_events = 0u64;
    let mut resolve_events = 0u64;
    let mut heart_attacks = 0u64;
    let mut act_outs = 0u64;
    let mut reactions = 0u64;
    let mut refusals = 0u64;
    let mut retreats = 0u64;
    let mut mutual_defeats = 0u64;
    let mut move_events = 0u64;
    let mut move_skips = 0u64;
    let mut hero_skills = BTreeMap::<String, u64>::new();
    if displacement {
        println!(
            "EXPERIMENTAL {PROFILE}: hero Baseline vs interpreted original monster brain data (candidate weighting).\nScenario: highwayman/hellion/bounty_hunter/vestal vs 2 cultist_brawler_A + brigand_fusilier_A + skeleton_arbalist_A; base equipment/skills; round limit 100.\nConditional damage, resisted displacement, backward movement and battle use limits enabled; see CALIBRATION.md."
        );
    } else {
        println!(
            "EXPERIMENTAL {PROFILE}: hero Baseline vs interpreted original monster brain data (candidate weighting).\nScenario: highwayman/crusader/jester/vestal vs 2 skeleton_common_A + 2 skeleton_courtier_A; base equipment/skills; round limit 100.\nTrait turn act-outs, assistance refusals and combat reactions enabled with candidate timing. Light, stall and special mechanics remain excluded; see CALIBRATION.md."
        );
    }
    let start = Instant::now();
    println!("initial_hero_stress={}", if high_stress { 99 } else { 0 });
    for offset in 0..count {
        let mut battle = if high_stress {
            Battle::new_with_stress(scenario.clone(), seed.wrapping_add(offset), &[99; 4])?
        } else {
            Battle::new(scenario.clone(), seed.wrapping_add(offset))?
        };
        while battle.outcome().is_none() {
            let o = battle.observation();
            let hero = o
                .units
                .iter()
                .find(|u| Some(u.id) == o.actor)
                .is_some_and(|u| u.hero);
            let events = if hero {
                let action = baseline.choose(&o).ok_or("policy returned no action")?;
                if displacement && matches!(action, dd_core::Action::Skill { .. }) {
                    let label = &o.actions.iter().find(|a| a.action == action).unwrap().label;
                    *hero_skills.entry(label.clone()).or_default() += 1;
                }
                if tracing {
                    println!("round {} actor {:?}: {:?}", o.round, o.actor, action);
                }
                battle.step(action)?
            } else {
                if tracing {
                    println!("round {} actor {:?}: monster brain", o.round, o.actor);
                }
                battle.step_monster()?
            };
            for e in &events {
                match e {
                    dd_core::Event::Stress { .. } => stress_events += 1,
                    dd_core::Event::Resolve { .. } => resolve_events += 1,
                    dd_core::Event::HeartAttack(_) => heart_attacks += 1,
                    dd_core::Event::ActOut { .. } => act_outs += 1,
                    dd_core::Event::Reaction { .. } => reactions += 1,
                    dd_core::Event::Refused { .. } => refusals += 1,
                    dd_core::Event::Move { .. } => move_events += 1,
                    dd_core::Event::MoveSkipped { .. } => move_skips += 1,
                    _ => {}
                }
            }
            if tracing {
                println!("  {events:?}");
            }
            actions += 1;
        }
        match battle.outcome().unwrap() {
            Outcome::HeroesWin => wins += 1,
            Outcome::MonstersWin => losses += 1,
            Outcome::RoundLimit => draws += 1,
            Outcome::Retreated => retreats += 1,
            Outcome::MutualDefeat => mutual_defeats += 1,
            Outcome::Teleported | Outcome::FlashbackFailed | Outcome::WaveInterlude => {
                return Err("unexpected relocation in legacy scene".into());
            }
            Outcome::ResolutionLimit => {
                return Err("reaction resolution budget exceeded; experiment failed".into());
            }
        }
        if tracing {
            let saved = battle.save_json()?;
            if Battle::load_json(&saved)? != battle {
                return Err("checkpoint verification failed".into());
            }
            println!(
                "Outcome: {:?}; checkpoint round-trip verified",
                battle.outcome()
            );
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    if displacement {
        println!(
            "move_events={move_events} move_skips={move_skips} resolution_limits=0\nHero skill uses: {hero_skills:?}"
        );
    }
    println!(
        "stress_events={stress_events} resolve_events={resolve_events} heart_attacks={heart_attacks}"
    );
    println!(
        "act_outs={act_outs} reactions={reactions} refusals={refusals} retreats={retreats} mutual_defeats={mutual_defeats}"
    );
    println!(
        "battles={count} seed={seed} hero_wins={wins} losses={losses} round_limits={draws} actions={actions}\nhero_win_rate={:.4}% elapsed_seconds={elapsed:.6} battles_per_second={:.2} actions_per_second={:.2}",
        100. * wins as f64 / count as f64,
        count as f64 / elapsed,
        actions as f64 / elapsed
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

