//! Slot inventory with original item definitions; acquisition is owned by the expedition.
use crate::{combat::Result, curios013::Curios, library};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub kind: String,
    pub id: String,
}
impl Item {
    pub fn new(kind: &str, id: &str) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    pub item: Item,
    pub amount: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    pub limit: u32,
    pub purchase: u32,
    pub sell: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InventoryRules {
    pub slots: usize,
    #[serde(default)]
    pub extra_limits: BTreeMap<String, u32>,
    pub items: BTreeMap<String, Value>,
    pub trinkets: BTreeMap<String, Value>,
}
impl InventoryRules {
    pub fn load(o: &crate::overlay::Overlay, curios: &Curios) -> Result<Self> {
        let mut configs = BTreeMap::new();
        let mut items = curios.items.clone();
        for p in o
            .paths
            .keys()
            .filter(|p| p.starts_with("inventory/") && p.ends_with(".inventory.items.darkest"))
        {
            for b in o.read_opening_versions(p)? {
                let d =
                    crate::darkest::parse_str(std::str::from_utf8(&b).map_err(|e| e.to_string())?);
                if !d.warnings.is_empty() {
                    return Err("invalid item inventory syntax".into());
                }
                for r in d.records {
                    if r.key != "inventory_item" {
                        return Err("unknown item inventory record".into());
                    }
                    let id = crate::combat::text(&r, "id")?;
                    let kind = crate::combat::text(&r, "type")?;
                    let raw = r
                        .params
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                p.values.iter().map(|v| v.text.clone()).collect::<Vec<_>>(),
                            )
                        })
                        .collect::<BTreeMap<_, _>>();
                    library::insert_json(
                        &mut items,
                        format!("{kind}:{id}"),
                        serde_json::json!(raw),
                    );
                }
            }
        }
        for p in o
            .paths
            .keys()
            .filter(|p| p.starts_with("inventory/") && p.ends_with("system_configs.darkest"))
        {
            for b in o.read_opening_versions(p)? {
                let d =
                    crate::darkest::parse_str(std::str::from_utf8(&b).map_err(|e| e.to_string())?);
                if !d.warnings.is_empty() {
                    return Err("invalid inventory config syntax".into());
                }
                for r in d.records {
                    if r.key != "inventory_system_config" {
                        return Err("unknown inventory system record".into());
                    }
                    let id = crate::combat::text(&r, "type")?;
                    if id == "raid" {
                        if r.params.iter().any(|p| {
                            !["type", "max_slots", "use_stack_limits"].contains(&p.name.as_str())
                        }) || crate::combat::text(&r, "use_stack_limits")? != "true"
                        {
                            return Err("unsupported raid inventory config".into());
                        }
                        let n = crate::combat::integer(&r, "max_slots")?;
                        if !(1..=128).contains(&n) {
                            return Err("invalid raid slot capacity".into());
                        }
                        library::insert_json(&mut configs, id.into(), serde_json::json!(n));
                    }
                }
            }
        }
        let slots = library::unique_json(&configs, "raid", "inventory config")?
            .as_u64()
            .ok_or("invalid slots")? as usize;
        Ok(Self {
            slots,
            extra_limits: BTreeMap::new(),
            items,
            trinkets: curios.trinkets.clone(),
        })
    }
    pub fn definition(&self, item: &Item) -> Result<ItemDef> {
        if item.kind == "trinket" {
            library::unique_json(&self.trinkets, &item.id, "inventory trinket")?;
            return Ok(ItemDef {
                limit: 1,
                purchase: 0,
                sell: 0,
            });
        }
        if item.kind == "journal_page" {
            item.id.parse::<u32>().map_err(|_| "invalid journal page")?;
            return Ok(ItemDef {
                limit: 1,
                purchase: 0,
                sell: 0,
            });
        }
        let v = library::unique_json(&self.items, &item.key(), "inventory item")?;
        let get = |k: &str, default: Option<u32>| -> Result<u32> {
            match v.get(k) {
                None => default.ok_or_else(|| format!("missing item {k}")),
                Some(v) => v
                    .as_array()
                    .filter(|v| v.len() == 1)
                    .and_then(|v| v[0].as_str())
                    .and_then(|v| v.parse::<u32>().ok())
                    .filter(|v| *v <= 1000000)
                    .ok_or_else(|| format!("invalid item {k}")),
            }
        };
        let limit = get("base_stack_limit", None)?;
        if limit == 0 {
            return Err("zero item stack limit".into());
        }
        Ok(ItemDef {
            limit: limit
                .checked_add(self.extra_limits.get(&item.key()).copied().unwrap_or(0))
                .ok_or("stack limit overflow")?,
            purchase: get("purchase_gold_value", Some(0))?,
            sell: get("sell_gold_value", Some(0))?,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bag {
    pub slots: Vec<Option<Stack>>,
}
impl Bag {
    pub fn new(r: &InventoryRules) -> Self {
        Self {
            slots: vec![None; r.slots],
        }
    }
    pub fn validate(&self, r: &InventoryRules) -> Result<()> {
        if self.slots.len() != r.slots {
            return Err("inventory capacity mismatch".into());
        }
        for s in self.slots.iter().flatten() {
            if s.amount == 0 || s.amount > r.definition(&s.item)?.limit {
                return Err("invalid inventory stack".into());
            }
        }
        Ok(())
    }
    pub fn count(&self, item: &Item) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| &s.item == item)
            .map(|s| s.amount)
            .sum()
    }
    /// Returns the amount actually accepted. The caller retains any remainder in a visible loot tray.
    pub fn insert(&mut self, r: &InventoryRules, stack: &Stack) -> Result<u32> {
        self.validate(r)?;
        if stack.amount == 0 || stack.amount > 1000000 {
            return Err("invalid incoming amount".into());
        }
        let limit = r.definition(&stack.item)?.limit;
        let mut left = stack.amount;
        for s in self
            .slots
            .iter_mut()
            .flatten()
            .filter(|s| s.item == stack.item)
        {
            let n = left.min(limit - s.amount);
            s.amount += n;
            left -= n;
        }
        for slot in self.slots.iter_mut().filter(|s| s.is_none()) {
            if left == 0 {
                break;
            }
            let n = left.min(limit);
            *slot = Some(Stack {
                item: stack.item.clone(),
                amount: n,
            });
            left -= n;
        }
        Ok(stack.amount - left)
    }
    pub fn consume(&mut self, item: &Item, amount: u32) -> Result<()> {
        if amount == 0 || self.count(item) < amount {
            return Err("not enough inventory item".into());
        }
        let mut left = amount;
        for slot in &mut self.slots {
            if let Some(s) = slot
                && &s.item == item
            {
                let n = s.amount.min(left);
                s.amount -= n;
                left -= n;
                if s.amount == 0 {
                    *slot = None;
                }
            }
        }
        Ok(())
    }
    pub fn discard(&mut self, slot: usize, amount: u32) -> Result<Stack> {
        let s = self
            .slots
            .get_mut(slot)
            .and_then(Option::as_mut)
            .ok_or("empty inventory slot")?;
        if amount == 0 || amount > s.amount {
            return Err("invalid discard amount".into());
        }
        let result = Stack {
            item: s.item.clone(),
            amount,
        };
        s.amount -= amount;
        if s.amount == 0 {
            self.slots[slot] = None;
        }
        Ok(result)
    }
}
