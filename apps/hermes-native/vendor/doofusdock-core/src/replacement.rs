//! Validated, policy-checked whole-layout replacement used by morph acceptance.
use crate::*;
use std::collections::BTreeMap;
#[derive(Default)]
struct Placement {
    tab: bool,
    split: bool,
    edges: Vec<DockEdge>,
    floating: bool,
}
/// Replacement never changes registered content or docking permissions.
pub fn validate_layout_replacement(
    source: &LayoutDocument,
    target: &LayoutDocument,
) -> DockResult<()> {
    validate_layout(source)?;
    validate_layout(target)?;
    if source.workspace != target.workspace
        || source.id != target.id
        || source.panels != target.panels
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Replacement must preserve workspace, layout and panel descriptors.",
            "replace-layout",
        ));
    }
    let mut places = BTreeMap::new();
    fn visit(
        node: &Node,
        tab: bool,
        split: bool,
        edges: Vec<DockEdge>,
        floating: bool,
        out: &mut BTreeMap<PanelId, Placement>,
    ) {
        match &node.kind {
            NodeKind::Panel { panel } | NodeKind::Placeholder { panel, .. } => {
                out.insert(
                    panel.clone(),
                    Placement {
                        tab,
                        split,
                        edges,
                        floating,
                    },
                );
            }
            NodeKind::Tabs { items, .. } => {
                for child in items {
                    visit(
                        child,
                        tab || items.len() > 1,
                        split,
                        edges.clone(),
                        floating,
                        out,
                    );
                }
            }
            NodeKind::Split {
                orientation,
                children,
                ..
            } => {
                for (i, child) in children.iter().enumerate() {
                    let mut next = edges.clone();
                    next.push(match (orientation, i == 0) {
                        (Orientation::Horizontal, true) => DockEdge::Left,
                        (Orientation::Horizontal, false) => DockEdge::Right,
                        (Orientation::Vertical, true) => DockEdge::Top,
                        (Orientation::Vertical, false) => DockEdge::Bottom,
                    });
                    visit(child, tab, true, next, floating, out);
                }
            }
            NodeKind::DocumentWell { groups, .. } => {
                for child in groups {
                    visit(child, tab, split, edges.clone(), floating, out);
                }
            }
            NodeKind::Extension { children, .. } => {
                for child in children {
                    visit(child, tab, split, edges.clone(), floating, out);
                }
            }
        }
    }
    if let Some(root) = &target.root {
        visit(root, false, false, vec![], false, &mut places);
    }
    for window in &target.floating_windows {
        visit(&window.root, false, false, vec![], true, &mut places);
    }
    for p in source.panels.iter() {
        let denied = if let Some(place) = places.get(&p.id) {
            (place.tab && !p.docking.allow_tab)
                || (place.split
                    && (!p.docking.allow_split
                        || place
                            .edges
                            .iter()
                            .any(|e| !p.docking.allowed_edges.contains(e))))
                || (place.floating && !p.docking.allow_float)
        } else {
            false
        };
        let closed = target.closed_panels.contains(&p.id)
            && !source.closed_panels.contains(&p.id)
            && p.close_policy == ClosePolicy::Forbidden;
        let drawer = target.auto_hide_panels.iter().any(|d| {
            d.panel == p.id
                && (!p.docking.allow_auto_hide || !p.docking.allowed_edges.contains(&d.edge))
        });
        if denied || closed || drawer {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Panel policy forbids target layout placement.",
                p.id.0.clone(),
            ));
        }
    }
    Ok(())
}
