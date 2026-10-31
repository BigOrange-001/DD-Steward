//! Candidate AI, Buff and stress semantics. See CALIBRATION.md.
use super::*;
use dd_data::mechanics::ReactionKind;
use dd_data::mechanics::{Brain, Desire};

impl Unit {
    pub(super) fn stealthed(&self) -> bool {
        self.alive() && self.modifier("stealth", "") > 0.
    }
    pub(super) fn has_status(&self, status: &str) -> bool {
        match status {
            "tagged" => self.marked.is_some(),
            "stunned" => self.stunned,
            "poisoned" => self
                .dots
                .iter()
                .any(|d| matches!(d.kind, EffectKind::Blight(_)) && d.remaining > 0),
            "bleeding" => self
                .dots
                .iter()
                .any(|d| matches!(d.kind, EffectKind::Bleed(_)) && d.remaining > 0),
            "burning" => self.burn > 0,
            _ => unreachable!("validated status"),
        }
    }
    pub(super) fn buff_active(&self, b: &Buff) -> bool {
        let mut probe = b.clone();
        probe.stat_type = "__condition_probe".into();
        probe.stat_sub_type = String::new();
        probe.amount = 1.;
        self.modifier_against("__condition_probe", "", None, None, &[probe], None) > 0.
    }
    pub(super) fn modifier(&self, kind: &str, sub: &str) -> f64 {
        self.modifier_for(kind, sub, None)
    }
    pub(super) fn modifier_for(&self, kind: &str, sub: &str, skill: Option<&str>) -> f64 {
        self.modifier_against(kind, sub, skill, None, &[], None)
    }
    pub(super) fn modifier_against(
        &self,
        kind: &str,
        sub: &str,
        skill: Option<&str>,
        opponent: Option<&Unit>,
        extra: &[Buff],
        attack_type: Option<&str>,
    ) -> f64 {
        self.buffs
            .iter()
            .map(|b| &b.definition)
            .chain(&self.equipment_modifiers)
            .chain(&self.light_modifiers)
            .chain(&self.aura_modifiers)
            .chain(extra)
            .filter(|b| {
                b.stat_type == kind
                    && b.stat_sub_type == sub
                    && ((match b.rule_type.as_str() {
                        "always" => true,
                        "afflicted" => matches!(self.resolve, Resolve::Afflicted(_)),
                        "virtued" => matches!(self.resolve, Resolve::Virtuous(_)),
                        "skill" => skill == Some(b.rule_data.string.as_str()),
                        "actorStatus" => {
                            opponent.is_some_and(|u| u.has_status(&b.rule_data.string))
                        }
                        "is_actor_status" => self.has_status(&b.rule_data.string),
                        "is_stealthed" => self.stealthed(),
                        "monsterType" => opponent.is_some_and(|u| {
                            u.blueprint
                                .mechanics
                                .enemy_types
                                .contains(&b.rule_data.string)
                        }),
                        "in_rank" => {
                            let rank = b.rule_data.float.unwrap_or(0.) as u8 + 1;
                            (self.rank_context..self.rank_context + self.size()).contains(&rank)
                        }
                        "target_in_rank" => opponent.is_some_and(|u| {
                            let rank = b.rule_data.float.unwrap_or(0.) as u8 + 1;
                            (u.rank_context..u.rank_context + u.size()).contains(&rank)
                        }),
                        "hpabove" => {
                            f64::from(self.hp)
                                / f64::from(self.shared_base_max.unwrap_or(self.blueprint.stats.hp))
                                > b.rule_data.float.unwrap_or(0.)
                        }
                        "hpbelow" => {
                            f64::from(self.hp)
                                / f64::from(self.shared_base_max.unwrap_or(self.blueprint.stats.hp))
                                < b.rule_data.float.unwrap_or(0.)
                        }
                        "stress_above" => f64::from(self.stress) > b.rule_data.float.unwrap_or(0.),
                        "stress_below" => f64::from(self.stress) < b.rule_data.float.unwrap_or(0.),
                        "firstroundonly" => self.round_context == 1,
                        "meleeonly" | "rangedonly" => {
                            let ty = attack_type.or_else(|| {
                                skill
                                    .and_then(|id| {
                                        self.blueprint
                                            .skills
                                            .iter()
                                            .chain(self.blueprint.mechanics.riposte.as_deref())
                                            .find(|s| s.id == id)
                                    })
                                    .map(|s| s.tactical.attack_type.as_str())
                                    .or(self.attack_context.as_deref())
                            });
                            ty == Some(if b.rule_type == "meleeonly" {
                                "melee"
                            } else {
                                "ranged"
                            })
                        }
                        "monsterSize" => opponent.is_some_and(|u| {
                            f64::from(u.size()) >= b.rule_data.float.unwrap_or(0.)
                        }),
                        "has_buff" => self
                            .buffs
                            .iter()
                            .any(|b2| b2.definition.id == b.rule_data.string),
                        "is_guarding" => self.guard.is_some(),
                        "is_guarded" => self.guarded_context,
                        "target_is_guarded" => opponent.is_some_and(|u| u.guarded_context),
                        "target_is_burning" => opponent.is_some_and(|u| u.burn > 0),
                        "any_is_burning" => self.any_burning_context,
                        "actor_base_class" => opponent
                            .is_some_and(|u| u.blueprint.mechanics.family == b.rule_data.string),
                        "at_deaths_door" => self.blueprint.hero && self.hp == 0 && self.alive(),
                        "riposte" => self.counter_context,
                        "lightabove" => self
                            .light_context
                            .is_some_and(|l| f64::from(l) > b.rule_data.float.unwrap_or(0.)),
                        "lightbelow" => self
                            .light_context
                            .is_some_and(|l| f64::from(l) < b.rule_data.float.unwrap_or(0.)),
                        "in_corridor" => self.corridor_context,
                        "walking_backwards" => self.walking_backwards_context,
                        "in_dungeon" => {
                            self.region_context.as_deref() == Some(b.rule_data.string.as_str())
                        }
                        "has_item_id" => {
                            self.items_context.contains(&b.rule_data.string)
                                || self
                                    .trinkets
                                    .iter()
                                    .any(|t| !t.destroyed && t.definition.id == b.rule_data.string)
                        }
                        "has_quirk" => {
                            self.blueprint
                                .mechanics
                                .build
                                .input
                                .quirks
                                .contains(&b.rule_data.string)
                                || self.diseases.contains(&b.rule_data.string)
                        }
                        "no_trinkets" => self.trinkets.iter().all(|t| t.destroyed),
                        "is_vampire" => self
                            .disease_tags
                            .values()
                            .flatten()
                            .any(|tag| tag == "vampire"),
                        "target_is_vampire" => opponent.is_some_and(|u| {
                            u.disease_tags
                                .values()
                                .flatten()
                                .any(|tag| tag == "vampire")
                        }),
                        "attacking_monster_type" => opponent.is_some_and(|u| {
                            u.blueprint
                                .mechanics
                                .enemy_types
                                .contains(&b.rule_data.string)
                        }),
                        "monster_type_count_min" => {
                            f64::from(
                                self.monster_counts
                                    .get(&b.rule_data.string)
                                    .copied()
                                    .unwrap_or(0),
                            ) >= b.rule_data.float.unwrap_or(0.)
                        }
                        "quest_uses_remaining_above" => self
                            .quest_uses_context
                            .get(&b.rule_data.string)
                            .is_some_and(|n| f64::from(*n) > b.rule_data.float.unwrap_or(0.)),
                        "negative_quirk_above" => {
                            self.blueprint.mechanics.build.negative_quirks as f64
                                > b.rule_data.float.unwrap_or(0.)
                        }
                        "in_mode" => self.mode.as_deref() == Some(b.rule_data.string.as_str()),
                        "never" | "always_false" => false,
                        _ => false,
                    }) ^ b.is_false_rule)
            })
            .map(|b| b.amount)
            .sum()
    }
    pub(super) fn effective_stats(&self) -> Stats {
        self.effective_stats_in(None, None)
    }
    pub(super) fn effective_stats_in(
        &self,
        opponent: Option<&Unit>,
        skill: Option<&Skill>,
    ) -> Stats {
        let mut stats = self.individual_stats_in(opponent, skill);
        if let Some(max) = self.shared_max {
            stats.hp = max;
        }
        stats
    }
    pub(super) fn individual_stats_in(
        &self,
        opponent: Option<&Unit>,
        skill: Option<&Skill>,
    ) -> Stats {
        let mut s = if matches!(self.life, Life::Corpse { .. }) {
            self.blueprint
                .corpse
                .as_ref()
                .map_or_else(|| self.blueprint.stats.clone(), |c| c.stats.clone())
        } else {
            self.blueprint.stats.clone()
        };
        if !self.alive() {
            return s;
        }
        let modifier = |kind: &str, sub: &str| {
            self.modifier_against(
                kind,
                sub,
                skill.map(|s| s.id.as_str()),
                opponent,
                &[],
                skill
                    .map(|s| s.tactical.attack_type.as_str())
                    .or_else(|| opponent.and_then(|u| u.attack_context.as_deref())),
            )
        };
        let stat = |base: f64, sub: &str| {
            (base + modifier("combat_stat_add", sub))
                * (1. + modifier("combat_stat_multiply", sub)).max(0.)
        };
        s.hp = stat(f64::from(s.hp), "max_hp").round().clamp(1., 1e6) as i32;
        s.dodge = stat(s.dodge, "defense_rating");
        s.protection = stat(s.protection, "protection_rating").clamp(0., 1.);
        s.accuracy = stat(s.accuracy, "attack_rating");
        s.crit = stat(s.crit, "crit_chance");
        s.speed = stat(f64::from(s.speed), "speed_rating")
            .round()
            .clamp(-10000., 10000.) as i32;
        s.stun_resist += modifier("resistance", "stun");
        s.bleed_resist += modifier("resistance", "bleed");
        s.blight_resist += modifier("resistance", "poison");
        s.debuff_resist += modifier("resistance", "debuff");
        s.move_resist += modifier("resistance", "move");
        s.death_resist += modifier("resistance", "death_blow");
        s
    }
}
impl Battle {
    pub(super) fn end_stealth(
        &mut self,
        target: UnitId,
        reason: StealthEndReason,
        events: &mut Vec<Event>,
    ) {
        let before = self.units[target].buffs.len();
        self.units[target]
            .buffs
            .retain(|b| b.definition.stat_type != "stealth");
        if self.units[target].buffs.len() != before {
            if self.units[target].alive() {
                self.units[target].hp = self.units[target].hp.min(self.units[target].stats().hp);
            }
            events.push(Event::StealthEnded { target, reason });
        }
    }
    pub(super) fn conditions_match(&self, target: UnitId, e: &dd_data::combat::Effect) -> bool {
        let u = &self.units[target];
        e.conditions.iter().all(|c| match c {
            dd_data::combat::Condition::Status(s) => u.has_status(s),
            dd_data::combat::Condition::MonsterType(s) => {
                u.blueprint.mechanics.enemy_types.contains(s)
            }
        })
    }
    pub(super) fn attack_values(
        &self,
        actor: UnitId,
        target: UnitId,
        s: &Skill,
        extra: &[Buff],
    ) -> (f64, [i32; 2], f64) {
        let u = &self.units[actor];
        let t = &self.units[target];
        let modifier = |kind: &str, sub: &str| {
            u.modifier_against(
                kind,
                sub,
                Some(&s.id),
                Some(t),
                extra,
                Some(&s.tactical.attack_type),
            )
        };
        let stat = |base: f64, sub| {
            (base + modifier("combat_stat_add", sub))
                * (1. + modifier("combat_stat_multiply", sub)).max(0.)
        };
        let raw = s.damage.unwrap_or([0, 0]);
        let mut range = [
            stat(f64::from(raw[0]), "damage_low").round().clamp(0., 1e6) as i32,
            stat(f64::from(raw[1]), "damage_high")
                .round()
                .clamp(0., 1e6) as i32,
        ];
        if s.tactical.dmg_per_burn_stack > 0. {
            let factor = 1. + f64::from(t.burn) * s.tactical.dmg_per_burn_stack;
            range = range.map(|n| (f64::from(n) * factor).round().clamp(0., 1e6) as i32);
        }
        range[1] = range[1].max(range[0]);
        (
            stat(s.accuracy, "attack_rating"),
            range,
            if !s.tactical.crit_valid {
                0.
            } else {
                (stat(s.crit, "crit_chance")
                    + t.modifier_against(
                        "crit_received_chance",
                        "",
                        Some(&s.id),
                        Some(u),
                        &[],
                        Some(&s.tactical.attack_type),
                    ))
                .clamp(0., 1.)
            },
        )
    }
    fn set_stress(&mut self, target: UnitId, value: i32, events: &mut Vec<Event>) {
        let before = self.units[target].stress;
        self.units[target].stress = value.clamp(0, 200);
        events.push(Event::Stress {
            target,
            change: self.units[target].stress - before,
            value: self.units[target].stress,
        });
    }

    pub(super) fn add_buff(
        &mut self,
        target: UnitId,
        definition: Buff,
        scope: BuffScope,
        events: &mut Vec<Event>,
    ) {
        self.add_buff_stacking(target, definition, scope, false, events)
    }
    pub(super) fn add_buff_stacking(
        &mut self,
        target: UnitId,
        definition: Buff,
        scope: BuffScope,
        stack_override: bool,
        events: &mut Vec<Event>,
    ) {
        if !self.units[target].alive() {
            return;
        }
        let remaining = if scope != BuffScope::Timed
            || matches!(
                definition.duration_type.as_deref(),
                Some("combat_end" | "quest_end")
            ) {
            None
        } else {
            definition.duration
        };
        let u = &mut self.units[target];
        // Candidate: refresh named timed buffs; separate stun recovery stacks.
        let stack = self.scenario.rules.stun_recovery_buff_consecutive_applies
            && self
                .scenario
                .rules
                .globals
                .as_ref()
                .unwrap()
                .stun_recovery
                .iter()
                .any(|b| b.id == definition.id);
        if !stack_override
            && !matches!(scope, BuffScope::Build | BuffScope::Disease)
            && (scope != BuffScope::Timed || !stack)
        {
            u.buffs
                .retain(|b| b.scope != scope || b.definition.id != definition.id);
        }
        events.push(Event::Buff {
            target,
            id: definition.id.clone(),
        });
        u.buffs.push(ActiveBuff {
            host_instance: None,
            battle_persistent: false,
            skill_source: self.lifecycle.skill_effect,
            source_disease: None,
            clearable: definition.clearable,
            debuff: definition.amount < 0. && !definition.force_positive,
            definition,
            remaining,
            applied_turn: u.turns,
            scope,
        });
        u.hp = u.hp.min(u.stats().hp);
    }
    pub(super) fn finish_turn(&mut self, id: UnitId) {
        let u = &mut self.units[id];
        for b in &mut u.buffs {
            if b.applied_turn < u.turns
                && let Some(n) = &mut b.remaining
            {
                *n = n.saturating_sub(1);
            }
        }
        u.buffs.retain(|b| b.remaining != Some(0));
        if u.marked.is_some_and(|until| until <= u.turns) {
            u.marked = None;
        }
        if u.alive() {
            u.hp = u.hp.min(u.stats().hp);
        }
    }
    pub(super) fn heal(&mut self, target: UnitId, amount: i32, events: &mut Vec<Event>) {
        if !self.units[target].alive() {
            return;
        }
        let amount = (f64::from(amount)
            * (1. + self.units[target].modifier("hp_heal_received_percent", "")).clamp(
                self.scenario.rules.heal_buff_percent_min,
                self.scenario.rules.heal_buff_percent_max,
            ))
        .round()
        .clamp(0., 1e6) as i32;
        if self.shared_heal(target, amount, events) {
            return;
        }
        let was_door = self.units[target].blueprint.hero && self.units[target].hp == 0;
        let amount = amount
            .min(self.units[target].stats().hp - self.units[target].hp)
            .max(0);
        self.units[target].hp += amount;
        events.push(Event::Heal { target, amount });
        if was_door && amount > 0 {
            self.units[target]
                .buffs
                .retain(|b| b.scope != BuffScope::DeathsDoor);
            let list = if self.units[target].heart_recovery_pending {
                self.units[target]
                    .blueprint
                    .mechanics
                    .heart_recovery
                    .clone()
            } else {
                self.units[target].blueprint.mechanics.recovery.clone()
            };
            self.units[target].heart_recovery_pending = false;
            for b in list {
                self.add_buff(target, b, BuffScope::Recovery, events);
            }
        }
    }
    pub(super) fn stress_change(&mut self, target: UnitId, raw: i32, events: &mut Vec<Event>) {
        if raw == 0 || !self.units[target].alive() || !self.units[target].blueprint.hero {
            return;
        }
        let rules = self.scenario.rules.stress.clone();
        let gain = if raw > 0 {
            (f64::from(raw)
                * (1.
                    + self.units[target].modifier("stress_dmg_received_percent", "")
                    + self.units[target]
                        .blueprint
                        .mechanics
                        .hero
                        .difficulty
                        .as_ref()
                        .map_or(0., |d| d.stress_modifier))
                .max(rules.stress_dmg_buff_min))
            .round()
            .clamp(0., 1e6) as i32
        } else {
            raw
        };
        self.apply_stress_delta(target, gain, events);
    }
    pub(super) fn apply_stress_delta(
        &mut self,
        target: UnitId,
        gain: i32,
        events: &mut Vec<Event>,
    ) {
        self.apply_stress_delta_mode(target, gain, events, true);
    }
    pub(super) fn apply_stress_delta_mode(
        &mut self,
        target: UnitId,
        gain: i32,
        events: &mut Vec<Event>,
        allow_resolve: bool,
    ) {
        if gain == 0 || !self.units[target].alive() || !self.units[target].blueprint.hero {
            return;
        }
        let rules = self.scenario.rules.stress.clone();
        let before = self.units[target].stress;
        self.units[target].stress = (before.saturating_add(gain)).clamp(0, 200);
        events.push(Event::Stress {
            target,
            change: self.units[target].stress - before,
            value: self.units[target].stress,
        });
        if self.units[target].stress == 0
            && matches!(self.units[target].resolve, Resolve::Afflicted(_))
        {
            self.units[target].resolve = Resolve::Calm;
            self.units[target].resolve_epoch += 1;
            self.units[target]
                .buffs
                .retain(|b| b.scope != BuffScope::Trait);
            events.push(Event::Resolve {
                target,
                state: Resolve::Calm,
            });
        }
        if allow_resolve
            && self.units[target].stress >= 100
            && self.units[target].resolve == Resolve::Calm
        {
            let affliction = (rules.affliction_base_onset_chance
                + self.units[target]
                    .blueprint
                    .mechanics
                    .hero
                    .difficulty
                    .as_ref()
                    .map_or(0., |d| d.affliction_modifier)
                - self.units[target].modifier("resolve_check_percent", ""))
            .clamp(rules.affliction_min_chance, rules.affliction_max_chance);
            let override_traits = self.units[target]
                .blueprint
                .mechanics
                .hero
                .resolve_override
                .clone();
            let roll = if override_traits.is_empty() {
                self.rng.roll(affliction)
            } else {
                true
            };
            let virtue = override_traits.is_empty()
                && !roll
                && !(self.is_required_dream_hero(target)
                    && self
                        .scenario
                        .rules
                        .flashback
                        .as_ref()
                        .is_some_and(|f| !f.allow_virtue));
            let choices = self
                .scenario
                .rules
                .globals
                .as_ref()
                .unwrap()
                .traits
                .iter()
                .filter(|t| t.virtue == virtue)
                .cloned()
                .collect::<Vec<_>>();
            let chosen = if override_traits.is_empty() {
                if self.scenario.rules.context.is_some() {
                    let weights = choices
                        .iter()
                        .map(|t| self.resolve_weight(target, t))
                        .collect::<Vec<_>>();
                    let Ok(index) = self.weighted_index(&weights) else {
                        self.lifecycle.failure = Some("no eligible resolve history outcome".into());
                        return;
                    };
                    choices[index].clone()
                } else {
                    choices[self.rng.below(choices.len() as u64) as usize].clone()
                }
            } else {
                let index = self
                    .weighted_index(&override_traits.iter().map(|v| v.1).collect::<Vec<_>>())
                    .expect("validated resolve weights");
                self.scenario
                    .rules
                    .globals
                    .as_ref()
                    .unwrap()
                    .traits
                    .iter()
                    .chain(&self.units[target].blueprint.mechanics.hero.override_defs)
                    .find(|t| t.id == override_traits[index].0)
                    .expect("validated resolve override")
                    .clone()
            };
            self.units[target].resolve = if virtue {
                Resolve::Virtuous(chosen.id.clone())
            } else {
                Resolve::Afflicted(chosen.id.clone())
            };
            let count = self.units[target]
                .resolve_history
                .entry(chosen.id.clone())
                .or_default();
            if *count >= 1000000 {
                self.lifecycle.failure = Some("resolve history bound exceeded".into());
                return;
            }
            *count += 1;
            self.units[target].resolve_epoch += 1;
            events.push(Event::Resolve {
                target,
                state: self.units[target].resolve.clone(),
            });
            for b in chosen.buffs {
                self.add_buff(target, b, BuffScope::Trait, events);
            }
            if !virtue {
                self.mode_forced(target, false, events);
            }
            if virtue {
                self.set_stress(
                    target,
                    self.scenario
                        .rules
                        .stress_affliction_critically_resisted
                        .round() as i32,
                    events,
                );
            }
        }
        if self.units[target].stress >= 200 {
            if matches!(self.units[target].resolve, Resolve::Virtuous(_)) {
                self.units[target].resolve = Resolve::Calm;
                self.units[target].resolve_epoch += 1;
                self.set_stress(
                    target,
                    self.scenario.rules.stress_affliction_deactivated.round() as i32,
                    events,
                );
                self.units[target]
                    .buffs
                    .retain(|b| b.scope != BuffScope::Trait);
                events.push(Event::Resolve {
                    target,
                    state: Resolve::Calm,
                });
            } else {
                events.push(Event::HeartAttack(target));
                self.set_stress(target, 170, events);
                if self.units[target].hp == 0 {
                    self.units[target].life = Life::Dead;
                    events.push(Event::Death(target));
                    self.death_stress(target, events);
                    self.hero_death(target, events);
                } else {
                    let hp = self.units[target].hp;
                    self.damage(target, hp, false, events);
                    self.units[target].heart_recovery_pending = true;
                }
            }
        }
    }
    pub(super) fn death_stress(&mut self, target: UnitId, events: &mut Vec<Event>) {
        self.end_stealth(target, StealthEndReason::Death, events);
        let r = &self.scenario.rules.stress;
        let (chance, amount) = (
            r.death_party_stress_chance,
            r.death_party_stress_damage.round() as i32,
        );
        for id in 0..self.units.len() {
            if id != target
                && self.units[id].blueprint.hero
                && self.units[id].alive()
                && self.rng.roll(chance)
            {
                self.stress_change(id, amount, events);
            }
        }
    }
    pub(super) fn door_stress(&mut self, target: UnitId, events: &mut Vec<Event>) {
        let repeat = self.units[target].entered_deaths_door;
        self.units[target].entered_deaths_door = true;
        let r = &self.scenario.rules.stress;
        let (own_chance, own, other_chance, other) = if repeat {
            (
                r.deaths_door_self_stress_chance_repeat,
                r.deaths_door_self_stress_dmg_repeat,
                r.deaths_door_companion_stress_chance_repeat,
                r.deaths_door_companion_stress_dmg_repeat,
            )
        } else {
            (
                r.deaths_door_self_stress_chance,
                r.deaths_door_self_stress_dmg,
                r.deaths_door_companion_stress_chance,
                r.deaths_door_companion_stress_dmg,
            )
        };
        if self.rng.roll(own_chance) {
            self.stress_change(target, own.round() as i32, events);
        }
        for id in 0..self.units.len() {
            if id != target
                && self.units[id].blueprint.hero
                && self.units[id].alive()
                && self.rng.roll(other_chance)
            {
                self.stress_change(id, other.round() as i32, events);
            }
        }
    }
    pub(super) fn crit_stress(&mut self, actor: UnitId, target: UnitId, events: &mut Vec<Event>) {
        let hero = self.units[actor].blueprint.hero;
        if hero
            && self.units[target]
                .blueprint
                .mechanics
                .boss
                .restrictions
                .no_crit_relief
        {
            return;
        }
        let r = if hero {
            &self.scenario.rules.stress.player_crit
        } else {
            &self.scenario.rules.stress.monster_crit
        };
        let (chance, amount, others, other_amount) = (
            r.relief_chance,
            r.stress_relief,
            r.companion_relief_chance,
            r.companion_stress_relief,
        );
        let main = if hero { actor } else { target };
        let sign = if hero { -1 } else { 1 };
        if self.rng.roll(chance) {
            self.stress_change(main, sign * amount.round() as i32, events);
        }
        for id in 0..self.units.len() {
            if id != main
                && self.units[id].blueprint.hero
                && self.units[id].alive()
                && self.rng.roll(others)
            {
                self.stress_change(id, sign * other_amount.round() as i32, events);
            }
        }
    }
    pub(super) fn apply_skill_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        e: &dd_data::combat::Effect,
        events: &mut Vec<Event>,
    ) {
        let before = self.lifecycle.skill_effect;
        self.lifecycle.skill_effect = true;
        self.dispatch_effect(actor, target, e, events);
        self.lifecycle.skill_effect = before;
    }
    pub(super) fn apply_external_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        e: &dd_data::combat::Effect,
        events: &mut Vec<Event>,
    ) {
        let before = self.lifecycle.skill_effect;
        self.lifecycle.skill_effect = false;
        self.dispatch_effect(actor, target, e, events);
        self.flush_effects(events);
        self.lifecycle.skill_effect = before;
    }
    pub(super) fn apply_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        e: &dd_data::combat::Effect,
        events: &mut Vec<Event>,
    ) {
        if let EffectKind::BattlePersistent(kind) = &e.kind {
            let mut inner = e.clone();
            inner.kind = *kind.clone();
            self.apply_effect(actor, target, &inner, events);
            let targets = match e.target {
                EffectTarget::Performer => vec![actor],
                EffectTarget::Target => vec![target],
                EffectTarget::Party => self
                    .units
                    .iter()
                    .enumerate()
                    .filter(|(_, u)| u.friendly() == self.units[actor].friendly())
                    .map(|(id, _)| id)
                    .collect(),
                _ => {
                    self.lifecycle.failure = Some("unsupported persistent effect target".into());
                    return;
                }
            };
            for id in targets {
                match kind.as_ref() {
                    EffectKind::Buffs(bs) => {
                        for active in &mut self.units[id].buffs {
                            if active.scope == BuffScope::Timed && bs.contains(&active.definition) {
                                active.remaining = None;
                                active.battle_persistent = true;
                            }
                        }
                    }
                    EffectKind::Riposte { .. } => {
                        if let Some(r) = &mut self.units[id].riposte {
                            r.permanent = true;
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if let EffectKind::InventoryEffect(kind) = &e.kind {
            if self.lifecycle.items.source.is_none() {
                self.lifecycle.failure = Some("inventory effect requires item context".into());
                return;
            }
            let mut effect = e.clone();
            effect.kind = *kind.clone();
            self.apply_effect(actor, target, &effect, events);
            return;
        }
        if let EffectKind::TrinketOp(op) = &e.kind {
            let applied = if self.rng.roll(e.chance.clamp(0., 1.)) {
                match self.trinket_operation(op, events) {
                    Ok(applied) => applied,
                    Err(error) => {
                        self.lifecycle.failure = Some(error);
                        false
                    }
                }
            } else {
                false
            };
            events.push(Event::Effect {
                target: actor,
                name: e.name.clone(),
                applied,
            });
            return;
        }
        if let EffectKind::Composite(ops) = &e.kind {
            for operation in ops {
                self.apply_effect(actor, target, operation, events);
            }
            return;
        }
        let targets = match e.target {
            EffectTarget::ThisTrinket => {
                self.lifecycle.failure = Some("unsupported non-item operation on trinket".into());
                return;
            }
            EffectTarget::EnemyRandom => {
                let choices = (0..self.units.len())
                    .filter(|id| {
                        self.units[*id].present()
                            && self.units[*id].friendly() != self.units[actor].friendly()
                            && (self.units[*id].alive() || e.can_apply_on_death)
                    })
                    .collect::<Vec<_>>();
                if choices.is_empty() {
                    vec![]
                } else {
                    vec![choices[self.rng.below(choices.len() as u64) as usize]]
                }
            }
            EffectTarget::EnemyRank(rank) => (0..self.units.len())
                .filter(|id| {
                    self.units[*id].present()
                        && self.units[*id].friendly() != self.units[actor].friendly()
                        && self.occupied_ranks(*id).any(|r| r == rank)
                })
                .collect(),
            EffectTarget::TargetEnemies => (0..self.units.len())
                .filter(|id| {
                    self.units[*id].present()
                        && self.units[*id].friendly() != self.units[target].friendly()
                })
                .collect(),
            EffectTarget::TargetParty => (0..self.units.len())
                .filter(|id| {
                    self.units[*id].present()
                        && self.units[*id].friendly() == self.units[target].friendly()
                })
                .collect(),
            EffectTarget::Target => vec![target],
            EffectTarget::Performer => vec![actor],
            EffectTarget::Party | EffectTarget::PartyOther => (0..self.units.len())
                .filter(|id| {
                    self.units[*id].friendly() == self.units[actor].friendly()
                        && (e.target != EffectTarget::PartyOther || *id != actor)
                })
                .collect(),
            EffectTarget::Enemies => (0..self.units.len())
                .filter(|id| {
                    self.units[*id].friendly() != self.units[actor].friendly()
                        && self.units[*id].present()
                })
                .collect(),
        };
        for id in targets {
            if matches!(e.kind, EffectKind::Horror(_)) && !self.units[id].blueprint.hero {
                events.push(Event::Effect {
                    target: id,
                    name: e.name.clone(),
                    applied: false,
                });
                continue;
            }
            if !self.units[id].alive() {
                if !e.kind.is_displacement()
                    && !matches!(
                        e.kind,
                        EffectKind::KillCorpses | EffectKind::Burn(_) | EffectKind::Lifecycle(_)
                    )
                {
                    continue;
                }
                if !self.units[id].present() || !e.can_apply_on_death {
                    events.push(Event::MoveSkipped {
                        actor: id,
                        reason: if !self.units[id].present() {
                            MoveFailure::NotPresent
                        } else {
                            MoveFailure::CorpseNotAllowed
                        },
                    });
                    continue;
                }
            }
            if self.reaction_block(actor, id, ReactionKind::BlockEffect, Some(&e.name), events) {
                continue;
            }
            if matches!(
                e.kind,
                EffectKind::Buffs(_)
                    | EffectKind::ProtectedBuffs(_)
                    | EffectKind::Stealth
                    | EffectKind::Guard { .. }
                    | EffectKind::Riposte { .. }
                    | EffectKind::Blocks { .. }
            ) && !e.resist_debuff
                && self.reaction_block(actor, id, ReactionKind::BlockBuff, Some(&e.name), events)
            {
                continue;
            }
            let s = self.units[id].effective_stats_in(Some(&self.units[actor]), None);
            let resist = match e.kind {
                EffectKind::Bleed(_) => s.bleed_resist,
                EffectKind::Blight(_) => s.blight_resist,
                EffectKind::Stun => s.stun_resist,
                EffectKind::Push(_) | EffectKind::Pull(_) | EffectKind::Shuffle => s.move_resist,
                _ if e.resist_debuff => s.debuff_resist,
                _ => 0.,
            };
            let bonus = match e.kind {
                EffectKind::Bleed(_) => self.units[actor].modifier("bleed_chance", ""),
                EffectKind::Blight(_) => self.units[actor].modifier("poison_chance", ""),
                EffectKind::Stun => self.units[actor].modifier("stun_chance", ""),
                EffectKind::Push(_) | EffectKind::Pull(_) | EffectKind::Shuffle => {
                    self.units[actor].modifier("move_chance", "")
                }
                _ if e.resist_debuff => self.units[actor].modifier("debuff_chance", ""),
                _ => 0.,
            };
            let (bonus, resist) = if matches!(
                e.kind,
                EffectKind::Boss(dd_data::special::BossOp::ShuffleDot { .. })
            ) {
                (
                    self.units[actor].modifier("move_chance", ""),
                    self.units[id].stats().move_resist,
                )
            } else {
                (bonus, resist)
            };
            let critical_bonus = if self.lifecycle.skill_effect
                && self.units[actor].critical_context
                && e.execution.critical_roll_bonus
            {
                -self.scenario.rules.effect_crit_roll_mod
            } else {
                0.
            };
            let mut applied = self
                .rng
                .roll((e.chance + bonus + critical_bonus - resist).clamp(0., 1.));
            if applied {
                match &e.kind {
                    EffectKind::HealthDamage(n) => self.damage(id, *n, false, events),
                    EffectKind::ControlledBurn { amount, rounds } => {
                        let zone = heroes010::RankBurn {
                            hero: self.units[id].blueprint.hero,
                            rank: self.rank(id),
                            amount: *amount,
                            expires: self.round + rounds,
                        };
                        self.lifecycle
                            .zones
                            .retain(|z| z.hero != zone.hero || z.rank != zone.rank);
                        self.lifecycle.zones.push(zone);
                    }
                    EffectKind::InstantBuffs(_) => {
                        applied = false;
                    }
                    EffectKind::RemoveVampire => {
                        let ids = self.units[id]
                            .disease_tags
                            .iter()
                            .filter(|(_, tags)| tags.iter().any(|t| t == "vampire"))
                            .map(|(id, _)| id.clone())
                            .collect::<Vec<_>>();
                        for source in ids {
                            self.remove_disease(id, &source);
                        }
                        self.refresh_context(false);
                    }
                    EffectKind::Disease {
                        id: selector,
                        crit_bonus,
                    } => {
                        applied = false;
                        if self.units[id].blueprint.hero {
                            if let Some(ctx) = self.scenario.rules.context.clone() {
                                let choices = ctx
                                    .diseases
                                    .values()
                                    .filter(|d| {
                                        (selector == "any" || selector == &d.id)
                                            && (selector != "any" || d.weight > 0.)
                                            && !self.units[id].diseases.contains(&d.id)
                                            && !d
                                                .incompatible
                                                .iter()
                                                .any(|v| self.units[id].diseases.contains(v))
                                    })
                                    .cloned()
                                    .collect::<Vec<_>>();
                                if !choices.is_empty()
                                    && self.rng.roll(
                                        (1. - self.units[id].blueprint.mechanics.disease_resist
                                            - self.units[id].modifier("resistance", "disease")
                                            + if *crit_bonus
                                                && self.lifecycle.skill_effect
                                                && self.units[actor].critical_context
                                            {
                                                self.scenario
                                                    .rules
                                                    .effect_crit_disease_resist_roll_mod
                                            } else {
                                                0.
                                            })
                                        .clamp(0., 1.),
                                    )
                                {
                                    let index = self
                                        .weighted_index(
                                            &choices
                                                .iter()
                                                .map(
                                                    |d| {
                                                        if selector == "any" {
                                                            d.weight
                                                        } else {
                                                            1.
                                                        }
                                                    },
                                                )
                                                .collect::<Vec<_>>(),
                                        )
                                        .expect("validated disease weights");
                                    let d = &choices[index];
                                    applied = self.contract_disease(id, d, events);
                                }
                            } else {
                                self.lifecycle.failure = Some("missing disease context".into());
                            }
                        }
                    }
                    EffectKind::SetMode(mode) => {
                        if self.units[id]
                            .blueprint
                            .mechanics
                            .hero
                            .modes
                            .iter()
                            .any(|m| &m.id == mode)
                            || self.units[id]
                                .blueprint
                                .mechanics
                                .boss
                                .modes
                                .iter()
                                .any(|m| &m.id == mode)
                        {
                            if self.units[id].mode.as_ref() != Some(mode)
                                && !self.units[id].blueprint.hero
                                && !self.units[id]
                                    .blueprint
                                    .mechanics
                                    .boss
                                    .modes
                                    .iter()
                                    .any(|m| &m.id == mode && m.keep_rank_clock)
                            {
                                self.units[id].rank_turns = 0;
                                self.units[id].turn_rank = self.rank(id);
                            }
                            self.units[id].mode = Some(mode.clone());
                            events.push(Event::Mode {
                                unit: id,
                                mode: mode.clone(),
                            });
                        } else {
                            self.lifecycle.failure = Some(format!("unknown actor mode {mode}"));
                            applied = false;
                        }
                    }

                    EffectKind::Boss(op) => {
                        applied = self.boss_operation(actor, id, op, events);
                    }
                    EffectKind::Lifecycle(l) => {
                        applied = self.lifecycle_effect(actor, id, l, e.can_apply_on_death, events)
                    }
                    EffectKind::ProtectedHorror(n) => {
                        if !self.is_required_dream_hero(id) {
                            self.lifecycle.failure =
                                Some("protected Horror requires flashback context".into());
                            applied = false;
                        } else {
                            self.lifecycle.special.dream.horror.insert(
                                id,
                                Dot {
                                    can_crit_heal: false,
                                    amount: *n,
                                    remaining: e.duration,
                                    kind: e.kind.clone(),
                                },
                            );
                        }
                    }
                    EffectKind::Horror(n) => self.units[id].dots.push(Dot {
                        can_crit_heal: false,
                        amount: *n,
                        remaining: e.duration,
                        kind: e.kind.clone(),
                    }),
                    EffectKind::ClearHorror => self.units[id]
                        .dots
                        .retain(|d| !matches!(d.kind, EffectKind::Horror(_))),
                    EffectKind::Stealth => self.add_buff(
                        id,
                        Buff {
                            activation: Default::default(),
                            id: format!("{}:stealth", e.name),
                            stat_type: "stealth".into(),
                            stat_sub_type: String::new(),
                            amount: 1.,
                            duration: Some(e.duration),
                            duration_type: None,
                            rule_type: "always".into(),
                            is_false_rule: false,
                            clearable: true,
                            force_positive: false,
                            rule_data: Default::default(),
                        },
                        BuffScope::Timed,
                        events,
                    ),
                    EffectKind::Unstealth => self.end_stealth(id, StealthEndReason::Effect, events),
                    EffectKind::Push(_) | EffectKind::Pull(_) | EffectKind::Shuffle => {
                        self.displace(id, &e.kind, events)
                    }
                    EffectKind::Bleed(n) | EffectKind::Blight(n) => {
                        let remaining = if matches!(e.kind, EffectKind::Blight(_)) {
                            (f64::from(e.duration)
                                * (1.
                                    + self.units[actor]
                                        .modifier("hp_dot_poison_duration_percent", ""))
                                .max(0.))
                            .round()
                            .clamp(1., 10000.) as u32
                        } else {
                            e.duration
                        };
                        let remaining =
                            if self.lifecycle.skill_effect && self.units[actor].critical_context {
                                (f64::from(remaining)
                                    * self
                                        .scenario
                                        .rules
                                        .candidates
                                        .dot_critical
                                        .duration_multiplier)
                                    .ceil()
                                    .clamp(1., 10000.) as u32
                            } else {
                                remaining
                            };
                        self.units[id].dots.push(Dot {
                            can_crit_heal: false,
                            amount: *n,
                            remaining,
                            kind: e.kind.clone(),
                        });
                    }
                    EffectKind::Stun => self.units[id].stunned = true,
                    EffectKind::Stress(n) => {
                        let amount = if *n < 0 {
                            (f64::from(*n)
                                * (1. + self.units[actor].modifier("stress_heal_percent", ""))
                                    .max(0.))
                            .round() as i32
                        } else {
                            (f64::from(*n)
                                * (1.
                                    + self.units[actor].modifier_for(
                                        "stress_dmg_percent",
                                        "",
                                        self.lifecycle.last_skill.as_deref(),
                                    ))
                                .max(0.))
                            .round() as i32
                        };
                        self.stress_change(id, amount, events);
                    }
                    EffectKind::Heal(n) => self.skill_heal(
                        actor,
                        id,
                        *n,
                        matches!(
                            e.target,
                            EffectTarget::Party
                                | EffectTarget::PartyOther
                                | EffectTarget::TargetParty
                        ),
                        e.execution
                            .allows_critical_heal(self.lifecycle.skill_effect, false),
                        events,
                    ),
                    EffectKind::Buffs(list) | EffectKind::ProtectedBuffs(list) => {
                        for b in list {
                            self.add_buff(id, b.clone(), BuffScope::Timed, events);
                            if let Some(active) = self.units[id]
                                .buffs
                                .iter_mut()
                                .rev()
                                .find(|active| active.definition.id == b.id)
                            {
                                active.clearable =
                                    b.clearable && !matches!(e.kind, EffectKind::ProtectedBuffs(_));
                                active.debuff = e.resist_debuff && !b.force_positive;
                            }
                        }
                    }
                    EffectKind::Mark => {
                        if self.units[id].modifier("tag_blocked", "") > 0. {
                            applied = false;
                        } else {
                            self.units[id].marked = Some(self.units[id].turns + e.duration)
                        }
                    }
                    EffectKind::Composite(_) => {
                        unreachable!("composites execute their checked children")
                    }
                    _ => applied = self.apply_tactical(actor, id, e, events),
                }
            }
            events.push(Event::Effect {
                target: id,
                name: e.name.clone(),
                applied,
            });
            self.cleanup_tactics(events);
            self.refresh_context(true);
        }
    }
    /// Host-only AI transition. All random choices use the saved combat RNG.
    /// Unsupported/zero-weight configurations return errors without mutation.
    pub fn step_monster(&mut self) -> Result<Vec<Event>> {
        let before = self.rng.clone();
        let result = self.monster_action().and_then(|a| self.step(a));
        if result.is_err() {
            self.rng = before;
        }
        result
    }
    pub(super) fn monster_action(&mut self) -> Result<Action> {
        let actor = self.actor.ok_or("no active actor")?;
        if self.lifecycle.special.selection.is_some() {
            return Err("player sacrifice selection required".into());
        }
        if self.units[actor].controlled.is_some() {
            return self.controlled_action();
        }
        if self.units[actor].blueprint.hero {
            return Err("active actor is a hero".into());
        }
        let brain = self.units[actor]
            .blueprint
            .mechanics
            .brain
            .clone()
            .ok_or("monster has no interpreted brain")?;
        let legal = self.legal_actions();
        let mut groups: BTreeMap<usize, Vec<&LegalAction>> = BTreeMap::new();
        for a in &legal {
            if let Action::Skill { skill, .. } | Action::SkillAuto { skill } = a.action {
                groups.entry(skill).or_default().push(a);
            }
        }
        if groups.is_empty() {
            return Ok(Action::Pass);
        }
        // The source can explicitly disable the single-valid-skill protocol.
        // Its absent default is a candidate; target desires remain mandatory.
        let single_skill =
            groups.len() == 1 && brain.ignore_desires_on_single_valid_skill.unwrap_or(true);
        let weighted = groups
            .iter()
            .map(|(&skill, actions)| -> Result<_> {
                let weight = brain
                    .skill_selection_desires
                    .iter()
                    .enumerate()
                    .filter(|(i, d)| {
                        self.units[actor]
                            .desire_rolls
                            .get(*i)
                            .copied()
                            .unwrap_or(true)
                            && self.skill_desire(actor, skill, d)
                    })
                    .map(|(_, d)| d)
                    .map(|d| self.desire_weight(d))
                    .sum::<f64>();
                if !weight.is_finite() {
                    return Err("nonfinite AI skill weight sum".into());
                }
                let targets = if matches!(actions[0].action, Action::SkillAuto { .. }) {
                    None
                } else {
                    Some(self.target_weights(actor, skill, actions, &brain)?)
                };
                let weight = if targets
                    .as_ref()
                    .is_none_or(|weights| weights.iter().any(|w| *w > 0.))
                {
                    if single_skill { 1. } else { weight.max(0.) }
                } else {
                    0.
                };
                Ok((skill, weight, actions, targets))
            })
            .collect::<Result<Vec<_>>>()?;
        let selected = self
            .weighted_index(&weighted.iter().map(|x| x.1).collect::<Vec<_>>())
            .map_err(|e| {
                format!(
                    "{e}; actor={} round={} turn={} hp={:?} skills={:?}",
                    self.units[actor].blueprint.id,
                    self.round,
                    self.units[actor].turns,
                    self.units
                        .iter()
                        .filter(|u| u.alive())
                        .map(|u| (&u.blueprint.id, u.hp, u.stats().hp))
                        .collect::<Vec<_>>(),
                    weighted
                        .iter()
                        .map(|(s, w, _, t)| (&self.units[actor].blueprint.skills[*s].id, *w, t))
                        .collect::<Vec<_>>()
                )
            })?;
        let (_, _, actions, weights) = &weighted[selected];
        if matches!(actions[0].action, Action::SkillAuto { .. }) {
            return Ok(actions[0].action.clone());
        }
        let index = self.weighted_index(weights.as_ref().unwrap())?;
        Ok(actions[index].action.clone())
    }
    pub(super) fn weighted_index(&mut self, weights: &[f64]) -> Result<usize> {
        let total: f64 = weights.iter().sum();
        if !total.is_finite() || total <= 0. {
            return Err("AI has legal actions but no positive desire weight".into());
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
    pub(super) fn desire_weight(&self, d: &Desire) -> f64 {
        d.data.base_chance
            + dd_data::ai::number(d, "per_round_chance").unwrap_or(0.)
                * f64::from(self.round.saturating_sub(1))
    }
    fn skill_desire(&self, actor: UnitId, index: usize, d: &Desire) -> bool {
        let u = &self.units[actor];
        let s = &u.blueprint.skills[index];
        if !d.data.combat_skill_id.is_empty() && d.data.combat_skill_id != s.id {
            return false;
        }
        if !self.ai_predicates(actor, d) {
            return false;
        }
        match d.kind.as_str() {
            "random_skill" => true,
            "specific_skill" => {
                s.id == d.data.combat_skill_id
                    || (d.data.combat_skill_id.is_empty() && u.blueprint.skills.len() == 1)
            }
            "preferred_skill" => u.blueprint.mechanics.preferred_skill == Some(index),
            "performing_turn_skill" => {
                s.id == d.data.combat_skill_id && d.data.performing_turn == Some(u.turns)
            }
            "stun_skill" => s.effects.iter().any(|e| e.kind.contains_stun()),
            "heal_skill" => {
                s.heal.is_some()
                    && self.targets(actor, s).iter().any(|id| {
                        f64::from(self.units[*id].hp) / f64::from(self.units[*id].stats().hp)
                            < d.data.hp_ratio_threshold.unwrap()
                    })
            }
            "effect_key_status_skill" => self.ai_effect_skill(actor, s, d),
            "ally_alive_skill" | "ally_dead_skill" | "ally_hp_ratio_skill" => {
                self.ai_ally_skill(actor, d)
            }
            "performer_dot_skill" => self.ai_dot_skill(actor, d),
            "performer_guard_skill" => true,
            "fill_ally_captor_empty_skill" => {
                s.effects.iter().any(|e| {
                    matches!(
                        e.kind,
                        EffectKind::Boss(dd_data::special::BossOp::Capture { .. })
                            | EffectKind::Lifecycle(dd_data::lifecycle::LifecycleEffect {
                                op: dd_data::lifecycle::LifeOp::Boss(
                                    dd_data::special::BossOp::Capture { .. }
                                ),
                                ..
                            })
                    )
                }) && self.units.iter().any(|v| {
                    v.active()
                        && v.blueprint
                            .mechanics
                            .boss
                            .empty
                            .as_ref()
                            .is_some_and(|c| c.performer == u.blueprint.mechanics.family)
                }) && self.targets(actor, s).iter().any(|id| {
                    self.units[*id].captive.is_none() && self.ai_target_predicates(*id, d)
                })
            }
            _ => false,
        }
    }
    pub(super) fn target_weights(
        &self,
        actor: UnitId,
        skill: usize,
        actions: &[&LegalAction],
        brain: &Brain,
    ) -> Result<Vec<f64>> {
        if self.units[actor].blueprint.skills[skill]
            .target
            .ranks
            .is_empty()
        {
            return Ok(vec![1.; actions.len()]);
        }
        // A fixed group is an entity set, never its arbitrary display anchor.
        if self.units[actor].blueprint.skills[skill].target.multi
            && actions.len() == 1
            && actions[0].targets.len() > 1
        {
            let members = actions[0]
                .targets
                .iter()
                .map(|id| {
                    let mut a = actions[0].clone();
                    a.action = Action::Skill { skill, target: *id };
                    a.targets = vec![*id];
                    a
                })
                .collect::<Vec<_>>();
            let weights =
                self.target_weights(actor, skill, &members.iter().collect::<Vec<_>>(), brain)?;
            let total = weights.iter().sum::<f64>();
            if !total.is_finite() {
                return Err("nonfinite AI group target sum".into());
            }
            return Ok(vec![total]);
        }
        let mut weights = vec![0.; actions.len()];
        let mut exclusive = vec![false; actions.len()];
        let mut has_exclusive = false;
        for d in &brain.target_selection_desires {
            if !d.data.specific_combat_skill_id.is_empty()
                && d.data.specific_combat_skill_id != self.units[actor].blueprint.skills[skill].id
            {
                continue;
            }
            let mut candidates = vec![];
            for (i, a) in actions.iter().enumerate() {
                let Action::Skill { target, .. } = a.action else {
                    continue;
                };
                let u = &self.units[target];
                if !self.ai_target_predicates(target, d) {
                    continue;
                }
                if (u.friendly() == self.units[actor].friendly()
                    && !d.data.is_friendly_target_desire)
                    || (u.friendly() != self.units[actor].friendly()
                        && !d.data.is_enemy_target_desire)
                {
                    continue;
                }
                let qualifies = match d.kind.as_str() {
                    "random_target" => true,
                    "fill_ally_captor_empty_target" => u.captive.is_none(),
                    "actor_dot_target" => dd_data::ai::string(d, "actor_dot_entry_id")
                        .is_some_and(|id| u.parasites.iter().any(|p| p.definition.id == id)),
                    "trinket_target" => dd_data::ai::string(d, "trinket_id").is_some_and(|id| {
                        u.trinkets
                            .iter()
                            .any(|t| !t.destroyed && t.definition.id == id)
                    }),
                    "vampire_target" => u
                        .disease_tags
                        .values()
                        .any(|tags| tags.iter().any(|t| t == "vampire")),
                    "marked_target" => u.marked.is_some(),
                    "stunned_target" => u.stunned,
                    "rank_target" => d.data.rank.map_or_else(
                        || {
                            self.lifecycle
                                .special
                                .marked_ranks
                                .get(&actor)
                                .is_some_and(|rs| {
                                    self.occupied_ranks(target).any(|r| rs.contains(&r))
                                })
                        },
                        |r| self.occupied_ranks(target).contains(&r),
                    ),
                    "health_target" => dd_data::ai::string(d, "ally_base_class_id")
                        .is_none_or(|family| u.blueprint.mechanics.family == family),
                    "resistance_target" => true,
                    "ally_class_target" => {
                        dd_data::ai::string(d, "ally_base_class_id")
                            == Some(u.blueprint.mechanics.family.as_str())
                    }
                    "guard_target" => true,
                    "next_initiative_target" => {
                        self.queue.iter().copied().find(|id| {
                            self.units[*id].alive()
                                && self.units[*id].friendly() != self.units[actor].friendly()
                        }) == Some(target)
                    }
                    "stress_target" => match u.resolve {
                        Resolve::Calm => d.data.can_target_not_overstressed,
                        Resolve::Afflicted(_) => d.data.can_target_afflicted,
                        Resolve::Virtuous(_) => d.data.can_target_virtued,
                    },
                    _ => false,
                };
                if qualifies {
                    let value = if d.kind == "resistance_target" {
                        self.ai_resistance(target, d)
                    } else if d.kind == "stress_target" {
                        f64::from(u.stress)
                    } else {
                        f64::from(u.hp) / f64::from(u.stats().hp)
                    };
                    candidates.push((i, value));
                }
            }
            if ["stress_target", "health_target", "resistance_target"].contains(&d.kind.as_str())
                && !candidates.is_empty()
            {
                let best = candidates
                    .iter()
                    .map(|(_, v)| *v)
                    .reduce(|a, b| {
                        if d.data.is_greater_comparison {
                            a.max(b)
                        } else {
                            a.min(b)
                        }
                    })
                    .unwrap();
                candidates.retain(|(_, v)| (*v - best).abs() < 1e-12);
            }
            for (i, _) in candidates {
                weights[i] += d.data.base_chance;
                if d.data.is_exclusive_desire && d.data.base_chance > 0. {
                    exclusive[i] = true;
                    has_exclusive = true;
                }
            }
        }
        if has_exclusive {
            for (w, e) in weights.iter_mut().zip(exclusive) {
                if !e {
                    *w = 0.;
                }
            }
        }
        for w in &mut weights {
            if !w.is_finite() {
                return Err("nonfinite AI target weight sum".into());
            }
            *w = w.max(0.);
        }
        Ok(weights)
    }
}
