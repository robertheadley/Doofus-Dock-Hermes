use doofusdock_core::*;
use std::collections::BTreeSet;
use std::sync::Arc;

fn fixture() -> LayoutDocument {
    let mut doc = LayoutDocument::new("contract", "workspace");
    for id in ["a", "b", "c", "d"] {
        Arc::make_mut(&mut doc.panels).push(PanelDescriptor::new(id, id.to_uppercase()));
    }
    let tabs = Node::tabs(
        "tabs-ab",
        vec![Node::panel("node-a", "a"), Node::panel("node-b", "b")],
        0,
    );
    let documents = Arc::new(Node {
        id: "documents".into(),
        kind: NodeKind::DocumentWell {
            active_group: 0,
            groups: vec![
                Node::tabs("document-a", vec![Node::panel("node-c", "c")], 0),
                Node::tabs("document-b", vec![Node::panel("node-d", "d")], 0),
            ],
        },
    });
    doc.root = Some(Node::split(
        "workspace-root",
        Orientation::Horizontal,
        vec![tabs, documents],
        vec![0.5, 0.5],
    ));
    doc
}

fn assert_operation_undo_redo(document: &LayoutDocument, operation: Operation) {
    let mut trial = document.clone();
    let mut history = UndoStack::new(4);
    let before = trial.clone();
    let result = history
        .execute(&mut trial, operation)
        .expect("operation should be accepted by transaction history");
    validate_layout(&result.document).expect("transaction result remains a valid layout");
    if result.diff.is_empty() {
        assert_eq!(
            history.undo_size(),
            0,
            "no-op operations do not create history"
        );
        return;
    }

    history.undo(&mut trial).expect("operation should undo");
    let mut expected_before = before;
    expected_before.revision = trial.revision;
    assert_eq!(trial, expected_before, "undo restores canonical state");

    history.redo(&mut trial).expect("operation should redo");
    let mut expected_after = result.document;
    expected_after.revision = trial.revision;
    assert_eq!(trial, expected_after, "redo reapplies canonical state");
}

#[test]
fn reorder_panel_appends_at_the_child_count_insertion_boundary() {
    let doc = fixture();
    let result = apply_operation(
        &doc,
        Operation::ReorderPanel {
            panel: "a".into(),
            container: "tabs-ab".into(),
            index: 2,
        },
    )
    .unwrap();
    let tabs = find_node(
        result.document.root.as_deref().unwrap(),
        &NodeId::from("tabs-ab"),
    )
    .expect("tab container remains in the layout");
    let NodeKind::Tabs {
        items,
        active_index,
        ..
    } = &tabs.kind
    else {
        panic!("reorder target remains a tab container");
    };
    assert_eq!(
        items
            .iter()
            .map(|item| match &item.kind {
                NodeKind::Panel { panel } => panel.clone(),
                _ => panic!("tab items remain panel leaves"),
            })
            .collect::<Vec<_>>(),
        vec![PanelId::from("b"), PanelId::from("a")]
    );
    assert_eq!(*active_index, 1);
    assert_eq!(result.document.revision, doc.revision + 1);

    let invalid = apply_operation(
        &doc,
        Operation::ReorderPanel {
            panel: "a".into(),
            container: "tabs-ab".into(),
            index: 3,
        },
    );
    assert!(invalid.is_err());
}

#[test]
fn all_public_layout_operations_commit_valid_documents() {
    let doc = fixture();
    let operations = vec![
        Operation::TabPanel {
            panel: "c".into(),
            target_tabs: "tabs-ab".into(),
            index: 2,
        },
        Operation::CenterPanel {
            panel: "c".into(),
            target_panel: "a".into(),
        },
        Operation::CenterPanelGroup {
            panels: vec!["c".into(), "d".into()],
            target_panel: "a".into(),
        },
        Operation::SwapPanels {
            first: "a".into(),
            second: "b".into(),
        },
        Operation::SplitPanel {
            panel: "c".into(),
            target: "node-a".into(),
            orientation: Orientation::Vertical,
            before: false,
            ratio: 0.3,
        },
        Operation::FloatPanel {
            panel: "c".into(),
            window: "float-c".into(),
            geometry: Rect {
                x: -30,
                y: 50,
                width: 480,
                height: 360,
            },
            monitor: "display-1".into(),
            scale: 1.25,
        },
        Operation::ReorderPanel {
            panel: "b".into(),
            container: "tabs-ab".into(),
            index: 0,
        },
        Operation::ClosePanel { panel: "c".into() },
        Operation::ResizeSplit {
            split: "workspace-root".into(),
            weights: vec![0.6, 0.4],
        },
        Operation::SetGroupMinimized {
            group: "tabs-ab".into(),
            minimized: true,
        },
        Operation::AutoHide {
            panel: "c".into(),
            edge: DockEdge::Right,
        },
        Operation::Maximize { panel: "c".into() },
        Operation::RestoreMaximized,
        Operation::SelectTab {
            tabs: "tabs-ab".into(),
            index: 1,
        },
        Operation::ActivateDocumentGroup {
            well: "documents".into(),
            index: 1,
        },
    ];
    for op in operations {
        let encoded = serde_json::to_value(&op).expect("operation request should serialize");
        assert_eq!(encoded["operation"], op.id());
        let decoded: Operation =
            serde_json::from_value(encoded).expect("operation request should deserialize");
        assert_eq!(decoded, op, "{} request should round trip", op.id());
        let result = apply_operation(&doc, op.clone()).expect("operation should succeed");
        validate_layout(&result.document).expect("operation result must remain valid");
        let expected_revision = if result.diff.is_empty() {
            doc.revision
        } else {
            doc.revision + 1
        };
        assert_eq!(result.document.revision, expected_revision);
        assert_operation_undo_redo(&doc, op);
    }
    let mut root_panel = doc.clone();
    root_panel.root = None;
    root_panel.auto_hide_panels.push(AutoHidePanel {
        panel: "c".into(),
        edge: DockEdge::Left,
    });
    let result = apply_operation(&root_panel, Operation::RootPanel { panel: "c".into() }).unwrap();
    assert!(result.document.root.is_some());
    assert!(result.document.auto_hide_panels.is_empty());
    assert_operation_undo_redo(&root_panel, Operation::RootPanel { panel: "c".into() });

    let closed = apply_operation(&doc, Operation::ClosePanel { panel: "c".into() }).unwrap();
    let reopened = apply_operation(
        &closed.document,
        Operation::ReopenPanel {
            panel: "c".into(),
            target_tabs: "tabs-ab".into(),
            index: 2,
        },
    )
    .unwrap();
    assert_operation_undo_redo(
        &closed.document,
        Operation::ReopenPanel {
            panel: "c".into(),
            target_tabs: "tabs-ab".into(),
            index: 2,
        },
    );
    validate_layout(&reopened.document).unwrap();
    let reordered = apply_operation(
        &doc,
        Operation::ReorderPanel {
            panel: "b".into(),
            container: "tabs-ab".into(),
            index: 0,
        },
    )
    .unwrap();
    let pinned = apply_operation(
        &apply_operation(
            &doc,
            Operation::AutoHide {
                panel: "c".into(),
                edge: DockEdge::Right,
            },
        )
        .unwrap()
        .document,
        Operation::PinPanel {
            panel: "c".into(),
            target_tabs: "tabs-ab".into(),
            index: 2,
        },
    )
    .unwrap();
    assert_operation_undo_redo(
        &apply_operation(
            &doc,
            Operation::AutoHide {
                panel: "c".into(),
                edge: DockEdge::Right,
            },
        )
        .unwrap()
        .document,
        Operation::PinPanel {
            panel: "c".into(),
            target_tabs: "tabs-ab".into(),
            index: 2,
        },
    );
    validate_layout(&reordered.document).unwrap();
    validate_layout(&pinned.document).unwrap();

    let added_panel = apply_operation(
        &doc,
        Operation::AddPanel {
            descriptor: PanelDescriptor::new("e", "E"),
            target: Some("tabs-ab".into()),
        },
    )
    .unwrap();
    assert_operation_undo_redo(
        &doc,
        Operation::AddPanel {
            descriptor: PanelDescriptor::new("e", "E"),
            target: Some("tabs-ab".into()),
        },
    );
    validate_layout(&added_panel.document).unwrap();
    assert!(
        added_panel
            .document
            .descriptor(&PanelId::from("e"))
            .is_some()
    );

    let added_group = apply_operation(
        &added_panel.document,
        Operation::AddGroup {
            descriptor: PanelDescriptor::new("f", "F"),
            target: Some("tabs-ab".into()),
            orientation: Orientation::Horizontal,
            before: false,
            ratio: 0.35,
        },
    )
    .unwrap();
    assert_operation_undo_redo(
        &added_panel.document,
        Operation::AddGroup {
            descriptor: PanelDescriptor::new("f", "F"),
            target: Some("tabs-ab".into()),
            orientation: Orientation::Horizontal,
            before: false,
            ratio: 0.35,
        },
    );
    validate_layout(&added_group.document).unwrap();

    let floated = apply_operation(
        &added_group.document,
        Operation::FloatPanel {
            panel: "f".into(),
            window: "float-f".into(),
            geometry: Rect {
                x: 40,
                y: 50,
                width: 320,
                height: 220,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
    )
    .unwrap();
    let moved_window = apply_operation(
        &floated.document,
        Operation::UpdateFloatingWindow {
            window: "float-f".into(),
            geometry: Rect {
                x: 80,
                y: 95,
                width: 400,
                height: 280,
            },
        },
    )
    .unwrap();
    assert_operation_undo_redo(
        &floated.document,
        Operation::UpdateFloatingWindow {
            window: "float-f".into(),
            geometry: Rect {
                x: 80,
                y: 95,
                width: 400,
                height: 280,
            },
        },
    );
    validate_layout(&moved_window.document).unwrap();
    assert!(moved_window.diff.patches.iter().any(|patch| matches!(
        patch,
        DiffPatch::WindowGeometryUpdated {
            window: WindowId(id),
            geometry: Rect {
                x: 80,
                y: 95,
                width: 400,
                height: 280
            }
        } if id == "float-f"
    )));

    let cleared = apply_operation(&moved_window.document, Operation::ClearLayout).unwrap();
    assert_operation_undo_redo(&moved_window.document, Operation::ClearLayout);
    validate_layout(&cleared.document).unwrap();
    assert!(cleared.document.root.is_none());
    assert!(cleared.document.floating_windows.is_empty());
    assert_eq!(cleared.document.closed_panels.len(), 6);

    let reopened_root = apply_operation(
        &cleared.document,
        Operation::ReopenPanel {
            panel: "a".into(),
            target_tabs: "root".into(),
            index: 0,
        },
    )
    .unwrap();
    validate_layout(&reopened_root.document).unwrap();
    assert!(reopened_root.document.root.is_some());
}

#[test]
fn panel_swap_preserves_leaf_positions_and_undoes_as_one_operation() {
    let original = fixture();
    let mut doc = original.clone();
    let mut history = UndoStack::new(4);
    let result = history
        .execute(
            &mut doc,
            Operation::SwapPanels {
                first: "a".into(),
                second: "b".into(),
            },
        )
        .unwrap();
    assert_eq!(result.operation_id, "swap-panels");
    assert_eq!(
        result
            .diff
            .patches
            .iter()
            .filter(|patch| matches!(patch, DiffPatch::PanelReparented { .. }))
            .count(),
        2,
        "both exchanged panel identities must be represented in the diff"
    );
    let root = doc.root.as_ref().unwrap();
    let first = find_node(root, &"node-a".into()).unwrap();
    let second = find_node(root, &"node-b".into()).unwrap();
    assert!(
        matches!(&first.kind, NodeKind::Panel { panel } if panel == &PanelId::from("b")),
        "first slot after swap: {:?}",
        first.kind
    );
    assert!(matches!(&second.kind, NodeKind::Panel { panel } if panel == &PanelId::from("a")));
    assert_eq!(doc.revision, original.revision + 1);
    history.undo(&mut doc).unwrap();
    let mut restored = original;
    restored.revision = doc.revision;
    assert_eq!(doc, restored);
}

#[test]
fn centered_panel_group_is_atomic_ordered_and_undoable() {
    fn tabs_with_panel<'a>(node: &'a Node, panel: &PanelId) -> Option<&'a Node> {
        match &node.kind {
            NodeKind::Tabs { items, .. } => {
                if items.iter().any(
                    |child| matches!(&child.kind, NodeKind::Panel { panel: id } if id == panel),
                ) {
                    Some(node)
                } else {
                    items.iter().find_map(|child| tabs_with_panel(child, panel))
                }
            }
            NodeKind::DocumentWell { groups, .. } => groups
                .iter()
                .find_map(|child| tabs_with_panel(child, panel)),
            NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => children
                .iter()
                .find_map(|child| tabs_with_panel(child, panel)),
            NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => None,
        }
    }
    let original = fixture();
    let mut doc = original.clone();
    let mut history = UndoStack::new(8);
    let operation = Operation::CenterPanelGroup {
        panels: vec!["c".into(), "d".into()],
        target_panel: "a".into(),
    };
    let result = history.execute(&mut doc, operation.clone()).unwrap();
    assert_eq!(result.operation_id, "center-panel-group");
    assert_eq!(doc.revision, original.revision + 1);
    let target_tabs = doc
        .root
        .as_ref()
        .and_then(|root| tabs_with_panel(root, &"a".into()))
        .expect("target panel remains in the primary layout");
    let NodeKind::Tabs {
        items,
        active_index,
        ..
    } = &target_tabs.kind
    else {
        panic!("target should be in a tab group");
    };
    assert_eq!(
        items
            .iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Panel { panel } => Some(panel.0.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec!["a", "c", "d", "b"]
    );
    assert_eq!(*active_index, 2);

    let before_invalid = doc.clone();
    let error = history
        .execute(
            &mut doc,
            Operation::CenterPanelGroup {
                panels: vec!["b".into(), "b".into()],
                target_panel: "a".into(),
            },
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::DuplicateId);
    assert_eq!(
        doc, before_invalid,
        "invalid group move must not mutate the document"
    );

    history.undo(&mut doc).unwrap();
    let mut restored = original;
    restored.revision = doc.revision;
    assert_eq!(doc, restored, "one undo restores the entire selection");
}

#[test]
fn operation_request_rejects_unknown_fields_and_discriminators() {
    assert!(
        serde_json::from_value::<Operation>(serde_json::json!({
            "operation": "clear_layout",
            "unexpected": true,
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Operation>(serde_json::json!({
            "operation": "not_a_real_operation",
        }))
        .is_err()
    );
}

#[test]
fn urne_v2_projection_is_typed_bounded_and_deterministic() {
    let doc = fixture();
    let first = UrneProjection::from_layout(&doc).unwrap();
    let second = UrneProjection::from_layout(&doc).unwrap();
    assert_eq!(first, second);
    assert_eq!(URNE_ENGINE, "urne");
    assert_eq!(URNE_VERSION, 2);
    assert!(first.diagnostics.is_empty());
    let types: std::collections::BTreeSet<_> = first
        .nodes
        .iter()
        .map(|node| node.node_type.as_str())
        .collect();
    assert!(types.contains("workspace"));
    assert!(types.contains("layout"));
    assert!(types.contains("region"));
    assert!(types.contains("panel"));
    let ids: std::collections::BTreeSet<_> =
        first.nodes.iter().map(|node| node.id.as_str()).collect();
    for edge in &first.edges {
        assert!(ids.contains(edge.source.as_str()));
        assert!(ids.contains(edge.target.as_str()));
        assert_eq!(edge.edge_type, "contains");
    }
    for node in &first.nodes {
        assert!(node.parent_ids.iter().all(|parent| {
            first
                .edges
                .iter()
                .any(|edge| edge.source == *parent && edge.target == node.id)
        }));
        assert!(node.child_ids.iter().all(|child| {
            first
                .edges
                .iter()
                .any(|edge| edge.source == node.id && edge.target == *child)
        }));
    }
    let json = serde_json::to_value(first).unwrap();
    assert_eq!(json["version"], URNE_VERSION);
    assert!(json.get("nodes").is_some() && json.get("edges").is_some());
    assert!(
        json.get("sources")
            .unwrap()
            .get("doofusdock.layout")
            .is_some()
    );
    let floated = apply_operation(
        &doc,
        Operation::FloatPanel {
            panel: "c".into(),
            window: "float-c".into(),
            geometry: Rect {
                x: 20,
                y: 30,
                width: 500,
                height: 400,
            },
            monitor: "main".into(),
            scale: 1.0,
        },
    )
    .unwrap();
    let floating_projection = UrneProjection::from_layout(&floated.document).unwrap();
    assert!(
        floating_projection
            .nodes
            .iter()
            .any(|node| node.node_type == "window")
    );
    assert!(
        floating_projection
            .nodes
            .iter()
            .filter(|node| node.node_type == "panel")
            .any(|node| node.metadata.get("minimum").is_some()
                && node.metadata.get("docking").is_some())
    );
}

#[test]
fn reusable_workspace_profiles_instantiate_resolve_and_round_trip() {
    let doc = fixture();
    let profiles = builtin_workspace_profiles();
    assert!(profiles.len() >= 9);
    for profile in &profiles {
        validate_profile(profile).unwrap();
    }
    let workbench = profiles
        .iter()
        .find(|profile| profile.id == "workbench")
        .unwrap();
    let assignments = std::collections::BTreeMap::from([
        ("navigation".to_owned(), vec![PanelId::from("a")]),
        ("main".to_owned(), vec![PanelId::from("b")]),
        ("inspector".to_owned(), vec![PanelId::from("c")]),
        // `output` is optional and intentionally omitted.
    ]);
    let applied = apply_workspace_profile(&doc, workbench, &assignments).unwrap();
    assert_eq!(applied.document.revision, doc.revision + 1);
    assert_eq!(applied.document.closed_panels, vec![PanelId::from("d")]);
    validate_layout(&applied.document).unwrap();
    assert_eq!(
        resolve_profile_region(&applied.document, "workbench", "main"),
        applied.semantic_regions.get("main").cloned()
    );

    // The generated profile is a normal canonical layout: persistence and the
    // URNE projection consume it without a profile-specific storage format.
    let encoded = serialize_layout(&applied.document).unwrap();
    let restored = deserialize_layout(&encoded, PersistenceLimits::default()).unwrap();
    assert_eq!(restored, applied.document);
    assert!(
        UrneProjection::from_layout(&restored)
            .unwrap()
            .nodes
            .iter()
            .any(|node| node.node_type == "region")
    );
    let repeated = apply_workspace_profile(&restored, workbench, &assignments).unwrap();
    assert!(repeated.diff.is_empty());
    assert_eq!(repeated.document.revision, restored.revision);

    // A region target continues to resolve after another panel is tab-docked
    // into its anchor panel.
    let edited = apply_operation(
        &applied.document,
        Operation::CenterPanel {
            panel: "c".into(),
            target_panel: "b".into(),
        },
    )
    .unwrap();
    assert!(resolve_profile_region(&edited.document, "workbench", "main").is_some());
}

#[test]
fn every_builtin_profile_instantiates_as_a_valid_canonical_layout() {
    let doc = fixture();
    let panel_ids = ["a", "b", "c", "d"];
    let profiles = builtin_workspace_profiles();
    assert_eq!(profiles.len(), 71);
    let profile_ids: std::collections::BTreeSet<_> =
        profiles.iter().map(|profile| profile.id.as_str()).collect();
    let catalog_ids = [
        "single-pane",
        "sidebar-navigation",
        "top-navigation",
        "bottom-navigation",
        "navigation-rail",
        "tabbed-navigation",
        "tree-navigation",
        "breadcrumb-navigation",
        "hub-and-spoke",
        "master-detail",
        "list-detail",
        "browse-preview",
        "wizard-stepper",
        "dashboard",
        "workbench",
        "document-workspace",
        "canvas",
        "inspector-properties",
        "multi-column-workspace",
        "split-view",
        "nested-split-view",
        "tabbed-panels",
        "docked-panels",
        "auto-hide-panels",
        "floating-panels",
        "overlay-panel",
        "slide-in-panel",
        "modal-dialog",
        "modeless-dialog",
        "focus-full-screen",
        "chat",
        "irc-layout",
        "threaded-conversation",
        "inbox-layout",
        "email-three-pane",
        "presence-collaboration",
        "feed-timeline",
        "table-grid",
        "spreadsheet",
        "card-grid",
        "gallery",
        "kanban-board",
        "calendar",
        "timeline-gantt",
        "map-based",
        "catalog-storefront",
        "search-results",
        "tree-table",
        "data-explorer",
        "media-player",
        "playlist-library",
        "video-editing",
        "audio-mixing",
        "image-editor",
        "3d-editor",
        "node-graph",
        "code-diff",
        "terminal-console",
        "monitoring-command-center",
        "one-column",
        "two-column",
        "three-column",
        "grid",
        "masonry",
        "asymmetric",
        "responsive-adaptive",
        "card-based",
        "layered",
        "spatial",
    ];
    assert_eq!(catalog_ids.len(), 69);
    for id in catalog_ids {
        assert!(profile_ids.contains(id), "missing catalog profile {id}");
    }
    for profile in &profiles {
        let assignments: std::collections::BTreeMap<_, _> = profile
            .regions
            .iter()
            .enumerate()
            .map(|(index, region)| (region.id.clone(), vec![PanelId::from(panel_ids[index])]))
            .collect();
        let application = apply_workspace_profile(&doc, profile, &assignments)
            .unwrap_or_else(|error| panic!("profile {} failed: {error}", profile.id));
        if profile.id == "layered" {
            let NodeKind::Extension {
                modality,
                payload,
                children,
            } = &application.document.root.as_ref().unwrap().kind
            else {
                panic!("layered profile did not instantiate as an extension")
            };
            assert_eq!(modality.0, LAYERED_MODALITY);
            assert_eq!(
                LayeredConfiguration::parse(payload, children)
                    .unwrap()
                    .back_to_front
                    .len(),
                children.len()
            );
        }
        assert_eq!(
            application.presentation, profile.presentation,
            "{}",
            profile.id
        );
        validate_layout(&application.document).unwrap();
        let encoded = serialize_layout(&application.document).unwrap();
        let restored = deserialize_layout(&encoded, PersistenceLimits::default()).unwrap();
        assert_eq!(restored, application.document, "{}", profile.id);
        assert!(
            UrneProjection::from_layout(&restored).is_ok(),
            "{}",
            profile.id
        );
    }

    let modal = profiles
        .iter()
        .find(|profile| profile.id == "modal-dialog")
        .unwrap();
    let modeless = profiles
        .iter()
        .find(|profile| profile.id == "modeless-dialog")
        .unwrap();
    assert!(matches!(
        modal.presentation,
        ProfilePresentation::Overlay { modal: true, .. }
    ));
    assert!(matches!(
        modeless.presentation,
        ProfilePresentation::ModelessWindow { .. }
    ));
    let main_only =
        std::collections::BTreeMap::from([("main".to_owned(), vec![PanelId::from("a")])]);
    let dialog_omitted = apply_workspace_profile(&doc, modal, &main_only).unwrap();
    assert_eq!(
        dialog_omitted.presentation,
        ProfilePresentation::Standard,
        "an omitted optional dialog must not leave a dangling modal presentation"
    );
    let mut malformed = modal.clone();
    malformed.presentation = ProfilePresentation::Focus {
        region: "missing-region".into(),
    };
    assert!(validate_profile(&malformed).is_err());
}

#[test]
fn workspace_profile_rejects_missing_required_and_duplicate_panel_assignments() {
    let doc = fixture();
    let workbench = builtin_workspace_profiles()
        .into_iter()
        .find(|profile| profile.id == "workbench")
        .unwrap();
    let missing_main =
        std::collections::BTreeMap::from([("navigation".to_owned(), vec![PanelId::from("a")])]);
    assert!(apply_workspace_profile(&doc, &workbench, &missing_main).is_err());
    assert_eq!(doc.revision, 0, "failed profile application is atomic");

    let duplicate = std::collections::BTreeMap::from([
        ("navigation".to_owned(), vec![PanelId::from("a")]),
        ("main".to_owned(), vec![PanelId::from("a")]),
    ]);
    assert!(apply_workspace_profile(&doc, &workbench, &duplicate).is_err());
    assert_eq!(doc.revision, 0, "failed profile application is atomic");
}

#[test]
fn diffs_report_structural_panel_and_window_changes() {
    let doc = fixture();
    let floated = apply_operation(
        &doc,
        Operation::FloatPanel {
            panel: "c".into(),
            window: "float-c".into(),
            geometry: Rect {
                x: 1,
                y: 2,
                width: 300,
                height: 200,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
    )
    .unwrap();
    assert!(
        floated
            .diff
            .patches
            .iter()
            .any(|p| matches!(p, DiffPatch::WindowCreated { .. }))
    );
    assert!(
        floated.diff.patches.iter().any(
            |p| matches!(p, DiffPatch::PanelReparented { panel: PanelId(id), .. } if id == "c")
        )
    );
    assert!(floated.diff.patches.iter().any(|p| matches!(
        p,
        DiffPatch::NodeCreated { .. } | DiffPatch::NodeRemoved { .. }
    )));
    let hidden = apply_operation(
        &doc,
        Operation::AutoHide {
            panel: "c".into(),
            edge: DockEdge::Bottom,
        },
    )
    .unwrap();
    assert!(hidden.diff.patches.iter().any(|p| matches!(
        p,
        DiffPatch::DrawerChanged {
            auto_hidden: true,
            ..
        }
    )));
    let closed = apply_operation(&doc, Operation::ClosePanel { panel: "c".into() }).unwrap();
    assert!(closed.diff.patches.iter().any(|p| matches!(p, DiffPatch::PanelVisibilityChanged { panel: PanelId(id), visible: false } if id == "c")));
}

#[test]
fn extension_payload_updates_are_bounded_differenced_undoable_and_persistent() {
    let mut doc = fixture();
    let root = doc.root.take().unwrap();
    doc.root = Some(Arc::new(Node {
        id: "spatial-container".into(),
        kind: NodeKind::Extension {
            modality: "doofusdock.spatial".into(),
            payload: r#"{"coordinate_space":"workspace","placement":"freeform"}"#.into(),
            children: vec![root],
        },
    }));
    validate_layout(&doc).unwrap();
    let old_payload = match &doc.root.as_ref().unwrap().kind {
        NodeKind::Extension { payload, .. } => payload.clone(),
        _ => unreachable!(),
    };
    let new_payload =
        r#"{"placements":[{"child":"node-a","x":0.25,"y":0.2,"width":0.35,"height":0.4}]}"#;
    let mut history = UndoStack::new(8);
    let updated = history
        .execute(
            &mut doc,
            Operation::UpdateExtensionPayload {
                container: "spatial-container".into(),
                payload: new_payload.into(),
            },
        )
        .unwrap();
    assert_eq!(doc.revision, 1);
    assert!(updated.diff.patches.iter().any(|patch| matches!(
        patch,
        DiffPatch::ExtensionPayloadUpdated { container: NodeId(id), modality: ModalityId(modality), payload }
            if id == "spatial-container" && modality == "doofusdock.spatial" && payload == new_payload
    )));
    let encoded = serialize_layout(&doc).unwrap();
    assert_eq!(
        deserialize_layout(&encoded, PersistenceLimits::default()).unwrap(),
        doc
    );

    history.undo(&mut doc).unwrap();
    assert_eq!(doc.revision, 2);
    assert!(
        matches!(&doc.root.as_ref().unwrap().kind, NodeKind::Extension { payload, .. } if payload == &old_payload)
    );
    history.redo(&mut doc).unwrap();
    assert_eq!(doc.revision, 3);
    assert!(
        matches!(&doc.root.as_ref().unwrap().kind, NodeKind::Extension { payload, .. } if payload == new_payload)
    );

    let before = doc.clone();
    assert!(
        apply_operation(
            &doc,
            Operation::UpdateExtensionPayload {
                container: "missing-container".into(),
                payload: "{}".into(),
            }
        )
        .is_err()
    );
    assert!(
        apply_operation(
            &doc,
            Operation::UpdateExtensionPayload {
                container: "spatial-container".into(),
                payload: "x".repeat(4097),
            }
        )
        .is_err()
    );
    assert_eq!(
        doc, before,
        "failed updates must leave canonical state unchanged"
    );
}

#[test]
fn constraints_history_and_persistence_round_trip() {
    let mut doc = fixture();
    for panel in Arc::make_mut(&mut doc.panels) {
        panel.minimum = Size::new(100, 60);
    }
    let preview = preview_split(
        &doc,
        &NodeId::from("workspace-root"),
        0,
        0.2,
        Size::new(1000, 600),
    )
    .unwrap();
    assert!((preview.weights[0] - 0.2).abs() < 1e-9);
    let projected = project_narrow_viewport(&doc, Size::new(1, 1)).unwrap();
    assert!(
        projected
            .collapsed_splits
            .contains(&NodeId::from("workspace-root"))
    );
    assert_eq!(
        clamp_to_work_area(
            Rect {
                x: -500,
                y: 2000,
                width: 900,
                height: 900
            },
            Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600
            }
        ),
        Rect {
            x: 0,
            y: 0,
            width: 800,
            height: 600
        }
    );

    let mut history = UndoStack::new(4);
    history
        .execute(
            &mut doc,
            Operation::SelectTab {
                tabs: "tabs-ab".into(),
                index: 1,
            },
        )
        .unwrap();
    assert_eq!(history.undo_size(), 1);
    history.undo(&mut doc).unwrap();
    assert_eq!(history.redo_size(), 1);
    history.redo(&mut doc).unwrap();
    let encoded = serialize_layout(&doc).unwrap();
    let decoded = deserialize_layout(&encoded, PersistenceLimits::default()).unwrap();
    assert_eq!(serialize_layout(&decoded).unwrap(), encoded);
    let corrupted = encoded.replacen("workspace-root", "workspace-bad", 1);
    assert!(deserialize_layout(&corrupted, PersistenceLimits::default()).is_err());
}

#[test]
fn schema_zero_migration_uses_cpp_defaults_and_canonical_field_order() {
    let legacy = include_str!("fixtures/layout_schema_v0.json");
    let sum = checksum_layout_json(legacy).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(legacy).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("checksum".into(), serde_json::Value::String(sum));
    let encoded = serde_json::to_string(&value).unwrap();
    let migrated = deserialize_layout(&encoded, PersistenceLimits::default()).unwrap();
    assert_eq!(migrated.schema_version, CURRENT_LAYOUT_SCHEMA);
    assert_eq!(migrated.id, LayoutId::from("legacy-layout"));
    assert_eq!(migrated.workspace, WorkspaceId::from("legacy-workspace"));
    assert_eq!(migrated.revision, 7);
    assert_eq!(migrated.panels[0].id, PanelId::from("legacy-panel"));
    assert_eq!(
        migrated.floating_windows[0].id,
        WindowId::from("legacy-float")
    );
    assert_eq!(migrated.floating_windows[0].monitor_id, "");
    assert_eq!(migrated.floating_windows[0].scale, 1.0);
    assert_eq!(migrated.floating_windows[0].z_order_hint, 0);

    let canonical = serialize_layout(&fixture()).unwrap();
    let positions = [
        "\"schema_version\"",
        "\"layout_id\"",
        "\"workspace_id\"",
        "\"revision\"",
        "\"panels\"",
        "\"closed_panels\"",
        "\"auto_hide_panels\"",
        "\"maximized_panel\"",
        "\"root\"",
        "\"windows\"",
    ]
    .map(|key| canonical.find(key).unwrap());
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "serialized top-level keys must match the C++ canonical order"
    );
}

#[test]
fn schema_one_migration_defaults_tab_groups_to_expanded() {
    let mut value: serde_json::Value =
        serde_json::from_str(&serialize_layout(&fixture()).unwrap()).unwrap();
    value["schema_version"] = serde_json::Value::from(1);
    let sum = checksum_layout_json(&serde_json::to_string(&value).unwrap()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("checksum".into(), serde_json::Value::String(sum));
    let restored = deserialize_layout(
        &serde_json::to_string(&value).unwrap(),
        PersistenceLimits::default(),
    )
    .unwrap();
    assert_eq!(restored.schema_version, CURRENT_LAYOUT_SCHEMA);
    let root = restored.root.as_ref().unwrap();
    let tabs = find_node(root, &NodeId::from("tabs-ab")).unwrap();
    assert!(matches!(
        tabs.kind,
        NodeKind::Tabs {
            minimized: false,
            ..
        }
    ));
}

#[test]
fn minimized_tab_group_operation_is_revisioned_diffed_and_persisted() {
    let doc = fixture();
    let operation = Operation::SetGroupMinimized {
        group: "tabs-ab".into(),
        minimized: true,
    };
    let changed = apply_operation(&doc, operation.clone()).unwrap();
    let reconverted = apply_operation(
        &changed.document,
        Operation::ConvertContainer {
            container: "tabs-ab".into(),
            target: ContainerModality::Tabbed { active_index: 1 },
        },
    )
    .unwrap();
    let reconverted_tabs = find_node(
        reconverted.document.root.as_ref().unwrap(),
        &NodeId::from("tabs-ab"),
    )
    .unwrap();
    assert!(matches!(
        reconverted_tabs.kind,
        NodeKind::Tabs {
            minimized: true,
            ..
        }
    ));
    let mut history = UndoStack::new(2);
    let mut reversible = doc.clone();
    history.execute(&mut reversible, operation).unwrap();
    history.undo(&mut reversible).unwrap();
    let tabs = find_node(reversible.root.as_ref().unwrap(), &NodeId::from("tabs-ab")).unwrap();
    assert!(matches!(
        tabs.kind,
        NodeKind::Tabs {
            minimized: false,
            ..
        }
    ));
    history.redo(&mut reversible).unwrap();
    let tabs = find_node(reversible.root.as_ref().unwrap(), &NodeId::from("tabs-ab")).unwrap();
    assert!(matches!(
        tabs.kind,
        NodeKind::Tabs {
            minimized: true,
            ..
        }
    ));
    assert_eq!(changed.document.revision, doc.revision + 1);
    assert!(
        changed
            .diff
            .patches
            .contains(&DiffPatch::TabGroupMinimizedChanged {
                group: "tabs-ab".into(),
                minimized: true,
            })
    );
    let encoded = serialize_layout(&changed.document).unwrap();
    let restored = deserialize_layout(&encoded, PersistenceLimits::default()).unwrap();
    let tabs = find_node(restored.root.as_ref().unwrap(), &NodeId::from("tabs-ab")).unwrap();
    assert!(matches!(
        tabs.kind,
        NodeKind::Tabs {
            minimized: true,
            ..
        }
    ));
}

#[test]
fn diff_reports_minimize_and_active_tab_changes_together() {
    let original = fixture();
    let selected = apply_operation(
        &original,
        Operation::SelectTab {
            tabs: "tabs-ab".into(),
            index: 1,
        },
    )
    .unwrap()
    .document;
    let minimized = apply_operation(
        &selected,
        Operation::SetGroupMinimized {
            group: "tabs-ab".into(),
            minimized: true,
        },
    )
    .unwrap()
    .document;
    let diff = diff_layouts(&original, &minimized);
    assert!(diff.patches.contains(&DiffPatch::ActiveTabChanged {
        tabs: "tabs-ab".into(),
        active_index: 1,
    }));
    assert!(diff.patches.contains(&DiffPatch::TabGroupMinimizedChanged {
        group: "tabs-ab".into(),
        minimized: true,
    }));
}

#[test]
fn zero_capacity_history_still_commits_transactions() {
    let mut doc = fixture();
    let mut history = UndoStack::new(0);
    let result = history
        .execute(
            &mut doc,
            Operation::SelectTab {
                tabs: "tabs-ab".into(),
                index: 1,
            },
        )
        .unwrap();
    assert_eq!(doc.revision, 1);
    assert_eq!(doc, result.document);
    assert_eq!(history.undo_size(), 0);
    assert!(history.undo(&mut doc).is_err());
}

#[test]
fn configurable_string_and_preference_tree_limits_are_enforced() {
    let doc = fixture();
    let encoded = serialize_layout(&doc).unwrap();
    let limits = PersistenceLimits {
        maximum_string_length: 4,
        ..PersistenceLimits::default()
    };
    assert!(deserialize_layout(&encoded, limits).is_err());

    let folder = std::env::temp_dir().join(format!("doofusdock-limit-test-{}", std::process::id()));
    let store = WorkspacePreferencesStore::new(&folder, limits);
    assert!(
        store
            .save(
                &WorkspaceId::from("ws"),
                &serde_json::json!({ "theme": "dark" })
            )
            .is_err()
    );
    if folder.exists() {
        std::fs::remove_dir_all(folder).unwrap();
    }
}

fn extension_descriptor(
    id: &str,
    activate: Option<ActivateHook>,
    shutdown: Option<ShutdownHook>,
) -> ExtensionDescriptor {
    ExtensionDescriptor {
        namespaced_id: id.into(),
        version: ApiVersion { major: 1, minor: 0 },
        api: CompatibilityRange {
            minimum: ApiVersion { major: 1, minor: 0 },
            maximum: ApiVersion { major: 1, minor: 2 },
        },
        on_activate: activate,
        on_shutdown: shutdown,
    }
}

#[test]
fn transition_provider_capabilities_are_checked_before_callback_execution() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_provider = calls.clone();
    let mut registry = ExtensionRegistry::default();
    registry
        .register_transition(TransitionExtension {
            descriptor: extension_descriptor("sample:capability-transition", None, None),
            duration: Arc::new(move |request| {
                calls_for_provider.fetch_add(1, Ordering::SeqCst);
                request.duration_ms
            }),
            required_capabilities: vec!["vector_timeline".into()],
            incompatible_capabilities: vec!["exclusive_surface".into()],
        })
        .unwrap();

    let invalid = registry.register_transition(TransitionExtension {
        descriptor: extension_descriptor("sample:invalid-transition", None, None),
        duration: Arc::new(|request| request.duration_ms),
        required_capabilities: vec!["same".into()],
        incompatible_capabilities: vec!["same".into()],
    });
    assert_eq!(invalid.unwrap_err().code, ErrorCode::InvalidLayout);
    assert_eq!(
        registry.counts()[2],
        1,
        "invalid provider is not partially registered"
    );
    registry.freeze();
    registry.activate("sample:capability-transition").unwrap();

    let request = |host_capabilities: Vec<String>| TransitionRequest {
        id: "sample:capability-transition".into(),
        duration_ms: 400,
        reduced_motion: false,
        host_capabilities,
    };
    let id = TransitionId::from("sample:capability-transition");
    let missing = registry
        .transition_duration(&id, &request(vec![]), 500)
        .unwrap_err();
    assert_eq!(missing.code, ErrorCode::UnsupportedOperation);
    assert_eq!(
        missing.context,
        "sample:capability-transition.vector_timeline"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let incompatible = registry
        .transition_duration(
            &id,
            &request(vec!["vector_timeline".into(), "exclusive_surface".into()]),
            500,
        )
        .unwrap_err();
    assert_eq!(incompatible.code, ErrorCode::UnsupportedOperation);
    assert_eq!(
        incompatible.context,
        "sample:capability-transition.exclusive_surface"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    assert_eq!(
        registry
            .transition_duration(&id, &request(vec!["vector_timeline".into()]), 500)
            .unwrap(),
        400
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let supported_host = HostCapabilities {
        supported: BTreeSet::from(["vector_timeline".into()]),
    };
    let supported = registry
        .capability_snapshot(&supported_host)
        .into_iter()
        .find(|snapshot| snapshot.id == "sample:capability-transition")
        .unwrap();
    assert_eq!(supported.supported_on_host, Some(true));
    assert!(supported.missing_capabilities.is_empty());

    let incompatible_host = HostCapabilities {
        supported: BTreeSet::from(["vector_timeline".into(), "exclusive_surface".into()]),
    };
    let unsupported = registry
        .capability_snapshot(&incompatible_host)
        .into_iter()
        .find(|snapshot| snapshot.id == "sample:capability-transition")
        .unwrap();
    assert_eq!(unsupported.supported_on_host, Some(false));
    assert!(
        unsupported
            .reason_unavailable
            .unwrap()
            .contains("exclusive_surface")
    );
}

#[test]
fn inactive_extension_callbacks_are_rejected_without_invoking_providers() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = ExtensionRegistry::default();
    let panel_calls = calls.clone();
    registry
        .register_panel_provider(PanelProviderExtension {
            descriptor: extension_descriptor("idle:panel", None, None),
            validate_panel: Arc::new(move |_| {
                panel_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        })
        .unwrap();
    let modality_calls = calls.clone();
    registry
        .register_modality(ModalityExtension {
            descriptor: extension_descriptor("idle:modality", None, None),
            validate_config: Arc::new(move |_| {
                modality_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
            operations: vec!["configure".into()],
            capabilities: vec![],
            fallback: ExtensionFallbackPolicy::RejectUnavailable,
            configuration_schema: serde_json::json!({"type":"object"}),
            required_capabilities: vec![],
            incompatible_capabilities: vec![],
        })
        .unwrap();
    let transition_calls = calls.clone();
    registry
        .register_transition(TransitionExtension {
            descriptor: extension_descriptor("idle:transition", None, None),
            duration: Arc::new(move |request| {
                transition_calls.fetch_add(1, Ordering::SeqCst);
                request.duration_ms
            }),
            required_capabilities: vec![],
            incompatible_capabilities: vec![],
        })
        .unwrap();
    let codec_calls = calls.clone();
    registry
        .register_persistence_codec(PersistenceCodecExtension {
            descriptor: extension_descriptor("idle:codec", None, None),
            encode: Arc::new(move |value| {
                codec_calls.fetch_add(1, Ordering::SeqCst);
                Ok(value.clone())
            }),
            decode: Arc::new(move |value| Ok(value.clone())),
        })
        .unwrap();
    registry
        .register_command(CommandExtension {
            descriptor: extension_descriptor("idle:command", None, None),
            execute: Arc::new(|_, _| {
                Err(DockError::new(
                    ErrorCode::Conflict,
                    "unexpected callback",
                    "idle",
                ))
            }),
        })
        .unwrap();
    let host_calls = calls.clone();
    registry
        .register_host_adapter(HostAdapterExtension {
            descriptor: extension_descriptor("idle:host", None, None),
            validate_diff: Arc::new(move |_| {
                host_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        })
        .unwrap();

    let doc = fixture();
    let transition = TransitionRequest {
        id: "idle:transition".into(),
        duration_ms: 10,
        reduced_motion: false,
        host_capabilities: vec![],
    };
    let checks = [
        registry
            .validate_panel("idle:panel", &doc.panels[0])
            .unwrap_err(),
        registry
            .validate_modality_config(&ModalityId::from("idle:modality"), &serde_json::json!({}))
            .unwrap_err(),
        registry
            .transition_duration(&TransitionId::from("idle:transition"), &transition, 20)
            .unwrap_err(),
        registry
            .encode_extension_state("idle:codec", &serde_json::json!({}))
            .unwrap_err(),
        registry
            .decode_extension_state("idle:codec", &serde_json::json!({}))
            .unwrap_err(),
        registry
            .execute_command("idle:command", &doc, &serde_json::json!({}))
            .unwrap_err(),
        registry
            .validate_host_diff("idle:host", &LayoutDiff::default())
            .unwrap_err(),
    ];
    assert!(checks.iter().all(|error| error.code == ErrorCode::Conflict));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "inactive providers must not run"
    );
}

#[test]
fn layout_providers_share_extension_identity_capabilities_and_lifecycle() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let make_extension = |id: &str, modality: &'static str| LayoutProviderExtension {
        descriptor: extension_descriptor(id, None, None),
        provider: Arc::new(ProviderForModality {
            modality,
            calls: calls.clone(),
        }),
    };
    struct ProviderForModality {
        modality: &'static str,
        calls: Arc<AtomicUsize>,
    }
    impl ExtensionLayoutProvider for ProviderForModality {
        fn modality(&self) -> &str {
            self.modality
        }
        fn arrange(
            &self,
            request: &ExtensionLayoutRequest<'_>,
        ) -> DockResult<Vec<ExtensionPlacement>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(request
                .children
                .iter()
                .enumerate()
                .map(|(child_index, _)| ExtensionPlacement {
                    child_index,
                    bounds: request.bounds,
                })
                .collect())
        }
    }

    let mut registry = ExtensionRegistry::default();
    registry
        .register_layout_provider(make_extension("sample:layout", "sample.custom_layout"))
        .unwrap();
    let duplicate = registry
        .register_layout_provider(make_extension("sample:layout-copy", "sample.custom_layout"))
        .unwrap_err();
    assert_eq!(duplicate.code, ErrorCode::DuplicateId);
    let builtin = registry
        .register_layout_provider(make_extension("sample:shadow", "doofusdock.spatial"))
        .unwrap_err();
    assert_eq!(builtin.code, ErrorCode::DuplicateId);

    let host = HostCapabilities::default();
    let snapshot = registry.capability_snapshot(&host);
    let capability = snapshot
        .iter()
        .find(|item| item.id == "sample:layout")
        .unwrap();
    assert_eq!(capability.kind, ExtensionKind::LayoutProvider);
    assert!(!capability.active);
    assert_eq!(capability.operations, vec!["arrange"]);
    assert_eq!(
        capability.provided_capabilities,
        vec!["layout_provider:sample.custom_layout"]
    );
    let projection = UrneProjection::from_capabilities(&registry, "test-host", &host).unwrap();
    let projected_provider = projection
        .nodes
        .iter()
        .find(|node| node.id.starts_with("provider:") && node.metadata["id"] == "sample:layout")
        .unwrap();
    assert_eq!(projected_provider.metadata["kind"], "layout_provider");

    let child_ids = [NodeId::from("child")];
    let child_metrics = [ExtensionChildMetrics {
        minimum: Size::new(20, 20),
        preferred: Size::new(100, 80),
    }];
    let request = ExtensionLayoutRequest {
        modality: "sample.custom_layout",
        payload: "{}",
        bounds: Rect {
            x: 0,
            y: 0,
            width: 200,
            height: 120,
        },
        child_ids: &child_ids,
        children: &child_metrics,
    };
    assert_eq!(
        registry.arrange_layout(&request).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    registry.freeze();
    registry.activate("sample:layout").unwrap();
    assert_eq!(registry.arrange_layout(&request).unwrap().len(), 1);
    registry.deactivate("sample:layout").unwrap();
    assert_eq!(
        registry.arrange_layout(&request).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn profile_providers_share_extension_identity_validation_and_lifecycle() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let provider_calls = calls.clone();
    let profile = builtin_workspace_profiles().into_iter().next().unwrap();
    let profile_id = profile.id.clone();
    let mut registry = ExtensionRegistry::default();
    registry
        .register_profile_provider(ProfileProviderExtension {
            descriptor: extension_descriptor("sample:profiles", None, None),
            provide_profile: Arc::new(move |requested| {
                provider_calls.fetch_add(1, Ordering::SeqCst);
                if requested == profile.id {
                    Ok(profile.clone())
                } else {
                    Err(DockError::new(
                        ErrorCode::NotFound,
                        "Profile is not available.",
                        requested,
                    ))
                }
            }),
        })
        .unwrap();
    assert_eq!(
        registry
            .register_command(CommandExtension {
                descriptor: extension_descriptor("sample:profiles", None, None),
                execute: Arc::new(|_, _| unreachable!()),
            })
            .unwrap_err()
            .code,
        ErrorCode::DuplicateId
    );
    let snapshot = registry.capability_snapshot(&HostCapabilities::default());
    let entry = snapshot
        .iter()
        .find(|item| item.id == "sample:profiles")
        .unwrap();
    assert_eq!(entry.kind, ExtensionKind::ProfileProvider);
    assert!(!entry.active);
    assert_eq!(entry.operations, vec!["get_profile"]);
    assert_eq!(entry.provided_capabilities, vec!["workspace_profile"]);
    let projection = UrneProjection::from_capabilities(
        &registry,
        "profile-test-host",
        &HostCapabilities::default(),
    )
    .unwrap();
    assert!(
        projection
            .edges
            .iter()
            .any(|edge| { edge.edge_type == "provides" && edge.source.starts_with("provider:") })
    );

    assert_eq!(
        registry
            .provide_profile("sample:profiles", &profile_id)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    registry.freeze();
    registry.activate("sample:profiles").unwrap();
    assert_eq!(
        registry
            .provide_profile("sample:profiles", &profile_id)
            .unwrap(),
        builtin_workspace_profiles()
            .into_iter()
            .find(|item| item.id == profile_id)
            .unwrap()
    );
    assert_eq!(
        registry
            .provide_profile("sample:profiles", "missing")
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    registry.deactivate("sample:profiles").unwrap();
    assert_eq!(
        registry
            .provide_profile("sample:profiles", &profile_id)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn profile_provider_rejects_mismatched_invalid_and_panicking_results() {
    let mut registry = ExtensionRegistry::default();
    registry
        .register_profile_provider(ProfileProviderExtension {
            descriptor: extension_descriptor("sample:bad-profile", None, None),
            provide_profile: Arc::new(|requested| {
                let mut profile = builtin_workspace_profiles().remove(0);
                match requested {
                    "mismatch" => profile.id = "different".into(),
                    "invalid" => profile.layout = ProfileLayout::Region("undeclared".into()),
                    "panic" => panic!("injected provider panic"),
                    _ => profile.id = requested.into(),
                }
                if requested == "invalid" {
                    profile.id = requested.into();
                }
                Ok(profile)
            }),
        })
        .unwrap();
    registry.freeze();
    registry.activate("sample:bad-profile").unwrap();
    assert_eq!(
        registry
            .provide_profile("sample:bad-profile", "mismatch")
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        registry
            .provide_profile("sample:bad-profile", "invalid")
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );
    assert_eq!(
        registry
            .provide_profile("sample:bad-profile", "panic")
            .unwrap_err()
            .code,
        ErrorCode::ExtensionError
    );
}

#[test]
fn renderer_providers_share_extension_identity_and_activation_lifecycle() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let provider_calls = calls.clone();
    let report = RendererCapabilityReport {
        renderer_id: "renderer:sample".into(),
        observations: vec![RendererCapabilityObservation {
            capability_id: "vector_chrome".into(),
            availability: RendererCapabilityAvailability::Available,
            details: "Vector rendering is active.".into(),
        }],
    };
    let mut registry = ExtensionRegistry::default();
    registry
        .register_renderer_provider(RendererProviderExtension {
            descriptor: extension_descriptor("sample:renderer", None, None),
            capability_report: Arc::new(move || {
                provider_calls.fetch_add(1, Ordering::SeqCst);
                Ok(report.clone())
            }),
        })
        .unwrap();
    assert_eq!(
        registry
            .register_command(CommandExtension {
                descriptor: extension_descriptor("sample:renderer", None, None),
                execute: Arc::new(|_, _| unreachable!()),
            })
            .unwrap_err()
            .code,
        ErrorCode::DuplicateId
    );
    let entry = registry
        .capability_snapshot(&HostCapabilities::default())
        .into_iter()
        .find(|item| item.id == "sample:renderer")
        .unwrap();
    assert_eq!(entry.kind, ExtensionKind::RendererProvider);
    assert_eq!(entry.operations, vec!["capability_report"]);
    assert_eq!(entry.provided_capabilities, vec!["renderer_capabilities"]);
    let projection = UrneProjection::from_capabilities(
        &registry,
        "renderer-provider-host",
        &HostCapabilities::default(),
    )
    .unwrap();
    assert!(projection.nodes.iter().any(|node| {
        node.node_type == "extension_provider" && node.metadata["kind"] == "renderer_provider"
    }));
    assert_eq!(
        registry
            .renderer_capability_report("sample:renderer")
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    registry.freeze();
    registry.activate("sample:renderer").unwrap();
    assert_eq!(
        registry
            .renderer_capability_report("sample:renderer")
            .unwrap()
            .renderer_id,
        "renderer:sample"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    registry.deactivate("sample:renderer").unwrap();
    assert_eq!(
        registry
            .renderer_capability_report("sample:renderer")
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let mut bad = ExtensionRegistry::default();
    bad.register_renderer_provider(RendererProviderExtension {
        descriptor: extension_descriptor("sample:bad-renderer", None, None),
        capability_report: Arc::new(|| {
            Ok(RendererCapabilityReport {
                renderer_id: "bad".into(),
                observations: vec![
                    RendererCapabilityObservation {
                        capability_id: "duplicate".into(),
                        availability: RendererCapabilityAvailability::Available,
                        details: "valid".into(),
                    },
                    RendererCapabilityObservation {
                        capability_id: "duplicate".into(),
                        availability: RendererCapabilityAvailability::Available,
                        details: "duplicate".into(),
                    },
                ],
            })
        }),
    })
    .unwrap();
    bad.freeze();
    bad.activate("sample:bad-renderer").unwrap();
    assert_eq!(
        bad.renderer_capability_report("sample:bad-renderer")
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );

    let mut panicking = ExtensionRegistry::default();
    panicking
        .register_renderer_provider(RendererProviderExtension {
            descriptor: extension_descriptor("sample:panicking-renderer", None, None),
            capability_report: Arc::new(|| panic!("injected renderer capability panic")),
        })
        .unwrap();
    panicking.freeze();
    panicking.activate("sample:panicking-renderer").unwrap();
    assert_eq!(
        panicking
            .renderer_capability_report("sample:panicking-renderer")
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );
}

#[test]
fn panel_shell_providers_return_validated_semantics_through_shared_lifecycle() {
    let panel = PanelDescriptor::new("editor", "Editor");
    let spec = PanelShellSpec {
        panel_id: panel.id.clone(),
        title_key: "panel.editor.title".into(),
        accessible_label: "Editor panel".into(),
        actions: vec![PanelShellActionSpec {
            id: "close".into(),
            title_key: "action.close".into(),
            kind: PanelShellActionKind::Close,
            enabled: true,
        }],
    };
    let mut registry = ExtensionRegistry::default();
    registry
        .register_panel_shell_provider(PanelShellProviderExtension {
            descriptor: extension_descriptor("sample:panel-shell", None, None),
            describe_shell: Arc::new({
                let spec = spec.clone();
                move |_| Ok(spec.clone())
            }),
        })
        .unwrap();
    let entry = registry
        .capability_snapshot(&HostCapabilities::default())
        .into_iter()
        .find(|item| item.id == "sample:panel-shell")
        .unwrap();
    assert_eq!(entry.kind, ExtensionKind::PanelShellProvider);
    assert_eq!(entry.operations, vec!["describe_shell"]);
    assert_eq!(entry.provided_capabilities, vec!["panel_shell"]);
    assert_eq!(
        registry
            .describe_panel_shell("sample:panel-shell", &panel)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let projection = UrneProjection::from_capabilities(
        &registry,
        "panel-shell-test-host",
        &HostCapabilities::default(),
    )
    .unwrap();
    assert!(
        projection
            .edges
            .iter()
            .any(|edge| { edge.edge_type == "provides" && edge.target.starts_with("capability:") })
    );

    registry.freeze();
    registry.activate("sample:panel-shell").unwrap();
    assert_eq!(
        registry
            .describe_panel_shell("sample:panel-shell", &panel)
            .unwrap(),
        spec
    );
    assert_eq!(registry.counts(), [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
    registry.deactivate("sample:panel-shell").unwrap();
    assert_eq!(
        registry
            .describe_panel_shell("sample:panel-shell", &panel)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );

    let mut duplicate_actions = spec.clone();
    duplicate_actions
        .actions
        .push(duplicate_actions.actions[0].clone());
    let mut invalid = ExtensionRegistry::default();
    invalid
        .register_panel_shell_provider(PanelShellProviderExtension {
            descriptor: extension_descriptor("sample:bad-shell", None, None),
            describe_shell: Arc::new(move |_| Ok(duplicate_actions.clone())),
        })
        .unwrap();
    invalid.freeze();
    invalid.activate("sample:bad-shell").unwrap();
    assert_eq!(
        invalid
            .describe_panel_shell("sample:bad-shell", &panel)
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );

    let mut mismatched_spec = spec;
    mismatched_spec.panel_id = PanelId("other-panel".into());
    let mut mismatched = ExtensionRegistry::default();
    mismatched
        .register_panel_shell_provider(PanelShellProviderExtension {
            descriptor: extension_descriptor("sample:mismatched-shell", None, None),
            describe_shell: Arc::new(move |_| Ok(mismatched_spec.clone())),
        })
        .unwrap();
    mismatched.freeze();
    mismatched.activate("sample:mismatched-shell").unwrap();
    assert_eq!(
        mismatched
            .describe_panel_shell("sample:mismatched-shell", &panel)
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );
}

#[test]
fn input_providers_return_revision_safe_canonical_intents() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let provider_calls = calls.clone();
    let mut registry = ExtensionRegistry::default();
    registry
        .register_input_provider(InputProviderExtension {
            descriptor: extension_descriptor("sample:input", None, None),
            route_input: Arc::new(move |_, _| {
                provider_calls.fetch_add(1, Ordering::SeqCst);
                Ok(InputIntent {
                    consumed: true,
                    focus_target: Some("tabs-ab".into()),
                    operations: vec![Operation::SelectTab {
                        tabs: "tabs-ab".into(),
                        index: 1,
                    }],
                })
            }),
        })
        .unwrap();
    let document = fixture();
    let event = InputEvent {
        device: InputDevice::Keyboard,
        phase: InputPhase::End,
        target_id: Some("tabs-ab".into()),
        position: None,
        key: Some("ArrowRight".into()),
        modifiers: vec!["shift".into()],
    };
    let entry = registry
        .capability_snapshot(&HostCapabilities::default())
        .into_iter()
        .find(|item| item.id == "sample:input")
        .unwrap();
    assert_eq!(entry.kind, ExtensionKind::InputProvider);
    assert_eq!(entry.provided_capabilities, vec!["input_intent"]);
    assert_eq!(
        registry
            .route_input("sample:input", &document, &event)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    registry.freeze();
    registry.activate("sample:input").unwrap();
    let intent = registry
        .route_input("sample:input", &document, &event)
        .unwrap();
    assert!(intent.consumed);
    assert_eq!(intent.focus_target.as_deref(), Some("tabs-ab"));
    assert_eq!(document.revision, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let invalid_event = InputEvent {
        position: Some([f64::NAN, 0.0]),
        ..event.clone()
    };
    assert_eq!(
        registry
            .route_input("sample:input", &document, &invalid_event)
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    registry.deactivate("sample:input").unwrap();
    assert_eq!(
        registry
            .route_input("sample:input", &document, &event)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let mut invalid = ExtensionRegistry::default();
    invalid
        .register_input_provider(InputProviderExtension {
            descriptor: extension_descriptor("sample:invalid-input", None, None),
            route_input: Arc::new(|_, _| {
                Ok(InputIntent {
                    consumed: true,
                    focus_target: None,
                    operations: vec![Operation::ResizeSplit {
                        split: "workspace-root".into(),
                        weights: vec![0.0, 0.0],
                    }],
                })
            }),
        })
        .unwrap();
    invalid.freeze();
    invalid.activate("sample:invalid-input").unwrap();
    assert!(
        invalid
            .route_input("sample:invalid-input", &document, &event)
            .is_err()
    );
    assert_eq!(document.revision, 0);
}

#[test]
fn accessibility_providers_return_bounded_semantic_trees() {
    let tree = AccessibilityTree {
        root: AccessibilityNodeSpec {
            id: "workspace".into(),
            role: AccessibilityRole::Window,
            label: "Workspace".into(),
            description: "Docking workspace".into(),
            enabled: true,
            selected: false,
            actions: vec![],
            children: vec![AccessibilityNodeSpec {
                id: "close-button".into(),
                role: AccessibilityRole::Button,
                label: "Close panel".into(),
                description: "Moves the panel to the closed set".into(),
                enabled: true,
                selected: false,
                actions: vec!["close".into()],
                children: vec![],
            }],
        },
    };
    let document = fixture();
    let mut registry = ExtensionRegistry::default();
    registry
        .register_accessibility_provider(AccessibilityProviderExtension {
            descriptor: extension_descriptor("sample:accessibility", None, None),
            describe_tree: Arc::new({
                let tree = tree.clone();
                move |_| Ok(tree.clone())
            }),
        })
        .unwrap();
    let entry = registry
        .capability_snapshot(&HostCapabilities::default())
        .into_iter()
        .find(|item| item.id == "sample:accessibility")
        .unwrap();
    assert_eq!(entry.kind, ExtensionKind::AccessibilityProvider);
    assert_eq!(entry.provided_capabilities, vec!["accessibility_tree"]);
    assert_eq!(
        registry
            .accessibility_tree("sample:accessibility", &document)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    registry.freeze();
    registry.activate("sample:accessibility").unwrap();
    assert_eq!(
        registry
            .accessibility_tree("sample:accessibility", &document)
            .unwrap(),
        tree
    );
    registry.deactivate("sample:accessibility").unwrap();
    assert_eq!(
        registry
            .accessibility_tree("sample:accessibility", &document)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );

    let mut duplicate_ids = tree.clone();
    duplicate_ids.root.children[0].id = "workspace".into();
    let mut invalid = ExtensionRegistry::default();
    invalid
        .register_accessibility_provider(AccessibilityProviderExtension {
            descriptor: extension_descriptor("sample:bad-accessibility", None, None),
            describe_tree: Arc::new(move |_| Ok(duplicate_ids.clone())),
        })
        .unwrap();
    invalid.freeze();
    invalid.activate("sample:bad-accessibility").unwrap();
    assert_eq!(
        invalid
            .accessibility_tree("sample:bad-accessibility", &document)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );

    let mut panicking = ExtensionRegistry::default();
    panicking
        .register_accessibility_provider(AccessibilityProviderExtension {
            descriptor: extension_descriptor("sample:panicking-accessibility", None, None),
            describe_tree: Arc::new(|_| panic!("injected accessibility provider panic")),
        })
        .unwrap();
    panicking.freeze();
    panicking
        .activate("sample:panicking-accessibility")
        .unwrap();
    assert_eq!(
        panicking
            .accessibility_tree("sample:panicking-accessibility", &document)
            .unwrap_err()
            .code,
        ErrorCode::ExtensionError
    );
}

#[test]
fn managed_layout_providers_fail_closed_on_invalid_placements_and_panics() {
    struct InvalidLayout;
    impl ExtensionLayoutProvider for InvalidLayout {
        fn modality(&self) -> &str {
            "sample.invalid_layout"
        }
        fn arrange(
            &self,
            _request: &ExtensionLayoutRequest<'_>,
        ) -> DockResult<Vec<ExtensionPlacement>> {
            Ok(vec![ExtensionPlacement {
                child_index: 99,
                bounds: Rect {
                    x: 0,
                    y: 0,
                    width: 10,
                    height: 10,
                },
            }])
        }
    }
    struct PanickingLayout;
    impl ExtensionLayoutProvider for PanickingLayout {
        fn modality(&self) -> &str {
            "sample.panicking_layout"
        }
        fn arrange(
            &self,
            _request: &ExtensionLayoutRequest<'_>,
        ) -> DockResult<Vec<ExtensionPlacement>> {
            panic!("injected layout provider panic")
        }
    }

    let mut registry = ExtensionRegistry::default();
    for (id, provider) in [
        (
            "sample:invalid-layout",
            Arc::new(InvalidLayout) as Arc<dyn ExtensionLayoutProvider>,
        ),
        (
            "sample:panicking-layout",
            Arc::new(PanickingLayout) as Arc<dyn ExtensionLayoutProvider>,
        ),
    ] {
        registry
            .register_layout_provider(LayoutProviderExtension {
                descriptor: extension_descriptor(id, None, None),
                provider,
            })
            .unwrap();
    }
    registry.freeze();
    registry.activate("sample:invalid-layout").unwrap();
    registry.activate("sample:panicking-layout").unwrap();
    let child_ids = [NodeId::from("child")];
    let children = [ExtensionChildMetrics::default()];
    let request = |modality: &'static str| ExtensionLayoutRequest {
        modality,
        payload: "{}",
        bounds: Rect {
            x: 0,
            y: 0,
            width: 50,
            height: 50,
        },
        child_ids: &child_ids,
        children: &children,
    };
    assert_eq!(
        registry
            .arrange_layout(&request("sample.invalid_layout"))
            .unwrap_err()
            .code,
        ErrorCode::InvalidLayout
    );
    assert_eq!(
        registry
            .arrange_layout(&request("sample.panicking_layout"))
            .unwrap_err()
            .code,
        ErrorCode::ExtensionError
    );
}

#[test]
fn extension_registry_validates_schemas_capabilities_lifecycle_and_callbacks() {
    use std::sync::Mutex;
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ExtensionRegistry::default();
    for id in ["sample:first", "sample:second"] {
        let started = events.clone();
        let stopped = events.clone();
        let descriptor = extension_descriptor(
            id,
            Some(Arc::new(move || {
                started.lock().unwrap().push(format!("start:{id}"));
                Ok(())
            })),
            Some(Arc::new(move || {
                stopped.lock().unwrap().push(format!("stop:{id}"))
            })),
        );
        registry
            .register_panel_provider(PanelProviderExtension {
                descriptor,
                validate_panel: Arc::new(|_| Ok(())),
            })
            .unwrap();
    }
    assert!(
        registry
            .register_transition(TransitionExtension {
                descriptor: extension_descriptor("sample:first", None, None),
                duration: Arc::new(|r| r.duration_ms),
                required_capabilities: vec![],
                incompatible_capabilities: vec![],
            })
            .is_err()
    );
    let schema = serde_json::json!({"type":"object","required":["edge"],"additionalProperties":false,"properties":{"edge":{"type":"string","enum":["left","right"]}}});
    let modality = |id: &str,
                    capabilities: Vec<&str>,
                    required: Vec<&str>,
                    incompatible: Vec<&str>| ModalityExtension {
        descriptor: extension_descriptor(id, None, None),
        validate_config: Arc::new(|_| Ok(())),
        operations: vec!["split".into(), "tab".into()],
        capabilities: capabilities.into_iter().map(str::to_owned).collect(),
        fallback: ExtensionFallbackPolicy::PreserveOpaquePayload,
        configuration_schema: schema.clone(),
        required_capabilities: required.into_iter().map(str::to_owned).collect(),
        incompatible_capabilities: incompatible.into_iter().map(str::to_owned).collect(),
    };
    registry
        .register_modality(modality(
            "sample:drawer",
            vec!["drawer"],
            vec!["split"],
            vec!["native"],
        ))
        .unwrap();
    registry
        .register_modality(modality("sample:split", vec!["split"], vec![], vec![]))
        .unwrap();
    registry
        .register_modality(modality("sample:native", vec!["native"], vec![], vec![]))
        .unwrap();
    assert!(
        registry
            .modality_supports_operation(&ModalityId::from("sample:drawer"), "split")
            .unwrap()
    );
    assert!(
        registry
            .validate_modality_combination(&["sample:drawer".into(), "sample:split".into()])
            .is_ok()
    );
    assert!(
        registry
            .validate_modality_combination(&["sample:drawer".into()])
            .is_err()
    );
    assert!(
        registry
            .validate_modality_combination(&[
                "sample:drawer".into(),
                "sample:split".into(),
                "sample:native".into()
            ])
            .is_err()
    );

    registry
        .register_transition(TransitionExtension {
            descriptor: extension_descriptor("sample:fade", None, None),
            duration: Arc::new(|r| r.duration_ms + 100),
            required_capabilities: vec![],
            incompatible_capabilities: vec![],
        })
        .unwrap();
    assert_eq!(
        registry
            .transition_duration(
                &TransitionId::from("missing:fade"),
                &TransitionRequest {
                    id: "missing:fade".into(),
                    duration_ms: 500,
                    reduced_motion: true,
                    host_capabilities: vec![],
                },
                300
            )
            .unwrap(),
        0
    );

    let doc = fixture();
    registry
        .register_command(CommandExtension {
            descriptor: extension_descriptor("sample:select", None, None),
            execute: Arc::new(|doc, args| {
                apply_operation(
                    doc,
                    Operation::SelectTab {
                        tabs: "tabs-ab".into(),
                        index: args["index"].as_u64().unwrap() as usize,
                    },
                )
            }),
        })
        .unwrap();
    registry
        .register_command(CommandExtension {
            descriptor: extension_descriptor("sample:bad-diff", None, None),
            execute: Arc::new(|doc, _| {
                let mut tx = apply_operation(
                    doc,
                    Operation::SelectTab {
                        tabs: "tabs-ab".into(),
                        index: 1,
                    },
                )?;
                tx.diff.patches.clear();
                Ok(tx)
            }),
        })
        .unwrap();
    registry
        .register_persistence_codec(PersistenceCodecExtension {
            descriptor: extension_descriptor("sample:codec", None, None),
            encode: Arc::new(|v| Ok(serde_json::json!({"wrapped":v}))),
            decode: Arc::new(|v| Ok(v["wrapped"].clone())),
        })
        .unwrap();
    registry
        .register_host_adapter(HostAdapterExtension {
            descriptor: extension_descriptor("sample:host", None, None),
            validate_diff: Arc::new(|diff| {
                if diff.is_empty() {
                    Ok(())
                } else {
                    Err(DockError::new(
                        ErrorCode::Conflict,
                        "unexpected patches",
                        "host",
                    ))
                }
            }),
        })
        .unwrap();
    assert_eq!(registry.counts(), [2, 3, 1, 1, 2, 1, 0, 0, 0, 0, 0, 0]);

    registry.freeze();
    for id in [
        "sample:first",
        "sample:second",
        "sample:drawer",
        "sample:split",
        "sample:native",
        "sample:fade",
        "sample:select",
        "sample:bad-diff",
        "sample:codec",
        "sample:host",
    ] {
        registry.activate(id).unwrap();
    }
    assert!(registry.active("sample:first"));
    assert!(
        registry
            .validate_modality_config(
                &ModalityId::from("sample:drawer"),
                &serde_json::json!({"edge":"left"})
            )
            .is_ok()
    );
    assert!(
        registry
            .validate_modality_config(
                &ModalityId::from("sample:drawer"),
                &serde_json::json!({"edge":"bottom"})
            )
            .is_err()
    );
    assert!(
        registry
            .validate_modality_config(&ModalityId::from("sample:drawer"), &serde_json::json!({}))
            .is_err()
    );
    assert_eq!(
        registry
            .transition_duration(
                &TransitionId::from("sample:fade"),
                &TransitionRequest {
                    id: "sample:fade".into(),
                    duration_ms: 500,
                    reduced_motion: false,
                    host_capabilities: vec![]
                },
                300,
            )
            .unwrap(),
        300
    );
    assert!(
        registry
            .execute_command("sample:select", &doc, &serde_json::json!({"index":1}))
            .is_ok()
    );
    assert_eq!(
        registry
            .execute_command("sample:bad-diff", &doc, &serde_json::json!({}))
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let packed = registry
        .encode_extension_state("sample:codec", &serde_json::json!(7))
        .unwrap();
    assert_eq!(
        registry
            .decode_extension_state("sample:codec", &packed)
            .unwrap(),
        serde_json::json!(7)
    );
    assert!(
        registry
            .validate_host_diff("sample:host", &LayoutDiff::default())
            .is_ok()
    );
    assert!(
        registry
            .validate_panel("sample:first", &doc.panels[0])
            .is_ok()
    );
    assert!(
        registry
            .register_transition(TransitionExtension {
                descriptor: extension_descriptor("sample:late", None, None),
                duration: Arc::new(|_| 0),
                required_capabilities: vec![],
                incompatible_capabilities: vec![],
            })
            .is_err()
    );
    registry.shutdown().unwrap();
    assert!(!registry.active("sample:first"));
    assert_eq!(
        registry
            .validate_panel("sample:first", &doc.panels[0])
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        *events.lock().unwrap(),
        vec![
            "start:sample:first",
            "start:sample:second",
            "stop:sample:second",
            "stop:sample:first"
        ]
    );
}

#[test]
fn versioned_persistence_codec_migrates_only_covered_versions_after_activation() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let callbacks = Arc::new(AtomicUsize::new(0));
    let first_calls = callbacks.clone();
    let second_calls = callbacks.clone();
    let mut registry = ExtensionRegistry::default();
    registry
        .register_versioned_persistence_codec(VersionedPersistenceCodecExtension {
            codec: PersistenceCodecExtension {
                descriptor: extension_descriptor("sample:versioned-codec", None, None),
                encode: Arc::new(|state| Ok(state.clone())),
                decode: Arc::new(|state| Ok(state.clone())),
            },
            current_schema_version: 2,
            migrations: vec![
                PersistenceCodecMigration {
                    from_schema_version: 0,
                    to_schema_version: 1,
                    migrate: Arc::new(move |state| {
                        first_calls.fetch_add(1, Ordering::SeqCst);
                        let mut value = state.clone();
                        value["adapter_v1"] = serde_json::json!(true);
                        Ok(value)
                    }),
                },
                PersistenceCodecMigration {
                    from_schema_version: 1,
                    to_schema_version: 2,
                    migrate: Arc::new(move |state| {
                        second_calls.fetch_add(1, Ordering::SeqCst);
                        let mut value = state.clone();
                        value["adapter_v2"] = serde_json::json!(true);
                        Ok(value)
                    }),
                },
            ],
        })
        .unwrap();
    assert_eq!(
        registry.persistence_codec_schema_version("sample:versioned-codec"),
        Some(2)
    );

    let source = serde_json::json!({"custom":"keep-this"});
    assert_eq!(
        registry
            .migrate_persistence_codec_state("sample:versioned-codec", 0, &source)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);

    registry.freeze();
    registry.activate("sample:versioned-codec").unwrap();
    let migrated = registry
        .migrate_persistence_codec_state("sample:versioned-codec", 0, &source)
        .unwrap();
    assert_eq!(migrated["custom"], "keep-this");
    assert_eq!(migrated["adapter_v1"], true);
    assert_eq!(migrated["adapter_v2"], true);
    assert_eq!(source, serde_json::json!({"custom":"keep-this"}));
    assert_eq!(callbacks.load(Ordering::SeqCst), 2);

    let current = serde_json::json!({"already":"current"});
    assert_eq!(
        registry
            .migrate_persistence_codec_state("sample:versioned-codec", 2, &current)
            .unwrap(),
        current
    );
    let future = registry
        .migrate_persistence_codec_state("sample:versioned-codec", 3, &source)
        .unwrap_err();
    assert_eq!(future.code, ErrorCode::SchemaError);
    assert_eq!(callbacks.load(Ordering::SeqCst), 2);
    assert!(
        registry
            .capability_snapshot(&HostCapabilities::default())
            .iter()
            .find(|capability| capability.id == "sample:versioned-codec")
            .unwrap()
            .operations
            .contains(&"migrate".to_owned())
    );
}

#[test]
fn extension_state_migration_rechecks_envelope_and_requires_registry() {
    let mut registry = ExtensionRegistry::default();
    registry
        .register_versioned_persistence_codec(VersionedPersistenceCodecExtension {
            codec: PersistenceCodecExtension {
                descriptor: extension_descriptor("sample:state", None, None),
                encode: Arc::new(|state| Ok(state.clone())),
                decode: Arc::new(|state| Ok(state.clone())),
            },
            current_schema_version: 2,
            migrations: vec![PersistenceCodecMigration {
                from_schema_version: 1,
                to_schema_version: 2,
                migrate: Arc::new(|state| {
                    let mut target = state.clone();
                    target["migrated"] = serde_json::json!(true);
                    Ok(target)
                }),
            }],
        })
        .unwrap();
    registry.freeze();
    registry.activate("sample:state").unwrap();
    let mut envelope = serde_json::json!({
        "schema_version": 1,
        "workspace_id": "extension-test",
        "preferences": {
            "extensions": { "sample:state": { "schema_version": 1, "data": { "keep": 7 } } },
            "host.preference": "preserve"
        }
    });
    let sum = checksum_layout_json(&serde_json::to_string(&envelope).unwrap()).unwrap();
    envelope["checksum"] = serde_json::json!(sum);
    let source = serde_json::to_string(&envelope).unwrap();
    assert_eq!(
        preview_persisted_migration(PersistedDataFormat::ExtensionState, &source)
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedOperation
    );
    let preview = preview_extension_state_migration(&registry, &source).unwrap();
    assert!(preview.requires_migration);
    assert_eq!(preview.extension_migrations[0].extension_id, "sample:state");
    let target: serde_json::Value = serde_json::from_str(&preview.target_json).unwrap();
    let target_checksum = checksum_layout_json(&preview.target_json).unwrap();
    assert_eq!(target["checksum"], target_checksum);
    assert_eq!(
        target["preferences"]["extensions"]["sample:state"]["data"]["keep"],
        7
    );
    assert_eq!(
        target["preferences"]["extensions"]["sample:state"]["data"]["migrated"],
        true
    );
    assert_eq!(target["preferences"]["host.preference"], "preserve");

    let mut corrupt = envelope.clone();
    corrupt["checksum"] = serde_json::json!("fnv1a64:0000000000000000");
    assert_eq!(
        preview_extension_state_migration(&registry, &corrupt.to_string())
            .unwrap_err()
            .code,
        ErrorCode::SerializationError
    );
    let mut future = envelope.clone();
    future["preferences"]["extensions"]["sample:state"]["schema_version"] = serde_json::json!(3);
    let checksum = checksum_layout_json(&future.to_string()).unwrap();
    future["checksum"] = serde_json::json!(checksum);
    assert_eq!(
        preview_extension_state_migration(&registry, &future.to_string())
            .unwrap_err()
            .code,
        ErrorCode::SchemaError
    );

    envelope["preferences"]["extensions"]["unknown:provider"] = serde_json::json!({
        "schema_version": 1, "data": {}
    });
    let sum = checksum_layout_json(&serde_json::to_string(&envelope).unwrap()).unwrap();
    envelope["checksum"] = serde_json::json!(sum);
    let bad = serde_json::to_string(&envelope).unwrap();
    assert_eq!(
        preview_extension_state_migration(&registry, &bad)
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedOperation
    );
}

#[test]
fn versioned_persistence_codec_rejects_gapped_chains_before_registration() {
    let mut registry = ExtensionRegistry::default();
    let result =
        registry.register_versioned_persistence_codec(VersionedPersistenceCodecExtension {
            codec: PersistenceCodecExtension {
                descriptor: extension_descriptor("sample:gapped-codec", None, None),
                encode: Arc::new(|state| Ok(state.clone())),
                decode: Arc::new(|state| Ok(state.clone())),
            },
            current_schema_version: 3,
            migrations: vec![
                PersistenceCodecMigration {
                    from_schema_version: 0,
                    to_schema_version: 1,
                    migrate: Arc::new(|state| Ok(state.clone())),
                },
                PersistenceCodecMigration {
                    from_schema_version: 2,
                    to_schema_version: 3,
                    migrate: Arc::new(|state| Ok(state.clone())),
                },
            ],
        });
    assert_eq!(result.unwrap_err().code, ErrorCode::SchemaError);
    assert_eq!(registry.counts()[3], 0);
    assert_eq!(
        registry.persistence_codec_schema_version("sample:gapped-codec"),
        None
    );
}

#[test]
fn versioned_persistence_codec_contains_migration_errors_and_panics() {
    let mut registry = ExtensionRegistry::default();
    for (id, migrate) in [
        (
            "sample:rejecting-codec",
            Arc::new(|_: &serde_json::Value| {
                Err(DockError::new(
                    ErrorCode::Conflict,
                    "legacy setting cannot be converted",
                    "provider-data",
                ))
            }) as CodecMigrationHook,
        ),
        (
            "sample:panicking-codec",
            Arc::new(|_: &serde_json::Value| -> DockResult<serde_json::Value> {
                panic!("migration panic")
            }) as CodecMigrationHook,
        ),
    ] {
        registry
            .register_versioned_persistence_codec(VersionedPersistenceCodecExtension {
                codec: PersistenceCodecExtension {
                    descriptor: extension_descriptor(id, None, None),
                    encode: Arc::new(|state| Ok(state.clone())),
                    decode: Arc::new(|state| Ok(state.clone())),
                },
                current_schema_version: 1,
                migrations: vec![PersistenceCodecMigration {
                    from_schema_version: 0,
                    to_schema_version: 1,
                    migrate,
                }],
            })
            .unwrap();
    }
    registry.freeze();
    registry.activate("sample:rejecting-codec").unwrap();
    registry.activate("sample:panicking-codec").unwrap();
    let source = serde_json::json!({"custom":"preserve-on-failure"});
    for id in ["sample:rejecting-codec", "sample:panicking-codec"] {
        assert_eq!(
            registry
                .migrate_persistence_codec_state(id, 0, &source)
                .unwrap_err()
                .code,
            ErrorCode::ExtensionError
        );
        assert_eq!(source["custom"], "preserve-on-failure");
    }
}

#[test]
fn extension_panics_are_contained_as_typed_errors() {
    let mut registry = ExtensionRegistry::default();
    registry
        .register_panel_provider(PanelProviderExtension {
            descriptor: extension_descriptor("sample:panic", None, None),
            validate_panel: Arc::new(|_| panic!("provider panic")),
        })
        .unwrap();
    registry.freeze();
    registry.activate("sample:panic").unwrap();
    assert_eq!(
        registry
            .validate_panel("sample:panic", &PanelDescriptor::new("p", "P"))
            .unwrap_err()
            .code,
        ErrorCode::ExtensionError
    );
}

#[test]
fn audit_stream_redacts_bounds_retention_and_writes_json_lines() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let errors = Arc::new(AtomicUsize::new(0));
    let sink_errors = errors.clone();
    let path = std::env::temp_dir().join(format!("doofusdock-audit-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let options = AuditOptions {
        max_retained_records: 1,
        max_record_bytes: 16_384,
        max_diagnostics_bytes: 16_384,
        include_actor: false,
        include_reason: false,
        error_sink: Some(Arc::new(move |_| {
            sink_errors.fetch_add(1, Ordering::Relaxed);
        })),
    };
    let mut audit = AuditStream::new(&path, options);
    let doc = fixture();
    let transaction = apply_operation(
        &doc,
        Operation::SelectTab {
            tabs: "tabs-ab".into(),
            index: 1,
        },
    )
    .unwrap();
    audit
        .append(&doc, Ok(transaction), "private-user", "secret reason")
        .unwrap();
    let failure = DockError::new(ErrorCode::NotFound, "private detail", "panel");
    audit
        .append_failure(
            &doc,
            &Operation::ClosePanel {
                panel: "absent".into(),
            },
            &failure,
            "private-user",
        )
        .unwrap();
    assert_eq!(audit.records().len(), 1);
    assert_eq!(audit.records()[0].actor, "[redacted]");
    assert_eq!(audit.records()[0].reason, "[redacted]");
    assert!(!audit.records()[0].succeeded);
    let diagnostics = audit.diagnostics_json().unwrap();
    assert!(
        !diagnostics.contains("private-user")
            && !diagnostics.contains("secret reason")
            && !diagnostics.contains("private detail")
    );
    let contents = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<serde_json::Value> = contents
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].get("operation_id").is_some());
    assert!(lines[0].get("operation").is_none());
    assert_eq!(lines[1]["result_code"], "2");
    let limited_errors = errors.clone();
    let mut limited = AuditStream::new(
        "",
        AuditOptions {
            max_record_bytes: 128,
            error_sink: Some(Arc::new(move |_| {
                limited_errors.fetch_add(1, Ordering::Relaxed);
            })),
            ..AuditOptions::default()
        },
    );
    assert!(
        limited
            .append(
                &doc,
                Ok(apply_operation(
                    &doc,
                    Operation::SelectTab {
                        tabs: "tabs-ab".into(),
                        index: 1
                    }
                )
                .unwrap()),
                "actor",
                "why"
            )
            .is_err()
    );
    assert_eq!(errors.load(Ordering::Relaxed), 1);
    let diagnostics_errors = errors.clone();
    let mut diagnostics_limited = AuditStream::new(
        "",
        AuditOptions {
            max_record_bytes: 16_384,
            max_diagnostics_bytes: 2,
            error_sink: Some(Arc::new(move |_| {
                diagnostics_errors.fetch_add(1, Ordering::Relaxed);
            })),
            ..AuditOptions::default()
        },
    );
    diagnostics_limited
        .append(
            &doc,
            Ok(apply_operation(
                &doc,
                Operation::SelectTab {
                    tabs: "tabs-ab".into(),
                    index: 1,
                },
            )
            .unwrap()),
            "actor",
            "reason",
        )
        .unwrap();
    assert!(diagnostics_limited.diagnostics_json().is_err());
    assert_eq!(errors.load(Ordering::Relaxed), 2);
    std::fs::remove_file(path).unwrap();
}
