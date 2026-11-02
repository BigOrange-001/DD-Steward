//! Optional observer and delta codec. No decision or simulator mutation lives here.
use super::*;
use arena021::Runtime021;

pub const SCHEMA: &str = "dd-replay-022-v2";

fn value<T: Serialize>(v: T) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// This projection accepts only filtered observations and the narrow identity bridge.
fn visible_frame(view: &View, deployment: &[u64]) -> Result<Value> {
    let (raid, farm, context, inventory, loot, roster, mut build, quest) = match view {
        View::Opening(o) => (
            Some(o.as_ref()), None,
            json!({"phase":"opening","stage":o.phase,"returns":o.results.len()}),
            value(&o.inventory)?, value(&o.loot)?, value(&o.heroes)?,
            json!(o.heroes.iter().filter(|h| o.party.contains(&h.guid)).map(|h|
                json!({"guid":h.guid,"selected":h.skills,"weapon":null,"armour":null,"equipped":null})).collect::<Vec<_>>()),
            Value::Null,
        ),
        View::Campaign(o) => {
            let farm = o.content019.as_ref().and_then(|c| c.farm.as_ref());
            let raid = o.expedition.as_ref();
            let quest = o.board.as_ref().and_then(|b| b.quests.iter().find(|q| Some(&q.id) == b.selected.as_ref()));
            let phase = if raid.is_some() {"expedition"} else if farm.is_some() {"farm"} else if o.provisions.is_some() {"provisions"} else {"town"};
            (
                raid, farm,
                json!({"phase":phase,"returns":o.returns,"quest":quest.map(|q| &q.id)}),
                if let Some(e) = raid {value(&e.inventory)?} else if let Some(f) = farm {value(&f.inventory)?} else if let Some(p) = &o.provisions {value(&p.inventory)?} else {Value::Null},
                if let Some(e) = raid {value(&e.loot)?} else if let Some(f) = farm {value(&f.loot)?} else {Value::Null},
                value(&o.town.heroes)?,
                json!(o.town.heroes.iter().filter(|h| o.town.party.contains(&h.guid)).map(|h|
                    json!({"guid":h.guid,"weapon":h.weapon,"armour":h.armour,"skills":h.skills,"selected":h.selected,"equipped":h.equipped})).collect::<Vec<_>>()),
                quest.map(|q| json!({"id":q.id,"plot":q.plot,"region":q.dungeon,"kind":q.kind,"difficulty":q.difficulty,"length":q.length,"goals":q.goals,"reward":q.reward,"extra_rewards":q.extra_rewards})).unwrap_or(Value::Null),
            )
        }
    };
    if let (Some(raid), Some(rows)) = (raid, build.as_array_mut()) {
        for row in rows {
            if let Some(equipment) = raid
                .heroes
                .iter()
                .find(|h| Some(h.guid) == row["guid"].as_u64())
                .and_then(|h| h.equipped023.as_ref())
            {
                row["equipped_current023"] = value(equipment)?;
            }
        }
    }
    let battle = raid
        .and_then(|e| e.battle.as_ref())
        .or_else(|| farm.map(|f| &f.battle));
    let mapping = battle
        .map(|b| {
            b.units
                .iter()
                .filter(|u| u.hero)
                .map(|u| {
                    let guid = deployment
                        .get(u.id)
                        .ok_or("visible hero lacks deployment identity")?;
                    Ok(json!({"unit_id":u.id,"guid":guid}))
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?;
    let progress = if let Some(e) = raid {
        json!({"ready":e.quest_ready,"map_completed":e.map.as_ref().map(|m|m.completed),
            "visited_known":e.map.as_ref().map(|m|m.nodes.iter().filter(|n|n.visited).count()),
            "visited_known_rooms":e.map.as_ref().map(|m|m.nodes.iter().filter(|n|n.room && n.visited).count()),
            "known_rooms":e.map.as_ref().map(|m|m.nodes.iter().filter(|n|n.room).count()),
            "visible_objectives":e.map.as_ref().map(|m|m.nodes.iter().filter(|n|n.content.as_ref().is_some_and(|c|c.objective)).collect::<Vec<_>>())})
    } else if let Some(f) = farm {
        json!({"farm":f.battle.farm})
    } else {
        Value::Null
    };
    let item_states = match view {
        View::Opening(o) => value(&o.item_states)?,
        View::Campaign(o) => {
            if let Some(e) = raid {
                value(&e.item_states)?
            } else if let Some(f) = farm {
                value(&f.item_states)?
            } else {
                value(&o.town.item_states)?
            }
        }
    };
    Ok(
        json!({"context":context,"inventory":inventory,"loot":loot,"item_states":item_states,
        "map":raid.and_then(|e|e.map.as_ref()),"quest":quest,"goal_progress":progress,
        "party_build":build,"roster":roster,"battle_heroes":mapping}),
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encoder {
    last: Option<Value>,
}
impl Encoder {
    pub fn encode(&mut self, frame: &Value) -> Result<Value> {
        let patch = match self.last.as_ref() {
            None => json!({"full":frame}),
            Some(last) => {
                let new_context = last["context"] != frame["context"];
                let mut set = serde_json::Map::new();
                let mut map = None;
                for (key, v) in frame.as_object().ok_or("invalid replay frame")? {
                    let reset = new_context && (key == "map" || key == "inventory") && !v.is_null();
                    if last.get(key) == Some(v) && !reset {
                        continue;
                    }
                    if key == "map" && !new_context && v.is_object() && last[key].is_object() {
                        let old = last[key]["nodes"]
                            .as_array()
                            .ok_or("invalid previous map")?;
                        let nodes = v["nodes"].as_array().ok_or("invalid map")?;
                        let upsert = nodes
                            .iter()
                            .filter(|n| !old.contains(n))
                            .collect::<Vec<_>>();
                        let remove = old
                            .iter()
                            .filter(|n| !nodes.iter().any(|v| v["id"] == n["id"]))
                            .map(|n| &n["id"])
                            .collect::<Vec<_>>();
                        let mut metadata = v.clone();
                        metadata.as_object_mut().unwrap().remove("nodes");
                        map = Some(
                            json!({"upsert":upsert,"remove":remove,"order":nodes.iter().map(|n| &n["id"]).collect::<Vec<_>>(),"meta":metadata}),
                        );
                    } else {
                        set.insert(key.clone(), v.clone());
                    }
                }
                let mut patch = json!({});
                if !set.is_empty() {
                    patch["set"] = Value::Object(set);
                }
                if let Some(m) = map {
                    patch["map_delta"] = m;
                }
                patch
            }
        };
        self.last = Some(frame.clone());
        Ok(patch)
    }
}

/// Reference decoder for viewer implementers and exact round-trip verification.
pub fn apply(state: &mut Value, patch: &Value) -> Result<()> {
    if let Some(full) = patch.get("full") {
        if !full.is_object() {
            return Err("invalid full replay frame".into());
        }
        *state = full.clone();
        return Ok(());
    }
    if !state.is_object() {
        return Err("replay delta before initial frame".into());
    }
    if let Some(set) = patch.get("set") {
        for (key, v) in set.as_object().ok_or("invalid replay set")? {
            state[key] = v.clone();
        }
    }
    if let Some(delta) = patch.get("map_delta") {
        let mut nodes: BTreeMap<String, Value> = state["map"]["nodes"]
            .as_array()
            .ok_or("map delta without map")?
            .iter()
            .map(|n| {
                Ok((
                    n["id"].as_str().ok_or("node id absent")?.to_owned(),
                    n.clone(),
                ))
            })
            .collect::<Result<_>>()?;
        for id in delta["remove"].as_array().ok_or("invalid map removals")? {
            nodes.remove(id.as_str().ok_or("invalid removed id")?);
        }
        for n in delta["upsert"].as_array().ok_or("invalid map upserts")? {
            nodes.insert(n["id"].as_str().ok_or("invalid node id")?.into(), n.clone());
        }
        let ordered = delta["order"]
            .as_array()
            .ok_or("map order absent")?
            .iter()
            .map(|id| {
                nodes
                    .remove(id.as_str().ok_or("invalid ordered id")?)
                    .ok_or("unknown ordered node".into())
            })
            .collect::<Result<Vec<_>>>()?;
        if !nodes.is_empty() {
            return Err("unordered map nodes".into());
        }
        state["map"] = delta["meta"].clone();
        state["map"]["nodes"] = value(ordered)?;
    }
    Ok(())
}

fn battle(view: &View) -> Option<&crate::Observation> {
    match view {
        View::Opening(o) => o.battle.as_ref(),
        View::Campaign(o) => o
            .expedition
            .as_ref()
            .and_then(|e| e.battle.as_ref())
            .or_else(|| {
                o.content019
                    .as_ref()
                    .and_then(|c| c.farm.as_ref())
                    .map(|f| &f.battle)
            }),
    }
}
fn encounter(view: &View) -> Option<&crate::Observation> {
    let b = battle(view)?;
    // A farm entry kernel exposes only heroes at the standing stone. It is not
    // an encounter. Interludes and terminal observations are also quiet edges.
    if b.outcome.is_some() || (b.round == 0 && b.farm.is_none()) {
        None
    } else {
        Some(b)
    }
}
fn combat_action(view: &View, action: &Value) -> Result<Value> {
    let Some(b) = battle(view) else {
        return Ok(Value::Null);
    };
    if action["Farm"] == json!("Exit") {
        return Ok(
            json!({"actor":b.actor,"skill_id":null,"selected_target":null,"targets":[],"action":"Retreat"}),
        );
    }
    let Some(fight) = action
        .get("Opening")
        .or_else(|| action.get("Raid"))
        .or_else(|| action.get("Farm"))
        .and_then(|v| v.get("Fight"))
    else {
        return Ok(Value::Null);
    };
    let a: crate::Action = serde_json::from_value(fight.clone()).map_err(|e| e.to_string())?;
    let legal = b
        .actions
        .iter()
        .find(|l| l.action == a)
        .ok_or("replay action not in visible legal actions")?;
    let skill = matches!(
        a,
        crate::Action::Skill { .. } | crate::Action::SkillAuto { .. }
    )
    .then_some(&legal.label);
    let selected = match a {
        crate::Action::Skill { target, .. }
        | crate::Action::Item { target, .. }
        | crate::Action::Sacrifice { target } => Some(target),
        _ => None,
    };
    Ok(
        json!({"actor":b.actor,"skill_id":skill,"selected_target":selected,"targets":legal.targets,"action":a}),
    )
}

#[derive(Clone)]
pub struct Replay {
    pub(crate) recording024c: bool,
    pub inner: Runtime021,
    encoder: Encoder,
    encounter: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save022 {
    schema_version: String,
    inner: String,
    encoder: Encoder,
    encounter: u64,
}
impl Replay {
    pub fn new(inner: Runtime021) -> Self {
        Self {
            recording024c: true,
            inner,
            encoder: Encoder::default(),
            encounter: 0,
        }
    }
    pub fn frame(&mut self) -> Result<Value> {
        let _p = dd_data::telemetry024c::scope("replay_frame");
        let runtime = &mut self.inner.inner.inner;
        let view = runtime.observe()?;
        let ids = if battle(&view).is_some() {
            match &runtime.actor {
                Actor::Opening(s) => s.replay_hero_guids022(),
                Actor::Campaign(s) => s.replay_hero_guids022()?,
            }
        } else {
            vec![]
        };
        visible_frame(&view, &ids)
    }
    pub fn step(&mut self) -> Result<Option<Value>> {
        self.step_using023(|r| r.step())
    }
    pub(super) fn step_using023(
        &mut self,
        step: impl FnOnce(&mut Runtime021) -> Result<Option<Record>>,
    ) -> Result<Option<Value>> {
        if !self.recording024c {
            let mut next = self.clone();
            let record = step(&mut next.inner)?;
            *self = next;
            return record.map(value).transpose();
        }
        let mut next = {
            let _p = dd_data::telemetry024c::scope("clone_replay022");
            self.clone()
        };
        let view = next.inner.inner.inner.observe()?;
        let before = next.frame()?;
        let diag = next.inner.diagnostic()?;
        let Some(mut record) = step(&mut next.inner)? else {
            self.inner = next.inner;
            return Ok(None);
        };
        let after_view = next.inner.inner.inner.observe()?;
        let after = next.frame()?;
        record.before["expedition"] = diag["expedition"].clone();
        record.after["expedition"] = next.inner.diagnostic()?["expedition"].clone();
        let first = next.encoder.last.is_none();
        let before_patch = next.encoder.encode(&before)?;
        let after_patch = next.encoder.encode(&after)?;
        let mut log = value(record)?;
        log["schema_version"] = json!(SCHEMA);
        log["combat_action"] = combat_action(&view, &log["action"])?;
        log["before"]["replay"] = before_patch;
        log["after"]["replay"] = after_patch;
        let b = encounter(&view);
        let a = encounter(&after_view);
        let mut events = vec![];
        // Visible boundaries only. Automatic battles entirely within an action
        // and final outcomes consumed before observation cannot be reconstructed.
        if first && b.is_some() {
            next.encounter += 1;
            events.push(json!({"kind":"start","id":next.encounter,"at":"before","boundary":"observed_entry"}));
        }
        if b.is_some() && (a.is_none() || before["context"] != after["context"]) {
            events.push(json!({"kind":"end","id":next.encounter,"at":"after","outcome":battle(&after_view).and_then(|b|b.outcome.as_ref()),"boundary":"observed_exit"}));
        }
        if a.is_some() && (b.is_none() || before["context"] != after["context"]) {
            next.encounter += 1;
            events.push(json!({"kind":"start","id":next.encounter,"at":"after","boundary":"observed_entry"}));
        }
        log["encounters"] = value(events)?;
        *self = next;
        Ok(Some(log))
    }
    pub fn checkpoint(&self) -> Result<String> {
        serde_json::to_string(&Save022 {
            schema_version: SCHEMA.into(),
            inner: self.inner.checkpoint()?,
            encoder: self.encoder.clone(),
            encounter: self.encounter,
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(
        text: &str,
        definition: Definition,
        rules: Option<crate::tactics021::Rules>,
    ) -> Result<Self> {
        let save = parse_save(text)?;
        let r = Self {
            recording024c: true,
            inner: Runtime021::restore(&save.inner, definition, rules)?,
            encoder: save.encoder,
            encounter: save.encounter,
        };
        validate_restored(r)
    }
    pub fn restore_config(text: &str) -> Result<Self> {
        let save = parse_save(text)?;
        let r = Self {
            recording024c: true,
            inner: Runtime021::restore_config(&save.inner)?,
            encoder: save.encoder,
            encounter: save.encounter,
        };
        validate_restored(r)
    }
}
fn parse_save(text: &str) -> Result<Save022> {
    if text.len() > 180 * 1024 * 1024 {
        return Err("022 snapshot exceeds bound".into());
    }
    let save: Save022 = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if save.schema_version != SCHEMA {
        return Err("022 schema mismatch".into());
    }
    Ok(save)
}
fn validate_restored(mut r: Replay) -> Result<Replay> {
    if let Some(last) = &r.encoder.last.clone()
        && *last != r.frame()?
    {
        return Err("022 delta checkpoint differs from current visible frame".into());
    }
    Ok(r)
}

