use super::*;
use dd_data::{
    ai::{boolean, number, string},
    mechanics::Desire,
};

impl Battle {
    pub(super) fn guarder(&self, ward: UnitId) -> Option<UnitId> {
        self.units
            .iter()
            .enumerate()
            .find(|(_, u)| {
                u.alive()
                    && !u.stunned
                    && u.guard
                        .as_ref()
                        .is_some_and(|g| g.ward == ward && g.remaining > 0)
            })
            .map(|(id, _)| id)
            .or_else(|| {
                self.units
                    .iter()
                    .enumerate()
                    .find(|(id, u)| {
                        *id != ward
                            && u.alive()
                            && !u.stunned
                            && u.friendly() == self.units[ward].friendly()
                            && u.blueprint.mechanics.boss.modes.iter().any(|m| {
                                u.mode.as_ref() == Some(&m.id)
                                    && m.guard_families
                                        .contains(&self.units[ward].blueprint.mechanics.family)
                            })
                    })
                    .map(|(id, _)| id)
            })
    }
    pub(super) fn ai_predicates(&self, actor: UnitId, d: &Desire) -> bool {
        let u = &self.units[actor];
        if dd_data::ai::boolean(d, "is_valid_when_corpse_enabled").is_some_and(|expected| {
            expected
                != self
                    .scenario
                    .rules
                    .context
                    .as_ref()
                    .is_none_or(|c| c.corpses)
        }) {
            return false;
        }
        if dd_data::ai::boolean(d, "first_initiative_only") == Some(true) && u.round_actions != 1 {
            return false;
        }
        let heroes = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, v)| v.blueprint.hero && v.alive())
            .collect::<Vec<_>>();
        let monsters = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.blueprint.hero && v.alive())
            .collect::<Vec<_>>();
        for (key, value) in &d.data.predicates {
            if key.ends_with("_min") || key.ends_with("_max") {
                let base = key.trim_end_matches("_min").trim_end_matches("_max");
                let actual = match base {
                    "heroes" => heroes.len() as f64,
                    "control_count" => {
                        self.units.iter().filter(|u| u.controlled.is_some()).count() as f64
                    }
                    "non_actor_dot_heroes" => heroes
                        .iter()
                        .filter(|(_, u)| u.parasites.is_empty())
                        .count() as f64,
                    "monsters" => monsters.len() as f64,
                    "monsters_size" => monsters
                        .iter()
                        .filter(|(_, v)| !v.blueprint.mechanics.boss.restrictions.no_brain_size)
                        .map(|(_, v)| f64::from(v.size()))
                        .sum(),
                    "marked_heroes" => {
                        heroes.iter().filter(|(_, v)| v.marked.is_some()).count() as f64
                    }
                    "non_marked_heroes" => {
                        heroes.iter().filter(|(_, v)| v.marked.is_none()).count() as f64
                    }
                    "non_stunned_heroes" => {
                        heroes.iter().filter(|(_, v)| !v.stunned).count() as f64
                    }
                    "non_deaths_door_heroes" => {
                        heroes.iter().filter(|(_, v)| v.hp > 0).count() as f64
                    }
                    "virtued_heroes" => heroes
                        .iter()
                        .filter(|(_, v)| matches!(v.resolve, Resolve::Virtuous(_)))
                        .count() as f64,
                    "guarded_monsters" => monsters
                        .iter()
                        .filter(|(id, u)| {
                            !u.blueprint.mechanics.boss.restrictions.no_brain_guard
                                && self.guarder(*id).is_some()
                        })
                        .count() as f64,
                    "non_guard_monsters" => monsters
                        .iter()
                        .filter(|(id, u)| {
                            !u.blueprint.mechanics.boss.restrictions.no_brain_guard
                                && self.guarder(*id).is_none()
                        })
                        .count() as f64,
                    "target_stress_heroes" => heroes
                        .iter()
                        .filter(|(_, v)| {
                            f64::from(v.stress) >= number(d, "target_stress_min").unwrap_or(0.)
                        })
                        .count() as f64,
                    "target_stress" => continue,
                    "performing_turn" => f64::from(u.turns),
                    "performer_rounds_in_rank" => f64::from(u.rank_turns),
                    "performer_health_ratio" => f64::from(u.hp) / f64::from(u.stats().hp),
                    "hp_ratio" => continue, // ally_hp_ratio_skill applies these to its matching ally.
                    _ => continue,
                };
                let expected = value.as_f64().unwrap();
                if (key.ends_with("_min") && actual < expected)
                    || (key.ends_with("_max") && actual > expected)
                {
                    return false;
                }
            }
        }
        if boolean(d, "is_guarding").is_some_and(|v| v != u.guard.is_some()) {
            return false;
        }
        if boolean(d, "is_guarded").is_some_and(|v| v != self.guarder(actor).is_some()) {
            return false;
        }
        true
    }
    pub(super) fn ai_effect_skill(&self, actor: UnitId, s: &Skill, d: &Desire) -> bool {
        let status = string(d, "effect_key_status").unwrap();
        s.effects.iter().any(|e| {
            e.conditions
                .iter()
                .any(|c| matches!(c,dd_data::combat::Condition::Status(v) if v==status))
        }) && self
            .targets(actor, s)
            .iter()
            .any(|id| self.units[*id].has_status(status))
    }
    pub(super) fn ai_ally_skill(&self, actor: UnitId, d: &Desire) -> bool {
        self.units.iter().enumerate().any(|(id, u)| {
            id != actor
                && u.friendly() == self.units[actor].friendly()
                && Some(u.blueprint.mechanics.family.as_str()) == string(d, "ally_base_class_id")
                && if d.kind == "ally_dead_skill" {
                    !u.alive()
                } else {
                    let ratio = f64::from(u.hp) / f64::from(u.stats().hp);
                    u.alive()
                        && ratio >= number(d, "hp_ratio_min").unwrap_or(0.)
                        && ratio <= number(d, "hp_ratio_max").unwrap_or(1.)
                }
        })
    }
    pub(super) fn ai_dot_skill(&self, actor: UnitId, d: &Desire) -> bool {
        let u = &self.units[actor];
        if dd_data::ai::boolean(d, "is_valid_when_corpse_enabled").is_some_and(|expected| {
            expected
                != self
                    .scenario
                    .rules
                    .context
                    .as_ref()
                    .is_none_or(|c| c.corpses)
        }) {
            return false;
        }
        let mut wanted = vec![];
        for (key, status) in [("has_bleed", "bleeding"), ("has_poison", "poisoned")] {
            if boolean(d, key) == Some(true) {
                wanted.push(u.has_status(status));
            }
        }
        if boolean(d, "has_all_dots").unwrap_or(false) {
            wanted.into_iter().all(|v| v)
        } else {
            wanted.into_iter().any(|v| v)
        }
    }
    pub(super) fn ai_target_predicates(&self, target: UnitId, d: &Desire) -> bool {
        let u = &self.units[target];
        if d.data.can_target_not_overstressed
            || d.data.can_target_afflicted
            || d.data.can_target_virtued
        {
            let allowed = match u.resolve {
                Resolve::Calm => d.data.can_target_not_overstressed,
                Resolve::Afflicted(_) => d.data.can_target_afflicted,
                Resolve::Virtuous(_) => d.data.can_target_virtued,
            };
            if !allowed {
                return false;
            }
        }
        if boolean(d, "is_guarded").is_some_and(|v| v != self.guarder(target).is_some())
            || boolean(d, "is_guarding").is_some_and(|v| v != u.guard.is_some())
        {
            return false;
        }
        if boolean(d, "can_target_deaths_door") == Some(false) && u.blueprint.hero && u.hp == 0 {
            return false;
        }
        if boolean(d, "can_target_last_hero") == Some(false)
            && u.blueprint.hero
            && self
                .units
                .iter()
                .filter(|u| u.blueprint.hero && u.alive())
                .count()
                == 1
        {
            return false;
        }
        if boolean(d, "can_target_stunned") == Some(false) && u.stunned {
            return false;
        }
        if let Some(ranks) = d
            .data
            .predicates
            .get("valid_ranks")
            .map(|value| dd_data::ai::rank_values(value).expect("validated AI ranks"))
            && !self.occupied_ranks(target).any(|r| ranks.contains(&r))
        {
            return false;
        }
        true
    }
    pub(super) fn ai_resistance(&self, target: UnitId, d: &Desire) -> f64 {
        let s = self.units[target].stats();
        match string(d, "resistance_type_id").unwrap() {
            "bleed" => s.bleed_resist,
            "poison" => s.blight_resist,
            "stun" => s.stun_resist,
            "move" => s.move_resist,
            "debuff" => s.debuff_resist,
            "death_blow" => s.death_resist,
            "disease" => {
                self.units[target].blueprint.mechanics.disease_resist
                    + self.units[target].modifier("resistance", "disease")
            }
            _ => unreachable!("validated AI resistance"),
        }
    }
}
