//! Source generator parameters, not claims about the original executable's algorithm.
use crate::{
    combat::{Result, number, text},
    darkest::Record,
    maps014::Source,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const ROOM_COUNTS: &[&str] = &[
    "room_battle",
    "room_guarded_curio",
    "room_curio",
    "room_guarded_treasure",
    "room_treasure",
];
pub const HALL_COUNTS: &[&str] = &[
    "hallway_battle",
    "hallway_trap",
    "hallway_obstacle",
    "hallway_curio",
    "hallway_hunger",
];
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub dungeon: String,
    pub size: String,
    pub quest: String,
}
impl Key {
    pub fn id(&self) -> String {
        format!("{}:{}:{}", self.dungeon, self.size, self.quest)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub key: Key,
    pub base_rooms: usize,
    pub corridor_budget: usize,
    pub grid: [usize; 2],
    pub spacing: usize,
    pub goal_rooms: usize,
    pub connectivity: f64,
    pub min_final_distance: usize,
    pub counts: BTreeMap<String, [u32; 2]>,
    pub nudge: [u32; 2],
    pub neighbour_bias: f64,
    pub centre_bias: f64,
    pub prisoner: Option<f64>,
    pub raw: BTreeMap<String, Vec<String>>,
    pub sources: Vec<Source>,
}
impl Rule {
    pub fn parse(r: &Record, source: Source) -> Result<Self> {
        if r.key != "map" {
            return Err("unknown generator record".into());
        }
        let mut raw: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for p in &r.params {
            if raw
                .insert(
                    p.name.clone(),
                    p.values.iter().map(|v| v.text.clone()).collect(),
                )
                .is_some()
            {
                return Err("duplicate generator parameter".into());
            }
        }
        let known = [
            "size",
            "quest_type",
            "dungeon_type",
            "base_room_number",
            "base_corridor_number",
            "gridsize",
            "spacing",
            "goal_room_number",
            "connectivity",
            "min_final_distance",
            "hallway_battle",
            "hallway_trap",
            "hallway_obstacle",
            "hallway_curio",
            "hallway_hunger",
            "total_room_battles",
            "room_battle",
            "room_guarded_curio",
            "room_curio",
            "room_guarded_treasure",
            "room_treasure",
            "secret_rooms",
            "nudge_span",
            "nudge_neighbour_bias",
            "nudge_map_centre_bias",
            "prisoner_room_chance",
        ];
        if let Some(k) = raw.keys().find(|k| !known.contains(&k.as_str())) {
            return Err(format!("unknown generator parameter {k}"));
        }
        let int = |key: &str| -> Result<usize> {
            let n = number(r, key)?;
            if n.fract() != 0. || !(0. ..=65536.).contains(&n) {
                return Err(format!("invalid generator integer {key}"));
            }
            Ok(n as usize)
        };
        let pair = |key: &str| -> Result<[u32; 2]> {
            let p = raw
                .get(key)
                .ok_or_else(|| format!("missing generator {key}"))?;
            if p.len() != 2 {
                return Err("generator pair requires two values".into());
            }
            let a = p[0].parse::<u32>().map_err(|_| "invalid generator pair")?;
            let b = p[1].parse::<u32>().map_err(|_| "invalid generator pair")?;
            if a > b || b > 65536 {
                return Err("invalid generator range".into());
            }
            Ok([a, b])
        };
        // gridsize is an ordered dimension pair, not a range.
        let g = raw.get("gridsize").ok_or("missing generator grid")?;
        if g.len() != 2 {
            return Err("invalid generator grid".into());
        }
        let grid = [
            g[0].parse().map_err(|_| "invalid grid width")?,
            g[1].parse().map_err(|_| "invalid grid height")?,
        ];
        let mut counts = BTreeMap::new();
        for key in ROOM_COUNTS
            .iter()
            .chain(HALL_COUNTS)
            .chain([&"total_room_battles", &"secret_rooms"])
        {
            counts.insert((*key).into(), pair(key)?);
        }
        let rule = Self {
            key: Key {
                dungeon: text(r, "dungeon_type")?.into(),
                size: text(r, "size")?.into(),
                quest: text(r, "quest_type")?.into(),
            },
            base_rooms: int("base_room_number")?,
            corridor_budget: int("base_corridor_number")?,
            grid,
            spacing: int("spacing")?,
            goal_rooms: int("goal_room_number")?,
            connectivity: number(r, "connectivity")?,
            min_final_distance: int("min_final_distance")?,
            counts,
            nudge: pair("nudge_span")?,
            neighbour_bias: number(r, "nudge_neighbour_bias")?,
            centre_bias: number(r, "nudge_map_centre_bias")?,
            prisoner: r
                .param("prisoner_room_chance")
                .map(|_| number(r, "prisoner_room_chance"))
                .transpose()?,
            raw,
            sources: vec![source],
        };
        rule.validate()?;
        Ok(rule)
    }
    pub fn validate(&self) -> Result<()> {
        if !["crypts", "weald", "warrens", "cove", "courtyard"].contains(&self.key.dungeon.as_str())
            || !["short", "medium", "long"].contains(&self.key.size.as_str())
            || ![
                "explore",
                "cleanse",
                "gather",
                "activate",
                "inventory_activate",
                "kill_boss",
            ]
            .contains(&self.key.quest.as_str())
        {
            return Err("unknown generator selector".into());
        }
        if self.grid.iter().any(|n| *n == 0 || *n > 64)
            || self.base_rooms < 2
            || self.base_rooms > self.grid[0] * self.grid[1]
            || self.goal_rooms != self.base_rooms
            || self.base_rooms > 256
            || self.spacing == 0
            || self.spacing > 64
            || self.min_final_distance >= self.base_rooms
            || self.corridor_budget > 65536
            || self.nudge[0] > self.nudge[1]
            || self.nudge[1] > 256
        {
            return Err("unsupported/infeasible generator dimensions".into());
        }
        if !self.connectivity.is_finite()
            || !(0. ..=1.).contains(&self.connectivity)
            || !self.neighbour_bias.is_finite()
            || self.neighbour_bias.abs() > 65536.
            || !self.centre_bias.is_finite()
            || self.centre_bias.abs() > 65536.
            || self
                .prisoner
                .is_some_and(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
        {
            return Err("invalid generator probability/bias".into());
        }
        for key in ROOM_COUNTS
            .iter()
            .chain(HALL_COUNTS)
            .chain([&"total_room_battles", &"secret_rooms"])
        {
            let r = self
                .counts
                .get(*key)
                .ok_or("missing generator count range")?;
            if r[0] > r[1] || r[1] > 65536 {
                return Err("invalid generator count range".into());
            }
        }
        if self.counts["total_room_battles"][0] as usize >= self.base_rooms {
            return Err("room battle minimum exceeds non-entrance capacity".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenerationRecord {
    pub topology_attempts: u32,
    pub candidate_seed: u64,
    pub rule: Rule,
    pub seed: u64,
    pub preplaced: BTreeMap<String, u32>,
    pub final_counts: BTreeMap<String, u32>,
    pub room_graph_edges: usize,
    pub final_distance: usize,
    pub corridor_attempts: usize,
    pub algorithm: String,
}
