//! Explicit host candidate values absent from the original rule table.
//! The versioned document and source are retained verbatim in battle checkpoints.
use crate::combat::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CombatCandidates {
    pub id: String,
    pub status: String,
    pub healing: CriticalHealing,
    pub dot_critical: CriticalDot,
    pub limits: CombatLimits,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CriticalHealing {
    pub single_chance: f64,
    pub multi_chance: f64,
    pub multiplier: f64,
    pub target_stress_relief: i32,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CriticalDot {
    pub duration_multiplier: f64,
    pub rounding: String,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CombatLimits {
    pub protection: f64,
    pub death_blow_resistance: f64,
    pub protection_source: String,
    pub death_blow_source: String,
}
impl CombatCandidates {
    /// This is a documented experiment preset, never a claim of parsed game rules.
    pub fn documented() -> Self {
        serde_json::from_str(r#"{
  "id": "combat-candidates-012-v1",
  "status": "candidate; not calibrated against the original executable; not admitted for formal training",
  "healing": {
    "single_chance": 0.12,
    "multi_chance": 0.05,
    "multiplier": 2.0,
    "target_stress_relief": 4,
    "source": "https://darkestdungeon.wiki.gg/wiki/Critical_Hit_%28Darkest_Dungeon%29"
  },
  "dot_critical": {
    "duration_multiplier": 1.5,
    "rounding": "ceil",
    "source": "https://darkestdungeon.wiki.gg/wiki/Critical_Hit_%28Darkest_Dungeon%29"
  },
  "limits": {
    "protection": 0.8,
    "death_blow_resistance": 0.87,
    "protection_source": "https://darkestdungeon.wiki.gg/wiki/Combat_Mechanics_%28Darkest_Dungeon%29",
    "death_blow_source": "https://darkestdungeon.wiki.gg/wiki/Death%27s_Door_%28Darkest_Dungeon%29"
  }
}
"#)
            .expect("versioned host candidate document")
    }
    pub fn validate(&self) -> Result<()> {
        for p in [
            self.healing.single_chance,
            self.healing.multi_chance,
            self.limits.protection,
            self.limits.death_blow_resistance,
        ] {
            if !p.is_finite() || !(0. ..=1.).contains(&p) {
                return Err("invalid host candidate probability/cap".into());
            }
        }
        if self.id.trim().is_empty()
            || self.status.trim().is_empty()
            || !self.healing.multiplier.is_finite()
            || !(1. ..=10.).contains(&self.healing.multiplier)
            || !(0..=1000).contains(&self.healing.target_stress_relief)
            || !self.dot_critical.duration_multiplier.is_finite()
            || !(1. ..=10.).contains(&self.dot_critical.duration_multiplier)
            || self.dot_critical.rounding != "ceil"
        {
            return Err("invalid host candidate rule document".into());
        }
        for source in [
            &self.healing.source,
            &self.dot_critical.source,
            &self.limits.protection_source,
            &self.limits.death_blow_source,
        ] {
            if !source.starts_with("https://") || source.len() > 2000 {
                return Err("host candidate requires an explicit source".into());
            }
        }
        Ok(())
    }
    /// Includes the values as well as the version: changing any value changes identity.
    pub fn fingerprint(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
    pub fn identity(&self) -> Result<String> {
        // Stable provenance identifier, not a cryptographic authentication scheme.
        let hash = self
            .fingerprint()?
            .bytes()
            .fold(0xcbf29ce484222325u64, |a, b| {
                (a ^ u64::from(b)).wrapping_mul(0x100000001b3)
            });
        Ok(format!("{}:{hash:016x}", self.id))
    }
}

