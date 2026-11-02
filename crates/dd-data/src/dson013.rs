//! Read-only, bounded DSON map decoder, independently implemented from the
//! public format specification. No save writer and no primitive type guessing.
//! https://github.com/robojumper/DarkestDungeonSaveEditor/blob/master/docs/dson.md
use crate::combat::Result;
use serde_json::{Map, Value};
pub fn hash(name: &str) -> u32 {
    name.bytes()
        .fold(0u32, |h, b| h.wrapping_mul(53).wrapping_add(u32::from(b)))
}
fn u32_at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .ok_or("truncated DSON integer")?
            .try_into()
            .unwrap(),
    ))
}
fn integer(b: &[u8], p: usize) -> Result<i32> {
    Ok(u32_at(b, p)? as i32)
}
struct Field {
    name: String,
    start: usize,
    end: usize,
    info: u32,
}
struct Reader<'a> {
    b: &'a [u8],
    fields: Vec<Field>,
    meta: Vec<[i32; 4]>,
    seen: Vec<bool>,
}
pub fn decode(b: &[u8]) -> Result<Value> {
    decode_depth(b, 0)
}
fn decode_depth(b: &[u8], depth: usize) -> Result<Value> {
    if depth > 16 || b.len() > 32 * 1024 * 1024 || b.get(..4) != Some(&[1, 177, 0, 0]) {
        return Err("invalid/oversized DSON map header".into());
    }
    if b.len() < 64 || u32_at(b, 8)? != 64 {
        return Err("unsupported DSON header length".into());
    }
    for p in [12, 28, 32, 36, 40, 52] {
        if u32_at(b, p)? != 0 {
            return Err("unknown DSON reserved header value".into());
        }
    }
    let objects = u32_at(b, 20)? as usize;
    let count = u32_at(b, 44)? as usize;
    let meta1 = u32_at(b, 24)? as usize;
    let meta2 = u32_at(b, 48)? as usize;
    let data = u32_at(b, 60)? as usize;
    let size = u32_at(b, 56)? as usize;
    if count == 0
        || count > 100000
        || objects == 0
        || objects > count
        || meta1 != 64
        || u32_at(b, 16)? as usize != objects * 16
        || meta2 != 64 + objects * 16
        || data != meta2 + count * 12
        || data.checked_add(size).is_none_or(|n| n > b.len())
    {
        return Err("invalid DSON table bounds".into());
    }
    if b.len() != data + size && !(b.len() == data + size + 1 && b.last() == Some(&0)) {
        return Err("DSON trailing bytes".into());
    }
    let meta = (0..objects)
        .map(|i| {
            Ok([
                integer(b, meta1 + i * 16)?,
                integer(b, meta1 + i * 16 + 4)?,
                integer(b, meta1 + i * 16 + 8)?,
                integer(b, meta1 + i * 16 + 12)?,
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let mut entries = vec![];
    let mut offsets = std::collections::BTreeSet::new();
    for i in 0..count {
        let p = meta2 + i * 12;
        let off = u32_at(b, p + 4)? as usize;
        let info = u32_at(b, p + 8)? & 0x7fff_ffff;
        let length = ((info >> 2) & 511) as usize;
        if info & 2 != 0
            || length < 2
            || off.checked_add(length).is_none_or(|n| n > size)
            || !offsets.insert(off)
        {
            return Err("invalid/duplicate DSON field offset".into());
        }
        let raw = &b[data + off..data + off + length];
        if raw.last() != Some(&0) {
            return Err("DSON field name lacks terminator".into());
        }
        let name = std::str::from_utf8(&raw[..length - 1])
            .map_err(|e| e.to_string())?
            .to_string();
        if hash(&name) != u32_at(b, p)? {
            return Err("DSON field name hash mismatch".into());
        }
        entries.push((name, off, length, info));
    }
    let offsets = offsets.into_iter().collect::<Vec<_>>();
    let ends = offsets
        .iter()
        .enumerate()
        .map(|(i, off)| (*off, offsets.get(i + 1).copied().unwrap_or(size)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let fields = entries
        .into_iter()
        .map(|(name, off, len, info)| {
            let end = ends[&off];
            if off + len > end {
                return Err("overlapping DSON field names".into());
            }
            Ok(Field {
                name,
                start: data + off + len,
                end: data + end,
                info,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut r = Reader {
        b,
        fields,
        meta,
        seen: vec![false; objects],
    };
    let (_, value, next) = r.field(0, None, depth)?;
    if next != count || r.seen.iter().any(|v| !*v) {
        return Err("DSON incomplete object tree".into());
    }
    Ok(value)
}
impl Reader<'_> {
    fn field(
        &mut self,
        index: usize,
        parent: Option<usize>,
        depth: usize,
    ) -> Result<(String, Value, usize)> {
        if depth > 128 {
            return Err("DSON object depth exceeded".into());
        }
        let f = self
            .fields
            .get(index)
            .ok_or("DSON object exceeds field table")?;
        let (name, start, end, info) = (f.name.clone(), f.start, f.end, f.info);
        if info & 1 != 0 {
            let object = (info >> 11) as usize;
            let m = *self.meta.get(object).ok_or("invalid DSON object index")?;
            if self.seen[object]
                || m[0] != parent.map_or(-1, |p| p as i32)
                || m[1] != index as i32
                || m[2] < 0
                || m[3] < m[2]
                || start != end
            {
                return Err("invalid DSON object metadata".into());
            }
            self.seen[object] = true;
            let mut next = index + 1;
            let mut map = Map::new();
            for _ in 0..m[2] {
                let (k, v, n) = self.field(next, Some(object), depth + 1)?;
                if map.insert(k, v).is_some() {
                    return Err("duplicate DSON object key".into());
                }
                next = n;
            }
            if next - index - 1 != m[3] as usize {
                return Err("DSON descendant count mismatch".into());
            }
            return Ok((name, Value::Object(map), next));
        }
        if info >> 11 != 0 {
            return Err("unknown primitive DSON info bits".into());
        }
        let aligned = (start + 3) & !3;
        let value = if [
            "populated",
            "reversed",
            "torch",
            "implied",
            "is_texture_override_valid",
            "hd_always_accessible",
            "crit_scout",
        ]
        .contains(&name.as_str())
        {
            if end - start != 1 || self.b[start] > 1 {
                return Err("invalid DSON bool".into());
            }
            Value::Bool(self.b[start] != 0)
        } else {
            // Alignment bytes have no semantic value in DSON. Original fixed
            // maps contain uninitialized bytes here, not just zero/CD padding.
            if aligned > end {
                return Err("invalid DSON alignment".into());
            }
            let payload = &self.b[aligned..end];
            match name.as_str() {
                "area" | "id" | "area_to" | "area_id" | "entrance_id" | "final_room_id" => {
                    if payload.len() != 4 {
                        return Err("invalid DSON four-character area id".into());
                    }
                    if payload == [0, 0, 0, 0] {
                        Value::Null
                    } else {
                        Value::String(
                            std::str::from_utf8(payload)
                                .map_err(|e| e.to_string())?
                                .to_string(),
                        )
                    }
                }
                "bounds" | "mappos" | "sidepos" => {
                    if ![8, 16].contains(&payload.len()) {
                        return Err("invalid DSON coordinate vector".into());
                    }
                    let mut m = Map::new();
                    for (i, key) in ["x", "y", "z", "w"]
                        .iter()
                        .take(payload.len() / 4)
                        .enumerate()
                    {
                        let v = f32::from_le_bytes(payload[i * 4..i * 4 + 4].try_into().unwrap());
                        if !v.is_finite() {
                            return Err("nonfinite map coordinate".into());
                        }
                        m.insert((*key).into(), Value::from(v));
                    }
                    Value::Object(m)
                }
                "texture_name" | "name" | "mash_name" | "static_save" => {
                    let length = u32_at(payload, 0)? as usize;
                    if length == 0
                        || length.checked_add(4) != Some(payload.len())
                        || payload.last() != Some(&0)
                    {
                        return Err("invalid DSON length-prefixed field".into());
                    }
                    if name == "static_save" {
                        decode_depth(&payload[4..], depth + 1)?
                    } else {
                        Value::String(
                            std::str::from_utf8(&payload[4..payload.len() - 1])
                                .map_err(|e| e.to_string())?
                                .into(),
                        )
                    }
                }
                "tile" | "version" | "knowledge" | "light" | "content" | "trap" | "mash_index"
                | "mash_type" | "kind" | "tile_to" | "type" | "curio_prop" | "cur" | "obstacle"
                | "texture_id" | "front_texture_id" | "unfilled" | "hap" | "sprite_id" => {
                    if payload.len() != 4 {
                        return Err(format!("invalid map integer {name}"));
                    }
                    Value::from(integer(payload, 0)?)
                }
                _ if name
                    .strip_prefix("tele")
                    .and_then(|s| s.strip_suffix("_dest"))
                    .is_some_and(|n| n.parse::<u32>().is_ok_and(|n| (1..=12).contains(&n)))
                    || name
                        .strip_prefix("hidden_room_")
                        .is_some_and(|n| n.parse::<u32>().is_ok()) =>
                {
                    if payload != [0, 0, 0, 0] {
                        return Err("unsupported nonzero map tag entry".into());
                    }
                    Value::from(0)
                }
                _ => return Err(format!("unknown DSON map primitive {name}")),
            }
        };
        Ok((name, value, index + 1))
    }
}

