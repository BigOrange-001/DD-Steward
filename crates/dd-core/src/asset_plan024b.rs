//! Visible next-party assignment used only for economic retention.
use super::{Context, assets::equip};
use std::collections::BTreeSet;
/// The same positive-score assignment on a visible copy, for H when E is off.
/// No host action, RNG, asset creation or source definition is involved.
pub fn planned(ctx: &Context, t: &crate::town016::Observation) -> BTreeSet<u64> {
    use crate::town016::Action;
    let mut t = t.clone();
    let mut keep = BTreeSet::new();
    while let Some(Action::Equip {
        hero,
        instance,
        slot,
    }) = equip(ctx, &t)
    {
        let Some((_, id, owner)) = t.trinkets.iter_mut().find(|(i, _, _)| *i == instance) else {
            break;
        };
        let id = id.clone();
        *owner = Some((hero, slot));
        keep.insert(instance);
        if let Some(h) = t.heroes.iter_mut().find(|h| h.guid == hero) {
            h.equipped.push((slot, instance, id.clone()));
        }
        t.actions.retain(|c|!matches!(c.action,Action::Equip{hero:h,instance:i,slot:s} if i==instance || (h==hero && (s==slot || t.trinkets.iter().any(|(j,n,_)|*j==i && n==&id)))));
    }
    keep
}
