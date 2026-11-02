//! Public field rescue before the legacy safety exit; no hidden state or new mechanics.
use super::*;
use crate::opening015::Action as A;
use dd_data::{hero010::CarryDotKind as D, inventory015::Item};
#[derive(Clone)]
pub struct Facts {
    pub zero_progress_guard: bool,
    pub heal_ratio: f64,
    pub feast_rations: f64,
    pub feast_stress: i32,
    pub full_rations: f64,
    pub relief_skills: BTreeMap<(String, String), Vec<String>>,
}
impl Facts {
    pub fn project(game: &GameData) -> Result<Self> {
        let v = game.overlay.json("shared/rules.json")?;
        let m = v["meals_table"]
            .as_array()
            .ok_or("026 missing original meals")?
            .iter()
            .find(|m| m["stress"].as_i64().is_some_and(|n| n < 0))
            .ok_or("026 missing original relief meal")?;
        let mut relief_skills = BTreeMap::<(String, String), Vec<String>>::new();
        let points = v["camp_start_camping_points"]
            .as_u64()
            .ok_or("026 missing camp points")?;
        for path in game
            .overlay
            .paths
            .keys()
            .filter(|p| p.starts_with("raid/camping/") && p.ends_with(".camping_skills.json"))
        {
            for bytes in game.overlay.read_town_versions(path)? {
                let raw: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                for row in raw["skills"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|r| r["cost"].as_u64().is_some_and(|n| n <= points))
                {
                    let scopes = row["effects"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|e| {
                            e["type"] == "stress"
                                && e["amount"].as_f64().is_some_and(|n| n < 0.)
                                && e["chance"].as_f64().is_some_and(|n| n >= 1.)
                                && e["requirements"].as_array().is_some_and(Vec::is_empty)
                        })
                        .filter_map(|e| e["selection"].as_str().map(str::to_string))
                        .collect::<Vec<_>>();
                    if scopes.is_empty() {
                        continue;
                    }
                    let id = row["id"].as_str().ok_or("026 camp skill id absent")?;
                    let classes = row["hero_classes"]
                        .as_array()
                        .ok_or("026 camp class list absent")?;
                    if classes.is_empty() {
                        relief_skills.insert(("*".into(), id.into()), scopes);
                    } else {
                        for c in classes.iter().filter_map(Value::as_str) {
                            relief_skills.insert((c.into(), id.into()), scopes.clone());
                        }
                    }
                }
            }
        }
        Ok(Self {
            zero_progress_guard: false,
            heal_ratio: v["provision_hp_heal"]
                .as_f64()
                .ok_or("026 missing original food heal")?,
            feast_rations: m["rations_per"]
                .as_f64()
                .ok_or("026 missing meal rations")?,
            full_rations: v["meals_table"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| {
                    m["healing"].as_f64().is_some_and(|n| n >= 0.)
                        && m["stress"].as_i64() == Some(0)
                })
                .filter_map(|m| m["rations_per"].as_f64())
                .max_by(f64::total_cmp)
                .unwrap_or(0.),
            relief_skills,
            feast_stress: m["stress"].as_i64().ok_or("026 missing meal stress")? as i32,
        })
    }
}
pub fn pending_loss(h: &crate::opening015::VisibleHero) -> i64 {
    let dots = h
        .status
        .dots
        .iter()
        .filter(|d| matches!(d.kind, D::Bleed | D::Blight))
        .map(|d| i64::from(d.amount.max(0)) * i64::from(d.remaining))
        .sum::<i64>();
    let mut burn = i64::from(h.status.burn.max(0));
    let mut total = dots;
    while burn > 0 {
        total += burn;
        burn /= 2;
    }
    total
}
fn emit(
    o: &Observation,
    m: &mut Memory,
    ctx: &Context<'_>,
    a: A,
    reason: &str,
) -> Result<Option<Proposal>> {
    m.state.insert(
        "field_rescue026_last".into(),
        json!({"week":o.week,"returns":o.returns,"reason":reason,"action":a}),
    );
    proposal(o, Action::Raid(a), "S026-03", reason, ctx).map(Some)
}
// A repeated complete public input proves the new early-exit path is cycling.
// Reuse the documented legacy gate for this sortie, rather than changing money,
// native week rules, or every ordinary risk exit.
pub(super) fn defer_repeated_exit(
    o: &Observation,
    m: &mut Memory,
    ctx: &Context<'_>,
) -> Result<bool> {
    if !ctx
        .sources
        .field026
        .as_ref()
        .is_some_and(|f| f.zero_progress_guard)
    {
        return Ok(false);
    }
    let r = o
        .expedition
        .as_ref()
        .ok_or("026 cycle guard without raid")?;
    let rooms = r.map.as_ref().map_or(0, |map| {
        map.nodes.iter().filter(|n| n.room && n.visited).count()
    });
    let legacy = ctx.profile.parameters["dungeon"]["legacy_exit"]["visited_rooms"]
        .as_u64()
        .ok_or("026 missing legacy fallback gate")? as usize;
    let quest = m
        .metrics
        .active
        .as_ref()
        .map(|v| v["quest"].clone())
        .unwrap_or_else(|| json!({"legacy":m.legacy.quest}));
    let training = o
        .town
        .heroes
        .iter()
        .filter(|h| r.party.contains(&h.guid))
        .collect::<Vec<_>>();
    let signature = json!({"week":o.week,"quest":quest,"party":r.party,"training":training,"heroes":r.heroes,"inventory":r.inventory,"wallet":r.wallet,"town_wallet":o.town.wallet,"torch":r.torch,"map":r.map});
    let active = m
        .state
        .get("zero_progress_guard026")
        .is_some_and(|v| v["week"] == o.week && v["returns"] == o.returns && v["quest"] == quest);
    let repeat = m.state.get("field_exit_signature026").is_some_and(|v| {
        v["signature"] == signature
            && v["returns"]
                .as_u64()
                .is_some_and(|n| n + 1 == u64::from(o.returns))
    });
    if !active && repeat && rooms < legacy {
        let count = m
            .state
            .get("zero_progress_guard026")
            .and_then(|v| v["count"].as_u64())
            .unwrap_or(0)
            + 1;
        m.state.insert("zero_progress_guard026".into(),json!({"count":count,"week":o.week,"returns":o.returns,"quest":quest,"fallback":"existing_legacy_room_gate","visited_rooms":legacy}));
    }
    if (active || repeat) && rooms < legacy {
        return Ok(true);
    }
    m.state.insert(
        "field_exit_signature026".into(),
        json!({"returns":o.returns,"signature":signature}),
    );
    Ok(false)
}
pub fn choose(o: &Observation, m: &mut Memory, ctx: &Context<'_>) -> Result<Option<Proposal>> {
    let r = o.expedition.as_ref().ok_or("026 missing field view")?;
    if !ctx.on("exploration")
        || r.battle.is_some()
        || r.camping.is_some()
        || !r.loot.is_empty()
        || r.quest_ready
    {
        return Ok(None);
    }
    let hs = r
        .heroes
        .iter()
        .filter(|h| h.alive && r.party.contains(&h.guid))
        .collect::<Vec<_>>();
    // Every cure below is already legal under the native source-backed item implementation.
    for h in &hs {
        for (item, kind) in [
            ("bandage", D::Bleed),
            ("antivenom", D::Blight),
            ("laudanum", D::Horror),
        ] {
            if h.status.dots.iter().any(|d|d.kind==kind&&d.remaining>0)&&let Some(a)=r.actions.iter().find(|a|matches!(a,A::Use{hero,slot}if *hero==h.guid&&r.inventory.slots[*slot].as_ref().is_some_and(|s|s.item.id==item))) {return emit(o,m,ctx,a.clone(),"cure_dot_before_exit");}
        }
    }
    // Feed natively one bite at a time; verify actual HP again before any walking.
    // Ignoring future regen/camps and requiring an up-front buffer is conservative.
    for h in &hs {
        let loss = pending_loss(h);
        if i64::from(h.hp)<=loss&&i64::from(h.maximum_hp)>loss&&h.food_remaining>0&&let Some(a)=r.actions.iter().find(|a|matches!(a,A::Use{hero,slot}if *hero==h.guid&&r.inventory.slots[*slot].as_ref().is_some_and(|s|s.item.kind=="provision"))) {return emit(o,m,ctx,a.clone(),"native_food_before_exit");}
    }
    let stress = ctx.profile.parameters["dungeon"]["legacy_exit"]["stress"]
        .as_i64()
        .ok_or("026 missing stress gate")?;
    let pressure = hs.iter().any(|h| i64::from(h.stress) >= stress);
    let start = A::Camp(crate::expedition017::CampAction::Start);
    if pressure
        && r.actions.contains(&start)
        && let Some(f) = &ctx.sources.field026
    {
        // Nominal original feast quote; the actual legal camping menu still owns pricing/modifiers.
        let food = r.inventory.count(&Item::new("provision", ""));
        let skill_relief = o
            .town
            .heroes
            .iter()
            .filter(|h| r.party.contains(&h.guid))
            .any(|actor| {
                actor.selected_camping.iter().any(|id| {
                    f.relief_skills
                        .get(&(actor.class.clone(), id.clone()))
                        .or_else(|| f.relief_skills.get(&("*".into(), id.clone())))
                        .is_some_and(|scopes| {
                            hs.iter()
                                .filter(|h| i64::from(h.stress) >= stress)
                                .any(|target| {
                                    scopes.iter().any(|s| {
                                        s == "party"
                                            || s == "self" && actor.guid == target.guid
                                            || ["party_other", "individual"].contains(&s.as_str())
                                                && actor.guid != target.guid
                                    })
                                })
                        })
                })
            });
        if f.feast_stress < 0 && f64::from(food) >= f.feast_rations * hs.len() as f64
            || skill_relief && f64::from(food) >= f.full_rations * hs.len() as f64
        {
            return emit(o, m, ctx, start, "camp_before_stress_exit");
        }
    }
    let rooms = r.map.as_ref().map_or(0, |map| {
        map.nodes.iter().filter(|n| n.room && n.visited).count()
    });
    let minimum = ctx.profile.parameters["sync026"]["field"]["minimum_visited_rooms"]
        .as_u64()
        .ok_or("026 missing room variant")? as usize;
    let risk = pressure
        || hs.iter().any(|h| i64::from(h.hp) <= pending_loss(h))
        || ctx.profile.parameters["coherence029"] != true
            && r.inventory.count(&Item::new("provision", "")) < r.party.len() as u32;
    if risk
        && (m.legacy.moves as u64)
            >= ctx.profile.parameters["sync026"]["field"]["minimum_moves"]
                .as_u64()
                .ok_or("026 missing movement gate")?
        && rooms >= minimum
        && r.actions.contains(&A::Leave { complete: false })
    {
        if defer_repeated_exit(o, m, ctx)? {
            return Ok(None);
        }
        return emit(
            o,
            m,
            ctx,
            A::Leave { complete: false },
            "verified_field_risk_exit",
        );
    }
    Ok(None)
}
pub fn retreat_record(
    view: &crate::campaign018::runner::View,
    log: &Value,
    bridge: Option<&Value>,
) -> Option<Value> {
    let a = &log["action"];
    let kind = if a["Raid"]["Leave"]["complete"] == false {
        "raid_abandon"
    } else if a["Raid"]["Fight"] == "Retreat" {
        "combat_attempt"
    } else if a["Farm"] == "Exit" {
        "farm_exit"
    } else {
        return None;
    };
    let (raid, training) = match view {
        crate::campaign018::runner::View::Opening(r) => (Some(r.as_ref()), Value::Null),
        crate::campaign018::runner::View::Campaign(o) => (
            o.expedition.as_ref(),
            json!(
                o.town
                    .heroes
                    .iter()
                    .filter(|h| o.town.party.contains(&h.guid))
                    .collect::<Vec<_>>()
            ),
        ),
    };
    let rooms = raid
        .and_then(|r| r.map.as_ref())
        .map(|m| m.nodes.iter().filter(|n| n.room && n.visited).count());
    Some(
        json!({"step":log["step"],"week":log["before"]["week"],"kind":kind,"visited_rooms":rooms,"at_most_two_rooms":rooms.map(|n|n<=2),"quest":log["before"]["quest"],"party":raid.map(|r|&r.party),"training":training,"field_heroes":raid.map(|r|&r.heroes),"battle_units":raid.and_then(|r|r.battle.as_ref()).map(|b|&b.units),"battle_heroes":bridge.map(|f|&f["battle_heroes"])}),
    )
}
