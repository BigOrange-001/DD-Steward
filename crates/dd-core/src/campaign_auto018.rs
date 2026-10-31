//! Explicit campaign extension. Default 017 definitions remain unchanged.
use super::*;
use dd_data::mechanics::{Buff, read_buff};
pub const PROFILE: &str = "candidate-campaign-018";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Activity {
    pub allowed: BTreeSet<String>,
    pub stress: Option<[u32; 2]>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub activities: BTreeMap<String, Activity>,
    pub candidate: Value,
    pub content: String,
    pub can_reenter: bool,
    pub minimum_stress: i32,
    pub reentry_buffs: Vec<Buff>,
    pub veteran_xp: f64,
    pub failure_xp: f64,
    pub failure_duration: u32,
    pub legacy_minimum_escape: u32,
}
impl Rules {
    pub fn load(game: &GameData) -> Result<Self> {
        if game
            .overlay
            .config
            .syntax_choices
            .get("camp_hp_recovery_conflict")
            .map(String::as_str)
            != Some("larger_original_penalty")
        {
            return Err("018 requires explicit ContentConfig::with_campaign018_candidates".into());
        }
        let candidate: Value =
            serde_json::from_str(r###"{
  "version": "campaign-candidates-018-v3",
  "semantics": {
    "fixed_hazard_index": "index_zero_only_if_one_distinct_effective_prop_type_otherwise_error",
    "dd_escape": "source_random_sacrifice_count_but_preserve_one_survivor_no_retreat_final_no_deliberate_week_skip",
    "legacy_escape_minimum": "source_value_retained_as_unresolved_host_boundary_not_assumed_dd_sacrifice_threshold",
    "dd_veterans": "actual_success_living_departure_guids_marked_permanently_roster_capacity_exempt_nonmarked_party_xp_additive_per_veteran",
    "failure_xp": "source_roster_buff_additive_per_failure_per_living_hero_consumed_on_that_hero_next_actual_completion",
    "dd_camping": "no_ordinary_night_ambush_or_shieldbreaker_nightmare_in_DD_wiki_rule_not_inferred_from_placeholder_shambler_pool_pending_original_calibration",
    "dd_teleport": "native_teleported_outcome_uses_only_original_equivalent_map_destination_sets_otherwise_error_unresolved_encounter_not_cleared",
    "repeat_goals": "one_actual_completion_each_court_bloodcrate_and_statue_in_addition_to_four_dd_and_four_court_plots",
    "deadline": "success_at_or_before_limit_precedes_timeout_at_free_town_boundary_native_game_failure_separate",
    "dd_stall_duplicate": "only_the_known_anonymous_dd_stall_pair_same_two_cultist_shrouded_D_with_weights_2_and_1_preserved_as_weighted_rows_exact_duplicates_dedup_other_conflicts_error",
    "dd_week": "one_actual_dd_return_always_advances_even_early_abort_no_artificial_wait_action",
    "final_journal": "exact_original_THANKS_specific_page_index_zero_without_flags_is_ordinary_carryable_unique_page_other_flag_forms_still_strict",
    "court_repeat_room_curios": "when_original_Courtyard_room_curios_pool_absent_use_original_hall_curios_for_014_generated_repeat_map_room_markers_weights_unchanged_pending_calibration",
    "class_activities": "source_allow_list_authoritative_stress_override_replaces_base_before_existing_modifiers_class_forbidden_add_quirk_side_effects_excluded_with_original_relative_weights_known_empty_pool_no_effect",
    "empty_trinket_loot_pool": "an_existing_rarity_with_no_region_class_or_uniqueness_eligible_items_produces_no_award_no_reroll_or_cross_region_substitution_unknown_rarity_or_conflicting_data_still_error_only_018_native_loot",
    "native_trinket_ownership": "actual_estate_including_equipped_trinkets_frozen_in_018_raid_contract_and_combined_with_raid_draw_history_for_original_pool_limits"
  }
}"###)
                .map_err(|e| e.to_string())?;
        let bytes = game
            .overlay
            .read_automation018("campaign/roster/roster.settings.json")?;
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        dd_data::campaign017::fields(&v, &["never_again_settings"], "never again settings")?;
        let rows = v["never_again_settings"]
            .as_array()
            .ok_or("missing never-again settings")?;
        let rows = rows
            .iter()
            .filter(|r| r["id"] == game.overlay.config.never_again)
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            return Err("missing/ambiguous never-again rule".into());
        }
        let row = rows[0];
        dd_data::campaign017::fields(
            row,
            &[
                "id",
                "darkest_dungeon_can_enter",
                "darkest_dungeon_buff_ids",
                "darkest_duungeon_minimum_stress",
            ],
            "never again rule",
        )?;
        let minimum = row["darkest_duungeon_minimum_stress"]
            .as_f64()
            .filter(|n| n.is_finite() && n.fract() == 0. && *n >= 0. && *n <= 200.)
            .ok_or("invalid reentry stress")? as i32;
        let reentry_buffs = row["darkest_dungeon_buff_ids"]
            .as_array()
            .ok_or("missing reentry buffs")?
            .iter()
            .map(|v| read_buff(&game.buffs, v.as_str().ok_or("invalid reentry buff")?))
            .collect::<Result<Vec<_>>>()?;
        let raw = |id: &str| dd_data::library::unique_json(&game.buffs, id, "DD XP buff");
        let veteran = raw("completed_darkest_dungeon_quest_party_resolve_xp")?;
        let failure = raw("darkest_dungeon_failure_roster_resolve_xp")?;
        for b in [veteran, failure] {
            if b["stat_type"] != "resolve_xp_bonus_percent" || b["rule_type"] != "always" {
                return Err("unsupported DD XP buff rule".into());
            }
        }
        if dd_data::campaign017::uint(failure, "duration")? != 1 {
            return Err("unsupported multicompletion DD XP lifetime".into());
        }
        if failure["duration_type"] != "quest_complete" {
            return Err("unsupported DD failure XP lifetime".into());
        }
        let mut activities = BTreeMap::new();
        for id in game.hero_ids() {
            let doc = game.hero_document(id)?;
            let rows = doc.records_keyed("activity_modifier").collect::<Vec<_>>();
            if rows.len() > 1 {
                return Err("duplicate class activity modifier".into());
            }
            if let Some(r) = rows.first() {
                if r.params.iter().any(|p| {
                    ![
                        "override_valid_activity_ids",
                        "override_stress_removal_amount_low",
                        "override_stress_removal_amount_high",
                    ]
                    .contains(&p.name.as_str())
                }) {
                    return Err("unknown class activity modifier".into());
                }
                let allowed = r
                    .param("override_valid_activity_ids")
                    .map(|p| {
                        p.values
                            .iter()
                            .map(|v| v.text.clone())
                            .collect::<BTreeSet<_>>()
                    })
                    .unwrap_or_default();
                if allowed.iter().any(|s| {
                    ![
                        "flagellation",
                        "treatment",
                        "disease_treatment",
                        "bar",
                        "gambling",
                        "brothel",
                        "prayer",
                        "meditation",
                    ]
                    .contains(&s.as_str())
                }) {
                    return Err("unknown class activity ID".into());
                }
                let stress = if r.param("override_stress_removal_amount_low").is_some()
                    || r.param("override_stress_removal_amount_high").is_some()
                {
                    let low = dd_data::combat::number(r, "override_stress_removal_amount_low")?;
                    let high = dd_data::combat::number(r, "override_stress_removal_amount_high")?;
                    if !low.is_finite()
                        || !high.is_finite()
                        || low.fract() != 0.
                        || high.fract() != 0.
                        || low < 0.
                        || high < low
                        || high > 10000.
                    {
                        return Err("invalid class stress treatment override".into());
                    }
                    Some([low as u32, high as u32])
                } else {
                    None
                };
                activities.insert(id.clone(), Activity { allowed, stress });
            }
        }
        Ok(Self {
            activities,
            candidate,
            content: game.overlay.config.id()?,
            can_reenter: row["darkest_dungeon_can_enter"]
                .as_bool()
                .ok_or("invalid reentry permission")?,
            minimum_stress: minimum,
            reentry_buffs,
            veteran_xp: veteran["amount"].as_f64().ok_or("missing veteran XP")?,
            failure_xp: failure["amount"].as_f64().ok_or("missing failure XP")?,
            failure_duration: dd_data::campaign017::uint(failure, "duration")?,
            legacy_minimum_escape: game.rules.min_hero_count_for_escape,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(in crate::town016) struct Owned {
    pub rules: Rules,
    pub marked: BTreeSet<u64>,
    pub failure: BTreeMap<u64, u32>,
    pub repeat_completions: BTreeMap<String, u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RaidRules {
    pub owned_trinkets: Vec<crate::initial013::Reward>,
    pub contract: Option<dd_data::contract011::Contract>,
    pub no_random_ambush: bool,
    pub no_torch_burn: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Progress {
    pub can_reenter: bool,
    pub failure_xp_stacks: BTreeMap<u64, u32>,
    pub completed: Vec<String>,
    pub marked: Vec<u64>,
    pub repeat_completions: BTreeMap<String, u32>,
}
impl Definition {
    pub fn with_automation018(mut self) -> Result<Self> {
        use sha2::{Digest, Sha256};
        let rules = Rules::load(&self.town.game)?;
        self.id = format!(
            "{PROFILE};sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&(&self.id, &rules)).map_err(|e| e.to_string())?)
        );
        self.automation = Some(Arc::new(rules));
        self.town_view = Arc::new(Mutex::new(None));
        self.sources = Arc::new(Mutex::new(BTreeMap::new()));
        self.inventories = Arc::new(Mutex::new(BTreeMap::new()));
        self.raids = Arc::new(Mutex::new(BTreeMap::new()));
        Ok(self)
    }
    pub(super) fn profile(&self) -> &'static str {
        if self.content019.is_some() {
            super::content019::PROFILE
        } else if self.automation.is_some() {
            PROFILE
        } else {
            CAMPAIGN_PROFILE
        }
    }
}
impl Session {
    pub(super) fn automation_progress(&self) -> Option<Progress> {
        self.state
            .city
            .campaign
            .as_ref()?
            .automation
            .as_ref()
            .map(|a| Progress {
                can_reenter: a.rules.can_reenter,
                failure_xp_stacks: a.failure.clone(),
                completed: self.state.city.completed_ids.iter().cloned().collect(),
                marked: a.marked.iter().copied().collect(),
                repeat_completions: a.repeat_completions.clone(),
            })
    }
    pub(super) fn validate_automation(&self) -> Result<()> {
        let a = self
            .state
            .city
            .campaign
            .as_ref()
            .and_then(|c| c.automation.as_ref());
        match (&self.definition.automation, a) {
            (None, None) => Ok(()),
            (Some(rules), Some(a)) if **rules == a.rules => {
                if a.marked
                    .iter()
                    .chain(a.failure.keys())
                    .any(|id| !self.state.city.heroes.iter().any(|h| h.guid == *id))
                    || a.failure.values().any(|n| *n == 0 || *n > 1000)
                    || a.repeat_completions
                        .keys()
                        .any(|k| !["farm_bloodcrate", "kill_statue"].contains(&k.as_str()))
                {
                    return Err("invalid automation ownership/progress".into());
                }
                Ok(())
            }
            _ => Err("automation rule/source mismatch".into()),
        }
    }
}

impl Session {
    pub(super) fn automation_after_return(
        &self,
        town: &mut TownSession,
        q: &board::Instance,
        complete: bool,
        party: &[u64],
    ) -> Result<()> {
        if self.definition.automation.is_none() {
            return Ok(());
        }
        if complete {
            let auto = town
                .state
                .campaign
                .as_mut()
                .unwrap()
                .automation
                .as_mut()
                .unwrap();
            for id in party {
                auto.failure.remove(id);
            }
            if q.raw["dungeon"] == "courtyard"
                && let Some(kind) = q.raw["type"]
                    .as_str()
                    .filter(|s| ["farm_bloodcrate", "kill_statue"].contains(s))
            {
                *auto.repeat_completions.entry(kind.into()).or_default() += 1;
            }
        }
        if q.raw["dungeon"] == "darkestdungeon" {
            if complete {
                let living = party
                    .iter()
                    .filter(|id| town.hero(**id).is_ok_and(|h| h.state.alive))
                    .copied()
                    .collect::<Vec<_>>();
                town.state
                    .campaign
                    .as_mut()
                    .unwrap()
                    .automation
                    .as_mut()
                    .unwrap()
                    .marked
                    .extend(living);
                if q.source["is_roster_stress_cleared_on_completion"] == true {
                    for id in town
                        .state
                        .heroes
                        .iter()
                        .filter(|h| h.state.alive)
                        .map(|h| h.guid)
                        .collect::<Vec<_>>()
                    {
                        let stress = town.hero(id)?.state.stress;
                        town.town_stress(id, -stress)?;
                    }
                }
            } else {
                let minimum = dd_data::campaign017::uint(
                    &q.source,
                    "roster_buff_on_failure_minimum_party_resolve_level",
                )?;
                let eligible = party.iter().any(|id| {
                    town.hero(*id)
                        .is_ok_and(|h| town.level(h.xp).is_ok_and(|n| n >= minimum))
                });
                let buffs = q.source["roster_buffs_to_apply_on_failure"]
                    .as_array()
                    .ok_or("missing DD failure buffs")?;
                if eligible && !buffs.is_empty() {
                    if buffs != &[json!("darkest_dungeon_failure_roster_resolve_xp")] {
                        return Err("unknown DD failure roster buff".into());
                    }
                    let ids = town
                        .state
                        .heroes
                        .iter()
                        .filter(|h| h.state.alive)
                        .map(|h| h.guid)
                        .collect::<Vec<_>>();
                    let a = town
                        .state
                        .campaign
                        .as_mut()
                        .unwrap()
                        .automation
                        .as_mut()
                        .unwrap();
                    for id in ids {
                        let n = a.failure.entry(id).or_default();
                        *n = n
                            .checked_add(a.rules.failure_duration)
                            .ok_or("DD failure stack overflow")?;
                    }
                }
            }
        }
        Ok(())
    }
}
impl TownSession {
    pub(in crate::town016) fn class_activity018(&self, class: &str) -> Option<&Activity> {
        self.state
            .campaign
            .as_ref()?
            .automation
            .as_ref()?
            .rules
            .activities
            .get(class)
    }
    pub(in crate::town016) fn marked_count018(&self) -> u32 {
        self.state
            .campaign
            .as_ref()
            .and_then(|c| c.automation.as_ref())
            .map_or(0, |a| {
                self.state
                    .heroes
                    .iter()
                    .filter(|h| h.state.alive && a.marked.contains(&h.guid))
                    .count() as u32
            })
    }
    pub(in crate::town016) fn xp_bonus018(&self, id: u64) -> f64 {
        self.state
            .campaign
            .as_ref()
            .and_then(|c| c.automation.as_ref())
            .map_or(0., |a| {
                let veteran = if a.marked.contains(&id) {
                    0.
                } else {
                    self.state
                        .party
                        .iter()
                        .filter(|id| a.marked.contains(id))
                        .count() as f64
                        * a.rules.veteran_xp
                };
                veteran + f64::from(a.failure.get(&id).copied().unwrap_or(0)) * a.rules.failure_xp
            })
    }
}

#[path = "campaign_runner018.rs"]
pub mod runner;
