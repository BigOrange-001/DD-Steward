//! Explicit, versioned content selection. No dependency is enabled implicitly.
use crate::combat::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

pub const PACKAGES: [(&str, &[&str]); 9] = [
    ("base", &[""]),
    (
        "crimson_court",
        &["dlc/580100_crimson_court/features/crimson_court"],
    ),
    (
        "districts",
        &["dlc/580100_crimson_court/features/districts"],
    ),
    (
        "flagellant",
        &["dlc/580100_crimson_court/features/flagellant"],
    ),
    ("color_of_madness", &["dlc/735730_color_of_madness"]),
    ("shieldbreaker", &["dlc/702540_shieldbreaker"]),
    (
        "fires_edge",
        &[
            "dlc/4964110_fires_edge",
            "dlc/4964110_fires_edge/features/fires_edge",
        ],
    ),
    ("duelist", &["dlc/4964110_fires_edge/features/duelist"]),
    ("runaway", &["dlc/4964110_fires_edge/features/runaway"]),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentConfig {
    pub packages: Vec<String>,
    pub mode: String,
    pub never_again: String,
    pub no_light: bool,
    #[serde(default)]
    pub effect_choices: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub syntax_choices: std::collections::BTreeMap<String, String>,
}
impl ContentConfig {
    pub fn fixture() -> Self {
        Self {
            packages: vec!["base".into()],
            mode: "base".into(),
            never_again: "strict".into(),
            no_light: false,
            effect_choices: Default::default(),
            syntax_choices: Default::default(),
        }
    }
    pub fn full() -> Self {
        Self {
            packages: PACKAGES.iter().map(|(id, _)| (*id).into()).collect(),
            mode: "radiant".into(),
            ..Self::fixture()
        }
    }
    pub fn with_boss_effect_candidates(self) -> Result<Self> {
        if !self.packages.iter().any(|p| p == "crimson_court") {
            return Err("boss effect choices require crimson_court".into());
        }
        self.with_enabled_boss_candidates()
    }
    /// Explicit source ambiguities for exactly the enabled packages; never adds dependencies.
    fn with_enabled_boss_candidates(mut self) -> Result<Self> {
        if self.packages.iter().any(|p| p == "crimson_court") {
            self.effect_choices
                .insert("Burn Hero 1".into(), "capture".into());
            self.effect_choices
                .insert("Inject Self Damage".into(), "performer".into());
            self.syntax_choices
                .insert("baron_rank_zero".into(), "auto".into());
        }
        if self
            .packages
            .iter()
            .any(|p| ["crimson_court", "color_of_madness"].contains(&p.as_str()))
        {
            self.syntax_choices
                .insert("boss_inactive_refs".into(), "retain".into());
        }
        if self.packages.iter().any(|p| p == "color_of_madness") {
            self.syntax_choices
                .insert("spire_target_backtick".into(), "strip".into());
            self.syntax_choices
                .insert("farm_croc_skill_prefix".into(), "com".into());
        }
        self.validate()?;
        Ok(self)
    }
    /// Full closure candidates, including the two explicit original trinket ambiguities.
    pub fn with_enabled_combat_candidates(self) -> Result<Self> {
        let mut selected = self.with_enabled_boss_candidates()?;
        selected
            .syntax_choices
            .insert("difficulty_effect_source".into(), "mode_file".into());
        selected
            .syntax_choices
            .insert("legacy_global_boundary".into(), "outside_combat_v1".into());
        if selected.packages.iter().any(|p| p == "color_of_madness") {
            selected
                .syntax_choices
                .insert("smoking_skull_conflict".into(), "poison_resistance".into());
            selected.effect_choices.insert(
                "Thirsting Blade Self Atk On Miss".into(),
                "performer".into(),
            );
        }
        selected.validate()?;
        Ok(selected)
    }
    /// Explicit 018 source ambiguity. Other presets preserve strict unresolved behavior.
    pub fn with_campaign018_candidates(self) -> Result<Self> {
        let mut selected = self.with_enabled_combat_candidates()?;
        selected.syntax_choices.insert(
            "camp_hp_recovery_conflict".into(),
            "larger_original_penalty".into(),
        );
        selected.validate()?;
        Ok(selected)
    }
    pub fn validate(&self) -> Result<()> {
        for (id, choice) in &self.syntax_choices {
            let package = match (id.as_str(), choice.as_str()) {
                ("difficulty_effect_source", "mode_file") => "base",
                ("camp_hp_recovery_conflict", "larger_original_penalty") => "base",
                ("legacy_global_boundary", "outside_combat_v1") => "base",
                ("boss_inactive_refs", "retain") => {
                    if !self
                        .packages
                        .iter()
                        .any(|p| ["crimson_court", "color_of_madness"].contains(&p.as_str()))
                    {
                        return Err(
                            "inactive reference choice requires crimson_court or color_of_madness"
                                .into(),
                        );
                    }
                    continue;
                }
                ("baron_rank_zero", "auto") => "crimson_court",
                ("smoking_skull_conflict", "poison_resistance") => "color_of_madness",
                ("spire_target_backtick", "strip") | ("farm_croc_skill_prefix", "com") => {
                    "color_of_madness"
                }
                _ => return Err("unknown explicit syntax candidate".into()),
            };
            if !self.packages.iter().any(|p| p == package) {
                return Err("syntax candidate requires enabled package".into());
            }
        }
        for (id, choice) in &self.effect_choices {
            let package = match (id.as_str(), choice.as_str()) {
                ("Burn Hero 1", "capture") | ("Inject Self Damage", "performer") => "crimson_court",
                ("Thirsting Blade Self Atk On Miss", "performer") => "color_of_madness",
                _ => return Err("unknown explicit effect conflict choice".into()),
            };
            if !self.packages.iter().any(|p| p == package) {
                return Err("effect choice requires enabled package".into());
            }
        }
        let mut selected = BTreeSet::new();
        for id in &self.packages {
            if !PACKAGES.iter().any(|(p, _)| *p == id) {
                return Err(format!("unknown or out-of-scope content package {id}"));
            }
            if !selected.insert(id.as_str()) {
                return Err(format!("duplicate content package {id}"));
            }
        }
        if !selected.contains("base") {
            return Err("content requires explicit base package".into());
        }
        for (id, dependency) in [
            ("fires_edge", "districts"),
            ("duelist", "fires_edge"),
            ("runaway", "fires_edge"),
        ] {
            if selected.contains(id) && !selected.contains(dependency) {
                return Err(format!("{id} requires {dependency}"));
            }
        }
        if !["base", "radiant", "new_game_plus", "bloodmoon"].contains(&self.mode.as_str()) {
            return Err(format!("unknown mode {}", self.mode));
        }
        if self.mode == "bloodmoon" && !selected.contains("crimson_court") {
            return Err("bloodmoon requires crimson_court".into());
        }
        if !["strict", "permissive"].contains(&self.never_again.as_str()) {
            return Err("unknown never_again option".into());
        }
        if self.no_light {
            return Err("no_light is not implemented".into());
        }
        Ok(())
    }
    pub fn normalized(&self) -> Result<Self> {
        self.validate()?;
        let mut value = self.clone();
        value.packages.sort();
        Ok(value)
    }
    pub fn id(&self) -> Result<String> {
        let config = self.normalized()?;
        let suffix = config
            .effect_choices
            .iter()
            .map(|(id, choice)| format!(";effect:{}={choice}", id.replace(' ', "%20")))
            .collect::<String>();
        let syntax = config
            .syntax_choices
            .iter()
            .map(|(id, v)| format!(";syntax:{id}={v}"))
            .collect::<String>();
        Ok(format!(
            "content-v1;packages={};mode={};never_again={};no_light=false{suffix}{syntax}",
            config.packages.join(","),
            config.mode,
            config.never_again
        ))
    }
    pub fn validate_root(&self, root: &Path) -> Result<()> {
        self.validate()?;
        for id in &self.packages {
            for path in PACKAGES.iter().find(|(p, _)| *p == id).unwrap().1 {
                if !root.join(path).is_dir() {
                    return Err(format!(
                        "missing content directory {} ({id})",
                        root.join(path).display()
                    ));
                }
            }
        }
        Ok(())
    }
}
