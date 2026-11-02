//! Mid-expedition equipment owns the same physical instances across slots,
//! finite inventory, native battles, death drops, and the town return.
use super::*;
use dd_data::trinket010::TrinketState;
type Origin = (u64, usize);
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Original {
    pub origin: Origin,
    pub state: TrinketState,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Ledger {
    slots: BTreeMap<u64, Vec<Option<Origin>>>,
    originals: Vec<Original>,
}
#[derive(Debug, Clone)]
pub(crate) struct Placement {
    pub origin: Option<Origin>,
    pub holder: Option<(u64, usize)>,
    pub state: TrinketState,
}
#[derive(Debug, Clone)]
pub(crate) struct Receipt {
    pub originals: Vec<Original>,
    pub carried: Vec<Placement>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EquippedView {
    pub slot: usize,
    pub id: String,
    pub remaining: Option<u32>,
    pub quest_remaining: Option<u32>,
    pub generation: u32,
    pub growth: Vec<(String, String, f64)>,
}
impl Session {
    fn equipment_boundary023(&self) -> Result<()> {
        if !dd_data::mechanisms023::enabled(
            &self.definition.data.content,
            dd_data::mechanisms023::FIXES,
        ) || self.state.campaign.is_none()
            || self.state.phase != Phase::Expedition
            || self.state.battle.is_some()
            || !self.state.loot.is_empty()
            || self.camp_view()?.is_some()
            || self.map()?.pending().is_some()
        {
            return Err("equipment changes require an idle task 023 exploration boundary".into());
        }
        Ok(())
    }
    fn start_equipment023(&mut self) -> Result<()> {
        if self.state.campaign.as_ref().unwrap().equipment023.is_some() {
            return Ok(());
        }
        let mut ledger = Ledger::default();
        for h in &self.state.heroes {
            let original = self.spec()?.canon[&h.guid]
                .mechanics
                .build
                .input
                .physical_trinkets
                .as_ref()
                .ok_or("missing original equipment slots")?;
            let current = h
                .state
                .build
                .physical_trinkets
                .as_ref()
                .ok_or("missing current equipment slots")?;
            let mut slots = vec![];
            for (slot, _) in original.iter().enumerate() {
                let state = current
                    .get(slot)
                    .ok_or("original equipment slot disappeared")?
                    .clone();
                ledger.originals.push(Original {
                    origin: (h.guid, slot),
                    state: state.clone(),
                });
                slots.push((h.state.alive && !state.destroyed).then_some((h.guid, slot)));
            }
            ledger.slots.insert(h.guid, slots);
        }
        self.state.campaign.as_mut().unwrap().equipment023 = Some(ledger);
        Ok(())
    }
    fn equipment_states023(&self, hero: u64) -> Result<Vec<TrinketState>> {
        self.state.heroes[self.hero_index(hero)?]
            .state
            .build
            .physical_trinkets
            .clone()
            .ok_or("missing physical equipment build".into())
    }
    fn set_equipment023(&mut self, hero: u64, states: Vec<TrinketState>) -> Result<()> {
        let index = self.hero_index(hero)?;
        let mut build = self.state.heroes[index].state.build.clone();
        build.trinkets = states.iter().map(|s| s.definition.id.clone()).collect();
        build.trinket_states.clear();
        build.physical_trinkets = Some(states.clone());
        let mut ids = BTreeSet::new();
        if states
            .iter()
            .filter(|s| !s.destroyed)
            .any(|s| !ids.insert(&s.definition.id))
        {
            return Err("a hero cannot equip two identical trinkets".into());
        }
        self.definition
            .game
            .equip_build(self.state.heroes[index].base.clone(), build.clone())?;
        let h = &mut self.state.heroes[index];
        h.state.build = build;
        h.state.trinket_slots = Some(
            states
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.destroyed)
                .map(|(i, _)| i)
                .collect(),
        );
        h.state.trinkets = states.into_iter().filter(|s| !s.destroyed).collect();
        self.field_with_hp_policy(|k| k.settle(), true)
    }
    pub(in super::super) fn equip_trinket023(
        &mut self,
        hero: u64,
        bag_slot: usize,
        equipment_slot: usize,
    ) -> Result<()> {
        self.equipment_boundary023()?;
        if !self.state.party.contains(&hero) || equipment_slot >= 2 {
            return Err("invalid living hero or equipment slot".into());
        }
        let stack = self
            .state
            .bag
            .slots
            .get(bag_slot)
            .and_then(Option::as_ref)
            .filter(|s| s.item.kind == "trinket" && s.amount == 1)
            .ok_or("equipment needs one trinket in a finite bag slot")?
            .clone();
        let item = self
            .state
            .campaign
            .as_ref()
            .unwrap()
            .physical_bag
            .get(&bag_slot)
            .cloned()
            .unwrap_or(Carried {
                origin: None,
                state: TrinketState::fresh(self.physical_definition(&stack.item.id)?),
            });
        self.start_equipment023()?;
        let mut states = self.equipment_states023(hero)?;
        let outgoing = states.get(equipment_slot).filter(|s| !s.destroyed).cloned();
        let origins = &mut self
            .state
            .campaign
            .as_mut()
            .unwrap()
            .equipment023
            .as_mut()
            .unwrap()
            .slots;
        let slots = origins.get_mut(&hero).ok_or("equipment identity absent")?;
        let old_origin = slots.get(equipment_slot).copied().flatten();
        slots.resize(states.len().max(equipment_slot + 1), None);
        slots[equipment_slot] = item.origin;
        while states.len() <= equipment_slot {
            let mut empty = item.state.clone();
            empty.destroyed = true;
            states.push(empty);
        }
        states[equipment_slot] = item.state;
        if let Some(state) = outgoing {
            self.state.bag.slots[bag_slot] = Some(Stack {
                item: Item::new("trinket", &state.definition.id),
                amount: 1,
            });
            self.state.campaign.as_mut().unwrap().physical_bag.insert(
                bag_slot,
                Carried {
                    origin: old_origin,
                    state,
                },
            );
        } else {
            self.state.bag.slots[bag_slot] = None;
            self.state
                .campaign
                .as_mut()
                .unwrap()
                .physical_bag
                .remove(&bag_slot);
        }
        self.set_equipment023(hero, states)?;
        self.sync_equipment023()
    }
    pub(in super::super) fn unequip_trinket023(
        &mut self,
        hero: u64,
        equipment_slot: usize,
    ) -> Result<()> {
        self.equipment_boundary023()?;
        if !self.state.party.contains(&hero) || equipment_slot >= 2 {
            return Err("invalid living hero or equipment slot".into());
        }
        let bag_slot = self
            .state
            .bag
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or("unequip needs an empty inventory slot")?;
        let mut states = self.equipment_states023(hero)?;
        let state = states
            .get(equipment_slot)
            .filter(|s| !s.destroyed)
            .ok_or("empty equipment slot")?
            .clone();
        self.start_equipment023()?;
        let slots = self
            .state
            .campaign
            .as_mut()
            .unwrap()
            .equipment023
            .as_mut()
            .unwrap()
            .slots
            .get_mut(&hero)
            .ok_or("equipment identity absent")?;
        let origin = slots
            .get_mut(equipment_slot)
            .ok_or("equipment origin slot absent")?
            .take();
        self.state.bag.slots[bag_slot] = Some(Stack {
            item: Item::new("trinket", &state.definition.id),
            amount: 1,
        });
        self.state
            .campaign
            .as_mut()
            .unwrap()
            .physical_bag
            .insert(bag_slot, Carried { origin, state });
        states[equipment_slot].destroyed = true;
        self.set_equipment023(hero, states)?;
        self.sync_equipment023()
    }
    pub(in super::super) fn equipment_actions023(&self) -> Vec<Action> {
        if self.equipment_boundary023().is_err() {
            return vec![];
        }
        let mut actions = vec![];
        for hero in &self.state.party {
            let Ok(index) = self.hero_index(*hero) else {
                continue;
            };
            let h = &self.state.heroes[index];
            let Ok(states) = self.equipment_states023(*hero) else {
                continue;
            };
            for equipment_slot in 0..2 {
                if self.state.bag.slots.iter().any(Option::is_none)
                    && states.get(equipment_slot).is_some_and(|s| !s.destroyed)
                {
                    actions.push(Action::UnequipTrinket {
                        hero: *hero,
                        equipment_slot,
                    });
                }
                for bag_slot in self
                    .state
                    .bag
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.as_ref().is_some_and(|s| s.item.kind == "trinket"))
                    .map(|(i, _)| i)
                {
                    let item = &self.state.bag.slots[bag_slot].as_ref().unwrap().item;
                    let raw = self.definition.data.bag_rules.trinkets.get(&item.id);
                    let allowed = raw.is_some_and(|v| {
                        v["hero_class_requirements"]
                            .as_array()
                            .is_none_or(|a| a.is_empty() || a.iter().any(|id| id == &h.base.id))
                    });
                    if allowed
                        && !states.iter().enumerate().any(|(slot, s)| {
                            slot != equipment_slot && !s.destroyed && s.definition.id == item.id
                        })
                    {
                        actions.push(Action::EquipTrinket {
                            hero: *hero,
                            bag_slot,
                            equipment_slot,
                        });
                    }
                }
            }
        }
        actions
    }
    fn equipment_placements023(&self, include_dead: bool) -> Result<Vec<Placement>> {
        let p = self.state.campaign.as_ref().ok_or("no expedition")?;
        let ledger = p.equipment023.as_ref().ok_or("no equipment ledger")?;
        let mut rows = vec![];
        for h in self
            .state
            .heroes
            .iter()
            .filter(|h| include_dead || h.state.alive)
        {
            let states = self.equipment_states023(h.guid)?;
            for (slot, state) in states
                .into_iter()
                .enumerate()
                .filter(|(_, s)| !s.destroyed || include_dead)
            {
                let origin = ledger
                    .slots
                    .get(&h.guid)
                    .and_then(|v| v.get(slot))
                    .copied()
                    .flatten();
                rows.push(Placement {
                    origin,
                    holder: Some((h.guid, slot)),
                    state,
                });
            }
        }
        for item in p.physical_bag.values() {
            rows.push(Placement {
                origin: item.origin,
                holder: None,
                state: item.state.clone(),
            });
        }
        Ok(rows)
    }
    pub(in super::super) fn sync_equipment023(&mut self) -> Result<()> {
        if !self
            .state
            .campaign
            .as_ref()
            .is_some_and(|p| p.equipment023.is_some())
        {
            return Ok(());
        }
        let mut rows = self.equipment_placements023(true)?;
        rows.extend(
            self.state
                .campaign
                .as_ref()
                .unwrap()
                .physical_tray
                .values()
                .map(|item| Placement {
                    origin: item.origin,
                    holder: None,
                    state: item.state.clone(),
                }),
        );
        let ledger = self
            .state
            .campaign
            .as_mut()
            .unwrap()
            .equipment023
            .as_mut()
            .unwrap();
        for row in rows {
            if let Some(origin) = row.origin {
                ledger
                    .originals
                    .iter_mut()
                    .find(|o| o.origin == origin)
                    .ok_or("unknown equipment origin")?
                    .state = row.state;
            }
        }
        Ok(())
    }
    pub(in super::super) fn dropped_origin023(
        &mut self,
        guid: u64,
        slot: usize,
        state: &TrinketState,
    ) -> Result<Option<Origin>> {
        let p = self.state.campaign.as_mut().ok_or("missing campaign")?;
        let Some(ledger) = &mut p.equipment023 else {
            return Ok(Some((guid, slot)));
        };
        let origin = ledger
            .slots
            .get_mut(&guid)
            .and_then(|v| v.get_mut(slot))
            .ok_or("missing dropped equipment position")?
            .take();
        if let Some(origin) = origin {
            ledger
                .originals
                .iter_mut()
                .find(|o| o.origin == origin)
                .ok_or("unknown dropped original")?
                .state = state.clone();
        }
        Ok(origin)
    }
    pub(in super::super) fn validate_equipment023(&self) -> Result<()> {
        let p = self
            .state
            .campaign
            .as_ref()
            .ok_or("missing expedition equipment")?;
        let l = p.equipment023.as_ref().ok_or("missing equipment ledger")?;
        if !dd_data::mechanisms023::enabled(
            &self.definition.data.content,
            dd_data::mechanisms023::FIXES,
        ) {
            return Err("new equipment ledger in legacy profile".into());
        }
        let expected = self
            .spec()?
            .canon
            .iter()
            .flat_map(|(guid, b)| {
                b.mechanics
                    .build
                    .input
                    .physical_trinkets
                    .iter()
                    .flatten()
                    .enumerate()
                    .map(move |(slot, _)| (*guid, slot))
            })
            .collect::<BTreeSet<_>>();
        if l.originals
            .iter()
            .map(|o| o.origin)
            .collect::<BTreeSet<_>>()
            != expected
            || l.originals.len() != expected.len()
            || l.slots.keys().copied().collect::<BTreeSet<_>>()
                != self.spec()?.canon.keys().copied().collect()
        {
            return Err("equipment origin registry differs from deployment".into());
        }
        let mut seen = BTreeSet::new();
        let mut rows = self.equipment_placements023(true)?;
        rows.extend(p.physical_tray.values().map(|i| Placement {
            origin: i.origin,
            holder: None,
            state: i.state.clone(),
        }));
        for row in rows {
            self.validate_physical_definition(&row.state)?;
            if let Some(origin) = row.origin {
                if !seen.insert(origin) || !expected.contains(&origin) {
                    return Err("duplicate/foreign equipment origin".into());
                }
                let original = l.originals.iter().find(|o| o.origin == origin).unwrap();
                if original.state != row.state {
                    return Err("equipment registry and slot state differ".into());
                }
            }
        }
        for o in &l.originals {
            self.validate_physical_definition(&o.state)?;
            let b = &self.spec()?.canon[&o.origin.0];
            let initial = &b.mechanics.build.input.physical_trinkets.as_ref().unwrap()[o.origin.1];
            if !self
                .definition
                .game
                .trinket_catalog(std::slice::from_ref(&initial.definition.id))?
                .contains_key(&o.state.definition.id)
            {
                return Err("equipment changed to an unrelated original".into());
            }
        }
        for (bag, items) in [(true, &p.physical_bag), (false, &p.physical_tray)] {
            for (slot, item) in items {
                let stack = if bag {
                    self.state.bag.slots.get(*slot).and_then(Option::as_ref)
                } else {
                    self.state.loot.get(*slot)
                };
                if item.state.destroyed
                    || stack.is_none_or(|s| {
                        s.item != Item::new("trinket", &item.state.definition.id) || s.amount != 1
                    })
                {
                    return Err("equipment does not match finite inventory slot".into());
                }
            }
        }
        Ok(())
    }
    pub(in super::super) fn equipment_receipt023(&self) -> Result<Option<Receipt>> {
        let Some(ledger) = self
            .state
            .campaign
            .as_ref()
            .and_then(|p| p.equipment023.as_ref())
        else {
            return Ok(None);
        };
        self.validate_equipment023()?;
        Ok(Some(Receipt {
            originals: ledger.originals.clone(),
            carried: if self.state.party.is_empty() {
                vec![]
            } else {
                self.equipment_placements023(false)?
            },
        }))
    }
    pub(in super::super) fn equipped_view023(&self, hero: u64) -> Option<Vec<EquippedView>> {
        self.state.campaign.as_ref()?.equipment023.as_ref()?;
        Some(
            self.equipment_states023(hero)
                .ok()?
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.destroyed)
                .map(|(slot, s)| EquippedView {
                    slot,
                    id: s.definition.id.clone(),
                    remaining: s.remaining,
                    quest_remaining: s.quest_remaining,
                    generation: s.generation,
                    growth: s
                        .growth
                        .iter()
                        .map(|b| (b.stat_type.clone(), b.stat_sub_type.clone(), b.amount))
                        .collect(),
                })
                .collect(),
        )
    }
}
