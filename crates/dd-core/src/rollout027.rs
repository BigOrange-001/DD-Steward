//! Public-observation-only surrogate battles. This API never accepts a real Battle.
use crate::*;
use dd_data::combat::GameData;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
#[path = "rollout027_arena.rs"]
pub mod arena;
#[path = "rollout027_corpus.rs"]
pub mod corpus;
#[path = "rollout027_decision.rs"]
mod decision;
#[path = "rollout027_diagnostic.rs"]
pub mod diagnostic;
#[path = "human030_arena.rs"]
pub mod human030;
#[path = "rollout027_model.rs"]
mod model;
#[path = "rollout027_retreat.rs"]
pub mod retreat;
pub use decision::{
    CandidateResult, Decision, SampleResult, choose, diagnostic_score030, evaluate_action,
};
pub use model::{Definitions, Surrogate, reconstruct};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub coherence029: bool,
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
    pub policy: BTreeMap<String, f64>,
    pub fast_policy: BTreeMap<String, f64>,
    pub estimates: BTreeMap<String, String>,
}
impl Settings {
    pub fn budgeted() -> Result<Self> {
        let v: Value = serde_json::from_str(r#"{"effective_samples":8}
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
  "version": "visible-flat-rollout-027-w126-v4",
  "source": "W-123/W-124/W-126: candidate radiant tuning",
  "samples": 8,
  "top_actions": 4,
  "horizon_rounds": 30,
  "transitions_per_round": 128,
  "mean_weight": 0.7,
  "tail_weight": 0.3,
  "tail_fraction": 0.1,
  "salt": "dd-sim-026-public-observation-v1",
  "weights": {
    "death": 200,
    "death_level": 1.5,
    "door": 100,
    "affliction": 300.0,
    "stress": 1.0,
    "excess_stress": 3.0,
    "missing_hp": 100.0,
    "round": 0,
    "retreat": 300,
    "unfinished_threat": 1.0,
    "critical_death": 1.5,
    "abandon_1": 2000.0,
    "abandon_3": 3000.0,
    "abandon_5": 4000.0,
    "detour": 300
  },
  "fast_policy": {
    "low_threat_enemy_count": 1,
    "low_threat_hp_fraction": 0.5,
    "heal_weight": 3.0,
    "relief_weight": 2.0,
    "damage_weight": 1.0,
    "control_weight": 1.0,
    "low_threat_recovery_multiplier": 2.0,
    "high_threat_relief_multiplier": 0.25,
    "movement_cost": 1.0,
    "pass_cost": 1000.0,
    "retreat_default_cost": 10000.0,
    "sacrifice_default_cost": 100000.0
  },
  "estimates": {
    "buff_application_age": "previous own turn when unavailable",
    "corpse_age": "fresh source lifetime when unavailable",
    "stall_counter": "public observed actions; unknown prehistory starts at zero, explicitly estimated",
    "enemy_history": "source defaults and visible opportunity counts; never true used-skills or cooldowns",
    "regen_critical": "unobservable origin; conservative non-critical regen",
    "fast_policy": "Finish, Recover, Protect; public candidate tail safety and native consequences; pilot2 then top2 each8",
    "horizon": "30 ordinary / 50 source-tagged boss; stop sample after death or recovered party; no round cost",
    "allocation": "pilot2; final top2 distinct first actions plus protected death-door escape/heal, each8; no eliminated pilot can win",
    "detour": "Candidate extra 300 for a proven scouted bypass, in addition to escape 300; no turn cost."
  },
  "policy": {
    "tail_min_hp": 0.35,
    "tail_max_stress": 95.0,
    "tail_threat_fraction": 0.3,
    "tail_hp_target": 1.0,
    "tail_stress_target": 0.0,
    "protect_heal": 8.0,
    "tail_kill_penalty": 500.0,
    "corpse_needed": 80.0,
    "corpse_other": 10.0,
    "tail_recovery": 5.0,
    "tail_damage_penalty": 4.0,
    "needed_attack": 20.0,
    "tail_pass_cost": 1.0,
    "boss_horizon": 50.0,
    "finalists": 2.0,
    "retreat_risk_threshold": 0.5,
    "retreat_probe_samples": 8,
    "retry_win_threshold": 0.5
  }
}
"#)
            .map_err(|e| e.to_string())?;
        let weights = [
            "detour",
            "critical_death",
            "abandon_1",
            "abandon_3",
            "abandon_5",
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
        if ![
            "retreat_risk_threshold",
            "retreat_probe_samples",
            "retry_win_threshold",
        ]
        .iter()
        .all(|k| s.policy.contains_key(*k))
            || !(0.0..=1.0).contains(&s.policy["retreat_risk_threshold"])
            || !(0.0..=1.0).contains(&s.policy["retry_win_threshold"])
            || s.policy["retreat_probe_samples"] < 8.
            || s.policy.len() != 18
            || s.policy.values().any(|v| !v.is_finite() || *v < 0.)
            || s.version != "visible-flat-rollout-027-w126-v4"
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
    #[serde(default)]
    pub history: PublicHistory,
    #[serde(default)]
    pub rescue: Rescue,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublicHistory {
    pub round: u32,
    pub count: u32,
    pub attacks: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rescue {
    /// Only the filtered map; absent in historical arena inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_map: Option<Value>,
    pub food: u32,
    pub food_heal: f64,
    pub camp: bool,
    #[serde(default)]
    pub camp_heal: f64,
    #[serde(default)]
    pub camp_relief: i32,
    #[serde(default)]
    pub camp_food: u32,
    #[serde(default)]
    pub food_remaining: BTreeMap<UnitId, u32>,
}
impl PublicHistory {
    pub fn observe(
        &mut self,
        before: &Observation,
        action: &Action,
        after: &Observation,
        defs: &Definitions,
    ) {
        if self.round != before.round {
            self.round = before.round;
            self.attacks = 0;
        }
        if let Some(id) = before.actor {
            let slot = match action {
                Action::Skill { skill, .. } | Action::SkillAuto { skill } => Some(*skill),
                _ => None,
            };
            if defs
                .units
                .get(&id)
                .filter(|b| b.hero)
                .and_then(|b| slot.and_then(|i| b.skills.get(i)))
                .is_some_and(|s| s.tactical.stall_invalidating)
            {
                self.attacks = self.attacks.saturating_add(1);
            }
        }
        if after.round > before.round {
            if let Some(c) = &defs.rules.context {
                let foes = after
                    .units
                    .iter()
                    .filter(|u| !u.hero && !u.corpse && u.hp > 0)
                    .filter_map(|u| defs.units.get(&u.id))
                    .filter(|b| !b.mechanics.boss.restrictions.no_stall_count)
                    .collect::<Vec<_>>();
                let heroes = after.units.iter().filter(|u| u.hero && !u.corpse).count();
                let eligible = heroes >= c.stall_party as usize
                    && !foes.is_empty()
                    && foes.len() <= c.stall_enemy as usize
                    && !foes.iter().any(|b| b.mechanics.lifecycle.disable_stall)
                    && self.attacks < c.stall_skills;
                let accelerated = foes.iter().all(|b| b.mechanics.lifecycle.accelerate_stall);
                let grace = if foes.iter().any(|b| u32::from(b.size) >= c.stall_extra_size) {
                    c.stall_extra_amount
                } else {
                    0
                };
                self.count = if eligible {
                    self.count.saturating_add(if accelerated {
                        if self.count > 0 {
                            c.stall_consecutive
                        } else {
                            c.stall_accelerated
                        }
                    } else {
                        1
                    })
                } else {
                    0
                };
                if self.count >= c.stall_reset + grace {
                    self.count = 0;
                }
            }
            self.round = after.round;
            self.attacks = 0;
        }
    }
}
