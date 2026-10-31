//! Crimson Court battle light and its original item override.
use crate::{
    combat::*,
    lighting::{Darkness, Lighting},
    mechanics::{Buff, read_buff},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Bloodlight {
    pub id: String,
    pub heroes: Vec<Buff>,
    pub torch_buffs: Vec<Buff>,
    pub darkness: Darkness,
}
impl Bloodlight {
    pub fn validate(&self) -> Result<()> {
        if self.id != "bloodlight" {
            return Err("unknown special battle light".into());
        }
        for b in self.heroes.iter().chain(&self.torch_buffs) {
            b.validate()?;
        }
        Lighting {
            torch: 100,
            table: self.darkness.clone(),
        }
        .validate()
    }
}
impl GameData {
    pub fn bloodlight(&self) -> Result<Bloodlight> {
        if !self
            .overlay
            .config
            .packages
            .iter()
            .any(|p| p == "crimson_court")
        {
            return Err("bloodlight requires crimson_court".into());
        }
        let raw = self.raid_entry("torch_settings_data_table", "bloodlight")?;
        if raw.as_object().is_none_or(|m| {
            m.keys().any(|k| {
                ![
                    "burn_down_enabled",
                    "hero_buff_list",
                    "overlays",
                    "torch_setting_item_buffs",
                ]
                .contains(&k.as_str())
            })
        }) || raw["burn_down_enabled"] != false
        {
            return Err("unknown bloodlight field".into());
        }
        let buffs = |v: &serde_json::Value| -> Result<Vec<Buff>> {
            v.as_array()
                .ok_or("invalid bloodlight buff list")?
                .iter()
                .map(|v| read_buff(&self.buffs, v.as_str().ok_or("invalid bloodlight buff ID")?))
                .collect()
        };
        let items = raw["torch_setting_item_buffs"]
            .as_array()
            .ok_or("missing bloodlight item override")?;
        if items.len() != 1 {
            return Err("ambiguous bloodlight item override".into());
        }
        let item = &items[0];
        if item.as_object().is_none_or(|m| {
            m.keys().any(|k| {
                ![
                    "item_type",
                    "item_name",
                    "hero_buff_list",
                    "monster_buff_list",
                ]
                .contains(&k.as_str())
            })
        }) || item["item_type"] != "supply"
            || item["item_name"] != "torch"
            || item["monster_buff_list"]
                .as_array()
                .is_none_or(|m| !m.is_empty())
        {
            return Err("unsupported bloodlight item target".into());
        }
        let v = Bloodlight {
            id: "bloodlight".into(),
            heroes: buffs(&raw["hero_buff_list"])?,
            torch_buffs: buffs(&item["hero_buff_list"])?,
            darkness: serde_json::from_value(
                self.raid_entry("raid_rules_override_data_table", "cc")?["darkness"].clone(),
            )
            .map_err(|e| e.to_string())?,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn bloodlight_torch(&self) -> Result<crate::items::Supply> {
        let b = self.bloodlight()?;
        let child = Effect {
            execution: Default::default(),
            name: "bloodlight_torch_party".into(),
            kind: EffectKind::Boss(crate::special::BossOp::PartyStackBuffs(b.torch_buffs)),
            chance: 1.,
            duration: 1,
            target: EffectTarget::Party,
            on_hit: true,
            on_miss: true,
            apply_once: true,
            resist_debuff: false,
            conditions: vec![],
            can_apply_on_death: false,
        };
        let mut effect = child;
        effect.target = EffectTarget::Performer;
        self.supply_with_override("torch", vec![effect])
    }
}

impl GameData {
    pub fn raid_torch_burn_enabled(&self, setting: &str) -> Result<bool> {
        if setting == "bloodlight" {
            self.bloodlight()?.validate()?;
            return Ok(false);
        }
        if !["default", "dd4"].contains(&setting) {
            return Err(format!("unsupported expedition torch setting {setting}"));
        }
        let raw = self.raid_entry("torch_settings_data_table", setting)?;
        crate::campaign017::fields(&raw, &["burn_down_enabled", "overlays"], "torch setting")?;
        if raw["overlays"]
            .as_array()
            .is_none_or(|a| a.iter().any(|v| !v.is_string()))
        {
            return Err("invalid cosmetic torch overlay list".into());
        }
        raw["burn_down_enabled"]
            .as_bool()
            .ok_or("missing source torch burn flag".into())
    }
}
