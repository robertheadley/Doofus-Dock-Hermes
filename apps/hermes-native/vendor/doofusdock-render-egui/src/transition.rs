//! egui rendering adapter for toolkit-neutral, transient presentation snapshots.
use doofusdock_presentation::{Box2, Snapshot, Visual};
use egui::{Rect, Ui, UiBuilder, pos2, vec2};

/// Counts how a visual snapshot was consumed by the egui host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisualSnapshotRenderReport {
    pub rendered: usize,
    pub skipped_invisible: usize,
    pub skipped_invalid: usize,
}

/// Render a transient visual snapshot in stable back-to-front order.
///
/// Each callback receives a child UI constrained to the visual's bounds, clip,
/// and opacity. The snapshot is presentation-only: the renderer does not alter
/// canonical layout state or apply operations.
pub fn render_visual_snapshot(
    ui: &mut Ui,
    snapshot: &Snapshot,
    viewport_clip: Rect,
    render_panel: impl FnMut(&mut Ui, &str, &Visual),
) -> VisualSnapshotRenderReport {
    render_visual_snapshot_with_scratch(ui, snapshot, viewport_clip, &mut Vec::new(), render_panel)
}

/// Render using a caller-owned ordering buffer that retains capacity across
/// frames. Hosts that render animated snapshots repeatedly can avoid rebuilding
/// this temporary reference list on every frame.
pub fn render_visual_snapshot_with_scratch<'a>(
    ui: &mut Ui,
    snapshot: &'a Snapshot,
    viewport_clip: Rect,
    scratch: &mut Vec<(&'a str, &'a Visual)>,
    mut render_panel: impl FnMut(&mut Ui, &str, &Visual),
) -> VisualSnapshotRenderReport {
    scratch.clear();
    scratch.extend(
        snapshot
            .panels
            .iter()
            .map(|(id, visual)| (id.0.as_str(), visual)),
    );
    scratch.sort_by(|(left_id, left), (right_id, right)| {
        left.z.cmp(&right.z).then_with(|| left_id.cmp(right_id))
    });

    let viewport_clip = viewport_clip.intersect(ui.clip_rect());
    let mut report = VisualSnapshotRenderReport::default();
    for (id, visual) in scratch.drain(..) {
        if !visual.bounds.valid() || !visual.clip.valid() || !visual.opacity.is_finite() {
            report.skipped_invalid += 1;
            continue;
        }
        if visual.opacity <= 0.0 || visual.bounds.w == 0.0 || visual.bounds.h == 0.0 {
            report.skipped_invisible += 1;
            continue;
        }

        let bounds = to_egui_rect(visual.bounds);
        let clip = to_egui_rect(visual.clip).intersect(viewport_clip);
        if clip.width() <= 0.0 || clip.height() <= 0.0 {
            report.skipped_invisible += 1;
            continue;
        }

        ui.scope_builder(
            UiBuilder::new()
                .id_salt(("visual-snapshot-panel", id))
                .max_rect(bounds),
            |ui| {
                ui.set_clip_rect(clip);
                ui.set_opacity(visual.opacity.clamp(0.0, 1.0) as f32);
                render_panel(ui, id, visual);
            },
        );
        report.rendered += 1;
    }
    report
}

fn to_egui_rect(bounds: Box2) -> Rect {
    Rect::from_min_size(
        pos2(bounds.x as f32, bounds.y as f32),
        vec2(bounds.w as f32, bounds.h as f32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use doofusdock_core::PanelId;
    use egui::{Color32, Context, RawInput};

    fn visual(x: f64, y: f64, w: f64, h: f64, clip: Box2, opacity: f64, z: i32) -> Visual {
        Visual {
            bounds: Box2 { x, y, w, h },
            clip,
            opacity,
            z,
        }
    }

    #[test]
    fn snapshot_renderer_applies_clip_opacity_and_stable_z_order() {
        let viewport = Box2 {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 80.0,
        };
        let mut snapshot = Snapshot::default();
        snapshot.panels.insert(
            PanelId::from("z-late"),
            visual(5.0, 5.0, 50.0, 40.0, viewport, 1.0, 4),
        );
        snapshot.panels.insert(
            PanelId::from("a-early"),
            visual(
                10.0,
                10.0,
                50.0,
                40.0,
                Box2 {
                    x: 20.0,
                    y: 15.0,
                    w: 20.0,
                    h: 25.0,
                },
                0.25,
                1,
            ),
        );
        snapshot.panels.insert(
            PanelId::from("invisible"),
            visual(0.0, 0.0, 10.0, 10.0, viewport, 0.0, 2),
        );
        snapshot.panels.insert(
            PanelId::from("invalid"),
            visual(f64::NAN, 0.0, 10.0, 10.0, viewport, 1.0, 3),
        );

        let context = egui::Context::default();
        let mut seen = Vec::new();
        let mut report = None;
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0))),
                ..RawInput::default()
            },
            &mut |ui: &mut Ui| {
                report = Some(render_visual_snapshot(
                    ui,
                    &snapshot,
                    Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
                    |ui, id, _visual| {
                        seen.push((id.to_owned(), ui.clip_rect(), ui.opacity()));
                        ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::WHITE);
                    },
                ));
            },
        );
        output.textures_delta.clear();

        assert_eq!(
            report,
            Some(VisualSnapshotRenderReport {
                rendered: 2,
                skipped_invisible: 1,
                skipped_invalid: 1,
            })
        );
        assert_eq!(seen[0].0, "a-early");
        assert_eq!(
            seen[0].1,
            Rect::from_min_size(pos2(20.0, 15.0), vec2(20.0, 25.0))
        );
        assert!((seen[0].2 - 0.25).abs() < 0.001);
        assert_eq!(seen[1].0, "z-late");
        assert_eq!(seen[1].2, 1.0);
    }

    #[test]
    fn snapshot_renderer_intersects_visual_clips_with_the_host_clip() {
        let mut snapshot = Snapshot::default();
        snapshot.panels.insert(
            PanelId::from("clipped"),
            visual(
                0.0,
                0.0,
                60.0,
                60.0,
                Box2 {
                    x: 20.0,
                    y: 10.0,
                    w: 40.0,
                    h: 40.0,
                },
                1.0,
                0,
            ),
        );
        let context = egui::Context::default();
        let host_clip = Rect::from_min_size(pos2(30.0, 20.0), vec2(20.0, 30.0));
        let mut observed_clip = None;
        let mut output = context.run_ui(RawInput::default(), &mut |ui: &mut Ui| {
            render_visual_snapshot(ui, &snapshot, host_clip, |ui, _, _| {
                observed_clip = Some(ui.clip_rect());
            });
        });
        output.textures_delta.clear();
        assert_eq!(observed_clip, Some(host_clip));
    }

    #[test]
    fn scratch_renderer_retains_capacity_and_drains_the_ordering_buffer() {
        let mut snapshot = Snapshot::default();
        let bounds = Box2 {
            x: 0.0,
            y: 0.0,
            w: 40.0,
            h: 40.0,
        };
        for index in 0..8 {
            snapshot.panels.insert(
                PanelId::from(format!("panel-{index}")),
                visual(index as f64, 0.0, 20.0, 20.0, bounds, 1.0, index),
            );
        }
        let viewport = Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 40.0));
        let context = Context::default();
        let mut scratch = Vec::with_capacity(snapshot.panels.len());
        let capacity = scratch.capacity();
        let mut output = context.run_ui(RawInput::default(), &mut |ui: &mut Ui| {
            render_visual_snapshot_with_scratch(
                ui,
                &snapshot,
                viewport,
                &mut scratch,
                |_, _, _| {},
            );
        });
        output.textures_delta.clear();

        assert_eq!(scratch.capacity(), capacity);
        assert!(scratch.is_empty());
    }
}
