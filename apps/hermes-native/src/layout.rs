use doofusdock_core::{
    ClosePolicy, CommandId, DockingService, LayoutDocument, LayoutId, LayoutStore, Node, NodeId,
    NodeKind, Operation, Orientation, PanelDescriptor, PanelId, PersistenceLimits,
    PersistencePolicy, ProfileLayout, ProfilePresentation, ProfileRegion, WorkspaceProfile,
    apply_workspace_profile,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub const SESSIONS: &str = "sessions";
pub const BOTS: &str = "hermes-bots:pane";
pub const WORKSPACE: &str = "workspace";
pub const REVIEW: &str = "review";
pub const FILES: &str = "files";
pub const TERMINAL: &str = "terminal";
pub const SESSION_COLOR_HUES: [u16; 12] = [0, 30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceMode {
    #[default]
    Advanced,
    Simple,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayConnectionKind {
    Local,
    Remote,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConnectionDescriptor {
    pub connection_id: String,
    pub kind: GatewayConnectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
    pub display_label: String,
}

impl GatewayConnectionDescriptor {
    pub fn local() -> Self {
        Self {
            connection_id: LOCAL_GATEWAY_CONNECTION_ID.into(),
            kind: GatewayConnectionKind::Local,
            endpoint_url: None,
            display_label: "Local Hermes".into(),
        }
    }

    pub fn remote(endpoint_url: &str) -> Result<Self, String> {
        let endpoint_url = crate::remote_probe::normalize_remote_base_url(endpoint_url)?;
        let host = url::Url::parse(&endpoint_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_else(|| endpoint_url.clone());
        Ok(Self {
            connection_id: remote_gateway_connection_id(&endpoint_url),
            kind: GatewayConnectionKind::Remote,
            endpoint_url: Some(endpoint_url),
            display_label: format!("Hermes · {host}"),
        })
    }
}

const LOCAL_GATEWAY_CONNECTION_ID: &str = "local";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PhysicalSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
    pub mode: InterfaceMode,
    pub advanced_profile: String,
    pub simple_sidebar_right: bool,
    #[serde(default = "default_true")]
    pub sessions_sidebar_open: bool,
    #[serde(default)]
    collapsed_sides: BTreeSet<PhysicalSide>,
    #[serde(default)]
    pub file_browser_open: bool,
    pub show_all_project_sessions: bool,
    pub dismissed_auto_project_ids: Vec<String>,
    pub pane_open: BTreeMap<String, bool>,
    pub show_ignored_files_roots: BTreeSet<String>,
    pub pinned_session_order: BTreeMap<String, Vec<String>>,
    pub sidebar_session_order: BTreeMap<String, Vec<String>>,
    pub session_color_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub master_detail_width_overrides: BTreeMap<String, f32>,
    #[serde(default)]
    pub last_remote_gateway_url: Option<String>,
    #[serde(default)]
    pub gateway_connections: Vec<GatewayConnectionDescriptor>,
    #[serde(default)]
    pub selected_gateway_connection_id: String,
    #[serde(default)]
    pub user_layout_presets: BTreeMap<String, UserLayoutPreset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserLayoutPreset {
    pub name: String,
    pub document: LayoutDocument,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            mode: InterfaceMode::Advanced,
            advanced_profile: "default".into(),
            simple_sidebar_right: false,
            sessions_sidebar_open: true,
            collapsed_sides: BTreeSet::new(),
            file_browser_open: false,
            show_all_project_sessions: false,
            dismissed_auto_project_ids: Vec::new(),
            pane_open: BTreeMap::from([
                (BOTS.into(), false),
                (REVIEW.into(), false),
                (FILES.into(), false),
                (TERMINAL.into(), false),
            ]),
            show_ignored_files_roots: BTreeSet::new(),
            pinned_session_order: BTreeMap::new(),
            sidebar_session_order: BTreeMap::new(),
            session_color_overrides: BTreeMap::new(),
            master_detail_width_overrides: BTreeMap::new(),
            last_remote_gateway_url: None,
            gateway_connections: vec![GatewayConnectionDescriptor::local()],
            selected_gateway_connection_id: LOCAL_GATEWAY_CONNECTION_ID.into(),
            user_layout_presets: BTreeMap::new(),
        }
    }
}

pub struct Workspace {
    pub dock: DockingService,
    store: LayoutStore,
    pub preferences: UiPreferences,
    pub persistence_error: Option<String>,
    dismissed_auto_project_ids_path: PathBuf,
    profiles: Vec<WorkspaceProfile>,
    session_reveals: BTreeMap<String, bool>,
    closed_session_panes: BTreeMap<String, VecDeque<ClosedSessionPane>>,
    data_directory: PathBuf,
    preferences_path: PathBuf,
}

const CLOSED_SESSION_PANE_LIMIT: usize = 64;
const AUTO_PROJECT_DISMISSAL_LIMIT: usize = 4096;
const AUTO_PROJECT_STATE_BYTES_LIMIT: u64 = 1024 * 1024;
const AUTO_PROJECT_LOCK_TTL: std::time::Duration = std::time::Duration::from_secs(10);
const AUTO_PROJECT_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(2);
static AUTO_PROJECT_STATE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AutoProjectDismissalState {
    version: u8,
    ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AutoProjectDismissalLock {
    version: u8,
    process_id: u32,
    expires_unix_ms: u128,
}

struct AutoProjectDismissalLockGuard {
    path: PathBuf,
}

impl Drop for AutoProjectDismissalLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn layout_command(kind: &str, revision: u64) -> CommandId {
    CommandId::from(format!("hermes-native-{kind}-{revision}"))
}

#[derive(Clone, Debug)]
struct ClosedSessionPane {
    panel: PanelId,
    session_id: String,
    profile: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoredSessionPane {
    pub panel: PanelId,
    pub session_id: String,
    pub profile: Option<String>,
}

impl Workspace {
    pub fn data_directory(&self) -> &Path {
        &self.data_directory
    }

    pub fn shows_ignored_files(&self, root: &Path) -> bool {
        self.preferences
            .show_ignored_files_roots
            .contains(&file_preference_path_key(root))
    }

    pub fn set_show_ignored_files(&mut self, root: &Path, show: bool) -> Result<bool, String> {
        if !root.is_absolute() {
            return Err("Files preference requires an absolute project root".into());
        }
        let key = file_preference_path_key(root);
        let changed = if show {
            self.preferences.show_ignored_files_roots.insert(key)
        } else {
            self.preferences.show_ignored_files_roots.remove(&key)
        };
        if changed {
            match write_preferences_atomic(&self.preferences_path, &self.preferences) {
                Ok(()) => self.persistence_error = None,
                Err(error) => {
                    if show {
                        self.preferences
                            .show_ignored_files_roots
                            .remove(&file_preference_path_key(root));
                    } else {
                        self.preferences
                            .show_ignored_files_roots
                            .insert(file_preference_path_key(root));
                    }
                    self.persistence_error = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(changed)
    }

    pub fn load_fallback() -> Self {
        Self::load_fallback_at(std::env::temp_dir().join("HermesNative-recovery"))
    }

    #[cfg(feature = "agent-automation")]
    pub fn load_fallback_isolated(directory: PathBuf) -> Self {
        Self::load_fallback_at(directory)
    }

    fn load_fallback_at(directory: PathBuf) -> Self {
        let dock =
            DockingService::new(base_document(), 128).expect("the built-in Hermes layout is valid");
        Self {
            dock,
            store: LayoutStore::new(
                directory.clone().join("layouts"),
                PersistenceLimits::default(),
            ),
            preferences: UiPreferences::default(),
            persistence_error: None,
            dismissed_auto_project_ids_path: directory.join("dismissed-auto-projects.json"),
            profiles: source_profiles(),
            session_reveals: BTreeMap::new(),
            closed_session_panes: BTreeMap::new(),
            preferences_path: directory.join("ui-preferences.json"),
            data_directory: directory,
        }
    }

    pub fn load() -> Result<Self, String> {
        let directory = data_directory();
        Self::load_from_with(
            directory.clone(),
            LayoutId::from("hermes-native-main"),
            directory.join("ui-preferences.json"),
        )
    }

    /// Give each native window an independent layout and UI preference file,
    /// while sharing the profile directory used for gateway credentials.
    pub fn load_window(window_id: u64) -> Result<Self, String> {
        Self::load_window_from(data_directory(), window_id)
    }

    fn load_window_from(directory: PathBuf, window_id: u64) -> Result<Self, String> {
        if window_id == 0 {
            return Err("Native window ID must be greater than zero".into());
        }
        let window_directory = directory.join("windows").join(window_id.to_string());
        fs::create_dir_all(&window_directory).map_err(|error| error.to_string())?;
        let preferences_path = window_directory.join("ui-preferences.json");
        if !preferences_path.exists() {
            let initial =
                read_preferences(&directory.join("ui-preferences.json")).unwrap_or_default();
            write_preferences_atomic(&preferences_path, &initial)?;
        }
        Self::load_from_with(
            directory.clone(),
            LayoutId::from(format!("hermes-native-window-{window_id}")),
            preferences_path,
        )
    }

    #[cfg(feature = "agent-automation")]
    pub fn load_isolated(directory: PathBuf) -> Result<Self, String> {
        Self::load_from(directory)
    }

    fn load_from(directory: PathBuf) -> Result<Self, String> {
        Self::load_from_with(
            directory.clone(),
            LayoutId::from("hermes-native-main"),
            directory.join("ui-preferences.json"),
        )
    }

    fn load_from_with(
        directory: PathBuf,
        layout_id: LayoutId,
        preferences_path: PathBuf,
    ) -> Result<Self, String> {
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let store = LayoutStore::new(directory.join("layouts"), PersistenceLimits::default());
        let selected_connection_id_was_persisted = fs::read(&preferences_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|value| value.get("selected_gateway_connection_id").is_some());
        let mut preferences = read_preferences(&preferences_path).unwrap_or_default();
        if migrate_gateway_connection_preferences(
            &mut preferences,
            selected_connection_id_was_persisted,
        ) {
            write_preferences_atomic(&preferences_path, &preferences)?;
        }
        let dismissed_auto_project_ids_path = directory.join("dismissed-auto-projects.json");
        match read_auto_project_dismissal_state(&dismissed_auto_project_ids_path)? {
            Some(ids) => preferences.dismissed_auto_project_ids = ids,
            None => {
                let seed_ids = if preferences_path == directory.join("ui-preferences.json") {
                    preferences.dismissed_auto_project_ids.clone()
                } else {
                    read_preferences(&directory.join("ui-preferences.json"))
                        .map(|preferences| preferences.dismissed_auto_project_ids)
                        .unwrap_or_default()
                };
                if !seed_ids.is_empty() {
                    let _lock =
                        acquire_auto_project_dismissal_lock(&dismissed_auto_project_ids_path)?;
                    if let Some(ids) =
                        read_auto_project_dismissal_state(&dismissed_auto_project_ids_path)?
                    {
                        preferences.dismissed_auto_project_ids = ids;
                    } else {
                        write_auto_project_dismissal_state(
                            &dismissed_auto_project_ids_path,
                            &seed_ids,
                        )?;
                        preferences.dismissed_auto_project_ids = seed_ids;
                    }
                }
            }
        }
        let profiles = source_profiles();
        if preferences.mode == InterfaceMode::Simple {
            preferences.simple_sidebar_right = preferences.advanced_profile == "sidebar-right";
        }

        let loaded = store.load(&layout_id).ok();
        let is_new = loaded.is_none();
        let mut document = loaded.unwrap_or_else(base_document);
        document.id = layout_id.clone();
        let mut dock = DockingService::new(document, 128).map_err(|error| error.to_string())?;

        if is_new {
            let initial_profile = active_profile_id(&preferences);
            apply_profile_to_service(
                &mut dock,
                profile(&profiles, initial_profile)?,
                &preferences,
                &BTreeMap::new(),
            )?;
        }
        let mut workspace = Self {
            data_directory: directory.clone(),
            dismissed_auto_project_ids_path,
            dock,
            store,
            preferences,
            persistence_error: None,
            profiles,
            session_reveals: BTreeMap::new(),
            closed_session_panes: BTreeMap::new(),
            preferences_path,
        };
        workspace.save_layout();
        Ok(workspace)
    }

    /// Add or front a stored Hermes session as a dynamic pane in the canonical
    /// DoofusDock layout. Pane identity includes both profile and stored ID.
    pub fn open_session_pane(
        &mut self,
        profile_id: Option<&str>,
        stored_session_id: &str,
        title: &str,
    ) -> Result<PanelId, String> {
        validate_session_pane_identity(profile_id, stored_session_id, title)?;
        self.normalize_nested_workspace_session_tabs()?;
        let panel = session_pane_id(profile_id, stored_session_id);
        let observed = self.dock.snapshot();
        if let Some(descriptor) = observed.document.descriptor(&panel) {
            let expected_profile = profile_id.unwrap_or_default();
            if descriptor
                .metadata
                .get("hermes.session.id")
                .map(String::as_str)
                != Some(stored_session_id)
                || descriptor
                    .metadata
                    .get("hermes.session.profile")
                    .map(String::as_str)
                    != Some(expected_profile)
            {
                return Err("Session pane ID collision failed identity validation".into());
            }
            if observed.document.closed_panels.contains(&panel) {
                let force_open = BTreeSet::from([panel.clone()]);
                let active_profile_id = self.active_profile().to_owned();
                let active_profile = profile(&self.profiles, &active_profile_id)?;
                apply_profile_to_service_with_reopen(
                    &mut self.dock,
                    active_profile,
                    &self.preferences,
                    &self.session_reveals,
                    &force_open,
                )?;
                self.save_layout();
            } else if let Some((tabs, index)) =
                find_tab_for_panel(observed.document.root.as_deref(), &panel)
            {
                self.dock
                    .transact(
                        observed.revision,
                        CommandId::from(format!(
                            "front-session-pane-{}-{}",
                            observed.revision, panel.0
                        )),
                        [Operation::SelectTab { tabs, index }],
                    )
                    .map_err(|error| error.to_string())?;
                self.save_layout();
            }
            return Ok(panel);
        }

        let workspace_target =
            find_tab_for_panel(observed.document.root.as_deref(), &PanelId::from(WORKSPACE))
                .map(|(tabs, _)| tabs)
                .or_else(|| find_panel_node(observed.document.root.as_deref(), WORKSPACE))
                .ok_or_else(|| {
                    "The Hermes workspace pane is not present in the Dock layout".to_owned()
                })?;
        let mut descriptor = PanelDescriptor::new(panel.0.clone(), bounded_session_title(title));
        descriptor.role = "hermes_session_tile".into();
        descriptor.minimum = doofusdock_core::Size::new(180, 140);
        descriptor.close_policy = ClosePolicy::Allowed;
        descriptor
            .metadata
            .insert("hermes.session.tile".into(), "1".into());
        descriptor
            .metadata
            .insert("hermes.session.id".into(), stored_session_id.into());
        descriptor.metadata.insert(
            "hermes.session.profile".into(),
            profile_id.unwrap_or_default().into(),
        );
        self.dock
            .transact(
                observed.revision,
                CommandId::from(format!(
                    "open-session-pane-{}-{}",
                    observed.revision, panel.0
                )),
                [Operation::AddPanel {
                    descriptor,
                    target: Some(workspace_target),
                }],
            )
            .map_err(|error| error.to_string())?;
        self.save_layout();
        Ok(panel)
    }

    fn normalize_nested_workspace_session_tabs(&mut self) -> Result<(), String> {
        let observed = self.dock.snapshot();
        let Some((workspace_tabs, _)) =
            find_tab_for_panel(observed.document.root.as_deref(), &PanelId::from(WORKSPACE))
        else {
            return Ok(());
        };
        let group_path =
            tab_group_path_for_panel(observed.document.root.as_deref(), &PanelId::from(WORKSPACE));
        let Some((_, workspace_group_len)) =
            find_tab_group(observed.document.root.as_deref(), &workspace_tabs)
        else {
            return Ok(());
        };
        let ancestor_groups = group_path
            .iter()
            .take(group_path.len().saturating_sub(1))
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        if ancestor_groups.is_empty() {
            return Ok(());
        }
        let moving = self
            .session_pane_ids()
            .into_iter()
            .filter(|panel| !observed.document.closed_panels.contains(panel))
            .filter(|panel| {
                find_tab_for_panel(observed.document.root.as_deref(), panel)
                    .is_some_and(|(tabs, _)| ancestor_groups.contains(&tabs))
            })
            .collect::<Vec<_>>();
        if moving.is_empty() {
            return Ok(());
        }
        let operations = moving
            .iter()
            .enumerate()
            .map(|(offset, panel)| Operation::TabPanel {
                panel: panel.clone(),
                target_tabs: workspace_tabs.clone(),
                index: workspace_group_len + offset,
            })
            .collect::<Vec<_>>();
        self.dock
            .transact(
                observed.revision,
                CommandId::from(format!(
                    "flatten-session-tabs-{}-{}",
                    observed.revision, workspace_tabs.0
                )),
                operations,
            )
            .map_err(|error| error.to_string())?;
        self.save_layout();
        Ok(())
    }

    /// Close a session pane through the canonical Dock operation. Busy-session
    /// confirmation is owned by the app layer and must happen before this call.
    pub fn close_session_pane(&mut self, panel: &PanelId) -> Result<(), String> {
        let observed = self.dock.snapshot();
        let descriptor = observed
            .document
            .descriptor(panel)
            .ok_or_else(|| "Session pane is not registered".to_owned())?;
        if descriptor.role != "hermes_session_tile" {
            return Err("Refusing to close a non-session pane as a session tile".into());
        }
        if observed.document.closed_panels.contains(panel) {
            return Ok(());
        }
        let session_id = descriptor
            .metadata
            .get("hermes.session.id")
            .cloned()
            .ok_or_else(|| "Session pane is missing its stored session ID".to_owned())?;
        let profile = descriptor
            .metadata
            .get("hermes.session.profile")
            .cloned()
            .filter(|profile| !profile.is_empty());
        self.dock
            .transact(
                observed.revision,
                CommandId::from(format!(
                    "close-session-pane-{}-{}",
                    observed.revision, panel.0
                )),
                [Operation::ClosePanel {
                    panel: panel.clone(),
                }],
            )
            .map_err(|error| error.to_string())?;
        let scope = profile.clone().unwrap_or_default();
        let closed = self.closed_session_panes.entry(scope).or_default();
        closed.push_back(ClosedSessionPane {
            panel: panel.clone(),
            session_id,
            profile,
        });
        while closed.len() > CLOSED_SESSION_PANE_LIMIT {
            closed.pop_front();
        }
        self.save_layout();
        Ok(())
    }

    pub fn open_file_preview(&mut self, path: &Path) -> Result<PanelId, String> {
        if !path.is_absolute() || path.to_string_lossy().len() > 32_768 {
            return Err("File preview path must be absolute and no longer than 32768 bytes".into());
        }
        let path = path.to_string_lossy().into_owned();
        let panel = file_preview_panel_id(&path);
        let observed = self.dock.snapshot();
        if let Some(existing) = observed.document.descriptor(&panel) {
            if existing.role != "hermes_file_preview"
                || existing.metadata.get("hermes.file.path") != Some(&path)
            {
                return Err(
                    "File preview identity collision; refusing to reuse another pane".into(),
                );
            }
            if !observed.document.closed_panels.contains(&panel) {
                if let Some((tabs, index)) =
                    find_tab_for_panel(observed.document.root.as_deref(), &panel)
                {
                    self.dock
                        .transact(
                            observed.revision,
                            layout_command("select-file-preview", observed.revision),
                            [Operation::SelectTab { tabs, index }],
                        )
                        .map_err(|error| error.to_string())?;
                    return Ok(panel);
                }
                return Ok(panel);
            }
            let target_tabs = observed
                .document
                .panels
                .iter()
                .filter(|descriptor| {
                    descriptor.role == "hermes_file_preview"
                        && !observed.document.closed_panels.contains(&descriptor.id)
                })
                .find_map(|descriptor| {
                    find_tab_for_panel(observed.document.root.as_deref(), &descriptor.id)
                        .map(|(tabs, _)| tabs)
                })
                .or_else(|| {
                    find_tab_for_panel(observed.document.root.as_deref(), &PanelId::from(WORKSPACE))
                        .map(|(tabs, _)| tabs)
                })
                .ok_or_else(|| "No tab group is available for the file preview".to_owned())?;
            let index = find_tab_group(observed.document.root.as_deref(), &target_tabs)
                .map_or(0, |(len, _)| len);
            self.dock
                .transact(
                    observed.revision,
                    layout_command("reopen-file-preview", observed.revision),
                    [Operation::ReopenPanel {
                        panel: panel.clone(),
                        target_tabs,
                        index,
                    }],
                )
                .map_err(|error| error.to_string())?;
            self.save_layout();
            return Ok(panel);
        }

        let mut descriptor = PanelDescriptor::new(
            panel.0.clone(),
            Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&path),
        );
        descriptor.role = "hermes_file_preview".into();
        descriptor.persistence = PersistencePolicy::Never;
        descriptor.close_policy = ClosePolicy::Allowed;
        descriptor.minimum = doofusdock_core::Size::new(240, 160);
        descriptor.metadata.insert("hermes.file.path".into(), path);

        let preview_group = observed
            .document
            .panels
            .iter()
            .filter(|descriptor| {
                descriptor.role == "hermes_file_preview"
                    && !observed.document.closed_panels.contains(&descriptor.id)
            })
            .find_map(|descriptor| {
                find_tab_for_panel(observed.document.root.as_deref(), &descriptor.id)
                    .map(|(tabs, _)| tabs)
                    .or_else(|| {
                        find_panel_node(observed.document.root.as_deref(), &descriptor.id.0)
                    })
            });
        let operation = if let Some(target) = preview_group {
            Operation::AddPanel {
                descriptor,
                target: Some(target),
            }
        } else {
            let target =
                find_panel_node(observed.document.root.as_deref(), WORKSPACE).ok_or_else(|| {
                    "The Hermes workspace pane is not present in the Dock layout".to_owned()
                })?;
            Operation::AddGroup {
                descriptor,
                target: Some(target),
                orientation: Orientation::Horizontal,
                before: false,
                ratio: 0.38,
            }
        };
        self.dock
            .transact(
                observed.revision,
                layout_command("open-file-preview", observed.revision),
                [operation],
            )
            .map_err(|error| error.to_string())?;
        Ok(panel)
    }

    pub fn close_file_preview(&mut self, panel: &PanelId) -> Result<(), String> {
        let observed = self.dock.snapshot();
        let descriptor = observed
            .document
            .descriptor(panel)
            .ok_or_else(|| "File preview pane is not registered".to_owned())?;
        if descriptor.role != "hermes_file_preview" {
            return Err("Refusing to close a non-preview pane as a file preview".into());
        }
        if !observed.document.closed_panels.contains(panel) {
            self.dock
                .transact(
                    observed.revision,
                    layout_command("close-file-preview", observed.revision),
                    [Operation::ClosePanel {
                        panel: panel.clone(),
                    }],
                )
                .map_err(|error| error.to_string())?;
            self.save_layout();
        }
        Ok(())
    }

    /// Reopen the most recently closed session pane owned by this Hermes
    /// profile. Closed history is intentionally in-memory, matching the
    /// pinned Desktop's reopen stack; the pane itself remains DoofusDock-owned.
    pub fn restore_last_closed_session_pane(
        &mut self,
        profile: Option<&str>,
        current_session_id: Option<&str>,
    ) -> Result<Option<RestoredSessionPane>, String> {
        let scope = profile.unwrap_or_default();
        loop {
            let closed = self
                .closed_session_panes
                .get_mut(scope)
                .and_then(VecDeque::pop_back);
            let Some(closed) = closed else {
                return Ok(None);
            };
            let document = self.dock.snapshot().document;
            let Some(descriptor) = document.descriptor(&closed.panel) else {
                continue;
            };
            if descriptor.role != "hermes_session_tile"
                || descriptor.metadata.get("hermes.session.id") != Some(&closed.session_id)
                || descriptor
                    .metadata
                    .get("hermes.session.profile")
                    .map(String::as_str)
                    != Some(closed.profile.as_deref().unwrap_or_default())
                || !document.closed_panels.contains(&closed.panel)
                || (closed.session_id == current_session_id.unwrap_or_default()
                    && closed.profile.as_deref().unwrap_or_default() == scope)
            {
                continue;
            }
            let panel = self.open_session_pane(
                closed.profile.as_deref(),
                &closed.session_id,
                &descriptor.title_key,
            )?;
            self.select_panel(&panel)?;
            return Ok(Some(RestoredSessionPane {
                panel,
                session_id: closed.session_id,
                profile: closed.profile,
            }));
        }
    }

    /// Select an existing panel through the canonical Dock tab operation.
    pub fn select_panel(&mut self, panel: &PanelId) -> Result<(), String> {
        let observed = self.dock.snapshot();
        if observed.document.descriptor(panel).is_none() {
            return Err("Panel is not registered in the Dock layout".into());
        }
        if observed.document.closed_panels.contains(panel) {
            return Err("A closed Dock panel cannot be selected".into());
        }
        if let Some((tabs, index)) = find_tab_for_panel(observed.document.root.as_deref(), panel) {
            let active = find_tabs_active_index(observed.document.root.as_deref(), &tabs);
            if active != Some(index) {
                self.dock
                    .transact(
                        observed.revision,
                        CommandId::from(format!("select-panel-{}-{}", observed.revision, panel.0)),
                        [Operation::SelectTab { tabs, index }],
                    )
                    .map_err(|error| error.to_string())?;
                self.save_layout();
            }
        }
        Ok(())
    }

    pub fn session_pane_ids(&self) -> Vec<PanelId> {
        self.dock
            .snapshot()
            .document
            .panels
            .iter()
            .filter(|descriptor| descriptor.role == "hermes_session_tile")
            .map(|descriptor| descriptor.id.clone())
            .collect()
    }

    /// Returns the stored identity for each session pane that is open in the
    /// current Dock tree. Closed or profile-hidden descriptors are excluded.
    pub fn open_session_pane_identities(&self) -> Vec<(String, Option<String>)> {
        let document = self.dock.snapshot().document;
        document
            .panels
            .iter()
            .filter(|descriptor| {
                descriptor.role == "hermes_session_tile"
                    && !document.closed_panels.contains(&descriptor.id)
                    && find_panel_node(document.root.as_deref(), &descriptor.id.0).is_some()
            })
            .filter_map(|descriptor| {
                let session_id = descriptor.metadata.get("hermes.session.id")?.clone();
                let profile = descriptor
                    .metadata
                    .get("hermes.session.profile")
                    .filter(|profile| !profile.is_empty())
                    .cloned();
                Some((session_id, profile))
            })
            .collect()
    }

    pub fn profile_ids(&self) -> Vec<String> {
        let mut ids = match self.preferences.mode {
            InterfaceMode::Advanced => vec!["default", "basic", "focus", "terminal-deck", "quad"],
            InterfaceMode::Simple => vec!["sidebar-left", "sidebar-right"],
        }
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        if self.preferences.mode == InterfaceMode::Advanced {
            ids.extend(self.preferences.user_layout_presets.keys().cloned());
        }
        ids
    }

    pub fn profile_title(&self, id: &str) -> String {
        self.preferences
            .user_layout_presets
            .get(id)
            .map(|preset| preset.name.clone())
            .unwrap_or_else(|| builtin_profile_title(id).to_owned())
    }

    pub fn is_user_profile(&self, id: &str) -> bool {
        self.preferences.user_layout_presets.contains_key(id)
    }

    pub fn save_user_layout_preset(&mut self, name: &str) -> Result<String, String> {
        if self.preferences.mode != InterfaceMode::Advanced {
            return Err("User layout presets can only be saved in Advanced mode".into());
        }
        let name = name.trim();
        if name.is_empty() || name.len() > 80 || name.chars().any(char::is_control) {
            return Err("Layout preset name must be 1–80 printable characters".into());
        }
        let mut id_slug = name
            .to_ascii_lowercase()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>();
        while id_slug.contains("--") {
            id_slug = id_slug.replace("--", "-");
        }
        let id_slug = id_slug.trim_matches('-');
        let id = if id_slug.is_empty() {
            format!("user-{:016x}", stable_text_hash(name))
        } else {
            format!("user-{id_slug}")
        };
        if builtin_profile_ids().contains(&id.as_str()) {
            return Err("Layout preset name conflicts with a built-in layout".into());
        }
        let mut next = self.preferences.clone();
        let document = strip_preset_live_panels(self.dock.snapshot().document);
        next.user_layout_presets.insert(
            id.clone(),
            UserLayoutPreset {
                name: name.to_owned(),
                document,
            },
        );
        next.advanced_profile = id.clone();
        match write_preferences_atomic(&self.preferences_path, &next) {
            Ok(()) => {
                self.preferences = next;
                self.persistence_error = None;
                self.save_layout();
                Ok(id)
            }
            Err(error) => {
                self.persistence_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn delete_user_layout_preset(&mut self, id: &str) -> Result<(), String> {
        if !self.preferences.user_layout_presets.contains_key(id) {
            return Err(format!("Unknown user layout preset: {id}"));
        }
        let was_active = self.active_profile() == id;
        let mut next = self.preferences.clone();
        next.user_layout_presets.remove(id);
        if was_active && next.advanced_profile == id {
            next.advanced_profile = "default".into();
        }
        write_preferences_atomic(&self.preferences_path, &next)?;
        self.preferences = next;
        self.persistence_error = None;
        if was_active {
            self.apply_active_profile()?;
        }
        Ok(())
    }

    pub fn active_profile(&self) -> &str {
        active_profile_id(&self.preferences)
    }

    pub fn last_remote_gateway_url(&self) -> Option<&str> {
        self.preferences.last_remote_gateway_url.as_deref()
    }

    pub fn gateway_connections(&self) -> &[GatewayConnectionDescriptor] {
        &self.preferences.gateway_connections
    }

    pub fn selected_gateway_connection_id(&self) -> &str {
        &self.preferences.selected_gateway_connection_id
    }

    pub fn save_remote_gateway_url(&mut self, url: &str) -> Result<(), String> {
        let normalized = crate::remote_probe::normalize_remote_base_url(url)?;
        let mut next = self.preferences.clone();
        next.last_remote_gateway_url = Some(normalized.clone());
        let descriptor = GatewayConnectionDescriptor::remote(&normalized)?;
        upsert_gateway_connection(&mut next.gateway_connections, descriptor.clone());
        next.selected_gateway_connection_id = descriptor.connection_id;
        write_preferences_atomic(&self.preferences_path, &next)?;
        self.preferences = next;
        self.persistence_error = None;
        Ok(())
    }

    pub fn set_profile(&mut self, id: &str) -> Result<(), String> {
        if !self.profile_ids().iter().any(|profile_id| profile_id == id) {
            return Err(format!("The {id} layout is unavailable in this mode."));
        }
        match self.preferences.mode {
            InterfaceMode::Advanced => self.preferences.advanced_profile = id.into(),
            InterfaceMode::Simple => self.preferences.simple_sidebar_right = id == "sidebar-right",
        }
        self.session_reveals.clear();
        self.apply_active_profile()?;
        self.save_preferences();
        Ok(())
    }

    pub fn set_show_all_project_sessions(&mut self, show: bool) -> Result<(), String> {
        if self.preferences.show_all_project_sessions == show {
            return Ok(());
        }
        let mut next = self.preferences.clone();
        next.show_all_project_sessions = show;
        match write_preferences_atomic(&self.preferences_path, &next) {
            Ok(()) => {
                self.preferences = next;
                self.persistence_error = None;
                Ok(())
            }
            Err(error) => {
                self.persistence_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn set_auto_project_dismissed(
        &mut self,
        project_id: &str,
        dismissed: bool,
    ) -> Result<bool, String> {
        validate_auto_project_id(project_id)?;
        let _lock = acquire_auto_project_dismissal_lock(&self.dismissed_auto_project_ids_path)?;
        let mut ids = read_auto_project_dismissal_state(&self.dismissed_auto_project_ids_path)?
            .unwrap_or_else(|| self.preferences.dismissed_auto_project_ids.clone());
        let already_dismissed = ids.iter().any(|id| id == project_id);
        if already_dismissed == dismissed {
            self.preferences.dismissed_auto_project_ids = ids;
            return Ok(false);
        }
        if dismissed {
            if ids.len() >= AUTO_PROJECT_DISMISSAL_LIMIT {
                return Err(format!(
                    "Dismissed auto-project list exceeds {AUTO_PROJECT_DISMISSAL_LIMIT} entries"
                ));
            }
            ids.push(project_id.to_owned());
        } else {
            ids.retain(|id| id != project_id);
        }
        write_auto_project_dismissal_state(&self.dismissed_auto_project_ids_path, &ids)?;
        self.preferences.dismissed_auto_project_ids = ids;
        self.persistence_error = None;
        Ok(true)
    }

    pub fn refresh_auto_project_dismissals(&mut self) -> Result<bool, String> {
        let Some(ids) = read_auto_project_dismissal_state(&self.dismissed_auto_project_ids_path)?
        else {
            return Ok(false);
        };
        if ids == self.preferences.dismissed_auto_project_ids {
            return Ok(false);
        }
        self.preferences.dismissed_auto_project_ids = ids;
        Ok(true)
    }

    pub fn pinned_session_order(&self, connection_key: &str) -> Vec<String> {
        self.preferences
            .pinned_session_order
            .get(connection_key)
            .cloned()
            .unwrap_or_default()
    }

    pub fn sidebar_session_order(&self, scope_key: &str) -> Vec<String> {
        self.preferences
            .sidebar_session_order
            .get(scope_key)
            .cloned()
            .unwrap_or_default()
    }

    pub fn master_detail_width_override(&self, resize_id: &str) -> Option<f32> {
        self.preferences
            .master_detail_width_overrides
            .get(resize_id)
            .copied()
    }

    pub fn set_master_detail_width_override(
        &mut self,
        resize_id: &str,
        width: Option<f32>,
    ) -> Result<(), String> {
        if resize_id.is_empty() || resize_id.len() > 128 || resize_id.chars().any(char::is_control)
        {
            return Err("Master/detail resize ID is invalid".into());
        }
        if width.is_some_and(|width| !width.is_finite() || !(0.0..=100_000.0).contains(&width)) {
            return Err("Master/detail width override is outside the safe range".into());
        }
        let current = self.master_detail_width_override(resize_id);
        if current == width {
            return Ok(());
        }
        let mut next = self.preferences.clone();
        if let Some(width) = width {
            next.master_detail_width_overrides
                .insert(resize_id.to_owned(), width);
        } else {
            next.master_detail_width_overrides.remove(resize_id);
        }
        match write_preferences_atomic(&self.preferences_path, &next) {
            Ok(()) => {
                self.preferences = next;
                self.persistence_error = None;
                Ok(())
            }
            Err(error) => {
                self.persistence_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn set_sidebar_session_order(
        &mut self,
        scope_key: &str,
        order: Vec<String>,
    ) -> Result<(), String> {
        validate_sidebar_session_key(scope_key, "session-order")?;
        if order.len() > 20_000 {
            return Err("Recent-session order exceeds 20000 entries".into());
        }
        let mut unique = BTreeSet::new();
        for session_id in &order {
            validate_sidebar_session_key(scope_key, session_id)?;
            if !unique.insert(session_id.as_str()) {
                return Err("Recent-session order contains duplicate IDs".into());
            }
        }
        if !order.is_empty()
            && !self
                .preferences
                .sidebar_session_order
                .contains_key(scope_key)
            && self.preferences.sidebar_session_order.len() >= 64
        {
            return Err("Recent-session ordering scope limit of 64 was reached".into());
        }
        if order.is_empty() {
            self.preferences.sidebar_session_order.remove(scope_key);
        } else {
            self.preferences
                .sidebar_session_order
                .insert(scope_key.to_owned(), order);
        }
        self.save_preferences();
        if let Some(error) = &self.persistence_error {
            Err(error.clone())
        } else {
            Ok(())
        }
    }

    pub fn session_color(&self, session_id: &str) -> Option<&str> {
        self.preferences
            .session_color_overrides
            .get(session_id)
            .map(String::as_str)
    }

    pub fn set_session_color(
        &mut self,
        session_id: &str,
        color: Option<&str>,
    ) -> Result<(), String> {
        if session_id.is_empty()
            || session_id.len() > 1024
            || session_id.chars().any(char::is_control)
        {
            return Err("Session color requires a valid session ID".into());
        }
        if let Some(color) = color {
            if !SESSION_COLOR_HUES
                .iter()
                .any(|hue| color == format!("hsl({hue} 68% 58%)"))
            {
                return Err("Session color is not one of the Hermes desktop swatches".into());
            }
            if !self
                .preferences
                .session_color_overrides
                .contains_key(session_id)
                && self.preferences.session_color_overrides.len() >= 20_000
            {
                return Err("Session color override limit of 20000 was reached".into());
            }
            self.preferences
                .session_color_overrides
                .insert(session_id.to_owned(), color.to_owned());
        } else {
            self.preferences.session_color_overrides.remove(session_id);
        }
        self.save_preferences();
        if let Some(error) = &self.persistence_error {
            Err(error.clone())
        } else {
            Ok(())
        }
    }

    pub fn set_pinned_session_state(
        &mut self,
        connection_key: &str,
        session_id: &str,
        pinned: bool,
    ) -> Result<(), String> {
        validate_pinned_session_key(connection_key, session_id)?;
        let mut order = self.pinned_session_order(connection_key);
        if pinned {
            if order.iter().any(|id| id == session_id) {
                return Ok(());
            }
            order.push(session_id.to_owned());
        } else {
            let previous_len = order.len();
            order.retain(|id| id != session_id);
            if order.len() == previous_len {
                return Ok(());
            }
        }
        self.set_pinned_session_order(connection_key, order)
    }

    pub fn set_pinned_session_order(
        &mut self,
        connection_key: &str,
        order: Vec<String>,
    ) -> Result<(), String> {
        if connection_key.is_empty()
            || connection_key.len() > 2048
            || connection_key.chars().any(char::is_control)
        {
            return Err("Pinned-session connection key is invalid".into());
        }
        if order.len() > 20_000 {
            return Err("Pinned-session order exceeds 20000 entries".into());
        }
        let mut unique = BTreeSet::new();
        for session_id in &order {
            validate_pinned_session_key(connection_key, session_id)?;
            if !unique.insert(session_id.as_str()) {
                return Err("Pinned-session order contains duplicate IDs".into());
            }
        }
        if !order.is_empty()
            && !self
                .preferences
                .pinned_session_order
                .contains_key(connection_key)
            && self.preferences.pinned_session_order.len() >= 64
        {
            return Err("Pinned-session connection limit of 64 was reached".into());
        }
        if order.is_empty() {
            self.preferences.pinned_session_order.remove(connection_key);
        } else {
            self.preferences
                .pinned_session_order
                .insert(connection_key.to_owned(), order);
        }
        self.save_preferences();
        if let Some(error) = &self.persistence_error {
            Err(error.clone())
        } else {
            Ok(())
        }
    }

    pub fn reorder_pinned_session_order(
        &mut self,
        connection_key: &str,
        resolved_order: Vec<String>,
    ) -> Result<(), String> {
        let mut order = self.pinned_session_order(connection_key);
        let mut moving = Vec::with_capacity(resolved_order.len());
        let mut seen = BTreeSet::new();
        for session_id in resolved_order {
            validate_pinned_session_key(connection_key, &session_id)?;
            if !seen.insert(session_id.clone()) {
                return Err("Pinned-session reorder contains duplicate IDs".into());
            }
            if !order.iter().any(|known| known == &session_id) {
                order.push(session_id.clone());
            }
            moving.push(session_id);
        }

        if moving.is_empty() {
            return Ok(());
        }
        if order.len() > 20_000 {
            return Err("Pinned-session order exceeds 20000 entries".into());
        }
        let moving_set = moving.iter().collect::<BTreeSet<_>>();
        let mut cursor = 0;
        for session_id in &mut order {
            if moving_set.contains(session_id) {
                *session_id = moving[cursor].clone();
                cursor += 1;
            }
        }
        self.set_pinned_session_order(connection_key, order)
    }
    pub fn set_mode(&mut self, mode: InterfaceMode) -> Result<(), String> {
        if self.preferences.mode == mode {
            return Ok(());
        }
        self.preferences.mode = mode;
        self.session_reveals.clear();
        self.apply_active_profile()?;
        self.save_preferences();
        Ok(())
    }

    pub fn pane_is_open(&self, panel: &str) -> bool {
        match panel {
            SESSIONS => return self.preferences.sessions_sidebar_open,
            WORKSPACE => return true,
            FILES => return self.preferences.file_browser_open,
            BOTS => return *self.preferences.pane_open.get(BOTS).unwrap_or(&false),
            _ => {}
        }
        if let Some(open) = self.session_reveals.get(panel) {
            return *open;
        }
        match self.preferences.mode {
            InterfaceMode::Advanced => *self.preferences.pane_open.get(panel).unwrap_or(&false),
            InterfaceMode::Simple => false,
        }
    }

    pub fn toggle_pane(&mut self, panel: &str) -> Result<(), String> {
        if !matches!(panel, SESSIONS | FILES | REVIEW | TERMINAL | BOTS) {
            return Err(format!(
                "Pane {panel:?} is not registered for toggle actions"
            ));
        }
        if matches!(panel, SESSIONS | FILES) && self.toggle_semantic_side(panel)? {
            return Ok(());
        }
        let open = !self.pane_is_open(panel);
        match panel {
            SESSIONS => self.preferences.sessions_sidebar_open = open,
            FILES => {
                self.preferences.file_browser_open = open;
                self.preferences.pane_open.insert(panel.into(), open);
                self.save_preferences();
            }
            REVIEW | TERMINAL => match self.preferences.mode {
                InterfaceMode::Advanced => {
                    self.preferences.pane_open.insert(panel.into(), open);
                    self.save_preferences();
                }
                InterfaceMode::Simple => {
                    self.session_reveals.insert(panel.into(), open);
                }
            },
            BOTS => {
                self.preferences.pane_open.insert(BOTS.into(), open);
                self.save_preferences();
            }
            _ => unreachable!("toggle panel allowlist is exhaustive"),
        }
        if matches!(panel, SESSIONS) {
            self.save_preferences();
        }
        self.apply_active_profile()?;
        if panel == BOTS && open {
            self.select_panel(&PanelId::from(BOTS))?;
        }
        Ok(())
    }

    fn toggle_semantic_side(&mut self, panel: &str) -> Result<bool, String> {
        let snapshot = self.dock.snapshot();
        let panel_id = PanelId::from(panel);
        if find_panel_node(snapshot.document.root.as_deref(), &panel_id.0).is_none() {
            return Ok(false);
        }
        let Some(side) = root_side_for_panel(&snapshot.document, &panel_id) else {
            return Ok(false);
        };
        let minimized = minimized_groups_on_side(&snapshot.document, side);
        let side_was_collapsed = self.preferences.collapsed_sides.contains(&side);
        let currently_open = self.pane_is_open(panel);
        let open = if side_was_collapsed || !minimized.is_empty() {
            // Like the pinned tree store, a semantic toggle first restores a
            // physically folded side, even when its pane preference is true.
            true
        } else {
            !currently_open
        };

        let mut next_preferences = self.preferences.clone();
        match panel {
            SESSIONS => next_preferences.sessions_sidebar_open = open,
            FILES => {
                next_preferences.file_browser_open = open;
                next_preferences.pane_open.insert(FILES.into(), open);
            }
            _ => unreachable!("semantic side allowlist is exhaustive"),
        }
        if open {
            next_preferences.collapsed_sides.remove(&side);
        } else {
            next_preferences.collapsed_sides.insert(side);
        }

        if !minimized.is_empty() {
            self.dock
                .transact(
                    snapshot.revision,
                    layout_command("restore-semantic-side", snapshot.revision),
                    minimized
                        .iter()
                        .cloned()
                        .map(|group| Operation::SetGroupMinimized {
                            group,
                            minimized: false,
                        }),
                )
                .map_err(|error| error.to_string())?;
        }
        if let Err(error) = write_preferences_atomic(&self.preferences_path, &next_preferences) {
            if !minimized.is_empty() {
                let restored = self.dock.snapshot();
                let rollback = self.dock.transact(
                    restored.revision,
                    layout_command("rollback-semantic-side-restore", restored.revision),
                    minimized
                        .into_iter()
                        .map(|group| Operation::SetGroupMinimized {
                            group,
                            minimized: true,
                        }),
                );
                if let Err(rollback_error) = rollback {
                    return Err(format!(
                        "Persist semantic side state failed ({error}); Dock rollback also failed ({rollback_error})"
                    ));
                }
            }
            self.persistence_error = Some(error.clone());
            return Err(error);
        }
        self.preferences = next_preferences;
        self.persistence_error = None;
        if !minimized.is_empty() {
            self.save_layout();
        }
        Ok(true)
    }

    /// Fold the direct outer group at one edge of the live root row. Returns
    /// `false` when that edge is not an eligible side group so callers can
    /// apply the source's semantic-pane fallback.
    pub fn toggle_positional_edge(&mut self, left: bool) -> Result<bool, String> {
        let snapshot = self.dock.snapshot();
        let Some((edge, side)) = positional_edge_node(&snapshot.document, left) else {
            return Ok(false);
        };
        let group_minimized = match &edge.kind {
            NodeKind::Tabs { minimized, .. } => Some(*minimized),
            NodeKind::Panel { .. } => None,
            _ => return Ok(false),
        };
        let side_collapsed = self.preferences.collapsed_sides.contains(&side);
        let shown = !side_collapsed && group_minimized.is_none_or(|minimized| !minimized);
        let mut next_preferences = self.preferences.clone();
        if shown {
            if let Some(_) = group_minimized {
                self.dock
                    .transact(
                        snapshot.revision,
                        layout_command(
                            if left {
                                "minimize-left-group"
                            } else {
                                "minimize-right-group"
                            },
                            snapshot.revision,
                        ),
                        [Operation::SetGroupMinimized {
                            group: edge.id.clone(),
                            minimized: true,
                        }],
                    )
                    .map_err(|error| error.to_string())?;
                self.save_layout();
            } else {
                next_preferences.collapsed_sides.insert(side);
                self.persist_preferences(next_preferences)?;
            }
            return Ok(true);
        }

        if group_minimized == Some(true) {
            self.dock
                .transact(
                    snapshot.revision,
                    layout_command(
                        if left {
                            "restore-left-group"
                        } else {
                            "restore-right-group"
                        },
                        snapshot.revision,
                    ),
                    [Operation::SetGroupMinimized {
                        group: edge.id.clone(),
                        minimized: false,
                    }],
                )
                .map_err(|error| error.to_string())?;
            self.save_layout();
        }
        next_preferences.collapsed_sides.remove(&side);
        let owner = semantic_owner_for_side(&snapshot.document, side, &self.preferences);
        set_semantic_owner_open(&mut next_preferences, owner, true);
        if let Err(error) = self.persist_preferences(next_preferences) {
            if group_minimized == Some(true) {
                let restored = self.dock.snapshot();
                let rollback = self.dock.transact(
                    restored.revision,
                    layout_command("rollback-positional-edge-restore", restored.revision),
                    [Operation::SetGroupMinimized {
                        group: edge.id.clone(),
                        minimized: true,
                    }],
                );
                if let Err(rollback_error) = rollback {
                    return Err(format!(
                        "Persist positional edge state failed ({error}); Dock rollback also failed ({rollback_error})"
                    ));
                }
                self.save_layout();
            }
            return Err(error);
        }
        Ok(true)
    }

    pub fn positional_edge_open(&self, left: bool) -> bool {
        let snapshot = self.dock.snapshot();
        positional_edge_node(&snapshot.document, left)
            .map(|(node, side)| {
                if self.preferences.collapsed_sides.contains(&side) {
                    return false;
                }
                match &node.kind {
                    NodeKind::Tabs { minimized, .. } => !minimized,
                    NodeKind::Panel { .. } => true,
                    _ => false,
                }
            })
            .unwrap_or_else(|| self.pane_is_open(if left { SESSIONS } else { FILES }))
    }

    /// Produce a presentation-only layout with semantically collapsed root
    /// sides removed. The canonical document and its tree identity remain
    /// untouched, so reopening restores the exact group and active tab.
    pub fn presentation_layout(&self, document: LayoutDocument) -> LayoutDocument {
        self.presentation_layout_with_collapsed_sides(document, &self.preferences.collapsed_sides)
    }

    /// Narrow viewports temporarily move both physical edge groups out of the
    /// dock projection. This is presentation state only; the canonical tree
    /// and saved collapse preferences remain untouched.
    pub fn narrow_presentation_layout(&self, document: LayoutDocument) -> LayoutDocument {
        let collapsed = BTreeSet::from([PhysicalSide::Left, PhysicalSide::Right]);
        self.presentation_layout_with_collapsed_sides(document, &collapsed)
    }

    pub fn semantic_panel_is_right(&self, panel: &str) -> bool {
        let document = self.dock.snapshot().document;
        root_side_for_panel(&document, &PanelId::from(panel))
            .map(|side| side == PhysicalSide::Right)
            .unwrap_or_else(|| {
                self.preferences.simple_sidebar_right
                    || (self.preferences.mode == InterfaceMode::Advanced
                        && self.preferences.advanced_profile == "sidebar-right")
            })
    }

    /// Return source-collapsible built-in panes currently live on a physical
    /// edge. This is presentation discovery only; narrow reveals do not open,
    /// close, or otherwise mutate the canonical document.
    pub fn narrow_collapsible_panels(&self, right: bool) -> Vec<String> {
        let document = self.dock.snapshot().document;
        [SESSIONS, BOTS, FILES, REVIEW]
            .into_iter()
            .filter(|panel| {
                root_side_for_panel(&document, &PanelId::from(*panel))
                    .is_some_and(|side| (side == PhysicalSide::Right) == right)
            })
            .map(str::to_owned)
            .collect()
    }

    pub fn narrow_panel_is_right(&self, panel: &str) -> bool {
        let document = self.dock.snapshot().document;
        root_side_for_panel(&document, &PanelId::from(panel))
            .map(|side| side == PhysicalSide::Right)
            .unwrap_or_else(|| match panel {
                SESSIONS => self.semantic_panel_is_right(SESSIONS),
                BOTS | FILES | REVIEW => true,
                _ => self.semantic_panel_is_right(panel),
            })
    }

    fn presentation_layout_with_collapsed_sides(
        &self,
        mut document: LayoutDocument,
        collapsed_sides: &BTreeSet<PhysicalSide>,
    ) -> LayoutDocument {
        if collapsed_sides.is_empty() {
            return document;
        }
        let Some(root) = root_row(&document) else {
            return document;
        };
        let NodeKind::Split {
            orientation: Orientation::Horizontal,
            weights,
            children,
        } = &root.kind
        else {
            return document;
        };
        let weights = weights.clone();
        let children = children.clone();
        let retained = children
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                root_child_side(&children, *index)
                    .is_none_or(|side| !collapsed_sides.contains(&side))
            })
            .map(|(index, child)| (index, child.clone()))
            .collect::<Vec<_>>();
        if retained.len() == children.len() {
            return document;
        }
        let row_id = root.id.clone();
        let mut hidden_panels = BTreeSet::new();
        for (index, child) in children.iter().enumerate() {
            if root_child_side(&children, index).is_some_and(|side| collapsed_sides.contains(&side))
            {
                collect_node_panels(child, &mut hidden_panels);
            }
        }
        for panel in hidden_panels {
            if !document.closed_panels.contains(&panel) {
                document.closed_panels.push(panel);
            }
        }
        if retained.len() == 1 {
            replace_root_row(&mut document, &row_id, retained[0].1.clone());
            return document;
        }
        if retained.is_empty() {
            return document;
        }
        let total = retained
            .iter()
            .map(|(index, _)| weights.get(*index).copied().unwrap_or(0.0))
            .sum::<f64>();
        let next_weights = retained
            .iter()
            .map(|(index, _)| {
                let weight = weights.get(*index).copied().unwrap_or(0.0);
                if total > f64::EPSILON {
                    weight / total
                } else {
                    1.0 / retained.len() as f64
                }
            })
            .collect::<Vec<_>>();
        replace_root_row(
            &mut document,
            &row_id,
            Node::split(
                row_id.clone(),
                Orientation::Horizontal,
                retained.into_iter().map(|(_, child)| child).collect(),
                next_weights,
            ),
        );
        document
    }

    fn persist_preferences(&mut self, next: UiPreferences) -> Result<(), String> {
        match write_preferences_atomic(&self.preferences_path, &next) {
            Ok(()) => {
                self.preferences = next;
                self.persistence_error = None;
                Ok(())
            }
            Err(error) => {
                self.persistence_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn apply_active_profile(&mut self) -> Result<(), String> {
        let id = self.active_profile().to_owned();
        if let Some(preset) = self.preferences.user_layout_presets.get(&id).cloned() {
            self.apply_user_layout_preset(&preset.document)?;
            return Ok(());
        }
        let p = profile(&self.profiles, &id)?;
        apply_profile_to_service(&mut self.dock, p, &self.preferences, &self.session_reveals)?;
        self.save_layout();
        Ok(())
    }

    fn apply_user_layout_preset(&mut self, saved: &LayoutDocument) -> Result<(), String> {
        let current = self.dock.snapshot();
        let mut target = saved.clone();
        target.id = current.document.id.clone();
        target.workspace = current.document.workspace.clone();
        target.revision = current.revision;
        let sessions = current
            .document
            .panels
            .iter()
            .filter(|panel| panel.role == "hermes_session_tile")
            .cloned()
            .collect::<Vec<_>>();
        let session_ids = sessions
            .iter()
            .map(|panel| panel.id.clone())
            .collect::<BTreeSet<_>>();
        let mut panels = target.panels.as_ref().clone();
        panels.extend(sessions.iter().cloned());
        target.panels = Arc::new(panels);
        target
            .closed_panels
            .retain(|panel| !session_ids.contains(panel));
        target.closed_panels.extend(
            current
                .document
                .closed_panels
                .iter()
                .filter(|panel| session_ids.contains(panel))
                .cloned(),
        );
        target.closed_panels.sort();
        target.closed_panels.dedup();
        let live = sessions
            .iter()
            .filter(|panel| !current.document.closed_panels.contains(&panel.id))
            .cloned()
            .collect::<Vec<_>>();
        target.root = adopt_live_sessions(target.root.take(), &live, &current.document)?;
        self.dock
            .apply_layout(current.revision, &target)
            .map_err(|error| error.to_string())?;
        self.save_layout();
        if self.persistence_error.is_some() {
            return Err(self.persistence_error.clone().unwrap_or_default());
        }
        Ok(())
    }

    pub fn save_layout(&mut self) {
        let document = persistent_layout(self.dock.snapshot().document);
        match self.store.save(&document) {
            Ok(()) => self.persistence_error = None,
            Err(error) => self.persistence_error = Some(error.to_string()),
        }
    }

    fn save_preferences(&mut self) {
        match write_preferences_atomic(&self.preferences_path, &self.preferences) {
            Ok(()) => self.persistence_error = None,
            Err(error) => self.persistence_error = Some(error),
        }
    }
}

fn positional_edge_node(document: &LayoutDocument, left: bool) -> Option<(&Node, PhysicalSide)> {
    let root = root_row(document)?;
    let NodeKind::Split {
        orientation: Orientation::Horizontal,
        children,
        ..
    } = &root.kind
    else {
        return None;
    };
    let edge_index = if left {
        0
    } else {
        children.len().checked_sub(1)?
    };
    let edge = children.get(edge_index)?;
    if node_contains_main_surface(edge)
        || !children
            .iter()
            .any(|child| node_contains_main_surface(child))
    {
        return None;
    }
    Some((
        edge.as_ref(),
        if left {
            PhysicalSide::Left
        } else {
            PhysicalSide::Right
        },
    ))
}

fn root_side_for_panel(document: &LayoutDocument, panel: &PanelId) -> Option<PhysicalSide> {
    let root = root_row(document)?;
    let NodeKind::Split {
        orientation: Orientation::Horizontal,
        children,
        ..
    } = &root.kind
    else {
        return None;
    };
    let index = children
        .iter()
        .position(|child| node_contains_panel(child, panel))?;
    root_child_side(children, index)
}

fn root_child_side(children: &[Arc<Node>], index: usize) -> Option<PhysicalSide> {
    let main_indices = children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| node_contains_main_surface(child).then_some(index))
        .collect::<Vec<_>>();
    let first_main = *main_indices.first()?;
    let last_main = *main_indices.last()?;
    if index < first_main {
        Some(PhysicalSide::Left)
    } else if index > last_main {
        Some(PhysicalSide::Right)
    } else {
        None
    }
}

fn minimized_groups_on_side(document: &LayoutDocument, side: PhysicalSide) -> Vec<NodeId> {
    fn collect(node: &Node, groups: &mut Vec<NodeId>) {
        match &node.kind {
            NodeKind::Tabs {
                items, minimized, ..
            } => {
                if *minimized {
                    groups.push(node.id.clone());
                }
                for child in items {
                    collect(child, groups);
                }
            }
            NodeKind::DocumentWell { groups: items, .. }
            | NodeKind::Split {
                children: items, ..
            }
            | NodeKind::Extension {
                children: items, ..
            } => {
                for child in items {
                    collect(child, groups);
                }
            }
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => {}
        }
    }

    let Some(root) = root_row(document) else {
        return Vec::new();
    };
    let NodeKind::Split {
        orientation: Orientation::Horizontal,
        children,
        ..
    } = &root.kind
    else {
        return Vec::new();
    };
    let mut groups = Vec::new();
    for (index, child) in children.iter().enumerate() {
        if root_child_side(children, index) == Some(side) {
            collect(child, &mut groups);
        }
    }
    groups
}

/// Hermes side controls operate on the horizontal row containing the main
/// surface. Column-root profiles place that row as a direct child of the root.
/// A split at the row's physical edge is still not a foldable leaf group.
fn root_row(document: &LayoutDocument) -> Option<&Node> {
    let root = document.root.as_deref()?;
    let NodeKind::Split {
        orientation,
        children,
        ..
    } = &root.kind
    else {
        return None;
    };
    if *orientation == Orientation::Horizontal {
        return Some(root);
    }
    children.iter().find_map(|child| {
        matches!(
            child.kind,
            NodeKind::Split {
                orientation: Orientation::Horizontal,
                ..
            }
        )
        .then_some(child.as_ref())
        .filter(|row| node_contains_main_surface(row))
    })
}

fn replace_root_row(document: &mut LayoutDocument, row_id: &NodeId, replacement: Arc<Node>) {
    let Some(root) = document.root.as_deref() else {
        return;
    };
    if &root.id == row_id {
        document.root = Some(replacement);
        return;
    }
    let NodeKind::Split {
        orientation,
        weights,
        children,
    } = &root.kind
    else {
        return;
    };
    if !children.iter().any(|child| &child.id == row_id) {
        return;
    }
    let children = children
        .iter()
        .map(|child| {
            if &child.id == row_id {
                replacement.clone()
            } else {
                child.clone()
            }
        })
        .collect();
    document.root = Some(Node::split(
        root.id.clone(),
        *orientation,
        children,
        weights.to_vec(),
    ));
}

fn semantic_owner_for_side(
    document: &LayoutDocument,
    side: PhysicalSide,
    preferences: &UiPreferences,
) -> &'static str {
    if root_side_for_panel(document, &PanelId::from(SESSIONS)) == Some(side) {
        SESSIONS
    } else if root_side_for_panel(document, &PanelId::from(FILES)) == Some(side) {
        FILES
    } else if (preferences.simple_sidebar_right
        || (preferences.mode == InterfaceMode::Advanced
            && preferences.advanced_profile == "sidebar-right"))
        == (side == PhysicalSide::Right)
    {
        SESSIONS
    } else {
        FILES
    }
}

fn set_semantic_owner_open(preferences: &mut UiPreferences, owner: &str, open: bool) {
    match owner {
        SESSIONS => preferences.sessions_sidebar_open = open,
        FILES => {
            preferences.file_browser_open = open;
            preferences.pane_open.insert(FILES.into(), open);
        }
        _ => unreachable!("semantic side owner is a registered pane"),
    }
}

fn node_contains_panel(node: &Node, panel: &PanelId) -> bool {
    match &node.kind {
        NodeKind::Panel { panel: id } | NodeKind::Placeholder { panel: id, .. } => id == panel,
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => items.iter().any(|child| node_contains_panel(child, panel)),
    }
}

fn collect_node_panels(node: &Node, panels: &mut BTreeSet<PanelId>) {
    match &node.kind {
        NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. } => {
            panels.insert(panel.clone());
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => {
            for child in items {
                collect_node_panels(child, panels);
            }
        }
    }
}

fn node_contains_main_surface(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. } => {
            panel.0 == WORKSPACE
                || panel.0.starts_with("session-tile:")
                || panel.0.starts_with("route-tile:")
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => items.iter().any(|child| node_contains_main_surface(child)),
    }
}

fn default_true() -> bool {
    true
}

fn file_preference_path_key(path: &Path) -> String {
    let clean = path
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    if cfg!(windows) {
        clean.to_lowercase()
    } else {
        clean
    }
}

fn validate_pinned_session_key(connection_key: &str, session_id: &str) -> Result<(), String> {
    if connection_key.is_empty()
        || connection_key.len() > 2048
        || connection_key.chars().any(char::is_control)
    {
        return Err("Pinned-session connection key is invalid".into());
    }
    if session_id.is_empty() || session_id.len() > 512 || session_id.chars().any(char::is_control) {
        return Err("Pinned session ID is invalid".into());
    }
    Ok(())
}

fn validate_sidebar_session_key(scope_key: &str, session_id: &str) -> Result<(), String> {
    if scope_key.is_empty()
        || scope_key.len() > 2048
        || scope_key.chars().any(char::is_control)
        || session_id.is_empty()
        || session_id.len() > 1024
        || session_id.chars().any(char::is_control)
    {
        return Err("Recent-session ordering key or session ID is invalid".into());
    }
    Ok(())
}
pub fn active_profile_id(preferences: &UiPreferences) -> &str {
    match preferences.mode {
        InterfaceMode::Advanced => &preferences.advanced_profile,
        InterfaceMode::Simple if preferences.simple_sidebar_right => "sidebar-right",
        InterfaceMode::Simple => "sidebar-left",
    }
}

fn apply_profile_to_service(
    dock: &mut DockingService,
    profile: &WorkspaceProfile,
    preferences: &UiPreferences,
    reveals: &BTreeMap<String, bool>,
) -> Result<(), String> {
    apply_profile_to_service_with_reopen(dock, profile, preferences, reveals, &BTreeSet::new())
}

fn apply_profile_to_service_with_reopen(
    dock: &mut DockingService,
    profile: &WorkspaceProfile,
    preferences: &UiPreferences,
    reveals: &BTreeMap<String, bool>,
    force_open: &BTreeSet<PanelId>,
) -> Result<(), String> {
    let observed = dock.snapshot();
    let mut assignments = BTreeMap::new();
    let mut workspace_panels = vec![PanelId::from(WORKSPACE)];
    workspace_panels.extend(observed.document.panels.iter().filter_map(|descriptor| {
        (descriptor.role == "hermes_session_tile"
            && (!observed.document.closed_panels.contains(&descriptor.id)
                || force_open.contains(&descriptor.id)))
        .then(|| descriptor.id.clone())
    }));
    assignments.insert(WORKSPACE.to_owned(), workspace_panels);
    for panel in [SESSIONS, BOTS, REVIEW, FILES, TERMINAL] {
        let open = match panel {
            // Sessions occupies a required profile region. Project it into the
            // profile first, then apply HideOnly through the Dock operation.
            SESSIONS => true,
            BOTS => preferences.pane_open.get(BOTS).copied().unwrap_or(false),
            FILES => preferences.file_browser_open,
            _ => reveals
                .get(panel)
                .copied()
                .unwrap_or_else(|| match preferences.mode {
                    InterfaceMode::Advanced => {
                        preferences.pane_open.get(panel).copied().unwrap_or(false)
                    }
                    InterfaceMode::Simple => false,
                }),
        };
        assignments.insert(
            panel.to_owned(),
            if open {
                vec![PanelId::from(panel)]
            } else {
                Vec::new()
            },
        );
    }

    let application = apply_workspace_profile(&observed.document, profile, &assignments)
        .map_err(|error| error.to_string())?;
    dock.apply_layout(observed.revision, &application.document)
        .map_err(|error| error.to_string())?;
    if !preferences.sessions_sidebar_open {
        let hidden = dock.snapshot();
        if !hidden
            .document
            .closed_panels
            .contains(&PanelId::from(SESSIONS))
        {
            dock.transact(
                hidden.revision,
                layout_command("hide-sessions-sidebar", hidden.revision),
                [Operation::ClosePanel {
                    panel: PanelId::from(SESSIONS),
                }],
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn validate_session_pane_identity(
    profile: Option<&str>,
    stored_session_id: &str,
    title: &str,
) -> Result<(), String> {
    if stored_session_id.is_empty()
        || stored_session_id.len() > 512
        || stored_session_id.chars().any(char::is_control)
    {
        return Err("Stored session ID is invalid".into());
    }
    if profile.is_some_and(|value| {
        value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
    }) {
        return Err("Session profile is invalid".into());
    }
    if title.len() > 512 || title.chars().any(char::is_control) {
        return Err("Session title is invalid".into());
    }
    Ok(())
}

fn bounded_session_title(title: &str) -> String {
    let title: String = title.chars().take(72).collect();
    if title.trim().is_empty() {
        "Session".into()
    } else {
        title
    }
}

fn session_pane_id(profile: Option<&str>, stored_session_id: &str) -> PanelId {
    // Fixed FNV-1a over explicit separators provides a stable, bounded panel ID.
    // Full identities remain in descriptor metadata and are checked on lookup.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in profile
        .unwrap_or_default()
        .bytes()
        .chain([0])
        .chain(stored_session_id.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    PanelId::from(format!("session-tile:{hash:016x}"))
}

fn file_preview_panel_id(path: &str) -> PanelId {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    PanelId::from(format!("file-preview:{hash:016x}"))
}

fn persistent_layout(mut document: LayoutDocument) -> LayoutDocument {
    let ephemeral = document
        .panels
        .iter()
        .filter(|descriptor| descriptor.persistence == PersistencePolicy::Never)
        .map(|descriptor| descriptor.id.clone())
        .collect::<BTreeSet<_>>();
    if ephemeral.is_empty() {
        return document;
    }
    document.root = document
        .root
        .as_deref()
        .and_then(|root| prune_ephemeral_node(root, &ephemeral));
    for window in &mut document.floating_windows {
        if let Some(root) = prune_ephemeral_node(&window.root, &ephemeral) {
            window.root = root;
        }
    }
    document
        .floating_windows
        .retain(|window| !contains_ephemeral(&window.root, &ephemeral));
    document.panels = Arc::new(
        document
            .panels
            .iter()
            .filter(|descriptor| !ephemeral.contains(&descriptor.id))
            .cloned()
            .collect(),
    );
    document
        .closed_panels
        .retain(|panel| !ephemeral.contains(panel));
    document
        .pinned_panels
        .retain(|panel| !ephemeral.contains(panel));
    document
        .auto_hide_panels
        .retain(|entry| !ephemeral.contains(&entry.panel));
    if document
        .maximized_panel
        .as_ref()
        .is_some_and(|panel| ephemeral.contains(panel))
    {
        document.maximized_panel = None;
    }
    document
}

fn builtin_profile_ids() -> [&'static str; 7] {
    [
        "default",
        "basic",
        "focus",
        "terminal-deck",
        "quad",
        "sidebar-left",
        "sidebar-right",
    ]
}

fn builtin_profile_title(id: &str) -> &'static str {
    match id {
        "default" => "Default",
        "basic" => "Basic",
        "focus" => "Focus",
        "terminal-deck" => "Terminal Deck",
        "quad" => "Quad",
        "sidebar-left" => "Sidebar Left",
        "sidebar-right" => "Sidebar Right",
        _ => "Default",
    }
}

fn stable_text_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn strip_preset_live_panels(mut document: LayoutDocument) -> LayoutDocument {
    let ephemeral = document
        .panels
        .iter()
        .filter(|panel| {
            panel.role == "hermes_session_tile" || panel.persistence == PersistencePolicy::Never
        })
        .map(|panel| panel.id.clone())
        .collect::<BTreeSet<_>>();
    if ephemeral.is_empty() {
        return document;
    }
    document.root = document
        .root
        .as_deref()
        .and_then(|root| prune_ephemeral_node(root, &ephemeral));
    for window in &mut document.floating_windows {
        if let Some(root) = prune_ephemeral_node(&window.root, &ephemeral) {
            window.root = root;
        }
    }
    document
        .floating_windows
        .retain(|window| !contains_ephemeral(&window.root, &ephemeral));
    document.panels = Arc::new(
        document
            .panels
            .iter()
            .filter(|panel| !ephemeral.contains(&panel.id))
            .cloned()
            .collect(),
    );
    document
        .closed_panels
        .retain(|panel| !ephemeral.contains(panel));
    document
        .pinned_panels
        .retain(|panel| !ephemeral.contains(panel));
    document
        .auto_hide_panels
        .retain(|entry| !ephemeral.contains(&entry.panel));
    if document
        .maximized_panel
        .as_ref()
        .is_some_and(|panel| ephemeral.contains(panel))
    {
        document.maximized_panel = None;
    }
    document
}

fn adopt_live_sessions(
    mut root: Option<Arc<Node>>,
    sessions: &[PanelDescriptor],
    previous: &LayoutDocument,
) -> Result<Option<Arc<Node>>, String> {
    if sessions.is_empty() {
        return Ok(root);
    }
    let workspace = PanelId::from(WORKSPACE);
    let active_session = active_panel_in_workspace_tabs(previous)
        .filter(|panel| sessions.iter().any(|session| session.id == *panel));
    let mut root_node = root
        .take()
        .ok_or_else(|| "Saved layout has no workspace root for open session tabs".to_owned())?;
    if let Some((tabs, _)) = find_tab_for_panel(Some(&root_node), &workspace) {
        if !append_session_nodes(&mut root_node, &tabs, sessions, active_session.as_ref()) {
            return Err("Saved layout workspace tab group could not accept live sessions".into());
        }
    } else if find_panel_node(Some(&root_node), WORKSPACE).is_some() {
        let mut items = vec![Node::panel("workspace-panel", WORKSPACE)];
        items.extend(sessions.iter().map(|session| session_node(session)));
        let active = active_session
            .as_ref()
            .and_then(|panel| sessions.iter().position(|session| &session.id == panel))
            .map(|index| index + 1)
            .unwrap_or(0);
        let replacement = Node::tabs("workspace-live-session-tabs", items, active);
        root_node = replace_panel_node(root_node, &workspace, replacement).ok_or_else(|| {
            "Saved layout workspace pane could not accept live sessions".to_owned()
        })?;
    } else {
        return Err("Saved layout does not contain the Hermes workspace pane".into());
    }
    Ok(Some(root_node))
}

fn session_node(session: &PanelDescriptor) -> Arc<Node> {
    Node::panel(format!("session-node-{}", session.id.0), session.id.clone())
}

fn active_panel_in_workspace_tabs(document: &LayoutDocument) -> Option<PanelId> {
    fn walk(node: &Node, target: &PanelId) -> Option<PanelId> {
        match &node.kind {
            NodeKind::Tabs {
                items,
                active_index,
                ..
            } => {
                if items
                    .iter()
                    .any(|item| matches!(&item.kind, NodeKind::Panel { panel } if panel == target))
                {
                    let active = items.get(*active_index)?;
                    if let NodeKind::Panel { panel } = &active.kind {
                        return Some(panel.clone());
                    }
                }
                items.iter().find_map(|item| walk(item, target))
            }
            NodeKind::DocumentWell { groups, .. }
            | NodeKind::Split {
                children: groups, ..
            }
            | NodeKind::Extension {
                children: groups, ..
            } => groups.iter().find_map(|item| walk(item, target)),
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
        }
    }
    document
        .root
        .as_deref()
        .and_then(|root| walk(root, &PanelId::from(WORKSPACE)))
}

fn append_session_nodes(
    root: &mut Arc<Node>,
    tabs: &NodeId,
    sessions: &[PanelDescriptor],
    active_session: Option<&PanelId>,
) -> bool {
    if &root.id == tabs {
        if let NodeKind::Tabs {
            items,
            active_index,
            minimized,
        } = &root.kind
        {
            let mut next = items.clone();
            let workspace_index = next
                .iter()
                .position(
                    |item| matches!(&item.kind, NodeKind::Panel { panel } if panel.0 == WORKSPACE),
                )
                .unwrap_or(0);
            next.extend(sessions.iter().map(session_node));
            let selected = active_session
                .and_then(|panel| sessions.iter().position(|session| &session.id == panel))
                .map(|index| items.len() + index)
                .unwrap_or(
                    (*active_index)
                        .min(items.len().saturating_sub(1))
                        .max(workspace_index),
                );
            *root = Arc::new(Node {
                id: root.id.clone(),
                kind: NodeKind::Tabs {
                    items: next,
                    active_index: selected,
                    minimized: *minimized,
                },
            });
            return true;
        }
        return false;
    }
    let mut changed = false;
    let kind = match &root.kind {
        NodeKind::Tabs {
            items,
            active_index,
            minimized,
        } => {
            let mut next = items.clone();
            for child in &mut next {
                if append_session_nodes(child, tabs, sessions, active_session) {
                    changed = true;
                    break;
                }
            }
            NodeKind::Tabs {
                items: next,
                active_index: *active_index,
                minimized: *minimized,
            }
        }
        NodeKind::Split {
            orientation,
            weights,
            children,
        } => {
            let mut next = children.clone();
            for child in &mut next {
                if append_session_nodes(child, tabs, sessions, active_session) {
                    changed = true;
                    break;
                }
            }
            NodeKind::Split {
                orientation: *orientation,
                weights: weights.clone(),
                children: next,
            }
        }
        NodeKind::DocumentWell {
            groups,
            active_group,
        } => {
            let mut next = groups.clone();
            for child in &mut next {
                if append_session_nodes(child, tabs, sessions, active_session) {
                    changed = true;
                    break;
                }
            }
            NodeKind::DocumentWell {
                groups: next,
                active_group: *active_group,
            }
        }
        NodeKind::Extension {
            modality,
            payload,
            children,
        } => {
            let mut next = children.clone();
            for child in &mut next {
                if append_session_nodes(child, tabs, sessions, active_session) {
                    changed = true;
                    break;
                }
            }
            NodeKind::Extension {
                modality: modality.clone(),
                payload: payload.clone(),
                children: next,
            }
        }
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => return false,
    };
    if changed {
        *root = Arc::new(Node {
            id: root.id.clone(),
            kind,
        });
    }
    changed
}

fn replace_panel_node(
    root: Arc<Node>,
    target: &PanelId,
    replacement: Arc<Node>,
) -> Option<Arc<Node>> {
    if matches!(&root.kind, NodeKind::Panel { panel } if panel == target) {
        return Some(replacement);
    }
    fn rewrite(node: &Arc<Node>, target: &PanelId, replacement: &Arc<Node>) -> (Arc<Node>, bool) {
        if matches!(&node.kind, NodeKind::Panel { panel } if panel == target) {
            return (replacement.clone(), true);
        }
        let (kind, changed) = match &node.kind {
            NodeKind::Tabs {
                items,
                active_index,
                minimized,
            } => {
                let (items, changed) = rewrite_children(items, target, replacement);
                (
                    NodeKind::Tabs {
                        items,
                        active_index: *active_index,
                        minimized: *minimized,
                    },
                    changed,
                )
            }
            NodeKind::Split {
                orientation,
                weights,
                children,
            } => {
                let (children, changed) = rewrite_children(children, target, replacement);
                (
                    NodeKind::Split {
                        orientation: *orientation,
                        weights: weights.clone(),
                        children,
                    },
                    changed,
                )
            }
            NodeKind::DocumentWell {
                groups,
                active_group,
            } => {
                let (groups, changed) = rewrite_children(groups, target, replacement);
                (
                    NodeKind::DocumentWell {
                        groups,
                        active_group: *active_group,
                    },
                    changed,
                )
            }
            NodeKind::Extension {
                modality,
                payload,
                children,
            } => {
                let (children, changed) = rewrite_children(children, target, replacement);
                (
                    NodeKind::Extension {
                        modality: modality.clone(),
                        payload: payload.clone(),
                        children,
                    },
                    changed,
                )
            }
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => return (node.clone(), false),
        };
        if changed {
            (
                Arc::new(Node {
                    id: node.id.clone(),
                    kind,
                }),
                true,
            )
        } else {
            (node.clone(), false)
        }
    }
    fn rewrite_children(
        children: &[Arc<Node>],
        target: &PanelId,
        replacement: &Arc<Node>,
    ) -> (Vec<Arc<Node>>, bool) {
        let mut changed_any = false;
        let children = children
            .iter()
            .map(|child| {
                let (next, changed) = rewrite(child, target, replacement);
                changed_any |= changed;
                next
            })
            .collect();
        (children, changed_any)
    }
    let (next, changed) = rewrite(&root, target, &replacement);
    changed.then_some(next)
}

fn contains_ephemeral(node: &Node, ephemeral: &BTreeSet<PanelId>) -> bool {
    match &node.kind {
        NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. } => {
            ephemeral.contains(panel)
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => items
            .iter()
            .any(|child| contains_ephemeral(child, ephemeral)),
    }
}

fn prune_ephemeral_node(node: &Node, ephemeral: &BTreeSet<PanelId>) -> Option<Arc<Node>> {
    match &node.kind {
        NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. }
            if ephemeral.contains(panel) =>
        {
            None
        }
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => Some(Arc::new(node.clone())),
        NodeKind::Tabs {
            items,
            active_index,
            minimized,
        } => {
            let items = items
                .iter()
                .filter_map(|item| prune_ephemeral_node(item, ephemeral))
                .collect::<Vec<_>>();
            match items.len() {
                0 => None,
                1 => items.into_iter().next(),
                _ => Some(Arc::new(Node {
                    id: node.id.clone(),
                    kind: NodeKind::Tabs {
                        active_index: (*active_index).min(items.len() - 1),
                        minimized: *minimized,
                        items,
                    },
                })),
            }
        }
        NodeKind::DocumentWell {
            groups,
            active_group,
        } => {
            let groups = groups
                .iter()
                .filter_map(|item| prune_ephemeral_node(item, ephemeral))
                .collect::<Vec<_>>();
            match groups.len() {
                0 => None,
                1 => groups.into_iter().next(),
                _ => Some(Arc::new(Node {
                    id: node.id.clone(),
                    kind: NodeKind::DocumentWell {
                        active_group: (*active_group).min(groups.len() - 1),
                        groups,
                    },
                })),
            }
        }
        NodeKind::Split {
            orientation,
            weights,
            children,
        } => {
            let retained = children
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    prune_ephemeral_node(item, ephemeral).map(|item| (index, item))
                })
                .collect::<Vec<_>>();
            match retained.len() {
                0 => None,
                1 => retained.into_iter().next().map(|(_, item)| item),
                _ => {
                    let total = retained
                        .iter()
                        .map(|(index, _)| weights.get(*index).copied().unwrap_or(1.0))
                        .sum::<f64>();
                    let normalized = retained
                        .iter()
                        .map(|(index, _)| weights.get(*index).copied().unwrap_or(1.0) / total)
                        .collect();
                    Some(Arc::new(Node {
                        id: node.id.clone(),
                        kind: NodeKind::Split {
                            orientation: *orientation,
                            weights: normalized,
                            children: retained.into_iter().map(|(_, item)| item).collect(),
                        },
                    }))
                }
            }
        }
        NodeKind::Extension {
            modality,
            payload,
            children,
        } => {
            let children = children
                .iter()
                .filter_map(|item| prune_ephemeral_node(item, ephemeral))
                .collect::<Vec<_>>();
            Some(Arc::new(Node {
                id: node.id.clone(),
                kind: NodeKind::Extension {
                    modality: modality.clone(),
                    payload: payload.clone(),
                    children,
                },
            }))
        }
    }
}

fn find_panel_node(root: Option<&Node>, panel_id: &str) -> Option<NodeId> {
    let node = root?;
    match &node.kind {
        NodeKind::Panel { panel } if panel.0 == panel_id => Some(node.id.clone()),
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => items
            .iter()
            .find_map(|child| find_panel_node(Some(child), panel_id)),
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
    }
}

fn find_tab_for_panel(root: Option<&Node>, panel: &PanelId) -> Option<(NodeId, usize)> {
    fn walk(node: &Node, panel: &PanelId) -> Option<(NodeId, usize)> {
        match &node.kind {
            NodeKind::Tabs { items, .. } => {
                if let Some(index) = items.iter().position(
                    |child| matches!(&child.kind, NodeKind::Panel { panel: id } if id == panel),
                ) {
                    return Some((node.id.clone(), index));
                }
                items.iter().find_map(|child| walk(child, panel))
            }
            NodeKind::DocumentWell { groups, .. }
            | NodeKind::Split {
                children: groups, ..
            }
            | NodeKind::Extension {
                children: groups, ..
            } => groups.iter().find_map(|child| walk(child, panel)),
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
        }
    }
    root.and_then(|root| walk(root, panel))
}

fn tab_group_path_for_panel(root: Option<&Node>, panel: &PanelId) -> Vec<NodeId> {
    fn contains_panel(node: &Node, panel: &PanelId) -> bool {
        match &node.kind {
            NodeKind::Panel { panel: candidate } => candidate == panel,
            NodeKind::Tabs { items, .. }
            | NodeKind::DocumentWell { groups: items, .. }
            | NodeKind::Split {
                children: items, ..
            }
            | NodeKind::Extension {
                children: items, ..
            } => items.iter().any(|child| contains_panel(child, panel)),
            NodeKind::Placeholder { .. } => false,
        }
    }
    fn walk(node: &Node, panel: &PanelId, path: &mut Vec<NodeId>) -> bool {
        if !contains_panel(node, panel) {
            return false;
        }
        if matches!(node.kind, NodeKind::Tabs { .. }) {
            path.push(node.id.clone());
        }
        match &node.kind {
            NodeKind::Tabs { items, .. }
            | NodeKind::DocumentWell { groups: items, .. }
            | NodeKind::Split {
                children: items, ..
            }
            | NodeKind::Extension {
                children: items, ..
            } => items.iter().any(|child| walk(child, panel, path)),
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => true,
        }
    }
    let mut path = Vec::new();
    if let Some(root) = root {
        walk(root, panel, &mut path);
    }
    path
}

fn find_tab_group(root: Option<&Node>, tabs: &NodeId) -> Option<(usize, usize)> {
    fn walk(node: &Node, tabs: &NodeId) -> Option<(usize, usize)> {
        if &node.id == tabs {
            return match &node.kind {
                NodeKind::Tabs {
                    items,
                    active_index,
                    ..
                } => Some((items.len(), *active_index)),
                _ => None,
            };
        }
        match &node.kind {
            NodeKind::Tabs { items, .. }
            | NodeKind::DocumentWell { groups: items, .. }
            | NodeKind::Split {
                children: items, ..
            }
            | NodeKind::Extension {
                children: items, ..
            } => items.iter().find_map(|child| walk(child, tabs)),
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
        }
    }
    root.and_then(|root| walk(root, tabs))
}

fn find_tabs_active_index(root: Option<&Node>, tabs: &NodeId) -> Option<usize> {
    fn walk(node: &Node, tabs: &NodeId) -> Option<usize> {
        if &node.id == tabs {
            return match &node.kind {
                NodeKind::Tabs { active_index, .. } => Some(*active_index),
                _ => None,
            };
        }
        match &node.kind {
            NodeKind::Tabs { items, .. }
            | NodeKind::DocumentWell { groups: items, .. }
            | NodeKind::Split {
                children: items, ..
            }
            | NodeKind::Extension {
                children: items, ..
            } => items.iter().find_map(|child| walk(child, tabs)),
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
        }
    }
    root.and_then(|root| walk(root, tabs))
}

fn source_profiles() -> Vec<WorkspaceProfile> {
    vec![
        make_profile(
            "default",
            "advanced",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "spl-root",
                Orientation::Horizontal,
                vec![
                    tabs(
                        "sessions-bots-tabs",
                        0,
                        vec![region(SESSIONS), region(BOTS)],
                    ),
                    region(WORKSPACE),
                    split(
                        "spl-right",
                        Orientation::Vertical,
                        vec![
                            split(
                                "spl-rail",
                                Orientation::Horizontal,
                                vec![region(REVIEW), region(FILES)],
                                vec![1.0, 1.2],
                            ),
                            region(TERMINAL),
                        ],
                        vec![1.6, 1.0],
                    ),
                ],
                vec![1.0, 3.4, 1.25],
            ),
        ),
        make_profile(
            "basic",
            "advanced",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "basic-root",
                Orientation::Horizontal,
                vec![
                    tabs(
                        "sessions-bots-tabs",
                        0,
                        vec![region(SESSIONS), region(BOTS)],
                    ),
                    split(
                        "basic-center",
                        Orientation::Vertical,
                        vec![region(WORKSPACE), region(TERMINAL)],
                        vec![3.0, 1.0],
                    ),
                    split(
                        "basic-right",
                        Orientation::Horizontal,
                        vec![region(REVIEW), region(FILES)],
                        vec![1.0, 1.2],
                    ),
                ],
                vec![1.0, 3.4, 1.25],
            ),
        ),
        make_profile(
            "focus",
            "advanced",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(FILES),
                optional(REVIEW),
                optional(TERMINAL),
            ],
            split(
                "focus-root",
                Orientation::Horizontal,
                vec![
                    tabs(
                        "sessions-bots-tabs",
                        0,
                        vec![region(SESSIONS), region(BOTS)],
                    ),
                    split(
                        "focus-main-terminal",
                        Orientation::Vertical,
                        vec![
                            tabs(
                                "focus-tabs",
                                0,
                                vec![region(WORKSPACE), region(FILES), region(REVIEW)],
                            ),
                            region(TERMINAL),
                        ],
                        vec![3.0, 1.0],
                    ),
                ],
                vec![1.0, 4.6],
            ),
        ),
        make_profile(
            "terminal-deck",
            "advanced",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(FILES),
                optional(REVIEW),
                optional(TERMINAL),
            ],
            split(
                "terminal-deck-root",
                Orientation::Vertical,
                vec![
                    split(
                        "terminal-deck-top",
                        Orientation::Horizontal,
                        vec![
                            tabs(
                                "sessions-bots-tabs",
                                0,
                                vec![region(SESSIONS), region(BOTS)],
                            ),
                            region(WORKSPACE),
                            tabs("terminal-deck-tabs", 0, vec![region(FILES), region(REVIEW)]),
                        ],
                        vec![1.0, 3.2, 1.2],
                    ),
                    region(TERMINAL),
                ],
                vec![3.0, 1.0],
            ),
        ),
        make_profile(
            "quad",
            "advanced",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(FILES),
                optional(TERMINAL),
                optional(REVIEW),
            ],
            split(
                "quad-root",
                Orientation::Vertical,
                vec![
                    split(
                        "quad-top",
                        Orientation::Horizontal,
                        vec![
                            tabs(
                                "quad-left-tabs",
                                0,
                                vec![region(SESSIONS), region(BOTS), region(FILES)],
                            ),
                            region(WORKSPACE),
                        ],
                        vec![1.0, 3.0],
                    ),
                    split(
                        "quad-bottom",
                        Orientation::Horizontal,
                        vec![region(TERMINAL), region(REVIEW)],
                        vec![1.4, 1.0],
                    ),
                ],
                vec![3.0, 1.0],
            ),
        ),
        make_profile(
            "sidebar-left",
            "simple",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "simple-left-root",
                Orientation::Horizontal,
                vec![
                    tabs(
                        "sessions-bots-tabs",
                        0,
                        vec![region(SESSIONS), region(BOTS)],
                    ),
                    split(
                        "simple-left-center",
                        Orientation::Vertical,
                        vec![region(WORKSPACE), region(TERMINAL)],
                        vec![3.0, 1.0],
                    ),
                    split(
                        "simple-left-tools",
                        Orientation::Horizontal,
                        vec![region(REVIEW), region(FILES)],
                        vec![1.0, 1.2],
                    ),
                ],
                vec![1.0, 3.4, 1.25],
            ),
        ),
        make_profile(
            "sidebar-right",
            "simple",
            &[
                required(SESSIONS),
                required(WORKSPACE),
                optional(BOTS),
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "simple-right-root",
                Orientation::Horizontal,
                vec![
                    split(
                        "simple-right-tools",
                        Orientation::Horizontal,
                        vec![region(FILES), region(REVIEW)],
                        vec![1.2, 1.0],
                    ),
                    split(
                        "simple-right-center",
                        Orientation::Vertical,
                        vec![region(WORKSPACE), region(TERMINAL)],
                        vec![3.0, 1.0],
                    ),
                    tabs(
                        "sessions-bots-tabs",
                        0,
                        vec![region(SESSIONS), region(BOTS)],
                    ),
                ],
                vec![1.25, 3.4, 1.0],
            ),
        ),
    ]
}

fn make_profile(
    id: &str,
    category: &str,
    regions: &[(&str, &str, bool)],
    layout: ProfileLayout,
) -> WorkspaceProfile {
    WorkspaceProfile {
        id: id.into(),
        version: 1,
        category: category.into(),
        title_key: format!("layout.{id}"),
        regions: regions
            .iter()
            .map(|(id, title, required)| ProfileRegion {
                id: (*id).into(),
                title_key: (*title).into(),
                required: *required,
            })
            .collect(),
        layout,
        presentation: ProfilePresentation::Standard,
    }
}

fn required(id: &'static str) -> (&'static str, &'static str, bool) {
    (id, id, true)
}

fn optional(id: &'static str) -> (&'static str, &'static str, bool) {
    (id, id, false)
}

fn region(id: &str) -> ProfileLayout {
    ProfileLayout::Region(id.into())
}

fn split(
    id: &str,
    orientation: Orientation,
    children: Vec<ProfileLayout>,
    weights: Vec<f64>,
) -> ProfileLayout {
    ProfileLayout::Split {
        id: id.into(),
        orientation,
        weights,
        children,
    }
}

fn tabs(id: &str, active_index: usize, regions: Vec<ProfileLayout>) -> ProfileLayout {
    ProfileLayout::Tabs {
        id: id.into(),
        active_index,
        regions,
    }
}

fn profile<'a>(profiles: &'a [WorkspaceProfile], id: &str) -> Result<&'a WorkspaceProfile, String> {
    profiles
        .iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| format!("Unknown Hermes layout profile: {id}"))
}

fn base_document() -> LayoutDocument {
    let mut document = LayoutDocument::new("hermes-native-main", "hermes-native-default");
    let panels = [
        (SESSIONS, "Sessions"),
        (BOTS, "Bots"),
        (WORKSPACE, "New session"),
        (REVIEW, "Review"),
        (FILES, "Files"),
        (TERMINAL, "Terminal"),
    ];
    let descriptors = panels
        .iter()
        .map(|(id, title)| {
            let mut descriptor = PanelDescriptor::new(*id, *title);
            descriptor.minimum =
                doofusdock_core::Size::new(if *id == BOTS { 260 } else { 180 }, 140);
            descriptor.close_policy = if matches!(*id, SESSIONS | BOTS | WORKSPACE) {
                ClosePolicy::HideOnly
            } else {
                ClosePolicy::Allowed
            };
            descriptor
        })
        .collect::<Vec<_>>();
    Arc::make_mut(&mut document.panels).extend(descriptors);
    document
        .closed_panels
        .extend(panels.iter().map(|(id, _)| PanelId::from(*id)));
    document
}

fn data_directory() -> PathBuf {
    std::env::var_os("HERMES_NATIVE_PROFILE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .map(|path| path.join("HermesNative"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("HermesNative"))
}

fn read_preferences(path: &std::path::Path) -> Option<UiPreferences> {
    let bytes = fs::read(path).ok()?;
    let mut preferences: UiPreferences = serde_json::from_slice(&bytes).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if value.get("file_browser_open").is_none() {
        preferences.file_browser_open = preferences.pane_open.get(FILES).copied().unwrap_or(false);
    }
    Some(preferences)
}

fn write_preferences_atomic(
    path: &std::path::Path,
    preferences: &UiPreferences,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Preferences path has no parent".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(preferences).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    let parsed: UiPreferences =
        serde_json::from_slice(&fs::read(&temporary).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if parsed.advanced_profile.is_empty() {
        let _ = fs::remove_file(&temporary);
        return Err("Refusing invalid empty advanced profile".into());
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        error.to_string()
    })
}

fn remote_gateway_connection_id(normalized_endpoint: &str) -> String {
    let digest = Sha256::digest(normalized_endpoint.as_bytes());
    let suffix = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("remote-{suffix}")
}

fn upsert_gateway_connection(
    connections: &mut Vec<GatewayConnectionDescriptor>,
    descriptor: GatewayConnectionDescriptor,
) {
    if let Some(existing) = connections
        .iter_mut()
        .find(|existing| existing.connection_id == descriptor.connection_id)
    {
        *existing = descriptor;
    } else {
        connections.push(descriptor);
    }
}

fn migrate_gateway_connection_preferences(
    preferences: &mut UiPreferences,
    selected_connection_id_was_persisted: bool,
) -> bool {
    let mut changed = false;
    if !preferences
        .gateway_connections
        .iter()
        .any(|connection| connection.kind == GatewayConnectionKind::Local)
    {
        preferences
            .gateway_connections
            .insert(0, GatewayConnectionDescriptor::local());
        changed = true;
    }

    if let Some(legacy_url) = preferences.last_remote_gateway_url.clone()
        && let Ok(descriptor) = GatewayConnectionDescriptor::remote(&legacy_url)
    {
        if preferences.last_remote_gateway_url.as_deref() != descriptor.endpoint_url.as_deref() {
            preferences.last_remote_gateway_url = descriptor.endpoint_url.clone();
            changed = true;
        }
        let remote_was_present = preferences
            .gateway_connections
            .iter()
            .any(|connection| connection.connection_id == descriptor.connection_id);
        if !remote_was_present {
            preferences.gateway_connections.push(descriptor.clone());
            changed = true;
        }
        if !selected_connection_id_was_persisted
            || (!remote_was_present
                && preferences.selected_gateway_connection_id == LOCAL_GATEWAY_CONNECTION_ID)
        {
            preferences.selected_gateway_connection_id = descriptor.connection_id;
            changed = true;
        }
    }

    let mut seen = BTreeSet::new();
    let before = preferences.gateway_connections.len();
    preferences
        .gateway_connections
        .retain(|connection| seen.insert(connection.connection_id.clone()));
    changed |= preferences.gateway_connections.len() != before;

    if !preferences
        .gateway_connections
        .iter()
        .any(|connection| connection.connection_id == preferences.selected_gateway_connection_id)
    {
        preferences.selected_gateway_connection_id = LOCAL_GATEWAY_CONNECTION_ID.into();
        changed = true;
    }
    changed
}

fn validate_auto_project_id(project_id: &str) -> Result<(), String> {
    if project_id.is_empty() || project_id.len() > 1024 || project_id.chars().any(char::is_control)
    {
        return Err("Auto-project ID is invalid".into());
    }
    Ok(())
}

fn validate_auto_project_ids(ids: &[String]) -> Result<(), String> {
    if ids.len() > AUTO_PROJECT_DISMISSAL_LIMIT {
        return Err(format!(
            "Dismissed auto-project list exceeds {AUTO_PROJECT_DISMISSAL_LIMIT} entries"
        ));
    }
    let mut unique = BTreeSet::new();
    for id in ids {
        validate_auto_project_id(id)?;
        if !unique.insert(id.as_str()) {
            return Err("Dismissed auto-project list contains duplicate IDs".into());
        }
    }
    Ok(())
}

fn read_auto_project_dismissal_state(path: &Path) -> Result<Option<Vec<String>>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("inspect shared auto-project state: {error}")),
    };
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("Shared auto-project state is not a regular file".into());
    }
    if metadata.len() > AUTO_PROJECT_STATE_BYTES_LIMIT {
        return Err("Shared auto-project state exceeds the 1 MiB limit".into());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("read shared auto-project state: {error}"))?;
    let state: AutoProjectDismissalState = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse shared auto-project state: {error}"))?;
    if state.version != 1 {
        return Err("Shared auto-project state has an unsupported version".into());
    }
    validate_auto_project_ids(&state.ids)?;
    Ok(Some(state.ids))
}

fn write_auto_project_dismissal_state(path: &Path, ids: &[String]) -> Result<(), String> {
    validate_auto_project_ids(ids)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Shared auto-project state has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("create shared auto-project state directory: {error}"))?;
    let state = AutoProjectDismissalState {
        version: 1,
        ids: ids.to_vec(),
    };
    let bytes = serde_json::to_vec(&state)
        .map_err(|error| format!("encode shared auto-project state: {error}"))?;
    if bytes.len() as u64 > AUTO_PROJECT_STATE_BYTES_LIMIT {
        return Err("Shared auto-project state exceeds the 1 MiB limit".into());
    }
    let sequence = AUTO_PROJECT_STATE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("json.tmp-{}-{sequence}", std::process::id()));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("create shared auto-project temp file: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("write shared auto-project temp file: {error}"))?;
        let parsed = read_auto_project_dismissal_state(&temporary)?
            .ok_or_else(|| "shared auto-project temp file disappeared".to_owned())?;
        if parsed != ids {
            return Err("shared auto-project temp file failed readback validation".into());
        }
        fs::rename(&temporary, path)
            .map_err(|error| format!("publish shared auto-project state: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn unix_time_millis() -> Result<u128, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))
}

fn acquire_auto_project_dismissal_lock(
    path: &Path,
) -> Result<AutoProjectDismissalLockGuard, String> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| "Shared auto-project state has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("create shared auto-project state directory: {error}"))?;
    let lock_path = path.with_extension("lock");
    let deadline = std::time::Instant::now() + AUTO_PROJECT_LOCK_WAIT;
    loop {
        let now = unix_time_millis()?;
        let record = AutoProjectDismissalLock {
            version: 1,
            process_id: std::process::id(),
            expires_unix_ms: now + AUTO_PROJECT_LOCK_TTL.as_millis(),
        };
        let bytes = serde_json::to_vec(&record)
            .map_err(|error| format!("encode auto-project lock receipt: {error}"))?;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
                    let _ = fs::remove_file(&lock_path);
                    return Err(format!("write auto-project lock receipt: {error}"));
                }
                return Ok(AutoProjectDismissalLockGuard { path: lock_path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let expired = fs::read(&lock_path)
                    .ok()
                    .and_then(|bytes| {
                        serde_json::from_slice::<AutoProjectDismissalLock>(&bytes).ok()
                    })
                    .is_some_and(|lock| lock.version == 1 && lock.expires_unix_ms <= now);
                let malformed_stale = fs::metadata(&lock_path)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age >= AUTO_PROJECT_LOCK_TTL);
                if expired || malformed_stale {
                    match fs::remove_file(&lock_path) {
                        Ok(()) => continue,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(error) => {
                            return Err(format!("recover stale auto-project lock: {error}"));
                        }
                    }
                }
                if std::time::Instant::now() >= deadline {
                    return Err(
                        "timed out waiting for shared auto-project state lock after 2 seconds"
                            .into(),
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => return Err(format!("acquire shared auto-project state lock: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_edge_toggle_minimizes_and_restores_only_the_outer_group() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-positional-edge-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_fallback_at(directory.clone());
        let mut document = base_document();
        let files = PanelId::from(FILES);
        let terminal = PanelId::from(TERMINAL);
        let main = PanelId::from(WORKSPACE);
        document
            .closed_panels
            .retain(|panel| panel != &files && panel != &terminal && panel != &main);
        document.root = Some(Node::split(
            "root-row",
            Orientation::Horizontal,
            vec![
                Node::tabs(
                    "left-group",
                    vec![
                        Node::panel("files-node", files),
                        Node::panel("terminal-node", terminal),
                    ],
                    1,
                ),
                Node::panel("main-node", main),
            ],
            vec![0.35, 0.65],
        ));
        workspace.dock = DockingService::new(document, 32).unwrap();

        assert!(!workspace.toggle_positional_edge(false).unwrap());
        assert!(workspace.toggle_positional_edge(true).unwrap());
        let folded = workspace.dock.snapshot();
        let NodeKind::Split { children, .. } = &folded.document.root.as_ref().unwrap().kind else {
            panic!("root row remains a split");
        };
        let NodeKind::Tabs {
            active_index,
            minimized,
            ..
        } = &children[0].kind
        else {
            panic!("left root edge remains a tab group");
        };
        assert!(*minimized);
        assert_eq!(*active_index, 1);

        assert!(workspace.toggle_positional_edge(true).unwrap());
        let restored = workspace.dock.snapshot();
        let NodeKind::Split { children, .. } = &restored.document.root.as_ref().unwrap().kind
        else {
            panic!("root row remains a split");
        };
        let NodeKind::Tabs {
            active_index,
            minimized,
            ..
        } = &children[0].kind
        else {
            panic!("left root edge remains a tab group");
        };
        assert!(!minimized);
        assert_eq!(*active_index, 1);
        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn semantic_file_visibility_restores_the_same_right_edge_group() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-semantic-side-restore-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_fallback_at(directory.clone());
        workspace.preferences.file_browser_open = true;
        workspace.preferences.pane_open.insert(FILES.into(), true);
        let files = PanelId::from(FILES);
        let terminal = PanelId::from(TERMINAL);
        let main = PanelId::from(WORKSPACE);
        let mut document = base_document();
        document
            .closed_panels
            .retain(|panel| panel != &files && panel != &terminal && panel != &main);
        document.root = Some(Node::split(
            "root-row",
            Orientation::Horizontal,
            vec![
                Node::panel("main-node", main),
                Node::tabs(
                    "right-group",
                    vec![
                        Node::panel("files-node", files),
                        Node::panel("terminal-node", terminal),
                    ],
                    1,
                ),
            ],
            vec![0.65, 0.35],
        ));
        workspace.dock = DockingService::new(document, 32).unwrap();

        assert!(workspace.toggle_semantic_side(FILES).unwrap());
        assert!(!workspace.preferences.file_browser_open);
        assert!(!workspace.pane_is_open(FILES));
        let canonical_while_hidden = workspace.dock.snapshot().document;
        assert_eq!(
            canonical_while_hidden.root.as_ref().unwrap().id.0,
            "root-row"
        );
        let NodeKind::Split { children, .. } = &canonical_while_hidden.root.as_ref().unwrap().kind
        else {
            panic!("canonical root remains the same split");
        };
        let NodeKind::Tabs {
            active_index,
            minimized,
            ..
        } = &children[1].kind
        else {
            panic!("right edge group remains canonical tabs");
        };
        assert!(!minimized);
        assert_eq!(*active_index, 1);
        let presentation = workspace.presentation_layout(canonical_while_hidden.clone());
        let NodeKind::Panel { panel } = &presentation.root.as_ref().unwrap().kind else {
            panic!("only the visible main pane is presented while Files is hidden");
        };
        assert_eq!(panel.0, WORKSPACE);

        assert!(workspace.toggle_positional_edge(false).unwrap());
        assert!(workspace.preferences.file_browser_open);
        assert!(workspace.pane_is_open(FILES));
        let restored = workspace.dock.snapshot().document;
        assert_eq!(restored.root.as_ref().unwrap().id.0, "root-row");
        let NodeKind::Split { children, .. } = &restored.root.as_ref().unwrap().kind else {
            panic!("restored root remains the same split");
        };
        let NodeKind::Tabs {
            active_index,
            minimized,
            ..
        } = &children[1].kind
        else {
            panic!("restored right edge group remains tabs");
        };
        assert!(!minimized);
        assert_eq!(*active_index, 1);
        let persisted: UiPreferences =
            serde_json::from_slice(&std::fs::read(&workspace.preferences_path).unwrap()).unwrap();
        assert!(persisted.file_browser_open);
        assert!(!persisted.collapsed_sides.contains(&PhysicalSide::Right));

        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn semantic_file_cross_restore_supports_main_row_inside_column_root() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-semantic-column-root-restore-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_fallback_at(directory.clone());
        workspace.preferences.file_browser_open = true;
        workspace.preferences.pane_open.insert(FILES.into(), true);
        let files = PanelId::from(FILES);
        let terminal = PanelId::from(TERMINAL);
        let main = PanelId::from(WORKSPACE);
        let review = PanelId::from(REVIEW);
        let mut document = base_document();
        document.closed_panels.retain(|panel| {
            panel != &files && panel != &terminal && panel != &main && panel != &review
        });
        document.root = Some(Node::split(
            "outer-column",
            Orientation::Vertical,
            vec![
                Node::split(
                    "root-row",
                    Orientation::Horizontal,
                    vec![
                        Node::panel("main-node", main),
                        Node::tabs(
                            "right-group",
                            vec![
                                Node::panel("files-node", files),
                                Node::panel("terminal-node", terminal),
                            ],
                            1,
                        ),
                    ],
                    vec![0.65, 0.35],
                ),
                Node::panel("bottom-review", review),
            ],
            vec![0.8, 0.2],
        ));
        workspace.dock = DockingService::new(document, 32).unwrap();

        assert!(workspace.toggle_semantic_side(FILES).unwrap());
        let canonical = workspace.dock.snapshot().document;
        assert!(
            workspace
                .preferences
                .collapsed_sides
                .contains(&PhysicalSide::Right)
        );
        let projected = workspace.presentation_layout(canonical.clone());
        let NodeKind::Split {
            orientation: Orientation::Vertical,
            children,
            ..
        } = &projected.root.as_ref().unwrap().kind
        else {
            panic!("outer column layout is retained by the presentation projection");
        };
        let NodeKind::Panel { panel } = &children[0].kind else {
            panic!("the hidden right edge is removed from the nested main row");
        };
        assert_eq!(panel.0, WORKSPACE);
        assert_eq!(children[1].id.0, "bottom-review");

        assert!(workspace.toggle_positional_edge(false).unwrap());
        assert!(workspace.preferences.file_browser_open);
        assert!(
            !workspace
                .preferences
                .collapsed_sides
                .contains(&PhysicalSide::Right)
        );
        let restored = workspace.dock.snapshot().document;
        assert_eq!(restored.root.as_ref().unwrap().id.0, "outer-column");
        let NodeKind::Split {
            orientation: Orientation::Vertical,
            children,
            ..
        } = &restored.root.as_ref().unwrap().kind
        else {
            panic!("outer column root is retained after restoring Files");
        };
        let NodeKind::Split {
            orientation: Orientation::Horizontal,
            children: row,
            ..
        } = &children[0].kind
        else {
            panic!("the canonical nested root row remains intact");
        };
        assert_eq!(row[0].id.0, "main-node");
        let NodeKind::Tabs {
            active_index,
            minimized,
            ..
        } = &row[1].kind
        else {
            panic!("the original right-side tab group is restored");
        };
        assert!(!minimized);
        assert_eq!(*active_index, 1);

        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn nested_physical_edge_split_falls_back_to_semantic_file_toggle() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-nested-edge-fallback-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_fallback_at(directory.clone());
        workspace.preferences.file_browser_open = true;
        workspace.preferences.pane_open.insert(FILES.into(), true);
        let files = PanelId::from(FILES);
        let terminal = PanelId::from(TERMINAL);
        let main = PanelId::from(WORKSPACE);
        let mut document = base_document();
        document
            .closed_panels
            .retain(|panel| panel != &files && panel != &terminal && panel != &main);
        document.root = Some(Node::split(
            "root-row",
            Orientation::Horizontal,
            vec![
                Node::panel("main-node", main),
                Node::split(
                    "right-nested-split",
                    Orientation::Vertical,
                    vec![
                        Node::panel("files-node", files),
                        Node::panel("terminal-node", terminal),
                    ],
                    vec![0.7, 0.3],
                ),
            ],
            vec![0.65, 0.35],
        ));
        workspace.dock = DockingService::new(document, 32).unwrap();

        // A nested split is not a foldable leaf group, so the titlebar handler
        // uses the same semantic Files fallback as the pinned Desktop.
        assert!(!workspace.toggle_positional_edge(false).unwrap());
        assert!(workspace.toggle_semantic_side(FILES).unwrap());
        assert!(!workspace.preferences.file_browser_open);
        let hidden = workspace.dock.snapshot().document;
        let projected = workspace.presentation_layout(hidden);
        let NodeKind::Panel { panel } = &projected.root.as_ref().unwrap().kind else {
            panic!("nested right side is hidden in the presentation projection");
        };
        assert_eq!(panel.0, WORKSPACE);

        assert!(!workspace.toggle_positional_edge(false).unwrap());
        assert!(workspace.toggle_semantic_side(FILES).unwrap());
        assert!(workspace.preferences.file_browser_open);
        let restored = workspace.dock.snapshot().document;
        let NodeKind::Split { children, .. } = &restored.root.as_ref().unwrap().kind else {
            panic!("semantic fallback preserves the canonical root row");
        };
        assert_eq!(children[1].id.0, "right-nested-split");

        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn last_remote_gateway_url_is_normalized_and_survives_workspace_reload() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-remote-gateway-preference-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        workspace
            .save_remote_gateway_url(" HTTPS://example.com/hermes/ ")
            .unwrap();
        assert_eq!(
            workspace.last_remote_gateway_url(),
            Some("https://example.com/hermes")
        );
        assert!(workspace.save_remote_gateway_url("file:///unsafe").is_err());
        drop(workspace);
        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            reloaded.last_remote_gateway_url(),
            Some("https://example.com/hermes")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_remote_gateway_preference_migrates_to_selected_connection() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-legacy-gateway-migration-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("ui-preferences.json"),
            br#"{"last_remote_gateway_url":" HTTPS://example.org/hermes/ "}"#,
        )
        .unwrap();

        let workspace = Workspace::load_from(directory.clone()).unwrap();
        let remote = workspace
            .gateway_connections()
            .iter()
            .find(|connection| connection.kind == GatewayConnectionKind::Remote)
            .expect("legacy remote URL is migrated to a connection descriptor");
        assert_eq!(
            remote.endpoint_url.as_deref(),
            Some("https://example.org/hermes")
        );
        assert_eq!(
            workspace.selected_gateway_connection_id(),
            remote.connection_id
        );
        assert!(workspace.gateway_connections().iter().any(|connection| {
            connection.kind == GatewayConnectionKind::Local
                && connection.connection_id == LOCAL_GATEWAY_CONNECTION_ID
        }));
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("ui-preferences.json")).unwrap())
                .unwrap();
        assert_eq!(
            persisted["selected_gateway_connection_id"],
            remote.connection_id
        );
        assert_eq!(
            persisted["gateway_connections"][1]["endpoint_url"],
            "https://example.org/hermes"
        );
        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn remote_gateway_connection_id_is_stable_across_workspace_reload() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-stable-gateway-id-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        workspace
            .save_remote_gateway_url("https://gateway.example.net/hermes/")
            .unwrap();
        let first_id = workspace.selected_gateway_connection_id().to_owned();
        assert!(first_id.starts_with("remote-"));
        drop(workspace);

        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(reloaded.selected_gateway_connection_id(), first_id);
        assert!(
            reloaded
                .gateway_connections()
                .iter()
                .any(|connection| connection.connection_id == first_id)
        );
        drop(reloaded);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn multiple_gateway_connection_descriptors_persist_independently() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-multiple-gateway-connections-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let staging =
            GatewayConnectionDescriptor::remote("https://staging.example.net/hermes").unwrap();
        let production =
            GatewayConnectionDescriptor::remote("https://production.example.net/hermes").unwrap();
        workspace
            .preferences
            .gateway_connections
            .extend([staging.clone(), production.clone()]);
        workspace.preferences.selected_gateway_connection_id = staging.connection_id.clone();
        workspace.save_preferences();
        drop(workspace);

        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        let ids = reloaded
            .gateway_connections()
            .iter()
            .map(|connection| connection.connection_id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(LOCAL_GATEWAY_CONNECTION_ID));
        assert!(ids.contains(staging.connection_id.as_str()));
        assert!(ids.contains(production.connection_id.as_str()));
        assert_eq!(
            reloaded.selected_gateway_connection_id(),
            staging.connection_id
        );
        drop(reloaded);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn gateway_connection_descriptor_serializes_only_non_secret_metadata() {
        let descriptor =
            GatewayConnectionDescriptor::remote("https://gateway.example.net/hermes").unwrap();
        let value = serde_json::to_value(&descriptor).unwrap();
        let object = value.as_object().expect("descriptor is an object");
        let keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
        assert_eq!(
            keys,
            BTreeSet::from(["connection_id", "display_label", "endpoint_url", "kind"])
        );
        for secret_key in [
            "token",
            "access_token",
            "refresh_token",
            "password",
            "secret",
        ] {
            assert!(!object.contains_key(secret_key));
        }
        assert_eq!(value["kind"], "remote");
        assert_eq!(value["endpoint_url"], "https://gateway.example.net/hermes");
    }

    #[test]
    fn show_all_project_sessions_preference_survives_workspace_reload() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-project-show-all-preference-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        assert!(!workspace.preferences.show_all_project_sessions);
        workspace.set_show_all_project_sessions(true).unwrap();
        drop(workspace);
        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert!(reloaded.preferences.show_all_project_sessions);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn dismissed_auto_projects_persist_and_restore_idempotently() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-dismissed-auto-projects-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        assert!(
            workspace
                .set_auto_project_dismissed("C:/work/repo", true)
                .unwrap()
        );
        assert!(
            !workspace
                .set_auto_project_dismissed("C:/work/repo", true)
                .unwrap()
        );
        drop(workspace);

        let mut reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            reloaded.preferences.dismissed_auto_project_ids,
            ["C:/work/repo"]
        );
        assert!(
            reloaded
                .set_auto_project_dismissed("C:/work/repo", false)
                .unwrap()
        );
        assert!(
            !reloaded
                .set_auto_project_dismissed("C:/work/repo", false)
                .unwrap()
        );
        assert!(reloaded.preferences.dismissed_auto_project_ids.is_empty());
        assert!(
            reloaded
                .set_auto_project_dismissed("bad\nid", true)
                .is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn auto_project_dismissals_are_shared_across_window_workspaces() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-shared-auto-projects-{}-{nonce}",
            std::process::id()
        ));
        let mut main = Workspace::load_from(directory.clone()).unwrap();
        let mut child = Workspace::load_window_from(directory.clone(), 73).unwrap();

        main.set_auto_project_dismissed("C:/work/one", true)
            .unwrap();
        assert!(child.refresh_auto_project_dismissals().unwrap());
        assert_eq!(
            child.preferences.dismissed_auto_project_ids,
            ["C:/work/one"]
        );
        child
            .set_auto_project_dismissed("C:/work/two", true)
            .unwrap();
        assert!(main.refresh_auto_project_dismissals().unwrap());
        assert_eq!(
            main.preferences.dismissed_auto_project_ids,
            ["C:/work/one", "C:/work/two"],
            "a stale window writer must merge with the latest shared snapshot"
        );
        child
            .set_auto_project_dismissed("C:/work/one", false)
            .unwrap();
        assert!(main.refresh_auto_project_dismissals().unwrap());
        assert_eq!(main.preferences.dismissed_auto_project_ids, ["C:/work/two"]);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn auto_project_state_recovers_expired_lock_and_rejects_invalid_state_atomically() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-auto-project-lock-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let state_path = workspace.dismissed_auto_project_ids_path.clone();
        let lock_path = state_path.with_extension("lock");
        std::fs::write(
            &lock_path,
            serde_json::to_vec(&AutoProjectDismissalLock {
                version: 1,
                process_id: u32::MAX,
                expires_unix_ms: 0,
            })
            .unwrap(),
        )
        .unwrap();
        assert!(
            workspace
                .set_auto_project_dismissed("C:/work/recovered", true)
                .unwrap()
        );
        assert!(!lock_path.exists(), "recovered stale lock is released");

        let before = workspace.preferences.dismissed_auto_project_ids.clone();
        std::fs::remove_file(&state_path).unwrap();
        std::fs::create_dir(&state_path).unwrap();
        assert!(
            workspace
                .set_auto_project_dismissed("C:/work/blocked", true)
                .is_err()
        );
        assert_eq!(workspace.preferences.dismissed_auto_project_ids, before);
        assert!(
            workspace
                .set_auto_project_dismissed("bad\nid", true)
                .is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn concurrent_auto_project_dismissal_updates_do_not_clobber() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-auto-project-concurrent-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let state_path = directory.join("dismissed-auto-projects.json");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let mut workers = Vec::new();
        for id in ["C:/work/one", "C:/work/two"] {
            let path = state_path.clone();
            let barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                let _lock = acquire_auto_project_dismissal_lock(&path).unwrap();
                let mut ids = read_auto_project_dismissal_state(&path)
                    .unwrap()
                    .unwrap_or_default();
                if !ids.iter().any(|existing| existing == id) {
                    ids.push(id.to_owned());
                }
                write_auto_project_dismissal_state(&path, &ids).unwrap();
            }));
        }
        barrier.wait();
        for worker in workers {
            worker.join().unwrap();
        }
        let ids = read_auto_project_dismissal_state(&state_path)
            .unwrap()
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"C:/work/one".to_owned()));
        assert!(ids.contains(&"C:/work/two".to_owned()));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn separate_window_uses_distinct_layout_and_preferences_but_inherits_gateway_url() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-window-workspace-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let mut main_preferences = UiPreferences::default();
        main_preferences.last_remote_gateway_url = Some("https://shared.example".into());
        write_preferences_atomic(&directory.join("ui-preferences.json"), &main_preferences)
            .unwrap();

        let mut window = Workspace::load_window_from(directory.clone(), 73).unwrap();
        assert_eq!(
            window.last_remote_gateway_url(),
            Some("https://shared.example")
        );
        window
            .save_remote_gateway_url("https://window.example")
            .unwrap();
        window.save_layout();

        let mut main = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            main.last_remote_gateway_url(),
            Some("https://shared.example")
        );
        main.save_layout();
        assert!(
            main.persistence_error.is_none(),
            "{:?}",
            main.persistence_error
        );
        assert!(
            main.store
                .load(&LayoutId::from("hermes-native-main"))
                .is_ok(),
            "main layout should use its fixed storage key: {:?}",
            main.store.load(&LayoutId::from("hermes-native-main"))
        );
        assert!(
            window
                .store
                .load(&LayoutId::from("hermes-native-window-73"))
                .is_ok(),
            "window layout should use its unique storage key"
        );
        let window_reloaded = Workspace::load_window_from(directory.clone(), 73).unwrap();
        assert_eq!(
            window_reloaded.last_remote_gateway_url(),
            Some("https://window.example")
        );
        assert!(
            window_reloaded
                .store
                .load(&LayoutId::from("hermes-native-window-73"))
                .is_ok()
        );
        drop(window_reloaded);
        drop(main);
        drop(window);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pinned_session_order_persists_per_gateway_without_profile_fragmentation() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-pinned-order-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        workspace
            .set_pinned_session_state("loopback|gateway-a", "session-b", true)
            .unwrap();
        workspace
            .set_pinned_session_state("loopback|gateway-a", "session-a", true)
            .unwrap();
        workspace
            .set_pinned_session_state("loopback|gateway-b", "session-c", true)
            .unwrap();
        workspace
            .set_pinned_session_state("loopback|gateway-a", "session-b", false)
            .unwrap();
        assert_eq!(
            workspace.pinned_session_order("loopback|gateway-a"),
            ["session-a"]
        );
        assert_eq!(
            workspace.pinned_session_order("loopback|gateway-b"),
            ["session-c"]
        );
        drop(workspace);
        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            reloaded.pinned_session_order("loopback|gateway-a"),
            ["session-a"]
        );
        assert_eq!(
            reloaded.pinned_session_order("loopback|gateway-b"),
            ["session-c"]
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pinned_session_reorder_preserves_unresolved_slots_and_adds_server_pins() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-pinned-reorder-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        workspace
            .set_pinned_session_order(
                "loopback|gateway-a",
                vec!["a".into(), "unresolved".into(), "b".into(), "c".into()],
            )
            .unwrap();
        workspace
            .reorder_pinned_session_order(
                "loopback|gateway-a",
                vec!["c".into(), "a".into(), "server-pin".into(), "b".into()],
            )
            .unwrap();
        assert_eq!(
            workspace.pinned_session_order("loopback|gateway-a"),
            ["c", "unresolved", "a", "server-pin", "b"]
        );
        assert!(
            workspace
                .reorder_pinned_session_order("loopback|gateway-a", vec!["c".into(), "c".into()])
                .is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recent_session_order_persists_per_profile_and_rejects_duplicates() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-recent-order-test-{}-{nonce}",
            std::process::id()
        ));
        let primary_scope = "http://127.0.0.1:8787|gateway-a|profile:research";
        let other_profile_scope = "http://127.0.0.1:8787|gateway-a|profile:work";
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        workspace
            .set_sidebar_session_order(primary_scope, vec!["b".into(), "a".into()])
            .unwrap();
        workspace
            .set_sidebar_session_order(other_profile_scope, vec!["c".into()])
            .unwrap();
        assert!(
            workspace
                .set_sidebar_session_order(
                    primary_scope,
                    vec!["duplicate".into(), "duplicate".into()]
                )
                .is_err()
        );
        drop(workspace);

        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(reloaded.sidebar_session_order(primary_scope), ["b", "a"]);
        assert_eq!(reloaded.sidebar_session_order(other_profile_scope), ["c"]);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn session_color_overrides_persist_and_reject_non_palette_values() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-session-color-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        workspace
            .set_session_color("session-a", Some("hsl(120 68% 58%)"))
            .unwrap();
        assert!(
            workspace
                .set_session_color("session-a", Some("#ff0000"))
                .is_err()
        );
        assert!(
            workspace
                .set_session_color("", Some("hsl(0 68% 58%)"))
                .is_err()
        );
        drop(workspace);

        let mut reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            reloaded.session_color("session-a"),
            Some("hsl(120 68% 58%)")
        );
        reloaded.set_session_color("session-a", None).unwrap();
        assert_eq!(reloaded.session_color("session-a"), None);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn file_preview_tabs_are_transient_and_do_not_enter_persisted_layouts() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-file-preview-test-{}-{nonce}",
            std::process::id()
        ));
        let files = directory.join("project");
        std::fs::create_dir_all(&files).unwrap();
        let first_path = files.join("README.md");
        let second_path = files.join("Cargo.toml");
        std::fs::write(&first_path, "# Sample").unwrap();
        std::fs::write(&second_path, "[package]").unwrap();

        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let first = workspace.open_file_preview(&first_path).unwrap();
        let second = workspace.open_file_preview(&second_path).unwrap();
        let live = workspace.dock.snapshot().document;
        assert_eq!(live.descriptor(&first).unwrap().role, "hermes_file_preview");
        assert_eq!(
            live.descriptor(&second).unwrap().role,
            "hermes_file_preview"
        );
        assert_eq!(
            find_tab_for_panel(live.root.as_deref(), &first).unwrap().0,
            find_tab_for_panel(live.root.as_deref(), &second).unwrap().0,
            "subsequent file previews must share the preview tab group"
        );

        workspace.save_layout();
        assert!(workspace.persistence_error.is_none());
        let persisted = workspace
            .store
            .load(&LayoutId::from("hermes-native-main"))
            .unwrap();
        assert!(persisted.descriptor(&first).is_none());
        assert!(persisted.descriptor(&second).is_none());
        assert!(find_panel_node(persisted.root.as_deref(), &first.0).is_none());
        assert!(find_panel_node(persisted.root.as_deref(), &second.0).is_none());

        workspace.close_file_preview(&first).unwrap();
        assert!(
            workspace
                .dock
                .snapshot()
                .document
                .closed_panels
                .contains(&first)
        );
        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn file_preview_identity_is_stable_and_path_specific() {
        let first = file_preview_panel_id("C:/project/README.md");
        let same = file_preview_panel_id("C:/project/README.md");
        let second = file_preview_panel_id("C:/project/Cargo.toml");
        assert_eq!(first, same);
        assert_ne!(first, second);
    }

    #[test]
    fn select_panel_uses_canonical_tabs_and_is_idempotent() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-select-panel-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let session = workspace
            .open_session_pane(Some("work"), "session-id", "Work session")
            .unwrap();
        let snapshot = workspace.dock.snapshot();
        let (tabs, session_index) =
            find_tab_for_panel(snapshot.document.root.as_deref(), &session).unwrap();
        assert_eq!(
            find_tabs_active_index(snapshot.document.root.as_deref(), &tabs),
            Some(session_index)
        );

        workspace.select_panel(&PanelId::from(WORKSPACE)).unwrap();
        let selected = workspace.dock.snapshot();
        assert_eq!(
            find_tabs_active_index(selected.document.root.as_deref(), &tabs),
            Some(0)
        );
        let revision = selected.revision;
        workspace.select_panel(&PanelId::from(WORKSPACE)).unwrap();
        assert_eq!(workspace.dock.snapshot().revision, revision);
        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn opening_a_session_flattens_legacy_nested_workspace_session_tabs() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-flatten-session-tabs-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let first = workspace
            .open_session_pane(Some("work"), "session-first", "First")
            .unwrap();

        let before_legacy_add = workspace.dock.snapshot();
        let second = session_pane_id(Some("work"), "session-legacy");
        let mut descriptor = PanelDescriptor::new(second.0.clone(), "Legacy session");
        descriptor.role = "hermes_session_tile".into();
        descriptor
            .metadata
            .insert("hermes.session.id".into(), "session-legacy".into());
        descriptor
            .metadata
            .insert("hermes.session.profile".into(), "work".into());
        descriptor
            .metadata
            .insert("hermes.session.tile".into(), "1".into());
        let workspace_node = find_panel_node(before_legacy_add.document.root.as_deref(), WORKSPACE)
            .expect("workspace leaf for the previous nested insertion behavior");
        workspace
            .dock
            .transact(
                before_legacy_add.revision,
                CommandId::from("simulate-legacy-nested-session-tab"),
                [Operation::AddPanel {
                    descriptor,
                    target: Some(workspace_node),
                }],
            )
            .unwrap();

        let before_second_legacy_add = workspace.dock.snapshot();
        let third_legacy = session_pane_id(Some("work"), "session-legacy-third");
        let mut legacy_descriptor = PanelDescriptor::new(third_legacy.0.clone(), "Legacy third");
        legacy_descriptor.role = "hermes_session_tile".into();
        legacy_descriptor
            .metadata
            .insert("hermes.session.id".into(), "session-legacy-third".into());
        legacy_descriptor
            .metadata
            .insert("hermes.session.profile".into(), "work".into());
        legacy_descriptor
            .metadata
            .insert("hermes.session.tile".into(), "1".into());
        let workspace_node =
            find_panel_node(before_second_legacy_add.document.root.as_deref(), WORKSPACE)
                .expect("workspace leaf for a second previous-version insertion");
        workspace
            .dock
            .transact(
                before_second_legacy_add.revision,
                CommandId::from("simulate-second-legacy-nested-session-tab"),
                [Operation::AddPanel {
                    descriptor: legacy_descriptor,
                    target: Some(workspace_node),
                }],
            )
            .unwrap();

        let before_migration = workspace.dock.snapshot();
        let workspace_tabs = find_tab_for_panel(
            before_migration.document.root.as_deref(),
            &PanelId::from(WORKSPACE),
        )
        .unwrap()
        .0;
        assert_ne!(
            find_tab_for_panel(before_migration.document.root.as_deref(), &first)
                .unwrap()
                .0,
            workspace_tabs,
            "fixture starts in the nested layout emitted by the previous app build"
        );
        assert_ne!(
            find_tab_for_panel(before_migration.document.root.as_deref(), &second)
                .unwrap()
                .0,
            workspace_tabs,
            "two older nested ancestor groups must be migrated in order"
        );

        let fourth = workspace
            .open_session_pane(Some("work"), "session-fourth", "Fourth")
            .unwrap();
        let normalized = workspace.dock.snapshot();
        let workspace_tabs = find_tab_for_panel(
            normalized.document.root.as_deref(),
            &PanelId::from(WORKSPACE),
        )
        .unwrap()
        .0;
        for session in [first, second, third_legacy, fourth] {
            assert_eq!(
                find_tab_for_panel(normalized.document.root.as_deref(), &session)
                    .unwrap()
                    .0,
                workspace_tabs,
                "all workspace session tiles must share one canonical tab group"
            );
        }
        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn session_panes_are_profile_scoped_and_survive_profile_close_reopen_and_restore() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-session-pane-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let work = workspace
            .open_session_pane(Some("work"), "duplicate-id", "Work transcript")
            .unwrap();
        let personal = workspace
            .open_session_pane(Some("personal"), "duplicate-id", "Personal transcript")
            .unwrap();
        assert_ne!(work, personal);
        assert_eq!(workspace.session_pane_ids().len(), 2);

        let document = workspace.dock.snapshot().document;
        for (panel, profile_id) in [(&work, "work"), (&personal, "personal")] {
            let descriptor = document.descriptor(panel).unwrap();
            assert_eq!(descriptor.role, "hermes_session_tile");
            assert_eq!(descriptor.metadata["hermes.session.id"], "duplicate-id");
            assert_eq!(descriptor.metadata["hermes.session.profile"], profile_id);
            assert!(find_tab_for_panel(document.root.as_deref(), panel).is_some());
        }

        workspace.set_profile("focus").unwrap();
        let document = workspace.dock.snapshot().document;
        assert!(find_tab_for_panel(document.root.as_deref(), &work).is_some());
        assert!(find_tab_for_panel(document.root.as_deref(), &personal).is_some());

        workspace.close_session_pane(&work).unwrap();
        assert!(
            workspace
                .dock
                .snapshot()
                .document
                .closed_panels
                .contains(&work)
        );
        let reopened = workspace
            .open_session_pane(Some("work"), "duplicate-id", "Work transcript")
            .unwrap();
        assert_eq!(reopened, work);
        let document = workspace.dock.snapshot().document;
        assert!(!document.closed_panels.contains(&work));
        assert!(find_tab_for_panel(document.root.as_deref(), &work).is_some());

        workspace
            .open_session_pane(Some("work"), "duplicate-id", "Updated title")
            .unwrap();
        let snapshot = workspace.dock.snapshot();
        let (tabs, selected_index) =
            find_tab_for_panel(snapshot.document.root.as_deref(), &work).unwrap();
        let active_index = find_active_tab_index(snapshot.document.root.as_deref(), &tabs).unwrap();
        assert_eq!(active_index, selected_index);
        workspace.save_layout();
        assert!(workspace.persistence_error.is_none());
        drop(workspace);

        let restored = Workspace::load_from(directory.clone()).unwrap();
        let document = restored.dock.snapshot().document;
        assert_eq!(restored.session_pane_ids().len(), 2);
        assert!(find_tab_for_panel(document.root.as_deref(), &work).is_some());
        assert!(find_tab_for_panel(document.root.as_deref(), &personal).is_some());
        drop(restored);
        assert!(directory.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn restore_closed_session_pane_is_lifo_profile_scoped_and_focuses_restored_tile() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-session-pane-reopen-test-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        let old_work = workspace
            .open_session_pane(Some("work"), "work-old", "Old work")
            .unwrap();
        let new_work = workspace
            .open_session_pane(Some("work"), "work-new", "New work")
            .unwrap();
        let personal = workspace
            .open_session_pane(Some("personal"), "personal-last", "Personal")
            .unwrap();

        workspace.close_session_pane(&old_work).unwrap();
        workspace.close_session_pane(&new_work).unwrap();
        workspace.close_session_pane(&personal).unwrap();

        let restored_work = workspace
            .restore_last_closed_session_pane(Some("work"), None)
            .unwrap()
            .expect("work tab restores");
        assert_eq!(restored_work.panel, new_work);
        assert_eq!(restored_work.session_id, "work-new");
        let snapshot = workspace.dock.snapshot();
        let (tabs, selected) = find_tab_for_panel(snapshot.document.root.as_deref(), &new_work)
            .expect("restored work tab is in the layout");
        assert_eq!(
            find_active_tab_index(snapshot.document.root.as_deref(), &tabs),
            Some(selected)
        );

        let restored_personal = workspace
            .restore_last_closed_session_pane(Some("personal"), None)
            .unwrap()
            .expect("personal stack is independent");
        assert_eq!(restored_personal.panel, personal);
        let restored_old_work = workspace
            .restore_last_closed_session_pane(Some("work"), None)
            .unwrap()
            .expect("older work tab remains on its profile stack");
        assert_eq!(restored_old_work.panel, old_work);
        assert!(
            workspace
                .restore_last_closed_session_pane(Some("work"), None)
                .unwrap()
                .is_none()
        );

        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn find_active_tab_index(root: Option<&Node>, tabs: &NodeId) -> Option<usize> {
        fn walk(node: &Node, tabs: &NodeId) -> Option<usize> {
            if &node.id == tabs {
                if let NodeKind::Tabs { active_index, .. } = &node.kind {
                    return Some(*active_index);
                }
                return None;
            }
            match &node.kind {
                NodeKind::Tabs { items, .. }
                | NodeKind::DocumentWell { groups: items, .. }
                | NodeKind::Split {
                    children: items, ..
                }
                | NodeKind::Extension {
                    children: items, ..
                } => items.iter().find_map(|child| walk(child, tabs)),
                NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
            }
        }
        root.and_then(|root| walk(root, tabs))
    }

    #[test]
    fn session_pane_identity_validation_is_bounded_and_rejects_controls() {
        assert!(validate_session_pane_identity(None, "", "Session").is_err());
        assert!(validate_session_pane_identity(None, "bad\nid", "Session").is_err());
        assert!(validate_session_pane_identity(Some(""), "session", "Session").is_err());
        assert!(validate_session_pane_identity(Some("work"), "session", "bad\ntitle").is_err());
        assert!(validate_session_pane_identity(None, &"s".repeat(513), "Session").is_err());
        assert_eq!(
            session_pane_id(Some("work"), "same"),
            session_pane_id(Some("work"), "same")
        );
        assert_ne!(
            session_pane_id(Some("work"), "same"),
            session_pane_id(Some("personal"), "same")
        );
        assert_eq!(bounded_session_title("\n"), "Session");
        assert_eq!(bounded_session_title(&"x".repeat(100)).chars().count(), 72);
    }

    #[test]
    fn every_source_profile_projects_with_all_panels_open() {
        let profiles = source_profiles();
        for candidate in &profiles {
            let mut dock = DockingService::new(base_document(), 128).unwrap();
            let preferences = UiPreferences {
                mode: if candidate.category == "simple" {
                    InterfaceMode::Simple
                } else {
                    InterfaceMode::Advanced
                },
                ..UiPreferences::default()
            };
            let reveals = BTreeMap::new();
            let mut all_open = preferences.clone();
            all_open.pane_open = BTreeMap::from([
                (BOTS.into(), true),
                (REVIEW.into(), true),
                (FILES.into(), true),
                (TERMINAL.into(), true),
            ]);
            all_open.file_browser_open = true;
            apply_profile_to_service(&mut dock, candidate, &all_open, &reveals)
                .unwrap_or_else(|error| panic!("{}: {error}", candidate.id));
            let document = dock.snapshot().document;
            let snapshot = doofusdock_presentation::project(
                &document,
                doofusdock_presentation::Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 1440.0,
                    h: 900.0,
                },
            )
            .unwrap_or_else(|error| panic!("{}: {error}", candidate.id));
            assert!(
                snapshot.panels.contains_key(&PanelId::from(WORKSPACE)),
                "{} missing workspace",
                candidate.id
            );
            assert!(
                !snapshot.panels.is_empty(),
                "{} projects no panels",
                candidate.id
            );
            assert!(snapshot.panels.contains_key(&PanelId::from(BOTS)));
        }
    }

    #[test]
    fn bots_pane_is_hide_only_and_joins_the_sessions_tab_group() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-bots-pane-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        assert!(!workspace.pane_is_open(BOTS));
        assert_eq!(
            workspace
                .dock
                .snapshot()
                .document
                .descriptor(&PanelId::from(BOTS))
                .unwrap()
                .close_policy,
            ClosePolicy::HideOnly
        );

        workspace.toggle_pane(BOTS).unwrap();
        assert!(workspace.pane_is_open(BOTS));
        let document = workspace.dock.snapshot().document;
        let (tabs, bot_index) = find_tab_for_panel(document.root.as_deref(), &PanelId::from(BOTS))
            .expect("Bots shares a tab group");
        assert_eq!(
            find_active_tab_index(document.root.as_deref(), &tabs),
            Some(bot_index)
        );

        workspace.toggle_pane(BOTS).unwrap();
        assert!(!workspace.pane_is_open(BOTS));
        assert!(
            workspace
                .dock
                .snapshot()
                .document
                .closed_panels
                .contains(&PanelId::from(BOTS))
        );
        drop(workspace);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn master_detail_width_override_persists_by_stable_resize_id_and_can_reset() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-master-detail-width-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            workspace.master_detail_width_override("capabilities-split"),
            None
        );
        workspace
            .set_master_detail_width_override("capabilities-split", Some(412.0))
            .unwrap();
        assert_eq!(
            workspace.master_detail_width_override("capabilities-split"),
            Some(412.0)
        );
        assert!(
            workspace
                .set_master_detail_width_override("capabilities-split", Some(f32::NAN))
                .is_err()
        );
        assert!(
            workspace
                .set_master_detail_width_override("bad\nid", Some(400.0))
                .is_err()
        );
        let mut reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            reloaded.master_detail_width_override("capabilities-split"),
            Some(412.0)
        );
        reloaded
            .set_master_detail_width_override("capabilities-split", None)
            .unwrap();
        let reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(
            reloaded.master_detail_width_override("capabilities-split"),
            None
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn show_ignored_files_preference_is_project_scoped_and_persistent() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data = std::env::temp_dir().join(format!(
            "HermesNative-files-preference-{}-{nonce}",
            std::process::id()
        ));
        let first = data.join("project-a");
        let second = data.join("project-b");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();

        let mut workspace = Workspace::load_from(data.clone()).unwrap();
        assert!(!workspace.shows_ignored_files(&first));
        assert!(workspace.set_show_ignored_files(&first, true).unwrap());
        assert!(!workspace.shows_ignored_files(&second));
        let reloaded = Workspace::load_from(data.clone()).unwrap();
        assert!(reloaded.shows_ignored_files(&first));
        assert!(!reloaded.shows_ignored_files(&second));

        let mut reloaded = reloaded;
        assert!(reloaded.set_show_ignored_files(&first, false).unwrap());
        assert!(
            !Workspace::load_from(data.clone())
                .unwrap()
                .shows_ignored_files(&first)
        );
        std::fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn simple_mode_hides_optional_panes_but_keeps_advanced_preferences() {
        let profiles = source_profiles();
        let mut dock = DockingService::new(base_document(), 128).unwrap();
        let mut preferences = UiPreferences::default();
        preferences.pane_open.insert(REVIEW.into(), true);
        let advanced = profile(&profiles, "default").unwrap();
        apply_profile_to_service(&mut dock, advanced, &preferences, &BTreeMap::new()).unwrap();
        preferences.mode = InterfaceMode::Simple;
        apply_profile_to_service(
            &mut dock,
            profile(&profiles, "sidebar-left").unwrap(),
            &preferences,
            &BTreeMap::new(),
        )
        .unwrap();
        let snapshot = dock.snapshot();
        assert!(
            snapshot
                .document
                .closed_panels
                .contains(&PanelId::from(REVIEW))
        );
        assert!(preferences.pane_open[REVIEW]);
    }

    #[test]
    fn semantic_sidebar_and_file_browser_visibility_persist_across_modes_and_reload() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-semantic-pane-visibility-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        assert!(workspace.pane_is_open(SESSIONS));
        assert!(!workspace.pane_is_open(FILES));

        workspace.toggle_pane(SESSIONS).unwrap();
        workspace.toggle_pane(FILES).unwrap();
        assert!(!workspace.pane_is_open(SESSIONS));
        assert!(workspace.pane_is_open(FILES));
        let snapshot = workspace.dock.snapshot();
        assert!(
            snapshot
                .document
                .closed_panels
                .contains(&PanelId::from(SESSIONS))
        );
        assert!(
            !snapshot
                .document
                .closed_panels
                .contains(&PanelId::from(WORKSPACE))
        );
        assert!(
            !snapshot
                .document
                .closed_panels
                .contains(&PanelId::from(FILES))
        );

        workspace.set_mode(InterfaceMode::Simple).unwrap();
        assert!(!workspace.pane_is_open(SESSIONS));
        assert!(workspace.pane_is_open(FILES));
        drop(workspace);

        let mut reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert!(!reloaded.pane_is_open(SESSIONS));
        assert!(reloaded.pane_is_open(FILES));
        reloaded.toggle_pane(SESSIONS).unwrap();
        reloaded.toggle_pane(FILES).unwrap();
        assert!(reloaded.pane_is_open(SESSIONS));
        assert!(!reloaded.pane_is_open(FILES));
        drop(reloaded);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_file_visibility_migrates_from_optional_pane_preference() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "HermesNative-legacy-file-visibility-{}-{nonce}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            br#"{"advanced_profile":"default","pane_open":{"files":true}}"#,
        )
        .unwrap();
        let preferences = read_preferences(&path).expect("legacy preferences parse");
        assert!(preferences.file_browser_open);
        assert!(preferences.sessions_sidebar_open);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn user_layout_presets_persist_geometry_and_retain_live_session_and_resting_panes() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "HermesNative-user-layout-preset-{}-{nonce}",
            std::process::id()
        ));
        let mut workspace = Workspace::load_from(directory.clone()).unwrap();
        if workspace.pane_is_open(FILES) {
            workspace.toggle_pane(FILES).unwrap();
        }
        let session = workspace
            .open_session_pane(Some("work"), "preset-live-session", "Live session")
            .unwrap();
        let preset_id = workspace.save_user_layout_preset("My work deck").unwrap();
        assert_eq!(preset_id, "user-my-work-deck");
        assert_eq!(workspace.profile_title(&preset_id), "My work deck");
        let saved = &workspace.preferences.user_layout_presets[&preset_id].document;
        assert!(
            saved.descriptor(&session).is_none(),
            "live session descriptor is excluded from preset geometry"
        );
        assert!(!saved.closed_panels.contains(&session));
        assert!(
            saved.closed_panels.contains(&PanelId::from(FILES)),
            "resting pane is retained in saved geometry"
        );

        workspace.set_profile("focus").unwrap();
        workspace.set_profile(&preset_id).unwrap();
        let applied = workspace.dock.snapshot().document;
        assert!(
            applied.descriptor(&session).is_some(),
            "current session descriptor survives preset apply"
        );
        assert!(
            !applied.closed_panels.contains(&session),
            "open session remains open after preset apply"
        );
        assert!(
            applied.closed_panels.contains(&PanelId::from(FILES)),
            "saved hidden pane state is restored"
        );
        let (session_tabs, _) = find_tab_for_panel(applied.root.as_deref(), &session).unwrap();
        let (workspace_tabs, _) =
            find_tab_for_panel(applied.root.as_deref(), &PanelId::from(WORKSPACE)).unwrap();
        assert_eq!(
            session_tabs, workspace_tabs,
            "session tabs are adopted by the workspace group"
        );

        drop(workspace);
        let mut reloaded = Workspace::load_from(directory.clone()).unwrap();
        assert_eq!(reloaded.active_profile(), preset_id);
        assert_eq!(reloaded.profile_title(&preset_id), "My work deck");
        let persisted = reloaded.dock.snapshot().document;
        assert!(persisted.descriptor(&session).is_some());
        assert!(find_tab_for_panel(persisted.root.as_deref(), &session).is_some());
        reloaded.delete_user_layout_preset(&preset_id).unwrap();
        assert_eq!(reloaded.active_profile(), "default");
        assert!(!reloaded.is_user_profile(&preset_id));
        drop(reloaded);
        let after_delete = Workspace::load_from(directory.clone()).unwrap();
        assert!(
            !after_delete.is_user_profile(&preset_id),
            "deletion persists across restart"
        );
        drop(after_delete);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
