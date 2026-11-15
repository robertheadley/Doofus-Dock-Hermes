//! Reusable themed panel title-bar primitives for egui hosts.
use egui::{
    Align2, Color32, FontId, Id, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
};

#[derive(Clone, Debug)]
pub struct PanelChromeStyle {
    pub titlebar: Color32,
    pub title: Color32,
    pub control: Color32,
    pub control_hover: Color32,
    pub control_text: Color32,
    pub control_border: Color32,
    pub focus: Color32,
    pub corner_radius: f32,
    pub font: FontId,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelChromeControl {
    pub glyph: &'static str,
    pub accessible_label: &'static str,
    pub tooltip: &'static str,
    pub enabled: bool,
}

pub struct PanelTitleBarResponse {
    /// Drag response covers only the title area, never the controls.
    pub drag: Response,
    pub activated_controls: Vec<usize>,
}

/// Paint a themed title bar and return semantic control activations to the
/// host. This widget never mutates canonical layout state.
pub fn panel_title_bar(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    title: &str,
    style: &PanelChromeStyle,
    controls: &[PanelChromeControl],
) -> PanelTitleBarResponse {
    ui.painter()
        .rect_filled(rect, style.corner_radius, style.titlebar);

    let gap = 3.0;
    let gaps_width = controls.len().saturating_sub(1) as f32 * gap;
    let available_button_width = if controls.is_empty() {
        0.0
    } else {
        (rect.width() - gaps_width - 6.0).max(0.0) / controls.len() as f32
    };
    let button_width = rect.height().clamp(16.0, 24.0).min(available_button_width);
    let controls_width = controls.len() as f32 * button_width + gaps_width + 6.0;
    let drag_end_x = (rect.max.x - controls_width).max(rect.min.x);
    let drag_rect = Rect::from_min_max(rect.min, egui::pos2(drag_end_x, rect.max.y));
    let drag = ui.interact(drag_rect, id.with("drag"), Sense::click_and_drag());
    let title_rect = Rect::from_min_max(
        egui::pos2((rect.min.x + 8.0).min(drag_end_x), rect.min.y),
        egui::pos2(drag_end_x, rect.max.y),
    );
    ui.painter()
        .with_clip_rect(title_rect.intersect(ui.clip_rect()))
        .text(
            title_rect.left_center(),
            Align2::LEFT_CENTER,
            title,
            style.font.clone(),
            style.title,
        );

    let mut activated_controls = Vec::new();
    let mut right = rect.max.x - 3.0;
    for (index, control) in controls.iter().enumerate().rev() {
        let left = right - button_width;
        let button_rect = Rect::from_min_max(
            egui::pos2(
                left,
                rect.min.y + (rect.height() - button_width).max(0.0) * 0.5,
            ),
            egui::pos2(right, rect.min.y + (rect.height() + button_width) * 0.5),
        );
        let sense = if control.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(button_rect, id.with(("control", index)), sense);
        response.widget_info(|| {
            WidgetInfo::labeled(Role::Button, control.enabled, control.accessible_label)
        });
        if response.hovered() && control.enabled {
            ui.painter()
                .rect_filled(button_rect, 2.0, style.control_hover);
        } else {
            ui.painter().rect_filled(button_rect, 2.0, style.control);
        }
        ui.painter().rect_stroke(
            button_rect,
            2.0,
            Stroke::new(1.0, style.control_border),
            StrokeKind::Inside,
        );
        ui.painter().text(
            button_rect.center(),
            Align2::CENTER_CENTER,
            control.glyph,
            style.font.clone(),
            if response.has_focus() {
                style.focus
            } else {
                style.control_text
            },
        );
        let clicked = response.clicked();
        if response.hovered() {
            response.on_hover_text(control.tooltip);
        }
        if clicked && control.enabled {
            activated_controls.push(index);
        }
        right = left - gap;
    }

    PanelTitleBarResponse {
        drag,
        activated_controls,
    }
}

/// Host-neutral intentions; the application routes these through its canonical
/// command service and supplies policy-derived enablement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelIntent {
    Add,
    Lock,
    Unlock,
    Pin,
    Unpin,
    Collapse,
    Expand,
    Dock,
    Float,
    Maximize,
    Restore,
    Close,
    Overflow,
}
#[derive(Clone, Copy, Debug)]
pub struct PanelShellAction {
    pub intent: PanelIntent,
    pub enabled: bool,
}
pub struct PanelShell<'a> {
    pub title: &'a str,
    pub icon: Option<&'a str>,
    pub status: Option<&'a str>,
    pub focused: bool,
    pub actions: &'a [PanelShellAction],
}
pub struct PanelShellResponse {
    pub drag: Response,
    pub intentions: Vec<PanelIntent>,
}

/// One host-neutral action exposed from a reusable panel or group menu.
#[derive(Clone, Copy, Debug)]
pub struct PanelMenuAction {
    pub intent: PanelIntent,
    pub label: &'static str,
    pub enabled: bool,
}

fn enabled_menu_intent(action: PanelMenuAction) -> Option<PanelIntent> {
    action.enabled.then_some(action.intent)
}

/// Paint a reusable action menu and return the selected semantic intention.
/// The host remains responsible for deriving enablement from policy and for
/// routing the returned intention through its canonical command service.
pub fn panel_action_menu(
    ui: &mut Ui,
    id: Id,
    trigger_label: &str,
    actions: &[PanelMenuAction],
) -> Option<PanelIntent> {
    ui.push_id(id, |ui| {
        ui.menu_button(trigger_label, |ui| {
            for action in actions {
                if ui
                    .add_enabled(action.enabled, egui::Button::new(action.label))
                    .clicked()
                {
                    let intent = enabled_menu_intent(*action);
                    ui.close();
                    return intent;
                }
            }
            None
        })
        .inner
    })
    .inner
    .flatten()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
pub struct ResizeHandleStyle {
    pub idle: Color32,
    pub hovered: Color32,
    pub dragged: Color32,
}

/// Paint a reusable split resize affordance and return its drag response.
/// The host owns the geometry update and canonical operation commit.
pub fn resize_handle(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    axis: ResizeAxis,
    enabled: bool,
    accessible_label: &str,
    style: ResizeHandleStyle,
) -> Response {
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, id, sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, accessible_label));
    if enabled && (response.hovered() || response.dragged()) {
        ui.output_mut(|output| {
            output.cursor_icon = match axis {
                ResizeAxis::Horizontal => egui::CursorIcon::ResizeHorizontal,
                ResizeAxis::Vertical => egui::CursorIcon::ResizeVertical,
            };
        });
    }
    let color = if response.dragged() {
        style.dragged
    } else if response.hovered() {
        style.hovered
    } else {
        style.idle
    };
    ui.painter().rect_filled(rect, 0.0, color);
    response
}

pub fn panel_shell(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    shell: PanelShell<'_>,
    style: &PanelChromeStyle,
) -> PanelShellResponse {
    let controls: Vec<_> = shell
        .actions
        .iter()
        .map(|action| {
            let (glyph, label) = match action.intent {
                PanelIntent::Add => ("+", "Add panel"),
                PanelIntent::Lock => ("L", "Lock group"),
                PanelIntent::Unlock => ("U", "Unlock group"),
                PanelIntent::Pin => ("P", "Pin panel"),
                PanelIntent::Unpin => ("P−", "Unpin panel"),
                PanelIntent::Collapse => ("−", "Collapse"),
                PanelIntent::Expand => ("+", "Expand"),
                PanelIntent::Dock => ("D", "Dock back"),
                PanelIntent::Float => ("F", "Float window"),
                PanelIntent::Maximize => ("M", "Maximize"),
                PanelIntent::Restore => ("R", "Restore"),
                PanelIntent::Close => ("×", "Close"),
                PanelIntent::Overflow => ("…", "More actions"),
            };
            PanelChromeControl {
                glyph,
                accessible_label: label,
                tooltip: label,
                enabled: action.enabled,
            }
        })
        .collect();
    let title = format!(
        "{}{}{}{}{}",
        shell.icon.unwrap_or(""),
        if shell.icon.is_some() { " " } else { "" },
        shell.title,
        if shell.status.is_some() { " · " } else { "" },
        shell.status.unwrap_or("")
    );
    let response = panel_title_bar(ui, id, rect, &title, style, &controls);
    response
        .drag
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &title));
    if shell.focused {
        ui.painter().rect_stroke(
            rect,
            style.corner_radius,
            Stroke::new(2.0, style.focus),
            StrokeKind::Inside,
        );
    }
    PanelShellResponse {
        drag: response.drag,
        intentions: response
            .activated_controls
            .into_iter()
            .map(|i| shell.actions[i].intent)
            .collect(),
    }
}

/// Stable, host-neutral choices exposed by the reusable group modality menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupModalityIntent {
    SplitHorizontal,
    AccordionSingle,
    AccordionMultiple,
    ShadeStack,
    Grid,
    Spatial,
    NodeGraph,
    Layered,
    MasterStack,
    BinarySpacePartition,
    Spiral,
    Dwindle,
    Monocle,
    EqualColumns,
    EqualRows,
    AdaptiveGrid,
    ScrollableColumns,
}

/// Render the shared group modality menu and return a semantic choice. The
/// host remains responsible for policy, capability negotiation, and commit.
pub fn group_modality_menu(
    ui: &mut Ui,
    id: Id,
    enabled: bool,
    extra_content: impl FnOnce(&mut Ui),
) -> Option<GroupModalityIntent> {
    ui.push_id(id, |ui| {
        ui.menu_button("…", |ui| {
            extra_content(ui);
            ui.separator();
            for (label, intent) in [
                ("Split horizontally", GroupModalityIntent::SplitHorizontal),
                (
                    "Accordion (single expansion)",
                    GroupModalityIntent::AccordionSingle,
                ),
                (
                    "Accordion (multiple expansion)",
                    GroupModalityIntent::AccordionMultiple,
                ),
                ("Shade stack", GroupModalityIntent::ShadeStack),
                ("Grid", GroupModalityIntent::Grid),
                ("Spatial", GroupModalityIntent::Spatial),
                ("Node graph", GroupModalityIntent::NodeGraph),
                ("Layered stack", GroupModalityIntent::Layered),
            ] {
                if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                    ui.close();
                    return Some(intent);
                }
            }
            if let Some(Some(intent)) = ui
                .menu_button("Additional tiling strategies", |ui| {
                    for (label, intent) in [
                        ("Master/Stack", GroupModalityIntent::MasterStack),
                        (
                            "Binary space partition",
                            GroupModalityIntent::BinarySpacePartition,
                        ),
                        ("Spiral", GroupModalityIntent::Spiral),
                        ("Dwindle", GroupModalityIntent::Dwindle),
                        ("Monocle", GroupModalityIntent::Monocle),
                        ("Equal columns", GroupModalityIntent::EqualColumns),
                        ("Equal rows", GroupModalityIntent::EqualRows),
                        ("Adaptive grid", GroupModalityIntent::AdaptiveGrid),
                        ("Scrollable columns", GroupModalityIntent::ScrollableColumns),
                    ] {
                        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                            ui.close();
                            return Some(intent);
                        }
                    }
                    None
                })
                .inner
            {
                ui.close();
                return Some(intent);
            }
            None
        })
        .inner
    })
    .inner
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Vec2;

    fn style() -> PanelChromeStyle {
        PanelChromeStyle {
            titlebar: Color32::DARK_GRAY,
            title: Color32::WHITE,
            control: Color32::GRAY,
            control_hover: Color32::LIGHT_BLUE,
            control_text: Color32::WHITE,
            control_border: Color32::BLACK,
            focus: Color32::YELLOW,
            corner_radius: 3.0,
            font: FontId::proportional(14.0),
        }
    }

    #[test]
    fn narrow_titlebar_keeps_controls_inside_and_separates_drag_area() {
        let context = egui::Context::default();
        let mut response = None;
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut Ui| {
            let rect = Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::new(40.0, 22.0));
            response = Some(panel_title_bar(
                ui,
                ui.scope_id().with("test-shell"),
                rect,
                "Editor",
                &style(),
                &[PanelChromeControl {
                    glyph: "×",
                    accessible_label: "Close Editor",
                    tooltip: "Close panel",
                    enabled: false,
                }],
            ));
        });
        output.textures_delta.clear();
        let response = response.unwrap();
        assert!(response.drag.rect.width() <= 40.0);
        assert!(response.activated_controls.is_empty());
    }

    #[test]
    fn enabled_shell_control_returns_a_semantic_activation() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::new(80.0, 24.0));
        let control_position = egui::pos2(72.0, 22.0);
        let mut activated = Vec::new();
        let events = [
            vec![egui::Event::PointerMoved(control_position)],
            vec![
                egui::Event::PointerMoved(control_position),
                egui::Event::PointerButton {
                    pos: control_position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec![
                egui::Event::PointerMoved(control_position),
                egui::Event::PointerButton {
                    pos: control_position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        ];
        for events in events {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(120.0, 80.0),
                )),
                events,
                ..Default::default()
            };
            let mut response = None;
            let mut output = context.run_ui(input, &mut |ui: &mut Ui| {
                response = Some(panel_title_bar(
                    ui,
                    ui.scope_id().with("click-shell"),
                    rect,
                    "Inspector",
                    &style(),
                    &[PanelChromeControl {
                        glyph: "×",
                        accessible_label: "Close Inspector",
                        tooltip: "Close panel",
                        enabled: true,
                    }],
                ));
            });
            output.textures_delta.clear();
            activated = response.unwrap().activated_controls;
        }
        assert_eq!(activated, vec![0]);
    }

    #[test]
    fn resize_handle_reports_drag_and_respects_disabled_state() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::new(8.0, 64.0));
        let style = ResizeHandleStyle {
            idle: Color32::GRAY,
            hovered: Color32::LIGHT_BLUE,
            dragged: Color32::YELLOW,
        };
        for enabled in [true, false] {
            let mut drag_started = false;
            for events in [
                vec![egui::Event::PointerMoved(egui::pos2(24.0, 30.0))],
                vec![
                    egui::Event::PointerMoved(egui::pos2(24.0, 30.0)),
                    egui::Event::PointerButton {
                        pos: egui::pos2(24.0, 30.0),
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                vec![
                    egui::Event::PointerMoved(egui::pos2(34.0, 30.0)),
                    egui::Event::PointerButton {
                        pos: egui::pos2(34.0, 30.0),
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            ] {
                let mut response = None;
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(120.0, 100.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    &mut |ui: &mut Ui| {
                        response = Some(resize_handle(
                            ui,
                            ui.scope_id().with("resize-handle-contract"),
                            rect,
                            ResizeAxis::Horizontal,
                            enabled,
                            "Resize split divider",
                            style,
                        ));
                    },
                );
                output.textures_delta.clear();
                drag_started |= response.unwrap().drag_started();
            }
            assert_eq!(drag_started, enabled);
        }
    }

    #[test]
    fn action_menu_is_inert_until_an_action_is_selected() {
        let context = egui::Context::default();
        let actions = [
            PanelMenuAction {
                intent: PanelIntent::Pin,
                label: "Pin panel",
                enabled: true,
            },
            PanelMenuAction {
                intent: PanelIntent::Close,
                label: "Close panel",
                enabled: false,
            },
        ];
        let mut selected = Some(PanelIntent::Overflow);
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(320.0, 180.0),
                )),
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                selected = panel_action_menu(
                    ui,
                    ui.scope_id().with("panel-actions-contract"),
                    "More actions",
                    &actions,
                );
            },
        );
        output.textures_delta.clear();
        assert_eq!(selected, None);
        assert_eq!(enabled_menu_intent(actions[0]), Some(PanelIntent::Pin));
        assert_eq!(enabled_menu_intent(actions[1]), None);
    }

    #[test]
    fn group_modality_menu_returns_no_choice_until_a_host_selects_one() {
        let context = egui::Context::default();
        let mut selected = Some(GroupModalityIntent::Grid);
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(320.0, 180.0),
                )),
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                selected = group_modality_menu(
                    ui,
                    ui.scope_id().with("group-menu-contract"),
                    true,
                    |ui| {
                        ui.label("Host supplied search");
                    },
                );
            },
        );
        output.textures_delta.clear();
        assert_eq!(selected, None);
    }

    #[test]
    fn reusable_shell_returns_only_enabled_host_intentions() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(160.0, 26.0));
        let pointer = egui::pos2(120.0, 13.0);
        let mut activated = Vec::new();
        let actions = [
            PanelShellAction {
                intent: PanelIntent::Pin,
                enabled: true,
            },
            PanelShellAction {
                intent: PanelIntent::Close,
                enabled: false,
            },
        ];
        for events in [
            vec![egui::Event::PointerMoved(pointer)],
            vec![egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let mut response = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(180.0, 50.0),
                    )),
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    response = Some(panel_shell(
                        ui,
                        ui.scope_id().with("semantic-shell"),
                        rect,
                        PanelShell {
                            title: "Panel",
                            icon: Some("P"),
                            status: Some("Modified"),
                            focused: true,
                            actions: &actions,
                        },
                        &style(),
                    ));
                },
            );
            output.textures_delta.clear();
            activated = response.unwrap().intentions;
        }
        assert_eq!(activated, vec![PanelIntent::Pin]);
    }
}
