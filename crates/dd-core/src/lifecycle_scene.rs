use super::*;
use dd_data::combat::GameData;

pub type EncounterInput = (Scenario, BTreeMap<String, Blueprint>, Vec<Vec<Blueprint>>);
pub fn lifecycle_scenario(g: &GameData, scene: &str) -> Result<EncounterInput> {
    let heroes = vec![
        g.hero(
            "highwayman",
            4,
            4,
            &["wicked_slice", "pistol_shot", "grape_shot_blast"],
        )?,
        g.hero("man_at_arms", 4, 4, &["retribution", "defender"])?,
        g.hero(
            "plague_doctor",
            4,
            4,
            &["noxious_blast", "plague_grenade", "battlefield_medicine"],
        )?,
        g.hero("vestal", 4, 4, &["divine_grace", "gods_comfort"])?,
    ];
    let first = match scene {
        "swarm" | "waves" => vec!["ectoplasm_large_A", "ectoplasm_A"],
        "multiple" => vec!["templar_melee_mb_D", "brigand_fusilier_B", "corpse_A"],
        "transform" => vec!["ectoplasm_A"],
        "rally" => vec!["corpse_A", "skeleton_courtier_A"],
        _ => {
            return Err(
                "unknown lifecycle scene; expected swarm|waves|multiple|rally|transform".into(),
            );
        }
    };
    let next = if scene == "waves" {
        vec![vec!["templar_melee_mb_D", "brigand_fusilier_B", "corpse_A"]]
    } else {
        vec![]
    };
    let mut roots = first
        .iter()
        .copied()
        .chain(next.iter().flatten().copied())
        .collect::<Vec<_>>();
    if scene == "transform" {
        roots.push("farmer_A");
    }
    if scene == "rally" {
        roots.extend(["skeleton_common_A", "skeleton_militia_A"]);
    }
    let catalog = g.entity_catalog(&roots)?;
    let monsters = first.iter().map(|id| catalog[*id].clone()).collect();
    let waves = next
        .iter()
        .map(|wave| wave.iter().map(|id| catalog[*id].clone()).collect())
        .collect();
    Ok((
        Scenario {
            heroes,
            monsters,
            rules: g.rules.clone(),
            round_limit: 100,
        },
        catalog,
        waves,
    ))
}
/// Waiting briefly makes the original summoning AI observable, then attacks.
pub struct LifecycleBaseline;
impl Policy for LifecycleBaseline {
    fn choose(&mut self, o: &Observation) -> Option<Action> {
        if o.round <= 2 {
            o.actions
                .iter()
                .find(|a| a.action == Action::Pass)
                .map(|a| a.action.clone())
        } else {
            Baseline.choose(o)
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LifecycleCounts {
    pub spawned: u64,
    pub changed: u64,
    pub deaths: u64,
    pub waves: u64,
    pub bonus: u64,
    pub repeated_opportunities: u64,
    pub zero_action_states: u64,
    pub initial_spawn_stealth: u64,
    seen: BTreeMap<(u32, UnitId), u32>,
}
impl LifecycleCounts {
    pub fn initial(&mut self, o: &Observation) {
        self.seen.clear();
        self.zero_action_states += o.units.iter().filter(|u| u.corpse).count() as u64;
        self.initial_spawn_stealth +=
            o.units.iter().filter(|u| !u.hero && u.stealthed).count() as u64;
    }
    pub fn record(&mut self, o: &Observation, events: &[Event]) {
        self.zero_action_states += o.units.iter().filter(|u| u.corpse).count() as u64;
        if let Some(id) = o.actor
            && o.units.iter().any(|u| u.id == id && !u.hero)
        {
            let count = self.seen.entry((o.round, id)).or_default();
            self.repeated_opportunities += u64::from(*count > 0);
            *count += 1;
        }
        self.phase(events);
        self.seen.retain(|(round, _), _| *round >= o.round);
    }
    pub fn phase(&mut self, events: &[Event]) {
        for e in events {
            match e {
                Event::Spawned { .. } => self.spawned += 1,
                Event::Changed { .. } => self.changed += 1,
                Event::Death(_) | Event::Corpse(_) => self.deaths += 1,
                Event::Wave(_) => self.waves += 1,
                Event::BonusAction { .. } => self.bonus += 1,
                _ => {}
            }
        }
    }
}
