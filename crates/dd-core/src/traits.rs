//! Candidate trait event interpreter. Primary actions create reaction jobs;
//! reaction effects never create synthetic hit/move stimuli. This gives an
//! acyclic event graph rather than recursive damage/pressure callbacks.
use super::*;
use dd_data::mechanics::{ReactionKind, TraitDef, TurnKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super) struct ReactionJob {
    reactor: UnitId,
    trait_id: String,
    resolve_epoch: u32,
    kind: ReactionKind,
    subject: UnitId,
}
pub(super) const MAX_REACTIONS_PER_ACTION: usize = 1024;

impl Battle {
    pub(super) fn displace(&mut self, target: UnitId, kind: &EffectKind, events: &mut Vec<Event>) {
        let ranks = self.placement_ranks(target);
        let before = self.rank(target);
        let rank = match kind {
            EffectKind::Push(n) => self.displacement_rank(target, before.saturating_add(*n)),
            EffectKind::Pull(n) => self.displacement_rank(target, before.saturating_sub(*n).max(1)),
            EffectKind::Shuffle if ranks.len() > 1 => {
                // Uniform over complete block boundaries, including corpses.
                let others = ranks
                    .iter()
                    .copied()
                    .filter(|r| *r != before)
                    .collect::<Vec<_>>();
                others[self.rng.below(others.len() as u64) as usize]
            }
            EffectKind::Shuffle => before,
            _ => unreachable!("displacement operation"),
        };
        if rank == before {
            events.push(Event::MoveSkipped {
                actor: target,
                reason: if ranks.len() == 1 {
                    MoveFailure::SingleUnit
                } else {
                    MoveFailure::Boundary
                },
            });
        } else {
            self.forced_move(target, rank, events);
        }
    }
    pub(super) fn trait_def(&self, actor: UnitId) -> Option<&TraitDef> {
        let name = match &self.units[actor].resolve {
            Resolve::Calm => return None,
            Resolve::Afflicted(n) | Resolve::Virtuous(n) => n,
        };
        self.scenario
            .rules
            .globals
            .as_ref()
            .unwrap()
            .traits
            .iter()
            .chain(&self.units[actor].blueprint.mechanics.hero.override_defs)
            .find(|t| t.id == *name)
    }
    fn effect_for_trait(&self, actor: UnitId, name: &str) -> dd_data::combat::Effect {
        // All active references are checked by validate_scenario, also on restore.
        self.units[actor].blueprint.mechanics.trait_effects[name].clone()
    }
    pub(super) fn reaction_block(
        &mut self,
        source: UnitId,
        target: UnitId,
        kind: ReactionKind,
        effect: Option<&str>,
        events: &mut Vec<Event>,
    ) -> bool {
        self.reaction_block_mode(source, target, kind, effect, events, false)
    }
    pub(super) fn reaction_block_at_camp(
        &mut self,
        source: UnitId,
        target: UnitId,
        kind: ReactionKind,
        events: &mut Vec<Event>,
    ) -> bool {
        self.reaction_block_mode(
            source,
            target,
            kind,
            None,
            events,
            matches!(
                kind,
                ReactionKind::BlockCampingMeal | ReactionKind::BlockCampingSkillPerformer
            ),
        )
    }
    fn reaction_block_mode(
        &mut self,
        source: UnitId,
        target: UnitId,
        kind: ReactionKind,
        effect: Option<&str>,
        events: &mut Vec<Event>,
        allow_self: bool,
    ) -> bool {
        if !self.units[target].alive() {
            return false;
        }
        // Named effect immunity also applies to self effects; other refusal gates
        // only apply to assistance requested by somebody else, except movement.
        if source == target
            && !allow_self
            && !matches!(
                kind,
                ReactionKind::BlockEffect
                    | ReactionKind::BlockMove
                    | ReactionKind::BlockCombatRetreat
            )
        {
            return false;
        }
        let reaction = self
            .trait_def(target)
            .and_then(|t| t.reactions.iter().find(|r| r.id == kind))
            .cloned();
        let Some(r) = reaction else {
            return false;
        };
        if kind == ReactionKind::BlockEffect && effect != Some(r.data.effect.as_str()) {
            return false;
        }
        if r.chance <= 0. || !self.rng.roll(r.chance) {
            return false;
        }
        events.push(Event::Refused {
            actor: source,
            target,
            kind,
            effect: effect.map(str::to_owned),
        });
        true
    }
    pub(super) fn queue_reaction(&mut self, reactor: UnitId, kind: ReactionKind, subject: UnitId) {
        if !self.units[reactor].alive() {
            return;
        }
        let Some(t) = self.trait_def(reactor) else {
            return;
        };
        if t.reactions.iter().any(|r| r.id == kind && r.chance > 0.) {
            self.pending_reactions.push_back(ReactionJob {
                reactor,
                trait_id: t.id.clone(),
                resolve_epoch: self.units[reactor].resolve_epoch,
                kind,
                subject,
            });
        }
    }
    pub(super) fn attack_reactions(&mut self, actor: UnitId, target: UnitId, hit: bool) {
        if self.units[target].blueprint.hero {
            self.queue_reaction(
                target,
                if hit {
                    ReactionKind::CommentSelfHit
                } else {
                    ReactionKind::CommentSelfMissed
                },
                target,
            );
            for id in 0..self.units.len() {
                if id != target && self.units[id].blueprint.hero {
                    self.queue_reaction(
                        id,
                        if hit {
                            ReactionKind::CommentAllyHit
                        } else {
                            ReactionKind::CommentAllyMissed
                        },
                        target,
                    );
                }
            }
        }
        if self.units[actor].blueprint.hero {
            for id in 0..self.units.len() {
                if id != actor && self.units[id].blueprint.hero {
                    self.queue_reaction(
                        id,
                        if hit {
                            ReactionKind::CommentAllyAttackHit
                        } else {
                            ReactionKind::CommentAllyAttackMissed
                        },
                        actor,
                    );
                }
            }
        }
    }
    pub(super) fn flush_reactions(&mut self, events: &mut Vec<Event>) {
        let mut processed = 0;
        self.flush_effects(events);
        while let Some(job) = self.pending_reactions.pop_front() {
            self.flush_effects(events);
            processed += 1;
            if processed > MAX_REACTIONS_PER_ACTION {
                let pending = self.pending_reactions.len() + 1;
                self.pending_reactions.clear();
                self.outcome = Some(Outcome::ResolutionLimit);
                self.actor = None;
                events.push(Event::ResolutionLimit { pending });
                return;
            }
            if !self.units[job.reactor].alive() {
                continue;
            }
            if self.units[job.reactor].resolve_epoch != job.resolve_epoch {
                continue;
            }
            let reaction = self
                .trait_def(job.reactor)
                .filter(|t| t.id == job.trait_id)
                .and_then(|t| t.reactions.iter().find(|r| r.id == job.kind))
                .cloned();
            let Some(r) = reaction else {
                continue;
            };
            if r.chance <= 0. || !self.rng.roll(r.chance) {
                continue;
            }
            events.push(Event::Reaction {
                actor: job.reactor,
                kind: job.kind,
                subject: job.subject,
            });
            let effect = self.effect_for_trait(job.reactor, &r.data.effect);
            // Candidate bark recipient: all other surviving heroes. The event
            // subject identifies the stimulus, not a hidden attack target.
            for id in 0..self.units.len() {
                if id != job.reactor && self.units[id].blueprint.hero && self.units[id].alive() {
                    self.apply_external_effect(job.reactor, id, &effect, events);
                }
            }
        }
    }
    pub(super) fn forced_move(&mut self, actor: UnitId, rank: u8, events: &mut Vec<Event>) {
        if !self.placement_ranks(actor).contains(&rank) {
            return;
        }
        if self.rank(actor) == rank {
            return;
        }
        self.move_to(actor, rank);
        events.push(Event::Move { actor, rank });
        if self.units[actor].blueprint.hero {
            for id in 0..self.units.len() {
                if id != actor && self.units[id].blueprint.hero {
                    self.queue_reaction(id, ReactionKind::CommentMove, actor);
                }
            }
        }
    }
    pub(super) fn voluntary_move(&mut self, actor: UnitId, rank: u8, events: &mut Vec<Event>) {
        let before = self.rank(actor);
        let (lo, hi) = (before.min(rank), before.max(rank));
        let side = self.units[actor].friendly();
        let shifted = (0..self.units.len())
            .filter(|id| {
                self.units[*id].alive()
                    && self.units[*id].friendly() == side
                    && (lo..=hi).contains(&self.rank(*id))
            })
            .collect::<Vec<_>>();
        for id in shifted {
            if self.reaction_block(actor, id, ReactionKind::BlockMove, None, events) {
                return;
            }
        }
        self.forced_move(actor, rank, events);
    }
    pub(super) fn attempt_retreat(&mut self, actor: UnitId, events: &mut Vec<Event>) {
        for id in 0..self.units.len() {
            if self.units[id].blueprint.hero
                && self.units[id].alive()
                && self.reaction_block(actor, id, ReactionKind::BlockCombatRetreat, None, events)
            {
                return;
            }
        }
        let chance = (self.scenario.rules.combat_retreat_chance
            + self.scenario.rules.combat_retreat_bonus_chance_per_attempt
                * f64::from(self.retreat_attempts))
        .clamp(0., 1.);
        self.retreat_attempts += 1;
        let success = self.rng.roll(chance);
        events.push(Event::RetreatAttempt {
            attempt: self.retreat_attempts,
            success,
        });
        if success {
            let prisoners = self
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.captive.is_some())
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            for id in prisoners {
                self.units[id].captive = None;
                self.retire(id, events);
                self.death_stress(id, events);
                self.hero_death(id, events);
                self.schedule_death(id, false, events);
            }
            let stress = if self.scenario.rules.farm.is_some() {
                0
            } else {
                self.scenario.rules.combat_retreat_stress.round() as i32
            };
            for id in 0..self.units.len() {
                if self.units[id].blueprint.hero && self.units[id].alive() {
                    self.stress_change(id, stress, events);
                }
            }
            // Existing queued primary-action reactions are still resolved before
            // exposing a settled result; refusal itself never creates an attack.
            if self.units.iter().any(|u| u.alive() && u.blueprint.hero) {
                self.outcome = Some(Outcome::Retreated);
            }
        }
    }
    pub(super) fn start_turn_actout(&mut self, actor: UnitId, events: &mut Vec<Event>) -> bool {
        if !self.units[actor].friendly() {
            return false;
        }
        let Some(t) = self.trait_def(actor) else {
            return false;
        };
        let choices = t
            .turn_acts
            .iter()
            .filter(|a| {
                a.chance > 0.
                    && !(a.id == TurnKind::RetreatFromCombat
                        && self.units.iter().any(|u| {
                            u.alive()
                                && u.blueprint.mechanics.boss.restrictions.block_retreat_actout
                        }))
                    && (a.valid_hero_class_ids.is_empty()
                        || a.valid_hero_class_ids
                            .contains(&self.units[actor].blueprint.id))
            })
            .cloned()
            .collect::<Vec<_>>();
        if choices.is_empty() {
            return false;
        }
        let selected = self
            .weighted_index(&choices.iter().map(|a| a.chance).collect::<Vec<_>>())
            .expect("validated positive trait weights");
        let a = &choices[selected];
        if a.id == TurnKind::Nothing {
            return false;
        }
        events.push(Event::ActOut { actor, kind: a.id });
        let effect = if a.data.string_value.is_empty() {
            None
        } else {
            Some(self.effect_for_trait(actor, &a.data.string_value))
        };
        match a.id {
            TurnKind::BarkStress | TurnKind::StressHealParty | TurnKind::BuffParty => {
                for id in 0..self.units.len() {
                    if self.units[id].alive()
                        && self.units[id].blueprint.hero
                        && (a.id != TurnKind::BarkStress || id != actor)
                    {
                        self.apply_external_effect(actor, id, effect.as_ref().unwrap(), events);
                    }
                }
                false
            }
            TurnKind::StressHealSelf | TurnKind::MarkSelf => {
                self.apply_external_effect(actor, actor, effect.as_ref().unwrap(), events);
                false
            }
            TurnKind::BuffRandomPartyMember => {
                let allies = (0..self.units.len())
                    .filter(|id| {
                        *id != actor && self.units[*id].alive() && self.units[*id].blueprint.hero
                    })
                    .collect::<Vec<_>>();
                if allies.is_empty() {
                    events.push(Event::ActOutNoTarget { actor, kind: a.id });
                } else {
                    let id = allies[self.rng.below(allies.len() as u64) as usize];
                    self.apply_external_effect(actor, id, effect.as_ref().unwrap(), events);
                }
                false
            }
            TurnKind::HealSelf => {
                let heal = (f64::from(self.units[actor].stats().hp) * a.data.number_value)
                    .round()
                    .clamp(0., 1e6) as i32;
                self.heal(actor, heal, events);
                false
            }
            TurnKind::IgnoreCommand => true,
            TurnKind::ChangePos => {
                let ranks = self
                    .placement_ranks(actor)
                    .into_iter()
                    .filter(|r| *r != self.rank(actor))
                    .collect::<Vec<_>>();
                let rank = if a.data.number_value == 0. {
                    if ranks.is_empty() {
                        None
                    } else {
                        Some(ranks[self.rng.below(ranks.len() as u64) as usize])
                    }
                } else {
                    let n = i32::from(self.rank(actor)) + a.data.number_value.round() as i32;
                    u8::try_from(n).ok().filter(|r| ranks.contains(r))
                };
                if let Some(rank) = rank {
                    self.forced_move(actor, rank, events);
                } else {
                    events.push(Event::ActOutNoTarget { actor, kind: a.id });
                }
                true
            }
            TurnKind::RandomCommand => {
                let skills = self
                    .legal_actions()
                    .into_iter()
                    .filter(|l| matches!(l.action, Action::Skill { .. } | Action::SkillAuto { .. }))
                    .collect::<Vec<_>>();
                if skills.is_empty() {
                    events.push(Event::ActOutNoTarget { actor, kind: a.id });
                } else {
                    let chosen = skills[self.rng.below(skills.len() as u64) as usize].clone();
                    events.push(Event::ForcedAction {
                        actor,
                        action: chosen.action.clone(),
                    });
                    let continues = match chosen.action {
                        Action::Skill { skill, .. } | Action::SkillAuto { skill } => {
                            self.units[actor].blueprint.skills[skill]
                                .tactical
                                .continue_turn
                        }
                        _ => false,
                    };
                    self.resolve_action(actor, chosen, events);
                    if continues {
                        return false;
                    }
                }
                true
            }
            TurnKind::AttackSelf | TurnKind::AttackFriendly => {
                let target = if a.id == TurnKind::AttackSelf {
                    Some(actor)
                } else {
                    let allies = (0..self.units.len())
                        .filter(|id| {
                            *id != actor
                                && self.units[*id].alive()
                                && self.units[*id].blueprint.hero
                        })
                        .collect::<Vec<_>>();
                    if allies.is_empty() {
                        None
                    } else {
                        Some(allies[self.rng.below(allies.len() as u64) as usize])
                    }
                };
                if let Some(target) = target {
                    let base = if a.id == TurnKind::AttackSelf {
                        self.units[actor].stats().hp
                    } else {
                        self.units[actor].blueprint.stats.damage[1]
                    };
                    let raw = (f64::from(base) * a.data.number_value)
                        .round()
                        .clamp(0., 1e6) as i32;
                    let damage = candidate_damage(raw, self.units[target].stats().protection);
                    self.damage(target, damage, false, events);
                    if let Some(effect) = &effect {
                        self.apply_external_effect(actor, target, effect, events);
                    }
                    self.attack_reactions(actor, target, true);
                    self.end_stealth(actor, StealthEndReason::Attack, events);
                } else {
                    events.push(Event::ActOutNoTarget { actor, kind: a.id });
                }
                true
            }
            TurnKind::ConsumeItem => {
                let priority = self
                    .supplies
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| self.supply_usable(*i, actor))
                    .map(|(_, s)| s.definition.consume_priority)
                    .max()
                    .unwrap_or(0);
                let choices = self
                    .supplies
                    .iter()
                    .enumerate()
                    .filter(|(i, s)| {
                        self.supply_usable(*i, actor)
                            && priority > 0
                            && s.definition.consume_priority == priority
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if choices.is_empty() {
                    events.push(Event::ActOutNoTarget { actor, kind: a.id });
                } else {
                    let index = choices[self.rng.below(choices.len() as u64) as usize];
                    if let Err(error) = self.consume_supply(actor, index, events) {
                        self.lifecycle.failure = Some(error);
                    }
                }
                false
            }
            // Enabled raid-limited retreat remains outside this task.
            TurnKind::RetreatFromCombat => {
                unreachable!("enabled out-of-scope act-out was validated")
            }
            TurnKind::Nothing => false,
        }
    }
}
