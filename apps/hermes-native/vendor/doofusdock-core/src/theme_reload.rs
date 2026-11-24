//! Opt-in, read-only source polling for staged theme-package updates.
use crate::ThemePackageEditor;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Maximum bytes read from a watched theme package.
pub const MAX_THEME_SOURCE_BYTES: u64 = 1024 * 1024;
/// Minimum polling interval used by `ThemeSourceWatcher`.
pub const THEME_SOURCE_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceSignature {
    len: u64,
    fingerprint: u64,
}

/// Watches one explicitly selected package path. Changes are validated and
/// staged; a host decides when a candidate should replace its active package.
///
/// This type never writes the source file. Poll it from the host's normal
/// update loop and call [`activate`](Self::activate) only after applying the
/// host's active-package and draft-conflict policy.
#[derive(Clone, Debug)]
pub struct ThemeSourceWatcher {
    path: PathBuf,
    enabled: bool,
    live_apply: bool,
    observed: SourceSignature,
    next_poll: Option<Instant>,
    candidate: Option<ThemePackageEditor>,
    previous: Option<ThemePackageEditor>,
    error: Option<String>,
    source_error: bool,
}

impl ThemeSourceWatcher {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let observed = source_signature(&path)?;
        Ok(Self {
            path,
            enabled: false,
            live_apply: false,
            observed,
            next_poll: None,
            candidate: None,
            previous: None,
            error: None,
            source_error: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.live_apply = false;
        }
        self.next_poll = None;
    }

    pub fn live_apply(&self) -> bool {
        self.live_apply
    }

    pub fn set_live_apply(&mut self, live_apply: bool) {
        self.live_apply = live_apply;
        if live_apply {
            self.enabled = true;
        }
        self.next_poll = None;
    }

    pub fn candidate(&self) -> Option<&ThemePackageEditor> {
        self.candidate.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn can_revert(&self) -> bool {
        self.previous.is_some()
    }

    pub fn clear_revert_history(&mut self) {
        self.previous = None;
    }

    /// Poll at most twice per second; returns true when a valid new package is staged.
    pub fn poll(&mut self, now: Instant) -> bool {
        if !self.enabled || self.next_poll.is_some_and(|deadline| now < deadline) {
            return false;
        }
        self.next_poll = Some(now + THEME_SOURCE_POLL_INTERVAL);
        let signature = match source_signature(&self.path) {
            Ok(signature) => signature,
            Err(error) => {
                self.error = Some(error);
                self.source_error = true;
                return false;
            }
        };
        if signature == self.observed {
            if self.source_error {
                self.error = None;
                self.source_error = false;
            }
            return false;
        }
        self.observed = signature;
        self.source_error = false;
        match ThemePackageEditor::load(&self.path) {
            Ok(candidate) => {
                self.candidate = Some(candidate);
                self.error = None;
                true
            }
            Err(error) => {
                self.candidate = None;
                self.error = Some(format!("Changed theme source was rejected: {error}"));
                self.source_error = false;
                false
            }
        }
    }

    pub fn activate(&mut self, active: &mut ThemePackageEditor) -> bool {
        let Some(candidate) = self.candidate.take() else {
            return false;
        };
        self.previous = Some(active.clone());
        *active = candidate;
        self.error = None;
        true
    }

    pub fn revert_activation(&mut self, active: &mut ThemePackageEditor) -> bool {
        let Some(previous) = self.previous.take() else {
            return false;
        };
        *active = previous;
        true
    }

    pub fn discard_candidate(&mut self) {
        self.candidate = None;
    }
}

fn source_signature(path: &Path) -> Result<SourceSignature, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Could not inspect watched theme source {}: {error}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "Watched theme source is not a regular file: {}",
            path.display()
        ));
    }
    if metadata.len() > MAX_THEME_SOURCE_BYTES {
        return Err(format!(
            "Watched theme source exceeds the 1 MiB limit: {}",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "Could not read watched theme source {}: {error}",
            path.display()
        )
    })?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_THEME_SOURCE_BYTES {
        return Err(format!(
            "Watched theme source changed during read: {}",
            path.display()
        ));
    }
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(SourceSignature {
        len: bytes.len() as u64,
        fingerprint: hasher.finish(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ThemeManifest, ThemeValue};
    use std::{
        fs::OpenOptions,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> PathBuf {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "doofusdock-theme-reload-{}-{id}.json",
            std::process::id()
        ))
    }

    fn editor_with_accent(accent: &str) -> ThemePackageEditor {
        let manifest = ThemeManifest::parse_json(include_str!(
            "../../../themes/doofusdock-workbench/theme.json"
        ))
        .unwrap();
        let mut editor = ThemePackageEditor::new(manifest).unwrap();
        editor
            .set_base_token("color.accent", ThemeValue::Color(accent.into()))
            .unwrap();
        editor
    }

    fn replace_atomically(path: &Path, editor: &ThemePackageEditor) {
        let temp = path.with_extension("reload-tmp");
        editor.clone().save(&temp).unwrap();
        fs::rename(temp, path).unwrap();
    }

    #[test]
    fn changed_package_is_staged_until_activation_and_can_be_reverted() {
        let path = temp_path();
        editor_with_accent("#F28C28").save(&path).unwrap();
        let mut watcher = ThemeSourceWatcher::new(&path).unwrap();
        watcher.set_enabled(true);
        let mut active = ThemePackageEditor::load(&path).unwrap();
        replace_atomically(&path, &editor_with_accent("#00AA44"));
        assert!(watcher.poll(Instant::now() + Duration::from_secs(1)));
        assert_eq!(
            active.manifest().tokens["color.accent"],
            ThemeValue::Color("#F28C28".into())
        );
        assert_eq!(
            watcher.candidate().unwrap().manifest().tokens["color.accent"],
            ThemeValue::Color("#00AA44".into())
        );
        assert!(watcher.activate(&mut active));
        assert_eq!(
            active.manifest().tokens["color.accent"],
            ThemeValue::Color("#00AA44".into())
        );
        assert!(watcher.revert_activation(&mut active));
        assert_eq!(
            active.manifest().tokens["color.accent"],
            ThemeValue::Color("#F28C28".into())
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_reload_keeps_active_package_and_surfaces_error() {
        let path = temp_path();
        editor_with_accent("#F28C28").save(&path).unwrap();
        let mut watcher = ThemeSourceWatcher::new(&path).unwrap();
        watcher.set_enabled(true);
        let active = ThemePackageEditor::load(&path).unwrap();
        let temp = path.with_extension("reload-tmp");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .unwrap();
        file.write_all(b"not a theme").unwrap();
        file.sync_all().unwrap();
        drop(file);
        fs::rename(temp, &path).unwrap();
        assert!(!watcher.poll(Instant::now() + Duration::from_secs(1)));
        assert!(watcher.candidate().is_none());
        assert!(watcher.error().is_some());
        assert!(!watcher.poll(Instant::now() + Duration::from_millis(1500)));
        assert!(
            watcher.error().is_some(),
            "parse errors remain visible until source changes"
        );
        assert_eq!(
            active.manifest().tokens["color.accent"],
            ThemeValue::Color("#F28C28".into())
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn temporary_source_access_error_clears_when_same_bytes_return() {
        let path = temp_path();
        editor_with_accent("#F28C28").save(&path).unwrap();
        let original = fs::read(&path).unwrap();
        let mut watcher = ThemeSourceWatcher::new(&path).unwrap();
        watcher.set_enabled(true);
        let first = Instant::now() + Duration::from_secs(1);
        assert!(!watcher.poll(first));

        fs::remove_file(&path).unwrap();
        assert!(!watcher.poll(first + THEME_SOURCE_POLL_INTERVAL));
        assert!(watcher.error().is_some());

        let temp = path.with_extension("restore-tmp");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .unwrap();
        file.write_all(&original).unwrap();
        file.sync_all().unwrap();
        drop(file);
        fs::rename(temp, &path).unwrap();

        assert!(!watcher.poll(first + THEME_SOURCE_POLL_INTERVAL * 2));
        assert!(watcher.error().is_none());
        assert!(watcher.candidate().is_none());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn watcher_is_opt_in_and_respects_poll_interval() {
        let path = temp_path();
        editor_with_accent("#F28C28").save(&path).unwrap();
        let mut watcher = ThemeSourceWatcher::new(&path).unwrap();
        let first = Instant::now() + Duration::from_secs(1);
        replace_atomically(&path, &editor_with_accent("#00AA44"));
        assert!(!watcher.poll(first));
        assert!(watcher.candidate().is_none());

        watcher.set_enabled(true);
        assert!(watcher.poll(first));
        watcher.discard_candidate();
        replace_atomically(&path, &editor_with_accent("#3366CC"));
        assert!(!watcher.poll(first + Duration::from_millis(499)));
        assert!(watcher.poll(first + Duration::from_millis(500)));
        assert_eq!(
            watcher.candidate().unwrap().manifest().tokens["color.accent"],
            ThemeValue::Color("#3366CC".into())
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn watcher_rejects_sources_over_the_size_limit() {
        let path = temp_path();
        fs::write(&path, vec![b' '; MAX_THEME_SOURCE_BYTES as usize + 1]).unwrap();
        let error = ThemeSourceWatcher::new(&path).unwrap_err();
        assert!(error.contains("1 MiB limit"));
        let _ = fs::remove_file(path);
    }
}
