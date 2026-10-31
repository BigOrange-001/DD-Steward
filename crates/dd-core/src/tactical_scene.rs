use super::*;
use dd_data::combat::GameData;

pub fn tactical_scenario(g: &GameData, scene: &str) -> Result<Scenario> {
    let (heroes, monsters) = match scene {
        "guard" => (
            vec![
                g.hero("man_at_arms", 1, 1, &["defender", "retribution"])?,
                g.hero(
                    "highwayman",
                    1,
                    1,
                    &["duelist_advance", "wicked_slice", "pistol_shot"],
                )?,
                g.hero("arbalest", 1, 1, &["blindfire"])?,
                g.hero(
                    "plague_doctor",
                    1,
                    1,
                    &["battlefield_medicine", "noxious_blast", "plague_grenade"],
                )?,
            ],
            vec![
                g.monster("skeleton_captain_A")?,
                g.monster("carrion_eater_big_A")?,
            ],
        ),
        "status" => (
            vec![
                g.hero("houndmaster", 1, 1, &["hounds_harry", "hounds_rush"])?,
                g.hero("arbalest", 1, 1, &["blindfire"])?,
                g.hero("antiquarian", 1, 1, &["protect_me", "invigorating_vapours"])?,
                g.hero(
                    "plague_doctor",
                    1,
                    1,
                    &["battlefield_medicine", "noxious_blast", "plague_grenade"],
                )?,
            ],
            vec![
                g.monster("carrion_eater_big_A")?,
                g.monster("spider_spitter_B")?,
                g.monster("fungal_bloat_B")?,
            ],
        ),
        _ => return Err("unknown tactical scene; expected guard|status".into()),
    };
    Ok(Scenario {
        heroes,
        monsters,
        rules: g.rules.clone(),
        round_limit: 100,
    })
}

/// Deliberately simple experiment policy. Its only input is the public observation.
pub struct TacticalBaseline;
impl Policy for TacticalBaseline {
    fn choose(&mut self, o: &Observation) -> Option<Action> {
        let actor = o.units.iter().find(|u| Some(u.id) == o.actor)?;
        let find = |name: &str, target: Option<UnitId>| {
            o.actions
                .iter()
                .find(|a| a.label == name && target.is_none_or(|id| a.targets.contains(&id)))
                .map(|a| a.action.clone())
        };
        let preferred = match actor.class.as_str() {
            "man_at_arms" => {
                let ward = o
                    .units
                    .iter()
                    .find(|u| u.hero && u.class == "highwayman")
                    .map(|u| u.id);
                if actor.guarding.is_none_or(|(_, n)| n <= 1) {
                    find("defender", ward)
                } else if actor.riposte.is_none() {
                    find("retribution", None)
                } else {
                    None
                }
            }
            "highwayman" if actor.riposte.is_none() => find("duelist_advance", None),
            "shieldbreaker" if actor.damage_blocks == 0 => find("serpents_sway", None),
            "antiquarian" if actor.guarded_by.is_none() => {
                let guard = o
                    .units
                    .iter()
                    .find(|u| u.hero && u.class == "houndmaster")
                    .map(|u| u.id);
                find("protect_me", guard)
            }
            "antiquarian" => find("invigorating_vapours", None),
            "houndmaster" if o.units.iter().filter(|u| !u.hero && !u.corpse).count() > 1 => {
                find("hounds_harry", None)
            }
            "plague_doctor" => o
                .units
                .iter()
                .filter(|u| u.hero && (!u.bleed.is_empty() || !u.blight.is_empty()))
                .find_map(|u| find("battlefield_medicine", Some(u.id))),
            _ => None,
        };
        preferred.or_else(|| Baseline.choose(o))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TacticalCounts {
    pub guards: u64,
    pub interceptions: u64,
    pub clears: u64,
    pub counters: u64,
    pub blocks: u64,
    pub random_actions: u64,
    pub cures: u64,
    pub dot_states: u64,
}
impl TacticalCounts {
    pub fn record(
        &mut self,
        before: &Observation,
        action: Option<&Action>,
        events: &[Event],
        after: &Observation,
    ) {
        self.random_actions += u64::from(matches!(action, Some(Action::SkillAuto { .. })));
        self.dot_states += after
            .units
            .iter()
            .filter(|u| !u.bleed.is_empty() || !u.blight.is_empty())
            .count() as u64;
        for e in events {
            match e {
                Event::GuardEstablished { .. } => self.guards += 1,
                Event::GuardIntercept { .. } => self.interceptions += 1,
                Event::GuardCleared { .. } => self.clears += 1,
                Event::CounterAttack { .. } => self.counters += 1,
                Event::DamageBlocked { .. } => self.blocks += 1,
                _ => {}
            }
        }
        if let Some(Action::Skill { target, .. }) = action
            && before
                .actions
                .iter()
                .any(|a| Some(&a.action) == action && a.label == "battlefield_medicine")
            && let Some(old) = before.units.iter().find(|u| u.id == *target)
            && (!old.bleed.is_empty() || !old.blight.is_empty())
            && after
                .units
                .iter()
                .any(|u| u.id == *target && u.bleed.is_empty() && u.blight.is_empty())
        {
            self.cures += 1;
        }
    }
}
