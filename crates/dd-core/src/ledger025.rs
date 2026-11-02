//! One public-price budget; baseline mode audits without changing old decisions.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub early_through_week: u32,
    pub early_reserve: u32,
    pub later_reserve: u32,
    pub pending_supply_reserve: u32,
    pub poverty_gold: u32,
    pub sources: Vec<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Supplies,
    Training,
    Recovery,
    Quirks,
    Lock,
    Buildings,
    Recruitment,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default)]
    pub heirlooms: super::heirlooms::Account,
    pub gold: u32,
    pub supplies: u32,
    pub reserve: u32,
    #[serde(default)]
    pub recovery_reserve: u32,
    pub spent: BTreeMap<String, u64>,
    pub by_hero: BTreeMap<u64, u64>,
    pub receipts: Vec<Value>,
    pub sold: Vec<Value>,
}
impl Ledger {
    pub fn refresh(
        &mut self,
        o: &Observation,
        p: &Profile,
        quote: Option<&crate::preparation024::economy::Quote>,
    ) {
        self.gold = o.town.wallet.get("gold:").copied().unwrap_or(0);
        self.reserve = if o.week <= p.ledger.early_through_week {
            p.ledger.early_reserve
        } else {
            p.ledger.later_reserve
        };
        self.recovery_reserve = self.reserve;
        self.supplies = quote
            .map(|q| {
                let task = o
                    .board
                    .as_ref()
                    .and_then(|b| b.quests.iter().find(|t| t.id == q.quest));
                q.items
                    .iter()
                    .map(|(id, (stock, price, gifts))| {
                        let spec = &p.parameters["legacy_supply_targets"][id];
                        let length = task.map_or(1, |q| q.length).to_string();
                        let mut count = spec
                            .get(&length)
                            .or_else(|| spec.get("default"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32;
                        if let Some(n) = task.and_then(|q| {
                            p.parameters["legacy_supply_overrides"][&q.dungeon][id].as_u64()
                        }) {
                            count = n as u32;
                        }
                        stock
                            .min(&count.saturating_sub(*gifts))
                            .saturating_mul(*price)
                    })
                    .fold(0u32, u32::saturating_add)
            })
            .unwrap_or(p.ledger.pending_supply_reserve);
    }
    pub fn refresh_planned(
        &mut self,
        o: &Observation,
        p: &Profile,
        quote: Option<&crate::preparation024::economy::Quote>,
        exploration: bool,
    ) {
        self.refresh(o, p, quote);
        if exploration
            && let Some(q) = quote
            && let Some(task) = o
                .board
                .as_ref()
                .and_then(|b| b.quests.iter().find(|t| t.id == q.quest))
            && let Some(table) = p.parameters["supplies"]["tables"][&task.dungeon]
                [task.length.to_string()]
            .as_object()
        {
            self.supplies = q
                .items
                .iter()
                .map(|(id, (stock, price, gifts))| {
                    let desired = table
                        .get(id)
                        .and_then(|v| v.get(1))
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    stock
                        .min(&desired.saturating_sub(*gifts))
                        .saturating_mul(*price)
                })
                .fold(0, u32::saturating_add);
        }
    }
    pub fn permits(&self, purpose: Purpose, cost: u32, selected: bool) -> bool {
        if cost == 0 {
            return true;
        }
        if matches!(purpose, Purpose::Supplies) {
            return cost <= self.gold;
        }
        if matches!(
            purpose,
            Purpose::Training | Purpose::Recovery | Purpose::Quirks | Purpose::Lock
        ) && !selected
        {
            return false;
        }
        let reserve = if purpose == Purpose::Recovery {
            self.recovery_reserve
        } else {
            self.reserve
        };
        cost <= self
            .gold
            .saturating_sub(self.supplies.saturating_add(reserve))
    }
    pub fn record(&mut self, p: &Proposal, week: u32) {
        if p.cost == 0 {
            return;
        }
        let category = serde_json::to_value(p.purpose)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        *self.spent.entry(category).or_default() += u64::from(p.cost);
        let hero = match &p.action {
            Action::Town(TownAction::Upgrade { hero, .. }) => *hero,
            Action::Town(TownAction::LearnCamp { hero, .. }) => Some(*hero),
            _ => None,
        };
        if let Some(id) = hero {
            *self.by_hero.entry(id).or_default() += u64::from(p.cost);
        }
        self.receipts
            .push(json!({"week":week,"purpose":p.purpose,"gold":p.cost,"hero":hero,"rule":p.rule}));
    }
}
