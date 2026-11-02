//! Result-only chain measurements from public views; never a hidden policy input.
use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub weeks: BTreeMap<u32, Value>,
    pub trips: Vec<Value>,
    pub active: Option<Value>,
    pub dismissed: Vec<Value>,
    pub idle_weeks: BTreeMap<u64, u32>,
    pub last_returns: Option<u32>,
    pub counts: BTreeMap<String, u64>,
    pub last_roster: BTreeMap<u64, Value>,
    pub supply_shortfalls: BTreeSet<u32>,
    pub windows: BTreeMap<String, (u32, Vec<u64>)>,
    pub window_gaps: BTreeMap<String, u32>,
}
impl Metrics {
    pub fn needs_observe(&self, o: &Observation, log: Option<&Value>) -> bool {
        self.last_returns != Some(o.returns)
            || log.is_some_and(|v| {
                v["action"]["Provision"] == "Embark"
                    || v["action"]["Raid"]["Leave"]["complete"] == false
                    || v["action"]["Town"].get("Dismiss").is_some()
            })
            || !self.supply_shortfalls.contains(&o.returns)
                && o.expedition.as_ref().is_some_and(|r| {
                    !r.quest_ready
                        && r.inventory
                            .count(&dd_data::inventory015::Item::new("provision", ""))
                            == 0
                })
                && self.active.is_some()
    }
    pub fn observe(&mut self, o: &Observation, log: Option<&Value>, ledger: &ledger::Ledger) {
        let gold = o.town.wallet.get("gold:").copied().unwrap_or(0);
        if self.active.is_some()
            && o.expedition.as_ref().is_some_and(|r| {
                !r.quest_ready
                    && r.inventory
                        .count(&dd_data::inventory015::Item::new("provision", ""))
                        == 0
            })
            && self.supply_shortfalls.insert(o.returns)
        {
            *self
                .counts
                .entry("food_exhausted_trips".into())
                .or_default() += 1;
        }
        if self.last_returns != Some(o.returns) {
            let heroes = o
                .town
                .heroes
                .iter()
                .filter(|h| h.alive && !h.quirks.contains_key("shard_hungry"))
                .collect::<Vec<_>>();
            let mut returned_party = BTreeSet::new();
            if let Some(mut trip) = self.active.take() {
                returned_party = trip["party"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_u64)
                    .collect();
                let start_gold = trip["gold"].as_i64().unwrap_or(0);
                let supply = trip["supply_gold"].as_i64().unwrap_or(0);
                let deaths = o.total_deaths as i64 - trip["deaths"].as_i64().unwrap_or(0);
                let complete = o.completions as i64 > trip["completions"].as_i64().unwrap_or(0);
                trip["return_week"] = json!(o.week);
                trip["return_gold"] = json!(gold);
                trip["return_deaths"] = json!(o.total_deaths);
                trip["return_completions"] = json!(o.completions);
                trip["net_income"] = json!(i64::from(gold) - start_gold - supply);
                trip["deaths_delta"] = json!(deaths);
                trip["complete"] = json!(complete);
                trip["abandoned"] = json!(!complete);
                self.trips.push(trip);
            }
            if self.last_returns.is_some() {
                for h in heroes
                    .iter()
                    .filter(|h| h.level >= 4 && !returned_party.contains(&h.guid))
                {
                    *self.idle_weeks.entry(h.guid).or_default() += 1;
                }
                if gold == 0 {
                    *self
                        .counts
                        .entry("bankrupt_weeks_zero_gold".into())
                        .or_default() += 1;
                }
            }
            for (plot, (max, previous)) in &self.windows {
                if !previous.is_empty()
                    && !o
                        .automation
                        .as_ref()
                        .is_some_and(|a| a.completed.contains(plot))
                    && !heroes.iter().any(|h| h.level <= *max)
                {
                    self.window_gaps.entry(plot.clone()).or_insert(o.week);
                }
            }
            if let Some(board) = &o.board {
                for q in board.quests.iter().filter(|q| {
                    q.plot
                        .as_deref()
                        .is_some_and(super::caretaker::is_goal_plot)
                }) {
                    if let (Some(plot), Some(max)) = (&q.plot, q.maximum_resolve_level) {
                        self.windows.insert(
                            plot.clone(),
                            (
                                max,
                                heroes
                                    .iter()
                                    .filter(|h| h.level <= max)
                                    .map(|h| h.guid)
                                    .collect(),
                            ),
                        );
                    }
                }
            }
            let mut alive = 0u64;
            let mut dead = 0u64;
            let mut dismissed = 0u64;
            let mut unknown = 0u64;
            for (id, n) in &ledger.by_hero {
                match o.town.heroes.iter().find(|h| h.guid == *id) {
                    Some(h) if h.alive => alive += n,
                    Some(_) => dead += n,
                    None if self
                        .dismissed
                        .iter()
                        .any(|h| h["hero"].as_u64() == Some(*id)) =>
                    {
                        dismissed += n
                    }
                    None => unknown += n,
                }
            }
            let qualified = heroes
                .iter()
                .filter(|h| h.level >= 4 && h.weapon >= 3 && h.armour >= 3)
                .count();
            let full = heroes
                .iter()
                .filter(|h| h.level >= 6 && h.weapon >= 4 && h.armour >= 4)
                .count();
            let apprentice_max = o
                .board
                .as_ref()
                .and_then(|b| b.quests.iter().find(|q| q.difficulty == 1))
                .and_then(|q| q.maximum_resolve_level);
            let veteran_max = o
                .board
                .as_ref()
                .and_then(|b| b.quests.iter().find(|q| q.difficulty == 3))
                .and_then(|q| q.maximum_resolve_level);
            self.weeks.insert(o.week,json!({"week":o.week,"returns":o.returns,"gold":gold,"caretaker56":super::caretaker::progress(o).expect("validated native checklist"),"progress90":o.steward020.as_ref().map(|s|s.achieved),"deaths":o.total_deaths,"completions":o.completions,"roster":heroes,"levels":(0..=6).map(|level|heroes.iter().filter(|h|h.level==level).count()).collect::<Vec<_>>(),"level1_plus":heroes.iter().filter(|h|h.level>=1).count(),"weapon_upgraded":heroes.iter().filter(|h|h.weapon>0).count(),"champion_qualified":qualified,"fully_equipped6":full,"qualified_team_upper_bounds":{"champion":qualified/4,"fully_equipped6":full/4,"apprentice":heroes.iter().filter(|h|apprentice_max.is_some_and(|m|h.level<=m)).count()/4,"veteran":heroes.iter().filter(|h|h.level>=2&&veteran_max.is_some_and(|m|h.level<=m)&&h.weapon>=2&&h.armour>=2).count()/4},"team_bound_note":"count/4 is a roster upper bound, not proof of four complementary roles","investment":{"alive":alive,"dead":dead,"dismissed":dismissed,"absent_unclassified":unknown},"ledger_spent":ledger.spent,"high_level_idle_weeks":self.idle_weeks,"observed_qualification_gaps":self.window_gaps}));
            self.last_roster = o.town.heroes.iter().map(|h| (h.guid, json!(h))).collect();
            self.last_returns = Some(o.returns);
        }
        if let Some(log) = log {
            if log["action"]["Provision"] == "Embark" {
                let q = &log["before"]["quest"];
                let key = format!(
                    "{}:{}:{}",
                    q["region"].as_str().unwrap_or("unknown"),
                    q["difficulty"],
                    q["length"]
                );
                *self.counts.entry(key).or_default() += 1;
                let week = log["before"]["week"].as_u64().unwrap_or(0);
                let supplies = ledger
                    .receipts
                    .iter()
                    .filter(|r| r["week"].as_u64() == Some(week) && r["purpose"] == "supplies")
                    .filter_map(|r| r["gold"].as_u64())
                    .sum::<u64>();
                self.active = Some(
                    json!({"quest":q,"party":log["before"]["party"],"start_week":week,"gold":log["before"]["gold"],"supply_gold":supplies,"deaths":log["before"]["deaths"],"completions":log["before"]["completions"],"boss_plan":log["strategy025"]["boss_plan"],"boss_fallback":log["strategy025"]["boss_fallback"]}),
                );
            }
            if log["action"]["Raid"]["Leave"]["complete"] == false {
                *self.counts.entry("abandons".into()).or_default() += 1;
            }
            if let Some(id) = log["action"]["Town"]["Dismiss"]["hero"].as_u64() {
                let h = self.last_roster.get(&id);
                self.dismissed.push(json!({"hero":id,"week":o.week,"level":h.map(|h|&h["level"]),"stress":h.map(|h|&h["stress"]),"investment":ledger.by_hero.get(&id).copied().unwrap_or(0)}));
            }
        }
    }
}

/// A trip owns its actual receipts, including repeated departures in one week.
pub fn trip_supply029(ledger: &ledger::Ledger, cursor: usize) -> u64 {
    ledger
        .receipts
        .iter()
        .skip(cursor)
        .filter(|r| r["purpose"] == "supplies")
        .filter_map(|r| r["gold"].as_u64())
        .sum()
}
