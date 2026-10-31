use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Control {
    pub controller: UnitId,
    pub remaining: u32,
    pub applied_turn: u32,
}
impl Battle {
    pub(super) fn control_hero(
        &mut self,
        actor: UnitId,
        target: UnitId,
        remaining: u32,
        events: &mut Vec<Event>,
    ) -> bool {
        if self.units[actor]
            .blueprint
            .mechanics
            .boss
            .controller_stress
            .is_none()
        {
            self.lifecycle.failure = Some("control requires original controller contract".into());
            return false;
        }
        if !self.units[actor].alive()
            || !self.units[target].friendly()
            || !self.units[target].active()
            || self.units[target].captive.is_some()
            || self
                .units
                .iter()
                .filter(|u| u.active() && u.friendly())
                .count()
                < 2
            || self.order[1]
                .iter()
                .filter(|id| self.units[**id].present())
                .map(|id| self.units[*id].size())
                .sum::<u8>()
                + self.units[target].size()
                > 4
        {
            return false;
        }
        self.clear_guards(target, true, true, events);
        self.order[0].retain(|id| *id != target);
        self.order[1].retain(|id| self.units[*id].present());
        self.order[1].push(target);
        self.units[target].controlled = Some(Control {
            controller: actor,
            remaining,
            applied_turn: self.units[target].turns,
        });
        events.push(Event::Controlled {
            hero: target,
            controller: actor,
            remaining,
        });
        self.refresh_context(true);
        true
    }
    fn release_control(&mut self, id: UnitId, events: &mut Vec<Event>) {
        let Some(c) = self.units[id].controlled.take() else {
            return;
        };
        self.clear_guards(id, true, true, events);
        self.order[1].retain(|other| *other != id);
        self.order[0].retain(|other| self.units[*other].present());
        if self.units[id].alive() {
            self.order[0].push(id);
        }
        events.push(Event::ControlReleased {
            hero: id,
            controller: c.controller,
        });
        self.refresh_context(true);
    }
    pub(super) fn cleanup_control(&mut self, events: &mut Vec<Event>) {
        let ids = self
            .units
            .iter()
            .enumerate()
            .filter_map(|(id, u)| {
                u.controlled
                    .as_ref()
                    .filter(|c| !u.alive() || !self.units[c.controller].alive() || c.remaining == 0)
                    .map(|_| id)
            })
            .collect::<Vec<_>>();
        for id in ids {
            self.release_control(id, events);
        }
    }
    pub(super) fn control_turn_end(&mut self, id: UnitId, events: &mut Vec<Event>) {
        if let Some(c) = self.units[id].controlled.clone() {
            let stress = self.units[c.controller]
                .blueprint
                .mechanics
                .boss
                .controller_stress
                .unwrap();
            if self.units[id].turns > c.applied_turn {
                self.stress_change(id, stress, events);
                self.units[id].controlled.as_mut().unwrap().remaining = self.units[id]
                    .controlled
                    .as_ref()
                    .unwrap()
                    .remaining
                    .saturating_sub(1);
            }
        }
        self.cleanup_control(events);
    }
    pub(super) fn controlled_action(&mut self) -> Result<Action> {
        let mut skills = BTreeMap::<usize, Vec<Action>>::new();
        for a in self.legal_actions() {
            if let Action::Skill { skill, .. } | Action::SkillAuto { skill } = a.action {
                skills.entry(skill).or_default().push(a.action);
            }
        }
        if skills.is_empty() {
            return Ok(Action::Pass);
        }
        let keys = skills.keys().copied().collect::<Vec<_>>();
        let skill = keys[self.rng.below(keys.len() as u64) as usize];
        let choices = &skills[&skill];
        Ok(choices[self.rng.below(choices.len() as u64) as usize].clone())
    }
    pub(super) fn validate_control(&self) -> Result<()> {
        for (id, u) in self.units.iter().enumerate() {
            if let Some(c) = &u.controlled {
                let controller = self
                    .units
                    .get(c.controller)
                    .ok_or("invalid checkpoint controller ID")?;
                if !u.blueprint.hero
                    || !u.alive()
                    || u.captive.is_some()
                    || !(1..=10000).contains(&c.remaining)
                    || c.applied_turn > u.turns
                    || !controller.alive()
                    || controller.blueprint.hero
                    || controller
                        .blueprint
                        .mechanics
                        .boss
                        .controller_stress
                        .is_none()
                    || !self.order[1].contains(&id)
                {
                    return Err("invalid checkpoint control relationship".into());
                }
            }
        }
        Ok(())
    }
}
