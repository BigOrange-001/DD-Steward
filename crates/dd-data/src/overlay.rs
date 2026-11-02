//! Layer inventory is metadata-only outside combat. Explicit collision audits
//! compare bytes without parsing non-combat data or selecting a winning layer.
use crate::{
    combat::Result,
    content::{ContentConfig, PACKAGES},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Layer {
    pub id: String,
    pub package: String,
    pub root: PathBuf,
    pub mode: bool,
    pub folder_override: bool,
}
#[derive(Debug, Clone)]
pub struct Overlay {
    pub(crate) cache024c: std::sync::Arc<crate::cache024c::Cache>,
    pub config: ContentConfig,
    pub is_new_game_plus: bool,
    pub layers: Vec<Layer>,
    /// All relative paths and sources, including ignored extensions and files
    /// shadowed by a mode. Inventory does not read file contents.
    pub paths: BTreeMap<String, Vec<usize>>,
    pub unknown_extensions: BTreeMap<String, usize>,
}
pub fn combat_path(path: &str) -> bool {
    (path.starts_with("trinkets/")
        && (path.ends_with(".entries.trinkets.json") || path.ends_with(".sets.trinkets.json")))
        || (path.starts_with("shared/quirk/") && path.ends_with("quirk_library.json"))
        || (path.starts_with("inventory/")
            && (path.ends_with(".supply.inventory.items.darkest")
                || path.ends_with(".estate.inventory.items.darkest")))
        || (path.starts_with("effects/") && path.ends_with(".effects.darkest"))
        || (path.starts_with("shared/buffs/") && path.ends_with(".json"))
        || (path.starts_with("dungeons/")
            && (path.ends_with(".mash.darkest") || path.ends_with(".wavemash.darkest")))
        || (path.starts_with("raid/flashback/") && path.ends_with(".flashbacks.json"))
        || (path.starts_with("raid/actor_dot/") && path.ends_with(".actor_dot.json"))
        || (path.starts_with("raid/ai/") && path.ends_with(".monster_brains.json"))
        || (path.starts_with("shared/trait/") && path.ends_with(".trait_library.json"))
        || path == "shared/trait/trait_library.json"
        || ((path.starts_with("heroes/") || path.starts_with("monsters/"))
            && path.ends_with(".info.darkest"))
        || path == "shared/rules.json"
}
impl Overlay {
    pub fn json(&self, relative: &str) -> Result<std::sync::Arc<Value>> {
        self.cached(&format!("json:{relative}"), &[relative.into()], || {
            serde_json::from_slice(&self.read(relative)?).map_err(|e| e.to_string())
        })
    }
    pub fn load(root: &Path, config: ContentConfig) -> Result<Self> {
        config.validate_root(root)?;
        let config = config.normalized()?;
        let mut layers = vec![Layer {
            id: "base".into(),
            package: "base".into(),
            root: root.into(),
            mode: false,
            folder_override: false,
        }];
        for id in config.packages.iter().filter(|id| *id != "base") {
            for (index, path) in PACKAGES
                .iter()
                .find(|(p, _)| *p == id)
                .unwrap()
                .1
                .iter()
                .enumerate()
            {
                layers.push(Layer {
                    id: format!("package:{id}:{index}"),
                    package: id.clone(),
                    root: root.join(path),
                    mode: false,
                    folder_override: false,
                });
            }
        }
        let packages = layers.clone();
        let mut is_new_game_plus = false;
        let mode_root = if config.mode == "bloodmoon" {
            root.join("dlc/580100_crimson_court/features/crimson_court/modes/bloodmoon")
        } else {
            root.join("modes").join(&config.mode)
        };
        // The packaged fixture intentionally has no interface-only modes/base.
        if config.mode != "base" || mode_root.exists() {
            let settings: Value = serde_json::from_slice(
                &crate::telemetry024c::read(mode_root.join("mode_settings.json"))
                    .map_err(|e| format!("missing mode settings {}: {e}", mode_root.display()))?,
            )
            .map_err(|e| format!("mode settings: {e}"))?;
            is_new_game_plus = settings["mode_settings"]
                .get("is_new_game_plus")
                .map(|v| v.as_bool().ok_or("invalid is_new_game_plus mode flag"))
                .transpose()?
                .unwrap_or(false);
            let folder_override = settings["mode_settings"]["folder_override"]
                .as_bool()
                .ok_or("mode settings missing folder_override")?;
            let replacement = settings["mode_settings"]
                .get("replaces_default_mode")
                .map(|v| v.as_str().ok_or("invalid replaces_default_mode value"))
                .transpose()?;
            if (config.mode == "bloodmoon" && replacement != Some("new_game_plus"))
                || (config.mode != "bloodmoon" && replacement.is_some())
            {
                return Err("unsupported mode replacement declaration".into());
            }
            layers.push(Layer {
                id: format!("mode:{}", config.mode),
                package: "base".into(),
                root: mode_root.clone(),
                mode: true,
                folder_override,
            });
        }
        for package in packages.iter().filter(|p| p.package != "base") {
            let path = package.root.join("modes").join(&config.mode);
            if path.is_dir() && path != mode_root {
                let settings_path = path.join("mode_settings.json");
                let folder_override = if settings_path.is_file() {
                    let settings: Value = serde_json::from_slice(
                        &crate::telemetry024c::read(&settings_path).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    if settings["mode_settings"]
                        .get("replaces_default_mode")
                        .is_some()
                    {
                        return Err("unsupported package mode replacement declaration".into());
                    }
                    settings["mode_settings"]["folder_override"]
                        .as_bool()
                        .ok_or("package mode settings missing folder_override")?
                } else {
                    true
                };
                layers.push(Layer {
                    id: format!("{}:mode:{}", package.id, config.mode),
                    package: package.package.clone(),
                    root: path,
                    mode: true,
                    folder_override,
                });
            }
        }
        let mut overlay = Self {
            cache024c: Default::default(),
            is_new_game_plus,
            config,
            layers,
            paths: BTreeMap::new(),
            unknown_extensions: BTreeMap::new(),
        };
        for index in 0..overlay.layers.len() {
            let root = overlay.layers[index].root.clone();
            overlay.scan(index, &root, &root)?;
        }
        Ok(overlay)
    }
    /// 017 adds the shared Courtyard CSV scope that lives above its feature directory.
    /// This is a separate enabled-content view: legacy combat/map/town inventories stay frozen.
    pub fn campaign_view(&self) -> Result<Self> {
        let mut view = self.clone();
        if !self.config.packages.iter().any(|p| p == "crimson_court") {
            return Ok(view);
        }
        if self
            .layers
            .iter()
            .any(|l| l.id == "package:crimson_court:shared017")
        {
            return Ok(view);
        }
        let root = self
            .layers
            .first()
            .ok_or("missing content base layer")?
            .root
            .join("dlc/580100_crimson_court");
        let index = view.layers.len();
        view.layers.push(Layer {
            id: "package:crimson_court:shared017".into(),
            package: "crimson_court".into(),
            root: root.clone(),
            mode: false,
            folder_override: false,
        });
        view.scan(index, &root, &root.join("curios"))?;
        let mode = root.join("modes").join(&self.config.mode);
        if mode.join("curios").is_dir() {
            let settings = mode.join("mode_settings.json");
            let folder_override = if settings.is_file() {
                let raw: Value = serde_json::from_slice(
                    &crate::telemetry024c::read(&settings).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if raw["mode_settings"].get("replaces_default_mode").is_some() {
                    return Err("unsupported shared-scope mode replacement".into());
                }
                raw["mode_settings"]["folder_override"]
                    .as_bool()
                    .ok_or("shared-scope mode missing folder_override")?
            } else {
                true
            };
            let index = view.layers.len();
            view.layers.push(Layer {
                id: format!("package:crimson_court:shared017:mode:{}", self.config.mode),
                package: "crimson_court".into(),
                root: mode.clone(),
                mode: true,
                folder_override,
            });
            view.scan(index, &mode, &mode.join("curios"))?;
        }
        Ok(view)
    }
    fn scan(&mut self, layer: usize, root: &Path, path: &Path) -> Result<()> {
        let mut entries = std::fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let p = entry.path();
            if path == root
                && ["dlc", "modes", "mods", "features"]
                    .iter()
                    .any(|name| entry.file_name() == *name)
            {
                continue;
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!("refusing linked content {}", p.display()));
            }
            if kind.is_dir() {
                self.scan(layer, root, &p)?;
            } else if kind.is_file() {
                let relative = p
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("non-UTF8 data path")?
                    .replace('\\', "/");
                self.paths.entry(relative).or_default().push(layer);
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("<none>");
                if !["darkest", "json", "csv"].contains(&ext) {
                    *self.unknown_extensions.entry(ext.into()).or_default() += 1;
                }
            }
        }
        Ok(())
    }
    pub fn sources(&self, relative: &str) -> Vec<usize> {
        let all = self.paths.get(relative).map(Vec::as_slice).unwrap_or(&[]);
        all.iter()
            .copied()
            .filter(|index| {
                let layer = &self.layers[*index];
                layer.mode
                    || !all.iter().any(|i| {
                        let mode = &self.layers[*i];
                        mode.mode && mode.folder_override && mode.package == layer.package
                    })
            })
            .collect()
    }
    pub fn read(&self, relative: &str) -> Result<Vec<u8>> {
        let mut versions = self.read_versions(relative)?;
        if versions.len() != 1 {
            return Err(format!("unresolved same-path conflict {relative}"));
        }
        Ok(versions.remove(0))
    }
    /// Libraries merge these versions by logical ID; scalar files require read().
    pub fn read_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !combat_path(relative) {
            return Err(format!("outside combat read scope: {relative}"));
        }
        self.read_active_versions(relative)
    }
    /// Stage 013: explicit original templates/libraries, never player save paths.
    pub fn read_initial_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !crate::initial013::source_path(relative) {
            return Err(format!("outside initial-level read scope: {relative}"));
        }
        self.read_active_versions(relative)
    }
    pub fn read_initial_source(&self, relative: &str, layer: usize) -> Result<Vec<u8>> {
        if !crate::initial013::source_path(relative) || !self.sources(relative).contains(&layer) {
            return Err("source is outside enabled initial layers".into());
        }
        crate::telemetry024c::read(self.layers[layer].root.join(relative))
            .map_err(|e| e.to_string())
    }
    pub fn read_initial(&self, relative: &str) -> Result<Vec<u8>> {
        let mut versions = self.read_initial_versions(relative)?;
        if versions.len() != 1 {
            return Err(format!("unresolved same-path conflict {relative}"));
        }
        Ok(versions.remove(0))
    }
    pub fn read_map_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !crate::maps014::source_path(relative) {
            return Err(format!("outside map read scope: {relative}"));
        }
        self.read_active_versions(relative)
    }
    pub fn read_map(&self, relative: &str) -> Result<Vec<u8>> {
        let mut v = self.read_map_versions(relative)?;
        if v.len() != 1 {
            return Err(format!("unresolved same-path map conflict {relative}"));
        }
        Ok(v.remove(0))
    }
    pub fn read_map_source(&self, relative: &str, index: usize) -> Result<Vec<u8>> {
        if !crate::maps014::source_path(relative) || !self.sources(relative).contains(&index) {
            return Err("outside enabled map source".into());
        }
        crate::telemetry024c::read(self.layers[index].root.join(relative))
            .map_err(|e| e.to_string())
    }
    pub fn read_opening_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !crate::opening015::source_path(relative) {
            return Err(format!("outside opening read scope: {relative}"));
        }
        self.read_active_versions(relative)
    }
    pub fn read_opening(&self, relative: &str) -> Result<Vec<u8>> {
        let mut v = self.read_opening_versions(relative)?;
        if v.len() != 1 {
            return Err(format!("unresolved opening same-path conflict {relative}"));
        }
        Ok(v.remove(0))
    }
    pub fn read_town_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !crate::town016::source_path(relative) {
            return Err(format!("outside town read scope: {relative}"));
        }
        self.read_active_versions(relative)
    }
    pub fn read_town(&self, relative: &str) -> Result<Vec<u8>> {
        let mut v = self.read_town_versions(relative)?;
        if v.len() != 1 {
            return Err(format!("unresolved town same-path conflict {relative}"));
        }
        Ok(v.remove(0))
    }
    /// Strictly scoped additional campaign inputs owned by task 019.
    pub fn read_content019(&self, relative: &str) -> Result<Vec<u8>> {
        if relative != "shared/com.rules.json"
            && !(relative.starts_with("campaign/provision/")
                && relative.ends_with(".json")
                && !relative.contains(".."))
        {
            return Err("outside content019 read scope".into());
        }
        if relative == "shared/com.rules.json"
            && !self.config.packages.iter().any(|p| p == "color_of_madness")
        {
            return Err("content019 CoM input requires enabled color_of_madness".into());
        }
        let mut versions = self.read_active_versions(relative)?;
        if versions.len() != 1 {
            return Err(format!(
                "unresolved content019 same-path conflict {relative}"
            ));
        }
        Ok(versions.remove(0))
    }
    /// Task 023 reads only the selected quirk behaviour tables. Old profiles
    /// neither parse them nor change their source fingerprints.
    pub fn read_quirk_actouts023(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !crate::mechanisms023::enabled(&self.config, crate::mechanisms023::FIXES)
            || !relative.starts_with("shared/quirk/")
            || !relative.ends_with("quirk_act_outs.json")
            || relative.contains("..")
        {
            return Err("outside task 023 quirk behaviour scope".into());
        }
        self.read_active_versions(relative)
    }
    /// The 018 owner adds only this previously unconsumed roster setting.
    pub fn read_automation018(&self, relative: &str) -> Result<Vec<u8>> {
        if relative != "campaign/roster/roster.settings.json" {
            return Err("outside automation018 read scope".into());
        }
        let mut versions = self.read_active_versions(relative)?;
        if versions.len() != 1 {
            return Err(format!(
                "unresolved automation018 same-path conflict {relative}"
            ));
        }
        Ok(versions.remove(0))
    }
    pub fn read_campaign_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        if !crate::campaign017::source_path(relative) {
            return Err(format!("outside campaign read scope: {relative}"));
        }
        self.read_active_versions(relative)
    }
    pub fn read_campaign_source(&self, relative: &str, layer: usize) -> Result<Vec<u8>> {
        if !crate::campaign017::source_path(relative) || !self.sources(relative).contains(&layer) {
            return Err("outside selected campaign source".into());
        }
        crate::telemetry024c::read(self.layers[layer].root.join(relative))
            .map_err(|e| e.to_string())
    }
    fn read_active_versions(&self, relative: &str) -> Result<Vec<Vec<u8>>> {
        let sources = self.sources(relative);
        let mut bytes = vec![];
        for index in sources {
            let path = self.layers[index].root.join(relative);
            let next = crate::telemetry024c::read(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if !bytes.contains(&next) {
                bytes.push(next);
            }
        }
        if bytes.is_empty() {
            return Err(format!("missing content file {relative}"));
        }
        Ok(bytes)
    }
    pub fn combat_files(&self, prefix: &str, suffix: &str) -> Vec<String> {
        self.paths
            .keys()
            .filter(|p| p.starts_with(prefix) && p.ends_with(suffix) && combat_path(p))
            .cloned()
            .collect()
    }
    /// Metadata only, including paths whose active sources may be identical.
    pub fn collisions(&self) -> Vec<(&str, Vec<usize>)> {
        self.paths
            .keys()
            .filter(|p| {
                Path::new(p)
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ["darkest", "json", "csv"].contains(&ext))
            })
            .filter_map(|path| {
                let sources = self.sources(path);
                (sources.len() > 1).then_some((path.as_str(), sources))
            })
            .collect()
    }
    /// Audit only: group active sources by exact bytes, retaining every source.
    /// Non-combat bytes never escape this function for business parsing. Normal
    /// loading does not call it; only indexed text collisions are eligible.
    pub fn collision_content_groups(&self, relative: &str) -> Result<Vec<Vec<usize>>> {
        let sources = self
            .collisions()
            .into_iter()
            .find_map(|(path, sources)| (path == relative).then_some(sources))
            .ok_or_else(|| format!("not an indexed text collision: {relative}"))?;
        let mut groups: Vec<(Vec<u8>, Vec<usize>)> = vec![];
        for index in sources {
            let path = self.layers[index].root.join(relative);
            let bytes = crate::telemetry024c::read(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if let Some((_, members)) = groups.iter_mut().find(|(old, _)| *old == bytes) {
                members.push(index);
            } else {
                groups.push((bytes, vec![index]));
            }
        }
        Ok(groups.into_iter().map(|(_, sources)| sources).collect())
    }
}

