//! New rules are reachable only with the explicit task 023 content identity.
use super::*;
use dd_data::mechanisms023 as m;

impl Battle {
    pub(super) fn quirk_actout023(&mut self, actor: UnitId, events: &mut Vec<Event>) -> bool {
        if !self.units[actor].blueprint.hero || !m::enabled(&self.scenario.rules.content, m::FIXES)
        {
            return false;
        }
        for t in self.behavior_defs023(actor) {
            if self
                .trait_def(actor)
                .is_some_and(|resolve| resolve.id == t.id)
            {
                continue;
            }
            if self.run_turn_table023(actor, &t, events) {
                return true;
            }
        }
        false
    }
    pub(super) fn surprise_shuffle023(&mut self, events: &mut Vec<Event>) {
        // Hero units are one cell. Preserve immobilized anchors and sample
        // uniformly from the finite set of derangements in each mobile segment.
        let side = 0;
        self.order[side].retain(|id| self.units[*id].present());
        let before = self.order[side].clone();
        fn permutations(
            left: Vec<UnitId>,
            prefix: Vec<UnitId>,
            old: &[UnitId],
            out: &mut Vec<Vec<UnitId>>,
        ) {
            if left.is_empty() {
                out.push(prefix);
                return;
            }
            for (i, id) in left.iter().enumerate() {
                if *id == old[prefix.len()] {
                    continue;
                }
                let mut tail = left.clone();
                tail.remove(i);
                let mut next = prefix.clone();
                next.push(*id);
                permutations(tail, next, old, out);
            }
        }
        let mut start = 0;
        for end in 0..=before.len() {
            if end == before.len() || self.units[before[end]].immobilized.is_some() {
                if end - start > 1 {
                    let mut variants = vec![];
                    permutations(
                        before[start..end].to_vec(),
                        vec![],
                        &before[start..end],
                        &mut variants,
                    );
                    let chosen = self.rng.below(variants.len() as u64) as usize;
                    self.order[side][start..end].copy_from_slice(&variants[chosen]);
                }
                start = end + 1;
            }
        }
        self.refresh_context(true);
        for (old, unit) in before.iter().enumerate() {
            let rank = self.rank(*unit);
            if rank != old as u8 + 1 {
                events.push(Event::Move { actor: *unit, rank });
                for other in before.iter().filter(|other| *other != unit) {
                    self.queue_reaction(
                        *other,
                        dd_data::mechanics::ReactionKind::CommentMove,
                        *unit,
                    );
                }
            }
        }
    }
}

