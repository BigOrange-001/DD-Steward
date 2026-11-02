//! Opt-in host profiling; never serialized or exposed to policies.
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::Path,
    sync::{Mutex, OnceLock},
    time::Instant,
};
#[derive(Default, Serialize)]
struct Metric {
    calls: u64,
    bytes: u64,
    nanos: u128,
}
#[derive(Default, Serialize)]
struct Metrics {
    phases: BTreeMap<String, Metric>,
    files: BTreeMap<String, Metric>,
}
static ENABLED: OnceLock<bool> = OnceLock::new();
static METRICS: OnceLock<Mutex<Metrics>> = OnceLock::new();
thread_local! { static STACK: RefCell<Vec<u128>> = const { RefCell::new(Vec::new()) }; }
pub struct Span {
    name: &'static str,
    start: Option<Instant>,
}
pub fn scope(name: &'static str) -> Span {
    let enabled = *ENABLED.get_or_init(|| std::env::var_os("DD_PROFILE_OUT").is_some());
    if enabled {
        STACK.with_borrow_mut(|s| s.push(0));
    }
    Span {
        name,
        start: enabled.then(Instant::now),
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let elapsed = start.elapsed().as_nanos();
            let children = STACK.with_borrow_mut(|s| {
                let c = s.pop().unwrap();
                if let Some(p) = s.last_mut() {
                    *p += elapsed;
                }
                c
            });
            let mut m = METRICS.get_or_init(Default::default).lock().unwrap();
            let v = m.phases.entry(self.name.into()).or_default();
            v.calls += 1;
            v.nanos += elapsed.saturating_sub(children);
        }
    }
}
pub fn read(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let span = scope("file_read");
    let bytes = std::fs::read(path.as_ref())?;
    if span.start.is_some() {
        let mut m = METRICS.get_or_init(Default::default).lock().unwrap();
        let v = m
            .files
            .entry(path.as_ref().display().to_string())
            .or_default();
        v.calls += 1;
        v.bytes += bytes.len() as u64;
    }
    Ok(bytes)
}
pub fn flush() {
    if let Some(path) = std::env::var_os("DD_PROFILE_OUT") {
        let m = METRICS.get_or_init(Default::default).lock().unwrap();
        if let Ok(bytes) = serde_json::to_vec_pretty(&*m) {
            let _ = std::fs::write(path, bytes);
        }
    }
}
