//! Shared original-table draw semantics for the expedition loot tray.
use super::{Result, Rng};
use crate::initial013::Reward;
use dd_data::curios013::Curios;
use std::collections::BTreeSet;
pub(super) struct Resolver<'a> {
    pub curios: &'a Curios,
    pub difficulty: i64,
    pub dungeon: String,
    pub hero_classes: BTreeSet<String>,
    pub rng: &'a mut Rng,
    pub owned: &'a [Reward],
}

impl Resolver<'_> {
    fn pick(&mut self, weights: &[f64]) -> Result<usize> {
        let total: f64 = weights.iter().sum();
        if weights.iter().any(|w| !w.is_finite() || *w < 0.) || !total.is_finite() || total <= 0. {
            return Err("invalid curio/loot weights".into());
        }
        let mut roll = (self.rng.next() >> 11) as f64 / ((1u64 << 53) as f64) * total;
        for (i, w) in weights.iter().enumerate() {
            roll -= w;
            if roll < 0. {
                return Ok(i);
            }
        }
        Ok(weights.iter().rposition(|w| *w > 0.).unwrap())
    }
    pub(super) fn loot(
        &mut self,
        id: &str,
        stack: &mut Vec<String>,
        rewards: &mut Vec<Reward>,
    ) -> Result<()> {
        self.loot_inner(id, stack, rewards, None, false)
    }
    pub(super) fn campaign_loot(
        &mut self,
        id: &str,
        stack: &mut Vec<String>,
        rewards: &mut Vec<Reward>,
        infestation: &str,
    ) -> Result<()> {
        self.loot_inner(id, stack, rewards, Some(infestation), false)
    }
    pub(super) fn automation_loot(
        &mut self,
        id: &str,
        stack: &mut Vec<String>,
        rewards: &mut Vec<Reward>,
        infestation: &str,
    ) -> Result<()> {
        self.loot_inner(id, stack, rewards, Some(infestation), true)
    }
    fn loot_inner(
        &mut self,
        id: &str,
        stack: &mut Vec<String>,
        rewards: &mut Vec<Reward>,
        infestation: Option<&str>,
        empty_trinket_pool: bool,
    ) -> Result<()> {
        if stack.len() > 16 || stack.iter().any(|s| s == id) || rewards.len() > 1000 {
            return Err("cyclic/oversized curio loot".into());
        }
        stack.push(id.into());
        let dungeon = self.dungeon.clone();
        let table = if let Some(stage) = infestation {
            self.curios
                .campaign_table(id, self.difficulty, &dungeon, stage)?
        } else {
            self.curios.table(id, self.difficulty, &dungeon)?
        }
        .clone();
        let entries = table["entries"].as_array().ok_or("missing loot entries")?;
        let weights = entries
            .iter()
            .map(|v| v["chances"].as_f64().ok_or("invalid loot chances".into()))
            .collect::<Result<Vec<_>>>()?;
        let entry = &entries[self.pick(&weights)?];
        dd_data::curios013::known_keys(entry, &["type", "chances", "data"])?;
        let data = &entry["data"];
        match entry["type"].as_str().ok_or("missing loot type")? {
            "nothing" => {
                dd_data::curios013::known_keys(data, &[])?;
            }
            "table" => {
                dd_data::curios013::known_keys(data, &["table"])?;
                self.loot_inner(
                    data["table"].as_str().ok_or("missing nested loot table")?,
                    stack,
                    rewards,
                    infestation,
                    empty_trinket_pool,
                )?;
            }
            "item" => {
                dd_data::curios013::known_keys(data, &["type", "id", "amount"])?;
                let kind = data["type"].as_str().ok_or("missing loot item kind")?;
                if !(["supply", "provision", "gold", "gem", "heirloom", "estate"].contains(&kind)
                    || infestation.is_some()
                        && ["trinket", "quest_item", "estate_currency", "shard"].contains(&kind))
                {
                    return Err(format!("unsupported loot item kind {kind}"));
                }
                let amount = data["amount"]
                    .as_u64()
                    .filter(|n| (1..=1000000).contains(n))
                    .ok_or("invalid loot amount")? as u32;
                rewards.push(Reward {
                    kind: kind.into(),
                    id: data["id"].as_str().ok_or("missing loot item id")?.into(),
                    amount,
                });
            }
            "journal_page" => {
                if let Some(page) =
                    dd_data::curios013::fixed_journal_page(data, infestation.is_some())?
                {
                    let id = page.to_string();
                    if !self.owned.iter().chain(rewards.iter()).any(|r| {
                        ["journal_page", "journal_page_auto"].contains(&r.kind.as_str())
                            && r.id == id
                    }) {
                        // Transient delivery marker, converted to a normal journal receipt
                        // by the campaign owner before any inventory/snapshot boundary.
                        rewards.push(Reward {
                            kind: if page == 0 {
                                "journal_page"
                            } else {
                                "journal_page_auto"
                            }
                            .into(),
                            id,
                            amount: 1,
                        });
                    }
                    stack.pop();
                    return Ok(());
                }
                let min = data["min_page_index"]
                    .as_i64()
                    .ok_or("missing journal bound")?;
                let max = data["max_page_index"]
                    .as_i64()
                    .ok_or("missing journal bound")?;
                if min < 1 || max < min || max > 10000 {
                    return Err("invalid journal bounds".into());
                }
                let owned = self
                    .owned
                    .iter()
                    .chain(rewards.iter())
                    .filter(|r| r.kind == "journal_page")
                    .map(|r| r.id.clone())
                    .collect::<BTreeSet<_>>();
                let eligible = (min..=max)
                    .map(|n| n.to_string())
                    .filter(|s| !owned.contains(s))
                    .collect::<Vec<_>>();
                if !eligible.is_empty() {
                    let n = self.rng.below(eligible.len() as u64) as usize;
                    rewards.push(Reward {
                        kind: "journal_page".into(),
                        id: eligible[n].clone(),
                        amount: 1,
                    });
                }
            }
            "trinket" => {
                dd_data::curios013::known_keys(data, &["rarity"])?;
                let rarity = data["rarity"].as_str().ok_or("missing trinket rarity")?;
                if let Some(reward) = self.trinket_inner(rarity, rewards, empty_trinket_pool)? {
                    rewards.push(reward);
                }
            }
            kind => return Err(format!("unsupported initial loot type {kind}")),
        }
        stack.pop();
        Ok(())
    }
    pub(super) fn trinket(&mut self, rarity: &str, rewards: &[Reward]) -> Result<Reward> {
        self.trinket_inner(rarity, rewards, false)?
            .ok_or_else(|| format!("no eligible {rarity} trinket"))
    }
    pub(super) fn trinket_inner(
        &mut self,
        rarity: &str,
        rewards: &[Reward],
        empty_trinket_pool: bool,
    ) -> Result<Option<Reward>> {
        let dungeon = self.dungeon.clone();
        let mut eligible = vec![];
        let mut known_rarity = false;
        for (id, v) in &self.curios.trinkets {
            if v.is_array() {
                if v.as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["rarity"].as_str() == Some(rarity))
                {
                    return Err(format!("ambiguous eligible trinket {id}"));
                }
                continue;
            }
            known_rarity |= v["rarity"].as_str() == Some(rarity);
            if let Some(classes) = v.get("hero_class_requirements") {
                let classes = classes
                    .as_array()
                    .ok_or("invalid trinket class requirements")?;
                if classes.iter().any(|c| c.as_str().is_none()) {
                    return Err("invalid trinket class id".into());
                }
                if !classes.is_empty()
                    && !classes
                        .iter()
                        .any(|c| self.hero_classes.contains(c.as_str().unwrap()))
                {
                    continue;
                }
            }
            if v["rarity"].as_str() != Some(rarity) {
                continue;
            }
            if !v["origin_dungeon"]
                .as_str()
                .is_none_or(|s| s.is_empty() || s == dungeon.as_str())
            {
                continue;
            }
            let limit = v["limit"].as_i64().ok_or("missing trinket limit")?;
            let owned = self
                .owned
                .iter()
                .chain(rewards.iter())
                .filter(|r| r.kind == "trinket" && r.id == *id)
                .map(|r| i64::from(r.amount))
                .sum::<i64>();
            if limit > 0 && owned >= limit {
                continue;
            }
            eligible.push(id.clone());
        }
        if eligible.is_empty() {
            if empty_trinket_pool && known_rarity {
                return Ok(None);
            }
            return Err(format!("no eligible {rarity} trinket"));
        }
        let n = self.rng.below(eligible.len() as u64) as usize;
        Ok(Some(Reward {
            kind: "trinket".into(),
            id: eligible[n].clone(),
            amount: 1,
        }))
    }
}
