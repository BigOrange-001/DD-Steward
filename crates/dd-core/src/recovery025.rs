//! W109: one explicit next-trip intent, funded after current training and supplies.
use super::*;
use crate::town016::Treatment;
pub type Ranges = BTreeMap<(u64, String), [i32; 2]>;
pub fn key_class(class: &str, ctx: &Context<'_>) -> bool {
    ctx.profile.parameters["town"]["key_classes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == class)
}
pub fn intent(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<()> {
    if m.state.get("next_trip_intent").is_some_and(|v| {
        v["returns"] == o.returns && v["current_party"] == json!(m.legacy.target_party)
    }) {
        return Ok(());
    }
    let mut future = o.clone();
    future
        .town
        .heroes
        .retain(|h| !m.legacy.target_party.contains(&h.guid));
    future.town.party.clear();
    let mut choices = BTreeMap::<u64, Vec<crate::town016::Choice>>::new();
    for c in &o.town.actions {
        let TownAction::Queue {
            hero,
            facility,
            treatment: Treatment::Stress,
        } = &c.action
        else {
            continue;
        };
        if m.legacy.target_party.contains(hero)
            || !m.ledger.permits(
                ledger::Purpose::Recovery,
                c.cost.get("gold:").copied().unwrap_or(0),
                true,
            )
        {
            continue;
        }
        let Some(h) = future.town.heroes.iter().find(|h| {
            h.guid == *hero
                && h.alive
                && !h.busy
                && !h.missing
                && key_class(&h.class, ctx)
                && h.stress
                    >= ctx.profile.parameters["town"]["recovery_stress"]
                        .as_i64()
                        .unwrap() as i32
        }) else {
            continue;
        };
        if ctx
            .sources
            .recovery_ranges
            .get(&(h.guid, facility.clone()))
            .is_some_and(|r| r[0] > 0)
        {
            choices.entry(*hero).or_default().push(c.clone());
        }
    }
    for h in &mut future.town.heroes {
        if let Some(cs) = choices.get(&h.guid) {
            let minimum = cs
                .iter()
                .filter_map(|c| {
                    let TownAction::Queue { facility, .. } = &c.action else {
                        return None;
                    };
                    ctx.sources
                        .recovery_ranges
                        .get(&(h.guid, facility.clone()))
                        .map(|r| r[0])
                })
                .min()
                .unwrap();
            h.stress = h.stress.saturating_sub(minimum).max(0);
        }
    }
    // Independent temporary planning has no spending, policy action or future board.
    let mut trial = Memory::default();
    let selected = super::town::select(&future, &mut trial, ctx);
    let mut treatments = vec![];
    let mut feasible = false;
    if let Some((_, party)) = &selected {
        let mut need = party
            .iter()
            .filter(|id| choices.contains_key(id))
            .copied()
            .collect::<Vec<_>>();
        need.sort_by_key(|id| (choices[id].len(), *id));
        fn assign(
            ids: &[u64],
            choices: &BTreeMap<u64, Vec<crate::town016::Choice>>,
            used: &mut BTreeSet<String>,
            cost: u32,
            budget: u32,
            out: &mut Vec<Value>,
        ) -> bool {
            let Some((id, rest)) = ids.split_first() else {
                return true;
            };
            let mut cs = choices[id].iter().collect::<Vec<_>>();
            cs.sort_by_key(|c| c.cost.get("gold:").copied().unwrap_or(0));
            for c in cs {
                let TownAction::Queue { facility, .. } = &c.action else {
                    continue;
                };
                let Some(total) = cost.checked_add(c.cost.get("gold:").copied().unwrap_or(0))
                else {
                    continue;
                };
                // A currently legal facility has at least one free slot. Use only
                // one per facility rather than assuming unexposed spare capacity.
                if total > budget || !used.insert(facility.clone()) {
                    continue;
                }
                out.push(json!(c));
                if assign(rest, choices, used, total, budget, out) {
                    return true;
                }
                out.pop();
                used.remove(facility);
            }
            false
        }
        feasible = assign(
            &need,
            &choices,
            &mut BTreeSet::new(),
            0,
            m.ledger
                .gold
                .saturating_sub(m.ledger.reserve.saturating_add(m.ledger.supplies)),
            &mut treatments,
        );
    }
    let (quest, party) = selected.map_or((None, vec![]), |(q, p)| (Some(q), p));
    m.state.insert("next_trip_intent".into(),json!({"returns":o.returns,"current_party":m.legacy.target_party,"quest":quest,"party":if feasible{party}else{vec![]},"treatments":if feasible{treatments}else{vec![]},"source":"current visible board and off-party roster; source minimum treatment; no future income; reconsider actual eligibility next return","feasible":feasible,"funded":false,"funded_heroes":[]}));
    Ok(())
}
pub fn permits(m: &Memory, action: &TownAction) -> bool {
    let Some(v) = m.state.get("next_trip_intent") else {
        return false;
    };
    v["treatments"].as_array().is_some_and(|cs| {
        cs.iter()
            .any(|c| serde_json::to_value(action).is_ok_and(|a| a == c["action"]))
    })
}
pub fn committed(m: &mut Memory, log: &Value) {
    let Some(v) = m.state.get_mut("next_trip_intent") else {
        return;
    };
    let action = &log["action"]["Town"];
    if v["treatments"]
        .as_array()
        .is_some_and(|cs| cs.iter().any(|c| c["action"] == *action))
    {
        v["funded"] = json!(true);
        if let Some(id) = action["Queue"]["hero"].as_u64() {
            let heroes = v["funded_heroes"].as_array_mut().unwrap();
            if !heroes.contains(&json!(id)) {
                heroes.push(json!(id));
            }
        }
    }
}
