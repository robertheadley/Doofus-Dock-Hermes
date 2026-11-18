use crate::*;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[cfg(windows)]
pub(crate) fn atomic_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    const REPLACE_EXISTING: u32 = 0x1;
    const WRITE_THROUGH: u32 = 0x8;
    let ok = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), REPLACE_EXISTING | WRITE_THROUGH) };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
pub(crate) fn atomic_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy, Debug)]
pub struct PersistenceLimits {
    pub maximum_bytes: usize,
    pub maximum_nodes: usize,
    pub maximum_depth: usize,
    pub maximum_string_length: usize,
    pub maximum_windows: usize,
}
impl Default for PersistenceLimits {
    fn default() -> Self {
        Self {
            maximum_bytes: 8 * 1024 * 1024,
            maximum_nodes: 10_000,
            maximum_depth: 128,
            maximum_string_length: 4096,
            maximum_windows: 64,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistenceState {
    Clean,
    Dirty,
    Failed,
}
#[derive(Clone, Debug)]
pub struct PersistenceStatus {
    pub state: PersistenceState,
    pub durable_revision: u64,
    pub active_revision: u64,
    pub last_error: String,
}
impl Default for PersistenceStatus {
    fn default() -> Self {
        Self {
            state: PersistenceState::Clean,
            durable_revision: 0,
            active_revision: 0,
            last_error: String::new(),
        }
    }
}
fn checksum(value: &Value) -> DockResult<String> {
    let mut root = value.clone();
    if let Value::Object(map) = &mut root {
        map.remove("checksum");
    } else {
        return Err(DockError::new(
            ErrorCode::SerializationError,
            "Layout root must be an object.",
            "json",
        ));
    }
    let bytes = serde_json::to_vec(&root)
        .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "json"))?;
    let mut h = 14695981039346656037u64;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211)
    }
    Ok(format!("fnv1a64:{h:016x}"))
}
pub fn checksum_layout_json(text: &str) -> DockResult<String> {
    let v: Value = serde_json::from_str(text)
        .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "json"))?;
    checksum(&v)
}
pub fn serialize_layout(doc: &LayoutDocument) -> DockResult<String> {
    validate_layout(doc)?;
    let mut ordered = doc.clone();
    std::sync::Arc::make_mut(&mut ordered.panels).sort_by(|a, b| a.id.cmp(&b.id));
    ordered.closed_panels.sort();
    ordered
        .auto_hide_panels
        .sort_by(|a, b| a.panel.cmp(&b.panel));
    let mut value = serde_json::to_value(ordered)
        .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "serialize"))?;
    let sum = checksum(&value)?;
    value
        .as_object_mut()
        .unwrap()
        .insert("checksum".into(), Value::String(sum));
    serde_json::to_string(&value)
        .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "serialize"))
}
fn transform_unknown(n: &mut NodePtr, known: &std::collections::HashSet<PanelId>) {
    let node = std::sync::Arc::make_mut(n);
    match &mut node.kind {
        NodeKind::Panel { panel } if !known.contains(panel) => {
            let p = panel.clone();
            node.kind = NodeKind::Placeholder {
                panel: p.clone(),
                title: format!("Unavailable panel: {}", p.0),
            }
        }
        NodeKind::Tabs { items, .. } => items.iter_mut().for_each(|x| transform_unknown(x, known)),
        NodeKind::DocumentWell { groups, .. } => {
            groups.iter_mut().for_each(|x| transform_unknown(x, known))
        }
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => children
            .iter_mut()
            .for_each(|x| transform_unknown(x, known)),
        _ => (),
    }
}
fn check_bounds(
    n: &Node,
    depth: usize,
    count: &mut usize,
    limits: PersistenceLimits,
) -> DockResult<()> {
    *count += 1;
    if depth > limits.maximum_depth || *count > limits.maximum_nodes {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Layout exceeds configured node or depth limit.",
            "tree",
        ));
    }
    if n.id.0.len() > limits.maximum_string_length {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Node ID exceeds configured string limit.",
            n.id.0.clone(),
        ));
    }
    match &n.kind {
        NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. }
            if panel.0.len() > limits.maximum_string_length =>
        {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Panel ID exceeds configured string limit.",
                panel.0.clone(),
            ));
        }
        NodeKind::Placeholder { title, .. } if title.len() > limits.maximum_string_length => {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Placeholder title exceeds configured string limit.",
                n.id.0.clone(),
            ));
        }
        NodeKind::Extension {
            modality, payload, ..
        } if modality.0.len() > limits.maximum_string_length
            || payload.len() > limits.maximum_string_length =>
        {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Extension data exceeds configured string limit.",
                n.id.0.clone(),
            ));
        }
        NodeKind::Tabs { items, .. } => {
            for x in items {
                check_bounds(x, depth + 1, count, limits)?
            }
        }
        NodeKind::DocumentWell { groups, .. } => {
            for x in groups {
                check_bounds(x, depth + 1, count, limits)?
            }
        }
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            for x in children {
                check_bounds(x, depth + 1, count, limits)?
            }
        }
        _ => (),
    }
    Ok(())
}
fn check_doc_bounds(doc: &LayoutDocument, limits: PersistenceLimits) -> DockResult<()> {
    let check = |value: &str, context: String| -> DockResult<()> {
        if value.len() > limits.maximum_string_length {
            Err(DockError::new(
                ErrorCode::ResourceLimit,
                "String exceeds configured limit.",
                context,
            ))
        } else {
            Ok(())
        }
    };
    check(&doc.id.0, "layout_id".into())?;
    check(&doc.workspace.0, "workspace_id".into())?;
    for panel in doc.panels.iter() {
        check(&panel.id.0, "panel_id".into())?;
        check(&panel.title_key, panel.id.0.clone())?;
        check(&panel.icon_key, panel.id.0.clone())?;
        check(&panel.provider_key, panel.id.0.clone())?;
    }
    for panel in &doc.closed_panels {
        check(&panel.0, "closed_panels".into())?;
    }
    for item in &doc.auto_hide_panels {
        check(&item.panel.0, "auto_hide_panels".into())?;
    }
    for window in &doc.floating_windows {
        check(&window.id.0, "window_id".into())?;
        check(&window.monitor_id, window.id.0.clone())?;
    }
    let mut count = 0;
    if let Some(root) = &doc.root {
        check_bounds(root, 0, &mut count, limits)?;
    }
    for window in &doc.floating_windows {
        check_bounds(&window.root, 0, &mut count, limits)?;
    }
    Ok(())
}
pub fn deserialize_layout(text: &str, limits: PersistenceLimits) -> DockResult<LayoutDocument> {
    if text.len() > limits.maximum_bytes {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Layout exceeds configured byte limit.",
            "json",
        ));
    }
    let mut value: Value = serde_json::from_str(text)
        .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "json"))?;
    let expected = value
        .get("checksum")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::SerializationError,
                "Layout checksum is missing.",
                "checksum",
            )
        })?;
    if checksum(&value)? != expected {
        return Err(DockError::new(
            ErrorCode::SerializationError,
            "Layout checksum does not match.",
            "checksum",
        ));
    }
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::SchemaError,
                "Schema version is missing.",
                "schema_version",
            )
        })?;
    if version > 1 {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Layout schema is newer than supported.",
            version.to_string(),
        ));
    }
    if version == 0 {
        value["schema_version"] = Value::from(1);
        if let Value::Object(ref mut o) = value {
            o.entry("closed_panels").or_insert(Value::Array(vec![]));
            o.entry("auto_hide_panels").or_insert(Value::Array(vec![]));
            o.entry("maximized_panel").or_insert(Value::Null);
        }
    }
    let mut doc: LayoutDocument = serde_json::from_value(value)
        .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "document"))?;
    if doc.floating_windows.len() > limits.maximum_windows {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Too many floating windows.",
            "windows",
        ));
    }
    let known: std::collections::HashSet<_> = doc.panels.iter().map(|p| p.id.clone()).collect();
    if let Some(root) = &mut doc.root {
        transform_unknown(root, &known);
    }
    for window in &mut doc.floating_windows {
        transform_unknown(&mut window.root, &known);
    }
    check_doc_bounds(&doc, limits)?;
    validate_layout(&doc)?;
    Ok(doc)
}
#[derive(Clone, Debug)]
pub struct LayoutStore {
    directory: PathBuf,
    limits: PersistenceLimits,
    status: PersistenceStatus,
}
impl LayoutStore {
    pub fn new(path: impl Into<PathBuf>, limits: PersistenceLimits) -> Self {
        Self {
            directory: path.into(),
            limits,
            status: PersistenceStatus::default(),
        }
    }
    pub fn status(&self) -> &PersistenceStatus {
        &self.status
    }
    fn path_for(&self, id: &LayoutId) -> DockResult<PathBuf> {
        if id.is_empty()
            || !id
                .0
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Unsafe layout ID.",
                id.0.clone(),
            ));
        }
        Ok(self.directory.join(format!("{}.json", id.0)))
    }
    pub fn save(&mut self, doc: &LayoutDocument) -> DockResult<()> {
        self.save_with_replace(doc, atomic_replace)
    }

    fn save_with_replace(
        &mut self,
        doc: &LayoutDocument,
        replace: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    ) -> DockResult<()> {
        let path = self.path_for(&doc.id)?;
        self.status.active_revision = doc.revision;
        let contents = match serialize_layout(doc) {
            Ok(contents) => contents,
            Err(error) => {
                self.status.state = PersistenceState::Dirty;
                self.status.last_error = error.to_string();
                return Err(error);
            }
        };
        let seq = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("json.{}.{}.tmp", std::process::id(), seq));
        let result: DockResult<()> = (|| {
            fs::create_dir_all(&self.directory).map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    e.to_string(),
                    self.directory.display().to_string(),
                )
            })?;
            use std::io::Write;
            let mut f = fs::File::create(&tmp).map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    e.to_string(),
                    tmp.display().to_string(),
                )
            })?;
            f.write_all(contents.as_bytes())
                .and_then(|_| f.sync_all())
                .map_err(|e| {
                    DockError::new(
                        ErrorCode::PersistenceError,
                        e.to_string(),
                        tmp.display().to_string(),
                    )
                })?;
            if path.exists() {
                let backup = path.with_extension("json.bak");
                fs::copy(&path, &backup).map_err(|e| {
                    DockError::new(
                        ErrorCode::PersistenceError,
                        e.to_string(),
                        backup.display().to_string(),
                    )
                })?;
            }
            replace(&tmp, &path).map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
            Ok(())
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            self.status.state = PersistenceState::Dirty;
            self.status.last_error = e.to_string();
            return Err(e);
        }
        self.status.state = PersistenceState::Clean;
        self.status.durable_revision = doc.revision;
        self.status.active_revision = doc.revision;
        self.status.last_error.clear();
        Ok(())
    }
    pub fn load(&self, id: &LayoutId) -> DockResult<LayoutDocument> {
        let p = self.path_for(id)?;
        let bytes = fs::read(&p).map_err(|e| {
            DockError::new(
                ErrorCode::PersistenceError,
                e.to_string(),
                p.display().to_string(),
            )
        })?;
        let text = std::str::from_utf8(&bytes).map_err(|e| {
            DockError::new(
                ErrorCode::SerializationError,
                e.to_string(),
                p.display().to_string(),
            )
        })?;
        let d = deserialize_layout(text, self.limits)?;
        if &d.id != id {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Stored layout ID does not match requested ID.",
                id.0.clone(),
            ));
        }
        Ok(d)
    }
    pub fn recover(&self, id: &LayoutId) -> DockResult<LayoutDocument> {
        match self.load(id) {
            Ok(d) => Ok(d),
            Err(primary) => {
                let p = self.path_for(id)?.with_extension("json.bak");
                let bytes = fs::read(&p).map_err(|e| {
                    DockError::new(
                        ErrorCode::PersistenceError,
                        format!("Primary invalid ({primary}); backup unavailable ({e})."),
                        p.display().to_string(),
                    )
                })?;
                let text = std::str::from_utf8(&bytes).map_err(|e| {
                    DockError::new(
                        ErrorCode::SerializationError,
                        e.to_string(),
                        p.display().to_string(),
                    )
                })?;
                let restored = deserialize_layout(text, self.limits)?;
                if &restored.id != id {
                    return Err(DockError::new(
                        ErrorCode::SchemaError,
                        "Backup layout ID does not match requested ID.",
                        id.0.clone(),
                    ));
                }
                Ok(restored)
            }
        }
    }
    pub fn list_workspace(&self, workspace: &WorkspaceId) -> DockResult<Vec<LayoutDocument>> {
        let mut out = vec![];
        if !self.directory.exists() {
            return Ok(out);
        }
        for e in fs::read_dir(&self.directory).map_err(|e| {
            DockError::new(
                ErrorCode::PersistenceError,
                e.to_string(),
                self.directory.display().to_string(),
            )
        })? {
            let p = e
                .map_err(|e| {
                    DockError::new(ErrorCode::PersistenceError, e.to_string(), "directory")
                })?
                .path();
            if p.extension().and_then(|x| x.to_str()) == Some("json")
                && let Some(s) = p.file_stem().and_then(|x| x.to_str())
                && let Ok(d) = self.load(&LayoutId(s.into()))
                && &d.workspace == workspace
            {
                out.push(d)
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use std::{
        fs,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    };

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> LayoutDocument {
        let mut document = LayoutDocument::new("recovery-layout", "recovery-workspace");
        Arc::make_mut(&mut document.panels).extend([
            PanelDescriptor::new("alpha", "Alpha"),
            PanelDescriptor::new("beta", "Beta"),
        ]);
        document.root = Some(Node::tabs(
            "tabs",
            vec![
                Node::panel("alpha-node", "alpha"),
                Node::panel("beta-node", "beta"),
            ],
            0,
        ));
        document
    }

    fn temp_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "doofusdock-persistence-replace-test-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn replacement_failure_after_backup_copy_preserves_durable_primary() {
        let directory = temp_directory();
        let mut store = LayoutStore::new(&directory, PersistenceLimits::default());
        let original = fixture();
        store.save(&original).unwrap();
        let primary = directory.join("recovery-layout.json");
        let original_bytes = fs::read(&primary).unwrap();

        let mut changed = original.clone();
        let mut history = UndoStack::new(4);
        history
            .execute(
                &mut changed,
                Operation::SelectTab {
                    tabs: "tabs".into(),
                    index: 1,
                },
            )
            .unwrap();

        let error = store
            .save_with_replace(&changed, |temporary, destination| {
                let backup = destination.with_extension("json.bak");
                assert!(
                    temporary.exists(),
                    "new document is flushed before replacement"
                );
                assert_eq!(fs::read(&backup).unwrap(), original_bytes);
                assert_eq!(fs::read(destination).unwrap(), original_bytes);
                Err(std::io::Error::other("injected atomic replacement failure"))
            })
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::PersistenceError);
        assert_eq!(fs::read(&primary).unwrap(), original_bytes);
        assert_eq!(store.load(&original.id).unwrap(), original);
        assert_eq!(store.status().active_revision, changed.revision);
        assert_eq!(store.status().durable_revision, original.revision);
        assert_eq!(store.status().state, PersistenceState::Dirty);
        assert!(!store.status().last_error.is_empty());
        assert!(fs::read_dir(&directory).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        }));

        fs::write(&primary, b"simulated torn primary write").unwrap();
        assert_eq!(store.recover(&original.id).unwrap(), original);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preference_replacement_failure_preserves_and_recovers_previous_checksummed_value() {
        let directory = temp_directory();
        let store = WorkspacePreferencesStore::new(&directory, PersistenceLimits::default());
        let workspace = WorkspaceId::from("recovery-workspace");
        let original = serde_json::json!({"theme":"light","pinned":["alpha"]});
        store.save(&workspace, &original).unwrap();
        let primary = directory.join("recovery-workspace.json");
        let original_bytes = fs::read(&primary).unwrap();
        let replacement = serde_json::json!({"theme":"dark","pinned":["beta"]});

        let error = store
            .save_with_replace(&workspace, &replacement, |temporary, destination| {
                assert!(
                    temporary.exists(),
                    "new preferences are flushed before replace"
                );
                assert_eq!(fs::read(destination).unwrap(), original_bytes);
                Err(std::io::Error::other(
                    "injected preference replacement failure",
                ))
            })
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::PersistenceError);
        assert_eq!(fs::read(&primary).unwrap(), original_bytes);
        let backup = directory.join("recovery-workspace.json.bak");
        assert_eq!(fs::read(&backup).unwrap(), original_bytes);
        assert_eq!(store.load(&workspace).unwrap(), original);
        assert!(fs::read_dir(&directory).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        }));

        fs::write(&primary, b"simulated torn preferences write").unwrap();
        assert_eq!(store.recover(&workspace).unwrap(), original);
        assert_eq!(store.load(&workspace).unwrap(), original);
        assert_eq!(fs::read(&backup).unwrap(), original_bytes);
        assert_eq!(fs::read(&primary).unwrap(), original_bytes);

        fs::remove_dir_all(directory).unwrap();
    }
}

fn check_preference_bounds(
    value: &Value,
    limits: PersistenceLimits,
    depth: usize,
    count: &mut usize,
) -> DockResult<()> {
    *count += 1;
    if depth > limits.maximum_depth || *count > limits.maximum_nodes {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Preferences exceed configured tree bounds.",
            "preferences",
        ));
    }
    match value {
        Value::String(value) if value.len() > limits.maximum_string_length => Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Preference string exceeds configured limit.",
            "preferences",
        )),
        Value::Array(items) => {
            for item in items {
                check_preference_bounds(item, limits, depth + 1, count)?;
            }
            Ok(())
        }
        Value::Object(items) => {
            for (key, value) in items {
                if key.len() > limits.maximum_string_length {
                    return Err(DockError::new(
                        ErrorCode::ResourceLimit,
                        "Preference key exceeds configured limit.",
                        "preferences",
                    ));
                }
                check_preference_bounds(value, limits, depth + 1, count)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
#[derive(Clone, Debug)]
pub struct WorkspacePreferencesStore {
    directory: PathBuf,
    limits: PersistenceLimits,
}
impl WorkspacePreferencesStore {
    pub fn new(path: impl Into<PathBuf>, limits: PersistenceLimits) -> Self {
        Self {
            directory: path.into(),
            limits,
        }
    }
    fn path_for(&self, workspace: &WorkspaceId) -> DockResult<PathBuf> {
        if workspace.is_empty()
            || !workspace
                .0
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Unsafe workspace ID.",
                workspace.0.clone(),
            ));
        }
        Ok(self.directory.join(format!("{}.json", workspace.0)))
    }
    pub fn save(&self, workspace: &WorkspaceId, preferences: &Value) -> DockResult<()> {
        self.save_with_replace(workspace, preferences, atomic_replace)
    }

    fn save_with_replace(
        &self,
        workspace: &WorkspaceId,
        preferences: &Value,
        replace: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    ) -> DockResult<()> {
        let path = self.path_for(workspace)?;
        let mut count = 0;
        check_preference_bounds(preferences, self.limits, 0, &mut count)?;
        let mut value = serde_json::json!({ "schema_version": 1, "workspace_id": workspace.0, "preferences": preferences });
        let sum = checksum(&value)?;
        value
            .as_object_mut()
            .unwrap()
            .insert("checksum".into(), Value::String(sum));
        let bytes = serde_json::to_vec(&value).map_err(|e| {
            DockError::new(ErrorCode::SerializationError, e.to_string(), "preferences")
        })?;
        if bytes.len() > self.limits.maximum_bytes {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Workspace preferences exceed configured byte limit.",
                "preferences",
            ));
        }
        fs::create_dir_all(&self.directory).map_err(|e| {
            DockError::new(
                ErrorCode::PersistenceError,
                e.to_string(),
                self.directory.display().to_string(),
            )
        })?;
        let seq = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("json.{}.{}.tmp", std::process::id(), seq));
        let result = (|| {
            use std::io::Write;
            let mut file = fs::File::create(&tmp).map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    e.to_string(),
                    tmp.display().to_string(),
                )
            })?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| {
                    DockError::new(
                        ErrorCode::PersistenceError,
                        e.to_string(),
                        tmp.display().to_string(),
                    )
                })?;
            // Only replace the recovery copy with a previously checksummed,
            // bounded value. A corrupt primary must never poison a good backup.
            if path.exists() && self.load(workspace).is_ok() {
                let backup = path.with_extension("json.bak");
                let backup_tmp = path.with_extension(format!(
                    "json.bak.{}.{}.tmp",
                    std::process::id(),
                    TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ));
                let preserve_result = (|| {
                    fs::copy(&path, &backup_tmp).map_err(|e| {
                        DockError::new(
                            ErrorCode::PersistenceError,
                            format!("Could not stage known-good preferences backup: {e}"),
                            backup_tmp.display().to_string(),
                        )
                    })?;
                    self.load_from_path(workspace, &backup_tmp)?;
                    atomic_replace(&backup_tmp, &backup).map_err(|e| {
                        DockError::new(
                            ErrorCode::PersistenceError,
                            format!("Could not atomically preserve known-good preferences: {e}"),
                            backup.display().to_string(),
                        )
                    })
                })();
                if preserve_result.is_err() {
                    let _ = fs::remove_file(&backup_tmp);
                }
                preserve_result?;
            }
            replace(&tmp, &path).map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }
    pub fn load(&self, workspace: &WorkspaceId) -> DockResult<Value> {
        let path = self.path_for(workspace)?;
        self.load_from_path(workspace, &path)
    }
    fn load_from_path(&self, workspace: &WorkspaceId, path: &Path) -> DockResult<Value> {
        let bytes = fs::read(path).map_err(|e| {
            DockError::new(
                ErrorCode::PersistenceError,
                e.to_string(),
                path.display().to_string(),
            )
        })?;
        if bytes.len() > self.limits.maximum_bytes {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Workspace preferences exceed configured byte limit.",
                "preferences",
            ));
        }
        let mut value: Value = serde_json::from_slice(&bytes).map_err(|e| {
            DockError::new(ErrorCode::SerializationError, e.to_string(), "preferences")
        })?;
        let expected = value
            .get("checksum")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::SerializationError,
                    "Preferences checksum is missing.",
                    "checksum",
                )
            })?
            .to_owned();
        if checksum(&value)? != expected {
            return Err(DockError::new(
                ErrorCode::SerializationError,
                "Preferences checksum does not match.",
                "checksum",
            ));
        }
        if value.get("schema_version").and_then(Value::as_u64) != Some(1)
            || value.get("workspace_id").and_then(Value::as_str) != Some(workspace.0.as_str())
        {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Preferences schema or workspace ID is invalid.",
                workspace.0.clone(),
            ));
        }
        let prefs = value
            .as_object_mut()
            .and_then(|o| o.remove("preferences"))
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::SchemaError,
                    "Preferences payload is missing.",
                    "preferences",
                )
            })?;
        let mut count = 0;
        check_preference_bounds(&prefs, self.limits, 0, &mut count)?;
        Ok(prefs)
    }
    /// Restore the last known-good checksummed preferences after corruption.
    /// The backup is validated before it is atomically written back to primary.
    pub fn recover(&self, workspace: &WorkspaceId) -> DockResult<Value> {
        let primary = self.path_for(workspace)?;
        match self.load_from_path(workspace, &primary) {
            Ok(value) => Ok(value),
            Err(primary_error) => {
                let backup = primary.with_extension("json.bak");
                let value = self.load_from_path(workspace, &backup).map_err(|backup_error| {
                    DockError::new(
                        ErrorCode::PersistenceError,
                        format!(
                            "Primary preferences are invalid ({primary_error}); known-good backup is unavailable or invalid ({backup_error})."
                        ),
                        primary.display().to_string(),
                    )
                })?;
                self.save(workspace, &value)?;
                Ok(value)
            }
        }
    }
}
