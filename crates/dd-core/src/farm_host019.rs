//! Farmstead expedition owner. The native 011 controller owns waves and kills;
//! this owner owns provisions, interludes, receipts and the actual town return.
use super::*;
use crate::{Battle, Outcome, Scenario, field015::PartyKernel};
use dd_data::{curios013::Outcome as CurioOutcome, field017::ResultKind, inventory015::Bag};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Active {
    quest: board::Instance,
    battle: String,
    started: bool,
    deaths_door: BTreeSet<usize>,
    afflictions: BTreeSet<usize>,
    bag: Bag,
    loot: Vec<Stack>,
    awards: Vec<Stack>,
    paid_kills: BTreeSet<usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    shared_goal_kills: BTreeMap<usize, (String, String)>,
    thresholds: BTreeSet<u32>,
    used_curios: BTreeSet<(u32, usize)>,
    camp: Option<Camp>,
    camp_buffs: Vec<CampBuff>,
    next_buff: u64,
    raid_quirks: Vec<(usize, Vec<String>)>,
    physical_bag: BTreeMap<usize, crate::expedition017::Carried>,
    physical_tray: BTreeMap<usize, crate::expedition017::Carried>,
    dropped: BTreeSet<usize>,
    initial: Vec<Stack>,
    acquired: Vec<Stack>,
    removed: Vec<Stack>,
    rng: Rng,
}
impl Active {
    fn consume(&mut self, item: &Item, amount: u32) -> Result<()> {
        self.bag.consume(item, amount)?;
        self.removed.push(Stack {
            item: item.clone(),
            amount,
        });
        Ok(())
    }
    fn validate_inventory(&self) -> Result<()> {
        let total = |rows: Vec<&Stack>| -> Result<BTreeMap<Item, u64>> {
            let mut totals = BTreeMap::new();
            for s in rows.into_iter().filter(|s| s.item.kind != "trinket") {
                let n = totals.entry(s.item.clone()).or_insert(0u64);
                *n = n
                    .checked_add(u64::from(s.amount))
                    .ok_or("farm inventory ledger overflow")?;
            }
            Ok(totals)
        };
        let mut expected = total(self.initial.iter().chain(&self.acquired).collect())?;
        for (item, amount) in total(self.removed.iter().collect())? {
            let n = expected
                .get_mut(&item)
                .ok_or("farm removed resource lacks acquisition")?;
            *n = n
                .checked_sub(amount)
                .ok_or("farm removed more resources than acquired")?;
        }
        expected.retain(|_, n| *n > 0);
        let actual = total(self.bag.slots.iter().flatten().chain(&self.loot).collect())?;
        if expected != actual {
            return Err("farm finite inventory conservation failed".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CampBuff {
    instance: u64,
    hero: usize,
    performer: usize,
    skill: String,
    source: dd_data::mechanics::Buff,
    remaining: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Camp {
    meal: bool,
    points: u32,
    spent: u32,
    uses: BTreeMap<u64, BTreeMap<String, u32>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    Start,
    Fight(crate::Action),
    Continue,
    Exit,
    Interact { hero: u64, item: Option<String> },
    Take { index: usize, amount: u32 },
    LeaveLoot,
    Discard { slot: usize, amount: u32 },
    Camp(crate::expedition017::CampAction),
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct View {
    pub quest: String,
    pub battle: crate::Observation,
    pub interlude: Option<InterludeView>,
    pub inventory: Bag,
    pub loot: Vec<Stack>,
    pub earned: Vec<Stack>,
    pub item_states: Vec<crate::expedition017::ItemView>,
    pub camping: Option<crate::expedition017::CampView>,
    pub actions: Vec<Action>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InterludeView {
    pub group: u32,
    pub wave: usize,
    pub curio: String,
    pub group_complete: bool,
}
impl Session {
    fn farm_curios019(&self) -> Result<&dd_data::curios013::Curios> {
        self.definition
            .content019
            .as_ref()
            .and_then(|r| r.farm_curios.as_ref())
            .ok_or("farm curio sources absent".into())
    }

    pub(in super::super) fn farm019_active(&self) -> bool {
        self.state
            .content019
            .as_ref()
            .is_some_and(|s| s.active.is_some())
    }
    fn load_farm019(&self, a: &Active) -> Result<Battle> {
        let b = Battle::load_json_with_content(&a.battle, &self.definition.data.content)?;
        let departure = self
            .state
            .city
            .departure
            .as_ref()
            .ok_or("farm has no departure owner")?;
        let id = a
            .quest
            .plot
            .as_deref()
            .ok_or("farm requires original plot profile")?;
        let plan = self
            .definition
            .town
            .game
            .farm_plan(id, self.state.city.completed_ids.clone())?;
        if b.scenario.rules.farm.as_ref() != Some(&plan)
            || b.scenario.rules.host_return_token.as_deref() != Some(&departure.token)
            || departure.quest.id != id
            || departure.party != self.state.city.party
            || b.scenario.heroes.len() != departure.party.len()
        {
            return Err("farm profile/source/departure mismatch".into());
        }
        Ok(b)
    }
    pub(in super::super) fn validate_farm019(&self) -> Result<()> {
        let Some(owned) = &self.state.content019 else {
            return Ok(());
        };
        for (id, until) in &owned.missing {
            let h = self.town().hero(*id)?.clone();
            if !h.state.alive || h.missing_until != *until {
                return Err("farm missing hero ownership mismatch".into());
            }
        }
        let Some(a) = &owned.active else {
            return Ok(());
        };
        if self.state.active.is_some() || self.state.preparation.is_some() {
            return Err("overlapping farm expedition owners".into());
        }
        let b = self.load_farm019(a)?;
        a.validate_inventory()?;
        a.bag
            .validate(&self.raid_inventory(&self.state.city.party)?)?;
        if a.deaths_door
            .iter()
            .chain(&a.afflictions)
            .any(|id| *id >= self.state.city.party.len())
        {
            return Err("farm progression metrics refer to foreign hero".into());
        }
        if a.paid_kills
            .iter()
            .any(|id| !b.lifecycle.special.killed.contains_key(id))
            || a.shared_goal_kills.iter().any(|(id, (source, last))| {
                b.units.get(*id).is_none_or(|u| {
                    u.alive()
                        || u.source_id != *source
                        || u.blueprint.id != *last
                        || u.blueprint.mechanics.boss.shared_health.is_none()
                }) || a.paid_kills.contains(id)
            })
            || a.thresholds
                .iter()
                .any(|g| *g == 0 || *g > b.lifecycle.special.farm.as_ref().unwrap().group + 1)
            || a.loot.iter().chain(&a.awards).any(|s| s.amount == 0)
            || a.camp.is_some() && b.farm_interlude().is_none()
        {
            return Err("invalid farm reward/camp provenance".into());
        }
        let farm = b.lifecycle.special.farm.as_ref().unwrap();
        let completed =
            farm.group + u32::from(b.farm_interlude().is_some_and(|i| i.group_complete));
        if a.started
            && (a.thresholds != (1..=completed).collect()
                || a.paid_kills != b.lifecycle.special.killed.keys().copied().collect())
            || !a.started
                && (!a.thresholds.is_empty()
                    || !a.paid_kills.is_empty()
                    || !a.awards.is_empty()
                    || !a.used_curios.is_empty()
                    || a.camp.is_some())
        {
            return Err("farm unique receipt ledger differs from native progress".into());
        }
        if let Some(c) = &a.camp {
            let i = b
                .farm_interlude()
                .ok_or("farm camp not at native interlude")?;
            if i.curio != "com_fireplace"
                || !a.used_curios.contains(&(i.group, i.wave))
                || c.points.checked_add(c.spent) != Some(self.farm_camp_points019(&b)?)
            {
                return Err("farm camp points/source identity mismatch".into());
            }
            for (guid, uses) in &c.uses {
                let h = self.town().hero(*guid)?.clone();
                if !self.state.city.party.contains(guid) {
                    return Err("farm camp uses foreign hero".into());
                }
                for (id, n) in uses {
                    let skill = dd_data::camp017::skill(
                        &self.definition.town.game,
                        self.farm_curios019()?,
                        &self.definition.town.data.camps,
                        &h.class,
                        id,
                        "farm",
                        a.quest.raw["difficulty"].as_u64().unwrap() as u32,
                    )?;
                    if *n == 0 || *n > skill.limit || !h.selected_camping.contains(id) {
                        return Err("farm camp skill use exceeds source/selection".into());
                    }
                }
            }
        }
        let mut buff_ids = BTreeSet::new();
        for buff in &a.camp_buffs {
            let guid = *self
                .state
                .city
                .party
                .get(buff.performer)
                .ok_or("farm host buff foreign performer")?;
            if buff.hero >= self.state.city.party.len() {
                return Err("farm host buff foreign recipient".into());
            }
            let hero = self.town().hero(guid)?.clone();
            let mut source_match = false;
            for id in hero.selected_camping.iter().filter(|id| **id == buff.skill) {
                let skill = dd_data::camp017::skill(
                    &self.definition.town.game,
                    self.farm_curios019()?,
                    &self.definition.town.data.camps,
                    &hero.class,
                    id,
                    "farm",
                    a.quest.raw["difficulty"]
                        .as_u64()
                        .ok_or("farm difficulty absent")? as u32,
                )?;
                source_match |= skill.effects.iter().any(|e| matches!(&e.operation,dd_data::camp017::Operation::Buff(b) if **b == buff.source));
            }
            if !source_match
                || buff.instance == 0
                || buff.instance > a.next_buff
                || !buff_ids.insert(buff.instance)
                || buff.remaining == Some(0)
                || (buff.source.duration_type.as_deref() == Some("combat_end")
                    && (buff.remaining.is_none() || buff.remaining > buff.source.duration))
                || (buff.source.duration_type.as_deref() == Some("quest_end")
                    && buff.remaining != buff.source.duration)
            {
                return Err("farm camping buff source/lifetime identity mismatch".into());
            }
        }
        let bag_slots = a
            .bag
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.as_ref().is_some_and(|s| s.item.kind == "trinket"))
            .map(|(i, _)| i)
            .collect::<BTreeSet<_>>();
        let tray_slots = a
            .loot
            .iter()
            .enumerate()
            .filter(|(_, s)| s.item.kind == "trinket")
            .map(|(i, _)| i)
            .collect::<BTreeSet<_>>();
        if bag_slots != a.physical_bag.keys().copied().collect()
            || tray_slots != a.physical_tray.keys().copied().collect()
        {
            return Err("farm finite trinket slots lack physical ownership".into());
        }
        let mut origins = BTreeSet::new();
        for (in_loot, map) in [(false, &a.physical_bag), (true, &a.physical_tray)] {
            for (slot, item) in map {
                self.definition
                    .town
                    .game
                    .validate_item_growth(&item.state)?;
                let stack = if in_loot {
                    a.loot.get(*slot)
                } else {
                    a.bag.slots.get(*slot).and_then(Option::as_ref)
                }
                .ok_or("farm physical asset has no finite slot")?;
                if stack.amount != 1
                    || stack.item.kind != "trinket"
                    || stack.item.id != item.state.definition.id
                    || item.state.definition != self.definition.town.trinket(&stack.item.id)?
                    || item.state.destroyed
                    || item.origin.is_some_and(|id| !origins.insert(id))
                {
                    return Err("farm physical asset slot/source/uniqueness mismatch".into());
                }
            }
        }
        Ok(())
    }
    pub(in super::super) fn embark_farm019(&mut self, p: raid::Preparation) -> Result<()> {
        let id = p.quest.plot.as_deref().ok_or("farm missing plot ID")?;
        let game = self.definition.town.game.clone();
        let plan = game.farm_plan(id, self.state.city.completed_ids.clone())?;
        let catalog = game.entity_catalog(&plan.monster_ids())?;
        let contract = HostQuest {
            id: id.into(),
            content_id: self.definition.data.content.id()?,
            source: format!("campaign019:{}", p.quest.id),
            raw: p.quest.raw.clone(),
            repeatable: p.quest.source["is_repeatable"] == true,
            advances_region: false,
        };
        let mut town = self.town();
        let domains = [
            "field_buffs",
            "curio",
            "camping",
            "hunger",
            "stacking",
            "provision",
            "loot",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let deployment = town.begin_sortie(contract, domains)?;
        let mut rules = town.expedition_rules(&deployment.quest, 100)?;
        rules.host_return_token = Some(deployment.token);
        let mut heroes = deployment.heroes;
        for (id, bp) in deployment.party.iter().zip(&mut heroes) {
            bp.mechanics.hero.host_passive_buffs.extend(
                deployment
                    .bonuses
                    .field_buffs
                    .get(id)
                    .cloned()
                    .unwrap_or_default(),
            );
            game.bind_host_buff_effects(bp)?;
        }
        let scenario = Scenario {
            heroes,
            monsters: vec![],
            rules,
            round_limit: u32::MAX,
        };
        let (supplies, reserves) = self.farm_supplies019(&p.bag, &deployment.bonuses)?;
        let b = Battle::new_farm_with_supplies(
            scenario,
            plan,
            catalog,
            p.quest.seed,
            supplies,
            reserves,
        )?;
        self.state.city = town.state;
        let initial = p.bag.slots.iter().flatten().cloned().collect();
        let mut active = Active {
            quest: p.quest,
            battle: String::new(),
            started: false,
            deaths_door: Default::default(),
            afflictions: Default::default(),
            bag: p.bag,
            loot: vec![],
            awards: vec![],
            paid_kills: Default::default(),
            shared_goal_kills: Default::default(),
            thresholds: Default::default(),
            used_curios: Default::default(),
            camp: None,
            camp_buffs: vec![],
            next_buff: 0,
            raid_quirks: vec![],
            physical_bag: Default::default(),
            physical_tray: Default::default(),
            dropped: Default::default(),
            initial,
            acquired: vec![],
            removed: vec![],
            rng: Rng(self.state.rng.next()),
        };
        self.definition.town.game.validate_farm_start019()?;
        self.farm_curios019()?.validate_farm_stone019()?;
        if b.scenario.rules.farm.as_ref().unwrap().tables["farm"]
            .starting_curio
            .as_deref()
            != Some("com_standingstone")
        {
            return Err("unknown farm starting curio".into());
        }
        // Wave construction is private precomputation. Its initial combat
        // effects and consumables become owned only after the source stone starts it.
        active.battle = b.save_json()?;
        self.state.content019.as_mut().unwrap().active = Some(active);
        Ok(())
    }
    fn farm_supplies019(
        &self,
        bag: &Bag,
        bonuses: &Bonuses,
    ) -> Result<(Vec<dd_data::items::SupplyStack>, BTreeMap<String, u32>)> {
        let mut totals = BTreeMap::<String, u32>::new();
        for stack in bag.slots.iter().flatten().filter(|s| {
            s.item.kind == "supply" || s.item.kind == "estate" && s.item.id == "the_blood"
        }) {
            *totals.entry(stack.item.id.clone()).or_default() += stack.amount;
        }
        let mut supplies = vec![];
        let mut reserves = BTreeMap::new();
        for (id, n) in totals {
            // Fixed farm lighting has no manual torch control. Looted torches
            // remain finite inventory without becoming combat actions.
            if id == "torch" {
                continue;
            }
            let definition = if let Some(d) = bonuses.replacements.get(&id) {
                d.clone()
            } else {
                self.definition.town.game.supply(&id)?
            };
            let remaining = n.min(definition.stack_limit);
            if n > remaining {
                reserves.insert(id, n - remaining);
            }
            supplies.push(dd_data::items::SupplyStack {
                definition,
                remaining,
            });
        }
        Ok((supplies, reserves))
    }
    fn reconcile_farm_supplies019(&self, a: &mut Active, b: &Battle) -> Result<()> {
        for s in &b.supplies {
            let item = Item::new(
                if s.definition.id == "the_blood" {
                    "estate"
                } else {
                    "supply"
                },
                &s.definition.id,
            );
            let remaining = s.remaining
                + b.supply_reserves
                    .get(&s.definition.id)
                    .copied()
                    .unwrap_or(0);
            let before = a.bag.count(&item);
            if remaining > before {
                return Err("farm battle created supplies".into());
            }
            if before > remaining {
                a.consume(&item, before - remaining)?;
            }
        }
        Ok(())
    }
    fn advance_farm019(&self, b: &mut Battle) -> Result<Vec<crate::Event>> {
        let mut events = vec![];
        while b.outcome().is_none() && b.actor.is_some_and(|id| !b.units[id].blueprint.hero) {
            events.extend(b.step_monster()?);
        }
        Ok(events)
    }
    fn farm_progress019(&self, a: &mut Active, b: &Battle, events: &[crate::Event]) -> Result<()> {
        for event in events {
            match event {
                crate::Event::DeathsDoor(id) if *id < self.state.city.party.len() => {
                    a.deaths_door.insert(*id);
                }
                crate::Event::Resolve {
                    target,
                    state: Resolve::Afflicted(_),
                } if *target < self.state.city.party.len() => {
                    a.afflictions.insert(*target);
                }
                _ => {}
            }
        }
        for (id, u) in b.units.iter().enumerate().take(self.state.city.party.len()) {
            if u.hp == 0 {
                a.deaths_door.insert(id);
            }
            if matches!(u.resolve, Resolve::Afflicted(_))
                && !matches!(
                    self.town().hero(self.state.city.party[id])?.state.resolve,
                    Resolve::Afflicted(_)
                )
            {
                a.afflictions.insert(id);
            }
        }
        Ok(())
    }
    fn sync_farm_items019(&self, a: &mut Active, b: &mut Battle) -> Result<()> {
        let mut states = vec![];
        let mut roots = vec![];
        for (slot, stack) in a.bag.slots.iter().enumerate().filter_map(|(i, s)| {
            s.as_ref()
                .filter(|s| s.item.kind == "trinket")
                .map(|s| (i, s))
        }) {
            let state = if let Some(item) = a.physical_bag.get(&slot) {
                item.state.clone()
            } else {
                let definition = self.definition.town.trinket(&stack.item.id)?;
                let item = crate::expedition017::Carried {
                    origin: None,
                    state: TrinketState::fresh(definition),
                };
                a.physical_bag.insert(slot, item.clone());
                item.state
            };
            self.definition.town.game.validate_item_growth(&state)?;
            roots.push(state.definition.id.clone());
            states.push(state);
        }
        let catalog = self.definition.town.game.trinket_catalog(&roots)?;
        for (id, definition) in &catalog {
            if b.lifecycle
                .items
                .catalog
                .get(id)
                .is_some_and(|old| old != definition)
            {
                return Err("farm inventory catalog conflicts with native definition".into());
            }
            b.lifecycle
                .items
                .catalog
                .insert(id.clone(), definition.clone());
        }
        let context = b
            .scenario
            .rules
            .context
            .as_mut()
            .ok_or("farm item context absent")?;
        context.trinket_inventory = states.clone();
        context.trinket_catalog.extend(catalog);
        b.lifecycle.items.bag = states;
        b.refresh_context(false);
        Ok(())
    }
    fn reconcile_farm_items019(&self, a: &mut Active, b: &mut Battle) -> Result<()> {
        let slots = a
            .bag
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.as_ref().is_some_and(|s| s.item.kind == "trinket"))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if b.lifecycle.items.bag.len() < slots.len() {
            return Err("farm native inventory lost source slots".into());
        }
        let native = b.lifecycle.items.bag.clone();
        for (slot, state) in slots.iter().zip(&native) {
            self.definition.town.game.validate_item_growth(state)?;
            if let Some((guid, index)) = a.physical_bag.get(slot).and_then(|item| item.origin) {
                let hero = self
                    .state
                    .city
                    .party
                    .iter()
                    .position(|id| *id == guid)
                    .ok_or("foreign farm recovered asset")?;
                *b.units[hero]
                    .trinkets
                    .get_mut(index)
                    .ok_or("foreign farm recovered asset slot")? = state.clone();
            }
            if state.destroyed {
                a.bag.slots[*slot] = None;
                a.physical_bag.remove(slot);
            } else {
                a.bag.slots[*slot] = Some(Stack {
                    item: Item::new("trinket", &state.definition.id),
                    amount: 1,
                });
                let item = a
                    .physical_bag
                    .get_mut(slot)
                    .ok_or("farm physical item source absent")?;
                item.state = state.clone();
                if let Some((guid, index)) = item.origin {
                    let hero = self
                        .state
                        .city
                        .party
                        .iter()
                        .position(|id| *id == guid)
                        .ok_or("foreign farm recovered asset")?;
                    *b.units[hero]
                        .trinkets
                        .get_mut(index)
                        .ok_or("foreign farm recovered asset slot")? = state.clone();
                }
            }
        }
        for state in native
            .into_iter()
            .skip(slots.len())
            .filter(|s| !s.destroyed)
        {
            let index = a.loot.len();
            a.loot.push(Stack {
                item: Item::new("trinket", &state.definition.id),
                amount: 1,
            });
            a.physical_tray.insert(
                index,
                crate::expedition017::Carried {
                    origin: None,
                    state,
                },
            );
        }
        for (index, hero) in b
            .hero_results()
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.alive && h.death_override.is_none())
        {
            if !a.dropped.insert(index) {
                continue;
            }
            for (state, slot) in hero.trinkets.iter().zip(
                hero.trinket_slots
                    .as_ref()
                    .ok_or("farm dropped asset slots absent")?,
            ) {
                let loot = a.loot.len();
                a.loot.push(Stack {
                    item: Item::new("trinket", &state.definition.id),
                    amount: 1,
                });
                a.physical_tray.insert(
                    loot,
                    crate::expedition017::Carried {
                        origin: Some((self.state.city.party[index], *slot)),
                        state: state.clone(),
                    },
                );
            }
        }
        Ok(())
    }
    fn farm_draw019(&self, a: &mut Active, draws: &[dd_data::curios013::Draw]) -> Result<()> {
        self.farm_draw_in_region019(a, draws, "farm")
    }
    fn farm_draw_in_region019(
        &self,
        a: &mut Active,
        draws: &[dd_data::curios013::Draw],
        region: &str,
    ) -> Result<()> {
        let curios = self.farm_curios019()?;
        let mut owned = self.town().owned_rewards();
        owned.extend(
            a.bag
                .slots
                .iter()
                .flatten()
                .chain(&a.loot)
                .chain(&a.awards)
                .map(|s| crate::initial013::Reward {
                    kind: s.item.kind.clone(),
                    id: s.item.id.clone(),
                    amount: s.amount,
                }),
        );
        let mut resolver = crate::loot015::Resolver {
            curios,
            difficulty: a.quest.raw["difficulty"]
                .as_i64()
                .ok_or("missing farm loot difficulty")?,
            dungeon: region.into(),
            hero_classes: self.definition.town.game.hero_ids().cloned().collect(),
            rng: &mut a.rng,
            owned: &owned,
        };
        let mut rewards = vec![];
        for draw in draws {
            for _ in 0..draw.count {
                resolver.automation_loot(
                    &draw.table,
                    &mut vec![],
                    &mut rewards,
                    &self.state.infestation,
                )?;
            }
        }
        for r in rewards {
            let stack = Stack {
                item: Item::new(&r.kind, &r.id),
                amount: r.amount,
            };
            a.acquired.push(stack.clone());
            if stack.item.kind == "trinket" {
                for _ in 0..stack.amount {
                    let slot = a.loot.len();
                    a.physical_tray.insert(
                        slot,
                        crate::expedition017::Carried {
                            origin: None,
                            state: TrinketState::fresh(
                                self.definition.town.trinket(&stack.item.id)?,
                            ),
                        },
                    );
                    a.loot.push(Stack {
                        item: stack.item.clone(),
                        amount: 1,
                    });
                }
            } else {
                a.loot.push(stack);
            }
        }
        Ok(())
    }
    fn farm_rewards019(&self, a: &mut Active, b: &mut Battle) -> Result<()> {
        let game = &self.definition.town.game;
        let plan = b
            .scenario
            .rules
            .farm
            .as_ref()
            .ok_or("farm missing native plan")?;
        let farm = b
            .lifecycle
            .special
            .farm
            .as_ref()
            .ok_or("farm missing native state")?;
        for (id, source, last) in b.shared_goal_kills020() {
            a.shared_goal_kills.entry(id).or_insert((source, last));
        }
        for (id, (_, last)) in &b.lifecycle.special.killed {
            if !a.paid_kills.insert(*id) {
                continue;
            }
            let u = &b.units[*id];
            if plan.tables[&farm.region].kill_exceptions.contains(last)
                || plan.tables[&farm.region]
                    .kill_exceptions
                    .contains(&u.source_id)
            {
                continue;
            }
            let loot = dd_data::opening_spawn015::campaign_monster_loot(
                game,
                last,
                plan.tier.into(),
                &farm.region,
                self.farm_curios019()?,
            )?;
            if !loot.raid_finish_quirks.is_empty() {
                a.raid_quirks.push((*id, loot.raid_finish_quirks.clone()));
            }
            self.farm_draw_in_region019(a, &loot.draws, &farm.region)?;
            let range = if u
                .blueprint
                .mechanics
                .boss
                .tags
                .iter()
                .any(|tag| tag == "boss")
            {
                plan.params.boss_shard_rewards
            } else if u.blueprint.size > 1 {
                plan.params.double_shard_rewards
            } else {
                plan.params.single_shard_rewards
            };
            let amount = range[0] + a.rng.below(u64::from(range[1] - range[0] + 1)) as u32;
            if amount > 0 {
                a.awards.push(Stack {
                    item: Item::new("shard", ""),
                    amount,
                });
            }
        }
        if let Some(i) = b.farm_interlude().filter(|i| i.group_complete).cloned()
            && a.thresholds.insert(i.group)
        {
            let mut extra = vec![];
            for u in b.units.iter().filter(|u| u.blueprint.hero && u.alive()) {
                extra.extend(
                    self.definition
                        .town
                        .data
                        .base
                        .class_extras
                        .get(&u.blueprint.id)
                        .ok_or("farm class loot source absent")?
                        .battle
                        .clone(),
                );
            }
            self.farm_draw_in_region019(a, &extra, &farm.region)?;
            let mut expired = BTreeSet::new();
            for buff in &mut a.camp_buffs {
                if buff.source.duration_type.as_deref() == Some("combat_end")
                    && let Some(n) = &mut buff.remaining
                {
                    *n = n.saturating_sub(1);
                    if *n == 0 {
                        expired.insert(buff.instance);
                    }
                }
            }
            a.camp_buffs.retain(|b| !expired.contains(&b.instance));
            for bp in &mut b.scenario.heroes {
                bp.mechanics
                    .hero
                    .host_buffs
                    .retain(|h| !expired.contains(&h.instance));
            }
            for unit in &mut b.units {
                unit.buffs
                    .retain(|b| b.host_instance.is_none_or(|id| !expired.contains(&id)));
                unit.blueprint
                    .mechanics
                    .hero
                    .host_buffs
                    .retain(|h| !expired.contains(&h.instance));
            }
            if let Some(rows) = a.quest.raw["threshold_rewards"].as_object() {
                let index = (i.group - 1)
                    .min(rows.len().saturating_sub(1) as u32)
                    .to_string();
                let reward = rows.get(&index).ok_or("missing farm threshold reward")?;
                for row in reward["threshold_loot"]["items"]
                    .as_object()
                    .ok_or("missing threshold loot")?
                    .values()
                {
                    let mut stack = farm_stack(row)?;
                    if stack.item.kind == "shard" {
                        stack.amount = (f64::from(stack.amount)
                            * plan.params.threshold_shard_reward_multiplier)
                            .floor() as u32;
                    }
                    if stack.amount > 0 {
                        a.awards.push(stack);
                    }
                }
                if let Some(trinkets) = reward["additional_threshold_trinket_rewards"].as_object() {
                    for row in trinkets.values() {
                        dd_data::campaign017::fields(
                            row,
                            &["chance_of_loot", "rarity_table"],
                            "farm threshold trinket",
                        )?;
                        let chance = row["chance_of_loot"]
                            .as_f64()
                            .filter(|n| (0. ..=1.).contains(n))
                            .ok_or("invalid threshold trinket chance")?;
                        if !a.rng.roll(chance) {
                            continue;
                        }
                        let choices = row["rarity_table"]
                            .as_object()
                            .ok_or("threshold rarity table absent")?
                            .values()
                            .collect::<Vec<_>>();
                        let weights = choices
                            .iter()
                            .map(|r| {
                                r["chance"]
                                    .as_f64()
                                    .ok_or("invalid threshold rarity weight")
                            })
                            .collect::<std::result::Result<Vec<_>, _>>()?;
                        let chosen = crate::expedition_sources017::pick(&mut a.rng, &weights)?;
                        let rarity = choices[chosen]["rarity"]
                            .as_str()
                            .ok_or("threshold rarity absent")?;
                        let mut owned = self.town().owned_rewards();
                        owned.extend(
                            a.bag
                                .slots
                                .iter()
                                .flatten()
                                .chain(&a.loot)
                                .chain(&a.awards)
                                .map(|s| crate::initial013::Reward {
                                    kind: s.item.kind.clone(),
                                    id: s.item.id.clone(),
                                    amount: s.amount,
                                }),
                        );
                        let mut resolver = crate::loot015::Resolver {
                            curios: self.farm_curios019()?,
                            difficulty: i64::from(plan.tier),
                            dungeon: "farm".into(),
                            hero_classes: self.definition.town.game.hero_ids().cloned().collect(),
                            rng: &mut a.rng,
                            owned: &owned,
                        };
                        if let Some(r) = resolver.trinket_inner(rarity, &[], true)? {
                            a.awards.push(Stack {
                                item: Item::new(&r.kind, &r.id),
                                amount: r.amount,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub(in super::super) fn farm019_view(&self) -> Result<Option<View>> {
        let Some(a) = self
            .state
            .content019
            .as_ref()
            .and_then(|s| s.active.as_ref())
        else {
            return Ok(None);
        };
        let b = self.load_farm019(a)?;
        let mut actions = vec![];
        let camping = self.farm_camp_view019(a, &b)?;
        let contract = &b.scenario.rules.farm.as_ref().unwrap().contract;
        if !a.started {
            actions.push(Action::Start);
            if contract.can_retreat {
                actions.push(Action::Exit);
            }
        } else if !a.loot.is_empty() {
            let rules = self.raid_inventory(&self.state.city.party)?;
            for (index, s) in a.loot.iter().enumerate() {
                let mut bag = a.bag.clone();
                let amount = bag.insert(&rules, s)?;
                if amount > 0 {
                    actions.push(Action::Take { index, amount });
                }
            }
            actions.push(Action::LeaveLoot);
        } else if let Some(camp) = &camping {
            if camp.phase == "meal" {
                for (id, amount, _, _) in &camp.meals {
                    if *amount <= a.bag.count(&Item::new("provision", "")) {
                        actions.push(Action::Camp(crate::expedition017::CampAction::Meal(
                            id.clone(),
                        )));
                    }
                }
            } else {
                actions.push(Action::Camp(crate::expedition017::CampAction::Rest));
                for skill in &camp.skills {
                    if skill.cost > camp.points || skill.remaining == 0 {
                        continue;
                    }
                    if skill.effects.iter().any(|e| e.selection == "individual") {
                        for id in &self.state.city.party {
                            if *id != skill.hero
                                && b.units
                                    [self.state.city.party.iter().position(|g| g == id).unwrap()]
                                .alive()
                            {
                                actions.push(Action::Camp(
                                    crate::expedition017::CampAction::Skill {
                                        hero: skill.hero,
                                        skill: skill.id.clone(),
                                        target: Some(*id),
                                    },
                                ));
                            }
                        }
                    } else {
                        actions.push(Action::Camp(crate::expedition017::CampAction::Skill {
                            hero: skill.hero,
                            skill: skill.id.clone(),
                            target: None,
                        }));
                    }
                }
            }
        } else if let Some(i) = b.farm_interlude() {
            actions.push(Action::Continue);
            if contract.can_retreat {
                actions.push(Action::Exit);
            }
            if !i.curio.is_empty() && !a.used_curios.contains(&(i.group, i.wave)) {
                for (index, guid) in self.state.city.party.iter().enumerate() {
                    if !b.units[index].alive() {
                        continue;
                    }
                    actions.push(Action::Interact {
                        hero: *guid,
                        item: None,
                    });
                    if i.curio != "com_fireplace" {
                        let curio = self.farm_curios019()?.campaign_curio(&i.curio)?;
                        for item in curio.items.keys() {
                            if a.bag.count(&Item::new("supply", item)) > 0 {
                                actions.push(Action::Interact {
                                    hero: *guid,
                                    item: Some(item.clone()),
                                });
                            }
                        }
                    }
                }
            }
        } else if b.outcome().is_none() {
            actions.extend(
                b.observation()
                    .actions
                    .into_iter()
                    .filter(|a| {
                        !matches!(a.action, crate::Action::Retreat)
                            || !contract.retreat_always_from_raid
                    })
                    .map(|a| Action::Fight(a.action)),
            );
        }
        if a.started
            && b.outcome().is_none()
            && contract.can_retreat
            && contract.retreat_always_from_raid
        {
            actions.push(Action::Exit);
        }
        if b.outcome().is_some() && b.outcome() != Some(Outcome::WaveInterlude) {
            actions.push(Action::Exit);
        }
        if b.outcome().is_some() {
            for (slot, s) in a.bag.slots.iter().enumerate() {
                if let Some(s) = s {
                    actions.push(Action::Discard {
                        slot,
                        amount: s.amount,
                    });
                }
            }
        }
        let mut public_battle = if a.started {
            b.observation()
        } else {
            self.farm_entry_kernel019(&b)?.core.observation()
        };
        if contract.retreat_always_from_raid {
            public_battle
                .actions
                .retain(|a| !matches!(a.action, crate::Action::Retreat));
        }
        if !a.started {
            public_battle.units.retain(|u| u.hero);
            public_battle.actor = None;
            public_battle.actions.clear();
            public_battle.round = 0;
            public_battle.wave = 0;
            public_battle.farm = None;
            public_battle.rank_targets.clear();
            public_battle.shared_health.clear();
            public_battle.rank_burns.clear();
        }
        Ok(Some(View {
            quest: a.quest.plot.clone().unwrap(),
            battle: public_battle,
            interlude: if !a.started {
                Some(InterludeView {
                    group: 0,
                    wave: 0,
                    curio: "com_standingstone".into(),
                    group_complete: false,
                })
            } else {
                b.farm_interlude().map(|i| InterludeView {
                    group: i.group,
                    wave: i.wave,
                    curio: i.curio.clone(),
                    group_complete: i.group_complete,
                })
            },
            inventory: a.bag.clone(),
            loot: a.loot.clone(),
            earned: a.awards.clone(),
            item_states: [(false, &a.physical_bag), (true, &a.physical_tray)]
                .into_iter()
                .flat_map(|(in_loot, items)| {
                    items
                        .iter()
                        .map(move |(slot, i)| crate::expedition017::ItemView {
                            in_loot,
                            slot: *slot,
                            remaining: i.state.remaining,
                            quest_remaining: i.state.quest_remaining,
                            growth: i
                                .state
                                .growth
                                .iter()
                                .map(|b| (b.stat_type.clone(), b.stat_sub_type.clone(), b.amount))
                                .collect(),
                        })
                })
                .collect(),
            camping,
            actions,
        }))
    }
    pub fn farm019_action(&mut self, action: Action) -> Result<()> {
        self.transaction(|s| {
            if !s
                .farm019_view()?
                .is_some_and(|v| v.actions.contains(&action))
            {
                return Err("illegal 019 farm phase/action".into());
            }
            let mut a = s
                .state
                .content019
                .as_mut()
                .unwrap()
                .active
                .take()
                .ok_or("no farm expedition")?;
            let mut b = s.load_farm019(&a)?;
            let native_action = matches!(
                &action,
                Action::Start
                    | Action::Fight(_)
                    | Action::Continue
                    | Action::Interact { .. }
                    | Action::Camp(_)
            );
            let supply_before = b
                .supplies
                .iter()
                .map(|i| {
                    (
                        i.definition.id.clone(),
                        i.remaining
                            + b.supply_reserves
                                .get(&i.definition.id)
                                .copied()
                                .unwrap_or(0),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut progress_events = vec![];
            match action {
                Action::Start => {
                    a.started = true;
                    s.reconcile_farm_supplies019(&mut a, &b)?;
                    progress_events.extend(s.advance_farm019(&mut b)?);
                }
                Action::Fight(action) => {
                    progress_events.extend(b.step(action)?);
                    progress_events.extend(s.advance_farm019(&mut b)?);
                }
                Action::Continue => {
                    progress_events.extend(b.continue_farm()?);
                    progress_events.extend(s.advance_farm019(&mut b)?);
                }
                Action::Exit => return s.finish_farm019(a, b),
                Action::Take { index, amount } => {
                    let before = a.bag.clone();
                    let item = a.loot[index].item.clone();
                    if a.bag.insert(
                        &s.raid_inventory(&s.state.city.party)?,
                        &Stack { item, amount },
                    )? != amount
                    {
                        return Err("farm loot does not fit".into());
                    }
                    a.loot[index].amount -= amount;
                    if let Some(item) = a.physical_tray.remove(&index) {
                        let slots = a
                            .bag
                            .slots
                            .iter()
                            .enumerate()
                            .filter(|(slot, stack)| {
                                before.slots[*slot].is_none()
                                    && stack.as_ref().is_some_and(|s| {
                                        s.item.kind == "trinket"
                                            && s.item.id == item.state.definition.id
                                    })
                            })
                            .map(|(i, _)| i)
                            .collect::<Vec<_>>();
                        if slots.len() != 1 || amount != 1 {
                            return Err("farm physical pickup destination ambiguous".into());
                        }
                        a.physical_bag.insert(slots[0], item);
                    }
                    if a.loot[index].amount == 0 {
                        a.loot.remove(index);
                        a.physical_tray = std::mem::take(&mut a.physical_tray)
                            .into_iter()
                            .map(|(i, item)| (if i > index { i - 1 } else { i }, item))
                            .collect();
                    }
                }
                Action::LeaveLoot => {
                    a.removed.extend(a.loot.clone());
                    a.loot.clear();
                    a.physical_tray.clear();
                }
                Action::Discard { slot, amount } => {
                    let item = a.bag.slots[slot]
                        .as_ref()
                        .ok_or("farm discard source absent")?
                        .item
                        .clone();
                    a.bag.discard(slot, amount)?;
                    a.removed.push(Stack { item, amount });
                    a.physical_bag.remove(&slot);
                }
                Action::Interact { hero, item } => s.farm_curio019(&mut a, &mut b, hero, item)?,
                Action::Camp(action) => s.farm_camp019(&mut a, &mut b, action)?,
            }
            if native_action {
                s.farm_progress019(&mut a, &b, &progress_events)?;
                for item in &b.supplies {
                    let before = supply_before
                        .get(&item.definition.id)
                        .copied()
                        .ok_or("farm supply created without source")?;
                    let after = item.remaining
                        + b.supply_reserves
                            .get(&item.definition.id)
                            .copied()
                            .unwrap_or(0);
                    if after > before {
                        return Err("farm supply count increased during native action".into());
                    }
                    if before > after {
                        a.consume(
                            &Item::new(
                                if item.definition.id == "the_blood" {
                                    "estate"
                                } else {
                                    "supply"
                                },
                                &item.definition.id,
                            ),
                            before - after,
                        )?;
                    }
                }
                s.reconcile_farm_items019(&mut a, &mut b)?;
            }
            s.farm_rewards019(&mut a, &mut b)?;
            s.reconcile_farm_camp019(&mut a, &mut b)?;
            let bonuses = &s
                .state
                .city
                .departure
                .as_ref()
                .ok_or("farm supply owner absent")?
                .required;
            let (supplies, reserves) = s.farm_supplies019(&a.bag, bonuses)?;
            b.supplies = supplies;
            b.supply_reserves = reserves;
            s.sync_farm_items019(&mut a, &mut b)?;
            if b.outcome().is_some_and(|o| o != Outcome::WaveInterlude) {
                return s.finish_farm019(a, b);
            }
            a.battle = b.save_json()?;
            s.state.content019.as_mut().unwrap().active = Some(a);
            Ok(())
        })
    }
    fn farm_curio019(
        &self,
        a: &mut Active,
        b: &mut Battle,
        hero: u64,
        item: Option<String>,
    ) -> Result<()> {
        let i = b
            .farm_interlude()
            .cloned()
            .ok_or("farm curio outside interlude")?;
        if !a.used_curios.insert((i.group, i.wave)) {
            return Err("farm curio already consumed".into());
        }
        if i.curio == "com_fireplace" {
            self.farm_curios019()?.farm_fireplace019()?;
            if item.is_some() {
                return Err("farm fireplace has no supply interaction".into());
            }
            let points = self.farm_camp_points019(b)?;
            a.camp = Some(Camp {
                meal: false,
                points,
                spent: 0,
                uses: Default::default(),
            });
            self.farm_curio_bonus019(b, hero, &i.curio)?;
            return Ok(());
        }
        let curio = self.farm_curios019()?.campaign_curio(&i.curio)?;
        let outcome = if let Some(item) = item {
            a.consume(&Item::new("supply", &item), 1)?;
            curio
                .items
                .get(&item)
                .cloned()
                .ok_or("unknown farm item interaction")?
        } else {
            let n = crate::expedition_sources017::pick(
                &mut a.rng,
                &curio.outcomes.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
            )?;
            curio.outcomes[n].0.clone()
        };
        match outcome {
            ResultKind::Basic(CurioOutcome::Nothing) => {}
            ResultKind::Basic(CurioOutcome::Loot {
                draws,
                nothing_chance,
            }) => {
                if nothing_chance == 0. || !a.rng.roll(nothing_chance) {
                    self.farm_draw019(a, &draws)?;
                    let class = self.town().hero(hero)?.class.clone();
                    let extra = &self
                        .definition
                        .town
                        .data
                        .base
                        .class_extras
                        .get(&class)
                        .ok_or("farm class curio loot absent")?
                        .curio;
                    self.farm_draw019(a, extra)?;
                }
            }
            ResultKind::Basic(CurioOutcome::Effect(effects)) => {
                let n = crate::expedition_sources017::pick(
                    &mut a.rng,
                    &effects.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
                )?;
                let effect = self.definition.town.game.field_effect(&effects[n].0)?;
                let id = self
                    .state
                    .city
                    .party
                    .iter()
                    .position(|id| *id == hero)
                    .ok_or("foreign farm curio hero")?;
                let mut k = PartyKernel { core: b.clone() };
                k.core.round = 0;
                k.core.refresh_context(false);
                k.core.apply_external_effect(id, id, &effect, &mut vec![]);
                k.settle()?;
                k.core.round = b.round;
                k.core.refresh_context(false);
                *b = k.core;
            }
            _ => return Err("unknown enabled farm curio mechanism".into()),
        }
        self.farm_curio_bonus019(b, hero, &i.curio)?;
        Ok(())
    }
    fn farm_curio_bonus019(&self, b: &mut Battle, hero: u64, curio: &str) -> Result<()> {
        let index = self
            .state
            .city
            .party
            .iter()
            .position(|id| *id == hero)
            .ok_or("farm curio bonus actor absent")?;
        let raw = library::unique_json(&self.farm_curios019()?.types, curio, "farm curio tags")?;
        let rows = raw["rows"].as_array().ok_or("farm curio tag rows absent")?;
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
        let mut heal = 0i32;
        for row in &self
            .state
            .city
            .departure
            .as_ref()
            .ok_or("farm bonus owner absent")?
            .required
            .curio
        {
            let heroes = row["hero_type_tags"]
                .as_array()
                .ok_or("farm curio hero tags absent")?;
            let curios = row["curio_tags"]
                .as_array()
                .ok_or("farm curio tags absent")?;
            if (heroes.is_empty()
                || heroes.iter().any(|t| {
                    t.as_str().is_some_and(|s| {
                        b.units[index]
                            .blueprint
                            .mechanics
                            .hero
                            .tags
                            .iter()
                            .any(|v| v == s)
                    })
                }))
                && curios
                    .iter()
                    .any(|t| t.as_str().is_some_and(|s| tags.contains(s)))
            {
                heal = heal
                    .checked_add(
                        dd_data::campaign017::uint(row, "stress_heal")?
                            .try_into()
                            .map_err(|_| "farm curio stress overflow")?,
                    )
                    .ok_or("farm curio stress overflow")?;
            }
        }
        if heal > 0 {
            let mut k = PartyKernel { core: b.clone() };
            k.core.round = 0;
            k.core.refresh_context(false);
            k.stress(index, -heal)?;
            k.core.round = b.round;
            k.core.refresh_context(false);
            *b = k.core;
        }
        Ok(())
    }
    fn finish_farm019(&mut self, mut a: Active, mut b: Battle) -> Result<()> {
        use crate::events017::ReturnKind;
        let plan = b.scenario.rules.farm.as_ref().ok_or("farm plan absent")?;
        let farm = b
            .lifecycle
            .special
            .farm
            .as_ref()
            .ok_or("farm runtime absent")?;
        if b.farm_interlude().is_some_and(|i| i.group_complete)
            && plan.params.recurring_wave_groups.is_empty()
            && farm.group as usize + 1 == plan.params.intro_wave_groups.len()
        {
            // A return after the actual final objective must settle the native
            // finite controller, rather than mislabel an achieved goal as abandon.
            b.continue_farm()?;
        }
        let native = b.outcome();
        if native.is_none() && a.started {
            let c = &b
                .scenario
                .rules
                .farm
                .as_ref()
                .ok_or("farm plan absent")?
                .contract;
            if !c.can_retreat || !c.retreat_always_from_raid {
                return Err(
                    "farm exit requires source unconditional raid retreat permission".into(),
                );
            }
            b.cleanup_captives(&mut vec![]);
            b.cleanup_control(&mut vec![]);
            b.refresh_context(false);
        }
        if a.started
            && native.is_some()
            && !matches!(
                native,
                Some(
                    Outcome::HeroesWin
                        | Outcome::MonstersWin
                        | Outcome::MutualDefeat
                        | Outcome::Retreated
                        | Outcome::WaveInterlude
                )
            )
        {
            return Err("unsupported farm terminal outcome is not completion".into());
        }
        if native == Some(Outcome::HeroesWin)
            && !b
                .lifecycle
                .special
                .farm
                .as_ref()
                .ok_or("farm runtime absent")?
                .complete
        {
            return Err("farm victory lacks native finite objective completion".into());
        }
        let finite_complete = native == Some(Outcome::HeroesWin);
        if finite_complete && a.quest.plot.as_deref() == Some("plot_color_of_madness_2") {
            return Err("endless farm cannot complete from a test budget".into());
        }
        if a.started {
            if native.is_some() {
                b.result()?;
            }
            self.farm_progress019(&mut a, &b, &[])?;
        }
        let mut heroes = if a.started {
            b.hero_results()
        } else {
            self.farm_entry_kernel019(&b)?.export().0
        };
        let killed_monsters = if a.started {
            b.lifecycle
                .special
                .killed
                .iter()
                .map(|(id, (source, variant))| (*id, source.clone(), variant.clone()))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let complete = finite_complete
            || a.quest
                .raw
                .get("completion_threshold")
                .and_then(Value::as_u64)
                .is_some_and(|threshold| {
                    a.thresholds
                        .iter()
                        .max()
                        .is_some_and(|group| u64::from(*group) > threshold)
                })
                && heroes.iter().any(|h| h.alive);
        let party = self.state.city.party.clone();
        let lost = self
            .definition
            .content019
            .as_ref()
            .and_then(|r| r.sources.as_ref())
            .ok_or("farm lost rules absent")?
            .lost
            .clone();
        let mut missing = vec![];
        let original_survivors = party
            .iter()
            .copied()
            .zip(&heroes)
            .filter(|(_, h)| h.alive)
            .map(|(id, _)| id)
            .collect::<BTreeSet<_>>();
        let mut reward_kernel = PartyKernel { core: b.clone() };
        reward_kernel.core.round = 0;
        self.farm_noncombat_buffs019(&a, &mut reward_kernel)?;
        let mut xp_awards = vec![];
        for (id, h) in party.iter().zip(&mut heroes) {
            if !h.alive && h.death_override.as_deref() == Some("in_time_and_space") {
                self.town().campaign_death_quirks(h)?;
                h.trinkets.clear();
                h.trinket_slots = Some(vec![]);
                h.alive = true;
                h.hp = h.max_hp;
                h.stress = lost.in_time_and_space_stress as i32;
                h.resolve = Resolve::Afflicted(lost.in_time_and_space_affliction_id.clone());
                h.death_override = None;
                missing.push(*id);
            }
        }
        let source = &a.quest.raw["threshold_rewards"];
        // Threshold identities are native completed groups. Return rewards are
        // separate from interlude loot and cannot be claimed a second time.
        if let Some(rows) = source.as_object()
            && let Some(group) = a
                .thresholds
                .iter()
                .max()
                .filter(|_| !original_survivors.is_empty())
        {
            let key = (group - 1)
                .min(rows.len().saturating_sub(1) as u32)
                .to_string();
            let reward = rows.get(&key).ok_or("threshold return source missing")?;
            let base = dd_data::campaign017::uint(reward, "resolve_xp")?;
            for guid in &original_survivors {
                let index = party
                    .iter()
                    .position(|id| id == guid)
                    .ok_or("farm reward hero absent")?;
                let bonus = self.town().experience_modifier(*guid)?
                    + reward_kernel.modifier(index, "resolve_xp_bonus_percent", "")
                    + self
                        .state
                        .city
                        .departure
                        .as_ref()
                        .unwrap()
                        .required
                        .field_buffs
                        .get(guid)
                        .into_iter()
                        .flatten()
                        .filter(|b| {
                            b.stat_type == "resolve_xp_bonus_percent"
                                && (b.rule_type != "in_dungeon" || b.rule_data.string == "farm")
                        })
                        .map(|b| b.amount)
                        .sum::<f64>();
                xp_awards.push((
                    *guid,
                    (f64::from(base) * (1. + bonus).max(0.)).round() as u32,
                ));
            }
            for row in reward["items_definition"]["items"]
                .as_object()
                .ok_or("threshold return items missing")?
                .values()
            {
                let stack = farm_stack(row)?;
                if stack.amount > 0 {
                    a.awards.push(stack);
                }
            }
        }
        let mut consume = 0.;
        let mut gain = 0.;
        for h in &heroes {
            gain += reward_kernel.modifier(h.id, "shard_reward_percent", "");
            consume += reward_kernel.modifier(h.id, "shard_consume_percent", "");
            gain += self
                .definition
                .content019
                .as_ref()
                .ok_or("farm shard owner absent")?
                .class_shards
                .get(&h.class)
                .copied()
                .ok_or("farm class shard consumer missing")?;
            let mut names = vec![];
            for id in h.build.quirks.iter().chain(&h.diseases) {
                let raw = library::unique_json(
                    &self.definition.town.data.base.quirk_defs,
                    id,
                    "farm shard quirk",
                )?;
                names.extend(
                    raw["buffs"]
                        .as_array()
                        .ok_or("farm quirk buffs absent")?
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or("invalid farm quirk buff")
                                .map(str::to_string)
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()?,
                );
            }
            for t in &h.trinkets {
                if !t.destroyed {
                    names.extend(t.definition.noncombat_buffs.clone());
                }
            }
            for id in names {
                let raw = library::unique_json(
                    &self.definition.town.game.buffs,
                    &id,
                    "farm shard modifier",
                )?;
                if !["shard_consume_percent", "shard_reward_percent"]
                    .iter()
                    .any(|s| raw["stat_type"] == *s)
                {
                    continue;
                }
                if raw["rule_type"] != "always"
                    || raw["is_false_rule"] != false
                    || raw["stat_sub_type"] != ""
                {
                    return Err("unknown enabled shard modifier rule".into());
                }
                let n = raw["amount"]
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or("invalid shard modifier amount")?;
                if raw["stat_type"] == "shard_consume_percent" {
                    consume += n;
                } else {
                    gain += n;
                }
            }
        }
        let total = a
            .awards
            .iter()
            .filter(|s| s.item.kind == "shard")
            .map(|s| u64::from(s.amount))
            .sum::<u64>()
            + u64::from(a.bag.count(&Item::new("shard", "")));
        for slot in &mut a.bag.slots {
            if slot.as_ref().is_some_and(|s| s.item.kind == "shard") {
                *slot = None;
            }
        }
        a.awards.retain(|s| s.item.kind != "shard");
        let amount = (total as f64 * (1. + gain).max(0.) * (1. - consume).max(0.)).floor();
        if !amount.is_finite() || amount > f64::from(u32::MAX) {
            return Err("farm shard settlement overflow".into());
        }
        if amount > 0. {
            a.awards.push(Stack {
                item: Item::new("shard", ""),
                amount: amount as u32,
            });
        }
        let token = self
            .state
            .city
            .departure
            .as_ref()
            .ok_or("farm missing return token")?
            .token
            .clone();
        let all_dead = original_survivors.is_empty();
        if all_dead {
            a.bag.slots.fill(None);
            a.physical_bag.clear();
            a.awards
                .retain(|s| s.item.kind == "shard" || s.item.id == "memory");
        }
        let until = self
            .state
            .clock
            .week
            .checked_add(lost.in_time_and_space_missing_duration)
            .ok_or("farm missing duration overflow")?;
        for id in &missing {
            let index = self.town().hero_index(*id)?;
            self.state.city.heroes[index].missing_until = until;
            self.state
                .content019
                .as_mut()
                .unwrap()
                .missing
                .insert(*id, until);
        }
        let regenerate = a.quest.source["regenerate_quests_on_non_town_progression"] == true;
        let raid_quirks = a
            .raid_quirks
            .iter()
            .map(|(id, choices)| {
                Ok(crate::expedition017::RaidQuirk {
                    kill: killed_monsters
                        .iter()
                        .position(|(key, _, _)| key == id)
                        .ok_or("farm raid quirk native kill absent")?,
                    choices: choices.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let receipt = crate::expedition017::Return {
            goal_kills: killed_monsters
                .iter()
                .map(|(_, source, last)| (source.clone(), last.clone()))
                .chain(a.shared_goal_kills.values().cloned())
                .collect(),
            token,
            kind: if complete {
                ReturnKind::Complete
            } else if all_dead {
                ReturnKind::Wipe
            } else {
                ReturnKind::Abandon
            },
            heroes: party.iter().copied().zip(heroes).collect(),
            inventory: a
                .bag
                .slots
                .into_iter()
                .flatten()
                .filter(|s| s.item.kind != "trinket")
                .collect(),
            battles: a.thresholds.len() as u32,
            survival: if a.started {
                b.lifecycle.special.farm.as_ref().unwrap().total
            } else {
                0
            },
            rooms: 0,
            kills: killed_monsters
                .into_iter()
                .map(|(_, a, b)| (a, b))
                .collect(),
            ends_infestation: false,
            prisoners: Default::default(),
            raid_quirks,
            map: String::new(),
            persistent: None,
            deaths_door: a.deaths_door.len() as u32,
            afflictions: a.afflictions.len() as u32,
            flashbacks: Default::default(),
            earned_rewards: a.awards,
            recovered: a.physical_bag.into_values().collect(),
        };
        self.state.active = Some(raid::Active {
            quest: a.quest,
            snapshot: String::new(),
        });
        self.accept_raid(receipt)?;
        for (id, amount) in xp_awards {
            let index = self.town().hero_index(id)?;
            self.state.city.heroes[index].xp = self.state.city.heroes[index]
                .xp
                .checked_add(amount)
                .ok_or("farm threshold XP overflow")?;
        }
        for id in missing {
            let until = self
                .state
                .clock
                .week
                .checked_add(lost.in_time_and_space_missing_duration)
                .ok_or("farm return date overflow")?;
            let index = self.town().hero_index(id)?;
            self.state.city.heroes[index].missing_until = until;
            self.state.city.heroes[index].state.build.trinkets.clear();
            self.state.city.heroes[index].state.build.physical_trinkets = Some(vec![]);
            self.state
                .content019
                .as_mut()
                .unwrap()
                .missing
                .insert(id, until);
        }
        self.refresh_content019()?;
        if regenerate {
            self.state.board.week = None;
            self.refresh_board()?;
        }
        Ok(())
    }
}

fn farm_stack(v: &Value) -> Result<Stack> {
    Ok(Stack {
        item: Item::new(
            v["type"].as_str().ok_or("farm reward kind absent")?,
            v["id"].as_str().ok_or("farm reward ID absent")?,
        ),
        amount: dd_data::campaign017::uint(v, "amount")?,
    })
}

// Camping executes source operations against the same native heroes at the
// settled interlude, without synthesizing combat turns or enemies.
impl Session {
    fn farm_noncombat_buffs019(&self, a: &Active, k: &mut PartyKernel) -> Result<()> {
        for buff in &a.camp_buffs {
            if self.definition.town.game.noncombat_buff(&buff.source.id)? {
                k.core.add_buff(
                    buff.hero,
                    buff.source.clone(),
                    crate::BuffScope::Build,
                    &mut vec![],
                );
            }
        }
        k.core.refresh_context(false);
        Ok(())
    }
    fn farm_entry_kernel019(&self, b: &Battle) -> Result<PartyKernel> {
        PartyKernel::new(b.scenario.heroes.clone(), b.scenario.rules.clone(), Rng(0))
    }
    fn reconcile_farm_camp019(&self, a: &mut Active, b: &mut Battle) -> Result<()> {
        let results = b.hero_results();
        for h in &results {
            if h.host_buff_instances.iter().any(|id| {
                !a.camp_buffs
                    .iter()
                    .any(|c| c.instance == *id && c.hero == h.id)
            }) {
                return Err("foreign farm native host buff receipt".into());
            }
        }
        let mut removed = BTreeSet::new();
        for buff in &a.camp_buffs {
            if !self.definition.town.game.noncombat_buff(&buff.source.id)?
                && !results
                    .iter()
                    .any(|h| h.id == buff.hero && h.host_buff_instances.contains(&buff.instance))
            {
                removed.insert(buff.instance);
            }
        }
        a.camp_buffs.retain(|c| !removed.contains(&c.instance));
        for bp in &mut b.scenario.heroes {
            bp.mechanics
                .hero
                .host_buffs
                .retain(|h| !removed.contains(&h.instance));
        }
        for u in &mut b.units {
            u.blueprint
                .mechanics
                .hero
                .host_buffs
                .retain(|h| !removed.contains(&h.instance));
        }
        b.refresh_context(false);
        Ok(())
    }
    fn farm_camp_points019(&self, b: &Battle) -> Result<u32> {
        let mut points = dd_data::campaign017::uint(
            &self.definition.town.data.base.globals,
            "camp_start_camping_points",
        )?;
        let departure = self
            .state
            .city
            .departure
            .as_ref()
            .ok_or("farm camp owner absent")?;
        for row in &departure.required.camping {
            let tags = row["hero_type_tags"]
                .as_array()
                .ok_or("farm camp district tags absent")?;
            if tags.is_empty()
                || b.units
                    .iter()
                    .filter(|u| u.blueprint.hero && u.alive())
                    .any(|u| {
                        u.blueprint
                            .mechanics
                            .hero
                            .tags
                            .iter()
                            .any(|tag| tags.iter().any(|v| v == tag))
                    })
            {
                points = points
                    .checked_add(dd_data::campaign017::uint(row, "camping_points")?)
                    .ok_or("farm respite overflow")?;
            }
        }
        Ok(points)
    }
    fn farm_camp_kernel019(
        &self,
        a: &Active,
        b: &Battle,
    ) -> Result<(PartyKernel, Vec<BTreeSet<String>>)> {
        let mut k = PartyKernel { core: b.clone() };
        k.core.round = 0;
        k.core.refresh_context(false);
        let before = k
            .core
            .units
            .iter()
            .map(|u| {
                u.buffs
                    .iter()
                    .map(|b| b.definition.id.clone())
                    .collect::<BTreeSet<_>>()
            })
            .collect::<Vec<_>>();
        k.exploration_buffs(&self.definition.town.game.buffs)?;
        self.farm_noncombat_buffs019(a, &mut k)?;
        k.camping_buffs(&self.definition.town.game.buffs)?;
        let added = k
            .core
            .units
            .iter()
            .zip(before)
            .map(|(u, old)| {
                u.buffs
                    .iter()
                    .map(|b| b.definition.id.clone())
                    .filter(|id| !old.contains(id))
                    .collect()
            })
            .collect();
        Ok((k, added))
    }
    fn farm_camp_view019(
        &self,
        a: &Active,
        b: &Battle,
    ) -> Result<Option<crate::expedition017::CampView>> {
        let Some(c) = &a.camp else {
            return Ok(None);
        };
        let mut skills = vec![];
        for (index, guid) in self.state.city.party.iter().enumerate() {
            if !b.units[index].alive() {
                continue;
            }
            let h = self.town().hero(*guid)?.clone();
            for id in &h.selected_camping {
                let s = dd_data::camp017::skill(
                    &self.definition.town.game,
                    self.farm_curios019()?,
                    &self.definition.town.data.camps,
                    &h.class,
                    id,
                    "farm",
                    a.quest.raw["difficulty"].as_u64().unwrap() as u32,
                )?;
                skills.push(crate::expedition017::VisibleSkill {
                    hero: *guid,
                    id: id.clone(),
                    cost: s.cost,
                    remaining: s.limit.saturating_sub(
                        c.uses
                            .get(guid)
                            .and_then(|m| m.get(id))
                            .copied()
                            .unwrap_or(0),
                    ),
                    effects: s
                        .effects
                        .into_iter()
                        .map(|e| crate::expedition017::VisibleEffect {
                            selection: e.selection,
                            requirements: e.requirements,
                            chance: e.chance,
                            operation: e.operation,
                        })
                        .collect(),
                });
            }
        }
        let mut meals = vec![];
        let (kernel, _) = self.farm_camp_kernel019(a, b)?;
        for row in self.definition.town.data.base.globals["meals_table"]
            .as_array()
            .ok_or("farm camp meals absent")?
        {
            let ratio = row["rations_per"]
                .as_f64()
                .ok_or("farm meal rations absent")?;
            let cost = kernel
                .core
                .units
                .iter()
                .take(self.state.city.party.len())
                .filter(|u| u.alive())
                .map(|u| (1. + u.modifier("food_consumption_percent", "")).max(0.) * ratio)
                .sum::<f64>()
                .ceil() as u32;
            meals.push((
                row["type"].as_str().ok_or("farm meal name absent")?.into(),
                cost,
                row["healing"].as_f64().ok_or("farm meal healing absent")?,
                row["stress"].as_i64().ok_or("farm meal stress absent")? as i32,
            ));
        }
        Ok(Some(crate::expedition017::CampView {
            phase: if c.meal { "skills" } else { "meal" }.into(),
            points: c.points,
            nightmare_warning: false,
            ambush_prevention: 1.,
            meals,
            skills,
        }))
    }
    fn farm_camp019(
        &self,
        a: &mut Active,
        b: &mut Battle,
        action: crate::expedition017::CampAction,
    ) -> Result<()> {
        use crate::expedition017::CampAction;
        let (mut k, temporary) = self.farm_camp_kernel019(a, b)?;
        match action {
            CampAction::Meal(id) => {
                let view = self.farm_camp_view019(a, b)?.ok_or("no farm camp")?;
                let (_, _, _, _) = view
                    .meals
                    .iter()
                    .find(|(name, _, _, _)| name == &id)
                    .ok_or("unknown farm meal")?;
                let rows = self.definition.town.data.base.globals["meals_table"]
                    .as_array()
                    .ok_or("farm meal sources absent")?;
                let row = rows
                    .iter()
                    .find(|r| r["type"] == id)
                    .ok_or("unknown farm meal")?;
                let starving = rows
                    .iter()
                    .find(|r| r["rations_per"] == 0)
                    .ok_or("farm starving meal absent")?;
                let mut cost = 0.;
                for index in 0..self.state.city.party.len() {
                    if !k.core.units[index].alive() {
                        continue;
                    }
                    let refused = k.core.reaction_block_at_camp(
                        index,
                        index,
                        dd_data::mechanics::ReactionKind::BlockCampingMeal,
                        &mut vec![],
                    );
                    let chosen = if refused { starving } else { row };
                    if !refused {
                        cost += (1. + k.modifier(index, "food_consumption_percent", "")).max(0.)
                            * row["rations_per"]
                                .as_f64()
                                .ok_or("farm meal ratio absent")?;
                    }
                    let heal = chosen["healing"]
                        .as_f64()
                        .ok_or("farm meal healing absent")?;
                    let bonus = if heal >= 0. {
                        k.modifier(index, "hp_heal_received_percent", "eat")
                    } else {
                        k.modifier(index, "starving_damage_percent", "")
                    };
                    k.hp(
                        index,
                        (f64::from(k.maximum(index)) * heal * (1. + bonus).max(0.)).round() as i32,
                    )?;
                    let stress = chosen["stress"].as_i64().ok_or("farm meal stress absent")? as i32;
                    let bonus = if stress < 0 {
                        k.modifier(index, "stress_heal_received_percent", "")
                            + k.modifier(index, "stress_heal_received_percent", "camping_eat")
                    } else {
                        k.modifier(index, "stress_dmg_received_percent", "camping_eat")
                    };
                    k.stress(
                        index,
                        (f64::from(stress) * (1. + bonus).max(0.)).round() as i32,
                    )?;
                }
                if cost > 0. {
                    a.consume(&Item::new("provision", ""), cost.ceil() as u32)?;
                }
                a.camp.as_mut().unwrap().meal = true;
            }
            CampAction::Rest => a.camp = None,
            CampAction::Skill {
                hero,
                skill,
                target,
            } => {
                let h = self.town().hero(hero)?.clone();
                let source = dd_data::camp017::skill(
                    &self.definition.town.game,
                    self.farm_curios019()?,
                    &self.definition.town.data.camps,
                    &h.class,
                    &skill,
                    "farm",
                    a.quest.raw["difficulty"].as_u64().unwrap() as u32,
                )?;
                let camp = a.camp.as_mut().ok_or("no farm camp")?;
                camp.points = camp
                    .points
                    .checked_sub(source.cost)
                    .ok_or("insufficient farm respite")?;
                *camp.uses.entry(hero).or_default().entry(skill).or_default() += 1;
                camp.spent = camp
                    .spent
                    .checked_add(source.cost)
                    .ok_or("farm respite ledger overflow")?;
                let actor = self
                    .state
                    .city
                    .party
                    .iter()
                    .position(|id| *id == hero)
                    .ok_or("farm camp performer absent")?;
                let performer_refused = k.core.reaction_block_at_camp(
                    actor,
                    actor,
                    dd_data::mechanics::ReactionKind::BlockCampingSkillPerformer,
                    &mut vec![],
                );
                let target_refused = target
                    .and_then(|id| self.state.city.party.iter().position(|g| *g == id))
                    .is_some_and(|index| {
                        k.core.reaction_block_at_camp(
                            actor,
                            index,
                            dd_data::mechanics::ReactionKind::BlockCampingSkillTarget,
                            &mut vec![],
                        )
                    });
                for group in &source.groups {
                    if performer_refused {
                        break;
                    }
                    let first = &source.effects[group[0]];
                    for (index, id) in self.state.city.party.iter().enumerate() {
                        if !k.core.units[index].alive() {
                            continue;
                        }
                        let recipient = match first.selection.as_str() {
                            "self" => *id == hero,
                            "individual" => Some(*id) == target && !target_refused,
                            "party" => true,
                            "party_other" => *id != hero && Some(*id) != target,
                            _ => return Err("unknown farm camp selection".into()),
                        };
                        if !recipient {
                            continue;
                        }
                        let mut roll = (a.rng.next() >> 11) as f64 / ((1u64 << 53) as f64);
                        for i in group {
                            let e = &source.effects[*i];
                            if roll < e.chance {
                                for r in &e.requirements {
                                    let u = &k.core.units[index];
                                    let applies = match r.as_str() {
                                        "religious" => u
                                            .blueprint
                                            .mechanics
                                            .hero
                                            .tags
                                            .iter()
                                            .any(|t| t == "religious"),
                                        "not_religious" => !u
                                            .blueprint
                                            .mechanics
                                            .hero
                                            .tags
                                            .iter()
                                            .any(|t| t == "religious"),
                                        "afflicted" => matches!(u.resolve, Resolve::Afflicted(_)),
                                        "has_deaths_door_recovery_buffs" => u
                                            .buffs
                                            .iter()
                                            .any(|b| b.scope == crate::BuffScope::Recovery),
                                        _ => return Err("unknown farm camp requirement".into()),
                                    };
                                    if !applies {
                                        roll = -1.;
                                        break;
                                    }
                                }
                                if roll >= 0. {
                                    self.farm_camp_operation019(
                                        a,
                                        &mut k,
                                        index,
                                        actor,
                                        &source.id,
                                        &e.operation,
                                    )?;
                                }
                                break;
                            }
                            roll -= e.chance;
                        }
                    }
                }
            }
            _ => return Err("illegal farm camp action".into()),
        }
        k.settle()?;
        let outside = a
            .camp_buffs
            .iter()
            .filter_map(
                |c| match self.definition.town.game.noncombat_buff(&c.source.id) {
                    Ok(true) => Some(Ok(c.source.id.clone())),
                    Ok(false) => None,
                    Err(e) => Some(Err(e)),
                },
            )
            .collect::<Result<BTreeSet<_>>>()?;
        for (u, added) in k.core.units.iter_mut().zip(temporary) {
            u.buffs.retain(|b| {
                b.scope != crate::BuffScope::Build
                    || (!added.contains(&b.definition.id) && !outside.contains(&b.definition.id))
            });
        }
        k.core.refresh_context(false);
        k.core.round = b.round;
        k.core.refresh_context(false);
        *b = k.core;
        Ok(())
    }
    fn farm_camp_operation019(
        &self,
        a: &mut Active,
        k: &mut PartyKernel,
        index: usize,
        actor: usize,
        skill: &str,
        op: &dd_data::camp017::Operation,
    ) -> Result<()> {
        use dd_data::camp017::Operation;
        match op {
            Operation::Heal(r) => k.hp(
                index,
                (f64::from(k.maximum(index))
                    * r
                    * (1. + k.modifier(actor, "hp_heal_percent", "")).max(0.))
                .round() as i32,
            )?,
            Operation::Damage(r) => {
                k.hp(index, -(f64::from(k.maximum(index)) * r).round() as i32)?
            }
            Operation::Stress(n) => {
                let bonus = if *n < 0 {
                    k.modifier(index, "stress_heal_received_percent", "")
                } else {
                    0.
                };
                k.stress(index, (f64::from(*n) * (1. + bonus).max(0.)).round() as i32)?;
            }
            Operation::BleedCure => k.core.units[index]
                .dots
                .retain(|d| !matches!(d.kind, crate::EffectKind::Bleed(_))),
            Operation::BlightCure => k.core.units[index]
                .dots
                .retain(|d| !matches!(d.kind, crate::EffectKind::Blight(_))),
            Operation::RecoveryCure => k.core.units[index]
                .buffs
                .retain(|b| b.scope != crate::BuffScope::Recovery),
            Operation::DiseaseCure => {
                for id in k.core.units[index].diseases.clone() {
                    if self
                        .definition
                        .town
                        .data
                        .base
                        .quirk_defs
                        .get(&id)
                        .is_some_and(|v| v["can_remove_with_camping_skill"] == true)
                    {
                        k.core.remove_disease(index, &id);
                    }
                }
            }
            Operation::PreventAmbush(_) | Operation::Torch(_) => {}
            Operation::Buff(buff) => {
                a.next_buff = a
                    .next_buff
                    .checked_add(1)
                    .ok_or("farm host buff identity overflow")?;
                let instance = a.next_buff;
                a.camp_buffs.push(CampBuff {
                    instance,
                    hero: index,
                    performer: actor,
                    skill: skill.into(),
                    source: *buff.clone(),
                    remaining: buff.duration,
                });
                if self.definition.town.game.noncombat_buff(&buff.id)? {
                    k.core
                        .add_buff(index, *buff.clone(), crate::BuffScope::Build, &mut vec![]);
                    return k.settle();
                }
                let mut projected = *buff.clone();
                projected.duration_type = Some("quest_end".into());
                projected.activation.remove_on_battle_complete = false;
                for bp in [
                    &mut k.core.scenario.heroes[index],
                    &mut k.core.units[index].blueprint,
                ] {
                    bp.mechanics
                        .hero
                        .host_buffs
                        .push(dd_data::hero010::HostBuff {
                            instance,
                            definition: projected.clone(),
                        });
                    self.definition.town.game.bind_host_buff_effects(bp)?;
                }
                k.core
                    .add_buff(index, projected, crate::BuffScope::Timed, &mut vec![]);
                k.core.units[index]
                    .buffs
                    .last_mut()
                    .ok_or("farm host buff not installed")?
                    .host_instance = Some(instance);
            }
            Operation::Loot { table, count } => self.farm_draw019(
                a,
                &[dd_data::curios013::Draw {
                    table: table.clone(),
                    count: *count,
                }],
            )?,
            Operation::RefreshUses => {
                a.camp
                    .as_mut()
                    .ok_or("farm refresh outside camp")?
                    .uses
                    .remove(&self.state.city.party[index]);
            }
        }
        k.settle()
    }
}
