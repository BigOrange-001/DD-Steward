//! Public heirloom balances and exact, non-recycling funding plans.
use super::*;
use crate::{
    campaign018::runner::View,
    town016::{Observation as Town, VisibleExchange},
};
fn material(s: &str) -> bool {
    s.starts_with("heirloom:")
}
fn wallet(v: &View) -> &BTreeMap<String, u32> {
    match v {
        View::Opening(o) => &o.wallet,
        View::Campaign(o) => &o.town.wallet,
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Account {
    pub initialized: bool,
    pub initial: BTreeMap<String, u64>,
    pub income: BTreeMap<String, u64>,
    pub building_spend: BTreeMap<String, u64>,
    pub other_spend: BTreeMap<String, u64>,
    pub exchange_spent: BTreeMap<String, u64>,
    pub exchange_received: BTreeMap<String, u64>,
    pub converted_unspent: BTreeMap<String, u64>,
    pub last_balance: BTreeMap<String, u64>,
    pub exchanges: u64,
    pub building_upgrades: u64,
    pub recycled_units: u64,
    pub crest_equivalent_loss: f64,
    pub unpriced_exchanges: u64,
}
fn add(m: &mut BTreeMap<String, u64>, key: &str, n: u64) {
    if n > 0 {
        *m.entry(key.into()).or_default() += n;
    }
}
fn rate(id: &str, xs: &[VisibleExchange]) -> Option<f64> {
    if id == "heirloom:crest" {
        return Some(1.);
    }
    xs.iter()
        .filter(|x| x.from == id && x.to == "heirloom:crest" && x.spend > 0)
        .map(|x| f64::from(x.gain) / f64::from(x.spend))
        .max_by(f64::total_cmp)
}
impl Account {
    pub fn record(&mut self, before: &View, after: &View, log: &Value) -> Result<()> {
        let a = wallet(before);
        let b = wallet(after);
        if !self.initialized {
            self.initial = a
                .iter()
                .filter(|(k, _)| material(k))
                .map(|(k, n)| (k.clone(), u64::from(*n)))
                .collect();
            self.initialized = true;
        }
        let mut spent = BTreeMap::<String, u64>::new();
        let mut received = BTreeMap::<String, u64>::new();
        let mut building = BTreeMap::<String, u64>::new();
        if let View::Campaign(o) = before {
            if let Some(v) = log["action"]["Town"].get("Exchange") {
                let index = v["index"]
                    .as_u64()
                    .ok_or("025 heirloom exchange index absent")?
                    as usize;
                let batches = v["batches"].as_u64().ok_or("025 heirloom batches absent")?;
                let x = o
                    .town
                    .exchanges
                    .iter()
                    .find(|x| x.index == index)
                    .ok_or("025 committed exchange lacks public rate")?;
                if material(&x.from) && material(&x.to) {
                    let debit = u64::from(x.spend) * batches;
                    let credit = u64::from(x.gain) * batches;
                    add(&mut spent, &x.from, debit);
                    add(&mut received, &x.to, credit);
                    self.exchanges += 1;
                    let native = u64::from(a.get(&x.from).copied().unwrap_or(0))
                        .saturating_sub(self.converted_unspent.get(&x.from).copied().unwrap_or(0));
                    self.recycled_units += debit.saturating_sub(native);
                    match (
                        rate(&x.from, &o.town.exchanges),
                        rate(&x.to, &o.town.exchanges),
                    ) {
                        (Some(f), Some(t)) => {
                            self.crest_equivalent_loss +=
                                (debit as f64 * f - credit as f64 * t).max(0.)
                        }
                        _ => self.unpriced_exchanges += 1,
                    }
                }
            }
            if log["action"]["Town"]
                .get("Upgrade")
                .is_some_and(|v| v["hero"].is_null())
            {
                let action = &log["action"]["Town"];
                let q = o
                    .town
                    .actions
                    .iter()
                    .chain(&o.town.upgrade_quotes)
                    .find(|q| serde_json::to_value(&q.action).is_ok_and(|v| &v == action))
                    .ok_or("025 building spend lacks public quote")?;
                building = q
                    .cost
                    .iter()
                    .filter(|(k, _)| material(k))
                    .map(|(k, n)| (k.clone(), u64::from(*n)))
                    .collect();
                self.building_upgrades += 1;
            }
        }
        let keys = a
            .keys()
            .chain(b.keys())
            .chain(spent.keys())
            .chain(received.keys())
            .chain(building.keys())
            .filter(|k| material(k))
            .cloned()
            .collect::<BTreeSet<_>>();
        for k in keys {
            let old = u64::from(a.get(&k).copied().unwrap_or(0));
            let now = u64::from(b.get(&k).copied().unwrap_or(0));
            let pay = spent.get(&k).copied().unwrap_or(0);
            let gain = received.get(&k).copied().unwrap_or(0);
            let build = building.get(&k).copied().unwrap_or(0);
            let expected = i128::from(old) + i128::from(gain) - i128::from(pay) - i128::from(build);
            let delta = i128::from(now) - expected;
            let other = (-delta).max(0) as u64;
            add(&mut self.income, &k, delta.max(0) as u64);
            add(&mut self.other_spend, &k, other);
            add(&mut self.building_spend, &k, build);
            add(&mut self.exchange_spent, &k, pay);
            add(&mut self.exchange_received, &k, gain);
            let converted = self.converted_unspent.get(&k).copied().unwrap_or(0);
            // Buildings/other expenses consume converted stock first. Exchanges use
            // native stock first; a legacy recycling action is still audited honestly.
            let after_expenses = converted.saturating_sub(build + other);
            let recycled = pay.saturating_sub(old.saturating_sub(after_expenses));
            let remaining = after_expenses
                .saturating_sub(recycled)
                .saturating_add(gain)
                .min(now);
            self.converted_unspent.insert(k.clone(), remaining);
            self.last_balance.insert(k, now);
        }
        Ok(())
    }
    /// Every returned plan funds the complete quote. Deficit currencies never
    /// become donors, so neither direct nor triangular recycling is possible.
    pub fn funding_plan(
        &self,
        o: &Town,
        cost: &BTreeMap<String, u32>,
    ) -> Option<Vec<(usize, u64)>> {
        let mut w = o
            .wallet
            .iter()
            .map(|(k, n)| (k.clone(), u64::from(*n)))
            .collect::<BTreeMap<_, _>>();
        let mut native = w
            .iter()
            .filter(|(k, _)| material(k))
            .map(|(k, n)| {
                (
                    k.clone(),
                    n.saturating_sub(
                        u64::from(cost.get(k).copied().unwrap_or(0))
                            .max(self.converted_unspent.get(k).copied().unwrap_or(0)),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let keys = w
            .keys()
            .chain(cost.keys())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut failed = BTreeSet::new();
        fn search(
            o: &Town,
            cost: &BTreeMap<String, u32>,
            w: &mut BTreeMap<String, u64>,
            native: &mut BTreeMap<String, u64>,
            keys: &[String],
            failed: &mut BTreeSet<Vec<u64>>,
        ) -> Option<Vec<(usize, u64)>> {
            let Some((currency, need)) = cost.iter().find_map(|(k, n)| {
                let have = w.get(k).copied().unwrap_or(0);
                (have < u64::from(*n)).then(|| (k.clone(), u64::from(*n) - have))
            }) else {
                return Some(vec![]);
            };
            if !material(&currency) {
                return None;
            }
            let state = keys
                .iter()
                .flat_map(|k| {
                    [
                        w.get(k).copied().unwrap_or(0),
                        native.get(k).copied().unwrap_or(0),
                    ]
                })
                .collect::<Vec<_>>();
            if failed.contains(&state) {
                return None;
            }
            for x in o.exchanges.iter().filter(|x| {
                x.to == currency && material(&x.from) && x.from != x.to && x.spend > 0 && x.gain > 0
            }) {
                if !o.actions.iter().any(|c| {
                    c.action
                        == TownAction::Exchange {
                            index: x.index,
                            batches: 1,
                        }
                }) {
                    continue;
                }
                let available = native.get(&x.from).copied().unwrap_or(0);
                let maximum =
                    (available / u64::from(x.spend)).min(need.div_ceil(u64::from(x.gain)));
                for batches in (1..=maximum).rev() {
                    let debit = batches * u64::from(x.spend);
                    let credit = batches * u64::from(x.gain);
                    let from = w.get(&x.from).copied().unwrap_or(0);
                    let to = w.get(&x.to).copied().unwrap_or(0);
                    if to + credit > u64::from(u32::MAX) {
                        continue;
                    }
                    w.insert(x.from.clone(), from - debit);
                    w.insert(x.to.clone(), to + credit);
                    native.insert(x.from.clone(), available - debit);
                    let found = search(o, cost, w, native, keys, failed);
                    w.insert(x.from.clone(), from);
                    w.insert(x.to.clone(), to);
                    native.insert(x.from.clone(), available);
                    if let Some(mut tail) = found {
                        tail.insert(0, (x.index, batches));
                        return Some(tail);
                    }
                }
            }
            failed.insert(state);
            None
        }
        search(o, cost, &mut w, &mut native, &keys, &mut failed)
    }
}
