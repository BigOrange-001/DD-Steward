//! Shared health belongs to the encounter; each body retains its own defenses,
//! status clock, skill and stable identity. Never collapse the four hit targets.
use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Runtime {
    pub pools: Vec<Pool>,
    pub marked_ranks: BTreeMap<UnitId, Vec<u8>>,
    pub last_damaged: Option<UnitId>,
    pub selection: Option<Selection>,
    pub dream: crate::nightmare011::Runtime,
    pub farm: Option<crate::farm011::Runtime>,
    pub killed: BTreeMap<UnitId, (String, String)>,
    pub escaped: BTreeMap<UnitId, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub actor: UnitId,
    pub skill: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Pool {
    pub id: String,
    pub members: Vec<UnitId>,
    pub hp: i32,
    pub max: i32,
    pub base_max: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedHealthView {
    pub id: String,
    pub members: Vec<UnitId>,
    pub hp: i32,
    pub max_hp: i32,
}
impl Battle {
    /// Physical shared-body deaths for quest goals, separate from one paid kill.
    pub(crate) fn shared_goal_kills020(&self) -> Vec<(UnitId, String, String)> {
        self.lifecycle
            .special
            .pools
            .iter()
            .filter(|p| p.hp == 0 && p.members.iter().all(|id| !self.units[*id].alive()))
            .flat_map(|p| p.members.iter())
            .filter(|id| !self.lifecycle.special.killed.contains_key(id))
            .map(|id| {
                (
                    *id,
                    self.units[*id].source_id.clone(),
                    self.units[*id].blueprint.id.clone(),
                )
            })
            .collect()
    }
    pub(super) fn initialize_shared_health(&mut self) -> Result<()> {
        self.lifecycle.special.pools.clear();
        for u in &mut self.units {
            u.shared_max = None;
            u.shared_base_max = None;
        }
        let mut groups = BTreeMap::<String, Vec<UnitId>>::new();
        for (id, u) in self.units.iter().enumerate().filter(|(_, u)| u.alive()) {
            if let Some(name) = &u.blueprint.mechanics.boss.shared_health {
                groups.entry(name.clone()).or_default().push(id);
            }
        }
        for (name, members) in groups {
            if members.len() != 4
                || members.iter().any(|id| {
                    self.units[*id].blueprint.hero
                        || self.units[*id].size() != 1
                        || self.units[*id].blueprint.corpse.is_some()
                })
                || self.order[1].len() != 4
            {
                return Err(
                    "shared-health encounter requires four independent one-cell bodies".into(),
                );
            }
            let reference = &self.units[members[0]].blueprint;
            if members
                .iter()
                .any(|id| self.units[*id].blueprint.stats.hp != reference.stats.hp)
            {
                return Err("incompatible shared-health difficulty variants".into());
            }
            for id in &members {
                for form in &self.units[*id]
                    .blueprint
                    .mechanics
                    .boss
                    .shifter
                    .as_ref()
                    .ok_or("shared body missing shifter")?
                    .forms
                {
                    let next = self
                        .lifecycle
                        .catalog
                        .get(&form.choice.class)
                        .ok_or("unresolved shared form")?;
                    if next.hero
                        || next.size != 1
                        || next.corpse.is_some()
                        || next.mechanics.boss.shared_health.as_ref() != Some(&name)
                        || next.stats.hp != reference.stats.hp
                    {
                        return Err("incompatible shared form definition".into());
                    }
                }
            }
            let base_max = members
                .iter()
                .map(|id| self.units[*id].blueprint.stats.hp)
                .sum::<i32>();
            let max = members
                .iter()
                .map(|id| self.units[*id].individual_stats_in(None, None).hp)
                .sum::<i32>();
            if !(1..=1000000).contains(&max) {
                return Err("shared-health maximum exceeds bound".into());
            }
            for id in &members {
                self.units[*id].shared_base_max = Some(base_max);
                self.units[*id].shared_max = Some(max);
                self.units[*id].hp = max;
            }
            self.lifecycle.special.pools.push(Pool {
                id: name,
                members,
                hp: max,
                max,
                base_max,
            });
        }
        self.refresh_context(false);
        self.validate_shared_health()
    }
    pub(super) fn refresh_shared_health(&mut self) {
        for pool in &mut self.lifecycle.special.pools {
            let max = pool
                .members
                .iter()
                .map(|id| self.units[*id].individual_stats_in(None, None).hp)
                .sum::<i32>();
            if !(1..=1000000).contains(&max) {
                self.lifecycle.failure = Some("invalid shared-health effective maximum".into());
                return;
            }
            pool.max = max;
            pool.hp = pool.hp.min(max);
            for id in &pool.members {
                self.units[*id].shared_max = Some(max);
                self.units[*id].shared_base_max = Some(pool.base_max);
                self.units[*id].hp = pool.hp;
            }
        }
    }
    pub(super) fn validate_shared_health(&self) -> Result<()> {
        for u in &self.units {
            if (u.blueprint.hero && u.source_id != u.blueprint.id)
                || (!u.blueprint.hero && !self.lifecycle.catalog.contains_key(&u.source_id))
            {
                return Err("invalid checkpoint stable source class".into());
            }
        }
        for (id, (source, last)) in &self.lifecycle.special.killed {
            let u = self.units.get(*id).ok_or("invalid killed entity ID")?;
            if u.blueprint.hero
                || source != &u.source_id
                || !self.lifecycle.catalog.contains_key(last)
            {
                return Err("invalid killed entity handoff".into());
            }
        }
        for (id, source) in &self.lifecycle.special.escaped {
            let u = self.units.get(*id).ok_or("invalid escaped entity ID")?;
            if u.blueprint.hero
                || u.alive()
                || source != &u.source_id
                || u.blueprint.mechanics.lifecycle.lifetime.is_none()
                    && !crate::execution025::shrieker_escape_source(self, *id)
            {
                return Err("invalid escaped entity handoff".into());
            }
        }
        if let Some(p) = &self.lifecycle.special.selection
            && (self.outcome.is_some()
                || self.actor != Some(p.actor)
                || !self.units.get(p.actor).is_some_and(|u| {
                    u.alive()
                        && u.blueprint
                            .skills
                            .get(p.skill)
                            .is_some_and(|s| s.tactical.boss.user_selected)
                })
                || self.sacrifice_actions().is_empty())
        {
            return Err("invalid checkpoint player selection".into());
        }
        if let Some(id) = self.lifecycle.special.last_damaged
            && id >= self.units.len()
        {
            return Err("invalid last damage reference".into());
        }
        let mut used = std::collections::BTreeSet::new();
        for pool in &self.lifecycle.special.pools {
            if pool.id != "formless"
                || pool.members.len() != 4
                || !(0..=pool.max).contains(&pool.hp)
                || !(1..=1000000).contains(&pool.max)
            {
                return Err("invalid checkpoint shared-health pool".into());
            }
            for id in &pool.members {
                let u = self.units.get(*id).ok_or("invalid shared body ID")?;
                if !used.insert(*id)
                    || u.blueprint.hero
                    || u.blueprint.size != 1
                    || u.blueprint.mechanics.boss.shared_health.as_ref() != Some(&pool.id)
                    || u.hp != pool.hp
                    || u.shared_max != Some(pool.max)
                    || u.shared_base_max != Some(pool.base_max)
                    || (pool.hp > 0) != u.alive()
                    || matches!(u.life, Life::Corpse { .. })
                {
                    return Err("invalid checkpoint shared body/cache".into());
                }
            }
            if pool.base_max
                != pool
                    .members
                    .iter()
                    .map(|id| self.units[*id].blueprint.stats.hp)
                    .sum::<i32>()
                || pool.max
                    != pool
                        .members
                        .iter()
                        .map(|id| self.units[*id].individual_stats_in(None, None).hp)
                        .sum::<i32>()
            {
                return Err("invalid checkpoint shared maximum".into());
            }
        }
        for (id, u) in self.units.iter().enumerate() {
            if (u.shared_max.is_some()
                || u.shared_base_max.is_some()
                || (u.alive() && u.blueprint.mechanics.boss.shared_health.is_some()))
                && !used.contains(&id)
            {
                return Err("orphan shared-health unit".into());
            }
        }
        Ok(())
    }
    pub(super) fn shared_damage(
        &mut self,
        target: UnitId,
        amount: i32,
        crit: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(index) = self
            .lifecycle
            .special
            .pools
            .iter()
            .position(|p| p.members.contains(&target))
        else {
            return false;
        };
        if amount <= 0 || !self.units[target].alive() {
            return true;
        }
        events.push(Event::Damage {
            target,
            amount,
            crit,
        });
        let pool = &mut self.lifecycle.special.pools[index];
        pool.hp = pool.hp.saturating_sub(amount).max(0);
        let hp = pool.hp;
        let max_hp = pool.max;
        let group = pool.id.clone();
        let members = pool.members.clone();
        for id in &members {
            self.units[*id].hp = hp;
        }
        events.push(Event::SharedHealth { group, hp, max_hp });
        if hp == 0 {
            for (part, id) in members.iter().enumerate() {
                self.units[*id].loot &= part == 0;
                if part == 0 {
                    self.lifecycle.special.killed.insert(
                        *id,
                        (
                            self.units[*id].source_id.clone(),
                            self.units[*id].blueprint.id.clone(),
                        ),
                    );
                }
                self.retire(*id, events);
                self.schedule_death(*id, crit, events);
            }
        }
        self.cleanup_tactics(events);
        self.refresh_context(true);
        true
    }
    pub(super) fn shared_heal(
        &mut self,
        target: UnitId,
        amount: i32,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(pool) = self
            .lifecycle
            .special
            .pools
            .iter_mut()
            .find(|p| p.members.contains(&target))
        else {
            return false;
        };
        let amount = amount.max(0).min(pool.max - pool.hp);
        if pool.hp == 0 {
            return true;
        }
        pool.hp += amount;
        for id in &pool.members {
            self.units[*id].hp = pool.hp;
        }
        events.push(Event::Heal { target, amount });
        events.push(Event::SharedHealth {
            group: pool.id.clone(),
            hp: pool.hp,
            max_hp: pool.max,
        });
        true
    }
    pub(super) fn shape_shift_round(&mut self, events: &mut Vec<Event>) {
        let ids = self
            .lifecycle
            .special
            .pools
            .iter()
            .flat_map(|p| p.members.iter().copied())
            .collect::<Vec<_>>();
        for id in ids {
            if !self.units[id].alive() {
                continue;
            }
            let Some(spec) = self.units[id].blueprint.mechanics.boss.shifter.clone() else {
                continue;
            };
            if !self.round.is_multiple_of(spec.frequency) {
                continue;
            }
            let rank = self.rank(id);
            let forms = spec
                .forms
                .iter()
                .filter(|f| f.ranks.contains(&rank) && f.choice.weight > 0.)
                .collect::<Vec<_>>();
            let Ok(index) =
                self.weighted_index(&forms.iter().map(|f| f.choice.weight).collect::<Vec<_>>())
            else {
                self.lifecycle.failure = Some("no eligible shape-shifter form".into());
                return;
            };
            let change = dd_data::lifecycle::ChangeSpec {
                choices: vec![forms[index].choice.clone()],
                reset_hp: false,
                reset_buffs: false,
                clear_initiative: false,
                clear_cooldowns: false,
            };
            // A single chosen candidate avoids a second random selection and keeps the
            // ordinary class-change bookkeeping for statuses, cooldowns and identity.
            let next = self.lifecycle.catalog[&change.choices[0].class].clone();
            if !self.apply_entity_change(id, next, &change, events) {
                self.lifecycle.failure = Some("shape-shifter class change failed".into());
                return;
            }
        }
        self.refresh_context(true);
    }
}

impl Battle {
    pub(super) fn sacrifice_actions(&self) -> Vec<LegalAction> {
        let Some(p) = &self.lifecycle.special.selection else {
            return vec![];
        };
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.active() && u.blueprint.hero)
            .map(|(target, _)| LegalAction {
                action: Action::Sacrifice { target },
                label: self.units[p.actor].blueprint.skills[p.skill].id.clone(),
                targets: vec![target],
                hit_chance: 1.,
                damage: [0, 0],
                heal: [0, 0],
                stress_heal: 0,
            })
            .collect()
    }
    pub(super) fn resolve_sacrifice(&mut self, action: Action) -> Result<Vec<Event>> {
        let legal = self
            .sacrifice_actions()
            .into_iter()
            .find(|a| a.action == action)
            .ok_or("illegal sacrifice selection")?;
        let selection = self.lifecycle.special.selection.take().unwrap();
        let skill = self.units[selection.actor].blueprint.skills[selection.skill].clone();
        let mut events = vec![Event::BonusAction {
            actor: selection.actor,
            skill: skill.id.clone(),
        }];
        self.actor = None;
        self.execute_skill(
            selection.actor,
            AttackRun {
                skill,
                targets: legal.targets,
                slot: Some(selection.skill),
                counter: false,
            },
            &mut events,
        );
        self.flush_lifecycle(&mut events);
        self.flush_reactions(&mut events);
        self.run_bonus_phase(Some(selection.actor), false, &mut events);
        self.advance(&mut events);
        Ok(events)
    }
}
