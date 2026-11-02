//! Opt-in task 023 identities. Original inputs remain the numerical authority.
use crate::{
    combat::Result,
    content::ContentConfig,
    mechanics::{Reaction, TurnAct},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROFILE: &str = "candidate-mechanisms-023-v1";
pub const FIXES: &str = "mechanisms023";
pub const FOOD: &str = "food_ceil023";
pub const HEAL: &str = "hero_heal_ceil023";
pub const SURPRISE: &str = "surprise_derangement023";
pub const ROUNDING: &str = "player_rounding023";
pub const DEBUFF: &str = "timed_debuff_stacks023";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidates {
    #[serde(default = "source_default")]
    pub source_sync: bool,
    #[serde(default)]
    pub player_favour_rounding: bool,
    pub food_ceil: bool,
    pub hero_heal_ceil: bool,
    pub surprise_derangement: bool,
    pub timed_debuff_stacks: bool,
}
fn source_default() -> bool {
    true
}
impl Default for Candidates {
    fn default() -> Self {
        Self {
            source_sync: true,
            player_favour_rounding: false,
            food_ceil: false,
            hero_heal_ceil: false,
            surprise_derangement: false,
            timed_debuff_stacks: false,
        }
    }
}
impl Candidates {
    pub fn legacy() -> Self {
        Self {
            source_sync: false,
            ..Self::default()
        }
    }
    pub fn documented() -> Self {
        Self {
            source_sync: true,
            player_favour_rounding: true,
            food_ceil: true,
            hero_heal_ceil: true,
            surprise_derangement: true,
            timed_debuff_stacks: true,
        }
    }
    pub fn apply(self, mut c: ContentConfig) -> Result<ContentConfig> {
        if !self.source_sync {
            if self.player_favour_rounding
                || self.food_ceil
                || self.hero_heal_ceil
                || self.surprise_derangement
                || self.timed_debuff_stacks
            {
                return Err("023 user candidates require the source-sync profile".into());
            }
            for key in [FIXES, FOOD, HEAL, SURPRISE, DEBUFF, ROUNDING] {
                c.syntax_choices.remove(key);
            }
            c.validate()?;
            return Ok(c);
        }
        c.syntax_choices.insert(FIXES.into(), PROFILE.into());
        for (key, yes) in [
            (ROUNDING, self.player_favour_rounding),
            (FOOD, self.food_ceil),
            (HEAL, self.hero_heal_ceil),
            (SURPRISE, self.surprise_derangement),
            (DEBUFF, self.timed_debuff_stacks),
        ] {
            if yes {
                c.syntax_choices
                    .insert(key.into(), "user-experience-v1".into());
            } else {
                c.syntax_choices.remove(key);
            }
        }
        c.validate()?;
        Ok(c)
    }
}
pub fn enabled(c: &ContentConfig, key: &str) -> bool {
    c.syntax_choices.contains_key(key)
}
/// Remove accumulated binary floating error within eight ulps at an integer.
/// This is used only by the explicit W029 candidate, not by old profiles.
pub fn stable_integer023(value: f64) -> f64 {
    let integer = value.round();
    if (value - integer).abs() <= 8. * f64::EPSILON * value.abs().max(1.) {
        integer
    } else {
        value
    }
}
pub fn ceil_final023(value: f64) -> f64 {
    stable_integer023(value).ceil()
}
pub fn floor_final023(value: f64) -> f64 {
    stable_integer023(value).floor()
}
pub fn food_amount(value: f64, c: &ContentConfig) -> i32 {
    if enabled(c, FOOD) {
        if enabled(c, ROUNDING) {
            ceil_final023(value)
        } else {
            value.ceil()
        }
    } else {
        value.round()
    }
    .clamp(0., 1e6) as i32
}
pub fn healing_amount(value: f64, hero: bool, c: &ContentConfig) -> i32 {
    if hero && enabled(c, HEAL) {
        if enabled(c, ROUNDING) {
            ceil_final023(value)
        } else {
            value.ceil()
        }
    } else {
        value.round()
    }
    .clamp(0., 1e6) as i32
}

pub type QuirkAct = crate::mechanics::TraitDef;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Library {
    quirk_act_outs: Vec<Row>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    quirk_id: String,
    combat_start_turn_act_outs: Vec<TurnAct>,
    reaction_act_outs: Vec<Reaction>,
}
pub fn parse_quirk_acts(bytes: &[u8]) -> Result<BTreeMap<String, QuirkAct>> {
    let lib: Library = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut result = BTreeMap::new();
    for row in lib.quirk_act_outs {
        if row.quirk_id.is_empty() || result.contains_key(&row.quirk_id) {
            return Err("duplicate/empty quirk act-out ID".into());
        }
        let trait_def = crate::mechanics::TraitDef {
            generate_chance_modifier: 0.,
            required_dungeon_history: vec![],
            id: row.quirk_id.clone(),
            virtue: false,
            buffs: vec![],
            turn_acts: row.combat_start_turn_act_outs,
            reactions: row.reaction_act_outs,
        };
        trait_def.validate()?;
        if trait_def.turn_acts.iter().all(|a| a.chance == 0.)
            && trait_def.reactions.iter().all(|r| r.chance == 0.)
        {
            continue;
        }
        result.insert(row.quirk_id, trait_def);
    }
    Ok(result)
}

pub fn load_quirk_acts(o: &crate::overlay::Overlay) -> Result<BTreeMap<String, QuirkAct>> {
    let mut all = BTreeMap::new();
    if !enabled(&o.config, FIXES) {
        return Ok(all);
    }
    for path in o
        .paths
        .keys()
        .filter(|p| p.starts_with("shared/quirk/") && p.ends_with("quirk_act_outs.json"))
    {
        for bytes in o.read_quirk_actouts023(path)? {
            for (id, act) in parse_quirk_acts(&bytes)? {
                if all.get(&id).is_some_and(|old| old != &act) {
                    return Err(format!("conflicting quirk act-out {id}"));
                }
                all.insert(id, act);
            }
        }
    }
    Ok(all)
}
