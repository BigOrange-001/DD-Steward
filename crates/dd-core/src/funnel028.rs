//! Sidecar measurement over filtered views only; never called by a policy.
use super::Runtime027;
use crate::campaign018::runner::View;
use dd_data::combat::{EffectKind, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn effects(k: &EffectKind, flags: &mut [bool; 3]) {
    match k {
        EffectKind::Heal(_) | EffectKind::HealPercent(_) | EffectKind::Regen(_) => flags[0] = true,
        EffectKind::Stress(n) if *n < 0 => flags[1] = true,
        EffectKind::Stun
        | EffectKind::Pull(_)
        | EffectKind::Push(_)
        | EffectKind::Shuffle
        | EffectKind::ShuffleParty => flags[2] = true,
        EffectKind::Composite(es) => {
            for e in es {
                effects(&e.kind, flags)
            }
        }
        EffectKind::BattlePersistent(k) | EffectKind::InventoryEffect(k) => effects(k, flags),
        _ => {}
    }
}
pub fn snapshot(r: &Runtime027, view: &View, enrich: bool) -> Result<Value> {
    let (raid, town, week, returns) = match view {
        View::Opening(o) => (Some(o.as_ref()), None, 0, 0),
        View::Campaign(o) => (o.expedition.as_ref(), Some(&o.town), o.week, o.returns),
    };
    let mut capabilities = BTreeMap::new();
    if let Some(raid) = raid.filter(|_| enrich) {
        let game = &r.inner.inner.replay.inner.inner.inner.definition.town.game;
        for id in &raid.party {
            if let Some(h) = raid.heroes.iter().find(|h| h.guid == *id) {
                let training = town.and_then(|t| t.heroes.iter().find(|h| h.guid == *id));
                let ranks = h
                    .skills
                    .iter()
                    .map(|id| {
                        (
                            id.as_str(),
                            training
                                .and_then(|h| h.skills.get(id))
                                .copied()
                                .unwrap_or(0),
                        )
                    })
                    .collect::<Vec<_>>();
                let bp = game.hero_ranked(
                    &h.class,
                    training.map_or(0, |h| h.weapon),
                    training.map_or(0, |h| h.armour),
                    &ranks,
                )?;
                let variables = &r
                    .inner
                    .inner
                    .replay
                    .inner
                    .inner
                    .inner
                    .definition
                    .town
                    .data
                    .base
                    .roster_variables;
                let level = training.map(|h| h.level).unwrap_or(
                    variables["resolve_level_thresholds"]
                        .as_array()
                        .ok_or("028 missing public XP thresholds")?
                        .iter()
                        .filter(|n| n.as_u64().is_some_and(|n| n <= h.xp as u64))
                        .count()
                        .saturating_sub(1) as u32,
                );
                let mut flags = [false; 3];
                for sk in &bp.skills {
                    if sk.heal.is_some() {
                        flags[0] = true;
                    }
                    for e in &sk.effects {
                        effects(&e.kind, &mut flags);
                    }
                }
                capabilities.insert(*id,json!({"level":level,"weapon":training.map_or(0,|h|h.weapon),"armour":training.map_or(0,|h|h.armour),"skills":ranks,"heal":flags[0],"relief":flags[1],"control":flags[2]}));
            }
        }
    }
    Ok(
        json!({"week":week,"returns":returns,"actions":raid.map(|o|o.actions.iter().filter(|a|!matches!(a,crate::opening015::Action::Fight(_))).collect::<Vec<_>>()),"training":capabilities,"wallet":town.map(|t|&t.wallet).or_else(||raid.map(|o|&o.wallet)),"trinkets":town.map(|t|&t.trinkets)}),
    )
}
