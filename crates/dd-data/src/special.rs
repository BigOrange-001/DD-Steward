//! Boss contracts are parsed from enabled originals; execution is in dd-core.
use crate::{combat::Result, lifecycle::Choice};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossSpec {
    pub wave_front: Option<bool>,
    pub shared_health: Option<String>,
    pub shifter: Option<Shifter>,
    pub tags: Vec<String>,
    pub modes: Vec<Mode>,
    pub restrictions: Restrictions,
    pub empty: Option<CaptorEmpty>,
    pub full: Option<CaptorFull>,
    pub controller_stress: Option<i32>,
    pub companion: Option<Companion>,
    pub reactions: Reactions,
    pub death_damage: Option<(String, i32)>,
    pub torch: Option<[u8; 2]>,
    pub backdrop: Option<(String, String, bool)>,
    pub audio: Vec<(String, f64)>,
    pub tutorial: Option<String>,
    pub end_infestation: bool,
    pub death: DeathContract,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub choice: Choice,
    pub ranks: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Shifter {
    pub forms: Vec<Form>,
    pub frequency: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Restrictions {
    pub no_direct_damage: bool,
    pub no_trinket_target: bool,
    pub no_trinket_attacker: bool,
    pub no_crit_effects: bool,
    pub cannot_miss: bool,
    pub no_kill_relief: bool,
    pub no_crit_relief: bool,
    pub no_brain_size: bool,
    pub no_brain_guard: bool,
    pub no_stall_count: bool,
    pub block_retreat_actout: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    pub id: String,
    pub initial: bool,
    pub guard_families: Vec<String>,
    pub targetable: bool,
    pub keep_rank_clock: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptorEmpty {
    pub performer: String,
    pub full: String,
    pub effects: Vec<crate::combat::Effect>,
    pub reset_hp: bool,
    pub count_damage: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptorFull {
    pub empty: String,
    pub release_on_death: bool,
    pub release_on_door: bool,
    pub release_on_affliction: bool,
    pub damage: f64,
    pub stress: i32,
    pub switch_on_death: bool,
    pub reset_hp: bool,
    pub previous_hp: bool,
    pub add_hp: bool,
    pub effects: Vec<crate::combat::Effect>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Companion {
    pub family: String,
    pub buffs: Vec<crate::mechanics::Buff>,
    pub heal: f64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reactions {
    pub hit_performer: Vec<crate::combat::Effect>,
    pub killed_others: Vec<crate::combat::Effect>,
    pub killed_by_hero: Vec<crate::combat::Effect>,
}
impl BossSpec {
    pub fn validate(&self) -> Result<()> {
        if self.shared_health.is_some() != self.shifter.is_some() {
            return Err("incomplete shared-health/shifter lifecycle".into());
        }
        if self
            .shared_health
            .as_ref()
            .is_some_and(|id| id != "formless")
            || self.tags.iter().any(|tag| tag != "boss")
        {
            return Err("unknown boss lifecycle identity/tag".into());
        }
        if let Some(s) = &self.shifter {
            if s.forms.is_empty() || s.forms.len() > 64 || !(1..=10000).contains(&s.frequency) {
                return Err("invalid shape-shifter lifecycle".into());
            }
            let mut weight = 0.;
            let mut ids = std::collections::BTreeSet::new();
            for f in &s.forms {
                f.choice.validate()?;
                if !ids.insert(&f.choice.class)
                    || f.ranks.is_empty()
                    || f.ranks.iter().any(|r| !(1..=4).contains(r))
                {
                    return Err("invalid shape-shifter choices/ranks".into());
                }
                weight += f.choice.weight;
            }
            if !weight.is_finite() || weight <= 0. {
                return Err("invalid shape-shifter weights".into());
            }
        }
        let mut modes = std::collections::BTreeSet::new();
        if self
            .modes
            .iter()
            .any(|m| !modes.insert(&m.id) || m.guard_families.iter().any(|s| s.is_empty()))
            || (!self.modes.is_empty() && self.modes.iter().filter(|m| m.initial).count() != 1)
        {
            return Err("invalid monster modes/default".into());
        }
        if self
            .controller_stress
            .is_some_and(|n| !(0..=1000000).contains(&n))
            || self
                .death_damage
                .as_ref()
                .is_some_and(|(f, n)| f.is_empty() || !(1..=1000000).contains(n))
            || self.torch.is_some_and(|[a, b]| a > b || b > 100)
        {
            return Err("invalid boss numeric contract".into());
        }
        if let Some(c) = &self.full
            && (c.empty.is_empty()
                || !(0. ..=1.).contains(&c.damage)
                || !(0..=1000000).contains(&c.stress))
        {
            return Err("invalid captor full".into());
        }
        if let Some(c) = &self.empty
            && (c.performer.is_empty() || c.full.is_empty())
        {
            return Err("invalid captor empty".into());
        }
        if let Some(c) = &self.companion {
            if c.family.is_empty() || !(0. ..=1.).contains(&c.heal) {
                return Err("invalid companion".into());
            }
            for b in &c.buffs {
                b.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum BossOp {
    PartyStackBuffs(Vec<crate::mechanics::Buff>),
    Capture { remove: bool, virtue_block: f64 },
    Control { turns: u32 },
    ActorDot(ActorDot),
    Presentation,
    Damage(i32),
    InstantShuffle,
    ShuffleDot { turns: u32 },
    Immobilize { enable: bool, duration: Option<u32> },
    RankTarget,
    ClearRanks(Vec<u8>),
    Bark(String),
    StealSkillBuffs,
}
impl BossOp {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::PartyStackBuffs(buffs) => {
                if buffs.is_empty() {
                    return Err("empty stacking battle item buffs".into());
                }
                for b in buffs {
                    b.validate()?;
                }
            }
            Self::ActorDot(d) => d.validate()?,
            Self::ShuffleDot { turns } if !(1..=10000).contains(turns) => {
                return Err("invalid shuffle duration".into());
            }
            Self::Control { turns } if !(1..=10000).contains(turns) => {
                return Err("invalid control duration".into());
            }
            Self::Damage(n) if !(0..=1000000).contains(n) => {
                return Err("invalid boss damage".into());
            }
            Self::Capture { virtue_block, .. } if !(0. ..=1.).contains(virtue_block) => {
                return Err("invalid virtue block chance".into());
            }
            Self::Immobilize {
                duration: Some(n), ..
            } if !(1..=10000).contains(n) => return Err("invalid immobilize duration".into()),
            Self::Bark(s) if s.is_empty() => return Err("empty boss bark".into()),
            Self::ClearRanks(rs) if rs.is_empty() || rs.iter().any(|r| !(1..=4).contains(r)) => {
                return Err("invalid marked rank clear".into());
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActorDot {
    pub id: String,
    pub stages: Vec<ActorDotStage>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActorDotStage {
    pub chance: f64,
    pub completion: Vec<crate::combat::Effect>,
    pub increment: Vec<crate::combat::Effect>,
}
impl ActorDot {
    pub fn validate(&self) -> Result<()> {
        if self.id != "egg"
            || self.stages.is_empty()
            || self.stages.len() > 64
            || self.stages.last().is_none_or(|s| s.chance != 1.)
            || self.stages.iter().any(|s| !(0. ..=1.).contains(&s.chance))
        {
            return Err("invalid actor DoT stages".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossSkill {
    pub can_miss: bool,
    pub knowledgeable: bool,
    pub user_selected: bool,
    pub starting_cooldown: u32,
    pub rank_damage: [f64; 4],
    pub damage_heal: Vec<String>,
    pub teleport: bool,
    pub presentation_rerank: bool,
}
impl Default for BossSkill {
    fn default() -> Self {
        Self {
            can_miss: true,
            knowledgeable: true,
            user_selected: false,
            starting_cooldown: 0,
            rank_damage: [0.; 4],
            damage_heal: vec![],
            teleport: false,
            presentation_rerank: true,
        }
    }
}
impl BossSkill {
    pub fn validate(&self) -> Result<()> {
        if self.starting_cooldown > 10000
            || self
                .rank_damage
                .iter()
                .any(|n| !n.is_finite() || !(-1. ..=1000.).contains(n))
            || self.damage_heal.iter().any(|s| s.is_empty())
        {
            return Err("invalid boss skill".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeathContract {
    pub transform_instead: bool,
    pub carry_min: Option<f64>,
    pub previous_hp: bool,
    pub change_effects: Vec<crate::combat::Effect>,
}
