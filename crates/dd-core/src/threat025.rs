//! Static definitions are projected only for enemy IDs already visible on screen.
use super::*;
use dd_data::combat::{EffectKind, EffectTarget};
#[derive(Debug, Clone, Default)]
pub struct Enemy {
    pub skills: Vec<Skill>,
    pub turns: u8,
    pub stun_resist: f64,
    pub move_resist: f64,
    pub accelerate: bool,
    pub boss: bool,
}
#[derive(Debug, Clone, Default)]
pub struct Skill {
    pub launch: Vec<u8>,
    pub ranks: Vec<u8>,
    pub damage: f64,
    pub stress: f64,
    pub aoe: bool,
    pub support: f64,
    pub complex: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Assessment {
    pub unit: usize,
    pub damage: f64,
    pub stress: f64,
    pub support: f64,
    pub lethal_risk: f64,
    pub dot_lethal: bool,
    pub aoe: bool,
    pub targets: Vec<usize>,
    pub uncertain: bool,
}
fn effect(k: &EffectKind) -> (f64, f64, f64, bool) {
    match k {
        EffectKind::Stress(n) | EffectKind::Horror(n) | EffectKind::ProtectedHorror(n) => {
            (0., f64::from((*n).max(0)), 0., false)
        }
        EffectKind::HealthDamage(n)
        | EffectKind::Bleed(n)
        | EffectKind::Blight(n)
        | EffectKind::Burn(n) => (f64::from((*n).max(0)), 0., 0., false),
        EffectKind::ControlledBurn { amount, .. } => (f64::from((*amount).max(0)), 0., 0., false),
        EffectKind::Composite(es) => es
            .iter()
            .map(|e| effect(&e.kind))
            .fold((0., 0., 0., false), |a, b| {
                (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 || b.3)
            }),
        EffectKind::BattlePersistent(k) | EffectKind::InventoryEffect(k) => effect(k),
        EffectKind::Heal(_)
        | EffectKind::HealPercent(_)
        | EffectKind::Regen(_)
        | EffectKind::Guard { .. }
        | EffectKind::Buffs(_)
        | EffectKind::InstantBuffs(_)
        | EffectKind::Mark
        | EffectKind::Stun
        | EffectKind::Shuffle
        | EffectKind::ShuffleParty
        | EffectKind::Pull(_)
        | EffectKind::Push(_) => (0., 0., 1., false),
        EffectKind::Boss(_) | EffectKind::Lifecycle(_) => (0., 0., 1., true),
        _ => (0., 0., 0., false),
    }
}
pub fn project(game: &GameData, id: &str) -> Result<Enemy> {
    let bp = game.monster(id)?;
    Ok(project_blueprint(&bp))
}
fn project_blueprint(bp: &dd_data::combat::Blueprint) -> Enemy {
    let mut skills = vec![];
    for s in &bp.skills {
        let mut skill = Skill {
            launch: s.launch.clone(),
            ranks: s.target.ranks.clone(),
            damage: if s.target.friendly {
                0.
            } else {
                s.damage.map_or(0., |d| f64::from(d[1].max(0)))
            },
            aoe: s.target.multi,
            ..Default::default()
        };
        for e in &s.effects {
            let (d, stress, support, complex) = effect(&e.kind);
            if !matches!(
                e.target,
                EffectTarget::Performer | EffectTarget::ThisTrinket
            ) {
                skill.damage += d;
                skill.stress += stress;
            }
            skill.support += support;
            skill.complex |= complex;
        }
        skills.push(skill)
    }
    Enemy {
        skills,
        turns: bp.mechanics.lifecycle.turns,
        stun_resist: bp.stats.stun_resist,
        move_resist: bp.stats.move_resist,
        accelerate: bp.mechanics.lifecycle.accelerate_stall || bp.mechanics.lifecycle.disable_stall,
        boss: bp
            .mechanics
            .enemy_types
            .iter()
            .chain(&bp.mechanics.boss.tags)
            .any(|s| s == "boss"),
    }
}
pub(crate) fn controlled_source026(bp: &dd_data::combat::Blueprint) -> Enemy {
    let mut e = project_blueprint(bp);
    for s in &mut e.skills {
        s.complex = true;
    }
    e
}
pub fn key(u: &crate::VisibleUnit) -> String {
    if u.controlled.is_some() {
        format!("controlled:{}:{}", u.id, u.class)
    } else {
        u.class.clone()
    }
}
/// A controlled hero is still an owned, known build. If a class is repeated,
/// use the union of the visible departing heroes' skills, marked uncertain.
/// No battle-to-estate hidden GUID mapping or future action queue is consulted.
pub fn controlled(game: &GameData, o: &Observation, u: &crate::VisibleUnit) -> Result<Enemy> {
    let raid = o
        .expedition
        .as_ref()
        .ok_or("025 controlled hero outside visible raid")?;
    let mut result = Enemy::default();
    let mut found = false;
    for h in o.town.heroes.iter().filter(|h| {
        h.class == u.class
            && raid
                .heroes
                .iter()
                .any(|r| r.guid == h.guid && r.alive && r.class == u.class)
    }) {
        let skills = h
            .selected
            .iter()
            .map(|id| {
                h.skills
                    .get(id)
                    .map(|rank| (id.as_str(), *rank))
                    .ok_or("025 visible selected skill lacks learned rank")
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let bp = game.hero_ranked(&h.class, h.weapon, h.armour, &skills)?;
        let mut e = project_blueprint(&bp);
        for s in &mut e.skills {
            s.complex = true;
        }
        result.skills.extend(e.skills);
        result.turns = result.turns.max(e.turns);
        result.stun_resist = result.stun_resist.max(e.stun_resist);
        result.move_resist = result.move_resist.max(e.move_resist);
        found = true;
    }
    if !found {
        return Err(format!(
            "025 controlled hero lacks known departing build {}",
            u.class
        ));
    }
    Ok(result)
}
pub fn evaluate(
    o: &crate::Observation,
    known: &BTreeMap<String, Enemy>,
) -> Result<Vec<Assessment>> {
    o.units
        .iter()
        .filter(|u| !u.hero && !u.corpse && u.hp > 0)
        .map(|u| {
            let e = known.get(&key(u)).ok_or_else(|| {
                format!(
                    "025 visible enemy lacks analytic source descriptor {}",
                    u.class
                )
            })?;
            let turns = u32::from(e.turns).saturating_sub(u32::from(u.stunned));
            let mut result = Assessment {
                unit: u.id,
                damage: 0.,
                stress: 0.,
                support: 0.,
                lethal_risk: 0.,
                dot_lethal: false,
                aoe: false,
                targets: vec![],
                uncertain: false,
            };
            for s in e
                .skills
                .iter()
                .filter(|s| s.launch.iter().any(|r| u.occupied_ranks.contains(r)))
            {
                let targets = o
                    .units
                    .iter()
                    .filter(|h| {
                        h.hero && !h.corpse && h.occupied_ranks.iter().any(|r| s.ranks.contains(r))
                    })
                    .map(|h| h.id)
                    .collect::<Vec<_>>();
                let damage = s.damage * f64::from(turns);
                result.damage = result.damage.max(damage);
                result.stress = result.stress.max(s.stress * f64::from(turns));
                result.support = result.support.max(s.support * f64::from(turns));
                result.aoe |= s.aoe;
                result.uncertain |= s.complex;
                for id in targets {
                    if !result.targets.contains(&id) {
                        result.targets.push(id)
                    }
                    if let Some(h) = o.units.iter().find(|h| h.id == id) {
                        result.lethal_risk = result.lethal_risk.max(damage / f64::from(h.hp.max(1)))
                    }
                }
            }
            let dot = u
                .bleed
                .iter()
                .chain(&u.blight)
                .filter(|(_, n)| *n > 0)
                .map(|(n, _)| *n)
                .sum::<i32>()
                + u.burn;
            result.dot_lethal = dot >= u.hp;
            if result.dot_lethal {
                result.damage = 0.;
                result.stress = 0.;
                result.support = 0.;
                result.lethal_risk = 0.;
            }
            Ok(result)
        })
        .collect()
}
