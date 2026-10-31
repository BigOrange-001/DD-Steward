//! Validated combat mechanics read from original JSON. Weight combination and
//! duration semantics are interpreted by the explicitly experimental core.
use crate::combat::{Result, Rules};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Buff {
    #[serde(default)]
    pub activation: Activation,
    #[serde(default = "default_clearable", rename = "is_clear_debuff_valid")]
    pub clearable: bool,
    #[serde(default)]
    pub force_positive: bool,
    pub id: String,
    pub stat_type: String,
    pub stat_sub_type: String,
    pub amount: f64,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub duration_type: Option<String>,
    pub rule_type: String,
    #[serde(default)]
    pub is_false_rule: bool,
    #[serde(default)]
    pub rule_data: RuleData,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuleData {
    #[serde(default)]
    pub string: String,
    #[serde(default)]
    pub float: Option<f64>,
}
fn default_clearable() -> bool {
    true
}
impl Buff {
    pub fn validate(&self) -> Result<()> {
        let known = match self.stat_type.as_str() {
            "stress_on_miss" if self.stat_sub_type == "STRESS_AMOUNT" => true,
            "stress_dmg_received_percent" if self.stat_sub_type == "mode" => true,
            "hp_heal_received_percent" if self.stat_sub_type == "eat" => true,
            "damage_reflect_percent" if self.stat_sub_type == "reflected_dmg" => true,
            "disable_combat_skill_attribute" => {
                ["heal", "guard"].contains(&self.stat_sub_type.as_str())
                    && [0., 1.].contains(&self.amount)
            }
            "combat_stat_add" | "combat_stat_multiply" => [
                "attack_rating",
                "defense_rating",
                "protection_rating",
                "speed_rating",
                "damage_low",
                "damage_high",
                "max_hp",
                "crit_chance",
            ]
            .contains(&self.stat_sub_type.as_str()),
            "resistance" => [
                "stun",
                "poison",
                "bleed",
                "disease",
                "move",
                "debuff",
                "death_blow",
                "trap",
            ]
            .contains(&self.stat_sub_type.as_str()),
            "stress_dmg_received_percent"
            | "stress_heal_percent"
            | "hp_heal_percent"
            | "hp_heal_amount"
            | "hp_heal_received_percent"
            | "resolve_check_percent"
            | "bleed_chance"
            | "poison_chance" => self.stat_sub_type.is_empty(),
            "damage_received_percent"
            | "damage_reflect_percent"
            | "stun_chance"
            | "debuff_chance"
            | "move_chance"
            | "guard_blocked"
            | "tag_blocked"
            | "guard_duration_percent"
            | "ignore_protection"
            | "hp_dot_burn_amount_percent"
            | "hp_dot_burn_decay_percent"
            | "hp_heal_dot_duration_received_percent"
            | "hp_dot_poison_duration_percent"
            | "stress_heal_received_percent" => self.stat_sub_type.is_empty(),
            "hp_dot_heal"
            | "stress_dmg_percent"
            | "random_target_attack_chance"
            | "wildfire_apply_adjacent_targets"
            | "vampire_evolution_duration"
            | "quirk_evolution_death_immune" => self.stat_sub_type.is_empty(),

            "crit_received_chance" => self.stat_sub_type.is_empty(),
            "stealth" | "ignore_stealth" => {
                self.stat_sub_type.is_empty()
                    && self.rule_type == "always"
                    && !self.is_false_rule
                    && [0., 1.].contains(&self.amount)
            }
            _ => false,
        };
        if !known {
            return Err(format!(
                "{}: unsupported Buff {}/{} or rule {}",
                self.id, self.stat_type, self.stat_sub_type, self.rule_type
            ));
        }
        self.validate_condition()
    }
    /// Shared condition/value validation for independently scoped consumers.
    pub fn validate_condition(&self) -> Result<()> {
        if ![
            "always",
            "afflicted",
            "virtued",
            "skill",
            "actorStatus",
            "is_actor_status",
            "monsterType",
            "is_stealthed",
            "in_rank",
            "target_in_rank",
            "hpabove",
            "hpbelow",
            "stress_above",
            "stress_below",
            "firstroundonly",
            "meleeonly",
            "rangedonly",
            "monsterSize",
            "has_buff",
            "is_guarding",
            "is_guarded",
            "target_is_guarded",
            "target_is_burning",
            "any_is_burning",
            "actor_base_class",
            "at_deaths_door",
            "riposte",
            "lightabove",
            "lightbelow",
            "in_dungeon",
            "attacking_monster_type",
            "has_item_id",
            "has_quirk",
            "no_trinkets",
            "quest_uses_remaining_above",
            "negative_quirk_above",
            "is_vampire",
            "target_is_vampire",
            "monster_type_count_min",
            "in_mode",
            "in_corridor",
            "walking_backwards",
            "always_false",
            "never",
        ]
        .contains(&self.rule_type.as_str())
            || (self.rule_type == "skill" && self.rule_data.string.is_empty())
        {
            return Err(format!(
                "{}: unsupported Buff {}/{} or rule {}",
                self.id, self.stat_type, self.stat_sub_type, self.rule_type
            ));
        }
        match self.rule_type.as_str() {
            "actorStatus" | "is_actor_status" => {
                crate::combat::Condition::Status(self.rule_data.string.clone()).validate()?
            }
            "monsterType" => {
                crate::combat::Condition::MonsterType(self.rule_data.string.clone()).validate()?
            }
            _ => {}
        }
        if self.rule_data.float.is_some_and(|x| !x.is_finite()) {
            return Err("non-finite Buff rule value".into());
        }
        let numeric = [
            "hpabove",
            "hpbelow",
            "in_rank",
            "target_in_rank",
            "monsterSize",
            "stress_above",
            "stress_below",
            "lightabove",
            "lightbelow",
            "negative_quirk_above",
            "monster_type_count_min",
            "quest_uses_remaining_above",
        ]
        .contains(&self.rule_type.as_str());
        if numeric && self.rule_data.float.is_none() {
            return Err("missing Buff condition threshold".into());
        }
        let x = self.rule_data.float.unwrap_or(0.);
        if (["hpabove", "hpbelow"].contains(&self.rule_type.as_str()) && !(0. ..=1.).contains(&x))
            || (["in_rank", "target_in_rank"].contains(&self.rule_type.as_str())
                && (x.fract() != 0. || !(0. ..=3.).contains(&x)))
            || (self.rule_type == "monsterSize" && (x.fract() != 0. || !(1. ..=4.).contains(&x)))
            || (["stress_above", "stress_below"].contains(&self.rule_type.as_str())
                && !(0. ..=200.).contains(&x))
        {
            return Err("invalid Buff condition threshold".into());
        }
        if (["lightabove", "lightbelow"].contains(&self.rule_type.as_str())
            && !(0. ..=100.).contains(&x))
            || ([
                "negative_quirk_above",
                "monster_type_count_min",
                "quest_uses_remaining_above",
            ]
            .contains(&self.rule_type.as_str())
                && (x.fract() != 0. || !(0. ..=1000000.).contains(&x)))
        {
            return Err("invalid context Buff threshold".into());
        }
        if self
            .activation
            .attack_additional_effect_ids
            .iter()
            .chain(&self.activation.clear_buffs_on_rule_active)
            .any(|id| id.is_empty())
            || self.activation.attack_additional_effect_ids.len() > 64
        {
            return Err("invalid activation references".into());
        }
        if [
            "has_buff",
            "actor_base_class",
            "has_item_id",
            "has_quirk",
            "in_dungeon",
            "in_mode",
            "monster_type_count_min",
            "attacking_monster_type",
        ]
        .contains(&self.rule_type.as_str())
            && self.rule_data.string.is_empty()
        {
            return Err("missing Buff condition identity".into());
        }
        if !self.amount.is_finite()
            || self.amount.abs() > 1000.
            || self.duration.is_some_and(|n| n == 0 || n > 10000)
            || self.duration_type.as_ref().is_some_and(|t| {
                !["combat_end", "quest_end", "round", "before_turn"].contains(&t.as_str())
            })
        {
            return Err(format!("{}: invalid Buff value/duration", self.id));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Activation {
    pub remove_if_not_active: bool,
    pub attack_additional_effect_ids: Vec<String>,
    pub clear_buffs_on_rule_active: Vec<String>,
    pub consume_on_rule_active: bool,
    pub remove_on_battle_complete: bool,
}
impl Default for Activation {
    fn default() -> Self {
        Self {
            remove_if_not_active: false,
            attack_additional_effect_ids: vec![],
            clear_buffs_on_rule_active: vec![],
            consume_on_rule_active: false,
            remove_on_battle_complete: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Brain {
    pub id: String,
    #[serde(default)]
    pub ignore_desires_on_single_valid_skill: Option<bool>,
    pub skill_cooldowns: Vec<Cooldown>,
    pub skill_selection_desires: Vec<Desire>,
    pub target_selection_desires: Vec<Desire>,
    pub bonus_initiative_desires: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cooldown {
    pub combat_skill_id: String,
    pub amount: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Desire {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: DesireData,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesireData {
    pub base_chance: f64,
    #[serde(default)]
    pub combat_skill_id: String,
    #[serde(default)]
    pub specific_combat_skill_id: String,
    #[serde(default)]
    pub is_exclusive_desire: bool,
    #[serde(default)]
    pub is_enemy_target_desire: bool,
    #[serde(default)]
    pub is_friendly_target_desire: bool,
    #[serde(default)]
    pub is_greater_comparison: bool,
    #[serde(default)]
    pub hp_ratio_threshold: Option<f64>,
    #[serde(default)]
    pub performing_turn: Option<u32>,
    #[serde(default)]
    pub can_target_not_overstressed: bool,
    #[serde(default)]
    pub can_target_afflicted: bool,
    #[serde(default)]
    pub can_target_virtued: bool,
    #[serde(default)]
    pub rank: Option<u8>,
    #[serde(default)]
    pub resistance: String,
    /// Preserved, explicitly validated tactical predicates. Never ignored.
    #[serde(flatten)]
    pub predicates: BTreeMap<String, Value>,
}
impl Brain {
    pub fn validate(&self, skills: &[String]) -> Result<()> {
        self.validate_candidates(skills, false)
    }
    pub fn validate_candidates(&self, skills: &[String], known_inactive: bool) -> Result<()> {
        crate::bonus::validate(&self.bonus_initiative_desires, skills)?;
        for c in &self.skill_cooldowns {
            if c.amount > 10000 || !skills.contains(&c.combat_skill_id) {
                return Err(format!("{}: invalid cooldown", self.id));
            }
        }
        for (target, desires) in [
            (false, &self.skill_selection_desires),
            (true, &self.target_selection_desires),
        ] {
            for d in desires {
                let allowed = if target {
                    [
                        "random_target",
                        "marked_target",
                        "stunned_target",
                        "stress_target",
                        "health_target",
                        "rank_target",
                        "resistance_target",
                        "ally_class_target",
                        "guard_target",
                        "next_initiative_target",
                        "fill_ally_captor_empty_target",
                        "actor_dot_target",
                        "trinket_target",
                        "vampire_target",
                    ]
                    .as_slice()
                } else {
                    [
                        "specific_skill",
                        "random_skill",
                        "preferred_skill",
                        "performing_turn_skill",
                        "stun_skill",
                        "heal_skill",
                        "effect_key_status_skill",
                        "ally_alive_skill",
                        "ally_dead_skill",
                        "ally_hp_ratio_skill",
                        "performer_dot_skill",
                        "performer_guard_skill",
                        "fill_ally_captor_empty_skill",
                    ]
                    .as_slice()
                };
                if !allowed.contains(&d.kind.as_str()) {
                    return Err(format!("{}: unsupported AI desire {}", self.id, d.kind));
                }
                if !d.data.base_chance.is_finite() {
                    return Err("invalid AI weight".into());
                }
                crate::ai::validate_predicates(d, target)?;
                for id in [&d.data.combat_skill_id, &d.data.specific_combat_skill_id] {
                    let retained_inactive = known_inactive
                        && ((self.id == "baron" && target && id == "baron_cocoon_buff")
                            || (self.id == "spire"
                                && ((target && id == "grow_seed_blue")
                                    || (!target
                                        && id == "sprout_to_seed_grey"
                                        && d.data.base_chance < 0.
                                        && crate::ai::number(d, "per_round_chance")
                                            .is_none_or(|n| n == 0.))))
                            || (["plowhorse_A", "plowhorse_B", "plowhorse_C"]
                                .contains(&self.id.as_str())
                                && !target
                                && id == "stumble"
                                && d.kind == "specific_skill"
                                && d.data.base_chance == 0.
                                && crate::ai::number(d, "per_round_chance")
                                    .is_none_or(|n| n == 0.)));
                    if !id.is_empty() && !skills.contains(id) && !retained_inactive {
                        return Err(format!("{}: AI references unknown skill {id}", self.id));
                    }
                }
                if d.kind == "specific_skill"
                    && d.data.combat_skill_id.is_empty()
                    && skills.len() > 1
                {
                    return Err("specific_skill needs id".into());
                }
                if d.kind == "performing_turn_skill"
                    && (d.data.combat_skill_id.is_empty()
                        || d.data.performing_turn.is_none_or(|n| n == 0))
                {
                    return Err("performing_turn_skill needs id and positive turn".into());
                }
                if (d.kind == "heal_skill" || d.data.hp_ratio_threshold.is_some())
                    && d.data
                        .hp_ratio_threshold
                        .is_none_or(|n| !(0. ..=1.).contains(&n))
                {
                    return Err("heal_skill needs HP threshold".into());
                }
                if d.kind == "rank_target" && d.data.rank.is_some_and(|n| !(1..=4).contains(&n)) {
                    return Err("rank_target needs rank 1..4".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ActorMechanics {
    pub boss: crate::special::BossSpec,
    pub build: crate::build010::BuildState,
    pub disease_resist: f64,
    pub extra_effects: BTreeMap<String, crate::combat::Effect>,
    pub hero: crate::hero010::HeroRules,
    pub lifecycle: crate::lifecycle::LifeSpec,
    pub family: String,
    pub life_link: Option<String>,
    pub riposte: Option<Box<crate::combat::Skill>>,
    pub enemy_types: Vec<String>,
    pub deaths_door: Vec<Buff>,
    pub recovery: Vec<Buff>,
    pub heart_recovery: Vec<Buff>,
    pub on_crit: Vec<Buff>,
    pub brain: Option<Brain>,
    pub preferred_skill: Option<usize>,
    pub trait_effects: BTreeMap<String, crate::combat::Effect>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnKind {
    Nothing,
    BarkStress,
    ChangePos,
    IgnoreCommand,
    RandomCommand,
    AttackFriendly,
    AttackSelf,
    MarkSelf,
    StressHealSelf,
    StressHealParty,
    BuffRandomPartyMember,
    BuffParty,
    HealSelf,
    RetreatFromCombat,
    ConsumeItem,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurnAct {
    pub id: TurnKind,
    pub data: TurnData,
    pub chance: f64,
    #[serde(default)]
    pub valid_hero_class_ids: Vec<String>,
    #[serde(default)]
    pub raid_limit: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurnData {
    pub number_value: f64,
    pub string_value: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReactionKind {
    BlockMove,
    BlockHeal,
    BlockBuff,
    BlockItem,
    BlockCombatRetreat,
    BlockEffect,
    CommentSelfHit,
    CommentSelfMissed,
    CommentAllyHit,
    CommentAllyMissed,
    CommentAllyAttackHit,
    CommentAllyAttackMissed,
    CommentMove,
    CommentCurioInteraction,
    CommentTrapTriggered,
    BlockCampingMeal,
    BlockCampingSkillPerformer,
    BlockCampingSkillTarget,
}
impl ReactionKind {
    pub fn is_block(self) -> bool {
        matches!(
            self,
            Self::BlockMove
                | Self::BlockHeal
                | Self::BlockBuff
                | Self::BlockItem
                | Self::BlockCombatRetreat
                | Self::BlockEffect
                | Self::BlockCampingMeal
                | Self::BlockCampingSkillPerformer
                | Self::BlockCampingSkillTarget
        )
    }
    pub fn is_combat(self) -> bool {
        !matches!(
            self,
            Self::BlockItem
                | Self::CommentCurioInteraction
                | Self::CommentTrapTriggered
                | Self::BlockCampingMeal
                | Self::BlockCampingSkillPerformer
                | Self::BlockCampingSkillTarget
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reaction {
    pub id: ReactionKind,
    pub data: ReactionData,
    pub chance: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReactionData {
    pub effect: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraitDef {
    pub generate_chance_modifier: f64,
    pub required_dungeon_history: Vec<String>,
    pub id: String,
    pub virtue: bool,
    pub buffs: Vec<Buff>,
    pub turn_acts: Vec<TurnAct>,
    pub reactions: Vec<Reaction>,
}
impl TraitDef {
    pub fn validate(&self) -> Result<()> {
        if !self.generate_chance_modifier.is_finite()
            || self.generate_chance_modifier.abs() > 1000000.
            || self.required_dungeon_history.iter().any(|id| {
                ![
                    "crypts",
                    "weald",
                    "warrens",
                    "cove",
                    "darkestdungeon",
                    "courtyard",
                    "farm",
                    "town",
                ]
                .contains(&id.as_str())
            })
        {
            return Err("invalid trait history metadata".into());
        }
        if self.id.is_empty()
            || self.turn_acts.is_empty()
            || self.turn_acts.len() > 256
            || self.reactions.len() > 64
        {
            return Err(format!("{}: invalid trait event table size", self.id));
        }
        for a in &self.turn_acts {
            if !a.chance.is_finite()
                || !(0. ..=1e6).contains(&a.chance)
                || !a.data.number_value.is_finite()
                || a.data.number_value.abs() > 1000.
                || a.raid_limit.is_some_and(|n| n == 0)
            {
                return Err(format!("{}: invalid act-out parameters", self.id));
            }
            if a.chance > 0.
                && (matches!(a.id, TurnKind::RetreatFromCombat) || a.raid_limit.is_some())
            {
                return Err(format!(
                    "{}: enabled inventory/raid-limited act-out {:?} is outside combat scope",
                    self.id, a.id
                ));
            }
            if a.id == TurnKind::ConsumeItem
                && (a.data.number_value != 0. || !a.data.string_value.is_empty())
            {
                return Err(format!("{}: unsupported consume-item parameters", self.id));
            }
            if a.chance > 0.
                && matches!(
                    a.id,
                    TurnKind::BarkStress
                        | TurnKind::MarkSelf
                        | TurnKind::StressHealSelf
                        | TurnKind::StressHealParty
                        | TurnKind::BuffRandomPartyMember
                        | TurnKind::BuffParty
                )
                && a.data.string_value.is_empty()
            {
                return Err(format!("{}: act-out missing effect", self.id));
            }
            if matches!(
                a.id,
                TurnKind::AttackSelf | TurnKind::AttackFriendly | TurnKind::HealSelf
            ) && !(0. ..=1.).contains(&a.data.number_value)
            {
                return Err(format!("{}: invalid fractional act-out amount", self.id));
            }
            if a.id == TurnKind::ChangePos
                && (a.data.number_value.fract() != 0. || a.data.number_value.abs() > 4.)
            {
                return Err(format!("{}: invalid act-out movement", self.id));
            }
        }
        let mut kinds = vec![];
        for r in &self.reactions {
            if !(0. ..=1.).contains(&r.chance) || kinds.contains(&r.id) {
                return Err(format!("{}: invalid or duplicate reaction", self.id));
            }
            if r.chance > 0. && !r.id.is_block() && r.data.effect.is_empty() {
                return Err(format!("{}: reaction missing effect", self.id));
            }
            kinds.push(r.id);
        }
        for b in &self.buffs {
            b.validate()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StressRules {
    pub affliction_base_onset_chance: f64,
    pub affliction_min_chance: f64,
    pub affliction_max_chance: f64,
    pub stress_affliction_resisted: f64,
    pub deaths_door_self_stress_chance: f64,
    pub deaths_door_companion_stress_chance: f64,
    pub deaths_door_self_stress_dmg: f64,
    pub deaths_door_companion_stress_dmg: f64,
    pub deaths_door_self_stress_chance_repeat: f64,
    pub deaths_door_companion_stress_chance_repeat: f64,
    pub deaths_door_self_stress_dmg_repeat: f64,
    pub deaths_door_companion_stress_dmg_repeat: f64,
    pub death_party_stress_chance: f64,
    pub death_party_stress_damage: f64,
    pub killing_blow_relief_chance: f64,
    pub killing_blow_stress_relief: f64,
    pub player_crit: CritStress,
    pub monster_crit: CritStress,
    pub stress_dmg_buff_min: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CritStress {
    #[serde(alias = "stress_chance")]
    pub relief_chance: f64,
    #[serde(alias = "stress_damage")]
    pub stress_relief: f64,
    #[serde(alias = "companion_stress_chance")]
    pub companion_relief_chance: f64,
    #[serde(alias = "companion_stress_damage")]
    pub companion_stress_relief: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CombatGlobals {
    pub stun_recovery: Vec<Buff>,
    pub traits: Vec<TraitDef>,
}
impl CombatGlobals {
    pub fn load(rules: &Value, library: &BTreeMap<String, Value>, traits: &Value) -> Result<Self> {
        Self::load_inner(rules, library, traits, false)
    }
    pub fn load_explicit(
        rules: &Value,
        library: &BTreeMap<String, Value>,
        traits: &Value,
    ) -> Result<Self> {
        Self::load_inner(rules, library, traits, true)
    }
    fn load_inner(
        rules: &Value,
        library: &BTreeMap<String, Value>,
        traits: &Value,
        explicit: bool,
    ) -> Result<Self> {
        let mut stun_recovery = vec![];
        for id in rules["stun_recovery_buffs"]
            .as_array()
            .ok_or("missing stun recovery list")?
        {
            stun_recovery.push(read_buff(
                library,
                id.as_str().ok_or("invalid recovery buff id")?,
            )?);
        }
        let mut definitions = vec![];
        for raw in traits["traits"].as_array().ok_or("missing traits")? {
            if !explicit && raw["is_generated"] != true {
                continue;
            }
            let virtue = match raw["overstress_type"].as_str() {
                Some("virtue") => true,
                Some("affliction") => false,
                _ => return Err("unknown overstress type".into()),
            };
            let buffs = raw["buff_ids"]
                .as_array()
                .ok_or("missing trait buffs")?
                .iter()
                .map(|s| read_buff(library, s.as_str().ok_or("invalid buff id")?))
                .collect::<Result<_>>()?;
            let definition = TraitDef {
                generate_chance_modifier: raw
                    .get("generate_chance_modifier")
                    .map(|v| v.as_f64().ok_or("invalid trait generation modifier"))
                    .transpose()?
                    .unwrap_or(0.),
                required_dungeon_history: raw
                    .get("required_dungeon_history")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()
                    .map_err(|e| e.to_string())?
                    .unwrap_or_default(),
                id: raw["id"].as_str().ok_or("missing trait id")?.into(),
                virtue,
                buffs,
                turn_acts: serde_json::from_value(raw["combat_start_turn_act_outs"].clone())
                    .map_err(|e| e.to_string())?,
                reactions: serde_json::from_value(raw["reaction_act_outs"].clone())
                    .map_err(|e| e.to_string())?,
            };
            definition.validate()?;
            definitions.push(definition);
        }
        if !explicit
            && (!definitions.iter().any(|t| t.virtue) || !definitions.iter().any(|t| !t.virtue))
        {
            return Err("missing affliction/virtue definitions".into());
        }
        Ok(Self {
            stun_recovery,
            traits: definitions,
        })
    }
}
pub fn read_buff(library: &BTreeMap<String, Value>, id: &str) -> Result<Buff> {
    read_buff_domain(library, id, false, false)
}
pub fn read_exploration_buff(library: &BTreeMap<String, Value>, id: &str) -> Result<Buff> {
    read_buff_domain(library, id, true, false)
}
/// A separately scoped camp consumer. The camp predicate is resolved only here,
/// never enabled for combat or ordinary exploration readers.
pub fn read_camping_buff(library: &BTreeMap<String, Value>, id: &str) -> Result<Buff> {
    read_buff_domain(library, id, false, true)
}
fn read_buff_domain(
    library: &BTreeMap<String, Value>,
    id: &str,
    exploration: bool,
    camping: bool,
) -> Result<Buff> {
    let raw = library
        .get(id)
        .ok_or_else(|| format!("missing Buff {id}"))?;
    if raw.is_array() {
        return Err(format!(
            "ambiguous Buff {id}: conflicting original definitions"
        ));
    }
    let mut canonical = raw.clone();
    let object = canonical
        .as_object_mut()
        .ok_or("Buff definition must be object")?;
    let allowed = [
        "id",
        "stat_type",
        "stat_sub_type",
        "amount",
        "duration",
        "duration_type",
        "rule_type",
        "is_false_rule",
        "rule_data",
        "is_clear_debuff_valid",
        "force_positive",
        "remove_if_not_active",
        "attack_additional_effect_ids",
        "clear_buffs_on_rule_active",
        "consume_on_rule_active",
        "remove_on_battle_complete",
        "description_tooltip_id",
        "fx",
        "has_description",
        "suppress_apply_poptext",
    ];
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{id}: unknown Buff field {key}"));
    }
    let mut activation = serde_json::Map::new();
    for key in [
        "remove_if_not_active",
        "attack_additional_effect_ids",
        "clear_buffs_on_rule_active",
        "consume_on_rule_active",
        "remove_on_battle_complete",
    ] {
        if let Some(value) = object.remove(key) {
            activation.insert(key.to_string(), value);
        }
    }
    object.insert("activation".into(), Value::Object(activation));
    for key in [
        "remove_if_not_active",
        "description_tooltip_id",
        "fx",
        "has_description",
        "suppress_apply_poptext",
    ] {
        object.remove(key);
    }
    if camping && object.get("rule_type").and_then(Value::as_str) == Some("in_camp") {
        object.insert("rule_type".into(), Value::String("always".into()));
    }
    let b: Buff = serde_json::from_value(canonical).map_err(|e| format!("Buff {id}: {e}"))?;
    if exploration || camping {
        let ordinary = [
            "food_consumption_percent",
            "starving_damage_percent",
            "resolve_xp_bonus_percent",
            "scouting_chance",
            "party_surprise_chance",
            "monsters_surprise_chance",
        ]
        .contains(&b.stat_type.as_str())
            && b.stat_sub_type.is_empty();
        let meal = (b.stat_type == "hp_heal_received_percent" && b.stat_sub_type == "eat")
            || (b.stat_type == "stress_dmg_received_percent" && b.stat_sub_type == "hunger");
        let camp = camping
            && ([
                "hp_heal_percent",
                "hp_heal_received_percent",
                "stress_heal_received_percent",
                "stress_dmg_received_percent",
            ]
            .contains(&b.stat_type.as_str())
                && ["", "eat", "camping_eat"].contains(&b.stat_sub_type.as_str()));
        if !ordinary && !meal && !camp && !(camping && b.validate().is_ok()) {
            return Err("unsupported exploration Buff statistic".into());
        }
        b.validate_condition()?;
    } else {
        b.validate()?;
    }
    Ok(b)
}
pub fn validate_stress(r: &Rules) -> Result<()> {
    let s = &r.stress;
    for p in [
        r.combat_retreat_chance,
        r.combat_retreat_bonus_chance_per_attempt,
        s.affliction_base_onset_chance,
        s.affliction_min_chance,
        s.affliction_max_chance,
        s.deaths_door_self_stress_chance,
        s.deaths_door_companion_stress_chance,
        s.deaths_door_self_stress_chance_repeat,
        s.deaths_door_companion_stress_chance_repeat,
        s.death_party_stress_chance,
        s.killing_blow_relief_chance,
        s.player_crit.relief_chance,
        s.player_crit.companion_relief_chance,
        s.monster_crit.relief_chance,
        s.monster_crit.companion_relief_chance,
    ] {
        if !(0. ..=1.).contains(&p) {
            return Err("invalid stress probability".into());
        }
    }
    if s.affliction_min_chance > s.affliction_max_chance {
        return Err("invalid resolve check bounds".into());
    }
    for n in [
        r.combat_retreat_stress,
        s.stress_affliction_resisted,
        s.deaths_door_self_stress_dmg,
        s.deaths_door_companion_stress_dmg,
        s.deaths_door_self_stress_dmg_repeat,
        s.deaths_door_companion_stress_dmg_repeat,
        s.death_party_stress_damage,
        s.killing_blow_stress_relief,
        s.player_crit.stress_relief,
        s.player_crit.companion_stress_relief,
        s.monster_crit.stress_relief,
        s.monster_crit.companion_stress_relief,
        r.passed_turn_stress_dmg,
    ] {
        if !n.is_finite() || !(0. ..=1000.).contains(&n) {
            return Err("invalid stress amount".into());
        }
    }
    if !(0. ..=1.).contains(&s.stress_dmg_buff_min) || s.stress_affliction_resisted >= 100. {
        return Err("invalid stress floor/resolve relief".into());
    }
    Ok(())
}
