//! Quest-owner operations, deliberately separate from a battle effect parser.
use crate::{
    combat::{GameData, Result, number, text},
    mechanics::{Buff, read_buff},
    trinket010::{Trinket, TrinketState},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operation {
    Quirk { positive_chance: f64 },
    Trinket { table: String },
    GuaranteedEvent,
    Growth(Vec<Buff>),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub id: String,
    pub chance: f64,
    pub operation: Operation,
}
impl GameData {
    pub fn quest_item_effects(&self, definition: &Trinket) -> Result<Vec<Effect>> {
        let Some(raw) = definition
            .quest_metadata
            .get("on_quest_complete_additional_effects")
        else {
            return Ok(vec![]);
        };
        let names = raw
            .as_array()
            .ok_or("invalid quest item effect references")?;
        let mut effects = vec![];
        for name in names {
            let name = name.as_str().ok_or("invalid quest item effect ID")?;
            let row = self.effect_source(name)?;
            let mut fields = std::collections::BTreeSet::new();
            if row.params.iter().any(|p| !fields.insert(&p.name)) {
                return Err("duplicate quest item effect field".into());
            }
            let allowed = [
                "name",
                "target",
                "curio_result_type",
                "chance",
                "on_hit",
                "on_miss",
                "queue",
                "gain_random_quirk_positive_percentage",
                "gain_random_quirk_negative",
                "gain_random_trinket",
                "guaranteed_town_event",
                "buff_ids",
            ];
            if row
                .params
                .iter()
                .any(|p| !allowed.contains(&p.name.as_str()))
            {
                return Err("unknown quest item effect field".into());
            }
            if text(row, "curio_result_type")? != "positive"
                || ["on_hit", "on_miss", "queue"]
                    .iter()
                    .any(|k| text(row, k).ok() != Some("true"))
            {
                return Err("unsupported quest item effect execution flags".into());
            }
            let target = text(row, "target")?;
            let mut operations = vec![];
            if row.param("gain_random_quirk_positive_percentage").is_some() {
                let p = number(row, "gain_random_quirk_positive_percentage")? / 100.;
                if target != "performer" || !(0. ..=1.).contains(&p) {
                    return Err("invalid quest random quirk chance/target".into());
                }
                operations.push(Operation::Quirk { positive_chance: p });
            }
            if row.param("gain_random_quirk_negative").is_some() {
                if target != "performer" || text(row, "gain_random_quirk_negative")? != "true" {
                    return Err("unsupported quest negative quirk operation".into());
                }
                operations.push(Operation::Quirk {
                    positive_chance: 0.,
                });
            }
            if row.param("gain_random_trinket").is_some() {
                if target != "this_trinket" {
                    return Err("invalid quest gained trinket target".into());
                }
                operations.push(Operation::Trinket {
                    table: text(row, "gain_random_trinket")?.into(),
                });
            }
            if row.param("guaranteed_town_event").is_some() {
                if target != "performer" || text(row, "guaranteed_town_event")? != "true" {
                    return Err("unsupported quest guaranteed event".into());
                }
                operations.push(Operation::GuaranteedEvent);
            }
            if let Some(ids) = row.param("buff_ids") {
                if target != "this_trinket" || ids.values.is_empty() {
                    return Err("invalid quest growth target/references".into());
                }
                operations.push(Operation::Growth(
                    ids.values
                        .iter()
                        .map(|v| read_buff(&self.buffs, &v.text))
                        .collect::<Result<Vec<_>>>()?,
                ));
            }
            let chance = number(row, "chance")?;
            if operations.len() != 1 || !(0. ..=1.).contains(&chance) {
                return Err("unsupported composite/empty quest item effect or probability".into());
            }
            effects.push(Effect {
                id: name.into(),
                chance,
                operation: operations.remove(0),
            });
        }
        Ok(effects)
    }
    pub fn validate_item_growth(&self, state: &TrinketState) -> Result<()> {
        state.validate()?;
        if state.growth.is_empty() {
            return Ok(());
        }
        let allowed = self
            .quest_item_effects(&state.definition)?
            .into_iter()
            .filter_map(|e| {
                if let Operation::Growth(v) = e.operation {
                    Some(v)
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        if state.growth.iter().any(|b| !allowed.contains(b)) {
            return Err("trinket growth does not match original quest operations".into());
        }
        Ok(())
    }
}
