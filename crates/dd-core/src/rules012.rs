//! Remaining battle formulas and the finite effect queue. All absent constants
//! come from the explicit, source-labelled host candidate document.
use super::*;
use dd_data::mechanics::ReactionKind;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct QueuedEffect {
    pub actor: UnitId,
    pub target: UnitId,
    pub effect: dd_data::combat::Effect,
    pub critical: bool,
    pub skill_source: bool,
    pub counter: bool,
    pub attack_type: Option<String>,
}
impl Battle {
    pub(super) fn supply_usable(&self, item: usize, target: UnitId) -> bool {
        let Some(stack) = self.supplies.get(item) else {
            return false;
        };
        let Some(unit) = self.units.get(target) else {
            return false;
        };
        stack.remaining > 0
            && unit.active()
            && unit.friendly()
            && (!stack.definition.effects.is_empty()
                || !stack.definition.semantics.sequential_effects.is_empty())
            && !(stack.definition.id == "spice" && matches!(unit.resolve, Resolve::Virtuous(_)))
            && !(stack.definition.id == "snake_scale"
                && unit.blocks >= self.scenario.rules.health_damage_blocks_max)
            && unit
                .item_uses
                .get(&stack.definition.id)
                .is_none_or(|n| *n < 1000000)
    }
    pub(super) fn consume_supply(
        &mut self,
        target: UnitId,
        item: usize,
        events: &mut Vec<Event>,
    ) -> Result<()> {
        if !self.supply_usable(item, target) {
            return Err("supply cannot be used on this recipient".into());
        }
        self.supplies[item].remaining -= 1;
        self.refill_supply(item);
        events.push(Event::SupplyConsumed {
            actor: target,
            item: self.supplies[item].definition.id.clone(),
            remaining: self.supplies[item].remaining
                + self
                    .supply_reserves
                    .get(&self.supplies[item].definition.id)
                    .copied()
                    .unwrap_or(0),
        });
        let definition = &self.supplies[item].definition;
        let mut effects = if definition.semantics.sequential_effects.is_empty() {
            definition.effects.clone()
        } else {
            let id = definition.id.clone();
            let count = self.units[target].item_uses.get(&id).copied().unwrap_or(0);
            let selected = definition.semantics.sequential_effects
                [(count as usize).min(definition.semantics.sequential_effects.len() - 1)]
            .clone();
            self.units[target].item_uses.insert(id, count + 1);
            vec![selected]
        };
        let mut default_blood = self.supplies[item].definition.id == "the_blood";
        if let Some(ctx) = self.scenario.rules.context.clone() {
            let uses = self.units[target]
                .diseases
                .iter()
                .filter_map(|id| ctx.diseases.get(id).map(|d| (id, d)))
                .flat_map(|(source, d)| {
                    d.item_changes
                        .iter()
                        .filter(|u| u.item == self.supplies[item].definition.id)
                        .map(move |u| (source.clone(), u.clone()))
                })
                .collect::<Vec<_>>();
            if !uses.is_empty() {
                default_blood = false;
                effects.clear();
            }
            for (source, usage) in uses {
                if let Some(new) = usage.change {
                    let d = ctx
                        .diseases
                        .get(&new)
                        .ok_or("missing disease item replacement")?;
                    self.remove_disease(target, &source);
                    self.give_disease(target, d, events);
                }
                effects.extend(usage.effects);
            }
        }
        if default_blood {
            let damage = (f64::from(self.units[target].stats().hp)
                * self.scenario.rules.the_blood_item_health_damage_percent)
                .round()
                .clamp(0., 1e6) as i32;
            self.damage(target, damage, false, events);
        }
        for effect in effects {
            self.dispatch_effect(target, target, &effect, events);
        }

        self.flush_effects(events);
        Ok(())
    }
    pub(super) fn skill_heal(
        &mut self,
        actor: UnitId,
        target: UnitId,
        raw: i32,
        multi: bool,
        eligible: bool,
        events: &mut Vec<Event>,
    ) {
        self.skill_heal_value(actor, target, f64::from(raw), multi, eligible, events);
    }
    pub(super) fn skill_heal_value(
        &mut self,
        actor: UnitId,
        target: UnitId,
        raw: f64,
        multi: bool,
        eligible: bool,
        events: &mut Vec<Event>,
    ) {
        if !self.units[target].alive() {
            return;
        }
        let factor = (1. + self.units[actor].modifier("hp_heal_percent", "")).clamp(
            self.scenario.rules.heal_buff_percent_min,
            self.scenario.rules.heal_buff_percent_max,
        );
        let flat = self.units[actor].modifier("hp_heal_amount", "");
        if self.rounding023() && self.units[target].blueprint.hero {
            let value = (raw + flat).max(0.) * factor;
            if value > 0.
                && self.reaction_block(actor, target, ReactionKind::BlockHeal, None, events)
            {
                return;
            }
            let rules = self.scenario.rules.candidates.healing.clone();
            let critical = eligible
                && self.rng.roll(if multi {
                    rules.multi_chance
                } else {
                    rules.single_chance
                });
            let amount = self
                .healing_received023(target, value * if critical { rules.multiplier } else { 1. });
            self.heal_final023(target, amount, events);
            if critical {
                events.push(Event::CriticalHeal {
                    actor,
                    target,
                    raw: raw.round().clamp(0., 1e6) as i32,
                    amount,
                });
                self.stress_change(target, -rules.target_stress_relief, events);
            }
            return;
        }
        let amount = dd_data::mechanisms023::healing_amount(
            (raw + flat).max(0.) * factor,
            self.units[target].blueprint.hero,
            &self.scenario.rules.content,
        );
        if amount > 0 && self.reaction_block(actor, target, ReactionKind::BlockHeal, None, events) {
            return;
        }
        let rules = self.scenario.rules.candidates.healing.clone();
        let critical = eligible
            && self.rng.roll(if multi {
                rules.multi_chance
            } else {
                rules.single_chance
            });
        let amount = if critical {
            dd_data::mechanisms023::healing_amount(
                f64::from(amount) * rules.multiplier,
                self.units[target].blueprint.hero,
                &self.scenario.rules.content,
            )
        } else {
            amount
        };
        self.heal(target, amount, events);
        if critical {
            events.push(Event::CriticalHeal {
                actor,
                target,
                raw: raw.round().clamp(0., 1e6) as i32,
                amount,
            });
            // A critical heal is not a critical attack: no class crit Buff,
            // piercing bonus or extended Wyrd Reconstruction self-bleed.
            if self.units[target].blueprint.hero {
                self.stress_change(target, -rules.target_stress_relief, events);
            }
        }
    }
    pub(super) fn restoration_tick(
        &mut self,
        target: UnitId,
        amount: i32,
        eligible: bool,
        events: &mut Vec<Event>,
    ) {
        let rules = self.scenario.rules.candidates.healing.clone();
        let critical = eligible && self.rng.roll(rules.single_chance);
        if self.rounding023() && self.units[target].blueprint.hero {
            let healed = self.healing_received023(
                target,
                f64::from(amount) * if critical { rules.multiplier } else { 1. },
            );
            self.heal_final023(target, healed, events);
            if critical {
                events.push(Event::CriticalHeal {
                    actor: target,
                    target,
                    raw: amount,
                    amount: healed,
                });
                self.stress_change(target, -rules.target_stress_relief, events);
            }
            return;
        }
        let healed = if critical {
            dd_data::mechanisms023::healing_amount(
                f64::from(amount) * rules.multiplier,
                self.units[target].blueprint.hero,
                &self.scenario.rules.content,
            )
        } else {
            amount
        };
        self.heal(target, healed, events);
        if critical {
            events.push(Event::CriticalHeal {
                actor: target,
                target,
                raw: amount,
                amount: healed,
            });
            if self.units[target].blueprint.hero {
                self.stress_change(target, -rules.target_stress_relief, events);
            }
        }
    }
    pub(super) fn dispatch_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        e: &dd_data::combat::Effect,
        events: &mut Vec<Event>,
    ) {
        if e.execution.queued {
            self.enqueue_effect(actor, target, e);
        } else {
            self.apply_effect(actor, target, e, events);
        }
    }
    pub(super) fn enqueue_effect(
        &mut self,
        actor: UnitId,
        target: UnitId,
        e: &dd_data::combat::Effect,
    ) {
        let mut effect = e.clone();
        effect.execution.queued = false;
        self.effect_jobs.push_back(QueuedEffect {
            actor,
            target,
            effect,
            critical: self.units[actor].critical_context,
            skill_source: self.lifecycle.skill_effect,
            counter: self.units[actor].counter_context,
            attack_type: self.units[actor].attack_context.clone(),
        });
    }
    /// Drain before reactions/counters and before exposing any decision boundary.
    pub(super) fn flush_effects(&mut self, events: &mut Vec<Event>) {
        let mut processed = 0;
        while let Some(job) = self.effect_jobs.pop_front() {
            processed += 1;
            if processed > traits::MAX_REACTIONS_PER_ACTION {
                self.lifecycle.failure = Some("effect resolution bound exceeded".into());
                self.effect_jobs.clear();
                return;
            }
            let old = (
                self.units[job.actor].critical_context,
                self.lifecycle.skill_effect,
                self.units[job.actor].counter_context,
                self.units[job.actor].attack_context.clone(),
            );
            self.units[job.actor].critical_context = job.critical;
            self.lifecycle.skill_effect = job.skill_source;
            self.units[job.actor].counter_context = job.counter;
            self.units[job.actor].attack_context = job.attack_type;
            self.apply_effect(job.actor, job.target, &job.effect, events);
            self.units[job.actor].critical_context = old.0;
            self.lifecycle.skill_effect = old.1;
            self.units[job.actor].counter_context = old.2;
            self.units[job.actor].attack_context = old.3;
        }
    }
}
