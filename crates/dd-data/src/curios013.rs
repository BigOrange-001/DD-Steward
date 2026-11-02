//! Fixed-level curio libraries. Unreferenced definitions remain raw; a referenced
//! conflicting or unsupported definition fails rather than taking file order.
use crate::{
    combat::{GameData, Result, number, text},
    library,
    overlay::Overlay,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Draw {
    pub table: String,
    pub count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Outcome {
    Nothing,
    Loot {
        draws: Vec<Draw>,
        nothing_chance: f64,
    },
    Effect(Vec<(String, f64)>),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Curio {
    pub id: String,
    pub full: bool,
    pub outcomes: Vec<(Outcome, f64)>,
    pub items: BTreeMap<String, Outcome>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Curios {
    pub darkness: BTreeMap<String, Value>,
    pub items: BTreeMap<String, Value>,
    pub loot_choices: BTreeMap<String, String>,
    pub props: BTreeMap<String, Value>,
    pub types: BTreeMap<String, Value>,
    pub loot: BTreeMap<String, Value>,
    pub trinkets: BTreeMap<String, Value>,
}
fn rows(b: &[u8]) -> Result<Vec<Vec<String>>> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(b)
        .records()
        .map(|r| {
            r.map(|r| r.iter().map(str::to_owned).collect())
                .map_err(|e| e.to_string())
        })
        .collect()
}
pub(crate) fn col(row: &[String], n: usize) -> &str {
    row.get(n).map(String::as_str).unwrap_or("")
}
pub fn type_blocks(bytes: &[u8]) -> Result<BTreeMap<String, Value>> {
    let rows = rows(bytes)?;
    let mut map = BTreeMap::new();
    let starts = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| col(r, 1).parse::<u32>().is_ok())
        .map(|(i, _)| i)
        .chain([rows.len()])
        .collect::<Vec<_>>();
    for window in starts.windows(2) {
        let block = &rows[window[0]..window[1]];
        if col(block.get(1).ok_or("short curio group")?, 2) != "ID STRING" {
            return Err("curio group lacks ID header".into());
        }
        let id = col(block.get(2).ok_or("curio group lacks id")?, 2);
        if id.is_empty() {
            return Err("empty curio id".into());
        }
        library::insert_json(&mut map, id.into(), json!({"rows":block}));
    }
    Ok(map)
}
pub fn known_keys(v: &Value, keys: &[&str]) -> Result<()> {
    for key in v.as_object().ok_or("expected loot object")?.keys() {
        if !keys.contains(&key.as_str()) {
            return Err(format!("unknown loot field {key}"));
        }
    }
    Ok(())
}
pub fn loot_key(id: &str, difficulty: i64, dungeon: &str) -> String {
    format!("{id}:{difficulty}:{dungeon}")
}
impl Curios {
    pub fn load(o: &Overlay) -> Result<Self> {
        Self::load_with_court_loot(o, false)
    }
    pub fn load_with_court_loot(o: &Overlay, court: bool) -> Result<Self> {
        if court && !o.config.packages.iter().any(|p| p == "crimson_court") {
            return Err("court loot selection requires crimson_court".into());
        }
        let mut all = Self {
            darkness: BTreeMap::new(),
            items: BTreeMap::new(),
            loot_choices: BTreeMap::new(),
            props: BTreeMap::new(),
            types: BTreeMap::new(),
            loot: BTreeMap::new(),
            trinkets: BTreeMap::new(),
        };
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("curios/") && p.ends_with(".csv"))
        {
            if !path.ends_with("curio_props.csv") && !path.ends_with("curio_type_library.csv") {
                return Err(format!("unknown curio library {path}"));
            }
            for b in o.read_initial_versions(path)? {
                if path.ends_with("curio_props.csv") {
                    for row in rows(&b)?.into_iter().skip(1) {
                        if col(&row, 0).is_empty() {
                            continue;
                        }
                        if row.len() < 3 {
                            return Err("short curio prop row".into());
                        }
                        library::insert_json(
                            &mut all.props,
                            col(&row, 0).into(),
                            json!({"row":row}),
                        );
                    }
                } else {
                    for (id, raw) in type_blocks(&b)? {
                        library::insert_json(&mut all.types, id, raw);
                    }
                }
            }
        }
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("loot/") && p.ends_with(".json"))
        {
            for layer in o.sources(path) {
                let b = o.read_initial_source(path, layer)?;
                let v: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
                if let Some(tables) = v.get("loot_tables") {
                    for table in tables.as_array().ok_or("invalid loot table array")? {
                        let id = table["id"].as_str().ok_or("loot id missing")?;
                        let d = table["difficulty"]
                            .as_i64()
                            .ok_or("loot difficulty missing")?;
                        let region = table["dungeon"].as_str().ok_or("loot dungeon missing")?;
                        if court && ["A", "S"].contains(&id) && d == 0 && region.is_empty() {
                            if path != "loot/common_overrides.loot.json" {
                                return Err(format!("unexpected selected loot source {path}"));
                            }
                            if o.layers[layer].id != "package:crimson_court:0" {
                                continue;
                            }
                            all.loot_choices.insert(
                                loot_key(id, d, region),
                                format!("{}/{}", o.layers[layer].id, path),
                            );
                        }
                        library::insert_json(&mut all.loot, loot_key(id, d, region), table.clone());
                    }
                } else if path == "loot/darkness_overrides.loot.json" {
                    known_keys(&v, &["darkness_bonuses"])?;
                    library::insert_json(&mut all.darkness, "definition".into(), v);
                } else {
                    return Err(format!("unknown loot library schema {path}"));
                }
            }
        }
        for path in o
            .paths
            .keys()
            .filter(|p| p.starts_with("trinkets/") && p.ends_with(".entries.trinkets.json"))
        {
            for b in o.read_versions(path)? {
                let v: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
                for t in v["entries"].as_array().ok_or("missing trinket entries")? {
                    let id = t["id"].as_str().ok_or("missing trinket id")?;
                    library::insert_json(&mut all.trinkets, id.into(), t.clone());
                }
            }
        }

        for path in o.combat_files("inventory/", ".inventory.items.darkest") {
            for bytes in o.read_versions(&path)? {
                let source = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
                let d = crate::darkest::parse_str(source);
                if !d.warnings.is_empty() {
                    return Err("invalid initial item inventory".into());
                }
                for record in &d.records {
                    if record.key != "inventory_item" {
                        return Err("unknown inventory record".into());
                    }
                    let id = text(record, "id")?;
                    let kind = text(record, "type")?;
                    let raw = record
                        .params
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                p.values.iter().map(|v| v.text.clone()).collect::<Vec<_>>(),
                            )
                        })
                        .collect::<BTreeMap<_, _>>();
                    library::insert_json(&mut all.items, format!("{kind}:{id}"), json!(raw));
                }
            }
        }
        Ok(all)
    }
    pub fn prop(&self, hash: u32) -> Result<(String, Curio)> {
        let ids = self
            .props
            .keys()
            .filter(|s| crate::dson013::hash(s) == hash)
            .collect::<Vec<_>>();
        if ids.len() != 1 {
            return Err(format!("missing/ambiguous curio prop hash {hash}"));
        }
        let raw = library::unique_json(&self.props, ids[0], "curio prop")?;
        let tid = raw["row"][2]
            .as_str()
            .ok_or("invalid curio type reference")?;
        Ok((ids[0].clone(), self.curio(tid)?))
    }
    pub fn curio(&self, id: &str) -> Result<Curio> {
        let rows = self.source_rows(id)?;
        let (full, outcomes, items) = decode_curio_rows(id, &rows, parse_outcome)?;
        Ok(Curio {
            id: id.into(),
            full,
            outcomes,
            items,
        })
    }
    pub(crate) fn source_rows(&self, id: &str) -> Result<Vec<Vec<String>>> {
        let raw = library::unique_json(&self.types, id, "curio")?;
        serde_json::from_value(raw["rows"].clone()).map_err(|_| format!("ambiguous curio {id}"))
    }

    pub fn require_zero_light_bonus(&self, light: i64) -> Result<()> {
        if !(0..=100).contains(&light) {
            return Err("invalid initial torchlight".into());
        }
        let root = library::unique_json(&self.darkness, "definition", "darkness bonus")?;
        let list = root["darkness_bonuses"]
            .as_array()
            .ok_or("missing darkness bonuses")?;
        let chest = list
            .iter()
            .filter(|v| v["key"].as_str() == Some("chest"))
            .collect::<Vec<_>>();
        if chest.len() != 1 {
            return Err("ambiguous chest darkness rule".into());
        }
        known_keys(chest[0], &["key", "data"])?;
        let rows = chest[0]["data"]
            .as_array()
            .ok_or("missing darkness tiers")?;
        let mut best = None;
        let mut seen = std::collections::BTreeSet::new();
        for row in rows {
            known_keys(row, &["darkness", "chance", "codes"])?;
            let at = row["darkness"]
                .as_i64()
                .filter(|n| (0..=100).contains(n))
                .ok_or("invalid darkness threshold")?;
            if !seen.insert(at) {
                return Err("duplicate darkness tier".into());
            }
            let chance = row["chance"]
                .as_f64()
                .filter(|v| (0. ..=1.).contains(v))
                .ok_or("invalid darkness chance")?;
            row["codes"]
                .as_array()
                .filter(|v| v.iter().all(|v| v.as_str().is_some()))
                .ok_or("invalid darkness codes")?;
            if at <= light && best.is_none_or(|(b, _)| at > b) {
                best = Some((at, chance));
            }
        }
        if best.is_none_or(|(_, p)| p != 0.) {
            return Err("initial map/curio phase requires zero source darkness bonus; lower-light loot needs expedition host".into());
        }
        Ok(())
    }
    pub fn inventory_item(&self, id: &str) -> Result<&Value> {
        let supply = format!("supply:{id}");
        let estate = format!("estate:{id}");
        if self.items.contains_key(&supply) && self.items.contains_key(&estate) {
            return Err("ambiguous initial item category".into());
        }
        let key = if self.items.contains_key(&supply) {
            supply
        } else {
            estate
        };
        library::unique_json(&self.items, &key, "initial inventory item")
    }
    pub fn validate_draw(
        &self,
        id: &str,
        difficulty: i64,
        dungeon: &str,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        self.validate_draw_context(id, difficulty, dungeon, stack, false)
    }
    pub fn validate_campaign_draw(
        &self,
        id: &str,
        difficulty: i64,
        dungeon: &str,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        self.validate_draw_context(id, difficulty, dungeon, stack, true)
    }
    fn validate_draw_context(
        &self,
        id: &str,
        difficulty: i64,
        dungeon: &str,
        stack: &mut Vec<String>,
        campaign: bool,
    ) -> Result<()> {
        if stack.len() > 16 || stack.iter().any(|s| s == id) {
            return Err("cyclic initial loot graph".into());
        }
        stack.push(id.into());
        let tables = if campaign {
            self.campaign_tables(id, difficulty, dungeon)?
        } else {
            vec![self.table(id, difficulty, dungeon)?]
        };
        for table in tables {
            let entries = table["entries"].as_array().ok_or("missing loot entries")?;
            let mut total = 0.;
            for e in entries {
                known_keys(e, &["type", "chances", "data"])?;
                let w = e["chances"]
                    .as_f64()
                    .filter(|n| n.is_finite() && *n >= 0.)
                    .ok_or("invalid initial loot weight")?;
                total += w;
                if w == 0. {
                    continue;
                }
                let data = &e["data"];
                match e["type"].as_str().ok_or("missing loot kind")? {
                    "nothing" => known_keys(data, &[])?,
                    "table" => {
                        known_keys(data, &["table"])?;
                        self.validate_draw_context(
                            data["table"].as_str().ok_or("missing nested table")?,
                            difficulty,
                            dungeon,
                            stack,
                            campaign,
                        )?;
                    }
                    "item" => {
                        known_keys(data, &["type", "id", "amount"])?;
                        let kind = data["type"].as_str().ok_or("missing reward kind")?;
                        if !["supply", "provision", "gold", "gem", "heirloom", "estate"]
                            .contains(&kind)
                            && !(campaign
                                && ["trinket", "estate_currency", "quest_item", "shard"]
                                    .contains(&kind))
                        {
                            return Err(format!("unsupported initial reward kind {kind}"));
                        }
                        data["amount"]
                            .as_u64()
                            .filter(|n| (1..=1000000).contains(n))
                            .ok_or("invalid reward amount")?;
                        let item = data["id"].as_str().ok_or("missing reward id")?;
                        if campaign {
                            if kind == "trinket" {
                                library::unique_json(
                                    &self.trinkets,
                                    item,
                                    "campaign loot trinket",
                                )?;
                            } else {
                                library::unique_json(
                                    &self.items,
                                    &format!("{kind}:{item}"),
                                    "campaign loot item",
                                )?;
                            }
                        }
                        if kind == "supply" || kind == "estate" {
                            self.inventory_item(item)?;
                        }
                    }
                    "trinket" => {
                        known_keys(data, &["rarity"])?;
                        data["rarity"].as_str().ok_or("missing trinket rarity")?;
                    }
                    "journal_page" => {
                        fixed_journal_page(data, campaign)?;
                    }
                    kind => return Err(format!("unsupported initial loot kind {kind}")),
                }
            }
            if !total.is_finite() || total <= 0. {
                return Err("no positive initial loot weight".into());
            }
        }
        stack.pop();
        Ok(())
    }
    pub fn campaign_tables(&self, id: &str, difficulty: i64, dungeon: &str) -> Result<Vec<&Value>> {
        let mut candidates = vec![];
        for d in [0, difficulty] {
            for region in ["", dungeon] {
                let key = loot_key(id, d, region);
                if let Some(value) = self.loot.get(&key)
                    && !candidates
                        .iter()
                        .any(|(a, b, _)| *a == (d != 0) && *b == (!region.is_empty()))
                {
                    candidates.push((d != 0, !region.is_empty(), value));
                }
            }
        }
        let best = candidates
            .iter()
            .filter(|(d, r, _)| {
                !candidates
                    .iter()
                    .any(|(dd, rr, _)| dd >= d && rr >= r && (dd > d || rr > r))
            })
            .collect::<Vec<_>>();
        if best.len() != 1 {
            return Err(format!("missing/ambiguous contextual loot table {id}"));
        }
        let value = best[0].2;
        let variants = if let Some(rows) = value.as_array() {
            rows.iter().collect::<Vec<_>>()
        } else {
            vec![value]
        };
        let mut conditions = std::collections::BTreeSet::new();
        for row in &variants {
            known_keys(
                row,
                &[
                    "id",
                    "difficulty",
                    "dungeon",
                    "entries",
                    "infestation_sequence_element",
                ],
            )?;
            let condition = row
                .get("infestation_sequence_element")
                .map(|v| {
                    v.as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or("invalid loot infestation condition")
                })
                .transpose()?;
            if variants.len() > 1 && (condition.is_none() || !conditions.insert(condition.unwrap()))
            {
                return Err(format!("conflicting contextual loot table {id}"));
            }
        }
        Ok(variants)
    }
    pub fn campaign_table(
        &self,
        id: &str,
        difficulty: i64,
        dungeon: &str,
        infestation: &str,
    ) -> Result<&Value> {
        let rows = self
            .campaign_tables(id, difficulty, dungeon)?
            .into_iter()
            .filter(|v| {
                v["infestation_sequence_element"]
                    .as_str()
                    .is_none_or(|stage| stage == infestation)
            })
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            return Err(format!("missing/ambiguous active campaign loot table {id}"));
        }
        Ok(rows[0])
    }
    pub fn table(&self, id: &str, difficulty: i64, dungeon: &str) -> Result<&Value> {
        // Exact region and difficulty are independent specializations. Ambiguous
        // incomparable matches fail, never resolve by iteration order.
        let mut candidates = vec![];
        for d in [0, difficulty] {
            for r in ["", dungeon] {
                let key = loot_key(id, d, r);
                if let Some(v) = self.loot.get(&key) {
                    candidates.push((d != 0, !r.is_empty(), v));
                }
            }
        }
        candidates.sort_by_key(|v| (v.0, v.1));
        candidates.dedup();
        let best = candidates
            .iter()
            .filter(|(d, r, _)| {
                !candidates
                    .iter()
                    .any(|(dd, rr, _)| (dd >= d && rr >= r) && (dd > d || rr > r))
            })
            .collect::<Vec<_>>();
        if best.len() != 1 {
            return Err(format!("missing/ambiguous contextual loot table {id}"));
        }
        let v = best[0].2;
        if v.is_array() {
            return Err(format!("conflicting loot table {id}"));
        }
        known_keys(v, &["id", "difficulty", "dungeon", "entries"])?;
        Ok(v)
    }
}
pub(crate) fn positive(s: &str) -> Result<f64> {
    let n: f64 = s.parse().map_err(|_| format!("invalid curio weight {s}"))?;
    if !n.is_finite() || !(0. ..=1000000.).contains(&n) {
        return Err("invalid curio weight bound".into());
    }
    Ok(n)
}
pub(crate) fn parse_outcome(kind: &str, row: &[String]) -> Result<Outcome> {
    match kind {
        "Nothing" => Ok(Outcome::Nothing),
        "Loot" => {
            let mut draws = vec![];
            let mut nothing = 0.;
            for n in [7, 10, 13] {
                let id = col(row, n);
                if id.is_empty() || id == "N/A" {
                    continue;
                }
                if id == "Nothing" {
                    let p = col(row, n + 2)
                        .strip_suffix('%')
                        .ok_or("missing nothing percentage")?;
                    nothing = positive(p)? / 100.;
                } else {
                    let count = positive(col(row, n + 1))?;
                    if count.fract() != 0. || count < 1. || count > 100. {
                        return Err("invalid curio draw count".into());
                    }
                    draws.push(Draw {
                        table: id.into(),
                        count: count as u32,
                    });
                }
            }
            if draws.is_empty() || nothing >= 1. {
                return Err("invalid curio loot row".into());
            }
            Ok(Outcome::Loot {
                draws,
                nothing_chance: nothing,
            })
        }
        "Effect" => {
            let mut list = vec![];
            for n in [7, 10, 13] {
                let id = col(row, n);
                if id.is_empty() || id == "N/A" {
                    continue;
                }
                let w = positive(col(row, n + 1))?;
                if w > 0. {
                    list.push((id.into(), w));
                }
            }
            if list.is_empty() {
                return Err("empty curio effect row".into());
            }
            Ok(Outcome::Effect(list))
        }
        _ => Err(format!("unsupported initial curio result {kind}")),
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CurioEffect {
    pub id: String,
    pub chance: f64,
    pub heal: i32,
    pub stress: i32,
    pub cure: bool,
    pub bleed: i32,
    pub poison: i32,
    pub duration: u32,
}
impl CurioEffect {
    pub fn load(g: &GameData, id: &str) -> Result<Self> {
        let list = g
            .effects
            .records
            .iter()
            .filter(|r| r.key == "effect" && text(r, "name").ok() == Some(id))
            .collect::<Vec<_>>();
        let first = *list.first().ok_or("missing curio effect")?;
        if list.iter().any(|r| !library::same_record(first, r)) {
            return Err(format!("ambiguous curio effect {id}"));
        }
        for p in &first.params {
            if ![
                "name",
                "target",
                "curio_result_type",
                "chance",
                "heal",
                "healstress",
                "stress",
                "cure",
                "dotBleed",
                "dotPoison",
                "duration",
                "on_hit",
                "on_miss",
                "queue",
                "curio",
                "can_apply_on_death",
            ]
            .contains(&p.name.as_str())
            {
                return Err(format!("unsupported curio effect parameter {}", p.name));
            }
        }
        if !["target", "performer"].contains(&text(first, "target")?) {
            return Err("unsupported curio effect target".into());
        }
        let n = |key: &str| -> Result<i32> {
            if first.param(key).is_none() {
                return Ok(0);
            }
            let n = number(first, key)?;
            if n.fract() != 0. || !(0. ..=10000.).contains(&n) {
                return Err("invalid curio effect integer".into());
            }
            Ok(n as i32)
        };
        let chance = number(first, "chance")?;
        if !(0. ..=1.).contains(&chance) {
            return Err("invalid curio effect chance".into());
        }
        for (key, values) in [
            ("on_hit", vec!["true"]),
            ("on_miss", vec!["false"]),
            ("queue", vec!["true", "false"]),
            ("curio", vec!["1"]),
            ("can_apply_on_death", vec!["true", "false"]),
            ("curio_result_type", vec!["positive", "negative"]),
        ] {
            if first.param(key).is_some() && !values.contains(&text(first, key)?) {
                return Err(format!("unsupported curio effect metadata {key}"));
            }
        }
        if (n("stress")? > 0 && n("healstress")? > 0) || (n("dotBleed")? > 0 && n("dotPoison")? > 0)
        {
            return Err("ambiguous combined initial curio effect".into());
        }
        let cure = n("cure")?;
        if (n("dotBleed")? > 0 || n("dotPoison")? > 0) && n("duration")? == 0 {
            return Err("curio dot lacks duration".into());
        }
        if cure > 1 {
            return Err("invalid curio cure".into());
        }
        Ok(Self {
            id: id.into(),
            chance,
            heal: n("heal")?,
            stress: n("stress")? - n("healstress")?,
            cure: cure == 1,
            bleed: n("dotBleed")?,
            poison: n("dotPoison")?,
            duration: n("duration")? as u32,
        })
    }
}

pub(crate) type DecodedCurio<T> = (bool, Vec<(T, f64)>, BTreeMap<String, T>);
pub(crate) fn decode_curio_rows<T>(
    id: &str,
    rows: &[Vec<String>],
    parse: impl Fn(&str, &[String]) -> Result<T>,
) -> Result<DecodedCurio<T>> {
    let mut outcomes = vec![];
    let mut items = BTreeMap::new();
    let mut item = false;
    let mut full = None;
    for (i, row) in rows.iter().enumerate().skip(2) {
        if col(row, 2) == "FULL CURIO?" {
            full = Some(match rows.get(i + 1).map(|r| col(r, 2)) {
                Some("Yes") => true,
                Some("No") => false,
                _ => return Err("invalid FULL CURIO flag".into()),
            });
        }
        if col(row, 2) == "Item Interactions" {
            item = true;
            continue;
        }
        if item {
            let key = col(row, 4);
            if key.is_empty() {
                continue;
            }
            if items.insert(key.into(), parse(col(row, 5), row)?).is_some() {
                return Err("duplicate curio item mapping".into());
            }
        } else {
            let w = col(row, 5);
            if w.is_empty() {
                continue;
            }
            let weight = positive(w)?;
            if weight > 0. {
                outcomes.push((parse(col(row, 4), row)?, weight));
            }
        }
    }
    let full = full.ok_or("missing FULL CURIO flag")?;
    if outcomes.is_empty() {
        return Err(format!("curio {id} has no outcomes"));
    }
    Ok((full, outcomes, items))
}

/// Only the explicit 017 auto-open/outside journal form is supported in addition
/// to the legacy random page range. Never silently discard presentation flags.
pub fn fixed_journal_page(data: &Value, campaign: bool) -> Result<Option<u32>> {
    if campaign && data == &json!({"specific_page_index": 0}) {
        // The original THANKS table is the explicit zero-index final journal.
        // Missing presentation flags use the 018 candidate: ordinary carryable page.
        return Ok(Some(0));
    }
    if data.get("specific_page_index").is_some() && campaign {
        known_keys(
            data,
            &[
                "specific_page_index",
                "is_auto_open",
                "read_outside_of_dungeon",
            ],
        )?;
        if data["is_auto_open"] != true || data["read_outside_of_dungeon"] != true {
            return Err("unsupported fixed journal delivery flags".into());
        }
        return data["specific_page_index"]
            .as_u64()
            .filter(|n| (1..=10000).contains(n))
            .map(|n| Some(n as u32))
            .ok_or("invalid specific journal page".into());
    }
    known_keys(data, &["min_page_index", "max_page_index"])?;
    Ok(None)
}


