use doofusdock_core::{
    ClosePolicy, DockingService, LayoutDocument, LayoutId, LayoutStore, Orientation,
    PanelDescriptor, PanelId, PersistenceLimits, ProfileLayout, ProfilePresentation, ProfileRegion,
    WorkspaceProfile, apply_workspace_profile,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

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
pub struct UiPreferences {
    pub mode: InterfaceMode,
    pub advanced_profile: String,
    pub simple_sidebar_right: bool,
    pub pane_open: BTreeMap<String, bool>,
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
}

impl Workspace {
    pub fn load_fallback() -> Self {
        let dock =
            DockingService::new(base_document(), 128).expect("the built-in Hermes layout is valid");
        Self {
            dock,
            store: LayoutStore::new(
                std::env::temp_dir().join("HermesNative-recovery"),
                PersistenceLimits::default(),
            ),
            preferences: UiPreferences::default(),
            persistence_error: None,
            profiles: source_profiles(),
            session_reveals: BTreeMap::new(),
        }
    }

    pub fn load() -> Result<Self, String> {
        let directory = data_directory();
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
            dock,
            store,
            preferences,
            persistence_error: None,
            profiles,
            session_reveals: BTreeMap::new(),
        };
        workspace.save_layout();
        Ok(workspace)
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
        let document = self.dock.snapshot().document;
        match self.store.save(&document) {
            Ok(()) => self.persistence_error = None,
            Err(error) => self.persistence_error = Some(error.to_string()),
        }
    }

    fn save_preferences(&mut self) {
        let path = data_directory().join("ui-preferences.json");
        match write_preferences_atomic(&path, &self.preferences) {
            Ok(()) => self.persistence_error = None,
            Err(error) => self.persistence_error = Some(error),
        }
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
    let mut assignments = BTreeMap::new();
    for panel in [SESSIONS, WORKSPACE, REVIEW, FILES, TERMINAL] {
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

    let observed = dock.snapshot();
    let application = apply_workspace_profile(&observed.document, profile, &assignments)
        .map_err(|error| error.to_string())?;
    dock.apply_layout(observed.revision, &application.document)
        .map_err(|error| error.to_string())?;
    Ok(())
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
