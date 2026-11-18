//! Deterministic, toolkit-neutral presentation. Frames never mutate canonical layouts.
mod animation;
pub use animation::*;
mod camera;
pub use camera::*;
mod scene;
mod spatial;
use doofusdock_core::*;
pub use scene::*;
use serde::{Deserialize, Serialize};
pub use spatial::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Box2 {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
impl Box2 {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.w, self.h]
            .iter()
            .all(|n| n.is_finite() && n.abs() <= 1_000_000.)
            && self.w >= 0.
            && self.h >= 0.
    }
    pub fn mix(self, b: Self, t: f64) -> Self {
        Self {
            x: lerp(self.x, b.x, t),
            y: lerp(self.y, b.y, t),
            w: lerp(self.w, b.w, t),
            h: lerp(self.h, b.h, t),
        }
    }
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    fn rect(self) -> Rect {
        Rect {
            x: self.x.round() as i32,
            y: self.y.round() as i32,
            width: self.w.round() as i32,
            height: self.h.round() as i32,
        }
    }
}
impl From<Rect> for Box2 {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x as f64,
            y: r.y as f64,
            w: r.width as f64,
            h: r.height as f64,
        }
    }
}
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Visual {
    pub bounds: Box2,
    pub clip: Box2,
    pub opacity: f64,
    pub z: i32,
}
/// Geometry and tab identity for a canonical minimized tab group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupRail {
    pub group: NodeId,
    pub bounds: Box2,
    pub panel_ids: Vec<PanelId>,
    pub active_index: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub panels: BTreeMap<PanelId, Visual>,
    pub nodes: BTreeMap<NodeId, Box2>,
    pub windows: BTreeMap<WindowId, Box2>,
    pub group_rails: BTreeMap<NodeId, GroupRail>,
}
/// Space reserved for a minimized tab group in a split projection.
pub const GROUP_RAIL_EXTENT: f64 = 112.0;
impl Snapshot {
    pub fn hit_test(&self, x: f64, y: f64) -> Option<PanelId> {
        self.panels
            .iter()
            .filter(|(_, v)| v.opacity >= 0.5 && v.bounds.contains(x, y) && v.clip.contains(x, y))
            .max_by_key(|(_, v)| v.z)
            .map(|(id, _)| id.clone())
    }
}
pub fn project(doc: &LayoutDocument, bounds: Box2) -> Result<Snapshot, String> {
    project_with_extensions(doc, bounds, &ExtensionRegistry::default())
}

/// Project one workspace-tag context without duplicating canonical panel or
/// application content ownership. Untagged panels are shared in every context.
pub fn project_with_workspace_tags(
    doc: &LayoutDocument,
    bounds: Box2,
    active_tags: &std::collections::BTreeSet<String>,
) -> Result<Snapshot, String> {
    let mut snapshot = project(doc, bounds)?;
    snapshot.panels.retain(|panel, _| {
        doc.descriptor(panel).is_some_and(|descriptor| {
            descriptor.workspace_memberships.is_empty()
                || !descriptor.workspace_memberships.is_disjoint(active_tags)
        })
    });
    Ok(snapshot)
}

/// Project with application-registered layout providers. Built-in layout
/// algorithms remain available; dynamic providers run through the lifecycle
/// managed `ExtensionRegistry`.
pub fn project_with_extensions(
    doc: &LayoutDocument,
    bounds: Box2,
    extensions: &ExtensionRegistry,
) -> Result<Snapshot, String> {
    validate_layout(doc).map_err(|e| e.to_string())?;
    if !bounds.valid() {
        return Err("Invalid projection bounds".into());
    }
    let mut result = Snapshot::default();
    let registry = ExtensionLayoutRegistry::with_builtins();
    fn walk(
        n: &Node,
        b: Box2,
        visible: bool,
        z: i32,
        doc: &LayoutDocument,
        registries: (&ExtensionLayoutRegistry, &ExtensionRegistry),
        out: &mut Snapshot,
    ) -> Result<(), String> {
        let (registry, extensions) = registries;
        out.nodes.insert(n.id.clone(), b);
        match &n.kind {
            NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. } => {
                out.panels.insert(
                    panel.clone(),
                    Visual {
                        bounds: b,
                        clip: b,
                        opacity: if visible { 1. } else { 0. },
                        z,
                    },
                );
            }
            NodeKind::Tabs {
                items,
                active_index,
                minimized,
            } => {
                if *minimized {
                    let rail_bounds = if b.w > GROUP_RAIL_EXTENT && b.h > GROUP_RAIL_EXTENT {
                        Box2 {
                            w: GROUP_RAIL_EXTENT,
                            ..b
                        }
                    } else {
                        b
                    };
                    out.nodes.insert(n.id.clone(), rail_bounds);
                    let mut panel_ids = Vec::new();
                    for child in items {
                        doofusdock_core::walk(child, &mut |leaf| {
                            if let NodeKind::Panel { panel } = &leaf.kind
                                && !panel_ids.contains(panel)
                            {
                                panel_ids.push(panel.clone());
                            }
                        });
                    }
                    out.group_rails.insert(
                        n.id.clone(),
                        GroupRail {
                            group: n.id.clone(),
                            bounds: rail_bounds,
                            panel_ids,
                            active_index: *active_index,
                        },
                    );
                    for child in items {
                        walk(child, rail_bounds, false, z, doc, registries, out)?;
                    }
                    return Ok(());
                }
                for (i, c) in items.iter().enumerate() {
                    walk(c, b, visible && i == *active_index, z, doc, registries, out)?;
                }
            }
            NodeKind::Split {
                orientation,
                weights,
                children,
            } => {
                let rail_count = children
                    .iter()
                    .filter(|child| {
                        matches!(
                            child.kind,
                            NodeKind::Tabs {
                                minimized: true,
                                ..
                            }
                        )
                    })
                    .count();
                let expanded_weight: f64 = children
                    .iter()
                    .zip(weights)
                    .filter(|(child, _)| {
                        !matches!(
                            child.kind,
                            NodeKind::Tabs {
                                minimized: true,
                                ..
                            }
                        )
                    })
                    .map(|(_, weight)| *weight)
                    .sum();
                let axis_extent = match orientation {
                    Orientation::Horizontal => b.w,
                    Orientation::Vertical => b.h,
                };
                let rail_extent = GROUP_RAIL_EXTENT.min(axis_extent / rail_count.max(1) as f64);
                let available_expanded = (axis_extent - rail_extent * rail_count as f64).max(0.0);
                let mut offset = 0.;
                for (i, c) in children.iter().enumerate() {
                    let mut child = b;
                    let minimized = matches!(
                        c.kind,
                        NodeKind::Tabs {
                            minimized: true,
                            ..
                        }
                    );
                    let extent = if minimized {
                        rail_extent
                    } else if expanded_weight > 0.0 {
                        available_expanded * weights[i] / expanded_weight
                    } else {
                        0.0
                    };
                    match orientation {
                        Orientation::Horizontal => {
                            child.x += offset;
                            child.w = extent;
                        }
                        Orientation::Vertical => {
                            child.y += offset;
                            child.h = extent;
                        }
                    }
                    offset += extent;
                    walk(c, child, visible, z, doc, registries, out)?;
                }
            }
            NodeKind::DocumentWell { groups, .. } => {
                for (i, c) in groups.iter().enumerate() {
                    let mut child = b;
                    child.x += b.w * i as f64 / groups.len() as f64;
                    child.w /= groups.len() as f64;
                    walk(c, child, visible, z, doc, registries, out)?;
                }
            }
            NodeKind::Extension {
                modality,
                payload,
                children,
            } => {
                let metrics: Vec<_> = children
                    .iter()
                    .map(|c| {
                        let mut min = Size::default();
                        doofusdock_core::walk(c, &mut |n| {
                            if let NodeKind::Panel { panel } = &n.kind
                                && let Some(p) = doc.descriptor(panel)
                            {
                                min.width = min.width.max(p.minimum.width);
                                min.height = min.height.max(p.minimum.height);
                            }
                        });
                        ExtensionChildMetrics {
                            minimum: min,
                            preferred: Size {
                                width: 240,
                                height: 160,
                            },
                        }
                    })
                    .collect();
                let child_ids: Vec<_> = children.iter().map(|child| child.id.clone()).collect();
                let request = ExtensionLayoutRequest {
                    modality: &modality.0,
                    payload,
                    bounds: b.rect(),
                    child_ids: &child_ids,
                    children: &metrics,
                };
                let placements = if extensions.has_layout_provider(&modality.0) {
                    extensions.arrange_layout(&request)
                } else {
                    registry.arrange(&request)
                }
                .map_err(|e| e.to_string())?;
                for p in placements {
                    walk(
                        &children[p.child_index],
                        p.bounds.into(),
                        visible,
                        z,
                        doc,
                        registries,
                        out,
                    )?;
                }
            }
        }
        Ok(())
    }
    if let Some(root) = &doc.root {
        walk(
            root,
            bounds,
            true,
            0,
            doc,
            (&registry, extensions),
            &mut result,
        )?;
    }
    for (index, window) in doc.floating_windows.iter().enumerate() {
        let b = window.geometry.into();
        result.windows.insert(window.id.clone(), b);
        walk(
            &window.root,
            b,
            true,
            1 + index as i32,
            doc,
            (&registry, extensions),
            &mut result,
        )?;
    }
    for panel in doc.panels.iter() {
        result.panels.entry(panel.id.clone()).or_insert(Visual {
            bounds,
            clip: bounds,
            opacity: 0.,
            z: 0,
        });
    }
    if let Some(max) = &doc.maximized_panel {
        for (id, v) in &mut result.panels {
            v.opacity = if id == max { 1. } else { 0. };
            if id == max {
                v.bounds = bounds;
                v.clip = bounds;
            }
        }
    }
    Ok(result)
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    Smooth,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    Spring { damping: f64, frequency: f64 },
}
impl Easing {
    pub fn sample(self, t: f64) -> f64 {
        let t = t.clamp(0., 1.);
        match self {
            Self::Linear => t,
            Self::Smooth => t * t * (3. - 2. * t),
            Self::EaseIn => t * t,
            Self::EaseOut => 1. - (1. - t) * (1. - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    4. * t * t * t
                } else {
                    1. - (-2. * t + 2.).powi(3) / 2.
                }
            }
            Self::CubicBezier { x1, y1, x2, y2 } => cubic_bezier(t, x1, y1, x2, y2),
            Self::Spring { damping, frequency } => {
                let damping = if damping.is_finite() {
                    damping.clamp(0.01, 32.)
                } else {
                    8.
                };
                let frequency = if frequency.is_finite() {
                    frequency.clamp(0.25, 12.)
                } else {
                    2.5
                };
                let raw = |t: f64| {
                    1. - (-damping * t).exp() * (std::f64::consts::TAU * frequency * t).cos()
                };
                let end = raw(1.);
                if end.abs() < f64::EPSILON {
                    t
                } else {
                    raw(t) / end
                }
            }
        }
    }

    pub fn valid(self) -> bool {
        match self {
            Self::CubicBezier { x1, y1, x2, y2 } => {
                [x1, y1, x2, y2]
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                    && x1 <= x2
            }
            Self::Spring { damping, frequency } => {
                damping.is_finite()
                    && (0.01..=32.0).contains(&damping)
                    && frequency.is_finite()
                    && (0.25..=12.0).contains(&frequency)
            }
            _ => true,
        }
    }
}

fn cubic_bezier(t: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    if ![x1, y1, x2, y2].iter().all(|v| v.is_finite()) {
        return t;
    }
    let coordinate = |u: f64, a: f64, b: f64| {
        let v = 1. - u;
        3. * v * v * u * a + 3. * v * u * u * b + u * u * u
    };
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..24 {
        let mid = (low + high) * 0.5;
        if coordinate(mid, x1, x2) < t {
            low = mid;
        } else {
            high = mid;
        }
    }
    coordinate((low + high) * 0.5, y1, y2)
}
#[derive(Clone, Debug)]
pub struct Morph {
    source_doc: LayoutDocument,
    target_doc: LayoutDocument,
    source: Snapshot,
    target: Snapshot,
    viewport: Box2,
    start: f64,
    duration: f64,
    easing: Easing,
    settled: bool,
    reduced_motion: bool,
}

/// Timing and provider policy for a layout morph. Provider output is capped by
/// both `maximum_duration_ms` and the presentation engine's 60 second bound.
#[derive(Clone, Debug)]
pub struct MorphTimingPolicy {
    pub transition: TransitionId,
    pub requested_duration_ms: u32,
    pub maximum_duration_ms: u32,
    pub reduced_motion: bool,
    /// Host capabilities negotiated before the timing provider is invoked.
    pub host_capabilities: Vec<String>,
    pub easing: Easing,
}

impl Morph {
    pub fn new(
        doc: &LayoutDocument,
        target: &LayoutDocument,
        viewport: Box2,
        start: f64,
        duration: f64,
        reduced_motion: bool,
    ) -> Result<Self, String> {
        Self::new_with_easing(
            doc,
            target,
            viewport,
            start,
            duration,
            reduced_motion,
            Easing::Smooth,
        )
    }
    pub fn new_with_easing(
        doc: &LayoutDocument,
        target: &LayoutDocument,
        viewport: Box2,
        start: f64,
        duration: f64,
        reduced_motion: bool,
        easing: Easing,
    ) -> Result<Self, String> {
        validate_layout_replacement(doc, target).map_err(|e| e.to_string())?;
        check_time(start, duration)?;
        if !easing.valid() {
            return Err("Morph easing parameters are invalid".into());
        }
        Ok(Self {
            source_doc: doc.clone(),
            target_doc: target.clone(),
            source: project(doc, viewport)?,
            target: project(target, viewport)?,
            viewport,
            start,
            duration: if reduced_motion { 0. } else { duration },
            easing,
            settled: false,
            reduced_motion,
        })
    }

    /// Resolve provider timing before constructing a morph. Provider errors
    /// fail before any canonical or transient state is changed.
    pub fn new_with_provider(
        doc: &LayoutDocument,
        target: &LayoutDocument,
        viewport: Box2,
        start: f64,
        policy: &MorphTimingPolicy,
        registry: &ExtensionRegistry,
    ) -> Result<Self, String> {
        check_time(start, 0.0)?;
        let duration = resolve_morph_duration(policy, registry)?;
        Self::new_with_easing(
            doc,
            target,
            viewport,
            start,
            duration,
            policy.reduced_motion,
            policy.easing,
        )
    }

    /// Retarget using the same provider contract as initial construction.
    /// Resolution and target projection complete before this morph is mutated.
    pub fn retarget_with_provider(
        &mut self,
        target: &LayoutDocument,
        time: f64,
        policy: &MorphTimingPolicy,
        registry: &ExtensionRegistry,
    ) -> Result<(), String> {
        check_time(time, 0.0)?;
        let duration = resolve_morph_duration(policy, registry)?;
        self.retarget_with_policy(target, time, duration, policy.easing, policy.reduced_motion)
    }
    pub fn progress(&self, time: f64) -> Result<f64, String> {
        if !time.is_finite() {
            return Err("Non-finite presentation time".into());
        }
        Ok(if self.duration == 0. {
            if time < self.start { 0. } else { 1. }
        } else {
            ((time - self.start) / self.duration).clamp(0., 1.)
        })
    }
    pub fn sample(&self, time: f64) -> Result<Snapshot, String> {
        let t = self.easing.sample(self.progress(time)?);
        if t == 0. {
            return Ok(self.source.clone());
        }
        if t == 1. {
            return Ok(self.target.clone());
        }
        let mut frame = Snapshot::default();
        let ids: BTreeSet<_> = self
            .source
            .panels
            .keys()
            .chain(self.target.panels.keys())
            .cloned()
            .collect();
        for id in ids {
            let a = self.source.panels.get(&id);
            let b = self.target.panels.get(&id);
            let fallback = Visual {
                bounds: Box2::default(),
                clip: Box2::default(),
                opacity: 0.,
                z: 0,
            };
            let a = a.unwrap_or(b.unwrap_or(&fallback));
            let b = b.unwrap_or(a);
            frame.panels.insert(
                id,
                Visual {
                    bounds: a.bounds.mix(b.bounds, t),
                    clip: a.clip.mix(b.clip, t),
                    opacity: lerp(a.opacity, b.opacity, t),
                    z: if t < 0.5 { a.z } else { b.z },
                },
            );
        }
        for id in self.source.nodes.keys().chain(self.target.nodes.keys()) {
            let a = self
                .source
                .nodes
                .get(id)
                .or_else(|| self.target.nodes.get(id))
                .unwrap();
            let b = self.target.nodes.get(id).unwrap_or(a);
            frame.nodes.insert(id.clone(), a.mix(*b, t));
        }
        for id in self.source.windows.keys().chain(self.target.windows.keys()) {
            let a = self
                .source
                .windows
                .get(id)
                .or_else(|| self.target.windows.get(id))
                .unwrap();
            let b = self.target.windows.get(id).unwrap_or(a);
            frame.windows.insert(id.clone(), a.mix(*b, t));
        }
        Ok(frame)
    }

    /// Sample the existing structural morph into a transform-aware scene frame
    /// and compose any transient per-panel decoration over that sample. The
    /// frame stays bound to the original canonical revision so stale animation
    /// output cannot authorize input after the layout changes.
    pub fn sample_scene(
        &self,
        time: f64,
        generation: u64,
        decorative_transforms: &BTreeMap<PanelId, Affine2>,
    ) -> DockResult<SceneFrame> {
        let snapshot = self.sample(time).map_err(|error| {
            DockError::new(
                ErrorCode::InvalidLayout,
                "Structural morph scene sampling failed.",
                error,
            )
        })?;
        SceneFrame::from_snapshot(
            &snapshot,
            self.viewport,
            self.source_doc.revision,
            generation,
        )?
        .with_panel_transforms(
            decorative_transforms,
            self.source_doc.revision,
            generation,
        )
    }

    pub fn retarget(
        &mut self,
        target: &LayoutDocument,
        time: f64,
        duration: f64,
    ) -> Result<(), String> {
        self.retarget_with_policy(target, time, duration, self.easing, self.reduced_motion)
    }

    fn retarget_with_policy(
        &mut self,
        target: &LayoutDocument,
        time: f64,
        duration: f64,
        easing: Easing,
        reduced_motion: bool,
    ) -> Result<(), String> {
        if self.settled {
            return Err("Morph already settled".into());
        }
        check_time(time, duration)?;
        if !easing.valid() {
            return Err("Morph easing parameters are invalid".into());
        }
        validate_layout_replacement(&self.source_doc, target).map_err(|e| e.to_string())?;
        let next = project(target, self.viewport)?;
        let source = self.sample(time)?;
        self.source = source;
        self.target = next;
        self.target_doc = target.clone();
        self.start = time;
        self.duration = if reduced_motion { 0.0 } else { duration };
        self.easing = easing;
        self.reduced_motion = reduced_motion;
        Ok(())
    }
    pub fn reverse(&mut self, time: f64, duration: f64) -> Result<(), String> {
        self.retarget(&self.source_doc.clone(), time, duration)
    }
    pub fn reverse_with_provider(
        &mut self,
        time: f64,
        policy: &MorphTimingPolicy,
        registry: &ExtensionRegistry,
    ) -> Result<(), String> {
        let source = self.source_doc.clone();
        self.retarget_with_provider(&source, time, policy, registry)
    }
    /// Refuses stale canonical state, so a user action cannot be overwritten by an old animation.
    pub fn finish(
        &mut self,
        time: f64,
        doc: &mut LayoutDocument,
        history: &mut UndoStack,
    ) -> Result<bool, String> {
        if self.settled || self.progress(time)? < 1. {
            return Ok(false);
        }
        if *doc != self.source_doc {
            return Err("Canonical layout changed during morph; cancel or replan".into());
        }
        DockingService::apply_layout_document(doc, history, doc.revision, &self.target_doc)
            .map_err(|e| e.to_string())?;
        self.settled = true;
        Ok(true)
    }
    pub fn cancel(&mut self) {
        self.settled = true;
    }
    pub fn is_settled(&self) -> bool {
        self.settled
    }
    pub fn target_document(&self) -> &LayoutDocument {
        &self.target_doc
    }
    pub fn source_document(&self) -> &LayoutDocument {
        &self.source_doc
    }
    pub fn easing(&self) -> Easing {
        self.easing
    }
}

fn resolve_morph_duration(
    policy: &MorphTimingPolicy,
    registry: &ExtensionRegistry,
) -> Result<f64, String> {
    if !policy.easing.valid() {
        return Err("Morph timing policy has invalid easing parameters".into());
    }
    let requested = policy.requested_duration_ms.min(60_000);
    let maximum = policy.maximum_duration_ms.min(60_000);
    let request = TransitionRequest {
        id: policy.transition.clone(),
        duration_ms: requested,
        reduced_motion: policy.reduced_motion,
        host_capabilities: policy.host_capabilities.clone(),
    };
    registry
        .transition_duration(&policy.transition, &request, maximum)
        .map(|duration| duration as f64 / 1000.0)
        .map_err(|error| error.to_string())
}

fn check_time(start: f64, duration: f64) -> Result<(), String> {
    if !start.is_finite() || !duration.is_finite() || !(0.0..=60.0).contains(&duration) {
        Err("Morph time must be finite and duration within 0..60 seconds".into())
    } else {
        Ok(())
    }
}

pub mod effects;
pub mod timeline;
