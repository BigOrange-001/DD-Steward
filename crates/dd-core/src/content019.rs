//! Explicit campaign content owner; parsing is never an execution proof.
use super::*;
#[path = "farm_host019.rs"]
pub mod farm;
pub const PROFILE: &str = "candidate-campaign-019-v1";
impl crate::town016::Session {
    pub(in crate::town016) fn contagion_immune019(&self, guid: u64) -> Result<bool> {
        if !self
            .state
            .campaign
            .as_ref()
            .is_some_and(|c| c.owner.starts_with(PROFILE))
        {
            return Ok(false);
        }
        let h = self.hero(guid)?;
        for id in h.quirks.keys().chain(&h.state.diseases) {
            let raw = library::unique_json(
                &self.definition.data.base.quirk_defs,
                id,
                "019 contagion immunity",
            )?;
            if raw["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|v| v == "contagions_immune"))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(in crate::town016) fn mercenary_count019(&self) -> u32 {
        if !self
            .state
            .campaign
            .as_ref()
            .is_some_and(|c| c.owner.starts_with(PROFILE))
        {
            return 0;
        }
        self.state
            .heroes
            .iter()
            .filter(|h| h.state.alive && h.quirks.contains_key("shard_hungry"))
            .count() as u32
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub sources: Option<dd_data::content019::FarmSources>,
    pub farm_curios: Option<dd_data::curios013::Curios>,
    pub provisions: BTreeMap<String, Value>,
    pub class_shards: BTreeMap<String, f64>,
    pub candidate: Value,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Owned {
    active: Option<farm::Active>,
    missing: BTreeMap<u64, u32>,
    visit: Option<u32>,
    recruits: Vec<Hero>,
    stock: BTreeMap<String, u32>,
    sold: BTreeMap<String, u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Buy(String),
    Hire(u64),
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct View {
    pub farm_enabled: bool,
    pub farm: Option<farm::View>,
    pub recruits: Vec<(u64, String, u32, Vec<String>)>,
    pub trinkets: Vec<(String, u32, u32)>,
    pub actions: Vec<Action>,
}
impl Definition {
    pub fn with_content019(mut self) -> Result<Self> {
        use sha2::{Digest, Sha256};
        if self.automation.is_none() || self.content019.is_some() {
            return Err("019 requires exactly one explicit 018 base definition".into());
        }
        let mut class_shards = BTreeMap::new();
        for class in self.town.game.hero_ids() {
            let doc = self.town.game.hero_document(class)?;
            let rows = doc.records_keyed("extra_shard_bonus").collect::<Vec<_>>();
            if rows.len() > 1
                || rows
                    .iter()
                    .any(|r| r.params.iter().any(|p| p.name != "amount"))
            {
                return Err("unknown/ambiguous class shard bonus".into());
            }
            let amount = rows
                .first()
                .map(|r| dd_data::combat::number(r, "amount"))
                .transpose()?
                .unwrap_or(0.);
            if !amount.is_finite() || amount < 0. {
                return Err("invalid class shard bonus".into());
            }
            class_shards.insert(class.clone(), amount);
        }
        let rules = Rules {
            class_shards,
            provisions: dd_data::content019::provision_additions(&self.town.game)?,
            sources: dd_data::content019::FarmSources::load(&self.town.game, &self.town.data)?,
            farm_curios: if self
                .town
                .data
                .content
                .packages
                .iter()
                .any(|p| p == "color_of_madness")
            {
                Some(dd_data::curios013::Curios::load_campaign(
                    &self.town.game.overlay,
                )?)
            } else {
                None
            },
            candidate: serde_json::from_str(r#"{
  "version": "candidate-content-019-v1",
  "source": "enabled original town/quest/wave sources; task019 explicit candidates, awaiting original calibration",
  "semantics": {
    "shard_market": "After the first finite farm mission, show the original number of unique eligible comet entries in stable ID order. Stock refreshes once per real town visit; purchased instances count against original limits. No gold discounts or invented resale price.",
    "mercenary_market": "Original required plot, recruit count, resolve level, starting quirks and minima. All enabled combat skills learned; source rank/equipment trees at that level. Refresh per real town visit. Shard-hungry mercenaries use the source special roster_limit independently of ordinary roster slots.",
    "navigation": "Accumulate only filtered visible geometry under public quest instance and expedition visit; persistent courtyard identities may retain seen geometry across sorties. Key counts always come from the current observation. No hidden encounter memory. New runner save wrapper binds memory version and rejects legacy snapshots.",
    "farm_exit": "Finite victories require the native 011 finite controller to finish. Endless has no host group cap. The original can_retreat and retreat_always_from_raid flags expose a guaranteed host Exit at a settled action boundary, including during a wave; this returns actual current hero/loot/progress state without fabricating native victory. An endless repeatable contract qualifies only after the original completion_threshold. Native chance-based Retreat is not offered as an alternative when the source authorizes unconditional raid exit. At the actual final finite interlude, Exit first asks the native controller to settle its achieved finite objective; it cannot relabel that earned result as abandonment and does not discard loot before the player chooses to return.",
    "farm_loot": "Each native killed entity is paid once. Monster loot uses current native region. Shard ranges select boss-tag, size 2+, or single; original threshold multiplier applies to each reached threshold shard award. Interlude awards use unique group identities. These selection/timing semantics remain candidates.",
    "farm_return_rewards": "Highest reached original threshold row supplies return XP and items; not cumulative. Original per-group threshold loot remains cumulative. Surviving heroes only receive threshold XP. Original class/quirk/trinket shard modifiers combine additively per category; gain then total mercenary fee, floor once. Lost mercenaries still charge.",
    "farm_loss": "Original in_time_and_space converts native deaths into source-duration missing heroes with source stress and source severity-1 affliction; removes death-sensitive quirks. Equipped trinkets are lost through existing native asset-loss consumer. A wipe loses carried inventory but retains earned shard and memory escrow. Ordinary finite-mission deaths stay dead.",
    "farm_camping": "Original com_fireplace starts camp without consuming player firewood. Missing inner source outcome weight is explicitly one; source outer weight is retained. Source camping meals, skills, limits, costs, buffs and finite supplies execute on the native party. No walking, hunger checks, night ambush, nightmares or player torch/light changes in Farmstead.",
    "farm_refresh": "The two finite Farmstead contracts use original enabled town progression goals for incomplete returns, and always progress on actual completion. No-progress exit from the starting stone does not progress a week. Native counted Farmstead kills supply the original survival goal; native deaths-door/affliction events supply their original gates. The survival counter interpretation remains a candidate. Endless return advances visit and refreshes source quest board without ticking town jobs or campaign week. Missing heroes return only when actual campaign weeks reach source duration. Markets refresh once per real town return, including non-progressing ordinary raids.",
    "farm_physical_items": "Native physical trinket state and original asset origin survive pickup, growth, transformation and return. Dead ordinary heroes drop recoverable finite loot; lost-in-time heroes do not drop equipped assets locally.",
    "provisions": "Merge only enabled indexed original campaign/provision additions and class grants. Source purchase_shard_value uses shard currency without gold discounts; per-currency invoice conservation and cancellation are strict.",
    "observation": "Farm policy sees filtered native battle, finite public inventory, revealed current curio and completed group identities. It does not receive native shard ranges, multiplier, future formations, hidden RNG or host snapshots.",
    "farm_entry": "Original starting_curio com_standingstone and exact global start_endless/skips_endless_wave_curio effect form an explicit once-only Start/Exit entry boundary. Native wave initialization may be privately precomputed, but no enemy observation, combat consumable debit or advancing enemy action is owned before Start. Entry observation and early-exit hero state use the same original hero blueprints in a round-zero PartyKernel. Early exit never fabricates a native victory or threshold; initial hero-only normalization uses a separate deterministic kernel RNG. Timing remains a candidate pending executable calibration.",
    "mercenary_contagion": "In the explicit 019 campaign owner, original contagions_immune quirk/disease tags exclude a hero from town transmission targets before any RNG roll. Original contact probabilities, roster limits and susceptible controls remain unchanged. This applies the source tag rather than recognizing a hardcoded mercenary GUID."
  },
  "evidence_links": [
    "https://darkestdungeon.wiki.gg/wiki/Farmstead",
    "https://darkestdungeon.wiki.gg/wiki/Stage_Coach",
    "https://darkestdungeon.wiki.gg/wiki/Experience",
    "https://darkestdungeon.wiki.gg/wiki/Quirks",
    "https://store.steampowered.com/news/posts/?appids=262060%2C345800&enddate=1585609536&feed=steam_community_announcements",
    "https://steamcommunity.com/sharedfiles/filedetails/?id=819597757&l=english"
  ],
  "calibration": "All numerical values are parsed from enabled local original inputs; external descriptions motivate candidates only. No claim of original-executable equivalence or production training suitability."
}
"#)
            .map_err(|e| e.to_string())?,
        };
        if let Some(curios) = &rules.farm_curios {
            let mut ids = BTreeSet::new();
            for stage in 0..3 {
                let plan = self.town.game.farm_plan(
                    &format!("plot_color_of_madness_{stage}"),
                    self.data.quests.keys().cloned().collect(),
                )?;
                for group in &plan.params.curio_groups {
                    ids.extend(group.curios.iter().cloned());
                }
                for group in plan
                    .params
                    .intro_wave_groups
                    .iter()
                    .chain(&plan.params.recurring_wave_groups)
                {
                    for wave in &group.wave_infos {
                        for id in std::iter::once(&wave.curio).chain(&wave.curio_names) {
                            if let Some(group) =
                                plan.params.curio_groups.iter().find(|g| &g.name == id)
                            {
                                ids.extend(group.curios.iter().cloned());
                            } else {
                                ids.insert(id.clone());
                            }
                        }
                    }
                }
                for table in plan.tables.values() {
                    ids.extend(table.curios.iter().cloned());
                    ids.extend(table.starting_curio.iter().cloned());
                }
            }
            for id in ids.into_iter().filter(|id| !id.is_empty()) {
                if id == "com_standingstone" {
                    curios.validate_farm_stone019()?;
                    self.town.game.validate_farm_start019()?;
                    continue;
                }
                if id == "com_fireplace" {
                    curios.farm_fireplace019()?;
                    continue;
                }
                let curio = curios
                    .campaign_curio(&id)
                    .map_err(|e| format!("farm curio {id}: {e}"))?;
                for result in curio
                    .outcomes
                    .iter()
                    .map(|(r, _)| r)
                    .chain(curio.items.values())
                {
                    use dd_data::{curios013::Outcome, field017::ResultKind};
                    match result {
                        ResultKind::Basic(Outcome::Nothing | Outcome::Loot { .. }) => {}
                        ResultKind::Basic(Outcome::Effect(effects)) => {
                            for (effect, _) in effects {
                                self.town.game.field_effect(effect)?;
                            }
                        }
                        _ => return Err(format!("unknown enabled farm curio mechanism {id}")),
                    }
                }
            }
        }
        self.id = format!(
            "{PROFILE};sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&(&self.id, &rules)).map_err(|e| e.to_string())?)
        );
        self.content019 = Some(Arc::new(rules));
        self.town_view = Arc::new(Mutex::new(None));
        Ok(self)
    }
}
impl Session {
    pub(super) fn validate_content019(&self) -> Result<()> {
        match (&self.definition.content019, &self.state.content019) {
            (None, None) => Ok(()),
            (Some(rules), Some(s)) => {
                self.validate_farm019()?;
                if s.visit.is_some_and(|v| v > self.state.city.visit)
                    || s.recruits.len() > 10000
                    || rules.sources.is_none() && (!s.stock.is_empty() || !s.recruits.is_empty())
                {
                    return Err("invalid 019 market ownership/source boundary".into());
                }
                let mut ids = BTreeSet::new();
                let mut town = self.town();
                for h in &s.recruits {
                    if !ids.insert(h.guid)
                        || town
                            .state
                            .heroes
                            .iter()
                            .chain(&town.state.recruits)
                            .any(|old| old.guid == h.guid)
                    {
                        return Err("duplicate 019 mercenary identity".into());
                    }
                    let source = &rules
                        .sources
                        .as_ref()
                        .ok_or("019 recruits without source")?
                        .mercenaries;
                    if town.level(h.xp)? != source.starting_resolve_level
                        || source
                            .starting_quirks
                            .iter()
                            .any(|q| !h.quirks.contains_key(q))
                    {
                        return Err("019 mercenary source mismatch".into());
                    }
                    town.state.recruits.push(h.clone());
                }
                town.validate()?;
                for (id, n) in &s.stock {
                    let raw = library::unique_json(
                        &self.definition.town.data.base.curios.trinkets,
                        id,
                        "019 stock",
                    )?;
                    if raw["rarity"] != "comet"
                        || *n > 1
                        || s.sold.get(id).copied().unwrap_or(0) + *n != 1
                    {
                        return Err("019 stock ledger/source mismatch".into());
                    }
                    dd_data::campaign017::uint(raw, "shard")?;
                }
                if s.sold.keys().any(|id| !s.stock.contains_key(id)) {
                    return Err("019 sale without stock provenance".into());
                }
                Ok(())
            }
            _ => Err(
                "019 profile requires explicit matching owned state; no implicit snapshot upgrade"
                    .into(),
            ),
        }
    }
    pub(super) fn refresh_content019(&mut self) -> Result<()> {
        let Some(rules) = self.definition.content019.clone() else {
            return Ok(());
        };
        let owned = self.state.content019.as_ref().ok_or("missing 019 state")?;
        if owned.visit == Some(self.state.city.visit) {
            return Ok(());
        }
        let mut next = Owned {
            visit: Some(self.state.city.visit),
            active: owned.active.clone(),
            missing: owned
                .missing
                .iter()
                .filter(|(_, until)| **until > self.state.clock.week)
                .map(|(id, until)| (*id, *until))
                .collect(),
            ..Default::default()
        };
        let Some(source) = &rules.sources else {
            self.state.content019 = Some(next);
            return Ok(());
        };
        let mut town = self.town();
        if source.shop.comet_trinkets_to_display > 0
            && town.state.completed_ids.contains("plot_color_of_madness_0")
        {
            for (id, raw) in &self.definition.town.data.base.curios.trinkets {
                if raw["rarity"] != "comet" {
                    continue;
                }
                let limit = dd_data::campaign017::uint(raw, "limit")?;
                if limit > 0
                    && town
                        .state
                        .assets
                        .iter()
                        .filter(|a| a.state.definition.id == *id)
                        .count()
                        >= limit as usize
                {
                    continue;
                }
                if raw["hero_class_requirements"]
                    .as_array()
                    .is_some_and(|rows| {
                        rows.iter().any(|v| {
                            v.as_str().is_none_or(|c| {
                                !self.definition.town.game.hero_ids().any(|id| id == c)
                            })
                        })
                    })
                {
                    continue;
                }
                dd_data::campaign017::uint(raw, "shard")?;
                town.definition.trinket(id)?;
                next.stock.insert(id.clone(), 1);
                if next.stock.len() >= source.shop.comet_trinkets_to_display as usize {
                    break;
                }
            }
        }
        if town
            .state
            .completed_ids
            .contains(&source.mercenaries.required_completed_plot_quest)
        {
            for _ in 0..source.mercenaries.number_of_recruits {
                let mut classes = vec![];
                let mut weights = vec![];
                for id in self.definition.town.game.hero_ids() {
                    let doc = self.definition.town.game.hero_document(id)?;
                    let r = doc
                        .records_keyed("generation")
                        .next()
                        .ok_or("missing mercenary class generation")?;
                    if dd_data::combat::text(r, "is_generation_enabled")? != "true" {
                        continue;
                    }
                    if r.param("town_event_dependency").is_some()
                        && !town
                            .state
                            .recruit_events
                            .contains(dd_data::combat::text(r, "town_event_dependency")?)
                    {
                        continue;
                    }
                    classes.push(id.clone());
                    weights.push(
                        dd_data::combat::number(r, "number_of_cards_in_deck")?
                            * dd_data::combat::number(r, "card_chance")?,
                    );
                }
                let class = classes[town.choose(&weights)?].clone();
                next.recruits
                    .push(town.generate_mercenary019(&class, &source.mercenaries)?);
            }
        }
        self.state.city = town.state;
        self.state.content019 = Some(next);
        Ok(())
    }
    pub(super) fn content019_view(&self) -> Result<Option<View>> {
        let Some(owned) = &self.state.content019 else {
            return Ok(None);
        };
        let town = self.town();
        let busy = self.raid_busy() || self.state.pending.is_some() || self.failure()?.is_some();
        let mut view = View {
            farm_enabled: self
                .definition
                .content019
                .as_ref()
                .is_some_and(|r| r.sources.is_some()),
            farm: self.farm019_view()?,
            recruits: vec![],
            trinkets: vec![],
            actions: vec![],
        };
        for h in &owned.recruits {
            view.recruits.push((
                h.guid,
                h.class.clone(),
                town.level(h.xp)?,
                h.quirks.keys().cloned().collect(),
            ));
            if !busy && self.mercenary_eligible019(h)? {
                view.actions.push(Action::Hire(h.guid));
            }
        }
        for (id, n) in &owned.stock {
            if *n == 0 {
                continue;
            }
            let price = dd_data::campaign017::uint(
                library::unique_json(
                    &self.definition.town.data.base.curios.trinkets,
                    id,
                    "019 shop",
                )?,
                "shard",
            )?;
            view.trinkets.push((id.clone(), price, *n));
            if !busy
                && town
                    .state
                    .wallet
                    .get(&town.currency("shard")?)
                    .copied()
                    .unwrap_or(0)
                    >= price
            {
                view.actions.push(Action::Buy(id.clone()));
            }
        }
        Ok(Some(view))
    }
    fn mercenary_eligible019(&self, h: &Hero) -> Result<bool> {
        let town = self.town();
        for id in h.quirks.keys() {
            let raw = library::unique_json(
                &self.definition.town.data.base.quirk_defs,
                id,
                "mercenary roster limit",
            )?;
            if let Some(limit) = raw["roster_limit"].as_u64()
                && town
                    .state
                    .heroes
                    .iter()
                    .filter(|h| h.state.alive && h.quirks.contains_key(id))
                    .count()
                    >= limit as usize
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn content019_action(&mut self, action: Action) -> Result<()> {
        self.transaction(|s| {
            if !s
                .content019_view()?
                .is_some_and(|v| v.actions.contains(&action))
            {
                return Err("019 action locked, unaffordable, exhausted or busy".into());
            }
            let mut town = s.town();
            match action {
                Action::Buy(ref id) => {
                    let raw = library::unique_json(
                        &s.definition.town.data.base.curios.trinkets,
                        id,
                        "019 purchase",
                    )?;
                    let limit = dd_data::campaign017::uint(raw, "limit")?;
                    if limit > 0
                        && town
                            .state
                            .assets
                            .iter()
                            .filter(|a| a.state.definition.id == *id)
                            .count()
                            >= limit as usize
                    {
                        return Err("019 trinket limit reached".into());
                    }
                    let price = dd_data::campaign017::uint(raw, "shard")?;
                    town.charge(&BTreeMap::from([(town.currency("shard")?, price)]))?;
                    town.add_asset(id, None)?;
                    let owned = s.state.content019.as_mut().unwrap();
                    *owned.stock.get_mut(id).ok_or("missing stock")? -= 1;
                    *owned.sold.entry(id.clone()).or_default() += 1;
                }
                Action::Hire(id) => {
                    if let Some(goals) = &mut s.state.goals020 {
                        goals.mercenaries.insert(id);
                    }
                    let owned = s.state.content019.as_mut().unwrap();
                    let index = owned
                        .recruits
                        .iter()
                        .position(|h| h.guid == id)
                        .ok_or("unknown mercenary")?;
                    let h = owned.recruits.remove(index);
                    town.install_hero_purchases(&h)?;
                    town.state.heroes.push(h);
                }
            }
            town.state.log.push(json!({"content019":action}));
            s.state.city = town.state;
            Ok(())
        })
    }
}

