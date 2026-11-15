use crate::gateway::{
    CapabilitiesSnapshot, GatewayDiscovery, GatewayEndpoint, GatewayEvent, PromptSubmitReceipt,
    SessionSearchResult, SessionSummary, SkillInfo, ToolsetInfo,
};
use crate::layout::{FILES, InterfaceMode, REVIEW, SESSIONS, TERMINAL, WORKSPACE, Workspace};
use crate::routes::AppRoute;
use doofusdock_core::{
    CommandId, DragSession, Node, NodeId, NodeKind, Operation, Orientation, PanelId,
};
use doofusdock_presentation::{Box2, project};
use doofusdock_render_egui::{
    chrome::{
        PanelChromeControl, PanelChromeStyle, ResizeAxis, ResizeHandleStyle, panel_title_bar,
        resize_handle,
    },
    tabs::{TabStripItem, TabStripState, tab_strip},
};
use egui::{Align, Color32, FontId, Id, Layout, Rect, RichText, Role, Stroke, Ui, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegisteredUiAction {
    Navigate {
        path: String,
    },
    OpenSession {
        session_id: String,
        profile: Option<String>,
    },
    TogglePane {
        panel: String,
    },
    SetInterfaceMode {
        mode: InterfaceMode,
    },
    SelectLayoutProfile {
        profile_id: String,
    },
    SelectCapabilitiesTab {
        tab: CapabilitiesTab,
    },
    RefreshCapabilities,
    SetToolsetEnabled {
        name: String,
        enabled: bool,
    },
    StopGeneration,
    RespondApproval {
        prompt_id: String,
        choice: String,
    },
    RespondClarify {
        prompt_id: String,
        answer: String,
    },
    CloseActiveTab,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitiesTab {
    #[default]
    Skills,
    Plugins,
    Connectors,
    Toolsets,
}

impl CapabilitiesTab {
    fn as_str(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Plugins => "plugins",
            Self::Connectors => "connectors",
            Self::Toolsets => "toolsets",
        }
    }

    fn from_query(path: &str) -> Self {
        let Some((_, query)) = path.split_once('?') else {
            return Self::Skills;
        };
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("tab="))
            .and_then(|value| match value {
                "skills" => Some(Self::Skills),
                "plugins" => Some(Self::Plugins),
                "connectors" => Some(Self::Connectors),
                "toolsets" => Some(Self::Toolsets),
                _ => None,
            })
            .unwrap_or_default()
    }
}

pub fn registered_ui_action_catalog() -> serde_json::Value {
    json!({
        "version": 1,
        "actions": [
            {"type":"navigate","permission":"ui_interaction","mutates":true,"schema":{"path":"string, max 2048 bytes"}},
            {"type":"open_session","permission":"ui_interaction","mutates":true,"schema":{"session_id":"string, max 512 bytes","profile":"string, max 128 bytes, or null for the default profile"}},
            {"type":"toggle_pane","permission":"ui_interaction","mutates":true,"schema":{"panel":[REVIEW,FILES,TERMINAL]}},
            {"type":"set_interface_mode","permission":"ui_interaction","mutates":true,"schema":{"mode":["advanced","simple"]}},
            {"type":"select_layout_profile","permission":"ui_interaction","mutates":true,"schema":{"profile_id":"one of the profiles available in the selected interface mode"}}
            ,{"type":"select_capabilities_tab","permission":"ui_interaction","mutates":true,"schema":{"tab":["skills","plugins","connectors","toolsets"]}}
            ,{"type":"refresh_capabilities","permission":"ui_interaction","mutates":false,"schema":{}}
            ,{"type":"set_toolset_enabled","permission":"ui_interaction","mutates":true,"schema":{"name":"one of the loaded toolsets, max 128 bytes","enabled":"boolean"}}
            ,{"type":"stop_generation","permission":"ui_interaction","mutates":true,"schema":{}}
            ,{"type":"respond_approval","permission":"ui_interaction","mutates":true,"schema":{"prompt_id":"currently visible approval request","choice":["once","session","always","deny"]}}
            ,{"type":"respond_clarify","permission":"ui_interaction","mutates":true,"schema":{"prompt_id":"currently visible clarify request","answer":"single answer text, batch JSON {answers:{qid:string|null}}, or empty string to skip"}}
            ,{"type":"close_active_tab","permission":"ui_interaction","mutates":true,"schema":{}}
        ]
    })
}

const BG: Color32 = Color32::from_rgb(22, 24, 28);
const PANEL: Color32 = Color32::from_rgb(29, 32, 38);
const PANEL_ALT: Color32 = Color32::from_rgb(35, 38, 45);
const BORDER: Color32 = Color32::from_rgb(56, 60, 69);
const TEXT: Color32 = Color32::from_rgb(225, 228, 234);
const MUTED: Color32 = Color32::from_rgb(151, 158, 170);
const ACCENT: Color32 = Color32::from_rgb(104, 164, 245);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropEdge {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug)]
struct PanelDropTarget {
    panel: PanelId,
    node: NodeId,
    edge: DropEdge,
    preview: Rect,
}

#[derive(Clone, Debug)]
struct DockTabGroup {
    node: NodeId,
    panels: Vec<PanelId>,
    active_index: usize,
}

#[derive(Clone, Debug)]
struct TabReorderSession {
    revision: u64,
    source: PanelId,
    container: NodeId,
    target_index: Option<usize>,
}

#[derive(Clone, Debug)]
struct SplitResizeSession {
    split: NodeId,
    revision: u64,
    divider: usize,
    initial_weights: Vec<f64>,
    current_weights: Vec<f64>,
    extent: f32,
    start: egui::Pos2,
}

#[derive(Clone, Debug)]
struct PendingToolsetUpdate {
    request_id: String,
    previous_enabled: bool,
}

#[derive(Clone, Debug, Default)]
struct SessionChatViewState {
    profile: Option<String>,
    runtime_session_id: Option<String>,
    transcript: Vec<serde_json::Value>,
    transcript_message_count: usize,
    transcript_has_more: bool,
    older_transcript_request: Option<(String, String, Option<String>, usize)>,
    older_transcript_error: Option<String>,
    draft: String,
}

#[derive(Clone, Debug)]
struct PendingPrompt {
    session_id: String,
    profile: Option<String>,
    submission_id: String,
    text: String,
    uncertain: bool,
    admission_id: Option<String>,
    execution_generation: Option<u64>,
    resolving_unknown: bool,
}

#[derive(Clone, Debug)]
struct ApprovalPrompt {
    session_id: String,
    profile: Option<String>,
    execution_generation: u64,
    prompt_id: String,
    server_request_id: Option<String>,
    command: String,
    description: String,
    choices: Vec<String>,
    response_pending: Option<(String, String)>,
}

#[derive(Clone, Debug)]
struct ClarifyPrompt {
    session_id: String,
    profile: Option<String>,
    execution_generation: u64,
    prompt_id: String,
    question: String,
    choices: Vec<String>,
    multi_select: bool,
    selected: Vec<String>,
    draft: String,
    response_pending: Option<(String, String)>,
}

#[derive(Clone, Debug)]
struct BatchClarifyQuestion {
    qid: String,
    question: String,
    choices: Vec<String>,
    multi_select: bool,
    selected: Vec<String>,
    draft: String,
}

#[derive(Clone, Debug)]
struct PendingClarifyLock {
    question_id: String,
    answer: Option<String>,
}

#[derive(Clone, Debug)]
struct BatchClarifyPrompt {
    session_id: String,
    profile: Option<String>,
    server_request_id: String,
    questions: Vec<BatchClarifyQuestion>,
    response_pending: bool,
    pending_locks: VecDeque<PendingClarifyLock>,
    lock_in_flight: Option<(String, String)>,
}

pub struct HermesNativeApp {
    workspace: Workspace,
    load_error: Option<String>,
    status: String,
    route: AppRoute,
    overlay_return: AppRoute,
    contributed_paths: BTreeSet<String>,
    gateway_discovery: Option<GatewayDiscovery>,
    gateway_endpoint: Option<GatewayEndpoint>,
    sessions: Vec<SessionSummary>,
    archived_sessions: Vec<SessionSummary>,
    show_archived_sessions: bool,
    archived_sessions_request_id: Option<String>,
    archived_sessions_loading: bool,
    archived_sessions_error: Option<String>,
    session_filter: String,
    session_search_changed_at: Option<std::time::Instant>,
    session_search_request_id: Option<String>,
    session_search_results: Vec<SessionSearchResult>,
    session_search_pending: bool,
    session_search_error: Option<String>,
    session_list_error: Option<String>,
    capabilities_tab: CapabilitiesTab,
    capabilities_query: String,
    capabilities_request_id: Option<String>,
    capabilities_pending: bool,
    capabilities_error: Option<String>,
    skills: Option<Vec<SkillInfo>>,
    toolsets: Option<Vec<ToolsetInfo>>,
    pending_toolset_updates: BTreeMap<String, PendingToolsetUpdate>,
    fonts_installed: bool,
    gateway_connected: bool,
    session_chat_views: BTreeMap<String, SessionChatViewState>,
    session_chat_view_order: VecDeque<String>,
    new_chat_draft: String,
    transcript: Vec<serde_json::Value>,
    transcript_message_count: usize,
    transcript_has_more: bool,
    older_transcript_request: Option<(String, String, Option<String>, usize)>,
    older_transcript_error: Option<String>,
    markdown_cache: CommonMarkCache,
    active_runtime_session_id: Option<String>,
    active_session_profile: Option<String>,
    draft: String,
    pending_create: Option<(String, String)>,
    untouched_blank_session: Option<(String, Option<String>)>,
    pending_submissions: BTreeMap<String, PendingPrompt>,
    recent_completed_admissions: VecDeque<String>,
    pending_interrupt: Option<(String, String)>,
    pending_session_pins: BTreeMap<String, String>,
    pending_session_unreads: BTreeMap<String, String>,
    pending_session_archives: BTreeMap<String, String>,
    pending_session_renames: BTreeMap<String, String>,
    runtime_session_renames: BTreeMap<String, (String, String)>,
    uncertain_session_renames: BTreeSet<String>,
    rename_session: Option<SessionSummary>,
    rename_draft: String,
    pending_session_close: Option<PanelId>,
    approval_prompts: BTreeMap<String, ApprovalPrompt>,
    clarify_prompts: BTreeMap<String, ClarifyPrompt>,
    batch_clarify_prompts: BTreeMap<String, BatchClarifyPrompt>,
    panel_drag: Option<DragSession<PanelDropTarget>>,
    panel_drag_source: Option<PanelId>,
    tab_reorder: Option<TabReorderSession>,
    dock_tabs: BTreeMap<String, TabStripState>,
    split_resize: Option<SplitResizeSession>,
}

impl HermesNativeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::from_workspace_result(true, Workspace::load(), Workspace::load_fallback())
    }

    #[cfg(feature = "agent-automation")]
    pub fn new_for_automation(profile_dir: std::path::PathBuf) -> Self {
        let fallback_dir = profile_dir.clone();
        Self::from_workspace_result(
            false,
            Workspace::load_isolated(profile_dir),
            Workspace::load_fallback_isolated(fallback_dir),
        )
    }

    #[cfg(feature = "agent-automation")]
    pub fn load_markdown_fixture_for_automation(&mut self) {
        let markdown = r#"# Markdown preview

This message exercises **bold**, *italic*, ~~strikethrough~~, `inline code`, and a [source link](https://example.com/hermes).

> A block quote with **emphasis**.

- unordered item one
- unordered item two

1. ordered item one
2. ordered item two

| View | Native rendering |
|:--|--:|
| Chat | Markdown |
| Tools | Cards |

```rust
fn render_message() -> &'static str {
    "Hello, Hermes"
}
```"#;
        self.route = AppRoute::Session("markdown-fixture".into());
        self.transcript = vec![json!({"role":"assistant","content":markdown})];
        self.status = "Markdown fixture · gateway isolated".into();
    }
    #[cfg(feature = "agent-automation")]
    pub fn load_long_session_fixture_for_automation(&mut self) {
        self.route = AppRoute::Session("long-session-fixture".into());
        self.active_session_profile = Some("fixture-profile".into());
        self.active_runtime_session_id = Some("runtime-long-session-fixture".into());
        self.transcript = vec![
            json!({"role":"user","content":"Show me the most recent part of a long conversation."}),
            json!({"role":"assistant","content":"Earlier turns can be loaded from the transcript history."}),
        ];
        self.transcript_message_count = 240;
        self.transcript_has_more = true;
        self.status = "Long transcript fixture · gateway isolated".into();
    }

    #[cfg(feature = "agent-automation")]
    pub fn load_session_pane_fixture_for_automation(&mut self) {
        let session_id = "pane-fixture-session";
        let profile = "fixture-profile";
        self.workspace
            .open_session_pane(Some(profile), session_id, "Native session pane")
            .expect("fixture pane identity is valid");
        self.sessions.push(SessionSummary {
            id: session_id.into(),
            profile: Some(profile.into()),
            title: "Native session pane".into(),
            preview: "This pane has its own transcript and composer draft.".into(),
            message_count: 2,
            source: "desktop".into(),
            started_at: 0.0,
            last_active: None,
            pinned: false,
            unread: false,
            archived: false,
        });
        self.session_chat_views.insert(
            session_identity_key(session_id, Some(profile)),
            SessionChatViewState {
                profile: Some(profile.into()),
                runtime_session_id: Some("pane-fixture-runtime".into()),
                transcript: vec![
                    json!({"role":"user","content":"How does the session pane work?"}),
                    json!({
                        "role":"assistant",
                        "content":"Each Dock pane owns its **profile-scoped** transcript and draft.\n\nClosing a busy tab leaves Hermes running in the background."
                    }),
                ],
                transcript_message_count: 2,
                draft: "Keep this draft with the pane".into(),
                ..SessionChatViewState::default()
            },
        );
        self.gateway_connected = true;
        self.status = "Session pane fixture · gateway isolated".into();
    }

    fn from_workspace_result(
        connect_gateway: bool,
        workspace: Result<Workspace, String>,
        fallback: Workspace,
    ) -> Self {
        let (workspace, load_error) = match workspace {
            Ok(workspace) => (workspace, None),
            Err(error) => (fallback, Some(error)),
        };
        Self {
            workspace,
            load_error,
            status: if connect_gateway {
                "Backend offline"
            } else {
                "Automation · gateway isolated"
            }
            .into(),
            route: AppRoute::NewChat,
            overlay_return: AppRoute::NewChat,
            contributed_paths: BTreeSet::new(),
            gateway_discovery: connect_gateway.then(GatewayDiscovery::start),
            gateway_endpoint: None,
            sessions: Vec::new(),
            archived_sessions: Vec::new(),
            show_archived_sessions: false,
            archived_sessions_request_id: None,
            archived_sessions_loading: false,
            archived_sessions_error: None,
            session_filter: String::new(),
            session_search_changed_at: None,
            session_search_request_id: None,
            session_search_results: Vec::new(),
            session_search_pending: false,
            session_search_error: None,
            session_list_error: None,
            capabilities_tab: CapabilitiesTab::default(),
            capabilities_query: String::new(),
            capabilities_request_id: None,
            capabilities_pending: false,
            capabilities_error: None,
            skills: None,
            toolsets: None,
            pending_toolset_updates: BTreeMap::new(),
            fonts_installed: false,
            gateway_connected: false,
            session_chat_views: BTreeMap::new(),
            session_chat_view_order: VecDeque::new(),
            new_chat_draft: String::new(),
            transcript: Vec::new(),
            transcript_message_count: 0,
            transcript_has_more: false,
            older_transcript_request: None,
            older_transcript_error: None,
            markdown_cache: CommonMarkCache::default(),
            active_runtime_session_id: None,
            active_session_profile: None,
            draft: String::new(),
            pending_create: None,
            untouched_blank_session: None,
            pending_submissions: BTreeMap::new(),
            recent_completed_admissions: VecDeque::new(),
            pending_interrupt: None,
            pending_session_pins: BTreeMap::new(),
            pending_session_unreads: BTreeMap::new(),
            pending_session_archives: BTreeMap::new(),
            pending_session_renames: BTreeMap::new(),
            runtime_session_renames: BTreeMap::new(),
            uncertain_session_renames: BTreeSet::new(),
            rename_session: None,
            rename_draft: String::new(),
            pending_session_close: None,
            approval_prompts: BTreeMap::new(),
            clarify_prompts: BTreeMap::new(),
            batch_clarify_prompts: BTreeMap::new(),
            panel_drag: None,
            panel_drag_source: None,
            tab_reorder: None,
            dock_tabs: BTreeMap::new(),
            split_resize: None,
        }
    }

    fn chrome_style() -> PanelChromeStyle {
        PanelChromeStyle {
            titlebar: PANEL_ALT,
            title: TEXT,
            control: Color32::from_rgb(43, 47, 55),
            control_hover: Color32::from_rgb(61, 69, 82),
            control_text: MUTED,
            control_border: BORDER,
            focus: ACCENT,
            corner_radius: 0.0,
            font: FontId::proportional(12.0),
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 48.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add_space(12.0);
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Hermes").size(17.0).strong().color(TEXT));
                    ui.add_space(16.0);
                    ui.label(RichText::new(&self.status).size(12.0).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Settings").clicked() {
                            self.execute_registered_action(RegisteredUiAction::Navigate {
                                path: "/settings".into(),
                            })
                            .unwrap_or_else(|error| self.status = error);
                        }
                        let previous = self.workspace.preferences.mode;
                        let mut selected_mode = previous;
                        egui::ComboBox::new("interface-mode", "Interface mode")
                            .selected_text(match previous {
                                InterfaceMode::Advanced => "Advanced",
                                InterfaceMode::Simple => "Simple",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut selected_mode,
                                    InterfaceMode::Advanced,
                                    "Advanced",
                                );
                                ui.selectable_value(
                                    &mut selected_mode,
                                    InterfaceMode::Simple,
                                    "Simple",
                                );
                            });
                        if previous != selected_mode {
                            self.execute_registered_action(RegisteredUiAction::SetInterfaceMode {
                                mode: selected_mode,
                            })
                            .unwrap_or_else(|error| self.status = error);
                        }
                        let active = self.workspace.active_profile().to_owned();
                        let options = self.workspace.profile_ids();
                        let title = profile_title(&active);
                        let mut selected = active.clone();
                        egui::ComboBox::new("layout-profile", "Layout profile")
                            .selected_text(title)
                            .show_ui(ui, |ui| {
                                for option in options {
                                    ui.selectable_value(
                                        &mut selected,
                                        option.to_owned(),
                                        profile_title(option),
                                    );
                                }
                            });
                        if selected != active {
                            self.execute_registered_action(
                                RegisteredUiAction::SelectLayoutProfile {
                                    profile_id: selected,
                                },
                            )
                            .unwrap_or_else(|error| self.status = error);
                        }
                    });
                });
            },
        );
    }

    fn pane_controls(&mut self, ui: &mut Ui) {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 36.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add_space(10.0);
                ui.horizontal_centered(|ui| {
                    for (id, label) in
                        [(REVIEW, "Review"), (FILES, "Files"), (TERMINAL, "Terminal")]
                    {
                        let open = self.workspace.pane_is_open(id);
                        let response = ui.selectable_label(open, label);
                        if response.clicked() {
                            self.execute_registered_action(RegisteredUiAction::TogglePane {
                                panel: id.into(),
                            })
                            .unwrap_or_else(|error| self.status = error);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Hermes Desktop · native Rust shell")
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
            },
        );
    }

    fn draw_panel(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        rect: Rect,
        drop_rect: Rect,
        show_titlebar: bool,
    ) {
        if rect.width() < 2.0 || rect.height() < 2.0 {
            return;
        }
        let rect = rect.intersect(ui.clip_rect());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::top_down(Align::Min)),
        );
        let descriptor = self
            .workspace
            .dock
            .snapshot()
            .document
            .descriptor(&PanelId::from(id))
            .cloned();
        let is_session_tile = descriptor
            .as_ref()
            .is_some_and(|descriptor| descriptor.role == "hermes_session_tile");
        let title = match id {
            SESSIONS => "Sessions",
            WORKSPACE => self.route.title(),
            REVIEW => "Review",
            FILES => "Files",
            TERMINAL => "Terminal",
            _ => descriptor
                .as_ref()
                .map(|descriptor| descriptor.title_key.as_str())
                .unwrap_or(id),
        };
        let controls = if id == SESSIONS || id == WORKSPACE {
            vec![]
        } else if is_session_tile {
            vec![PanelChromeControl {
                glyph: "×",
                accessible_label: "Close session tab",
                tooltip: "Close session tab",
                enabled: true,
            }]
        } else {
            vec![PanelChromeControl {
                glyph: "×",
                accessible_label: "Hide panel",
                tooltip: "Hide panel",
                enabled: true,
            }]
        };
        if show_titlebar {
            let bar = child.allocate_space(Vec2::new(rect.width(), 32.0)).1;
            let response = panel_title_bar(
                &mut child,
                Id::unique(("panel-title", id)),
                bar,
                title,
                &Self::chrome_style(),
                &controls,
            );
            response
                .drag
                .widget_info(|| egui::WidgetInfo::labeled(Role::Group, true, title));
            if response.drag.drag_started() {
                self.begin_panel_drag(PanelId::from(id));
            }
            if !response.activated_controls.is_empty() {
                if is_session_tile {
                    self.request_close_session_pane(&PanelId::from(id));
                } else {
                    self.execute_registered_action(RegisteredUiAction::TogglePane {
                        panel: id.into(),
                    })
                    .unwrap_or_else(|error| self.status = error);
                }
            }
        }
        self.update_panel_drop_target(&child, id, drop_rect);
        child.add_space(8.0);
        match id {
            SESSIONS => self.sessions_content(&mut child),
            WORKSPACE => self.workspace_content(&mut child),
            REVIEW => self.empty_panel(&mut child, "No review is active"),
            FILES => self.empty_panel(&mut child, "Session files will appear here"),
            TERMINAL => self.empty_panel(
                &mut child,
                "Terminal is unavailable while the backend is offline",
            ),
            _ if is_session_tile => {
                self.session_pane_content(
                    &mut child,
                    descriptor.as_ref().expect("matched descriptor"),
                );
            }
            _ => {}
        }
    }

    fn draw_split_resize_handles(
        &mut self,
        ui: &mut Ui,
        layout: &doofusdock_core::LayoutDocument,
        snapshot: &doofusdock_presentation::Snapshot,
    ) {
        let mut splits = Vec::new();
        collect_dock_splits(layout.root.as_deref(), &mut splits);
        for split in splits {
            let Some(bounds) = snapshot.nodes.get(&split.node) else {
                continue;
            };
            let rect = Rect::from_min_max(
                egui::pos2(bounds.x as f32, bounds.y as f32),
                egui::pos2((bounds.x + bounds.w) as f32, (bounds.y + bounds.h) as f32),
            );
            let horizontal = split.orientation == Orientation::Horizontal;
            let extent = if horizontal {
                rect.width()
            } else {
                rect.height()
            };
            if extent <= 0.0 || split.weights.len() < 2 {
                continue;
            }
            let weights = self
                .split_resize
                .as_ref()
                .filter(|resize| resize.split == split.node)
                .map_or_else(
                    || split.weights.clone(),
                    |resize| resize.current_weights.clone(),
                );
            let divider_size = 8.0;
            let mut position = if horizontal { rect.left() } else { rect.top() };
            for divider in 0..weights.len() - 1 {
                position += extent * weights[divider] as f32;
                let handle_rect = if horizontal {
                    Rect::from_min_max(
                        egui::pos2(position - divider_size * 0.5, rect.top()),
                        egui::pos2(position + divider_size * 0.5, rect.bottom()),
                    )
                } else {
                    Rect::from_min_max(
                        egui::pos2(rect.left(), position - divider_size * 0.5),
                        egui::pos2(rect.right(), position + divider_size * 0.5),
                    )
                };
                let label = format!("Resize split {} divider {}", split.node.0, divider + 1);
                let response = resize_handle(
                    ui,
                    Id::unique(("hermes-split-resize", &split.node.0, divider)),
                    handle_rect,
                    if horizontal {
                        ResizeAxis::Horizontal
                    } else {
                        ResizeAxis::Vertical
                    },
                    true,
                    &label,
                    ResizeHandleStyle {
                        idle: BORDER,
                        hovered: ACCENT,
                        dragged: Color32::from_rgb(132, 184, 255),
                    },
                );
                if response.drag_started()
                    && let Some(start) = response.interact_pointer_pos()
                {
                    self.split_resize = Some(SplitResizeSession {
                        split: split.node.clone(),
                        revision: layout.revision,
                        divider,
                        initial_weights: split.weights.clone(),
                        current_weights: split.weights.clone(),
                        extent,
                        start,
                    });
                    self.status = "Resize split · release to apply".into();
                }
                if let Some(resize) = &mut self.split_resize
                    && resize.split == split.node
                    && resize.divider == divider
                    && (response.dragged() || response.drag_stopped())
                    && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
                {
                    let delta = pointer - resize.start;
                    let delta_axis = if horizontal { delta.x } else { delta.y };
                    resize.current_weights = resized_split_weights(
                        &resize.initial_weights,
                        resize.divider,
                        delta_axis,
                        resize.extent,
                    );
                }
            }
        }
    }

    fn finish_split_resize(&mut self) {
        let Some(resize) = self.split_resize.take() else {
            return;
        };
        let snapshot = self.workspace.dock.snapshot();
        if snapshot.revision != resize.revision {
            self.status = "Split resize cancelled because the layout changed".into();
            return;
        }
        match self.workspace.dock.transact(
            snapshot.revision,
            CommandId::from(format!(
                "resize-split-{}-r{}",
                resize.split.0, resize.revision
            )),
            [Operation::ResizeSplit {
                split: resize.split,
                weights: resize.current_weights,
            }],
        ) {
            Ok(_) => {
                self.workspace.save_layout();
                if let Some(error) = &self.workspace.persistence_error {
                    self.status = format!("Split resized; persistence failed: {error}");
                } else {
                    self.status = "Split resized".into();
                }
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn begin_panel_drag(&mut self, panel: PanelId) {
        let snapshot = self.workspace.dock.snapshot();
        match DragSession::begin(snapshot.revision, vec![panel.clone()]) {
            Ok(drag) => {
                self.panel_drag = Some(drag);
                self.panel_drag_source = Some(panel);
                self.status = "Move panel · drop to tab or split".into();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn update_panel_drop_target(&mut self, ui: &Ui, panel_id: &str, rect: Rect) {
        let (Some(source), Some(drag)) = (&self.panel_drag_source, &mut self.panel_drag) else {
            return;
        };
        let target_panel = PanelId::from(panel_id);
        if source == &target_panel {
            return;
        }
        let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !rect.contains(pointer) {
            return;
        }
        let snapshot = self.workspace.dock.snapshot();
        let Some(node) = find_panel_node(snapshot.document.root.as_deref(), panel_id) else {
            return;
        };
        let edge = compute_drop_edge(rect, pointer);
        let target = PanelDropTarget {
            panel: target_panel,
            node,
            edge,
            preview: compute_drop_preview(rect, edge),
        };
        if let Err(error) = drag.update_preview(snapshot.revision, Some(target)) {
            self.status = error.to_string();
            self.panel_drag = None;
            self.panel_drag_source = None;
        }
    }

    fn paint_panel_drop_preview(&self, ui: &Ui) {
        let Some(target) = self
            .panel_drag
            .as_ref()
            .and_then(DragSession::preview_target)
        else {
            return;
        };
        ui.painter().rect_filled(
            target.preview,
            2.0,
            Color32::from_rgba_unmultiplied(104, 164, 245, 72),
        );
        ui.painter().rect_stroke(
            target.preview,
            2.0,
            Stroke::new(2.0, ACCENT),
            egui::StrokeKind::Inside,
        );
    }

    fn finish_panel_drag(&mut self) {
        let Some(drag) = self.panel_drag.take() else {
            return;
        };
        let source = self.panel_drag_source.take();
        let revision = self.workspace.dock.snapshot().revision;
        match drag.commit(revision) {
            Ok(Some(target)) => {
                let Some(source) = source else {
                    return;
                };
                self.commit_panel_drop(source, target);
            }
            Ok(None) => self.status = "Panel move cancelled".into(),
            Err(error) => self.status = error.to_string(),
        }
    }

    fn finish_tab_reorder(&mut self) -> bool {
        let Some(reorder) = self.tab_reorder.take() else {
            return false;
        };
        let Some(target_index) = reorder.target_index else {
            // No tab in the source strip is under the pointer at release; keep
            // the normal panel drag alive so it can commit a dock/split drop.
            return false;
        };
        self.panel_drag = None;
        self.panel_drag_source = None;
        let snapshot = self.workspace.dock.snapshot();
        if snapshot.revision != reorder.revision {
            self.status = "Tab reorder cancelled because the layout changed".into();
            return true;
        }
        let Some(source_index) = collect_dock_tab_groups(snapshot.document.root.as_deref())
            .into_iter()
            .find(|group| group.node == reorder.container)
            .and_then(|group| {
                group
                    .panels
                    .iter()
                    .position(|panel| panel == &reorder.source)
            })
        else {
            self.status = "Tab reorder cancelled because its source tab disappeared".into();
            return true;
        };
        if source_index == target_index {
            self.status = "Tab order unchanged".into();
            return true;
        }
        // ReorderPanel's index is the pre-removal insertion slot. Adjust it so
        // dropping on a tab means that tab's final canonical position.
        let insertion_index = if target_index > source_index {
            target_index + 1
        } else {
            target_index
        };
        match self.workspace.dock.transact(
            reorder.revision,
            CommandId::from(format!(
                "reorder-tab-{}-{}-r{}",
                reorder.container.0, reorder.source.0, reorder.revision
            )),
            [Operation::ReorderPanel {
                panel: reorder.source,
                container: reorder.container,
                index: insertion_index,
            }],
        ) {
            Ok(_) => {
                self.workspace.save_layout();
                self.status = "Dock tabs reordered".into();
            }
            Err(error) => self.status = error.to_string(),
        }
        true
    }

    fn commit_panel_drop(&mut self, source: PanelId, target: PanelDropTarget) {
        let snapshot = self.workspace.dock.snapshot();
        let operation = panel_drop_operation(source, target);
        match self.workspace.dock.transact(
            snapshot.revision,
            CommandId::from("panel-drag-drop"),
            [operation],
        ) {
            Ok(_) => {
                self.workspace.save_layout();
                self.status = "Panel layout updated".into();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn cancel_panel_drag(&mut self) {
        self.panel_drag = None;
        self.panel_drag_source = None;
        self.status = "Panel move cancelled".into();
    }

    fn sessions_content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let new = ui.add_sized(
            [ui.available_width() - 20.0, 34.0],
            egui::Button::new(RichText::new("+  New session").color(TEXT)),
        );
        if new.clicked() {
            self.create_blank_session_tab();
        }
        self.sidebar_route(ui, AppRoute::Capabilities);
        self.sidebar_route(ui, AppRoute::Messaging);
        self.sidebar_route(ui, AppRoute::Artifacts);
        self.sidebar_route(ui, AppRoute::Cron);
        ui.add_space(18.0);
        let view_label = if self.show_archived_sessions {
            "Show active sessions"
        } else {
            "Show archived sessions"
        };
        if ui.button(view_label).accessible_name(view_label).clicked() {
            self.show_archived_sessions = !self.show_archived_sessions;
            self.session_search_request_id = None;
            self.session_search_pending = false;
            self.session_search_results.clear();
            self.session_search_error = None;
            self.session_search_changed_at = None;
            if self.show_archived_sessions {
                self.load_archived_sessions();
            } else if !self.session_filter.trim().is_empty() {
                self.session_search_changed_at = Some(std::time::Instant::now());
            }
        }
        let search = ui
            .add(
                egui::TextEdit::singleline(&mut self.session_filter)
                    .hint_text("Search sessions")
                    .desired_width(ui.available_width()),
            )
            .accessible_name("Search sessions");
        if search.changed() {
            let query_is_empty = self.session_filter.trim().is_empty();
            self.session_search_request_id = None;
            self.session_search_results.clear();
            self.session_search_error = None;
            self.session_search_pending = false;
            self.session_search_changed_at =
                (!query_is_empty && !self.show_archived_sessions).then(std::time::Instant::now);
        }
        let query = self.session_filter.trim().to_owned();
        if query.is_empty() {
            self.session_search_changed_at = None;
            self.session_search_request_id = None;
            self.session_search_pending = false;
            self.session_search_error = None;
            self.session_search_results.clear();
        } else if !self.show_archived_sessions
            && let Some(changed_at) = self.session_search_changed_at
        {
            let debounce = std::time::Duration::from_millis(200);
            if changed_at.elapsed() >= debounce {
                self.session_search_changed_at = None;
                if self.gateway_connected {
                    if let Some(gateway) = &self.gateway_discovery {
                        let request_id = operation_id("session-search");
                        match gateway.search_sessions(request_id.clone(), query.clone()) {
                            Ok(()) => {
                                self.session_search_request_id = Some(request_id);
                                self.session_search_pending = true;
                            }
                            Err(error) => {
                                self.session_search_error = Some(error);
                            }
                        }
                    }
                }
            } else {
                ui.ctx()
                    .request_repaint_after(debounce - changed_at.elapsed());
            }
        }
        ui.add_space(8.0);
        let sessions = if self.show_archived_sessions {
            self.archived_sessions
                .iter()
                .filter(|session| session_matches_filter(session, &query))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            merge_session_search_results(
                &self.sessions,
                &query,
                &self.session_search_results,
                self.session_search_pending,
            )
        };
        if self.archived_sessions_loading && self.show_archived_sessions {
            ui.label(
                RichText::new("Loading archived sessions…")
                    .size(11.0)
                    .color(MUTED),
            );
        }
        if self.archived_sessions_error.is_some() && self.show_archived_sessions {
            ui.label(
                RichText::new(format!(
                    "Archived sessions unavailable: {}",
                    self.archived_sessions_error.as_deref().unwrap_or_default()
                ))
                .size(11.0)
                .color(MUTED),
            );
        }
        if self.session_search_pending && !self.show_archived_sessions {
            ui.label(
                RichText::new("Searching all sessions…")
                    .size(11.0)
                    .color(MUTED),
            );
        }
        if let Some(error) = self
            .session_search_error
            .as_ref()
            .filter(|_| !self.show_archived_sessions)
        {
            ui.label(
                RichText::new(format!("Full-history search unavailable: {error}"))
                    .size(11.0)
                    .color(MUTED),
            );
        }
        if let Some(error) = &self.session_list_error {
            ui.label(
                RichText::new(format!("Session recency refresh unavailable: {error}"))
                    .size(11.0)
                    .color(MUTED),
            );
        }
        if sessions.is_empty() {
            if self.show_archived_sessions && !self.archived_sessions_loading {
                ui.label(
                    RichText::new(if query.is_empty() {
                        "No archived sessions"
                    } else {
                        "No matching archived sessions"
                    })
                    .size(13.0)
                    .color(MUTED),
                );
            } else if !self.show_archived_sessions && query.is_empty() && self.sessions.is_empty() {
                ui.label(RichText::new("No sessions yet").size(13.0).color(MUTED));
            } else if !self.show_archived_sessions && !self.session_search_pending {
                ui.label(
                    RichText::new("No matching sessions")
                        .size(13.0)
                        .color(MUTED),
                );
            }
        }
        for session in sessions {
            let title = if session.title.trim().is_empty() {
                session.id.clone()
            } else {
                session.title.clone()
            };
            let profile_label = session.profile.as_deref().unwrap_or_default();
            let foreign_profile = session.profile.as_deref().is_some_and(|profile| {
                self.gateway_endpoint
                    .as_ref()
                    .is_some_and(|endpoint| profile != endpoint.profile_id)
            });
            let scoped_profile_row =
                !profile_label.is_empty() && (self.show_archived_sessions || foreign_profile);
            let row_name = if scoped_profile_row {
                format!(
                    "{title}, profile {}, session {}",
                    session.profile.as_deref().unwrap_or_default(),
                    session.id
                )
            } else {
                format!("{title}, session {}", session.id)
            };
            let selected = self.route == AppRoute::Session(session.id.clone());
            let row = ui
                .selectable_label(selected, RichText::new(&title).color(TEXT))
                .accessible_name(row_name);
            if row.clicked() {
                self.execute_registered_action(RegisteredUiAction::OpenSession {
                    session_id: session.id.clone(),
                    profile: session.profile.clone(),
                })
                .unwrap_or_else(|error| self.status = error);
            }
            let action_key = session_identity_key(&session.id, session.profile.as_deref());
            let pin_pending = self.pending_session_pins.contains_key(&action_key);
            let unread_pending = self.pending_session_unreads.contains_key(&action_key);
            let archive_pending =
                self.pending_session_archives
                    .contains_key(&session_identity_key(
                        &session.id,
                        session.profile.as_deref(),
                    ));
            let rename_pending = self.pending_session_renames.contains_key(&action_key);
            let pin_verb = if session.pinned { "Unpin" } else { "Pin" };
            let pin_label = if scoped_profile_row {
                format!("{pin_verb} profile {profile_label} session {}", session.id)
            } else if session.pinned {
                format!("Unpin session {}", session.id)
            } else {
                format!("Pin session {}", session.id)
            };
            let unread_verb = if session.unread {
                "Mark read"
            } else {
                "Mark unread"
            };
            let unread_label = if scoped_profile_row {
                format!(
                    "{unread_verb} profile {profile_label} session {}",
                    session.id
                )
            } else if session.unread {
                format!("Mark read session {}", session.id)
            } else {
                format!("Mark unread session {}", session.id)
            };
            let archive_verb = if session.archived {
                "Unarchive"
            } else {
                "Archive"
            };
            let archive_label = if scoped_profile_row {
                format!(
                    "{archive_verb} profile {profile_label} session {}",
                    session.id
                )
            } else if session.archived {
                format!("Unarchive session {}", session.id)
            } else {
                format!("Archive session {}", session.id)
            };
            let menu = ui.menu_button("⋯", |ui| {
                let rename_label = if scoped_profile_row {
                    format!("Rename profile {profile_label} session {}", session.id)
                } else {
                    format!("Rename session {}", session.id)
                };
                if ui
                    .add_enabled(
                        self.gateway_connected && !rename_pending,
                        egui::Button::new(rename_label),
                    )
                    .clicked()
                {
                    self.rename_draft = session.title.clone();
                    self.rename_session = Some(session.clone());
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.gateway_connected && !pin_pending,
                        egui::Button::new(&pin_label),
                    )
                    .clicked()
                {
                    self.start_session_pin_update(session.clone(), !session.pinned);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.gateway_connected && !unread_pending,
                        egui::Button::new(&unread_label),
                    )
                    .clicked()
                {
                    self.start_session_unread_update(session.clone(), !session.unread);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.gateway_connected && !archive_pending,
                        egui::Button::new(&archive_label),
                    )
                    .clicked()
                {
                    self.start_session_archive_update(session.clone(), !session.archived);
                    ui.close();
                }
            });
            let menu_name = if scoped_profile_row {
                format!(
                    "Session actions profile {profile_label} session {}",
                    session.id
                )
            } else {
                format!("Session actions session {}", session.id)
            };
            menu.response.accessible_name(menu_name);
            if !session.preview.is_empty() {
                ui.label(RichText::new(&session.preview).size(11.0).color(MUTED));
            }
            ui.label(
                RichText::new(format!(
                    "{} · {} messages · {}{}",
                    session_age_label(
                        session_recency_timestamp(&session),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0.0, |duration| duration.as_secs_f64())
                    ),
                    session.message_count,
                    if session.source.is_empty() {
                        "Hermes"
                    } else {
                        &session.source
                    },
                    if scoped_profile_row {
                        format!(" · profile {profile_label}")
                    } else {
                        String::new()
                    }
                ))
                .size(10.0)
                .color(MUTED),
            );
            ui.add_space(4.0);
        }
    }

    fn sidebar_route(&mut self, ui: &mut egui::Ui, target: AppRoute) {
        let selected = self.route == target;
        if ui.selectable_label(selected, target.title()).clicked() {
            let path = target.path();
            self.execute_registered_action(RegisteredUiAction::Navigate { path })
                .unwrap_or_else(|error| self.status = error);
        }
    }

    fn workspace_content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(24.0);
        match self.route.clone() {
            AppRoute::NewChat => self.new_chat_content(ui),
            AppRoute::Session(session_id) => self.session_content(ui, &session_id),
            AppRoute::Capabilities => self.capabilities_content(ui),
            AppRoute::Messaging => {
                self.route_unavailable(ui, "Connect to Hermes to load messaging connections.")
            }
            AppRoute::Artifacts => {
                self.route_unavailable(ui, "Connect to Hermes to load artifacts.")
            }
            AppRoute::Extension(_) => self.route_unavailable(
                ui,
                "This contributed route has no registered native renderer.",
            ),
            AppRoute::Unknown(path) => {
                self.route_unavailable(ui, &format!("Unknown Hermes route: {path}"))
            }
            overlay if overlay.is_overlay() => {
                self.route_unavailable(ui, "This screen is shown as an overlay.")
            }
            _ => self.route_unavailable(ui, "Connect to Hermes to continue."),
        }
        if matches!(self.route, AppRoute::NewChat | AppRoute::Session(_)) {
            self.approval_cards(ui);
            self.clarify_cards(ui);
            self.batch_clarify_cards(ui);
            self.composer(ui);
        }
    }

    fn session_content(&mut self, ui: &mut Ui, session_id: &str) {
        if self.transcript.is_empty() {
            if !self.gateway_connected {
                self.route_unavailable(ui, "Connect to Hermes to open this session.");
            } else if self.transcript_has_more {
                let loading = self.older_transcript_request.is_some();
                let load_older = ui
                    .add_enabled(
                        !loading,
                        egui::Button::new(if loading {
                            "Loading earlier messages…"
                        } else {
                            "Show earlier messages"
                        }),
                    )
                    .accessible_name(if loading {
                        "Loading earlier messages"
                    } else {
                        "Show earlier messages"
                    });
                if load_older.clicked() {
                    self.load_older_transcript(session_id.to_owned());
                }
                if let Some(error) = &self.older_transcript_error {
                    ui.label(RichText::new(error).size(12.0).color(Color32::LIGHT_RED));
                }
            } else if self.active_runtime_session_id.is_some() {
                self.route_unavailable(ui, "This session has no messages yet.");
            } else {
                self.route_unavailable(ui, &format!("Loading session {session_id}…"));
            }
        } else {
            if self.transcript_has_more {
                let loading = self.older_transcript_request.is_some();
                let load_older = ui
                    .add_enabled(
                        self.gateway_connected && !loading,
                        egui::Button::new(if loading {
                            "Loading earlier messages…"
                        } else {
                            "Show earlier messages"
                        }),
                    )
                    .accessible_name(if loading {
                        "Loading earlier messages"
                    } else {
                        "Show earlier messages"
                    });
                if load_older.clicked() {
                    self.load_older_transcript(session_id.to_owned());
                }
            }
            if let Some(error) = &self.older_transcript_error {
                ui.label(RichText::new(error).size(12.0).color(Color32::LIGHT_RED));
            }
            let messages = self.transcript.clone();
            let markdown_cache = &mut self.markdown_cache;
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - 76.0).max(120.0))
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width().min(860.0));
                    for message in messages {
                        let (speaker, body) = transcript_message(&message);
                        let entry = ui.group(|ui| {
                            ui.label(RichText::new(speaker.clone()).strong().color(ACCENT));
                            render_transcript_markdown(ui, markdown_cache, &body);
                        });
                        entry.response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                Role::Group,
                                true,
                                format!("{speaker}: {body}"),
                            )
                        });
                        ui.add_space(8.0);
                    }
                });
        }
    }

    fn approval_cards(&mut self, ui: &mut Ui) {
        let Some(session_id) = self.active_runtime_session_id.as_deref() else {
            return;
        };
        let prompts = self
            .approval_prompts
            .values()
            .filter(|prompt| prompt.session_id == session_id)
            .cloned()
            .collect::<Vec<_>>();
        for prompt in prompts {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(RichText::new("Permission required").strong().color(ACCENT));
                if !prompt.command.is_empty() {
                    ui.label(RichText::new(&prompt.command).monospace().color(TEXT));
                }
                if !prompt.description.is_empty() {
                    ui.label(RichText::new(&prompt.description).color(MUTED));
                }
                ui.horizontal_wrapped(|ui| {
                    for choice in &prompt.choices {
                        let pending = prompt.response_pending.is_some();
                        let label = match choice.as_str() {
                            "once" => "Allow once",
                            "session" => "Allow this session",
                            "always" => "Always allow",
                            "deny" => "Deny",
                            _ => continue,
                        };
                        let response = ui
                            .add_enabled(
                                self.gateway_connected && !pending,
                                egui::Button::new(label),
                            )
                            .accessible_name(format!("{label} approval {}", prompt.prompt_id));
                        if response.clicked() {
                            self.execute_registered_action(RegisteredUiAction::RespondApproval {
                                prompt_id: prompt.prompt_id.clone(),
                                choice: choice.clone(),
                            })
                            .unwrap_or_else(|error| self.status = error);
                        }
                    }
                    if prompt.response_pending.is_some() {
                        ui.label(RichText::new("Sending decision…").color(MUTED));
                    }
                });
            });
        }
    }

    fn clarify_cards(&mut self, ui: &mut Ui) {
        let Some(session_id) = self.active_runtime_session_id.as_deref() else {
            return;
        };
        let prompts = self
            .clarify_prompts
            .values()
            .filter(|prompt| prompt.session_id == session_id)
            .cloned()
            .collect::<Vec<_>>();
        for prompt in prompts {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(RichText::new(&prompt.question).strong().color(ACCENT));
                if !prompt.choices.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for choice in &prompt.choices {
                            let selected = prompt.selected.contains(choice);
                            let response = ui.add_enabled_ui(
                                self.gateway_connected && prompt.response_pending.is_none(),
                                |ui| {
                                    ui.selectable_label(selected, choice)
                                        .accessible_name(format!(
                                            "Clarify choice {} for {}",
                                            choice, prompt.prompt_id
                                        ))
                                },
                            );
                            if response.inner.clicked()
                                && let Some(current) =
                                    self.clarify_prompts.get_mut(&prompt.prompt_id)
                            {
                                if current.multi_select {
                                    if let Some(index) = current
                                        .selected
                                        .iter()
                                        .position(|selected| selected == choice)
                                    {
                                        current.selected.remove(index);
                                    } else {
                                        current.selected.push(choice.clone());
                                    }
                                } else {
                                    current.selected.clear();
                                    current.selected.push(choice.clone());
                                }
                            }
                        }
                    });
                }
                let draft = self
                    .clarify_prompts
                    .get_mut(&prompt.prompt_id)
                    .map(|current| &mut current.draft);
                if let Some(draft) = draft {
                    ui.add(
                        egui::TextEdit::singleline(draft)
                            .desired_width(ui.available_width())
                            .hint_text("Type your answer…"),
                    )
                    .accessible_name(format!("Clarify answer {}", prompt.prompt_id));
                }
                ui.horizontal(|ui| {
                    let answer = clarify_answer(&prompt);
                    let pending = prompt.response_pending.is_some();
                    let submit = ui
                        .add_enabled(
                            self.gateway_connected && !pending && !answer.is_empty(),
                            egui::Button::new("Continue"),
                        )
                        .accessible_name(format!("Submit clarify answer {}", prompt.prompt_id));
                    if submit.clicked() {
                        self.execute_registered_action(RegisteredUiAction::RespondClarify {
                            prompt_id: prompt.prompt_id.clone(),
                            answer,
                        })
                        .unwrap_or_else(|error| self.status = error);
                    }
                    let skip = ui
                        .add_enabled(
                            self.gateway_connected && !pending,
                            egui::Button::new("Skip"),
                        )
                        .accessible_name(format!("Skip clarify request {}", prompt.prompt_id));
                    if skip.clicked() {
                        self.execute_registered_action(RegisteredUiAction::RespondClarify {
                            prompt_id: prompt.prompt_id.clone(),
                            answer: String::new(),
                        })
                        .unwrap_or_else(|error| self.status = error);
                    }
                    if pending {
                        ui.label(RichText::new("Sending answer…").color(MUTED));
                    }
                });
            });
        }
    }

    fn batch_clarify_cards(&mut self, ui: &mut Ui) {
        let Some(session_id) = self.active_runtime_session_id.as_deref() else {
            return;
        };
        let prompts = self
            .batch_clarify_prompts
            .values()
            .filter(|prompt| prompt.session_id == session_id)
            .cloned()
            .collect::<Vec<_>>();
        for prompt in prompts {
            ui.add_space(8.0);
            ui.group(|ui| {
                let answered = prompt
                    .questions
                    .iter()
                    .filter(|question| batch_question_answer(question).is_some())
                    .count();
                ui.label(
                    RichText::new(if prompt.questions.len() == 1 {
                        "1 question".to_owned()
                    } else {
                        format!("{answered} of {} answered", prompt.questions.len())
                    })
                    .size(11.0)
                    .color(MUTED),
                );
                for question in &prompt.questions {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&question.question).strong().color(TEXT))
                            .accessible_name(format!(
                                "Clarify question {}: {}",
                                question.qid, question.question
                            ));
                        if !question.choices.is_empty() {
                            ui.label(
                                RichText::new(if question.multi_select {
                                    "Select all that apply"
                                } else {
                                    "Pick one"
                                })
                                .size(10.0)
                                .color(MUTED),
                            );
                        }
                    });
                    if !question.choices.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            for choice in &question.choices {
                                let selected = question.selected.contains(choice);
                                let response = ui.add_enabled_ui(
                                    self.gateway_connected && !prompt.response_pending,
                                    |ui| {
                                        ui.selectable_label(selected, choice).accessible_name(
                                            format!("Clarify choice {choice} for {}", question.qid),
                                        )
                                    },
                                );
                                if response.inner.clicked()
                                    && let Some(current) = self
                                        .batch_clarify_prompts
                                        .get_mut(&prompt.server_request_id)
                                    && let Some(current_question) = current
                                        .questions
                                        .iter_mut()
                                        .find(|candidate| candidate.qid == question.qid)
                                {
                                    if current_question.multi_select {
                                        if let Some(index) = current_question
                                            .selected
                                            .iter()
                                            .position(|selected| selected == choice)
                                        {
                                            current_question.selected.remove(index);
                                        } else {
                                            current_question.selected.push(choice.clone());
                                        }
                                    } else {
                                        current_question.selected.clear();
                                        current_question.selected.push(choice.clone());
                                    }
                                }
                            }
                        });
                    }
                    let draft = self
                        .batch_clarify_prompts
                        .get_mut(&prompt.server_request_id)
                        .and_then(|current| {
                            current
                                .questions
                                .iter_mut()
                                .find(|candidate| candidate.qid == question.qid)
                        })
                        .map(|current| &mut current.draft);
                    if let Some(draft) = draft {
                        let changed = ui
                            .add(
                                egui::TextEdit::singleline(draft)
                                    .desired_width(ui.available_width())
                                    .hint_text("Type your answer…"),
                            )
                            .accessible_name(format!(
                                "Clarify answer {} for {}",
                                question.qid, prompt.server_request_id
                            ))
                            .changed();
                        if changed
                            && !question.multi_select
                            && let Some(current) = self
                                .batch_clarify_prompts
                                .get_mut(&prompt.server_request_id)
                            && let Some(current_question) = current
                                .questions
                                .iter_mut()
                                .find(|candidate| candidate.qid == question.qid)
                        {
                            current_question.selected.clear();
                        }
                    }
                }

                ui.horizontal(|ui| {
                    let answer = batch_clarify_answer(&prompt);
                    let submit = ui
                        .add_enabled(
                            self.gateway_connected && !prompt.response_pending && answer.is_some(),
                            egui::Button::new("Confirm and continue"),
                        )
                        .accessible_name(format!(
                            "Confirm and continue clarify answers {}",
                            prompt.server_request_id
                        ));
                    if submit.clicked()
                        && let Some(answer) = answer
                    {
                        self.execute_registered_action(RegisteredUiAction::RespondClarify {
                            prompt_id: prompt.server_request_id.clone(),
                            answer,
                        })
                        .unwrap_or_else(|error| self.status = error);
                    }
                    let skip = ui
                        .add_enabled(
                            self.gateway_connected && !prompt.response_pending,
                            egui::Button::new("Skip"),
                        )
                        .accessible_name(format!(
                            "Skip clarify request {}",
                            prompt.server_request_id
                        ));
                    if skip.clicked() {
                        self.execute_registered_action(RegisteredUiAction::RespondClarify {
                            prompt_id: prompt.server_request_id.clone(),
                            answer: String::new(),
                        })
                        .unwrap_or_else(|error| self.status = error);
                    }
                    if prompt.response_pending {
                        ui.label(RichText::new("Sending answers…").color(MUTED));
                    }
                });
            });
        }
    }

    fn capabilities_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab, label) in [
                (CapabilitiesTab::Skills, "Skills"),
                (CapabilitiesTab::Plugins, "Plugins"),
                (CapabilitiesTab::Connectors, "Connectors"),
                (CapabilitiesTab::Toolsets, "Toolsets"),
            ] {
                if ui
                    .selectable_label(self.capabilities_tab == tab, label)
                    .clicked()
                {
                    self.execute_registered_action(RegisteredUiAction::SelectCapabilitiesTab {
                        tab,
                    })
                    .unwrap_or_else(|error| self.capabilities_error = Some(error));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let refresh = ui.add_enabled(
                    self.gateway_connected && !self.capabilities_pending,
                    egui::Button::new("Refresh"),
                );
                if refresh.clicked() {
                    self.execute_registered_action(RegisteredUiAction::RefreshCapabilities)
                        .unwrap_or_else(|error| self.capabilities_error = Some(error));
                }
            });
        });
        ui.separator();
        if matches!(
            self.capabilities_tab,
            CapabilitiesTab::Skills | CapabilitiesTab::Toolsets
        ) {
            ui.add(
                egui::TextEdit::singleline(&mut self.capabilities_query)
                    .hint_text("Search capabilities")
                    .desired_width(ui.available_width()),
            )
            .accessible_name("Search capabilities");
        } else {
            ui.add_space(28.0);
        }
        ui.add_space(12.0);
        if self.capabilities_pending {
            ui.label(
                RichText::new("Loading capabilities…")
                    .size(12.0)
                    .color(MUTED),
            );
            return;
        }
        if let Some(error) = &self.capabilities_error {
            ui.label(
                RichText::new(format!("Capabilities could not be loaded: {error}"))
                    .size(12.0)
                    .color(MUTED),
            );
            return;
        }
        match self.capabilities_tab {
            CapabilitiesTab::Skills => {
                let Some(skills) = &self.skills else {
                    self.route_unavailable(ui, "Connect to Hermes to browse installed Skills.");
                    return;
                };
                let query = self.capabilities_query.trim().to_lowercase();
                let visible = skills
                    .iter()
                    .filter(|skill| {
                        query.is_empty()
                            || skill.name.to_lowercase().contains(&query)
                            || skill.category.to_lowercase().contains(&query)
                            || skill.description.to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                ui.label(
                    RichText::new(format!("Installed Skills · {}", visible.len()))
                        .strong()
                        .color(TEXT),
                );
                if visible.is_empty() {
                    self.empty_panel(
                        ui,
                        if query.is_empty() {
                            "No installed Skills"
                        } else {
                            "No matching Skills"
                        },
                    );
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for skill in visible {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&skill.name).strong().color(TEXT));
                                ui.label(
                                    RichText::new(if skill.enabled {
                                        "Enabled"
                                    } else {
                                        "Disabled"
                                    })
                                    .color(MUTED),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(
                                            skill.provenance.as_deref().unwrap_or("local"),
                                        )
                                        .color(MUTED),
                                    );
                                });
                            });
                            if !skill.category.is_empty() {
                                ui.label(RichText::new(&skill.category).size(11.0).color(ACCENT));
                            }
                            if !skill.description.is_empty() {
                                ui.label(RichText::new(&skill.description).size(12.0).color(MUTED));
                            }
                        });
                        ui.add_space(6.0);
                    }
                });
            }
            CapabilitiesTab::Toolsets => {
                let Some(toolsets) = &self.toolsets else {
                    self.route_unavailable(ui, "Connect to Hermes to browse Toolsets.");
                    return;
                };
                let query = self.capabilities_query.trim().to_lowercase();
                let visible = toolsets
                    .iter()
                    .filter(|toolset| {
                        query.is_empty()
                            || toolset.name.to_lowercase().contains(&query)
                            || toolset.label.to_lowercase().contains(&query)
                            || toolset.description.to_lowercase().contains(&query)
                            || toolset
                                .tools
                                .iter()
                                .any(|tool| tool.to_lowercase().contains(&query))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                ui.label(
                    RichText::new(format!("Toolsets · {}", visible.len()))
                        .strong()
                        .color(TEXT),
                );
                if visible.is_empty() {
                    self.empty_panel(
                        ui,
                        if query.is_empty() {
                            "No Toolsets available"
                        } else {
                            "No matching Toolsets"
                        },
                    );
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for toolset in visible {
                        let update_pending =
                            self.pending_toolset_updates.contains_key(&toolset.name);
                        let display_name = if toolset.label.is_empty() {
                            toolset.name.clone()
                        } else {
                            toolset.label.clone()
                        };
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(if toolset.label.is_empty() {
                                        &toolset.name
                                    } else {
                                        &toolset.label
                                    })
                                    .strong()
                                    .color(TEXT),
                                );
                                ui.label(
                                    RichText::new(if toolset.enabled {
                                        "Enabled"
                                    } else {
                                        "Disabled"
                                    })
                                    .color(MUTED),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(if toolset.configured {
                                            "Configured"
                                        } else {
                                            "Needs setup"
                                        })
                                        .color(MUTED),
                                    );
                                });
                                let mut enabled = toolset.enabled;
                                let toggle = ui
                                    .add_enabled(
                                        !update_pending,
                                        egui::Checkbox::new(&mut enabled, ""),
                                    )
                                    .on_hover_text(if toolset.enabled {
                                        format!("Disable {display_name}")
                                    } else {
                                        format!("Enable {display_name}")
                                    })
                                    .accessible_name(format!("Toggle Toolset {display_name}"));
                                if toggle.changed() {
                                    self.execute_registered_action(
                                        RegisteredUiAction::SetToolsetEnabled {
                                            name: toolset.name.clone(),
                                            enabled,
                                        },
                                    )
                                    .unwrap_or_else(|error| self.capabilities_error = Some(error));
                                }
                                if update_pending {
                                    ui.label(RichText::new("Saving…").size(11.0).color(MUTED));
                                }
                            });
                            if !toolset.description.is_empty() {
                                ui.label(
                                    RichText::new(&toolset.description).size(12.0).color(MUTED),
                                );
                            }
                            if !toolset.tools.is_empty() {
                                ui.label(
                                    RichText::new(toolset.tools.join(" · "))
                                        .size(11.0)
                                        .color(ACCENT),
                                );
                            }
                        });
                        ui.add_space(6.0);
                    }
                });
            }
            CapabilitiesTab::Plugins => {
                self.route_unavailable(
                    ui,
                    "Plugin inventory and package actions are not connected yet.",
                );
            }
            CapabilitiesTab::Connectors => {
                self.route_unavailable(
                    ui,
                    "Connector inventory and account actions are not connected yet.",
                );
            }
        }
    }

    fn new_chat_content(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.15).max(8.0));
            ui.label(
                RichText::new("Welcome to Hermes")
                    .size(24.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Connect to your Hermes backend to begin a session.")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(18.0);
            ui.horizontal_centered(|ui| {
                let connect = ui.add_sized(
                    [190.0, 38.0],
                    egui::Button::new(RichText::new("Connect to Hermes").color(TEXT))
                        .fill(Color32::from_rgb(57, 103, 163)),
                );
                if connect.clicked() {
                    self.status = "Backend offline · gateway connection is not configured".into();
                }
                ui.add_sized(
                    [170.0, 38.0],
                    egui::Button::new(RichText::new("Install Hermes locally").color(TEXT)),
                );
            });
        });
    }

    fn composer(&mut self, ui: &mut Ui) {
        let session_id = self.active_runtime_session_id.clone();
        let pending_key = session_id.as_deref().map(|session_id| {
            session_identity_key(session_id, self.active_session_profile.as_deref())
        });
        let pending = pending_key
            .as_ref()
            .and_then(|key| self.pending_submissions.get(key))
            .cloned();
        let waiting_for_this_session = pending.is_some();
        let waiting_for_new_session = session_id.is_none() && self.pending_create.is_some();
        let can_submit =
            self.gateway_connected && !waiting_for_this_session && !waiting_for_new_session;
        let composer_width = (ui.available_width() - 72.0).max(160.0);
        ui.separator();
        ui.horizontal(|ui| {
            let input = ui
                .add_enabled(
                    self.gateway_connected && !waiting_for_new_session,
                    egui::TextEdit::multiline(&mut self.draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(2)
                        .desired_width(composer_width)
                        .hint_text(if self.gateway_connected {
                            "Message Hermes…"
                        } else {
                            "Waiting for Hermes gateway…"
                        }),
                )
                .accessible_name("Message composer");
            input.on_hover_text("Type a message to Hermes");
            if waiting_for_this_session {
                let uncertain = pending.as_ref().is_some_and(|pending| pending.uncertain);
                if uncertain {
                    if ui
                        .add_enabled(
                            self.gateway_connected,
                            egui::Button::new("Retry same message"),
                        )
                        .accessible_name("Retry message with original submission ID")
                        .clicked()
                    {
                        self.retry_uncertain_prompt();
                    }
                } else {
                    let stopping =
                        self.pending_interrupt
                            .as_ref()
                            .is_some_and(|(_, interrupt_session)| {
                                Some(interrupt_session) == session_id.as_ref()
                            });
                    let stop = ui
                        .add_enabled(
                            self.gateway_connected && !stopping,
                            egui::Button::new(if stopping { "Stopping…" } else { "Stop" }),
                        )
                        .accessible_name(if stopping {
                            "Stopping response"
                        } else {
                            "Stop response"
                        });
                    if stop.clicked() {
                        self.execute_registered_action(RegisteredUiAction::StopGeneration)
                            .unwrap_or_else(|error| self.status = error);
                    }
                }
            } else {
                let button = ui.add_enabled(
                    can_submit && !self.draft.trim().is_empty(),
                    egui::Button::new("Send"),
                );
                if button.clicked() {
                    if let Some(session_id) = session_id {
                        self.submit_prompt(session_id);
                    } else {
                        self.create_session_from_draft();
                    }
                }
            }
        });
        if waiting_for_this_session || waiting_for_new_session {
            ui.label(RichText::new("Waiting for Hermes…").size(11.0).color(MUTED));
        }
    }

    fn load_older_transcript(&mut self, session_id: String) {
        if self.older_transcript_request.is_some()
            || !self.transcript_has_more
            || self.route != AppRoute::Session(session_id.clone())
        {
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.older_transcript_error = Some("Hermes gateway is unavailable".into());
            return;
        };
        let request_id = operation_id("transcript-older");
        let profile = self.active_session_profile.clone();
        let offset = self.transcript.len();
        if let Err(error) = gateway.load_older_transcript(
            request_id.clone(),
            session_id.clone(),
            profile.clone(),
            offset,
        ) {
            self.older_transcript_error = Some(error.clone());
            self.status = error;
            return;
        }
        self.older_transcript_error = None;
        self.older_transcript_request = Some((request_id, session_id, profile, offset));
        self.save_active_chat_view();
    }

    fn create_session_from_draft(&mut self) {
        let text = self.draft.trim().to_owned();
        if text.is_empty() {
            return;
        }
        self.begin_create_session(text);
    }

    fn create_blank_session_tab(&mut self) {
        self.save_active_chat_view();
        if let Some((session_id, profile)) = self.untouched_blank_session.clone() {
            let key = session_identity_key(&session_id, profile.as_deref());
            let still_untouched = self
                .session_chat_views
                .get(&key)
                .is_none_or(|view| view.transcript.is_empty() && view.draft.trim().is_empty());
            let owner_profile = self
                .gateway_endpoint
                .as_ref()
                .map(|endpoint| endpoint.profile_id.clone())
                .or_else(|| self.active_session_profile.clone());
            if owner_profile == profile && still_untouched {
                match self.workspace.open_session_pane(
                    profile.as_deref(),
                    &session_id,
                    "New session",
                ) {
                    Ok(_) => {
                        self.active_session_profile = profile;
                        self.navigate_after_save(AppRoute::Session(session_id));
                        self.status = "Reused the untouched new-session tab".into();
                        return;
                    }
                    Err(error) => {
                        self.status = error;
                        return;
                    }
                }
            }
            self.untouched_blank_session = None;
        }
        self.begin_create_session(String::new());
    }

    fn begin_create_session(&mut self, text: String) {
        if self.pending_create.is_some() {
            return;
        }
        let (Some(gateway), Some(endpoint)) = (&self.gateway_discovery, &self.gateway_endpoint)
        else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        let request_id = operation_id("create");
        let params = serde_json::json!({
            "cols": 96,
            "source": "desktop",
            "profile": endpoint.profile_id,
            "request_id": request_id
        });
        if let Err(error) = gateway.create_session(request_id.clone(), params) {
            self.status = error;
            return;
        }
        self.pending_create = Some((request_id, text));
        self.status = "Creating Hermes session…".into();
    }

    fn start_session_pin_update(&mut self, session: SessionSummary, pinned: bool) {
        let session_id = session.id.clone();
        let action_key = session_identity_key(&session.id, session.profile.as_deref());
        if self.pending_session_pins.contains_key(&action_key) {
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        let request_id = operation_id("session-pin");
        if let Err(error) = gateway.set_session_pinned(
            request_id.clone(),
            session_id,
            session.profile.clone(),
            pinned,
        ) {
            self.status = error;
            return;
        }
        self.pending_session_pins.insert(action_key, request_id);
        self.status = "Updating session pin…".into();
    }

    fn start_session_unread_update(&mut self, session: SessionSummary, unread: bool) {
        let session_id = session.id.clone();
        let action_key = session_identity_key(&session.id, session.profile.as_deref());
        if self.pending_session_unreads.contains_key(&action_key) {
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        let request_id = operation_id("session-unread");
        if let Err(error) = gateway.set_session_unread(
            request_id.clone(),
            session_id,
            session.profile.clone(),
            unread,
        ) {
            self.status = error;
            return;
        }
        self.pending_session_unreads.insert(action_key, request_id);
        self.status = if unread {
            "Marking session unread…".into()
        } else {
            "Marking session read…".into()
        };
    }

    fn start_session_rename_update(&mut self, session: SessionSummary) {
        let title = self.rename_draft.trim().to_owned();
        if title == session.title.trim() {
            self.rename_session = None;
            return;
        }
        if title.len() > 16_384 || title.chars().any(char::is_control) {
            self.status = "Session title is invalid".into();
            return;
        }
        let action_key = session_identity_key(&session.id, session.profile.as_deref());
        if self.pending_session_renames.contains_key(&action_key) {
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        let request_id = operation_id("session-rename");
        let mut runtime_rename = None;
        let rename_result = if self.route == AppRoute::Session(session.id.clone())
            && self.active_session_profile == session.profile
        {
            if let Some(runtime_session_id) = &self.active_runtime_session_id {
                runtime_rename = Some((runtime_session_id.clone(), title.clone()));
                gateway.rename_runtime_session(
                    request_id.clone(),
                    session.id.clone(),
                    runtime_session_id.clone(),
                    session.profile.clone(),
                    title.clone(),
                )
            } else {
                gateway.rename_session(request_id.clone(), session.id, session.profile, title)
            }
        } else {
            gateway.rename_session(request_id.clone(), session.id, session.profile, title)
        };
        if let Err(error) = rename_result {
            self.status = error;
            return;
        }
        if let Some(runtime_rename) = runtime_rename {
            self.runtime_session_renames
                .insert(action_key.clone(), runtime_rename);
        }
        self.pending_session_renames.insert(action_key, request_id);
        self.status = "Renaming session…".into();
    }

    fn retry_uncertain_session_rename(&mut self, session: SessionSummary) {
        let action_key = session_identity_key(&session.id, session.profile.as_deref());
        if !self.uncertain_session_renames.contains(&action_key) {
            return;
        }
        let Some(request_id) = self.pending_session_renames.get(&action_key).cloned() else {
            return;
        };
        let Some((runtime_session_id, title)) = self.runtime_session_renames.get(&action_key)
        else {
            return;
        };
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable for exact rename retry".into();
            return;
        };
        match gateway.retry_runtime_session_rename(
            request_id,
            session.id,
            runtime_session_id.clone(),
            session.profile,
            title.clone(),
        ) {
            Ok(()) => {
                self.uncertain_session_renames.remove(&action_key);
                self.status = "Checking rename outcome…".into();
            }
            Err(error) => self.status = error,
        }
    }

    fn start_session_archive_update(&mut self, session: SessionSummary, archived: bool) {
        let session_id = session.id.clone();
        let action_key = session_identity_key(&session.id, session.profile.as_deref());
        if self.pending_session_archives.contains_key(&action_key) {
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        let request_id = operation_id("session-archive");
        if let Err(error) = gateway.set_session_archived(
            request_id.clone(),
            session_id,
            session.profile.clone(),
            archived,
        ) {
            self.status = error;
            return;
        }
        self.pending_session_archives.insert(action_key, request_id);
        self.status = if archived {
            "Archiving session…".into()
        } else {
            "Restoring session…".into()
        };
    }

    fn load_archived_sessions(&mut self) {
        self.archived_sessions_error = None;
        if !self.gateway_connected {
            self.archived_sessions_error = Some("Hermes gateway is unavailable".into());
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.archived_sessions_error = Some("Hermes gateway is unavailable".into());
            return;
        };
        let request_id = operation_id("archived-sessions");
        match gateway.load_archived_sessions(request_id.clone()) {
            Ok(()) => {
                self.archived_sessions_request_id = Some(request_id);
                self.archived_sessions_loading = true;
            }
            Err(error) => self.archived_sessions_error = Some(error),
        }
    }

    fn submit_prompt(&mut self, session_id: String) {
        let text = self.draft.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let profile = self.active_session_profile.clone();
        let pending_key = session_identity_key(&session_id, profile.as_deref());
        if self.pending_submissions.contains_key(&pending_key) {
            self.status =
                "Finish or recover this session's pending message before sending another".into();
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        let submission_id = operation_id("submit");
        if let Err(error) = gateway.submit_prompt(
            session_id.clone(),
            self.active_session_profile.clone(),
            submission_id.clone(),
            text.clone(),
        ) {
            self.status = error;
            return;
        }
        if let AppRoute::Session(stored_session_id) = &self.route
            && self
                .untouched_blank_session
                .as_ref()
                .is_some_and(|(blank_id, blank_profile)| {
                    blank_id == stored_session_id && blank_profile == &self.active_session_profile
                })
        {
            self.untouched_blank_session = None;
        }
        self.transcript.push(serde_json::json!({
            "id":submission_id,
            "submission_id":submission_id,
            "role":"user",
            "content":text
        }));
        self.pending_submissions.insert(
            pending_key,
            PendingPrompt {
                session_id: session_id.clone(),
                profile,
                submission_id: submission_id.clone(),
                text,
                uncertain: false,
                admission_id: None,
                execution_generation: None,
                resolving_unknown: false,
            },
        );
        self.draft.clear();
        self.status = "Sending message to Hermes…".into();
    }

    fn retry_uncertain_prompt(&mut self) {
        let Some(session_id) = self.active_runtime_session_id.as_deref() else {
            return;
        };
        let key = session_identity_key(session_id, self.active_session_profile.as_deref());
        let Some(pending) = self.pending_submissions.get(&key).cloned() else {
            return;
        };
        if !pending.uncertain {
            return;
        }
        let Some(gateway) = &self.gateway_discovery else {
            self.status = "Hermes gateway is unavailable".into();
            return;
        };
        match gateway.submit_prompt(
            pending.session_id.clone(),
            pending.profile.clone(),
            pending.submission_id.clone(),
            pending.text.clone(),
        ) {
            Ok(()) => {
                if let Some(pending) = self.pending_submissions.get_mut(&key) {
                    pending.uncertain = false;
                }
                self.status = "Checking original message admission…".into();
            }
            Err(error) => self.status = error,
        }
    }

    fn handle_prompt_submitted(
        &mut self,
        session_id: String,
        submission_id: String,
        result: Result<PromptSubmitReceipt, String>,
    ) {
        let Some(key) = self.pending_submissions.iter().find_map(|(key, pending)| {
            (pending.session_id == session_id && pending.submission_id == submission_id)
                .then(|| key.clone())
        }) else {
            return;
        };
        let Some(pending) = self.pending_submissions.get(&key).cloned() else {
            return;
        };
        let is_active = self.active_runtime_session_id.as_deref() == Some(session_id.as_str())
            && self.active_session_profile == pending.profile;
        match result {
            Ok(PromptSubmitReceipt::Admitted {
                admission_id,
                status,
                execution_generation,
            }) => {
                if let Some(stored_session_id) =
                    self.stored_session_id_for_runtime(&session_id, pending.profile.as_deref())
                {
                    self.record_session_in_sidebar(
                        &stored_session_id,
                        pending.profile.as_deref(),
                        &pending.text,
                    );
                }
                if let Some(transcript) =
                    self.transcript_for_runtime_mut(&session_id, pending.profile.as_deref())
                    && let Some(message) = transcript.iter_mut().find(|message| {
                        message
                            .get("submission_id")
                            .and_then(|value| value.as_str())
                            == Some(submission_id.as_str())
                    })
                {
                    message["admission_id"] = serde_json::Value::String(admission_id.clone());
                }
                if let Some(pending) = self.pending_submissions.get_mut(&key) {
                    pending.admission_id = Some(admission_id.clone());
                    pending.execution_generation = execution_generation;
                    pending.uncertain = false;
                }
                if status == "terminal" || self.recent_completed_admissions.contains(&admission_id)
                {
                    self.pending_submissions.remove(&key);
                }
                if is_active {
                    self.status = match status.as_str() {
                        "queued" => "Message queued in Hermes".into(),
                        "unknown" => "Message admitted; execution outcome needs recovery".into(),
                        "terminal" => "Hermes message already settled".into(),
                        _ => "Message accepted by Hermes".into(),
                    };
                    if status == "unknown" {
                        self.resolve_unknown_submission();
                    }
                }
            }
            Ok(PromptSubmitReceipt::VoiceStopped) => {
                if let Some(transcript) =
                    self.transcript_for_runtime_mut(&session_id, pending.profile.as_deref())
                {
                    transcript.retain(|message| {
                        message
                            .get("submission_id")
                            .and_then(|value| value.as_str())
                            != Some(submission_id.as_str())
                    });
                }
                self.pending_submissions.remove(&key);
                if is_active {
                    self.status = "Voice chat stopped; no message was admitted".into();
                }
            }
            Err(error)
                if error.starts_with("Hermes prompt.submit refused:")
                    || error == "Hermes session profile scope is invalid" =>
            {
                let message = self
                    .transcript_for_runtime_mut(&session_id, pending.profile.as_deref())
                    .and_then(|transcript| {
                        transcript
                            .iter()
                            .position(|message| {
                                message
                                    .get("submission_id")
                                    .and_then(|value| value.as_str())
                                    == Some(submission_id.as_str())
                            })
                            .map(|index| transcript.remove(index))
                    });
                if is_active {
                    if let Some(message) = message {
                        self.draft = message
                            .get("content")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_owned();
                    }
                } else if let Some(message) = message {
                    self.set_cached_draft_for_runtime(
                        &session_id,
                        pending.profile.as_deref(),
                        message
                            .get("content")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                    );
                }
                self.pending_submissions.remove(&key);
                if is_active {
                    self.status = error;
                }
            }
            Err(error) => {
                if let Some(pending) = self.pending_submissions.get_mut(&key) {
                    pending.uncertain = true;
                }
                if is_active {
                    self.status =
                        format!("Message admission unknown; retry the same message: {error}");
                }
            }
        }
    }

    fn transcript_for_runtime_mut(
        &mut self,
        runtime_session_id: &str,
        profile: Option<&str>,
    ) -> Option<&mut Vec<serde_json::Value>> {
        if self.active_runtime_session_id.as_deref() == Some(runtime_session_id)
            && self.active_session_profile.as_deref() == profile
        {
            return Some(&mut self.transcript);
        }
        let key = self.session_chat_views.iter().find_map(|(key, view)| {
            (view.runtime_session_id.as_deref() == Some(runtime_session_id)
                && view.profile.as_deref() == profile)
                .then(|| key.clone())
        })?;
        self.session_chat_views
            .get_mut(&key)
            .map(|view| &mut view.transcript)
    }

    fn set_cached_draft_for_runtime(
        &mut self,
        runtime_session_id: &str,
        profile: Option<&str>,
        draft: String,
    ) {
        if let Some(view) = self.session_chat_views.values_mut().find(|view| {
            view.runtime_session_id.as_deref() == Some(runtime_session_id)
                && view.profile.as_deref() == profile
        }) {
            view.draft = draft;
        }
    }

    fn stored_session_id_for_runtime(
        &self,
        runtime_session_id: &str,
        profile: Option<&str>,
    ) -> Option<String> {
        if self.active_runtime_session_id.as_deref() == Some(runtime_session_id)
            && self.active_session_profile.as_deref() == profile
            && let AppRoute::Session(stored_session_id) = &self.route
        {
            return Some(stored_session_id.clone());
        }
        self.session_chat_views.iter().find_map(|(key, view)| {
            (view.runtime_session_id.as_deref() == Some(runtime_session_id)
                && view.profile.as_deref() == profile)
                .then(|| {
                    key.split_once('\0')
                        .map(|(_, stored_id)| stored_id.to_owned())
                })
                .flatten()
        })
    }

    fn record_session_in_sidebar(&mut self, stored_id: &str, profile: Option<&str>, text: &str) {
        if self
            .sessions
            .iter()
            .any(|session| session.id == stored_id && session.profile.as_deref() == profile)
        {
            return;
        }
        let title = text.chars().take(72).collect::<String>();
        self.sessions.insert(
            0,
            SessionSummary {
                id: stored_id.into(),
                profile: profile.map(str::to_owned),
                title,
                preview: text.chars().take(120).collect(),
                message_count: 0,
                source: "desktop".into(),
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0.0, |duration| duration.as_secs_f64()),
                last_active: None,
                pinned: false,
                unread: false,
                archived: false,
            },
        );
    }

    fn resolve_unknown_submission(&mut self) {
        let Some(session_id) = self.active_runtime_session_id.as_deref() else {
            return;
        };
        let key = session_identity_key(session_id, self.active_session_profile.as_deref());
        let Some(pending) = self.pending_submissions.get(&key) else {
            return;
        };
        if pending.resolving_unknown {
            return;
        }
        let (Some(admission_id), Some(generation)) =
            (pending.admission_id.clone(), pending.execution_generation)
        else {
            return;
        };
        let Some(gateway) = &self.gateway_discovery else {
            return;
        };
        match gateway.resolve_unknown_prompt(
            session_id.to_owned(),
            pending.profile.clone(),
            admission_id,
            generation,
        ) {
            Ok(()) => {
                if let Some(pending) = self.pending_submissions.get_mut(&key) {
                    pending.resolving_unknown = true;
                }
                self.status = "Resolving unknown Hermes message…".into();
            }
            Err(error) => self.status = error,
        }
    }

    fn handle_server_request(
        &mut self,
        server_request_id: String,
        method: String,
        params: serde_json::Value,
    ) {
        if method == "clarify" {
            self.handle_batch_clarify_request(server_request_id, params);
            return;
        }
        if method != "approval" {
            self.reject_server_request(
                server_request_id,
                -32601,
                "No native handler for this request",
            );
            return;
        }
        let session_id = params
            .get("session_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty());
        let Some(session_id) = session_id else {
            self.reject_server_request(
                server_request_id,
                -32602,
                "Approval request omitted its session ID",
            );
            return;
        };
        let request_profile = params
            .get("profile")
            .and_then(|value| value.as_str())
            .or_else(|| {
                self.gateway_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.profile_id.as_str())
            });
        if !self.session_runtime_is_known(session_id, request_profile) {
            self.reject_server_request(
                server_request_id,
                4404,
                "This window has no unique open session pane for the request",
            );
            return;
        }
        let prompt_id = params
            .get("request_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(&server_request_id)
            .to_owned();
        let allow_permanent = params
            .get("allow_permanent")
            .and_then(|value| value.as_bool())
            != Some(false);
        let choices = params
            .get("choices")
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|choice| choice.as_str())
                    .filter(|choice| {
                        matches!(*choice, "once" | "session" | "always" | "deny")
                            && (allow_permanent || *choice != "always")
                    })
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if choices.is_empty() {
            self.reject_server_request(
                server_request_id,
                -32602,
                "Approval request offered no supported choices",
            );
            return;
        }
        if self.approval_prompts.contains_key(&prompt_id) {
            self.reject_server_request(
                server_request_id,
                -32600,
                "Approval request is already being shown",
            );
            return;
        }
        self.approval_prompts.insert(
            prompt_id.clone(),
            ApprovalPrompt {
                session_id: session_id.to_owned(),
                profile: request_profile.map(str::to_owned),
                execution_generation: 0,
                prompt_id,
                server_request_id: Some(server_request_id),
                command: params
                    .get("command")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                description: params
                    .get("description")
                    .and_then(|value| value.as_str())
                    .unwrap_or("dangerous command")
                    .to_owned(),
                choices,
                response_pending: None,
            },
        );
        self.status = "A tool approval is waiting for your decision".into();
    }

    fn session_runtime_is_known(&self, runtime_session_id: &str, profile: Option<&str>) -> bool {
        if let (Some(request_profile), Some(endpoint)) = (profile, self.gateway_endpoint.as_ref())
            && request_profile != endpoint.profile_id
        {
            return false;
        }
        let mut owners = BTreeSet::new();
        if self.active_runtime_session_id.as_deref() == Some(runtime_session_id)
            && profile.is_none_or(|profile| self.active_session_profile.as_deref() == Some(profile))
            && let AppRoute::Session(stored_id) = &self.route
        {
            owners.insert(session_identity_key(
                stored_id,
                self.active_session_profile.as_deref(),
            ));
        }
        for (key, view) in &self.session_chat_views {
            if view.runtime_session_id.as_deref() == Some(runtime_session_id)
                && profile.is_none_or(|profile| view.profile.as_deref() == Some(profile))
            {
                owners.insert(key.clone());
            }
        }
        owners.len() == 1
    }

    fn handle_batch_clarify_request(
        &mut self,
        server_request_id: String,
        params: serde_json::Value,
    ) {
        let session_id = params
            .get("session_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty());
        let Some(session_id) = session_id else {
            self.reject_server_request(
                server_request_id,
                -32602,
                "Clarify request omitted its session ID",
            );
            return;
        };
        let request_profile = params
            .get("profile")
            .and_then(|value| value.as_str())
            .or_else(|| {
                self.gateway_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.profile_id.as_str())
            });
        if !self.session_runtime_is_known(session_id, request_profile) {
            self.reject_server_request(
                server_request_id,
                4404,
                "This window has no unique open session pane for the request",
            );
            return;
        }
        let rows = params
            .get("questions")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        if rows.len() > 5 {
            self.reject_server_request(
                server_request_id,
                -32602,
                "Clarify requests may contain at most five questions",
            );
            return;
        }
        if rows.is_empty() {
            self.reject_server_request(
                server_request_id,
                -32602,
                "Clarify requests must contain at least one question",
            );
            return;
        }
        let replay_answers = params.get("answers").and_then(|value| value.as_object());
        let mut questions = Vec::new();
        let mut seen_ids = BTreeSet::new();
        for row in rows {
            let Some(row) = row.as_object() else {
                continue;
            };
            let qid = row
                .get("qid")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .trim();
            let question = row
                .get("question")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .trim();
            if qid.is_empty()
                || question.is_empty()
                || qid.chars().count() > 128
                || question.chars().count() > 16_000
                || !seen_ids.insert(qid.to_owned())
            {
                continue;
            }
            let choices = row
                .get("choices")
                .and_then(|value| value.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str())
                        .filter(|value| {
                            !value.trim().is_empty()
                                && bare_clarify_choice(value).chars().count() <= 8_000
                        })
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let multi_select = !choices.is_empty()
                && row
                    .get("multi_select")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
            let mut selected = Vec::new();
            let mut draft = String::new();
            if let Some(answer) = replay_answers
                .and_then(|answers| answers.get(qid))
                .and_then(serde_json::Value::as_str)
            {
                let values = if multi_select {
                    serde_json::from_str::<Vec<String>>(answer)
                        .unwrap_or_else(|_| vec![answer.to_owned()])
                } else {
                    vec![answer.to_owned()]
                };
                for value in values {
                    if let Some(choice) = choices
                        .iter()
                        .find(|choice| bare_clarify_choice(choice) == value)
                    {
                        if !selected.contains(choice) {
                            selected.push(choice.clone());
                        }
                    } else if multi_select {
                        if !draft.is_empty() {
                            draft.push_str(", ");
                        }
                        draft.push_str(&value);
                    } else {
                        draft = value;
                    }
                }
            }
            questions.push(BatchClarifyQuestion {
                qid: qid.to_owned(),
                question: question.to_owned(),
                choices,
                multi_select,
                selected,
                draft,
            });
        }
        if questions.is_empty() {
            if let Some(gateway) = &self.gateway_discovery {
                let _ =
                    gateway.respond_server_request(server_request_id, Ok(serde_json::json!({})));
            }
            return;
        }
        if self.batch_clarify_prompts.contains_key(&server_request_id) {
            self.reject_server_request(
                server_request_id,
                -32600,
                "Clarify request is already being shown",
            );
            return;
        }
        self.batch_clarify_prompts.insert(
            server_request_id.clone(),
            BatchClarifyPrompt {
                session_id: session_id.to_owned(),
                profile: request_profile.map(str::to_owned),
                server_request_id,
                questions,
                response_pending: false,
                pending_locks: VecDeque::new(),
                lock_in_flight: None,
            },
        );
        self.status = "Hermes has questions for you".into();
    }

    fn reject_server_request(&self, id: String, code: i64, message: &str) {
        if let Some(gateway) = &self.gateway_discovery {
            let _ = gateway.respond_server_request(id, Err((code, message.to_owned())));
        }
    }

    fn handle_server_request_answered(&mut self, id: String, result: Result<(), String>) {
        if let Some(prompt_id) = self
            .batch_clarify_prompts
            .keys()
            .find(|prompt_id| prompt_id.as_str() == id.as_str())
            .cloned()
        {
            match result {
                Ok(()) => {
                    self.batch_clarify_prompts.remove(&prompt_id);
                    self.status = "Clarify request closed".into();
                }
                Err(error) => {
                    if let Some(prompt) = self.batch_clarify_prompts.get_mut(&prompt_id) {
                        prompt.response_pending = false;
                    }
                    self.status = error;
                }
            }
            return;
        }
        let Some(prompt_id) = self
            .approval_prompts
            .iter()
            .find_map(|(prompt_id, prompt)| {
                (prompt.server_request_id.as_deref() == Some(&id)).then(|| prompt_id.clone())
            })
        else {
            return;
        };
        match result {
            Ok(()) => {
                self.approval_prompts.remove(&prompt_id);
                self.status = "Approval decision sent to Hermes".into();
            }
            Err(error) => {
                if let Some(prompt) = self.approval_prompts.get_mut(&prompt_id) {
                    prompt.response_pending = None;
                }
                self.status = error;
            }
        }
    }

    fn send_next_batch_clarify_lock(&mut self, prompt_id: &str) -> Result<(), String> {
        let Some(prompt) = self.batch_clarify_prompts.get(prompt_id) else {
            return Err("This clarify request is no longer pending".into());
        };
        if prompt.lock_in_flight.is_some() {
            return Err("A clarify answer is already being saved".into());
        }
        let Some(lock) = prompt.pending_locks.front().cloned() else {
            return Err("No clarify answers remain to send".into());
        };
        let request_id = operation_id("clarify-lock");
        self.gateway_discovery
            .as_ref()
            .ok_or_else(|| "Hermes gateway writer is unavailable".to_owned())?
            .lock_clarify(
                request_id.clone(),
                prompt.server_request_id.clone(),
                lock.question_id.clone(),
                prompt.profile.clone(),
                lock.answer,
            )?;
        if let Some(prompt) = self.batch_clarify_prompts.get_mut(prompt_id) {
            prompt.lock_in_flight = Some((request_id, lock.question_id));
        }
        Ok(())
    }

    fn handle_batch_clarify_locked(
        &mut self,
        request_id: String,
        clarify_request_id: String,
        question_id: String,
        result: Result<crate::gateway::ClarifyLockReceipt, String>,
    ) {
        let Some(prompt) = self.batch_clarify_prompts.get(&clarify_request_id) else {
            return;
        };
        if prompt.lock_in_flight.as_ref() != Some(&(request_id, question_id.clone()))
            || prompt
                .pending_locks
                .front()
                .map(|lock| lock.question_id.as_str())
                != Some(question_id.as_str())
        {
            return;
        }
        match result {
            Ok(receipt) if receipt.expired => {
                self.batch_clarify_prompts.remove(&clarify_request_id);
                self.status = "Hermes already ended this question".into();
            }
            Ok(_) => {
                if let Some(prompt) = self.batch_clarify_prompts.get_mut(&clarify_request_id) {
                    prompt.pending_locks.pop_front();
                    prompt.lock_in_flight = None;
                    if prompt.pending_locks.is_empty() {
                        self.batch_clarify_prompts.remove(&clarify_request_id);
                        self.status = "Hermes received your answers".into();
                        return;
                    }
                }
                if let Err(error) = self.send_next_batch_clarify_lock(&clarify_request_id) {
                    if let Some(prompt) = self.batch_clarify_prompts.get_mut(&clarify_request_id) {
                        prompt.pending_locks.clear();
                        prompt.lock_in_flight = None;
                        prompt.response_pending = false;
                    }
                    self.status = error;
                }
            }
            Err(error) => {
                if let Some(prompt) = self.batch_clarify_prompts.get_mut(&clarify_request_id) {
                    prompt.pending_locks.clear();
                    prompt.lock_in_flight = None;
                    prompt.response_pending = false;
                }
                self.status = error;
            }
        }
    }

    fn apply_background_chat_event(
        &mut self,
        runtime_session_id: &str,
        event: &serde_json::Value,
        event_type: &str,
        payload: &serde_json::Value,
        execution_generation: Option<u64>,
    ) {
        if !matches!(
            event_type,
            "tool.start" | "tool.complete" | "message.start" | "message.delta" | "message.complete"
        ) {
            return;
        }
        let event_profile = event
            .get("profile")
            .or_else(|| payload.get("profile"))
            .and_then(|value| value.as_str());
        let matches = self
            .session_chat_views
            .iter()
            .filter(|(_, view)| {
                view.runtime_session_id.as_deref() == Some(runtime_session_id)
                    && event_profile.is_none_or(|profile| view.profile.as_deref() == Some(profile))
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        // Runtime IDs may be profile-local on some backends. Fail closed if an
        // event cannot be assigned to exactly one cached profile/session view.
        let [key] = matches.as_slice() else { return };
        self.session_chat_view_order
            .retain(|candidate| candidate != key);
        self.session_chat_view_order.push_back(key.clone());
        let Some(view) = self.session_chat_views.get_mut(key) else {
            return;
        };
        apply_stream_event_to_transcript(
            &mut view.transcript,
            event_type,
            payload,
            execution_generation,
        );
        view.transcript_message_count = view.transcript_message_count.max(view.transcript.len());
    }

    fn handle_server_event(&mut self, event: serde_json::Value) {
        let session_id = event.get("session_id").and_then(|value| value.as_str());
        let event_type = event
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let payload = event
            .get("payload")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let execution_generation = event
            .get("execution_generation")
            .or_else(|| payload.get("execution_generation"))
            .and_then(|value| value.as_u64());
        if event_type == "message.complete"
            && let Some(admission_id) = payload.get("admission_id").and_then(|value| value.as_str())
        {
            if !self
                .recent_completed_admissions
                .iter()
                .any(|id| id == admission_id)
            {
                self.recent_completed_admissions
                    .push_back(admission_id.to_owned());
                if self.recent_completed_admissions.len() > 32 {
                    self.recent_completed_admissions.pop_front();
                }
            }
            let settled_key = self.pending_submissions.iter().find_map(|(key, pending)| {
                (Some(pending.session_id.as_str()) == session_id
                    && pending.admission_id.as_deref() == Some(admission_id))
                .then(|| key.clone())
            });
            if let Some(key) = settled_key {
                self.pending_submissions.remove(&key);
            }
        }
        let Some(active_session) = self.active_runtime_session_id.as_deref() else {
            return;
        };
        if session_id.is_some_and(|session_id| session_id != active_session) {
            self.apply_background_chat_event(
                session_id.unwrap_or_default(),
                &event,
                event_type,
                &payload,
                execution_generation,
            );
            return;
        }
        match event_type {
            "approval.request" => {
                let prompt_id = payload
                    .get("prompt_id")
                    .or_else(|| payload.get("request_id"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty());
                let execution_generation = event
                    .get("execution_generation")
                    .or_else(|| payload.get("execution_generation"))
                    .and_then(|value| value.as_u64());
                let choices = payload
                    .get("choices")
                    .and_then(|value| value.as_array())
                    .map(|choices| {
                        choices
                            .iter()
                            .filter_map(|choice| choice.as_str())
                            .filter(|choice| {
                                matches!(*choice, "once" | "session" | "always" | "deny")
                            })
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if let (Some(prompt_id), Some(execution_generation)) =
                    (prompt_id, execution_generation)
                    && !choices.is_empty()
                {
                    let server_request_id = self
                        .approval_prompts
                        .get(prompt_id)
                        .and_then(|current| current.server_request_id.clone());
                    self.approval_prompts.insert(
                        prompt_id.to_owned(),
                        ApprovalPrompt {
                            session_id: active_session.to_owned(),
                            profile: event
                                .get("profile")
                                .or_else(|| payload.get("profile"))
                                .and_then(|value| value.as_str())
                                .map(str::to_owned)
                                .or_else(|| self.active_session_profile.clone()),
                            execution_generation,
                            prompt_id: prompt_id.to_owned(),
                            server_request_id,
                            command: payload
                                .get("command")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_owned(),
                            description: payload
                                .get("description")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_owned(),
                            choices,
                            response_pending: None,
                        },
                    );
                    self.status = "A tool approval is waiting for your decision".into();
                }
            }
            "approval.settled" => {
                if let Some(prompt_id) = payload
                    .get("prompt_id")
                    .or_else(|| payload.get("request_id"))
                    .and_then(|value| value.as_str())
                {
                    self.approval_prompts.remove(prompt_id);
                }
            }
            "clarify.request" => {
                let prompt_id = payload
                    .get("prompt_id")
                    .or_else(|| payload.get("request_id"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty());
                let execution_generation = event
                    .get("execution_generation")
                    .or_else(|| payload.get("execution_generation"))
                    .and_then(|value| value.as_u64());
                let question = payload
                    .get("question")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty());
                let choices = payload
                    .get("choices")
                    .and_then(|value| value.as_array())
                    .map(|choices| {
                        choices
                            .iter()
                            .filter_map(|choice| choice.as_str())
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if let (Some(prompt_id), Some(execution_generation), Some(question)) =
                    (prompt_id, execution_generation, question)
                {
                    let mut clarify = ClarifyPrompt {
                        session_id: active_session.to_owned(),
                        profile: event
                            .get("profile")
                            .or_else(|| payload.get("profile"))
                            .and_then(|value| value.as_str())
                            .map(str::to_owned)
                            .or_else(|| self.active_session_profile.clone()),
                        execution_generation,
                        prompt_id: prompt_id.to_owned(),
                        question: question.to_owned(),
                        choices,
                        multi_select: payload
                            .get("multi_select")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                        selected: Vec::new(),
                        draft: String::new(),
                        response_pending: None,
                    };
                    if let Some(answer) = payload
                        .get("answers")
                        .and_then(|value| value.get(prompt_id))
                        .and_then(|value| value.as_str())
                    {
                        stage_clarify_answer(&mut clarify, answer);
                    }
                    self.clarify_prompts.insert(prompt_id.to_owned(), clarify);
                    self.status = "Hermes is waiting for your answer".into();
                }
            }
            "clarify.settled" => {
                if let Some(prompt_id) = payload
                    .get("prompt_id")
                    .or_else(|| payload.get("request_id"))
                    .and_then(|value| value.as_str())
                {
                    self.clarify_prompts.remove(prompt_id);
                }
            }
            "tool.start" | "tool.complete" | "message.delta" | "message.complete" => {
                apply_stream_event_to_transcript(
                    &mut self.transcript,
                    event_type,
                    &payload,
                    execution_generation,
                );
                if event_type == "tool.start" {
                    let name = payload
                        .get("name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("tool");
                    self.status = format!("Running {name}…");
                } else if event_type == "message.complete" {
                    self.status = "Hermes response complete".into();
                }
            }
            "message.start" => self.status = "Hermes is thinking…".into(),
            "error" => {
                self.status = payload
                    .get("message")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Hermes returned an error")
                    .to_owned();
            }
            _ => {}
        }
    }

    fn route_unavailable(&self, ui: &mut egui::Ui, message: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.18).max(8.0));
            ui.label(
                RichText::new(self.route.title())
                    .size(22.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(message).size(14.0).color(MUTED));
        });
    }

    fn cache_session_chat_view(&mut self, key: String, view: SessionChatViewState) {
        self.session_chat_view_order
            .retain(|existing| existing != &key);
        self.session_chat_view_order.push_back(key.clone());
        self.session_chat_views.insert(key, view);
        while self.session_chat_views.len() > 32 {
            let active_key = match &self.route {
                AppRoute::Session(session_id) => Some(session_identity_key(
                    session_id,
                    self.active_session_profile.as_deref(),
                )),
                _ => None,
            };
            let candidate = self
                .session_chat_view_order
                .iter()
                .find(|candidate| {
                    if active_key.as_ref() == Some(*candidate) {
                        return false;
                    }
                    let Some(view) = self.session_chat_views.get(*candidate) else {
                        return true;
                    };
                    if view.older_transcript_request.is_some()
                        || view.transcript.iter().any(|message| {
                            message.get("streaming").and_then(|value| value.as_bool()) == Some(true)
                        })
                    {
                        return false;
                    }
                    let Some(runtime) = view.runtime_session_id.as_deref() else {
                        return true;
                    };
                    let runtime_key = session_identity_key(runtime, view.profile.as_deref());
                    !self.pending_submissions.contains_key(&runtime_key)
                        && !self
                            .pending_interrupt
                            .as_ref()
                            .is_some_and(|(_, id)| id == runtime)
                        && !self
                            .approval_prompts
                            .values()
                            .any(|prompt| prompt.session_id == runtime)
                        && !self
                            .clarify_prompts
                            .values()
                            .any(|prompt| prompt.session_id == runtime)
                        && !self
                            .batch_clarify_prompts
                            .values()
                            .any(|prompt| prompt.session_id == runtime)
                })
                .cloned();
            let Some(candidate) = candidate else { break };
            self.session_chat_view_order.retain(|key| key != &candidate);
            self.session_chat_views.remove(&candidate);
        }
    }

    fn save_active_chat_view(&mut self) {
        match &self.route {
            AppRoute::Session(session_id) => {
                let key = session_identity_key(session_id, self.active_session_profile.as_deref());
                self.cache_session_chat_view(
                    key,
                    SessionChatViewState {
                        profile: self.active_session_profile.clone(),
                        runtime_session_id: self.active_runtime_session_id.clone(),
                        transcript: self.transcript.clone(),
                        transcript_message_count: self.transcript_message_count,
                        transcript_has_more: self.transcript_has_more,
                        older_transcript_request: self.older_transcript_request.clone(),
                        older_transcript_error: self.older_transcript_error.clone(),
                        draft: self.draft.clone(),
                    },
                );
            }
            AppRoute::NewChat => self.new_chat_draft.clone_from(&self.draft),
            _ => {}
        }
    }

    fn navigate(&mut self, route: AppRoute) {
        self.save_active_chat_view();
        self.navigate_after_save(route);
    }

    fn navigate_after_save(&mut self, route: AppRoute) {
        let route = AppRoute::parse(&route.path(), &self.contributed_paths);
        if route.is_overlay() && !self.route.is_overlay() {
            self.overlay_return = self.route.clone();
        }
        match &route {
            AppRoute::Session(session_id) => {
                let key = session_identity_key(session_id, self.active_session_profile.as_deref());
                if let Some(view) = self.session_chat_views.get(&key).cloned() {
                    self.transcript = view.transcript;
                    self.transcript_message_count = view.transcript_message_count;
                    self.transcript_has_more = view.transcript_has_more;
                    self.older_transcript_request = view.older_transcript_request;
                    self.older_transcript_error = view.older_transcript_error;
                    self.active_runtime_session_id = view.runtime_session_id;
                    self.draft = view.draft;
                } else {
                    self.transcript.clear();
                    self.transcript_message_count = 0;
                    self.transcript_has_more = false;
                    self.older_transcript_request = None;
                    self.older_transcript_error = None;
                    self.active_runtime_session_id = None;
                    self.draft.clear();
                }
                if self.active_runtime_session_id.is_none()
                    && let Some(gateway) = &self.gateway_discovery
                    && let Err(error) = gateway
                        .resume_session(session_id.clone(), self.active_session_profile.clone())
                {
                    self.status = error;
                }
            }
            AppRoute::NewChat => {
                self.transcript.clear();
                self.transcript_message_count = 0;
                self.transcript_has_more = false;
                self.older_transcript_request = None;
                self.older_transcript_error = None;
                self.active_runtime_session_id = None;
                self.active_session_profile = self
                    .gateway_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.profile_id.clone());
                self.draft.clone_from(&self.new_chat_draft);
            }
            _ => {}
        }
        if route != AppRoute::Capabilities {
            self.capabilities_request_id = None;
            self.capabilities_pending = false;
        }
        self.route = route;
    }

    pub fn navigate_path(&mut self, target: &str) -> Result<(), String> {
        let route = AppRoute::parse(target, &self.contributed_paths);
        if let AppRoute::Session(session_id) = &route {
            // Save before replacing the selected profile so the previous chat
            // remains keyed by its own profile/session identity.
            self.save_active_chat_view();
            let matches = self
                .sessions
                .iter()
                .chain(self.archived_sessions.iter())
                .filter(|session| session.id == *session_id)
                .map(|session| session.profile.clone())
                .collect::<BTreeSet<_>>();
            if matches.len() > 1 {
                return Err(
                    "Session ID matches multiple profiles; use open_session with a profile".into(),
                );
            }
            self.active_session_profile = matches.into_iter().next().flatten().or_else(|| {
                self.gateway_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.profile_id.clone())
            });
            let title = self
                .sessions
                .iter()
                .chain(self.archived_sessions.iter())
                .find(|session| {
                    session.id == *session_id && session.profile == self.active_session_profile
                })
                .map(|session| session.title.as_str())
                .filter(|title| !title.trim().is_empty())
                .unwrap_or(session_id);
            self.workspace.open_session_pane(
                self.active_session_profile.as_deref(),
                session_id,
                title,
            )?;
            self.navigate_after_save(route.clone());
        } else {
            self.workspace.select_panel(&PanelId::from(WORKSPACE))?;
            self.navigate(route.clone());
        }
        let capabilities_tab =
            (route == AppRoute::Capabilities).then(|| CapabilitiesTab::from_query(target));
        if let Some(tab) = capabilities_tab {
            self.capabilities_tab = tab;
            if self.gateway_connected && self.skills.is_none() && !self.capabilities_pending {
                self.load_capabilities()
                    .unwrap_or_else(|error| self.capabilities_error = Some(error));
            }
        }
        Ok(())
    }

    pub fn execute_registered_action(&mut self, action: RegisteredUiAction) -> Result<(), String> {
        match action {
            RegisteredUiAction::Navigate { path } => {
                if path.len() > 2048 {
                    return Err("Navigation path exceeds 2048 bytes".into());
                }
                self.navigate_path(&path)?;
                self.status = format!("Opened {}", self.route.title());
            }
            RegisteredUiAction::OpenSession {
                session_id,
                profile,
            } => {
                if session_id.is_empty()
                    || session_id.len() > 512
                    || session_id.chars().any(char::is_control)
                {
                    return Err("Session ID is invalid".into());
                }
                if profile.as_ref().is_some_and(|profile| {
                    profile.is_empty()
                        || profile.len() > 128
                        || profile.chars().any(char::is_control)
                }) {
                    return Err("Session profile is invalid".into());
                }
                let title = self
                    .sessions
                    .iter()
                    .chain(self.archived_sessions.iter())
                    .find(|session| session.id == session_id && session.profile == profile)
                    .map(|session| session.title.clone())
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| session_id.clone());
                self.save_active_chat_view();
                self.workspace
                    .open_session_pane(profile.as_deref(), &session_id, &title)?;
                self.active_session_profile = profile;
                self.navigate_after_save(AppRoute::Session(session_id));
                self.status = "Opened session tab".into();
            }
            RegisteredUiAction::TogglePane { panel } => {
                if !matches!(panel.as_str(), REVIEW | FILES | TERMINAL) {
                    return Err(format!(
                        "Pane {panel:?} is not registered for toggle actions"
                    ));
                }
                self.workspace.toggle_pane(&panel)?;
                self.status = format!(
                    "{} {}",
                    panel_title(&panel),
                    if self.workspace.pane_is_open(&panel) {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
            }
            RegisteredUiAction::SetInterfaceMode { mode } => {
                self.workspace.set_mode(mode)?;
                self.status = format!(
                    "{} interface",
                    match mode {
                        InterfaceMode::Advanced => "Advanced",
                        InterfaceMode::Simple => "Simple",
                    }
                );
            }
            RegisteredUiAction::SelectLayoutProfile { profile_id } => {
                self.workspace.set_profile(&profile_id)?;
                self.status = format!("{} layout", profile_title(&profile_id));
            }
            RegisteredUiAction::SelectCapabilitiesTab { tab } => {
                self.navigate_path(&format!("/capabilities?tab={}", tab.as_str()))?;
            }
            RegisteredUiAction::RefreshCapabilities => self.load_capabilities()?,
            RegisteredUiAction::SetToolsetEnabled { name, enabled } => {
                self.set_toolset_enabled(name, enabled)?;
            }
            RegisteredUiAction::StopGeneration => self.stop_generation()?,
            RegisteredUiAction::RespondApproval { prompt_id, choice } => {
                self.respond_to_approval(prompt_id, choice)?;
            }
            RegisteredUiAction::RespondClarify { prompt_id, answer } => {
                self.respond_to_clarify(prompt_id, answer)?;
            }
            RegisteredUiAction::CloseActiveTab => self.close_active_tab()?,
        }
        Ok(())
    }

    fn load_capabilities(&mut self) -> Result<(), String> {
        if !self.gateway_connected {
            return Err("Connect to Hermes before loading Skills and Toolsets".into());
        }
        let request_id = operation_id("capabilities");
        let gateway = self
            .gateway_discovery
            .as_ref()
            .ok_or_else(|| "Hermes gateway reader is unavailable".to_owned())?;
        gateway.load_capabilities(request_id.clone())?;
        self.capabilities_request_id = Some(request_id);
        self.capabilities_pending = true;
        self.capabilities_error = None;
        Ok(())
    }

    fn set_toolset_enabled(&mut self, name: String, enabled: bool) -> Result<(), String> {
        if !self.gateway_connected {
            return Err("Connect to Hermes before changing a Toolset".into());
        }
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err("Toolset name is invalid".into());
        }
        let Some(toolsets) = &self.toolsets else {
            return Err("Load Toolsets before changing one".into());
        };
        let Some(toolset) = toolsets.iter().find(|toolset| toolset.name == name) else {
            return Err(format!("Toolset {name:?} is not in the loaded inventory"));
        };
        if self.pending_toolset_updates.contains_key(&name) {
            return Err(format!(
                "Toolset {name:?} already has an update in progress"
            ));
        }
        if toolset.enabled == enabled {
            return Ok(());
        }
        let previous_enabled = toolset.enabled;
        let label = if toolset.label.is_empty() {
            toolset.name.clone()
        } else {
            toolset.label.clone()
        };
        let request_id = operation_id("toolset-enabled");
        let gateway = self
            .gateway_discovery
            .as_ref()
            .ok_or_else(|| "Hermes gateway writer is unavailable".to_owned())?;
        gateway.set_toolset_enabled(request_id.clone(), name.clone(), enabled)?;
        if let Some(toolset) = self
            .toolsets
            .as_mut()
            .and_then(|toolsets| toolsets.iter_mut().find(|toolset| toolset.name == name))
        {
            toolset.enabled = enabled;
        }
        self.pending_toolset_updates.insert(
            name.clone(),
            PendingToolsetUpdate {
                request_id,
                previous_enabled,
            },
        );
        self.status = format!(
            "{} {label}…",
            if enabled { "Enabling" } else { "Disabling" }
        );
        Ok(())
    }

    fn stop_generation(&mut self) -> Result<(), String> {
        if !self.gateway_connected {
            return Err("Connect to Hermes before stopping a response".into());
        }
        let session_id = self
            .active_runtime_session_id
            .clone()
            .ok_or_else(|| "There is no active Hermes session to stop".to_owned())?;
        let pending_key = session_identity_key(&session_id, self.active_session_profile.as_deref());
        let Some(pending) = self.pending_submissions.get(&pending_key) else {
            return Err("There is no active response to stop".into());
        };
        if pending.uncertain {
            return Err("Confirm the pending message admission before stopping a response".into());
        }
        if self
            .pending_interrupt
            .as_ref()
            .is_some_and(|(_, pending_session)| pending_session == &session_id)
        {
            return Err("A stop request is already in progress".into());
        }
        let request_id = operation_id("interrupt");
        self.gateway_discovery
            .as_ref()
            .ok_or_else(|| "Hermes gateway writer is unavailable".to_owned())?
            .interrupt_session(
                request_id.clone(),
                session_id.clone(),
                self.active_session_profile.clone(),
            )?;
        self.pending_interrupt = Some((request_id, session_id));
        self.status = "Stopping Hermes response…".into();
        Ok(())
    }

    fn respond_to_approval(&mut self, prompt_id: String, choice: String) -> Result<(), String> {
        if !self.gateway_connected {
            return Err("Connect to Hermes before responding to an approval".into());
        }
        let Some(prompt) = self.approval_prompts.get(&prompt_id) else {
            return Err("This approval is no longer pending".into());
        };
        if !self.session_runtime_is_known(&prompt.session_id, prompt.profile.as_deref()) {
            return Err("This approval no longer maps to a unique open session pane".into());
        }
        if !matches!(choice.as_str(), "once" | "session" | "always" | "deny")
            || !prompt.choices.iter().any(|allowed| allowed == &choice)
        {
            return Err("That decision is not available for this approval".into());
        }
        if prompt.response_pending.is_some() {
            return Err("A decision is already being sent for this approval".into());
        }
        let session_id = prompt.session_id.clone();
        let generation = prompt.execution_generation;
        let profile = prompt.profile.clone();
        let server_request_id = prompt.server_request_id.clone();
        let gateway = self
            .gateway_discovery
            .as_ref()
            .ok_or_else(|| "Hermes gateway writer is unavailable".to_owned())?;
        if let Some(server_request_id) = server_request_id {
            gateway.respond_server_request(
                server_request_id.clone(),
                Ok(serde_json::json!({"choice":choice})),
            )?;
            if let Some(prompt) = self.approval_prompts.get_mut(&prompt_id) {
                prompt.response_pending = Some((server_request_id, choice));
            }
            self.status = "Sending approval decision to Hermes…".into();
            return Ok(());
        }
        let request_id = operation_id("approval");
        gateway.respond_approval(
            request_id.clone(),
            session_id,
            profile,
            generation,
            prompt_id.clone(),
            choice.clone(),
        )?;
        if let Some(prompt) = self.approval_prompts.get_mut(&prompt_id) {
            prompt.response_pending = Some((request_id, choice));
        }
        self.status = "Sending approval decision to Hermes…".into();
        Ok(())
    }

    fn respond_to_clarify(&mut self, prompt_id: String, answer: String) -> Result<(), String> {
        if !self.gateway_connected {
            return Err("Connect to Hermes before answering a question".into());
        }
        if let Some(prompt) = self.batch_clarify_prompts.get(&prompt_id) {
            if !self.session_runtime_is_known(&prompt.session_id, prompt.profile.as_deref()) {
                return Err("These questions no longer map to a unique open session pane".into());
            }
            if prompt.response_pending {
                return Err("Answers are already being sent for these questions".into());
            }
            let server_request_id = prompt.server_request_id.clone();
            if answer.is_empty() {
                self.gateway_discovery
                    .as_ref()
                    .ok_or_else(|| "Hermes gateway writer is unavailable".to_owned())?
                    .respond_server_request(server_request_id, Ok(serde_json::json!({})))?;
                if let Some(prompt) = self.batch_clarify_prompts.get_mut(&prompt_id) {
                    prompt.response_pending = true;
                }
                self.status = "Skipping Hermes questions…".into();
                return Ok(());
            }

            let value: serde_json::Value = serde_json::from_str(&answer)
                .map_err(|_| "Clarify answers are not valid JSON".to_owned())?;
            let answers = value
                .get("answers")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| "Clarify answers must contain an answers object".to_owned())?;
            let expected = prompt
                .questions
                .iter()
                .map(|question| question.qid.as_str())
                .collect::<BTreeSet<_>>();
            if answers.keys().any(|qid| !expected.contains(qid.as_str()))
                || expected.iter().any(|qid| !answers.contains_key(*qid))
                || answers.values().any(|value| match value {
                    serde_json::Value::Null => false,
                    serde_json::Value::String(value) => {
                        value.len() > 16_384 || value.chars().any(|character| character == '\0')
                    }
                    _ => true,
                })
                || answers.values().all(serde_json::Value::is_null)
            {
                return Err("Clarify answers do not match the pending questions".into());
            }
            let pending_locks = prompt
                .questions
                .iter()
                .map(|question| PendingClarifyLock {
                    question_id: question.qid.clone(),
                    answer: answers
                        .get(&question.qid)
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                })
                .collect();
            if let Some(prompt) = self.batch_clarify_prompts.get_mut(&prompt_id) {
                prompt.response_pending = true;
                prompt.pending_locks = pending_locks;
            }
            if let Err(error) = self.send_next_batch_clarify_lock(&prompt_id) {
                if let Some(prompt) = self.batch_clarify_prompts.get_mut(&prompt_id) {
                    prompt.response_pending = false;
                    prompt.pending_locks.clear();
                    prompt.lock_in_flight = None;
                }
                return Err(error);
            }
            self.status = "Sending answers to Hermes…".into();
            return Ok(());
        }
        if answer.len() > 16_384 || answer.chars().any(|character| character == '\0') {
            return Err("Clarify answer must be at most 16384 bytes and contain no NUL".into());
        }
        let Some(prompt) = self.clarify_prompts.get(&prompt_id) else {
            return Err("This question is no longer pending".into());
        };
        if !self.session_runtime_is_known(&prompt.session_id, prompt.profile.as_deref()) {
            return Err("This question no longer maps to a unique open session pane".into());
        }
        if prompt.response_pending.is_some() {
            return Err("An answer is already being sent for this question".into());
        }
        let session_id = prompt.session_id.clone();
        let generation = prompt.execution_generation;
        let profile = prompt.profile.clone();
        let request_id = operation_id("clarify");
        self.gateway_discovery
            .as_ref()
            .ok_or_else(|| "Hermes gateway writer is unavailable".to_owned())?
            .respond_clarify(
                request_id.clone(),
                session_id,
                profile,
                generation,
                prompt_id.clone(),
                answer.clone(),
            )?;
        if let Some(prompt) = self.clarify_prompts.get_mut(&prompt_id) {
            prompt.response_pending = Some((request_id, answer));
        }
        self.status = if self.clarify_prompts.get(&prompt_id).is_some_and(|prompt| {
            prompt
                .response_pending
                .as_ref()
                .is_some_and(|(_, value)| value.is_empty())
        }) {
            "Skipping Hermes question…".into()
        } else {
            "Sending answer to Hermes…".into()
        };
        Ok(())
    }

    fn handle_clarify_responded(
        &mut self,
        request_id: String,
        session_id: String,
        prompt_id: String,
        answer: String,
        result: Result<(), String>,
    ) {
        let Some(prompt) = self.clarify_prompts.get(&prompt_id) else {
            return;
        };
        if prompt.session_id != session_id
            || prompt.response_pending.as_ref() != Some(&(request_id.clone(), answer.clone()))
        {
            return;
        }
        match result {
            Ok(()) => {
                self.clarify_prompts.remove(&prompt_id);
                self.status = if answer.is_empty() {
                    "Hermes question skipped".into()
                } else {
                    "Hermes received your answer".into()
                };
            }
            Err(error) => {
                if let Some(prompt) = self.clarify_prompts.get_mut(&prompt_id) {
                    prompt.response_pending = None;
                }
                self.status = error;
            }
        }
    }

    fn handle_approval_responded(
        &mut self,
        request_id: String,
        session_id: String,
        prompt_id: String,
        choice: String,
        result: Result<(), String>,
    ) {
        let Some(prompt) = self.approval_prompts.get(&prompt_id) else {
            return;
        };
        if prompt.session_id != session_id
            || prompt.response_pending.as_ref() != Some(&(request_id.clone(), choice.clone()))
        {
            return;
        }
        match result {
            Ok(()) => {
                self.approval_prompts.remove(&prompt_id);
                self.status = "Hermes received the approval decision".into();
            }
            Err(error) => {
                if let Some(prompt) = self.approval_prompts.get_mut(&prompt_id) {
                    prompt.response_pending = None;
                }
                self.status = error;
            }
        }
    }

    fn handle_session_interrupted(
        &mut self,
        request_id: String,
        session_id: String,
        result: Result<bool, String>,
    ) {
        if !self
            .pending_interrupt
            .as_ref()
            .is_some_and(|(pending_request, pending_session)| {
                pending_request == &request_id && pending_session == &session_id
            })
        {
            return;
        }
        self.pending_interrupt = None;
        match result {
            Ok(true) => {
                if let Some(message) = self.transcript.iter_mut().rev().find(|message| {
                    message.get("role").and_then(|value| value.as_str()) == Some("assistant")
                        && message.get("streaming").and_then(|value| value.as_bool()) == Some(true)
                }) {
                    message["streaming"] = serde_json::Value::Bool(false);
                    message["interrupted"] = serde_json::Value::Bool(true);
                }
                self.pending_submissions
                    .retain(|_, pending| pending.session_id != session_id);
                self.status = "Hermes response stopped".into();
            }
            Ok(false) => {
                self.pending_submissions
                    .retain(|_, pending| pending.session_id != session_id);
                self.status = "Hermes reported no active response".into();
            }
            Err(error) => self.status = error,
        }
    }

    #[cfg(feature = "agent-automation")]
    fn current_route_path(&self) -> String {
        if self.route == AppRoute::Capabilities {
            format!("/capabilities?tab={}", self.capabilities_tab.as_str())
        } else {
            self.route.path()
        }
    }

    fn show_route_overlay(&mut self, ctx: &egui::Context) {
        if !self.route.is_overlay() {
            return;
        }
        let mut close = false;
        egui::Modal::new(Id::unique("hermes-route-overlay")).show(ctx, |ui| {
            ui.set_min_width(420.0);
            ui.set_max_width(640.0);
            ui.horizontal(|ui| {
                ui.heading(self.route.title());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui.button("Close").clicked();
                });
            });
            ui.separator();
            ui.label(RichText::new("Connect to Hermes to load this screen.").color(MUTED));
        });
        if close {
            self.execute_registered_action(RegisteredUiAction::Navigate {
                path: self.overlay_return.path(),
            })
            .unwrap_or_else(|error| self.status = error);
        }
    }

    fn show_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(session) = self.rename_session.clone() else {
            return;
        };
        let action_key = session_identity_key(&session.id, session.profile.as_deref());
        let pending = self.pending_session_renames.contains_key(&action_key);
        let uncertain = self.uncertain_session_renames.contains(&action_key);
        let mut save = false;
        let mut retry = false;
        let mut cancel = false;
        egui::Window::new("Rename session")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Session {}", session.id));
                ui.add_enabled_ui(!pending, |ui| {
                    ui.text_edit_singleline(&mut self.rename_draft)
                        .accessible_name("Session title");
                });
                ui.horizontal(|ui| {
                    if uncertain {
                        retry = ui
                            .button("Retry same request")
                            .accessible_name("Retry session rename with original request")
                            .clicked();
                    }
                    save = ui
                        .add_enabled(!pending, egui::Button::new("Save"))
                        .accessible_name("Save session title")
                        .clicked();
                    cancel = ui
                        .add_enabled(!pending, egui::Button::new("Cancel"))
                        .accessible_name("Cancel session rename")
                        .clicked();
                });
            });
        if save {
            self.start_session_rename_update(session);
        } else if retry {
            self.retry_uncertain_session_rename(session);
        } else if cancel {
            self.rename_session = None;
        }
    }

    fn empty_panel(&self, ui: &mut egui::Ui, message: &str) {
        ui.add_space(8.0);
        ui.label(RichText::new(message).size(12.0).color(MUTED));
    }

    fn session_pane_content(&mut self, ui: &mut Ui, descriptor: &doofusdock_core::PanelDescriptor) {
        let (Some(session_id), Some(profile_value)) = (
            descriptor.metadata.get("hermes.session.id"),
            descriptor.metadata.get("hermes.session.profile"),
        ) else {
            self.empty_panel(ui, "This session pane has incomplete identity metadata");
            return;
        };
        let session_id = session_id.clone();
        let profile = (!profile_value.is_empty()).then(|| profile_value.clone());
        let key = session_identity_key(&session_id, profile.as_deref());

        self.save_active_chat_view();
        let previous_route = self.route.clone();
        let previous_profile = self.active_session_profile.clone();
        let previous_runtime = self.active_runtime_session_id.clone();
        let previous_transcript = std::mem::take(&mut self.transcript);
        let previous_message_count = self.transcript_message_count;
        let previous_has_more = self.transcript_has_more;
        let previous_older_request = self.older_transcript_request.take();
        let previous_older_error = self.older_transcript_error.take();
        let previous_draft = std::mem::take(&mut self.draft);

        let view = self
            .session_chat_views
            .get(&key)
            .cloned()
            .unwrap_or_else(|| SessionChatViewState {
                profile: profile.clone(),
                ..SessionChatViewState::default()
            });
        self.route = AppRoute::Session(session_id.clone());
        self.active_session_profile = profile;
        self.active_runtime_session_id = view.runtime_session_id;
        self.transcript = view.transcript;
        self.transcript_message_count = view.transcript_message_count;
        self.transcript_has_more = view.transcript_has_more;
        self.older_transcript_request = view.older_transcript_request;
        self.older_transcript_error = view.older_transcript_error;
        self.draft = view.draft;

        self.session_content(ui, &session_id);
        self.approval_cards(ui);
        self.clarify_cards(ui);
        self.batch_clarify_cards(ui);
        self.composer(ui);
        self.save_active_chat_view();

        self.route = previous_route;
        self.active_session_profile = previous_profile;
        self.active_runtime_session_id = previous_runtime;
        self.transcript = previous_transcript;
        self.transcript_message_count = previous_message_count;
        self.transcript_has_more = previous_has_more;
        self.older_transcript_request = previous_older_request;
        self.older_transcript_error = previous_older_error;
        self.draft = previous_draft;
    }

    fn session_pane_is_busy(&self, panel: &PanelId) -> bool {
        let document = self.workspace.dock.snapshot().document;
        let Some(descriptor) = document.descriptor(panel) else {
            return false;
        };
        let (Some(session_id), Some(profile_value)) = (
            descriptor.metadata.get("hermes.session.id"),
            descriptor.metadata.get("hermes.session.profile"),
        ) else {
            return false;
        };
        let profile = (!profile_value.is_empty()).then_some(profile_value.as_str());
        let key = session_identity_key(session_id, profile);
        let runtime = self
            .session_chat_views
            .get(&key)
            .and_then(|view| view.runtime_session_id.clone())
            .or_else(|| {
                (self.route == AppRoute::Session(session_id.clone())
                    && self.active_session_profile.as_deref() == profile)
                    .then(|| self.active_runtime_session_id.clone())
                    .flatten()
            });
        let Some(runtime) = runtime else {
            return false;
        };
        let runtime_key = session_identity_key(&runtime, profile);
        self.pending_submissions.contains_key(&runtime_key)
            || self
                .pending_interrupt
                .as_ref()
                .is_some_and(|(_, session)| session == &runtime)
            || self
                .approval_prompts
                .values()
                .any(|prompt| prompt.session_id == runtime)
            || self
                .clarify_prompts
                .values()
                .any(|prompt| prompt.session_id == runtime)
            || self
                .batch_clarify_prompts
                .values()
                .any(|prompt| prompt.session_id == runtime)
    }

    fn close_session_pane(&mut self, panel: &PanelId) {
        match self.workspace.close_session_pane(panel) {
            Ok(()) => {
                self.pending_session_close = None;
                self.status = "Session tab closed · Hermes continues in the background".into();
            }
            Err(error) => {
                self.pending_session_close = None;
                self.status = error;
            }
        }
    }

    fn close_active_tab(&mut self) -> Result<(), String> {
        let snapshot = self.workspace.dock.snapshot();
        let groups = collect_dock_tab_groups(snapshot.document.root.as_deref());
        let workspace_group = groups
            .iter()
            .find(|group| group.panels.contains(&PanelId::from(WORKSPACE)));
        let active_panel = workspace_group
            .and_then(|group| group.panels.get(group.active_index))
            .cloned()
            .unwrap_or_else(|| PanelId::from(WORKSPACE));

        if active_panel != PanelId::from(WORKSPACE) {
            if snapshot
                .document
                .descriptor(&active_panel)
                .is_some_and(|descriptor| descriptor.role == "hermes_session_tile")
            {
                self.request_close_session_pane(&active_panel);
            }
            return Ok(());
        }

        let AppRoute::Session(current_session_id) = self.route.clone() else {
            return Ok(());
        };
        if let Some((panel, session_id, profile)) =
            next_session_pane_for_workspace(&snapshot.document)
        {
            self.save_active_chat_view();
            self.workspace.close_session_pane(&panel)?;
            self.workspace.select_panel(&PanelId::from(WORKSPACE))?;
            self.active_session_profile = profile;
            self.navigate_after_save(AppRoute::Session(session_id));
            self.status = format!("Promoted the next session tab from {current_session_id}");
            return Ok(());
        }

        self.new_chat_draft.clear();
        self.navigate(AppRoute::NewChat);
        self.status = "Closed the workspace chat · New session".into();
        Ok(())
    }

    fn request_close_session_pane(&mut self, panel: &PanelId) {
        if self.session_pane_is_busy(panel) {
            self.pending_session_close = Some(panel.clone());
        } else {
            self.close_session_pane(panel);
        }
    }

    fn show_session_close_dialog(&mut self, ctx: &egui::Context) {
        let Some(panel) = self.pending_session_close.clone() else {
            return;
        };
        let title = self
            .workspace
            .dock
            .snapshot()
            .document
            .descriptor(&panel)
            .map(|descriptor| descriptor.title_key.clone())
            .unwrap_or_else(|| "session".into());
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Close active session tab?")
            .id(Id::NULL.with(("close-session-tab", &panel.0)))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("{title} is still active."));
                ui.label("Closing this tab will not stop Hermes. The session can continue in the background.");
                ui.horizontal(|ui| {
                    confirm = ui.button("Close tab").clicked();
                    cancel = ui.button("Keep open").clicked();
                });
            });
        if confirm {
            self.close_session_pane(&panel);
        } else if cancel {
            self.pending_session_close = None;
        }
    }

    fn update_gateway_discovery(&mut self, ctx: &egui::Context) {
        let Some(discovery) = &self.gateway_discovery else {
            return;
        };
        let mut load_capabilities_after_connect = false;
        let mut load_archived_after_connect = false;
        if !self.gateway_connected {
            if let Some(result) = discovery.poll() {
                match result {
                    Ok(connection) => {
                        self.gateway_connected = true;
                        self.status =
                            format!("Gateway connected · {} sessions", connection.sessions.len());
                        self.gateway_endpoint = Some(connection.endpoint);
                        if self.route == AppRoute::NewChat {
                            self.active_session_profile = self
                                .gateway_endpoint
                                .as_ref()
                                .map(|endpoint| endpoint.profile_id.clone());
                        }
                        self.sessions = connection.sessions;
                        if self.show_archived_sessions {
                            load_archived_after_connect = true;
                        }
                        if self.route == AppRoute::Capabilities {
                            load_capabilities_after_connect = true;
                        }
                    }
                    Err(reason) => {
                        self.gateway_connected = false;
                        self.status = format!("Backend unavailable: {reason}");
                    }
                }
            }
        }
        let mut events = Vec::new();
        while let Some(event) = discovery.poll_event() {
            events.push(event);
        }
        for event in events {
            match event {
                GatewayEvent::SessionCreated(result) => match result {
                    Ok(created) => {
                        let Some((request_id, _)) = self.pending_create.as_ref() else {
                            continue;
                        };
                        if request_id != &created.request_id {
                            self.status = "Hermes returned an unmatched session creation".into();
                            continue;
                        }
                        let (_, text) = self.pending_create.take().expect("pending create checked");
                        let profile = self
                            .gateway_endpoint
                            .as_ref()
                            .map(|endpoint| endpoint.profile_id.clone());
                        let pane_result = self.workspace.open_session_pane(
                            profile.as_deref(),
                            &created.stored_session_id,
                            if text.is_empty() {
                                "New session"
                            } else {
                                &text
                            },
                        );
                        if !text.is_empty() {
                            self.sessions.retain(|session| {
                                session.id != created.stored_session_id
                                    || session.profile != profile
                            });
                            self.sessions.insert(
                                0,
                                SessionSummary {
                                    id: created.stored_session_id.clone(),
                                    profile: profile.clone(),
                                    title: text.chars().take(72).collect(),
                                    preview: text.chars().take(120).collect(),
                                    message_count: 0,
                                    source: "desktop".into(),
                                    started_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map_or(0.0, |duration| duration.as_secs_f64()),
                                    last_active: None,
                                    pinned: false,
                                    unread: false,
                                    archived: false,
                                },
                            );
                        }
                        self.route = AppRoute::Session(created.stored_session_id.clone());
                        self.transcript.clear();
                        self.transcript_message_count = 0;
                        self.transcript_has_more = false;
                        self.older_transcript_request = None;
                        self.older_transcript_error = None;
                        self.active_runtime_session_id = Some(created.runtime_session_id.clone());
                        self.active_session_profile = self
                            .gateway_endpoint
                            .as_ref()
                            .map(|endpoint| endpoint.profile_id.clone());
                        let blank_session = text.is_empty();
                        if blank_session {
                            self.untouched_blank_session = Some((
                                created.stored_session_id.clone(),
                                self.active_session_profile.clone(),
                            ));
                        }
                        self.draft = text;
                        if blank_session {
                            self.status = "New Hermes session tab created".into();
                        } else {
                            self.submit_prompt(created.runtime_session_id);
                        }
                        if let Err(error) = pane_result {
                            self.status = format!(
                                "Session created, but its Dock tab could not be opened: {error}"
                            );
                        }
                    }
                    Err(error) => {
                        self.pending_create = None;
                        self.status = error;
                    }
                },
                GatewayEvent::SessionResumed(Ok(snapshot)) => {
                    let key =
                        session_identity_key(&snapshot.session_id, snapshot.profile.as_deref());
                    let mut view = self
                        .session_chat_views
                        .get(&key)
                        .cloned()
                        .unwrap_or_default();
                    view.profile = snapshot.profile.clone();
                    view.transcript = snapshot.messages;
                    view.transcript_message_count =
                        snapshot.message_count.max(view.transcript.len());
                    view.transcript_has_more =
                        view.transcript.len() < view.transcript_message_count;
                    view.older_transcript_request = None;
                    view.older_transcript_error = None;
                    view.runtime_session_id = Some(snapshot.runtime_session_id.clone());
                    self.cache_session_chat_view(key, view.clone());
                    if self.route == AppRoute::Session(snapshot.session_id.clone())
                        && self.active_session_profile == snapshot.profile
                    {
                        self.transcript = view.transcript.clone();
                        self.transcript_message_count = view.transcript_message_count;
                        self.transcript_has_more = view.transcript_has_more;
                        self.older_transcript_request = None;
                        self.older_transcript_error = None;
                        self.active_runtime_session_id = Some(snapshot.runtime_session_id);
                        self.status = format!("Session open · {} messages", snapshot.message_count);
                    }
                }
                GatewayEvent::SessionResumed(Err(error)) => self.status = error,
                GatewayEvent::OlderTranscriptLoaded {
                    request_id,
                    session_id,
                    profile,
                    offset,
                    result,
                } => {
                    self.save_active_chat_view();
                    let key = session_identity_key(&session_id, profile.as_deref());
                    let Some(view) = self.session_chat_views.get_mut(&key) else {
                        continue;
                    };
                    let expected = view.older_transcript_request.as_ref().is_some_and(
                        |(pending_id, pending_session, pending_profile, pending_offset)| {
                            pending_id == &request_id
                                && pending_session == &session_id
                                && pending_profile == &profile
                                && pending_offset == &offset
                        },
                    );
                    if expected {
                        view.older_transcript_request = None;
                        match result {
                            Ok(page) => {
                                if page.replace_all {
                                    view.transcript = page.messages;
                                    view.transcript_has_more = false;
                                } else {
                                    view.transcript = merge_older_transcript_page(
                                        page.messages,
                                        std::mem::take(&mut view.transcript),
                                    );
                                    view.transcript_has_more = page.has_more;
                                }
                                view.transcript_message_count =
                                    view.transcript_message_count.max(view.transcript.len());
                                view.older_transcript_error = None;
                                self.status = format!(
                                    "Loaded earlier messages · {} visible",
                                    view.transcript.len()
                                );
                            }
                            Err(error) => {
                                view.older_transcript_error = Some(error.clone());
                                self.status = format!("Could not load earlier messages: {error}");
                            }
                        }
                        if self.route == AppRoute::Session(session_id.clone())
                            && self.active_session_profile == profile
                        {
                            self.transcript = view.transcript.clone();
                            self.transcript_message_count = view.transcript_message_count;
                            self.transcript_has_more = view.transcript_has_more;
                            self.older_transcript_request = None;
                            self.older_transcript_error = view.older_transcript_error.clone();
                        }
                    }
                }
                GatewayEvent::PromptSubmitted {
                    session_id,
                    submission_id,
                    result,
                } => self.handle_prompt_submitted(session_id, submission_id, result),
                GatewayEvent::UnknownPromptResolved {
                    session_id,
                    admission_id,
                    result,
                } => {
                    let key = self.pending_submissions.iter().find_map(|(key, pending)| {
                        (pending.session_id == session_id
                            && pending.admission_id.as_deref() == Some(admission_id.as_str()))
                        .then(|| key.clone())
                    });
                    if let Some(key) = key {
                        match result {
                            Ok(status) if status == "terminal" => {
                                self.pending_submissions.remove(&key);
                                self.status = "Unknown Hermes message settled".into();
                            }
                            Ok(status) => {
                                if let Some(pending) = self.pending_submissions.get_mut(&key) {
                                    pending.resolving_unknown = false;
                                }
                                self.status = format!("Hermes admission remains {status}");
                            }
                            Err(error) => {
                                if let Some(pending) = self.pending_submissions.get_mut(&key) {
                                    pending.resolving_unknown = false;
                                }
                                self.status = error;
                            }
                        }
                    }
                }
                GatewayEvent::SessionInterrupted {
                    request_id,
                    session_id,
                    result,
                } => self.handle_session_interrupted(request_id, session_id, result),
                GatewayEvent::ApprovalResponded {
                    request_id,
                    session_id,
                    prompt_id,
                    choice,
                    result,
                } => self
                    .handle_approval_responded(request_id, session_id, prompt_id, choice, result),
                GatewayEvent::ClarifyResponded {
                    request_id,
                    session_id,
                    prompt_id,
                    answer,
                    result,
                } => {
                    self.handle_clarify_responded(request_id, session_id, prompt_id, answer, result)
                }
                GatewayEvent::ClarifyLocked {
                    request_id,
                    clarify_request_id,
                    question_id,
                    result,
                } => self.handle_batch_clarify_locked(
                    request_id,
                    clarify_request_id,
                    question_id,
                    result,
                ),
                GatewayEvent::ServerRequestReceived { id, method, params } => {
                    self.handle_server_request(id, method, params)
                }
                GatewayEvent::ServerRequestAnswered { id, result } => {
                    self.handle_server_request_answered(id, result)
                }
                GatewayEvent::ServerEvent(event) => self.handle_server_event(event),
                GatewayEvent::SessionsSearched {
                    request_id,
                    query,
                    result,
                } => {
                    if self.session_search_request_id.as_deref() != Some(&request_id)
                        || self.session_filter.trim() != query
                    {
                        continue;
                    }
                    self.session_search_pending = false;
                    match result {
                        Ok(results) => {
                            self.session_search_results = results;
                            self.session_search_error = None;
                        }
                        Err(error) => {
                            self.session_search_results.clear();
                            self.session_search_error = Some(error);
                        }
                    }
                }
                GatewayEvent::SessionsLoaded(Ok(loaded)) => {
                    self.sessions =
                        merge_sessions_by_profile(loaded, std::mem::take(&mut self.sessions));
                    self.session_list_error = None;
                }
                GatewayEvent::SessionsLoaded(Err(error)) => {
                    self.session_list_error = Some(error);
                }
                GatewayEvent::ArchivedSessionsLoaded { request_id, result } => {
                    if self.archived_sessions_request_id.as_deref() != Some(&request_id) {
                        continue;
                    }
                    self.archived_sessions_request_id = None;
                    self.archived_sessions_loading = false;
                    match result {
                        Ok(sessions) => {
                            self.archived_sessions = sessions;
                            self.archived_sessions_error = None;
                        }
                        Err(error) => self.archived_sessions_error = Some(error),
                    }
                }
                GatewayEvent::SessionPinUpdated {
                    request_id,
                    session_id,
                    profile,
                    pinned,
                    result,
                } => {
                    let action_key = session_identity_key(&session_id, profile.as_deref());
                    if self.pending_session_pins.get(&action_key) != Some(&request_id) {
                        continue;
                    }
                    self.pending_session_pins.remove(&action_key);
                    match result {
                        Ok(()) => {
                            if let Some(session) = self
                                .sessions
                                .iter_mut()
                                .chain(self.archived_sessions.iter_mut())
                                .find(|session| {
                                    session.id == session_id && session.profile == profile
                                })
                            {
                                session.pinned = pinned;
                            }
                            self.status = if pinned {
                                "Session pinned".into()
                            } else {
                                "Session unpinned".into()
                            };
                        }
                        Err(error) => self.status = error,
                    }
                }
                GatewayEvent::SessionUnreadUpdated {
                    request_id,
                    session_id,
                    profile,
                    unread,
                    result,
                } => {
                    let action_key = session_identity_key(&session_id, profile.as_deref());
                    if self.pending_session_unreads.get(&action_key) != Some(&request_id) {
                        continue;
                    }
                    self.pending_session_unreads.remove(&action_key);
                    match result {
                        Ok(()) => {
                            if let Some(session) = self
                                .sessions
                                .iter_mut()
                                .chain(self.archived_sessions.iter_mut())
                                .find(|session| {
                                    session.id == session_id && session.profile == profile
                                })
                            {
                                session.unread = unread;
                            }
                            self.status = if unread {
                                "Session marked unread".into()
                            } else {
                                "Session marked read".into()
                            };
                        }
                        Err(error) => self.status = error,
                    }
                }
                GatewayEvent::SessionArchiveUpdated {
                    request_id,
                    session_id,
                    profile,
                    archived,
                    result,
                } => {
                    let action_key = session_identity_key(&session_id, profile.as_deref());
                    if self.pending_session_archives.get(&action_key) != Some(&request_id) {
                        continue;
                    }
                    self.pending_session_archives.remove(&action_key);
                    match result {
                        Ok(()) => {
                            if archived {
                                if let Some(mut session) = self
                                    .sessions
                                    .iter()
                                    .find(|session| {
                                        session.id == session_id && session.profile == profile
                                    })
                                    .cloned()
                                {
                                    session.archived = true;
                                    self.archived_sessions.retain(|row| {
                                        row.id != session_id || row.profile != profile
                                    });
                                    self.archived_sessions.insert(0, session);
                                }
                                self.sessions.retain(|session| {
                                    session.id != session_id || session.profile != profile
                                });
                                self.session_search_results.retain(|result| {
                                    result.session_id != session_id || result.profile != profile
                                });
                            } else if let Some(session) =
                                self.archived_sessions.iter_mut().find(|session| {
                                    session.id == session_id && session.profile == profile
                                })
                            {
                                session.archived = false;
                                let session = session.clone();
                                self.archived_sessions
                                    .retain(|row| row.id != session_id || row.profile != profile);
                                if !self
                                    .sessions
                                    .iter()
                                    .any(|row| row.id == session_id && row.profile == profile)
                                {
                                    self.sessions.insert(0, session);
                                }
                            }
                            self.status = if archived {
                                "Session archived".into()
                            } else {
                                "Session restored".into()
                            };
                        }
                        Err(error) => self.status = error,
                    }
                }
                GatewayEvent::SessionRenamed {
                    request_id,
                    session_id,
                    profile,
                    result,
                } => {
                    let action_key = session_identity_key(&session_id, profile.as_deref());
                    if self.pending_session_renames.get(&action_key) != Some(&request_id) {
                        continue;
                    }
                    match result {
                        Ok(title) => {
                            self.pending_session_renames.remove(&action_key);
                            self.runtime_session_renames.remove(&action_key);
                            self.uncertain_session_renames.remove(&action_key);
                            for session in self
                                .sessions
                                .iter_mut()
                                .chain(self.archived_sessions.iter_mut())
                            {
                                if session.id == session_id && session.profile == profile {
                                    session.title = title.clone();
                                }
                            }
                            for result in &mut self.session_search_results {
                                if result.session_id == session_id && result.profile == profile {
                                    result.title = Some(title.clone());
                                }
                            }
                            self.rename_session = None;
                            self.status = "Session renamed".into();
                        }
                        Err(error) => {
                            if self.runtime_session_renames.contains_key(&action_key)
                                && !error.starts_with("Hermes session.mutate refused:")
                                && !error.starts_with("Hermes session.resume")
                                && !error.starts_with("Hermes rename snapshot")
                            {
                                self.uncertain_session_renames.insert(action_key);
                                self.status = format!(
                                    "Rename outcome unknown; retry the original request: {error}"
                                );
                            } else {
                                self.pending_session_renames.remove(&action_key);
                                self.runtime_session_renames.remove(&action_key);
                                self.uncertain_session_renames.remove(&action_key);
                                self.status = error;
                            }
                        }
                    }
                }
                GatewayEvent::ToolsetEnabled {
                    request_id,
                    name,
                    result,
                } => {
                    let Some(pending) = self.pending_toolset_updates.get(&name) else {
                        continue;
                    };
                    if pending.request_id != request_id {
                        continue;
                    }
                    let pending = self
                        .pending_toolset_updates
                        .remove(&name)
                        .expect("pending toolset update checked");
                    match result {
                        Ok(enabled) => {
                            if let Some(toolset) = self.toolsets.as_mut().and_then(|toolsets| {
                                toolsets.iter_mut().find(|toolset| toolset.name == name)
                            }) {
                                toolset.enabled = enabled;
                            }
                            self.capabilities_error = None;
                            self.status =
                                format!("{} {name}", if enabled { "Enabled" } else { "Disabled" });
                        }
                        Err(error) => {
                            if let Some(toolset) = self.toolsets.as_mut().and_then(|toolsets| {
                                toolsets.iter_mut().find(|toolset| toolset.name == name)
                            }) {
                                toolset.enabled = pending.previous_enabled;
                            }
                            if self.route == AppRoute::Capabilities {
                                self.capabilities_error = Some(error.clone());
                            }
                            self.status = error;
                        }
                    }
                }
                GatewayEvent::CapabilitiesLoaded { request_id, result } => {
                    if self.route != AppRoute::Capabilities
                        || self.capabilities_request_id.as_deref() != Some(&request_id)
                    {
                        continue;
                    }
                    self.capabilities_pending = false;
                    self.capabilities_request_id = None;
                    match result {
                        Ok(CapabilitiesSnapshot { skills, toolsets }) => {
                            self.skills = Some(skills);
                            self.toolsets = Some(toolsets);
                            self.capabilities_error = None;
                        }
                        Err(error) => {
                            self.capabilities_error = Some(error);
                        }
                    }
                }
                GatewayEvent::Disconnected(error) => {
                    self.gateway_connected = false;
                    self.archived_sessions_request_id = None;
                    self.archived_sessions_loading = false;
                    self.pending_interrupt = None;
                    for prompt in self.approval_prompts.values_mut() {
                        prompt.response_pending = None;
                    }
                    for prompt in self.clarify_prompts.values_mut() {
                        prompt.response_pending = None;
                    }
                    for prompt in self.batch_clarify_prompts.values_mut() {
                        prompt.response_pending = false;
                        prompt.pending_locks.clear();
                        prompt.lock_in_flight = None;
                    }
                    self.session_search_pending = false;
                    self.session_search_request_id = None;
                    self.capabilities_pending = false;
                    self.capabilities_request_id = None;
                    self.status = error;
                }
            }
        }
        if load_capabilities_after_connect {
            self.load_capabilities()
                .unwrap_or_else(|error| self.capabilities_error = Some(error));
        }
        if load_archived_after_connect {
            self.load_archived_sessions();
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn merge_older_transcript_page(
    older: Vec<serde_json::Value>,
    current: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let overlap = (1..=older.len().min(current.len()))
        .rev()
        .find(|length| older[older.len() - length..] == current[..*length])
        .unwrap_or(0);
    let older_prefix_len = older.len() - overlap;
    let mut merged = Vec::with_capacity(older_prefix_len + current.len());
    merged.extend(older.into_iter().take(older_prefix_len));
    merged.extend(current);
    merged
}

fn apply_stream_event_to_transcript(
    transcript: &mut Vec<serde_json::Value>,
    event_type: &str,
    payload: &serde_json::Value,
    execution_generation: Option<u64>,
) {
    match event_type {
        "tool.start" | "tool.complete" => {
            upsert_tool_row(transcript, event_type, payload, execution_generation);
        }
        "message.delta" => {
            let delta = payload
                .get("text")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if delta.is_empty() {
                return;
            }
            if let Some(message) = transcript.iter_mut().rev().find(|message| {
                message.get("role").and_then(|value| value.as_str()) == Some("assistant")
                    && message.get("streaming").and_then(|value| value.as_bool()) == Some(true)
            }) {
                if let Some(content) = message
                    .get("content")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
                {
                    message["content"] = serde_json::Value::String(format!("{content}{delta}"));
                }
            } else {
                transcript
                    .push(serde_json::json!({"role":"assistant","content":delta,"streaming":true}));
            }
        }
        "message.complete" => {
            let final_text = payload
                .get("text")
                .or_else(|| payload.get("rendered"))
                .and_then(|value| value.as_str());
            if let Some(message) = transcript.iter_mut().rev().find(|message| {
                message.get("role").and_then(|value| value.as_str()) == Some("assistant")
                    && message.get("streaming").and_then(|value| value.as_bool()) == Some(true)
            }) {
                if let Some(text) = final_text.filter(|text| !text.is_empty()) {
                    message["content"] = serde_json::Value::String(text.to_owned());
                }
                message["streaming"] = serde_json::Value::Bool(false);
            } else if let Some(text) = final_text.filter(|text| !text.is_empty()) {
                transcript.push(serde_json::json!({"role":"assistant","content":text}));
            }
        }
        _ => {}
    }
}

fn transcript_message(message: &serde_json::Value) -> (String, String) {
    if message.get("type").and_then(|value| value.as_str()) == Some("tool_call") {
        let name = message
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("tool");
        let status = message
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("running");
        let speaker = format!("{name} · {status}");
        let content = [
            "result_text",
            "summary",
            "inline_diff",
            "result",
            "preview",
            "args_text",
            "args",
            "context",
        ]
        .iter()
        .find_map(|key| message.get(*key).filter(|value| !value.is_null()))
        .unwrap_or(&serde_json::Value::Null);
        return (speaker, bounded_transcript_value(content, 6_000));
    }
    let role = message
        .get("role")
        .or_else(|| message.get("type"))
        .and_then(|value| value.as_str())
        .unwrap_or("Hermes");
    let content = message.get("content").unwrap_or(&serde_json::Value::Null);
    let body = match content {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter_map(|part| {
                part.as_str().map(str::to_owned).or_else(|| {
                    part.get("text")
                        .and_then(|value| value.as_str())
                        .map(str::to_owned)
                })
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    (role.to_owned(), body)
}

fn render_transcript_markdown(ui: &mut Ui, cache: &mut CommonMarkCache, markdown: &str) {
    CommonMarkViewer::new().show(ui, cache, markdown);
}
fn bounded_transcript_value(value: &serde_json::Value, max_chars: usize) -> String {
    let text = value.as_str().map(str::to_owned).unwrap_or_else(|| {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    });
    if text.chars().count() <= max_chars {
        text
    } else {
        let prefix = text.chars().take(max_chars).collect::<String>();
        format!("{prefix}…")
    }
}

fn upsert_tool_row(
    transcript: &mut Vec<serde_json::Value>,
    event_type: &str,
    payload: &serde_json::Value,
    execution_generation: Option<u64>,
) {
    let Some(tool_id) = payload.get("tool_id").and_then(|value| value.as_str()) else {
        return;
    };
    let is_start = event_type == "tool.start";
    let index = transcript
        .iter_mut()
        .enumerate()
        .rev()
        .find_map(|(index, row)| {
            let same_call = row.get("type").and_then(|value| value.as_str()) == Some("tool_call")
                && row.get("tool_call_id").and_then(|value| value.as_str()) == Some(tool_id)
                && execution_generation.is_none_or(|generation| {
                    row.get("execution_generation")
                        .and_then(|value| value.as_u64())
                        == Some(generation)
                });
            let state = row.get("status").and_then(|value| value.as_str());
            if same_call && (state == Some("running") || (!is_start && state == Some("complete"))) {
                Some(index)
            } else {
                None
            }
        });
    let index = index.unwrap_or_else(|| {
        transcript.push(serde_json::json!({
            "type":"tool_call",
            "tool_call_id":tool_id,
            "execution_generation":execution_generation,
            "status": if is_start { "running" } else { "complete" }
        }));
        transcript.len() - 1
    });
    let row = &mut transcript[index];
    if let Some(name) = payload.get("name").filter(|value| !value.is_null()) {
        row["name"] = name.clone();
    } else if row.get("name").is_none() {
        row["name"] = serde_json::json!("tool");
    }
    if let Some(generation) = execution_generation {
        row["execution_generation"] = serde_json::json!(generation);
    }
    if is_start {
        row["status"] = serde_json::json!("running");
        for key in ["args", "args_text", "context", "preview", "labels"] {
            if let Some(value) = payload.get(key).filter(|value| !value.is_null()) {
                row[key] = value.clone();
            }
        }
    } else {
        row["status"] = serde_json::json!("complete");
        for key in [
            "args",
            "result",
            "result_text",
            "summary",
            "inline_diff",
            "duration_s",
            "todos",
            "revision",
            "labels",
            "image_path",
        ] {
            if let Some(value) = payload.get(key).filter(|value| !value.is_null()) {
                row[key] = value.clone();
            }
        }
    }
}

fn clarify_answer(prompt: &ClarifyPrompt) -> String {
    if prompt.multi_select {
        let mut values = prompt.selected.clone();
        let draft = prompt.draft.trim();
        if !draft.is_empty() {
            values.push(draft.to_owned());
        }
        if values.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&values).unwrap_or_default()
        }
    } else {
        prompt
            .selected
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| prompt.draft.trim().to_owned())
    }
}

fn batch_clarify_answer(prompt: &BatchClarifyPrompt) -> Option<String> {
    let mut answers = serde_json::Map::new();
    let mut answered_count = 0_usize;
    for question in &prompt.questions {
        let answer = batch_question_answer(question)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null);
        answered_count += usize::from(!answer.is_null());
        answers.insert(question.qid.clone(), answer);
    }
    if answered_count == 0 {
        return None;
    }
    serde_json::to_string(&serde_json::json!({"answers": answers})).ok()
}

fn batch_question_answer(question: &BatchClarifyQuestion) -> Option<String> {
    let draft = question.draft.trim();
    let values = question
        .selected
        .iter()
        .map(|choice| bare_clarify_choice(choice).to_owned())
        .collect::<Vec<_>>();
    if question.multi_select {
        let mut values = values;
        if !draft.is_empty() {
            values.push(draft.to_owned());
        }
        (!values.is_empty())
            .then(|| serde_json::to_string(&values).ok())
            .flatten()
    } else if let Some(choice) = values.first() {
        Some(choice.clone())
    } else {
        (!draft.is_empty()).then(|| draft.to_owned())
    }
}

fn bare_clarify_choice(choice: &str) -> &str {
    choice
        .strip_suffix(" (Recommended)")
        .unwrap_or(choice)
        .trim_end()
}

fn stage_clarify_answer(prompt: &mut ClarifyPrompt, answer: &str) {
    if prompt.multi_select {
        let values =
            serde_json::from_str::<Vec<String>>(answer).unwrap_or_else(|_| vec![answer.to_owned()]);
        for value in &values {
            if prompt.choices.iter().any(|choice| choice == value)
                && !prompt.selected.contains(value)
            {
                prompt.selected.push(value.clone());
            }
        }
        prompt.draft = values
            .into_iter()
            .filter(|value| !prompt.choices.contains(value))
            .collect::<Vec<_>>()
            .join(", ");
    } else if prompt.choices.iter().any(|choice| choice == answer) {
        prompt.selected.push(answer.to_owned());
    } else {
        prompt.draft = answer.to_owned();
    }
}

fn session_identity_key(session_id: &str, profile: Option<&str>) -> String {
    format!("{}\0{session_id}", profile.unwrap_or("default"))
}

fn merge_sessions_by_profile(
    loaded: Vec<SessionSummary>,
    existing: Vec<SessionSummary>,
) -> Vec<SessionSummary> {
    let mut seen = BTreeSet::new();
    loaded
        .into_iter()
        .chain(existing)
        .filter(|session| {
            seen.insert(session_identity_key(
                &session.id,
                session.profile.as_deref(),
            ))
        })
        .collect()
}

fn session_matches_filter(session: &SessionSummary, filter: &str) -> bool {
    let query = filter.trim();
    if query.is_empty() {
        return true;
    }
    [
        session.title.as_str(),
        session.preview.as_str(),
        session.id.as_str(),
    ]
    .iter()
    .any(|field| field.to_lowercase().contains(&query.to_lowercase()))
}

fn merge_session_search_results(
    loaded: &[SessionSummary],
    query: &str,
    server_results: &[SessionSearchResult],
    pending: bool,
) -> Vec<SessionSummary> {
    let query = query.trim();
    if query.is_empty() {
        return loaded.to_vec();
    }
    let local_matches = loaded
        .iter()
        .filter(|session| session_matches_filter(session, query))
        .cloned()
        .collect::<Vec<_>>();
    if pending {
        return local_matches;
    }

    let mut seen = BTreeSet::<String>::new();
    let mut merged = Vec::new();
    for result in server_results {
        if !seen.insert(result.session_id.clone()) {
            continue;
        }
        let session = loaded
            .iter()
            .find(|session| session.id == result.session_id)
            .cloned()
            .unwrap_or_else(|| session_summary_from_search(result));
        merged.push(session);
    }
    for session in local_matches {
        if seen.insert(session.id.clone()) {
            merged.push(session);
        }
    }
    merged
}

fn session_summary_from_search(result: &SessionSearchResult) -> SessionSummary {
    SessionSummary {
        id: result.session_id.clone(),
        profile: result.profile.clone(),
        title: result
            .title
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_owned(),
        preview: result
            .snippet
            .replace(">>>", "")
            .replace("<<<", "")
            .trim()
            .to_owned(),
        message_count: 0,
        source: result.source.clone().unwrap_or_default(),
        started_at: result
            .last_active
            .or(result.session_started)
            .unwrap_or_default(),
        last_active: result.last_active,
        pinned: false,
        unread: false,
        archived: false,
    }
}

fn session_age_label(started_at: f64, now: f64) -> String {
    let seconds = (now - started_at).max(0.0) as u64;
    match seconds {
        0..60 => "now".into(),
        60..3600 => format!("{}m", seconds / 60),
        3600..86400 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

fn session_recency_timestamp(session: &SessionSummary) -> f64 {
    session
        .last_active
        .filter(|timestamp| *timestamp != 0.0)
        .unwrap_or(session.started_at)
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

fn next_session_pane_for_workspace(
    document: &doofusdock_core::LayoutDocument,
) -> Option<(PanelId, String, Option<String>)> {
    let workspace = PanelId::from(WORKSPACE);
    let mut candidates = Vec::new();
    if let Some(group) = collect_dock_tab_groups(document.root.as_deref())
        .into_iter()
        .find(|group| group.panels.contains(&workspace))
        && let Some(workspace_index) = group.panels.iter().position(|panel| panel == &workspace)
    {
        candidates.extend(group.panels.iter().skip(workspace_index + 1).cloned());
        candidates.extend(group.panels.iter().take(workspace_index).rev().cloned());
    }
    candidates.extend(
        document
            .panels
            .iter()
            .filter(|descriptor| descriptor.role == "hermes_session_tile")
            .map(|descriptor| descriptor.id.clone()),
    );

    let mut seen = BTreeSet::new();
    candidates.into_iter().find_map(|panel| {
        if !seen.insert(panel.clone()) || document.closed_panels.contains(&panel) {
            return None;
        }
        let descriptor = document.descriptor(&panel)?;
        if descriptor.role != "hermes_session_tile"
            || find_panel_node(document.root.as_deref(), &panel.0).is_none()
        {
            return None;
        }
        let session_id = descriptor.metadata.get("hermes.session.id")?.clone();
        let profile = descriptor
            .metadata
            .get("hermes.session.profile")
            .filter(|profile| !profile.is_empty())
            .cloned();
        Some((panel, session_id, profile))
    })
}

fn collect_dock_tab_groups(root: Option<&Node>) -> Vec<DockTabGroup> {
    fn first_panel(node: &Node) -> Option<PanelId> {
        match &node.kind {
            NodeKind::Panel { panel } => Some(panel.clone()),
            NodeKind::Tabs { items, .. }
            | NodeKind::DocumentWell { groups: items, .. }
            | NodeKind::Split {
                children: items, ..
            }
            | NodeKind::Extension {
                children: items, ..
            } => items.iter().find_map(|item| first_panel(item)),
            NodeKind::Placeholder { .. } => None,
        }
    }
    fn walk(node: &Node, groups: &mut Vec<DockTabGroup>) {
        match &node.kind {
            NodeKind::Tabs {
                items,
                active_index,
            } => {
                let panels = items
                    .iter()
                    .filter_map(|item| first_panel(item))
                    .collect::<Vec<_>>();
                if panels.len() == items.len() && panels.len() > 1 {
                    groups.push(DockTabGroup {
                        node: node.id.clone(),
                        panels,
                        active_index: (*active_index).min(items.len().saturating_sub(1)),
                    });
                }
                for child in items {
                    walk(child, groups);
                }
            }
            NodeKind::Split { children, .. }
            | NodeKind::DocumentWell {
                groups: children, ..
            }
            | NodeKind::Extension { children, .. } => {
                for child in children {
                    walk(child, groups);
                }
            }
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => {}
        }
    }
    let mut groups = Vec::new();
    if let Some(root) = root {
        walk(root, &mut groups);
    }
    groups
}

struct DockSplit {
    node: NodeId,
    orientation: Orientation,
    weights: Vec<f64>,
}

fn collect_dock_splits(root: Option<&Node>, splits: &mut Vec<DockSplit>) {
    let Some(node) = root else {
        return;
    };
    match &node.kind {
        NodeKind::Split {
            orientation,
            weights,
            children,
        } => {
            splits.push(DockSplit {
                node: node.id.clone(),
                orientation: *orientation,
                weights: weights.clone(),
            });
            for child in children {
                collect_dock_splits(Some(child), splits);
            }
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Extension {
            children: items, ..
        } => {
            for child in items {
                collect_dock_splits(Some(child), splits);
            }
        }
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => {}
    }
}

fn set_split_weights(root: &mut Option<Arc<Node>>, target: &NodeId, weights: &[f64]) -> bool {
    root.as_mut()
        .is_some_and(|node| set_split_weights_at(node, target, weights))
}

fn set_split_weights_at(node: &mut Arc<Node>, target: &NodeId, weights: &[f64]) -> bool {
    let node = Arc::make_mut(node);
    if &node.id == target {
        if let NodeKind::Split {
            weights: current, ..
        } = &mut node.kind
        {
            if current.len() == weights.len() {
                current.copy_from_slice(weights);
                return true;
            }
        }
        return false;
    }
    match &mut node.kind {
        NodeKind::Split { children, .. }
        | NodeKind::Tabs {
            items: children, ..
        }
        | NodeKind::DocumentWell {
            groups: children, ..
        }
        | NodeKind::Extension { children, .. } => children
            .iter_mut()
            .any(|child| set_split_weights_at(child, target, weights)),
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => false,
    }
}

fn resized_split_weights(initial: &[f64], divider: usize, delta: f32, extent: f32) -> Vec<f64> {
    let mut weights = initial.to_vec();
    if divider + 1 >= weights.len() || extent <= 0.0 {
        return weights;
    }
    let pair = weights[divider] + weights[divider + 1];
    let first = (weights[divider] + (delta / extent) as f64).clamp(0.05 * pair, 0.95 * pair);
    weights[divider] = first;
    weights[divider + 1] = pair - first;
    weights
}

fn compute_drop_edge(rect: Rect, pointer: egui::Pos2) -> DropEdge {
    let x = (pointer.x - rect.left()) / rect.width().max(1.0);
    let y = (pointer.y - rect.top()) / rect.height().max(1.0);
    if x < 0.22 {
        DropEdge::Left
    } else if x > 0.78 {
        DropEdge::Right
    } else if y < 0.22 {
        DropEdge::Top
    } else if y > 0.78 {
        DropEdge::Bottom
    } else {
        DropEdge::Center
    }
}

fn compute_drop_preview(rect: Rect, edge: DropEdge) -> Rect {
    match edge {
        DropEdge::Center => rect.shrink2(Vec2::new(rect.width() * 0.18, rect.height() * 0.18)),
        DropEdge::Left => Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + rect.width() * 0.4, rect.bottom()),
        ),
        DropEdge::Right => Rect::from_min_max(
            egui::pos2(rect.right() - rect.width() * 0.4, rect.top()),
            rect.max,
        ),
        DropEdge::Top => Rect::from_min_max(
            rect.min,
            egui::pos2(rect.right(), rect.top() + rect.height() * 0.4),
        ),
        DropEdge::Bottom => Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - rect.height() * 0.4),
            rect.max,
        ),
    }
}

fn panel_drop_operation(source: PanelId, target: PanelDropTarget) -> Operation {
    if target.edge == DropEdge::Center {
        return Operation::CenterPanel {
            panel: source,
            target_panel: target.panel,
        };
    }
    let (orientation, before) = match target.edge {
        DropEdge::Left => (Orientation::Horizontal, true),
        DropEdge::Right => (Orientation::Horizontal, false),
        DropEdge::Top => (Orientation::Vertical, true),
        DropEdge::Bottom => (Orientation::Vertical, false),
        DropEdge::Center => unreachable!("center drop handled above"),
    };
    Operation::SplitPanel {
        panel: source,
        target: target.node,
        orientation,
        before,
        ratio: 0.4,
    }
}

#[cfg(all(test, feature = "agent-automation"))]
mod tests {
    use super::*;
    use egui_kittest::kittest::{NodeT, Queryable};

    #[test]
    fn older_transcript_history_exposes_an_accessible_control() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("long-session".into());
        app.active_session_profile = Some("fixture-profile".into());
        app.gateway_connected = true;
        app.transcript = vec![json!({"role":"assistant","content":"recent"})];
        app.transcript_message_count = 240;
        app.transcript_has_more = true;
        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_ui(|ui| app.draw(ui));
        let button = harness
            .query_by_label("Show earlier messages")
            .expect("long transcript exposes the earlier-page action");
        assert_eq!(format!("{:?}", button.accesskit_node().role()), "Button");
        drop(harness);
        app.transcript.clear();
        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_ui(|ui| app.draw(ui));
        assert!(
            harness.query_by_label("Show earlier messages").is_some(),
            "history remains reachable when resume supplied no tail rows"
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn markdown_fixture_renders_table_and_code_copy_action_accessibly() {
        let (mut app, profile) = isolated_app();
        app.load_markdown_fixture_for_automation();
        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));

        let copy = harness
            .query_by_label("Copy")
            .expect("fenced code exposes a copy control");
        assert_eq!(format!("{:?}", copy.accesskit_node().role()), "Button");
        assert!(
            harness
                .query_all_by(|node| node.value().as_deref() == Some("Native rendering"))
                .next()
                .is_some(),
            "GitHub-style table cells remain visible to agent locators"
        );

        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }
    #[test]
    fn older_transcript_page_prepends_and_deduplicates_boundary_overlap() {
        let older = vec![json!({"id":1}), json!({"id":2}), json!({"id":3})];
        let current = vec![json!({"id":3}), json!({"id":4})];
        assert_eq!(
            merge_older_transcript_page(older, current),
            vec![
                json!({"id":1}),
                json!({"id":2}),
                json!({"id":3}),
                json!({"id":4})
            ]
        );
    }

    #[test]
    fn session_filter_matches_title_preview_and_id_without_case_sensitivity() {
        let session = SessionSummary {
            id: "session-42".into(),
            profile: None,
            title: "Repair Gateway".into(),
            preview: "Fix the reconnect handler".into(),
            message_count: 4,
            source: "desktop".into(),
            started_at: 0.0,
            last_active: None,
            pinned: false,
            unread: false,
            archived: false,
        };
        assert!(session_matches_filter(&session, "repair"));
        assert!(session_matches_filter(&session, "RECONNECT"));
        assert!(session_matches_filter(&session, "42"));
        assert!(session_matches_filter(&session, "   "));
        assert!(!session_matches_filter(&session, "missing"));
        assert_eq!(session_age_label(995.0, 1000.0), "now");
        assert_eq!(session_age_label(640.5, 1000.5), "6m");
        assert_eq!(session_age_label(0.0, 7200.0), "2h");
        let mut recency = session.clone();
        recency.started_at = 100.0;
        recency.last_active = Some(995.0);
        assert_eq!(session_recency_timestamp(&recency), 995.0);
        recency.last_active = Some(0.0);
        assert_eq!(session_recency_timestamp(&recency), 100.0);
    }

    #[test]
    fn session_merge_keeps_colliding_ids_in_separate_profiles() {
        let make_session = |profile: Option<&str>, title: &str| SessionSummary {
            id: "shared-session-id".into(),
            profile: profile.map(str::to_owned),
            title: title.into(),
            preview: String::new(),
            message_count: 0,
            source: "desktop".into(),
            started_at: 1.0,
            last_active: None,
            pinned: false,
            unread: false,
            archived: false,
        };
        let merged = merge_sessions_by_profile(
            vec![
                make_session(None, "Default profile"),
                make_session(Some("work"), "Work profile"),
            ],
            vec![make_session(Some("work"), "Stale duplicate")],
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].title, "Default profile");
        assert_eq!(merged[1].title, "Work profile");
    }

    #[test]
    fn registered_open_session_scopes_colliding_ids_and_path_navigation_rejects_ambiguity() {
        let (mut app, profile_dir) = isolated_app();
        let default_session = SessionSummary {
            id: "shared-id".into(),
            profile: None,
            title: "Default".into(),
            preview: String::new(),
            message_count: 0,
            source: "desktop".into(),
            started_at: 1.0,
            last_active: None,
            pinned: false,
            unread: false,
            archived: false,
        };
        let mut work_session = default_session.clone();
        work_session.profile = Some("work".into());
        app.sessions = vec![default_session, work_session];
        app.execute_registered_action(RegisteredUiAction::OpenSession {
            session_id: "shared-id".into(),
            profile: Some("work".into()),
        })
        .expect("explicit profile selects the intended session");
        assert_eq!(app.route, AppRoute::Session("shared-id".into()));
        assert_eq!(app.active_session_profile.as_deref(), Some("work"));
        let pane = app.workspace.session_pane_ids().into_iter().find(|panel| {
            app.workspace
                .dock
                .snapshot()
                .document
                .descriptor(panel)
                .is_some_and(|descriptor| {
                    descriptor
                        .metadata
                        .get("hermes.session.profile")
                        .map(String::as_str)
                        == Some("work")
                        && descriptor
                            .metadata
                            .get("hermes.session.id")
                            .map(String::as_str)
                            == Some("shared-id")
                })
        });
        assert!(
            pane.is_some(),
            "opening a session registers a profile-safe Dock tile"
        );
        let error = app
            .execute_registered_action(RegisteredUiAction::Navigate {
                path: "/shared-id".into(),
            })
            .expect_err("path-only navigation must not choose a profile silently");
        assert!(error.contains("multiple profiles"));
        assert_eq!(app.active_session_profile.as_deref(), Some("work"));
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn dynamic_session_pane_renders_its_profile_view_without_replacing_the_main_chat() {
        let (mut app, profile_dir) = isolated_app();
        app.gateway_connected = true;
        app.route = AppRoute::Session("main-session".into());
        app.active_session_profile = Some("personal".into());
        app.active_runtime_session_id = Some("runtime-personal".into());
        app.transcript = vec![json!({"role":"assistant","content":"Personal main transcript"})];
        app.draft = "Personal main draft".into();
        let panel = app
            .workspace
            .open_session_pane(Some("work"), "shared-pane-id", "Work session")
            .expect("register profile-scoped session pane");
        app.session_chat_views.insert(
            session_identity_key("shared-pane-id", Some("work")),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-work".into()),
                transcript: vec![json!({
                    "role":"assistant",
                    "content":"Work pane transcript"
                })],
                draft: "Work pane draft".into(),
                ..SessionChatViewState::default()
            },
        );

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let work_transcript_is_visible = harness
            .query_all_by(|node| node.value().as_deref() == Some("Work pane transcript"))
            .next()
            .is_some();
        let work_draft_is_visible = harness
            .query_all_by(|node| node.value().as_deref() == Some("Work pane draft"))
            .next()
            .is_some();
        drop(harness);
        assert!(
            work_transcript_is_visible,
            "the canonical Dock tile renders the matching profile-scoped transcript"
        );
        assert!(
            work_draft_is_visible,
            "the composer uses the pane-specific draft"
        );
        assert!(
            app.workspace
                .dock
                .snapshot()
                .document
                .descriptor(&panel)
                .is_some_and(|descriptor| descriptor.role == "hermes_session_tile")
        );
        assert_eq!(app.route, AppRoute::Session("main-session".into()));
        assert_eq!(app.active_session_profile.as_deref(), Some("personal"));
        assert_eq!(
            app.active_runtime_session_id.as_deref(),
            Some("runtime-personal")
        );
        assert_eq!(app.transcript[0]["content"], "Personal main transcript");
        assert_eq!(app.draft, "Personal main draft");
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn active_session_pane_close_requires_confirmation_and_does_not_interrupt_hermes() {
        let (mut app, profile_dir) = isolated_app();
        let panel = app
            .workspace
            .open_session_pane(Some("work"), "busy-session", "Busy session")
            .expect("register busy session pane");
        let key = session_identity_key("busy-session", Some("work"));
        app.session_chat_views.insert(
            key.clone(),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-busy".into()),
                ..SessionChatViewState::default()
            },
        );
        app.pending_submissions.insert(
            session_identity_key("runtime-busy", Some("work")),
            PendingPrompt {
                session_id: "runtime-busy".into(),
                profile: Some("work".into()),
                submission_id: "submission-busy".into(),
                text: "working".into(),
                uncertain: false,
                admission_id: None,
                execution_generation: Some(1),
                resolving_unknown: false,
            },
        );
        assert!(app.session_pane_is_busy(&panel));
        app.request_close_session_pane(&panel);
        assert_eq!(app.pending_session_close.as_ref(), Some(&panel));
        assert!(
            !app.workspace
                .dock
                .snapshot()
                .document
                .closed_panels
                .contains(&panel)
        );
        assert!(
            app.pending_submissions
                .contains_key(&session_identity_key("runtime-busy", Some("work")))
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn close_primary_workspace_promotes_nearest_session_pane_by_profile() {
        let (mut app, profile_dir) = isolated_app();
        app.route = AppRoute::Session("main-session".into());
        app.active_session_profile = Some("personal".into());
        app.active_runtime_session_id = Some("main-runtime".into());
        app.transcript = vec![json!({"role":"assistant","content":"Main transcript"})];
        app.draft = "Main draft".into();
        let next_panel = app
            .workspace
            .open_session_pane(Some("work"), "next-session", "Next session")
            .expect("open the next canonical tab");
        app.workspace
            .select_panel(&PanelId::from(WORKSPACE))
            .expect("activate the primary workspace tab");
        app.session_chat_views.insert(
            session_identity_key("next-session", Some("work")),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("next-runtime".into()),
                transcript: vec![json!({"role":"assistant","content":"Promoted transcript"})],
                draft: "Promoted draft".into(),
                ..SessionChatViewState::default()
            },
        );

        app.execute_registered_action(RegisteredUiAction::CloseActiveTab)
            .expect("close the primary workspace tab");

        let document = app.workspace.dock.snapshot().document;
        assert!(document.closed_panels.contains(&next_panel));
        assert_eq!(app.route, AppRoute::Session("next-session".into()));
        assert_eq!(app.active_session_profile.as_deref(), Some("work"));
        assert_eq!(
            app.active_runtime_session_id.as_deref(),
            Some("next-runtime")
        );
        assert_eq!(app.transcript[0]["content"], "Promoted transcript");
        assert_eq!(app.draft, "Promoted draft");
        assert!(
            app.session_chat_views
                .contains_key(&session_identity_key("main-session", Some("personal")))
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn close_occupied_primary_without_other_tabs_returns_to_new_draft() {
        let (mut app, profile_dir) = isolated_app();
        app.route = AppRoute::Session("main-session".into());
        app.active_session_profile = Some("work".into());
        app.active_runtime_session_id = Some("main-runtime".into());
        app.transcript = vec![json!({"role":"assistant","content":"Main transcript"})];
        app.new_chat_draft = "stale draft".into();

        app.execute_registered_action(RegisteredUiAction::CloseActiveTab)
            .expect("close the occupied workspace chat");

        assert_eq!(app.route, AppRoute::NewChat);
        assert!(app.new_chat_draft.is_empty());
        assert!(app.active_runtime_session_id.is_none());
        assert!(app.transcript.is_empty());
        assert!(
            app.session_chat_views
                .contains_key(&session_identity_key("main-session", Some("work")))
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn close_selected_busy_session_tab_uses_existing_confirmation() {
        let (mut app, profile_dir) = isolated_app();
        let panel = app
            .workspace
            .open_session_pane(Some("work"), "busy-session", "Busy session")
            .expect("open session tab");
        app.session_chat_views.insert(
            session_identity_key("busy-session", Some("work")),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-busy".into()),
                ..SessionChatViewState::default()
            },
        );
        app.pending_submissions.insert(
            session_identity_key("runtime-busy", Some("work")),
            PendingPrompt {
                session_id: "runtime-busy".into(),
                profile: Some("work".into()),
                submission_id: "submission-busy".into(),
                text: "working".into(),
                uncertain: false,
                admission_id: None,
                execution_generation: Some(1),
                resolving_unknown: false,
            },
        );
        app.workspace
            .select_panel(&panel)
            .expect("select the busy tab");

        app.execute_registered_action(RegisteredUiAction::CloseActiveTab)
            .expect("close action should request confirmation");

        assert_eq!(app.pending_session_close.as_ref(), Some(&panel));
        assert!(
            !app.workspace
                .dock
                .snapshot()
                .document
                .closed_panels
                .contains(&panel)
        );
        assert!(
            app.pending_submissions
                .contains_key(&session_identity_key("runtime-busy", Some("work")))
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn new_session_reuses_the_untouched_blank_tile_for_the_same_profile() {
        let (mut app, profile_dir) = isolated_app();
        app.active_session_profile = Some("work".into());
        app.untouched_blank_session = Some(("blank-session".into(), Some("work".into())));

        app.create_blank_session_tab();

        assert_eq!(app.route, AppRoute::Session("blank-session".into()));
        assert_eq!(app.active_session_profile.as_deref(), Some("work"));
        assert!(
            app.pending_create.is_none(),
            "reuse does not issue session.create again"
        );
        assert_eq!(app.workspace.session_pane_ids().len(), 1);
        assert_eq!(app.status, "Reused the untouched new-session tab");
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn new_session_does_not_reuse_a_blank_tile_after_its_draft_is_edited() {
        let (mut app, profile_dir) = isolated_app();
        app.active_session_profile = Some("work".into());
        app.untouched_blank_session = Some(("blank-session".into(), Some("work".into())));
        app.session_chat_views.insert(
            session_identity_key("blank-session", Some("work")),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-blank".into()),
                draft: "keep this draft in its own tab".into(),
                ..SessionChatViewState::default()
            },
        );

        app.create_blank_session_tab();

        assert!(app.untouched_blank_session.is_none());
        assert!(app.pending_create.is_none());
        assert!(app.workspace.session_pane_ids().is_empty());
        assert_eq!(app.status, "Hermes gateway is unavailable");
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn hidden_session_submission_receipt_updates_its_cached_pane_transcript() {
        let (mut app, profile_dir) = isolated_app();
        let key = session_identity_key("shared-id", Some("work"));
        app.session_chat_views.insert(
            key.clone(),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-work".into()),
                transcript: vec![json!({
                    "role":"user",
                    "content":"work only",
                    "submission_id":"submission-work"
                })],
                draft: String::new(),
                ..SessionChatViewState::default()
            },
        );
        app.pending_submissions.insert(
            session_identity_key("runtime-work", Some("work")),
            PendingPrompt {
                session_id: "runtime-work".into(),
                profile: Some("work".into()),
                submission_id: "submission-work".into(),
                text: "work only".into(),
                uncertain: false,
                admission_id: None,
                execution_generation: None,
                resolving_unknown: false,
            },
        );

        app.handle_prompt_submitted(
            "runtime-work".into(),
            "submission-work".into(),
            Ok(PromptSubmitReceipt::Admitted {
                admission_id: "admission-work".into(),
                status: "started".into(),
                execution_generation: Some(3),
            }),
        );

        assert_eq!(
            app.session_chat_views[&key].transcript[0]["admission_id"],
            "admission-work"
        );
        assert!(
            app.transcript.is_empty(),
            "a background receipt must not touch the visible chat"
        );
        assert_eq!(
            app.pending_submissions[&session_identity_key("runtime-work", Some("work"))]
                .execution_generation,
            Some(3)
        );
        assert_eq!(app.sessions[0].id, "shared-id");
        assert_eq!(app.sessions[0].profile.as_deref(), Some("work"));
        assert_eq!(app.sessions[0].title, "work only");
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn session_rows_expose_the_stored_id_in_the_accessible_name() {
        let (mut app, profile) = isolated_app();
        app.sessions.push(SessionSummary {
            id: "sess-42".into(),
            profile: None,
            title: "Pinned parity review".into(),
            preview: "Inspect session routes".into(),
            message_count: 3,
            source: "desktop".into(),
            started_at: 100.0,
            last_active: Some(200.0),
            pinned: false,
            unread: false,
            archived: false,
        });
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let row = harness
            .query_all_by(|node| {
                node.label().as_deref() == Some("Pinned parity review, session sess-42")
            })
            .next()
            .expect("session row has an accessible stable identity");
        assert_eq!(format!("{:?}", row.accesskit_node().role()), "Button");
        assert!(
            row.rect().is_positive(),
            "session row must be visible to locators"
        );
        let menu = harness
            .query_all_by(|node| node.label().as_deref() == Some("Session actions session sess-42"))
            .next()
            .expect("session action menu has a stable locator");
        menu.click();
        harness.run();
        let pin = harness
            .query_all_by(|node| node.label().as_deref() == Some("Pin session sess-42"))
            .next()
            .expect("session pin action is exposed");
        assert!(pin.accesskit_node().is_disabled());
        let unread = harness
            .query_all_by(|node| node.label().as_deref() == Some("Mark unread session sess-42"))
            .next()
            .expect("session unread action is exposed");
        assert!(unread.accesskit_node().is_disabled());
        let archive = harness
            .query_all_by(|node| node.label().as_deref() == Some("Archive session sess-42"))
            .next()
            .expect("session archive action is exposed");
        assert!(archive.accesskit_node().is_disabled());
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn archived_view_shows_archived_rows_and_a_reachable_unarchive_action() {
        let (mut app, profile) = isolated_app();
        app.show_archived_sessions = true;
        app.gateway_endpoint = Some(GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "fixture-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: "http://127.0.0.1:1".into(),
            capabilities: Vec::new(),
            control_home: None,
        });
        app.archived_sessions.push(SessionSummary {
            id: "old-session-9".into(),
            profile: Some("work".into()),
            title: "Archived planning".into(),
            preview: "Review previous notes".into(),
            message_count: 9,
            source: "desktop".into(),
            started_at: 100.0,
            last_active: Some(200.0),
            pinned: false,
            unread: false,
            archived: true,
        });
        app.navigate_path("/old-session-9")
            .expect("unique archived session path resolves its profile");
        assert_eq!(app.active_session_profile.as_deref(), Some("work"));
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let row = harness
            .query_all_by(|node| {
                node.label().as_deref()
                    == Some("Archived planning, profile work, session old-session-9")
            })
            .next()
            .expect("archived row is present in archived view");
        assert!(!row.accesskit_node().is_disabled());
        assert!(row.rect().is_positive());
        let menu = harness
            .query_all_by(|node| {
                node.label().as_deref()
                    == Some("Session actions profile work session old-session-9")
            })
            .next()
            .expect("archived row has its actions menu");
        menu.click();
        harness.run();
        let restore = harness
            .query_all_by(|node| {
                node.label().as_deref() == Some("Unarchive profile work session old-session-9")
            })
            .next()
            .expect("archived row exposes its restore action");
        assert!(restore.accesskit_node().is_disabled());
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn foreign_profile_session_menu_opens_rename_dialog_with_accessible_input() {
        let (mut app, profile_dir) = isolated_app();
        app.gateway_connected = true;
        app.gateway_endpoint = Some(GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "fixture-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: "http://127.0.0.1:1".into(),
            capabilities: Vec::new(),
            control_home: None,
        });
        app.sessions.push(SessionSummary {
            id: "session-42".into(),
            profile: Some("work".into()),
            title: "Old title".into(),
            preview: String::new(),
            message_count: 1,
            source: "desktop".into(),
            started_at: 1.0,
            last_active: None,
            pinned: false,
            unread: false,
            archived: false,
        });
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        harness
            .query_by_label("Session actions profile work session session-42")
            .expect("profile-scoped menu exists")
            .click();
        harness.run();
        let rename = harness
            .query_by_label("Rename profile work session session-42")
            .expect("profile-scoped rename action exists");
        assert!(!rename.accesskit_node().is_disabled());
        rename.click();
        harness.run();
        assert!(harness.query_by_label("Session title").is_some());
        assert!(harness.query_by_label("Save session title").is_some());
        drop(harness);
        assert_eq!(
            app.rename_session
                .as_ref()
                .and_then(|row| row.profile.as_deref()),
            Some("work")
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn sessions_view_toggle_switches_to_cached_archived_rows() {
        let (mut app, profile) = isolated_app();
        app.archived_sessions.push(SessionSummary {
            id: "saved-session-3".into(),
            profile: None,
            title: "Saved draft".into(),
            preview: "An archived preview".into(),
            message_count: 2,
            source: "desktop".into(),
            started_at: 100.0,
            last_active: Some(150.0),
            pinned: false,
            unread: false,
            archived: true,
        });
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        harness.get_by_label("Show archived sessions").click();
        harness.run();
        assert!(
            harness
                .get_by_label("Saved draft, session saved-session-3")
                .rect()
                .is_positive()
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn server_session_search_keeps_rank_and_appends_loaded_only_matches() {
        let loaded = vec![
            SessionSummary {
                id: "loaded-hit".into(),
                profile: None,
                title: "Matched loaded session".into(),
                preview: "current result".into(),
                message_count: 4,
                source: "desktop".into(),
                started_at: 90.0,
                last_active: None,
                pinned: false,
                unread: false,
                archived: false,
            },
            SessionSummary {
                id: "branch-hit".into(),
                profile: None,
                title: "Local cwd match".into(),
                preview: "needle from local preview".into(),
                message_count: 2,
                source: "desktop".into(),
                started_at: 80.0,
                last_active: None,
                pinned: false,
                unread: false,
                archived: false,
            },
        ];
        let server = vec![
            SessionSearchResult {
                session_id: "archived-hit".into(),
                profile: None,
                title: Some("Archived match".into()),
                snippet: "old >>>needle<<< result".into(),
                last_active: Some(75.0),
                session_started: Some(70.0),
                lineage_root: Some("root-1".into()),
                model: None,
                role: None,
                source: Some("desktop".into()),
            },
            SessionSearchResult {
                session_id: "loaded-hit".into(),
                profile: None,
                title: Some("Backend title".into()),
                snippet: "server snippet".into(),
                last_active: None,
                session_started: None,
                lineage_root: None,
                model: None,
                role: None,
                source: None,
            },
        ];

        let results = merge_session_search_results(&loaded, "needle", &server, false);
        assert_eq!(
            results
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["archived-hit", "loaded-hit", "branch-hit"]
        );
        assert_eq!(results[0].preview, "old needle result");
        assert_eq!(results[0].started_at, 75.0);
        assert_eq!(results[1].title, "Matched loaded session");

        let immediate = merge_session_search_results(&loaded, "needle", &server, true);
        assert_eq!(immediate.len(), 1);
        assert_eq!(immediate[0].id, "branch-hit");
    }

    #[test]
    fn canonical_panel_drop_commits_a_tab_operation_and_persists_it() {
        let (mut app, profile) = isolated_app();
        app.workspace
            .toggle_pane(FILES)
            .expect("reveal files panel for test");
        let before = app.workspace.dock.snapshot();
        let target_node = find_panel_node(before.document.root.as_deref(), WORKSPACE)
            .expect("workspace node in quad layout");
        let target = PanelDropTarget {
            panel: PanelId::from(WORKSPACE),
            node: target_node,
            edge: DropEdge::Center,
            preview: Rect::ZERO,
        };

        app.commit_panel_drop(PanelId::from(FILES), target);

        let after = app.workspace.dock.snapshot();
        assert!(
            after.revision > before.revision,
            "drop transaction did not commit: {}",
            app.status
        );
        assert!(find_panel_node(after.document.root.as_deref(), FILES).is_some());
        let groups = collect_dock_tab_groups(after.document.root.as_deref());
        assert_eq!(groups.len(), 1, "center drop did not form one tab group");
        assert_eq!(
            groups[0].panels,
            vec![PanelId::from(WORKSPACE), PanelId::from(FILES)]
        );
        let bounds = Box2 {
            x: 0.0,
            y: 0.0,
            w: 1200.0,
            h: 800.0,
        };
        let projected = project(&after.document, bounds).expect("project centered tab group");
        assert!(projected.panels[&PanelId::from(FILES)].opacity > 0.0);
        assert_eq!(projected.panels[&PanelId::from(WORKSPACE)].opacity, 0.0);
        app.workspace
            .dock
            .transact(
                after.revision,
                CommandId::from("select-workspace-tab-test"),
                [Operation::SelectTab {
                    tabs: groups[0].node.clone(),
                    index: 0,
                }],
            )
            .expect("select the other canonical tab");
        let selected = app.workspace.dock.snapshot();
        let projected = project(&selected.document, bounds).expect("project selected tab");
        assert!(projected.panels[&PanelId::from(WORKSPACE)].opacity > 0.0);
        assert_eq!(projected.panels[&PanelId::from(FILES)].opacity, 0.0);
        assert_eq!(app.status, "Panel layout updated");
        assert!(profile.join("layouts").exists());
        assert!(
            app.workspace.persistence_error.is_none(),
            "layout persistence failed: {:?}",
            app.workspace.persistence_error
        );
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn tab_drag_without_tab_target_preserves_normal_panel_drop_session() {
        let (mut app, profile) = isolated_app();
        let snapshot = app.workspace.dock.snapshot();
        let source = PanelId::from(SESSIONS);
        app.begin_panel_drag(source.clone());
        app.tab_reorder = Some(TabReorderSession {
            revision: snapshot.revision,
            source: source.clone(),
            container: NodeId::from("tabs-test-container"),
            target_index: None,
        });

        assert!(!app.finish_tab_reorder());
        assert!(app.panel_drag.is_some(), "normal panel drag was discarded");
        assert_eq!(app.panel_drag_source, Some(source));
        app.cancel_panel_drag();
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn panel_drop_regions_match_center_and_each_edge() {
        let rect = Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(210.0, 120.0));
        assert_eq!(
            compute_drop_edge(rect, egui::pos2(110.0, 70.0)),
            DropEdge::Center
        );
        assert_eq!(
            compute_drop_edge(rect, egui::pos2(20.0, 70.0)),
            DropEdge::Left
        );
        assert_eq!(
            compute_drop_edge(rect, egui::pos2(200.0, 70.0)),
            DropEdge::Right
        );
        assert_eq!(
            compute_drop_edge(rect, egui::pos2(110.0, 25.0)),
            DropEdge::Top
        );
        assert_eq!(
            compute_drop_edge(rect, egui::pos2(110.0, 115.0)),
            DropEdge::Bottom
        );
    }

    #[test]
    fn split_resize_preserves_weight_pair_and_clamps_each_side() {
        let centered = resized_split_weights(&[0.25, 0.75, 0.4], 0, 25.0, 100.0);
        assert!((centered[0] - 0.5).abs() < 1e-9);
        assert!((centered[1] - 0.5).abs() < 1e-9);
        assert_eq!(centered[2], 0.4);
        let upper_clamped = resized_split_weights(&[0.25, 0.75], 0, 100.0, 100.0);
        assert!((upper_clamped[0] - 0.95).abs() < 1e-9);
        assert!((upper_clamped[1] - 0.05).abs() < 1e-9);
        let lower_clamped = resized_split_weights(&[0.25, 0.75], 0, -100.0, 100.0);
        assert!((lower_clamped[0] - 0.05).abs() < 1e-9);
        assert!((lower_clamped[1] - 0.95).abs() < 1e-9);
        assert_eq!(
            resized_split_weights(&[0.25, 0.75], 2, 20.0, 100.0),
            vec![0.25, 0.75]
        );
        assert_eq!(
            resized_split_weights(&[0.25, 0.75], 0, 20.0, 0.0),
            vec![0.25, 0.75]
        );
    }

    fn isolated_app() -> (HermesNativeApp, std::path::PathBuf) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let profile = std::env::temp_dir().join(format!(
            "HermesNative-Agent-Test-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&profile).expect("create isolated test profile");
        (
            HermesNativeApp::new_for_automation(profile.clone()),
            profile,
        )
    }

    fn seed_pending_submission(
        app: &mut HermesNativeApp,
        submission_id: &str,
        admission_id: Option<&str>,
        uncertain: bool,
    ) {
        let session_id = "runtime-session";
        let profile = Some("work".to_owned());
        app.active_session_profile = profile.clone();
        app.pending_submissions.insert(
            session_identity_key(session_id, profile.as_deref()),
            PendingPrompt {
                session_id: session_id.into(),
                profile,
                submission_id: submission_id.into(),
                text: "hello".into(),
                uncertain,
                admission_id: admission_id.map(str::to_owned),
                execution_generation: None,
                resolving_unknown: false,
            },
        );
    }

    #[test]
    fn message_stream_events_project_into_the_active_runtime_transcript() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        seed_pending_submission(&mut app, "submission-1", Some("admission-1"), false);

        app.handle_server_event(serde_json::json!({
            "type":"message.start","session_id":"runtime-session",
            "payload":{"turn_id":"turn-1"}
        }));
        app.handle_server_event(serde_json::json!({
            "type":"message.delta","session_id":"runtime-session",
            "payload":{"text":"Hello "}
        }));
        app.handle_server_event(serde_json::json!({
            "type":"message.delta","session_id":"runtime-session",
            "payload":{"text":"Hermes"}
        }));
        app.handle_server_event(serde_json::json!({
            "type":"message.complete","session_id":"runtime-session",
            "payload":{"admission_id":"admission-1","text":"Hello Hermes!","status":"complete"}
        }));

        assert_eq!(app.transcript.len(), 1);
        assert_eq!(app.transcript[0]["role"], "assistant");
        assert_eq!(app.transcript[0]["content"], "Hello Hermes!");
        assert_eq!(app.transcript[0]["streaming"], false);
        assert!(app.pending_submissions.is_empty());
        assert_eq!(app.status, "Hermes response complete");
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn uncertain_submission_keeps_identity_until_matching_admission_settles() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        seed_pending_submission(&mut app, "submission-1", None, false);
        app.transcript.push(serde_json::json!({
            "role":"user","content":"hello","submission_id":"submission-1"
        }));
        app.handle_prompt_submitted(
            "runtime-session".into(),
            "submission-1".into(),
            Err("Hermes prompt.submit request timed out".into()),
        );
        let key = session_identity_key("runtime-session", Some("work"));
        assert!(app.pending_submissions.contains_key(&key));
        assert!(app.pending_submissions[&key].uncertain);
        assert_eq!(app.transcript.len(), 1);
        assert!(app.draft.is_empty());
        app.gateway_connected = true;
        {
            let harness = egui_kittest::Harness::builder()
                .with_size(egui::vec2(1440.0, 900.0))
                .build_ui(|ui| app.draw(ui));
            let retry = harness
                .query_by_label("Retry message with original submission ID")
                .expect("uncertain send exposes an agent-accessible retry");
            assert!(!retry.accesskit_node().is_disabled());
        }

        app.handle_prompt_submitted(
            "runtime-session".into(),
            "submission-1".into(),
            Ok(PromptSubmitReceipt::Admitted {
                admission_id: "admission-1".into(),
                status: "started".into(),
                execution_generation: None,
            }),
        );
        assert_eq!(app.transcript[0]["admission_id"], "admission-1");
        assert!(!app.pending_submissions[&key].uncertain);
        app.handle_server_event(serde_json::json!({
            "type":"message.complete","session_id":"runtime-session",
            "payload":{"admission_id":"someone-else","text":"another turn"}
        }));
        assert!(app.pending_submissions.contains_key(&key));
        app.handle_server_event(serde_json::json!({
            "type":"message.complete","session_id":"runtime-session",
            "payload":{"admission_id":"admission-1","text":"reply"}
        }));
        assert!(app.pending_submissions.is_empty());
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn completion_before_submit_receipt_settles_matching_admission() {
        let (mut app, profile) = isolated_app();
        app.active_runtime_session_id = Some("runtime-session".into());
        seed_pending_submission(&mut app, "submission-1", None, false);
        app.handle_server_event(serde_json::json!({
            "type":"message.complete","session_id":"runtime-session",
            "payload":{"admission_id":"admission-1","text":"reply"}
        }));
        assert!(!app.pending_submissions.is_empty());
        app.handle_prompt_submitted(
            "runtime-session".into(),
            "submission-1".into(),
            Ok(PromptSubmitReceipt::Admitted {
                admission_id: "admission-1".into(),
                status: "started".into(),
                execution_generation: None,
            }),
        );
        assert!(app.pending_submissions.is_empty());
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn interrupt_receipts_are_correlated_and_preserve_terminal_outcomes() {
        let (mut app, profile) = isolated_app();
        app.active_runtime_session_id = Some("runtime-session".into());
        seed_pending_submission(&mut app, "submission-1", Some("admission-1"), false);
        app.pending_interrupt = Some(("interrupt-1".into(), "runtime-session".into()));
        app.transcript
            .push(serde_json::json!({"role":"assistant","content":"partial","streaming":true}));

        app.handle_session_interrupted(
            "stale-interrupt".into(),
            "runtime-session".into(),
            Ok(true),
        );
        assert_eq!(app.pending_interrupt.as_ref().unwrap().0, "interrupt-1");
        assert_eq!(app.transcript[0]["streaming"], true);

        app.handle_session_interrupted("interrupt-1".into(), "runtime-session".into(), Ok(true));
        assert!(app.pending_interrupt.is_none());
        assert!(app.pending_submissions.is_empty());
        assert_eq!(app.transcript[0]["streaming"], false);
        assert_eq!(app.transcript[0]["interrupted"], true);
        assert_eq!(app.status, "Hermes response stopped");

        seed_pending_submission(&mut app, "submission-2", Some("admission-2"), false);
        app.pending_interrupt = Some(("interrupt-2".into(), "runtime-session".into()));
        app.handle_session_interrupted("interrupt-2".into(), "runtime-session".into(), Ok(false));
        assert!(app.pending_submissions.is_empty());
        assert_eq!(app.status, "Hermes reported no active response");

        seed_pending_submission(&mut app, "submission-3", Some("admission-3"), false);
        app.pending_interrupt = Some(("interrupt-3".into(), "runtime-session".into()));
        app.handle_session_interrupted(
            "interrupt-3".into(),
            "runtime-session".into(),
            Err("gateway refused interruption".into()),
        );
        assert!(app.pending_interrupt.is_none());
        assert!(!app.pending_submissions.is_empty());
        assert_eq!(app.status, "gateway refused interruption");

        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn pending_response_exposes_enabled_accessible_stop_control() {
        let (mut app, profile) = isolated_app();
        app.gateway_connected = true;
        app.active_runtime_session_id = Some("runtime-session".into());
        seed_pending_submission(&mut app, "submission-1", Some("admission-1"), false);
        let state = app.automation_state();
        assert_eq!(
            state["chat"]["active_runtime_session_id"],
            "runtime-session"
        );
        assert_eq!(state["chat"]["submission_pending"], true);
        assert_eq!(state["chat"]["can_stop"], true);
        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let stop_controls = harness
            .query_all_by(|node| node.label().as_deref() == Some("Stop response"))
            .collect::<Vec<_>>();
        assert_eq!(stop_controls.len(), 1);
        assert!(
            !stop_controls[0].accesskit_node().is_disabled(),
            "active-response Stop control must be enabled"
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn inbound_approval_request_uses_the_accessible_active_session_card() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        app.handle_server_request(
            "server-request-1".into(),
            "approval".into(),
            serde_json::json!({
                "session_id":"runtime-session",
                "request_id":"approval-1",
                "command":"git push origin main",
                "description":"Publish this branch",
                "choices":["once","always","deny"],
                "allow_permanent":false
            }),
        );
        let prompt = app
            .approval_prompts
            .get("approval-1")
            .expect("active-session approval card");
        assert_eq!(
            prompt.server_request_id.as_deref(),
            Some("server-request-1")
        );
        assert_eq!(prompt.choices, ["once", "deny"]);
        assert_eq!(prompt.command, "git push origin main");

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let allow = harness
            .query_all_by(|node| node.label().as_deref() == Some("Allow once approval approval-1"))
            .next()
            .expect("accessible allow-once control");
        assert!(allow.accesskit_node().is_disabled());
        assert_eq!(
            harness
                .query_all_by(
                    |node| node.label().as_deref() == Some("Always allow approval approval-1")
                )
                .count(),
            0,
            "the card must honor allow_permanent=false"
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn inbound_batch_clarify_request_renders_accessible_question_controls() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        app.handle_server_request(
            "clarify-wire-1".into(),
            "clarify".into(),
            serde_json::json!({
                "session_id":"runtime-session",
                "questions":[
                    {"qid":"q1","question":"Which package should be updated?","choices":["Core","Desktop (Recommended)"]},
                    {"qid":"q2","question":"Which checks should run?","choices":["Unit","UI"],"multi_select":true},
                    {"qid":"q3","question":"Should I preserve the current profile?"}
                ],
                "answers":{"q1":"Desktop","q3":null}
            }),
        );
        app.gateway_connected = true;
        let prompt = app
            .batch_clarify_prompts
            .get("clarify-wire-1")
            .expect("active-session batch clarify card");
        assert_eq!(prompt.questions.len(), 3);
        assert_eq!(prompt.questions[0].selected, ["Desktop (Recommended)"]);
        assert!(prompt.questions[2].selected.is_empty());
        assert_eq!(app.automation_state()["chat"]["clarify_pending_count"], 1);

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        for label in [
            "Clarify question q1: Which package should be updated?",
            "Clarify question q2: Which checks should run?",
            "Clarify question q3: Should I preserve the current profile?",
            "Clarify choice Desktop (Recommended) for q1",
            "Clarify choice Unit for q2",
            "Clarify answer q1 for clarify-wire-1",
            "Confirm and continue clarify answers clarify-wire-1",
            "Skip clarify request clarify-wire-1",
        ] {
            assert!(
                harness
                    .query_all_by(|node| node.label().as_deref() == Some(label))
                    .next()
                    .is_some(),
                "missing accessible batch clarify node: {label}"
            );
        }
        for label in [
            "Confirm and continue clarify answers clarify-wire-1",
            "Skip clarify request clarify-wire-1",
        ] {
            let node = harness
                .query_all_by(|node| node.label().as_deref() == Some(label))
                .next()
                .expect("batch clarify action");
            assert!(!node.accesskit_node().is_disabled());
        }
        let replayed_choice = harness
            .query_all_by(|node| {
                node.label().as_deref() == Some("Clarify choice Desktop (Recommended) for q1")
            })
            .next()
            .expect("editable replayed choice");
        assert!(!replayed_choice.accesskit_node().is_disabled());
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn hidden_session_approval_and_clarify_requests_render_in_their_dock_pane() {
        let (mut app, profile_dir) = isolated_app();
        app.gateway_connected = true;
        app.workspace
            .open_session_pane(Some("work"), "stored-work", "Work session")
            .expect("open the background session tile");
        app.session_chat_views.insert(
            session_identity_key("stored-work", Some("work")),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-work".into()),
                ..SessionChatViewState::default()
            },
        );
        app.handle_server_request(
            "approval-wire-hidden".into(),
            "approval".into(),
            json!({
                "session_id":"runtime-work",
                "profile":"work",
                "request_id":"approval-hidden-prompt",
                "command":"run tests",
                "choices":["once","deny"]
            }),
        );
        app.handle_server_request(
            "clarify-wire-hidden".into(),
            "clarify".into(),
            json!({
                "session_id":"runtime-work",
                "profile":"work",
                "questions":[{"qid":"hidden-q1","question":"Choose a target","choices":["Desktop","CLI"]}]
            }),
        );
        assert!(app.approval_prompts.contains_key("approval-hidden-prompt"));
        assert!(
            app.batch_clarify_prompts
                .contains_key("clarify-wire-hidden")
        );
        let approval_error = app
            .execute_registered_action(RegisteredUiAction::RespondApproval {
                prompt_id: "approval-hidden-prompt".into(),
                choice: "once".into(),
            })
            .expect_err("isolated fixture has no gateway writer");
        assert_eq!(approval_error, "Hermes gateway writer is unavailable");
        let clarify_error = app
            .execute_registered_action(RegisteredUiAction::RespondClarify {
                prompt_id: "clarify-wire-hidden".into(),
                answer: String::new(),
            })
            .expect_err("isolated fixture has no gateway writer");
        assert_eq!(clarify_error, "Hermes gateway writer is unavailable");

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        for label in [
            "Allow once approval approval-hidden-prompt",
            "Deny approval approval-hidden-prompt",
            "Clarify question hidden-q1: Choose a target",
            "Clarify choice Desktop for hidden-q1",
            "Confirm and continue clarify answers clarify-wire-hidden",
        ] {
            assert!(
                harness
                    .query_all_by(|node| node.label().as_deref() == Some(label))
                    .next()
                    .is_some(),
                "hidden session pane did not expose {label}"
            );
        }
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn server_requests_fail_closed_when_a_runtime_matches_multiple_profiles() {
        let (mut app, profile_dir) = isolated_app();
        for profile in ["work", "personal"] {
            app.session_chat_views.insert(
                session_identity_key("stored-session", Some(profile)),
                SessionChatViewState {
                    profile: Some(profile.into()),
                    runtime_session_id: Some("ambiguous-runtime".into()),
                    ..SessionChatViewState::default()
                },
            );
        }
        app.handle_server_request(
            "approval-wire-ambiguous".into(),
            "approval".into(),
            json!({
                "session_id":"ambiguous-runtime",
                "request_id":"ambiguous-prompt",
                "choices":["once"]
            }),
        );
        assert!(app.approval_prompts.is_empty());
        drop(app);
        std::fs::remove_dir_all(profile_dir).expect("remove isolated test profile");
    }

    #[test]
    fn batch_clarify_answers_preserve_replay_and_normalize_recommended_choices() {
        let prompt = BatchClarifyPrompt {
            session_id: "runtime-session".into(),
            profile: None,
            server_request_id: "clarify-wire-1".into(),
            questions: vec![
                BatchClarifyQuestion {
                    qid: "q1".into(),
                    question: "Pick one".into(),
                    choices: vec!["Desktop (Recommended)".into()],
                    multi_select: false,
                    selected: vec!["Desktop (Recommended)".into()],
                    draft: String::new(),
                },
                BatchClarifyQuestion {
                    qid: "q2".into(),
                    question: "Choose many".into(),
                    choices: vec!["Unit".into(), "UI".into()],
                    multi_select: true,
                    selected: vec!["Unit".into(), "UI".into()],
                    draft: String::new(),
                },
                BatchClarifyQuestion {
                    qid: "q3".into(),
                    question: "Replay".into(),
                    choices: Vec::new(),
                    multi_select: false,
                    selected: Vec::new(),
                    draft: String::new(),
                },
            ],
            response_pending: false,
            pending_locks: VecDeque::new(),
            lock_in_flight: None,
        };
        let serialized = batch_clarify_answer(&prompt).expect("partially completed batch answers");
        let value: serde_json::Value = serde_json::from_str(&serialized).expect("valid JSON");
        assert_eq!(value["answers"]["q1"], "Desktop");
        assert_eq!(value["answers"]["q2"], "[\"Unit\",\"UI\"]");
        assert_eq!(value["answers"]["q3"], serde_json::Value::Null);
    }

    #[test]
    fn approval_event_renders_only_backend_choices_and_rejects_stale_actions() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        app.handle_server_event(serde_json::json!({
            "type":"approval.request",
            "session_id":"runtime-session",
            "execution_generation":7,
            "payload":{
                "prompt_id":"prompt-1",
                "command":"git push origin main",
                "description":"Publish this branch",
                "choices":["once","deny"]
            }
        }));
        assert_eq!(app.approval_prompts.len(), 1);
        let prompt = app
            .approval_prompts
            .get("prompt-1")
            .expect("pending prompt");
        assert_eq!(prompt.execution_generation, 7);
        assert_eq!(prompt.choices, ["once", "deny"]);
        assert!(
            app.execute_registered_action(RegisteredUiAction::RespondApproval {
                prompt_id: "prompt-1".into(),
                choice: "always".into(),
            })
            .is_err(),
            "a choice absent from the backend must not be sent"
        );
        assert!(
            app.execute_registered_action(RegisteredUiAction::RespondApproval {
                prompt_id: "stale-prompt".into(),
                choice: "once".into(),
            })
            .is_err(),
            "stale prompt IDs must not be sent"
        );

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let once = harness
            .query_all_by(|node| node.label().as_deref() == Some("Allow once approval prompt-1"))
            .next()
            .expect("accessible allow-once action");
        assert!(
            once.accesskit_node().is_disabled(),
            "offline approval actions stay disabled"
        );
        assert_eq!(
            harness
                .query_all_by(
                    |node| node.label().as_deref() == Some("Always allow approval prompt-1")
                )
                .count(),
            0,
            "the UI must not invent a choice the backend did not offer"
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn clarify_event_stages_multi_select_answer_and_exposes_accessible_actions() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        app.handle_server_event(serde_json::json!({
            "type":"clarify.request",
            "session_id":"runtime-session",
            "execution_generation":11,
            "payload":{
                "prompt_id":"question-1",
                "question":"Which colors should I use?",
                "choices":["red","blue"],
                "multi_select":true
            }
        }));
        let prompt = app
            .clarify_prompts
            .get_mut("question-1")
            .expect("pending clarify prompt");
        prompt.selected = vec!["red".into(), "blue".into()];
        prompt.draft = "gold".into();
        assert_eq!(clarify_answer(prompt), "[\"red\",\"blue\",\"gold\"]");
        assert!(
            app.execute_registered_action(RegisteredUiAction::RespondClarify {
                prompt_id: "stale-question".into(),
                answer: "red".into(),
            })
            .is_err(),
            "stale prompt IDs must not be sent"
        );
        assert!(
            app.execute_registered_action(RegisteredUiAction::RespondClarify {
                prompt_id: "question-1".into(),
                answer: "x".repeat(16_385),
            })
            .is_err(),
            "oversized answers must not be sent"
        );

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let red = harness
            .query_all_by(|node| {
                node.label().as_deref() == Some("Clarify choice red for question-1")
            })
            .next()
            .expect("accessible choice");
        assert!(
            red.accesskit_node().is_disabled(),
            "offline choices stay disabled"
        );
        let skip = harness
            .query_all_by(|node| node.label().as_deref() == Some("Skip clarify request question-1"))
            .next()
            .expect("accessible skip action");
        assert!(
            skip.accesskit_node().is_disabled(),
            "offline skip stays disabled"
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn tool_start_and_complete_render_generation_scoped_transcript_cards() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());
        app.handle_server_event(serde_json::json!({
            "type":"tool.start",
            "session_id":"runtime-session",
            "execution_generation":3,
            "payload":{"tool_id":"reused-id","name":"terminal","args":{"command":"pwd"}}
        }));
        app.handle_server_event(serde_json::json!({
            "type":"tool.complete",
            "session_id":"runtime-session",
            "execution_generation":3,
            "payload":{"tool_id":"reused-id","name":"terminal","result_text":"C:/project"}
        }));
        app.handle_server_event(serde_json::json!({
            "type":"tool.start",
            "session_id":"runtime-session",
            "execution_generation":4,
            "payload":{"tool_id":"reused-id","name":"terminal","args":{"command":"git status"}}
        }));
        app.handle_server_event(serde_json::json!({
            "type":"tool.complete",
            "session_id":"runtime-session",
            "execution_generation":4,
            "payload":{"tool_id":"reused-id","name":"terminal","summary":"Working tree clean"}
        }));

        assert_eq!(app.transcript.len(), 2);
        assert_eq!(app.transcript[0]["execution_generation"], 3);
        assert_eq!(app.transcript[0]["status"], "complete");
        assert_eq!(app.transcript[1]["execution_generation"], 4);
        assert_eq!(app.transcript[1]["status"], "complete");
        assert_eq!(
            transcript_message(&app.transcript[0]),
            ("terminal · complete".into(), "C:/project".into())
        );
        assert_eq!(
            transcript_message(&app.transcript[1]),
            ("terminal · complete".into(), "Working tree clean".into())
        );

        let harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(|ui| app.draw(ui));
        let labels = harness
            .query_all_by(|_| true)
            .filter_map(|node| node.accesskit_node().label())
            .collect::<Vec<_>>();
        assert_eq!(
            harness
                .query_all_by(|node| {
                    node.label()
                        .is_some_and(|label| label.starts_with("terminal · complete:"))
                })
                .count(),
            2,
            "tool call states must appear in the accessible transcript; labels: {labels:?}"
        );
        drop(harness);
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }

    #[test]
    fn switching_duplicate_session_ids_restores_profile_scoped_draft_and_transcript() {
        let (mut app, profile_dir) = isolated_app();
        app.route = AppRoute::Session("same-id".into());
        app.active_session_profile = Some("work".into());
        app.active_runtime_session_id = Some("runtime-work".into());
        app.transcript = vec![json!({"role":"assistant","content":"work transcript"})];
        app.draft = "work draft".into();
        app.save_active_chat_view();

        app.execute_registered_action(RegisteredUiAction::OpenSession {
            session_id: "same-id".into(),
            profile: Some("personal".into()),
        })
        .unwrap();
        app.active_runtime_session_id = Some("runtime-personal".into());
        app.transcript = vec![json!({"role":"assistant","content":"personal transcript"})];
        app.draft = "personal draft".into();
        app.save_active_chat_view();

        app.execute_registered_action(RegisteredUiAction::OpenSession {
            session_id: "same-id".into(),
            profile: Some("work".into()),
        })
        .unwrap();
        assert_eq!(
            app.active_runtime_session_id.as_deref(),
            Some("runtime-work")
        );
        assert_eq!(app.transcript[0]["content"], "work transcript");
        assert_eq!(app.draft, "work draft");

        app.execute_registered_action(RegisteredUiAction::OpenSession {
            session_id: "same-id".into(),
            profile: Some("personal".into()),
        })
        .unwrap();
        assert_eq!(
            app.active_runtime_session_id.as_deref(),
            Some("runtime-personal")
        );
        assert_eq!(app.transcript[0]["content"], "personal transcript");
        assert_eq!(app.draft, "personal draft");
        assert_eq!(app.session_chat_views.len(), 2);
        drop(app);
        std::fs::remove_dir_all(profile_dir).unwrap();
    }

    #[test]
    fn session_view_cache_is_bounded_and_keeps_the_visible_session() {
        let (mut app, profile_dir) = isolated_app();
        app.route = AppRoute::Session("visible".into());
        app.active_session_profile = Some("work".into());
        let visible_key = session_identity_key("visible", Some("work"));
        app.cache_session_chat_view(
            visible_key.clone(),
            SessionChatViewState {
                profile: Some("work".into()),
                runtime_session_id: Some("runtime-visible".into()),
                ..SessionChatViewState::default()
            },
        );
        for index in 0..40 {
            let id = format!("session-{index}");
            app.cache_session_chat_view(
                session_identity_key(&id, Some("work")),
                SessionChatViewState {
                    profile: Some("work".into()),
                    transcript: vec![json!({"role":"assistant","content":id})],
                    ..SessionChatViewState::default()
                },
            );
        }
        assert_eq!(app.session_chat_views.len(), 32);
        assert!(app.session_chat_views.contains_key(&visible_key));
        assert!(
            app.session_chat_views
                .contains_key(&session_identity_key("session-39", Some("work")))
        );
        assert!(
            !app.session_chat_views
                .contains_key(&session_identity_key("session-0", Some("work")))
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).unwrap();
    }

    #[test]
    fn background_stream_events_update_only_the_exact_cached_runtime_and_profile() {
        let (mut app, profile_dir) = isolated_app();
        app.route = AppRoute::Session("visible".into());
        app.active_runtime_session_id = Some("runtime-visible".into());
        for profile in ["work", "personal"] {
            app.session_chat_views.insert(
                session_identity_key("same-id", Some(profile)),
                SessionChatViewState {
                    profile: Some(profile.into()),
                    runtime_session_id: Some("runtime-duplicate".into()),
                    ..SessionChatViewState::default()
                },
            );
        }
        app.handle_server_event(json!({
            "type":"message.delta","session_id":"runtime-duplicate",
            "payload":{"text":"ambiguous"}
        }));
        assert!(
            app.session_chat_views
                .values()
                .all(|view| view.transcript.is_empty())
        );

        app.handle_server_event(json!({
            "type":"message.delta","session_id":"runtime-duplicate","profile":"work",
            "payload":{"text":"work background"}
        }));
        let work = &app.session_chat_views[&session_identity_key("same-id", Some("work"))];
        let personal = &app.session_chat_views[&session_identity_key("same-id", Some("personal"))];
        assert_eq!(work.transcript[0]["content"], "work background");
        assert!(personal.transcript.is_empty());
        assert!(
            app.transcript.is_empty(),
            "background deltas must not change visible chat"
        );
        drop(app);
        std::fs::remove_dir_all(profile_dir).unwrap();
    }

    #[test]
    fn events_from_a_different_runtime_session_do_not_touch_the_visible_chat() {
        let (mut app, profile) = isolated_app();
        app.route = AppRoute::Session("stored-session".into());
        app.active_runtime_session_id = Some("runtime-session".into());

        app.handle_server_event(serde_json::json!({
            "type":"message.delta","session_id":"another-runtime-session",
            "payload":{"text":"wrong chat"}
        }));

        assert!(app.transcript.is_empty());
        drop(app);
        std::fs::remove_dir_all(profile).expect("remove isolated test profile");
    }
}

impl HermesNativeApp {
    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let close_active_tab = ctx.input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::W,
            ))
        });
        if close_active_tab {
            self.execute_registered_action(RegisteredUiAction::CloseActiveTab)
                .unwrap_or_else(|error| self.status = error);
        }
        if !self.fonts_installed {
            crate::fonts::install(&ctx);
            self.fonts_installed = true;
        }
        self.update_gateway_discovery(&ctx);
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = BG;
        visuals.panel_fill = BG;
        visuals.extreme_bg_color = PANEL;
        visuals.widgets.noninteractive.fg_stroke.color = TEXT;
        visuals.widgets.inactive.bg_fill = PANEL_ALT;
        visuals.widgets.inactive.fg_stroke.color = TEXT;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 59, 69);
        visuals.widgets.active.bg_fill = Color32::from_rgb(60, 83, 115);
        visuals.selection.bg_fill = Color32::from_rgb(53, 91, 141);
        ctx.set_visuals(visuals);

        ui.painter().rect_filled(ui.max_rect(), 0.0, BG);
        self.toolbar(ui);
        self.pane_controls(ui);
        let content_rect = ui.available_rect_before_wrap();
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(content_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        {
            let ui = &mut content;
            let bounds = ui.available_rect_before_wrap();
            let projection_bounds = Box2 {
                x: bounds.min.x as f64,
                y: bounds.min.y as f64,
                w: bounds.width() as f64,
                h: bounds.height() as f64,
            };
            let layout = self.workspace.dock.snapshot().document;
            if let Some(drag) = &mut self.panel_drag {
                if let Err(error) = drag.update_preview(layout.revision, None) {
                    self.status = error.to_string();
                    self.panel_drag = None;
                    self.panel_drag_source = None;
                }
            }
            let mut projected_layout = layout.clone();
            if let Some(resize) = &self.split_resize {
                if resize.revision != layout.revision
                    || !set_split_weights(
                        &mut projected_layout.root,
                        &resize.split,
                        &resize.current_weights,
                    )
                {
                    self.split_resize = None;
                    self.status = "Split resize cancelled because the layout changed".into();
                    projected_layout = layout.clone();
                }
            }
            let closed_panels = layout.closed_panels.clone();
            let snapshot = match project(&projected_layout, projection_bounds) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.load_error = Some(error);
                    return;
                }
            };
            let tab_groups = collect_dock_tab_groups(layout.root.as_deref());
            let mut panels = snapshot
                .panels
                .iter()
                .map(|(panel, visual)| (panel.clone(), visual.clone()))
                .filter(|(panel, visual)| visual.opacity > 0.0 && !closed_panels.contains(panel))
                .collect::<Vec<_>>();
            panels.sort_by_key(|(_, visual)| visual.z);
            for (id, visual) in panels {
                let rect = Rect::from_min_max(
                    egui::pos2(visual.bounds.x as f32, visual.bounds.y as f32),
                    egui::pos2(
                        (visual.bounds.x + visual.bounds.w) as f32,
                        (visual.bounds.y + visual.bounds.h) as f32,
                    ),
                );
                ui.painter().rect_filled(rect, 0.0, PANEL);
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, BORDER),
                    egui::StrokeKind::Inside,
                );
                if tab_groups.iter().any(|group| group.panels.contains(&id)) {
                    let content_rect =
                        Rect::from_min_max(egui::pos2(rect.left(), rect.top() + 32.0), rect.max);
                    self.draw_panel(ui, id.0.as_str(), content_rect, rect, false);
                } else {
                    self.draw_panel(ui, id.0.as_str(), rect, rect, true);
                }
            }
            for group in &tab_groups {
                let Some(group_bounds) = snapshot.nodes.get(&group.node) else {
                    continue;
                };
                let group_rect = Rect::from_min_max(
                    egui::pos2(group_bounds.x as f32, group_bounds.y as f32),
                    egui::pos2(
                        (group_bounds.x + group_bounds.w) as f32,
                        (group_bounds.y + group_bounds.h) as f32,
                    ),
                );
                let tab_rect =
                    Rect::from_min_size(group_rect.min, Vec2::new(group_rect.width(), 32.0));
                let titles = group
                    .panels
                    .iter()
                    .filter_map(|panel| {
                        layout.descriptor(panel).map(|descriptor| {
                            if panel.0 == WORKSPACE {
                                self.route.title().to_owned()
                            } else {
                                descriptor.title_key.clone()
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                let labels = titles
                    .iter()
                    .map(|title| format!("Dock tab {title}"))
                    .collect::<Vec<_>>();
                let items = titles
                    .iter()
                    .zip(&labels)
                    .zip(&group.panels)
                    .map(|((title, label), panel)| TabStripItem {
                        title,
                        accessible_label: label,
                        icon: None,
                        modified: false,
                        pinned: false,
                        closable: layout
                            .descriptor(panel)
                            .is_some_and(|descriptor| descriptor.role == "hermes_session_tile"),
                        multi_selected: false,
                    })
                    .collect::<Vec<_>>();
                if items.len() < 2 {
                    continue;
                }
                let response = {
                    let state = self.dock_tabs.entry(group.node.0.clone()).or_default();
                    tab_strip(
                        ui,
                        Id::unique(("dock-tabs", &group.node.0)),
                        tab_rect,
                        &items,
                        group.active_index,
                        state,
                        &Self::chrome_style(),
                    )
                };
                if let Some(index) = response.drag_started
                    && let Some(panel) = group.panels.get(index)
                {
                    self.begin_panel_drag(panel.clone());
                    self.tab_reorder = Some(TabReorderSession {
                        revision: layout.revision,
                        source: panel.clone(),
                        container: group.node.clone(),
                        target_index: None,
                    });
                }
                if let Some(index) = response.close
                    && let Some(panel) = group.panels.get(index)
                    && layout
                        .descriptor(panel)
                        .is_some_and(|descriptor| descriptor.role == "hermes_session_tile")
                {
                    self.request_close_session_pane(panel);
                    break;
                }
                if let Some(reorder) = &mut self.tab_reorder
                    && reorder.container == group.node
                {
                    reorder.target_index = response.hovered.map(|(index, _)| index);
                }
                if let Some(index) = response.selected
                    && index != group.active_index
                {
                    match self.workspace.dock.transact(
                        layout.revision,
                        CommandId::from(format!("select-dock-tab-{}-{index}", group.node.0)),
                        [Operation::SelectTab {
                            tabs: group.node.clone(),
                            index,
                        }],
                    ) {
                        Ok(_) => {
                            self.workspace.save_layout();
                            self.status = "Dock tab selected".into();
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                    break;
                }
            }
            self.draw_split_resize_handles(ui, &layout, &snapshot);
            self.paint_panel_drop_preview(ui);
            let cancel_drag = (self.panel_drag.is_some() || self.split_resize.is_some())
                && ctx.input(|input| input.key_pressed(egui::Key::Escape));
            if cancel_drag {
                if self.panel_drag.is_some() {
                    self.cancel_panel_drag();
                } else {
                    self.status = "Split resize cancelled".into();
                }
                self.tab_reorder = None;
                self.split_resize = None;
            } else if ui.input(|input| input.pointer.primary_released()) {
                if !self.finish_tab_reorder() {
                    self.finish_panel_drag();
                }
                self.finish_split_resize();
            }
            if let Some(error) = &self.load_error {
                ui.painter().text(
                    bounds.left_bottom() + Vec2::new(12.0, -12.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("Layout recovery: {error}"),
                    FontId::proportional(11.0),
                    Color32::LIGHT_RED,
                );
            }
        }
        self.show_route_overlay(&ctx);
        self.show_rename_dialog(&ctx);
        self.show_session_close_dialog(&ctx);
    }

    #[cfg(feature = "agent-automation")]
    pub fn automation_state(&self) -> serde_json::Value {
        let layout = self.workspace.dock.snapshot();
        let mode = match self.workspace.preferences.mode {
            InterfaceMode::Advanced => "advanced",
            InterfaceMode::Simple => "simple",
        };
        serde_json::json!({
            "route": self.current_route_path(),
            "route_title": self.route.title(),
            "interface_mode": mode,
            "layout_profile": self.workspace.active_profile(),
            "layout_revision": layout.revision,
            "layout_document": &layout.document,
            "persistence_error": self.workspace.persistence_error,
            "dock_tab_groups": collect_dock_tab_groups(layout.document.root.as_deref())
                .iter()
                .map(|group| serde_json::json!({
                    "node": group.node.0,
                    "active_index": group.active_index,
                    "panels": group.panels.iter().map(|panel| panel.0.clone()).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>(),
            "available_profiles": self.workspace.profile_ids(),
            "panels": {
                "sessions": self.workspace.pane_is_open(SESSIONS),
                "workspace": self.workspace.pane_is_open(WORKSPACE),
                "review": self.workspace.pane_is_open(REVIEW),
                "files": self.workspace.pane_is_open(FILES),
                "terminal": self.workspace.pane_is_open(TERMINAL),
            },
            "gateway": {
                "state": if self.gateway_connected { "connected" } else { "isolated" },
                "session_count": self.sessions.len(),
                "show_archived_sessions": self.show_archived_sessions,
                "archived_session_count": self.archived_sessions.len(),
                "archived_sessions_loading": self.archived_sessions_loading,
            },
            "session_rename": {
                "open": self.rename_session.is_some(),
                "session_id": self.rename_session.as_ref().map(|session| session.id.as_str()),
                "profile": self.rename_session.as_ref().and_then(|session| session.profile.as_deref()),
                "pending_count": self.pending_session_renames.len(),
            },
            "chat": {
                "active_runtime_session_id": self.active_runtime_session_id.as_deref(),
                "transcript_visible_message_count": self.transcript.len(),
                "transcript_total_message_count": self.transcript_message_count,
                "transcript_has_older_messages": self.transcript_has_more,
                "transcript_page_loading": self.older_transcript_request.is_some(),
                "transcript_page_error": self.older_transcript_error.as_deref(),
                "active_profile": self.active_session_profile.as_deref(),
                "cached_session_view_count": self.session_chat_views.len(),
                "submission_pending": self.active_runtime_session_id.as_ref().is_some_and(|session_id| {
                    self.pending_submissions.contains_key(&session_identity_key(session_id, self.active_session_profile.as_deref()))
                }),
                "submission_pending_count": self.pending_submissions.len(),
                "submission_uncertain": self.active_runtime_session_id.as_ref().and_then(|session_id| {
                    self.pending_submissions.get(&session_identity_key(session_id, self.active_session_profile.as_deref()))
                }).is_some_and(|pending| pending.uncertain),
                "admission_id": self.active_runtime_session_id.as_ref().and_then(|session_id| {
                    self.pending_submissions.get(&session_identity_key(session_id, self.active_session_profile.as_deref()))
                }).and_then(|pending| pending.admission_id.as_deref()),
                "interrupt_pending": self.pending_interrupt.is_some(),
                "approval_pending_count": self.approval_prompts.len(),
                "clarify_pending_count": self.clarify_prompts.len() + self.batch_clarify_prompts.len(),
                "can_stop": self.gateway_connected
                    && self.active_runtime_session_id.as_ref().is_some_and(|session_id| {
                        self.pending_submissions.get(&session_identity_key(session_id, self.active_session_profile.as_deref()))
                            .is_some_and(|pending| !pending.uncertain)
                    }),
            },
            "transcript_message_count": self.transcript.len(),
            "load_error": self.load_error.as_deref(),
        })
    }
}

impl eframe::App for HermesNativeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

fn operation_id(kind: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("hermes-native-{kind}-{}-{nanos}", std::process::id())
}

fn profile_title(id: &str) -> &'static str {
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

fn panel_title(id: &str) -> &'static str {
    match id {
        REVIEW => "Review",
        FILES => "Files",
        TERMINAL => "Terminal",
        _ => "Panel",
    }
}
