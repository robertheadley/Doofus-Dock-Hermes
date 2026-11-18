//! Revision-bound spatial queries over a projected scene.
//!
//! The index is disposable derived state. It can answer interaction queries
//! only for the canonical revision and presentation generation it was built
//! from; callers must rebuild it after either changes.
use crate::{Box2, Snapshot};
use doofusdock_core::{DockError, DockResult, ErrorCode, PanelId};
use std::collections::{BTreeMap, BTreeSet};

const MAX_CELLS_PER_PANEL: usize = 4096;
const MAX_TOTAL_MEMBERSHIPS: usize = 1_000_000;
const LINEAR_POINT_QUERY_MAX_PANELS: usize = 16;
const LINEAR_RECT_QUERY_MAX_PANELS: usize = 100;

#[derive(Clone, Debug)]
struct IndexedPanel {
    visual_bounds: Box2,
    interaction_bounds: Box2,
    docking_bounds: Box2,
    collision_bounds: Box2,
    clip: Box2,
    z: i32,
    collision_policy: CollisionPolicy,
}

/// Which projected boundary a spatial query should use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialBoundsKind {
    Visual,
    Interaction,
    Docking,
    Collision,
}

/// How a panel participates when another panel overlaps its collision bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollisionPolicy {
    /// Overlap is forbidden when a caller requests collision rejection.
    #[default]
    Solid,
    /// Overlap is allowed and remains visible in collision diagnostics.
    Overlap,
    /// Ignore this panel when evaluating collision candidates.
    PassThrough,
    /// Behaves as a collision target and may attract a moving panel.
    Magnetic,
    /// May be a snap target, but its bounds do not block overlap.
    SnapOnly,
}

/// Derived workspace-space boundaries and collision behavior for one panel.
/// The visual bounds continue to come from the canonical presentation snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelSpatialBounds {
    pub interaction: Box2,
    pub docking: Box2,
    pub collision: Box2,
    pub collision_policy: CollisionPolicy,
}

impl PanelSpatialBounds {
    /// Preserve the legacy behavior where every boundary equals the projection.
    pub fn uniform(bounds: Box2) -> Self {
        Self {
            interaction: bounds,
            docking: bounds,
            collision: bounds,
            collision_policy: CollisionPolicy::Solid,
        }
    }
}

/// Uniform-grid index over visible projected panels.
///
/// Construction is bounded and deterministic. A cell size of 128 logical
/// points is a useful default for desktop workspaces; applications can choose
/// another size for their scene scale.
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    revision: u64,
    generation: u64,
    cell_size: f64,
    panels: BTreeMap<PanelId, IndexedPanel>,
    cells: BTreeMap<(i64, i64), Vec<PanelId>>,
}

impl SpatialIndex {
    pub fn build(
        snapshot: &Snapshot,
        revision: u64,
        generation: u64,
        cell_size: f64,
    ) -> DockResult<Self> {
        Self::build_with_panel_bounds(snapshot, revision, generation, cell_size, &BTreeMap::new())
    }

    /// Build an index with caller-supplied derived boundaries for individual
    /// panels. Missing entries use the legacy projected visual bounds for all
    /// query purposes. Supplied bounds are clipped by the same projected clip.
    pub fn build_with_panel_bounds(
        snapshot: &Snapshot,
        revision: u64,
        generation: u64,
        cell_size: f64,
        panel_bounds: &BTreeMap<PanelId, PanelSpatialBounds>,
    ) -> DockResult<Self> {
        if !cell_size.is_finite() || !(1.0..=1_000_000.0).contains(&cell_size) {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Spatial cell size must be finite and within the supported range.",
                cell_size.to_string(),
            ));
        }

        let mut index = Self {
            revision,
            generation,
            cell_size,
            panels: BTreeMap::new(),
            cells: BTreeMap::new(),
        };
        let mut memberships = 0usize;
        for panel in panel_bounds.keys() {
            if !snapshot.panels.contains_key(panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Spatial boundary metadata targets a panel absent from the snapshot.",
                    panel.0.clone(),
                ));
            }
        }
        for (panel, supplied) in panel_bounds {
            for (kind, bounds) in [
                (SpatialBoundsKind::Interaction, supplied.interaction),
                (SpatialBoundsKind::Docking, supplied.docking),
                (SpatialBoundsKind::Collision, supplied.collision),
            ] {
                if !bounds.valid() || bounds.w <= 0.0 || bounds.h <= 0.0 {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Spatial panel boundary must be finite and positive.",
                        format!("panel={}:kind={kind:?}", panel.0),
                    ));
                }
            }
        }
        for (panel, visual) in &snapshot.panels {
            if visual.opacity < 0.5 || visual.bounds.w <= 0.0 || visual.bounds.h <= 0.0 {
                continue;
            }
            if !visual.bounds.valid() || !visual.clip.valid() {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Cannot index invalid projected panel geometry.",
                    panel.0.clone(),
                ));
            }
            let visual_bounds = intersection(visual.bounds, visual.clip);
            let supplied = panel_bounds
                .get(panel)
                .copied()
                .unwrap_or_else(|| PanelSpatialBounds::uniform(visual.bounds));
            let interaction_bounds = intersection(supplied.interaction, visual.clip);
            let docking_bounds = intersection(supplied.docking, visual.clip);
            let collision_bounds = intersection(supplied.collision, visual.clip);
            let Some(index_bounds) = union_nonempty([
                visual_bounds,
                interaction_bounds,
                docking_bounds,
                collision_bounds,
            ]) else {
                continue;
            };
            let (left, top, right, bottom) = index.cell_range(index_bounds)?;
            let count = cell_count(left, top, right, bottom)?;
            if count > MAX_CELLS_PER_PANEL
                || memberships.saturating_add(count) > MAX_TOTAL_MEMBERSHIPS
            {
                return Err(DockError::new(
                    ErrorCode::ResourceLimit,
                    "Projected scene exceeds the spatial-index membership limit.",
                    panel.0.clone(),
                ));
            }
            memberships += count;
            index.panels.insert(
                panel.clone(),
                IndexedPanel {
                    visual_bounds,
                    interaction_bounds,
                    docking_bounds,
                    collision_bounds,
                    clip: visual.clip,
                    z: visual.z,
                    collision_policy: supplied.collision_policy,
                },
            );
            for y in top..=bottom {
                for x in left..=right {
                    index.cells.entry((x, y)).or_default().push(panel.clone());
                }
            }
        }
        // BTreeMap iteration and panel insertion are stable; dedupe in case a
        // future index strategy inserts the same object more than once.
        for ids in index.cells.values_mut() {
            ids.sort();
            ids.dedup();
        }
        Ok(index)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    /// Ensure a query is tied to the current authoritative snapshot.
    pub fn validate_snapshot(&self, revision: u64, generation: u64) -> DockResult<()> {
        if self.revision != revision || self.generation != generation {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Spatial index is stale and cannot authorize an interaction.",
                format!(
                    "index=({},{}), current=({revision},{generation})",
                    self.revision, self.generation
                ),
            ));
        }
        Ok(())
    }

    /// Return the frontmost visible panel containing a point.
    pub fn point_containing(&self, x: f64, y: f64) -> Option<PanelId> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        if self.panels.len() <= LINEAR_POINT_QUERY_MAX_PANELS {
            return self
                .panels
                .iter()
                .filter(|(_, panel)| {
                    panel.interaction_bounds.contains(x, y) && panel.clip.contains(x, y)
                })
                .max_by(|(id_a, a), (id_b, b)| a.z.cmp(&b.z).then_with(|| id_b.cmp(id_a)))
                .map(|(id, _)| id.clone());
        }
        let cell = (floor_cell(x, self.cell_size), floor_cell(y, self.cell_size));
        self.cells
            .get(&cell)?
            .iter()
            .filter_map(|id| self.panels.get(id).map(|panel| (id, panel)))
            .filter(|(_, panel)| {
                panel.interaction_bounds.contains(x, y) && panel.clip.contains(x, y)
            })
            .max_by(|(id_a, a), (id_b, b)| a.z.cmp(&b.z).then_with(|| id_b.cmp(id_a)))
            .map(|(id, _)| id.clone())
    }

    /// Return panels intersecting a query rectangle in stable panel-ID order.
    pub fn intersecting(&self, rect: Box2) -> DockResult<Vec<PanelId>> {
        self.intersecting_kind(rect, SpatialBoundsKind::Visual)
    }

    /// Return panels intersecting a query rectangle using a selected boundary.
    pub fn intersecting_kind(
        &self,
        rect: Box2,
        kind: SpatialBoundsKind,
    ) -> DockResult<Vec<PanelId>> {
        if !rect.valid() {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Spatial query rectangle is invalid.",
                "intersecting",
            ));
        }
        if rect.w == 0.0 || rect.h == 0.0 {
            return Ok(Vec::new());
        }
        if self.panels.len() <= LINEAR_RECT_QUERY_MAX_PANELS {
            return Ok(self
                .panels
                .iter()
                .filter_map(|(id, panel)| {
                    let bounds = match kind {
                        SpatialBoundsKind::Visual => panel.visual_bounds,
                        SpatialBoundsKind::Interaction => panel.interaction_bounds,
                        SpatialBoundsKind::Docking => panel.docking_bounds,
                        SpatialBoundsKind::Collision => panel.collision_bounds,
                    };
                    overlaps(bounds, rect).then(|| id.clone())
                })
                .collect());
        }
        let (left, top, right, bottom) = self.cell_range(rect)?;
        let count = cell_count(left, top, right, bottom)?;
        if count > MAX_TOTAL_MEMBERSHIPS {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Spatial query exceeds the bounded cell scan limit.",
                "intersecting",
            ));
        }
        let mut candidates = BTreeSet::new();
        for y in top..=bottom {
            for x in left..=right {
                if let Some(ids) = self.cells.get(&(x, y)) {
                    candidates.extend(ids.iter().cloned());
                }
            }
        }
        Ok(candidates
            .into_iter()
            .filter(|id| {
                self.panels.get(id).is_some_and(|panel| {
                    let bounds = match kind {
                        SpatialBoundsKind::Visual => panel.visual_bounds,
                        SpatialBoundsKind::Interaction => panel.interaction_bounds,
                        SpatialBoundsKind::Docking => panel.docking_bounds,
                        SpatialBoundsKind::Collision => panel.collision_bounds,
                    };
                    overlaps(bounds, rect)
                })
            })
            .collect())
    }

    /// Return panels whose docking boundaries intersect the supplied region.
    pub fn docking_targets(&self, rect: Box2) -> DockResult<Vec<PanelId>> {
        self.intersecting_kind(rect, SpatialBoundsKind::Docking)
    }

    /// Return collision-bound intersections with their declared behavior.
    pub fn collision_candidates(&self, rect: Box2) -> DockResult<Vec<(PanelId, CollisionPolicy)>> {
        self.intersecting_kind(rect, SpatialBoundsKind::Collision)
            .map(|ids| {
                ids.into_iter()
                    .filter_map(|id| {
                        let policy = self.panels.get(&id)?.collision_policy;
                        (policy != CollisionPolicy::PassThrough).then_some((id, policy))
                    })
                    .collect()
            })
    }

    /// Find the best directional focus candidate using projected geometry.
    /// Primary-axis distance wins, then perpendicular distance, then stable ID.
    pub fn focus_direction(&self, from: &PanelId, direction: FocusDirection) -> Option<PanelId> {
        let source = self.panels.get(from)?;
        let (sx, sy) = center(source.visual_bounds);
        self.panels
            .iter()
            .filter(|(id, _)| *id != from)
            .filter_map(|(id, panel)| {
                let (x, y) = center(panel.visual_bounds);
                let dx = x - sx;
                let dy = y - sy;
                let (primary, secondary) = match direction {
                    FocusDirection::Left if dx < 0.0 => (-dx, dy.abs()),
                    FocusDirection::Right if dx > 0.0 => (dx, dy.abs()),
                    FocusDirection::Up if dy < 0.0 => (-dy, dx.abs()),
                    FocusDirection::Down if dy > 0.0 => (dy, dx.abs()),
                    _ => return None,
                };
                Some((id, primary, secondary))
            })
            .min_by(|(id_a, p_a, s_a), (id_b, p_b, s_b)| {
                p_a.total_cmp(p_b)
                    .then_with(|| s_a.total_cmp(s_b))
                    .then_with(|| id_a.cmp(id_b))
            })
            .map(|(id, _, _)| id.clone())
    }

    pub fn nearest(&self, x: f64, y: f64) -> Option<PanelId> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        self.panels
            .iter()
            .map(|(id, panel)| {
                let (cx, cy) = center(panel.visual_bounds);
                (id, (cx - x).mul_add(cx - x, (cy - y) * (cy - y)))
            })
            .min_by(|(id_a, da), (id_b, db)| da.total_cmp(db).then_with(|| id_a.cmp(id_b)))
            .map(|(id, _)| id.clone())
    }

    fn cell_range(&self, bounds: Box2) -> DockResult<(i64, i64, i64, i64)> {
        let left = floor_cell(bounds.x, self.cell_size);
        let top = floor_cell(bounds.y, self.cell_size);
        let right = floor_cell(bounds.x + bounds.w, self.cell_size);
        let bottom = floor_cell(bounds.y + bounds.h, self.cell_size);
        if [left, top, right, bottom]
            .iter()
            .any(|n| n.abs() > 1_000_000)
        {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Spatial geometry falls outside the supported cell range.",
                "spatial-index",
            ));
        }
        Ok((left, top, right, bottom))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapAxis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    Edge,
    Center,
    Grid,
    EqualSpacing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapGuide {
    pub axis: SnapAxis,
    pub coordinate: f64,
    pub kind: SnapKind,
    pub target: Option<PanelId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapResult {
    pub bounds: Box2,
    pub guides: Vec<SnapGuide>,
    /// Stable IDs of non-pass-through panels whose collision bounds overlap the result.
    pub collisions: Vec<PanelId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SnapCollisionPolicy {
    /// Return collision metadata and leave the host to decide what to do.
    #[default]
    Report,
    /// Reject a snapped result that intersects a Solid or Magnetic collision boundary.
    Reject,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapSettings {
    /// Attraction threshold in physical screen points.
    pub threshold_points: f64,
    pub dpi_scale: f64,
    pub zoom: f64,
    /// Optional workspace-space grid spacing.
    pub grid_spacing: Option<f64>,
    /// Higher values win before distance; equal priorities choose the nearest
    /// correction and then the stable target panel ID.
    pub edge_priority: i16,
    pub center_priority: i16,
    pub grid_priority: i16,
    pub collision_policy: SnapCollisionPolicy,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            threshold_points: 12.0,
            dpi_scale: 1.0,
            zoom: 1.0,
            grid_spacing: None,
            edge_priority: 10,
            center_priority: 8,
            grid_priority: 4,
            collision_policy: SnapCollisionPolicy::Report,
        }
    }
}

#[derive(Clone)]
struct SnapCandidate {
    delta: f64,
    coordinate: f64,
    kind: SnapKind,
    priority: i16,
    target: Option<PanelId>,
    feature_order: u8,
}

impl SpatialIndex {
    /// Snap a transient rectangle to nearby projected edges, centers, or a
    /// workspace grid. This returns presentation geometry only.
    pub fn snap_bounds(
        &self,
        moving: Box2,
        moving_panel: Option<&PanelId>,
        settings: SnapSettings,
    ) -> DockResult<SnapResult> {
        self.snap_bounds_with_spacing(moving, moving_panel, settings, None)
    }

    /// Snap a transient rectangle to nearby edges, centers, a grid, or the
    /// equal gap between the nearest visible neighbors on an axis. Equal
    /// spacing is opt-in and its priority competes with the other candidates.
    pub fn snap_bounds_with_spacing(
        &self,
        moving: Box2,
        moving_panel: Option<&PanelId>,
        settings: SnapSettings,
        spacing_priority: Option<i16>,
    ) -> DockResult<SnapResult> {
        if !moving.valid()
            || moving.w <= 0.0
            || moving.h <= 0.0
            || !settings.threshold_points.is_finite()
            || !(0.0..=256.0).contains(&settings.threshold_points)
            || !settings.dpi_scale.is_finite()
            || !(0.25..=8.0).contains(&settings.dpi_scale)
            || !settings.zoom.is_finite()
            || !(0.25..=8.0).contains(&settings.zoom)
            || settings
                .grid_spacing
                .is_some_and(|grid| !grid.is_finite() || !(1.0..=1_000_000.0).contains(&grid))
        {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Snapping geometry or scale configuration is invalid.",
                "snap-bounds",
            ));
        }

        let threshold = settings.threshold_points / (settings.dpi_scale * settings.zoom);
        let query = Box2 {
            x: moving.x - threshold,
            y: moving.y - threshold,
            w: moving.w + threshold * 2.0,
            h: moving.h + threshold * 2.0,
        };
        let nearby = self.intersecting_kind(query, SpatialBoundsKind::Docking)?;
        let mut best_x: Option<SnapCandidate> = None;
        let mut best_y: Option<SnapCandidate> = None;

        for target_id in &nearby {
            if moving_panel.is_some_and(|moving_id| moving_id == target_id) {
                continue;
            }
            let Some(target) = self.panels.get(target_id) else {
                continue;
            };
            collect_axis_candidates(
                &mut best_x,
                SnapAxis::X,
                moving,
                target.docking_bounds,
                settings,
                threshold,
                Some(target_id.clone()),
            );
            collect_axis_candidates(
                &mut best_y,
                SnapAxis::Y,
                moving,
                target.docking_bounds,
                settings,
                threshold,
                Some(target_id.clone()),
            );
        }

        if spacing_priority.is_some() {
            collect_equal_spacing_candidate(
                &mut best_x,
                SnapAxis::X,
                moving,
                &self.panels,
                moving_panel,
                threshold,
                spacing_priority.unwrap_or_default(),
            );
            collect_equal_spacing_candidate(
                &mut best_y,
                SnapAxis::Y,
                moving,
                &self.panels,
                moving_panel,
                threshold,
                spacing_priority.unwrap_or_default(),
            );
        }

        if let Some(grid) = settings.grid_spacing {
            let x = (moving.x / grid).round() * grid;
            let y = (moving.y / grid).round() * grid;
            offer_candidate(
                &mut best_x,
                SnapCandidate {
                    delta: x - moving.x,
                    coordinate: x,
                    kind: SnapKind::Grid,
                    priority: settings.grid_priority,
                    target: None,
                    feature_order: 0,
                },
                threshold,
            );
            offer_candidate(
                &mut best_y,
                SnapCandidate {
                    delta: y - moving.y,
                    coordinate: y,
                    kind: SnapKind::Grid,
                    priority: settings.grid_priority,
                    target: None,
                    feature_order: 0,
                },
                threshold,
            );
        }

        let mut result = SnapResult {
            bounds: moving,
            guides: Vec::with_capacity(2),
            collisions: Vec::new(),
        };
        if let Some(candidate) = best_x {
            result.bounds.x += candidate.delta;
            result.guides.push(SnapGuide {
                axis: SnapAxis::X,
                coordinate: candidate.coordinate,
                kind: candidate.kind,
                target: candidate.target,
            });
        }
        if let Some(candidate) = best_y {
            result.bounds.y += candidate.delta;
            result.guides.push(SnapGuide {
                axis: SnapAxis::Y,
                coordinate: candidate.coordinate,
                kind: candidate.kind,
                target: candidate.target,
            });
        }
        let collision_candidates = self
            .collision_candidates(result.bounds)?
            .into_iter()
            .filter(|(panel, _)| !moving_panel.is_some_and(|moving| moving == panel))
            .collect::<Vec<_>>();
        result.collisions = collision_candidates
            .iter()
            .map(|(panel, _)| panel.clone())
            .collect();
        let blocking_collisions = collision_candidates.iter().any(|(_, policy)| {
            matches!(policy, CollisionPolicy::Solid | CollisionPolicy::Magnetic)
        });
        if settings.collision_policy == SnapCollisionPolicy::Reject && blocking_collisions {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Snapped geometry overlaps one or more blocking collision boundaries.",
                result
                    .collisions
                    .iter()
                    .map(|panel| panel.0.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        Ok(result)
    }
}

fn collect_axis_candidates(
    best: &mut Option<SnapCandidate>,
    axis: SnapAxis,
    moving: Box2,
    target: Box2,
    settings: SnapSettings,
    threshold: f64,
    target_id: Option<PanelId>,
) {
    let (moving_start, moving_end, moving_center, target_start, target_end, target_center) =
        match axis {
            SnapAxis::X => (
                moving.x,
                moving.x + moving.w,
                moving.x + moving.w / 2.0,
                target.x,
                target.x + target.w,
                target.x + target.w / 2.0,
            ),
            SnapAxis::Y => (
                moving.y,
                moving.y + moving.h,
                moving.y + moving.h / 2.0,
                target.y,
                target.y + target.h,
                target.y + target.h / 2.0,
            ),
        };
    let edge_priority = settings.edge_priority;
    let center_priority = settings.center_priority;
    for (feature_order, (source, destination)) in [
        (moving_start, target_start),
        (moving_start, target_end),
        (moving_end, target_start),
        (moving_end, target_end),
    ]
    .into_iter()
    .enumerate()
    {
        offer_candidate(
            best,
            SnapCandidate {
                delta: destination - source,
                coordinate: destination,
                kind: SnapKind::Edge,
                priority: edge_priority,
                target: target_id.clone(),
                feature_order: feature_order as u8,
            },
            threshold,
        );
    }
    offer_candidate(
        best,
        SnapCandidate {
            delta: target_center - moving_center,
            coordinate: target_center,
            kind: SnapKind::Center,
            priority: center_priority,
            target: target_id,
            feature_order: 4,
        },
        threshold,
    );
}

fn collect_equal_spacing_candidate(
    best: &mut Option<SnapCandidate>,
    axis: SnapAxis,
    moving: Box2,
    targets: &BTreeMap<PanelId, IndexedPanel>,
    moving_panel: Option<&PanelId>,
    threshold: f64,
    priority: i16,
) {
    let (moving_start, moving_end, moving_size) = match axis {
        SnapAxis::X => (moving.x, moving.x + moving.w, moving.w),
        SnapAxis::Y => (moving.y, moving.y + moving.h, moving.h),
    };
    let before = targets
        .iter()
        .filter(|(id, panel)| {
            !moving_panel.is_some_and(|moving_id| moving_id == *id)
                && overlaps_cross_axis(axis, moving, panel.visual_bounds)
        })
        .filter_map(|(id, panel)| {
            let end = axis_end(axis, panel.visual_bounds);
            (end <= moving_start).then_some((id, panel.visual_bounds, end))
        })
        .min_by(|(a_id, _, a_end), (b_id, _, b_end)| {
            (moving_start - a_end)
                .total_cmp(&(moving_start - b_end))
                .then_with(|| a_id.cmp(b_id))
        });
    let after = targets
        .iter()
        .filter(|(id, panel)| {
            !moving_panel.is_some_and(|moving_id| moving_id == *id)
                && overlaps_cross_axis(axis, moving, panel.visual_bounds)
        })
        .filter_map(|(id, panel)| {
            let start = axis_start(axis, panel.visual_bounds);
            (start >= moving_end).then_some((id, panel.visual_bounds, start))
        })
        .min_by(|(a_id, _, a_start), (b_id, _, b_start)| {
            (a_start - moving_end)
                .total_cmp(&(b_start - moving_end))
                .then_with(|| a_id.cmp(b_id))
        });
    let (Some((before_id, _, before_end)), Some((_, _, after_start))) = (before, after) else {
        return;
    };
    let available_gap = after_start - before_end;
    if available_gap < moving_size {
        return;
    }
    let desired_start = before_end + (available_gap - moving_size) / 2.0;
    let delta = desired_start - moving_start;
    offer_candidate(
        best,
        SnapCandidate {
            delta,
            coordinate: desired_start,
            kind: SnapKind::EqualSpacing,
            priority,
            target: Some(before_id.clone()),
            feature_order: 5,
        },
        threshold,
    );
}

fn overlaps_cross_axis(axis: SnapAxis, moving: Box2, target: Box2) -> bool {
    let (moving_start, moving_end, target_start, target_end) = match axis {
        SnapAxis::X => (moving.y, moving.y + moving.h, target.y, target.y + target.h),
        SnapAxis::Y => (moving.x, moving.x + moving.w, target.x, target.x + target.w),
    };
    moving_start < target_end && target_start < moving_end
}

fn axis_start(axis: SnapAxis, bounds: Box2) -> f64 {
    match axis {
        SnapAxis::X => bounds.x,
        SnapAxis::Y => bounds.y,
    }
}

fn axis_end(axis: SnapAxis, bounds: Box2) -> f64 {
    match axis {
        SnapAxis::X => bounds.x + bounds.w,
        SnapAxis::Y => bounds.y + bounds.h,
    }
}

fn offer_candidate(best: &mut Option<SnapCandidate>, candidate: SnapCandidate, threshold: f64) {
    if !candidate.delta.is_finite() || candidate.delta.abs() > threshold {
        return;
    }
    let replace = best.as_ref().is_none_or(|current| {
        candidate
            .priority
            .cmp(&current.priority)
            .then_with(|| current.delta.abs().total_cmp(&candidate.delta.abs()))
            .then_with(|| current.target.cmp(&candidate.target))
            .then_with(|| current.feature_order.cmp(&candidate.feature_order))
            .is_gt()
    });
    if replace {
        *best = Some(candidate);
    }
}

fn floor_cell(value: f64, size: f64) -> i64 {
    (value / size).floor() as i64
}

fn cell_count(left: i64, top: i64, right: i64, bottom: i64) -> DockResult<usize> {
    let width = right.saturating_sub(left).saturating_add(1) as u64;
    let height = bottom.saturating_sub(top).saturating_add(1) as u64;
    usize::try_from(width.saturating_mul(height)).map_err(|_| {
        DockError::new(
            ErrorCode::ResourceLimit,
            "Spatial cell range overflowed.",
            "spatial-index",
        )
    })
}

fn center(bounds: Box2) -> (f64, f64) {
    (bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0)
}

fn intersection(a: Box2, b: Box2) -> Box2 {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Box2 {
        x,
        y,
        w: (a.x + a.w).min(b.x + b.w) - x,
        h: (a.y + a.h).min(b.y + b.h) - y,
    }
}

fn union_nonempty(bounds: impl IntoIterator<Item = Box2>) -> Option<Box2> {
    let mut bounds = bounds
        .into_iter()
        .filter(|bounds| bounds.w > 0.0 && bounds.h > 0.0);
    let first = bounds.next()?;
    Some(bounds.fold(first, |union, next| {
        let x = union.x.min(next.x);
        let y = union.y.min(next.y);
        Box2 {
            x,
            y,
            w: union.x.max(union.x + union.w).max(next.x + next.w) - x,
            h: union.y.max(union.y + union.h).max(next.y + next.h) - y,
        }
    }))
}

fn overlaps(a: Box2, b: Box2) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Visual;

    fn snapshot() -> Snapshot {
        let mut snapshot = Snapshot::default();
        let add = |snapshot: &mut Snapshot, id: &str, bounds: Box2, z| {
            snapshot.panels.insert(
                PanelId::from(id),
                Visual {
                    bounds,
                    clip: bounds,
                    opacity: 1.0,
                    z,
                },
            );
        };
        add(
            &mut snapshot,
            "a",
            Box2 {
                x: 0.,
                y: 0.,
                w: 80.,
                h: 80.,
            },
            0,
        );
        add(
            &mut snapshot,
            "b",
            Box2 {
                x: 120.,
                y: 0.,
                w: 80.,
                h: 80.,
            },
            0,
        );
        add(
            &mut snapshot,
            "c",
            Box2 {
                x: 0.,
                y: 120.,
                w: 80.,
                h: 80.,
            },
            0,
        );
        add(
            &mut snapshot,
            "overlay",
            Box2 {
                x: 20.,
                y: 20.,
                w: 40.,
                h: 40.,
            },
            2,
        );
        snapshot
    }

    #[test]
    fn point_and_rect_queries_respect_clips_and_frontmost_z() {
        let index = SpatialIndex::build(&snapshot(), 4, 7, 64.).unwrap();
        assert_eq!(
            index.point_containing(30., 30.),
            Some(PanelId::from("overlay"))
        );
        assert_eq!(index.point_containing(5., 5.), Some(PanelId::from("a")));
        assert_eq!(index.point_containing(f64::NAN, 5.), None);
        assert_eq!(
            index
                .intersecting(Box2 {
                    x: 110.,
                    y: -5.,
                    w: 100.,
                    h: 90.
                })
                .unwrap(),
            vec![PanelId::from("b")]
        );
    }

    #[test]
    fn adaptive_query_paths_preserve_results_across_dispatch_thresholds() {
        for count in [16, 17, 100, 101] {
            let mut projected = Snapshot::default();
            for index in 0..count {
                let bounds = Box2 {
                    x: (index * 200) as f64,
                    y: 0.0,
                    w: 100.0,
                    h: 100.0,
                };
                projected.panels.insert(
                    PanelId::from(format!("panel-{index:04}")),
                    Visual {
                        bounds,
                        clip: bounds,
                        opacity: 1.0,
                        z: index % 3,
                    },
                );
            }
            let index = SpatialIndex::build(&projected, 1, count as u64, 64.0).unwrap();
            assert_eq!(
                index.point_containing(20.0, 20.0),
                Some(PanelId::from("panel-0000")),
                "point query dispatch at {count} panels"
            );
            assert_eq!(
                index
                    .intersecting(Box2 {
                        x: 10.0,
                        y: 10.0,
                        w: 20.0,
                        h: 20.0,
                    })
                    .unwrap(),
                vec![PanelId::from("panel-0000")],
                "rectangle query dispatch at {count} panels"
            );
        }
    }

    #[test]
    fn focus_is_directional_and_stable_for_ties() {
        let index = SpatialIndex::build(&snapshot(), 4, 7, 64.).unwrap();
        assert_eq!(
            index.focus_direction(&PanelId::from("a"), FocusDirection::Right),
            Some(PanelId::from("b"))
        );
        assert_eq!(
            index.focus_direction(&PanelId::from("a"), FocusDirection::Down),
            Some(PanelId::from("c"))
        );
        assert_eq!(
            index.focus_direction(&PanelId::from("b"), FocusDirection::Right),
            None
        );
        assert_eq!(index.nearest(180., 30.), Some(PanelId::from("b")));
    }

    #[test]
    fn rejects_stale_indexes_invalid_configuration_and_unbounded_geometry() {
        let index = SpatialIndex::build(&snapshot(), 4, 7, 64.).unwrap();
        assert!(index.validate_snapshot(5, 7).is_err());
        assert!(index.validate_snapshot(4, 8).is_err());
        assert!(SpatialIndex::build(&snapshot(), 4, 7, 0.).is_err());
        let mut huge = Snapshot::default();
        huge.panels.insert(
            PanelId::from("huge"),
            Visual {
                bounds: Box2 {
                    x: 0.,
                    y: 0.,
                    w: 1_000_000.,
                    h: 1_000_000.,
                },
                clip: Box2 {
                    x: 0.,
                    y: 0.,
                    w: 1_000_000.,
                    h: 1_000_000.,
                },
                opacity: 1.,
                z: 0,
            },
        );
        assert!(SpatialIndex::build(&huge, 0, 0, 1.).is_err());
    }

    #[test]
    fn snapping_aligns_edges_and_centers_in_dpi_and_zoom_adjusted_space() {
        let mut snapshot = Snapshot::default();
        let target_id = PanelId::from("target");
        let target = Box2 {
            x: 98.,
            y: 98.,
            w: 80.,
            h: 80.,
        };
        snapshot.panels.insert(
            target_id.clone(),
            Visual {
                bounds: target,
                clip: target,
                opacity: 1.,
                z: 0,
            },
        );
        let index = SpatialIndex::build(&snapshot, 3, 2, 64.).unwrap();
        let moving = Box2 {
            x: 10.,
            y: 10.,
            w: 80.,
            h: 80.,
        };
        let result = index
            .snap_bounds(
                moving,
                Some(&PanelId::from("moving")),
                SnapSettings {
                    threshold_points: 10.,
                    dpi_scale: 2.,
                    zoom: 0.5,
                    ..SnapSettings::default()
                },
            )
            .unwrap();
        assert_eq!(result.bounds.x, 18.);
        assert_eq!(result.bounds.y, 18.);
        assert_eq!(result.guides.len(), 2);
        assert!(
            result
                .guides
                .iter()
                .all(|guide| guide.kind == SnapKind::Edge)
        );
        assert!(
            result
                .guides
                .iter()
                .all(|guide| guide.target == Some(target_id.clone()))
        );
    }

    #[test]
    fn snapping_center_priority_resolves_equal_distance_edge_candidates() {
        let mut snapshot = Snapshot::default();
        let target_id = PanelId::from("center-target");
        let bounds = Box2 {
            x: 110.0,
            y: 100.0,
            w: 40.0,
            h: 40.0,
        };
        snapshot.panels.insert(
            target_id.clone(),
            Visual {
                bounds,
                clip: bounds,
                opacity: 1.0,
                z: 0,
            },
        );
        let index = SpatialIndex::build(&snapshot, 8, 3, 64.0).unwrap();
        let moving = Box2 {
            x: 100.0,
            y: 100.0,
            w: 40.0,
            h: 40.0,
        };
        let result = index
            .snap_bounds(
                moving,
                None,
                SnapSettings {
                    threshold_points: 12.0,
                    edge_priority: 10,
                    center_priority: 20,
                    ..SnapSettings::default()
                },
            )
            .unwrap();
        let x_guide = result
            .guides
            .iter()
            .find(|guide| guide.axis == SnapAxis::X)
            .unwrap();
        assert_eq!(x_guide.kind, SnapKind::Center);
        assert_eq!(x_guide.target, Some(target_id));
        assert_eq!(result.bounds.x, 110.0);
    }

    #[test]
    fn snapping_corner_axes_choose_nearest_distinct_targets_deterministically() {
        let mut snapshot = Snapshot::default();
        let x_target = PanelId::from("z-x-target");
        let x_bounds = Box2 {
            x: 146.0,
            y: 90.0,
            w: 40.0,
            h: 60.0,
        };
        let y_target = PanelId::from("a-y-target");
        let y_bounds = Box2 {
            x: 90.0,
            y: 146.0,
            w: 60.0,
            h: 40.0,
        };
        snapshot.panels.insert(
            x_target.clone(),
            Visual {
                bounds: x_bounds,
                clip: x_bounds,
                opacity: 1.0,
                z: 0,
            },
        );
        snapshot.panels.insert(
            y_target.clone(),
            Visual {
                bounds: y_bounds,
                clip: y_bounds,
                opacity: 1.0,
                z: 0,
            },
        );
        let index = SpatialIndex::build(&snapshot, 9, 4, 64.0).unwrap();
        let result = index
            .snap_bounds(
                Box2 {
                    x: 100.0,
                    y: 100.0,
                    w: 40.0,
                    h: 40.0,
                },
                None,
                SnapSettings {
                    threshold_points: 12.0,
                    ..SnapSettings::default()
                },
            )
            .unwrap();
        let x_guide = result
            .guides
            .iter()
            .find(|guide| guide.axis == SnapAxis::X)
            .unwrap();
        let y_guide = result
            .guides
            .iter()
            .find(|guide| guide.axis == SnapAxis::Y)
            .unwrap();
        assert_eq!(x_guide.target, Some(x_target));
        assert_eq!(y_guide.target, Some(y_target));
        assert_eq!((result.bounds.x, result.bounds.y), (106.0, 106.0));
    }
    #[test]
    fn snapping_equalizes_gap_between_nearest_siblings_with_stable_guides() {
        let mut snapshot = Snapshot::default();
        for (id, bounds) in [
            (
                "left",
                Box2 {
                    x: 20.0,
                    y: 0.0,
                    w: 80.0,
                    h: 100.0,
                },
            ),
            (
                "right",
                Box2 {
                    x: 300.0,
                    y: 0.0,
                    w: 80.0,
                    h: 100.0,
                },
            ),
        ] {
            snapshot.panels.insert(
                PanelId::from(id),
                Visual {
                    bounds,
                    clip: bounds,
                    opacity: 1.0,
                    z: 0,
                },
            );
        }
        let index = SpatialIndex::build(&snapshot, 17, 4, 64.0).unwrap();
        let moving = Box2 {
            x: 106.0,
            y: 20.0,
            w: 180.0,
            h: 40.0,
        };
        let result = index
            .snap_bounds_with_spacing(
                moving,
                None,
                SnapSettings {
                    threshold_points: 12.0,
                    edge_priority: 10,
                    ..SnapSettings::default()
                },
                Some(20),
            )
            .unwrap();

        assert_eq!(result.bounds.x, 110.0);
        let guide = result
            .guides
            .iter()
            .find(|guide| guide.axis == SnapAxis::X)
            .unwrap();
        assert_eq!(guide.kind, SnapKind::EqualSpacing);
        assert_eq!(guide.target, Some(PanelId::from("left")));
        assert!(result.collisions.is_empty());

        let disabled = index
            .snap_bounds(moving, None, SnapSettings::default())
            .unwrap();
        assert!(
            disabled
                .guides
                .iter()
                .all(|guide| guide.kind != SnapKind::EqualSpacing)
        );

        let moving_id = PanelId::from("left");
        let excluded = index
            .snap_bounds_with_spacing(moving, Some(&moving_id), SnapSettings::default(), Some(20))
            .unwrap();
        assert!(
            excluded
                .guides
                .iter()
                .all(|guide| guide.kind != SnapKind::EqualSpacing)
        );
    }

    #[test]
    fn snapping_equal_spacing_supports_vertical_neighbors() {
        let mut snapshot = Snapshot::default();
        for (id, bounds) in [
            (
                "top",
                Box2 {
                    x: 0.0,
                    y: 20.0,
                    w: 100.0,
                    h: 80.0,
                },
            ),
            (
                "bottom",
                Box2 {
                    x: 0.0,
                    y: 300.0,
                    w: 100.0,
                    h: 80.0,
                },
            ),
        ] {
            snapshot.panels.insert(
                PanelId::from(id),
                Visual {
                    bounds,
                    clip: bounds,
                    opacity: 1.0,
                    z: 0,
                },
            );
        }
        let index = SpatialIndex::build(&snapshot, 18, 4, 64.0).unwrap();
        let result = index
            .snap_bounds_with_spacing(
                Box2 {
                    x: 20.0,
                    y: 106.0,
                    w: 40.0,
                    h: 180.0,
                },
                None,
                SnapSettings {
                    threshold_points: 12.0,
                    ..SnapSettings::default()
                },
                Some(20),
            )
            .unwrap();

        assert_eq!(result.bounds.y, 110.0);
        assert!(
            result
                .guides
                .iter()
                .any(|guide| guide.axis == SnapAxis::Y && guide.kind == SnapKind::EqualSpacing)
        );
    }

    #[test]
    fn snapping_grid_priority_is_deterministic_and_invalid_scales_fail_closed() {
        let index = SpatialIndex::build(&snapshot(), 3, 2, 64.).unwrap();
        let moving = Box2 {
            x: 51.,
            y: 49.,
            w: 20.,
            h: 20.,
        };
        let settings = SnapSettings {
            threshold_points: 10.,
            grid_spacing: Some(50.),
            edge_priority: 1,
            center_priority: 1,
            grid_priority: 20,
            ..SnapSettings::default()
        };
        let result = index.snap_bounds(moving, None, settings).unwrap();
        assert_eq!(result.bounds.x, 50.);
        assert_eq!(result.bounds.y, 50.);
        assert!(
            result
                .guides
                .iter()
                .all(|guide| guide.kind == SnapKind::Grid)
        );
        assert!(
            index
                .snap_bounds(
                    moving,
                    None,
                    SnapSettings {
                        zoom: 0.,
                        ..settings
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn snapping_reports_overlap_and_can_fail_closed_by_collision_policy() {
        let mut snapshot = Snapshot::default();
        let target = PanelId::from("target");
        snapshot.panels.insert(
            target.clone(),
            Visual {
                bounds: Box2 {
                    x: 10.0,
                    y: 0.0,
                    w: 40.0,
                    h: 40.0,
                },
                clip: Box2 {
                    x: 10.0,
                    y: 0.0,
                    w: 40.0,
                    h: 40.0,
                },
                opacity: 1.0,
                z: 0,
            },
        );
        let index = SpatialIndex::build(&snapshot, 12, 2, 32.0).unwrap();
        let moving = Box2 {
            x: 8.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        };
        let reported = index
            .snap_bounds(moving, None, SnapSettings::default())
            .unwrap();
        assert_eq!(reported.bounds.x, 10.0);
        assert_eq!(reported.collisions, vec![target.clone()]);
        assert!(
            index
                .snap_bounds(
                    moving,
                    None,
                    SnapSettings {
                        collision_policy: SnapCollisionPolicy::Reject,
                        ..SnapSettings::default()
                    },
                )
                .is_err()
        );

        let separated = index
            .snap_bounds(Box2 { y: 80.0, ..moving }, None, SnapSettings::default())
            .unwrap();
        assert!(separated.collisions.is_empty());
    }

    #[test]
    fn spatial_queries_use_separate_panel_boundaries_and_collision_modes() {
        let mut projected = snapshot();
        let panel = PanelId::from("b");
        projected.panels.get_mut(&panel).unwrap().clip = Box2 {
            x: 100.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        let metadata = BTreeMap::from([(
            panel.clone(),
            PanelSpatialBounds {
                interaction: Box2 {
                    x: 140.0,
                    y: 10.0,
                    w: 20.0,
                    h: 20.0,
                },
                docking: Box2 {
                    x: 100.0,
                    y: 10.0,
                    w: 20.0,
                    h: 20.0,
                },
                collision: Box2 {
                    x: 205.0,
                    y: 10.0,
                    w: 30.0,
                    h: 30.0,
                },
                collision_policy: CollisionPolicy::Overlap,
            },
        )]);
        let index =
            SpatialIndex::build_with_panel_bounds(&projected, 4, 7, 32.0, &metadata).unwrap();

        assert_eq!(index.point_containing(145.0, 15.0), Some(panel.clone()));
        assert_eq!(index.point_containing(125.0, 15.0), None);
        assert_eq!(
            index
                .intersecting(Box2 {
                    x: 125.0,
                    y: 10.0,
                    w: 5.0,
                    h: 5.0,
                })
                .unwrap(),
            vec![panel.clone()]
        );
        assert_eq!(
            index
                .docking_targets(Box2 {
                    x: 105.0,
                    y: 15.0,
                    w: 5.0,
                    h: 5.0,
                })
                .unwrap(),
            vec![panel.clone()]
        );
        assert_eq!(
            index
                .collision_candidates(Box2 {
                    x: 210.0,
                    y: 15.0,
                    w: 5.0,
                    h: 5.0,
                })
                .unwrap(),
            vec![(panel.clone(), CollisionPolicy::Overlap)]
        );
        let allowed_overlap = index
            .snap_bounds(
                Box2 {
                    x: 210.0,
                    y: 15.0,
                    w: 5.0,
                    h: 5.0,
                },
                None,
                SnapSettings {
                    collision_policy: SnapCollisionPolicy::Reject,
                    ..SnapSettings::default()
                },
            )
            .unwrap();
        assert_eq!(allowed_overlap.collisions, vec![panel]);
    }

    #[test]
    fn spatial_boundary_metadata_rejects_unknown_panels_and_invalid_bounds() {
        let projected = snapshot();
        let unknown = BTreeMap::from([(
            PanelId::from("unknown"),
            PanelSpatialBounds::uniform(Box2 {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            }),
        )]);
        assert!(SpatialIndex::build_with_panel_bounds(&projected, 1, 1, 32.0, &unknown).is_err());

        let invalid = BTreeMap::from([(
            PanelId::from("b"),
            PanelSpatialBounds {
                interaction: Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: f64::NAN,
                    h: 10.0,
                },
                ..PanelSpatialBounds::uniform(Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 10.0,
                    h: 10.0,
                })
            },
        )]);
        assert!(SpatialIndex::build_with_panel_bounds(&projected, 1, 1, 32.0, &invalid).is_err());
    }

    #[test]
    fn each_collision_mode_has_explicit_diagnostic_and_rejection_semantics() {
        let policies = [
            (CollisionPolicy::Solid, true, true),
            (CollisionPolicy::Overlap, true, false),
            (CollisionPolicy::PassThrough, false, false),
            (CollisionPolicy::Magnetic, true, true),
            (CollisionPolicy::SnapOnly, true, false),
        ];
        for (policy, reported, rejected) in policies {
            let mut projected = snapshot();
            let panel = PanelId::from("b");
            projected.panels.get_mut(&panel).unwrap().clip = Box2 {
                x: 100.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            };
            let metadata = BTreeMap::from([(
                panel.clone(),
                PanelSpatialBounds {
                    collision: Box2 {
                        x: 195.0,
                        y: 10.0,
                        w: 30.0,
                        h: 30.0,
                    },
                    collision_policy: policy,
                    ..PanelSpatialBounds::uniform(Box2 {
                        x: 120.0,
                        y: 0.0,
                        w: 80.0,
                        h: 80.0,
                    })
                },
            )]);
            let index =
                SpatialIndex::build_with_panel_bounds(&projected, 1, 1, 32.0, &metadata).unwrap();
            let moving = Box2 {
                x: 210.0,
                y: 15.0,
                w: 5.0,
                h: 5.0,
            };
            assert_eq!(
                index
                    .collision_candidates(moving)
                    .unwrap()
                    .contains(&(panel.clone(), policy)),
                reported,
                "collision diagnostic behavior for {policy:?}"
            );
            assert_eq!(
                index
                    .snap_bounds(
                        moving,
                        None,
                        SnapSettings {
                            collision_policy: SnapCollisionPolicy::Reject,
                            ..SnapSettings::default()
                        },
                    )
                    .is_err(),
                rejected,
                "snap rejection behavior for {policy:?}"
            );
        }
    }
}
