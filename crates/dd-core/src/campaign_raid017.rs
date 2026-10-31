//! The campaign owns provisions, deployment and the terminal expedition receipt.
use super::*;
use board::Instance;
use dd_data::inventory015::{Bag, InventoryRules};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Preparation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    commerce019: Option<Commerce019>,
    pub(super) quest: Instance,
    pub(super) party: Vec<u64>,
    pub(super) bag: Bag,
    #[serde(with = "item_map")]
    store: BTreeMap<Item, u32>,
    #[serde(with = "item_map")]
    prices: BTreeMap<Item, u32>,
    paid: u32,
    opening_gold: u32,
    #[serde(with = "item_map")]
    purchases: BTreeMap<Item, u32>,
    #[serde(with = "item_map")]
    discarded: BTreeMap<Item, u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Commerce019 {
    opening: BTreeMap<String, u32>,
    paid: BTreeMap<String, u32>,
    currencies: Vec<(Item, String)>,
}
impl Preparation {
    fn currency(&self, item: &Item) -> Result<&str> {
        if let Some(c) = &self.commerce019 {
            c.currencies
                .iter()
                .find(|(i, _)| i == item)
                .map(|(_, c)| c.as_str())
                .ok_or("019 provision currency missing".into())
        } else {
            Ok("gold:")
        }
    }
}
mod item_map {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        map: &BTreeMap<Item, u32>,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<BTreeMap<Item, u32>, D::Error> {
        let rows = Vec::<(Item, u32)>::deserialize(deserializer)?;
        let mut result = BTreeMap::new();
        for (key, value) in rows {
            if result.insert(key, value).is_some() {
                return Err(serde::de::Error::custom(
                    "duplicate provision item ledger key",
                ));
            }
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Active {
    pub(super) quest: Instance,
    pub(super) snapshot: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProvisionAction {
    Buy { item: Item, amount: u32 },
    Discard { slot: usize, amount: u32 },
    Cancel,
    Embark,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Provisions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies019: Option<Vec<(Item, String)>>,
    pub quest: String,
    pub party: Vec<u64>,
    pub inventory: Bag,
    pub store: Vec<(Item, u32, u32)>,
    pub paid: u32,
    pub actions: Vec<ProvisionAction>,
}
fn stack(v: &Value) -> Result<Stack> {
    Ok(Stack {
        item: Item::new(
            v["type"].as_str().ok_or("missing item kind")?,
            v["id"].as_str().ok_or("missing item ID")?,
        ),
        amount: dd_data::campaign017::uint(v, "amount")?,
    })
}
impl Session {
    pub(super) fn raid_busy(&self) -> bool {
        self.state.preparation.is_some() || self.state.active.is_some() || self.farm019_active()
    }
    pub(super) fn raid_inventory(&self, party: &[u64]) -> Result<InventoryRules> {
        let town = self.town();
        let mut key = party
            .iter()
            .map(|id| Ok(town.hero(*id)?.class.clone()))
            .collect::<Result<Vec<_>>>()?;
        key.sort();
        key.extend(
            self.state
                .city
                .districts
                .iter()
                .map(|id| format!("district:{id}")),
        );
        let cached = self
            .definition
            .inventories
            .lock()
            .map_err(|_| "campaign inventory cache poisoned")?
            .get(&key)
            .cloned();
        if let Some(rules) = cached {
            return Ok(rules);
        }

        let curios = dd_data::curios013::Curios::load_campaign(&self.definition.town.game.overlay)?;
        let mut rules = InventoryRules::load(&self.definition.town.game.overlay, &curios)?;
        for id in party {
            let class = self.town().hero(*id)?.class.clone();
            for (key, amount) in &self
                .definition
                .town
                .data
                .base
                .class_extras
                .get(&class)
                .ok_or("missing source class inventory extras")?
                .limits
            {
                let n = rules.extra_limits.entry(key.clone()).or_default();
                *n = n.checked_add(*amount).ok_or("stack bonus overflow")?;
            }
        }
        for row in self.town().bonuses(party, &mut [])?.stacking {
            let key = row["item"].as_str().ok_or("missing district stack item")?;
            let n = rules.extra_limits.entry(key.into()).or_default();
            *n = n
                .checked_add(dd_data::campaign017::uint(&row, "amount")?)
                .ok_or("district stack overflow")?;
        }
        self.definition
            .inventories
            .lock()
            .map_err(|_| "campaign inventory cache poisoned")?
            .insert(key, rules.clone());
        Ok(rules)
    }
    pub(super) fn execution(&self, q: &Instance) -> &'static str {
        if self.definition.content019.is_some() && q.raw["map_name"] == "town_invasion_0" {
            return "019 named expedition";
        }
        if self
            .definition
            .content019
            .as_ref()
            .is_some_and(|r| r.sources.is_some())
            && matches!(
                q.plot.as_deref(),
                Some(
                    "plot_color_of_madness_0"
                        | "plot_color_of_madness_1"
                        | "plot_color_of_madness_2"
                )
            )
        {
            return "019 native farm expedition";
        }
        if self.definition.automation.is_some()
            && (q
                .plot
                .as_deref()
                .is_some_and(|id| id.starts_with("plot_darkest_dungeon_"))
                || q.raw["dungeon"] == "courtyard"
                    && ["farm_bloodcrate", "kill_statue"]
                        .iter()
                        .any(|k| q.raw["type"] == *k)
                || ["crypts", "weald", "warrens", "cove"]
                    .iter()
                    .any(|r| q.raw["dungeon"] == *r)
                    && q.raw["map_name"].is_null()
                    && [
                        "cleanse",
                        "explore",
                        "gather",
                        "inventory_activate",
                        "kill_boss",
                    ]
                    .iter()
                    .any(|k| q.raw["type"] == *k))
        {
            return "018 native expedition";
        }

        if q.raw["dungeon"] == "courtyard"
            && q.plot.as_deref().is_some_and(|id| {
                [
                    "plot_crimson_court_0",
                    "plot_crimson_court_1",
                    "plot_crimson_court_2",
                    "plot_crimson_court_3",
                ]
                .contains(&id)
            })
        {
            "courtyard integration under acceptance"
        } else if q.raw["dungeon"] == "crypts"
            && ["cleanse", "explore", "gather", "inventory_activate"]
                .iter()
                .any(|k| q.raw["type"] == *k)
        {
            "ordinary ruins integration under acceptance"
        } else if q.raw["map_name"] == "crow_map1" {
            "single encounter retention integration under acceptance"
        } else {
            "expedition integration pending"
        }
    }
    pub(super) fn departure_eligible(&self, q: &Instance) -> Result<()> {
        self.check_party_for_quest(q, false)
    }
    pub(super) fn preparation_eligibility(&self, q: &Instance) -> Result<()> {
        self.check_party_for_quest(q, true)
    }
    pub(super) fn quest_resolve_limit(&self, q: &Instance) -> Result<Option<u32>> {
        if self
            .state
            .city
            .campaign
            .as_ref()
            .is_some_and(|c| c.has("remove_quest_hero_level_restriction", ""))
        {
            return Ok(None);
        }
        let difficulty = dd_data::campaign017::uint(&q.raw, "difficulty")? as usize;
        self.definition.data.restrictions["restriction"]["difficulty"]["resolve_level_threshold_table"][difficulty]
            .as_u64().and_then(|n|u32::try_from(n).ok()).map(Some).ok_or("missing source quest resolve limit".into())
    }
    fn check_party_for_quest(&self, q: &Instance, preview: bool) -> Result<()> {
        if self.failure()?.is_some() || self.state.pending.is_some() {
            return Err("campaign not at a free town boundary".into());
        }
        let mut town = self.town();
        if q.raw["dungeon"] == "darkestdungeon"
            && let Some(auto) = town
                .state
                .campaign
                .as_ref()
                .and_then(|c| c.automation.as_ref())
            && !auto.rules.can_reenter
            && town.state.party.iter().any(|id| auto.marked.contains(id))
        {
            return Err("never again: marked hero cannot reenter under strict setting".into());
        }
        // Board metadata describes this party/dependency eligibility at a free
        // town boundary. The active expedition is exposed separately and the
        // actual prepare command still enforces its exclusive phase.
        if preview {
            town.state.departure = None;
        }
        if town.state.party.is_empty() || town.state.party.iter().any(|id| !town.available(*id)) {
            return Err("no available selected party".into());
        }
        if self.definition.content019.is_some() {
            for id in &town.state.party {
                if town.hero(*id)?.quirks.contains_key("shard_hungry") {
                    let raw = library::unique_json(
                        &self.definition.town.data.base.quirk_defs,
                        "shard_hungry",
                        "mercenary mission eligibility",
                    )?;
                    if !raw["required_plot_quests_to_embark"]
                        .as_array()
                        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == q.plot.as_deref()))
                    {
                        return Err("mercenary requires source-listed farm quest".into());
                    }
                }
            }
        }
        if self.definition.automation.is_some()
            && town.state.party.iter().any(|id| {
                town.hero(*id).is_ok_and(|h| {
                    matches!(h.state.resolve, Resolve::Calm) && h.state.stress >= 100
                })
            })
        {
            return Err("deferred resolve must be treated before deployment".into());
        }
        let difficulty = dd_data::campaign017::uint(&q.raw, "difficulty")? as usize;
        let max = self.definition.data.restrictions["restriction"]["difficulty"]["resolve_level_threshold_table"][difficulty].as_u64().ok_or("missing source resolve restriction")?;
        let exempt = self
            .state
            .city
            .campaign
            .as_ref()
            .is_some_and(|c| c.has("remove_quest_hero_level_restriction", ""));
        if !exempt
            && town.state.party.iter().any(|id| {
                town.hero(*id)
                    .and_then(|h| town.level(h.xp))
                    .is_ok_and(|n| u64::from(n) > max)
            })
        {
            return Err("hero exceeds source quest resolve restriction".into());
        }
        if let Some(items) =
            q.source["estate_inventory_dependency"]["items_definition"]["items"].as_object()
        {
            for v in items.values() {
                let s = stack(v)?;
                if self
                    .state
                    .city
                    .wallet
                    .get(&s.item.key())
                    .copied()
                    .unwrap_or(0)
                    < s.amount
                {
                    return Err(format!("missing estate quest dependency {}", s.item.key()));
                }
            }
        }
        Ok(())
    }
    pub fn prepare_quest(&mut self) -> Result<()> {
        self.transaction(|s| {
            if s.raid_busy() {
                return Err("raid preparation already active".into());
            }
            let id = s
                .state
                .board
                .selected
                .as_ref()
                .ok_or("select a quest first")?;
            let q = s
                .state
                .board
                .entries
                .iter()
                .find(|q| &q.id == id)
                .ok_or("selected quest missing")?
                .clone();
            // Other board definitions retain real unlock status without a false executable action.
            if s.execution(&q) == "expedition integration pending" {
                return Err("quest has no accepted expedition executor yet".into());
            }
            s.departure_eligible(&q)?;
            s.state.preparation = Some(s.provision_template(q)?);
            Ok(())
        })
    }
    fn provision_template(&self, q: Instance) -> Result<Preparation> {
        let party = self.state.city.party.clone();
        let rules = self.raid_inventory(&party)?;
        let length = dd_data::campaign017::uint(&q.raw, "length")? as usize;
        let source = &self.definition.town.data.base.provision;
        let mut bag = Bag::new(&rules);
        let grants = source["raid_starting_length_inventory_item_lists"][length]
            .as_array()
            .ok_or("missing source provision grants")?
            .clone();
        let mut all_grants = grants;
        if let Some(content) = &self.definition.content019 {
            for extra in content.provisions.values() {
                if let Some(rows) = extra.get("raid_starting_length_inventory_item_lists") {
                    all_grants.extend(
                        rows.get(length)
                            .and_then(Value::as_array)
                            .ok_or("019 length grant outside original table")?
                            .clone(),
                    );
                }
                if let Some(rows) = extra["raid_starting_hero_class_item_lists"].as_array() {
                    for row in rows {
                        if party.iter().any(|id| {
                            self.town()
                                .hero(*id)
                                .is_ok_and(|h| Some(h.class.as_str()) == row["hero_class"].as_str())
                        }) {
                            for _ in party.iter().filter(|id| {
                                self.town().hero(**id).is_ok_and(|h| {
                                    Some(h.class.as_str()) == row["hero_class"].as_str()
                                })
                            }) {
                                all_grants.extend(
                                    row["item_lists"]
                                        .as_array()
                                        .ok_or("019 class grants absent")?
                                        .clone(),
                                );
                            }
                        }
                    }
                }
            }
            if let Some(extra) = q.source.get("additional_provisions") {
                dd_data::campaign017::fields(
                    extra,
                    &["system_config_type", "items"],
                    "019 quest provisions",
                )?;
                if extra["system_config_type"] != "quest_provision" {
                    return Err("unknown 019 quest provision system".into());
                }
                all_grants.extend(
                    extra["items"]
                        .as_object()
                        .ok_or("019 quest provision items absent")?
                        .values()
                        .cloned(),
                );
            }
        }
        for id in &party {
            let h = self.town().hero(*id)?.clone();
            for row in source["raid_starting_hero_class_item_lists"]
                .as_array()
                .ok_or("missing source class grants")?
            {
                if row["hero_class"] == h.class {
                    all_grants.extend(
                        row["item_lists"]
                            .as_array()
                            .ok_or("missing class grant items")?
                            .clone(),
                    );
                }
            }
        }
        for goal in q.raw["goal_ids"].as_array().ok_or("missing quest goals")? {
            let g = library::unique_json(
                &self.definition.data.goals,
                goal.as_str().ok_or("invalid goal ID")?,
                "provision goal",
            )?;
            if let Some(items) = g["starting_items"].as_array() {
                all_grants.extend(items.clone());
            }
        }
        for v in all_grants {
            let item = stack(&v)?;
            if item.amount > 0 && bag.insert(&rules, &item)? != item.amount {
                return Err("source provision grants exceed finite bag".into());
            }
        }
        if self.definition.content019.is_some() && q.raw["dungeon"] == "farm" {
            for slot in &mut bag.slots {
                if slot.as_ref().is_some_and(|s| {
                    q.source["disabled_provisions"]
                        .as_array()
                        .is_some_and(|rows| {
                            rows.iter()
                                .any(|r| r["item_type"] == s.item.kind && r["item_id"] == s.item.id)
                        })
                }) {
                    *slot = None;
                }
            }
        }
        for grant in &self.state.city.supply_grants {
            if bag.insert(&rules, grant)? != grant.amount {
                return Err("district provision grant exceeds bag".into());
            }
        }
        if let Some(world) = q.plot.as_ref().and_then(|id| self.state.worlds.get(id)) {
            for key in &world.keys {
                if bag.insert(&rules, key)? != key.amount {
                    return Err("persistent quest keys exceed finite provisions".into());
                }
            }
        }
        let mut store = BTreeMap::new();
        let mut prices = BTreeMap::new();
        let mut commerce019 = self.definition.content019.as_ref().map(|_| Commerce019 {
            opening: self.state.city.wallet.clone(),
            paid: Default::default(),
            currencies: vec![],
        });
        let mut store_rows = source["default_store_inventory_item_lists"][length]
            .as_array()
            .ok_or("missing source provision store")?
            .clone();
        if let Some(content) = &self.definition.content019 {
            for extra in content.provisions.values() {
                if let Some(rows) = extra.get("default_store_inventory_item_lists") {
                    store_rows.extend(
                        rows.get(length)
                            .and_then(Value::as_array)
                            .ok_or("019 store length outside original table")?
                            .clone(),
                    );
                }
            }
        }
        for row in &store_rows {
            let item = stack(row)?;
            if self.definition.content019.is_some()
                && q.raw["dungeon"] == "farm"
                && q.source["disabled_provisions"]
                    .as_array()
                    .is_some_and(|rows| {
                        rows.iter().any(|r| {
                            r["item_type"] == item.item.kind && r["item_id"] == item.item.id
                        })
                    })
            {
                continue;
            }
            let context = self.state.city.campaign.as_ref().unwrap();
            let amount = (f64::from(item.amount)
                * (1. + context.sum("provision_item_type_amount_change", &item.item.kind)).max(0.))
            .floor() as u32;
            let mut price = (f64::from(rules.definition(&item.item)?.purchase)
                * (1. + context.sum("provision_item_type_cost_change", &item.item.kind)).max(0.))
            .round() as u32;
            if let Some(ledger) = &mut commerce019 {
                let raw =
                    library::unique_json(&rules.items, &item.item.key(), "019 provision currency")?;
                let currency = if let Some(shard) = raw["purchase_shard_value"].as_array() {
                    if shard.len() != 1
                        || rules.definition(&item.item)?.purchase != 0
                        || raw["can_modify_cost"] != json!(["false"])
                    {
                        return Err("unknown mixed shard provision quote".into());
                    }
                    price = shard[0]
                        .as_str()
                        .ok_or("invalid shard provision price")?
                        .parse::<u32>()
                        .map_err(|_| "invalid shard provision price")?;
                    self.town().currency("shard")?
                } else {
                    "gold:".into()
                };
                ledger.currencies.push((item.item.clone(), currency));
            }
            if store.insert(item.item.clone(), amount).is_some() {
                return Err("conflicting provision store item".into());
            }
            prices.insert(item.item, price);
        }
        Ok(Preparation {
            commerce019,
            quest: q,
            party,
            bag,
            store,
            prices,
            paid: 0,
            opening_gold: self.state.city.wallet.get("gold:").copied().unwrap_or(0),
            purchases: BTreeMap::new(),
            discarded: BTreeMap::new(),
        })
    }
    pub(super) fn provision_view(&self) -> Result<Option<Provisions>> {
        let Some(p) = &self.state.preparation else {
            return Ok(None);
        };
        let rules = self.raid_inventory(&p.party)?;
        let mut actions = vec![ProvisionAction::Cancel, ProvisionAction::Embark];
        for (item, n) in &p.store {
            let mut trial = p.bag.clone();
            if *n > 0
                && self
                    .state
                    .city
                    .wallet
                    .get(p.currency(item)?)
                    .copied()
                    .unwrap_or(0)
                    >= p.prices[item]
                && trial.insert(
                    &rules,
                    &Stack {
                        item: item.clone(),
                        amount: 1,
                    },
                )? == 1
            {
                actions.push(ProvisionAction::Buy {
                    item: item.clone(),
                    amount: 1,
                });
            }
        }
        for (slot, s) in p.bag.slots.iter().enumerate() {
            if let Some(s) = s {
                actions.push(ProvisionAction::Discard {
                    slot,
                    amount: s.amount,
                });
            }
        }
        Ok(Some(Provisions {
            currencies019: p.commerce019.as_ref().map(|c| c.currencies.clone()),
            quest: p.quest.id.clone(),
            party: p.party.clone(),
            inventory: p.bag.clone(),
            store: p
                .store
                .iter()
                .map(|(item, n)| (item.clone(), *n, p.prices[item]))
                .collect(),
            paid: p.paid,
            actions,
        }))
    }
    pub fn provision_action(&mut self, action: ProvisionAction) -> Result<()> {
        self.transaction(|s| {
            let mut p = s.state.preparation.take().ok_or("no provision page")?;
            match action {
                ProvisionAction::Cancel => {
                    if let Some(c) = &p.commerce019 {
                        for (currency, paid) in &c.paid {
                            let n = s.state.city.wallet.entry(currency.clone()).or_default();
                            *n = n
                                .checked_add(*paid)
                                .ok_or("019 provision refund overflow")?;
                        }
                    } else {
                        let n = s.state.city.wallet.entry("gold:".into()).or_default();
                        *n = n.checked_add(p.paid).ok_or("provision refund overflow")?;
                    }
                    return Ok(());
                }
                ProvisionAction::Buy { item, amount } => {
                    let currency = p.currency(&item)?.to_string();
                    let remaining = p.store.get_mut(&item).ok_or("item not in source store")?;
                    if amount == 0 || amount > *remaining {
                        return Err("invalid provision quantity".into());
                    }
                    let price = p.prices[&item]
                        .checked_mul(amount)
                        .ok_or("purchase overflow")?;
                    let balance = s.state.city.wallet.get(&currency).copied().unwrap_or(0);
                    if price > balance {
                        return Err("insufficient provision gold".into());
                    }
                    if p.bag.insert(
                        &s.raid_inventory(&p.party)?,
                        &Stack {
                            item: item.clone(),
                            amount,
                        },
                    )? != amount
                    {
                        return Err("provisions do not fit".into());
                    }
                    *remaining -= amount;
                    let n = p.purchases.entry(item).or_default();
                    *n = n.checked_add(amount).ok_or("purchase amount overflow")?;
                    if currency == "gold:" {
                        p.paid = p
                            .paid
                            .checked_add(price)
                            .ok_or("purchase ledger overflow")?;
                    }
                    if let Some(c) = &mut p.commerce019 {
                        let n = c.paid.entry(currency.clone()).or_default();
                        *n = n
                            .checked_add(price)
                            .ok_or("019 provision ledger overflow")?;
                    }
                    s.state.city.wallet.insert(currency, balance - price);
                }
                ProvisionAction::Discard { slot, amount } => {
                    let item = p
                        .bag
                        .slots
                        .get(slot)
                        .and_then(Option::as_ref)
                        .ok_or("empty provision slot")?
                        .item
                        .clone();
                    p.bag.discard(slot, amount)?;
                    let n = p.discarded.entry(item).or_default();
                    *n = n.checked_add(amount).ok_or("discard ledger overflow")?;
                }
                ProvisionAction::Embark => {
                    return s.embark_campaign(p);
                }
            }
            s.state.preparation = Some(p);
            Ok(())
        })
    }
    fn embark_campaign(&mut self, p: Preparation) -> Result<()> {
        self.departure_eligible(&p.quest)?;
        if p.party != self.state.city.party {
            return Err("prepared party changed".into());
        }
        if self.definition.content019.is_some() && p.quest.raw["dungeon"] == "farm" {
            return self.embark_farm019(p);
        }
        let contract = HostQuest {
            id: p.quest.plot.clone().unwrap_or_else(|| p.quest.id.clone()),
            content_id: self.definition.data.content.id()?,
            source: format!("campaign017:{}", p.quest.id),
            raw: p.quest.raw.clone(),
            repeatable: p.quest.plot.is_none() || p.quest.source["is_repeatable"] == true,
            advances_region: p.quest.plot.is_none()
                || p.quest.source["completion_dungeon_xp"] == true,
        };
        let mut town = self.town();
        if p.quest.raw["dungeon"] == "darkestdungeon"
            && let Some(auto) = town
                .state
                .campaign
                .as_ref()
                .and_then(|c| c.automation.as_ref())
                .cloned()
        {
            for id in &p.party {
                if auto.marked.contains(id) {
                    let stress = town.hero(*id)?.state.stress;
                    if stress < auto.rules.minimum_stress {
                        town.town_stress(*id, auto.rules.minimum_stress - stress)?;
                    }
                }
            }
        }
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
        self.state.city = town.state;
        if p.quest.source["estate_inventory_dependency"]["consume_on_attempt"] == true {
            for v in p.quest.source["estate_inventory_dependency"]["items_definition"]["items"]
                .as_object()
                .ok_or("invalid dependency items")?
                .values()
            {
                let stack = stack(v)?;
                let n = self
                    .state
                    .city
                    .wallet
                    .get_mut(&stack.item.key())
                    .ok_or("missing invitation")?;
                *n = n
                    .checked_sub(stack.amount)
                    .ok_or("insufficient invitation")?;
            }
        }
        let mut expedition = self.build_expedition(&p.quest, &deployment.token, p.bag)?;
        if let Some(world) = p
            .quest
            .plot
            .as_ref()
            .and_then(|id| self.state.worlds.get(id))
        {
            expedition.resume_persistent(world)?;
        }
        let cache_key = self.raid_definition_key(&p.quest, &deployment.token)?;
        self.definition
            .raids
            .lock()
            .map_err(|_| "campaign cache poisoned")?
            .insert(cache_key, expedition.definitions());
        if p.quest.source["is_persistent_through_campaign"] == true {
            let plot = p
                .quest
                .plot
                .as_ref()
                .ok_or("persistent quest missing plot identity")?;
            self.state
                .board
                .persistent
                .insert(plot.clone(), p.quest.clone());
        }
        self.state.city.supply_grants.clear();
        self.state.active = Some(Active {
            quest: p.quest,
            snapshot: expedition.checkpoint()?,
        });
        Ok(())
    }
    fn raid_definition_key(&self, q: &Instance, token: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut city = self.state.city.clone();
        // The delivered grant is already frozen in departure.required and is
        // cleared after actual embark. It cannot change the definition key.
        city.supply_grants.clear();
        let bytes = serde_json::to_vec(&(
            token,
            q,
            city,
            &self.state.infestation,
            &self.state.infestation_clock,
            &self.state.flashbacks,
            &self.state.started_plots,
            self.state.clock.week,
        ))
        .map_err(|e| e.to_string())?;
        Ok(format!("raid-definition:{:x}", Sha256::digest(bytes)))
    }
    fn expedition_sources(
        &self,
        q: &Instance,
    ) -> Result<Arc<crate::expedition_sources017::Sources>> {
        let key = format!(
            "{};seed:{};infestation:{}",
            q.id, q.seed, self.state.infestation
        );
        if let Some(sources) = self
            .definition
            .sources
            .lock()
            .map_err(|_| "campaign source cache poisoned")?
            .get(&key)
            .cloned()
        {
            return Ok(sources);
        }
        let game = self.definition.town.game.clone();
        let catalog = dd_data::catalog014::Catalog::load(
            &game.overlay,
            dd_data::maps014::Candidates::for_content(&self.definition.data.content)?,
        )?;
        let mut map = if let Some(plot) = &q.plot {
            match crate::maps014::quest_entry(&catalog, plot, q.seed)? {
                crate::maps014::Entry::Map(m) => *m,
                _ => return Err("wave expedition outside this executor".into()),
            }
        } else if self.definition.automation.is_some()
            && q.raw["dungeon"] == "courtyard"
            && ["farm_bloodcrate", "kill_statue"]
                .iter()
                .any(|k| q.raw["type"] == *k)
        {
            let kind = q.raw["type"].as_str().unwrap();
            let (_, goal) = catalog.repeat_rule(kind)?;
            if q.raw["goal_ids"] != json!([goal]) {
                return Err("repeat task goal/source mismatch".into());
            }
            crate::maps014::repeat_entry(&catalog, kind, q.seed)?
        } else {
            let key = dd_data::generation014::Key {
                dungeon: q.raw["dungeon"].as_str().ok_or("missing dungeon")?.into(),
                quest: q.raw["type"].as_str().ok_or("missing quest type")?.into(),
                size: match dd_data::campaign017::uint(&q.raw, "length")? {
                    1 => "short",
                    2 => "medium",
                    3 => "long",
                    _ => return Err("invalid random quest length".into()),
                }
                .into(),
            };
            let goals: Vec<String> =
                serde_json::from_value(q.raw["goal_ids"].clone()).map_err(|e| e.to_string())?;
            crate::maps014::campaign_random_entry(&catalog, &key, &goals, q.seed)?
        };
        self.infest_map(&mut map, q)?;
        let mut effective = q.raw.clone();
        for key in [
            "is_stall_enabled",
            "is_surprise_enabled",
            "is_scouting_enabled",
        ] {
            effective[key] = q.source.get(key).cloned().unwrap_or(json!(true));
        }
        let sources = Arc::new(if self.definition.automation.is_some() {
            crate::expedition_sources017::Sources::bind_automation018(
                &game, &catalog, map, effective, q.seed,
            )?
        } else {
            crate::expedition_sources017::Sources::bind(&game, &catalog, map, effective, q.seed)?
        });
        self.definition
            .sources
            .lock()
            .map_err(|_| "campaign source cache poisoned")?
            .insert(key, sources.clone());
        Ok(sources)
    }
    pub(super) fn build_expedition(
        &self,
        q: &Instance,
        token: &str,
        bag: Bag,
    ) -> Result<crate::opening015::Session> {
        let game = self.definition.town.game.clone();
        let sources = (*self.expedition_sources(q)?).clone();
        let town = self.town();
        let mut heroes = vec![];
        for guid in &self.state.city.party {
            let h = town.hero(*guid)?;
            let mut base =
                town.blueprint(h, dd_data::campaign017::uint(&q.raw, "difficulty")?, true)?;
            base.mechanics.hero.initial_hp = None;
            base.mechanics.hero.carry = None;
            let mut state = h.state.clone();
            state.difficulty = base.mechanics.hero.difficulty.clone();
            state.build = base.mechanics.build.input.clone();
            state.trinkets = base.mechanics.build.trinkets.clone();
            state.trinket_slots = Some((0..state.trinkets.len()).collect());
            heroes.push(crate::opening015::Hero {
                guid: *guid,
                name: h.name.clone(),
                xp: h.xp,
                base,
                state,
            });
        }
        let mut blueprints = heroes.iter().map(|h| h.base.clone()).collect::<Vec<_>>();
        let inherited = blueprints
            .iter()
            .map(|b| b.mechanics.hero.exploration_buffs.len())
            .collect::<Vec<_>>();
        let mut bonuses = town.bonuses(&self.state.city.party, &mut blueprints)?;
        for (blueprint, count) in blueprints.iter_mut().zip(inherited) {
            let passive = blueprint.mechanics.hero.exploration_buffs.split_off(count);
            for b in &passive {
                if !matches!(
                    b.definition.duration_type.as_deref(),
                    None | Some("quest_end")
                ) {
                    return Err("unsupported passive district/event Buff expiry".into());
                }
            }
            blueprint
                .mechanics
                .hero
                .host_passive_buffs
                .extend(passive.into_iter().map(|b| b.definition));
        }
        if let Some(departure) = &self.state.city.departure {
            bonuses.supplies = departure.required.supplies.clone();
            if bonuses != departure.required {
                return Err("campaign district source obligations changed during sortie".into());
            }
        }
        for (hero, mut blueprint) in heroes.iter_mut().zip(blueprints) {
            if q.raw["dungeon"] == "darkestdungeon"
                && let Some(auto) = town
                    .state
                    .campaign
                    .as_ref()
                    .and_then(|c| c.automation.as_ref())
                && auto.marked.contains(&hero.guid)
            {
                blueprint
                    .mechanics
                    .hero
                    .host_passive_buffs
                    .extend(auto.rules.reentry_buffs.clone());
            }
            hero.base = blueprint;
        }
        let curios = dd_data::curios013::Curios::load_campaign(&game.overlay)?;
        let mut camping = BTreeMap::new();
        for guid in &self.state.city.party {
            let h = town.hero(*guid)?;
            let mut known = BTreeMap::new();
            for id in &h.camping {
                known.insert(
                    id.clone(),
                    dd_data::camp017::skill(
                        &game,
                        &curios,
                        &self.definition.town.data.camps,
                        &h.class,
                        id,
                        q.raw["dungeon"].as_str().ok_or("missing camping region")?,
                        dd_data::campaign017::uint(&q.raw, "difficulty")?,
                    )?,
                );
            }
            camping.insert(
                *guid,
                crate::expedition017::CampDeck {
                    known,
                    selected: h.selected_camping.clone(),
                },
            );
        }
        let context =
            crate::expedition017::Context {
                automation: if self.definition.automation.is_some() {
                    Some(super::automation::RaidRules {
                        owned_trinkets: town.owned_rewards(),
                        contract: if q.raw["dungeon"] == "darkestdungeon" {
                            Some(game.battle_contract(
                                q.plot.as_deref().ok_or("DD plot identity absent")?,
                            )?)
                        } else {
                            None
                        },
                        no_random_ambush: q.raw["dungeon"] == "darkestdungeon",
                        no_torch_burn: !game.raid_torch_burn_enabled(
                            q.raw["torch_setting"].as_str().unwrap_or("default"),
                        )?,
                    })
                } else {
                    None
                },
                token: token.into(),
                bonuses,
                camping,
                camping_slots: self.definition.town.data.candidates.camping_slots,
                completed: self.state.city.completed_ids.clone(),
                infestation: self.state.infestation.clone(),
                activity: self.infestation_activity()?,
                suppressed: self.state.infestation_clock.suppressed.clone(),
                roaming: BTreeSet::new(),
                flashbacks: self.state.flashbacks.clone(),
                exit: self.definition.data.exit.clone(),
                resolve_history: self
                    .state
                    .city
                    .heroes
                    .iter()
                    .map(|h| (h.guid, h.state.resolve_history.clone()))
                    .collect(),
                locked_quirks: self
                    .state
                    .city
                    .heroes
                    .iter()
                    .map(|h| {
                        (
                            h.guid,
                            h.quirks
                                .iter()
                                .filter(|(_, f)| f.locked)
                                .map(|(id, _)| id.clone())
                                .collect(),
                        )
                    })
                    .collect(),
            };
        crate::opening015::Session::from_campaign(
            game,
            self.definition.town.data.base.clone(),
            sources,
            context,
            heroes,
            bag,
            q.seed,
        )
    }
    pub(super) fn expedition(&self) -> Result<crate::opening015::Session> {
        let active = self
            .state
            .active
            .as_ref()
            .ok_or("no active campaign expedition")?;
        let token = &self
            .state
            .city
            .departure
            .as_ref()
            .ok_or("missing campaign departure")?
            .token;
        let cache_key = self.raid_definition_key(&active.quest, token)?;
        let cached = self
            .definition
            .raids
            .lock()
            .map_err(|_| "campaign cache poisoned")?
            .get(&cache_key)
            .cloned();
        let def = if let Some(d) = cached {
            d
        } else {
            let rules = self.raid_inventory(&self.state.city.party)?;
            let def = self
                .build_expedition(&active.quest, token, Bag::new(&rules))?
                .definitions();
            self.definition
                .raids
                .lock()
                .map_err(|_| "campaign cache poisoned")?
                .insert(cache_key, def.clone());
            def
        };
        crate::opening015::Session::restore(&active.snapshot, &def)
    }
    pub fn expedition_action(&mut self, action: crate::opening015::Action) -> Result<()> {
        self.transaction(|s| {
            let mut e = s.expedition()?;
            e.act(action)?;
            s.state.active.as_mut().unwrap().snapshot = e.checkpoint()?;
            if e.observe()?.phase == crate::opening015::Phase::Finished {
                let receipt = e.campaign_return()?;
                s.accept_raid(receipt)?;
            }
            Ok(())
        })
    }
    pub(super) fn expedition_view(&self) -> Result<Option<crate::opening015::Observation>> {
        if self.state.active.is_some() {
            Ok(Some(self.expedition()?.observe()?))
        } else {
            Ok(None)
        }
    }
    pub(super) fn validate_raid(&self) -> Result<()> {
        for (plot, world) in &self.state.worlds {
            let quest = self
                .state
                .board
                .persistent
                .get(plot)
                .ok_or("persistent world without stable quest instance")?;
            if self.state.city.completed_ids.contains(plot) {
                return Err("completed quest retained as active world".into());
            }
            world.validate(
                self.expedition_sources(quest)?.as_ref(),
                &self.definition.data.content,
            )?;
        }
        if self.state.preparation.is_some()
            && (self.state.active.is_some() || self.state.city.departure.is_some())
        {
            return Err("preparation overlaps departure".into());
        }
        if (self.state.active.is_some() || self.farm019_active())
            != self.state.city.departure.is_some()
        {
            return Err("unowned campaign departure".into());
        }
        if let Some(p) = &self.state.preparation {
            p.bag.validate(&self.raid_inventory(&p.party)?)?;
            let template = self.provision_template(p.quest.clone())?;
            if p.commerce019.is_some() != self.definition.content019.is_some() {
                return Err("019 provision ledger profile mismatch".into());
            }
            if let Some(c) = &p.commerce019 {
                if Some(&c.currencies) != template.commerce019.as_ref().map(|c| &c.currencies) {
                    return Err("019 provision currency source mismatch".into());
                }
                let mut expected_paid = BTreeMap::<String, u32>::new();
                for (item, amount) in &p.purchases {
                    let n = expected_paid.entry(p.currency(item)?.into()).or_default();
                    *n = n
                        .checked_add(
                            p.prices
                                .get(item)
                                .copied()
                                .ok_or("019 unknown provision price")?
                                .checked_mul(*amount)
                                .ok_or("019 provision price overflow")?,
                        )
                        .ok_or("019 provision payment overflow")?;
                }
                if expected_paid != c.paid {
                    return Err("019 provision payment differs from quantities".into());
                }
                let mut wallet = c.opening.clone();
                for (currency, paid) in &c.paid {
                    let n = wallet.entry(currency.clone()).or_default();
                    *n = n
                        .checked_sub(*paid)
                        .ok_or("019 provision negative balance")?;
                }
                if wallet != self.state.city.wallet {
                    return Err("019 provision currency conservation failed".into());
                }
            }
            if p.prices != template.prices || p.store.keys().ne(template.store.keys()) {
                return Err("saved provision source prices or catalog differ".into());
            }
            let mut paid = 0u32;
            let mut items = BTreeMap::<Item, u32>::new();
            for stack in template.bag.slots.iter().flatten() {
                let n = items.entry(stack.item.clone()).or_default();
                *n = n.checked_add(stack.amount).ok_or("grant total overflow")?;
            }
            for (item, n) in &p.purchases {
                let original = template
                    .store
                    .get(item)
                    .ok_or("purchase outside source store")?;
                if *n == 0 || *n > *original {
                    return Err("saved provision quantity exceeds source".into());
                }
                if p.currency(item)? == "gold:" {
                    paid = paid
                        .checked_add(
                            template.prices[item]
                                .checked_mul(*n)
                                .ok_or("saved cost overflow")?,
                        )
                        .ok_or("saved cost overflow")?;
                }
                let total = items.entry(item.clone()).or_default();
                *total = total.checked_add(*n).ok_or("saved quantity overflow")?;
            }
            for (item, n) in &p.discarded {
                let total = items.get_mut(item).ok_or("discard lacks provenance")?;
                *total = total
                    .checked_sub(*n)
                    .ok_or("discard exceeds acquired provisions")?;
            }
            for (item, n) in &items {
                if p.bag.count(item) != *n {
                    return Err("provision bag differs from acquisition/discard ledger".into());
                }
            }
            if p.bag
                .slots
                .iter()
                .flatten()
                .any(|s| !items.contains_key(&s.item))
            {
                return Err("unacquired provision item".into());
            }
            for (item, n) in &template.store {
                if p.store[item] != n - p.purchases.get(item).copied().unwrap_or(0) {
                    return Err("provision stock differs from purchase ledger".into());
                }
            }
            if paid != p.paid
                || p.opening_gold.checked_sub(paid)
                    != Some(self.state.city.wallet.get("gold:").copied().unwrap_or(0))
            {
                return Err("provision wallet conservation failed".into());
            }

            if p.party != self.state.city.party || !self.state.board.entries.contains(&p.quest) {
                return Err("foreign provision quest or party".into());
            }
        }
        if self.state.active.is_some() {
            self.expedition()?.checkpoint()?;
        }
        Ok(())
    }
    pub(super) fn accept_raid(&mut self, receipt: crate::expedition017::Return) -> Result<()> {
        use crate::events017::ReturnKind;
        let active = self
            .state
            .active
            .take()
            .ok_or("missing active quest receipt")?;
        let departure = self
            .state
            .city
            .departure
            .as_ref()
            .ok_or("missing bound departure")?
            .clone();
        let old_growth020 = if self.definition.goals020.is_some() {
            departure
                .party
                .iter()
                .map(|id| {
                    let h = self.town().hero(*id)?.clone();
                    Ok((*id, h.class, h.xp, h.quirks.contains_key("shard_hungry")))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![]
        };
        let prior_kills020 = if self.definition.goals020.is_some() {
            active
                .quest
                .plot
                .as_ref()
                .and_then(|id| self.state.worlds.get(id))
                .map(|w| w.kill_history020().to_vec())
                .unwrap_or_default()
        } else {
            vec![]
        };
        if self.definition.goals020.is_some() && !receipt.kills.starts_with(&prior_kills020) {
            return Err("020 persistent kill history prefix differs".into());
        }
        if receipt.token != departure.token
            || receipt
                .heroes
                .iter()
                .map(|(id, _)| *id)
                .collect::<BTreeSet<_>>()
                != departure.party.iter().copied().collect()
        {
            return Err("foreign campaign terminal receipt".into());
        }
        let complete = receipt.kind == ReturnKind::Complete;
        if let Some(plot) = &active.quest.plot {
            if complete {
                self.state.worlds.remove(plot);
                self.state.board.persistent.remove(plot);
            } else if let Some(world) = &receipt.persistent {
                self.state.worlds.insert(plot.clone(), world.clone());
            }
        }
        let advance = if self.definition.content019.is_some()
            && active.quest.plot.as_deref() == Some("plot_color_of_madness_2")
        {
            false
        } else {
            complete || self.raid_advances_week(&active.quest, &receipt)?
        };
        let living_before = self
            .state
            .city
            .heroes
            .iter()
            .filter(|h| h.state.alive)
            .map(|h| h.guid)
            .collect::<BTreeSet<_>>();
        let result_type = active.quest.source["town_event_result_id_on_complete"]
            .as_str()
            .unwrap_or("");
        if !result_type.is_empty() && result_type != "remove_vampire" {
            return Err("unknown quest completion effect type".into());
        }
        let story_cure = if complete {
            let plot = active.quest.plot.as_deref().unwrap_or("");
            self.definition
                .data
                .events
                .values()
                .filter(|e| e.is_object())
                .any(|e| {
                    e["requirements"]["finished_plot_quests"]
                        .as_array()
                        .is_some_and(|ids| ids.iter().any(|v| v == plot))
                        && e["data"].as_array().is_some_and(|effects| {
                            effects.iter().any(|v| v["type"] == "remove_vampire")
                        })
                })
        } else {
            false
        };
        let cure = complete && (result_type == "remove_vampire" || story_cure);
        let mut results = vec![];
        for guid in &departure.party {
            let mut h = receipt
                .heroes
                .iter()
                .find(|(id, _)| id == guid)
                .ok_or("missing returned identity")?
                .1
                .clone();
            h.id = results.len();
            h.host_buff_instances.clear();
            if cure && h.alive {
                self.cure_crimson(&mut h)?;
            }
            results.push(h);
        }
        let departed_assets = self
            .state
            .city
            .assets
            .iter()
            .filter(|a| departure.assets.contains(&a.instance))
            .cloned()
            .collect::<Vec<_>>();
        let mut recovered = BTreeSet::new();
        let mut acquired = vec![];
        for item in &receipt.recovered {
            self.definition
                .town
                .game
                .validate_item_growth(&item.state)?;
            if let Some((guid, native_slot)) = item.origin {
                let mut owned = departed_assets
                    .iter()
                    .filter(|a| a.owner.is_some_and(|(id, _)| id == guid))
                    .collect::<Vec<_>>();
                owned.sort_by_key(|a| a.owner.unwrap().1);
                let original = owned
                    .get(native_slot)
                    .ok_or("foreign recovered equipment slot")?;
                let h = receipt
                    .heroes
                    .iter()
                    .find(|(id, _)| *id == guid)
                    .ok_or("foreign recovered hero")?;
                if h.1.alive
                    || !recovered.insert(original.instance)
                    || h.1
                        .build
                        .physical_trinkets
                        .as_ref()
                        .and_then(|v| v.get(native_slot))
                        != Some(&item.state)
                {
                    return Err("duplicate/foreign recovered physical equipment".into());
                }
            } else {
                acquired.push(item.state.clone());
            }
        }
        let mut town = self.town();
        if cure {
            for h in town.state.heroes.iter_mut().filter(|h| h.state.alive) {
                self.cure_crimson(&mut h.state)?;
            }
        }
        town.state.campaign.as_mut().unwrap().week = Some(
            self.state
                .clock
                .week
                .checked_add(u32::from(advance))
                .ok_or("week overflow")?,
        );
        self.close_event_plots(&mut town, active.quest.plot.as_deref(), complete, advance)?;
        if complete {
            self.recover_quest_assets(&mut town, &active.quest)?;
        }
        let quirk_key = if complete {
            "party_quirks_to_apply_on_completion"
        } else {
            "party_quirks_to_apply_on_failure"
        };
        let plot_quirks = match active.quest.source.get(quirk_key) {
            None => &[][..],
            Some(v) => v
                .as_array()
                .ok_or("invalid source quest quirk distribution")?
                .as_slice(),
        };
        town.settle_sortie(
            &receipt.token,
            results,
            receipt.inventory,
            complete,
            recovered,
            Some(CampaignSettlement {
                advance,
                regroup: receipt.kind == ReturnKind::Regroup,
                physical: &acquired,
                quirks: plot_quirks,
                raid_quirks: &receipt.raid_quirks,
            }),
        )?;
        self.automation_after_return(&mut town, &active.quest, complete, &departure.party)?;
        if std::mem::take(
            &mut town
                .state
                .campaign
                .as_mut()
                .unwrap()
                .quest_event_opportunity,
        ) {
            self.state.events.guarantee_opportunity();
        }
        // Unrecovered equipment retains its actual final state, not a freshly
        // minted copy of the initial definition. Destroyed items cannot return.
        for original in &departed_assets {
            if town
                .state
                .campaign
                .as_ref()
                .unwrap()
                .weekly_lost_assets
                .iter()
                .any(|a| a.instance == original.instance)
                || town
                    .state
                    .assets
                    .iter()
                    .any(|a| a.instance == original.instance)
            {
                continue;
            }
            let (guid, _) = original.owner.ok_or("departed equipment lacks owner")?;
            let mut owned = departed_assets
                .iter()
                .filter(|a| a.owner.is_some_and(|(id, _)| id == guid))
                .collect::<Vec<_>>();
            owned.sort_by_key(|a| a.owner.unwrap().1);
            let slot = owned
                .iter()
                .position(|a| a.instance == original.instance)
                .ok_or("lost physical slot missing")?;
            let hero = &receipt
                .heroes
                .iter()
                .find(|(id, _)| *id == guid)
                .ok_or("lost equipment hero missing")?
                .1;
            let state = hero
                .build
                .physical_trinkets
                .as_ref()
                .and_then(|v| v.get(slot))
                .ok_or("lost native physical slot missing")?;
            if !state.destroyed {
                self.record_lost_asset(Asset {
                    instance: original.instance,
                    state: state.clone(),
                    owner: None,
                })?;
            }
        }
        let week = town.effective_week();
        for lost in std::mem::take(&mut town.state.campaign.as_mut().unwrap().weekly_lost_assets) {
            self.record_asset_loss_at(lost, week)?;
        }
        let dead = town
            .state
            .heroes
            .iter()
            .filter(|h| !h.state.alive && living_before.contains(&h.guid))
            .map(|h| h.guid)
            .collect::<BTreeSet<_>>();
        self.state
            .clock
            .commit_return(&receipt.token, receipt.kind, &dead, advance)?;
        for id in &dead {
            self.state.graveyard.insert(*id, town.hero(*id)?.clone());
        }
        self.return_protected_assets(&mut town)?;
        if complete {
            let key = format!(
                "{}:{}",
                active.quest.raw["dungeon"]
                    .as_str()
                    .ok_or("missing quest guarantee region")?,
                active.quest.raw["type"]
                    .as_str()
                    .ok_or("missing quest guarantee type")?
            );
            if self.definition.data.guarantees.contains_key(&key) {
                let guarantee = library::unique_json(
                    &self.definition.data.guarantees,
                    &key,
                    "quest guaranteed event",
                )?;
                self.state.events.queue(
                    &self.definition.data,
                    guarantee["event_id"]
                        .as_str()
                        .ok_or("missing guaranteed event ID")?,
                    &format!("{};quest-complete", receipt.token),
                )?;
            }
            for id in &active.quest.extra_rewards {
                town.bank_item(&Stack {
                    item: Item::new("trinket", id),
                    amount: 1,
                })?;
            }
        }
        for reward in &receipt.earned_rewards {
            town.bank_item(reward)?;
        }
        for id in &receipt.flashbacks {
            if !self.state.flashbacks.insert(id.clone()) {
                return Err("duplicate campaign nightmare receipt".into());
            }
        }
        self.rescued_prisoners(&mut town, &active.quest, &receipt.prisoners)?;
        self.goals020_return(
            &town,
            &active.quest,
            complete,
            &old_growth020,
            &receipt.goal_kills,
            &receipt.kills[prior_kills020.len()..],
        )?;
        self.state.city = town.state;
        if advance {
            // Retire the previous weekly board before a new event may pause for
            // payment/selection. Persistent worlds alone survive that boundary.
            let persistent = std::mem::take(&mut self.state.board.persistent);
            self.state.board = Board {
                entries: persistent.values().cloned().collect(),
                persistent,
                ..Default::default()
            };
        }
        if receipt.ends_infestation {
            for (source, last) in &receipt.kills {
                for id in [source, last] {
                    if self
                        .definition
                        .town
                        .game
                        .monster(id)?
                        .mechanics
                        .boss
                        .end_infestation
                    {
                        self.suppress_infestation_family(id)?;
                    }
                }
            }
        }
        if advance {
            self.advance_infestation(true)?;
        }
        self.state.log.push(json!({"raid":active.quest.id,"kind":receipt.kind,"week":self.state.clock.week,"returns":self.state.clock.returns,"advance":advance,"battles":receipt.battles,"rooms":receipt.rooms,"kills":receipt.kills,"ends_infestation":receipt.ends_infestation,"prisoners":receipt.prisoners,"raid_quirks":receipt.raid_quirks,"map_checkpoint_bytes":receipt.map.len()}));
        if advance && self.failure()?.is_none() {
            self.process_town_events()?;
        }
        self.refresh_content019()?;
        Ok(())
    }
    fn rescued_prisoners(
        &self,
        town: &mut TownSession,
        q: &Instance,
        nodes: &BTreeSet<String>,
    ) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }
        let sources = self.expedition_sources(q)?;
        let rows = self.definition.data.prisoner["hero_spawn_difficulty_table"]
            .as_array()
            .ok_or("missing original prisoner spawning")?;
        let matched = rows
            .iter()
            .filter(|r| r["key"] == q.raw["difficulty"])
            .collect::<Vec<_>>();
        if matched.len() != 1 {
            return Err("missing/ambiguous prisoner difficulty table".into());
        }
        let options = matched[0]["data"]
            .as_array()
            .ok_or("missing prisoner generation options")?;
        let weights = options
            .iter()
            .map(|v| {
                v["chance"]
                    .as_f64()
                    .ok_or_else(|| "missing prisoner source chance".to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        for node in nodes {
            if !matches!(
                sources.special.get(node),
                Some(crate::expedition_sources017::Special::Prisoner(_))
            ) {
                return Err("rescued prisoner lacks physical source".into());
            }
            let option = &options[town.choose(&weights)?]["value"];
            dd_data::campaign017::fields(
                option,
                &[
                    "level",
                    "hero_type_tags",
                    "starting_positive_quirks",
                    "starting_negative_quirks",
                    "starting_camping_skills",
                    "starting_shared_camping_skills",
                    "starting_combat_skills",
                ],
                "prisoner generation",
            )?;
            let tags = option["hero_type_tags"]
                .as_array()
                .ok_or("missing prisoner class tags")?;
            let mut classes = vec![];
            for class in self.definition.town.game.hero_ids() {
                let b = self.definition.town.game.hero(
                    class,
                    0,
                    0,
                    &[&self.definition.town.game.hero_skill_ids(class)?[0]],
                )?;
                if tags.is_empty()
                    || tags.iter().any(|t| {
                        t.as_str()
                            .is_some_and(|s| b.mechanics.hero.tags.iter().any(|v| v == s))
                    })
                {
                    classes.push(class.clone());
                }
            }
            if classes.is_empty() {
                return Err("no enabled prisoner class matches original tags".into());
            }
            let i = town.state.rng.below(classes.len() as u64) as usize;
            let hero = town.generate_prisoner(&classes[i], option)?;
            town.state
                .log
                .push(json!({"prisoner":node,"quest":q.id,"hero":hero.guid}));
            town.state.recruits.push(hero);
        }
        Ok(())
    }
    fn raid_advances_week(&self, q: &Instance, r: &crate::expedition017::Return) -> Result<bool> {
        if self.definition.automation.is_some() && q.raw["dungeon"] == "darkestdungeon" {
            return Ok(true);
        }

        if r.kind == crate::events017::ReturnKind::Regroup
            && q.source["is_persistent_through_campaign"] == true
        {
            return Ok(true);
        }
        if q.source["is_town_progression_goals_enabled"] == false {
            return Ok(true);
        }
        for id in &self.definition.data.town_goal_ids {
            let g = library::unique_json(&self.definition.data.goals, id, "town progression goal")?;
            let amount = g["data"]["amount"]
                .as_u64()
                .ok_or("missing town goal amount")?;
            let met = match g["type"].as_str().ok_or("missing town goal kind")? {
                "battle" => u64::from(r.battles) >= amount,
                "explore_room" => u64::from(r.rooms) >= amount,
                "deaths_door" => u64::from(r.deaths_door) >= amount,
                "trait_applied" => u64::from(r.afflictions) >= amount,
                "survival"
                    if self.definition.content019.is_some() && q.raw["dungeon"] == "farm" =>
                {
                    u64::from(r.survival) >= amount
                }
                // Legacy owners keep their historical execution boundary.
                "survival" => false,
                other => return Err(format!("unknown town progression goal {other}")),
            };
            if met {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
