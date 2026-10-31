//! Source rarity ordering is partial: siblings inserted at one anchor need no
//! guessed order to compare the original quest minimum-rarity thresholds.
use crate::{combat::Result, library, overlay::Overlay};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rarities {
    pub base: Vec<String>,
    pub values: BTreeMap<String, Value>,
    #[serde(default)]
    pub visual_only: BTreeSet<String>,
    #[serde(default)]
    pub unreferenced: BTreeSet<String>,
}
impl Rarities {
    pub fn load(overlay: &Overlay) -> Result<Option<Self>> {
        let mut values = BTreeMap::new();
        let mut base = None;
        for path in overlay
            .paths
            .keys()
            .filter(|p| p.starts_with("trinkets/") && p.ends_with(".rarities.trinkets.json"))
        {
            for bytes in overlay.read_campaign_versions(path)? {
                let raw: Value =
                    serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))?;
                crate::campaign017::fields(&raw, &["rarities"], "rarity document")?;
                let rows = crate::campaign017::array(&raw, "rarities")?;
                if path == "trinkets/base.rarities.trinkets.json" {
                    let order = rows
                        .iter()
                        .map(|v| {
                            v["id"]
                                .as_str()
                                .map(str::to_string)
                                .ok_or("missing base rarity ID".into())
                        })
                        .collect::<Result<Vec<_>>>()?;
                    if base.as_ref().is_some_and(|old| old != &order) {
                        return Err("ambiguous base rarity order".into());
                    }
                    base = Some(order);
                }
                for row in rows {
                    let id = row["id"].as_str().ok_or("missing rarity ID")?;
                    library::insert_json(&mut values, id.into(), row.clone());
                }
            }
        }
        if values.is_empty() {
            return Ok(None);
        }
        let base = base.ok_or("missing base rarity order")?;
        let mut gameplay = BTreeSet::new();
        let mut visual = BTreeSet::new();
        for rows in
            library::json_library(overlay, "trinkets/", ".entries.trinkets.json", "entries")?
                .values()
        {
            let rows = if let Some(rows) = rows.as_array() {
                rows.iter().collect::<Vec<_>>()
            } else {
                vec![rows]
            };
            for row in rows {
                if let Some(id) = row["rarity"].as_str() {
                    gameplay.insert(id.to_string());
                }
                if let Some(id) = row["visual_rarity"].as_str() {
                    visual.insert(id.to_string());
                }
            }
        }
        let visual_only = visual
            .difference(&gameplay)
            .filter(|id| values.contains_key(*id))
            .cloned()
            .collect();
        let unreferenced = values
            .keys()
            .filter(|id| !gameplay.contains(*id) && !visual.contains(*id))
            .cloned()
            .collect();
        let result = Self {
            unreferenced,
            base,
            values,
            visual_only,
        };
        result.validate()?;
        Ok(Some(result))
    }
    pub fn validate(&self) -> Result<()> {
        if self.base.is_empty()
            || self.base.iter().collect::<BTreeSet<_>>().len() != self.base.len()
        {
            return Err("invalid base rarity order".into());
        }
        for (id, values) in &self.values {
            let rows = if let Some(rows) = values.as_array() {
                rows.iter().collect::<Vec<_>>()
            } else {
                vec![values]
            };
            for raw in rows {
                crate::campaign017::fields(
                    raw,
                    &["id", "award_category", "insert_before"],
                    "rarity",
                )?;
                if raw["id"] != *id
                    || ![
                        "dd",
                        "trophy",
                        "battle",
                        "universal",
                        "quest",
                        "kickstarter",
                    ]
                    .contains(
                        &raw["award_category"]
                            .as_str()
                            .ok_or("missing rarity award category")?,
                    )
                {
                    return Err("unsupported rarity identity/category".into());
                }
            }
        }
        if self.base.iter().any(|id| !self.values.contains_key(id)) {
            return Err("missing base rarity entry".into());
        }
        Ok(())
    }
    pub fn audit(&self) -> Value {
        let errors = self
            .values
            .keys()
            .filter(|id| !self.visual_only.contains(*id) && !self.unreferenced.contains(*id))
            .filter_map(|id| self.anchor(id).err().map(|e| (id.clone(), e)))
            .collect::<BTreeMap<_, _>>();
        let unused_errors = self
            .unreferenced
            .iter()
            .filter_map(|id| self.anchor(id).err().map(|e| (id.clone(), e)))
            .collect::<BTreeMap<_, _>>();
        serde_json::json!({"base":self.base,"values":self.values,"visual_only":self.visual_only,"unreferenced":self.unreferenced,"ordering_errors":errors,"unreferenced_ordering_errors":unused_errors})
    }
    fn anchor(&self, id: &str) -> Result<usize> {
        if self.visual_only.contains(id) {
            return Err("visual rarity used as gameplay classification".into());
        }
        let mut id = id;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(id) {
                return Err("cyclic rarity insertion".into());
            }
            let raw = library::unique_json(&self.values, id, "rarity ordering")?;
            if let Some(index) = self.base.iter().position(|v| v == id) {
                if raw.get("insert_before").is_some() {
                    return Err("base rarity insertion conflict".into());
                }
                return Ok(index);
            }
            id = raw["insert_before"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or("ambiguous addon rarity order")?;
        }
    }
    pub fn at_least(&self, id: &str, threshold: &str) -> Result<bool> {
        let threshold = self
            .base
            .iter()
            .position(|v| v == threshold)
            .ok_or("ambiguous non-base rarity threshold")?;
        Ok(self.anchor(id)? <= threshold)
    }
    pub fn protected(
        &self,
        id: &str,
        categories: &BTreeSet<String>,
        ids: &BTreeSet<String>,
    ) -> Result<bool> {
        if self.unreferenced.contains(id) {
            return Err("unreferenced rarity cannot classify physical asset".into());
        }
        if self.visual_only.contains(id) {
            return Err("visual rarity used as gameplay classification".into());
        }
        let raw = library::unique_json(&self.values, id, "retention rarity")?;
        Ok(ids.contains(id)
            || categories.contains(
                raw["award_category"]
                    .as_str()
                    .ok_or("missing rarity category")?,
            ))
    }
}

pub fn quest_count(source: &Value) -> Result<usize> {
    match source.get("trinket_retention_count") {
        None => Ok(0),
        Some(v) => v
            .as_u64()
            .filter(|n| *n <= 10000)
            .map(|n| n as usize)
            .ok_or("invalid original retention count".into()),
    }
}
