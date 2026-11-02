//! Skill choices and physical trinket assignment share one set of public cards.
use super::*;
use crate::town016::VisibleHero;
pub fn desired(h: &VisibleHero, ctx: &Context<'_>) -> Vec<String> {
    let mut skills = super::boss::desired(h, ctx).unwrap_or_default();
    for id in ctx.profile.parameters["loadouts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["class_id"] == h.class)
        .and_then(|l| l["skills"].as_array())
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !skills.iter().any(|s| s == id) {
            skills.push(id.into())
        }
    }
    skills.truncate(4);
    skills
}

pub fn skills(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("skills") {
        return Ok(None);
    }
    for h in o
        .town
        .heroes
        .iter()
        .filter(|h| m.legacy.target_party.contains(&h.guid))
    {
        let s = super::town::selected(h, ctx);
        if s.iter().collect::<BTreeSet<_>>() != h.selected.iter().collect::<BTreeSet<_>>() {
            return proposal(
                o,
                Action::Town(TownAction::SelectSkills {
                    hero: h.guid,
                    skills: s,
                }),
                "S025-05",
                "skills",
                ctx,
            )
            .map(Some);
        }
        if !ctx.on("growth") {
            for id in desired(h, ctx)
                .iter()
                .filter(|id| !h.skills.contains_key(*id))
            {
                if let Some(c)=o.town.actions.iter().find(|c|matches!(&c.action,TownAction::Upgrade{hero:Some(g),tree,..}if *g==h.guid&&*tree==format!("{}.{}",h.class,id))&&m.ledger.permits(ledger::Purpose::Training,c.cost.get("gold:").copied().unwrap_or(0),true)){return proposal(o,Action::Town(c.action.clone()),"S025-05","learn_and_use",ctx).map(Some)}
            }
        }
    }
    Ok(None)
}
pub fn score(h: &VisibleHero, id: &str, ctx: &Context<'_>) -> Result<f64> {
    let c = ctx
        .sources
        .cards
        .assets
        .get(id)
        .ok_or_else(|| format!("025 public trinket card absent {id}"))?;
    if !c.classes.is_empty() && !c.classes.contains(&h.class) {
        return Ok(f64::NEG_INFINITY);
    }
    let cfg = &ctx.profile.parameters["assets"];
    let w = |k: &str| cfg["weights"][k].as_f64().unwrap_or(0.);
    let mut healing = 0f64;
    let mut relief = 0f64;
    let mut control = 0f64;
    let mut damage = 0f64;
    for id in super::town::selected(h, ctx) {
        if let Some(a) = ctx
            .sources
            .cards
            .skills
            .get(&h.class)
            .and_then(|s| s.get(&id))
            .filter(|a| a.available_at_start)
        {
            healing = healing.max(a.healing + a.healing_percent * f64::from(h.stats.hp));
            relief = relief.max(a.stress_relief);
            control = control.max(a.control);
            damage = damage.max(a.damage)
        }
    }
    let mean = f64::from(h.stats.damage[0] + h.stats.damage[1]) / 2.;
    let crit = (f64::from(h.stats.damage[1]) * ctx.sources.cards.crit_multiplier - mean).max(0.);
    let light = if ctx.sources.mode.ends_with(":dark") {
        0.
    } else {
        cfg["planning_light"].as_f64().unwrap_or(100.)
    };
    let mut value = 0.;
    for b in &c.buffs {
        let rule = b["rule_type"].as_str().unwrap_or("always");
        let threshold = b["rule_data"]["float"].as_f64().unwrap_or(0.);
        if !(rule == "always"
            || rule == "lightabove" && light > threshold
            || rule == "lightbelow" && light < threshold)
        {
            continue;
        }
        let stat = b["stat_type"].as_str().unwrap_or("");
        let sub = b["stat_sub_type"].as_str().unwrap_or("");
        let role = match stat {
            "hp_heal_percent" => f64::from(healing > 0.),
            "stress_heal_percent" => f64::from(relief > 0.),
            "stun_chance" => f64::from(control > 0.),
            _ => 1.,
        };
        value += b["amount"].as_f64().unwrap_or(0.)
            * role
            * match (stat, sub) {
                ("combat_stat_add", "crit_chance") => {
                    if damage > 0. {
                        crit * w("crit_expected_damage")
                    } else {
                        0.
                    }
                }
                ("combat_stat_add", "attack_rating") => w("accuracy"),
                ("combat_stat_add", "defense_rating") => w("dodge"),
                ("combat_stat_add", "speed_rating") => w("speed"),
                ("combat_stat_multiply", "damage_low" | "damage_high") => mean * w("damage") / 2.,
                ("combat_stat_multiply", "max_hp") => f64::from(h.stats.hp) * w("hp"),
                ("resistance", "bleed" | "poison" | "disease") => w("resistance"),
                _ => w(stat),
            };
    }
    value += cfg["named_bias"][id][&h.class]
        .as_f64()
        .or_else(|| cfg["named_bias"][id]["*"].as_f64())
        .unwrap_or(0.);
    Ok(value)
}
pub fn trinkets(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("trinkets") {
        return Ok(None);
    }
    let mut best = None;
    for c in &o.town.actions {
        let TownAction::Equip {
            hero,
            instance,
            slot,
        } = c.action
        else {
            continue;
        };
        let Some(h) = o.town.heroes.iter().find(|h| {
            h.guid == hero
                && m.legacy.target_party.contains(&hero)
                && !h.equipped.iter().any(|(s, _, _)| *s == slot)
        }) else {
            continue;
        };
        let Some((_, id, None)) = o.town.trinkets.iter().find(|(i, _, _)| *i == instance) else {
            continue;
        };
        if id == "dd_trinket" {
            continue;
        }
        let v = score(h, id, ctx)?;
        if v > 0. && best.as_ref().is_none_or(|(value, _, _)| v > *value) {
            best = Some((v, (hero, instance, slot), c.action.clone()))
        }
    }
    match best {
        Some((_, _, a)) => proposal(o, Action::Town(a), "S025-06", "trinkets", ctx).map(Some),
        None => Ok(None),
    }
}
pub fn keep(o: &Observation, m: &Memory, ctx: &Context<'_>) -> Result<BTreeSet<u64>> {
    let mut keep = BTreeSet::new();
    let ids = m
        .legacy
        .target_party
        .iter()
        .copied()
        .chain(m.future_plan.iter().flat_map(|p| {
            p["party"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
        }))
        .collect::<BTreeSet<_>>();
    for h in o.town.heroes.iter().filter(|h| ids.contains(&h.guid)) {
        let mut candidates = o
            .town
            .trinkets
            .iter()
            .map(|(i, id, _)| Ok((score(h, id, ctx)?, *i)))
            .collect::<Result<Vec<_>>>()?;
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        keep.extend(
            candidates
                .into_iter()
                .filter(|(s, _)| *s > 0.)
                .take(2)
                .map(|(_, i)| i),
        );
    }
    Ok(keep)
}
pub fn sell(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("poverty") || m.ledger.gold >= m.ledger.supplies {
        return Ok(None);
    }
    let keep = keep(o, m, ctx)?;
    let mut candidates = vec![];
    for (i, id, owner) in &o.town.trinkets {
        if owner.is_some()
            || keep.contains(i)
            || ctx.profile.parameters["protected_trinkets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == id)
        {
            continue;
        }
        let a = TownAction::Sell { instance: *i };
        if !o.town.actions.iter().any(|c| c.action == a) {
            continue;
        }
        let value = o
            .town
            .heroes
            .iter()
            .filter(|h| h.alive)
            .map(|h| score(h, id, ctx))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .fold(0f64, f64::max);
        candidates.push((value, *i, a))
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    if let Some((v, i, a)) = candidates.first() {
        m.state
            .insert("sale".into(), json!({"instance":i,"score":v,"week":o.week}));
        return proposal(o, Action::Town(a.clone()), "S025-07", "sell_idle", ctx).map(Some);
    }
    Ok(None)
}
