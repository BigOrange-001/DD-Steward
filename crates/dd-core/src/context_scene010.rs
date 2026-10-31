use super::*;
use dd_data::{build010::Surprise, combat::GameData, items::SupplyStack};
pub fn context_scenario(g: &GameData) -> Result<(Scenario, Vec<SupplyStack>)> {
    let full = g.hero_ids().any(|id| id == "runaway");
    let mut heroes = if full {
        vec![
            g.hero(
                "flagellant",
                4,
                4,
                &["punish", "exsanguinate", "reclaim", "redeem"],
            )?,
            g.hero(
                "duelist",
                4,
                4,
                &[
                    "anticipation",
                    "touche",
                    "feint",
                    "disengage",
                    "fleche",
                    "coup_de_grace",
                    "the_boot",
                ],
            )?,
            g.hero(
                "runaway",
                4,
                4,
                &["firefly", "controlled_burn", "run_and_hide", "backdraft"],
            )?,
            g.hero("arbalest", 4, 4, &["sniper_shot", "flare"])?,
        ]
    } else {
        vec![
            g.hero(
                "abomination",
                4,
                4,
                &[
                    "transform",
                    "manacles",
                    "vomit",
                    "absolution",
                    "rake",
                    "rage",
                    "slam",
                ],
            )?,
            g.hero("highwayman", 4, 4, &["wicked_slice", "duelist_advance"])?,
            g.hero(
                "plague_doctor",
                4,
                4,
                &["plague_grenade", "battlefield_medicine"],
            )?,
            g.hero("vestal", 4, 4, &["divine_grace", "gods_comfort"])?,
        ]
    };
    let monsters = if full {
        vec![
            g.monster("carrion_eater_big_B")?,
            g.monster("virago_shroom_B")?,
            g.monster("skeleton_courtier_B")?,
        ]
    } else {
        vec![
            g.monster("skeleton_captain_B")?,
            g.monster("skeleton_common_B")?,
            g.monster("skeleton_courtier_B")?,
        ]
    };
    if full {
        heroes[2] = g.equip_build(
            heroes[2].clone(),
            dd_data::build010::Build {
                trinkets: vec!["rw_pyro_accelerant".into()],
                ..Default::default()
            },
        )?;
        heroes[0] = g.equip_build(
            heroes[0].clone(),
            dd_data::build010::Build {
                trinkets: vec!["damage_stone".into()],
                quirks: vec!["tough".into()],
                diseases: vec!["rabies".into()],
                ..Default::default()
            },
        )?;
    }
    let mut rules = g.rules.clone();
    rules.lighting = Some(g.lighting(25)?);
    rules.context = Some(g.battle_context(
        if full { "weald" } else { "crypts" }.into(),
        Surprise::Neither,
        false,
    )?);
    let supplies = if full {
        vec![SupplyStack {
            definition: g.supply("holy_water")?,
            remaining: 1,
        }]
    } else {
        vec![]
    };
    Ok((
        Scenario {
            heroes,
            monsters,
            rules,
            round_limit: 100,
        },
        supplies,
    ))
}
pub struct ContextBaseline;
impl Policy for ContextBaseline {
    fn choose(&mut self, o: &Observation) -> Option<Action> {
        let actor = o.actor?;
        if let Some(a) = o.actions.iter().find(|a| {
            a.label == "holy_water" && matches!(a.action,Action::Item{target,..}if target==actor)
        }) {
            return Some(a.action.clone());
        }
        if o.torch.is_some_and(|n| n < 50)
            && let Some(a) = o.actions.iter().find(|a| {
                a.label == "torch" && matches!(a.action,Action::Item{target,..}if target==actor)
            })
        {
            return Some(a.action.clone());
        }
        let unit = o.units.iter().find(|u| u.id == actor)?;
        let switch = if unit.mode.as_deref() == Some("human") {
            Some("transform")
        } else if unit.mode.as_deref() == Some("stance_defensive") {
            Some("anticipation")
        } else {
            None
        };
        if let Some(name) = switch
            && let Some(a) = o.actions.iter().find(|a| a.label == name)
        {
            return Some(a.action.clone());
        }
        if let Some(a) = o.actions.iter().find(|a| a.label == "controlled_burn") {
            return Some(a.action.clone());
        }
        Baseline.choose(o)
    }
}
