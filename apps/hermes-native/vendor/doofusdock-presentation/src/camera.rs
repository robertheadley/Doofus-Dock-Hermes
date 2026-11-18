//! Derived workspace camera and overview projection helpers.
//!
//! Camera state belongs to the host/view layer. These projections never edit
//! canonical layout and retain panel IDs so hosts can resolve an overview
//! activation through their normal revision-checked command path.
use crate::{Box2, Snapshot};
use doofusdock_core::{DockError, DockResult, ErrorCode, PanelId};
use std::collections::BTreeMap;

const MIN_ZOOM: f64 = 0.05;
const MAX_ZOOM: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkspaceCamera {
    /// Visible workspace-space rectangle. Width and height determine zoom.
    pub view: Box2,
}

impl WorkspaceCamera {
    pub fn new(view: Box2) -> DockResult<Self> {
        validate_rect(view, "camera-view")?;
        Ok(Self { view })
    }

    /// Map workspace coordinates into a viewport while preserving aspect.
    pub fn project(self, source: Box2, viewport: Box2) -> DockResult<Box2> {
        validate_rect(self.view, "camera-view")?;
        validate_rect(source, "camera-source")?;
        validate_rect(viewport, "camera-viewport")?;
        let sx = viewport.w / self.view.w;
        let sy = viewport.h / self.view.h;
        let scale = sx.min(sy);
        let width = source.w * scale;
        let height = source.h * scale;
        let x = viewport.x
            + (source.x - self.view.x) * scale
            + (viewport.w - self.view.w * scale) * 0.5;
        let y = viewport.y
            + (source.y - self.view.y) * scale
            + (viewport.h - self.view.h * scale) * 0.5;
        Ok(Box2 {
            x,
            y,
            w: width,
            h: height,
        })
    }

    /// Invert a viewport point into workspace coordinates.
    pub fn unproject_point(self, point: (f64, f64), viewport: Box2) -> DockResult<(f64, f64)> {
        validate_rect(self.view, "camera-view")?;
        validate_rect(viewport, "camera-viewport")?;
        if !point.0.is_finite() || !point.1.is_finite() {
            return Err(camera_error("Camera point is invalid.", "point"));
        }
        let scale = (viewport.w / self.view.w).min(viewport.h / self.view.h);
        let offset_x = (viewport.w - self.view.w * scale) * 0.5;
        let offset_y = (viewport.h - self.view.h * scale) * 0.5;
        Ok((
            self.view.x + (point.0 - viewport.x - offset_x) / scale,
            self.view.y + (point.1 - viewport.y - offset_y) / scale,
        ))
    }

    pub fn pan(self, delta_x: f64, delta_y: f64) -> DockResult<Self> {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return Err(camera_error("Camera pan delta must be finite.", "pan"));
        }
        Self::new(Box2 {
            x: self.view.x + delta_x,
            y: self.view.y + delta_y,
            ..self.view
        })
    }

    /// Zoom around a viewport-space anchor while keeping the anchored world
    /// point fixed. Scale is bounded to avoid unusable or unbounded views.
    pub fn zoom_at(self, factor: f64, anchor: (f64, f64), viewport: Box2) -> DockResult<Self> {
        validate_rect(self.view, "camera-view")?;
        if !factor.is_finite() || factor <= 0.0 {
            return Err(camera_error(
                "Camera zoom factor must be positive and finite.",
                "zoom",
            ));
        }
        let world = self.unproject_point(anchor, viewport)?;
        let current = (viewport.w / self.view.w).min(viewport.h / self.view.h);
        let target = (current * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let ratio = current / target;
        let width = self.view.w * ratio;
        let height = self.view.h * ratio;
        let view_aspect = viewport.w / viewport.h;
        let (width, height) = if width / height > view_aspect {
            (width, width / view_aspect)
        } else {
            (height * view_aspect, height)
        };
        let anchor_x = (anchor.0 - viewport.x) / viewport.w;
        let anchor_y = (anchor.1 - viewport.y) / viewport.h;
        Self::new(Box2 {
            x: world.0 - anchor_x * width,
            y: world.1 - anchor_y * height,
            w: width,
            h: height,
        })
    }

    pub fn fit(bounds: Box2, viewport: Box2, padding: f64) -> DockResult<Self> {
        validate_rect(bounds, "camera-bounds")?;
        validate_rect(viewport, "camera-viewport")?;
        if !padding.is_finite() || !(0.0..=0.45).contains(&padding) {
            return Err(camera_error(
                "Camera fit padding is outside 0..=0.45.",
                "fit",
            ));
        }
        let width = bounds.w.max(1.0) / (1.0 - 2.0 * padding);
        let height = bounds.h.max(1.0) / (1.0 - 2.0 * padding);
        let aspect = viewport.w / viewport.h;
        let (width, height) = if width / height > aspect {
            (width, width / aspect)
        } else {
            (height * aspect, height)
        };
        let cx = bounds.x + bounds.w * 0.5;
        let cy = bounds.y + bounds.h * 0.5;
        Self::new(Box2 {
            x: cx - width * 0.5,
            y: cy - height * 0.5,
            w: width,
            h: height,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverviewProjection {
    pub viewport: Box2,
    pub panels: BTreeMap<PanelId, Box2>,
}

impl OverviewProjection {
    /// Fit all visible panel bounds into an overview viewport and retain a
    /// stable panel-ID hit map. Invisible panels are omitted.
    pub fn build(snapshot: &Snapshot, viewport: Box2, padding: f64) -> DockResult<Self> {
        validate_rect(viewport, "overview-viewport")?;
        if !padding.is_finite() || !(0.0..=0.45).contains(&padding) {
            return Err(camera_error(
                "Overview padding is outside 0..=0.45.",
                "overview",
            ));
        }
        let mut visible = snapshot.panels.iter().filter(|(_, visual)| {
            visual.opacity >= 0.5 && visual.bounds.w > 0.0 && visual.bounds.h > 0.0
        });
        let Some((_, first)) = visible.next() else {
            return Ok(Self {
                viewport,
                panels: BTreeMap::new(),
            });
        };
        validate_rect(first.bounds, "overview-panel")?;
        let mut bounds = first.bounds;
        for (_, panel) in visible {
            validate_rect(panel.bounds, "overview-panel")?;
            let right = bounds.x + bounds.w;
            let bottom = bounds.y + bounds.h;
            let next_right = (panel.bounds.x + panel.bounds.w).max(right);
            let next_bottom = (panel.bounds.y + panel.bounds.h).max(bottom);
            bounds.x = bounds.x.min(panel.bounds.x);
            bounds.y = bounds.y.min(panel.bounds.y);
            bounds.w = next_right - bounds.x;
            bounds.h = next_bottom - bounds.y;
        }
        let camera = WorkspaceCamera::fit(bounds, viewport, padding)?;
        let panels = snapshot
            .panels
            .iter()
            .filter(|(_, visual)| visual.opacity >= 0.5)
            .map(|(id, visual)| Ok((id.clone(), camera.project(visual.bounds, viewport)?)))
            .collect::<DockResult<BTreeMap<_, _>>>()?;
        Ok(Self { viewport, panels })
    }

    pub fn hit_test(&self, x: f64, y: f64) -> Option<PanelId> {
        if !x.is_finite() || !y.is_finite() || !self.viewport.contains(x, y) {
            return None;
        }
        self.panels
            .iter()
            .rev()
            .find(|(_, bounds)| bounds.contains(x, y))
            .map(|(id, _)| id.clone())
    }
}

fn validate_rect(rect: Box2, context: &str) -> DockResult<()> {
    if !rect.valid() || rect.w <= 0.0 || rect.h <= 0.0 {
        return Err(camera_error(
            "Camera rectangle must have positive finite bounds.",
            context,
        ));
    }
    Ok(())
}

fn camera_error(message: &str, context: &str) -> DockError {
    DockError::new(ErrorCode::SchemaError, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Visual;

    #[test]
    fn camera_fit_pan_zoom_and_inverse_are_deterministic() {
        let viewport = Box2 {
            x: 10.0,
            y: 20.0,
            w: 800.0,
            h: 600.0,
        };
        let bounds = Box2 {
            x: -100.0,
            y: 40.0,
            w: 1200.0,
            h: 500.0,
        };
        let camera = WorkspaceCamera::fit(bounds, viewport, 0.1).unwrap();
        let projected = camera.project(bounds, viewport).unwrap();
        assert!(projected.x >= viewport.x && projected.y >= viewport.y);
        assert!(projected.x + projected.w <= viewport.x + viewport.w + 1e-9);
        assert!(projected.y + projected.h <= viewport.y + viewport.h + 1e-9);
        let p = (315.0, 280.0);
        let world = camera.unproject_point(p, viewport).unwrap();
        let zoomed = camera.zoom_at(1.7, p, viewport).unwrap();
        let world_after = zoomed.unproject_point(p, viewport).unwrap();
        assert!((world.0 - world_after.0).abs() < 1e-8);
        assert!((world.1 - world_after.1).abs() < 1e-8);
        assert_eq!(camera.pan(10.0, -5.0).unwrap().view.x, camera.view.x + 10.0);
    }

    #[test]
    fn overview_projects_visible_panels_and_maps_clicks_back_to_identity() {
        let mut snapshot = Snapshot::default();
        snapshot.panels.insert(
            PanelId::from("a"),
            Visual {
                bounds: Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 200.0,
                    h: 100.0,
                },
                clip: Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 200.0,
                    h: 100.0,
                },
                opacity: 1.0,
                z: 0,
            },
        );
        snapshot.panels.insert(
            PanelId::from("hidden"),
            Visual {
                bounds: Box2 {
                    x: 300.0,
                    y: 0.0,
                    w: 200.0,
                    h: 100.0,
                },
                clip: Box2 {
                    x: 300.0,
                    y: 0.0,
                    w: 200.0,
                    h: 100.0,
                },
                opacity: 0.0,
                z: 1,
            },
        );
        let overview = OverviewProjection::build(
            &snapshot,
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 300.0,
            },
            0.1,
        )
        .unwrap();
        assert_eq!(overview.panels.len(), 1);
        let rect = overview.panels.get(&PanelId::from("a")).unwrap();
        assert_eq!(
            overview.hit_test(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
            Some(PanelId::from("a"))
        );
        assert_eq!(overview.hit_test(-1.0, -1.0), None);
    }

    #[test]
    fn camera_and_overview_reject_invalid_bounds_and_padding() {
        assert!(
            WorkspaceCamera::new(Box2 {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 4.0
            })
            .is_err()
        );
        assert!(
            WorkspaceCamera::fit(
                Box2::default(),
                Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 10.0,
                    h: 10.0
                },
                0.5
            )
            .is_err()
        );
        assert!(
            WorkspaceCamera::new(Box2 {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0
            })
            .unwrap()
            .zoom_at(
                f64::NAN,
                (0.0, 0.0),
                Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 10.0,
                    h: 10.0
                }
            )
            .is_err()
        );
    }
}
