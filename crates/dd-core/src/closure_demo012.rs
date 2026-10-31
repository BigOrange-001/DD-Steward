use dd_core::{PROFILE, validation012};
use dd_data::{
    combat::{GameData, Result},
    content::ContentConfig,
};
use std::{io::Write, path::PathBuf};

pub fn run(command: &str, args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("--candidate") {
        return Err("closure experiments require --candidate".into());
    }
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    if command == "config-matrix" {
        let mut configs = validation012::configurations();
        let mut start = 0usize;
        let mut limit = configs.len();
        let mut selected_mode = None;
        let mut it = args[1..].iter();
        while let Some(k) = it.next() {
            match k.as_str() {
                "--only-mode" => selected_mode = Some(it.next().ok_or("missing mode")?.clone()),
                "--start" => {
                    start = it
                        .next()
                        .ok_or("missing start")?
                        .parse()
                        .map_err(|_| "invalid start")?
                }
                "--limit" => {
                    limit = it
                        .next()
                        .ok_or("missing limit")?
                        .parse()
                        .map_err(|_| "invalid limit")?
                }
                _ => return Err("unknown config matrix argument".into()),
            }
        }
        if let Some(mode) = &selected_mode {
            if !["base", "radiant", "new_game_plus", "bloodmoon"].contains(&mode.as_str()) {
                return Err("unknown matrix mode".into());
            }
            configs.retain(|c| &c.mode == mode);
        }
        if start >= configs.len() || limit == 0 {
            return Err("invalid configuration slice".into());
        }
        let log_path = out.join(format!("012-config-matrix-{start}.jsonl"));
        let mut file =
            std::io::BufWriter::new(std::fs::File::create(&log_path).map_err(|e| e.to_string())?);
        let mut values = vec![];
        let mut failures = 0;
        println!(
            "EXPERIMENTAL {PROFILE}: all valid package/mode/never_again combinations; explicit source candidates"
        );
        println!("total={} start={start} limit={limit}", configs.len());
        for (index, c) in configs.iter().enumerate().skip(start).take(limit) {
            let value = match GameData::load_with_content(&dd_data::game_dir(), c.clone())
                .and_then(|g| validation012::configuration_case(&g))
            {
                Ok(row) => {
                    if !row.errors.is_empty() {
                        failures += 1;
                    }
                    serde_json::to_value(row).map_err(|e| e.to_string())?
                }
                Err(e) => {
                    failures += 1;
                    serde_json::json!({"content_id":c.id()?,"errors":[e]})
                }
            };
            writeln!(
                file,
                "{}",
                serde_json::to_string(&value).map_err(|e| e.to_string())?
            )
            .map_err(|e| e.to_string())?;
            file.flush().map_err(|e| e.to_string())?;
            values.push(value);
            println!("case={}/{} failures={failures}", index + 1, configs.len());
        }
        let complete = selected_mode.is_none() && start == 0 && values.len() == configs.len();
        std::fs::write(
            out.join("012-config-matrix.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
            "profile":PROFILE,"only_mode":selected_mode,"total":configs.len(),"processed":values.len(),"complete":complete,
            "failures":failures,"cases":values}))
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!(
            "processed={} complete={complete} failures={failures}",
            values.len()
        );
        if failures > 0 {
            return Err("configuration matrix has explicit failures; see JSONL report".into());
        }
        return Ok(());
    }
    if command == "unit-matrix" {
        if args.len() != 1 {
            return Err("unknown unit matrix argument".into());
        }
        let mut rows = vec![];
        let mut failures = 0;
        for mode in ["base", "radiant", "new_game_plus", "bloodmoon"] {
            let mut c = ContentConfig::full();
            c.mode = mode.into();
            let g = GameData::load_with_content(
                &dd_data::game_dir(),
                c.with_enabled_combat_candidates()?,
            )?;
            let probes = validation012::remaining_unit_matrix(&g)?;
            let count = probes.iter().filter(|r| r.get("error").is_some()).count();
            failures += count;
            println!(
                "mode={mode} untagged_probes={} failures={count}",
                probes.len()
            );
            rows.push(
                serde_json::json!({"mode":mode,"content_id":g.rules.content.id()?,"probes":probes}),
            );
        }
        std::fs::write(
            out.join("012-unit-matrix.json"),
            serde_json::to_vec_pretty(&rows).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if failures > 0 {
            return Err("original NPC turn matrix contains failures".into());
        }
        return Ok(());
    }
    if command == "build-matrix" {
        if args.len() != 1 {
            return Err("unknown build matrix argument".into());
        }
        let g = GameData::load_with_content(
            &dd_data::game_dir(),
            ContentConfig::full().with_enabled_combat_candidates()?,
        )?;
        let rows = validation012::build_matrix(&g)?;
        let failures = rows.iter().filter(|r| r.get("error").is_some()).count();
        let disabled = rows
            .iter()
            .filter(|r| r["scope"] == "disabled_actor")
            .count();
        std::fs::write(out.join("012-build-matrix.json"),serde_json::to_vec_pretty(&serde_json::json!({"content_id":g.rules.content.id()?,"candidate_id":g.rules.candidates.identity()?,"rows":rows,"failures":failures,"disabled":disabled})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        println!(
            "EXPERIMENTAL {PROFILE}: equipped build definitions and source actions; rows={} disabled={disabled} failures={failures}",
            rows.len()
        );
        if failures > 0 {
            return Err("build matrix contains failures".into());
        }
        return Ok(());
    }
    if command == "skill-matrix" {
        let mut fixture = false;
        let mut selected_mode = None;
        let mut it = args[1..].iter();
        while let Some(k) = it.next() {
            match k.as_str() {
                "--content" => {
                    fixture = match it.next().map(String::as_str) {
                        Some("fixture") => true,
                        Some("full") => false,
                        _ => return Err("invalid content".into()),
                    }
                }
                "--mode" => selected_mode = Some(it.next().ok_or("missing mode")?.clone()),
                _ => return Err("unknown skill matrix argument".into()),
            }
        }
        let modes = if let Some(mode) = selected_mode {
            vec![mode]
        } else if fixture {
            vec!["base".into()]
        } else {
            ["base", "radiant", "new_game_plus", "bloodmoon"]
                .map(str::to_string)
                .to_vec()
        };
        let mut values = vec![];
        let mut failures = 0;
        println!(
            "EXPERIMENTAL {PROFILE}: all original skill levels, source class kits and legal actor opportunities"
        );
        for mode in modes {
            let mut c = if fixture {
                ContentConfig::fixture()
            } else {
                ContentConfig::full()
            };
            c.mode = mode.clone();
            c = c.with_enabled_combat_candidates()?;
            let g = GameData::load_with_content(&dd_data::game_dir(), c)?;
            let mut executed = 0;
            for class in g.hero_ids() {
                for level in 0..=4 {
                    for skill in g.hero_skill_ids(class)? {
                        match validation012::skill_probe(&g, class, level, &skill) {
                            Ok(row) => {
                                executed += 1;
                                values.push(serde_json::json!({"mode":mode,"content_id":g.rules.content.id()?,"probe":row}));
                            }
                            Err(e) => {
                                failures += 1;
                                values.push(serde_json::json!({"mode":mode,"class":class,"skill":skill,"level":level,"error":e}));
                            }
                        }
                    }
                }
            }
            println!(
                "mode={mode} classes={} executed={executed} cumulative_failures={failures}",
                g.hero_ids().count()
            );
        }
        let name = if fixture {
            "012-skill-matrix-fixture.json"
        } else {
            "012-skill-matrix-full.json"
        };
        std::fs::write(
            out.join(name),
            serde_json::to_vec_pretty(&values).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if failures > 0 {
            return Err("skill matrix contains failures; see per-case source report".into());
        }
        return Ok(());
    }
    Err("unknown closure experiment".into())
}
