//! Connected opening stage. The policy sees only Observation; definitions and
//! checkpoints belong to the host. Every mutation commits atomically.
use super::{
    api012::{BattleInput, BattleSession, Decision},
    field015::PartyKernel,
    *,
};
use dd_data::{
    combat::GameData,
    curios013::Outcome as CurioOutcome,
    inventory015::{Bag, Item, Stack},
    maps014::Candidates as MapCandidates,
    opening015::{Candidates, Data, PROFILE as EXP_PROFILE},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
#[path = "expedition017.rs"]
pub mod campaign;
#[derive(Clone)]
pub struct Definition {
    campaign: Option<Arc<campaign::Spec>>,
    data: Arc<Data>,
    game: Arc<GameData>,
    id: String,
    maps: Vec<crate::maps014::Definition>,
    field_rules: Vec<dd_data::combat::Rules>,
    cache: Arc<Mutex<Caches>>,
}
#[derive(Default)]
struct Caches {
    physical_items: BTreeMap<String, dd_data::trinket010::Trinket>,
    inventories: BTreeMap<Vec<String>, Arc<dd_data::inventory015::InventoryRules>>,
    supplies: BTreeMap<String, dd_data::items::Supply>,
    blueprints: BTreeSet<Vec<u8>>,
    catalogs: BTreeMap<Vec<String>, BTreeMap<String, Blueprint>>,
}
impl Definition {
    pub fn load(game: GameData, candidates: Candidates) -> Result<Self> {
        let data = Data::load(&game, candidates)?;
        Self::new(game, data)
    }
    fn new(game: GameData, data: Data) -> Result<Self> {
        Self::from_shared(Arc::new(game), data)
    }
    pub(crate) fn from_shared(game: Arc<GameData>, data: Data) -> Result<Self> {
        use sha2::{Digest, Sha256};
        let records = |d: &dd_data::darkest::Document| {
            d.records
                .iter()
                .map(|r| {
                    (
                        &r.key,
                        r.params
                            .iter()
                            .map(|p| {
                                (
                                    &p.name,
                                    p.values.iter().map(|v| v.text.clone()).collect::<Vec<_>>(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .map(|(k, p)| {
                    (
                        k.clone(),
                        p.into_iter()
                            .map(|(k, v)| (k.clone(), v))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let inputs = serde_json::to_vec(&(
            &data,
            records(&game.effects),
            &game.buffs,
            game.info_documents(true).map(records).collect::<Vec<_>>(),
        ))
        .map_err(|e| e.to_string())?;
        let id = format!("{EXP_PROFILE};sha256:{:x}", Sha256::digest(inputs));
        let maps = data
            .levels
            .iter()
            .map(|l| {
                crate::maps014::Definition::new(
                    l.map.clone(),
                    data.content.clone(),
                    MapCandidates::for_content(&data.content)?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let field_rules = data
            .levels
            .iter()
            .map(|l| {
                let quest = if l.map.id == "old_road" {
                    &l.quest
                } else {
                    &l.quest["quest"]
                };
                let mut r = game.rules.clone();
                r.context = Some(game.battle_context(
                    quest["dungeon"].as_str().ok_or("missing region")?.into(),
                    dd_data::build010::Surprise::Neither,
                    false,
                )?);
                r.lighting = Some(game.lighting(100)?);
                Ok(r)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            campaign: None,
            data: Arc::new(data),
            game,
            id,
            maps,
            field_rules,
            cache: Arc::new(Mutex::new(Caches::default())),
        })
    }
    fn supply(&self, id: &str) -> Result<dd_data::items::Supply> {
        if let Some(supply) = self
            .campaign
            .as_ref()
            .and_then(|s| s.context.bonuses.replacements.get(id))
        {
            return Ok(supply.clone());
        }
        if let Some(s) = self
            .cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .supplies
            .get(id)
            .cloned()
        {
            return Ok(s);
        }
        let s = if id == "torch"
            && self
                .campaign
                .as_ref()
                .is_some_and(|c| c.sources.quest["dungeon"] == "courtyard")
        {
            self.game.bloodlight_torch()?
        } else if id == "torch" {
            self.game.supply_with_light_candidate(
                id,
                dd_data::items::HostItemCandidate {
                    light_amount: self.data.candidates.torch_amount,
                    source: self.data.candidates.source.clone(),
                },
            )?
        } else {
            self.game.supply(id)?
        };
        self.cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .supplies
            .insert(id.into(), s.clone());
        Ok(s)
    }
    fn catalog(&self, ids: &[String]) -> Result<BTreeMap<String, Blueprint>> {
        let mut key = ids.to_vec();
        key.sort();
        key.dedup();
        if let Some(c) = self
            .cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .catalogs
            .get(&key)
            .cloned()
        {
            return Ok(c);
        }
        let c = self
            .game
            .entity_catalog(&key.iter().map(String::as_str).collect::<Vec<_>>())?;
        self.cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .catalogs
            .insert(key, c.clone());
        Ok(c)
    }
    fn validate_base(&self, b: &Blueprint) -> Result<()> {
        if let Some(spec) = &self.campaign {
            if !spec.canon.values().any(|canonical| canonical == b) {
                return Err("ranked hero differs from bound campaign deployment".into());
            }
            return Ok(());
        }
        let bytes = serde_json::to_vec(b).map_err(|e| e.to_string())?;
        if self
            .cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .blueprints
            .contains(&bytes)
        {
            return Ok(());
        }
        let skills = b.skills.iter().map(|s| s.id.as_str()).collect::<Vec<_>>();
        let fresh = self.game.hero(&b.id, 0, 0, &skills)?;
        let mut expected = self
            .game
            .equip_build(fresh, b.mechanics.build.input.clone())?;
        if let Some(d) = &b.mechanics.hero.difficulty {
            expected = self.game.hero_at_difficulty(
                expected,
                d.resolve_level,
                d.quest_level,
                d.entering_raid,
            )?;
        }
        if expected != *b {
            return Err("opening hero blueprint differs from original definitions".into());
        }
        self.cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .blueprints
            .insert(bytes);
        Ok(())
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn data(&self) -> &Data {
        &self.data
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hero {
    pub guid: u64,
    pub name: String,
    pub xp: u32,
    pub base: Blueprint,
    pub state: HeroResult,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    OldRoad,
    Preparation,
    FirstRuins,
    Expedition,
    Finished,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StageResult {
    pub level: String,
    pub completed: bool,
    pub outcome: String,
    pub heroes: Vec<(u64, HeroResult)>,
    pub brought_home: Vec<Stack>,
    pub rewards: Vec<Stack>,
    pub xp: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    campaign: Option<campaign::Progress>,
    profile: String,
    definition_id: String,
    phase: Phase,
    level: usize,
    heroes: Vec<Hero>,
    party: Vec<u64>,
    bag: Bag,
    wallet: BTreeMap<String, u32>,
    map: String,
    battle: Option<String>,
    battle_party: Vec<u64>,
    battle_start_supplies: BTreeMap<String, u32>,
    loot: Vec<Stack>,
    rolled: Vec<crate::initial013::Reward>,
    torch: u8,
    rng: Rng,
    next_guid: u64,
    results: Vec<StageResult>,
    battles: u32,
    interactions: u32,
    steps: u32,
    store: BTreeMap<String, u32>,
    forced_checked: BTreeSet<(usize, String)>,
    food_uses: BTreeMap<u64, u32>,
    provision_party: Vec<u64>,
    overrides: BTreeMap<String, Spawn>,
    spawn_epoch: u64,
    chosen: BTreeMap<String, usize>,
    limits: BTreeMap<String, u32>,
    hunger_buffer: u32,
    last_room: String,
    encounter_from: String,
    examined: BTreeSet<String>,
    encounter_known: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Spawn {
    kind: String,
    entry: Option<usize>,
}
impl Spawn {
    fn content(&self) -> dd_data::maps014::Content {
        use dd_data::maps014::Content;
        match self.kind.as_str() {
            "ac_battle" => Content {
                code: Some(1),
                battle: Some(format!("return:{}", self.entry.unwrap_or(usize::MAX))),
                ..Default::default()
            },
            "ac_hunger" => Content {
                code: Some(8),
                hunger: true,
                ..Default::default()
            },
            "ac_trap" => Content {
                code: Some(4),
                trap: Some("index:0".into()),
                ..Default::default()
            },
            _ => Content::default(),
        }
    }
}
#[derive(Clone)]
pub struct Session {
    definition: Definition,
    state: State,
    active_map: crate::maps014::Definition,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    Move(String),
    Unlock(String),
    Camp(campaign::CampAction),
    Backstep(String),
    Interact { hero: u64, item: Option<String> },
    Use { hero: u64, slot: usize },
    Discard { slot: usize, amount: u32 },
    Take { index: usize, amount: u32 },
    LeaveLoot,
    Obstacle { shovel: bool },
    Hunger { eat: bool },
    Disarm { hero: u64 },
    Fight(crate::Action),
    Leave { complete: bool },
    Buy { item: Item, amount: u32 },
    Embark,
    Reorder(Vec<u64>),
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleHero {
    pub guid: u64,
    pub name: String,
    pub class: String,
    pub alive: bool,
    pub hp: i32,
    pub maximum_hp: i32,
    pub stress: i32,
    pub resolve: Resolve,
    pub status: dd_data::hero010::CarryStatus,
    pub quirks: Vec<String>,
    pub skills: Vec<String>,
    pub xp: u32,
    pub food_remaining: u32,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Observation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub direct_rewards: Vec<Stack>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub item_states: Vec<campaign::ItemView>,
    pub profile: String,
    pub content_id: String,
    pub phase: Phase,
    pub heroes: Vec<VisibleHero>,
    pub party: Vec<u64>,
    pub inventory: Bag,
    pub wallet: BTreeMap<String, u32>,
    pub map: Option<crate::maps014::Observation>,
    pub battle: Option<crate::Observation>,
    pub loot: Vec<Stack>,
    pub torch: u8,
    pub quest_ready: bool,
    pub actions: Vec<Action>,
    pub results: Vec<Value>,
    pub stack_limits: BTreeMap<String, u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camping: Option<campaign::CampView>,
}
impl Session {
    pub fn new(definition: Definition, seed: u64) -> Result<Self> {
        let d = &definition.data;
        let torch = d.raid_source["data"]["party"]["torchlight"]
            .as_u64()
            .filter(|n| *n <= 100)
            .ok_or("invalid initial light")? as u8;
        let mut bag = Bag::new(&d.bag_rules);
        for stack in d.starting_stacks()? {
            if bag.insert(&d.bag_rules, &stack)? != stack.amount {
                return Err("starting inventory does not fit".into());
            }
        }
        let map = crate::maps014::Session::new(definition.maps[0].clone(), seed, BTreeMap::new())?
            .checkpoint()?;
        let next_guid = d.next_guid;
        let mut s = Self {
            state: State {
                campaign: None,
                profile: EXP_PROFILE.into(),
                definition_id: definition.id.clone(),
                phase: Phase::OldRoad,
                level: 0,
                heroes: vec![],
                party: vec![],
                bag,
                wallet: d.wallet.clone(),
                map,
                battle: None,
                battle_party: vec![],
                battle_start_supplies: BTreeMap::new(),
                loot: vec![],
                rolled: vec![],
                torch,
                rng: Rng(seed),
                next_guid,
                results: vec![],
                battles: 0,
                interactions: 0,
                steps: 0,
                store: BTreeMap::new(),
                forced_checked: BTreeSet::new(),
                food_uses: BTreeMap::new(),
                provision_party: vec![],
                overrides: BTreeMap::new(),
                spawn_epoch: 0,
                chosen: BTreeMap::new(),
                limits: BTreeMap::new(),
                hunger_buffer: d.raid_source["data"]["party"]["inventory"]["hunger_room_buffer"]
                    .as_u64()
                    .ok_or("missing hunger buffer")? as u32,
                last_room: d.levels[0].map.entrance.clone(),
                encounter_from: d.levels[0].map.entrance.clone(),
                examined: BTreeSet::new(),
                encounter_known: false,
            },
            active_map: definition.maps[0].clone(),
            definition,
        };
        let seeds = s.definition.data.initial.clone();
        for h in seeds {
            let kernel = PartyKernel::new(
                vec![h.blueprint.clone()],
                s.field_rules()?,
                s.state.rng.clone(),
            )?;
            let (mut states, rng) = kernel.export();
            s.state.rng = rng;
            let state = states.remove(0);
            s.state.party.push(h.guid);
            let mut base = h.blueprint;
            base.mechanics.hero.carry = None;
            base.mechanics.hero.initial_hp = None;
            s.state.heroes.push(Hero {
                guid: h.guid,
                name: h.name,
                xp: h.xp,
                base,
                state,
            });
        }
        s.state.provision_party = s.state.party.clone();
        s.validate()?;
        Ok(s)
    }
    pub fn definitions(&self) -> Definition {
        self.definition.clone()
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(&self.state).map_err(|e| e.to_string())
    }
    pub fn restore(json: &str, definition: &Definition) -> Result<Self> {
        if json.len() > 64 * 1024 * 1024 {
            return Err("opening checkpoint exceeds bound".into());
        }
        let state: State = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if state.profile != definition.profile() || state.definition_id != definition.id {
            return Err("opening checkpoint profile/content/source/candidate mismatch".into());
        }
        let active_map = Self::map_definition(definition, &state)?;
        let s = Self {
            state,
            definition: definition.clone(),
            active_map,
        };
        s.validate()?;
        Ok(s)
    }
    fn map(&self) -> Result<crate::maps014::Session> {
        crate::maps014::Session::restore(&self.state.map, &self.active_map)
    }
    fn map_definition(
        definition: &Definition,
        state: &State,
    ) -> Result<crate::maps014::Definition> {
        let mut map = definition
            .data
            .levels
            .get(state.level)
            .ok_or("unknown opening level")?
            .map
            .clone();
        if state.spawn_epoch == 0 {
            if !state.overrides.is_empty() {
                return Err("runtime content lacks epoch".into());
            }
            return Ok(definition.maps[state.level].clone());
        }
        for (id, event) in &state.overrides {
            if !["ac_battle", "ac_hunger", "ac_trap"].contains(&event.kind.as_str()) {
                return Err("unknown runtime content kind".into());
            }
            let n = map
                .nodes
                .get_mut(id)
                .ok_or("unknown runtime content node")?;
            if n.room || n.hidden {
                return Err("runtime content cannot replace an opening room".into());
            }
            if event.kind == "ac_battle" {
                let e = definition.data.levels[state.level]
                    .pools
                    .entries
                    .get(event.entry.ok_or("missing pool entry")?)
                    .ok_or("invalid pool entry")?;
                if !e.eligible_in_opening {
                    return Err("ineligible opening content pool".into());
                }
            } else if event.entry.is_some() {
                return Err("nonbattle content has a formation".into());
            }
            n.content = event.content();
        }
        map.raw["runtime015"] = json!({"epoch":state.spawn_epoch});
        crate::maps014::Definition::new(
            map,
            definition.data.content.clone(),
            MapCandidates::for_content(&definition.data.content)?,
        )
    }
    fn inventory_rules(&self) -> Result<Arc<dd_data::inventory015::InventoryRules>> {
        let mut classes = self
            .state
            .provision_party
            .iter()
            .map(|id| Ok(self.state.heroes[self.hero_index(*id)?].base.id.clone()))
            .collect::<Result<Vec<_>>>()?;
        classes.sort();
        if let Some(r) = self
            .definition
            .cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .inventories
            .get(&classes)
            .cloned()
        {
            return Ok(r);
        }
        let mut rules = self.definition.data.bag_rules.clone();
        for class in &classes {
            let extra = self
                .definition
                .data
                .class_extras
                .get(class)
                .ok_or("missing class inventory extras")?;
            for (key, n) in &extra.limits {
                let v = rules.extra_limits.entry(key.clone()).or_default();
                *v = v.checked_add(*n).ok_or("extra stack overflow")?;
            }
        }
        let rules = Arc::new(rules);
        self.definition
            .cache
            .lock()
            .map_err(|_| "opening cache poisoned")?
            .inventories
            .insert(classes, rules.clone());
        Ok(rules)
    }
    fn extra_loot(&self, hero: Option<u64>) -> Result<Vec<dd_data::curios013::Draw>> {
        let mut draws = vec![];
        for guid in &self.state.party {
            if hero.is_some_and(|id| id != *guid) {
                continue;
            }
            let class = &self.state.heroes[self.hero_index(*guid)?].base.id;
            let extra = self
                .definition
                .data
                .class_extras
                .get(class)
                .ok_or("missing class loot extras")?;
            draws.extend(
                if hero.is_some() {
                    &extra.curio
                } else {
                    &extra.battle
                }
                .iter()
                .cloned(),
            );
        }
        Ok(draws)
    }
    fn field_rules(&self) -> Result<dd_data::combat::Rules> {
        let mut r = self.definition.field_rules[self.state.level].clone();
        r.lighting.as_mut().ok_or("missing field lighting")?.torch = self.state.torch;
        if let Some(c) = r.context.as_mut() {
            c.corridor = !self.active_map.map().nodes[self.map()?.position()].room;
        }
        Ok(r)
    }
    fn quest(&self) -> &Value {
        let q = &self.definition.data.levels[self.state.level].quest;
        if self.state.level == 0 {
            q
        } else {
            &q["quest"]
        }
    }
    fn hero_index(&self, guid: u64) -> Result<usize> {
        self.state
            .heroes
            .iter()
            .position(|h| h.guid == guid)
            .ok_or("unknown hero identity".into())
    }
    fn carry(&self, h: &Hero) -> Result<Blueprint> {
        if h.base.mechanics.build.input != h.state.build {
            let mut b = h.state.carry_into(&self.definition.game, h.base.clone())?;
            self.project_camp_buffs(h.guid, &mut b)?;
            return Ok(b);
        }
        let mut b = h.base.clone();
        b.mechanics.hero.difficulty = h
            .state
            .difficulty
            .clone()
            .or(b.mechanics.hero.difficulty.clone())
            .map(|mut d| {
                d.entering_raid = false;
                d
            });
        b.mechanics.hero.resolve_history = h.state.resolve_history.clone();
        b.mechanics.hero.dungeon_history = h.state.dungeon_history.clone();
        b.mechanics.hero.initial_hp = Some(h.state.hp);
        b.mechanics.hero.initial_mode = h.state.mode.clone();
        b.mechanics.hero.carry = Some(dd_data::hero010::HeroCarry {
            status: h.state.status.clone(),
            stress: h.state.stress,
            resolve: match &h.state.resolve {
                Resolve::Calm => dd_data::hero010::CarryResolve::Calm,
                Resolve::Afflicted(id) => dd_data::hero010::CarryResolve::Afflicted(id.clone()),
                Resolve::Virtuous(id) => dd_data::hero010::CarryResolve::Virtuous(id.clone()),
            },
            buffs: h.state.persistent_buffs.clone(),
        });
        self.project_camp_buffs(h.guid, &mut b)?;
        Ok(b)
    }
    fn party_blueprints(&self) -> Result<Vec<Blueprint>> {
        self.state
            .party
            .iter()
            .map(|id| {
                let h = &self.state.heroes[self.hero_index(*id)?];
                self.carry(h)
            })
            .collect()
    }
    fn encounter_heroes(&self) -> Result<Vec<Blueprint>> {
        let mut heroes = self.party_blueprints()?;
        for b in &mut heroes {
            if let Some(c) = b.mechanics.hero.carry.as_mut() {
                let buffs = std::mem::take(&mut c.buffs);
                for buff in buffs {
                    if buff.definition.duration_type.as_deref() != Some("quest_end")
                        && buff.definition.activation.remove_on_battle_complete
                    {
                        b.mechanics.hero.exploration_buffs.push(buff);
                    } else {
                        c.buffs.push(buff);
                    }
                }
            }
        }
        Ok(heroes)
    }
    fn kernel(&self) -> Result<PartyKernel> {
        self.kernel_with_hp_policy(false)
    }
    fn kernel_with_hp_policy(&self, clamp_existing_hp: bool) -> Result<PartyKernel> {
        let mut k = if clamp_existing_hp {
            PartyKernel::after_host_buff_change(
                self.party_blueprints()?,
                self.field_rules()?,
                self.state.rng.clone(),
            )?
        } else {
            PartyKernel::new(
                self.party_blueprints()?,
                self.field_rules()?,
                self.state.rng.clone(),
            )?
        };
        k.exploration_buffs(&self.definition.game.buffs)?;
        self.camp_field_buffs(&mut k)?;
        self.campaign_field_bonuses(&mut k)?;
        if self.camp_active() {
            k.camping_buffs(&self.definition.game.buffs)?;
        }
        Ok(k)
    }
    fn food_required(&self) -> Result<u32> {
        let k = self.kernel()?;
        let mut n = 0;
        for id in 0..self.state.party.len() {
            n += (1. + k.modifier(id, "food_consumption_percent", ""))
                .max(0.)
                .ceil() as u32;
        }
        Ok(n)
    }
    fn field(&mut self, run: impl FnOnce(&mut PartyKernel) -> Result<()>) -> Result<()> {
        self.field_with_hp_policy(run, false)
    }
    fn field_with_hp_policy(
        &mut self,
        run: impl FnOnce(&mut PartyKernel) -> Result<()>,
        clamp_existing_hp: bool,
    ) -> Result<()> {
        if self.state.party.is_empty() {
            return Ok(());
        }
        let mut k = self.kernel_with_hp_policy(clamp_existing_hp)?;
        run(&mut k)?;
        let door = k
            .core
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.entered_deaths_door)
            .map(|(i, _)| self.state.party[i])
            .collect();
        self.campaign_record_door(door);
        let field_order = k.core.order[0]
            .iter()
            .filter(|id| k.core.units[**id].alive())
            .map(|id| self.state.party[*id])
            .collect::<Vec<_>>();
        let (states, rng) = k.export();
        self.state.rng = rng;
        let party = self.state.party.clone();
        self.reconcile_camp_buffs(&states, &party)?;
        for (id, state) in party.iter().zip(states) {
            let idx = self.hero_index(*id)?;
            self.state.heroes[idx].state = state;
        }
        self.state.party.retain(|id| {
            self.state
                .heroes
                .iter()
                .any(|h| h.guid == *id && h.state.alive)
        });
        if self.definition.campaign.is_some() {
            self.state.party = field_order;
        }
        Ok(())
    }
    fn validate(&self) -> Result<()> {
        let s = &self.state;
        let d = &self.definition.data;
        if s.profile != self.definition.profile()
            || s.definition_id != self.definition.id
            || s.level >= d.levels.len()
            || s.heroes.len() > 32
            || s.party.len() > 4
            || s.steps > 100000
            || s.results.len() > 2
        {
            return Err("invalid opening stage state".into());
        }
        self.validate_campaign()?;
        s.bag.validate(&*self.inventory_rules()?)?;
        if s.provision_party.len() > 4
            || s.provision_party.iter().collect::<BTreeSet<_>>().len() != s.provision_party.len()
            || s.provision_party
                .iter()
                .any(|id| !s.heroes.iter().any(|h| h.guid == *id))
        {
            return Err("invalid provisioned party".into());
        }
        let mut ids = BTreeSet::new();
        for h in &s.heroes {
            self.definition.validate_base(&h.base)?;
            h.state.status.validate()?;
            if !ids.insert(h.guid)
                || h.guid >= s.next_guid
                || h.base.id != h.state.class
                || !h.base.hero
                || h.state.hp < 0
                || h.state.hp > h.state.max_hp
                || h.state.stress < 0
                || h.state.stress > 200
            {
                return Err("invalid opening hero state".into());
            }
        }
        let mut party = BTreeSet::new();
        for id in &s.party {
            let h = &s.heroes[self.hero_index(*id)?];
            if !party.insert(id) || !h.state.alive {
                return Err("dead/duplicate opening party hero".into());
            }
        }
        for (id, n) in &s.food_uses {
            if !ids.contains(id) || *n > self.definition.data.candidates.manual_food_limit {
                return Err("invalid food satiation state".into());
            }
        }
        if s.spawn_epoch > 100000
            || s.hunger_buffer > 128
            || s.examined
                .iter()
                .any(|id| !self.active_map.map().nodes.contains_key(id))
            || s.chosen.iter().any(|(node, i)| {
                !self.active_map.map().nodes.contains_key(node)
                    || !d.levels[s.level]
                        .pools
                        .entries
                        .get(*i)
                        .is_some_and(|e| e.eligible_in_opening)
            })
        {
            return Err("invalid runtime encounter state".into());
        }
        for v in &s.loot {
            d.bag_rules.definition(&v.item)?;
            if v.amount == 0 || v.amount > 1000000 {
                return Err("invalid loot tray".into());
            }
        }
        let m = self.map()?;
        if s.battle.is_some()
            && !s.campaign.as_ref().is_some_and(|p| p.auxiliary.is_some())
            && m.pending()
                .is_none_or(|p| p.kind != crate::maps014::EventKind::Encounter)
        {
            return Err("battle lacks matching map encounter".into());
        }
        if let Some(saved) = &s.battle {
            let mut b = BattleSession::restore(saved, &d.content, &d.rules.candidates)?;
            if !matches!(b.advance()?.decision, Decision::Player(_)) {
                return Err("opening battle checkpoint is not a player boundary".into());
            }
            if s.battle_party.len() > 4 || s.battle_party.iter().any(|id| !ids.contains(id)) {
                return Err("invalid battle identity mapping".into());
            }
        }
        if (s.phase == Phase::Preparation && (s.level != 0 || s.results.len() != 1))
            || (s.phase == Phase::OldRoad && s.level != 0)
            || (s.phase == Phase::FirstRuins && s.level != 1)
        {
            return Err("inconsistent opening phase".into());
        }
        Ok(())
    }
    fn transaction(&mut self, run: impl FnOnce(&mut Self) -> Result<()>) -> Result<()> {
        let mut n = self.clone();
        run(&mut n)?;
        n.validate()?;
        *self = n;
        Ok(())
    }
    fn weighted(&mut self, w: &[f64]) -> Result<usize> {
        let total = w.iter().sum::<f64>();
        if w.iter().any(|v| !v.is_finite() || *v < 0.) || !total.is_finite() || total <= 0. {
            return Err("invalid opening weights".into());
        }
        let mut roll = (self.state.rng.next() >> 11) as f64 / ((1u64 << 53) as f64) * total;
        for (i, v) in w.iter().enumerate() {
            roll -= v;
            if roll < 0. {
                return Ok(i);
            }
        }
        Ok(w.iter().rposition(|v| *v > 0.).unwrap())
    }
    pub fn ready(&self) -> Result<bool> {
        if self.definition.campaign.is_some() {
            return self.campaign_ready();
        }
        let m = self.map()?;
        if m.pending().is_some() {
            return Ok(false);
        }
        let level = &self.definition.data.levels[self.state.level];
        match level.goal["type"].as_str() {
            Some("tutorial_room") => {
                let room = level.goal["data"]["room_id"]
                    .as_str()
                    .ok_or("missing goal room")?;
                let id = format!("{room}:0");
                Ok(m.visited().contains(&id)
                    && !m
                        .observe()
                        .nodes
                        .iter()
                        .any(|n| n.id == id && n.content.as_ref().is_some_and(|c| c.battle)))
            }
            Some("explore_room") => {
                let rooms = level
                    .map
                    .nodes
                    .values()
                    .filter(|n| n.room && !n.hidden)
                    .collect::<Vec<_>>();
                let visited = rooms.iter().filter(|n| m.visited().contains(&n.id)).count();
                let p = level.goal["data"]["percentage"]
                    .as_f64()
                    .ok_or("missing exploration ratio")?;
                Ok(visited as f64 >= rooms.len() as f64 * p)
            }
            _ => Err("unsupported opening quest goal".into()),
        }
    }
    pub fn observe(&self) -> Result<Observation> {
        let m = self.map()?;
        let mut map_observation = m.observe();
        for n in &mut map_observation.nodes {
            if let Some(c) = &mut n.content {
                c.hunger = false;
            }
        }
        self.campaign_filter_map(&mut map_observation)?;
        let mut battle = None;
        if let Some(saved) = &self.state.battle {
            let mut b = BattleSession::restore(
                saved,
                &self.definition.data.content,
                &self.definition.data.rules.candidates,
            )?;
            if let Decision::Player(o) = b.advance()?.decision {
                battle = Some(o);
            } else {
                return Err("unsettled battle observation".into());
            }
        }
        let mut actions = vec![];
        if self.state.phase != Phase::Finished {
            for (slot, s) in self.state.bag.slots.iter().enumerate() {
                if let Some(s) = s
                    && self.state.battle.is_none()
                {
                    actions.push(Action::Discard {
                        slot,
                        amount: s.amount,
                    });
                }
            }
            if !self.state.loot.is_empty() {
                for (i, s) in self.state.loot.iter().enumerate() {
                    let mut trial = self.state.bag.clone();
                    let accepted = trial.insert(&*self.inventory_rules()?, s)?;
                    if accepted > 0 {
                        actions.push(Action::Take {
                            index: i,
                            amount: accepted,
                        });
                    }
                }
                actions.push(Action::LeaveLoot);
            } else if self.state.phase == Phase::Preparation {
                for (key, n) in &self.state.store {
                    if *n > 0 {
                        let (k, id) = key.split_once(':').ok_or("bad store key")?;
                        let item = Item::new(k, id);
                        let price = self.inventory_rules()?.definition(&item)?.purchase;
                        let mut trial = self.state.bag.clone();
                        if self.state.wallet.get("gold:").copied().unwrap_or(0) >= price
                            && trial.insert(
                                &*self.inventory_rules()?,
                                &Stack {
                                    item: item.clone(),
                                    amount: 1,
                                },
                            )? == 1
                        {
                            actions.push(Action::Buy { item, amount: 1 });
                        }
                    }
                }
                if self.state.party.len() == 4 {
                    actions.push(Action::Embark);
                }
            } else if let Some(camp_actions) = self.camp_actions()? {
                actions.extend(camp_actions.into_iter().map(Action::Camp));
            } else if let Some(b) = &battle {
                actions.extend(
                    b.actions
                        .iter()
                        .filter(|a| {
                            self.state.level > 0
                                || self.definition.campaign.is_some()
                                || a.action != crate::Action::Retreat
                        })
                        .map(|a| Action::Fight(a.action.clone())),
                );
            } else {
                match m.pending().map(|p| p.kind) {
                    Some(crate::maps014::EventKind::Encounter) => {}
                    Some(crate::maps014::EventKind::Hunger) => {
                        actions.push(Action::Hunger { eat: false });
                        if self.state.bag.count(&Item::new("provision", ""))
                            >= self.food_required()?
                        {
                            actions.push(Action::Hunger { eat: true });
                        }
                    }
                    Some(crate::maps014::EventKind::Obstacle) => {
                        actions.push(Action::Obstacle { shovel: false });
                        let item = if let Some(spec) = &self.definition.campaign {
                            match spec
                                .sources
                                .obstacles
                                .get(m.position())
                                .ok_or("unbound obstacle action")?
                                .interaction
                                .as_str()
                            {
                                "torch_obstacle" => "torch",
                                "shovel_obstacle" => "shovel",
                                _ => "",
                            }
                        } else {
                            "shovel"
                        };
                        if !item.is_empty() && self.state.bag.count(&Item::new("supply", item)) > 0
                        {
                            actions.push(Action::Obstacle { shovel: true });
                        }
                    }
                    Some(crate::maps014::EventKind::Trap) => {
                        actions.extend(
                            self.state
                                .party
                                .iter()
                                .map(|id| Action::Disarm { hero: *id }),
                        );
                    }
                    Some(crate::maps014::EventKind::Curio) => {
                        return Err("external curio receipt leaked into opening".into());
                    }
                    None => {
                        for a in m.legal_actions() {
                            match a {
                                crate::maps014::Action::Move(to) => {
                                    actions.push(Action::Move(to.clone()));
                                    if !self.active_map.map().nodes[m.position()].room
                                        && m.visited().contains(&to)
                                        && self
                                            .active_map
                                            .map()
                                            .distances(&self.state.last_room, false)
                                            .get(&to)
                                            < self
                                                .active_map
                                                .map()
                                                .distances(&self.state.last_room, false)
                                                .get(m.position())
                                    {
                                        actions.push(Action::Backstep(to));
                                    }
                                }
                                crate::maps014::Action::Interact => {
                                    let full = self.curio_accepts_items(m.position())?;
                                    for hero in &self.state.party {
                                        if self.campaign_curio_can_interact(m.position(), None)? {
                                            actions.push(Action::Interact {
                                                hero: *hero,
                                                item: None,
                                            });
                                        }
                                        for s in self
                                            .state
                                            .bag
                                            .slots
                                            .iter()
                                            .flatten()
                                            .filter(|s| full && s.item.kind == "supply")
                                        {
                                            if self.campaign_curio_can_interact(
                                                m.position(),
                                                Some(&s.item.id),
                                            )? {
                                                actions.push(Action::Interact {
                                                    hero: *hero,
                                                    item: Some(s.item.id.clone()),
                                                });
                                            }
                                        }
                                    }
                                }
                                crate::maps014::Action::Unlock(id) => {
                                    if self.definition.campaign.is_some() {
                                        actions.push(Action::Unlock(id));
                                    } else {
                                        return Err("unsupported lock in opening".into());
                                    }
                                }
                            }
                        }
                        let ready = self.ready()?;
                        if ready {
                            actions.push(Action::Leave { complete: true });
                        }
                        if self.state.level > 0 || self.definition.campaign.is_some() {
                            actions.push(Action::Leave { complete: false });
                        }
                    }
                }
                if m.pending().is_none() {
                    for (slot, s) in self.state.bag.slots.iter().enumerate() {
                        if let Some(s) = s
                            && ([
                                "torch",
                                "bandage",
                                "antivenom",
                                "medicinal_herbs",
                                "holy_water",
                                "laudanum",
                                "the_blood",
                            ]
                            .contains(&s.item.id.as_str())
                                && (s.item.kind == "supply"
                                    || (s.item.kind == "estate" && s.item.id == "the_blood"))
                                || s.item.kind == "provision")
                        {
                            for id in &self.state.party {
                                let h = &self.state.heroes[self.hero_index(*id)?];
                                if s.item.kind != "provision"
                                    || h.state.hp < h.state.max_hp
                                        && self.state.food_uses.get(id).copied().unwrap_or(0)
                                            < self.definition.data.candidates.manual_food_limit
                                {
                                    actions.push(Action::Use { hero: *id, slot });
                                }
                            }
                        }
                    }
                }
            }
        }
        if self.state.phase != Phase::Finished
            && self.state.battle.is_none()
            && self.state.loot.is_empty()
            && m.pending().is_none()
            && self.camp_view()?.is_none()
        {
            fn permutations(v: &mut Vec<u64>, i: usize, out: &mut Vec<Action>) {
                if i == v.len() {
                    out.push(Action::Reorder(v.clone()));
                } else {
                    for j in i..v.len() {
                        v.swap(i, j);
                        permutations(v, i + 1, out);
                        v.swap(i, j);
                    }
                }
            }
            let mut v = self.state.party.clone();
            permutations(&mut v, 0, &mut actions);
        }
        actions.extend(self.camp_idle_actions()?.into_iter().map(Action::Camp));
        let heroes = self
            .state
            .heroes
            .iter()
            .map(|h| VisibleHero {
                guid: h.guid,
                name: h.name.clone(),
                class: h.state.class.clone(),
                alive: h.state.alive,
                hp: h.state.hp,
                maximum_hp: h.state.max_hp,
                stress: h.state.stress,
                resolve: h.state.resolve.clone(),
                status: h.state.status.clone(),
                quirks: h.state.build.quirks.clone(),
                skills: h.base.skills.iter().map(|s| s.id.clone()).collect(),
                xp: h.xp,
                food_remaining: self
                    .definition
                    .data
                    .candidates
                    .manual_food_limit
                    .saturating_sub(self.state.food_uses.get(&h.guid).copied().unwrap_or(0)),
            })
            .collect();
        if self
            .definition
            .campaign
            .as_ref()
            .and_then(|s| s.context.automation.as_ref())
            .and_then(|a| a.contract.as_ref())
            .is_some_and(|c| !c.can_retreat)
        {
            actions.retain(|a| !matches!(a, Action::Leave { complete: false }));
        }
        Ok(Observation{direct_rewards:self.campaign_direct_rewards(),item_states:self.campaign_item_view(),camping:self.camp_view()?,profile:self.definition.profile().into(),content_id:self.definition.data.content.id()?,phase:self.state.phase,heroes,party:self.state.party.clone(),inventory:self.state.bag.clone(),wallet:self.state.wallet.clone(),map:if matches!(self.state.phase,Phase::OldRoad|Phase::FirstRuins|Phase::Expedition){Some(map_observation)}else{None},battle,loot:self.state.loot.clone(),torch:self.state.torch,quest_ready:self.ready()?,actions,results:self.state.results.iter().map(|r|json!({"level":r.level,"completed":r.completed,"outcome":r.outcome,"brought_home":r.brought_home,"rewards":r.rewards,"xp":r.xp})).collect(),stack_limits:self.state.bag.slots.iter().flatten().chain(self.state.loot.iter()).map(|s|Ok((s.item.key(),self.inventory_rules()?.definition(&s.item)?.limit))).collect::<Result<BTreeMap<_,_>>>()?})
    }
    pub fn act(&mut self, a: Action) -> Result<()> {
        self.transaction(|s| s.act_inner(a))
    }
    fn act_inner(&mut self, a: Action) -> Result<()> {
        // Amount-bearing inventory actions validate their exact requested quantity below.
        let listed = match &a {
            Action::Take { index, amount } => {
                *amount > 0
                    && self
                        .state
                        .loot
                        .get(*index)
                        .is_some_and(|v| *amount <= v.amount)
            }
            Action::Discard { slot, amount } => {
                *amount > 0
                    && self
                        .state
                        .bag
                        .slots
                        .get(*slot)
                        .and_then(Option::as_ref)
                        .is_some_and(|v| *amount <= v.amount)
                    && self.state.phase != Phase::Finished
                    && self.state.battle.is_none()
            }
            Action::Buy { item, amount } => {
                *amount > 0
                    && self.state.phase == Phase::Preparation
                    && self
                        .state
                        .store
                        .get(&item.key())
                        .is_some_and(|n| *amount <= *n)
            }
            Action::Reorder(ids) => {
                self.state.battle.is_none()
                    && self.state.loot.is_empty()
                    && self.map()?.pending().is_none()
                    && ids.len() == self.state.party.len()
                    && ids.iter().collect::<BTreeSet<_>>() == self.state.party.iter().collect()
                    && self.state.phase != Phase::Finished
                    && self.camp_view()?.is_none()
            }
            _ => self.observe()?.actions.contains(&a),
        };
        if !listed {
            return Err("illegal opening action".into());
        }
        match a {
            Action::Move(to) => self.move_to(&to, false)?,
            Action::Unlock(id) => self.campaign_unlock(&id)?,
            Action::Camp(action) => self.camp_act(action)?,
            Action::Backstep(to) => self.move_to(&to, true)?,
            Action::Interact { hero, item } => self.interact(hero, item)?,
            Action::Use { hero, slot } => self.use_item(hero, slot)?,
            Action::Discard { slot, amount } => {
                self.state.bag.discard(slot, amount)?;
                self.campaign_item_discard(slot);
            }
            Action::Take { index, amount } => {
                let mut stack = self.state.loot[index].clone();
                stack.amount = amount;
                let before = self.state.bag.clone();
                let n = self.state.bag.insert(&*self.inventory_rules()?, &stack)?;
                if n == 0 {
                    return Err("inventory has no space".into());
                }
                let removed = self.state.loot[index].amount == n;
                self.campaign_item_pickup(index, n, &before, removed)?;
                self.state.loot[index].amount -= n;
                if self.state.loot[index].amount == 0 {
                    self.state.loot.remove(index);
                }
            }
            Action::LeaveLoot => {
                self.state.loot.clear();
                self.campaign_item_clear_tray();
            }
            Action::Obstacle { shovel } => self.obstacle(shovel)?,
            Action::Hunger { eat } => self.hunger(eat)?,
            Action::Disarm { hero } => self.trap(hero)?,
            Action::Fight(a) => self.fight(a)?,
            Action::Leave { complete } => self.finish(complete)?,
            Action::Buy { item, amount } => {
                let price = self
                    .definition
                    .data
                    .bag_rules
                    .definition(&item)?
                    .purchase
                    .checked_mul(amount)
                    .ok_or("purchase overflow")?;
                let gold = self.state.wallet.get("gold:").copied().unwrap_or(0);
                if price > gold {
                    return Err("insufficient gold".into());
                }
                if self.state.bag.insert(
                    &*self.inventory_rules()?,
                    &Stack {
                        item: item.clone(),
                        amount,
                    },
                )? != amount
                {
                    return Err("purchase does not fit inventory".into());
                }
                self.state.wallet.insert("gold:".into(), gold - price);
                *self.state.store.get_mut(&item.key()).unwrap() -= amount;
            }
            Action::Embark => self.embark()?,
            Action::Reorder(ids) => self.state.party = ids,
        }
        if self.definition.campaign.is_some() {
            self.campaign_reveal_layout()?;
            self.campaign_skip_hunger()?;
        }
        if self.state.battle.is_none()
            && self.state.loot.is_empty()
            && self.camp_view()?.is_none()
            && matches!(
                self.state.phase,
                Phase::OldRoad | Phase::FirstRuins | Phase::Expedition
            )
            && self.map()?.pending().is_none()
            && !self.state.party.is_empty()
        {
            self.forced_curio()?;
        }
        if self.state.party.is_empty()
            && matches!(
                self.state.phase,
                Phase::OldRoad | Phase::FirstRuins | Phase::Expedition
            )
        {
            self.state.battle = None;
            self.finish(false)?;
        }
        self.campaign_sync_keys()?;
        Ok(())
    }

    fn move_to(&mut self, to: &str, backward: bool) -> Result<()> {
        let mut m = self.map()?;
        let from = m.position().to_string();
        let target = self
            .active_map
            .map()
            .nodes
            .get(to)
            .ok_or("unknown map destination")?;
        let entering_corridor = self.active_map.map().nodes[&from].room && !target.room;
        if entering_corridor {
            self.state.last_room = from.clone();
            let area = self.active_map.map().nodes[to].area.clone();
            if self.definition.campaign.is_none()
                && m.visited()
                    .iter()
                    .any(|id| self.active_map.map().nodes[id].area == area)
            {
                self.respawn(&mut m, &area)?;
            }
            if self.state.hunger_buffer > 0 {
                self.state.hunger_buffer -= 1;
            }
        }
        self.state.encounter_from = from.clone();
        let seen = m.visited().contains(to);
        self.state.encounter_known = m
            .observe()
            .nodes
            .iter()
            .find(|n| n.id == to)
            .and_then(|n| n.content.as_ref())
            .is_some_and(|c| c.battle);
        let known_trap = m
            .observe()
            .nodes
            .iter()
            .find(|n| n.id == to)
            .and_then(|n| n.content.as_ref())
            .is_some_and(|c| c.trap);
        if !m
            .legal_actions()
            .contains(&crate::maps014::Action::Move(to.into()))
        {
            return Err("illegal map transition".into());
        }
        let cost = if self.definition.field_rules[self.state.level]
            .bloodlight
            .is_some()
            || self
                .definition
                .campaign
                .as_ref()
                .and_then(|s| s.context.automation.as_ref())
                .is_some_and(|a| a.no_torch_burn)
        {
            0
        } else if seen {
            self.definition.data.candidates.light_seen
        } else {
            self.definition.data.candidates.light_new[&self.definition.data.content.mode]
        };
        self.state.torch = self.state.torch.saturating_sub(cost);
        let hs = self.definition.data.globals["hallway_stress"].clone();
        let chance = hs[if backward {
            "hallway_per_tile_stress_damage_chance_backing_up"
        } else {
            "hallway_per_tile_stress_damage_chance_fwd"
        }]
        .as_f64()
        .ok_or("missing hallway stress chance")?;
        let amount = hs[if backward {
            "hallway_per_tile_stress_damage_backing_up"
        } else {
            "hallway_per_tile_stress_damage_fwd"
        }]
        .as_f64()
        .ok_or("missing hallway stress amount")? as i32;
        let hallway = !self.active_map.map().nodes[to].room;
        let single_recipient = self
            .definition
            .campaign
            .as_ref()
            .is_some_and(|c| c.sources.quest["dungeon"] == "courtyard");
        self.field(|k| {
            if let Some(c) = k.core.scenario.rules.context.as_mut() {
                c.corridor = hallway;
                c.walking_backwards = backward;
            }
            k.core.refresh_context(false);
            k.walk_dot()?;
            if hallway && single_recipient {
                if k.core.rng.roll(chance) {
                    let alive = k
                        .core
                        .units
                        .iter()
                        .enumerate()
                        .filter(|(_, u)| u.alive())
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>();
                    if !alive.is_empty() {
                        let i = k.core.rng.below(alive.len() as u64) as usize;
                        k.stress(alive[i], amount)?;
                    }
                }
            } else if hallway {
                for id in 0..k.core.units.len() {
                    if k.core.rng.roll(chance) {
                        k.stress(id, amount)?;
                    }
                }
            }
            Ok(())
        })?;
        if self.state.party.is_empty() {
            return Ok(());
        }
        if self.active_map.map().nodes[to].content.hunger
            && !m.remaining(to, crate::maps014::EventKind::Hunger)
            && self.state.hunger_buffer == 0
        {
            m.rearm_hunger(to)?;
        }
        m.act(crate::maps014::Action::Move(to.into()))?;
        self.state.steps += 1;
        self.state.map = m.checkpoint()?;
        if let Some(p) = m.pending().cloned() {
            match p.kind {
                crate::maps014::EventKind::Encounter => self.start_battle()?,
                crate::maps014::EventKind::Hunger if self.state.hunger_buffer > 0 => {
                    self.map_settle()?;
                }
                crate::maps014::EventKind::Trap if !known_trap => {
                    let i = self.state.rng.below(self.state.party.len() as u64) as usize;
                    let id = self.state.party[i];
                    self.trigger_trap(id, false)?;
                }
                _ => {}
            }
        }
        if self.state.battle.is_none()
            && self.map()?.pending().is_none()
            && self.active_map.map().nodes[to].room
            && (self.state.level > 0 || self.definition.campaign.is_some())
            && !seen
        {
            self.scout()?;
        }
        if self.state.battle.is_none() && self.map()?.pending().is_none() {
            self.forced_curio()?;
        }
        let _ = from;
        Ok(())
    }
    fn scout(&mut self) -> Result<()> {
        let mut m = self.map()?;
        if self.definition.data.levels[self.state.level].quest["is_scouting_enabled"] != true {
            return Ok(());
        }
        let base = self.definition.data.globals["scouting_chance_base"]
            .as_f64()
            .ok_or("missing scouting rule")?;
        let light = self
            .field_rules()?
            .lighting
            .as_ref()
            .ok_or("missing scout light")?
            .at(self.state.torch)?
            .player_scouting_increase
            / 100.;
        let kernel = self.kernel()?;
        let bonus = (0..self.state.party.len())
            .map(|id| kernel.modifier(id, "scouting_chance", ""))
            .sum::<f64>();
        if !self.state.rng.roll((base + light + bonus).clamp(0., 1.)) {
            return Ok(());
        }
        let crit = self.definition.data.globals["scouting_crit_success"]
            .as_f64()
            .ok_or("missing critical scouting")?;
        let hops = if self.state.rng.roll(crit) { 2 } else { 1 };
        let graph = &self.definition.data.levels[self.state.level].map;
        let mut todo = VecDeque::from([(m.position().to_string(), 0)]);
        let mut seen = BTreeSet::new();
        let mut secret_tags = BTreeSet::new();
        while let Some((id, n)) = todo.pop_front() {
            if self.definition.campaign.is_some()
                && n + usize::from(graph.nodes[&id].room && id != m.position()) > hops
            {
                continue;
            }
            if !seen.insert(id.clone()) {
                continue;
            }
            let count = n + usize::from(graph.nodes[&id].room && id != m.position());
            if count > hops {
                continue;
            }
            for l in &graph.nodes[&id].links {
                if self.definition.campaign.is_some()
                    && count + usize::from(graph.nodes[&l.to].room && l.to != m.position()) > hops
                {
                    continue;
                }
                if (l.hidden.is_none() || self.definition.campaign.is_some() && hops == 2)
                    && !seen.contains(&l.to)
                {
                    if let Some(tag) = &l.hidden {
                        secret_tags.insert(tag.clone());
                    }
                    todo.push_back((l.to.clone(), count));
                }
            }
        }
        m.scout(&seen, &secret_tags)?;
        self.campaign_full_scout(&seen);
        self.state.map = m.checkpoint()?;
        Ok(())
    }
    fn map_settle(&mut self) -> Result<()> {
        let mut m = self.map()?;
        let p = m.pending().cloned().ok_or("no map event to settle")?;
        m.resume(&p, crate::maps014::Resolution::Settled)?;
        self.state.map = m.checkpoint()?;
        Ok(())
    }
    fn obstacle(&mut self, shovel: bool) -> Result<()> {
        if self.definition.campaign.is_some() {
            return self.campaign_obstacle(shovel);
        }
        let h = self.definition.data.levels[self.state.level]
            .obstacle
            .clone();
        if shovel {
            self.state.bag.consume(&Item::new("supply", "shovel"), 1)?;
        } else {
            self.state.torch = (i32::from(self.state.torch) + h.light).clamp(0, 100) as u8;
        }
        let effects = if shovel { h.success } else { h.failure };
        self.field(|k| {
            for id in 0..k.core.units.len() {
                if !shovel {
                    k.hp(id, (f64::from(k.maximum(id)) * h.health).round() as i32)?;
                }
                for e in &effects {
                    if !k.core.units[id].alive() {
                        break;
                    }
                    k.curio_effect(id, e)?;
                }
            }
            Ok(())
        })?;
        self.map_settle()
    }
    fn hunger(&mut self, eat: bool) -> Result<()> {
        let n = self.food_required()?;
        if eat && n > 0 {
            self.state.bag.consume(&Item::new("provision", ""), n)?;
        }
        let key = if eat {
            "hallway_hunger_HPrestore"
        } else {
            "hallway_hunger_starve_HPdmg"
        };
        let ratio = self.definition.data.globals[key]
            .as_f64()
            .ok_or("missing hunger HP rule")?;
        let stress = self.definition.data.candidates.hunger_stress;
        self.field(|k| {
            for id in 0..k.core.units.len() {
                let modifier = if eat {
                    k.modifier(id, "hp_heal_received_percent", "eat")
                } else {
                    k.modifier(id, "starving_damage_percent", "")
                };
                let amount =
                    (f64::from(k.maximum(id)) * ratio * (1. + modifier).max(0.)).round() as i32;
                k.hp(id, if eat { amount } else { -amount })?;
                if !eat {
                    let amount = (f64::from(stress)
                        * (1. + k.modifier(id, "stress_dmg_received_percent", "hunger")).max(0.))
                    .round() as i32;
                    k.stress(id, amount)?;
                }
            }
            Ok(())
        })?;
        self.state.hunger_buffer =
            self.definition.data.raid_source["data"]["party"]["inventory"]["hunger_room_buffer"]
                .as_u64()
                .ok_or("missing hunger room buffer")? as u32;
        self.map_settle()
    }
    fn trap(&mut self, hero: u64) -> Result<()> {
        self.trigger_trap(hero, true)
    }
    fn trigger_trap(&mut self, hero: u64, disarm: bool) -> Result<()> {
        if self.definition.campaign.is_some() {
            return self.campaign_trap(hero, disarm);
        }
        let m = self.map()?;
        let h = self.definition.data.levels[self.state.level]
            .traps
            .get(m.position())
            .unwrap_or(&self.definition.data.levels[self.state.level].default_trap)
            .clone();
        let id = self
            .state
            .party
            .iter()
            .position(|v| *v == hero)
            .ok_or("trap target is not a living party member")?;
        let b = &self.state.heroes[self.hero_index(hero)?].base;
        let d = self.definition.game.hero_document(&b.id)?;
        let resist = d
            .records_keyed("resistances")
            .next()
            .ok_or("missing hero resistances")?;
        let base = dd_data::combat::number(resist, "trap")?
            + self.kernel()?.modifier(id, "resistance", "trap");
        let difficulty = self.quest()["difficulty"]
            .as_u64()
            .ok_or("missing trap difficulty")? as usize;
        let table = self.definition.data.globals["difficulty_trap_base"]
            .as_array()
            .ok_or("missing trap difficulty table")?;
        let penalty = table
            .get(difficulty)
            .and_then(Value::as_f64)
            .ok_or("invalid trap level")?;
        let bonus = self.definition.data.globals["trap_scout_disarm_bonus"]
            .as_f64()
            .ok_or("missing trap scouting bonus")?;
        let success = disarm && self.state.rng.roll((base + bonus - penalty).clamp(0., 1.));
        let effects = if success { h.success } else { h.failure };
        self.field(|k| {
            if !success {
                k.hp(id, (f64::from(k.maximum(id)) * h.health).round() as i32)?;
            }
            for e in &effects {
                if !k.core.units[id].alive() {
                    break;
                }
                k.curio_effect(id, e)?;
            }
            Ok(())
        })?;
        self.map_settle()
    }
    fn supplies(&self) -> Result<(Vec<dd_data::items::SupplyStack>, BTreeMap<String, u32>)> {
        let mut totals = BTreeMap::new();
        for s in self.state.bag.slots.iter().flatten().filter(|s| {
            s.item.kind == "supply" || (s.item.kind == "estate" && s.item.id == "the_blood")
        }) {
            *totals.entry(s.item.id.clone()).or_insert(0u32) += s.amount;
        }
        let mut supplies = vec![];
        let mut reserves = BTreeMap::new();
        for (id, n) in totals {
            let def = self.definition.supply(&id)?;
            let current = n.min(def.stack_limit);
            if n > current {
                reserves.insert(id, n - current);
            }
            supplies.push(dd_data::items::SupplyStack {
                definition: def,
                remaining: current,
            });
        }
        Ok((supplies, reserves))
    }
    fn pool_eligible(&self, index: usize) -> Result<bool> {
        let e = self.definition.data.levels[self.state.level]
            .pools
            .entries
            .get(index)
            .ok_or("unknown hallway pool index")?;
        if !e.eligible_in_opening {
            return Ok(false);
        }
        for (k, v) in &e.conditions {
            let parse = |i: usize| {
                v.get(i)
                    .ok_or("missing encounter condition value")?
                    .parse::<f64>()
                    .map_err(|_| "invalid encounter condition value")
            };
            match k.as_str() {
                "darkness_range" => {
                    if f64::from(self.state.torch) < parse(0)?
                        || f64::from(self.state.torch) > parse(1)?
                    {
                        return Ok(false);
                    }
                }
                "inventory_valid_item_percent_range" => {
                    let used = self.state.bag.slots.iter().flatten().count() as f64
                        / self.state.bag.slots.len() as f64;
                    if used < parse(0)? || used > parse(1)? {
                        return Ok(false);
                    }
                }
                "limit" => {
                    if f64::from(self.state.limits.get(&e.id).copied().unwrap_or(0)) >= parse(0)? {
                        return Ok(false);
                    }
                }
                "min_vampire_heroes" | "max_vampire_heroes" => {
                    let n = self
                        .state
                        .heroes
                        .iter()
                        .filter(|h| {
                            self.state.party.contains(&h.guid)
                                && h.state
                                    .build
                                    .diseases
                                    .iter()
                                    .any(|id| id.contains("crimson_curse"))
                        })
                        .count() as f64;
                    if k == "min_vampire_heroes" && n < parse(0)?
                        || k == "max_vampire_heroes" && n > parse(0)?
                    {
                        return Ok(false);
                    }
                }
                "can_be_ambush" => {}
                "completed_flashback_id"
                | "plot_quest_requirement"
                | "random_dungeon_roaming_id"
                | "infestation_activity_level" => return Ok(false),
                _ => return Err("unsupported opening condition".into()),
            }
        }
        Ok(true)
    }
    fn ordinary_pool(&mut self) -> Result<usize> {
        let entries = &self.definition.data.levels[self.state.level].pools.entries;
        let mut ids = vec![];
        let mut weights = vec![];
        for (i, e) in entries.iter().enumerate() {
            if !e.conditional && !e.additional && self.pool_eligible(i)? {
                ids.push(i);
                weights.push(e.weight);
            }
        }
        let i = self.weighted(&weights)?;
        Ok(ids[i])
    }
    fn respawn(&mut self, m: &mut crate::maps014::Session, area: &str) -> Result<()> {
        let rows = self.definition.data.globals["corridor_return_content"]
            .as_array()
            .ok_or("missing corridor return rules")?
            .clone();
        let mut changed = BTreeMap::new();
        for kind in ["ac_battle", "ac_hunger", "ac_trap"] {
            let rows = rows.iter().filter(|v| v["key"] == kind).collect::<Vec<_>>();
            if rows.len() != 1 {
                return Err("ambiguous corridor return rule".into());
            }
            let row = &rows[0]["data"];
            let table = &row["base_dark_mod_table"];
            let lowest = table["lowest_excluded"]
                .as_bool()
                .ok_or("missing corridor darkness convention")?;
            let bands = table["range_table"]
                .as_array()
                .ok_or("missing corridor darkness table")?;
            let level = f64::from(self.state.torch);
            let bands = bands
                .iter()
                .filter(|b| {
                    let lo = b["range"]["lower"].as_f64();
                    let hi = b["range"]["upper"].as_f64();
                    match (lo, hi) {
                        (Some(lo), Some(hi)) => {
                            if lowest {
                                level > lo && level <= hi
                            } else {
                                level >= lo && level < hi
                            }
                        }
                        _ => false,
                    }
                })
                .collect::<Vec<_>>();
            if bands.len() != 1 {
                return Err("ambiguous corridor darkness band".into());
            }
            let chance = row["base_chance"]
                .as_f64()
                .ok_or("missing corridor base chance")?
                + bands[0]["value"]
                    .as_f64()
                    .ok_or("missing corridor dark modifier")?;
            if !(0. ..=1.).contains(&chance) {
                return Err("invalid corridor spawn chance".into());
            }
            if kind == "ac_hunger" && self.state.hunger_buffer > 0 {
                continue;
            }
            if !self.state.rng.roll(chance) {
                continue;
            }
            let vacant = self
                .active_map
                .map()
                .nodes
                .iter()
                .filter(|(id, n)| {
                    !n.room
                        && n.area == area
                        && !changed.contains_key(*id)
                        && ![
                            crate::maps014::EventKind::Encounter,
                            crate::maps014::EventKind::Trap,
                            crate::maps014::EventKind::Obstacle,
                            crate::maps014::EventKind::Hunger,
                            crate::maps014::EventKind::Curio,
                        ]
                        .iter()
                        .any(|k| m.remaining(id, *k))
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            if vacant.is_empty() {
                continue;
            }
            let id = vacant[self.state.rng.below(vacant.len() as u64) as usize].clone();
            let entry = if kind == "ac_battle" {
                Some(self.ordinary_pool()?)
            } else {
                None
            };
            let spawn = Spawn {
                kind: kind.into(),
                entry,
            };
            changed.insert(id.clone(), spawn.content());
            self.state.overrides.insert(id.clone(), spawn);
            self.state.chosen.remove(&id);
            self.state.examined.remove(&id);
        }
        if !changed.is_empty() {
            self.state.spawn_epoch += 1;
            m.replace_contents(&changed, self.state.spawn_epoch)?;
            self.active_map = m.definitions();
            self.state.map = m.checkpoint()?;
        }
        Ok(())
    }
    fn formation(&self, node: &str) -> Result<dd_data::opening015::Formation> {
        if self.definition.campaign.is_some() {
            return self.campaign_formation(node);
        }
        if let Some(index) = self
            .state
            .chosen
            .get(node)
            .copied()
            .or_else(|| self.state.overrides.get(node).and_then(|s| s.entry))
        {
            let pools = &self.definition.data.levels[self.state.level].pools;
            let e = pools
                .entries
                .get(index)
                .filter(|e| e.eligible_in_opening)
                .ok_or("invalid concrete encounter choice")?;
            let monsters = e
                .monsters
                .iter()
                .map(|id| {
                    pools
                        .catalog
                        .get(id)
                        .cloned()
                        .ok_or_else(|| format!("missing dynamic entity {id}"))
                })
                .collect::<Result<Vec<_>>>()?;
            return Ok(dd_data::opening015::Formation {
                monsters,
                loot: e.loot.clone(),
                sources: vec![e.id.clone()],
            });
        }
        self.definition.data.levels[self.state.level]
            .formations
            .get(node)
            .cloned()
            .ok_or("missing concrete opening encounter".into())
    }
    fn start_battle(&mut self) -> Result<()> {
        if self.definition.campaign.is_some() {
            self.campaign_choose_encounter()?;
            self.campaign_count_encounter()?;
        }
        let m = self.map()?;
        let node = m.position().to_string();
        if self.definition.campaign.is_none() && self.state.examined.insert(node.clone()) {
            let count = self.definition.data.levels[self.state.level]
                .pools
                .entries
                .len();
            for index in 0..count {
                let e = &self.definition.data.levels[self.state.level].pools.entries[index];
                if (e.conditional || e.additional) && self.pool_eligible(index)? {
                    let chance = e.weight;
                    if !(0. ..=1.).contains(&chance) {
                        return Err("conditional encounter chance exceeds probability".into());
                    }
                    if self.state.rng.roll(chance) {
                        self.state.chosen.insert(node.clone(), index);
                        break;
                    }
                }
            }
            if let Some(index) = self
                .state
                .chosen
                .get(&node)
                .copied()
                .or_else(|| self.state.overrides.get(&node).and_then(|s| s.entry))
            {
                let id = self.definition.data.levels[self.state.level].pools.entries[index]
                    .id
                    .clone();
                *self.state.limits.entry(id).or_default() += 1;
            }
        }
        let f = self.formation(&node)?;
        let mut heroes = self.encounter_heroes()?;
        for h in &mut heroes {
            if let Some(d) = &mut h.mechanics.hero.difficulty {
                d.entering_raid = false;
            }
        }
        let mut rules = self.field_rules()?;
        let mut context = rules.context.take().ok_or("missing battle context")?;
        context.corridor = !self.active_map.map().nodes[m.position()].room;
        context.stall = self.definition.data.levels[self.state.level].quest["is_stall_enabled"]
            .as_bool()
            .unwrap_or(false);
        if self.definition.campaign.is_some() && context.stall {
            self.campaign_stall(&mut context)?;
        }
        if f.monsters
            .iter()
            .any(|m| m.mechanics.lifecycle.always_surprise)
        {
            context.surprise = dd_data::build010::Surprise::Heroes;
        } else if f
            .monsters
            .iter()
            .any(|m| m.mechanics.lifecycle.always_be_surprised)
        {
            context.surprise = dd_data::build010::Surprise::Monsters;
        } else if self.state.level > 0
            && self.definition.data.levels[self.state.level].quest["is_surprise_enabled"] == true
        {
            let light = rules
                .lighting
                .as_ref()
                .ok_or("missing surprise light")?
                .at(self.state.torch)?;
            let source_key =
                |party: bool| match (context.corridor, self.state.encounter_known, party) {
                    (true, true, true) => "surprise_known_corridor_party_base_chance",
                    (true, true, false) => "surprise_known_corridor_monsters_base_chance",
                    (true, false, true) => "surprise_corridor_party_base_chance",
                    (true, false, false) => "surprise_corridor_monsters_base_chance",
                    (false, true, true) => "surprise_known_room_party_base_chance",
                    (false, true, false) => "surprise_known_room_monsters_base_chance",
                    (false, false, true) => "surprise_room_party_base_chance",
                    (false, false, false) => "surprise_room_monsters_base_chance",
                };
            let kernel = self.kernel()?;
            let mut party_bonus = 0.;
            let mut enemy_bonus = 0.;
            for id in 0..self.state.party.len() {
                party_bonus += kernel.modifier(id, "party_surprise_chance", "");
                enemy_bonus += kernel.modifier(id, "monsters_surprise_chance", "");
            }
            let cap = |key: &str| {
                self.definition.data.globals[key]
                    .as_f64()
                    .ok_or("missing surprise cap")
            };
            let hero_base = self.definition.data.globals[source_key(true)]
                .as_f64()
                .ok_or("missing party surprise")?;
            let enemy_base = self.definition.data.globals[source_key(false)]
                .as_f64()
                .ok_or("missing enemy surprise")?;
            let hero_chance = if hero_base < 0.
                || f.monsters
                    .iter()
                    .any(|m| !m.mechanics.lifecycle.can_surprise)
            {
                0.
            } else {
                (hero_base + light.heroes_surprised_increase / 100. + party_bonus)
                    .clamp(0., cap("surprise_max_party_surprised_chance")?)
            };
            let monster_chance = if enemy_base < 0.
                || f.monsters
                    .iter()
                    .any(|m| !m.mechanics.lifecycle.can_be_surprised)
            {
                0.
            } else {
                (enemy_base + light.monsters_surprised_increase / 100. + enemy_bonus)
                    .clamp(0., cap("surprise_max_monsters_surprised_chance")?)
            };
            context.surprise = if self.state.rng.roll(hero_chance) {
                dd_data::build010::Surprise::Heroes
            } else if self.state.rng.roll(monster_chance) {
                dd_data::build010::Surprise::Monsters
            } else {
                dd_data::build010::Surprise::Neither
            };
        }
        self.campaign_item_context(&mut context)?;
        self.camp_battle_context(&mut rules, &mut context)?;
        for bounds in f.monsters.iter().filter_map(|m| m.mechanics.boss.torch) {
            self.state.torch = self.state.torch.clamp(bounds[0], bounds[1]);
        }
        rules
            .lighting
            .as_mut()
            .ok_or("missing encounter light")?
            .torch = self.state.torch;
        rules.context = Some(context);
        let (supplies, reserves) = self.supplies()?;
        self.state.battle_start_supplies = supplies
            .iter()
            .map(|s| {
                (
                    s.definition.id.clone(),
                    s.remaining + reserves.get(&s.definition.id).copied().unwrap_or(0),
                )
            })
            .collect();
        self.state.battle_party = self.state.party.clone();
        let mut roots = f.monsters.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
        if self.definition.campaign.is_some()
            && let Some(c) = &rules.context
        {
            roots.extend(c.reinforcements.iter().map(|m| m.id.clone()));
        }
        let catalog = self.definition.catalog(&roots)?;
        let mut b = BattleSession::new_with_reserves(
            BattleInput {
                scenario: Scenario {
                    heroes,
                    monsters: f.monsters,
                    rules,
                    round_limit: 1000,
                },
                seed: self.state.rng.next(),
                supplies,
                catalog,
                waves: vec![],
            },
            reserves,
        )?;
        let result = b.advance()?;
        self.state.battles += 1;
        self.apply_battle(b, result.decision)
    }
    fn fight(&mut self, a: crate::Action) -> Result<()> {
        let mut b = BattleSession::restore(
            self.state.battle.as_ref().ok_or("no active battle")?,
            &self.definition.data.content,
            &self.definition.data.rules.candidates,
        )?;
        b.bind_host_buff_effects(&self.definition.game)?;
        let t = b.act(a)?;
        self.apply_battle(b, t.decision)
    }
    fn apply_battle(&mut self, b: BattleSession, decision: Decision) -> Result<()> {
        let door = b
            .entered_deaths_door()
            .iter()
            .filter_map(|id| self.state.battle_party.get(*id).copied())
            .collect();
        self.campaign_record_door(door);
        match decision {
            Decision::Player(o) => {
                self.reconcile_supplies(o.supplies.iter().cloned().collect())?;
                let states = b.party_state();
                self.reconcile_camp_buffs(&states, &self.state.battle_party.clone())?;
                for h in states {
                    let guid = *self
                        .state
                        .battle_party
                        .get(h.id)
                        .ok_or("unknown live battle identity")?;
                    let i = self.hero_index(guid)?;
                    self.state.heroes[i].state = h;
                }
                self.state.party = b
                    .party_order()
                    .iter()
                    .map(|id| self.state.battle_party[*id])
                    .collect();
                self.state.torch = o.torch.ok_or("live battle lost light")?;
                self.state.battle = Some(b.checkpoint()?);
            }
            Decision::WaveInterlude { .. } => {
                return Err("unexpected farm boundary in opening".into());
            }
            Decision::Finished(result) => {
                self.reconcile_camp_buffs(&result.heroes, &self.state.battle_party.clone())?;
                let camp_expired = self.camp_after_battle();
                self.campaign_battle_result(
                    &result,
                    &b.loot_eligible_kills(),
                    b.shared_goal_kills020()
                        .into_iter()
                        .map(|(_, s, l)| (s, l))
                        .collect(),
                )?;
                self.state.food_uses.clear();
                let mut remaining = BTreeMap::new();
                for s in &result.supplies {
                    *remaining.entry(s.definition.id.clone()).or_insert(0u32) += s.remaining;
                }
                self.reconcile_supplies(remaining)?;
                for h in &result.heroes {
                    let guid = *self
                        .state
                        .battle_party
                        .get(h.id)
                        .ok_or("unknown battle result identity")?;
                    let i = self.hero_index(guid)?;
                    self.state.heroes[i].state = h.clone();
                }
                self.state.party = b
                    .party_order()
                    .iter()
                    .map(|id| self.state.battle_party[*id])
                    .collect();
                self.state.torch = result.torch.ok_or("battle lost light state")?;
                self.state.battle = None;
                if camp_expired {
                    self.field_with_hp_policy(|k| k.settle(), true)?;
                }
                if self.complete_auxiliary(&result)? {
                    return Ok(());
                }
                match result.outcome {
                    Outcome::HeroesWin => {
                        let m = self.map()?;
                        let draws = if self.definition.campaign.is_some() {
                            vec![]
                        } else {
                            self.formation(m.position())?.loot
                        };
                        self.draws(&draws)?;
                        self.draws(&self.extra_loot(None)?)?;
                        self.dark_bonus("battle")?;
                        self.map_settle()?;
                        if self.state.loot.is_empty() {
                            self.forced_curio()?;
                        }
                    }
                    Outcome::Retreated => {
                        let mut m = self.map()?;
                        let receipt = m.pending().cloned().ok_or("retreat lost encounter")?;
                        // A dedicated map retreat transition preserves the unresolved encounter.
                        let to = if self.active_map.map().nodes[&receipt.node].room {
                            self.state.encounter_from.clone()
                        } else {
                            self.state.last_room.clone()
                        };
                        m.retreat_from_encounter(&receipt, &to)?;
                        self.state.map = m.checkpoint()?;
                        if self
                            .definition
                            .campaign
                            .as_ref()
                            .and_then(|s| s.context.automation.as_ref())
                            .and_then(|a| a.contract.as_ref())
                            .is_some_and(|c| c.retreat_always_from_raid)
                        {
                            self.campaign_finish(false)?;
                        }
                    }
                    Outcome::Teleported
                        if self
                            .definition
                            .campaign
                            .as_ref()
                            .is_some_and(|s| s.context.automation.is_some()) =>
                    {
                        let tags = self
                            .active_map
                            .map()
                            .teleport_targets
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>();
                        // DD3 repeats twelve destination labels for the same original set.
                        // Equivalent sets are unambiguous; differing sets require a source binding.
                        let destinations = &self.active_map.map().teleport_targets;
                        if tags.is_empty()
                            || destinations.values().any(|v| v != &destinations[&tags[0]])
                        {
                            return Err(
                                "missing/ambiguous original battle teleport destinations".into()
                            );
                        }
                        let mut m = self.map()?;
                        let receipt = m.pending().cloned().ok_or("teleport lost encounter")?;
                        m.teleport_from_encounter(&receipt, &tags[0])?;
                        self.state.map = m.checkpoint()?;
                        self.state.last_room = m.position().to_string();
                        self.state.encounter_from = m.position().to_string();
                        if m.pending()
                            .is_some_and(|p| p.kind == crate::maps014::EventKind::Encounter)
                        {
                            self.start_battle()?;
                        }
                    }
                    Outcome::MonstersWin | Outcome::MutualDefeat => self.state.party.clear(),
                    other => {
                        return Err(format!(
                            "opening battle did not reach a normal terminal outcome: {other:?}"
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    fn reconcile_supplies(&mut self, remaining: BTreeMap<String, u32>) -> Result<()> {
        for (id, before) in &self.state.battle_start_supplies {
            let after = remaining.get(id).copied().unwrap_or(0);
            if after > *before {
                return Err("battle created inventory supplies".into());
            }
            if *before > after {
                self.state.bag.consume(
                    &Item::new(
                        if id == "the_blood" {
                            "estate"
                        } else {
                            "supply"
                        },
                        id,
                    ),
                    *before - after,
                )?;
            }
        }
        self.state.battle_start_supplies = remaining;
        Ok(())
    }
    fn draws(&mut self, draws: &[dd_data::curios013::Draw]) -> Result<()> {
        if self.definition.campaign.is_some() {
            return self.campaign_draws(draws);
        }
        let d = &self.definition.data;
        let mut rolled = vec![];
        let mut resolver = crate::loot015::Resolver {
            curios: &d.curios,
            difficulty: self.quest()["difficulty"]
                .as_i64()
                .ok_or("missing loot difficulty")?,
            dungeon: self.quest()["dungeon"]
                .as_str()
                .ok_or("missing loot region")?
                .into(),
            hero_classes: self.definition.game.hero_ids().cloned().collect(),
            rng: &mut self.state.rng,
            owned: &self.state.rolled,
        };
        for draw in draws {
            for _ in 0..draw.count {
                resolver.loot(&draw.table, &mut vec![], &mut rolled)?;
            }
        }
        for r in &rolled {
            let item = Item::new(&r.kind, &r.id);
            self.inventory_rules()?.definition(&item)?;
            self.state.loot.push(Stack {
                item,
                amount: r.amount,
            });
        }
        self.state.rolled.extend(rolled);
        Ok(())
    }
    fn dark_bonus(&mut self, kind: &str) -> Result<()> {
        let raw = dd_data::library::unique_json(
            &self.definition.data.curios.darkness,
            "definition",
            "opening dark bonuses",
        )?;
        let rows = raw["darkness_bonuses"]
            .as_array()
            .ok_or("missing darkness bonuses")?;
        let family = rows.iter().filter(|v| v["key"] == kind).collect::<Vec<_>>();
        if family.len() != 1 {
            return Err("ambiguous darkness bonus".into());
        }
        let bands = family[0]["data"]
            .as_array()
            .ok_or("missing darkness bands")?;
        let row = bands
            .iter()
            .filter(|v| {
                v["darkness"]
                    .as_u64()
                    .is_some_and(|n| n <= u64::from(self.state.torch))
            })
            .max_by_key(|v| v["darkness"].as_u64())
            .ok_or("no darkness tier")?;
        let chance = row["chance"]
            .as_f64()
            .filter(|n| (0. ..=1.).contains(n))
            .ok_or("invalid darkness chance")?;
        let draws = row["codes"]
            .as_array()
            .ok_or("missing darkness codes")?
            .iter()
            .map(|v| {
                Ok(dd_data::curios013::Draw {
                    table: v.as_str().ok_or("invalid darkness table")?.into(),
                    count: 1,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if chance > 0. && self.state.rng.roll(chance) {
            self.draws(&draws)?;
        }
        Ok(())
    }
    fn interact(&mut self, hero: u64, item: Option<String>) -> Result<()> {
        self.curio(hero, item, false)
    }
    fn curio(&mut self, hero: u64, item: Option<String>, keep: bool) -> Result<()> {
        if self.definition.campaign.is_some() {
            return self.campaign_curio(hero, item, keep);
        }
        let mut m = self.map()?;
        let name = self.active_map.map().nodes[m.position()]
            .content
            .curio
            .as_ref()
            .ok_or("no active curio")?
            .clone();
        let (_, c) = self
            .definition
            .data
            .curios
            .prop(dd_data::dson013::hash(&name))?;
        let id = self
            .state
            .party
            .iter()
            .position(|v| *v == hero)
            .ok_or("curio target is not a living party hero")?;
        let outcome = if let Some(item) = &item {
            if !c.full {
                return Err("tutorial curio does not accept items".into());
            }
            let Some(o) = c.items.get(item) else {
                return Ok(());
            };
            self.state.bag.consume(&Item::new("supply", item), 1)?;
            o.clone()
        } else {
            let i = self.weighted(&c.outcomes.iter().map(|v| v.1).collect::<Vec<_>>())?;
            c.outcomes[i].0.clone()
        };
        m.act(crate::maps014::Action::Interact)?;
        match outcome {
            CurioOutcome::Nothing => {}
            CurioOutcome::Loot {
                draws,
                nothing_chance,
            } => {
                if nothing_chance == 0. || !self.state.rng.roll(nothing_chance) {
                    self.draws(&draws)?;
                    self.draws(&self.extra_loot(Some(hero))?)?;
                    self.dark_bonus("chest")?;
                }
            }
            CurioOutcome::Effect(es) => {
                let i = self.weighted(&es.iter().map(|v| v.1).collect::<Vec<_>>())?;
                let e = self.definition.data.levels[self.state.level]
                    .effects
                    .get(&es[i].0)
                    .ok_or("missing loaded curio effect")?
                    .clone();
                self.field(|k| {
                    k.curio_effect(id, &e)?;
                    Ok(())
                })?;
            }
        }
        let p = m.pending().cloned().ok_or("missing curio boundary")?;
        m.resume(&p, crate::maps014::Resolution::Settled)?;
        self.state.map = m.checkpoint()?;
        self.state.interactions += 1;
        if keep {
            self.state.loot.clear();
        }
        Ok(())
    }
    fn forced_curio(&mut self) -> Result<()> {
        if self.definition.campaign.is_some() {
            return self.campaign_forced_curio();
        }
        let m = self.map()?;
        if !m
            .legal_actions()
            .contains(&crate::maps014::Action::Interact)
        {
            return Ok(());
        }
        if !self
            .state
            .forced_checked
            .insert((self.state.level, m.position().to_string()))
        {
            return Ok(());
        }
        let name = self.active_map.map().nodes[m.position()]
            .content
            .curio
            .as_ref()
            .ok_or("missing forced curio")?;
        let (_, c) = self
            .definition
            .data
            .curios
            .prop(dd_data::dson013::hash(name))?;
        let raw = dd_data::library::unique_json(
            &self.definition.data.curios.types,
            &c.id,
            "forced curio tags",
        )?;
        let rows = raw["rows"].as_array().ok_or("missing curio rows")?;
        let start = rows.iter().position(|r| r[2] == "TAGS");
        let mut tags = BTreeSet::new();
        if let Some(i) = start {
            for row in rows.iter().skip(i + 1) {
                let tag = row[2].as_str().unwrap_or("");
                if tag.is_empty() {
                    break;
                }
                tags.insert(tag.to_string());
            }
        }
        for guid in self.state.party.clone() {
            let quirks = self.state.heroes[self.hero_index(guid)?]
                .state
                .build
                .quirks
                .clone();
            for q in quirks {
                let v = dd_data::library::unique_json(
                    &self.definition.data.quirk_defs,
                    &q,
                    "forced interaction quirk",
                )?;
                if let Some(tag) = v.get("curio_tag").and_then(Value::as_str)
                    && (tag == "All" || tags.contains(tag))
                {
                    let chance = v["curio_tag_chance"]
                        .as_f64()
                        .filter(|n| (0. ..=1.).contains(n))
                        .ok_or("invalid forced curio chance")?;
                    let keep = v["keep_loot"].as_bool().unwrap_or(false);
                    if self.state.rng.roll(chance) {
                        return self.curio(guid, None, keep);
                    }
                }
            }
        }
        Ok(())
    }
    fn use_item(&mut self, hero: u64, slot: usize) -> Result<()> {
        let s = self
            .state
            .bag
            .slots
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("empty item slot")?
            .clone();
        let id = self
            .state
            .party
            .iter()
            .position(|v| *v == hero)
            .ok_or("item target not in party")?;
        if s.item.kind == "provision" {
            let ratio = self.definition.data.globals["provision_hp_heal"]
                .as_f64()
                .ok_or("missing food heal")?;
            let h = &self.state.heroes[self.hero_index(hero)?];
            if h.state.hp >= h.state.max_hp {
                return Err("hero does not need food healing".into());
            }
            if self.state.food_uses.get(&hero).copied().unwrap_or(0)
                >= self.definition.data.candidates.manual_food_limit
            {
                return Err("hero is satiated".into());
            }
            *self.state.food_uses.entry(hero).or_default() += 1;
            self.state.bag.consume(&s.item, 1)?;
            return self.field(|k| {
                k.hp(
                    id,
                    (f64::from(k.maximum(id))
                        * ratio
                        * (1. + k.modifier(id, "hp_heal_received_percent", "eat")).max(0.))
                    .round() as i32,
                )?;
                Ok(())
            });
        }
        if s.item.kind != "supply" && !(s.item.kind == "estate" && s.item.id == "the_blood") {
            return Err("item is not usable during exploration".into());
        }
        match s.item.id.as_str() {
            "torch"
                if self.definition.field_rules[self.state.level]
                    .bloodlight
                    .is_some() =>
            {
                self.campaign_torch(id)?;
            }
            "torch" => {
                self.state.bag.consume(&s.item, 1)?;
                self.state.torch = (i32::from(self.state.torch)
                    + self.definition.data.candidates.torch_amount)
                    .clamp(0, 100) as u8;
            }
            "bandage" | "antivenom" | "medicinal_herbs" | "holy_water" | "laudanum"
            | "the_blood" => {
                let supply = self.definition.supply(&s.item.id)?;
                self.state.bag.consume(&s.item, 1)?;
                self.field(|k| {
                    k.core.supplies = vec![dd_data::items::SupplyStack {
                        definition: supply,
                        remaining: 1,
                    }];
                    k.core.consume_supply(id, 0, &mut vec![])?;
                    k.settle()
                })?;
            }
            _ => {
                return Err("item only supports curio/obstacle interaction in this opening".into());
            }
        }
        Ok(())
    }

    fn finish(&mut self, completed: bool) -> Result<()> {
        if self.definition.campaign.is_some() {
            return self.campaign_finish(completed);
        }
        if completed && !self.ready()? {
            return Err("quest goal not complete".into());
        }
        if !completed && !self.state.party.is_empty() {
            let amount = self.definition.data.candidates.abandon_stress;
            let check = self.definition.data.candidates.abandon_resolve_check;
            self.field(|k| {
                for id in 0..k.core.units.len() {
                    k.abandon_stress(id, amount, check)?;
                }
                Ok(())
            })?;
        }
        let mut rewards = vec![];
        let mut xp = 0;
        if completed {
            let reward = self.quest()["completion_reward"].clone();
            xp = reward["resolve_xp"].as_u64().ok_or("missing quest xp")? as u32;
            for v in reward["items_definition"]["items"]
                .as_object()
                .ok_or("missing quest reward items")?
                .values()
            {
                let amount = v["amount"].as_u64().ok_or("missing reward amount")? as u32;
                if amount > 0 {
                    rewards.push(Stack {
                        item: Item::new(
                            v["type"].as_str().ok_or("missing reward type")?,
                            v["id"].as_str().ok_or("missing reward id")?,
                        ),
                        amount,
                    });
                }
            }
            let extra=self.definition.data.levels[self.state.level].quest["additional_trinket_completion_rewards"].as_array().cloned().unwrap_or_default();
            for row in extra {
                let rarity = row["rarity"]
                    .as_str()
                    .ok_or("missing completion trinket rarity")?;
                let amount = row["amount"]
                    .as_u64()
                    .filter(|n| *n <= 128)
                    .ok_or("invalid trinket amount")?;
                let d = &self.definition.data;
                let mut resolver = crate::loot015::Resolver {
                    curios: &d.curios,
                    difficulty: self.quest()["difficulty"]
                        .as_i64()
                        .ok_or("missing reward difficulty")?,
                    dungeon: self.quest()["dungeon"]
                        .as_str()
                        .ok_or("missing reward region")?
                        .into(),
                    hero_classes: self.definition.game.hero_ids().cloned().collect(),
                    rng: &mut self.state.rng,
                    owned: &self.state.rolled,
                };
                let mut drawn = vec![];
                for _ in 0..amount {
                    let r = resolver.trinket(rarity, &drawn)?;
                    rewards.push(Stack {
                        item: Item::new(&r.kind, &r.id),
                        amount: r.amount,
                    });
                    drawn.push(r);
                }
                self.state.rolled.extend(drawn);
            }
            let kernel = self.kernel()?;
            let awards = self
                .state
                .party
                .iter()
                .enumerate()
                .map(|(id, guid)| {
                    (
                        *guid,
                        (f64::from(xp)
                            * (1. + kernel.modifier(id, "resolve_xp_bonus_percent", "")).max(0.))
                        .round() as u32,
                    )
                })
                .collect::<BTreeMap<_, _>>();
            for h in &mut self.state.heroes {
                if h.state.alive
                    && let Some(award) = awards.get(&h.guid)
                {
                    h.xp = h.xp.checked_add(*award).ok_or("hero xp overflow")?;
                }
            }
        }
        let dead = self.state.party.is_empty();
        let mut brought_home = if dead {
            vec![]
        } else {
            self.state
                .bag
                .slots
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>()
        };
        if !completed && !dead {
            let mut totals = BTreeMap::<Item, u32>::new();
            for stack in brought_home {
                *totals.entry(stack.item).or_default() += stack.amount;
            }
            brought_home = vec![];
            for (item, n) in totals {
                let raw = dd_data::library::unique_json(
                    &self.definition.data.return_rates,
                    &item.kind,
                    "failure retention",
                )?;
                let rate = raw["rate"]
                    .as_f64()
                    .filter(|v| (0. ..=1.).contains(v))
                    .ok_or("invalid failure keep rate")?;
                let amount = (f64::from(n) * rate).floor() as u32;
                if amount > 0 {
                    brought_home.push(Stack { item, amount });
                }
            }
        }
        for stack in brought_home.iter().chain(&rewards) {
            self.bank(stack)?;
        }
        let outcome = if completed {
            "completed"
        } else if dead {
            "party_dead"
        } else {
            "abandoned"
        };
        self.state.results.push(StageResult {
            level: self.definition.data.levels[self.state.level].map.id.clone(),
            completed,
            outcome: outcome.into(),
            heroes: self
                .state
                .heroes
                .iter()
                .map(|h| (h.guid, h.state.clone()))
                .collect(),
            brought_home,
            rewards,
            xp,
        });
        self.state.battle = None;
        self.state.loot.clear();
        self.state.bag = Bag::new(&*self.inventory_rules()?);
        if self.state.level == 0 {
            self.town_boundary()?;
            self.state.phase = Phase::Preparation;
            self.fill_opening_party()?;
            self.state.provision_party = self.state.party.clone();
            self.prepare_store()?;
        } else {
            self.town_boundary()?;
            self.state.phase = Phase::Finished;
        }
        Ok(())
    }
    fn bank(&mut self, stack: &Stack) -> Result<()> {
        let key = if ["supply", "provision", "gem"].contains(&stack.item.kind.as_str()) {
            let price = self.inventory_rules()?.definition(&stack.item)?.sell;
            let amount = stack
                .amount
                .checked_mul(price)
                .ok_or("sale value overflow")?;
            let gold = self.state.wallet.entry("gold:".into()).or_default();
            *gold = gold.checked_add(amount).ok_or("wallet overflow")?;
            return Ok(());
        } else {
            stack.item.key()
        };
        let n = self.state.wallet.entry(key).or_default();
        *n = n.checked_add(stack.amount).ok_or("wallet overflow")?;
        Ok(())
    }
    fn town_boundary(&mut self) -> Result<()> {
        let mut blueprints = vec![];
        let ids = self.state.party.clone();
        for id in &ids {
            let h = &self.state.heroes[self.hero_index(*id)?];
            let mut b = self.carry(h)?;
            b.mechanics.hero.initial_hp = None;
            let carry = b
                .mechanics
                .hero
                .carry
                .as_mut()
                .ok_or("missing town carry")?;
            carry.status = Default::default();
            carry.buffs.clear();
            blueprints.push(b);
        }
        if !blueprints.is_empty() {
            let k = PartyKernel::new(blueprints, self.field_rules()?, self.state.rng.clone())?;
            let (states, rng) = k.export();
            self.state.rng = rng;
            for (id, state) in ids.into_iter().zip(states) {
                let i = self.hero_index(id)?;
                self.state.heroes[i].state = state;
            }
        }
        Ok(())
    }
    fn fill_opening_party(&mut self) -> Result<()> {
        let classes = self.definition.data.recruit_classes.clone();
        for class in classes {
            self.recruit(&class)?;
        }
        let wanted = self.definition.data.raid_source["data"]["party"]["start_heroes_size"]
            .as_u64()
            .filter(|n| *n <= 4)
            .ok_or("invalid opening target party size")? as usize;
        while self.state.party.len() < wanted {
            let mut classes = vec![];
            let mut weights = vec![];
            for class in self.definition.game.hero_ids() {
                let doc = self.definition.game.hero_document(class)?;
                let generation = doc
                    .records_keyed("generation")
                    .next()
                    .ok_or("missing hero generation")?;
                let enabled = dd_data::combat::text(generation, "is_generation_enabled")?;
                if enabled == "true" {
                    classes.push(class.clone());
                    weights.push(dd_data::combat::number(
                        generation,
                        "number_of_cards_in_deck",
                    )?);
                } else if enabled != "false" {
                    return Err("invalid class generation switch".into());
                }
            }
            let i = self.weighted(&weights)?;
            self.recruit(&classes[i])?;
        }
        Ok(())
    }
    fn recruit(&mut self, class: &str) -> Result<()> {
        let doc = self.definition.game.hero_document(class)?.clone();
        let generation = doc
            .records_keyed("generation")
            .next()
            .ok_or("missing recruit generation")?;
        let number = |k: &str| -> Result<u32> {
            let n = dd_data::combat::number(generation, k)?;
            if n.fract() != 0. || !(0. ..=100.).contains(&n) {
                return Err("invalid recruit generation count".into());
            }
            Ok(n as u32)
        };
        let mut skills = self.definition.game.hero_skill_ids(class)?;
        for i in (1..skills.len()).rev() {
            let j = self.state.rng.below((i + 1) as u64) as usize;
            skills.swap(i, j);
        }
        let amount = number("number_of_random_combat_skills")? as usize;
        if amount == 0 || amount > skills.len() {
            return Err("invalid recruit skill draw".into());
        }
        skills.truncate(amount);
        let refs = skills.iter().map(String::as_str).collect::<Vec<_>>();
        let base = self.definition.game.hero(class, 0, 0, &refs)?;
        let mut selected = vec![];
        for positive in [true, false] {
            let lo = number(if positive {
                "number_of_positive_quirks_min"
            } else {
                "number_of_negative_quirks_min"
            })?;
            let hi = number(if positive {
                "number_of_positive_quirks_max"
            } else {
                "number_of_negative_quirks_max"
            })?;
            if lo > hi {
                return Err("invalid recruit quirk range".into());
            }
            let n = lo + self.state.rng.below(u64::from(hi - lo + 1)) as u32;
            for _ in 0..n {
                let mut eligible = vec![];
                let mut weights = vec![];
                for (id, raw) in &self.definition.data.quirk_defs {
                    let v = dd_data::library::unique_json(
                        &self.definition.data.quirk_defs,
                        id,
                        "recruit quirk",
                    )?;
                    if v["is_positive"].as_bool() != Some(positive)
                        || v["is_disease"].as_bool() != Some(false)
                        || selected.contains(id)
                        || base.mechanics.hero.incompatible_quirks.contains(id)
                    {
                        continue;
                    }
                    let w = v["random_chance"].as_f64().ok_or("missing quirk weight")?;
                    if w <= 0. {
                        continue;
                    }
                    let conflict = v["incompatible_quirks"].as_array().is_some_and(|a| {
                        a.iter().any(|v| {
                            v.as_str()
                                .is_some_and(|id| selected.iter().any(|s| s == id))
                        })
                    }) || selected.iter().any(|s| {
                        self.definition.data.quirk_defs[s]["incompatible_quirks"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|v| v == id))
                    });
                    if !conflict {
                        eligible.push(id.clone());
                        weights.push(w);
                    }
                    let _ = raw;
                }
                let i = self.weighted(&weights)?;
                selected.push(eligible[i].clone());
            }
        }
        let mut b = self.definition.game.equip_build(
            base,
            dd_data::build010::Build {
                quirks: selected,
                ..Default::default()
            },
        )?;
        b.mechanics.hero.carry = Some(dd_data::hero010::HeroCarry {
            status: Default::default(),
            stress: 0,
            resolve: dd_data::hero010::CarryResolve::Calm,
            buffs: vec![],
        });
        let k = PartyKernel::new(vec![b.clone()], self.field_rules()?, self.state.rng.clone())?;
        let (mut result, rng) = k.export();
        self.state.rng = rng;
        let guid = self.state.next_guid;
        self.state.next_guid += 1;
        b.mechanics.hero.carry = None;
        b.mechanics.hero.initial_hp = None;
        self.state.heroes.push(Hero {
            guid,
            name: class.into(),
            xp: 0,
            base: b,
            state: result.remove(0),
        });
        self.state.party.push(guid);
        Ok(())
    }
    fn prepare_store(&mut self) -> Result<()> {
        let length = self.definition.data.levels[1].quest["quest"]["length"]
            .as_u64()
            .ok_or("missing provision length")? as usize;
        let rows = self.definition.data.provision["default_store_inventory_item_lists"]
            .as_array()
            .and_then(|a| a.get(length))
            .and_then(Value::as_array)
            .ok_or("missing provision store")?;
        self.state.store.clear();
        for v in rows {
            let item = Item::new(
                v["type"].as_str().ok_or("missing store type")?,
                v["id"].as_str().ok_or("missing store id")?,
            );
            let amount = v["amount"]
                .as_u64()
                .filter(|n| *n <= 1000000)
                .ok_or("invalid store count")? as u32;
            self.inventory_rules()?.definition(&item)?;
            if self.state.store.insert(item.key(), amount).is_some() {
                return Err("ambiguous provision store item".into());
            }
        }
        // Original length/class grants are applied before player purchases and occupy actual slots.
        let length_items =
            self.definition.data.provision["raid_starting_length_inventory_item_lists"]
                .as_array()
                .and_then(|a| a.get(length))
                .and_then(Value::as_array)
                .ok_or("missing starting provision list")?
                .clone();
        let mut grants = length_items;
        for id in &self.state.party {
            let class = &self.state.heroes[self.hero_index(*id)?].state.class;
            for list in self.definition.data.provision["raid_starting_hero_class_item_lists"]
                .as_array()
                .ok_or("missing class provision grants")?
            {
                if list["hero_class"] == *class {
                    grants.extend(
                        list["item_lists"]
                            .as_array()
                            .ok_or("invalid class starting items")?
                            .clone(),
                    );
                }
            }
        }
        for v in grants {
            let amount = v["amount"].as_u64().ok_or("invalid starting grant")? as u32;
            if amount > 0 {
                let stack = Stack {
                    item: Item::new(
                        v["type"].as_str().ok_or("missing grant kind")?,
                        v["id"].as_str().ok_or("missing grant id")?,
                    ),
                    amount,
                };
                if self.state.bag.insert(&*self.inventory_rules()?, &stack)? != amount {
                    return Err("starting provisions do not fit".into());
                }
            }
        }
        Ok(())
    }
    fn embark(&mut self) -> Result<()> {
        if self.state.party.len() != 4 {
            return Err("opening requires a prepared full party".into());
        }
        self.state.level = 1;
        self.state.phase = Phase::FirstRuins;
        self.active_map = self.definition.maps[1].clone();
        self.state.overrides.clear();
        self.state.spawn_epoch = 0;
        self.state.chosen.clear();
        self.state.limits.clear();
        self.state.examined.clear();
        self.state.hunger_buffer =
            self.definition.data.raid_source["data"]["party"]["inventory"]["hunger_room_buffer"]
                .as_u64()
                .ok_or("missing hunger room buffer")? as u32;
        self.state.last_room = self.active_map.map().entrance.clone();
        self.state.encounter_from = self.state.last_room.clone();
        self.state.torch = self.definition.data.raid_source["data"]["party"]["torchlight"]
            .as_u64()
            .ok_or("missing source raid light")? as u8;
        self.state.map = crate::maps014::Session::new(
            self.definition.maps[1].clone(),
            self.state.rng.next(),
            BTreeMap::new(),
        )?
        .checkpoint()?;
        let thresholds = self.definition.data.roster_variables["resolve_level_thresholds"]
            .as_array()
            .ok_or("missing resolve thresholds")?
            .clone();
        let mut entry_costs = vec![];
        for id in self.state.party.clone() {
            let i = self.hero_index(id)?;
            let level = thresholds
                .iter()
                .filter(|v| {
                    v.as_u64()
                        .is_some_and(|n| n <= u64::from(self.state.heroes[i].xp))
                })
                .count()
                .saturating_sub(1) as u32;
            let mut b = self.state.heroes[i].base.clone();
            b = self.definition.game.hero_at_difficulty(
                b,
                level,
                self.quest()["difficulty"]
                    .as_u64()
                    .ok_or("missing quest level")? as u32,
                true,
            )?;
            let initial = b
                .mechanics
                .hero
                .difficulty
                .as_ref()
                .ok_or("missing entry difficulty")?
                .entry_stress;
            self.state.heroes[i].base = b;
            entry_costs.push(initial);
        }
        self.field(|k| {
            for (id, n) in entry_costs.iter().enumerate() {
                k.stress(id, *n)?;
            }
            Ok(())
        })?;
        Ok(())
    }
    pub fn metrics(&self) -> Value {
        json!({"phase":self.state.phase,"deaths":self.state.heroes.iter().filter(|h|!h.state.alive).count(),"survivors":self.state.heroes.iter().filter(|h|h.state.alive).count(),"battles":self.state.battles,"interactions":self.state.interactions,"steps":self.state.steps,"results":self.state.results.iter().map(|r|json!({"level":r.level,"outcome":r.outcome,"xp":r.xp})).collect::<Vec<_>>()})
    }
}

#[derive(Clone, Serialize)]
pub struct TownHandoff {
    pub(crate) receipt: String,
    pub(crate) definition_id: String,
    pub(crate) heroes: Vec<Hero>,
    pub(crate) party: Vec<u64>,
    pub(crate) wallet: BTreeMap<String, u32>,
    pub(crate) results: Vec<StageResult>,
    pub(crate) next_guid: u64,
}
impl Session {
    pub fn town_handoff(&self) -> Result<TownHandoff> {
        use sha2::{Digest, Sha256};
        if self.state.phase != Phase::Finished {
            return Err("opening has not finished both expeditions".into());
        }
        self.validate()?;
        Ok(TownHandoff {
            receipt: format!(
                "opening-return:{:x}",
                Sha256::digest(self.checkpoint()?.as_bytes())
            ),
            definition_id: self.definition.id.clone(),
            heroes: self.state.heroes.clone(),
            party: self.state.party.clone(),
            wallet: self.state.wallet.clone(),
            results: self.state.results.clone(),
            next_guid: self.state.next_guid,
        })
    }
}
