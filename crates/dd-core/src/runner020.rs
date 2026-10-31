//! Explicit Steward runner. Never upgrades legacy snapshots or fabricates history.
use super::*;
pub const PROFILE020: &str = "candidate-auto-020-v1";
#[derive(Clone)]
pub struct Runtime020 {
    pub inner: Runtime,
    pub navigation: crate::navigation019::Memory,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Save020 {
    profile: String,
    content: dd_data::content::ContentConfig,
    navigation: crate::navigation019::Memory,
    inner: String,
}
impl Runtime020 {
    pub fn new(config: Config) -> Result<Self> {
        let d = config.definition()?.with_content019()?.with_steward020()?;
        Self::with_definition(config, d)
    }
    pub fn with_definition(config: Config, definition: Definition) -> Result<Self> {
        if definition.goals020.is_none() {
            return Err("020 requires a Steward definition".into());
        }
        Ok(Self {
            inner: Runtime::with_definition(config, definition)?,
            navigation: Default::default(),
        })
    }
    pub fn step(&mut self) -> Result<Option<Record>> {
        let mut next = self.clone();
        let mut r = next
            .inner
            .step_inner_with_navigation(Some(&mut next.navigation))?;
        if let Some(record) = &mut r {
            next.enrich(&mut record.before)?;
            next.enrich(&mut record.after)?;
        }
        *self = next;
        Ok(r)
    }
    pub fn refresh(&mut self) -> Result<Value> {
        let mut v = self.inner.refresh()?;
        self.enrich(&mut v)?;
        Ok(v)
    }
    fn enrich(&self, v: &mut Value) -> Result<()> {
        if v.get("steward").is_none() {
            let c = self
                .inner
                .definition
                .goals020
                .as_ref()
                .ok_or("020 goal catalog absent")?;
            let p = crate::campaign017::goals020::initial_progress(c);
            v["goals"] = json!(p.goals.iter().map(|g| (&g.id, g.done)).collect::<Vec<_>>());
            v["steward"] = serde_json::to_value(p).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn checkpoint(&self) -> Result<String> {
        self.navigation.validate()?;
        serde_json::to_string(&Save020 {
            profile: PROFILE020.into(),
            content: self.inner.definition.data.content.clone(),
            navigation: self.navigation.clone(),
            inner: self.inner.checkpoint()?,
        })
        .map_err(|e| e.to_string())
    }
    pub fn restore(text: &str, definition: Definition) -> Result<Self> {
        if text.len() > 128 * 1024 * 1024 || definition.goals020.is_none() {
            return Err("invalid 020 snapshot/definition".into());
        }
        let save: Save020 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if save.profile != PROFILE020 || save.content != definition.data.content {
            return Err("020 profile mismatch; legacy history requires a new campaign".into());
        }
        save.navigation.validate()?;
        Ok(Self {
            inner: Runtime::restore(&save.inner, definition)?,
            navigation: save.navigation,
        })
    }
    pub fn restore_config(text: &str) -> Result<Self> {
        if text.len() > 128 * 1024 * 1024 {
            return Err("020 snapshot exceeds bound".into());
        }
        let save: Save020 = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if save.profile != PROFILE020 {
            return Err("020 profile mismatch".into());
        }
        let inner: Save = serde_json::from_str(&save.inner).map_err(|e| e.to_string())?;
        if inner.config.mode != save.content.mode
            || inner.config.never_again != save.content.never_again
        {
            return Err("020 saved content/config mismatch".into());
        }
        let d = Definition::load(
            GameData::load_with_content(&dd_data::game_dir(), save.content)?,
            CampaignCandidates::documented()?,
        )?
        .with_automation018()?
        .with_content019()?
        .with_steward020()?;
        Self::restore(text, d)
    }
}
