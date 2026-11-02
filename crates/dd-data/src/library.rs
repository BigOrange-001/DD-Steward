//! ID merge without choosing an order for conflicting definitions.
use crate::{
    combat::{Result, text},
    darkest::{Document, Record},
    overlay::Overlay,
};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn insert_json(map: &mut BTreeMap<String, Value>, id: String, value: Value) {
    match map.entry(id) {
        std::collections::btree_map::Entry::Vacant(e) => {
            e.insert(value);
        }
        std::collections::btree_map::Entry::Occupied(mut e) => {
            if e.get() == &value {
                return;
            }
            if let Some(variants) = e.get_mut().as_array_mut() {
                if !variants.contains(&value) {
                    variants.push(value);
                }
            } else {
                let old = e.get().clone();
                e.insert(Value::Array(vec![old, value]));
            }
        }
    }
}
pub fn unique_json<'a>(
    map: &'a BTreeMap<String, Value>,
    id: &str,
    kind: &str,
) -> Result<&'a Value> {
    let value = map.get(id).ok_or_else(|| format!("missing {kind} {id}"))?;
    if value.is_array() {
        return Err(format!(
            "ambiguous {kind} {id}: conflicting original definitions"
        ));
    }
    Ok(value)
}
pub fn json_library(
    overlay: &Overlay,
    prefix: &str,
    suffix: &str,
    array: &str,
) -> Result<BTreeMap<String, Value>> {
    Ok(json_library_ordered(overlay, prefix, suffix, array)?.0)
}
pub fn json_library_ordered(
    overlay: &Overlay,
    prefix: &str,
    suffix: &str,
    array: &str,
) -> Result<(BTreeMap<String, Value>, Vec<String>)> {
    Ok((*json_library_arc(overlay, prefix, suffix, array)?).clone())
}
pub type JsonLibrary = (BTreeMap<String, Value>, Vec<String>);
pub fn json_library_arc(
    overlay: &Overlay,
    prefix: &str,
    suffix: &str,
    array: &str,
) -> Result<std::sync::Arc<JsonLibrary>> {
    let files = overlay.combat_files(prefix, suffix);
    overlay.cached(
        &format!("json_library:{prefix}:{suffix}:{array}"),
        &files,
        || json_library_uncached(overlay, prefix, suffix, array),
    )
}
fn json_library_uncached(
    overlay: &Overlay,
    prefix: &str,
    suffix: &str,
    array: &str,
) -> Result<JsonLibrary> {
    let _span = crate::telemetry024c::scope("parse_json_library");
    let mut map = BTreeMap::new();
    let mut order = vec![];
    let mut files = overlay.combat_files(prefix, suffix);
    files.sort_by_key(|p| (overlay.sources(p).into_iter().min(), p.clone()));
    for path in files {
        for bytes in overlay.read_versions(&path)? {
            let raw: Value = serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))?;
            for value in raw[array]
                .as_array()
                .ok_or_else(|| format!("{path}: missing {array}"))?
            {
                let id = value["id"]
                    .as_str()
                    .ok_or_else(|| format!("{path}: missing {array} id"))?;
                if !map.contains_key(id) {
                    order.push(id.to_owned());
                }
                insert_json(&mut map, id.into(), value.clone());
            }
        }
    }
    Ok((map, order))
}
pub fn read_document(overlay: &Overlay, relative: &str) -> Result<Document> {
    let bytes = overlay.read(relative)?;
    parse_document(&bytes, relative)
}
fn parse_document(bytes: &[u8], relative: &str) -> Result<Document> {
    if bytes.contains(&0) {
        return Err(format!("{relative}: binary .darkest content"));
    }
    let source = std::str::from_utf8(bytes).map_err(|e| format!("{relative}: not UTF-8: {e}"))?;
    let d = crate::darkest::parse_str(source);
    if !d.warnings.is_empty() {
        return Err(format!("{relative}: {:?}", d.warnings));
    }
    Ok(d)
}
/// Locations and whitespace are not content. Retain parameter/value ordering
/// and spelling; unexplained differences remain ambiguous rather than guessed.
pub fn same_record(a: &Record, b: &Record) -> bool {
    a.key == b.key
        && a.params.len() == b.params.len()
        && a.params.iter().zip(&b.params).all(|(a, b)| {
            a.name == b.name
                && a.values
                    .iter()
                    .map(|v| &v.text)
                    .eq(b.values.iter().map(|v| &v.text))
        })
}
pub fn same_document(a: &Document, b: &Document) -> bool {
    a.records.len() == b.records.len()
        && a.records
            .iter()
            .zip(&b.records)
            .all(|(a, b)| same_record(a, b))
}
pub fn effects(overlay: &Overlay) -> Result<Document> {
    effects_with_modes(overlay, true)
}
pub fn effects_with_modes(overlay: &Overlay, include_mode_effects: bool) -> Result<Document> {
    let mut result = Document {
        records: vec![],
        warnings: vec![],
    };
    let mut ids: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for path in overlay.combat_files("effects/", ".effects.darkest") {
        if !include_mode_effects && path.ends_with("/mode.effects.darkest") {
            continue;
        }
        for bytes in overlay.read_versions(&path)? {
            for record in parse_document(&bytes, &path)?.records {
                let id = text(&record, "name")?.to_owned();
                let variants = ids.entry(id).or_default();
                if !variants
                    .iter()
                    .any(|i| same_record(&result.records[*i], &record))
                {
                    variants.push(result.records.len());
                    result.records.push(record);
                }
            }
        }
    }
    Ok(result)
}
pub fn units(overlay: &Overlay, heroes: bool) -> Result<BTreeMap<String, Vec<Document>>> {
    let mut units: BTreeMap<String, Vec<Document>> = BTreeMap::new();
    for path in overlay.combat_files(
        if heroes { "heroes/" } else { "monsters/" },
        ".info.darkest",
    ) {
        let parts = path.split('/').collect::<Vec<_>>();
        let expected = if heroes { 3 } else { 4 };
        if parts.len() != expected {
            return Err(format!("unsupported combat unit path {path}"));
        }
        let id = parts[parts.len() - 2];
        if parts.last().unwrap() != &format!("{id}.info.darkest") {
            return Err(format!("unit filename mismatch {path}"));
        }
        for bytes in overlay.read_versions(&path)? {
            let d = parse_document(&bytes, &path)?;
            let variants = units.entry(id.to_owned()).or_default();
            if !variants.iter().any(|old| same_document(old, &d)) {
                variants.push(d);
            }
        }
    }
    Ok(units)
}
pub fn unique_unit<'a>(
    units: &'a BTreeMap<String, Vec<Document>>,
    id: &str,
    kind: &str,
) -> Result<&'a Document> {
    let variants = units
        .get(id)
        .ok_or_else(|| format!("missing {kind} {id}"))?;
    if variants.len() != 1 {
        return Err(format!("{id}: ambiguous {kind} definitions"));
    }
    Ok(&variants[0])
}

