//! Host-only025 candidate switches. No policy can inspect this execution scope.
use dd_data::combat::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Options {
    pub zero_final_weights: bool,
    pub csv_corridor_retreat: bool,
    #[serde(default)]
    pub disease_quirk_conflicts: bool,
    #[serde(default)]
    pub shrieker_contract: bool,
}
impl Options {
    pub fn candidate(enabled: bool) -> Self {
        Self {
            zero_final_weights: enabled,
            csv_corridor_retreat: enabled,
            disease_quirk_conflicts: enabled,
            shrieker_contract: enabled,
        }
    }
}
#[derive(Default)]
struct Scope {
    options: Options,
    events: Vec<Value>,
}
thread_local! {
    static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
    static CONTEXT: RefCell<Vec<Options>> = const { RefCell::new(Vec::new()) };
}
struct ContextReset;
impl Drop for ContextReset {
    fn drop(&mut self) {
        CONTEXT.with(|s| {
            s.borrow_mut().pop();
        });
    }
}
/// Definitions reconstructed for observation/restore must see the same host
/// switches as actions. This ambient context never records hypothetical events.
pub fn with_options<T>(options: Options, action: impl FnOnce() -> Result<T>) -> Result<T> {
    CONTEXT.with(|s| s.borrow_mut().push(options));
    let _reset = ContextReset;
    action()
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        SCOPE.with(|s| s.borrow_mut().take());
    }
}
pub fn run<T>(options: Options, action: impl FnOnce() -> Result<T>) -> Result<(T, Vec<Value>)> {
    SCOPE.with(|s| {
        if s.borrow().is_some() {
            return Err("nested025 execution scope".to_string());
        }
        *s.borrow_mut() = Some(Scope {
            options,
            events: vec![],
        });
        Ok(())
    })?;
    let _reset = Reset;
    let value = action();
    let events = SCOPE.with(|s| s.borrow_mut().take().unwrap().events);
    value.map(|v| (v, events))
}
pub(crate) fn options() -> Options {
    SCOPE
        .with(|s| s.borrow().as_ref().map(|s| s.options))
        .unwrap_or_else(|| CONTEXT.with(|s| s.borrow().last().copied().unwrap_or_default()))
}
pub(crate) fn shrieker_escape_source(b: &crate::Battle, id: crate::UnitId) -> bool {
    options().shrieker_contract
        && b.scenario
            .rules
            .encounter_contract
            .as_ref()
            .is_some_and(|c| c.id.starts_with("plot_trinket_retention_"))
        && !b.units[id].blueprint.hero
        && b.units[id]
            .blueprint
            .skills
            .iter()
            .any(|s| s.id == "escape")
}
/// Decline an ordinary quirk gain that conflicts with an existing disease.
/// Selection weights and the selected ID stay unchanged; no redraw or purge.
pub(crate) fn quirk_disease_conflict(
    defs: &std::collections::BTreeMap<String, Value>,
    id: &str,
    diseases: &[String],
) -> Result<bool> {
    if !options().disease_quirk_conflicts {
        return Ok(false);
    }
    let q = dd_data::library::unique_json(defs, id, "025 incoming quirk")?;
    for d in diseases {
        let old = dd_data::library::unique_json(defs, d, "025 existing disease")?;
        if q["incompatible_quirks"]
            .as_array()
            .is_some_and(|v| v.iter().any(|s| s == d))
            || old["incompatible_quirks"]
                .as_array()
                .is_some_and(|v| v.iter().any(|s| s == id))
        {
            record(serde_json::json!({"kind":"quirk_disease_conflict","quirk":id,"disease":d}));
            return Ok(true);
        }
    }
    Ok(false)
}
pub(crate) fn record(event: Value) {
    SCOPE.with(|s| {
        if let Some(s) = &mut *s.borrow_mut() {
            s.events.push(event);
        }
    });
}
/// Isolated026 surrogate scope: simulated fallback events never reach real audits.
pub(crate) fn hypothetical026<T>(
    options: Options,
    action: impl FnOnce() -> Result<T>,
) -> Result<T> {
    struct Restore(Option<Scope>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SCOPE.with(|s| *s.borrow_mut() = self.0.take());
        }
    }
    let old = SCOPE.with(|s| {
        s.replace(Some(Scope {
            options,
            events: vec![],
        }))
    });
    let _restore = Restore(old);
    action()
}


