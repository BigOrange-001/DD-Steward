//! Handwritten combat candidates. Evaluation sees only the filtered Observation.
//! Static identifiers are validated by the host; no enemy models are retained.
use crate::{Action, LegalAction, Observation, VisibleUnit};
use dd_data::combat::{GameData, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: &str = "visible-combat-021-v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub version: String,
    pub threats: Vec<String>,
    pub safe_tail: Vec<String>,
    pub rules: Vec<Rule>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub priority: u32,
    pub classes: Vec<String>,
    pub encounters: Vec<String>,
    pub skills: Vec<SkillRef>,
    pub items: Vec<String>,
    pub condition: Condition,
    pub action: Intent,
    pub parameters: Parameters,
    pub rationale: String,
    pub source: String,
    pub applicable_version: String,
    pub difficulty: String,
    pub validation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRef {
    pub class: String,
    pub id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Condition {
    Rescue,
    Cleanse,
    Retreat,
    Kill,
    Stun,
    Stress,
    Tail,
    Heal,
    Focus,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Intent {
    Heal,
    Cleanse,
    Retreat,
    Attack,
    Stun,
    StressHeal,
    Recover,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub hp_fraction: f64,
    pub stress: i32,
    pub recovery_stress: i32,
    pub max_round: u32,
    pub min_targets: usize,
    pub party_limit: usize,
    pub min_hit: f64,
}
#[derive(Debug, Clone)]
pub struct Rules {
    document: Document,
    hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    pub version: String,
    pub recovery_used: bool,
    pub last_round: u32,
    pub wave: usize,
}
impl Default for Memory {
    fn default() -> Self {
        Self {
            version: VERSION.into(),
            recovery_used: false,
            last_round: 0,
            wave: 0,
        }
    }
}
impl Memory {
    pub fn validate(&self) -> Result<()> {
        if self.version != VERSION {
            return Err("unknown 021 combat memory".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Candidate {
    pub action: Action,
    pub rule_id: String,
    pub priority: u32,
    pub score: f64,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Evaluation {
    pub choice: Option<Candidate>,
    pub memory: Memory,
}

impl Rules {
    pub fn documented(game: &GameData) -> Result<Self> {
        Self::parse(r#"{
  "version": "visible-combat-021-v1",
  "threats": [
    "skeleton_courtier_A",
    "skeleton_courtier_B",
    "skeleton_courtier_C",
    "cultist_witch_A",
    "cultist_witch_B",
    "cultist_witch_C",
    "madman_A",
    "madman_B",
    "madman_C"
  ],
  "safe_tail": [
    "skeleton_defender_A",
    "skeleton_common_A"
  ],
  "rules": [
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-01",
      "priority": 10,
      "condition": "Rescue",
      "action": "Heal",
      "rationale": "濒死英雄先用当前合法治疗脱离0HP；只比较可见区间，不预知是否暴击。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Vestal/Strategy"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [
        {
          "class": "plague_doctor",
          "id": "battlefield_medicine"
        }
      ],
      "items": [
        "bandage",
        "antivenom"
      ],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-02",
      "priority": 20,
      "condition": "Cleanse",
      "action": "Cleanse",
      "rationale": "低生命且持续伤害存在时清除对应DoT；瘟疫医生可用原技能救援。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Getting_Started"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 2,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-03",
      "priority": 30,
      "condition": "Retreat",
      "action": "Retreat",
      "rationale": "多人濒死或残队受伤且仍有多个敌人时尝试合法撤退，不假定一定成功。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Vestal/Strategy"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "2015 Early Access tactical direction only; current skills and numbers from Observation",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-04",
      "priority": 40,
      "condition": "Kill",
      "action": "Attack",
      "rationale": "直接伤害下界可击杀且无守护/阻挡时，优先减少活敌数量；不是保证命中。",
      "source": "https://steamcommunity.com/sharedfiles/filedetails/?id=385431020"
    },
    {
      "classes": [
        "plague_doctor",
        "vestal",
        "crusader"
      ],
      "encounters": [],
      "skills": [
        {
          "class": "plague_doctor",
          "id": "blinding_gas"
        },
        {
          "class": "plague_doctor",
          "id": "disorienting_blast"
        },
        {
          "class": "vestal",
          "id": "dazzling_light"
        },
        {
          "class": "crusader",
          "id": "stunning_blow"
        }
      ],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-05",
      "priority": 50,
      "condition": "Stun",
      "action": "Stun",
      "rationale": "早期轮次对未眩晕且无可见恢复抗性标记的高威胁目标使用已装备控制。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Plague_Doctor_(Darkest_Dungeon)/Strategy"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 80,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-06",
      "priority": 60,
      "condition": "Stress",
      "action": "StressHeal",
      "rationale": "压力接近折磨阈值时用合法减压；不改变配队或城镇运营。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Stress"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.75,
        "stress": 80,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-07",
      "priority": 70,
      "condition": "Tail",
      "action": "Recover",
      "rationale": "仅前两轮单个白名单敌人已眩晕且队伍无濒死高压时，一场最多一次规则回复。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Stalling"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-08",
      "priority": 80,
      "condition": "Heal",
      "action": "Heal",
      "rationale": "极低生命时进行可见有效治疗；紧急程度低于救援、撤退和直接击杀。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Vestal/Strategy"
    },
    {
      "classes": [],
      "encounters": [],
      "skills": [],
      "items": [],
      "parameters": {
        "hp_fraction": 0.25,
        "stress": 180,
        "max_round": 2,
        "min_targets": 1,
        "party_limit": 2,
        "min_hit": 0.65,
        "recovery_stress": 20
      },
      "applicable_version": "DD1 PvE; current local full content; community patch unspecified",
      "difficulty": "radiant/strict evaluated; other modes unvalidated",
      "validation": "未验证",
      "id": "T021-09",
      "priority": 90,
      "condition": "Focus",
      "action": "Attack",
      "rationale": "集火活跃的已知压力威胁；同优先级按命中预览和剩余生命选择。",
      "source": "https://darkestdungeon.wiki.gg/wiki/Getting_Started"
    }
  ]
}
"#, game)
    }
    pub fn parse(text: &str, game: &GameData) -> Result<Self> {
        let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if document.version != VERSION || document.rules.is_empty() || document.rules.len() > 100 {
            return Err("invalid 021 table version/rule count".into());
        }
        let monsters = game.monster_ids().cloned().collect::<BTreeSet<_>>();
        let heroes = game.hero_ids().cloned().collect::<BTreeSet<_>>();
        let mut ids = BTreeSet::new();
        let mut priorities = BTreeSet::new();
        let check_monsters = |values: &[String]| -> Result<()> {
            if values.iter().any(|id| !monsters.contains(id)) {
                return Err("unknown 021 encounter ID".into());
            }
            Ok(())
        };
        check_monsters(&document.threats)?;
        check_monsters(&document.safe_tail)?;
        for r in &document.rules {
            if !r.id.starts_with("T021-")
                || r.id.len() != 7
                || !r.id[5..].bytes().all(|v| v.is_ascii_digit())
                || !ids.insert(&r.id)
                || !priorities.insert(r.priority)
                || r.priority > 1000
                || [
                    &r.rationale,
                    &r.source,
                    &r.applicable_version,
                    &r.difficulty,
                ]
                .iter()
                .any(|s| s.trim().is_empty())
                || !["未验证", "开发种子有效", "开发种子无效", "疑似还原度问题"]
                    .contains(&r.validation.as_str())
            {
                return Err("invalid 021 rule identity/source/validation".into());
            }
            if r.classes.iter().any(|id| !heroes.contains(id)) {
                return Err("unknown 021 class ID".into());
            }
            check_monsters(&r.encounters)?;
            for s in &r.skills {
                if !heroes.contains(&s.class)
                    || !game.hero_skill_ids(&s.class)?.contains(&s.id)
                    || (!r.classes.is_empty() && !r.classes.contains(&s.class))
                {
                    return Err("unknown or inconsistent 021 skill ID".into());
                }
            }
            for item in &r.items {
                game.supply(item)?;
            }
            let p = &r.parameters;
            if !p.hp_fraction.is_finite()
                || !(0.0..=1.0).contains(&p.hp_fraction)
                || !p.min_hit.is_finite()
                || !(0.0..=1.0).contains(&p.min_hit)
                || !(0..=200).contains(&p.stress)
                || !(0..=200).contains(&p.recovery_stress)
                || !(1..=100).contains(&p.max_round)
                || !(1..=4).contains(&p.min_targets)
                || !(1..=4).contains(&p.party_limit)
            {
                return Err("021 rule parameter outside bounds".into());
            }
            let expected = match r.condition {
                Condition::Rescue | Condition::Heal => Intent::Heal,
                Condition::Cleanse => Intent::Cleanse,
                Condition::Retreat => Intent::Retreat,
                Condition::Kill | Condition::Focus => Intent::Attack,
                Condition::Stun => Intent::Stun,
                Condition::Stress => Intent::StressHeal,
                Condition::Tail => Intent::Recover,
            };
            if r.action != expected || (r.condition == Condition::Stun && r.skills.is_empty()) {
                return Err("inconsistent 021 condition/action".into());
            }
        }
        Ok(Self {
            document,
            hash: format!("sha256:{:x}", Sha256::digest(text.as_bytes())),
        })
    }
    pub fn hash(&self) -> &str {
        &self.hash
    }
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// All matching legal candidates, ordered by priority then visible benefit.
    /// Reusable by future candidate filtering; no simulator or host handle exists here.
    pub fn candidates(&self, o: &Observation, memory: &Memory) -> Result<Vec<Candidate>> {
        memory.validate()?;
        // player_selection denotes a special sacrifice prompt, not an ordinary hero turn.
        if o.player_selection || o.outcome.is_some() {
            return Ok(vec![]);
        }
        let Some(actor) = o.units.iter().find(|u| Some(u.id) == o.actor) else {
            return Ok(vec![]);
        };
        if !friendly(actor) {
            return Ok(vec![]);
        }
        let friends = o.units.iter().filter(|u| friendly(u)).collect::<Vec<_>>();
        let enemies = o
            .units
            .iter()
            .filter(|u| !u.hero && !u.corpse)
            .collect::<Vec<_>>();
        let mut candidates = vec![];
        for r in &self.document.rules {
            if !r.classes.is_empty() && !r.classes.contains(&actor.class) {
                continue;
            }
            if !r.encounters.is_empty() && !enemies.iter().any(|u| r.encounters.contains(&u.class))
            {
                continue;
            }
            for a in &o.actions {
                if !r.skills.is_empty()
                    && !r
                        .skills
                        .iter()
                        .any(|s| s.class == actor.class && s.id == a.label)
                    && !r.items.contains(&a.label)
                {
                    continue;
                }
                if matches!(a.action, Action::Item { .. }) && !r.items.contains(&a.label) {
                    continue;
                }
                if let Some(score) = self.benefit(r, a, o, memory, &friends, &enemies) {
                    candidates.push(Candidate {
                        action: a.action.clone(),
                        rule_id: r.id.clone(),
                        priority: r.priority,
                        score,
                    });
                }
            }
        }
        candidates.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| b.score.total_cmp(&a.score))
        });
        Ok(candidates)
    }
    pub fn evaluate(&self, o: &Observation, memory: &Memory) -> Result<Evaluation> {
        memory.validate()?;
        let mut next = memory.clone();
        if o.round < next.last_round || o.wave != next.wave {
            next.recovery_used = false;
        }
        next.last_round = o.round;
        next.wave = o.wave;
        let choice = self.candidates(o, &next)?.into_iter().next();
        if choice.as_ref().is_some_and(|c| {
            self.document
                .rules
                .iter()
                .any(|r| r.id == c.rule_id && r.condition == Condition::Tail)
        }) {
            next.recovery_used = true;
        }
        Ok(Evaluation {
            choice,
            memory: next,
        })
    }
    fn benefit(
        &self,
        r: &Rule,
        a: &LegalAction,
        o: &Observation,
        m: &Memory,
        friends: &[&VisibleUnit],
        enemies: &[&VisibleUnit],
    ) -> Option<f64> {
        let p = &r.parameters;
        let targets = o
            .units
            .iter()
            .filter(|u| a.targets.contains(&u.id))
            .collect::<Vec<_>>();
        let allied = targets
            .iter()
            .copied()
            .filter(|u| friendly(u))
            .collect::<Vec<_>>();
        let hostile = targets
            .iter()
            .copied()
            .filter(|u| !u.hero && !u.corpse)
            .collect::<Vec<_>>();
        let skill = matches!(a.action, Action::Skill { .. } | Action::SkillAuto { .. });
        let healing = skill && a.heal[1] > 0;
        let attack = skill && a.damage[1] > 0 && a.hit_chance >= p.min_hit;
        let healing_value = || {
            allied
                .iter()
                .map(|u| f64::from((u.max_hp - u.hp).min(a.heal[1]).max(0)))
                .sum::<f64>()
        };
        let stress_value = || {
            allied
                .iter()
                .map(|u| f64::from(u.stress.min(a.stress_heal).max(0)))
                .sum::<f64>()
        };
        let low = |u: &&VisibleUnit| f64::from(u.hp) <= f64::from(u.max_hp) * p.hp_fraction;
        match r.condition {
            Condition::Rescue if healing && allied.iter().any(|u| u.hp == 0) => {
                Some(1000. * allied.iter().filter(|u| u.hp == 0).count() as f64 + healing_value())
            }
            Condition::Cleanse if skill || matches!(a.action, Action::Item { .. }) => {
                let n = allied
                    .iter()
                    .filter(|u| {
                        let bleeding = !u.bleed.is_empty();
                        let blighted = !u.blight.is_empty();
                        (u.hp == 0 || low(u))
                            && match a.label.as_str() {
                                "bandage" => bleeding,
                                "antivenom" => blighted,
                                "battlefield_medicine" => bleeding || blighted || u.hp == 0,
                                _ => false,
                            }
                    })
                    .count();
                (n > 0).then_some(n as f64)
            }
            Condition::Retreat if a.action == Action::Retreat => {
                let doors = friends.iter().filter(|u| u.hp == 0).count();
                let vulnerable = friends.iter().any(|u| low(u) || u.stress >= p.stress);
                (doors >= p.min_targets
                    || (friends.len() <= p.party_limit
                        && enemies.len() >= p.min_targets
                        && vulnerable))
                    .then_some(1.)
            }
            Condition::Kill if attack => {
                let kills = hostile
                    .iter()
                    .filter(|u| {
                        a.damage[0] >= u.hp
                            && u.hp > 0
                            && u.guarded_by.is_none()
                            && u.damage_blocks == 0
                    })
                    .count();
                (kills > 0).then_some(kills as f64 * 100. * a.hit_chance)
            }
            Condition::Stun if skill && o.round <= p.max_round && a.hit_chance >= p.min_hit => {
                let eligible = hostile
                    .iter()
                    .filter(|u| {
                        !u.stunned
                            && u.guarded_by.is_none()
                            && self.document.threats.contains(&u.class)
                            && !u
                                .buffs
                                .iter()
                                .any(|(id, _)| id.to_ascii_lowercase().contains("stun"))
                    })
                    .count();
                (eligible >= p.min_targets).then_some(eligible as f64 * a.hit_chance)
            }
            Condition::Stress
                if skill && a.stress_heal > 0 && allied.iter().any(|u| u.stress >= p.stress) =>
            {
                Some(stress_value())
            }
            Condition::Tail
                if !m.recovery_used
                    && o.round <= p.max_round
                    && enemies.len() == 1
                    && enemies[0].stunned
                    && self.document.safe_tail.contains(&enemies[0].class)
                    && friends.len() > p.party_limit
                    && friends.iter().all(|u| u.hp > 0 && u.stress < p.stress)
                    && ((healing && allied.iter().any(low))
                        || (skill
                            && a.stress_heal > 0
                            && allied.iter().any(|u| u.stress >= p.recovery_stress))) =>
            {
                Some(healing_value() + stress_value())
            }
            Condition::Heal if healing && allied.iter().any(low) => Some(healing_value()),
            Condition::Focus if attack => {
                let priority = hostile
                    .iter()
                    .filter(|u| {
                        self.document.threats.contains(&u.class)
                            && !u.stunned
                            && u.guarded_by.is_none()
                    })
                    .collect::<Vec<_>>();
                (!priority.is_empty()).then(|| {
                    priority
                        .iter()
                        .map(|u| {
                            a.hit_chance
                                * (f64::from(a.damage[0] + a.damage[1]) / 2.
                                    + 10. / f64::from(u.hp.max(1)))
                        })
                        .sum()
                })
            }
            _ => None,
        }
    }
}
fn friendly(u: &VisibleUnit) -> bool {
    u.hero && !u.corpse && u.controlled.is_none() && !u.captive.is_some_and(|(_, removed)| removed)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub decisions: u64,
    pub hits: u64,
    pub fallback: u64,
    pub rules: BTreeMap<String, u64>,
    pub classes: BTreeMap<String, [u64; 2]>,
    pub skills: BTreeMap<String, [u64; 2]>,
}
impl Coverage {
    pub fn record(&mut self, o: &Observation, action: &Action, id: Option<&str>) {
        self.decisions += 1;
        let hit = usize::from(id.is_some());
        if let Some(id) = id {
            self.hits += 1;
            *self.rules.entry(id.into()).or_default() += 1;
        } else {
            self.fallback += 1;
        }
        let class = o
            .units
            .iter()
            .find(|u| Some(u.id) == o.actor)
            .map_or("unknown", |u| u.class.as_str());
        self.classes.entry(class.into()).or_default()[hit] += 1;
        let label = o
            .actions
            .iter()
            .find(|a| &a.action == action)
            .map_or("unknown", |a| a.label.as_str());
        self.skills.entry(format!("{class}:{label}")).or_default()[hit] += 1;
    }
}

