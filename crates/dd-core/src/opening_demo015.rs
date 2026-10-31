pub(crate) use dd_core::opening_policy015::choose;
use dd_core::opening015::{Definition, Phase, Session};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
    opening015::{Candidates, Data},
};
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Instant;
pub fn run(command: &str, args: &[String]) -> Result<()> {
    let mut mode = "radiant".to_string();
    let mut count = 1u64;
    let mut candidate = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--candidate" => candidate = true,
            "--mode" | "--seeds" => {
                let key = &args[i];
                i += 1;
                let v = args.get(i).ok_or("missing opening option")?;
                if key == "--mode" {
                    mode = v.clone()
                } else {
                    count = v.parse().map_err(|_| "bad opening seed count")?;
                }
            }
            _ => {
                return Err(
                    "usage: opening-audit|opening-probe --candidate --mode <mode> --seeds N".into(),
                );
            }
        }
        i += 1;
    }
    if !candidate {
        return Err("opening requires explicit versioned candidates".into());
    }
    if count == 0 || count > 10000 {
        return Err("invalid opening seed bound".into());
    }
    let mut config = ContentConfig::full().with_enabled_combat_candidates()?;
    config.mode = mode.clone();
    let game = GameData::load_with_content(&dd_data::game_dir(), config)?;
    let candidates = Candidates::documented()?;
    let start = Instant::now();
    if command == "opening-audit" {
        let data = Data::load(&game, candidates)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"profile":dd_data::opening015::PROFILE,"content":data.content,"candidates":data.candidates,"slots":data.bag_rules.slots,"failure_return_rates":data.return_rates,"estate_sources":data.estate_sources,"initial":data.initial.iter().map(|h|json!({"guid":h.guid,"name":h.name,"class":h.blueprint.id,"xp":h.xp,"hp":h.blueprint.mechanics.hero.initial_hp,"skills":h.blueprint.skills.iter().map(|s|s.id.clone()).collect::<Vec<_>>(),"build":h.blueprint.mechanics.build.input})).collect::<Vec<_>>(),"recruits":data.recruit_classes,"class_extras":data.class_extras,"levels":data.levels.iter().map(|l|json!({"map":l.map.id,"nodes":l.map.nodes.len(),"source":l.map.source,"quest":l.quest,"goal":l.goal,"formations":l.formations.iter().map(|(id,f)|(id,json!({"monsters":f.monsters.iter().map(|m|m.id.clone()).collect::<Vec<_>>(),"loot":f.loot,"sources":f.sources}))).collect::<BTreeMap<_,_>>(),"pools":l.pools.entries,"pool_notes":l.pools.source_notes,"dependency_catalog":l.pools.catalog.keys().collect::<Vec<_>>(),"traps":l.traps,"default_trap":l.default_trap,"obstacle":l.obstacle})).collect::<Vec<_>>()})).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let def = Definition::load(game, candidates)?;
    let mut rows = vec![];
    for seed in 1..=count {
        let mut s = Session::new(def.clone(), seed)?;
        let mut actions = 0;
        loop {
            let o = s.observe()?;
            if o.phase == Phase::Finished {
                break;
            }
            let a = choose(&o)?;
            let mut replay = Session::restore(&s.checkpoint()?, &def)?;
            s.act(a.clone())
                .map_err(|e| format!("seed {seed} step {actions} {a:?}: {e}"))?;
            replay.act(a)?;
            if s.checkpoint()? != replay.checkpoint()? || s.observe()? != replay.observe()? {
                return Err("opening step replay mismatch".into());
            }
            actions += 1;
            if actions > 10000 {
                return Err("opening policy step limit".into());
            }
        }
        rows.push(json!({"seed":seed,"actions":actions,"metrics":s.metrics(),"replay":"equal"}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"profile":dd_data::opening015::PROFILE,"mode":mode,"definition_id":def.id(),"seeds":count,"rows":rows,"elapsed_seconds":start.elapsed().as_secs_f64()})).map_err(|e|e.to_string())?);
    Ok(())
}
