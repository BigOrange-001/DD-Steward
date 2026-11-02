//! Nearest source-checked Boss template with functional replacements (026 only).
use super::*;
#[derive(Clone)]
struct Partial {
    ids: Vec<u64>,
    matched: usize,
    score: f64,
    heal: bool,
    relief: bool,
    controls: u8,
}
pub fn party(
    t: &crate::town016::Observation,
    q: &crate::campaign017::QuestView,
    ctx: &Context<'_>,
) -> Option<(String, Vec<u64>)> {
    let family = ctx.profile.boss_parties.family(q)?;
    let cfg = &ctx.profile.parameters["sync026"]["boss"];
    let keep = cfg["beam_per_function_signature"].as_u64()? as usize;
    let mut values = BTreeMap::new();
    for h in &t.heroes {
        for rank in 1..=4 {
            values.insert(
                (h.guid, rank),
                (town::roles(h, rank, ctx), town::rank_score(h, rank, ctx)),
            );
        }
    }
    let mut choices = vec![];
    for p in ctx.profile.boss_parties.plans.iter().filter(|p| {
        p.family == family
            && ["采纳", "条件采纳"].contains(&p.decision.as_str())
            && boss::matches_scope(p, q)
            && ctx
                .sources
                .boss_checks
                .get(&p.id)
                .is_some_and(|v| v["usable"] == true)
    }) {
        let mut beam = vec![Partial {
            ids: vec![],
            matched: 0,
            score: 0.,
            heal: false,
            relief: false,
            controls: 0,
        }];
        for rank in 1..=4 {
            let member = p.slots.iter().find(|m| m.rank == rank)?;
            let mut buckets = BTreeMap::<(bool, bool, u8), Vec<Partial>>::new();
            for prev in &beam {
                for h in t.heroes.iter().filter(|h| {
                    h.alive
                        && !h.busy
                        && !h.missing
                        && !h.quirks.contains_key("shard_hungry")
                        && !prev.ids.contains(&h.guid)
                        && crate::preparation024::composition::compatible(t, &prev.ids, h)
                }) {
                    if family == "vvulf"
                        && (h.level
                            < ctx.profile.boss_parties.policy["vvulf_min_level"].as_u64()? as u32
                            || h.weapon
                                < ctx.profile.boss_parties.policy["vvulf_min_gear"].as_u64()?
                                    as usize
                            || h.armour
                                < ctx.profile.boss_parties.policy["vvulf_min_gear"].as_u64()?
                                    as usize)
                    {
                        continue;
                    }
                    let exact=h.class==member.class_id&&member.skills.iter().all(|s|h.skills.contains_key(s)&&(ctx.on("skills")||h.selected.contains(s))||ctx.on("skills")&&t.actions.iter().any(|c|matches!(&c.action,TownAction::Upgrade{hero:Some(id),tree,..}if *id==h.guid&&*tree==format!("{}.{}",h.class,s))));
                    // A class match lacking its named build is a functional replacement, not a verified match.
                    let (r, score) = values[&(h.guid, rank)];
                    if r[4] <= 0. {
                        continue;
                    }
                    let mut next = prev.clone();
                    next.ids.push(h.guid);
                    next.matched += usize::from(exact);
                    next.score += score;
                    next.heal |=
                        r[1] >= ctx.profile.parameters["town"]["medium_min_heal"].as_f64()?;
                    next.relief |=
                        r[2] >= ctx.profile.parameters["town"]["medium_min_relief"].as_f64()?;
                    next.controls = (next.controls + u8::from(r[3] > 0.)).min(2);
                    buckets
                        .entry((next.heal, next.relief, next.controls))
                        .or_default()
                        .push(next);
                }
            }
            beam = buckets
                .into_values()
                .flat_map(|mut v| {
                    v.sort_by(|a, b| {
                        b.matched
                            .cmp(&a.matched)
                            .then_with(|| b.score.total_cmp(&a.score))
                            .then_with(|| a.ids.cmp(&b.ids))
                    });
                    v.truncate(keep);
                    v
                })
                .collect();
        }
        for mut b in beam {
            if q.length > 1
                && (!b.heal
                    || !b.relief
                        && usize::from(b.controls)
                            < ctx.profile.parameters["town"]["medium_min_controls_without_relief"]
                                .as_u64()? as usize)
            {
                continue;
            }
            if family == "prophet" && q.difficulty >= 5 {
                b.score += usable_debuffs(p, &b.ids, t, ctx) as f64
                    * ctx.profile.boss_parties.policy["prophet_champion_debuff_bonus"].as_f64()?;
            }
            choices.push((b.matched, b.score, p.id.clone(), b.ids));
        }
    }
    choices.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.total_cmp(&a.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });
    choices
        .into_iter()
        .next()
        .map(|(_, _, id, party)| (id, party))
}
pub fn matching(
    profile: &Profile,
    id: &str,
    party: &[u64],
    t: &crate::town016::Observation,
) -> usize {
    profile
        .boss_parties
        .plans
        .iter()
        .find(|p| p.id == id)
        .map_or(0, |p| {
            party
                .iter()
                .enumerate()
                .filter(|(i, id)| {
                    p.slots.iter().any(|m| {
                        m.rank == *i as u8 + 1
                            && t.heroes
                                .iter()
                                .any(|h| h.guid == **id && h.class == m.class_id)
                    })
                })
                .count()
        })
}

pub fn usable_debuffs(
    p: &boss::Plan,
    ids: &[u64],
    t: &crate::town016::Observation,
    ctx: &Context<'_>,
) -> usize {
    p.slots.iter().map(|slot| {
  let Some(h)=ids.get(slot.rank.saturating_sub(1) as usize).and_then(|id|t.heroes.iter().find(|h|h.guid==*id&&h.class==slot.class_id)) else{return 0;};
  slot.skills.iter().filter(|id|["intimidate","weakening_curse"].contains(&id.as_str())&&ctx.sources.cards.skills.get(&h.class).and_then(|m|m.get(*id)).is_some_and(|s|s.launch.contains(&slot.rank))&&(h.skills.contains_key(*id)&&(ctx.on("skills")||h.selected.contains(*id))||ctx.on("skills")&&t.actions.iter().any(|c|matches!(&c.action,TownAction::Upgrade{hero:Some(g),tree,..}if *g==h.guid&&*tree==format!("{}.{}",h.class,id))))).count()
 }).sum()
}
