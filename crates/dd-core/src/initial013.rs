//! First two fixed levels, with a host-owned state and a separate visible view.
//! Travel hazards and encounters are explicit host handoffs, not auto-cleared.
use super::{Resolve, Rng};
use dd_data::{
    combat::{GameData, Result, Rules, Stats},
    curios013::{CurioEffect, Curios, Outcome},
    hero010::{CarryDot, CarryDotKind, CarryStatus},
    initial013::{InitialMap, Position},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "candidate-initial-013";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub hero_classes: BTreeSet<String>,
    pub map: InitialMap,
    pub curios: Curios,
    pub effects: BTreeMap<String, CurioEffect>,
    pub rules: Rules,
    pub content_id: String,
    pub difficulty: i64,
    pub dungeon: String,
    pub torch: i64,
    pub quest_source: serde_json::Value,
}
impl Input {
    pub fn load(g: &GameData, id: &str) -> Result<Self> {
        Self::load_with_court_loot(g, id, false)
    }
    pub fn load_with_court_loot(g: &GameData, id: &str, court: bool) -> Result<Self> {
        let map = InitialMap::load(&g.overlay, id)?;
        let curios = Curios::load_with_court_loot(&g.overlay, court)?;
        let mut effects = BTreeMap::new();
        for cell in &map.cells {
            if cell.curio_hash != 0 {
                let (_, c) = curios.prop(cell.curio_hash)?;
                for o in c.outcomes.iter().map(|v| &v.0).chain(c.items.values()) {
                    if let Outcome::Effect(list) = o {
                        for (name, _) in list {
                            effects.insert(name.clone(), CurioEffect::load(g, name)?);
                        }
                    }
                }
            }
        }
        let path = if id == "old_road" {
            "scripts/starting_save/persist.raid.json"
        } else {
            "campaign/quest/quest.plot_quests.json"
        };
        let quest_source: serde_json::Value =
            serde_json::from_slice(&g.overlay.read_initial(path)?).map_err(|e| e.to_string())?;
        let quest = if id == "old_road" {
            &quest_source["data"]["raid_instance"]
        } else {
            quest_source["plot_quests"]
                .as_array()
                .ok_or("missing plot quests")?
                .iter()
                .find(|v| v["id"].as_str() == Some("plot_tutorial_crypts"))
                .ok_or("missing initial ruins quest")?
                .get("quest")
                .ok_or("missing initial quest definition")?
        };
        let difficulty = quest["difficulty"]
            .as_i64()
            .ok_or("missing initial difficulty")?;
        let dungeon: String = quest["dungeon"]
            .as_str()
            .ok_or("missing initial dungeon")?
            .into();
        let initial_raid: serde_json::Value = serde_json::from_slice(
            &g.overlay
                .read_initial("scripts/starting_save/persist.raid.json")?,
        )
        .map_err(|e| e.to_string())?;
        let torch = initial_raid["data"]["party"]["torchlight"]
            .as_i64()
            .ok_or("missing initial torchlight")?;

        for cell in &map.cells {
            if cell.curio_hash != 0 {
                let (_, c) = curios.prop(cell.curio_hash)?;
                for outcome in c.outcomes.iter().map(|o| &o.0).chain(c.items.values()) {
                    if let Outcome::Loot { draws, .. } = outcome {
                        for draw in draws {
                            curios.validate_draw(&draw.table, difficulty, &dungeon, &mut vec![])?;
                        }
                    }
                }
            }
        }
        Ok(Self {
            hero_classes: g.hero_ids().cloned().collect(),
            map,
            curios,
            effects,
            rules: g.rules.clone(),
            content_id: g.overlay.config.id()?,
            difficulty,
            dungeon,
            torch,
            quest_source,
        })
    }
}
/// Narrow host actor for this map/curio phase. Advanced builds/automatic quirk
/// interactions belong to the future expedition host, not silently imported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub class: String,
    pub stats: Stats,
    pub hp: i32,
    pub stress: i32,
    pub resolve: Resolve,
    pub history: BTreeMap<String, u32>,
    pub status: CarryStatus,
}
impl Actor {
    pub fn fresh(g: &GameData, class: &str) -> Result<Self> {
        if !["crusader", "highwayman", "plague_doctor", "vestal"].contains(&class) {
            return Err("initial curio actor supports only the first four classes".into());
        }
        let ids = g.hero_skill_ids(class)?;
        let skill = ids.first().ok_or("initial class has no skills")?;
        let b = g.hero(class, 0, 0, &[skill.as_str()])?;
        Ok(Self {
            class: class.into(),
            hp: b.stats.hp,
            stats: b.stats,
            stress: 0,
            resolve: Resolve::Calm,
            history: BTreeMap::new(),
            status: CarryStatus::default(),
        })
    }
    fn modifier(&self, r: &Rules, kind: &str, sub: &str) -> f64 {
        let id = match &self.resolve {
            Resolve::Calm => return 0.,
            Resolve::Afflicted(id) | Resolve::Virtuous(id) => id,
        };
        r.globals
            .as_ref()
            .and_then(|g| g.traits.iter().find(|t| &t.id == id))
            .map_or(0., |t| {
                t.buffs
                    .iter()
                    .filter(|b| {
                        b.stat_type == kind
                            && b.stat_sub_type == sub
                            && (b.rule_type == "always"
                                || (b.rule_type == "virtued"
                                    && matches!(self.resolve, Resolve::Virtuous(_))))
                            && !b.is_false_rule
                    })
                    .map(|b| b.amount)
                    .sum()
            })
    }
    pub fn maximum_hp(&self, r: &Rules) -> i32 {
        ((f64::from(self.stats.hp) + self.modifier(r, "combat_stat_add", "max_hp"))
            * (1. + self.modifier(r, "combat_stat_multiply", "max_hp")).max(0.))
        .round()
        .clamp(1., 1e6) as i32
    }
    pub fn carry_into(
        &self,
        mut hero: dd_data::combat::Blueprint,
    ) -> Result<dd_data::combat::Blueprint> {
        self.validate()?;
        if !hero.hero
            || hero.id != self.class
            || hero.stats != self.stats
            || hero.mechanics.hero.carry.is_some()
        {
            return Err("initial actor handoff requires matching fresh hero".into());
        }
        use dd_data::hero010::{CarryResolve, HeroCarry};
        hero.mechanics.hero.initial_hp = Some(self.hp);
        hero.mechanics.hero.resolve_history = self.history.clone();
        hero.mechanics.hero.carry = Some(HeroCarry {
            status: self.status.clone(),
            stress: self.stress,
            resolve: match &self.resolve {
                Resolve::Calm => CarryResolve::Calm,
                Resolve::Afflicted(id) => CarryResolve::Afflicted(id.clone()),
                Resolve::Virtuous(id) => CarryResolve::Virtuous(id.clone()),
            },
            buffs: vec![],
        });
        Ok(hero)
    }
    fn validate(&self) -> Result<()> {
        self.status.validate()?;
        if [
            self.stats.dodge,
            self.stats.protection,
            self.stats.accuracy,
            self.stats.crit,
            self.stats.death_resist,
            self.stats.stun_resist,
            self.stats.bleed_resist,
            self.stats.blight_resist,
            self.stats.debuff_resist,
            self.stats.move_resist,
        ]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1e6)
        {
            return Err("invalid initial actor stat".into());
        }
        if self.stats.hp < 1
            || self.hp < 1
            || self.hp > self.stats.hp
            || !(0..200).contains(&self.stress)
            || self.history.len() > 1000
            || self.history.values().any(|n| *n >= 1000000)
        {
            return Err("unsupported/dead initial curio actor".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    pub kind: String,
    pub id: String,
    pub amount: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Handoff {
    pub at: Position,
    pub content: i64,
    pub mash_index: i64,
    pub mash_name: Option<String>,
    pub content_id: String,
    pub level: String,
    pub serial: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Action {
    Move(Position),
    Interact { actor: usize, item: Option<String> },
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleCell {
    pub at: Position,
    pub curio: Option<String>,
    pub cleared: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleActor {
    pub class: String,
    pub hp: i32,
    pub max_hp: i32,
    pub stress: i32,
    pub resolve: Resolve,
    pub dots: Vec<CarryDot>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Observation {
    pub level: String,
    pub position: Position,
    pub visited: Vec<VisibleCell>,
    pub exits: Vec<Position>,
    pub actors: Vec<VisibleActor>,
    pub inventory: BTreeMap<String, u32>,
    pub rewards: Vec<Reward>,
    pub pending: bool,
    pub actions: Vec<Action>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    pub curio: String,
    pub item: Option<String>,
    pub consumed: bool,
    pub effect: Option<String>,
    pub resisted: bool,
    pub rewards: Vec<Reward>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    input: std::sync::Arc<Input>,
    id: String,
}
impl std::ops::Deref for Definition {
    type Target = Input;
    fn deref(&self) -> &Input {
        &self.input
    }
}
impl Definition {
    pub fn new(input: Input) -> Result<Self> {
        input.rules.validate()?;
        input.curios.require_zero_light_bonus(input.torch)?;
        use sha2::{Digest, Sha256};
        let id = format!(
            "{PROFILE};sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&input).map_err(|e| e.to_string())?)
        );
        Ok(Self {
            input: std::sync::Arc::new(input),
            id,
        })
    }
    pub fn candidate_id(&self) -> String {
        format!(
            "{PROFILE};manual-curios-bright-fresh-v1;loot={:?}",
            self.input.curios.loot_choices
        )
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    candidate_id: String,
    profile: String,
    input_id: String,
    position: Position,
    visited: BTreeSet<Position>,
    cleared: BTreeSet<Position>,
    resolved: BTreeSet<Position>,
    actors: Vec<Actor>,
    inventory: BTreeMap<String, u32>,
    rewards: Vec<Reward>,
    rng: Rng,
    pending: Option<Handoff>,
    serial: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    profile: String,
    input: Definition,
    position: Position,
    visited: BTreeSet<Position>,
    cleared: BTreeSet<Position>,
    resolved: BTreeSet<Position>,
    actors: Vec<Actor>,
    inventory: BTreeMap<String, u32>,
    rewards: Vec<Reward>,
    rng: Rng,
    pending: Option<Handoff>,
    serial: u64,
}
impl Session {
    pub fn new(
        input: Input,
        actors: Vec<Actor>,
        inventory: BTreeMap<String, u32>,
        seed: u64,
    ) -> Result<Self> {
        let p = input.map.entrance.clone();
        let s = Self {
            profile: PROFILE.into(),
            input: Definition::new(input)?,
            position: p.clone(),
            visited: BTreeSet::from([p]),
            cleared: BTreeSet::new(),
            resolved: BTreeSet::new(),
            actors,
            inventory,
            rewards: vec![],
            rng: Rng(seed),
            pending: None,
            serial: 0,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn candidate_id(&self) -> String {
        self.input.candidate_id()
    }
    pub fn definitions(&self) -> Definition {
        self.input.clone()
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.validate()?;
        let c = Checkpoint {
            candidate_id: self.candidate_id(),
            profile: self.profile.clone(),
            input_id: self.input.id.clone(),
            position: self.position.clone(),
            visited: self.visited.clone(),
            cleared: self.cleared.clone(),
            resolved: self.resolved.clone(),
            actors: self.actors.clone(),
            inventory: self.inventory.clone(),
            rewards: self.rewards.clone(),
            rng: self.rng.clone(),
            pending: self.pending.clone(),
            serial: self.serial,
        };
        serde_json::to_string(&c).map_err(|e| e.to_string())
    }
    pub fn restore(json: &str, expected: &Input) -> Result<Self> {
        Self::restore_bound(json, &Definition::new(expected.clone())?)
    }
    pub fn restore_bound(json: &str, expected: &Definition) -> Result<Self> {
        if json.len() > 8 * 1024 * 1024 {
            return Err("initial snapshot too large".into());
        }
        let c: Checkpoint = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if c.profile != PROFILE
            || c.input_id != expected.id
            || c.candidate_id != expected.candidate_id()
        {
            return Err("initial snapshot profile/content/source mismatch".into());
        }
        let s = Self {
            profile: c.profile,
            input: expected.clone(),
            position: c.position,
            visited: c.visited,
            cleared: c.cleared,
            resolved: c.resolved,
            actors: c.actors,
            inventory: c.inventory,
            rewards: c.rewards,
            rng: c.rng,
            pending: c.pending,
            serial: c.serial,
        };
        s.validate()?;
        Ok(s)
    }
    fn validate(&self) -> Result<()> {
        if self.profile != PROFILE
            || self.input.content_id != self.input.rules.content.id()?
            || self.actors.is_empty()
            || self.actors.len() > 4
            || self.inventory.len() > 1000
            || self.inventory.values().any(|n| *n == 0 || *n > 1000000)
            || self.rewards.len() > 10000
            || self.serial > 1000000
        {
            return Err("invalid initial snapshot bounds".into());
        }
        self.input.map.cell(&self.position)?;
        for a in &self.actors {
            a.validate()?;
            if !self.input.hero_classes.contains(&a.class) {
                return Err("unknown initial actor class".into());
            }
            if let Resolve::Afflicted(id) | Resolve::Virtuous(id) = &a.resolve {
                let t = self
                    .input
                    .rules
                    .globals
                    .as_ref()
                    .and_then(|g| g.traits.iter().find(|t| &t.id == id))
                    .ok_or("unknown curio resolve state")?;
                if t.virtue != matches!(a.resolve, Resolve::Virtuous(_)) {
                    return Err("curio resolve type mismatch".into());
                }
            }
        }
        for id in self.inventory.keys() {
            self.input.curios.inventory_item(id)?;
        }
        for p in self
            .visited
            .iter()
            .chain(&self.cleared)
            .chain(&self.resolved)
        {
            self.input.map.cell(p)?;
        }
        if !self.visited.contains(&self.position)
            || !self.visited.is_superset(&self.cleared)
            || !self.visited.is_superset(&self.resolved)
        {
            return Err("invalid initial discovery state".into());
        }
        if let Some(h) = &self.pending {
            let c = self.input.map.cell(&self.position)?;
            if h.at != self.position
                || h.content != c.content
                || h.mash_index != c.mash_index
                || h.mash_name != c.mash_name
                || h.serial != self.serial
                || h.content_id != self.input.content_id
                || h.level != self.input.map.id
                || self.resolved.contains(&h.at)
            {
                return Err("invalid initial handoff".into());
            }
        }
        for p in &self.cleared {
            if self.input.map.cell(p)?.curio_hash == 0 {
                return Err("cleared non-curio".into());
            }
        }
        Ok(())
    }
    pub fn actors(&self) -> &[Actor] {
        &self.actors
    }
    pub fn inventory(&self) -> &BTreeMap<String, u32> {
        &self.inventory
    }
    pub fn reward_ledger(&self) -> &[Reward] {
        &self.rewards
    }
    pub fn handoff(&self) -> Option<&Handoff> {
        self.pending.as_ref()
    }
    /// Trusted host acknowledgement only. It represents externally completed
    /// settlement; this phase does not claim to simulate that event. Never policy API.
    pub fn resume(&mut self, receipt: &Handoff) -> Result<()> {
        if self.pending.as_ref() != Some(receipt) {
            return Err("stale/mismatched host handoff receipt".into());
        }
        self.resolved.insert(receipt.at.clone());
        self.pending = None;
        Ok(())
    }
    pub fn observe(&self) -> Result<Observation> {
        let mut visited = vec![];
        for p in &self.visited {
            let c = self.input.map.cell(p)?;
            let curio = if c.curio_hash != 0 {
                Some(self.input.curios.prop(c.curio_hash)?.0)
            } else {
                None
            };
            visited.push(VisibleCell {
                at: p.clone(),
                curio,
                cleared: self.cleared.contains(p),
            });
        }
        let exits = self.input.map.cell(&self.position)?.neighbours.clone();
        let mut actions = vec![];
        if self.pending.is_none() {
            actions.extend(exits.iter().cloned().map(Action::Move));
            let c = self.input.map.cell(&self.position)?;
            if c.curio_hash != 0 && !self.cleared.contains(&self.position) {
                let (_, curio) = self.input.curios.prop(c.curio_hash)?;
                for actor in 0..self.actors.len() {
                    actions.push(Action::Interact { actor, item: None });
                    if curio.full {
                        for id in self.inventory.keys() {
                            actions.push(Action::Interact {
                                actor,
                                item: Some(id.clone()),
                            });
                        }
                    }
                }
            }
        }
        Ok(Observation {
            level: self.input.map.id.clone(),
            position: self.position.clone(),
            visited,
            exits,
            actors: self
                .actors
                .iter()
                .map(|a| VisibleActor {
                    class: a.class.clone(),
                    hp: a.hp,
                    max_hp: a.maximum_hp(&self.input.rules),
                    stress: a.stress,
                    resolve: a.resolve.clone(),
                    dots: a.status.dots.clone(),
                })
                .collect(),
            inventory: self.inventory.clone(),
            rewards: self.rewards.clone(),
            pending: self.pending.is_some(),
            actions,
        })
    }
    pub fn act(&mut self, action: Action) -> Result<Option<Interaction>> {
        let mut next = self.clone();
        if !next.observe()?.actions.contains(&action) {
            return Err("illegal initial-level action".into());
        }
        let result = match action {
            Action::Move(to) => {
                next.position = to.clone();
                next.visited.insert(to.clone());
                let c = next.input.map.cell(&to)?;
                if [1, 3, 4, 6, 8, 10].contains(&c.content) && !next.resolved.contains(&to) {
                    next.serial += 1;
                    next.pending = Some(Handoff {
                        at: to,
                        content: c.content,
                        mash_index: c.mash_index,
                        mash_name: c.mash_name.clone(),
                        content_id: next.input.content_id.clone(),
                        level: next.input.map.id.clone(),
                        serial: next.serial,
                    });
                }
                None
            }
            Action::Interact { actor, item } => Some(next.interact(actor, item)?),
        };
        next.validate()?;
        *self = next;
        Ok(result)
    }
    fn pick(&mut self, weights: &[f64]) -> Result<usize> {
        let total: f64 = weights.iter().sum();
        if weights.iter().any(|w| !w.is_finite() || *w < 0.) || !total.is_finite() || total <= 0. {
            return Err("invalid curio/loot weights".into());
        }
        let mut roll = (self.rng.next() >> 11) as f64 / ((1u64 << 53) as f64) * total;
        for (i, w) in weights.iter().enumerate() {
            roll -= w;
            if roll < 0. {
                return Ok(i);
            }
        }
        Ok(weights.iter().rposition(|w| *w > 0.).unwrap())
    }
    fn interact(&mut self, actor: usize, item: Option<String>) -> Result<Interaction> {
        let hash = self.input.map.cell(&self.position)?.curio_hash;
        let (_, c) = self.input.curios.prop(hash)?;
        let mut consumed = false;
        let outcome = if let Some(id) = &item {
            let Some(o) = c.items.get(id) else {
                return Ok(Interaction {
                    curio: c.id,
                    item,
                    consumed: false,
                    effect: None,
                    resisted: false,
                    rewards: vec![],
                });
            };
            let n = self.inventory.get_mut(id).ok_or("missing curio item")?;
            *n -= 1;
            if *n == 0 {
                self.inventory.remove(id);
            }
            consumed = true;
            o.clone()
        } else {
            let n = self.pick(&c.outcomes.iter().map(|o| o.1).collect::<Vec<_>>())?;
            c.outcomes[n].0.clone()
        };
        let mut result = Interaction {
            curio: c.id,
            item,
            consumed,
            effect: None,
            resisted: false,
            rewards: vec![],
        };
        match outcome {
            Outcome::Nothing => {}
            Outcome::Loot {
                draws,
                nothing_chance,
            } => {
                if nothing_chance == 0. || !self.rng.roll(nothing_chance) {
                    for d in draws {
                        for _ in 0..d.count {
                            self.loot(&d.table, &mut vec![], &mut result.rewards)?;
                        }
                    }
                }
            }
            Outcome::Effect(effects) => {
                let n = self.pick(&effects.iter().map(|o| o.1).collect::<Vec<_>>())?;
                let e = self
                    .input
                    .effects
                    .get(&effects[n].0)
                    .ok_or("missing loaded curio effect")?
                    .clone();
                result.resisted = !self.effect(actor, &e)?;
                result.effect = Some(e.id);
            }
        }
        for r in &result.rewards {
            if r.kind == "supply" {
                let n = self.inventory.entry(r.id.clone()).or_default();
                *n = n.checked_add(r.amount).ok_or("inventory overflow")?;
            }
        }
        self.rewards.extend(result.rewards.clone());
        self.cleared.insert(self.position.clone());
        Ok(result)
    }
    fn loot(&mut self, id: &str, stack: &mut Vec<String>, rewards: &mut Vec<Reward>) -> Result<()> {
        if stack.len() > 16 || stack.iter().any(|s| s == id) || rewards.len() > 1000 {
            return Err("cyclic/oversized curio loot".into());
        }
        stack.push(id.into());
        let dungeon = self.input.dungeon.clone();
        let table = self
            .input
            .curios
            .table(id, self.input.difficulty, &dungeon)?
            .clone();
        let entries = table["entries"].as_array().ok_or("missing loot entries")?;
        let weights = entries
            .iter()
            .map(|v| v["chances"].as_f64().ok_or("invalid loot chances".into()))
            .collect::<Result<Vec<_>>>()?;
        let entry = &entries[self.pick(&weights)?];
        dd_data::curios013::known_keys(entry, &["type", "chances", "data"])?;
        let data = &entry["data"];
        match entry["type"].as_str().ok_or("missing loot type")? {
            "nothing" => {
                dd_data::curios013::known_keys(data, &[])?;
            }
            "table" => {
                dd_data::curios013::known_keys(data, &["table"])?;
                self.loot(
                    data["table"].as_str().ok_or("missing nested loot table")?,
                    stack,
                    rewards,
                )?;
            }
            "item" => {
                dd_data::curios013::known_keys(data, &["type", "id", "amount"])?;
                let kind = data["type"].as_str().ok_or("missing loot item kind")?;
                if !["supply", "provision", "gold", "gem", "heirloom", "estate"].contains(&kind) {
                    return Err(format!("unsupported loot item kind {kind}"));
                }
                let amount = data["amount"]
                    .as_u64()
                    .filter(|n| (1..=1000000).contains(n))
                    .ok_or("invalid loot amount")? as u32;
                rewards.push(Reward {
                    kind: kind.into(),
                    id: data["id"].as_str().ok_or("missing loot item id")?.into(),
                    amount,
                });
            }
            "journal_page" => {
                dd_data::curios013::known_keys(data, &["min_page_index", "max_page_index"])?;
                let min = data["min_page_index"]
                    .as_i64()
                    .ok_or("missing journal bound")?;
                let max = data["max_page_index"]
                    .as_i64()
                    .ok_or("missing journal bound")?;
                if min < 1 || max < min || max > 10000 {
                    return Err("invalid journal bounds".into());
                }
                let owned = self
                    .rewards
                    .iter()
                    .chain(rewards.iter())
                    .filter(|r| r.kind == "journal_page")
                    .map(|r| r.id.clone())
                    .collect::<BTreeSet<_>>();
                let eligible = (min..=max)
                    .map(|n| n.to_string())
                    .filter(|s| !owned.contains(s))
                    .collect::<Vec<_>>();
                if !eligible.is_empty() {
                    let n = self.rng.below(eligible.len() as u64) as usize;
                    rewards.push(Reward {
                        kind: "journal_page".into(),
                        id: eligible[n].clone(),
                        amount: 1,
                    });
                }
            }
            "trinket" => {
                dd_data::curios013::known_keys(data, &["rarity"])?;
                let rarity = data["rarity"].as_str().ok_or("missing trinket rarity")?;
                let mut eligible = vec![];
                for (id, v) in &self.input.curios.trinkets {
                    if v.is_array() {
                        if v.as_array()
                            .unwrap()
                            .iter()
                            .any(|v| v["rarity"].as_str() == Some(rarity))
                        {
                            return Err(format!("ambiguous eligible trinket {id}"));
                        }
                        continue;
                    }
                    if let Some(classes) = v.get("hero_class_requirements") {
                        let classes = classes
                            .as_array()
                            .ok_or("invalid trinket class requirements")?;
                        if classes.iter().any(|c| c.as_str().is_none()) {
                            return Err("invalid trinket class id".into());
                        }
                        if !classes.is_empty()
                            && !classes
                                .iter()
                                .any(|c| self.input.hero_classes.contains(c.as_str().unwrap()))
                        {
                            continue;
                        }
                    }
                    if v["rarity"].as_str() != Some(rarity) {
                        continue;
                    }
                    if !v["origin_dungeon"]
                        .as_str()
                        .is_none_or(|s| s.is_empty() || s == dungeon.as_str())
                    {
                        continue;
                    }
                    let limit = v["limit"].as_i64().ok_or("missing trinket limit")?;
                    let owned = self
                        .rewards
                        .iter()
                        .chain(rewards.iter())
                        .filter(|r| r.kind == "trinket" && r.id == *id)
                        .map(|r| i64::from(r.amount))
                        .sum::<i64>();
                    if limit > 0 && owned >= limit {
                        continue;
                    }
                    eligible.push(id.clone());
                }
                if eligible.is_empty() {
                    return Err(format!("no eligible {rarity} trinket"));
                }
                let n = self.rng.below(eligible.len() as u64) as usize;
                rewards.push(Reward {
                    kind: "trinket".into(),
                    id: eligible[n].clone(),
                    amount: 1,
                });
            }
            kind => return Err(format!("unsupported initial loot type {kind}")),
        }
        stack.pop();
        Ok(())
    }
    fn effect(&mut self, actor: usize, e: &CurioEffect) -> Result<bool> {
        let a = &self.actors[actor];
        let resist = if e.bleed > 0 {
            a.stats.bleed_resist + a.modifier(&self.input.rules, "resistance", "bleed")
        } else if e.poison > 0 {
            a.stats.blight_resist + a.modifier(&self.input.rules, "resistance", "poison")
        } else {
            0.
        };
        if !self.rng.roll((e.chance - resist).clamp(0., 1.)) {
            return Ok(false);
        }
        let a = &mut self.actors[actor];
        a.stress = (a.stress + e.stress).max(0);
        if a.stress >= 200 {
            return Err("initial curio heart attack requires expedition host settlement".into());
        }
        if a.stress == 0 && matches!(a.resolve, Resolve::Afflicted(_)) {
            a.resolve = Resolve::Calm;
        }
        if a.stress >= 100 && a.resolve == Resolve::Calm {
            let r = &self.input.rules;
            let globals = r
                .globals
                .as_ref()
                .ok_or("curio resolve globals unavailable")?;
            let virtue = !self.rng.roll(r.stress.affliction_base_onset_chance.clamp(
                r.stress.affliction_min_chance,
                r.stress.affliction_max_chance,
            ));
            let choices = globals
                .traits
                .iter()
                .filter(|t| t.virtue == virtue && t.required_dungeon_history.is_empty())
                .cloned()
                .collect::<Vec<_>>();
            let weights = choices
                .iter()
                .map(|t| {
                    let base = if virtue {
                        r.base_virtue_history_wf
                    } else {
                        r.base_affliction_history_wf
                    };
                    let n = *a.history.get(&t.id).unwrap_or(&0) as usize;
                    let inc = &r.affliction_history_increment;
                    if inc.is_empty() {
                        return 0.;
                    }
                    (base + t.generate_chance_modifier + inc[n.min(inc.len() - 1)]).max(0.)
                })
                .collect::<Vec<_>>();
            let n = self.pick(&weights)?;
            let a = &mut self.actors[actor];
            *a.history.entry(choices[n].id.clone()).or_default() += 1;
            a.resolve = if virtue {
                Resolve::Virtuous(choices[n].id.clone())
            } else {
                Resolve::Afflicted(choices[n].id.clone())
            };
            if virtue {
                a.stress = self
                    .input
                    .rules
                    .stress_affliction_critically_resisted
                    .round() as i32;
            }
            a.hp = a.hp.min(a.maximum_hp(&self.input.rules));
        }
        let max_hp = self.actors[actor].maximum_hp(&self.input.rules);
        let a = &mut self.actors[actor];
        a.hp = (a.hp + e.heal).min(max_hp);
        if e.cure {
            a.status
                .dots
                .retain(|d| !matches!(d.kind, CarryDotKind::Bleed | CarryDotKind::Blight));
        }
        for (kind, amount) in [
            (CarryDotKind::Bleed, e.bleed),
            (CarryDotKind::Blight, e.poison),
        ] {
            if amount > 0 {
                a.status.dots.push(CarryDot {
                    kind,
                    amount,
                    remaining: e.duration,
                    can_crit_heal: false,
                });
            }
        }
        Ok(true)
    }
}

