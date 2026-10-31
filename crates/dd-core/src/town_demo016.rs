use dd_core::{
    Baseline, Policy,
    api012::{BattleSession, Decision},
    opening015::{Definition as OpeningDefinition, Phase, Session as OpeningSession},
    town016::{Action, Definition, HostQuest, Observation, Session},
};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
    opening015::Candidates as OpeningCandidates,
    town016::Candidates,
};
use serde_json::json;
use std::collections::BTreeSet;
use std::time::Instant;
fn city_action(o: &Observation, used: &BTreeSet<String>) -> Option<Action> {
    let priority = |a: &Action| match a {
        Action::Upgrade {
            hero: None, tree, ..
        } if tree == "stage_coach.rostersize" => 0,
        Action::Equip { .. } => 1,
        Action::Upgrade { hero: None, .. } => 2,
        Action::Upgrade { hero: Some(_), .. } => 3,
        Action::LearnCamp { .. } => 4,
        Action::Hire { .. } if o.party.len() < 4 => 5,
        Action::Cancel { .. } => 6,
        Action::Queue { .. } => 7,
        _ => 100,
    };
    o.actions
        .iter()
        .filter(|c| priority(&c.action) < 100 && !used.contains(&format!("{:?}", c.action)))
        .min_by_key(|c| priority(&c.action))
        .map(|c| c.action.clone())
}
fn step(town: &mut Session, action: Action) -> Result<()> {
    let definition = town.definitions();
    let mut restored = Session::restore(&town.checkpoint()?, &definition)?;
    town.act(action.clone())?;
    restored.act(action)?;
    if town.checkpoint()? != restored.checkpoint()? || town.observe()? != restored.observe()? {
        return Err("town decision replay mismatch".into());
    }
    Ok(())
}
pub fn run(command: &str, args: &[String]) -> Result<()> {
    let mut mode = "radiant".to_string();
    let mut count = 1u64;
    let mut candidate = false;
    let mut returns = 0u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str(){"--candidate"=>candidate=true,"--mode"|"--seeds"|"--interface-returns"=>{let key=&args[i];i+=1;let value=args.get(i).ok_or("missing town option")?;match key.as_str(){"--mode"=>mode=value.clone(),"--seeds"=>count=value.parse().map_err(|_|"bad town seed count")?,_=>returns=value.parse().map_err(|_|"bad town interface count")?}},_=>return Err("usage: town-audit|town-probe --candidate --mode <mode> --seeds N [--interface-returns N]".into())}
        i += 1;
    }
    if !candidate || count == 0 || count > 10000 || returns > 100 {
        return Err("town requires explicit candidates and bounded probe count".into());
    }
    let mut config = ContentConfig::full().with_enabled_combat_candidates()?;
    config.mode = mode.clone();
    let game = GameData::load_with_content(&dd_data::game_dir(), config.clone())?;
    let definition = Definition::load(
        GameData::load_with_content(&dd_data::game_dir(), config.clone())?,
        Candidates::documented()?,
    )?;
    if command == "town-audit" {
        println!(
            "{}",
            serde_json::to_string_pretty(&definition.data().audit()).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let started = Instant::now();
    let opening_def = OpeningDefinition::load(
        GameData::load_with_content(&dd_data::game_dir(), config)?,
        OpeningCandidates::documented()?,
    )?;
    let mut opening = OpeningSession::new(opening_def, 1)?;
    let mut opening_actions = 0;
    while opening.observe()?.phase != Phase::Finished {
        let a = crate::opening_demo015::choose(&opening.observe()?)?;
        opening.act(a)?;
        opening_actions += 1;
        if opening_actions > 10000 {
            return Err("opening policy limit".into());
        }
    }
    let handoff = opening.town_handoff()?;
    let mut rows = vec![];
    for seed in 1..=count {
        let mut town = Session::from_opening(definition.clone(), handoff.clone(), seed)?;
        let mut steps = 0;
        let mut native_actions = 0;
        let mut seen = BTreeSet::new();
        for _ in 0..8 {
            let o = town.observe()?;
            let Some(a) = city_action(&o, &seen) else {
                break;
            };
            seen.insert(format!("{a:?}"));
            let mut restored = Session::restore(&town.checkpoint()?, &definition)?;
            town.act(a.clone())?;
            restored.act(a)?;
            if town.checkpoint()? != restored.checkpoint()?
                || town.observe()? != restored.observe()?
            {
                return Err("town decision replay mismatch".into());
            }
            steps += 1;
        }
        for n in 0..returns {
            let o = town.observe()?;
            for job in &o.jobs {
                if !job.locked {
                    step(&mut town, Action::Cancel { hero: job.hero })?;
                    steps += 1;
                }
            }
            let mut o = town.observe()?;
            let available = o
                .heroes
                .iter()
                .filter(|h| {
                    h.alive
                        && !h.busy
                        && !h.missing
                        && (h.stress < 100 || !matches!(h.resolve, dd_core::Resolve::Calm))
                })
                .map(|h| h.guid)
                .take(4)
                .collect::<Vec<_>>();
            if available.is_empty()
                && let Some(c) = o
                    .actions
                    .iter()
                    .find(|a| matches!(a.action, Action::Hire { .. }))
            {
                step(&mut town, c.action.clone())?;
                steps += 1;
                o = town.observe()?;
            }
            let available = o
                .heroes
                .iter()
                .filter(|h| {
                    h.alive
                        && !h.busy
                        && !h.missing
                        && (h.stress < 100 || !matches!(h.resolve, dd_core::Resolve::Calm))
                })
                .map(|h| h.guid)
                .take(4)
                .collect::<Vec<_>>();
            step(&mut town, Action::Party(available))?;
            steps += 1;
            let raw = definition.data().base.levels[1].quest["quest"].clone();
            let q=HostQuest{id:format!("controlled-interface-{n}"),content_id:definition.data().content.id()?,source:"controlled interface fixture; reward fields from original plot_tutorial_crypts; no natural map completion claim".into(),raw,repeatable:true,advances_region:true};
            let before = town.checkpoint()?;
            let mut restored = Session::restore(&before, &definition)?;
            let deployment = town.begin_sortie(q.clone(), BTreeSet::new())?;
            let replay = restored.begin_sortie(q, BTreeSet::new())?;
            if deployment != replay {
                return Err("departure replay differs".into());
            }
            let rules = town.expedition_rules(&deployment.quest, 100)?;
            let monster = game.monster("skeleton_common_A")?;
            let input = deployment.battle_input(
                vec![monster],
                rules,
                seed + n as u64,
                vec![],
                Default::default(),
            )?;
            let mut battle = BattleSession::new(input)?;
            let result = loop {
                match battle.advance()?.decision {
                    Decision::Player(o) => {
                        let action = Baseline
                            .choose(&o)
                            .ok_or("native interface policy exhausted")?;
                        let mut restored = BattleSession::restore(
                            &battle.checkpoint()?,
                            &game.rules.content,
                            &game.rules.candidates,
                        )?;
                        let actual = battle.act(action.clone())?;
                        native_actions += 1;
                        let replay = restored.act(action)?;
                        if actual != replay || battle.checkpoint()? != restored.checkpoint()? {
                            return Err("town native battle replay differs".into());
                        }
                    }
                    Decision::Finished(r) => break r,
                    _ => return Err("unexpected interlude in town interface probe".into()),
                }
            };
            let completed = result.outcome == dd_core::Outcome::HeroesWin;
            town.finish_sortie(
                &deployment.token,
                result.clone(),
                vec![],
                completed,
                BTreeSet::new(),
            )?;
            restored.finish_sortie(&replay.token, result, vec![], completed, BTreeSet::new())?;
            if town.checkpoint()? != restored.checkpoint()?
                || town.observe()? != restored.observe()?
            {
                return Err("return replay differs".into());
            }
            steps += 2;
        }
        let o = town.observe()?;
        rows.push(json!({"seed":seed,"steps":steps,"native_player_actions":native_actions,"visit":o.visit,"heroes":o.heroes.iter().map(|h|json!({"guid":h.guid,"alive":h.alive,"xp":h.xp,"level":h.level,"weapon":h.weapon,"armour":h.armour,"skills":h.skills})).collect::<Vec<_>>(),"wallet":o.wallet,"assets":o.trinkets.len(),"recruits":o.recruits.len(),"jobs":o.jobs.len(),"replay":"equal"}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"profile":dd_data::town016::PROFILE,"mode":mode,"definition_id":definition.id(),"opening_seed":1,"opening_actions":opening_actions,"town_seeds":count,"interface_returns_per_seed":returns,"interface_scope":"controlled native-battle return protocol probe; not additional natural quest completion","rows":rows,"elapsed_seconds":started.elapsed().as_secs_f64()})).map_err(|e|e.to_string())?);
    Ok(())
}
