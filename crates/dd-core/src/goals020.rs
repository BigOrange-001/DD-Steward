//! Campaign-owned, return-certified Steward history. Policy gets only public progress.
use super::*;
use dd_data::objectives020::{Catalog, PROFILE};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Stamp {
    pub week: u32,
    pub returns: u32,
    pub quest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct KillProof {
    pub at: Stamp,
    pub kills: Vec<(String, String)>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct GrowthProof {
    pub at: Stamp,
    pub hero: u64,
    pub before_xp: u32,
    pub after_xp: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Owned {
    pub profile: String,
    pub catalog: String,
    pub baseline_returns: u32,
    pub last_return: u32,
    pub native_kills: u64,
    pub bosses: BTreeMap<String, KillProof>,
    pub trained: BTreeMap<String, GrowthProof>,
    pub milestones: BTreeMap<String, Stamp>,
    pub mercenaries: BTreeSet<u64>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Goal {
    pub id: String,
    pub label: String,
    pub category: String,
    pub done: bool,
    pub progress: u32,
    pub required: u32,
    pub first_week: Option<u32>,
    pub first_return: Option<u32>,
    pub hero: Option<u64>,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Count {
    pub done: usize,
    pub total: usize,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClassProgress {
    pub ordinary_alive: usize,
    pub ordinary_max_level: Option<u32>,
    pub mercenaries_alive: usize,
    pub earned: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Progress {
    pub classes: BTreeMap<String, ClassProgress>,
    pub profile: String,
    pub full_scope: bool,
    pub complete_configured: bool,
    pub complete: bool,
    pub categories: BTreeMap<String, Count>,
    pub achieved: usize,
    pub required: usize,
    pub week: u32,
    pub returns: u32,
    pub tracked_returns: u32,
    pub completions: u32,
    pub deaths: usize,
    pub native_kills: u64,
    pub goals: Vec<Goal>,
}
pub fn label(id: &str) -> String {
    let clean = id
        .trim_start_matches("boss:")
        .trim_start_matches("kill_")
        .trim_start_matches("class:");
    if let Some(plot) = id.strip_prefix("plot:plot_") {
        return match plot {
            "darkest_dungeon_1" => "极暗地牢：完成第一次任务",
            "darkest_dungeon_2" => "极暗地牢：完成第二次任务",
            "darkest_dungeon_3" => "极暗地牢：完成第三次任务",
            "darkest_dungeon_4" => "极暗地牢：完成最终任务",
            "crimson_court_0" => "庭院：清理初始蜂巢",
            "crimson_court_1" => "庭院：完成男爵任务",
            "crimson_court_2" => "庭院：完成子爵任务",
            "crimson_court_3" => "庭院：完成伯爵夫人任务",
            "color_of_madness_0" => "农庄：完成初次任务",
            "color_of_madness_1" => "农庄：完成磨坊主任务",
            "color_of_madness_2" => "农庄：完成一次无尽收割合同",
            _ => plot,
        }
        .into();
    }
    if id == "repeat:farm_bloodcrate" {
        return "庭院：完成采血任务".into();
    }
    if id == "repeat:kill_statue" {
        return "庭院：完成石像守卫任务".into();
    }
    let (family, variant) = if id.starts_with("class:") {
        (clean, "")
    } else {
        clean.rsplit_once('_').unwrap_or((clean, ""))
    };
    let name = match family {
        "necromancer" => "死灵法师",
        "prophet" => "先知",
        "swine_prince" => "猪人王",
        "formless_flesh" => "血肉",
        "hag" => "女巫",
        "brigand_cannon" => "强盗火炮",
        "siren" => "塞壬",
        "drowned_crew" => "溺亡船员",
        "collector" => "收藏家",
        "shambler" => "蹒跚爬行怪",
        "fanatic" => "狂信徒",
        "thing" => "星空之物",
        "crow" => "尖啸者",
        "brigand_sapper" => "狼袭：沃尔夫",
        "crocodile" | "com_crocodile" => "鳄鱼",
        "statue" => "花园守卫",
        "baron" => "男爵",
        "viscount" => "子爵",
        "countess" => "伯爵夫人",
        "miller" => "磨坊主",
        "galaxy" => "沉睡者",
        "shuffler" => "蹒跚恐魔",
        "ancestor_heart" => "黑暗之心",
        "templar_melee" => "圣殿骑士（近战）",
        "templar_ranged" => "圣殿骑士（远程）",
        "cell_white" => "巨型白细胞",
        "abomination" => "憎恶",
        "antiquarian" => "古董商",
        "arbalest" => "弩手",
        "bounty_hunter" => "赏金猎人",
        "crusader" => "十字军",
        "grave_robber" => "盗墓贼",
        "hellion" => "恶女",
        "highwayman" => "强盗",
        "houndmaster" => "训犬师",
        "jester" => "小丑",
        "leper" => "麻风病人",
        "man_at_arms" => "老兵",
        "occultist" => "神秘学者",
        "plague_doctor" => "瘟疫医生",
        "vestal" => "修女",
        "flagellant" => "鞭笞者",
        "shieldbreaker" => "破盾者",
        "runaway" => "逃离者",
        "duelist" => "决斗家",
        _ => family,
    };
    if id.starts_with("class:") {
        format!("{name}：副本返程升至六级")
    } else {
        format!("{name}（{variant}）")
    }
}
pub fn initial_progress(c: &Catalog) -> Progress {
    let goals = c
        .bosses
        .iter()
        .map(|(id, b)| (id.clone(), "bosses", b.amount))
        .chain(
            c.classes
                .iter()
                .map(|id| (format!("class:{id}"), "classes", 1)),
        )
        .chain(
            c.plots
                .iter()
                .map(|id| (format!("plot:{id}"), "major_content", 1)),
        )
        .chain(
            c.repeats
                .iter()
                .map(|id| (format!("repeat:{id}"), "major_content", 1)),
        )
        .map(|(id, category, required)| Goal {
            label: label(&id),
            id,
            category: category.into(),
            done: false,
            progress: 0,
            required,
            first_week: None,
            first_return: None,
            hero: None,
        })
        .collect::<Vec<_>>();
    let mut categories = BTreeMap::new();
    for g in &goals {
        categories
            .entry(g.category.clone())
            .or_insert(Count { done: 0, total: 0 })
            .total += 1;
    }
    Progress {
        classes: BTreeMap::new(),
        profile: PROFILE.into(),
        full_scope: c.full_scope,
        complete_configured: false,
        complete: false,
        categories,
        achieved: 0,
        required: goals.len(),
        week: 0,
        returns: 0,
        tracked_returns: 0,
        completions: 0,
        deaths: 0,
        native_kills: 0,
        goals,
    }
}
impl Definition {
    pub fn with_steward020(mut self) -> Result<Self> {
        use sha2::{Digest, Sha256};
        if self.content019.is_none() || self.goals020.is_some() {
            return Err("020 requires exactly one explicit 019 base".into());
        }
        let rules = Catalog::load(&self.town.game, &self.data, &self.town.data)?;
        // Keep the 019 owner prefix for its source-owned market/mercenary consumers.
        self.id = format!(
            "{};{PROFILE};sha256:{:x}",
            self.id,
            Sha256::digest(serde_json::to_vec(&rules).map_err(|e| e.to_string())?)
        );
        self.goals020 = Some(Arc::new(rules));
        self.town_view = Arc::new(Mutex::new(None));
        Ok(self)
    }
    pub fn steward_catalog020(&self) -> Option<&Catalog> {
        self.goals020.as_deref()
    }
}
impl Session {
    pub(super) fn init_goals020(&mut self) -> Result<()> {
        if let Some(c) = &self.definition.goals020 {
            self.state.goals020 = Some(Owned {
                profile: PROFILE.into(),
                catalog: c.identity()?,
                baseline_returns: self.state.clock.returns,
                last_return: self.state.clock.returns,
                native_kills: 0,
                bosses: BTreeMap::new(),
                trained: BTreeMap::new(),
                milestones: BTreeMap::new(),
                mercenaries: BTreeSet::new(),
            });
        }
        Ok(())
    }
    pub(super) fn validate_goals020(&self) -> Result<()> {
        let (c, s) = match (&self.definition.goals020, &self.state.goals020) {
            (None, None) => return Ok(()),
            (Some(c), Some(s)) => (c, s),
            _ => return Err(
                "020 objective owner/profile mismatch; legacy history cannot be silently upgraded"
                    .into(),
            ),
        };
        if s.profile != PROFILE
            || s.catalog != c.identity()?
            || s.baseline_returns != 2
            || s.last_return != self.state.clock.returns
            || s.last_return < s.baseline_returns
            || s.bosses.len() > c.bosses.len()
            || s.trained.len() > c.classes.len()
        {
            return Err("invalid 020 source or return boundary".into());
        }
        let stamp = |at: &Stamp| -> Result<()> {
            if at.week > self.state.clock.week
                || at.week < 2
                || at.returns <= s.baseline_returns
                || at.returns > s.last_return
                || at.quest.is_empty()
                || at.quest.len() > 4096
            {
                return Err("invalid 020 achievement boundary".into());
            }
            Ok(())
        };
        for (id, p) in &s.bosses {
            stamp(&p.at)?;
            let b = c.bosses.get(id).ok_or("unknown 020 boss achievement")?;
            if p.kills.len() != b.amount as usize
                || p.kills.iter().any(|(_, last)| !b.targets.contains(last))
                || p.kills.iter().any(|(source, _)| {
                    !self
                        .definition
                        .town
                        .game
                        .monster_ids()
                        .any(|id| id == source)
                })
            {
                return Err("020 boss proof differs from source target/count".into());
            }
        }
        for (class, p) in &s.trained {
            stamp(&p.at)?;
            if !c.classes.contains(class)
                || p.hero == 0
                || p.hero >= self.state.city.next_guid
                || s.mercenaries.contains(&p.hero)
                || p.before_xp >= c.training_xp
                || p.after_xp < c.training_xp
            {
                return Err("invalid 020 return-growth achievement".into());
            }
            if self
                .state
                .city
                .heroes
                .iter()
                .chain(self.state.graveyard.values())
                .any(|h| h.guid == p.hero && h.class != *class)
            {
                return Err("020 growth hero/class differs from retained identity".into());
            }
        }
        if s.mercenaries
            .iter()
            .any(|id| *id == 0 || *id >= self.state.city.next_guid)
        {
            return Err("invalid 020 mercenary provenance".into());
        }
        let mut expected: BTreeSet<String> = c
            .plots
            .iter()
            .filter(|id| self.state.city.completed_ids.contains(*id))
            .map(|id| format!("plot:{id}"))
            .collect();
        if let Some(a) = self
            .state
            .city
            .campaign
            .as_ref()
            .and_then(|c| c.automation.as_ref())
        {
            expected.extend(
                c.repeats
                    .iter()
                    .filter(|id| a.repeat_completions.get(*id).copied().unwrap_or(0) > 0)
                    .map(|id| format!("repeat:{id}")),
            );
        }
        if expected != s.milestones.keys().cloned().collect() {
            return Err("020 milestone evidence differs from native completion history".into());
        }
        for at in s.milestones.values() {
            stamp(at)?;
        }
        Ok(())
    }
    pub(super) fn goals020_return(
        &mut self,
        town: &TownSession,
        q: &board::Instance,
        complete: bool,
        old: &[(u64, String, u32, bool)],
        kills: &[(String, String)],
        new_kills: &[(String, String)],
    ) -> Result<()> {
        let Some(c) = &self.definition.goals020 else {
            return Ok(());
        };
        let s = self
            .state
            .goals020
            .as_mut()
            .ok_or("missing 020 objective history")?;
        if s.last_return.checked_add(1) != Some(self.state.clock.returns) {
            return Err("duplicate or skipped 020 native return".into());
        }
        let at = Stamp {
            week: self.state.clock.week,
            returns: self.state.clock.returns,
            quest: q.id.clone(),
        };
        s.native_kills = s
            .native_kills
            .checked_add(new_kills.len() as u64)
            .ok_or("020 kill counter overflow")?;
        for (id, b) in &c.bosses {
            if s.bosses.contains_key(id)
                || !new_kills.iter().any(|(_, last)| b.targets.contains(last))
            {
                continue;
            }
            let matches = kills
                .iter()
                .filter(|(_, last)| b.targets.contains(last))
                .take(b.amount as usize)
                .cloned()
                .collect::<Vec<_>>();
            if matches.len() == b.amount as usize {
                s.bosses.insert(
                    id.clone(),
                    KillProof {
                        at: at.clone(),
                        kills: matches,
                    },
                );
            }
        }
        for (guid, class, xp, mercenary) in old {
            if *mercenary || s.mercenaries.contains(guid) || *xp >= c.training_xp {
                continue;
            }
            let h = town.hero(*guid)?;
            if h.class != *class {
                return Err("returned hero changed class identity".into());
            }
            if h.state.alive && !h.quirks.contains_key("shard_hungry") && h.xp >= c.training_xp {
                s.trained.entry(class.clone()).or_insert(GrowthProof {
                    at: at.clone(),
                    hero: *guid,
                    before_xp: *xp,
                    after_xp: h.xp,
                });
            }
        }
        if complete {
            if let Some(plot) = q.plot.as_ref().filter(|id| c.plots.contains(*id)) {
                s.milestones
                    .entry(format!("plot:{plot}"))
                    .or_insert(at.clone());
            }
            if let Some(kind) = q.raw["type"].as_str().filter(|k| c.repeats.contains(*k)) {
                s.milestones.entry(format!("repeat:{kind}")).or_insert(at);
            }
        }
        s.last_return = self.state.clock.returns;
        Ok(())
    }
    pub fn steward_progress020(&self) -> Result<Option<Progress>> {
        let Some(c) = &self.definition.goals020 else {
            return Ok(None);
        };
        let s = self.state.goals020.as_ref().ok_or("missing 020 history")?;
        let mut goals = vec![];
        for (id, b) in &c.bosses {
            let proof = s.bosses.get(id);
            goals.push(Goal {
                label: label(id),
                id: id.clone(),
                category: "bosses".into(),
                done: proof.is_some(),
                progress: if proof.is_some() { b.amount } else { 0 },
                required: b.amount,
                first_week: proof.map(|p| p.at.week),
                first_return: proof.map(|p| p.at.returns),
                hero: None,
            });
        }
        for class in &c.classes {
            let proof = s.trained.get(class);
            goals.push(Goal {
                label: label(&format!("class:{class}")),
                id: format!("class:{class}"),
                category: "classes".into(),
                done: proof.is_some(),
                progress: u32::from(proof.is_some()),
                required: 1,
                first_week: proof.map(|p| p.at.week),
                first_return: proof.map(|p| p.at.returns),
                hero: proof.map(|p| p.hero),
            });
        }
        for id in &c.plots {
            let done = self.state.city.completed_ids.contains(id);
            let at = s.milestones.get(&format!("plot:{id}"));
            goals.push(Goal {
                label: label(&format!("plot:{id}")),
                id: format!("plot:{id}"),
                category: "major_content".into(),
                done,
                progress: u32::from(done),
                required: 1,
                first_week: at.map(|at| at.week),
                first_return: at.map(|at| at.returns),
                hero: None,
            });
        }
        for kind in &c.repeats {
            let at = s.milestones.get(&format!("repeat:{kind}"));
            let n = self
                .state
                .city
                .campaign
                .as_ref()
                .and_then(|c| c.automation.as_ref())
                .and_then(|a| a.repeat_completions.get(kind))
                .copied()
                .unwrap_or(0);
            goals.push(Goal {
                label: label(&format!("repeat:{kind}")),
                id: format!("repeat:{kind}"),
                category: "major_content".into(),
                done: n > 0,
                progress: n.min(1),
                required: 1,
                first_week: at.map(|at| at.week),
                first_return: at.map(|at| at.returns),
                hero: None,
            });
        }
        let mut categories = BTreeMap::<String, Count>::new();
        for g in &goals {
            let n = categories
                .entry(g.category.clone())
                .or_insert(Count { done: 0, total: 0 });
            n.total += 1;
            n.done += usize::from(g.done);
        }
        let achieved = goals.iter().filter(|g| g.done).count();
        let required = goals.len();
        let complete_configured = required > 0 && achieved == required;
        Ok(Some(Progress {
            classes: c
                .classes
                .iter()
                .map(|class| {
                    let heroes = self
                        .state
                        .city
                        .heroes
                        .iter()
                        .filter(|h| h.state.alive && h.class == *class)
                        .collect::<Vec<_>>();
                    let ordinary = heroes
                        .iter()
                        .filter(|h| {
                            !s.mercenaries.contains(&h.guid)
                                && !h.quirks.contains_key("shard_hungry")
                        })
                        .collect::<Vec<_>>();
                    let max = ordinary
                        .iter()
                        .map(|h| self.town().level(h.xp))
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .max();
                    Ok((
                        class.clone(),
                        ClassProgress {
                            ordinary_alive: ordinary.len(),
                            ordinary_max_level: max,
                            mercenaries_alive: heroes.len() - ordinary.len(),
                            earned: s.trained.contains_key(class),
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
            profile: PROFILE.into(),
            full_scope: c.full_scope,
            complete_configured,
            complete: c.full_scope && complete_configured,
            categories,
            achieved,
            required,
            week: self.state.clock.week,
            returns: self.state.clock.returns,
            tracked_returns: s.last_return - s.baseline_returns,
            completions: self.state.clock.completions,
            deaths: self.state.clock.total_deaths(),
            native_kills: s.native_kills,
            goals,
        }))
    }
}
