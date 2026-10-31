use super::*;
use dd_data::trinket010::{Trinket, TrinketOp, TrinketState};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Source {
    Equipped(UnitId, usize),
    Bag(usize),
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Runtime {
    pub catalog: BTreeMap<String, Trinket>,
    pub bag: Vec<TrinketState>,
    pub sequence: u64,
    pub ended: bool,
    #[serde(skip)]
    pub source: Option<Source>,
    #[serde(skip)]
    pub root: Option<u64>,
    #[serde(skip)]
    pub once: BTreeSet<(u64, Source, u32, String, String)>,
    #[serde(skip)]
    pub charged: BTreeSet<(u64, Source, u32, String)>,
    #[serde(skip)]
    pub generations: BTreeMap<(u64, Source), u32>,
}
pub(super) fn equipment(u: &Unit) -> Vec<Buff> {
    let live = u
        .trinkets
        .iter()
        .filter(|t| !t.destroyed)
        .collect::<Vec<_>>();
    let mut buffs = live
        .iter()
        .flat_map(|t| {
            t.definition
                .buffs
                .iter()
                .chain(&t.growth)
                .map(|buff| {
                    let mut buff = buff.clone();
                    if u.blueprint
                        .mechanics
                        .build
                        .input
                        .physical_trinkets
                        .is_some()
                        && buff.rule_type == "quest_uses_remaining_above"
                        && buff.rule_data.string == t.definition.id
                    {
                        let active = t
                            .quest_remaining
                            .is_some_and(|n| f64::from(n) > buff.rule_data.float.unwrap_or(0.))
                            ^ buff.is_false_rule;
                        buff.rule_type = "always".into();
                        buff.is_false_rule = !active;
                    }
                    buff
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if live.len() == 2
        && let Some(set) = live[0]
            .definition
            .set
            .as_ref()
            .filter(|s| live[1].definition.set.as_ref() == Some(*s))
        && let Some(extra) = u.blueprint.mechanics.build.sets.get(set)
    {
        buffs.extend(extra.clone());
    }
    buffs
}
impl Battle {
    pub(super) fn initialize_trinkets(&mut self) -> Result<()> {
        for u in &self.units {
            for (id, def) in &u.blueprint.mechanics.build.trinket_catalog {
                if self
                    .lifecycle
                    .items
                    .catalog
                    .get(id)
                    .is_some_and(|old| old != def)
                {
                    return Err("conflicting battle trinket definitions".into());
                }
                self.lifecycle.items.catalog.insert(id.clone(), def.clone());
            }
        }
        if let Some(ctx) = &self.scenario.rules.context {
            for (id, def) in &ctx.trinket_catalog {
                if self
                    .lifecycle
                    .items
                    .catalog
                    .get(id)
                    .is_some_and(|old| old != def)
                {
                    return Err("conflicting inventory trinket definitions".into());
                }
                self.lifecycle.items.catalog.insert(id.clone(), def.clone());
            }
            self.lifecycle.items.bag = ctx.trinket_inventory.clone();
        }
        self.refresh_context(false);
        self.validate_trinkets()
    }
    pub(super) fn validate_trinkets(&self) -> Result<()> {
        if self.lifecycle.items.source.is_some()
            || self.lifecycle.items.root.is_some()
            || !self.lifecycle.items.once.is_empty()
            || !self.lifecycle.items.charged.is_empty()
            || !self.lifecycle.items.generations.is_empty()
        {
            return Err("unsettled trinket trigger boundary".into());
        }
        for (id, def) in &self.lifecycle.items.catalog {
            if id != &def.id {
                return Err("invalid trinket catalog ID".into());
            }
            for b in &def.buffs {
                b.validate()?;
            }
            for e in def.effects.values().flatten() {
                validate_effect(e)?;
                validate_effect_context(e, &self.scenario.rules)?;
            }
        }
        for state in self
            .units
            .iter()
            .flat_map(|u| &u.trinkets)
            .chain(&self.lifecycle.items.bag)
        {
            state.validate()?;
            if self.lifecycle.items.catalog.get(&state.definition.id) != Some(&state.definition) {
                return Err("unresolved trinket state".into());
            }
        }
        Ok(())
    }
    fn item(&self, key: Source) -> Option<&TrinketState> {
        match key {
            Source::Equipped(id, slot) => self.units.get(id)?.trinkets.get(slot),
            Source::Bag(slot) => self.lifecycle.items.bag.get(slot),
        }
    }
    fn item_mut(&mut self, key: Source) -> Option<&mut TrinketState> {
        match key {
            Source::Equipped(id, slot) => self.units.get_mut(id)?.trinkets.get_mut(slot),
            Source::Bag(slot) => self.lifecycle.items.bag.get_mut(slot),
        }
    }
    pub(super) fn begin_item_root(&mut self) -> Option<u64> {
        let previous = self.lifecycle.items.root;
        self.lifecycle.items.sequence = self.lifecycle.items.sequence.wrapping_add(1);
        self.lifecycle.items.root = Some(self.lifecycle.items.sequence);
        let root = self.lifecycle.items.sequence;
        for (id, u) in self.units.iter().enumerate() {
            for (slot, t) in u.trinkets.iter().enumerate() {
                self.lifecycle
                    .items
                    .generations
                    .insert((root, Source::Equipped(id, slot)), t.generation);
            }
        }
        for (slot, t) in self.lifecycle.items.bag.iter().enumerate() {
            self.lifecycle
                .items
                .generations
                .insert((root, Source::Bag(slot)), t.generation);
        }
        previous
    }
    pub(super) fn end_item_root(&mut self, previous: Option<u64>) {
        if let Some(root) = self.lifecycle.items.root {
            self.lifecycle.items.once.retain(|v| v.0 != root);
            self.lifecycle.items.charged.retain(|v| v.0 != root);
            self.lifecycle
                .items
                .generations
                .retain(|(r, _), _| *r != root);
        }
        self.lifecycle.items.root = previous;
    }
    fn change_item(
        &mut self,
        key: Source,
        to: Option<String>,
        events: &mut Vec<Event>,
    ) -> Result<()> {
        let old = self.item(key).ok_or("missing source trinket")?.clone();
        let next = to
            .as_ref()
            .map(|id| {
                self.lifecycle
                    .items
                    .catalog
                    .get(id)
                    .cloned()
                    .ok_or_else(|| format!("missing transformed trinket {id}"))
            })
            .transpose()?;
        let slot = self.item_mut(key).ok_or("missing source trinket")?;
        if let Some(def) = next {
            *slot = TrinketState::fresh(def);
            slot.generation = old.generation.saturating_add(1);
        } else {
            slot.destroyed = true;
            slot.generation = old.generation.saturating_add(1);
        }
        events.push(Event::TrinketChanged {
            unit: match key {
                Source::Equipped(id, _) => Some(id),
                Source::Bag(_) => None,
            },
            from: old.definition.id,
            to,
        });
        self.refresh_context(false);
        Ok(())
    }
    pub(super) fn trinket_operation(
        &mut self,
        op: &TrinketOp,
        events: &mut Vec<Event>,
    ) -> Result<bool> {
        let key = self
            .lifecycle
            .items
            .source
            .ok_or("item operation requires a trinket trigger context")?;
        match op {
            TrinketOp::Destroy => self.change_item(key, None, events)?,
            TrinketOp::Gain(id) => {
                let definition = self
                    .lifecycle
                    .items
                    .catalog
                    .get(id)
                    .cloned()
                    .ok_or("unresolved gained trinket")?;
                if self.lifecycle.items.bag.len() >= 10000 {
                    return Err("battle trinket inventory bound exceeded".into());
                }
                self.lifecycle
                    .items
                    .bag
                    .push(TrinketState::fresh(definition));
                events.push(Event::TrinketGained(id.clone()));
            }
            TrinketOp::Increase { min, max } => {
                let amount = self.rng.range([*min as i32, *max as i32]) as u32;
                let state = self.item_mut(key).ok_or("missing item operation source")?;
                let maximum = state
                    .maximum
                    .ok_or("cannot increase an unlimited trigger counter")?;
                let new = maximum
                    .checked_add(amount)
                    .filter(|n| *n <= 1000000)
                    .ok_or("trigger counter bound exceeded")?;
                state.maximum = Some(new);
                state.remaining = state.remaining.map(|n| n + amount);
            }
        }
        Ok(true)
    }
    fn charge_item(
        &mut self,
        key: Source,
        event: &str,
        generation: u32,
        root: u64,
        events: &mut Vec<Event>,
    ) -> Result<()> {
        if !self
            .lifecycle
            .items
            .charged
            .insert((root, key, generation, event.into()))
        {
            return Ok(());
        }
        let (block_trigger, block_quest) = match key {
            Source::Equipped(id, slot) => self.units[id]
                .trinkets
                .iter()
                .enumerate()
                .filter(|(s, t)| *s != slot && !t.destroyed)
                .fold((false, false), |(a, b), (_, t)| {
                    (
                        a || t.definition.blocks_trigger,
                        b || t.definition.blocks_quest,
                    )
                }),
            Source::Bag(_) => (false, false),
        };
        let state = self.item_mut(key).ok_or("missing charged item")?;
        if state.generation != generation {
            return Ok(());
        }
        if !block_trigger {
            state.remaining = state.remaining.map(|n| n.saturating_sub(1));
        }
        if !block_quest {
            state.quest_remaining = state.quest_remaining.map(|n| n.saturating_sub(1));
        }
        let transform = state.definition.transform_empty.clone();
        let destroy = state.definition.destroy_on_empty;
        let empty = state.remaining == Some(0);
        if empty && (transform.is_some() || destroy) {
            self.change_item(key, transform, events)?;
        }
        self.refresh_context(false);
        Ok(())
    }
    pub(super) fn fire_trinkets(
        &mut self,
        owner: UnitId,
        target: UnitId,
        event: &str,
        hit: bool,
        events: &mut Vec<Event>,
    ) {
        if self.units[owner]
            .blueprint
            .mechanics
            .boss
            .restrictions
            .no_trinket_attacker
            || (owner != target
                && self.units[target]
                    .blueprint
                    .mechanics
                    .boss
                    .restrictions
                    .no_trinket_target)
        {
            return;
        }
        let own_root = self.lifecycle.items.root.is_none();
        let previous = if own_root {
            self.begin_item_root()
        } else {
            None
        };
        let root = self.lifecycle.items.root.unwrap();
        let mut sources = (0..self.units[owner].trinkets.len())
            .map(|slot| Source::Equipped(owner, slot))
            .collect::<Vec<_>>();
        let party_owner = self
            .units
            .iter()
            .enumerate()
            .find(|(_, u)| u.alive() && u.blueprint.hero)
            .map(|(id, _)| id)
            .unwrap_or(0);
        if owner == party_owner {
            sources.extend((0..self.lifecycle.items.bag.len()).map(Source::Bag));
        }
        for source in sources {
            let Some(state) = self.item(source).cloned().filter(|s| s.enabled()) else {
                continue;
            };
            if self.lifecycle.items.generations.get(&(root, source)) != Some(&state.generation) {
                continue;
            }
            let Some(effects) = state.definition.effects.get(event) else {
                continue;
            };
            let mut any = false;
            for effect in effects {
                if (hit && !effect.on_hit) || (!hit && !effect.on_miss) {
                    continue;
                }
                if matches!(source, Source::Bag(_))
                    && !matches!(effect.kind, EffectKind::InventoryEffect(_))
                {
                    continue;
                }
                if effect.apply_once
                    && !self.lifecycle.items.once.insert((
                        root,
                        source,
                        state.generation,
                        event.into(),
                        effect.name.clone(),
                    ))
                {
                    continue;
                }
                if self
                    .item(source)
                    .is_none_or(|s| s.generation != state.generation || s.destroyed)
                {
                    break;
                }
                let saved = self.lifecycle.items.source.replace(source);
                let before = events.len();
                if matches!(effect.kind, EffectKind::InstantBuffs(_)) {
                    any = true;
                } else {
                    self.apply_external_effect(owner, target, effect, events);
                    any |= events[before..]
                        .iter()
                        .any(|e| matches!(e, Event::Effect { applied: true, .. }));
                }
                self.lifecycle.items.source = saved;
            }
            if any
                && let Err(error) = self.charge_item(source, event, state.generation, root, events)
            {
                self.lifecycle.failure = Some(error);
            }
        }
        if own_root {
            self.end_item_root(previous);
        }
    }
    pub(super) fn trinket_modifiers(&self, owner: UnitId, event: &str) -> Vec<Buff> {
        self.units[owner]
            .trinkets
            .iter()
            .filter(|s| s.enabled())
            .filter_map(|s| s.definition.effects.get(event))
            .flatten()
            .filter_map(|e| {
                if let EffectKind::InstantBuffs(b) = &e.kind {
                    Some(b.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
    pub(super) fn trinket_death(&mut self, id: UnitId, events: &mut Vec<Event>) {
        self.fire_trinkets(id, id, "was_killed", true, events);
        let changes = self.units[id]
            .trinkets
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.destroyed)
            .filter_map(|(slot, s)| {
                s.definition
                    .transform_death
                    .clone()
                    .map(|target| (slot, target))
            })
            .collect::<Vec<_>>();
        for (slot, target) in changes {
            if let Err(error) = self.change_item(Source::Equipped(id, slot), Some(target), events) {
                self.lifecycle.failure = Some(error);
            }
        }
        for other in 0..self.scenario.heroes.len() {
            self.fire_trinkets(other, id, "was_killed_all_heroes", true, events);
        }
    }
    pub(super) fn trinket_round_end(&mut self, events: &mut Vec<Event>) {
        for id in 0..self.scenario.heroes.len() {
            if self.units[id].alive() {
                self.fire_trinkets(id, id, "round_end", true, events);
            }
        }
    }
    pub(super) fn trinket_battle_end(&mut self, events: &mut Vec<Event>) {
        if self.lifecycle.items.ended {
            return;
        }
        self.lifecycle.items.ended = true;
        for id in 0..self.scenario.heroes.len() {
            if self.units[id].alive() {
                self.fire_trinkets(id, id, "kill_all_monsters", true, events);
                self.fire_trinkets(id, id, "battle_finished_successfully", true, events);
            }
        }
    }
}
