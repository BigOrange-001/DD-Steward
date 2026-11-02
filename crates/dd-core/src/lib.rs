//! Experimental combat kernel. Candidate formulas are explicit in PROFILE;
//! this is not a calibrated reproduction of the original executable.
use dd_data::combat::{Blueprint, EffectKind, EffectTarget, Rules, Skill, Stats};
use dd_data::mechanics::Buff;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
mod ai008;
pub mod api012;
mod boss011;
mod farm011;
mod rules012;
pub mod validation012;
pub use farm011::{Interlude as FarmInterlude, WaveView};
mod boss_scene011;
pub use boss_scene011::{
    boss_battle, boss_encounter_battle, boss_encounter_battle_in_region, boss_scenario, boss_step,
};
mod capture011;
mod control011;
mod nightmare011;
pub use boss011::SharedHealthView;
mod lifecycle;
mod lifecycle_scene;
pub use lifecycle_scene::{LifecycleBaseline, LifecycleCounts, lifecycle_scenario};
mod battle_result010;
mod context_scene010;
mod disease010;
mod heroes010;
mod trinkets010;
pub use battle_result010::{BattleResult, HeroResult, InventoryReceipt};
pub use context_scene010::{ContextBaseline, context_scenario};
mod scheduling;
mod systems;
mod tactical;
mod tactical_scene;
pub use tactical_scene::{TacticalBaseline, TacticalCounts, tactical_scenario};
pub mod execution024b;
pub mod execution025;
mod mechanisms023;
pub mod preparation024;
pub mod rollout026;
mod rounding023;
mod traits;

pub const PROFILE: &str = "candidate-combat-012";

/// Full four-cell formation with a parsed two-cell, single-action monster.
pub fn formation_scenario(g: &dd_data::combat::GameData) -> Result<Scenario> {
    Ok(Scenario {
        heroes: vec![
            g.hero(
                "highwayman",
                4,
                4,
                &["point_blank_shot", "pistol_shot", "wicked_slice"],
            )?,
            g.hero("leper", 4, 4, &["hew", "chop"])?,
            g.hero("bounty_hunter", 4, 4, &["collect_bounty", "come_hither"])?,
            g.hero("vestal", 4, 4, &["divine_grace", "gods_comfort"])?,
        ],
        monsters: vec![
            g.monster("skeleton_common_A")?,
            g.monster("templar_ranged_D")?,
            g.monster("skeleton_courtier_A")?,
        ],
        rules: g.rules.clone(),
        round_limit: 100,
    })
}

/// Fixed candidate-004 scene, using only parsed base-game data.
pub fn displacement_scenario(g: &dd_data::combat::GameData) -> Result<Scenario> {
    Ok(Scenario {
        heroes: vec![
            g.hero("highwayman", 0, 0, &["point_blank_shot", "pistol_shot"])?,
            g.hero("hellion", 0, 0, &["wicked_hack", "barbaric_yawp"])?,
            g.hero(
                "bounty_hunter",
                0,
                0,
                &["collect_bounty", "come_hither", "flashbang", "target_tag"],
            )?,
            g.hero("vestal", 0, 0, &["divine_grace", "gods_comfort"])?,
        ],
        monsters: vec![
            g.monster("cultist_brawler_A")?,
            g.monster("cultist_brawler_A")?,
            g.monster("brigand_fusilier_A")?,
            g.monster("skeleton_arbalist_A")?,
        ],
        rules: g.rules.clone(),
        round_limit: 100,
    })
}
/// Uses supported original skills and ordinary monsters in both presets.
pub fn stealth_scenario(g: &dd_data::combat::GameData) -> Result<Scenario> {
    Ok(Scenario {
        heroes: vec![
            g.hero(
                "grave_robber",
                0,
                0,
                &["shadow_fade", "lunge", "thrown_dagger"],
            )?,
            g.hero("highwayman", 0, 0, &["wicked_slice", "pistol_shot"])?,
            g.hero("bounty_hunter", 0, 0, &["collect_bounty", "come_hither"])?,
            g.hero("vestal", 0, 0, &["divine_grace", "gods_comfort"])?,
        ],
        monsters: vec![
            g.monster("skeleton_common_A")?,
            g.monster("cultist_brawler_A")?,
            g.monster("skeleton_courtier_A")?,
            g.monster("skeleton_arbalist_A")?,
        ],
        rules: g.rules.clone(),
        round_limit: 100,
    })
}

/// A policy example that sees only the public observation.
pub struct StealthBaseline;
impl Policy for StealthBaseline {
    fn choose(&mut self, o: &Observation) -> Option<Action> {
        if o.units
            .iter()
            .any(|u| Some(u.id) == o.actor && u.class == "grave_robber" && !u.stealthed)
            && let Some(a) = o.actions.iter().find(|a| a.label == "shadow_fade")
        {
            return Some(a.action.clone());
        }
        Baseline.choose(o)
    }
}
pub type Result<T> = std::result::Result<T, String>;
pub type UnitId = usize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scenario {
    pub heroes: Vec<Blueprint>,
    pub monsters: Vec<Blueprint>,
    pub rules: Rules,
    pub round_limit: u32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    HeroesWin,
    MonstersWin,
    RoundLimit,
    Retreated,
    MutualDefeat,
    ResolutionLimit,
    Teleported,
    FlashbackFailed,
    WaveInterlude,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Sacrifice { target: UnitId },
    Item { item: usize, target: UnitId },
    Skill { skill: usize, target: UnitId },
    SkillAuto { skill: usize },
    Move { rank: u8 },
    Pass,
    Retreat,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Life {
    Alive,
    Corpse { expires: u32 },
    Dead,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Dot {
    can_crit_heal: bool,
    amount: i32,
    remaining: u32,
    kind: EffectKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Unit {
    miss_streak: u32,
    item_uses: BTreeMap<String, u32>,
    source_id: String,
    shuffle_remaining: u32,
    phase_overkill: i32,
    previous_form: Option<(String, i32)>,
    controlled: Option<control011::Control>,
    captive: Option<capture011::Captive>,
    parasites: Vec<capture011::Parasite>,
    immobilized: Option<capture011::Immobilized>,
    shared_max: Option<i32>,
    shared_base_max: Option<i32>,
    trinkets: Vec<dd_data::trinket010::TrinketState>,
    equipment_modifiers: Vec<Buff>,
    disease_turns: BTreeMap<String, u32>,
    resolve_history: BTreeMap<String, u32>,
    critical_context: bool,
    disease_tags: BTreeMap<String, Vec<String>>,
    quest_uses_context: BTreeMap<String, u32>,
    item_blocked: std::collections::BTreeSet<usize>,
    aura_modifiers: Vec<Buff>,
    region_context: Option<String>,
    corridor_context: bool,
    walking_backwards_context: bool,
    light_context: Option<u8>,
    items_context: Vec<String>,
    monster_counts: BTreeMap<String, u32>,
    diseases: Vec<String>,
    mode: Option<String>,
    door_reaction_round: Option<u32>,
    hp_reaction_active: Vec<bool>,
    light_modifiers: Vec<Buff>,
    round_actions: u32,
    round_budget: u8,
    round_extra: u8,
    turn_adjust: i8,
    next_bonus: u8,
    turn_uses: BTreeMap<usize, u32>,
    turn_history: BTreeMap<String, u32>,
    used_skills: BTreeMap<String, u32>,
    desire_rolls: Vec<bool>,
    loot: bool,
    expires: Option<u32>,
    link_bound: bool,
    death_rank: u8,
    blueprint: Blueprint,
    hp: i32,
    life: Life,
    stunned: bool,
    dots: Vec<Dot>,
    buffs: Vec<ActiveBuff>,
    stress: i32,
    resolve: Resolve,
    resolve_epoch: u32,
    entered_deaths_door: bool,
    turns: u32,
    rank_turns: u32,
    turn_rank: u8,
    cooldowns: BTreeMap<usize, u32>,
    skill_remaining: Vec<Option<u32>>,
    marked: Option<u32>,
    heart_recovery_pending: bool,
    guard: Option<GuardState>,
    riposte: Option<RiposteState>,
    blocks: u32,
    blocks_timer: Option<(u32, u32)>,
    rank_context: u8,
    round_context: u32,
    guarded_context: bool,
    burn: i32,
    any_burning_context: bool,
    counter_context: bool,
    attack_context: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GuardState {
    ward: UnitId,
    remaining: u32,
    applied_turn: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RiposteState {
    permanent: bool,
    remaining: u32,
    applied_turn: u32,
    hit_chance: f64,
    miss_chance: f64,
    buffs: Vec<Buff>,
}
struct AttackRun {
    skill: Skill,
    targets: Vec<UnitId>,
    slot: Option<usize>,
    counter: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Resolve {
    Calm,
    Afflicted(String),
    Virtuous(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum BuffScope {
    Build,
    Disease,
    Timed,
    DeathsDoor,
    Recovery,
    Trait,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ActiveBuff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host_instance: Option<u64>,
    battle_persistent: bool,
    skill_source: bool,
    source_disease: Option<String>,
    clearable: bool,
    debuff: bool,
    definition: Buff,
    remaining: Option<u32>,
    applied_turn: u32,
    scope: BuffScope,
}
impl Unit {
    fn present(&self) -> bool {
        self.life != Life::Dead && !self.captive.as_ref().is_some_and(|c| c.removed)
    }
    fn active(&self) -> bool {
        self.alive() && self.present()
    }
    fn friendly(&self) -> bool {
        self.blueprint.hero && self.controlled.is_none()
    }
    fn alive(&self) -> bool {
        self.life == Life::Alive
    }
    fn size(&self) -> u8 {
        match self.life {
            Life::Alive => self.blueprint.size,
            Life::Corpse { .. } => self
                .blueprint
                .corpse
                .as_ref()
                .map_or(self.blueprint.size, |c| c.size),
            Life::Dead => 0,
        }
    }
    fn stats(&self) -> Stats {
        self.effective_stats()
    }
}

/// Only this owned value is passed to a policy. No RNG, initiative queue,
/// opponent decision, checkpoint, or reference back to Battle is exposed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    pub candidate_id: String,
    pub farm: Option<WaveView>,
    pub player_selection: bool,
    pub rank_targets: BTreeMap<UnitId, Vec<u8>>,
    pub shared_health: Vec<SharedHealthView>,
    pub rank_burns: Vec<(bool, u8, i32, u32)>,
    pub torch: Option<u8>,
    pub wave: usize,
    pub content_id: String,
    pub supplies: Vec<(String, u32)>,
    pub round: u32,
    pub actor: Option<UnitId>,
    pub units: Vec<VisibleUnit>,
    pub actions: Vec<LegalAction>,
    pub outcome: Option<Outcome>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibleBuild {
    pub trinkets: Vec<String>,
    pub quirks: Vec<String>,
    pub diseases: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibleUnit {
    pub item_uses: Vec<(String, u32)>,
    pub shuffle_remaining: u32,
    pub permanent_riposte: bool,
    pub controlled: Option<(UnitId, u32)>,
    pub captive: Option<(UnitId, bool)>,
    pub parasites: Vec<String>,
    pub immobilized: bool,
    pub trinkets: Vec<(String, Option<u32>, Option<u32>)>,
    pub build: Option<VisibleBuild>,
    pub diseases: Vec<String>,
    pub mode: Option<String>,
    pub actions_remaining: usize,
    pub turn_skill_remaining: Vec<(String, u32)>,
    pub burn: i32,
    pub regeneration: Vec<(i32, u32)>,
    pub guarding: Option<(UnitId, u32)>,
    pub guarded_by: Option<UnitId>,
    pub riposte: Option<u32>,
    pub damage_blocks: u32,
    pub id: UnitId,
    pub class: String,
    pub hero: bool,
    pub rank: u8,
    pub size: u8,
    pub occupied_ranks: Vec<u8>,
    pub hp: i32,
    pub max_hp: i32,
    pub corpse: bool,
    pub stunned: bool,
    pub stealthed: bool,
    pub bleed: Vec<(i32, u32)>,
    pub blight: Vec<(i32, u32)>,
    pub horror: Vec<(i32, u32)>,
    pub stress: i32,
    pub resolve: Resolve,
    pub buffs: Vec<(String, Option<u32>)>,
    pub marked: bool,
    /// Limited equipped hero skills only. Enemy skill inventories are hidden.
    pub skill_remaining: Vec<(String, u32)>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalAction {
    pub action: Action,
    pub label: String,
    pub targets: Vec<UnitId>,
    pub hit_chance: f64,
    pub damage: [i32; 2],
    pub heal: [i32; 2],
    pub stress_heal: i32,
}
pub trait Policy {
    fn choose(&mut self, observation: &Observation) -> Option<Action>;
}

/// Baseline for plumbing and experiments. It is not the original monster AI.
/// Heals injured allies, otherwise prioritizes a low-HP living enemy.
pub struct Baseline;
impl Policy for Baseline {
    fn choose(&mut self, o: &Observation) -> Option<Action> {
        o.actions
            .iter()
            .max_by(|a, b| score(a, o).total_cmp(&score(b, o)))
            .map(|a| a.action.clone())
    }
}
fn score(a: &LegalAction, o: &Observation) -> f64 {
    match a.action {
        Action::Sacrifice { target } => o
            .units
            .iter()
            .find(|u| u.id == target)
            .map_or(f64::NEG_INFINITY, |u| -f64::from(u.hp)),
        Action::Item { .. } => -10000.,
        Action::Pass => -1000.,
        Action::Move { rank } => -100. - f64::from(rank),
        Action::Retreat => -2000.,
        Action::Skill { .. } | Action::SkillAuto { .. } => a
            .targets
            .iter()
            .map(|id| {
                let u = o.units.iter().find(|u| u.id == *id).unwrap();
                if a.stress_heal > 0 {
                    f64::from(u.stress.min(a.stress_heal)) * 2.
                } else if a.heal[1] > 0 {
                    f64::from((u.max_hp - u.hp).min(a.heal[1])) * if u.hp == 0 { 100. } else { 1. }
                } else {
                    let damage = f64::from(a.damage[0] + a.damage[1]) / 2. * a.hit_chance;
                    damage
                        + if u.corpse {
                            -20.
                        } else {
                            8. / f64::from(u.hp.max(1))
                        }
                }
            })
            .sum(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        let threshold = n.wrapping_neg() % n;
        loop {
            let v = self.next();
            if v >= threshold {
                return v % n;
            }
        }
    }
    fn roll(&mut self, p: f64) -> bool {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64) < p
    }
    fn range(&mut self, r: [i32; 2]) -> i32 {
        r[0] + self.below((i64::from(r[1]) - i64::from(r[0]) + 1) as u64) as i32
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Event {
    FarmWave {
        group: u32,
        wave: usize,
        quota: u32,
        region: String,
        light: String,
    },
    FarmInterlude {
        group: u32,
        wave: usize,
        group_complete: bool,
    },
    Escaped {
        id: UnitId,
        class: String,
    },
    Teleported {
        actor: UnitId,
    },
    Controlled {
        hero: UnitId,
        controller: UnitId,
        remaining: u32,
    },
    ControlReleased {
        hero: UnitId,
        controller: UnitId,
    },
    Captured {
        hero: UnitId,
        captor: UnitId,
        removed: bool,
    },
    Released {
        hero: UnitId,
        captor: UnitId,
    },
    Bark {
        actor: UnitId,
        id: String,
    },
    SharedHealth {
        group: String,
        hp: i32,
        max_hp: i32,
    },
    TrinketChanged {
        unit: Option<UnitId>,
        from: String,
        to: Option<String>,
    },
    TrinketGained(String),
    Mode {
        unit: UnitId,
        mode: String,
    },
    Light {
        before: u8,
        after: u8,
    },
    BonusAction {
        actor: UnitId,
        skill: String,
    },
    Spawned {
        unit: UnitId,
        class: String,
        rank: u8,
        loot: bool,
    },
    Changed {
        unit: UnitId,
        from: String,
        to: String,
    },
    Revived(UnitId),
    Wave(usize),
    InitiativeChanged {
        unit: UnitId,
        amount: i32,
        next_round: bool,
    },
    LifecycleResolved {
        actor: UnitId,
        target: UnitId,
        applied: bool,
    },
    GuardEstablished {
        protector: UnitId,
        ward: UnitId,
    },
    GuardCleared {
        protector: UnitId,
        ward: UnitId,
    },
    GuardIntercept {
        selected: UnitId,
        target: UnitId,
    },
    CounterAttack {
        actor: UnitId,
        target: UnitId,
    },
    DamageBlocked {
        target: UnitId,
        remaining: u32,
    },
    SupplyConsumed {
        actor: UnitId,
        item: String,
        remaining: u32,
    },
    StealthEnded {
        target: UnitId,
        reason: StealthEndReason,
    },
    MoveSkipped {
        actor: UnitId,
        reason: MoveFailure,
    },
    Round(u32),
    Miss {
        actor: UnitId,
        target: UnitId,
    },
    Damage {
        target: UnitId,
        amount: i32,
        crit: bool,
    },
    CriticalHeal {
        actor: UnitId,
        target: UnitId,
        raw: i32,
        amount: i32,
    },
    FatedHit {
        actor: UnitId,
        target: UnitId,
    },
    Heal {
        target: UnitId,
        amount: i32,
    },
    Effect {
        target: UnitId,
        name: String,
        applied: bool,
    },
    StunSkip(UnitId),
    DeathsDoor(UnitId),
    Death(UnitId),
    Corpse(UnitId),
    Move {
        actor: UnitId,
        rank: u8,
    },
    Pass(UnitId),
    Stress {
        target: UnitId,
        change: i32,
        value: i32,
    },
    Resolve {
        target: UnitId,
        state: Resolve,
    },
    HeartAttack(UnitId),
    Buff {
        target: UnitId,
        id: String,
    },
    ActOut {
        actor: UnitId,
        kind: dd_data::mechanics::TurnKind,
    },
    ActOutNoTarget {
        actor: UnitId,
        kind: dd_data::mechanics::TurnKind,
    },
    ForcedAction {
        actor: UnitId,
        action: Action,
    },
    Reaction {
        actor: UnitId,
        kind: dd_data::mechanics::ReactionKind,
        subject: UnitId,
    },
    Refused {
        actor: UnitId,
        target: UnitId,
        kind: dd_data::mechanics::ReactionKind,
        effect: Option<String>,
    },
    RetreatAttempt {
        attempt: u32,
        success: bool,
    },
    ResolutionLimit {
        pending: usize,
    },
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum StealthEndReason {
    Attack,
    Effect,
    Death,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum MoveFailure {
    SingleUnit,
    Boundary,
    NotPresent,
    CorpseNotAllowed,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Battle {
    #[serde(skip)]
    effect_jobs: VecDeque<rules012::QueuedEffect>,
    lifecycle: lifecycle::Runtime,
    profile: String,
    content_id: String,
    scenario: Scenario,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    supply_reserves: BTreeMap<String, u32>,
    supplies: Vec<dd_data::items::SupplyStack>,
    units: Vec<Unit>,
    order: [Vec<UnitId>; 2],
    queue: VecDeque<UnitId>,
    actor: Option<UnitId>,
    round: u32,
    rng: Rng,
    outcome: Option<Outcome>,
    pending_reactions: VecDeque<traits::ReactionJob>,
    retreat_attempts: u32,
}
// Keep deserialization private so callers cannot bypass load_json validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedBattle {
    #[serde(default)]
    supply_reserves: BTreeMap<String, u32>,
    lifecycle: lifecycle::Runtime,
    profile: String,
    content_id: String,
    scenario: Scenario,
    supplies: Vec<dd_data::items::SupplyStack>,
    units: Vec<Unit>,
    order: [Vec<UnitId>; 2],
    queue: VecDeque<UnitId>,
    actor: Option<UnitId>,
    round: u32,
    rng: Rng,
    outcome: Option<Outcome>,
    pending_reactions: VecDeque<traits::ReactionJob>,
    retreat_attempts: u32,
}
impl Battle {
    pub fn new(scenario: Scenario, seed: u64) -> Result<Self> {
        Self::new_with_supplies(scenario, seed, vec![])
    }
    pub fn new_with_supplies(
        scenario: Scenario,
        seed: u64,
        supplies: Vec<dd_data::items::SupplyStack>,
    ) -> Result<Self> {
        let catalog = scenario
            .monsters
            .iter()
            .chain(
                scenario
                    .rules
                    .context
                    .iter()
                    .flat_map(|c| c.reinforcements.iter()),
            )
            .map(|b| (b.id.clone(), b.clone()))
            .collect();
        Self::build(scenario, seed, supplies, catalog, vec![])
    }
    fn build(
        scenario: Scenario,
        seed: u64,
        supplies: Vec<dd_data::items::SupplyStack>,
        catalog: BTreeMap<String, Blueprint>,
        waves: Vec<Vec<Blueprint>>,
    ) -> Result<Self> {
        Self::build_with_reserves(scenario, seed, supplies, catalog, waves, BTreeMap::new())
    }
    fn build_with_reserves(
        scenario: Scenario,
        seed: u64,
        supplies: Vec<dd_data::items::SupplyStack>,
        catalog: BTreeMap<String, Blueprint>,
        waves: Vec<Vec<Blueprint>>,
        reserves: BTreeMap<String, u32>,
    ) -> Result<Self> {
        validate_scenario(&scenario)?;
        validate_supplies(&supplies)?;
        let content_id = scenario.rules.content.id()?;
        let count = scenario.heroes.len();
        let units = scenario
            .heroes
            .iter()
            .chain(&scenario.monsters)
            .map(Unit::fresh)
            .collect::<Vec<_>>();
        let total = units.len();
        let mut b = Self {
            lifecycle: lifecycle::Runtime {
                catalog,
                waves,
                ..Default::default()
            },
            profile: PROFILE.into(),
            content_id,
            scenario,
            supplies,
            supply_reserves: reserves,
            units,
            order: [(0..count).collect(), (count..total).collect()],
            queue: VecDeque::new(),
            actor: None,
            round: 0,
            rng: Rng(seed),
            outcome: None,
            effect_jobs: VecDeque::new(),
            pending_reactions: VecDeque::new(),
            retreat_attempts: 0,
        };
        for i in 0..b.supplies.len() {
            if b.supplies[i].remaining == 0 {
                b.refill_supply(i);
            }
        }
        b.validate_supply_reserves()?;
        b.validate_environment_supplies()?;
        b.refresh_context(false);
        b.validate_catalog()?;
        b.initialize_trinkets()?;
        let mut events = vec![];
        for id in 0..total {
            b.initialize_entity(id, &mut events);
            b.apply_wave_spawn(id, &mut events);
        }
        b.initialize_farm(&mut events)?;
        b.initialize_shared_health()?;
        b.initialize_flashback(&mut events)?;
        for id in 0..total {
            if let Some(d) = b.units[id].blueprint.mechanics.hero.difficulty.clone()
                && d.entering_raid
            {
                b.apply_stress_delta(id, d.entry_stress, &mut events);
            }
        }
        b.flush_effects(&mut events);
        b.flush_reactions(&mut events);
        b.flush_lifecycle(&mut events);
        if let Some(ctx) = &b.scenario.rules.context {
            use dd_data::build010::Surprise;
            let surprise = ctx.surprise;
            let monsters = b
                .units
                .iter()
                .filter(|u| !u.blueprint.hero)
                .map(|u| &u.blueprint.mechanics.lifecycle)
                .collect::<Vec<_>>();
            if (surprise == Surprise::Heroes
                && !monsters.iter().any(|s| s.always_surprise)
                && monsters.iter().any(|s| !s.can_surprise))
                || (surprise == Surprise::Monsters && monsters.iter().any(|s| !s.can_be_surprised))
                || (surprise != Surprise::Heroes && monsters.iter().any(|s| s.always_surprise))
                || (surprise != Surprise::Monsters
                    && monsters.iter().any(|s| s.always_be_surprised))
            {
                return Err("surprise outcome contradicts original encounter flags".into());
            }
            if surprise == Surprise::Heroes {
                if dd_data::mechanisms023::enabled(
                    &b.scenario.rules.content,
                    dd_data::mechanisms023::SURPRISE,
                ) {
                    b.surprise_shuffle023(&mut events);
                } else {
                    b.shuffle_party(0, &mut events);
                }
                b.flush_reactions(&mut events);
            }
        }
        b.flush_lifecycle(&mut events);
        b.advance(&mut events);
        if let Some(error) = b.lifecycle.failure.take() {
            return Err(error);
        }
        b.validate_captives()?;
        b.validate_control()?;
        Ok(b)
    }
    /// Host-only scenario initialization, before any action; threshold checks
    /// are caused by subsequent stress changes. Initial values must be 0..99.
    pub fn new_with_stress(scenario: Scenario, seed: u64, stress: &[i32]) -> Result<Self> {
        Self::new_with_stress_and_supplies(scenario, seed, stress, vec![])
    }
    pub fn new_with_stress_and_supplies(
        scenario: Scenario,
        seed: u64,
        stress: &[i32],
        supplies: Vec<dd_data::items::SupplyStack>,
    ) -> Result<Self> {
        if scenario
            .heroes
            .iter()
            .any(|h| h.mechanics.hero.carry.is_some())
        {
            return Err("initial stress supplied twice".into());
        }
        if stress.len() != scenario.heroes.len() || stress.iter().any(|s| !(0..100).contains(s)) {
            return Err("initial stress must supply 0..99 for each hero".into());
        }
        let mut b = Self::new_with_supplies(scenario, seed, supplies)?;
        for (u, s) in b.units.iter_mut().zip(stress) {
            u.stress = *s;
        }
        Ok(b)
    }
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
    pub fn observation(&self) -> Observation {
        let units = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.present() || u.captive.is_some())
            .map(|(id, u)| VisibleUnit {
                shuffle_remaining: u.shuffle_remaining,
                permanent_riposte: u.riposte.as_ref().is_some_and(|r| r.permanent),
                controlled: u.controlled.as_ref().map(|c| (c.controller, c.remaining)),
                captive: u.captive.as_ref().map(|c| (c.captor, c.removed)),
                parasites: u
                    .parasites
                    .iter()
                    .map(|p| p.definition.id.clone())
                    .collect(),
                immobilized: u.immobilized.is_some(),
                trinkets: if u.blueprint.hero {
                    u.trinkets
                        .iter()
                        .filter(|t| !t.destroyed)
                        .map(|t| (t.definition.id.clone(), t.remaining, t.quest_remaining))
                        .collect()
                } else {
                    vec![]
                },
                build: u.blueprint.hero.then(|| VisibleBuild {
                    trinkets: u
                        .trinkets
                        .iter()
                        .filter(|t| !t.destroyed)
                        .map(|t| t.definition.id.clone())
                        .collect(),
                    quirks: u.blueprint.mechanics.build.input.quirks.clone(),
                    diseases: u.diseases.clone(),
                }),
                diseases: if u.blueprint.hero {
                    u.diseases.clone()
                } else {
                    vec![]
                },
                mode: u.mode.clone(),
                item_uses: if u.blueprint.hero {
                    u.item_uses.iter().map(|(id, n)| (id.clone(), *n)).collect()
                } else {
                    vec![]
                },
                actions_remaining: self.queue.iter().filter(|x| **x == id).count()
                    + usize::from(self.actor == Some(id)),
                turn_skill_remaining: if u.blueprint.hero {
                    u.blueprint
                        .skills
                        .iter()
                        .enumerate()
                        .filter_map(|(index, s)| {
                            s.tactical.per_turn_limit.map(|n| {
                                (
                                    s.id.clone(),
                                    n.saturating_sub(u.turn_uses.get(&index).copied().unwrap_or(0)),
                                )
                            })
                        })
                        .collect()
                } else {
                    vec![]
                },
                burn: u.burn,
                regeneration: u
                    .dots
                    .iter()
                    .filter(|d| matches!(d.kind, EffectKind::Regen(_)))
                    .map(|d| (d.amount, d.remaining))
                    .collect(),
                guarding: u.guard.as_ref().map(|g| (g.ward, g.remaining)),
                guarded_by: self.guarder(id),
                riposte: u.riposte.as_ref().map(|r| r.remaining),
                damage_blocks: u.blocks,
                id,
                class: u.blueprint.id.clone(),
                hero: u.friendly(),
                rank: self.rank(id),
                size: u.size(),
                occupied_ranks: if u.present() {
                    self.occupied_ranks(id).collect()
                } else {
                    vec![]
                },
                hp: u.hp,
                max_hp: u.stats().hp,
                corpse: !u.alive(),
                stunned: u.stunned,
                stealthed: u.stealthed(),
                stress: u.stress,
                resolve: u.resolve.clone(),
                buffs: u
                    .buffs
                    .iter()
                    .map(|b| (b.definition.id.clone(), b.remaining))
                    .collect(),
                marked: u.marked.is_some(),
                skill_remaining: if u.blueprint.hero {
                    u.blueprint
                        .skills
                        .iter()
                        .zip(&u.skill_remaining)
                        .filter_map(|(s, n)| n.map(|n| (s.id.clone(), n)))
                        .collect()
                } else {
                    vec![]
                },
                bleed: u
                    .dots
                    .iter()
                    .filter(|d| matches!(d.kind, EffectKind::Bleed(_)))
                    .map(|d| (d.amount, d.remaining))
                    .collect(),
                blight: u
                    .dots
                    .iter()
                    .filter(|d| matches!(d.kind, EffectKind::Blight(_)))
                    .map(|d| (d.amount, d.remaining))
                    .collect(),
                horror: u
                    .dots
                    .iter()
                    .filter(|d| matches!(d.kind, EffectKind::Horror(_)))
                    .map(|d| (d.amount, d.remaining))
                    .chain(
                        self.lifecycle
                            .special
                            .dream
                            .horror
                            .get(&id)
                            .map(|d| (d.amount, d.remaining)),
                    )
                    .collect(),
            })
            .collect();
        Observation {
            candidate_id: self
                .scenario
                .rules
                .candidates
                .identity()
                .expect("validated candidate identity"),
            farm: self.farm_view(),
            player_selection: self.lifecycle.special.selection.is_some(),
            rank_targets: self
                .lifecycle
                .special
                .marked_ranks
                .iter()
                .filter(|(_, rs)| !rs.is_empty())
                .map(|(id, rs)| (*id, rs.clone()))
                .collect(),
            shared_health: self
                .lifecycle
                .special
                .pools
                .iter()
                .filter(|p| p.hp > 0)
                .map(|p| SharedHealthView {
                    id: p.id.clone(),
                    members: p.members.clone(),
                    hp: p.hp,
                    max_hp: p.max,
                })
                .collect(),
            rank_burns: self
                .lifecycle
                .zones
                .iter()
                .map(|z| {
                    (
                        z.hero,
                        z.rank,
                        z.amount,
                        z.expires.saturating_sub(self.round),
                    )
                })
                .collect(),
            torch: self.scenario.rules.lighting.as_ref().map(|l| l.torch),
            wave: self.lifecycle.wave + 1,
            content_id: self.content_id.clone(),
            supplies: self
                .supplies
                .iter()
                .map(|s| {
                    (
                        s.definition.id.clone(),
                        s.remaining
                            + self
                                .supply_reserves
                                .get(&s.definition.id)
                                .copied()
                                .unwrap_or(0),
                    )
                })
                .collect(),
            round: self.round,
            actor: self.actor,
            units,
            actions: if self.lifecycle.special.selection.is_some()
                || self.actor.is_some_and(|id| self.units[id].friendly())
            {
                self.legal_actions()
            } else {
                vec![]
            },
            outcome: self.outcome,
        }
    }
    fn rank(&self, id: UnitId) -> u8 {
        if !self.units[id].present() {
            return 0;
        }
        let mut rank = 1;
        for &other in &self.order[usize::from(!self.units[id].friendly())] {
            if other == id {
                return rank;
            }
            rank += self.units[other].size();
        }
        0
    }
    fn occupied_ranks(&self, id: UnitId) -> std::ops::Range<u8> {
        let start = self.rank(id);
        start..start + self.units[id].size()
    }
    fn overlaps_ranks(&self, id: UnitId, ranks: &[u8]) -> bool {
        self.occupied_ranks(id).any(|r| ranks.contains(&r))
    }
    // Complete blocks can only be inserted at these front-rank boundaries.
    fn placement_ranks(&self, actor: UnitId) -> Vec<u8> {
        let mut ranks = vec![1];
        for &id in &self.order[usize::from(!self.units[actor].friendly())] {
            if id != actor && self.units[id].present() {
                ranks.push(ranks.last().unwrap() + self.units[id].size());
            }
        }
        if self.units[actor].immobilized.is_some() {
            return vec![self.rank(actor)];
        }
        let side = usize::from(!self.units[actor].friendly());
        let pinned = self.order[side]
            .iter()
            .filter(|id| self.units[**id].present() && self.units[**id].immobilized.is_some())
            .copied()
            .collect::<Vec<_>>();
        ranks.retain(|rank| {
            let mut next = self.order[side]
                .iter()
                .filter(|id| **id != actor && self.units[**id].present())
                .copied()
                .collect::<Vec<_>>();
            let mut boundary = 1;
            let index = next
                .iter()
                .position(|id| {
                    let here = boundary;
                    boundary += self.units[*id].size();
                    here == *rank
                })
                .unwrap_or(next.len());
            next.insert(index, actor);
            let mut boundary = 1;
            next.iter().all(|id| {
                let okay = !pinned.contains(id) || boundary == self.rank(*id);
                boundary += self.units[*id].size();
                okay
            })
        });
        ranks
    }
    fn displacement_rank(&self, actor: UnitId, wanted: u8) -> u8 {
        let before = self.rank(actor);
        let ranks = self.placement_ranks(actor);
        if wanted < before {
            ranks.into_iter().find(|r| *r >= wanted).unwrap_or(before)
        } else {
            ranks
                .into_iter()
                .rev()
                .find(|r| *r <= wanted)
                .unwrap_or(before)
        }
    }
    fn targets(&self, actor: UnitId, skill: &Skill) -> Vec<UnitId> {
        if skill.target.ranks.is_empty() {
            return vec![actor];
        }
        self.units
            .iter()
            .enumerate()
            .filter(|(id, u)| {
                u.present()
                    && u.blueprint.mechanics.lifecycle.targetable
                    && u.blueprint
                        .mechanics
                        .boss
                        .modes
                        .iter()
                        .find(|m| u.mode.as_ref() == Some(&m.id))
                        .is_none_or(|m| m.targetable)
                    && (skill.target.friendly == (u.friendly() == self.units[actor].friendly()))
                    && (!skill.target.friendly
                        || u.alive() && u.blueprint.mechanics.lifecycle.friendly_targetable)
                    && (skill.target.friendly
                        || skill.target.multi
                        || !u.stealthed()
                        || skill.ignore_stealth
                        || self.units[actor].modifier("ignore_stealth", "") > 0.)
                    && self.overlaps_ranks(*id, &skill.target.ranks)
                    && (skill.tactical.self_target_valid || *id != actor)
                    && (!skill.target.random
                        || if u.alive() {
                            u.blueprint.mechanics.lifecycle.random_targetable
                        } else {
                            u.blueprint
                                .corpse
                                .as_ref()
                                .map_or(u.blueprint.mechanics.lifecycle.random_targetable, |c| {
                                    c.random_targetable
                                })
                        })
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn legal_actions(&self) -> Vec<LegalAction> {
        if self.lifecycle.special.selection.is_some() {
            return self.sacrifice_actions();
        }
        let Some(actor) = self.actor else {
            return vec![];
        };
        let u = &self.units[actor];
        let mut actions = vec![];
        if u.friendly() {
            for (item, stack) in self.supplies.iter().enumerate().filter(|(_, s)| {
                s.remaining > 0
                    && (!s.definition.effects.is_empty()
                        || !s.definition.semantics.sequential_effects.is_empty())
            }) {
                if u.item_blocked.contains(&item)
                    || (stack.definition.host_candidate.is_some()
                        && self.torch_bounds()[0] == self.torch_bounds()[1])
                {
                    continue;
                }
                for (target, hero) in self
                    .units
                    .iter()
                    .enumerate()
                    .filter(|(_, u)| u.alive() && u.blueprint.hero)
                {
                    let _ = hero;
                    if !self.supply_usable(item, target) {
                        continue;
                    }
                    actions.push(LegalAction {
                        action: Action::Item { item, target },
                        label: stack.definition.id.clone(),
                        targets: vec![target],
                        hit_chance: 1.,
                        damage: [0, 0],
                        heal: [0, 0],
                        stress_heal: 0,
                    });
                }
            }
        }
        for (idx, s) in u.blueprint.skills.iter().enumerate() {
            if s.effects.iter().any(|e| e.kind.contains_guard())
                && u.modifier("disable_combat_skill_attribute", "guard") > 0.
            {
                continue;
            }
            if s.heal.is_some() && u.modifier("disable_combat_skill_attribute", "heal") > 0. {
                continue;
            }
            if s.tactical.hp_range.is_some_and(|r| {
                let ratio = f64::from(u.hp) / f64::from(u.stats().hp);
                ratio < r[0] || ratio > r[1]
            }) || (!s.tactical.valid_modes.is_empty()
                && !u
                    .mode
                    .as_ref()
                    .is_some_and(|m| s.tactical.valid_modes.contains(m)))
                || (s.tactical.requires_burning
                    && !self.units.iter().any(|u| u.present() && u.burn > 0))
                || !self.overlaps_ranks(actor, &s.launch)
                || u.skill_remaining[idx] == Some(0)
                || s.tactical
                    .per_turn_limit
                    .is_some_and(|limit| u.turn_uses.get(&idx).copied().unwrap_or(0) >= limit)
                || u.cooldowns.get(&idx).is_some_and(|until| u.turns < *until)
            {
                continue;
            }
            let targets = self.targets(actor, s);
            for &target in &targets {
                let selected = if s.target.multi || s.target.random {
                    targets.clone()
                } else {
                    vec![target]
                };
                let t = &self.units[target];
                // Deterministic display preview; observation must never draw RNG.
                let extra = s
                    .effects
                    .iter()
                    .filter(|e| {
                        matches!(e.kind, EffectKind::InstantBuffs(_))
                            || !e.conditions.is_empty() && self.conditions_match(target, e)
                    })
                    .flat_map(|e| {
                        if let EffectKind::Buffs(b) | EffectKind::InstantBuffs(b) = &e.kind {
                            b.as_slice()
                        } else {
                            &[]
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let (accuracy, raw_range, _) = self.attack_values(actor, target, s, &extra);
                let range = if self.rounding023() && !s.target.friendly {
                    let base = self.damage_bounds023(actor, target, s, &extra);
                    base.map(|raw| self.damage_value023(actor, target, s, raw))
                } else {
                    raw_range.map(|n| candidate_damage(n, t.stats().protection))
                };
                actions.push(LegalAction {
                    action: if s.target.random {
                        Action::SkillAuto { skill: idx }
                    } else {
                        Action::Skill { skill: idx, target }
                    },
                    label: s.id.clone(),
                    targets: selected,
                    hit_chance: if s.target.friendly || !t.alive() {
                        1.
                    } else {
                        candidate_hit(accuracy, t.stats().dodge, &self.scenario.rules)
                    },
                    damage: range,
                    heal: s.heal.unwrap_or([0, 0]).map(|n| {
                        if self.rounding023() && self.units[target].blueprint.hero {
                            let raw = (f64::from(n) + u.modifier("hp_heal_amount", "")).max(0.)
                                * (1. + u.modifier("hp_heal_percent", "")).clamp(
                                    self.scenario.rules.heal_buff_percent_min,
                                    self.scenario.rules.heal_buff_percent_max,
                                );
                            return self.healing_received023(target, raw);
                        }
                        dd_data::mechanisms023::healing_amount(
                            f64::from(n) * (1. + u.modifier("hp_heal_percent", "")),
                            u.blueprint.hero,
                            &self.scenario.rules.content,
                        )
                    }),
                    stress_heal: s
                        .effects
                        .iter()
                        .filter_map(|e| {
                            if let EffectKind::Stress(n) = e.kind {
                                Some(if self.rounding023() {
                                    dd_data::mechanisms023::ceil_final023(
                                        f64::from((-n).max(0))
                                            * (1. + u.modifier("stress_heal_percent", "")).max(0.),
                                    )
                                    .clamp(0., 1e6) as i32
                                } else {
                                    (-n).max(0)
                                })
                            } else {
                                None
                            }
                        })
                        .sum(),
                });
                if s.target.multi || s.target.random {
                    break;
                }
            }
        }
        let rank = usize::from(self.rank(actor));
        for r in self.placement_ranks(actor).into_iter().map(usize::from) {
            if r != rank
                && u.immobilized.is_none()
                && ((r < rank && rank - r <= u.blueprint.move_forward)
                    || (r > rank && r - rank <= u.blueprint.move_back))
            {
                actions.push(LegalAction {
                    action: Action::Move { rank: r as u8 },
                    label: "move".into(),
                    targets: vec![],
                    hit_chance: 1.,
                    damage: [0, 0],
                    heal: [0, 0],
                    stress_heal: 0,
                });
            }
        }
        actions.push(LegalAction {
            action: Action::Pass,
            label: "pass".into(),
            targets: vec![],
            hit_chance: 1.,
            damage: [0, 0],
            heal: [0, 0],
            stress_heal: 0,
        });
        if u.friendly()
            && self
                .scenario
                .rules
                .encounter_contract
                .as_ref()
                .is_none_or(|c| c.can_retreat)
        {
            actions.push(LegalAction {
                action: Action::Retreat,
                label: "retreat".into(),
                targets: vec![],
                hit_chance: 1.,
                damage: [0, 0],
                heal: [0, 0],
                stress_heal: 0,
            });
        }
        actions
    }
    /// Invalid actions leave state and RNG unchanged.
    pub fn step(&mut self, action: Action) -> Result<Vec<Event>> {
        let backup = Some(self.clone());
        let result = self.step_settled(action);
        if let Some(error) = self.lifecycle.failure.take() {
            if let Some(before) = backup {
                *self = before;
            }
            return Err(error);
        }
        if result.is_err()
            && let Some(before) = backup
        {
            *self = before;
        }
        result
    }
    fn step_settled(&mut self, action: Action) -> Result<Vec<Event>> {
        if self.lifecycle.special.selection.is_some() {
            return self.resolve_sacrifice(action);
        }
        if self.outcome.is_some() {
            return Err("battle is finished".into());
        }
        let continues = match &action {
            Action::Skill { skill, .. } | Action::SkillAuto { skill } => self.units
                [self.actor.unwrap()]
            .blueprint
            .skills
            .get(*skill)
            .is_some_and(|s| s.tactical.continue_turn),
            _ => false,
        };
        let legal = self
            .legal_actions()
            .into_iter()
            .find(|a| a.action == action)
            .ok_or("illegal action")?;
        let actor = self.actor.take().unwrap();
        let mut events = vec![];
        if let Action::Item { item, target } = action {
            if self.reaction_block(
                actor,
                target,
                dd_data::mechanics::ReactionKind::BlockItem,
                Some(&self.supplies[item].definition.id.clone()),
                &mut events,
            ) {
                self.units[actor].item_blocked.insert(item);
                self.flush_reactions(&mut events);
                self.flush_lifecycle(&mut events);
                if !self.terminal() {
                    if self.units[actor].alive() && !self.units[actor].stunned {
                        self.actor = Some(actor);
                    } else {
                        self.actor = None;
                        self.advance(&mut events);
                    }
                }
                return Ok(events);
            }
            self.consume_supply(target, item, &mut events)?;
            self.flush_lifecycle(&mut events);
            self.flush_reactions(&mut events);
            self.flush_lifecycle(&mut events);
            self.actor = Some(actor);
            if self.terminal() {
                self.actor = None;
            } else if !self.units[actor].alive() || self.units[actor].stunned {
                self.actor = None;
                self.advance(&mut events);
            }
            return Ok(events);
        }
        self.resolve_action(actor, legal, &mut events);
        self.flush_lifecycle(&mut events);
        if continues && self.units[actor].alive() && !self.units[actor].stunned {
            self.flush_reactions(&mut events);
            self.flush_lifecycle(&mut events);
            if !self.terminal() && self.units[actor].alive() && !self.units[actor].stunned {
                self.actor = Some(actor);
            } else if self.outcome.is_none() {
                self.settle_opportunity(actor, &mut events);
                self.advance(&mut events);
            }
            return Ok(events);
        }
        self.settle_opportunity(actor, &mut events);
        self.advance(&mut events);
        Ok(events)
    }
    fn resolve_action(&mut self, actor: UnitId, legal: LegalAction, events: &mut Vec<Event>) {
        let action = legal.action.clone();
        // Class changes can replace the actor's skill list during execution.
        let hostile_attack = match &action {
            Action::Skill { skill, .. } | Action::SkillAuto { skill } => {
                !self.units[actor].blueprint.skills[*skill].target.friendly
            }
            _ => false,
        };
        self.lifecycle.special.last_damaged = None;
        self.lifecycle.bonus_used.retain(|(id, index)| {
            self.units[*id]
                .blueprint
                .mechanics
                .brain
                .as_ref()
                .is_none_or(|b| b.bonus_initiative_desires[*index]["type"] != "ally_last_damaged")
        });
        match action {
            Action::Sacrifice { .. } => {
                unreachable!("sacrifice resolves at an explicit pending boundary")
            }
            Action::Item { .. } => unreachable!("items resolve without a paid opportunity"),
            Action::Pass => {
                events.push(Event::Pass(actor));
                self.stress_float023(actor, self.scenario.rules.passed_turn_stress_dmg, events);
            }
            Action::Move { rank } => {
                self.voluntary_move(actor, rank, events);
            }
            Action::Retreat => self.attempt_retreat(actor, events),
            Action::Skill { skill, .. } => {
                *self.units[actor].turn_uses.entry(skill).or_default() += 1;
                let id = self.units[actor].blueprint.skills[skill].id.clone();
                *self.units[actor]
                    .turn_history
                    .entry(id.clone())
                    .or_default() += 1;
                if let Some(remaining) = &mut self.units[actor].skill_remaining[skill] {
                    *remaining -= 1;
                    *self.units[actor].used_skills.entry(id).or_default() += 1;
                }
                let s = self.units[actor].blueprint.skills[skill].clone();
                let targets = self.executed_targets(actor, &s, legal.targets);
                self.execute_skill(
                    actor,
                    AttackRun {
                        skill: s,
                        targets,
                        slot: Some(skill),
                        counter: false,
                    },
                    events,
                );
            }
            Action::SkillAuto { skill } => {
                *self.units[actor].turn_uses.entry(skill).or_default() += 1;
                let id = self.units[actor].blueprint.skills[skill].id.clone();
                *self.units[actor]
                    .turn_history
                    .entry(id.clone())
                    .or_default() += 1;
                if let Some(remaining) = &mut self.units[actor].skill_remaining[skill] {
                    *remaining -= 1;
                    *self.units[actor].used_skills.entry(id).or_default() += 1;
                }
                let s = self.units[actor].blueprint.skills[skill].clone();
                let targets = self.executed_targets(actor, &s, legal.targets);
                self.execute_skill(
                    actor,
                    AttackRun {
                        skill: s,
                        targets,
                        slot: Some(skill),
                        counter: false,
                    },
                    events,
                );
            }
        }
        if hostile_attack {
            self.parasite_attack_tick(actor, events);
        }
        self.capture_action_tick(events);
    }
    fn execute_skill(&mut self, actor: UnitId, run: AttackRun, events: &mut Vec<Event>) {
        let AttackRun {
            skill: mut s,
            targets,
            slot,
            counter,
        } = run;
        if let Some(mode) = &self.units[actor].mode
            && let Some(e) = s.tactical.mode_effects.get(mode)
        {
            s.effects.extend(e.clone());
        }
        if !counter {
            if self.units[actor].blueprint.hero && s.tactical.stall_invalidating {
                self.lifecycle.invalidating_actions =
                    self.lifecycle.invalidating_actions.saturating_add(1);
            }
            self.lifecycle.last_skill = Some(s.id.clone());
        }
        let previous_item_root = self.begin_item_root();
        let trinket_event = if counter {
            "riposte_skill"
        } else if s.target.friendly {
            "friendly_skill"
        } else {
            "attack_skill"
        };
        let prior_critical = self.units[actor].critical_context;
        let prior_counter = self.units[actor].counter_context;
        let prior_type = self.units[actor]
            .attack_context
            .replace(s.tactical.attack_type.clone());
        self.units[actor].counter_context = counter;
        let mut struck = std::collections::BTreeSet::new();
        let mut once = std::collections::BTreeSet::new();
        let mut any_crit = false;
        let mut relief_done = false;
        let mut attack_hit = false;
        let mut attack_attempted = false;
        for selected in targets.clone() {
            self.units[actor].critical_context = false;
            if !self.units[actor].alive() {
                break;
            }
            let target = self.redirect_target(actor, selected, &targets, &s, events);
            if !struck.insert(target) {
                continue;
            }
            if !self.units[target].present() {
                continue;
            }
            let living_before = self.units[target].alive();
            let copy_burn = self.units[target].burn;
            let behind_rank = self.rank(target) + self.units[target].size();
            let burn_receiver = (0..self.units.len()).find(|id| {
                *id != target
                    && self.units[*id].present()
                    && self.units[*id].friendly() == self.units[target].friendly()
                    && self.rank(*id) == behind_rank
            });

            // Conditional performer bonuses belong to this target's
            // attack only. Never install them in persistent Buff state.
            let mut extra = if counter {
                self.units[actor]
                    .riposte
                    .as_ref()
                    .map_or_else(Vec::new, |r| r.buffs.clone())
            } else {
                vec![]
            };
            for e in &s.effects {
                if !e.conditions.is_empty()
                    && !(e.apply_once && once.contains(&e.name))
                    && self.conditions_match(target, e)
                    && self.rng.roll(e.chance.clamp(0., 1.))
                    && !self.reaction_block(
                        actor,
                        actor,
                        dd_data::mechanics::ReactionKind::BlockEffect,
                        Some(&e.name),
                        events,
                    )
                    && let EffectKind::Buffs(buffs) | EffectKind::ProtectedBuffs(buffs) = &e.kind
                {
                    extra.extend(buffs.clone());
                }
            }
            for e in &s.effects {
                if let EffectKind::InstantBuffs(b) = &e.kind {
                    extra.extend(b.clone());
                }
            }
            extra.extend(self.trinket_modifiers(actor, trinket_event));
            let (accuracy, range, critical) = self.attack_values(actor, target, &s, &extra);
            let mut hit = true;
            if let Some(range) = s.heal {
                let raw = self.rng.range(range);
                self.skill_heal(
                    actor,
                    target,
                    raw,
                    s.target.multi,
                    s.tactical.crit_valid,
                    events,
                );
            } else if !s.target.friendly {
                let corpse = !self.units[target].alive();
                attack_attempted |= !corpse;
                let miss_bonus = if self.units[actor].blueprint.hero {
                    f64::from(self.units[actor].miss_streak.saturating_sub(1))
                        * self.scenario.rules.consecutive_miss_reduction_acc_buff
                } else {
                    0.
                };
                hit = corpse
                    || self.rng.roll(
                        if !s.tactical.boss.can_miss
                            || self.units[target]
                                .blueprint
                                .mechanics
                                .boss
                                .restrictions
                                .cannot_miss
                        {
                            1.
                        } else {
                            candidate_hit(
                                accuracy + miss_bonus,
                                self.units[target]
                                    .effective_stats_in(Some(&self.units[actor]), Some(&s))
                                    .dodge,
                                &self.scenario.rules,
                            )
                        },
                    );
                if !hit
                    && self.units[actor].blueprint.hero
                    && self.units[actor]
                        .blueprint
                        .mechanics
                        .build
                        .input
                        .quirks
                        .iter()
                        .any(|q| q == "fated")
                    && self
                        .rng
                        .roll(self.scenario.rules.quirks_fated_miss2hitconvertchance)
                {
                    hit = true;
                    events.push(Event::FatedHit { actor, target });
                }
                attack_hit |= hit && !corpse;
                if !hit {
                    events.push(Event::Miss { actor, target });
                } else {
                    let crit = self.rng.roll(critical);
                    self.units[actor].critical_context = crit && !corpse;
                    any_crit |= crit
                        && !corpse
                        && !self.units[target]
                            .blueprint
                            .mechanics
                            .boss
                            .restrictions
                            .no_crit_effects;
                    let amount = if self.rounding023() {
                        let base = self.damage_base023(actor, target, &s, &extra, Some(crit));
                        self.direct_damage_float023(actor, target, &s, base, events)
                    } else {
                        let base = if crit {
                            let table = if self.units[actor].blueprint.hero {
                                &self.scenario.rules.player_crit_multiplier_table
                            } else {
                                &self.scenario.rules.monster_crit_multiplier_table
                            };
                            (f64::from(range[1]) * table[0].value).round() as i32
                        } else {
                            self.rng.range(range)
                        };
                        self.direct_damage_amount(actor, target, &s, base, events)
                    };
                    let living = self.units[target].alive();
                    let hp_before = self.units[target].hp;
                    self.damage(target, amount, crit, events);
                    if !s.tactical.boss.damage_heal.is_empty() {
                        let actual = (hp_before - self.units[target].hp).max(0);
                        let recipients = self
                            .units
                            .iter()
                            .enumerate()
                            .filter(|(_, u)| {
                                u.alive()
                                    && s.tactical
                                        .boss
                                        .damage_heal
                                        .contains(&u.blueprint.mechanics.family)
                            })
                            .map(|(id, _)| id)
                            .collect::<Vec<_>>();
                        for id in recipients {
                            self.heal(id, actual, events);
                        }
                    }
                    self.reflect_damage(actor, target, amount, events);
                    if crit && !corpse && (!self.units[actor].blueprint.hero || !relief_done) {
                        self.crit_stress(actor, target, events);
                        if self.units[actor].blueprint.hero
                            && !self.units[target]
                                .blueprint
                                .mechanics
                                .boss
                                .restrictions
                                .no_crit_relief
                        {
                            relief_done = true;
                        }
                    }
                    if living
                        && !self.units[target].alive()
                        && self.units[actor].blueprint.hero
                        && !self.units[target]
                            .blueprint
                            .mechanics
                            .boss
                            .restrictions
                            .no_kill_relief
                    {
                        let r = &self.scenario.rules.stress;
                        let (chance, relief) =
                            (r.killing_blow_relief_chance, r.killing_blow_stress_relief);
                        if self.rng.roll(chance) {
                            self.stress_float023(actor, -relief, events);
                        }
                    }
                }
            }
            if hit && s.tactical.boss.teleport && self.outcome.is_none() {
                events.push(Event::Teleported { actor });
                if !self.farm_teleport(actor, events) {
                    self.outcome = Some(Outcome::Teleported);
                }
            }
            for e in s
                .effects
                .iter()
                .filter(|e| !e.kind.is_displacement())
                .chain(s.effects.iter().filter(|e| e.kind.is_displacement()))
            {
                if (hit && !e.on_hit) || (!hit && !e.on_miss) {
                    continue;
                }
                if e.apply_once && !once.insert(e.name.clone()) {
                    continue;
                }
                if !e.conditions.is_empty() || matches!(e.kind, EffectKind::InstantBuffs(_)) {
                    continue;
                }
                if matches!(&e.kind,EffectKind::Lifecycle(l) if l.requires_kill)
                    && (!living_before || self.units[target].alive())
                {
                    continue;
                }
                if e.kind.is_displacement() {
                    self.flush_effects(events);
                }
                self.apply_skill_effect(actor, target, e, events);
            }
            self.flush_effects(events);
            if hit
                && s.tactical.copy_burn_behind
                && copy_burn > 0
                && let Some(behind) = burn_receiver.filter(|id| self.units[*id].present())
            {
                self.units[behind].burn = self.units[behind]
                    .burn
                    .saturating_add(copy_burn)
                    .min(1000000);
            }
            if hit && !s.target.friendly {
                self.boss_hit_reactions(actor, target, living_before, events);
            }
            if hit {
                let ids = self.units[actor]
                    .buffs
                    .iter()
                    .filter(|b| self.units[actor].buff_active(&b.definition))
                    .flat_map(|b| b.definition.activation.attack_additional_effect_ids.clone())
                    .collect::<Vec<_>>();
                for id in ids {
                    if let Some(e) = self.units[actor]
                        .blueprint
                        .mechanics
                        .extra_effects
                        .get(&id)
                        .cloned()
                    {
                        self.apply_skill_effect(actor, target, &e, events);
                    } else {
                        self.lifecycle.failure = Some(format!("missing triggered effect {id}"));
                    }
                }
            }
            if !hit && !s.target.friendly {
                let stress = self.units[actor]
                    .modifier("stress_on_miss", "STRESS_AMOUNT")
                    .clamp(0., 1e6);
                if self.rounding023() {
                    if stress > 0. {
                        self.stress_float023(actor, stress, events);
                    }
                } else {
                    let stress = stress.round() as i32;
                    if stress > 0 {
                        self.stress_change(actor, stress, events);
                    }
                }
            }
            self.fire_trinkets(actor, target, trinket_event, hit, events);
            if !s.target.friendly && living_before {
                self.fire_trinkets(
                    target,
                    actor,
                    if hit { "was_hit" } else { "on_dodge" },
                    hit,
                    events,
                );
                if hit && self.units[actor].critical_context && counter {
                    self.fire_trinkets(actor, target, "riposte_crit", true, events);
                }
                if !self.units[target].alive() {
                    self.fire_trinkets(actor, target, "kill_performer", true, events);
                    if counter {
                        self.fire_trinkets(actor, target, "riposte_kill", true, events);
                    }
                }
            }
            self.flush_effects(events);
            if !s.target.friendly && living_before {
                self.attack_reactions(actor, target, hit);
                if !counter {
                    self.counter_attack(target, actor, hit, &s, events);
                }
            }
        }
        if attack_attempted && self.units[actor].blueprint.hero {
            self.units[actor].miss_streak = if attack_hit {
                0
            } else {
                self.units[actor].miss_streak.saturating_add(1).min(1000000)
            };
        }
        if any_crit {
            let buffs = self.units[actor].blueprint.mechanics.on_crit.clone();
            for b in buffs {
                self.add_buff(actor, b, BuffScope::Timed, events);
            }
        }
        if !s.target.friendly {
            self.end_stealth(actor, StealthEndReason::Attack, events);
        }
        let cooldown = self.units[actor]
            .blueprint
            .mechanics
            .brain
            .as_ref()
            .and_then(|b| b.skill_cooldowns.iter().find(|c| c.combat_skill_id == s.id))
            .map(|c| self.units[actor].turns + c.amount + 1);
        if let Some(until) = cooldown
            && slot.is_some()
            && let Some(slot) = self.units[actor]
                .blueprint
                .skills
                .iter()
                .position(|skill| skill.id == s.id)
        {
            self.units[actor].cooldowns.insert(slot, until);
        }
        if s.move_forward > 0 && self.units[actor].alive() {
            let wanted = self.rank(actor).saturating_sub(s.move_forward as u8).max(1);
            let rank = self.displacement_rank(actor, wanted);
            self.forced_move(actor, rank, events);
        }
        if s.move_back > 0 && self.units[actor].alive() {
            let wanted = self.rank(actor).saturating_add(s.move_back as u8);
            let rank = self.displacement_rank(actor, wanted);
            self.forced_move(actor, rank, events);
        }
        let consumed = self.units[actor]
            .buffs
            .iter()
            .filter(|b| {
                b.definition.activation.consume_on_rule_active
                    && self.units[actor].buff_active(&b.definition)
            })
            .map(|b| b.definition.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        self.units[actor]
            .buffs
            .retain(|b| !consumed.contains(&b.definition.id));
        self.end_item_root(previous_item_root);
        self.units[actor].critical_context = prior_critical;
        self.units[actor].counter_context = prior_counter;
        self.units[actor].attack_context = prior_type;
        self.cleanup_tactics(events);
        self.refresh_context(true);
    }
    fn move_to(&mut self, actor: UnitId, rank: u8) {
        debug_assert!(self.placement_ranks(actor).contains(&rank));
        let side = usize::from(!self.units[actor].friendly());
        self.order[side].retain(|id| self.units[*id].present() && *id != actor);
        let mut start = 1;
        let index = self.order[side]
            .iter()
            .position(|id| {
                let boundary = start;
                start += self.units[*id].size();
                boundary == rank
            })
            .unwrap_or(self.order[side].len());
        self.order[side].insert(index, actor);
        self.refresh_context(true);
    }
    fn refresh_rank_history(&mut self) {
        for id in 0..self.units.len() {
            let rank = self.rank(id);
            if self.units[id].turn_rank != rank {
                self.units[id].turn_rank = rank;
                self.units[id].rank_turns = 0;
            }
        }
    }
    fn damage(&mut self, target: UnitId, amount: i32, crit: bool, events: &mut Vec<Event>) {
        if self.units[target]
            .blueprint
            .mechanics
            .boss
            .restrictions
            .no_direct_damage
        {
            return;
        }
        self.damage_unrestricted(target, amount, crit, events);
    }
    fn damage_unrestricted(
        &mut self,
        target: UnitId,
        amount: i32,
        crit: bool,
        events: &mut Vec<Event>,
    ) {
        if amount <= 0
            || (!self.units[target].present()
                && !self.units[target]
                    .captive
                    .as_ref()
                    .is_some_and(|c| c.removed && self.units[target].alive()))
        {
            return;
        }
        for u in &mut self.units {
            if let Some(c) = &mut u.captive
                && c.captor == target
            {
                c.damage = c.damage.saturating_add(amount).min(1000000);
            }
        }
        if self.shared_damage(target, amount, crit, events) {
            return;
        }
        let was_alive = self.units[target].alive();
        self.lifecycle.special.last_damaged = Some(target);
        let prior_rank = self.rank(target);
        events.push(Event::Damage {
            target,
            amount,
            crit,
        });
        let hero = self.units[target].blueprint.hero;
        if hero {
            if self.units[target].hp == 0 && self.is_required_dream_hero(target) {
                return;
            }
            if self.units[target].hp == 0 {
                if !self.rng.roll(
                    1. - self.scenario.rules.deathblow_chance
                        * (1.
                            - self.units[target].stats().death_resist.clamp(
                                0.,
                                self.scenario.rules.candidates.limits.death_blow_resistance,
                            )),
                ) {
                    self.units[target].life = Life::Dead;
                    events.push(Event::Death(target));
                    self.death_stress(target, events);
                    self.hero_death(target, events);
                }
            } else {
                self.units[target].hp = (self.units[target].hp - amount).max(0);
                if self.units[target].hp == 0 {
                    events.push(Event::DeathsDoor(target));
                    let buffs = self.units[target].blueprint.mechanics.deaths_door.clone();
                    for b in buffs {
                        self.add_buff(target, b, BuffScope::DeathsDoor, events);
                    }
                    self.door_stress(target, events);
                    self.hero_door(target, events);
                }
            }
        } else {
            let u = &mut self.units[target];
            u.phase_overkill = (amount - u.hp).max(0);
            u.hp = (u.hp - amount).max(0);
            if u.hp == 0 {
                let corpse = if u.alive() {
                    u.blueprint.corpse.as_ref().filter(|c| {
                        (!crit || c.on_crit)
                            && self
                                .scenario
                                .rules
                                .context
                                .as_ref()
                                .is_none_or(|c| c.corpses)
                    })
                } else {
                    None
                };
                if let Some(c) = corpse {
                    u.hp = c.stats.hp;
                    u.life = Life::Corpse {
                        expires: self.round + c.rounds,
                    };
                    u.dots.clear();
                    u.buffs.clear();
                    u.stunned = false;
                    events.push(Event::Corpse(target));
                } else {
                    u.life = Life::Dead;
                    events.push(Event::Death(target));
                }
            }
        }
        if !self.units[target].alive() {
            self.units[target].parasites.clear();
            self.end_stealth(target, StealthEndReason::Death, events);
        }
        if was_alive && !self.units[target].alive() {
            if !self.units[target].blueprint.hero
                && self.units[target]
                    .blueprint
                    .mechanics
                    .lifecycle
                    .death_change
                    .is_none()
            {
                self.lifecycle.special.killed.insert(
                    target,
                    (
                        self.units[target].source_id.clone(),
                        self.units[target].blueprint.id.clone(),
                    ),
                );
            }
            self.units[target].death_rank = prior_rank;
            self.schedule_death(target, crit, events);
        }
        self.cleanup_tactics(events);
        self.refresh_context(true);
    }
    fn terminal(&mut self) -> bool {
        // Reactions during opportunity preparation may retire another queued actor.
        self.queue.retain(|id| self.units[*id].active());
        if self.outcome.is_some() {
            self.lifecycle.new_wave = false;
            return true;
        }
        if !self.lifecycle.jobs.is_empty() {
            return false;
        }
        if self.lifecycle.wave < self.lifecycle.waves.len()
            && self.units.iter().any(|u| u.alive() && u.blueprint.hero)
            && !self.units.iter().any(|u| u.alive() && !u.blueprint.hero)
        {
            return false;
        }
        if self
            .lifecycle
            .special
            .farm
            .as_ref()
            .is_some_and(|s| !s.complete && s.interlude.is_none())
            && self.units.iter().any(|u| u.alive() && u.blueprint.hero)
        {
            return false;
        }
        if !self.units.iter().any(|u| u.alive()) {
            self.outcome = Some(Outcome::MutualDefeat);
        } else if !self.units.iter().any(|u| u.alive() && !u.blueprint.hero) {
            self.outcome = Some(Outcome::HeroesWin);
        } else if !self.units.iter().any(|u| u.alive() && u.blueprint.hero) {
            self.outcome = Some(Outcome::MonstersWin);
        }
        if self.outcome.is_some() {
            self.lifecycle.new_wave = false;
        }
        self.outcome.is_some()
    }
    fn advance(&mut self, events: &mut Vec<Event>) {
        loop {
            self.flush_lifecycle(events);
            self.next_wave(events);
            self.cleanup_tactics(events);
            self.refresh_context(true);
            if let Some(s) = &self.lifecycle.special.selection {
                self.actor = Some(s.actor);
                return;
            }
            if self.terminal() {
                self.actor = None;
                return;
            }
            if self.queue.is_empty() {
                let new_wave = std::mem::take(&mut self.lifecycle.new_wave);
                if self.round > 0 && !new_wave {
                    self.stall_finish(events);
                    self.trinket_round_end(events);
                    self.run_bonus_phase(None, false, events);
                    self.flush_lifecycle(events);
                    if let Err(e) = self.farm_round_end(events) {
                        self.lifecycle.failure = Some(e);
                        return;
                    }
                    self.flush_lifecycle(events);
                    if self.next_wave(events) {
                        continue;
                    }
                    if self.terminal() {
                        self.actor = None;
                        return;
                    }
                }
                if self.round >= self.scenario.round_limit {
                    self.outcome = Some(Outcome::RoundLimit);
                    self.actor = None;
                    return;
                }
                self.round += 1;
                self.lifecycle.zones.retain(|z| z.expires > self.round);
                self.lifecycle.bonus_used.clear();
                self.prepare_round();
                self.refresh_context(true);
                events.push(Event::Round(self.round));
                for (id, u) in self.units.iter_mut().enumerate() {
                    if matches!(u.life,Life::Corpse { expires } if expires<=self.round) {
                        u.life = Life::Dead;
                        u.hp = 0;
                        events.push(Event::Death(id));
                    }
                }
                let expired = self
                    .units
                    .iter()
                    .enumerate()
                    .filter_map(|(id, u)| {
                        (u.present() && u.expires.is_some_and(|n| n <= self.round)).then_some(id)
                    })
                    .collect::<Vec<_>>();
                for id in expired {
                    if self.units[id]
                        .blueprint
                        .mechanics
                        .boss
                        .tags
                        .iter()
                        .any(|t| t == "boss")
                    {
                        self.lifecycle
                            .special
                            .escaped
                            .insert(id, self.units[id].source_id.clone());
                        events.push(Event::Escaped {
                            id,
                            class: self.units[id].source_id.clone(),
                        });
                    }
                    self.retire(id, events);
                }
                self.refresh_context(true);
                self.shape_shift_round(events);
                self.roll_round();
                self.run_bonus_phase(None, true, events);
                self.flush_lifecycle(events);
                if self.lifecycle.special.selection.is_some() {
                    continue;
                }
                if self.queue.is_empty() {
                    continue;
                }
            }
            let id = self.queue.pop_front().unwrap();
            if !self.units[id].active() {
                continue;
            }
            self.hero_opportunity(id, events);
            self.units[id].turns += 1;
            self.units[id].round_actions += 1;
            self.units[id].turn_uses.clear();
            self.units[id].item_blocked.clear();
            self.units[id].turn_history.clear();
            let rank = self.rank(id);
            if self.units[id].turn_rank == rank {
                self.units[id].rank_turns += 1;
            } else {
                self.units[id].turn_rank = rank;
                self.units[id].rank_turns = 1;
            }
            if self.units[id].shuffle_remaining > 0 {
                self.units[id].shuffle_remaining -= 1;
                self.displace(id, &EffectKind::Shuffle, events);
            }
            self.flashback_tick(id, events);
            let mut dots = std::mem::take(&mut self.units[id].dots);
            dots.sort_by_key(|d| !matches!(d.kind, EffectKind::Regen(_)));
            for mut dot in dots {
                if !self.units[id].alive() {
                    break;
                }
                if matches!(dot.kind, EffectKind::Regen(_)) {
                    self.restoration_tick(id, dot.amount, dot.can_crit_heal, events);
                } else if matches!(dot.kind, EffectKind::Horror(_)) {
                    self.stress_change(id, dot.amount, events);
                } else {
                    self.damage(id, dot.amount, false, events);
                }
                dot.remaining -= 1;
                if dot.remaining > 0 && self.units[id].alive() {
                    self.units[id].dots.push(dot);
                }
            }
            if !self.units[id].alive() {
                continue;
            }
            if self.terminal() {
                self.actor = None;
                return;
            }
            self.run_bonus_phase(Some(id), true, events);
            self.flush_lifecycle(events);
            if !self.units[id].alive() {
                continue;
            }
            if self.units[id].stunned {
                self.units[id].stunned = false;
                events.push(Event::StunSkip(id));
                let buffs = self
                    .scenario
                    .rules
                    .globals
                    .as_ref()
                    .unwrap()
                    .stun_recovery
                    .clone();
                for buff in buffs {
                    self.add_buff(id, buff, BuffScope::Timed, events);
                }
                self.complete_turn(id, events);
                self.flush_lifecycle(events);
                self.run_bonus_phase(Some(id), false, events);
                self.flush_reactions(events);
                continue;
            }
            self.actor = Some(id);
            if self.quirk_actout023(id, events) || self.start_turn_actout(id, events) {
                self.actor = None;
                self.complete_turn(id, events);
                self.flush_lifecycle(events);
                self.run_bonus_phase(Some(id), false, events);
                self.flush_reactions(events);
                continue;
            }
            self.flush_reactions(events);
            if self.terminal() {
                self.actor = None;
                return;
            }
            if !self.units[id].alive() {
                self.actor = None;
                continue;
            }
            if self.units[id].stunned {
                self.actor = None;
                self.settle_opportunity(id, events);
                continue;
            }
            return;
        }
    }
    /// Host-only persistence, including hidden RNG/initiative. Never hand this to a policy.
    pub fn save_json(&self) -> Result<String> {
        self.validate_supply_reserves()?;
        self.validate_environment_supplies()?;
        self.validate_trinkets()?;
        self.validate_shared_health()?;
        self.validate_captives()?;
        self.validate_control()?;
        self.validate_flashback()?;
        self.validate_farm()?;
        if self.lifecycle.skill_effect {
            return Err("unsettled skill effect provenance".into());
        }
        if !self.effect_jobs.is_empty()
            || !self.pending_reactions.is_empty()
            || !self.lifecycle.jobs.is_empty()
            || self.lifecycle.failure.is_some()
        {
            return Err("cannot save an unsettled reaction queue".into());
        }
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
    pub fn load_json(json: &str) -> Result<Self> {
        Self::load_json_with_content(json, &dd_data::content::ContentConfig::fixture())
    }
    /// Host explicitly supplies the expected configuration. Persistence never
    /// consults a mutable process-wide environment variable or a game directory.
    pub fn load_json_with_content(
        json: &str,
        expected: &dd_data::content::ContentConfig,
    ) -> Result<Self> {
        Self::load_json_with_candidates(
            json,
            expected,
            &dd_data::candidate012::CombatCandidates::documented(),
        )
    }
    pub fn load_json_with_candidates(
        json: &str,
        expected: &dd_data::content::ContentConfig,
        candidates: &dd_data::candidate012::CombatCandidates,
    ) -> Result<Self> {
        let expected_id = expected.id()?;
        let expected_candidates = candidates.fingerprint()?;
        // Reject old schemas by profile before decoding newly required fields.
        #[derive(Deserialize)]
        struct ProfileHeader {
            profile: String,
        }
        let header: ProfileHeader = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if header.profile != PROFILE {
            return Err("unsupported checkpoint profile".into());
        }
        let saved: SavedBattle = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if saved.scenario.rules.candidates.fingerprint()? != expected_candidates {
            return Err("checkpoint candidate rule configuration mismatch".into());
        }
        if saved.content_id != expected_id || saved.scenario.rules.content.id()? != expected_id {
            return Err("checkpoint content configuration mismatch".into());
        }
        let b = Self {
            lifecycle: saved.lifecycle,
            profile: saved.profile,
            content_id: saved.content_id,
            scenario: saved.scenario,
            supplies: saved.supplies,
            supply_reserves: saved.supply_reserves,
            units: saved.units,
            order: saved.order,
            queue: saved.queue,
            actor: saved.actor,
            round: saved.round,
            rng: saved.rng,
            outcome: saved.outcome,
            effect_jobs: VecDeque::new(),
            pending_reactions: saved.pending_reactions,
            retreat_attempts: saved.retreat_attempts,
        };
        if b.profile != PROFILE {
            return Err("unsupported checkpoint profile".into());
        }
        validate_scenario(&b.scenario)?;
        b.validate_catalog()?;
        b.validate_trinkets()?;
        b.validate_environment_supplies()?;
        b.validate_shared_health()?;
        b.validate_captives()?;
        b.validate_control()?;
        b.validate_flashback()?;
        b.validate_farm()?;
        if b.lifecycle.skill_effect {
            return Err("unsettled checkpoint skill provenance".into());
        }
        if b.lifecycle.zones.len() > 8
            || b.lifecycle.zones.iter().any(|z| {
                !(1..=4).contains(&z.rank)
                    || !(1..=10000).contains(&z.amount)
                    || z.expires <= b.round
                    || z.expires > b.round + 10000
            })
        {
            return Err("invalid checkpoint rank burn".into());
        }
        validate_supplies(&b.supplies)?;
        b.validate_supply_reserves()?;
        if !b.pending_reactions.is_empty()
            || u64::from(b.retreat_attempts) > u64::from(b.scenario.round_limit) * 4
        {
            return Err("checkpoint is not a settled action boundary".into());
        }
        let original: Vec<_> = b
            .scenario
            .heroes
            .iter()
            .chain(&b.scenario.monsters)
            .collect();
        if b.units.len() < original.len()
            || b.round == 0
            || b.round > b.scenario.round_limit
            || !b.lifecycle.jobs.is_empty()
            || b.lifecycle.new_wave
            || b.lifecycle.failure.is_some()
        {
            return Err("invalid checkpoint size/round".into());
        }
        let mut ids = vec![];
        for (side, order) in b.order.iter().enumerate() {
            for &id in order {
                if id >= b.units.len()
                    || usize::from(!b.units[id].friendly()) != side
                    || ids.contains(&id)
                {
                    return Err("invalid checkpoint formation".into());
                }
                ids.push(id);
            }
            if order
                .iter()
                .map(|id| usize::from(b.units[*id].size()))
                .sum::<usize>()
                > 4
            {
                return Err("invalid checkpoint formation capacity".into());
            }
        }
        let mut queued = vec![];
        for u in &b.units {
            if u.miss_streak > 1000000
                || u.item_uses.len() > 1000
                || u.item_uses
                    .iter()
                    .any(|(id, n)| id.is_empty() || *n > 1000000)
                || u.dots
                    .iter()
                    .any(|d| d.can_crit_heal && !matches!(d.kind, EffectKind::Regen(_)))
            {
                return Err("invalid checkpoint miss/supply/status history".into());
            }
        }
        for &id in &b.queue {
            if id >= b.units.len()
                || !b.units[id].active()
                || b.queue.iter().filter(|x| **x == id).count() > 8
            {
                return Err("invalid checkpoint queue".into());
            }
            queued.push(id);
        }
        if let Some(id) = b.actor {
            if id >= b.units.len()
                || !b.units[id].active()
                || b.units[id].stunned
                || b.outcome.is_some()
            {
                return Err("invalid checkpoint actor".into());
            }
        } else if b.outcome.is_none() {
            return Err("missing checkpoint actor".into());
        }
        for (id, u) in b.units.iter().enumerate() {
            if u.buffs.iter().any(|b| {
                b.battle_persistent && (b.remaining.is_some() || b.scope != BuffScope::Timed)
            }) {
                return Err("invalid checkpoint permanent Buff scope".into());
            }
            if u.shuffle_remaining > 10000 || !(0..=1000000).contains(&u.phase_overkill) {
                return Err("invalid checkpoint shuffle/overkill clock".into());
            }
            let expected_cache = u
                .blueprint
                .mechanics
                .brain
                .as_ref()
                .map(|brain| vec![true; brain.skill_selection_desires.len()])
                .unwrap_or_default();
            if (u.alive() && u.desire_rolls != expected_cache)
                || (!u.alive() && !u.desire_rolls.is_empty() && u.desire_rolls != expected_cache)
            {
                return Err("invalid checkpoint desire cache".into());
            }
            if let Some((class, hp)) = &u.previous_form
                && (!u.blueprint.mechanics.boss.death.previous_hp
                    || !b.lifecycle.catalog.contains_key(class)
                    || !(1..=1000000).contains(hp))
            {
                return Err("invalid checkpoint previous form".into());
            }
            let mut expected_aura = b
                .units
                .iter()
                .enumerate()
                .filter(|(_, source)| source.alive())
                .flat_map(|(source_id, source)| {
                    if source.blueprint.hero != u.blueprint.hero {
                        source.blueprint.mechanics.lifecycle.aura_heroes.clone()
                    } else if source_id != id {
                        source.blueprint.mechanics.lifecycle.aura_allies.clone()
                    } else {
                        vec![]
                    }
                })
                .collect::<Vec<_>>();
            expected_aura.extend(b.companion_modifiers(id));
            let mut expected_counts = BTreeMap::<String, u32>::new();
            for source in b.units.iter().filter(|v| v.alive() && !v.blueprint.hero) {
                for tag in &source.blueprint.mechanics.enemy_types {
                    *expected_counts.entry(tag.clone()).or_default() += 1;
                }
            }
            let mut expected_quest = b
                .scenario
                .rules
                .context
                .as_ref()
                .map(|c| c.quest_uses_remaining.clone())
                .unwrap_or_default();
            for t in &u.trinkets {
                if !t.destroyed
                    && let Some(n) = t.quest_remaining
                {
                    expected_quest.insert(t.definition.id.clone(), n);
                }
            }
            if u.equipment_modifiers != crate::trinkets010::equipment(u)
                || u.quest_uses_context != expected_quest
                || u.item_blocked.iter().any(|i| *i >= b.supplies.len())
            {
                return Err("invalid checkpoint equipment/context".into());
            }
            if u.corridor_context
                != b.scenario
                    .rules
                    .context
                    .as_ref()
                    .is_some_and(|c| c.corridor)
                || u.walking_backwards_context
                    != b.scenario
                        .rules
                        .context
                        .as_ref()
                        .is_some_and(|c| c.walking_backwards)
            {
                return Err("invalid checkpoint encounter location".into());
            }
            if u.aura_modifiers != expected_aura
                || u.monster_counts != expected_counts
                || u.region_context != b.scenario.rules.context.as_ref().map(|c| c.region.clone())
                || u.light_context != b.scenario.rules.lighting.as_ref().map(|l| l.torch)
                || u.items_context
                    != b.supplies
                        .iter()
                        .filter(|s| s.remaining > 0)
                        .map(|s| s.definition.id.clone())
                        .collect::<Vec<_>>()
            {
                return Err("invalid checkpoint environment context".into());
            }
            if let Some(ctx) = &b.scenario.rules.context {
                let mut slots = 0;
                for disease in &u.diseases {
                    let def = ctx
                        .diseases
                        .get(disease)
                        .ok_or("unknown checkpoint disease")?;
                    slots += def.slots;
                    if u.disease_tags.get(disease) != Some(&def.tags) {
                        return Err("invalid checkpoint disease tags".into());
                    }
                    if def.evolution.is_some()
                        && u.alive()
                        && u.disease_turns
                            .get(disease)
                            .is_none_or(|n| *n == 0 || *n > 10000)
                    {
                        return Err("invalid checkpoint disease counter".into());
                    }
                }
                if slots > ctx.disease_max
                    || u.disease_tags.len() != u.diseases.len()
                    || u.disease_turns.iter().any(|(source, n)| {
                        *n == 0
                            || !u.diseases.contains(source)
                            || ctx
                                .diseases
                                .get(source)
                                .is_none_or(|d| d.evolution.is_none())
                    })
                {
                    return Err("invalid checkpoint disease state".into());
                }
                for buff in u.buffs.iter().filter(|b| b.source_disease.is_some()) {
                    let source = buff.source_disease.as_ref().unwrap();
                    if !u.diseases.contains(source)
                        || !ctx
                            .diseases
                            .get(source)
                            .is_some_and(|d| d.buffs.contains(&buff.definition))
                    {
                        return Err("invalid checkpoint disease Buff source".into());
                    }
                }
            } else if !u.diseases.is_empty()
                || !u.disease_turns.is_empty()
                || !u.disease_tags.is_empty()
            {
                return Err("missing checkpoint disease context".into());
            }
            if u.resolve_history.iter().any(|(name, n)| {
                *n > 1000000
                    || !b
                        .scenario
                        .rules
                        .globals
                        .as_ref()
                        .unwrap()
                        .traits
                        .iter()
                        .chain(&u.blueprint.mechanics.hero.override_defs)
                        .any(|t| &t.id == name)
            }) {
                return Err("invalid checkpoint resolve history".into());
            }
            if u.hp_reaction_active.len() != u.blueprint.mechanics.hero.low_hp.len()
                || u.mode.as_ref().is_some_and(|id| {
                    !u.blueprint.mechanics.hero.modes.iter().any(|m| &m.id == id)
                        && !u.blueprint.mechanics.boss.modes.iter().any(|m| &m.id == id)
                })
                || ((u.blueprint.mechanics.hero.modes.is_empty()
                    && u.blueprint.mechanics.boss.modes.is_empty())
                    != u.mode.is_none())
                || u.door_reaction_round.is_some_and(|r| r > b.round)
                || u.diseases
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != u.diseases.len()
            {
                return Err("invalid checkpoint hero state".into());
            }
            if u.light_modifiers
                != b.scenario
                    .rules
                    .lighting
                    .as_ref()
                    .map(|l| l.buffs(u.blueprint.hero))
                    .transpose()?
                    .unwrap_or_default()
                || u.critical_context
                || u.counter_context
                || (i16::from(u.blueprint.mechanics.lifecycle.turns) + i16::from(u.turn_adjust)) < 0
                || (i16::from(u.blueprint.mechanics.lifecycle.turns) + i16::from(u.turn_adjust)) > 4
                || u.next_bonus > 4
                || u.round_actions > u32::from(u.round_budget)
                || u.round_budget > 8
                || u.round_extra > 4
                || b.queue.iter().filter(|x| **x == id).count()
                    > usize::from(u.round_budget).saturating_sub(u.round_actions as usize)
                || u.turn_uses.iter().any(|(index, n)| {
                    *index >= u.blueprint.skills.len()
                        || *n == 0
                        || u.blueprint.skills[*index]
                            .tactical
                            .per_turn_limit
                            .is_some_and(|limit| *n > limit)
                })
                || u.expires
                    .is_some_and(|n| u.present() && n <= b.round || n > b.round + 10000)
                || u.attack_context.is_some()
                || u.rank_context != b.rank(id)
                || u.round_context != b.round
                || u.turn_rank > 4
                || u.death_rank > 4
                || u.rank_turns > u.turns
                || !(0..=1000000).contains(&u.burn)
                || (!u.present() && u.captive.is_none() && u.burn != 0)
                || u.shuffle_remaining > 10000
                || u.phase_overkill < 0
                || u.phase_overkill > 1000000
                || u.blocks > 10000
                || (!u.alive() && (u.riposte.is_some() || u.guard.is_some() || u.blocks != 0))
                || u.guarded_context != b.guarder(id).is_some()
                || u.any_burning_context != b.units.iter().any(|v| v.present() && v.burn > 0)
            {
                return Err("invalid checkpoint tactical context".into());
            }
            if let Some(g) = &u.guard
                && (g.ward == id
                    || g.remaining == 0
                    || g.remaining > 10000
                    || g.applied_turn > u.turns
                    || u.stunned
                    || !b
                        .units
                        .get(g.ward)
                        .is_some_and(|ward| ward.alive() && ward.friendly() == u.friendly())
                    || b.units.iter().enumerate().any(|(other, v)| {
                        other != id
                            && v.guard.as_ref().is_some_and(|h| {
                                h.ward == g.ward || h.ward == id || g.ward == other
                            })
                    }))
            {
                return Err("invalid checkpoint guard relationship".into());
            }
            if let Some(r) = &u.riposte {
                if r.remaining == 0
                    || r.remaining > 10000
                    || r.applied_turn > u.turns
                    || !r.hit_chance.is_finite()
                    || !r.miss_chance.is_finite()
                    || !(0. ..=1.).contains(&r.hit_chance)
                    || !(0. ..=1.).contains(&r.miss_chance)
                    || u.blueprint.mechanics.riposte.is_none()
                {
                    return Err("invalid checkpoint counter state".into());
                }
                for buff in &r.buffs {
                    buff.validate()?;
                }
            }
            if u.blocks_timer.is_some_and(|(remaining, applied)| {
                remaining == 0 || remaining > 10000 || applied > u.turns || u.blocks == 0
            }) {
                return Err("invalid checkpoint block clock".into());
            }
            if u.skill_remaining.len() != u.blueprint.skills.len()
                || u.turn_history.values().any(|n| *n == 0 || *n > 10000)
                || u.blueprint.skills.iter().enumerate().any(|(index, s)| {
                    u.turn_uses.get(&index).copied().unwrap_or(0)
                        != u.turn_history.get(&s.id).copied().unwrap_or(0)
                })
                || u.used_skills.values().any(|n| *n == 0 || *n > 10000)
                || u.blueprint
                    .skills
                    .iter()
                    .zip(&u.skill_remaining)
                    .any(|(s, n)| {
                        s.per_battle_limit.map(|limit| {
                            limit.saturating_sub(u.used_skills.get(&s.id).copied().unwrap_or(0))
                        }) != *n
                    })
                || u.skill_remaining
                    .iter()
                    .zip(&u.blueprint.skills)
                    .any(|(n, s)| match (n, s.per_battle_limit) {
                        (Some(n), Some(limit)) => *n > limit,
                        (None, None) => false,
                        _ => true,
                    })
            {
                return Err("invalid checkpoint skill uses".into());
            }
            if u.resolve_epoch
                > b.scenario
                    .round_limit
                    .saturating_mul(8)
                    .saturating_mul(1024)
            {
                return Err("invalid checkpoint trait generation".into());
            }
            if u.turns > b.round.saturating_mul(8) + 1
                || !(0..200).contains(&u.stress)
                || (!u.blueprint.hero && (u.stress != 0 || u.resolve != Resolve::Calm))
            {
                return Err("invalid checkpoint stress/turn count".into());
            }
            if let Resolve::Afflicted(name) | Resolve::Virtuous(name) = &u.resolve {
                let virtue = matches!(u.resolve, Resolve::Virtuous(_));
                if !b
                    .scenario
                    .rules
                    .globals
                    .as_ref()
                    .unwrap()
                    .traits
                    .iter()
                    .chain(&u.blueprint.mechanics.hero.override_defs)
                    .any(|t| t.id == *name && t.virtue == virtue)
                {
                    return Err("unknown checkpoint resolve trait".into());
                }
            }
            let mut host_instances = std::collections::BTreeSet::new();
            for buff in &u.buffs {
                buff.definition.validate()?;
                if let Some(instance) = buff.host_instance
                    && (!host_instances.insert(instance)
                        || buff.scope != BuffScope::Timed
                        || !u
                            .blueprint
                            .mechanics
                            .hero
                            .host_buffs
                            .iter()
                            .any(|b| b.instance == instance && b.definition == buff.definition))
                {
                    return Err("invalid checkpoint host Buff ownership/source".into());
                }
                if !u.alive() && buff.definition.stat_type == "stealth" {
                    return Err("invalid checkpoint stealth on nonliving unit".into());
                }
                if buff.applied_turn > u.turns
                    || buff.remaining.is_some_and(|n| n == 0 || n > 10000)
                {
                    return Err("invalid checkpoint Buff timer".into());
                }
            }
            if u.cooldowns.iter().any(|(skill, until)| {
                *skill >= u.blueprint.skills.len() || *until > u.turns + 10001
            }) || u.marked.is_some_and(|until| until > u.turns + 10000)
            {
                return Err("invalid checkpoint cooldown/mark".into());
            }
            if (original.get(id).copied() != Some(&u.blueprint)
                && b.lifecycle.catalog.get(&u.blueprint.id) != Some(&u.blueprint))
                || (u.present() && !ids.contains(&id))
                || u.hp < 0
                || (matches!(u.life, Life::Corpse { .. })
                    && (u.blueprint.hero
                        || u.blueprint.corpse.is_none()
                            && u.blueprint.mechanics.lifecycle.role
                                != dd_data::lifecycle::EntityRole::Corpse))
            {
                return Err("invalid checkpoint unit".into());
            }
            if u.hp > u.stats().hp
                || (u.present() && !u.blueprint.hero && u.hp == 0)
                || matches!(u.life, Life::Corpse { expires } if expires <= b.round)
                || u.dots.iter().any(|d| {
                    d.remaining == 0 || d.remaining > 10000
                        || (!u.blueprint.hero && matches!(d.kind, EffectKind::Horror(_)))
                        || !matches!(d.kind, EffectKind::Bleed(n) | EffectKind::Blight(n) | EffectKind::Horror(n) | EffectKind::Regen(n) if n == d.amount && (0..=1000000).contains(&n))
                })
            {
                return Err("invalid checkpoint health/effect".into());
            }
        }
        let heroes = b.units.iter().any(|u| u.alive() && u.blueprint.hero);
        let monsters = b.units.iter().any(|u| u.alive() && !u.blueprint.hero);
        if b.lifecycle.bonus_used.iter().any(|(id, index)| {
            b.units
                .get(*id)
                .and_then(|u| u.blueprint.mechanics.brain.as_ref())
                .is_none_or(|brain| *index >= brain.bonus_initiative_desires.len())
        }) {
            return Err("invalid bonus scheduling marker".into());
        }
        match b.outcome {
            Some(Outcome::HeroesWin) if !heroes || monsters => {
                return Err("invalid victory checkpoint".into());
            }
            Some(Outcome::MonstersWin) if heroes || !monsters => {
                return Err("invalid defeat checkpoint".into());
            }
            Some(Outcome::RoundLimit)
                if !heroes
                    || (!monsters && b.scenario.rules.farm.is_none())
                    || b.round != b.scenario.round_limit
                    || !b.queue.is_empty() =>
            {
                return Err("invalid round-limit checkpoint".into());
            }
            Some(Outcome::MutualDefeat) if heroes || monsters => {
                return Err("invalid simultaneous defeat checkpoint".into());
            }
            Some(Outcome::Retreated) if !heroes || !monsters => {
                return Err("invalid retreat checkpoint".into());
            }
            None if !heroes
                || (!monsters
                    && !b
                        .lifecycle
                        .special
                        .farm
                        .as_ref()
                        .is_some_and(|s| !s.complete && s.interlude.is_none())) =>
            {
                return Err("missing checkpoint outcome".into());
            }
            _ => {}
        }
        Ok(b)
    }
}

/// Candidate: displayed ACC-DODGE, clamped at game cutoffs, plus hidden
/// (1-hit_max) compensation. Consecutive-miss compensation is NOT implemented.
pub fn candidate_hit(accuracy: f64, dodge: f64, rules: &Rules) -> f64 {
    ((accuracy - dodge).clamp(rules.hit_min, rules.hit_max) + (1. - rules.hit_max)).clamp(0., 1.)
}
pub fn candidate_damage(base: i32, protection: f64) -> i32 {
    if base == 0 {
        0
    } else {
        (f64::from(base) * (1. - protection.clamp(0., 1.)))
            .round()
            .max(1.) as i32
    }
}
fn validate_effect(effect: &dd_data::combat::Effect) -> Result<()> {
    validate_effect_depth(effect, 0)
}
fn validate_boss_effect(op: &dd_data::special::BossOp, depth: u8) -> Result<()> {
    if depth > 16 {
        return Err("boss effect nesting limit".into());
    }
    if let dd_data::special::BossOp::ActorDot(d) = op {
        for stage in &d.stages {
            for e in stage.completion.iter().chain(&stage.increment) {
                validate_effect_depth(e, depth + 1)?;
            }
        }
    }
    Ok(())
}
fn validate_effect_depth(effect: &dd_data::combat::Effect, depth: u8) -> Result<()> {
    if effect
        .execution
        .source_heal_type
        .as_ref()
        .is_some_and(|s| !["hero_skill", "monster_skill", "curio"].contains(&s.as_str()))
    {
        return Err("invalid checkpoint healing source".into());
    }
    if let EffectKind::BattlePersistent(kind) = &effect.kind {
        if !matches!(
            kind.as_ref(),
            EffectKind::Buffs(_) | EffectKind::Riposte { .. }
        ) {
            return Err("invalid permanent effect operation".into());
        }
        let mut inner = effect.clone();
        inner.kind = *kind.clone();
        validate_effect_depth(&inner, depth + 1)?;
    }
    if let EffectKind::InventoryEffect(kind) = &effect.kind {
        let mut inner = effect.clone();
        inner.kind = *kind.clone();
        validate_effect_depth(&inner, depth + 1)?;
    }
    if let EffectKind::TrinketOp(op) = &effect.kind {
        use dd_data::trinket010::TrinketOp;
        match op {
            TrinketOp::Gain(id) if id.is_empty() => return Err("invalid gained item".into()),
            TrinketOp::Increase { min, max } if min > max || *max > 1000000 => {
                return Err("invalid trigger increase".into());
            }
            _ => {}
        }
    }
    if let EffectKind::InstantBuffs(_) = &effect.kind
        && (effect.chance != 1. || effect.target != EffectTarget::Performer)
    {
        return Err("unsupported instant Buff filter".into());
    }
    if let EffectKind::Boss(op) = &effect.kind {
        op.validate()?;
        validate_boss_effect(op, depth)?;
    }
    if let EffectKind::Lifecycle(l) = &effect.kind {
        l.op.validate()?;
        if let dd_data::lifecycle::LifeOp::Boss(op) = &l.op {
            validate_boss_effect(op, depth)?;
        }
        if let dd_data::lifecycle::LifeOp::Phase { effects, .. } = &l.op {
            for e in effects {
                validate_effect_depth(e, depth + 1)?;
            }
        }
    }
    if depth > 16 {
        return Err("effect nesting limit".into());
    }
    if let EffectKind::Composite(ops) = &effect.kind {
        if ops.is_empty() || ops.len() > 64 {
            return Err("invalid composite operation count".into());
        }
        for op in ops {
            if op.target != effect.target
                || op.on_hit != effect.on_hit
                || op.on_miss != effect.on_miss
                || op.apply_once != effect.apply_once
            {
                return Err("inconsistent composite filters".into());
            }
            validate_effect_depth(op, depth + 1)?;
        }
    }
    for c in &effect.conditions {
        c.validate()?;
    }
    if !effect.conditions.is_empty()
        && (effect.target != EffectTarget::Performer
            || !matches!(
                effect.kind,
                EffectKind::Buffs(_) | EffectKind::ProtectedBuffs(_)
            )
            || !effect.on_hit
            || effect.on_miss)
    {
        return Err("unsupported conditional effect shape".into());
    }
    if let EffectKind::Buffs(buffs)
    | EffectKind::ProtectedBuffs(buffs)
    | EffectKind::InstantBuffs(buffs) = &effect.kind
    {
        if buffs.is_empty() {
            return Err("empty Buff effect".into());
        }
        for b in buffs {
            b.validate()?;
        }
    }
    match &effect.kind {
        EffectKind::Burn(n) | EffectKind::Regen(n) | EffectKind::HealthDamage(n)
            if !(0..=1000000).contains(n) =>
        {
            return Err("invalid sustained amount".into());
        }
        EffectKind::ControlledBurn { amount, rounds }
            if !(1..=10000).contains(amount) || !(1..=10000).contains(rounds) =>
        {
            return Err("invalid rank fire".into());
        }
        EffectKind::SetMode(mode)
            if ![
                "human",
                "beast",
                "stance_defensive",
                "stance_aggressive",
                "guard",
                "land",
                "submerged",
                "normal",
                "pawing",
                "onscreen",
                "offscreen",
                "",
            ]
            .contains(&mode.as_str()) =>
        {
            return Err("unknown mode value".into());
        }
        EffectKind::Disease { id, .. } if id.is_empty() => return Err("invalid disease ID".into()),
        EffectKind::HealPercent(n) if !n.is_finite() || !(0. ..=1.).contains(n) => {
            return Err("invalid healing ratio".into());
        }
        EffectKind::Riposte { hit, miss, buffs } => {
            if !hit.is_finite()
                || !miss.is_finite()
                || !(0. ..=1.).contains(hit)
                || !(0. ..=1.).contains(miss)
            {
                return Err("invalid counter chance".into());
            }
            for b in buffs {
                b.validate()?;
            }
        }
        EffectKind::Blocks { count, turns }
            if !(1..=10000).contains(count) || turns.is_some_and(|n| !(1..=10000).contains(&n)) =>
        {
            return Err("invalid damage blocks".into());
        }
        EffectKind::Cure {
            bleed: false,
            poison: false,
            burn: false,
        }
        | EffectKind::ClearGuard {
            guarding: false,
            guarded: false,
        } => return Err("empty state clear".into()),
        _ => {}
    }
    if effect.duration == 0
        || effect.duration > 10000
        || !effect.chance.is_finite()
        || !(0. ..=1000.).contains(&effect.chance)
        || matches!(effect.kind,EffectKind::Bleed(n)|EffectKind::Blight(n)|EffectKind::Horror(n)|EffectKind::ProtectedHorror(n) if !(0..=1000000).contains(&n))
        || matches!(effect.kind,EffectKind::Stress(n)|EffectKind::Heal(n) if n.unsigned_abs()>1000000)
        || matches!(effect.kind,EffectKind::Push(n)|EffectKind::Pull(n) if !(1..=4).contains(&n))
    {
        return Err("invalid effect".into());
    }
    Ok(())
}
fn validate_effect_context(e: &dd_data::combat::Effect, rules: &Rules) -> Result<()> {
    match &e.kind {
        EffectKind::Disease { id, .. } => {
            let ctx = rules.context.as_ref().ok_or("missing disease context")?;
            if id != "any" && !ctx.diseases.contains_key(id) {
                return Err(format!("unresolved disease {id}"));
            }
        }
        EffectKind::Composite(es) => {
            for e in es {
                validate_effect_context(e, rules)?
            }
        }
        EffectKind::InventoryEffect(kind) => {
            let mut e = e.clone();
            e.kind = *kind.clone();
            validate_effect_context(&e, rules)?
        }
        kind if kind.needs_lighting() && rules.lighting.is_none() => {
            return Err("missing battle lighting context".into());
        }
        _ => {}
    }
    Ok(())
}
fn validate_supplies(supplies: &[dd_data::items::SupplyStack]) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    if supplies.len() > 1000 {
        return Err("too many supply stacks".into());
    }
    for s in supplies {
        s.definition.validate()?;
        if s.remaining > s.definition.stack_limit || !ids.insert(&s.definition.id) {
            return Err("invalid battle supply quantity or duplicate ID".into());
        }
        for e in s
            .definition
            .effects
            .iter()
            .chain(&s.definition.semantics.sequential_effects)
        {
            validate_effect(e)?;
        }
    }
    Ok(())
}
fn validate_scenario(s: &Scenario) -> Result<()> {
    s.rules.validate()?;
    if let Some(f) = &s.rules.flashback {
        f.validate()?;
        for e in &f.starting {
            validate_effect(e)?;
        }
        if !s.heroes.iter().any(|h| f.required.contains(&h.id)) {
            return Err("flashback requires its original hero class".into());
        }
    }

    s.rules.content.validate()?;
    for b in s.heroes.iter().chain(&s.monsters) {
        if let Some(c) = &b.mechanics.hero.difficulty {
            c.validate()?;
        }
        if let Some(c) = &b.mechanics.hero.carry {
            c.status.validate()?;
        }
    }
    if !s.rules.base_affliction_history_wf.is_finite()
        || s.rules.base_affliction_history_wf <= 0.
        || !s.rules.base_virtue_history_wf.is_finite()
        || s.rules.base_virtue_history_wf <= 0.
        || s.rules.affliction_history_increment.is_empty()
        || s.rules.affliction_history_increment.len() > 10000
        || s.rules
            .affliction_history_increment
            .iter()
            .any(|n| !n.is_finite() || !(0. ..=1000000.).contains(n))
    {
        return Err("invalid resolve history rules".into());
    }
    if s.rules.context.is_none()
        && s.heroes
            .iter()
            .any(|b| !b.mechanics.build.input.diseases.is_empty())
    {
        return Err("missing initial disease context".into());
    }
    if let Some(ctx) = &s.rules.context {
        ctx.validate()?;
        if ctx.content != s.rules.content {
            return Err("context content mismatch".into());
        }
    }
    fn needs_disease(e: &dd_data::combat::Effect) -> bool {
        match &e.kind {
            EffectKind::Disease { .. } => true,
            EffectKind::Composite(es) => es.iter().any(needs_disease),
            _ => false,
        }
    }
    if s.rules.context.is_none()
        && s.heroes
            .iter()
            .chain(&s.monsters)
            .flat_map(|b| b.skills.iter())
            .flat_map(|s| &s.effects)
            .any(needs_disease)
    {
        return Err("missing disease context".into());
    }
    if s.rules.lighting.is_none()
        && s.heroes.iter().chain(&s.monsters).any(|b| {
            b.skills
                .iter()
                .flat_map(|s| &s.effects)
                .chain(b.mechanics.trait_effects.values())
                .chain(&b.mechanics.lifecycle.spawn)
                .chain(&b.mechanics.lifecycle.death_effects)
                .chain(b.mechanics.lifecycle.wave_spawn.iter().map(|e| &e.effect))
                .any(|e| e.kind.needs_lighting())
        })
    {
        return Err("missing battle lighting context".into());
    }
    if let Some(b) = &s.rules.bloodlight {
        b.validate()?;
        if s.rules.farm.is_some()
            || s.rules.flashback.is_some()
            || s.rules
                .lighting
                .as_ref()
                .is_none_or(|l| l.table != b.darkness || l.torch != 100)
        {
            return Err("inconsistent bloodlight battle context".into());
        }
    }
    if let Some(c) = &s.rules.encounter_contract {
        c.validate()?;
    }
    if let Some(plan) = &s.rules.farm {
        plan.validate()?;
        if s.rules.flashback.is_some() {
            return Err("farm and flashback contexts conflict".into());
        }
    }
    if let Some(light) = &s.rules.lighting {
        light.validate()?;
    }
    dd_data::mechanics::validate_stress(&s.rules)?;
    let globals = s
        .rules
        .globals
        .as_ref()
        .ok_or("missing combat Buff/trait definitions")?;
    for t in &globals.traits {
        t.validate()?;
    }
    let mut trait_ids = std::collections::BTreeSet::new();
    if globals.traits.iter().any(|t| !trait_ids.insert(&t.id)) {
        return Err("ambiguous trait id".into());
    }
    if !globals.traits.iter().any(|t| t.virtue) || !globals.traits.iter().any(|t| !t.virtue) {
        return Err("missing resolve outcomes".into());
    }
    for b in globals
        .stun_recovery
        .iter()
        .chain(globals.traits.iter().flat_map(|t| &t.buffs))
    {
        b.validate()?;
    }
    if s.round_limit == 0
        || s.round_limit > 10000 && s.rules.farm.is_none()
        || s.heroes.is_empty()
        || (s.monsters.is_empty() && s.rules.farm.is_none())
        || s.heroes.len() > 4
        || s.monsters.len() > 4
    {
        return Err("scenario requires 1..4 units per side and 1..10000 rounds".into());
    }
    if s.rules.initiative_base < 1
        || s.rules.initiative_base > 10000
        || !(0. ..=1.).contains(&s.rules.hit_min)
        || !(s.rules.hit_min..=1.).contains(&s.rules.hit_max)
    {
        return Err("invalid scenario rules".into());
    }
    for table in [
        &s.rules.player_crit_multiplier_table,
        &s.rules.monster_crit_multiplier_table,
    ] {
        if table.len() != 1
            || !table[0].value.is_finite()
            || table[0].value < 1.
            || table[0].value > 100.
        {
            return Err("invalid critical table".into());
        }
    }
    for (id, hero) in s.heroes.iter().enumerate() {
        if s.heroes.iter().enumerate().any(|(other, ally)| {
            id != other
                && hero
                    .mechanics
                    .hero
                    .incompatible_party_tags
                    .iter()
                    .any(|tag| ally.mechanics.hero.tags.contains(tag))
        }) {
            return Err("incompatible hero party tags".into());
        }
    }
    for (hero, units) in [(true, &s.heroes), (false, &s.monsters)] {
        if units.iter().map(|u| usize::from(u.size)).sum::<usize>() > 4 {
            return Err("formation exceeds four ranks".into());
        }
        for u in units {
            if u.skills.iter().any(|s| {
                s.effects.iter().any(|e| {
                    e.kind.contains_riposte()
                        && (e.target == EffectTarget::Performer
                            || (e.target == EffectTarget::Target
                                && s.target.friendly
                                && s.tactical.self_target_valid))
                })
            }) && u.mechanics.riposte.is_none()
            {
                return Err("riposte effect requires original counter skill".into());
            }
            if u.hero {
                for t in globals
                    .traits
                    .iter()
                    .chain(&u.mechanics.hero.override_defs)
                    .chain(&u.mechanics.build.quirk_acts023)
                {
                    t.validate()?;
                    let names = t
                        .turn_acts
                        .iter()
                        .filter(|a| {
                            a.chance > 0.
                                && (a.valid_hero_class_ids.is_empty()
                                    || a.valid_hero_class_ids.contains(&u.id))
                        })
                        .map(|a| a.data.string_value.as_str())
                        .chain(
                            t.reactions
                                .iter()
                                .filter(|r| {
                                    r.chance > 0.
                                        && (r.id.is_combat()
                                            || dd_data::mechanisms023::enabled(
                                                &s.rules.content,
                                                dd_data::mechanisms023::FIXES,
                                            ))
                                })
                                .map(|r| r.data.effect.as_str()),
                        );
                    for name in names.filter(|n| !n.is_empty()) {
                        let e =
                            u.mechanics.trait_effects.get(name).ok_or_else(|| {
                                format!("{}: unresolved trait effect {name}", u.id)
                            })?;
                        if e.name != name {
                            return Err("trait effect name mismatch".into());
                        }
                        if !e.conditions.is_empty() {
                            return Err(
                                "conditional trait effect requires skill attack context".into()
                            );
                        }
                        validate_effect(e)?;
                    }
                }
            }
            for b in u
                .mechanics
                .deaths_door
                .iter()
                .chain(&u.mechanics.recovery)
                .chain(&u.mechanics.heart_recovery)
                .chain(&u.mechanics.on_crit)
            {
                b.validate()?;
            }
            if let Some(brain) = &u.mechanics.brain {
                brain.validate_candidates(
                    &u.skills.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                    s.rules
                        .content
                        .syntax_choices
                        .get("boss_inactive_refs")
                        .map(String::as_str)
                        == Some("retain"),
                )?;
                if u.hero || u.brain_id.as_ref().is_some_and(|id| *id != brain.id) {
                    return Err("invalid actor brain".into());
                }
            } else if u.brain_id.is_some() {
                return Err("missing interpreted monster brain".into());
            }
            if u.hero != hero
                || !(1..=4).contains(&u.size)
                || (u.hero && u.size != 1)
                || (u.skills.is_empty() && u.mechanics.lifecycle.turns > 0)
                || u.stats.hp < 1
                || u.stats.hp > 1000000
                || u.stats.speed.unsigned_abs() > 10000
                || u.move_back > 4
                || u.move_forward > 4
            {
                return Err("invalid blueprint".into());
            }
            for stats in std::iter::once(&u.stats).chain(u.corpse.iter().map(|c| &c.stats)) {
                if [
                    stats.dodge,
                    stats.protection,
                    stats.accuracy,
                    stats.crit,
                    stats.death_resist,
                    stats.stun_resist,
                    stats.bleed_resist,
                    stats.blight_resist,
                    stats.debuff_resist,
                    stats.move_resist,
                ]
                .iter()
                .any(|n| !n.is_finite() || n.abs() > 1000.)
                {
                    return Err("invalid stat".into());
                }
            }
            if u.corpse.as_ref().is_some_and(|c| {
                c.size == 0
                    || c.size > u.size
                    || c.stats.hp < 1
                    || c.rounds == 0
                    || c.rounds > 10000
            }) {
                return Err("invalid corpse".into());
            }
            let h = &u.mechanics.hero;
            if s.rules.context.is_none()
                && (!h.resolve_history.is_empty() || !h.dungeon_history.is_empty())
            {
                return Err("inherited history requires battle context".into());
            }
            if h.resolve_history.iter().any(|(id, n)| {
                *n > 1000000
                    || !globals
                        .traits
                        .iter()
                        .chain(&h.override_defs)
                        .any(|t| &t.id == id)
            }) {
                return Err("invalid initial resolve history".into());
            }
            if !h.fixed_skills.is_empty()
                && (h.fixed_skills.len() != u.skills.len()
                    || h.fixed_skills
                        .iter()
                        .any(|id| !u.skills.iter().any(|s| &s.id == id)))
            {
                return Err("class requires its complete fixed skill set".into());
            }
            if h.initial_hp.is_some_and(|hp| !(0..=1000000).contains(&hp))
                || h.initial_mode
                    .as_ref()
                    .is_some_and(|id| !h.modes.iter().any(|m| &m.id == id))
                || h.enter_cooldown > 10000
            {
                return Err("invalid hero initial state".into());
            }
            if h.host_passive_buffs.len() > 10000 || !u.hero && !h.host_passive_buffs.is_empty() {
                return Err("invalid host passive Buff collection".into());
            }
            for buff in &h.host_passive_buffs {
                buff.validate()?;
                if !matches!(buff.duration_type.as_deref(), None | Some("quest_end")) {
                    return Err("unsupported host passive Buff expiry".into());
                }
            }
            if h.host_buffs.len() > 10000
                || (!u.hero && !h.host_buffs.is_empty())
                || h.host_buffs
                    .iter()
                    .map(|b| b.instance)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != h.host_buffs.len()
            {
                return Err("invalid host buff instance catalog".into());
            }
            for b in &h.host_buffs {
                b.definition.validate()?;
                if b.instance == 0 || b.definition.duration_type.as_deref() != Some("quest_end") {
                    return Err("host buff requires an explicit host-owned lifetime".into());
                }
            }
            if h.exploration_buffs.len() > 10000 {
                return Err("too many exploration Buffs".into());
            }
            for b in &h.exploration_buffs {
                b.definition.validate()?;
                if b.remaining.is_some_and(|n| n == 0 || n > 10000) {
                    return Err("invalid exploration Buff boundary".into());
                }
            }
            if let Some(c) = &h.carry {
                use dd_data::hero010::CarryResolve;
                if !(0..200).contains(&c.stress) {
                    return Err("invalid inherited stress".into());
                }
                match &c.resolve {
                    CarryResolve::Calm if c.stress >= 100 => {
                        return Err("missing inherited resolve state".into());
                    }
                    CarryResolve::Afflicted(id) | CarryResolve::Virtuous(id) => {
                        let virtue = matches!(c.resolve, CarryResolve::Virtuous(_));
                        if (!virtue && c.stress == 0)
                            || !globals
                                .traits
                                .iter()
                                .chain(&h.override_defs)
                                .any(|t| &t.id == id && t.virtue == virtue)
                        {
                            return Err("invalid inherited resolve trait".into());
                        }
                    }
                    _ => {}
                }
                if c.buffs.len() > 10000 {
                    return Err("too many inherited Buffs".into());
                }
                for b in &c.buffs {
                    b.definition.validate()?;
                    if (b.definition.duration_type.as_deref() != Some("quest_end")
                        && b.definition.activation.remove_on_battle_complete)
                        || b.remaining.is_some_and(|n| n == 0 || n > 10000)
                    {
                        return Err("invalid inherited Buff boundary".into());
                    }
                }
            }
            let mut mode_ids = std::collections::BTreeSet::new();
            for mode in &h.modes {
                if !mode_ids.insert(&mode.id) || mode.stress < 0 || mode.stress > 1000 {
                    return Err("invalid mode definition".into());
                }
                for name in mode.affliction_skill.iter().chain(&mode.end_skill) {
                    if !h.forced.contains_key(name) {
                        return Err("missing forced mode skill".into());
                    }
                }
                for e in &mode.start {
                    validate_effect(e)?;
                }
            }
            if !h.modes.is_empty() && !h.modes.iter().any(|m| m.default) {
                return Err("mode missing default".into());
            }
            for (id, w) in &h.resolve_override {
                if !w.is_finite()
                    || *w < 0.
                    || !globals
                        .traits
                        .iter()
                        .chain(&h.override_defs)
                        .any(|t| &t.id == id)
                {
                    return Err("invalid resolve override".into());
                }
            }
            for b in u
                .mechanics
                .build
                .buffs
                .iter()
                .chain(
                    u.mechanics
                        .build
                        .trinkets
                        .iter()
                        .flat_map(|t| &t.definition.buffs),
                )
                .chain(u.mechanics.build.disease_buffs.values().flatten())
            {
                if ["lightabove", "lightbelow"].contains(&b.rule_type.as_str())
                    && s.rules.lighting.is_none()
                {
                    return Err("conditional build requires lighting context".into());
                }
                if [
                    "in_dungeon",
                    "in_corridor",
                    "walking_backwards",
                    "quest_uses_remaining_above",
                ]
                .contains(&b.rule_type.as_str())
                    && s.rules.context.is_none()
                {
                    return Err("conditional build requires battle context".into());
                }
            }
            for b in u
                .mechanics
                .build
                .buffs
                .iter()
                .chain(&u.mechanics.deaths_door)
                .chain(&u.mechanics.recovery)
                .chain(&u.mechanics.on_crit)
            {
                b.validate()?;
                for id in &b.activation.attack_additional_effect_ids {
                    if !u.mechanics.extra_effects.contains_key(id) {
                        return Err("unresolved build trigger".into());
                    }
                }
            }
            for e in u.mechanics.extra_effects.values() {
                validate_effect(e)?;
            }
            for skill in u
                .skills
                .iter()
                .chain(u.mechanics.riposte.as_deref())
                .chain(h.forced.values())
            {
                let t = &skill.tactical;
                t.boss.validate()?;
                if t.valid_modes.iter().any(|m| {
                    !h.modes.iter().any(|v| &v.id == m)
                        && !u.mechanics.boss.modes.iter().any(|v| &v.id == m)
                }) || t.mode_effects.keys().any(|m| {
                    !h.modes.iter().any(|v| &v.id == m)
                        && !u.mechanics.boss.modes.iter().any(|v| &v.id == m)
                }) || !t.dmg_per_burn_stack.is_finite()
                    || !(0. ..=100.).contains(&t.dmg_per_burn_stack)
                {
                    return Err("invalid skill mode/burn dependency".into());
                }
                for e in t.mode_effects.values().flatten() {
                    validate_effect(e)?;
                }
                if t.hp_range.is_some_and(|r| {
                    !r[0].is_finite()
                        || !r[1].is_finite()
                        || !(0. ..=1.).contains(&r[0])
                        || !(r[0]..=1.).contains(&r[1])
                }) {
                    return Err("invalid skill HP predicate".into());
                }
                if t.continue_turn && t.per_turn_limit.is_none()
                    || t.per_turn_limit.is_some_and(|n| n == 0 || n > 10000)
                {
                    return Err("free skill requires finite per-turn limit".into());
                }
                if !["", "melee", "ranged", "heal", "0", "teleport"]
                    .contains(&t.attack_type.as_str())
                    || !t.extra_targets_chance.is_finite()
                    || !(0. ..=1.).contains(&t.extra_targets_chance)
                    || t.extra_targets_count > 3
                    || (skill.target.multi && skill.target.random)
                    || (!t.self_target_valid && skill.target.ranks.is_empty())
                {
                    return Err("invalid tactical skill".into());
                }
                if !skill.accuracy.is_finite()
                    || !skill.crit.is_finite()
                    || skill.move_forward > 4
                    || skill.move_back > 4
                    || (skill.move_back > 0 && skill.move_forward > 0)
                {
                    return Err("invalid skill accuracy/critical/movement".into());
                }
                if skill.launch.is_empty()
                    || (skill.target.ranks.is_empty()
                        && (!skill.target.friendly || skill.target.multi))
                    || skill.per_battle_limit.is_some_and(|n| n == 0 || n > 10000)
                    || skill
                        .launch
                        .iter()
                        .chain(&skill.target.ranks)
                        .any(|r| !(1..=4).contains(r))
                    || (skill.target.friendly && skill.damage.is_some())
                    || (!skill.target.friendly && (skill.damage.is_none() || skill.heal.is_some()))
                {
                    return Err("invalid skill".into());
                }
                for range in [skill.damage, skill.heal].into_iter().flatten() {
                    if range[0] < 0 || range[1] < range[0] || range[1] > 1000000 {
                        return Err("invalid skill range".into());
                    }
                }
                for effect in &skill.effects {
                    validate_effect(effect)?;
                    validate_effect_context(effect, &s.rules)?;
                }
            }
            u.mechanics.boss.validate()?;
            let boss = &u.mechanics.boss;
            for e in boss
                .reactions
                .hit_performer
                .iter()
                .chain(&boss.reactions.killed_others)
                .chain(&boss.reactions.killed_by_hero)
                .chain(boss.empty.iter().flat_map(|c| &c.effects))
                .chain(boss.full.iter().flat_map(|c| &c.effects))
            {
                validate_effect(e)?;
            }

            let life = &u.mechanics.lifecycle;
            if life.turns > 4
                || life.role == dd_data::lifecycle::EntityRole::Corpse && life.lifetime.is_none()
                || u.hero
                    && (life.turns != 1 || life.role != dd_data::lifecycle::EntityRole::Fighter)
                || life.lifetime.is_some_and(|n| n == 0 || n > 10000)
            {
                return Err("invalid actor lifecycle".into());
            }
            for e in life
                .spawn
                .iter()
                .chain(&life.death_effects)
                .chain(life.wave_spawn.iter().map(|s| &s.effect))
            {
                validate_effect(e)?;
            }
            if let Some(c) = &life.death_change {
                dd_data::lifecycle::LifeOp::Change(c.clone()).validate()?;
            }
        }
    }
    Ok(())
}



pub mod initial013;

pub mod generate014;
pub mod maps014;

mod field015;
pub mod opening015;

mod loot015;

pub mod town016;

pub mod events017;

pub use town016::campaign as campaign017;

pub mod opening_policy015;

pub mod expedition_sources017;

pub use opening015::campaign as expedition017;

pub mod campaign_policy017;

pub use town016::campaign::automation as campaign018;

pub mod auto_policy018;
pub mod navigation019;
pub mod tactics021;

mod shared024c;
pub mod strategy025;

pub mod rollout027;
