//! Estate campaign owner. Policies receive Observation, never state or definitions.
#[path = "campaign_auto018.rs"]
pub mod automation;
#[path = "campaign_board017.rs"]
mod board;
#[path = "content019.rs"]
pub mod content019;
#[path = "campaign_effects017.rs"]
mod effects;
#[path = "goals020.rs"]
pub mod goals020;
#[path = "infestation017.rs"]
mod infestation;
#[path = "campaign_retention017.rs"]
mod retention;
#[path = "campaign_schedules017.rs"]
mod schedules;
pub use crate::events017::Failure;
use board::Board;
pub use board::{BoardView, QuestView};
#[path = "campaign_services017.rs"]
mod services;
use super::{Definition as TownDefinition, Session as TownSession, State as EstateState, *};
use crate::events017::{Clock, Facts, Runtime, Selection};
use dd_data::campaign017::{
    Candidates as CampaignCandidates, Data as CampaignData, PROFILE as CAMPAIGN_PROFILE,
};
use effects::Offer;
#[path = "campaign_raid017.rs"]
mod raid;
pub use effects::{EventAction, EventOffer};
pub use raid::{ProvisionAction, Provisions};
pub(super) use services::TownContext;
type TownViewCache = Arc<Mutex<Option<(String, super::Observation)>>>;
#[derive(Clone)]
pub struct Definition {
    goals020: Option<Arc<dd_data::objectives020::Catalog>>,
    content019: Option<Arc<content019::Rules>>,
    town_view: TownViewCache,
    automation: Option<Arc<automation::Rules>>,
    sources: Arc<Mutex<BTreeMap<String, Arc<crate::expedition_sources017::Sources>>>>,
    inventories: Arc<Mutex<BTreeMap<Vec<String>, dd_data::inventory015::InventoryRules>>>,
    raids: Arc<Mutex<BTreeMap<String, crate::opening015::Definition>>>,
    town: TownDefinition,
    data: Arc<CampaignData>,
    id: String,
}
impl Definition {
    pub fn load(game: GameData, candidates: CampaignCandidates) -> Result<Self> {
        let town = TownDefinition::load(game, Candidates::documented()?)?;
        let data = CampaignData::load(&town.game, candidates)?;
        Self::new(town, data)
    }
    fn new(town: TownDefinition, data: CampaignData) -> Result<Self> {
        use sha2::{Digest, Sha256};
        data.candidates.validate()?;
        if data.content.packages.iter().any(|p| p == "crimson_court")
            && (!data.contagion.is_object() || !data.infestation.is_object())
        {
            return Err("missing enabled courtyard campaign sources".into());
        }
        if data.content != town.data.content {
            return Err("campaign/town content differs".into());
        }
        let source = crate::expedition_sources017::fingerprint(&town.game)?;
        let id = format!(
            "{CAMPAIGN_PROFILE};sha256:{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&data, town.id(), &source)).map_err(|e| e.to_string())?
            )
        );
        Ok(Self {
            goals020: None,
            content019: None,
            town_view: Arc::new(Mutex::new(None)),
            automation: None,
            sources: Arc::new(Mutex::new(BTreeMap::new())),
            inventories: Arc::new(Mutex::new(BTreeMap::new())),
            raids: Arc::new(Mutex::new(BTreeMap::new())),
            town,
            data: Arc::new(data),
            id,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn data(&self) -> &CampaignData {
        &self.data
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    goals020: Option<goals020::Owned>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content019: Option<content019::Owned>,
    profile: String,
    definition_id: String,
    city: EstateState,
    preparation: Option<raid::Preparation>,
    active: Option<raid::Active>,
    worlds: BTreeMap<String, crate::expedition017::Persistent>,
    clock: Clock,
    events: Runtime,
    setting: String,
    pending: Option<Selection>,
    visible_event: Option<String>,
    rng: Rng,
    infestation: String,
    infestation_clock: infestation::Clock,
    flashbacks: BTreeSet<String>,
    started_plots: BTreeSet<String>,
    plot_windows: BTreeMap<String, schedules::PlotWindow>,
    graveyard: BTreeMap<u64, Hero>,
    lost_assets: Vec<Asset>,
    asset_returns: Vec<retention::PendingAsset>,
    offer: Option<Offer>,
    board: Board,
    log: Vec<Value>,
}
#[derive(Clone)]
pub struct Session {
    definition: Definition,
    state: State,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Observation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward020: Option<goals020::Progress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content019: Option<content019::View>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation: Option<automation::Progress>,
    pub profile: String,
    pub week: u32,
    pub returns: u32,
    pub completions: u32,
    pub total_deaths: usize,
    pub town: super::Observation,
    pub current_event: Option<String>,
    pub event_choice_pending: bool,
    pub event_offer: Option<EventOffer>,
    pub board: Option<BoardView>,
    pub failure: Option<Failure>,
    pub provisions: Option<Provisions>,
    pub expedition: Option<crate::opening015::Observation>,
}
impl Session {
    pub fn from_opening(definition: Definition, handoff: TownHandoff, seed: u64) -> Result<Self> {
        let town = TownSession::from_opening(definition.town.clone(), handoff, seed)?;
        Self::from_opening_town(definition, town, seed)
    }
    pub fn from_opening_town(definition: Definition, town: TownSession, seed: u64) -> Result<Self> {
        if town.definition.id != definition.town.id
            || town.state.visit != 2
            || town.state.departure.is_some()
            || town.state.campaign.is_some()
        {
            return Err("campaign import requires known completed opening boundary; later history needs explicit migration".into());
        }
        let clock = Clock::opening(
            town.state.visit,
            town.state.completed,
            town.state
                .heroes
                .iter()
                .filter(|h| !h.state.alive)
                .map(|h| h.guid)
                .collect(),
        )?;
        let infestation = if definition
            .data
            .content
            .packages
            .iter()
            .any(|p| p == "crimson_court")
        {
            definition.data.infestation["sequence_elements"][0]["id"]
                .as_str()
                .ok_or("missing initial infestation stage")?
                .into()
        } else {
            String::new()
        };
        let mut s = Self {
            state: State {
                goals020: None,
                content019: definition
                    .content019
                    .as_ref()
                    .map(|_| content019::Owned::default()),
                profile: definition.profile().into(),
                definition_id: definition.id.clone(),
                graveyard: town
                    .state
                    .heroes
                    .iter()
                    .filter(|h| !h.state.alive)
                    .map(|h| (h.guid, h.clone()))
                    .collect(),
                city: town.state,
                preparation: None,
                active: None,
                worlds: BTreeMap::new(),
                clock,
                events: Runtime::default(),
                setting: "normal".into(),
                pending: None,
                visible_event: None,
                rng: Rng(seed),
                infestation,
                infestation_clock: Default::default(),
                flashbacks: BTreeSet::new(),
                started_plots: BTreeSet::new(),
                plot_windows: BTreeMap::new(),
                lost_assets: vec![],
                asset_returns: vec![],
                offer: None,
                board: Board::default(),
                log: vec![],
            },
            definition,
        };
        s.advance_infestation(false)?;
        s.state.city.campaign = Some(TownContext::new(
            s.definition.id.clone(),
            false,
            s.definition.data.contagion.clone(),
        ));
        if let Some(rules) = &s.definition.automation {
            s.state.city.campaign.as_mut().unwrap().automation = Some(automation::Owned {
                rules: (**rules).clone(),
                marked: BTreeSet::new(),
                failure: BTreeMap::new(),
                repeat_completions: BTreeMap::new(),
            });
        }
        let aliases = s
            .state
            .city
            .wallet
            .keys()
            .filter_map(|key| key.strip_prefix("estate_currency:").map(str::to_string))
            .collect::<Vec<_>>();
        for id in aliases {
            let key = s.town().currency(&id)?;
            let old = format!("estate_currency:{id}");
            if old != key {
                let n = s.state.city.wallet.remove(&old).unwrap();
                let v = s.state.city.wallet.entry(key).or_default();
                *v = v.checked_add(n).ok_or("imported currency overflow")?;
            }
        }
        s.init_goals020()?;
        s.process_town_events()?;
        s.validate()?;
        Ok(s)
    }
    fn town(&self) -> TownSession {
        TownSession {
            definition: self.definition.town.clone(),
            state: self.state.city.clone(),
        }
    }
    fn transaction<T>(&mut self, run: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let mut next = self.clone();
        let value = run(&mut next)?;
        next.validate()?;
        *self = next;
        Ok(value)
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(&self.state).map_err(|e| e.to_string())
    }
    pub fn restore(json: &str, definition: &Definition) -> Result<Self> {
        if json.len() > 64 * 1024 * 1024 {
            return Err("campaign checkpoint exceeds bound".into());
        }
        let s = Self {
            definition: definition.clone(),
            state: serde_json::from_str(json).map_err(|e| e.to_string())?,
        };
        s.validate()?;
        Ok(s)
    }
    fn validate(&self) -> Result<()> {
        let s = &self.state;
        if s.profile != self.definition.profile()
            || s.definition_id != self.definition.id
            || s.clock.returns != s.city.visit
            || s.clock.completions != s.city.completed
            || s.clock.week > s.clock.returns
        {
            return Err("invalid campaign profile/source/counter boundary".into());
        }
        if s.city
            .campaign
            .as_ref()
            .is_none_or(|c| c.owner != self.definition.id)
        {
            return Err("campaign service context owner differs".into());
        }
        if s.pending.is_some() != s.offer.is_some() {
            return Err("inconsistent pending event offer".into());
        }
        self.validate_automation()?;
        self.validate_content019()?;
        self.validate_goals020()?;
        self.validate_plot_windows()?;
        self.validate_infestation()?;
        self.validate_offer()?;
        self.validate_asset_returns()?;
        self.validate_raid()?;
        self.validate_board()?;
        let context = s.city.campaign.as_ref().unwrap();
        let mut expected = vec![];
        for id in &context.event_ids {
            expected.extend(
                self.definition.data.event(id)?["data"]
                    .as_array()
                    .ok_or("missing source event effects")?
                    .iter()
                    .map(services::Effect::parse)
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        if context.week.is_some_and(|week| week != s.clock.week) {
            return Err("town service week differs from campaign clock".into());
        }
        if context.contagion != self.definition.data.contagion {
            return Err("campaign contagion source differs".into());
        }
        if expected != context.effects {
            return Err("campaign service effects differ from original event sources".into());
        }
        s.clock.validate()?;
        self.town().validate()?;
        s.events.validate_at(&self.definition.data, s.clock.week)?;
        dd_data::library::unique_json(&self.definition.data.settings, &s.setting, "event setting")?;
        if let Some(p) = &s.pending {
            self.definition.data.event(&p.event)?;
        }
        if let Some(id) = &s.visible_event {
            self.definition.data.event(id)?;
        }
        if !s.infestation.is_empty()
            && !self.definition.data.infestation["sequence_elements"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v["id"] == s.infestation))
        {
            return Err("unknown campaign infestation stage".into());
        }
        if s.clock.dead.iter().any(|id| !s.graveyard.contains_key(id)) {
            return Err("unknown campaign historical death".into());
        }
        Ok(())
    }
    pub fn set_event_setting(&mut self, id: &str) -> Result<()> {
        self.transaction(|s| {
            if s.raid_busy() || s.state.pending.is_some() {
                return Err("event setting requires free town boundary".into());
            }
            library::unique_json(&s.definition.data.settings, id, "event frequency")?;
            s.state.setting = id.into();
            Ok(())
        })
    }
    pub fn failure(&self) -> Result<Option<Failure>> {
        if !["new_game_plus", "bloodmoon"].contains(&self.definition.data.content.mode.as_str()) {
            return Ok(None);
        }
        let rules = &self.definition.town.data.base.globals;
        let limits = crate::events017::Limits {
            week: dd_data::campaign017::uint(rules, "new_game_plus_week_limit")?,
            deaths: dd_data::campaign017::uint(rules, "new_game_plus_hero_death_limit")?,
        };
        Ok(limits.reached(
            self.state.clock.week,
            self.state.clock.total_deaths(),
            self.state
                .city
                .completed_ids
                .contains("plot_darkest_dungeon_4"),
        ))
    }
    pub fn town_action(&mut self, action: super::Action) -> Result<()> {
        self.transaction(|s| {
            if s.failure()?.is_some() {
                return Err("campaign has ended".into());
            }
            if s.raid_busy() {
                return Err(
                    "town actions unavailable during raid preparation or expedition".into(),
                );
            }
            if s.state.pending.is_some() && !Self::during_offer(&action) {
                return Err("resolve town event choice before this estate action".into());
            }
            let mut town = s.town();
            town.act(action)?;
            if let Some(auto) = town
                .state
                .campaign
                .as_mut()
                .and_then(|c| c.automation.as_mut())
            {
                auto.marked
                    .retain(|id| town.state.heroes.iter().any(|h| h.guid == *id));
                auto.failure
                    .retain(|id, _| town.state.heroes.iter().any(|h| h.guid == *id));
            }
            s.state.city = town.state;
            Ok(())
        })
    }
    fn facts(&self) -> Result<Facts> {
        let town = self.town();
        let mut f = Facts {
            open_event_plots: self.state.plot_windows.keys().cloned().collect(),
            week: self.state.clock.week,
            dead_heroes: self
                .state
                .city
                .heroes
                .iter()
                .filter(|h| !h.state.alive)
                .count() as u32,
            stored_trinkets: self.stealable_assets(&town)?.len() as u32,
            finished_plots: self.state.city.completed_ids.clone(),
            infestation: self.state.infestation.clone(),
            available_districts: self.definition.town.data.districts.len() as u32,
            ..Default::default()
        };
        for h in self.state.city.heroes.iter().filter(|h| h.state.alive) {
            *f.hero_levels.entry(town.level(h.xp)?).or_default() += 1;
        }
        for (owner, tree, code) in &self.state.city.purchases {
            if owner.is_none() {
                f.purchases.insert((tree.clone(), code.clone()));
            }
        }
        for b in BUILDINGS {
            if town.unlocked(b)? {
                f.unlocked_buildings.insert(b.into());
            }
        }
        for id in self.definition.data.quests.keys() {
            if self.plot_available(id)? {
                f.available_plots.insert(id.clone());
            }
        }
        Ok(f)
    }
    pub(crate) fn process_town_events(&mut self) -> Result<()> {
        self.transaction(|s| {
            if s.state.pending.is_some() {
                return Err("town event choice already pending".into());
            }
            let f = s.facts()?;
            let event = s.state.events.select(
                &s.definition.data,
                &f,
                &s.state.setting,
                &mut s.state.rng,
            )?;
            let districts = s
                .state
                .city
                .campaign
                .as_ref()
                .is_some_and(|c| c.districts_unlocked);
            let automation = s
                .state
                .city
                .campaign
                .as_mut()
                .and_then(|c| c.automation.take());
            s.state.city.campaign = Some(TownContext::new(
                s.definition.id.clone(),
                districts,
                s.definition.data.contagion.clone(),
            ));
            s.state.city.campaign.as_mut().unwrap().automation = automation;
            s.state.city.campaign.as_mut().unwrap().week = Some(s.state.clock.week);
            s.state.visible_event = event.as_ref().map(|e| e.event.clone());
            if let Some(event) = event {
                s.begin_event(event)?;
            }
            s.advance_infestation(false)?;
            s.refresh_board()?;
            s.refresh_content019()?;
            Ok(())
        })
    }
    pub fn observe(&self) -> Result<Observation> {
        let unavailable = self.failure()?.is_some() || self.raid_busy();
        let mut town = if self.definition.automation.is_some() {
            self.cached_town_view(!unavailable)?
        } else {
            self.town().observe()?
        };
        if unavailable {
            town.actions.clear();
        }
        if self.state.pending.is_some() {
            town.actions
                .retain(|choice| Self::during_offer(&choice.action));
        }
        Ok(Observation {
            steward020: self.steward_progress020()?,
            content019: self.content019_view()?,
            automation: self.automation_progress(),
            profile: self.definition.profile().into(),
            week: self.state.clock.week,
            returns: self.state.clock.returns,
            completions: self.state.clock.completions,
            total_deaths: self.state.clock.total_deaths(),
            town,
            current_event: self.state.visible_event.clone(),
            event_choice_pending: self.state.pending.is_some(),
            event_offer: self.event_offer()?,
            board: if self.failure()?.is_none() {
                self.board_view()?
            } else {
                None
            },
            failure: self.failure()?,
            provisions: self.provision_view()?,
            expedition: self.expedition_view()?,
        })
    }
    fn cached_town_view(&self, include_choices: bool) -> Result<super::Observation> {
        use sha2::{Digest, Sha256};
        let key = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&self.definition.id, &self.state.city, include_choices,))
                    .map_err(|e| e.to_string())?
            )
        );
        if let Some((_, view)) = self
            .definition
            .town_view
            .lock()
            .map_err(|_| "town view cache poisoned")?
            .as_ref()
            .filter(|(saved, _)| saved == &key)
        {
            return Ok(view.clone());
        }
        let view = self.town().observe_with_choices(include_choices)?;
        *self
            .definition
            .town_view
            .lock()
            .map_err(|_| "town view cache poisoned")? = Some((key, view.clone()));
        Ok(view)
    }
}
