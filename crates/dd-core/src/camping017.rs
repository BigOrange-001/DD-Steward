//! A real field phase: finite firewood/food, selected source skills and native
//! refusal/health/stress settlement. No combat turn is synthesized for camping.
use super::*;
use dd_data::camp017::{Operation, Skill};
use dd_data::mechanics::ReactionKind;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Deck {
    pub known: BTreeMap<String, Skill>,
    pub selected: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Start,
    Meal(String),
    Skill {
        hero: u64,
        skill: String,
        target: Option<u64>,
    },
    Rest,
    Select {
        hero: u64,
        skills: Vec<String>,
    },
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct View {
    pub phase: String,
    pub points: u32,
    pub nightmare_warning: bool,
    pub ambush_prevention: f64,
    pub meals: Vec<(String, u32, f64, i32)>,
    pub skills: Vec<VisibleSkill>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleEffect {
    pub selection: String,
    pub requirements: Vec<String>,
    pub chance: f64,
    pub operation: Operation,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VisibleSkill {
    pub hero: u64,
    pub id: String,
    pub cost: u32,
    pub remaining: u32,
    pub effects: Vec<VisibleEffect>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    pub participants: Vec<u64>,
    pub meal: bool,
    pub points: u32,
    pub uses: BTreeMap<u64, BTreeMap<String, u32>>,
    pub ambush_reduction: f64,
    pub nightmare: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct FieldBuff {
    pub instance: u64,
    pub hero: u64,
    pub source: String,
    pub buff: dd_data::mechanics::Buff,
    pub remaining: Option<u32>,
}
impl Session {
    pub(in super::super) fn validate_camping(&self) -> Result<()> {
        let spec = self.spec()?;
        let p = self.state.campaign.as_ref().unwrap();
        // Direct nightmare awards are determined by completed original results,
        // not arbitrary saved inventory. Auto-read pages bypass the finite bag.
        let mut expected = BTreeMap::<Item, u32>::new();
        for id in &p.completed_flashbacks {
            let f = spec
                .sources
                .flashbacks
                .get(id)
                .ok_or("unknown completed nightmare")?;
            for result in f.results.iter().filter(|r| r.success) {
                if let dd_data::nightmare011::FlashOp::Reward { kind, id } = &result.operation {
                    let item = match kind.as_str() {
                        "trinket" => Item::new("trinket", id),
                        "loot" => {
                            let table = self.definition.data.curios.campaign_table(
                                id,
                                spec.sources.quest["difficulty"]
                                    .as_i64()
                                    .ok_or("nightmare reward difficulty missing")?,
                                spec.sources.quest["dungeon"]
                                    .as_str()
                                    .ok_or("nightmare reward dungeon missing")?,
                                &spec.context.infestation,
                            )?;
                            let entries = table["entries"]
                                .as_array()
                                .ok_or("nightmare reward entries missing")?;
                            if entries.len() != 1 || entries[0]["type"] != "journal_page" {
                                return Err("unsupported direct nightmare loot contract".into());
                            }
                            let page =
                                dd_data::curios013::fixed_journal_page(&entries[0]["data"], true)?
                                    .ok_or(
                                        "nightmare journal must have explicit automatic delivery",
                                    )?;
                            Item::new("journal_page", &page.to_string())
                        }
                        _ => return Err("unknown direct nightmare reward".into()),
                    };
                    *expected.entry(item).or_default() += 1;
                }
            }
        }
        let mut actual = BTreeMap::<Item, u32>::new();
        for reward in &p.earned_rewards {
            if reward.amount == 0 {
                return Err("empty direct nightmare reward".into());
            }
            let n = actual.entry(reward.item.clone()).or_default();
            *n = n
                .checked_add(reward.amount)
                .ok_or("direct nightmare reward overflow")?;
        }
        if actual != expected {
            return Err("direct nightmare rewards differ from completed source results".into());
        }
        let validate_selection = |hero: &u64, ids: &[String]| -> Result<()> {
            let deck = spec
                .context
                .camping
                .get(hero)
                .ok_or("camp skill owner absent")?;
            if ids.len() > spec.context.camping_slots
                || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
                || ids.iter().any(|id| !deck.known.contains_key(id))
            {
                return Err("invalid selected camp skills".into());
            }
            Ok(())
        };
        for (hero, ids) in &p.camp_selected {
            validate_selection(hero, ids)?;
        }
        if let Some(c) = &p.camp {
            let maximum = self.camp_points_for(&c.participants)?;
            if self.state.phase != Phase::Expedition
                || self.state.battle.is_some()
                || self.map()?.pending().is_some()
                || c.participants.is_empty()
                || c.participants.iter().collect::<BTreeSet<_>>().len() != c.participants.len()
                || c.participants.iter().any(|id| !spec.canon.contains_key(id))
                || self
                    .state
                    .party
                    .iter()
                    .any(|id| !c.participants.contains(id))
                || c.points > maximum
                || !c.ambush_reduction.is_finite()
                || c.ambush_reduction < 0.
                || (!c.meal
                    && (c.points != maximum
                        || !c.uses.is_empty()
                        || c.ambush_reduction != 0.
                        || c.nightmare.is_some()))
                || c.nightmare.as_ref().is_some_and(|id| {
                    !spec.sources.flashbacks.contains_key(id)
                        || p.dream_attempts.get(id) != Some(&1)
                })
            {
                return Err("invalid active camp boundary".into());
            }
            for (hero, uses) in &c.uses {
                let deck = spec
                    .context
                    .camping
                    .get(hero)
                    .ok_or("camp use owner absent")?;
                if uses
                    .iter()
                    .any(|(id, n)| deck.known.get(id).is_none_or(|s| *n == 0 || *n > s.limit))
                {
                    return Err("invalid camping skill use ledger".into());
                }
            }
        }
        if p.dream_attempts.len() > 1
            || p.dream_attempts
                .iter()
                .any(|(id, n)| *n != 1 || !spec.sources.flashbacks.contains_key(id))
            || p.completed_flashbacks.iter().any(|id| {
                p.dream_attempts.get(id) != Some(&1) || spec.context.flashbacks.contains(id)
            })
        {
            return Err("invalid nightmare attempt/completion ledger".into());
        }
        let mut instances = BTreeSet::new();
        for b in &p.field_buffs {
            let source_match = spec
                .context
                .camping
                .values()
                .flat_map(|deck| deck.known.values())
                .any(|s| {
                    b.source == format!("{}.{}", s.class, s.id)
                        && s.effects
                            .iter()
                            .any(|e| matches!(&e.operation,Operation::Buff(v) if **v == b.buff))
                })
                || spec.sources.flashbacks.values().any(|f| {
                    b.source == format!("completed_flashback:{}", f.id)
                        && f.completed_camping_buffs.contains(&b.buff)
                });
            if !source_match
                || b.instance == 0
                || b.instance > p.next_camp_buff
                || !instances.insert(b.instance)
                || !spec.canon.contains_key(&b.hero)
                || b.remaining == Some(0)
                || (b.buff.duration_type.as_deref() == Some("combat_end")
                    && (b.remaining.is_none() || b.remaining > b.buff.duration))
                || (b.buff.duration_type.as_deref() == Some("quest_end")
                    && b.remaining != b.buff.duration)
            {
                return Err("invalid source camping Buff ledger".into());
            }
        }
        if let Some(aux) = &p.auxiliary {
            let source_match =
                match &aux.kind {
                    AuxKind::Pool { index, .. } => spec
                        .sources
                        .pools
                        .entries
                        .get(*index)
                        .is_some_and(|e| e.monsters == aux.monsters),
                    AuxKind::Curio(id) => spec
                        .sources
                        .named
                        .get(id)
                        .is_some_and(|rows| rows.iter().any(|r| r.monsters == aux.monsters)),
                    AuxKind::Nightmare(id) => {
                        p.dream_attempts.get(id) == Some(&1)
                            && spec.sources.named.values().flatten().any(|r| {
                                r.flashback.as_ref() == Some(id) && r.monsters == aux.monsters
                            })
                    }
                };
            if !source_match || self.state.battle.is_none() || p.camp.is_some() {
                return Err("invalid auxiliary battle/source boundary".into());
            }
        }
        Ok(())
    }
    pub(in super::super) fn camp_active(&self) -> bool {
        self.camp_state().is_some()
    }
    fn camp_state(&self) -> Option<&State> {
        self.state.campaign.as_ref().and_then(|p| p.camp.as_ref())
    }
    pub(in super::super) fn camp_view(&self) -> Result<Option<View>> {
        let Some(c) = self.camp_state() else {
            return Ok(None);
        };
        let mut skills = vec![];
        for hero in &self.state.party {
            if let Some(deck) = self.spec()?.context.camping.get(hero) {
                for id in self.camp_selected(*hero)? {
                    let skill = deck.known.get(&id).ok_or("unknown selected camp skill")?;
                    skills.push(VisibleSkill {
                        hero: *hero,
                        id,
                        cost: skill.cost,
                        remaining: skill.limit.saturating_sub(
                            c.uses
                                .get(hero)
                                .and_then(|m| m.get(&skill.id))
                                .copied()
                                .unwrap_or(0),
                        ),
                        effects: skill
                            .effects
                            .iter()
                            .map(|e| VisibleEffect {
                                selection: e.selection.clone(),
                                requirements: e.requirements.clone(),
                                chance: e.chance,
                                operation: e.operation.clone(),
                            })
                            .collect(),
                    });
                }
            }
        }
        let mut meals = vec![];
        for row in self.definition.data.globals["meals_table"]
            .as_array()
            .ok_or("missing original meal table")?
        {
            meals.push((
                row["type"].as_str().ok_or("missing source meal ID")?.into(),
                self.camp_food_cost(row)?,
                row["healing"].as_f64().ok_or("missing meal HP ratio")?,
                row["stress"].as_i64().ok_or("missing meal stress")? as i32,
            ));
        }
        Ok(Some(View {
            phase: if c.meal { "skills" } else { "meal" }.into(),
            points: c.points,
            nightmare_warning: c.meal && c.nightmare.is_some(),
            ambush_prevention: c.ambush_reduction,
            meals,
            skills,
        }))
    }
    fn camp_selected(&self, hero: u64) -> Result<Vec<String>> {
        Ok(self
            .state
            .campaign
            .as_ref()
            .unwrap()
            .camp_selected
            .get(&hero)
            .cloned()
            .unwrap_or_else(|| {
                self.spec()
                    .unwrap()
                    .context
                    .camping
                    .get(&hero)
                    .map(|d| d.selected.clone())
                    .unwrap_or_default()
            }))
    }
    fn camp_points(&self) -> Result<u32> {
        self.camp_points_for(&self.state.party)
    }
    fn camp_points_for(&self, party: &[u64]) -> Result<u32> {
        let mut points =
            dd_data::campaign017::uint(&self.definition.data.globals, "camp_start_camping_points")?;
        for row in &self.spec()?.context.bonuses.camping {
            let tags = row["hero_type_tags"]
                .as_array()
                .ok_or("missing district camp tags")?;
            if tags.is_empty()
                || party.iter().any(|id| {
                    self.state.heroes[self.hero_index(*id).unwrap()]
                        .base
                        .mechanics
                        .hero
                        .tags
                        .iter()
                        .any(|tag| tags.iter().any(|v| v == tag))
                })
            {
                points = points
                    .checked_add(dd_data::campaign017::uint(row, "camping_points")?)
                    .ok_or("camp point overflow")?;
            }
        }
        Ok(points)
    }
    fn camp_food_cost(&self, row: &Value) -> Result<u32> {
        let rations = row["rations_per"]
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.)
            .ok_or("invalid meal rations")?;
        let k = self.kernel()?;
        Ok(((0..self.state.party.len())
            .map(|i| (1. + k.modifier(i, "food_consumption_percent", "")).max(0.) * rations)
            .sum::<f64>())
        .ceil() as u32)
    }
    pub(in super::super) fn camp_actions(&self) -> Result<Option<Vec<Action>>> {
        if self.definition.campaign.is_none() {
            return Ok(None);
        }
        let Some(c) = self.camp_state() else {
            return Ok(None);
        };
        if !c.meal {
            let mut actions = vec![];
            for row in self.definition.data.globals["meals_table"]
                .as_array()
                .ok_or("missing original meal options")?
            {
                if self.camp_food_cost(row)? <= self.state.bag.count(&Item::new("provision", "")) {
                    actions.push(Action::Meal(
                        row["type"].as_str().ok_or("missing meal ID")?.into(),
                    ));
                }
            }
            return Ok(Some(actions));
        }
        let mut actions = vec![Action::Rest];
        for hero in &self.state.party {
            let Some(deck) = self.spec()?.context.camping.get(hero) else {
                continue;
            };
            for id in self.camp_selected(*hero)? {
                let skill = deck.known.get(&id).ok_or("unknown equipped camp skill")?;
                let used = c
                    .uses
                    .get(hero)
                    .and_then(|m| m.get(&id))
                    .copied()
                    .unwrap_or(0);
                if skill.cost > c.points || used >= skill.limit {
                    continue;
                }
                if skill.effects.iter().any(|e| e.selection == "individual") {
                    actions.extend(self.state.party.iter().filter(|id| *id != hero).map(|id| {
                        Action::Skill {
                            hero: *hero,
                            skill: skill.id.clone(),
                            target: Some(*id),
                        }
                    }));
                } else {
                    actions.push(Action::Skill {
                        hero: *hero,
                        skill: id,
                        target: None,
                    });
                }
            }
        }
        Ok(Some(actions))
    }
    pub(in super::super) fn camp_idle_actions(&self) -> Result<Vec<Action>> {
        if self.definition.campaign.is_none()
            || self.state.phase != Phase::Expedition
            || self.state.battle.is_some()
            || !self.state.loot.is_empty()
            || self.map()?.pending().is_some()
            || self.camp_state().is_some()
        {
            return Ok(vec![]);
        }
        let mut out = vec![];
        let node = self.map()?.position().to_string();
        if self.active_map.map().nodes[&node].room
            && node != self.active_map.map().entrance
            && self.state.bag.count(&Item::new("supply", "firewood")) > 0
        {
            out.push(Action::Start);
        }
        for hero in &self.state.party {
            if let Some(deck) = self.spec()?.context.camping.get(hero) {
                let ids = deck.known.keys().cloned().collect::<Vec<_>>();
                fn combinations(
                    ids: &[String],
                    i: usize,
                    left: usize,
                    current: &mut Vec<String>,
                    out: &mut Vec<Vec<String>>,
                ) {
                    if i == ids.len() {
                        if !current.is_empty() {
                            out.push(current.clone());
                        }
                        return;
                    }
                    combinations(ids, i + 1, left, current, out);
                    if left > 0 {
                        current.push(ids[i].clone());
                        combinations(ids, i + 1, left - 1, current, out);
                        current.pop();
                    }
                }
                let mut sets = vec![];
                combinations(
                    &ids,
                    0,
                    self.spec()?.context.camping_slots,
                    &mut vec![],
                    &mut sets,
                );
                out.extend(sets.into_iter().map(|skills| Action::Select {
                    hero: *hero,
                    skills,
                }));
            }
        }
        Ok(out)
    }
    pub(in super::super) fn camp_act(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Select { hero, skills } => {
                self.state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .camp_selected
                    .insert(hero, skills);
                Ok(())
            }
            Action::Start => {
                self.state
                    .bag
                    .consume(&Item::new("supply", "firewood"), 1)?;
                let points = self.camp_points()?;
                self.state.campaign.as_mut().unwrap().camp = Some(State {
                    participants: self.state.party.clone(),
                    meal: false,
                    points,
                    uses: BTreeMap::new(),
                    ambush_reduction: 0.,
                    nightmare: None,
                });
                Ok(())
            }
            Action::Meal(id) => self.camp_meal(&id),
            Action::Skill {
                hero,
                skill,
                target,
            } => self.camp_skill(hero, &skill, target),
            Action::Rest => self.camp_rest(),
        }
    }
    fn camp_meal(&mut self, id: &str) -> Result<()> {
        let row = self.definition.data.globals["meals_table"]
            .as_array()
            .ok_or("missing source meal table")?
            .iter()
            .find(|r| r["type"] == id)
            .ok_or("unknown meal ID")?
            .clone();
        let starvation = self.definition.data.globals["meals_table"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rations_per"] == 0)
            .ok_or("missing source starving meal")?
            .clone();
        let ratio = row["rations_per"].as_f64().ok_or("missing rations")?;
        let mut cost = 0.;
        self.field(|k| {
            for i in 0..k.core.units.len() {
                let refused = k.core.reaction_block_at_camp(
                    i,
                    i,
                    ReactionKind::BlockCampingMeal,
                    &mut vec![],
                );
                let chosen = if refused { &starvation } else { &row };
                if !refused {
                    cost += (1. + k.modifier(i, "food_consumption_percent", "")).max(0.) * ratio;
                }
                let hp = chosen["healing"].as_f64().ok_or("missing meal healing")?;
                let hp = hp
                    * if hp >= 0. {
                        (1. + k.modifier(i, "hp_heal_received_percent", "eat")).max(0.)
                    } else {
                        (1. + k.modifier(i, "starving_damage_percent", "")).max(0.)
                    };
                k.hp(i, (f64::from(k.maximum(i)) * hp).round() as i32)?;
                let stress = chosen["stress"].as_i64().ok_or("missing meal stress")? as i32;
                let modifier = if stress < 0 {
                    k.modifier(i, "stress_heal_received_percent", "")
                        + k.modifier(i, "stress_heal_received_percent", "camping_eat")
                } else {
                    k.modifier(i, "stress_dmg_received_percent", "camping_eat")
                };
                k.stress(
                    i,
                    (f64::from(stress) * (1. + modifier).max(0.)).round() as i32,
                )?;
            }
            Ok(())
        })?;
        let cost = cost.ceil() as u32;
        if cost > 0 {
            self.state.bag.consume(&Item::new("provision", ""), cost)?;
        }
        self.state
            .campaign
            .as_mut()
            .unwrap()
            .camp
            .as_mut()
            .unwrap()
            .meal = true;
        self.camp_schedule_nightmare()?;
        Ok(())
    }
    fn camp_requirements(&self, hero: u64, requirements: &[String]) -> Result<bool> {
        let h = &self.state.heroes[self.hero_index(hero)?];
        for r in requirements {
            let matches = match r.as_str() {
                "religious" => h.base.mechanics.hero.tags.iter().any(|t| t == "religious"),
                "not_religious" => !h.base.mechanics.hero.tags.iter().any(|t| t == "religious"),
                "afflicted" => matches!(h.state.resolve, Resolve::Afflicted(_)),
                "has_deaths_door_recovery_buffs" => h.state.persistent_buffs.iter().any(|b| {
                    h.base
                        .mechanics
                        .recovery
                        .iter()
                        .any(|original| original.id == b.definition.id)
                }),
                _ => return Err("unknown active camp requirement".into()),
            };
            if !matches {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn camp_skill(&mut self, hero: u64, id: &str, target: Option<u64>) -> Result<()> {
        let skill = self
            .spec()?
            .context
            .camping
            .get(&hero)
            .and_then(|d| d.known.get(id))
            .ok_or("unknown active camp skill")?
            .clone();
        let camp = self.state.campaign.as_mut().unwrap().camp.as_mut().unwrap();
        camp.points = camp
            .points
            .checked_sub(skill.cost)
            .ok_or("insufficient respite")?;
        *camp
            .uses
            .entry(hero)
            .or_default()
            .entry(id.into())
            .or_default() += 1;
        let mut performer_refused = false;
        let mut target_refused = false;
        let actor = self
            .state
            .party
            .iter()
            .position(|g| *g == hero)
            .ok_or("camp performer absent")?;
        let target_index = target.and_then(|g| self.state.party.iter().position(|id| *id == g));
        self.field(|k| {
            performer_refused = k.core.reaction_block_at_camp(
                actor,
                actor,
                ReactionKind::BlockCampingSkillPerformer,
                &mut vec![],
            );
            if !performer_refused && let Some(i) = target_index {
                target_refused = k.core.reaction_block_at_camp(
                    actor,
                    i,
                    ReactionKind::BlockCampingSkillTarget,
                    &mut vec![],
                );
            }
            k.settle()
        })?;
        if performer_refused {
            return Ok(());
        }
        for group in &skill.groups {
            let first = &skill.effects[group[0]];
            let recipients = match first.selection.as_str() {
                "self" => vec![hero],
                "individual" => {
                    if target_refused {
                        vec![]
                    } else {
                        target.into_iter().collect()
                    }
                }
                "party" => self.state.party.clone(),
                "party_other" => self
                    .state
                    .party
                    .iter()
                    .filter(|id| **id != hero && Some(**id) != target)
                    .copied()
                    .collect(),
                _ => return Err("unknown camp recipient selection".into()),
            };
            for recipient in recipients {
                if !self.state.party.contains(&recipient) {
                    continue;
                }
                let mut roll = (self.state.rng.next() >> 11) as f64 / ((1u64 << 53) as f64);
                for index in group {
                    let effect = &skill.effects[*index];
                    if roll < effect.chance {
                        if self.camp_requirements(recipient, &effect.requirements)? {
                            self.camp_operation(
                                recipient,
                                &effect.operation,
                                &format!("{}.{}", skill.class, skill.id),
                                Some(hero),
                            )?;
                        }
                        break;
                    }
                    roll -= effect.chance;
                }
            }
        }
        Ok(())
    }
    fn camp_operation(
        &mut self,
        hero: u64,
        op: &Operation,
        source: &str,
        performer: Option<u64>,
    ) -> Result<()> {
        let index = self
            .state
            .party
            .iter()
            .position(|id| *id == hero)
            .ok_or("camp recipient absent")?;
        let actor = performer
            .and_then(|id| self.state.party.iter().position(|g| *g == id))
            .unwrap_or(index);
        match op {
            Operation::Heal(r) => self.field(|k| {
                let bonus = k.modifier(actor, "hp_heal_percent", "");
                k.hp(
                    index,
                    (f64::from(k.maximum(index)) * r * (1. + bonus).max(0.)).round() as i32,
                )
            }),
            Operation::Damage(r) => {
                self.field(|k| k.hp(index, -(f64::from(k.maximum(index)) * r).round() as i32))
            }
            Operation::Stress(n) => self.field(|k| {
                let bonus = if *n < 0 {
                    k.modifier(index, "stress_heal_received_percent", "")
                } else {
                    0.
                };
                k.stress(index, (f64::from(*n) * (1. + bonus).max(0.)).round() as i32)
            }),
            Operation::BleedCure | Operation::BlightCure => self.field(|k| {
                k.core.units[index].dots.retain(|d| {
                    !matches!(
                        (&d.kind, op),
                        (EffectKind::Bleed(_), Operation::BleedCure)
                            | (EffectKind::Blight(_), Operation::BlightCure)
                    )
                });
                k.settle()
            }),
            Operation::RecoveryCure => self.field(|k| {
                k.core.units[index]
                    .buffs
                    .retain(|b| b.scope != BuffScope::Recovery);
                k.settle()
            }),
            Operation::DiseaseCure => {
                let h = &self.state.heroes[self.hero_index(hero)?];
                let ids = h
                    .state
                    .diseases
                    .iter()
                    .filter_map(|id| {
                        self.definition
                            .data
                            .quirk_defs
                            .get(id)
                            .filter(|v| v["can_remove_with_camping_skill"] == true)
                            .map(|_| id.clone())
                    })
                    .collect::<Vec<_>>();
                self.field(|k| {
                    for id in ids {
                        k.core.remove_disease(index, &id);
                    }
                    k.settle()
                })
            }
            Operation::PreventAmbush(n) => {
                self.state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .camp
                    .as_mut()
                    .unwrap()
                    .ambush_reduction += n;
                Ok(())
            }
            Operation::Torch(n) => {
                if self.definition.field_rules[0].bloodlight.is_none() {
                    self.state.torch = (i32::from(self.state.torch) + n).clamp(0, 100) as u8;
                }
                Ok(())
            }
            Operation::Buff(b) => {
                let p = self.state.campaign.as_mut().unwrap();
                p.next_camp_buff = p
                    .next_camp_buff
                    .checked_add(1)
                    .ok_or("camp buff identity overflow")?;
                p.field_buffs.push(FieldBuff {
                    instance: p.next_camp_buff,
                    hero,
                    source: source.into(),
                    buff: *b.clone(),
                    remaining: b.duration,
                });
                self.field_with_hp_policy(|k| k.settle(), true)
            }
            Operation::Loot { table, count } => self.draws(&[dd_data::curios013::Draw {
                table: table.clone(),
                count: *count,
            }]),
            Operation::RefreshUses => {
                self.state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .camp
                    .as_mut()
                    .unwrap()
                    .uses
                    .remove(&hero);
                Ok(())
            }
        }
    }
    pub(in super::super) fn camp_field_buffs(&self, k: &mut PartyKernel) -> Result<()> {
        let Some(progress) = &self.state.campaign else {
            return Ok(());
        };
        for b in &progress.field_buffs {
            if self.definition.game.noncombat_buff(&b.buff.id)?
                && let Some(i) = self.state.party.iter().position(|id| *id == b.hero)
            {
                k.core
                    .add_buff(i, b.buff.clone(), BuffScope::Build, &mut vec![]);
            }
        }
        k.core.refresh_context(false);
        Ok(())
    }
    pub(in super::super) fn project_camp_buffs(
        &self,
        hero: u64,
        blueprint: &mut Blueprint,
    ) -> Result<()> {
        if let Some(p) = &self.state.campaign {
            for b in p.field_buffs.iter().filter(|b| b.hero == hero) {
                if !self.definition.game.noncombat_buff(&b.buff.id)? {
                    let mut definition = b.buff.clone();
                    definition.duration_type = Some("quest_end".into());
                    definition.activation.remove_on_battle_complete = false;
                    blueprint
                        .mechanics
                        .hero
                        .host_buffs
                        .push(dd_data::hero010::HostBuff {
                            instance: b.instance,
                            definition,
                        });
                }
            }
        }
        self.definition.game.bind_host_buff_effects(blueprint)?;
        Ok(())
    }
    pub(in super::super) fn reconcile_camp_buffs(
        &mut self,
        states: &[HeroResult],
        guids: &[u64],
    ) -> Result<()> {
        let Some(p) = self.state.campaign.as_mut() else {
            return Ok(());
        };
        let existing = p
            .field_buffs
            .iter()
            .map(|b| (b.instance, b.hero))
            .collect::<BTreeMap<_, _>>();
        for h in states {
            let hero = *guids.get(h.id).ok_or("unknown host buff hero receipt")?;
            if h.host_buff_instances
                .iter()
                .any(|id| existing.get(id) != Some(&hero))
            {
                return Err("foreign native host buff instance".into());
            }
        }
        let mut keep = std::collections::BTreeSet::new();
        for b in &p.field_buffs {
            if self.definition.game.noncombat_buff(&b.buff.id)?
                || !guids.contains(&b.hero)
                || states.iter().any(|h| {
                    guids.get(h.id) == Some(&b.hero) && h.host_buff_instances.contains(&b.instance)
                })
            {
                keep.insert(b.instance);
            }
        }
        p.field_buffs.retain(|b| keep.contains(&b.instance));
        Ok(())
    }
    pub(in super::super) fn camp_after_battle(&mut self) -> bool {
        if let Some(p) = &mut self.state.campaign {
            let count = p.field_buffs.len();
            for b in &mut p.field_buffs {
                if b.buff.duration_type.as_deref() == Some("combat_end") {
                    b.remaining = b.remaining.map(|n| n.saturating_sub(1));
                }
            }
            p.field_buffs.retain(|b| b.remaining != Some(0));
            return count != p.field_buffs.len();
        }
        false
    }
    fn camp_schedule_nightmare(&mut self) -> Result<()> {
        if self
            .spec()?
            .context
            .automation
            .as_ref()
            .is_some_and(|a| a.no_random_ambush)
        {
            return Ok(());
        }
        let p = self.state.campaign.as_ref().unwrap();
        if p.dream_attempts.values().any(|n| *n > 0) {
            return Ok(());
        }
        let completed = self
            .spec()?
            .context
            .flashbacks
            .union(&p.completed_flashbacks)
            .cloned()
            .collect::<BTreeSet<_>>();
        let required_present = |f: &dd_data::nightmare011::Flashback| {
            f.required.iter().all(|class| {
                self.state
                    .party
                    .iter()
                    .any(|id| self.state.heroes[self.hero_index(*id).unwrap()].base.id == *class)
            })
        };
        let sources = &self.spec()?.sources;
        let candidates = sources
            .flashbacks
            .values()
            .filter(|f| {
                !completed.contains(&f.id)
                    && (f.trigger.prerequisite.is_empty()
                        || completed.contains(&f.trigger.prerequisite))
                    && required_present(f)
                    && (f.allow_virtue
                        || !self.state.heroes.iter().any(|h| {
                            self.state.party.contains(&h.guid)
                                && f.required.contains(&h.base.id)
                                && matches!(h.state.resolve, Resolve::Virtuous(_))
                        }))
                    && sources
                        .named
                        .values()
                        .flatten()
                        .any(|row| row.flashback.as_deref() == Some(&f.id))
            })
            .cloned()
            .collect::<Vec<_>>();
        if candidates.len() > 1 {
            return Err("multiple eligible nightmare progression branches".into());
        }
        if let Some(f) = candidates.first() {
            if self.state.rng.roll(f.trigger.chance) {
                let p = self.state.campaign.as_mut().unwrap();
                let n = p.dream_attempts.entry(f.id.clone()).or_default();
                *n = n.checked_add(1).ok_or("nightmare attempt overflow")?;
                if *n > f.trigger.raid_limit {
                    return Err("original nightmare raid limit exceeded".into());
                }
                p.camp.as_mut().unwrap().nightmare = Some(f.id.clone());
            }
        } else if !sources.flashbacks.is_empty()
            && sources.flashbacks.keys().all(|id| completed.contains(id))
        {
            let terminal = sources
                .flashbacks
                .values()
                .filter(|f| {
                    !sources
                        .flashbacks
                        .values()
                        .any(|next| next.trigger.prerequisite == f.id)
                })
                .cloned()
                .collect::<Vec<_>>();
            if terminal.len() != 1 {
                return Err("ambiguous completed nightmare chain".into());
            }
            let f = &terminal[0];
            let recipients = self
                .state
                .heroes
                .iter()
                .filter(|h| self.state.party.contains(&h.guid) && f.required.contains(&h.base.id))
                .map(|h| h.guid)
                .collect::<Vec<_>>();
            for hero in recipients {
                for buff in &f.completed_camping_buffs {
                    self.camp_operation(
                        hero,
                        &Operation::Buff(Box::new(buff.clone())),
                        &format!("completed_flashback:{}", f.id),
                        None,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn camp_rest(&mut self) -> Result<()> {
        let c = self
            .state
            .campaign
            .as_mut()
            .unwrap()
            .camp
            .take()
            .ok_or("no active camp")?;
        self.state.torch =
            dd_data::campaign017::uint(&self.definition.data.globals, "camp_restore_torch")?
                .try_into()
                .map_err(|_| "invalid original camp light")?;
        if let Some(id) = c.nightmare {
            let choices = self
                .spec()?
                .sources
                .named
                .values()
                .flatten()
                .filter(|row| row.flashback.as_deref() == Some(&id))
                .cloned()
                .collect::<Vec<_>>();
            let i = pick(
                &mut self.state.rng,
                &choices.iter().map(|c| c.weight).collect::<Vec<_>>(),
            )?;
            self.state.campaign.as_mut().unwrap().auxiliary = Some(Auxiliary {
                monsters: choices[i].monsters.clone(),
                kind: AuxKind::Nightmare(id),
            });
            return self.start_battle();
        }
        if self
            .spec()?
            .context
            .automation
            .as_ref()
            .is_some_and(|a| a.no_random_ambush)
        {
            return Ok(());
        }
        let chance = self.definition.data.globals["ambush_camping_base_chance"]
            .as_f64()
            .ok_or("missing original camp ambush chance")?;
        if self
            .state
            .rng
            .roll((chance - c.ambush_reduction).clamp(0., 1.))
        {
            if self.definition.field_rules[0].bloodlight.is_none() {
                self.state.torch = (i32::from(self.state.torch)
                    + self.definition.data.globals["ambush_torch_reduction"]
                        .as_i64()
                        .ok_or("missing ambush light delta")? as i32)
                    .clamp(0, 100) as u8;
            }
            self.campaign_aux_pool("hall")?;
        }
        Ok(())
    }
    pub(in super::super) fn camp_battle_context(
        &mut self,
        rules: &mut dd_data::combat::Rules,
        context: &mut dd_data::build010::BattleContext,
    ) -> Result<()> {
        let Some(aux) = self
            .state
            .campaign
            .as_ref()
            .and_then(|p| p.auxiliary.as_ref())
        else {
            return Ok(());
        };
        match &aux.kind {
            AuxKind::Nightmare(id) => {
                rules.flashback = Some(
                    self.spec()?
                        .sources
                        .flashbacks
                        .get(id)
                        .ok_or("unknown planned nightmare")?
                        .clone(),
                );
                context.stall = false;
                context.surprise = dd_data::build010::Surprise::Neither;
                rules
                    .lighting
                    .as_mut()
                    .ok_or("nightmare needs explicit lighting")?
                    .torch = self.state.torch;
            }
            AuxKind::Pool { ambush: true, .. } => {
                let heroes = self.definition.data.globals["surprise_ambush_party_base_chance"]
                    .as_f64()
                    .ok_or("missing ambush hero surprise source")?;
                let monsters = self.definition.data.globals["surprise_ambush_monsters_base_chance"]
                    .as_f64()
                    .ok_or("missing ambush monster surprise source")?;
                context.surprise = if self.state.rng.roll(heroes) {
                    dd_data::build010::Surprise::Heroes
                } else if self.state.rng.roll(monsters) {
                    dd_data::build010::Surprise::Monsters
                } else {
                    dd_data::build010::Surprise::Neither
                };
            }
            _ => {}
        }
        Ok(())
    }
    pub(in super::super) fn complete_auxiliary(
        &mut self,
        result: &crate::BattleResult,
    ) -> Result<bool> {
        let Some(aux) = self
            .state
            .campaign
            .as_mut()
            .and_then(|p| p.auxiliary.take())
        else {
            return Ok(false);
        };
        if let AuxKind::Nightmare(id) = aux.kind {
            let f = self
                .spec()?
                .sources
                .flashbacks
                .get(&id)
                .ok_or("unknown native nightmare result")?;
            if result.flashback_id.as_deref() != Some(&id)
                || result.flashback_result.is_none()
                || result.completed_flashback_camping_buffs
                    != if result.flashback_result == Some(true) {
                        f.completed_camping_buffs.clone()
                    } else {
                        vec![]
                    }
            {
                return Err("native nightmare contract differs from planned source".into());
            }
            if result.flashback_result == Some(true) {
                if !self
                    .state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .completed_flashbacks
                    .insert(id)
                {
                    return Err("duplicate nightmare completion".into());
                }
                for (kind, id) in &result.encounter_rewards {
                    match kind.as_str() {
                        "loot" => self.draws(&[dd_data::curios013::Draw {
                            table: id.clone(),
                            count: 1,
                        }])?,
                        "trinket" => {
                            self.state
                                .campaign
                                .as_mut()
                                .unwrap()
                                .earned_rewards
                                .push(Stack {
                                    item: Item::new("trinket", id),
                                    amount: 1,
                                })
                        }
                        _ => return Err("unknown native nightmare reward".into()),
                    }
                }
            }
            if result.outcome == Outcome::FlashbackFailed {
                return Ok(true);
            }
        }
        match result.outcome {
            Outcome::HeroesWin | Outcome::Retreated => {}
            Outcome::MonstersWin | Outcome::MutualDefeat => self.state.party.clear(),
            other => return Err(format!("unhandled auxiliary encounter result {other:?}")),
        }
        Ok(true)
    }
}

