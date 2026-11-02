//! Independent public-observation pipeline and a single estate ledger.
use crate::{
    auto_policy018::{self, Action},
    campaign017::{Observation, ProvisionAction},
    town016::Action as TownAction,
};
use dd_data::combat::{GameData, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[path = "boss025.rs"]
pub mod boss;
#[path = "caretaker025.rs"]
pub mod caretaker;
#[path = "combat025.rs"]
pub mod combat;
#[path = "dungeon025.rs"]
pub mod dungeon;
#[path = "heirlooms025.rs"]
pub mod heirlooms;
#[path = "ledger025.rs"]
pub mod ledger;
#[path = "loadout025.rs"]
pub mod loadout;
#[path = "metrics025.rs"]
pub mod metrics;
#[path = "policy025.rs"]
pub mod policy;
#[path = "recovery025.rs"]
pub mod recovery;
#[path = "sync026.rs"]
pub mod sync026;
#[path = "threat025.rs"]
pub mod threat;
#[path = "town025.rs"]
pub mod town;
#[path = "training025.rs"]
pub mod training;
#[path = "usage025.rs"]
pub mod usage;
pub const PROFILE: &str = "candidate-strategy-025-v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub sources: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub group: String,
    pub sources: Vec<String>,
    pub note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: String,
    pub revision: u32,
    pub status: String,
    pub groups: Vec<Group>,
    pub rules: BTreeMap<String, Rule>,
    pub town_order: Vec<String>,
    pub legacy_order: Vec<String>,
    pub ledger: ledger::Settings,
    pub modes: BTreeMap<String, Value>,
    pub parameters: Value,
    #[serde(skip)]
    pub boss_parties: boss::Catalog,
}
impl Profile {
    pub fn documented() -> Result<Self> {
        Self::parse(r#"{
  "version": "candidate-strategy-025-v1",
  "revision": 14,
  "status": "unverified integrated025 candidate; fixed before phase6; no tuning from outcomes",
  "groups": [
    {
      "id": "growth",
      "label": "育成",
      "sources": [
        "W-018",
        "W-019",
        "W-020",
        "W-082",
        "W-083",
        "W-084",
        "W-087",
        "W-091"
      ]
    },
    {
      "id": "skills",
      "label": "技能站位",
      "sources": [
        "W-006",
        "W-007",
        "W-047",
        "W-065",
        "W-066",
        "W-067",
        "W-068",
        "W-069",
        "W-070",
        "W-071",
        "W-072",
        "W-073",
        "W-074",
        "W-075",
        "W-076"
      ]
    },
    {
      "id": "income",
      "label": "上限赚钱",
      "sources": [
        "W-019",
        "W-043",
        "W-056",
        "W-057",
        "W-060",
        "W-061",
        "W-085",
        "W-094",
        "W-095",
        "W-100",
        "W-103"
      ]
    },
    {
      "id": "stress",
      "label": "刷压",
      "sources": [
        "W-008",
        "W-011",
        "W-014",
        "W-040",
        "W-055",
        "W-059",
        "W-088",
        "W-089",
        "W-090",
        "W-101"
      ]
    },
    {
      "id": "poverty",
      "label": "缺钱",
      "sources": [
        "W-015",
        "W-043",
        "W-058",
        "W-062",
        "W-063"
      ]
    },
    {
      "id": "trinkets",
      "label": "饰品",
      "sources": [
        "W-024",
        "W-025",
        "W-026",
        "W-027",
        "W-034",
        "W-077",
        "W-078",
        "W-079"
      ]
    },
    {
      "id": "quirks",
      "label": "怪癖",
      "sources": [
        "W-022",
        "W-049",
        "W-050",
        "W-051",
        "W-086"
      ]
    },
    {
      "id": "party",
      "label": "配队",
      "sources": [
        "W-001",
        "W-002",
        "W-013",
        "W-032",
        "W-033",
        "W-041",
        "W-042",
        "W-046"
      ]
    },
    {
      "id": "exploration",
      "label": "探索和背包",
      "sources": [
        "W-036",
        "W-037",
        "W-038",
        "W-039",
        "W-044",
        "W-045",
        "W-053",
        "W-064"
      ]
    },
    {
      "id": "town",
      "label": "城镇建设",
      "sources": [
        "W-003",
        "W-016",
        "W-017",
        "W-021",
        "W-023",
        "W-030",
        "W-031"
      ]
    },
    {
      "id": "combat",
      "label": "战斗决策",
      "sources": [
        "W-004",
        "W-005",
        "W-007",
        "W-010",
        "W-048",
        "W-098",
        "W-099"
      ]
    },
    {
      "id": "boss",
      "label": "Boss",
      "sources": [
        "W-009",
        "W-012",
        "W-054",
        "W-102"
      ]
    },
    {
      "id": "cohorts",
      "label": "长期和梯队",
      "sources": [
        "W-028",
        "W-052",
        "W-081",
        "W-096",
        "W-097"
      ]
    }
  ],
  "rules": {
    "S025-00": {
      "group": "baseline",
      "sources": [
        "W-104"
      ],
      "note": "unchanged baseline fallback; no disabled-group hits"
    },
    "S025-01": {
      "group": "party",
      "sources": [
        "W-001",
        "W-058"
      ],
      "note": "public executable party and task plan"
    },
    "S025-02": {
      "group": "town",
      "sources": [
        "W-016",
        "W-017",
        "W-021",
        "W-022",
        "W-030",
        "W-031",
        "W-092"
      ],
      "note": "roster-wide facilities and source-priced heirloom exchange"
    },
    "S025-03": {
      "group": "town",
      "sources": [
        "W-016",
        "W-096"
      ],
      "note": "recruitment and successors"
    },
    "S025-04": {
      "group": "growth",
      "sources": [
        "W-018",
        "W-020",
        "W-082",
        "W-084",
        "W-087"
      ],
      "note": "selected party only: weapon, armour, core skills"
    },
    "S025-05": {
      "group": "skills",
      "sources": [
        "W-006",
        "W-047",
        "W-065",
        "W-066",
        "W-067",
        "W-068",
        "W-069",
        "W-070",
        "W-071",
        "W-072",
        "W-073",
        "W-074",
        "W-075",
        "W-076"
      ],
      "note": "learn and use selected skills"
    },
    "S025-06": {
      "group": "trinkets",
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
      "note": "physical equipment allocation"
    },
    "S025-07": {
      "group": "poverty",
      "sources": [
        "W-058",
        "W-063"
      ],
      "note": "idle asset funding and affordable supplies"
    },
    "S025-08": {
      "group": "quirks",
      "sources": [
        "W-022",
        "W-049",
        "W-050"
      ],
      "note": "fatal negative quirks"
    },
    "S025-09": {
      "group": "quirks",
      "sources": [
        "W-051",
        "W-086",
        "W-103"
      ],
      "note": "positive quirk locking after essential spending"
    },
    "S025-10": {
      "group": "growth",
      "sources": [
        "W-083",
        "W-091"
      ],
      "note": "off-party key hero relief"
    },
    "S025-11": {
      "group": "exploration",
      "sources": [
        "W-053"
      ],
      "note": "one quoted provision plan"
    },
    "S025-12": {
      "group": "growth",
      "sources": [
        "W-091"
      ],
      "note": "dismissal investment loss compared with treatment"
    },
    "S025-13": {
      "group": "income",
      "sources": [
        "W-019",
        "W-085",
        "W-094",
        "W-095",
        "W-100",
        "W-103"
      ],
      "note": "risk-adjusted task income and progression"
    },
    "S025-14": {
      "group": "order",
      "sources": [
        "W-083"
      ],
      "note": "town order"
    },
    "S025-15": {
      "group": "cohorts",
      "sources": [
        "W-052",
        "W-081",
        "W-096"
      ],
      "note": "eligibility windows and successor cohorts"
    },
    "S025-16": {
      "group": "cohorts",
      "sources": [
        "W-097"
      ],
      "note": "two-trip visible forward plan"
    },
    "S025-17": {
      "group": "boss",
      "sources": [
        "W-009",
        "W-012",
        "W-102"
      ],
      "note": "functional boss party and first-round safety"
    },
    "S025-18": {
      "group": "stress",
      "sources": [
        "W-088",
        "W-089",
        "W-090",
        "W-101"
      ],
      "note": "average and worst-member stress risk"
    },
    "S025-19": {
      "group": "combat",
      "sources": [
        "W-098",
        "W-099"
      ],
      "note": "visible-enemy threat assessment"
    },
    "S025-20": {
      "group": "exploration",
      "sources": [
        "W-036",
        "W-037",
        "W-038",
        "W-039",
        "W-088"
      ],
      "note": "early camp, scouting and meals"
    },
    "S025-21": {
      "group": "income",
      "sources": [
        "W-057",
        "W-094",
        "W-100"
      ],
      "note": "post-objective visible-route greed and champion exit"
    },
    "S025-22": {
      "group": "income",
      "sources": [
        "W-043",
        "W-050",
        "W-056"
      ],
      "note": "source curio item and antiquarian operator"
    },
    "S025-23": {
      "group": "exploration",
      "sources": [
        "W-060",
        "W-061",
        "W-062",
        "W-064"
      ],
      "note": "route-aware finite inventory"
    },
    "S025-24": {
      "group": "poverty",
      "sources": [
        "W-015",
        "W-058"
      ],
      "note": "legal recovery retreat"
    },
    "S025-25": {
      "group": "trinkets",
      "sources": [
        "W-035"
      ],
      "note": "existing equipment and food rounding"
    },
    "S025-26": {
      "group": "cohorts",
      "sources": [
        "W-028",
        "W-095"
      ],
      "note": "apprentice altar opportunity"
    },
    "S025-27": {
      "group": "exploration",
      "sources": [
        "W-044",
        "W-045",
        "W-095",
        "user-2026-11-02 restore old safety/navigation retreat"
      ],
      "note": "visible nearest frontier and secret stash"
    },
    "S025-30": {
      "group": "combat",
      "sources": [
        "W-005",
        "W-099"
      ],
      "note": "compare legal rescue, guard and movement"
    },
    "S025-31": {
      "group": "combat",
      "sources": [
        "W-005",
        "W-068"
      ],
      "note": "clear lethal visible DOT"
    },
    "S025-32": {
      "group": "combat",
      "sources": [
        "W-004",
        "W-007",
        "W-048",
        "W-098"
      ],
      "note": "visible threat reduction; backline only ties"
    },
    "S025-33": {
      "group": "combat",
      "sources": [
        "W-004",
        "W-007"
      ],
      "note": "source control probability candidate"
    },
    "S025-34": {
      "group": "combat",
      "sources": [
        "W-010",
        "W-070",
        "W-071",
        "W-074",
        "W-076"
      ],
      "note": "class recovery and finisher positions"
    },
    "S025-35": {
      "group": "stress",
      "sources": [
        "W-008",
        "W-011",
        "W-014",
        "W-040",
        "W-055",
        "W-089",
        "W-090"
      ],
      "note": "low threat recovery with source-invalidating corpse attacks"
    },
    "S025-36": {
      "group": "boss",
      "sources": [
        "W-009",
        "W-012",
        "W-102"
      ],
      "note": "functional boss harm reduction"
    },
    "S025-37": {
      "group": "combat",
      "sources": [
        "W-005",
        "W-099"
      ],
      "note": "guard move heal landing comparison"
    },
    "S025-38": {
      "group": "combat",
      "sources": [
        "W-015",
        "W-099",
        "W-105"
      ],
      "note": "visible multiple death-door legal retreat"
    },
    "S025-39": {
      "group": "combat",
      "sources": [
        "W-007",
        "W-009",
        "W-098"
      ],
      "note": "source hostile damage reduction competes with visible damage and control; self penalties excluded"
    }
  },
  "town_order": [
    "events",
    "unload",
    "facilities",
    "recruit",
    "plan",
    "skills",
    "training",
    "trinkets",
    "sell",
    "quirks",
    "recovery",
    "supplies"
  ],
  "legacy_order": [
    "events",
    "recruit",
    "recovery",
    "quirks",
    "facilities",
    "plan",
    "training",
    "skills",
    "trinkets",
    "sell",
    "supplies"
  ],
  "ledger": {
    "early_through_week": 13,
    "early_reserve": 3000,
    "later_reserve": 6000,
    "pending_supply_reserve": 2350,
    "poverty_gold": 2350,
    "sources": [
      "W-058",
      "W-082",
      "W-083",
      "W-084"
    ]
  },
  "modes": {
    "radiant:lit": {
      "champion_gear": 3,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "radiant:dark": {
      "champion_gear": 4,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "base:lit": {
      "champion_gear": 3,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "base:dark": {
      "champion_gear": 4,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "new_game_plus:lit": {
      "champion_gear": 4,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "new_game_plus:dark": {
      "champion_gear": 4,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "bloodmoon:lit": {
      "champion_gear": 4,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    },
    "bloodmoon:dark": {
      "champion_gear": 4,
      "champion_min_level": 4,
      "champion_after_week": 26,
      "average_stress_short": 50,
      "average_stress_camp": 60,
      "worst_stress": 99,
      "minimum_controls_high_stress": 2,
      "unscouted_hp_fraction": 0.5,
      "retreat_hp_fraction": 0.25
    }
  },
  "parameters": {
    "candidate_basis": "User experience and adopted videos; no threshold fitting. Bloodmoon lit stays conservatively gear4 pending confirmation.",
    "legacy_supply_targets": {
      "food": {
        "1": 12,
        "2": 24,
        "default": 48
      },
      "torch": {
        "1": 8,
        "default": 16
      },
      "shovel": {
        "1": 2,
        "default": 4
      },
      "bandage": {
        "1": 1,
        "default": 3
      },
      "medicinal_herbs": {
        "1": 1,
        "default": 3
      },
      "skeleton_key": {
        "default": 2
      }
    },
    "supplies": {
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
    "skill_core": {
      "crusader": [
        "holy_lance",
        "smite"
      ],
      "man_at_arms": [
        "crush",
        "defender",
        "rampart"
      ],
      "occultist": [
        "hands_from_abyss",
        "wyrd_reconstruction",
        "bloodlet"
      ],
      "highwayman": [
        "duelist_advance",
        "wicked_slice"
      ],
      "plague_doctor": [
        "blinding_gas",
        "battlefield_medicine",
        "noxious_blast"
      ],
      "houndmaster": [
        "hounds_rush",
        "howl"
      ],
      "arbalest": [
        "battlefield_bandage",
        "sniper_shot"
      ],
      "leper": [
        "chop",
        "solemnity"
      ],
      "antiquarian": [
        "kris_stab",
        "protect_me"
      ],
      "abomination": [
        "manacles",
        "absolution"
      ],
      "vestal": [
        "divine_grace",
        "gods_comfort"
      ],
      "jester": [
        "inspiring_tune",
        "battle_ballad"
      ],
      "shieldbreaker": [
        "serpents_sway",
        "pierce"
      ],
      "flagellant": [
        "rain_of_sorrows",
        "punish",
        "reclaim"
      ],
      "hellion": [
        "wicked_hack",
        "iron_swan"
      ],
      "bounty_hunter": [
        "collect_bounty",
        "come_hither"
      ],
      "grave_robber": [
        "lunge",
        "thrown_dagger"
      ]
    },
    "protected_trinkets": [
      "focus_ring",
      "fortifying_garlic"
    ],
    "ordinary_exclude_quirks": [
      "shard_hungry"
    ],
    "legacy_supply_overrides": {
      "courtyard": {
        "torch": 4
      },
      "weald": {
        "antivenom": 2
      }
    },
    "town": {
      "facility_order": [
        "stage_coach.numrecruits",
        "stage_coach.rostersize",
        "blacksmith.weapon",
        "blacksmith.armour",
        "guild.skill_levels",
        "sanitarium.cost",
        "sanitarium.slots"
      ],
      "recruit_tiers": 2,
      "roster_tiers": 3,
      "roster_target": 16,
      "key_classes": [
        "occultist",
        "vestal",
        "man_at_arms",
        "plague_doctor"
      ],
      "lock_quirks": [
        "natural_swing",
        "natural_eye",
        "quick_reflexes",
        "tough",
        "hard_skinned",
        "hippocratic",
        "clotter",
        "thick_blooded"
      ],
      "lock_level": 2,
      "power_weight": 5,
      "stress_weight": 0.2,
      "healer_slot_bonus": 8,
      "training_cost_weight": 0.001,
      "cheap_power_penalty": 8,
      "farm_level": 6,
      "farm_gear": 4,
      "veteran_gear": 2,
      "veteran_training_plan": "W-111: complete source-priced dependencies, shared facilities once, existing reserves; missing/locked/resolve failures reject; no future income",
      "apprentice_gear": 1,
      "veteran_min_level": 2,
      "native_goal_bonus": 1000,
      "main_goal_bonus": 500,
      "veteran_bonus": 150,
      "mixed_veteran_bonus": 0,
      "champion_bonus": 200,
      "medium_bonus": 40,
      "long_bonus": 20,
      "farm_penalty": 500,
      "poverty_short_bonus": 3000,
      "poverty_other_penalty": 3000,
      "window_bonus": 20,
      "xp_gold": 1175,
      "positive_quirk_value": 500,
      "negative_quirk_value": 1500,
      "recovery_stress": 60,
      "dismiss_stress": 80,
      "fallback_recovery_quote": 1500,
      "recovery_per_visit": 45,
      "sources": [
        "W-083",
        "W-084",
        "W-091",
        "W-096",
        "W-106"
      ],
      "pending": [
        "bank/district payoff not quantified; conservatively deferred",
        "rebuild uses recorded investment and public next quote bound; no assumed historic discount"
      ],
      "medium_min_heal": 1,
      "medium_min_relief": 1,
      "medium_min_controls_without_relief": 2,
      "veteran_required_level3": 2,
      "template_bonus": 5,
      "focus_reward_bonus": 100,
      "region_weight": 2
    },
    "party_weights": {
      "damage": 1,
      "healing": 8,
      "relief": 6,
      "control": 20,
      "coverage": 10
    },
    "quirk_settings": {
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
    "skills_sources": [
      "W-065..W-079",
      "W-087"
    ],
    "sale_sources": [
      "W-058",
      "W-063",
      "W-077",
      "W-079"
    ],
    "dungeon": {
      "camp_after_rooms": 2,
      "camp_stress": 50,
      "camp_low_hp": 0.5,
      "camp_healthy_hp": 0.8,
      "recovery_retreat_hp": 0.25,
      "low_health_members": 2,
      "afflictions_stop": 2,
      "food_reserve_per_hero": 1,
      "altar_level": 2,
      "altar_torch": 1,
      "bag_values": {
        "quest_item": 100000,
        "estate_currency": 9000,
        "estate": 8000,
        "gold": 1,
        "gem": 10,
        "trinket": 1000,
        "journal_page": -1,
        "provision": 300,
        "supply": 200,
        "firewood": 10000,
        "default": 100
      },
      "sources": [
        "W-028",
        "W-035..W-039",
        "W-094",
        "W-095",
        "W-100"
      ],
      "pending": [
        "No estimate of unscouted battle count before Boss; no invented three-battle timing",
        "Shieldbreaker early camp deferred without publicly demonstrated preparation; crisis camp remains legal",
        "Deferred chest revisit uses visible settled frontier; no hidden rooms inspected"
      ],
      "camp_scout_weight": 100,
      "camp_ambush_weight": 12,
      "camp_stress_resist_weight": 20,
      "camp_nightmare_resist_weight": 100,
      "camp_skills": {
        "houndmaster": [
          "release_the_hound",
          "therapy_dog",
          "mans_best_friend"
        ],
        "bounty_hunter": [
          "scout_ahead"
        ],
        "grave_robber": [
          "night_steps",
          "gallows_humor"
        ],
        "highwayman": [
          "gallows_humor"
        ],
        "jester": [
          "every_rose",
          "turn_back_time"
        ]
      },
      "shieldbreaker_pep_talk": "pep_talk",
      "legacy_exit": {
        "visited_rooms": 4,
        "stress": 160,
        "source": "user restore of existing auto_policy018/024: walked at least once, four visited rooms, unfinished, legal leave; stress160 or deaths door or food below party count"
      }
    },
    "combat": {
      "damage_weight": 1,
      "stress_weight": 0.35,
      "support_weight": 5,
      "lethal_weight": 30,
      "threat_reduction_weight": 2,
      "heal_weight": 1.5,
      "rescue_bonus": 120,
      "guard_weight": 30,
      "move_weight": 20,
      "control_weight": 1,
      "backline_tie": 0.001,
      "dot_dead_attack_penalty": 30,
      "tail_enemy_limit": 2,
      "tail_hp_fraction": 0.35,
      "tail_stress_limit": 10,
      "tail_max_round": 8,
      "recovery_streak_limit": 3,
      "corpse_invalidating_bonus": 8,
      "tail_recovery_bonus": 15,
      "self_heal_hp_fraction": 0.5,
      "finisher_after_round": 3,
      "combo_bonus": 8,
      "early_buff_rounds": 2,
      "early_buff_bonus": 4,
      "transform_enemy_count": 3,
      "transform_safe_stress": 50,
      "transform_bonus": 5,
      "boss_debuff_bonus": 15,
      "pending": [
        "damage envelopes ignore exact action scheduling and proc chains; conservative analytic candidate",
        "stun uses source base resistance and skill chance; dynamic Buff amounts not reconstructed",
        "S29 reset claim not adopted; source stall state stays private",
        "Boss fixed teams W054 not implemented; functional roles only"
      ]
    },
    "court_gate": {
      "sources": [
        "W-003",
        "W-019",
        "W-101",
        "user-2026-11-01 scope correction; preparation-024.json gates"
      ],
      "group": "income",
      "minimum": {
        "1": 3,
        "3": 4,
        "5": 5,
        "6": 5
      },
      "gear": {
        "1": 3,
        "3": 4,
        "5": 4,
        "6": 4
      },
      "note": "Eligibility precedes goal priority; hard current level/weapon/armour gate, also on functional recovery. Existing024 candidate units preserved.",
      "intro": {
        "plot": "plot_crimson_court_0",
        "minimum_levels": [
          2,
          2,
          1,
          1
        ],
        "gear_follows_own_level": true,
        "maximum_gear": 4,
        "minimum_heal": 1.0,
        "minimum_relief": 1.0,
        "minimum_control": 1.0,
        "sources": [
          "W-107 user 2026-11-01",
          "W-001 public functional roles"
        ],
        "note": "2211 and stronger includes2221; roles must be available at the assigned initial ranks. No additional bleed/HP rule inferred."
      }
    },
    "regional_boss_gate": {
      "status": "candidate awaiting user confirmation",
      "sources": [
        "user-2026-11-01 post-baseline correction"
      ],
      "regions": [
        "crypts",
        "warrens",
        "weald",
        "cove"
      ],
      "plot_prefix": "plot_kill_",
      "minimum_levels": {
        "1": [
          2,
          2,
          1,
          1
        ],
        "3": [
          3,
          3,
          2,
          2
        ],
        "5": [
          4,
          4,
          4,
          4
        ]
      },
      "note": "Shared025 qualification before goal priority and functional fallback. Apprentice2211 newly requested; higher tiers retain existing025 all-on resolve gates. Existing equipment, core skill, stress and role checks unchanged. All-off bypass retains023."
    },
    "recovery_budget": {
      "sources": [
        "W-109 / user-2026-11-02 supersedes e218ed6: current training first; next-trip critical roles only"
      ],
      "note": "Stress Recovery restores 3000 through week13 / 6000 later plus current supplies. Only critical heroes in the explicit next-trip intent. Training, Quirks and Lock unchanged.",
      "working_gold_reserve": "same_as_training",
      "next_trip": "current visible board and off-party roster; minimum source treatment range; no future income; recheck actual gates next return",
      "facility_capacity": "conservative one currently legal free slot per facility"
    },
    "heirloom_exchange": {
      "mode": "complete_direct_native_surplus",
      "source": "user-2026-11-02: fund complete quote before any exchange; do not recycle converted stock",
      "credited_stock": "building and other payments consume credited stock first; native new income can be exchanged later",
      "loss_metric": "original public direct liquidation rates to crest; raw currencies also retained; metric is not used to choose trades"
    }
  }
}
"#)
    }
    pub fn parse(s: &str) -> Result<Self> {
        let mut p: Self = serde_json::from_str(s).map_err(|e| e.to_string())?;
        p.boss_parties = boss::Catalog::load()?;
        if p.version != PROFILE
            || p.groups.len() != 13
            || p.groups
                .iter()
                .map(|g| &g.id)
                .collect::<BTreeSet<_>>()
                .len()
                != 13
            || p.rules.values().any(|r| {
                r.sources.is_empty()
                    || r.group != "baseline"
                        && r.group != "order"
                        && !p.groups.iter().any(|g| g.id == r.group)
            })
        {
            return Err("invalid025 profile, groups, or rule provenance".into());
        }
        Ok(p)
    }
    pub fn binding(&self, enabled: &BTreeSet<String>, order: bool) -> Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    self,
                    enabled,
                    order,
                    caretaker::catalog()?,
                    r#"{
  "version": "visible-combat-025-v1",
  "threats": [],
  "safe_tail": [],
  "rules": [
    {
      "id": "S025-30",
      "priority": 30,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Rescue",
      "action": "Heal",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "compare legal rescue, guard and movement",
      "source": "W-005,W-099",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-31",
      "priority": 31,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Cleanse",
      "action": "Cleanse",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "clear lethal visible DOT",
      "source": "W-005,W-068",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-32",
      "priority": 32,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "visible threat reduction; backline only ties",
      "source": "W-004,W-007,W-048,W-098",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-33",
      "priority": 33,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Stun",
      "action": "Stun",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "source control probability candidate",
      "source": "W-004,W-007",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-34",
      "priority": 34,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "class recovery and finisher positions",
      "source": "W-010,W-070,W-071,W-074,W-076",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-35",
      "priority": 35,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Tail",
      "action": "Recover",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "low threat recovery with source-invalidating corpse attacks",
      "source": "W-008,W-011,W-014,W-040,W-055,W-089,W-090",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-36",
      "priority": 36,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "functional boss harm reduction",
      "source": "W-009,W-012,W-102",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-37",
      "priority": 37,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Heal",
      "action": "Heal",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "guard move heal landing comparison",
      "source": "W-005,W-099",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-38",
      "priority": 38,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Retreat",
      "action": "Retreat",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "multiple visible death-door heroes; legal retreat",
      "source": "W-015,W-099,W-105",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    },
    {
      "id": "S025-39",
      "priority": 39,
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "condition": "Focus",
      "action": "Attack",
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 60,
        "recovery_stress": 20,
        "max_round": 8,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65
      },
      "rationale": "hostile source damage reduction; exclude performer penalties",
      "source": "W-007,W-009,W-098",
      "applicable_version": "candidate025; source values from visible projection",
      "difficulty": "mode/light data in strategy025; radiant lit measured",
      "validation": "未验证"
    }
  ]
}
"#,
                    &self.boss_parties
                ))
                .map_err(|e| e.to_string())?
            )
        ))
    }
    pub fn enabled(&self, text: &str) -> Result<(BTreeSet<String>, bool)> {
        let mut enabled = if text == "none" {
            BTreeSet::new()
        } else {
            self.groups.iter().map(|g| g.id.clone()).collect()
        };
        let mut order = text != "none";
        if !["all", "none"].contains(&text) {
            if !text.starts_with("all,") {
                enabled.clear();
            }
            for g in text.split(',').filter(|g| *g != "all") {
                if g == "-order" {
                    order = false;
                    continue;
                }
                let name = g.trim_start_matches('-');
                if !self.groups.iter().any(|x| x.id == name) {
                    return Err(format!("unknown025 group {name}"));
                }
                if g.starts_with('-') {
                    enabled.remove(name);
                } else {
                    enabled.insert(name.into());
                }
            }
        }
        Ok((enabled, order))
    }
}
#[derive(Clone)]
pub struct Sources {
    pub field026: Option<std::sync::Arc<sync026::field::Facts>>,
    pub recovery_ranges: recovery::Ranges,
    pub training: std::sync::Arc<training::Catalog>,
    pub training_key: Vec<u8>,
    pub training_heirlooms: heirlooms::Account,
    pub cards: std::sync::Arc<crate::preparation024::Catalog>,
    pub mode: String,
    pub curios: BTreeMap<String, dd_data::field017::Curio>,
    pub enemies: std::sync::Arc<BTreeMap<String, threat::Enemy>>,
    pub invalidating: BTreeSet<(String, String)>,
    pub damage_debuff: BTreeSet<(String, String)>,
    pub self_recovery: combat::SelfRecovery,
    pub action_model: std::sync::Arc<combat::ActionModel>,
    pub tactics: std::sync::Arc<crate::tactics021::Document>,
    pub boss_checks: std::sync::Arc<BTreeMap<String, Value>>,
    pub current_boss: Option<(String, Vec<u64>)>,
}
impl Sources {
    pub fn refresh_visible(
        &mut self,
        game: &GameData,
        o: &Observation,
        p: &Profile,
        curios: &dd_data::curios013::Curios,
    ) -> Result<()> {
        if let Some(raid) = &o.expedition
            && let Some(name) = raid
                .map
                .as_ref()
                .and_then(|map| map.nodes.iter().find(|n| n.id == map.position))
                .and_then(|n| n.content.as_ref())
                .and_then(|c| c.curio_name.as_ref())
            && !self.curios.contains_key(name)
        {
            // Quest markers have separate host contracts, not a generic curio row.
            // Leave them to the unchanged legal host interaction path.
            let card = if curios.props.contains_key(name) {
                let raw =
                    dd_data::library::unique_json(&curios.props, name, "025 visible curio prop")?;
                let type_id = raw["row"][2]
                    .as_str()
                    .ok_or("025 visible curio lacks type reference")?;
                let objective = raid
                    .map
                    .as_ref()
                    .and_then(|m| m.nodes.iter().find(|n| n.id == m.position))
                    .and_then(|n| n.content.as_ref())
                    .is_some_and(|c| c.objective);
                if objective && !curios.types.contains_key(type_id) {
                    // The host has already exposed a quest-contract control.
                    // It is not a generic Curios table and is executed there.
                    None
                } else {
                    Some(curios.campaign_prop(name)?.1)
                }
            } else if curios.types.contains_key(name) {
                Some(curios.campaign_curio(name)?)
            } else {
                None
            };
            if let Some(card) = card {
                self.curios.insert(name.clone(), card);
            }
        }
        let battle = o
            .expedition
            .as_ref()
            .and_then(|r| r.battle.as_ref())
            .or_else(|| {
                o.content019
                    .as_ref()
                    .and_then(|c| c.farm.as_ref())
                    .map(|f| &f.battle)
            });
        if let Some(b) = battle {
            for u in b.units.iter().filter(|u| !u.hero && !u.corpse && u.hp > 0) {
                if u.controlled.is_some() {
                    std::sync::Arc::make_mut(&mut self.enemies)
                        .insert(threat::key(u), threat::controlled(game, o, u)?);
                } else if !self.enemies.contains_key(&u.class) {
                    std::sync::Arc::make_mut(&mut self.enemies)
                        .insert(u.class.clone(), threat::project(game, &u.class)?);
                }
            }
        }
        let cfg = serde_json::from_value(p.parameters["quirk_settings"].clone())
            .map_err(|e| e.to_string())?;
        if o.town
            .heroes
            .iter()
            .flat_map(|h| h.quirks.keys())
            .any(|id| !self.cards.quirks.contains_key(id))
        {
            crate::preparation024::estate::refresh(
                game,
                &o.town,
                &cfg,
                &mut std::sync::Arc::make_mut(&mut self.cards).quirks,
            )?;
        }
        for id in crate::preparation024::assets::visible_ids(o) {
            if !self.cards.assets.contains_key(&id) {
                std::sync::Arc::make_mut(&mut self.cards).assets.insert(
                    id.clone(),
                    crate::preparation024::assets::project(game, &id)?,
                );
            }
        }
        Ok(())
    }
    pub fn project(game: &GameData) -> Result<Self> {
        let (invalidating, damage_debuff) = combat::project_skills(game)?;
        Ok(Self {
            field026: None,
            recovery_ranges: BTreeMap::new(),
            training: std::sync::Arc::new(training::Catalog::default()),
            training_key: vec![],
            training_heirlooms: heirlooms::Account::default(),
            cards: std::sync::Arc::new(crate::preparation024::Catalog::project(game)?),
            mode: format!(
                "{}:{}",
                game.overlay.config.mode,
                if game.overlay.config.no_light {
                    "dark"
                } else {
                    "lit"
                }
            ),
            invalidating,
            damage_debuff,
            self_recovery: combat::project_self_recovery(game)?,
            action_model: std::sync::Arc::new(combat::project_action_model(game)?),
            tactics: std::sync::Arc::new(combat::document()?),
            boss_checks: std::sync::Arc::new(boss::checks(game, &boss::Catalog::load()?)?),
            current_boss: None,
            enemies: std::sync::Arc::new(BTreeMap::new()),
            curios: BTreeMap::new(),
        })
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    pub returns: Option<u32>,
    pub legacy: auto_policy018::Memory,
    pub ledger: ledger::Ledger,
    pub metrics: std::sync::Arc<metrics::Metrics>,
    #[serde(default)]
    pub usage: usage::Usage,
    pub hits: BTreeMap<String, u64>,
    pub stage: String,
    pub future_plan: Vec<Value>,
    pub state: BTreeMap<String, Value>,
    pub combat: combat::Memory,
    pub boss_attempts: BTreeMap<String, BTreeMap<String, u64>>,
    pub boss_last: BTreeMap<String, Value>,
}
pub struct Context<'a> {
    pub profile: &'a Profile,
    pub enabled: &'a BTreeSet<String>,
    pub ordered: bool,
    pub sources: &'a Sources,
}
impl Context<'_> {
    pub fn on(&self, id: &str) -> bool {
        self.enabled.contains(id)
    }
    pub fn rule(&self, id: &str) -> Result<String> {
        let r = self
            .profile
            .rules
            .get(id)
            .ok_or_else(|| format!("unknown025 rule {id}"))?;
        if r.group == "baseline" || r.group == "order" && self.ordered || self.on(&r.group) {
            Ok(id.into())
        } else {
            Err(format!("disabled025 rule proposed: {id}"))
        }
    }
}
#[derive(Clone)]
pub struct Proposal {
    pub action: Action,
    pub rule: String,
    pub stage: String,
    pub cost: u32,
    pub purpose: ledger::Purpose,
}
pub fn price(o: &Observation, a: &Action) -> Result<u32> {
    match a {
        Action::Town(
            TownAction::Party(_)
            | TownAction::SelectSkills { .. }
            | TownAction::SelectCamping { .. },
        ) => Ok(0),
        Action::Town(a) => o
            .town
            .actions
            .iter()
            .find(|c| &c.action == a)
            .map(|c| c.cost.get("gold:").copied().unwrap_or(0))
            .ok_or("025 action absent from public town choices".into()),
        Action::Provision(ProvisionAction::Buy { item, amount }) => {
            let p = o
                .provisions
                .as_ref()
                .ok_or("025 purchase without public provisions")?;
            if p.currencies019
                .as_ref()
                .and_then(|c| c.iter().find(|(i, _)| i == item))
                .is_some_and(|(_, c)| c != "gold:")
            {
                return Ok(0);
            }
            p.store
                .iter()
                .find(|(i, _, _)| i == item)
                .and_then(|(_, _, price)| price.checked_mul(*amount))
                .ok_or("025 missing/overflowed public shop price".into())
        }
        Action::Event(crate::campaign017::EventAction::Decline) => Ok(0),
        Action::Event(_) => Ok(o
            .event_offer
            .as_ref()
            .ok_or("025 event without public offer")?
            .cost
            .get("gold:")
            .copied()
            .unwrap_or(0)),
        _ => Ok(0),
    }
}
pub fn purpose(a: &Action) -> ledger::Purpose {
    use ledger::Purpose as P;
    match a {
        Action::Provision(_) => P::Supplies,
        Action::Town(TownAction::Upgrade { hero: Some(_), .. } | TownAction::LearnCamp { .. }) => {
            P::Training
        }
        Action::Town(TownAction::Queue {
            treatment: crate::town016::Treatment::Stress,
            ..
        }) => P::Recovery,
        Action::Town(TownAction::Queue {
            treatment: crate::town016::Treatment::LockQuirk(_),
            ..
        }) => P::Lock,
        Action::Town(TownAction::Queue { .. }) => P::Quirks,
        Action::Town(TownAction::Hire { .. }) | Action::Event(_) => P::Recruitment,
        _ => P::Buildings,
    }
}
pub fn proposal(
    o: &Observation,
    a: Action,
    id: &str,
    stage: &str,
    ctx: &Context<'_>,
) -> Result<Proposal> {
    Ok(Proposal {
        cost: price(o, &a)?,
        purpose: purpose(&a),
        action: a,
        rule: ctx.rule(id)?,
        stage: stage.into(),
    })
}


