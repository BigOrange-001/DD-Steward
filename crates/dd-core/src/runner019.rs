//! Explicit new runner profile. Legacy 018 saves remain strict and unchanged.
use super::*;
pub const PROFILE019: &str = "candidate-auto-019-v1";
#[derive(Clone)]
pub struct Runtime019 {
    pub inner: Runtime,
    pub navigation: crate::navigation019::Memory,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save019 {
    profile: String,
    navigation: crate::navigation019::Memory,
    inner: String,
}
impl Runtime019 {
    pub fn new(config: Config) -> Result<Self> {
        let definition = config.definition()?.with_content019()?;
        Ok(Self {
            inner: Runtime::with_definition(config, definition)?,
            navigation: Default::default(),
        })
    }
    pub fn restore_config(text: &str) -> Result<Self> {
        let save: Save019 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let inner: Save = serde_json::from_str(&save.inner).map_err(|e| e.to_string())?;
        let definition = inner.config.definition()?.with_content019()?;
        Self::restore(text, definition)
    }
    pub fn step(&mut self) -> Result<Option<Record>> {
        let mut next = self.clone();
        let record = next
            .inner
            .step_inner_with_navigation(Some(&mut next.navigation))?;
        *self = next;
        Ok(record)
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.navigation.validate()?;
        serde_json::to_string(&Save019 {
            profile: PROFILE019.into(),
            navigation: self.navigation.clone(),
            inner: self.inner.checkpoint()?,
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str, definition: Definition) -> Result<Self> {
        if text.len() > 128 * 1024 * 1024 {
            return Err("019 runner snapshot exceeds bound".into());
        }
        let save: Save019 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if save.profile != PROFILE019 {
            return Err("019 runner profile mismatch".into());
        }
        save.navigation.validate()?;
        Ok(Self {
            inner: Runtime::restore(&save.inner, definition)?,
            navigation: save.navigation,
        })
    }
}
