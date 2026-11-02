//! Host boundary for future expedition/campaign callers. A policy receives only
//! `Observation`; inputs, checkpoints, results and interludes remain host-owned.
use super::*;
use dd_data::{candidate012::CombatCandidates, content::ContentConfig, items::SupplyStack};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BattleInput {
    pub scenario: Scenario,
    pub seed: u64,
    pub supplies: Vec<SupplyStack>,
    pub catalog: BTreeMap<String, Blueprint>,
    pub waves: Vec<Vec<Blueprint>>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Decision {
    Player(Observation),
    WaveInterlude {
        visible: Observation,
        handoff: FarmInterlude,
    },
    Finished(BattleResult),
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Transition {
    /// Settled events from the actual action and automatic transitions only.
    pub events: Vec<Event>,
    pub decision: Decision,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BattleSession {
    battle: Battle,
}
impl BattleSession {
    pub(crate) fn shared_goal_kills020(&self) -> Vec<(UnitId, String, String)> {
        self.battle.shared_goal_kills020()
    }
    pub fn new(input: BattleInput) -> Result<Self> {
        Self::new_with_reserves(input, BTreeMap::new())
    }
    /// Physical stacks are installed before automatic initialization and initiative.
    pub fn new_with_reserves(
        mut input: BattleInput,
        reserves: BTreeMap<String, u32>,
    ) -> Result<Self> {
        // These definitions were supplied directly, not inferred from a missing dependency.
        for unit in input
            .scenario
            .monsters
            .iter()
            .chain(input.waves.iter().flatten())
        {
            if let Some(old) = input.catalog.get(&unit.id) {
                if old != unit {
                    return Err("conflicting input entity definitions".into());
                }
            } else {
                input.catalog.insert(unit.id.clone(), unit.clone());
            }
        }
        Self::from_battle(Battle::build_with_reserves(
            input.scenario,
            input.seed,
            input.supplies,
            input.catalog,
            input.waves,
            reserves,
        )?)
    }
    /// Wrap an already validated special encounter, including Farmstead/nightmares.
    pub fn from_battle(battle: Battle) -> Result<Self> {
        battle.save_json()?;
        Ok(Self { battle })
    }
    /// Complete source-derived host trigger metadata in an existing encounter.
    /// Runtime actors and scenario definitions must stay identical. No RNG,
    /// active buffs, health, scheduling or other simulation state is changed.
    pub(crate) fn bind_host_buff_effects(
        &mut self,
        game: &dd_data::combat::GameData,
    ) -> Result<()> {
        self.transaction(|s| {
            for hero in &mut s.battle.scenario.heroes {
                game.bind_host_buff_effects(hero)?;
            }
            for unit in &mut s.battle.units {
                if unit.blueprint.hero {
                    game.bind_host_buff_effects(&mut unit.blueprint)?;
                }
            }
            s.battle.save_json()?;
            Ok(())
        })
    }
    pub(crate) fn party_state(&self) -> Vec<HeroResult> {
        self.battle.hero_results()
    }
    pub(crate) fn party_order(&self) -> Vec<UnitId> {
        self.battle.order[0]
            .iter()
            .copied()
            .filter(|id| self.battle.units[*id].alive())
            .collect()
    }
    /// Host bookkeeping only; never added to the policy observation.
    pub(crate) fn entered_deaths_door(&self) -> Vec<UnitId> {
        self.battle
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.blueprint.hero && u.entered_deaths_door)
            .map(|(id, _)| id)
            .collect()
    }
    pub(crate) fn loot_eligible_kills(&self) -> std::collections::BTreeSet<UnitId> {
        self.battle
            .lifecycle
            .special
            .killed
            .keys()
            .copied()
            .filter(|id| self.battle.units[*id].loot)
            .collect()
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.battle.save_json()
    }
    pub fn restore(
        json: &str,
        content: &ContentConfig,
        candidates: &CombatCandidates,
    ) -> Result<Self> {
        Self::from_battle(Battle::load_json_with_candidates(
            json, content, candidates,
        )?)
    }
    /// Finish automatic work until a real player choice, wave handoff or final result.
    /// An unsupported automatic transition restores the entire entry state.
    pub fn advance(&mut self) -> Result<Transition> {
        self.transaction(|s| s.advance_inner(vec![]))
    }
    pub fn act(&mut self, action: Action) -> Result<Transition> {
        self.transaction(|s| {
            if s.battle.outcome().is_some() || !s.is_player() {
                return Err("not a player decision boundary".into());
            }
            let events = s.battle.step(action)?;
            s.advance_inner(events)
        })
    }
    pub fn continue_wave(&mut self) -> Result<Transition> {
        self.transaction(|s| {
            if s.battle.outcome() != Some(Outcome::WaveInterlude) {
                return Err("not a wave handoff boundary".into());
            }
            let events = s.battle.continue_farm()?;
            s.advance_inner(events)
        })
    }
    fn transaction<T>(&mut self, run: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let before = self.clone();
        match run(self) {
            Ok(value) => Ok(value),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }
    fn is_player(&self) -> bool {
        self.battle.lifecycle.special.selection.is_some()
            || self
                .battle
                .actor
                .is_some_and(|id| self.battle.units[id].friendly())
    }
    fn advance_inner(&mut self, mut events: Vec<Event>) -> Result<Transition> {
        // Safety bound is an API resource limit, not a simulated game number.
        for _ in 0..4096 {
            if self.battle.outcome() == Some(Outcome::WaveInterlude) {
                self.battle.save_json()?;
                return Ok(Transition {
                    events,
                    decision: Decision::WaveInterlude {
                        visible: self.battle.observation(),
                        handoff: self
                            .battle
                            .farm_interlude()
                            .ok_or("missing wave handoff")?
                            .clone(),
                    },
                });
            }
            if self.battle.outcome().is_some() {
                self.battle.save_json()?;
                return Ok(Transition {
                    events,
                    decision: Decision::Finished(self.battle.result()?),
                });
            }
            if self.is_player() {
                self.battle.save_json()?;
                return Ok(Transition {
                    events,
                    decision: Decision::Player(self.battle.observation()),
                });
            }
            if self.battle.actor.is_none() {
                return Err("automatic boundary has no actor".into());
            }
            events.extend(self.battle.step_monster()?);
        }
        Err("automatic transition resource limit exceeded".into())
    }
}

impl Battle {
    pub(super) fn validate_supply_reserves(&self) -> Result<()> {
        for (id, n) in &self.supply_reserves {
            if *n == 0
                || *n > 1000000
                || !self
                    .supplies
                    .iter()
                    .any(|s| s.definition.id == *id && s.remaining > 0)
            {
                return Err("invalid physical supply reserves".into());
            }
        }
        Ok(())
    }
    pub(super) fn refill_supply(&mut self, item: usize) {
        let s = &mut self.supplies[item];
        if s.remaining == 0
            && let Some(n) = self.supply_reserves.get_mut(&s.definition.id)
        {
            let refill = (*n).min(s.definition.stack_limit);
            s.remaining = refill;
            *n -= refill;
            if *n == 0 {
                self.supply_reserves.remove(&s.definition.id);
            }
        }
    }
}

