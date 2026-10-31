//! Resolve the historical planning inventory by actual use and strict loaders.
//! A label never converts a failed battle dependency into executable coverage.
use crate::{
    build010::Build,
    combat::{GameData, Result},
    mechanics::CombatGlobals,
    support,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct ClosureReport {
    pub content_id: String,
    pub candidate_id: String,
    pub hero_classes: usize,
    pub monster_variants: usize,
    pub counts: BTreeMap<String, usize>,
    pub unresolved: Vec<String>,
    pub rows: BTreeMap<String, ClosureRow>,
    pub global_rules: BTreeMap<String, crate::rules_inventory012::RuleUse>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ClosureRow {
    pub kind: String,
    pub scope: String,
    pub binding: String,
    pub sources: Vec<String>,
    pub references: Vec<String>,
    pub fields: Vec<String>,
    pub check: Option<String>,
}
fn accepted(check: Option<&str>) -> bool {
    matches!(
        check,
        Some(
            "loadable"
                | "schema supported"
                | "schema supported; execution depends on referencing actor"
        )
    )
}
pub fn inventory(g: &GameData) -> Result<ClosureReport> {
    let report = support::inventory(g)?;
    let quirks = g.quirks()?;
    let diseases = g.disease_catalog()?;
    let raw_rules: serde_json::Value =
        serde_json::from_slice(&g.overlay.read("shared/rules.json")?).map_err(|e| e.to_string())?;
    let trait_check = CombatGlobals::load_explicit(
        &raw_rules,
        &g.buffs,
        &serde_json::json!({"traits":g.traits.values().collect::<Vec<_>>()}),
    )
    .map(|_| ());
    let base = g.hero("highwayman", 0, 0, &["wicked_slice"])?;
    let mut rows = BTreeMap::new();
    for (key, node) in &report.nodes {
        let mut check = node.execution_check.clone();
        let mut scope = "battle";
        let mut binding=match node.kind.as_str() {
            "buff"=>"mechanics::Buff + systems::modifier + source-specific activation",
            "effect"=>"GameData::effect + systems/tactical/lifecycle + finite source effect queue",
            "monster"=>"GameData::monster + entity_catalog + source role lifecycle",
            "hero"|"combat_skill"=>"GameData::hero + execute_skill + class state",
            "skill"|"riposte_skill"=>"strict parent entity skill interpreter",
            "brain"=>"Brain::validate + two-stage desire interpreter + source cooldowns",
            "quirk"=>"equip_build/disease_catalog + passive and explicit Fated rules; external acquisition metadata",
            "trinket"|"trinket_set"=>"trinket_catalog/equip_build + triggered effect and budget interpreter",
            "trait"=>"CombatGlobals + turn/reaction/resolve interpreter",
            "supply"=>"supply source definition + consume_supply; no purchasing",
            "encounter"=>"original battle formation + host activation metadata; no encounter generation",
            "farm_battle"=>"original farm_plan + continuous wave controller",
            "flashback"=>"original flashback + protected Horror and result controller",
            "actor_dot"=>"actor_dot source + after-attack parasite stages",
            "entity_family"=>"original path family membership + entity_catalog + life_link/guard/summon family matching",
            _=>"unclassified dependency",
        }.to_string();
        if !node.reachable {
            scope = "unreferenced_library";
            binding = "no enabled battle reference; no execution claim".into();
        } else if node.kind == "buff" && g.noncombat_buff(key.trim_start_matches("buff:"))? {
            scope = "external";
            binding="source stat/rule denotes town/camp/exploration/reward; retained in host noncombat_buffs".into();
        } else if node.kind == "effect" && key.starts_with("effect:On Quest Complete ") {
            // Verify use, not just the name: each consumer must retain this exact ID
            // in its quest metadata and must not execute it as a combat trigger.
            let id = key.trim_start_matches("effect:");
            let external = !node.consumers.is_empty()
                && node.consumers.iter().all(|consumer| {
                    let Some(tid) = consumer.strip_prefix("trinket:") else {
                        return false;
                    };
                    let Ok(catalog) = g.trinket_catalog(&[tid.into()]) else {
                        return false;
                    };
                    let Some(t) = catalog.get(tid) else {
                        return false;
                    };
                    t.quest_metadata.to_string().contains(id)
                        && !t.effects.values().flatten().any(|e| e.name == id)
                });
            if external {
                scope = "external";
                binding =
                    "verified quest-completion metadata; acquisition/town event belongs to host"
                        .into();
            }
        } else if node.kind == "effect"
            && node.sources.iter().any(|s| {
                s.definition
                    .get("params")
                    .and_then(|p| p.as_array())
                    .is_some_and(|ps| {
                        ps.iter()
                            .any(|p| p["name"] == "spawn_target_actor_base_class_id")
                    })
            })
        {
            let id = key.trim_start_matches("effect:");
            let contextual = !node.consumers.is_empty()
                && node.consumers.iter().all(|consumer| {
                    let Some(mid) = consumer.strip_prefix("monster:") else {
                        return false;
                    };
                    g.monster(mid).is_ok_and(|b| {
                        b.mechanics
                            .lifecycle
                            .wave_spawn
                            .iter()
                            .any(|e| e.effect.name == id && e.target_family.is_some())
                    })
                });
            if contextual {
                check = Some("loadable in source wave-spawn context".into());
                scope = "contextual_battle";
                binding = "lifecycle009::wave_spawn target_family + core::apply_wave_spawn".into();
            }
        } else if node.kind == "supply" && key == "supply:torch" {
            let tested = g.supply_with_light_candidate(
                "torch",
                crate::items::HostItemCandidate {
                    light_amount: 25,
                    source: "https://darkestdungeon-archive.fandom.com/wiki/Torch".into(),
                },
            );
            tested?;
            scope = "explicit_host_input";
            binding="validated HostItemCandidate required; bloodlight has a separate original-data item override".into();
        } else if node.kind == "supply" {
            let def = g.supply(key.trim_start_matches("supply:"))?;
            if def.effects.is_empty() && def.semantics.sequential_effects.is_empty() {
                scope = "external";
                binding =
                    "source inventory with no battle-use effect; no legal battle action".into();
            }
        } else if node.kind == "hero" {
            let id = key.trim_start_matches("hero:");
            let skills = g.hero_skill_ids(id)?;
            check = Some(
                match g.hero(id, 0, 0, &[skills.first().ok_or("hero has no skills")?]) {
                    Ok(_) => "loadable".into(),
                    Err(e) => e,
                },
            );
        } else if node.kind == "quirk" {
            let id = key.trim_start_matches("quirk:");
            let raw = crate::library::unique_json(&quirks, id, "quirk")?;
            let mut build = Build::default();
            if raw["is_disease"] == true {
                build.diseases.push(id.into());
                if let Some(e) = diseases.get(id).and_then(|d| d.evolution.as_ref()) {
                    build.initial_disease_turns.insert(id.into(), e.min);
                }
            } else {
                build.quirks.push(id.into());
            }
            check = Some(match g.equip_build(base.clone(), build) {
                Ok(_) => "loadable".into(),
                Err(e) => e,
            });
        } else if node.kind == "trinket_set" {
            let raw = &node.sources.first().ok_or("missing set source")?.definition;
            let id = key.trim_start_matches("trinket_set:");
            if raw
                .as_object()
                .is_none_or(|o| o.keys().any(|k| !["id", "buffs"].contains(&k.as_str())))
                || raw["id"] != id
            {
                check = Some("unknown trinket set source schema".into());
            } else {
                let result = raw["buffs"]
                    .as_array()
                    .ok_or("missing set Buffs")?
                    .iter()
                    .try_for_each(|bid| -> Result<()> {
                        let id = bid.as_str().ok_or("invalid set Buff ID")?;
                        if !g.noncombat_buff(id)? {
                            crate::mechanics::read_buff(&g.buffs, id)?;
                        }
                        Ok(())
                    });
                check = Some(match result {
                    Ok(_) => "schema supported".into(),
                    Err(e) => e,
                });
            }
        } else if node.kind == "trait" {
            check = Some(match &trait_check {
                Ok(_) => "schema supported".into(),
                Err(e) => e.clone(),
            });
        } else if node.kind == "trinket" {
            let raw = &node
                .sources
                .first()
                .ok_or("missing trinket source")?
                .definition;
            let required = raw["hero_class_requirements"]
                .as_array()
                .ok_or("missing trinket requirements")?;
            if !required.is_empty()
                && required
                    .iter()
                    .all(|v| v.as_str().is_some_and(|id| !g.hero_ids().any(|h| h == id)))
            {
                let known = required.iter().all(|v| {
                    v.as_str().is_some_and(|id| {
                        [
                            "musketeer",
                            "flagellant",
                            "shieldbreaker",
                            "runaway",
                            "duelist",
                        ]
                        .contains(&id)
                    })
                });
                if known {
                    scope = "disabled_actor";
                    binding="original explicit class requirement refers only to excluded/disabled actor content; never equipped".into();
                }
            }
        }
        let validated = matches!(
            scope,
            "external"
                | "unreferenced_library"
                | "explicit_host_input"
                | "disabled_actor"
                | "contextual_battle"
        ) || accepted(check.as_deref());
        if !validated && check.is_some() {
            scope = "unresolved";
        }
        if !validated && check.is_none() {
            scope = "parent_dependency";
        }
        if binding == "unclassified dependency" {
            scope = "unresolved";
        }
        rows.insert(
            key.clone(),
            ClosureRow {
                kind: node.kind.clone(),
                scope: scope.into(),
                binding,
                sources: node
                    .sources
                    .iter()
                    .map(|s| match s.line {
                        Some(n) => format!("{}:{n}", s.path),
                        None => s.path.clone(),
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                references: node.references.iter().cloned().collect(),
                fields: node.fields.keys().cloned().collect(),
                check,
            },
        );
    }
    // Parent-only skill/set nodes are accepted only when an actually checked
    // consumer closes over them. Missing or cyclic orphan roots stay unresolved.
    for _ in 0..rows.len() {
        let ready = rows
            .iter()
            .filter(|(_, r)| r.scope == "parent_dependency")
            .filter_map(|(id, _)| {
                report.nodes[id]
                    .consumers
                    .iter()
                    .any(|parent| {
                        rows.get(parent).is_some_and(|p| {
                            matches!(p.scope.as_str(), "battle" | "contextual_battle")
                        })
                    })
                    .then_some(id.clone())
            })
            .collect::<Vec<_>>();
        if ready.is_empty() {
            break;
        }
        for id in ready {
            rows.get_mut(&id).unwrap().scope = "battle".into();
        }
    }
    let mut unresolved = vec![];
    let mut counts = BTreeMap::new();
    for (id, row) in &mut rows {
        if row.scope == "parent_dependency" {
            row.scope = "unresolved".into();
        }
        if row.scope == "unresolved" {
            unresolved.push(id.clone());
        }
        *counts.entry(row.scope.clone()).or_insert(0) += 1;
    }
    let global_rules = crate::rules_inventory012::inventory(&raw_rules, &g.rules)?;
    for (id, row) in &global_rules {
        if row.scope == "unresolved" {
            unresolved.push(format!("global_rule:{id}"));
        }
    }
    Ok(ClosureReport {
        global_rules,
        content_id: report.content_id,
        candidate_id: g.rules.candidates.identity()?,
        hero_classes: report.hero_classes,
        monster_variants: report.monster_variants,
        counts,
        unresolved,
        rows,
    })
}
