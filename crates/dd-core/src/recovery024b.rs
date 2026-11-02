//! P024-23: finite, data-ordered preparation recovery, using public inputs only.
use super::Context;
use crate::{
    auto_policy018::Memory,
    campaign017::{Observation, QuestView},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Strict,
    WithoutHealRelief,
    WithoutTraining,
    Legacy,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub id: String,
    pub sources: Vec<String>,
    pub stages: Vec<Stage>,
    pub second_stage_party_size: usize,
    pub note: String,
}
impl Settings {
    pub fn validate(&self) -> bool {
        self.id == "P024-23"
            && !self.sources.is_empty()
            && self.second_stage_party_size == 4
            && self.stages
                == [
                    Stage::Strict,
                    Stage::WithoutHealRelief,
                    Stage::WithoutTraining,
                    Stage::Legacy,
                ]
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counts {
    pub attempts: [u64; 4],
    pub selected: [u64; 4],
    pub exhausted: u64,
}
pub fn choose(
    o: &Observation,
    m: &Memory,
    ctx: &Context,
    counts: &mut Counts,
) -> Option<(QuestView, Vec<u64>, usize)> {
    for (index, stage) in ctx.profile.recovery.stages.iter().enumerate() {
        counts.attempts[index] += 1;
        let found = match stage {
            Stage::Strict => super::policy::choose_quest(o, m, ctx),
            Stage::WithoutHealRelief => {
                let mut relaxed = ctx.clone();
                relaxed.profile.composition.medium_min_heal = 0.;
                relaxed.profile.composition.medium_min_relief = 0.;
                // Keep the existing task ranking and C gates. This layer does
                // not downgrade a quest or relax control, source legality or tags.
                super::policy::choose_quest(o, m, &relaxed)
                    .filter(|(_, p)| p.len() == ctx.profile.recovery.second_stage_party_size)
            }
            Stage::WithoutTraining => {
                let mut relaxed = ctx.clone();
                relaxed.enabled.remove("C");
                relaxed.profile.composition.medium_min_heal = 0.;
                relaxed.profile.composition.medium_min_relief = 0.;
                o.board
                    .as_ref()
                    .into_iter()
                    .flat_map(|b| &b.quests)
                    .filter(|q| {
                        q.execution != "expedition integration pending"
                            && !m.blocked.contains(q.plot.as_ref().unwrap_or(&q.id))
                            && q.estate_requirements
                                .iter()
                                .all(|(id, n)| o.town.wallet.get(id).copied().unwrap_or(0) >= *n)
                    })
                    .filter_map(|q| {
                        let p = super::policy::party(o, q, true, &relaxed);
                        (!p.is_empty()).then_some((q.clone(), p))
                    })
                    .min_by_key(|(q, _)| (q.difficulty, ctx.length_priority(q), q.id.clone()))
            }
            Stage::Legacy => crate::auto_policy018::recovery_selection024b(o, m),
        };
        if let Some((q, p)) = found {
            counts.selected[index] += 1;
            return Some((q, p, index));
        }
    }
    counts.exhausted += 1;
    None
}
