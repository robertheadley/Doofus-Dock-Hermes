use doofusdock_core::*;
use std::collections::BTreeSet;
use std::sync::Arc;

fn fixture() -> LayoutDocument {
    let mut document = LayoutDocument::new("service-layout", "service-workspace");
    Arc::make_mut(&mut document.panels).extend([
        PanelDescriptor::new("alpha", "Alpha"),
        PanelDescriptor::new("beta", "Beta"),
    ]);
    document.root = Some(Node::tabs(
        "tabs",
        vec![
            Node::panel("alpha-node", "alpha"),
            Node::panel("beta-node", "beta"),
        ],
        0,
    ));
    document
}

fn floating_fixture() -> LayoutDocument {
    let mut document = fixture();
    document.root = Some(Node::panel("alpha-node", "alpha"));
    document.floating_windows.push(FloatingWindow {
        id: "beta-window".into(),
        geometry: Rect {
            x: 40,
            y: 50,
            width: 400,
            height: 300,
        },
        monitor_id: "primary".into(),
        scale: 1.0,
        z_order_hint: 0,
        picture_in_picture: false,
        scratchpad: None,
        root: Node::panel("beta-node", "beta"),
    });
    document
}

#[test]
fn facade_batch_is_one_atomic_revision_and_undoes_as_one_command() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let result = service
        .transact(
            0,
            "select-and-close".into(),
            [
                Operation::SelectTab {
                    tabs: "tabs".into(),
                    index: 1,
                },
                Operation::ClosePanel {
                    panel: "alpha".into(),
                },
            ],
        )
        .unwrap();
    assert_eq!(result.operation_id, "select-and-close");
    assert_eq!(result.document.revision, 1);
    assert_eq!(result.document.closed_panels, vec![PanelId::from("alpha")]);
    assert_eq!(service.snapshot().undo_depth, 1);

    let undone = service.undo(1).unwrap();
    assert_eq!(undone.document.revision, 2);
    assert!(undone.document.closed_panels.is_empty());
    assert_eq!(service.snapshot().redo_depth, 1);
}

#[test]
fn service_publishes_immutable_commit_and_restore_events_after_transactions() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    service
        .transact(
            0,
            "close-alpha".into(),
            [Operation::ClosePanel {
                panel: "alpha".into(),
            }],
        )
        .unwrap();
    let committed = service.read_events(subscription, 8).unwrap();
    assert_eq!(committed.lost_events, 0);
    assert!(committed.events.len() >= 2);
    assert_eq!(committed.events[0].revision, 1);
    assert_eq!(
        committed.events[0].kind,
        FrameworkEventKind::LayoutCommitted
    );
    assert_eq!(committed.events[0].command, "close-alpha".into());
    assert!(committed.events.iter().any(|event| {
        event.kind == FrameworkEventKind::PanelVisibilityChanged
            && event
                .payload
                .get("panel")
                .is_some_and(|panel| panel == "alpha")
            && event
                .payload
                .get("visible")
                .is_some_and(|visible| visible == "false")
    }));

    service.undo(1).unwrap();
    let restored = service.read_events(subscription, 8).unwrap();
    assert!(!restored.events.is_empty());
    assert_eq!(restored.events[0].revision, 2);
    assert_eq!(restored.events[0].kind, FrameworkEventKind::LayoutRestored);
    assert_eq!(
        restored.events[0].sequence,
        committed.events.last().unwrap().sequence + 1
    );
}

#[test]
fn service_publishes_panel_creation_and_close_facts_with_visibility_changes() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    service
        .transact(
            0,
            "add-gamma".into(),
            [Operation::AddPanel {
                descriptor: PanelDescriptor::new("gamma", "Gamma"),
                target: Some("tabs".into()),
            }],
        )
        .unwrap();
    let created = service.read_events(subscription, 16).unwrap();
    assert!(created.events.iter().any(|event| {
        event.kind == FrameworkEventKind::PanelCreated
            && event.command == CommandId::from("add-gamma")
            && event
                .payload
                .get("panel")
                .is_some_and(|panel| panel == "gamma")
    }));

    service
        .transact(
            1,
            "close-alpha".into(),
            [Operation::ClosePanel {
                panel: "alpha".into(),
            }],
        )
        .unwrap();
    let closed = service.read_events(subscription, 16).unwrap();
    assert!(closed.events.iter().any(|event| {
        event.kind == FrameworkEventKind::PanelVisibilityChanged
            && event.command == CommandId::from("close-alpha")
            && event
                .payload
                .get("panel")
                .is_some_and(|panel| panel == "alpha")
            && event
                .payload
                .get("visible")
                .is_some_and(|visible| visible == "false")
    }));
    assert!(closed.events.iter().any(|event| {
        event.kind == FrameworkEventKind::PanelClosed
            && event.command == CommandId::from("close-alpha")
            && event
                .payload
                .get("panel")
                .is_some_and(|panel| panel == "alpha")
            && event
                .payload
                .get("visible")
                .is_some_and(|visible| visible == "false")
    }));
}

#[test]
fn host_focus_observations_are_revision_bound_and_must_match_current_placement() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    service
        .transact(
            0,
            "select-beta".into(),
            [Operation::SelectTab {
                tabs: "tabs".into(),
                index: 1,
            }],
        )
        .unwrap();
    let subscription = service.subscribe_events().unwrap();
    let observed = service
        .observe_host_event(
            1,
            "host-focus-beta".into(),
            HostObservation::PanelFocused {
                panel: "beta".into(),
                floating_window: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(observed.kind, FrameworkEventKind::FocusChanged);
    assert_eq!(observed.revision, 1);
    assert_eq!(
        observed.payload.get("panel").map(String::as_str),
        Some("beta")
    );
    assert!(!observed.payload.contains_key("window"));
    assert_eq!(service.snapshot().revision, 1);

    assert!(
        service
            .observe_host_event(
                0,
                "stale-focus".into(),
                HostObservation::PanelFocused {
                    panel: "beta".into(),
                    floating_window: None,
                },
            )
            .is_err()
    );
    assert!(
        service
            .observe_host_event(
                1,
                "wrong-host-focus".into(),
                HostObservation::PanelFocused {
                    panel: "beta".into(),
                    floating_window: Some("missing-window".into()),
                },
            )
            .is_err()
    );
    assert!(
        service
            .observe_host_event(
                1,
                "unknown-panel-focus".into(),
                HostObservation::PanelFocused {
                    panel: "missing-panel".into(),
                    floating_window: None,
                },
            )
            .is_err()
    );
    let events = service.read_events(subscription, 8).unwrap();
    assert_eq!(events.events.len(), 1);
    assert_eq!(events.events[0], observed);
}

#[test]
fn host_focus_observation_accepts_the_panel_current_floating_host() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    service
        .transact(
            0,
            "float-beta".into(),
            [Operation::FloatPanel {
                panel: "beta".into(),
                window: "beta-window".into(),
                geometry: Rect {
                    x: 40,
                    y: 50,
                    width: 400,
                    height: 300,
                },
                monitor: "primary".into(),
                scale: 1.0,
            }],
        )
        .unwrap();
    let subscription = service.subscribe_events().unwrap();
    let observed = service
        .observe_host_event(
            1,
            "native-focus-beta-window".into(),
            HostObservation::PanelFocused {
                panel: "beta".into(),
                floating_window: Some("beta-window".into()),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        observed.payload.get("window").map(String::as_str),
        Some("beta-window")
    );
    assert_eq!(
        service.read_events(subscription, 8).unwrap().events,
        vec![observed]
    );
}

#[test]
fn native_viewport_observations_are_typed_revision_bound_and_non_mutating() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    let root = service
        .observe_host_event(
            0,
            "root-viewport-focused".into(),
            HostObservation::NativeViewportFocused { window: None },
        )
        .unwrap()
        .unwrap();
    assert_eq!(root.kind, FrameworkEventKind::NativeViewportFocused);
    assert_eq!(root.revision, 0);
    assert_eq!(
        root.payload.get("viewport").map(String::as_str),
        Some("root")
    );
    let mut initially_floating = DockingService::new(floating_fixture(), 8).unwrap();
    let initial_close = initially_floating
        .observe_host_event(
            0,
            "initial-native-close-request".into(),
            HostObservation::NativeViewportCloseRequested {
                window: "beta-window".into(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        initial_close.kind,
        FrameworkEventKind::NativeViewportCloseRequested
    );
    assert_eq!(initial_close.revision, 0);
    let initial_geometry = initially_floating
        .observe_host_event(
            0,
            "initial-native-geometry".into(),
            HostObservation::NativeViewportGeometryObserved {
                window: "beta-window".into(),
                geometry: Rect {
                    x: -1280,
                    y: 120,
                    width: 640,
                    height: 480,
                },
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        initial_geometry.kind,
        FrameworkEventKind::NativeViewportGeometryObserved
    );
    assert_eq!(initial_geometry.revision, 0);
    assert_eq!(
        initial_geometry.payload.get("geometry").map(String::as_str),
        Some("-1280,120,640,480")
    );
    assert_eq!(initially_floating.snapshot().revision, 0);

    service
        .transact(
            0,
            "float-beta".into(),
            [Operation::FloatPanel {
                panel: "beta".into(),
                window: "beta-window".into(),
                geometry: Rect {
                    x: 40,
                    y: 50,
                    width: 400,
                    height: 300,
                },
                monitor: "primary".into(),
                scale: 1.0,
            }],
        )
        .unwrap();
    let focused = service
        .observe_host_event(
            1,
            "beta-viewport-focused".into(),
            HostObservation::NativeViewportFocused {
                window: Some("beta-window".into()),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(focused.kind, FrameworkEventKind::NativeViewportFocused);
    assert_eq!(
        focused.payload.get("window").map(String::as_str),
        Some("beta-window")
    );

    let close = service
        .observe_host_event(
            1,
            "beta-viewport-close-requested".into(),
            HostObservation::NativeViewportCloseRequested {
                window: "beta-window".into(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(close.kind, FrameworkEventKind::NativeViewportCloseRequested);
    assert_eq!(close.revision, 1);
    assert_eq!(service.snapshot().revision, 1);

    assert!(
        service
            .observe_host_event(
                1,
                "unknown-viewport-focus".into(),
                HostObservation::NativeViewportFocused {
                    window: Some("missing-window".into()),
                },
            )
            .is_err()
    );
    assert!(
        service
            .observe_host_event(
                0,
                "stale-viewport-close".into(),
                HostObservation::NativeViewportCloseRequested {
                    window: "beta-window".into(),
                },
            )
            .is_err()
    );
    let events = service.read_events(subscription, 16).unwrap();
    assert_eq!(events.lost_events, 0);
    assert!(events.events.contains(&root));
    assert!(events.events.contains(&focused));
    assert!(events.events.contains(&close));
}

#[test]
fn native_root_geometry_observation_is_bounded_revision_bound_and_non_mutating() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    let geometry = Rect {
        x: -1200,
        y: 80,
        width: 1600,
        height: 900,
    };
    let event = service
        .observe_host_event(
            0,
            "root-geometry-initial".into(),
            HostObservation::NativeRootViewportGeometryObserved { geometry },
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        event.kind,
        FrameworkEventKind::NativeRootViewportGeometryObserved
    );
    assert_eq!(event.revision, 0);
    assert_eq!(
        event.payload.get("geometry").map(String::as_str),
        Some("-1200,80,1600,900")
    );
    assert_eq!(service.snapshot().revision, 0);
    assert!(
        service
            .observe_host_event(
                0,
                "invalid-root-geometry".into(),
                HostObservation::NativeRootViewportGeometryObserved {
                    geometry: Rect {
                        width: 0,
                        height: 900,
                        ..geometry
                    },
                },
            )
            .is_err()
    );
    let batch = service.read_events(subscription, 4).unwrap();
    assert_eq!(batch.events, vec![event]);
}

#[test]
fn transition_lifecycle_observations_are_revision_bound_and_non_mutating() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    service
        .transact(
            0,
            "prepare-transition-revision".into(),
            [Operation::ClosePanel {
                panel: "alpha".into(),
            }],
        )
        .unwrap();
    let subscription = service.subscribe_events().unwrap();
    let started = service
        .observe_host_event(
            1,
            "transition-start-a".into(),
            HostObservation::TransitionStarted {
                transition: "profile-morph-1".into(),
            },
        )
        .unwrap()
        .unwrap();
    let completed = service
        .observe_host_event(
            1,
            "transition-complete-a".into(),
            HostObservation::TransitionCompleted {
                transition: "profile-morph-1".into(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(started.kind, FrameworkEventKind::TransitionStarted);
    assert_eq!(completed.kind, FrameworkEventKind::TransitionCompleted);
    assert_eq!(started.revision, 1);
    assert_eq!(completed.revision, 1);
    assert_eq!(
        started.payload.get("transition").map(String::as_str),
        Some("profile-morph-1")
    );
    assert_eq!(completed.payload, started.payload);
    assert_eq!(service.snapshot().revision, 1);
    assert!(
        service
            .observe_host_event(
                0,
                "stale-transition-complete".into(),
                HostObservation::TransitionCompleted {
                    transition: "profile-morph-1".into(),
                },
            )
            .is_err()
    );
    assert!(
        service
            .observe_host_event(
                0,
                "empty-transition-id".into(),
                HostObservation::TransitionStarted {
                    transition: "".into(),
                },
            )
            .is_err()
    );
    assert_eq!(
        service.read_events(subscription, 8).unwrap().events,
        vec![started, completed]
    );
}

#[test]
fn monitor_topology_observations_are_revision_bound_and_non_mutating() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    let removed = service
        .observe_host_event(
            0,
            "display-removed".into(),
            HostObservation::MonitorChanged {
                monitor: "DISPLAY2".into(),
                work_area: Rect {
                    x: 1920,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                scale_milli: 1500,
                primary: false,
                connected: false,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(removed.kind, FrameworkEventKind::MonitorChanged);
    assert_eq!(removed.revision, 0);
    assert_eq!(
        removed.payload.get("monitor").map(String::as_str),
        Some("DISPLAY2")
    );
    assert_eq!(
        removed.payload.get("work_area").map(String::as_str),
        Some("1920,0,1920,1080")
    );
    assert_eq!(
        removed.payload.get("connected").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        removed.payload.get("scale").map(String::as_str),
        Some("1.500")
    );
    assert_eq!(
        removed.payload.get("primary").map(String::as_str),
        Some("false")
    );
    assert_eq!(service.snapshot().revision, 0);

    assert!(
        service
            .observe_host_event(
                1,
                "stale-display-change".into(),
                HostObservation::MonitorChanged {
                    monitor: "DISPLAY2".into(),
                    work_area: Rect {
                        x: 0,
                        y: 0,
                        width: 1920,
                        height: 1080,
                    },
                    scale_milli: 1000,
                    primary: false,
                    connected: true,
                },
            )
            .is_err()
    );
    assert!(
        service
            .observe_host_event(
                0,
                "invalid-display-change".into(),
                HostObservation::MonitorChanged {
                    monitor: "".into(),
                    work_area: Rect {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 1080,
                    },
                    scale_milli: 0,
                    primary: false,
                    connected: true,
                },
            )
            .is_err()
    );
    assert_eq!(
        service.read_events(subscription, 8).unwrap().events,
        vec![removed]
    );
}

#[test]
fn service_event_rule_commit_suppresses_its_own_follow_up_events() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    service
        .transact(
            0,
            "user-select-beta".into(),
            [Operation::SelectTab {
                tabs: "tabs".into(),
                index: 1,
            }],
        )
        .unwrap();
    let initial_events = service.read_events(subscription, 16).unwrap();
    let rule = WindowRule {
        id: "pin-selected-panel-on-commit".into(),
        enabled: true,
        priority: 10,
        overrides: Vec::new(),
        event_kinds: BTreeSet::from([FrameworkEventKind::LayoutCommitted]),
        predicate: WindowRulePredicate {
            panel_id: Some(PanelId::from("beta")),
            ..WindowRulePredicate::default()
        },
        actions: vec![WindowRuleAction::SetPinned { pinned: true }],
    };
    let rules = WindowRuleSet {
        schema_version: 1,
        rules: vec![rule],
    };
    let snapshot = service.snapshot();
    let context = WindowRuleContext::for_panel(
        &snapshot.document,
        &PanelId::from("beta"),
        HostCapabilities {
            supported: BTreeSet::from(["workspace:tabs".into()]),
        },
    )
    .unwrap();
    let mut consumer = WindowRuleEventConsumer::default();
    let triggered = consumer
        .evaluate_batch(&rules, &initial_events, std::slice::from_ref(&context))
        .unwrap();
    assert_eq!(triggered.len(), 1);
    let plan = WindowRulePlan::prepare(triggered[0].evaluation.clone(), &context).unwrap();
    let rule_command = CommandId::from("rule-pin-beta");
    consumer.suppress_command(rule_command.clone()).unwrap();
    plan.commit(&mut service, rule_command).unwrap();
    assert!(
        service
            .snapshot()
            .document
            .pinned_panels
            .contains(&PanelId::from("beta"))
    );

    let rule_events = service.read_events(subscription, 16).unwrap();
    assert!(rule_events.events.iter().any(|event| {
        event.command == CommandId::from("rule-pin-beta")
            && event.kind == FrameworkEventKind::LayoutCommitted
    }));
    let follow_up = consumer.evaluate_batch(&rules, &rule_events, &[]).unwrap();
    assert!(follow_up.is_empty());
    let post_commit = service.snapshot();
    let post_context = WindowRuleContext::for_panel(
        &post_commit.document,
        &PanelId::from("beta"),
        HostCapabilities {
            supported: BTreeSet::from(["workspace:tabs".into()]),
        },
    )
    .unwrap();
    let mut unguarded = WindowRuleEventConsumer::default();
    assert_eq!(
        unguarded
            .evaluate_batch(&rules, &rule_events, &[post_context])
            .unwrap()
            .len(),
        1,
        "the generated commit event would retrigger without command suppression"
    );
}

#[test]
fn failed_batch_is_atomic_and_stale_revision_is_rejected() {
    let mut service = DockingService::new(fixture(), 8).unwrap();
    let subscription = service.subscribe_events().unwrap();
    let error = service
        .transact(
            0,
            "failing".into(),
            [
                Operation::SelectTab {
                    tabs: "tabs".into(),
                    index: 1,
                },
                Operation::ClosePanel {
                    panel: "unknown".into(),
                },
            ],
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    let snapshot = service.snapshot();
    assert_eq!(snapshot.revision, 0);
    assert_eq!(snapshot.undo_depth, 0);
    assert!(
        service
            .read_events(subscription, 8)
            .unwrap()
            .events
            .is_empty()
    );
    assert!(matches!(
        &snapshot.document.root.as_ref().unwrap().kind,
        NodeKind::Tabs {
            active_index: 0,
            ..
        }
    ));

    let conflict = service
        .transact(
            9,
            "stale".into(),
            [Operation::ClosePanel {
                panel: "alpha".into(),
            }],
        )
        .unwrap_err();
    assert_eq!(conflict.code, ErrorCode::Conflict);
    assert_eq!(service.snapshot().revision, 0);
    assert!(
        service
            .read_events(subscription, 8)
            .unwrap()
            .events
            .is_empty()
    );
}

#[test]
fn equivalent_operation_batches_produce_identical_snapshots_and_diffs() {
    let execute = || {
        let mut service = DockingService::new(fixture(), 8).unwrap();
        service
            .transact(
                0,
                "deterministic-batch".into(),
                [
                    Operation::SelectTab {
                        tabs: "tabs".into(),
                        index: 1,
                    },
                    Operation::ClosePanel {
                        panel: "alpha".into(),
                    },
                ],
            )
            .unwrap()
    };

    let first = execute();
    let second = execute();
    assert_eq!(first, second);
}

#[test]
fn application_owned_state_uses_the_same_revision_checked_facade() {
    let mut document = fixture();
    let mut history = UndoStack::new(8);
    let result = DockingService::transact_document(
        &mut document,
        &mut history,
        0,
        "application-command".into(),
        [Operation::ClosePanel {
            panel: "alpha".into(),
        }],
    )
    .unwrap();
    assert_eq!(result.document.revision, 1);
    assert_eq!(document, result.document);

    let stale = DockingService::transact_document(
        &mut document,
        &mut history,
        0,
        "stale-application-command".into(),
        [Operation::ClosePanel {
            panel: "beta".into(),
        }],
    )
    .unwrap_err();
    assert_eq!(stale.code, ErrorCode::Conflict);
    assert_eq!(document.revision, 1);
    assert_eq!(history.undo_size(), 1);
}

#[test]
fn concurrent_callers_with_one_snapshot_commit_once_and_reject_the_stale_writer() {
    use std::sync::{Barrier, Mutex};

    let service = Arc::new(Mutex::new(DockingService::new(fixture(), 8).unwrap()));
    let gate = Arc::new(Barrier::new(3));
    let callers = [("close-alpha", "alpha"), ("close-beta", "beta")].map(|(command, panel)| {
        let service = Arc::clone(&service);
        let gate = Arc::clone(&gate);
        std::thread::spawn(move || {
            let revision = service.lock().unwrap().snapshot().revision;
            gate.wait();
            service.lock().unwrap().transact(
                revision,
                command.into(),
                [Operation::ClosePanel {
                    panel: panel.into(),
                }],
            )
        })
    });

    gate.wait();
    let outcomes: Vec<_> = callers
        .into_iter()
        .map(|caller| caller.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| result
                .as_ref()
                .is_err_and(|error| error.code == ErrorCode::Conflict))
            .count(),
        1
    );

    let final_state = service.lock().unwrap().snapshot();
    assert_eq!(final_state.revision, 1);
    assert_eq!(final_state.undo_depth, 1);
    assert_eq!(final_state.document.closed_panels.len(), 1);
}
