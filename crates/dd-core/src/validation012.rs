//! Reproducible host acceptance probes. Controlled skill probes are distinct
//! from the natural Boss/Farmstead scenarios; no policy receives the registry.
use super::*;
use dd_data::{
    combat::GameData,
    content::{ContentConfig, PACKAGES},
};

pub fn configurations() -> Vec<ContentConfig> {
    let mut result = vec![];
    for mask in 0..(1usize << (PACKAGES.len() - 1)) {
        for mode in ["base", "radiant", "new_game_plus", "bloodmoon"] {
            for never in ["strict", "permissive"] {
                let mut c = ContentConfig::fixture();
                for (bit, (id, _)) in PACKAGES.iter().skip(1).enumerate() {
                    if mask & (1 << bit) != 0 {
                        c.packages.push((*id).into());
                    }
                }
                c.mode = mode.into();
                c.never_again = never.into();
                if c.validate().is_ok() {
                    result.push(
                        c.with_enabled_combat_candidates()
                            .expect("validated enabled candidates"),
                    );
                }
            }
        }
    }
    result
}
#[derive(Debug, Clone, Serialize)]
pub struct ConfigCase {
    pub content_id: String,
    pub candidate_id: String,
    pub packages: Vec<String>,
    pub mode: String,
    pub never_again: String,
    pub original_ngp_flag: bool,
    pub heroes: Vec<String>,
    pub monsters: Vec<String>,
    pub level_zero_skills: usize,
    pub catalog_definitions: usize,
    pub actions: usize,
    pub outcomes: Vec<Outcome>,
    pub errors: Vec<String>,
}
pub fn configuration_case(g: &GameData) -> Result<ConfigCase> {
    let mut row = ConfigCase {
        content_id: g.rules.content.id()?,
        candidate_id: g.rules.candidates.identity()?,
        packages: g.rules.content.packages.clone(),
        mode: g.rules.content.mode.clone(),
        never_again: g.rules.content.never_again.clone(),
        original_ngp_flag: g.overlay.is_new_game_plus,
        heroes: g.hero_ids().cloned().collect(),
        monsters: g.monster_ids().cloned().collect(),
        level_zero_skills: 0,
        catalog_definitions: 0,
        actions: 0,
        outcomes: vec![],
        errors: vec![],
    };
    if let Some(error) = &g.combat_globals_error {
        row.errors.push(format!("global traits: {error}"));
    }
    for class in &row.heroes {
        for id in g.hero_skill_ids(class)? {
            match g.hero(class, 0, 0, &[&id]) {
                Ok(_) => row.level_zero_skills += 1,
                Err(e) => row.errors.push(format!("{class}.{id}: {e}")),
            }
        }
    }
    match g.entity_catalog(&row.monsters.iter().map(String::as_str).collect::<Vec<_>>()) {
        Ok(c) => row.catalog_definitions = c.len(),
        Err(e) => {
            row.errors.push(format!("entity dependency closure: {e}"));
            for id in &row.monsters {
                if let Err(e) = g.monster(id) {
                    row.errors.push(format!("{id}: {e}"));
                }
            }
        }
    }
    // A short ordinary encounter actually runs every valid package/mode/option
    // combination; definition and dependency checks above cover all its roots.
    for seed in [1, 17] {
        let mut rules = g.rules.clone();
        rules.context =
            Some(g.battle_context("crypts".into(), dd_data::build010::Surprise::Neither, false)?);
        rules.lighting = Some(g.lighting(if seed == 1 { 0 } else { 100 })?);
        let s = Scenario {
            heroes: vec![
                g.hero("highwayman", 0, 0, &["wicked_slice", "opened_vein"])?,
                g.hero("vestal", 0, 0, &["divine_grace"])?,
                g.hero("crusader", 0, 0, &["smite"])?,
                g.hero("houndmaster", 0, 0, &["hounds_rush"])?,
            ],
            monsters: vec![
                g.monster("skeleton_common_A")?,
                g.monster("skeleton_courtier_A")?,
            ],
            rules,
            round_limit: 40,
        };
        let mut b = Battle::new(s, seed)?;
        for _ in 0..1000 {
            if let Some(o) = b.outcome() {
                row.outcomes.push(o);
                break;
            }
            let mut restored = Battle::load_json_with_candidates(
                &b.save_json()?,
                &g.rules.content,
                &g.rules.candidates,
            )?;
            let o = b.observation();
            let events = if o.units.iter().any(|u| Some(u.id) == o.actor && u.hero) {
                let a = Baseline.choose(&o).ok_or("no acceptance policy action")?;
                let e = b.step(a.clone())?;
                if e != restored.step(a)? {
                    return Err("configuration action events diverged".into());
                }
                e
            } else {
                let e = b.step_monster()?;
                if e != restored.step_monster()? {
                    return Err("configuration NPC events diverged".into());
                }
                e
            };
            let _ = events;
            if b != restored
                || b.observation() != restored.observation()
                || b.legal_actions() != restored.legal_actions()
            {
                return Err("configuration state/observation/actions diverged".into());
            }
            row.actions += 1;
        }
    }
    if row.outcomes.len() != 2 {
        row.errors.push("configuration battle action bound".into());
    }
    Ok(row)
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillProbe {
    pub class: String,
    pub skill: String,
    pub level: i32,
    pub actions: usize,
    pub event_count: usize,
}
pub fn skill_probe(g: &GameData, class: &str, level: i32, skill: &str) -> Result<SkillProbe> {
    skill_probe_with_build(g, class, level, skill, None)
}
fn skill_probe_with_build(
    g: &GameData,
    class: &str,
    level: i32,
    skill: &str,
    build: Option<dd_data::build010::Build>,
) -> Result<SkillProbe> {
    let mut hero = g.hero(class, level as usize, level, &[skill])?;
    if !hero.mechanics.hero.fixed_skills.is_empty() {
        hero = g.hero(
            class,
            level as usize,
            level,
            &hero
                .mechanics
                .hero
                .fixed_skills
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
    }
    if let Some(build) = build {
        hero = g.equip_build(hero, build)?;
    }
    let slot = hero
        .skills
        .iter()
        .position(|s| s.id == skill)
        .ok_or("probe skill missing")?;
    let spec = hero.skills[slot].tactical.clone();
    if let Some(mode) = spec.valid_modes.first() {
        hero.mechanics.hero.initial_mode = Some(mode.clone());
    }
    if let Some(range) = spec.hp_range {
        hero.mechanics.hero.initial_hp = Some(
            (f64::from(hero.stats.hp) * (range[0] + range[1]) / 2.)
                .round()
                .max(1.) as i32,
        );
    }
    let actor = usize::from(
        *hero.skills[slot]
            .launch
            .iter()
            .min()
            .ok_or("missing probe launch ranks")?
            - 1,
    );
    let mut heroes = vec![g.hero("vestal", level as usize, level, &["divine_grace"])?; 4];
    heroes[actor] = hero;
    let mut rules = g.rules.clone();
    rules.context =
        Some(g.battle_context("crypts".into(), dd_data::build010::Surprise::Neither, false)?);
    rules.lighting = Some(g.lighting(100)?);
    let mut b = Battle::new(
        Scenario {
            heroes,
            monsters: vec![g.monster("skeleton_common_A")?; 4],
            rules,
            round_limit: 100,
        },
        1,
    )?;
    let mut actions = 0;
    while b.outcome().is_none() && b.actor != Some(actor) {
        if b.actor.is_some_and(|id| !b.units[id].friendly()) {
            b.step_monster()?;
        } else {
            b.step(Action::Pass)?;
        }
        actions += 1;
        if actions > 1000 {
            return Err("probe never reached actor".into());
        }
    }
    if spec.requires_burning {
        b.apply_encounter_effect(actor, 4, g.effect("RW Burn 2")?)?;
    }
    let action=b.observation().actions.iter().find(|a|matches!(a.action,Action::Skill {skill,..}|Action::SkillAuto {skill} if skill==slot))
        .map(|a|a.action.clone()).ok_or_else(||format!("no legal probe action {class}.{skill}:{level}"))?;
    let mut restored =
        Battle::load_json_with_candidates(&b.save_json()?, &g.rules.content, &g.rules.candidates)?;
    let events = b.step(action.clone())?;
    if events != restored.step(action)?
        || b != restored
        || b.observation() != restored.observation()
    {
        return Err("skill probe restore diverged".into());
    }
    b.save_json()?;
    Ok(SkillProbe {
        class: class.into(),
        skill: skill.into(),
        level,
        actions: actions + 1,
        event_count: events.len(),
    })
}

/// Each enabled build definition is actually equipped and used in one restored
/// action. This checks binding, not that every probabilistic trigger has fired.
pub fn build_matrix(g: &GameData) -> Result<Vec<serde_json::Value>> {
    use dd_data::{build010::Build, library};
    let entries =
        library::json_library(&g.overlay, "trinkets/", ".entries.trinkets.json", "entries")?;
    let mut rows = vec![];
    let mut sets = BTreeMap::<String, Vec<(String, String)>>::new();
    let run = |class: &str, build: Build| -> Result<SkillProbe> {
        let skill = g
            .hero_skill_ids(class)?
            .into_iter()
            .next()
            .ok_or("class has no skills")?;
        skill_probe_with_build(g, class, 4, &skill, Some(build))
    };
    for (id, raw) in &entries {
        let requirements = raw["hero_class_requirements"]
            .as_array()
            .ok_or("missing trinket requirements")?;
        let class = if requirements.is_empty() {
            Some("highwayman")
        } else {
            requirements
                .iter()
                .filter_map(|v| v.as_str())
                .find(|c| g.hero_ids().any(|id| id == c))
        };
        let Some(class) = class else {
            rows.push(serde_json::json!({"kind":"trinket","id":id,"scope":"disabled_actor","requirements":requirements}));
            continue;
        };
        let result = run(
            class,
            Build {
                trinkets: vec![id.clone()],
                ..Default::default()
            },
        );
        rows.push(match result {
            Ok(probe) => serde_json::json!({"kind":"trinket","id":id,"class":class,"probe":probe}),
            Err(error) => serde_json::json!({"kind":"trinket","id":id,"class":class,"error":error}),
        });
        if let Some(set) = raw["set_id"].as_str().filter(|s| !s.is_empty()) {
            sets.entry(set.into())
                .or_default()
                .push((id.clone(), class.into()));
        }
    }
    for (id, members) in sets {
        let Some(pair) = members.iter().enumerate().find_map(|(n, a)| {
            members
                .iter()
                .skip(n + 1)
                .find(|b| b.1 == a.1)
                .map(|b| (a, b))
        }) else {
            return Err(format!("set {id} has no enabled legal pair"));
        };
        let result = run(
            &pair.0.1,
            Build {
                trinkets: vec![pair.0.0.clone(), pair.1.0.clone()],
                ..Default::default()
            },
        );
        rows.push(match result {
            Ok(probe) => serde_json::json!({"kind":"trinket_set","id":id,"probe":probe}),
            Err(error) => serde_json::json!({"kind":"trinket_set","id":id,"error":error}),
        });
    }
    for (id, raw) in g.quirks()? {
        let disease = raw["is_disease"]
            .as_bool()
            .ok_or("missing quirk category")?;
        let mut build = if disease {
            Build {
                diseases: vec![id.clone()],
                ..Default::default()
            }
        } else {
            Build {
                quirks: vec![id.clone()],
                ..Default::default()
            }
        };
        if disease && let Some(evolution) = g.disease_catalog()?[&id].evolution.as_ref() {
            build
                .initial_disease_turns
                .insert(id.clone(), evolution.min);
        }
        let result = run("highwayman", build);
        rows.push(match result {
            Ok(probe) => {
                serde_json::json!({"kind":if disease{"disease"}else{"quirk"},"id":id,"probe":probe})
            }
            Err(error) => {
                serde_json::json!({"kind":if disease{"disease"}else{"quirk"},"id":id,"error":error})
            }
        });
    }
    Ok(rows)
}

/// Controlled turn/death probes for the complement of the source Boss tag. Heroes yield
/// real opportunities until the original NPC acts; no Brain/stat substitution.
pub fn remaining_unit_matrix(g: &GameData) -> Result<Vec<serde_json::Value>> {
    let mut bosses = std::collections::BTreeSet::new();
    for path in g.overlay.combat_files("monsters/", ".info.darkest") {
        let d = dd_data::library::read_document(&g.overlay, &path)?;
        if d.records_keyed("tag")
            .any(|r| dd_data::combat::text(r, "id") == Ok("boss"))
        {
            bosses.insert(
                path.split('/')
                    .nth(2)
                    .ok_or("invalid source path")?
                    .to_string(),
            );
        }
    }
    let mut rows = vec![];
    for id in g.monster_ids().filter(|id| !bosses.contains(*id)) {
        let result = (|| -> Result<serde_json::Value> {
            let root = g.monster(id)?;
            let mut ids = vec![id.clone()];
            if let Some(family) = &root.mechanics.life_link
                && !root.mechanics.lifecycle.targetable
            {
                let members = g
                    .monster_ids()
                    .map(|id| g.monster(id))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .filter(|u| &u.mechanics.family == family && u.size + root.size <= 4)
                    .collect::<Vec<_>>();
                if members.len() != 1 {
                    return Err(format!(
                        "probe requires explicit linked family formation: {family}"
                    ));
                }
                ids.push(members[0].id.clone());
            }
            let mut b = boss_battle(
                g,
                &ids.iter().map(String::as_str).collect::<Vec<_>>(),
                "crypts",
                17,
            )?;
            let zero = b.units[4].blueprint.mechanics.lifecycle.turns == 0;
            let mut acted = false;
            let mut actions = 0;
            for _ in 0..2000 {
                if b.outcome().is_some() {
                    break;
                }
                let mut restored =
                    Battle::load_json_with_content(&b.save_json()?, &g.rules.content)?;
                let events = if !b.observation().player_selection
                    && b.actor.is_some_and(|a| !b.units[a].friendly())
                {
                    acted |= b.actor == Some(4);
                    let events = b.step_monster()?;
                    if events != restored.step_monster()? {
                        return Err("NPC probe event replay mismatch".into());
                    }
                    events
                } else {
                    let action = if !acted && !zero && !b.observation().player_selection {
                        Action::Pass
                    } else {
                        Baseline
                            .choose(&b.observation())
                            .ok_or("missing NPC probe player action")?
                    };
                    let events = b.step(action.clone())?;
                    if events != restored.step(action)? {
                        return Err("NPC probe player event replay mismatch".into());
                    }
                    events
                };
                let _ = events;
                if b != restored || b.observation() != restored.observation() {
                    return Err("NPC probe state replay mismatch".into());
                }
                actions += 1;
            }
            if b.outcome().is_none() || (!zero && !acted) {
                return Err("NPC did not execute original opportunity/finish".into());
            }
            Ok(
                serde_json::json!({"id":id,"role":if zero{"zero-action component"}else{"original NPC turn"},"npc_acted":acted,"formation":ids,"actions":actions,"outcome":b.outcome()}),
            )
        })();
        rows.push(match result {
            Ok(row) => row,
            Err(error) => serde_json::json!({"id":id,"error":error}),
        });
    }
    Ok(rows)
}
