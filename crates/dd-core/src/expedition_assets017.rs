//! Finite physical equipment drops, separate from fresh ordinary loot IDs.
use super::*;
use dd_data::trinket010::TrinketState;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Carried {
    pub origin: Option<(u64, usize)>,
    pub state: TrinketState,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ItemView {
    pub in_loot: bool,
    pub slot: usize,
    pub remaining: Option<u32>,
    pub quest_remaining: Option<u32>,
    pub growth: Vec<(String, String, f64)>,
}
impl Session {
    fn physical_definition(&self, id: &str) -> Result<dd_data::trinket010::Trinket> {
        let cached = self
            .definition
            .cache
            .lock()
            .map_err(|_| "item source cache poisoned")?
            .physical_items
            .get(id)
            .cloned();
        if let Some(value) = cached {
            return Ok(value);
        }
        let catalog = self.definition.game.trinket_catalog(&[id.into()])?;
        let value = catalog
            .get(id)
            .ok_or("missing physical item source")?
            .clone();
        self.definition
            .cache
            .lock()
            .map_err(|_| "item source cache poisoned")?
            .physical_items
            .extend(catalog);
        Ok(value)
    }
    fn validate_physical_definition(&self, state: &TrinketState) -> Result<()> {
        self.definition.game.validate_item_growth(state)?;
        if self.physical_definition(&state.definition.id)? != state.definition {
            return Err("carried item definition differs from original source".into());
        }
        Ok(())
    }
    pub(in super::super) fn campaign_item_context(
        &self,
        context: &mut dd_data::build010::BattleContext,
    ) -> Result<()> {
        let Some(p) = &self.state.campaign else {
            return Ok(());
        };
        if !context.trinket_inventory.is_empty() {
            return Err("unbound native inventory at campaign encounter".into());
        }
        for (slot, stack) in self
            .state
            .bag
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, s)| {
                s.as_ref()
                    .filter(|s| s.item.kind == "trinket")
                    .map(|s| (slot, s))
            })
        {
            let state = if let Some(item) = p.physical_bag.get(&slot) {
                item.state.clone()
            } else {
                TrinketState::fresh(self.physical_definition(&stack.item.id)?)
            };
            self.validate_physical_definition(&state)?;
            context.trinket_inventory.push(state);
        }
        for (id, value) in &self
            .definition
            .cache
            .lock()
            .map_err(|_| "item source cache poisoned")?
            .physical_items
        {
            if context
                .trinket_catalog
                .get(id)
                .is_some_and(|old| old != value)
            {
                return Err("conflicting campaign inventory catalog".into());
            }
            context.trinket_catalog.insert(id.clone(), value.clone());
        }
        Ok(())
    }
    fn update_dead_equipment(&mut self, origin: (u64, usize), state: &TrinketState) -> Result<()> {
        let index = self.hero_index(origin.0)?;
        let hero = &mut self.state.heroes[index];
        if hero.state.alive {
            return Err("carried origin still equipped on living hero".into());
        }
        let states = hero
            .state
            .build
            .physical_trinkets
            .as_mut()
            .ok_or("missing physical death build")?;
        *states
            .get_mut(origin.1)
            .ok_or("missing carried physical origin")? = state.clone();
        hero.state.build.trinkets = states.iter().map(|s| s.definition.id.clone()).collect();
        hero.state.trinkets = states.iter().filter(|s| !s.destroyed).cloned().collect();
        hero.state.trinket_slots = Some(
            states
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.destroyed)
                .map(|(slot, _)| slot)
                .collect(),
        );
        Ok(())
    }
    fn reconcile_native_items(
        &mut self,
        result: &crate::BattleResult,
    ) -> Result<Vec<TrinketState>> {
        let slots = self
            .state
            .bag
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.as_ref().is_some_and(|s| s.item.kind == "trinket"))
            .map(|(slot, _)| slot)
            .collect::<Vec<_>>();
        let Some(receipt) = &result.host_inventory_states else {
            if !slots.is_empty() {
                return Err("missing settled physical inventory receipt".into());
            }
            return Ok(result.trinket_inventory.clone());
        };
        if receipt.input_slots != slots.len() {
            return Err("foreign native inventory input slot receipt".into());
        }
        let states = &receipt.states;
        if states.len() < slots.len() {
            return Err("native inventory lost physical slot receipts".into());
        }
        let mut gained = vec![];
        for (index, state) in states.iter().enumerate() {
            self.validate_physical_definition(state)?;
            if let Some(slot) = slots.get(index) {
                let origin = self
                    .state
                    .campaign
                    .as_ref()
                    .unwrap()
                    .physical_bag
                    .get(slot)
                    .and_then(|i| i.origin);
                if let Some(origin) = origin {
                    self.update_dead_equipment(origin, state)?;
                }
                if state.destroyed {
                    self.state.bag.slots[*slot] = None;
                    self.state
                        .campaign
                        .as_mut()
                        .unwrap()
                        .physical_bag
                        .remove(slot);
                } else {
                    self.state.bag.slots[*slot] = Some(Stack {
                        item: Item::new("trinket", &state.definition.id),
                        amount: 1,
                    });
                    self.state.campaign.as_mut().unwrap().physical_bag.insert(
                        *slot,
                        Carried {
                            origin,
                            state: state.clone(),
                        },
                    );
                }
            } else if !state.destroyed {
                gained.push(state.clone());
            }
        }
        Ok(gained)
    }
    pub(in super::super) fn validate_physical_items(&self) -> Result<()> {
        let Some(p) = &self.state.campaign else {
            return Ok(());
        };
        let mut origins = BTreeSet::new();
        for (bag, rows) in [(true, &p.physical_bag), (false, &p.physical_tray)] {
            for (slot, item) in rows {
                self.validate_physical_definition(&item.state)?;
                if item.state.destroyed {
                    return Err("destroyed carried physical item".into());
                }
                let stack = if bag {
                    self.state.bag.slots.get(*slot).and_then(Option::as_ref)
                } else {
                    self.state.loot.get(*slot)
                };
                if stack.is_none_or(|s| {
                    s.amount != 1
                        || s.item.kind != "trinket"
                        || s.item.id != item.state.definition.id
                }) {
                    return Err("physical item does not match its finite slot".into());
                }
                if let Some((guid, native_slot)) = item.origin {
                    if !origins.insert((guid, native_slot)) {
                        return Err("duplicated recovered equipment origin".into());
                    }
                    let hero = &self.state.heroes[self.hero_index(guid)?];
                    if hero.state.alive {
                        return Err("living equipped item duplicated in loot".into());
                    }
                    let states = hero
                        .state
                        .build
                        .physical_trinkets
                        .as_ref()
                        .ok_or("missing original physical slots")?;
                    if states.get(native_slot) != Some(&item.state) {
                        return Err("recovery differs from dead physical slot".into());
                    }
                }
            }
        }
        Ok(())
    }
    pub(in super::super) fn campaign_item_view(&self) -> Vec<ItemView> {
        let Some(p) = &self.state.campaign else {
            return vec![];
        };
        [(false, &p.physical_bag), (true, &p.physical_tray)]
            .into_iter()
            .flat_map(|(in_loot, rows)| {
                rows.iter().map(move |(slot, item)| ItemView {
                    in_loot,
                    slot: *slot,
                    remaining: item.state.remaining,
                    quest_remaining: item.state.quest_remaining,
                    growth: item
                        .state
                        .growth
                        .iter()
                        .map(|b| (b.stat_type.clone(), b.stat_sub_type.clone(), b.amount))
                        .collect(),
                })
            })
            .collect()
    }
    pub(in super::super) fn campaign_item_discard(&mut self, slot: usize) {
        if let Some(p) = &mut self.state.campaign {
            p.physical_bag.remove(&slot);
        }
    }
    pub(in super::super) fn campaign_item_clear_tray(&mut self) {
        if let Some(p) = &mut self.state.campaign {
            p.physical_tray.clear();
        }
    }
    pub(in super::super) fn campaign_item_pickup(
        &mut self,
        index: usize,
        amount: u32,
        before: &Bag,
        removed: bool,
    ) -> Result<()> {
        let Some(p) = &mut self.state.campaign else {
            return Ok(());
        };
        if let Some(item) = p.physical_tray.remove(&index) {
            if amount != 1 || !removed {
                return Err("partial physical equipment pickup".into());
            }
            let slots = self
                .state
                .bag
                .slots
                .iter()
                .enumerate()
                .filter(|(slot, s)| {
                    before.slots[*slot].is_none()
                        && s.as_ref().is_some_and(|s| {
                            s.item.kind == "trinket"
                                && s.item.id == item.state.definition.id
                                && s.amount == 1
                        })
                })
                .map(|(slot, _)| slot)
                .collect::<Vec<_>>();
            if slots.len() != 1 {
                return Err("physical pickup has no unique finite destination".into());
            }
            if p.physical_bag.insert(slots[0], item).is_some() {
                return Err("physical pickup overwrote occupied identity".into());
            }
        }
        if removed {
            p.physical_tray = std::mem::take(&mut p.physical_tray)
                .into_iter()
                .map(|(slot, item)| (if slot > index { slot - 1 } else { slot }, item))
                .collect();
        }
        Ok(())
    }
    pub(in super::super) fn campaign_item_drops(
        &mut self,
        result: &crate::BattleResult,
    ) -> Result<()> {
        let gained = self.reconcile_native_items(result)?;
        let mut items = vec![];
        if result.outcome == Outcome::HeroesWin {
            for h in result.heroes.iter().filter(|h| !h.alive) {
                let guid = *self
                    .state
                    .battle_party
                    .get(h.id)
                    .ok_or("unknown equipment death identity")?;
                let slots = h
                    .trinket_slots
                    .as_ref()
                    .ok_or("missing dropped equipment slots")?;
                for (state, slot) in h.trinkets.iter().zip(slots) {
                    items.push(Carried {
                        origin: Some((guid, *slot)),
                        state: state.clone(),
                    });
                }
            }
        }
        if result.heroes.iter().any(|h| h.alive) {
            for state in &gained {
                items.push(Carried {
                    origin: None,
                    state: state.clone(),
                });
            }
        }
        for item in items {
            self.validate_physical_definition(&item.state)?;
            let stack = Stack {
                item: Item::new("trinket", &item.state.definition.id),
                amount: 1,
            };
            self.inventory_rules()?.definition(&stack.item)?;
            let slot = self.state.loot.len();
            self.state.loot.push(stack);
            self.state
                .campaign
                .as_mut()
                .unwrap()
                .physical_tray
                .insert(slot, item);
        }
        Ok(())
    }
}
