use super::*;
use dd_data::special::BossOp;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Parasite {
    pub source: UnitId,
    pub definition: dd_data::special::ActorDot,
    pub stage: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Immobilized {
    pub until: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Captive {
    pub captor: UnitId,
    pub removed: bool,
    pub previous_hp: i32,
    pub damage: i32,
    pub resolve_epoch: u32,
}
impl Battle {
    pub(super) fn boss_operation(
        &mut self,
        actor: UnitId,
        target: UnitId,
        op: &BossOp,
        events: &mut Vec<Event>,
    ) -> bool {
        match op {
            BossOp::PartyStackBuffs(buffs) => {
                let targets = self
                    .units
                    .iter()
                    .enumerate()
                    .filter(|(_, u)| u.alive() && u.friendly() == self.units[target].friendly())
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>();
                for id in targets {
                    for b in buffs {
                        self.add_buff_stacking(id, b.clone(), BuffScope::Timed, true, events);
                    }
                }
                true
            }
            BossOp::Control { turns } => self.control_hero(actor, target, *turns, events),
            BossOp::ActorDot(definition) => {
                if !self.units[target].alive() {
                    return false;
                }
                if self.units[target]
                    .parasites
                    .iter()
                    .any(|p| p.definition.id == definition.id)
                {
                    return false;
                }
                self.units[target].parasites.push(Parasite {
                    source: actor,
                    definition: definition.clone(),
                    stage: 0,
                });
                true
            }
            BossOp::Presentation => true,
            BossOp::ShuffleDot { turns } => {
                self.units[target].shuffle_remaining = *turns;
                true
            }
            BossOp::Damage(n) => {
                self.damage(target, *n, false, events);
                true
            }
            BossOp::InstantShuffle => {
                self.displace(target, &EffectKind::Shuffle, events);
                true
            }
            BossOp::Capture {
                remove,
                virtue_block,
            } => {
                if matches!(self.units[target].resolve, Resolve::Virtuous(_))
                    && self.rng.roll(*virtue_block)
                {
                    return false;
                }
                self.capture_hero(actor, target, *remove, events)
            }
            BossOp::Immobilize { enable, duration } => {
                self.units[target].immobilized = if *enable {
                    Some(Immobilized {
                        until: duration.map(|d| self.units[target].turns + d),
                    })
                } else {
                    None
                };
                true
            }
            BossOp::RankTarget => {
                let rs = self.occupied_ranks(target).collect::<Vec<_>>();
                let selected = self
                    .lifecycle
                    .special
                    .marked_ranks
                    .entry(actor)
                    .or_default();
                for r in rs {
                    if !selected.contains(&r) {
                        selected.push(r);
                    }
                }
                true
            }
            BossOp::ClearRanks(rs) => {
                if let Some(selected) = self.lifecycle.special.marked_ranks.get_mut(&target) {
                    selected.retain(|r| !rs.contains(r));
                }
                true
            }
            BossOp::Bark(id) => {
                events.push(Event::Bark {
                    actor: target,
                    id: id.clone(),
                });
                true
            }
            BossOp::StealSkillBuffs => {
                let buffs = self.units[target]
                    .buffs
                    .iter()
                    .filter(|b| b.skill_source)
                    .cloned()
                    .collect::<Vec<_>>();
                self.units[target].buffs.retain(|b| !buffs.contains(b));
                for b in buffs {
                    let previous = self.units[actor].buffs.len();
                    self.add_buff(actor, b.definition, b.scope, events);
                    if self.units[actor].buffs.len() > previous {
                        let stolen = self.units[actor].buffs.last_mut().unwrap();
                        stolen.remaining = b.remaining;
                        stolen.skill_source = b.skill_source;
                        stolen.battle_persistent = b.battle_persistent;
                    }
                }
                true
            }
        }
    }
    pub(super) fn capture_hero(
        &mut self,
        actor: UnitId,
        target: UnitId,
        removed: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        if !self.units[actor].alive()
            || !self.units[target].friendly()
            || !self.units[target].active()
            || self.units[target].captive.is_some()
            || self.units[target].hp == 0
            || self
                .units
                .iter()
                .filter(|u| u.active() && u.friendly())
                .count()
                < 2
        {
            return false;
        }
        let family = self.units[actor].blueprint.mechanics.family.clone();
        let candidates = self
            .units
            .iter()
            .enumerate()
            .filter_map(|(id, u)| {
                u.blueprint
                    .mechanics
                    .boss
                    .empty
                    .as_ref()
                    .filter(|c| u.active() && c.performer == family)
                    .map(|c| (id, c.clone()))
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            if candidates.len() > 1 {
                self.lifecycle.failure = Some("ambiguous empty captor".into());
            }
            return false;
        }
        let (captor, spec) = candidates[0].clone();
        let next = self.lifecycle.catalog[&spec.full].clone();
        let hp = self.units[captor].hp;
        let previous_hp = if spec.count_damage {
            hp - hp.min(next.stats.hp)
        } else {
            hp
        };
        let change = dd_data::lifecycle::ChangeSpec {
            choices: vec![],
            reset_hp: spec.reset_hp,
            reset_buffs: false,
            clear_initiative: true,
            clear_cooldowns: false,
        };
        if !self.apply_entity_change(captor, next, &change, events) {
            return false;
        }
        self.clear_guards(target, true, true, events);
        self.units[target].captive = Some(Captive {
            captor,
            removed,
            previous_hp,
            damage: 0,
            resolve_epoch: self.units[target].resolve_epoch,
        });
        if removed {
            self.queue.retain(|id| *id != target);
            self.order[0].retain(|id| *id != target);
        }
        for e in spec.effects {
            self.apply_effect(captor, target, &e, events);
        }
        events.push(Event::Captured {
            hero: target,
            captor,
            removed,
        });
        self.refresh_context(true);
        true
    }
    fn release_captive(&mut self, hero: UnitId, events: &mut Vec<Event>) {
        let Some(c) = self.units[hero].captive.take() else {
            return;
        };
        let Some(spec) = self.units[c.captor].blueprint.mechanics.boss.full.clone() else {
            self.lifecycle.failure = Some("captive has no full captor".into());
            return;
        };
        let alive = self.units[c.captor].alive();
        if self.units[hero].alive() && c.removed && !self.order[0].contains(&hero) {
            self.order[0].retain(|id| self.units[*id].present());
            self.order[0].insert(0, hero);
        }
        for e in spec.effects.clone() {
            self.apply_effect(c.captor, hero, &e, events);
        }
        self.units[hero].immobilized = None;
        events.push(Event::Released {
            hero,
            captor: c.captor,
        });
        // A linked captor is removed with its master. It must not resurrect itself.
        let linked = self.units[c.captor]
            .blueprint
            .mechanics
            .life_link
            .as_ref()
            .is_none_or(|f| {
                self.units
                    .iter()
                    .any(|u| u.alive() && u.blueprint.mechanics.family == *f)
            });
        if linked && (alive || spec.switch_on_death) {
            let hp = self.units[c.captor].hp;
            let next = self.lifecycle.catalog[&spec.empty].clone();
            let change = dd_data::lifecycle::ChangeSpec {
                choices: vec![],
                reset_hp: spec.reset_hp,
                reset_buffs: false,
                clear_initiative: true,
                clear_cooldowns: false,
            };
            self.apply_entity_change(c.captor, next, &change, events);
            if spec.previous_hp {
                let restored = c
                    .previous_hp
                    .saturating_add(if spec.add_hp { hp } else { 0 })
                    .min(self.units[c.captor].stats().hp);
                if restored <= 0 {
                    self.retire(c.captor, events);
                } else {
                    self.units[c.captor].hp = restored;
                }
            }
        }
        self.refresh_context(true);
    }
    pub(super) fn cleanup_captives(&mut self, events: &mut Vec<Event>) {
        let release = self
            .units
            .iter()
            .enumerate()
            .filter_map(|(id, u)| {
                let c = u.captive.as_ref()?;
                let captor = &self.units[c.captor];
                let spec = captor.blueprint.mechanics.boss.full.as_ref()?;
                ((!u.alive())
                    || (!captor.alive() && spec.release_on_death)
                    || (spec.release_on_door && u.hp == 0)
                    || (spec.release_on_affliction && u.resolve_epoch != c.resolve_epoch))
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        for id in release {
            self.release_captive(id, events);
        }
    }
    pub(super) fn capture_action_tick(&mut self, events: &mut Vec<Event>) {
        self.cleanup_captives(events);
        let prisoners = self
            .units
            .iter()
            .enumerate()
            .filter_map(|(id, u)| u.captive.as_ref().map(|c| (id, c.captor)))
            .collect::<Vec<_>>();
        for (hero, captor) in prisoners {
            let Some(spec) = self.units[captor].blueprint.mechanics.boss.full.clone() else {
                continue;
            };
            let raw = f64::from(self.units[hero].stats().hp) * spec.damage;
            let amount = if self.rounding023() {
                self.damage_quant023(captor, hero, raw)
            } else {
                raw.round().clamp(0., 1e6) as i32
            };
            self.damage(hero, amount, false, events);
            self.stress_change(hero, spec.stress, events);
        }
        let companions = self
            .units
            .iter()
            .enumerate()
            .filter_map(|(id, u)| {
                u.blueprint
                    .mechanics
                    .boss
                    .companion
                    .as_ref()
                    .filter(|c| {
                        u.alive()
                            && c.heal > 0.
                            && self.units.iter().any(|other| {
                                other.alive() && other.blueprint.mechanics.family == c.family
                            })
                    })
                    .map(|c| (id, c.heal))
            })
            .collect::<Vec<_>>();
        for (id, p) in companions {
            let raw = f64::from(self.units[id].stats().hp) * p;
            if self.rounding023() && self.units[id].blueprint.hero {
                self.heal_float023(id, raw, events);
            } else {
                self.heal(id, raw.round() as i32, events);
            }
        }
        self.cleanup_captives(events);
    }
    pub(super) fn validate_captives(&self) -> Result<()> {
        let mut captors = std::collections::BTreeSet::new();
        for (id, u) in self.units.iter().enumerate() {
            if u.parasites.len() > 64 || (!u.alive() && !u.parasites.is_empty()) {
                return Err("invalid checkpoint parasites".into());
            }
            for p in &u.parasites {
                p.definition.validate()?;
                if p.source >= self.units.len() || p.stage >= p.definition.stages.len() {
                    return Err("invalid checkpoint parasite source/stage".into());
                }
                for stage in &p.definition.stages {
                    for e in stage.completion.iter().chain(&stage.increment) {
                        validate_effect(e)?;
                    }
                }
            }
            if let Some(c) = &u.captive {
                let full = self
                    .units
                    .get(c.captor)
                    .and_then(|c| c.blueprint.mechanics.boss.full.as_ref())
                    .ok_or("invalid checkpoint captor")?;
                if !u.blueprint.hero
                    || !u.alive()
                    || !captors.insert(c.captor)
                    || c.damage < 0
                    || c.damage > 1000000
                    || c.previous_hp < 0
                    || c.resolve_epoch > u.resolve_epoch
                    || !self.units[c.captor].alive()
                    || (full.release_on_door && u.hp == 0)
                    || (c.removed && self.order[0].contains(&id))
                    || (!c.removed && !self.order[0].contains(&id))
                {
                    return Err("invalid checkpoint captive relationship".into());
                }
            }
            if u.immobilized.as_ref().is_some_and(|n| {
                n.until
                    .is_some_and(|until| until < u.turns || until > u.turns + 10000)
            }) {
                return Err("invalid checkpoint immobilize clock".into());
            }
        }
        if self.units.iter().enumerate().any(|(id, u)| {
            u.alive() && u.blueprint.mechanics.boss.full.is_some() && !captors.contains(&id)
        }) {
            return Err("full captor has no captive relationship".into());
        }
        if self.lifecycle.special.marked_ranks.iter().any(|(id, rs)| {
            *id >= self.units.len()
                || rs.iter().any(|r| !(1..=4).contains(r))
                || rs.iter().collect::<std::collections::BTreeSet<_>>().len() != rs.len()
        }) {
            return Err("invalid checkpoint rank telegraph".into());
        }
        Ok(())
    }
}

impl Battle {
    pub(super) fn companion_modifiers(&self, id: UnitId) -> Vec<Buff> {
        let u = &self.units[id];
        u.blueprint
            .mechanics
            .boss
            .companion
            .as_ref()
            .filter(|c| {
                u.alive()
                    && self
                        .units
                        .iter()
                        .any(|other| other.alive() && other.blueprint.mechanics.family == c.family)
            })
            .map(|c| c.buffs.clone())
            .unwrap_or_default()
            .into_iter()
            .chain(self.farm_buffs(id))
            .chain(if u.blueprint.hero {
                self.scenario
                    .rules
                    .bloodlight
                    .as_ref()
                    .map(|b| b.heroes.clone())
                    .unwrap_or_default()
            } else {
                vec![]
            })
            .collect()
    }
    pub(super) fn boss_hit_reactions(
        &mut self,
        attacker: UnitId,
        target: UnitId,
        was_alive: bool,
        events: &mut Vec<Event>,
    ) {
        let spec = self.units[target]
            .blueprint
            .mechanics
            .boss
            .reactions
            .clone();
        if was_alive {
            for e in spec.hit_performer {
                self.apply_effect(target, attacker, &e, events);
            }
        }
        if was_alive && !self.units[target].alive() {
            if self.units[attacker].blueprint.hero {
                for e in spec.killed_by_hero {
                    self.apply_effect(target, attacker, &e, events);
                }
            }
            let others = self
                .units
                .iter()
                .enumerate()
                .filter(|(id, u)| {
                    *id != target
                        && u.alive()
                        && u.blueprint.hero == self.units[target].blueprint.hero
                })
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            for id in others {
                for e in &spec.killed_others {
                    self.apply_effect(target, id, e, events);
                }
            }
        }
    }
}

impl Battle {
    pub(super) fn parasite_attack_tick(&mut self, id: UnitId, events: &mut Vec<Event>) {
        let parasites = std::mem::take(&mut self.units[id].parasites);
        for mut p in parasites {
            if !self.units[id].alive() {
                break;
            }
            let stage = &p.definition.stages[p.stage];
            let completed = self.rng.roll(stage.chance);
            for e in if completed {
                &stage.completion
            } else {
                &stage.increment
            } {
                self.apply_effect(p.source, id, e, events);
            }
            if !completed && self.units[id].alive() {
                p.stage += 1;
                self.units[id].parasites.push(p);
            }
        }
    }
}

impl Battle {
    pub(super) fn validate_environment_supplies(&self) -> Result<()> {
        fn stack_buffs(e: &dd_data::combat::Effect) -> bool {
            match &e.kind {
                EffectKind::Boss(BossOp::PartyStackBuffs(_)) => true,
                EffectKind::Composite(es) => es.iter().any(stack_buffs),
                _ => false,
            }
        }
        for s in &self.supplies {
            if let Some(blood) = self
                .scenario
                .rules
                .bloodlight
                .as_ref()
                .filter(|_| s.definition.id == "torch")
            {
                if s.definition.host_candidate.is_some() || s.definition.effects.len() != 1 {
                    return Err("bloodlight requires its original torch item override".into());
                }
                let expected = &s.definition.effects[0];
                if !matches!(&expected.kind,EffectKind::Boss(BossOp::PartyStackBuffs(bs))if bs==&blood.torch_buffs)
                {
                    return Err("bloodlight torch disagrees with source buffs".into());
                }
            } else if s.definition.effects.iter().any(stack_buffs) {
                return Err("bloodlight torch requires explicit bloodlight battle context".into());
            }
        }
        Ok(())
    }
}
