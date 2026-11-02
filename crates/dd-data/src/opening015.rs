//! Narrow data boundary for the connected original opening expeditions.
use crate::{
    combat::{Blueprint, GameData, Result, integer, text},
    curios013::{CurioEffect, Curios, Outcome},
    inventory015::*,
    library,
    maps014::Map,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub const PROFILE: &str = "candidate-expedition-015";
pub fn source_path(p: &str) -> bool {
    crate::initial013::source_path(p)
        || crate::maps014::source_path(p)
        || [
            "scripts/starting_save/persist.roster.json",
            "scripts/starting_save/persist.estate.json",
            "campaign/town/buildings/stage_coach/stage_coach.building.json",
            "campaign/provision/provision.json",
            "shared/rules.json",
            "props/prop_definitions.json",
            "props/trap_definitions.json",
            "props/obstacle_definitions.json",
            "campaign/roster/roster.variables.json",
        ]
        .contains(&p)
        || (p.starts_with("inventory/") && p.ends_with(".darkest"))
        || (p.starts_with("campaign/estate/") && p.ends_with(".json"))
        || (p.starts_with("dungeons/") && p.ends_with(".props.darkest"))
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Candidates {
    pub version: String,
    pub semantics: BTreeMap<String, String>,
    pub source: String,
    pub light_new: BTreeMap<String, u8>,
    pub light_seen: u8,
    pub hunger_stress: i32,
    pub manual_food_limit: u32,
    pub abandon_stress: i32,
    pub abandon_resolve_check: bool,
    pub torch_amount: i32,
    pub none_loot_empty_count: String,
    pub town_boundary: String,
    pub old_road_named_mashes: BTreeMap<String, String>,
    pub recruit_generation: String,
    pub old_road_death_replacement: String,
}
impl Candidates {
    pub fn documented() -> Result<Self> {
        serde_json::from_str(r#"{
  "version": "opening-candidates-015-v1",
  "source": "https://darkestdungeon.wiki.gg/wiki/Light_meter; https://darkestdungeon.wiki.gg/wiki/Hunger; https://darkestdungeon.wiki.gg/wiki/Old_Road; https://darkestdungeon.wiki.gg/wiki/Retreating; community-only fullness hypothesis",
  "light_new": {
    "base": 6,
    "radiant": 6,
    "new_game_plus": 7,
    "bloodmoon": 7
  },
  "light_seen": 1,
  "hunger_stress": 20,
  "torch_amount": 25,
  "town_boundary": "heal_hp_clear_temporary_keep_identity_stress_quirks_v1",
  "old_road_named_mashes": {
    "0": "tutorial_1",
    "1": "tutorial_2"
  },
  "recruit_generation": "source_count_uniform_skills_weighted_compatible_quirks_v1",
  "old_road_death_replacement": "enabled_class_deck_v1",
  "manual_food_limit": 4,
  "abandon_stress": 20,
  "abandon_resolve_check": false,
  "none_loot_empty_count": "explicit_inactive_none_v1",
  "semantics": {
    "old_road_missing_flags": "scouting_surprise_stall_retreat_disabled_v1",
    "field_ticks": "one_dot_tick_each_tile_no_combat_start_triggers_v1",
    "corridor_return": "battle_hunger_trap_independent_in_source_order_v1",
    "hunger_buffer": "source_room_buffer_decrement_on_corridor_entry_rearm_hunger_when_zero_v1",
    "conditional_encounters": "canonical_first_passing_override_saved_once_limits_per_entry_v1",
    "can_be_ambush": "camping_ambush_only_opening_has_no_camp_v1",
    "retreat": "same_formation_full_enemy_reset_no_loot_v1",
    "backstep": "explicit_backward_action_toward_previous_room_forward_turn_available_v1",
    "party_stack_bonus": "fixed_for_provisioned_party_until_return_including_dead_heroes_v1",
    "fractional_field_values": "nearest_integer_nonnegative_consumption_ceil_v1",
    "quest_quirks": "retain_existing_no_campaign_random_gain_v1",
    "non_inventory_collectibles": "trinkets_and_journal_single_slot_v1",
    "exploration_buffs": "distinct_pre_encounter_effects_applied_before_initiative_not_previous_battle_carry_v1",
    "abandonment_inventory": "original_keep_rates_quantity_floor_per_item_total_v1"
  }
}
"#)
            .map_err(|e| e.to_string())
    }
    pub fn validate(&self) -> Result<()> {
        if self != &Self::documented()? {
            return Err("unknown/modified opening candidates".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeroSeed {
    pub guid: u64,
    pub name: String,
    pub blueprint: Blueprint,
    pub xp: u32,
    pub source: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Formation {
    pub monsters: Vec<Blueprint>,
    pub loot: Vec<crate::curios013::Draw>,
    pub sources: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hazard {
    pub health: f64,
    pub light: i32,
    pub success: Vec<CurioEffect>,
    pub failure: Vec<CurioEffect>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub map: Map,
    pub quest: Value,
    pub goal: Value,
    pub formations: BTreeMap<String, Formation>,
    pub pools: crate::opening_spawn015::Pools,
    pub traps: BTreeMap<String, Hazard>,
    pub default_trap: Hazard,
    pub obstacle: Hazard,
    pub effects: BTreeMap<String, CurioEffect>,
    pub curio_types: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ClassExtras {
    pub limits: BTreeMap<String, u32>,
    pub battle: Vec<crate::curios013::Draw>,
    pub curio: Vec<crate::curios013::Draw>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub candidates: Candidates,
    pub content: crate::content::ContentConfig,
    pub levels: Vec<Level>,
    pub initial: Vec<HeroSeed>,
    pub roster_source: Value,
    pub raid_source: Value,
    pub recruit_classes: Vec<String>,
    pub recruit_source: Value,
    pub provision: Value,
    pub roster_variables: Value,
    pub wallet: BTreeMap<String, u32>,
    pub bag_rules: InventoryRules,
    pub class_extras: BTreeMap<String, ClassExtras>,
    pub return_rates: BTreeMap<String, Value>,
    pub estate_sources: BTreeMap<String, Vec<Value>>,
    pub curios: Curios,
    pub globals: Value,
    pub quirk_defs: BTreeMap<String, Value>,
    pub rules: crate::combat::Rules,
    pub next_guid: u64,
}
fn read(g: &GameData, p: &str) -> Result<Value> {
    serde_json::from_slice(&g.overlay.read_opening(p)?).map_err(|e| format!("{p}: {e}"))
}
fn positive(v: &Value, key: &str) -> Result<u32> {
    v[key]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("missing opening integer {key}"))
}
fn stack(v: &Value) -> Result<Stack> {
    Ok(Stack {
        item: Item::new(
            v["type"].as_str().ok_or("missing source item type")?,
            v["id"].as_str().unwrap_or(""),
        ),
        amount: positive(v, "amount")?,
    })
}
impl Data {
    pub fn load(g: &GameData, candidates: Candidates) -> Result<Self> {
        candidates.validate()?;
        let cfg = g.overlay.config.clone();
        let cat = crate::catalog014::Catalog::load(
            &g.overlay,
            crate::maps014::Candidates::for_content(&cfg)?,
        )?;
        let court = cfg.packages.iter().any(|p| p == "crimson_court");
        let curios = Curios::load_with_court_loot(&g.overlay, court)?;
        let bag_rules = InventoryRules::load(&g.overlay, &curios)?;
        let mut limits = BTreeMap::new();
        for p in
            g.overlay.paths.keys().filter(|p| {
                p.starts_with("inventory/") && p.ends_with("extra_stack_limits.darkest")
            })
        {
            for b in g.overlay.read_opening_versions(p)? {
                let doc =
                    crate::darkest::parse_str(std::str::from_utf8(&b).map_err(|e| e.to_string())?);
                if !doc.warnings.is_empty() {
                    return Err("invalid extra stack definitions".into());
                }
                for r in doc.records {
                    if r.key != "extra_stack_limit"
                        || r.params.iter().any(|p| {
                            !["id", "item_type", "item_id", "amount"].contains(&p.name.as_str())
                        })
                    {
                        return Err("unsupported extra stack rule".into());
                    }
                    library::insert_json(
                        &mut limits,
                        text(&r, "id")?.into(),
                        json!({"kind":text(&r,"item_type")?,"id":if r.param("item_id").is_some(){text(&r,"item_id")?}else{"*"},"amount":integer(&r,"amount")?}),
                    );
                }
            }
        }
        let mut class_extras = BTreeMap::new();
        for class in g.hero_ids() {
            let doc = g.hero_document(class)?;
            let mut extra = ClassExtras::default();
            for r in &doc.records {
                match r.key.as_str() {
                    "extra_stack_limit" => {
                        let raw =
                            library::unique_json(&limits, text(r, "id")?, "extra stack limit")?;
                        let item = Item::new(
                            raw["kind"].as_str().ok_or("bad stack type")?,
                            raw["id"].as_str().ok_or("bad stack item")?,
                        );
                        bag_rules.definition(&item)?;
                        let n = raw["amount"]
                            .as_u64()
                            .filter(|v| *v <= 1000000)
                            .ok_or("bad extra stack amount")?
                            as u32;
                        *extra.limits.entry(item.key()).or_default() += n;
                    }
                    "extra_battle_loot" | "extra_curio_loot" => {
                        let count = integer(r, "count")?;
                        if !(0..=1000).contains(&count) {
                            return Err("bad extra loot count".into());
                        }
                        let draw = crate::curios013::Draw {
                            table: text(r, "code")?.into(),
                            count: count as u32,
                        };
                        if count > 0 {
                            for region in ["weald", "crypts"] {
                                curios.validate_draw(&draw.table, 1, region, &mut vec![])?;
                            }
                        }
                        if r.key == "extra_battle_loot" {
                            extra.battle.push(draw);
                        } else {
                            extra.curio.push(draw);
                        }
                    }
                    _ => {}
                }
            }
            class_extras.insert(class.clone(), extra);
        }
        let mut return_rates = BTreeMap::new();
        let mut estate_sources = BTreeMap::new();
        for p in g.overlay.paths.keys().filter(|p| {
            p.starts_with("campaign/estate/") && p.ends_with(".estate.json")
                || p.as_str() == "campaign/estate/estate.json"
        }) {
            let mut values = vec![];
            for bytes in g.overlay.read_opening_versions(p)? {
                let raw: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                for row in raw["quest_fail_keep_rates"]
                    .as_array()
                    .ok_or("missing failure keep rates")?
                {
                    let key = row["type"].as_str().ok_or("missing return item type")?;
                    library::insert_json(&mut return_rates, key.into(), row.clone());
                }
                values.push(raw);
            }
            estate_sources.insert(p.clone(), values);
        }
        let roster = read(g, "scripts/starting_save/persist.roster.json")?;
        let raid = read(g, "scripts/starting_save/persist.raid.json")?;
        let quirk_defs = g.quirks()?;
        let mut initial = vec![];
        for guid in raid["data"]["party"]["heroes"]
            .as_array()
            .ok_or("missing starting party")?
        {
            let guid = guid.as_u64().ok_or("invalid starting guid")?;
            let h = &roster["data"]["heroes"][guid.to_string()];
            let class = h["heroClass"].as_str().ok_or("missing starting class")?;
            let skills = h["skills"]["selected_combat_skills"]
                .as_object()
                .ok_or("missing selected skills")?;
            if skills.values().any(|v| v.as_u64() != Some(0))
                || h["weapon_rank"] != 0
                || h["armour_rank"] != 0
            {
                return Err("unsupported opening equipment/skill rank".into());
            }
            let selected = skills.keys().map(String::as_str).collect::<Vec<_>>();
            let b = g.hero(class, 0, 0, &selected)?;
            let build = crate::build010::Build {
                quirks: h["quirks"]
                    .as_object()
                    .ok_or("missing starting quirks")?
                    .keys()
                    .cloned()
                    .collect(),
                ..Default::default()
            };
            let mut b = g.equip_build(b, build)?;
            let hp = h["actor"]["current_hp"]
                .as_i64()
                .ok_or("missing opening hp")? as i32;
            let stress = h["stress"].as_i64().ok_or("missing opening stress")? as i32;
            if h["affliction_severity"] != 0
                || h["actor"]["stunned"] != 0
                || h["actor"]["buff_group"]
                    .as_object()
                    .is_none_or(|v| !v.is_empty())
            {
                return Err("unsupported active starting status".into());
            }
            b.mechanics.hero.initial_hp = Some(hp);
            b.mechanics.hero.carry = Some(crate::hero010::HeroCarry {
                status: Default::default(),
                stress,
                resolve: crate::hero010::CarryResolve::Calm,
                buffs: vec![],
            });
            initial.push(HeroSeed {
                guid,
                name: h["actor"]["name_id"]
                    .as_str()
                    .ok_or("missing hero name")?
                    .into(),
                blueprint: b,
                xp: positive(h, "resolveXp")?,
                source: h.clone(),
            });
        }
        let recruits = read(
            g,
            "campaign/town/buildings/stage_coach/stage_coach.building.json",
        )?;
        let recruit_classes = recruits["data"]["stores"]
            .as_array()
            .ok_or("missing recruit store")?
            .iter()
            .find(|v| v["id"] == "hero_recruit")
            .ok_or("missing hero recruit")?["data"]["first_hero_classes"]
            .as_array()
            .ok_or("missing fixed first recruits")?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or("invalid first recruit".into())
            })
            .collect::<Result<Vec<_>>>()?;
        let estate = read(g, "scripts/starting_save/persist.estate.json")?;
        let mut wallet = BTreeMap::new();
        for v in estate["data"]["wallet"]
            .as_object()
            .ok_or("missing initial wallet")?
            .values()
        {
            let mut s = stack(v)?;
            if !bag_rules.items.contains_key(&s.item.key())
                && bag_rules
                    .items
                    .contains_key(&format!("heirloom:{}", s.item.kind))
            {
                s.item = Item::new("heirloom", &s.item.kind);
            }
            wallet.insert(s.item.key(), s.amount);
        }
        let globals = read(g, "shared/rules.json")?;
        let provision = read(g, "campaign/provision/provision.json")?;
        let base_props = read(g, "props/prop_definitions.json")?;
        let trap_props = read(g, "props/trap_definitions.json")?;
        let hazard = |base: &Value, override_data: &Value| -> Result<Hazard> {
            let mut d = base.as_object().ok_or("invalid prop defaults")?.clone();
            for (k, v) in override_data.as_object().ok_or("invalid prop override")? {
                d.insert(k.clone(), v.clone());
            }
            let d = Value::Object(d);
            let effects = |k: &str| -> Result<Vec<CurioEffect>> {
                d[k].as_array()
                    .ok_or("missing hazard effects")?
                    .iter()
                    .map(|v| CurioEffect::load(g, v.as_str().ok_or("invalid effect id")?))
                    .collect()
            };
            Ok(Hazard {
                health: d["health"].as_f64().ok_or("missing hazard hp")?,
                light: d["torchlight"].as_f64().ok_or("missing hazard light")? as i32,
                success: effects("success_effects")?,
                failure: effects("fail_effects")?,
            })
        };
        let prop = |name: &str| -> Result<&Value> {
            base_props["props"]
                .as_array()
                .ok_or("missing base props")?
                .iter()
                .find(|v| v["name"] == name)
                .map(|v| &v["default_data"])
                .ok_or("missing inherited prop".into())
        };
        let obstacle = hazard(prop("obstacle")?, &json!({}))?;
        let mut levels = vec![];
        for id in ["old_road", "tutorial_crypts"] {
            let map = cat.fixed(id)?;
            let q = if id == "old_road" {
                raid["data"]["raid_instance"].clone()
            } else {
                cat.quests
                    .get("plot_tutorial_crypts")
                    .ok_or("missing first ruins task")?
                    .clone()
            };
            let quest = if id == "old_road" { &q } else { &q["quest"] };
            let goalid = quest["goal_ids"][0]
                .as_str()
                .ok_or("missing initial goal")?;
            let goal = library::unique_json(&cat.goal_defs, goalid, "opening goal")?.clone();
            let region = quest["dungeon"].as_str().ok_or("missing quest dungeon")?;
            let difficulty = positive(quest, "difficulty")?;
            let props_path = format!("dungeons/{region}/{region}.props.darkest");
            let props_bytes = g.overlay.read_opening(&props_path)?;
            let props_doc = crate::darkest::parse_str(
                std::str::from_utf8(&props_bytes).map_err(|e| e.to_string())?,
            );
            if !props_doc.warnings.is_empty() {
                return Err("invalid region prop syntax".into());
            }
            let traps = props_doc
                .records_keyed("traps")
                .filter(|r| crate::combat::number(r, "chance").is_ok_and(|n| n > 0.))
                .collect::<Vec<_>>();
            if traps.len() != 1 {
                return Err("ambiguous fixed opening trap selector".into());
            }
            let ids = traps[0].param("types").ok_or("missing trap type")?;
            if ids.values.len() != 1 {
                return Err("unsupported opening trap alternatives".into());
            }
            let trap_name = &ids.values[0].text;
            let source_trap = trap_props["props"]
                .as_array()
                .ok_or("missing trap props")?
                .iter()
                .find(|v| v["name"].as_str() == Some(trap_name))
                .ok_or("missing region trap definition")?;
            let default_trap = hazard(prop("trap")?, &source_trap["default_data"])?;

            let mashpath = format!("dungeons/{region}/{region}.{difficulty}.mash.darkest");
            let mash = library::read_document(&g.overlay, &mashpath)?;
            let mut formations = BTreeMap::new();
            let mut effects = BTreeMap::new();
            let mut curio_types = BTreeMap::new();
            let mut traps = BTreeMap::new();
            for (nid, n) in &map.nodes {
                if let Some(mashref) = &n.content.battle {
                    let name = if let Some(index) = mashref.strip_prefix("index:") {
                        if id != "old_road" {
                            return Err("unmapped opening formation index".into());
                        }
                        candidates
                            .old_road_named_mashes
                            .get(index)
                            .ok_or("unknown old road index")?
                            .clone()
                    } else {
                        mashref.clone()
                    };
                    let rows = mash
                        .records
                        .iter()
                        .filter(|r| r.key == "named" && text(r, "name").ok() == Some(name.as_str()))
                        .collect::<Vec<_>>();
                    let r = *rows
                        .first()
                        .ok_or_else(|| format!("missing named opening mash {name}"))?;
                    if rows.iter().any(|v| !library::same_record(r, v)) {
                        return Err("ambiguous opening mash".into());
                    }
                    let ids = r
                        .param("types")
                        .ok_or("missing formation types")?
                        .values
                        .iter()
                        .map(|v| v.text.clone())
                        .collect::<Vec<_>>();
                    let mut loot = vec![];
                    let mut monsters = vec![];
                    for mid in ids {
                        monsters.push(g.monster(&mid)?);
                        let suffix = format!("/{mid}/{mid}.info.darkest");
                        let p = g
                            .overlay
                            .combat_files("monsters/", ".info.darkest")
                            .into_iter()
                            .find(|p| p.ends_with(&suffix))
                            .ok_or("missing monster source")?;
                        let d = library::read_document(&g.overlay, &p)?;
                        for r in d.records_keyed("loot") {
                            let code = text(r, "code")?;
                            let count = integer(r, "count")?;
                            if !(0..=1000).contains(&count) {
                                return Err("invalid monster loot count".into());
                            }
                            if count > 0 {
                                curios.validate_draw(
                                    code,
                                    difficulty as i64,
                                    region,
                                    &mut vec![],
                                )?;
                                loot.push(crate::curios013::Draw {
                                    table: code.into(),
                                    count: count as u32,
                                });
                            }
                        }
                    }
                    formations.insert(
                        nid.clone(),
                        Formation {
                            monsters,
                            loot,
                            sources: vec![mashpath.clone(), name],
                        },
                    );
                }
                if let Some(name) = &n.content.curio {
                    if name.starts_with("hash:") {
                        return Err(format!("unresolved opening curio {name}"));
                    }
                    let (prop, c) = curios.prop(crate::dson013::hash(name))?;
                    curio_types.insert(nid.clone(), prop);
                    for o in c.outcomes.iter().map(|v| &v.0).chain(c.items.values()) {
                        match o {
                            Outcome::Effect(es) => {
                                for (id, _) in es {
                                    effects.insert(id.clone(), CurioEffect::load(g, id)?);
                                }
                            }
                            Outcome::Loot { draws, .. } => {
                                for d in draws {
                                    curios.validate_draw(
                                        &d.table,
                                        difficulty as i64,
                                        region,
                                        &mut vec![],
                                    )?;
                                }
                            }
                            Outcome::Nothing => {}
                        }
                    }
                }
                if n.content.trap.is_some() {
                    traps.insert(nid.clone(), default_trap.clone());
                }
            }
            let pools = crate::opening_spawn015::load(g, region, difficulty, &curios)?;
            levels.push(Level {
                map,
                quest: q,
                goal,
                formations,
                pools,
                traps,
                default_trap,
                obstacle: obstacle.clone(),
                effects,
                curio_types,
            });
        }
        Ok(Self {
            content: cfg,
            candidates,
            levels,
            initial,
            roster_source: roster.clone(),
            raid_source: raid,
            recruit_classes,
            recruit_source: recruits,
            provision,
            roster_variables: read(g, "campaign/roster/roster.variables.json")?,
            wallet,
            bag_rules,
            class_extras,
            return_rates,
            estate_sources,
            curios,
            globals,
            quirk_defs,
            rules: g.rules.clone(),
            next_guid: positive(&roster["data"], "nextGuid")? as u64,
        })
    }
    pub fn starting_stacks(&self) -> Result<Vec<Stack>> {
        self.raid_source["data"]["party"]["inventory"]["items"]
            .as_object()
            .ok_or("missing source inventory")?
            .values()
            .map(stack)
            .collect()
    }
}
