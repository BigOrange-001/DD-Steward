use super::*;

impl Battle {
    /// Host environment update, outside a policy action. No inventory bypass is
    /// exposed in Observation; player torch use must be a validated item action.
    pub(super) fn torch_bounds(&self) -> [u8; 2] {
        // Nightmare light is a documented candidate; normal Boss bounds come from the original records.
        if self.scenario.rules.flashback.is_some()
            || self.scenario.rules.farm.is_some()
            || self.scenario.rules.bloodlight.is_some()
        {
            return [100, 100];
        }
        self.units
            .iter()
            .filter(|u| u.alive())
            .filter_map(|u| u.blueprint.mechanics.boss.torch)
            .fold([0, 100], |a, b| [a[0].max(b[0]), a[1].min(b[1])])
    }
    pub fn set_torch(&mut self, torch: u8) -> Result<Vec<Event>> {
        let [min, max] = self.torch_bounds();
        if !(min..=max).contains(&torch) || self.outcome.is_some() {
            return Err("invalid lighting boundary/value".into());
        }
        let light = self
            .scenario
            .rules
            .lighting
            .as_mut()
            .ok_or("missing battle lighting context")?;
        let before = light.torch;
        light.torch = torch;
        self.refresh_context(false);
        Ok(vec![Event::Light {
            before,
            after: torch,
        }])
    }

    pub(super) fn refresh_context(&mut self, history: bool) {
        let [min, max] = self.torch_bounds();
        if min > max {
            self.lifecycle.failure = Some("incompatible encounter torch bounds".into());
            return;
        }
        if let Some(light) = self.scenario.rules.lighting.as_mut() {
            light.torch = light.torch.clamp(min, max);
        }
        if history {
            self.refresh_rank_history();
        }
        let ranks = (0..self.units.len())
            .map(|id| self.rank(id))
            .collect::<Vec<_>>();
        let guarded = (0..self.units.len())
            .map(|id| self.guarder(id).is_some())
            .collect::<Vec<_>>();
        let mut counts = BTreeMap::<String, u32>::new();
        for u in self.units.iter().filter(|u| u.alive() && !u.blueprint.hero) {
            for tag in &u.blueprint.mechanics.enemy_types {
                *counts.entry(tag.clone()).or_default() += 1;
            }
        }
        let auras = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.alive())
            .map(|(id, u)| {
                (
                    id,
                    u.blueprint.hero,
                    u.blueprint.mechanics.lifecycle.aura_heroes.clone(),
                    u.blueprint.mechanics.lifecycle.aura_allies.clone(),
                )
            })
            .collect::<Vec<_>>();
        let companions = (0..self.units.len())
            .map(|id| self.companion_modifiers(id))
            .collect::<Vec<_>>();
        let burning = self.units.iter().any(|u| u.present() && u.burn > 0);
        for (id, u) in self.units.iter_mut().enumerate() {
            u.aura_modifiers = auras
                .iter()
                .flat_map(|(source, side, heroes, allies)| {
                    if u.blueprint.hero != *side {
                        heroes.clone()
                    } else if id != *source {
                        allies.clone()
                    } else {
                        vec![]
                    }
                })
                .collect();
            u.aura_modifiers.extend(companions[id].clone());
            u.equipment_modifiers = crate::trinkets010::equipment(u);
            u.light_modifiers = self
                .scenario
                .rules
                .lighting
                .as_ref()
                .map(|l| l.buffs(u.blueprint.hero).expect("validated lighting"))
                .unwrap_or_default();
            u.region_context = self
                .scenario
                .rules
                .context
                .as_ref()
                .map(|c| c.region.clone());
            u.corridor_context = self
                .scenario
                .rules
                .context
                .as_ref()
                .is_some_and(|c| c.corridor);
            u.walking_backwards_context = self
                .scenario
                .rules
                .context
                .as_ref()
                .is_some_and(|c| c.walking_backwards);
            u.light_context = self.scenario.rules.lighting.as_ref().map(|c| c.torch);
            u.items_context = self
                .supplies
                .iter()
                .filter(|s| s.remaining > 0)
                .map(|s| s.definition.id.clone())
                .collect();
            u.monster_counts = counts.clone();
            u.quest_uses_context = self
                .scenario
                .rules
                .context
                .as_ref()
                .map(|c| c.quest_uses_remaining.clone())
                .unwrap_or_default();
            for item in &u.trinkets {
                if !item.destroyed
                    && let Some(n) = item.quest_remaining
                {
                    u.quest_uses_context.insert(item.definition.id.clone(), n);
                }
            }
            u.rank_context = ranks[id];
            u.round_context = self.round;
            u.guarded_context = guarded[id];
            u.any_burning_context = burning;
        }
        self.refresh_shared_health();
        for u in &mut self.units {
            let clear = u
                .buffs
                .iter()
                .filter(|b| u.buff_active(&b.definition))
                .flat_map(|b| b.definition.activation.clear_buffs_on_rule_active.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let removed = u
                .buffs
                .iter()
                .filter(|b| {
                    clear.contains(&b.definition.id)
                        || (b.definition.activation.remove_if_not_active
                            && !u.buff_active(&b.definition))
                })
                .map(|b| b.definition.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            u.buffs.retain(|b| !removed.contains(&b.definition.id));
            if u.alive() {
                u.hp = u.hp.min(u.stats().hp);
            }
        }
        self.refresh_shared_health();
    }
    pub(super) fn shuffle_party(&mut self, id: UnitId, events: &mut Vec<Event>) {
        let side = usize::from(!self.units[id].friendly());
        self.order[side].retain(|id| self.units[*id].present());
        let before = self.order[side]
            .iter()
            .map(|id| (*id, self.rank(*id)))
            .collect::<Vec<_>>();
        let mut start = 0;
        for end in 0..=before.len() {
            if end == before.len() || self.units[before[end].0].immobilized.is_some() {
                for i in (start + 1..end).rev() {
                    let j = start + self.rng.below((i - start + 1) as u64) as usize;
                    self.order[side].swap(i, j);
                }
                start = end + 1;
            }
        }
        self.refresh_context(true);
        for (unit, old_rank) in before {
            let rank = self.rank(unit);
            if old_rank != rank {
                events.push(Event::Move { actor: unit, rank });
                if self.units[unit].blueprint.hero {
                    for other in 0..self.units.len() {
                        if other != unit && self.units[other].blueprint.hero {
                            self.queue_reaction(
                                other,
                                dd_data::mechanics::ReactionKind::CommentMove,
                                unit,
                            );
                        }
                    }
                }
            }
        }
    }
    pub(super) fn executed_targets(
        &mut self,
        actor: UnitId,
        s: &Skill,
        mut targets: Vec<UnitId>,
    ) -> Vec<UnitId> {
        if s.effects.iter().any(|e| {
            matches!(
                e.kind,
                EffectKind::Boss(dd_data::special::BossOp::ClearRanks(_))
            ) && e.target == EffectTarget::Performer
        }) && let Some(rs) = self
            .lifecycle
            .special
            .marked_ranks
            .get(&actor)
            .filter(|rs| !rs.is_empty())
        {
            return self
                .targets(actor, s)
                .into_iter()
                .filter(|id| self.occupied_ranks(*id).any(|r| rs.contains(&r)))
                .collect();
        }
        if s.target.random {
            let index = self.rng.below(targets.len() as u64) as usize;
            targets = vec![targets[index]];
        }
        if s.tactical.extra_targets_count > 0 && self.rng.roll(s.tactical.extra_targets_chance) {
            let mut candidates = self
                .targets(actor, s)
                .into_iter()
                .filter(|id| {
                    !targets.contains(id)
                        && (self.units[*id].alive()
                            || self.units[*id]
                                .blueprint
                                .corpse
                                .as_ref()
                                .is_some_and(|c| c.random_targetable))
                })
                .collect::<Vec<_>>();
            for _ in 0..s.tactical.extra_targets_count {
                if candidates.is_empty() {
                    break;
                }
                let i = self.rng.below(candidates.len() as u64) as usize;
                targets.push(candidates.remove(i));
            }
        }
        targets
    }
    pub(super) fn redirect_target(
        &mut self,
        _actor: UnitId,
        mut selected: UnitId,
        all: &[UnitId],
        s: &Skill,
        events: &mut Vec<Event>,
    ) -> UnitId {
        if !s.target.friendly && all.len() == 1 {
            let chance =
                self.units[_actor].modifier_for("random_target_attack_chance", "", Some(&s.id));
            if chance > 0. && self.rng.roll(chance.clamp(0., 1.)) {
                let choices = self.targets(_actor, s);
                if !choices.is_empty() {
                    selected = choices[self.rng.below(choices.len() as u64) as usize];
                }
            }
        }
        if !s.target.friendly
            && !s.tactical.ignore_guard
            && let Some(protector) = self.guarder(selected)
            && (all.len() == 1 || !all.contains(&protector))
        {
            events.push(Event::GuardIntercept {
                selected,
                target: protector,
            });
            return protector;
        }
        selected
    }
    pub(super) fn direct_damage_amount(
        &mut self,
        actor: UnitId,
        target: UnitId,
        s: &Skill,
        base: i32,
        events: &mut Vec<Event>,
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
        let protection = if s.tactical.ignore_protection
            || self.units[actor].modifier_for("ignore_protection", "", Some(&s.id)) > 0.
        {
            0.
        } else {
            self.units[target]
                .effective_stats_in(Some(&self.units[actor]), Some(s))
                .protection
                .min(self.scenario.rules.candidates.limits.protection)
        };
        let rank = 4usize.saturating_sub(self.rank(target) as usize);
        let base = if s.tactical.boss.rank_damage != [0.; 4] {
            (f64::from(base) * (1. + s.tactical.boss.rank_damage[rank.min(3)]))
                .round()
                .clamp(0., 1000000.) as i32
        } else {
            base
        };
        let raw = candidate_damage(base, protection);
        let factor = (1.
            + self.units[target].modifier_against(
                "damage_received_percent",
                "",
                Some(&s.id),
                Some(&self.units[actor]),
                &[],
                Some(&s.tactical.attack_type),
            ))
        .max(0.);
        let amount = (f64::from(raw) * factor).round().clamp(0., 1e6) as i32;
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
    pub(super) fn reflect_damage(
        &mut self,
        actor: UnitId,
        target: UnitId,
        amount: i32,
        events: &mut Vec<Event>,
    ) {
        if amount > 0 && self.units[actor].alive() && self.units[target].alive() {
            let factor = self.units[target].modifier_against(
                "damage_reflect_percent",
                "",
                None,
                Some(&self.units[actor]),
                &[],
                self.units[actor].attack_context.as_deref(),
            ) + self.units[target].modifier_against(
                "damage_reflect_percent",
                "reflected_dmg",
                None,
                Some(&self.units[actor]),
                &[],
                self.units[actor].attack_context.as_deref(),
            );
            let raw = f64::from(amount) * factor.max(0.);
            let reflected = if self.rounding023() {
                self.damage_quant023(target, actor, raw)
            } else {
                raw.round().clamp(0., 1e6) as i32
            };
            if reflected > 0 {
                self.damage(actor, reflected, false, events);
            }
        }
    }
    pub(super) fn counter_attack(
        &mut self,
        actor: UnitId,
        target: UnitId,
        hit: bool,
        incoming: &Skill,
        events: &mut Vec<Event>,
    ) {
        if !incoming.tactical.can_be_riposted
            || !self.units[actor].alive()
            || self.units[actor].stunned
            || !self.units[target].alive()
        {
            return;
        }
        let Some(state) = self.units[actor].riposte.as_ref() else {
            return;
        };
        let chance = if hit {
            state.hit_chance
        } else {
            state.miss_chance
        };
        let Some(skill) = self.units[actor]
            .blueprint
            .mechanics
            .riposte
            .as_deref()
            .cloned()
        else {
            return;
        };
        if !self.overlaps_ranks(actor, &skill.launch)
            || !self.overlaps_ranks(target, &skill.target.ranks)
            || !self.rng.roll(chance)
        {
            return;
        }
        events.push(Event::CounterAttack { actor, target });
        let selected = if skill.target.multi {
            self.targets(actor, &skill)
        } else {
            vec![target]
        };
        let selected = self.executed_targets(actor, &skill, selected);
        self.execute_skill(
            actor,
            AttackRun {
                skill,
                targets: selected,
                slot: None,
                counter: true,
            },
            events,
        );
    }
    pub(super) fn clear_guards(
        &mut self,
        id: UnitId,
        guarding: bool,
        guarded: bool,
        events: &mut Vec<Event>,
    ) {
        for protector in 0..self.units.len() {
            if self.units[protector]
                .guard
                .as_ref()
                .is_some_and(|g| (guarding && protector == id) || (guarded && g.ward == id))
            {
                let ward = self.units[protector].guard.take().unwrap().ward;
                events.push(Event::GuardCleared { protector, ward });
            }
        }
    }
    pub(super) fn cleanup_tactics(&mut self, events: &mut Vec<Event>) {
        for id in 0..self.units.len() {
            let invalid = self.units[id].guard.as_ref().is_some_and(|g| {
                !self.units[id].alive()
                    || self.units[id].stunned
                    || g.remaining == 0
                    || !self
                        .units
                        .get(g.ward)
                        .is_some_and(|u| u.alive() && u.modifier("guard_blocked", "") <= 0.)
            });
            if invalid {
                self.clear_guards(id, true, false, events);
            }
            if !self.units[id].alive() {
                self.units[id].riposte = None;
                self.units[id].blocks = 0;
                self.units[id].blocks_timer = None;
            }
            if !self.units[id].present() {
                self.units[id].burn = 0;
            }
        }
    }
    pub(super) fn apply_tactical(
        &mut self,
        actor: UnitId,
        id: UnitId,
        e: &dd_data::combat::Effect,
        events: &mut Vec<Event>,
    ) -> bool {
        match &e.kind {
            EffectKind::Burn(n) => {
                let value = (f64::from(*n)
                    * (1. + self.units[actor].modifier("hp_dot_burn_amount_percent", "")).max(0.))
                .round() as i32;
                self.units[id].burn = self.units[id].burn.saturating_add(value).min(1000000);
                let wildfire = self.units[actor]
                    .buffs
                    .iter()
                    .map(|b| &b.definition)
                    .chain(&self.units[actor].equipment_modifiers)
                    .any(|b| {
                        b.stat_type == "wildfire_apply_adjacent_targets"
                            && self.units[actor].buff_active(b)
                    });
                if wildfire {
                    let before = self.rank(id).saturating_sub(1);
                    let after = self.rank(id) + self.units[id].size();
                    let adjacent = (0..self.units.len())
                        .filter(|other| {
                            *other != id
                                && self.units[*other].present()
                                && self.units[*other].friendly() == self.units[id].friendly()
                                && self
                                    .occupied_ranks(*other)
                                    .any(|r| r == before || r == after)
                        })
                        .collect::<Vec<_>>();
                    for other in adjacent {
                        self.units[other].burn =
                            self.units[other].burn.saturating_add(value).min(1000000);
                    }
                }
            }
            EffectKind::RefreshSkills => {
                for (slot, skill) in self.units[id].blueprint.skills.clone().iter().enumerate() {
                    if skill.per_battle_limit.is_some() {
                        self.units[id].skill_remaining[slot] = skill.per_battle_limit;
                        self.units[id].used_skills.remove(&skill.id);
                    }
                }
            }
            EffectKind::Regen(n) => {
                let remaining = (f64::from(e.duration)
                    * (1. + self.units[id].modifier("hp_heal_dot_duration_received_percent", ""))
                        .max(0.))
                .round()
                .clamp(1., 10000.) as u32;
                self.units[id].dots.push(Dot {
                    can_crit_heal: e
                        .execution
                        .allows_critical_heal(self.lifecycle.skill_effect, true),
                    amount: *n,
                    remaining,
                    kind: e.kind.clone(),
                });
            }
            EffectKind::HealPercent(n) => {
                let amount = f64::from(self.units[id].stats().hp) * n;
                self.skill_heal_value(
                    actor,
                    id,
                    amount,
                    matches!(
                        e.target,
                        EffectTarget::Party | EffectTarget::PartyOther | EffectTarget::TargetParty
                    ),
                    e.execution
                        .allows_critical_heal(self.lifecycle.skill_effect, false),
                    events,
                );
            }
            EffectKind::Cure {
                bleed,
                poison,
                burn,
            } => {
                self.units[id].dots.retain(|d| {
                    !((*bleed && matches!(d.kind, EffectKind::Bleed(_)))
                        || (*poison && matches!(d.kind, EffectKind::Blight(_))))
                });
                if *burn {
                    self.units[id].burn = 0;
                }
            }
            EffectKind::Unstun => self.units[id].stunned = false,
            EffectKind::ClearMark => self.units[id].marked = None,
            EffectKind::ClearBuffs { positive } => {
                self.units[id].buffs.retain(|b| {
                    b.scope != BuffScope::Timed
                        || (!*positive && !b.clearable)
                        || b.debuff == *positive
                });
                self.units[id].hp = self.units[id].hp.min(self.units[id].stats().hp);
            }
            EffectKind::KillCorpses => {
                if matches!(self.units[id].life, Life::Corpse { .. }) {
                    self.units[id].life = Life::Dead;
                    self.units[id].hp = 0;
                    self.units[id].burn = 0;
                    events.push(Event::Death(id));
                } else {
                    return false;
                }
            }
            EffectKind::ShuffleParty => self.shuffle_party(id, events),
            EffectKind::Guard { reverse } => {
                let (protector, ward) = if *reverse { (id, actor) } else { (actor, id) };
                if self.units[actor].modifier("disable_combat_skill_attribute", "guard") > 0. {
                    return false;
                }
                if protector == ward
                    || !self.units[protector].alive()
                    || !self.units[ward].alive()
                    || self.units[protector].friendly() != self.units[ward].friendly()
                    || self.units[protector].stunned
                    || self.units[ward].modifier("guard_blocked", "") > 0.
                    || !self.units[ward]
                        .blueprint
                        .mechanics
                        .lifecycle
                        .can_be_guarded
                {
                    return false;
                }
                self.clear_guards(protector, true, true, events);
                self.clear_guards(ward, true, true, events);
                let duration = (f64::from(e.duration)
                    * (1. + self.units[protector].modifier("guard_duration_percent", "")).max(0.))
                .round()
                .clamp(1., 10000.) as u32;
                self.units[protector].guard = Some(GuardState {
                    ward,
                    remaining: duration,
                    applied_turn: self.units[protector].turns,
                });
                events.push(Event::GuardEstablished { protector, ward });
            }
            EffectKind::ClearGuard { guarding, guarded } => {
                self.clear_guards(id, *guarding, *guarded, events)
            }
            EffectKind::Riposte { hit, miss, buffs } => {
                if self.units[id].blueprint.mechanics.riposte.is_none() {
                    return false;
                }
                self.units[id].riposte = Some(RiposteState {
                    permanent: false,
                    remaining: e.duration,
                    applied_turn: self.units[id].turns,
                    hit_chance: *hit,
                    miss_chance: *miss,
                    buffs: buffs.clone(),
                });
            }
            EffectKind::ClearRiposte => self.units[id].riposte = None,
            EffectKind::Blocks { count, turns } => {
                self.units[id].blocks = self.units[id]
                    .blocks
                    .saturating_add(*count)
                    .min(self.scenario.rules.health_damage_blocks_max);
                self.units[id].blocks_timer = turns.map(|n| (n, self.units[id].turns));
            }
            _ => unreachable!("tactical operation dispatched explicitly"),
        }
        self.cleanup_tactics(events);
        self.refresh_context(true);
        true
    }
    pub(super) fn complete_turn(&mut self, id: UnitId, events: &mut Vec<Event>) {
        if self.units[id].alive() && self.units[id].burn > 0 {
            let amount = self.units[id].burn;
            self.damage(id, amount, false, events);
            if self.units[id].present() {
                let decay = (0.5
                    * (1. + self.units[id].modifier("hp_dot_burn_decay_percent", "")).max(0.))
                .clamp(0., 1.);
                self.units[id].burn = (f64::from(amount) * (1. - decay)).floor() as i32;
            }
        }
        self.finish_turn(id);
        if self.units[id]
            .immobilized
            .as_ref()
            .is_some_and(|d| d.until.is_some_and(|n| n <= self.units[id].turns))
        {
            self.units[id].immobilized = None;
        }
        let u = &mut self.units[id];
        if let Some(g) = &mut u.guard
            && g.applied_turn < u.turns
        {
            g.remaining = g.remaining.saturating_sub(1);
        }
        if let Some(r) = &mut u.riposte {
            if !r.permanent && r.applied_turn < u.turns {
                r.remaining = r.remaining.saturating_sub(1);
            }
            if r.remaining == 0 {
                u.riposte = None;
            }
        }
        if let Some((remaining, applied)) = &mut u.blocks_timer {
            if *applied < u.turns {
                *remaining = remaining.saturating_sub(1);
            }
            if *remaining == 0 {
                u.blocks = 0;
                u.blocks_timer = None;
            }
        }
        self.control_turn_end(id, events);
        self.cleanup_tactics(events);
        self.refresh_context(true);
    }
}
