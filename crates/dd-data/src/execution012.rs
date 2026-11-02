use crate::combat::{Result, flag, text};
use crate::darkest::Record;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EffectExecution {
    pub queued: bool,
    pub critical_roll_bonus: bool,
    pub critical_heal: Option<bool>,
    pub source_heal_type: Option<String>,
}
impl EffectExecution {
    pub fn allows_critical_heal(&self, skill_source: bool, restoration: bool) -> bool {
        self.critical_heal.unwrap_or(
            skill_source
                || restoration
                || self
                    .source_heal_type
                    .as_deref()
                    .is_some_and(|s| s != "curio"),
        )
    }
    pub(crate) fn from_record(r: &Record) -> Result<Self> {
        let source_heal_type = r
            .param("source_heal_type")
            .map(|_| text(r, "source_heal_type").map(str::to_string))
            .transpose()?;
        if source_heal_type
            .as_ref()
            .is_some_and(|s| !["hero_skill", "monster_skill", "curio"].contains(&s.as_str()))
        {
            return Err("unknown healing source type".into());
        }
        let critical_heal = r
            .param("can_crit_heal")
            .map(|_| flag(r, "can_crit_heal", false))
            .transpose()?;
        if (critical_heal == Some(true) || source_heal_type.is_some())
            && !["heal", "heal_percent", "dotHpHeal", "healstress"]
                .iter()
                .any(|k| r.param(k).is_some())
        {
            return Err("critical healing metadata requires a healing operation".into());
        }
        Ok(Self {
            queued: flag(r, "queue", false)?,
            critical_roll_bonus: !flag(r, "crit_doesnt_apply_to_roll", false)?,
            critical_heal,
            source_heal_type,
        })
    }
}

