use super::*;
impl Session {
    pub(super) fn money(&self, base: u32, factor: f64) -> Result<u32> {
        let precision = self.definition.data.candidates.currency_precision;
        if precision == 0 || !factor.is_finite() || factor < 0. || factor > 1000. {
            return Err("invalid currency factor".into());
        }
        let scaled = (factor * precision as f64).round() as u64;
        let amount = u64::from(base)
            .checked_mul(scaled)
            .ok_or("currency overflow")?
            .div_ceil(precision);
        u32::try_from(amount).map_err(|_| "currency overflow".into())
    }
    pub(super) fn cost_rows(&self, rows: &Value, discount: f64) -> Result<BTreeMap<String, u32>> {
        let mut cost = BTreeMap::new();
        if !discount.is_finite() {
            return Err("invalid town discount".into());
        }
        for row in rows.as_array().ok_or("invalid town currency costs")? {
            let id = row["type"].as_str().ok_or("missing cost currency")?;
            let amount = uint(row, "amount")?;
            let amount = if id == "gold" {
                self.money(amount, (1. - discount).max(0.))?
            } else {
                amount
            };
            let value = cost.entry(self.currency(id)?).or_insert(0u32);
            *value = value.checked_add(amount).ok_or("cost overflow")?;
        }
        Ok(cost)
    }
    pub(super) fn discount(&self, rows: &Value) -> Result<f64> {
        let mut total = 0.;
        for row in rows.as_array().ok_or("invalid discount tiers")? {
            let Some(code) = row["upgrade_requirement_code"].as_str() else {
                return Err("discount missing source upgrade".into());
            };
            let tree = row["upgrade_tree_id"]
                .as_str()
                .ok_or("discount missing source tree")?;
            if self.purchased(None, tree, code) {
                total += row["discount_percent"]
                    .as_f64()
                    .ok_or("invalid source discount")?;
            }
        }
        Ok(total)
    }
    pub(super) fn modifier(
        &self,
        h: &Hero,
        stat: &str,
        sub: &str,
        activity: Option<&str>,
    ) -> Result<f64> {
        let b = self.blueprint(h, 1, false)?;
        let mut amount = if stat == "stress_heal_received_percent" && activity.is_some() {
            self.state
                .campaign
                .as_ref()
                .map_or(0., |c| c.sum("stress_treatment_debuff", ""))
        } else {
            0.
        };
        let mut buffs = b.mechanics.build.noncombat_buffs;
        if let Some(c) = &self.state.campaign {
            buffs.extend(
                c.effects
                    .iter()
                    .filter(|e| e.kind == "in_activity_buff")
                    .map(|e| e.id.clone()),
            );
        }
        for id in buffs {
            let raw = library::unique_json(&self.definition.game.buffs, &id, "town modifier")?;
            let key = raw["stat_type"].as_str().ok_or("missing Buff stat")?;
            let scope = raw["rule_type"].as_str().unwrap_or("always");
            if !(key == "upgrade_discount"
                || key == "stress_from_idle_in_town"
                || ["in_town", "in_activity"].contains(&scope))
            {
                continue;
            }
            if ![
                "upgrade_discount",
                "stress_from_idle_in_town",
                "stress_heal_received_percent",
                "activity_side_effect_chance",
            ]
            .contains(&key)
            {
                return Err(format!("unsupported town Buff statistic {key}"));
            }
            let active = match scope {
                "always" | "in_town" => true,
                "in_activity" => activity == raw["rule_data"]["string"].as_str(),
                _ => return Err(format!("unsupported town modifier rule {scope}")),
            };
            let active = active != raw["is_false_rule"].as_bool().unwrap_or(false);
            if active && key == stat && raw["stat_sub_type"].as_str().unwrap_or("") == sub {
                amount += raw["amount"]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or("invalid town modifier amount")?;
            }
        }
        Ok(amount)
    }
    pub(super) fn upgrade_quote(
        &self,
        hero: Option<u64>,
        tree: &str,
        code: &str,
    ) -> Result<BTreeMap<String, u32>> {
        let data = &self.definition.data;
        let t = data.tree(tree)?;
        let n = data.node(tree, code)?;
        if t["is_instanced"].as_bool() != Some(hero.is_some()) || self.purchased(hero, tree, code) {
            return Err("wrong owner or already purchased upgrade".into());
        }
        let tags = t["tags"].as_array().ok_or("missing upgrade tags")?;
        let mut discount = 0.;
        if let Some(id) = hero {
            if !self.available(id) {
                return Err("hero unavailable for training".into());
            }
            let h = self.hero(id)?;
            if !tree.starts_with(&format!("{}.", h.class)) {
                return Err("wrong class upgrade".into());
            }
            if self.level(h.xp)?
                < n["prerequisite_resolve_level"]
                    .as_u64()
                    .ok_or("missing training resolve requirement")? as u32
            {
                return Err("resolve level insufficient for training".into());
            }
            let (kind, building, key) = if tags.iter().any(|v| v == "weapon") {
                ("weapon", "blacksmith", "equipment_cost_discount_upgrades")
            } else if tags.iter().any(|v| v == "armour") {
                ("armour", "blacksmith", "equipment_cost_discount_upgrades")
            } else if tags.iter().any(|v| v == "combat_skill") {
                (
                    "combat_skill",
                    "guild",
                    "combat_skill_cost_discount_upgrades",
                )
            } else {
                return Err("unsupported hero upgrade category".into());
            };
            if !self.unlocked(building)? {
                return Err("facility locked".into());
            }
            discount = self.discount(data.building(building, key)?)?
                + self.modifier(
                    h,
                    "upgrade_discount",
                    if kind == "armour" { "armor" } else { kind },
                    None,
                )?;
        } else {
            let building = tree.split('.').next().ok_or("invalid building tree")?;
            if !self.unlocked(building)? {
                return Err("facility locked".into());
            }
        }
        for req in n["prerequisite_requirements"]
            .as_array()
            .ok_or("missing upgrade prerequisites")?
        {
            let dep = req["tree_id"].as_str().ok_or("missing dependency tree")?;
            let node = req["requirement_code"]
                .as_str()
                .ok_or("missing dependency code")?;
            let owner = if data.tree(dep)?["is_instanced"] == true {
                hero
            } else {
                None
            };
            if !self.purchased(owner, dep, node) {
                return Err("upgrade prerequisite not purchased".into());
            }
        }
        if let Some(c) = &self.state.campaign {
            let mut tags = tags.iter().filter_map(Value::as_str).collect::<Vec<_>>();
            if hero.is_none() {
                tags.push("building");
            }
            if tags.iter().any(|t| c.free(t)) {
                return self.cost_rows(&n["currency_cost"], 0.).map(|mut cost| {
                    for n in cost.values_mut() {
                        *n = 0;
                    }
                    cost
                });
            }
            discount += tags
                .iter()
                .map(|t| c.sum("upgrade_tag_discount", t))
                .sum::<f64>();
        }
        self.cost_rows(&n["currency_cost"], discount)
    }
    pub(super) fn add_asset(&mut self, id: &str, owner: Option<(u64, usize)>) -> Result<u64> {
        let state = TrinketState::fresh(self.definition.trinket(id)?);
        let instance = self.state.next_asset;
        self.state.next_asset = self
            .state
            .next_asset
            .checked_add(1)
            .ok_or("asset identity overflow")?;
        self.state.assets.push(Asset {
            instance,
            state,
            owner,
        });
        Ok(instance)
    }
    pub(super) fn trinket_price(&self, id: &str, selling: bool) -> Result<u32> {
        let raw = library::unique_json(
            &self.definition.data.base.curios.trinkets,
            id,
            "town trinket",
        )?;
        if raw.get("price").is_none() {
            return Err("trinket has no gold trading price".into());
        }
        let price = uint(raw, "price")?;
        let store = self.definition.data.store("nomad_wagon.trinket_supply")?;
        let discount = if selling {
            self.tier(
                &store["trinket_sell_value_discount_upgrades"],
                "nomad_wagon.cost",
            )?["discount_percent"]
                .as_f64()
                .ok_or("missing source sale discount")?
        } else {
            self.discount(&store["trinket_cost_discount_upgrades"])?
        };
        if !discount.is_finite() || !(0. ..=1.).contains(&discount) {
            return Err("invalid trinket price discount".into());
        }
        let mut factor = 1. - discount;
        if let Some(c) = &self.state.campaign {
            if selling {
                factor *= c
                    .effects
                    .iter()
                    .filter(|e| e.kind == "trinket_sell_multiplier")
                    .map(|e| e.amount)
                    .product::<f64>();
            } else {
                factor *= (1. - c.sum("upgrade_tag_discount", "trinket")).max(0.);
                if !c.used.contains_key("nomad_wagon_trinket_discount") {
                    factor *= (1. - c.sum("nomad_wagon_trinket_discount", "")).max(0.);
                }
            }
        }
        self.money(price, factor)
    }
    pub(super) fn install_hero_purchases(&mut self, h: &Hero) -> Result<()> {
        for (kind, n) in [("weapon", h.weapon), ("armour", h.armour)] {
            for rank in 0..n {
                let tree = format!("{}.{kind}", h.class);
                self.definition.data.node(&tree, &rank.to_string())?;
                self.state
                    .purchases
                    .insert((Some(h.guid), tree, rank.to_string()));
            }
        }
        for (skill, rank) in &h.skills {
            let tree = format!("{}.{}", h.class, skill);
            for n in 0..=*rank {
                self.definition.data.node(&tree, &n.to_string())?;
                self.state
                    .purchases
                    .insert((Some(h.guid), tree.clone(), n.to_string()));
            }
        }
        Ok(())
    }
    pub(super) fn camp_quote(&self, hero: u64, skill: &str) -> Result<BTreeMap<String, u32>> {
        if !self.available(hero) || !self.unlocked("camping_trainer")? {
            return Err("camping training unavailable".into());
        }
        let h = self.hero(hero)?;
        if h.camping.contains(skill) {
            return Err("camping skill already learned".into());
        }
        let raw = self
            .definition
            .data
            .camp(&format!("{}.{}", h.class, skill))?;
        let requirements = raw["upgrade_requirements"]
            .as_array()
            .filter(|v| v.len() == 1)
            .ok_or("unsupported camping upgrade shape")?;
        let r = &requirements[0];
        if r["code"] != "0"
            || r["prerequisite_requirements"]
                .as_array()
                .is_none_or(|v| !v.is_empty())
        {
            return Err("unsupported camping prerequisites".into());
        }
        let discount = self.discount(
            self.definition
                .data
                .building("camping_trainer", "camping_skill_cost_discount_upgrades")?,
        )? + self.modifier(h, "upgrade_discount", "camping_skill", None)?;
        if self
            .state
            .campaign
            .as_ref()
            .is_some_and(|c| c.free("camping_skill"))
        {
            return self.cost_rows(&r["currency_cost"], 0.).map(|mut cost| {
                for n in cost.values_mut() {
                    *n = 0;
                }
                cost
            });
        }
        let discount = discount
            + self
                .state
                .campaign
                .as_ref()
                .map_or(0., |c| c.sum("upgrade_tag_discount", "camping_skill"));
        self.cost_rows(&r["currency_cost"], discount)
    }
    pub(super) fn treatment_quote(
        &self,
        hero: u64,
        facility: &str,
        treatment: &Treatment,
    ) -> Result<BTreeMap<String, u32>> {
        if !self.available(hero) {
            return Err("hero unavailable for treatment".into());
        }
        let building = facility.split('.').next().ok_or("invalid facility")?;
        if !self.unlocked(building)? {
            return Err("treatment facility locked".into());
        }
        let a = self.definition.data.activity(facility)?;
        let activity = facility
            .split_once('.')
            .ok_or("invalid activity identifier")?
            .1;
        if self
            .state
            .campaign
            .as_ref()
            .is_some_and(|c| c.has("activity_lock", activity))
        {
            return Err("activity locked by town event".into());
        }
        let slots = uint(self.tier(&a["slot_upgrades"], facility)?, "number_of_slots")? as usize;
        let used = self
            .state
            .jobs
            .iter()
            .filter(|j| j.facility == facility)
            .count()
            + self.state.caretaker.get(facility).copied().unwrap_or(0);
        if used >= slots {
            return Err("facility slots full".into());
        }
        let h = self.hero(hero)?;
        if self
            .class_activity018(&h.class)
            .is_some_and(|r| !r.allowed.is_empty() && !r.allowed.contains(activity))
        {
            return Err("class source forbids this activity".into());
        }
        if let Some(reqs) = a["requirements"].as_array() {
            for req in reqs {
                match req["type"].as_str() {
                    Some("not_have_quirks") => {
                        if req["data"]["quirk_library_names"]
                            .as_array()
                            .ok_or("missing activity quirk constraints")?
                            .iter()
                            .any(|id| id.as_str().is_some_and(|id| h.quirks.contains_key(id)))
                        {
                            return Err("hero quirk prevents activity".into());
                        }
                    }
                    Some("is_diseased") => {
                        if h.state.diseases.is_empty() {
                            return Err("hero has no disease".into());
                        }
                    }
                    _ => return Err("unsupported town activity requirement".into()),
                }
            }
        }
        // The contagion owner is deliberately outside this permanent standard-facility slice.
        let cursed = |h: &Hero| {
            h.state.diseases.iter().any(|id| {
                library::unique_json(&self.definition.data.base.quirk_defs, id, "disease")
                    .is_ok_and(|v| v.get("contagious_class_id").is_some())
            })
        };
        if self.state.campaign.is_none()
            && self.state.jobs.iter().any(|j| {
                j.facility.split('.').next() == Some(building)
                    && (cursed(h) || self.hero(j.hero).is_ok_and(cursed))
            })
        {
            return Err("town contagion interaction requires campaign contagion host".into());
        }
        let quote_one = |t: &Treatment| -> Result<u32> {
            let key = match t {
                Treatment::Stress => {
                    if !["abbey", "tavern"].contains(&building) {
                        return Err("not a stress relief activity".into());
                    }
                    if h.state.stress == 0 && matches!(h.state.resolve, Resolve::Calm) {
                        return Err("hero does not need stress relief".into());
                    }
                    "cost_upgrades"
                }
                Treatment::Disease(id) => {
                    if facility != "sanitarium.disease_treatment" || !h.state.diseases.contains(id)
                    {
                        return Err("wrong disease treatment target".into());
                    }
                    let q = library::unique_json(
                        &self.definition.data.base.quirk_defs,
                        id,
                        "disease treatment",
                    )?;
                    let source_unlocked = self.state.campaign.is_some()
                        && q["contagious_class_id"] == "disease_vampire_passive"
                        && q["required_plot_quests_to_modify"]
                            .as_array()
                            .is_some_and(|ids| {
                                !ids.is_empty()
                                    && ids.iter().all(|id| {
                                        id.as_str()
                                            .is_some_and(|id| self.state.completed_ids.contains(id))
                                    })
                            });
                    if (q["can_modify_in_activity"] != true && !source_unlocked)
                        || q["required_plot_quests_to_modify"]
                            .as_array()
                            .is_some_and(|v| {
                                v.iter().any(|id| {
                                    id.as_str()
                                        .is_none_or(|id| !self.state.completed_ids.contains(id))
                                })
                            })
                    {
                        return Err("disease cannot be treated in current progress".into());
                    }
                    "disease_quirk_cost_upgrades"
                }
                Treatment::RemoveQuirk(id) | Treatment::LockQuirk(id) => {
                    if facility != "sanitarium.treatment" || !h.quirks.contains_key(id) {
                        return Err("wrong quirk treatment target".into());
                    }
                    let q = library::unique_json(
                        &self.definition.data.base.quirk_defs,
                        id,
                        "quirk treatment",
                    )?;
                    if q["is_disease"] != false || q["can_modify_in_activity"] != true {
                        return Err("quirk cannot be modified".into());
                    }
                    if matches!(t, Treatment::LockQuirk(_)) {
                        if q["is_positive"] != true || h.quirks[id].locked {
                            return Err("quirk cannot be reinforced".into());
                        }
                        let cap = uint(
                            &self.definition.data.base.globals,
                            "quirks_max_locked_positive",
                        )? as usize;
                        let used = h
                            .quirks
                            .iter()
                            .filter(|(id, f)| {
                                f.locked
                                    && library::unique_json(
                                        &self.definition.data.base.quirk_defs,
                                        id,
                                        "quirk",
                                    )
                                    .is_ok_and(|v| {
                                        v["is_positive"] == true
                                            && v["random_chance"].as_f64().is_some_and(|p| p > 0.)
                                    })
                            })
                            .count();
                        if used >= cap {
                            return Err("positive reinforcement slots full".into());
                        }
                    }
                    if q["is_positive"] == true {
                        "positive_quirk_cost_upgrades"
                    } else if h.quirks[id].locked {
                        "permanent_negative_quirk_cost_upgrades"
                    } else {
                        "negative_quirk_cost_upgrades"
                    }
                }
                Treatment::Combined { .. } => return Err("nested quirk treatment".into()),
            };
            let row = self.tier(&a[key], facility)?;
            if row["cost_currency"]["type"] != "gold" {
                return Err("unsupported treatment currency".into());
            }
            uint(&row["cost_currency"], "amount")
        };
        let base = if let Treatment::Combined {
            negative,
            positive,
            lock_positive,
        } = treatment
        {
            let nq = library::unique_json(
                &self.definition.data.base.quirk_defs,
                negative,
                "combined quirk treatment",
            )?;
            let pq = library::unique_json(
                &self.definition.data.base.quirk_defs,
                positive,
                "combined quirk treatment",
            )?;
            if nq["is_positive"] != false || pq["is_positive"] != true {
                return Err("combined treatment needs one quirk of each sign".into());
            }
            quote_one(&Treatment::RemoveQuirk(negative.clone()))?
                .checked_add(quote_one(&if *lock_positive {
                    Treatment::LockQuirk(positive.clone())
                } else {
                    Treatment::RemoveQuirk(positive.clone())
                })?)
                .ok_or("combined treatment cost overflow")?
        } else {
            quote_one(treatment)?
        };
        let level = self.level(h.xp)? as usize;
        let values = self
            .definition
            .data
            .base
            .estate_sources
            .get("campaign/estate/estate.json")
            .ok_or("missing estate activity multipliers")?;
        let mut tables = BTreeMap::new();
        for value in values {
            library::insert_json(
                &mut tables,
                "multiplier".into(),
                json!({"value":value["hero_activity_cost_multiplier_table"]}),
            );
        }
        let table = library::unique_json(&tables, "multiplier", "activity cost multipliers")?;
        let multiplier = table["value"]
            .get(level)
            .and_then(Value::as_f64)
            .ok_or("missing treatment resolve multiplier")?;
        let factor = self.state.campaign.as_ref().map_or(1., |c| {
            if c.has("free_activity", activity) {
                0.
            } else {
                (1. + c.sum("activity_cost_change", activity)).max(0.)
            }
        });
        let value = self.money(base, multiplier * factor)?;
        Ok(BTreeMap::from([(self.currency("gold")?, value)]))
    }
    pub(super) fn apply(&mut self, action: Action) -> Result<()> {
        match action {
            Action::ToggleSkill { hero, skill } => {
                let h = self.hero(hero)?;
                if !h.skills.contains_key(&skill) {
                    return Err("cannot select unlearned skill".into());
                }
                let mut skills = h.selected.clone();
                if let Some(i) = skills.iter().position(|id| id == &skill) {
                    skills.remove(i);
                } else {
                    skills.push(skill);
                }
                self.apply(Action::SelectSkills { hero, skills })?;
            }
            Action::ToggleCamping { hero, skill } => {
                let h = self.hero(hero)?;
                if !h.camping.contains(&skill) {
                    return Err("cannot select unlearned camping skill".into());
                }
                let mut skills = h.selected_camping.clone();
                if let Some(i) = skills.iter().position(|id| id == &skill) {
                    skills.remove(i);
                } else {
                    skills.push(skill);
                }
                self.apply(Action::SelectCamping { hero, skills })?;
            }
            Action::AssignParty { hero, slot } => {
                if slot >= 4 || slot > self.state.party.len() {
                    return Err("invalid party slot".into());
                }
                let mut party = self.state.party.clone();
                if let Some(hero) = hero {
                    if let Some(old) = party.iter().position(|id| *id == hero) {
                        if slot >= party.len() {
                            return Err("hero already in party".into());
                        }
                        party.swap(old, slot);
                    } else if slot == party.len() {
                        party.push(hero);
                    } else {
                        party[slot] = hero;
                    }
                } else {
                    if slot >= party.len() {
                        return Err("empty party slot".into());
                    }
                    party.remove(slot);
                }
                if party.is_empty() {
                    self.state.party.clear();
                } else {
                    self.apply(Action::Party(party))?;
                }
            }
            Action::Upgrade { hero, tree, code } => {
                let cost = self.upgrade_quote(hero, &tree, &code)?;
                self.charge(&cost)?;
                self.state
                    .purchases
                    .insert((hero, tree.clone(), code.clone()));
                if let Some(id) = hero {
                    let i = self.hero_index(id)?;
                    let t = self.definition.data.tree(&tree)?;
                    let tags = t["tags"].as_array().unwrap();
                    let n = code
                        .parse::<usize>()
                        .map_err(|_| "invalid hero rank code")?;
                    if tags.iter().any(|v| v == "weapon") {
                        if self.state.heroes[i].weapon != n {
                            return Err("weapon upgrades must be sequential".into());
                        }
                        self.state.heroes[i].weapon = n + 1;
                    } else if tags.iter().any(|v| v == "armour") {
                        if self.state.heroes[i].armour != n {
                            return Err("armour upgrades must be sequential".into());
                        }
                        self.state.heroes[i].armour = n + 1;
                    } else {
                        let skill = tree.split_once('.').ok_or("bad skill tree")?.1.to_string();
                        if self.state.heroes[i]
                            .skills
                            .get(&skill)
                            .copied()
                            .map_or(n != 0, |old| old as usize + 1 != n)
                        {
                            return Err("skill upgrades must be sequential".into());
                        }
                        self.state.heroes[i].skills.insert(skill, n as i32);
                    }
                    self.blueprint(&self.state.heroes[i], 1, false)?;
                }
            }
            Action::LearnCamp { hero, skill } => {
                let cost = self.camp_quote(hero, &skill)?;
                self.charge(&cost)?;
                let i = self.hero_index(hero)?;
                self.state.heroes[i].camping.insert(skill);
            }
            Action::SelectSkills { hero, skills } => {
                if !self.available(hero) {
                    return Err("hero unavailable".into());
                }
                let i = self.hero_index(hero)?;
                let doc = self
                    .definition
                    .game
                    .hero_document(&self.state.heroes[i].class)?;
                if doc
                    .records_keyed("skill_selection")
                    .next()
                    .is_some_and(|r| {
                        dd_data::combat::text(r, "can_select_combat_skills").ok() == Some("false")
                    })
                    && skills.iter().cloned().collect::<BTreeSet<_>>()
                        != self.state.heroes[i].skills.keys().cloned().collect()
                {
                    return Err("class has fixed combat skills".into());
                }
                if skills.is_empty() || skills.iter().collect::<BTreeSet<_>>().len() != skills.len()
                {
                    return Err("invalid skill selection".into());
                }
                self.state.heroes[i].selected = skills;
                self.blueprint(&self.state.heroes[i], 1, false)?;
            }
            Action::SelectCamping { hero, skills } => {
                if !self.available(hero) {
                    return Err("hero unavailable".into());
                }
                let i = self.hero_index(hero)?;
                if skills.len() > self.definition.data.candidates.camping_slots
                    || skills.iter().collect::<BTreeSet<_>>().len() != skills.len()
                    || skills
                        .iter()
                        .any(|id| !self.state.heroes[i].camping.contains(id))
                {
                    return Err("invalid camping skill selection".into());
                }
                self.state.heroes[i].selected_camping = skills;
            }
            Action::Equip {
                hero,
                instance,
                slot,
            } => {
                if !self.available(hero)
                    || slot >= self.definition.data.candidates.trinket_slots
                    || self
                        .state
                        .assets
                        .iter()
                        .any(|a| a.owner == Some((hero, slot)))
                {
                    return Err("equipment slot unavailable".into());
                }
                let index = self
                    .state
                    .assets
                    .iter()
                    .position(|a| a.instance == instance && a.owner.is_none())
                    .ok_or("trinket not in estate inventory")?;
                let definition_id = &self.state.assets[index].state.definition.id;
                if self.state.campaign.is_some()
                    && self.state.assets.iter().any(|a| {
                        a.owner.is_some_and(|(id, _)| id == hero)
                            && a.state.definition.id == *definition_id
                    })
                {
                    return Err("duplicate build entry".into());
                }
                self.state.assets[index].owner = Some((hero, slot));
                self.blueprint(self.hero(hero)?, 1, false)?;
            }
            Action::Unequip { hero, slot } => {
                if !self.available(hero) {
                    return Err("hero unavailable".into());
                }
                let a = self
                    .state
                    .assets
                    .iter_mut()
                    .find(|a| a.owner == Some((hero, slot)))
                    .ok_or("empty equipment slot")?;
                a.owner = None;
            }
            Action::Sell { instance } => {
                if !self.unlocked("nomad_wagon")? {
                    return Err("wagon locked".into());
                }
                let i = self
                    .state
                    .assets
                    .iter()
                    .position(|a| a.instance == instance && a.owner.is_none())
                    .ok_or("trinket not free in estate")?;
                let gold = self.trinket_price(&self.state.assets[i].state.definition.id, true)?;
                self.credit(&BTreeMap::from([(self.currency("gold")?, gold)]))?;
                self.state.assets.remove(i);
            }
            Action::Buy { index } => {
                if !self.unlocked("nomad_wagon")? {
                    return Err("wagon locked".into());
                }
                let id = self
                    .state
                    .wagon
                    .get(index)
                    .ok_or("missing wagon item")?
                    .clone();
                let cost =
                    BTreeMap::from([(self.currency("gold")?, self.trinket_price(&id, false)?)]);
                self.charge(&cost)?;
                self.add_asset(&id, None)?;
                self.state.wagon.remove(index);
            }
            Action::Exchange { index, batches } => {
                if batches == 0 {
                    return Err("zero exchange quantity".into());
                }
                let row = self
                    .definition
                    .data
                    .exchange
                    .get(index)
                    .ok_or("unknown exchange")?;
                let from = self.currency(
                    row["exchange_from_type"]
                        .as_str()
                        .ok_or("bad exchange currency")?,
                )?;
                let to = self.currency(
                    row["exchange_to_type"]
                        .as_str()
                        .ok_or("bad exchange currency")?,
                )?;
                let spend = uint(row, "exchange_from_amount")?
                    .checked_mul(batches)
                    .ok_or("exchange overflow")?;
                let gain = uint(row, "exchange_to_amount")?
                    .checked_mul(batches)
                    .ok_or("exchange overflow")?;
                self.charge(&BTreeMap::from([(from, spend)]))?;
                self.credit(&BTreeMap::from([(to, gain)]))?;
            }
            Action::Hire { guid } => {
                if self.state.heroes.iter().filter(|h| h.state.alive).count()
                    >= self.capacity()? as usize
                {
                    return Err("roster full".into());
                }
                let i = self
                    .state
                    .recruits
                    .iter()
                    .position(|h| h.guid == guid)
                    .ok_or("recruit not available")?;
                let h = self.state.recruits.remove(i);
                self.install_hero_purchases(&h)?;
                self.state.heroes.push(h);
            }
            Action::Dismiss { hero } => {
                if !self.available(hero) {
                    return Err("hero cannot be dismissed".into());
                }
                let i = self.hero_index(hero)?;
                for a in &mut self.state.assets {
                    if a.owner.is_some_and(|(id, _)| id == hero) {
                        a.owner = None;
                    }
                }
                self.state.party.retain(|id| *id != hero);
                self.state.purchases.retain(|(id, _, _)| *id != Some(hero));
                self.state.heroes.remove(i);
                self.state.log.push(json!({"dismissed":hero}));
            }
            Action::Queue {
                hero,
                facility,
                treatment,
            } => {
                let cost = self.treatment_quote(hero, &facility, &treatment)?;
                self.charge(&cost)?;
                self.state.party.retain(|id| *id != hero);
                self.state.jobs.push(Job {
                    hero,
                    facility,
                    treatment,
                    paid: cost,
                    locked_until: None,
                });
            }
            Action::Cancel { hero } => {
                let i = self
                    .state
                    .jobs
                    .iter()
                    .position(|j| j.hero == hero && j.locked_until.is_none())
                    .ok_or("no cancellable treatment")?;
                let j = self.state.jobs.remove(i);
                self.credit(&j.paid)?;
            }
            Action::BuildDistrict(id) => {
                if self
                    .state
                    .campaign
                    .as_ref()
                    .is_some_and(|c| !c.districts_unlocked)
                {
                    return Err("district construction not yet unlocked by campaign event".into());
                }
                if self.state.districts.contains(&id) {
                    return Err("district already built".into());
                }
                let d = self.definition.data.district(&id)?;
                let cost = self.cost_rows(&d["currency_cost"], 0.)?;
                self.charge(&cost)?;
                self.state.districts.insert(id);
            }
            Action::Party(ids) => {
                if ids.is_empty()
                    || ids.len() > 4
                    || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
                    || ids.iter().any(|id| !self.available(*id))
                {
                    return Err("invalid departing party".into());
                }
                let heroes = ids
                    .iter()
                    .map(|id| self.blueprint(self.hero(*id)?, 1, false))
                    .collect::<Result<Vec<_>>>()?;
                for (i, h) in heroes.iter().enumerate() {
                    if heroes.iter().enumerate().any(|(j, other)| {
                        i != j
                            && h.mechanics
                                .hero
                                .incompatible_party_tags
                                .iter()
                                .any(|t| other.mechanics.hero.tags.contains(t))
                    }) {
                        return Err("incompatible party tags".into());
                    }
                }
                self.state.party = ids;
            }
        }
        Ok(())
    }
    fn upgrade_candidates(&self) -> Result<Vec<Action>> {
        let mut actions = vec![];
        for tree in self.definition.data.trees.keys() {
            let t = self.definition.data.tree(tree)?;
            if t["is_instanced"] == true {
                for h in self
                    .state
                    .heroes
                    .iter()
                    .filter(|h| tree.starts_with(&format!("{}.", h.class)))
                {
                    for node in t["requirements"].as_array().unwrap() {
                        actions.push(Action::Upgrade {
                            hero: Some(h.guid),
                            tree: tree.clone(),
                            code: node["code"].as_str().unwrap().into(),
                        });
                    }
                }
            } else {
                for node in t["requirements"].as_array().unwrap() {
                    actions.push(Action::Upgrade {
                        hero: None,
                        tree: tree.clone(),
                        code: node["code"].as_str().unwrap().into(),
                    });
                }
            }
        }
        Ok(actions)
    }
    pub(super) fn campaign_upgrade_quotes(&self) -> Result<Vec<Choice>> {
        let mut quotes = vec![];
        for action in self.upgrade_candidates()? {
            let Action::Upgrade { hero, tree, code } = &action else {
                unreachable!()
            };
            match self.upgrade_quote(*hero, tree, code) {
                Ok(cost) => quotes.push(Choice { action, cost }),
                Err(e) if unavailable(&e) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(quotes)
    }
    pub(super) fn choices(&self) -> Result<Vec<Choice>> {
        let _profile = dd_data::telemetry024c::scope("town_choices");
        if self.state.departure.is_some() {
            return Ok(vec![]);
        }
        let mut actions = self.upgrade_candidates()?;
        for h in &self.state.heroes {
            for key in self
                .definition
                .data
                .camps
                .keys()
                .filter(|k| k.starts_with(&format!("{}.", h.class)))
            {
                actions.push(Action::LearnCamp {
                    hero: h.guid,
                    skill: key.split_once('.').unwrap().1.into(),
                });
            }
            for a in &self.state.assets {
                if let Some((id, slot)) = a.owner {
                    if id == h.guid {
                        actions.push(Action::Unequip { hero: h.guid, slot });
                    }
                } else {
                    for slot in 0..self.definition.data.candidates.trinket_slots {
                        actions.push(Action::Equip {
                            hero: h.guid,
                            instance: a.instance,
                            slot,
                        });
                    }
                }
            }
            actions.push(Action::Dismiss { hero: h.guid });
            actions.push(Action::Cancel { hero: h.guid });
            for facility in self.definition.data.activities.keys() {
                if facility.starts_with("sanitarium.") {
                    for id in h.quirks.keys() {
                        actions.push(Action::Queue {
                            hero: h.guid,
                            facility: facility.clone(),
                            treatment: Treatment::RemoveQuirk(id.clone()),
                        });
                        actions.push(Action::Queue {
                            hero: h.guid,
                            facility: facility.clone(),
                            treatment: Treatment::LockQuirk(id.clone()),
                        });
                    }
                    for id in &h.state.diseases {
                        actions.push(Action::Queue {
                            hero: h.guid,
                            facility: facility.clone(),
                            treatment: Treatment::Disease(id.clone()),
                        });
                    }
                } else {
                    actions.push(Action::Queue {
                        hero: h.guid,
                        facility: facility.clone(),
                        treatment: Treatment::Stress,
                    });
                }
            }
        }
        for h in &self.state.heroes {
            for id in h.skills.keys() {
                actions.push(Action::ToggleSkill {
                    hero: h.guid,
                    skill: id.clone(),
                });
            }
            for id in &h.camping {
                actions.push(Action::ToggleCamping {
                    hero: h.guid,
                    skill: id.clone(),
                });
            }
            for slot in 0..=self.state.party.len().min(3) {
                actions.push(Action::AssignParty {
                    hero: Some(h.guid),
                    slot,
                });
            }
            let positive = h
                .quirks
                .keys()
                .filter(|id| {
                    library::unique_json(&self.definition.data.base.quirk_defs, id, "quirk")
                        .is_ok_and(|q| q["is_positive"] == true)
                })
                .cloned()
                .collect::<Vec<_>>();
            let negative = h
                .quirks
                .keys()
                .filter(|id| {
                    library::unique_json(&self.definition.data.base.quirk_defs, id, "quirk")
                        .is_ok_and(|q| q["is_positive"] == false)
                })
                .cloned()
                .collect::<Vec<_>>();
            for p in &positive {
                for n in &negative {
                    for lock_positive in [true, false] {
                        actions.push(Action::Queue {
                            hero: h.guid,
                            facility: "sanitarium.treatment".into(),
                            treatment: Treatment::Combined {
                                positive: p.clone(),
                                negative: n.clone(),
                                lock_positive,
                            },
                        });
                    }
                }
            }
        }
        for slot in 0..self.state.party.len() {
            actions.push(Action::AssignParty { hero: None, slot });
        }
        for a in &self.state.assets {
            actions.push(Action::Sell {
                instance: a.instance,
            });
        }
        for index in 0..self.state.wagon.len() {
            actions.push(Action::Buy { index });
        }
        for index in 0..self.definition.data.exchange.len() {
            actions.push(Action::Exchange { index, batches: 1 });
        }
        for h in &self.state.recruits {
            actions.push(Action::Hire { guid: h.guid });
        }
        actions.extend(
            self.definition
                .data
                .districts
                .keys()
                .cloned()
                .map(Action::BuildDistrict),
        );
        let mut choices = vec![];
        for action in actions {
            let cost = match &action {
                Action::Upgrade { hero, tree, code } => self.upgrade_quote(*hero, tree, code),
                Action::LearnCamp { hero, skill } => self.camp_quote(*hero, skill),
                Action::Buy { index } => self
                    .trinket_price(&self.state.wagon[*index], false)
                    .map(|n| BTreeMap::from([("gold:".into(), n)])),
                Action::BuildDistrict(id) => {
                    self.cost_rows(&self.definition.data.district(id)?["currency_cost"], 0.)
                }
                Action::Queue {
                    hero,
                    facility,
                    treatment,
                } => self.treatment_quote(*hero, facility, treatment),
                Action::Exchange { index, batches } => {
                    let r = &self.definition.data.exchange[*index];
                    Ok(BTreeMap::from([(
                        self.currency(r["exchange_from_type"].as_str().unwrap())?,
                        uint(r, "exchange_from_amount")? * *batches,
                    )]))
                }
                _ => Ok(BTreeMap::new()),
            };
            let cost = match cost {
                Ok(cost) => cost,
                Err(e) if unavailable(&e) => continue,
                Err(e) => return Err(e),
            };
            let mut trial = self.clone();
            match trial.apply(action.clone()) {
                Ok(()) => choices.push(Choice { action, cost }),
                Err(e) if unavailable(&e) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(choices)
    }
}

// Only known legality failures suppress a generated choice. Source/unsupported errors escape.
fn unavailable(error: &str) -> bool {
    [
        "armour upgrades must be sequential",
        "camping skill already learned",
        "camping training unavailable",
        "class has fixed combat skills",
        "class source forbids this activity",
        "combined treatment needs one quirk of each sign",
        "disease cannot be treated in current progress",
        "district already built",
        "district construction not yet unlocked by campaign event",
        "activity locked by town event",
        "equipment slot unavailable",
        "facility locked",
        "facility slots full",
        "hero already in party",
        "hero cannot be dismissed",
        "hero does not need stress relief",
        "hero has no disease",
        "hero quirk prevents activity",
        "hero unavailable",
        "hero unavailable for training",
        "hero unavailable for treatment",
        "incompatible party tags",
        "invalid camping skill selection",
        "invalid departing party",
        "invalid party slot",
        "invalid skill selection",
        "not a stress relief activity",
        "positive reinforcement slots full",
        "quirk cannot be modified",
        "quirk cannot be reinforced",
        "resolve level insufficient for training",
        "roster full",
        "skill upgrades must be sequential",
        "town contagion interaction requires campaign contagion host",
        "treatment facility locked",
        "upgrade prerequisite not purchased",
        "wagon locked",
        "weapon upgrades must be sequential",
        "wrong class upgrade",
        "wrong disease treatment target",
        "wrong owner or already purchased upgrade",
        "wrong quirk treatment target",
        "insufficient estate assets",
        "trinket not free in estate",
        "no cancellable treatment",
        "trinket class restriction",
        "duplicate build entry",
        "too many equipped skills for class",
        "trinket has no gold trading price",
    ]
    .contains(&error)
}
