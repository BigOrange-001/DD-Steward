//! Original fixed map readers. Geometry, hidden connections and semantic refs
//! are preserved independently of the later expedition settlement system.
use crate::{combat::Result, maps014::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn node(id: String, area: String, room: bool, position: [f64; 2], metadata: Value) -> Node {
    Node {
        id,
        area,
        room,
        position,
        hidden: false,
        tags: BTreeSet::new(),
        content: Content::default(),
        gate: None,
        teleport: None,
        links: vec![],
        metadata,
    }
}
fn empty(id: &str, region: &str, source: Source, raw: Value) -> Map {
    Map {
        id: id.into(),
        region: region.into(),
        source,
        raw,
        nodes: BTreeMap::new(),
        entrance: String::new(),
        goals: BTreeSet::new(),
        fog: false,
        persistent: false,
        key_sources: BTreeMap::new(),
        teleport_targets: BTreeMap::new(),
        generation: None,
    }
}
fn obj(v: &Value) -> Result<&serde_json::Map<String, Value>> {
    v.as_object().ok_or("expected fixed map object".into())
}
fn coord(v: &Value) -> Result<[f64; 2]> {
    Ok([
        v["x"]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or("invalid map x")?,
        v["y"]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or("invalid map y")?,
    ])
}
fn hash_ref(v: Option<&Value>) -> Result<Option<String>> {
    let n = v.map(integer).transpose()?.unwrap_or(0);
    if !(i32::MIN as i64..=u32::MAX as i64).contains(&n) {
        return Err("invalid curio hash range".into());
    }
    Ok((n != 0).then(|| format!("hash:{}", n as u32)))
}
pub fn dson(
    id: &str,
    region: &str,
    bytes: &[u8],
    source: Source,
    props: &BTreeMap<u32, String>,
) -> Result<Map> {
    let raw = if bytes.starts_with(&[1, 177, 0, 0]) {
        crate::dson013::decode(bytes)?
    } else {
        serde_json::from_slice(bytes).map_err(|e| e.to_string())?
    };
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
    )?;
    let static_areas = m
        .get("static_dynamic")
        .map_or(&m["areas"], |s| &s["static_save"]["areas"]);
    let dynamic = m.get("static_dynamic").map(|s| &s["areas"]);
    let mut map = empty(id, region, source, raw.clone());
    let mut edges = vec![];
    let mut tags = BTreeMap::new();
    for (aid, a) in obj(static_areas)? {
        for key in obj(a)?.keys() {
            if ![
                "id",
                "kind",
                "unfilled",
                "bounds",
                "knowledge",
                "reversed",
                "name",
                "torch",
                "tiles",
                "tags",
            ]
            .contains(&key.as_str())
                && !key
                    .strip_prefix("door")
                    .is_some_and(|n| n.parse::<u32>().is_ok_and(|n| n < 8))
            {
                return Err(format!("unknown fixed area field {key}"));
            }
        }
        if area_id(&a["id"])? != *aid {
            return Err("fixed area id mismatch".into());
        }
        let kind = integer(&a["kind"])?;
        if ![0, 1].contains(&kind) {
            return Err("unsupported fixed area kind".into());
        }
        let room = kind == 0;
        let area_tags = a
            .get("tags")
            .map(obj)
            .transpose()?
            .map(|v| v.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        for tag in &area_tags {
            tags.entry(tag.clone())
                .or_insert_with(BTreeSet::new)
                .insert(format!("{aid}:0"));
        }
        let tiles = obj(&a["tiles"])?;
        if tiles.is_empty() || tiles.len() > 65536 || (room && tiles.len() != 1) {
            return Err("invalid fixed tile count".into());
        }
        for i in 0..tiles.len() {
            let t = tiles
                .get(&format!("tile{i}"))
                .ok_or("noncontiguous fixed tiles")?;
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
            )?;
            let d = dynamic.map_or(t, |dy| &dy[aid]["tiles"][format!("tile{i}")]);
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
                )?;
            }
            if area_id(&t["area_id"])? != *aid {
                return Err("fixed tile area mismatch".into());
            }
            let key = format!("{aid}:{i}");
            let mut n = node(
                key.clone(),
                aid.clone(),
                room,
                coord(&t["mappos"])?,
                json!({"static":t,"dynamic":d}),
            );
            n.tags = area_tags.clone();
            n.hidden = n.tags.iter().any(|t| t.starts_with("hidden_room_"));
            let code = integer(&d["content"])?;
            if ![0, 1, 3, 4, 6, 7, 8, 9, 10, 13].contains(&code) {
                return Err(format!("unsupported fixed content code {code}"));
            }
            n.content.code = Some(code);
            n.content.curio = hash_ref(t.get("curio_prop").or_else(|| t.get("cur")))?.map(|s| {
                s.strip_prefix("hash:")
                    .and_then(|s| s.parse::<u32>().ok())
                    .and_then(|h| props.get(&h).cloned())
                    .unwrap_or(s)
            });
            let mash = integer(&d["mash_index"])?;
            if [1, 6, 10].contains(&code) {
                n.content.battle = Some(
                    d.get("mash_name")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("index:{mash}")),
                );
            }
            if code == 3 {
                n.content.obstacle = Some(format!(
                    "index:{}",
                    t.get("obstacle").map(integer).transpose()?.unwrap_or(0)
                ));
            }
            if code == 4 {
                n.content.trap = Some(format!(
                    "index:{}",
                    d.get("trap").map(integer).transpose()?.unwrap_or(0)
                ));
            }
            n.content.hunger = code == 8;
            map.nodes.insert(key.clone(), n);
            if i > 0 {
                edges.push((format!("{aid}:{}", i - 1), key, None));
            }
        }
        if room {
            for i in 0..8 {
                let Some(door) = a.get(format!("door{i}")) else {
                    continue;
                };
                known(door, &["area_to", "tile_to", "implied", "type"])?;
                let to = area_id(&door["area_to"])?;
                if to == "none" {
                    continue;
                }
                let ty = integer(&door["type"])?;
                if ![0, 1, 2].contains(&ty) {
                    return Err("unknown map door type".into());
                }
                let hidden = if ty == 2 {
                    Some(
                        area_tags
                            .iter()
                            .find(|t| t.starts_with("hidden_room_"))
                            .cloned()
                            .ok_or("secret door lacks room tag")?,
                    )
                } else {
                    None
                };
                edges.push((
                    format!("{aid}:0"),
                    format!("{to}:{}", integer(&door["tile_to"])?),
                    hidden,
                ));
            }
        }
    }
    for (a, b, h) in edges {
        map.link(&a, &b, h)?;
    }
    map.entrance = format!("{}:0", area_id(&m["entrance_id"])?);
    if !m["final_room_id"].is_null() {
        let final_id = area_id(&m["final_room_id"])?;
        if final_id != "none" {
            map.goals.insert(format!("{final_id}:0"));
        }
    }
    map.teleport_targets = tags
        .into_iter()
        .filter(|(k, _)| k.starts_with("tele"))
        .collect();
    map.validate()?;
    Ok(map)
}
fn token_value(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}
pub fn csv_map(
    id: &str,
    region: &str,
    bytes: &[u8],
    source: Source,
    gates: &BTreeMap<String, String>,
    keys: &BTreeMap<String, KeyAward>,
    choices: &BTreeMap<String, String>,
) -> Result<Map> {
    // csv's CRLF position can stop before LF. Normalize line endings first so
    // skipped empty records can be counted without inventing rows between records.
    let normalized = std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())?
        .replace("\r\n", "\n");
    let bytes = normalized.as_bytes();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes);
    let mut rows = vec![];
    let mut previous_end = 0usize;
    let mut record = csv::StringRecord::new();
    while reader.read_record(&mut record).map_err(|e| e.to_string())? {
        let end = reader.position().byte() as usize;
        let mut start = previous_end;
        while start < end && [10, 13].contains(&bytes[start]) {
            if bytes[start] == 13 && bytes.get(start + 1) == Some(&10) {
                start += 2;
            } else {
                start += 1;
            }
            rows.push(Vec::<String>::new());
        }
        rows.push(
            record
                .iter()
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>(),
        );
        previous_end = end;
    }
    if rows.len() < 3 || rows[0].first().map(String::as_str) != Some("global") {
        return Err("unknown fixed CSV header".into());
    }
    let mut map = empty(id, region, source, json!({"csv":rows}));
    map.fog = match rows[1].first().map(String::as_str) {
        Some("fog_of_war=on") => true,
        Some("fog_of_war=off") => false,
        _ => return Err("unknown map fog setting".into()),
    };
    let mut grid = BTreeMap::new();
    let mut hidden = vec![];
    let mut entry = vec![];
    for (y, row) in rows.iter().enumerate().skip(2) {
        for (x, raw) in row.iter().enumerate() {
            if raw.is_empty() || raw.starts_with("//") {
                continue;
            }
            let mut split = raw.split(';');
            let kind = split.next().unwrap();
            if !["room", "r", "c"].contains(&kind) {
                if raw.contains(';') {
                    return Err(format!("unknown map cell kind {kind}"));
                }
                continue;
            }
            let room = kind != "c";
            let key = format!("{x},{y}");
            let mut n = node(
                key.clone(),
                key.clone(),
                room,
                [x as f64, y as f64],
                json!({"cell":raw}),
            );
            let mut collected: BTreeMap<String, Vec<Option<String>>> = BTreeMap::new();
            for part in split.filter(|v| !v.is_empty()) {
                let (k, v) = part
                    .split_once('=')
                    .map_or((part, None), |(k, v)| (k, Some(token_value(v))));
                let list = collected.entry(k.to_string()).or_default();
                if !list.contains(&v) {
                    list.push(v);
                }
            }
            let mut params = BTreeMap::new();
            let mut visual = BTreeMap::new();
            for (k, values) in collected {
                let v = if values.len() == 1 {
                    values[0].clone()
                } else if k == "texture" {
                    if values.iter().any(Option::is_none) {
                        return Err("missing duplicate texture id".into());
                    }
                    visual.insert(k.clone(), values);
                    Some("retained_visual_ambiguity".into())
                } else if k == "curio" {
                    let at = format!("{id}@{x},{y}");
                    let chosen = choices
                        .get(&at)
                        .ok_or_else(|| format!("conflicting CSV curio at {at}"))?;
                    if !values.contains(&Some(chosen.clone())) {
                        return Err("selected CSV curio definition is absent".into());
                    }
                    Some(chosen.clone())
                } else {
                    return Err(format!("conflicting CSV map token {k}"));
                };
                params.insert(k, v);
            }
            n.metadata["visual_ambiguities"] = json!(visual);
            for (k, v) in &params {
                match k.as_str() {
                    "texture" | "icon" => {
                        v.as_ref().ok_or("map graphic metadata lacks value")?;
                    }
                    "entrance" => {
                        if !room || v.is_some() {
                            return Err("invalid map entrance marker".into());
                        }
                        entry.push(key.clone());
                    }
                    "final_room" => {
                        if !room || v.is_some() {
                            return Err("invalid map final marker".into());
                        }
                        map.goals.insert(key.clone());
                    }
                    "mash" => {
                        n.content.battle = Some(v.clone().unwrap_or_else(|| {
                            if room {
                                "pool:room".into()
                            } else {
                                "pool:hallway".into()
                            }
                        }))
                    }
                    "curio" => n.content.curio = Some(v.clone().ok_or("map curio lacks id")?),
                    "trap" => n.content.trap = Some(v.clone().ok_or("map trap lacks id")?),
                    "obstacle" => {
                        n.content.obstacle = Some(v.clone().ok_or("map obstacle lacks id")?)
                    }
                    "hunger" => {
                        if v.is_some() {
                            return Err("invalid hunger flag".into());
                        }
                        n.content.hunger = true;
                    }
                    "quest_curio" => {
                        if v.is_some() {
                            return Err("invalid quest curio flag".into());
                        }
                        n.content.quest = true;
                    }
                    "tag" => {
                        n.tags.insert(v.clone().ok_or("map tag lacks id")?);
                    }
                    "name" => {
                        n.teleport = Some(format!(
                            "{}_dest",
                            v.clone().ok_or("map named obstacle lacks id")?
                        ));
                    }
                    "door_prop" => {
                        let name = v.as_ref().ok_or("map gate lacks prop")?;
                        n.gate = Some(
                            gates
                                .get(name)
                                .ok_or_else(|| format!("unknown map gate {name}"))?
                                .clone(),
                        );
                    }
                    "hidden_door" => {
                        hidden.push((key.clone(), v.clone().ok_or("hidden door lacks tag")?))
                    }
                    "hidden" => {
                        if v.is_some() {
                            return Err("invalid hidden flag".into());
                        }
                        n.hidden = true;
                    }
                    "disable_torch" => {
                        if v.is_some() {
                            return Err("invalid disable_torch flag".into());
                        }
                    }
                    _ => return Err(format!("unknown fixed CSV map token {k}")),
                }
            }
            if n.content.quest {
                map.goals.insert(key.clone());
            }
            if let Some(curio) = &n.content.curio
                && let Some(award) = keys.get(curio)
            {
                map.key_sources.insert(key.clone(), award.clone());
            }
            grid.insert((x, y), key.clone());
            map.nodes.insert(key, n);
        }
    }
    if entry.len() != 1 {
        return Err("fixed CSV requires exactly one entrance".into());
    }
    map.entrance = entry.remove(0);
    for (&(x, y), id) in &grid {
        for at in [(x + 1, y), (x, y + 1)] {
            if let Some(to) = grid.get(&at) {
                if map.nodes[id].hidden || map.nodes[to].hidden {
                    continue;
                }
                map.link(id, to, None)?;
            }
        }
    }
    for (from, tag) in hidden {
        let targets = map
            .nodes
            .iter()
            .filter(|(_, n)| n.tags.contains(&tag) && n.hidden)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if targets.len() != 1 {
            return Err(format!("ambiguous hidden map target {tag}"));
        }
        map.link(&from, &targets[0], Some(tag))?;
    }
    for (id, n) in &map.nodes {
        for tag in &n.tags {
            if tag.ends_with("_dest") {
                map.teleport_targets
                    .entry(tag.clone())
                    .or_default()
                    .insert(id.clone());
            }
        }
    }

    let gates = map
        .nodes
        .iter()
        .filter(|(_, n)| n.gate.is_some())
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in gates {
        let rooms = map.nodes[&id]
            .links
            .iter()
            .filter(|l| l.hidden.is_none() && map.nodes[&l.to].room)
            .map(|l| l.to.clone())
            .collect::<Vec<_>>();
        if rooms.len() != 1 {
            return Err("CSV gate must identify one adjacent locked room".into());
        }
        map.nodes.get_mut(&id).unwrap().metadata["gate_room"] = json!(rooms[0]);
    }
    for n in map.nodes.values() {
        if let Some(tag) = &n.teleport
            && (n.content.obstacle.as_deref() != Some("mist")
                || !map.teleport_targets.contains_key(tag))
        {
            return Err(format!("missing map teleport group {tag}"));
        }
    }
    map.validate()?;
    Ok(map)
}
