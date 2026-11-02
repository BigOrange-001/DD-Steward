//! Source-bound project objectives, independent of policy and hidden campaign state.
use crate::{
    campaign017::{Data, uint},
    combat::{GameData, Result, text},
    library,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "steward-objectives-020-v1";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: String,
    pub training_level: u32,
    pub training_rule: String,
    pub kill_rule: String,
    pub extra_families: BTreeMap<String, String>,
    pub conditional_packages: BTreeMap<String, String>,
    pub components: BTreeSet<String>,
    pub unreachable: BTreeMap<String, String>,
    pub precursors: BTreeMap<String, String>,
    pub major_content: String,
    pub scope: String,
    pub calibration: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Boss {
    pub id: String,
    pub targets: BTreeSet<String>,
    pub amount: u32,
    pub source_goals: BTreeSet<String>,
    pub source_plots: BTreeSet<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: String,
    pub layer: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub profile: String,
    pub content: String,
    pub full_scope: bool,
    pub policy: Policy,
    pub training_xp: u32,
    pub classes: BTreeSet<String>,
    pub bosses: BTreeMap<String, Boss>,
    pub plots: BTreeSet<String>,
    pub repeats: BTreeSet<String>,
    pub dispositions: BTreeMap<String, String>,
    pub sources: Vec<Source>,
}
impl Catalog {
    pub fn load(g: &GameData, campaign: &Data, town: &crate::town016::Data) -> Result<Self> {
        let policy: Policy =
            serde_json::from_str(r#"{
  "version": "steward-objectives-020-v1",
  "training_level": 6,
  "training_rule": "User confirmed: only an actual expedition return crossing from below level six to six counts; shard_hungry mercenaries excluded; achievement remains after death or dismissal. Recruitment at six and town-only XP never count.",
  "kill_rule": "Native final-death receipts, settled once at actual raid return. A boss goal's original target count must be met within that raid history; shared Flesh parts are one goal. Transformation is not death. Persistent histories are consumed by new suffix only for statistics, while goal matching uses the authenticated whole encounter history.",
  "extra_families": {
    "collector": "all",
    "fanatic": "all",
    "thing": "all",
    "shambler": "all",
    "crocodile": "all",
    "com_crocodile": "all",
    "crow": "all",
    "miller": "C",
    "galaxy": "D",
    "statue_head": "C",
    "templar_melee": "D",
    "templar_ranged": "D",
    "cell_white": "D"
  },
  "components": [
    "body_average",
    "body_bloated",
    "body_emaciated",
    "brigand_fuseman",
    "bulrush",
    "cattail",
    "cauldron_empty",
    "cauldron_full",
    "com_bulrush",
    "com_cattail",
    "curtain",
    "drowned_anchor",
    "drowned_anchored",
    "nest",
    "pew_large",
    "pew_medium",
    "pew_small",
    "pyre_empty",
    "pyre_full",
    "shambler_tentacle",
    "statue_hand",
    "statue_shield",
    "swine_piglet"
  ],
  "unreachable": {
    "miller_A": "No enabled farm contract selects a tier-one boss wave; first contract has only ordinary intro waves.",
    "statue_head_A": "Enabled courtyard repeat kill_statue source selects the C goal; fixed named statue formations also select C.",
    "statue_head_B": "No enabled courtyard campaign goal or fixed named statue formation selects B."
  },
  "precursors": {
    "spire_D": "Sleeper encounter predecessor; count actual terminal galaxy_D death, not spire destruction."
  },
  "major_content": "All enabled DD, courtyard and farm plot entries plus the source courtyard bloodcrate and guardian repeats. Disabled tests excluded. Boss kills and mission completion are separate requirements.",
  "scope": "Full Steward success requires the complete ROADMAP package set. Disabled packages have no corresponding objectives or reads and are explicitly marked as configured-subset evaluation, never full Steward success.",
  "calibration": "Classification is an explicit project objective interpretation, not original achievement or executable equivalence. Future unclassified enabled boss families error. No policy access to source definitions, hidden encounters or RNG.",
  "conditional_packages": {
    "crow_D": "color_of_madness",
    "shambler_E": "color_of_madness"
  }
}
"#)
                .map_err(|e| e.to_string())?;
        if policy.version != PROFILE || policy.training_level != 6 {
            return Err("unknown Steward objective policy".into());
        }
        let training_xp = town.base.roster_variables["resolve_level_thresholds"]
            .as_array()
            .and_then(|v| v.get(policy.training_level as usize))
            .and_then(|n| n.as_u64())
            .and_then(|n| u32::try_from(n).ok())
            .ok_or("missing original level-six threshold")?;
        let mut c = Self {
            profile: PROFILE.into(),
            content: g.overlay.config.id()?,
            full_scope: g.overlay.config.packages.iter().collect::<BTreeSet<_>>()
                == crate::content::ContentConfig::full()
                    .packages
                    .iter()
                    .collect::<BTreeSet<_>>(),
            policy,
            training_xp,
            classes: g.hero_ids().cloned().collect(),
            bosses: BTreeMap::new(),
            plots: BTreeSet::new(),
            repeats: BTreeSet::new(),
            dispositions: BTreeMap::new(),
            sources: vec![],
        };
        for (id, q) in &campaign.quests {
            if q["is_enabled"] == false {
                continue;
            }
            if [
                "plot_darkest_dungeon_",
                "plot_crimson_court_",
                "plot_color_of_madness_",
            ]
            .iter()
            .any(|p| id.starts_with(p))
            {
                c.plots.insert(id.clone());
            }
            for goal in q["quest"]["goal_ids"]
                .as_array()
                .ok_or("missing source quest goals")?
            {
                let goal = goal.as_str().ok_or("invalid source goal ID")?;
                if campaign
                    .goals
                    .get(goal)
                    .is_some_and(|v| v["type"] == "kill_monster")
                {
                    c.add_goal(campaign, goal, Some(id))?;
                }
            }
        }
        if g.overlay
            .config
            .packages
            .iter()
            .any(|p| p == "crimson_court")
        {
            let maps = crate::catalog014::Catalog::load(
                &g.overlay,
                crate::maps014::Candidates::for_content(&g.overlay.config)?,
            )?;
            for kind in ["farm_bloodcrate", "kill_statue"] {
                let (_, goal) = maps.repeat_rule(kind)?;
                if campaign
                    .goals
                    .get(&goal)
                    .is_some_and(|v| v["type"] == "kill_monster")
                {
                    c.add_goal(campaign, &goal, None)?;
                }
                c.repeats.insert(kind.into());
            }
        }
        let ids: BTreeSet<_> = g.monster_ids().cloned().collect();
        let encounters = g.boss_encounters()?;
        for id in &ids {
            let (family, variant) = id.rsplit_once('_').ok_or("monster ID without variant")?;
            if c.policy
                .extra_families
                .get(family)
                .is_some_and(|v| v == "all" || v == variant)
                && !c.bosses.values().any(|b| b.targets.contains(id))
            {
                if !encounters.iter().any(|e| e.monsters.contains(id))
                    && !["templar_melee", "templar_ranged", "cell_white", "galaxy"]
                        .contains(&family)
                {
                    if c.policy
                        .conditional_packages
                        .get(id)
                        .is_some_and(|package| !g.overlay.config.packages.contains(package))
                    {
                        c.dispositions.insert(id.clone(),"retained base definition; its explicitly classified encounter package is disabled and no enabled encounter references it".into());
                        continue;
                    }
                    return Err(format!(
                        "Steward extra boss lacks an enabled encounter: {id}"
                    ));
                }
                let key = format!("boss:{id}");
                c.bosses.insert(
                    key.clone(),
                    Boss {
                        id: key,
                        targets: BTreeSet::from([id.clone()]),
                        amount: 1,
                        source_goals: BTreeSet::new(),
                        source_plots: BTreeSet::new(),
                    },
                );
            }
        }
        for path in g.overlay.combat_files("monsters/", ".info.darkest") {
            let doc = library::read_document(&g.overlay, &path)?;
            if !doc
                .records_keyed("tag")
                .any(|r| text(r, "id").ok() == Some("boss"))
            {
                continue;
            }
            let id = path.split('/').nth(2).ok_or("invalid boss source path")?;
            let family = id.rsplit_once('_').ok_or("invalid boss variant")?.0;
            let disposition = if let Some(reason) = c.dispositions.get(id) {
                reason.clone()
            } else if c.bosses.values().any(|b| b.targets.contains(id)) {
                "required canonical target or equivalent source-goal form".into()
            } else if c.policy.components.contains(family) {
                "supporting component; not a separate boss objective".into()
            } else if let Some(reason) = c.policy.unreachable.get(id) {
                format!("unreachable variant: {reason}")
            } else if let Some(reason) = c.policy.precursors.get(id) {
                format!("encounter precursor: {reason}")
            } else {
                return Err(format!(
                    "unclassified enabled boss {id}; update objective disposition explicitly"
                ));
            };
            c.dispositions.insert(id.into(), disposition);
        }
        if g.overlay
            .config
            .packages
            .iter()
            .any(|p| p == "color_of_madness")
        {
            let plan = g.farm_plan(
                "plot_color_of_madness_0",
                campaign.quests.keys().cloned().collect(),
            )?;
            if plan
                .params
                .intro_wave_groups
                .iter()
                .chain(&plan.params.recurring_wave_groups)
                .flat_map(|group| &group.wave_infos)
                .any(|w| !w.kind.is_empty())
            {
                return Err(
                    "first farm now selects special waves; revisit miller_A disposition".into(),
                );
            }
        }
        // Audit only active logical sources. No disabled-package directory is read.
        for path in g.overlay.paths.keys().filter(|p| {
            (p.starts_with("monsters/") || p.starts_with("heroes/")) && p.ends_with(".info.darkest")
                || p.starts_with("dungeons/")
                    && (p.ends_with(".mash.darkest") || p.ends_with(".wavemash.darkest"))
                || p.starts_with("campaign/quest/") && p.ends_with(".json")
                || p.starts_with("campaign/roster/")
                || p.starts_with("maps/") && (p.ends_with(".dm") || p.ends_with(".csv"))
        }) {
            for layer in g.overlay.sources(path) {
                let bytes = crate::telemetry024c::read(g.overlay.layers[layer].root.join(path))
                    .map_err(|e| e.to_string())?;
                c.sources.push(Source {
                    path: path.clone(),
                    layer: g.overlay.layers[layer].id.clone(),
                    sha256: format!("{:x}", Sha256::digest(bytes)),
                });
            }
        }
        if c.bosses.is_empty() || c.classes.is_empty() || c.plots.is_empty() {
            return Err("empty Steward objective catalog".into());
        }
        Ok(c)
    }
    fn add_goal(&mut self, data: &Data, goal: &str, plot: Option<&str>) -> Result<()> {
        let raw = library::unique_json(&data.goals, goal, "Steward kill goal")?;
        let targets = raw["data"]["monster_class_ids"]
            .as_array()
            .ok_or("missing source boss targets")?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "invalid boss target".to_string())
            })
            .collect::<Result<BTreeSet<_>>>()?;
        let amount = uint(&raw["data"], "amount")?;
        if targets.is_empty() || amount == 0 {
            return Err("empty original boss objective".into());
        }
        if let Some(b) = self
            .bosses
            .values_mut()
            .find(|b| b.targets == targets && b.amount == amount)
        {
            b.source_goals.insert(goal.into());
            if let Some(plot) = plot {
                b.source_plots.insert(plot.into());
            }
            return Ok(());
        }
        let id = format!("boss:{goal}");
        self.bosses.insert(
            id.clone(),
            Boss {
                id,
                targets,
                amount,
                source_goals: BTreeSet::from([goal.into()]),
                source_plots: plot.map(str::to_string).into_iter().collect(),
            },
        );
        Ok(())
    }
    pub fn identity(&self) -> Result<String> {
        Ok(format!(
            "{PROFILE};sha256:{:x}",
            Sha256::digest(serde_json::to_vec(self).map_err(|e| e.to_string())?)
        ))
    }
}
