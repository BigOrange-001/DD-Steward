//! Native localization checklist, independent of the historical020 catalog.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub localization_id: String,
    pub label: String,
    pub source: String,
    pub plot: Option<String>,
    pub class_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub version: String,
    pub source: String,
    pub goals: Vec<Goal>,
}
pub fn catalog() -> Result<Catalog> {
    let c: Catalog = serde_json::from_str(r#"{
  "version": "native-caretaker-56-v1",
  "source": "W-106; enabled localization English entries, 37 plots plus 19 ordinary classes",
  "goals": [
    {
      "id": "plot_darkest_dungeon_1",
      "localization_id": "str_caretaker_goal_plot_darkest_dungeon_1",
      "label": "Succeed in your first assault on the Darkest Dungeon.",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_darkest_dungeon_1"
    },
    {
      "id": "plot_darkest_dungeon_2",
      "localization_id": "str_caretaker_goal_plot_darkest_dungeon_2",
      "label": "Succeed in your second assault on the Darkest Dungeon.",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_darkest_dungeon_2"
    },
    {
      "id": "plot_darkest_dungeon_3",
      "localization_id": "str_caretaker_goal_plot_darkest_dungeon_3",
      "label": "Succeed in your third assault on the Darkest Dungeon.",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_darkest_dungeon_3"
    },
    {
      "id": "plot_darkest_dungeon_4",
      "localization_id": "str_caretaker_goal_plot_darkest_dungeon_4",
      "label": "Complete the final Darkest Dungeon quest.",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_darkest_dungeon_4"
    },
    {
      "id": "plot_kill_brigand_cannon_1",
      "localization_id": "str_caretaker_goal_plot_kill_brigand_cannon_1",
      "label": "Defeat the Brigand 8 Pounder",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_brigand_cannon_1"
    },
    {
      "id": "plot_kill_brigand_cannon_2",
      "localization_id": "str_caretaker_goal_plot_kill_brigand_cannon_2",
      "label": "Defeat the Brigand 12 Pounder",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_brigand_cannon_2"
    },
    {
      "id": "plot_kill_brigand_cannon_3",
      "localization_id": "str_caretaker_goal_plot_kill_brigand_cannon_3",
      "label": "Defeat the Brigand 16 Pounder",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_brigand_cannon_3"
    },
    {
      "id": "plot_kill_drowned_crew_1",
      "localization_id": "str_caretaker_goal_plot_kill_drowned_crew_1",
      "label": "Defeat the Sodden Crew",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_drowned_crew_1"
    },
    {
      "id": "plot_kill_drowned_crew_2",
      "localization_id": "str_caretaker_goal_plot_kill_drowned_crew_2",
      "label": "Defeat the Sunken Crew",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_drowned_crew_2"
    },
    {
      "id": "plot_kill_drowned_crew_3",
      "localization_id": "str_caretaker_goal_plot_kill_drowned_crew_3",
      "label": "Defeat the Drowned Crew",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_drowned_crew_3"
    },
    {
      "id": "plot_kill_formless_flesh_1",
      "localization_id": "str_caretaker_goal_plot_kill_formless_flesh_1",
      "label": "Defeat the Inchoate Flesh",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_formless_flesh_1"
    },
    {
      "id": "plot_kill_formless_flesh_2",
      "localization_id": "str_caretaker_goal_plot_kill_formless_flesh_2",
      "label": "Defeat the Unstable Flesh",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_formless_flesh_2"
    },
    {
      "id": "plot_kill_formless_flesh_3",
      "localization_id": "str_caretaker_goal_plot_kill_formless_flesh_3",
      "label": "Defeat the Formless Flesh",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_formless_flesh_3"
    },
    {
      "id": "plot_kill_hag_1",
      "localization_id": "str_caretaker_goal_plot_kill_hag_1",
      "label": "Defeat the Wizened Hag",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_hag_1"
    },
    {
      "id": "plot_kill_hag_2",
      "localization_id": "str_caretaker_goal_plot_kill_hag_2",
      "label": "Defeat the Hag",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_hag_2"
    },
    {
      "id": "plot_kill_hag_3",
      "localization_id": "str_caretaker_goal_plot_kill_hag_3",
      "label": "Defeat the Hag Witch",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_hag_3"
    },
    {
      "id": "plot_kill_necromancer_1",
      "localization_id": "str_caretaker_goal_plot_kill_necromancer_1",
      "label": "Defeat the Apprentice Necromancer",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_necromancer_1"
    },
    {
      "id": "plot_kill_necromancer_2",
      "localization_id": "str_caretaker_goal_plot_kill_necromancer_2",
      "label": "Defeat the Necromancer",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_necromancer_2"
    },
    {
      "id": "plot_kill_necromancer_3",
      "localization_id": "str_caretaker_goal_plot_kill_necromancer_3",
      "label": "Defeat the Necromancer Lord",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_necromancer_3"
    },
    {
      "id": "plot_kill_prophet_1",
      "localization_id": "str_caretaker_goal_plot_kill_prophet_1",
      "label": "Defeat the Sonorous Prophet",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_prophet_1"
    },
    {
      "id": "plot_kill_prophet_2",
      "localization_id": "str_caretaker_goal_plot_kill_prophet_2",
      "label": "Defeat the Fulminating Prophet",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_prophet_2"
    },
    {
      "id": "plot_kill_prophet_3",
      "localization_id": "str_caretaker_goal_plot_kill_prophet_3",
      "label": "Defeat the Gibbering Prophet",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_prophet_3"
    },
    {
      "id": "plot_kill_siren_1",
      "localization_id": "str_caretaker_goal_plot_kill_siren_1",
      "label": "Defeat the Siren",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_siren_1"
    },
    {
      "id": "plot_kill_siren_2",
      "localization_id": "str_caretaker_goal_plot_kill_siren_2",
      "label": "Defeat the Alluring Siren",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_siren_2"
    },
    {
      "id": "plot_kill_siren_3",
      "localization_id": "str_caretaker_goal_plot_kill_siren_3",
      "label": "Defeat the Beguiling Siren",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_siren_3"
    },
    {
      "id": "plot_kill_swine_prince_1",
      "localization_id": "str_caretaker_goal_plot_kill_swine_prince_1",
      "label": "Defeat the Swine Prince",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_swine_prince_1"
    },
    {
      "id": "plot_kill_swine_prince_2",
      "localization_id": "str_caretaker_goal_plot_kill_swine_prince_2",
      "label": "Defeat the Swine King",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_swine_prince_2"
    },
    {
      "id": "plot_kill_swine_prince_3",
      "localization_id": "str_caretaker_goal_plot_kill_swine_prince_3",
      "label": "Defeat the Swine God",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_kill_swine_prince_3"
    },
    {
      "id": "plot_town_invasion_0",
      "localization_id": "str_caretaker_goal_plot_town_invasion_0",
      "label": "Repel the Brigand Incursion",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_town_invasion_0"
    },
    {
      "id": "plot_tutorial_crypts",
      "localization_id": "str_caretaker_goal_plot_tutorial_crypts",
      "label": "Successfully complete your first foray into the Ruins",
      "source": "localization/miscellaneous.string_table.xml",
      "plot": "plot_tutorial_crypts"
    },
    {
      "id": "plot_crimson_court_0",
      "localization_id": "str_caretaker_goal_plot_crimson_court_0",
      "label": "Explore the Courtyard",
      "source": "dlc/580100_crimson_court/localization/CC.string_table.xml",
      "plot": "plot_crimson_court_0"
    },
    {
      "id": "plot_crimson_court_1",
      "localization_id": "str_caretaker_goal_plot_crimson_court_1",
      "label": "Dismiss the Baron.",
      "source": "dlc/580100_crimson_court/localization/CC.string_table.xml",
      "plot": "plot_crimson_court_1"
    },
    {
      "id": "plot_crimson_court_2",
      "localization_id": "str_caretaker_goal_plot_crimson_court_2",
      "label": "Unseat the Viscount.",
      "source": "dlc/580100_crimson_court/localization/CC.string_table.xml",
      "plot": "plot_crimson_court_2"
    },
    {
      "id": "plot_crimson_court_3",
      "localization_id": "str_caretaker_goal_plot_crimson_court_3",
      "label": "Depose the Countess.",
      "source": "dlc/580100_crimson_court/localization/CC.string_table.xml",
      "plot": "plot_crimson_court_3"
    },
    {
      "id": "plot_color_of_madness_0",
      "localization_id": "str_caretaker_goal_plot_color_of_madness_0",
      "label": "Explore the Farmstead",
      "source": "dlc/735730_color_of_madness/localization/CoM.string_table.xml",
      "plot": "plot_color_of_madness_0"
    },
    {
      "id": "plot_color_of_madness_1",
      "localization_id": "str_caretaker_goal_plot_color_of_madness_1",
      "label": "Defeat the Miller",
      "source": "dlc/735730_color_of_madness/localization/CoM.string_table.xml",
      "plot": "plot_color_of_madness_1"
    },
    {
      "id": "plot_color_of_madness_2",
      "localization_id": "str_caretaker_goal_plot_color_of_madness_2",
      "label": "Defeat the Sleeper",
      "source": "dlc/735730_color_of_madness/localization/CoM.string_table.xml",
      "plot": "plot_color_of_madness_2"
    },
    {
      "id": "class:abomination",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise abomination to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "abomination"
    },
    {
      "id": "class:antiquarian",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise antiquarian to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "antiquarian"
    },
    {
      "id": "class:arbalest",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise arbalest to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "arbalest"
    },
    {
      "id": "class:bounty_hunter",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise bounty_hunter to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "bounty_hunter"
    },
    {
      "id": "class:crusader",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise crusader to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "crusader"
    },
    {
      "id": "class:duelist",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise duelist to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "duelist"
    },
    {
      "id": "class:flagellant",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise flagellant to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "flagellant"
    },
    {
      "id": "class:grave_robber",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise grave_robber to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "grave_robber"
    },
    {
      "id": "class:hellion",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise hellion to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "hellion"
    },
    {
      "id": "class:highwayman",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise highwayman to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "highwayman"
    },
    {
      "id": "class:houndmaster",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise houndmaster to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "houndmaster"
    },
    {
      "id": "class:jester",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise jester to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "jester"
    },
    {
      "id": "class:leper",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise leper to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "leper"
    },
    {
      "id": "class:man_at_arms",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise man_at_arms to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "man_at_arms"
    },
    {
      "id": "class:occultist",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise occultist to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "occultist"
    },
    {
      "id": "class:plague_doctor",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise plague_doctor to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "plague_doctor"
    },
    {
      "id": "class:runaway",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise runaway to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "runaway"
    },
    {
      "id": "class:shieldbreaker",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise shieldbreaker to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "shieldbreaker"
    },
    {
      "id": "class:vestal",
      "localization_id": "str_caretaker_goal_hero_resolve",
      "label": "Raise vestal to Resolve Level 6",
      "source": "localization/miscellaneous.string_table.xml",
      "class_id": "vestal"
    }
  ]
}
"#)
        .map_err(|e| e.to_string())?;
    if c.goals.len() != 56
        || c.goals.iter().map(|g| &g.id).collect::<BTreeSet<_>>().len() != 56
        || c.goals.iter().filter(|g| g.plot.is_some()).count() != 37
        || c.goals
            .iter()
            .any(|g| g.plot.is_some() == g.class_id.is_some())
    {
        return Err("invalid native caretaker56 checklist".into());
    }
    Ok(c)
}
pub fn progress(o: &Observation) -> Result<Value> {
    let c = catalog()?;
    let mut achieved = 0;
    let mut main = 0;
    let goals = c
        .goals
        .iter()
        .map(|g| {
            let done = g.plot.as_ref().is_some_and(|p| {
                o.automation
                    .as_ref()
                    .is_some_and(|a| a.completed.contains(p))
            }) || g.class_id.as_ref().is_some_and(|id| {
                o.steward020
                    .as_ref()
                    .and_then(|s| s.classes.get(id))
                    .is_some_and(|c| c.earned)
            });
            achieved += usize::from(done);
            let primary = g
                .plot
                .as_deref()
                .is_some_and(|p| p != "plot_tutorial_crypts");
            main += usize::from(done && primary);
            json!({"id":g.id,"label":g.label,"localization_id":g.localization_id,"done":done})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"profile":c.version,"achieved":achieved,"required":56,"main_achieved":main,"main_required":36,"goals":goals}),
    )
}
pub fn is_goal_plot(id: &str) -> bool {
    catalog().is_ok_and(|c| c.goals.iter().any(|g| g.plot.as_deref() == Some(id)))
}
