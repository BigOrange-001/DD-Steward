//! Explicitly approved candidate ownership, not evidence of original triggers.
//! Values are parsed from the effective original rules, never supplied here.
use crate::combat::Rules;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LegacyHostInputs {
    pub interpretation_id: String,
    pub calibration_status: String,
    pub source: String,
    pub deactivation_chance_small: f64,
    pub deactivation_chance_med: f64,
    pub min_hero_count_for_escape: u32,
}
impl Rules {
    pub fn legacy_host_inputs(&self) -> Option<LegacyHostInputs> {
        if self
            .content
            .syntax_choices
            .get("legacy_global_boundary")
            .map(String::as_str)
            != Some("outside_combat_v1")
        {
            return None;
        }
        Some(LegacyHostInputs {
            interpretation_id:"legacy-global-host-012-v1".into(),
            calibration_status:"candidate ownership; awaiting original calibration; no original trigger claim".into(),
            source:"shared/rules.json in effective content overlay; deactivation -> outside-combat recovery; min hero count -> expedition exit".into(),
            deactivation_chance_small:self.deactivation_chance_small,
            deactivation_chance_med:self.deactivation_chance_med,
            min_hero_count_for_escape:self.min_hero_count_for_escape,
        })
    }
}
