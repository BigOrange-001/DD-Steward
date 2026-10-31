use super::*;
use dd_data::build010::Disease;
impl Battle {
    pub(super) fn remove_disease(&mut self, id: UnitId, source: &str) {
        self.units[id]
            .buffs
            .retain(|b| b.source_disease.as_deref() != Some(source));
        self.units[id].diseases.retain(|d| d != source);
        self.units[id].disease_tags.remove(source);
        self.units[id].disease_turns.remove(source);
    }
    pub(super) fn give_disease(&mut self, id: UnitId, d: &Disease, events: &mut Vec<Event>) {
        self.units[id].diseases.push(d.id.clone());
        self.units[id]
            .disease_tags
            .insert(d.id.clone(), d.tags.clone());
        if let Some(e) = &d.evolution {
            let turns = (self.rng.range([e.min as i32, e.max as i32])
                + self.units[id]
                    .modifier("vampire_evolution_duration", "")
                    .round() as i32)
                .clamp(1, 10000) as u32;
            self.units[id].disease_turns.insert(d.id.clone(), turns);
        }
        for b in &d.buffs {
            self.add_buff(id, b.clone(), BuffScope::Disease, events);
            if let Some(b) = self.units[id].buffs.last_mut() {
                b.source_disease = Some(d.id.clone());
            }
        }
        self.units[id].hp = self.units[id].hp.min(self.units[id].stats().hp);
        self.refresh_context(false);
    }
    pub(super) fn contract_disease(
        &mut self,
        id: UnitId,
        d: &Disease,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(ctx) = self.scenario.rules.context.clone() else {
            return false;
        };
        let mut slots = self.units[id]
            .diseases
            .iter()
            .filter_map(|id| ctx.diseases.get(id))
            .map(|d| d.slots)
            .sum::<u32>()
            + d.slots;
        let mut possible = self.units[id]
            .diseases
            .iter()
            .filter_map(|id| ctx.diseases.get(id))
            .filter(|d| d.replaceable)
            .cloned()
            .collect::<Vec<_>>();
        let mut removed = vec![];
        while slots > ctx.disease_max && !possible.is_empty() {
            let index = self.rng.below(possible.len() as u64) as usize;
            let old = possible.remove(index);
            slots -= old.slots;
            removed.push(old.id);
        }
        if slots > ctx.disease_max {
            return false;
        }
        for source in removed {
            self.remove_disease(id, &source);
        }
        self.give_disease(id, d, events);
        true
    }
    pub(super) fn tick_diseases(&mut self, id: UnitId, events: &mut Vec<Event>) {
        if !self.units[id].alive() {
            return;
        }
        let mut expired = vec![];
        for (source, n) in &mut self.units[id].disease_turns {
            *n = n.saturating_sub(1);
            if *n == 0 {
                expired.push(source.clone());
            }
        }
        for source in expired {
            let ctx = self.scenario.rules.context.clone();
            let Some(ctx) = ctx else {
                self.lifecycle.failure = Some("missing disease evolution context".into());
                return;
            };
            let Some(e) = ctx.diseases.get(&source).and_then(|d| d.evolution.clone()) else {
                self.lifecycle.failure = Some("unknown disease evolution source".into());
                return;
            };
            if e.death {
                if self.units[id].modifier("quirk_evolution_death_immune", "") > 0. {
                    self.units[id].disease_turns.insert(source, 1);
                    continue;
                }
                self.units[id].disease_turns.remove(&source);
                self.retire(id, events);
                self.death_stress(id, events);
                self.hero_death(id, events);
                return;
            }
            let Some(next) = e.next.and_then(|id| ctx.diseases.get(&id).cloned()) else {
                self.lifecycle.failure = Some("missing evolved disease".into());
                return;
            };
            self.remove_disease(id, &source);
            self.give_disease(id, &next, events);
        }
    }
}
