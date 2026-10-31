//! Town-event effects and explicit player choices. Applied inside campaign transactions.
use super::services::Effect;
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Offer {
    pub cost: BTreeMap<String, u32>,
    pub dead: Vec<u64>,
    pub recruit: Option<Hero>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventAction {
    Accept,
    Decline,
    Revive(u64),
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EventOffer {
    pub cost: BTreeMap<String, u32>,
    pub dead: Vec<super::super::VisibleHero>,
    pub recruit: Option<super::super::VisibleHero>,
    pub actions: Vec<EventAction>,
}
fn count(e: &Effect) -> Result<u32> {
    if e.amount < 0. || e.amount.fract() != 0. || e.amount > 1000000. {
        Err(format!("invalid event effect count {}", e.kind))
    } else {
        Ok(e.amount as u32)
    }
}
impl Session {
    fn disabled_event_class(&self, id: &str) -> bool {
        let package = match id {
            "musketeer" => return true,
            "duelist" => "duelist",
            "runaway" => "runaway",
            "flagellant" => "flagellant",
            "shieldbreaker" => "shieldbreaker",
            _ => return false,
        };
        !self
            .definition
            .town
            .game
            .overlay
            .config
            .packages
            .iter()
            .any(|p| p == package)
    }
    pub(super) fn during_offer(action: &super::super::Action) -> bool {
        matches!(
            action,
            super::super::Action::Dismiss { .. }
                | super::super::Action::Unequip { .. }
                | super::super::Action::AssignParty { .. }
                | super::super::Action::Party(_)
        )
    }
    pub(super) fn validate_offer(&self) -> Result<()> {
        if let Some(o) = &self.state.offer {
            if o.dead.iter().collect::<BTreeSet<_>>().len() != o.dead.len()
                || o.dead
                    .iter()
                    .any(|id| !self.state.graveyard.contains_key(id))
            {
                return Err("invalid resurrection offer".into());
            }
            if let Some(h) = &o.recruit
                && (h.guid >= self.state.city.next_guid
                    || self.state.city.heroes.iter().any(|old| old.guid == h.guid))
            {
                return Err("invalid paid recruit offer".into());
            }
            let mut expected = BTreeMap::<String, u32>::new();
            for e in self.event_effects(
                &self
                    .state
                    .pending
                    .as_ref()
                    .ok_or("offer without selected event")?
                    .event,
            )? {
                if e.kind == "event_cost" {
                    let key = self.town().currency(&e.id)?;
                    let n = expected.entry(key).or_default();
                    *n = n.checked_add(count(&e)?).ok_or("event cost overflow")?;
                }
            }
            if expected != o.cost {
                return Err("event offer cost differs from source".into());
            }
        }
        let mut assets = self
            .state
            .city
            .assets
            .iter()
            .map(|a| a.instance)
            .collect::<BTreeSet<_>>();
        for a in &self.state.lost_assets {
            if a.owner.is_some()
                || !assets.insert(a.instance)
                || a.instance >= self.state.city.next_asset
            {
                return Err("invalid lost trinket ledger".into());
            }
            self.validate_retention_asset(a)?;
            if self.protected_asset(a)? {
                return Err("protected item in recoverable loss pool".into());
            }
        }
        for (id, h) in &self.state.graveyard {
            if *id != h.guid || h.state.alive || !self.state.clock.dead.contains(id) {
                return Err("invalid graveyard history".into());
            }
        }
        Ok(())
    }
    fn event_effects(&self, id: &str) -> Result<Vec<Effect>> {
        self.definition.data.event(id)?["data"]
            .as_array()
            .ok_or("missing event effects")?
            .iter()
            .map(Effect::parse)
            .collect()
    }
    pub(super) fn event_offer(&self) -> Result<Option<EventOffer>> {
        let Some(o) = &self.state.offer else {
            return Ok(None);
        };
        if self.failure()?.is_some() {
            return Ok(None);
        }
        let town = self.town();
        let mut actions = vec![EventAction::Decline];
        let room =
            town.state.heroes.iter().filter(|h| h.state.alive).count() < town.capacity()? as usize;
        let affordable = o
            .cost
            .iter()
            .all(|(k, n)| town.state.wallet.get(k).copied().unwrap_or(0) >= *n);
        if affordable && (o.recruit.is_none() || room) && o.dead.is_empty() {
            actions.push(EventAction::Accept);
        }
        if affordable {
            actions.extend(
                o.dead
                    .iter()
                    .filter(|id| {
                        (room
                            || town
                                .state
                                .campaign
                                .as_ref()
                                .and_then(|c| c.automation.as_ref())
                                .is_some_and(|a| a.marked.contains(id)))
                            && !town
                                .state
                                .heroes
                                .iter()
                                .any(|h| h.guid == **id && h.state.alive)
                    })
                    .copied()
                    .map(EventAction::Revive),
            );
        }
        Ok(Some(EventOffer {
            cost: o.cost.clone(),
            dead: o
                .dead
                .iter()
                .map(|id| town.visible_hero(&self.state.graveyard[id]))
                .collect::<Result<_>>()?,
            recruit: o
                .recruit
                .as_ref()
                .map(|h| town.visible_hero(h))
                .transpose()?,
            actions,
        }))
    }
    pub fn event_action(&mut self, action: EventAction) -> Result<()> {
        self.transaction(|s| {
            if s.failure()?.is_some() {
                return Err("campaign has ended".into());
            }
            if !s
                .event_offer()?
                .ok_or("no pending town event choice")?
                .actions
                .contains(&action)
            {
                return Err("illegal town event decision".into());
            }
            let selected = s.state.pending.take().ok_or("no pending event")?;
            let offer = s.state.offer.take().ok_or("no pending offer")?;
            if action == EventAction::Decline {
                s.state
                    .log
                    .push(json!({"event":selected.event,"choice":"decline"}));
                s.refresh_board()?;
                return Ok(());
            }
            let mut town = s.town();
            town.charge(&offer.cost)?;
            if let EventAction::Revive(id) = action {
                s.revive(&mut town, id)?;
            }
            if let Some(h) = offer.recruit {
                town.install_hero_purchases(&h)?;
                town.state.heroes.push(h);
            }
            s.state.city = town.state;
            s.apply_event(&selected.event)?;
            s.state
                .log
                .push(json!({"event":selected.event,"choice":action,"cost":offer.cost}));
            s.refresh_board()?;
            Ok(())
        })
    }
    pub(super) fn begin_event(&mut self, selected: Selection) -> Result<()> {
        let effects = self.event_effects(&selected.event)?;
        let mut offer = Offer {
            cost: BTreeMap::new(),
            dead: vec![],
            recruit: None,
        };
        let mut town = self.town();
        for e in &effects {
            match e.kind.as_str() {
                "event_cost" => {
                    let key = town.currency(&e.id)?;
                    let n = offer.cost.entry(key).or_default();
                    *n = n.checked_add(count(e)?).ok_or("event cost overflow")?;
                }
                "dead_recruit" => {
                    let mut ids = self
                        .state
                        .graveyard
                        .keys()
                        .filter(|id| {
                            !town
                                .state
                                .heroes
                                .iter()
                                .any(|h| h.guid == **id && h.state.alive)
                        })
                        .copied()
                        .collect::<Vec<_>>();
                    for i in (1..ids.len()).rev() {
                        let j = self.state.rng.below((i + 1) as u64) as usize;
                        ids.swap(i, j);
                    }
                    ids.truncate(count(e)? as usize);
                    offer.dead = ids;
                    if offer.dead.is_empty() {
                        return Err("resurrection event has no eligible graves".into());
                    }
                }
                "quirked_recruit" => {
                    if offer.recruit.is_some() {
                        return Err("multiple paid recruits in one event".into());
                    }
                    let class = self.random_class(&mut town)?;
                    let mut h = town.generate(&class, count(e)?, 0, 0, 0, 0)?;
                    // Quirks are explicit event content, not a substitute ordinary stagecoach hire.
                    town.install_hero_purchases(&h)?;
                    town.state.heroes.push(h.clone());
                    for id in e.id.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        town.add_quirk(h.guid, id)?;
                    }
                    h = town.state.heroes.pop().ok_or("missing staged recruit")?;
                    town.state
                        .purchases
                        .retain(|(id, _, _)| *id != Some(h.guid));
                    offer.recruit = Some(h);
                }
                _ => {}
            }
        }
        self.state.city = town.state;
        if !offer.cost.is_empty() || !offer.dead.is_empty() || offer.recruit.is_some() {
            self.state.pending = Some(selected);
            self.state.offer = Some(offer);
        } else {
            self.apply_event(&selected.event)?;
        }
        Ok(())
    }
    fn random_class(&self, town: &mut TownSession) -> Result<String> {
        let mut classes = vec![];
        let mut weights = vec![];
        for id in self.definition.town.game.hero_ids() {
            let d = self.definition.town.game.hero_document(id)?;
            let r = d
                .records_keyed("generation")
                .next()
                .ok_or("missing class generation")?;
            let eligible = r.param("town_event_dependency").is_none()
                || town
                    .state
                    .recruit_events
                    .contains(dd_data::combat::text(r, "town_event_dependency")?);
            if eligible && dd_data::combat::text(r, "is_generation_enabled")? == "true" {
                classes.push(id.to_string());
                weights.push(
                    dd_data::combat::number(r, "number_of_cards_in_deck")?
                        * dd_data::combat::number(r, "card_chance")?,
                );
            }
        }
        Ok(classes[town.choose(&weights)?].clone())
    }
    fn revive(&self, town: &mut TownSession, id: u64) -> Result<()> {
        let mut h = self
            .state
            .graveyard
            .get(&id)
            .ok_or("unknown grave")?
            .clone();
        h.weapon = 0;
        h.armour = 0;
        h.missing_until = 0;
        for rank in h.skills.values_mut() {
            *rank = 0;
        }
        h.state.stress = self.definition.data.candidates.parameters["revival_stress"] as i32;
        h.state.resolve = Resolve::Calm;
        h.state.persistent_buffs.clear();
        h.state.status = Default::default();
        // Initial HP comes from the source blueprint. Historical resolve/dungeon identity is retained.
        let blueprint = town.blueprint(&h, 1, false)?;
        let kernel = PartyKernel::new(
            vec![blueprint],
            town.definition.game.rules.clone(),
            town.state.rng.clone(),
        )?;
        let (mut states, rng) = kernel.export();
        town.state.rng = rng;
        h.state = states.remove(0);
        town.state
            .purchases
            .retain(|(owner, _, _)| *owner != Some(id));
        town.install_hero_purchases(&h)?;
        if let Some(i) = town.state.heroes.iter().position(|old| old.guid == id) {
            town.state.heroes[i] = h;
        } else {
            town.state.heroes.push(h);
        }
        Ok(())
    }
    pub(super) fn apply_event(&mut self, id: &str) -> Result<()> {
        let effects = self.event_effects(id)?;
        let mut town = self.town();
        // Unlock recruitment before generating this event's specific offers.
        for class in self.definition.town.game.hero_ids() {
            if self
                .definition
                .town
                .game
                .hero_document(class)?
                .records_keyed("generation")
                .any(|r| dd_data::combat::text(r, "town_event_dependency").ok() == Some(id))
            {
                town.state.recruit_events.insert(id.into());
            }
        }
        for e in &effects {
            if e.kind == "idle_resolve_level" && self.disabled_event_class(&e.id) {
                self.state
                    .log
                    .push(json!({"event":id,"inactive_disabled_class":e.id}));
                continue;
            }
            match e.kind.as_str() {
                "bonus_currency" => {
                    town.credit(&BTreeMap::from([(town.currency(&e.id)?, count(e)?)]))?;
                }
                "bonus_recruit" | "bonus_recruit_stage_coach_level" => {
                    if self.disabled_event_class(&e.id) {
                        self.state
                            .log
                            .push(json!({"event":id,"inactive_disabled_class":e.id}));
                        continue;
                    }
                    self.definition.town.game.hero_document(&e.id)?;
                    let level = if e.kind == "bonus_recruit_stage_coach_level" {
                        count(e)?
                    } else {
                        0
                    };
                    let n = if e.kind == "bonus_recruit" {
                        count(e)?
                    } else {
                        1
                    };
                    for _ in 0..n {
                        let h = town.generate(&e.id, level, 0, 0, 0, 0)?;
                        town.state.recruits.push(h);
                    }
                }
                "stage_coach_bonus_recruits" => {
                    for _ in 0..count(e)? {
                        let class = self.random_class(&mut town)?;
                        let h = town.generate(&class, 0, 0, 0, 0, 0)?;
                        town.state.recruits.push(h);
                    }
                }
                "plot_quest" => {
                    dd_data::library::unique_json(
                        &self.definition.data.quests,
                        &e.id,
                        "event plot quest",
                    )?;
                    self.open_event_plot(&e.id, id)?;
                }
                "trinket_retention_add_from_storage" => {
                    let mut free = self.stealable_assets(&town)?;
                    if free.len() < count(e)? as usize {
                        return Err(
                            "insufficient eligible physical trinkets for source theft".into()
                        );
                    }
                    for i in (1..free.len()).rev() {
                        let j = self.state.rng.below((i + 1) as u64) as usize;
                        free.swap(i, j);
                    }
                    free.truncate(count(e)? as usize);
                    let mut kept = vec![];
                    for a in std::mem::take(&mut town.state.assets) {
                        if free.contains(&a.instance) {
                            self.state.lost_assets.push(a);
                        } else {
                            kept.push(a);
                        }
                    }
                    town.state.assets = kept;
                }
                "remove_vampire" => {
                    for h in town.state.heroes.iter_mut().filter(|h| h.state.alive) {
                        self.cure_crimson(&mut h.state)?;
                    }
                }
                "roster_disease_heal" => {
                    for h in town.state.heroes.iter_mut().filter(|h| h.state.alive) {
                        for _ in 0..count(e)? {
                            if h.state.diseases.is_empty() {
                                break;
                            }
                            let i = town.state.rng.below(h.state.diseases.len() as u64) as usize;
                            let id = h.state.diseases.remove(i);
                            h.state.disease_turns.remove(&id);
                        }
                    }
                }
                "roster_stress_heal" => {
                    let ids = town
                        .state
                        .heroes
                        .iter()
                        .filter(|h| h.state.alive)
                        .map(|h| h.guid)
                        .collect::<Vec<_>>();
                    for id in ids {
                        town.town_stress(id, -(count(e)? as i32))?;
                    }
                }
                "districts_unlocked" => {
                    town.state
                        .campaign
                        .as_mut()
                        .ok_or("missing campaign service context")?
                        .districts_unlocked = true;
                }
                "event_cost" | "dead_recruit" | "quirked_recruit" => {} // Already executed by the atomic explicit choice above.
                "embark_party_buff" | "in_activity_buff" | "idle_buff" => {
                    library::unique_json(
                        &self.definition.town.game.buffs,
                        &e.id,
                        "town event buff",
                    )?;
                }
                "idle_resolve_level" => {
                    self.definition.town.game.hero_document(&e.id)?;
                    count(e)?;
                }
                "idle_negative_quirk_remove" | "upgrade_tag_free" => {
                    count(e)?;
                }
                "activity_lock"
                | "activity_cost_change"
                | "provision_item_type_cost_change"
                | "provision_item_type_amount_change"
                | "upgrade_tag_discount"
                | "free_activity"
                | "remove_quest_hero_level_restriction"
                | "stress_treatment_debuff"
                | "shard_buff"
                | "trinket_sell_multiplier"
                | "heirloom_gain_multiplier"
                | "nomad_wagon_trinket_discount" => {}
                _ => return Err(format!("unsupported town event effect {}", e.kind)),
            }
        }
        town.state
            .campaign
            .as_mut()
            .ok_or("missing event service context")?
            .effects
            .extend(effects);
        town.state
            .campaign
            .as_mut()
            .unwrap()
            .event_ids
            .push(id.into());
        self.state.city = town.state;
        self.state.log.push(json!({"event":id,"applied":true}));
        Ok(())
    }
}
