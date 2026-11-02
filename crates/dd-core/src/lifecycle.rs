use super::*;
use dd_data::lifecycle::{ChangeSpec, EntityRole, LifeOp, SummonSpec};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Runtime {
    pub skill_effect: bool,
    pub special: crate::boss011::Runtime,
    pub items: crate::trinkets010::Runtime,
    pub stall_count: u32,
    pub invalidating_actions: u32,
    pub zones: Vec<heroes010::RankBurn>,
    pub failure: Option<String>,
    pub catalog: BTreeMap<String, Blueprint>,
    pub waves: Vec<Vec<Blueprint>>,
    pub wave: usize,
    pub jobs: VecDeque<Job>,
    pub bonus_used: BTreeSet<(UnitId, usize)>,
    pub last_skill: Option<String>,
    pub new_wave: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Job {
    pub allow_dead: bool,
    pub actor: UnitId,
    pub target: UnitId,
    pub op: LifeOp,
}

impl Unit {
    pub(super) fn fresh(b: &Blueprint) -> Self {
        Self {
            miss_streak: 0,
            item_uses: BTreeMap::new(),
            source_id: b.id.clone(),
            shuffle_remaining: 0,
            phase_overkill: 0,
            previous_form: None,
            controlled: None,
            captive: None,
            parasites: vec![],
            immobilized: None,
            shared_max: None,
            shared_base_max: None,
            blueprint: b.clone(),
            trinkets: b.mechanics.build.trinkets.clone(),
            equipment_modifiers: vec![],
            disease_turns: b.mechanics.build.input.initial_disease_turns.clone(),
            resolve_history: b.mechanics.hero.resolve_history.clone(),
            critical_context: false,
            disease_tags: b.mechanics.build.disease_tags.clone(),
            quest_uses_context: BTreeMap::new(),
            item_blocked: std::collections::BTreeSet::new(),
            aura_modifiers: vec![],
            region_context: None,
            corridor_context: false,
            walking_backwards_context: false,
            light_context: None,
            items_context: vec![],
            monster_counts: BTreeMap::new(),
            diseases: b.mechanics.build.input.diseases.clone(),
            mode: None,
            door_reaction_round: None,
            hp_reaction_active: vec![false; b.mechanics.hero.low_hp.len()],
            light_modifiers: vec![],
            hp: b.mechanics.hero.initial_hp.unwrap_or(b.stats.hp),
            life: if b.mechanics.lifecycle.role == EntityRole::Corpse {
                Life::Corpse {
                    expires: b
                        .mechanics
                        .lifecycle
                        .lifetime
                        .expect("validated corpse lifetime"),
                }
            } else {
                Life::Alive
            },
            stunned: false,
            dots: vec![],
            buffs: vec![],
            stress: 0,
            resolve: Resolve::Calm,
            resolve_epoch: 0,
            entered_deaths_door: false,
            turns: 0,
            rank_turns: 0,
            turn_rank: 0,
            cooldowns: b
                .skills
                .iter()
                .enumerate()
                .filter(|(_, s)| s.tactical.boss.starting_cooldown > 0)
                .map(|(i, s)| (i, s.tactical.boss.starting_cooldown + 1))
                .collect(),
            skill_remaining: b.skills.iter().map(|s| s.per_battle_limit).collect(),
            marked: None,
            heart_recovery_pending: false,
            guard: None,
            riposte: None,
            blocks: 0,
            blocks_timer: None,
            rank_context: 0,
            round_context: 0,
            guarded_context: false,
            burn: 0,
            any_burning_context: false,
            counter_context: false,
            attack_context: None,
            round_actions: 0,
            round_budget: 0,
            round_extra: 0,
            turn_adjust: 0,
            next_bonus: 0,
            turn_uses: BTreeMap::new(),
            turn_history: BTreeMap::new(),
            used_skills: BTreeMap::new(),
            desire_rolls: vec![],
            loot: b.mechanics.lifecycle.loot,
            expires: None,
            link_bound: false,
            death_rank: 0,
        }
    }
    pub(super) fn action_count(&self) -> u8 {
        (i16::from(self.blueprint.mechanics.lifecycle.turns) + i16::from(self.turn_adjust))
            .clamp(0, 4) as u8
    }
}

impl Battle {
    /// Host input only. Neither the registry nor future waves enter Observation.
    pub fn new_encounter(
        scenario: Scenario,
        seed: u64,
        catalog: BTreeMap<String, Blueprint>,
        waves: Vec<Vec<Blueprint>>,
    ) -> Result<Self> {
        Self::new_encounter_with_supplies(scenario, seed, catalog, waves, vec![])
    }
    pub fn new_encounter_with_supplies(
        scenario: Scenario,
        seed: u64,
        catalog: BTreeMap<String, Blueprint>,
        waves: Vec<Vec<Blueprint>>,
        supplies: Vec<dd_data::items::SupplyStack>,
    ) -> Result<Self> {
        Self::build(scenario, seed, supplies, catalog, waves)
    }
    pub(super) fn validate_catalog(&self) -> Result<()> {
        if self.lifecycle.wave > self.lifecycle.waves.len() || self.lifecycle.waves.len() > 10000 {
            return Err("invalid wave index/count".into());
        }
        let skill_ids = self
            .scenario
            .heroes
            .iter()
            .chain(&self.scenario.monsters)
            .chain(self.lifecycle.catalog.values())
            .flat_map(|b| b.skills.iter().map(|s| s.id.as_str()))
            .collect::<BTreeSet<_>>();
        for b in self
            .scenario
            .heroes
            .iter()
            .chain(&self.scenario.monsters)
            .chain(self.lifecycle.catalog.values())
        {
            if let Some(brain) = &b.mechanics.brain {
                for e in &brain.bonus_initiative_desires {
                    if let Some(id) = e["data"]["last_combat_skill_id"].as_str()
                        && !skill_ids.contains(id)
                    {
                        return Err(format!("unresolved bonus last-skill reference {id}"));
                    }
                }
            }
            for reference in dd_data::lifecycle::blueprint_references(b) {
                if !self.lifecycle.catalog.contains_key(&reference) {
                    return Err(format!(
                        "unresolved entity reference {} -> {reference}",
                        b.id
                    ));
                }
            }
        }
        if let Some(plan) = &self.scenario.rules.farm {
            for id in plan.monster_ids() {
                if !self.lifecycle.catalog.contains_key(id) {
                    return Err(format!("unresolved farm pool reference {id}"));
                }
            }
        }
        for (key, b) in &self.lifecycle.catalog {
            if key != &b.id || b.hero {
                return Err("invalid entity catalog key/side".into());
            }
            let mut s = self.scenario.clone();
            s.monsters = vec![b.clone()];
            validate_scenario(&s)?;
        }
        if let Some(ctx) = &self.scenario.rules.context {
            for bp in &ctx.reinforcements {
                if self.lifecycle.catalog.get(&bp.id) != Some(bp) {
                    return Err("missing explicit reinforcement definition".into());
                }
            }
        }
        for wave in &self.lifecycle.waves {
            let mut s = self.scenario.clone();
            s.monsters = wave.clone();
            validate_scenario(&s)?;
            for b in wave {
                if self.lifecycle.catalog.get(&b.id) != Some(b) {
                    return Err("wave definition is absent from entity catalog".into());
                }
            }
        }
        // Reject zero-time spawn/change cycles; ordinary skill self-summons are legal.
        fn visit(
            id: &str,
            catalog: &BTreeMap<String, Blueprint>,
            active: &mut BTreeSet<String>,
            done: &mut BTreeSet<String>,
        ) -> Result<()> {
            if done.contains(id) {
                return Ok(());
            }
            if !active.insert(id.into()) {
                return Err("cyclic immediate spawn lifecycle".into());
            }
            let b = &catalog[id];
            let mut edges = BTreeSet::new();
            for e in &b.mechanics.lifecycle.spawn {
                dd_data::lifecycle::effect_references(e, &mut edges);
            }
            fn kills(e: &dd_data::combat::Effect) -> bool {
                match &e.kind {
                    EffectKind::Lifecycle(l) => matches!(l.op, LifeOp::Kill),
                    EffectKind::Composite(es) => es.iter().any(kills),
                    _ => false,
                }
            }
            if b.mechanics.lifecycle.spawn.iter().any(kills) {
                if let Some(change) = &b.mechanics.lifecycle.death_change {
                    edges.extend(change.choices.iter().map(|c| c.class.clone()));
                }
                for e in &b.mechanics.lifecycle.death_effects {
                    dd_data::lifecycle::effect_references(e, &mut edges);
                }
            }
            for edge in edges {
                visit(&edge, catalog, active, done)?;
            }
            active.remove(id);
            done.insert(id.into());
            Ok(())
        }
        let mut done = BTreeSet::new();
        for id in self.lifecycle.catalog.keys() {
            visit(id, &self.lifecycle.catalog, &mut BTreeSet::new(), &mut done)?;
        }
        Ok(())
    }
    pub(super) fn initialize_entity(&mut self, id: UnitId, events: &mut Vec<Event>) {
        self.initialize_entity_with_hp_policy(id, events, false)
    }
    pub(super) fn initialize_entity_with_hp_policy(
        &mut self,
        id: UnitId,
        events: &mut Vec<Event>,
        clamp_existing_hp: bool,
    ) {
        if let Some(family) = self.units[id].blueprint.mechanics.life_link.as_ref()
            && self.units.iter().enumerate().any(|(other, u)| {
                other
                    >= self
                        .lifecycle
                        .special
                        .farm
                        .as_ref()
                        .map_or(0, |s| s.stage_start)
                    && u.blueprint.mechanics.family == *family
            })
        {
            self.units[id].link_bound = true;
        }
        if self.units[id].mode.is_none() {
            if let Some(m) = self.units[id]
                .blueprint
                .mechanics
                .boss
                .modes
                .iter()
                .find(|m| m.initial)
            {
                self.units[id].mode = Some(m.id.clone());
            }
            let hero = self.units[id].blueprint.mechanics.hero.clone();
            let defaults = hero.modes.iter().filter(|m| m.default).collect::<Vec<_>>();
            if !defaults.is_empty() {
                let mode = if let Some(id) = &hero.initial_mode {
                    hero.modes
                        .iter()
                        .find(|m| &m.id == id)
                        .expect("validated initial mode")
                } else {
                    defaults[if defaults.len() == 1 {
                        0
                    } else {
                        self.rng.below(defaults.len() as u64) as usize
                    }]
                };
                self.units[id].mode = Some(mode.id.clone());
                for effect in &mode.start {
                    self.apply_effect(id, id, effect, events);
                }
            }
        }
        for b in self.units[id].blueprint.mechanics.build.buffs.clone() {
            self.add_buff(id, b, BuffScope::Build, events);
        }
        for (source, buffs) in self.units[id]
            .blueprint
            .mechanics
            .build
            .disease_buffs
            .clone()
        {
            for b in buffs {
                self.add_buff(id, b, BuffScope::Disease, events);
                if let Some(b) = self.units[id].buffs.last_mut() {
                    b.source_disease = Some(source.clone());
                }
            }
        }
        if self.units[id].blueprint.hero {
            if let Some(carry) = self.units[id].blueprint.mechanics.hero.carry.clone() {
                self.units[id].item_uses = carry.status.item_uses.clone();
                self.units[id].burn = carry.status.burn;
                self.units[id].dots = carry
                    .status
                    .dots
                    .iter()
                    .map(|d| Dot {
                        can_crit_heal: d.can_crit_heal,
                        amount: d.amount,
                        remaining: d.remaining,
                        kind: match d.kind {
                            dd_data::hero010::CarryDotKind::Bleed => EffectKind::Bleed(d.amount),
                            dd_data::hero010::CarryDotKind::Blight => EffectKind::Blight(d.amount),
                            dd_data::hero010::CarryDotKind::Horror => EffectKind::Horror(d.amount),
                            dd_data::hero010::CarryDotKind::Regen => EffectKind::Regen(d.amount),
                        },
                    })
                    .collect();
                use dd_data::hero010::CarryResolve;
                self.units[id].stress = carry.stress;
                self.units[id].resolve = match carry.resolve {
                    CarryResolve::Calm => Resolve::Calm,
                    CarryResolve::Afflicted(id) => Resolve::Afflicted(id),
                    CarryResolve::Virtuous(id) => Resolve::Virtuous(id),
                };
                if let Some(t) = self.trait_def(id).cloned() {
                    for b in t.buffs {
                        self.add_buff(id, b, BuffScope::Trait, events);
                    }
                }
                for b in carry.buffs {
                    self.add_buff(id, b.definition, BuffScope::Timed, events);
                    if let Some(active) = self.units[id].buffs.last_mut() {
                        active.remaining = b.remaining;
                    }
                }
            }
            for b in self.units[id]
                .blueprint
                .mechanics
                .hero
                .exploration_buffs
                .clone()
            {
                self.add_buff(id, b.definition, BuffScope::Timed, events);
                if let Some(active) = self.units[id].buffs.last_mut() {
                    active.remaining = b.remaining;
                }
            }
            for buff in self.units[id]
                .blueprint
                .mechanics
                .hero
                .host_passive_buffs
                .clone()
            {
                self.add_buff(id, buff, BuffScope::Build, events);
            }
            for host in self.units[id].blueprint.mechanics.hero.host_buffs.clone() {
                self.add_buff_stacking(id, host.definition, BuffScope::Timed, true, events);
                if let Some(buff) = self.units[id].buffs.last_mut() {
                    buff.host_instance = Some(host.instance);
                }
            }
            let requested = self.units[id].blueprint.mechanics.hero.initial_hp;
            let max = self.units[id].stats().hp;
            if requested.is_some_and(|n| n < 0 || n > max && !clamp_existing_hp) {
                self.lifecycle.failure = Some("invalid initial hero HP for build".into());
            }
            self.units[id].hp = requested.unwrap_or(max).clamp(0, max);
            if self.units[id].hp == 0 {
                for b in self.units[id].blueprint.mechanics.deaths_door.clone() {
                    self.add_buff(id, b, BuffScope::DeathsDoor, events);
                }
            }
        }
        let spec = self.units[id].blueprint.mechanics.lifecycle.clone();
        self.units[id].expires = spec.lifetime.map(|n| self.round.max(1) + n);
        if spec.role == EntityRole::Corpse {
            self.units[id].life = Life::Corpse {
                expires: self.round.max(1) + spec.lifetime.expect("validated corpse lifetime"),
            };
        }
        self.refresh_context(true);
        if self.round > 0 {
            self.prepare_entity_desires(id);
        }
        for e in spec.spawn {
            self.apply_effect(id, id, &e, events);
        }
    }
    pub(super) fn apply_wave_spawn(&mut self, id: UnitId, events: &mut Vec<Event>) {
        for item in self.units[id]
            .blueprint
            .mechanics
            .lifecycle
            .wave_spawn
            .clone()
        {
            let targets = if let Some(family) = item.target_family {
                self.units
                    .iter()
                    .enumerate()
                    .filter_map(|(id, u)| {
                        (u.alive() && u.blueprint.mechanics.family == family).then_some(id)
                    })
                    .collect()
            } else {
                vec![id]
            };
            for target in targets {
                self.apply_effect(id, target, &item.effect, events);
            }
        }
    }
    pub(super) fn lifecycle_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        l: &dd_data::lifecycle::LifecycleEffect,
        allow_dead: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        if l.queued {
            self.lifecycle.jobs.push_back(Job {
                allow_dead,
                actor,
                target,
                op: l.op.clone(),
            });
            true
        } else {
            self.run_life_op(actor, target, &l.op, events)
        }
    }
    fn available_rank(&self, b: &Blueprint, ranks: &[u8], replace: Option<UnitId>) -> Option<u8> {
        let mut rank = 1;
        let mut boundaries = vec![1];
        for id in &self.order[1] {
            if Some(*id) != replace && self.units[*id].present() {
                rank += self.units[*id].size();
                boundaries.push(rank);
            }
        }
        if rank - 1 + b.size > 4 {
            return None;
        }
        boundaries
            .into_iter()
            .find(|r| ranks.is_empty() || ranks.contains(r))
    }
    pub(super) fn change_entity(
        &mut self,
        id: UnitId,
        spec: &ChangeSpec,
        events: &mut Vec<Event>,
    ) -> bool {
        if self.units[id].blueprint.hero {
            return false;
        }
        let occupied = self.order[1]
            .iter()
            .filter(|u| **u != id && self.units[**u].present())
            .map(|u| self.units[*u].size())
            .sum::<u8>();
        let choices = spec
            .choices
            .iter()
            .filter(|c| {
                self.lifecycle
                    .catalog
                    .get(&c.class)
                    .is_some_and(|b| occupied + b.size <= 4)
            })
            .collect::<Vec<_>>();
        if choices.is_empty() {
            return false;
        }
        let Ok(index) = self.weighted_index(&choices.iter().map(|c| c.weight).collect::<Vec<_>>())
        else {
            return false;
        };
        let b = self.lifecycle.catalog[&choices[index].class].clone();
        self.apply_entity_change(id, b, spec, events)
    }
    pub(super) fn apply_entity_change(
        &mut self,
        id: UnitId,
        b: Blueprint,
        spec: &ChangeSpec,
        events: &mut Vec<Event>,
    ) -> bool {
        let old = self.units[id].blueprint.id.clone();
        if b.mechanics.boss.death.previous_hp {
            if self.units[id].previous_form.is_none() {
                self.units[id].previous_form = Some((old.clone(), self.units[id].hp));
            }
        } else {
            self.units[id].previous_form = None;
        }
        if self.units[id].mode.as_ref().is_some_and(|mode| {
            !b.mechanics.hero.modes.iter().any(|m| &m.id == mode)
                && !b.mechanics.boss.modes.iter().any(|m| &m.id == mode)
        }) {
            self.units[id].mode = None;
        }

        if self.units[id].blueprint.mechanics.life_link != b.mechanics.life_link {
            self.units[id].link_bound = false;
        }
        if old != b.id {
            self.lifecycle.bonus_used.retain(|(unit, _)| *unit != id);
        }
        if !self.order[1].contains(&id) {
            let wanted = self.units[id].death_rank.max(1);
            self.order[1].retain(|id| self.units[*id].present());
            let mut rank = 1;
            let mut index = 0;
            for (i, other) in self.order[1].iter().enumerate() {
                if rank <= wanted {
                    index = i;
                }
                rank += self.units[*other].size();
                if rank <= wanted {
                    index = i + 1;
                }
            }
            self.order[1].insert(index, id);
        }
        self.clear_guards(id, true, true, events);
        self.units[id].riposte = None;
        self.units[id].blocks = 0;
        self.units[id].blocks_timer = None;
        let cooldowns = self.units[id]
            .cooldowns
            .iter()
            .map(|(index, n)| (self.units[id].blueprint.skills[*index].id.clone(), *n))
            .collect::<BTreeMap<_, _>>();
        let u = &mut self.units[id];
        u.blueprint = b;
        u.life = Life::Alive;
        u.hp = if spec.reset_hp {
            u.blueprint.stats.hp
        } else {
            u.hp.clamp(1, u.shared_max.unwrap_or(u.blueprint.stats.hp))
        };
        u.skill_remaining = u
            .blueprint
            .skills
            .iter()
            .map(|s| {
                s.per_battle_limit
                    .map(|n| n.saturating_sub(u.used_skills.get(&s.id).copied().unwrap_or(0)))
            })
            .collect();
        u.turn_uses = u
            .blueprint
            .skills
            .iter()
            .enumerate()
            .filter_map(|(index, s)| u.turn_history.get(&s.id).map(|n| (index, *n)))
            .collect();
        u.desire_rolls.clear();
        u.turn_adjust = 0;
        u.next_bonus = 0;
        if spec.reset_buffs {
            u.buffs.clear();
            u.dots.clear();
            u.burn = 0;
            u.stunned = false;
            u.marked = None;
        }
        if spec.clear_cooldowns {
            u.cooldowns = u
                .blueprint
                .skills
                .iter()
                .enumerate()
                .filter(|(_, s)| s.tactical.boss.starting_cooldown > 0)
                .map(|(i, s)| (i, u.turns + s.tactical.boss.starting_cooldown + 1))
                .collect();
        } else {
            u.cooldowns = u
                .blueprint
                .skills
                .iter()
                .enumerate()
                .filter_map(|(index, s)| cooldowns.get(&s.id).map(|n| (index, *n)))
                .collect();
        }
        if spec.clear_initiative {
            self.queue.retain(|u| *u != id);
        }
        events.push(Event::Changed {
            unit: id,
            from: old,
            to: self.units[id].blueprint.id.clone(),
        });
        self.initialize_entity(id, events);
        true
    }
    pub(super) fn summon(
        &mut self,
        actor: UnitId,
        spec: &SummonSpec,
        events: &mut Vec<Event>,
    ) -> bool {
        if self.units[actor].blueprint.hero {
            return false;
        }
        let mut any = false;
        for _ in 0..spec.count {
            let corpses = self.order[1]
                .iter()
                .copied()
                .filter(|id| {
                    matches!(self.units[*id].life, Life::Corpse { .. })
                        && self.units[*id].blueprint.corpse.as_ref().map_or(
                            self.units[*id]
                                .blueprint
                                .mechanics
                                .lifecycle
                                .can_be_summon_rank,
                            |c| c.summonable,
                        )
                })
                .collect::<Vec<_>>();
            let choices = spec
                .choices
                .iter()
                .filter_map(|c| {
                    let b = self.lifecycle.catalog.get(&c.class)?;
                    let count = self
                        .units
                        .iter()
                        .filter(|u| u.alive() && u.blueprint.id == c.class)
                        .count() as u32;
                    if c.limit.is_some_and(|n| count >= n) {
                        return None;
                    }
                    if !spec.replace_corpse
                        && let Some(rank) = self.available_rank(b, &spec.ranks, None)
                    {
                        return Some((c, rank, None));
                    }
                    corpses.iter().find_map(|corpse| {
                        let preferred = if spec.replace_corpse {
                            vec![self.rank(*corpse)]
                        } else {
                            spec.ranks.clone()
                        };
                        self.available_rank(b, &preferred, Some(*corpse))
                            .map(|rank| (c, rank, Some(*corpse)))
                    })
                })
                .collect::<Vec<_>>();
            if choices.is_empty() {
                break;
            }
            let Ok(i) =
                self.weighted_index(&choices.iter().map(|(c, _, _)| c.weight).collect::<Vec<_>>())
            else {
                break;
            };
            let (choice, rank, corpse) = choices[i];
            let b = self.lifecycle.catalog[&choice.class].clone();
            if let Some(dead) = corpse {
                self.retire(dead, events);
            }
            let id = self.units.len();
            let mut u = Unit::fresh(&b);
            u.loot = spec.loot;
            self.units.push(u);
            self.order[1].push(id);
            self.move_to(id, rank);
            events.push(Event::Spawned {
                unit: id,
                class: b.id.clone(),
                rank,
                loot: spec.loot,
            });
            self.initialize_entity(id, events);
            if spec.roll_initiative && self.units[id].alive() {
                let n = self.units[id].action_count();
                self.units[id].round_budget = n;
                for _ in 0..n {
                    self.queue.push_back(id);
                }
            }
            any = true;
        }
        any
    }
    pub(super) fn retire(&mut self, id: UnitId, events: &mut Vec<Event>) {
        if !self.units[id].present() {
            return;
        }
        if let Some(hp) = self
            .lifecycle
            .special
            .pools
            .iter()
            .find(|p| p.hp > 0 && p.members.contains(&id))
            .map(|p| p.hp)
        {
            self.shared_damage(id, hp, false, events);
            return;
        }
        self.units[id].death_rank = self.rank(id);
        self.units[id].hp = 0;
        self.units[id].life = Life::Dead;
        self.units[id].dots.clear();
        self.units[id].parasites.clear();
        self.units[id].buffs.clear();
        self.units[id].stunned = false;
        self.units[id].marked = None;
        self.units[id].expires = None;
        self.queue.retain(|x| *x != id);
        events.push(Event::Death(id));
        self.cleanup_tactics(events);
        self.queue.retain(|id| self.units[*id].active());
        self.refresh_context(true);
    }
    fn run_life_op(
        &mut self,
        actor: UnitId,
        target: UnitId,
        op: &LifeOp,
        events: &mut Vec<Event>,
    ) -> bool {
        match op {
            LifeOp::Boss(op) => self.boss_operation(actor, target, op, events),
            LifeOp::StealDots { kind } => {
                let dots = self.units[target]
                    .dots
                    .iter()
                    .filter(|d| {
                        matches!(
                            (&d.kind, kind.as_str()),
                            (EffectKind::Bleed(_), "hp_dot_bleed")
                                | (EffectKind::Blight(_), "hp_dot_poison")
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                self.units[target].dots.retain(|d| !dots.contains(d));
                self.units[actor].dots.extend(dots);
                true
            }
            LifeOp::Light { amount } => {
                let [min, max] = self.torch_bounds();
                let Some(light) = self.scenario.rules.lighting.as_mut() else {
                    self.lifecycle.failure = Some("missing battle lighting context".into());
                    return false;
                };
                let before = light.torch;
                light.torch =
                    (i32::from(before) + amount).clamp(i32::from(min), i32::from(max)) as u8;
                let after = light.torch;
                events.push(Event::Light { before, after });
                self.refresh_context(false);
                true
            }
            LifeOp::Summon(s) => self.summon(actor, s, events),
            LifeOp::Change(s) => self.change_entity(target, s, events),
            LifeOp::Phase {
                change,
                effects,
                carry_min,
                overkill,
            } => {
                if !self.change_entity(target, change, events) {
                    return false;
                }
                if let Some(min) = carry_min {
                    let max = self.units[target].stats().hp;
                    self.units[target].hp = (max - overkill)
                        .max((f64::from(max) * min).round() as i32)
                        .clamp(1, max);
                }
                for e in effects {
                    self.apply_effect(target, target, e, events);
                }
                true
            }
            LifeOp::Kill => {
                let alive = self.units[target].alive();
                // Original Crow escape uses kill_self_queued. In the scoped
                // retention contract this is a departure, not a player kill.
                let escaped = actor == target
                    && self.lifecycle.last_skill.as_deref() == Some("escape")
                    && crate::execution025::shrieker_escape_source(self, target);
                self.retire(target, events);
                if alive && escaped {
                    self.lifecycle
                        .special
                        .escaped
                        .insert(target, self.units[target].source_id.clone());
                    events.push(Event::Escaped {
                        id: target,
                        class: self.units[target].source_id.clone(),
                    });
                }
                if alive
                    && !escaped
                    && !self.units[target].blueprint.hero
                    && self.units[target]
                        .blueprint
                        .mechanics
                        .lifecycle
                        .death_change
                        .is_none()
                {
                    self.lifecycle.special.killed.insert(
                        target,
                        (
                            self.units[target].source_id.clone(),
                            self.units[target].blueprint.id.clone(),
                        ),
                    );
                }
                if alive && !escaped {
                    if self.units[target].blueprint.hero {
                        self.death_stress(target, events);
                        self.hero_death(target, events);
                    }
                    self.schedule_death(target, false, events);
                }
                true
            }
            LifeOp::Initiative { amount, next_round } => {
                if !self.units[target].alive() {
                    return false;
                }
                if *next_round {
                    if *amount < 0 || i32::from(self.units[target].next_bonus) + amount > 4 {
                        return false;
                    }
                    self.units[target].next_bonus += *amount as u8;
                } else {
                    let n = i32::from(self.units[target].action_count()) + amount;
                    if !(0..=4).contains(&n) {
                        return false;
                    }
                    self.units[target].turn_adjust += *amount as i8;
                    let total = self.units[target].action_count() + self.units[target].round_extra;
                    let spent = self.units[target].round_actions as u8;
                    self.units[target].round_budget = total.max(spent);
                    let wanted = usize::from(total.saturating_sub(spent));
                    let pending = self.queue.iter().filter(|id| **id == target).count();
                    if wanted > pending {
                        for _ in 0..wanted - pending {
                            self.queue.push_back(target);
                        }
                    } else {
                        let mut remaining = pending - wanted;
                        self.queue.retain(|id| {
                            if *id == target && remaining > 0 {
                                remaining -= 1;
                                false
                            } else {
                                true
                            }
                        });
                    }
                }
                events.push(Event::InitiativeChanged {
                    unit: target,
                    amount: *amount,
                    next_round: *next_round,
                });
                true
            }
        }
    }
    pub(super) fn schedule_death(&mut self, id: UnitId, crit: bool, events: &mut Vec<Event>) {
        let spec = self.units[id].blueprint.mechanics.lifecycle.clone();
        if let Some(brain) = self.units[id].blueprint.mechanics.brain.clone() {
            for d in brain
                .bonus_initiative_desires
                .iter()
                .filter(|d| d["type"] == "death")
            {
                let name = d["data"]["combat_skill_id_override"].as_str().unwrap();
                let skill = self.units[id]
                    .blueprint
                    .skills
                    .iter()
                    .find(|s| s.id == name)
                    .unwrap()
                    .clone();
                events.push(Event::BonusAction {
                    actor: id,
                    skill: name.into(),
                });
                for e in skill.effects {
                    self.apply_effect(id, id, &e, events);
                }
            }
        }
        if let Some((family, amount)) = self.units[id].blueprint.mechanics.boss.death_damage.clone()
        {
            let targets = self
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.alive() && u.blueprint.mechanics.family == family)
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            for target in targets {
                self.damage_unrestricted(target, amount, false, events);
            }
        }
        if let Some(mut c) = spec.death_change
            && (!crit || spec.death_on_crit)
        {
            let death = self.units[id].blueprint.mechanics.boss.death.clone();
            if death.previous_hp {
                let Some((_, hp)) = self.units[id].previous_form.take() else {
                    self.lifecycle.failure = Some("missing previous form HP".into());
                    return;
                };
                self.units[id].hp = hp;
                c.reset_hp = false;
            }
            self.lifecycle.jobs.push_back(Job {
                allow_dead: true,
                actor: id,
                target: id,
                op: LifeOp::Phase {
                    change: c,
                    effects: death.change_effects,
                    carry_min: death.carry_min,
                    overkill: self.units[id].phase_overkill,
                },
            });
        }
        for e in spec.death_effects {
            self.apply_effect(id, id, &e, events);
        }
        self.queue.retain(|x| *x != id);
    }
    pub(super) fn flush_lifecycle(&mut self, events: &mut Vec<Event>) {
        self.flush_effects(events);
        self.hero_hp_reactions(events);
        for id in 0..self.units.len() {
            if let Some(family) = self.units[id].blueprint.mechanics.life_link.as_ref()
                && self.units.iter().enumerate().any(|(id, u)| {
                    id >= self
                        .lifecycle
                        .special
                        .farm
                        .as_ref()
                        .map_or(0, |s| s.stage_start)
                        && u.blueprint.mechanics.family == *family
                })
            {
                self.units[id].link_bound = true;
            }
        }
        self.flush_effects(events);
        let mut job_count = 0;
        while !self.lifecycle.jobs.is_empty() || !self.effect_jobs.is_empty() {
            self.flush_effects(events);
            let Some(job) = self.lifecycle.jobs.pop_front() else {
                break;
            };
            job_count += 1;
            if job_count > traits::MAX_REACTIONS_PER_ACTION {
                self.lifecycle.failure = Some("lifecycle resolution bound exceeded".into());
                self.lifecycle.jobs.clear();
                break;
            }
            let applied = (job.allow_dead || self.units[job.target].alive())
                && self.run_life_op(job.actor, job.target, &job.op, events);
            events.push(Event::LifecycleResolved {
                actor: job.actor,
                target: job.target,
                applied,
            });
        }
        // An absent master never creates a link. Once a family member actually
        // exists, its last death removes all linked dependants in finite passes.
        loop {
            let linked = self
                .units
                .iter()
                .enumerate()
                .filter_map(|(id, u)| {
                    let family = u.blueprint.mechanics.life_link.as_ref()?;
                    (u.alive()
                        && u.link_bound
                        && !self.units.iter().enumerate().any(|(n, v)| {
                            n >= self
                                .lifecycle
                                .special
                                .farm
                                .as_ref()
                                .map_or(0, |s| s.stage_start)
                                && v.alive()
                                && v.blueprint.mechanics.family == *family
                        }))
                    .then_some(id)
                })
                .collect::<Vec<_>>();
            if linked.is_empty() {
                break;
            }
            for id in linked {
                self.units[id].loot &= self.units[id].blueprint.mechanics.lifecycle.link_loot;
                self.retire(id, events);
            }
        }
        if self.pending_reactions.is_empty()
            && self.units.iter().all(|u| u.attack_context.is_none())
            && !self.units.iter().any(|u| u.alive() && !u.blueprint.hero)
            && self.lifecycle.wave >= self.lifecycle.waves.len()
            && self.scenario.rules.farm.is_none()
            && !(self.scenario.monsters.is_empty()
                && dd_data::mechanisms023::enabled(
                    &self.scenario.rules.content,
                    dd_data::mechanisms023::FIXES,
                ))
        {
            for id in 0..self.scenario.heroes.len() {
                if self.units[id].alive() {
                    self.mode_forced(id, true, events);
                }
            }
        }
        if self.pending_reactions.is_empty()
            && self.units.iter().all(|u| u.attack_context.is_none())
            && self.units.iter().any(|u| u.alive() && u.blueprint.hero)
            && !self.units.iter().any(|u| u.alive() && !u.blueprint.hero)
            && self.lifecycle.wave >= self.lifecycle.waves.len()
            && self.scenario.rules.farm.is_none()
            && !(self.scenario.monsters.is_empty()
                && dd_data::mechanisms023::enabled(
                    &self.scenario.rules.content,
                    dd_data::mechanisms023::FIXES,
                ))
        {
            self.trinket_battle_end(events);
        }
        self.flush_effects(events);
        let mut job_count = 0;
        while !self.lifecycle.jobs.is_empty() || !self.effect_jobs.is_empty() {
            self.flush_effects(events);
            let Some(job) = self.lifecycle.jobs.pop_front() else {
                break;
            };
            job_count += 1;
            if job_count > traits::MAX_REACTIONS_PER_ACTION {
                self.lifecycle.failure = Some("lifecycle resolution bound exceeded".into());
                self.lifecycle.jobs.clear();
                break;
            }
            let applied = (job.allow_dead || self.units[job.target].alive())
                && self.run_life_op(job.actor, job.target, &job.op, events);
            events.push(Event::LifecycleResolved {
                actor: job.actor,
                target: job.target,
                applied,
            });
        }

        self.settle_flashback(events);
        self.cleanup_captives(events);
        self.cleanup_control(events);
        self.farm_settle(events);
        self.farm_group_end(events);
        self.cleanup_tactics(events);
        self.queue.retain(|id| self.units[*id].active());
        self.refresh_context(true);
    }
    /// Host revive uses the saved blueprint and original HP; it cannot revive heroes.
    pub fn revive_entity(&mut self, id: UnitId) -> Result<Vec<Event>> {
        if self.outcome.is_some()
            || id >= self.units.len()
            || self.units[id].alive()
            || self.units[id].blueprint.hero
            || self.units[id].blueprint.mechanics.lifecycle.role == EntityRole::Corpse
        {
            return Err("invalid revive target/boundary".into());
        }
        let b = self.units[id].blueprint.clone();
        let rank = self
            .available_rank(&b, &[], self.units[id].present().then_some(id))
            .ok_or("revive exceeds formation capacity")?;
        let mut candidate = self.clone();
        let mut events = vec![];
        candidate.clear_guards(id, true, true, &mut events);
        candidate.units[id] = Unit::fresh(&b);
        candidate.units[id].loot = self.units[id].loot;
        candidate.order[1].retain(|x| *x != id);
        candidate.order[1].push(id);
        candidate.move_to(id, rank);
        events.push(Event::Revived(id));
        candidate.initialize_entity(id, &mut events);
        candidate.flush_lifecycle(&mut events);
        *self = candidate;
        Ok(events)
    }
    pub fn transform_entity(&mut self, id: UnitId, spec: ChangeSpec) -> Result<Vec<Event>> {
        LifeOp::Change(spec.clone()).validate()?;
        if self.outcome.is_some()
            || id >= self.units.len()
            || !self.units[id].present()
            || self.units[id].blueprint.hero
            || spec
                .choices
                .iter()
                .any(|c| !self.lifecycle.catalog.contains_key(&c.class))
        {
            return Err("invalid transform target/reference/boundary".into());
        }
        let mut candidate = self.clone();
        let mut events = vec![];
        if !candidate.change_entity(id, &spec, &mut events) {
            return Err("transform exceeds formation capacity or has no eligible choice".into());
        }
        candidate.flush_lifecycle(&mut events);
        candidate.validate_shared_health()?;
        candidate.validate_captives()?;
        candidate.validate_control()?;
        if candidate.actor == Some(id)
            && (spec.clear_initiative
                || !candidate.units[id].alive()
                || candidate.units[id].action_count() == 0)
        {
            candidate.actor = None;
            candidate.advance(&mut events);
        }
        *self = candidate;
        Ok(events)
    }
    /// Explicit host phase effects for future encounter controllers; never a policy action.
    pub fn apply_encounter_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        e: dd_data::combat::Effect,
    ) -> Result<Vec<Event>> {
        validate_effect(&e)?;
        if self.outcome.is_some()
            || actor >= self.units.len()
            || target >= self.units.len()
            || !self.units[actor].present()
            || !self.units[target].present()
            || !e.conditions.is_empty()
        {
            return Err("invalid encounter effect boundary/target".into());
        }
        let mut references = BTreeSet::new();
        dd_data::lifecycle::effect_references(&e, &mut references);
        if references
            .iter()
            .any(|id| !self.lifecycle.catalog.contains_key(id))
        {
            return Err("unresolved encounter effect entity reference".into());
        }
        let mut candidate = self.clone();
        let mut events = vec![];
        candidate.apply_effect(actor, target, &e, &mut events);
        candidate.flush_lifecycle(&mut events);
        candidate.flush_reactions(&mut events);
        if let Some(error) = candidate.lifecycle.failure.take() {
            return Err(error);
        }
        let new_wave = candidate.next_wave(&mut events);
        if candidate.terminal() {
            candidate.actor = None;
        } else if new_wave
            || candidate
                .actor
                .is_none_or(|id| !candidate.units[id].active() || candidate.units[id].stunned)
        {
            candidate.actor = None;
            candidate.advance(&mut events);
        }
        *self = candidate;
        Ok(events)
    }
    pub(super) fn next_wave(&mut self, events: &mut Vec<Event>) -> bool {
        if self.lifecycle.wave >= self.lifecycle.waves.len()
            || !self.units.iter().any(|u| u.alive() && u.blueprint.hero)
            || self.units.iter().any(|u| u.alive() && !u.blueprint.hero)
        {
            return false;
        }
        for id in self.order[1].clone() {
            self.retire(id, events);
        }
        self.order[1].clear();
        self.queue.clear();
        let wave = self.lifecycle.waves[self.lifecycle.wave].clone();
        self.lifecycle.wave += 1;
        self.lifecycle.new_wave = true;
        self.lifecycle.bonus_used.clear();
        self.lifecycle.zones.clear();
        for unit in self
            .units
            .iter_mut()
            .filter(|u| u.alive() && u.blueprint.hero)
        {
            for (slot, skill) in unit.blueprint.skills.iter().enumerate() {
                if skill.tactical.refresh_each_wave {
                    unit.skill_remaining[slot] = skill.per_battle_limit;
                    unit.used_skills.remove(&skill.id);
                }
            }
        }
        for b in wave {
            let id = self.units.len();
            self.units.push(Unit::fresh(&b));
            self.order[1].push(id);
            events.push(Event::Spawned {
                unit: id,
                class: b.id.clone(),
                rank: self.rank(id),
                loot: b.mechanics.lifecycle.loot,
            });
            self.initialize_entity(id, events);
            self.apply_wave_spawn(id, events);
        }
        if let Err(error) = self.initialize_shared_health() {
            self.lifecycle.failure = Some(error);
            return false;
        }
        self.flush_lifecycle(events);
        events.push(Event::Wave(self.lifecycle.wave + 1));
        true
    }
}
