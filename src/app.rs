use crate::layout::{PANEL_CHAT, PANEL_EXPLORER, PANEL_INSPECTOR, PANEL_TERMINAL, Workspace};
use doofusdock_core::{
    CommandId, LayoutDocument, Node, NodeId, NodeKind, Operation, Orientation, PanelId,
};
use doofusdock_presentation::{Box2, Snapshot};
use doofusdock_render_egui::tabs::{TabStripItem, TabStripState, tab_strip};
use eframe::egui::{
    self, Align, Color32, FontId, Id, Layout, Order, Pos2, Rect, RichText, Sense, Ui, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

const BG: Color32 = Color32::from_rgb(17, 20, 25);
const PANEL: Color32 = Color32::from_rgb(25, 29, 36);
const PANEL_ALT: Color32 = Color32::from_rgb(31, 36, 44);
const BORDER: Color32 = Color32::from_rgb(53, 61, 72);
const TEXT: Color32 = Color32::from_rgb(225, 231, 239);
const MUTED: Color32 = Color32::from_rgb(139, 151, 167);
const ACCENT: Color32 = Color32::from_rgb(125, 178, 255);
static NEXT_COMMAND: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub struct HermesApp {
    workspace: Workspace,
    persistence_error: Option<String>,
    panel_rects: BTreeMap<String, Rect>,
    dragging_panel: Option<String>,
    tab_states: BTreeMap<NodeId, TabStripState>,
    nav: &'static str,
}

impl HermesApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let workspace = Workspace::load().unwrap_or_else(|error| {
            panic!("Hermes workspace could not load its canonical DoofusDock layout: {error}")
        });
        Self {
            workspace,
            persistence_error: None,
            panel_rects: BTreeMap::new(),
            dragging_panel: None,
            tab_states: BTreeMap::new(),
            nav: "Chat",
        }
    }

    fn draw_chrome(&mut self, ui: &mut Ui) {
        egui::Frame::new()
            .fill(BG)
            .inner_margin(egui::Margin::symmetric(18, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("HERMES").size(14.0).strong().color(TEXT));
                    ui.label(RichText::new("/ DOOFUSDOCK").size(11.0).color(MUTED));
                    ui.add_space(24.0);
                    for item in ["Chat", "Sessions", "Models", "Doofus"] {
                        let selected = self.nav == item;
                        let button =
                            egui::Button::new(RichText::new(item).size(12.0).color(if selected {
                                TEXT
                            } else {
                                MUTED
                            }))
                            .fill(if selected { PANEL_ALT } else { BG })
                            .stroke(egui::Stroke::new(1.0, if selected { BORDER } else { BG }));
                        if ui.add(button).clicked() {
                            self.nav = match item {
                                "Sessions" => "Sessions",
                                "Models" => "Models",
                                "Doofus" => "Doofus",
                                _ => "Chat",
                            };
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        status_pill(ui, "Gateway offline", Color32::from_rgb(242, 176, 93));
                        ui.label(RichText::new("LOCAL PROFILE").size(10.0).color(MUTED));
                    });
                });
            });
        ui.add_space(6.0);
    }

    fn draw_status(&self, ui: &mut Ui) {
        egui::Frame::new()
            .fill(Color32::from_rgb(13, 16, 20))
            .inner_margin(egui::Margin::symmetric(14, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(Color32::from_rgb(242, 176, 93)));
                    ui.label(
                        RichText::new("Not connected to Hermes")
                            .size(11.0)
                            .color(MUTED),
                    );
                    ui.separator();
                    let snapshot = self.workspace.dock.snapshot();
                    ui.label(
                        RichText::new(format!("Layout revision {}", snapshot.revision))
                            .size(11.0)
                            .color(MUTED),
                    );
                    if let Some(error) = self
                        .persistence_error
                        .as_deref()
                        .or(self.workspace.persistence_error.as_deref())
                    {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("Layout save failed: {error}"))
                                    .size(11.0)
                                    .color(Color32::LIGHT_RED),
                            );
                        });
                    }
                });
            });
    }

    fn draw_workspace(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::Frame::new()
            .fill(BG)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let available = ui.available_rect_before_wrap();
                let bounds = Box2 {
                    x: available.left() as f64,
                    y: available.top() as f64,
                    w: available.width() as f64,
                    h: available.height() as f64,
                };
                let snapshot = match self.workspace.snapshot(bounds) {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(format!("Workspace layout error\n{error}"))
                                    .color(Color32::LIGHT_RED),
                            );
                        });
                        ui.allocate_space(available.size());
                        return;
                    }
                };
                let document = self.workspace.dock.snapshot().document;
                let mut tab_groups = Vec::new();
                if let Some(root) = document.root.as_deref() {
                    collect_tab_groups(root, &document, &snapshot, &mut tab_groups);
                }
                let mut tabbed_panels = BTreeSet::new();
                for (tabs_id, rect, selected, items) in &tab_groups {
                    if let Some((active_panel, _, _)) = items.get(*selected) {
                        tabbed_panels.insert(active_panel.0.clone());
                    }
                    self.draw_tab_group(&ctx, tabs_id, *rect, *selected, items);
                }
                let mut visible = snapshot
                    .panels
                    .iter()
                    .filter(|(_, visual)| visual.opacity >= 0.5)
                    .map(|(id, visual)| {
                        let mut rect = to_rect(visual.bounds);
                        if tabbed_panels.contains(&id.0) {
                            rect.min.y += 30.0;
                            rect.max.y -= 1.0;
                        }
                        (id.0.clone(), rect)
                    })
                    .collect::<Vec<_>>();
                visible.sort_by(|a, b| a.0.cmp(&b.0));
                self.panel_rects = visible.iter().cloned().collect();
                for (panel_id, rect) in &visible {
                    let title = document
                        .descriptor(&panel_id.clone().into())
                        .map_or(panel_id.as_str(), |descriptor| {
                            descriptor.title_key.as_str()
                        });
                    let painter = ui.painter_at(*rect);
                    painter.rect_filled(*rect, egui::CornerRadius::same(7), PANEL);
                    painter.rect_stroke(
                        *rect,
                        egui::CornerRadius::same(7),
                        egui::Stroke::new(1.0, BORDER),
                        egui::StrokeKind::Inside,
                    );
                    let title_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 34.0));
                    painter.rect_filled(
                        title_rect,
                        egui::CornerRadius {
                            nw: 7,
                            ne: 7,
                            sw: 0,
                            se: 0,
                        },
                        PANEL_ALT,
                    );
                    painter.text(
                        title_rect.left_center() + Vec2::new(13.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        title,
                        FontId::proportional(12.0),
                        TEXT,
                    );
                    painter.line_segment(
                        [title_rect.left_bottom(), title_rect.right_bottom()],
                        egui::Stroke::new(1.0, BORDER),
                    );
                }
                for (panel, rect) in visible {
                    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 34.0));
                    let drag =
                        ui.interact(header, Id::unique(("drag", panel.as_str())), Sense::drag());
                    if drag.drag_started() {
                        self.dragging_panel = Some(panel.clone());
                    }
                    let title = document
                        .descriptor(&panel.clone().into())
                        .map_or(panel.as_str(), |descriptor| descriptor.title_key.as_str());
                    paint_panel_content(&ui.painter_at(rect), &panel, title, self.nav, rect);
                }
                if let Some(source) = self.dragging_panel.clone()
                    && ctx.input(|input| input.pointer.any_released())
                {
                    let drop = ctx
                        .input(|input| input.pointer.interact_pos())
                        .and_then(|pos| {
                            self.panel_rects
                                .iter()
                                .find(|(id, rect)| **id != source && rect.contains(pos))
                                .map(|(id, rect)| (id.clone(), *rect, pos))
                        });
                    if let Some((target, rect, pos)) = drop {
                        self.move_panel(&document, &source, &target, rect, pos);
                    }
                    self.dragging_panel = None;
                }
            });
    }

    fn draw_tab_group(
        &mut self,
        ctx: &egui::Context,
        tabs_id: &NodeId,
        bounds: Box2,
        selected: usize,
        items: &[(PanelId, String, usize)],
    ) {
        let rect = to_rect(bounds);
        let strip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 30.0));
        egui::Area::new(Id::unique(("tab-strip", tabs_id.0.as_str())))
            .order(Order::Foreground)
            .fixed_pos(strip.min)
            .show(ctx, |ui| {
                let item_specs = items
                    .iter()
                    .map(|(_, title, _)| TabStripItem {
                        title,
                        accessible_label: title,
                        icon: None,
                        modified: false,
                        pinned: false,
                        closable: false,
                        multi_selected: false,
                    })
                    .collect::<Vec<_>>();
                ui.set_min_size(strip.size());
                ui.set_max_size(strip.size());
                let state = self.tab_states.entry(tabs_id.clone()).or_default();
                let style = doofusdock_render_egui::chrome::PanelChromeStyle {
                    titlebar: PANEL_ALT,
                    title: TEXT,
                    control: PANEL,
                    control_hover: BORDER,
                    control_text: TEXT,
                    control_border: BORDER,
                    focus: ACCENT,
                    corner_radius: 6.0,
                    font: FontId::proportional(11.0),
                };
                let response = tab_strip(
                    ui,
                    Id::unique(("tabs", tabs_id.0.as_str())),
                    strip,
                    &item_specs,
                    selected,
                    state,
                    &style,
                );
                if let Some(index) = response.selected {
                    self.apply_layout_operation(
                        Operation::SelectTab {
                            tabs: tabs_id.clone(),
                            index,
                        },
                        "select_tab",
                    );
                }
            });
    }

    fn move_panel(
        &mut self,
        document: &LayoutDocument,
        source: &str,
        target: &str,
        rect: Rect,
        pos: Pos2,
    ) {
        let (operation, command) =
            if let Some(target_panel) = find_panel_node(document.root.as_deref(), target) {
                let dx = (pos.x - rect.left()) / rect.width().max(1.0);
                let dy = (pos.y - rect.top()) / rect.height().max(1.0);
                let edge = 0.22;
                let operation = if dx < edge || dx > 1.0 - edge || dy < edge || dy > 1.0 - edge {
                    let (orientation, before) = if dx < edge || dx > 1.0 - edge {
                        (Orientation::Horizontal, dx < 0.5)
                    } else {
                        (Orientation::Vertical, dy < 0.5)
                    };
                    Operation::SplitPanel {
                        panel: source.into(),
                        target: target_panel,
                        orientation,
                        before,
                        ratio: 0.30,
                    }
                } else {
                    Operation::CenterPanel {
                        panel: source.into(),
                        target_panel: target.into(),
                    }
                };
                (operation, "move_panel")
            } else {
                return;
            };
        self.apply_layout_operation(operation, command);
    }

    fn apply_layout_operation(&mut self, operation: Operation, label: &str) {
        let snapshot = self.workspace.dock.snapshot();
        let sequence = NEXT_COMMAND.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let command = CommandId::from(format!("hermes-ui-{label}-{sequence}"));
        match self
            .workspace
            .dock
            .transact(snapshot.revision, command, [operation])
        {
            Ok(_) => {
                self.workspace.save();
                self.persistence_error = self.workspace.persistence_error.clone();
            }
            Err(error) => self.persistence_error = Some(error.to_string()),
        }
    }
}

impl eframe::App for HermesApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        ui.visuals_mut().override_text_color = Some(TEXT);
        ui.painter().rect_filled(ui.max_rect(), 0.0, BG);
        egui::Panel::top("hermes-chrome")
            .frame(egui::Frame::new().fill(BG))
            .show(ui, |ui| self.draw_chrome(ui));
        egui::Panel::bottom("hermes-status")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(BG))
            .show(ui, |ui| self.draw_status(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show(ui, |ui| self.draw_workspace(ui));
    }
}

fn to_rect(bounds: Box2) -> Rect {
    Rect::from_min_size(
        Pos2::new(bounds.x as f32, bounds.y as f32),
        Vec2::new(bounds.w as f32, bounds.h as f32),
    )
}

fn collect_tab_groups(
    node: &Node,
    document: &LayoutDocument,
    snapshot: &Snapshot,
    output: &mut Vec<(NodeId, Box2, usize, Vec<(PanelId, String, usize)>)>,
) {
    match &node.kind {
        NodeKind::Tabs {
            items,
            active_index,
            ..
        } => {
            if let Some(bounds) = snapshot.nodes.get(&node.id) {
                let tabs = items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        let panel = first_panel(item)?;
                        let title = document
                            .descriptor(&panel)
                            .map_or_else(|| panel.0.clone(), |desc| desc.title_key.clone());
                        Some((panel, title, index))
                    })
                    .collect::<Vec<_>>();
                if !tabs.is_empty() {
                    output.push((node.id.clone(), *bounds, *active_index, tabs));
                }
            }
            for child in items {
                collect_tab_groups(child, document, snapshot, output);
            }
        }
        NodeKind::Split { children, .. }
        | NodeKind::DocumentWell {
            groups: children, ..
        }
        | NodeKind::Extension { children, .. } => {
            for child in children {
                collect_tab_groups(child, document, snapshot, output);
            }
        }
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => {}
    }
}

fn first_panel(node: &Node) -> Option<PanelId> {
    match &node.kind {
        NodeKind::Panel { panel } => Some(panel.clone()),
        NodeKind::Tabs {
            items,
            active_index,
            ..
        } => items
            .get(*active_index)
            .and_then(|child| first_panel(child)),
        NodeKind::DocumentWell {
            groups,
            active_group,
        } => groups
            .get(*active_group)
            .and_then(|child| first_panel(child)),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.first().and_then(|child| first_panel(child))
        }
        NodeKind::Placeholder { panel, .. } => Some(panel.clone()),
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

fn paint_panel_content(
    painter: &egui::Painter,
    id: &str,
    _panel_title: &str,
    nav: &str,
    rect: Rect,
) {
    let left = rect.left() + 14.0;
    let top = rect.top() + 48.0;
    let width = (rect.width() - 28.0).max(80.0);
    match id {
        PANEL_EXPLORER => {
            painter.text(
                Pos2::new(left, top),
                egui::Align2::LEFT_TOP,
                "WORKSPACE",
                FontId::proportional(10.0),
                MUTED,
            );
            paint_wrapped(
                painter,
                "Connect a Hermes gateway to browse projects and sessions.",
                Pos2::new(left, top + 28.0),
                width,
                12.0,
                TEXT,
            );
            let pill = Rect::from_min_size(Pos2::new(left, top + 78.0), Vec2::new(126.0, 24.0));
            painter.rect_filled(
                pill,
                egui::CornerRadius::same(9),
                Color32::from_rgba_unmultiplied(MUTED.r(), MUTED.g(), MUTED.b(), 28),
            );
            painter.rect_stroke(
                pill,
                egui::CornerRadius::same(9),
                egui::Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(MUTED.r(), MUTED.g(), MUTED.b(), 90),
                ),
                egui::StrokeKind::Inside,
            );
            painter.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                "Waiting for connection",
                FontId::proportional(10.0),
                MUTED,
            );
        }
        PANEL_CHAT => {
            let (heading, detail) = match nav {
                "Sessions" => (
                    "Sessions",
                    "Session history will load from the Hermes gateway after the native client connects.",
                ),
                "Models" => (
                    "Models & quota",
                    "Provider catalogs, quotas, and model switching will come from the Hermes gateway.",
                ),
                "Doofus" => (
                    "Doofus Control",
                    "Tasks, evidence, activity, and vault metadata will come from the authenticated Doofus bridge.",
                ),
                _ => (
                    "Start with Hermes",
                    "Conversations and the composer will be available after the native gateway client connects.",
                ),
            };
            let center = Pos2::new(rect.center().x, rect.top() + rect.height() * 0.36);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                heading,
                FontId::proportional(25.0),
                TEXT,
            );
            paint_wrapped_centered(
                painter,
                detail,
                Pos2::new(center.x, center.y + 34.0),
                (rect.width() - 80.0).max(120.0),
                13.0,
                MUTED,
            );
            painter.text(
                Pos2::new(center.x, center.y + 75.0),
                egui::Align2::CENTER_CENTER,
                "Workspace geometry is owned by the saved DoofusDock layout.",
                FontId::proportional(11.0),
                ACCENT,
            );
        }
        PANEL_INSPECTOR => {
            painter.text(
                Pos2::new(left, top),
                egui::Align2::LEFT_TOP,
                "INSPECTOR",
                FontId::proportional(10.0),
                MUTED,
            );
            paint_wrapped(
                painter,
                "No preview or Doofus task is selected.",
                Pos2::new(left, top + 28.0),
                width,
                12.0,
                TEXT,
            );
            paint_wrapped(
                painter,
                "Vault data and task evidence will be read from the authenticated Doofus bridge.",
                Pos2::new(left, top + 54.0),
                width,
                11.0,
                MUTED,
            );
        }
        PANEL_TERMINAL => {
            painter.text(
                Pos2::new(left, top),
                egui::Align2::LEFT_TOP,
                "TERMINAL · TESTS · LOGS",
                FontId::proportional(10.0),
                MUTED,
            );
            painter.text(
                Pos2::new(left, top + 28.0),
                egui::Align2::LEFT_TOP,
                "No terminal process is attached.",
                FontId::proportional(12.0),
                TEXT,
            );
        }
        _ => {}
    }
}

fn paint_wrapped(
    painter: &egui::Painter,
    text: &str,
    position: Pos2,
    width: f32,
    size: f32,
    color: Color32,
) {
    let galley = painter.layout(text.to_owned(), FontId::proportional(size), color, width);
    painter.galley(position, galley, color);
}

fn paint_wrapped_centered(
    painter: &egui::Painter,
    text: &str,
    center: Pos2,
    width: f32,
    size: f32,
    color: Color32,
) {
    let galley = painter.layout(text.to_owned(), FontId::proportional(size), color, width);
    painter.galley(center - galley.size() * 0.5, galley, color);
}

fn status_pill(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            28,
        ))
        .stroke(egui::Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90),
        ))
        .corner_radius(egui::CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(10.0).color(color));
        });
}
