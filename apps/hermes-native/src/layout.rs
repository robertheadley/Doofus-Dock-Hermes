use doofusdock_core::{
    ClosePolicy, CommandId, DockingService, LayoutDocument, LayoutId, LayoutStore, Node, NodeId,
    NodeKind, Operation, Orientation, PanelDescriptor, PanelId, PersistenceLimits,
    PersistencePolicy, ProfileLayout, ProfilePresentation, ProfileRegion, WorkspaceProfile,
    apply_workspace_profile,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const SESSIONS: &str = "sessions";
pub const WORKSPACE: &str = "workspace";
pub const REVIEW: &str = "review";
pub const FILES: &str = "files";
pub const TERMINAL: &str = "terminal";
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceMode {
    #[default]
    Advanced,
    Simple,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
    pub mode: InterfaceMode,
    pub advanced_profile: String,
    pub simple_sidebar_right: bool,
    pub pane_open: BTreeMap<String, bool>,
    pub show_ignored_files_roots: BTreeSet<String>,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            mode: InterfaceMode::Advanced,
            advanced_profile: "default".into(),
            simple_sidebar_right: false,
            pane_open: BTreeMap::from([
                (REVIEW.into(), false),
                (FILES.into(), false),
                (TERMINAL.into(), false),
            ]),
            show_ignored_files_roots: BTreeSet::new(),
        }
    }
}

pub struct Workspace {
    pub dock: DockingService,
    store: LayoutStore,
    pub preferences: UiPreferences,
    pub persistence_error: Option<String>,
    profiles: Vec<WorkspaceProfile>,
    session_reveals: BTreeMap<String, bool>,
    closed_session_panes: BTreeMap<String, VecDeque<ClosedSessionPane>>,
    data_directory: PathBuf,
}

const CLOSED_SESSION_PANE_LIMIT: usize = 64;

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
            let path = self.data_directory.join("ui-preferences.json");
            match write_preferences_atomic(&path, &self.preferences) {
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
            profiles: source_profiles(),
            session_reveals: BTreeMap::new(),
            closed_session_panes: BTreeMap::new(),
            data_directory: directory,
        }
    }

    pub fn load() -> Result<Self, String> {
        Self::load_from(data_directory())
    }

    #[cfg(feature = "agent-automation")]
    pub fn load_isolated(directory: PathBuf) -> Result<Self, String> {
        Self::load_from(directory)
    }

    fn load_from(directory: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let store = LayoutStore::new(directory.join("layouts"), PersistenceLimits::default());
        let preferences_path = directory.join("ui-preferences.json");
        let mut preferences = read_preferences(&preferences_path).unwrap_or_default();
        let profiles = source_profiles();
        if preferences.mode == InterfaceMode::Simple {
            preferences.simple_sidebar_right = preferences.advanced_profile == "sidebar-right";
        }

        let layout_id = LayoutId::from("hermes-native-main");
        let loaded = store.load(&layout_id).ok();
        let is_new = loaded.is_none();
        let document = loaded.unwrap_or_else(base_document);
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
            dock,
            store,
            preferences,
            persistence_error: None,
            profiles,
            session_reveals: BTreeMap::new(),
            closed_session_panes: BTreeMap::new(),
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

    pub fn profile_ids(&self) -> Vec<&'static str> {
        match self.preferences.mode {
            InterfaceMode::Advanced => vec!["default", "basic", "focus", "terminal-deck", "quad"],
            InterfaceMode::Simple => vec!["sidebar-left", "sidebar-right"],
        }
    }

    pub fn active_profile(&self) -> &str {
        active_profile_id(&self.preferences)
    }

    pub fn set_profile(&mut self, id: &str) -> Result<(), String> {
        if !self.profile_ids().contains(&id) {
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
        if matches!(panel, SESSIONS | WORKSPACE) {
            return true;
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
        let open = !self.pane_is_open(panel);
        match self.preferences.mode {
            InterfaceMode::Advanced => {
                self.preferences.pane_open.insert(panel.into(), open);
                self.save_preferences();
            }
            InterfaceMode::Simple => {
                self.session_reveals.insert(panel.into(), open);
            }
        }
        self.apply_active_profile()
    }

    pub fn apply_active_profile(&mut self) -> Result<(), String> {
        let id = self.active_profile().to_owned();
        let p = profile(&self.profiles, &id)?;
        apply_profile_to_service(&mut self.dock, p, &self.preferences, &self.session_reveals)?;
        self.save_layout();
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
        let path = self.data_directory.join("ui-preferences.json");
        match write_preferences_atomic(&path, &self.preferences) {
            Ok(()) => self.persistence_error = None,
            Err(error) => self.persistence_error = Some(error),
        }
    }
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
    for panel in [SESSIONS, REVIEW, FILES, TERMINAL] {
        let open = matches!(panel, SESSIONS | WORKSPACE)
            || reveals
                .get(panel)
                .copied()
                .unwrap_or_else(|| match preferences.mode {
                    InterfaceMode::Advanced => {
                        preferences.pane_open.get(panel).copied().unwrap_or(false)
                    }
                    InterfaceMode::Simple => false,
                });
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
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "spl-root",
                Orientation::Horizontal,
                vec![
                    region(SESSIONS),
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
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "basic-root",
                Orientation::Horizontal,
                vec![
                    region(SESSIONS),
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
                optional(FILES),
                optional(REVIEW),
                optional(TERMINAL),
            ],
            split(
                "focus-root",
                Orientation::Horizontal,
                vec![
                    region(SESSIONS),
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
                            region(SESSIONS),
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
                            tabs("quad-left-tabs", 0, vec![region(SESSIONS), region(FILES)]),
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
                optional(REVIEW),
                optional(FILES),
                optional(TERMINAL),
            ],
            split(
                "simple-left-root",
                Orientation::Horizontal,
                vec![
                    region(SESSIONS),
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
                    region(SESSIONS),
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
        (WORKSPACE, "New session"),
        (REVIEW, "Review"),
        (FILES, "Files"),
        (TERMINAL, "Terminal"),
    ];
    let descriptors = panels
        .iter()
        .map(|(id, title)| {
            let mut descriptor = PanelDescriptor::new(*id, *title);
            descriptor.minimum = doofusdock_core::Size::new(180, 140);
            descriptor.close_policy = if *id == WORKSPACE {
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
    serde_json::from_slice(&bytes).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

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
                (REVIEW.into(), true),
                (FILES.into(), true),
                (TERMINAL.into(), true),
            ]);
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
        }
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
        preferences.pane_open.insert(FILES.into(), true);
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
                .contains(&PanelId::from(FILES))
        );
        assert!(preferences.pane_open[FILES]);
    }
}
