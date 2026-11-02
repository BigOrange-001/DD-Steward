//! Host-only synthetic estate presets. These assets are never normal campaign progress.
use super::*;
impl Runtime027 {
    pub fn test_start(&mut self, kind: &str) -> Result<()> {
        if kind == "old-road" {
            return Ok(());
        }
        if !["veteran", "champion"].contains(&kind) {
            return Err("unknown027 start".into());
        }
        let runtime = &mut self.inner.inner.replay.inner.inner.inner;
        if runtime.decisions != 0 {
            return Err("test start only allowed before first decision".into());
        }
        // Use the existing validated opening import bridge, then replace only synthetic estate assets.
        let mut opening = crate::opening015::Session::new(
            runtime.definition.town.opening.clone(),
            runtime.config.seed,
        )?;
        for _ in 0..10000 {
            let o = opening.observe()?;
            if o.phase == crate::opening015::Phase::Finished {
                break;
            }
            opening.act(crate::opening_policy015::choose(&o)?)?;
        }
        let mut campaign = crate::campaign017::Session::from_opening(
            runtime.definition.clone(),
            opening.town_handoff()?,
            runtime.config.seed,
        )?;
        let mut town = campaign.town();
        let champion = kind == "champion";
        let preset: Value = serde_json::from_str(r#"{
  "version": "candidate-estate-start-027-v1",
  "gold": 100000,
  "veteran_level": 3,
  "champion_class_trinkets": 3,
  "champion_general_per_rarity": 2,
  "omit_ancestral": true,
  "simplification": "bounded rich collection; full inventory preset stalls estate planning; no changes to native economy or mechanics"
}
"#)
            .map_err(|e| e.to_string())?;
        let maximum = town.definition.data.base.roster_variables["resolve_level_thresholds"]
            .as_array()
            .ok_or("027 resolve thresholds absent")?
            .len()
            .saturating_sub(1) as u32;
        let level = if champion {
            maximum
        } else {
            (preset["veteran_level"]
                .as_u64()
                .ok_or("027 start level absent")? as u32)
                .min(maximum)
        };
        town.state.heroes = vec![].into();
        town.state.assets = vec![].into();
        town.state.party.clear();
        town.state.recruits = vec![].into();
        town.state.jobs.clear();
        town.state.purchases.clear();
        town.state.districts.clear();
        let profile = crate::preparation024::Profile::documented()?;
        let classes = town.definition.game.hero_ids().cloned().collect::<Vec<_>>();
        for class in classes {
            let mut hero = town.generate(&class, level, 0, 0, 0, 0)?;
            // Learn every source-defined skill; selected bar follows the established default where available.
            for id in town.definition.game.hero_skill_ids(&class)? {
                let tree = town.definition.data.tree(&format!("{class}.{id}"))?;
                let rank = tree["requirements"]
                    .as_array()
                    .ok_or("027 skill requirements absent")?
                    .iter()
                    .filter(|n| {
                        n["prerequisite_resolve_level"]
                            .as_u64()
                            .is_some_and(|n| n <= level as u64)
                    })
                    .filter_map(|n| n["code"].as_str()?.parse::<i32>().ok())
                    .max()
                    .ok_or("027 no skill tier")?;
                hero.skills.insert(id, rank);
            }
            if let Some(loadout) = profile.loadouts.iter().find(|l| l.class_id == class) {
                hero.selected = loadout.skills.clone();
            }
            let doc = town.definition.game.hero_document(&class)?;
            if doc
                .records_keyed("skill_selection")
                .next()
                .is_some_and(|r| {
                    dd_data::combat::text(r, "can_select_combat_skills").ok() == Some("false")
                })
            {
                hero.selected = town.definition.game.hero_skill_ids(&class)?;
            }
            town.install_hero_purchases(&hero)?;
            town.state.heroes.push(hero);
        }
        let trees = town.definition.data.trees.clone();
        for (id, tree) in trees.iter().filter(|(_, t)| t["is_instanced"] == false) {
            let nodes = tree["requirements"]
                .as_array()
                .ok_or("027 building nodes absent")?;
            let cap = if champion {
                nodes.len()
            } else {
                (nodes.len() * 3).div_ceil(4)
            };
            for n in nodes.iter().take(cap) {
                town.state.purchases.insert((
                    None,
                    id.clone(),
                    n["code"].as_str().ok_or("027 upgrade code absent")?.into(),
                ));
            }
        }
        if champion {
            town.state.districts.insert("illuminators_guild".into());
        }
        town.state.wallet.insert(
            town.currency("gold")?,
            preset["gold"].as_u64().ok_or("027 start gold absent")? as u32,
        );
        town.state.highest_dungeon = if champion { 5 } else { 3 };
        // Source plot thresholds determine regional XP, avoiding handwritten region progression.
        let target = if champion { 5 } else { 3 };
        let mut levels = BTreeMap::<String, usize>::new();
        for (id, q) in &campaign.definition.data.quests {
            let region = q["quest"]["dungeon"].as_str().unwrap_or("");
            if !["crypts", "warrens", "weald", "cove"].contains(&region) {
                continue;
            }
            let d = q["quest"]["difficulty"].as_u64().unwrap_or(0);
            if d < target && q["is_enabled"] != false {
                town.state.completed_ids.insert(id.clone());
            }
            if d == target {
                let x = levels.entry(region.into()).or_default();
                *x = (*x).max(q["dungeon_level"].as_u64().unwrap_or(0) as usize);
            }
        }
        let region_thresholds = campaign
            .definition
            .data
            .value("progression.level_threshold_table")?;
        for (region, level) in levels {
            let xp = region_thresholds[level]
                .as_u64()
                .ok_or("027 region threshold absent")? as u32;
            town.state.dungeon_xp.insert(region, xp);
        }
        // All trinkets are filtered through the enabled inventory catalogue and original rarity data.
        let items = town.definition.data.base.bag_rules.trinkets.clone();
        let mut rarities = BTreeSet::new();
        let mut trinket_counts = BTreeMap::<String, usize>::new();
        for (id, raw) in &items {
            let rarity = raw["rarity"].as_str().ok_or("027 trinket rarity absent")?;
            if raw["is_currency"] == true || (champion && rarity == "ancestral") {
                continue;
            }
            let class = raw["hero_class_requirements"]
                .as_array()
                .and_then(|xs| xs.first())
                .and_then(Value::as_str);
            let group = class
                .map(str::to_string)
                .unwrap_or_else(|| format!("general:{rarity}"));
            let cap = preset[if class.is_some() {
                "champion_class_trinkets"
            } else {
                "champion_general_per_rarity"
            }]
            .as_u64()
            .ok_or("027 trinket budget absent")? as usize;
            let count = trinket_counts.entry(group).or_default();
            let include = if champion {
                *count < cap
            } else {
                ["common", "rare"].contains(&rarity) && rarities.insert(rarity.to_string())
            };
            if include {
                town.add_asset(id, None)?;
                *count += 1;
            }
        }
        town.validate()?;
        campaign.state.city = town.state;
        campaign.state.board = Default::default();
        campaign.refresh_board()?;
        campaign.state.board.entries.retain(|q| {
            ["crypts", "warrens", "weald", "cove"]
                .contains(&q.raw["dungeon"].as_str().unwrap_or(""))
        });
        let ids = campaign
            .state
            .board
            .entries
            .iter()
            .map(|q| q.id.clone())
            .collect::<BTreeSet<_>>();
        campaign
            .state
            .board
            .persistent
            .retain(|_, q| ids.contains(&q.id));
        campaign.state.board.generated = campaign
            .state
            .board
            .entries
            .iter()
            .filter(|q| q.plot.is_none())
            .count() as u32;
        campaign.validate()?;
        runtime.actor = Actor::Campaign(Box::new(campaign));
        runtime.cached = None;
        self.options.start = kind.into();
        if let View::Campaign(o) = runtime.observe()? {
            let progress = s25::caretaker::progress(&o)?;
            self.options.baseline_goals = progress["goals"]
                .as_array()
                .ok_or("027 goals absent")?
                .iter()
                .filter(|g| g["done"] == true)
                .filter_map(|g| g["id"].as_str().map(str::to_string))
                .collect();
        }
        Ok(())
    }
}
