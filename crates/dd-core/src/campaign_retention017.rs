//! Physical retention rewards, source rarity thresholds and protected returns.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingAsset {
    pub week: u32,
    pub asset: Asset,
}
impl Session {
    pub(super) fn validate_retention_asset(&self, asset: &Asset) -> Result<()> {
        self.definition
            .town
            .game
            .validate_item_growth(&asset.state)?;
        if asset.owner.is_some()
            || asset.state.destroyed
            || self.definition.town.trinket(&asset.state.definition.id)? != asset.state.definition
        {
            return Err("retained physical asset differs from source".into());
        }
        Ok(())
    }
    fn asset_rarity(&self, asset: &Asset) -> Result<String> {
        let raw = library::unique_json(
            &self.definition.town.data.base.curios.trinkets,
            &asset.state.definition.id,
            "physical asset rarity",
        )?;
        Ok(raw["rarity"].as_str().ok_or("missing asset rarity")?.into())
    }
    pub(super) fn protected_asset(&self, asset: &Asset) -> Result<bool> {
        self.definition
            .data
            .rarities
            .as_ref()
            .ok_or("missing original rarity ordering")?
            .protected(
                &self.asset_rarity(asset)?,
                &self.definition.data.candidates.protected_award_categories,
                &self.definition.data.candidates.protected_rarities,
            )
    }
    pub(super) fn stealable_assets(&self, town: &TownSession) -> Result<Vec<u64>> {
        let mut result = vec![];
        for asset in town.state.assets.iter().filter(|a| a.owner.is_none()) {
            if !self.protected_asset(asset)? {
                result.push(asset.instance);
            }
        }
        Ok(result)
    }
    pub(super) fn record_lost_asset(&mut self, asset: Asset) -> Result<()> {
        self.record_asset_loss_at(asset, self.state.clock.week)
    }
    pub(super) fn record_asset_loss_at(&mut self, mut asset: Asset, week: u32) -> Result<()> {
        asset.owner = None;
        if self
            .state
            .lost_assets
            .iter()
            .any(|a| a.instance == asset.instance)
            || self
                .state
                .asset_returns
                .iter()
                .any(|a| a.asset.instance == asset.instance)
        {
            return Err("duplicate physical loss receipt".into());
        }
        if self.protected_asset(&asset)? {
            self.state.asset_returns.push(PendingAsset {
                week: week
                    .checked_add(1)
                    .ok_or("asset return deadline overflow")?,
                asset,
            });
        } else {
            self.state.lost_assets.push(asset);
        }
        Ok(())
    }
    pub(super) fn return_protected_assets(&mut self, town: &mut TownSession) -> Result<()> {
        let mut pending = vec![];
        for row in std::mem::take(&mut self.state.asset_returns) {
            if row.week <= self.state.clock.week {
                if town
                    .state
                    .assets
                    .iter()
                    .any(|a| a.instance == row.asset.instance)
                {
                    return Err("protected return duplicates live asset".into());
                }
                self.state.log.push(json!({"protected_asset_return":row.asset.instance,"week":self.state.clock.week}));
                town.state.assets.push(row.asset);
            } else {
                pending.push(row);
            }
        }
        self.state.asset_returns = pending;
        Ok(())
    }
    pub(super) fn validate_asset_returns(&self) -> Result<()> {
        let mut seen = self
            .state
            .city
            .assets
            .iter()
            .chain(&self.state.lost_assets)
            .map(|a| a.instance)
            .collect::<BTreeSet<_>>();
        for row in &self.state.asset_returns {
            if row.week
                != self
                    .state
                    .clock
                    .week
                    .checked_add(1)
                    .ok_or("protected return week overflow")?
                || row.asset.owner.is_some()
                || !seen.insert(row.asset.instance)
                || row.asset.instance >= self.state.city.next_asset
                || !self.protected_asset(&row.asset)?
            {
                return Err("invalid protected asset return ledger".into());
            }
            self.validate_retention_asset(&row.asset)?;
        }
        Ok(())
    }
    pub(super) fn retention_plot(&self) -> Result<Option<String>> {
        let mut eligible = vec![];
        for id in self.definition.data.quests.keys() {
            let q = library::unique_json(&self.definition.data.quests, id, "retention quest")?;
            if q["is_enabled"] == false
                || q["is_repeatable"] != true && self.state.city.completed_ids.contains(id)
                || (q["is_generated_by_event"] == true || q["is_started_by_event"] == true)
                    && !self.state.plot_windows.contains_key(id)
            {
                continue;
            }
            let n = dd_data::retention017::quest_count(q)?;
            if n == 0 || self.state.lost_assets.len() < n {
                continue;
            }
            let minimum = q["trinket_retention_minimum_rarity"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or("missing retention rarity threshold")?;
            let rarities = self
                .definition
                .data
                .rarities
                .as_ref()
                .ok_or("missing retention rarity source")?;
            let mut enough = false;
            for asset in &self.state.lost_assets {
                if rarities.at_least(&self.asset_rarity(asset)?, minimum)? {
                    enough = true;
                }
            }
            if enough {
                eligible.push((
                    dd_data::campaign017::uint(&q["quest"], "difficulty")?,
                    id.clone(),
                ));
            }
        }
        let best = eligible.iter().map(|(level, _)| *level).max();
        let choices = eligible
            .into_iter()
            .filter(|(level, _)| Some(*level) == best)
            .map(|(_, id)| id)
            .collect::<Vec<_>>();
        match choices.len() {
            0 => Ok(None),
            1 => Ok(Some(choices[0].clone())),
            _ => Err("ambiguous retention quest difficulty".into()),
        }
    }
    pub(super) fn draw_retention_assets(&mut self, source: &Value) -> Result<Vec<u64>> {
        let n = dd_data::retention017::quest_count(source)?;
        if n == 0 {
            return Ok(vec![]);
        }
        if n > 10000 || self.state.lost_assets.len() < n {
            return Err("invalid source retention reward count".into());
        }
        let mut ids = self
            .state
            .lost_assets
            .iter()
            .map(|a| a.instance)
            .collect::<Vec<_>>();
        for i in (1..ids.len()).rev() {
            let j = self.state.rng.below((i + 1) as u64) as usize;
            ids.swap(i, j);
        }
        ids.truncate(n);
        Ok(ids)
    }
    pub(super) fn recover_quest_assets(
        &mut self,
        town: &mut TownSession,
        quest: &board::Instance,
    ) -> Result<()> {
        for id in &quest.recovered_assets {
            let index = self
                .state
                .lost_assets
                .iter()
                .position(|a| a.instance == *id)
                .ok_or("retention reward already claimed")?;
            let asset = self.state.lost_assets.remove(index);
            if town
                .state
                .assets
                .iter()
                .any(|a| a.instance == asset.instance)
            {
                return Err("retention reward duplicated live physical asset".into());
            }
            town.state.assets.push(asset);
        }
        Ok(())
    }
}


