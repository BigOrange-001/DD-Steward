//! Read-only dependency inventory. This never relaxes the execution loader.
use crate::{
    combat::{GameData, Result, text},
    darkest::Record,
    mechanics::read_buff,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct Report {
    pub content_id: String,
    pub semantics: &'static str,
    pub hero_classes: usize,
    pub monster_variants: usize,
    pub roots: Vec<String>,
    pub nodes: BTreeMap<String, Node>,
}
#[derive(Debug, Default, Serialize)]
pub struct Node {
    pub kind: String,
    pub sources: Vec<Source>,
    pub references: BTreeSet<String>,
    pub consumers: BTreeSet<String>,
    pub fields: BTreeMap<String, Assignment>,
    pub execution_check: Option<String>,
    pub reachable: bool,
    pub implementation: String,
    pub candidate: String,
    pub evidence: Vec<String>,
    pub conflicting_definitions: bool,
    pub entity_role: Option<String>,
}
#[derive(Debug, PartialEq, Serialize)]
pub struct Source {
    pub path: String,
    pub layers: Vec<String>,
    pub line: Option<u32>,
    pub definition: Value,
}
#[derive(Debug, Serialize)]
pub struct Assignment {
    pub task: u16,
    pub reason: &'static str,
}

fn assign(symbol: &str) -> Assignment {
    if [
        "immobilize",
        "unimmobilize",
        "performer_rank_target",
        "clear_rank_target",
        "steal_buff_source_type",
        "dotSource",
        "dotShuffle",
        "rank_damage_modifiers",
        "damage_heal_base_class_ids",
        "is_user_selected_targets",
        "can_be_damaged_directly",
        "disabled_act_out_combat_start_turn_types",
        "always_guard_actor_base_class_ids",
        "keep_rounds_in_ranks",
        "controller",
        "torchlight_modifier",
        "use_previous_monster_hp",
        "carry_over_hp_min_percent",
        "can_die_from_damage",
        "change_class_effects",
        "flashback",
        "actor_dot",
        "wave_infos",
        "killRange",
        "completion_chance",
        "duration_elements",
        "ignore_desires_on_single_valid_skill",
    ]
    .iter()
    .any(|k| symbol.contains(k))
    {
        return Assignment {
            task: 11,
            reason: "Boss lifecycle, explicit source/target contract or wave battle input",
        };
    }
    if [
        "trigger_limit",
        "quest_uses",
        "transform_on_trigger_limit_exhausted",
        "transform_on_wearer_killed",
        "transform_on_quest_complete",
        "destroy_on_triggers_exhausted",
        "blocks_other_slot",
        "is_in_inventory",
        "controlled_burn",
        "skill_instant",
        "steal_buff_stat_type",
        "copy_burn_behind",
        "dmg_per_burn_stack",
        "stress_on_miss",
        "evolution_",
        "perfectionist",
    ]
    .iter()
    .any(|key| symbol.contains(key))
    {
        return Assignment {
            task: 10,
            reason: "class, equipment trigger or explicit battle state",
        };
    }
    let (task, reason) = if [
        "summon",
        "spawn",
        "transform",
        "life_link",
        "revive",
        "number_of_turns",
        "initiative",
        "per_round_chance",
        "first_initiative",
        "next_initiative",
        "set_monster_class",
        "continue_turn",
        "per_turn_limit",
        "bonus_action",
        "wave_effects",
        "death_class",
        "life_time",
    ]
    .iter()
    .any(|s| symbol.contains(s))
    {
        (9, "action scheduling or entity lifecycle")
    } else if [
        "shared_health",
        "shape_shifter",
        "apply_with_result",
        "captor",
        "capture",
        "actor_dot",
        "control_count",
        "death_damage",
        "skill_reaction",
        "battle_backdrop",
        "boss",
    ]
    .iter()
    .any(|s| symbol.contains(s))
    {
        (11, "special encounter mechanics")
    } else if [
        "torch",
        "light",
        "disease",
        "quirk",
        "trinket",
        "vampire",
        "curse",
        "in_dungeon",
        "in_mode",
        "quest",
        "wave",
        "stall",
        "surprise",
        "valid_modes",
        "mode",
        "history",
        "uses_remaining",
        "corpse_enabled",
        "living_other_enemy_buffs",
        "living_hero_buff_instance_ids",
    ]
    .iter()
    .any(|s| symbol.contains(s))
    {
        (
            10,
            "build, class or external battle context; complete waves in 011",
        )
    } else if [
        "guard",
        "riposte",
        "buff",
        "target",
        "damage",
        "heal",
        "stun",
        "poison",
        "bleed",
        "burn",
        "dot",
        "chance",
        "resist",
        "rank",
        "stress",
        "hp",
        "mark",
        "actor",
        "skill",
        "cure",
        "stealth",
        "protection",
        "crit",
        "duration",
        "speed",
        "performer",
        "heroes",
        "monsters",
        "ally",
        "effect",
        "clear",
        "debuff",
        "tag",
    ]
    .iter()
    .any(|s| symbol.contains(s))
    {
        (
            8,
            "ordinary tactical rule; execution status is checked separately",
        )
    } else {
        (12, "explicit review in full closure; not claimed supported")
    };
    Assignment { task, reason }
}
fn record_value(r: &Record) -> Value {
    json!({"record":r.key,"params":r.params.iter().map(|p| json!({"name":p.name,"values":p.values.iter().map(|v| &v.text).collect::<Vec<_>>() })).collect::<Vec<_>>()})
}

pub fn inventory(g: &GameData) -> Result<Report> {
    let mut report = Report {
        content_id: g.overlay.config.id()?,
        semantics: "Schema/dependency inventory is not executable coverage. Unknown symbols remain explicit review items. Sources use only enabled overlay versions.",
        hero_classes: g.hero_ids().count(),
        monster_variants: g.monster_ids().count(),
        roots: vec![],
        nodes: BTreeMap::new(),
    };
    for hero in [true, false] {
        for path in g
            .overlay
            .combat_files(if hero { "heroes/" } else { "monsters/" }, ".info.darkest")
        {
            let parts = path.split('/').collect::<Vec<_>>();
            if parts.len() != if hero { 3 } else { 4 } {
                continue;
            }
            let id = parts[parts.len() - 2];
            if parts.last() != Some(&format!("{id}.info.darkest").as_str()) {
                continue;
            }
            let root = format!("{}:{id}", if hero { "hero" } else { "monster" });
            if !report.roots.contains(&root) {
                report.roots.push(root.clone());
            }
            for bytes in g.overlay.read_versions(&path)? {
                let d = crate::darkest::parse_str(
                    std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
                );
                if !d.warnings.is_empty() {
                    return Err(format!("{path}: {:?}", d.warnings));
                }
                for r in &d.records {
                    let node_id =
                        if ["combat_skill", "skill", "riposte_skill"].contains(&r.key.as_str()) {
                            format!(
                                "{}:{id}:{}:{}",
                                r.key,
                                text(r, "id")?,
                                text(r, "level").unwrap_or("monster")
                            )
                        } else {
                            root.clone()
                        };
                    if node_id != root {
                        report
                            .nodes
                            .entry(root.clone())
                            .or_default()
                            .references
                            .insert(node_id.clone());
                    }
                    let node = report.nodes.entry(node_id.clone()).or_default();
                    node.kind = if node_id == root {
                        if hero { "hero" } else { "monster" }
                    } else {
                        &r.key
                    }
                    .into();
                    source(node, g, &path, Some(r.line), record_value(r));
                    add_record(node, r);
                    if hero && r.key == "combat_skill" {
                        let level = text(r, "level")?
                            .parse()
                            .map_err(|_| "invalid skill level in inventory")?;
                        node.execution_check =
                            Some(match g.hero(id, 0, level, &[text(r, "id")?]) {
                                Ok(_) => "loadable".into(),
                                Err(e) => e,
                            });
                    }
                }
            }
            if !hero {
                let node = report.nodes.entry(root).or_default();
                node.execution_check = Some(match g.monster(id) {
                    Ok(b) => {
                        node.entity_role = Some(format!("{:?}", b.mechanics.lifecycle.role));
                        "loadable".into()
                    }
                    Err(e) => e,
                });
            }
        }
    }
    for path in g.overlay.combat_files("effects/", ".effects.darkest") {
        for bytes in g.overlay.read_versions(&path)? {
            let d =
                crate::darkest::parse_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?);
            if !d.warnings.is_empty() {
                return Err(format!("{path}: {:?}", d.warnings));
            }
            for r in d.records_keyed("effect") {
                let name = text(r, "name")?;
                let node = report.nodes.entry(format!("effect:{name}")).or_default();
                node.kind = "effect".into();
                source(node, g, &path, Some(r.line), record_value(r));
                add_record(node, r);
                node.execution_check = Some(match g.effect(name) {
                    Ok(_) => "schema supported; execution depends on referencing actor".into(),
                    Err(e) => e,
                });
            }
        }
    }
    for (prefix, suffix, array, kind) in [
        ("raid/actor_dot/", ".actor_dot.json", "entries", "actor_dot"),
        (
            "raid/flashback/",
            ".flashbacks.json",
            "flashbacks",
            "flashback",
        ),
        ("shared/buffs/", ".json", "buffs", "buff"),
        (
            "raid/ai/",
            ".monster_brains.json",
            "monster_brains",
            "brain",
        ),
        ("shared/trait/", "trait_library.json", "traits", "trait"),
        ("shared/quirk/", "quirk_library.json", "quirks", "quirk"),
        ("trinkets/", ".entries.trinkets.json", "entries", "trinket"),
        ("trinkets/", ".sets.trinkets.json", "sets", "trinket_set"),
    ] {
        for path in g.overlay.combat_files(prefix, suffix) {
            for bytes in g.overlay.read_versions(&path)? {
                let raw: Value =
                    serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))?;
                let values = raw[array]
                    .as_array()
                    .ok_or_else(|| format!("{path}: missing {array}"))?;
                for value in values {
                    let id = value["id"].as_str().ok_or("missing library id")?;
                    let node = report.nodes.entry(format!("{kind}:{id}")).or_default();
                    node.kind = kind.into();
                    source(node, g, &path, None, value.clone());
                    json_fields(node, kind, value);
                    if kind == "flashback" {
                        report.roots.push(format!("flashback:{id}"));
                        node.execution_check = Some(match g.flashback(id) {
                            Ok(_) => "schema supported".into(),
                            Err(e) => e,
                        });
                    }
                    if kind == "actor_dot" {
                        node.execution_check = Some(match g.effect("Parasite Egg Inject") {
                            Ok(_) => "schema supported".into(),
                            Err(e) => e,
                        });
                    }
                    if kind == "buff" {
                        node.execution_check = Some(match read_buff(&g.buffs, id) {
                            Ok(_) => "schema supported".into(),
                            Err(e) => e,
                        });
                    }
                    if ["quirk", "trinket", "trinket_set"].contains(&kind) {
                        report.roots.push(format!("{kind}:{id}"));
                    }
                    if kind == "trinket" {
                        node.execution_check = Some(match g.trinket_catalog(&[id.to_string()]) {
                            Ok(_) => "loadable".into(),
                            Err(e) => e,
                        });
                    }
                    if kind == "trait" {
                        report.roots.push(format!("trait:{id}"));
                    }
                }
            }
        }
    }
    for path in g
        .overlay
        .combat_files("inventory/", ".inventory.items.darkest")
    {
        for bytes in g.overlay.read_versions(&path)? {
            let d =
                crate::darkest::parse_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?);
            if !d.warnings.is_empty() {
                return Err(format!("{path}: {:?}", d.warnings));
            }
            for r in &d.records {
                if !["supply", "estate"].contains(&text(r, "type")?) {
                    continue;
                }
                let id = text(r, "id")?;
                let name = format!("supply:{id}");
                report.roots.push(name.clone());
                let node = report.nodes.entry(name).or_default();
                node.kind = "supply".into();
                source(node, g, &path, Some(r.line), record_value(r));
                add_record(node, r);
                node.execution_check = Some(match g.supply(id) {
                    Ok(supply) => {
                        for effect in supply
                            .effects
                            .iter()
                            .chain(&supply.semantics.sequential_effects)
                        {
                            node.references.insert(format!("effect:{}", effect.name));
                        }
                        "loadable".into()
                    }
                    Err(e) => e,
                });
            }
        }
    }
    for d in g.boss_encounters()? {
        let key = format!("encounter:{}", d.id);
        report.roots.push(key.clone());
        let node = report.nodes.entry(key).or_default();
        node.kind = "encounter".into();
        node.references
            .extend(d.monsters.iter().map(|id| format!("monster:{id}")));
        if let Some(id) = &d.flashback {
            node.references.insert(format!("flashback:{id}"));
        }
        for (path, line, params) in &d.conditions {
            source(node, g, path, Some(*line), json!(params));
            for k in params.keys() {
                node.fields.insert(format!("encounter.{k}"),Assignment{task:11,reason:"battle formation and host activation metadata; no encounter generation"});
            }
        }
        node.execution_check = Some(
            match g.entity_catalog(&d.monsters.iter().map(String::as_str).collect::<Vec<_>>()) {
                Ok(_) => "loadable".into(),
                Err(e) => e,
            },
        );
        node.evidence.push("tasks/011-boss-matrix-full.txt: actual original AI and per-action replay for three seeds".into());
    }
    if g.overlay
        .config
        .packages
        .iter()
        .any(|p| p == "color_of_madness")
    {
        for id in [
            "plot_color_of_madness_0",
            "plot_color_of_madness_1",
            "plot_color_of_madness_2",
        ] {
            let node = report.nodes.entry(format!("farm:{id}")).or_default();
            node.kind = "farm_battle".into();
            report.roots.push(format!("farm:{id}"));
            match g.farm_plan(id, Default::default()) {
                Ok(plan) => {
                    source(
                        node,
                        g,
                        &plan.source,
                        None,
                        serde_json::to_value(&plan).map_err(|e| e.to_string())?,
                    );
                    node.references
                        .extend(plan.monster_ids().iter().map(|id| format!("monster:{id}")));
                    node.execution_check = Some(match g.entity_catalog(&plan.monster_ids()) {
                        Ok(_) => "loadable".into(),
                        Err(e) => e,
                    });
                }
                Err(e) => node.execution_check = Some(e),
            }
            node.evidence.push("boss011_tests::installed_farm_all_intro_groups_recurring_space_lighting_reflection_and_boss_phases: lifecycle probe with host-restored heroes; farm-demo records are separate natural survival runs".into());
        }
    }
    let mut families: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, node) in &report.nodes {
        if node.kind == "monster" {
            for source in &node.sources {
                let parts = source.path.split('/').collect::<Vec<_>>();
                if parts.len() == 4 && parts[0] == "monsters" {
                    families
                        .entry(parts[1].into())
                        .or_default()
                        .push(id.clone());
                }
            }
        }
    }
    for (family, members) in families {
        let node = report
            .nodes
            .entry(format!("entity_family:{family}"))
            .or_default();
        node.kind = "entity_family".into();
        node.execution_check = Some("schema supported".into());
        node.references.extend(members);
    }
    report.roots.sort();
    report.roots.dedup();
    let mut pending = report.roots.clone();
    while let Some(id) = pending.pop() {
        let node = report.nodes.entry(id.clone()).or_default();
        if node.reachable {
            continue;
        }
        node.reachable = true;
        if node.kind.is_empty() {
            node.kind = "missing_reference".into();
            node.execution_check = Some("missing referenced definition".into());
        }
        pending.extend(node.references.iter().cloned());
    }
    let edges = report
        .nodes
        .iter()
        .flat_map(|(id, n)| n.references.iter().map(move |r| (id.clone(), r.clone())))
        .collect::<Vec<_>>();
    for (id, r) in edges {
        report.nodes.entry(r).or_default().consumers.insert(id);
    }
    let ambiguous = g
        .ambiguous_ids()
        .into_iter()
        .flat_map(|(kind, ids)| {
            let prefix = match kind {
                "effects" => "effect",
                "Buffs" => "buff",
                "Brains" => "brain",
                "traits" => "trait",
                "heroes" => "hero",
                "monsters" => "monster",
                _ => kind,
            };
            ids.into_iter().map(move |id| format!("{prefix}:{id}"))
        })
        .collect::<BTreeSet<_>>();
    for (id, n) in &mut report.nodes {
        n.conflicting_definitions = ambiguous.contains(id);
        n.implementation = match n.execution_check.as_deref() {
            Some(
                "loadable"
                | "schema supported"
                | "schema supported; execution depends on referencing actor",
            ) => "accepted by strict loader; runtime coverage requires evidence",
            Some(_) => "rejected by execution loader",
            None => "inventory only; no execution claim",
        }
        .into();
        n.candidate = "CALIBRATION.md: 001-011 candidates including Boss, captures, special objectives and original wave inputs; task assignment is planning, not support".into();
        if id.starts_with("monster:")
            && [
                "carrion_eater_big",
                "skeleton_captain",
                "gargoyle",
                "spider_spitter",
                "fungal_bloat",
                "fishman_crabby",
            ]
            .iter()
            .any(|family| id.starts_with(&format!("monster:{family}_")))
        {
            n.evidence.push(
                "crates/dd-core/tests/tactical_loading.rs: actual attack and snapshot roundtrip"
                    .into(),
            );
        }
        if id.starts_with("combat_skill:") {
            n.evidence.push("crates/dd-core/tests/heroes010.rs: all original levels execute and restore with complete fixed class kits".into());
        }
        if id.starts_with("trinket:") || id.starts_with("quirk:") {
            n.evidence.push("tasks/010-builds-full.json: definition preflight only; runtime cases in context010_tests.rs".into());
        }
        if id.starts_with("combat_skill:")
            && [
                "defender",
                "retribution",
                "duelist_advance",
                "guard_dog",
                "protect_me",
                "invigorating_vapours",
                "blindfire",
                "battlefield_medicine",
            ]
            .iter()
            .any(|skill| id.contains(&format!(":{skill}:")))
        {
            n.evidence.push("crates/dd-core/tests/tactical_loading.rs: all five original skill levels execute and replay".into());
        }
        if [
            "monster:ectoplasm_A",
            "monster:ectoplasm_large_A",
            "monster:templar_melee_mb_D",
            "monster:brigand_fusilier_B",
            "monster:corpse_A",
        ]
        .contains(&id.as_str())
        {
            n.evidence.push("crates/dd-core/tests/lifecycle_replay.rs: original spawn/zero/multiple/summon/wave definitions and per-step replay".into());
        }
    }
    Ok(report)
}
fn source(node: &mut Node, g: &GameData, path: &str, line: Option<u32>, definition: Value) {
    let source = Source {
        path: path.into(),
        layers: g
            .overlay
            .sources(path)
            .iter()
            .map(|i| g.overlay.layers[*i].id.clone())
            .collect(),
        line,
        definition,
    };
    if !node.sources.contains(&source) {
        node.sources.push(source);
    }
}
fn add_record(node: &mut Node, r: &Record) {
    node.fields
        .insert(format!("record:{}", r.key), assign(&r.key));
    for p in &r.params {
        node.fields
            .insert(format!("{}.{}", r.key, p.name), assign(&p.name));
        let kind = match p.name.as_str() {
            "effect"
            | "effects"
            | "wave_effects"
            | "battle_start_effect_ids"
            | "enter_effects"
            | "change_class_effects"
            | "captured_effects"
            | "released_effects"
            | "performer_effects"
            | "target_effects"
            | "was_hit_performer_effects"
            | "was_killed_other_monsters_effects"
            | "was_killed_by_hero_effects"
            | "capture_effects"
            | "release_effects"
            | "sequential_use_effects" => Some("effect"),
            "affliction_combat_skill_id" | "battle_complete_combat_skill_id" => None,
            "buff_ids"
            | "buffs"
            | "recovery_buffs"
            | "heart_attack_buffs"
            | "recovery_heart_attack_buffs"
            | "living_hero_buff_instance_ids"
            | "living_other_enemy_buffs" => Some("buff"),
            "monster_class_id"
            | "monster_class_ids"
            | "summon_monster_class_id"
            | "summon_monster_class_ids"
            | "summon_monsters"
            | "set_monster_class_id"
            | "set_monster_class_ids"
            | "random_monster_class_ids" => Some("monster"),
            "actor_dot" => Some("actor_dot"),
            "captor_full_class"
            | "captor_empty_class"
            | "captor_full_monster_class"
            | "captor_empty_monster_class" => Some("monster"),
            "monster_class" if r.key == "companion" => Some("entity_family"),
            "base_class" if r.key == "life_link" => Some("entity_family"),
            "target_base_class_id"
            | "performing_monster_captor_base_class"
            | "always_guard_actor_base_class_ids"
            | "spawn_target_actor_base_class_id" => Some("entity_family"),
            "id" if r.key == "monster_brain" => Some("brain"),
            _ => None,
        };
        if let Some(kind) = kind {
            for v in &p.values {
                if !v.text.is_empty() {
                    node.references.insert(format!("{kind}:{}", v.text));
                }
            }
        }
    }
}
fn json_fields(node: &mut Node, prefix: &str, value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                node.fields.insert(format!("{prefix}.{key}"), assign(key));
                if ["stat_type", "rule_type", "type"].contains(&key.as_str())
                    && let Some(s) = v.as_str()
                {
                    node.fields.insert(format!("{prefix}.{key}={s}"), assign(s));
                }
                if let Some(id) = v.as_str().filter(|s| !s.is_empty()) {
                    let kind = match key.as_str() {
                        "set_id" => Some("trinket_set"),
                        "transform_on_trigger_limit_exhausted"
                        | "transform_on_wearer_killed"
                        | "transform_on_quest_complete" => Some("trinket"),
                        "evolution_class_id" | "change_quirk_class_id" => Some("quirk"),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        node.references.insert(format!("{kind}:{id}"));
                    }
                }
                if (key.ends_with("additional_effects") || key == "attack_additional_effect_ids")
                    && let Some(values) = v.as_array()
                {
                    for id in values.iter().filter_map(Value::as_str) {
                        node.references.insert(format!("effect:{id}"));
                    }
                }
                if ["effect", "string_value"].contains(&key.as_str())
                    && let Some(s) = v.as_str()
                    && !s.is_empty()
                {
                    node.references.insert(format!("effect:{s}"));
                }
                if [
                    "required_heroes_starting_effects",
                    "completion_effects",
                    "increment_effects",
                ]
                .contains(&key.as_str())
                    && let Some(values) = v.as_array()
                {
                    for id in values.iter().filter_map(Value::as_str) {
                        node.references.insert(format!("effect:{id}"));
                    }
                }
                if [
                    "buff_ids",
                    "buffs",
                    "hero_buffs",
                    "monster_buffs",
                    "completed_camping_start_buffs",
                ]
                .contains(&key.as_str())
                    && let Some(values) = v.as_array()
                {
                    for id in values.iter().filter_map(Value::as_str) {
                        node.references.insert(format!("buff:{id}"));
                    }
                }
                json_fields(node, &format!("{prefix}.{key}"), v);
            }
        }
        Value::Array(values) => {
            for v in values {
                json_fields(node, prefix, v)
            }
        }
        _ => {}
    }
}

