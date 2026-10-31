//! Continuous Farmstead battle controller; curios and campaign rewards are host handoffs.
use super::*;
use dd_data::farm011::{Formation, Plan, Wave};
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Runtime {
    pub reflection: Option<usize>,
    pub group: u32,
    pub template: usize,
    pub wave: usize,
    pub quota: u32,
    pub kills: u32,
    pub overflow: u32,
    pub total: u32,
    pub counted: std::collections::BTreeSet<UnitId>,
    pub stage_start: usize,
    pub roots: Vec<UnitId>,
    pub gate: Option<UnitId>,
    pub last_fill: Option<u32>,
    pub death_round: Option<u32>,
    pub region: String,
    pub light: String,
    pub boss: bool,
    pub interlude: Option<Interlude>,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Interlude {
    pub group: u32,
    pub wave: usize,
    pub curio: String,
    pub background: String,
    pub group_complete: bool,
    pub overflow: u32,
    pub single_shards: [u32; 2],
    pub double_shards: [u32; 2],
    pub boss_shards: [u32; 2],
    pub threshold_multiplier: f64,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WaveView {
    pub reflection: Option<String>,
    pub group: u32,
    pub wave: usize,
    pub kills: u32,
    pub quota: u32,
    pub total_kills: u32,
    pub region: String,
    pub light: String,
    pub interlude: bool,
}
impl Battle {
    pub fn new_farm(
        scenario: Scenario,
        plan: Plan,
        catalog: BTreeMap<String, Blueprint>,
        seed: u64,
    ) -> Result<Self> {
        Self::new_farm_with_supplies(scenario, plan, catalog, seed, vec![], BTreeMap::new())
    }
    pub fn new_farm_with_supplies(
        mut scenario: Scenario,
        plan: Plan,
        catalog: BTreeMap<String, Blueprint>,
        seed: u64,
        supplies: Vec<dd_data::items::SupplyStack>,
        reserves: BTreeMap<String, u32>,
    ) -> Result<Self> {
        plan.validate()?;
        if scenario.rules.flashback.is_some() || !scenario.monsters.is_empty() {
            return Err(
                "farm constructor requires empty initial enemy formation and no flashback".into(),
            );
        }
        if scenario
            .rules
            .encounter_contract
            .as_ref()
            .is_some_and(|c| c != &plan.contract)
        {
            return Err("explicit farm contract conflicts with source profile".into());
        }
        scenario.rules.encounter_contract = Some(plan.contract.clone());
        scenario.rules.lighting = Some(dd_data::lighting::Lighting {
            torch: 100,
            table: plan.darkness.clone(),
        });
        scenario.rules.farm = Some(plan);
        Self::build_with_reserves(scenario, seed, supplies, catalog, vec![], reserves)
    }
    pub fn farm_interlude(&self) -> Option<&Interlude> {
        self.lifecycle
            .special
            .farm
            .as_ref()
            .and_then(|s| s.interlude.as_ref())
    }
    pub(super) fn farm_view(&self) -> Option<WaveView> {
        self.lifecycle.special.farm.as_ref().map(|s| WaveView {
            reflection: self.scenario.rules.farm.as_ref().and_then(|p| {
                s.reflection
                    .and_then(|i| p.reflections.get(i))
                    .filter(|r| s.total >= r.threshold)
                    .map(|r| r.id.clone())
            }),
            group: s.group + 1,
            wave: s.wave + 1,
            kills: s.kills,
            quota: s.quota,
            total_kills: s.total,
            region: s.region.clone(),
            light: s.light.clone(),
            interlude: s.interlude.is_some(),
        })
    }
    fn farm_wave(&self) -> Wave {
        let p = self.scenario.rules.farm.as_ref().unwrap();
        let s = self.lifecycle.special.farm.as_ref().unwrap();
        if (s.group as usize) < p.params.intro_wave_groups.len() {
            p.params.intro_wave_groups[s.group as usize].wave_infos[s.wave].clone()
        } else {
            p.params.recurring_wave_groups[s.template].wave_infos[s.wave].clone()
        }
    }
    pub(super) fn initialize_farm(&mut self, events: &mut Vec<Event>) -> Result<()> {
        if self.scenario.rules.farm.is_none() {
            return Ok(());
        }
        let n = self.scenario.rules.farm.as_ref().unwrap().reflections.len();
        let reflection = if n == 0 {
            None
        } else {
            Some(self.rng.below(n as u64) as usize)
        };
        self.lifecycle.special.farm = Some(Runtime {
            reflection,
            ..Default::default()
        });
        self.begin_farm_wave(events)
    }
    fn begin_farm_wave(&mut self, events: &mut Vec<Event>) -> Result<()> {
        let p = self.scenario.rules.farm.clone().unwrap();
        let wave = self.farm_wave();
        let quota =
            wave.kills[0] + self.rng.below(u64::from(wave.kills[1] - wave.kills[0] + 1)) as u32;
        let mut light = "wave_farm".to_string();
        let mut region = "farm".to_string();
        if wave.kind == "space" {
            let keys = p
                .lights
                .iter()
                .filter(|(_, l)| !l.regions.is_empty())
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            if keys.is_empty() {
                return Err("no time/space lighting choices".into());
            }
            light = keys[self.rng.below(keys.len() as u64) as usize].clone();
            let regions = p.lights[&light]
                .regions
                .iter()
                .filter(|id| {
                    p.tables.get(*id).is_some_and(|t| {
                        (!t.recurring_only
                            || self.lifecycle.special.farm.as_ref().unwrap().group as usize
                                >= p.params.intro_wave_groups.len())
                            && t.requirements.iter().all(|q| p.eligible_quests.contains(q))
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            if regions.is_empty() {
                return Err("no eligible time/space region".into());
            }
            let weights = regions
                .iter()
                .map(|r| p.tables[r].random_chance)
                .collect::<Vec<_>>();
            region = regions[self.weighted_index(&weights)?].clone();
        } else if self.lifecycle.special.farm.as_ref().unwrap().group > 0 {
            light = "wave_pre_time_and_space".into();
        }
        let mut boss = wave.kind == "boss" || wave.kind == "miniboss";
        let mut space_boss = None;
        if wave.kind == "space" && !p.tables[&region].local_minibosses.is_empty() {
            let state = self.lifecycle.special.farm.as_ref().unwrap();
            let group = if (state.group as usize) < p.params.intro_wave_groups.len() {
                &p.params.intro_wave_groups[state.group as usize]
            } else {
                &p.params.recurring_wave_groups[state.template]
            };
            let progress = (state.wave + 1) as f64 / group.wave_infos.len() as f64;
            let ease = if p.tables["farm"].ease.as_deref() == Some("easeSqrt") {
                progress.sqrt()
            } else {
                progress
            };
            if self.rng.roll(p.params.space_miniboss_chance * ease) {
                boss = true;
                space_boss = Some(p.tables[&region].local_minibosses.clone());
            }
        }
        let stage_start = self.units.len();
        {
            let s = self.lifecycle.special.farm.as_mut().unwrap();
            s.quota = quota;
            s.kills = s.overflow;
            s.overflow = 0;
            s.region = region.clone();
            s.light = light.clone();
            s.boss = boss;
            s.stage_start = stage_start;
            s.roots.clear();
            s.gate = None;
            s.last_fill = None;
            s.death_round = None;
            s.interlude = None;
        }
        if let Some(ctx) = self.scenario.rules.context.as_mut() {
            ctx.region = region.clone();
            ctx.stall = false;
        }
        self.queue.clear();
        self.actor = None;
        self.lifecycle.bonus_used.clear();
        self.lifecycle.new_wave = self.round > 0;
        events.push(Event::FarmWave {
            group: self.lifecycle.special.farm.as_ref().unwrap().group + 1,
            wave: self.lifecycle.special.farm.as_ref().unwrap().wave + 1,
            quota,
            region: region.clone(),
            light: light.clone(),
        });
        if boss {
            let candidates = if let Some(pool) = space_boss {
                pool
            } else if wave.kind == "boss" {
                p.tables["farm"]
                    .bosses
                    .iter()
                    .filter(|f| f.background.as_deref() == Some(&wave.background))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                p.tables["farm"].minibosses.clone()
            };
            let f = self.farm_choose(&candidates)?;
            for id in &f.monsters {
                let n = self.farm_spawn(id, false, events)?;
                self.lifecycle.special.farm.as_mut().unwrap().roots.push(n);
            }
        } else {
            self.fill_farm(events)?;
        }
        self.initialize_shared_health()?;
        self.refresh_context(true);
        let l = p.lights[&light].clone();
        for id in stage_start..self.units.len() {
            for e in &l.monster_effects {
                self.apply_effect(id, id, e, events);
            }
        }
        for id in 0..self.scenario.heroes.len() {
            if self.units[id].alive() {
                for e in &l.hero_effects {
                    self.apply_effect(id, id, e, events);
                }
            }
        }
        self.flush_lifecycle(events);
        Ok(())
    }
    fn farm_choose(&mut self, formations: &[Formation]) -> Result<Formation> {
        if formations.is_empty() {
            return Err("missing eligible wave formation".into());
        }
        let n = self.weighted_index(&formations.iter().map(|f| f.weight).collect::<Vec<_>>())?;
        Ok(formations[n].clone())
    }
    pub(super) fn farm_spawn(
        &mut self,
        class: &str,
        front: bool,
        events: &mut Vec<Event>,
    ) -> Result<UnitId> {
        let b = self
            .lifecycle
            .catalog
            .get(class)
            .cloned()
            .ok_or_else(|| format!("unresolved farm spawn {class}"))?;
        let occupied = self.order[1]
            .iter()
            .filter(|id| self.units[**id].present())
            .map(|id| self.units[*id].size())
            .sum::<u8>();
        if occupied + b.size > 4 {
            return Err("farm spawn exceeds formation capacity".into());
        }
        let id = self.units.len();
        self.units.push(Unit::fresh(&b));
        if front {
            self.order[1].insert(0, id);
        } else {
            self.order[1].push(id);
        }
        events.push(Event::Spawned {
            unit: id,
            class: class.into(),
            rank: self.rank(id),
            loot: b.mechanics.lifecycle.loot,
        });
        self.initialize_entity(id, events);
        self.apply_wave_spawn(id, events);
        Ok(id)
    }
    fn fill_farm(&mut self, events: &mut Vec<Event>) -> Result<()> {
        let p = self.scenario.rules.farm.clone().unwrap();
        let s = self.lifecycle.special.farm.as_ref().unwrap().clone();
        let table = p.tables[&s.region].clone();
        // Replace corpses only when a delayed reinforcement batch actually occurs.
        for id in self.order[1].clone() {
            if !self.units[id].alive() {
                self.retire(id, events);
            }
        }
        self.order[1].retain(|id| self.units[*id].present());
        loop {
            let size = self.order[1]
                .iter()
                .filter(|id| self.units[**id].present())
                .map(|id| self.units[*id].size())
                .sum::<u8>();
            if size >= 4 {
                break;
            }
            let eligible = table
                .mash
                .iter()
                .filter(|f| {
                    f.monsters
                        .iter()
                        .map(|id| self.lifecycle.catalog[id].size)
                        .sum::<u8>()
                        <= 4 - size
                })
                .cloned()
                .collect::<Vec<_>>();
            if eligible.is_empty() {
                break;
            }
            let f = self.farm_choose(&eligible)?;
            for id in &f.monsters {
                let preferred = self.lifecycle.catalog[id]
                    .mechanics
                    .boss
                    .wave_front
                    .unwrap_or(false);
                let front = preferred
                    && table
                        .front_chance
                        .is_some_and(|chance| self.rng.roll(chance));
                self.farm_spawn(id, front, events)?;
            }
        }
        self.lifecycle.special.farm.as_mut().unwrap().last_fill = Some(self.round);
        Ok(())
    }
    pub(super) fn farm_settle(&mut self, events: &mut Vec<Event>) {
        let Some(mut s) = self.lifecycle.special.farm.clone() else {
            return;
        };
        if s.interlude.is_some()
            || s.complete
            || !self.units.iter().any(|u| u.alive() && u.blueprint.hero)
        {
            return;
        }
        let p = self.scenario.rules.farm.as_ref().unwrap();
        let except = &p.tables[&s.region].kill_exceptions;
        for (id, u) in self.units.iter().enumerate().skip(s.stage_start) {
            if !u.alive()
                && self.lifecycle.special.killed.contains_key(&id)
                && !except.contains(&u.blueprint.id)
                && !except.contains(&u.source_id)
                && s.counted.insert(id)
            {
                s.total += 1;
                s.kills += 1;
                s.death_round = Some(self.round);
            }
        }
        let boss_finished =
            s.boss && !s.roots.is_empty() && s.roots.iter().all(|id| !self.units[*id].alive());
        self.lifecycle.special.farm = Some(s);
        if boss_finished && let Err(e) = self.finish_farm_wave(events) {
            self.lifecycle.failure = Some(e);
        }
    }
    pub(super) fn farm_round_end(&mut self, events: &mut Vec<Event>) -> Result<()> {
        let Some(s) = self.lifecycle.special.farm.clone() else {
            return Ok(());
        };
        if s.boss || s.complete || s.interlude.is_some() || self.outcome.is_some() {
            return Ok(());
        }
        let p = self.scenario.rules.farm.clone().unwrap();
        if s.kills >= s.quota && s.gate.is_none() {
            for id in self.order[1].clone() {
                self.retire(id, events);
            }
            self.order[1].clear();
            let gate = p.tables["farm"]
                .gate
                .as_ref()
                .ok_or("missing original wave teleport entity")?;
            let id = self.farm_spawn(gate, false, events)?;
            self.lifecycle.special.farm.as_mut().unwrap().gate = Some(id);
            return Ok(());
        }
        if s.gate.is_some() {
            return Ok(());
        }
        if s.death_round.is_some_and(|r| r < self.round)
            && s.last_fill.is_none_or(|r| r + 1 < self.round)
        {
            self.fill_farm(events)?;
            self.lifecycle.special.farm.as_mut().unwrap().death_round = None;
            return Ok(());
        }
        let crystal = p.tables["farm"]
            .crystal
            .as_ref()
            .ok_or("missing original crystal replacement")?;
        for id in self.order[1].clone() {
            if matches!(self.units[id].life, Life::Corpse { .. }) {
                let count = self.units[id].size();
                self.retire(id, events);
                self.order[1].retain(|n| self.units[*n].present());
                for _ in 0..count {
                    self.farm_spawn(crystal, false, events)?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn farm_teleport(&mut self, actor: UnitId, events: &mut Vec<Event>) -> bool {
        if self
            .lifecycle
            .special
            .farm
            .as_ref()
            .is_some_and(|s| s.gate == Some(actor))
        {
            if let Err(e) = self.finish_farm_wave(events) {
                self.lifecycle.failure = Some(e);
            }
            true
        } else {
            false
        }
    }
    fn finish_farm_wave(&mut self, events: &mut Vec<Event>) -> Result<()> {
        let p = self.scenario.rules.farm.clone().unwrap();
        let wave = self.farm_wave();
        let s = self.lifecycle.special.farm.as_ref().unwrap().clone();
        if s.interlude.is_some() {
            return Ok(());
        }
        let group = if (s.group as usize) < p.params.intro_wave_groups.len() {
            &p.params.intro_wave_groups[s.group as usize]
        } else {
            &p.params.recurring_wave_groups[s.template]
        };
        let group_complete = s.wave + 1 == group.wave_infos.len();
        if !wave.curio.is_empty() && !wave.curio_names.is_empty() {
            return Err("ambiguous wave curio fields".into());
        }
        let mut curio = if wave.curio_names.is_empty() {
            wave.curio.clone()
        } else {
            wave.curio_names[self.rng.below(wave.curio_names.len() as u64) as usize].clone()
        };
        if let Some(g) = p.params.curio_groups.iter().find(|g| g.name == curio) {
            if g.curios.is_empty() {
                return Err("empty wave curio group".into());
            }
            curio = g.curios[self.rng.below(g.curios.len() as u64) as usize].clone();
        }
        let mut background = wave.background.clone();
        if let Some(g) = p
            .params
            .background_groups
            .iter()
            .find(|g| g.name == background)
        {
            if g.backgrounds.is_empty() {
                return Err("empty wave background group".into());
            }
            background = g.backgrounds[self.rng.below(g.backgrounds.len() as u64) as usize].clone();
        }
        let interlude = Interlude {
            group: s.group + 1,
            wave: s.wave + 1,
            curio,
            background,
            group_complete,
            overflow: if s.boss {
                0
            } else {
                s.kills.saturating_sub(s.quota)
            },
            single_shards: p.params.single_shard_rewards,
            double_shards: p.params.double_shard_rewards,
            boss_shards: p.params.boss_shard_rewards,
            threshold_multiplier: p.params.threshold_shard_reward_multiplier,
        };
        for id in self.order[1].clone() {
            if !self.units[id].blueprint.hero {
                self.retire(id, events);
            }
        }
        self.cleanup_captives(events);
        self.cleanup_control(events);
        self.order[1].clear();
        self.queue.clear();
        self.actor = None;
        self.lifecycle.new_wave = false;
        self.lifecycle.special.farm.as_mut().unwrap().interlude = Some(interlude);
        self.outcome = Some(Outcome::WaveInterlude);
        events.push(Event::FarmInterlude {
            group: s.group + 1,
            wave: s.wave + 1,
            group_complete,
        });
        self.refresh_context(true);
        Ok(())
    }
    /// Resume after the host has handled (or explicitly skipped) this wave's curio.
    pub fn continue_farm(&mut self) -> Result<Vec<Event>> {
        if self.outcome != Some(Outcome::WaveInterlude) {
            return Err("no settled farm interlude".into());
        }
        let mut b = self.clone();
        let p = b.scenario.rules.farm.clone().ok_or("missing wave plan")?;
        let interlude = b.farm_interlude().cloned().ok_or("missing wave handoff")?;
        let mut events = vec![];
        let mut s = b.lifecycle.special.farm.take().unwrap();
        s.overflow = interlude.overflow;
        s.interlude = None;
        if interlude.group_complete {
            s.group += 1;
            s.wave = 0;
            if s.group as usize >= p.params.intro_wave_groups.len() {
                if p.params.recurring_wave_groups.is_empty() {
                    s.complete = true;
                    b.outcome = Some(Outcome::HeroesWin);
                    b.lifecycle.special.farm = Some(s);
                    *self = b;
                    return Ok(events);
                }
                s.template = b.rng.below(p.params.recurring_wave_groups.len() as u64) as usize;
            }
            b.lifecycle.items.ended = false;
            for u in b.units.iter_mut().filter(|u| u.blueprint.hero && u.alive()) {
                for (i, skill) in u.blueprint.skills.iter().enumerate() {
                    u.skill_remaining[i] = skill.per_battle_limit;
                    u.used_skills.remove(&skill.id);
                }
            }
        } else {
            s.wave += 1;
        }
        b.lifecycle.special.farm = Some(s);
        b.outcome = None;
        b.begin_farm_wave(&mut events)?;
        for u in b.units.iter_mut().filter(|u| u.blueprint.hero && u.alive()) {
            for (i, skill) in u.blueprint.skills.iter().enumerate() {
                if skill.tactical.refresh_each_wave {
                    u.skill_remaining[i] = skill.per_battle_limit;
                    u.used_skills.remove(&skill.id);
                }
            }
        }
        b.advance(&mut events);
        if let Some(error) = b.lifecycle.failure.take() {
            return Err(error);
        }
        *self = b;
        Ok(events)
    }
    pub(super) fn farm_group_end(&mut self, events: &mut Vec<Event>) {
        if self.lifecycle.items.ended
            || !self.farm_interlude().is_some_and(|i| i.group_complete)
            || !self.pending_reactions.is_empty()
            || self.units.iter().any(|u| u.attack_context.is_some())
        {
            return;
        }
        self.trinket_battle_end(events);
        for id in 0..self.scenario.heroes.len() {
            if self.units[id].alive() {
                self.mode_forced(id, true, events);
                self.units[id].buffs.retain(|b| {
                    b.scope != BuffScope::Timed
                        || (!b.battle_persistent
                            && b.definition.duration_type.as_deref() != Some("combat_end"))
                });
                if self.units[id].riposte.as_ref().is_some_and(|r| r.permanent) {
                    self.units[id].riposte = None;
                }
            }
        }
        self.refresh_context(true);
    }
    pub(super) fn farm_buffs(&self, id: UnitId) -> Vec<Buff> {
        let Some(s) = self.lifecycle.special.farm.as_ref() else {
            return vec![];
        };
        let Some(p) = &self.scenario.rules.farm else {
            return vec![];
        };
        let Some(l) = p.lights.get(&s.light) else {
            return vec![];
        };
        let mut buffs = if self.units[id].blueprint.hero {
            l.hero_buffs.clone()
        } else {
            l.monster_buffs.clone()
        };
        if let Some(r) = s
            .reflection
            .and_then(|i| p.reflections.get(i))
            .filter(|r| s.total >= r.threshold)
        {
            buffs.extend(if self.units[id].blueprint.hero {
                r.hero_buffs.clone()
            } else {
                r.monster_buffs.clone()
            });
        }
        buffs
    }
    pub(super) fn validate_farm(&self) -> Result<()> {
        let Some(p) = &self.scenario.rules.farm else {
            if self.lifecycle.special.farm.is_some() {
                return Err("orphan farm runtime".into());
            }
            return Ok(());
        };
        p.validate()?;
        let s = self
            .lifecycle
            .special
            .farm
            .as_ref()
            .ok_or("missing farm runtime")?;
        if !self.lifecycle.waves.is_empty()
            || s.total != s.counted.len() as u32
            || s.reflection.is_some_and(|i| i >= p.reflections.len())
            || (s.reflection.is_none() != p.reflections.is_empty())
            || self.scenario.rules.flashback.is_some()
            || !p.tables.contains_key(&s.region)
            || !p.lights.contains_key(&s.light)
            || s.stage_start > self.units.len()
            || s.roots
                .iter()
                .chain(s.gate.iter())
                .any(|id| *id < s.stage_start || *id >= self.units.len())
            || s.quota == 0
            || s.kills > s.total + s.overflow
            || s.counted
                .iter()
                .any(|id| *id >= self.units.len() || self.units[*id].alive())
            || s.interlude.is_some() != (self.outcome == Some(Outcome::WaveInterlude))
        {
            return Err("invalid checkpoint farm state".into());
        }
        let group = if (s.group as usize) < p.params.intro_wave_groups.len() {
            p.params.intro_wave_groups.get(s.group as usize)
        } else {
            p.params.recurring_wave_groups.get(s.template)
        };
        if !s.complete && group.is_none_or(|g| s.wave >= g.wave_infos.len()) {
            return Err("invalid checkpoint wave cursor".into());
        }
        if self.scenario.rules.encounter_contract.as_ref() != Some(&p.contract)
            || self
                .scenario
                .rules
                .lighting
                .as_ref()
                .is_none_or(|l| l.torch != 100 || l.table != p.darkness)
            || s.last_fill.is_some_and(|r| r > self.round)
            || s.death_round.is_some_and(|r| r > self.round)
        {
            return Err("checkpoint farm context disagrees with source plan".into());
        }
        if s.complete {
            if !p.params.recurring_wave_groups.is_empty()
                || s.group as usize != p.params.intro_wave_groups.len()
                || self.outcome != Some(Outcome::HeroesWin)
            {
                return Err("invalid completed farm profile".into());
            }
        } else if let Some(group) = group {
            let wave = &group.wave_infos[s.wave];
            if !(wave.kills[0]..=wave.kills[1]).contains(&s.quota) {
                return Err("checkpoint wave quota disagrees with source".into());
            }
            if let Some(i) = &s.interlude {
                let curio = if wave.curio_names.is_empty() {
                    vec![wave.curio.clone()]
                } else {
                    wave.curio_names.clone()
                };
                let valid_curio = curio.iter().any(|id| {
                    p.params
                        .curio_groups
                        .iter()
                        .find(|g| &g.name == id)
                        .map_or_else(|| id == &i.curio, |g| g.curios.contains(&i.curio))
                });
                let valid_bg = p
                    .params
                    .background_groups
                    .iter()
                    .find(|g| g.name == wave.background)
                    .map_or_else(
                        || wave.background == i.background,
                        |g| g.backgrounds.contains(&i.background),
                    );
                if i.group != s.group + 1
                    || i.wave != s.wave + 1
                    || i.group_complete != (s.wave + 1 == group.wave_infos.len())
                    || i.overflow
                        != if s.boss {
                            0
                        } else {
                            s.kills.saturating_sub(s.quota)
                        }
                    || !valid_curio
                    || !valid_bg
                    || i.single_shards != p.params.single_shard_rewards
                    || i.double_shards != p.params.double_shard_rewards
                    || i.boss_shards != p.params.boss_shard_rewards
                    || i.threshold_multiplier != p.params.threshold_shard_reward_multiplier
                {
                    return Err("checkpoint farm handoff disagrees with source".into());
                }
            }
        }
        Ok(())
    }
}
