//! Static public skill descriptions and visible roster preparation; no host state.
use crate::town016::{Observation, VisibleHero};
use dd_data::combat::{EffectKind, EffectTarget, GameData, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[path = "asset_plan024b.rs"]
pub mod asset_plan;
#[path = "assets024.rs"]
pub mod assets;
#[path = "party024.rs"]
pub mod composition;
#[path = "economy024.rs"]
pub mod economy;
#[path = "estate024.rs"]
pub mod estate;
#[path = "funnel024b.rs"]
pub mod funnel;
#[path = "gates024.rs"]
pub mod gates;
#[path = "merchant024.rs"]
pub mod merchant;
#[path = "auto_policy024.rs"]
pub mod policy;
#[path = "provisions024.rs"]
pub mod provisions;
#[path = "recovery024b.rs"]
pub mod recovery;

pub const GROUPS: [&str; 9] = ["A", "A2", "B", "C", "D", "H", "E", "G", "F"];
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Loadout {
    pub id: String,
    pub group: String,
    pub sources: Vec<String>,
    pub class_id: String,
    pub skills: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalLoadout {
    pub class_id: String,
    pub missing_role: String,
    pub unless_missing: Option<String>,
    pub replace: String,
    pub with_skill: String,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Learning {
    pub id: String,
    pub sources: Vec<String>,
    pub reserve_gold: u32,
    pub budget_basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub revision: u32,
    pub profile: String,
    pub status: String,
    pub group_defaults: BTreeMap<String, bool>,
    pub loadouts: Vec<Loadout>,
    pub legacy_preferences: BTreeMap<String, Vec<String>>,
    pub role_weights: BTreeMap<String, f64>,
    pub role_rule: serde_json::Value,
    pub power: serde_json::Value,
    pub alternatives: BTreeMap<String, serde_json::Value>,
    pub active_variants: BTreeMap<String, String>,
    pub conditional_loadouts: Vec<ConditionalLoadout>,
    pub learning: Learning,
    pub composition: composition::Settings,
    pub gates: gates::Settings,
    pub provisions: provisions::Settings,
    pub economy: economy::Settings,
    pub assets: assets::Settings,
    pub merchant: merchant::Settings,
    pub estate: estate::Settings,
    pub recovery: recovery::Settings,
}
impl Profile {
    pub fn documented() -> Result<Self> {
        Self::parse(r#"{
  "profile": "candidate-preparation-024-v1",
  "status": "unverified user-experience candidate; 024b development integration and checks passed; not for formal training",
  "group_defaults": {
    "A": true,
    "A2": true,
    "B": true,
    "C": true,
    "D": true,
    "H": true,
    "E": true,
    "G": true,
    "F": true
  },
  "loadouts": [
    {
      "id": "P024-01",
      "group": "A",
      "sources": [
        "W-065",
        "W-066"
      ],
      "class_id": "vestal",
      "skills": [
        "dazzling_light",
        "judgement",
        "divine_grace",
        "gods_comfort"
      ]
    },
    {
      "id": "P024-02",
      "group": "A",
      "sources": [
        "W-006",
        "W-007",
        "W-065"
      ],
      "class_id": "occultist",
      "skills": [
        "daemons_pull",
        "hands_from_abyss",
        "bloodlet",
        "wyrd_reconstruction"
      ]
    },
    {
      "id": "P024-03",
      "group": "A",
      "sources": [
        "W-065",
        "W-067"
      ],
      "class_id": "man_at_arms",
      "skills": [
        "defender",
        "command",
        "rampart",
        "crush"
      ]
    },
    {
      "id": "P024-04",
      "group": "A",
      "sources": [
        "W-065",
        "W-068"
      ],
      "class_id": "plague_doctor",
      "skills": [
        "noxious_blast",
        "blinding_gas",
        "disorienting_blast",
        "battlefield_medicine"
      ]
    },
    {
      "id": "P024-05",
      "group": "A",
      "sources": [
        "W-047",
        "W-065",
        "W-069"
      ],
      "class_id": "houndmaster",
      "skills": [
        "hounds_rush",
        "howl",
        "lick_wounds",
        "whistle"
      ]
    },
    {
      "id": "P024-06",
      "group": "A",
      "sources": [
        "W-047",
        "W-070"
      ],
      "class_id": "leper",
      "skills": [
        "chop",
        "hew",
        "solemnity",
        "intimidate"
      ]
    },
    {
      "id": "P024-07",
      "group": "A",
      "sources": [
        "W-071"
      ],
      "class_id": "jester",
      "skills": [
        "inspiring_tune",
        "battle_ballad",
        "slice_off",
        "harvest"
      ]
    },
    {
      "id": "P024-08",
      "group": "A",
      "sources": [
        "W-072"
      ],
      "class_id": "shieldbreaker",
      "skills": [
        "pierce",
        "serpents_sway",
        "break_guard",
        "adders_kiss"
      ]
    },
    {
      "id": "P024-09",
      "group": "A",
      "sources": [
        "W-073"
      ],
      "class_id": "antiquarian",
      "skills": [
        "kris_stab",
        "protect_me",
        "invigorating_vapours",
        "fortifying_vapours"
      ]
    },
    {
      "id": "P024-10",
      "group": "A",
      "sources": [
        "W-074"
      ],
      "class_id": "crusader",
      "skills": [
        "smite",
        "stunning_blow",
        "holy_lance",
        "inspiring_cry"
      ]
    },
    {
      "id": "P024-11",
      "group": "A",
      "sources": [
        "W-075"
      ],
      "class_id": "flagellant",
      "skills": [
        "punish",
        "rain_of_sorrows",
        "redeem",
        "reclaim"
      ]
    },
    {
      "id": "P024-12",
      "group": "A",
      "sources": [
        "W-047",
        "W-076"
      ],
      "class_id": "abomination",
      "skills": [
        "manacles",
        "absolution",
        "vomit",
        "transform"
      ]
    }
  ],
  "legacy_preferences": {
    "highwayman": [
      "wicked_slice",
      "pistol_shot",
      "duelist_advance",
      "point_blank_shot"
    ],
    "crusader": [
      "smite",
      "stunning_blow",
      "holy_lance",
      "inspiring_cry"
    ],
    "plague_doctor": [
      "noxious_blast",
      "plague_grenade",
      "battlefield_medicine",
      "blinding_gas"
    ],
    "vestal": [
      "divine_grace",
      "gods_comfort",
      "judgement",
      "dazzling_light"
    ],
    "houndmaster": [
      "hounds_rush",
      "hounds_harry",
      "lick_wounds",
      "howl"
    ],
    "man_at_arms": [
      "crush",
      "retribution",
      "defender",
      "bolster"
    ],
    "hellion": [
      "if_it_bleeds",
      "iron_swan",
      "breakthru",
      "adrenaline_rush"
    ],
    "flagellant": [
      "punish",
      "rain_of_sorrows",
      "exsanguinate",
      "redeem"
    ],
    "occultist": [
      "wyrd_reconstruction",
      "bloodlet",
      "hands_from_abyss",
      "abyssal_artillery"
    ],
    "runaway": [
      "searing_strike",
      "firefly",
      "run_and_hide",
      "backdraft"
    ],
    "shieldbreaker": [
      "pierce",
      "break_guard",
      "serpents_sway",
      "adders_kiss"
    ]
  },
  "role_weights": {
    "skill_coverage": 100,
    "damage": 4,
    "healing": 12,
    "stress_relief": 12,
    "control": 10,
    "recovery_mobility": 3,
    "healing_presence": 4,
    "relief_presence": 3,
    "target_reach": 0.25
  },
  "power": {
    "id": "P024-13",
    "sources": [
      "W-018",
      "W-019"
    ],
    "weights": [
      1,
      1,
      1,
      1
    ]
  },
  "alternatives": {
    "jester_finale": [
      "dirk_stab",
      "heroic_end",
      "inspiring_tune",
      "battle_ballad"
    ],
    "crusader_healing": "battle_heal",
    "flagellant_relief": "endure",
    "plague_backline": "plague_grenade"
  },
  "active_variants": {},
  "conditional_loadouts": [
    {
      "class_id": "crusader",
      "missing_role": "healing",
      "unless_missing": null,
      "replace": "inspiring_cry",
      "with_skill": "battle_heal",
      "source": "W-074"
    },
    {
      "class_id": "flagellant",
      "missing_role": "stress_relief",
      "unless_missing": "healing",
      "replace": "redeem",
      "with_skill": "endure",
      "source": "W-075"
    }
  ],
  "role_rule": {
    "id": "P024-14",
    "sources": [
      "W-047"
    ],
    "basis": "public launch/target masks and effect summaries; candidate weights"
  },
  "learning": {
    "id": "P024-15",
    "sources": [
      "W-065"
    ],
    "reserve_gold": 4500,
    "budget_basis": "retain the pre-existing essential-skill expedition reserve; H may replace it with the explicit next-raid budget"
  },
  "revision": 19,
  "composition": {
    "id": "P024-16",
    "sources": [
      "W-001",
      "W-002",
      "W-013",
      "W-032",
      "W-033",
      "W-041",
      "W-042",
      "W-046"
    ],
    "templates": [
      {
        "name": "preferred",
        "slots": [
          [
            "man_at_arms"
          ],
          [
            "occultist"
          ],
          [
            "plague_doctor"
          ],
          [
            "houndmaster"
          ]
        ]
      },
      {
        "name": "double-hound",
        "slots": [
          [
            "man_at_arms"
          ],
          [
            "occultist"
          ],
          [
            "houndmaster"
          ],
          [
            "houndmaster"
          ]
        ]
      },
      {
        "name": "hound-arbalest",
        "slots": [
          [
            "man_at_arms"
          ],
          [
            "occultist"
          ],
          [
            "houndmaster"
          ],
          [
            "arbalest"
          ]
        ]
      },
      {
        "name": "guard-vestal-jester",
        "slots": [
          [
            "man_at_arms"
          ],
          [],
          [
            "jester"
          ],
          [
            "vestal"
          ]
        ]
      },
      {
        "name": "guard-occultist-relief",
        "slots": [
          [
            "man_at_arms"
          ],
          [
            "occultist"
          ],
          [
            "jester",
            "houndmaster"
          ],
          []
        ]
      }
    ],
    "beam_width": 32,
    "candidates_per_template_slot": 4,
    "weights": {
      "damage": 1,
      "healing": 8,
      "relief": 6,
      "control": 20,
      "guard": 15,
      "shuffle": 10,
      "power": 5,
      "training_cost": 0.001,
      "region": 2
    },
    "medium_min_heal": 1,
    "medium_min_relief": 1,
    "medium_min_control": 0.5,
    "region_resists": {
      "crypts": [
        0.6,
        0.4
      ],
      "warrens": [
        0.45,
        0.6
      ],
      "weald": [
        0.5,
        0.6
      ],
      "cove": [
        0.6,
        0.4
      ]
    },
    "region_basis": "W-042 unweighted public regional summary; not the actual unscouted enemy lineup",
    "wealthy_gold": 12000,
    "emergency_short_min_party": 1
  },
  "gates": {
    "id": "P024-17",
    "sources": [
      "W-003",
      "W-018",
      "W-019",
      "W-023",
      "W-076"
    ],
    "tiers": {
      "1": {
        "minimum": 0,
        "gear_floor": 0,
        "gear_cap": 0
      },
      "3": {
        "minimum": 2,
        "gear_floor": 2,
        "gear_cap": 3
      },
      "5": {
        "minimum": 4,
        "gear_floor": 4,
        "gear_cap": 4
      },
      "6": {
        "minimum": 5,
        "gear_floor": 4,
        "gear_cap": 4
      }
    },
    "court_minimum": {
      "1": 3,
      "3": 4,
      "5": 5,
      "6": 5
    },
    "court_gear": {
      "1": 3,
      "3": 4,
      "5": 4,
      "6": 4
    },
    "champion_after_week": 26,
    "apprentice_required_one_level": 1,
    "allow_all_zero_rebuild_short": true,
    "ordinary_regions": [
      "crypts",
      "weald",
      "warrens",
      "cove"
    ],
    "length_priority": {
      "1": 2,
      "2": 0,
      "3": 1
    },
    "ignored_skills": [
      "transform"
    ],
    "notes": "Zero-based equipment/skill ranks. Keep conservative courtyard floors; allow an all-level-zero short ordinary rebuilding trip only when no available level-one hero exists. Week26 is a conservative candidate definition of early champion avoidance, not a source rule."
  },
  "provisions": {
    "id": "P024-18",
    "sources": [
      "W-053"
    ],
    "tables": {
      "crypts": {
        "1": {
          "food": [
            8,
            12
          ],
          "torch": [
            6,
            8
          ],
          "shovel": [
            0,
            1
          ],
          "holy_water": [
            1,
            2
          ],
          "medicinal_herbs": [
            1,
            2
          ],
          "skeleton_key": [
            1,
            2
          ],
          "bandage": [
            0,
            0
          ],
          "antivenom": [
            0,
            0
          ]
        },
        "2": {
          "food": [
            20,
            24
          ],
          "torch": [
            14,
            16
          ],
          "shovel": [
            1,
            2
          ],
          "holy_water": [
            2,
            3
          ],
          "medicinal_herbs": [
            2,
            3
          ],
          "skeleton_key": [
            2,
            4
          ],
          "bandage": [
            0,
            0
          ],
          "antivenom": [
            0,
            0
          ]
        },
        "3": {
          "food": [
            32,
            36
          ],
          "torch": [
            20,
            22
          ],
          "shovel": [
            2,
            3
          ],
          "holy_water": [
            3,
            4
          ],
          "medicinal_herbs": [
            3,
            4
          ],
          "skeleton_key": [
            3,
            4
          ],
          "bandage": [
            0,
            2
          ],
          "antivenom": [
            0,
            0
          ]
        }
      },
      "weald": {
        "1": {
          "food": [
            8,
            12
          ],
          "torch": [
            6,
            8
          ],
          "shovel": [
            1,
            2
          ],
          "holy_water": [
            0,
            1
          ],
          "medicinal_herbs": [
            0,
            1
          ],
          "skeleton_key": [
            1,
            2
          ],
          "bandage": [
            1,
            2
          ],
          "antivenom": [
            1,
            2
          ]
        },
        "2": {
          "food": [
            16,
            20
          ],
          "torch": [
            14,
            16
          ],
          "shovel": [
            3,
            4
          ],
          "holy_water": [
            0,
            2
          ],
          "medicinal_herbs": [
            1,
            2
          ],
          "skeleton_key": [
            2,
            4
          ],
          "bandage": [
            2,
            3
          ],
          "antivenom": [
            2,
            3
          ]
        },
        "3": {
          "food": [
            24,
            32
          ],
          "torch": [
            20,
            22
          ],
          "shovel": [
            3,
            5
          ],
          "holy_water": [
            0,
            2
          ],
          "medicinal_herbs": [
            1,
            2
          ],
          "skeleton_key": [
            3,
            4
          ],
          "bandage": [
            3,
            4
          ],
          "antivenom": [
            4,
            5
          ]
        }
      },
      "warrens": {
        "1": {
          "food": [
            8,
            12
          ],
          "torch": [
            6,
            8
          ],
          "shovel": [
            0,
            1
          ],
          "holy_water": [
            1,
            2
          ],
          "medicinal_herbs": [
            1,
            2
          ],
          "skeleton_key": [
            1,
            2
          ],
          "bandage": [
            0,
            1
          ],
          "antivenom": [
            0,
            0
          ]
        },
        "2": {
          "food": [
            16,
            20
          ],
          "torch": [
            14,
            16
          ],
          "shovel": [
            1,
            2
          ],
          "holy_water": [
            2,
            3
          ],
          "medicinal_herbs": [
            2,
            3
          ],
          "skeleton_key": [
            2,
            3
          ],
          "bandage": [
            1,
            2
          ],
          "antivenom": [
            0,
            1
          ]
        },
        "3": {
          "food": [
            24,
            32
          ],
          "torch": [
            20,
            22
          ],
          "shovel": [
            2,
            3
          ],
          "holy_water": [
            3,
            4
          ],
          "medicinal_herbs": [
            3,
            4
          ],
          "skeleton_key": [
            2,
            3
          ],
          "bandage": [
            2,
            4
          ],
          "antivenom": [
            2,
            3
          ]
        }
      },
      "cove": {
        "1": {
          "food": [
            8,
            12
          ],
          "torch": [
            6,
            8
          ],
          "shovel": [
            2,
            3
          ],
          "holy_water": [
            0,
            1
          ],
          "medicinal_herbs": [
            1,
            2
          ],
          "skeleton_key": [
            1,
            2
          ],
          "bandage": [
            0,
            1
          ],
          "antivenom": [
            0,
            1
          ]
        },
        "2": {
          "food": [
            20,
            24
          ],
          "torch": [
            14,
            16
          ],
          "shovel": [
            3,
            4
          ],
          "holy_water": [
            0,
            2
          ],
          "medicinal_herbs": [
            2,
            3
          ],
          "skeleton_key": [
            1,
            2
          ],
          "bandage": [
            3,
            4
          ],
          "antivenom": [
            0,
            1
          ]
        },
        "3": {
          "food": [
            32,
            36
          ],
          "torch": [
            20,
            22
          ],
          "shovel": [
            3,
            5
          ],
          "holy_water": [
            0,
            2
          ],
          "medicinal_herbs": [
            3,
            5
          ],
          "skeleton_key": [
            2,
            3
          ],
          "bandage": [
            5,
            6
          ],
          "antivenom": [
            0,
            2
          ]
        }
      }
    },
    "essentials": [
      "food",
      "torch"
    ],
    "order": [
      "food",
      "torch",
      "shovel",
      "skeleton_key",
      "holy_water",
      "medicinal_herbs",
      "bandage",
      "antivenom"
    ],
    "notes": "User-approved community table. Fill lower bounds first; balance food/torch by coverage; optional upper-bound extras last. Source prices, stock, capacity, class gifts and legality remain host-owned. Farm/courtyard/darkestdungeon and no-light are outside this table; retain legacy behavior there. Do not discard gifted excess."
  },
  "economy": {
    "id": "P024-19",
    "sources": [
      "W-058",
      "W-063"
    ],
    "poverty_gold": 2350,
    "food_floor": 8,
    "protected_items": [
      "focus_ring",
      "fortifying_garlic"
    ],
    "qualified_merchant_overrides_cheap": true
  },
  "assets": {
    "id": "P024-20",
    "sources": [
      "W-024",
      "W-025",
      "W-026",
      "W-027",
      "W-034",
      "W-077",
      "W-078",
      "W-079"
    ],
    "weights": {
      "scouting_chance": 100.0,
      "stun_chance": 50.0,
      "hp_heal_percent": 50.0,
      "stress_heal_percent": 50.0,
      "move_chance": 20.0,
      "debuff_chance": 20.0,
      "accuracy": 20.0,
      "dodge": 10.0,
      "speed": 2.0,
      "damage": 1.0,
      "hp": 0.1,
      "resistance": 5.0,
      "crit_expected_damage": 10.0,
      "stress_dmg_received_percent": -5.0
    },
    "planning_light": 100.0,
    "stress_penalty_with_relief": 0.5,
    "named_bias": {
      "fortifying_garlic": {
        "flagellant": 10.0,
        "*": 1.0
      },
      "focus_ring": {
        "*": 1.0
      },
      "ancestors_map": {
        "*": 3.0
      },
      "ancestors_scroll": {
        "*": 1.0
      }
    },
    "focus_reward_priority": 4
  },
  "merchant": {
    "id": "P024-21",
    "sources": [
      "W-043"
    ],
    "poverty_gold": 2350,
    "escort_power": {
      "1": 1.0,
      "3": 2.5
    },
    "escort_gear": {
      "1": 1,
      "3": 2
    },
    "escort_skills": {
      "1": 1.0,
      "3": 2.0
    },
    "candidate_limit": 12
  },
  "estate": {
    "id": "P024-22",
    "sources": [
      "W-016",
      "W-017",
      "W-020",
      "W-049"
    ],
    "recruit_tree": "stage_coach.numrecruits",
    "recruit_levels": 2,
    "ordered_trees": [
      "guild.skill_levels",
      "guild.cost",
      "blacksmith.weapon",
      "blacksmith.armour",
      "blacksmith.cost"
    ],
    "reserve_gold": 4500,
    "skip_turn_quirks": [
      "imposter_syndrome"
    ],
    "deferred": [
      "bust facility capacity and treatment cost optimization",
      "positive quirk locking budget",
      "bank and other district future-cash ROI",
      "dungeon quirk removal is 025"
    ]
  },
  "recovery": {
    "id": "P024-23",
    "sources": [
      "W-001",
      "W-058"
    ],
    "stages": [
      "strict",
      "without_heal_relief",
      "without_training",
      "legacy"
    ],
    "second_stage_party_size": 4,
    "note": "024b: keep the current quest ranking in layer2; drop training only in layer3 and prefer the lowest difficulty; layer4 calls the unchanged old selector. No source legality or class-tag bypass."
  }
}
"#)
    }
    pub fn parse(s: &str) -> Result<Self> {
        let p: Self = serde_json::from_str(s).map_err(|e| e.to_string())?;
        if !p.recovery.validate()
            || p.estate.recruit_levels != 2
            || p.estate.ordered_trees.is_empty()
            || !(3..=16).contains(&p.merchant.candidate_limit)
            || [1, 3].iter().any(|d| {
                !p.merchant.escort_power.contains_key(d)
                    || !p.merchant.escort_gear.contains_key(d)
                    || !p.merchant.escort_skills.contains_key(d)
            })
            || p.assets.weights.values().any(|v| !v.is_finite())
            || !(0. ..=1.).contains(&p.assets.stress_penalty_with_relief)
            || p.economy.poverty_gold == 0
            || p.economy.food_floor == 0
            || !p.provisions.validate()
            || !p.gates.validate()
            || !p.composition.validate()
            || p.revision == 0
            || p.profile != "candidate-preparation-024-v1"
            || p.group_defaults.len() != GROUPS.len()
            || GROUPS.iter().any(|g| !p.group_defaults.contains_key(*g))
        {
            return Err("024 profile/groups mismatch".into());
        }
        let mut ids = BTreeSet::new();
        for l in &p.loadouts {
            if !l.id.starts_with("P024-")
                || !ids.insert(&l.id)
                || l.sources.is_empty()
                || l.sources.iter().any(|w| !w.starts_with("W-"))
                || l.skills.len() != 4
                || l.skills.iter().collect::<BTreeSet<_>>().len() != 4
            {
                return Err("024 loadout lacks unique skills or provenance".into());
            }
        }
        for key in p.active_variants.values() {
            if p.alternatives
                .get(key)
                .and_then(|v| v.as_array())
                .is_none_or(|v| v.len() != 4 || v.iter().any(|s| s.as_str().is_none()))
            {
                return Err("024 invalid loadout variant".into());
            }
        }
        for key in [
            "skill_coverage",
            "damage",
            "healing",
            "stress_relief",
            "control",
            "recovery_mobility",
            "healing_presence",
            "relief_presence",
            "target_reach",
        ] {
            if p.role_weights
                .get(key)
                .is_none_or(|n| !n.is_finite() || *n < 0.)
            {
                return Err("024 invalid role weight".into());
            }
        }
        if p.power["weights"].as_array().is_none_or(|a| {
            a.len() != 4
                || a.iter()
                    .any(|v| v.as_f64().is_none_or(|n| !n.is_finite() || n < 0.))
                || a.iter().filter_map(|v| v.as_f64()).sum::<f64>() <= 0.
        }) {
            return Err("024 invalid power components".into());
        }
        Ok(p)
    }
    pub fn binding(&self, enabled: &BTreeSet<String>) -> Result<String> {
        use sha2::{Digest, Sha256};
        // Acceptance status is descriptive metadata, never a decision input.
        // Clearing blocked after verification must not invalidate the tested save.
        let mut execution = self.clone();
        execution.status.clear();
        Ok(format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&execution, enabled, "preparation-policy-024b-v1"))
                    .map_err(|e| e.to_string())?
            )
        ))
    }
    pub fn hash(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).map_err(|e| e.to_string())?)
        ))
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Ability {
    pub available_at_start: bool,
    pub conditional_healing: bool,
    pub launch: Vec<u8>,
    pub target: Vec<u8>,
    pub damage: f64,
    pub healing: f64,
    pub healing_percent: f64,
    pub stress_relief: f64,
    pub control: f64,
    pub guard: bool,
    pub bleed: bool,
    pub blight: bool,
    pub movement: bool,
}
/// Only skill-card facts are projected. No source objects, monsters, queues or RNG.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Catalog {
    #[serde(default)]
    pub cart_quote: Option<economy::Quote>,
    pub skills: BTreeMap<String, BTreeMap<String, Ability>>,
    pub fixed_skills: BTreeSet<String>,
    pub assets: BTreeMap<String, assets::Card>,
    pub crit_multiplier: f64,
    pub quirks: BTreeMap<String, estate::QuirkCard>,
}
fn effect(a: &mut Ability, e: &dd_data::combat::Effect) {
    let team = !matches!(
        e.target,
        EffectTarget::Performer | EffectTarget::ThisTrinket
    );
    match &e.kind {
        EffectKind::Composite(es) => {
            for e in es {
                effect(a, e)
            }
        }
        EffectKind::BattlePersistent(k) | EffectKind::InventoryEffect(k) => {
            let mut e = e.clone();
            e.kind = *k.clone();
            effect(a, &e)
        }
        EffectKind::Heal(n) if team => a.healing += f64::from(*n).max(0.),
        EffectKind::HealPercent(n) if team => a.healing_percent += n.max(0.),
        EffectKind::Regen(n) if team => a.healing += f64::from(*n).max(0.),
        EffectKind::Stress(n) if *n < 0 && team => a.stress_relief += f64::from(-*n),
        EffectKind::Stun
        | EffectKind::Pull(_)
        | EffectKind::Push(_)
        | EffectKind::Shuffle
        | EffectKind::ShuffleParty => a.control += e.chance,
        EffectKind::Guard { .. } => a.guard = true,
        EffectKind::Bleed(_) => a.bleed = true,
        EffectKind::Blight(_) => a.blight = true,
        _ => (),
    }
}
impl Catalog {
    pub fn project(game: &GameData) -> Result<Self> {
        let mut c = Self {
            crit_multiplier: game
                .rules
                .player_crit_multiplier_table
                .first()
                .ok_or("024 missing public critical multiplier")?
                .value,
            ..Default::default()
        };
        for class in game.hero_ids() {
            let mut skills = BTreeMap::new();
            for id in game.hero_skill_ids(class)? {
                let h = game.hero(class, 0, 0, &[&id])?;
                if !h.mechanics.hero.fixed_skills.is_empty() {
                    c.fixed_skills.insert(class.clone());
                }
                let s = h
                    .skills
                    .iter()
                    .find(|s| s.id == id)
                    .ok_or("024 skill projection missing")?;
                let starting_mode = h.mechanics.hero.initial_mode.as_deref().or_else(|| {
                    h.mechanics
                        .hero
                        .modes
                        .iter()
                        .find(|m| m.default)
                        .map(|m| m.id.as_str())
                });
                let mut a = Ability {
                    conditional_healing: s.tactical.hp_range.is_some_and(|range| range[1] < 1.),
                    available_at_start: s.tactical.valid_modes.is_empty()
                        || starting_mode.is_none_or(|mode| {
                            s.tactical.valid_modes.iter().any(|valid| valid == mode)
                        }),
                    launch: s.launch.clone(),
                    target: s.target.ranks.clone(),
                    damage: s.damage.map_or(0., |d| f64::from(d[0] + d[1]) / 2.),
                    healing: if s.target.friendly {
                        s.heal.map_or(0., |d| f64::from(d[0] + d[1]) / 2.)
                    } else {
                        0.
                    },
                    movement: s.move_forward > 0 || s.move_back > 0,
                    ..Default::default()
                };
                for e in &s.effects {
                    effect(&mut a, e)
                }
                skills.insert(id, a);
            }
            c.skills.insert(class.clone(), skills);
        }
        Ok(c)
    }
}
#[derive(Debug, Clone)]
pub struct Context {
    pub profile: Profile,
    pub enabled: BTreeSet<String>,
    pub catalog: Catalog,
    pub role_gaps: BTreeSet<String>,
}
impl Context {
    pub fn on(&self, group: &str) -> bool {
        self.enabled.contains(group)
    }
    pub fn length_priority(&self, q: &crate::campaign017::QuestView) -> u32 {
        if self.on("C")
            && q.plot.is_none()
            && self.profile.gates.ordinary_regions.contains(&q.dungeon)
        {
            self.profile
                .gates
                .length_priority
                .get(&q.length)
                .copied()
                .unwrap_or(q.length)
        } else {
            q.length
        }
    }
    pub fn desired_skills(&self, class: &str) -> Vec<String> {
        if let Some(l) = self.profile.loadouts.iter().find(|l| l.class_id == class) {
            let mut skills = if let Some(key) = self.profile.active_variants.get(class) {
                self.profile.alternatives[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap().to_string())
                    .collect()
            } else {
                l.skills.clone()
            };
            for c in &self.profile.conditional_loadouts {
                if c.class_id == class
                    && self.role_gaps.contains(&c.missing_role)
                    && c.unless_missing
                        .as_ref()
                        .is_none_or(|g| !self.role_gaps.contains(g))
                    && let Some(slot) = skills.iter_mut().find(|s| **s == c.replace)
                {
                    *slot = c.with_skill.clone()
                }
            }
            return skills;
        }
        self.profile
            .legacy_preferences
            .get(class)
            .cloned()
            .unwrap_or_default()
    }
    pub fn preferences(&self, class: &str) -> Vec<String> {
        if self.on("A") {
            self.desired_skills(class)
        } else {
            self.profile
                .legacy_preferences
                .get(class)
                .cloned()
                .unwrap_or_default()
        }
    }
    pub fn selected(&self, h: &VisibleHero) -> Vec<String> {
        if self.catalog.fixed_skills.contains(&h.class) {
            return h.selected.clone();
        }
        let preferences = self.preferences(&h.class);
        if preferences.is_empty() {
            return h.selected.clone();
        }
        let mut ids = preferences
            .into_iter()
            .filter(|id| h.skills.contains_key(id))
            .take(4)
            .collect::<Vec<_>>();
        for id in h.selected.iter().chain(h.skills.keys()) {
            if ids.len() < 4 && !ids.contains(id) {
                ids.push(id.clone())
            }
        }
        ids
    }
    pub fn learn_missing(&self, town: &Observation) -> Option<crate::town016::Action> {
        let reserve = self.profile.learning.reserve_gold;
        for h in town.heroes.iter().filter(|h| town.party.contains(&h.guid)) {
            for skill in self.desired_skills(&h.class) {
                if h.skills.contains_key(&skill) {
                    continue;
                }
                if let Some(c)=town.actions.iter().find(|c|matches!(&c.action,crate::town016::Action::Upgrade{hero:Some(id),tree,..} if *id==h.guid && *tree==format!("{}.{}",h.class,skill))
                    && c.cost.iter().all(|(key,n)|town.wallet.get(key).copied().unwrap_or(0).saturating_sub(if key=="gold:"{reserve}else{0})>=*n)) {return Some(c.action.clone())}
            }
        }
        None
    }
    pub fn rank_score(&self, h: &VisibleHero, rank: u8) -> f64 {
        let mut score = 0.;
        for id in self.selected(h) {
            if let Some(a) = self.catalog.skills.get(&h.class).and_then(|s| s.get(&id))
                && a.available_at_start
                && a.launch.contains(&rank)
            {
                let w = &self.profile.role_weights;
                score += w["skill_coverage"]
                    + a.damage * w["damage"] * (1. + a.target.len() as f64 * w["target_reach"])
                    + (a.healing + a.healing_percent * f64::from(h.stats.hp)) * w["healing"]
                    + a.stress_relief * w["stress_relief"]
                    + a.control * w["control"];
                if a.movement {
                    score += w["recovery_mobility"]
                }
            }
        }
        score
    }
    pub fn arrange(&self, town: &Observation, party: Vec<u64>) -> Vec<u64> {
        if !self.on("A") {
            return party;
        }
        self.arrange_known(town, party)
    }
    pub fn arrange_known(&self, town: &Observation, party: Vec<u64>) -> Vec<u64> {
        if party.len() < 2 {
            return party;
        }
        fn search(
            ctx: &Context,
            t: &Observation,
            remaining: &[u64],
            current: &mut Vec<u64>,
            best: &mut (f64, Vec<u64>),
        ) {
            if remaining.is_empty() {
                let score = current
                    .iter()
                    .enumerate()
                    .filter_map(|(i, id)| {
                        t.heroes
                            .iter()
                            .find(|h| h.guid == *id)
                            .map(|h| ctx.rank_score(h, (i + 1) as u8))
                    })
                    .sum::<f64>();
                if score > best.0 {
                    *best = (score, current.clone())
                }
                return;
            }
            for (i, id) in remaining.iter().enumerate() {
                let mut next = remaining.to_vec();
                next.remove(i);
                current.push(*id);
                search(ctx, t, &next, current, best);
                current.pop();
            }
        }
        let mut best = (f64::NEG_INFINITY, party.clone());
        search(self, town, &party, &mut vec![], &mut best);
        best.1
    }
    pub fn town_party(&self, town: &Observation) -> Vec<u64> {
        self.arrange(town, crate::campaign_policy017::party(town))
    }
    pub fn reorder(
        &self,
        town: &Observation,
        party: &[u64],
        actions: &[crate::opening015::Action],
    ) -> Option<crate::opening015::Action> {
        let score = |ids: &[u64]| {
            ids.iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    town.heroes
                        .iter()
                        .find(|h| h.guid == *id)
                        .map(|h| self.rank_score(h, (i + 1) as u8))
                })
                .sum::<f64>()
        };
        let current = score(party);
        actions
            .iter()
            .filter_map(|a| {
                if let crate::opening015::Action::Reorder(ids) = a {
                    Some((score(ids), a))
                } else {
                    None
                }
            })
            .filter(|(n, _)| *n > current)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, a)| a.clone())
    }
    pub fn power(&self, h: &VisibleHero) -> f64 {
        let weights = self.profile.power["weights"].as_array().unwrap();
        strength(h)
            .iter()
            .zip(weights)
            .map(|(n, w)| n * w.as_f64().unwrap())
            .sum::<f64>()
            / weights.iter().map(|w| w.as_f64().unwrap()).sum::<f64>()
    }
    pub fn considering(&self, town: &Observation) -> Self {
        let mut ctx = self.clone();
        ctx.role_gaps.clear();
        let party = if town.party.is_empty() {
            crate::campaign_policy017::party(town)
        } else {
            town.party.clone()
        };
        let mut healing = 0f64;
        let mut relief = 0f64;
        for h in town.heroes.iter().filter(|h| party.contains(&h.guid)) {
            for s in ctx.selected(h) {
                if let Some(a) = ctx.catalog.skills.get(&h.class).and_then(|sks| sks.get(&s))
                    && a.available_at_start
                {
                    healing = healing.max(a.healing + a.healing_percent * f64::from(h.stats.hp));
                    relief = relief.max(a.stress_relief)
                }
            }
        }
        if healing < self.profile.role_weights["healing_presence"] {
            ctx.role_gaps.insert("healing".into());
        }
        if relief < self.profile.role_weights["relief_presence"] {
            ctx.role_gaps.insert("stress_relief".into());
        }
        ctx
    }
}
/// A summary, not a simulator stat. Every component comes from the visible roster.
pub fn strength(h: &VisibleHero) -> [f64; 4] {
    let skill = if h.selected.is_empty() {
        0.
    } else {
        h.selected
            .iter()
            .map(|s| f64::from(*h.skills.get(s).unwrap_or(&0)))
            .sum::<f64>()
            / h.selected.len() as f64
    };
    [f64::from(h.level), h.weapon as f64, h.armour as f64, skill]
}
