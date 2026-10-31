//! Source-valued stable weekly task instances. Generation runs once at a town boundary.
use super::*;
use dd_data::campaign017::uint;
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Board {
    pub week: Option<u32>,
    pub desired: u32,
    pub generated: u32,
    pub shortfall: u32,
    pub entries: Vec<Instance>,
    pub selected: Option<String>,
    pub persistent: BTreeMap<String, Instance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Instance {
    pub id: String,
    pub seed: u64,
    pub plot: Option<String>,
    pub raw: Value,
    pub source: Value,
    pub extra_rewards: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recovered_assets: Vec<u64>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QuestView {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub estate_requirements: BTreeMap<String, u32>,
    pub preparation_eligible: bool,
    pub maximum_resolve_level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preparation_error: Option<String>,
    pub id: String,
    pub plot: Option<String>,
    pub dungeon: String,
    pub difficulty: u32,
    pub length: u32,
    pub kind: String,
    pub goals: Vec<Value>,
    pub reward: Value,
    pub extra_rewards: Vec<String>,
    /// Separate simulator capability from actual campaign unlock eligibility.
    pub execution: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recovered_equipment: Vec<(u64, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_week: Option<u32>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoardView {
    pub week: u32,
    pub desired: u32,
    pub generated: u32,
    pub shortfall: u32,
    pub quests: Vec<QuestView>,
    pub selected: Option<String>,
}
fn number(v: &Value) -> Result<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("invalid campaign source integer".into())
}
fn indexed(v: &Value, index: usize) -> Result<&Value> {
    let rows = v
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or("missing source table rows")?;
    Ok(&rows[index.min(rows.len() - 1)])
}
impl Session {
    fn region_level(&self, region: &str) -> Result<u32> {
        let xp = self.state.city.dungeon_xp.get(region).copied().unwrap_or(0);
        Ok(self
            .definition
            .data
            .value("progression.level_threshold_table")?
            .as_array()
            .ok_or("missing region level thresholds")?
            .iter()
            .filter(|v| v.as_u64().is_some_and(|n| n <= u64::from(xp)))
            .count()
            .saturating_sub(1) as u32)
    }
    pub(super) fn plot_available(&self, id: &str) -> Result<bool> {
        let q = library::unique_json(&self.definition.data.quests, id, "campaign plot quest")?;
        if q["is_enabled"] == false
            || q["is_repeatable"] != true && self.state.city.completed_ids.contains(id)
        {
            return Ok(false);
        }
        if (q["is_started_by_event"] == true || q["is_generated_by_event"] == true)
            && !self.state.plot_windows.contains_key(id)
        {
            return Ok(false);
        }
        if q["plot_quest_dependency"]
            .as_str()
            .is_some_and(|dep| !dep.is_empty() && !self.state.city.completed_ids.contains(dep))
        {
            return Ok(false);
        }
        if dd_data::retention017::quest_count(q)? > 0
            && self.retention_plot()?.as_deref() != Some(id)
        {
            return Ok(false);
        }
        let region = q["quest"]["dungeon"]
            .as_str()
            .ok_or("plot quest missing dungeon")?;
        if q["dungeon_level"].as_u64().unwrap_or(0) > u64::from(self.region_level(region)?) {
            return Ok(false);
        }
        Ok(true)
    }
    fn quest_identity(&self, plot: Option<&str>, index: usize) -> String {
        use sha2::{Digest, Sha256};
        let key = if let Some(id) = plot {
            format!("{};plot:{id}", self.state.city.estate_id)
        } else {
            format!(
                "{};week:{};random:{index}",
                self.state.city.estate_id, self.state.clock.week
            )
        };
        format!("quest:{:x}", Sha256::digest(key.as_bytes()))
    }
    fn goals_for(&mut self, kind: &str, region: &str) -> Result<Vec<String>> {
        let groups = dd_data::catalog014::combined_goal_groups(
            &self.definition.data.goal_groups,
            kind,
            region,
        )?;
        let index = self.state.rng.below(groups.len() as u64) as usize;
        let goals = groups[index].clone();
        if goals.is_empty() {
            return Err(format!("empty generated source goals {region}:{kind}"));
        }
        for id in &goals {
            library::unique_json(&self.definition.data.goals, id, "campaign goal")?;
        }
        Ok(goals)
    }
    fn extra_rewards(
        &self,
        town: &mut TownSession,
        rows: &Value,
        already: &[String],
        region: &str,
    ) -> Result<Vec<String>> {
        let mut result = vec![];
        for row in rows.as_array().ok_or("missing plot trinket rewards")? {
            let rarity = row["rarity"]
                .as_str()
                .ok_or("missing trinket reward rarity")?;
            for _ in 0..uint(row, "amount")? {
                let mut owned = already.to_vec();
                owned.extend(result.clone());
                result.push(town.draw_quest_trinket(rarity, &owned, region)?);
            }
        }
        Ok(result)
    }
    fn random_instance(
        &mut self,
        town: &mut TownSession,
        region: &str,
        row: &Value,
        difficulty: u32,
        index: usize,
        drawn: &[String],
    ) -> Result<Instance> {
        let length = uint(row, "length")?;
        let kind = row["type"].as_str().ok_or("missing generated quest type")?;
        let goals = if kind == "farm_bloodcrate" {
            vec!["gather_bloodcrate".into()]
        } else if kind == "kill_statue" {
            vec!["kill_statue_C".into()]
        } else {
            self.goals_for(kind, region)?
        };
        let data = &self.definition.data;
        let mut rewards = data
            .value("rewards.item_table")?
            .get(difficulty as usize)
            .and_then(|v| v.get(length as usize))
            .and_then(Value::as_array)
            .ok_or("missing source gold reward")?
            .clone();
        let heirlooms = &data.table(&format!("rewards.heirloom_type_map:{region}"))?["types"];
        let heirlooms = heirlooms
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("missing source heirloom types")?;
        let selected = self.state.rng.below(heirlooms.len() as u64) as usize;
        let heirloom = heirlooms[selected].as_str().ok_or("invalid heirloom ID")?;
        let amount = data.table(&format!("rewards.heirloom_amount_table:{heirloom}"))?["amounts"]
            .get(difficulty as usize)
            .and_then(|v| v.get(length as usize))
            .ok_or("missing source heirloom award")?;
        rewards.push(json!({"type":"heirloom","id":heirloom,"amount":number(amount)?}));
        let xp = data
            .value("rewards.resolve_xp_table")?
            .get(difficulty as usize)
            .and_then(|v| v.get(length as usize))
            .ok_or("missing source quest XP")?;
        let mut rarity = vec![];
        let mut weights = vec![];
        for id in data.tables.keys() {
            if !id.starts_with("rewards.trinket_chance_table:") {
                continue;
            }
            let t = library::unique_json(&data.tables, id, "quest rarity weights")?;
            if t["dungeon"].as_str().is_some_and(|d| d != region) {
                continue;
            }
            let w = t["chances"]
                .get(difficulty as usize)
                .and_then(|v| v.get(length as usize))
                .and_then(Value::as_f64)
                .ok_or("missing rarity weight")?;
            if w > 0. {
                rarity.push(t["rarity"].as_str().ok_or("invalid rarity")?.to_string());
                weights.push(w);
            }
        }
        let extra = if rarity.is_empty() {
            return Err("no positive source quest reward rarity".into());
        } else {
            let i = town.choose(&weights)?;
            vec![town.draw_quest_trinket(&rarity[i], drawn, region)?]
        };
        let items = rewards
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect::<serde_json::Map<_, _>>();
        let mut raw = json!({"dungeon":region,"type":kind,"difficulty":difficulty,"length":length,"is_plot_quest":false,"goal_ids":goals,"completion_reward":{"resolve_xp":number(xp)?,"items_definition":{"items":items,"system_config_type":"quest_rewards"}}});
        for key in ["torch_setting", "raid_rules_override"] {
            if let Some(value) = row.get(key) {
                raw[key] = value.clone();
            }
        }
        Ok(Instance {
            recovered_assets: vec![],
            id: self.quest_identity(None, index),
            seed: self.state.rng.next(),
            plot: None,
            raw,
            source: row.clone(),
            extra_rewards: extra,
        })
    }
    pub(super) fn refresh_board(&mut self) -> Result<()> {
        if self.state.pending.is_some() || self.state.board.week == Some(self.state.clock.week) {
            return Ok(());
        }
        if self.definition.data.tables.is_empty() {
            self.state.board = Board {
                week: Some(self.state.clock.week),
                ..Default::default()
            };
            return Ok(());
        }
        let mut town = self.town();
        let mut board = Board {
            week: Some(self.state.clock.week),
            persistent: self.state.board.persistent.clone(),
            ..Default::default()
        };
        let mut extras = vec![];
        let plots = self
            .definition
            .data
            .quests
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for id in &plots {
            if !self.plot_available(id)? {
                continue;
            }
            let q =
                library::unique_json(&self.definition.data.quests, id, "campaign plot")?.clone();
            let instance = if let Some(active) = board.persistent.get(id) {
                active.clone()
            } else {
                Instance {
                    recovered_assets: self.draw_retention_assets(&q)?,
                    id: self.quest_identity(Some(id), 0),
                    seed: self.state.rng.next(),
                    plot: Some(id.clone()),
                    raw: q["quest"].clone(),
                    source: q.clone(),
                    extra_rewards: self.extra_rewards(
                        &mut town,
                        &q["additional_trinket_completion_rewards"],
                        &extras,
                        q["quest"]["dungeon"]
                            .as_str()
                            .ok_or("missing plot reward region")?,
                    )?,
                }
            };
            extras.extend(instance.extra_rewards.clone());
            board.entries.push(instance);
        }
        let index = self.state.clock.returns as usize;
        let mut wanted = number(indexed(
            self.definition
                .data
                .value("number.number_of_quests_per_town_visit_table")?,
            index,
        )?)?;
        for row in self.definition.data.modifiers.values() {
            wanted = wanted
                .checked_add(number(indexed(&row["value"], index)?)?)
                .ok_or("board count overflow")?;
        }
        if wanted > 10000 {
            return Err("source board count exceeds bound".into());
        }
        board.desired = wanted;
        let cap = number(
            self.definition
                .data
                .value("dungeon.generated_quests_max_threshold")?,
        )?;
        let mut regions = vec![];
        for (key, value) in &self.definition.data.tables {
            if !key.starts_with("dungeon.generated_dungeons:") {
                continue;
            }
            let region =
                library::unique_json(&self.definition.data.tables, key, "generated dungeon")?;
            if self.state.clock.completions < uint(region, "required_number_of_quests_finished")?
                || region["required_plot_quest"]
                    .as_str()
                    .is_some_and(|id| !self.state.city.completed_ids.contains(id))
            {
                continue;
            }
            let id = value["id"].as_str().ok_or("missing region ID")?;
            let table = self
                .definition
                .data
                .table(&format!("type.available_quests_table:{id}"))?;
            let rows = indexed(
                &table["generated_quest_table"],
                self.region_level(id)? as usize,
            )?
            .as_array()
            .ok_or("invalid generated quest choices")?
            .iter()
            .filter(|r| {
                r["required_plot_quest_completed"]
                    .as_str()
                    .is_none_or(|q| self.state.city.completed_ids.contains(q))
                    && r["required_plot_quest_not_completed"]
                        .as_str()
                        .is_none_or(|q| !self.state.city.completed_ids.contains(q))
            })
            .cloned()
            .collect::<Vec<_>>();
            if !rows.is_empty() {
                regions.push((id.to_string(), rows));
            }
        }
        let mut difficulties = vec![];
        let mut weights = vec![];
        for row in self
            .definition
            .data
            .value("difficulty.generated_resolve_level_difficulties")?
            .as_array()
            .ok_or("missing difficulty buckets")?
        {
            let count = town
                .state
                .heroes
                .iter()
                .filter(|h| {
                    h.state.alive
                        && row["resolve_levels"].as_array().is_some_and(|a| {
                            town.level(h.xp)
                                .is_ok_and(|l| a.iter().any(|n| n.as_u64() == Some(u64::from(l))))
                        })
                })
                .count();
            if count > 0 {
                difficulties.push(uint(row, "difficulty")?);
                weights.push(count as f64);
            }
        }
        // Empty living roster still exposes a source novice recovery mission, never a fabricated hire.
        if difficulties.is_empty() {
            let row = &self
                .definition
                .data
                .value("difficulty.generated_resolve_level_difficulties")?[0];
            difficulties.push(uint(row, "difficulty")?);
            weights.push(1.);
        }
        let counted = board
            .entries
            .iter()
            .filter(|e| e.raw["counted_in_generation"] != false)
            .count() as u32;
        let mut per_region = BTreeMap::<String, u32>::new();
        let target = wanted.saturating_sub(counted);
        for i in 0..target {
            let available = regions
                .iter()
                .enumerate()
                .filter(|(idx, (region, _))| {
                    per_region.get(region).copied().unwrap_or(0) < cap
                        && (i as usize >= regions.len() || *idx == i as usize)
                })
                .map(|(idx, _)| idx)
                .collect::<Vec<_>>();
            if available.is_empty() {
                break;
            }
            let chosen = available[self.state.rng.below(available.len() as u64) as usize];
            let (region, rows) = &regions[chosen];
            let row_weights = rows
                .iter()
                .map(|r| {
                    r["chance"]
                        .as_f64()
                        .ok_or_else(|| "missing quest type weight".into())
                })
                .collect::<Result<Vec<_>>>()?;
            let row = &rows[town.choose(&row_weights)?];
            let difficulty = if let Some(n) = row.get("difficulty_override") {
                number(n)?
            } else {
                difficulties[town.choose(&weights)?]
            };
            let instance =
                self.random_instance(&mut town, region, row, difficulty, i as usize, &extras)?;
            extras.extend(instance.extra_rewards.clone());
            board.entries.push(instance);
            *per_region.entry(region.clone()).or_default() += 1;
            board.generated += 1;
        }
        board.shortfall = target.saturating_sub(board.generated);
        self.state.city = town.state;
        self.state.board = board;
        Ok(())
    }
    pub fn select_quest(&mut self, id: Option<String>) -> Result<()> {
        self.transaction(|s| {
            if s.failure()?.is_some() {
                return Err("campaign has ended".into());
            }
            if s.state.pending.is_some() || s.raid_busy() {
                return Err("quest selection unavailable".into());
            }
            if id
                .as_ref()
                .is_some_and(|id| !s.state.board.entries.iter().any(|q| &q.id == id))
            {
                return Err("quest not on this board".into());
            }
            s.state.board.selected = id;
            Ok(())
        })
    }
    pub(super) fn board_view(&self) -> Result<Option<BoardView>> {
        if self.state.pending.is_some() {
            return Ok(None);
        }
        let b = &self.state.board;
        let Some(week) = b.week else { return Ok(None) };
        let mut quests = vec![];
        for q in &b.entries {
            let region = q.raw["dungeon"].as_str().ok_or("missing quest dungeon")?;
            let preparation_error = if self.execution(q) == "expedition integration pending" {
                Some("quest has no accepted expedition executor yet".into())
            } else {
                self.preparation_eligibility(q).err()
            };
            let mut estate_requirements = BTreeMap::new();
            if self.definition.automation.is_some()
                && let Some(items) =
                    q.source["estate_inventory_dependency"]["items_definition"]["items"].as_object()
            {
                for item in items.values() {
                    let key = dd_data::inventory015::Item::new(
                        item["type"]
                            .as_str()
                            .ok_or("missing estate dependency kind")?,
                        item["id"].as_str().ok_or("missing estate dependency ID")?,
                    )
                    .key();
                    let amount = estate_requirements.entry(key).or_insert(0u32);
                    *amount = amount
                        .checked_add(uint(item, "amount")?)
                        .ok_or("estate dependency overflow")?;
                }
            }
            quests.push(QuestView {
                estate_requirements,
                preparation_eligible: preparation_error.is_none(),
                maximum_resolve_level: self.quest_resolve_limit(q)?,
                preparation_error,
                id: q.id.clone(),
                plot: q.plot.clone(),
                dungeon: region.into(),
                difficulty: uint(&q.raw, "difficulty")?,
                length: uint(&q.raw, "length")?,
                kind: q.raw["type"].as_str().ok_or("missing quest type")?.into(),
                goals: q.raw["goal_ids"]
                    .as_array()
                    .ok_or("missing quest goals")?
                    .iter()
                    .map(|id| {
                        library::unique_json(
                            &self.definition.data.goals,
                            id.as_str().ok_or("invalid goal")?,
                            "board goal",
                        )
                        .cloned()
                    })
                    .collect::<Result<_>>()?,
                reward: q.raw["completion_reward"].clone(),
                extra_rewards: q.extra_rewards.clone(),
                recovered_equipment: q
                    .recovered_assets
                    .iter()
                    .map(|id| {
                        self.state
                            .lost_assets
                            .iter()
                            .find(|a| a.instance == *id)
                            .map(|a| (*id, a.state.definition.id.clone()))
                            .ok_or("board recovery asset missing".into())
                    })
                    .collect::<Result<_>>()?,
                execution: self.execution(q).into(),
                expires_week: q
                    .plot
                    .as_ref()
                    .and_then(|id| self.state.plot_windows.get(id))
                    .and_then(|w| w.expires),
            });
        }
        Ok(Some(BoardView {
            week,
            desired: b.desired,
            generated: b.generated,
            shortfall: b.shortfall,
            quests,
            selected: b.selected.clone(),
        }))
    }
    fn validate_generated(&self, q: &Instance) -> Result<()> {
        dd_data::campaign017::fields(
            &q.raw,
            &[
                "dungeon",
                "type",
                "difficulty",
                "length",
                "is_plot_quest",
                "goal_ids",
                "completion_reward",
                "torch_setting",
                "raid_rules_override",
            ],
            "generated board quest",
        )?;
        if q.raw["is_plot_quest"] != false {
            return Err("random board quest has plot flag".into());
        }
        if !q.recovered_assets.is_empty() {
            return Err("random quest has physical recovery rewards".into());
        }
        let region = q.raw["dungeon"].as_str().ok_or("missing saved dungeon")?;
        let kind = q.raw["type"].as_str().ok_or("missing saved quest type")?;
        let difficulty = uint(&q.raw, "difficulty")? as usize;
        let length = uint(&q.raw, "length")? as usize;
        let data = &self.definition.data;
        let table = data.table(&format!("type.available_quests_table:{region}"))?;
        if !table["generated_quest_table"]
            .as_array()
            .ok_or("missing source generation")?
            .iter()
            .filter_map(Value::as_array)
            .flatten()
            .any(|v| v == &q.source)
            || q.raw["length"] != q.source["length"]
            || q.raw["type"] != q.source["type"]
        {
            return Err("saved random quest is not a source row".into());
        }
        if let Some(value) = q.source.get("difficulty_override") {
            if value != &q.raw["difficulty"] {
                return Err("saved difficulty override differs".into());
            }
        } else if !data
            .value("difficulty.generated_resolve_level_difficulties")?
            .as_array()
            .ok_or("missing difficulty source")?
            .iter()
            .any(|v| v["difficulty"] == q.raw["difficulty"])
        {
            return Err("saved difficulty not in source buckets".into());
        }
        for key in ["torch_setting", "raid_rules_override"] {
            if q.raw.get(key) != q.source.get(key) {
                return Err("saved quest setting differs".into());
            }
        }
        let options = match kind {
            "farm_bloodcrate" => vec![vec!["gather_bloodcrate".into()]],
            "kill_statue" => vec![vec!["kill_statue_C".into()]],
            _ => dd_data::catalog014::combined_goal_groups(&data.goal_groups, kind, region)?,
        };
        let goals: Vec<String> =
            serde_json::from_value(q.raw["goal_ids"].clone()).map_err(|e| e.to_string())?;
        if !options.contains(&goals) {
            return Err("saved quest goals differ from source alternatives".into());
        }
        let expected_xp = data
            .value("rewards.resolve_xp_table")?
            .get(difficulty)
            .and_then(|v| v.get(length))
            .ok_or("missing saved quest XP source")?;
        let base = data
            .value("rewards.item_table")?
            .get(difficulty)
            .and_then(|v| v.get(length))
            .and_then(Value::as_array)
            .ok_or("missing saved reward source")?;
        let mut matched = false;
        for heirloom in data.table(&format!("rewards.heirloom_type_map:{region}"))?["types"]
            .as_array()
            .ok_or("missing heirloom source")?
        {
            let id = heirloom.as_str().ok_or("bad heirloom ID")?;
            let amount = data.table(&format!("rewards.heirloom_amount_table:{id}"))?["amounts"]
                .get(difficulty)
                .and_then(|v| v.get(length))
                .ok_or("missing heirloom amount")?;
            let mut items = base.clone();
            items.push(json!({"type":"heirloom","id":id,"amount":amount}));
            let items = items
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v))
                .collect::<serde_json::Map<_, _>>();
            if q.raw["completion_reward"]
                == json!({"resolve_xp":expected_xp,"items_definition":{"system_config_type":"quest_rewards","items":items}})
            {
                matched = true;
            }
        }
        if !matched {
            return Err("saved quest reward differs from source".into());
        }
        if q.extra_rewards.len() != 1 {
            return Err("random quest must retain one frozen source rarity reward".into());
        }
        let raw = library::unique_json(
            &self.definition.town.data.base.curios.trinkets,
            &q.extra_rewards[0],
            "board reward trinket",
        )?;
        let mut valid = false;
        for (id, t) in &data.tables {
            if id.starts_with("rewards.trinket_chance_table:")
                && t["rarity"] == raw["rarity"]
                && t["dungeon"].as_str().is_none_or(|d| d == region)
                && t["chances"]
                    .get(difficulty)
                    .and_then(|v| v.get(length))
                    .and_then(Value::as_f64)
                    .is_some_and(|n| n > 0.)
            {
                valid = true;
            }
        }
        if !valid {
            return Err("saved trinket rarity is outside source reward pool".into());
        }
        Ok(())
    }
    pub(super) fn validate_board(&self) -> Result<()> {
        let b = &self.state.board;
        if b.week.is_some_and(|w| w > self.state.clock.week)
            || b.entries.len() > 10000
            || b.selected
                .as_ref()
                .is_some_and(|id| !b.entries.iter().any(|q| &q.id == id))
            || b.generated as usize != b.entries.iter().filter(|q| q.plot.is_none()).count()
        {
            return Err("invalid campaign board boundary".into());
        }
        if b.entries
            .iter()
            .map(|q| &q.id)
            .collect::<BTreeSet<_>>()
            .len()
            != b.entries.len()
        {
            return Err("duplicate weekly quest instance".into());
        }
        for (key, q) in &b.persistent {
            if q.plot.as_deref() != Some(key) || !b.entries.iter().any(|entry| entry == q) {
                return Err("persistent quest missing or differs on current board".into());
            }
        }
        let mut ids = BTreeSet::new();
        for q in b.entries.iter().chain(b.persistent.values()) {
            let persistent = b.persistent.values().any(|p| p.id == q.id);
            if !ids.insert(&q.id) && !persistent {
                return Err("duplicate board instance".into());
            }
            if let Some(plot) = &q.plot {
                let original =
                    library::unique_json(&self.definition.data.quests, plot, "saved plot")?;
                if &q.source != original
                    || q.raw != original["quest"]
                    || q.id != self.quest_identity(Some(plot), 0)
                {
                    return Err("saved plot differs from source".into());
                }
                if b.persistent.contains_key(plot)
                    && original["is_persistent_through_campaign"] != true
                {
                    return Err("nonpersistent source quest stored as persistent".into());
                }
                let recover = dd_data::retention017::quest_count(original)?;
                if q.recovered_assets.len() != recover
                    || q.recovered_assets.iter().collect::<BTreeSet<_>>().len() != recover
                    || q.recovered_assets
                        .iter()
                        .any(|id| !self.state.lost_assets.iter().any(|a| a.instance == *id))
                {
                    return Err("invalid frozen physical recovery rewards".into());
                }
                let mut expected = BTreeMap::<String, u32>::new();
                for row in original["additional_trinket_completion_rewards"]
                    .as_array()
                    .ok_or("missing plot bonus rewards")?
                {
                    let rarity = row["rarity"].as_str().ok_or("missing rarity")?;
                    let n = expected.entry(rarity.into()).or_default();
                    *n = n
                        .checked_add(uint(row, "amount")?)
                        .ok_or("reward amount overflow")?;
                }
                for id in &q.extra_rewards {
                    let raw = library::unique_json(
                        &self.definition.town.data.base.curios.trinkets,
                        id,
                        "saved plot reward",
                    )?;
                    let count = expected
                        .get_mut(raw["rarity"].as_str().ok_or("missing trinket rarity")?)
                        .ok_or("wrong plot reward rarity")?;
                    *count = count
                        .checked_sub(1)
                        .ok_or("too many plot reward trinkets")?;
                }
                if expected.values().any(|n| *n != 0) {
                    return Err("missing frozen plot reward trinket".into());
                }
            } else {
                self.validate_generated(q)?;
            }
            for id in q.raw["goal_ids"].as_array().ok_or("missing saved goals")? {
                library::unique_json(
                    &self.definition.data.goals,
                    id.as_str().ok_or("invalid saved goal")?,
                    "saved quest goal",
                )?;
            }
            for id in &q.extra_rewards {
                self.definition.town.trinket(id)?;
            }
        }
        Ok(())
    }
}
