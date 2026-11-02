//! Read-only026 measurements. Nothing in this module is consulted by any policy.
use super::*;
use crate::campaign018::runner::View;
use std::sync::Arc;
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Audit {
    pub counts: BTreeMap<String, u64>,
    pub xp_by_class: BTreeMap<String, u64>,
    pub departures: Arc<Vec<Value>>,
    pub diagnostic_errors: BTreeMap<String, u64>,
    tail_seen: Arc<BTreeSet<String>>,
    encounter_serial: u64,
    in_battle: bool,
}
fn xp(v: &View) -> BTreeMap<u64, (String, u32)> {
    match v {
        View::Opening(o) => o
            .heroes
            .iter()
            .map(|h| (h.guid, (h.class.clone(), h.xp)))
            .collect(),
        View::Campaign(o) => o
            .town
            .heroes
            .iter()
            .map(|h| (h.guid, (h.class.clone(), h.xp)))
            .collect(),
    }
}
fn battle(v: &View) -> Option<&crate::Observation> {
    match v {
        View::Opening(o) => o.battle.as_ref(),
        View::Campaign(o) => o
            .expedition
            .as_ref()
            .and_then(|r| r.battle.as_ref())
            .or_else(|| {
                o.content019
                    .as_ref()
                    .and_then(|c| c.farm.as_ref())
                    .map(|f| &f.battle)
            }),
    }
}
impl Audit {
    fn count(&mut self, key: &str) {
        *self.counts.entry(key.into()).or_default() += 1;
    }
    pub fn observe(
        &mut self,
        before: &View,
        after: &View,
        log: &Value,
        m: &Memory,
        p: &Profile,
        s: &Sources,
    ) {
        let was = battle(before).is_some();
        let now = battle(after).is_some();
        if was && !self.in_battle || !was && now {
            self.encounter_serial += 1;
        }
        self.in_battle = now;
        let old = xp(before);
        for (id, (class, n)) in xp(after) {
            if let Some((_, previous)) = old.get(&id) {
                *self.xp_by_class.entry(class).or_default() +=
                    u64::from(n.saturating_sub(*previous));
            }
        }
        if log["action"]["Provision"] == "Embark"
            && let View::Campaign(o) = after
            && let Some(r) = &o.expedition
        {
            let mut types = BTreeMap::<String, u64>::new();
            let mut party = vec![];
            for guid in &r.party {
                if let Some(h) = r.heroes.iter().find(|h| h.guid == *guid) {
                    let ids = h
                        .equipped023
                        .as_ref()
                        .map(|es| es.iter().map(|e| e.id.clone()).collect::<Vec<_>>())
                        .unwrap_or_else(|| {
                            o.town
                                .heroes
                                .iter()
                                .find(|x| x.guid == *guid)
                                .map_or(vec![], |h| {
                                    h.equipped.iter().map(|(_, _, id)| id.clone()).collect()
                                })
                        });
                    for id in &ids {
                        *types.entry(id.clone()).or_default() += 1;
                    }
                    party.push(json!({"guid":guid,"class":h.class,"count":ids.len(),"trinket_ids":ids,"physical":h.equipped023}));
                }
            }
            Arc::make_mut(&mut self.departures).push(json!({"week":o.week,"returns":o.returns,"quest":log["before"]["quest"],"party":party,"types_by_id":types,"total":types.values().sum::<u64>(),"boss_plan":m.state.get("boss_plan"),"boss_match":m.state.get("boss_match")}));
        }
        let Some(b) = battle(before) else {
            return;
        };
        let action = log["action"]["Raid"]
            .get("Fight")
            .or_else(|| log["action"]["Farm"].get("Fight"))
            .or_else(|| log["action"]["Opening"].get("Fight"));
        let Some(chosen) = action
            .and_then(|v| serde_json::from_value::<crate::Action>(v.clone()).ok())
            .and_then(|a| b.actions.iter().find(|x| x.action == a))
        else {
            return;
        };
        let fraction = p.parameters["combat"]["self_heal_hp_fraction"]
            .as_f64()
            .unwrap_or(0.5);
        let wounded = b
            .units
            .iter()
            .filter(|u| u.hero && !u.corpse && f64::from(u.hp) <= f64::from(u.max_hp) * fraction)
            .map(|u| u.id)
            .collect::<BTreeSet<_>>();
        let can_heal = b
            .actions
            .iter()
            .any(|a| a.heal[1] > 0 && a.targets.iter().any(|id| wounded.contains(id)));
        if can_heal {
            self.count("wounded_legal_healing_opportunities");
            if chosen.heal[1] > 0 && chosen.targets.iter().any(|id| wounded.contains(id)) {
                self.count("wounded_healing_selected");
            } else {
                self.count("wounded_other_action_selected");
            }
        }
        let assessments = match threat::evaluate(b, &s.enemies) {
            Ok(v) => v,
            Err(e) => {
                *self.diagnostic_errors.entry(e).or_default() += 1;
                return;
            }
        };
        let minhp = b
            .units
            .iter()
            .filter(|u| u.hero && !u.corpse)
            .map(|u| u.hp.max(1))
            .min()
            .unwrap_or(1);
        let tail = !assessments.is_empty()
            && assessments.len()
                <= p.parameters["combat"]["tail_enemy_limit"]
                    .as_u64()
                    .unwrap_or(2) as usize
            && b.round
                <= p.parameters["combat"]["tail_max_round"]
                    .as_u64()
                    .unwrap_or(8) as u32
            && assessments.iter().map(|a| a.damage).sum::<f64>()
                <= f64::from(minhp)
                    * p.parameters["combat"]["tail_hp_fraction"]
                        .as_f64()
                        .unwrap_or(0.35)
            && assessments.iter().map(|a| a.stress).sum::<f64>()
                <= p.parameters["combat"]["tail_stress_limit"]
                    .as_f64()
                    .unwrap_or(10.)
            && assessments.iter().all(|a| !a.uncertain && a.support == 0.)
            && b.units
                .iter()
                .filter(|u| !u.hero && !u.corpse && u.hp > 0)
                .all(|u| {
                    s.enemies
                        .get(&threat::key(u))
                        .is_some_and(|e| !e.boss && !e.accelerate)
                });
        if tail && (chosen.stress_heal > 0 || chosen.heal[1] > 0) {
            self.count("tail_recovery_actions");
            if chosen.stress_heal > 0 {
                self.count("tail_relief_actions");
            }
            if chosen.heal[1] > 0 {
                self.count("tail_healing_actions");
            }
            let key = json!([self.encounter_serial, b.wave]).to_string();
            if Arc::make_mut(&mut self.tail_seen).insert(key) {
                self.count("tail_recovery_sessions");
            }
        }
    }
    pub fn medium_summary(m: &Memory) -> Value {
        let trips = m
            .metrics
            .trips
            .iter()
            .filter(|t| t["quest"]["length"] == 2)
            .collect::<Vec<_>>();
        json!({"returned":trips.len(),"completed":trips.iter().filter(|t|t["complete"]==true).count(),"pending":m.metrics.active.as_ref().is_some_and(|t|t["quest"]["length"]==2)})
    }
}
