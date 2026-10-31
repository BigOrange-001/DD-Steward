//! Campaign-owned extension of the shared exploration executor. Old opening
//! definitions keep their original profile and settlement branches.
use super::*;
use crate::expedition_sources017::{Sources, Special, pick};
use dd_data::field017::ResultKind;
#[path = "camping017.rs"]
mod camping;
pub(crate) use camping::Deck as CampDeck;
pub use camping::{Action as CampAction, View as CampView};
pub(crate) use camping::{VisibleEffect, VisibleSkill};
#[path = "expedition_assets017.rs"]
mod assets;
pub(crate) use assets::Carried;
pub use assets::ItemView;
pub const PROFILE: &str = "candidate-expedition-017";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<crate::campaign018::RaidRules>,
    pub token: String,
    pub bonuses: crate::town016::Bonuses,
    pub camping: BTreeMap<u64, CampDeck>,
    pub camping_slots: usize,
    pub completed: BTreeSet<String>,
    pub infestation: String,
    pub activity: String,
    pub suppressed: BTreeSet<String>,
    pub roaming: BTreeSet<String>,
    pub flashbacks: BTreeSet<String>,
    pub exit: Value,
    pub resolve_history: BTreeMap<u64, BTreeMap<String, u32>>,
    pub locked_quirks: BTreeMap<u64, BTreeSet<String>>,
}
#[derive(Clone, Serialize)]
pub(super) struct Spec {
    pub sources: Sources,
    pub context: Context,
    pub canon: BTreeMap<u64, Blueprint>,
}
/// Source-ordered, finite loot obligation from one actual native kill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RaidQuirk {
    pub(crate) kill: usize,
    pub choices: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Progress {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shared_goal_kills: Vec<(String, String)>,
    physical_bag: BTreeMap<usize, Carried>,
    physical_tray: BTreeMap<usize, Carried>,
    baseline_cleared: usize,
    baseline_rooms: usize,
    baseline_prisoners: BTreeSet<String>,
    activated: BTreeSet<String>,
    cleared: BTreeSet<String>,
    kills: Vec<(String, String)>,
    prisoners: BTreeSet<String>,
    raid_quirks: Vec<RaidQuirk>,
    ends_infestation: bool,
    terminal: Option<crate::events017::ReturnKind>,
    pub(super) auxiliary: Option<Auxiliary>,
    partial_scout: BTreeMap<String, BTreeSet<String>>,
    deaths_door: BTreeSet<u64>,
    progress_before_return: Option<(u32, u32)>,
    camp: Option<camping::State>,
    camp_selected: BTreeMap<u64, Vec<String>>,
    field_buffs: Vec<camping::FieldBuff>,
    next_camp_buff: u64,
    dream_attempts: BTreeMap<String, u32>,
    completed_flashbacks: BTreeSet<String>,
    earned_rewards: Vec<Stack>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Auxiliary {
    monsters: Vec<String>,
    kind: AuxKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum AuxKind {
    Pool { index: usize, ambush: bool },
    Curio(String),
    Nightmare(String),
}
/// A paused physical world, deliberately excluding the previous party, camp
/// Buffs and completion rewards. Only this owner can create/reinstall it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Persistent {
    source_id: String,
    map: String,
    progress: Progress,
    chosen: BTreeMap<String, usize>,
    rng: Rng,
    rolled: Vec<crate::initial013::Reward>,
    last_room: String,
    encounter_from: String,
    hunger_buffer: u32,
    pub keys: Vec<Stack>,
}
fn validate_shared_goal_kills020(sources: &Sources, p: &Progress) -> Result<()> {
    if p.shared_goal_kills.len() > p.kills.len().saturating_mul(3) {
        return Err("shared goal deaths exceed native four-body groups".into());
    }
    for (source, last) in &p.shared_goal_kills {
        let a = sources
            .catalog
            .get(source)
            .ok_or("unknown shared goal source")?;
        let b = sources
            .catalog
            .get(last)
            .ok_or("unknown shared goal form")?;
        let group = b
            .mechanics
            .boss
            .shared_health
            .as_ref()
            .ok_or("goal death is not a shared body")?;
        if a.mechanics.boss.shared_health.as_ref() != Some(group)
            || !p.kills.iter().any(|(_, last)| {
                sources
                    .catalog
                    .get(last)
                    .is_some_and(|b| b.mechanics.boss.shared_health.as_ref() == Some(group))
            })
        {
            return Err("shared goal death lacks counted group provenance".into());
        }
    }
    Ok(())
}
impl Persistent {
    pub(crate) fn kill_history020(&self) -> &[(String, String)] {
        &self.progress.kills
    }
    pub(crate) fn validate(
        &self,
        sources: &Sources,
        content: &dd_data::content::ContentConfig,
    ) -> Result<()> {
        use sha2::{Digest, Sha256};
        let source_id = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(sources).map_err(|e| e.to_string())?)
        );
        if self.source_id != source_id || !sources.map.persistent {
            return Err("paused world source differs".into());
        }
        let definition = crate::maps014::Definition::new(
            sources.map.clone(),
            content.clone(),
            MapCandidates::for_content(content)?,
        )?;
        let map = crate::maps014::Session::resume_campaign_map(&self.map, &definition)?;
        let p = &self.progress;
        validate_shared_goal_kills020(sources, p)?;
        if p.terminal.is_some()
            || p.camp.is_some()
            || p.auxiliary.is_some()
            || !p.field_buffs.is_empty()
            || !p.physical_bag.is_empty()
            || !p.physical_tray.is_empty()
            || !p.earned_rewards.is_empty()
            || !p.completed_flashbacks.is_empty()
            || !p.dream_attempts.is_empty()
            || !p.deaths_door.is_empty()
            || p.progress_before_return.is_some()
            || p.baseline_cleared != p.cleared.len()
            || p.baseline_prisoners != p.prisoners
            || p.baseline_rooms
                != map
                    .visited()
                    .iter()
                    .filter(|id| sources.map.nodes[*id].room)
                    .count()
            || p.cleared
                .iter()
                .chain(&p.activated)
                .chain(&p.prisoners)
                .any(|id| !sources.map.nodes.contains_key(id))
            || self.chosen.iter().any(|(node, i)| {
                !sources.map.nodes.contains_key(node) || *i >= sources.pools.entries.len()
            })
            || !sources.map.nodes.contains_key(&self.last_room)
            || !sources.map.nodes.contains_key(&self.encounter_from)
        {
            return Err("invalid paused expedition progress".into());
        }
        let mut items = BTreeSet::new();
        for key in &self.keys {
            let maximum = sources
                .map
                .key_sources
                .values()
                .filter(|s| s.item == key.item.id)
                .map(|s| s.amount)
                .sum::<u32>();
            if key.item.kind != "quest_item"
                || key.amount == 0
                || key.amount > maximum
                || !items.insert(&key.item)
                || !sources
                    .map
                    .nodes
                    .values()
                    .any(|n| n.gate.as_ref() == Some(&key.item.id))
            {
                return Err("invalid paused physical key provenance".into());
            }
        }
        Ok(())
    }
}
/// Only this executor creates the certificate, after its actual terminal state.
/// Fields are private outside the crate; a policy cannot assert completion.
#[derive(Debug, Clone)]
pub(crate) struct Return {
    pub goal_kills: Vec<(String, String)>,
    pub token: String,
    pub kind: crate::events017::ReturnKind,
    pub heroes: Vec<(u64, HeroResult)>,
    pub inventory: Vec<Stack>,
    pub battles: u32,
    pub survival: u32,
    pub rooms: u32,
    pub kills: Vec<(String, String)>,
    pub ends_infestation: bool,
    pub prisoners: BTreeSet<String>,
    pub raid_quirks: Vec<RaidQuirk>,
    pub map: String,
    pub persistent: Option<Persistent>,
    pub deaths_door: u32,
    pub afflictions: u32,
    pub flashbacks: BTreeSet<String>,
    pub earned_rewards: Vec<Stack>,
    pub recovered: Vec<Carried>,
}
impl Definition {
    pub(super) fn profile(&self) -> &'static str {
        if self.campaign.is_some() {
            PROFILE
        } else {
            EXP_PROFILE
        }
    }
}
impl Session {
    pub(crate) fn from_campaign(
        game: Arc<GameData>,
        mut data: Data,
        sources: Sources,
        context: Context,
        heroes: Vec<Hero>,
        bag: Bag,
        seed: u64,
    ) -> Result<Self> {
        use sha2::{Digest, Sha256};
        if heroes.is_empty() || heroes.len() > 4 || heroes.iter().any(|h| !h.state.alive) {
            return Err("invalid campaign expedition party".into());
        }
        let canon = heroes.iter().map(|h| (h.guid, h.base.clone())).collect();
        let spec = Arc::new(Spec {
            sources,
            context,
            canon,
        });
        let mut level = data.levels[1].clone();
        level.map = spec.sources.map.clone();
        level.quest = spec.sources.quest.clone();
        level.pools = spec.sources.pools.clone();
        level.formations.clear();
        data.levels = vec![level];
        data.curios = dd_data::curios013::Curios::load_campaign(&game.overlay)?;
        data.bag_rules = dd_data::inventory015::InventoryRules::load(&game.overlay, &data.curios)?;
        for row in &spec.context.bonuses.stacking {
            let key = row["item"].as_str().ok_or("missing district stack item")?;
            let amount = dd_data::campaign017::uint(row, "amount")?;
            let n = data.bag_rules.extra_limits.entry(key.into()).or_default();
            *n = n.checked_add(amount).ok_or("district stack overflow")?;
        }
        if let Some(loot) = &spec.context.bonuses.loot_darkness {
            data.curios
                .darkness
                .insert("definition".into(), json!({"darkness_bonuses":loot}));
        }
        let map_def = crate::maps014::Definition::new(
            spec.sources.map.clone(),
            data.content.clone(),
            MapCandidates::for_content(&data.content)?,
        )?;
        let mut rules = game.rules.clone();
        let region = spec.sources.quest["dungeon"]
            .as_str()
            .ok_or("missing expedition region")?;
        rules.host_return_token = Some(spec.context.token.clone());
        if let Some(auto) = &spec.context.automation {
            rules.encounter_contract = auto.contract.clone();
        }
        rules.context = Some(game.battle_context(
            region.into(),
            dd_data::build010::Surprise::Neither,
            false,
        )?);
        rules.lighting = Some(game.lighting(100)?);
        if let Some(light) = &spec.context.bonuses.lighting {
            rules.lighting.as_mut().unwrap().table = light.clone();
        }
        if region == "courtyard" {
            data.globals["hallway_stress"] = game.courtyard_walk_rules()?;
            let blood = game.bloodlight()?;
            rules.lighting = Some(dd_data::lighting::Lighting {
                torch: 100,
                table: blood.darkness.clone(),
            });
            rules.bloodlight = Some(blood);
        }
        let id = format!(
            "{PROFILE};sha256:{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&*spec, &data.candidates, &rules, &data.bag_rules))
                    .map_err(|e| e.to_string())?
            )
        );
        let cache = Caches::default();
        let definition = Definition {
            campaign: Some(spec),
            data: Arc::new(data),
            game,
            id: id.clone(),
            maps: vec![map_def.clone()],
            field_rules: vec![rules],
            cache: Arc::new(Mutex::new(cache)),
        };
        let party = heroes.iter().map(|h| h.guid).collect::<Vec<_>>();
        let next_guid = heroes
            .iter()
            .map(|h| h.guid)
            .max()
            .unwrap()
            .checked_add(1)
            .ok_or("identity overflow")?;
        let map =
            crate::maps014::Session::new(map_def.clone(), seed, BTreeMap::new())?.checkpoint()?;
        let entrance = map_def.map().entrance.clone();
        let hunger_buffer =
            definition.data.raid_source["data"]["party"]["inventory"]["hunger_room_buffer"]
                .as_u64()
                .ok_or("missing hunger buffer")? as u32;
        let mut s = Self {
            definition,
            active_map: map_def,
            state: State {
                campaign: Some(Progress::default()),
                profile: PROFILE.into(),
                definition_id: id,
                phase: Phase::Expedition,
                level: 0,
                heroes,
                party: party.clone(),
                provision_party: party,
                bag,
                wallet: BTreeMap::new(),
                map,
                battle: None,
                battle_party: vec![],
                battle_start_supplies: BTreeMap::new(),
                loot: vec![],
                rolled: vec![],
                torch: 100,
                rng: Rng(seed),
                next_guid,
                results: vec![],
                battles: 0,
                interactions: 0,
                steps: 0,
                store: BTreeMap::new(),
                forced_checked: BTreeSet::new(),
                food_uses: BTreeMap::new(),
                overrides: BTreeMap::new(),
                spawn_epoch: 0,
                chosen: BTreeMap::new(),
                limits: BTreeMap::new(),
                hunger_buffer,
                last_room: entrance.clone(),
                encounter_from: entrance,
                examined: BTreeSet::new(),
                encounter_known: false,
            },
        };
        let entry_costs = s
            .state
            .party
            .iter()
            .map(|guid| {
                s.state.heroes[s.hero_index(*guid).unwrap()]
                    .base
                    .mechanics
                    .hero
                    .difficulty
                    .as_ref()
                    .map_or(0, |d| d.entry_stress)
            })
            .collect::<Vec<_>>();
        // Town restores living heroes to full health. Equipment/training/event
        // changes can alter max HP since the previous result; derive the new
        // maximum in the actual regional kernel rather than carry stale HP.
        s.field_with_hp_policy(
            |k| {
                for (i, n) in entry_costs.iter().enumerate() {
                    k.core.units[i].hp = k.core.units[i].stats().hp;
                    k.core.apply_stress_delta(i, *n, &mut vec![]);
                }
                k.settle()
            },
            true,
        )?;
        s.campaign_reveal_layout()?;
        s.campaign_install_additional()?;
        s.campaign_sync_keys()?;
        if s.map()?
            .pending()
            .is_some_and(|p| p.kind == crate::maps014::EventKind::Encounter)
        {
            s.start_battle()?;
        }
        s.validate()?;
        Ok(s)
    }
    pub(super) fn spec(&self) -> Result<&Spec> {
        self.definition
            .campaign
            .as_deref()
            .ok_or("not a campaign expedition".into())
    }

    fn persistent_source_id(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&self.spec()?.sources).map_err(|e| e.to_string())?)
        ))
    }
    fn held_persistent_keys(&self) -> Result<Vec<Stack>> {
        let ids = self
            .active_map
            .map()
            .nodes
            .values()
            .filter_map(|n| n.gate.as_ref())
            .collect::<BTreeSet<_>>();
        let mut out = vec![];
        if !self.state.party.is_empty() {
            for id in ids {
                let item = self.campaign_key_item(id)?;
                let amount = self.state.bag.count(&item);
                if amount > 0 {
                    out.push(Stack { item, amount });
                }
            }
        }
        Ok(out)
    }
    fn persistent_world(&self) -> Result<Option<Persistent>> {
        if !self.active_map.map().persistent
            || self.state.campaign.as_ref().unwrap().terminal
                == Some(crate::events017::ReturnKind::Complete)
        {
            return Ok(None);
        }
        let mut progress = self.state.campaign.as_ref().unwrap().clone();
        progress.physical_bag.clear();
        progress.physical_tray.clear();
        progress.terminal = None;
        progress.progress_before_return = None;
        progress.camp = None;
        progress.camp_selected.clear();
        progress.field_buffs.clear();
        progress.next_camp_buff = 0;
        progress.dream_attempts.clear();
        progress.completed_flashbacks.clear();
        progress.earned_rewards.clear();
        progress.raid_quirks.clear();
        progress.ends_infestation = false;
        progress.auxiliary = None;
        progress.deaths_door.clear();
        progress.baseline_cleared = progress.cleared.len();
        progress.baseline_rooms = self
            .map()?
            .visited()
            .iter()
            .filter(|id| self.active_map.map().nodes[*id].room)
            .count();
        progress.baseline_prisoners = progress.prisoners.clone();
        Ok(Some(Persistent {
            source_id: self.persistent_source_id()?,
            map: self.state.map.clone(),
            progress,
            chosen: self.state.chosen.clone(),
            rng: self.state.rng.clone(),
            rolled: self.state.rolled.clone(),
            last_room: self.state.last_room.clone(),
            encounter_from: self.state.encounter_from.clone(),
            hunger_buffer: self.state.hunger_buffer,
            keys: self.held_persistent_keys()?,
        }))
    }
    pub(crate) fn resume_persistent(&mut self, world: &Persistent) -> Result<()> {
        if !self.active_map.map().persistent || world.source_id != self.persistent_source_id()? {
            return Err("persistent world source/quest mismatch".into());
        }
        world.validate(&self.spec()?.sources, &self.definition.data.content)?;
        let map = crate::maps014::Session::resume_campaign_map(&world.map, &self.active_map)?;
        self.state.map = map.checkpoint()?;
        self.state.campaign = Some(world.progress.clone());
        self.state.chosen = world.chosen.clone();
        self.state.rng = world.rng.clone();
        self.state.rolled = world.rolled.clone();
        self.state.last_room = world.last_room.clone();
        self.state.encounter_from = world.encounter_from.clone();
        self.state.hunger_buffer = world.hunger_buffer;
        self.campaign_sync_keys()?;
        if self
            .map()?
            .pending()
            .is_some_and(|p| p.kind == crate::maps014::EventKind::Encounter)
        {
            self.start_battle()?;
        }
        self.validate()
    }
    pub(super) fn campaign_direct_rewards(&self) -> Vec<Stack> {
        self.state
            .campaign
            .as_ref()
            .map_or_else(Vec::new, |p| p.earned_rewards.clone())
    }
    pub(super) fn validate_campaign(&self) -> Result<()> {
        let Some(spec) = &self.definition.campaign else {
            if self.state.campaign.is_some() || self.state.phase == Phase::Expedition {
                return Err("campaign progress in legacy opening".into());
            }
            return Ok(());
        };
        let p = self
            .state
            .campaign
            .as_ref()
            .ok_or("campaign progress absent")?;
        let rules = &self.definition.field_rules[0];
        if rules.bloodlight.is_some()
            && rules
                .lighting
                .as_ref()
                .is_none_or(|l| self.state.torch != l.torch)
        {
            return Err("campaign bloodlight state differs from fixed source light".into());
        }
        if self.state.level != 0
            || !matches!(self.state.phase, Phase::Expedition | Phase::Finished)
            || (self.state.phase == Phase::Finished) != p.terminal.is_some()
            || self.state.heroes.len() != spec.canon.len()
            || self
                .state
                .heroes
                .iter()
                .any(|h| spec.canon.get(&h.guid) != Some(&h.base))
            || p.cleared
                .iter()
                .chain(&p.activated)
                .chain(&p.prisoners)
                .any(|id| !spec.sources.map.nodes.contains_key(id))
        {
            return Err("invalid bound expedition progress/source".into());
        }
        for hero in &self.state.heroes {
            for item in hero
                .state
                .trinkets
                .iter()
                .chain(hero.state.build.physical_trinkets.iter().flatten())
            {
                self.definition.game.validate_item_growth(item)?;
            }
        }
        let mut quirk_kills = BTreeSet::new();
        for award in &p.raid_quirks {
            let (_, last) = p
                .kills
                .get(award.kill)
                .ok_or("raid quirk lacks native kill")?;
            if !quirk_kills.insert(award.kill)
                || award.choices.is_empty()
                || spec
                    .sources
                    .monster_loot
                    .get(last)
                    .is_none_or(|loot| loot.raid_finish_quirks != award.choices)
            {
                return Err("raid quirk pool differs from original killed-monster loot".into());
            }
        }
        validate_shared_goal_kills020(&spec.sources, p)?;
        self.validate_physical_items()?;
        self.validate_camping()?;
        if p.terminal.is_some() != p.progress_before_return.is_some() {
            return Err("missing/future frozen activity receipt".into());
        }
        if let Some(kind) = p.terminal {
            use crate::events017::ReturnKind;
            if (kind == ReturnKind::Complete && !self.campaign_ready()?)
                || (kind == ReturnKind::Wipe && !self.state.party.is_empty())
                || (kind == ReturnKind::Regroup && !spec.sources.map.persistent)
            {
                return Err("terminal certificate contradicts actual expedition state".into());
            }
        }
        Ok(())
    }
    pub(super) fn curio_accepts_items(&self, node: &str) -> Result<bool> {
        if let Some(spec) = &self.definition.campaign {
            return Ok(spec.sources.curios.get(node).is_some_and(|c| c.full)
                || spec.sources.special.contains_key(node));
        }
        let name = self.active_map.map().nodes[node]
            .content
            .curio
            .as_ref()
            .ok_or("missing curio source")?;
        Ok(self
            .definition
            .data
            .curios
            .prop(dd_data::dson013::hash(name))?
            .1
            .full)
    }
    pub(super) fn campaign_ready(&self) -> Result<bool> {
        let m = self.map()?;
        if m.pending().is_some() || self.state.party.is_empty() {
            return Ok(false);
        }
        let spec = self.spec()?;
        let p = self.state.campaign.as_ref().unwrap();
        for goal in spec.sources.goals.values() {
            let d = &goal["data"];
            let done = match goal["type"].as_str().ok_or("missing quest goal type")? {
                "explore" | "explore_room" => {
                    let rooms = spec
                        .sources
                        .map
                        .nodes
                        .values()
                        .filter(|n| n.room && !n.hidden)
                        .collect::<Vec<_>>();
                    let visited = rooms.iter().filter(|n| m.visited().contains(&n.id)).count();
                    let percentage = d["percentage"]
                        .as_f64()
                        .ok_or("missing exploration percentage")?;
                    let amount = d["amount"].as_u64().unwrap_or(0) as usize;
                    visited >= amount.max((rooms.len() as f64 * percentage).ceil() as usize)
                }
                "battle_room" | "kill_room" => spec
                    .sources
                    .map
                    .nodes
                    .values()
                    .filter(|n| n.room && !n.hidden && n.content.battle.is_some())
                    .all(|n| p.cleared.contains(&n.id)),
                "gather" => {
                    let item = source_stack(&d["item"])?;
                    self.state.bag.count(&item.item) >= item.amount
                }
                "activate" => {
                    let name = d["curio_name"].as_str().ok_or("missing activation curio")?;
                    p.activated
                        .iter()
                        .filter(|id| spec.sources.curio_names.get(*id).is_some_and(|n| n == name))
                        .count()
                        >= dd_data::campaign017::uint(d, "amount")? as usize
                }
                "kill_monster" => {
                    let targets = d["monster_class_ids"]
                        .as_array()
                        .ok_or("missing kill target classes")?;
                    let amount = dd_data::campaign017::uint(d, "amount")? as usize;
                    p.kills
                        .iter()
                        .chain(&p.shared_goal_kills)
                        .filter(|(source, last)| {
                            targets
                                .iter()
                                .any(|v| v.as_str() == Some(source) || v.as_str() == Some(last))
                        })
                        .count()
                        >= amount
                }
                other => return Err(format!("unsupported executable quest goal {other}")),
            };
            if !done {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn campaign_filter_map(&self, o: &mut crate::maps014::Observation) -> Result<()> {
        let Some(spec) = &self.definition.campaign else {
            return Ok(());
        };

        // Gate knowledge is persistent player memory: a gate was visible as
        // soon as a visited node exposed an exit to it. Do not infer anything
        // from unvisited/scouted neighbours or undisclosed links.
        let discovered = o
            .nodes
            .iter()
            .filter(|n| n.visited)
            .flat_map(|n| std::iter::once(n.id.clone()).chain(n.exits.iter().cloned()))
            .collect::<BTreeSet<_>>();
        for node in &mut o.nodes {
            if discovered.contains(&node.id) {
                node.gate_key = spec.sources.map.nodes[&node.id].gate.clone();
            }
            if !node.visited
                && let Some(scopes) = self
                    .state
                    .campaign
                    .as_ref()
                    .unwrap()
                    .partial_scout
                    .get(&node.id)
                && let Some(c) = &mut node.content
            {
                let all = scopes.contains("all");
                c.battle &= all
                    || scopes.contains(if node.room {
                        "room_battles"
                    } else {
                        "hall_battles"
                    });
                c.curio &= all
                    || scopes.contains("curios")
                    || scopes.contains("room_treasures")
                        && spec.sources.map.nodes[&node.id]
                            .content
                            .curio
                            .as_ref()
                            .is_some_and(|s| s.contains("treasure"));
                c.trap &= all || scopes.contains("traps");
                c.obstacle &= all || scopes.contains("obstacles");
                c.objective &= all;
                c.curio_name = None;
            }
            if node.visited
                && let Some(c) = &mut node.content
                && c.curio
            {
                c.curio_name = spec.sources.curio_names.get(&node.id).cloned();
            }
        }
        Ok(())
    }
    fn campaign_install_additional(&mut self) -> Result<()> {
        let mut selected = vec![];
        for i in 0..self.spec()?.sources.pools.entries.len() {
            let entry = &self.spec()?.sources.pools.entries[i];
            if !entry.additional || !self.campaign_pool_eligible(i)? {
                continue;
            }
            let chance = entry.weight;
            if !(0. ..=1.).contains(&chance) {
                return Err("additional encounter chance outside probability".into());
            }
            if self.state.rng.roll(chance) {
                selected.push(i);
            }
        }
        let mut map = self.map()?;
        let mut vacant = self
            .active_map
            .map()
            .nodes
            .values()
            .filter(|n| !n.room && !n.hidden && n.gate.is_none() && n.content.is_empty())
            .map(|n| n.id.clone())
            .collect::<Vec<_>>();
        let mut changed = BTreeMap::new();
        for index in selected {
            if vacant.is_empty() {
                return Err("no empty physical corridor for source additional encounter".into());
            }
            let position = self.state.rng.below(vacant.len() as u64) as usize;
            let node = vacant.remove(position);
            let spawn = Spawn {
                kind: "ac_battle".into(),
                entry: Some(index),
            };
            changed.insert(node.clone(), spawn.content());
            self.state.overrides.insert(node.clone(), spawn);
            self.state.chosen.insert(node, index);
        }
        if !changed.is_empty() {
            self.state.spawn_epoch = self
                .state
                .spawn_epoch
                .checked_add(1)
                .ok_or("content epoch overflow")?;
            map.replace_contents(&changed, self.state.spawn_epoch)?;
            self.active_map = map.definitions();
            self.state.map = map.checkpoint()?;
        }
        Ok(())
    }
    pub(super) fn campaign_count_encounter(&mut self) -> Result<()> {
        if self.state.campaign.as_ref().unwrap().auxiliary.is_none() {
            let node = self.map()?.position().to_string();
            if self.state.examined.insert(node.clone())
                && let Some(index) = self.state.chosen.get(&node)
            {
                let id = self.spec()?.sources.pools.entries[*index].id.clone();
                let n = self.state.limits.entry(id).or_default();
                *n = n.checked_add(1).ok_or("encounter limit overflow")?;
            }
        }
        Ok(())
    }
    pub(super) fn campaign_choose_encounter(&mut self) -> Result<()> {
        if self.state.campaign.as_ref().unwrap().auxiliary.is_some() {
            return Ok(());
        }
        let node = self.map()?.position().to_string();
        if self.spec()?.sources.fixed_formations.contains_key(&node)
            || self.state.chosen.contains_key(&node)
        {
            return Ok(());
        }
        let area = if self.active_map.map().nodes[&node].room {
            "room"
        } else {
            "hall"
        };
        let mut conditional = vec![];
        for (i, e) in self.spec()?.sources.pools.entries.iter().enumerate() {
            if e.conditional && e.area.as_deref() == Some(area) && self.campaign_pool_eligible(i)? {
                if !(0. ..=1.).contains(&e.weight) {
                    return Err("conditional encounter chance outside probability".into());
                }
                conditional.push((i, e.weight));
            }
        }
        let total = conditional.iter().map(|(_, w)| *w).sum::<f64>();
        if total > 1. {
            return Err("conditional replacement probabilities overlap beyond one".into());
        }
        if total > 0. {
            let mut weights = conditional.iter().map(|(_, w)| *w).collect::<Vec<_>>();
            weights.push(1. - total);
            let selected = pick(&mut self.state.rng, &weights)?;
            if let Some((index, _)) = conditional.get(selected) {
                self.state.chosen.insert(node, *index);
                return Ok(());
            }
        }
        let entries = &self.spec()?.sources.pools.entries;
        let mut available = vec![];
        for (i, e) in entries.iter().enumerate() {
            if e.area.as_deref() == Some(area)
                && !e.conditional
                && !e.additional
                && self.campaign_pool_eligible(i)?
            {
                available.push((i, e.weight));
            }
        }
        let i = pick(
            &mut self.state.rng,
            &available.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
        )?;
        self.state.chosen.insert(node, available[i].0);
        Ok(())
    }
    pub(super) fn campaign_pool_eligible(&self, index: usize) -> Result<bool> {
        let spec = self.spec()?;
        let e = &spec.sources.pools.entries[index];
        if e.additional
            && e.monsters
                .iter()
                .any(|id| spec.context.suppressed.contains(id))
        {
            return Ok(false);
        }
        for (k, values) in &e.conditions {
            let num = |i: usize| -> Result<f64> {
                values
                    .get(i)
                    .ok_or("missing encounter condition")?
                    .parse()
                    .map_err(|_| "invalid encounter condition".into())
            };
            let vampires = || {
                self.state
                    .heroes
                    .iter()
                    .filter(|h| {
                        self.state.party.contains(&h.guid)
                            && h.state.build.diseases.iter().any(|id| {
                                self.definition.data.quirk_defs.get(id).is_some_and(|v| {
                                    v["contagious_class_id"] == "disease_vampire_passive"
                                })
                            })
                    })
                    .count() as f64
            };
            let ok = match k.as_str() {
                "darkness_range" => {
                    f64::from(self.state.torch) >= num(0)? && f64::from(self.state.torch) <= num(1)?
                }
                "inventory_valid_item_percent_range" => {
                    let used = self.state.bag.slots.iter().flatten().count() as f64
                        / self.state.bag.slots.len() as f64;
                    used >= num(0)? && used <= num(1)?
                }
                "limit" => f64::from(self.state.limits.get(&e.id).copied().unwrap_or(0)) < num(0)?,
                "min_vampire_heroes" => vampires() >= num(0)?,
                "max_vampire_heroes" => vampires() <= num(0)?,
                "plot_quest_requirement" => {
                    values.iter().all(|v| spec.context.completed.contains(v))
                }
                "infestation_activity_level" => values.contains(&spec.context.activity),
                "random_dungeon_roaming_id" => {
                    values.iter().all(|v| spec.context.roaming.contains(v))
                }
                "completed_flashback_id" => {
                    values.iter().all(|id| spec.context.flashbacks.contains(id))
                }
                "can_be_ambush" => true,
                other => return Err(format!("unknown campaign encounter condition {other}")),
            };
            if !ok {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn campaign_formation(&self, node: &str) -> Result<dd_data::opening015::Formation> {
        let spec = self.spec()?;
        let ids = if let Some(ids) = &self.state.campaign.as_ref().unwrap().auxiliary {
            ids.monsters.clone()
        } else if let Some(ids) = spec.sources.fixed_formations.get(node) {
            ids.clone()
        } else {
            let index = self
                .state
                .chosen
                .get(node)
                .ok_or("unbound concrete encounter")?;
            spec.sources.pools.entries[*index].monsters.clone()
        };
        Ok(dd_data::opening015::Formation {
            monsters: ids
                .iter()
                .map(|id| {
                    spec.sources
                        .catalog
                        .get(id)
                        .cloned()
                        .ok_or_else(|| format!("unbound native monster {id}"))
                })
                .collect::<Result<Vec<_>>>()?,
            loot: vec![],
            sources: vec![node.into()],
        })
    }
    pub(super) fn campaign_stall(
        &mut self,
        context: &mut dd_data::build010::BattleContext,
    ) -> Result<()> {
        let options = self
            .spec()?
            .sources
            .named
            .get("pool:stall")
            .ok_or("missing original stall formations")?
            .clone();
        let i = pick(
            &mut self.state.rng,
            &options.iter().map(|r| r.weight).collect::<Vec<_>>(),
        )?;
        context.reinforcements = options[i]
            .monsters
            .iter()
            .map(|id| {
                self.spec()?
                    .sources
                    .catalog
                    .get(id)
                    .cloned()
                    .ok_or("unbound reinforcement".into())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }
    pub(super) fn campaign_battle_result(
        &mut self,
        result: &crate::BattleResult,
        loot_eligible: &BTreeSet<UnitId>,
        shared_goal_kills: Vec<(String, String)>,
    ) -> Result<()> {
        if self.definition.campaign.is_none() {
            return Ok(());
        }
        self.campaign_item_drops(result)?;
        self.state
            .campaign
            .as_mut()
            .unwrap()
            .shared_goal_kills
            .extend(shared_goal_kills);
        let node = self.map()?.position().to_string();
        let mut draws = vec![];
        for (id, source, last) in &result.killed_monsters {
            self.state
                .campaign
                .as_mut()
                .unwrap()
                .kills
                .push((source.clone(), last.clone()));
            if result.outcome != Outcome::HeroesWin || !loot_eligible.contains(id) {
                continue;
            }
            let loot = self
                .spec()?
                .sources
                .monster_loot
                .get(last)
                .ok_or_else(|| format!("unbound killed monster loot {last}"))?
                .clone();
            draws.extend(loot.draws);
            let p = self.state.campaign.as_mut().unwrap();
            if !loot.raid_finish_quirks.is_empty() {
                p.raid_quirks.push(RaidQuirk {
                    kill: p.kills.len() - 1,
                    choices: loot.raid_finish_quirks,
                });
            }
        }
        let p = self.state.campaign.as_mut().unwrap();
        p.ends_infestation |= result.ends_infestation_sequence;
        if result.outcome == Outcome::HeroesWin && p.auxiliary.is_none() {
            p.cleared.insert(node);
        }
        self.campaign_draws(&draws)
    }
    pub(super) fn campaign_draws(&mut self, draws: &[dd_data::curios013::Draw]) -> Result<()> {
        let stage = self.spec()?.context.infestation.clone();
        let region = self.quest()["dungeon"]
            .as_str()
            .ok_or("missing loot region")?
            .to_string();
        let difficulty = self.quest()["difficulty"]
            .as_i64()
            .ok_or("missing loot difficulty")?;
        let mut rolled = vec![];
        let automation = self.spec()?.context.automation.is_some();
        let mut owned = self.state.rolled.clone();
        if let Some(auto) = &self.spec()?.context.automation {
            owned.extend(auto.owned_trinkets.clone());
        }
        let mut resolver = crate::loot015::Resolver {
            curios: &self.definition.data.curios,
            difficulty,
            dungeon: region,
            hero_classes: self.definition.game.hero_ids().cloned().collect(),
            rng: &mut self.state.rng,
            owned: &owned,
        };
        for draw in draws {
            for _ in 0..draw.count {
                if automation {
                    resolver.automation_loot(&draw.table, &mut vec![], &mut rolled, &stage)?;
                } else {
                    resolver.campaign_loot(&draw.table, &mut vec![], &mut rolled, &stage)?;
                }
            }
        }
        for r in &mut rolled {
            if r.kind == "journal_page_auto" {
                r.kind = "journal_page".into();
                let reward = Stack {
                    item: Item::new(&r.kind, &r.id),
                    amount: r.amount,
                };
                self.inventory_rules()?.definition(&reward.item)?;
                self.state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .earned_rewards
                    .push(reward);
                continue;
            }
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
    pub(super) fn campaign_sync_keys(&mut self) -> Result<()> {
        if self.definition.campaign.is_none() {
            return Ok(());
        }
        let mut keys = BTreeMap::new();
        for key in self
            .active_map
            .map()
            .nodes
            .values()
            .filter_map(|n| n.gate.as_ref())
        {
            let item = self.campaign_key_item(key)?;
            let count = self.state.bag.count(&item);
            if count > 0 {
                keys.insert(key.clone(), count);
            }
        }
        let mut m = self.map()?;
        m.replace_host_keys(keys)?;
        self.state.map = m.checkpoint()?;
        Ok(())
    }
    pub(super) fn campaign_key_item(&self, id: &str) -> Result<Item> {
        let found = self
            .definition
            .data
            .bag_rules
            .items
            .keys()
            .filter_map(|k| k.split_once(':'))
            .filter(|(_, item)| *item == id)
            .collect::<Vec<_>>();
        if found.len() != 1 {
            return Err(format!("missing/ambiguous physical map key {id}"));
        }
        Ok(Item::new(found[0].0, id))
    }
    pub(super) fn campaign_unlock(&mut self, node: &str) -> Result<()> {
        self.spec()?;
        self.campaign_sync_keys()?;
        let key = self
            .active_map
            .map()
            .nodes
            .get(node)
            .and_then(|n| n.gate.clone())
            .ok_or("missing gate key")?;
        let mut m = self.map()?;
        m.act(crate::maps014::Action::Unlock(node.into()))?;
        self.state.bag.consume(&self.campaign_key_item(&key)?, 1)?;
        self.state.map = m.checkpoint()?;
        Ok(())
    }
    pub(super) fn campaign_native(
        &mut self,
        index: usize,
        effect: &dd_data::combat::Effect,
    ) -> Result<()> {
        self.field(|k| {
            k.core
                .apply_external_effect(index, index, effect, &mut vec![]);
            k.settle()
        })
    }
    pub(super) fn campaign_record_door(&mut self, guids: Vec<u64>) {
        if let Some(p) = &mut self.state.campaign {
            p.deaths_door.extend(guids);
        }
    }
    pub(super) fn campaign_curio_can_interact(
        &self,
        node: &str,
        item: Option<&str>,
    ) -> Result<bool> {
        let Some(spec) = &self.definition.campaign else {
            return Ok(true);
        };
        match spec.sources.special.get(node) {
            Some(Special::Prisoner(_)) => Ok(item.is_none()),
            Some(Special::Quest(goals)) => {
                if item.is_some() {
                    return Ok(false);
                }
                for id in goals {
                    let goal = &spec.sources.goals[id];
                    if goal["type"] == "activate" {
                        let starting = goal["starting_items"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default();
                        if starting.len() > 1 {
                            return Err("ambiguous activation item".into());
                        }
                        if let Some(v) = starting.first() {
                            let s = source_stack(v)?;
                            let n = dd_data::campaign017::uint(&goal["data"], "amount")?;
                            if n == 0 || !s.amount.is_multiple_of(n) {
                                return Err("invalid activation item ratio".into());
                            }
                            if self.state.bag.count(&s.item) < s.amount / n {
                                return Ok(false);
                            }
                        }
                    }
                }
                Ok(true)
            }
            None => Ok(true),
        }
    }
    pub(super) fn campaign_torch(&mut self, index: usize) -> Result<()> {
        let definition = self.definition.game.bloodlight_torch()?;
        self.state.bag.consume(&Item::new("supply", "torch"), 1)?;
        self.field(|k| {
            k.core.supplies = vec![dd_data::items::SupplyStack {
                definition,
                remaining: 1,
            }];
            k.core.consume_supply(index, 0, &mut vec![])?;
            k.settle()
        })
    }
    pub(super) fn campaign_trap(&mut self, hero: u64, disarm: bool) -> Result<()> {
        let node = self.map()?.position().to_string();
        let hazard = self
            .spec()?
            .sources
            .traps
            .get(&node)
            .ok_or("unbound trap")?
            .clone();
        let index = self
            .state
            .party
            .iter()
            .position(|id| *id == hero)
            .ok_or("invalid trap hero")?;
        let class = self.state.heroes[self.hero_index(hero)?].base.id.clone();
        let resistance = dd_data::combat::number(
            self.definition
                .game
                .hero_document(&class)?
                .records_keyed("resistances")
                .next()
                .ok_or("missing trap resistance")?,
            "trap",
        )?;
        let difficulty = self.quest()["difficulty"]
            .as_u64()
            .ok_or("missing trap level")? as usize;
        let globals = &self.definition.data.globals;
        let penalty = globals["difficulty_trap_base"][difficulty]
            .as_f64()
            .ok_or("missing trap level penalty")?;
        let bonus = globals["trap_scout_disarm_bonus"]
            .as_f64()
            .ok_or("missing trap scout bonus")?;
        let chance = (resistance + bonus - penalty
            + self.kernel()?.modifier(index, "resistance", "trap"))
        .clamp(0., 1.);
        let success = disarm && self.state.rng.roll(chance);
        if !success {
            self.field(|k| {
                k.hp(
                    index,
                    (f64::from(k.maximum(index)) * hazard.health).round() as i32,
                )
            })?;
            if self.definition.field_rules[0].bloodlight.is_none() {
                self.state.torch = (i32::from(self.state.torch) + hazard.light).clamp(0, 100) as u8;
            }
        }
        for e in if success {
            &hazard.success
        } else {
            &hazard.failure
        } {
            if !self.state.party.contains(&hero) {
                break;
            }
            let index = self.state.party.iter().position(|id| *id == hero).unwrap();
            self.campaign_native(index, e)?;
        }
        self.map_settle()?;
        if !success
            && let Some(area) = hazard.ambush
            && !self.state.party.is_empty()
        {
            self.campaign_aux_pool(&area)?;
        }
        Ok(())
    }
    pub(super) fn campaign_obstacle(&mut self, use_item: bool) -> Result<()> {
        let node = self.map()?.position().to_string();
        let hazard = self
            .spec()?
            .sources
            .obstacles
            .get(&node)
            .ok_or("unbound obstacle")?
            .clone();
        if use_item {
            let item = match hazard.interaction.as_str() {
                "shovel_obstacle" => "shovel",
                "torch_obstacle" => "torch",
                _ => return Err("obstacle does not accept an item".into()),
            };
            self.state.bag.consume(&Item::new("supply", item), 1)?;
        } else if self.definition.field_rules[0].bloodlight.is_none() {
            self.state.torch = (i32::from(self.state.torch) + hazard.light).clamp(0, 100) as u8;
        }
        for hero in self.state.party.clone() {
            let Some(index) = self.state.party.iter().position(|id| *id == hero) else {
                continue;
            };
            if !use_item {
                self.field(|k| {
                    k.hp(
                        index,
                        (f64::from(k.maximum(index)) * hazard.health).round() as i32,
                    )
                })?;
            }
            for e in if use_item {
                &hazard.success
            } else {
                &hazard.failure
            } {
                let Some(index) = self.state.party.iter().position(|id| *id == hero) else {
                    break;
                };
                self.campaign_native(index, e)?;
            }
        }
        if !use_item && hazard.teleport && !self.state.party.is_empty() {
            let mut m = self.map()?;
            let receipt = m
                .pending()
                .cloned()
                .ok_or("missing teleport obstacle receipt")?;
            m.resume(&receipt, crate::maps014::Resolution::Teleport)?;
            self.state.map = m.checkpoint()?;
            if m.pending()
                .is_some_and(|p| p.kind == crate::maps014::EventKind::Encounter)
            {
                self.start_battle()?;
            }
        } else {
            self.map_settle()?;
        }
        Ok(())
    }
    pub(super) fn campaign_aux_pool(&mut self, area: &str) -> Result<()> {
        let mut options = vec![];
        for (i, e) in self.spec()?.sources.pools.entries.iter().enumerate() {
            if e.area.as_deref() == Some(area)
                && !e.conditional
                && !e.additional
                && !e
                    .conditions
                    .get("can_be_ambush")
                    .is_some_and(|v| v == &["false".to_string()])
                && self.campaign_pool_eligible(i)?
            {
                options.push((i, e.weight));
            }
        }
        let i = pick(
            &mut self.state.rng,
            &options.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
        )?;
        let entry = &self.spec()?.sources.pools.entries[options[i].0];
        let monsters = entry.monsters.clone();
        let id = entry.id.clone();
        let used = self.state.limits.entry(id).or_default();
        *used = used
            .checked_add(1)
            .ok_or("auxiliary encounter limit overflow")?;
        self.state.campaign.as_mut().unwrap().auxiliary = Some(Auxiliary {
            monsters,
            kind: AuxKind::Pool {
                index: options[i].0,
                ambush: true,
            },
        });
        self.start_battle()
    }
    pub(super) fn campaign_curio(
        &mut self,
        hero: u64,
        item: Option<String>,
        keep: bool,
    ) -> Result<()> {
        let mut m = self.map()?;
        let node = m.position().to_string();
        if !self.state.party.contains(&hero) {
            return Err("invalid curio hero".into());
        }
        let special = self.spec()?.sources.special.get(&node).cloned();
        let mut summon = None;
        if let Some(special) = special {
            if item.is_some() {
                return Err(
                    "quest and prisoner interactions use their source-defined items automatically"
                        .into(),
                );
            }
            match special {
                Special::Quest(goals) => {
                    for goal in goals {
                        let g = self.spec()?.sources.goals[&goal].clone();
                        match g["type"].as_str() {
                            Some("gather") => {
                                let mut stack = source_stack(&g["data"]["item"])?;
                                // One source item per marked container; the goal amount is the total to retain.
                                let markers = self
                                    .spec()?
                                    .sources
                                    .special
                                    .values()
                                    .filter(
                                        |s| matches!(s, Special::Quest(ids) if ids.contains(&goal)),
                                    )
                                    .count() as u32;
                                if markers == 0 || !stack.amount.is_multiple_of(markers) {
                                    return Err(
                                        "gather amount does not divide source container count"
                                            .into(),
                                    );
                                }
                                stack.amount /= markers;
                                self.state.loot.push(stack);
                            }
                            Some("activate") => {
                                let starting =
                                    g["starting_items"].as_array().cloned().unwrap_or_default();
                                if starting.len() > 1 {
                                    return Err("ambiguous activation item binding".into());
                                }
                                if let Some(v) = starting.first() {
                                    let mut stack = source_stack(v)?;
                                    let amount = dd_data::campaign017::uint(&g["data"], "amount")?;
                                    if amount == 0 || !stack.amount.is_multiple_of(amount) {
                                        return Err(
                                            "activation starting items do not divide goal count"
                                                .into(),
                                        );
                                    }
                                    stack.amount /= amount;
                                    self.state.bag.consume(&stack.item, stack.amount)?;
                                }
                                self.state
                                    .campaign
                                    .as_mut()
                                    .unwrap()
                                    .activated
                                    .insert(node.clone());
                                if self.spec()?.context.automation.is_some()
                                    && let Some(table) = g["data"]["additional_loot"].as_str()
                                {
                                    self.draws(&[dd_data::curios013::Draw {
                                        table: table.into(),
                                        count: 1,
                                    }])?;
                                }
                            }
                            _ => return Err("unsupported quest curio operation".into()),
                        }
                    }
                }
                Special::Prisoner(_) => {
                    self.state
                        .campaign
                        .as_mut()
                        .unwrap()
                        .prisoners
                        .insert(node.clone());
                }
            }
        } else {
            let curio = self
                .spec()?
                .sources
                .curios
                .get(&node)
                .ok_or("unbound curio")?
                .clone();
            let result = if let Some(item) = item {
                if !curio.full {
                    return Err("curio accepts no items".into());
                }
                let Some(result) = curio.items.get(&item) else {
                    return Ok(());
                };
                self.state.bag.consume(&Item::new("supply", &item), 1)?;
                result.clone()
            } else {
                let i = pick(
                    &mut self.state.rng,
                    &curio.outcomes.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
                )?;
                curio.outcomes[i].0.clone()
            };
            let choices = match &result {
                ResultKind::Quirk(r)
                | ResultKind::Purge(r)
                | ResultKind::Disease(r)
                | ResultKind::Scout(r)
                | ResultKind::Summon(r) => Some(r),
                _ => None,
            };
            let chosen = if let Some(rows) = choices {
                let i = pick(
                    &mut self.state.rng,
                    &rows.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
                )?;
                Some(rows[i].0.clone())
            } else {
                None
            };
            match result {
                ResultKind::Basic(CurioOutcome::Nothing) => {}
                ResultKind::Basic(CurioOutcome::Loot {
                    draws,
                    nothing_chance,
                }) => {
                    if nothing_chance == 0. || !self.state.rng.roll(nothing_chance) {
                        self.draws(&draws)?;
                        self.draws(&self.extra_loot(Some(hero))?)?;
                        self.dark_bonus("chest")?;
                    }
                }
                ResultKind::Basic(CurioOutcome::Effect(rows)) => {
                    let i = pick(
                        &mut self.state.rng,
                        &rows.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
                    )?;
                    let effect = self
                        .spec()?
                        .sources
                        .effects
                        .get(&rows[i].0)
                        .ok_or("unbound field effect")?
                        .clone();
                    let index = self.state.party.iter().position(|id| *id == hero).unwrap();
                    self.campaign_native(index, &effect)?;
                }
                ResultKind::Quirk(_) => {
                    self.campaign_quirk(hero, chosen.as_deref().unwrap(), false, false)?
                }
                ResultKind::Purge(_) => {
                    self.campaign_quirk(hero, chosen.as_deref().unwrap(), true, false)?
                }
                ResultKind::Disease(_) => {
                    let id = chosen.as_deref().unwrap();
                    if let Some(effect) = self.spec()?.sources.effects.get(id).cloned() {
                        let index = self.state.party.iter().position(|v| *v == hero).unwrap();
                        self.campaign_native(index, &effect)?;
                    } else {
                        self.campaign_quirk(hero, id, false, true)?;
                    }
                }
                ResultKind::Scout(_) => {
                    summon = Some((false, chosen.unwrap()));
                }
                ResultKind::Summon(_) => {
                    summon = Some((true, chosen.unwrap()));
                }
            }
        }
        m.act(crate::maps014::Action::Interact)?;
        let receipt = m.pending().cloned().ok_or("missing curio receipt")?;
        m.resume(&receipt, crate::maps014::Resolution::Settled)?;
        self.state.map = m.checkpoint()?;
        self.state.interactions += 1;
        self.campaign_curio_bonus(hero, &node)?;
        self.campaign_sync_keys()?;
        if keep {
            self.state.loot.clear();
        }
        if let Some((battle, name)) = summon {
            if battle && !self.state.party.is_empty() {
                let options = self
                    .spec()?
                    .sources
                    .named
                    .get(&name)
                    .ok_or("unbound curio summon")?
                    .clone();
                let i = pick(
                    &mut self.state.rng,
                    &options.iter().map(|r| r.weight).collect::<Vec<_>>(),
                )?;
                self.state.campaign.as_mut().unwrap().auxiliary = Some(Auxiliary {
                    monsters: options[i].monsters.clone(),
                    kind: AuxKind::Curio(name),
                });
                self.start_battle()?;
            } else if !battle {
                self.campaign_curio_scout(&name)?;
            }
        }
        Ok(())
    }
    pub(super) fn campaign_full_scout(&mut self, nodes: &BTreeSet<String>) {
        if let Some(p) = self.state.campaign.as_mut() {
            p.partial_scout.retain(|id, _| !nodes.contains(id));
        }
    }
    pub(super) fn campaign_curio_scout(&mut self, descriptor: &str) -> Result<()> {
        let (range, scope) = descriptor
            .split_once(" - ")
            .ok_or("invalid scout descriptor")?;
        let range = range.parse::<usize>().map_err(|_| "invalid scout range")?;
        let mut m = self.map()?;
        let known = m
            .observe()
            .nodes
            .into_iter()
            .filter(|n| n.content.is_some())
            .map(|n| n.id)
            .collect::<BTreeSet<_>>();
        let graph = self.active_map.map();
        let mut todo = VecDeque::from([(m.position().to_string(), 0)]);
        let mut nodes = BTreeSet::new();
        while let Some((id, n)) = todo.pop_front() {
            if graph.nodes[&id].hidden || nodes.contains(&id) {
                continue;
            }
            let hops = n + usize::from(graph.nodes[&id].room && id != m.position());
            if range != 0 && hops > range {
                continue;
            }
            nodes.insert(id.clone());
            for link in &graph.nodes[&id].links {
                if link.hidden.is_none() {
                    todo.push_back((link.to.clone(), hops));
                }
            }
        }
        m.scout(&nodes, &BTreeSet::new())?;
        if scope == "all" {
            self.campaign_full_scout(&nodes);
        } else {
            for node in nodes.difference(&known) {
                self.state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .partial_scout
                    .entry(node.clone())
                    .or_default()
                    .insert(scope.into());
            }
        }
        self.state.map = m.checkpoint()?;
        Ok(())
    }
    pub(super) fn campaign_quirk(
        &mut self,
        hero: u64,
        request: &str,
        purge: bool,
        disease: bool,
    ) -> Result<()> {
        let index = self.hero_index(hero)?;
        let mut build = self.state.heroes[index].state.build.clone();
        let defs = &self.definition.data.quirk_defs;
        let mut pool = vec![];
        let mut weights = vec![];
        for id in defs.keys() {
            let q = dd_data::library::unique_json(defs, id, "curio quirk")?;
            let exists = build.quirks.contains(id) || build.diseases.contains(id);
            if q["is_disease"].as_bool() != Some(disease) || purge != exists {
                continue;
            }
            if ["positive", "negative"].contains(&request) {
                if q["is_positive"].as_bool() != Some(request == "positive") {
                    continue;
                }
            } else if request != "random" && request != id {
                continue;
            }
            if !purge
                && self.state.heroes[index]
                    .base
                    .mechanics
                    .hero
                    .incompatible_quirks
                    .contains(id)
            {
                continue;
            }
            let weight = if request == "random" || ["positive", "negative"].contains(&request) {
                q["random_chance"]
                    .as_f64()
                    .ok_or("missing random quirk weight")?
            } else {
                1.
            };
            if weight > 0. {
                pool.push(id.clone());
                weights.push(weight);
            }
        }
        if pool.is_empty() {
            return Ok(());
        }
        let chosen = pool[pick(&mut self.state.rng, &weights)?].clone();
        if disease {
            let definition = self
                .definition
                .game
                .disease_catalog()?
                .get(&chosen)
                .ok_or("missing native disease")?
                .clone();
            let index = self
                .state
                .party
                .iter()
                .position(|id| *id == hero)
                .ok_or("disease target absent")?;
            return self.field(|k| {
                if !k.core.units[index].diseases.contains(&definition.id)
                    && !definition
                        .incompatible
                        .iter()
                        .any(|id| k.core.units[index].diseases.contains(id))
                {
                    k.core.contract_disease(index, &definition, &mut vec![]);
                }
                k.settle()
            });
        }
        if purge {
            build.quirks.retain(|id| id != &chosen);
        } else {
            let q = dd_data::library::unique_json(defs, &chosen, "curio quirk")?;
            let sign = q["is_positive"].as_bool().ok_or("missing quirk sign")?;
            let locked = self
                .spec()?
                .context
                .locked_quirks
                .get(&hero)
                .cloned()
                .unwrap_or_default();
            let conflicts = build
                .quirks
                .iter()
                .filter(|old| {
                    q["incompatible_quirks"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|v| v == *old))
                        || dd_data::library::unique_json(defs, old, "quirk").is_ok_and(|r| {
                            r["incompatible_quirks"]
                                .as_array()
                                .is_some_and(|a| a.iter().any(|v| v == &chosen))
                        })
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            if !conflicts.is_disjoint(&locked) {
                return Ok(());
            }
            build.quirks.retain(|id| !conflicts.contains(id));
            let same = build
                .quirks
                .iter()
                .filter(|id| {
                    dd_data::library::unique_json(defs, id, "quirk")
                        .is_ok_and(|r| r["is_positive"].as_bool() == Some(sign))
                })
                .cloned()
                .collect::<Vec<_>>();
            let cap = if sign {
                self.definition.game.rules.quirks_max_positive
            } else {
                self.definition.game.rules.quirks_max_negative
            } as usize;
            if same.len() >= cap {
                let eligible = same
                    .into_iter()
                    .filter(|id| !locked.contains(id))
                    .collect::<Vec<_>>();
                if eligible.is_empty() {
                    return Ok(());
                }
                let remove = &eligible[self.state.rng.below(eligible.len() as u64) as usize];
                build.quirks.retain(|id| id != remove);
            }
            build.quirks.push(chosen);
        }
        self.state.heroes[index].state.build = build;
        self.field(|k| k.settle())
    }
    pub(super) fn campaign_reveal_layout(&mut self) -> Result<()> {
        let Some(spec) = &self.definition.campaign else {
            return Ok(());
        };
        if !spec.sources.map.persistent {
            return Ok(());
        }
        let mut map = self.map()?;
        let graph = self.active_map.map();
        let mut layout = BTreeSet::new();
        for room in map
            .visited()
            .iter()
            .filter(|id| graph.nodes[*id].room && !graph.nodes[*id].hidden)
        {
            let mut todo = VecDeque::from([room.clone()]);
            let mut seen = BTreeSet::new();
            while let Some(id) = todo.pop_front() {
                if !seen.insert(id.clone()) || graph.nodes[&id].hidden {
                    continue;
                }
                layout.insert(id.clone());
                if id != *room && graph.nodes[&id].room {
                    continue;
                }
                for link in &graph.nodes[&id].links {
                    if link.hidden.is_none() {
                        todo.push_back(link.to.clone());
                    }
                }
            }
        }
        map.reveal_layout(&layout)?;
        self.state.map = map.checkpoint()?;
        Ok(())
    }
    pub(super) fn campaign_field_bonuses(&self, kernel: &mut PartyKernel) -> Result<()> {
        if let Some(spec) = &self.definition.campaign {
            for (hero, buffs) in &spec.context.bonuses.field_buffs {
                if let Some(i) = self.state.party.iter().position(|id| id == hero) {
                    for buff in buffs {
                        kernel
                            .core
                            .add_buff(i, buff.clone(), BuffScope::Build, &mut vec![]);
                    }
                }
            }
            kernel.core.refresh_context(false);
        }
        Ok(())
    }
    pub(super) fn campaign_skip_hunger(&mut self) -> Result<()> {
        if self
            .definition
            .campaign
            .as_ref()
            .is_some_and(|s| s.context.bonuses.disable_hunger)
            && self
                .map()?
                .pending()
                .is_some_and(|p| p.kind == crate::maps014::EventKind::Hunger)
        {
            self.map_settle()?;
        }
        Ok(())
    }
    fn campaign_curio_bonus(&mut self, hero: u64, node: &str) -> Result<()> {
        let spec = self.spec()?;
        if !self.state.party.contains(&hero) {
            return Ok(());
        }
        if spec.context.bonuses.curio.is_empty() {
            return Ok(());
        }
        let Some(curio) = spec.sources.curios.get(node) else {
            return Ok(());
        };
        let raw = dd_data::library::unique_json(
            &self.definition.data.curios.types,
            &curio.id,
            "district curio tags",
        )?;
        let rows = raw["rows"]
            .as_array()
            .ok_or("missing district curio tag rows")?;
        let mut tags = BTreeSet::new();
        if let Some(start) = rows.iter().position(|r| r[2] == "TAGS") {
            for row in rows.iter().skip(start + 1) {
                let tag = row[2].as_str().unwrap_or("");
                if tag.is_empty() {
                    break;
                }
                tags.insert(tag);
            }
        }
        let index = self
            .state
            .party
            .iter()
            .position(|id| *id == hero)
            .ok_or("district curio actor absent")?;
        let actor_tags = &self.state.heroes[self.hero_index(hero)?]
            .base
            .mechanics
            .hero
            .tags;
        let mut heal = 0i32;
        for row in &spec.context.bonuses.curio {
            let hero_tags = row["hero_type_tags"]
                .as_array()
                .ok_or("missing district curio hero tags")?;
            let curio_tags = row["curio_tags"]
                .as_array()
                .ok_or("missing district curio tags")?;
            if (hero_tags.is_empty()
                || hero_tags.iter().any(|t| {
                    t.as_str()
                        .is_some_and(|s| actor_tags.iter().any(|v| v == s))
                }))
                && curio_tags
                    .iter()
                    .any(|t| t.as_str().is_some_and(|s| tags.contains(s)))
            {
                heal = heal
                    .checked_add(
                        dd_data::campaign017::uint(row, "stress_heal")?
                            .try_into()
                            .map_err(|_| "district stress overflow")?,
                    )
                    .ok_or("district stress overflow")?;
            }
        }
        if heal > 0 {
            self.field(|k| k.stress(index, -heal))?;
        }
        Ok(())
    }
    pub(super) fn campaign_forced_curio(&mut self) -> Result<()> {
        let m = self.map()?;
        if !m
            .legal_actions()
            .contains(&crate::maps014::Action::Interact)
            || self.spec()?.sources.special.contains_key(m.position())
        {
            return Ok(());
        }
        if !self
            .state
            .forced_checked
            .insert((0, m.position().to_string()))
        {
            return Ok(());
        }
        let curio = self
            .spec()?
            .sources
            .curios
            .get(m.position())
            .ok_or("unbound forced curio")?;
        let raw = dd_data::library::unique_json(
            &self.definition.data.curios.types,
            &curio.id,
            "curio tags",
        )?;
        let rows = raw["rows"].as_array().ok_or("missing curio tag rows")?;
        let mut tags = BTreeSet::new();
        if let Some(start) = rows.iter().position(|r| r[2] == "TAGS") {
            for row in rows.iter().skip(start + 1) {
                let tag = row[2].as_str().unwrap_or("");
                if tag.is_empty() {
                    break;
                }
                tags.insert(tag.to_string());
            }
        }
        for hero in self.state.party.clone() {
            let quirks = self.state.heroes[self.hero_index(hero)?]
                .state
                .build
                .quirks
                .clone();
            for quirk in quirks {
                let q = dd_data::library::unique_json(
                    &self.definition.data.quirk_defs,
                    &quirk,
                    "forced curio quirk",
                )?;
                if let Some(tag) = q["curio_tag"].as_str()
                    && (tag == "All" || tags.contains(tag))
                {
                    let chance = q["curio_tag_chance"]
                        .as_f64()
                        .filter(|n| (0. ..=1.).contains(n))
                        .ok_or("invalid forced curio chance")?;
                    let keep = q["keep_loot"].as_bool().unwrap_or(false);
                    if self.state.rng.roll(chance) {
                        return self.campaign_curio(hero, None, keep);
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn campaign_finish(&mut self, complete: bool) -> Result<()> {
        use crate::events017::ReturnKind;
        if self.state.phase != Phase::Expedition || self.state.battle.is_some() {
            return Err("unsettled expedition cannot return".into());
        }
        if !complete
            && !self.state.party.is_empty()
            && let Some(contract) = self
                .spec()?
                .context
                .automation
                .as_ref()
                .and_then(|a| a.contract.clone())
        {
            if !contract.can_retreat {
                return Err("original quest forbids retreat".into());
            }
            let count = (contract.raid_retreat_deaths as usize)
                .min(self.state.party.len().saturating_sub(1));
            for _ in 0..count {
                let index = self.state.rng.below(self.state.party.len() as u64) as usize;
                self.field(|k| {
                    let mut events = vec![];
                    k.core.units[index].hp = 0;
                    k.core.units[index].life = crate::Life::Dead;
                    k.core.hero_death(index, &mut events);
                    k.core.death_stress(index, &mut events);
                    k.settle()
                })?;
            }
        }
        let kind = if self.state.party.is_empty() {
            ReturnKind::Wipe
        } else if complete {
            if !self.campaign_ready()? {
                return Err("actual quest objectives unmet".into());
            }
            ReturnKind::Complete
        } else if self.active_map.map().persistent {
            ReturnKind::Regroup
        } else {
            ReturnKind::Abandon
        };
        let progress = (
            self.state.campaign.as_ref().unwrap().deaths_door.len() as u32,
            self.campaign_affliction_count()?,
        );
        self.state.campaign.as_mut().unwrap().progress_before_return = Some(progress);
        if matches!(kind, ReturnKind::Abandon | ReturnKind::Regroup) {
            let key = if kind == ReturnKind::Regroup {
                "regroup_penalty"
            } else {
                "fail_penalty"
            };
            let amount = self.spec()?.context.exit[key]["stress_damage"]
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or("missing original return stress penalty")?;
            self.field(|k| {
                for id in 0..k.core.units.len() {
                    k.abandon_stress(id, amount, false)?;
                }
                Ok(())
            })?;
        }
        self.state.campaign.as_mut().unwrap().camp = None;
        self.state.campaign.as_mut().unwrap().terminal = Some(if self.state.party.is_empty() {
            ReturnKind::Wipe
        } else {
            kind
        });
        self.state.phase = Phase::Finished;
        self.state.loot.clear();
        self.campaign_item_clear_tray();
        Ok(())
    }
    fn campaign_affliction_count(&self) -> Result<u32> {
        let context = &self.spec()?.context;
        let mut n = 0u32;
        for h in &self.state.heroes {
            for (id, count) in &h.state.resolve_history {
                let affliction = self
                    .definition
                    .game
                    .rules
                    .globals
                    .as_ref()
                    .is_some_and(|g| {
                        g.traits
                            .iter()
                            .chain(&h.base.mechanics.hero.override_defs)
                            .any(|t| &t.id == id && !t.virtue)
                    });
                if affliction {
                    n = n
                        .checked_add(
                            count.saturating_sub(
                                context
                                    .resolve_history
                                    .get(&h.guid)
                                    .and_then(|m| m.get(id))
                                    .copied()
                                    .unwrap_or(0),
                            ),
                        )
                        .ok_or("activity trait count overflow")?;
                }
            }
        }
        Ok(n)
    }
    pub(crate) fn campaign_return(&self) -> Result<Return> {
        self.validate()?;
        let p = self
            .state
            .campaign
            .as_ref()
            .ok_or("not a campaign sortie")?;
        Ok(Return {
            goal_kills: p
                .kills
                .iter()
                .chain(&p.shared_goal_kills)
                .cloned()
                .collect(),
            survival: 0,
            recovered: if self.state.party.is_empty() {
                vec![]
            } else {
                p.physical_bag.values().cloned().collect()
            },
            token: self.spec()?.context.token.clone(),
            kind: p.terminal.ok_or("expedition still active")?,
            heroes: self
                .state
                .heroes
                .iter()
                .map(|h| (h.guid, h.state.clone()))
                .collect(),
            inventory: if self.state.party.is_empty() {
                vec![]
            } else {
                let keys = if self.active_map.map().persistent {
                    self.held_persistent_keys()?
                } else {
                    vec![]
                };
                self.state
                    .bag
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(slot, _)| !p.physical_bag.contains_key(slot))
                    .filter_map(|(_, s)| s.as_ref())
                    .filter(|s| !keys.iter().any(|k| k.item == s.item))
                    .cloned()
                    .collect()
            },
            battles: p
                .cleared
                .len()
                .checked_sub(p.baseline_cleared)
                .ok_or("invalid clear activity baseline")? as u32,
            rooms: self
                .map()?
                .visited()
                .iter()
                .filter(|id| self.active_map.map().nodes[*id].room)
                .count()
                .checked_sub(p.baseline_rooms)
                .ok_or("invalid room activity baseline")? as u32,
            kills: p.kills.clone(),
            ends_infestation: p.ends_infestation,
            prisoners: p
                .prisoners
                .difference(&p.baseline_prisoners)
                .cloned()
                .collect(),
            raid_quirks: p.raid_quirks.clone(),
            map: self.state.map.clone(),
            persistent: self.persistent_world()?,
            deaths_door: p
                .progress_before_return
                .ok_or("unfrozen activity receipt")?
                .0,
            afflictions: p
                .progress_before_return
                .ok_or("unfrozen activity receipt")?
                .1,
            flashbacks: p.completed_flashbacks.clone(),
            earned_rewards: p.earned_rewards.clone(),
        })
    }
}

fn source_stack(v: &Value) -> Result<Stack> {
    Ok(Stack {
        item: Item::new(
            v["type"].as_str().ok_or("missing objective item kind")?,
            v["id"].as_str().ok_or("missing objective item ID")?,
        ),
        amount: dd_data::campaign017::uint(v, "amount")?,
    })
}
