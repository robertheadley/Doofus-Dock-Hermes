//! Deterministic perspective-projected cube-face transitions.
//!
//! This module computes a transient two-face presentation frame. Hosts own
//! texture snapshots, input/focus handoff, timing, interruption policy, and the
//! authoritative layout; sampling never mutates canonical state.
use crate::Box2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CubeRotationAxis {
    /// Rotate around the vertical axis; the next face enters from the left or right.
    Horizontal,
    /// Rotate around the horizontal axis; the next face enters from the top or bottom.
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CubeRotationDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CubeFaceKind {
    Outgoing,
    Incoming,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CubePoint2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CubeVertex {
    pub position: CubePoint2,
    pub uv: [f32; 2],
    /// Camera-space depth used for painter ordering; larger values are nearer.
    pub depth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CubeProjectedFace {
    pub kind: CubeFaceKind,
    pub vertices: [CubeVertex; 4],
    pub visible: bool,
    pub mean_depth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CubeTransitionFrame {
    pub progress: f64,
    pub bounds: Box2,
    pub outgoing: CubeProjectedFace,
    pub incoming: CubeProjectedFace,
    /// Draw back-to-front in this order. Invisible faces are omitted.
    pub draw_order: Vec<CubeFaceKind>,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CubeTransition3D {
    from_bounds: Box2,
    to_bounds: Box2,
    axis: CubeRotationAxis,
    direction: CubeRotationDirection,
    camera_distance: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeTransitionError {
    InvalidBounds,
    InvalidProgress,
    InvalidPerspective,
}

impl CubeTransition3D {
    pub fn new(
        from_bounds: Box2,
        to_bounds: Box2,
        axis: CubeRotationAxis,
        direction: CubeRotationDirection,
    ) -> Result<Self, CubeTransitionError> {
        if !valid_bounds(from_bounds) || !valid_bounds(to_bounds) {
            return Err(CubeTransitionError::InvalidBounds);
        }
        Ok(Self {
            from_bounds,
            to_bounds,
            axis,
            direction,
            camera_distance: None,
        })
    }

    /// Set the camera distance from the cube center. It must exceed half the
    /// largest endpoint dimension so no face reaches or crosses the camera.
    pub fn with_camera_distance(mut self, distance: f64) -> Result<Self, CubeTransitionError> {
        let largest = self
            .from_bounds
            .w
            .max(self.from_bounds.h)
            .max(self.to_bounds.w)
            .max(self.to_bounds.h);
        if !distance.is_finite() || distance <= largest * 0.5 + f64::EPSILON {
            return Err(CubeTransitionError::InvalidPerspective);
        }
        self.camera_distance = Some(distance);
        Ok(self)
    }

    /// Sample any normalized point on the transition. Sampling is stateless,
    /// so hosts may reverse by sampling progress downward or retarget from a
    /// captured frame without changing the canonical layout.
    pub fn sample(&self, progress: f64) -> Result<CubeTransitionFrame, CubeTransitionError> {
        if !progress.is_finite() || !(0.0..=1.0).contains(&progress) {
            return Err(CubeTransitionError::InvalidProgress);
        }
        let bounds = self.from_bounds.mix(self.to_bounds, progress);
        let width = bounds.w;
        let height = bounds.h;
        let span = match self.axis {
            CubeRotationAxis::Horizontal => width,
            CubeRotationAxis::Vertical => height,
        };
        let camera = self.camera_distance.unwrap_or_else(|| {
            self.from_bounds
                .w
                .max(self.from_bounds.h)
                .max(self.to_bounds.w)
                .max(self.to_bounds.h)
                * 4.0
        });
        let depth = span;
        if !camera.is_finite() || camera <= depth * 0.5 + f64::EPSILON {
            return Err(CubeTransitionError::InvalidPerspective);
        }
        let focal = camera - depth * 0.5;
        let sign = if self.direction == CubeRotationDirection::Forward {
            1.0
        } else {
            -1.0
        };
        let theta = std::f64::consts::FRAC_PI_2 * progress;
        let angle = match self.axis {
            CubeRotationAxis::Horizontal => -sign * theta,
            CubeRotationAxis::Vertical => sign * theta,
        };
        let center = CubePoint2 {
            x: bounds.x + bounds.w * 0.5,
            y: bounds.y + bounds.h * 0.5,
        };
        let outgoing_points = match self.axis {
            CubeRotationAxis::Horizontal => [
                Point3::new(-width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, -height / 2.0, depth / 2.0),
                Point3::new(-width / 2.0, -height / 2.0, depth / 2.0),
            ],
            CubeRotationAxis::Vertical => [
                Point3::new(-width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, -height / 2.0, depth / 2.0),
                Point3::new(-width / 2.0, -height / 2.0, depth / 2.0),
            ],
        };
        let incoming_points = match (self.axis, self.direction) {
            (CubeRotationAxis::Horizontal, CubeRotationDirection::Forward) => [
                Point3::new(width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, height / 2.0, -depth / 2.0),
                Point3::new(width / 2.0, -height / 2.0, -depth / 2.0),
                Point3::new(width / 2.0, -height / 2.0, depth / 2.0),
            ],
            (CubeRotationAxis::Horizontal, CubeRotationDirection::Reverse) => [
                Point3::new(-width / 2.0, height / 2.0, -depth / 2.0),
                Point3::new(-width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(-width / 2.0, -height / 2.0, depth / 2.0),
                Point3::new(-width / 2.0, -height / 2.0, -depth / 2.0),
            ],
            (CubeRotationAxis::Vertical, CubeRotationDirection::Forward) => [
                Point3::new(-width / 2.0, height / 2.0, -depth / 2.0),
                Point3::new(width / 2.0, height / 2.0, -depth / 2.0),
                Point3::new(width / 2.0, height / 2.0, depth / 2.0),
                Point3::new(-width / 2.0, height / 2.0, depth / 2.0),
            ],
            (CubeRotationAxis::Vertical, CubeRotationDirection::Reverse) => [
                Point3::new(-width / 2.0, -height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, -height / 2.0, depth / 2.0),
                Point3::new(width / 2.0, -height / 2.0, -depth / 2.0),
                Point3::new(-width / 2.0, -height / 2.0, -depth / 2.0),
            ],
        };
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let outgoing_normal = Point3::new(0.0, 0.0, 1.0);
        let incoming_normal = match (self.axis, self.direction) {
            (CubeRotationAxis::Horizontal, CubeRotationDirection::Forward) => {
                Point3::new(1.0, 0.0, 0.0)
            }
            (CubeRotationAxis::Horizontal, CubeRotationDirection::Reverse) => {
                Point3::new(-1.0, 0.0, 0.0)
            }
            (CubeRotationAxis::Vertical, CubeRotationDirection::Forward) => {
                Point3::new(0.0, 1.0, 0.0)
            }
            (CubeRotationAxis::Vertical, CubeRotationDirection::Reverse) => {
                Point3::new(0.0, -1.0, 0.0)
            }
        };
        let projection = ProjectionCamera {
            angle,
            axis: self.axis,
            center,
            distance: camera,
            focal,
        };
        let outgoing = project_face(
            CubeFaceKind::Outgoing,
            outgoing_points,
            uv,
            outgoing_normal,
            projection,
        )?;
        // Vertex order is already top-left, top-right, bottom-right,
        // bottom-left after the corresponding face reaches the front.
        let incoming_uv = uv;
        let incoming = project_face(
            CubeFaceKind::Incoming,
            incoming_points,
            incoming_uv,
            incoming_normal,
            projection,
        )?;
        let mut draw_order = [outgoing.clone(), incoming.clone()]
            .into_iter()
            .filter(|face| face.visible)
            .collect::<Vec<_>>();
        draw_order.sort_by(|a, b| a.mean_depth.total_cmp(&b.mean_depth));
        Ok(CubeTransitionFrame {
            progress,
            bounds,
            outgoing,
            incoming,
            draw_order: draw_order.into_iter().map(|face| face.kind).collect(),
            complete: progress >= 1.0,
        })
    }
}

fn valid_bounds(bounds: Box2) -> bool {
    bounds.valid() && bounds.w > 0.0 && bounds.h > 0.0
}

#[derive(Clone, Copy)]
struct Point3 {
    x: f64,
    y: f64,
    z: f64,
}
impl Point3 {
    const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    fn rotate(self, angle: f64, axis: CubeRotationAxis) -> Self {
        let (sin, cos) = angle.sin_cos();
        match axis {
            CubeRotationAxis::Horizontal => Self {
                x: self.x * cos + self.z * sin,
                y: self.y,
                z: -self.x * sin + self.z * cos,
            },
            CubeRotationAxis::Vertical => Self {
                x: self.x,
                y: self.y * cos - self.z * sin,
                z: self.y * sin + self.z * cos,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct ProjectionCamera {
    angle: f64,
    axis: CubeRotationAxis,
    center: CubePoint2,
    distance: f64,
    focal: f64,
}

fn project_face(
    kind: CubeFaceKind,
    points: [Point3; 4],
    uv: [[f32; 2]; 4],
    normal: Point3,
    camera: ProjectionCamera,
) -> Result<CubeProjectedFace, CubeTransitionError> {
    let rotated_normal = normal.rotate(camera.angle, camera.axis);
    let mut total_depth = 0.0;
    let mut vertices = [CubeVertex::default(); 4];
    for (index, point) in points.into_iter().enumerate() {
        let point = point.rotate(camera.angle, camera.axis);
        let denominator = camera.distance - point.z;
        if !denominator.is_finite() || denominator <= f64::EPSILON {
            return Err(CubeTransitionError::InvalidPerspective);
        }
        let scale = camera.focal / denominator;
        let position = CubePoint2 {
            x: camera.center.x + point.x * scale,
            y: camera.center.y - point.y * scale,
        };
        if !position.x.is_finite()
            || !position.y.is_finite()
            || position.x.abs() > 1_000_000.0
            || position.y.abs() > 1_000_000.0
        {
            return Err(CubeTransitionError::InvalidPerspective);
        }
        total_depth += point.z;
        vertices[index] = CubeVertex {
            position,
            uv: uv[index],
            depth: point.z,
        };
    }
    Ok(CubeProjectedFace {
        kind,
        vertices,
        visible: rotated_normal.z > 1e-8,
        mean_depth: total_depth / 4.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_faces_reach_outgoing_and_incoming_endpoints_with_dimension_changes() {
        let transition = CubeTransition3D::new(
            Box2 {
                x: 10.0,
                y: 20.0,
                w: 200.0,
                h: 100.0,
            },
            Box2 {
                x: 30.0,
                y: 40.0,
                w: 300.0,
                h: 160.0,
            },
            CubeRotationAxis::Horizontal,
            CubeRotationDirection::Forward,
        )
        .unwrap();
        let start = transition.sample(0.0).unwrap();
        let end = transition.sample(1.0).unwrap();
        assert!(start.outgoing.visible && !start.incoming.visible);
        assert!(!end.outgoing.visible && end.incoming.visible);
        assert_eq!(start.bounds, transition.from_bounds);
        assert_eq!(end.bounds, transition.to_bounds);
        assert!(!start.complete && end.complete);
        let xs = end.incoming.vertices.map(|v| v.position.x);
        let width = xs.into_iter().fold(f64::NEG_INFINITY, f64::max)
            - xs.into_iter().fold(f64::INFINITY, f64::min);
        assert!((width - 300.0).abs() < 1e-6);
        assert!(transition.sample(0.5).unwrap().draw_order.len() == 2);
    }

    #[test]
    fn both_axes_and_directions_are_repeatable_reversible_and_bounded() {
        for axis in [CubeRotationAxis::Horizontal, CubeRotationAxis::Vertical] {
            for direction in [
                CubeRotationDirection::Forward,
                CubeRotationDirection::Reverse,
            ] {
                let transition = CubeTransition3D::new(
                    Box2 {
                        x: 0.0,
                        y: 0.0,
                        w: 120.0,
                        h: 80.0,
                    },
                    Box2 {
                        x: 5.0,
                        y: -4.0,
                        w: 80.0,
                        h: 120.0,
                    },
                    axis,
                    direction,
                )
                .unwrap();
                assert_eq!(
                    transition.sample(0.37).unwrap(),
                    transition.sample(0.37).unwrap()
                );
                assert!(transition.sample(0.0).unwrap().outgoing.visible);
                assert!(transition.sample(1.0).unwrap().incoming.visible);
            }
        }
        let t = CubeTransition3D::new(
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
            CubeRotationAxis::Horizontal,
            CubeRotationDirection::Forward,
        )
        .unwrap();
        assert_eq!(t.sample(-0.1), Err(CubeTransitionError::InvalidProgress));
        assert_eq!(
            t.sample(f64::NAN),
            Err(CubeTransitionError::InvalidProgress)
        );
        assert!(t.clone().with_camera_distance(10.0).is_err());
    }
}
