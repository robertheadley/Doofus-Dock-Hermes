//! Reusable transient modal input and focus scope for egui hosts.
use egui::{Color32, Context, Id, Ui};

/// Host-owned modal state. It is deliberately separate from canonical layout.
#[derive(Clone, Debug, Default)]
pub struct ModalScopeState {
    id: Option<Id>,
    title: String,
    restore_focus: Option<Id>,
    focus_close_button: bool,
}

impl ModalScopeState {
    /// Opens a modal scope and remembers the currently focused host widget.
    /// Repeated opens while a scope is active are idempotent.
    pub fn open(&mut self, ctx: &Context, id: Id, title: impl Into<String>) -> bool {
        if self.id.is_some() {
            return false;
        }
        self.restore_focus = ctx.memory(|memory| memory.focused());
        self.id = Some(id);
        self.title = title.into();
        self.focus_close_button = true;
        true
    }

    pub fn is_open(&self) -> bool {
        self.id.is_some()
    }

    /// Draws the modal backdrop and dialog. Background UI should be disabled
    /// by the host for the duration of this scope.
    pub fn show(&mut self, ctx: &Context) {
        let Some(id) = self.id else {
            if let Some(focus) = self.restore_focus.take() {
                ctx.memory_mut(|memory| memory.request_focus(focus));
            }
            return;
        };
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.close();
            return;
        }

        let mut close = false;
        let mut close_button_focused = false;
        let _ = egui::Modal::new(id)
            .backdrop_color(Color32::from_black_alpha(150))
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.heading(&self.title);
                ui.label("Modal scope is active. Background docking and shortcuts are suspended.");
                ui.add_space(12.0);
                let response = ui.button("Close dialog");
                if self.focus_close_button {
                    response.request_focus();
                    close_button_focused = response.has_focus();
                }
                close = response.clicked();
            });
        if close_button_focused {
            self.focus_close_button = false;
        }
        if close {
            self.close();
        }
    }

    /// Closes the scope and restores focus to the widget active before open.
    pub fn close(&mut self) -> bool {
        if self.id.take().is_none() {
            return false;
        }
        self.title.clear();
        self.focus_close_button = false;
        true
    }
}

/// Wraps host content in an enabled/disabled UI scope based on modal state.
pub fn with_modal_background<R>(
    ui: &mut Ui,
    modal_open: bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.add_enabled_ui(!modal_open, add_contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Sense, Vec2};

    #[test]
    fn modal_disables_background_and_escape_restores_previous_focus() {
        let ctx = Context::default();
        let source_id = Id::unique("modal-test-source");
        let mut modal = ModalScopeState::default();
        let mut background_clicked = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(500.0, 360.0),
                )),
                focused: true,
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                ui.interact(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(80.0, 40.0)),
                    source_id,
                    Sense::click(),
                );
            },
        );
        output.textures_delta.clear();
        ctx.memory_mut(|memory| memory.request_focus(source_id));
        assert!(modal.open(&ctx, Id::unique("modal-test"), "Modal test"));

        for events in [
            vec![egui::Event::PointerMoved(egui::pos2(250.0, 220.0))],
            vec![egui::Event::PointerButton {
                pos: egui::pos2(250.0, 220.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos: egui::pos2(250.0, 220.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(500.0, 360.0),
                    )),
                    focused: true,
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    with_modal_background(ui, modal.is_open(), |ui| {
                        if ui
                            .put(
                                egui::Rect::from_min_size(
                                    egui::pos2(200.0, 200.0),
                                    Vec2::new(100.0, 36.0),
                                ),
                                egui::Button::new("Background action"),
                            )
                            .clicked()
                        {
                            background_clicked = true;
                        }
                    });
                    modal.show(&ctx);
                },
            );
            output.textures_delta.clear();
        }
        assert!(modal.is_open());
        assert!(!background_clicked, "modal blocks background activation");
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(500.0, 360.0),
                )),
                focused: true,
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: Some(egui::Key::Escape),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                modal.show(&ctx);
                ui.interact(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(80.0, 40.0)),
                    source_id,
                    Sense::click(),
                );
            },
        );
        output.textures_delta.clear();
        assert!(!modal.is_open());
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(500.0, 360.0),
                )),
                focused: true,
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                ui.interact(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(80.0, 40.0)),
                    source_id,
                    Sense::click(),
                );
                modal.show(&ctx);
            },
        );
        output.textures_delta.clear();
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(source_id));
    }
}
