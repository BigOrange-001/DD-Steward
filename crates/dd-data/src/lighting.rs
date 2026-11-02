//! Battle darkness table. Visual lighting files are deliberately not read.
use crate::{
    combat::{GameData, Result},
    mechanics::{Buff, RuleData},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub torch: u8,
    pub table: Darkness,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Darkness {
    pub lowest_excluded: bool,
    pub range_table: Vec<Band>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub range: Bounds,
    pub value: LightValue,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub lower: f64,
    pub upper: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LightValue {
    pub stress_chance_increase: f64,
    pub stress_damage_increase: f64,
    pub monster_attack_increase: f64,
    pub monster_damage_increase: f64,
    pub monster_crit_increase: f64,
    pub loot_increase_gold: f64,
    pub loot_increase_treasure_draws: f64,
    pub player_crit_increase: f64,
    pub player_def_increase: f64,
    pub player_scouting_increase: f64,
    pub monsters_surprised_increase: f64,
    pub heroes_surprised_increase: f64,
}
impl Lighting {
    pub fn validate(&self) -> Result<()> {
        if self.torch > 100
            || self.table.range_table.is_empty()
            || self.table.range_table.len() > 100
        {
            return Err("invalid battle lighting".into());
        }
        for band in &self.table.range_table {
            if !band.range.lower.is_finite()
                || !band.range.upper.is_finite()
                || band.range.lower >= band.range.upper
            {
                return Err("invalid darkness range".into());
            }
            let values = serde_json::to_value(&band.value).map_err(|e| e.to_string())?;
            if values
                .as_object()
                .unwrap()
                .values()
                .any(|v| v.as_f64().is_none_or(|n| !n.is_finite() || n.abs() > 1e6))
            {
                return Err("invalid darkness modifier".into());
            }
        }
        // Check all range intersections, including intervals between integer
        // torch values, so a malformed table cannot conceal overlap or gaps.
        let mut bands = self.table.range_table.iter().collect::<Vec<_>>();
        bands.sort_by(|a, b| a.range.lower.total_cmp(&b.range.lower));
        if bands[0].range.lower >= 0.
            || bands.last().unwrap().range.upper < 100.
            || bands
                .windows(2)
                .any(|pair| pair[0].range.upper != pair[1].range.lower)
        {
            return Err("ambiguous or incomplete darkness table".into());
        }
        for torch in 0..=100 {
            self.at(torch)?;
        }
        Ok(())
    }
    pub fn at(&self, torch: u8) -> Result<&LightValue> {
        let n = f64::from(torch);
        let mut found = self.table.range_table.iter().filter(|b| {
            if self.table.lowest_excluded {
                n > b.range.lower && n <= b.range.upper
            } else {
                n >= b.range.lower && n < b.range.upper
            }
        });
        let value = found.next().ok_or("missing darkness band")?;
        if found.next().is_some() {
            return Err("ambiguous darkness band".into());
        }
        Ok(&value.value)
    }
    pub fn buffs(&self, hero: bool) -> Result<Vec<Buff>> {
        let v = self.at(self.torch)?;
        let rows = if hero {
            vec![
                (
                    "combat_stat_add",
                    "defense_rating",
                    v.player_def_increase / 100.,
                ),
                (
                    "combat_stat_add",
                    "crit_chance",
                    v.player_crit_increase / 100.,
                ),
                (
                    "stress_dmg_received_percent",
                    "",
                    v.stress_damage_increase / 100.,
                ),
            ]
        } else {
            vec![
                (
                    "combat_stat_add",
                    "attack_rating",
                    v.monster_attack_increase / 100.,
                ),
                (
                    "combat_stat_add",
                    "crit_chance",
                    v.monster_crit_increase / 100.,
                ),
                (
                    "combat_stat_multiply",
                    "damage_low",
                    v.monster_damage_increase / 100.,
                ),
                (
                    "combat_stat_multiply",
                    "damage_high",
                    v.monster_damage_increase / 100.,
                ),
            ]
        };
        rows.into_iter()
            .map(|(kind, sub, amount)| {
                let b = Buff {
                    activation: Default::default(),
                    id: format!("darkness:{kind}:{sub}"),
                    stat_type: kind.into(),
                    stat_sub_type: sub.into(),
                    amount,
                    duration: None,
                    duration_type: None,
                    rule_type: "always".into(),
                    is_false_rule: false,
                    rule_data: RuleData::default(),
                    clearable: false,
                    force_positive: false,
                };
                b.validate()?;
                Ok(b)
            })
            .collect()
    }
}
impl GameData {
    pub fn lighting(&self, torch: u8) -> Result<Lighting> {
        let raw = self.overlay.json("shared/rules.json")?;
        let table = serde_json::from_value(
            raw.get("darkness")
                .cloned()
                .ok_or("missing darkness table")?,
        )
        .map_err(|e| format!("darkness: {e}"))?;
        let light = Lighting { torch, table };
        light.validate()?;
        Ok(light)
    }
}
