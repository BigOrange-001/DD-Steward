//! Current legal activity ranges; no treatment roll, side effect or future state.
use super::*;
impl Session {
    pub(crate) fn recovery_ranges025(
        &self,
        o: &Observation,
    ) -> Result<crate::strategy025::recovery::Ranges> {
        let mut ranges = BTreeMap::new();
        for c in &o.actions {
            let Action::Queue {
                hero,
                facility,
                treatment: Treatment::Stress,
            } = &c.action
            else {
                continue;
            };
            let h = self.hero(*hero)?;
            let a = self.definition.data.activity(facility)?;
            let row = self.tier(&a["stress_upgrades"], facility)?;
            let [lo, hi] = self
                .class_activity018(&h.class)
                .and_then(|r| r.stress)
                .unwrap_or([uint(row, "heal_low")?, uint(row, "heal_high")?]);
            if lo > hi {
                return Err("invalid stress treatment range".into());
            }
            let activity = facility.split_once('.').ok_or("025 invalid activity")?.1;
            let multiplier = (1.
                + self.modifier(h, "stress_heal_received_percent", "", Some(activity))?)
            .max(0.);
            ranges.insert(
                (*hero, facility.clone()),
                [
                    (f64::from(lo) * multiplier).round() as i32,
                    (f64::from(hi) * multiplier).round() as i32,
                ],
            );
        }
        Ok(ranges)
    }
}

