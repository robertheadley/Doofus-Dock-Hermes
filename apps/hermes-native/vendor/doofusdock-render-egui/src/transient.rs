//! Reusable host-owned overlay, drawer, and peek presentation scopes.
//!
//! These surfaces are presentation state only. Hosts keep them outside
//! `LayoutDocument` and disable background input while `is_modal()` is true.
use egui::{Color32, Context, Id, Order, Pos2, Rect, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransientSurfaceKind {
    /// Centered overlay. Modal overlays block host background input.
    Overlay { modal: bool },
    /// Edge-attached drawer, optionally modal.
    Drawer { edge: SurfaceEdge, modal: bool },
    /// Lightweight edge peek that closes on Escape or an outside click.
    Peek { edge: SurfaceEdge },
}

impl TransientSurfaceKind {
    pub fn is_modal(self) -> bool {
        match self {
            Self::Overlay { modal } | Self::Drawer { modal, .. } => modal,
            Self::Peek { .. } => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceDismissal {
    Escape,
    OutsideClick,
    CloseButton,
}

/// One transient surface and its keyboard-focus lifecycle.
#[derive(Clone, Debug, Default)]
pub struct TransientSurfaceState {
    id: Option<Id>,
    title: String,
    kind: Option<TransientSurfaceKind>,
    restore_focus: Option<Id>,
    initial_focus: Option<Id>,
    request_initial_focus: bool,
}

impl TransientSurfaceState {
    /// Opens one surface and records the currently focused host widget.
    /// Repeated opens while active are idempotently rejected.
    pub fn open(
        &mut self,
        ctx: &Context,
        id: Id,
        title: impl Into<String>,
        kind: TransientSurfaceKind,
        initial_focus: Option<Id>,
    ) -> bool {
        if self.id.is_some() {
            return false;
        }
        self.restore_focus = ctx.memory(|memory| memory.focused());
        self.id = Some(id);
        self.title = title.into();
        self.kind = Some(kind);
        self.initial_focus = initial_focus;
        self.request_initial_focus = initial_focus.is_some();
        true
    }

    pub fn is_open(&self) -> bool {
        self.id.is_some()
    }

    pub fn is_modal(&self) -> bool {
        self.kind.is_some_and(TransientSurfaceKind::is_modal)
    }

    /// Closes the surface. Focus is restored by the next `show` call after the
    /// host has rebuilt the background widget tree.
    pub fn close(&mut self) -> bool {
        if self.id.take().is_none() {
            return false;
        }
        self.title.clear();
        self.kind = None;
        self.initial_focus = None;
        self.request_initial_focus = false;
        true
    }

    /// Draws a bounded transient surface and invokes `add_contents` inside it.
    /// The host remains responsible for disabling background input for modal
    /// surfaces by consulting `is_modal()` before rendering its workspace.
    pub fn show(
        &mut self,
        ctx: &Context,
        screen_rect: Rect,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Option<SurfaceDismissal> {
        let Some(id) = self.id else {
            if let Some(focus) = self.restore_focus.take() {
                ctx.memory_mut(|memory| memory.request_focus(focus));
            }
            return None;
        };
        let Some(kind) = self.kind else {
            self.close();
            return None;
        };
        let rect = surface_rect(screen_rect, kind);
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.close();
            return Some(SurfaceDismissal::Escape);
        }
        if matches!(kind, TransientSurfaceKind::Peek { .. })
            && ctx.input(|input| input.pointer.primary_clicked())
            && ctx
                .input(|input| input.pointer.interact_pos())
                .is_some_and(|pointer| !rect.contains(pointer))
        {
            self.close();
            return Some(SurfaceDismissal::OutsideClick);
        }

        if kind.is_modal() {
            ctx.layer_painter(egui::LayerId::new(Order::Foreground, id.with("backdrop")))
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(150));
        }

        let mut close = false;
        let title = self.title.clone();
        let initial_focus = self.initial_focus;
        let request_initial_focus = self.request_initial_focus;
        let area_id = id.with("surface");
        egui::Area::new(area_id)
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .interactable(true)
            .constrain(false)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_size(rect.size());
                    ui.set_max_size(rect.size());
                    ui.horizontal(|ui| {
                        ui.heading(&title);
                        let response = ui.button("Close");
                        if request_initial_focus && initial_focus.is_none() {
                            response.request_focus();
                        }
                        close = response.clicked();
                    });
                    ui.separator();
                    add_contents(ui);
                });
            });
        if request_initial_focus {
            if let Some(focus) = initial_focus {
                ctx.memory_mut(|memory| memory.request_focus(focus));
            }
            self.request_initial_focus = false;
        }
        if close {
            self.close();
            Some(SurfaceDismissal::CloseButton)
        } else {
            None
        }
    }
}

/// Computes deterministic, screen-bounded placement for a transient surface.
pub fn surface_rect(screen: Rect, kind: TransientSurfaceKind) -> Rect {
    let size = screen.size();
    let (width, height) = match kind {
        TransientSurfaceKind::Overlay { .. } => {
            (size.x.clamp(0.0, 640.0), size.y.clamp(0.0, 480.0))
        }
        TransientSurfaceKind::Drawer { edge, .. } => match edge {
            SurfaceEdge::Left | SurfaceEdge::Right => (size.x.clamp(0.0, 360.0), size.y.max(0.0)),
            SurfaceEdge::Top | SurfaceEdge::Bottom => (size.x.max(0.0), size.y.clamp(0.0, 280.0)),
        },
        TransientSurfaceKind::Peek { edge } => match edge {
            SurfaceEdge::Left | SurfaceEdge::Right => (size.x.clamp(0.0, 400.0), size.y.max(0.0)),
            SurfaceEdge::Top | SurfaceEdge::Bottom => (size.x.max(0.0), size.y.clamp(0.0, 320.0)),
        },
    };
    let min = match kind {
        TransientSurfaceKind::Overlay { .. } => Pos2::new(
            screen.center().x - width / 2.0,
            screen.center().y - height / 2.0,
        ),
        TransientSurfaceKind::Drawer { edge, .. } | TransientSurfaceKind::Peek { edge } => {
            match edge {
                SurfaceEdge::Left | SurfaceEdge::Top => screen.min,
                SurfaceEdge::Right => Pos2::new(screen.right() - width, screen.top()),
                SurfaceEdge::Bottom => Pos2::new(screen.left(), screen.bottom() - height),
            }
        }
    };
    Rect::from_min_size(min, Vec2::new(width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Sense;

    #[test]
    fn surface_rects_are_edge_attached_and_bounded() {
        let screen = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(800.0, 600.0));
        let right = surface_rect(
            screen,
            TransientSurfaceKind::Drawer {
                edge: SurfaceEdge::Right,
                modal: false,
            },
        );
        assert_eq!(right.right(), screen.right());
        assert_eq!(right.size(), Vec2::new(360.0, 600.0));
        let overlay = surface_rect(screen, TransientSurfaceKind::Overlay { modal: true });
        assert_eq!(overlay.center(), screen.center());
        assert!(screen.contains(overlay.min) && screen.contains(overlay.max));
        let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(80.0, 40.0));
        let peek = surface_rect(
            tiny,
            TransientSurfaceKind::Peek {
                edge: SurfaceEdge::Bottom,
            },
        );
        assert_eq!(peek, tiny);
    }

    #[test]
    fn surface_lifecycle_is_idempotent_and_escape_restores_focus() {
        let ctx = Context::default();
        let source = Id::unique("transient-source");
        let mut state = TransientSurfaceState::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(500.0, 360.0));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                focused: true,
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                ui.interact(
                    Rect::from_min_size(Pos2::ZERO, Vec2::splat(80.0)),
                    source,
                    Sense::click(),
                );
            },
        );
        output.textures_delta.clear();
        ctx.memory_mut(|memory| memory.request_focus(source));
        let id = Id::unique("transient-surface");
        let kind = TransientSurfaceKind::Drawer {
            edge: SurfaceEdge::Right,
            modal: true,
        };
        assert!(state.open(&ctx, id, "Inspector", kind, None));
        assert!(!state.open(&ctx, id, "Inspector", kind, None));
        assert!(state.is_modal());

        let mut background_clicked = false;
        for events in [
            vec![egui::Event::PointerMoved(Pos2::new(100.0, 100.0))],
            vec![egui::Event::PointerButton {
                pos: Pos2::new(100.0, 100.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos: Pos2::new(100.0, 100.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    focused: true,
                    events,
                    ..Default::default()
                },
                &mut |ui: &mut Ui| {
                    crate::modal::with_modal_background(ui, state.is_modal(), |ui| {
                        if ui
                            .add_sized([140.0, 36.0], egui::Button::new("Background"))
                            .clicked()
                        {
                            background_clicked = true;
                        }
                    });
                    state.show(&ctx, screen, |_| {});
                },
            );
            output.textures_delta.clear();
        }
        assert!(!background_clicked);
        let result = state.show(&ctx, screen, |_| {});
        assert_eq!(result, None);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
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
                crate::modal::with_modal_background(ui, state.is_modal(), |_| {});
                assert_eq!(
                    state.show(&ctx, screen, |_| {}),
                    Some(SurfaceDismissal::Escape)
                );
            },
        );
        output.textures_delta.clear();
        assert!(!state.is_open());
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                focused: true,
                ..Default::default()
            },
            &mut |ui: &mut Ui| {
                ui.interact(
                    Rect::from_min_size(Pos2::ZERO, Vec2::splat(80.0)),
                    source,
                    Sense::click(),
                );
                state.show(&ctx, screen, |_| {});
            },
        );
        output.textures_delta.clear();
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(source));
    }

    #[test]
    fn peek_dismisses_on_outside_click_but_drawer_does_not() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(500.0, 360.0));
        for (kind, expected) in [
            (
                TransientSurfaceKind::Peek {
                    edge: SurfaceEdge::Right,
                },
                Some(SurfaceDismissal::OutsideClick),
            ),
            (
                TransientSurfaceKind::Drawer {
                    edge: SurfaceEdge::Right,
                    modal: false,
                },
                None,
            ),
        ] {
            let mut state = TransientSurfaceState::default();
            state.open(&ctx, Id::unique("outside-click"), "Surface", kind, None);
            let pointer = Pos2::new(20.0, 20.0);
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    focused: true,
                    events: vec![
                        egui::Event::PointerMoved(pointer),
                        egui::Event::PointerButton {
                            pos: pointer,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerButton {
                            pos: pointer,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                &mut |_ui: &mut Ui| assert_eq!(state.show(&ctx, screen, |_| {}), expected),
            );
            output.textures_delta.clear();
            assert_eq!(state.is_open(), expected.is_none());
        }
    }
}
