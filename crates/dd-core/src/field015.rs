//! Reuse the hero settlement kernel outside initiative. No synthetic enemies,
//! combat turns, battle-start triggers or victory are created for exploration.
use super::*;
pub(super) struct PartyKernel {
    pub core: Battle,
}
impl PartyKernel {
    pub fn new(heroes: Vec<Blueprint>, rules: Rules, rng: Rng) -> Result<Self> {
        Self::with_hp_policy(heroes, rules, rng, false)
    }
    /// A source-valued host Buff was added or expired. Existing HP is clamped to
    /// the new native maximum; ordinary inputs keep strict initial-HP validation.
    pub fn after_host_buff_change(heroes: Vec<Blueprint>, rules: Rules, rng: Rng) -> Result<Self> {
        Self::with_hp_policy(heroes, rules, rng, true)
    }
    fn with_hp_policy(
        heroes: Vec<Blueprint>,
        rules: Rules,
        rng: Rng,
        clamp_existing_hp: bool,
    ) -> Result<Self> {
        if heroes.is_empty() || heroes.iter().any(|b| !b.hero) {
            return Err("field settlement requires living hero inputs".into());
        }
        rules.validate()?;
        let content_id = rules.content.id()?;
        let count = heroes.len();
        let units = heroes.iter().map(Unit::fresh).collect();
        let mut b = Battle {
            profile: PROFILE.into(),
            content_id,
            scenario: Scenario {
                heroes,
                monsters: vec![],
                rules,
                round_limit: 1,
            },
            units,
            order: [(0..count).collect(), vec![]],
            queue: VecDeque::new(),
            actor: None,
            round: 0,
            rng,
            outcome: None,
            effect_jobs: VecDeque::new(),
            pending_reactions: VecDeque::new(),
            retreat_attempts: 0,
            lifecycle: Default::default(),
            supply_reserves: BTreeMap::new(),
            supplies: vec![],
        };
        b.refresh_context(false);
        let mut events = vec![];
        for id in 0..count {
            b.units[id].mode = b.units[id].blueprint.mechanics.hero.initial_mode.clone();
            if clamp_existing_hp {
                b.initialize_entity_with_hp_policy(id, &mut events, true);
            } else {
                b.initialize_entity(id, &mut events);
            }
        }
        b.refresh_context(false);
        if let Some(e) = b.lifecycle.failure.take() {
            return Err(e);
        }
        Ok(Self { core: b })
    }
    pub fn exploration_buffs(&mut self, defs: &BTreeMap<String, serde_json::Value>) -> Result<()> {
        for id in 0..self.core.units.len() {
            for name in self.core.units[id]
                .blueprint
                .mechanics
                .build
                .noncombat_buffs
                .clone()
            {
                let raw = dd_data::library::unique_json(defs, &name, "exploration Buff")?;
                if ["in_camp", "in_town", "in_activity"]
                    .contains(&raw["rule_type"].as_str().unwrap_or(""))
                {
                    continue;
                }
                if [
                    "food_consumption_percent",
                    "starving_damage_percent",
                    "resolve_xp_bonus_percent",
                    "scouting_chance",
                    "party_surprise_chance",
                    "monsters_surprise_chance",
                ]
                .contains(&raw["stat_type"].as_str().unwrap_or(""))
                    || (raw["stat_type"] == "hp_heal_received_percent"
                        && raw["stat_sub_type"] == "eat")
                    || (raw["stat_type"] == "stress_dmg_received_percent"
                        && raw["stat_sub_type"] == "hunger")
                {
                    let b = dd_data::mechanics::read_exploration_buff(defs, &name)?;
                    self.core.add_buff(id, b, BuffScope::Build, &mut vec![]);
                }
            }
        }
        self.core.refresh_context(false);
        Ok(())
    }
    pub fn camping_buffs(&mut self, defs: &BTreeMap<String, serde_json::Value>) -> Result<()> {
        for id in 0..self.core.units.len() {
            for name in self.core.units[id]
                .blueprint
                .mechanics
                .build
                .noncombat_buffs
                .clone()
            {
                let raw = dd_data::library::unique_json(defs, &name, "camping Buff")?;
                if raw["rule_type"] == "in_camp" || raw["stat_sub_type"] == "camping_eat" {
                    let b = dd_data::mechanics::read_camping_buff(defs, &name)?;
                    self.core.add_buff(id, b, BuffScope::Build, &mut vec![]);
                }
            }
        }
        self.core.refresh_context(false);
        Ok(())
    }
    pub fn settle(&mut self) -> Result<()> {
        let mut events = vec![];
        self.core.flush_effects(&mut events);
        self.core.flush_reactions(&mut events);
        self.core.flush_lifecycle(&mut events);
        self.core.refresh_context(false);
        if let Some(e) = self.core.lifecycle.failure.take() {
            return Err(e);
        }
        Ok(())
    }
    pub fn stress(&mut self, id: usize, n: i32) -> Result<()> {
        self.core.stress_change(id, n, &mut vec![]);
        self.settle()
    }
    pub fn abandon_stress(&mut self, id: usize, amount: i32, allow_resolve: bool) -> Result<()> {
        self.core
            .apply_stress_delta_mode(id, amount, &mut vec![], allow_resolve);
        self.settle()
    }
    pub fn hp(&mut self, id: usize, n: i32) -> Result<()> {
        if n >= 0 {
            self.core.heal(id, n, &mut vec![]);
        } else {
            self.core.damage_unrestricted(id, -n, false, &mut vec![]);
        }
        self.settle()
    }
    pub fn maximum(&self, id: usize) -> i32 {
        self.core.units[id].stats().hp
    }
    pub fn modifier(&self, id: usize, k: &str, sub: &str) -> f64 {
        self.core.units[id].modifier(k, sub)
    }
    pub fn curio_effect(&mut self, id: usize, e: &dd_data::curios013::CurioEffect) -> Result<bool> {
        let u = &self.core.units[id];
        if !u.alive() {
            return Err("dead field target".into());
        }
        let resist = if e.bleed > 0 {
            u.stats().bleed_resist
        } else if e.poison > 0 {
            u.stats().blight_resist
        } else {
            0.
        };
        if !self.core.rng.roll((e.chance - resist).clamp(0., 1.)) {
            return Ok(false);
        }
        self.stress(id, e.stress)?;
        if e.cure {
            self.core.units[id]
                .dots
                .retain(|d| !matches!(d.kind, EffectKind::Bleed(_) | EffectKind::Blight(_)));
        }
        self.hp(id, e.heal)?;
        for (kind, n) in [
            (EffectKind::Bleed(e.bleed), e.bleed),
            (EffectKind::Blight(e.poison), e.poison),
        ] {
            if n > 0 && self.core.units[id].alive() {
                self.core.units[id].dots.push(Dot {
                    kind,
                    amount: n,
                    remaining: e.duration,
                    can_crit_heal: false,
                });
            }
        }
        self.settle()?;
        Ok(true)
    }
    pub fn walk_dot(&mut self) -> Result<()> {
        for id in 0..self.core.units.len() {
            if !self.core.units[id].alive() {
                continue;
            }
            let dots = self.core.units[id].dots.clone();
            for d in dots {
                if !self.core.units[id].alive() {
                    break;
                }
                match d.kind {
                    EffectKind::Bleed(_) | EffectKind::Blight(_) => self.hp(id, -d.amount)?,
                    EffectKind::Horror(_) => self.stress(id, d.amount)?,
                    EffectKind::Regen(_) => self.hp(id, d.amount)?,
                    _ => return Err("unsupported field dot".into()),
                }
            }
            for d in &mut self.core.units[id].dots {
                d.remaining = d.remaining.saturating_sub(1);
            }
            self.core.units[id].dots.retain(|d| d.remaining > 0);
            if self.core.units[id].burn > 0 {
                let n = self.core.units[id].burn;
                self.hp(id, -n)?;
                self.core.units[id].burn /= 2;
            }
        }
        self.settle()
    }
    pub fn export(self) -> (Vec<HeroResult>, Rng) {
        let mut states = self.core.hero_results();
        for (state, u) in states.iter_mut().zip(&self.core.units) {
            state.persistent_buffs = u
                .buffs
                .iter()
                .filter(|b| {
                    b.host_instance.is_none()
                        && matches!(b.scope, BuffScope::Timed | BuffScope::Recovery)
                })
                .map(|b| dd_data::hero010::CarryBuff {
                    definition: b.definition.clone(),
                    remaining: b.remaining,
                })
                .collect();
        }
        (states, self.core.rng)
    }
}

