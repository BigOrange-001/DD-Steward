//! Source event quest windows and once-only ignore consequences.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct PlotWindow {
    pub event: String,
    pub opened: u32,
    pub expires: Option<u32>,
}
impl Session {
    pub(super) fn open_event_plot(&mut self, id: &str, event: &str) -> Result<()> {
        let q = library::unique_json(&self.definition.data.quests, id, "event plot")?;
        if q["is_started_by_event"] != true && q["is_generated_by_event"] != true {
            return Err("event plot has no source activation flag".into());
        }
        if self.state.plot_windows.contains_key(id) {
            return Err("event plot already has a live instance".into());
        }
        let expires = if q["is_generated_by_event"] == true {
            Some(
                self.state
                    .clock
                    .week
                    .checked_add(1)
                    .ok_or("event plot deadline overflow")?,
            )
        } else {
            None
        };
        self.state.started_plots.insert(id.into());
        self.state.plot_windows.insert(
            id.into(),
            PlotWindow {
                event: event.into(),
                opened: self.state.clock.week,
                expires,
            },
        );
        Ok(())
    }
    pub(super) fn validate_plot_windows(&self) -> Result<()> {
        for id in &self.state.started_plots {
            library::unique_json(&self.definition.data.quests, id, "started event plot")?;
        }
        for (id, w) in &self.state.plot_windows {
            let q = library::unique_json(&self.definition.data.quests, id, "event plot window")?;
            let e = self.definition.data.event(&w.event)?;
            if !self.state.started_plots.contains(id)
                || w.opened > self.state.clock.week
                || !e["data"].as_array().is_some_and(|a| {
                    a.iter()
                        .any(|v| v["type"] == "plot_quest" && v["string_data"] == *id)
                })
                || q["is_started_by_event"] != true && q["is_generated_by_event"] != true
                || q["is_repeatable"] != true && self.state.city.completed_ids.contains(id)
            {
                return Err("invalid source event plot window".into());
            }
            let expected = if q["is_generated_by_event"] == true {
                Some(w.opened.checked_add(1).ok_or("deadline overflow")?)
            } else {
                None
            };
            if w.expires != expected || w.expires.is_some_and(|v| v <= self.state.clock.week) {
                return Err("stale or modified event plot deadline".into());
            }
        }
        Ok(())
    }
    pub(super) fn close_event_plots(
        &mut self,
        town: &mut TownSession,
        attempted: Option<&str>,
        completed: bool,
        advance: bool,
    ) -> Result<()> {
        let due_week = self
            .state
            .clock
            .week
            .checked_add(u32::from(advance))
            .ok_or("event deadline week overflow")?;
        let due = self
            .state
            .plot_windows
            .iter()
            .filter(|(id, w)| {
                completed && attempted == Some(id.as_str())
                    || advance && w.expires.is_some_and(|v| v <= due_week)
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in due {
            let ignored = attempted != Some(id.as_str());
            if ignored {
                let q =
                    library::unique_json(&self.definition.data.quests, &id, "ignored event plot")?
                        .clone();
                for loss in dd_data::campaign017::array(&q, "upgrade_tags_to_remove_on_ignore")? {
                    dd_data::campaign017::fields(
                        loss,
                        &["upgrade_tag", "amount"],
                        "ignored plot upgrade loss",
                    )?;
                    let tag = loss["upgrade_tag"]
                        .as_str()
                        .ok_or("missing upgrade loss tag")?;
                    town.remove_event_upgrades(tag, dd_data::campaign017::uint(loss, "amount")?)?;
                }
            }
            self.state.plot_windows.remove(&id);
            self.state.log.push(json!({"event_plot_closed":id,"week":due_week,"ignored":ignored,"completed":completed&&attempted==Some(id.as_str())}));
        }
        Ok(())
    }
}
impl TownSession {
    fn remove_event_upgrades(&mut self, tag: &str, amount: u32) -> Result<()> {
        if amount > 10000 {
            return Err("source upgrade loss count bound".into());
        }
        let mut known = false;
        for id in self.definition.data.trees.keys() {
            let raw = self.definition.data.tree(id)?;
            if dd_data::campaign017::array(raw, "tags")?
                .iter()
                .any(|v| v == tag)
            {
                known = true;
            }
        }
        if !known {
            return Err("unknown source upgrade loss tag".into());
        }
        for _ in 0..amount {
            let mut eligible = vec![];
            for (owner, tree, code) in &self.state.purchases {
                if owner.is_some() {
                    continue;
                }
                let raw = self.definition.data.tree(tree)?;
                if !raw["tags"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v == tag))
                {
                    continue;
                }
                let mut used = false;
                for (other_owner, t, c) in &self.state.purchases {
                    if other_owner.is_some() || t == tree && c == code {
                        continue;
                    }
                    let node = self.definition.data.node(t, c)?;
                    if dd_data::campaign017::array(node, "prerequisite_requirements")?
                        .iter()
                        .any(|r| r["tree_id"] == *tree && r["requirement_code"] == *code)
                    {
                        used = true;
                    }
                }
                if !used {
                    eligible.push((None, tree.clone(), code.clone()));
                }
            }
            if eligible.is_empty() {
                break;
            }
            let chosen = self.state.rng.below(eligible.len() as u64) as usize;
            let lost = eligible.remove(chosen);
            self.state.purchases.remove(&lost);
            self.state
                .log
                .push(json!({"ignored_plot_upgrade_loss":lost,"source_tag":tag}));
        }
        Ok(())
    }
}
