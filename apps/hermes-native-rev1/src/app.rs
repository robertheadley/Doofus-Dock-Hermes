use crate::layout::{FILES, InterfaceMode, REVIEW, SESSIONS, TERMINAL, WORKSPACE, Workspace};
use doofusdock_presentation::{Box2, project};
use doofusdock_render_egui::{
    chrome::{PanelChromeControl, PanelChromeStyle, panel_title_bar},
    tabs::{TabStripItem, TabStripState, tab_strip},
};
use egui::{Align, Color32, FontId, Id, Layout, Rect, RichText, Stroke, Ui, Vec2};

const BG: Color32 = Color32::from_rgb(22, 24, 28);
const PANEL: Color32 = Color32::from_rgb(29, 32, 38);
const PANEL_ALT: Color32 = Color32::from_rgb(35, 38, 45);
const BORDER: Color32 = Color32::from_rgb(56, 60, 69);
const TEXT: Color32 = Color32::from_rgb(225, 228, 234);
const MUTED: Color32 = Color32::from_rgb(151, 158, 170);
const ACCENT: Color32 = Color32::from_rgb(104, 164, 245);

pub struct HermesNativeApp {
    workspace: Workspace,
    load_error: Option<String>,
    tabs: TabStripState,
    status: String,
}

impl HermesNativeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        match Workspace::load() {
            Ok(workspace) => Self {
                workspace,
                load_error: None,
                tabs: TabStripState::default(),
                status: "Backend offline".into(),
            },
            Err(error) => Self {
                workspace: Workspace::load_fallback(),
                load_error: Some(error),
                tabs: TabStripState::default(),
                status: "Backend offline".into(),
            },
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
                        let previous = self.workspace.preferences.mode;
                        let mut selected_mode = previous;
                        egui::ComboBox::from_id_salt("interface-mode")
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
                            if let Err(error) = self.workspace.set_mode(selected_mode) {
                                self.status = error;
                            }
                        }
                        let active = self.workspace.active_profile().to_owned();
                        let options = self.workspace.profile_ids();
                        let title = profile_title(&active);
                        let mut selected = active.clone();
                        egui::ComboBox::from_id_salt("layout-profile")
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
                            if let Err(error) = self.workspace.set_profile(&selected) {
                                self.status = error;
                            }
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
                            if let Err(error) = self.workspace.toggle_pane(id) {
                                self.status = error;
                            }
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

    fn draw_panel(&mut self, ui: &mut egui::Ui, id: &str, rect: Rect) {
        if rect.width() < 2.0 || rect.height() < 2.0 {
            return;
        }
        let rect = rect.intersect(ui.clip_rect());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::top_down(Align::Min)),
        );
        let title = match id {
            SESSIONS => "Sessions",
            WORKSPACE => "New session",
            REVIEW => "Review",
            FILES => "Files",
            TERMINAL => "Terminal",
            _ => id,
        };
        let controls = if id == SESSIONS || id == WORKSPACE {
            vec![]
        } else {
            vec![PanelChromeControl {
                glyph: "×",
                accessible_label: "Hide panel",
                tooltip: "Hide panel",
                enabled: true,
            }]
        };
        let bar = child.allocate_space(Vec2::new(rect.width(), 32.0)).1;
        let response = panel_title_bar(
            &mut child,
            Id::unique(("panel-title", id)),
            bar,
            title,
            &Self::chrome_style(),
            &controls,
        );
        if !response.activated_controls.is_empty() {
            if let Err(error) = self.workspace.toggle_pane(id) {
                self.status = error;
            }
        }
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
            _ => {}
        }
    }

    fn sessions_content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let new = ui.add_sized(
            [ui.available_width() - 20.0, 34.0],
            egui::Button::new(RichText::new("＋  New session").color(TEXT)),
        );
        if new.clicked() {
            self.status = "Backend offline · connect Hermes to create a session".into();
        }
        ui.add_space(18.0);
        ui.label(RichText::new("No sessions yet").size(13.0).color(MUTED));
    }

    fn workspace_content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let strip_rect = ui.allocate_space(Vec2::new(ui.available_width(), 34.0)).1;
        let items = [TabStripItem {
            title: "New session",
            accessible_label: "New session",
            icon: None,
            modified: false,
            pinned: false,
            closable: false,
            multi_selected: false,
        }];
        let mut selected = self.tabs.last_selected.unwrap_or(0);
        let tab_response = tab_strip(
            ui,
            Id::unique("workspace-tabs"),
            strip_rect,
            &items,
            0,
            &mut self.tabs,
            &Self::chrome_style(),
        );
        if tab_response.selected.is_some() {
            selected = 0;
        }
        let _ = selected;
        ui.add_space(24.0);
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
            ui.horizontal(|ui| {
                let connect = ui.add_sized(
                    [190.0, 38.0],
                    egui::Button::new(RichText::new("Connect to Hermes").color(TEXT))
                        .fill(Color32::from_rgb(57, 103, 163)),
                );
                if connect.clicked() {
                    self.status = "Backend offline · connection settings are not configured".into();
                }
                ui.add_sized(
                    [170.0, 38.0],
                    egui::Button::new(RichText::new("Install Hermes locally").color(TEXT)),
                );
            });
        });
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut String::new())
                    .hint_text("Connect to Hermes to send a message"),
            );
        });
    }

    fn empty_panel(&self, ui: &mut egui::Ui, message: &str) {
        ui.add_space(8.0);
        ui.label(RichText::new(message).size(12.0).color(MUTED));
    }
}

impl eframe::App for HermesNativeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
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
            let closed_panels = layout.closed_panels.clone();
            let snapshot = match project(&layout, projection_bounds) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.load_error = Some(error);
                    return;
                }
            };
            let mut panels = snapshot
                .panels
                .into_iter()
                .filter(|(panel, _)| !closed_panels.contains(panel))
                .collect::<Vec<_>>();
            panels.sort_by_key(|(_, visual)| visual.z);
            for (id, visual) in panels {
                let rect = Rect::from_min_size(
                    egui::pos2(visual.bounds.x as f32, visual.bounds.y as f32),
                    Vec2::new(visual.bounds.w as f32, visual.bounds.h as f32),
                );
                ui.painter().rect_filled(rect, 0.0, PANEL);
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, BORDER),
                    egui::StrokeKind::Inside,
                );
                self.draw_panel(ui, id.0.as_str(), rect);
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
    }
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
