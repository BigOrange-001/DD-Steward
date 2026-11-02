//! Typed views of original combat data. Unknown skill mechanics are rejected,
//! while the generic parser keeps every field for future implementations.
use crate::darkest::{self, Document, Record};
use crate::mechanics::{ActorMechanics, Brain, Buff, CombatGlobals, StressRules, read_buff};
#[path = "effects011.rs"]
mod boss_effects;
#[path = "boss011.rs"]
mod boss_parser;
#[path = "effects010.rs"]
mod context_effects;
#[path = "effects008.rs"]
mod extended;
#[path = "lifecycle009.rs"]
mod lifecycle_parser;
#[path = "trinket_effect010.rs"]
mod trinket_effects;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub use crate::execution012::EffectExecution;

pub type Result<T> = std::result::Result<T, String>;

pub fn document(path: &Path) -> Result<Document> {
    let d = darkest::parse_file(path).map_err(|e| format!("{}: {e:?}", path.display()))?;
    if !d.warnings.is_empty() {
        return Err(format!("{}: {:?}", path.display(), d.warnings));
    }
    Ok(d)
}
pub fn text<'a>(r: &'a Record, name: &str) -> Result<&'a str> {
    r.param(name)
        .filter(|p| p.values.len() == 1)
        .and_then(|p| p.values.first())
        .map(|v| v.text.as_str())
        .ok_or_else(|| format!("line {} {}: missing .{name}", r.line, r.key))
}
pub fn number(r: &Record, name: &str) -> Result<f64> {
    scalar(text(r, name)?)
}
pub fn scalar(s: &str) -> Result<f64> {
    let (s, scale) = s.strip_suffix('%').map_or((s, 1.), |s| (s, 0.01));
    let n: f64 = s.parse().map_err(|_| format!("invalid number: {s}"))?;
    if !n.is_finite() {
        return Err("non-finite number".into());
    }
    Ok(n * scale)
}
pub(crate) fn integer(r: &Record, name: &str) -> Result<i32> {
    let n = number(r, name)?;
    if n.fract() != 0. || n < i32::MIN as f64 || n > i32::MAX as f64 {
        return Err(format!(".{name} must be an integer"));
    }
    Ok(n as i32)
}
fn pair(r: &Record, name: &str) -> Result<[i32; 2]> {
    let p = r.param(name).ok_or_else(|| format!("missing .{name}"))?;
    if p.values.len() != 2 {
        return Err(format!(".{name} requires two values"));
    }
    let a = p.values[0]
        .text
        .parse()
        .map_err(|_| format!("invalid .{name}"))?;
    let b = p.values[1]
        .text
        .parse()
        .map_err(|_| format!("invalid .{name}"))?;
    if a < 0 || b < 0 || (name != "move" && a > b) {
        return Err(format!("invalid .{name} range"));
    }
    Ok([a, b])
}
pub(crate) fn flag(r: &Record, name: &str, default: bool) -> Result<bool> {
    if r.param(name).is_none() {
        return Ok(default);
    }
    match text(r, name)?.to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid boolean .{name}")),
    }
}
fn one<'a>(d: &'a Document, key: &str) -> Result<&'a Record> {
    let mut found = d.records.iter().filter(|r| r.key == key);
    let r = found.next().ok_or_else(|| format!("missing {key}"))?;
    if found.next().is_some() {
        return Err(format!("ambiguous {key}"));
    }
    Ok(r)
}
fn named<'a>(d: &'a Document, key: &str, id: &str) -> Result<&'a Record> {
    let mut matches = d
        .records
        .iter()
        .filter(|r| r.key == key && text(r, "name").ok() == Some(id));
    let first = matches
        .next()
        .ok_or_else(|| format!("missing {key} {id}"))?;
    if matches.any(|r| !crate::library::same_record(first, r)) {
        return Err(format!(
            "ambiguous {key} {id}: conflicting original definitions"
        ));
    }
    Ok(first)
}
fn safe_id(id: &str) -> Result<()> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("invalid data id: {id}"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rules {
    /// Opaque host receipt binding; no effect on battle settlement or policy observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_return_token: Option<String>,
    #[serde(default = "crate::candidate012::CombatCandidates::documented")]
    pub candidates: crate::candidate012::CombatCandidates,
    #[serde(default)]
    pub bloodlight: Option<crate::blood011::Bloodlight>,
    #[serde(default)]
    pub encounter_contract: Option<crate::contract011::Contract>,
    #[serde(default)]
    pub farm: Option<crate::farm011::Plan>,
    #[serde(default)]
    pub flashback: Option<crate::nightmare011::Flashback>,
    #[serde(rename = "base_affliction_history_WF")]
    pub base_affliction_history_wf: f64,
    #[serde(rename = "base_virtue_history_WF")]
    pub base_virtue_history_wf: f64,
    pub affliction_history_increment: Vec<f64>,
    pub quirks_max_diseases: u32,
    pub quirks_max_positive: u32,
    pub quirks_max_negative: u32,
    pub the_blood_item_health_damage_percent: f64,
    pub effect_crit_disease_resist_roll_mod: f64,
    pub effect_crit_roll_mod: f64,
    pub consecutive_miss_reduction_acc_buff: f64,
    pub quirks_fated_miss2hitconvertchance: f64,
    pub health_damage_blocks_max: u32,
    pub heal_buff_percent_min: f64,
    pub heal_buff_percent_max: f64,
    pub deathblow_chance: f64,
    pub deactivation_chance_small: f64,
    pub deactivation_chance_med: f64,
    pub min_hero_count_for_escape: u32,
    #[serde(rename = "excessCritMult", alias = "exessCritMult")]
    #[serde(default)]
    pub excess_crit_mult: Option<f64>,
    #[serde(rename = "excessCritMax")]
    pub excess_crit_max: f64,
    pub stun_onset_buffs: Vec<String>,
    pub stun_onset_buff_consecutive_applies: bool,
    pub stun_ends_turn: bool,
    pub affliction_critical_success_chance: f64,
    pub stress_affliction_critically_resisted: f64,
    pub stress_affliction_deactivated: f64,
    pub affliction_severity_table: Vec<Multiplier>,
    pub new_game_plus_monster_max_hp_modifier: f64,
    pub new_game_plus_monster_crit_modifier: f64,
    #[serde(default)]
    pub context: Option<crate::build010::BattleContext>,
    #[serde(default)]
    pub lighting: Option<crate::lighting::Lighting>,
    /// Host content metadata, never a combat formula or hidden policy state.
    #[serde(default = "crate::content::ContentConfig::fixture")]
    pub content: crate::content::ContentConfig,
    #[serde(rename = "cutoffAlwaysMiss")]
    pub hit_min: f64,
    #[serde(rename = "cutoffAlwaysHit")]
    pub hit_max: f64,
    pub initiative_base: i32,
    pub player_crit_multiplier_table: Vec<Multiplier>,
    pub monster_crit_multiplier_table: Vec<Multiplier>,
    pub combat_retreat_chance: f64,
    pub combat_retreat_bonus_chance_per_attempt: f64,
    pub combat_retreat_stress: f64,
    pub passed_turn_stress_dmg: f64,
    pub stun_recovery_buff_consecutive_applies: bool,
    #[serde(flatten)]
    pub stress: StressRules,
    #[serde(default)]
    pub globals: Option<crate::shared027::Shared<CombatGlobals>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Multiplier {
    pub chance: f64,
    pub value: f64,
}
impl Rules {
    pub fn validate(&self) -> Result<()> {
        self.candidates.validate()?;
        if self
            .host_return_token
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() > 512 || v.chars().any(char::is_control))
        {
            return Err("invalid host return token".into());
        }
        if !(0. ..=1.).contains(&self.deactivation_chance_small)
            || !(0. ..=1.).contains(&self.deactivation_chance_med)
            || !(1..=4).contains(&self.min_hero_count_for_escape)
        {
            return Err("invalid source legacy host parameters".into());
        }
        if !self.stun_onset_buffs.is_empty()
            || !self.stun_ends_turn
            || self.excess_crit_mult.is_some_and(|v| v != 0.)
            || self.affliction_severity_table.len() != 1
            || self.affliction_severity_table[0].value != 1.
            || self.affliction_severity_table[0].chance <= 0.
        {
            return Err(
                "unsupported non-inert stun/excess-critical/affliction-severity rule".into(),
            );
        }
        if !(0. ..=1.).contains(&self.excess_crit_max)
            || !(0. ..=1.).contains(&self.deathblow_chance)
            || !(0. ..=1.).contains(&self.affliction_critical_success_chance)
            || !self.stress_affliction_critically_resisted.is_finite()
            || !(0. ..100.).contains(&self.stress_affliction_critically_resisted)
            || !self.stress_affliction_deactivated.is_finite()
            || !(0. ..100.).contains(&self.stress_affliction_deactivated)
        {
            return Err("invalid parsed resolve/deathblow rule".into());
        }
        if !self.effect_crit_roll_mod.is_finite()
            || !(-1. ..=1.).contains(&self.effect_crit_roll_mod)
            || !self.new_game_plus_monster_max_hp_modifier.is_finite()
            || !(0. ..=10.).contains(&self.new_game_plus_monster_max_hp_modifier)
            || !self.new_game_plus_monster_crit_modifier.is_finite()
            || !(0. ..=1.).contains(&self.new_game_plus_monster_crit_modifier)
            || !self.consecutive_miss_reduction_acc_buff.is_finite()
            || !(0. ..=1.).contains(&self.consecutive_miss_reduction_acc_buff)
            || !self.quirks_fated_miss2hitconvertchance.is_finite()
            || !(0. ..=1.).contains(&self.quirks_fated_miss2hitconvertchance)
            || !(1..=10000).contains(&self.health_damage_blocks_max)
            || !self.heal_buff_percent_min.is_finite()
            || !self.heal_buff_percent_max.is_finite()
            || self.heal_buff_percent_min < 0.
            || self.heal_buff_percent_max < self.heal_buff_percent_min
        {
            return Err("invalid parsed battle rule values".into());
        }
        if !(0. ..=1.).contains(&self.hit_min)
            || !(self.hit_min..=1.).contains(&self.hit_max)
            || self.initiative_base < 1
        {
            return Err("invalid hit/initiative rules".into());
        }
        for table in [
            &self.player_crit_multiplier_table,
            &self.monster_crit_multiplier_table,
        ] {
            if table.len() != 1 || table[0].chance <= 0. || table[0].value < 1. {
                return Err("unsupported critical multiplier table".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stats {
    pub hp: i32,
    pub dodge: f64,
    pub protection: f64,
    pub speed: i32,
    pub damage: [i32; 2],
    pub accuracy: f64,
    pub crit: f64,
    pub death_resist: f64,
    pub stun_resist: f64,
    pub bleed_resist: f64,
    pub blight_resist: f64,
    pub debuff_resist: f64,
    pub move_resist: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Target {
    pub ranks: Vec<u8>,
    pub friendly: bool,
    pub multi: bool,
    pub random: bool,
}
impl Target {
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Ok(Self {
                ranks: vec![],
                friendly: true,
                multi: false,
                random: false,
            });
        }
        let (mut friendly, mut multi, mut random) = (false, false, false);
        let mut s = s;
        while let Some(c) = s.chars().next().filter(|c| ['@', '~', '?'].contains(c)) {
            let flag = match c {
                '@' => &mut friendly,
                '~' => &mut multi,
                '?' => &mut random,
                _ => unreachable!(),
            };
            if *flag {
                return Err("duplicate target prefix".into());
            }
            *flag = true;
            s = &s[1..];
        }
        if multi && random {
            return Err("contradictory target modes".into());
        }
        let mut ranks = vec![];
        for c in s.chars() {
            let n = c
                .to_digit(10)
                .filter(|n| (1..=4).contains(n))
                .ok_or_else(|| format!("unsupported target code {s}"))? as u8;
            if ranks.contains(&n) {
                return Err(format!("duplicate rank in {s}"));
            }
            ranks.push(n);
        }
        if ranks.is_empty() {
            return Err("self-only/empty targets not implemented".into());
        }
        Ok(Self {
            ranks,
            friendly,
            multi,
            random,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffectKind {
    RefreshSkills,
    BattlePersistent(Box<EffectKind>),
    Boss(crate::special::BossOp),
    InventoryEffect(Box<EffectKind>),
    TrinketOp(crate::trinket010::TrinketOp),
    HealthDamage(i32),
    SetMode(String),
    RemoveVampire,
    InstantBuffs(Vec<Buff>),
    ControlledBurn {
        amount: i32,
        rounds: u32,
    },
    Lifecycle(crate::lifecycle::LifecycleEffect),
    Disease {
        id: String,
        crit_bonus: bool,
    },
    Composite(Vec<Effect>),
    ProtectedBuffs(Vec<Buff>),
    Burn(i32),
    Regen(i32),
    HealPercent(f64),
    Cure {
        bleed: bool,
        poison: bool,
        burn: bool,
    },
    Unstun,
    ClearMark,
    ClearBuffs {
        positive: bool,
    },
    KillCorpses,
    ShuffleParty,
    Guard {
        reverse: bool,
    },
    ClearGuard {
        guarding: bool,
        guarded: bool,
    },
    Riposte {
        hit: f64,
        miss: f64,
        buffs: Vec<Buff>,
    },
    ClearRiposte,
    Blocks {
        count: u32,
        turns: Option<u32>,
    },
    Stealth,
    Unstealth,
    Horror(i32),
    ProtectedHorror(i32),
    ClearHorror,
    Push(u8),
    Pull(u8),
    Shuffle,
    Bleed(i32),
    Blight(i32),
    Stun,
    Stress(i32),
    Heal(i32),
    Buffs(Vec<Buff>),
    Mark,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffectTarget {
    ThisTrinket,
    EnemyRandom,
    EnemyRank(u8),
    TargetParty,
    TargetEnemies,
    Target,
    Performer,
    Party,
    PartyOther,
    Enemies,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Condition {
    Status(String),
    MonsterType(String),
}
impl Condition {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Status(s)
                if ["tagged", "poisoned", "stunned", "bleeding", "burning"]
                    .contains(&s.as_str()) =>
            {
                Ok(())
            }
            Self::MonsterType(s)
                if [
                    "eldritch",
                    "unholy",
                    "man",
                    "beast",
                    "corpse",
                    "husk",
                    "stonework",
                    "unfortunate_soul",
                    "vampire",
                ]
                .contains(&s.as_str()) =>
            {
                Ok(())
            }
            _ => Err(format!("unknown combat condition {self:?}")),
        }
    }
}
impl EffectKind {
    pub fn needs_lighting(&self) -> bool {
        match self {
            Self::Lifecycle(crate::lifecycle::LifecycleEffect {
                op: crate::lifecycle::LifeOp::Light { .. },
                ..
            }) => true,
            Self::Composite(ops) => ops.iter().any(|e| e.kind.needs_lighting()),
            Self::InventoryEffect(inner) => inner.needs_lighting(),
            _ => false,
        }
    }
    pub fn contains_guard(&self) -> bool {
        match self {
            Self::Guard { .. } => true,
            Self::Composite(es) => es.iter().any(|e| e.kind.contains_guard()),
            _ => false,
        }
    }
    pub fn contains_riposte(&self) -> bool {
        match self {
            Self::BattlePersistent(inner) => inner.contains_riposte(),
            Self::Riposte { .. } => true,
            Self::Composite(ops) => ops.iter().any(|e| e.kind.contains_riposte()),
            _ => false,
        }
    }
    pub fn is_displacement(&self) -> bool {
        matches!(
            self,
            Self::Push(_) | Self::Pull(_) | Self::Shuffle | Self::ShuffleParty
        )
    }
    pub fn contains_stun(&self) -> bool {
        match self {
            Self::Stun => true,
            Self::Composite(ops) => ops.iter().any(|e| e.kind.contains_stun()),
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    #[serde(default)]
    pub execution: EffectExecution,
    pub name: String,
    pub kind: EffectKind,
    pub chance: f64,
    pub duration: u32,
    pub target: EffectTarget,
    pub on_hit: bool,
    pub on_miss: bool,
    pub apply_once: bool,
    pub resist_debuff: bool,
    pub conditions: Vec<Condition>,
    pub can_apply_on_death: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub tactical: TacticalSkill,
    pub id: String,
    pub ignore_stealth: bool,
    pub launch: Vec<u8>,
    pub target: Target,
    pub accuracy: f64,
    pub damage: Option<[i32; 2]>,
    pub crit: f64,
    pub heal: Option<[i32; 2]>,
    pub effects: Vec<Effect>,
    pub move_forward: usize,
    pub per_battle_limit: Option<u32>,
    pub move_back: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TacticalSkill {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_damage023: Option<([i32; 2], f64)>,
    #[serde(default = "true_value")]
    pub crit_valid: bool,
    pub boss: crate::special::BossSkill,
    pub valid_modes: Vec<String>,
    pub mode_effects: BTreeMap<String, Vec<Effect>>,
    pub refresh_each_wave: bool,
    pub requires_burning: bool,
    pub stall_invalidating: bool,
    pub dmg_per_burn_stack: f64,
    pub copy_burn_behind: bool,
    pub hp_range: Option<[f64; 2]>,
    pub continue_turn: bool,
    pub per_turn_limit: Option<u32>,
    pub attack_type: String,
    pub self_target_valid: bool,
    pub extra_targets_count: u8,
    pub extra_targets_chance: f64,
    pub ignore_guard: bool,
    pub ignore_protection: bool,
    pub can_be_riposted: bool,
}
fn true_value() -> bool {
    true
}
impl Default for TacticalSkill {
    fn default() -> Self {
        Self {
            source_damage023: None,
            crit_valid: true,
            boss: Default::default(),
            valid_modes: vec![],
            mode_effects: BTreeMap::new(),
            refresh_each_wave: false,
            requires_burning: false,
            stall_invalidating: true,
            dmg_per_burn_stack: 0.,
            copy_burn_behind: false,
            hp_range: None,
            continue_turn: false,
            per_turn_limit: None,
            attack_type: "melee".into(),
            self_target_valid: true,
            extra_targets_count: 0,
            extra_targets_chance: 0.,
            ignore_guard: false,
            ignore_protection: false,
            can_be_riposted: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Corpse {
    pub summonable: bool,
    pub random_targetable: bool,
    pub size: u8,
    pub stats: Stats,
    pub rounds: u32,
    pub on_crit: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blueprint {
    pub id: String,
    pub hero: bool,
    pub size: u8,
    pub stats: Stats,
    pub skills: Vec<Skill>,
    pub move_back: usize,
    pub move_forward: usize,
    pub corpse: Option<Corpse>,
    pub brain_id: Option<String>,
    pub mechanics: ActorMechanics,
}

pub struct GameData {
    pub quirk_acts023: BTreeMap<String, crate::mechanisms023::QuirkAct>,
    pub root: PathBuf,
    pub rules: Rules,
    pub effects: Document,
    pub brains: BTreeMap<String, serde_json::Value>,
    pub buffs: BTreeMap<String, serde_json::Value>,
    pub source_buff_conflicts: BTreeMap<String, serde_json::Value>,
    selected_mode_effects: BTreeMap<String, Record>,
    pub traits: BTreeMap<String, serde_json::Value>,
    pub overlay: crate::overlay::Overlay,
    /// Inventory loading succeeds without claiming unsupported mechanics run.
    pub combat_globals_error: Option<String>,
    heroes: BTreeMap<String, Vec<Document>>,
    monsters: BTreeMap<String, Vec<Document>>,
}
impl GameData {
    pub fn load(root: &Path) -> Result<Self> {
        let data = Self::load_with_content(root, crate::content::ContentConfig::fixture())?;
        if let Some(error) = &data.combat_globals_error {
            return Err(error.clone());
        }
        Ok(data)
    }
    pub fn load_with_content(root: &Path, config: crate::content::ContentConfig) -> Result<Self> {
        let overlay = crate::overlay::Overlay::load(root, config)?;
        let quirk_acts023 = crate::mechanisms023::load_quirk_acts(&overlay)?;
        let raw_rules = overlay.json("shared/rules.json")?;
        let mut rules: Rules =
            serde_json::from_value((*raw_rules).clone()).map_err(|e| e.to_string())?;
        rules.content = overlay.config.clone();
        crate::rules_inventory012::inventory(&raw_rules, &rules)?;
        rules.validate()?;
        if (rules.affliction_critical_success_chance + rules.stress.affliction_base_onset_chance
            - 1.)
            .abs()
            > 1e-9
        {
            return Err(
                "source resolve success/affliction probabilities must be complementary".into(),
            );
        }
        let effects = crate::library::effects(&overlay)?;
        let selected_mode_effects = crate::mode_effects012::select(&overlay)?;
        let mut buffs = crate::library::json_library(&overlay, "shared/buffs/", ".json", "buffs")?;
        let mut source_buff_conflicts = BTreeMap::new();
        if overlay
            .config
            .syntax_choices
            .get("smoking_skull_conflict")
            .map(String::as_str)
            == Some("poison_resistance")
        {
            let id = "TRINKET_com_ant_smoking_skull_blight_res_D15";
            let raw = buffs
                .get(id)
                .ok_or("selected original Buff is missing")?
                .clone();
            let variants = raw
                .as_array()
                .ok_or("selected Buff is no longer conflicting")?;
            if variants.len() != 2 || variants.iter().any(|v| v["id"] != id) {
                return Err("selected Buff conflict shape changed".into());
            }
            let selected = variants
                .iter()
                .filter(|v| v["stat_type"] == "resistance" && v["stat_sub_type"] == "poison")
                .collect::<Vec<_>>();
            if selected.len() != 1
                || !variants.iter().any(|v| {
                    v["stat_type"] == "combat_stat_add" && v["stat_sub_type"] == "attack_rating"
                })
            {
                return Err("selected Buff conflict alternatives changed".into());
            }
            let chosen = selected[0].clone();
            source_buff_conflicts.insert(id.into(), raw);
            buffs.insert(id.into(), chosen);
        }
        if overlay
            .config
            .syntax_choices
            .get("camp_hp_recovery_conflict")
            .map(String::as_str)
            == Some("larger_original_penalty")
        {
            let id = "QUIRK_in_camp_HPRECOVERY_D";
            let raw = buffs.get(id).ok_or("selected camp Buff missing")?.clone();
            let chosen = select_campaign018_camp_buff(&raw)?;
            source_buff_conflicts.insert(id.into(), raw);
            buffs.insert(id.into(), chosen);
        }
        let brains = crate::library::json_library(
            &overlay,
            "raid/ai/",
            ".monster_brains.json",
            "monster_brains",
        )?;
        let (traits, trait_order) = crate::library::json_library_ordered(
            &overlay,
            "shared/trait/",
            "trait_library.json",
            "traits",
        )?;
        let mut generated = vec![];
        let mut generated_error = None;
        for id in &trait_order {
            let definition = &traits[id];
            if definition.is_array() {
                if definition
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["is_generated"] == true)
                {
                    // Generated traits are implicitly referenced by the resolve
                    // distribution. Non-generated traits stay lazy library data.
                    generated_error = Some(format!(
                        "ambiguous trait {id}: used by resolve distribution"
                    ));
                }
            } else {
                generated.push(definition.clone());
            }
        }
        let globals = if let Some(error) = generated_error {
            Err(error)
        } else {
            CombatGlobals::load(
                &raw_rules,
                &buffs,
                &serde_json::json!({"traits": generated}),
            )
        };
        let combat_globals_error = globals.as_ref().err().cloned();
        rules.globals = globals.ok().map(Into::into);
        crate::mechanics::validate_stress(&rules)?;
        let heroes = crate::library::units(&overlay, true)?;
        let monsters = crate::library::units(&overlay, false)?;
        Ok(Self {
            root: root.to_owned(),
            rules,
            effects,
            brains,
            buffs,
            source_buff_conflicts,
            quirk_acts023,
            selected_mode_effects,
            traits,
            overlay,
            combat_globals_error,
            heroes,
            monsters,
        })
    }
    pub fn hero_ids(&self) -> impl Iterator<Item = &String> {
        self.heroes.keys()
    }
    pub fn ambiguous_ids(&self) -> BTreeMap<&str, Vec<String>> {
        let mut result: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        let mut effects: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
        for r in &self.effects.records {
            if let Ok(name) = text(r, "name") {
                let variants = effects.entry(name.into()).or_default();
                if !variants
                    .iter()
                    .any(|old| crate::library::same_record(old, r))
                {
                    variants.push(r);
                }
            }
        }
        result.insert(
            "effects",
            effects
                .into_iter()
                .filter_map(|(id, v)| (v.len() > 1).then_some(id))
                .collect(),
        );
        for (kind, library) in [
            ("Buffs", &self.buffs),
            ("Brains", &self.brains),
            ("traits", &self.traits),
        ] {
            result.insert(
                kind,
                library
                    .iter()
                    .filter_map(|(id, v)| v.is_array().then_some(id.clone()))
                    .collect(),
            );
        }
        result
            .entry("Buffs")
            .or_default()
            .extend(self.source_buff_conflicts.keys().cloned());
        for (kind, library) in [("heroes", &self.heroes), ("monsters", &self.monsters)] {
            result.insert(
                kind,
                library
                    .iter()
                    .filter_map(|(id, v)| (v.len() > 1).then_some(id.clone()))
                    .collect(),
            );
        }
        result
    }
    pub fn monster_ids(&self) -> impl Iterator<Item = &String> {
        self.monsters.keys()
    }
    pub fn hero_document(&self, id: &str) -> Result<&Document> {
        safe_id(id)?;
        crate::library::unique_unit(&self.heroes, id, "hero")
    }
    pub fn info_documents(&self, heroes: bool) -> impl Iterator<Item = &Document> {
        (if heroes { &self.heroes } else { &self.monsters })
            .values()
            .flat_map(|v| v.iter())
    }
    pub fn hero_skill_ids(&self, id: &str) -> Result<Vec<String>> {
        let mut ids = vec![];
        for d in self
            .heroes
            .get(id)
            .ok_or_else(|| format!("missing hero {id}"))?
        {
            for r in d.records_keyed("combat_skill") {
                if integer(r, "level")? == 0 {
                    let id = text(r, "id")?.to_owned();
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        }
        Ok(ids)
    }
    pub fn hero(
        &self,
        id: &str,
        equipment: usize,
        level: i32,
        equipped: &[&str],
    ) -> Result<Blueprint> {
        self.hero_ranks_inner(
            id,
            equipment,
            equipment,
            &equipped
                .iter()
                .map(|name| (*name, level))
                .collect::<Vec<_>>(),
            level,
        )
    }
    /// Town-owned training: independent weapon, armour and learned skill ranks.
    pub fn hero_ranked(
        &self,
        id: &str,
        weapon: usize,
        armour: usize,
        equipped: &[(&str, i32)],
    ) -> Result<Blueprint> {
        self.hero_ranks_inner(id, weapon, armour, equipped, 0)
    }
    fn hero_ranks_inner(
        &self,
        id: &str,
        weapon: usize,
        armour: usize,
        equipped: &[(&str, i32)],
        forced_level: i32,
    ) -> Result<Blueprint> {
        safe_id(id)?;
        if equipped.is_empty() || equipped.len() > 7 {
            return Err("equip 1..7 skills within the class limit".into());
        }
        let d = self.hero_document(id)?.clone();
        crate::schema012::unit_fields(&d)?;
        let maximum = d
            .records_keyed("skill_selection")
            .next()
            .map(|r| integer(r, "number_of_selected_combat_skills_max"))
            .transpose()?
            .unwrap_or(4);
        if equipped.len() > maximum as usize {
            return Err("too many equipped skills for class".into());
        }
        let w = named(&d, "weapon", &format!("{id}_weapon_{weapon}"))?;
        let a = named(&d, "armour", &format!("{id}_armour_{armour}"))?;
        let r = one(&d, "resistances")?;
        // Ordinary heroes occupy one rank; unlike monsters their info files
        // do not declare display.size. No art/render files are needed.
        let stats = Stats {
            hp: integer(a, "hp")?,
            dodge: number(a, "def")?,
            protection: number(a, "prot")?,
            speed: integer(w, "spd")? + integer(a, "spd")?,
            damage: pair(w, "dmg")?,
            accuracy: number(w, "atk")?,
            crit: number(w, "crit")?,
            death_resist: number(r, "death_blow")?,
            stun_resist: number(r, "stun")?,
            bleed_resist: number(r, "bleed")?,
            blight_resist: number(r, "poison")?,
            debuff_resist: number(r, "debuff")?,
            move_resist: number(r, "move")?,
        };
        let mut skills = vec![];
        for (name, level) in equipped {
            if skills.iter().any(|s: &Skill| s.id == *name) {
                return Err("duplicate equipped skill".into());
            }
            let r = d
                .records
                .iter()
                .find(|r| {
                    r.key == "combat_skill"
                        && text(r, "id").ok() == Some(*name)
                        && integer(r, "level").ok() == Some(*level)
                })
                .ok_or_else(|| format!("missing skill {name} level {level}"))?;
            skills.push(self.skill(r, Some(&stats))?);
        }
        let m = one(&d, "combat_move_skill")?;
        let movement = pair(m, "move")?;
        let mut mechanics = self.hero_mechanics(&d, id)?;
        mechanics.hero = crate::hero010::parse(self, &d)?;
        for t in &mechanics.hero.override_defs {
            for name in t
                .turn_acts
                .iter()
                .filter(|a| {
                    a.chance > 0.
                        && (a.valid_hero_class_ids.is_empty()
                            || a.valid_hero_class_ids.iter().any(|c| c == id))
                })
                .map(|a| a.data.string_value.as_str())
                .chain(
                    t.reactions
                        .iter()
                        .filter(|a| a.chance > 0. && a.id.is_combat())
                        .map(|a| a.data.effect.as_str()),
                )
                .filter(|n| !n.is_empty())
            {
                mechanics
                    .trait_effects
                    .insert(name.to_string(), self.effect(name)?);
            }
        }
        for name in mechanics
            .hero
            .modes
            .iter()
            .flat_map(|m| m.affliction_skill.iter().chain(&m.end_skill))
        {
            let level = equipped
                .iter()
                .find(|(id, _)| *id == name)
                .map(|(_, level)| *level)
                .unwrap_or(forced_level);
            let r = d
                .records_keyed("combat_skill")
                .find(|r| {
                    text(r, "id").ok() == Some(name.as_str())
                        && integer(r, "level").ok() == Some(level)
                })
                .ok_or("missing forced mode skill")?;
            mechanics
                .hero
                .forced
                .insert(name.clone(), self.skill(r, Some(&stats))?);
        }
        mechanics.disease_resist = number(r, "disease")?;
        if skills
            .iter()
            .flat_map(|s| {
                s.effects
                    .iter()
                    .chain(s.tactical.mode_effects.values().flatten())
            })
            .any(|e| e.kind.contains_riposte())
            || mechanics
                .hero
                .modes
                .iter()
                .flat_map(|m| &m.start)
                .any(|e| e.kind.contains_riposte())
        {
            let r = d
                .records_keyed("riposte_skill")
                .next()
                .ok_or("missing original riposte skill")?;
            mechanics.riposte = Some(Box::new(self.skill(r, Some(&stats))?));
        }
        let mut refs = std::collections::BTreeSet::new();
        fn collect(e: &Effect, refs: &mut std::collections::BTreeSet<String>) {
            match &e.kind {
                EffectKind::Buffs(bs)
                | EffectKind::ProtectedBuffs(bs)
                | EffectKind::InstantBuffs(bs) => {
                    for b in bs {
                        refs.extend(b.activation.attack_additional_effect_ids.clone());
                    }
                }
                EffectKind::Composite(es) => {
                    for e in es {
                        collect(e, refs)
                    }
                }
                _ => {}
            }
        }
        for s in &skills {
            for e in s
                .effects
                .iter()
                .chain(s.tactical.mode_effects.values().flatten())
            {
                collect(e, &mut refs);
            }
        }
        for id in refs {
            mechanics
                .extra_effects
                .insert(id.clone(), self.effect(&id)?);
        }
        Ok(Blueprint {
            id: id.into(),
            hero: true,
            size: 1,
            stats,
            skills,
            move_back: movement[0] as usize,
            move_forward: movement[1] as usize,
            corpse: None,
            brain_id: None,
            mechanics,
        })
    }
    fn monster_document(&self, id: &str) -> Result<Document> {
        safe_id(id)?;
        crate::library::unique_unit(&self.monsters, id, "monster").cloned()
    }
    fn family(&self, id: &str, hero: bool) -> Result<String> {
        if hero {
            return Ok(id.into());
        }
        let mut families = std::collections::BTreeSet::new();
        for path in self.overlay.combat_files("monsters/", ".info.darkest") {
            let p = path.split('/').collect::<Vec<_>>();
            if p.len() == 4 && p[2] == id && p[3] == format!("{id}.info.darkest") {
                families.insert(p[1].to_string());
            }
        }
        if families.len() != 1 {
            return Err(format!("{id}: missing or ambiguous monster family"));
        }
        Ok(families.into_iter().next().unwrap())
    }
    fn monster_mode_stats(&self, r: &Record) -> Result<Stats> {
        let mut stats = monster_stats(r)?;
        if self.overlay.is_new_game_plus {
            stats.hp = (f64::from(stats.hp)
                * (1. + self.rules.new_game_plus_monster_max_hp_modifier))
                .ceil()
                .clamp(1., 1e6) as i32;
        }
        Ok(stats)
    }
    pub fn monster(&self, id: &str) -> Result<Blueprint> {
        let d = self.monster_document(id)?;
        crate::schema012::unit_fields(&d)?;
        let size = body_size(&d)?;
        let turns = integer(one(&d, "initiative")?, "number_of_turns_per_round")?;
        if !(0..=4).contains(&turns) {
            return Err(format!("{id}: invalid action count; expected 0..4"));
        }
        // Reject special monster lifecycles instead of interpreting them as normal enemies.
        let allowed = [
            "display",
            "enemy_type",
            "stats",
            "skill",
            "personality",
            "loot",
            "initiative",
            "monster_brain",
            "death_class",
            "life_link",
            "battle_modifier",
            "wave_spawning",
            "riposte_skill",
            "spawn",
            "life_time",
            "additional_effect",
            "crit",
            "tag",
            "shared_health",
            "shape_shifter",
            "mode",
            "controller",
            "companion",
            "skill_reaction",
            "death_damage",
            "captor_empty",
            "captor_full",
            "torchlight_modifier",
            "battle_backdrop",
            "audio_modifier",
            "tutorial",
            "mash_modifier",
        ];
        for r in &d.records {
            if !allowed.contains(&r.key.as_str()) {
                return Err(format!("{id}: unsupported record {}", r.key));
            }
        }
        let stats = self.monster_mode_stats(one(&d, "stats")?)?;
        let skills = d
            .records_keyed("skill")
            .map(|r| self.skill(r, None))
            .collect::<Result<Vec<_>>>()?;
        if skills.is_empty() && turns > 0 {
            return Err("monster has no skills".into());
        }
        let brain_id = text(one(&d, "monster_brain")?, "id")?.to_owned();
        if !self.brains.contains_key(&brain_id) {
            return Err(format!("missing brain {brain_id}"));
        }
        let corpse = if let Some(r) = d.records_keyed("death_class").next() {
            if text(r, "type").ok() != Some("corpse") {
                None
            } else {
                if let Some(p) = r.params.iter().find(|p| {
                    !["type", "monster_class_id", "is_valid_on_crit"].contains(&p.name.as_str())
                }) {
                    return Err(format!("unsupported corpse death_class field {}", p.name));
                }
                let c = self.monster_document(text(r, "monster_class_id")?)?;
                let corpse_size = body_size(&c)?;
                if corpse_size > size {
                    return Err(format!("{id}: corpse cannot exceed living body size"));
                }
                Some(Corpse {
                    summonable: flag(one(&c, "battle_modifier")?, "can_be_summon_rank", false)?,
                    random_targetable: flag(
                        one(&c, "battle_modifier")?,
                        "can_be_random_target",
                        true,
                    )?,
                    size: corpse_size,
                    stats: self.monster_mode_stats(one(&c, "stats")?)?,
                    rounds: integer(one(&c, "life_time")?, "alive_round_limit")?
                        .try_into()
                        .map_err(|_| "invalid corpse lifetime")?,
                    on_crit: flag(r, "is_valid_on_crit", true)?,
                })
            }
        } else {
            None
        };
        Ok(Blueprint {
            id: id.into(),
            hero: false,
            size,
            stats,
            skills: skills.clone(),
            move_back: 0,
            move_forward: 0,
            corpse,
            brain_id: Some(brain_id),
            mechanics: {
                let mut brain: Brain = serde_json::from_value(
                    crate::library::unique_json(
                        &self.brains,
                        text(one(&d, "monster_brain")?, "id")?,
                        "brain",
                    )?
                    .clone(),
                )
                .map_err(|e| e.to_string())?;
                if id == "com_crocodile_D"
                    && self
                        .overlay
                        .config
                        .syntax_choices
                        .get("farm_croc_skill_prefix")
                        .map(String::as_str)
                        == Some("com")
                {
                    let known = [
                        "angry_hive",
                        "teeth_rake",
                        "move_from_left",
                        "move_from_right",
                        "submerge",
                        "submerge_nuke",
                    ];
                    for c in &mut brain.skill_cooldowns {
                        if !known.contains(&c.combat_skill_id.as_str()) {
                            return Err("unrecognized farm crocodile cooldown alias".into());
                        }
                        c.combat_skill_id = format!("com_{}", c.combat_skill_id);
                    }
                    for d in &mut brain.skill_selection_desires {
                        if !d.data.combat_skill_id.is_empty() {
                            if !known.contains(&d.data.combat_skill_id.as_str()) {
                                return Err("unrecognized farm crocodile skill alias".into());
                            }
                            d.data.combat_skill_id = format!("com_{}", d.data.combat_skill_id);
                        }
                    }
                }
                brain.validate_candidates(
                    &skills
                        .iter()
                        .map(|s: &Skill| s.id.clone())
                        .collect::<Vec<_>>(),
                    self.overlay
                        .config
                        .syntax_choices
                        .get("boss_inactive_refs")
                        .map(String::as_str)
                        == Some("retain"),
                )?;
                ActorMechanics {
                    boss: boss_parser::parse(self, &d)?,
                    lifecycle: lifecycle_parser::spec(self, &d, turns as u8)?,
                    riposte: d
                        .records_keyed("riposte_skill")
                        .next()
                        .map(|r| self.skill(r, None).map(Box::new))
                        .transpose()?,
                    family: self.family(id, false)?,
                    life_link: d
                        .records_keyed("life_link")
                        .next()
                        .map(|r| text(r, "base_class").map(str::to_string))
                        .transpose()?,
                    enemy_types: d
                        .records_keyed("enemy_type")
                        .map(|r| text(r, "id").map(str::to_owned))
                        .collect::<Result<Vec<_>>>()?,
                    brain: Some(brain),
                    preferred_skill: {
                        let r = one(&d, "personality")?;
                        if turns == 0 && r.param("prefskill").is_some_and(|p| p.values.is_empty()) {
                            None
                        } else {
                            usize::try_from(integer(r, "prefskill")?).ok()
                        }
                    },
                    ..ActorMechanics::default()
                }
            },
        })
    }
    pub fn trait_effects_for_class(&self, class: &str) -> Result<BTreeMap<String, Effect>> {
        let mut result = BTreeMap::new();
        for t in self
            .rules
            .globals
            .as_ref()
            .ok_or_else(|| {
                self.combat_globals_error
                    .clone()
                    .unwrap_or_else(|| "missing trait definitions".into())
            })?
            .traits
            .iter()
            .chain(self.quirk_acts023.values())
        {
            let names = t
                .turn_acts
                .iter()
                .filter(|a| {
                    a.chance > 0.
                        && (a.valid_hero_class_ids.is_empty()
                            || a.valid_hero_class_ids.iter().any(|c| c == class))
                })
                .map(|a| a.data.string_value.as_str())
                .chain(
                    t.reactions
                        .iter()
                        .filter(|r| r.chance > 0.)
                        .map(|r| r.data.effect.as_str()),
                );
            for name in names.filter(|n| !n.is_empty()) {
                if !result.contains_key(name) {
                    let effect = self
                        .effect(name)
                        .map_err(|e| format!("{class} trait {}: {e}", t.id))?;
                    if !effect.conditions.is_empty() {
                        return Err(format!(
                            "{class}: conditional effect requires skill attack context"
                        ));
                    }
                    result.insert(name.to_string(), effect);
                }
            }
        }
        Ok(result)
    }
    fn hero_mechanics(&self, d: &Document, class: &str) -> Result<ActorMechanics> {
        let buffs_for = |name: &str| -> Result<Vec<Buff>> {
            let r = one(d, "deaths_door")?;
            r.param(name)
                .ok_or_else(|| format!("missing deaths_door .{name}"))?
                .values
                .iter()
                .map(|v| read_buff(&self.buffs, &v.text))
                .collect()
        };
        let mut on_crit = vec![];
        if let Some(p) = one(d, "crit")?.param("effects") {
            for name in &p.values {
                let e = self.effect(&name.text)?;
                if !e.conditions.is_empty() {
                    return Err("conditional on-crit effect requires skill attack context".into());
                }
                if let EffectKind::Buffs(b) = e.kind {
                    on_crit.extend(b)
                } else {
                    return Err("unsupported on-crit effect".into());
                }
            }
        }
        Ok(ActorMechanics {
            player_rounding023: crate::mechanisms023::enabled(
                &self.overlay.config,
                crate::mechanisms023::ROUNDING,
            ),
            family: class.into(),
            deaths_door: buffs_for("buffs")?,
            recovery: buffs_for("recovery_buffs")?,
            heart_recovery: buffs_for("recovery_heart_attack_buffs")?,
            on_crit,
            trait_effects: self.trait_effects_for_class(class)?,
            ..ActorMechanics::default()
        })
    }
    fn skill(&self, r: &Record, hero: Option<&Stats>) -> Result<Skill> {
        let id = text(r, "id")?.to_owned();
        let allowed = [
            "id",
            "level",
            "type",
            "atk",
            "dmg",
            "crit",
            "launch",
            "target",
            "is_crit_valid",
            "effect",
            "generation_guaranteed",
            "is_stall_invalidating",
            "heal",
            "move",
            "per_battle_limit",
            "ignore_stealth",
            "self_target_valid",
            "extra_targets_count",
            "extra_targets_chance",
            "ignore_guard",
            "ignore_protection",
            "can_be_riposted",
            "is_continue_turn",
            "per_turn_limit",
            "valid_modes",
            "human_effects",
            "beast_effects",
            "stance_defensive_effects",
            "stance_aggressive_effects",
            "requires_burning",
            "refresh_after_each_wave",
            "dmg_per_burn_stack",
            "copy_burn_behind",
            "required_performer_hp_range",
            "can_miss",
            "is_knowledgeable",
            "is_user_selected_targets",
            "starting_cooldown",
            "rank_damage_modifiers",
            "damage_heal_base_class_ids",
            "is_monster_rerank_valid_on_friendly_presentation_end",
        ];
        for p in &r.params {
            if !allowed.contains(&p.name.as_str()) {
                return Err(format!("{id}: unsupported skill parameter .{}", p.name));
            }
        }
        let launch = Target::parse(text(r, "launch")?)?;
        // Original assistance skills may omit type; their heal/target fields
        // already define their operation. Explicit unknown types are rejected.
        if r.param("type").is_some() {
            let skill_type = text(r, "type")?;
            if !["melee", "ranged", "heal", "", "0", "teleport"].contains(&skill_type) {
                return Err(format!("{id}: unsupported skill type {skill_type}"));
            }
        }
        let implicit_self = if r.param("target").is_none() {
            let effects = r
                .params_named("effect")
                .flat_map(|p| &p.values)
                .map(|v| self.effect(&v.text))
                .collect::<Result<Vec<_>>>()?;
            hero.is_none()
                && !effects.is_empty()
                && effects.iter().all(|e| e.target == EffectTarget::Performer)
                && pair(r, "dmg").ok() == Some([0, 0])
        } else {
            false
        };
        let target = if implicit_self || r.param("target").is_some_and(|p| p.values.is_empty()) {
            Target::parse("")?
        } else {
            let raw = text(r, "target")?;
            let selected = self
                .overlay
                .config
                .syntax_choices
                .get("spire_target_backtick")
                .map(String::as_str)
                == Some("strip");
            let corrected = if selected
                && raw == "`@4321"
                && ["yellow", "red", "black", "purple", "grey"]
                    .iter()
                    .any(|color| {
                        id == format!("grow_seed_{color}")
                            || id == format!("sprout_to_seedling_{color}")
                    }) {
                "@4321"
            } else {
                raw
            };
            Target::parse(corrected)?
        };
        let per_battle_limit = r
            .param("per_battle_limit")
            .map(|_| -> Result<u32> {
                let n = integer(r, "per_battle_limit")?;
                if !(1..=10000).contains(&n) {
                    return Err("invalid per_battle_limit".into());
                }
                Ok(n as u32)
            })
            .transpose()?;
        let heal = r.param("heal").map(|_| pair(r, "heal")).transpose()?;
        if heal.is_some() && !target.friendly {
            return Err(format!(
                "{id}: only hostile attacks or friendly heals supported"
            ));
        }
        let damage = if target.friendly {
            None
        } else if let Some(h) = hero {
            let m = 1. + number(r, "dmg")?;
            Some([
                (h.damage[0] as f64 * m).round().max(0.) as i32,
                (h.damage[1] as f64 * m).round().max(0.) as i32,
            ])
        } else {
            Some(pair(r, "dmg")?)
        };
        let movement = r.param("move").map(|_| pair(r, "move")).transpose()?;
        if movement.is_some_and(|p| p[0] > 4 || p[1] > 4 || (p[0] > 0 && p[1] > 0)) {
            return Err(format!("{id}: unsupported skill movement range"));
        }
        let mut effects = vec![];
        for p in r.params_named("effect") {
            for v in &p.values {
                effects.push(self.effect(&v.text)?);
            }
        }
        let accuracy = if heal.is_some() {
            1.
        } else {
            number(r, "atk")? + hero.map_or(0., |h| h.accuracy)
        };
        let crit = if heal.is_some() || !flag(r, "is_crit_valid", true)? {
            0.
        } else {
            r.param("crit")
                .map(|_| number(r, "crit"))
                .transpose()?
                .unwrap_or(0.)
                + hero.map_or(0., |h| h.crit)
                + if hero.is_none() && self.overlay.is_new_game_plus {
                    self.rules.new_game_plus_monster_crit_modifier
                } else {
                    0.
                }
        };
        Ok(Skill {
            tactical: {
                let count = r
                    .param("extra_targets_count")
                    .map(|_| integer(r, "extra_targets_count"))
                    .transpose()?
                    .unwrap_or(0);
                let chance = r
                    .param("extra_targets_chance")
                    .map(|_| number(r, "extra_targets_chance"))
                    .transpose()?
                    .unwrap_or(0.);
                if !(0..=3).contains(&count) || !(0. ..=1.).contains(&chance) {
                    return Err("invalid extra target count/chance".into());
                }
                TacticalSkill {
                    source_damage023: if !target.friendly
                        && crate::mechanisms023::enabled(
                            &self.overlay.config,
                            crate::mechanisms023::ROUNDING,
                        ) {
                        hero.map(|h| -> Result<([i32; 2], f64)> {
                            Ok((h.damage, 1. + number(r, "dmg")?))
                        })
                        .transpose()?
                    } else {
                        None
                    },
                    crit_valid: flag(r, "is_crit_valid", true)?,
                    boss: boss_parser::skill(r)?,
                    hp_range: r
                        .param("required_performer_hp_range")
                        .map(|p| -> Result<[f64; 2]> {
                            if p.values.len() != 2 {
                                return Err("invalid HP range".into());
                            }
                            let a = scalar(&p.values[0].text)?;
                            let b = scalar(&p.values[1].text)?;
                            if !(0. ..=1.).contains(&a) || !(a..=1.).contains(&b) {
                                return Err("invalid HP range".into());
                            }
                            Ok([a, b])
                        })
                        .transpose()?,
                    dmg_per_burn_stack: r
                        .param("dmg_per_burn_stack")
                        .map(|_| number(r, "dmg_per_burn_stack"))
                        .transpose()?
                        .unwrap_or(0.),
                    copy_burn_behind: flag(r, "copy_burn_behind", false)?,
                    valid_modes: r
                        .param("valid_modes")
                        .map(|p| p.values.iter().map(|v| v.text.clone()).collect())
                        .unwrap_or_default(),
                    mode_effects: ["human", "beast", "stance_defensive", "stance_aggressive"]
                        .into_iter()
                        .filter_map(|mode| r.param(&format!("{mode}_effects")).map(|p| (mode, p)))
                        .map(|(mode, p)| {
                            Ok((
                                mode.to_string(),
                                p.values
                                    .iter()
                                    .map(|v| self.effect(&v.text))
                                    .collect::<Result<Vec<_>>>()?,
                            ))
                        })
                        .collect::<Result<BTreeMap<_, _>>>()?,
                    refresh_each_wave: flag(r, "refresh_after_each_wave", false)?,
                    requires_burning: flag(r, "requires_burning", false)?,
                    stall_invalidating: flag(r, "is_stall_invalidating", true)?,
                    continue_turn: flag(r, "is_continue_turn", false)?,
                    per_turn_limit: r
                        .param("per_turn_limit")
                        .map(|_| {
                            integer(r, "per_turn_limit").and_then(|n| {
                                if (1..=10000).contains(&n) {
                                    Ok(n as u32)
                                } else {
                                    Err("invalid per_turn_limit".into())
                                }
                            })
                        })
                        .transpose()?,
                    attack_type: text(r, "type").unwrap_or("").into(),
                    self_target_valid: flag(r, "self_target_valid", true)?,
                    extra_targets_count: count as u8,
                    extra_targets_chance: chance,
                    ignore_guard: flag(r, "ignore_guard", false)?,
                    ignore_protection: flag(r, "ignore_protection", false)?,
                    can_be_riposted: flag(r, "can_be_riposted", true)?,
                }
            },
            id,
            ignore_stealth: flag(r, "ignore_stealth", false)?,
            launch: launch.ranks,
            target,
            accuracy,
            damage,
            crit,
            heal,
            effects,
            move_forward: movement.map_or(0, |p| p[1] as usize),
            per_battle_limit,
            move_back: movement.map_or(0, |p| p[0] as usize),
        })
    }
    pub(crate) fn effect_source(&self, name: &str) -> Result<&Record> {
        let r = if self
            .overlay
            .config
            .effect_choices
            .get(name)
            .map(String::as_str)
            == Some("capture")
        {
            let candidates = self
                .effects
                .records_keyed("effect")
                .filter(|r| {
                    text(r, "name").ok() == Some(name) && integer(r, "capture").ok() == Some(1)
                })
                .collect::<Vec<_>>();
            let first = candidates
                .first()
                .ok_or("selected capture definition is missing")?;
            if candidates
                .iter()
                .any(|r| !crate::library::same_record(first, r))
            {
                return Err("selected capture definition remains ambiguous".into());
            }
            *first
        } else if let Some(selected) = self.selected_mode_effects.get(name) {
            selected
        } else {
            named(&self.effects, "effect", name)?
        };
        Ok(r)
    }
    /// Resolve original attack-trigger definitions for expedition-injected buffs.
    /// Build the complete closure transactionally; unknown or conflicting IDs fail.
    pub fn bind_host_buff_effects(&self, blueprint: &mut Blueprint) -> Result<()> {
        use std::collections::BTreeSet;
        fn collect(kind: &EffectKind, pending: &mut BTreeSet<String>) {
            match kind {
                EffectKind::Buffs(bs)
                | EffectKind::ProtectedBuffs(bs)
                | EffectKind::InstantBuffs(bs)
                | EffectKind::Riposte { buffs: bs, .. } => {
                    for b in bs {
                        pending.extend(b.activation.attack_additional_effect_ids.iter().cloned());
                    }
                }
                EffectKind::Composite(es) => {
                    for e in es {
                        collect(&e.kind, pending);
                    }
                }
                EffectKind::BattlePersistent(k) | EffectKind::InventoryEffect(k) => {
                    collect(k, pending)
                }
                _ => {}
            }
        }
        let h = &blueprint.mechanics.hero;
        let mut pending = h
            .host_buffs
            .iter()
            .map(|b| &b.definition)
            .chain(h.host_passive_buffs.iter())
            .chain(h.exploration_buffs.iter().map(|b| &b.definition))
            .chain(
                h.carry
                    .iter()
                    .flat_map(|c| c.buffs.iter().map(|b| &b.definition)),
            )
            .flat_map(|b| b.activation.attack_additional_effect_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        if pending.is_empty() {
            return Ok(());
        }
        let mut effects = blueprint.mechanics.extra_effects.clone();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop_first() {
            if !visited.insert(id.clone()) {
                continue;
            }
            let effect = self.effect(&id)?;
            if effects.get(&id).is_some_and(|old| old != &effect) {
                return Err(format!("conflicting host trigger effect {id}"));
            }
            collect(&effect.kind, &mut pending);
            effects.insert(id, effect);
        }
        blueprint.mechanics.extra_effects = effects;
        Ok(())
    }
    pub fn effect(&self, name: &str) -> Result<Effect> {
        self.effect_from_record(self.effect_source(name)?, name)
    }
    pub(crate) fn effect_from_record(&self, r: &Record, name: &str) -> Result<Effect> {
        let execution = EffectExecution::from_record(r)?;
        let mut effect = (|| -> Result<Effect> {
            let mut normalized = r.clone();
            if self
                .overlay
                .config
                .effect_choices
                .get(name)
                .map(String::as_str)
                == Some("performer")
            {
                let target = normalized
                    .params
                    .iter_mut()
                    .find(|p| p.name == "target")
                    .ok_or("missing selected target")?;
                if target
                    .values
                    .iter()
                    .map(|v| v.text.as_str())
                    .collect::<Vec<_>>()
                    != vec!["performer", "target"]
                {
                    return Err("selected ambiguous target shape changed".into());
                }
                target.values.truncate(1);
            }
            if normalized.param("chance").is_none() {
                normalized.params.push(crate::darkest::Param {
                    name: "chance".into(),
                    values: vec![crate::darkest::Value {
                        text: "100%".into(),
                        quoted: false,
                    }],
                });
            }
            if normalized.param("duration").is_some()
                && integer(&normalized, "duration")? == -1
                && text(&normalized, "buff_duration_type").ok() == Some("combat_end")
            {
                return extended::parse(self, &normalized, name);
            }
            if normalized.param("duration").is_some()
                && integer(&normalized, "duration")? == -1
                && text(&normalized, "buff_duration_type").ok() != Some("combat_end")
            {
                let mut raw = normalized.clone();
                raw.params.retain(|p| p.name != "duration");
                raw.params.push(crate::darkest::Param {
                    name: "duration".into(),
                    values: vec![crate::darkest::Value {
                        text: "1".into(),
                        quoted: false,
                    }],
                });
                let mut e = if extended::applies(&raw) {
                    extended::parse(self, &raw, name)?
                } else {
                    self.effect_legacy(&raw, name)?
                };
                if !matches!(e.kind, EffectKind::Buffs(_) | EffectKind::Riposte { .. }) {
                    return Err("battle-persistent duration requires Buffs or riposte".into());
                }
                e.kind = EffectKind::BattlePersistent(Box::new(e.kind));
                return Ok(e);
            }
            let r = &normalized;
            if boss_effects::applies(r) {
                return boss_effects::parse(self, r, name);
            }
            if r.param("disease").is_some() {
                return context_effects::disease(r, name);
            }
            if trinket_effects::applies(r) {
                return trinket_effects::parse(self, r, name);
            }
            if r.param("skill_instant").is_some() {
                if !flag(r, "skill_instant", false)? {
                    return Err("false skill_instant is unsupported".into());
                }
                let mut raw = r.clone();
                raw.params.retain(|p| p.name != "skill_instant");
                let mut effect = if extended::applies(&raw) {
                    extended::parse(self, &raw, name)?
                } else {
                    self.effect_legacy(&raw, name)?
                };
                if let EffectKind::Buffs(b) = effect.kind {
                    effect.kind = EffectKind::InstantBuffs(b);
                } else {
                    return Err("skill_instant requires Buffs".into());
                }
                return Ok(effect);
            }
            if r.param("remove_vampire").is_some() {
                return context_effects::remove_vampire(r, name);
            }
            if r.param("controlled_burn_amount").is_some() {
                return context_effects::rank_burn(r, name);
            }
            if r.param("set_mode").is_some() || r.param("steal_buff_stat_type").is_some() {
                return context_effects::special(r, name);
            }
            if r.param("torch_increase").is_some() || r.param("torch_decrease").is_some() {
                return context_effects::parse(r, name);
            }
            if lifecycle_parser::has(r) {
                return lifecycle_parser::effect(self, r, name);
            }
            if extended::applies(r) {
                return extended::parse(self, r, name);
            }
            self.effect_legacy(r, name)
        })()?;
        fn execution_for_source(effect: &mut Effect, execution: &EffectExecution) {
            effect.execution = execution.clone();
            if let EffectKind::Composite(children) = &mut effect.kind {
                for child in children {
                    execution_for_source(child, execution);
                }
            }
        }
        execution_for_source(&mut effect, &execution);
        Ok(effect)
    }

    fn effect_legacy(&self, r: &Record, name: &str) -> Result<Effect> {
        let allowed = [
            "name",
            "item",
            "dotStress",
            "dotSource",
            "clearDotStress",
            "target",
            "curio_result_type",
            "chance",
            "dotBleed",
            "dotPoison",
            "stun",
            "stress",
            "healstress",
            "heal",
            "tag",
            "duration",
            "on_hit",
            "on_miss",
            "queue",
            "can_apply_on_death",
            "apply_once",
            "buff_ids",
            "buff_source_type",
            "buff_duration_type",
            "combat_stat_buff",
            "debuff",
            "attack_rating_add",
            "defense_rating_add",
            "protection_rating_add",
            "speed_rating_add",
            "crit_chance_add",
            "damage_low_multiply",
            "damage_high_multiply",
            "source_heal_type",
            "can_crit_heal",
            "crit_doesnt_apply_to_roll",
            "keyStatus",
            "monsterType",
            "push",
            "pull",
            "shuffletarget",
            "stealth",
            "unstealth",
        ];
        for p in &r.params {
            if !allowed.contains(&p.name.as_str()) {
                return Err(format!("{name}: unsupported effect parameter .{}", p.name));
            }
        }
        if r.param("item").is_some() && integer(r, "item")? != 1 {
            return Err(format!("{name}: unsupported item flag"));
        }
        let target = match text(r, "target")? {
            "target" => EffectTarget::Target,
            "target_enemy_group" => EffectTarget::TargetEnemies,
            "performer" => EffectTarget::Performer,
            "performer_group" => EffectTarget::Party,
            "performer_group_other" => EffectTarget::PartyOther,
            other => return Err(format!("{name}: unsupported effect target {other}")),
        };
        let mut conditions = vec![];
        for (field, status) in [("keyStatus", true), ("monsterType", false)] {
            if r.param(field).is_some() {
                let value = text(r, field)?.to_owned();
                let condition = if status {
                    Condition::Status(value)
                } else {
                    Condition::MonsterType(value)
                };
                condition.validate()?;
                conditions.push(condition);
            }
        }
        let mut buffs = vec![];
        if let Some(p) = r.param("buff_ids") {
            for id in &p.values {
                buffs.push(read_buff(&self.buffs, &id.text)?);
            }
        }
        for (field, stat_type, sub) in [
            ("attack_rating_add", "combat_stat_add", "attack_rating"),
            ("defense_rating_add", "combat_stat_add", "defense_rating"),
            (
                "protection_rating_add",
                "combat_stat_add",
                "protection_rating",
            ),
            ("speed_rating_add", "combat_stat_add", "speed_rating"),
            ("crit_chance_add", "combat_stat_add", "crit_chance"),
            ("damage_low_multiply", "combat_stat_multiply", "damage_low"),
            (
                "damage_high_multiply",
                "combat_stat_multiply",
                "damage_high",
            ),
        ] {
            if r.param(field).is_some() {
                buffs.push(Buff {
                    activation: Default::default(),
                    id: format!("{name}:{field}"),
                    stat_type: stat_type.into(),
                    stat_sub_type: sub.into(),
                    amount: number(r, field)?,
                    duration: None,
                    duration_type: None,
                    rule_type: "always".into(),
                    is_false_rule: false,
                    clearable: true,
                    force_positive: false,
                    rule_data: crate::mechanics::RuleData::default(),
                });
            }
        }
        let kinds = [
            "dotStress",
            "clearDotStress",
            "dotBleed",
            "dotPoison",
            "stun",
            "stress",
            "healstress",
            "heal",
            "tag",
            "push",
            "pull",
            "shuffletarget",
            "stealth",
            "unstealth",
        ]
        .iter()
        .filter(|n| r.param(n).is_some())
        .count()
            + usize::from(!buffs.is_empty());
        if kinds != 1 {
            return Err(format!("{name}: expected one supported effect operation"));
        }
        let distance = |field| -> Result<u8> {
            let n = integer(r, field)?;
            if !(1..=4).contains(&n) {
                return Err(format!("{name}: invalid displacement distance"));
            }
            Ok(n as u8)
        };
        let kind = if r.param("dotStress").is_some() {
            if r.param("dotSource").is_some() {
                if text(r, "dotSource")? != "bsrc_flashback_start" {
                    return Err("unsupported Horror source".into());
                }
                EffectKind::ProtectedHorror(integer(r, "dotStress")?)
            } else {
                EffectKind::Horror(integer(r, "dotStress")?)
            }
        } else if r.param("clearDotStress").is_some() {
            if integer(r, "clearDotStress")? != 1 {
                return Err(format!("{name}: unsupported clearDotStress value"));
            }
            EffectKind::ClearHorror
        } else if r.param("stealth").is_some() || r.param("unstealth").is_some() {
            let field = if r.param("stealth").is_some() {
                "stealth"
            } else {
                "unstealth"
            };
            if integer(r, field)? != 1 {
                return Err(format!("{name}: unsupported {field} value"));
            }
            if field == "stealth" {
                EffectKind::Stealth
            } else {
                EffectKind::Unstealth
            }
        } else if r.param("push").is_some() {
            EffectKind::Push(distance("push")?)
        } else if r.param("pull").is_some() {
            EffectKind::Pull(distance("pull")?)
        } else if let Some(p) = r.param("shuffletarget") {
            if !p.values.is_empty() {
                return Err(format!("{name}: unsupported shuffletarget value"));
            }
            EffectKind::Shuffle
        } else if !buffs.is_empty() {
            for b in &mut buffs {
                if r.param("duration").is_some() {
                    b.duration = Some(
                        integer(r, "duration")?
                            .try_into()
                            .map_err(|_| "invalid effect duration")?,
                    );
                }
                if let Some(p) = r.param("buff_duration_type") {
                    b.duration_type = p.values.first().map(|v| v.text.clone());
                }
                b.validate()?;
            }
            EffectKind::Buffs(buffs)
        } else if r.param("dotBleed").is_some() {
            EffectKind::Bleed(integer(r, "dotBleed")?)
        } else if r.param("dotPoison").is_some() {
            EffectKind::Blight(integer(r, "dotPoison")?)
        } else if r.param("stress").is_some() {
            EffectKind::Stress(integer(r, "stress")?)
        } else if r.param("healstress").is_some() {
            EffectKind::Stress(-integer(r, "healstress")?)
        } else if r.param("heal").is_some() {
            EffectKind::Heal(integer(r, "heal")?)
        } else if r.param("tag").is_some() {
            EffectKind::Mark
        } else {
            EffectKind::Stun
        };
        let duration = if matches!(
            kind,
            EffectKind::Bleed(_)
                | EffectKind::Blight(_)
                | EffectKind::Horror(_)
                | EffectKind::ProtectedHorror(_)
                | EffectKind::Mark
                | EffectKind::Stealth
        ) {
            integer(r, "duration")?
                .try_into()
                .map_err(|_| "invalid duration")?
        } else {
            1
        };
        if matches!(kind, EffectKind::Stealth) && !(1..=10000).contains(&duration) {
            return Err(format!("{name}: invalid stealth duration"));
        }
        if matches!(kind, EffectKind::Horror(n)|EffectKind::ProtectedHorror(n) if !(0..=1000000).contains(&n))
            || (matches!(kind, EffectKind::Horror(_) | EffectKind::ProtectedHorror(_))
                && !(1..=10000).contains(&duration))
        {
            return Err(format!("{name}: invalid Horror value or duration"));
        }
        if !conditions.is_empty()
            && (target != EffectTarget::Performer
                || !matches!(kind, EffectKind::Buffs(_))
                || !flag(r, "on_hit", true)?
                || flag(r, "on_miss", false)?)
        {
            return Err(format!("{name}: unsupported conditional effect shape"));
        }
        Ok(Effect {
            execution: EffectExecution::from_record(r)?,
            name: name.into(),
            kind: kind.clone(),
            chance: number(r, "chance")?,
            duration,
            target,
            on_hit: flag(r, "on_hit", true)?,
            on_miss: flag(r, "on_miss", false)?,
            apply_once: flag(r, "apply_once", false)?,
            resist_debuff: flag(r, "debuff", false)?
                || (text(r, "curio_result_type").ok() == Some("negative")
                    && matches!(kind, EffectKind::Buffs(_))),
            conditions,
            can_apply_on_death: flag(r, "can_apply_on_death", false)?,
        })
    }
}
fn body_size(d: &Document) -> Result<u8> {
    let size = integer(one(d, "display")?, "size")?;
    if !(1..=4).contains(&size) {
        return Err(format!("unsupported body size {size}: expected 1..4"));
    }
    Ok(size as u8)
}
fn monster_stats(r: &Record) -> Result<Stats> {
    Ok(Stats {
        hp: integer(r, "hp")?,
        dodge: number(r, "def")?,
        protection: number(r, "prot")?,
        speed: integer(r, "spd")?,
        damage: [0, 0],
        accuracy: 0.,
        crit: 0.,
        death_resist: 0.,
        stun_resist: number(r, "stun_resist")?,
        bleed_resist: number(r, "bleed_resist")?,
        blight_resist: number(r, "poison_resist")?,
        debuff_resist: number(r, "debuff_resist")?,
        move_resist: number(r, "move_resist")?,
    })
}



fn select_campaign018_camp_buff(raw: &serde_json::Value) -> Result<serde_json::Value> {
    let variants = raw
        .as_array()
        .filter(|v| v.len() == 2)
        .ok_or("camp Buff conflict shape changed")?;
    let mut normalized = vec![];
    let mut amounts = vec![];
    for value in variants {
        if value["id"] != "QUIRK_in_camp_HPRECOVERY_D"
            || value["stat_type"] != "hp_heal_percent"
            || value["stat_sub_type"] != ""
            || value["rule_type"] != "in_camp"
            || value["is_false_rule"] != false
        {
            return Err("camp Buff conflict alternatives changed".into());
        }
        let mut object = value.as_object().ok_or("invalid camp Buff record")?.clone();
        let amount = object
            .remove("amount")
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v < 0.)
            .ok_or("invalid original camp Buff amount")?;
        normalized.push(object);
        amounts.push(amount);
    }
    if normalized[0] != normalized[1] || amounts[0] == amounts[1] {
        return Err("camp Buff definitions differ beyond original penalty".into());
    }
    Ok(variants[usize::from(amounts[1] < amounts[0])].clone())
}
