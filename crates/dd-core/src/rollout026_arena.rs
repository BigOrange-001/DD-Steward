//! Paired, source-kernel evaluation of frozen public scenes. No campaign hidden state.
use super::*;
use crate::strategy025::{self as s25, Context};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial {
    pub index: usize,
    pub result: Option<SampleResult>,
    pub error: Option<String>,
    pub fallback_reasons: BTreeMap<String, u64>,
    pub rollout_decisions: u64,
    pub guard_triggers: u64,
}
pub struct Engine {
    game: GameData,
    profile: s25::Profile,
    sources: s25::Sources,
    rules21: crate::tactics021::Rules,
    enabled: BTreeSet<String>,
    ordered: bool,
    settings: Settings,
}
fn dummy(o: &Observation) -> crate::opening015::Observation {
    crate::opening015::Observation {
        direct_rewards: vec![],
        item_states: vec![],
        profile: "public026-arena".into(),
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
    }
}
impl Engine {
    pub fn load() -> Result<Self> {
        let game = GameData::load_with_content(&dd_data::game_dir(), full_content()?)?;
        let profile = s25::Profile::documented()?;
        let (enabled, ordered) = profile.enabled("all")?;
        let sources = s25::Sources::project(&game)?;
        let rules21 = crate::tactics021::Rules::documented(&game)?;
        Ok(Self {
            game,
            profile,
            sources,
            rules21,
            enabled,
            ordered,
            settings: Settings::budgeted()?,
        })
    }
    fn refresh(
        &self,
        o: &Observation,
        input: &PublicBattle,
        sources: &mut s25::Sources,
    ) -> Result<()> {
        for u in o.units.iter().filter(|u| !u.hero && !u.corpse && u.hp > 0) {
            let key = s25::threat::key(u);
            if sources.enemies.contains_key(&key) {
                continue;
            }
            let e = if u.controlled.is_some() {
                let mut e = s25::threat::Enemy::default();
                let mut found = false;
                for h in input.heroes.values().filter(|h| h.class == u.class) {
                    let bp = self.game.hero_ranked(
                        &h.class,
                        h.weapon,
                        h.armour,
                        &h.skills
                            .iter()
                            .map(|(s, n)| (s.as_str(), *n))
                            .collect::<Vec<_>>(),
                    )?;
                    let other = s25::threat::controlled_source026(&bp);
                    e.skills.extend(other.skills);
                    e.turns = e.turns.max(other.turns);
                    e.stun_resist = e.stun_resist.max(other.stun_resist);
                    e.move_resist = e.move_resist.max(other.move_resist);
                    found = true;
                }
                if !found {
                    return Err("controlled hero missing public training".into());
                }
                e
            } else {
                s25::threat::project(&self.game, &u.class)?
            };
            Arc::make_mut(&mut sources.enemies).insert(key, e);
        }
        Ok(())
    }
    fn static_action(
        &self,
        o: &Observation,
        m: &mut s25::Memory,
        sources: &s25::Sources,
        m21: &mut crate::tactics021::Memory,
    ) -> Result<Action> {
        let ctx = Context {
            profile: &self.profile,
            sources,
            enabled: &self.enabled,
            ordered: self.ordered,
        };
        if let Some((a, _)) = s25::combat::choose(o, m, &ctx)? {
            return Ok(a);
        }
        let result = self.rules21.evaluate(o, m21)?;
        *m21 = result.memory;
        if let Some(c) = result.choice {
            return Ok(c.action);
        }
        let crate::opening015::Action::Fight(a) = crate::auto_policy018::raid_action(&dummy(o))?
        else {
            return Err("legacy battle fallback did not return Fight".into());
        };
        Ok(a)
    }
    fn trial(
        &self,
        input: &PublicBattle,
        defs: &Definitions,
        index: usize,
        ordinal: usize,
        rollout: bool,
    ) -> Trial {
        self.trial_traced(input, defs, index, ordinal, rollout, None)
    }
    fn trial_traced(
        &self,
        input: &PublicBattle,
        defs: &Definitions,
        index: usize,
        ordinal: usize,
        rollout: bool,
        mut trace: Option<&mut Vec<Value>>,
    ) -> Trial {
        let mut fallbacks = BTreeMap::new();
        let mut decisions = 0;
        let mut guards = 0;
        let result = execution025::hypothetical026(input.mechanisms, || {
            execution024b::hypothetical026(input.single_target_fallback, || {
                let mut evaluation = self.settings.clone();
                evaluation.salt.push_str(":independent-arena-evaluation-v1");
                let mut model =
                    reconstruct(input, defs, 0)?.sample(sample_seed(input, &evaluation, {
                        let n = ordinal + index;
                        n * (n + 1) / 2 + index
                    })?)?;
                model.battle.scenario.round_limit = 1000;
                let mut memory = s25::Memory::default();
                let mut m21 = crate::tactics021::Memory::default();
                let mut sources = self.sources.clone();
                let original = input
                    .observation
                    .units
                    .iter()
                    .filter(|u| u.hero && !u.corpse)
                    .map(|u| u.id)
                    .collect::<BTreeSet<_>>();
                let mut doors = 0;
                let mut afflictions = BTreeSet::new();
                let mut guard = crate::campaign018::runner::mechanisms023::Guard::default();
                let mut steps = 0;
                while model.battle.outcome().is_none() {
                    if steps >= 1000 * self.settings.transitions_per_round {
                        return Err("arena_transition_limit".into());
                    }
                    let o = model.observation();
                    let a = if o.actor.is_some_and(|id| model.battle.units[id].friendly())
                        || o.player_selection
                    {
                        self.refresh(&o, input, &mut sources)?;
                        let guard_action = guard.action(
                            &crate::campaign018::runner::View::Opening(Box::new(dummy(&o))),
                        );
                        guards = guard.triggers;
                        let guard_action = guard_action?;
                        let action = if let Some(crate::auto_policy018::Action::Raid(
                            crate::opening015::Action::Fight(action),
                        )) = guard_action
                        {
                            action
                        } else if rollout {
                            let ctx = Context {
                                profile: &self.profile,
                                sources: &sources,
                                enabled: &self.enabled,
                                ordered: self.ordered,
                            };
                            let ranked = s25::combat::ranked026(&o, &memory, &ctx)?;
                            let mut current = input.clone();
                            current.observation = o.clone();
                            current.retreat_legal =
                                Some(o.actions.iter().any(|a| a.action == Action::Retreat));
                            let chosen = Definitions::project(&self.game, &current)
                                .and_then(|defs| choose(&current, &defs, &self.settings, &ranked));
                            match chosen {
                                Ok(d) => {
                                    decisions += 1;
                                    d.action
                                }
                                Err(e) => {
                                    *fallbacks.entry(e).or_default() += 1;
                                    self.static_action(&o, &mut memory, &sources, &mut m21)?
                                }
                            }
                        } else {
                            self.static_action(&o, &mut memory, &sources, &mut m21)?
                        };
                        s25::combat::finish(&o, &action, &mut memory, &sources);
                        action
                    } else {
                        model.battle.monster_action()?
                    };
                    let trace_before=trace.as_ref().map(|_| {
                        let actor=model.battle.actor;
                        let legal=model.battle.legal_actions().into_iter().find(|x|x.action==a);
                        let i=match a {Action::Skill{skill,..}|Action::SkillAuto{skill}=>Some(skill),_=>None};
                        let skill=actor.zip(i).and_then(|(id,i)|model.battle.units[id].blueprint.skills.get(i));
                        let values=actor.zip(skill).map(|(id,s)|legal.as_ref().into_iter().flat_map(|l|&l.targets).map(|target|serde_json::json!({"target":target,"values_hit_damage_crit":model.battle.attack_values(id,*target,s,&[]),"target_stats":model.battle.units[*target].stats()})).collect::<Vec<_>>());
                        serde_json::json!({"round":o.round,"actor":actor,"action":a,"legal_preview":legal,"skill_source":skill,"actor_stats":actor.map(|id|model.battle.units[id].stats()),"attack_values":values,"before":o.units,"torch":o.torch})
                    });
                    let events = model.step(a)?;
                    if let Some(rows) = trace.as_deref_mut() {
                        let mut row = trace_before.unwrap();
                        row["events"] = serde_json::to_value(&events).map_err(|e| e.to_string())?;
                        row["after"] = serde_json::to_value(model.observation().units)
                            .map_err(|e| e.to_string())?;
                        rows.push(row);
                    }
                    for event in events {
                        match event {
                            Event::DeathsDoor(id) if original.contains(&id) => doors += 1,
                            Event::Resolve {
                                target,
                                state: Resolve::Afflicted(_),
                            } if original.contains(&target) => {
                                afflictions.insert(target);
                            }
                            _ => {}
                        }
                    }
                    steps += 1;
                }
                Ok(super::decision::summarize(
                    &model,
                    input,
                    &self.settings,
                    doors,
                    &afflictions,
                ))
            })
        });
        match result {
            Ok(result) => Trial {
                index,
                result: Some(result),
                error: None,
                fallback_reasons: fallbacks,
                rollout_decisions: decisions,
                guard_triggers: guards,
            },
            Err(error) => Trial {
                index,
                result: None,
                error: Some(error),
                fallback_reasons: fallbacks,
                rollout_decisions: decisions,
                guard_triggers: guards,
            },
        }
    }
    pub fn trace_case(&self, scene: &Value, index: usize, out: &Path) -> Result<()> {
        let record: Value = serde_json::from_slice(
            &std::fs::read(scene["file"].as_str().ok_or("scene path absent")?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let input: PublicBattle =
            serde_json::from_value(record["input"].clone()).map_err(|e| e.to_string())?;
        let defs = Definitions::project(&self.game, &input)?;
        let mut steps = vec![];
        let trial = self.trial_traced(
            &input,
            &defs,
            index,
            scene["ordinal"].as_u64().ok_or("ordinal absent")? as usize,
            true,
            Some(&mut steps),
        );
        std::fs::write(out,serde_json::to_vec_pretty(&serde_json::json!({"source":scene,"input":input,"trial":trial,"steps":steps,"note":"Fresh public-observation surrogate trace; no real hidden state. Same fixed paired arena trial namespace."})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
    }
    fn probe(&self, input: &PublicBattle) -> Result<Value> {
        let mut sources = self.sources.clone();
        let mut memory = s25::Memory::default();
        let mut m21 = crate::tactics021::Memory::default();
        self.refresh(&input.observation, input, &mut sources)?;
        let ctx = Context {
            profile: &self.profile,
            sources: &sources,
            enabled: &self.enabled,
            ordered: self.ordered,
        };
        let ranked = s25::combat::ranked026(&input.observation, &memory, &ctx)?;
        let d = choose(
            input,
            &Definitions::project(&self.game, input)?,
            &self.settings,
            &ranked,
        )?;
        let old = self.static_action(&input.observation, &mut memory, &sources, &mut m21)?;
        Ok(serde_json::json!({"rules_action":old,"rollout":d}))
    }
    pub fn questions(&self, manifest: &Path, out: &Path) -> Result<()> {
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let mut rows = vec![];
        for scene in manifest["rows"].as_array().ok_or("missing scene rows")? {
            let record: Value = serde_json::from_slice(
                &std::fs::read(scene["file"].as_str().ok_or("scene file absent")?)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let input: PublicBattle =
                serde_json::from_value(record["input"].clone()).map_err(|e| e.to_string())?;
            let probes = execution025::hypothetical026(input.mechanisms, || {
                execution024b::hypothetical026(input.single_target_fallback, || {
                    let first = self
                        .probe(&input)
                        .unwrap_or_else(|e| serde_json::json!({"error":e}));
                    let mut derived = None;
                    if scene["category"] == "ordinary" {
                        let defs = Definitions::project(&self.game, &input)?;
                        let mut model =
                            reconstruct(&input, &defs, sample_seed(&input, &self.settings, 0)?)?;
                        let mut sources = self.sources.clone();
                        let mut m = s25::Memory::default();
                        let mut m21 = crate::tactics021::Memory::default();
                        for _ in 0..512 {
                            let o = model.observation();
                            if o.outcome.is_some() {
                                break;
                            }
                            let player =
                                o.actor.is_some_and(|id| model.battle.units[id].friendly());
                            if player
                                && o.units
                                    .iter()
                                    .filter(|u| !u.hero && !u.corpse && u.hp > 0)
                                    .count()
                                    == 1
                            {
                                let mut low = input.clone();
                                low.observation = o;
                                low.retreat_legal = Some(
                                    low.observation
                                        .actions
                                        .iter()
                                        .any(|a| a.action == Action::Retreat),
                                );
                                derived = Some(
                                    self.probe(&low)
                                        .unwrap_or_else(|e| serde_json::json!({"error":e})),
                                );
                                break;
                            }
                            let a = if player {
                                self.refresh(&o, &input, &mut sources)?;
                                let a = self.static_action(&o, &mut m, &sources, &mut m21)?;
                                s25::combat::finish(&o, &a, &mut m, &sources);
                                a
                            } else {
                                model.battle.monster_action()?
                            };
                            model.step(a)?;
                        }
                    }
                    Ok(serde_json::json!({"opening":first,"derived_one_enemy":derived}))
                })
            })
            .unwrap_or_else(|e| serde_json::json!({"error":e}));
            rows.push(
                serde_json::json!({"id":scene["id"],"category":scene["category"],"probes":probes}),
            );
        }
        std::fs::write(out,serde_json::to_vec_pretty(&serde_json::json!({"rows":rows,"note":"First-decision candidate estimates only; one-enemy states are source-surrogate derivatives, not extra real openings or causal 200-trial endpoints."})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
    }
    fn case(&self, scene: &Value, out: &Path, trials: usize, rollout: bool) -> Result<Value> {
        let id = scene["id"].as_str().ok_or("arena scene lacks id")?;
        if scene["status"] != "PreparedPartialHistorical" {
            return Err(scene["reason"]
                .as_str()
                .unwrap_or("scene not reconstructed")
                .to_string());
        }
        let mode = if rollout { "rollout" } else { "rules" };
        let path = out.join(format!("{id}-{mode}.json"));
        let record: Value = serde_json::from_slice(
            &std::fs::read(scene["file"].as_str().ok_or("arena scene path absent")?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let input: PublicBattle =
            serde_json::from_value(record["input"].clone()).map_err(|e| e.to_string())?;
        let mut rows = vec![];
        if path.exists() {
            let saved: Value =
                serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if saved["implementation"] != "026-arena-m2-budget-v1"
                || saved["settings"]
                    != serde_json::to_value(&self.settings).map_err(|e| e.to_string())?
                || saved["input"] != record["input"]
            {
                return Err("arena resume settings/input mismatch".into());
            }
            rows = serde_json::from_value(saved["trials"].clone()).map_err(|e| e.to_string())?;
        }
        let defs = Definitions::project(&self.game, &input)?;
        reconstruct(&input, &defs, 0)?;
        fn save(path: &Path, value: &Value) -> Result<()> {
            let temp = path.with_extension("tmp");
            std::fs::write(&temp, serde_json::to_vec(value).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            std::fs::rename(temp, path).map_err(|e| e.to_string())?;
            Ok(())
        }
        while rows.len() < trials {
            let first = rows.len();
            let end = (first + 10).min(trials);
            let ordinal = scene["ordinal"].as_u64().ok_or("arena ordinal absent")? as usize;
            let cursor = AtomicUsize::new(first);
            let batch = Mutex::new(Vec::new());
            std::thread::scope(|scope| {
                for _ in 0..4.min(end - first) {
                    scope.spawn(|| {
                        loop {
                            let index = cursor.fetch_add(1, Ordering::Relaxed);
                            if index >= end {
                                break;
                            }
                            let row = self.trial(&input, &defs, index, ordinal, rollout);
                            batch.lock().unwrap().push(row);
                        }
                    });
                }
            });
            let mut batch = batch.into_inner().map_err(|e| e.to_string())?;
            batch.sort_by_key(|r| r.index);
            rows.extend(batch);
            save(
                &path,
                &serde_json::json!({"implementation":"026-arena-m2-budget-v1","id":id,"mode":mode,"settings":self.settings,"input":input,"trials":rows,"source":scene}),
            )?;
        }
        let result = serde_json::json!({"implementation":"026-arena-m2-budget-v1","id":id,"mode":mode,"settings":self.settings,"input":input,"trials":rows,"source":scene});
        save(&path, &result)?;
        Ok(
            serde_json::json!({"id":id,"mode":mode,"file":path,"trials":rows.len(),"errors":rows.iter().filter(|r|r.error.is_some()).count()}),
        )
    }
}
pub fn run(manifest: &Path, out: &Path, trials: usize, quick: bool, jobs: usize) -> Result<Value> {
    if trials == 0 || jobs == 0 {
        return Err("arena trials/jobs must be positive".into());
    }
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let scenes = manifest["rows"]
        .as_array()
        .ok_or("arena manifest rows absent")?
        .iter()
        .filter(|r| !quick || r["quick"] == true)
        .cloned()
        .collect::<Vec<_>>();
    let tasks = scenes
        .into_iter()
        .flat_map(|s| [(s.clone(), false), (s, true)])
        .collect::<Vec<_>>();
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    let engine = Engine::load()?;
    let index = AtomicUsize::new(0);
    let results = Mutex::new(vec![]);
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let i = index.fetch_add(1, Ordering::Relaxed);
                    let Some((scene, rollout)) = tasks.get(i) else {
                        break;
                    };
                    let result = engine.case(scene, out, trials, *rollout).unwrap_or_else(
                        |e| serde_json::json!({"id":scene["id"],"mode":if *rollout{"rollout"}else{"rules"},"error":e}),
                    );
                    println!(
                        "finished {} {}",
                        scene["id"],
                        if *rollout { "rollout" } else { "rules" }
                    );
                    results.lock().unwrap().push(result);
                }
            });
        }
    });
    let mut results = results.into_inner().map_err(|e| e.to_string())?;
    results.sort_by_key(|r| format!("{}:{}", r["id"], r["mode"]));
    let result = serde_json::json!({"results":results,"seconds":start.elapsed().as_secs_f64(),"trials":trials,"quick":quick,"jobs":jobs,"settings":engine.settings,"evaluation_salt_suffix":":independent-arena-evaluation-v1","no_holdout":true});
    std::fs::write(
        out.join("summary.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(result)
}
