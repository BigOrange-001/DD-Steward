//! Advisers accept public values only. A worker cannot access the human world.
use super::*;
use std::{sync::Arc, thread::JoinHandle};
#[derive(Clone, Serialize, Deserialize)]
pub struct Job {
    pub input: mc::PublicBattle,
    pub ranked: Vec<(f64, crate::LegalAction, String)>,
    pub wrapper: String,
}
pub struct Cache {
    pub rule: Value,
    pub job: Option<Job>,
    pub model_error: Option<String>,
    pub defs: Option<mc::Definitions>,
    pub memory: s25::Memory,
    pub memory21: crate::tactics021::Memory,
    pub mc: Option<Value>,
    pub worker: Option<JoinHandle<Result<Value>>>,
    pub seen_rule: bool,
    pub seen_mc: bool,
}
fn combat(v: &View) -> Option<&crate::Observation> {
    match v {
        View::Opening(o) => o.battle.as_ref(),
        View::Campaign(o) => o
            .expedition
            .as_ref()
            .and_then(|r| r.battle.as_ref())
            .or_else(|| {
                o.content019
                    .as_ref()
                    .and_then(|c| c.farm.as_ref())
                    .map(|f| &f.battle)
            }),
    }
}
pub fn wrap(wrapper: &str, a: &crate::Action) -> Value {
    if wrapper == "Arena" {
        json!(a)
    } else {
        json!({wrapper:{"Fight":a}})
    }
}
fn battle_rule(
    o: &crate::Observation,
    m: &mut s25::Memory,
    m21: &mut crate::tactics021::Memory,
    ctx: &s25::Context<'_>,
    rules: &crate::tactics021::Rules,
    opening: bool,
) -> Result<(crate::Action, String, String)> {
    if !opening && let Some((a, id)) = s25::combat::choose(o, m, ctx)? {
        let why = ctx.profile.rules.get(&id).map_or(id.clone(), |r| {
            format!("{}；{}", r.sources.join("、"), r.note)
        });
        return Ok((a, id, why));
    }
    let evaluation = rules.evaluate(o, m21)?;
    *m21 = evaluation.memory;
    if let Some(c) = evaluation.choice {
        let why = rules
            .document()
            .rules
            .iter()
            .find(|r| r.id == c.rule_id)
            .map_or(c.rule_id.clone(), |r| r.rationale.clone());
        return Ok((c.action, c.rule_id, why));
    }
    // Same frozen public fallback as the existing runner.
    let dummy = crate::opening015::Observation {
        direct_rewards: vec![],
        item_states: vec![],
        profile: "human030-advice".into(),
        content_id: o.content_id.clone(),
        phase: crate::opening015::Phase::Expedition,
        heroes: vec![],
        party: vec![],
        inventory: dd_data::inventory015::Bag { slots: vec![] },
        wallet: BTreeMap::new(),
        map: None,
        battle: Some(o.clone()),
        loot: vec![],
        torch: o.torch.unwrap_or(0),
        quest_ready: false,
        actions: o
            .actions
            .iter()
            .map(|a| crate::opening015::Action::Fight(a.action.clone()))
            .collect(),
        results: vec![],
        stack_limits: BTreeMap::new(),
        camping: None,
    };
    let crate::opening015::Action::Fight(a) = crate::auto_policy018::raid_action(&dummy)? else {
        return Err("030 baseline did not select fight".into());
    };
    Ok((a, "S025-00/018".into(), "原有公开战斗回落".into()))
}
pub fn prepare(h: &mut Human) -> Result<Cache> {
    let game = h.advice_game.clone();
    let profile = rollout027::profile(false)?;
    let mut profile = (*profile).clone();
    profile.parameters["coherence029"] = json!(true);
    let (enabled, ordered) = profile.enabled("all")?;
    let rules = crate::tactics021::Rules::documented(&game)?;
    let mut memory = h.memory.clone();
    let mut memory21 = h.memory21.clone();
    let mut input = None;
    let mut model_error = None;
    let mut wrapper = "Arena".to_string();
    let (view, mut sources) = match &mut h.world {
        World::Campaign(r) => {
            let v = r.observation()?;
            let mut sources = r.inner.sources_visible026(&v)?;
            if let View::Campaign(o) = &v {
                sources.training_heirlooms = memory.ledger.heirlooms.clone();
                if o.expedition.is_none() && o.provisions.is_none() {
                    let Actor::Campaign(host) = &r.inner.inner.replay.inner.inner.inner.actor
                    else {
                        return Err("030 town quote phase".into());
                    };
                    sources.training = Arc::new(host.training_quotes025()?);
                    sources.recovery_ranges = host.recovery_ranges025(&o.town)?;
                }
                let quote = if o.provisions.is_none() && !o.town.party.is_empty() {
                    memory
                        .legacy
                        .quest
                        .as_ref()
                        .and_then(|id| {
                            o.board
                                .as_ref()
                                .and_then(|b| b.quests.iter().find(|q| &q.id == id))
                        })
                        .map(|q| {
                            let Actor::Campaign(host) =
                                &r.inner.inner.replay.inner.inner.inner.actor
                            else {
                                return Err("030 supply quote phase".into());
                            };
                            host.preparation_quote024b(&q.id)
                                .map(|p| crate::preparation024::economy::Quote::from_visible(&p))
                        })
                        .transpose()?
                } else {
                    None
                };
                memory
                    .ledger
                    .refresh_planned(o, &profile, quote.as_ref(), true);
                sources.current_boss = memory
                    .state
                    .get("boss_plan")
                    .and_then(Value::as_str)
                    .map(|id| (id.into(), memory.legacy.target_party.clone()));
            }
            if let Some(b) = combat(&v) {
                wrapper = match &v {
                    View::Opening(_) => "Opening",
                    View::Campaign(o)
                        if o.content019.as_ref().is_some_and(|c| c.farm.is_some()) =>
                    {
                        "Farm"
                    }
                    _ => "Raid",
                }
                .into();
                let frame = r.frame()?;
                match r.public_input(b, &frame, &v) {
                    Ok(mut i) => {
                        i.history = h.history.clone();
                        input = Some(i);
                    }
                    Err(e) => model_error = Some(e),
                }
            }
            (Some(v), sources)
        }
        World::Arena(a) => {
            let mut i = a.input.clone();
            i.observation = a.observation();
            i.history = h.history.clone();
            i.retreat_legal = Some(
                i.observation
                    .actions
                    .iter()
                    .any(|a| a.action == crate::Action::Retreat),
            );
            input = Some(i);
            (None, s25::Sources::project(&game)?)
        }
    };
    if let Some(i) = &input {
        for u in i
            .observation
            .units
            .iter()
            .filter(|u| !u.hero && !u.corpse && u.hp > 0)
        {
            let key = s25::threat::key(u);
            if !sources.enemies.contains_key(&key) {
                let e = if u.controlled.is_some() {
                    let hero = i
                        .heroes
                        .values()
                        .find(|h| h.class == u.class)
                        .ok_or("030 controlled public training absent")?;
                    let bp = game.hero_ranked(
                        &hero.class,
                        hero.weapon,
                        hero.armour,
                        &hero
                            .skills
                            .iter()
                            .map(|(s, n)| (s.as_str(), *n))
                            .collect::<Vec<_>>(),
                    )?;
                    s25::threat::controlled_source026(&bp)
                } else {
                    s25::threat::project(&game, &u.class)?
                };
                Arc::make_mut(&mut sources.enemies).insert(key, e);
            }
        }
    }
    let ctx = s25::Context {
        profile: &profile,
        sources: &sources,
        enabled: &enabled,
        ordered,
    };
    let mut ranked = vec![];
    let (action, id, reason) = if let Some(o) = input
        .as_ref()
        .map(|i| &i.observation)
        .or_else(|| view.as_ref().and_then(combat))
    {
        ranked = s25::combat::ranked026(o, &memory, &ctx)?;
        let mut picked = battle_rule(
            o,
            &mut memory,
            &mut memory21,
            &ctx,
            &rules,
            wrapper == "Opening",
        )?;
        if picked.0 == crate::Action::Retreat && wrapper != "Opening" {
            let allowed = input
                .as_ref()
                .map(|i| {
                    let defs = mc::Definitions::project029(&game, i, true)?;
                    let m = mc::reconstruct(i, &defs, 0)?;
                    let mut s = mc::Settings::budgeted()?;
                    s.coherence029 = true;
                    mc::retreat::assess(&m, i, &s).map(|a| a.allowed)
                })
                .transpose()
                .unwrap_or(None)
                .unwrap_or(false);
            if !allowed {
                let mut safe = o.clone();
                safe.actions.retain(|a| a.action != crate::Action::Retreat);
                picked = battle_rule(&safe, &mut memory, &mut memory21, &ctx, &rules, false)?;
            }
        }
        (wrap(&wrapper, &picked.0), picked.1, picked.2)
    } else {
        match view.as_ref().ok_or("030 missing public view")? {
            View::Opening(o) => (
                json!({"Opening":crate::opening_policy015::choose(o)?}),
                "015".into(),
                "原有老路公开流程".into(),
            ),
            View::Campaign(o) => {
                let p = s25::policy::choose(o, &mut memory, &ctx)?;
                if let Some(p) = p {
                    let why = profile.rules.get(&p.rule).map_or(p.stage.clone(), |r| {
                        format!("{}；{}", r.sources.join("、"), r.note)
                    });
                    (json!(p.action), p.rule, why)
                } else {
                    let d = crate::auto_policy018::choose(o, &mut memory.legacy)?;
                    (json!(d.action), "S025-00/018".into(), d.reason)
                }
            }
        }
    };
    let defs = match input
        .as_ref()
        .map(|i| mc::Definitions::project029(&game, i, true))
        .transpose()
    {
        Ok(d) => d,
        Err(e) => {
            model_error = Some(e);
            None
        }
    };
    let delegated = matches!(view, Some(View::Opening(_)))
        || (input.is_none() && view.as_ref().and_then(combat).is_none());
    let delegated_mc=delegated.then(||json!({"action":action,"delegated":"existing runner delegates this phase to rules","top3":[]}));
    let failed_mc = if !delegated && defs.is_none() {
        input.as_ref().map(|i| {
            fallback(
                &Job {
                    input: i.clone(),
                    ranked: ranked.clone(),
                    wrapper: wrapper.clone(),
                },
                model_error
                    .clone()
                    .unwrap_or_else(|| "model unavailable".into()),
            )
        }).or_else(||view.as_ref().and_then(combat).map(|o|{
            let a=ranked.iter().map(|(_,a,_)|a).chain(o.actions.iter()).find(|a|a.action!=crate::Action::Retreat);
            json!({"action":a.map(|a|wrap(&wrapper,&a.action)),"fallback":model_error,"top3":[],"scoring_unavailable":true})
        }))
    } else {
        None
    };
    Ok(Cache {
        rule: json!({"action":action,"rule_id":id,"reason":reason,"memory_approximation":"public human trajectory; counterfactual intentions are approximate"}),
        job: input.map(|input| Job {
            input,
            ranked,
            wrapper,
        }),
        model_error,
        defs,
        memory,
        memory21,
        mc: delegated_mc.or(failed_mc),
        worker: None,
        seen_rule: false,
        seen_mc: false,
    })
}
pub fn run(game: &GameData, job: &Job) -> Result<Value> {
    Ok(mc::Definitions::project029(game, &job.input, true)
        .and_then(|defs| run_defs(job, &defs))
        .unwrap_or_else(|e| fallback(job, e)))
}
/// Offline diagnostic over a public log. Never accepts a native world or save.
pub fn compare(game: &GameData, job: &Job, human: &Value, cached: Option<&Value>) -> Result<Value> {
    let advice = if let Some(v) = cached {
        v.clone()
    } else {
        run(game, job)?
    };
    let raw = if job.wrapper == "Arena" {
        human.clone()
    } else {
        human[&job.wrapper]["Fight"].clone()
    };
    let action: crate::Action = serde_json::from_value(raw).map_err(|e| e.to_string())?;
    let Some(decision) = advice.get("decision") else {
        return Ok(
            json!({"rollout":advice,"human_score":null,"ai_score":null,"human_in_candidates":null,"score_reason":"model fallback; candidate search and comparable scores unavailable"}),
        );
    };
    let d: mc::Decision = serde_json::from_value(decision.clone()).map_err(|e| e.to_string())?;
    let in_candidates = d.candidates.iter().any(|c| c.action == action)
        || (d.candidates.is_empty() && d.action == action);
    let score = |a: &crate::Action| {
        let has_final = d
            .candidates
            .iter()
            .any(|c| &c.action == a && !c.categories.iter().any(|s| s == "eliminated_pilot"));
        d.candidates.iter().filter(|c|&c.action==a).filter(|c|!has_final||!c.categories.iter().any(|s|s=="eliminated_pilot")).max_by(|a,b|a.value.total_cmp(&b.value)).map(|c|json!({"mean":c.mean,"worst_tail":c.worst_tail,"value":c.value,"samples":c.samples,"categories":c.categories,"source":"cached actual search score"}))
    };
    let mut settings = mc::Settings::budgeted()?;
    settings.coherence029 = true;
    let mut human_score = score(&action);
    let mut ai_score = score(&d.action);
    if human_score.is_none() || ai_score.is_none() {
        let diagnostic = (|| -> Result<_> {
            let defs = mc::Definitions::project029(game, &job.input, true)?;
            if human_score.is_none() {
                human_score = Some(
                    mc::diagnostic_score030(&job.input, &defs, &settings, &action)?["best"].clone(),
                );
            }
            if ai_score.is_none() {
                ai_score = if d.action == action {
                    human_score.clone()
                } else {
                    Some(
                        mc::diagnostic_score030(&job.input, &defs, &settings, &d.action)?["best"]
                            .clone(),
                    )
                };
            }
            Ok(())
        })();
        if let Err(e) = diagnostic {
            return Ok(
                json!({"rollout":advice,"human_score":human_score,"ai_score":ai_score,"human_in_candidates":in_candidates,"score_error":e}),
            );
        }
    }
    Ok(
        json!({"rollout":advice,"human_score":human_score,"ai_score":ai_score,"human_in_candidates":in_candidates,"cached_shown_advice":cached.is_some()}),
    )
}
fn fallback(job: &Job, error: String) -> Value {
    let a = job
        .ranked
        .iter()
        .map(|(_, a, _)| a)
        .chain(job.input.observation.actions.iter())
        .find(|a| a.action != crate::Action::Retreat);
    json!({"action":a.map(|a|wrap(&job.wrapper,&a.action)),"fallback":error,"top3":[],"scoring_unavailable":true})
}
fn run_defs(job: &Job, defs: &mc::Definitions) -> Result<Value> {
    let mut s = mc::Settings::budgeted()?;
    s.coherence029 = true;
    let decision = mc::choose(&job.input, &defs, &s, &job.ranked)?;
    let mut rows = decision.candidates.clone();
    rows.sort_by(|a, b| {
        a.categories
            .iter()
            .any(|s| s == "eliminated_pilot")
            .cmp(&b.categories.iter().any(|s| s == "eliminated_pilot"))
            .then(b.value.total_cmp(&a.value))
    });
    let mut distinct = vec![];
    let mut top = vec![];
    for row in rows {
        if distinct.contains(&row.action) {
            continue;
        }
        distinct.push(row.action.clone());
        top.push(json!({"action":wrap(&job.wrapper,&row.action),"mean":row.mean,"worst_tail":row.worst_tail,"value":row.value,"samples":row.samples,"categories":row.categories}));
        if top.len() == 3 {
            break;
        }
    }
    Ok(json!({"action":wrap(&job.wrapper,&decision.action),"decision":decision,"top3":top}))
}
impl Cache {
    pub fn poll(&mut self, enabled: bool) {
        if enabled
            && self.mc.is_none()
            && self.worker.is_none()
            && let Some(job) = self.job.clone()
            && let Some(defs) = self.defs.clone()
        {
            self.worker = Some(std::thread::spawn(move || {
                Ok(run_defs(&job, &defs).unwrap_or_else(|e| fallback(&job, e)))
            }));
        }
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            let result = self
                .worker
                .take()
                .unwrap()
                .join()
                .unwrap_or_else(|_| Err("030 advice worker panicked".into()));
            self.mc = Some(result.unwrap_or_else(|e| json!({"error":e})));
        }
    }
    pub fn visible(&self, enabled: bool) -> Value {
        if !enabled {
            return json!({"enabled":false});
        }
        json!({"enabled":true,"rules":self.rule,"rollout":self.mc,"pending":self.worker.is_some(),"model_error":self.model_error,"seen_rules":self.seen_rule,"seen_rollout":self.seen_mc})
    }
}
