use doofusdock_core::*;
use std::sync::Arc;

fn fixture() -> LayoutDocument {
    let mut document = LayoutDocument::new("native-layout", "native-workspace");
    Arc::make_mut(&mut document.panels).extend([
        PanelDescriptor::new("home", "Home"),
        PanelDescriptor::new("one", "One"),
        PanelDescriptor::new("two", "Two"),
    ]);
    document.root = Some(Node::panel("home-node", "home"));
    document.floating_windows.push(FloatingWindow {
        id: "window-one".into(),
        geometry: Rect {
            x: 30,
            y: 40,
            width: 480,
            height: 320,
        },
        monitor_id: "primary".into(),
        scale: 1.0,
        z_order_hint: 12,
        picture_in_picture: false,
        scratchpad: None,
        root: Node::tabs(
            "floating-tabs",
            vec![
                Node::panel("one-node", "one"),
                Node::panel("two-node", "two"),
            ],
            0,
        ),
    });
    document.floating_windows.push(FloatingWindow {
        id: "window-two".into(),
        geometry: Rect {
            x: 90,
            y: 80,
            width: 360,
            height: 240,
        },
        monitor_id: "primary".into(),
        scale: 1.0,
        z_order_hint: 2,
        picture_in_picture: false,
        scratchpad: None,
        root: Node::panel("other-node", "home"),
    });
    // A panel cannot appear in both the root and a floating window; keep the
    // second window as a distinct registered panel for z-order coverage.
    Arc::make_mut(&mut document.panels).push(PanelDescriptor::new("other", "Other"));
    document.floating_windows[1].root = Node::panel("other-node", "other");
    document
}

#[test]
fn center_panel_can_target_a_detached_window_and_remains_one_undoable_revision() {
    let mut document = fixture();
    validate_layout(&document).unwrap();
    let original = document.clone();
    let mut history = UndoStack::new(4);

    let result = history
        .execute(
            &mut document,
            Operation::CenterPanel {
                panel: "home".into(),
                target_panel: "one".into(),
            },
        )
        .unwrap();

    assert_eq!(result.document.revision, original.revision + 1);
    assert!(document.root.is_none());
    let floating_root = &document.floating_windows[0].root;
    assert!(
        matches!(&floating_root.kind, NodeKind::Tabs { items, active_index, .. }
        if *active_index == 1
            && items.iter().map(|item| item.id.0.as_str()).collect::<Vec<_>>()
                == ["one-node", "home-node", "two-node"])
    );
    validate_layout(&document).unwrap();

    history.undo(&mut document).unwrap();
    assert_eq!(document.root, original.root);
    assert_eq!(document.floating_windows, original.floating_windows);
    assert_eq!(document.revision, original.revision + 2);
    history.redo(&mut document).unwrap();
    assert!(document.root.is_none());
    assert!(find_panel_leaf(&document.floating_windows[0].root, &PanelId::from("home")).is_some());
    assert_eq!(document.revision, original.revision + 3);
    validate_layout(&document).unwrap();
}

#[test]
fn center_panel_group_can_target_a_detached_window() {
    let mut document = fixture();
    Arc::make_mut(&mut document.panels).push(PanelDescriptor::new("source-two", "Source two"));
    document.root = Some(Node::tabs(
        "root-source-tabs",
        vec![
            Node::panel("home-node", "home"),
            Node::panel("source-two-node", "source-two"),
        ],
        0,
    ));
    validate_layout(&document).unwrap();
    let original = document.clone();
    let mut history = UndoStack::new(4);

    let result = history
        .execute(
            &mut document,
            Operation::CenterPanelGroup {
                panels: vec!["home".into(), "source-two".into()],
                target_panel: "one".into(),
            },
        )
        .unwrap();

    assert_eq!(result.document.revision, original.revision + 1);
    assert!(document.root.is_none());
    assert!(
        matches!(&document.floating_windows[0].root.kind, NodeKind::Tabs { items, active_index, .. }
        if *active_index == 2
            && items.iter().map(|item| item.id.0.as_str()).collect::<Vec<_>>()
                == ["one-node", "home-node", "source-two-node", "two-node"])
    );
    validate_layout(&document).unwrap();

    history.undo(&mut document).unwrap();
    assert_eq!(document.root, original.root);
    assert_eq!(document.floating_windows, original.floating_windows);
    history.redo(&mut document).unwrap();
    assert!(document.root.is_none());
    validate_layout(&document).unwrap();
}

#[test]
fn floating_a_tab_group_is_one_undoable_operation_and_honors_member_policy() {
    let mut document = LayoutDocument::new("float-group", "workspace");
    Arc::make_mut(&mut document.panels).extend([
        PanelDescriptor::new("one", "One"),
        PanelDescriptor::new("two", "Two"),
    ]);
    document.root = Some(Node::tabs(
        "group-tabs",
        vec![
            Node::panel("one-node", "one"),
            Node::panel("two-node", "two"),
        ],
        1,
    ));
    validate_layout(&document).unwrap();

    let original = document.clone();
    let mut history = UndoStack::new(4);
    let result = history
        .execute(
            &mut document,
            Operation::FloatGroup {
                group: "group-tabs".into(),
                window: "group-window".into(),
                geometry: Rect {
                    x: 40,
                    y: 60,
                    width: 720,
                    height: 480,
                },
                monitor: "display-1".into(),
                scale: 1.25,
            },
        )
        .unwrap();
    assert_eq!(result.document.revision, 1);
    assert!(document.root.is_none());
    assert_eq!(document.floating_windows.len(), 1);
    let group = document.floating_windows[0].root.clone();
    assert_eq!(group.id, NodeId::from("group-tabs"));
    assert!(find_panel_leaf(&group, &PanelId::from("one")).is_some());
    assert!(find_panel_leaf(&group, &PanelId::from("two")).is_some());
    validate_layout(&document).unwrap();

    history.undo(&mut document).unwrap();
    assert_eq!(document.root, original.root);
    assert!(document.floating_windows.is_empty());
    validate_layout(&document).unwrap();
    history.redo(&mut document).unwrap();
    assert_eq!(document.floating_windows[0].root, group);
    validate_layout(&document).unwrap();

    let mut forbidden = original;
    Arc::make_mut(&mut forbidden.panels)
        .iter_mut()
        .find(|panel| panel.id == PanelId::from("two"))
        .unwrap()
        .docking
        .allow_float = false;
    let before = forbidden.clone();
    let error = apply_operation(
        &forbidden,
        Operation::FloatGroup {
            group: "group-tabs".into(),
            window: "rejected-window".into(),
            geometry: Rect {
                x: 0,
                y: 0,
                width: 500,
                height: 400,
            },
            monitor: "display-1".into(),
            scale: 1.0,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedOperation);
    assert_eq!(forbidden, before);
}

#[test]
fn recover_floating_window_is_validated_diffed_and_undoable() {
    let mut document = fixture();
    let original = document.floating_windows[0].clone();
    let mut history = UndoStack::new(4);
    let result = history
        .execute(
            &mut document,
            Operation::RecoverFloatingWindow {
                window: original.id.clone(),
                geometry: Rect {
                    x: 120,
                    y: 210,
                    width: 640,
                    height: 400,
                },
                monitor_id: "portrait-display".into(),
                scale: 1.5,
            },
        )
        .unwrap();
    assert_eq!(result.document.revision, 1);
    assert!(result.diff.patches.iter().any(|patch| matches!(
        patch,
        DiffPatch::WindowGeometryUpdated { window, geometry }
            if window == &original.id && geometry.x == 120 && geometry.width == 640
    )));
    assert!(result.diff.patches.iter().any(|patch| matches!(
        patch,
        DiffPatch::WindowMonitorChanged { window, monitor, scale }
            if window == &original.id && monitor == "portrait-display" && *scale == 1.5
    )));
    let recovered = document
        .floating_windows
        .iter()
        .find(|window| window.id == original.id)
        .unwrap();
    assert_eq!(recovered.monitor_id, "portrait-display");
    assert_eq!(recovered.scale, 1.5);
    validate_layout(&document).unwrap();

    history.undo(&mut document).unwrap();
    assert_eq!(
        document
            .floating_windows
            .iter()
            .find(|window| window.id == original.id)
            .unwrap(),
        &original
    );
    history.redo(&mut document).unwrap();
    assert_eq!(document.revision, 3);
    validate_layout(&document).unwrap();

    for (monitor_id, scale) in [("", 1.0), ("display-1", f64::NAN)] {
        let before = document.clone();
        let error = apply_operation(
            &document,
            Operation::RecoverFloatingWindow {
                window: original.id.clone(),
                geometry: Rect {
                    x: 0,
                    y: 0,
                    width: 400,
                    height: 300,
                },
                monitor_id: monitor_id.into(),
                scale,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ConstraintViolation);
        assert_eq!(document, before);
    }
}

#[test]
fn native_window_activation_demotes_previous_topmost_and_reports_order_diff() {
    let document = fixture();
    let result = apply_operation(
        &document,
        Operation::ActivateFloatingWindow {
            window: "window-two".into(),
        },
    )
    .unwrap();
    assert_eq!(result.document.revision, 1);
    let active = result
        .document
        .floating_windows
        .iter()
        .find(|window| window.id.0 == "window-two")
        .unwrap();
    let prior = result
        .document
        .floating_windows
        .iter()
        .find(|window| window.id.0 == "window-one")
        .unwrap();
    assert!(active.z_order_hint > prior.z_order_hint);
    assert!(result.diff.patches.iter().any(|patch| matches!(patch, DiffPatch::WindowZOrderChanged { window, .. } if window.0 == "window-one")));
    assert!(result.diff.patches.iter().any(|patch| matches!(patch, DiffPatch::WindowZOrderChanged { window, .. } if window.0 == "window-two")));
}

#[test]
fn activating_a_native_window_is_undoable_and_redoable() {
    let mut document = fixture();
    let mut history = UndoStack::new(4);
    let original_order = document
        .floating_windows
        .iter()
        .map(|window| (window.id.clone(), window.z_order_hint))
        .collect::<Vec<_>>();

    history
        .execute(
            &mut document,
            Operation::ActivateFloatingWindow {
                window: "window-two".into(),
            },
        )
        .unwrap();
    let activated_order = document
        .floating_windows
        .iter()
        .map(|window| (window.id.clone(), window.z_order_hint))
        .collect::<Vec<_>>();
    assert_ne!(activated_order, original_order);

    history.undo(&mut document).unwrap();
    assert_eq!(
        document
            .floating_windows
            .iter()
            .map(|window| (window.id.clone(), window.z_order_hint))
            .collect::<Vec<_>>(),
        original_order
    );

    history.redo(&mut document).unwrap();
    assert_eq!(
        document
            .floating_windows
            .iter()
            .map(|window| (window.id.clone(), window.z_order_hint))
            .collect::<Vec<_>>(),
        activated_order
    );
}

#[test]
fn picture_in_picture_is_persisted_bounded_single_panel_and_diffed() {
    let mut document = fixture();
    document.floating_windows[0].root = Node::panel("pip-panel-node", "one");
    let enabled = apply_operation(
        &document,
        Operation::SetPictureInPicture {
            window: "window-one".into(),
            enabled: true,
        },
    )
    .unwrap();
    assert!(enabled.document.floating_windows[0].picture_in_picture);
    assert!(enabled.diff.patches.iter().any(|patch| matches!(
        patch,
        DiffPatch::WindowPictureInPictureChanged { window, enabled: true }
            if window.0 == "window-one"
    )));
    validate_layout(&enabled.document).unwrap();
    let serialized = serialize_layout(&enabled.document).unwrap();
    let restored = deserialize_layout(&serialized, PersistenceLimits::default()).unwrap();
    assert!(restored.floating_windows[0].picture_in_picture);

    // Documents written before the optional field was introduced still load
    // with the normal-window default.
    let mut legacy: serde_json::Value =
        serde_json::from_str(&serialize_layout(&document).unwrap()).unwrap();
    for window in legacy["windows"].as_array_mut().unwrap() {
        window.as_object_mut().unwrap().remove("picture_in_picture");
    }
    let restored_legacy =
        deserialize_layout(&legacy.to_string(), PersistenceLimits::default()).unwrap();
    assert!(
        restored_legacy
            .floating_windows
            .iter()
            .all(|window| !window.picture_in_picture)
    );

    // A multi-panel floating group cannot be partially relabeled as PiP.
    let invalid_document = fixture();
    let before = invalid_document.clone();
    let error = apply_operation(
        &invalid_document,
        Operation::SetPictureInPicture {
            window: "window-one".into(),
            enabled: true,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ConstraintViolation);
    assert_eq!(invalid_document, before);
}

#[test]
fn picture_in_picture_toggle_is_undoable_and_redoable() {
    let mut document = fixture();
    document.floating_windows[0].root = Node::panel("pip-panel-node", "one");
    let mut history = UndoStack::new(4);

    history
        .execute(
            &mut document,
            Operation::SetPictureInPicture {
                window: "window-one".into(),
                enabled: true,
            },
        )
        .unwrap();
    assert!(document.floating_windows[0].picture_in_picture);

    history.undo(&mut document).unwrap();
    assert!(!document.floating_windows[0].picture_in_picture);
    validate_layout(&document).unwrap();

    history.redo(&mut document).unwrap();
    assert!(document.floating_windows[0].picture_in_picture);
    validate_layout(&document).unwrap();
}

#[test]
fn docking_a_native_window_is_one_atomic_undoable_operation() {
    let mut document = fixture();
    // Use the additional window as an independent registered native window.
    let mut history = UndoStack::new(8);
    let result = history
        .execute(
            &mut document,
            Operation::DockFloatingWindow {
                window: "window-one".into(),
                target_panel: Some("home".into()),
            },
        )
        .unwrap();
    assert_eq!(result.document.revision, 1);
    assert!(
        result
            .document
            .floating_windows
            .iter()
            .all(|window| window.id.0 != "window-one")
    );
    assert_eq!(result.document.closed_panels.len(), 0);
    assert!(
        find_node(
            result.document.root.as_ref().unwrap(),
            &NodeId("floating-tabs".into())
        )
        .is_some()
    );
    validate_layout(&result.document).unwrap();
    assert_eq!(history.undo_size(), 1);
    history.undo(&mut document).unwrap();
    assert_eq!(document.floating_windows.len(), 2);
    assert_eq!(document.revision, 2);
}

#[test]
fn failed_native_dock_target_does_not_change_window_or_revision() {
    let document = fixture();
    let error = apply_operation(
        &document,
        Operation::DockFloatingWindow {
            window: "window-one".into(),
            target_panel: Some("two".into()),
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(document.revision, 0);
    assert_eq!(document.floating_windows.len(), 2);
}

#[test]
fn closing_a_group_is_atomic_reopenable_and_undoable() {
    let mut document = fixture();
    let mut descriptors = (*document.panels).clone();
    descriptors
        .iter_mut()
        .find(|panel| panel.id.0 == "two")
        .unwrap()
        .close_policy = ClosePolicy::HideOnly;
    document.panels = Arc::new(descriptors);
    let mut history = UndoStack::new(8);
    let result = history
        .execute(
            &mut document,
            Operation::CloseFloatingWindow {
                window: "window-one".into(),
            },
        )
        .unwrap();
    assert_eq!(result.document.revision, 1);
    assert_eq!(
        result.document.closed_panels,
        vec!["one".into(), "two".into()]
    );
    assert_eq!(result.document.floating_windows.len(), 1);
    validate_layout(&result.document).unwrap();

    history.undo(&mut document).unwrap();
    assert_eq!(document.floating_windows.len(), 2);
    assert!(document.closed_panels.is_empty());
    validate_layout(&document).unwrap();
}

#[test]
fn non_closable_member_keeps_the_entire_native_window_intact() {
    let mut document = fixture();
    let descriptors = Arc::make_mut(&mut document.panels);
    descriptors
        .iter_mut()
        .find(|panel| panel.id.0 == "two")
        .unwrap()
        .close_policy = ClosePolicy::Forbidden;
    let before = document.clone();
    let error = apply_operation(
        &document,
        Operation::CloseFloatingWindow {
            window: "window-one".into(),
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedOperation);
    assert_eq!(document, before);
    assert_eq!(document.floating_windows.len(), 2);
    assert!(document.closed_panels.is_empty());
}

#[test]
fn closing_a_panel_group_is_one_atomic_undoable_transaction() {
    let mut document = fixture();
    let before = document.clone();
    let mut history = UndoStack::new(8);
    let result = history
        .execute(
            &mut document,
            Operation::ClosePanelGroup {
                panels: vec!["one".into(), "two".into()],
            },
        )
        .unwrap();
    assert_eq!(result.document.revision, 1);
    assert_eq!(
        result.document.closed_panels,
        vec!["one".into(), "two".into()]
    );
    assert_eq!(history.undo_size(), 1);
    history.undo(&mut document).unwrap();
    let mut expected = before;
    expected.revision = document.revision;
    assert_eq!(document, expected);
}

#[test]
fn forbidden_panel_rejects_group_close_without_partial_mutation() {
    let mut document = fixture();
    Arc::make_mut(&mut document.panels)
        .iter_mut()
        .find(|panel| panel.id.0 == "two")
        .unwrap()
        .close_policy = ClosePolicy::Forbidden;
    let before = document.clone();
    let error = apply_operation(
        &document,
        Operation::ClosePanelGroup {
            panels: vec!["one".into(), "two".into()],
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedOperation);
    assert_eq!(document, before);
}

#[test]
fn removed_monitor_and_mixed_scale_recovery_keep_windows_inside_work_area() {
    let mut document = fixture();
    document.floating_windows[0].monitor_id = "removed-monitor".into();
    document.floating_windows[0].scale = 2.0;
    document.floating_windows[0].geometry = Rect {
        x: -2400,
        y: 80,
        width: 1200,
        height: 800,
    };
    let monitors = [MonitorInfo {
        id: "primary-monitor".into(),
        work_area: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: 1.0,
        primary: true,
    }];
    let operations = plan_floating_window_recovery(&document, &monitors).unwrap();
    assert_eq!(operations.len(), 2);
    let (result, monitor_change_reported) = operations
        .into_iter()
        .try_fold((document, false), |(current, changed), operation| {
            apply_operation(&current, operation).map(|result| {
                let changed = changed
                    || result.diff.patches.iter().any(|patch| {
                        matches!(patch, DiffPatch::WindowMonitorChanged { window, .. } if window.0 == "window-one")
                    });
                (result.document, changed)
            })
        })
        .unwrap();
    let window = result
        .floating_windows
        .iter()
        .find(|window| window.id.0 == "window-one")
        .unwrap();
    assert_eq!(window.monitor_id, "primary-monitor");
    assert_eq!(window.scale, 1.0);
    assert_eq!(
        window.geometry,
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080
        }
    );
    assert!(monitor_change_reported);
}

#[test]
fn floating_window_monitor_recovery_breaks_equal_ties_by_stable_monitor_id() {
    let mut window = fixture().floating_windows[0].clone();
    window.monitor_id = "removed-monitor".into();
    window.geometry = Rect {
        x: 0,
        y: 10,
        width: 100,
        height: 60,
    };
    let alpha = MonitorInfo {
        id: "alpha".into(),
        work_area: Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 100,
        },
        scale: 1.0,
        primary: false,
    };
    let zulu = MonitorInfo {
        id: "zulu".into(),
        work_area: Rect {
            x: 60,
            y: 0,
            width: 40,
            height: 100,
        },
        scale: 1.0,
        primary: false,
    };

    let first = recover_floating_window_geometry(&window, &[zulu.clone(), alpha.clone()]).unwrap();
    let reordered =
        recover_floating_window_geometry(&window, &[alpha.clone(), zulu.clone()]).unwrap();
    assert_eq!(first, reordered);
    assert_eq!(first.1.id, "alpha");

    window.geometry.x = 5_000;
    window.geometry.y = 5_000;
    let fallback_first =
        recover_floating_window_geometry(&window, &[zulu.clone(), alpha.clone()]).unwrap();
    let fallback_reordered = recover_floating_window_geometry(&window, &[alpha, zulu]).unwrap();
    assert_eq!(fallback_first, fallback_reordered);
    assert_eq!(fallback_first.1.id, "alpha");
}

#[test]
fn explicit_monitor_placement_scales_and_clamps_a_floating_window() {
    let document = fixture();
    let target = MonitorInfo {
        id: "portrait-secondary".into(),
        work_area: Rect {
            x: 1920,
            y: 0,
            width: 1080,
            height: 1920,
        },
        scale: 2.0,
        primary: false,
    };
    let (geometry, selected) =
        place_floating_window_on_monitor(&document.floating_windows[0], &target).unwrap();
    assert_eq!(selected, target);
    assert_eq!(
        geometry,
        Rect {
            x: 1920,
            y: 40,
            width: 240,
            height: 160,
        }
    );
}

#[test]
fn moving_a_window_between_monitors_updates_monitor_and_scale_metadata() {
    let mut document = fixture();
    document.floating_windows[0].geometry = Rect {
        x: 2000,
        y: 100,
        width: 600,
        height: 400,
    };
    let monitors = [
        MonitorInfo {
            id: "primary".into(),
            work_area: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale: 1.0,
            primary: true,
        },
        MonitorInfo {
            id: "secondary".into(),
            work_area: Rect {
                x: 1920,
                y: 0,
                width: 1280,
                height: 1024,
            },
            scale: 1.5,
            primary: false,
        },
    ];

    let operations = plan_floating_window_recovery(&document, &monitors).unwrap();
    assert_eq!(operations.len(), 1);
    let mut history = UndoStack::new(4);
    let revision = document.revision;
    let result = DockingService::transact_document(
        &mut document,
        &mut history,
        revision,
        CommandId::from("monitor-scale-change"),
        operations,
    )
    .unwrap();
    let moved = result
        .document
        .floating_windows
        .iter()
        .find(|window| window.id.0 == "window-one")
        .unwrap();
    assert_eq!(moved.monitor_id, "secondary");
    assert_eq!(moved.scale, 1.5);
    assert_eq!(moved.geometry.width, 400);
    assert_eq!(moved.geometry.height, 267);
    assert_eq!(result.document.revision, 1);
}
