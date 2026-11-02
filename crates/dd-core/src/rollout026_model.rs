//! Rebuilds runtime state from visible values and original definitions, never a save.
use super::*;
use dd_data::{
    build010::{Build, Surprise},
    items::SupplyStack,
    mechanics::read_buff,
};
#[derive(Clone)]
pub struct Definitions {
    reinforcements: std::sync::Arc<Vec<(f64, Vec<Blueprint>)>>,
    rules: Rules,
    units: BTreeMap<UnitId, Blueprint>,
    catalog: BTreeMap<String, Blueprint>,
    buffs: BTreeMap<String, Buff>,
    supplies: Vec<SupplyStack>,
}
#[derive(Clone)]
pub struct Surrogate {
    pub(super) battle: Battle,
    boundary: Observation,
    reinforcements: std::sync::Arc<Vec<(f64, Vec<Blueprint>)>>,
}
impl Surrogate {
    pub fn observation(&self) -> Observation {
        self.battle.observation()
    }
    pub(super) fn sample(&self, seed: u64) -> Result<Self> {
        let mut m = self.clone();
        resample(&mut m.battle, &m.boundary, seed);
        if !m.reinforcements.is_empty() {
            let n = crate::expedition_sources017::pick(
                &mut m.battle.rng,
                &m.reinforcements.iter().map(|(w, _)| *w).collect::<Vec<_>>(),
            )?;
            if let Some(c) = &mut m.battle.scenario.rules.context {
                c.reinforcements = m.reinforcements[n].1.clone();
            }
        }
        Ok(m)
    }
    pub(super) fn step(&mut self, action: Action) -> Result<Vec<Event>> {
        // The surrogate is disposable on error; real Battle::step stays transactional.
        let events = self.battle.step_settled(action)?;
        if let Some(e) = self.battle.lifecycle.failure.take() {
            return Err(e);
        }
        Ok(events)
    }
}
fn resample(b: &mut Battle, o: &Observation, seed: u64) {
    b.rng = Rng(seed);
    let mut rolls = vec![];
    for v in &o.units {
        let count = v
            .actions_remaining
            .saturating_sub(usize::from(o.actor == Some(v.id)));
        for _ in 0..count {
            let speed = b.units[v.id].stats().speed;
            rolls.push((
                v.id,
                speed + b.rng.below(b.scenario.rules.initiative_base as u64) as i32 + 1,
                speed,
                b.rng.next(),
            ));
        }
    }
    rolls.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)));
    b.queue = rolls.into_iter().map(|r| r.0).collect();
}
impl Definitions {
    pub fn project(game: &GameData, input: &PublicBattle) -> Result<Self> {
        if input.content != game.overlay.config {
            return Err("declared_content_mismatch".into());
        }
        let o = &input.observation;
        let mut units = BTreeMap::new();
        for (id, h) in &input.heroes {
            if h.grown_equipment {
                return Err("grown_equipment_not_reconstructed".into());
            }
            let ranks = h
                .skills
                .iter()
                .map(|(s, n)| (s.as_str(), *n))
                .collect::<Vec<_>>();
            let mut bp = game.hero_ranked(&h.class, h.weapon, h.armour, &ranks)?;
            if let Some(u) = o.units.iter().find(|u| u.id == *id) {
                let b = u.build.as_ref().ok_or("hero_build_not_visible")?;
                let catalog = game.trinket_catalog(&b.trinkets)?;
                let physical = u
                    .trinkets
                    .iter()
                    .map(|(name, remaining, quest)| {
                        let def = catalog
                            .get(name)
                            .ok_or("visible_trinket_definition_missing")?;
                        let mut state = dd_data::trinket010::TrinketState::fresh(def.clone());
                        state.remaining = *remaining;
                        state.quest_remaining = *quest;
                        Ok(state)
                    })
                    .collect::<Result<Vec<_>>>()?;
                bp = game.equip_build(
                    bp,
                    Build {
                        trinkets: b.trinkets.clone(),
                        quirks: b.quirks.clone(),
                        diseases: b.diseases.clone(),
                        physical_trinkets: Some(physical),
                        ..Default::default()
                    },
                )?;
            }
            bp = game.hero_at_difficulty(bp, h.level, input.difficulty, false)?;
            game.bind_host_buff_effects(&mut bp)?;
            units.insert(*id, bp);
        }
        let names = o
            .units
            .iter()
            .filter(|u| !u.hero && u.controlled.is_none())
            .map(|u| u.class.clone())
            .collect::<Vec<_>>();
        let mut catalog =
            game.entity_catalog(&names.iter().map(String::as_str).collect::<Vec<_>>())?;
        let mut reinforcements = vec![];
        if let Some(region) = &input.region {
            for group in
                crate::expedition_sources017::rollout_stall026(game, region, input.difficulty)?
            {
                let definitions = game.entity_catalog(
                    &group
                        .monsters
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )?;
                let bps = group
                    .monsters
                    .iter()
                    .map(|id| {
                        definitions
                            .get(id)
                            .cloned()
                            .ok_or("stall_definition_missing".into())
                    })
                    .collect::<Result<Vec<_>>>()?;
                catalog.extend(definitions);
                reinforcements.push((group.weight, bps));
            }
            if reinforcements.is_empty() {
                return Err("stall_source_empty".into());
            }
        }
        for u in o.units.iter().filter(|u| !u.hero && u.controlled.is_none()) {
            units.insert(
                u.id,
                catalog
                    .get(&u.class)
                    .ok_or("enemy_definition_missing")?
                    .clone(),
            );
        }
        let mut rules = game.rules.clone();
        if let Some(region) = &input.region {
            let mut context = game.battle_context(region.clone(), Surprise::Neither, false)?;
            context.stall = true;
            context.reinforcements = reinforcements
                .first()
                .ok_or("stall_source_empty")?
                .1
                .clone();
            context.corridor = input.corridor;
            rules.context = Some(context);
        }
        rules.lighting = o.torch.map(|t| game.lighting(t)).transpose()?;
        if input.region.as_deref() == Some("courtyard") {
            rules.bloodlight = Some(game.bloodlight()?);
        }
        if let Some(plot) = &input.plot
            && (plot.starts_with("plot_trinket_retention_")
                || plot.starts_with("plot_darkest_dungeon_")
                || plot.starts_with("plot_color_of_madness_"))
        {
            rules.encounter_contract = Some(game.battle_contract(plot)?);
        }
        if let Some(allowed) = input.retreat_legal {
            if let Some(c) = &rules.encounter_contract {
                if c.can_retreat != allowed {
                    return Err("public_retreat_contract_mismatch".into());
                }
            } else {
                rules.encounter_contract=Some(dd_data::contract011::Contract {id:"public-retreat-legality-026".into(),source:"filtered legal-action boundary; conservative constant until next observation".into(),can_retreat:allowed,raid_retreat_deaths:0,retreat_always_from_raid:false,death_override:None});
            }
        }
        let mut buffs = BTreeMap::new();
        fn collect(v: &Value, known: &mut BTreeMap<String, Vec<Buff>>) {
            match v {
                Value::Array(a) => {
                    for v in a {
                        collect(v, known)
                    }
                }
                Value::Object(m) => {
                    if m.contains_key("stat_type")
                        && m.contains_key("rule_type")
                        && let Ok(b) = serde_json::from_value::<Buff>(v.clone())
                    {
                        let rows = known.entry(b.id.clone()).or_default();
                        if !rows.contains(&b) {
                            rows.push(b);
                        }
                    }
                    for v in m.values() {
                        collect(v, known)
                    }
                }
                _ => {}
            }
        }
        let mut inline = BTreeMap::new();
        collect(
            &serde_json::to_value((&units, &catalog, &rules)).map_err(|e| e.to_string())?,
            &mut inline,
        );
        for (id, _) in o.units.iter().flat_map(|u| &u.buffs) {
            // Curios, traps and prior skills can leave generated buffs whose
            // original effect is public but not among currently equipped skills.
            if let Some((name, _)) = id.rsplit_once(':')
                && !inline.contains_key(id)
                && let Ok(effect) = game.effect(name)
            {
                collect(
                    &serde_json::to_value(effect).map_err(|e| e.to_string())?,
                    &mut inline,
                );
            }
            let definition = match read_buff(&game.buffs, id) {
                Ok(b) => b,
                Err(e) => match inline.get(id).map(Vec::as_slice) {
                    Some([b]) => b.clone(),
                    Some(_) => return Err(format!("buff_definition_ambiguous:{id}")),
                    None => return Err(format!("buff_definition:{id}:{e}")),
                },
            };
            buffs.insert(id.clone(), definition);
        }
        let candidates = dd_data::opening015::Candidates::documented()?;
        let supplies = o
            .supplies
            .iter()
            .map(|(id, n)| {
                Ok(SupplyStack {
                    definition: if id == "torch" {
                        if input.region.as_deref() == Some("courtyard") {
                            game.bloodlight_torch()?
                        } else {
                            game.supply_with_light_candidate(
                                id,
                                dd_data::items::HostItemCandidate {
                                    light_amount: candidates.torch_amount,
                                    source: candidates.source.clone(),
                                },
                            )?
                        }
                    } else {
                        game.supply(id)?
                    },
                    remaining: *n,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            reinforcements: std::sync::Arc::new(reinforcements),
            rules,
            units,
            catalog,
            buffs,
            supplies,
        })
    }
}
pub fn reconstruct(input: &PublicBattle, defs: &Definitions, seed: u64) -> Result<Surrogate> {
    reconstruct_checked(input, defs, seed, None)
}
/// Historical022/023 logs saved complete units but omitted several battle fields.
/// Complete only those omitted fields from source; never count this as full-log roundtrip.
pub(super) fn historical(input: &PublicBattle, defs: &Definitions) -> Result<PublicBattle> {
    let known = ["units", "round", "actor", "torch"];
    let model = reconstruct_checked(input, defs, 0, Some(&known))?;
    let mut completed = input.clone();
    completed.observation = model.observation();
    Ok(completed)
}
fn reconstruct_checked(
    input: &PublicBattle,
    defs: &Definitions,
    seed: u64,
    known: Option<&[&str]>,
) -> Result<Surrogate> {
    let o = &input.observation;
    if o.outcome.is_some() || o.player_selection || o.farm.is_some() {
        return Err("unsupported_terminal_selection_or_farm".into());
    }
    if o.units.iter().any(|u| {
        u.controlled.is_some() || u.captive.is_some() || !u.parasites.is_empty() || u.immobilized
    }) {
        return Err("unsupported_control_capture_parasite".into());
    }
    if !o.shared_health.is_empty() || !o.rank_targets.is_empty() {
        return Err("unsupported_shared_health_or_rank_mark".into());
    }
    if defs.rules.candidates.identity()? != o.candidate_id
        || defs.rules.content.id()? != o.content_id
    {
        return Err("public_definition_identity_mismatch".into());
    }
    let max = o
        .units
        .iter()
        .map(|u| u.id)
        .chain(input.heroes.keys().copied())
        .max()
        .ok_or("no_visible_units")?;
    let placeholder = defs.units.values().next().ok_or("no_public_blueprint")?;
    let mut units = (0..=max)
        .map(|id| {
            let mut u = Unit::fresh(defs.units.get(&id).unwrap_or(placeholder));
            u.life = Life::Dead;
            u.hp = 0;
            u
        })
        .collect::<Vec<_>>();
    for v in &o.units {
        let bp = defs
            .units
            .get(&v.id)
            .ok_or("visible_unit_definition_missing")?;
        if bp.mechanics.lifecycle.lifetime.is_some() && !v.corpse {
            return Err("entity_lifetime_age_unavailable".into());
        }
        if v.class != bp.id || v.hero != bp.hero {
            return Err("visible_unit_definition_class_mismatch".into());
        }
        let mut u = Unit::fresh(bp);
        u.link_bound = bp
            .mechanics
            .life_link
            .as_ref()
            .is_some_and(|family| defs.units.values().any(|b| b.mechanics.family == *family));
        u.life = if v.corpse {
            Life::Corpse {
                expires: o.round + bp.corpse.as_ref().map_or(1, |c| c.rounds),
            }
        } else {
            Life::Alive
        };
        u.hp = v.hp;
        u.stunned = v.stunned;
        u.stress = v.stress;
        u.resolve = v.resolve.clone();
        u.mode = v.mode.clone();
        u.item_uses = v.item_uses.iter().cloned().collect();
        u.shuffle_remaining = v.shuffle_remaining;
        let budget = bp
            .mechanics
            .lifecycle
            .turns
            .max(v.actions_remaining.min(255) as u8);
        u.round_budget = budget;
        u.round_actions = u32::from(budget).saturating_sub(v.actions_remaining as u32)
            + u32::from(o.actor == Some(v.id));
        u.turns = o.round.saturating_sub(1) * u32::from(budget) + u.round_actions;
        u.rank_turns = u.turns;
        u.turn_rank = v.rank;
        u.entered_deaths_door = v.hero && v.hp == 0;
        u.marked = v.marked.then_some(u.turns + 1);
        u.burn = v.burn;
        u.blocks = v.damage_blocks;
        u.guard = v.guarding.map(|(ward, remaining)| GuardState {
            ward,
            remaining,
            applied_turn: u.turns.saturating_sub(1),
        });
        if let Some(remaining) = v.riposte {
            let found = bp
                .skills
                .iter()
                .flat_map(|s| &s.effects)
                .find_map(|e| {
                    if let EffectKind::Riposte { hit, miss, buffs } = &e.kind {
                        Some((*hit, *miss, buffs.clone()))
                    } else {
                        None
                    }
                })
                .ok_or("riposte_definition_not_visible")?;
            u.riposte = Some(RiposteState {
                permanent: v.permanent_riposte,
                remaining,
                applied_turn: u.turns.saturating_sub(1),
                hit_chance: found.0,
                miss_chance: found.1,
                buffs: found.2,
            });
        }
        for (name, remaining) in &v.skill_remaining {
            let i = bp
                .skills
                .iter()
                .position(|s| s.id == *name)
                .ok_or("visible_limited_skill_missing")?;
            u.skill_remaining[i] = Some(*remaining);
        }
        for (name, left) in &v.turn_skill_remaining {
            let i = bp
                .skills
                .iter()
                .position(|s| s.id == *name)
                .ok_or("visible_turn_skill_missing")?;
            u.turn_uses.insert(
                i,
                bp.skills[i]
                    .tactical
                    .per_turn_limit
                    .unwrap_or(0)
                    .saturating_sub(*left),
            );
        }
        for (name, remaining) in &v.buffs {
            let definition = defs.buffs.get(name).ok_or("active_buff_missing")?.clone();
            let scope = if bp.mechanics.build.buffs.iter().any(|b| b.id == *name) {
                BuffScope::Build
            } else {
                BuffScope::Timed
            };
            u.buffs.push(ActiveBuff {
                host_instance: None,
                battle_persistent: false,
                skill_source: scope == BuffScope::Timed,
                source_disease: None,
                clearable: definition.clearable,
                debuff: definition.amount < 0. && !definition.force_positive,
                definition,
                remaining: *remaining,
                applied_turn: u.turns.saturating_sub(1),
                scope,
            });
        }
        for (ds, kind) in [
            (&v.bleed, 0),
            (&v.blight, 1),
            (&v.horror, 2),
            (&v.regeneration, 3),
        ] {
            for (amount, remaining) in ds {
                u.dots.push(Dot {
                    can_crit_heal: false,
                    amount: *amount,
                    remaining: *remaining,
                    kind: match kind {
                        0 => EffectKind::Bleed(*amount),
                        1 => EffectKind::Blight(*amount),
                        2 => EffectKind::Horror(*amount),
                        _ => EffectKind::Regen(*amount),
                    },
                });
            }
        }
        units[v.id] = u;
    }
    let mut order = [vec![], vec![]];
    for side in [0, 1] {
        let mut visible = o
            .units
            .iter()
            .filter(|u| usize::from(!u.hero) == side && (!u.corpse || u.size > 0))
            .collect::<Vec<_>>();
        visible.sort_by_key(|u| u.rank);
        order[side] = visible.iter().map(|u| u.id).collect();
    }
    let mut b = Battle {
        profile: PROFILE.into(),
        content_id: o.content_id.clone(),
        scenario: Scenario {
            heroes: input
                .heroes
                .keys()
                .filter_map(|id| defs.units.get(id).cloned())
                .collect(),
            monsters: o
                .units
                .iter()
                .filter(|u| !u.hero)
                .filter_map(|u| defs.units.get(&u.id).cloned())
                .collect(),
            rules: defs.rules.clone(),
            round_limit: o.round.saturating_add(100),
        },
        supplies: defs.supplies.clone(),
        supply_reserves: BTreeMap::new(),
        units,
        order,
        queue: VecDeque::new(),
        actor: o.actor,
        round: o.round,
        rng: Rng(seed),
        outcome: None,
        pending_reactions: VecDeque::new(),
        retreat_attempts: 0,
        effect_jobs: VecDeque::new(),
        lifecycle: lifecycle::Runtime {
            catalog: defs.catalog.clone(),
            wave: o.wave.saturating_sub(1),
            ..Default::default()
        },
    };
    b.lifecycle.zones = o
        .rank_burns
        .iter()
        .map(|(hero, rank, amount, n)| heroes010::RankBurn {
            hero: *hero,
            rank: *rank,
            amount: *amount,
            expires: o.round + *n,
        })
        .collect();
    b.lifecycle.stall_count = o.round.saturating_sub(1);
    b.initialize_trinkets()?;
    b.refresh_context(false);
    for v in &o.units {
        b.prepare_entity_desires(v.id);
    }
    resample(&mut b, o, seed);
    let rebuilt = b.observation();
    if &rebuilt != o {
        let a = serde_json::to_value(o).map_err(|e| e.to_string())?;
        let c = serde_json::to_value(rebuilt).map_err(|e| e.to_string())?;
        let keys = a
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| a[*k] != c[*k] && known.is_none_or(|xs| xs.contains(&k.as_str())))
            .cloned()
            .collect::<Vec<_>>();
        if !keys.is_empty() {
            let detail = if keys.iter().any(|k| k == "units") {
                o.units
                    .iter()
                    .zip(&b.observation().units)
                    .flat_map(|(a, c)| {
                        let av = serde_json::to_value(a).unwrap();
                        let cv = serde_json::to_value(c).unwrap();
                        av.as_object()
                            .unwrap()
                            .keys()
                            .filter(|k| av[*k] != cv[*k])
                            .map(|k| format!("{}.{}", a.id, k))
                            .collect::<Vec<_>>()
                    })
                    .take(8)
                    .collect::<Vec<_>>()
                    .join(",")
            } else {
                String::new()
            };
            return Err(format!("roundtrip_mismatch:{}:{detail}", keys.join(",")));
        }
    }
    Ok(Surrogate {
        battle: b,
        boundary: o.clone(),
        reinforcements: defs.reinforcements.clone(),
    })
}
