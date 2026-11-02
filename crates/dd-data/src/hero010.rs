//! Class state is parsed from original hero records, never selected by class ID.
use crate::combat::*;
use crate::darkest::{Document, Record};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroRules {
    /// Expedition passive sources (districts/current town event), excluded from
    /// temporary carry export so repeated field reconstruction cannot stack them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_passive_buffs: Vec<crate::mechanics::Buff>,
    /// Explicit expedition-owned buffs. Lifetime and physical provenance stay with
    /// the host; native dispels and maximum-HP changes still apply normally.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_buffs: Vec<HostBuff>,
    /// Explicit exploration effects entering this encounter, distinct from prior battle carry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exploration_buffs: Vec<CarryBuff>,
    #[serde(default)]
    pub difficulty: Option<crate::difficulty012::HeroDifficulty>,
    pub tags: Vec<String>,
    pub incompatible_party_tags: Vec<String>,
    pub incompatible_quirks: Vec<String>,
    pub carry: Option<HeroCarry>,
    pub dungeon_history: std::collections::BTreeSet<String>,
    pub resolve_history: std::collections::BTreeMap<String, u32>,
    pub fixed_skills: Vec<String>,
    pub override_defs: Vec<crate::mechanics::TraitDef>,
    pub initial_hp: Option<i32>,
    pub forced: std::collections::BTreeMap<String, Skill>,
    pub modes: Vec<Mode>,
    pub initial_mode: Option<String>,
    pub enter_door: Vec<Effect>,
    pub enter_cooldown: u32,
    pub low_hp: Vec<HpReaction>,
    pub death: Vec<DeathReaction>,
    pub resolve_override: Vec<(String, f64)>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HostBuff {
    pub instance: u64,
    pub definition: crate::mechanics::Buff,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    pub id: String,
    pub default: bool,
    pub stress: i32,
    pub affliction_skill: Option<String>,
    pub end_skill: Option<String>,
    pub start: Vec<Effect>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HpReaction {
    pub ratio: f64,
    pub under: bool,
    pub effects: Vec<Effect>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathReaction {
    pub allies: bool,
    pub enemies: bool,
    pub effects: Vec<Effect>,
}
fn flag(r: &Record, k: &str, default: bool) -> Result<bool> {
    match r.param(k) {
        None => Ok(default),
        Some(_) => match text(r, k)? {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("invalid hero flag {k}")),
        },
    }
}
fn known(r: &Record, keys: &[&str]) -> Result<()> {
    if let Some(p) = r.params.iter().find(|p| !keys.contains(&p.name.as_str())) {
        return Err(format!("unsupported hero {}.{}", r.key, p.name));
    }
    Ok(())
}
fn effects(g: &GameData, r: &Record, k: &str) -> Result<Vec<Effect>> {
    r.param(k)
        .map(|p| p.values.iter().map(|v| g.effect(&v.text)).collect())
        .unwrap_or(Ok(vec![]))
}
pub fn parse(g: &GameData, d: &Document) -> Result<HeroRules> {
    let mut h = HeroRules::default();
    for r in d.records_keyed("tag") {
        known(r, &["id"])?;
        h.tags.push(text(r, "id")?.to_owned());
    }
    for r in d.records_keyed("incompatible_party_member") {
        known(r, &["id", "hero_tag"])?;
        text(r, "id")?;
        h.incompatible_party_tags
            .push(text(r, "hero_tag")?.to_owned());
    }
    if let Some(r) = d.records_keyed("quirk_modifier").next() {
        known(r, &["incompatible_class_ids"])?;
        h.incompatible_quirks = r
            .param("incompatible_class_ids")
            .ok_or("missing class quirk restrictions")?
            .values
            .iter()
            .map(|v| v.text.clone())
            .collect();
    }

    if let Some(r) = d.records_keyed("skill_selection").next() {
        known(
            r,
            &[
                "can_select_combat_skills",
                "number_of_selected_combat_skills_max",
            ],
        )?;
        if !flag(r, "can_select_combat_skills", true)? {
            for s in d.records_keyed("combat_skill") {
                if number(s, "level")? == 0. {
                    h.fixed_skills.push(text(s, "id")?.to_string());
                }
            }
        }
    }

    for r in d.records_keyed("mode") {
        known(
            r,
            &[
                "id",
                "is_raid_default",
                "bark_override_id",
                "affliction_combat_skill_id",
                "battle_complete_combat_skill_id",
                "stress_damage_per_turn",
                "battle_start_effect_ids",
            ],
        )?;
        let id = text(r, "id")?.to_owned();
        if id.is_empty() || h.modes.iter().any(|m| m.id == id) {
            return Err("invalid/duplicate mode".into());
        }
        let stress = if r.param("stress_damage_per_turn").is_some() {
            number(r, "stress_damage_per_turn")?
        } else {
            0.
        };
        if stress.fract() != 0. || !(0. ..=1000.).contains(&stress) {
            return Err("invalid mode stress".into());
        }
        h.modes.push(Mode {
            id,
            default: flag(r, "is_raid_default", false)?,
            stress: stress as i32,
            affliction_skill: r
                .param("affliction_combat_skill_id")
                .map(|_| text(r, "affliction_combat_skill_id").map(str::to_owned))
                .transpose()?,
            end_skill: r
                .param("battle_complete_combat_skill_id")
                .map(|_| text(r, "battle_complete_combat_skill_id").map(str::to_owned))
                .transpose()?,
            start: effects(g, r, "battle_start_effect_ids")?,
        });
    }
    if !h.modes.is_empty() && !h.modes.iter().any(|m| m.default) {
        return Err("mode has no raid default".into());
    }
    if let Some(r) = d.records_keyed("deaths_door").next() {
        h.enter_door = effects(g, r, "enter_effects")?;
        let cooldown = r
            .param("enter_effect_round_cooldown")
            .map(|_| number(r, "enter_effect_round_cooldown"))
            .transpose()?
            .unwrap_or(0.);
        if cooldown.fract() != 0. || !(0. ..=10000.).contains(&cooldown) {
            return Err("invalid door reaction cooldown".into());
        }
        h.enter_cooldown = cooldown as u32;
    }
    for r in d.records_keyed("hp_reaction") {
        known(r, &["hp_ratio", "is_under", "effects"])?;
        let ratio = number(r, "hp_ratio")?;
        if !(0. ..=1.).contains(&ratio) {
            return Err("invalid HP reaction".into());
        }
        h.low_hp.push(HpReaction {
            ratio,
            under: flag(r, "is_under", true)?,
            effects: effects(g, r, "effects")?,
        });
    }
    for r in d.records_keyed("death_reaction") {
        known(r, &["target_allies", "target_enemies", "effects"])?;
        h.death.push(DeathReaction {
            allies: flag(r, "target_allies", false)?,
            enemies: flag(r, "target_enemies", false)?,
            effects: effects(g, r, "effects")?,
        });
    }
    if let Some(r) = d.records_keyed("overstressed_modifier").next() {
        known(
            r,
            &["override_trait_type_ids", "override_trait_type_chances"],
        )?;
        let ids = &r
            .param("override_trait_type_ids")
            .ok_or("missing resolve override")?
            .values;
        let weights = &r
            .param("override_trait_type_chances")
            .ok_or("missing resolve weights")?
            .values;
        if ids.len() != weights.len() || ids.is_empty() {
            return Err("resolve override weights mismatch".into());
        }
        for (id, w) in ids.iter().zip(weights) {
            let n = scalar(&w.text)?;
            if n < 0. {
                return Err("invalid resolve weight".into());
            }
            h.resolve_override.push((id.text.clone(), n));
        }
    }
    if !h.resolve_override.is_empty()
        && h.resolve_override.iter().map(|(_, w)| w).sum::<f64>() <= 0.
    {
        return Err("empty resolve override distribution".into());
    }
    if !h.resolve_override.is_empty() {
        let defs = h
            .resolve_override
            .iter()
            .map(|(id, _)| crate::library::unique_json(&g.traits, id, "class trait").cloned())
            .collect::<Result<Vec<_>>>()?;
        let rules = g.overlay.json("shared/rules.json")?;
        h.override_defs = crate::mechanics::CombatGlobals::load_explicit(
            &rules,
            &g.buffs,
            &serde_json::json!({"traits":defs}),
        )?
        .traits;
    }
    Ok(h)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CarryBuff {
    pub definition: crate::mechanics::Buff,
    pub remaining: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CarryResolve {
    Calm,
    Afflicted(String),
    Virtuous(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeroCarry {
    #[serde(default)]
    pub status: CarryStatus,
    pub stress: i32,
    pub resolve: CarryResolve,
    pub buffs: Vec<CarryBuff>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CarryStatus {
    pub dots: Vec<CarryDot>,
    pub burn: i32,
    /// Per-recipient sequential supply history, carried until the host starts a new raid.
    pub item_uses: std::collections::BTreeMap<String, u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CarryDot {
    pub kind: CarryDotKind,
    pub amount: i32,
    pub remaining: u32,
    pub can_crit_heal: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CarryDotKind {
    Bleed,
    Blight,
    Horror,
    Regen,
}
impl CarryStatus {
    pub fn validate(&self) -> Result<()> {
        if !(0..=1000000).contains(&self.burn)
            || self.dots.len() > 1000
            || self.dots.iter().any(|d| {
                !(0..=1000000).contains(&d.amount)
                    || !(1..=10000).contains(&d.remaining)
                    || (d.can_crit_heal && d.kind != CarryDotKind::Regen)
            })
            || self.item_uses.len() > 1000
            || self.item_uses.iter().any(|(id, n)| {
                id.is_empty()
                    || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    || *n > 1000000
            })
        {
            return Err("invalid carried battle status".into());
        }
        Ok(())
    }
}
