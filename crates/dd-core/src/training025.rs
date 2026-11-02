//! Complete public training dependencies and source-priced multi-tier plans.
use super::*;
pub type Key = (Option<u64>, String, String);
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub cost: BTreeMap<String, u32>,
    pub requires: Vec<Key>,
    pub unavailable: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Catalog {
    pub nodes: BTreeMap<Key, Node>,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub actions: Vec<crate::town016::Choice>,
    pub cost: BTreeMap<String, u32>,
}
impl Catalog {
    pub fn plan(&self, t: &crate::town016::Observation, goals: &[Key]) -> Result<Plan> {
        fn visit(
            c: &Catalog,
            t: &crate::town016::Observation,
            k: &Key,
            seen: &mut BTreeSet<Key>,
            active: &mut BTreeSet<Key>,
            p: &mut Plan,
        ) -> Result<()> {
            if t.purchased.contains(k) || seen.contains(k) {
                return Ok(());
            }
            if !active.insert(k.clone()) {
                return Err("025 cyclic training prerequisites".into());
            }
            let n = c
                .nodes
                .get(k)
                .ok_or_else(|| format!("025 no complete public training quote: {k:?}"))?;
            if let Some(e) = &n.unavailable {
                return Err(format!("{e}: {k:?}"));
            }
            for dep in &n.requires {
                visit(c, t, dep, seen, active, p)?;
            }
            active.remove(k);
            seen.insert(k.clone());
            for (currency, amount) in &n.cost {
                let v = p.cost.entry(currency.clone()).or_default();
                *v = v.checked_add(*amount).ok_or("025 training cost overflow")?;
            }
            p.actions.push(crate::town016::Choice {
                action: TownAction::Upgrade {
                    hero: k.0,
                    tree: k.1.clone(),
                    code: k.2.clone(),
                },
                cost: n.cost.clone(),
            });
            Ok(())
        }
        let mut p = Plan::default();
        let mut seen = BTreeSet::new();
        let mut active = BTreeSet::new();
        for k in goals {
            visit(self, t, k, &mut seen, &mut active, &mut p)?;
        }
        Ok(p)
    }
}
pub fn goals(
    t: &crate::town016::Observation,
    ids: &[u64],
    target: usize,
    ctx: &Context<'_>,
) -> Vec<Key> {
    let mut goals = vec![];
    for h in t.heroes.iter().filter(|h| ids.contains(&h.guid)) {
        for (kind, have) in [("weapon", h.weapon), ("armour", h.armour)] {
            if have < target {
                goals.push((
                    Some(h.guid),
                    format!("{}.{kind}", h.class),
                    (target - 1).to_string(),
                ));
            }
        }
        for skill in ctx.profile.parameters["skill_core"][&h.class]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if h.skills.get(skill).copied().unwrap_or(-1) < target as i32
                && (super::town::selected(h, ctx).iter().any(|s| s == skill)
                    || ctx.on("skills")
                        && super::loadout::desired(h, ctx).iter().any(|s| s == skill))
            {
                goals.push((
                    Some(h.guid),
                    format!("{}.{skill}", h.class),
                    target.to_string(),
                ));
            }
        }
    }
    goals
}
