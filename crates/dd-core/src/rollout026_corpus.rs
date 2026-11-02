//! Imports only historic public replay fields, with an explicit coverage mask.
use super::*;
pub fn raw_input(game: &GameData, raw: &Value) -> Result<PublicBattle> {
    let units: Vec<VisibleUnit> =
        serde_json::from_value(raw["units"].clone()).map_err(|e| e.to_string())?;
    // Historical summaries omitted the visible Prophet rank telegraph. Do not
    // invent one from the true RNG or pretend it was empty.
    if units.iter().any(|u| u.class.starts_with("prophet_")) {
        return Err("historical_rank_telegraph_not_recorded".into());
    }
    input_from_frame(game, raw, units)
}
pub fn input_from_frame(
    game: &GameData,
    raw: &Value,
    units: Vec<VisibleUnit>,
) -> Result<PublicBattle> {
    let frame = &raw["frame"];
    let mut heroes = BTreeMap::new();
    let mappings = frame["battle_heroes"]
        .as_array()
        .ok_or("historical_deployment_bridge_absent")?;
    for u in units.iter().filter(|u| u.hero) {
        let guid = mappings
            .iter()
            .find(|v| v["unit_id"].as_u64() == Some(u.id as u64))
            .and_then(|v| v["guid"].as_u64())
            .ok_or("historical_visible_guid_absent")?;
        let h = frame["roster"]
            .as_array()
            .and_then(|hs| hs.iter().find(|h| h["guid"].as_u64() == Some(guid)))
            .ok_or("historical_roster_hero_absent")?;
        let build = frame["party_build"]
            .as_array()
            .and_then(|hs| hs.iter().find(|h| h["guid"].as_u64() == Some(guid)))
            .ok_or("historical_training_absent")?;
        let skills = build["selected"]
            .as_array()
            .ok_or("historical_selected_skills_absent")?
            .iter()
            .map(|s| {
                let id = s.as_str().ok_or("historical_bad_skill_id")?;
                let rank = build["skills"][id]
                    .as_i64()
                    .ok_or("historical_skill_rank_absent")?;
                Ok((
                    id.to_string(),
                    i32::try_from(rank).map_err(|e| e.to_string())?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let grown_equipment = build["equipped_current023"].as_array().is_some_and(|a| {
            a.iter()
                .any(|v| v["growth"].as_array().is_some_and(|g| !g.is_empty()))
        });
        heroes.insert(
            u.id,
            HeroTraining {
                class: u.class.clone(),
                weapon: build["weapon"].as_u64().ok_or("historical_weapon_absent")? as usize,
                armour: build["armour"].as_u64().ok_or("historical_armour_absent")? as usize,
                skills,
                level: h["level"].as_u64().ok_or("historical_level_absent")? as u32,
                grown_equipment,
            },
        );
    }
    let mut supplies = BTreeMap::<String, u32>::new();
    for slot in frame["inventory"]["slots"]
        .as_array()
        .ok_or("historical_inventory_absent")?
        .iter()
        .filter(|v| !v.is_null())
    {
        let item = &slot["item"];
        if item["kind"] == "supply" || (item["kind"] == "estate" && item["id"] == "the_blood") {
            let id = item["id"].as_str().ok_or("historical_supply_id_absent")?;
            *supplies.entry(id.into()).or_default() += slot["amount"]
                .as_u64()
                .ok_or("historical_supply_count_absent")?
                as u32;
        }
    }
    Ok(PublicBattle {
        content: game.overlay.config.clone(),
        observation: Observation {
            candidate_id: game.rules.candidates.identity()?,
            content_id: game.rules.content.id()?,
            farm: None,
            player_selection: false,
            rank_targets: BTreeMap::new(),
            shared_health: vec![],
            rank_burns: vec![],
            torch: raw["torch"].as_u64().map(|t| t as u8),
            wave: 1,
            supplies: supplies.into_iter().collect(),
            round: raw["round"].as_u64().ok_or("historical_round_absent")? as u32,
            actor: raw["actor"].as_u64().map(|id| id as usize),
            units,
            actions: vec![],
            outcome: None,
        },
        heroes,
        region: frame["quest"]["region"].as_str().map(str::to_string),
        difficulty: frame["quest"]["difficulty"]
            .as_u64()
            .ok_or("historical_difficulty_absent")? as u32,
        corridor: frame["corridor"].as_bool().unwrap_or(false),
        plot: frame["quest"]["plot"].as_str().map(str::to_string),
        retreat_legal: raw["retreat_legal"].as_bool(),
        mechanisms: execution025::Options::candidate(true),
        single_target_fallback: true,
    })
}
pub fn prepare(raw: &std::path::Path, out: &std::path::Path) -> Result<Value> {
    use std::fs;
    let index: Value =
        serde_json::from_slice(&fs::read(raw.join("index.json")).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if out.join("index.json").exists() {
        return Err("refuse to replace prepared026 corpus".into());
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let game = GameData::load_with_content(&dd_data::game_dir(), full_content()?)?;
    let mut rows = vec![];
    let mut counts = BTreeMap::<String, usize>::new();
    let mut seen = BTreeMap::<String, String>::new();
    for source in index["rows"].as_array().ok_or("bad raw026 corpus index")? {
        let path = std::path::Path::new(source["file"].as_str().ok_or("missing raw scene path")?);
        let raw: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let result = raw_input(&game, &raw).and_then(|input| {
            Definitions::project(&game, &input).and_then(|defs| complete_historical(&input, &defs))
        });
        let mut row = source.clone();
        match result {
            Ok(input) => {
                let unique = serde_json::to_string(&input).map_err(|e| e.to_string())?;
                let duplicate = seen.get(&unique).cloned();
                if duplicate.is_none() {
                    seen.insert(unique, source["id"].as_str().unwrap().to_string());
                }
                {
                    let id = source["id"].as_str().unwrap();
                    let file = out.join(format!("{id}.json"));
                    let scene = serde_json::json!({"input":input,"source":source,"coverage":{"historical_exact":["units (every recorded field)","round","actor","torch"],"source_completed":["actions","candidate_id","content_id","supplies","wave","empty unrecorded special-state fields"],"full_original_observation_available":false}});
                    fs::write(
                        &file,
                        serde_json::to_vec_pretty(&scene).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    row["file"] = serde_json::json!(file);
                    row["status"] = serde_json::json!("PreparedPartialHistorical");
                    row["duplicate_public_of"] = serde_json::json!(duplicate);
                    row["length"] = raw["frame"]["quest"]["length"].clone();
                    *counts
                        .entry(source["category"].as_str().unwrap().into())
                        .or_default() += 1;
                }
            }
            Err(e) => {
                row["status"] = serde_json::json!("RejectedBeforeEvaluation");
                row["reason"] = serde_json::json!(e);
            }
        }
        rows.push(row);
    }
    let result = serde_json::json!({"rows":rows,"prepared":counts,"kind":"historical observed-unit roundtrip; not a claim that omitted original fields were checked","raw":raw});
    fs::write(
        out.join("index.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(result)
}
