use super::*;

impl Battle {
    pub(super) fn settle_opportunity(&mut self, id: UnitId, events: &mut Vec<Event>) {
        self.complete_turn(id, events);
        self.fire_trinkets(id, id, "turn_end", true, events);
        self.flush_lifecycle(events);
        self.run_bonus_phase(Some(id), false, events);
        self.flush_reactions(events);
        self.flush_lifecycle(events);
    }
    pub(super) fn prepare_entity_desires(&mut self, id: UnitId) {
        if !self.units[id].alive() {
            self.units[id].desire_rolls.clear();
            return;
        }
        self.units[id].desire_rolls = self.units[id]
            .blueprint
            .mechanics
            .brain
            .as_ref()
            .map(|b| vec![true; b.skill_selection_desires.len()])
            .unwrap_or_default();
    }
    pub(super) fn prepare_round(&mut self) {
        for u in &mut self.units {
            u.round_actions = 0;
            if !u.alive() {
                u.desire_rolls.clear();
                continue;
            }
            u.desire_rolls = u
                .blueprint
                .mechanics
                .brain
                .as_ref()
                .map(|b| vec![true; b.skill_selection_desires.len()])
                .unwrap_or_default();
        }
    }
    pub(super) fn roll_round(&mut self) {
        self.lifecycle.invalidating_actions = 0;
        let mut rolls = vec![];
        for (id, u) in self
            .units
            .iter_mut()
            .enumerate()
            .filter(|(_, u)| u.active())
        {
            let count = u.action_count() + u.next_bonus;
            u.round_budget = count;
            u.round_extra = u.next_bonus;
            u.next_bonus = 0;
            for _ in 0..count {
                let speed = u.stats().speed;
                rolls.push((
                    id,
                    speed + self.rng.below(self.scenario.rules.initiative_base as u64) as i32 + 1,
                    speed,
                    self.rng.next(),
                ));
            }
        }
        rolls.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)));
        self.queue = rolls.into_iter().map(|r| r.0).collect();
        if self.round == 1
            && let Some(ctx) = &self.scenario.rules.context
        {
            let surprised = match ctx.surprise {
                dd_data::build010::Surprise::Neither => None,
                dd_data::build010::Surprise::Heroes => Some(true),
                dd_data::build010::Surprise::Monsters => Some(false),
            };
            if let Some(side) = surprised {
                let mut ids = self.queue.drain(..).collect::<Vec<_>>();
                ids.sort_by_key(|id| self.units[*id].blueprint.hero == side);
                self.queue = ids.into();
            }
        }
    }
    pub(super) fn run_bonus_phase(
        &mut self,
        current: Option<UnitId>,
        pre: bool,
        events: &mut Vec<Event>,
    ) {
        let phase = if current.is_none() {
            if pre {
                "is_round_start"
            } else {
                "is_round_finish"
            }
        } else if self.queue.is_empty() {
            "is_round_finish"
        } else {
            "is_round_in_progress"
        };
        for actor in 0..self.units.len() {
            let original_class = self.units[actor].blueprint.id.clone();
            let Some(brain) = self.units[actor].blueprint.mechanics.brain.clone() else {
                continue;
            };
            for (index, entry) in brain.bonus_initiative_desires.iter().enumerate() {
                if self.lifecycle.bonus_used.contains(&(actor, index)) {
                    continue;
                }
                let data = &entry["data"];
                let flag = |k: &str, default: bool| {
                    data.get(k).and_then(|v| v.as_bool()).unwrap_or(default)
                };
                if !flag(if pre { "is_pre_turn" } else { "is_post_turn" }, false)
                    || !flag(phase, false)
                    || !self.units[actor].alive()
                {
                    continue;
                }
                if self.units[actor].stunned
                    && (flag("ignore_if_stun", false) || !flag("is_valid_if_stunned", false))
                {
                    continue;
                }
                let number = |k: &str| data.get(k).and_then(|v| v.as_f64());
                let bounded = |stem: &str, value: f64| {
                    number(&format!("{stem}_min")).is_none_or(|n| value >= n)
                        && number(&format!("{stem}_max")).is_none_or(|n| value <= n)
                };
                let heroes = self
                    .units
                    .iter()
                    .filter(|u| u.alive() && u.blueprint.hero)
                    .count();
                let monsters = self
                    .units
                    .iter()
                    .filter(|u| u.alive() && !u.blueprint.hero)
                    .count();
                if !bounded("heroes", heroes as f64)
                    || !bounded("monsters", monsters as f64)
                    || !bounded("performing_turn", f64::from(self.units[actor].turns))
                    || number("round_min").is_some_and(|n| f64::from(self.round) < n)
                    || number("monsters_size_limit").is_some_and(|n| {
                        self.units
                            .iter()
                            .filter(|u| u.alive() && !u.blueprint.hero)
                            .map(|u| f64::from(u.size()))
                            .sum::<f64>()
                            > n
                    })
                {
                    continue;
                }
                let kind = entry["type"].as_str().unwrap();
                if let Some(threshold) = number("health_ratio_threshold") {
                    let ratio =
                        f64::from(self.units[actor].hp) / f64::from(self.units[actor].stats().hp);
                    if if flag("is_under_threshold", true) {
                        ratio > threshold
                    } else {
                        ratio < threshold
                    } {
                        continue;
                    }
                }
                if !bounded(
                    "performer_health_ratio",
                    f64::from(self.units[actor].hp) / f64::from(self.units[actor].stats().hp),
                ) {
                    continue;
                }
                let qualified = match kind {
                    "actor_dot_count" => {
                        let id = data["actor_dot_entry_id"].as_str().unwrap();
                        let count = self
                            .units
                            .iter()
                            .filter(|u| {
                                u.alive()
                                    && ((u.blueprint.hero && flag("count_heroes", false))
                                        || (!u.blueprint.hero && flag("count_monsters", false)))
                            })
                            .flat_map(|u| &u.parasites)
                            .filter(|p| p.definition.id == id)
                            .count();
                        bounded("actor_dot_count", count as f64)
                    }
                    "trinket" => self.units.iter().any(|u| {
                        u.alive()
                            && u.trinkets.iter().any(|t| {
                                !t.destroyed
                                    && Some(t.definition.id.as_str()) == data["trinket_id"].as_str()
                            })
                    }),
                    "ally_last_damaged" => self.lifecycle.special.last_damaged.is_some_and(|id| {
                        id != actor
                            && self.units[id].blueprint.mechanics.family
                                == data["ally_base_class_id"].as_str().unwrap()
                    }),
                    "death" => false,
                    "guaranteed" => true,
                    "ally_actor_class_count" => {
                        let family = data["ally_base_class_id"].as_str().unwrap();
                        let count = self
                            .units
                            .iter()
                            .enumerate()
                            .filter(|(id, u)| {
                                *id != actor
                                    && u.present()
                                    && (if matches!(u.life, Life::Corpse { .. }) {
                                        if u.size() > 1 {
                                            family == "corpse_large"
                                        } else {
                                            family == "corpse"
                                        }
                                    } else {
                                        u.alive() && u.blueprint.mechanics.family == family
                                    })
                            })
                            .count();
                        bounded("ally_count", count as f64)
                            && number("ally_actor_class_count").is_none_or(|n| count as f64 >= n)
                    }
                    "hp_ratio_threshold" => {
                        let ratio = f64::from(self.units[actor].hp)
                            / f64::from(self.units[actor].stats().hp);
                        let threshold = number("health_ratio_threshold").unwrap();
                        if flag("is_under_threshold", true) {
                            ratio <= threshold
                        } else {
                            ratio >= threshold
                        }
                    }
                    "last_skill" => {
                        self.lifecycle.last_skill.as_deref()
                            == data["last_combat_skill_id"].as_str()
                    }
                    _ => false,
                };
                if !qualified {
                    continue;
                }
                self.lifecycle.bonus_used.insert((actor, index));
                let override_id = data["combat_skill_id_override"].as_str().unwrap_or("");
                let slot = self.units[actor]
                    .blueprint
                    .skills
                    .iter()
                    .position(|s| s.id == override_id);
                if let Some(skill) = slot.filter(|s| {
                    self.units[actor].blueprint.skills[*s]
                        .tactical
                        .boss
                        .user_selected
                }) {
                    if pre {
                        self.lifecycle.failure =
                            Some("pre-turn player selection is unsupported".into());
                        return;
                    }
                    self.lifecycle.special.selection = Some(boss011::Selection { actor, skill });
                    return;
                }
                if flag("decrement_combat_skill_id_override_cooldown", false) {
                    for (index, until) in &mut self.units[actor].cooldowns {
                        if slot.is_none_or(|s| s == *index) {
                            *until = until.saturating_sub(1);
                        }
                    }
                }
                let before_actor = self.actor;
                self.actor = Some(actor);
                let prior_cooldown = if !flag("is_cooldown_enabled", true) {
                    slot.and_then(|s| self.units[actor].cooldowns.remove(&s).map(|n| (s, n)))
                } else {
                    None
                };
                let legal = self.legal_actions();
                let choices=legal.iter().filter(|a|matches!(a.action,Action::Skill{skill,..}|Action::SkillAuto{skill} if slot==Some(skill))).collect::<Vec<_>>();
                let chosen = if override_id.is_empty() {
                    self.monster_action()
                        .map(|a| (!matches!(a, Action::Pass)).then_some(a))
                } else if choices.is_empty() {
                    Ok(None)
                } else if choices.len() == 1 {
                    Ok(Some(choices[0].action.clone()))
                } else if let Some(slot) = slot {
                    self.target_weights(actor, slot, &choices, &brain)
                        .and_then(|weights| {
                            // An override with no eligible target does not fire. It must not
                            // fall back to a different skill or a random unmarked target.
                            if weights.iter().all(|w| *w <= 0.) {
                                Ok(None)
                            } else {
                                self.weighted_index(&weights)
                                    .map(|i| Some(choices[i].action.clone()))
                            }
                        })
                } else {
                    Ok(None)
                };
                let chosen = match chosen {
                    Ok(a) => a,
                    Err(error) => {
                        self.lifecycle.failure = Some(format!("bonus initiative: {error}"));
                        return;
                    }
                };
                if let Some(a) = chosen
                    && let Some(legal) = legal.into_iter().find(|l| l.action == a)
                {
                    self.lifecycle.bonus_used.insert((actor, index));
                    events.push(Event::BonusAction {
                        actor,
                        skill: override_id.into(),
                    });
                    self.resolve_action(actor, legal, events);
                    self.flush_lifecycle(events);
                    self.flush_reactions(events);
                }
                if let Some((s, n)) = prior_cooldown {
                    self.units[actor].cooldowns.insert(s, n);
                }
                self.actor = before_actor;
                if self.units[actor].blueprint.id != original_class {
                    break;
                }
            }
        }
    }
}
