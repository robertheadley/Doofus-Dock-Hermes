use doofusdock_core::*;
use std::sync::Arc;

fn fixture() -> LayoutDocument {
    let mut document = LayoutDocument::new("group-minimize", "workspace");
    Arc::make_mut(&mut document.panels).extend([
        PanelDescriptor::new("editor", "Editor"),
        PanelDescriptor::new("files", "Files"),
    ]);
    document.root = Some(Node::split(
        "root",
        Orientation::Horizontal,
        vec![
            Node::tabs("group", vec![Node::panel("editor-node", "editor")], 0),
            Node::panel("files-node", "files"),
        ],
        vec![0.4, 0.6],
    ));
    document
}

#[test]
fn minimized_state_defaults_when_deserializing_pre_feature_nodes() {
    let mut value = serde_json::to_value(Node::tabs(
        "group",
        vec![Node::panel("editor-node", "editor")],
        0,
    ))
    .unwrap();
    value.as_object_mut().unwrap().remove("minimized");
    let restored: Node = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.kind,
        NodeKind::Tabs {
            minimized: false,
            ..
        }
    ));
}

#[test]
fn group_minimize_is_revision_checked_diffed_audited_and_undoable() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    let result = service
        .transact(
            0,
            "minimize-group".into(),
            [Operation::SetGroupMinimized {
                group: "group".into(),
                minimized: true,
            }],
        )
        .unwrap();
    assert_eq!(result.document.revision, 1);
    assert!(matches!(
        result.diff.patches.as_slice(),
        [DiffPatch::TabGroupMinimizedChanged { group, minimized: true }]
            if group == &NodeId::from("group")
    ));
    assert!(matches!(
        result.document.root.as_deref().unwrap().kind,
        NodeKind::Split { ref children, .. }
            if matches!(children[0].kind, NodeKind::Tabs { minimized: true, .. })
    ));
    let events = service.read_events(subscription, 16).unwrap();
    assert!(events.events.iter().any(|event| {
        event.kind == FrameworkEventKind::GroupChanged
            && event
                .payload
                .get("group")
                .is_some_and(|value| value == "group")
            && event
                .payload
                .get("minimized")
                .is_some_and(|value| value == "true")
    }));

    let undone = service.undo(1).unwrap();
    assert!(matches!(
        undone.document.root.as_deref().unwrap().kind,
        NodeKind::Split { ref children, .. }
            if matches!(children[0].kind, NodeKind::Tabs { minimized: false, .. })
    ));
    let redone = service.redo(2).unwrap();
    assert!(matches!(
        redone.document.root.as_deref().unwrap().kind,
        NodeKind::Split { ref children, .. }
            if matches!(children[0].kind, NodeKind::Tabs { minimized: true, .. })
    ));
    service.read_events(subscription, 32).unwrap();

    let no_op = service
        .transact(
            3,
            "minimize-group-again".into(),
            [Operation::SetGroupMinimized {
                group: "group".into(),
                minimized: true,
            }],
        )
        .unwrap();
    assert_eq!(no_op.document.revision, 3);
    assert!(no_op.diff.is_empty());
    assert_eq!(service.snapshot().undo_depth, 1);
    assert!(
        service
            .read_events(subscription, 32)
            .unwrap()
            .events
            .is_empty()
    );

    let stale = service.transact(
        2,
        "stale-minimize".into(),
        [Operation::SetGroupMinimized {
            group: "group".into(),
            minimized: false,
        }],
    );
    assert_eq!(stale.unwrap_err().code, ErrorCode::Conflict);
}

#[test]
fn group_minimize_preflight_and_wire_catalog_are_registered() {
    let document = fixture();
    let operation = Operation::SetGroupMinimized {
        group: "group".into(),
        minimized: true,
    };
    let report = preflight_operation(&document, &operation, &HostCapabilities::default()).unwrap();
    assert!(report.supported);
    assert_eq!(report.operation_id, "set_group_minimized");
    assert_eq!(report.target_ids, vec!["group:group"]);
    let schema = builtin_operation_json_schema();
    assert!(schema["oneOf"].as_array().unwrap().iter().any(|branch| {
        branch["properties"]["operation"]["const"] == "set_group_minimized"
            && branch["properties"]["arguments"]["properties"]["minimized"]["type"] == "boolean"
    }));

    let wrong_kind = preflight_operation(
        &document,
        &Operation::SetGroupMinimized {
            group: "files-node".into(),
            minimized: true,
        },
        &HostCapabilities::default(),
    )
    .unwrap();
    assert!(!wrong_kind.core_available);
    assert_eq!(
        wrong_kind.rejection_code.as_deref(),
        Some("unsupported_operation")
    );
}

#[test]
fn minimized_layout_persists_and_older_layout_round_trips() {
    let minimized = apply_operation(
        &fixture(),
        Operation::SetGroupMinimized {
            group: "group".into(),
            minimized: true,
        },
    )
    .unwrap()
    .document;
    let serialized = serialize_layout(&minimized).unwrap();
    let loaded = deserialize_layout(&serialized, PersistenceLimits::default()).unwrap();
    assert!(matches!(
        loaded.root.as_deref().unwrap().kind,
        NodeKind::Split { ref children, .. }
            if matches!(children[0].kind, NodeKind::Tabs { minimized: true, .. })
    ));
}
