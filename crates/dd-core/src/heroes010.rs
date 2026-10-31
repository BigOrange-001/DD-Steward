use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct RankBurn {
    pub hero: bool,
    pub rank: u8,
    pub amount: i32,
    pub expires: u32,
}
impl Battle {
    pub(super) fn resolve_weight(&self, id: UnitId, t: &dd_data::mechanics::TraitDef) -> f64 {
        let h = &self.units[id].blueprint.mechanics.hero;
        if t.required_dungeon_history.iter().any(|tag| {
            !h.dungeon_history.contains(tag)
                && !self
                    .scenario
                    .rules
                    .context
                    .as_ref()
                    .is_some_and(|c| c.history.contains(tag))
        }) {
            return 0.;
        }
        let count = self.units[id]
            .resolve_history
            .get(&t.id)
            .copied()
            .unwrap_or(0) as usize;
        let increments = &self.scenario.rules.affliction_history_increment;
        let base = if t.virtue {
            self.scenario.rules.base_virtue_history_wf
        } else {
            self.scenario.rules.base_affliction_history_wf
        };
        (base + t.generate_chance_modifier + increments[count.min(increments.len() - 1)]).max(0.)
    }

    pub(super) fn stall_finish(&mut self, events: &mut Vec<Event>) {
        let Some(ctx) = self.scenario.rules.context.clone().filter(|c| c.stall) else {
            return;
        };
        let heroes = self
            .units
            .iter()
            .filter(|u| u.alive() && u.blueprint.hero)
            .count() as u32;
        let foes = self
            .units
            .iter()
            .filter(|u| {
                u.alive()
                    && !u.blueprint.hero
                    && !u.blueprint.mechanics.boss.restrictions.no_stall_count
            })
            .collect::<Vec<_>>();
        let eligible = heroes >= ctx.stall_party
            && !foes.is_empty()
            && foes.len() as u32 <= ctx.stall_enemy
            && !foes
                .iter()
                .any(|u| u.blueprint.mechanics.lifecycle.disable_stall)
            && self.lifecycle.invalidating_actions < ctx.stall_skills;
        let accelerated = foes
            .iter()
            .all(|u| u.blueprint.mechanics.lifecycle.accelerate_stall);
        let grace = if foes
            .iter()
            .any(|u| u32::from(u.size()) >= ctx.stall_extra_size)
        {
            ctx.stall_extra_amount
        } else {
            0
        };
        if eligible {
            let increment = if accelerated {
                if self.lifecycle.stall_count > 0 {
                    ctx.stall_consecutive
                } else {
                    ctx.stall_accelerated
                }
            } else {
                1
            };
            self.lifecycle.stall_count = self
                .lifecycle
                .stall_count
                .saturating_add(increment)
                .min(1000000);
            if !accelerated && self.lifecycle.stall_count >= ctx.stall_stress_round + grace {
                let targets = (0..self.units.len())
                    .filter(|id| self.units[*id].alive() && self.units[*id].blueprint.hero)
                    .collect::<Vec<_>>();
                for id in targets {
                    self.apply_effect(id, id, &ctx.stall_effect, events);
                }
            }
            if self.lifecycle.stall_count >= ctx.stall_summon_round + grace
                && let Some(actor) = (0..self.units.len())
                    .find(|id| self.units[*id].alive() && !self.units[*id].blueprint.hero)
            {
                for bp in ctx.reinforcements {
                    let spec = dd_data::lifecycle::SummonSpec {
                        choices: vec![dd_data::lifecycle::Choice {
                            class: bp.id,
                            weight: 1.,
                            limit: None,
                        }],
                        count: 1,
                        ranks: vec![],
                        roll_initiative: false,
                        replace_corpse: false,
                        loot: false,
                    };
                    self.summon(actor, &spec, events);
                }
            }
            if self.lifecycle.stall_count >= ctx.stall_reset + grace {
                self.lifecycle.stall_count = 0;
            }
        } else {
            self.lifecycle.stall_count = 0;
        }
    }

    pub(super) fn mode_forced(&mut self, id: UnitId, ending: bool, events: &mut Vec<Event>) {
        let name = self.units[id]
            .blueprint
            .mechanics
            .hero
            .modes
            .iter()
            .find(|m| Some(&m.id) == self.units[id].mode.as_ref())
            .and_then(|m| {
                if ending {
                    m.end_skill.clone()
                } else {
                    m.affliction_skill.clone()
                }
            });
        if let Some(name) = name
            && let Some(skill) = self.units[id]
                .blueprint
                .mechanics
                .hero
                .forced
                .get(&name)
                .cloned()
        {
            self.execute_skill(
                id,
                AttackRun {
                    skill,
                    targets: vec![id],
                    slot: None,
                    counter: false,
                },
                events,
            );
        }
    }

    pub(super) fn hero_opportunity(&mut self, id: UnitId, events: &mut Vec<Event>) {
        self.tick_diseases(id, events);
        self.lifecycle.zones.retain(|z| z.expires > self.round);
        let burn = self
            .lifecycle
            .zones
            .iter()
            .filter(|z| {
                z.hero == self.units[id].blueprint.hero
                    && self.occupied_ranks(id).any(|r| r == z.rank)
            })
            .map(|z| z.amount)
            .sum::<i32>();
        self.units[id].burn = self.units[id].burn.saturating_add(burn).min(1000000);
        let regen = self.units[id]
            .modifier("hp_dot_heal", "")
            .round()
            .clamp(0., 1e6) as i32;
        if regen > 0 {
            self.heal(id, regen, events);
        }
        let stress = self.units[id]
            .blueprint
            .mechanics
            .hero
            .modes
            .iter()
            .find(|m| Some(&m.id) == self.units[id].mode.as_ref())
            .map_or(0, |m| m.stress);
        if stress > 0 {
            let stress = (f64::from(stress)
                * (1. + self.units[id].modifier("stress_dmg_received_percent", "mode")).max(0.))
            .round() as i32;
            self.stress_change(id, stress, events);
        }
    }
    pub(super) fn hero_door(&mut self, id: UnitId, events: &mut Vec<Event>) {
        let h = self.units[id].blueprint.mechanics.hero.clone();
        if self.units[id]
            .door_reaction_round
            .is_none_or(|r| self.round >= r + h.enter_cooldown)
        {
            self.units[id].door_reaction_round = Some(self.round);
            for e in h.enter_door {
                self.apply_effect(id, id, &e, events);
            }
        }
    }
    pub(super) fn hero_death(&mut self, id: UnitId, events: &mut Vec<Event>) {
        self.units[id].parasites.clear();
        self.trinket_death(id, events);
        for r in self.units[id].blueprint.mechanics.hero.death.clone() {
            let targets = (0..self.units.len())
                .filter(|other| {
                    self.units[*other].alive()
                        && if self.units[*other].blueprint.hero == self.units[id].blueprint.hero {
                            r.allies
                        } else {
                            r.enemies
                        }
                })
                .collect::<Vec<_>>();
            for target in targets {
                for e in &r.effects {
                    self.apply_effect(id, target, e, events);
                }
            }
        }
    }
    pub(super) fn hero_hp_reactions(&mut self, events: &mut Vec<Event>) {
        for id in 0..self.units.len() {
            if !self.units[id].alive() {
                continue;
            }
            let hp = f64::from(self.units[id].hp) / f64::from(self.units[id].stats().hp);
            for (index, r) in self.units[id]
                .blueprint
                .mechanics
                .hero
                .low_hp
                .clone()
                .into_iter()
                .enumerate()
            {
                let active = if r.under { hp < r.ratio } else { hp > r.ratio };
                let previous = self.units[id].hp_reaction_active[index];
                self.units[id].hp_reaction_active[index] = active;
                if active && !previous {
                    for e in r.effects {
                        self.apply_effect(id, id, &e, events);
                    }
                }
            }
        }
    }
}
