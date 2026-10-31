//! Visit-bound event service effects; only a campaign definition may restore them.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Effect {
    pub kind: String,
    pub id: String,
    pub amount: f64,
}
impl Effect {
    pub fn parse(v: &Value) -> Result<Self> {
        let e = Self {
            kind: v["type"]
                .as_str()
                .ok_or("missing event effect type")?
                .into(),
            id: v["string_data"]
                .as_str()
                .ok_or("missing event effect string")?
                .into(),
            amount: v["number_data"]
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or("invalid event effect amount")?,
        };
        if !dd_data::campaign017::EFFECTS.contains(&e.kind.as_str()) {
            return Err(format!("unsupported event effect {}", e.kind));
        }
        let known = match e.kind.as_str() {
            "activity_lock" | "activity_cost_change" => [
                "bar",
                "gambling",
                "brothel",
                "meditation",
                "prayer",
                "flagellation",
            ]
            .contains(&e.id.as_str()),
            "free_activity" => [
                "bar",
                "gambling",
                "brothel",
                "meditation",
                "prayer",
                "flagellation",
                "treatment",
                "disease_treatment",
            ]
            .contains(&e.id.as_str()),
            "provision_item_type_cost_change" | "provision_item_type_amount_change" => {
                ["supply", "provision"].contains(&e.id.as_str())
            }
            "upgrade_tag_free" => {
                ["armour", "weapon", "building", "camping_skill"].contains(&e.id.as_str())
            }
            "upgrade_tag_discount" => [
                "trinket",
                "armour",
                "weapon",
                "building",
                "camping_skill",
                "combat_skill",
            ]
            .contains(&e.id.as_str()),
            "dead_recruit"
            | "remove_quest_hero_level_restriction"
            | "trinket_retention_add_from_storage"
            | "stage_coach_bonus_recruits"
            | "remove_vampire"
            | "stress_treatment_debuff"
            | "districts_unlocked"
            | "shard_buff"
            | "roster_disease_heal"
            | "roster_stress_heal"
            | "trinket_sell_multiplier"
            | "idle_negative_quirk_remove"
            | "heirloom_gain_multiplier"
            | "nomad_wagon_trinket_discount" => e.id.is_empty(),
            _ => !e.id.is_empty(),
        };
        if !known {
            return Err(format!("unsupported {} value {}", e.kind, e.id));
        }
        Ok(e)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct TownContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<super::automation::Owned>,
    pub owner: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub week: Option<u32>,
    pub effects: Vec<Effect>,
    pub event_ids: Vec<String>,
    pub used: BTreeMap<String, u32>,
    pub districts_unlocked: bool,
    pub contagion: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quest_event_opportunity: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weekly_lost_assets: Vec<super::super::Asset>,
}
impl TownContext {
    pub(in super::super) fn new(owner: String, districts_unlocked: bool, contagion: Value) -> Self {
        Self {
            automation: None,
            owner,
            week: None,
            effects: vec![],
            event_ids: vec![],
            used: BTreeMap::new(),
            districts_unlocked,
            contagion,
            quest_event_opportunity: false,
            weekly_lost_assets: vec![],
        }
    }
    pub(in super::super) fn validate(&self) -> Result<()> {
        if !(self.owner.starts_with(CAMPAIGN_PROFILE)
            || self.owner.starts_with(super::automation::PROFILE)
            || self.owner.starts_with(super::content019::PROFILE))
            || self.effects.len() > 1000
        {
            return Err("invalid campaign town context".into());
        }
        if self.weekly_lost_assets.len() > 10000 {
            return Err("weekly asset loss receipt bound".into());
        }
        for asset in &self.weekly_lost_assets {
            if asset.owner.is_some() {
                return Err("invalid weekly loss owner".into());
            }
            asset.state.validate()?;
        }
        for e in &self.effects {
            if !e.amount.is_finite() || !dd_data::campaign017::EFFECTS.contains(&e.kind.as_str()) {
                return Err("invalid campaign event service effect".into());
            }
        }
        Ok(())
    }
    pub(in super::super) fn sum(&self, kind: &str, id: &str) -> f64 {
        self.effects
            .iter()
            .filter(|e| e.kind == kind && e.id == id)
            .map(|e| e.amount)
            .sum()
    }
    pub(in super::super) fn has(&self, kind: &str, id: &str) -> bool {
        self.effects.iter().any(|e| e.kind == kind && e.id == id)
    }
    pub(in super::super) fn free(&self, tag: &str) -> bool {
        self.sum("upgrade_tag_free", tag) > f64::from(self.used.get(tag).copied().unwrap_or(0))
    }
    pub(in super::super) fn usage(
        &self,
        action: &super::super::Action,
        data: &Data,
    ) -> Result<Option<String>> {
        Ok(match action {
            super::super::Action::Upgrade { hero, tree, .. } => {
                let tags = data.tree(tree)?["tags"]
                    .as_array()
                    .ok_or("missing upgrade tags")?
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .chain(hero.is_none().then_some("building".to_string()))
                    .collect::<Vec<_>>();
                tags.into_iter().find(|t| self.free(t))
            }
            super::super::Action::LearnCamp { .. } if self.free("camping_skill") => {
                Some("camping_skill".into())
            }
            super::super::Action::Buy { .. }
                if self
                    .effects
                    .iter()
                    .any(|e| e.kind == "nomad_wagon_trinket_discount")
                    && !self.used.contains_key("nomad_wagon_trinket_discount") =>
            {
                Some("nomad_wagon_trinket_discount".into())
            }
            _ => None,
        })
    }
    pub(in super::super) fn consume(&mut self, usage: Option<String>) -> Result<()> {
        if let Some(key) = usage {
            let n = self.used.entry(key).or_default();
            *n = n.checked_add(1).ok_or("event usage overflow")?;
        }
        Ok(())
    }
}

impl TownSession {
    pub(in super::super) fn campaign_idle_relief(&self, amount: i32) -> Result<i32> {
        let Some(c) = &self.state.campaign else {
            return Ok(amount);
        };
        let mut bonus = 0.;
        for e in c.effects.iter().filter(|e| e.kind == "idle_buff") {
            let b = library::unique_json(&self.definition.game.buffs, &e.id, "idle event buff")?;
            if b["stat_type"] != "stress_heal_received_percent"
                || b["rule_type"] != "always"
                || b["is_false_rule"] != false
            {
                return Err("unsupported idle event buff rule".into());
            }
            bonus += b["amount"]
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or("invalid idle event buff amount")?;
        }
        Ok((f64::from(amount) * (1. + bonus).max(0.)).round() as i32)
    }
    pub(in super::super) fn campaign_idle_events(
        &mut self,
        party: &[u64],
        occupied: &BTreeSet<u64>,
    ) -> Result<()> {
        let Some(c) = &self.state.campaign else {
            return Ok(());
        };
        let effects = c.effects.clone();
        let ids = self
            .state
            .heroes
            .iter()
            .filter(|h| {
                h.state.alive
                    && !party.contains(&h.guid)
                    && !occupied.contains(&h.guid)
                    && h.missing_until <= self.effective_week()
            })
            .map(|h| h.guid)
            .collect::<Vec<_>>();
        for e in effects {
            match e.kind.as_str() {
                "idle_resolve_level" => {
                    for id in &ids {
                        let i = self.hero_index(*id)?;
                        if self.state.heroes[i].class != e.id {
                            continue;
                        }
                        let max =
                            self.definition.data.base.roster_variables["resolve_level_thresholds"]
                                .as_array()
                                .ok_or("missing resolve thresholds")?
                                .len()
                                .checked_sub(1)
                                .ok_or("empty resolve thresholds")?
                                as u32;
                        let level = self
                            .level(self.state.heroes[i].xp)?
                            .checked_add(e.amount as u32)
                            .ok_or("idle resolve overflow")?
                            .min(max);
                        let xp =
                            self.definition.data.base.roster_variables["resolve_level_thresholds"]
                                [level as usize]
                                .as_u64()
                                .and_then(|n| u32::try_from(n).ok())
                                .ok_or("missing idle resolve threshold")?;
                        self.state.heroes[i].xp = self.state.heroes[i].xp.max(xp);
                    }
                }
                "idle_negative_quirk_remove" => {
                    for id in &ids {
                        for _ in 0..e.amount as u32 {
                            let i = self.hero_index(*id)?;
                            let eligible = self.state.heroes[i]
                                .quirks
                                .keys()
                                .filter(|q| {
                                    library::unique_json(
                                        &self.definition.data.base.quirk_defs,
                                        q,
                                        "idle quirk removal",
                                    )
                                    .is_ok_and(|q| {
                                        q["is_positive"] == false && q["is_disease"] == false
                                    })
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            if eligible.is_empty() {
                                break;
                            }
                            let chosen = self.state.rng.below(eligible.len() as u64) as usize;
                            self.state.heroes[i].quirks.remove(&eligible[chosen]);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub(in super::super) fn campaign_contagion(&mut self) -> Result<()> {
        let Some(c) = &self.state.campaign else {
            return Ok(());
        };
        let source = c.contagion.clone();
        if source.is_null() {
            return Ok(());
        }
        let alive = self
            .state
            .heroes
            .iter()
            .filter(|h| h.state.alive)
            .map(|h| h.guid)
            .collect::<Vec<_>>();
        // Contacts are frozen before transmission: a newly infected hero never infects others this week.
        let infected = alive
            .iter()
            .filter(|id| {
                self.hero(**id).is_ok_and(|h| {
                    h.state.diseases.iter().any(|id| {
                        library::unique_json(
                            &self.definition.data.base.quirk_defs,
                            id,
                            "contagion disease",
                        )
                        .is_ok_and(|q| {
                            q["tags"]
                                .as_array()
                                .is_some_and(|a| a.iter().any(|v| v == "vampire"))
                        })
                    })
                })
            })
            .copied()
            .collect::<BTreeSet<_>>();
        if infected.is_empty() {
            return Ok(());
        }
        let mut contracted = 0usize;
        for id in &alive {
            if infected.contains(id) || self.contagion_immune019(*id)? {
                continue;
            }
            let activity = self
                .state
                .jobs
                .iter()
                .find(|j| j.hero == *id)
                .map(|j| j.facility.clone());
            let contacts = |same_activity: bool| {
                self.state
                    .jobs
                    .iter()
                    .filter(|j| {
                        infected.contains(&j.hero)
                            && activity.as_ref().is_some_and(|a| {
                                if same_activity {
                                    &j.facility == a
                                } else {
                                    j.facility.split('.').next() == a.split('.').next()
                                }
                            })
                    })
                    .count()
            };
            let exact = contacts(true);
            let building = contacts(false);
            let (scope, n) = if exact > 0 {
                ("activity", exact)
            } else if building > 0 {
                ("building", building)
            } else {
                ("idle", infected.len())
            };
            let rows = source[scope]
                .as_array()
                .ok_or("missing town contagion scope")?;
            for row in rows {
                if row["quirk_tag"] != "vampire" {
                    return Err("unsupported town contagion family".into());
                }
                let num = |key: &str| -> Result<f64> {
                    row[key]
                        .as_f64()
                        .filter(|n| n.is_finite())
                        .ok_or_else(|| format!("invalid town contagion {key}"))
                };
                let limit = num("roster_infected_limit_percent")?;
                let cap = (alive.len() as f64 * num("max_contraction_percent")?).ceil() as usize;
                if contracted >= cap
                    || (infected.len() + contracted) as f64 >= alive.len() as f64 * limit
                {
                    continue;
                }
                let mut kernel = PartyKernel::new(
                    vec![self.blueprint(self.hero(*id)?, 1, false)?],
                    self.definition.game.rules.clone(),
                    Rng(1),
                )?;
                kernel.exploration_buffs(&self.definition.game.buffs)?;
                let resist = kernel.core.units[0].blueprint.mechanics.disease_resist
                    + kernel.modifier(0, "resistance", "disease");
                let effective = (resist + num("disease_resistance_addition_mod")?)
                    * num("disease_resistance_multiplication_mod")?;
                let lo = num("min_chance")?;
                let hi = num("max_chance")?;
                if lo < 0. || hi > 1. || hi < lo {
                    return Err("invalid contagion probability interval".into());
                }
                let p = (num("base_contraction_chance")?
                    + n as f64 * num("contraction_chance_per_infected_hero")?
                    - effective)
                    .clamp(lo, hi);
                if self.state.rng.roll(p) {
                    self.add_quirk(*id, "disease_vampire_passive")?;
                    contracted += 1;
                    self.state
                        .log
                        .push(json!({"contagion":id,"scope":scope,"contacts":n}));
                }
            }
        }
        Ok(())
    }
}
