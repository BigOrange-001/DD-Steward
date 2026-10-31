//! Explicit initial-level source boundary; the combat loader stays unchanged.
pub fn source_path(path: &str) -> bool {
    [
        "scripts/starting_save/persist.map.json",
        "scripts/starting_save/persist.raid.json",
        "maps/tutorial_crypts.dm",
        "campaign/quest/quest.plot_quests.json",
    ]
    .contains(&path)
        || (path.starts_with("curios/") && path.ends_with(".csv"))
        || (path.starts_with("loot/") && path.ends_with(".json"))
}

use crate::{combat::Result, overlay::Overlay};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub area: String,
    pub tile: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub position: Position,
    pub content: i64,
    pub curio_hash: u32,
    pub mash_index: i64,
    pub mash_name: Option<String>,
    pub neighbours: Vec<Position>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InitialMap {
    pub id: String,
    pub entrance: Position,
    pub rooms: BTreeSet<String>,
    pub cells: Vec<Cell>,
    pub raw: Value,
    pub source: String,
    pub source_layers: Vec<String>,
}
fn obj(v: &Value) -> Result<&serde_json::Map<String, Value>> {
    v.as_object().ok_or("expected map object".into())
}
fn int(v: &Value) -> Result<i64> {
    v.as_i64()
        .or_else(|| v.as_u64().map(|v| v as i64))
        .ok_or("expected map integer".into())
}
fn area_id(v: &Value) -> Result<String> {
    if let Some(s) = v.as_str() {
        return Ok(s.into());
    }
    let n = int(v)?;
    if !(0..=u32::MAX as i64).contains(&n) {
        return Err("invalid encoded area id".into());
    }
    String::from_utf8((n as u32).to_le_bytes().to_vec()).map_err(|e| e.to_string())
}
fn known(v: &Value, keys: &[&str], extra: impl Fn(&str) -> bool) -> Result<()> {
    for k in obj(v)?.keys() {
        if !keys.contains(&k.as_str()) && !extra(k) {
            return Err(format!("unknown initial map field {k}"));
        }
    }
    Ok(())
}
pub fn parse_map(
    id: &str,
    raw: Value,
    source: String,
    source_layers: Vec<String>,
) -> Result<InitialMap> {
    if !["old_road", "tutorial_crypts"].contains(&id) {
        return Err("unknown initial level".into());
    }
    let root = raw.get("data").unwrap_or(&raw);
    let m = &root["map"];
    known(
        m,
        &[
            "bounds",
            "entrance_id",
            "final_room_id",
            "areas",
            "static_dynamic",
            "populated",
        ],
        |_| false,
    )?;
    let split = m.get("static_dynamic");
    if let Some(split) = split {
        known(split, &["static_save", "areas"], |_| false)?;
        known(&split["static_save"], &["areas", "ext_data"], |_| false)?;
        if let Some(ext) = split["static_save"].get("ext_data") {
            known(ext, &["tile_textures"], |_| false)?;
            for v in obj(&ext["tile_textures"])?.values() {
                known(v, &["area", "tile", "texture_name"], |_| false)?;
                area_id(&v["area"])?;
                int(&v["tile"])?;
                if v["texture_name"].as_str().is_none() {
                    return Err("invalid initial texture metadata".into());
                }
            }
        }
    }

    let areas = split.map_or(&m["areas"], |s| &s["static_save"]["areas"]);
    let dynamic = split.map(|s| &s["areas"]);
    if obj(areas)?.is_empty() || obj(areas)?.len() > 256 {
        return Err("invalid map area count".into());
    }
    let mut cells = vec![];
    let mut rooms = BTreeSet::new();
    let mut doors = vec![];
    for (aid, area) in obj(areas)? {
        known(
            area,
            &[
                "id",
                "kind",
                "unfilled",
                "bounds",
                "knowledge",
                "reversed",
                "name",
                "torch",
                "tiles",
            ],
            |k| {
                k.strip_prefix("door")
                    .is_some_and(|s| s.parse::<usize>().is_ok_and(|n| n < 8))
            },
        )?;
        if area_id(&area["id"])? != *aid {
            return Err("area id mismatch".into());
        }
        let kind = int(&area["kind"])?;
        if ![0, 1].contains(&kind) {
            return Err(format!("unsupported map area kind {kind}"));
        }
        if kind == 0 {
            rooms.insert(aid.clone());
        }
        if let Some(dy) = dynamic {
            known(&dy[aid], &["knowledge", "reversed", "tiles"], |_| false)?;
            if obj(&dy[aid]["tiles"])?
                .keys()
                .ne(obj(&area["tiles"])?.keys())
            {
                return Err("static/dynamic tile mismatch".into());
            }
        }
        let tiles = obj(&area["tiles"])?;
        if tiles.is_empty() || tiles.len() > 256 || (kind == 0 && tiles.len() != 1) {
            return Err("invalid tile count".into());
        }
        for n in 0..tiles.len() {
            let key = format!("tile{n}");
            let t = tiles.get(&key).ok_or("noncontiguous tiles")?;
            known(
                t,
                &[
                    "type",
                    "light",
                    "content",
                    "hap",
                    "knowledge",
                    "cur",
                    "sprite_id",
                    "trap",
                    "obstacle",
                    "area_id",
                    "door_to",
                    "mappos",
                    "texture_id",
                    "front_texture_id",
                    "is_texture_override_valid",
                    "sidepos",
                    "mash_index",
                    "curio_prop",
                    "hd_always_accessible",
                ],
                |_| false,
            )?;
            if area_id(&t["area_id"])? != *aid {
                return Err("tile area mismatch".into());
            }
            let d = dynamic.map_or(t, |dy| &dy[aid]["tiles"][&key]);
            if dynamic.is_some() {
                known(
                    d,
                    &[
                        "light",
                        "content",
                        "knowledge",
                        "trap",
                        "mash_index",
                        "mash_type",
                        "mash_name",
                        "crit_scout",
                    ],
                    |_| false,
                )?;
            }
            let content = int(&d["content"])?;
            if ![0, 1, 3, 4, 6, 7, 8, 10].contains(&content) {
                return Err(format!("unsupported initial tile content {content}"));
            }
            let hash = t
                .get("curio_prop")
                .or_else(|| t.get("cur"))
                .map(int)
                .transpose()?
                .unwrap_or(0) as u32;
            if [6, 7, 10].contains(&content) && hash == 0 {
                return Err("curio content lacks prop hash".into());
            }
            let mut neighbours = vec![];
            if n > 0 {
                neighbours.push(Position {
                    area: aid.clone(),
                    tile: n - 1,
                });
            }
            if n + 1 < tiles.len() {
                neighbours.push(Position {
                    area: aid.clone(),
                    tile: n + 1,
                });
            }
            cells.push(Cell {
                position: Position {
                    area: aid.clone(),
                    tile: n,
                },
                content,
                curio_hash: hash,
                mash_index: int(&d["mash_index"])?,
                mash_name: d
                    .get("mash_name")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
                neighbours,
            });
        }
        // Room doors authoritatively join corridor endpoints. Corridor door_to values
        // in old saves can point to a room even on interior tiles; those are metadata.
        if kind == 0 {
            for n in 0..8 {
                let door = &area[format!("door{n}")];
                if door.is_null() {
                    continue;
                }
                known(door, &["area_to", "tile_to", "implied", "type"], |_| false)?;
                if int(&door["type"])? != 0 {
                    return Err("unsupported initial door type".into());
                }
                let to = area_id(&door["area_to"])?;
                if to != "none" {
                    let tile = usize::try_from(int(&door["tile_to"])?)
                        .map_err(|_| "negative door tile")?;
                    doors.push((
                        Position {
                            area: aid.clone(),
                            tile: 0,
                        },
                        Position { area: to, tile },
                    ));
                }
            }
        }
    }
    for (a, b) in doors {
        let bi = cells
            .iter()
            .position(|c| c.position == b)
            .ok_or("dangling map door")?;
        let ai = cells.iter().position(|c| c.position == a).unwrap();
        if rooms.contains(&b.area)
            || ![
                0,
                cells.iter().filter(|c| c.position.area == b.area).count() - 1,
            ]
            .contains(&b.tile)
        {
            return Err("room door does not target corridor endpoint".into());
        }
        cells[ai].neighbours.push(b.clone());
        cells[bi].neighbours.push(a);
    }
    for c in &mut cells {
        c.neighbours.sort();
        c.neighbours.dedup();
    }

    for c in cells.iter().filter(|c| !rooms.contains(&c.position.area)) {
        let count = cells
            .iter()
            .filter(|n| n.position.area == c.position.area)
            .count();
        let external = c
            .neighbours
            .iter()
            .filter(|n| n.area != c.position.area)
            .count();
        let expected =
            usize::from(c.position.tile == 0) + usize::from(c.position.tile + 1 == count);
        if external != expected {
            return Err("corridor endpoint must have exactly one room door".into());
        }
    }
    let entrance = Position {
        area: area_id(&m["entrance_id"])?,
        tile: 0,
    };
    if !rooms.contains(&entrance.area) {
        return Err("entrance must be a room".into());
    }
    let mut seen = BTreeSet::from([entrance.clone()]);
    let mut stack = vec![entrance.clone()];
    while let Some(p) = stack.pop() {
        for n in &cells
            .iter()
            .find(|c| c.position == p)
            .ok_or("missing graph tile")?
            .neighbours
        {
            if seen.insert(n.clone()) {
                stack.push(n.clone());
            }
        }
    }
    if seen.len() != cells.len() {
        return Err("disconnected initial map".into());
    }
    Ok(InitialMap {
        id: id.into(),
        entrance,
        rooms,
        cells,
        raw,
        source,
        source_layers,
    })
}
impl InitialMap {
    pub fn load(overlay: &Overlay, id: &str) -> Result<Self> {
        let path = match id {
            "old_road" => "scripts/starting_save/persist.map.json",
            "tutorial_crypts" => "maps/tutorial_crypts.dm",
            _ => return Err("unknown initial level".into()),
        };
        let bytes = overlay.read_initial(path)?;
        let raw = if bytes.starts_with(&[1, 177, 0, 0]) {
            crate::dson013::decode(&bytes)?
        } else {
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?
        };
        let layers = overlay
            .sources(path)
            .iter()
            .map(|n| overlay.layers[*n].id.clone())
            .collect();
        parse_map(id, raw, path.into(), layers)
    }
    pub fn cell(&self, p: &Position) -> Result<&Cell> {
        self.cells
            .iter()
            .find(|c| c.position == *p)
            .ok_or("unknown map position".into())
    }
}
