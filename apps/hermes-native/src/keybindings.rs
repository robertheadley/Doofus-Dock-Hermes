//! Persisted shortcut overrides for first-party Hermes Native actions.
//!
//! This module intentionally contains only actions with a working native
//! handler. A settings page must not advertise pinned Desktop actions that
//! the native app cannot execute yet.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const FORMAT_VERSION: u32 = 1;
const MAX_COMBOS_PER_ACTION: usize = 4;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPlatform {
    Mac,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct ActionDefinition {
    pub id: &'static str,
    pub category: &'static str,
    defaults: &'static [&'static str],
    mac_defaults: Option<&'static [&'static str]>,
}

impl ActionDefinition {
    pub fn defaults_for(self, platform: KeyPlatform) -> Result<Vec<String>, String> {
        let values = if platform == KeyPlatform::Mac {
            self.mac_defaults.unwrap_or(self.defaults)
        } else {
            self.defaults
        };
        values
            .iter()
            .map(|value| normalize_combo(value, platform))
            .collect()
    }
}

/// Action IDs follow the pinned Hermes keybind vocabulary where available.
/// These are the current hard-coded native shortcuts; unsupported Hermes
/// actions are deliberately absent until their native handlers exist.
pub const NATIVE_ACTIONS: &[ActionDefinition] = &[
    ActionDefinition {
        id: "nav.commandPalette",
        category: "navigation",
        defaults: &["mod+k", "mod+p"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "view.findInPage",
        category: "view",
        defaults: &["mod+f"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "view.findNext",
        category: "view",
        defaults: &["mod+g"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "view.findPrevious",
        category: "view",
        defaults: &["mod+shift+g"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "session.next",
        category: "session",
        defaults: &["ctrl+tab", "ctrl+pagedown"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "session.prev",
        category: "session",
        defaults: &["ctrl+shift+tab", "ctrl+pageup"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "view.closeTab",
        category: "view",
        defaults: &["mod+w"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "view.reopenTab",
        category: "view",
        defaults: &["ctrl+shift+t"],
        mac_defaults: Some(&["mod+ctrl+shift+t"]),
    },
    ActionDefinition {
        id: "view.toggleSidebar",
        category: "view",
        defaults: &["mod+b"],
        mac_defaults: None,
    },
    ActionDefinition {
        id: "view.toggleRightSidebar",
        category: "view",
        defaults: &["mod+j"],
        mac_defaults: None,
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredBindings {
    schema_version: u32,
    bindings: BTreeMap<String, Vec<String>>,
}

/// Local keybinding state, independent of gateway profiles and dock presets.
#[derive(Debug, Clone)]
pub struct KeybindingStore {
    path: PathBuf,
    platform: KeyPlatform,
    overrides: BTreeMap<String, Vec<String>>,
}

impl KeybindingStore {
    /// Load overrides from `path`; a missing file means all source-compatible
    /// native defaults. Invalid files fail explicitly and are never overwritten.
    pub fn load(path: impl Into<PathBuf>, platform: KeyPlatform) -> Result<Self, String> {
        let path = path.into();
        let overrides = match fs::read(&path) {
            Ok(bytes) => {
                let stored: StoredBindings = serde_json::from_slice(&bytes).map_err(|error| {
                    format!("Invalid keybinding file {}: {error}", path.display())
                })?;
                if stored.schema_version != FORMAT_VERSION {
                    return Err(format!(
                        "Unsupported keybinding schema version {} in {}",
                        stored.schema_version,
                        path.display()
                    ));
                }
                validate_overrides(stored.bindings, platform)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => {
                return Err(format!(
                    "Could not read keybindings {}: {error}",
                    path.display()
                ));
            }
        };
        Ok(Self {
            path,
            platform,
            overrides,
        })
    }

    /// Resolve action bindings: explicit override first, otherwise shipped
    /// native default. An empty override is a deliberate clear.
    pub fn bindings_for(&self, action_id: &str) -> Result<Vec<String>, String> {
        let action = action_definition(action_id)
            .ok_or_else(|| format!("Unsupported native keybinding action: {action_id}"))?;
        if let Some(override_bindings) = self.overrides.get(action_id) {
            return Ok(override_bindings.clone());
        }
        action.defaults_for(self.platform)
    }

    pub fn overrides(&self) -> &BTreeMap<String, Vec<String>> {
        &self.overrides
    }

    pub fn set_binding(&mut self, action_id: &str, combos: &[&str]) -> Result<(), String> {
        if action_definition(action_id).is_none() {
            return Err(format!("Unsupported native keybinding action: {action_id}"));
        }
        let normalized = normalize_bindings(combos.iter().copied(), self.platform)?;
        self.commit_override(action_id, normalized)
    }

    /// Clear the action while retaining an explicit empty override, so its
    /// default does not silently return on the next read.
    pub fn clear_binding(&mut self, action_id: &str) -> Result<(), String> {
        if action_definition(action_id).is_none() {
            return Err(format!("Unsupported native keybinding action: {action_id}"));
        }
        self.commit_override(action_id, Vec::new())
    }

    /// Remove the override and restore the platform's shipped native default.
    pub fn reset_binding(&mut self, action_id: &str) -> Result<(), String> {
        if action_definition(action_id).is_none() {
            return Err(format!("Unsupported native keybinding action: {action_id}"));
        }
        let mut next = self.overrides.clone();
        next.remove(action_id);
        self.commit(next)
    }

    pub fn reset_all(&mut self) -> Result<(), String> {
        self.commit(BTreeMap::new())
    }

    /// Return deterministic conflict IDs for the requested action's current
    /// combos. Intentional conflicts are allowed and reported, not rejected.
    pub fn conflicts_for(&self, action_id: &str) -> Result<Vec<String>, String> {
        let current = self.bindings_for(action_id)?;
        let current: BTreeSet<_> = current.into_iter().collect();
        let mut conflicts = Vec::new();
        for action in NATIVE_ACTIONS {
            if action.id == action_id {
                continue;
            }
            if self
                .bindings_for(action.id)?
                .iter()
                .any(|combo| current.contains(combo))
            {
                conflicts.push(action.id.to_owned());
            }
        }
        Ok(conflicts)
    }

    fn commit_override(&mut self, action_id: &str, combos: Vec<String>) -> Result<(), String> {
        let mut next = self.overrides.clone();
        next.insert(action_id.to_owned(), combos);
        self.commit(next)
    }

    fn commit(&mut self, overrides: BTreeMap<String, Vec<String>>) -> Result<(), String> {
        let checked = validate_overrides(overrides, self.platform)?;
        write_atomic(
            &self.path,
            &StoredBindings {
                schema_version: FORMAT_VERSION,
                bindings: checked.clone(),
            },
        )?;
        self.overrides = checked;
        Ok(())
    }
}

/// Resolve the app's standard per-user location. Tests and callers may use
/// `KeybindingStore::load` with an explicit path to avoid process-global env.
pub fn default_path() -> PathBuf {
    std::env::var_os("HERMES_NATIVE_PROFILE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .map(|path| path.join("HermesNative"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("HermesNative"))
        .join("keybindings.json")
}

/// Canonicalize a Hermes-style combo such as `Mod + Shift + ]`.
///
/// On non-Mac platforms physical Control folds into the primary (`mod`)
/// modifier, matching Hermes' `canonicalizeCombo`; on Mac it stays distinct.
pub fn normalize_combo(value: &str, platform: KeyPlatform) -> Result<String, String> {
    let parts: Vec<_> = value
        .split('+')
        .map(|part| part.trim().to_ascii_lowercase())
        .collect();
    if parts.is_empty() || parts.iter().any(String::is_empty) {
        return Err(format!("Invalid key combo: {value:?}"));
    }
    let mut modifiers = BTreeSet::new();
    let mut base: Option<String> = None;
    for token in parts {
        let modifier = match token.as_str() {
            "mod" | "cmd" | "command" | "meta" => Some("mod"),
            "ctrl" | "control" => Some("ctrl"),
            "alt" | "option" => Some("alt"),
            "shift" => Some("shift"),
            _ => None,
        };
        if let Some(modifier) = modifier {
            if !modifiers.insert(modifier) {
                return Err(format!("Duplicate {modifier} modifier in combo {value:?}"));
            }
            continue;
        }
        if base.replace(normalize_base_key(&token)?).is_some() {
            return Err(format!("Key combo has more than one base key: {value:?}"));
        }
    }
    let base = base.ok_or_else(|| format!("Key combo has no base key: {value:?}"))?;
    if platform == KeyPlatform::Other && modifiers.remove("ctrl") {
        modifiers.insert("mod");
    }
    let mut canonical = Vec::new();
    for modifier in ["mod", "ctrl", "alt", "shift"] {
        if modifiers.contains(modifier) {
            canonical.push(modifier);
        }
    }
    canonical.push(&base);
    Ok(canonical.join("+"))
}

fn normalize_base_key(value: &str) -> Result<String, String> {
    let key = match value {
        "esc" => "escape".to_owned(),
        "pgup" => "pageup".to_owned(),
        "pgdn" => "pagedown".to_owned(),
        "arrowup" => "up".to_owned(),
        "arrowdown" => "down".to_owned(),
        "arrowleft" => "left".to_owned(),
        "arrowright" => "right".to_owned(),
        other => other.to_owned(),
    };
    let valid_single = key.len() == 1
        && (key.as_bytes()[0].is_ascii_alphanumeric() || "`\\[];',./=-".contains(key.as_str()));
    let valid_named = matches!(
        key.as_str(),
        "enter"
            | "escape"
            | "space"
            | "backspace"
            | "delete"
            | "tab"
            | "pageup"
            | "pagedown"
            | "up"
            | "down"
            | "left"
            | "right"
    );
    let valid_function = key
        .strip_prefix('f')
        .filter(|digits| {
            !digits.is_empty() && digits.len() <= 2 && digits.chars().all(|c| c.is_ascii_digit())
        })
        .and_then(|digits| digits.parse::<u8>().ok())
        .is_some_and(|number| (1..=24).contains(&number));
    if valid_single || valid_named || valid_function {
        Ok(key)
    } else {
        Err(format!("Unsupported shortcut key: {value:?}"))
    }
}

fn normalize_bindings<'a>(
    combos: impl IntoIterator<Item = &'a str>,
    platform: KeyPlatform,
) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for combo in combos {
        if normalized.len() == MAX_COMBOS_PER_ACTION {
            return Err(format!(
                "An action may have at most {MAX_COMBOS_PER_ACTION} key combos"
            ));
        }
        let combo = normalize_combo(combo, platform)?;
        if normalized.contains(&combo) {
            return Err(format!("Duplicate key combo {combo:?}"));
        }
        normalized.push(combo);
    }
    Ok(normalized)
}

fn validate_overrides(
    overrides: BTreeMap<String, Vec<String>>,
    platform: KeyPlatform,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut normalized = BTreeMap::new();
    for (action_id, combos) in overrides {
        if action_definition(&action_id).is_none() {
            // Preserve unknown IDs for forward compatibility, but do not
            // expose them as actions. Validate their values before retaining.
            if !valid_action_id(&action_id) {
                return Err(format!("Invalid keybinding action ID: {action_id:?}"));
            }
        }
        let entries: Vec<&str> = combos.iter().map(String::as_str).collect();
        normalized.insert(action_id, normalize_bindings(entries, platform)?);
    }
    Ok(normalized)
}

fn valid_action_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'_' | b'-'))
}

fn action_definition(action_id: &str) -> Option<ActionDefinition> {
    NATIVE_ACTIONS
        .iter()
        .copied()
        .find(|action| action.id == action_id)
}

fn write_atomic(path: &Path, stored: &StoredBindings) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Keybinding path has no parent directory".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Could not create keybinding directory {}: {error}",
            parent.display()
        )
    })?;
    let bytes = serde_json::to_vec_pretty(stored)
        .map_err(|error| format!("Could not serialize keybindings: {error}"))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .ok_or_else(|| "Keybinding path has no file name".to_owned())?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{file_name}.tmp-{}-{sequence}",
        std::process::id()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Could not create temporary keybinding file: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("Could not write temporary keybinding file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not sync temporary keybinding file: {error}"))?;
        drop(file);
        let checked: StoredBindings =
            serde_json::from_slice(&fs::read(&temporary).map_err(|error| {
                format!("Could not validate temporary keybinding file: {error}")
            })?)
            .map_err(|error| format!("Temporary keybinding file did not validate: {error}"))?;
        if checked != *stored {
            return Err("Temporary keybinding file changed during validation".to_owned());
        }
        fs::rename(&temporary, path).map_err(|error| {
            format!(
                "Could not atomically replace keybinding file {}: {error}",
                path.display()
            )
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hermes-native-keybindings-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create isolated test directory");
            Self(path)
        }

        fn file(&self) -> PathBuf {
            self.0.join("keybindings.json")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn defaults_are_available_without_creating_a_file() {
        let directory = TestDirectory::new();
        let store = KeybindingStore::load(directory.file(), KeyPlatform::Other).unwrap();
        assert_eq!(
            store.bindings_for("nav.commandPalette").unwrap(),
            ["mod+k", "mod+p"]
        );
        assert!(!directory.file().exists());
        assert!(store.bindings_for("view.showBrowser").is_err());
    }

    #[test]
    fn custom_binding_survives_reload_clear_reset_and_reset_all() {
        let directory = TestDirectory::new();
        let path = directory.file();
        let mut store = KeybindingStore::load(&path, KeyPlatform::Other).unwrap();
        store
            .set_binding("nav.commandPalette", &["CTRL + SHIFT + K"])
            .unwrap();
        assert_eq!(
            store.bindings_for("nav.commandPalette").unwrap(),
            ["mod+shift+k"]
        );

        let mut reloaded = KeybindingStore::load(&path, KeyPlatform::Other).unwrap();
        assert_eq!(
            reloaded.bindings_for("nav.commandPalette").unwrap(),
            ["mod+shift+k"]
        );
        reloaded.clear_binding("nav.commandPalette").unwrap();
        assert!(
            reloaded
                .bindings_for("nav.commandPalette")
                .unwrap()
                .is_empty()
        );
        let cleared = KeybindingStore::load(&path, KeyPlatform::Other).unwrap();
        assert!(
            cleared
                .bindings_for("nav.commandPalette")
                .unwrap()
                .is_empty()
        );

        reloaded.reset_binding("nav.commandPalette").unwrap();
        assert_eq!(
            reloaded.bindings_for("nav.commandPalette").unwrap(),
            ["mod+k", "mod+p"]
        );
        reloaded
            .set_binding("view.toggleSidebar", &["alt+b"])
            .unwrap();
        reloaded.reset_all().unwrap();
        assert_eq!(
            reloaded.bindings_for("view.toggleSidebar").unwrap(),
            ["mod+b"]
        );
        assert!(
            KeybindingStore::load(&path, KeyPlatform::Other)
                .unwrap()
                .overrides()
                .is_empty()
        );
    }

    #[test]
    fn normalization_orders_modifiers_and_folds_control_on_non_mac() {
        assert_eq!(
            normalize_combo("SHIFT + CTRL + K", KeyPlatform::Other).unwrap(),
            "mod+shift+k"
        );
        assert_eq!(
            normalize_combo("shift+control+cmd+K", KeyPlatform::Mac).unwrap(),
            "mod+ctrl+shift+k"
        );
        assert_eq!(
            normalize_combo("mod+PgUp", KeyPlatform::Mac).unwrap(),
            "mod+pageup"
        );
        assert_eq!(
            normalize_combo("ctrl+tab", KeyPlatform::Mac).unwrap(),
            "ctrl+tab"
        );
        assert_eq!(
            normalize_combo("ctrl+tab", KeyPlatform::Other).unwrap(),
            "mod+tab"
        );
        assert_eq!(
            normalize_combo("PgDn", KeyPlatform::Other).unwrap(),
            "pagedown"
        );
        assert_eq!(
            normalize_combo("Shift + X", KeyPlatform::Other).unwrap(),
            "shift+x"
        );
    }

    #[test]
    fn invalid_bindings_and_unknown_actions_do_not_mutate_persisted_state() {
        let directory = TestDirectory::new();
        let path = directory.file();
        let mut store = KeybindingStore::load(&path, KeyPlatform::Other).unwrap();
        assert!(
            store
                .set_binding("view.notImplemented", &["mod+x"])
                .is_err()
        );
        assert!(
            store
                .set_binding("nav.commandPalette", &["mod+mod+x"])
                .is_err()
        );
        assert!(
            store
                .set_binding("nav.commandPalette", &["ctrl+x", "mod+x"])
                .is_err()
        );
        assert!(!path.exists());
        assert_eq!(
            store.bindings_for("nav.commandPalette").unwrap(),
            ["mod+k", "mod+p"]
        );
    }

    #[test]
    fn conflicts_are_canonical_and_ordered_by_supported_action_list() {
        let directory = TestDirectory::new();
        let mut store = KeybindingStore::load(directory.file(), KeyPlatform::Other).unwrap();
        store.set_binding("view.findInPage", &["CTRL+K"]).unwrap();
        assert_eq!(
            store.conflicts_for("view.findInPage").unwrap(),
            ["nav.commandPalette"]
        );
        store.set_binding("view.findNext", &["MOD+K"]).unwrap();
        assert_eq!(
            store.conflicts_for("nav.commandPalette").unwrap(),
            ["view.findInPage", "view.findNext"]
        );
    }

    #[test]
    fn corrupt_or_future_version_files_fail_without_overwriting() {
        let directory = TestDirectory::new();
        let path = directory.file();
        fs::write(&path, b"not json").unwrap();
        assert!(KeybindingStore::load(&path, KeyPlatform::Other).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not json");

        fs::write(
            &path,
            serde_json::to_vec(&StoredBindings {
                schema_version: FORMAT_VERSION + 1,
                bindings: BTreeMap::new(),
            })
            .unwrap(),
        )
        .unwrap();
        assert!(KeybindingStore::load(&path, KeyPlatform::Other).is_err());
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["schema_version"], FORMAT_VERSION + 1);
    }
}
