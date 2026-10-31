use super::*;
use dd_data::nightmare011::FlashOp;
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Runtime {
    pub ended: Option<bool>,
    pub horror: BTreeMap<UnitId, Dot>,
    pub rewards: Vec<(String, String)>,
}
impl Battle {
    pub(super) fn is_required_dream_hero(&self, id: UnitId) -> bool {
        self.scenario
            .rules
            .flashback
            .as_ref()
            .is_some_and(|f| f.required.contains(&self.units[id].blueprint.id))
            && self.lifecycle.special.dream.ended.is_none()
    }
    pub(super) fn initialize_flashback(&mut self, events: &mut Vec<Event>) -> Result<()> {
        let Some(f) = self.scenario.rules.flashback.clone() else {
            return Ok(());
        };
        if f.required.iter().any(|class| {
            !self
                .units
                .iter()
                .any(|u| u.blueprint.hero && u.alive() && &u.blueprint.id == class)
        }) {
            return Err("missing required flashback hero class".into());
        }
        for id in 0..self.scenario.heroes.len() {
            if !f.required.contains(&self.units[id].blueprint.id) {
                continue;
            }
            if matches!(self.units[id].resolve, Resolve::Virtuous(_)) && !f.allow_virtue {
                return Err("virtuous required hero cannot enter this flashback".into());
            }
            let floor = (f64::from(self.units[id].stats().hp) * f.min_health).ceil() as i32;
            self.heal(id, (floor - self.units[id].hp).max(0), events);
            for e in &f.starting {
                match e.kind {
                    EffectKind::Horror(n) | EffectKind::ProtectedHorror(n) => {
                        self.lifecycle.special.dream.horror.insert(
                            id,
                            Dot {
                                can_crit_heal: false,
                                amount: n,
                                remaining: e.duration,
                                kind: e.kind.clone(),
                            },
                        );
                    }
                    _ => self.apply_effect(id, id, e, events),
                }
            }
        }
        Ok(())
    }
    pub(super) fn flashback_tick(&mut self, id: UnitId, events: &mut Vec<Event>) {
        if let Some(mut dot) = self.lifecycle.special.dream.horror.remove(&id)
            && self.units[id].alive()
        {
            self.stress_change(id, dot.amount, events);
            dot.remaining -= 1;
            if dot.remaining > 0 {
                self.lifecycle.special.dream.horror.insert(id, dot);
            }
        }
    }
    pub(super) fn settle_flashback(&mut self, events: &mut Vec<Event>) {
        let Some(f) = self.scenario.rules.flashback.clone() else {
            return;
        };
        if self.lifecycle.special.dream.ended.is_some() {
            return;
        }
        let failed = self.outcome == Some(Outcome::Retreated)
            || self.units.iter().any(|u| {
                u.blueprint.hero
                    && f.required.contains(&u.blueprint.id)
                    && (!u.alive() || u.hp == 0)
            });
        let won = !self.units.iter().any(|u| u.alive() && !u.blueprint.hero);
        if !failed && !won {
            return;
        }
        self.lifecycle.special.dream.ended = Some(!failed);
        self.lifecycle.special.dream.horror.clear();
        if failed {
            for id in self.order[1].clone() {
                self.retire(id, events);
            }
            self.outcome = Some(Outcome::FlashbackFailed);
        }
        for r in f.results.into_iter().filter(|r| r.success != failed) {
            if let FlashOp::Reward { kind, id } = &r.operation {
                self.lifecycle
                    .special
                    .dream
                    .rewards
                    .push((kind.clone(), id.clone()));
                continue;
            }
            for id in 0..self.scenario.heroes.len() {
                if !self.units[id].alive() {
                    continue;
                }
                let required = f.required.contains(&self.units[id].blueprint.id);
                if (r.target == "required_hero" && !required)
                    || (r.target == "not_required_hero" && required)
                {
                    continue;
                }
                match &r.operation {
                    FlashOp::HealthFloor(p) => {
                        let min = (f64::from(self.units[id].stats().hp) * p).ceil() as i32;
                        self.heal(id, (min - self.units[id].hp).max(0), events);
                    }
                    FlashOp::StressCap(n) => {
                        let amount = (self.units[id].stress - *n).max(0);
                        self.stress_change(id, -amount, events);
                    }
                    FlashOp::Buff(b) => self.add_buff(id, b.clone(), BuffScope::Timed, events),
                    FlashOp::Reward { .. } => {}
                }
            }
        }
        self.refresh_context(true);
    }
    pub(super) fn validate_flashback(&self) -> Result<()> {
        let state = &self.lifecycle.special.dream;
        if self.scenario.rules.flashback.is_none() {
            if state != &Runtime::default() {
                return Err("orphan flashback state".into());
            }
            return Ok(());
        }
        let definition = self.scenario.rules.flashback.as_ref().unwrap();
        for (id, d) in &state.horror {
            if !definition.starting.iter().any(|e|e.kind==d.kind && e.duration>=d.remaining && matches!(e.kind,EffectKind::Horror(n)|EffectKind::ProtectedHorror(n)if n==d.amount)){return Err("checkpoint Horror disagrees with source effect".into());}
            if !self.units.get(*id).is_some_and(|u| {
                u.alive()
                    && self
                        .scenario
                        .rules
                        .flashback
                        .as_ref()
                        .unwrap()
                        .required
                        .contains(&u.blueprint.id)
            }) || d.remaining == 0
                || d.remaining > 10000
                || !matches!(d.kind,EffectKind::Horror(n)|EffectKind::ProtectedHorror(n)if n==d.amount)
                || state.ended.is_some()
            {
                return Err("invalid checkpoint protected Horror".into());
            }
        }
        if state.ended == Some(false) && self.outcome != Some(Outcome::FlashbackFailed) {
            return Err("invalid checkpoint flashback failure outcome".into());
        }
        if state.ended == Some(true) && self.outcome != Some(Outcome::HeroesWin) {
            return Err("invalid checkpoint flashback victory outcome".into());
        }
        let expected = definition
            .results
            .iter()
            .filter_map(|r| {
                if Some(r.success) == state.ended {
                    if let FlashOp::Reward { kind, id } = &r.operation {
                        Some((kind.clone(), id.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if expected != state.rewards {
            return Err("invalid checkpoint flashback rewards".into());
        }
        Ok(())
    }
}
