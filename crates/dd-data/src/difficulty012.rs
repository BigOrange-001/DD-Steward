//! Resolve-level versus quest-level battle modifiers. Entry stress is charged
//! only when the host explicitly marks a new raid, never on every battle.
use crate::combat::{Blueprint, GameData, Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeroDifficulty {
    pub resolve_level: u32,
    pub quest_level: u32,
    pub effective: u32,
    pub entering_raid: bool,
    pub entry_stress: i32,
    pub stress_modifier: f64,
    pub affliction_modifier: f64,
}
impl HeroDifficulty {
    pub fn validate(&self) -> Result<()> {
        if self.resolve_level > 100
            || self.quest_level > 100
            || self.effective > 100
            || !(0..=1000).contains(&self.entry_stress)
            || !self.stress_modifier.is_finite()
            || !(0. ..=10.).contains(&self.stress_modifier)
            || !self.affliction_modifier.is_finite()
            || !(0. ..=1.).contains(&self.affliction_modifier)
        {
            return Err("invalid hero difficulty context".into());
        }
        Ok(())
    }
}
impl GameData {
    pub fn hero_at_difficulty(
        &self,
        mut hero: Blueprint,
        resolve_level: u32,
        quest_level: u32,
        entering_raid: bool,
    ) -> Result<Blueprint> {
        if !hero.hero {
            return Err("quest difficulty requires a hero".into());
        }
        let raw: serde_json::Value =
            serde_json::from_slice(&self.overlay.read("shared/rules.json")?)
                .map_err(|e| e.to_string())?;
        let bound = |key: &str| -> Result<u32> {
            raw[key]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0. && v.fract() == 0. && *v <= 100.)
                .map(|v| v as u32)
                .ok_or_else(|| format!("invalid difficulty bound {key}"))
        };
        let min = bound("effectiveDifficultyMin")?;
        let max = bound("effectiveDifficultyMax")?;
        if min > max || resolve_level > max || quest_level > max {
            return Err("hero/quest level outside original difficulty range".into());
        }
        let effective = quest_level.saturating_sub(resolve_level).clamp(min, max);
        let at = |key: &str| -> Result<f64> {
            raw[key]
                .as_array()
                .filter(|a| a.len() == max as usize + 1)
                .and_then(|a| a.get(effective as usize))
                .and_then(|v| v.as_f64())
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("invalid difficulty table {key}"))
        };
        let entry = at("effectiveDifficultyDungeonStartingStress")?;
        if entry.fract() != 0. {
            return Err("nonintegral entry stress".into());
        }
        let context = HeroDifficulty {
            resolve_level,
            quest_level,
            effective,
            entering_raid,
            entry_stress: entry as i32,
            stress_modifier: at("effectiveDifficultyStressDmgModifiers")?,
            affliction_modifier: at("effectiveDifficultyAfflictionOnsetModifiers")?,
        };
        context.validate()?;
        hero.mechanics.hero.difficulty = Some(context);
        Ok(hero)
    }
}
