//! Host-only, thread-local execution scope. It changes no source identity,
//! observation, ordering key or saved battle/RNG representation.
use dd_data::combat::Result;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fallback {
    pub monster: String,
    pub actor: usize,
    pub round: u32,
    pub turn: u32,
    pub skill: String,
    pub target: usize,
    pub target_class: String,
}
#[derive(Default)]
struct Scope {
    enabled: bool,
    records: Vec<Fallback>,
}
thread_local! {
    static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        SCOPE.with(|s| s.borrow_mut().take());
    }
}
/// Called only by a host runner. Failed outer transactions discard audit events.
/// RAII also clears the thread-local setting on an unwind; scopes cannot nest.
pub fn run<T>(enabled: bool, action: impl FnOnce() -> Result<T>) -> Result<(T, Vec<Fallback>)> {
    SCOPE.with(|s| {
        let mut s = s.borrow_mut();
        if s.is_some() {
            return Err("nested 024b host execution scope".to_string());
        }
        *s = Some(Scope {
            enabled,
            ..Default::default()
        });
        Ok(())
    })?;
    let _reset = Reset;
    let result = action();
    let records = SCOPE.with(|s| s.borrow_mut().take().unwrap().records);
    result.map(|v| (v, records))
}
pub(crate) fn enabled() -> bool {
    SCOPE.with(|s| s.borrow().as_ref().is_some_and(|s| s.enabled))
}
pub(crate) fn record(event: Fallback) {
    SCOPE.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            s.records.push(event);
        }
    });
}
/// Isolated026 surrogate scope; retain the outer real-action audit unchanged.
pub(crate) fn hypothetical026<T>(enabled: bool, action: impl FnOnce() -> Result<T>) -> Result<T> {
    struct Restore(Option<Scope>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SCOPE.with(|s| *s.borrow_mut() = self.0.take());
        }
    }
    let old = SCOPE.with(|s| {
        s.replace(Some(Scope {
            enabled,
            records: vec![],
        }))
    });
    let _restore = Restore(old);
    action()
}

