//! Read-only action audit. Counters never influence policy choices.
use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub torches_bought: u64,
    pub torches_used: u64,
    pub reorders: u64,
    pub field_items_used: BTreeMap<String, u64>,
}
impl Usage {
    pub fn record(&mut self, before: &crate::campaign018::runner::View, log: &Value) {
        let action = &log["action"];
        for branch in ["Provision", "Opening"] {
            let buy = &action[branch]["Buy"];
            if buy["item"]["kind"] == "supply" && buy["item"]["id"] == "torch" {
                self.torches_bought += buy["amount"].as_u64().unwrap_or(0);
            }
        }
        let raid = match before {
            crate::campaign018::runner::View::Opening(o) => Some(o.as_ref()),
            crate::campaign018::runner::View::Campaign(o) => o.expedition.as_ref(),
        };
        for branch in ["Raid", "Opening"] {
            if action[branch].get("Reorder").is_some() {
                self.reorders += 1;
            }
            if let (Some(o), Some(slot)) = (raid, action[branch]["Use"]["slot"].as_u64())
                && let Some(Some(stack)) = o.inventory.slots.get(slot as usize)
            {
                *self.field_items_used.entry(stack.item.key()).or_default() += 1;
                if stack.item.kind == "supply" && stack.item.id == "torch" {
                    self.torches_used += 1;
                }
            }
        }
    }
    pub fn warnings(&self) -> Vec<String> {
        let mut out = vec![];
        if self.torches_bought > 0 && self.torches_used == 0 {
            out.push("bought_torches_but_never_used".into());
        }
        if self.reorders == 0 {
            out.push("no_dungeon_reorder_observed".into());
        }
        out
    }
}
