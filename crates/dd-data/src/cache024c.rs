//! Definition-local parsed-source cache. Raw reads remain live; edits invalidate entries.
use std::{
    any::Any,
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};
type Stamp = Vec<(PathBuf, u64, SystemTime)>;
struct Entry {
    stamp: Stamp,
    value: Arc<dyn Any + Send + Sync>,
}
#[derive(Default)]
pub struct Cache(Mutex<BTreeMap<String, Entry>>);
impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceCache")
    }
}
impl crate::overlay::Overlay {
    pub fn cached<T: Any + Send + Sync>(
        &self,
        key: &str,
        files: &[String],
        build: impl FnOnce() -> crate::combat::Result<T>,
    ) -> crate::combat::Result<Arc<T>> {
        let key = format!(
            "{}:{key}:{:?}:{:?}",
            std::any::type_name::<T>(),
            self.config,
            self.layers
        );
        let stamp = {
            let _span = crate::telemetry024c::scope("cache_validation");
            files
                .iter()
                .flat_map(|p| {
                    self.sources(p)
                        .into_iter()
                        .map(move |i| self.layers[i].root.join(p))
                })
                .map(|p| {
                    let m = std::fs::metadata(&p)?;
                    Ok((p, m.len(), m.modified()?))
                })
                .collect::<std::io::Result<Stamp>>()
        };
        let Ok(stamp) = stamp else {
            return build().map(Arc::new);
        };
        {
            let cache = self
                .cache024c
                .0
                .lock()
                .map_err(|_| "parsed-source cache poisoned")?;
            if let Some(entry) = cache.get(&key).filter(|e| e.stamp == stamp) {
                return entry
                    .value
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| "parsed-source cache type mismatch".into());
            }
        }
        let value = Arc::new(build()?);
        self.cache024c
            .0
            .lock()
            .map_err(|_| "parsed-source cache poisoned")?
            .insert(
                key,
                Entry {
                    stamp,
                    value: value.clone(),
                },
            );
        Ok(value)
    }
}

