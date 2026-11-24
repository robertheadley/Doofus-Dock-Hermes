//! Reusable scrollable egui tab strip with host-owned canonical actions.
use egui::{Align2, Id, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo};

use crate::chrome::PanelChromeStyle;

#[derive(Clone, Copy, Debug)]
pub struct TabStripItem<'a> {
    pub title: &'a str,
    pub accessible_label: &'a str,
    pub icon: Option<&'a str>,
    pub modified: bool,
    pub pinned: bool,
    pub closable: bool,
    /// Included in the host's transient multi-selection presentation.
    pub multi_selected: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabStripState {
    pub first_visible: usize,
    /// Transient search query. Hosts may save this UI preference separately.
    pub query: String,
    last_query: String,
    pub last_selected: Option<usize>,
    /// Canonical item index to refocus after a host-committed reorder.
    pub focus_request: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct TabStripResponse {
    pub selected: Option<usize>,
    /// Ctrl-click toggles this canonical tab index without changing the active tab.
    pub multi_toggled: Option<usize>,
    pub pin_toggle: Option<usize>,
    pub close: Option<usize>,
    pub drag_started: Option<usize>,
    /// `(canonical_item_index, insertion_boundary)` from Ctrl+Shift+PageUp/Down.
    pub reorder: Option<(usize, usize)>,
    pub focused: Option<usize>,
    pub hovered: Option<(usize, Rect)>,
    /// Canonical item indices and clipped-to-strip tab rectangles drawn this frame.
    pub visible_bounds: Vec<(usize, Rect)>,
}

/// Paint a bounded, keyboard-navigable tab strip. Overflow is exposed through
/// previous/next controls; only visible tabs receive widgets each frame.
pub fn tab_strip(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    items: &[TabStripItem<'_>],
    selected_index: usize,
    state: &mut TabStripState,
    style: &PanelChromeStyle,
) -> TabStripResponse {
    let mut pin_toggle = None;
    let mut context_menu = |ui: &mut Ui, index: usize, item: TabStripItem<'_>| {
        if ui
            .button(if item.pinned { "Unpin tab" } else { "Pin tab" })
            .clicked()
        {
            pin_toggle = Some(index);
            ui.close();
        }
    };
    let mut response = tab_strip_with_context_menu(
        ui,
        id,
        rect,
        items,
        selected_index,
        state,
        style,
        &mut context_menu,
    );
    response.pin_toggle = pin_toggle;
    response
}

/// Paint a tab strip while letting the host provide additional context actions.
/// The callback receives the canonical item index and the item's display data.
#[allow(clippy::too_many_arguments)]
pub fn tab_strip_with_context_menu(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    items: &[TabStripItem<'_>],
    selected_index: usize,
    state: &mut TabStripState,
    style: &PanelChromeStyle,
    mut context_menu: impl FnMut(&mut Ui, usize, TabStripItem<'_>),
) -> TabStripResponse {
    ui.painter()
        .rect_filled(rect, style.corner_radius, style.control);
    let mut output = TabStripResponse::default();
    let order = if state.query.trim().is_empty() && !items.iter().any(|item| item.pinned) {
        TabOrder::Canonical(items.len())
    } else {
        TabOrder::Filtered(tab_display_order(items, &state.query))
    };
    if state.last_query != state.query {
        state.first_visible = 0;
        state.last_selected = None;
        state.last_query.clone_from(&state.query);
    }
    if order.is_empty() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        state.first_visible = 0;
        state.last_selected = None;
        return output;
    }

    let tab_min_width = 88.0_f32;
    let nav_width = 22.0_f32;
    let initial_capacity = ((rect.width() / tab_min_width).floor() as usize).max(1);
    let overflow = order.len() > initial_capacity;
    let content_left = if overflow { nav_width } else { 0.0 };
    let content_right = if overflow { nav_width } else { 0.0 };
    let content_width = (rect.width() - content_left - content_right).max(0.0);
    let tab_width = if overflow {
        (content_width / initial_capacity as f32).max(1.0)
    } else {
        (content_width / order.len() as f32).max(1.0)
    };
    let visible_count = ((content_width / tab_width).floor() as usize)
        .max(1)
        .min(order.len());
    let max_first = order.len().saturating_sub(visible_count);
    state.first_visible = state.first_visible.min(max_first);
    if state.last_selected != Some(selected_index) {
        let selected_index = order.position(selected_index).unwrap_or(0);
        if selected_index < state.first_visible {
            state.first_visible = selected_index;
        } else if selected_index >= state.first_visible.saturating_add(visible_count) {
            state.first_visible = selected_index
                .saturating_add(1)
                .saturating_sub(visible_count);
        }
    }
    state.last_selected = Some(selected_index);
    state.first_visible = state.first_visible.min(max_first);

    if overflow {
        let prev_rect =
            Rect::from_min_max(rect.min, egui::pos2(rect.min.x + nav_width, rect.max.y));
        let next_rect =
            Rect::from_min_max(egui::pos2(rect.max.x - nav_width, rect.min.y), rect.max);
        let previous = ui.interact(prev_rect, id.with("previous"), Sense::click());
        let next = ui.interact(next_rect, id.with("next"), Sense::click());
        paint_nav(ui, prev_rect, "‹", previous.hovered(), style);
        paint_nav(ui, next_rect, "›", next.hovered(), style);
        previous.widget_info(|| WidgetInfo::labeled(Role::Button, true, "Scroll tabs backward"));
        next.widget_info(|| WidgetInfo::labeled(Role::Button, true, "Scroll tabs forward"));
        if previous.clicked() {
            state.first_visible = state.first_visible.saturating_sub(visible_count);
        }
        if next.clicked() {
            state.first_visible = state
                .first_visible
                .saturating_add(visible_count)
                .min(max_first);
        }
    }

    for slot in state.first_visible..state.first_visible.saturating_add(visible_count) {
        let index = order.at(slot);
        let item = items[index];
        let left = rect.min.x + content_left + (slot - state.first_visible) as f32 * tab_width;
        let tab_rect = Rect::from_min_max(
            egui::pos2(left, rect.min.y + 2.0),
            egui::pos2(
                (left + tab_width - 2.0).min(rect.max.x - content_right),
                rect.max.y - 2.0,
            ),
        );
        output.visible_bounds.push((index, tab_rect));
        let selected = index == selected_index;
        let response = ui.interact(tab_rect, id.with(("tab", index)), Sense::click_and_drag());
        response.widget_info(|| {
            WidgetInfo::selected(
                Role::Tab,
                true,
                selected,
                format!(
                    "{}{}{}{}",
                    item.accessible_label,
                    if item.pinned { ", pinned" } else { "" },
                    if item.modified { ", modified" } else { "" },
                    if item.multi_selected {
                        ", multi-selected"
                    } else {
                        ""
                    }
                ),
            )
        });
        response.context_menu(|ui| context_menu(ui, index, item));
        ui.painter().rect_filled(
            tab_rect,
            style.corner_radius,
            if selected || item.multi_selected {
                style.control_hover
            } else if response.hovered() {
                style.focus
            } else {
                style.control
            },
        );
        if selected || item.multi_selected {
            ui.painter().rect_stroke(
                tab_rect,
                style.corner_radius,
                Stroke::new(1.0, style.control_border),
                StrokeKind::Inside,
            );
        }

        let close_width = if item.closable { 18.0 } else { 0.0 };
        let label_left = tab_rect.min.x + 6.0;
        if let Some(icon) = item.icon {
            ui.painter().text(
                egui::pos2(label_left, tab_rect.center().y),
                Align2::LEFT_CENTER,
                icon,
                style.font.clone(),
                style.title,
            );
        }
        let icon_width = item.icon.map_or(0.0, |_| 16.0);
        let marker_width = if item.modified { 9.0 } else { 0.0 };
        let text_rect = Rect::from_min_max(
            egui::pos2(label_left + icon_width, tab_rect.min.y),
            egui::pos2(
                (tab_rect.max.x - close_width - marker_width - 4.0).max(label_left),
                tab_rect.max.y,
            ),
        );
        ui.painter().with_clip_rect(text_rect).text(
            text_rect.left_center(),
            Align2::LEFT_CENTER,
            item.title,
            style.font.clone(),
            style.title,
        );
        if item.pinned {
            ui.painter().line_segment(
                [tab_rect.left_bottom(), tab_rect.right_bottom()],
                Stroke::new(2.0, style.focus),
            );
        }
        if item.modified {
            ui.painter().circle_filled(
                egui::pos2(tab_rect.max.x - close_width - 7.0, tab_rect.center().y),
                2.5,
                style.focus,
            );
        }
        if response.clicked() {
            let multi_toggle = ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(event, egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                        ..
                    } if modifiers.ctrl)
                })
            });
            if multi_toggle {
                output.multi_toggled = Some(index);
            } else {
                output.selected = Some(index);
                ui.memory_mut(|memory| memory.request_focus(response.id));
            }
        }
        if response.drag_started() {
            output.drag_started = Some(index);
        }
        if response.hovered() {
            output.hovered = Some((index, tab_rect));
        }
        if state.focus_request == Some(index) {
            ui.memory_mut(|memory| memory.request_focus(response.id));
        }
        if response.has_focus() {
            output.focused = Some(index);
            if state.focus_request == Some(index) {
                state.focus_request = None;
            }
            let reorder_direction = ui.input_mut(|input| {
                let modifiers = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
                if input.consume_key(modifiers, egui::Key::PageUp) {
                    Some(-1_i8)
                } else if input.consume_key(modifiers, egui::Key::PageDown) {
                    Some(1_i8)
                } else {
                    None
                }
            });
            if state.query.trim().is_empty()
                && !items.iter().any(|candidate| candidate.pinned)
                && let Some(direction) = reorder_direction
            {
                let insertion = match direction {
                    -1 if index > 0 => Some(index - 1),
                    1 if index + 1 < items.len() => Some(index + 2),
                    _ => None,
                };
                if let Some(insertion) = insertion {
                    output.reorder = Some((index, insertion));
                }
            }
            let direction = ui.input(|input| {
                if input.key_pressed(egui::Key::ArrowRight) {
                    1_i8
                } else if input.key_pressed(egui::Key::ArrowLeft) {
                    -1_i8
                } else {
                    0_i8
                }
            });
            if direction != 0 {
                let target = if direction > 0 {
                    order.at((slot + 1).min(order.len() - 1))
                } else {
                    order.at(slot.saturating_sub(1))
                };
                output.selected = Some(target);
                ui.memory_mut(|memory| memory.request_focus(id.with(("tab", target))));
            }
        }

        if item.closable {
            let close_rect = Rect::from_min_max(
                egui::pos2(tab_rect.max.x - 18.0, tab_rect.min.y),
                tab_rect.max,
            );
            let close = ui.interact(close_rect, id.with(("close", index)), Sense::click());
            let close_target = if item.accessible_label.trim().is_empty() {
                item.title.trim()
            } else {
                item.accessible_label.trim()
            };
            close.widget_info(|| {
                if close_target.is_empty() {
                    WidgetInfo::labeled(Role::Button, true, format_args!("Close tab {}", index + 1))
                } else {
                    WidgetInfo::labeled(Role::Button, true, format_args!("Close {}", close_target))
                }
            });
            if close.hovered() {
                ui.painter()
                    .rect_filled(close_rect, 2.0, style.control_hover);
            }
            ui.painter().text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "×",
                style.font.clone(),
                style.title,
            );
            if close.clicked() {
                output.close = Some(index);
            }
            if close.hovered() {
                close.on_hover_text(format!("Close {}", item.accessible_label));
            }
        }
    }

    output
}

enum TabOrder {
    Canonical(usize),
    Filtered(Vec<usize>),
}
impl TabOrder {
    fn len(&self) -> usize {
        match self {
            Self::Canonical(len) => *len,
            Self::Filtered(order) => order.len(),
        }
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn at(&self, index: usize) -> usize {
        match self {
            Self::Canonical(_) => index,
            Self::Filtered(order) => order[index],
        }
    }
    fn position(&self, canonical: usize) -> Option<usize> {
        match self {
            Self::Canonical(len) => (canonical < *len).then_some(canonical),
            Self::Filtered(order) => order.iter().position(|index| *index == canonical),
        }
    }
}

/// Stable partition of canonical indices: pinned first, then unpinned; each
/// partition preserves canonical order. Search never changes canonical selection.
pub fn tab_display_order(items: &[TabStripItem<'_>], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let matches = |item: &TabStripItem<'_>| {
        query.is_empty()
            || item.title.to_lowercase().contains(&query)
            || item.accessible_label.to_lowercase().contains(&query)
    };
    let mut order = Vec::with_capacity(items.len());
    for pinned in [true, false] {
        order.extend(
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.pinned == pinned && matches(item))
                .map(|(index, _)| index),
        );
    }
    order
}

/// Optional search editor for a toolbar or overflow menu. Painting the strip
/// remains virtualized and all responses use the original canonical indices.
pub fn tab_search(ui: &mut Ui, state: &mut TabStripState) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(&mut state.query)
            .hint_text("Search tabs")
            .desired_width(140.0),
    )
}

fn paint_nav(ui: &Ui, rect: Rect, glyph: &str, hovered: bool, style: &PanelChromeStyle) {
    if hovered {
        ui.painter()
            .rect_filled(rect, style.corner_radius, style.control_hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        style.font.clone(),
        style.title,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, FontId, Vec2};

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
    fn close_tab_accessibility_names_identify_the_target_tab() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let items = [
            TabStripItem {
                title: "first.png",
                accessible_label: "First image",
                icon: None,
                modified: false,
                pinned: false,
                closable: true,
                multi_selected: false,
            },
            TabStripItem {
                title: "histogram",
                accessible_label: "Histogram panel",
                icon: None,
                modified: false,
                pinned: false,
                closable: true,
                multi_selected: false,
            },
            TabStripItem {
                title: "Untitled",
                accessible_label: "",
                icon: None,
                modified: false,
                pinned: false,
                closable: true,
                multi_selected: false,
            },
            TabStripItem {
                title: "",
                accessible_label: "",
                icon: None,
                modified: false,
                pinned: false,
                closable: true,
                multi_selected: false,
            },
        ];
        let mut state = TabStripState::default();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(420.0, 32.0),
                )),
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                tab_strip(
                    ui,
                    ui.scope_id().with("accessible-tabs"),
                    Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(420.0, 32.0)),
                    &items,
                    0,
                    &mut state,
                    &style(),
                );
            },
        );
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("tab strip exposes its AccessKit tree");
        let close_labels: Vec<_> = update
            .nodes
            .iter()
            .filter_map(|(_, node)| {
                (node.role() == Role::Button)
                    .then(|| node.label())
                    .flatten()
                    .filter(|label| label.starts_with("Close "))
            })
            .collect();
        assert!(close_labels.contains(&"Close First image"));
        assert!(close_labels.contains(&"Close Histogram panel"));
        assert!(close_labels.contains(&"Close Untitled"));
        assert!(close_labels.contains(&"Close tab 4"));
        assert_eq!(close_labels.len(), 4);
        output.textures_delta.clear();

        for key in [egui::Key::Enter, egui::Key::Space] {
            let mut focus_frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(420.0, 32.0),
                    )),
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    let id = ui.scope_id().with("accessible-tabs");
                    ui.ctx().memory_mut(|memory| {
                        memory.request_focus(id.with(("close", 0)));
                    });
                    tab_strip(
                        ui,
                        id,
                        Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(420.0, 32.0)),
                        &items,
                        0,
                        &mut state,
                        &style(),
                    );
                },
            );
            focus_frame.textures_delta.clear();

            let mut close_request = None;
            let mut key_frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(420.0, 32.0),
                    )),
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    close_request = Some(
                        tab_strip(
                            ui,
                            ui.scope_id().with("accessible-tabs"),
                            Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(420.0, 32.0)),
                            &items,
                            0,
                            &mut state,
                            &style(),
                        )
                        .close,
                    );
                },
            );
            key_frame.textures_delta.clear();
            assert_eq!(
                close_request,
                Some(Some(0)),
                "{key:?} activates focused close"
            );
        }
    }

    #[test]
    fn filtered_pointer_and_repeated_keyboard_navigation_use_canonical_indices() {
        let context = egui::Context::default();
        let items = [
            TabStripItem {
                title: "Hidden",
                accessible_label: "Hidden",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            },
            TabStripItem {
                title: "Match A",
                accessible_label: "Match A",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            },
            TabStripItem {
                title: "Match B",
                accessible_label: "Match B",
                icon: None,
                modified: true,
                pinned: true,
                closable: false,
                multi_selected: false,
            },
        ];
        let mut state = TabStripState {
            query: "match".into(),
            ..Default::default()
        };
        let pointer = egui::pos2(50.0, 15.0);
        let mut run = |events| {
            let mut result = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    result = Some(tab_strip(
                        ui,
                        ui.scope_id().with("filtered"),
                        Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(300.0, 30.0)),
                        &items,
                        0,
                        &mut state,
                        &style(),
                    ));
                },
            );
            output.textures_delta.clear();
            result.unwrap()
        };
        run(vec![egui::Event::PointerMoved(pointer)]);
        run(vec![egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }]);
        let click = run(vec![egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(click.selected, Some(2));
        assert_eq!(
            click
                .visible_bounds
                .iter()
                .map(|(index, _)| *index)
                .collect::<Vec<_>>(),
            vec![2, 1],
            "topology bounds retain canonical indices after filtering"
        );
        for (key, expected) in [(egui::Key::ArrowRight, 1), (egui::Key::ArrowLeft, 2)] {
            let response = run(vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(response.selected, Some(expected));
        }
    }

    #[test]
    fn search_and_pinning_preserve_canonical_response_indices_and_bounded_drawing() {
        let mut items = vec![
            TabStripItem {
                title: "Document",
                accessible_label: "Document",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            };
            10_000
        ];
        items[9999].title = "Needle";
        items[9999].pinned = true;
        assert_eq!(tab_display_order(&items, "needLE"), vec![9999]);
        let order = tab_display_order(&items, "");
        assert_eq!(order[0], 9999);
        assert_eq!(order[1], 0);
        let context = egui::Context::default();
        let mut state = TabStripState::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut Ui| {
            tab_strip(
                ui,
                ui.scope_id().with("bounded"),
                Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(260.0, 30.0)),
                &items,
                9999,
                &mut state,
                &style(),
            );
        });
        assert!(output.shapes.len() < 50);
        output.textures_delta.clear();
        assert_eq!(state.first_visible, 0);
    }

    #[test]
    fn overflow_is_bounded_and_selected_tab_is_scrolled_into_view() {
        let context = egui::Context::default();
        let items: Vec<_> = (0..50)
            .map(|index| TabStripItem {
                title: "Document",
                accessible_label: "Document",
                icon: None,
                modified: index == 49,
                pinned: false,
                closable: true,
                multi_selected: false,
            })
            .collect();
        let mut state = TabStripState::default();
        let mut output = None;
        let mut frame = context.run_ui(egui::RawInput::default(), &mut |ui: &mut Ui| {
            output = Some(tab_strip(
                ui,
                ui.scope_id().with("tab-strip"),
                Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(260.0, 30.0)),
                &items,
                49,
                &mut state,
                &style(),
            ));
        });
        frame.textures_delta.clear();
        assert!(state.first_visible > 0);
        assert!(state.first_visible < items.len());
        assert!(output.unwrap().selected.is_none());
    }

    #[test]
    fn pointer_selection_returns_the_canonical_tab_index() {
        let context = egui::Context::default();
        let items = [
            TabStripItem {
                title: "One",
                accessible_label: "One",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            },
            TabStripItem {
                title: "Two",
                accessible_label: "Two",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            },
            TabStripItem {
                title: "Three",
                accessible_label: "Three",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            },
        ];
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(300.0, 30.0));
        let pointer = egui::pos2(150.0, 15.0);
        let mut state = TabStripState::default();
        let mut run_frame = |events| {
            let mut response = None;
            let mut frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(320.0, 60.0),
                    )),
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    response = Some(tab_strip(
                        ui,
                        ui.scope_id().with("selectable-tabs"),
                        rect,
                        &items,
                        0,
                        &mut state,
                        &style(),
                    ));
                },
            );
            frame.textures_delta.clear();
            response.unwrap()
        };
        run_frame(vec![egui::Event::PointerMoved(pointer)]);
        run_frame(vec![
            egui::Event::PointerMoved(pointer),
            egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let response = run_frame(vec![
            egui::Event::PointerMoved(pointer),
            egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        assert_eq!(response.selected, Some(1));
        let keyboard = run_frame(vec![egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: Some(egui::Key::ArrowRight),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(keyboard.selected, Some(2));
    }

    #[test]
    fn overflow_navigation_moves_without_forcing_inactive_selection_back_into_view() {
        let context = egui::Context::default();
        let items: Vec<_> = (0..12)
            .map(|_| TabStripItem {
                title: "Document",
                accessible_label: "Document",
                icon: None,
                modified: false,
                pinned: false,
                closable: false,
                multi_selected: false,
            })
            .collect();
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(260.0, 30.0));
        let mut state = TabStripState {
            first_visible: 4,
            last_selected: Some(0),
            ..Default::default()
        };
        let pointer = egui::pos2(10.0, 15.0);
        for pressed in [None, Some(true), Some(false)] {
            let mut events = vec![egui::Event::PointerMoved(pointer)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: pointer,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(280.0, 50.0),
                    )),
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    let _ = tab_strip(
                        ui,
                        ui.scope_id().with("scrollable-tabs"),
                        rect,
                        &items,
                        0,
                        &mut state,
                        &style(),
                    );
                },
            );
            frame.textures_delta.clear();
        }
        assert!(state.first_visible < 4);
        assert_eq!(state.last_selected, Some(0));
    }

    #[test]
    fn focused_tab_reorders_with_ctrl_shift_page_keys_and_respects_bounds() {
        let context = egui::Context::default();
        let items = ["One", "Two", "Three"].map(|name| TabStripItem {
            title: name,
            accessible_label: name,
            icon: None,
            modified: false,
            pinned: false,
            closable: false,
            multi_selected: false,
        });
        let mut state = TabStripState::default();
        let selected = std::cell::Cell::new(0);
        let pointer = egui::pos2(150.0, 15.0);
        let mut run = |events| {
            let mut result = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(300.0, 40.0),
                    )),
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    result = Some(tab_strip(
                        ui,
                        ui.scope_id().with("keyboard-reorder"),
                        Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(300.0, 30.0)),
                        &items,
                        selected.get(),
                        &mut state,
                        &style(),
                    ));
                },
            );
            output.textures_delta.clear();
            let response = result.unwrap();
            if let Some(next) = response.selected {
                selected.set(next);
            }
            response
        };
        run(vec![egui::Event::PointerMoved(pointer)]);
        run(vec![egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(
            run(vec![egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }])
            .selected,
            Some(1)
        );
        let modifiers = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
        let key_event = |key| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        };
        assert_eq!(
            run(vec![key_event(egui::Key::PageUp)]).reorder,
            Some((1, 0))
        );
        assert_eq!(
            run(vec![key_event(egui::Key::PageDown)]).reorder,
            Some((1, 3))
        );

        selected.set(0);
        run(vec![egui::Event::PointerMoved(egui::pos2(50.0, 15.0))]);
        run(vec![egui::Event::PointerButton {
            pos: egui::pos2(50.0, 15.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }]);
        run(vec![egui::Event::PointerButton {
            pos: egui::pos2(50.0, 15.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(run(vec![key_event(egui::Key::PageUp)]).reorder, None);
    }

    #[test]
    fn host_focus_request_survives_tab_rebuild_and_confirms_the_same_index() {
        let context = egui::Context::default();
        let items = ["Activity", "Editor"].map(|name| TabStripItem {
            title: name,
            accessible_label: name,
            icon: None,
            modified: false,
            pinned: false,
            closable: false,
            multi_selected: false,
        });
        let mut state = TabStripState {
            focus_request: Some(1),
            ..Default::default()
        };
        for frame in 0..2 {
            let mut response = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(240.0, 40.0),
                    )),
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    response = Some(tab_strip(
                        ui,
                        ui.scope_id().with("reorder-focus"),
                        Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(240.0, 30.0)),
                        &items,
                        1,
                        &mut state,
                        &style(),
                    ));
                },
            );
            output.textures_delta.clear();
            if frame == 1 {
                assert_eq!(response.unwrap().focused, Some(1));
                assert_eq!(state.focus_request, None);
            }
        }
    }

    #[test]
    fn ctrl_click_toggles_multi_selection_without_changing_active_tab() {
        let context = egui::Context::default();
        let items = [TabStripItem {
            title: "One",
            accessible_label: "One",
            icon: None,
            modified: false,
            pinned: false,
            closable: false,
            multi_selected: false,
        }];
        let mut state = TabStripState::default();
        let pointer = egui::pos2(40.0, 15.0);
        let ctrl = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let mut output = None;
        for pressed in [None, Some(true), Some(false)] {
            let mut events = vec![egui::Event::PointerMoved(pointer)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: pointer,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: ctrl,
                });
            }
            let mut frame = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(180.0, 32.0),
                    )),
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    output = Some(tab_strip(
                        ui,
                        ui.scope_id().with("multi-select-tabs"),
                        Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(180.0, 32.0)),
                        &items,
                        0,
                        &mut state,
                        &style(),
                    ));
                },
            );
            frame.textures_delta.clear();
        }
        let response = output.unwrap();
        assert_eq!(response.multi_toggled, Some(0));
        assert_eq!(response.selected, None);
    }
}
