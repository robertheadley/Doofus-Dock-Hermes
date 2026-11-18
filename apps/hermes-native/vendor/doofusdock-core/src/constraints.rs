use crate::*;
use std::cmp::{max, min};
#[derive(Clone, Debug, PartialEq)]
pub struct SplitPreview {
    pub weights: Vec<f64>,
    pub clamped: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutProjection {
    pub view: LayoutDocument,
    pub collapsed_splits: Vec<NodeId>,
    pub open_drawer: Option<PanelId>,
    pub overlay: Option<PanelId>,
    pub full_screen: bool,
}
/// Host-supplied monitor geometry expressed in the same logical coordinate
/// space as `FloatingWindow::geometry`.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorInfo {
    pub id: String,
    pub work_area: Rect,
    pub scale: f64,
    pub primary: bool,
}

/// Scale and clamp a floating window onto an explicitly selected monitor.
/// The returned monitor metadata is suitable for the canonical recovery operation.
pub fn place_floating_window_on_monitor(
    window: &FloatingWindow,
    monitor: &MonitorInfo,
) -> DockResult<(Rect, MonitorInfo)> {
    if window.geometry.width <= 0
        || window.geometry.height <= 0
        || !window.scale.is_finite()
        || window.scale <= 0.0
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Floating window geometry or scale is invalid for monitor placement.",
            window.id.0.clone(),
        ));
    }
    if monitor.id.trim().is_empty()
        || !monitor.scale.is_finite()
        || monitor.scale <= 0.0
        || monitor.work_area.width <= 0
        || monitor.work_area.height <= 0
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Monitor identity, scale, or work area is invalid for window placement.",
            monitor.id.clone(),
        ));
    }
    let ratio = window.scale / monitor.scale;
    let scaled = Rect {
        x: window.geometry.x,
        y: window.geometry.y,
        width: (window.geometry.width as f64 * ratio)
            .round()
            .clamp(1.0, i32::MAX as f64) as i32,
        height: (window.geometry.height as f64 * ratio)
            .round()
            .clamp(1.0, i32::MAX as f64) as i32,
    };
    Ok((
        clamp_to_work_area(scaled, monitor.work_area),
        monitor.clone(),
    ))
}

fn monitor_for_geometry<'a>(
    geometry: Rect,
    preferred_monitor: &str,
    monitors: &'a [MonitorInfo],
) -> Option<&'a MonitorInfo> {
    let center_x = geometry.x as f64 + geometry.width.max(0) as f64 / 2.0;
    let center_y = geometry.y as f64 + geometry.height.max(0) as f64 / 2.0;
    if let Some(monitor) = monitors
        .iter()
        .filter(|monitor| {
            center_x >= monitor.work_area.x as f64
                && center_x < monitor.work_area.x.saturating_add(monitor.work_area.width) as f64
                && center_y >= monitor.work_area.y as f64
                && center_y < monitor.work_area.y.saturating_add(monitor.work_area.height) as f64
        })
        .max_by_key(|monitor| monitor.primary)
    {
        return Some(monitor);
    }

    let intersection = |monitor: &&MonitorInfo| {
        let left = geometry.x.max(monitor.work_area.x) as f64;
        let top = geometry.y.max(monitor.work_area.y) as f64;
        let right = (geometry.x.saturating_add(geometry.width))
            .min(monitor.work_area.x.saturating_add(monitor.work_area.width))
            as f64;
        let bottom = (geometry.y.saturating_add(geometry.height))
            .min(monitor.work_area.y.saturating_add(monitor.work_area.height))
            as f64;
        (right - left).max(0.0) * (bottom - top).max(0.0)
    };
    if let Some(monitor) = monitors
        .iter()
        .max_by(|left, right| intersection(left).total_cmp(&intersection(right)))
        && intersection(&monitor) > 0.0
    {
        return Some(monitor);
    }

    monitors
        .iter()
        .find(|monitor| monitor.id == preferred_monitor)
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
}

/// Keep a floating window reachable after display removal or DPI change.
/// Geometry is rescaled to preserve approximate physical size, then clamped
/// wholly inside the selected monitor's work area.
pub fn recover_floating_window_geometry(
    window: &FloatingWindow,
    monitors: &[MonitorInfo],
) -> DockResult<(Rect, MonitorInfo)> {
    if window.geometry.width <= 0
        || window.geometry.height <= 0
        || !window.scale.is_finite()
        || window.scale <= 0.0
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Floating window geometry or scale is invalid for monitor recovery.",
            window.id.0.clone(),
        ));
    }
    let monitor =
        monitor_for_geometry(window.geometry, &window.monitor_id, monitors).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "No available monitor work areas were supplied.",
                window.id.0.clone(),
            )
        })?;
    if monitor.id.trim().is_empty()
        || !monitor.scale.is_finite()
        || monitor.scale <= 0.0
        || monitor.work_area.width <= 0
        || monitor.work_area.height <= 0
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Monitor work area or scale is invalid.",
            monitor.id.clone(),
        ));
    }
    let ratio = window.scale / monitor.scale;
    let scaled = Rect {
        x: window.geometry.x,
        y: window.geometry.y,
        width: (window.geometry.width as f64 * ratio)
            .round()
            .clamp(1.0, i32::MAX as f64) as i32,
        height: (window.geometry.height as f64 * ratio)
            .round()
            .clamp(1.0, i32::MAX as f64) as i32,
    };
    Ok((
        clamp_to_work_area(scaled, monitor.work_area),
        monitor.clone(),
    ))
}

pub fn plan_floating_window_recovery(
    document: &LayoutDocument,
    monitors: &[MonitorInfo],
) -> DockResult<Vec<Operation>> {
    validate_layout(document)?;
    let mut operations = Vec::new();
    for window in &document.floating_windows {
        let (geometry, monitor) = recover_floating_window_geometry(window, monitors)?;
        if geometry != window.geometry
            || monitor.id != window.monitor_id
            || monitor.scale != window.scale
        {
            operations.push(Operation::RecoverFloatingWindow {
                window: window.id.clone(),
                geometry,
                monitor_id: monitor.id,
                scale: monitor.scale,
            });
        }
    }
    Ok(operations)
}
fn descriptor<'a>(doc: &'a LayoutDocument, id: &PanelId) -> Option<&'a PanelDescriptor> {
    doc.descriptor(id)
}
fn minimum(n: &Node, doc: &LayoutDocument) -> Size {
    match &n.kind {
        NodeKind::Panel { panel } => descriptor(doc, panel)
            .map(|p| p.minimum)
            .unwrap_or_default(),
        NodeKind::Tabs { items, .. } | NodeKind::DocumentWell { groups: items, .. } => items
            .iter()
            .map(|x| minimum(x, doc))
            .fold(Size::default(), |a, b| {
                Size::new(max(a.width, b.width), max(a.height, b.height))
            }),
        NodeKind::Split {
            orientation,
            children,
            ..
        } => children
            .iter()
            .map(|x| minimum(x, doc))
            .fold(Size::default(), |mut a, b| {
                if *orientation == Orientation::Horizontal {
                    a.width = a.width.saturating_add(b.width);
                    a.height = max(a.height, b.height)
                } else {
                    a.height = a.height.saturating_add(b.height);
                    a.width = max(a.width, b.width)
                }
                a
            }),
        NodeKind::Extension { children, .. } => children
            .iter()
            .map(|x| minimum(x, doc))
            .fold(Size::default(), |a, b| {
                Size::new(max(a.width, b.width), max(a.height, b.height))
            }),
        _ => Size::default(),
    }
}
fn max_intersection(a: i32, b: i32) -> i32 {
    if a == 0 {
        b
    } else if b == 0 {
        a
    } else {
        min(a, b)
    }
}
fn maximum(n: &Node, doc: &LayoutDocument) -> Size {
    match &n.kind {
        NodeKind::Panel { panel } => descriptor(doc, panel)
            .map(|p| p.maximum)
            .unwrap_or_default(),
        NodeKind::Tabs { items, .. } | NodeKind::DocumentWell { groups: items, .. } => items
            .iter()
            .map(|x| maximum(x, doc))
            .fold(Size::default(), |a, b| {
                Size::new(
                    max_intersection(a.width, b.width),
                    max_intersection(a.height, b.height),
                )
            }),
        NodeKind::Split {
            orientation,
            children,
            ..
        } => {
            let ms: Vec<_> = children.iter().map(|x| maximum(x, doc)).collect();
            if *orientation == Orientation::Horizontal {
                Size::new(
                    if ms.iter().any(|x| x.width == 0) {
                        0
                    } else {
                        ms.iter().map(|x| x.width).sum()
                    },
                    ms.iter().fold(0, |a, x| max_intersection(a, x.height)),
                )
            } else {
                Size::new(
                    ms.iter().fold(0, |a, x| max_intersection(a, x.width)),
                    if ms.iter().any(|x| x.height == 0) {
                        0
                    } else {
                        ms.iter().map(|x| x.height).sum()
                    },
                )
            }
        }
        _ => Size::default(),
    }
}
pub fn preview_split(
    doc: &LayoutDocument,
    id: &NodeId,
    divider: usize,
    requested: f64,
    available: Size,
) -> DockResult<SplitPreview> {
    validate_layout(doc)?;
    let n = all_find(doc, id).ok_or_else(|| {
        DockError::new(
            ErrorCode::NotFound,
            "Split node was not found.",
            id.0.clone(),
        )
    })?;
    let (orientation, children, weights) = if let NodeKind::Split {
        orientation,
        children,
        weights,
    } = &n.kind
    {
        (*orientation, children, weights)
    } else {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Preview target is not a split.",
            id.0.clone(),
        ));
    };
    if children.len() < 2 || divider + 1 >= children.len() {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Divider index is outside split.",
            divider.to_string(),
        ));
    }
    if !requested.is_finite() || available.width < 0 || available.height < 0 {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Preview geometry or weight is invalid.",
            id.0.clone(),
        ));
    }
    let axis = if orientation == Orientation::Horizontal {
        available.width
    } else {
        available.height
    };
    let a = minimum(&children[divider], doc);
    let b = minimum(&children[divider + 1], doc);
    let x = maximum(&children[divider], doc);
    let y = maximum(&children[divider + 1], doc);
    let (amin, bmin, amax, bmax) = if orientation == Orientation::Horizontal {
        (a.width, b.width, x.width, y.width)
    } else {
        (a.height, b.height, x.height, y.height)
    };
    let pair = weights[divider] + weights[divider + 1];
    let den = axis as f64 * pair;
    if axis <= 0
        || den <= 0.0
        || (amin + bmin) as f64 > den + den.max(1.0) * 1e-12
        || (amax != 0 && amin > amax)
        || (bmax != 0 && bmin > bmax)
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Available size cannot satisfy adjacent panel constraints.",
            id.0.clone(),
        ));
    }
    let lo = (amin as f64 / den).max(if bmax == 0 {
        0.0
    } else {
        1.0 - bmax as f64 / den
    });
    let hi = (1.0 - bmin as f64 / den).min(if amax == 0 { 1.0 } else { amax as f64 / den });
    if lo > hi + 1e-12 {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Adjacent panel constraints cannot fit.",
            id.0.clone(),
        ));
    }
    let hi = hi.max(lo);
    let v = requested.clamp(lo, hi);
    let mut out = weights.clone();
    let total = out[divider] + out[divider + 1];
    out[divider] = v * total;
    out[divider + 1] = (1.0 - v) * total;
    Ok(SplitPreview {
        weights: out,
        clamped: v != requested,
    })
}
fn all_find<'a>(d: &'a LayoutDocument, id: &NodeId) -> Option<&'a Node> {
    if let Some(root) = &d.root
        && let Some(n) = find_node(root, id)
    {
        return Some(n);
    }
    d.floating_windows
        .iter()
        .find_map(|w| find_node(&w.root, id))
}
fn find_node<'a>(n: &'a Node, id: &NodeId) -> Option<&'a Node> {
    crate::find_node(n, id)
}
pub fn clamp_to_work_area(mut r: Rect, a: Rect) -> Rect {
    r.width = r.width.clamp(0, a.width.max(0));
    r.height = r.height.clamp(0, a.height.max(0));
    let max_x =
        (a.x as i64 + (a.width - r.width).max(0) as i64).clamp(i32::MIN as i64, i32::MAX as i64);
    let max_y =
        (a.y as i64 + (a.height - r.height).max(0) as i64).clamp(i32::MIN as i64, i32::MAX as i64);
    r.x = (r.x as i64).clamp(a.x as i64, max_x) as i32;
    r.y = (r.y as i64).clamp(a.y as i64, max_y) as i32;
    r
}
pub fn project_narrow_viewport(doc: &LayoutDocument, size: Size) -> DockResult<LayoutProjection> {
    if size.width < 0 || size.height < 0 {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Viewport dimensions cannot be negative.",
            "viewport",
        ));
    }
    let mut out = doc.clone();
    let mut collapsed = Vec::new();
    fn proj(n: &Node, d: &LayoutDocument, s: Size, c: &mut Vec<NodeId>) -> NodePtr {
        match &n.kind {
            NodeKind::Split {
                orientation,
                weights,
                children,
            } => {
                let need = minimum(n, d);
                let cc: Vec<_> = children.iter().map(|x| proj(x, d, s, c)).collect();
                if need.width > s.width || need.height > s.height {
                    c.push(n.id.clone());
                    Node::tabs(n.id.clone(), cc, 0)
                } else {
                    Node::split(n.id.clone(), *orientation, cc, weights.clone())
                }
            }
            NodeKind::Tabs {
                active_index,
                items,
                minimized,
                ..
            } => Node::tabs_with_minimized(
                n.id.clone(),
                items.iter().map(|x| proj(x, d, s, c)).collect(),
                *active_index,
                *minimized,
            ),
            NodeKind::DocumentWell {
                active_group,
                groups,
            } => std::sync::Arc::new(Node {
                id: n.id.clone(),
                kind: NodeKind::DocumentWell {
                    active_group: *active_group,
                    groups: groups.iter().map(|x| proj(x, d, s, c)).collect(),
                },
            }),
            NodeKind::Extension {
                modality,
                payload,
                children,
            } => std::sync::Arc::new(Node {
                id: n.id.clone(),
                kind: NodeKind::Extension {
                    modality: modality.clone(),
                    payload: payload.clone(),
                    children: children.iter().map(|x| proj(x, d, s, c)).collect(),
                },
            }),
            _ => std::sync::Arc::new(n.clone()),
        }
    }
    if let Some(root) = &doc.root {
        out.root = Some(proj(root, doc, size, &mut collapsed))
    }
    for w in &mut out.floating_windows {
        w.root = proj(&w.root, doc, w.geometry.size(), &mut collapsed)
    }
    Ok(LayoutProjection {
        view: out,
        collapsed_splits: collapsed,
        open_drawer: None,
        overlay: None,
        full_screen: false,
    })
}

/// Canonical content minimum, excluding host-specific chrome metrics.
pub fn subtree_minimum_size(node: &Node, document: &LayoutDocument) -> Size {
    minimum(node, document)
}
