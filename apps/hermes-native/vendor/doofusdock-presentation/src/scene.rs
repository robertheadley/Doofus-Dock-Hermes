//! Derived two-dimensional scene transforms and inverse hit mapping.
//!
//! This presentation snapshot is never a layout authority. Hosts may use it
//! for virtualized composition and pointer mapping; canonical changes still
//! require revision-bound operations.
use crate::{
    Box2, FlattenedPath2D, Path2D, PathBooleanOp2D, PathCommand2D, Snapshot, ValidatedShape2D,
};
use doofusdock_core::{
    DockError, DockResult, ErrorCode, ExtensionRegistry, LayoutDocument, Node, NodeId, NodeKind,
    PanelId,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_SCENE_NODES: usize = 10_000;
const MAX_SCENE_DEPTH: usize = 128;
const MAX_SCENE_VALUE: f64 = 1.0e9;
const MAX_SURFACE_GEOMETRY_VERTICES: usize = 1_000_000;
const SCENE_SIGNATURE_INITIAL: u128 = 0x6c62272e07bb014262b821756295c58d;
const SCENE_SIGNATURE_PRIME: u128 = 0x0000000001000000000000000000013b;

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

/// Binds stable canonical group identifiers to paths in one derived scene
/// snapshot. Rebuild this map when the projected tree changes; the `NodeId`
/// remains the canonical identity while each path is snapshot-local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneGroupIdentityMap {
    source_tree_signature: u128,
    child_paths: BTreeMap<NodeId, Vec<usize>>,
}

impl SceneGroupIdentityMap {
    /// Bind canonical group IDs to their current scene paths. IDs and paths
    /// must both be unique, and every path must resolve to a scene group.
    pub fn from_scene_paths(
        root: &SceneNode,
        bindings: impl IntoIterator<Item = (NodeId, Vec<usize>)>,
    ) -> DockResult<Self> {
        let source_tree_signature = scene_tree_signature(root)?;
        let mut child_paths = BTreeMap::new();
        let mut unique_paths = BTreeSet::new();
        for (group_id, child_path) in bindings {
            if group_id.is_empty() {
                return Err(scene_error(
                    "Canonical group identity must not be empty.",
                    format!("path={child_path:?}"),
                ));
            }
            if !scene_path_resolves_to_group(root, &child_path) {
                return Err(scene_error(
                    "Canonical group identity path must resolve to a scene group.",
                    format!("group={}:path={child_path:?}", group_id.0),
                ));
            }
            if !unique_paths.insert(child_path.clone()) {
                return Err(scene_error(
                    "A scene group path cannot be bound to multiple canonical identities.",
                    format!("path={child_path:?}"),
                ));
            }
            if child_paths
                .insert(group_id.clone(), child_path.clone())
                .is_some()
            {
                return Err(scene_error(
                    "Canonical group identity is bound more than once.",
                    group_id.0,
                ));
            }
        }
        Ok(Self {
            source_tree_signature,
            child_paths,
        })
    }

    pub fn child_path(&self, group_id: &NodeId) -> Option<&[usize]> {
        self.child_paths.get(group_id).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.child_paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.child_paths.is_empty()
    }
}

/// A scene tree and identity map derived together from one canonical layout projection.
/// The tree keeps canonical group nesting while using the same projected panel geometry
/// as the flat snapshot path.
#[derive(Clone, Debug)]
pub struct LayoutSceneProjection {
    pub scene_root: SceneNode,
    pub frame: SceneFrame,
    pub group_identities: SceneGroupIdentityMap,
}

impl LayoutSceneProjection {
    /// Project the built-in layouts into one revision-bound scene projection.
    pub fn from_layout(
        document: &LayoutDocument,
        viewport: Box2,
        generation: u64,
    ) -> DockResult<Self> {
        Self::from_layout_with_extensions(
            document,
            viewport,
            generation,
            &ExtensionRegistry::default(),
        )
    }

    /// Project the canonical layout through the same registered layout providers
    /// that determine panel geometry, then bind visible groups to those positions.
    pub fn from_layout_with_extensions(
        document: &LayoutDocument,
        viewport: Box2,
        generation: u64,
        extensions: &ExtensionRegistry,
    ) -> DockResult<Self> {
        let snapshot =
            crate::project_with_extensions(document, viewport, extensions).map_err(|error| {
                DockError::new(
                    ErrorCode::InvalidLayout,
                    "Canonical scene projection failed.",
                    error,
                )
            })?;
        Self::from_snapshot(document, &snapshot, viewport, generation)
    }

    /// Preserve canonical group nesting for a caller-provided projection snapshot.
    /// This supports workspace-tag or other host-owned snapshot policies.
    pub fn from_snapshot(
        document: &LayoutDocument,
        snapshot: &Snapshot,
        viewport: Box2,
        generation: u64,
    ) -> DockResult<Self> {
        if !viewport.valid() {
            return Err(scene_error(
                "Invalid canonical scene viewport.",
                format!("{viewport:?}"),
            ));
        }
        let mut scene_root = SceneNode::group(scene_rect(viewport));
        let mut bindings = Vec::new();
        let mut scene_node_count = 1usize;
        let zero = ScenePoint::default();

        if let Some(root) = &document.root {
            let path = vec![scene_root.children.len()];
            if let Some(projected) = append_layout_scene_node(
                root,
                snapshot,
                zero,
                &path,
                &mut bindings,
                1,
                &mut scene_node_count,
            )? {
                scene_root.children.push(projected);
            }
        }
        for window in &document.floating_windows {
            let path = vec![scene_root.children.len()];
            if let Some(projected) = append_layout_scene_node(
                &window.root,
                snapshot,
                zero,
                &path,
                &mut bindings,
                1,
                &mut scene_node_count,
            )? {
                scene_root.children.push(projected);
            }
        }

        let group_identities = SceneGroupIdentityMap::from_scene_paths(&scene_root, bindings)?;
        let frame = SceneFrame::build(&scene_root, document.revision, generation)?;
        Ok(Self {
            scene_root,
            frame,
            group_identities,
        })
    }
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
    path: Option<FlattenedPath2D>,
    /// Arbitrary even-odd clip topology requires triangle-mesh surface output.
    mesh_only: bool,
    /// Shared group clips live in scene/group space and must not follow an
    /// individual panel's later decorative transform.
    fixed_on_panel_transform: bool,
}

/// A transient clip shared by the listed panels in one scene/group coordinate
/// space. The host supplies the group's world transform and current members;
/// membership is never inferred from canonical layout state.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneGroupClip {
    pub panels: BTreeSet<PanelId>,
    pub path: FlattenedPath2D,
    pub world_transform: Affine2,
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

/// Triangulated texture geometry for surfaces constrained by arbitrary
/// validated even-odd scene clips. Unlike `SceneSurfaceGeometry`, this mesh
/// supports concave clips and holes without representing them as a triangle
/// fan. UVs stay attached to the original panel surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSurfaceMeshGeometry {
    pub panel: PanelId,
    pub opacity: f64,
    pub vertices: Vec<SceneSurfaceVertex>,
    pub indices: Vec<u32>,
}

/// A pointer hit resolved into both stable panel identity and panel-local coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneHit {
    pub panel: PanelId,
    pub local_point: ScenePoint,
}

/// Phase of one primary pointer stream routed through a scene frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenePointerPhase {
    Move,
    Press,
    Release,
    Cancel,
}

/// Result of routing a pointer sample to an interactive scene surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScenePointerDispatch {
    /// Surface that receives the current sample, mapped to its local space.
    pub target: Option<SceneHit>,
    /// True when the sample is being delivered to an active press capture.
    pub captured: bool,
    /// Capture canceled because its frame revision or generation changed.
    pub cancelled: Option<PanelId>,
}

/// Host-neutral capture state for one primary pointer stream.
///
/// Presses use the scene's visible hit surface and optional shape/path mask.
/// Once captured, move and release samples continue to the same panel outside
/// its visible geometry. A canonical revision or presentation generation
/// change cancels the old capture so stale widgets cannot keep receiving input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScenePointerRouter {
    capture: Option<ScenePointerCapture>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScenePointerCapture {
    panel: PanelId,
    revision: u64,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneFrame {
    revision: u64,
    generation: u64,
    elements: Vec<SceneElement>,
    source_tree_signature: u128,
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
        let mut builder = SceneBuildState {
            tree_signature: SCENE_SIGNATURE_INITIAL,
            ..SceneBuildState::default()
        };
        collect_scene(root, Affine2::IDENTITY, 1.0, true, &[], 0, &mut builder)?;
        let source_tree_signature = builder.tree_signature;
        Ok(Self {
            revision,
            generation,
            elements: builder.elements,
            source_tree_signature,
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

    /// Report whether any panel uses arbitrary path topology that cannot be
    /// represented by the legacy convex surface polygon API.
    pub fn requires_indexed_surface_mesh(&self) -> bool {
        self.elements
            .iter()
            .flat_map(|element| &element.clips)
            .any(|clip| clip.mesh_only)
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
                if let Some(path) = &clip.path {
                    validate_convex_clip_path(path)?;
                }
                let clip_polygon = match &clip.path {
                    Some(path) => path.contours()[0]
                        .iter()
                        .take(path.contours()[0].len().saturating_sub(usize::from(
                            path.contours()[0].first() == path.contours()[0].last(),
                        )))
                        .map(|[x, y]| {
                            clip.world_transform
                                .transform_point(ScenePoint { x: *x, y: *y })
                        })
                        .collect::<Vec<_>>(),
                    None => transformed_rect(clip.local_bounds, clip.world_transform).to_vec(),
                };
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

    /// Project texture geometry as an indexed triangle mesh. This path uses
    /// bounded planar Boolean intersections, so concave and multi-contour
    /// clips (including holes) remain exact in both rendered geometry and UVs.
    pub fn surface_mesh_geometry(
        &self,
        revision: u64,
        generation: u64,
    ) -> DockResult<Vec<SceneSurfaceMeshGeometry>> {
        self.validate_generation(revision, generation)?;
        let mut geometry = Vec::with_capacity(self.elements.len());
        let mut vertex_count = 0usize;
        for element in &self.elements {
            let bounds = element.local_bounds;
            let local_surface = path_from_rect(bounds)?;
            let mut visible =
                local_surface
                    .transformed(element.world_transform)
                    .map_err(|error| {
                        scene_error("Panel surface transform is invalid.", error.to_string())
                    })?;
            for clip in &element.clips {
                let clip_path = match &clip.path {
                    Some(path) => path.transformed(clip.world_transform).map_err(|error| {
                        scene_error("Scene clip transform is invalid.", error.to_string())
                    })?,
                    None => path_from_rect(clip.local_bounds)?
                        .transformed(clip.world_transform)
                        .map_err(|error| {
                            scene_error(
                                "Scene rectangle clip transform is invalid.",
                                error.to_string(),
                            )
                        })?,
                };
                visible = visible
                    .boolean(&clip_path, PathBooleanOp2D::Intersection)
                    .map_err(|error| {
                        scene_error(
                            "Scene surface clipping failed validation.",
                            error.to_string(),
                        )
                    })?;
                if visible.contours().is_empty() {
                    break;
                }
            }
            let triangles = visible.tessellate().map_err(|error| {
                scene_error(
                    "Scene surface tessellation failed validation.",
                    error.to_string(),
                )
            })?;
            let new_vertices = triangles.len().checked_mul(3).ok_or_else(|| {
                scene_error(
                    "Scene surface mesh exceeded its bounded vertex limit.",
                    "overflow",
                )
            })?;
            vertex_count = vertex_count.checked_add(new_vertices).ok_or_else(|| {
                scene_error(
                    "Scene surface mesh exceeded its bounded vertex limit.",
                    "overflow",
                )
            })?;
            if vertex_count > MAX_SURFACE_GEOMETRY_VERTICES {
                return Err(scene_error(
                    "Scene surface mesh exceeded its bounded vertex limit.",
                    format!("vertices={vertex_count}"),
                ));
            }
            if triangles.is_empty() {
                continue;
            }
            let inverse = element.world_transform.inverse()?;
            let mut vertices = Vec::with_capacity(new_vertices);
            let mut indices = Vec::with_capacity(new_vertices);
            for triangle in triangles {
                for point in triangle {
                    let local = inverse.transform_point(ScenePoint {
                        x: point[0],
                        y: point[1],
                    });
                    vertices.push(SceneSurfaceVertex {
                        position: ScenePoint {
                            x: point[0],
                            y: point[1],
                        },
                        uv: ScenePoint {
                            x: ((local.x - bounds.x) / bounds.width).clamp(0.0, 1.0),
                            y: ((local.y - bounds.y) / bounds.height).clamp(0.0, 1.0),
                        },
                    });
                    indices.push((indices.len()) as u32);
                }
            }
            geometry.push(SceneSurfaceMeshGeometry {
                panel: element.panel.clone(),
                opacity: element.opacity,
                vertices,
                indices,
            });
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
            for clip in element
                .clips
                .iter_mut()
                .skip(element.panel_clip_start)
                .filter(|clip| !clip.fixed_on_panel_transform)
            {
                clip.world_transform = transform.compose(clip.world_transform)?;
                clip.world_bounds = clip.world_transform.transform_bounds(clip.local_bounds)?;
            }
        }
        Ok(composed)
    }

    /// Add panel-local convex path clips to a revision-bound scene frame.
    /// Curved boundaries should be flattened by the caller with a bounded
    /// tolerance/segment budget. The clips follow the panel's composed
    /// transform and apply to both surface geometry and hit testing.
    pub fn with_panel_clip_paths(
        &self,
        clips: &BTreeMap<PanelId, FlattenedPath2D>,
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        self.validate_generation(revision, generation)?;
        for (panel, path) in clips {
            if !self.elements.iter().any(|element| &element.panel == panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Scene clip targets a panel absent from the frame.",
                    &panel.0,
                ));
            }
            validate_convex_clip_path(path)?;
        }
        let mut clipped = self.clone();
        for element in &mut clipped.elements {
            let Some(path) = clips.get(&element.panel) else {
                continue;
            };
            let local_bounds = scene_rect(path.bounds());
            local_bounds.validate()?;
            element.clips.push(ClipConstraint {
                local_bounds,
                world_bounds: element.world_transform.transform_bounds(local_bounds)?,
                world_transform: element.world_transform,
                path: Some(path.clone()),
                mesh_only: false,
                fixed_on_panel_transform: false,
            });
        }
        Ok(clipped)
    }

    /// Add revision-bound panel-local even-odd clips with bounded arbitrary
    /// contour topology. Use `surface_mesh_geometry` to render these clips;
    /// hit testing uses the same filled geometry.
    pub fn with_panel_mesh_clip_paths(
        &self,
        clips: &BTreeMap<PanelId, FlattenedPath2D>,
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        self.validate_generation(revision, generation)?;
        for (panel, path) in clips {
            if !self.elements.iter().any(|element| &element.panel == panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Scene clip targets a panel absent from the frame.",
                    &panel.0,
                ));
            }
            path.tessellate().map_err(|error| {
                scene_error(
                    "Scene mesh clip path is invalid or unbounded.",
                    error.to_string(),
                )
            })?;
        }
        let mut clipped = self.clone();
        for element in &mut clipped.elements {
            let Some(path) = clips.get(&element.panel) else {
                continue;
            };
            let local_bounds = scene_rect(path.bounds());
            local_bounds.validate()?;
            element.clips.push(ClipConstraint {
                local_bounds,
                world_bounds: element.world_transform.transform_bounds(local_bounds)?,
                world_transform: element.world_transform,
                path: Some(path.clone()),
                mesh_only: true,
                fixed_on_panel_transform: false,
            });
        }
        Ok(clipped)
    }

    /// Add bounded arbitrary-topology clips shared by caller-declared scene
    /// groups. The same group-space path and transform is applied to every
    /// listed panel, constraining rendered surface meshes and hit testing.
    /// Callers should rebuild the declaration when group membership changes.
    pub fn with_group_mesh_clip_paths(
        &self,
        groups: &[SceneGroupClip],
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        self.validate_generation(revision, generation)?;
        if groups.len() > MAX_SCENE_NODES {
            return Err(scene_error(
                "Group clip count exceeds the bounded scene limit.",
                format!("groups={}", groups.len()),
            ));
        }
        let panel_indexes = self
            .elements
            .iter()
            .enumerate()
            .map(|(index, element)| (element.panel.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut membership_count = 0usize;
        for group in groups {
            if group.panels.is_empty() {
                return Err(scene_error(
                    "Group clip must target at least one panel.",
                    "empty-panel-set",
                ));
            }
            membership_count = membership_count
                .checked_add(group.panels.len())
                .ok_or_else(|| {
                    scene_error("Group clip membership count overflowed.", "overflow")
                })?;
            if membership_count > MAX_SCENE_NODES {
                return Err(scene_error(
                    "Group clip membership exceeds the bounded scene limit.",
                    format!("memberships={membership_count}"),
                ));
            }
            if let Some(panel) = group
                .panels
                .iter()
                .find(|panel| !panel_indexes.contains_key(*panel))
            {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Group clip targets a panel absent from the scene frame.",
                    &panel.0,
                ));
            }
            group.path.tessellate().map_err(|error| {
                scene_error(
                    "Group mesh clip path is invalid or unbounded.",
                    error.to_string(),
                )
            })?;
            let bounds = scene_rect(group.path.bounds());
            bounds.validate()?;
            group.world_transform.transform_bounds(bounds)?;
        }

        let mut clipped = self.clone();
        for group in groups {
            let local_bounds = scene_rect(group.path.bounds());
            let world_bounds = group.world_transform.transform_bounds(local_bounds)?;
            for panel in &group.panels {
                let Some(index) = panel_indexes.get(panel).copied() else {
                    return Err(DockError::new(
                        ErrorCode::NotFound,
                        "Group clip panel disappeared during validation.",
                        &panel.0,
                    ));
                };
                clipped.elements[index].clips.push(ClipConstraint {
                    local_bounds,
                    world_transform: group.world_transform,
                    world_bounds,
                    path: Some(group.path.clone()),
                    mesh_only: true,
                    fixed_on_panel_transform: true,
                });
            }
        }
        Ok(clipped)
    }

    /// Derive a group's visible descendant panel membership and world
    /// transform from the scene tree, then apply its arbitrary-topology mask.
    /// `root` must be the same scene snapshot used to build this frame.
    /// `group_child_path` is a snapshot-local sequence of child indexes from
    /// `root`; rebuild it when the scene tree changes. This removes duplicated
    /// member lists but does not infer a persistent group identity.
    pub fn with_group_mesh_clip_for_subtree(
        &self,
        root: &SceneNode,
        group_child_path: &[usize],
        path: FlattenedPath2D,
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        self.validate_generation(revision, generation)?;
        if scene_tree_signature(root)? != self.source_tree_signature {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Group clip tree does not match the scene frame source snapshot.",
                format!("revision={revision}:generation={generation}"),
            ));
        }
        let group = scene_group_clip_from_subtree(root, group_child_path, path)?;
        self.with_group_mesh_clip_paths(&[group], revision, generation)
    }

    /// Apply a group mask by stable canonical `NodeId`, using the snapshot
    /// binding produced for this exact scene tree. Hosts rebuild the binding
    /// map after projection changes, but keep the ID from the canonical tree.
    pub fn with_group_mesh_clip_for_identity(
        &self,
        root: &SceneNode,
        identities: &SceneGroupIdentityMap,
        group_id: &NodeId,
        path: FlattenedPath2D,
        revision: u64,
        generation: u64,
    ) -> DockResult<Self> {
        self.validate_generation(revision, generation)?;
        if identities.source_tree_signature != self.source_tree_signature {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Canonical group identity map does not match the scene frame source snapshot.",
                format!("revision={revision}:generation={generation}"),
            ));
        }
        let child_path = identities.child_path(group_id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Canonical group identity is not bound in this scene snapshot.",
                &group_id.0,
            )
        })?;
        self.with_group_mesh_clip_for_subtree(root, child_path, path, revision, generation)
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
        self.hit_test_detail_matching(point, revision, generation, |_, _| Ok(true))
    }

    /// Resolve a transformed/clipped pointer hit while optionally applying a
    /// validated local hit shape per panel. Panels without an entry retain the
    /// existing rectangular hit surface; rejecting a front surface continues
    /// the search for an eligible surface behind it.
    pub fn hit_test_detail_with_shapes(
        &self,
        point: ScenePoint,
        revision: u64,
        generation: u64,
        shapes: &BTreeMap<PanelId, ValidatedShape2D>,
    ) -> DockResult<Option<SceneHit>> {
        self.validate_generation(revision, generation)?;
        for panel in shapes.keys() {
            if !self.elements.iter().any(|element| &element.panel == panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Hit shape targets a panel absent from the scene frame.",
                    &panel.0,
                ));
            }
        }
        self.hit_test_detail_matching(point, revision, generation, |panel, local| {
            match shapes.get(panel) {
                Some(shape) => shape.contains([local.x, local.y]).map_err(|error| {
                    scene_error(
                        "Panel hit shape rejected its local point.",
                        error.to_string(),
                    )
                }),
                None => Ok(true),
            }
        })
    }

    /// Resolve a transformed/clipped pointer hit while applying optional
    /// validated flattened panel-local paths. Panels without an entry retain
    /// rectangular hit behavior; a path miss falls through to visible panels
    /// behind the rejected surface.
    pub fn hit_test_detail_with_paths(
        &self,
        point: ScenePoint,
        revision: u64,
        generation: u64,
        paths: &BTreeMap<PanelId, FlattenedPath2D>,
    ) -> DockResult<Option<SceneHit>> {
        self.validate_generation(revision, generation)?;
        for panel in paths.keys() {
            if !self.elements.iter().any(|element| &element.panel == panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Hit path targets a panel absent from the scene frame.",
                    &panel.0,
                ));
            }
        }
        self.hit_test_detail_matching(point, revision, generation, |panel, local| {
            match paths.get(panel) {
                Some(path) => path.contains([local.x, local.y]).map_err(|error| {
                    scene_error(
                        "Panel hit path rejected its local point.",
                        error.to_string(),
                    )
                }),
                None => Ok(true),
            }
        })
    }

    fn hit_test_detail_matching(
        &self,
        point: ScenePoint,
        revision: u64,
        generation: u64,
        mut matches: impl FnMut(&PanelId, ScenePoint) -> DockResult<bool>,
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
            let mut visible_in_clips = true;
            for clip in &element.clips {
                let inverse = clip.world_transform.inverse()?;
                let local_point = inverse.transform_point(point);
                if !clip.local_bounds.contains(local_point) {
                    visible_in_clips = false;
                    break;
                }
                let inside_path = match &clip.path {
                    Some(path) => {
                        path.contains([local_point.x, local_point.y])
                            .map_err(|error| {
                                scene_error(
                                    "Scene clip path rejected its local point.",
                                    error.to_string(),
                                )
                            })?
                    }
                    None => true,
                };
                if !inside_path {
                    visible_in_clips = false;
                    break;
                }
            }
            if visible_in_clips && matches(&element.panel, local)? {
                return Ok(Some(SceneHit {
                    panel: element.panel.clone(),
                    local_point: local,
                }));
            }
        }
        Ok(None)
    }

    fn panel_local_point(
        &self,
        panel: &PanelId,
        point: ScenePoint,
        revision: u64,
        generation: u64,
    ) -> DockResult<ScenePoint> {
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
        let mut elements = self
            .elements
            .iter()
            .filter(|element| &element.panel == panel);
        let element = elements.next().ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Pointer capture targets a panel absent from the scene frame.",
                &panel.0,
            )
        })?;
        if elements.next().is_some() {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Pointer capture panel is ambiguous in the scene frame.",
                &panel.0,
            ));
        }
        let local = element.world_transform.inverse()?.transform_point(point);
        if !local.x.is_finite()
            || !local.y.is_finite()
            || local.x.abs() > MAX_SCENE_VALUE
            || local.y.abs() > MAX_SCENE_VALUE
        {
            return Err(scene_error(
                "Mapped panel-local pointer position is invalid or unbounded.",
                format!("panel={panel:?} local={local:?}"),
            ));
        }
        Ok(local)
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

impl ScenePointerRouter {
    /// Route an unmasked pointer stream. Press selects the frontmost visible
    /// panel; move and release remain captured until release or cancellation.
    pub fn route(
        &mut self,
        frame: &SceneFrame,
        phase: ScenePointerPhase,
        point: ScenePoint,
        revision: u64,
        generation: u64,
    ) -> DockResult<ScenePointerDispatch> {
        self.route_with(frame, phase, point, revision, generation, |point| {
            frame.hit_test_detail(point, revision, generation)
        })
    }

    /// Route through optional validated panel-local hit shapes. A masked front
    /// surface miss may select a visible panel behind it on press.
    pub fn route_with_shapes(
        &mut self,
        frame: &SceneFrame,
        phase: ScenePointerPhase,
        point: ScenePoint,
        revision: u64,
        generation: u64,
        shapes: &BTreeMap<PanelId, ValidatedShape2D>,
    ) -> DockResult<ScenePointerDispatch> {
        self.route_with(frame, phase, point, revision, generation, |point| {
            frame.hit_test_detail_with_shapes(point, revision, generation, shapes)
        })
    }

    /// Route through optional validated panel-local compound-path masks.
    pub fn route_with_paths(
        &mut self,
        frame: &SceneFrame,
        phase: ScenePointerPhase,
        point: ScenePoint,
        revision: u64,
        generation: u64,
        paths: &BTreeMap<PanelId, FlattenedPath2D>,
    ) -> DockResult<ScenePointerDispatch> {
        self.route_with(frame, phase, point, revision, generation, |point| {
            frame.hit_test_detail_with_paths(point, revision, generation, paths)
        })
    }

    /// Forget any active capture. Hosts call this on pointer loss, viewport
    /// deactivation, or other platform cancellation without a release sample.
    pub fn cancel(&mut self) -> Option<PanelId> {
        self.capture.take().map(|capture| capture.panel)
    }

    fn route_with(
        &mut self,
        frame: &SceneFrame,
        phase: ScenePointerPhase,
        point: ScenePoint,
        revision: u64,
        generation: u64,
        mut hit_test: impl FnMut(ScenePoint) -> DockResult<Option<SceneHit>>,
    ) -> DockResult<ScenePointerDispatch> {
        frame.validate_generation(revision, generation)?;
        let mut dispatch = ScenePointerDispatch::default();
        if self
            .capture
            .as_ref()
            .is_some_and(|capture| capture.revision != revision || capture.generation != generation)
        {
            dispatch.cancelled = self.cancel();
        }

        match phase {
            ScenePointerPhase::Move => {
                if let Some(capture) = &self.capture {
                    dispatch.target = Some(SceneHit {
                        panel: capture.panel.clone(),
                        local_point: frame.panel_local_point(
                            &capture.panel,
                            point,
                            revision,
                            generation,
                        )?,
                    });
                    dispatch.captured = true;
                } else {
                    dispatch.target = hit_test(point)?;
                }
            }
            ScenePointerPhase::Press => {
                if let Some(capture) = &self.capture {
                    dispatch.target = Some(SceneHit {
                        panel: capture.panel.clone(),
                        local_point: frame.panel_local_point(
                            &capture.panel,
                            point,
                            revision,
                            generation,
                        )?,
                    });
                    dispatch.captured = true;
                } else if let Some(hit) = hit_test(point)? {
                    self.capture = Some(ScenePointerCapture {
                        panel: hit.panel.clone(),
                        revision,
                        generation,
                    });
                    dispatch.target = Some(hit);
                    dispatch.captured = true;
                }
            }
            ScenePointerPhase::Release => {
                if let Some(capture) = self.capture.take() {
                    let local_point =
                        frame.panel_local_point(&capture.panel, point, revision, generation)?;
                    dispatch.target = Some(SceneHit {
                        panel: capture.panel,
                        local_point,
                    });
                    dispatch.captured = true;
                }
            }
            ScenePointerPhase::Cancel => {
                dispatch.cancelled = self.cancel().or(dispatch.cancelled);
            }
        }
        Ok(dispatch)
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

fn append_layout_scene_node(
    node: &Node,
    snapshot: &Snapshot,
    parent_origin: ScenePoint,
    path: &[usize],
    bindings: &mut Vec<(NodeId, Vec<usize>)>,
    depth: usize,
    scene_node_count: &mut usize,
) -> DockResult<Option<SceneNode>> {
    *scene_node_count = (*scene_node_count).saturating_add(1);
    if *scene_node_count > MAX_SCENE_NODES || depth > MAX_SCENE_DEPTH {
        return Err(scene_error(
            "Canonical layout scene exceeds its node or nesting limit.",
            format!("nodes={scene_node_count}:depth={depth}"),
        ));
    }
    let node_bounds = snapshot.nodes.get(&node.id).copied().ok_or_else(|| {
        scene_error(
            "Canonical node has no geometry in the projected snapshot.",
            &node.id.0,
        )
    })?;

    match &node.kind {
        NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. } => {
            let Some(visual) = snapshot.panels.get(panel) else {
                return Ok(None);
            };
            if visual.opacity <= 0.0 {
                return Ok(None);
            }
            let local_bounds = SceneRect {
                x: 0.0,
                y: 0.0,
                width: visual.bounds.w,
                height: visual.bounds.h,
            };
            let mut projected = SceneNode::panel(panel.clone(), local_bounds);
            projected.transform = Affine2::translate(
                visual.bounds.x - parent_origin.x,
                visual.bounds.y - parent_origin.y,
            )?;
            projected.clip = Some(SceneRect {
                x: visual.clip.x - visual.bounds.x,
                y: visual.clip.y - visual.bounds.y,
                width: visual.clip.w,
                height: visual.clip.h,
            });
            projected.opacity = visual.opacity;
            projected.z_order = visual.z;
            Ok(Some(projected))
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => {
            let mut projected = SceneNode::group(SceneRect {
                x: 0.0,
                y: 0.0,
                width: node_bounds.w,
                height: node_bounds.h,
            });
            projected.transform = Affine2::translate(
                node_bounds.x - parent_origin.x,
                node_bounds.y - parent_origin.y,
            )?;
            for child in items {
                let mut child_path = path.to_vec();
                child_path.push(projected.children.len());
                if let Some(child_node) = append_layout_scene_node(
                    child,
                    snapshot,
                    ScenePoint {
                        x: node_bounds.x,
                        y: node_bounds.y,
                    },
                    &child_path,
                    bindings,
                    depth + 1,
                    scene_node_count,
                )? {
                    projected.children.push(child_node);
                }
            }
            if projected.children.is_empty() {
                return Ok(None);
            }
            bindings.push((node.id.clone(), path.to_vec()));
            Ok(Some(projected))
        }
    }
}
fn scene_rect(bounds: Box2) -> SceneRect {
    SceneRect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.w,
        height: bounds.h,
    }
}

fn path_from_rect(rect: SceneRect) -> DockResult<FlattenedPath2D> {
    rect.validate()?;
    Path2D {
        commands: vec![
            PathCommand2D::MoveTo {
                point: [rect.x, rect.y],
            },
            PathCommand2D::LineTo {
                point: [rect.x + rect.width, rect.y],
            },
            PathCommand2D::LineTo {
                point: [rect.x + rect.width, rect.y + rect.height],
            },
            PathCommand2D::LineTo {
                point: [rect.x, rect.y + rect.height],
            },
            PathCommand2D::Close,
        ],
    }
    .flatten(0.01, 8)
    .map_err(|error| {
        scene_error(
            "Scene rectangle could not be converted to a path.",
            error.to_string(),
        )
    })
}

fn validate_convex_clip_path(path: &FlattenedPath2D) -> DockResult<()> {
    let contours = path.contours();
    if contours.len() != 1 {
        return Err(scene_error(
            "Scene path clips must contain one convex contour.",
            format!("contours={}", contours.len()),
        ));
    }
    let contour = &contours[0];
    let vertex_count = contour
        .len()
        .saturating_sub(usize::from(contour.first() == contour.last()));
    if !(3..=2048).contains(&vertex_count) {
        return Err(scene_error(
            "Scene clip path vertex count is outside the supported range.",
            format!("vertices={vertex_count}"),
        ));
    }
    let points = &contour[..vertex_count];
    let mut orientation = 0.0_f64;
    for index in 0..points.len() {
        let [ax, ay] = points[index];
        let [bx, by] = points[(index + 1) % points.len()];
        let [cx, cy] = points[(index + 2) % points.len()];
        let cross = (bx - ax) * (cy - by) - (by - ay) * (cx - bx);
        if cross.abs() <= 1.0e-9 {
            continue;
        }
        if orientation == 0.0 {
            orientation = cross.signum();
        } else if orientation != cross.signum() {
            return Err(scene_error(
                "Scene path clips must be convex.",
                "mixed-turn-orientation",
            ));
        }
    }
    if orientation == 0.0 {
        return Err(scene_error(
            "Scene clip path has no non-collinear boundary.",
            "zero-area-path",
        ));
    }
    Ok(())
}

#[derive(Default)]
struct SceneBuildState {
    count: usize,
    panel_ids: std::collections::BTreeSet<PanelId>,
    elements: Vec<SceneElement>,
    tree_signature: u128,
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
    update_scene_signature_node(node, &mut builder.tree_signature);
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
            path: None,
            mesh_only: false,
            fixed_on_panel_transform: false,
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
        scene_signature_bytes(&mut builder.tree_signature, &(index as u64).to_le_bytes());
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

fn scene_group_clip_from_subtree(
    root: &SceneNode,
    child_path: &[usize],
    path: FlattenedPath2D,
) -> DockResult<SceneGroupClip> {
    if child_path.len() > MAX_SCENE_DEPTH {
        return Err(scene_error(
            "Group clip child path exceeds the scene depth limit.",
            format!("depth={}", child_path.len()),
        ));
    }

    let mut node = root;
    let mut parent_world = Affine2::IDENTITY;
    let mut parent_opacity = 1.0;
    let mut parent_visible = true;
    let mut depth = 0usize;
    let (group, world_transform) = loop {
        node.bounds.validate()?;
        if !node.opacity.is_finite() || !(0.0..=1.0).contains(&node.opacity) {
            return Err(scene_error(
                "Scene opacity must be finite and between zero and one.",
                format!("{}", node.opacity),
            ));
        }
        let world = parent_world.compose(node.transform)?;
        let opacity = parent_opacity * node.opacity;
        let visible = parent_visible && node.visible && opacity > 0.0;
        if depth == child_path.len() {
            if node.panel.is_some() {
                return Err(scene_error(
                    "Group clip path must identify a scene group node.",
                    format!("depth={depth}"),
                ));
            }
            break (node, world);
        }
        let child_index = child_path[depth];
        node = node.children.get(child_index).ok_or_else(|| {
            scene_error(
                "Group clip child path does not resolve in the scene tree.",
                format!("depth={depth}:child={child_index}"),
            )
        })?;
        parent_world = world;
        parent_opacity = opacity;
        parent_visible = visible;
        depth += 1;
    };

    let mut builder = SceneBuildState::default();
    collect_scene(
        group,
        parent_world,
        parent_opacity,
        parent_visible,
        &[],
        depth,
        &mut builder,
    )?;
    let panels = builder
        .elements
        .into_iter()
        .map(|element| element.panel)
        .collect::<BTreeSet<_>>();
    if panels.is_empty() {
        return Err(scene_error(
            "Group clip scene node has no visible descendant panels.",
            format!("depth={depth}"),
        ));
    }
    Ok(SceneGroupClip {
        panels,
        path,
        world_transform,
    })
}

fn scene_path_resolves_to_group(root: &SceneNode, child_path: &[usize]) -> bool {
    if child_path.len() > MAX_SCENE_DEPTH {
        return false;
    }
    let mut node = root;
    for child_index in child_path {
        let Some(child) = node.children.get(*child_index) else {
            return false;
        };
        node = child;
    }
    node.panel.is_none()
}

fn scene_tree_signature(root: &SceneNode) -> DockResult<u128> {
    fn visit(node: &SceneNode, hash: &mut u128, depth: usize, count: &mut usize) -> DockResult<()> {
        *count += 1;
        if depth > MAX_SCENE_DEPTH || *count > MAX_SCENE_NODES {
            return Err(scene_error(
                "Scene tree signature exceeds the node or nesting limit.",
                format!("nodes={count}:depth={depth}"),
            ));
        }
        update_scene_signature_node(node, hash);
        let mut child_indexes = (0..node.children.len()).collect::<Vec<_>>();
        child_indexes.sort_by(|a, b| {
            node.children[*a]
                .z_order
                .cmp(&node.children[*b].z_order)
                .then_with(|| node.children[*a].panel.cmp(&node.children[*b].panel))
        });
        for index in child_indexes {
            scene_signature_bytes(hash, &(index as u64).to_le_bytes());
            visit(&node.children[index], hash, depth + 1, count)?;
        }
        Ok(())
    }

    let mut hash = SCENE_SIGNATURE_INITIAL;
    let mut count = 0;
    visit(root, &mut hash, 0, &mut count)?;
    Ok(hash)
}

fn update_scene_signature_node(node: &SceneNode, hash: &mut u128) {
    for value in [
        node.bounds.x,
        node.bounds.y,
        node.bounds.width,
        node.bounds.height,
        node.transform.m11,
        node.transform.m12,
        node.transform.m21,
        node.transform.m22,
        node.transform.tx,
        node.transform.ty,
        node.opacity,
    ] {
        scene_signature_bytes(hash, &value.to_bits().to_le_bytes());
    }
    match node.panel.as_ref() {
        Some(panel) => {
            scene_signature_bytes(hash, &[1]);
            scene_signature_bytes(hash, &(panel.0.len() as u64).to_le_bytes());
            scene_signature_bytes(hash, panel.0.as_bytes());
        }
        None => scene_signature_bytes(hash, &[0]),
    }
    match node.clip {
        Some(clip) => {
            scene_signature_bytes(hash, &[1]);
            for value in [clip.x, clip.y, clip.width, clip.height] {
                scene_signature_bytes(hash, &value.to_bits().to_le_bytes());
            }
        }
        None => scene_signature_bytes(hash, &[0]),
    }
    scene_signature_bytes(hash, &[u8::from(node.visible)]);
    scene_signature_bytes(hash, &node.z_order.to_le_bytes());
    scene_signature_bytes(hash, &(node.children.len() as u64).to_le_bytes());
}

fn scene_signature_bytes(hash: &mut u128, value: &[u8]) {
    for byte in value {
        *hash ^= u128::from(*byte);
        *hash = hash.wrapping_mul(SCENE_SIGNATURE_PRIME);
    }
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
    clip: &[ScenePoint],
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
    use crate::Shape2D;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> SceneRect {
        SceneRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn pointer_capture_routes_outside_surface_and_cancels_after_frame_change() {
        let mut panel = SceneNode::panel("interactive", rect(0.0, 0.0, 20.0, 20.0));
        panel.transform = Affine2::translate(10.0, 5.0).unwrap();
        let frame = SceneFrame::build(&panel, 2, 3).unwrap();
        let mut router = ScenePointerRouter::default();

        let pressed = router
            .route(
                &frame,
                ScenePointerPhase::Press,
                ScenePoint { x: 15.0, y: 10.0 },
                2,
                3,
            )
            .unwrap();
        assert_eq!(
            pressed.target.as_ref().unwrap().panel,
            PanelId::from("interactive")
        );
        assert_eq!(
            pressed.target.as_ref().unwrap().local_point,
            ScenePoint { x: 5.0, y: 5.0 }
        );
        assert!(pressed.captured);

        let moved = router
            .route(
                &frame,
                ScenePointerPhase::Move,
                ScenePoint { x: 50.0, y: 50.0 },
                2,
                3,
            )
            .unwrap();
        assert_eq!(
            moved.target.as_ref().unwrap().panel,
            PanelId::from("interactive")
        );
        assert_eq!(
            moved.target.as_ref().unwrap().local_point,
            ScenePoint { x: 40.0, y: 45.0 }
        );
        assert!(moved.captured);

        let changed_frame = SceneFrame::build(&panel, 2, 4).unwrap();
        let after_change = router
            .route(
                &changed_frame,
                ScenePointerPhase::Move,
                ScenePoint { x: 50.0, y: 50.0 },
                2,
                4,
            )
            .unwrap();
        assert_eq!(after_change.cancelled, Some(PanelId::from("interactive")));
        assert!(!after_change.captured);
        assert!(after_change.target.is_none());
        assert!(router.capture.is_none());

        router
            .route(
                &changed_frame,
                ScenePointerPhase::Press,
                ScenePoint { x: 15.0, y: 10.0 },
                2,
                4,
            )
            .unwrap();
        let new_revision = SceneFrame::build(&panel, 3, 4).unwrap();
        let after_revision = router
            .route(
                &new_revision,
                ScenePointerPhase::Move,
                ScenePoint { x: 50.0, y: 50.0 },
                3,
                4,
            )
            .unwrap();
        assert_eq!(after_revision.cancelled, Some(PanelId::from("interactive")));
        assert!(!after_revision.captured);
    }

    #[test]
    fn masked_pointer_press_falls_through_but_release_stays_captured() {
        let mut back = SceneNode::panel("back", rect(0.0, 0.0, 20.0, 20.0));
        back.z_order = 1;
        let mut front = SceneNode::panel("front", rect(0.0, 0.0, 20.0, 20.0));
        front.z_order = 2;
        let mut root = SceneNode::group(rect(0.0, 0.0, 20.0, 20.0));
        root.children = vec![back, front];
        let frame = SceneFrame::build(&root, 5, 8).unwrap();
        let shape = ValidatedShape2D::new(Shape2D::Circle {
            center: [10.0, 10.0],
            radius: 4.0,
        })
        .unwrap();
        let shapes = BTreeMap::from([(PanelId::from("front"), shape)]);
        let mut router = ScenePointerRouter::default();

        let pressed = router
            .route_with_shapes(
                &frame,
                ScenePointerPhase::Press,
                ScenePoint { x: 2.0, y: 2.0 },
                5,
                8,
                &shapes,
            )
            .unwrap();
        assert_eq!(
            pressed.target.as_ref().unwrap().panel,
            PanelId::from("back")
        );

        let released = router
            .route_with_shapes(
                &frame,
                ScenePointerPhase::Release,
                ScenePoint { x: 30.0, y: 30.0 },
                5,
                8,
                &shapes,
            )
            .unwrap();
        assert_eq!(
            released.target.as_ref().unwrap().panel,
            PanelId::from("back")
        );
        assert_eq!(
            released.target.as_ref().unwrap().local_point,
            ScenePoint { x: 30.0, y: 30.0 }
        );
        assert!(released.captured);
        assert!(router.capture.is_none());
    }

    #[test]
    fn masked_path_pointer_press_falls_through_to_visible_surface() {
        use crate::{Path2D, PathCommand2D};

        let mut back = SceneNode::panel("path-back", rect(0.0, 0.0, 20.0, 20.0));
        back.z_order = 1;
        let mut front = SceneNode::panel("path-front", rect(0.0, 0.0, 20.0, 20.0));
        front.z_order = 2;
        let mut root = SceneNode::group(rect(0.0, 0.0, 20.0, 20.0));
        root.children = vec![back, front];
        let frame = SceneFrame::build(&root, 5, 8).unwrap();
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [8.0, 8.0] },
                PathCommand2D::LineTo { point: [12.0, 8.0] },
                PathCommand2D::LineTo {
                    point: [12.0, 12.0],
                },
                PathCommand2D::LineTo { point: [8.0, 12.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        let paths = BTreeMap::from([(PanelId::from("path-front"), path)]);
        let mut router = ScenePointerRouter::default();

        let pressed = router
            .route_with_paths(
                &frame,
                ScenePointerPhase::Press,
                ScenePoint { x: 2.0, y: 2.0 },
                5,
                8,
                &paths,
            )
            .unwrap();
        assert_eq!(pressed.target.unwrap().panel, PanelId::from("path-back"));
    }

    #[test]
    fn pointer_cancel_notifies_the_captured_surface_once() {
        let panel = SceneNode::panel("capture", rect(0.0, 0.0, 10.0, 10.0));
        let frame = SceneFrame::build(&panel, 1, 1).unwrap();
        let mut router = ScenePointerRouter::default();
        router
            .route(
                &frame,
                ScenePointerPhase::Press,
                ScenePoint { x: 5.0, y: 5.0 },
                1,
                1,
            )
            .unwrap();
        let canceled = router
            .route(
                &frame,
                ScenePointerPhase::Cancel,
                ScenePoint::default(),
                1,
                1,
            )
            .unwrap();
        assert_eq!(canceled.cancelled, Some(PanelId::from("capture")));
        assert_eq!(router.cancel(), None);
    }

    #[test]
    fn captured_pointer_rejects_unbounded_inverse_mapped_coordinates() {
        let mut panel = SceneNode::panel("bounded-capture", rect(0.0, 0.0, 10.0, 10.0));
        panel.transform = Affine2::new(1.0e-7, 0.0, 0.0, 1.0e-7, 0.0, 0.0).unwrap();
        let frame = SceneFrame::build(&panel, 1, 1).unwrap();
        let mut router = ScenePointerRouter::default();
        router
            .route(
                &frame,
                ScenePointerPhase::Press,
                ScenePoint {
                    x: 5.0e-7,
                    y: 5.0e-7,
                },
                1,
                1,
            )
            .unwrap();
        assert!(
            router
                .route(
                    &frame,
                    ScenePointerPhase::Move,
                    ScenePoint { x: 1.0e9, y: 1.0e9 },
                    1,
                    1,
                )
                .is_err()
        );
        assert_eq!(router.cancel(), Some(PanelId::from("bounded-capture")));
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
    fn flattened_convex_scene_clip_constrains_surface_geometry_and_hit_testing() {
        use crate::{Path2D, PathCommand2D};

        let mut root = SceneNode::group(rect(0.0, 0.0, 40.0, 30.0));
        let mut panel = SceneNode::panel("clipped", rect(0.0, 0.0, 20.0, 10.0));
        panel.transform = Affine2::translate(5.0, 3.0).unwrap();
        root.children.push(panel);
        let frame = SceneFrame::build(&root, 12, 4).unwrap();
        let k = 0.552_284_749_830_793_6;
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [14.0, 5.0] },
                PathCommand2D::CubicTo {
                    control1: [14.0, 5.0 + 4.0 * k],
                    control2: [10.0 + 4.0 * k, 9.0],
                    point: [10.0, 9.0],
                },
                PathCommand2D::CubicTo {
                    control1: [10.0 - 4.0 * k, 9.0],
                    control2: [6.0, 5.0 + 4.0 * k],
                    point: [6.0, 5.0],
                },
                PathCommand2D::CubicTo {
                    control1: [6.0, 5.0 - 4.0 * k],
                    control2: [10.0 - 4.0 * k, 1.0],
                    point: [10.0, 1.0],
                },
                PathCommand2D::CubicTo {
                    control1: [10.0 + 4.0 * k, 1.0],
                    control2: [14.0, 5.0 - 4.0 * k],
                    point: [14.0, 5.0],
                },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.1, 128)
        .unwrap();
        let clips = BTreeMap::from([(PanelId::from("clipped"), path.clone())]);
        let clipped = frame.with_panel_clip_paths(&clips, 12, 4).unwrap();
        assert!(!clipped.requires_indexed_surface_mesh());
        let surface = &clipped.surface_geometry(12, 4).unwrap()[0];
        assert!(surface.vertices.len() > 4);
        assert!(surface.vertices.iter().all(|vertex| {
            path.contains([vertex.position.x - 5.0, vertex.position.y - 3.0])
                .unwrap()
        }));
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 15.0, y: 8.0 }, 12, 4)
                .unwrap(),
            Some(PanelId::from("clipped"))
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 11.0, y: 4.0 }, 12, 4)
                .unwrap(),
            None
        );
        assert!(clipped.with_panel_clip_paths(&clips, 13, 4).is_err());
    }

    #[test]
    fn scene_path_clips_reject_concave_and_multi_contour_paths() {
        use crate::{Path2D, PathCommand2D};

        let frame = SceneFrame::build(&SceneNode::panel("panel", rect(0.0, 0.0, 20.0, 10.0)), 2, 3)
            .unwrap();
        let concave = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [5.0, 5.0] },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        assert!(
            frame
                .with_panel_clip_paths(&BTreeMap::from([(PanelId::from("panel"), concave)]), 2, 3)
                .is_err()
        );
        let multiple = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [8.0, 0.0] },
                PathCommand2D::LineTo { point: [8.0, 8.0] },
                PathCommand2D::LineTo { point: [0.0, 8.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo { point: [2.0, 2.0] },
                PathCommand2D::LineTo { point: [6.0, 2.0] },
                PathCommand2D::LineTo { point: [6.0, 6.0] },
                PathCommand2D::LineTo { point: [2.0, 6.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        assert!(
            frame
                .with_panel_clip_paths(&BTreeMap::from([(PanelId::from("panel"), multiple)]), 2, 3)
                .is_err()
        );
    }

    #[test]
    fn arbitrary_mesh_clips_render_concave_paths_and_preserve_even_odd_holes() {
        use crate::{Path2D, PathCommand2D};

        let frame = SceneFrame::build(&SceneNode::panel("panel", rect(0.0, 0.0, 20.0, 20.0)), 2, 3)
            .unwrap();
        let concave = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 5.0] },
                PathCommand2D::LineTo { point: [5.0, 5.0] },
                PathCommand2D::LineTo { point: [5.0, 20.0] },
                PathCommand2D::LineTo { point: [0.0, 20.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        let clipped = frame
            .with_panel_mesh_clip_paths(
                &BTreeMap::from([(PanelId::from("panel"), concave.clone())]),
                2,
                3,
            )
            .unwrap();
        assert!(clipped.requires_indexed_surface_mesh());
        let mesh = &clipped.surface_mesh_geometry(2, 3).unwrap()[0];
        assert!(!mesh.indices.is_empty());
        assert!(clipped.surface_geometry(2, 3).is_err());
        assert_eq!(mesh.indices.len() % 3, 0);
        for triangle in mesh.vertices.as_chunks::<3>().0 {
            let center = [
                triangle.iter().map(|vertex| vertex.position.x).sum::<f64>() / 3.0,
                triangle.iter().map(|vertex| vertex.position.y).sum::<f64>() / 3.0,
            ];
            assert!(concave.contains(center).unwrap());
        }
        assert!(mesh.vertices.iter().all(|vertex| {
            (0.0..=1.0).contains(&vertex.uv.x) && (0.0..=1.0).contains(&vertex.uv.y)
        }));
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 2.0, y: 10.0 }, 2, 3)
                .unwrap(),
            Some(PanelId::from("panel"))
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 10.0, y: 10.0 }, 2, 3)
                .unwrap(),
            None
        );

        let donut = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [20.0, 20.0],
                },
                PathCommand2D::LineTo { point: [0.0, 20.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo { point: [5.0, 5.0] },
                PathCommand2D::LineTo { point: [15.0, 5.0] },
                PathCommand2D::LineTo {
                    point: [15.0, 15.0],
                },
                PathCommand2D::LineTo { point: [5.0, 15.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        let donut_frame = frame
            .with_panel_mesh_clip_paths(
                &BTreeMap::from([(PanelId::from("panel"), donut.clone())]),
                2,
                3,
            )
            .unwrap();
        let mesh = &donut_frame.surface_mesh_geometry(2, 3).unwrap()[0];
        assert!(!mesh.indices.is_empty());
        for triangle in mesh.vertices.as_chunks::<3>().0 {
            let center = [
                triangle.iter().map(|vertex| vertex.position.x).sum::<f64>() / 3.0,
                triangle.iter().map(|vertex| vertex.position.y).sum::<f64>() / 3.0,
            ];
            assert!(donut.contains(center).unwrap());
        }
        assert_eq!(
            donut_frame
                .hit_test(ScenePoint { x: 10.0, y: 10.0 }, 2, 3)
                .unwrap(),
            None
        );
    }

    #[test]
    fn shared_group_mesh_clip_applies_to_members_for_rendering_and_hit_testing() {
        use crate::{Path2D, PathCommand2D};

        let mut root = SceneNode::group(rect(0.0, 0.0, 70.0, 30.0));
        let mut right = SceneNode::panel("group-right", rect(0.0, 0.0, 15.0, 30.0));
        let mut outside = SceneNode::panel("not-in-group", rect(0.0, 0.0, 10.0, 10.0));
        right.transform = Affine2::translate(25.0, 0.0).unwrap();
        outside.transform = Affine2::translate(50.0, 0.0).unwrap();
        root.children = vec![
            {
                let mut left = SceneNode::panel("group-left", rect(0.0, 0.0, 15.0, 30.0));
                left.transform = Affine2::translate(10.0, 0.0).unwrap();
                left
            },
            right,
            outside,
        ];
        let frame = SceneFrame::build(&root, 7, 2).unwrap();
        let donut = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [30.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [30.0, 30.0],
                },
                PathCommand2D::LineTo { point: [0.0, 30.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo {
                    point: [20.0, 10.0],
                },
                PathCommand2D::LineTo {
                    point: [20.0, 20.0],
                },
                PathCommand2D::LineTo {
                    point: [10.0, 20.0],
                },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        let group = SceneGroupClip {
            panels: BTreeSet::from([PanelId::from("group-left"), PanelId::from("group-right")]),
            path: donut.clone(),
            world_transform: Affine2::translate(10.0, 0.0).unwrap(),
        };
        let clipped = frame.with_group_mesh_clip_paths(&[group], 7, 2).unwrap();
        assert!(clipped.requires_indexed_surface_mesh());
        assert_eq!(clipped.surface_mesh_geometry(7, 2).unwrap().len(), 3);
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 15.0, y: 5.0 }, 7, 2)
                .unwrap(),
            Some(PanelId::from("group-left"))
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 35.0, y: 5.0 }, 7, 2)
                .unwrap(),
            Some(PanelId::from("group-right"))
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 25.0, y: 15.0 }, 7, 2)
                .unwrap(),
            None
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 55.0, y: 5.0 }, 7, 2)
                .unwrap(),
            Some(PanelId::from("not-in-group"))
        );

        let shifted = clipped
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("group-left"),
                    Affine2::translate(100.0, 0.0).unwrap(),
                )]),
                7,
                2,
            )
            .unwrap();
        assert_eq!(
            shifted
                .hit_test(ScenePoint { x: 115.0, y: 5.0 }, 7, 2)
                .unwrap(),
            None,
            "shared group-space clips stay fixed when a panel is decoratively transformed"
        );
        let too_many_groups = vec![
            SceneGroupClip {
                panels: BTreeSet::from([PanelId::from("group-left")]),
                path: donut.clone(),
                world_transform: Affine2::IDENTITY,
            };
            MAX_SCENE_NODES + 1
        ];
        assert!(
            frame
                .with_group_mesh_clip_paths(&too_many_groups, 7, 2)
                .is_err()
        );
        let too_many_members = (0..=MAX_SCENE_NODES)
            .map(|index| PanelId::from(format!("excess-member-{index}")))
            .collect::<BTreeSet<_>>();
        assert!(
            frame
                .with_group_mesh_clip_paths(
                    &[SceneGroupClip {
                        panels: too_many_members,
                        path: donut.clone(),
                        world_transform: Affine2::IDENTITY,
                    }],
                    7,
                    2,
                )
                .is_err()
        );
        assert!(
            frame
                .with_group_mesh_clip_paths(
                    &[SceneGroupClip {
                        panels: BTreeSet::from([PanelId::from("missing")]),
                        path: donut,
                        world_transform: Affine2::IDENTITY,
                    }],
                    7,
                    2
                )
                .is_err()
        );
    }

    #[test]
    fn subtree_group_clip_derives_descendant_members_and_composed_transform() {
        use crate::{Path2D, PathCommand2D};

        let mut root = SceneNode::group(rect(0.0, 0.0, 80.0, 40.0));
        root.transform = Affine2::translate(2.0, 3.0).unwrap();
        let mut outer = SceneNode::group(rect(0.0, 0.0, 50.0, 30.0));
        outer.transform = Affine2::translate(10.0, 0.0).unwrap();
        let mut inner = SceneNode::group(rect(0.0, 0.0, 30.0, 20.0));
        inner.transform = Affine2::translate(5.0, 0.0).unwrap();
        inner.children = vec![
            SceneNode::panel("subtree-first", rect(0.0, 0.0, 10.0, 10.0)),
            SceneNode::panel("subtree-second", rect(10.0, 0.0, 10.0, 10.0)),
        ];
        outer.children = vec![inner];
        let mut outside = SceneNode::panel("subtree-outside", rect(0.0, 0.0, 10.0, 10.0));
        outside.transform = Affine2::translate(40.0, 0.0).unwrap();
        root.children = vec![outer, outside];

        let frame = SceneFrame::build(&root, 11, 4).unwrap();
        let local_clip = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 16)
        .unwrap();
        let clipped = frame
            .with_group_mesh_clip_for_subtree(&root, &[0, 0], local_clip.clone(), 11, 4)
            .unwrap();

        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 20.0, y: 8.0 }, 11, 4)
                .unwrap(),
            Some(PanelId::from("subtree-first")),
            "the nested group transform includes every ancestor"
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 30.0, y: 8.0 }, 11, 4)
                .unwrap(),
            None,
            "the same path is applied to every visible descendant"
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 45.0, y: 8.0 }, 11, 4)
                .unwrap(),
            Some(PanelId::from("subtree-outside")),
            "panels outside the selected subtree are not members"
        );
        let meshes = clipped.surface_mesh_geometry(11, 4).unwrap();
        assert!(
            meshes
                .iter()
                .find(|mesh| mesh.panel == PanelId::from("subtree-first"))
                .is_some_and(|mesh| !mesh.indices.is_empty())
        );
        assert!(
            !meshes
                .iter()
                .any(|mesh| mesh.panel == PanelId::from("subtree-second")),
            "fully clipped-away surfaces are omitted from mesh output"
        );
        assert!(
            meshes
                .iter()
                .find(|mesh| mesh.panel == PanelId::from("subtree-outside"))
                .is_some_and(|mesh| !mesh.indices.is_empty())
        );
        assert!(
            frame
                .with_group_mesh_clip_for_subtree(&root, &[0, 0, 0], local_clip.clone(), 11, 4)
                .is_err()
        );
        assert!(
            frame
                .with_group_mesh_clip_for_subtree(&root, &[0, 9], local_clip.clone(), 11, 4)
                .is_err()
        );
        let mut stale_root = root.clone();
        stale_root.children[0].transform = Affine2::translate(11.0, 0.0).unwrap();
        assert!(
            frame
                .with_group_mesh_clip_for_subtree(&stale_root, &[0, 0], local_clip, 11, 4,)
                .is_err(),
            "a mismatched scene tree must fail closed"
        );
    }

    #[test]
    fn canonical_layout_projection_builds_group_identity_bindings_for_shape_clipping() {
        use doofusdock_core::{Node, Orientation, PanelDescriptor};

        let mut document = LayoutDocument::new("scene-layout", "workspace");
        std::sync::Arc::make_mut(&mut document.panels)
            .extend(["first", "hidden", "outside"].map(|panel| PanelDescriptor::new(panel, panel)));
        document.root = Some(Node::split(
            "split-root",
            Orientation::Horizontal,
            vec![
                Node::tabs(
                    "tabs-group",
                    vec![
                        Node::panel("first-node", "first"),
                        Node::panel("hidden-node", "hidden"),
                    ],
                    0,
                ),
                Node::panel("outside-node", "outside"),
            ],
            vec![0.5, 0.5],
        ));

        let projection = LayoutSceneProjection::from_layout(
            &document,
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
            7,
        )
        .unwrap();
        let tabs_id = NodeId::from("tabs-group");
        let snapshot = crate::project(
            &document,
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
        )
        .unwrap();
        let from_snapshot = LayoutSceneProjection::from_snapshot(
            &document,
            &snapshot,
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
            7,
        )
        .unwrap();
        assert_eq!(
            from_snapshot.group_identities.child_path(&tabs_id),
            projection.group_identities.child_path(&tabs_id)
        );
        assert_eq!(projection.group_identities.len(), 2);
        assert!(projection.group_identities.child_path(&tabs_id).is_some());

        let clipped = projection
            .frame
            .with_group_mesh_clip_for_identity(
                &projection.scene_root,
                &projection.group_identities,
                &tabs_id,
                Path2D {
                    commands: vec![
                        PathCommand2D::MoveTo { point: [0.0, 0.0] },
                        PathCommand2D::LineTo { point: [50.0, 0.0] },
                        PathCommand2D::LineTo {
                            point: [50.0, 100.0],
                        },
                        PathCommand2D::LineTo {
                            point: [0.0, 100.0],
                        },
                        PathCommand2D::Close,
                    ],
                }
                .flatten(0.5, 128)
                .unwrap(),
                document.revision,
                7,
            )
            .unwrap();

        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 25.0, y: 25.0 }, document.revision, 7)
                .unwrap(),
            Some(PanelId::from("first"))
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 75.0, y: 25.0 }, document.revision, 7)
                .unwrap(),
            None
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 125.0, y: 25.0 }, document.revision, 7)
                .unwrap(),
            Some(PanelId::from("outside"))
        );
    }
    #[test]
    fn canonical_group_identity_binding_resolves_current_scene_path_and_rejects_stale_maps() {
        use crate::{Path2D, PathCommand2D, SceneGroupIdentityMap};

        let mut root = SceneNode::group(rect(0.0, 0.0, 50.0, 20.0));
        let mut group = SceneNode::group(rect(0.0, 0.0, 30.0, 20.0));
        group.transform = Affine2::translate(5.0, 0.0).unwrap();
        group.children = vec![SceneNode::panel(
            "identity-member",
            rect(0.0, 0.0, 20.0, 20.0),
        )];
        let mut outside = SceneNode::panel("identity-outside", rect(0.0, 0.0, 10.0, 10.0));
        outside.transform = Affine2::translate(40.0, 0.0).unwrap();
        root.children = vec![group, outside];

        let group_id = NodeId::from("canonical-group-42");
        let identities =
            SceneGroupIdentityMap::from_scene_paths(&root, [(group_id.clone(), vec![0])]).unwrap();
        assert_eq!(identities.len(), 1);
        assert_eq!(identities.child_path(&group_id), Some([0].as_slice()));

        let frame = SceneFrame::build(&root, 12, 5).unwrap();
        let clip = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 20.0],
                },
                PathCommand2D::LineTo { point: [0.0, 20.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 16)
        .unwrap();
        let clipped = frame
            .with_group_mesh_clip_for_identity(&root, &identities, &group_id, clip.clone(), 12, 5)
            .unwrap();
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 8.0, y: 5.0 }, 12, 5)
                .unwrap(),
            Some(PanelId::from("identity-member"))
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 20.0, y: 5.0 }, 12, 5)
                .unwrap(),
            None
        );
        assert_eq!(
            clipped
                .hit_test(ScenePoint { x: 47.0, y: 5.0 }, 12, 5)
                .unwrap(),
            Some(PanelId::from("identity-outside"))
        );

        let missing = NodeId::from("missing-group");
        assert!(
            frame
                .with_group_mesh_clip_for_identity(
                    &root,
                    &identities,
                    &missing,
                    clip.clone(),
                    12,
                    5,
                )
                .is_err()
        );
        assert!(
            SceneGroupIdentityMap::from_scene_paths(&root, [(NodeId::from(""), vec![0])]).is_err()
        );
        assert!(
            SceneGroupIdentityMap::from_scene_paths(
                &root,
                [(group_id.clone(), vec![0]), (group_id.clone(), vec![0])]
            )
            .is_err()
        );
        assert!(
            SceneGroupIdentityMap::from_scene_paths(
                &root,
                [
                    (NodeId::from("group-one"), vec![0]),
                    (NodeId::from("group-two"), vec![0])
                ]
            )
            .is_err()
        );
        assert!(
            SceneGroupIdentityMap::from_scene_paths(
                &root,
                [(NodeId::from("panel-is-not-a-group"), vec![0, 0])]
            )
            .is_err()
        );

        let mut changed = root.clone();
        changed.children[0].transform = Affine2::translate(6.0, 0.0).unwrap();
        assert!(
            frame
                .with_group_mesh_clip_for_identity(&changed, &identities, &group_id, clip, 12, 5,)
                .is_err()
        );
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
    fn transformed_scene_hit_testing_respects_optional_panel_shapes_and_falls_through() {
        let mut root = SceneNode::group(rect(0.0, 0.0, 50.0, 50.0));
        root.children
            .push(SceneNode::panel("lower", rect(0.0, 0.0, 50.0, 50.0)));
        root.children
            .push(SceneNode::panel("upper", rect(0.0, 0.0, 50.0, 50.0)));
        let frame = SceneFrame::build(&root, 5, 8).unwrap();
        let shapes = BTreeMap::from([(
            PanelId::from("upper"),
            ValidatedShape2D::new(crate::Shape2D::Circle {
                center: [25.0, 25.0],
                radius: 10.0,
            })
            .unwrap(),
        )]);

        let center = frame
            .hit_test_detail_with_shapes(ScenePoint { x: 25.0, y: 25.0 }, 5, 8, &shapes)
            .unwrap()
            .unwrap();
        assert_eq!(center.panel, PanelId::from("upper"));

        let corner = frame
            .hit_test_detail_with_shapes(ScenePoint { x: 2.0, y: 2.0 }, 5, 8, &shapes)
            .unwrap()
            .unwrap();
        assert_eq!(corner.panel, PanelId::from("lower"));

        let stale_shape = BTreeMap::from([(
            PanelId::from("removed"),
            ValidatedShape2D::new(crate::Shape2D::Circle {
                center: [1.0, 1.0],
                radius: 1.0,
            })
            .unwrap(),
        )]);
        assert_eq!(
            frame
                .hit_test_detail_with_shapes(ScenePoint { x: 2.0, y: 2.0 }, 5, 8, &stale_shape)
                .unwrap_err()
                .code,
            ErrorCode::NotFound
        );
    }

    #[test]
    fn transformed_scene_hit_testing_respects_optional_compound_paths() {
        let mut root = SceneNode::group(rect(0.0, 0.0, 50.0, 50.0));
        root.children
            .push(SceneNode::panel("lower", rect(0.0, 0.0, 50.0, 50.0)));
        root.children
            .push(SceneNode::panel("upper", rect(0.0, 0.0, 50.0, 50.0)));
        let frame = SceneFrame::build(&root, 5, 8)
            .unwrap()
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("upper"),
                    Affine2::translate(10.0, 5.0).unwrap(),
                )]),
                5,
                8,
            )
            .unwrap();
        let path = crate::Path2D {
            commands: vec![
                crate::PathCommand2D::MoveTo {
                    point: [20.0, 20.0],
                },
                crate::PathCommand2D::LineTo {
                    point: [30.0, 20.0],
                },
                crate::PathCommand2D::LineTo {
                    point: [25.0, 30.0],
                },
                crate::PathCommand2D::Close,
            ],
        }
        .flatten(1.0, 64)
        .unwrap();
        let paths = BTreeMap::from([(PanelId::from("upper"), path)]);

        let inside = frame
            .hit_test_detail_with_paths(ScenePoint { x: 35.0, y: 30.0 }, 5, 8, &paths)
            .unwrap()
            .unwrap();
        assert_eq!(inside.panel, PanelId::from("upper"));

        let outside = frame
            .hit_test_detail_with_paths(ScenePoint { x: 31.0, y: 34.0 }, 5, 8, &paths)
            .unwrap()
            .unwrap();
        assert_eq!(outside.panel, PanelId::from("lower"));

        let stale = BTreeMap::from([(
            PanelId::from("removed"),
            crate::Path2D {
                commands: vec![
                    crate::PathCommand2D::MoveTo { point: [0.0, 0.0] },
                    crate::PathCommand2D::LineTo { point: [1.0, 0.0] },
                    crate::PathCommand2D::LineTo { point: [0.0, 1.0] },
                    crate::PathCommand2D::Close,
                ],
            }
            .flatten(1.0, 8)
            .unwrap(),
        )]);
        assert_eq!(
            frame
                .hit_test_detail_with_paths(ScenePoint { x: 0.5, y: 0.5 }, 5, 8, &stale)
                .unwrap_err()
                .code,
            ErrorCode::NotFound
        );
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
