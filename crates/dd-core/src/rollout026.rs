//! Public-observation-only surrogate battles. This API never accepts a real Battle.
use crate::*;
use dd_data::combat::GameData;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
#[path = "rollout026_arena.rs"]
pub mod arena;
#[path = "rollout026_corpus.rs"]
pub mod corpus;
#[path = "rollout026_decision.rs"]
mod decision;
#[path = "rollout026_model.rs"]
mod model;
pub use decision::{CandidateResult, Decision, SampleResult, choose, evaluate_action};
pub use model::{Definitions, Surrogate, reconstruct};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: String,
    pub source: String,
    pub samples: usize,
    pub top_actions: usize,
    pub horizon_rounds: u32,
    pub transitions_per_round: usize,
    pub mean_weight: f64,
    pub tail_weight: f64,
    pub tail_fraction: f64,
    pub salt: String,
    pub weights: BTreeMap<String, f64>,
    pub fast_policy: BTreeMap<String, f64>,
    pub estimates: BTreeMap<String, String>,
}
impl Settings {
    pub fn budgeted() -> Result<Self> {
        let v: Value = serde_json::from_str(r#"{
  "version": "026-budget-v1",
  "effective_samples": 2,
  "documented_default": 24,
  "basis": "Speed-only preflight: same 500-step seed1 prefixes, 24 samples 135.2474473s versus off18.753281s (7.21x). Fixed before observing completed arena comparisons; no weight tuning. Validate 26-week budget separately."
}
"#)
            .map_err(|e| e.to_string())?;
        let mut s = Self::documented()?;
        s.samples = v["effective_samples"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= s.samples as u64)
            .ok_or("invalid026 sample budget")? as usize;
        Ok(s)
    }
    pub fn documented() -> Result<Self> {
        let s: Self = serde_json::from_str(r#"{
  "version":"visible-flat-rollout-026-v1",
  "source":"User W-118/W-120/W-122 and026-battle-rollout.md; unverified candidate",
  "samples":24,
  "top_actions":4,
  "horizon_rounds":12,
  "transitions_per_round":128,
  "mean_weight":0.7,
  "tail_weight":0.3,
  "tail_fraction":0.1,
  "salt":"dd-sim-026-public-observation-v1",
  "weights":{"death":1000.0,"death_level":0.25,"door":150.0,"affliction":300.0,"stress":1.0,"excess_stress":3.0,"missing_hp":100.0,"round":2.0,"retreat":150.0,"unfinished_threat":1.0},
  "fast_policy":{"low_threat_enemy_count":1,"low_threat_hp_fraction":0.5,"heal_weight":3.0,"relief_weight":2.0,"damage_weight":1.0,"control_weight":1.0,"low_threat_recovery_multiplier":2.0,"high_threat_relief_multiplier":0.25,"movement_cost":1.0,"pass_cost":1000.0,"retreat_default_cost":10000.0,"sacrifice_default_cost":100000.0},
  "estimates":{"buff_application_age":"previous own turn when unavailable","corpse_age":"fresh source lifetime when unavailable","stall_counter":"visible elapsed rounds estimate; never real counter; source reinforcement formation sampled independently","enemy_history":"source defaults and visible opportunity counts; never true used-skills or cooldowns","regen_critical":"unobservable origin; conservative non-critical regen","fast_policy":"simple W120 surrogate policy, all numbers candidate; no actual future queue access"}
}
"#)
            .map_err(|e| e.to_string())?;
        let weights = [
            "death",
            "death_level",
            "door",
            "affliction",
            "stress",
            "excess_stress",
            "missing_hp",
            "round",
            "retreat",
            "unfinished_threat",
        ];
        let fast = [
            "low_threat_enemy_count",
            "low_threat_hp_fraction",
            "heal_weight",
            "relief_weight",
            "damage_weight",
            "control_weight",
            "low_threat_recovery_multiplier",
            "high_threat_relief_multiplier",
            "movement_cost",
            "pass_cost",
            "retreat_default_cost",
            "sacrifice_default_cost",
        ];
        if s.version != "visible-flat-rollout-026-v1"
            || s.samples == 0
            || !(3..=5).contains(&s.top_actions)
            || s.horizon_rounds == 0
            || s.transitions_per_round == 0
            || !s.mean_weight.is_finite()
            || !s.tail_weight.is_finite()
            || s.mean_weight < 0.
            || s.tail_weight < 0.
            || (s.mean_weight + s.tail_weight - 1.).abs() > 1e-9
            || !s.tail_fraction.is_finite()
            || s.tail_fraction <= 0.
            || s.tail_fraction > 1.
            || s.weights.len() != weights.len()
            || weights.iter().any(|k| !s.weights.contains_key(*k))
            || s.fast_policy.len() != fast.len()
            || fast.iter().any(|k| !s.fast_policy.contains_key(*k))
            || s.weights
                .values()
                .chain(s.fast_policy.values())
                .any(|n| !n.is_finite() || *n < 0.)
        {
            return Err("invalid026 rollout settings".into());
        }
        Ok(s)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroTraining {
    pub class: String,
    pub weapon: usize,
    pub armour: usize,
    pub skills: Vec<(String, i32)>,
    pub level: u32,
    #[serde(default)]
    pub grown_equipment: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicBattle {
    pub content: dd_data::content::ContentConfig,
    pub observation: Observation,
    /// From player-visible roster and022 frozen deployment identity bridge.
    pub heroes: BTreeMap<UnitId, HeroTraining>,
    pub region: Option<String>,
    pub difficulty: u32,
    pub corridor: bool,
    pub plot: Option<String>,
    #[serde(default)]
    pub retreat_legal: Option<bool>,
    #[serde(default)]
    pub mechanisms: execution025::Options,
    #[serde(default)]
    pub single_target_fallback: bool,
}
pub fn sample_seed(input: &PublicBattle, s: &Settings, index: usize) -> Result<u64> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(&(input, &s.salt, index)).map_err(|e| e.to_string())?;
    Ok(u64::from_le_bytes(
        Sha256::digest(bytes)[..8].try_into().unwrap(),
    ))
}
pub fn full_content() -> Result<dd_data::content::ContentConfig> {
    dd_data::mechanisms023::Candidates::documented()
        .apply(dd_data::content::ContentConfig::full().with_campaign018_candidates()?)
}
/// Explicit partial historical import: complete source-inferable omitted fields.
/// `input.observation.units/round/actor/torch` must still match exactly.
pub fn complete_historical(input: &PublicBattle, defs: &Definitions) -> Result<PublicBattle> {
    model::historical(input, defs)
}
