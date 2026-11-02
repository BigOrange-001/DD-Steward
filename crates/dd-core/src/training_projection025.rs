//! Read-only025 price projection. Never executes upgrades or exposes host state.
use super::*;
use crate::strategy025::training::{Catalog, Node};
impl Session {
    pub(crate) fn training_quotes025(&self) -> Result<Catalog> {
        let mut result = Catalog::default();
        for tree in self.definition.data.trees.keys() {
            let t = self.definition.data.tree(tree)?;
            let instanced = t["is_instanced"] == true;
            let owners = if instanced {
                self.state
                    .heroes
                    .iter()
                    .filter(|h| tree.starts_with(&format!("{}.", h.class)))
                    .map(|h| Some(h.guid))
                    .collect::<Vec<_>>()
            } else {
                vec![None]
            };
            for owner in owners {
                for node in t["requirements"]
                    .as_array()
                    .ok_or("025 missing training requirements")?
                {
                    let code = node["code"].as_str().ok_or("025 missing training code")?;
                    if self.purchased(owner, tree, code) {
                        continue;
                    }
                    let mut quoted = self.clone();
                    let mut requires = vec![];
                    for req in node["prerequisite_requirements"]
                        .as_array()
                        .ok_or("025 missing training dependencies")?
                    {
                        let dep = req["tree_id"]
                            .as_str()
                            .ok_or("025 missing dependency tree")?;
                        let code = req["requirement_code"]
                            .as_str()
                            .ok_or("025 missing dependency code")?;
                        let id = if self.definition.data.tree(dep)?["is_instanced"] == true {
                            owner
                        } else {
                            None
                        };
                        let key = (id, dep.to_string(), code.to_string());
                        quoted.state.purchases.insert(key.clone());
                        requires.push(key);
                    }
                    // Only direct prerequisites are assumed for quoting this node.
                    // Actual discounts, availability and resolve requirements remain current.
                    let (cost, unavailable) = match quoted.upgrade_quote(owner, tree, code) {
                        Ok(cost) => (cost, None),
                        Err(e)
                            if [
                                "hero unavailable for training",
                                "resolve level insufficient for training",
                                "facility locked",
                            ]
                            .contains(&e.as_str()) =>
                        {
                            (BTreeMap::new(), Some(e))
                        }
                        Err(e) => return Err(e),
                    };
                    result.nodes.insert(
                        (owner, tree.clone(), code.into()),
                        Node {
                            cost,
                            requires,
                            unavailable,
                        },
                    );
                }
            }
        }
        Ok(result)
    }
}

