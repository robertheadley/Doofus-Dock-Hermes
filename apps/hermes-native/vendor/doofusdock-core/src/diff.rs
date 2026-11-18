use crate::{DockEdge, LayoutDocument, Node, NodeId, NodeKind, PanelId, Rect, WindowId};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffNodeKind {
    Panel,
    Split,
    Tabs,
    DocumentWell,
    Placeholder,
    Extension,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DiffPatch {
    SplitOrientationChanged {
        split: NodeId,
        orientation: crate::Orientation,
    },
    TabPinChanged {
        panel: PanelId,
        pinned: bool,
    },
    PanelWorkspaceMembershipChanged {
        panel: PanelId,
        tags: std::collections::BTreeSet<String>,
    },
    NodeCreated {
        id: NodeId,
        node: crate::NodePtr,
    },
    NodeRemoved {
        id: NodeId,
    },
    RootChanged {
        old_root: Option<NodeId>,
        new_root: Option<NodeId>,
    },
    PanelReparented {
        panel: PanelId,
        from_parent: NodeId,
        to_parent: NodeId,
        insertion_index: usize,
        from_window: Option<WindowId>,
        to_window: Option<WindowId>,
    },
    ContainerKindChanged {
        id: NodeId,
        kind: DiffNodeKind,
    },
    ContainerOrderChanged {
        parent: NodeId,
        ordered_children: Vec<NodeId>,
    },
    ActiveTabChanged {
        tabs: NodeId,
        active_index: usize,
    },
    GroupMinimizedChanged {
        group: NodeId,
        minimized: bool,
    },
    SplitRatioUpdated {
        split: NodeId,
        weights: Vec<f64>,
    },
    ExtensionPayloadUpdated {
        container: NodeId,
        modality: crate::ModalityId,
        payload: String,
    },
    WindowCreated {
        window: WindowId,
    },
    WindowDestroyed {
        window: WindowId,
    },
    WindowGeometryUpdated {
        window: WindowId,
        geometry: Rect,
    },
    WindowZOrderChanged {
        window: WindowId,
        z_order: i32,
    },
    WindowMonitorChanged {
        window: WindowId,
        monitor: String,
        scale: f64,
    },
    WindowPictureInPictureChanged {
        window: WindowId,
        enabled: bool,
    },
    ScratchpadChanged {
        window: WindowId,
        config: Option<crate::ScratchpadConfig>,
    },
    PanelVisibilityChanged {
        panel: PanelId,
        visible: bool,
    },
    DrawerChanged {
        panel: PanelId,
        auto_hidden: bool,
        edge: DockEdge,
    },
    MaximizedPanelChanged {
        panel: Option<PanelId>,
    },
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutDiff {
    pub patches: Vec<DiffPatch>,
}
impl LayoutDiff {
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TransactionResult {
    pub document: LayoutDocument,
    pub diff: LayoutDiff,
    pub operation_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct PanelLocation {
    parent: NodeId,
    index: usize,
    window: Option<WindowId>,
}
fn kind(k: &NodeKind) -> DiffNodeKind {
    match k {
        NodeKind::Panel { .. } => DiffNodeKind::Panel,
        NodeKind::Split { .. } => DiffNodeKind::Split,
        NodeKind::Tabs { .. } => DiffNodeKind::Tabs,
        NodeKind::DocumentWell { .. } => DiffNodeKind::DocumentWell,
        NodeKind::Placeholder { .. } => DiffNodeKind::Placeholder,
        NodeKind::Extension { .. } => DiffNodeKind::Extension,
    }
}
fn children(k: &NodeKind) -> Vec<NodeId> {
    match k {
        NodeKind::Tabs { items, .. } => items.iter().map(|n| n.id.clone()).collect(),
        NodeKind::DocumentWell { groups, .. } => groups.iter().map(|n| n.id.clone()).collect(),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter().map(|n| n.id.clone()).collect()
        }
        _ => vec![],
    }
}
fn collect<'a>(
    n: &'a Node,
    parent: Option<(&NodeId, usize)>,
    window: Option<&WindowId>,
    nodes: &mut BTreeMap<NodeId, &'a Node>,
    panels: &mut BTreeMap<PanelId, PanelLocation>,
) {
    nodes.insert(n.id.clone(), n);
    if let NodeKind::Panel { panel } = &n.kind {
        let (parent, index) = parent
            .map(|(id, i)| (id.clone(), i))
            .unwrap_or_else(|| (n.id.clone(), 0));
        panels.insert(
            panel.clone(),
            PanelLocation {
                parent,
                index,
                window: window.cloned(),
            },
        );
    }
    match &n.kind {
        NodeKind::Tabs { items, .. } => {
            for (i, child) in items.iter().enumerate() {
                collect(child, Some((&n.id, i)), window, nodes, panels);
            }
        }
        NodeKind::DocumentWell { groups, .. } => {
            for (i, group) in groups.iter().enumerate() {
                collect(group, Some((&n.id, i)), window, nodes, panels);
            }
        }
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            for (i, child) in children.iter().enumerate() {
                collect(child, Some((&n.id, i)), window, nodes, panels);
            }
        }
        _ => (),
    }
}
fn collect_doc(
    doc: &LayoutDocument,
) -> (BTreeMap<NodeId, &Node>, BTreeMap<PanelId, PanelLocation>) {
    let mut nodes = BTreeMap::new();
    let mut panels = BTreeMap::new();
    if let Some(root) = &doc.root {
        collect(root, None, None, &mut nodes, &mut panels);
    }
    for window in &doc.floating_windows {
        collect(
            &window.root,
            None,
            Some(&window.id),
            &mut nodes,
            &mut panels,
        );
    }
    (nodes, panels)
}
pub fn diff_layouts(a: &LayoutDocument, b: &LayoutDocument) -> LayoutDiff {
    let (old_nodes, old_panels) = collect_doc(a);
    let (new_nodes, new_panels) = collect_doc(b);
    let mut d = LayoutDiff::default();
    for old_window in &a.floating_windows {
        if let Some(new_window) = b
            .floating_windows
            .iter()
            .find(|window| window.id == old_window.id)
            && old_window.picture_in_picture != new_window.picture_in_picture
        {
            d.patches.push(DiffPatch::WindowPictureInPictureChanged {
                window: old_window.id.clone(),
                enabled: new_window.picture_in_picture,
            });
        }
    }
    for panel in a.pinned_panels.symmetric_difference(&b.pinned_panels) {
        d.patches.push(DiffPatch::TabPinChanged {
            panel: panel.clone(),
            pinned: b.pinned_panels.contains(panel),
        });
    }
    for before in a.panels.iter() {
        if let Some(after) = b.panels.iter().find(|panel| panel.id == before.id)
            && before.workspace_memberships != after.workspace_memberships
        {
            d.patches.push(DiffPatch::PanelWorkspaceMembershipChanged {
                panel: before.id.clone(),
                tags: after.workspace_memberships.clone(),
            });
        }
    }
    for id in old_nodes.keys() {
        if !new_nodes.contains_key(id) {
            d.patches.push(DiffPatch::NodeRemoved { id: id.clone() });
        }
    }
    for (id, node) in &new_nodes {
        if !old_nodes.contains_key(id) {
            d.patches.push(DiffPatch::NodeCreated {
                id: id.clone(),
                node: std::sync::Arc::new((*node).clone()),
            });
        }
    }
    for (id, before) in &old_nodes {
        if let Some(after) = new_nodes.get(id)
            && let (NodeKind::Tabs { minimized: old, .. }, NodeKind::Tabs { minimized: new, .. }) =
                (&before.kind, &after.kind)
            && old != new
        {
            d.patches.push(DiffPatch::GroupMinimizedChanged {
                group: id.clone(),
                minimized: *new,
            });
        }
    }
    let old_root = a.root.as_ref().map(|n| n.id.clone());
    let new_root = b.root.as_ref().map(|n| n.id.clone());
    if old_root != new_root {
        d.patches
            .push(DiffPatch::RootChanged { old_root, new_root });
    }
    for (panel, from) in &old_panels {
        if let Some(to) = new_panels.get(panel)
            && (from.parent != to.parent || from.index != to.index || from.window != to.window)
        {
            d.patches.push(DiffPatch::PanelReparented {
                panel: panel.clone(),
                from_parent: from.parent.clone(),
                to_parent: to.parent.clone(),
                insertion_index: to.index,
                from_window: from.window.clone(),
                to_window: to.window.clone(),
            });
        }
    }
    for (panel, to) in &new_panels {
        if !old_panels.contains_key(panel) {
            d.patches.push(DiffPatch::PanelReparented {
                panel: panel.clone(),
                from_parent: NodeId::default(),
                to_parent: to.parent.clone(),
                insertion_index: to.index,
                from_window: None,
                to_window: to.window.clone(),
            });
        }
    }
    for (id, node) in &new_nodes {
        let Some(previous) = old_nodes.get(id) else {
            continue;
        };
        if kind(&previous.kind) != kind(&node.kind) {
            d.patches.push(DiffPatch::ContainerKindChanged {
                id: node.id.clone(),
                kind: kind(&node.kind),
            });
        }
        let order = children(&node.kind);
        if children(&previous.kind) != order && !order.is_empty() {
            d.patches.push(DiffPatch::ContainerOrderChanged {
                parent: node.id.clone(),
                ordered_children: order,
            });
        }
        if let (
            NodeKind::Split {
                orientation: old, ..
            },
            NodeKind::Split {
                orientation: new, ..
            },
        ) = (&previous.kind, &node.kind)
            && old != new
        {
            d.patches.push(DiffPatch::SplitOrientationChanged {
                split: node.id.clone(),
                orientation: *new,
            });
        }
        match (&previous.kind, &node.kind) {
            (
                NodeKind::Tabs {
                    active_index: old, ..
                },
                NodeKind::Tabs {
                    active_index: new, ..
                },
            ) if old != new => d.patches.push(DiffPatch::ActiveTabChanged {
                tabs: node.id.clone(),
                active_index: *new,
            }),
            (
                NodeKind::DocumentWell {
                    active_group: old, ..
                },
                NodeKind::DocumentWell {
                    active_group: new, ..
                },
            ) if old != new => d.patches.push(DiffPatch::ActiveTabChanged {
                tabs: node.id.clone(),
                active_index: *new,
            }),
            (NodeKind::Split { weights: old, .. }, NodeKind::Split { weights: new, .. })
                if old != new =>
            {
                d.patches.push(DiffPatch::SplitRatioUpdated {
                    split: node.id.clone(),
                    weights: new.clone(),
                })
            }
            (
                NodeKind::Extension {
                    modality: old_modality,
                    payload: old_payload,
                    ..
                },
                NodeKind::Extension {
                    modality: new_modality,
                    payload: new_payload,
                    ..
                },
            ) if old_modality != new_modality || old_payload != new_payload => {
                d.patches.push(DiffPatch::ExtensionPayloadUpdated {
                    container: node.id.clone(),
                    modality: new_modality.clone(),
                    payload: new_payload.clone(),
                })
            }
            _ => (),
        }
    }
    let old_windows: BTreeMap<_, _> = a
        .floating_windows
        .iter()
        .map(|w| (w.id.clone(), w))
        .collect();
    let new_windows: BTreeMap<_, _> = b
        .floating_windows
        .iter()
        .map(|w| (w.id.clone(), w))
        .collect();
    for id in old_windows.keys() {
        if !new_windows.contains_key(id) {
            d.patches
                .push(DiffPatch::WindowDestroyed { window: id.clone() });
        }
    }
    for (id, window) in &new_windows {
        match old_windows.get(id) {
            None => d
                .patches
                .push(DiffPatch::WindowCreated { window: id.clone() }),
            Some(old) => {
                if old.geometry != window.geometry {
                    d.patches.push(DiffPatch::WindowGeometryUpdated {
                        window: id.clone(),
                        geometry: window.geometry,
                    });
                }
                if old.z_order_hint != window.z_order_hint {
                    d.patches.push(DiffPatch::WindowZOrderChanged {
                        window: id.clone(),
                        z_order: window.z_order_hint,
                    });
                }
                if old.monitor_id != window.monitor_id || old.scale != window.scale {
                    d.patches.push(DiffPatch::WindowMonitorChanged {
                        window: id.clone(),
                        monitor: window.monitor_id.clone(),
                        scale: window.scale,
                    });
                }
                if old.scratchpad != window.scratchpad {
                    d.patches.push(DiffPatch::ScratchpadChanged {
                        window: id.clone(),
                        config: window.scratchpad.clone(),
                    });
                }
            }
        }
    }
    for panel in &a.closed_panels {
        if !b.closed_panels.contains(panel) {
            d.patches.push(DiffPatch::PanelVisibilityChanged {
                panel: panel.clone(),
                visible: true,
            });
        }
    }
    for panel in &b.closed_panels {
        if !a.closed_panels.contains(panel) {
            d.patches.push(DiffPatch::PanelVisibilityChanged {
                panel: panel.clone(),
                visible: false,
            });
        }
    }
    for drawer in &a.auto_hide_panels {
        if !b.auto_hide_panels.iter().any(|x| x.panel == drawer.panel) {
            d.patches.push(DiffPatch::DrawerChanged {
                panel: drawer.panel.clone(),
                auto_hidden: false,
                edge: drawer.edge,
            });
        }
    }
    for drawer in &b.auto_hide_panels {
        let old = a.auto_hide_panels.iter().find(|x| x.panel == drawer.panel);
        if old.is_none_or(|x| x.edge != drawer.edge) {
            d.patches.push(DiffPatch::DrawerChanged {
                panel: drawer.panel.clone(),
                auto_hidden: true,
                edge: drawer.edge,
            });
        }
    }
    if a.maximized_panel != b.maximized_panel {
        d.patches.push(DiffPatch::MaximizedPanelChanged {
            panel: b.maximized_panel.clone(),
        });
    }
    d
}
