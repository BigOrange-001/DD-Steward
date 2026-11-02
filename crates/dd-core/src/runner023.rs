//! Mechanism baseline, with a separate visible-state deadlock safeguard.
use super::*;
use arena021::{Runtime021, Strategy};
use dd_data::mechanisms023::Candidates;
use replay022::Replay;
pub const PROFILE: &str = "candidate-auto-023-v1";
const UNCHANGED_ENEMY_ROUNDS: u32 = 32;

pub(crate) fn policy_view(view: &View) -> Option<View> {
    let changed = match view {
        View::Opening(o) => o.actions.iter().any(equipment_action),
        View::Campaign(o) => o
            .expedition
            .as_ref()
            .is_some_and(|e| e.actions.iter().any(equipment_action)),
    };
    if !changed {
        return None;
    }
    let mut copy = view.clone();
    match &mut copy {
        View::Opening(o) => o.actions.retain(|a| !equipment_action(a)),
        View::Campaign(o) => {
            if let Some(e) = &mut o.expedition {
                e.actions.retain(|a| !equipment_action(a));
            }
        }
    }
    Some(copy)
}
fn equipment_action(a: &crate::opening015::Action) -> bool {
    matches!(
        a,
        crate::opening015::Action::EquipTrinket { .. }
            | crate::opening015::Action::UnequipTrinket { .. }
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    signature: Option<Value>,
    since: u32,
    round: u32,
    abandon_after_retreat: bool,
    pub triggers: u64,
    pub retreat_attempts: u64,
    pub farm_exits: u64,
    pub raid_abandons: u64,
}
impl Guard {
    pub(crate) fn action(&mut self, view: &View) -> Result<Option<Action>> {
        use crate::campaign017::content019::farm::Action as Farm;
        use crate::opening015::Action as Raid;
        let (battle, farm, raid) = match view {
            View::Opening(o) => (o.battle.as_ref(), None, Some(o.as_ref())),
            View::Campaign(o) => {
                let farm = o.content019.as_ref().and_then(|c| c.farm.as_ref());
                (
                    o.expedition
                        .as_ref()
                        .and_then(|e| e.battle.as_ref())
                        .or_else(|| farm.map(|f| &f.battle)),
                    farm,
                    o.expedition.as_ref(),
                )
            }
        };
        let battle = battle.filter(|b| b.round > 0 && b.outcome.is_none());
        let Some(b) = battle else {
            self.signature = None;
            if self.abandon_after_retreat {
                if let Some(r) = raid
                    && r.actions.contains(&Raid::Leave { complete: false })
                {
                    self.abandon_after_retreat = false;
                    self.raid_abandons += 1;
                    return Ok(Some(Action::Raid(Raid::Leave { complete: false })));
                }
                if raid.is_none() {
                    self.abandon_after_retreat = false;
                }
            }
            return Ok(None);
        };
        let enemies = json!(
            b.units
                .iter()
                .filter(|u| !u.hero && !u.corpse)
                .map(|u| (u.id, &u.class, u.hp))
                .collect::<Vec<_>>()
        );
        if self.signature.as_ref() != Some(&enemies) || b.round < self.round {
            self.signature = Some(enemies);
            self.since = b.round;
        }
        self.round = b.round;
        if b.round.saturating_sub(self.since) < UNCHANGED_ENEMY_ROUNDS
            && !self.abandon_after_retreat
        {
            return Ok(None);
        }
        if !self.abandon_after_retreat {
            self.triggers += 1;
        }
        if let Some(f) = farm
            && f.actions.contains(&Farm::Exit)
        {
            self.farm_exits += 1;
            self.signature = None;
            return Ok(Some(Action::Farm(Farm::Exit)));
        }
        if let Some(r) = raid
            && r.actions.contains(&Raid::Fight(crate::Action::Retreat))
        {
            self.abandon_after_retreat = true;
            self.retreat_attempts += 1;
            return Ok(Some(Action::Raid(Raid::Fight(crate::Action::Retreat))));
        }
        Err("023 deadlock safeguard reached an encounter without a legal escape; rules and outcome were not bypassed".into())
    }
}

#[derive(Clone)]
pub struct Runtime023 {
    pub replay: Replay,
    pub candidates: Candidates,
    pub guard_enabled: bool,
    pub guard: Guard,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    profile: String,
    candidates: Candidates,
    guard_enabled: bool,
    guard: Guard,
    replay: String,
}
pub fn definition(config: &Config, candidates: Candidates) -> Result<Definition> {
    let mut c = dd_data::content::ContentConfig::full().with_campaign018_candidates()?;
    c.mode = config.mode.clone();
    c.never_again = config.never_again.clone();
    let c = candidates.apply(c)?;
    Definition::load(
        GameData::load_with_content(&dd_data::game_dir(), c)?,
        CampaignCandidates::documented()?,
    )?
    .with_automation018()?
    .with_content019()?
    .with_steward020()
}
impl Runtime023 {
    pub fn new(
        config: Config,
        strategy: Strategy,
        candidates: Candidates,
        guard_enabled: bool,
    ) -> Result<Self> {
        let d = definition(&config, candidates)?;
        let rules = if strategy == Strategy::Rules021 {
            Some(Runtime021::rules_for(&d)?)
        } else {
            None
        };
        Ok(Self {
            replay: Replay::new(Runtime021::with_definition(config, d, strategy, rules)?),
            candidates,
            guard_enabled,
            guard: Guard::default(),
        })
    }
    pub fn step(&mut self) -> Result<Option<Value>> {
        if self.replay.inner.inner.inner.status != Status::Running {
            return Ok(None);
        }
        let mut next = {
            let _p = dd_data::telemetry024c::scope("clone_runner023");
            self.clone()
        };
        let forced = if next.guard_enabled {
            next.guard
                .action(&next.replay.inner.inner.inner.observe()?)?
        } else {
            None
        };
        let guard_action = forced.is_some();
        let log = next.replay.step_using023(|r| match forced {
            Some(action) => force(r, action),
            None => r.step(),
        })?;
        let log=log.map(|mut log|{
            log["mechanism023"]=json!({"profile":PROFILE,"candidates":next.candidates,"guard_enabled":next.guard_enabled,"guard_action":guard_action,"guard":next.guard});log
        });
        *self = next;
        Ok(log)
    }
    pub(super) fn step_using025(
        &mut self,
        policy: impl FnOnce(&mut arena021::Runtime021) -> Result<Option<Record>>,
    ) -> Result<Option<Value>> {
        if self.replay.inner.inner.inner.status != Status::Running {
            return Ok(None);
        }
        let mut next = {
            let _p = dd_data::telemetry024c::scope("clone_runner023");
            self.clone()
        };
        let forced = if next.guard_enabled {
            next.guard
                .action(&next.replay.inner.inner.inner.observe()?)?
        } else {
            None
        };
        let guard_action = forced.is_some();
        let log = next.replay.step_using023(|r| match forced {
            Some(action) => force(r, action),
            None => policy(r),
        })?;
        let log=log.map(|mut log|{
            log["mechanism023"]=json!({"profile":PROFILE,"candidates":next.candidates,"guard_enabled":next.guard_enabled,"guard_action":guard_action,"guard":next.guard});log
        });
        *self = next;
        Ok(log)
    }
    pub fn checkpoint(&self) -> Result<String> {
        serde_json::to_string(&Save {
            profile: PROFILE.into(),
            candidates: self.candidates,
            guard_enabled: self.guard_enabled,
            guard: self.guard.clone(),
            replay: self.replay.checkpoint()?,
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str) -> Result<Self> {
        if text.len() > 200 * 1024 * 1024 {
            return Err("023 snapshot exceeds bound".into());
        }
        let s: Save = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.profile != PROFILE {
            return Err("023 profile mismatch".into());
        }
        let replay = Replay::restore_config(&s.replay)?;
        let config = &replay.inner.inner.inner.definition.data.content;
        let expected = s.candidates.apply(config.clone())?;
        if &expected != config {
            return Err("023 switches differ from bound content".into());
        }
        if s.guard.since > s.guard.round
            || s.guard.raid_abandons > s.guard.triggers
            || s.guard.farm_exits > s.guard.triggers
        {
            return Err("invalid 023 safeguard counters".into());
        }
        Ok(Self {
            replay,
            candidates: s.candidates,
            guard_enabled: s.guard_enabled,
            guard: s.guard,
        })
    }
}

/// Execute the same legal outer actions and normal host return path. This does
/// not award a win, change combat numbers, or advance a week artificially.
fn force(r: &mut Runtime021, action: Action) -> Result<Option<Record>> {
    let mut next = r.clone();
    let runtime = &mut next.inner.inner;
    let before = runtime.observe()?;
    runtime.terminal(&before)?;
    if runtime.status != Status::Running {
        return Ok(None);
    }
    let before_summary = summary(&before)?;
    match (&mut runtime.actor, &action) {
        (Actor::Campaign(s), Action::Raid(a)) => s.expedition_action(a.clone())?,
        (Actor::Campaign(s), Action::Farm(a)) => s.farm019_action(a.clone())?,
        _ => return Err("023 safeguard is restricted to legal campaign escape actions".into()),
    }
    runtime.decisions = runtime
        .decisions
        .checked_add(1)
        .ok_or("decision overflow")?;
    runtime.cached = None;
    let after = runtime.observe()?;
    runtime.terminal(&after)?;
    let mut record = Record {
        step: runtime.decisions,
        action: serde_json::to_value(action).map_err(|e| e.to_string())?,
        reason: "023:deadlock safeguard; unchanged visible enemy HP for 32 rounds; legal escape"
            .into(),
        before: before_summary,
        after: summary(&after)?,
        status: runtime.status.clone(),
    };
    next.inner.enrich(&mut record.before)?;
    next.inner.enrich(&mut record.after)?;
    *r = next;
    Ok(Some(record))
}

