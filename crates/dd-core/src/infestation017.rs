//! Source sequence timing, distinct from returns and completed missions.
use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Clock {
    remaining: Option<u32>,
    week: Option<u32>,
    pub suppressed: BTreeSet<String>,
}
impl Session {
    fn infestation_rows(&self) -> Result<&Vec<Value>> {
        self.definition.data.infestation["sequence_elements"]
            .as_array()
            .ok_or("missing enabled infestation sequence")
            .map_err(str::to_string)
    }
    fn infestation_index(&self) -> Result<usize> {
        let rows = self.infestation_rows()?;
        let indices = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r["id"] == self.state.infestation)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if indices.len() != 1 {
            return Err("missing/ambiguous infestation stage".into());
        }
        Ok(indices[0])
    }
    pub(super) fn infestation_activity(&self) -> Result<String> {
        if self.state.infestation.is_empty() {
            return Ok(String::new());
        }
        self.infestation_rows()?[self.infestation_index()?]["activity_level"]
            .as_str()
            .map(str::to_string)
            .ok_or("missing infestation activity".into())
    }
    fn infestation_duration(&mut self, index: usize) -> Result<u32> {
        let row = &self.infestation_rows()?[index];
        let lo = dd_data::campaign017::uint(row, "minimum_duration_week")?;
        let hi = dd_data::campaign017::uint(row, "maximum_duration_week")?;
        if lo > hi || hi > 1000000 {
            return Err("invalid infestation duration bounds".into());
        }
        Ok(lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32)
    }
    pub(super) fn advance_infestation(&mut self, advance: bool) -> Result<()> {
        if self.state.infestation.is_empty() {
            return Ok(());
        }
        let mut index = self.infestation_index()?;
        if self.state.infestation_clock.remaining.is_none() {
            self.state.infestation_clock.remaining = Some(self.infestation_duration(index)?);
            self.state.infestation_clock.week = Some(self.state.clock.week);
        }
        if advance {
            if self
                .state
                .infestation_clock
                .week
                .and_then(|w| w.checked_add(1))
                != Some(self.state.clock.week)
            {
                return Err("duplicate/skipped infestation week".into());
            }
            self.state.infestation_clock.week = Some(self.state.clock.week);
            let n = self.state.infestation_clock.remaining.as_mut().unwrap();
            *n = n.saturating_sub(1);
        }
        let allowed = |r: &Value, s: &Session| -> Result<bool> {
            let plot = r["plot_quest_dependency"]
                .as_str()
                .ok_or("missing infestation plot dependency")?;
            let event = r["town_event_dependency"]
                .as_str()
                .ok_or("missing infestation event dependency")?;
            Ok(
                (plot.is_empty() || s.state.city.completed_ids.contains(plot))
                    && (event.is_empty() || s.state.events.occurred(event)),
            )
        };
        // Completed story entry points supersede an earlier still-timed sequence.
        let mut future = None;
        for (i, row) in self.infestation_rows()?.iter().enumerate().skip(index + 1) {
            if row["plot_quest_dependency"]
                .as_str()
                .is_some_and(|p| !p.is_empty())
                && allowed(row, self)?
            {
                future = Some(i);
            }
        }
        if let Some(i) = future {
            index = i;
            self.enter_infestation(index)?;
        }
        for _ in 0..self.infestation_rows()?.len() {
            if self.state.infestation_clock.remaining != Some(0)
                || index + 1 >= self.infestation_rows()?.len()
                || !allowed(&self.infestation_rows()?[index + 1], self)?
            {
                break;
            }
            index += 1;
            self.enter_infestation(index)?;
        }
        Ok(())
    }
    fn enter_infestation(&mut self, index: usize) -> Result<()> {
        let row = self.infestation_rows()?[index].clone();
        self.state.infestation = row["id"].as_str().ok_or("missing infestation ID")?.into();
        self.state.infestation_clock.remaining = Some(self.infestation_duration(index)?);
        if row["clear_additional_mashes"]
            .as_bool()
            .ok_or("missing infestation clear flag")?
        {
            self.state.infestation_clock.suppressed.clear();
        }
        self.state
            .log
            .push(json!({"infestation":self.state.infestation,"week":self.state.clock.week}));
        Ok(())
    }
    pub(super) fn validate_infestation(&self) -> Result<()> {
        if self.state.infestation.is_empty() {
            if self.state.infestation_clock != Clock::default() {
                return Err("infestation state with disabled content".into());
            }
            return Ok(());
        }
        let row = &self.infestation_rows()?[self.infestation_index()?];
        if self.state.infestation_clock.week != Some(self.state.clock.week)
            || self.state.infestation_clock.remaining.is_none_or(|n| {
                row["maximum_duration_week"]
                    .as_u64()
                    .is_none_or(|max| u64::from(n) > max)
            })
        {
            return Err("infestation timer differs from source/current week".into());
        }
        for class in &self.state.infestation_clock.suppressed {
            if !self
                .definition
                .town
                .game
                .monster(class)?
                .mechanics
                .boss
                .end_infestation
            {
                return Err("unflagged infestation suppression receipt".into());
            }
        }
        Ok(())
    }
    pub(super) fn infest_map(
        &self,
        map: &mut dd_data::maps014::Map,
        q: &board::Instance,
    ) -> Result<()> {
        if self.state.infestation.is_empty() {
            return Ok(());
        }
        let activity = self.infestation_activity()?;
        let rows = self.definition.data.infestation["activity_levels"]
            .as_array()
            .ok_or("missing infestation activities")?;
        let rows = rows
            .iter()
            .filter(|r| r["id"] == activity)
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            return Err("missing/ambiguous infestation activity".into());
        }
        let row = rows[0];
        if !row["dungeons"]
            .as_array()
            .ok_or("missing infested regions")?
            .iter()
            .any(|id| *id == q.raw["dungeon"])
        {
            return Ok(());
        }
        let key = match dd_data::campaign017::uint(&q.raw, "length")? {
            1 => "short_curio_count",
            2 => "med_curio_count",
            3 => "long_curio_count",
            _ => return Err("unsupported infestation quest length".into()),
        };
        let bounds = row[key]
            .as_array()
            .filter(|a| a.len() == 2)
            .ok_or("missing egg count range")?;
        let lo = bounds[0].as_u64().ok_or("invalid egg count lower bound")?;
        let hi = bounds[1].as_u64().ok_or("invalid egg count upper bound")?;
        if lo > hi || hi > 10000 {
            return Err("invalid egg count range".into());
        }
        let mut rng = Rng(q.seed);
        let count = (lo + rng.below(hi - lo + 1)) as usize;
        if count == 0 {
            return Ok(());
        }
        let mut vacant = map
            .nodes
            .values()
            .filter(|n| {
                !n.room
                    && !n.hidden
                    && n.gate.is_none()
                    && n.content.curio.is_none()
                    && !n.content.quest
            })
            .map(|n| n.id.clone())
            .collect::<Vec<_>>();
        if vacant.len() < count {
            return Err("insufficient physical slots for source infestation curios".into());
        }
        for i in (1..vacant.len()).rev() {
            let j = rng.below((i + 1) as u64) as usize;
            vacant.swap(i, j);
        }
        let selected = vacant.into_iter().take(count).collect::<Vec<_>>();
        if map.generation.is_some() {
            crate::generate014::validate_generated(map)?;
            map.raw["017_generation_before_infestation"] =
                serde_json::to_value(map.generation.take()).map_err(|e| e.to_string())?;
        }
        for id in &selected {
            map.nodes.get_mut(id).unwrap().content.curio = Some(
                row["infestation_curio"]
                    .as_str()
                    .ok_or("missing egg curio")?
                    .into(),
            );
        }
        map.raw["017_infestation_additions"] = json!({"stage":self.state.infestation,"activity":activity,"range":bounds,"nodes":selected});
        map.source
            .layers
            .push(format!("campaign-infestation:{}", self.state.infestation));
        map.validate()
    }
    pub(super) fn suppress_infestation_family(&mut self, id: &str) -> Result<()> {
        let game = &self.definition.town.game;
        if !game.monster(id)?.mechanics.boss.end_infestation {
            return Err("suppression lacks native flagged monster".into());
        }
        let suffix = format!("/{id}/{id}.info.darkest");
        let paths = game
            .overlay
            .combat_files("monsters/", ".info.darkest")
            .into_iter()
            .filter(|p| p.ends_with(&suffix))
            .collect::<Vec<_>>();
        if paths.len() != 1 {
            return Err("ambiguous suppressed monster family source".into());
        }
        let family = paths[0]
            .split('/')
            .nth(1)
            .ok_or("missing monster family source")?;
        for path in game
            .overlay
            .combat_files(&format!("monsters/{family}/"), ".info.darkest")
        {
            let variant = path
                .split('/')
                .nth(2)
                .ok_or("missing monster variant source")?;
            if game.monster(variant)?.mechanics.boss.end_infestation {
                self.state
                    .infestation_clock
                    .suppressed
                    .insert(variant.into());
            }
        }
        Ok(())
    }
    pub(super) fn cure_crimson(&self, hero: &mut HeroResult) -> Result<()> {
        let mut remove = BTreeSet::new();
        for id in &hero.diseases {
            let q =
                library::unique_json(&self.definition.town.data.base.quirk_defs, id, "curse cure")?;
            if q["contagious_class_id"] == "disease_vampire_passive" {
                remove.insert(id.clone());
            }
        }
        hero.diseases.retain(|id| !remove.contains(id));
        hero.build.diseases.retain(|id| !remove.contains(id));
        hero.build
            .initial_disease_turns
            .retain(|id, _| !remove.contains(id));
        hero.disease_turns.retain(|id, _| !remove.contains(id));
        Ok(())
    }
}

