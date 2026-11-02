//! Original wave battle inputs only. No quest progression, maps or curio execution.
use crate::{
    combat::*,
    lighting::Darkness,
    mechanics::{Buff, read_buff},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub contract: crate::contract011::Contract,
    pub id: String,
    pub tier: u8,
    pub source: String,
    pub params: Params,
    pub tables: BTreeMap<String, Table>,
    pub lights: BTreeMap<String, Light>,
    pub darkness: Darkness,
    pub eligible_quests: BTreeSet<String>,
    pub reflections: Vec<Reflection>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reflection {
    pub id: String,
    pub threshold: u32,
    pub hero_buffs: Vec<Buff>,
    pub monster_buffs: Vec<Buff>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Params {
    #[serde(default)]
    pub curio_groups: Vec<Curios>,
    #[serde(default)]
    pub background_groups: Vec<Backgrounds>,
    pub intro_wave_groups: Vec<Group>,
    #[serde(default)]
    pub recurring_wave_groups: Vec<Group>,
    #[serde(default)]
    pub space_miniboss_chance: f64,
    #[serde(default = "one")]
    pub threshold_shard_reward_multiplier: f64,
    pub single_shard_rewards: [u32; 2],
    pub double_shard_rewards: [u32; 2],
    pub boss_shard_rewards: [u32; 2],
}
fn one() -> f64 {
    1.
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Curios {
    pub name: String,
    pub curios: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Backgrounds {
    pub name: String,
    pub backgrounds: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub wave_infos: Vec<Wave>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    #[serde(rename = "killRange")]
    pub kills: [u32; 2],
    #[serde(default)]
    pub curio: String,
    #[serde(default, rename = "curioNames")]
    pub curio_names: Vec<String>,
    #[serde(default)]
    pub background: String,
    #[serde(default, rename = "type")]
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Formation {
    pub weight: f64,
    pub monsters: Vec<String>,
    pub background: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub curios: Vec<String>,
    pub backgrounds: Vec<String>,
    pub curio_background: Option<String>,
    pub source: String,
    pub mash: Vec<Formation>,
    pub bosses: Vec<Formation>,
    pub minibosses: Vec<Formation>,
    pub local_minibosses: Vec<Formation>,
    pub kill_exceptions: Vec<String>,
    pub crystal: Option<String>,
    pub gate: Option<String>,
    pub stall: Option<String>,
    pub front_chance: Option<f64>,
    pub random_chance: f64,
    pub recurring_only: bool,
    pub requirements: Vec<String>,
    pub ease: Option<String>,
    pub starting_curio: Option<String>,
    pub starting_bg: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Light {
    pub id: String,
    pub hero_buffs: Vec<Buff>,
    pub monster_buffs: Vec<Buff>,
    pub hero_effects: Vec<Effect>,
    pub monster_effects: Vec<Effect>,
    pub regions: Vec<String>,
}
impl Plan {
    pub fn validate(&self) -> Result<()> {
        self.contract.validate()?;
        if self.contract.id != self.id {
            return Err("farm contract/profile mismatch".into());
        }
        if ![1, 3, 6].contains(&self.tier)
            || self.params.intro_wave_groups.is_empty()
            || !(0. ..=1.).contains(&self.params.space_miniboss_chance)
            || !self.params.threshold_shard_reward_multiplier.is_finite()
        {
            return Err("invalid farm plan".into());
        }
        for range in [
            self.params.single_shard_rewards,
            self.params.double_shard_rewards,
            self.params.boss_shard_rewards,
        ] {
            if range[0] > range[1] || range[1] > 1000000 {
                return Err("invalid shard reward range".into());
            }
        }
        for g in self
            .params
            .intro_wave_groups
            .iter()
            .chain(&self.params.recurring_wave_groups)
        {
            if g.wave_infos.is_empty() || g.wave_infos.len() > 64 {
                return Err("invalid wave group".into());
            }
            for w in &g.wave_infos {
                if w.kills[0] == 0
                    || w.kills[0] > w.kills[1]
                    || w.kills[1] > 1000000
                    || !["", "space", "boss", "miniboss"].contains(&w.kind.as_str())
                {
                    return Err("unknown wave type or quota".into());
                }
            }
        }
        if !self.tables.contains_key("farm") {
            return Err("missing farm wave table".into());
        }
        for t in self.tables.values() {
            if t.mash.is_empty()
                || !t.random_chance.is_finite()
                || t.random_chance < 0.
                || t.front_chance.is_some_and(|n| !(0. ..=1.).contains(&n))
            {
                return Err("invalid wave encounter pool".into());
            }
            for f in t
                .mash
                .iter()
                .chain(&t.bosses)
                .chain(&t.minibosses)
                .chain(&t.local_minibosses)
            {
                if f.monsters.is_empty()
                    || f.monsters.len() > 4
                    || !f.weight.is_finite()
                    || f.weight <= 0.
                {
                    return Err("invalid wave formation".into());
                }
            }
        }
        for r in &self.reflections {
            if r.threshold == 0 || r.threshold > 1000000 {
                return Err("invalid reflection threshold".into());
            }
            for b in r.hero_buffs.iter().chain(&r.monster_buffs) {
                b.validate()?;
            }
        }
        for l in self.lights.values() {
            for b in l.hero_buffs.iter().chain(&l.monster_buffs) {
                b.validate()?;
            }
        }
        crate::lighting::Lighting {
            torch: 100,
            table: self.darkness.clone(),
        }
        .validate()?;
        Ok(())
    }
    pub fn monster_ids(&self) -> Vec<&str> {
        let mut ids = BTreeSet::new();
        for t in self.tables.values() {
            for f in t
                .mash
                .iter()
                .chain(&t.bosses)
                .chain(&t.minibosses)
                .chain(&t.local_minibosses)
            {
                ids.extend(f.monsters.iter().map(String::as_str));
            }
            ids.extend(
                t.crystal
                    .iter()
                    .chain(&t.gate)
                    .chain(&t.stall)
                    .map(String::as_str),
            );
        }
        ids.into_iter().collect()
    }
}
// Same path may contain additive keyed tables. Resolve the requested key, never a file winner.
impl GameData {
    fn battle_json_versions(&self, path: &str) -> Result<Vec<Value>> {
        if ![
            "scripts/raid_settings.json",
            "campaign/quest/com.quest.plot_quests.json",
            "campaign/quest/com.quest.modifiers.json",
        ]
        .contains(&path)
        {
            return Err("outside wave battle input scope".into());
        }
        let mut versions = vec![];
        for index in self.overlay.sources(path) {
            let file = self.overlay.layers[index].root.join(path);
            let raw = crate::telemetry024c::read(&file)
                .map_err(|e| format!("{}: {e}", file.display()))?;
            let v: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
            if !versions.contains(&v) {
                versions.push(v);
            }
        }
        if versions.is_empty() {
            return Err(format!("missing battle input {path}"));
        }
        Ok(versions)
    }
    pub(crate) fn raid_entry(&self, table: &str, key: &str) -> Result<Value> {
        let mut found = vec![];
        for v in self.battle_json_versions("scripts/raid_settings.json")? {
            for r in v[table].as_array().ok_or("missing raid settings table")? {
                if r["key"] == key {
                    let value = r["data"].clone();
                    if !found.contains(&value) {
                        found.push(value);
                    }
                }
            }
        }
        if found.len() != 1 {
            return Err(format!("missing or ambiguous raid setting {table}/{key}"));
        }
        Ok(found.remove(0))
    }
    pub fn farm_plan(&self, id: &str, eligible_quests: BTreeSet<String>) -> Result<Plan> {
        if !self
            .overlay
            .config
            .packages
            .iter()
            .any(|p| p == "color_of_madness")
        {
            return Err("farm battle requires enabled color_of_madness".into());
        }
        if ![
            "plot_color_of_madness_0",
            "plot_color_of_madness_1",
            "plot_color_of_madness_2",
        ]
        .contains(&id)
        {
            return Err("unknown farm battle profile".into());
        }
        let path = "campaign/quest/com.quest.plot_quests.json";
        let mut defs = vec![];
        for v in self.battle_json_versions(path)? {
            for row in v["plot_quests"]
                .as_array()
                .ok_or("missing wave input library")?
            {
                if row["id"] == id {
                    let q = &row["quest"];
                    let definition = (
                        q["difficulty"].clone(),
                        q["wave_params"].clone(),
                        row.get("potential_quest_modifiers")
                            .cloned()
                            .unwrap_or(serde_json::json!([])),
                    );
                    if !defs.contains(&definition) {
                        defs.push(definition);
                    }
                }
            }
        }
        if defs.len() != 1 {
            return Err("missing or ambiguous farm wave parameters".into());
        }
        let (tier, raw, reflection_ids) = defs.remove(0);
        let tier = tier
            .as_u64()
            .and_then(|n| u8::try_from(n).ok())
            .ok_or("invalid farm difficulty")?;
        let params: Params =
            serde_json::from_value(raw).map_err(|e| format!("wave_params: {e}"))?;
        let mut lights = BTreeMap::new();
        for name in [
            "wave_farm",
            "wave_pre_time_and_space",
            "wave_transition_a",
            "wave_transition_b",
            "wave_transition_c",
            "wave_transition_d",
        ] {
            let v = self.raid_entry("torch_settings_data_table", name)?;
            let known = [
                "id",
                "burn_down_enabled",
                "hero_buff_list",
                "monster_buff_list",
                "hero_effect_list",
                "monster_effect_list",
                "wave_transition_dungeons",
                "overlays",
                "overlay_custom_animation",
                "custom_colour_grade",
                "custom_quest_modifier_texture_id",
                "transition_id",
                "transition_sprite",
                "transition_intro_audio_event",
                "transition_result_audio_event",
                "torch_setting_item_buffs",
                "torch_setting_item_performer_effects",
                "torch_music_override",
                "torch_music_parameters",
                "torch_ambience_parameters",
            ];
            if v.as_object()
                .ok_or("invalid wave lighting")?
                .keys()
                .any(|k| !known.contains(&k.as_str()))
                || v["burn_down_enabled"] != false
            {
                return Err("unknown wave lighting field or dynamic light".into());
            }
            for key in [
                "torch_setting_item_buffs",
                "torch_setting_item_performer_effects",
            ] {
                if v[key].as_array().is_none_or(|a| !a.is_empty()) {
                    return Err("unsupported wave light item override".into());
                }
            }
            let strings = |key: &str| -> Result<Vec<String>> {
                if v.get(key).is_none() {
                    return Ok(vec![]);
                }
                v[key]
                    .as_array()
                    .ok_or("invalid light list")?
                    .iter()
                    .map(|s| {
                        s.as_str()
                            .map(str::to_string)
                            .ok_or("invalid light reference".into())
                    })
                    .collect()
            };
            lights.insert(
                name.into(),
                Light {
                    id: name.into(),
                    hero_buffs: strings("hero_buff_list")?
                        .iter()
                        .map(|id| read_buff(&self.buffs, id))
                        .collect::<Result<_>>()?,
                    monster_buffs: strings("monster_buff_list")?
                        .iter()
                        .map(|id| read_buff(&self.buffs, id))
                        .collect::<Result<_>>()?,
                    hero_effects: strings("hero_effect_list")?
                        .iter()
                        .map(|id| self.effect(id))
                        .collect::<Result<_>>()?,
                    monster_effects: strings("monster_effect_list")?
                        .iter()
                        .map(|id| self.effect(id))
                        .collect::<Result<_>>()?,
                    regions: strings("wave_transition_dungeons")?,
                },
            );
        }
        let darkness: Darkness = serde_json::from_value(
            self.raid_entry("raid_rules_override_data_table", "wave")?["darkness"].clone(),
        )
        .map_err(|e| format!("wave darkness: {e}"))?;
        let mut tables = BTreeMap::new();
        for region in [
            "farm",
            "cove",
            "crypts",
            "warrens",
            "weald",
            "darkestdungeon",
            "town",
        ] {
            let p = format!("dungeons/{region}/{region}.{tier}.wavemash.darkest");
            if region != "farm"
                && !params
                    .intro_wave_groups
                    .iter()
                    .chain(&params.recurring_wave_groups)
                    .flat_map(|g| &g.wave_infos)
                    .any(|w| w.kind == "space")
            {
                continue;
            }
            if self.overlay.sources(&p).is_empty() {
                continue;
            }
            tables.insert(region.into(), self.wave_table(&p)?);
        }
        let mut reflections = vec![];
        for id in reflection_ids
            .as_array()
            .ok_or("invalid reflection selection")?
        {
            let id = id.as_str().ok_or("invalid reflection ID")?;
            let mut rows = vec![];
            for v in self.battle_json_versions("campaign/quest/com.quest.modifiers.json")? {
                for row in v["modifiers"]
                    .as_array()
                    .ok_or("missing battle modifier library")?
                {
                    if row["id"] == id && !rows.contains(row) {
                        rows.push(row.clone());
                    }
                }
            }
            if rows.len() != 1 {
                return Err("missing or ambiguous wave reflection".into());
            }
            let row = rows.remove(0);
            if row.as_object().is_none_or(|m| {
                m.keys().any(|k| {
                    ![
                        "id",
                        "condition_type",
                        "condition_number_data",
                        "hero_buffs",
                        "monster_buffs",
                    ]
                    .contains(&k.as_str())
                })
            }) || row["condition_type"] != "wave_kill_count"
            {
                return Err("unsupported wave reflection condition".into());
            }
            let buffs = |key: &str| -> Result<Vec<Buff>> {
                row[key]
                    .as_array()
                    .ok_or("missing reflection buff list")?
                    .iter()
                    .map(|v| {
                        read_buff(&self.buffs, v.as_str().ok_or("invalid reflection buff ID")?)
                    })
                    .collect()
            };
            reflections.push(Reflection {
                id: id.into(),
                threshold: row["condition_number_data"]
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("invalid reflection threshold")?,
                hero_buffs: buffs("hero_buffs")?,
                monster_buffs: buffs("monster_buffs")?,
            });
        }
        let plan = Plan {
            contract: self.battle_contract(id)?,
            id: id.into(),
            tier,
            source: path.into(),
            params,
            tables,
            lights,
            darkness,
            eligible_quests,
            reflections,
        };
        plan.validate()?;
        Ok(plan)
    }
    fn wave_table(&self, path: &str) -> Result<Table> {
        let d = crate::library::read_document(&self.overlay, path)?;
        let mut t = Table {
            curios: vec![],
            backgrounds: vec![],
            curio_background: None,
            source: path.into(),
            mash: vec![],
            bosses: vec![],
            minibosses: vec![],
            local_minibosses: vec![],
            kill_exceptions: vec![],
            crystal: None,
            gate: None,
            stall: None,
            front_chance: None,
            random_chance: 1.,
            recurring_only: false,
            requirements: vec![],
            ease: None,
            starting_bg: None,
            starting_curio: None,
        };
        let mut single = BTreeSet::new();
        for r in d.records {
            let list = |key: &str| -> Result<Vec<String>> {
                Ok(r.param(key)
                    .ok_or("missing wave record list")?
                    .values
                    .iter()
                    .map(|v| v.text.clone())
                    .collect())
            };
            let allowed: &[&str] = match r.key.as_str() {
                "mash" | "boss" | "miniboss" | "localminiboss" => {
                    &["chance", "types", "background"]
                }
                "killexception" | "crystal" | "stall" | "teleport" | "curios" | "backgrounds" => {
                    &["types"]
                }
                "preferred_spawn_position_chance" | "prefered_spawn_position_chance" => &["chance"],
                "miniboss_chance_ease_func"
                | "curio_background"
                | "starting_bg"
                | "starting_curio" => &["type"],
                "requirements" => &["quests"],
                "specifications" => &["random_chance", "recurring_only"],
                _ => return Err(format!("unknown wave table record {}", r.key)),
            };
            if r.params.iter().any(|p| !allowed.contains(&p.name.as_str())) {
                return Err(format!("unknown wave table field {}", r.key));
            }
            if !["mash", "boss", "miniboss", "localminiboss"].contains(&r.key.as_str())
                && !single.insert(r.key.clone())
            {
                return Err("duplicate wave singleton".into());
            }
            match r.key.as_str() {
                "mash" | "boss" | "miniboss" | "localminiboss" => {
                    let f = Formation {
                        weight: r
                            .param("chance")
                            .map(|_| number(&r, "chance"))
                            .transpose()?
                            .unwrap_or(1.),
                        monsters: list("types")?,
                        background: r
                            .param("background")
                            .map(|_| text(&r, "background").map(str::to_string))
                            .transpose()?,
                    };
                    match r.key.as_str() {
                        "mash" => t.mash.push(f),
                        "boss" => t.bosses.push(f),
                        "miniboss" => t.minibosses.push(f),
                        _ => t.local_minibosses.push(f),
                    }
                }
                "crystal" | "stall" | "teleport" => {
                    let ids = list("types")?;
                    if ids.len() != 1 {
                        return Err("invalid wave special entity".into());
                    }
                    match r.key.as_str() {
                        "crystal" => t.crystal = Some(ids[0].clone()),
                        "stall" => t.stall = Some(ids[0].clone()),
                        _ => t.gate = Some(ids[0].clone()),
                    }
                }
                "killexception" => t.kill_exceptions = list("types")?,
                "preferred_spawn_position_chance" | "prefered_spawn_position_chance" => {
                    if t.front_chance.is_some() {
                        return Err("duplicate wave placement chance".into());
                    }
                    t.front_chance = Some(number(&r, "chance")?);
                }
                "specifications" => {
                    t.random_chance = number(&r, "random_chance")?;
                    t.recurring_only = r
                        .param("recurring_only")
                        .map(|_| flag(&r, "recurring_only", false))
                        .transpose()?
                        .unwrap_or(false);
                }
                "requirements" => t.requirements = list("quests")?,
                "miniboss_chance_ease_func" => {
                    let v = text(&r, "type")?;
                    if v != "easeSqrt" {
                        return Err("unknown wave miniboss easing".into());
                    }
                    t.ease = Some(v.into());
                }
                "starting_bg" => t.starting_bg = Some(text(&r, "type")?.into()),
                "starting_curio" => t.starting_curio = Some(text(&r, "type")?.into()),
                "curio_background" => {
                    t.curio_background = Some(text(&r, "type")?.into());
                }
                "curios" => {
                    t.curios = list("types")?;
                }
                "backgrounds" => {
                    t.backgrounds = list("types")?;
                }
                _ => unreachable!(),
            }
        }
        Ok(t)
    }
}
