use super::*;
use dd_data::{build010::Surprise, combat::GameData};
pub fn boss_scenario(
    g: &GameData,
    ids: &[&str],
    region: &str,
) -> Result<(Scenario, BTreeMap<String, Blueprint>)> {
    let catalog = g.entity_catalog(ids)?;
    let monsters = ids
        .iter()
        .map(|id| catalog[*id].clone())
        .collect::<Vec<_>>();
    let mut rules = g.rules.clone();
    let surprise = if monsters
        .iter()
        .any(|b| b.mechanics.lifecycle.always_surprise)
    {
        Surprise::Heroes
    } else if monsters
        .iter()
        .any(|b| b.mechanics.lifecycle.always_be_surprised)
    {
        Surprise::Monsters
    } else {
        Surprise::Neither
    };
    rules.context = Some(g.battle_context(region.into(), surprise, false)?);
    let torch = monsters
        .iter()
        .filter_map(|b| b.mechanics.boss.torch)
        .map(|r| r[1])
        .min()
        .unwrap_or(100);
    rules.lighting = Some(g.lighting(torch)?);
    if region == "courtyard"
        && g.overlay
            .config
            .packages
            .iter()
            .any(|p| p == "crimson_court")
    {
        let blood = g.bloodlight()?;
        rules.lighting = Some(dd_data::lighting::Lighting {
            torch: 100,
            table: blood.darkness.clone(),
        });
        rules.bloodlight = Some(blood);
    }

    if region == "darkestdungeon"
        && !g
            .overlay
            .sources("campaign/quest/quest.plot_quests.json")
            .is_empty()
    {
        let quest = if ids.iter().any(|id| id.starts_with("ancestor_")) {
            "plot_darkest_dungeon_4"
        } else if ids.iter().any(|id| id.starts_with("cell_")) {
            "plot_darkest_dungeon_3"
        } else if ids.iter().any(|id| id.starts_with("templar_")) {
            "plot_darkest_dungeon_2"
        } else {
            "plot_darkest_dungeon_1"
        };
        rules.encounter_contract = Some(g.battle_contract(quest)?);
    }
    let s = Scenario {
        heroes: vec![
            g.hero(
                "highwayman",
                4,
                4,
                &["wicked_slice", "pistol_shot", "duelist_advance"],
            )?,
            g.hero(
                "houndmaster",
                4,
                4,
                &["hounds_rush", "hounds_harry", "lick_wounds"],
            )?,
            g.hero(
                "vestal",
                4,
                4,
                &["divine_grace", "gods_comfort", "judgement"],
            )?,
            g.hero(
                "plague_doctor",
                4,
                4,
                &["noxious_blast", "plague_grenade", "battlefield_medicine"],
            )?,
        ],
        monsters,
        rules,
        round_limit: 100,
    };
    Ok((s, catalog))
}
pub fn boss_battle(g: &GameData, ids: &[&str], region: &str, seed: u64) -> Result<Battle> {
    let (s, catalog) = boss_scenario(g, ids, region)?;
    Battle::new_encounter(s, seed, catalog, vec![])
}
pub fn boss_step(b: &mut Battle) -> Result<Vec<Event>> {
    let o = b.observation();
    if o.player_selection || o.units.iter().any(|u| Some(u.id) == o.actor && u.hero) {
        b.step(Baseline.choose(&o).ok_or("missing player action")?)
    } else {
        b.step_monster()
    }
}

pub fn boss_encounter_battle(
    g: &GameData,
    d: &dd_data::encounter011::Encounter,
    seed: u64,
) -> Result<Battle> {
    boss_encounter_battle_in_region(g, d, seed, None)
}
pub fn boss_encounter_battle_in_region(
    g: &GameData,
    d: &dd_data::encounter011::Encounter,
    seed: u64,
    shared_region: Option<&str>,
) -> Result<Battle> {
    let ids = d.monsters.iter().map(String::as_str).collect::<Vec<_>>();
    let region = if d.region == "_shared" {
        shared_region.ok_or("shared roaming formation requires explicit host dungeon context")?
    } else {
        &d.region
    };
    let (mut s, catalog) = boss_scenario(g, &ids, region)?;
    if let Some(id) = &d.flashback {
        s.rules.flashback = Some(g.flashback(id)?);
        s.heroes[0] = g.hero(
            "shieldbreaker",
            4,
            4,
            &["pierce", "break_guard", "spearing", "serpents_sway"],
        )?;
    }
    Battle::new_encounter(s, seed, catalog, vec![])
}
