//! Editable reviewed candidates. Only public quest identities can select a party.
use super::*;
use crate::{campaign017::QuestView, town016::VisibleHero};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: String,
    pub source: String,
    pub notes: Vec<String>,
    pub policy: Value,
    pub families: BTreeMap<String, Vec<String>>,
    pub plans: Vec<Plan>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub family: String,
    pub decision: String,
    pub user_note: String,
    pub source: String,
    pub basis: String,
    pub refs: Vec<String>,
    pub scope: String,
    pub forms: String,
    pub goal: String,
    pub raw_skills: String,
    pub raw_loadout: String,
    pub conditions: String,
    pub rule: String,
    pub risk: String,
    pub gear_advice: String,
    pub slots: Vec<Member>,
    pub unresolved: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub rank: u8,
    #[serde(rename = "class")]
    pub class_id: String,
    pub skills: Vec<String>,
    pub selection: String,
}
impl Catalog {
    pub fn load() -> Result<Self> {
        let text = if let Some(path) = std::env::var_os("DD_BOSS_PARTIES_025") {
            std::fs::read_to_string(path).map_err(|e| format!("025 boss data: {e}"))?
        } else {
            r#"{
  "version": "boss-parties-025-v1",
  "source": "W-105; boss-adopted-2026-11-01.md and boss-calibration-expanded.html; original hero IDs checked locally",
  "notes": [
    "rank1 is front; roster candidates only, not validated winning teams",
    "Unresolved English identifiers and ambiguous corrosion disable that plan; never guess",
    "Named minimum skills are combined with functional learned loadout, all source ranks rechecked at runtime",
    "User notes override source suggestions; wandering encounters are comparison only"
  ],
  "families": {
    "necro": [
      "plot_kill_necromancer_"
    ],
    "prophet": [
      "plot_kill_prophet_"
    ],
    "swine": [
      "plot_kill_swine_prince_"
    ],
    "flesh": [
      "plot_kill_formless_flesh_"
    ],
    "hag": [
      "plot_kill_hag_"
    ],
    "cannon": [
      "plot_kill_brigand_cannon_"
    ],
    "siren": [
      "plot_kill_siren_"
    ],
    "crew": [
      "plot_kill_drowned_crew_"
    ],
    "baron": [
      "plot_crimson_court_1"
    ],
    "viscount": [
      "plot_crimson_court_2"
    ],
    "countess": [
      "plot_crimson_court_3"
    ],
    "miller": [
      "plot_color_of_madness_1"
    ],
    "sleeper": [
      "plot_color_of_madness_2"
    ],
    "dd1": [
      "plot_darkest_dungeon_1"
    ],
    "templar": [
      "plot_darkest_dungeon_2"
    ],
    "cell": [
      "plot_darkest_dungeon_3"
    ],
    "heart": [
      "plot_darkest_dungeon_4"
    ],
    "vvulf": [
      "plot_town_invasion_0"
    ]
  },
  "plans": [
    {
      "id": "BP-necro-A-main",
      "family": "necro",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "N"
      ],
      "scope": "学徒",
      "forms": "necromancer_A",
      "goal": "boss:kill_necromancer_A",
      "raw_skills": "瘟疫：瘟疫手雷（plague_grenade）；强盗：手枪射击、决斗突刺；十字军：惩击，带后排圣矛作为乱位备选。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "本体后移 → 后排腐蚀与手枪继续打本体；普通召唤物妨碍输出或威胁失控时再处理。",
      "risk": "强盗突刺会换位；若反复贴前，要用零距离射击或手动复位，不能假设十字军每轮都能圣矛。",
      "gear_advice": "整理者建议：瘟疫加腐蚀成功与速度；输出补命中；修女保治疗。不是视频实装的两件饰品。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-necro-B-main",
      "family": "necro",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "N"
      ],
      "scope": "精英",
      "forms": "necromancer_B",
      "goal": "boss:kill_necromancer_B",
      "raw_skills": "瘟疫：瘟疫手雷（plague_grenade）；强盗：手枪射击、决斗突刺；十字军：惩击，带后排圣矛作为乱位备选。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "本体后移 → 后排腐蚀与手枪继续打本体；普通召唤物妨碍输出或威胁失控时再处理。",
      "risk": "强盗突刺会换位；若反复贴前，要用零距离射击或手动复位，不能假设十字军每轮都能圣矛。",
      "gear_advice": "整理者建议：瘟疫加腐蚀成功与速度；输出补命中；修女保治疗。不是视频实装的两件饰品。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-necro-C-main",
      "family": "necro",
      "decision": "采纳",
      "user_note": "死灵法师相对较简单 其他高强度阵容也可以尝试 但是尽量不要依赖出血dot",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "N"
      ],
      "scope": "冠军",
      "forms": "necromancer_C",
      "goal": "boss:kill_necromancer_C",
      "raw_skills": "瘟疫：瘟疫手雷（plague_grenade）；强盗：手枪射击、决斗突刺；十字军：惩击，带后排圣矛作为乱位备选。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "本体后移 → 后排腐蚀与手枪继续打本体；普通召唤物妨碍输出或威胁失控时再处理。",
      "risk": "强盗突刺会换位；若反复贴前，要用零距离射击或手动复位，不能假设十字军每轮都能圣矛。",
      "gear_advice": "整理者建议：瘟疫加腐蚀成功与速度；输出补命中；修女保治疗。不是视频实装的两件饰品。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-prophet-A-main",
      "family": "prophet",
      "decision": "采纳",
      "user_note": "",
      "source": "社区整队框架 · 站位由我安排",
      "basis": "社区整队框架 · 站位由我安排",
      "refs": [
        "P",
        "L45",
        "F16"
      ],
      "scope": "学徒",
      "forms": "prophet_A",
      "goal": "boss:kill_prophet_A",
      "raw_skills": "必带：麻风威慑（intimidate）、神秘虚弱诅咒（weakening_curse）；训犬带舔舐伤口、守护；弩手狙击，照明弹可作解晕备选。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "降攻可稳定叠加且首轮有保命 → 麻风和神秘维持降攻，其余打后排；断层时先保命。",
      "risk": "威慑须在1号位；降攻未命中、被晕和开局尚未叠起都可能致命；不承诺零伤害。",
      "gear_advice": "建议麻风/神秘补抗晕与减益成功，输出补命中；圣水、解毒剂与已有蛇鳞按库存准备。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "lick_wounds",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-prophet-A-guard",
      "family": "prophet",
      "decision": "采纳",
      "user_note": "双守护打普通 精英都可以 冠军最好用减攻阵容",
      "source": "我的备选配队推断",
      "basis": "我的备选配队推断",
      "refs": [
        "P"
      ],
      "scope": "学徒",
      "forms": "prophet_A",
      "goal": "boss:kill_prophet_A",
      "raw_skills": "必带老兵守护；恶女钢铁天鹅/流血攻击；狗猎犬冲锋与守护备援；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "落石目标可被安全守护 → 老兵承伤，狗和恶女打后排，修女维持血线；两个落石目标时另算移动或保命。",
      "risk": "守护者和被守护者同时处于受击区域时，不能假设守护能吞掉两次伤害；首轮守护前的出手顺序需要评估。",
      "gear_advice": "老兵防护、血量、抗晕；后排输出命中；若使用后排守护盾饰品，必须另改站位，不能直接套本行。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-prophet-B-main",
      "family": "prophet",
      "decision": "采纳",
      "user_note": "",
      "source": "社区整队框架 · 站位由我安排",
      "basis": "社区整队框架 · 站位由我安排",
      "refs": [
        "P",
        "L45",
        "F16"
      ],
      "scope": "精英",
      "forms": "prophet_B",
      "goal": "boss:kill_prophet_B",
      "raw_skills": "必带：麻风威慑（intimidate）、神秘虚弱诅咒（weakening_curse）；训犬带舔舐伤口、守护；弩手狙击，照明弹可作解晕备选。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "降攻可稳定叠加且首轮有保命 → 麻风和神秘维持降攻，其余打后排；断层时先保命。",
      "risk": "威慑须在1号位；降攻未命中、被晕和开局尚未叠起都可能致命；不承诺零伤害。",
      "gear_advice": "建议麻风/神秘补抗晕与减益成功，输出补命中；圣水、解毒剂与已有蛇鳞按库存准备。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "lick_wounds",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-prophet-B-guard",
      "family": "prophet",
      "decision": "采纳",
      "user_note": "",
      "source": "我的备选配队推断",
      "basis": "我的备选配队推断",
      "refs": [
        "P"
      ],
      "scope": "精英",
      "forms": "prophet_B",
      "goal": "boss:kill_prophet_B",
      "raw_skills": "必带老兵守护；恶女钢铁天鹅/流血攻击；狗猎犬冲锋与守护备援；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "落石目标可被安全守护 → 老兵承伤，狗和恶女打后排，修女维持血线；两个落石目标时另算移动或保命。",
      "risk": "守护者和被守护者同时处于受击区域时，不能假设守护能吞掉两次伤害；首轮守护前的出手顺序需要评估。",
      "gear_advice": "老兵防护、血量、抗晕；后排输出命中；若使用后排守护盾饰品，必须另改站位，不能直接套本行。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-prophet-C-main",
      "family": "prophet",
      "decision": "采纳",
      "user_note": "",
      "source": "社区整队框架 · 站位由我安排",
      "basis": "社区整队框架 · 站位由我安排",
      "refs": [
        "P",
        "L45",
        "F16"
      ],
      "scope": "冠军",
      "forms": "prophet_C",
      "goal": "boss:kill_prophet_C",
      "raw_skills": "必带：麻风威慑（intimidate）、神秘虚弱诅咒（weakening_curse）；训犬带舔舐伤口、守护；弩手狙击，照明弹可作解晕备选。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "降攻可稳定叠加且首轮有保命 → 麻风和神秘维持降攻，其余打后排；断层时先保命。",
      "risk": "威慑须在1号位；降攻未命中、被晕和开局尚未叠起都可能致命；不承诺零伤害。",
      "gear_advice": "建议麻风/神秘补抗晕与减益成功，输出补命中；圣水、解毒剂与已有蛇鳞按库存准备。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "lick_wounds",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-prophet-C-guard",
      "family": "prophet",
      "decision": "采纳",
      "user_note": "",
      "source": "我的备选配队推断",
      "basis": "我的备选配队推断",
      "refs": [
        "P"
      ],
      "scope": "冠军",
      "forms": "prophet_C",
      "goal": "boss:kill_prophet_C",
      "raw_skills": "必带老兵守护；恶女钢铁天鹅/流血攻击；狗猎犬冲锋与守护备援；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "落石目标可被安全守护 → 老兵承伤，狗和恶女打后排，修女维持血线；两个落石目标时另算移动或保命。",
      "risk": "守护者和被守护者同时处于受击区域时，不能假设守护能吞掉两次伤害；首轮守护前的出手顺序需要评估。",
      "gear_advice": "老兵防护、血量、抗晕；后排输出命中；若使用后排守护盾饰品，必须另改站位，不能直接套本行。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-swine-A-main",
      "family": "swine",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "S",
        "F11",
        "L35"
      ],
      "scope": "学徒",
      "forms": "swine_prince_A",
      "goal": "boss:kill_swine_prince_A",
      "raw_skills": "必带：弩手照明弹（flare）、麻风威慑、神秘虚弱诅咒；训犬猎犬冲锋、守护。关闭会误伤威尔伯的反击/群攻路线。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。精英/冠军威尔伯眩晕会干扰清标记；严格无光禁升光技能时照明弹路线不可用。",
      "rule": "清标记与叠降攻均具备 → 弩手先看是否需要清标，狗输出，麻风神秘抑制猪王；猪王活着时避开威尔伯。",
      "risk": "照明弹不能解除弩手自身的晕/标记；若被晕或慢于猪王，不能按已清标计算。照明弹会升光，严格无光禁升光技能时此方案不成立。",
      "gear_advice": "弩手重点抗晕，降攻手补抗晕与减益成功；狗的命中、伤害与饼干用于攻坚。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-swine-A-burst",
      "family": "swine",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位由我安排",
      "basis": "玩家自述整队 · 站位由我安排",
      "refs": [
        "R",
        "L35",
        "S"
      ],
      "scope": "学徒",
      "forms": "swine_prince_A",
      "goal": "boss:kill_swine_prince_A",
      "raw_skills": "必带弩手照明弹（flare）；狗猎犬冲锋；老兵守护、指挥（不带反击）；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。精英/冠军威尔伯眩晕会干扰清标记；严格无光禁升光技能时照明弹路线不可用。",
      "rule": "弩手能及时清标 → 狗打主输出，老兵保关键人并指挥，修女维持血线；不攻击威尔伯。",
      "risk": "狗在2号位不能减压；修女仅负责血量，需靠营地与道中安排压力。清标记失败仍要能接住重击。",
      "gear_advice": "弩手抗晕，狗命中/伤害与饼干；老兵防护；无光不能选照明弹时必须改方案。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-swine-B-main",
      "family": "swine",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "S",
        "F11",
        "L35"
      ],
      "scope": "精英",
      "forms": "swine_prince_B",
      "goal": "boss:kill_swine_prince_B",
      "raw_skills": "必带：弩手照明弹（flare）、麻风威慑、神秘虚弱诅咒；训犬猎犬冲锋、守护。关闭会误伤威尔伯的反击/群攻路线。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。精英/冠军威尔伯眩晕会干扰清标记；严格无光禁升光技能时照明弹路线不可用。",
      "rule": "清标记与叠降攻均具备 → 弩手先看是否需要清标，狗输出，麻风神秘抑制猪王；猪王活着时避开威尔伯。",
      "risk": "照明弹不能解除弩手自身的晕/标记；若被晕或慢于猪王，不能按已清标计算。照明弹会升光，严格无光禁升光技能时此方案不成立。",
      "gear_advice": "弩手重点抗晕，降攻手补抗晕与减益成功；狗的命中、伤害与饼干用于攻坚。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-swine-B-burst",
      "family": "swine",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位由我安排",
      "basis": "玩家自述整队 · 站位由我安排",
      "refs": [
        "R",
        "L35",
        "S"
      ],
      "scope": "精英",
      "forms": "swine_prince_B",
      "goal": "boss:kill_swine_prince_B",
      "raw_skills": "必带弩手照明弹（flare）；狗猎犬冲锋；老兵守护、指挥（不带反击）；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。精英/冠军威尔伯眩晕会干扰清标记；严格无光禁升光技能时照明弹路线不可用。",
      "rule": "弩手能及时清标 → 狗打主输出，老兵保关键人并指挥，修女维持血线；不攻击威尔伯。",
      "risk": "狗在2号位不能减压；修女仅负责血量，需靠营地与道中安排压力。清标记失败仍要能接住重击。",
      "gear_advice": "弩手抗晕，狗命中/伤害与饼干；老兵防护；无光不能选照明弹时必须改方案。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-swine-C-main",
      "family": "swine",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "S",
        "F11",
        "L35"
      ],
      "scope": "冠军",
      "forms": "swine_prince_C",
      "goal": "boss:kill_swine_prince_C",
      "raw_skills": "必带：弩手照明弹（flare）、麻风威慑、神秘虚弱诅咒；训犬猎犬冲锋、守护。关闭会误伤威尔伯的反击/群攻路线。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。精英/冠军威尔伯眩晕会干扰清标记；严格无光禁升光技能时照明弹路线不可用。",
      "rule": "清标记与叠降攻均具备 → 弩手先看是否需要清标，狗输出，麻风神秘抑制猪王；猪王活着时避开威尔伯。",
      "risk": "照明弹不能解除弩手自身的晕/标记；若被晕或慢于猪王，不能按已清标计算。照明弹会升光，严格无光禁升光技能时此方案不成立。",
      "gear_advice": "弩手重点抗晕，降攻手补抗晕与减益成功；狗的命中、伤害与饼干用于攻坚。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-swine-C-burst",
      "family": "swine",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位由我安排",
      "basis": "玩家自述整队 · 站位由我安排",
      "refs": [
        "R",
        "L35",
        "S"
      ],
      "scope": "冠军",
      "forms": "swine_prince_C",
      "goal": "boss:kill_swine_prince_C",
      "raw_skills": "必带弩手照明弹（flare）；狗猎犬冲锋；老兵守护、指挥（不带反击）；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。精英/冠军威尔伯眩晕会干扰清标记；严格无光禁升光技能时照明弹路线不可用。",
      "rule": "弩手能及时清标 → 狗打主输出，老兵保关键人并指挥，修女维持血线；不攻击威尔伯。",
      "risk": "狗在2号位不能减压；修女仅负责血量，需靠营地与道中安排压力。清标记失败仍要能接住重击。",
      "gear_advice": "弩手抗晕，狗命中/伤害与饼干；老兵防护；无光不能选照明弹时必须改方案。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-flesh-A-main",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "F"
      ],
      "scope": "学徒",
      "forms": "formless_guard_A, formless_melee_A, formless_ranged_A, formless_weak_A",
      "goal": "boss:kill_formless_flesh_A",
      "raw_skills": "必带：猎犬骚扰（hounds_harry）；瘟疫手雷、战地医疗；苦修惩罚、救赎（redeem）/重生（reclaim）按解锁选择。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "多部位共享血量 → 狗群体流血、瘟疫后排腐蚀叠伤，修女保血，苦修处理前排并备急救。",
      "risk": "训犬在2号位不能群体减压，恢复期需先移回后排再用；瘟疫手雷和战地医疗须保持3/4号位。苦修恢复技能有次数与血线限制，不能当无限奶。",
      "gear_advice": "狗与苦修补流血成功；瘟疫补腐蚀成功和速度；保治疗与防暴击连续击穿。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_harry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-flesh-A-video",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "UP 口述整队 · 站位由我安排",
      "basis": "UP 口述整队 · 站位由我安排",
      "refs": [
        "L17"
      ],
      "scope": "学徒",
      "forms": "formless_guard_A, formless_melee_A, formless_ranged_A, formless_weak_A",
      "goal": "boss:kill_formless_flesh_A",
      "raw_skills": "瘟疫手雷/战地医疗；咒缚者人形控制与自疗、变身输出；老兵守护；苦修救赎/重生。前置道中与 Boss 的技能、变身次数另核。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "接受没有修女的恢复结构 → 瘟疫后排 DOT、苦修前排流血与恢复，老兵保人，咒缚者按需要控场或输出。",
      "risk": "原视频只是出征规划，未核实其完整 Boss 战、饰品和站位；无狗群体流血，不能把 W-012 的收益移植过来。跨难度适用仅候选。",
      "gear_advice": "晕率、DOT 成功与治疗条件；恢复余量及营地减压优先于只堆输出。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "abomination",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-flesh-B-main",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "F"
      ],
      "scope": "精英",
      "forms": "formless_guard_B, formless_melee_B, formless_ranged_B, formless_weak_B",
      "goal": "boss:kill_formless_flesh_B",
      "raw_skills": "必带：猎犬骚扰（hounds_harry）；瘟疫手雷、战地医疗；苦修惩罚、救赎（redeem）/重生（reclaim）按解锁选择。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "多部位共享血量 → 狗群体流血、瘟疫后排腐蚀叠伤，修女保血，苦修处理前排并备急救。",
      "risk": "训犬在2号位不能群体减压，恢复期需先移回后排再用；瘟疫手雷和战地医疗须保持3/4号位。苦修恢复技能有次数与血线限制，不能当无限奶。",
      "gear_advice": "狗与苦修补流血成功；瘟疫补腐蚀成功和速度；保治疗与防暴击连续击穿。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_harry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-flesh-B-video",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "UP 口述整队 · 站位由我安排",
      "basis": "UP 口述整队 · 站位由我安排",
      "refs": [
        "L17"
      ],
      "scope": "精英",
      "forms": "formless_guard_B, formless_melee_B, formless_ranged_B, formless_weak_B",
      "goal": "boss:kill_formless_flesh_B",
      "raw_skills": "瘟疫手雷/战地医疗；咒缚者人形控制与自疗、变身输出；老兵守护；苦修救赎/重生。前置道中与 Boss 的技能、变身次数另核。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "接受没有修女的恢复结构 → 瘟疫后排 DOT、苦修前排流血与恢复，老兵保人，咒缚者按需要控场或输出。",
      "risk": "原视频只是出征规划，未核实其完整 Boss 战、饰品和站位；无狗群体流血，不能把 W-012 的收益移植过来。跨难度适用仅候选。",
      "gear_advice": "晕率、DOT 成功与治疗条件；恢复余量及营地减压优先于只堆输出。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "abomination",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-flesh-C-main",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "F"
      ],
      "scope": "冠军",
      "forms": "formless_guard_C, formless_melee_C, formless_ranged_C, formless_weak_C",
      "goal": "boss:kill_formless_flesh_C",
      "raw_skills": "必带：猎犬骚扰（hounds_harry）；瘟疫手雷、战地医疗；苦修惩罚、救赎（redeem）/重生（reclaim）按解锁选择。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "多部位共享血量 → 狗群体流血、瘟疫后排腐蚀叠伤，修女保血，苦修处理前排并备急救。",
      "risk": "训犬在2号位不能群体减压，恢复期需先移回后排再用；瘟疫手雷和战地医疗须保持3/4号位。苦修恢复技能有次数与血线限制，不能当无限奶。",
      "gear_advice": "狗与苦修补流血成功；瘟疫补腐蚀成功和速度；保治疗与防暴击连续击穿。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_harry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-flesh-C-video",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "UP 口述整队 · 站位由我安排",
      "basis": "UP 口述整队 · 站位由我安排",
      "refs": [
        "L17"
      ],
      "scope": "冠军",
      "forms": "formless_guard_C, formless_melee_C, formless_ranged_C, formless_weak_C",
      "goal": "boss:kill_formless_flesh_C",
      "raw_skills": "瘟疫手雷/战地医疗；咒缚者人形控制与自疗、变身输出；老兵守护；苦修救赎/重生。前置道中与 Boss 的技能、变身次数另核。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "接受没有修女的恢复结构 → 瘟疫后排 DOT、苦修前排流血与恢复，老兵保人，咒缚者按需要控场或输出。",
      "risk": "原视频只是出征规划，未核实其完整 Boss 战、饰品和站位；无狗群体流血，不能把 W-012 的收益移植过来。跨难度适用仅候选。",
      "gear_advice": "晕率、DOT 成功与治疗条件；恢复余量及营地减压优先于只堆输出。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "abomination",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-hag-A-main",
      "family": "hag",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "H",
        "R"
      ],
      "scope": "学徒",
      "forms": "hag_A",
      "goal": "boss:kill_hag_A",
      "raw_skills": "恶女：流血攻击（if_it_bleeds）、钢铁天鹅（iron_swan）；强盗：手枪/决斗突刺；神秘：治疗；弩手：狙击、战地绷带作第二个急救来源。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "准备好三人仍能打后排 → 以击杀女巫为主；有人出锅到死门时评估即时治疗，主输出被抓导致输出瘫痪时再考虑救锅。",
      "risk": "恶女钢铁天鹅仅1号位可用；人被抓后的前移必须改用仍合法技能。弩手出锅落前排时需要复位。不要把一直不救锅当硬规则。 有人仍在锅里时撤退会丢下该英雄，不能把它当普通撤退。",
      "gear_advice": "输出补命中、伤害，治疗考虑出手速度；备绷带和已有蛇鳞。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-hag-B-main",
      "family": "hag",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "H",
        "R"
      ],
      "scope": "精英",
      "forms": "hag_B",
      "goal": "boss:kill_hag_B",
      "raw_skills": "恶女：流血攻击（if_it_bleeds）、钢铁天鹅（iron_swan）；强盗：手枪/决斗突刺；神秘：治疗；弩手：狙击、战地绷带作第二个急救来源。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "准备好三人仍能打后排 → 以击杀女巫为主；有人出锅到死门时评估即时治疗，主输出被抓导致输出瘫痪时再考虑救锅。",
      "risk": "恶女钢铁天鹅仅1号位可用；人被抓后的前移必须改用仍合法技能。弩手出锅落前排时需要复位。不要把一直不救锅当硬规则。 有人仍在锅里时撤退会丢下该英雄，不能把它当普通撤退。",
      "gear_advice": "输出补命中、伤害，治疗考虑出手速度；备绷带和已有蛇鳞。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-hag-C-main",
      "family": "hag",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "H",
        "R"
      ],
      "scope": "冠军",
      "forms": "hag_C",
      "goal": "boss:kill_hag_C",
      "raw_skills": "恶女：流血攻击（if_it_bleeds）、钢铁天鹅（iron_swan）；强盗：手枪/决斗突刺；神秘：治疗；弩手：狙击、战地绷带作第二个急救来源。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "准备好三人仍能打后排 → 以击杀女巫为主；有人出锅到死门时评估即时治疗，主输出被抓导致输出瘫痪时再考虑救锅。",
      "risk": "恶女钢铁天鹅仅1号位可用；人被抓后的前移必须改用仍合法技能。弩手出锅落前排时需要复位。不要把一直不救锅当硬规则。 有人仍在锅里时撤退会丢下该英雄，不能把它当普通撤退。",
      "gear_advice": "输出补命中、伤害，治疗考虑出手速度；备绷带和已有蛇鳞。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-cannon-A-main",
      "family": "cannon",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "B"
      ],
      "scope": "学徒",
      "forms": "brigand_cannon_A",
      "goal": "boss:kill_brigand_cannon_A",
      "raw_skills": "必带后排覆盖：瘟疫致盲毒气（blinding_gas）作无伤害控制；强盗手枪射击；恶女钢铁天鹅/流血攻击覆盖不同位置。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "每轮先阻止火柴手行动 → 能杀就杀，不能稳定击杀则控住；确认无开炮风险才打炮身。",
      "risk": "神秘前排晕够不到4号位；不能把它当后排兜底。炮体抗性高，DOT/减防不能想当然有效。",
      "gear_advice": "火柴手处理者优先命中；瘟疫补晕率；不能用一次暴击才够杀的伤害当稳定门槛。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-cannon-B-main",
      "family": "cannon",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "B"
      ],
      "scope": "精英",
      "forms": "brigand_cannon_B",
      "goal": "boss:kill_brigand_cannon_B",
      "raw_skills": "必带后排覆盖：瘟疫致盲毒气（blinding_gas）作无伤害控制；强盗手枪射击；恶女钢铁天鹅/流血攻击覆盖不同位置。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "每轮先阻止火柴手行动 → 能杀就杀，不能稳定击杀则控住；确认无开炮风险才打炮身。",
      "risk": "神秘前排晕够不到4号位；不能把它当后排兜底。炮体抗性高，DOT/减防不能想当然有效。",
      "gear_advice": "火柴手处理者优先命中；瘟疫补晕率；不能用一次暴击才够杀的伤害当稳定门槛。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-cannon-C-main",
      "family": "cannon",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "B"
      ],
      "scope": "冠军",
      "forms": "brigand_cannon_C",
      "goal": "boss:kill_brigand_cannon_C",
      "raw_skills": "必带后排覆盖：瘟疫致盲毒气（blinding_gas）作无伤害控制；强盗手枪射击；恶女钢铁天鹅/流血攻击覆盖不同位置。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "每轮先阻止火柴手行动 → 能杀就杀，不能稳定击杀则控住；确认无开炮风险才打炮身。",
      "risk": "神秘前排晕够不到4号位；不能把它当后排兜底。炮体抗性高，DOT/减防不能想当然有效。",
      "gear_advice": "火柴手处理者优先命中；瘟疫补晕率；不能用一次暴击才够杀的伤害当稳定门槛。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-siren-A-main",
      "family": "siren",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SI",
        "F15"
      ],
      "scope": "学徒",
      "forms": "siren_A",
      "goal": "boss:kill_siren_A",
      "raw_skills": "临战检查已装备技能：古商闪光粉/卧倒/防身；神秘治疗与标记；狗猎犬冲锋；破盾穿刺。卸下不准备让敌方使用的高风险技能。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "把低危险诱导位作为概率帮助 → 三人也能继续输出与治疗；魅惑主力时先保人，减少误伤队友。",
      "risk": "不能保证只抓古商；神秘可能给塞壬大奶、古商可能给敌方增益。破盾移动可把神秘挤离前排，控场技能需重新检查。",
      "gear_advice": "可用圣水或减益抗性装备调整其他人的被抓倾向；破盾补命中与血量。只记优先属性，不补造具体饰品。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "antiquarian",
          "skills": [
            "flashpowder",
            "cower"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-siren-B-main",
      "family": "siren",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SI",
        "F15"
      ],
      "scope": "精英",
      "forms": "siren_B",
      "goal": "boss:kill_siren_B",
      "raw_skills": "临战检查已装备技能：古商闪光粉/卧倒/防身；神秘治疗与标记；狗猎犬冲锋；破盾穿刺。卸下不准备让敌方使用的高风险技能。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "把低危险诱导位作为概率帮助 → 三人也能继续输出与治疗；魅惑主力时先保人，减少误伤队友。",
      "risk": "不能保证只抓古商；神秘可能给塞壬大奶、古商可能给敌方增益。破盾移动可把神秘挤离前排，控场技能需重新检查。",
      "gear_advice": "可用圣水或减益抗性装备调整其他人的被抓倾向；破盾补命中与血量。只记优先属性，不补造具体饰品。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "antiquarian",
          "skills": [
            "flashpowder",
            "cower"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-siren-C-main",
      "family": "siren",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SI",
        "F15"
      ],
      "scope": "冠军",
      "forms": "siren_C",
      "goal": "boss:kill_siren_C",
      "raw_skills": "临战检查已装备技能：古商闪光粉/卧倒/防身；神秘治疗与标记；狗猎犬冲锋；破盾穿刺。卸下不准备让敌方使用的高风险技能。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "把低危险诱导位作为概率帮助 → 三人也能继续输出与治疗；魅惑主力时先保人，减少误伤队友。",
      "risk": "不能保证只抓古商；神秘可能给塞壬大奶、古商可能给敌方增益。破盾移动可把神秘挤离前排，控场技能需重新检查。",
      "gear_advice": "可用圣水或减益抗性装备调整其他人的被抓倾向；破盾补命中与血量。只记优先属性，不补造具体饰品。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "antiquarian",
          "skills": [
            "flashpowder",
            "cower"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-crew-A-main",
      "family": "crew",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "C"
      ],
      "scope": "学徒",
      "forms": "drowned_captain_A",
      "goal": "boss:kill_drowned_crew_A",
      "raw_skills": "十字军：圣矛、惩击；神秘：深渊之手、治疗；赏金：收集赏金/终结者；瘟疫：腐蚀、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "锚手已成功绑人 → 优先集中直伤解锚，再打本体；锚手尚未抛锚时避免无意义削血被回满。",
      "risk": "被绑英雄仍在时不能直接撤退，否则会丢下他；后排被拉到1号位时先检查合法技能。此候选缺稳定战中减压，依赖快速解锚及营地。 十字军圣矛前进会把神秘挤到3号位，届时深渊之手不可用；须复位或改用治疗/标记。",
      "gear_advice": "前排控制补晕率、输出补命中与爆发；解毒与绷带按道中需要准备。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty",
            "finish_him"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "ambiguous corrosion description; no invented plague skill"
      ]
    },
    {
      "id": "BP-crew-B-main",
      "family": "crew",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "C"
      ],
      "scope": "精英",
      "forms": "drowned_captain_B",
      "goal": "boss:kill_drowned_crew_B",
      "raw_skills": "十字军：圣矛、惩击；神秘：深渊之手、治疗；赏金：收集赏金/终结者；瘟疫：腐蚀、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "锚手已成功绑人 → 优先集中直伤解锚，再打本体；锚手尚未抛锚时避免无意义削血被回满。",
      "risk": "被绑英雄仍在时不能直接撤退，否则会丢下他；后排被拉到1号位时先检查合法技能。此候选缺稳定战中减压，依赖快速解锚及营地。 十字军圣矛前进会把神秘挤到3号位，届时深渊之手不可用；须复位或改用治疗/标记。",
      "gear_advice": "前排控制补晕率、输出补命中与爆发；解毒与绷带按道中需要准备。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty",
            "finish_him"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "ambiguous corrosion description; no invented plague skill"
      ]
    },
    {
      "id": "BP-crew-C-main",
      "family": "crew",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "C"
      ],
      "scope": "冠军",
      "forms": "drowned_captain_C",
      "goal": "boss:kill_drowned_crew_C",
      "raw_skills": "十字军：圣矛、惩击；神秘：深渊之手、治疗；赏金：收集赏金/终结者；瘟疫：腐蚀、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "锚手已成功绑人 → 优先集中直伤解锚，再打本体；锚手尚未抛锚时避免无意义削血被回满。",
      "risk": "被绑英雄仍在时不能直接撤退，否则会丢下他；后排被拉到1号位时先检查合法技能。此候选缺稳定战中减压，依赖快速解锚及营地。 十字军圣矛前进会把神秘挤到3号位，届时深渊之手不可用；须复位或改用治疗/标记。",
      "gear_advice": "前排控制补晕率、输出补命中与爆发；解毒与绷带按道中需要准备。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty",
            "finish_him"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "ambiguous corrosion description; no invented plague skill"
      ]
    },
    {
      "id": "BP-collector-A-main",
      "family": "collector",
      "decision": "待观察",
      "user_note": "收藏家是游荡boss 是遭遇战 可能出现在任何副本 参考价值不大",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "CO"
      ],
      "scope": "学徒",
      "forms": "collector_A",
      "goal": "boss:collector_A",
      "raw_skills": "神秘：深渊之手、恶魔牵引、治疗；瘟疫：致盲毒气/手雷；狗猎犬冲锋；恶女钢铁天鹅/流血攻击。神秘在2号位兼顾拉前与晕。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "首轮有控制机会 → 控本体后集火；本体已退后 → 继续打后排，强盗头威胁失控时转控/转火。",
      "risk": "首轮先手与眩晕都不能保证；不要把没召唤出的未来头阵提前当可见信息。",
      "gear_advice": "前排神秘补速度和晕率；后排输出补命中，避免脆皮死门后被连击。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull",
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-collector-B-main",
      "family": "collector",
      "decision": "待观察",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "CO"
      ],
      "scope": "精英",
      "forms": "collector_B",
      "goal": "boss:collector_B",
      "raw_skills": "神秘：深渊之手、恶魔牵引、治疗；瘟疫：致盲毒气/手雷；狗猎犬冲锋；恶女钢铁天鹅/流血攻击。神秘在2号位兼顾拉前与晕。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "首轮有控制机会 → 控本体后集火；本体已退后 → 继续打后排，强盗头威胁失控时转控/转火。",
      "risk": "首轮先手与眩晕都不能保证；不要把没召唤出的未来头阵提前当可见信息。",
      "gear_advice": "前排神秘补速度和晕率；后排输出补命中，避免脆皮死门后被连击。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull",
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-collector-C-main",
      "family": "collector",
      "decision": "待观察",
      "user_note": "有控的时候评估风险更可控 但是最好也模拟一下判断是否要打完 控制战损 如果大概率打不过 减员 还是撤退",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "CO"
      ],
      "scope": "冠军",
      "forms": "collector_C",
      "goal": "boss:collector_C",
      "raw_skills": "神秘：深渊之手、恶魔牵引、治疗；瘟疫：致盲毒气/手雷；狗猎犬冲锋；恶女钢铁天鹅/流血攻击。神秘在2号位兼顾拉前与晕。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "首轮有控制机会 → 控本体后集火；本体已退后 → 继续打后排，强盗头威胁失控时转控/转火。",
      "risk": "首轮先手与眩晕都不能保证；不要把没召唤出的未来头阵提前当可见信息。",
      "gear_advice": "前排神秘补速度和晕率；后排输出补命中，避免脆皮死门后被连击。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull",
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shambler-A-main",
      "family": "shambler",
      "decision": "条件采纳",
      "user_note": "爬爬主要是看乱位对阵容的影响情况和稳定输出",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SH",
        "L19"
      ],
      "scope": "学徒",
      "forms": "shambler_A",
      "goal": "boss:shambler_A",
      "raw_skills": "神秘治疗/标记；十字军圣矛、战斗治疗；强盗决斗突刺、手枪、零距离；破盾穿刺、蛇舞。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。不可自动外推农庄 E 变体。",
      "rule": "乱位后优先维持可用输出与恢复 → 及时清不断强化的触手，再寻找本体斩杀窗口。",
      "risk": "开局和战中乱位使表中仅为目标站位。触手不随本体死去而消失；低级控制方案不可直接外推冠军或农庄E。",
      "gear_advice": "输出命中与穿甲路线；前排脆角色备保命。黑暗中不把高光饰品加成算进去。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "serpents_sway"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shambler-A-control",
      "family": "shambler",
      "decision": "条件采纳",
      "user_note": "",
      "source": "字幕局部＋我的配队推断",
      "basis": "字幕局部＋我的配队推断",
      "refs": [
        "L19",
        "SH"
      ],
      "scope": "学徒",
      "forms": "shambler_A",
      "goal": "boss:shambler_A",
      "raw_skills": "神秘深渊之手、治疗；瘟疫致盲毒气/迷乱冲击、战地医疗；狗猎犬冲锋/目标口哨；苦修惩罚、救赎。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。不可自动外推农庄 E 变体。",
      "rule": "学徒/精英的控制成功率和乱位恢复尚可接受 → 神秘与瘟疫控制，狗/苦修输出并准备恢复；不能控制就改快杀或退出。",
      "risk": "被打乱后控制/治疗会失效；两名DOT手可能无法及时清触手。老陈只给出局部路线，完整四人为我补齐。",
      "gear_advice": "核心晕率与速度、DOT 成功；必须保瘟疫在后排，不能只按初始站位计算。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "whistle"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "blinding_gas",
            "disorienting_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shambler-B-main",
      "family": "shambler",
      "decision": "条件采纳",
      "user_note": "冠军爬很难打 看阵容状态",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SH",
        "L19"
      ],
      "scope": "精英",
      "forms": "shambler_B",
      "goal": "boss:shambler_B",
      "raw_skills": "神秘治疗/标记；十字军圣矛、战斗治疗；强盗决斗突刺、手枪、零距离；破盾穿刺、蛇舞。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。不可自动外推农庄 E 变体。",
      "rule": "乱位后优先维持可用输出与恢复 → 及时清不断强化的触手，再寻找本体斩杀窗口。",
      "risk": "开局和战中乱位使表中仅为目标站位。触手不随本体死去而消失；低级控制方案不可直接外推冠军或农庄E。",
      "gear_advice": "输出命中与穿甲路线；前排脆角色备保命。黑暗中不把高光饰品加成算进去。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "serpents_sway"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shambler-B-control",
      "family": "shambler",
      "decision": "条件采纳",
      "user_note": "",
      "source": "字幕局部＋我的配队推断",
      "basis": "字幕局部＋我的配队推断",
      "refs": [
        "L19",
        "SH"
      ],
      "scope": "精英",
      "forms": "shambler_B",
      "goal": "boss:shambler_B",
      "raw_skills": "神秘深渊之手、治疗；瘟疫致盲毒气/迷乱冲击、战地医疗；狗猎犬冲锋/目标口哨；苦修惩罚、救赎。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。不可自动外推农庄 E 变体。",
      "rule": "学徒/精英的控制成功率和乱位恢复尚可接受 → 神秘与瘟疫控制，狗/苦修输出并准备恢复；不能控制就改快杀或退出。",
      "risk": "被打乱后控制/治疗会失效；两名DOT手可能无法及时清触手。老陈只给出局部路线，完整四人为我补齐。",
      "gear_advice": "核心晕率与速度、DOT 成功；必须保瘟疫在后排，不能只按初始站位计算。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "whistle"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "blinding_gas",
            "disorienting_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shambler-C-main",
      "family": "shambler",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SH",
        "L19"
      ],
      "scope": "冠军",
      "forms": "shambler_C",
      "goal": "boss:shambler_C",
      "raw_skills": "神秘治疗/标记；十字军圣矛、战斗治疗；强盗决斗突刺、手枪、零距离；破盾穿刺、蛇舞。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。不可自动外推农庄 E 变体。",
      "rule": "乱位后优先维持可用输出与恢复 → 及时清不断强化的触手，再寻找本体斩杀窗口。",
      "risk": "开局和战中乱位使表中仅为目标站位。触手不随本体死去而消失；低级控制方案不可直接外推冠军或农庄E。",
      "gear_advice": "输出命中与穿甲路线；前排脆角色备保命。黑暗中不把高光饰品加成算进去。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "serpents_sway"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shambler-E-main",
      "family": "shambler",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SH",
        "L19"
      ],
      "scope": "极暗·农庄变体",
      "forms": "shambler_E",
      "goal": "boss:shambler_E",
      "raw_skills": "神秘治疗/标记；十字军圣矛、战斗治疗；强盗决斗突刺、手枪、零距离；破盾穿刺、蛇舞。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。E 是农庄单独变体，无本轮逐变体实战样本。",
      "rule": "乱位后优先维持可用输出与恢复 → 及时清不断强化的触手，再寻找本体斩杀窗口。",
      "risk": "开局和战中乱位使表中仅为目标站位。触手不随本体死去而消失；低级控制方案不可直接外推冠军或农庄E。",
      "gear_advice": "输出命中与穿甲路线；前排脆角色备保命。黑暗中不把高光饰品加成算进去。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "serpents_sway"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shrieker-D-main",
      "family": "shrieker",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SR"
      ],
      "scope": "极暗·农庄变体",
      "forms": "crow_D",
      "goal": "boss:crow_D",
      "raw_skills": "小丑战歌（battle_ballad）；狗猎犬冲锋/守护；老兵指挥（command）、守护，必要时反击。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。D 为农庄单独目标；本轮没有这变体的阵容实战，只是外推候选。",
      "rule": "本次目标是击杀本体 → 加命中后集中攻击，狗饼干用于窗口，老兵保护危险人；不把活过任务当击杀。",
      "risk": "缺稳定队友治疗，只有狗自疗与守护；有减员风险时停止追杀。若只回收饰品可走四轮生存，但那不满足项目击杀目标。",
      "gear_advice": "命中优先于只堆伤害；按库存给狗输出装。双狗是普通游戏合法重复职业。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "jester",
          "skills": [
            "battle_ballad"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shrieker-A-main",
      "family": "shrieker",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SR"
      ],
      "scope": "学徒",
      "forms": "crow_A",
      "goal": "boss:kill_crow_A",
      "raw_skills": "小丑战歌（battle_ballad）；狗猎犬冲锋/守护；老兵指挥（command）、守护，必要时反击。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "本次目标是击杀本体 → 加命中后集中攻击，狗饼干用于窗口，老兵保护危险人；不把活过任务当击杀。",
      "risk": "缺稳定队友治疗，只有狗自疗与守护；有减员风险时停止追杀。若只回收饰品可走四轮生存，但那不满足项目击杀目标。",
      "gear_advice": "命中优先于只堆伤害；按库存给狗输出装。双狗是普通游戏合法重复职业。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "jester",
          "skills": [
            "battle_ballad"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shrieker-B-main",
      "family": "shrieker",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SR"
      ],
      "scope": "精英",
      "forms": "crow_B",
      "goal": "boss:kill_crow_B",
      "raw_skills": "小丑战歌（battle_ballad）；狗猎犬冲锋/守护；老兵指挥（command）、守护，必要时反击。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "本次目标是击杀本体 → 加命中后集中攻击，狗饼干用于窗口，老兵保护危险人；不把活过任务当击杀。",
      "risk": "缺稳定队友治疗，只有狗自疗与守护；有减员风险时停止追杀。若只回收饰品可走四轮生存，但那不满足项目击杀目标。",
      "gear_advice": "命中优先于只堆伤害；按库存给狗输出装。双狗是普通游戏合法重复职业。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "jester",
          "skills": [
            "battle_ballad"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-shrieker-C-main",
      "family": "shrieker",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SR"
      ],
      "scope": "冠军",
      "forms": "crow_C",
      "goal": "boss:kill_crow_C",
      "raw_skills": "小丑战歌（battle_ballad）；狗猎犬冲锋/守护；老兵指挥（command）、守护，必要时反击。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "本次目标是击杀本体 → 加命中后集中攻击，狗饼干用于窗口，老兵保护危险人；不把活过任务当击杀。",
      "risk": "缺稳定队友治疗，只有狗自疗与守护；有减员风险时停止追杀。若只回收饰品可走四轮生存，但那不满足项目击杀目标。",
      "gear_advice": "命中优先于只堆伤害；按库存给狗输出装。双狗是普通游戏合法重复职业。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "jester",
          "skills": [
            "battle_ballad"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-vvulf-D-main",
      "family": "vvulf",
      "decision": "采纳",
      "user_note": "这个是固定队好像 刷这个事件最好凑齐神装这队 不然不好打",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "V"
      ],
      "scope": "极暗",
      "forms": "brigand_sapper_D",
      "goal": "boss:kill_brigand_sapper_D",
      "raw_skills": "必带老兵守护（defender）；小丑战歌、振奋旋律；恶女钢铁天鹅/流血攻击；修女单疗/群疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。",
      "rule": "炸弹落在其他人位置 → 老兵守护目标并承伤；其余人持续打沃尔夫，修女维护老兵血线。",
      "risk": "本任务无营火，不能把营地增益算入出征门槛。炸弹不能靠狗守护的闪避收益替代老兵防护。撤离有牺牲风险。",
      "gear_advice": "老兵防护、血量、抗晕与速度；修女治疗；输出命中。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-fanatic-A-main",
      "family": "fanatic",
      "decision": "条件采纳",
      "user_note": "狂信者最好不要打 这个东西强度很高 特别是冠军本 可以堆闪避+反击来打",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "FA"
      ],
      "scope": "学徒",
      "forms": "fanatic_A",
      "goal": "boss:fanatic_A",
      "raw_skills": "强盗决斗突刺/手枪；老兵反击（retribution）、守护；苦修惩罚/苦痛之雨、救赎；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "开局乱位先修复技能可用性 → 用流血与反击推进，随时准备接出火刑架的英雄。",
      "risk": "修女被抓或挤至最前会断奶。拆火刑架会改变 Boss 行为，不能当免费救人；有人仍在架上时撤退会丢下他。",
      "gear_advice": "命中、流血成功和治疗，保护治疗者避免乱位后长时间无法施法。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "retribution"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-fanatic-B-main",
      "family": "fanatic",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "FA"
      ],
      "scope": "精英",
      "forms": "fanatic_B",
      "goal": "boss:fanatic_B",
      "raw_skills": "强盗决斗突刺/手枪；老兵反击（retribution）、守护；苦修惩罚/苦痛之雨、救赎；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "开局乱位先修复技能可用性 → 用流血与反击推进，随时准备接出火刑架的英雄。",
      "risk": "修女被抓或挤至最前会断奶。拆火刑架会改变 Boss 行为，不能当免费救人；有人仍在架上时撤退会丢下他。",
      "gear_advice": "命中、流血成功和治疗，保护治疗者避免乱位后长时间无法施法。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "retribution"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-fanatic-C-main",
      "family": "fanatic",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "FA"
      ],
      "scope": "冠军",
      "forms": "fanatic_C",
      "goal": "boss:fanatic_C",
      "raw_skills": "强盗决斗突刺/手枪；老兵反击（retribution）、守护；苦修惩罚/苦痛之雨、救赎；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "开局乱位先修复技能可用性 → 用流血与反击推进，随时准备接出火刑架的英雄。",
      "risk": "修女被抓或挤至最前会断奶。拆火刑架会改变 Boss 行为，不能当免费救人；有人仍在架上时撤退会丢下他。",
      "gear_advice": "命中、流血成功和治疗，保护治疗者避免乱位后长时间无法施法。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "retribution"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-thing-A-main",
      "family": "thing",
      "decision": "条件采纳",
      "user_note": "这个是游荡boss 同爬",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "TH"
      ],
      "scope": "学徒",
      "forms": "thing_A",
      "goal": "boss:thing_A",
      "raw_skills": "盗墓：当面一击（pick_to_the_face）、影遁；破盾穿刺（pierce）；小丑战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "目标后半获得高防护 → 用穿甲直伤缩短高危段，先确保结晶不会爆，再继续打本体。",
      "risk": "双输出位移可能互相挤位；需按速度安排。若要改 DOT 队必须重算清 DOT 时点；本表未采用需严格控速的慢晕技巧。",
      "gear_advice": "两名脆输出补命中和存活；不把跨回合被清掉的 DOT 算作完整收益。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "grave_robber",
          "skills": [
            "pick_to_the_face",
            "shadow_fade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "source English identifier not present in these original classes: pick_to_the_face"
      ]
    },
    {
      "id": "BP-thing-B-main",
      "family": "thing",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "TH"
      ],
      "scope": "精英",
      "forms": "thing_B",
      "goal": "boss:thing_B",
      "raw_skills": "盗墓：当面一击（pick_to_the_face）、影遁；破盾穿刺（pierce）；小丑战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "目标后半获得高防护 → 用穿甲直伤缩短高危段，先确保结晶不会爆，再继续打本体。",
      "risk": "双输出位移可能互相挤位；需按速度安排。若要改 DOT 队必须重算清 DOT 时点；本表未采用需严格控速的慢晕技巧。",
      "gear_advice": "两名脆输出补命中和存活；不把跨回合被清掉的 DOT 算作完整收益。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "grave_robber",
          "skills": [
            "pick_to_the_face",
            "shadow_fade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "source English identifier not present in these original classes: pick_to_the_face"
      ]
    },
    {
      "id": "BP-thing-C-main",
      "family": "thing",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "TH"
      ],
      "scope": "冠军",
      "forms": "thing_C",
      "goal": "boss:thing_C",
      "raw_skills": "盗墓：当面一击（pick_to_the_face）、影遁；破盾穿刺（pierce）；小丑战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "目标后半获得高防护 → 用穿甲直伤缩短高危段，先确保结晶不会爆，再继续打本体。",
      "risk": "双输出位移可能互相挤位；需按速度安排。若要改 DOT 队必须重算清 DOT 时点；本表未采用需严格控速的慢晕技巧。",
      "gear_advice": "两名脆输出补命中和存活；不把跨回合被清掉的 DOT 算作完整收益。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "grave_robber",
          "skills": [
            "pick_to_the_face",
            "shadow_fade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "source English identifier not present in these original classes: pick_to_the_face"
      ]
    },
    {
      "id": "BP-croco-D-main",
      "family": "croco",
      "decision": "条件采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "CR"
      ],
      "scope": "极暗·农庄变体",
      "forms": "com_crocodile_D",
      "goal": "boss:com_crocodile_D",
      "raw_skills": "必带破盾穿刺牵引（puncture）、神秘恶魔牵引/深渊之手；狗守护瘟疫；瘟疫迷乱冲击、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。D 为农庄单独目标；本轮没有这变体的阵容实战，只是外推候选。",
      "rule": "鳄鱼进入可潜水的中间位置 → 优先拉至边位或控制，再输出；防止拉扯使治疗/控制失位。",
      "risk": "移位和控制会失败；破盾前进会改变神秘位置；潜水后高闪避的大招不能当必然可控。",
      "gear_advice": "拉拽者补移位成功，控制者补晕率；输出命中。不能只堆流血忽略当前抗性。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "disorienting_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-croco-A-main",
      "family": "croco",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "CR"
      ],
      "scope": "学徒",
      "forms": "crocodile_A",
      "goal": "boss:crocodile_A",
      "raw_skills": "必带破盾穿刺牵引（puncture）、神秘恶魔牵引/深渊之手；狗守护瘟疫；瘟疫迷乱冲击、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。学徒不等于零级且没技能也能打 Boss。",
      "rule": "鳄鱼进入可潜水的中间位置 → 优先拉至边位或控制，再输出；防止拉扯使治疗/控制失位。",
      "risk": "移位和控制会失败；破盾前进会改变神秘位置；潜水后高闪避的大招不能当必然可控。",
      "gear_advice": "拉拽者补移位成功，控制者补晕率；输出命中。不能只堆流血忽略当前抗性。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "disorienting_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-croco-B-main",
      "family": "croco",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "CR"
      ],
      "scope": "精英",
      "forms": "crocodile_B",
      "goal": "boss:crocodile_B",
      "raw_skills": "必带破盾穿刺牵引（puncture）、神秘恶魔牵引/深渊之手；狗守护瘟疫；瘟疫迷乱冲击、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "鳄鱼进入可潜水的中间位置 → 优先拉至边位或控制，再输出；防止拉扯使治疗/控制失位。",
      "risk": "移位和控制会失败；破盾前进会改变神秘位置；潜水后高闪避的大招不能当必然可控。",
      "gear_advice": "拉拽者补移位成功，控制者补晕率；输出命中。不能只堆流血忽略当前抗性。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "disorienting_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-croco-C-main",
      "family": "croco",
      "decision": "不采纳",
      "user_note": "冠军鳄鱼可以再路线里绕开 不推荐打",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "CR"
      ],
      "scope": "冠军",
      "forms": "crocodile_C",
      "goal": "boss:crocodile_C",
      "raw_skills": "必带破盾穿刺牵引（puncture）、神秘恶魔牵引/深渊之手；狗守护瘟疫；瘟疫迷乱冲击、战地医疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "鳄鱼进入可潜水的中间位置 → 优先拉至边位或控制，再输出；防止拉扯使治疗/控制失位。",
      "risk": "移位和控制会失败；破盾前进会改变神秘位置；潜水后高闪避的大招不能当必然可控。",
      "gear_advice": "拉拽者补移位成功，控制者补晕率；输出命中。不能只堆流血忽略当前抗性。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss",
            "daemons_pull"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "disorienting_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-baron-B-main",
      "family": "baron",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "BA",
        "F10"
      ],
      "scope": "精英",
      "forms": "baron_B",
      "goal": "boss:kill_baron_B",
      "raw_skills": "强盗决斗突刺、手枪；苦修惩罚/苦痛之雨与救赎；小丑战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "虫卵仍在时不能依赖治疗 → 按当前血线开卵并清怪；男爵显形后以流血和反击推进，恢复可用时及时抬血。",
      "risk": "不要把一次全开卵当通用动作；男爵藏在未打开的卵中属于未知内容，AI不能偷读。修女此时可能暂时无事可做，但需保后续恢复。",
      "gear_advice": "输出命中、流血成功；修女治疗与抗晕。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-baron-B-video",
      "family": "baron",
      "decision": "采纳",
      "user_note": "",
      "source": "既有字幕整队记录 · 站位由我安排",
      "basis": "既有字幕整队记录 · 站位由我安排",
      "refs": [
        "F10"
      ],
      "scope": "精英",
      "forms": "baron_B",
      "goal": "boss:kill_baron_B",
      "raw_skills": "男爵前换老兵反击（retribution），保留守护；苦修惩罚、救赎；瘟疫战地医疗/增益；小丑战歌/振奋旋律。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。",
      "rule": "继续使用庭院道中队进男爵 → 临战把老兵盾击换为反击，靠苦修流血与队伍恢复推进；评估虫卵禁疗阶段。",
      "risk": "十七不开自限每职业一人、不开除、零死亡；普通档可换更适配英雄。该视频复盘鳄鱼挂不上流血、进死门靠美德，不证明此队一路安全。",
      "gear_advice": "保证流血成功和生存；不把已发生过的美德补救当作稳定恢复组成。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "retribution"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-viscount-C-main",
      "family": "viscount",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "VI"
      ],
      "scope": "冠军",
      "forms": "viscount_C",
      "goal": "boss:kill_viscount_C",
      "raw_skills": "小丑战歌、振奋旋律；狗猎犬冲锋/守护；苦修惩罚、救赎；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "已能到达 Boss 且恢复余量足 → 用战歌、狗饼干与流血争取缩短战斗；是否先打食物按实际场面决定。",
      "risk": "本轮未确认尸体进食的最佳击杀顺序，不写死；这队也没有保证能硬过沿途每条鳄鱼，路线和阵容需一并审核。",
      "gear_advice": "命中、流血成功与治疗；长道中保留营火和恢复资源。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-countess-C-main",
      "family": "countess",
      "decision": "条件采纳",
      "user_note": "好像还有其他配队",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "CT"
      ],
      "scope": "庭院终局",
      "forms": "countess_D, countess_E, countess_F",
      "goal": "boss:kill_countess_C",
      "raw_skills": "必带老兵战术增益（bolster）、守护、盾击；恶女流血攻击/突破/肾上腺素；苦修惩罚、救赎/放血；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。目标配置后缀 C 不表示沿用普通冠军短本风险。",
      "rule": "稳定守护与恢复具备 → 老兵先防御增益并保护修女，流血输出；有寄生卵者攻击前先核血线和爆卵风险。",
      "risk": "恶女在2号位不能用钢铁天鹅或前排放血，主用流血攻击。老兵盾击会推挤前排，需安排复位；寄生卵可能在死门击杀行动者。",
      "gear_advice": "可用晕率饰品应对刷低晕抗阶段；后排抗晕、治疗；蛇鳞可作为蛋伤害保命储备。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "hellion",
          "skills": [
            "if_it_bleeds",
            "breakthru",
            "adrenaline_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "bolster",
            "rampart"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-guardian-C-main",
      "family": "guardian",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "G"
      ],
      "scope": "冠军",
      "forms": "statue_head_C",
      "goal": "boss:kill_statue_C",
      "raw_skills": "必带破盾穿刺、穿刺牵引；老兵守护/指挥；小丑战歌、短剑突刺；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。",
      "rule": "本体被盾守护 → 破盾穿刺牵引解守护或穿甲打本体，先减少枪矛威胁；保留盾，避免激活本体狂暴路线。",
      "risk": "破盾前进会把老兵挤至2号位，主技能需仍可用。群攻不要误伤并打碎盾；旧站版本差异待核。",
      "gear_advice": "破盾命中/伤害，治疗与守护存活；不把 DOT 作为石像主输出。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "dirk_stab"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-miller-B-main",
      "family": "miller",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "M"
      ],
      "scope": "精英·剧情农庄",
      "forms": "miller_B",
      "goal": "boss:kill_miller",
      "raw_skills": "破盾穿刺/穿刺牵引、蝰蛇之吻；强盗决斗突刺、手枪、零距离；小丑战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。B 为剧情、C 为无尽，不将无尽死亡规则外推剧情。",
      "rule": "冰冻农夫守护本体 → 破盾解守护；连续收割威胁高 → 保血后用穿甲、腐蚀与反击压缩战斗。",
      "risk": "破守护不等于消除收割；清掉随从、占满敌位等局面可能让 Boss 更频繁收割。前置波次和两输出互相移位需验。",
      "gear_advice": "若实际已有米尔德里德项链可优先考虑输出者佩戴；其余补命中、治疗，不假设必刷到。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-miller-B-double_sb",
      "family": "miller",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "FM"
      ],
      "scope": "精英·剧情农庄",
      "forms": "miller_B",
      "goal": "boss:kill_miller",
      "raw_skills": "双破盾：穿刺、穿刺牵引；按顺序配蝰蛇之吻或贯穿；修女双疗；小丑战歌、振奋旋律。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。精英重新检查晕抗、DOT 抗性与实际强化，不从学徒胜利直接外推。B 为剧情、C 为无尽，不将无尽死亡规则外推剧情。",
      "rule": "目标只是推进磨坊主/沉睡者且短期资源足 → 双破盾穿甲与解守护，小丑稳命中压力，修女保血。",
      "risk": "Wiki 明确提醒长线抗乱位、生命和控制薄弱；重复职业违反十七唯一自限，但普通游戏允许。不是千杀通用模板。",
      "gear_advice": "需控速度避免两人抢1号位，补命中和生存；没有指定必须双贯穿。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-miller-C-main",
      "family": "miller",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "M"
      ],
      "scope": "冠军·无尽农庄",
      "forms": "miller_C",
      "goal": "boss:miller_C",
      "raw_skills": "破盾穿刺/穿刺牵引、蝰蛇之吻；强盗决斗突刺、手枪、零距离；小丑战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。B 为剧情、C 为无尽，不将无尽死亡规则外推剧情。",
      "rule": "冰冻农夫守护本体 → 破盾解守护；连续收割威胁高 → 保血后用穿甲、腐蚀与反击压缩战斗。",
      "risk": "破守护不等于消除收割；清掉随从、占满敌位等局面可能让 Boss 更频繁收割。前置波次和两输出互相移位需验。",
      "gear_advice": "若实际已有米尔德里德项链可优先考虑输出者佩戴；其余补命中、治疗，不假设必刷到。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-miller-C-double_sb",
      "family": "miller",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "FM"
      ],
      "scope": "冠军·无尽农庄",
      "forms": "miller_C",
      "goal": "boss:miller_C",
      "raw_skills": "双破盾：穿刺、穿刺牵引；按顺序配蝰蛇之吻或贯穿；修女双疗；小丑战歌、振奋旋律。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。冠军独立复核命中、速度、抗性、首轮和撤退，不能只换高等级英雄。B 为剧情、C 为无尽，不将无尽死亡规则外推剧情。",
      "rule": "目标只是推进磨坊主/沉睡者且短期资源足 → 双破盾穿甲与解守护，小丑稳命中压力，修女保血。",
      "risk": "Wiki 明确提醒长线抗乱位、生命和控制薄弱；重复职业违反十七唯一自限，但普通游戏允许。不是千杀通用模板。",
      "gear_advice": "需控速度避免两人抢1号位，补命中和生存；没有指定必须双贯穿。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-sleeper-D-main",
      "family": "sleeper",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "SL",
        "FR"
      ],
      "scope": "极暗·无尽农庄",
      "forms": "galaxy_D",
      "goal": "boss:galaxy_D",
      "raw_skills": "必带狗目标口哨（target_whistle）、弩手狙击/战地绷带、赏金收集赏金、修女群体治疗；狗在2号位可带闷棍处理道中。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。",
      "rule": "先能通过裂变和磨坊主等前置波次 → 狗减防标记、弩手/赏金爆发，修女用群疗维持多人离开死门。",
      "risk": "标记爆发不能忽略裂变阶段召唤；优先危害大的结晶，升级前削血但杀不掉可能被回满；转阶段无休整。 狗在2号位不能减压，恢复窗口先调位；不要在战斗中挤掉弩手的狙击位置。",
      "gear_advice": "治疗、命中与伤害；长线恢复依靠狗减压与波次安全窗口；不把纯闪避当足够防线。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "whistle",
            "blackjack"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "source English identifier not present in these original classes: target_whistle"
      ]
    },
    {
      "id": "BP-sleeper-D-double_sb",
      "family": "sleeper",
      "decision": "条件采纳",
      "user_note": "",
      "source": "社区明确整队 · 站位由我安排",
      "basis": "社区明确整队 · 站位由我安排",
      "refs": [
        "FM"
      ],
      "scope": "极暗·无尽农庄",
      "forms": "galaxy_D",
      "goal": "boss:galaxy_D",
      "raw_skills": "双破盾：穿刺、穿刺牵引；按顺序配蝰蛇之吻或贯穿；修女双疗；小丑战歌、振奋旋律。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。",
      "rule": "目标只是推进磨坊主/沉睡者且短期资源足 → 双破盾穿甲与解守护，小丑稳命中压力，修女保血。",
      "risk": "Wiki 明确提醒长线抗乱位、生命和控制薄弱；重复职业违反十七唯一自限，但普通游戏允许。不是千杀通用模板。",
      "gear_advice": "需控速度避免两人抢1号位，补命中和生存；没有指定必须双贯穿。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-dd1-D-main",
      "family": "dd1",
      "decision": "采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "D1"
      ],
      "scope": "极暗",
      "forms": "shuffler_D",
      "goal": "boss:kill_shuffler_D",
      "raw_skills": "神秘治疗/标记；十字军圣矛、战斗治疗/振奋呐喊；强盗决斗突刺、手枪；苦修惩罚、救赎，必要时移动到前排。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。",
      "rule": "被乱位后仍要能出招 → 机动输出与全站位治疗维持推进；祭司可控时先控，别无脑反复杀召唤。",
      "risk": "反击可能误杀祭司并触发新召唤；瘟疫低级控制队不能直接搬来。苦修救赎受次数与血线约束。",
      "gear_advice": "命中、DOT/抗性与保命；严格无光时振奋呐喊等升光技能禁用，换允许技能。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "holy_lance",
            "battle_heal",
            "inspiring_cry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-templar-近战D-main",
      "family": "templar",
      "decision": "条件采纳",
      "user_note": "这个可以再考虑考虑其他攻略阵容",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "D2"
      ],
      "scope": "极暗",
      "forms": "templar_melee_D",
      "goal": "boss:templar_melee_D",
      "raw_skills": "必带老兵守护；狗守护作应急、猎犬冲锋；恶女钢铁天鹅/流血攻击；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。近战/远程是两个目标，同队不是两次成功样本。",
      "rule": "三人装备火焰护符、第四人有持续守护 → 维护守护优先于多打一刀；双圣殿先集中清掉一个。",
      "risk": "守护没挂上或被晕断层时第四人暴露；不能只靠护甲抵抗启示。三处遭遇与道中都要算，不是只算双Boss。",
      "gear_advice": "火焰护符是方案必要条件：给老兵、狗、恶女；修女不戴时由戴护符老兵持续守护。余位优先抗晕、治疗、命中。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-templar-远程D-main",
      "family": "templar",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "D2"
      ],
      "scope": "极暗",
      "forms": "templar_ranged_D",
      "goal": "boss:templar_ranged_D",
      "raw_skills": "必带老兵守护；狗守护作应急、猎犬冲锋；恶女钢铁天鹅/流血攻击；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。近战/远程是两个目标，同队不是两次成功样本。",
      "rule": "三人装备火焰护符、第四人有持续守护 → 维护守护优先于多打一刀；双圣殿先集中清掉一个。",
      "risk": "守护没挂上或被晕断层时第四人暴露；不能只靠护甲抵抗启示。三处遭遇与道中都要算，不是只算双Boss。",
      "gear_advice": "火焰护符是方案必要条件：给老兵、狗、恶女；修女不戴时由戴护符老兵持续守护。余位优先抗晕、治疗、命中。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-cell-D-main",
      "family": "cell",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "D3"
      ],
      "scope": "极暗",
      "forms": "cell_white_D",
      "goal": "boss:cell_white_D",
      "raw_skills": "必带恶女钢铁天鹅、狗猎犬冲锋、瘟疫手雷/致盲毒气/战地医疗；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。",
      "rule": "白细胞柄有传送威胁 → 后排双晕/直伤先压制它，再持续处理大体型本体；保留抗 DOT 恢复。",
      "risk": "全队必须保后排覆盖；不要把固定地图未侦察内容直接授予 AI。巨型胞囊未单列管家Boss目标，但此行同时说明其遭遇。",
      "gear_advice": "后排处理者补命中与速度、瘟疫补晕率；备解毒剂，营火用于长途恢复。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine",
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-heart-D-main",
      "family": "heart",
      "decision": "待观察",
      "user_note": "",
      "source": "我的配队推断",
      "basis": "我的配队推断",
      "refs": [
        "D4"
      ],
      "scope": "极暗",
      "forms": "ancestor_heart_D",
      "goal": "boss:kill_ancestor_heart_D",
      "raw_skills": "强盗决斗突刺/手枪/零距离；十字军惩击、圣矛、战斗治疗；小丑战歌/终曲；修女双疗。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。牺牲与职业培养排期、血月死亡预算一起评估。",
      "rule": "选择常规四人终局队 → 先祖阶段保恢复，进入心脏阶段按剩余队友安排输出与牺牲次序，确保最后两人仍能攻击和恢复。",
      "risk": "常规四人会面对剧情强制牺牲，计入血月死亡预算；具体保留谁须按当时状态，不能固定牺牲治疗者。零死亡自限不默认适用。",
      "gear_advice": "命中、伤害与治疗；不把极端一刀流所需顶配和暴击当普通前提。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "heroic_end"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BP-heart-D-duo",
      "family": "heart",
      "decision": "不采纳",
      "user_note": "不做成就 用正常4人队就可以",
      "source": "社区打法存在性 · 站位由我安排",
      "basis": "社区打法存在性 · 站位由我安排",
      "refs": [
        "D4"
      ],
      "scope": "极暗",
      "forms": "ancestor_heart_D",
      "goal": "boss:kill_ancestor_heart_D",
      "raw_skills": "十字军必带战斗治疗，减压技能按当前光照规则允许性选择；强盗决斗突刺、零距离、手枪。",
      "raw_loadout": "",
      "conditions": "默认普通游戏候选；难度模式、光照、实际刀甲技能、饰品和完整道中都须满足。牺牲与职业培养排期、血月死亡预算一起评估。",
      "rule": "仅在明确选择少人规避剧情牺牲且已能打穿先祖时 → 十字军治疗减压，强盗反击输出，按技能移动。",
      "risk": "Wiki 提到此双人组合能完成，但不等于这次名册能打；前阶段行动数更少，难度高。严格无光不准升光时十字军减压受限。",
      "gear_advice": "足够命中、生存和恢复；不默认拥有高美德或无限叠增益。",
      "slots": [
        {
          "rank": 1,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "crusader",
          "skills": [
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "—",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "—",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-necro-dance",
      "family": "necro",
      "decision": "条件采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 技能组部分推断",
      "basis": "作者明确整队与站位 · 技能组部分推断",
      "refs": [
        "XN"
      ],
      "scope": "难度需分别核对",
      "forms": "necromancer_A",
      "goal": "encounter:necro",
      "raw_skills": "这套的关键是破盾穿刺（pierce）和十字军圣矛（holy_lance），并配出手先后；恶女钢铁天鹅，修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "破盾先行动并前进 → 十字军被推到3号位后用圣矛；恶女与破盾保持后排覆盖，不用流血做道中主轴。",
      "risk": "破盾被控或慢于十字军时圣矛循环中断；恶女不在1号位不能钢铁天鹅。不能把预期出手序当真实队列。",
      "gear_advice": "整理者建议：破盾速度高于十字军但不能保证绝对先手；输出命中，修女治疗。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "crusader",
          "skills": [
            "holy_lance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-prophet-single_debuff",
      "family": "prophet",
      "decision": "条件采纳",
      "user_note": "",
      "source": "来源明确整队 · 站位由我安排",
      "basis": "来源明确整队 · 站位由我安排",
      "refs": [
        "XP",
        "P"
      ],
      "scope": "学徒/精英优先候选；冠军另核",
      "forms": "prophet_A",
      "goal": "encounter:prophet",
      "raw_skills": "必带威慑；狗猎犬冲锋/守护；弩手狙击；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。学徒/精英优先候选；冠军另核。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "希望有稳定治疗且接受单降攻手 → 麻风维持威慑，弩手和狗打后排，狗在危险时守护；不足就改双降攻。",
      "risk": "缺神秘学者后叠降攻更慢且没有第二降攻手；按用户批注，冠军仍优先原双降攻队。本行主要是名册不齐时的对照。",
      "gear_advice": "建议麻风抗晕/减益成功，修女治疗；不把单麻风当无需装备支持的减伤保险。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-swine-flag_core",
      "family": "swine",
      "decision": "未标定",
      "user_note": "",
      "source": "作者明确整队与站位 · 动作由我整理",
      "basis": "作者明确整队与站位 · 动作由我整理",
      "refs": [
        "XGD",
        "S"
      ],
      "scope": "难度需分别核对",
      "forms": "swine_prince_A",
      "goal": "encounter:swine",
      "raw_skills": "必带弩手照明弹（flare）、狗守护、麻风威慑；苦修惩罚与救赎/重生，不能把低血技能当随时可用。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "清标记和前排自立可用 → 弩手优先消标记，狗护弩手/输出，苦修补治疗与流血，麻风降攻或直伤。",
      "risk": "严格无光禁升光技能时不能用照明弹路线；没有神秘学者后降攻堆叠较弱。禁止群攻/反击误伤威尔伯。",
      "gear_advice": "建议弩手抗晕、苦修流血与存活；麻风命中/减益按本场职责选择。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "intimidate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "flagellant",
          "skills": [
            "punish",
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-flesh-double_hound",
      "family": "flesh",
      "decision": "未标定",
      "user_note": "",
      "source": "玩家提案整队 · 无通关样本",
      "basis": "玩家提案整队 · 无通关样本",
      "refs": [
        "XF",
        "F"
      ],
      "scope": "难度需分别核对",
      "forms": "formless_guard_A, formless_melee_A, formless_ranged_A, formless_weak_A",
      "goal": "encounter:flesh",
      "raw_skills": "两名狗均必须带猎犬骚扰（hounds_harry），道中猎犬冲锋；老兵守护/指挥；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "多部位共享血条 → 双狗猎犬骚扰叠群体流血，老兵保护脆位，修女抬血；直伤转打当前软部位。",
      "risk": "一个狗在2号位不能减压，恢复时调位；三人偏辅助使速度依赖 DOT；普通档允许双狗，十七唯一档不允许。",
      "gear_advice": "建议流血成功、命中和生存；狗饼干不能当成提高流血强度/成功的装备，收益另算。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "hounds_harry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "hounds_harry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-flesh-occ_jester",
      "family": "flesh",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家明确整队与方向 · 四技能不完整",
      "basis": "玩家明确整队与方向 · 四技能不完整",
      "refs": [
        "XF"
      ],
      "scope": "难度需分别核对",
      "forms": "formless_guard_A, formless_melee_A, formless_ranged_A, formless_weak_A",
      "goal": "encounter:flesh",
      "raw_skills": "必带猎犬骚扰；小丑战歌/振奋旋律；神秘治疗；苦修惩罚、苦痛之雨和至少一种恢复。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "接受少一名修女且追求更快 DOT → 狗群体流血，苦修打软部位，小丑给命中速度，神秘按血线治疗/减益。",
      "risk": "神秘在4号位不能用深渊之手；没有修女群疗，零奶、流血或两人死门时更危险。不是上一套的同等容错替换。",
      "gear_advice": "建议狗/苦修流血成功，神秘治疗；解毒/绷带分担恢复。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_harry"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-hag-mobile",
      "family": "hag",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 技能/动作纠偏",
      "basis": "作者明确整队与站位 · 技能/动作纠偏",
      "refs": [
        "XGD",
        "H"
      ],
      "scope": "难度需分别核对",
      "forms": "hag_A",
      "goal": "encounter:hag",
      "raw_skills": "破盾穿刺；盗墓投掷匕首/突刺、影遁用于复位；恶女钢铁天鹅/流血攻击；修女群疗（前移到2仍可用）。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "有人被抓后仍有后排打击 → 破盾/盗墓灵活输出，恶女改用当前可施放的后排技能，修女接出锅者。",
      "risk": "前进技能会让恶女失去1号位；修女落1号位无法治疗。旧文主张立即打锅不能无条件照搬；要按被抓角色与输出重新判断。",
      "gear_advice": "建议命中与治疗速度；蛇鳞仅在实际有库存时考虑。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "grave_robber",
          "skills": [
            "thrown_dagger",
            "shadow_fade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-cannon-pierce",
      "family": "cannon",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 技能/动作纠偏",
      "basis": "作者明确整队与站位 · 技能/动作纠偏",
      "refs": [
        "XGD",
        "B"
      ],
      "scope": "难度需分别核对",
      "forms": "brigand_cannon_A",
      "goal": "encounter:cannon",
      "raw_skills": "盗墓投掷匕首必带；破盾穿刺；恶女钢铁天鹅/流血攻击；修女审判作为补刀而非必控4号位。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "每轮能稳定处理火柴手 → 盗墓/破盾/恶女分担后排击杀，再利用穿甲打炮身。",
      "risk": "没有瘟疫双晕兜底；修女眩目圣光够不到敌4号位。破盾前进挤掉恶女1号位时需保留盗墓/破盾后排处理动作。",
      "gear_advice": "输出命中优先，预留第二个能打到火柴手的人；不能依赖暴击才够杀。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "grave_robber",
          "skills": [
            "thrown_dagger"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "judgement"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-siren-pd_anti",
      "family": "siren",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位与技能推断",
      "basis": "玩家自述整队 · 站位与技能推断",
      "refs": [
        "R",
        "SI"
      ],
      "scope": "难度需分别核对",
      "forms": "siren_A",
      "goal": "encounter:siren",
      "raw_skills": "瘟疫战地医疗/后排控制；神秘治疗；强盗手枪和决斗突刺。临战审查已装备的攻击/治疗，防止魅惑后威胁队友或奶 Boss。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "诱导古商只是概率帮助 → 用瘟疫控随从与解 DOT，神秘治血，强盗承担输出；主力被抓也要能续局。",
      "risk": "没有守护者；强盗魅惑后的反击/直伤危险，神秘可能奶 Boss；瘟疫被挤前排会失去医疗。诱导失败时必须有退出/控场判断。",
      "gear_advice": "建议控制成功与命中；圣水调整抗性不等于保证抓古商。",
      "slots": [
        {
          "rank": 1,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "antiquarian",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-crew-blight_mobile",
      "family": "crew",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 动作由我整理",
      "basis": "作者明确整队与站位 · 动作由我整理",
      "refs": [
        "XGD",
        "C"
      ],
      "scope": "难度需分别核对",
      "forms": "drowned_captain_A",
      "goal": "encounter:crew",
      "raw_skills": "瘟疫手雷/瘟毒炸裂/战地医疗；破盾穿刺；十字军惩击/圣矛/战斗治疗；神秘治疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。难度需分别核对。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "需要腐蚀与抗拉扯 → 瘟疫处理本体 DOT，前排直伤优先解锚，神秘与十字军提供恢复。",
      "risk": "神秘4号位无前排晕；不要在锚手未抛锚时浪费可被回满的伤害。有人仍被绑时撤退会留下他；破盾移位需保瘟疫后排。",
      "gear_advice": "建议前排命中、瘟疫腐蚀成功；严格无光时不把十字军升光减压算进去。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine",
            "noxious_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-collector-mark_reach",
      "family": "collector",
      "decision": "待观察",
      "user_note": "",
      "source": "遭遇战示例 · 来源明确整队",
      "basis": "遭遇战示例 · 来源明确整队",
      "refs": [
        "XCO",
        "CO"
      ],
      "scope": "遭遇后按当前难度与状态判断",
      "forms": "collector_A",
      "goal": "encounter:collector",
      "raw_skills": "狗2号位可闷棍；赏金靠近点/终结者；神秘恶魔牵引、治疗/标记；弩手狙击/战地绷带。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。遭遇后按当前难度与状态判断。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "当前队已经具备远程、拉前与恢复 → 比较首轮爆发/控制和接下来承伤；打不过或高减员风险则撤，不专为等收藏家预留这队。",
      "risk": "神秘3号位不能深渊之手；狗2号位不能减压。原旧帖预排真实出手序与两轮杀只是理想案例，不能用于 AI 偷看队列。",
      "gear_advice": "建议命中、拉拽成功与有效晕率；弩手作为零奶时的恢复备援。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "finish_him"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "blackjack"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "occultist",
          "skills": [
            "daemons_pull"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-shambler-abom_front",
      "family": "shambler",
      "decision": "待观察",
      "user_note": "麻风随机遭遇爬会很难受 乱位 祭坛爬爬可以打",
      "source": "玩家自述整队 · 站位推断",
      "basis": "玩家自述整队 · 站位推断",
      "refs": [
        "XSH",
        "SH"
      ],
      "scope": "冠军样本；学徒/精英另核，E未验证",
      "forms": "shambler_A",
      "goal": "encounter:shambler",
      "raw_skills": "咒缚者变身后耙击/猛撞复位（该职业技能不按普通四技能挑选）；麻风劈斩、庄严；强盗决斗突刺/零距离；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。冠军样本；学徒/精英另核，E未验证。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "现队乱位后仍能恢复，且前排能快速压住触手 → 咒缚耙击、麻风群劈分担触手，强盗机动反击；本体死后仍需清触手。",
      "risk": "这是冠军玩家自述样本，不是通用安全队。麻风后排与修女前排会让能力断档；不接受“反向开局必安全”或“总有人能打”的保证。",
      "gear_advice": "来源提营火准备；整理者建议补命中、咒缚输出及修女存活。黑暗中不算高光加成。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "solemnity"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "abomination",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "highwayman",
          "skills": [
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-shrieker-arbalest_mark",
      "family": "shrieker",
      "decision": "条件采纳",
      "user_note": "要击杀才带",
      "source": "作者明确整队与站位 · 目标分层",
      "basis": "作者明确整队与站位 · 目标分层",
      "refs": [
        "XGD",
        "SR"
      ],
      "scope": "事件A/B/C分别核对；D未验证",
      "forms": "crow_D",
      "goal": "encounter:shrieker",
      "raw_skills": "弩手狙击/标记、战地绷带；双狗猎犬冲锋；老兵守护/指挥，禁止把两目标都想要当默认。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。事件A/B/C分别核对；D未验证。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "先确定任务意图 → 回收可走生存，打巢求钱先算窗口；确要杀鸟才以命中、标记和双狗集中输出。",
      "risk": "没有修女、没有小丑；拖到逃走任务可能完成但不等于击杀。按 W-106 它不是必杀管家项，D 农庄样本仍缺。",
      "gear_advice": "建议命中、狗输出和防猝死；不要把窝低闪避当鸟也容易命中。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-vvulf-burst",
      "family": "vvulf",
      "decision": "待观察",
      "user_note": "不太稳 看情况作为备选 没有足够减压 我推荐还是修丑兵体系",
      "source": "作者明确整队与站位 · 条件待确认",
      "basis": "作者明确整队与站位 · 条件待确认",
      "refs": [
        "XGD",
        "V"
      ],
      "scope": "狼袭任务",
      "forms": "brigand_sapper_D",
      "goal": "encounter:vvulf",
      "raw_skills": "老兵守护必带；赏金收集赏金/终结者；盗墓投掷匕首、当面一击/复位；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。狼袭任务。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "守护装备和先手都准备好 → 老兵接炸弹，赏金与盗墓集中单体，修女抬守护者；道中也需可控。",
      "risk": "单守护被控或来不及挂会露出脆位；短本没有营地补救。证明有其他作者配队，不证明不需要强化就能打。",
      "gear_advice": "建议老兵防护/血量/抗晕和足够先手，输出命中。用户说“神装”的具体清单仍待定义，不能凭队名就满足。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty",
            "finish_him"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "grave_robber",
          "skills": [
            "pick_to_the_face",
            "thrown_dagger"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-fanatic-dodge_counter",
      "family": "fanatic",
      "decision": "条件采纳",
      "user_note": "",
      "source": "我的遭遇战示例 · 来源局部支持",
      "basis": "我的遭遇战示例 · 来源局部支持",
      "refs": [
        "FA"
      ],
      "scope": "优先避战；冠军尤其严格",
      "forms": "fanatic_A",
      "goal": "encounter:fanatic",
      "raw_skills": "古商振奋蒸汽；强盗决斗突刺/手枪；老兵反击/守护/战术增益；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。优先避战；冠军尤其严格。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "避战仍优先；若当前队已有高闪避支持且决定接战 → 古商维持闪避，老兵/强盗反击并恢复站位，修女保血。",
      "risk": "没有流血主C，战斗可能长；火刑架/乱位可让奶断档，拆架改变 Boss 行为。架上有人时撤退会丢下他。",
      "gear_advice": "需要实际闪避装备/增益和恢复余量；仅多一个古商不等于闪避体系成型。不默认带碎片粉尘。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "bolster"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "antiquarian",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-thing-dot_caltrops",
      "family": "thing",
      "decision": "待观察",
      "user_note": "",
      "source": "我的遭遇战示例 · 来源局部支持",
      "basis": "我的遭遇战示例 · 来源局部支持",
      "refs": [
        "TH"
      ],
      "scope": "遭遇后按当前状态判断",
      "forms": "thing_A",
      "goal": "encounter:thing",
      "raw_skills": "瘟疫手雷/医疗；苦修前后排流血和恢复；狗目标口哨；赏金铁蒺藜（本机ID hook_and_slice：英雄2/3/4施放，只打敌3/4）/直伤。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。遭遇后按当前状态判断。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "当前队具备清结晶、持续伤害和恢复 → 瘟疫/苦修争取在 Boss 动作前挂 DOT，狗减防，赏金铁蒺藜协助速度/伤害；先保人再贪完整 DOT。",
      "risk": "无修女稳定群疗；回合清 DOT 会吃掉预期伤害。已有治疗次数、DOT与血线不够时，这队名不构成接战依据。 铁蒺藜必须有敌3/4目标；Boss不在此打击面时不能计入减速/增伤收益。",
      "gear_advice": "建议 DOT 成功、命中与可实现的速度；不假设能固定先于 Boss。",
      "slots": [
        {
          "rank": 1,
          "class": "flagellant",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "bounty_hunter",
          "skills": [
            "hook_and_slice"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "whistle"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-croco-bleed_backreach",
      "family": "croco",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · A/B候选",
      "basis": "作者明确整队与站位 · A/B候选",
      "refs": [
        "XGD",
        "CR"
      ],
      "scope": "仅学徒/精英候选；冠军绕开",
      "forms": "com_crocodile_D",
      "goal": "encounter:croco",
      "raw_skills": "恶女钢铁天鹅/流血攻击；苦修惩罚/苦痛之雨及恢复；小丑战歌/振奋旋律；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。仅学徒/精英候选；冠军绕开。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "无法可靠拉拽却有足够流血/续航 → 保前后排覆盖和压力恢复，按鳄鱼位置选直伤/流血；大招前检查所有人血线。",
      "risk": "没有拉出中间位的关键能力，不能当潜水封锁队；高流血抗性、恶女被拉离1号位或奶被控会失效。冠军继续按你的意见绕开，不新增采纳C/D。",
      "gear_advice": "建议命中、流血成功与治疗；圣水/绷带按当前配置准备。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-baron-aggressive_bleed",
      "family": "baron",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 技能组推断",
      "basis": "作者明确整队与站位 · 技能组推断",
      "refs": [
        "XGD",
        "BA"
      ],
      "scope": "庭院男爵",
      "forms": "baron_B",
      "goal": "encounter:baron",
      "raw_skills": "恶女钢铁天鹅/流血攻击；苦修惩罚/苦痛之雨/救赎；强盗决斗突刺/手枪；神秘治疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院男爵。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "恢复和道中减压预算允许 → 用前后排流血与强盗伤害压缩禁疗阶段，神秘/苦修承担能治疗时的恢复。",
      "risk": "无稳定战中群体减压，神秘4号位不能晕。未开卵内容不可偷看；禁疗和恢复次数/血线限制要一起算。",
      "gear_advice": "建议流血成功与命中；前置道中用营火和补给补压力，不凭高输出省掉恢复。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "redeem"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "occultist",
          "skills": [
            "wyrd_reconstruction"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-viscount-pd_buff",
      "family": "viscount",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 原版打击面核对",
      "basis": "作者明确整队与站位 · 原版打击面核对",
      "refs": [
        "XGD",
        "VI"
      ],
      "scope": "庭院子爵",
      "forms": "viscount_C",
      "goal": "encounter:viscount",
      "raw_skills": "瘟疫壮胆蒸汽（emboldening_vapours）、战地医疗；切割（incision）可在1/2/3号位用，但仅触及敌1/2；恶女前排放血/流血攻击；强盗切开伤口/手枪；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院子爵。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "愿意用增益换更快斩杀 → 瘟疫壮胆与医疗，恶女/强盗集中输出；按食物与反击的实际可见状态决定是否转火。",
      "risk": "瘟疫切割在3号位合法，但只打敌1/2；子爵不在打击面时改增益/医疗。壮胆有使用次数限制。强盗突刺会挤走恶女1号位，依赖前排放血时须先处理此冲突。",
      "gear_advice": "建议命中、流血成功、输出增益和治疗；有精确营地安排再算增益覆盖。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "if_it_bleeds",
            "bleed_out"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "plague_doctor",
          "skills": [
            "battlefield_medicine",
            "emboldening_vapours"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-countess-guard_hellion",
      "family": "countess",
      "decision": "条件采纳",
      "user_note": "这里标记为条件采纳的就是我认为可行的备选配队",
      "source": "玩家明确整队 · 完整四技能由我建议",
      "basis": "玩家明确整队 · 完整四技能由我建议",
      "refs": [
        "XC1",
        "XFAIL"
      ],
      "scope": "庭院伯爵夫人",
      "forms": "countess_D, countess_E, countess_F",
      "goal": "encounter:countess",
      "raw_skills": "必带老兵守护/指挥/战术增益；恶女流血输出；小丑战歌、振奋旋律；修女双疗。",
      "raw_loadout": "整理者建议四技能（不是原作者完整实装）：\n4修女：单体治疗、群体治疗、审判、眩目圣光；有卵时以恢复为主。\n3小丑：战歌、振奋旋律、短剑突刺、谢幕；谢幕前先算蛋与复位。\n2老兵：守护、指挥、战术增益、盾击；盾击会前进，只在站位允许时用。\n1恶女：前排放血、流血攻击、突破、肾上腺素；被挤离1后改合法技能。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院伯爵夫人。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "开局能及时挂守护且续护不断 → 老兵保护恶女，其他人以治疗/战歌为主，降低攻击时爆卵的机会。",
      "risk": "守护前已中卵、守护断档、恶女乱到后排都会破坏单核节奏；已有卵不会被守护清除。不能承诺“从不被挂卵”；未覆盖的群攻照样危险。",
      "gear_advice": "来源偏好闪避与老兵抗晕、营地先上增益；碎片粉尘只是额外资源分支，不默认拥有或使用。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "if_it_bleeds",
            "bleed_out",
            "breakthru",
            "adrenaline_rush"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command",
            "bolster",
            "rampart"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune",
            "dirk_stab",
            "heroic_end"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort",
            "judgement",
            "dazzling_light"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-countess-guard_sb",
      "family": "countess",
      "decision": "条件采纳",
      "user_note": "",
      "source": "玩家明确整队 · 技能组/站位由我建议",
      "basis": "玩家明确整队 · 技能组/站位由我建议",
      "refs": [
        "XC2",
        "CT"
      ],
      "scope": "庭院伯爵夫人",
      "forms": "countess_D, countess_E, countess_F",
      "goal": "encounter:countess",
      "raw_skills": "破盾穿刺（pierce）是主轴，蛇舞保命；老兵守护；小丑战歌/减压；修女双疗。",
      "raw_loadout": "整理者建议四技能：4修女双疗、审判、眩目圣光；3小丑战歌、振奋旋律、短剑突刺、谢幕；2老兵守护、指挥、战术增益、盾击；1破盾穿刺、蛇舞、穿刺牵引、蝰蛇之吻。后两项为道中/复位备用，Boss主攻不依赖腐蚀。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院伯爵夫人。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "希望用机动与穿甲应对高防护 → 破盾穿刺为主；老兵视卵/血线保护输出者，其余恢复和战歌。",
      "risk": "破盾依旧脆，守护迟到/乱位可能带来蛋；别把伯爵夫人的全部攻击算成可反击。穿甲不等于能防蛋伤。",
      "gear_advice": "建议破盾命中与生存、老兵抗晕、修女治疗；蛇鳞按实际库存，不把腐蚀成功当必然。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "serpents_sway",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command",
            "bolster",
            "rampart"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune",
            "dirk_stab",
            "heroic_end"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort",
            "judgement",
            "dazzling_light"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": [
        "ambiguous corrosion description; no invented plague skill"
      ]
    },
    {
      "id": "BX-countess-leper_anchor",
      "family": "countess",
      "decision": "条件采纳",
      "user_note": "",
      "source": "来源明确站位/技能/饰品 · 自述样本",
      "basis": "来源明确站位/技能/饰品 · 自述样本",
      "refs": [
        "XC3"
      ],
      "scope": "庭院伯爵夫人 · 装备要求高",
      "forms": "countess_D, countess_E, countess_F",
      "goal": "encounter:countess",
      "raw_skills": "此队依赖特定全套技能：后排老兵只用守护/指挥/战术增益/咆哮；麻风劈砍、复仇、抵抗、庄严；苦修重生提供恢复。",
      "raw_loadout": "来源明确四技能：\n4老兵：守护、指挥、战术增益、咆哮。\n3修女：单疗、群疗、眩目圣光、审判。\n2苦修：惩罚、苦痛之雨、放血（exsanguinate）、重生（reclaim）。\n1麻风：劈砍、复仇（revenge）、抵抗（withstand）、庄严（solemnity）。\n苦修低血技能有使用条件与次数；重生自伤流血也要计算。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院伯爵夫人 · 装备要求高。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "已用抗移位和守护维护前排 → 老兵保护麻风并指挥，麻风用直伤与自恢复，修女/苦修维持血线。",
      "risk": "没有小丑，压力是明显短板；老兵4号位不能反击/盾击，麻风后排会丢输出。配置没齐不能只抄四人；不能赌满血就不会被强制乱位。",
      "gear_advice": "来源列老兵守护者之盾+重靴，修女治疗/晕饰品+重靴，苦修流血/生存+重靴；具体译名效果需按当前原版再核，不保证绝对免疫移动。",
      "slots": [
        {
          "rank": 1,
          "class": "leper",
          "skills": [
            "chop",
            "revenge",
            "withstand",
            "solemnity"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "reclaim",
            "exsanguinate"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort",
            "judgement",
            "dazzling_light"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command",
            "bolster",
            "bellow"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-countess-mobile_crusader",
      "family": "countess",
      "decision": "条件采纳",
      "user_note": "",
      "source": "玩家明确整队与方向 · 技能争议保留",
      "basis": "玩家明确整队与方向 · 技能争议保留",
      "refs": [
        "XC3"
      ],
      "scope": "庭院伯爵夫人",
      "forms": "countess_D, countess_E, countess_F",
      "goal": "encounter:countess",
      "raw_skills": "十字军圣矛与战斗治疗；破盾穿刺；狗守护/猎犬冲锋；修女双疗。",
      "raw_loadout": "整理者建议：4修女双疗、审判、眩目圣光；3狗猎犬冲锋、守护、舔舐伤口、群体减压；2破盾穿刺、蛇舞、穿刺牵引、蝰蛇之吻（仅备用）；1十字军惩击、圣矛、战斗治疗、眩晕。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院伯爵夫人。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "更看重乱位后仍有行动 → 破盾/十字军边攻击边恢复阵型，狗承担守护/减压，修女恢复。",
      "risk": "来源先建议蝰蛇之吻腐蚀，后被指出抗性高并改推荐穿刺；本行主攻穿甲。修女被挤1号位仍不能奶；狗守护比老兵短，维护成本要算。",
      "gear_advice": "来源给过职业饰品，但不是本轮必需清单；建议命中、治疗和狗的生存。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "holy_lance",
            "battle_heal",
            "stunning_blow"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard",
            "serpents_sway",
            "adders_kiss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "lick_wounds",
            "guard_dog",
            "howl"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort",
            "judgement",
            "dazzling_light"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-countess-flag_sb",
      "family": "countess",
      "decision": "条件采纳",
      "user_note": "",
      "source": "玩家明确整队与站位 · 条件待核",
      "basis": "玩家明确整队与站位 · 条件待核",
      "refs": [
        "XC4"
      ],
      "scope": "庭院伯爵夫人 / 长道中",
      "forms": "countess_D, countess_E, countess_F",
      "goal": "encounter:countess",
      "raw_skills": "苦修惩罚/苦痛之雨、救赎/重生；破盾穿刺/蛇舞；修女双疗；小丑战歌/振奋旋律。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。庭院伯爵夫人 / 长道中。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "追求整趟庭院的恢复和前后排覆盖 → 苦修流血/恢复、破盾穿甲，小丑稳压力；危险卵出现时改辅助动作或保命。",
      "risk": "无老兵持续守护，不是守护单核的同等容错替代。两名前排都可能有卵；苦修有限恢复不可当无限药。",
      "gear_advice": "建议流血成功、破盾命中、修女治疗、小丑生存；不是自动拥有全套神装。",
      "slots": [
        {
          "rank": 1,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "serpents_sway"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "flagellant",
          "skills": [
            "punish",
            "rain_of_sorrows",
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-guardian-double_pierce",
      "family": "guardian",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 原版打击面另核",
      "basis": "作者明确整队与站位 · 原版打击面另核",
      "refs": [
        "XGD",
        "G"
      ],
      "scope": "可选石像守卫任务",
      "forms": "statue_head_C",
      "goal": "encounter:guardian",
      "raw_skills": "破盾穿刺/穿刺牵引；盗墓当面一击（pick）、投掷匕首；修女双疗；十字军惩击/恢复。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。可选石像守卫任务。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "石像高防护且盾在守护 → 破盾解守护/穿甲，盗墓当面一击打能触及的部位，先降低枪矛威胁。",
      "risk": "盗墓3号位可当面一击但只能触及前两敌位；不要让群攻误毁盾。它不是管家必做项，去刷饰品才评估收益。",
      "gear_advice": "建议穿甲者命中/伤害和修女治疗；恢复压力靠营火/十字军允许的技能。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "grave_robber",
          "skills": [
            "pick_to_the_face",
            "thrown_dagger"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-miller-arb_jester",
      "family": "miller",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 技能/站位核对",
      "basis": "社区明确整队 · 技能/站位核对",
      "refs": [
        "FM",
        "M"
      ],
      "scope": "剧情/无尽分别核；不保证无限长线",
      "forms": "miller_B",
      "goal": "encounter:miller",
      "raw_skills": "神秘深渊之手/治疗；狗闷棍/猎犬冲锋/口哨；弩手狙击/战地绷带；小丑战歌/振奋旋律。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。剧情/无尽分别核；不保证无限长线。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "需要整段波次而不是只赢末尾一场 → 用前排神秘控制、狗点杀/减防、弩手远程与备援治疗，小丑维持命中压力。",
      "risk": "没有修女群疗，多人同时死门时恢复更吃顺序；神秘0奶要有弩手兜底。磨坊主守护/连续收割、沉睡者高压仍需单独算。",
      "gear_advice": "建议神秘生存和晕率、弩手命中/治疗支援；狗2号位不计入减压，减压由小丑承担。",
      "slots": [
        {
          "rank": 1,
          "class": "occultist",
          "skills": [
            "hands_from_abyss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "whistle",
            "blackjack"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-sleeper-arb_jester",
      "family": "sleeper",
      "decision": "采纳",
      "user_note": "",
      "source": "社区明确整队 · 技能/站位核对",
      "basis": "社区明确整队 · 技能/站位核对",
      "refs": [
        "FM",
        "SL"
      ],
      "scope": "剧情/无尽分别核；不保证无限长线",
      "forms": "galaxy_D",
      "goal": "encounter:sleeper",
      "raw_skills": "神秘深渊之手/治疗；狗闷棍/猎犬冲锋/口哨；弩手狙击/战地绷带；小丑战歌/振奋旋律。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。剧情/无尽分别核；不保证无限长线。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "需要整段波次而不是只赢末尾一场 → 用前排神秘控制、狗点杀/减防、弩手远程与备援治疗，小丑维持命中压力。",
      "risk": "没有修女群疗，多人同时死门时恢复更吃顺序；神秘0奶要有弩手兜底。磨坊主守护/连续收割、沉睡者高压仍需单独算。",
      "gear_advice": "建议神秘生存和晕率、弩手命中/治疗支援；狗2号位不计入减压，减压由小丑承担。",
      "slots": [
        {
          "rank": 1,
          "class": "occultist",
          "skills": [
            "hands_from_abyss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "whistle",
            "blackjack"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-miller-pd_bh",
      "family": "miller",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 技能/动作纠偏",
      "basis": "作者明确整队与站位 · 技能/动作纠偏",
      "refs": [
        "XGD",
        "M"
      ],
      "scope": "剧情磨坊主 / 无尽另核",
      "forms": "miller_B",
      "goal": "encounter:miller",
      "raw_skills": "瘟疫瘟毒炸裂/手雷、医疗；破盾穿刺/穿刺牵引；赏金收集赏金与标记；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。剧情磨坊主 / 无尽另核。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "有足够前置波次恢复且想兼顾腐蚀与爆发 → 瘟疫/破盾叠腐蚀、破盾解守护，赏金按可见标记输出，修女保血。",
      "risk": "破盾移动会换赏金的前排位置；无战中减压职业，长线受限。标记不保证始终有效，冰冻守护可打断爆发。",
      "gear_advice": "项链米尔德里德的挂坠只在已有时计入；其余腐蚀成功、命中和治疗。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "shieldbreaker",
          "skills": [
            "pierce",
            "break_guard"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "noxious_blast"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-sleeper-pd_cru",
      "family": "sleeper",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 技能/动作纠偏",
      "basis": "作者明确整队与站位 · 技能/动作纠偏",
      "refs": [
        "XGD",
        "SL",
        "FR"
      ],
      "scope": "沉睡者 / 整段波次待核",
      "forms": "galaxy_D",
      "goal": "encounter:sleeper",
      "raw_skills": "瘟疫医疗/壮胆/手雷；赏金收集赏金、上勾拳/闪光弹用于道中；十字军惩击/战斗治疗；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。沉睡者 / 整段波次待核。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "想用双恢复与不同控制处理前置波次 → 双前排直伤、瘟疫 DOT/增益，修女群疗；保持能打裂变结晶。",
      "risk": "没有专职战中减压；不能把道中强晕当能一直控制沉睡者。应评估在高压力前结束合同，不默认千杀可行。",
      "gear_advice": "建议治疗、命中、瘟疫成功率；给赏金的项链只对磨坊主有针对性，不能当沉睡者增伤。",
      "slots": [
        {
          "rank": 1,
          "class": "crusader",
          "skills": [
            "smite",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-dd1-jestal",
      "family": "dd1",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位与四技能推断",
      "basis": "玩家自述整队 · 站位与四技能推断",
      "refs": [
        "R",
        "D1"
      ],
      "scope": "极暗一整趟",
      "forms": "shuffler_D",
      "goal": "encounter:dd1",
      "raw_skills": "强盗决斗突刺/零距离、手枪；恶女突破/流血攻击；小丑短剑突刺/战歌/减压；修女双疗。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗一整趟。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "希望有稳定血压恢复且名册适合 → 修女/小丑撑道中，强盗反击与移动，恶女流血；乱位后先恢复可施放能力。",
      "risk": "修女进1号位会断奶；强盗反击可能误杀祭司触发新召唤。和原神秘/十字军方案相比，恢复更集中、乱位脆弱性也更明显。",
      "gear_advice": "建议命中、抗 DOT 与治疗/小丑生存；先决定是否允许为复位付出行动。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "if_it_bleeds",
            "breakthru"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "highwayman",
          "skills": [
            "pistol_shot",
            "duelist_advance",
            "point_blank_shot"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "dirk_stab"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-templar-redundant",
      "family": "templar",
      "decision": "采纳",
      "user_note": "",
      "source": "原作者明确整队/站位 · 技能组另标",
      "basis": "原作者明确整队/站位 · 技能组另标",
      "refs": [
        "XT1"
      ],
      "scope": "极暗二整趟（三处圣殿遭遇）",
      "forms": "templar_melee_D",
      "goal": "encounter:templar",
      "raw_skills": "必带老兵守护/盾击、狗守护/闷棍/猎犬冲锋；弩手狙击/战地绷带（照明弹为允许升光时的解晕分支）；修女双疗。",
      "raw_loadout": "来源明确核心：老兵守护+盾击；狗2号位闷棍/冲锋/守护，第四格按自疗或恢复期减压替换；弩手狙击、战地绷带、标记，允许升光才考虑照明弹；修女双疗+审判+眩目圣光。Boss优先保守护；道中需要控制与尽快削祭司。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗二整趟（三处圣殿遭遇）。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "三枚护符装备正确且至少一名守护者先动 → 优先保护修女，狗可先应急接护再输出/控制，弩手分担伤害和恢复。",
      "risk": "狗2号位不能减压，需在安全窗口调整/换技能；守护者双晕或先手失败仍可能致命。来源推崇冗余，不等于真的免疫 RNG。",
      "gear_advice": "火焰护符给弩手、狗、老兵，修女双治疗饰品；建议营地行军计划/战术准备先手，速度饰品按实际缺口补。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "rampart"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "blackjack"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort",
            "judgement",
            "dazzling_light"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage",
            "flare"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-templar-mark_stun",
      "family": "templar",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位/技能由我安排",
      "basis": "玩家自述整队 · 站位/技能由我安排",
      "refs": [
        "XT2",
        "D2"
      ],
      "scope": "极暗二整趟（三处圣殿遭遇）",
      "forms": "templar_melee_D",
      "goal": "encounter:templar",
      "raw_skills": "神秘深渊之手、治疗、标记；狗猎犬冲锋/守护；弩手狙击/战地绷带；老兵守护。",
      "raw_loadout": "整理者建议四技能：4弩手狙击、战地绷带、标记、盲射；3狗冲锋、守护、口哨、群体减压；2神秘深渊之手、治疗、标记、虚弱诅咒；1老兵守护、盾击、指挥、战术增益。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗二整趟（三处圣殿遭遇）。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "神秘能在前排安全施法 → 用强晕减少关键敌方行动，狗/弩手标记爆发；老兵持续保护未带护符的神秘。",
      "risk": "前排神秘脆、0奶和自身流血有风险；老兵还没护住前尤危险。不能把已知行动规律当可见实际队列，不能按晕必成规划。",
      "gear_advice": "三枚护符给弩手、狗、老兵；神秘剩余饰品可补晕率与生存。作者提锅/沙瓶，但不保证达到100%晕。",
      "slots": [
        {
          "rank": 1,
          "class": "man_at_arms",
          "skills": [
            "defender",
            "command",
            "bolster",
            "rampart"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "weakening_curse",
            "hands_from_abyss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [
            "howl",
            "whistle"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage",
            "blindfire"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-templar-rear_maa",
      "family": "templar",
      "decision": "条件采纳",
      "user_note": "",
      "source": "来源明确站位/四技能/装备 · 有险情",
      "basis": "来源明确站位/四技能/装备 · 有险情",
      "refs": [
        "XT3"
      ],
      "scope": "极暗二整趟 · 站位依赖高",
      "forms": "templar_melee_D",
      "goal": "encounter:templar",
      "raw_skills": "这个站位必须特殊技能组：老兵不带不可用的盾击/反击；修女2号位主要用群疗，单疗不可用。",
      "raw_loadout": "来源明确四技能：4老兵咆哮、守护、指挥、战术增益；3小丑收获、切除、战歌、振奋旋律；2修女钉锤打击、圣光之手、群疗、单疗（单疗此位置无法用，仅复位后）；1恶女劈砍、钢铁天鹅、流血攻击、肾上腺素。不可同时给修女算上单疗能力。",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗二整趟 · 站位依赖高。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "接受把老兵固定4号位并用前排修女群疗 → 老兵护修女、指挥，恶女主输出，小丑支援血压；先算首轮露空。",
      "risk": "作者实际遇到圣殿先手让修女进死门。恶女被推后/修女被挤到1号位会大幅失去输出与治疗；评论明确反驳其“完美”说法。",
      "gear_advice": "来源老兵护符+守护者之盾；小丑护符+先祖卷轴，恶女护符+蜡烛，修女双治疗；高光饰品严格无光时不能照搬。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds",
            "adrenaline_rush",
            "wicked_hack"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort",
            "mace_bash",
            "hand_of_light"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune",
            "slice_off",
            "harvest"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "man_at_arms",
          "skills": [
            "command",
            "bolster",
            "bellow"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-cell-pd_flag",
      "family": "cell",
      "decision": "条件采纳",
      "user_note": "治疗可能不足？",
      "source": "玩家自述整队 · 挑战限制已标",
      "basis": "玩家自述整队 · 挑战限制已标",
      "refs": [
        "XD3",
        "D3"
      ],
      "scope": "极暗三整趟",
      "forms": "cell_white_D",
      "goal": "encounter:cell",
      "raw_skills": "瘟疫后排双晕/手雷/医疗；恶女钢铁天鹅、咆哮；苦修苦痛之雨、重生/救赎；狗冲锋/减压。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗三整趟。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "想用多路后排控制/伤害和苦修续航 → 先压白细胞柄，再处理胞囊；狗与营火承担压力。",
      "risk": "原帖来源有每职业一次的自限，不是普通档必须无修女。苦修低血技能不能随时奶；控制失败/手雷未命中可能放出传送。",
      "gear_advice": "建议后排处理者命中/速度、瘟疫晕率，苦修抗自身流血；大补给与营地恢复另算。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "flagellant",
          "skills": [
            "rain_of_sorrows",
            "redeem",
            "reclaim"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "houndmaster",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-cell-pd_jester",
      "family": "cell",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述整队 · 站位/技能核对",
      "basis": "玩家自述整队 · 站位/技能核对",
      "refs": [
        "XD3",
        "D3"
      ],
      "scope": "极暗三整趟",
      "forms": "cell_white_D",
      "goal": "encounter:cell",
      "raw_skills": "瘟疫致盲毒气/手雷/战地医疗；神秘深渊之手/治疗；恶女钢铁天鹅；小丑战歌/减压。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗三整趟。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "名册有前排神秘且可接受单大奶波动 → 后排瘟疫控/毒，小丑稳命中压力，神秘晕与治疗，恶女直打白细胞柄。",
      "risk": "无修女群疗，神秘被晕会断恢复；白细胞柄没能及时压制就可能延长整趟任务。不是一招双晕无限锁。",
      "gear_advice": "建议神秘生存、瘟疫速度/晕率、输出命中；有解毒剂和医疗兜底 DOT。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [
            "hands_from_abyss"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "jester",
          "skills": [
            "battle_ballad",
            "inspiring_tune"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "plague_doctor",
          "skills": [
            "plague_grenade",
            "battlefield_medicine",
            "blinding_gas"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-heart-mark_four",
      "family": "heart",
      "decision": "采纳",
      "user_note": "",
      "source": "玩家自述四人队 · 站位与技能推断",
      "basis": "玩家自述四人队 · 站位与技能推断",
      "refs": [
        "R",
        "D4"
      ],
      "scope": "极暗四正常四人",
      "forms": "ancestor_heart_D",
      "goal": "encounter:heart",
      "raw_skills": "赏金收集赏金；神秘标记/治疗；弩手狙击/战地绷带；十字军战斗治疗、圣矛和近战。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗四正常四人。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "选择正常四人推进 → 道中/先祖阶段用标记与恢复，心脏阶段随当前状态决定谁留下，确保最后两人仍能输出和互救。",
      "risk": "十字军圣矛会挤动神秘、影响前排晕；牺牲使远程位前移可能废技能，不能固定把辅助先送走。沿用你的意见，不新增双人/一刀成就流。",
      "gear_advice": "建议命中、伤害与恢复；考虑牺牲后的站位，不默认仍有四人的标记回报。",
      "slots": [
        {
          "rank": 1,
          "class": "bounty_hunter",
          "skills": [
            "collect_bounty"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "occultist",
          "skills": [],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "crusader",
          "skills": [
            "holy_lance",
            "battle_heal"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "arbalest",
          "skills": [
            "sniper_shot",
            "battlefield_bandage"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    },
    {
      "id": "BX-heart-vestal_reach",
      "family": "heart",
      "decision": "采纳",
      "user_note": "",
      "source": "作者明确整队与站位 · 动作由我整理",
      "basis": "作者明确整队与站位 · 动作由我整理",
      "refs": [
        "XGD",
        "D4"
      ],
      "scope": "极暗四正常四人",
      "forms": "ancestor_heart_D",
      "goal": "encounter:heart",
      "raw_skills": "修女双疗；盗墓投掷匕首/当面一击/机动；狗猎犬冲锋、守护、自疗；恶女钢铁天鹅/流血攻击。",
      "raw_loadout": "",
      "conditions": "本轮新增，未由玩家标定；不是跨全部难度验证。极暗四正常四人。通用门槛沿用 W-019；英雄可用性、刀甲/技能强化、装备和完整道中必须一起评估。",
      "rule": "希望先有稳定群疗与多路伤害 → 前阶段保状态，最终以可见血压和剩余技能选择牺牲与输出。",
      "risk": "失去修女后要靠自疗/剩余恢复续局；狗站1会失去冲锋。不要照抄旧文固定牺牲某两人的建议。",
      "gear_advice": "建议命中与存活，狗/恶女自疗供残局；不假设有大量蛇鳞或一次爆发跳过剧情。",
      "slots": [
        {
          "rank": 1,
          "class": "hellion",
          "skills": [
            "iron_swan",
            "if_it_bleeds"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 2,
          "class": "houndmaster",
          "skills": [
            "hounds_rush",
            "guard_dog"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 3,
          "class": "grave_robber",
          "skills": [
            "pick_to_the_face",
            "thrown_dagger"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        },
        {
          "rank": 4,
          "class": "vestal",
          "skills": [
            "divine_grace",
            "gods_comfort"
          ],
          "selection": "minimum named skills; remaining slots use verified functional loadout"
        }
      ],
      "unresolved": []
    }
  ],
  "policy": {
    "prophet_champion_debuff_bonus": 100,
    "stress_weight": 0.01,
    "vvulf_min_level": 6,
    "vvulf_min_gear": 4,
    "sources": [
      "W-105"
    ],
    "candidate": "static roster fit; not empirical win probability"
  }
}
"#.to_string()
        };
        let c: Self = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if c.version != "boss-parties-025-v1"
            || c.plans.is_empty()
            || c.plans.len() > 500
            || c.plans.iter().map(|p| &p.id).collect::<BTreeSet<_>>().len() != c.plans.len()
        {
            return Err("invalid025 boss candidate catalog".into());
        }
        Ok(c)
    }
    pub fn family<'a>(&'a self, q: &QuestView) -> Option<&'a str> {
        let plot = q.plot.as_ref()?;
        self.families
            .iter()
            .find(|(_, prefixes)| prefixes.iter().any(|p| plot.starts_with(p)))
            .map(|(f, _)| f.as_str())
    }
}
pub fn checks(game: &GameData, c: &Catalog) -> Result<BTreeMap<String, Value>> {
    let heroes = game.hero_ids().cloned().collect::<BTreeSet<_>>();
    let mut out = BTreeMap::new();
    for p in &c.plans {
        let mut errors = p.unresolved.clone();
        let mut details = vec![];
        let ranks = p.slots.iter().map(|s| s.rank).collect::<BTreeSet<_>>();
        if p.slots.len() != 4 || ranks != BTreeSet::from([1, 2, 3, 4]) {
            errors.push("requires four distinct ranks; achievements/duos are excluded".into())
        }
        for member in &p.slots {
            if !heroes.contains(&member.class_id) {
                errors.push(format!(
                    "class absent from enabled content: {}",
                    member.class_id
                ));
                continue;
            }
            let ids = game.hero_skill_ids(&member.class_id)?;
            if member.skills.is_empty() {
                errors.push(format!(
                    "{} rank{}: source supplies no unambiguous minimum skill IDs",
                    member.class_id, member.rank
                ));
            }
            if member.skills.len() > 4 {
                errors.push(format!("{} rank{}: more than four named skills; alternatives need explicit data selection",member.class_id,member.rank));
            }
            let mut usable = false;
            for id in &member.skills {
                if !ids.contains(id) {
                    errors.push(format!("unknown original skill {}.{id}", member.class_id));
                    continue;
                }
                let bp = game.hero(&member.class_id, 0, 0, &[id])?;
                let s = bp
                    .skills
                    .iter()
                    .find(|s| &s.id == id)
                    .ok_or("025 boss skill projection absent")?;
                usable |= s.launch.contains(&member.rank);
                details.push(json!({"class":member.class_id,"rank":member.rank,"skill":id,"launch":s.launch,"targets":s.target.ranks,"friendly":s.target.friendly,"initial_rank_legal":s.launch.contains(&member.rank)}));
            }
            if !usable {
                errors.push(format!(
                    "{} rank{}: no named skill usable at initial station",
                    member.class_id, member.rank
                ));
            }
        }
        let selected = ["采纳", "条件采纳"].contains(&p.decision.as_str());
        let preplan = c.families.contains_key(&p.family);
        out.insert(p.id.clone(),json!({"usable":errors.is_empty(),"decision_enabled":selected,"public_quest_family":preplan,"errors":errors,"skills":details,"note":"Off-rank named skills are backup only; no contribution is counted at an illegal station."}));
    }
    Ok(out)
}
pub(super) fn matches_scope(p: &Plan, q: &QuestView) -> bool {
    let tier = if q.difficulty == 1 {
        "A"
    } else if q.difficulty == 3 {
        "B"
    } else {
        "C"
    };
    if p.id.starts_with("BP-")
        && let Some(v) = p.id.split('-').nth(2)
        && ["A", "B", "C"].contains(&v)
    {
        return v == tier;
    }
    if p.id.starts_with("BX-") && p.scope.contains("学徒/精英") && q.difficulty >= 5 {
        return false;
    }
    true
}

pub fn party(
    t: &crate::town016::Observation,
    q: &QuestView,
    ctx: &Context<'_>,
) -> Option<(String, Vec<u64>)> {
    if !ctx.on("boss") {
        return None;
    }
    if ctx.profile.parameters["sync026"]["boss"]["enabled"] == true {
        return super::sync026::boss_flex::party(t, q, ctx);
    }
    let family = ctx.profile.boss_parties.family(q)?;
    let mut choices = vec![];
    for p in ctx.profile.boss_parties.plans.iter().filter(|p| {
        p.family == family
            && ["采纳", "条件采纳"].contains(&p.decision.as_str())
            && matches_scope(p, q)
            && ctx
                .sources
                .boss_checks
                .get(&p.id)
                .is_some_and(|v| v["usable"] == true)
    }) {
        let mut ids = vec![];
        let mut score = 0.;
        let mut slots = p.slots.iter().collect::<Vec<_>>();
        slots.sort_by_key(|m| m.rank);
        for member in slots {
            let mut candidates=t.heroes.iter().filter(|h|h.alive&&!h.busy&&!h.missing&&!h.quirks.contains_key("shard_hungry")&&h.class==member.class_id&&!ids.contains(&h.guid)&&crate::preparation024::composition::compatible(t,&ids,h)&&member.skills.iter().all(|s|h.skills.contains_key(s)&& (ctx.on("skills")||h.selected.contains(s))||ctx.on("skills")&&t.actions.iter().any(|c|matches!(&c.action,TownAction::Upgrade{hero:Some(id),tree,..}if *id==h.guid&&*tree==format!("{}.{}",h.class,s))))).collect::<Vec<&VisibleHero>>();
            candidates.sort_by_key(|h| (std::cmp::Reverse(h.level), h.stress, h.guid));
            if let Some(h) = candidates.first() {
                if family == "vvulf"
                    && (h.level
                        < ctx.profile.boss_parties.policy["vvulf_min_level"]
                            .as_u64()
                            .unwrap() as u32
                        || h.weapon
                            < ctx.profile.boss_parties.policy["vvulf_min_gear"]
                                .as_u64()
                                .unwrap() as usize
                        || h.armour
                            < ctx.profile.boss_parties.policy["vvulf_min_gear"]
                                .as_u64()
                                .unwrap() as usize)
                {
                    break;
                }
                score += f64::from(h.level) + h.weapon as f64 + h.armour as f64
                    - f64::from(h.stress)
                        * ctx.profile.boss_parties.policy["stress_weight"]
                            .as_f64()
                            .unwrap();
                ids.push(h.guid);
            } else {
                break;
            }
        }
        if ids.len() != 4 {
            continue;
        }
        if family == "prophet" && q.difficulty >= 5 {
            let debuff = p
                .slots
                .iter()
                .flat_map(|s| s.skills.iter())
                .filter(|s| ["intimidate", "weakening_curse"].contains(&s.as_str()))
                .count();
            score += debuff as f64
                * ctx.profile.boss_parties.policy["prophet_champion_debuff_bonus"]
                    .as_f64()
                    .unwrap();
        }
        choices.push((score, p.id.clone(), ids));
    }
    choices.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    choices.into_iter().next().map(|(_, id, p)| (id, p))
}
pub fn identify(
    t: &crate::town016::Observation,
    q: &QuestView,
    ids: &[u64],
    ctx: &Context<'_>,
) -> Option<String> {
    let mut exact = t.clone();
    exact.heroes.retain(|h| ids.contains(&h.guid));
    let (id, p) = party(&exact, q, ctx)?;
    (p == ids).then_some(id)
}
pub fn desired(h: &VisibleHero, ctx: &Context<'_>) -> Option<Vec<String>> {
    if !ctx.on("boss") {
        return None;
    }
    let (id, party) = ctx.sources.current_boss.as_ref()?;
    let rank = party.iter().position(|id| *id == h.guid)? as u8 + 1;
    if h.class == "vestal"
        && rank == 3
        && ctx.profile.parameters["sync026"]["boss"]["enabled"] == true
    {
        return Some(
            ctx.profile.parameters["sync026"]["boss"]["vestal_rank3_default"]
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        );
    }
    let p = ctx
        .profile
        .boss_parties
        .plans
        .iter()
        .find(|p| &p.id == id)?;
    let m = p
        .slots
        .iter()
        .find(|m| m.rank == rank && m.class_id == h.class)?;
    Some(m.skills.clone())
}

/// Report-only candidate diagnosis uses the same selector with one plan at a time.
/// It neither changes the main choice nor reconstructs an unscouted encounter.
pub fn diagnose(t: &crate::town016::Observation, q: &QuestView, ctx: &Context<'_>) -> Vec<Value> {
    let Some(family) = ctx.profile.boss_parties.family(q) else {
        return vec![];
    };
    let mut rows = vec![];
    for plan in ctx
        .profile
        .boss_parties
        .plans
        .iter()
        .filter(|p| p.family == family)
    {
        let (reason, details) = if !["采纳", "条件采纳"].contains(&plan.decision.as_str()) {
            ("user_decision_disabled", json!(plan.decision))
        } else if !ctx
            .sources
            .boss_checks
            .get(&plan.id)
            .is_some_and(|v| v["usable"] == true)
        {
            (
                "source_check_failed",
                ctx.sources
                    .boss_checks
                    .get(&plan.id)
                    .cloned()
                    .unwrap_or(Value::Null),
            )
        } else if !matches_scope(plan, q) {
            ("difficulty_scope", json!(plan.scope))
        } else if q.execution == "expedition integration pending"
            || !q
                .estate_requirements
                .iter()
                .all(|(key, n)| t.wallet.get(key).copied().unwrap_or(0) >= *n)
        {
            (
                "quest_not_executable",
                json!({"execution":q.execution,"requirements":q.estate_requirements}),
            )
        } else {
            let mut p = ctx.profile.clone();
            p.boss_parties.plans = vec![plan.clone()];
            let candidate = Context {
                profile: &p,
                sources: ctx.sources,
                enabled: ctx.enabled,
                ordered: ctx.ordered,
            };
            match party(t, q, &candidate) {
                Some((_, ids)) => ("eligible_candidate", json!({"party":ids})),
                None => (
                    "roster_skills_or_conditions",
                    json!({"eligible_class_counts":plan.slots.iter().map(|m|(&m.class_id,t.heroes.iter().filter(|h|h.class==m.class_id&&h.alive&&!h.busy&&!h.missing).count())).collect::<BTreeMap<_,_>>(),"named_skills":plan.slots}),
                ),
            }
        };
        rows.push(
            json!({"id":plan.id,"quest":q.id,"family":family,"reason":reason,"details":details}),
        );
    }
    rows
}
