//!026-only conservative corrections. Old025 profiles have no sync026 block.
use super::*;
#[path = "sync026_boss.rs"]
pub mod boss_flex;
#[path = "sync026_diagnostics.rs"]
pub mod diagnostics;
#[path = "sync026_field.rs"]
pub mod field;
pub fn apply(profile: &mut Profile) -> Result<()> {
    let value: Value = serde_json::from_str(r#"{
  "version": "candidate-sync-026-v1",
  "sources": [
    "W-121",
    "W-112",
    "W-102",
    "W-114",
    "W-115"
  ],
  "heirlooms": {
    "enabled": true,
    "allowed_prefixes": [
      "blacksmith.",
      "guild."
    ],
    "max_gap_percent": 20,
    "max_batches": 3,
    "projection": "Do not count unperformed exchanges toward multi-building training gates; actual affordable upgrades remain executable."
  },
  "boss": {
    "enabled": true,
    "beam_per_function_signature": 2,
    "vestal_rank3_default": [
      "judgement",
      "divine_grace",
      "gods_comfort",
      "dazzling_light"
    ],
    "source": "W-112/W-102; empty slots use verified025 class loadouts; W-105 roaming and achievement exclusions retained; original2211 gate unchanged"
  },
  "field": {
    "enabled": true,
    "minimum_visited_rooms": 0,
    "minimum_moves": 1,
    "source": "W-114/W-115; native cures and food, all remaining bleed/blight and decaying burn, regen ignored conservatively; nominal original feast affordability before start, actual camp menu remains authoritative"
  },
  "diagnostics": true
}
"#)
        .map_err(|e| e.to_string())?;
    if value["version"] != "candidate-sync-026-v1" {
        return Err("unknown026 correction profile".into());
    }
    profile.parameters["sync026"] = value;
    if profile.parameters["sync026"]["boss"]["enabled"] == true {
        profile.rules.insert("S026-02".into(),Rule{group:"boss".into(),sources:vec!["W-112".into(),"W-102".into()],note:"rank3 Vestal judgement default; existing critical-HP/low-pressure thresholds select healing; rollout retains its own comparison".into()});
        for plan in &mut profile.boss_parties.plans {
            for member in &mut plan.slots {
                if member.class_id == "vestal" && member.rank == 3 {
                    member.skills = profile.parameters["sync026"]["boss"]["vestal_rank3_default"]
                        .as_array()
                        .ok_or("missing026 vestal defaults")?
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect();
                } else if member.skills.is_empty() {
                    member.skills = profile.parameters["loadouts"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|v| v["class_id"] == member.class_id)
                        .and_then(|v| v["skills"].as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .take(4)
                        .map(str::to_string)
                        .collect();
                }
            }
        }
    }
    if profile.parameters["sync026"]["field"]["enabled"] == true {
        profile.rules.insert(
            "S026-03".into(),
            Rule {
                group: "exploration".into(),
                sources: vec!["W-114".into(), "W-115".into()],
                note: "native rescue and useful camp before visible field safety exit".into(),
            },
        );
    }
    Ok(())
}
pub fn funding_plan(
    t: &crate::town016::Observation,
    account: &heirlooms::Account,
    q: &crate::town016::Choice,
    ctx: &Context<'_>,
    committed: u64,
) -> Option<Vec<(usize, u64)>> {
    let cfg = &ctx.profile.parameters["sync026"]["heirlooms"];
    let plan = account.funding_plan(t, &q.cost)?;
    if cfg["enabled"] != true || plan.is_empty() {
        return Some(plan);
    }
    let TownAction::Upgrade {
        hero: None, tree, ..
    } = &q.action
    else {
        return None;
    };
    if !cfg["allowed_prefixes"]
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .any(|p| tree.starts_with(p))
    {
        return None;
    }
    if plan
        .iter()
        .try_fold(committed, |n, (_, b)| n.checked_add(*b))?
        > cfg["max_batches"].as_u64()?
    {
        return None;
    }
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b > 0 {
            let n = a % b;
            a = b;
            b = n;
        }
        a
    }
    let mut rates = vec![];
    let mut denominator = 1u128;
    for (key, price) in q.cost.iter().filter(|(k, _)| k.starts_with("heirloom:")) {
        let (num, den) = if key == "heirloom:crest" {
            (1u128, 1u128)
        } else {
            let x = t
                .exchanges
                .iter()
                .filter(|x| &x.from == key && x.to == "heirloom:crest" && x.spend > 0 && x.gain > 0)
                .max_by(|a, b| {
                    (u64::from(a.gain) * u64::from(b.spend))
                        .cmp(&(u64::from(b.gain) * u64::from(a.spend)))
                })?;
            (u128::from(x.gain), u128::from(x.spend))
        };
        denominator = denominator.checked_mul(den / gcd(denominator, den))?;
        rates.push((key, *price, num, den));
    }
    let mut total = 0u128;
    let mut deficit = 0u128;
    for (key, price, num, den) in rates {
        let scale = num.checked_mul(denominator / den)?;
        total = total.checked_add(u128::from(price).checked_mul(scale)?)?;
        deficit = deficit.checked_add(
            u128::from(price.saturating_sub(t.wallet.get(key).copied().unwrap_or(0)))
                .checked_mul(scale)?,
        )?;
    }
    if total == 0
        || deficit.checked_mul(100)?
            > total.checked_mul(u128::from(cfg["max_gap_percent"].as_u64()?))?
    {
        return None;
    }
    Some(plan)
}
