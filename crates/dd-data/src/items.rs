//! Battle supplies only; acquisition, loot and estate inventory are separate.
use crate::{
    combat::{Effect, EffectTarget, GameData, Result, number, text},
    library::same_record,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    #[serde(default)]
    pub semantics: SupplySemantics,
    #[serde(default)]
    pub host_candidate: Option<HostItemCandidate>,
    pub id: String,
    pub stack_limit: u32,
    pub consume_priority: u32,
    pub effects: Vec<Effect>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SupplySemantics {
    pub sequential_effects: Vec<Effect>,
    /// Parsed purchase/presentation fields are retained for the future host;
    /// they do not implement purchasing or rendering in the battle engine.
    pub source_metadata: std::collections::BTreeMap<String, Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SupplyStack {
    pub definition: Supply,
    pub remaining: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HostItemCandidate {
    pub light_amount: i32,
    pub source: String,
}
impl Supply {
    pub fn validate(&self) -> Result<()> {
        for (key, values) in &self.semantics.source_metadata {
            let valid = values.len() == 1
                && match key.as_str() {
                    "use_id_take_sfx"
                    | "wait_for_item_use_to_finish"
                    | "can_modify_cost"
                    | "estate_can_be_provision" => matches!(values[0].as_str(), "true" | "false"),
                    "purchase_gold_value" | "purchase_shard_value" | "sell_gold_value" => values[0]
                        .parse::<f64>()
                        .is_ok_and(|v| v.is_finite() && v >= 0. && v.fract() == 0.),
                    "fx_bone_override" | "sfx_override" | "tutorial_id" => values[0].len() <= 2000,
                    _ => false,
                };
            if !valid {
                return Err(format!("invalid supply source metadata {key}"));
            }
        }
        if let Some(c) = &self.host_candidate
            && (self.id != "torch"
                || !(1..=100).contains(&c.light_amount)
                || c.source.trim().is_empty()
                || c.source.len() > 2000
                || self.effects.len() != 1
                || !matches!(&self.effects[0].kind,crate::combat::EffectKind::Lifecycle(crate::lifecycle::LifecycleEffect{op:crate::lifecycle::LifeOp::Light{amount},..})if *amount==c.light_amount))
        {
            return Err("invalid host item candidate".into());
        }
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            || !(1..=10000).contains(&self.stack_limit)
            || self.consume_priority > 10000
            || (self.consume_priority > 0
                && self.effects.is_empty()
                && self.semantics.sequential_effects.is_empty())
            || self
                .effects
                .iter()
                .chain(&self.semantics.sequential_effects)
                .any(|e| {
                    ![EffectTarget::Performer, EffectTarget::Target].contains(&e.target)
                        || !e.conditions.is_empty()
                })
        {
            return Err(format!("{}: unsupported battle supply definition", self.id));
        }
        Ok(())
    }
}
impl GameData {
    /// Resolve only an explicitly carried item. Unknown effects never get
    /// omitted from selection: constructing that inventory fails instead.
    pub fn supply(&self, id: &str) -> Result<Supply> {
        self.supply_inner(id, None, None)
    }
    /// Only an explicit host request may supply an absent, calibrated-later rule.
    pub fn supply_with_light_candidate(
        &self,
        id: &str,
        candidate: HostItemCandidate,
    ) -> Result<Supply> {
        if id != "torch" {
            return Err("candidate light supply must be torch".into());
        }
        self.supply_inner(id, Some(candidate), None)
    }
    pub(crate) fn supply_with_override(&self, id: &str, effects: Vec<Effect>) -> Result<Supply> {
        self.supply_inner(id, None, Some(effects))
    }
    fn supply_inner(
        &self,
        id: &str,
        candidate: Option<HostItemCandidate>,
        override_effects: Option<Vec<Effect>>,
    ) -> Result<Supply> {
        let mut records = vec![];
        for path in self
            .overlay
            .combat_files("inventory/", ".inventory.items.darkest")
        {
            for bytes in self.overlay.read_versions(&path)? {
                if bytes.contains(&0) {
                    return Err(format!("{path}: binary supply file"));
                }
                let source = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
                let d = crate::darkest::parse_str(source);
                if !d.warnings.is_empty() {
                    return Err(format!("{path}: {:?}", d.warnings));
                }
                for r in d.records {
                    if r.key != "inventory_item" {
                        return Err(format!("{path}: unsupported inventory record {}", r.key));
                    }
                    if ["supply", "estate"].contains(&text(&r, "type")?)
                        && text(&r, "id")? == id
                        && !records.iter().any(|old| same_record(old, &r))
                    {
                        records.push(r);
                    }
                }
            }
        }
        if records.len() != 1 {
            return Err(format!(
                "{id}: expected one battle supply definition, found {}",
                records.len()
            ));
        }
        let r = &records[0];
        for p in &r.params {
            if ![
                "type",
                "id",
                "base_stack_limit",
                "purchase_gold_value",
                "sell_gold_value",
                "act_out_consume_priority",
                "replace_buffs",
                "estate_can_be_provision",
                "tutorial_id",
                "sfx_override",
                "use_id_take_sfx",
                "purchase_shard_value",
                "fx_bone_override",
                "sequential_use_effects",
                "wait_for_item_use_to_finish",
                "can_modify_cost",
            ]
            .contains(&p.name.as_str())
            {
                return Err(format!("{id}: unsupported supply parameter .{}", p.name));
            }
        }
        if text(r, "type")? == "estate" && text(r, "estate_can_be_provision")? != "true" {
            return Err("estate item is not provisionable".into());
        }
        if r.param("replace_buffs").is_some() && text(r, "replace_buffs")? != "true" {
            return Err(format!("{id}: unsupported replace_buffs value"));
        }
        let integer = |field: &str| -> Result<u32> {
            let n = number(r, field)?;
            if n.fract() != 0. || !(0. ..=10000.).contains(&n) {
                return Err(format!("{id}: invalid supply {field}"));
            }
            Ok(n as u32)
        };
        let priority = if r.param("act_out_consume_priority").is_some() {
            integer("act_out_consume_priority")?
        } else {
            0
        };
        let sequential_effects = r
            .param("sequential_use_effects")
            .map(|p| {
                if p.values.is_empty() {
                    return Err("empty sequential supply effects".into());
                }
                p.values
                    .iter()
                    .map(|v| self.effect(&v.text))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let mut source_metadata = std::collections::BTreeMap::new();
        for field in [
            "use_id_take_sfx",
            "wait_for_item_use_to_finish",
            "can_modify_cost",
            "estate_can_be_provision",
        ] {
            if let Some(p) = r.param(field) {
                crate::combat::flag(r, field, false)?;
                source_metadata.insert(
                    field.into(),
                    p.values.iter().map(|v| v.text.clone()).collect(),
                );
            }
        }
        for field in [
            "purchase_gold_value",
            "purchase_shard_value",
            "sell_gold_value",
        ] {
            if let Some(p) = r.param(field) {
                let n = number(r, field)?;
                if n < 0. || n.fract() != 0. {
                    return Err("invalid supply cost metadata".into());
                }
                source_metadata.insert(
                    field.into(),
                    p.values.iter().map(|v| v.text.clone()).collect(),
                );
            }
        }
        for field in ["fx_bone_override", "sfx_override", "tutorial_id"] {
            if let Some(p) = r.param(field) {
                text(r, field)?;
                source_metadata.insert(
                    field.into(),
                    p.values.iter().map(|v| v.text.clone()).collect(),
                );
            }
        }
        let value = Supply {
            semantics: SupplySemantics {
                sequential_effects: sequential_effects.clone(),
                source_metadata,
            },
            host_candidate: candidate.clone(),
            id: id.into(),
            stack_limit: integer("base_stack_limit")?,
            consume_priority: priority,
            effects: if let Some(effects) = override_effects {
                effects
            } else if !sequential_effects.is_empty() {
                vec![]
            } else if priority > 0 || text(r, "type")? == "estate" {
                if let Some(c) = candidate {
                    vec![crate::combat::Effect {
                        execution: Default::default(),
                        name: "host_candidate_torch_light".into(),
                        kind: crate::combat::EffectKind::Lifecycle(
                            crate::lifecycle::LifecycleEffect {
                                op: crate::lifecycle::LifeOp::Light {
                                    amount: c.light_amount,
                                },
                                queued: true,
                                requires_kill: false,
                            },
                        ),
                        chance: 1.,
                        duration: 1,
                        target: EffectTarget::Performer,
                        on_hit: true,
                        on_miss: true,
                        apply_once: true,
                        resist_debuff: false,
                        conditions: vec![],
                        can_apply_on_death: false,
                    }]
                } else {
                    vec![self.effect(id)?]
                }
            } else {
                vec![]
            },
        };
        value.validate()?;
        Ok(value)
    }
}

