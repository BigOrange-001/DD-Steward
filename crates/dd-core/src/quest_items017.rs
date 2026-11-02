//! Physical quest budgets belong to the trusted campaign completion boundary.
use super::*;
use dd_data::quest_items017::Operation;
impl Session {
    pub(super) fn campaign_quest_items(&mut self, party: &[u64]) -> Result<()> {
        if self.state.campaign.is_none() {
            return Err("quest item execution needs a campaign owner".into());
        }
        // Freeze the equipped slots before gains/transforms: no newly awarded item
        // participates in the same completion, and a blocker never protects itself.
        let equipped = self
            .state
            .assets
            .iter()
            .filter(|a| {
                a.owner.is_some_and(|(id, _)| {
                    party.contains(&id) && self.hero(id).is_ok_and(|h| h.state.alive)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        for original in &equipped {
            if !original.state.enabled() {
                continue;
            }
            let guid = original.owner.ok_or("quest equipment missing owner")?.0;
            let effects = self
                .definition
                .game
                .quest_item_effects(&original.state.definition)?;
            for effect in effects {
                if !self.state.rng.roll(effect.chance) {
                    continue;
                }
                match effect.operation {
                    Operation::Growth(buffs) => {
                        let asset = self
                            .state
                            .assets
                            .iter_mut()
                            .find(|a| a.instance == original.instance)
                            .ok_or("quest growth lost physical item")?;
                        asset.state.growth.extend(buffs);
                        self.definition.game.validate_item_growth(&asset.state)?;
                    }
                    Operation::GuaranteedEvent => {
                        self.state
                            .campaign
                            .as_mut()
                            .unwrap()
                            .quest_event_opportunity = true;
                    }
                    Operation::Quirk { positive_chance } => {
                        let positive = self.state.rng.roll(positive_chance);
                        self.quest_random_quirk(guid, positive)?;
                    }
                    Operation::Trinket { table } => {
                        let owned = self.owned_rewards();
                        let mut resolver = crate::loot015::Resolver {
                            curios: &self.definition.data.base.curios,
                            difficulty: 0,
                            dungeon: String::new(),
                            hero_classes: self.definition.game.hero_ids().cloned().collect(),
                            rng: &mut self.state.rng,
                            owned: &owned,
                        };
                        let mut rewards = vec![];
                        resolver.loot(&table, &mut vec![], &mut rewards)?;
                        for reward in rewards {
                            self.bank_item(&Stack {
                                item: Item::new(&reward.kind, &reward.id),
                                amount: reward.amount,
                            })?;
                        }
                    }
                }
                self.state
                    .log
                    .push(json!({"quest_item":original.instance,"hero":guid,"effect":effect.id}));
            }
            let protected = equipped.iter().any(|a| {
                a.instance != original.instance
                    && a.owner.map(|o| o.0) == Some(guid)
                    && !a.state.destroyed
                    && a.state.definition.blocks_quest
            });
            if !protected {
                let asset = self
                    .state
                    .assets
                    .iter_mut()
                    .find(|a| a.instance == original.instance)
                    .ok_or("quest budget lost physical item")?;
                asset.state.quest_remaining =
                    asset.state.quest_remaining.map(|n| n.saturating_sub(1));
            }
        }
        Ok(())
    }
    pub(super) fn campaign_death_quirks(&self, result: &mut HeroResult) -> Result<()> {
        if result.alive {
            return Ok(());
        }
        let mut retained = vec![];
        for id in &result.build.quirks {
            let raw =
                library::unique_json(&self.definition.data.base.quirk_defs, id, "dead hero quirk")?;
            if !raw["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|v| v == "remove_on_death"))
            {
                retained.push(id.clone());
            }
        }
        result.build.quirks = retained;
        Ok(())
    }
    pub(super) fn campaign_raid_quirks(
        &mut self,
        party: &[u64],
        awards: &[crate::expedition017::RaidQuirk],
    ) -> Result<()> {
        for award in awards {
            if award.choices.is_empty() {
                return Err("empty native raid quirk pool".into());
            }
            for id in &award.choices {
                let raw = library::unique_json(
                    &self.definition.data.base.quirk_defs,
                    id,
                    "native raid quirk",
                )?;
                let tags = raw["tags"].as_array().ok_or("missing raid quirk tags")?;
                if raw["is_positive"] != true
                    || raw["is_disease"] != false
                    || raw["can_be_replaced_by_new_quirk"] != false
                    || ["singleton", "remove_on_death", "cant_lock"]
                        .iter()
                        .any(|tag| !tags.iter().any(|v| v == tag))
                {
                    return Err("unsupported native raid quirk ownership semantics".into());
                }
            }
            // The original loot rows run from Scythemaster to Isolation. The
            // documented candidate consumes that source list as a stack.
            let next = award
                .choices
                .iter()
                .rev()
                .find(|id| {
                    !self
                        .state
                        .heroes
                        .iter()
                        .any(|h| h.state.alive && h.quirks.contains_key(*id))
                })
                .cloned();
            let Some(id) = next else {
                self.state
                    .log
                    .push(json!({"native_raid_quirk":"pool_exhausted"}));
                continue;
            };
            let mut eligible = vec![];
            for guid in party {
                let h = self.hero(*guid)?;
                if !h.state.alive
                    || self
                        .state
                        .campaign
                        .as_ref()
                        .is_some_and(|c| c.owner.starts_with(super::campaign::content019::PROFILE))
                        && h.missing_until > self.effective_week()
                    || self
                        .blueprint(h, 1, false)?
                        .mechanics
                        .hero
                        .incompatible_quirks
                        .contains(&id)
                {
                    continue;
                }
                let mut positive = 0;
                let mut replaceable = false;
                for (id, flags) in &h.quirks {
                    let raw = library::unique_json(
                        &self.definition.data.base.quirk_defs,
                        id,
                        "raid quirk recipient",
                    )?;
                    if raw["is_positive"] == true {
                        positive += 1;
                        replaceable |= !flags.locked && raw["can_be_replaced_by_new_quirk"] == true;
                    }
                }
                if positive < self.definition.game.rules.quirks_max_positive as usize || replaceable
                {
                    eligible.push(*guid);
                }
            }
            if eligible.is_empty() {
                self.state
                    .log
                    .push(json!({"native_raid_quirk":id,"unavailable_recipient":true}));
                continue;
            }
            let guid = eligible[self.state.rng.below(eligible.len() as u64) as usize];
            self.add_quirk(guid, &id)?;
            let i = self.hero_index(guid)?;
            let flags = self.state.heroes[i]
                .quirks
                .get_mut(&id)
                .ok_or("native raid quirk was not applied")?;
            flags.locked = true;
            self.state.heroes[i].state.build.quirks =
                self.state.heroes[i].quirks.keys().cloned().collect();
            self.state
                .log
                .push(json!({"native_raid_quirk":id,"hero":guid,"locked":true}));
        }
        Ok(())
    }
    pub(super) fn campaign_plot_quirks(&mut self, party: &[u64], rows: &[Value]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut weights = vec![];
        for row in rows {
            dd_data::campaign017::fields(row, &["quirk_ids", "chance"], "quest quirk row")?;
            for id in dd_data::campaign017::array(row, "quirk_ids")? {
                library::unique_json(
                    &self.definition.data.base.quirk_defs,
                    id.as_str().ok_or("invalid quest quirk ID")?,
                    "quest completion quirk",
                )?;
            }
            weights.push(
                row["chance"]
                    .as_f64()
                    .filter(|w| w.is_finite() && *w >= 0.)
                    .ok_or("invalid quest quirk weight")?,
            );
        }
        for guid in party
            .iter()
            .filter(|id| self.hero(**id).is_ok_and(|h| h.state.alive))
            .copied()
            .collect::<Vec<_>>()
        {
            let row = &rows[self.choose(&weights)?];
            for id in dd_data::campaign017::array(row, "quirk_ids")? {
                let id = id.as_str().ok_or("invalid quest quirk ID")?;
                let before = self.hero(guid)?.quirks.clone();
                self.add_quirk(guid, id)?;
                let changed = before != self.hero(guid)?.quirks;
                self.state
                    .log
                    .push(json!({"quest_fixed_quirk":id,"hero":guid,"applied":changed}));
            }
            let i = self.hero_index(guid)?;
            self.state.heroes[i].state.build.quirks =
                self.state.heroes[i].quirks.keys().cloned().collect();
        }
        Ok(())
    }
    fn quest_random_quirk(&mut self, guid: u64, positive: bool) -> Result<()> {
        let hero = self.hero(guid)?.clone();
        let blueprint = self.blueprint(&hero, 1, false)?;
        let mut ids = vec![];
        let mut weights = vec![];
        for id in self.definition.data.base.quirk_defs.keys() {
            let raw = library::unique_json(
                &self.definition.data.base.quirk_defs,
                id,
                "quest item quirk",
            )?;
            if raw["is_disease"] != false
                || raw["is_positive"].as_bool() != Some(positive)
                || hero.quirks.contains_key(id)
                || blueprint.mechanics.hero.incompatible_quirks.contains(id)
            {
                continue;
            }
            let weight = raw["random_chance"]
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.)
                .ok_or("invalid source quest quirk weight")?;
            if weight > 0. {
                ids.push(id.clone());
                weights.push(weight);
            }
        }
        if !ids.is_empty() {
            let id = ids[self.choose(&weights)?].clone();
            self.add_quirk(guid, &id)?;
            let i = self.hero_index(guid)?;
            self.state.heroes[i].state.build.quirks =
                self.state.heroes[i].quirks.keys().cloned().collect();
        }
        Ok(())
    }
}

