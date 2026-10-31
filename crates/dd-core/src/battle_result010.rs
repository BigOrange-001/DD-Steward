//! Owned host handoff. This is not part of the policy Observation.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeroResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_buff_instances: Vec<u64>,
    pub difficulty: Option<dd_data::difficulty012::HeroDifficulty>,
    pub content_id: String,
    pub candidate_id: String,
    pub status: dd_data::hero010::CarryStatus,
    pub death_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trinket_slots: Option<Vec<usize>>,
    pub trinkets: Vec<dd_data::trinket010::TrinketState>,
    pub disease_turns: BTreeMap<String, u32>,
    pub resolve_history: BTreeMap<String, u32>,
    pub dungeon_history: std::collections::BTreeSet<String>,
    pub id: UnitId,
    pub class: String,
    pub alive: bool,
    pub hp: i32,
    pub max_hp: i32,
    pub stress: i32,
    pub resolve: Resolve,
    pub build: dd_data::build010::Build,
    pub diseases: Vec<String>,
    pub persistent_buffs: Vec<dd_data::hero010::CarryBuff>,
    pub mode: Option<String>,
    pub limited_skill_uses: BTreeMap<String, u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InventoryReceipt {
    pub input_slots: usize,
    pub states: Vec<dd_data::trinket010::TrinketState>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattleResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_inventory_states: Option<Box<InventoryReceipt>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_return_token: Option<String>,
    pub legacy_host_inputs: Option<dd_data::host_rules012::LegacyHostInputs>,
    pub candidate_id: String,
    pub encounter_contract: Option<dd_data::contract011::Contract>,
    pub ends_infestation_sequence: bool,
    pub completed_flashback_camping_buffs: Vec<Buff>,
    pub farm_interlude: Option<crate::FarmInterlude>,
    pub killed_monsters: Vec<(UnitId, String, String)>,
    pub escaped_monsters: Vec<(UnitId, String)>,
    pub flashback_id: Option<String>,
    pub flashback_result: Option<bool>,
    pub encounter_rewards: Vec<(String, String)>,
    pub profile: String,
    pub content_id: String,
    pub outcome: Outcome,
    pub heroes: Vec<HeroResult>,
    pub supplies: Vec<dd_data::items::SupplyStack>,
    pub trinket_inventory: Vec<dd_data::trinket010::TrinketState>,
    pub torch: Option<u8>,
    pub region: Option<String>,
    pub history: std::collections::BTreeSet<String>,
}
impl Battle {
    pub fn result(&self) -> Result<BattleResult> {
        let outcome = self.outcome.ok_or("battle has not finished")?;
        if !self.effect_jobs.is_empty()
            || !self.pending_reactions.is_empty()
            || !self.lifecycle.jobs.is_empty()
            || self.lifecycle.failure.is_some()
        {
            return Err("battle result is not settled".into());
        }
        Ok(BattleResult {
            host_inventory_states: if self.scenario.rules.host_return_token.is_some()
                && self
                    .scenario
                    .rules
                    .context
                    .as_ref()
                    .is_some_and(|c| !c.trinket_inventory.is_empty())
            {
                Some(Box::new(InventoryReceipt {
                    input_slots: self
                        .scenario
                        .rules
                        .context
                        .as_ref()
                        .unwrap()
                        .trinket_inventory
                        .len(),
                    states: self.lifecycle.items.bag.clone(),
                }))
            } else {
                None
            },
            host_return_token: self.scenario.rules.host_return_token.clone(),
            legacy_host_inputs: self.scenario.rules.legacy_host_inputs(),
            candidate_id: self.scenario.rules.candidates.identity()?,
            encounter_contract: self.scenario.rules.encounter_contract.clone(),
            ends_infestation_sequence: self
                .lifecycle
                .special
                .killed
                .keys()
                .any(|id| self.units[*id].blueprint.mechanics.boss.end_infestation),
            completed_flashback_camping_buffs: self
                .scenario
                .rules
                .flashback
                .as_ref()
                .filter(|_| self.lifecycle.special.dream.ended == Some(true))
                .map(|f| f.completed_camping_buffs.clone())
                .unwrap_or_default(),
            farm_interlude: self.farm_interlude().cloned(),
            killed_monsters: self
                .lifecycle
                .special
                .killed
                .iter()
                .map(|(id, (source, last))| (*id, source.clone(), last.clone()))
                .collect(),
            escaped_monsters: self
                .lifecycle
                .special
                .escaped
                .iter()
                .map(|(id, class)| (*id, class.clone()))
                .collect(),
            flashback_id: self.scenario.rules.flashback.as_ref().map(|f| f.id.clone()),
            flashback_result: self.lifecycle.special.dream.ended,
            encounter_rewards: self.lifecycle.special.dream.rewards.clone(),
            profile: self.profile.clone(),
            content_id: self.content_id.clone(),
            outcome,
            heroes: self.hero_results(),
            supplies: self
                .supplies
                .iter()
                .cloned()
                .chain(self.supplies.iter().filter_map(|s| {
                    self.supply_reserves.get(&s.definition.id).map(|n| {
                        dd_data::items::SupplyStack {
                            definition: s.definition.clone(),
                            remaining: *n,
                        }
                    })
                }))
                .collect(),
            trinket_inventory: self
                .lifecycle
                .items
                .bag
                .iter()
                .filter(|t| !t.destroyed)
                .cloned()
                .collect(),
            torch: self.scenario.rules.lighting.as_ref().map(|c| c.torch),
            history: self
                .scenario
                .rules
                .context
                .as_ref()
                .map(|c| c.history.clone())
                .unwrap_or_default(),
            region: self
                .scenario
                .rules
                .context
                .as_ref()
                .map(|c| c.region.clone()),
        })
    }
    pub(super) fn hero_results(&self) -> Vec<HeroResult> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.blueprint.hero)
            .map(|(id, u)| HeroResult {
                host_buff_instances: u.buffs.iter().filter_map(|b| b.host_instance).collect(),
                death_override: if !u.alive() {
                    self.scenario
                        .rules
                        .encounter_contract
                        .as_ref()
                        .and_then(|c| c.death_override.clone())
                } else {
                    None
                },
                trinket_slots: self.scenario.rules.host_return_token.as_ref().map(|_| {
                    u.trinkets
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| !t.destroyed)
                        .map(|(i, _)| i)
                        .collect()
                }),
                trinkets: u
                    .trinkets
                    .iter()
                    .filter(|t| !t.destroyed)
                    .cloned()
                    .collect(),
                disease_turns: u.disease_turns.clone(),
                resolve_history: u.resolve_history.clone(),
                dungeon_history: u
                    .blueprint
                    .mechanics
                    .hero
                    .dungeon_history
                    .iter()
                    .cloned()
                    .chain(
                        self.scenario
                            .rules
                            .context
                            .iter()
                            .flat_map(|c| c.history.iter().cloned()),
                    )
                    .collect(),
                difficulty: u.blueprint.mechanics.hero.difficulty.clone(),
                content_id: self.content_id.clone(),
                candidate_id: self
                    .scenario
                    .rules
                    .candidates
                    .identity()
                    .expect("validated candidate identity"),
                status: dd_data::hero010::CarryStatus {
                    dots: u
                        .dots
                        .iter()
                        .map(|d| dd_data::hero010::CarryDot {
                            kind: match d.kind {
                                EffectKind::Bleed(_) => dd_data::hero010::CarryDotKind::Bleed,
                                EffectKind::Blight(_) => dd_data::hero010::CarryDotKind::Blight,
                                EffectKind::Horror(_) => dd_data::hero010::CarryDotKind::Horror,
                                EffectKind::Regen(_) => dd_data::hero010::CarryDotKind::Regen,
                                _ => unreachable!("validated ongoing status"),
                            },
                            amount: d.amount,
                            remaining: d.remaining,
                            can_crit_heal: d.can_crit_heal,
                        })
                        .collect(),
                    burn: u.burn,
                    item_uses: u.item_uses.clone(),
                },
                id,
                class: u.blueprint.id.clone(),
                alive: u.alive(),
                hp: u.hp,
                max_hp: u.stats().hp,
                stress: u.stress,
                resolve: u.resolve.clone(),
                build: {
                    let mut build = u.blueprint.mechanics.build.input.clone();
                    if build.physical_trinkets.is_some() {
                        // Keep destroyed placeholders: native slot indices must continue
                        // to address the same original physical asset next encounter.
                        build.trinkets =
                            u.trinkets.iter().map(|t| t.definition.id.clone()).collect();
                        build.physical_trinkets = Some(u.trinkets.clone());
                        build.trinket_states.clear();
                    } else {
                        build.trinkets = u
                            .trinkets
                            .iter()
                            .filter(|t| !t.destroyed)
                            .map(|t| t.definition.id.clone())
                            .collect();
                        build.trinket_states = u
                            .trinkets
                            .iter()
                            .filter(|t| !t.destroyed)
                            .map(|t| (t.definition.id.clone(), t.clone()))
                            .collect();
                    }
                    build.diseases = u.diseases.clone();
                    build.initial_disease_turns = u.disease_turns.clone();
                    build
                },
                diseases: u.diseases.clone(),
                mode: u.mode.clone(),
                persistent_buffs: u
                    .buffs
                    .iter()
                    .filter(|b| {
                        b.host_instance.is_none()
                            && !(b.scope == BuffScope::Build
                                && u.blueprint
                                    .mechanics
                                    .hero
                                    .host_passive_buffs
                                    .contains(&b.definition))
                            && (b.definition.duration_type.as_deref() == Some("quest_end")
                                || !b.definition.activation.remove_on_battle_complete)
                    })
                    .map(|b| dd_data::hero010::CarryBuff {
                        definition: b.definition.clone(),
                        remaining: b.remaining,
                    })
                    .collect(),
                limited_skill_uses: u.used_skills.clone(),
            })
            .collect()
    }
}

impl HeroResult {
    /// Carry battle changes into a fresh original-data blueprint chosen by the host.
    /// Dead heroes cannot be turned into survivors by this handoff.
    pub fn carry_into(&self, g: &dd_data::combat::GameData, hero: Blueprint) -> Result<Blueprint> {
        if self.content_id != g.rules.content.id()?
            || self.candidate_id != g.rules.candidates.identity()?
        {
            return Err("surviving hero handoff configuration mismatch".into());
        }
        if !self.alive || hero.id != self.class || !hero.hero {
            return Err("invalid surviving hero handoff".into());
        }
        let mut hero = g.equip_build(hero, self.build.clone())?;
        if hero.mechanics.hero.difficulty.is_none() {
            hero.mechanics.hero.difficulty = self.difficulty.clone().map(|mut c| {
                c.entering_raid = false;
                c
            });
        }
        hero.mechanics.hero.resolve_history = self.resolve_history.clone();
        hero.mechanics.hero.dungeon_history = self.dungeon_history.clone();
        hero.mechanics.hero.initial_hp = Some(self.hp);
        hero.mechanics.hero.initial_mode = self.mode.clone();
        use dd_data::hero010::{CarryResolve, HeroCarry};
        self.status.validate()?;
        hero.mechanics.hero.carry = Some(HeroCarry {
            status: self.status.clone(),
            stress: self.stress,
            resolve: match &self.resolve {
                Resolve::Calm => CarryResolve::Calm,
                Resolve::Afflicted(id) => CarryResolve::Afflicted(id.clone()),
                Resolve::Virtuous(id) => CarryResolve::Virtuous(id.clone()),
            },
            buffs: self.persistent_buffs.clone(),
        });
        Ok(hero)
    }
}
