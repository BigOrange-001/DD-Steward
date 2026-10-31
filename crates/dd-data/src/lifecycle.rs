//! Scheduling and identity contracts. All actor statistics remain parsed data.
use crate::combat::{Blueprint, Effect, EffectKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityRole {
    #[default]
    Fighter,
    Construct,
    Corpse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeSpec {
    pub disable_stall: bool,
    pub accelerate_stall: bool,
    pub can_surprise: bool,
    pub can_be_surprised: bool,
    pub always_surprise: bool,
    pub always_be_surprised: bool,
    pub aura_heroes: Vec<crate::mechanics::Buff>,
    pub aura_allies: Vec<crate::mechanics::Buff>,
    pub turns: u8,
    pub role: EntityRole,
    pub lifetime: Option<u32>,
    pub spawn: Vec<Effect>,
    pub wave_spawn: Vec<SpawnEffect>,
    pub targetable: bool,
    pub random_targetable: bool,
    pub friendly_targetable: bool,
    pub can_be_guarded: bool,
    pub can_be_summon_rank: bool,
    pub death_effects: Vec<Effect>,
    pub death_change: Option<ChangeSpec>,
    pub death_on_crit: bool,
    pub loot: bool,
    pub link_loot: bool,
}
impl Default for LifeSpec {
    fn default() -> Self {
        Self {
            disable_stall: false,
            accelerate_stall: false,
            can_surprise: true,
            can_be_surprised: true,
            always_surprise: false,
            always_be_surprised: false,
            aura_heroes: vec![],
            aura_allies: vec![],
            turns: 1,
            role: EntityRole::Fighter,
            lifetime: None,
            spawn: vec![],
            wave_spawn: vec![],
            targetable: true,
            random_targetable: true,
            friendly_targetable: true,
            can_be_guarded: true,
            can_be_summon_rank: false,
            death_effects: vec![],
            death_change: None,
            death_on_crit: true,
            loot: true,
            link_loot: true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnEffect {
    pub target_family: Option<String>,
    pub effect: Effect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub class: String,
    pub weight: f64,
    pub limit: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummonSpec {
    pub choices: Vec<Choice>,
    pub count: u8,
    pub ranks: Vec<u8>,
    pub roll_initiative: bool,
    pub replace_corpse: bool,
    pub loot: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSpec {
    pub choices: Vec<Choice>,
    pub reset_hp: bool,
    pub reset_buffs: bool,
    pub clear_initiative: bool,
    pub clear_cooldowns: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LifeOp {
    Boss(crate::special::BossOp),
    StealDots {
        kind: String,
    },
    Light {
        amount: i32,
    },
    Summon(SummonSpec),
    Change(ChangeSpec),
    Phase {
        change: ChangeSpec,
        effects: Vec<Effect>,
        carry_min: Option<f64>,
        overkill: i32,
    },
    Kill,
    Initiative {
        amount: i32,
        next_round: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEffect {
    pub op: LifeOp,
    pub queued: bool,
    pub requires_kill: bool,
}
impl Choice {
    pub fn validate(&self) -> Result<()> {
        if self.class.is_empty()
            || !self
                .class
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            || !self.weight.is_finite()
            || self.weight < 0.
            || self.limit.is_some_and(|n| n == 0 || n > 10000)
        {
            return Err("invalid lifecycle choice".into());
        }
        Ok(())
    }
}
fn choices(c: &[Choice]) -> Result<()> {
    if c.is_empty() || c.len() > 64 {
        return Err("invalid lifecycle choice list".into());
    }
    for x in c {
        x.validate()?;
    }
    let total: f64 = c.iter().map(|x| x.weight).sum();
    if !total.is_finite() || total <= 0. {
        return Err("invalid lifecycle weights".into());
    }
    Ok(())
}
impl LifeOp {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Boss(op) => op.validate()?,
            Self::StealDots { kind }
                if !["hp_dot_bleed", "hp_dot_poison"].contains(&kind.as_str()) =>
            {
                return Err("unsupported stolen status".into());
            }
            Self::Light { amount } if *amount == 0 || amount.unsigned_abs() > 10000 => {
                return Err("invalid lighting operation".into());
            }
            Self::Summon(s) => {
                choices(&s.choices)?;
                if s.count == 0 || s.count > 4 || s.ranks.iter().any(|r| !(1..=4).contains(r)) {
                    return Err("invalid summon count/ranks".into());
                }
            }
            Self::Change(c) => choices(&c.choices)?,
            Self::Phase {
                change,
                carry_min,
                overkill,
                ..
            } => {
                choices(&change.choices)?;
                if *overkill < 0 || carry_min.is_some_and(|n| !(0. ..=1.).contains(&n)) {
                    return Err("invalid phase carry".into());
                }
            }
            Self::Initiative {
                amount,
                next_round: true,
            } if *amount < 0 => return Err("negative next-round bonus is unsupported".into()),
            Self::Initiative { amount, .. } if *amount == 0 || amount.unsigned_abs() > 4 => {
                return Err("invalid initiative change".into());
            }
            _ => {}
        }
        Ok(())
    }
    pub fn references(&self) -> Vec<&str> {
        match self {
            Self::Summon(s) => s.choices.iter().map(|c| c.class.as_str()).collect(),
            Self::Change(s) | Self::Phase { change: s, .. } => {
                s.choices.iter().map(|c| c.class.as_str()).collect()
            }
            _ => vec![],
        }
    }
}
pub fn effect_references(e: &Effect, ids: &mut BTreeSet<String>) {
    match &e.kind {
        EffectKind::Lifecycle(l) => ids.extend(l.op.references().into_iter().map(str::to_string)),
        EffectKind::Composite(es) => {
            for e in es {
                effect_references(e, ids)
            }
        }
        _ => {}
    }
}
pub fn blueprint_references(b: &Blueprint) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if let Some(s) = &b.mechanics.boss.shifter {
        ids.extend(s.forms.iter().map(|f| f.choice.class.clone()));
    }
    if let Some(c) = &b.mechanics.boss.empty {
        ids.insert(c.full.clone());
    }
    if let Some(c) = &b.mechanics.boss.full {
        ids.insert(c.empty.clone());
    }
    for e in &b.mechanics.boss.death.change_effects {
        effect_references(e, &mut ids);
    }
    for e in b
        .skills
        .iter()
        .flat_map(|s| &s.effects)
        .chain(&b.mechanics.lifecycle.spawn)
        .chain(&b.mechanics.lifecycle.death_effects)
        .chain(b.mechanics.lifecycle.wave_spawn.iter().map(|s| &s.effect))
    {
        effect_references(e, &mut ids);
    }
    if let Some(c) = &b.mechanics.lifecycle.death_change {
        ids.extend(c.choices.iter().map(|c| c.class.clone()));
    }
    ids
}
impl crate::combat::GameData {
    /// Iterative closure handles legitimate self/mutual summon references.
    pub fn entity_catalog(&self, roots: &[&str]) -> Result<BTreeMap<String, Blueprint>> {
        let mut found = BTreeMap::new();
        let mut pending = roots.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        while let Some(id) = pending.pop() {
            if found.contains_key(&id) {
                continue;
            }
            let b = self.monster(&id)?;
            pending.extend(blueprint_references(&b));
            found.insert(id, b);
            if found.len() > 10000 {
                return Err("entity catalog limit".into());
            }
        }
        Ok(found)
    }
}
