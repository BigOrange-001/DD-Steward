//! User W-029 candidate: keep float factors until the final combat quantity.
use super::*;
impl Battle {
    pub(super) fn rounding023(&self) -> bool {
        dd_data::mechanisms023::enabled(
            &self.scenario.rules.content,
            dd_data::mechanisms023::ROUNDING,
        )
    }
    pub(super) fn stress_float023(&mut self, target: UnitId, raw: f64, events: &mut Vec<Event>) {
        if !self.rounding023() || (raw >= 0. && self.scenario.monsters.is_empty()) {
            self.stress_change(target, raw.round().clamp(-1e6, 1e6) as i32, events);
            return;
        }
        if !self.units[target].alive() || !self.units[target].blueprint.hero {
            return;
        }
        let multiplier = if raw > 0. {
            (1. + self.units[target].modifier("stress_dmg_received_percent", "")
                + self.units[target]
                    .blueprint
                    .mechanics
                    .hero
                    .difficulty
                    .as_ref()
                    .map_or(0., |d| d.stress_modifier))
            .max(self.scenario.rules.stress.stress_dmg_buff_min)
        } else {
            1.
        };
        self.apply_stress_delta(
            target,
            dd_data::mechanisms023::floor_final023(raw * multiplier).clamp(-1e6, 1e6) as i32,
            events,
        );
    }
    pub(super) fn damage_quant023(&self, actor: UnitId, target: UnitId, raw: f64) -> i32 {
        let rounded = if self.units[actor].friendly() && !self.units[target].friendly() {
            dd_data::mechanisms023::ceil_final023(raw)
        } else if !self.units[actor].friendly() && self.units[target].friendly() {
            dd_data::mechanisms023::floor_final023(raw)
        } else {
            raw.round()
        };
        rounded.clamp(0., 1e6) as i32
    }
    pub(super) fn damage_bounds023(
        &self,
        actor: UnitId,
        target: UnitId,
        s: &Skill,
        extra: &[Buff],
    ) -> [f64; 2] {
        let (base, skill_factor) = s
            .tactical
            .source_damage023
            .unwrap_or((s.damage.unwrap_or([0, 0]), 1.));
        let u = &self.units[actor];
        let enemy = &self.units[target];
        let modifier = |kind, sub| {
            u.modifier_against(
                kind,
                sub,
                Some(&s.id),
                Some(enemy),
                extra,
                Some(&s.tactical.attack_type),
            )
        };
        let mut result = ["damage_low", "damage_high"].map(|sub| {
            let i = usize::from(sub == "damage_high");
            let factor = 1. + modifier("combat_stat_multiply", sub);
            let factor = if factor.abs() <= 8. * f64::EPSILON {
                0.
            } else {
                factor.max(0.)
            };
            let stat = ((f64::from(base[i]) * skill_factor + modifier("combat_stat_add", sub))
                * factor)
                .clamp(0., 1e6);
            (stat * (1. + f64::from(enemy.burn) * s.tactical.dmg_per_burn_stack)).clamp(0., 1e6)
        });
        result[1] = result[1].max(result[0]);
        result
    }
    pub(super) fn damage_base023(
        &mut self,
        actor: UnitId,
        target: UnitId,
        s: &Skill,
        extra: &[Buff],
        critical: Option<bool>,
    ) -> f64 {
        let bounds = self.damage_bounds023(actor, target, s, extra);
        if critical == Some(true) {
            return bounds[1]
                * if self.units[actor].blueprint.hero {
                    self.scenario.rules.player_crit_multiplier_table[0].value
                } else {
                    self.scenario.rules.monster_crit_multiplier_table[0].value
                };
        }
        let base = s
            .tactical
            .source_damage023
            .map_or(s.damage.unwrap_or([0, 0]), |v| v.0);
        let roll = self.rng.range(base);
        if base[0] == base[1] {
            bounds[0]
        } else {
            bounds[0]
                + (bounds[1] - bounds[0]).max(0.) * f64::from(roll - base[0])
                    / f64::from(base[1] - base[0])
        }
    }
    pub(super) fn damage_value023(
        &self,
        actor: UnitId,
        target: UnitId,
        s: &Skill,
        base: f64,
    ) -> i32 {
        if self.units[target]
            .blueprint
            .mechanics
            .boss
            .restrictions
            .no_direct_damage
        {
            return 0;
        }
        let rank = 4usize.saturating_sub(self.rank(target) as usize).min(3);
        let base = if s.tactical.boss.rank_damage != [0.; 4] {
            (base * (1. + s.tactical.boss.rank_damage[rank])).clamp(0., 1e6)
        } else {
            base
        };
        let protection = if s.tactical.ignore_protection
            || self.units[actor].modifier_for("ignore_protection", "", Some(&s.id)) > 0.
        {
            0.
        } else {
            self.units[target]
                .effective_stats_in(Some(&self.units[actor]), Some(s))
                .protection
                .min(self.scenario.rules.candidates.limits.protection)
                .clamp(0., 1.)
        };
        // Preserve the existing protection-stage minimum, including exact zero.
        let protected = if base <= 8. * f64::EPSILON {
            0.
        } else {
            (base * (1. - protection)).max(1.)
        };
        let received = (1.
            + self.units[target].modifier_against(
                "damage_received_percent",
                "",
                Some(&s.id),
                Some(&self.units[actor]),
                &[],
                Some(&s.tactical.attack_type),
            ))
        .max(0.);
        self.damage_quant023(actor, target, protected * received)
    }
    pub(super) fn direct_damage_float023(
        &mut self,
        actor: UnitId,
        target: UnitId,
        s: &Skill,
        base: f64,
        events: &mut Vec<Event>,
    ) -> i32 {
        let amount = self.damage_value023(actor, target, s, base);
        if amount > 0 && self.units[target].blocks > 0 {
            self.units[target].blocks -= 1;
            if self.units[target].blocks == 0 {
                self.units[target].blocks_timer = None;
            }
            events.push(Event::DamageBlocked {
                target,
                remaining: self.units[target].blocks,
            });
            0
        } else {
            amount
        }
    }
}
