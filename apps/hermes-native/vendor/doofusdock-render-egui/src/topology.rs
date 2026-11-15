//! Theme-resolved chrome derived from canonical container topology.
use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Ui};
use std::collections::BTreeSet;

const MAX_TOPOLOGY_CHILDREN: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopologyRole {
    Split,
    Tabs,
    DocumentWell,
    Extension,
}

#[derive(Clone, Copy, Debug)]
pub struct TopologyPalette {
    pub border: Color32,
    pub accent: Color32,
    pub focus: Color32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub high_contrast: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedTopologyChrome {
    pub role: TopologyRole,
    pub outline: Stroke,
    pub corner_radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleEdges {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleCorners {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_right: bool,
    pub bottom_left: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChildTopology {
    pub id: String,
    pub rect: Rect,
    pub edges: VisibleEdges,
    pub corners: VisibleCorners,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SharedBoundary {
    pub first: String,
    pub second: String,
    pub first_side: BoundarySide,
    pub second_side: BoundarySide,
    pub start: Pos2,
    pub end: Pos2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoundarySide {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExteriorBoundary {
    pub owner: String,
    pub side: BoundarySide,
    pub start: Pos2,
    pub end: Pos2,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedSiblingTopology {
    pub children: Vec<ChildTopology>,
    pub exterior_boundaries: Vec<ExteriorBoundary>,
    pub shared_boundaries: Vec<SharedBoundary>,
}

impl VisibleEdges {
    fn all() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }
}

/// Resolve an outline only for multi-child canonical groups. No bitmap skin or
/// renderer resource is required, so the same result works in vector fallback.
pub fn resolve_container(
    role: TopologyRole,
    children: usize,
    palette: TopologyPalette,
) -> Option<ResolvedTopologyChrome> {
    if children < 2
        || !palette.corner_radius.is_finite()
        || !palette.border_width.is_finite()
        || palette.corner_radius < 0.0
        || palette.border_width <= 0.0
    {
        return None;
    }
    let color = if palette.high_contrast {
        palette.focus
    } else {
        match role {
            TopologyRole::Split | TopologyRole::Extension => palette.border,
            TopologyRole::Tabs => palette.accent,
            TopologyRole::DocumentWell => palette.focus,
        }
    };
    Some(ResolvedTopologyChrome {
        role,
        outline: Stroke::new(
            if palette.high_contrast {
                palette.border_width.max(2.0)
            } else {
                palette.border_width
            },
            color,
        ),
        corner_radius: match role {
            TopologyRole::Split => palette.corner_radius * 0.65,
            TopologyRole::Tabs => palette.corner_radius,
            TopologyRole::DocumentWell => palette.corner_radius * 1.25,
            TopologyRole::Extension => palette.corner_radius * 0.85,
        },
    })
}

pub fn paint_container_outline(ui: &Ui, rect: Rect, chrome: ResolvedTopologyChrome) {
    ui.painter().rect_stroke(
        rect,
        chrome.corner_radius,
        chrome.outline,
        StrokeKind::Inside,
    );
}

/// Resolve shared sibling seams and exposed edges from projected child bounds.
/// Input rectangles are presentation geometry; output is sorted by stable ID.
/// Touching edges within `tolerance` are represented by one shared boundary.
pub fn resolve_sibling_topology(
    children: &[(String, Rect)],
    tolerance: f32,
) -> Option<ResolvedSiblingTopology> {
    if children.len() > MAX_TOPOLOGY_CHILDREN
        || !tolerance.is_finite()
        || !(0.0..=64.0).contains(&tolerance)
    {
        return None;
    }
    let mut sorted = children.to_vec();
    let mut ids = BTreeSet::new();
    for (id, rect) in &sorted {
        if id.is_empty()
            || id.len() > 256
            || id.chars().any(char::is_control)
            || !ids.insert(id.as_str())
            || !rect.min.x.is_finite()
            || !rect.min.y.is_finite()
            || !rect.max.x.is_finite()
            || !rect.max.y.is_finite()
            || rect.width() <= 0.0
            || rect.height() <= 0.0
        {
            return None;
        }
    }
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut topology = ResolvedSiblingTopology {
        children: sorted
            .iter()
            .map(|(id, rect)| ChildTopology {
                id: id.clone(),
                rect: *rect,
                edges: VisibleEdges::all(),
                corners: VisibleCorners {
                    top_left: false,
                    top_right: false,
                    bottom_right: false,
                    bottom_left: false,
                },
            })
            .collect(),
        exterior_boundaries: Vec::new(),
        shared_boundaries: Vec::new(),
    };

    for left_index in 0..topology.children.len() {
        for right_index in (left_index + 1)..topology.children.len() {
            let left = topology.children[left_index].rect;
            let right = topology.children[right_index].rect;
            let y_min = left.min.y.max(right.min.y);
            let y_max = left.max.y.min(right.max.y);
            let x_min = left.min.x.max(right.min.x);
            let x_max = left.max.x.min(right.max.x);
            if y_max > y_min {
                if (left.max.x - right.min.x).abs() <= tolerance {
                    let x = (left.max.x + right.min.x) * 0.5;
                    topology.shared_boundaries.push(SharedBoundary {
                        first: topology.children[left_index].id.clone(),
                        second: topology.children[right_index].id.clone(),
                        first_side: BoundarySide::Right,
                        second_side: BoundarySide::Left,
                        start: Pos2::new(x, y_min),
                        end: Pos2::new(x, y_max),
                    });
                } else if (right.max.x - left.min.x).abs() <= tolerance {
                    let x = (right.max.x + left.min.x) * 0.5;
                    topology.shared_boundaries.push(SharedBoundary {
                        first: topology.children[left_index].id.clone(),
                        second: topology.children[right_index].id.clone(),
                        first_side: BoundarySide::Left,
                        second_side: BoundarySide::Right,
                        start: Pos2::new(x, y_min),
                        end: Pos2::new(x, y_max),
                    });
                }
            }
            if x_max > x_min {
                if (left.max.y - right.min.y).abs() <= tolerance {
                    let y = (left.max.y + right.min.y) * 0.5;
                    topology.shared_boundaries.push(SharedBoundary {
                        first: topology.children[left_index].id.clone(),
                        second: topology.children[right_index].id.clone(),
                        first_side: BoundarySide::Bottom,
                        second_side: BoundarySide::Top,
                        start: Pos2::new(x_min, y),
                        end: Pos2::new(x_max, y),
                    });
                } else if (right.max.y - left.min.y).abs() <= tolerance {
                    let y = (right.max.y + left.min.y) * 0.5;
                    topology.shared_boundaries.push(SharedBoundary {
                        first: topology.children[left_index].id.clone(),
                        second: topology.children[right_index].id.clone(),
                        first_side: BoundarySide::Top,
                        second_side: BoundarySide::Bottom,
                        start: Pos2::new(x_min, y),
                        end: Pos2::new(x_max, y),
                    });
                }
            }
        }
    }
    topology.exterior_boundaries =
        resolve_exterior_boundaries(&topology.children, &topology.shared_boundaries);
    for child in &mut topology.children {
        let segments = topology
            .exterior_boundaries
            .iter()
            .filter(|segment| segment.owner == child.id)
            .collect::<Vec<_>>();
        child.edges = VisibleEdges {
            top: segments
                .iter()
                .any(|segment| segment.side == BoundarySide::Top),
            right: segments
                .iter()
                .any(|segment| segment.side == BoundarySide::Right),
            bottom: segments
                .iter()
                .any(|segment| segment.side == BoundarySide::Bottom),
            left: segments
                .iter()
                .any(|segment| segment.side == BoundarySide::Left),
        };
        child.corners = VisibleCorners {
            top_left: corner_is_exposed(&segments, BoundarySide::Top, child.rect.left_top())
                && corner_is_exposed(&segments, BoundarySide::Left, child.rect.left_top()),
            top_right: corner_is_exposed(&segments, BoundarySide::Top, child.rect.right_top())
                && corner_is_exposed(&segments, BoundarySide::Right, child.rect.right_top()),
            bottom_right: corner_is_exposed(
                &segments,
                BoundarySide::Bottom,
                child.rect.right_bottom(),
            ) && corner_is_exposed(
                &segments,
                BoundarySide::Right,
                child.rect.right_bottom(),
            ),
            bottom_left: corner_is_exposed(
                &segments,
                BoundarySide::Bottom,
                child.rect.left_bottom(),
            ) && corner_is_exposed(
                &segments,
                BoundarySide::Left,
                child.rect.left_bottom(),
            ),
        };
    }
    topology.shared_boundaries.sort_by(|a, b| {
        a.first
            .cmp(&b.first)
            .then_with(|| a.second.cmp(&b.second))
            .then_with(|| a.start.y.total_cmp(&b.start.y))
            .then_with(|| a.start.x.total_cmp(&b.start.x))
    });
    Some(topology)
}

/// Paint only the resolved exterior edges and one stroke per shared seam.
pub fn paint_sibling_topology(
    ui: &Ui,
    topology: &ResolvedSiblingTopology,
    chrome: ResolvedTopologyChrome,
) {
    let painter = ui.painter();
    for boundary in &topology.exterior_boundaries {
        painter.line_segment([boundary.start, boundary.end], chrome.outline);
    }
    for boundary in &topology.shared_boundaries {
        painter.line_segment([boundary.start, boundary.end], chrome.outline);
    }
}

fn resolve_exterior_boundaries(
    children: &[ChildTopology],
    shared: &[SharedBoundary],
) -> Vec<ExteriorBoundary> {
    let mut exterior = Vec::new();
    for child in children {
        for side in [
            BoundarySide::Top,
            BoundarySide::Right,
            BoundarySide::Bottom,
            BoundarySide::Left,
        ] {
            let (start, end, fixed, vertical) = match side {
                BoundarySide::Top => (child.rect.min.x, child.rect.max.x, child.rect.min.y, false),
                BoundarySide::Right => (child.rect.min.y, child.rect.max.y, child.rect.max.x, true),
                BoundarySide::Bottom => {
                    (child.rect.min.x, child.rect.max.x, child.rect.max.y, false)
                }
                BoundarySide::Left => (child.rect.min.y, child.rect.max.y, child.rect.min.x, true),
            };
            let mut hidden = shared
                .iter()
                .filter_map(|boundary| {
                    let boundary_side = if boundary.first == child.id {
                        boundary.first_side
                    } else if boundary.second == child.id {
                        boundary.second_side
                    } else {
                        return None;
                    };
                    (boundary_side == side).then(|| {
                        if vertical {
                            (
                                boundary.start.y.min(boundary.end.y),
                                boundary.start.y.max(boundary.end.y),
                            )
                        } else {
                            (
                                boundary.start.x.min(boundary.end.x),
                                boundary.start.x.max(boundary.end.x),
                            )
                        }
                    })
                })
                .collect::<Vec<_>>();
            hidden.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
            let mut cursor = start;
            for (hidden_start, hidden_end) in hidden {
                let hidden_start = hidden_start.clamp(start, end);
                let hidden_end = hidden_end.clamp(start, end);
                if hidden_start > cursor {
                    exterior.push(edge_segment(
                        &child.id,
                        side,
                        cursor,
                        hidden_start,
                        fixed,
                        vertical,
                    ));
                }
                cursor = cursor.max(hidden_end);
            }
            if cursor < end {
                exterior.push(edge_segment(&child.id, side, cursor, end, fixed, vertical));
            }
        }
    }
    exterior.sort_by(|a, b| {
        a.owner
            .cmp(&b.owner)
            .then_with(|| a.side.cmp(&b.side))
            .then_with(|| a.start.y.total_cmp(&b.start.y))
            .then_with(|| a.start.x.total_cmp(&b.start.x))
    });
    exterior
}

fn edge_segment(
    owner: &str,
    side: BoundarySide,
    start: f32,
    end: f32,
    fixed: f32,
    vertical: bool,
) -> ExteriorBoundary {
    let (start, end) = if vertical {
        (Pos2::new(fixed, start), Pos2::new(fixed, end))
    } else {
        (Pos2::new(start, fixed), Pos2::new(end, fixed))
    };
    ExteriorBoundary {
        owner: owner.to_owned(),
        side,
        start,
        end,
    }
}

fn corner_is_exposed(segments: &[&ExteriorBoundary], side: BoundarySide, corner: Pos2) -> bool {
    segments.iter().any(|segment| {
        segment.side == side
            && ((segment.start.x - corner.x).abs() <= 0.001
                && (segment.start.y - corner.y).abs() <= 0.001
                || (segment.end.x - corner.x).abs() <= 0.001
                    && (segment.end.y - corner.y).abs() <= 0.001)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn palette(high_contrast: bool) -> TopologyPalette {
        TopologyPalette {
            border: Color32::from_rgb(30, 40, 50),
            accent: Color32::from_rgb(60, 70, 80),
            focus: Color32::from_rgb(240, 250, 255),
            corner_radius: 8.0,
            border_width: 1.0,
            high_contrast,
        }
    }

    #[test]
    fn topology_styles_are_deterministic_and_follow_group_kind() {
        let first = resolve_container(TopologyRole::Split, 2, palette(false)).unwrap();
        assert_eq!(
            first,
            resolve_container(TopologyRole::Split, 2, palette(false)).unwrap()
        );
        assert_eq!(first.role, TopologyRole::Split);
        assert_eq!(first.outline.color, palette(false).border);
        let tab_style = resolve_container(TopologyRole::Tabs, 2, palette(false)).unwrap();
        assert_eq!(tab_style.role, TopologyRole::Tabs);
        assert_eq!(tab_style.outline.color, palette(false).accent);
        assert_eq!(tab_style.corner_radius, 8.0);
    }

    #[test]
    fn topology_styles_fall_back_to_high_contrast_vector_and_skip_leaves() {
        let style = resolve_container(TopologyRole::Tabs, 2, palette(true)).unwrap();
        assert_eq!(style.outline.color, palette(true).focus);
        assert_eq!(style.outline.width, 2.0);
        assert!(resolve_container(TopologyRole::Split, 1, palette(true)).is_none());
        assert!(resolve_container(TopologyRole::Tabs, 1, palette(true)).is_none());
    }

    #[test]
    fn sibling_resolver_emits_one_stable_seam_and_exposed_corner_masks() {
        let first = (
            "left".to_owned(),
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 100.0)),
        );
        let second = (
            "right".to_owned(),
            Rect::from_min_max(Pos2::new(54.0, 0.0), Pos2::new(104.0, 100.0)),
        );
        let forward = resolve_sibling_topology(&[first.clone(), second.clone()], 4.0).unwrap();
        let reverse = resolve_sibling_topology(&[second, first], 4.0).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.children.len(), 2);
        assert_eq!(forward.shared_boundaries.len(), 1);
        let seam = &forward.shared_boundaries[0];
        assert_eq!(seam.start, Pos2::new(52.0, 0.0));
        assert_eq!(seam.end, Pos2::new(52.0, 100.0));
        let left = forward
            .children
            .iter()
            .find(|child| child.id == "left")
            .unwrap();
        assert!(!left.edges.right);
        assert!(left.corners.top_left && left.corners.bottom_left);
        assert!(!left.corners.top_right && !left.corners.bottom_right);
        assert_eq!(forward.exterior_boundaries.len(), 6);
    }

    #[test]
    fn sibling_resolver_preserves_unshared_segments_at_t_junctions() {
        let children = vec![
            (
                "left".to_owned(),
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 100.0)),
            ),
            (
                "right-top".to_owned(),
                Rect::from_min_max(Pos2::new(54.0, 0.0), Pos2::new(104.0, 40.0)),
            ),
            (
                "right-bottom".to_owned(),
                Rect::from_min_max(Pos2::new(54.0, 44.0), Pos2::new(104.0, 100.0)),
            ),
        ];
        let resolved = resolve_sibling_topology(&children, 4.0).unwrap();
        assert_eq!(resolved.shared_boundaries.len(), 3);
        let left_right_edges = resolved
            .exterior_boundaries
            .iter()
            .filter(|edge| edge.owner == "left" && edge.side == BoundarySide::Right)
            .collect::<Vec<_>>();
        assert_eq!(left_right_edges.len(), 1);
        assert_eq!(left_right_edges[0].start.y, 40.0);
        assert_eq!(left_right_edges[0].end.y, 44.0);
        assert!(resolved.shared_boundaries.iter().any(|edge| {
            edge.first_side == BoundarySide::Bottom || edge.second_side == BoundarySide::Bottom
        }));

        let partial = resolve_sibling_topology(
            &[
                (
                    "a".to_owned(),
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 100.0)),
                ),
                (
                    "b".to_owned(),
                    Rect::from_min_max(Pos2::new(54.0, 20.0), Pos2::new(104.0, 80.0)),
                ),
            ],
            4.0,
        )
        .unwrap();
        let exposed = partial
            .exterior_boundaries
            .iter()
            .filter(|edge| edge.owner == "a" && edge.side == BoundarySide::Right)
            .collect::<Vec<_>>();
        assert_eq!(exposed.len(), 2);
        assert_eq!(exposed[0].start.y, 0.0);
        assert_eq!(exposed[0].end.y, 20.0);
        assert_eq!(exposed[1].start.y, 80.0);
        assert_eq!(exposed[1].end.y, 100.0);
    }

    #[test]
    fn sibling_resolver_rejects_ambiguous_or_unbounded_inputs() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(10.0, 10.0));
        assert!(resolve_sibling_topology(&[("x".into(), rect), ("x".into(), rect)], 0.0).is_none());
        assert!(resolve_sibling_topology(&[("x".into(), rect)], f32::INFINITY).is_none());
        assert!(resolve_sibling_topology(&[("x".into(), Rect::NOTHING)], 0.0).is_none());
        let too_many = (0..=MAX_TOPOLOGY_CHILDREN)
            .map(|index| (format!("panel-{index}"), rect))
            .collect::<Vec<_>>();
        assert!(resolve_sibling_topology(&too_many, 0.0).is_none());
    }
}
