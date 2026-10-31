use super::*;
impl Session {
    pub(super) fn district_effects(&self) -> Result<Vec<(String, Value)>> {
        let mut rows = vec![];
        for id in &self.state.districts {
            for row in self.definition.data.district(id)?["buff_list"]
                .as_array()
                .ok_or("missing district effects")?
            {
                rows.push((id.clone(), row.clone()));
            }
        }
        Ok(rows)
    }
    pub(super) fn bonuses(&self, party: &[u64], heroes: &mut [Blueprint]) -> Result<Bonuses> {
        let mut out = Bonuses {
            supplies: self.state.supply_grants.clone(),
            ..Default::default()
        };
        for (district, row) in self.district_effects()? {
            let applies = |b: &Blueprint| -> Result<bool> {
                let tags = row["hero_type_tags"]
                    .as_array()
                    .ok_or("missing district class tags")?;
                Ok(tags.is_empty()
                    || tags.iter().any(|t| {
                        t.as_str()
                            .is_some_and(|t| b.mechanics.hero.tags.iter().any(|v| v == t))
                    }))
            };
            match row["type"].as_str().ok_or("missing district type")? {
                "DistrictHeroBuffData" => {
                    for (i, b) in heroes.iter_mut().enumerate() {
                        if !applies(b)? {
                            continue;
                        }
                        for id in row["buff_list"]
                            .as_array()
                            .ok_or("missing district Buff IDs")?
                        {
                            let id = id.as_str().ok_or("invalid district Buff ID")?;
                            if self.definition.game.noncombat_buff(id)? {
                                let buff = dd_data::mechanics::read_exploration_buff(
                                    &self.definition.game.buffs,
                                    id,
                                )?;
                                out.field_buffs.entry(party[i]).or_default().push(buff);
                            } else {
                                let buff =
                                    dd_data::mechanics::read_buff(&self.definition.game.buffs, id)?;
                                b.mechanics.hero.exploration_buffs.push(
                                    dd_data::hero010::CarryBuff {
                                        remaining: buff.duration,
                                        definition: buff,
                                    },
                                );
                            }
                        }
                    }
                }
                "DistrictLightBuffData" => {
                    let light: dd_data::lighting::Darkness =
                        serde_json::from_value(row["darkness"].clone())
                            .map_err(|e| e.to_string())?;
                    if out.lighting.as_ref().is_some_and(|old| old != &light) {
                        return Err("conflicting district light tables".into());
                    }
                    out.lighting = Some(light);
                    if out
                        .loot_darkness
                        .as_ref()
                        .is_some_and(|old| old != &row["loot_darkness_bonus"])
                    {
                        return Err("conflicting district loot light tables".into());
                    }
                    out.loot_darkness = Some(row["loot_darkness_bonus"].clone());
                }
                "DistrictCurioInteractionBuffData" => out.curio.push(row.clone()),
                "DistrictCampingBuffData" => out.camping.push(row.clone()),
                "DistrictDisableBuffData" => {
                    if row["disabled_type"] != "hunger" {
                        return Err("unknown district disable operation".into());
                    }
                    out.disable_hunger = true;
                }
                "DistrictStackingBuffData" => {
                    let name = row["extra_stack_limit_id"]
                        .as_str()
                        .ok_or("missing district stack id")?;
                    let mut defs = BTreeMap::new();
                    for p in self.definition.game.overlay.paths.keys().filter(|p| {
                        p.starts_with("inventory/") && p.ends_with("extra_stack_limits.darkest")
                    }) {
                        for bytes in self.definition.game.overlay.read_town_versions(p)? {
                            let doc = dd_data::darkest::parse_str(
                                std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
                            );
                            if !doc.warnings.is_empty() {
                                return Err("invalid stack limit syntax".into());
                            }
                            for r in &doc.records {
                                if r.key != "extra_stack_limit" {
                                    return Err("unsupported stack extension record".into());
                                }
                                let id = dd_data::combat::text(r, "id")?;
                                let item = if r.param("item_id").is_some() {
                                    Some(dd_data::combat::text(r, "item_id")?)
                                } else {
                                    None
                                };
                                library::insert_json(
                                    &mut defs,
                                    id.into(),
                                    json!({"item_type":dd_data::combat::text(r,"item_type")?,"item_id":item,"amount":dd_data::combat::number(r,"amount")?}),
                                );
                            }
                        }
                    }
                    let raw = library::unique_json(&defs, name, "district stack limit")?;
                    let amount = raw["amount"]
                        .as_f64()
                        .filter(|n| n.is_finite() && *n >= 0. && n.fract() == 0.)
                        .ok_or("invalid stack increment")? as u32;
                    for key in self.definition.data.base.bag_rules.items.keys() {
                        let (kind, id) = key.split_once(':').ok_or("invalid inventory key")?;
                        if raw["item_type"] == kind
                            && raw["item_id"].as_str().is_none_or(|wanted| wanted == id)
                        {
                            out.stacking.push(json!({"item":key,"amount":amount}));
                        }
                    }
                }
                "DistrictReplacementInventoryEffectBuffData" => {
                    let id = row["item_id"].as_str().ok_or("missing district item")?;
                    let supply =
                        self.definition
                            .data
                            .replacement(&self.definition.game, &district, id)?;
                    if out.replacements.get(id).is_some_and(|old| old != &supply) {
                        return Err("conflicting district item replacement".into());
                    }
                    out.replacements.insert(id.into(), supply);
                }
                "DistrictPrestigeBuffData" => out.prestige = true,
                "DistrictEstateBuffData"
                | "DistrictSupplyBuffData"
                | "DistrictRosterStressReliefBuffData"
                | "DistrictRandomLootBuffData" => {}
                _ => return Err("unknown active district effect".into()),
            }
        }
        if let Some(context) = &self.state.campaign {
            for effect in context
                .effects
                .iter()
                .filter(|e| e.kind == "embark_party_buff")
            {
                if self.definition.game.noncombat_buff(&effect.id)? {
                    let buff = dd_data::mechanics::read_exploration_buff(
                        &self.definition.game.buffs,
                        &effect.id,
                    )?;
                    for id in party {
                        out.field_buffs.entry(*id).or_default().push(buff.clone());
                    }
                } else {
                    let buff =
                        dd_data::mechanics::read_buff(&self.definition.game.buffs, &effect.id)?;
                    for hero in heroes.iter_mut() {
                        hero.mechanics
                            .hero
                            .exploration_buffs
                            .push(dd_data::hero010::CarryBuff {
                                remaining: buff.duration,
                                definition: buff.clone(),
                            });
                    }
                }
            }
        }
        Ok(out)
    }
    pub(super) fn district_week(&mut self, opening_gold: u32) -> Result<()> {
        self.state.supply_grants.clear();
        for (_, row) in self.district_effects()? {
            match row["type"].as_str().ok_or("missing district type")? {
                "DistrictEstateBuffData" => {
                    let rate = row["weekly_gold_interest"]
                        .as_f64()
                        .filter(|n| n.is_finite() && *n >= 0. && *n <= 1.)
                        .ok_or("invalid interest rate")?;
                    let amount = (f64::from(opening_gold) * rate).floor() as u32;
                    self.credit(&BTreeMap::from([(self.currency("gold")?, amount)]))?;
                    self.state.log.push(json!({"district_interest":amount}));
                }
                "DistrictSupplyBuffData" => {
                    let lo = uint(&row, "range_min")?;
                    let hi = uint(&row, "range_max")?;
                    if lo > hi || hi > 1000000 {
                        return Err("invalid district supply range".into());
                    }
                    let amount = lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32;
                    let stack = Stack {
                        item: Item::new(
                            row["item_type"]
                                .as_str()
                                .ok_or("missing district supply kind")?,
                            row["item_name"]
                                .as_str()
                                .ok_or("missing district supply item")?,
                        ),
                        amount,
                    };
                    self.definition
                        .data
                        .base
                        .bag_rules
                        .definition(&stack.item)?;
                    match row["target_inventory"].as_str() {
                        Some("estate") => {
                            self.credit(&BTreeMap::from([(stack.item.key(), stack.amount)]))?
                        }
                        Some("provision") => self.state.supply_grants.push(stack),
                        _ => return Err("unsupported district target inventory".into()),
                    }
                }
                "DistrictRandomLootBuffData" => {
                    let code = if self
                        .definition
                        .data
                        .content
                        .packages
                        .iter()
                        .any(|p| p == "crimson_court")
                    {
                        row["cc_loot_table_code"]
                            .as_str()
                            .or(row["loot_table_code"].as_str())
                    } else {
                        row["loot_table_code"].as_str()
                    }
                    .ok_or("missing district loot table")?;
                    let count = uint(&row, "roll_count")?;
                    let owned = self.owned_rewards();
                    let mut resolver = crate::loot015::Resolver {
                        curios: &self.definition.data.base.curios,
                        difficulty: 0,
                        dungeon: "".into(),
                        hero_classes: self.definition.game.hero_ids().cloned().collect(),
                        rng: &mut self.state.rng,
                        owned: &owned,
                    };
                    let mut draws = vec![];
                    for _ in 0..count {
                        resolver.loot(code, &mut vec![], &mut draws)?;
                    }
                    for r in draws {
                        self.bank_item(&Stack {
                            item: Item::new(&r.kind, &r.id),
                            amount: r.amount,
                        })?;
                    }
                }
                "DistrictHeroBuffData"
                | "DistrictLightBuffData"
                | "DistrictCurioInteractionBuffData"
                | "DistrictCampingBuffData"
                | "DistrictDisableBuffData"
                | "DistrictStackingBuffData"
                | "DistrictReplacementInventoryEffectBuffData"
                | "DistrictPrestigeBuffData"
                | "DistrictRosterStressReliefBuffData" => {}
                _ => return Err("unknown district weekly operation".into()),
            }
        }
        Ok(())
    }
    pub(super) fn idle_relief(&self) -> Result<i32> {
        let mut amount = self.definition.data.base.roster_variables["town_visit_town_progression"]
            ["idle_hero_stress_heal"]
            .as_f64()
            .ok_or("missing idle stress relief")?;
        for (_, r) in self.district_effects()? {
            if r["type"] == "DistrictRosterStressReliefBuffData" {
                amount = amount.max(
                    r["stress_relief"]
                        .as_f64()
                        .ok_or("missing district relief")?,
                );
            }
        }
        Ok(amount.round() as i32)
    }
}
