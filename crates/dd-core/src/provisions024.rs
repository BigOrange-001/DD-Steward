//! D: minimum table quantities before optional extras, with balanced essentials.
use crate::campaign017::{ProvisionAction, Provisions, QuestView};
use dd_data::inventory015::Item;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub tables: BTreeMap<String, BTreeMap<u32, BTreeMap<String, [u32; 2]>>>,
    pub essentials: Vec<String>,
    pub order: Vec<String>,
    pub notes: String,
}
impl Settings {
    pub fn validate(&self) -> bool {
        let known = [
            "food",
            "torch",
            "shovel",
            "holy_water",
            "medicinal_herbs",
            "skeleton_key",
            "bandage",
            "antivenom",
        ];
        !self.sources.is_empty()
            && self.tables.len() == 4
            && ["crypts", "weald", "warrens", "cove"]
                .iter()
                .all(|id| self.tables.contains_key(*id))
            && self.order.iter().collect::<BTreeSet<_>>().len() == known.len()
            && self.essentials.iter().all(|s| self.order.contains(s))
            && self.order.len() == known.len()
            && self.order.iter().all(|s| known.contains(&s.as_str()))
            && self.tables.values().all(|lengths| {
                [1, 2, 3].iter().all(|n| lengths.contains_key(n))
                    && lengths.values().all(|items| {
                        items.len() == known.len()
                            && items
                                .iter()
                                .all(|(id, n)| known.contains(&id.as_str()) && n[0] <= n[1])
                    })
            })
    }
    pub fn table(&self, q: &QuestView) -> Option<&BTreeMap<String, [u32; 2]>> {
        self.tables.get(&q.dungeon)?.get(&q.length)
    }
}
pub fn item(id: &str) -> Item {
    if id == "food" {
        Item::new("provision", "")
    } else {
        Item::new("supply", id)
    }
}
pub fn choose(cfg: &Settings, q: &QuestView, p: &Provisions) -> Option<ProvisionAction> {
    let table = cfg.table(q)?;
    let buy = |id: &str| ProvisionAction::Buy {
        item: item(id),
        amount: 1,
    };
    let count = |id: &str| p.inventory.count(&item(id));
    let balanced = |ids: &[String], column: usize| {
        ids.iter()
            .filter(|id| count(id) < table[*id][column] && p.actions.contains(&buy(id)))
            .min_by(|a, b| {
                let fraction =
                    |id: &String| f64::from(count(id)) / f64::from(table[id][column].max(1));
                fraction(a).total_cmp(&fraction(b))
            })
            .map(|id| buy(id))
    };
    if cfg.essentials.iter().any(|id| count(id) < table[id][0]) {
        return Some(balanced(&cfg.essentials, 0).unwrap_or(ProvisionAction::Embark));
    }
    for id in &cfg.order {
        if count(id) < table[id][0] && p.actions.contains(&buy(id)) {
            return Some(buy(id));
        }
    }
    if cfg.order.iter().any(|id| count(id) < table[id][0]) {
        return Some(ProvisionAction::Embark);
    }
    if let Some(a) = balanced(&cfg.essentials, 1) {
        return Some(a);
    }
    for id in &cfg.order {
        if count(id) < table[id][1] && p.actions.contains(&buy(id)) {
            return Some(buy(id));
        }
    }
    Some(ProvisionAction::Embark)
}

/// W-058 poverty provisioning: buy table-minimum food before balancing extras.
pub fn choose_food_first(cfg: &Settings, q: &QuestView, p: &Provisions) -> Option<ProvisionAction> {
    let minimum = cfg.table(q)?.get("food")?[0];
    let food = item("food");
    let a = ProvisionAction::Buy {
        item: food.clone(),
        amount: 1,
    };
    if p.inventory.count(&food) < minimum && p.actions.contains(&a) {
        Some(a)
    } else {
        choose(cfg, q, p)
    }
}
