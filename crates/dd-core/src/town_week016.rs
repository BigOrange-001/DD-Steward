use super::*;

#[derive(Clone, Copy)]
pub(super) struct CampaignSettlement<'a> {
    pub advance: bool,
    pub regroup: bool,
    pub physical: &'a [TrinketState],
    pub quirks: &'a [Value],
    pub raid_quirks: &'a [crate::expedition017::RaidQuirk],
}
impl Session {
    pub(super) fn region_award(&self, length: u32) -> Result<u32> {
        self.definition.data.progression["dungeon"]["quest_completion_xp_table"]
            .get(length as usize)
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| "missing region progression award".into())
    }
    pub(super) fn update_highest(&mut self) -> Result<()> {
        let t = self.definition.data.progression["dungeon"]["level_threshold_table"]
            .as_array()
            .ok_or("missing region thresholds")?;
        let mut highest = 0;
        for xp in self.state.dungeon_xp.values() {
            let n = t
                .iter()
                .filter(|v| v.as_u64().is_some_and(|n| n <= u64::from(*xp)))
                .count()
                .saturating_sub(1) as u32;
            highest = highest.max(n);
        }
        self.state.highest_dungeon = highest;
        Ok(())
    }

    pub(super) fn roll_camping(&mut self, h: &mut Hero, extra: u32) -> Result<()> {
        self.roll_camping_counts(h, extra, None)
    }
    fn roll_camping_counts(
        &mut self,
        h: &mut Hero,
        extra: u32,
        counts: Option<(usize, usize)>,
    ) -> Result<()> {
        let doc = self.definition.game.hero_document(&h.class)?;
        let row = doc
            .records_keyed("generation")
            .next()
            .ok_or("missing class generation")?;
        let specific = dd_data::combat::number(row, "number_of_class_specific_camping_skills")?
            as usize
            + extra as usize;
        let shared = dd_data::combat::number(row, "number_of_shared_camping_skills")? as usize;
        let (specific, shared) = counts.unwrap_or((specific, shared));
        let exclusive = h.class == "flagellant";
        let counts = if exclusive {
            vec![(false, specific + shared)]
        } else {
            vec![(false, specific), (true, shared)]
        };
        for (is_shared, count) in counts {
            let mut pool = self
                .definition
                .data
                .camps
                .iter()
                .filter(|(id, v)| {
                    id.starts_with(&format!("{}.", h.class))
                        && !h.camping.contains(v["id"].as_str().unwrap_or(""))
                        && v["hero_classes"].as_array().is_some()
                })
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            // The original configuration's class threshold describes shared versus class-specific skills.
            pool.retain(|key| {
                let id = self.definition.data.camps[key]["id"].as_str().unwrap_or("");
                let classes = self
                    .definition
                    .data
                    .camps
                    .keys()
                    .filter(|k| k.split_once('.').is_some_and(|(_, s)| s == id))
                    .count();
                (classes > self.definition.data.camp_threshold as usize) == is_shared
            });
            for i in (1..pool.len()).rev() {
                let j = self.state.rng.below((i + 1) as u64) as usize;
                pool.swap(i, j);
            }
            if pool.is_empty() && count > 0 {
                return Err(format!(
                    "missing {} camping pool shared={is_shared}",
                    h.class
                ));
            }
            h.camping.extend(
                pool.into_iter()
                    .take(count)
                    .map(|id| id.split_once('.').unwrap().1.to_string()),
            );
        }
        Ok(())
    }
    pub(super) fn generate(
        &mut self,
        class: &str,
        level: u32,
        extra_positive: u32,
        extra_negative: u32,
        extra_combat: u32,
        extra_camp: u32,
    ) -> Result<Hero> {
        self.generate_using(
            class,
            level,
            [extra_positive, extra_negative, extra_combat, extra_camp],
            None,
        )
    }
    pub(super) fn generate_prisoner(&mut self, class: &str, source: &Value) -> Result<Hero> {
        self.generate_using(class, uint(source, "level")?, [0; 4], Some(source))
    }
    pub(super) fn generate_mercenary019(
        &mut self,
        class: &str,
        source: &dd_data::content019::MercenaryStore,
    ) -> Result<Hero> {
        let doc = self.definition.game.hero_document(class)?;
        let generation = doc
            .records_keyed("generation")
            .next()
            .ok_or("missing mercenary generation")?;
        let mut params = serde_json::Map::new();
        params.insert("level".into(), json!(source.starting_resolve_level));
        for (target, lo, hi, minimum) in [
            (
                "starting_positive_quirks",
                "number_of_positive_quirks_min",
                "number_of_positive_quirks_max",
                source.minimum_number_of_positive_quirks,
            ),
            (
                "starting_negative_quirks",
                "number_of_negative_quirks_min",
                "number_of_negative_quirks_max",
                source.minimum_number_of_negative_quirks,
            ),
        ] {
            params.insert(target.into(),json!({"min":(dd_data::combat::number(generation,lo)? as u32).max(minimum),"max":(dd_data::combat::number(generation,hi)? as u32).max(minimum)}));
        }
        let combat = self.definition.game.hero_skill_ids(class)?.len() as u32;
        params.insert(
            "starting_combat_skills".into(),
            json!({"min":combat,"max":combat}),
        );
        let mut h = self.generate_using(
            class,
            source.starting_resolve_level,
            [0; 4],
            Some(&Value::Object(params)),
        )?;
        for id in &source.starting_quirks {
            h.quirks.insert(
                id.clone(),
                QuirkFlags {
                    locked: true,
                    mission_count: 0,
                },
            );
        }
        let bp = self.blueprint(&h, source.starting_resolve_level, false)?;
        let kernel = PartyKernel::new(
            vec![bp],
            self.definition.game.rules.clone(),
            self.state.rng.clone(),
        )?;
        let (mut states, rng) = kernel.export();
        self.state.rng = rng;
        h.state = states.remove(0);
        Ok(h)
    }
    fn generate_using(
        &mut self,
        class: &str,
        level: u32,
        extras: [u32; 4],
        source: Option<&Value>,
    ) -> Result<Hero> {
        let [extra_positive, extra_negative, extra_combat, extra_camp] = extras;
        let range = |v: &Value| -> Result<(u32, u32)> {
            dd_data::campaign017::fields(v, &["min", "max"], "prisoner generation range")?;
            let lo = uint(v, "min")?;
            let hi = uint(v, "max")?;
            if lo > hi || hi > 100 {
                return Err("invalid prisoner generation range".into());
            }
            Ok((lo, hi))
        };
        let combat_override = if let Some(v) = source {
            let (lo, hi) = range(&v["starting_combat_skills"])?;
            Some(lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32)
        } else {
            None
        };
        let doc = self.definition.game.hero_document(class)?.clone();
        let generation = doc
            .records_keyed("generation")
            .next()
            .ok_or("missing source generation")?;
        let get = |key: &str| -> Result<u32> {
            if let Some(source) = source {
                let found = match key {
                    "number_of_positive_quirks_min" => {
                        Some(range(&source["starting_positive_quirks"])?.0)
                    }
                    "number_of_positive_quirks_max" => {
                        Some(range(&source["starting_positive_quirks"])?.1)
                    }
                    "number_of_negative_quirks_min" => {
                        Some(range(&source["starting_negative_quirks"])?.0)
                    }
                    "number_of_negative_quirks_max" => {
                        Some(range(&source["starting_negative_quirks"])?.1)
                    }
                    "number_of_random_combat_skills" => combat_override,
                    _ => None,
                };
                if let Some(n) = found {
                    return Ok(n);
                }
            }
            let v = dd_data::combat::number(generation, key)?;
            if v < 0. || v.fract() != 0. || v > 100. {
                return Err("invalid generation count".into());
            }
            Ok(v as u32)
        };
        let mut ids = self.definition.game.hero_skill_ids(class)?;
        for i in (1..ids.len()).rev() {
            let j = self.state.rng.below((i + 1) as u64) as usize;
            ids.swap(i, j);
        }
        let guaranteed = doc
            .records_keyed("combat_skill")
            .filter(|r| {
                dd_data::combat::number(r, "level").ok() == Some(0.)
                    && dd_data::combat::text(r, "generation_guaranteed").ok() == Some("true")
            })
            .map(|r| dd_data::combat::text(r, "id").map(str::to_string))
            .collect::<Result<BTreeSet<_>>>()?;
        ids.sort_by_key(|id| !guaranteed.contains(id));
        let amount = (get("number_of_random_combat_skills")? + extra_combat) as usize;
        if guaranteed.len() > amount {
            return Err("guaranteed combat skills exceed source generation count".into());
        }
        ids.truncate(amount.min(ids.len()));
        if ids.is_empty() {
            return Err("empty generated skill set".into());
        }
        let mut quirks = BTreeMap::new();
        let mut blueprint = self.definition.game.hero(class, 0, 0, &[ids[0].as_str()])?;
        for positive in [true, false] {
            let lo = get(if positive {
                "number_of_positive_quirks_min"
            } else {
                "number_of_negative_quirks_min"
            })?;
            let hi = get(if positive {
                "number_of_positive_quirks_max"
            } else {
                "number_of_negative_quirks_max"
            })?;
            if lo > hi {
                return Err("invalid generation quirk range".into());
            }
            let extra = if positive {
                extra_positive
            } else {
                extra_negative
            };
            let max = if positive {
                self.definition.game.rules.quirks_max_positive
            } else {
                self.definition.game.rules.quirks_max_negative
            };
            let n = (lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32 + extra).min(max);
            for _ in 0..n {
                let mut pool = vec![];
                let mut weights = vec![];
                for id in self.definition.data.base.quirk_defs.keys() {
                    let q = library::unique_json(
                        &self.definition.data.base.quirk_defs,
                        id,
                        "generated quirk",
                    )?;
                    if q["is_positive"].as_bool() != Some(positive)
                        || q["is_disease"] != false
                        || quirks.contains_key(id)
                        || blueprint.mechanics.hero.incompatible_quirks.contains(id)
                    {
                        continue;
                    }
                    let w = q["random_chance"]
                        .as_f64()
                        .ok_or("missing quirk generation weight")?;
                    if w <= 0. {
                        continue;
                    }
                    let incompatible = |raw: &Value, set: &BTreeMap<String, QuirkFlags>| {
                        raw["incompatible_quirks"].as_array().is_some_and(|r| {
                            r.iter()
                                .any(|x| x.as_str().is_some_and(|x| set.contains_key(x)))
                        })
                    };
                    if incompatible(q, &quirks)
                        || quirks.keys().any(|old| {
                            library::unique_json(
                                &self.definition.data.base.quirk_defs,
                                old,
                                "generated quirk",
                            )
                            .is_ok_and(|v| {
                                v["incompatible_quirks"]
                                    .as_array()
                                    .is_some_and(|r| r.iter().any(|v| v == id))
                            })
                        })
                    {
                        continue;
                    }
                    pool.push(id.clone());
                    weights.push(w);
                }
                let i = self.choose(&weights)?;
                quirks.insert(pool[i].clone(), QuirkFlags::default());
            }
        }
        let max = doc
            .records_keyed("skill_selection")
            .next()
            .map(|r| dd_data::combat::number(r, "number_of_selected_combat_skills_max"))
            .transpose()?
            .unwrap_or(4.) as usize;
        let selected = ids.iter().take(max).cloned().collect::<Vec<_>>();
        let rank = |kind: &str, gear: bool| -> Result<usize> {
            let tree = self.definition.data.tree(&format!("{class}.{kind}"))?;
            let mut maximum = 0;
            for node in tree["requirements"]
                .as_array()
                .ok_or("missing recruit training tree")?
            {
                if uint(node, "prerequisite_resolve_level")? <= level {
                    let n = node["code"]
                        .as_str()
                        .ok_or("missing recruit rank")?
                        .parse::<usize>()
                        .map_err(|_| "invalid recruit rank")?;
                    maximum = maximum.max(n + usize::from(gear));
                }
            }
            Ok(maximum)
        };
        let weapon = rank("weapon", true)?;
        let armour = rank("armour", true)?;
        let skills = ids
            .iter()
            .map(|id| Ok((id.clone(), rank(id, false)? as i32)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        blueprint = self.definition.game.hero_ranked(
            class,
            weapon,
            armour,
            &selected
                .iter()
                .map(|id| (id.as_str(), skills[id]))
                .collect::<Vec<_>>(),
        )?;
        blueprint = self.definition.game.equip_build(
            blueprint,
            Build {
                quirks: quirks.keys().cloned().collect(),
                ..Default::default()
            },
        )?;
        let kernel = PartyKernel::new(
            vec![blueprint],
            self.definition.game.rules.clone(),
            self.state.rng.clone(),
        )?;
        let (mut states, rng) = kernel.export();
        self.state.rng = rng;
        let guid = self.state.next_guid;
        self.state.next_guid = self
            .state
            .next_guid
            .checked_add(1)
            .ok_or("hero identity overflow")?;
        let thresholds = self.definition.data.base.roster_variables["resolve_level_thresholds"]
            .as_array()
            .ok_or("missing experience table")?;
        let xp = thresholds
            .get(level as usize)
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or("invalid recruit resolve level")?;
        let mut h = Hero {
            guid,
            name: format!("{class}_{guid}"),
            class: class.into(),
            xp,
            weapon,
            armour,
            skills,
            selected,
            camping: BTreeSet::new(),
            selected_camping: vec![],
            quirks,
            state: states.remove(0),
            missing_until: 0,
        };
        let counts =
            if let Some(source) = source.filter(|s| s.get("starting_camping_skills").is_some()) {
                let (lo, hi) = range(&source["starting_camping_skills"])?;
                let specific = lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32;
                let (lo, hi) = range(&source["starting_shared_camping_skills"])?;
                let shared = lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32;
                Some((specific as usize, shared as usize))
            } else {
                None
            };
        self.roll_camping_counts(&mut h, extra_camp, counts)?;
        h.selected_camping = h
            .camping
            .iter()
            .take(self.definition.data.candidates.camping_slots)
            .cloned()
            .collect();
        Ok(h)
    }
    pub(super) fn owned_rewards(&self) -> Vec<crate::initial013::Reward> {
        self.state
            .assets
            .iter()
            .map(|a| crate::initial013::Reward {
                kind: "trinket".into(),
                id: a.state.definition.id.clone(),
                amount: 1,
            })
            .collect()
    }
    pub(super) fn draw_trinket(&mut self, rarity: &str, extra: &[String]) -> Result<String> {
        self.draw_quest_trinket(rarity, extra, "")
    }
    pub(super) fn draw_quest_trinket(
        &mut self,
        rarity: &str,
        extra: &[String],
        dungeon: &str,
    ) -> Result<String> {
        let mut owned = self.owned_rewards();
        owned.extend(extra.iter().map(|id| crate::initial013::Reward {
            kind: "trinket".into(),
            id: id.clone(),
            amount: 1,
        }));
        let mut resolver = crate::loot015::Resolver {
            curios: &self.definition.data.base.curios,
            difficulty: 0,
            dungeon: dungeon.into(),
            hero_classes: self.definition.game.hero_ids().cloned().collect(),
            rng: &mut self.state.rng,
            owned: &owned,
        };
        let r = resolver.trinket(rarity, &[])?;
        self.definition.trinket(&r.id)?;
        Ok(r.id)
    }
    pub(super) fn refresh_markets(&mut self) -> Result<()> {
        self.state.recruits.clear();
        self.state.wagon.clear();
        self.state.caretaker.clear();
        let normal = uint(
            self.store_tier("stage_coach.hero_recruit", "number_of_recruits_upgrades")?,
            "amount",
        )?;
        let target = uint(
            &self.definition.data.base.raid_source["data"]["party"],
            "start_heroes_size",
        )?;
        let living = self.state.heroes.iter().filter(|h| h.state.alive).count() as u32;
        let n = normal.max(target.saturating_sub(living));
        let mut deck = BTreeMap::new();
        for class in self.definition.game.hero_ids() {
            let d = self.definition.game.hero_document(class)?;
            let r = d
                .records_keyed("generation")
                .next()
                .ok_or("missing class generation")?;
            let eligible = match r.param("town_event_dependency") {
                Some(_) => self
                    .state
                    .recruit_events
                    .contains(dd_data::combat::text(r, "town_event_dependency")?),
                None => true,
            };
            if eligible && dd_data::combat::text(r, "is_generation_enabled")? == "true" {
                let cards = dd_data::combat::number(r, "number_of_cards_in_deck")?;
                let chance = dd_data::combat::number(r, "card_chance")?;
                if cards.fract() != 0. || cards < 0. || !(0. ..=1.).contains(&chance) {
                    return Err("invalid recruit class deck".into());
                }
                deck.insert(class.clone(), (cards as u32, chance));
            }
        }
        let upgrades =
            self.definition.data.store("stage_coach.hero_recruit")?["upgraded_recruits_upgrades"]
                .as_array()
                .ok_or("missing advanced recruit tiers")?
                .clone();
        let mut advanced = vec![];
        for row in upgrades {
            let tree = row["upgrade_tree_id"]
                .as_str()
                .ok_or("missing advanced recruit tree")?;
            let code = row["upgrade_requirement_code"]
                .as_str()
                .ok_or("missing advanced recruit code")?;
            if self.purchased(None, tree, code) {
                advanced.push(row);
            }
        }
        for slot in 0..n {
            let classes = deck.keys().cloned().collect::<Vec<_>>();
            let weights = classes
                .iter()
                .map(|id| f64::from(deck[id].0) * deck[id].1)
                .collect::<Vec<_>>();
            let class = classes[self.choose(&weights)?].clone();
            deck.get_mut(&class).unwrap().0 -= 1;
            let mut chosen = if slot == 0 {
                advanced
                    .iter()
                    .rev()
                    .find(|row| {
                        row["guaranteed_previous_raid_dead_hero_levels"]
                            .as_array()
                            .is_some_and(|levels| {
                                levels.iter().any(|level| {
                                    level.as_u64().is_some_and(|level| {
                                        self.state.previous_deaths.contains(&(level as u32))
                                    })
                                })
                            })
                    })
                    .cloned()
            } else {
                None
            };
            for row in advanced.iter().rev() {
                if chosen.is_some() {
                    break;
                }
                if self.state.rng.roll(
                    row["chance"]
                        .as_f64()
                        .ok_or("missing advanced recruit chance")?,
                ) {
                    chosen = Some(row.clone());
                    break;
                }
            }
            let (level, pos, neg, combat, camp) = if let Some(row) = chosen {
                (
                    uint(&row, "level")?,
                    uint(&row, "number_of_extra_positive_quirks")?,
                    uint(&row, "number_of_extra_negative_quirks")?,
                    uint(&row, "number_of_extra_combat_skills")?,
                    uint(&row, "number_of_extra_camping_skills")?,
                )
            } else {
                (0, 0, 0, 0, 0)
            };
            let h = self.generate(&class, level, pos, neg, combat, camp)?;
            self.state.recruits.push(h);
        }
        let store = self
            .definition
            .data
            .store("nomad_wagon.trinket_supply")?
            .clone();
        let n = uint(
            self.tier(
                &store["number_of_trinkets_upgrades"],
                "nomad_wagon.numitems",
            )?,
            "amount",
        )?;
        let rarities = store["rarity_generation_table"]
            .as_array()
            .ok_or("missing wagon rarity weights")?;
        let weights = rarities
            .iter()
            .map(|v| {
                v["chance"]
                    .as_f64()
                    .ok_or_else(|| "bad rarity weight".to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        for _ in 0..n {
            let i = self.choose(&weights)?;
            let id = self.draw_trinket(
                rarities[i]["rarity"].as_str().ok_or("bad rarity")?,
                &self.state.wagon.clone(),
            )?;
            self.state.wagon.push(id);
        }
        let mut eligible = vec![];
        for (id, a) in &self.definition.data.activities {
            if a["miscellaneous"]["caretaker_friendly"] == true
                && self.unlocked(id.split('.').next().unwrap())?
            {
                eligible.push(id.clone());
            }
        }
        if !eligible.is_empty() {
            for _ in 0..self.definition.data.candidates.caretaker_slots {
                let i = self.state.rng.below(eligible.len() as u64) as usize;
                *self.state.caretaker.entry(eligible[i].clone()).or_default() += 1;
            }
        }
        Ok(())
    }
}

impl Session {
    /// Trusted campaign owner acknowledges an original recruitment event; no event scheduler is implied.
    pub fn record_recruit_event(&mut self, event: &str) -> Result<()> {
        self.transaction(|s| {
            if s.state.departure.is_some() || s.state.recruit_events.contains(event) {
                return Err("recruit event already acknowledged or expedition active".into());
            }
            let known = s.definition.game.hero_ids().any(|id| {
                s.definition.game.hero_document(id).is_ok_and(|d| {
                    d.records_keyed("generation").any(|r| {
                        dd_data::combat::text(r, "town_event_dependency").ok() == Some(event)
                    })
                })
            });
            if !known {
                return Err("unknown recruitment event dependency".into());
            }
            s.state.recruit_events.insert(event.into());
            s.state.log.push(json!({"host_recruit_event":event}));
            Ok(())
        })
    }
    pub fn begin_sortie(
        &mut self,
        quest: HostQuest,
        acknowledged: BTreeSet<String>,
    ) -> Result<Deployment> {
        self.transaction(|s| {
            if s.state.departure.is_some() {
                return Err("expedition already active".into());
            }
            quest.validate(&s.definition.data.content.id()?)?;
            if quest.raw["dungeon"] == "farm"
                && !s
                    .state
                    .campaign
                    .as_ref()
                    .is_some_and(|c| c.owner.starts_with(super::campaign::content019::PROFILE))
            {
                return Err(
                    "farm campaign and mercenary owner is outside standard town scope".into(),
                );
            }
            if !quest.repeatable && s.state.completed_ids.contains(&quest.id) {
                return Err("plot mission already completed".into());
            }
            if s.state.party.is_empty() || s.state.party.iter().any(|id| !s.available(*id)) {
                return Err("no available departing party".into());
            }
            let party = s.state.party.clone();
            let mut heroes = vec![];
            for id in &party {
                let h = s.hero(*id)?;
                if matches!(h.state.resolve, Resolve::Calm) && h.state.stress >= 100 {
                    return Err("deferred resolve must be treated before deployment".into());
                }
                for id in h.quirks.keys() {
                    let raw = library::unique_json(
                        &s.definition.data.base.quirk_defs,
                        id,
                        "embark quirk",
                    )?;
                    if id == "shard_hungry"
                        && s.state.campaign.as_ref().is_some_and(|c| {
                            c.owner.starts_with(super::campaign::content019::PROFILE)
                        })
                    {
                        if !raw["required_plot_quests_to_embark"]
                            .as_array()
                            .is_some_and(|ids| ids.iter().any(|id| id == &quest.id))
                        {
                            return Err(
                                "mercenary is eligible only for its source-listed farm quest"
                                    .into(),
                            );
                        }
                        continue;
                    }
                    if raw["required_plot_quests_to_embark"]
                        .as_array()
                        .is_some_and(|v| {
                            v.iter().any(|id| {
                                id.as_str()
                                    .is_none_or(|id| !s.state.completed_ids.contains(id))
                            })
                        })
                    {
                        return Err("hero quirk blocks this campaign departure".into());
                    }
                }
                let mut bp = s.blueprint(h, uint(&quest.raw, "difficulty")?, true)?;
                if s.state.campaign.is_some() {
                    for t in &bp.mechanics.build.trinkets {
                        s.definition.game.quest_item_effects(&t.definition)?;
                    }
                } else {
                    for t in &bp.mechanics.build.trinkets {
                        if t.definition
                            .quest_metadata
                            .get("on_quest_complete_additional_effects")
                            .is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
                        {
                            return Err(
                                "quest-completion item effects require their expedition owner"
                                    .into(),
                            );
                        }
                    }
                    for t in &mut bp.mechanics.build.trinkets {
                        t.quest_remaining = t.definition.quest_uses;
                    }
                    for t in bp.mechanics.build.input.trinket_states.values_mut() {
                        t.quest_remaining = t.definition.quest_uses;
                    }
                }
                heroes.push(bp);
            }
            let bonuses = s.bonuses(&party, &mut heroes)?;
            if !bonuses.domains().is_subset(&acknowledged) {
                return Err(format!(
                    "expedition host has not accepted required district domains: {:?}",
                    bonuses
                        .domains()
                        .difference(&acknowledged)
                        .collect::<Vec<_>>()
                ));
            }
            let token = format!("{};departure:{}", s.state.estate_id, s.state.next_departure);
            s.state.next_departure = s
                .state
                .next_departure
                .checked_add(1)
                .ok_or("departure identity overflow")?;
            let assets = s
                .state
                .assets
                .iter()
                .filter(|a| a.owner.is_some_and(|(id, _)| party.contains(&id)))
                .map(|a| a.instance)
                .collect();
            s.state.departure = Some(Departure {
                token: token.clone(),
                party: party.clone(),
                quest: quest.clone(),
                assets,
                opening_gold: s.state.wallet.get("gold:").copied().unwrap_or(0),
                required: bonuses.clone(),
            });
            Ok(Deployment {
                token,
                party,
                heroes,
                quest,
                bonuses,
            })
        })
    }
    pub fn expedition_rules(&self, quest: &HostQuest, torch: u8) -> Result<Rules> {
        quest.validate(&self.definition.data.content.id()?)?;
        let mut r = self.definition.game.rules.clone();
        r.context = Some(
            self.definition.game.battle_context(
                quest.raw["dungeon"]
                    .as_str()
                    .ok_or("missing region")?
                    .into(),
                dd_data::build010::Surprise::Neither,
                false,
            )?,
        );
        r.lighting = Some(self.definition.game.lighting(torch)?);
        Ok(r)
    }
    /// The expedition owner verifies objectives and actual picked loot. Returns must
    /// originate from a settled native battle and match the live departure identities.
    pub fn finish_sortie(
        &mut self,
        token: &str,
        result: crate::BattleResult,
        picked: Vec<Stack>,
        completed: bool,
        recovered: BTreeSet<u64>,
    ) -> Result<()> {
        self.transaction(|s| s.finish_inner(token, result, picked, completed, recovered))
    }
    fn finish_inner(
        &mut self,
        token: &str,
        result: crate::BattleResult,
        picked: Vec<Stack>,
        completed: bool,
        recovered: BTreeSet<u64>,
    ) -> Result<()> {
        let departure = self.state.departure.clone().ok_or("no active departure")?;
        if result.host_return_token.as_deref() != Some(token)
            || token != departure.token
            || self.state.receipts.contains(token)
            || result.profile != crate::PROFILE
            || result.content_id != self.definition.data.content.id()?
            || result.candidate_id != self.definition.game.rules.candidates.identity()?
            || result.heroes.len() != departure.party.len()
            || result.region.as_deref() != departure.quest.raw["dungeon"].as_str()
        {
            return Err("stale/foreign native mission result".into());
        }
        if !matches!(
            result.outcome,
            crate::Outcome::HeroesWin
                | crate::Outcome::MonstersWin
                | crate::Outcome::MutualDefeat
                | crate::Outcome::Retreated
        ) {
            return Err("unsettled/special expedition outcome needs its campaign owner".into());
        }
        if result.ends_infestation_sequence
            || result.flashback_id.is_some()
            || result.farm_interlude.is_some()
        {
            return Err("special campaign return requires its story owner".into());
        }
        if !result.trinket_inventory.is_empty() {
            return Err("carried trinket inventory needs physical expedition asset mapping".into());
        }
        self.settle_sortie(token, result.heroes, picked, completed, recovered, None)
    }
    pub(super) fn settle_sortie(
        &mut self,
        token: &str,
        heroes: Vec<HeroResult>,
        mut picked: Vec<Stack>,
        completed: bool,
        recovered: BTreeSet<u64>,
        campaign: Option<CampaignSettlement<'_>>,
    ) -> Result<()> {
        let departure = self.state.departure.clone().ok_or("no active departure")?;
        if token != departure.token
            || self.state.receipts.contains(token)
            || heroes.len() != departure.party.len()
        {
            return Err("stale/foreign owner return".into());
        }
        if recovered.iter().any(|id| !departure.assets.contains(id)) {
            return Err("unknown recovered physical trinket".into());
        }
        let mut seen = BTreeSet::new();
        let mut returned = BTreeMap::new();
        for h in heroes {
            if h.id >= departure.party.len()
                || !seen.insert(h.id)
                || h.class != self.hero(departure.party[h.id])?.class
                || h.content_id != self.definition.data.content.id()?
                || h.candidate_id != self.definition.game.rules.candidates.identity()?
            {
                return Err("returned hero identity/content mismatch".into());
            }
            returned.insert(departure.party[h.id], h);
        }
        if completed && !returned.values().any(|h| h.alive) {
            return Err("dead party cannot complete a mission".into());
        }
        // Match equipment transformations to the original physical items; picked loot
        // and recovered dead equipment use separate receipts and never clone an asset.
        let mut keep = BTreeSet::new();
        let mut new_assets = vec![];
        let mut recovered_seen = BTreeSet::new();
        for (guid, h) in &returned {
            let mut owned = self
                .state
                .assets
                .iter()
                .filter(|a| {
                    departure.assets.contains(&a.instance)
                        && a.owner.is_some_and(|(id, _)| id == *guid)
                })
                .cloned()
                .collect::<Vec<_>>();
            owned.sort_by_key(|a| a.owner.unwrap().1);
            let slots = h
                .trinket_slots
                .as_ref()
                .ok_or("missing native physical slot receipt")?;
            if slots.len() != h.trinkets.len()
                || slots.iter().collect::<BTreeSet<_>>().len() != slots.len()
            {
                return Err("invalid native physical slot receipt".into());
            }
            for (state, native_slot) in h.trinkets.iter().zip(slots) {
                state.validate()?;
                if state.destroyed {
                    continue;
                }
                if self.definition.trinket(&state.definition.id)? != state.definition {
                    return Err("returned trinket source differs".into());
                }
                let a = owned
                    .get(*native_slot)
                    .ok_or("unowned returned equipment slot")?
                    .clone();
                let graph = self
                    .definition
                    .game
                    .trinket_catalog(std::slice::from_ref(&a.state.definition.id))?;
                if !graph.contains_key(&state.definition.id) {
                    return Err("returned physical slot changed to unrelated equipment".into());
                }
                if h.alive || recovered.contains(&a.instance) {
                    if !h.alive {
                        recovered_seen.insert(a.instance);
                    }
                    keep.insert(a.instance);
                    new_assets.push(Asset {
                        instance: a.instance,
                        state: state.clone(),
                        owner: if h.alive { a.owner } else { None },
                    });
                }
            }
        }
        if recovered_seen != recovered {
            return Err("recovery includes surviving/destroyed/unreturned equipment".into());
        }
        self.state
            .assets
            .retain(|a| !departure.assets.contains(&a.instance));
        if completed {
            for asset in &mut new_assets {
                if asset.owner.is_none() {
                    continue;
                }
                let metadata = &asset.state.definition.quest_metadata;
                if let Some(next) = metadata["transform_on_quest_complete"].as_str() {
                    let transform = match metadata.get("transform_on_quest_complete_chance") {
                        None => true,
                        Some(v) => {
                            let chance = v
                                .as_f64()
                                .filter(|n| (0. ..=1.).contains(n))
                                .ok_or("invalid quest asset transform chance")?;
                            self.state.rng.roll(chance)
                        }
                    };
                    if transform {
                        let generation = asset
                            .state
                            .generation
                            .checked_add(1)
                            .ok_or("asset generation overflow")?;
                        let mut state = TrinketState::fresh(self.definition.trinket(next)?);
                        state.generation = generation;
                        self.state.log.push(json!({"quest_asset":asset.instance,"from":asset.state.definition.id,"to":next}));
                        asset.state = state;
                    }
                }
            }
        }
        self.state.assets.extend(new_assets);
        self.state.previous_deaths.clear();
        let base_xp = if completed {
            uint(&departure.quest.raw["completion_reward"], "resolve_xp")?
        } else {
            0
        };
        for (guid, mut returned_state) in returned {
            if campaign.is_some() {
                self.campaign_death_quirks(&mut returned_state)?;
            }
            let i = self.hero_index(guid)?;
            let mut flags = self.state.heroes[i].quirks.clone();
            flags.retain(|id, _| returned_state.build.quirks.contains(id));
            for id in &returned_state.build.quirks {
                flags.entry(id.clone()).or_default();
            }
            self.state.heroes[i].quirks = flags;
            let alive = returned_state.alive;
            self.state.heroes[i].state = returned_state;
            self.state.heroes[i].state.status = Default::default();
            if alive {
                self.state.heroes[i].state.hp = self.state.heroes[i].state.max_hp;
            }
            self.state.heroes[i].state.persistent_buffs.clear();
            if alive {
                let bonus = departure
                    .required
                    .field_buffs
                    .get(&guid)
                    .into_iter()
                    .flatten()
                    .filter(|b| b.stat_type == "resolve_xp_bonus_percent")
                    .filter(|b| {
                        campaign.is_none()
                            || b.rule_type != "in_dungeon"
                            || departure.quest.raw["dungeon"] == b.rule_data.string
                    })
                    .map(|b| b.amount)
                    .sum::<f64>()
                    + self.experience_modifier(guid)?;
                let xp = (f64::from(base_xp) * (1. + bonus).max(0.)).round() as u32;
                self.state.heroes[i].xp = self.state.heroes[i]
                    .xp
                    .checked_add(xp)
                    .ok_or("experience overflow")?;
                if (campaign.is_none() || completed)
                    && !(self
                        .state
                        .campaign
                        .as_ref()
                        .is_some_and(|c| c.owner.starts_with(super::campaign::content019::PROFILE))
                        && self.state.heroes[i].missing_until > self.effective_week())
                {
                    self.mature_quirks(guid)?;
                }
            } else {
                self.state
                    .previous_deaths
                    .push(self.level(self.state.heroes[i].xp)?);
            }
        }
        let all_dead = departure
            .party
            .iter()
            .all(|id| self.hero(*id).is_ok_and(|h| !h.state.alive));
        if all_dead {
            picked.clear();
        }
        if !completed && !all_dead && !campaign.is_some_and(|c| c.regroup) {
            let mut totals = BTreeMap::<Item, u32>::new();
            for stack in picked {
                let n = totals.entry(stack.item).or_default();
                *n = n.checked_add(stack.amount).ok_or("return loot overflow")?;
            }
            picked = vec![];
            for (item, n) in totals {
                let key = if self
                    .definition
                    .data
                    .base
                    .return_rates
                    .contains_key(&item.id)
                    && item.kind == "estate_currency"
                {
                    &item.id
                } else {
                    &item.kind
                };
                let raw = library::unique_json(
                    &self.definition.data.base.return_rates,
                    key,
                    "failure retention",
                )?;
                let rate = raw["rate"]
                    .as_f64()
                    .filter(|n| (0. ..=1.).contains(n))
                    .ok_or("invalid failure retention")?;
                let amount = (f64::from(n) * rate).floor() as u32;
                if amount > 0 {
                    picked.push(Stack { item, amount });
                }
            }
        }
        for stack in &picked {
            if stack.amount == 0 || stack.amount > 1000000 {
                return Err("invalid returned inventory quantity".into());
            }
            self.bank_item(stack)?;
        }
        if !all_dead && let Some(context) = campaign {
            let retained = if completed || context.regroup {
                true
            } else {
                let rate = library::unique_json(
                    &self.definition.data.base.return_rates,
                    "trinket",
                    "physical return retention",
                )?["rate"]
                    .as_f64()
                    .filter(|n| (0. ..=1.).contains(n))
                    .ok_or("invalid physical return retention")?;
                rate.floor() == 1.
            };
            if retained {
                for state in context.physical {
                    self.definition.game.validate_item_growth(state)?;
                    if state.destroyed
                        || self.definition.trinket(&state.definition.id)? != state.definition
                    {
                        return Err("invalid physical picked asset source".into());
                    }
                    let instance = self.add_asset(&state.definition.id, None)?;
                    self.state
                        .assets
                        .iter_mut()
                        .find(|a| a.instance == instance)
                        .unwrap()
                        .state = state.clone();
                }
            }
        }
        if completed {
            self.state.completed = self
                .state
                .completed
                .checked_add(1)
                .ok_or("quest count overflow")?;
            self.state.completed_ids.insert(departure.quest.id.clone());
            for row in departure.quest.raw["completion_reward"]["items_definition"]["items"]
                .as_object()
                .ok_or("missing source quest rewards")?
                .values()
            {
                let amount = uint(row, "amount")?;
                if amount > 0 {
                    self.bank_item(&Stack {
                        item: Item::new(
                            row["type"].as_str().ok_or("missing reward kind")?,
                            row["id"].as_str().ok_or("missing reward id")?,
                        ),
                        amount,
                    })?;
                }
            }
            if departure.quest.advances_region {
                let award = self.region_award(uint(&departure.quest.raw, "length")?)?;
                let v = self
                    .state
                    .dungeon_xp
                    .entry(departure.quest.raw["dungeon"].as_str().unwrap().into())
                    .or_default();
                *v = v.checked_add(award).ok_or("region progress overflow")?;
                self.update_highest()?;
            }
        }
        if let Some(context) = campaign {
            self.campaign_plot_quirks(&departure.party, context.quirks)?;
            self.campaign_raid_quirks(&departure.party, context.raid_quirks)?;
        }
        if completed && campaign.is_some() {
            self.campaign_quest_items(&departure.party)?;
        }
        self.state.visit = self
            .state
            .visit
            .checked_add(1)
            .ok_or("town visit overflow")?;
        self.state.departure = None;
        self.state.party.retain(|id| {
            self.state
                .heroes
                .iter()
                .any(|h| h.guid == *id && h.state.alive)
        });
        if campaign.is_none_or(|c| c.advance) {
            let occupied = self
                .state
                .jobs
                .iter()
                .map(|j| j.hero)
                .collect::<BTreeSet<_>>();
            let idle = self.campaign_idle_relief(self.idle_relief()?)?;
            for i in 0..self.state.heroes.len() {
                let guid = self.state.heroes[i].guid;
                if self.state.heroes[i].state.alive
                    && !departure.party.contains(&guid)
                    && !occupied.contains(&guid)
                    && self.state.heroes[i].missing_until <= self.effective_week()
                {
                    let extra = self
                        .modifier(
                            &self.state.heroes[i],
                            "stress_from_idle_in_town",
                            "STRESS_AMOUNT",
                            None,
                        )?
                        .round() as i32;
                    self.town_stress(guid, extra - idle)?;
                }
            }
            self.campaign_idle_events(&departure.party, &occupied)?;
            self.campaign_contagion()?;
            self.settle_jobs()?;
            self.disease_week()?;
            self.district_week(departure.opening_gold)?;
            self.refresh_markets()?;
        }
        self.state.receipts.insert(token.into());
        self.state.log.push(json!({"mission":departure.quest.id,"completed":completed,"party_dead":all_dead,"visit":self.state.visit,"xp":base_xp}));
        if self.state.log.len() > 1000 {
            self.state.log.drain(..self.state.log.len() - 1000);
        }
        let _ = keep;
        Ok(())
    }
    pub(super) fn bank_item(&mut self, stack: &Stack) -> Result<()> {
        if stack.amount == 0 || stack.amount > 1000000 {
            return Err("invalid estate incoming amount".into());
        }
        if stack.item.kind == "trinket_unlock"
            && self
                .state
                .campaign
                .as_ref()
                .is_some_and(|c| c.automation.is_some())
        {
            if stack.item.id != "dd_trinket" {
                return Err("unknown actual trinket unlock".into());
            }
            for _ in 0..stack.amount {
                self.add_asset(&stack.item.id, None)?;
            }
            return Ok(());
        }
        self.definition
            .data
            .base
            .bag_rules
            .definition(&stack.item)?;
        if stack.item.kind == "trinket" {
            for _ in 0..stack.amount {
                self.add_asset(&stack.item.id, None)?;
            }
        } else if ["supply", "provision", "gem"].contains(&stack.item.kind.as_str()) {
            let value = self
                .definition
                .data
                .base
                .bag_rules
                .definition(&stack.item)?
                .sell
                .checked_mul(stack.amount)
                .ok_or("return sale overflow")?;
            self.credit(&BTreeMap::from([(self.currency("gold")?, value)]))?;
        } else {
            let mut factor = 1.;
            if let Some(context) = &self.state.campaign {
                if stack.item.kind == "heirloom" {
                    for effect in context
                        .effects
                        .iter()
                        .filter(|e| e.kind == "heirloom_gain_multiplier")
                    {
                        factor *= effect.amount;
                    }
                } else if stack.item.kind == "shard" {
                    factor += context.sum("shard_buff", "");
                }
            }
            let amount = (f64::from(stack.amount) * factor).floor();
            if !amount.is_finite() || !(0. ..=f64::from(u32::MAX)).contains(&amount) {
                return Err("event resource gain overflow/domain".into());
            }
            self.credit(&BTreeMap::from([(stack.item.key(), amount as u32)]))?;
        }
        Ok(())
    }
    pub(super) fn experience_modifier(&self, guid: u64) -> Result<f64> {
        let h = self.hero(guid)?;
        let mut k = PartyKernel::new(
            vec![self.blueprint(h, 1, false)?],
            self.definition.game.rules.clone(),
            Rng(1),
        )?;
        k.exploration_buffs(&self.definition.game.buffs)?;
        Ok(k.modifier(0, "resolve_xp_bonus_percent", "") + self.xp_bonus018(guid))
    }
    pub(super) fn town_stress(&mut self, guid: u64, change: i32) -> Result<()> {
        let i = self.hero_index(guid)?;
        let h = &self.state.heroes[i];
        if !h.state.alive {
            return Ok(());
        }
        let mut k = PartyKernel::new(
            vec![self.blueprint(h, 1, false)?],
            self.definition.game.rules.clone(),
            self.state.rng.clone(),
        )?;
        k.abandon_stress(0, change, false)?;
        let (mut states, rng) = k.export();
        self.state.rng = rng;
        self.state.heroes[i].state = states.remove(0);
        if !self.state.heroes[i].state.alive {
            self.state.party.retain(|id| *id != guid);
            self.state.jobs.retain(|j| j.hero != guid);
            self.state
                .assets
                .retain(|a| a.owner.is_none_or(|(id, _)| id != guid));
        }
        Ok(())
    }
    fn mature_quirks(&mut self, guid: u64) -> Result<()> {
        let i = self.hero_index(guid)?;
        let limit = uint(
            &self.definition.data.base.globals,
            "quirks_max_locked_negative",
        )? as usize;
        let threshold = uint(
            &self.definition.data.base.globals,
            "quirk_negative_locked_after_turn_count",
        )?;
        let chance = self.definition.data.base.globals["quirk_chance_to_lock_negative"]
            .as_f64()
            .ok_or("missing negative maturation chance")?;
        let mut locked = self.state.heroes[i]
            .quirks
            .iter()
            .filter(|(id, f)| {
                f.locked
                    && library::unique_json(&self.definition.data.base.quirk_defs, id, "quirk")
                        .is_ok_and(|q| q["is_positive"] == false)
            })
            .count();
        let ids = self.state.heroes[i]
            .quirks
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for id in ids {
            let q = library::unique_json(
                &self.definition.data.base.quirk_defs,
                &id,
                "quirk maturation",
            )?;
            let f = self.state.heroes[i].quirks.get_mut(&id).unwrap();
            f.mission_count = f
                .mission_count
                .checked_add(1)
                .ok_or("quirk history overflow")?;
            if q["is_positive"] == false
                && q["can_be_replaced_by_new_quirk"] == true
                && !f.locked
                && f.mission_count >= threshold
                && locked < limit
                && self.state.rng.roll(chance)
            {
                f.locked = true;
                locked += 1;
            }
        }
        Ok(())
    }
    pub(super) fn settle_jobs(&mut self) -> Result<()> {
        let jobs = std::mem::take(&mut self.state.jobs);
        for j in jobs {
            if !self.hero(j.hero)?.state.alive {
                continue;
            }
            if let Some(until) = j.locked_until {
                if self.effective_week() < until {
                    self.state.jobs.push(j);
                }
                continue;
            }
            let a = self.definition.data.activity(&j.facility)?.clone();
            match &j.treatment {
                Treatment::Stress => {
                    let row = self.tier(&a["stress_upgrades"], &j.facility)?;
                    let source = [uint(row, "heal_low")?, uint(row, "heal_high")?];
                    let [lo, hi] = self
                        .class_activity018(&self.hero(j.hero)?.class)
                        .and_then(|r| r.stress)
                        .unwrap_or(source);
                    if lo > hi {
                        return Err("invalid stress treatment range".into());
                    }
                    let base = lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32;
                    let activity = j.facility.split_once('.').unwrap().1;
                    let received = self.modifier(
                        self.hero(j.hero)?,
                        "stress_heal_received_percent",
                        "",
                        Some(activity),
                    )?;
                    self.town_stress(
                        j.hero,
                        -(f64::from(base) * (1. + received).max(0.)).round() as i32,
                    )?;
                    let chance = self.tier(&a["affliction_cure_upgrades"], &j.facility)?["chance"]
                        .as_f64()
                        .ok_or("invalid affliction cure chance")?;
                    if self.state.rng.roll(chance) {
                        let i = self.hero_index(j.hero)?;
                        self.state.heroes[i].state.resolve = Resolve::Calm;
                    }
                    if let Some(effect) = self.side_effect(&j, &a)? {
                        self.state.log.push(
                            json!({"hero":j.hero,"facility":j.facility,"side_effect":effect}),
                        );
                    }
                }
                Treatment::Combined {
                    negative,
                    positive,
                    lock_positive,
                } => {
                    let chance = self.tier(&a["quirk_treatment_upgrades"], &j.facility)?["chance"]
                        .as_f64()
                        .ok_or("missing quirk cure chance")?;
                    if self.state.rng.roll(chance) {
                        self.apply_treatment(j.hero, &Treatment::RemoveQuirk(negative.clone()))?;
                        self.apply_treatment(
                            j.hero,
                            &if *lock_positive {
                                Treatment::LockQuirk(positive.clone())
                            } else {
                                Treatment::RemoveQuirk(positive.clone())
                            },
                        )?;
                    }
                }
                other => {
                    let chance = self.tier(&a["quirk_treatment_upgrades"], &j.facility)?["chance"]
                        .as_f64()
                        .ok_or("missing treatment chance")?;
                    if self.state.rng.roll(chance) {
                        self.apply_treatment(j.hero, other)?;
                        if matches!(other, Treatment::Disease(_)) {
                            let all =
                                self.tier(
                                    &a["disease_quirk_cure_all_chance_upgrades"],
                                    &j.facility,
                                )?["chance"]
                                    .as_f64()
                                    .ok_or("missing multi disease chance")?;
                            if self.state.rng.roll(all) {
                                let i = self.hero_index(j.hero)?;
                                let diseases = self.state.heroes[i].state.diseases.clone();
                                for id in diseases {
                                    let raw = library::unique_json(
                                        &self.definition.data.base.quirk_defs,
                                        &id,
                                        "disease cure",
                                    )?;
                                    if raw["can_modify_in_activity"] == true
                                        && raw["required_plot_quests_to_modify"]
                                            .as_array()
                                            .is_none_or(|a| {
                                                a.iter().all(|id| {
                                                    id.as_str().is_some_and(|id| {
                                                        self.state.completed_ids.contains(id)
                                                    })
                                                })
                                            })
                                    {
                                        self.apply_treatment(j.hero, &Treatment::Disease(id))?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            self.state
                .log
                .push(json!({"hero":j.hero,"facility":j.facility,"treatment_complete":true}));
        }
        Ok(())
    }
    fn apply_treatment(&mut self, guid: u64, t: &Treatment) -> Result<()> {
        let i = self.hero_index(guid)?;
        match t {
            Treatment::RemoveQuirk(id) => {
                self.state.heroes[i].quirks.remove(id);
            }
            Treatment::LockQuirk(id) => {
                self.state.heroes[i]
                    .quirks
                    .get_mut(id)
                    .ok_or("treated quirk disappeared")?
                    .locked = true;
            }
            Treatment::Disease(id) => {
                self.state.heroes[i].state.diseases.retain(|d| d != id);
                self.state.heroes[i].state.disease_turns.remove(id);
            }
            _ => return Err("invalid direct treatment operation".into()),
        }
        Ok(())
    }
    pub(super) fn add_quirk(&mut self, guid: u64, id: &str) -> Result<()> {
        let i = self.hero_index(guid)?;
        if self.state.heroes[i].quirks.contains_key(id) {
            return Ok(());
        }
        let q = library::unique_json(&self.definition.data.base.quirk_defs, id, "town quirk gain")?
            .clone();
        if q["is_disease"] == true {
            let h = self.hero(guid)?.clone();
            if h.state.diseases.iter().any(|d| d == id) {
                return Ok(());
            }
            let mut rules = self.definition.game.rules.clone();
            rules.context = Some(self.definition.game.battle_context(
                "town".into(),
                dd_data::build010::Surprise::Neither,
                false,
            )?);
            let disease = rules
                .context
                .as_ref()
                .unwrap()
                .diseases
                .get(id)
                .ok_or("missing town disease definition")?
                .clone();
            let mut kernel = PartyKernel::new(
                vec![self.blueprint(&h, 1, false)?],
                rules,
                self.state.rng.clone(),
            )?;
            if !disease
                .incompatible
                .iter()
                .any(|id| h.state.diseases.contains(id))
            {
                kernel.core.contract_disease(0, &disease, &mut vec![]);
            }
            kernel.settle()?;
            let (mut states, rng) = kernel.export();
            self.state.rng = rng;
            self.state.heroes[i].state = states.remove(0);
            return Ok(());
        }
        if q["is_disease"] != false {
            return Err("missing town quirk category".into());
        }
        let positive = q["is_positive"].as_bool().ok_or("missing quirk sign")?;
        let singleton = q["tags"]
            .as_array()
            .is_some_and(|tags| tags.iter().any(|v| v == "singleton"));
        if self.state.campaign.is_some()
            && singleton
            && self
                .state
                .heroes
                .iter()
                .any(|h| h.state.alive && h.quirks.contains_key(id))
        {
            return Ok(());
        }

        let conflict = self.state.heroes[i]
            .quirks
            .keys()
            .filter(|old| {
                q["incompatible_quirks"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|x| x == *old))
                    || library::unique_json(&self.definition.data.base.quirk_defs, old, "quirk")
                        .is_ok_and(|o| {
                            o["incompatible_quirks"]
                                .as_array()
                                .is_some_and(|a| a.iter().any(|v| v == id))
                        })
            })
            .cloned()
            .collect::<Vec<_>>();
        if conflict
            .iter()
            .any(|id| self.state.heroes[i].quirks[id].locked)
        {
            return Ok(());
        }
        for id in conflict {
            self.state.heroes[i].quirks.remove(&id);
        }
        let same = self.state.heroes[i]
            .quirks
            .iter()
            .filter(|(id, _)| {
                library::unique_json(&self.definition.data.base.quirk_defs, id, "quirk")
                    .is_ok_and(|q| q["is_positive"].as_bool() == Some(positive))
            })
            .map(|(id, f)| (id.clone(), f.locked))
            .collect::<Vec<_>>();
        let cap = if positive {
            self.definition.game.rules.quirks_max_positive
        } else {
            self.definition.game.rules.quirks_max_negative
        } as usize;
        if same.len() >= cap {
            let replace = same
                .iter()
                .filter(|(id, locked)| {
                    !*locked
                        && (self.state.campaign.is_none()
                            || self
                                .definition
                                .data
                                .base
                                .quirk_defs
                                .get(id)
                                .is_some_and(|q| q["can_be_replaced_by_new_quirk"] == true))
                })
                .collect::<Vec<_>>();
            if replace.is_empty() {
                return Ok(());
            }
            let index = self.state.rng.below(replace.len() as u64) as usize;
            self.state.heroes[i].quirks.remove(&replace[index].0);
        }
        let locked = self.state.campaign.is_some()
            && q["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|v| v == "cant_lock"));
        self.state.heroes[i].quirks.insert(
            id.into(),
            QuirkFlags {
                locked,
                mission_count: 0,
            },
        );
        Ok(())
    }
    pub(super) fn side_effect(&mut self, j: &Job, activity: &Value) -> Result<Option<String>> {
        let side = &activity["side_effects"];
        let chance = side["chance"]
            .as_f64()
            .filter(|n| (0. ..=1.).contains(n))
            .ok_or("invalid activity side effect chance")?;
        if !self.state.rng.roll(chance) {
            return Ok(None);
        }
        let rows = side["results"]
            .as_array()
            .ok_or("missing side effect results")?;
        let activity_id = j.facility.split_once('.').unwrap().1;
        let forbidden = if self
            .state
            .campaign
            .as_ref()
            .is_some_and(|c| c.automation.is_some())
        {
            self.blueprint(self.hero(j.hero)?, 1, false)?
                .mechanics
                .hero
                .incompatible_quirks
        } else {
            vec![]
        };

        let weights = rows
            .iter()
            .map(|r| {
                let kind = r["type"].as_str().ok_or("missing side effect type")?;
                if kind == "add_quirk"
                    && !forbidden.is_empty()
                    && r["data"]
                        .as_array()
                        .ok_or("missing side effect data")?
                        .iter()
                        .all(|v| {
                            v["quirk_library_name"]
                                .as_str()
                                .is_some_and(|id| forbidden.iter().any(|s| s == id))
                        })
                {
                    return Ok(0.);
                }
                let weight = r["chance"].as_f64().ok_or("invalid side effect weight")?;
                Ok((weight
                    + self.modifier(
                        self.hero(j.hero)?,
                        "activity_side_effect_chance",
                        kind,
                        Some(activity_id),
                    )?)
                .max(0.))
            })
            .collect::<Result<Vec<_>>>()?;
        if !forbidden.is_empty() && weights.iter().all(|w| *w == 0.) {
            return Ok(None);
        }
        let selected = &rows[self.choose(&weights)?];
        let kind = selected["type"]
            .as_str()
            .ok_or("missing side effect type")?
            .to_string();
        let data = selected["data"]
            .as_array()
            .ok_or("missing side effect data")?;
        if kind == "activity_lock" {
            if !data.is_empty() {
                return Err("unsupported activity lock payload".into());
            }
            let mut locked = j.clone();
            locked.locked_until = Some(self.effective_week() + 1);
            locked.paid.clear();
            self.state.jobs.push(locked);
            return Ok(Some(kind));
        }
        let weights = data
            .iter()
            .map(|r| {
                if kind == "add_quirk"
                    && r["quirk_library_name"]
                        .as_str()
                        .is_some_and(|id| forbidden.iter().any(|s| s == id))
                {
                    return Ok(0.);
                }
                r["chance"]
                    .as_f64()
                    .ok_or_else(|| "missing side effect data weight".into())
            })
            .collect::<Result<Vec<_>>>()?;
        let row = data
            .get(self.choose(&weights)?)
            .ok_or("empty positive side effect")?
            .clone();
        match kind.as_str() {
            "go_missing" => {
                let duration = uint(&row, "duration")?;
                let i = self.hero_index(j.hero)?;
                self.state.heroes[i].missing_until = self
                    .effective_week()
                    .checked_add(duration)
                    .and_then(|n| n.checked_add(1))
                    .ok_or("absence duration overflow")?;
            }
            "add_quirk" => self.add_quirk(
                j.hero,
                row["quirk_library_name"]
                    .as_str()
                    .ok_or("missing granted quirk")?,
            )?,
            "apply_buff" => {
                let i = self.hero_index(j.hero)?;
                for id in row["buff_library_ids"]
                    .as_array()
                    .ok_or("missing town Buff IDs")?
                {
                    let id = id.as_str().ok_or("invalid town Buff ID")?;
                    let b = dd_data::mechanics::read_buff(&self.definition.game.buffs, id)?;
                    self.state.heroes[i]
                        .state
                        .persistent_buffs
                        .retain(|old| old.definition.id != id);
                    self.state.heroes[i]
                        .state
                        .persistent_buffs
                        .push(dd_data::hero010::CarryBuff {
                            remaining: b.duration,
                            definition: b,
                        });
                }
            }
            "remove_currency" | "add_currency" => {
                let currency =
                    self.currency(row["type"].as_str().ok_or("missing side effect currency")?)?;
                let amount = uint(&row, "amount")?;
                if kind == "add_currency" {
                    self.credit(&BTreeMap::from([(currency, amount)]))?;
                } else {
                    let balance = self.state.wallet.entry(currency).or_default();
                    *balance = balance.saturating_sub(amount);
                }
            }
            "add_trinket" => {
                let id = self.draw_trinket(
                    row["rarity"].as_str().ok_or("missing side effect rarity")?,
                    &[],
                )?;
                self.add_asset(&id, None)?;
            }
            "remove_trinket" => {
                let rarity = row["rarity"].as_str().ok_or("missing loss rarity")?;
                let candidates = self
                    .state
                    .assets
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| {
                        a.owner.is_none()
                            && library::unique_json(
                                &self.definition.data.base.curios.trinkets,
                                &a.state.definition.id,
                                "loss trinket",
                            )
                            .is_ok_and(|v| v["rarity"] == rarity)
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if !candidates.is_empty() {
                    let i = candidates[self.state.rng.below(candidates.len() as u64) as usize];
                    let lost = self.state.assets.remove(i);
                    if let Some(context) = &mut self.state.campaign {
                        context.weekly_lost_assets.push(lost);
                    }
                }
            }
            _ => return Err(format!("unsupported positive activity side effect {kind}")),
        }
        Ok(Some(kind))
    }
    pub(super) fn disease_week(&mut self) -> Result<()> {
        let ids = self
            .state
            .heroes
            .iter()
            .filter(|h| h.state.alive)
            .map(|h| h.guid)
            .collect::<Vec<_>>();
        for guid in ids {
            let i = self.hero_index(guid)?;
            let h = self.state.heroes[i].clone();
            if h.state.disease_turns.is_empty() {
                continue;
            }
            let mut rules = self.definition.game.rules.clone();
            rules.context = Some(self.definition.game.battle_context(
                "town".into(),
                dd_data::build010::Surprise::Neither,
                false,
            )?);
            let mut k = PartyKernel::new(
                vec![self.blueprint(&h, 1, false)?],
                rules,
                self.state.rng.clone(),
            )?;
            let sources = h.state.disease_turns.keys().cloned().collect::<Vec<_>>();
            let mut fed = false;
            for source in &sources {
                let raw = library::unique_json(
                    &self.definition.data.base.quirk_defs,
                    source,
                    "town disease evolution",
                )?;
                if let Some(threshold) =
                    raw["evolution_town_attempt_use_item_duration_threshold"].as_u64()
                    && u64::from(h.state.disease_turns[source]) <= threshold
                    && self
                        .state
                        .wallet
                        .get("estate:the_blood")
                        .copied()
                        .unwrap_or(0)
                        > 0
                {
                    let supply = self.definition.game.supply("the_blood")?;
                    k.core.supplies = vec![dd_data::items::SupplyStack {
                        definition: supply,
                        remaining: 1,
                    }];
                    k.core.consume_supply(0, 0, &mut vec![])?;
                    k.settle()?;
                    *self.state.wallet.get_mut("estate:the_blood").unwrap() -= 1;
                    fed = true;
                    break;
                }
            }
            if !fed {
                for source in &sources {
                    let raw = library::unique_json(
                        &self.definition.data.base.quirk_defs,
                        source,
                        "town disease evolution",
                    )?;
                    let amount = uint(raw, "evolution_town_progression_duration_change")?;
                    if amount > 0
                        && let Some(n) = k.core.units[0].disease_turns.get_mut(source)
                    {
                        *n = n.saturating_sub(amount - 1);
                    }
                }
                k.core.tick_diseases(0, &mut vec![]);
                k.settle()?;
            }
            let (mut results, rng) = k.export();
            self.state.rng = rng;
            self.state.heroes[i].state = results.remove(0);
            if !self.state.heroes[i].state.alive {
                self.state.party.retain(|id| *id != guid);
                self.state.jobs.retain(|j| j.hero != guid);
                self.state
                    .assets
                    .retain(|a| a.owner.is_none_or(|(id, _)| id != guid));
            }
        }
        Ok(())
    }
}
