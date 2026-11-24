//! Platform-independent input normalization helpers for egui hosts.

use std::collections::{BTreeMap, BTreeSet};

use doofusdock_core::{DockError, DockResult, ErrorCode, PanelId};
use doofusdock_presentation::{
    FlattenedPath2D, SceneFrame, SceneHit, ScenePoint, ScenePointerPhase, ScenePointerRouter,
    ValidatedShape2D,
};
use egui::{
    Event, FullOutput, Modifiers, PlatformOutput, PointerButton, Pos2, RawInput, TouchDeviceId,
    TouchId, TouchPhase,
};

type Contact = (TouchDeviceId, TouchId);

/// One egui pointer event translated into the target panel's local coordinate
/// space. Hosts can deliver it to that panel's widget tree without changing
/// the canonical layout or the shared viewport input stream.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePanelPointerEvent {
    pub panel: PanelId,
    pub event: Event,
    pub captured: bool,
}

/// Result of translating a single egui pointer event through a scene frame.
/// A stale capture can be canceled at the same time a fresh event is routed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneEguiPointerDispatch {
    pub routed: Option<ScenePanelPointerEvent>,
    pub cancelled: Option<PanelId>,
}

/// Keyboard, text, IME, and clipboard input for a panel already focused by
/// the host. This adapter forwards input; it does not choose or change focus.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePanelFocusedInput {
    pub panel: PanelId,
    pub event: Event,
}

/// Revision-bound bridge for non-pointer input to a host-selected focused panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneEguiFocusedInputAdapter;

/// Read-only platform output selected for the host-focused panel while keeping
/// every visible panel's output available for accessibility composition.
///
/// The host should take cursor, IME, and side-effect commands from
/// [`Self::focused_output`]. Accessibility snapshots are per egui context, so
/// a host embedding several panel contexts must compose the visible outputs
/// instead of forwarding only the focused panel's AccessKit update.
pub struct SceneEguiPlatformOutputPlan<'a> {
    focused_panel: PanelId,
    focused_output: &'a PlatformOutput,
    visible_outputs: BTreeMap<PanelId, &'a PlatformOutput>,
}

impl<'a> SceneEguiPlatformOutputPlan<'a> {
    pub fn focused_panel(&self) -> &PanelId {
        &self.focused_panel
    }

    /// Platform output for the panel the host has already focused.
    pub fn focused_output(&self) -> &'a PlatformOutput {
        self.focused_output
    }

    /// Per-context outputs for every panel visible in the validated frame.
    /// Hosts use these to assemble an accessibility tree and must avoid
    /// executing commands from panels that do not own host focus.
    pub fn visible_outputs(&self) -> &BTreeMap<PanelId, &'a PlatformOutput> {
        &self.visible_outputs
    }
}

/// Selects revision-bound panel platform output without merging independent
/// egui contexts or allowing background panels to issue host side effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneEguiPlatformOutputAdapter;

impl SceneEguiPlatformOutputAdapter {
    /// Validate the host focus and return only current visible panel outputs.
    ///
    /// `outputs` is keyed by the same stable panel IDs used by the scene. Extra
    /// retained contexts are ignored; missing visible outputs fail closed.
    pub fn select<'a>(
        &self,
        frame: &SceneFrame,
        focused_panel: &PanelId,
        outputs: &'a BTreeMap<PanelId, FullOutput>,
        revision: u64,
        generation: u64,
    ) -> DockResult<SceneEguiPlatformOutputPlan<'a>> {
        if frame.revision() != revision || frame.generation() != generation {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Platform output targets a stale scene frame.",
                format!(
                    "frame={}:{} requested={revision}:{generation}",
                    frame.revision(),
                    frame.generation()
                ),
            ));
        }

        let visible_panels = frame
            .elements()
            .iter()
            .map(|element| element.panel.clone())
            .collect::<BTreeSet<_>>();
        if !visible_panels.contains(focused_panel) {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Platform output focus is not visible in the scene frame.",
                &focused_panel.0,
            ));
        }

        let mut visible_outputs = BTreeMap::new();
        for panel in visible_panels {
            let output = outputs.get(&panel).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "A visible panel has no current platform output.",
                    &panel.0,
                )
            })?;
            visible_outputs.insert(panel, &output.platform_output);
        }
        let focused_output = visible_outputs[focused_panel];

        Ok(SceneEguiPlatformOutputPlan {
            focused_panel: focused_panel.clone(),
            focused_output,
            visible_outputs,
        })
    }
}

impl SceneEguiFocusedInputAdapter {
    /// Forward keyboard/text/IME/clipboard input only when the host's focused
    /// panel is visible in this frame. Pointer, scroll, and window events are
    /// ignored; focus selection and accessibility stay host-owned.
    pub fn route(
        &self,
        frame: &SceneFrame,
        focused_panel: &PanelId,
        event: &Event,
        revision: u64,
        generation: u64,
    ) -> DockResult<Option<ScenePanelFocusedInput>> {
        if frame.revision() != revision || frame.generation() != generation {
            return Err(doofusdock_core::DockError::new(
                doofusdock_core::ErrorCode::Conflict,
                "Focused input targets a stale scene frame.",
                format!(
                    "frame={}:{} requested={revision}:{generation}",
                    frame.revision(),
                    frame.generation()
                ),
            ));
        }
        if !frame
            .elements()
            .iter()
            .any(|element| &element.panel == focused_panel)
        {
            return Err(doofusdock_core::DockError::new(
                doofusdock_core::ErrorCode::NotFound,
                "Focused input targets a panel that is not visible in the scene frame.",
                &focused_panel.0,
            ));
        }
        let is_focused_input = matches!(
            event,
            Event::Key { .. }
                | Event::Text(_)
                | Event::Ime(_)
                | Event::Paste(_)
                | Event::Copy
                | Event::Cut
        );
        Ok(is_focused_input.then(|| ScenePanelFocusedInput {
            panel: focused_panel.clone(),
            event: event.clone(),
        }))
    }
}

/// Scroll event routed to the frontmost hit panel. The host should send a
/// `PointerMoved(pointer_position)` event before `event` in that panel's context.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePanelScrollEvent {
    pub panel: PanelId,
    pub pointer_position: Pos2,
    pub event: Event,
}

/// Read-only scene hit testing for egui wheel/trackpad events.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneEguiScrollAdapter;

impl SceneEguiScrollAdapter {
    /// Route one MouseWheel event using the host's current world-space pointer.
    /// Non-wheel events and points outside all visible panels return `None`.
    pub fn route(
        &self,
        frame: &SceneFrame,
        pointer_position: Pos2,
        event: &Event,
        revision: u64,
        generation: u64,
    ) -> DockResult<Option<ScenePanelScrollEvent>> {
        self.route_with(pointer_position, event, |point| {
            frame.hit_test_detail(point, revision, generation)
        })
    }

    /// Route through validated panel-local primitive hit shapes.
    pub fn route_with_shapes(
        &self,
        frame: &SceneFrame,
        pointer_position: Pos2,
        event: &Event,
        revision: u64,
        generation: u64,
        shapes: &BTreeMap<PanelId, ValidatedShape2D>,
    ) -> DockResult<Option<ScenePanelScrollEvent>> {
        self.route_with(pointer_position, event, |point| {
            frame.hit_test_detail_with_shapes(point, revision, generation, shapes)
        })
    }

    /// Route through validated panel-local flattened path masks.
    pub fn route_with_paths(
        &self,
        frame: &SceneFrame,
        pointer_position: Pos2,
        event: &Event,
        revision: u64,
        generation: u64,
        paths: &BTreeMap<PanelId, FlattenedPath2D>,
    ) -> DockResult<Option<ScenePanelScrollEvent>> {
        self.route_with(pointer_position, event, |point| {
            frame.hit_test_detail_with_paths(point, revision, generation, paths)
        })
    }

    fn route_with(
        &self,
        pointer_position: Pos2,
        event: &Event,
        mut hit_test: impl FnMut(ScenePoint) -> DockResult<Option<SceneHit>>,
    ) -> DockResult<Option<ScenePanelScrollEvent>> {
        let Event::MouseWheel { delta, .. } = event else {
            return Ok(None);
        };
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Scroll delta must be finite.",
                format!("delta={delta:?}"),
            ));
        }
        let point = ScenePoint {
            x: f64::from(pointer_position.x),
            y: f64::from(pointer_position.y),
        };
        let Some(hit) = hit_test(point)? else {
            return Ok(None);
        };
        let local_pointer = Pos2::new(hit.local_point.x as f32, hit.local_point.y as f32);
        if !local_pointer.x.is_finite() || !local_pointer.y.is_finite() {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Scroll target local pointer is outside egui's coordinate range.",
                format!("panel={} local={:?}", hit.panel.0, hit.local_point),
            ));
        }
        Ok(Some(ScenePanelScrollEvent {
            panel: hit.panel,
            pointer_position: local_pointer,
            event: event.clone(),
        }))
    }
}

/// egui adapter for the presentation crate's primary-pointer capture router.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneEguiPointerAdapter {
    router: ScenePointerRouter,
}

impl SceneEguiPointerAdapter {
    /// Translate pointer movement/button events through unmasked scene hit
    /// testing. Non-pointer events are ignored. `PointerGone` cancels capture.
    pub fn route(
        &mut self,
        frame: &SceneFrame,
        event: &Event,
        revision: u64,
        generation: u64,
    ) -> DockResult<SceneEguiPointerDispatch> {
        self.route_with(event, |router, phase, point| {
            router.route(frame, phase, point, revision, generation)
        })
    }

    /// Translate pointer events using validated panel-local primitive masks.
    pub fn route_with_shapes(
        &mut self,
        frame: &SceneFrame,
        event: &Event,
        revision: u64,
        generation: u64,
        shapes: &BTreeMap<PanelId, ValidatedShape2D>,
    ) -> DockResult<SceneEguiPointerDispatch> {
        self.route_with(event, |router, phase, point| {
            router.route_with_shapes(frame, phase, point, revision, generation, shapes)
        })
    }

    /// Translate pointer events using flattened panel-local compound paths.
    pub fn route_with_paths(
        &mut self,
        frame: &SceneFrame,
        event: &Event,
        revision: u64,
        generation: u64,
        paths: &BTreeMap<PanelId, FlattenedPath2D>,
    ) -> DockResult<SceneEguiPointerDispatch> {
        self.route_with(event, |router, phase, point| {
            router.route_with_paths(frame, phase, point, revision, generation, paths)
        })
    }

    /// Explicitly cancel the active scene capture, for example on focus loss.
    pub fn cancel(&mut self) -> Option<PanelId> {
        self.router.cancel()
    }

    fn route_with(
        &mut self,
        event: &Event,
        mut route: impl FnMut(
            &mut ScenePointerRouter,
            ScenePointerPhase,
            ScenePoint,
        ) -> DockResult<doofusdock_presentation::ScenePointerDispatch>,
    ) -> DockResult<SceneEguiPointerDispatch> {
        let (phase, point, button) = match event {
            Event::PointerMoved(pos) => (
                ScenePointerPhase::Move,
                ScenePoint {
                    x: f64::from(pos.x),
                    y: f64::from(pos.y),
                },
                None,
            ),
            Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => (
                if *pressed {
                    ScenePointerPhase::Press
                } else {
                    ScenePointerPhase::Release
                },
                ScenePoint {
                    x: f64::from(pos.x),
                    y: f64::from(pos.y),
                },
                Some((*button, *pressed, *modifiers)),
            ),
            Event::PointerGone => {
                return Ok(SceneEguiPointerDispatch {
                    routed: None,
                    cancelled: self.router.cancel(),
                });
            }
            _ => return Ok(SceneEguiPointerDispatch::default()),
        };
        let dispatch = route(&mut self.router, phase, point)?;
        let routed = dispatch.target.and_then(|target| {
            let local = Pos2::new(target.local_point.x as f32, target.local_point.y as f32);
            let event = match (phase, button) {
                (ScenePointerPhase::Move, _) => Event::PointerMoved(local),
                (
                    ScenePointerPhase::Press | ScenePointerPhase::Release,
                    Some((button, pressed, modifiers)),
                ) => Event::PointerButton {
                    pos: local,
                    button,
                    pressed,
                    modifiers,
                },
                _ => return None,
            };
            Some(ScenePanelPointerEvent {
                panel: target.panel,
                event,
                captured: dispatch.captured,
            })
        });
        Ok(SceneEguiPointerDispatch {
            routed,
            cancelled: dispatch.cancelled,
        })
    }
}

/// Converts a single active touch into egui's primary pointer events.
///
/// Original [`Event::Touch`] events are retained for gesture recognition. When
/// multiple contacts are active, pointer synthesis is canceled until every
/// contact ends so a pinch cannot become an accidental click or drag.
#[derive(Clone, Debug, Default)]
pub struct TouchPointerAdapter {
    active: BTreeSet<Contact>,
    primary: Option<Contact>,
    suppress_until_all_ended: bool,
}

impl TouchPointerAdapter {
    /// Retain touch events and add pointer events for safe single-touch input.
    pub fn normalize(&mut self, input: &mut RawInput) {
        if !input.focused {
            self.cancel_pointer(&mut input.events);
            self.active.clear();
            self.suppress_until_all_ended = false;
            return;
        }

        let mut normalized = Vec::with_capacity(input.events.len() + 4);
        for event in std::mem::take(&mut input.events) {
            let touch = match &event {
                Event::Touch {
                    device_id,
                    id,
                    phase,
                    pos,
                    ..
                } => Some(((*device_id, *id), *phase, *pos)),
                _ => None,
            };
            normalized.push(event);
            if let Some((contact, phase, pos)) = touch {
                self.process_touch(contact, phase, pos, &mut normalized);
            }
        }
        input.events = normalized;
    }

    fn process_touch(
        &mut self,
        contact: Contact,
        phase: TouchPhase,
        pos: Pos2,
        events: &mut Vec<Event>,
    ) {
        match phase {
            TouchPhase::Start => {
                let was_empty = self.active.is_empty();
                self.active.insert(contact);
                if was_empty && !self.suppress_until_all_ended {
                    self.primary = Some(contact);
                    events.push(Event::PointerMoved(pos));
                    events.push(pointer_button(pos, true));
                } else if self.active.len() > 1 {
                    self.suppress_until_all_ended = true;
                    self.cancel_pointer(events);
                }
            }
            TouchPhase::Move => {
                if self.primary == Some(contact) && !self.suppress_until_all_ended {
                    events.push(Event::PointerMoved(pos));
                }
            }
            TouchPhase::End => {
                self.active.remove(&contact);
                if self.primary == Some(contact) {
                    if !self.suppress_until_all_ended {
                        events.push(Event::PointerMoved(pos));
                        events.push(pointer_button(pos, false));
                        events.push(Event::PointerGone);
                    }
                    self.primary = None;
                }
                if self.active.is_empty() {
                    self.suppress_until_all_ended = false;
                    self.primary = None;
                }
            }
            TouchPhase::Cancel => {
                self.active.remove(&contact);
                if self.primary == Some(contact) {
                    self.cancel_pointer(events);
                }
                if self.active.is_empty() {
                    self.suppress_until_all_ended = false;
                    self.primary = None;
                }
            }
        }
    }

    fn cancel_pointer(&mut self, events: &mut Vec<Event>) {
        if self.primary.take().is_some() {
            // Release far outside the logical viewport so egui cannot report a
            // click on the widget that owned the canceled press.
            let outside = Pos2::new(-1.0e6, -1.0e6);
            events.push(pointer_button(outside, false));
            events.push(Event::PointerGone);
        }
    }
}

fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doofusdock_presentation::{Affine2, SceneNode, SceneRect};
    use egui::{Button, OutputCommand, Rect, Vec2};

    const DEVICE: TouchDeviceId = TouchDeviceId(1);
    const FIRST: TouchId = TouchId(10);
    const SECOND: TouchId = TouchId(11);

    fn touch(id: TouchId, phase: TouchPhase, x: f32, y: f32) -> Event {
        Event::Touch {
            device_id: DEVICE,
            id,
            phase,
            pos: Pos2::new(x, y),
            force: None,
        }
    }

    fn normalize(adapter: &mut TouchPointerAdapter, events: Vec<Event>) -> Vec<Event> {
        let mut input = RawInput {
            events,
            ..Default::default()
        };
        adapter.normalize(&mut input);
        input.events
    }

    fn translated_scene() -> SceneFrame {
        let mut panel = SceneNode::panel(
            "scene-input",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 40.0,
                height: 30.0,
            },
        );
        panel.transform = Affine2::translate(10.0, 20.0).unwrap();
        SceneFrame::build(&panel, 7, 3).unwrap()
    }

    fn two_panel_scene() -> SceneFrame {
        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 240.0,
            height: 80.0,
        });
        root.children = vec![
            SceneNode::panel(
                "platform-left",
                SceneRect {
                    x: 0.0,
                    y: 0.0,
                    width: 120.0,
                    height: 80.0,
                },
            ),
            SceneNode::panel(
                "platform-right",
                SceneRect {
                    x: 120.0,
                    y: 0.0,
                    width: 120.0,
                    height: 80.0,
                },
            ),
        ];
        SceneFrame::build(&root, 19, 6).unwrap()
    }

    #[test]
    fn platform_output_selects_focused_side_effects_and_preserves_visible_contexts() {
        let frame = two_panel_scene();
        let focused = PanelId::from("platform-left");
        let background = PanelId::from("platform-right");
        let mut focused_output = FullOutput::default();
        focused_output
            .platform_output
            .commands
            .push(OutputCommand::CopyText("focused clipboard".into()));
        let mut background_output = FullOutput::default();
        background_output
            .platform_output
            .commands
            .push(OutputCommand::CopyText("background clipboard".into()));
        let outputs = BTreeMap::from([
            (focused.clone(), focused_output),
            (background.clone(), background_output),
        ]);

        let plan = SceneEguiPlatformOutputAdapter
            .select(&frame, &focused, &outputs, 19, 6)
            .unwrap();

        assert_eq!(plan.focused_panel(), &focused);
        assert_eq!(
            plan.focused_output().commands,
            vec![OutputCommand::CopyText("focused clipboard".into())]
        );
        assert_eq!(plan.visible_outputs().len(), 2);
        assert!(plan.visible_outputs().contains_key(&focused));
        assert!(plan.visible_outputs().contains_key(&background));
        assert_eq!(
            plan.visible_outputs()[&background].commands,
            vec![OutputCommand::CopyText("background clipboard".into())]
        );
    }

    #[test]
    fn platform_output_rejects_stale_focus_and_missing_visible_context() {
        let frame = two_panel_scene();
        let focused = PanelId::from("platform-left");
        let background = PanelId::from("platform-right");
        let outputs = BTreeMap::from([
            (focused.clone(), FullOutput::default()),
            (background.clone(), FullOutput::default()),
        ]);
        let adapter = SceneEguiPlatformOutputAdapter;

        let stale_error = match adapter.select(&frame, &focused, &outputs, 20, 6) {
            Err(error) => error,
            Ok(_) => panic!("stale platform output should be rejected"),
        };
        assert_eq!(stale_error.code, ErrorCode::Conflict);

        let incomplete = BTreeMap::from([(focused.clone(), FullOutput::default())]);
        let error = match adapter.select(&frame, &focused, &incomplete, 19, 6) {
            Err(error) => error,
            Ok(_) => panic!("a missing visible panel output should be rejected"),
        };
        assert_eq!(error.code, ErrorCode::NotFound);
        assert_eq!(error.context, background.0);

        let hidden_error = match adapter.select(&frame, &PanelId::from("hidden"), &outputs, 19, 6) {
            Err(error) => error,
            Ok(_) => panic!("hidden panel platform output should be rejected"),
        };
        assert_eq!(hidden_error.code, ErrorCode::NotFound);
    }

    #[test]
    fn scene_adapter_maps_egui_pointer_events_to_panel_local_coordinates() {
        let frame = translated_scene();
        let mut adapter = SceneEguiPointerAdapter::default();
        let moved = adapter
            .route(&frame, &Event::PointerMoved(Pos2::new(15.0, 26.0)), 7, 3)
            .unwrap();
        assert_eq!(
            moved.routed,
            Some(ScenePanelPointerEvent {
                panel: "scene-input".into(),
                event: Event::PointerMoved(Pos2::new(5.0, 6.0)),
                captured: false,
            })
        );

        let pressed = adapter
            .route(
                &frame,
                &Event::PointerButton {
                    pos: Pos2::new(15.0, 26.0),
                    button: PointerButton::Secondary,
                    pressed: true,
                    modifiers: Modifiers::CTRL,
                },
                7,
                3,
            )
            .unwrap();
        assert_eq!(
            pressed.routed,
            Some(ScenePanelPointerEvent {
                panel: "scene-input".into(),
                event: Event::PointerButton {
                    pos: Pos2::new(5.0, 6.0),
                    button: PointerButton::Secondary,
                    pressed: true,
                    modifiers: Modifiers::CTRL,
                },
                captured: true,
            })
        );
    }

    #[test]
    fn scene_adapter_keeps_capture_outside_bounds_and_reports_pointer_loss() {
        let frame = translated_scene();
        let mut adapter = SceneEguiPointerAdapter::default();
        adapter
            .route(
                &frame,
                &Event::PointerButton {
                    pos: Pos2::new(15.0, 26.0),
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
                7,
                3,
            )
            .unwrap();
        let moved = adapter
            .route(&frame, &Event::PointerMoved(Pos2::new(100.0, 120.0)), 7, 3)
            .unwrap();
        assert_eq!(
            moved.routed,
            Some(ScenePanelPointerEvent {
                panel: "scene-input".into(),
                event: Event::PointerMoved(Pos2::new(90.0, 100.0)),
                captured: true,
            })
        );
        let gone = adapter.route(&frame, &Event::PointerGone, 7, 3).unwrap();
        assert_eq!(gone.cancelled, Some("scene-input".into()));
        assert!(gone.routed.is_none());
        assert_eq!(adapter.cancel(), None);
    }

    #[test]
    fn scene_adapter_events_activate_a_panel_owned_egui_button() {
        let frame = translated_scene();
        let mut adapter = SceneEguiPointerAdapter::default();
        let point = Pos2::new(28.0, 36.0);
        let hover = adapter
            .route(&frame, &Event::PointerMoved(point), 7, 3)
            .unwrap()
            .routed
            .unwrap();
        let press = adapter
            .route(
                &frame,
                &Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
                7,
                3,
            )
            .unwrap()
            .routed
            .unwrap();
        let release = adapter
            .route(
                &frame,
                &Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                },
                7,
                3,
            )
            .unwrap()
            .routed
            .unwrap();
        assert_eq!(press.panel, "scene-input".into());
        assert_eq!(release.panel, "scene-input".into());

        let mut contexts = crate::panel_contexts::SceneEguiPanelContexts::default();
        let target = hover.panel;
        let decoy = PanelId::from("other-panel");
        let mut clicked = false;
        let mut decoy_clicked = false;
        for event in [hover.event, press.event, release.event] {
            let mut output = contexts.run_panel_ui(
                &target,
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(80.0, 60.0))),
                    events: vec![event],
                    ..Default::default()
                },
                |ui: &mut egui::Ui| {
                    clicked |= ui
                        .add_sized(Vec2::new(60.0, 28.0), Button::new("Panel action"))
                        .clicked();
                },
            );
            output.textures_delta.clear();
            let mut output = contexts.run_panel_ui(
                &decoy,
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(80.0, 60.0))),
                    ..Default::default()
                },
                |ui: &mut egui::Ui| {
                    decoy_clicked |= ui
                        .add_sized(Vec2::new(60.0, 28.0), Button::new("Panel action"))
                        .clicked();
                },
            );
            output.textures_delta.clear();
        }
        assert!(
            clicked,
            "routed panel-local press/release should activate its widget"
        );
        assert!(
            !decoy_clicked,
            "unrouted panel should not receive the click"
        );
        assert_eq!(contexts.len(), 2);
    }

    #[test]
    fn focused_input_adapter_routes_keyboard_text_and_clipboard_events() {
        let frame = translated_scene();
        let adapter = SceneEguiFocusedInputAdapter;
        let panel = PanelId::from("scene-input");
        let events = [
            Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            },
            Event::Text("type here".into()),
            Event::Ime(egui::ImeEvent::Commit("composition".into())),
            Event::Paste("pasted text".into()),
            Event::Copy,
            Event::Cut,
        ];
        for event in events {
            assert_eq!(
                adapter.route(&frame, &panel, &event, 7, 3).unwrap(),
                Some(ScenePanelFocusedInput {
                    panel: panel.clone(),
                    event,
                })
            );
        }
        assert_eq!(
            adapter
                .route(
                    &frame,
                    &panel,
                    &Event::PointerMoved(Pos2::new(15.0, 26.0)),
                    7,
                    3
                )
                .unwrap(),
            None,
            "pointer input stays on the pointer adapter"
        );
    }

    #[test]
    fn focused_input_adapter_rejects_stale_and_hidden_targets() {
        let adapter = SceneEguiFocusedInputAdapter;
        let frame = translated_scene();
        let panel = PanelId::from("scene-input");
        let key = Event::Key {
            key: egui::Key::Enter,
            physical_key: Some(egui::Key::Enter),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        };
        assert_eq!(
            adapter.route(&frame, &panel, &key, 8, 3).unwrap_err().code,
            doofusdock_core::ErrorCode::Conflict
        );

        let mut hidden = SceneNode::panel(
            "hidden-panel",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 40.0,
                height: 30.0,
            },
        );
        hidden.visible = false;
        let hidden_frame = SceneFrame::build(&hidden, 7, 3).unwrap();
        assert_eq!(
            adapter
                .route(&hidden_frame, &PanelId::from("hidden-panel"), &key, 7, 3)
                .unwrap_err()
                .code,
            doofusdock_core::ErrorCode::NotFound
        );
    }

    #[test]
    fn focused_input_adapter_text_reaches_the_panel_owned_egui_context() {
        let frame = translated_scene();
        let adapter = SceneEguiFocusedInputAdapter;
        let panel = PanelId::from("scene-input");
        let other_panel = PanelId::from("other-panel");
        let mut contexts = crate::panel_contexts::SceneEguiPanelContexts::default();
        let mut value = String::new();
        let mut other_value = String::new();
        let screen_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(80.0, 60.0));

        let mut initial = contexts.run_panel_ui(
            &panel,
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui: &mut egui::Ui| {
                ui.text_edit_singleline(&mut value).request_focus();
            },
        );
        initial.textures_delta.clear();
        let mut initial = contexts.run_panel_ui(
            &other_panel,
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui: &mut egui::Ui| {
                ui.text_edit_singleline(&mut other_value);
            },
        );
        initial.textures_delta.clear();

        let routed = adapter
            .route(&frame, &panel, &Event::Text("panel input".into()), 7, 3)
            .unwrap()
            .unwrap();
        let mut result = contexts.run_panel_ui(
            &routed.panel,
            RawInput {
                screen_rect: Some(screen_rect),
                events: vec![routed.event],
                ..Default::default()
            },
            |ui: &mut egui::Ui| {
                ui.text_edit_singleline(&mut value).request_focus();
            },
        );
        result.textures_delta.clear();

        assert_eq!(value, "panel input");
        assert!(other_value.is_empty());
    }

    #[test]
    fn scroll_adapter_targets_the_front_panel_and_maps_pointer_local() {
        let frame = translated_scene();
        let adapter = SceneEguiScrollAdapter;
        let event = Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(12.0, -8.0),
            phase: TouchPhase::Move,
            source: egui::MouseWheelSource::Trackpad,
            modifiers: Modifiers::SHIFT,
        };

        let routed = adapter
            .route(&frame, Pos2::new(15.0, 26.0), &event, 7, 3)
            .unwrap()
            .unwrap();
        assert_eq!(routed.panel, "scene-input".into());
        assert_eq!(routed.pointer_position, Pos2::new(5.0, 6.0));
        assert_eq!(routed.event, event);
    }

    #[test]
    fn routed_scroll_event_changes_a_panel_owned_egui_scroll_area() {
        let frame = translated_scene();
        let adapter = SceneEguiScrollAdapter;
        let event = Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -24.0),
            phase: TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::default(),
        };
        let routed = adapter
            .route(&frame, Pos2::new(15.0, 26.0), &event, 7, 3)
            .unwrap()
            .unwrap();
        let mut contexts = crate::panel_contexts::SceneEguiPanelContexts::default();
        let other_panel = PanelId::from("other-panel");
        let screen_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(40.0, 30.0));
        let draw_panel = |ui: &mut egui::Ui| {
            egui::ScrollArea::vertical()
                .id_salt("scene-panel-scroll")
                .max_height(18.0)
                .show(ui, |ui| {
                    for row in 0..20 {
                        ui.label(format!("row {row}"));
                    }
                })
                .state
                .clamped_offset()
                .y
        };
        let mut offset = 0.0;
        let mut initial = contexts.run_panel_ui(
            &routed.panel,
            RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                ..Default::default()
            },
            |ui: &mut egui::Ui| offset = draw_panel(ui),
        );
        initial.textures_delta.clear();
        assert_eq!(offset, 0.0);
        let mut other_offset = -1.0;
        let mut initial = contexts.run_panel_ui(
            &other_panel,
            RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                ..Default::default()
            },
            |ui: &mut egui::Ui| other_offset = draw_panel(ui),
        );
        initial.textures_delta.clear();
        assert_eq!(other_offset, 0.0);

        let mut scrolled = contexts.run_panel_ui(
            &routed.panel,
            RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![Event::PointerMoved(routed.pointer_position), routed.event],
                ..Default::default()
            },
            |ui: &mut egui::Ui| offset = draw_panel(ui),
        );
        scrolled.textures_delta.clear();
        assert!(
            offset > 0.0,
            "routed wheel event should scroll the panel widget"
        );
        let mut other = contexts.run_panel_ui(
            &other_panel,
            RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                ..Default::default()
            },
            |ui: &mut egui::Ui| other_offset = draw_panel(ui),
        );
        other.textures_delta.clear();
        assert_eq!(
            other_offset, 0.0,
            "other panel scroll state must be untouched"
        );
    }

    #[test]
    fn scroll_adapter_applies_shape_fallthrough_and_fails_closed_on_stale_frames() {
        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        });
        let lower = SceneNode::panel(
            "lower",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );
        let mut upper = SceneNode::panel(
            "upper",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );
        upper.z_order = 1;
        root.children = vec![lower, upper];
        let frame = SceneFrame::build(&root, 7, 3).unwrap();
        let adapter = SceneEguiScrollAdapter;
        let event = Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: Vec2::new(0.0, 1.0),
            phase: TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::default(),
        };
        let mut shapes = BTreeMap::new();
        shapes.insert(
            PanelId::from("upper"),
            ValidatedShape2D::new(doofusdock_presentation::Shape2D::Circle {
                center: [50.0, 50.0],
                radius: 5.0,
            })
            .unwrap(),
        );

        let routed = adapter
            .route_with_shapes(&frame, Pos2::new(10.0, 10.0), &event, 7, 3, &shapes)
            .unwrap()
            .unwrap();
        assert_eq!(routed.panel, "lower".into());
        assert_eq!(routed.pointer_position, Pos2::new(10.0, 10.0));
        assert_eq!(routed.event, event);
        assert_eq!(
            adapter
                .route(&frame, Pos2::new(10.0, 10.0), &event, 8, 3)
                .unwrap_err()
                .code,
            doofusdock_core::ErrorCode::Conflict
        );
    }

    #[test]
    fn single_touch_becomes_a_click_and_preserves_touch_events() {
        let mut adapter = TouchPointerAdapter::default();
        let start = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Start, 12.0, 24.0)],
        );
        assert!(matches!(
            start[0],
            Event::Touch {
                phase: TouchPhase::Start,
                ..
            }
        ));
        assert!(matches!(start[1], Event::PointerMoved(pos) if pos == Pos2::new(12.0, 24.0)));
        assert!(matches!(
            start[2],
            Event::PointerButton {
                pressed: true,
                button: PointerButton::Primary,
                ..
            }
        ));

        let moved = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Move, 18.0, 30.0)],
        );
        assert!(matches!(
            moved[0],
            Event::Touch {
                phase: TouchPhase::Move,
                ..
            }
        ));
        assert!(matches!(moved[1], Event::PointerMoved(pos) if pos == Pos2::new(18.0, 30.0)));

        let end = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::End, 18.0, 30.0)],
        );
        assert!(matches!(
            end[0],
            Event::Touch {
                phase: TouchPhase::End,
                ..
            }
        ));
        assert!(
            matches!(end[2], Event::PointerButton { pressed: false, pos, .. } if pos == Pos2::new(18.0, 30.0))
        );
        assert!(matches!(end[3], Event::PointerGone));
    }

    #[test]
    fn second_contact_cancels_press_and_suppresses_until_all_lift() {
        let mut adapter = TouchPointerAdapter::default();
        normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Start, 10.0, 10.0)],
        );
        let second = normalize(
            &mut adapter,
            vec![touch(SECOND, TouchPhase::Start, 40.0, 40.0)],
        );
        assert!(
            matches!(second[1], Event::PointerButton { pressed: false, pos, .. } if pos.x < -1000.0)
        );
        assert!(matches!(second[2], Event::PointerGone));

        let move_after_second = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Move, 11.0, 11.0)],
        );
        assert_eq!(move_after_second.len(), 1);
        normalize(
            &mut adapter,
            vec![touch(SECOND, TouchPhase::End, 40.0, 40.0)],
        );
        let end = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::End, 11.0, 11.0)],
        );
        assert_eq!(end.len(), 1);

        let fresh = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Start, 5.0, 6.0)],
        );
        assert_eq!(fresh.len(), 3);
    }

    #[test]
    fn cancel_and_focus_loss_release_without_synthesizing_a_click() {
        let mut adapter = TouchPointerAdapter::default();
        normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Start, 10.0, 10.0)],
        );
        let canceled = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Cancel, 10.0, 10.0)],
        );
        assert!(
            matches!(canceled[1], Event::PointerButton { pressed: false, pos, .. } if pos.x < -1000.0)
        );
        assert!(matches!(canceled[2], Event::PointerGone));

        normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Start, 10.0, 10.0)],
        );
        let mut lost_focus = RawInput {
            focused: false,
            ..Default::default()
        };
        adapter.normalize(&mut lost_focus);
        assert!(matches!(
            lost_focus.events[0],
            Event::PointerButton { pressed: false, .. }
        ));
        assert!(matches!(lost_focus.events[1], Event::PointerGone));

        let fresh = normalize(
            &mut adapter,
            vec![touch(FIRST, TouchPhase::Start, 5.0, 6.0)],
        );
        assert_eq!(fresh.len(), 3);
    }

    #[test]
    fn normalized_touch_activates_a_regular_egui_button() {
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(320.0, 240.0));
        let mut adapter = TouchPointerAdapter::default();
        let mut button_center = Pos2::ZERO;
        let mut clicked = false;

        let mut initial = RawInput {
            screen_rect: Some(screen),
            time: Some(0.0),
            ..Default::default()
        };
        adapter.normalize(&mut initial);
        let mut output = ctx.run_ui(initial, |ui| {
            let response = ui.button("Touch target");
            button_center = response.rect.center();
        });
        output.textures_delta.clear();

        let mut press = RawInput {
            screen_rect: Some(screen),
            time: Some(0.02),
            events: vec![touch(
                FIRST,
                TouchPhase::Start,
                button_center.x,
                button_center.y,
            )],
            ..Default::default()
        };
        adapter.normalize(&mut press);
        let mut output = ctx.run_ui(press, |ui| {
            let _ = ui.button("Touch target");
        });
        output.textures_delta.clear();

        let mut release = RawInput {
            screen_rect: Some(screen),
            time: Some(0.04),
            events: vec![touch(
                FIRST,
                TouchPhase::End,
                button_center.x,
                button_center.y,
            )],
            ..Default::default()
        };
        adapter.normalize(&mut release);
        let mut output = ctx.run_ui(release, |ui| {
            clicked = ui.button("Touch target").clicked();
        });
        output.textures_delta.clear();
        assert!(
            clicked,
            "a normal egui button should register the touch click"
        );
    }
}
