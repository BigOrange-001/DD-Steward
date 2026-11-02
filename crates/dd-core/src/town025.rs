//! Independent town planning; all inputs are public and all purchases use one ledger.
use super::*;
use crate::{
    campaign017::QuestView,
    town016::{Treatment, VisibleHero},
};
fn n(ctx: &Context<'_>, key: &str) -> f64 {
    ctx.profile.parameters["town"][key].as_f64().unwrap_or(0.)
}
fn available(h: &VisibleHero) -> bool {
    h.alive && !h.busy && !h.missing && !h.quirks.contains_key("shard_hungry")
}
pub fn selected(h: &VisibleHero, ctx: &Context<'_>) -> Vec<String> {
    if !ctx.on("skills") || ctx.sources.cards.fixed_skills.contains(&h.class) {
        return h.selected.clone();
    }
    let mut s = super::loadout::desired(h, ctx)
        .into_iter()
        .filter(|id| h.skills.contains_key(id))
        .take(4)
        .collect::<Vec<_>>();
    for id in h.selected.iter().chain(h.skills.keys()) {
        if s.len() < 4 && !s.contains(id) {
            s.push(id.clone())
        }
    }
    s
}
pub(super) fn roles(h: &VisibleHero, rank: u8, ctx: &Context<'_>) -> [f64; 5] {
    let mut r = [0.; 5];
    for s in selected(h, ctx) {
        if let Some(a) = ctx
            .sources
            .cards
            .skills
            .get(&h.class)
            .and_then(|m| m.get(&s))
            .filter(|a| a.available_at_start && a.launch.contains(&rank))
        {
            r[0] += a.damage;
            r[1] = r[1].max(if a.conditional_healing {
                0.
            } else {
                a.healing + a.healing_percent * f64::from(h.stats.hp)
            });
            r[2] = r[2].max(a.stress_relief);
            r[3] += a.control;
            r[4] += 1.;
        }
    }
    r
}
fn power(h: &VisibleHero) -> f64 {
    crate::preparation024::strength(h).iter().sum::<f64>() / 4.
}
pub(super) fn rank_score(h: &VisibleHero, rank: u8, ctx: &Context<'_>) -> f64 {
    let r = roles(h, rank, ctx);
    let w = &ctx.profile.parameters["party_weights"];
    r.iter()
        .zip(["damage", "healing", "relief", "control", "coverage"])
        .map(|(v, k)| v * w[k].as_f64().unwrap())
        .sum::<f64>()
        + if ctx.on("stress") {
            (1..=4)
                .map(|rank| roles(h, rank, ctx)[4])
                .fold(f64::INFINITY, f64::min)
                * ctx.profile.parameters["composition"]["weights"]["shuffle"]
                    .as_f64()
                    .unwrap()
        } else {
            0.
        }
        + power(h) * n(ctx, "power_weight")
        - f64::from(h.stress) * n(ctx, "stress_weight")
}
pub fn arrange(t: &crate::town016::Observation, ids: Vec<u64>, ctx: &Context<'_>) -> Vec<u64> {
    fn recurse(
        t: &crate::town016::Observation,
        left: &[u64],
        p: &mut Vec<u64>,
        best: &mut (f64, Vec<u64>),
        ctx: &Context<'_>,
    ) {
        if left.is_empty() {
            let s = p
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    t.heroes
                        .iter()
                        .find(|h| h.guid == *id)
                        .map(|h| rank_score(h, i as u8 + 1, ctx))
                })
                .sum();
            if s > best.0 {
                *best = (s, p.clone())
            }
            return;
        }
        for (i, id) in left.iter().enumerate() {
            let mut next = left.to_vec();
            next.remove(i);
            p.push(*id);
            recurse(t, &next, p, best, ctx);
            p.pop();
        }
    }
    if !ctx.on("skills") {
        return ids;
    }
    let mut best = (f64::NEG_INFINITY, ids.clone());
    recurse(t, &ids, &mut vec![], &mut best, ctx);
    best.1
}
fn compatible(t: &crate::town016::Observation, ids: &[u64], h: &VisibleHero) -> bool {
    crate::preparation024::composition::compatible(t, ids, h)
}
pub(super) fn gear_target(q: &QuestView, ctx: &Context<'_>) -> usize {
    if q.dungeon == "farm" && ctx.on("cohorts") || q.difficulty >= 6 {
        n(ctx, "farm_gear") as usize
    } else if q.difficulty >= 5 {
        ctx.profile.modes[&ctx.sources.mode]["champion_gear"]
            .as_u64()
            .unwrap() as usize
    } else if q.difficulty >= 3 {
        n(ctx, "veteran_gear") as usize
    } else {
        n(ctx, "apprentice_gear") as usize
    }
}
pub fn training_cost(
    t: &crate::town016::Observation,
    h: &VisibleHero,
    target: usize,
    ctx: &Context<'_>,
) -> u32 {
    if ctx.on("income")
        && target == n(ctx, "veteran_gear") as usize
        && !ctx.sources.training.nodes.is_empty()
    {
        let goals = super::training::goals(t, &[h.guid], target, ctx);
        return ctx
            .sources
            .training
            .plan(t, &goals)
            .map(|p| p.cost.get("gold:").copied().unwrap_or(0))
            .unwrap_or(u32::MAX);
    }
    t.upgrade_quotes
        .iter()
        .filter(|c| {
            let TownAction::Upgrade {
                hero: Some(id),
                tree,
                code,
            } = &c.action
            else {
                return false;
            };
            let _ = code;
            if *id != h.guid {
                return false;
            }
            let s = tree.strip_prefix(&format!("{}.", h.class)).unwrap_or("");
            let core = ctx.profile.parameters["skill_core"][&h.class]
                .as_array()
                .is_some_and(|v| v.iter().any(|v| v == s));
            (s == "weapon" && h.weapon < target)
                || (s == "armour" && h.armour < target)
                || (core && h.skills.get(s).copied().unwrap_or(-1) < target as i32)
        })
        .map(|c| c.cost.get("gold:").copied().unwrap_or(0))
        .fold(0, u32::saturating_add)
}
fn selection_party(
    o: &Observation,
    q: &QuestView,
    m: &Memory,
    ctx: &Context<'_>,
) -> Result<Vec<u64>> {
    if ctx.on("growth")
        && let Some(intent) = m.state.get("next_trip_intent")
        && intent["funded"] == true
        && intent["returns"]
            .as_u64()
            .is_some_and(|r| r + 1 == u64::from(o.returns))
        && let Some(ids) = intent["party"]
            .as_array()
            .map(|p| p.iter().filter_map(Value::as_u64).collect::<Vec<_>>())
        && ids.len() == 4
    {
        let mut planned = o.clone();
        planned.town.heroes.retain(|h| ids.contains(&h.guid));
        if let Ok(p) = party_reason(&planned, q, ctx, false)
            && p.len() == 4
            && court_ready(o, q, &p, ctx)
            && regional_ready(o, q, &p, ctx)
        {
            return Ok(p);
        }
    }
    party_reason(o, q, ctx, false)
}
fn party_reason(
    o: &Observation,
    q: &QuestView,
    ctx: &Context<'_>,
    relax: bool,
) -> Result<Vec<u64>> {
    let t = eligible_town(o, q, ctx, relax);
    let poverty = ctx.on("poverty")
        && o.town.wallet.get("gold:").copied().unwrap_or(0) < ctx.profile.ledger.poverty_gold
        && q.difficulty == 1;
    let mut ids = vec![];
    if let Some((_, p)) = super::boss::party(&t, q, ctx) {
        ids = p;
    } else if let Some(p) = merchant(&t, q, ctx) {
        ids = p;
    } else if ctx.on("party") || ctx.on("stress") {
        for slot in [4, 1, 2, 3] {
            let candidate = t
                .heroes
                .iter()
                .filter(|h| !ids.contains(&h.guid) && compatible(&t, &ids, h))
                .max_by(|a, b| {
                    let score = |h: &VisibleHero| {
                        let r = roles(h, slot, ctx);
                        rank_score(h, slot, ctx)
                            + if slot == 4 {
                                r[1] * n(ctx, "healer_slot_bonus")
                            } else {
                                0.
                            }
                            - f64::from(training_cost(&t, h, gear_target(q, ctx), ctx))
                                * n(ctx, "training_cost_weight")
                            + if poverty {
                                -power(h) * n(ctx, "cheap_power_penalty")
                            } else {
                                0.
                            }
                    };
                    score(a)
                        .total_cmp(&score(b))
                        .then_with(|| b.guid.cmp(&a.guid))
                });
            if let Some(h) = candidate {
                ids.push(h.guid)
            }
        }
        ids = arrange(&t, ids, ctx);
    } else {
        ids = crate::campaign_policy017::party(&t)
    }
    if ids.len() < 4 && !relax {
        return Err("roster_level_or_compatibility: fewer than four eligible heroes".into());
    }
    if ctx.on("party")
        && !poverty
        && !ids.iter().any(|id| {
            t.heroes
                .iter()
                .any(|h| h.guid == *id && h.class == "antiquarian")
        })
        && super::boss::party(&t, q, ctx).is_none()
    {
        ids = templates(&t, q, ids, ctx);
    }
    let hs = t
        .heroes
        .iter()
        .filter(|h| ids.contains(&h.guid))
        .collect::<Vec<_>>();
    if poverty
        && !hs.iter().any(|h| h.level >= 1)
        && let Some(h) = t
            .heroes
            .iter()
            .filter(|h| h.level >= 1 && !ids.contains(&h.guid))
            .min_by_key(|h| (h.level, h.stress, h.guid))
    {
        for i in (0..ids.len()).rev() {
            let mut p = ids.clone();
            p.remove(i);
            if compatible(&t, &p, h) {
                ids[i] = h.guid;
                break;
            }
        }
    }
    let hs = t
        .heroes
        .iter()
        .filter(|h| ids.contains(&h.guid))
        .collect::<Vec<_>>();
    if !relax && ctx.on("party") && q.length > 1 {
        let rs = ids
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                t.heroes
                    .iter()
                    .find(|h| h.guid == *id)
                    .map(|h| roles(h, i as u8 + 1, ctx))
            })
            .collect::<Vec<_>>();
        let heal = rs.iter().map(|r| r[1]).fold(0f64, f64::max);
        let relief = rs.iter().map(|r| r[2]).fold(0f64, f64::max);
        let controls = rs.iter().filter(|r| r[3] > 0.).count();
        if heal < n(ctx, "medium_min_heal")
            || relief < n(ctx, "medium_min_relief")
                && controls < n(ctx, "medium_min_controls_without_relief") as usize
        {
            return Err("party_functions: healing, relief or control below existing gate".into());
        }
    }
    if !relax
        && ctx.on("income")
        && q.difficulty == 3
        && hs.iter().filter(|h| h.level >= 3).count() < n(ctx, "veteran_required_level3") as usize
    {
        return Err("resolve: too few level3 heroes for existing veteran gate".into());
    }
    if !relax && ctx.on("income") && q.difficulty >= 3 {
        let target = gear_target(q, ctx);
        if q.difficulty == 3 {
            veteran_plan(o, q, &ids, ctx)?;
        } else {
            let cost = hs
                .iter()
                .map(|h| training_cost(&t, h, target, ctx))
                .fold(0, u32::saturating_add);
            let budget = o
                .town
                .wallet
                .get("gold:")
                .copied()
                .unwrap_or(0)
                .saturating_sub(
                    ctx.profile.ledger.later_reserve + ctx.profile.ledger.pending_supply_reserve,
                );
            if hs.iter().any(|h| {
                (h.weapon < target || h.armour < target) && training_cost(&t, h, target, ctx) == 0
            }) || cost > budget
            {
                return Err("champion_training: existing gate".into());
            }
        }
        if q.difficulty >= 5 && hs.iter().any(|h| h.weapon < target || h.armour < target) {
            return Err("champion_equipment: existing actual gear gate".into());
        }
    }
    if !relax && ctx.on("stress") && !hs.is_empty() {
        let avg = hs.iter().map(|h| h.stress).sum::<i32>() as f64 / hs.len() as f64;
        let mode = &ctx.profile.modes[&ctx.sources.mode];
        let limit = mode[if q.length > 1 {
            "average_stress_camp"
        } else {
            "average_stress_short"
        }]
        .as_f64()
        .unwrap();
        let control = ids
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                t.heroes
                    .iter()
                    .find(|h| h.guid == *id)
                    .map(|h| roles(h, i as u8 + 1, ctx)[3] > 0.)
            })
            .filter(|v| *v)
            .count();
        if avg > limit
            || hs.iter().any(|h| h.stress >= 70)
                && control < mode["minimum_controls_high_stress"].as_u64().unwrap() as usize
        {
            return Err("stress: existing average or high stress control gate".into());
        }
    }
    Ok(ids)
}
pub(super) fn veteran_plan(
    o: &Observation,
    q: &QuestView,
    ids: &[u64],
    ctx: &Context<'_>,
) -> Result<super::training::Plan> {
    let heroes = ids
        .iter()
        .filter_map(|id| o.town.heroes.iter().find(|h| h.guid == *id && available(h)))
        .collect::<Vec<_>>();
    if heroes.len() != 4
        || heroes
            .iter()
            .any(|h| h.level < n(ctx, "veteran_min_level") as u32)
        || heroes.iter().filter(|h| h.level >= 3).count()
            < n(ctx, "veteran_required_level3") as usize
    {
        return Err("resolve: existing four-person veteran level gate".into());
    }
    let goals = super::training::goals(&o.town, ids, gear_target(q, ctx), ctx);
    let plan = ctx.sources.training.plan(&o.town, &goals)?;
    let budget = o
        .town
        .wallet
        .get("gold:")
        .copied()
        .unwrap_or(0)
        .saturating_sub(
            ctx.profile.ledger.later_reserve + ctx.profile.ledger.pending_supply_reserve,
        );
    if plan.cost.get("gold:").copied().unwrap_or(0) > budget {
        return Err(format!(
            "training_gold: full quote {} exceeds budget {budget}",
            plan.cost.get("gold:").copied().unwrap_or(0)
        ));
    }
    if !ctx.on("town")
        && plan
            .actions
            .iter()
            .any(|c| matches!(c.action, TownAction::Upgrade { hero: None, .. }))
    {
        return Err("facility_group_off: missing building dependencies".into());
    }
    let materials: BTreeMap<String, u32> = plan
        .cost
        .iter()
        .filter(|(k, _)| k.as_str() != "gold:")
        .map(|(k, n)| (k.clone(), *n))
        .collect();
    //026 never promises a multi-building training bundle on unperformed trades.
    if ctx.profile.parameters["sync026"]["heirlooms"]["enabled"] == true
        && materials
            .iter()
            .any(|(k, n)| o.town.wallet.get(k).copied().unwrap_or(0) < *n)
    {
        return Err("facility_heirlooms:026 waits for actual material funding before promising full training".into());
    }
    // All shared facility costs appear once in the complete dependency closure.
    if ctx
        .sources
        .training_heirlooms
        .funding_plan(&o.town, &materials)
        .is_none()
    {
        return Err("facility_heirlooms: complete shared prerequisite cost unavailable".into());
    }
    Ok(plan)
}
pub fn select(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Option<(QuestView, Vec<u64>)> {
    if !["party", "income", "poverty", "cohorts", "boss", "stress"]
        .iter()
        .any(|g| ctx.on(g))
    {
        if ctx.on("growth")
            && let Some(intent) = m.state.get("next_trip_intent")
            && intent["funded"] == true
            && intent["returns"]
                .as_u64()
                .is_some_and(|r| r + 1 == u64::from(o.returns))
            && let Some(ids) = intent["party"]
                .as_array()
                .map(|p| p.iter().filter_map(Value::as_u64).collect::<Vec<_>>())
            && ids.len() == 4
        {
            let mut planned = o.clone();
            planned.town.heroes.retain(|h| ids.contains(&h.guid));
            if let Some(selected) = fallback(&planned, m, ctx) {
                return Some(selected);
            }
        }
        return fallback(o, m, ctx);
    }
    let board = o.board.as_ref()?;
    if ctx.on("boss")
        && m.state.get("boss_audited_return").and_then(Value::as_u64) != Some(u64::from(o.returns))
    {
        for q in &board.quests {
            let t = eligible_town(o, q, ctx, false);
            for row in super::boss::diagnose(&t, q, ctx) {
                let id = row["id"].as_str().unwrap().to_string();
                let reason = row["reason"].as_str().unwrap().to_string();
                *m.boss_attempts
                    .entry(id.clone())
                    .or_default()
                    .entry(reason)
                    .or_default() += 1;
                m.boss_last.insert(id, row);
            }
        }
        m.state
            .insert("boss_audited_return".into(), json!(o.returns));
    }

    if ctx.on("poverty") {
        let checks=board.quests.iter().filter(|q|q.plot.is_none()&&[1,3].contains(&q.difficulty)&&q.length<=2).map(|q|json!({"quest":q.id,"difficulty":q.difficulty,"qualified_escorts":escorts(&o.town,q,ctx).len(),"merchants":o.town.heroes.iter().filter(|h|available(h)&&h.class=="antiquarian"&&q.maximum_resolve_level.is_none_or(|max|h.level<=max)).count(),"gold":o.town.wallet.get("gold:").copied().unwrap_or(0)})).collect::<Vec<_>>();
        m.state
            .entry("merchant_funnel".into())
            .or_insert_with(|| json!({}))[o.returns.to_string()] = json!(checks);
    }

    let poor = ctx.on("poverty")
        && o.town.wallet.get("gold:").copied().unwrap_or(0) < ctx.profile.ledger.poverty_gold;
    let mut veterans = BTreeMap::new();
    if ctx.on("income") {
        let mut rows = vec![];
        for q in board.quests.iter().filter(|q| q.difficulty == 3) {
            let result = if q.execution == "expedition integration pending" {
                Err("execution: integration pending".into())
            } else if m.legacy.blocked.contains(q.plot.as_ref().unwrap_or(&q.id)) {
                Err("blocked: previous strategy route failure".into())
            } else if !q
                .estate_requirements
                .iter()
                .all(|(k, n)| o.town.wallet.get(k).copied().unwrap_or(0) >= *n)
            {
                Err("estate_requirements: public wallet insufficient".into())
            } else {
                selection_party(o, q, m, ctx).and_then(|p| {
                    if !court_ready(o, q, &p, ctx) {
                        Err("court_gate: existing gate".into())
                    } else if !regional_ready(o, q, &p, ctx) {
                        Err("regional_boss_gate: existing gate".into())
                    } else {
                        Ok(p)
                    }
                })
            };
            let reason = result.as_ref().err().cloned();
            if let Some(reason) = &reason {
                let key = reason.split(':').next().unwrap_or(reason);
                let counts = m
                    .state
                    .entry("veteran_rejections".into())
                    .or_insert_with(|| json!({}));
                counts[key] = json!(counts[key].as_u64().unwrap_or(0) + 1);
            }
            let plan = result
                .as_ref()
                .ok()
                .and_then(|ids| veteran_plan(o, q, ids, ctx).ok());
            rows.push(json!({"quest":q.id,"plot":q.plot,"reason":reason,"party":result.as_ref().ok(),"plan":plan}));
            veterans.insert(q.id.clone(), result);
        }
        let event = json!({"week":o.week,"returns":o.returns,"candidates":rows});
        m.state
            .insert("veteran_last_selection".into(), event.clone());
        m.state
            .entry("veteran_selections".into())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(event);
    }
    let mut options = board
        .quests
        .iter()
        .filter(|q| {
            q.execution != "expedition integration pending"
                && !m.legacy.blocked.contains(q.plot.as_ref().unwrap_or(&q.id))
                && q.estate_requirements
                    .iter()
                    .all(|(c, n)| o.town.wallet.get(c).copied().unwrap_or(0) >= *n)
                && !(ctx.on("income")
                    && q.difficulty == 5
                    && o.week
                        < ctx.profile.modes[&ctx.sources.mode]["champion_after_week"]
                            .as_u64()
                            .unwrap() as u32)
        })
        .filter_map(|q| {
            let p = veterans.get(&q.id).map_or_else(
                || selection_party(o, q, m, ctx).unwrap_or_default(),
                |p| p.clone().unwrap_or_default(),
            );
            if p.len() < 4 || !court_ready(o, q, &p, ctx) || !regional_ready(o, q, &p, ctx) {
                return None;
            }
            let hs = o
                .town
                .heroes
                .iter()
                .filter(|h| p.contains(&h.guid))
                .collect::<Vec<_>>();
            let min = hs.iter().map(|h| h.level).min().unwrap_or(0);
            let native = q
                .plot
                .as_deref()
                .is_some_and(super::caretaker::is_goal_plot)
                && !q.plot.as_ref().is_some_and(|p| {
                    o.automation
                        .as_ref()
                        .is_some_and(|a| a.completed.contains(p))
                });
            let mut score = 0.;
            if native {
                score += n(ctx, "native_goal_bonus")
            }
            if q.dungeon == "darkestdungeon" {
                score += n(ctx, "main_goal_bonus")
            }
            if ctx.on("income") {
                score += if q.difficulty == 3 && min >= 3 {
                    n(ctx, "veteran_bonus")
                } else if q.difficulty == 3 {
                    n(ctx, "mixed_veteran_bonus")
                } else if q.difficulty == 5 {
                    n(ctx, "champion_bonus")
                } else {
                    0.
                };
                score += if q.length == 2 {
                    n(ctx, "medium_bonus")
                } else if q.length == 3 {
                    n(ctx, "long_bonus")
                } else {
                    0.
                }
            }
            if q.dungeon == "farm" && ctx.on("cohorts") {
                score -= n(ctx, "farm_penalty")
            }
            if poor {
                score += if q.difficulty == 1 && q.length == 1 {
                    n(ctx, "poverty_short_bonus")
                } else {
                    -n(ctx, "poverty_other_penalty")
                }
            }
            if ctx.on("income")
                && (mentions(&q.reward, "focus_ring")
                    || q.extra_rewards.iter().any(|r| r == "focus_ring"))
            {
                score += n(ctx, "focus_reward_bonus");
            }
            if ctx.on("cohorts") && native {
                score += hs
                    .iter()
                    .filter(|h| q.maximum_resolve_level.is_some_and(|max| h.level == max))
                    .count() as f64
                    * n(ctx, "window_bonus")
            }
            Some((score, q.clone(), p))
        })
        .collect::<Vec<_>>();
    options.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    if ctx.on("cohorts") {
        m.future_plan=options.iter().take(2).map(|(_,q,p)|json!({"quest":q.id,"party":p,"source":"current visible board only; next board unknown","recompute_on_return":true})).collect()
    }
    let selected = options
        .into_iter()
        .next()
        .map(|(_, q, p)| (q, p))
        .or_else(|| fallback(o, m, ctx));
    if ctx.on("growth")
        && let Some(intent) = m.state.get("next_trip_intent")
        && intent["funded"] == true
        && intent["returns"]
            .as_u64()
            .is_some_and(|r| r + 1 == u64::from(o.returns))
    {
        let intended = intent["party"]
            .as_array()
            .map(|p| p.iter().filter_map(Value::as_u64).collect::<Vec<_>>())
            .unwrap_or_default();
        let matched = selected.as_ref().map_or(0, |(_, p)| {
            p.iter().filter(|id| intended.contains(id)).count()
        });
        m.state.entry("next_trip_outcomes".into()).or_insert_with(||json!([])).as_array_mut().unwrap().push(json!({"week":o.week,"returns":o.returns,"intended":intended,"matched":matched,"actual":selected.as_ref().map(|(_,p)|p),"note":"prefer the intended roster only while current source and task gates still pass"}));
    }
    if ctx.profile.parameters["sync026"]["boss"]["enabled"] == true {
        m.state.remove("boss_match");
    }
    m.state.remove("boss_plan");
    m.state.remove("boss_quest");
    if let Some((q, p)) = &selected
        && ctx.on("boss")
    {
        if let Some(id) = super::boss::identify(&o.town, q, p, ctx) {
            if ctx.profile.parameters["sync026"]["boss"]["enabled"] == true {
                let count = super::sync026::boss_flex::matching(ctx.profile, &id, p, &o.town);
                m.state.insert(
                    "boss_match".into(),
                    json!({"plan":id,"matched_positions":count,"party":p}),
                );
            }
            m.state.insert("boss_plan".into(), json!(id));
            m.state.insert("boss_quest".into(), json!(q.id));
        } else if let Some(family) = ctx.profile.boss_parties.family(q) {
            m.state.insert("boss_fallback".into(),json!({"quest":q.id,"family":family,"reason":"no source-verified candidate matching executable roster, skill purchases and risk; functional party retained"}));
        }
    }
    selected
}
fn allowed(o: &Observation, m: &Memory, c: &crate::town016::Choice) -> bool {
    c.cost
        .iter()
        .all(|(k, n)| *n <= o.town.wallet.get(k).copied().unwrap_or(0))
        && m.ledger.permits(
            purpose(&Action::Town(c.action.clone())),
            c.cost.get("gold:").copied().unwrap_or(0),
            true,
        )
}
fn emit(
    o: &Observation,
    m: &Memory,
    ctx: &Context<'_>,
    a: TownAction,
    id: &str,
    stage: &str,
) -> Result<Option<Proposal>> {
    if let Some(c) = o.town.actions.iter().find(|c| c.action == a)
        && allowed(o, m, c)
    {
        return proposal(o, Action::Town(a), id, stage, ctx).map(Some);
    }
    Ok(None)
}
fn fund_facility(
    o: &Observation,
    m: &mut Memory,
    ctx: &Context<'_>,
    q: &crate::town016::Choice,
) -> Result<Option<Proposal>> {
    if !m.ledger.permits(
        ledger::Purpose::Buildings,
        q.cost.get("gold:").copied().unwrap_or(0),
        true,
    ) {
        return Ok(None);
    }
    if let Some(p) = emit(o, m, ctx, q.action.clone(), "S025-02", "facilities")? {
        m.state.remove("facility_funding");
        return Ok(Some(p));
    }
    let sync = ctx.profile.parameters["sync026"]["heirlooms"]["enabled"] == true;
    let committed = if sync
        && m.state
            .get("facility_funding")
            .is_some_and(|v| v["action"] == json!(q.action))
    {
        m.state["facility_funding"]["batches026"]
            .as_u64()
            .unwrap_or(0)
    } else {
        0
    };
    if let Some(plan) =
        super::sync026::funding_plan(&o.town, &m.ledger.heirlooms, q, ctx, committed)
        && let Some((index, _)) = plan.first()
    {
        let action = TownAction::Exchange {
            index: *index,
            batches: 1,
        };
        if let Some(p) = emit(o, m, ctx, action, "S025-02", "facilities_exchange")? {
            let mut funding = json!({"returns":o.returns,"action":q.action,"cost":q.cost});
            if sync {
                funding["batches026"] = json!(committed + 1);
            }
            m.state.insert("facility_funding".into(), funding);
            return Ok(Some(p));
        }
    }
    Ok(None)
}
pub fn facilities(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("town") {
        return Ok(None);
    }
    if ctx.profile.parameters["heirloom_exchange"]["mode"] != "complete_direct_native_surplus" {
        return Err("unknown025 heirloom funding policy".into());
    }
    if let Some(pending) = m.state.get("facility_funding").cloned() {
        if pending["returns"].as_u64() == Some(u64::from(o.returns))
            && let Some(q) = o.town.upgrade_quotes.iter().find(|q| {
                serde_json::to_value(&q.action).is_ok_and(|a| a == pending["action"])
                    && json!(q.cost) == pending["cost"]
            })
            && let Some(p) = fund_facility(o, m, ctx, q)?
        {
            return Ok(Some(p));
        }
        m.state.remove("facility_funding");
    }
    if ctx.on("income")
        && let Some(q) = o.board.as_ref().and_then(|b| {
            b.quests
                .iter()
                .find(|q| Some(&q.id) == m.legacy.quest.as_ref() && q.difficulty == 3)
        })
        && let Ok(plan) = veteran_plan(o, q, &m.legacy.target_party, ctx)
    {
        for wanted in plan
            .actions
            .iter()
            .filter(|c| matches!(c.action, TownAction::Upgrade { hero: None, .. }))
        {
            if let Some(quote) = o
                .town
                .upgrade_quotes
                .iter()
                .find(|c| c.action == wanted.action)
                && let Some(p) = fund_facility(o, m, ctx, quote)?
            {
                return Ok(Some(p));
            }
        }
    }
    let target = o
        .town
        .heroes
        .iter()
        .filter(|h| available(h) || h.alive && !h.quirks.contains_key("shard_hungry"))
        .map(|h| h.level)
        .max()
        .unwrap_or(0)
        .clamp(1, 4) as usize;
    for tree in ctx.profile.parameters["town"]["facility_order"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
    {
        let bought = o
            .town
            .purchased
            .iter()
            .filter(|(h, t, _)| h.is_none() && t == tree)
            .count();
        let cap = if tree.contains("recruit") {
            n(ctx, "recruit_tiers") as usize
        } else if tree.contains("roster") {
            n(ctx, "roster_tiers") as usize
        } else {
            target
        };
        if bought >= cap {
            continue;
        }
        let Some(q) = o
            .town
            .upgrade_quotes
            .iter()
            .find(|q| matches!(&q.action,TownAction::Upgrade{hero:None,tree:t,..}if t==tree))
        else {
            continue;
        };
        if let Some(p) = fund_facility(o, m, ctx, q)? {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

pub fn recruit(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("town") && !ctx.on("cohorts") {
        return Ok(None);
    }
    let alive = o.town.heroes.iter().filter(|h| h.alive).count();
    if alive >= n(ctx, "roster_target") as usize {
        return Ok(None);
    }
    let c = o
        .town
        .actions
        .iter()
        .filter(|c| matches!(c.action, TownAction::Hire { .. }) && allowed(o, m, c))
        .min_by_key(|c| {
            let TownAction::Hire { guid } = c.action else {
                unreachable!()
            };
            let h = o.town.recruits.iter().find(|h| h.guid == guid).unwrap();
            (
                o.town
                    .heroes
                    .iter()
                    .filter(|x| x.alive && x.class == h.class)
                    .count(),
                !ctx.profile.parameters["town"]["key_classes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c == &h.class),
                h.guid,
            )
        });
    match c {
        Some(c) => proposal(
            o,
            Action::Town(c.action.clone()),
            if ctx.on("town") { "S025-03" } else { "S025-15" },
            "recruit",
            ctx,
        )
        .map(Some),
        None => Ok(None),
    }
}
pub fn training(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("growth") {
        return Ok(None);
    }
    if ctx.on("income")
        && let Some(q) = o.board.as_ref().and_then(|b| {
            b.quests
                .iter()
                .find(|q| Some(&q.id) == m.legacy.quest.as_ref() && q.difficulty == 3)
        })
    {
        let goals =
            super::training::goals(&o.town, &m.legacy.target_party, gear_target(q, ctx), ctx);
        let plan = ctx.sources.training.plan(&o.town, &goals)?;
        if !plan.actions.is_empty() {
            for wanted in &plan.actions {
                if matches!(wanted.action, TownAction::Upgrade { hero: Some(_), .. })
                    && let Some(p) = emit(
                        o,
                        m,
                        ctx,
                        wanted.action.clone(),
                        "S025-04",
                        "veteran_planned_training",
                    )?
                {
                    return Ok(Some(p));
                }
            }
            // Required facilities are funded before ordinary preparation extras.
            return Ok(None);
        }
    }
    let mut choices = o
        .town
        .actions
        .iter()
        .filter(|c| allowed(o, m, c))
        .filter_map(|c| {
            let TownAction::Upgrade {
                hero: Some(id),
                tree,
                ..
            } = &c.action
            else {
                return None;
            };
            let h = o
                .town
                .heroes
                .iter()
                .find(|h| h.guid == *id && m.legacy.target_party.contains(id))?;
            let s = tree.strip_prefix(&format!("{}.", h.class))?;
            let priority = if s == "weapon" {
                0
            } else if s == "armour" {
                1
            } else if ctx.profile.parameters["skill_core"][&h.class]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v == s))
                && (selected(h, ctx).iter().any(|v| v == s)
                    || ctx.on("skills") && super::loadout::desired(h, ctx).iter().any(|v| v == s))
            {
                2
            } else if selected(h, ctx).iter().any(|v| v == s)
                || ctx.on("skills") && super::loadout::desired(h, ctx).iter().any(|v| v == s)
            {
                3
            } else {
                return None;
            };
            Some((priority, *id, c))
        })
        .collect::<Vec<_>>();
    choices.sort_by_key(|(p, id, _)| (*p, *id));
    if let Some((_, _, c)) = choices.first() {
        return proposal(
            o,
            Action::Town(c.action.clone()),
            "S025-04",
            "training",
            ctx,
        )
        .map(Some);
    }
    Ok(None)
}
pub fn loss(o: &Observation, h: &VisibleHero, m: &Memory, ctx: &Context<'_>) -> u64 {
    let paid = m.ledger.by_hero.get(&h.guid).copied().unwrap_or(0);
    let rebuild = o
        .town
        .upgrade_quotes
        .iter()
        .filter(|q| matches!(&q.action,TownAction::Upgrade{hero:Some(id),..}if *id==h.guid))
        .map(|q| q.cost.get("gold:").copied().unwrap_or(0))
        .sum::<u32>();
    let negative = h
        .quirks
        .iter()
        .filter(|(id, _)| {
            ctx.sources
                .cards
                .quirks
                .get(*id)
                .is_some_and(|q| q.negative)
        })
        .count() as u64;
    let positive = h.quirks.len() as u64 - negative;
    (paid.max(u64::from(rebuild))
        + u64::from(h.xp) * n(ctx, "xp_gold") as u64
        + positive * n(ctx, "positive_quirk_value") as u64)
        .saturating_sub(negative * n(ctx, "negative_quirk_value") as u64)
}
pub fn recovery(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("growth") || m.legacy.target_party.len() != 4 {
        return Ok(None);
    }
    // This also protects training priority when town order is disabled.
    if training(o, m, ctx)?.is_some() {
        return Ok(None);
    }
    super::recovery::intent(o, m, ctx)?;
    let mut cs=o.town.actions.iter().filter(|c|matches!(&c.action,TownAction::Queue{hero,treatment:Treatment::Stress,..}if !m.legacy.target_party.contains(hero))).filter(|c|allowed(o,m,c)&&super::recovery::permits(m,&c.action)).filter_map(|c|{let TownAction::Queue{hero,..}=c.action else{return None};let h=o.town.heroes.iter().find(|h|h.guid==hero&&available(h)&&h.stress>=n(ctx,"recovery_stress") as i32)?;super::recovery::key_class(&h.class,ctx).then_some((std::cmp::Reverse(h.stress),c.cost.get("gold:").copied().unwrap_or(0),hero,c))}).collect::<Vec<_>>();
    cs.sort_by_key(|(s, c, h, _)| (*s, *c, *h));
    if let Some((_, _, _, c)) = cs.first() {
        return proposal(
            o,
            Action::Town(c.action.clone()),
            "S025-10",
            "off_party_recovery",
            ctx,
        )
        .map(Some);
    }
    if !o.town.recruits.is_empty() {
        for h in o.town.heroes.iter().filter(|h| {
            available(h)
                && !m.legacy.target_party.contains(&h.guid)
                && h.stress >= n(ctx, "dismiss_stress") as i32
        }) {
            let cheapest=o.town.upgrade_quotes.iter().chain(o.town.actions.iter()).filter(|c|matches!(c.action,TownAction::Queue{hero,treatment:Treatment::Stress,..}if hero==h.guid)).filter_map(|c|c.cost.get("gold:")).min().copied().unwrap_or(n(ctx,"fallback_recovery_quote") as u32);
            let times = (h.stress.max(0) as u64).div_ceil(n(ctx, "recovery_per_visit") as u64);
            if loss(o, h, m, ctx) < u64::from(cheapest) * times
                && let Some(p) = emit(
                    o,
                    m,
                    ctx,
                    TownAction::Dismiss { hero: h.guid },
                    "S025-12",
                    "investment_dismiss",
                )?
            {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
}
pub fn quirks(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("quirks") {
        return Ok(None);
    }
    for c in &o.town.actions {
        let TownAction::Queue {
            hero, treatment, ..
        } = &c.action
        else {
            continue;
        };
        let Some(h) = o
            .town
            .heroes
            .iter()
            .find(|h| h.guid == *hero && available(h) && !m.legacy.target_party.contains(hero))
        else {
            continue;
        };
        let id = match treatment {
            Treatment::RemoveQuirk(id) => {
                if !ctx.sources.cards.quirks.get(id).is_some_and(|q| {
                    q.negative && (q.accuracy || q.dot_resistance || q.curio || q.skip_turn)
                }) {
                    continue;
                }
                "S025-08"
            }
            Treatment::LockQuirk(id) => {
                if h.level != n(ctx, "lock_level") as u32
                    || !ctx.profile.parameters["town"]["lock_quirks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v == id)
                {
                    continue;
                }
                "S025-09"
            }
            _ => continue,
        };
        if allowed(o, m, c) {
            return proposal(o, Action::Town(c.action.clone()), id, "quirks", ctx).map(Some);
        }
    }
    Ok(None)
}

fn fallback(o: &Observation, m: &Memory, ctx: &Context<'_>) -> Option<(QuestView, Vec<u64>)> {
    if ctx.profile.parameters["coherence029"] == true && ctx.on("stress") {
        let mut safe = o.clone();
        // W-088: a conservative subset guarantees the mean line for the old fallback.
        let limit = ctx.profile.modes[&ctx.sources.mode]["average_stress_short"]
            .as_f64()
            .unwrap();
        safe.town.heroes.retain(|h| f64::from(h.stress) <= limit);
        if let Some((q, p)) = auto_policy018::recovery_selection024b(&safe, &m.legacy)
            && !p.is_empty()
            && farm_ready(o, &q, &p, ctx)
            && court_ready(o, &q, &p, ctx)
            && regional_ready(o, &q, &p, ctx)
            && (q.difficulty != 3 || !ctx.on("income") || veteran_plan(o, &q, &p, ctx).is_ok())
        {
            return Some((q, p));
        }
        // No safe subset: retain finite existing emergency recovery instead of a strategy deadlock.
    }
    let mut public = o.clone();
    if ctx.on("cohorts")
        && o.town
            .heroes
            .iter()
            .filter(|h| {
                available(h)
                    && h.level >= n(ctx, "farm_level") as u32
                    && h.weapon >= n(ctx, "farm_gear") as usize
                    && h.armour >= n(ctx, "farm_gear") as usize
            })
            .count()
            < 4
        && let Some(b) = &mut public.board
    {
        b.quests.retain(|q| q.dungeon != "farm");
    }
    if let Some(board) = &mut public.board {
        board.quests.retain(|q| {
            o.town
                .heroes
                .iter()
                .filter(|h| available(h) && court_hero_ready(h, q, ctx))
                .count()
                >= 4
                || !court_gated(q, ctx)
        });
    }
    loop {
        let selected = auto_policy018::recovery_selection024b(&public, &m.legacy)?;
        if farm_ready(o, &selected.0, &selected.1, ctx)
            && court_ready(o, &selected.0, &selected.1, ctx)
            && regional_ready(o, &selected.0, &selected.1, ctx)
            && (!ctx.on("income")
                || selected.0.difficulty != 3
                || veteran_plan(o, &selected.0, &selected.1, ctx).is_ok())
        {
            return Some(selected);
        }
        let board = public.board.as_mut()?;
        let before = board.quests.len();
        board.quests.retain(|q| q.id != selected.0.id);
        if board.quests.len() == before {
            return None;
        }
    }
}
fn regional_tier<'a>(q: &QuestView, ctx: &'a Context<'_>) -> Option<&'a Vec<Value>> {
    let c = &ctx.profile.parameters["regional_boss_gate"];
    if q.kind != "kill_boss"
        || !c["regions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == &q.dungeon)
        || !q
            .plot
            .as_deref()
            .is_some_and(|p| p.starts_with(c["plot_prefix"].as_str().unwrap()))
    {
        return None;
    }
    c["minimum_levels"][q.difficulty.to_string()].as_array()
}
fn regional_minimum(h: &VisibleHero, q: &QuestView, ctx: &Context<'_>) -> bool {
    regional_tier(q, ctx).is_none_or(|tier| {
        tier.last()
            .is_some_and(|x| u64::from(h.level) >= x.as_u64().unwrap())
    })
}
pub(super) fn regional_ready(
    o: &Observation,
    q: &QuestView,
    party: &[u64],
    ctx: &Context<'_>,
) -> bool {
    let Some(tier) = regional_tier(q, ctx) else {
        return true;
    };
    if party.len() != tier.len() {
        return false;
    }
    let mut levels = party
        .iter()
        .filter_map(|id| {
            o.town
                .heroes
                .iter()
                .find(|h| h.guid == *id && available(h))
                .map(|h| h.level)
        })
        .collect::<Vec<_>>();
    levels.sort_unstable_by(|a, b| b.cmp(a));
    levels.len() == tier.len()
        && levels
            .iter()
            .zip(tier)
            .all(|(a, b)| u64::from(*a) >= b.as_u64().unwrap())
}
fn court_gated(q: &QuestView, ctx: &Context<'_>) -> bool {
    ctx.on("income") && q.dungeon == "courtyard"
}
fn court_hero_ready(h: &VisibleHero, q: &QuestView, ctx: &Context<'_>) -> bool {
    if !court_gated(q, ctx) {
        return true;
    }
    let cfg = &ctx.profile.parameters["court_gate"];
    if q.plot.as_deref() == cfg["intro"]["plot"].as_str() {
        let cap = cfg["intro"]["maximum_gear"].as_u64().unwrap() as usize;
        let target = (h.level as usize).min(cap);
        return h.level >= 1 && h.weapon >= target && h.armour >= target;
    }
    let key = q.difficulty.to_string();
    let minimum = cfg["minimum"][&key].as_u64().unwrap_or(u64::MAX);
    let gear = cfg["gear"][&key].as_u64().unwrap_or(u64::MAX);
    u64::from(h.level) >= minimum && h.weapon as u64 >= gear && h.armour as u64 >= gear
}
pub(super) fn court_ready(
    o: &Observation,
    q: &QuestView,
    party: &[u64],
    ctx: &Context<'_>,
) -> bool {
    if !court_gated(q, ctx) {
        return true;
    }
    if party.len() != 4
        || !party.iter().all(|id| {
            o.town
                .heroes
                .iter()
                .any(|h| h.guid == *id && available(h) && court_hero_ready(h, q, ctx))
        })
    {
        return false;
    }
    let cfg = &ctx.profile.parameters["court_gate"]["intro"];
    if q.plot.as_deref() != cfg["plot"].as_str() {
        return true;
    }
    let mut levels = party
        .iter()
        .map(|id| o.town.heroes.iter().find(|h| h.guid == *id).unwrap().level)
        .collect::<Vec<_>>();
    levels.sort_unstable_by(|a, b| b.cmp(a));
    if !levels
        .iter()
        .zip(cfg["minimum_levels"].as_array().unwrap())
        .all(|(a, b)| u64::from(*a) >= b.as_u64().unwrap())
    {
        return false;
    }
    let rs = party
        .iter()
        .enumerate()
        .map(|(i, id)| {
            roles(
                o.town.heroes.iter().find(|h| h.guid == *id).unwrap(),
                i as u8 + 1,
                ctx,
            )
        })
        .collect::<Vec<_>>();
    rs.iter()
        .any(|r| r[1] >= cfg["minimum_heal"].as_f64().unwrap())
        && rs
            .iter()
            .any(|r| r[2] >= cfg["minimum_relief"].as_f64().unwrap())
        && rs
            .iter()
            .any(|r| r[3] >= cfg["minimum_control"].as_f64().unwrap())
}
pub(super) fn farm_ready(o: &Observation, q: &QuestView, party: &[u64], ctx: &Context<'_>) -> bool {
    q.dungeon != "farm"
        || !ctx.on("cohorts")
        || party.len() == 4
            && party.iter().all(|id| {
                o.town.heroes.iter().any(|h| {
                    h.guid == *id
                        && available(h)
                        && h.level >= n(ctx, "farm_level") as u32
                        && h.weapon >= n(ctx, "farm_gear") as usize
                        && h.armour >= n(ctx, "farm_gear") as usize
                })
            })
}

fn mentions(v: &Value, id: &str) -> bool {
    match v {
        Value::String(s) => s == id,
        Value::Array(a) => a.iter().any(|v| mentions(v, id)),
        Value::Object(m) => m.values().any(|v| mentions(v, id)),
        _ => false,
    }
}
fn region(h: &VisibleHero, q: &QuestView, ctx: &Context<'_>) -> f64 {
    let a = &ctx.profile.parameters["composition"]["region_resists"][&q.dungeon];
    selected(h,ctx).iter().filter_map(|s|ctx.sources.cards.skills.get(&h.class).and_then(|m|m.get(s))).map(|s|if s.bleed{1.-a.get(0).and_then(Value::as_f64).unwrap_or(0.5)}else{0.}+if s.blight{1.-a.get(1).and_then(Value::as_f64).unwrap_or(0.5)}else{0.}).sum::<f64>()*n(ctx,"region_weight")
}
fn templates(
    t: &crate::town016::Observation,
    q: &QuestView,
    default: Vec<u64>,
    ctx: &Context<'_>,
) -> Vec<u64> {
    let value = |p: &[u64]| {
        p.iter()
            .enumerate()
            .filter_map(|(i, id)| {
                t.heroes.iter().find(|h| h.guid == *id).map(|h| {
                    rank_score(h, i as u8 + 1, ctx) + region(h, q, ctx)
                        - f64::from(training_cost(t, h, gear_target(q, ctx), ctx))
                            * n(ctx, "training_cost_weight")
                })
            })
            .sum::<f64>()
    };
    let mut best = (value(&default), default);
    for template in ctx.profile.parameters["composition"]["templates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(slots) = template["slots"].as_array() else {
            continue;
        };
        let mut p = vec![];
        for (i, allowed) in slots.iter().enumerate() {
            let Some(allowed) = allowed.as_array() else {
                break;
            };
            let h = t
                .heroes
                .iter()
                .filter(|h| {
                    !p.contains(&h.guid)
                        && compatible(t, &p, h)
                        && (allowed.is_empty() || allowed.iter().any(|c| c == &h.class))
                })
                .max_by(|a, b| {
                    let v = |h: &VisibleHero| {
                        rank_score(h, i as u8 + 1, ctx) + region(h, q, ctx)
                            - f64::from(training_cost(t, h, gear_target(q, ctx), ctx))
                                * n(ctx, "training_cost_weight")
                    };
                    v(a).total_cmp(&v(b)).then_with(|| b.guid.cmp(&a.guid))
                });
            if let Some(h) = h {
                p.push(h.guid)
            } else {
                break;
            }
        }
        if p.len() == 4 {
            let v = value(&p) + n(ctx, "template_bonus");
            if v > best.0 {
                best = (v, p)
            }
        }
    }
    best.1
}

fn escorts<'a>(
    t: &'a crate::town016::Observation,
    q: &QuestView,
    ctx: &Context<'_>,
) -> Vec<&'a VisibleHero> {
    let d = q.difficulty.to_string();
    let cfg = &ctx.profile.parameters["merchant"];
    let mut hs = t
        .heroes
        .iter()
        .filter(|h| {
            available(h)
                && h.class != "antiquarian"
                && h.stress < 70
                && h.level >= q.difficulty.saturating_sub(1)
                && q.maximum_resolve_level.is_none_or(|m| h.level <= m)
                && power(h) >= cfg["escort_power"][&d].as_f64().unwrap_or(f64::INFINITY)
                && h.weapon >= cfg["escort_gear"][&d].as_u64().unwrap_or(u64::MAX) as usize
                && h.armour >= cfg["escort_gear"][&d].as_u64().unwrap_or(u64::MAX) as usize
                && crate::preparation024::strength(h)[3]
                    >= cfg["escort_skills"][&d].as_f64().unwrap_or(f64::INFINITY)
        })
        .collect::<Vec<_>>();
    hs.sort_by(|a, b| {
        power(b)
            .total_cmp(&power(a))
            .then_with(|| a.guid.cmp(&b.guid))
    });
    hs.truncate(cfg["candidate_limit"].as_u64().unwrap() as usize);
    hs
}
fn merchant(t: &crate::town016::Observation, q: &QuestView, ctx: &Context<'_>) -> Option<Vec<u64>> {
    let cfg = &ctx.profile.parameters["merchant"];
    if !ctx.on("poverty")
        || t.wallet.get("gold:").copied().unwrap_or(0)
            >= cfg["poverty_gold"].as_u64().unwrap() as u32
        || q.plot.is_some()
        || ctx.profile.parameters["composition"]["region_resists"]
            .get(&q.dungeon)
            .is_none()
        || ![1, 3].contains(&q.difficulty)
        || q.length > 2
    {
        return None;
    }
    let es = escorts(t, q, ctx);
    let mut best = None;
    for ant in t.heroes.iter().filter(|h| {
        available(h)
            && h.class == "antiquarian"
            && h.stress < 70
            && q.maximum_resolve_level.is_none_or(|m| h.level <= m)
    }) {
        for a in 0..es.len() {
            for b in a + 1..es.len() {
                for c in b + 1..es.len() {
                    let p = arrange(t, vec![es[a].guid, es[b].guid, es[c].guid, ant.guid], ctx);
                    if p.iter().enumerate().any(|(i, id)| {
                        !compatible(t, &p[..i], t.heroes.iter().find(|h| h.guid == *id).unwrap())
                    }) {
                        continue;
                    }
                    let rs = p
                        .iter()
                        .enumerate()
                        .map(|(i, id)| {
                            roles(
                                t.heroes.iter().find(|h| h.guid == *id).unwrap(),
                                i as u8 + 1,
                                ctx,
                            )
                        })
                        .collect::<Vec<_>>();
                    if q.length > 1
                        && (rs.iter().map(|r| r[1]).fold(0f64, f64::max)
                            < n(ctx, "medium_min_heal")
                            || rs.iter().map(|r| r[2]).fold(0f64, f64::max)
                                < n(ctx, "medium_min_relief")
                            || rs.iter().map(|r| r[3]).sum::<f64>()
                                < ctx.profile.parameters["composition"]["medium_min_control"]
                                    .as_f64()
                                    .unwrap())
                    {
                        continue;
                    }
                    let score = p
                        .iter()
                        .enumerate()
                        .map(|(i, id)| {
                            rank_score(
                                t.heroes.iter().find(|h| h.guid == *id).unwrap(),
                                i as u8 + 1,
                                ctx,
                            )
                        })
                        .sum::<f64>();
                    if best.as_ref().is_none_or(|(s, _)| score > *s) {
                        best = Some((score, p));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

pub(super) fn eligible_town(
    o: &Observation,
    q: &QuestView,
    ctx: &Context<'_>,
    relax: bool,
) -> crate::town016::Observation {
    let minimum = if q.dungeon == "farm" && ctx.on("cohorts") || q.difficulty >= 6 {
        n(ctx, "farm_level") as u32
    } else if q.difficulty >= 5 {
        ctx.profile.modes[&ctx.sources.mode]["champion_min_level"]
            .as_u64()
            .unwrap() as u32
    } else if q.difficulty >= 3 {
        n(ctx, "veteran_min_level") as u32
    } else {
        0
    };
    let mut t = o.town.clone();
    t.heroes.retain(|h| regional_minimum(h, q, ctx));
    t.heroes.retain(|h| court_hero_ready(h, q, ctx));
    t.heroes.retain(|h| {
        available(h)
            && q.maximum_resolve_level.is_none_or(|m| h.level <= m)
            && (relax || h.level >= minimum)
            && h.stress
                < if relax {
                    201
                } else if ctx.on("stress") {
                    ctx.profile.modes[&ctx.sources.mode]["worst_stress"]
                        .as_i64()
                        .unwrap() as i32
                } else {
                    70
                }
            && !(q.dungeon == "darkestdungeon"
                && o.automation
                    .as_ref()
                    .is_some_and(|a| !a.can_reenter && a.marked.contains(&h.guid)))
    });
    if q.dungeon == "farm" && ctx.on("cohorts") {
        t.heroes.retain(|h| {
            h.level >= n(ctx, "farm_level") as u32
                && h.weapon >= n(ctx, "farm_gear") as usize
                && h.armour >= n(ctx, "farm_gear") as usize
        })
    }
    t
}
