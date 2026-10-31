//! Explicit, finite source choice approved for original difficulty conflicts.
//! Raw library variants and their provenance remain available to the audit.
use crate::{
    combat::{Result, text},
    darkest::Record,
    library::same_record,
    overlay::Overlay,
};
use std::collections::BTreeMap;
pub const IDS: [&str; 26] = [
    "Ancestor Summon Perfect",
    "Ancestor Summon Mixed",
    "Ancestor Summon Flawed",
    "Ancestor Summon Nebula",
    "Ancestor Damage Marked",
    "Ancestor Mark",
    "Flawed Stun",
    "Ancestor Perfect Bleed",
    "Ancestor Flawed Bleed",
    "Ancestor Perfect Stress",
    "Ancestor Flawed Stress",
    "Ancestor Small Stress",
    "Ancestor Shuffle",
    "Ancestor Disrupt",
    "Ancestor Big Rider Stress",
    "Ancestor Big Contemplate Stress",
    "Egg Heal Hero",
    "Egg Blight Hero",
    "Pod AOE Blight",
    "Ancestor Fuse Two",
    "Ancestor Fuse All",
    "Heart Stress",
    "Heart Stress Rider",
    "Heart Stun",
    "Heart Bleed",
    "Heart Blight",
];
pub(crate) fn select(overlay: &Overlay) -> Result<BTreeMap<String, Record>> {
    let result = BTreeMap::new();
    if overlay
        .config
        .syntax_choices
        .get("difficulty_effect_source")
        .map(String::as_str)
        != Some("mode_file")
    {
        return Ok(result);
    }
    let path = "effects/mode.effects.darkest";
    // A miniature fixture can lack the mode library. In that case there is no
    // selected replacement: the ordinary unique-ID/error rule remains in force.
    if !overlay.paths.contains_key(path) {
        return Ok(result);
    }
    let bytes = overlay.read(path)?;
    let d = crate::darkest::parse_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?);
    if !d.warnings.is_empty() {
        return Err("mode effect source contains parse warnings".into());
    }
    select_records(d)
}
fn select_records(d: crate::darkest::Document) -> Result<BTreeMap<String, Record>> {
    let mut result = BTreeMap::new();
    for r in d.records {
        if r.key != "effect" {
            return Err("unknown mode effect record".into());
        }
        let id = text(&r, "name")?;
        if !IDS.contains(&id) {
            continue;
        }
        if let Some(old) = result.get(id)
            && !same_record(old, &r)
        {
            return Err(format!("selected mode effect remains ambiguous: {id}"));
        }
        result.insert(id.to_string(), r);
    }
    Ok(result)
}
