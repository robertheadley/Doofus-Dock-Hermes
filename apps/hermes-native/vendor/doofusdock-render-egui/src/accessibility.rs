//! Compose independent scene-panel egui accessibility trees as AccessKit subtrees.
//!
//! Panel node IDs remain in their own tree namespaces. The host root gets one
//! graft node per visible panel, transformed by the same local-to-world matrix
//! used by the scene renderer. Native action requests can be routed back to
//! their owning panel context with [`SceneEguiAccessibilityUpdatePlan::route_action`].

use std::collections::BTreeMap;

use accesskit::{ActionRequest, Affine, Node, NodeId, Rect, Role, TreeId, TreeUpdate};
use doofusdock_core::{DockError, DockResult, ErrorCode, PanelId};
use doofusdock_presentation::{SceneFrame, ScenePoint};
use egui::{Event, FullOutput, PlatformOutput};
use uuid::Uuid;

use crate::input::ScenePanelFocusedInput;

/// Maximum number of visible panel trees in one composed accessibility update.
pub const MAX_SCENE_ACCESSIBILITY_PANELS: usize = 256;
/// Maximum combined AccessKit nodes accepted from the root and visible panels.
pub const MAX_SCENE_ACCESSIBILITY_NODES: usize = 65_536;
/// Maximum UTF-8 bytes accepted for each host-provided panel accessible name.
pub const MAX_SCENE_ACCESSIBILITY_LABEL_BYTES: usize = 256;

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const ACCESSIBILITY_TREE_NAMESPACE: Uuid =
    Uuid::from_u128(0x53f3_9a0b_9cb7_4b6a_a029_8e0c_13b2_97f2);

/// Revision-bound AccessKit updates for a host root and its visible panel subtrees.
#[derive(Debug)]
pub struct SceneEguiAccessibilityUpdatePlan {
    focused_panel: PanelId,
    root_update: TreeUpdate,
    panel_updates: BTreeMap<PanelId, TreeUpdate>,
    focus_update: TreeUpdate,
    panel_tree_ids: BTreeMap<PanelId, TreeId>,
    graft_panels: BTreeMap<NodeId, PanelId>,
}

/// Host action produced by routing a native accessibility request.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneEguiAccessibilityAction {
    /// A screen reader focused a visible panel graft; the host should update its
    /// focused panel and deliver subsequent keyboard input there.
    FocusPanel(PanelId),
    /// An action targets a node inside one panel's egui tree. Hosts switch to
    /// this panel before delivering the event when `focus_panel` is true.
    PanelInput {
        input: ScenePanelFocusedInput,
        focus_panel: bool,
    },
}

impl SceneEguiAccessibilityUpdatePlan {
    pub fn focused_panel(&self) -> &PanelId {
        &self.focused_panel
    }

    /// Root-tree update containing the host tree plus one transformed graft per panel.
    ///
    /// Apply this first. Its focus remains on the host root until the scene
    /// panel subtrees exist, since AccessKit rejects focus on an uninitialized
    /// graft.
    pub fn root_update(&self) -> &TreeUpdate {
        &self.root_update
    }

    /// Subtree updates keyed by the canonical panel identity.
    pub fn panel_updates(&self) -> &BTreeMap<PanelId, TreeUpdate> {
        &self.panel_updates
    }

    /// Root-tree focus change to apply after `root_update` and every panel
    /// subtree update have been installed.
    pub fn focus_update(&self) -> &TreeUpdate {
        &self.focus_update
    }

    /// Stable, panel-specific AccessKit tree IDs used by the graft nodes.
    pub fn panel_tree_ids(&self) -> &BTreeMap<PanelId, TreeId> {
        &self.panel_tree_ids
    }

    /// Route a native AccessKit action to its panel egui context.
    ///
    /// The returned event targets `TreeId::ROOT`, which is the namespace egui
    /// expects inside each independent context. The host should deliver this
    /// event to the returned panel and let its normal context processing decide
    /// whether the action is applicable.
    pub fn route_action(
        &self,
        request: &ActionRequest,
    ) -> DockResult<SceneEguiAccessibilityAction> {
        if request.target_tree == TreeId::ROOT {
            let panel = self.graft_panels.get(&request.target_node).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Root accessibility action targets no visible panel graft.",
                    format!("node_id={:?}", request.target_node),
                )
            })?;
            if request.action != accesskit::Action::Focus {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel graft supports focus actions only.",
                    format!("action={:?} panel={}", request.action, panel.0),
                ));
            }
            return Ok(SceneEguiAccessibilityAction::FocusPanel(panel.clone()));
        }
        let (panel, _) = self
            .panel_tree_ids
            .iter()
            .find(|(_, tree_id)| **tree_id == request.target_tree)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "AccessKit action targets no visible scene panel tree.",
                    format!("tree_id={:?}", request.target_tree),
                )
            })?;
        let mut forwarded = request.clone();
        forwarded.target_tree = TreeId::ROOT;
        Ok(SceneEguiAccessibilityAction::PanelInput {
            input: ScenePanelFocusedInput {
                panel: panel.clone(),
                event: Event::AccessKitActionRequest(forwarded),
            },
            focus_panel: request.action == accesskit::Action::Focus,
        })
    }
}

/// Builds a host root tree with transformed panel subtrees for one scene frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneEguiAccessibilityAdapter;

impl SceneEguiAccessibilityAdapter {
    /// Compose the root context's AccessKit output with all visible panel trees.
    ///
    /// `labels` must supply a non-empty accessible name for each visible panel.
    /// The host root update must include its root node; egui's root context emits
    /// a complete per-pass tree suitable for this operation. Panel tree node IDs
    /// are isolated with stable per-panel `TreeId`s rather than being rewritten.
    // The inputs intentionally expose the scene binding and accessibility
    // outputs independently so hosts can keep ownership and revision checks
    // explicit at the call site.
    #[allow(clippy::too_many_arguments)]
    pub fn compose(
        &self,
        frame: &SceneFrame,
        focused_panel: &PanelId,
        labels: &BTreeMap<PanelId, String>,
        host_output: &PlatformOutput,
        panel_outputs: &BTreeMap<PanelId, FullOutput>,
        revision: u64,
        generation: u64,
    ) -> DockResult<SceneEguiAccessibilityUpdatePlan> {
        validate_scene(frame, focused_panel, revision, generation)?;
        let mut visible_elements = BTreeMap::new();
        let mut visible_order = Vec::new();
        for element in frame.elements() {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                visible_elements.entry(element.panel.clone())
            {
                visible_order.push(element.panel.clone());
                entry.insert(element);
            }
        }
        if visible_order.len() > MAX_SCENE_ACCESSIBILITY_PANELS {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Visible scene exceeds the accessibility panel limit.",
                format!(
                    "visible={} limit={MAX_SCENE_ACCESSIBILITY_PANELS}",
                    visible_order.len()
                ),
            ));
        }

        let mut root_update = host_output.accesskit_update.clone().ok_or_else(|| {
            DockError::new(
                ErrorCode::UnsupportedOperation,
                "Host root context did not emit an AccessKit tree update.",
                "enable egui AccessKit output on the root context",
            )
        })?;
        if root_update.tree_id != TreeId::ROOT {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Host root AccessKit update uses a non-root tree ID.",
                format!("tree_id={:?}", root_update.tree_id),
            ));
        }
        let root_info = root_update.tree.as_ref().ok_or_else(|| {
            DockError::new(
                ErrorCode::UnsupportedOperation,
                "Host root AccessKit update does not include tree metadata.",
                "a full root-context update is required for graft composition",
            )
        })?;
        let root_id = root_info.root;
        let root_index = root_update
            .nodes
            .iter()
            .position(|(id, _)| *id == root_id)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Host root update omitted its root node.",
                    format!("root_id={root_id:?}"),
                )
            })?;

        let mut tree_owner = BTreeMap::new();
        let mut graft_owner = root_update
            .nodes
            .iter()
            .map(|(id, _)| (*id, None::<PanelId>))
            .collect::<BTreeMap<_, _>>();
        let mut panel_tree_ids = BTreeMap::new();
        let mut panel_updates = BTreeMap::new();
        let mut graft_nodes = Vec::with_capacity(visible_order.len());
        let mut total_nodes = root_update.nodes.len();

        for panel in &visible_order {
            let label = labels.get(panel).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "A visible panel has no host-provided accessible name.",
                    &panel.0,
                )
            })?;
            validate_label(panel, label)?;
            let element = visible_elements[panel];
            let output = panel_outputs.get(panel).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "A visible scene panel has no current egui output.",
                    &panel.0,
                )
            })?;
            let mut update = output
                .platform_output
                .accesskit_update
                .clone()
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "A visible panel context did not emit an AccessKit tree update.",
                        &panel.0,
                    )
                })?;
            if update.tree_id != TreeId::ROOT {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Panel AccessKit update must use its context-local root tree ID.",
                    format!("panel={} tree_id={:?}", panel.0, update.tree_id),
                ));
            }
            let panel_tree = update.tree.as_ref().ok_or_else(|| {
                DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel AccessKit update does not include tree metadata.",
                    &panel.0,
                )
            })?;
            if !update.nodes.iter().any(|(id, _)| *id == panel_tree.root) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel AccessKit update omitted its root node.",
                    format!("panel={} root_id={:?}", panel.0, panel_tree.root),
                ));
            }
            total_nodes = total_nodes.checked_add(update.nodes.len()).ok_or_else(|| {
                DockError::new(
                    ErrorCode::ResourceLimit,
                    "Combined scene accessibility node count overflowed.",
                    &panel.0,
                )
            })?;
            if total_nodes > MAX_SCENE_ACCESSIBILITY_NODES {
                return Err(DockError::new(
                    ErrorCode::ResourceLimit,
                    "Combined scene exceeds the accessibility node limit.",
                    format!("nodes={total_nodes} limit={MAX_SCENE_ACCESSIBILITY_NODES}"),
                ));
            }

            let tree_id = panel_tree_id(panel);
            if tree_id == TreeId::ROOT || tree_owner.insert(tree_id, panel.clone()).is_some() {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Panel accessibility tree IDs are not unique.",
                    &panel.0,
                ));
            }
            let graft_id = panel_graft_node_id(panel);
            if graft_owner.insert(graft_id, Some(panel.clone())).is_some() {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Panel accessibility graft node collides with the host tree.",
                    &panel.0,
                ));
            }

            let mut graft = Node::new(Role::Group);
            graft.set_label(label.clone());
            graft.set_bounds(Rect::new(
                element.local_bounds.x,
                element.local_bounds.y,
                element.local_bounds.x + element.local_bounds.width,
                element.local_bounds.y + element.local_bounds.height,
            ));
            graft.set_transform(scene_affine(element.world_transform));
            graft.set_tree_id(tree_id);
            graft.add_action(accesskit::Action::Focus);
            graft_nodes.push((graft_id, graft));

            update.tree_id = tree_id;
            panel_tree_ids.insert(panel.clone(), tree_id);
            panel_updates.insert(panel.clone(), update);
        }

        let root_node = &mut root_update.nodes[root_index].1;
        let mut children = root_node.children().to_vec();
        for (graft_id, _) in &graft_nodes {
            if children.contains(graft_id) {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Host root already contains a panel graft node ID.",
                    format!("graft_id={graft_id:?}"),
                ));
            }
            children.push(*graft_id);
        }
        root_node.set_children(children);
        root_update.nodes.extend(graft_nodes);
        let focused_graft = panel_graft_node_id(focused_panel);
        root_update.focus = root_id;
        let focus_update = TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            tree_id: TreeId::ROOT,
            focus: focused_graft,
        };
        let graft_panels = visible_order
            .into_iter()
            .map(|panel| (panel_graft_node_id(&panel), panel))
            .collect();

        Ok(SceneEguiAccessibilityUpdatePlan {
            focused_panel: focused_panel.clone(),
            root_update,
            panel_updates,
            focus_update,
            panel_tree_ids,
            graft_panels,
        })
    }
}

fn validate_scene(
    frame: &SceneFrame,
    focused_panel: &PanelId,
    revision: u64,
    generation: u64,
) -> DockResult<()> {
    if frame.revision() != revision || frame.generation() != generation {
        return Err(DockError::new(
            ErrorCode::Conflict,
            "Accessibility composition targets a stale scene frame.",
            format!(
                "frame={}:{} requested={revision}:{generation}",
                frame.revision(),
                frame.generation()
            ),
        ));
    }
    if !frame
        .elements()
        .iter()
        .any(|element| &element.panel == focused_panel)
    {
        return Err(DockError::new(
            ErrorCode::NotFound,
            "Host focus is not a visible scene panel.",
            &focused_panel.0,
        ));
    }
    Ok(())
}

fn validate_label(panel: &PanelId, label: &str) -> DockResult<()> {
    if label.trim().is_empty()
        || label.len() > MAX_SCENE_ACCESSIBILITY_LABEL_BYTES
        || label.chars().any(char::is_control)
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Panel accessible name is empty, too long, or contains control characters.",
            &panel.0,
        ));
    }
    Ok(())
}

fn fnv64(seed: u64, value: &[u8]) -> u64 {
    value.iter().fold(seed, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn panel_tree_id(panel: &PanelId) -> TreeId {
    let bytes = panel.0.as_bytes();
    TreeId(Uuid::new_v5(&ACCESSIBILITY_TREE_NAMESPACE, bytes))
}

fn panel_graft_node_id(panel: &PanelId) -> NodeId {
    NodeId(
        fnv64(
            0xcbf2_9ce4_8422_2325 ^ 0xa076_1d64_78bd_642f,
            panel.0.as_bytes(),
        ) | (1 << 63),
    )
}

fn scene_affine(transform: doofusdock_presentation::Affine2) -> Affine {
    let origin = transform.transform_point(ScenePoint::default());
    let x_axis = transform.transform_point(ScenePoint { x: 1.0, y: 0.0 });
    let y_axis = transform.transform_point(ScenePoint { x: 0.0, y: 1.0 });
    Affine::new([
        x_axis.x - origin.x,
        x_axis.y - origin.y,
        y_axis.x - origin.x,
        y_axis.y - origin.y,
        origin.x,
        origin.y,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{Action, ActionData, NodeId, TreeInfo};
    use accesskit_consumer::{Tree, TreeChangeHandler};
    use doofusdock_presentation::{Affine2, SceneNode, SceneRect};

    #[derive(Default)]
    struct NoopTreeChangeHandler;

    impl TreeChangeHandler for NoopTreeChangeHandler {
        fn node_added(&mut self, _node: &accesskit_consumer::NodeRef) {}

        fn node_updated(
            &mut self,
            _old_node: &accesskit_consumer::NodeRef,
            _new_node: &accesskit_consumer::NodeRef,
        ) {
        }

        fn focus_moved(
            &mut self,
            _old_node: Option<&accesskit_consumer::NodeRef>,
            _new_node: Option<&accesskit_consumer::NodeRef>,
        ) {
        }

        fn node_removed(&mut self, _node: &accesskit_consumer::NodeRef) {}
    }

    fn tree(root_id: NodeId, child_id: NodeId) -> TreeUpdate {
        let mut root = Node::new(Role::Group);
        root.set_children([child_id]);
        let mut child = Node::new(Role::Button);
        child.set_label("Run".to_owned());
        child.set_bounds(Rect::new(4.0, 6.0, 44.0, 26.0));
        TreeUpdate {
            nodes: vec![(root_id, root), (child_id, child)],
            tree: Some(TreeInfo::new(root_id)),
            tree_id: TreeId::ROOT,
            focus: child_id,
        }
    }

    fn two_panel_frame() -> SceneFrame {
        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 240.0,
            height: 80.0,
        });
        let mut first = SceneNode::panel(
            "left",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 120.0,
                height: 80.0,
            },
        );
        first.transform = Affine2::translate(10.0, 20.0).unwrap();
        let second = SceneNode::panel(
            "right",
            SceneRect {
                x: 120.0,
                y: 0.0,
                width: 120.0,
                height: 80.0,
            },
        );
        root.children = vec![first, second];
        SceneFrame::build(&root, 19, 6).unwrap()
    }

    fn outputs() -> (PlatformOutput, BTreeMap<PanelId, FullOutput>) {
        let host_root = NodeId(1);
        let mut host_node = Node::new(Role::Window);
        host_node.set_label("Application".to_owned());
        let host = PlatformOutput {
            accesskit_update: Some(TreeUpdate {
                nodes: vec![(host_root, host_node)],
                tree: Some(TreeInfo::new(host_root)),
                tree_id: TreeId::ROOT,
                focus: host_root,
            }),
            ..Default::default()
        };
        let mut left = FullOutput::default();
        left.platform_output.accesskit_update = Some(tree(NodeId(10), NodeId(11)));
        let mut right = FullOutput::default();
        // Deliberately reuse the same node IDs: the subtree tree IDs isolate them.
        right.platform_output.accesskit_update = Some(tree(NodeId(10), NodeId(11)));
        (
            host,
            BTreeMap::from([("left".into(), left), ("right".into(), right)]),
        )
    }

    fn labels() -> BTreeMap<PanelId, String> {
        BTreeMap::from([
            ("left".into(), "Editor".into()),
            ("right".into(), "Console".into()),
        ])
    }

    #[test]
    fn compose_grafts_transformed_panel_subtrees_and_routes_actions_back() {
        let frame = two_panel_frame();
        let (host, outputs) = outputs();
        let plan = SceneEguiAccessibilityAdapter
            .compose(&frame, &"right".into(), &labels(), &host, &outputs, 19, 6)
            .unwrap();

        assert_eq!(plan.root_update().tree_id, TreeId::ROOT);
        assert_eq!(plan.root_update().focus, NodeId(1));
        assert_eq!(
            plan.focus_update().focus,
            panel_graft_node_id(&"right".into())
        );
        assert_eq!(plan.panel_updates().len(), 2);
        let left_tree = plan.panel_tree_ids()[&"left".into()];
        let right_tree = plan.panel_tree_ids()[&"right".into()];
        assert_ne!(left_tree, right_tree);
        assert_eq!(plan.panel_updates()[&"left".into()].tree_id, left_tree);
        assert_eq!(plan.panel_updates()[&"right".into()].tree_id, right_tree);
        assert_eq!(plan.panel_updates()[&"left".into()].focus, NodeId(11));
        assert!(plan.root_update().nodes.iter().any(|(id, node)| {
            *id == panel_graft_node_id(&"left".into())
                && node.tree_id() == Some(left_tree)
                && node.label() == Some("Editor")
                && node.transform().is_some()
                && node.supports_action(Action::Focus)
        }));
        let left_element = frame
            .elements()
            .iter()
            .find(|el| el.panel == "left".into())
            .unwrap();
        let origin = left_element
            .world_transform
            .transform_point(ScenePoint::default());
        let transform = plan
            .root_update()
            .nodes
            .iter()
            .find(|(id, _)| *id == panel_graft_node_id(&"left".into()))
            .and_then(|(_, node)| node.transform())
            .unwrap();
        assert_eq!(
            *transform * accesskit::Point::new(0.0, 0.0),
            accesskit::Point::new(origin.x, origin.y)
        );

        let routed = plan
            .route_action(&ActionRequest {
                action: Action::Click,
                target_tree: right_tree,
                target_node: NodeId(11),
                data: Some(ActionData::Value("test".into())),
            })
            .unwrap();
        let SceneEguiAccessibilityAction::PanelInput {
            input: routed,
            focus_panel,
        } = routed
        else {
            panic!("panel subtree action must be routed to egui");
        };
        assert!(!focus_panel);
        assert_eq!(routed.panel, "right".into());
        let Event::AccessKitActionRequest(request) = routed.event else {
            panic!("routed action must become an egui AccessKitActionRequest");
        };
        assert_eq!(request.target_tree, TreeId::ROOT);
        assert_eq!(request.target_node, NodeId(11));

        let focus_inside = plan
            .route_action(&ActionRequest {
                action: Action::Focus,
                target_tree: right_tree,
                target_node: NodeId(11),
                data: None,
            })
            .unwrap();
        let SceneEguiAccessibilityAction::PanelInput { input, focus_panel } = focus_inside else {
            panic!("subtree focus must both select its panel and reach egui");
        };
        assert!(focus_panel);
        assert_eq!(input.panel, "right".into());

        let focus = plan
            .route_action(&ActionRequest {
                action: Action::Focus,
                target_tree: TreeId::ROOT,
                target_node: panel_graft_node_id(&"left".into()),
                data: None,
            })
            .unwrap();
        assert_eq!(
            focus,
            SceneEguiAccessibilityAction::FocusPanel("left".into())
        );
    }

    #[test]
    fn composed_updates_apply_as_one_accesskit_tree_with_isolated_panel_focus() {
        let frame = two_panel_frame();
        let (host, outputs) = outputs();
        let plan = SceneEguiAccessibilityAdapter
            .compose(&frame, &"right".into(), &labels(), &host, &outputs, 19, 6)
            .unwrap();

        // AccessKit requires the parent tree to publish graft nodes before the
        // first update for each subtree, and rejects focus on a graft until its
        // subtree exists. Apply the whole plan in the required order and let
        // the real consumer validate the connected tree and focus chain.
        let mut consumer = Tree::new(plan.root_update().clone(), true);
        let mut changes = NoopTreeChangeHandler;
        for update in plan.panel_updates().values() {
            consumer.update_and_process_changes(update.clone(), &mut changes);
        }
        consumer.update_and_process_changes(plan.focus_update().clone(), &mut changes);

        let children: Vec<_> = consumer.state().root().children().collect();
        assert_eq!(children.len(), 2);
        let left = children
            .iter()
            .find(|node| node.label().as_deref() == Some("Editor"))
            .expect("left graft is visible in the consumer tree");
        let right = children
            .iter()
            .find(|node| node.label().as_deref() == Some("Console"))
            .expect("right graft is visible in the consumer tree");
        assert_eq!(left.children().len(), 1);
        assert_eq!(right.children().len(), 1);

        let left_child = left.children().next().unwrap().children().next().unwrap();
        let right_child = right.children().next().unwrap().children().next().unwrap();
        assert_eq!(left_child.label().as_deref(), Some("Run"));
        assert_eq!(right_child.label().as_deref(), Some("Run"));
        assert_eq!(
            left_child.locate(),
            (NodeId(11), plan.panel_tree_ids()[&"left".into()])
        );
        assert_eq!(
            right_child.locate(),
            (NodeId(11), plan.panel_tree_ids()[&"right".into()])
        );

        let focus = consumer
            .state()
            .focus()
            .expect("host focus resolves into a panel");
        assert_eq!(
            focus.locate(),
            (NodeId(11), plan.panel_tree_ids()[&"right".into()])
        );
    }

    #[test]
    fn composer_rejects_stale_or_incomplete_accessibility_inputs() {
        let frame = two_panel_frame();
        let (host, outputs) = outputs();
        assert_eq!(
            SceneEguiAccessibilityAdapter
                .compose(&frame, &"right".into(), &labels(), &host, &outputs, 20, 6)
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        let incomplete = BTreeMap::from([("left".into(), outputs[&"left".into()].clone())]);
        let error = match SceneEguiAccessibilityAdapter.compose(
            &frame,
            &"right".into(),
            &labels(),
            &host,
            &incomplete,
            19,
            6,
        ) {
            Err(error) => error,
            Ok(_) => panic!("missing panel output must be rejected"),
        };
        assert_eq!(error.code, ErrorCode::NotFound);

        let nameless = BTreeMap::from([("right".into(), "Console".into())]);
        let error = match SceneEguiAccessibilityAdapter.compose(
            &frame,
            &"right".into(),
            &nameless,
            &host,
            &outputs,
            19,
            6,
        ) {
            Err(error) => error,
            Ok(_) => panic!("missing visible panel name must be rejected"),
        };
        assert_eq!(error.code, ErrorCode::NotFound);
    }
}
