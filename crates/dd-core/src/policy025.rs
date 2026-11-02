//! Idempotent public town stages. Later phases replace these baseline operations.
use super::*;
pub fn choose(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if o.expedition.is_some() && !m.legacy.was_raiding {
        m.legacy.raids += 1;
        m.legacy.moves = 0;
        m.legacy.was_raiding = true;
    } else if o.expedition.is_none() {
        m.legacy.was_raiding = false;
    }
    if let Some(p) = super::combat::raid(o, m, ctx)? {
        return Ok(Some(p));
    }
    if o.expedition.is_some() {
        let p = super::dungeon::choose(o, m, ctx)?;
        if p.as_ref()
            .is_some_and(|p| matches!(p.action, Action::Raid(crate::opening015::Action::Move(_))))
        {
            // Fallback018 counts its own moves; only count025-owned moves here.
            m.legacy.moves += 1;
        }
        return Ok(p);
    }
    if o.content019.as_ref().is_some_and(|c| c.farm.is_some()) {
        return Ok(None);
    }
    if m.returns != Some(o.returns) {
        m.returns = Some(o.returns);
        m.legacy.last_return = o.returns;
        m.legacy.quest = None;
        m.legacy.target_party.clear();
        m.legacy.town_steps = 0;
    }
    if let Some(offer) = &o.event_offer {
        let mut d = auto_policy018::choose(o, &mut m.legacy)?;
        let cost = price(o, &d.action)?;
        if !m.ledger.permits(purpose(&d.action), cost, false)
            && offer
                .actions
                .contains(&crate::campaign017::EventAction::Decline)
        {
            d.action = Action::Event(crate::campaign017::EventAction::Decline);
        }
        return proposal(o, d.action, "S025-00", "events", ctx).map(Some);
    }
    if let Some(p) = &o.provisions {
        if ctx.on("exploration") && !ctx.sources.mode.ends_with(":dark") {
            let cfg: crate::preparation024::provisions::Settings =
                serde_json::from_value(ctx.profile.parameters["supplies"].clone())
                    .map_err(|e| e.to_string())?;
            if let Some(q) = o
                .board
                .as_ref()
                .and_then(|b| b.quests.iter().find(|q| q.id == p.quest))
                && let Some(a) = if ctx.profile.parameters["coherence029"] == true {
                    crate::preparation024::provisions::choose_food_first(&cfg, q, p)
                } else {
                    crate::preparation024::provisions::choose(&cfg, q, p)
                }
            {
                return proposal(o, Action::Provision(a), "S025-11", "quoted_provisions", ctx)
                    .map(Some);
            }
        }
        let d = auto_policy018::choose(o, &mut m.legacy)?;
        return proposal(o, d.action, "S025-00", "supplies", ctx).map(Some);
    }
    for step in [
        unload as fn(&Observation, &mut Memory, &Context<'_>) -> Result<Option<Proposal>>,
        facilities,
        recruit,
    ] {
        if let Some(p) = step(o, m, ctx)? {
            return Ok(Some(p));
        }
    }
    let valid = m.legacy.quest.as_ref().is_some_and(|id| {
        o.board
            .as_ref()
            .is_some_and(|b| b.quests.iter().any(|q| &q.id == id))
    }) && !m.legacy.target_party.is_empty()
        && m.legacy.target_party.iter().all(|id| {
            o.town
                .heroes
                .iter()
                .any(|h| h.guid == *id && h.alive && !h.busy && !h.missing)
        });
    if !valid {
        let Some((q, p)) = super::town::select(o, m, ctx) else {
            if let Some(c) = o.town.actions.iter().find(|c| {
                matches!(
                    c.action,
                    TownAction::Cancel { .. } | TownAction::Hire { .. }
                ) && m.ledger.permits(
                    purpose(&Action::Town(c.action.clone())),
                    c.cost.get("gold:").copied().unwrap_or(0),
                    false,
                )
            }) {
                return proposal(
                    o,
                    Action::Town(c.action.clone()),
                    "S025-00",
                    "finite_town_recovery",
                    ctx,
                )
                .map(Some);
            }
            return Err("025 no executable public task/party or legal hire/cancel recovery".into());
        };
        m.legacy.quest = Some(q.id);
        m.legacy.target_party = p;
    }
    if o.town.party != m.legacy.target_party {
        return proposal(
            o,
            Action::Town(TownAction::Party(m.legacy.target_party.clone())),
            selection_rule(m, ctx),
            "select_party",
            ctx,
        )
        .map(Some);
    }
    if o.board.as_ref().and_then(|b| b.selected.as_ref()) != m.legacy.quest.as_ref() {
        return proposal(
            o,
            Action::Select(m.legacy.quest.clone().unwrap()),
            selection_rule(m, ctx),
            "select_quest",
            ctx,
        )
        .map(Some);
    }
    if let Some(p) = replan_training_budget029(o, m, ctx)? {
        return Ok(Some(p));
    }
    let order = if ctx.ordered {
        &ctx.profile.town_order
    } else {
        &ctx.profile.legacy_order
    };
    for stage in order {
        let p = match stage.as_str() {
            "events" => events(o, m, ctx),
            "unload" => unload(o, m, ctx),
            "facilities" => facilities(o, m, ctx),
            "recruit" => recruit(o, m, ctx),
            "plan" => plan(o, m, ctx),
            "skills" => skills(o, m, ctx),
            "training" => training(o, m, ctx),
            "trinkets" => trinkets(o, m, ctx),
            "sell" => sell(o, m, ctx),
            "quirks" => quirks(o, m, ctx),
            "recovery" => recovery(o, m, ctx),
            "supplies" => supplies(o, m, ctx),
            _ => return Err(format!("unknown025 pipeline stage {stage}")),
        }?;
        if p.is_some() {
            return Ok(p);
        }
    }
    let mut visible = o.clone();
    let selected = true;
    visible.town.actions.retain(|c| {
        let a = Action::Town(c.action.clone());
        let cost = c.cost.get("gold:").copied().unwrap_or(0);
        let relevant = match &c.action {
            TownAction::Upgrade { hero: Some(id), .. } | TownAction::LearnCamp { hero: id, .. } => {
                m.legacy.target_party.contains(id) && !ctx.on("growth")
            }
            TownAction::Queue {
                hero, treatment, ..
            } => {
                !m.legacy.target_party.contains(hero)
                    && !(matches!(treatment, crate::town016::Treatment::Stress) && ctx.on("growth")
                        || !matches!(treatment, crate::town016::Treatment::Stress)
                            && ctx.on("quirks"))
            }
            TownAction::Upgrade { hero: None, .. } | TownAction::Exchange { .. } => !ctx.on("town"),
            TownAction::Hire { .. } => !ctx.on("town") && !ctx.on("cohorts"),
            TownAction::SelectSkills { .. } => !ctx.on("skills"),
            TownAction::Equip { .. } | TownAction::Unequip { .. } => !ctx.on("trinkets"),
            TownAction::Dismiss { .. } => !ctx.on("growth"),
            _ => true,
        };
        relevant && m.ledger.permits(purpose(&a), cost, selected)
    });
    let city = if ctx.on("skills") {
        crate::campaign_policy017::city_without_skill_selection025
    } else {
        crate::campaign_policy017::city
    };
    let d = auto_policy018::choose_with_city025(&visible, &mut m.legacy, city)?;
    proposal(o, d.action, "S025-00", "legacy_preparation", ctx).map(Some)
}
pub use super::town::{facilities, quirks, recovery, recruit, training};
pub fn unload(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if !ctx.on("trinkets")
        || m.state.get("unloaded_return").and_then(Value::as_u64) == Some(u64::from(o.returns))
    {
        return Ok(None);
    }
    if let Some(c) = o
        .town
        .actions
        .iter()
        .find(|c| matches!(c.action, TownAction::Unequip { .. }))
    {
        return proposal(o, Action::Town(c.action.clone()), "S025-06", "unload", ctx).map(Some);
    }
    m.state.insert("unloaded_return".into(), json!(o.returns));
    Ok(None)
}
macro_rules! stage {($($name:ident),*)=>{$(pub fn $name(_o:&Observation,_m:&mut Memory,_ctx:&Context<'_>)->Result<Option<Proposal>>{Ok(None)})*};}
stage!(events, plan);
pub use super::loadout::{sell, trinkets};
pub fn skills(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    if let Some(p) = super::loadout::skills(o, m, ctx)? {
        return Ok(Some(p));
    }
    super::dungeon::town_camp(o, m, ctx, false)
}
pub fn supplies(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    super::dungeon::town_camp(o, m, ctx, true)
}

fn selection_rule<'a>(m: &Memory, ctx: &Context<'_>) -> &'a str {
    if ctx.on("boss") && m.state.contains_key("boss_plan") {
        "S025-17"
    } else if ctx.on("party") {
        "S025-01"
    } else if ctx.on("income") {
        "S025-13"
    } else if ctx.on("cohorts") {
        "S025-15"
    } else if ctx.on("stress") {
        "S025-18"
    } else if ctx.ordered {
        "S025-14"
    } else {
        "S025-00"
    }
}

/// W-111: the admission promise must pass the same actual supply ledger as spending.
pub(super) fn replan_training_budget029(
    o: &Observation,
    m: &mut Memory,
    ctx: &Context<'_>,
) -> Result<Option<Proposal>> {
    if ctx.profile.parameters["coherence029"] != true || !ctx.on("income") {
        return Ok(None);
    }
    let Some(q) = o.board.as_ref().and_then(|b| {
        b.quests
            .iter()
            .find(|q| Some(&q.id) == m.legacy.quest.as_ref() && q.difficulty == 3)
    }) else {
        return Ok(None);
    };
    let goals = super::training::goals(
        &o.town,
        &m.legacy.target_party,
        super::town::gear_target(q, ctx),
        ctx,
    );
    let plan = ctx.sources.training.plan(&o.town, &goals)?;
    let gold = plan.cost.get("gold:").copied().unwrap_or(0);
    if m.ledger.permits(ledger::Purpose::Training, gold, true) {
        return Ok(None);
    }
    m.state.insert("training_budget029".into(),json!({"returns":o.returns,"quest":q.id,"required_gold":gold,"wallet":m.ledger.gold,"supplies":m.ledger.supplies,"reserve":m.ledger.reserve,"reason":"W-111 uses the actual shared spending ledger"}));
    let mut alternative = o.clone();
    if let Some(b) = &mut alternative.board {
        b.quests.retain(|a| {
            a.difficulty < q.difficulty || a.difficulty == q.difficulty && a.length < q.length
        });
    }
    let mut memory = m.clone();
    memory.legacy.quest = None;
    memory.legacy.target_party.clear();
    if let Some((task, party)) = super::town::select(&alternative, &mut memory, ctx) {
        memory.legacy.quest = Some(task.id.clone());
        memory.legacy.target_party = party.clone();
        *m = memory;
        let action = if o.town.party != party {
            Action::Town(TownAction::Party(party))
        } else {
            Action::Select(task.id)
        };
        return proposal(o, action, "S025-13", "shared_training_budget_fallback", ctx).map(Some);
    }
    // No new money or new strategy. Preserve the existing finite emergency route;
    // report this unsupported operating corner instead of introducing a deadlock.
    m.state.get_mut("training_budget029").unwrap()["unresolved_no_alternative"] = json!(true);
    Ok(None)
}
