//! Derived two-dimensional scene transforms and inverse hit mapping.
//!
//! This presentation snapshot is never a layout authority. Hosts may use it
//! for virtualized composition and pointer mapping; canonical changes still
//! require revision-bound operations.
use crate::{Box2, Snapshot};
use doofusdock_core::{DockError, DockResult, ErrorCode, LayoutDocument, PanelId};
use std::collections::BTreeMap;

const MAX_SCENE_NODES: usize = 10_000;
const MAX_SCENE_DEPTH: usize = 128;
const MAX_SCENE_VALUE: f64 = 1.0e9;
const MAX_SURFACE_GEOMETRY_VERTICES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScenePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SceneRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl SceneRect {
    pub fn validate(self) -> DockResult<()> {
        if ![self.x, self.y, self.width, self.height]
            .into_iter()
            .all(|value| value.is_finite() && value.abs() <= MAX_SCENE_VALUE)
            || self.width <= 0.0
            || self.height <= 0.0
        {
            return Err(scene_error(
                "Scene rectangle is invalid or unbounded.",
                format!("{self:?}"),
            ));
        }
        Ok(())
    }

    fn contains(self, point: ScenePoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }
}

/// A local-to-parent affine transform represented as a 2×3 matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine2 {
    pub m11: f64,
    pub m12: f64,
    pub m21: f64,
    pub m22: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Default for Affine2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine2 {
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub fn new(m11: f64, m12: f64, m21: f64, m22: f64, tx: f64, ty: f64) -> DockResult<Self> {
        let transform = Self {
            m11,
            m12,
            m21,
            m22,
            tx,
            ty,
        };
        transform.validate()?;
        Ok(transform)
    }

    pub fn translate(x: f64, y: f64) -> DockResult<Self> {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    pub fn scale_rotate(
        scale_x: f64,
        scale_y: f64,
        rotation_radians: f64,
        origin: ScenePoint,
    ) -> DockResult<Self> {
        if !scale_x.is_finite()
            || !scale_y.is_finite()
            || scale_x.abs() <= f64::EPSILON
            || scale_y.abs() <= f64::EPSILON
            || !rotation_radians.is_finite()
        {
            return Err(scene_error(
                "Scene scale or rotation is invalid.",
                "scale-rotate",
            ));
        }
        let (sin, cos) = rotation_radians.sin_cos();
        let m11 = cos * scale_x;
        let m12 = -sin * scale_y;
        let m21 = sin * scale_x;
        let m22 = cos * scale_y;
        Self::new(
            m11,
            m12,
            m21,
            m22,
            origin.x - m11 * origin.x - m12 * origin.y,
            origin.y - m21 * origin.x - m22 * origin.y,
        )
    }

    pub fn transform_point(self, point: ScenePoint) -> ScenePoint {
        ScenePoint {
            x: self.m11 * point.x + self.m12 * point.y + self.tx,
            y: self.m21 * point.x + self.m22 * point.y + self.ty,
        }
    }

    /// Compose parent with child, yielding a transform from child-local space
    /// into the parent's parent space.
    pub fn compose(self, child: Self) -> DockResult<Self> {
        Self::new(
            self.m11 * child.m11 + self.m12 * child.m21,
            self.m11 * child.m12 + self.m12 * child.m22,
            self.m21 * child.m11 + self.m22 * child.m21,
            self.m21 * child.m12 + self.m22 * child.m22,
            self.m11 * child.tx + self.m12 * child.ty + self.tx,
            self.m21 * child.tx + self.m22 * child.ty + self.ty,
        )
    }

    pub fn inverse(self) -> DockResult<Self> {
        self.validate()?;
        let determinant = self.m11 * self.m22 - self.m12 * self.m21;
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
            return Err(scene_error(
                "Scene transform is singular.",
                format!("{self:?}"),
            ));
        }
        let inverse = 1.0 / determinant;
        let m11 = self.m22 * inverse;
        let m12 = -self.m12 * inverse;
        let m21 = -self.m21 * inverse;
        let m22 = self.m11 * inverse;
        Self::new(
            m11,
            m12,
            m21,
            m22,
            -(m11 * self.tx + m12 * self.ty),
            -(m21 * self.tx + m22 * self.ty),
        )
    }

    pub fn transform_bounds(self, rect: SceneRect) -> DockResult<SceneRect> {
        rect.validate()?;
        let points = [
            ScenePoint {
                x: rect.x,
                y: rect.y,
            },
            ScenePoint {
                x: rect.x + rect.width,
                y: rect.y,
            },
            ScenePoint {
                x: rect.x,
                y: rect.y + rect.height,
            },
            ScenePoint {
                x: rect.x + rect.width,
                y: rect.y + rect.height,
            },
        ]
        .map(|point| self.transform_point(point));
        let min_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        let bounds = SceneRect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        };
        bounds.validate()?;
        Ok(bounds)
    }

    fn validate(self) -> DockResult<()> {
        if ![self.m11, self.m12, self.m21, self.m22, self.tx, self.ty]
            .into_iter()
            .all(|value| value.is_finite() && value.abs() <= MAX_SCENE_VALUE)
        {
            return Err(scene_error(
                "Scene transform is invalid or unbounded.",
                format!("{self:?}"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneNode {
    pub panel: Option<PanelId>,
    pub bounds: SceneRect,
    pub transform: Affine2,
    pub clip: Option<SceneRect>,
    pub opacity: f64,
    pub visible: bool,
    pub z_order: i32,
    pub children: Vec<SceneNode>,
}

impl SceneNode {
    pub fn group(bounds: SceneRect) -> Self {
        Self {
            panel: None,
            bounds,
            transform: Affine2::IDENTITY,
            clip: None,
            opacity: 1.0,
            visible: true,
            z_order: 0,
            children: Vec::new(),
        }
    }

    pub fn panel(panel: impl Into<PanelId>, bounds: SceneRect) -> Self {
        Self {
            panel: Some(panel.into()),
            ..Self::group(bounds)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ClipConstraint {
    local_bounds: SceneRect,
    world_transform: Affine2,
    world_bounds: SceneRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneElement {
    pub panel: PanelId,
    pub local_bounds: SceneRect,
    pub world_transform: Affine2,
    pub world_bounds: SceneRect,
    pub opacity: f64,
    clips: Vec<ClipConstraint>,
    /// Parent clips stay fixed when this panel receives a decorative transform.
    panel_clip_start: usize,
}

/// A transformed and clipped vertex for mapping host-provided panel pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneSurfaceVertex {
    pub position: ScenePoint,
    /// Normalized source-surface coordinates.
    pub uv: ScenePoint,
}

/// Visible polygon for one host-provided application surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSurfaceGeometry {
    pub panel: PanelId,
    pub opacity: f64,
    /// Convex polygon in scene coordinates, with UVs mapped from panel-local bounds.
    pub vertices: Vec<SceneSurfaceVertex>,
}

/// A pointer hit resolved into both stable panel identity and panel-local coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneHit {
    pub panel: PanelId,
    pub local_point: ScenePoint,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneFrame {
    revision: u64,
    generation: u64,
    elements: Vec<SceneElement>,
}

impl SceneFrame {
    /// Build a derived transform scene directly from canonical layout.
    /// Rectangular panel geometry remains the source projection; transforms
    /// added by presentation callers remain transient and never write back.
    pub fn from_layout(
        document: &LayoutDocument,
        viewport: Box2,
        generation: u64,
    ) -> DockResult<Self> {
        let snapshot = crate::project(document, viewport).map_err(|error| {
            DockError::new(
                ErrorCode::InvalidLayout,
                "Canonical scene projection failed.",
                error,
            )
        })?;
        Self::from_snapshot(&snapshot, viewport, document.revision, generation)
    }

    /// Adapt an existing canonical projection into the hierarchical scene
    /// representation used for transform composition and inverse hit tests.
    pub fn from_snapshot(
        snapshot: &Snapshot,
        viewport: Box2,
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        validate_box(viewport, "scene-viewport")?;
        let mut root = SceneNode::group(scene_rect(viewport));
        root.children.reserve(snapshot.panels.len());
        for (panel, visual) in &snapshot.panels {
            let bounds = scene_rect(visual.bounds);
            let clip = scene_rect(visual.clip);
            bounds.validate()?;
            clip.validate()?;
            let mut node = SceneNode::panel(
                panel.clone(),
                SceneRect {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height,
                },
            );
            node.transform = Affine2::translate(bounds.x, bounds.y)?;
            node.clip = Some(SceneRect {
                x: clip.x - bounds.x,
                y: clip.y - bounds.y,
                width: clip.width,
                height: clip.height,
            });
            node.opacity = visual.opacity;
            node.visible = visual.opacity > 0.0;
            node.z_order = visual.z;
            root.children.push(node);
        }
        Self::build(&root, revision, generation)
    }

    pub fn build(root: &SceneNode, revision: u64, generation: u64) -> DockResult<Self> {
        let mut builder = SceneBuildState::default();
        collect_scene(root, Affine2::IDENTITY, 1.0, true, &[], 0, &mut builder)?;
        Ok(Self {
            revision,
            generation,
            elements: builder.elements,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn elements(&self) -> &[SceneElement] {
        &self.elements
    }

    /// Project texture geometry for caller-owned panel surfaces. Every inherited
    /// clip is applied in world space, and UVs remain tied to the panel's local
    /// content even when its scene transform rotates or scales it.
    pub fn surface_geometry(
        &self,
        revision: u64,
        generation: u64,
    ) -> DockResult<Vec<SceneSurfaceGeometry>> {
        self.validate_generation(revision, generation)?;
        let mut geometry = Vec::with_capacity(self.elements.len());
        let mut vertex_count = 0usize;
        for element in &self.elements {
            let bounds = element.local_bounds;
            let mut polygon = vec![
                SceneSurfaceVertex {
                    position: element.world_transform.transform_point(ScenePoint {
                        x: bounds.x,
                        y: bounds.y,
                    }),
                    uv: ScenePoint { x: 0.0, y: 0.0 },
                },
                SceneSurfaceVertex {
                    position: element.world_transform.transform_point(ScenePoint {
                        x: bounds.x + bounds.width,
                        y: bounds.y,
                    }),
                    uv: ScenePoint { x: 1.0, y: 0.0 },
                },
                SceneSurfaceVertex {
                    position: element.world_transform.transform_point(ScenePoint {
                        x: bounds.x + bounds.width,
                        y: bounds.y + bounds.height,
                    }),
                    uv: ScenePoint { x: 1.0, y: 1.0 },
                },
                SceneSurfaceVertex {
                    position: element.world_transform.transform_point(ScenePoint {
                        x: bounds.x,
                        y: bounds.y + bounds.height,
                    }),
                    uv: ScenePoint { x: 0.0, y: 1.0 },
                },
            ];
            for clip in &element.clips {
                let clip_polygon = transformed_rect(clip.local_bounds, clip.world_transform);
                polygon = clip_surface_polygon(&polygon, &clip_polygon);
                if polygon.len() < 3 {
                    break;
                }
            }
            if polygon.len() >= 3 {
                vertex_count = vertex_count.checked_add(polygon.len()).ok_or_else(|| {
                    DockError::new(
                        ErrorCode::ResourceLimit,
                        "Scene surface geometry exceeded its bounded vertex limit.",
                        "vertex-count-overflow",
                    )
                })?;
                if vertex_count > MAX_SURFACE_GEOMETRY_VERTICES {
                    return Err(DockError::new(
                        ErrorCode::ResourceLimit,
                        "Scene surface geometry exceeded its bounded vertex limit.",
                        vertex_count.to_string(),
                    ));
                }
                geometry.push(SceneSurfaceGeometry {
                    panel: element.panel.clone(),
                    opacity: element.opacity,
                    vertices: polygon,
                });
            }
        }
        Ok(geometry)
    }

    /// Return a derived frame with panel-local decorative transforms composed
    /// over its existing structural/world transforms. This accepts samples
    /// from an existing transition provider without introducing interpolation
    /// or changing canonical geometry. Parent clipping remains in parent
    /// space; a panel's own clip follows that panel's transform.
    pub fn with_panel_transforms(
        &self,
        transforms: &BTreeMap<PanelId, Affine2>,
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        self.validate_generation(revision, generation)?;
        for panel in transforms.keys() {
            if !self.elements.iter().any(|element| &element.panel == panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Decorative transform targets a panel absent from the scene frame.",
                    &panel.0,
                ));
            }
        }
        let mut composed = self.clone();
        for element in &mut composed.elements {
            let Some(transform) = transforms.get(&element.panel).copied() else {
                continue;
            };
            element.world_transform = transform.compose(element.world_transform)?;
            element.world_bounds = element
                .world_transform
                .transform_bounds(element.local_bounds)?;
            for clip in element.clips.iter_mut().skip(element.panel_clip_start) {
                clip.world_transform = transform.compose(clip.world_transform)?;
                clip.world_bounds = clip.world_transform.transform_bounds(clip.local_bounds)?;
            }
        }
        Ok(composed)
    }

    pub fn hit_test(
        &self,
        point: ScenePoint,
        revision: u64,
        generation: u64,
    ) -> DockResult<Option<PanelId>> {
        Ok(self
            .hit_test_detail(point, revision, generation)?
            .map(|hit| hit.panel))
    }

    /// Resolve a world-space pointer into the frontmost visible panel's local
    /// surface coordinates so an application can route input to transformed content.
    pub fn hit_test_detail(
        &self,
        point: ScenePoint,
        revision: u64,
        generation: u64,
    ) -> DockResult<Option<SceneHit>> {
        self.validate_generation(revision, generation)?;
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x.abs() > MAX_SCENE_VALUE
            || point.y.abs() > MAX_SCENE_VALUE
        {
            return Err(scene_error(
                "Pointer position is invalid or unbounded.",
                format!("{point:?}"),
            ));
        }
        for element in self.elements.iter().rev() {
            if !element.world_bounds.contains(point) {
                continue;
            }
            let local = element.world_transform.inverse()?.transform_point(point);
            if !element.local_bounds.contains(local) {
                continue;
            }
            let visible_in_clips = element.clips.iter().all(|clip| {
                clip.world_transform
                    .inverse()
                    .is_ok_and(|inverse| clip.local_bounds.contains(inverse.transform_point(point)))
            });
            if visible_in_clips {
                return Ok(Some(SceneHit {
                    panel: element.panel.clone(),
                    local_point: local,
                }));
            }
        }
        Ok(None)
    }

    pub fn visible_in(
        &self,
        viewport: SceneRect,
        revision: u64,
        generation: u64,
    ) -> DockResult<Vec<&SceneElement>> {
        self.validate_generation(revision, generation)?;
        viewport.validate()?;
        Ok(self
            .elements
            .iter()
            .filter(|element| {
                intersects(element.world_bounds, viewport)
                    && element
                        .clips
                        .iter()
                        .all(|clip| intersects(clip.world_bounds, viewport))
            })
            .collect())
    }

    fn validate_generation(&self, revision: u64, generation: u64) -> DockResult<()> {
        if self.revision != revision || self.generation != generation {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Scene frame is stale.",
                format!(
                    "frame={}:{} current={revision}:{generation}",
                    self.revision, self.generation
                ),
            ));
        }
        Ok(())
    }
}

fn validate_box(bounds: Box2, context: &str) -> DockResult<()> {
    if !bounds.valid() || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return Err(scene_error(
            "Canonical scene viewport must have positive finite bounds.",
            context,
        ));
    }
    Ok(())
}

fn scene_rect(bounds: Box2) -> SceneRect {
    SceneRect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.w,
        height: bounds.h,
    }
}

#[derive(Default)]
struct SceneBuildState {
    count: usize,
    panel_ids: std::collections::BTreeSet<PanelId>,
    elements: Vec<SceneElement>,
}

fn collect_scene(
    node: &SceneNode,
    parent: Affine2,
    parent_opacity: f64,
    parent_visible: bool,
    inherited_clips: &[ClipConstraint],
    depth: usize,
    builder: &mut SceneBuildState,
) -> DockResult<()> {
    builder.count += 1;
    if builder.count > MAX_SCENE_NODES || depth > MAX_SCENE_DEPTH {
        return Err(scene_error(
            "Scene tree exceeds its node or nesting limit.",
            format!("nodes={}:depth={depth}", builder.count),
        ));
    }
    node.bounds.validate()?;
    if !node.opacity.is_finite() || !(0.0..=1.0).contains(&node.opacity) {
        return Err(scene_error(
            "Scene opacity must be finite and between zero and one.",
            format!("{}", node.opacity),
        ));
    }
    let world = parent.compose(node.transform)?;
    let opacity = parent_opacity * node.opacity;
    let visible = parent_visible && node.visible && opacity > 0.0;
    let mut clips = inherited_clips.to_vec();
    let panel_clip_start = clips.len();
    if let Some(clip) = node.clip {
        clip.validate()?;
        clips.push(ClipConstraint {
            local_bounds: clip,
            world_transform: world,
            world_bounds: world.transform_bounds(clip)?,
        });
    }
    if let Some(panel) = &node.panel {
        if panel.is_empty() || !builder.panel_ids.insert(panel.clone()) {
            return Err(scene_error(
                "Scene panel identity is empty or duplicated.",
                &panel.0,
            ));
        }
        if visible {
            builder.elements.push(SceneElement {
                panel: panel.clone(),
                local_bounds: node.bounds,
                world_transform: world,
                world_bounds: world.transform_bounds(node.bounds)?,
                opacity,
                clips: clips.clone(),
                panel_clip_start,
            });
        }
    }
    let mut child_indices = (0..node.children.len()).collect::<Vec<_>>();
    child_indices.sort_by(|a, b| {
        node.children[*a]
            .z_order
            .cmp(&node.children[*b].z_order)
            .then_with(|| node.children[*a].panel.cmp(&node.children[*b].panel))
    });
    for index in child_indices {
        collect_scene(
            &node.children[index],
            world,
            opacity,
            visible,
            &clips,
            depth + 1,
            builder,
        )?;
    }
    Ok(())
}

fn intersects(first: SceneRect, second: SceneRect) -> bool {
    first.x < second.x + second.width
        && second.x < first.x + first.width
        && first.y < second.y + second.height
        && second.y < first.y + first.height
}

fn transformed_rect(rect: SceneRect, transform: Affine2) -> [ScenePoint; 4] {
    [
        ScenePoint {
            x: rect.x,
            y: rect.y,
        },
        ScenePoint {
            x: rect.x + rect.width,
            y: rect.y,
        },
        ScenePoint {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
        ScenePoint {
            x: rect.x,
            y: rect.y + rect.height,
        },
    ]
    .map(|point| transform.transform_point(point))
}

fn clip_surface_polygon(
    subject: &[SceneSurfaceVertex],
    clip: &[ScenePoint; 4],
) -> Vec<SceneSurfaceVertex> {
    let signed_area = clip
        .iter()
        .zip(clip.iter().cycle().skip(1))
        .take(clip.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f64>();
    if signed_area.abs() <= f64::EPSILON {
        return Vec::new();
    }
    let orientation = signed_area.signum();
    let mut output = subject.to_vec();
    for (edge_start, edge_end) in clip
        .iter()
        .zip(clip.iter().cycle().skip(1))
        .take(clip.len())
    {
        if output.is_empty() {
            break;
        }
        let input = std::mem::take(&mut output);
        let mut previous = *input.last().expect("non-empty polygon");
        let mut previous_distance = orientation * cross(*edge_start, *edge_end, previous.position);
        for current in input {
            let current_distance = orientation * cross(*edge_start, *edge_end, current.position);
            let previous_inside = previous_distance >= -1.0e-9;
            let current_inside = current_distance >= -1.0e-9;
            if previous_inside != current_inside {
                let denominator = previous_distance - current_distance;
                if denominator.abs() > f64::EPSILON {
                    let t = (previous_distance / denominator).clamp(0.0, 1.0);
                    output.push(interpolate_surface_vertex(previous, current, t));
                }
            }
            if current_inside {
                output.push(current);
            }
            previous = current;
            previous_distance = current_distance;
        }
    }
    output
}

fn cross(a: ScenePoint, b: ScenePoint, point: ScenePoint) -> f64 {
    (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x)
}

fn interpolate_surface_vertex(
    start: SceneSurfaceVertex,
    end: SceneSurfaceVertex,
    t: f64,
) -> SceneSurfaceVertex {
    let interpolate = |a: f64, b: f64| a + (b - a) * t;
    SceneSurfaceVertex {
        position: ScenePoint {
            x: interpolate(start.position.x, end.position.x),
            y: interpolate(start.position.y, end.position.y),
        },
        uv: ScenePoint {
            x: interpolate(start.uv.x, end.uv.x),
            y: interpolate(start.uv.y, end.uv.y),
        },
    }
}

fn scene_error(message: &str, context: impl Into<String>) -> DockError {
    DockError::new(ErrorCode::SchemaError, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> SceneRect {
        SceneRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn affine_composition_and_inverse_map_rotated_scaled_points() {
        let parent =
            Affine2::scale_rotate(2.0, 2.0, std::f64::consts::FRAC_PI_2, ScenePoint::default())
                .unwrap();
        let child = Affine2::translate(10.0, 0.0).unwrap();
        let composed = parent.compose(child).unwrap();
        let point = ScenePoint { x: 4.0, y: 3.0 };
        let mapped = composed.transform_point(point);
        assert!((mapped.x + 6.0).abs() < 1e-9);
        assert!((mapped.y - 28.0).abs() < 1e-9);
        let restored = composed.inverse().unwrap().transform_point(mapped);
        assert!((restored.x - point.x).abs() < 1e-9);
        assert!((restored.y - point.y).abs() < 1e-9);
        assert!(
            Affine2::new(1.0, 2.0, 2.0, 4.0, 0.0, 0.0)
                .unwrap()
                .inverse()
                .is_err()
        );
    }

    #[test]
    fn nested_clips_visibility_z_order_and_staleness_are_respected() {
        let mut root = SceneNode::group(rect(0.0, 0.0, 100.0, 100.0));
        root.clip = Some(rect(0.0, 0.0, 20.0, 50.0));
        let mut low = SceneNode::panel("low", rect(0.0, 0.0, 30.0, 30.0));
        low.transform = Affine2::translate(10.0, 10.0).unwrap();
        low.z_order = 1;
        let mut high = SceneNode::panel("high", rect(0.0, 0.0, 30.0, 30.0));
        high.transform = Affine2::translate(10.0, 10.0).unwrap();
        high.z_order = 2;
        root.children = vec![high, low];

        let frame = SceneFrame::build(&root, 7, 11).unwrap();
        assert_eq!(
            frame
                .hit_test(ScenePoint { x: 15.0, y: 15.0 }, 7, 11)
                .unwrap(),
            Some("high".into())
        );
        assert_eq!(
            frame
                .hit_test(ScenePoint { x: 55.0, y: 15.0 }, 7, 11)
                .unwrap(),
            None
        );
        assert!(
            frame
                .hit_test(ScenePoint { x: 15.0, y: 15.0 }, 8, 11)
                .is_err()
        );
        assert!(
            frame
                .visible_in(rect(30.0, 10.0, 10.0, 10.0), 7, 11)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn singular_node_transforms_fail_scene_projection_closed() {
        let mut panel = SceneNode::panel("broken", rect(0.0, 0.0, 10.0, 10.0));
        panel.transform = Affine2::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        assert!(SceneFrame::build(&panel, 1, 1).is_err());
    }

    #[test]
    fn decorative_panel_transforms_compose_over_structural_frames_and_keep_parent_clips_fixed() {
        let mut root = SceneNode::group(rect(0.0, 0.0, 100.0, 100.0));
        root.clip = Some(rect(0.0, 0.0, 20.0, 50.0));
        let mut panel = SceneNode::panel("editor", rect(0.0, 0.0, 20.0, 20.0));
        panel.transform = Affine2::translate(10.0, 10.0).unwrap();
        root.children.push(panel);
        let frame = SceneFrame::build(&root, 7, 11).unwrap();
        let transforms = BTreeMap::from([(
            PanelId::from("editor"),
            Affine2::translate(30.0, 0.0).unwrap(),
        )]);
        let composed = frame.with_panel_transforms(&transforms, 7, 11).unwrap();

        assert_eq!(
            frame
                .hit_test(ScenePoint { x: 15.0, y: 15.0 }, 7, 11)
                .unwrap(),
            Some("editor".into())
        );
        assert_eq!(
            frame
                .hit_test(ScenePoint { x: 45.0, y: 15.0 }, 7, 11)
                .unwrap(),
            None
        );
        assert_eq!(
            composed
                .hit_test(ScenePoint { x: 15.0, y: 15.0 }, 7, 11)
                .unwrap(),
            None
        );
        assert_eq!(
            composed
                .hit_test(ScenePoint { x: 45.0, y: 15.0 }, 7, 11)
                .unwrap(),
            None,
            "parent clipping remains fixed during decorative movement",
        );
        assert!(composed.with_panel_transforms(&transforms, 8, 11).is_err());
        assert!(
            composed
                .with_panel_transforms(
                    &BTreeMap::from([(PanelId::from("missing"), Affine2::IDENTITY)]),
                    7,
                    11,
                )
                .is_err()
        );
    }

    #[test]
    fn panel_local_clip_moves_with_its_decorative_transform() {
        let mut panel = SceneNode::panel("editor", rect(0.0, 0.0, 20.0, 20.0));
        panel.transform = Affine2::translate(10.0, 10.0).unwrap();
        panel.clip = Some(rect(0.0, 0.0, 10.0, 20.0));
        let frame = SceneFrame::build(&panel, 2, 3).unwrap();
        let moved = frame
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("editor"),
                    Affine2::translate(30.0, 0.0).unwrap(),
                )]),
                2,
                3,
            )
            .unwrap();
        assert_eq!(
            moved
                .hit_test(ScenePoint { x: 45.0, y: 15.0 }, 2, 3)
                .unwrap(),
            Some("editor".into())
        );
        assert_eq!(
            moved
                .hit_test(ScenePoint { x: 55.0, y: 15.0 }, 2, 3)
                .unwrap(),
            None
        );
    }

    #[test]
    fn surface_geometry_maps_uvs_and_applies_inherited_clips() {
        let mut root = SceneNode::group(rect(0.0, 0.0, 100.0, 100.0));
        root.clip = Some(rect(0.0, 0.0, 20.0, 50.0));
        let mut panel = SceneNode::panel("editor", rect(0.0, 0.0, 30.0, 20.0));
        panel.transform = Affine2::translate(10.0, 10.0).unwrap();
        root.children.push(panel);
        let frame = SceneFrame::build(&root, 5, 8).unwrap();

        let surfaces = frame.surface_geometry(5, 8).unwrap();
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].panel, PanelId::from("editor"));
        assert!(surfaces[0].vertices.len() >= 4);
        assert!(
            surfaces[0]
                .vertices
                .iter()
                .all(|vertex| vertex.position.x <= 20.0 + 1.0e-8)
        );
        assert!(
            surfaces[0]
                .vertices
                .iter()
                .all(|vertex| vertex.uv.x <= (1.0 / 3.0) + 1.0e-8)
        );
        assert!(frame.surface_geometry(6, 8).is_err());
    }

    #[test]
    fn transformed_surface_geometry_keeps_local_uvs_and_moves_panel_clip() {
        let mut panel = SceneNode::panel("editor", rect(0.0, 0.0, 20.0, 20.0));
        panel.transform = Affine2::translate(10.0, 10.0).unwrap();
        panel.clip = Some(rect(0.0, 0.0, 10.0, 20.0));
        let frame = SceneFrame::build(&panel, 5, 8).unwrap();
        let shifted = frame
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("editor"),
                    Affine2::translate(30.0, 0.0).unwrap(),
                )]),
                5,
                8,
            )
            .unwrap();

        let vertices = &shifted.surface_geometry(5, 8).unwrap()[0].vertices;
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.position.x >= 40.0 - 1.0e-8)
        );
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.position.x <= 50.0 + 1.0e-8)
        );
        assert!(vertices.iter().all(|vertex| vertex.uv.x <= 0.5 + 1.0e-8));
        assert!(vertices.iter().any(|vertex| vertex.uv.y > 0.99));
    }

    #[test]
    fn transformed_pointer_hit_returns_inverse_mapped_panel_local_coordinates() {
        let mut panel = SceneNode::panel("editor", rect(0.0, 0.0, 20.0, 10.0));
        panel.transform = Affine2::new(0.0, -2.0, 2.0, 0.0, 50.0, 10.0).unwrap();
        let frame = SceneFrame::build(&panel, 5, 8).unwrap();
        let expected_local = ScenePoint { x: 6.0, y: 4.0 };
        let world = frame.elements()[0]
            .world_transform
            .transform_point(expected_local);

        let hit = frame
            .hit_test_detail(world, 5, 8)
            .unwrap()
            .expect("transformed panel should receive the pointer");
        assert_eq!(hit.panel, PanelId::from("editor"));
        assert!((hit.local_point.x - expected_local.x).abs() < 1.0e-9);
        assert!((hit.local_point.y - expected_local.y).abs() < 1.0e-9);
        assert_eq!(
            frame.hit_test(world, 5, 8).unwrap(),
            Some(PanelId::from("editor"))
        );
        assert!(frame.hit_test_detail(world, 5, 9).is_err());
    }

    #[test]
    fn canonical_layout_adapter_preserves_panel_identity_and_revision_binding() {
        let mut document = LayoutDocument::new("scene-layout", "workspace");
        document.panels = vec![doofusdock_core::PanelDescriptor::new("editor", "Editor")].into();
        document.root = Some(doofusdock_core::Node::panel("editor-node", "editor"));
        let viewport = Box2 {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        };
        let frame = SceneFrame::from_layout(&document, viewport, 4).unwrap();

        assert_eq!(frame.revision(), document.revision);
        assert_eq!(frame.generation(), 4);
        assert_eq!(
            frame
                .hit_test(ScenePoint { x: 400.0, y: 300.0 }, document.revision, 4)
                .unwrap(),
            Some(PanelId::from("editor"))
        );
        assert!(
            frame
                .hit_test(ScenePoint { x: 400.0, y: 300.0 }, document.revision + 1, 4)
                .is_err()
        );
    }
}
