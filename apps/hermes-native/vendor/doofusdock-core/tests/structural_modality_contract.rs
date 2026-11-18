use doofusdock_core::*;
use std::{collections::BTreeSet, sync::Arc};
fn document() -> LayoutDocument {
    let mut d = LayoutDocument::new("layout", "workspace");
    Arc::make_mut(&mut d.panels).extend([
        PanelDescriptor::new("one", "One"),
        PanelDescriptor::new("two", "Two"),
        PanelDescriptor::new("home", "Home"),
    ]);
    d.root = Some(Node::tabs(
        "tabs",
        vec![
            Node::panel("one-node", "one"),
            Node::panel("two-node", "two"),
            Node::panel("home-node", "home"),
        ],
        1,
    ));
    d
}
fn host() -> HostCapabilities {
    HostCapabilities {
        supported: [
            "workspace:tabs",
            "workspace:docking",
            "workspace:accordion",
            "workspace:shade_stack",
            "workspace:layered",
            "workspace:grid",
            "workspace:spatial",
            "workspace:node_graph",
            "workspace:auto_hide",
            "workspace:focus",
            "workspace:layout_strategy",
            "workspace:scrollable_columns",
            "native_viewport",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    }
}

#[test]
fn node_graph_modality_negotiates_converts_and_round_trips_through_spatial() {
    let initial = document();
    let mut service = DockingService::new(initial.clone(), 16).unwrap();
    let graph_plan = plan_modality_change(
        &initial,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::NodeGraph,
        },
        &host(),
    )
    .unwrap();
    assert_eq!(service.snapshot().document, initial);
    let graph = graph_plan.commit(&mut service).unwrap().document;
    let graph_node = graph.root.as_ref().unwrap();
    assert!(matches!(
        &graph_node.kind,
        NodeKind::Extension { modality, payload, children }
            if modality.0 == "doofusdock.node_graph"
                && payload == "{}"
                && children.iter().map(|child| child.id.clone()).collect::<Vec<_>>()
                    == vec!["one-node".into(), "two-node".into(), "home-node".into()]
    ));

    let grid = plan_modality_change(
        &graph,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::Grid { columns: 2, gap: 8 },
        },
        &host(),
    )
    .unwrap()
    .commit(&mut service)
    .unwrap()
    .document;
    let grid_round_trip = plan_modality_change(
        &grid,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::NodeGraph,
        },
        &host(),
    )
    .unwrap()
    .commit(&mut service)
    .unwrap()
    .document;
    assert_eq!(grid_round_trip.root, graph.root);

    let spatial = plan_modality_change(
        &grid_round_trip,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::Spatial,
        },
        &host(),
    )
    .unwrap()
    .commit(&mut service)
    .unwrap()
    .document;
    let persisted = serialize_layout(&spatial).unwrap();
    let restored = deserialize_layout(&persisted, PersistenceLimits::default()).unwrap();
    assert_eq!(restored.root, spatial.root);
    let graph_again = plan_modality_change(
        &spatial,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::NodeGraph,
        },
        &host(),
    )
    .unwrap()
    .commit(&mut service)
    .unwrap()
    .document;
    assert_eq!(graph_again.root, graph.root);
    let undone = service.undo(graph_again.revision).unwrap();
    assert_eq!(undone.document.root, spatial.root);
    assert_eq!(
        service
            .redo(undone.document.revision)
            .unwrap()
            .document
            .root,
        graph.root
    );
}

#[test]
fn built_in_container_conversion_matrix_preserves_panel_and_child_identity() {
    let modalities = [
        ContainerModality::Tabbed { active_index: 1 },
        ContainerModality::Split {
            orientation: Orientation::Horizontal,
            weights: vec![],
        },
        ContainerModality::Accordion { multiple: false },
        ContainerModality::Accordion { multiple: true },
        ContainerModality::ShadeStack,
        ContainerModality::Grid { columns: 2, gap: 6 },
        ContainerModality::Spatial,
        ContainerModality::NodeGraph,
        ContainerModality::Layered,
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::MasterStack,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::BinarySpacePartition,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::Spiral,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::Dwindle,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::Monocle,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::EqualColumns,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::EqualRows,
            gap: 8,
        },
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::AdaptiveGrid,
            gap: 8,
        },
        ContainerModality::ScrollableColumns {
            gap: 8,
            width: 240,
            minimum_width: 120,
            maximum_width: Some(420),
        },
    ];
    let expected_panels = BTreeSet::from([
        PanelId::from("one"),
        PanelId::from("two"),
        PanelId::from("home"),
    ]);
    let expected_children = BTreeSet::from([
        NodeId::from("one-node"),
        NodeId::from("two-node"),
        NodeId::from("home-node"),
    ]);

    for source in &modalities {
        for target in &modalities {
            let mut service = DockingService::new(document(), 64).unwrap();
            if !matches!(source, ContainerModality::Tabbed { .. }) {
                let source_plan = plan_modality_change(
                    &service.snapshot().document,
                    ModalityChange::Container {
                        container: "tabs".into(),
                        target: source.clone(),
                    },
                    &host(),
                )
                .unwrap();
                source_plan.commit(&mut service).unwrap();
            }
            let snapshot = service.snapshot();
            let target_plan = plan_modality_change(
                &snapshot.document,
                ModalityChange::Container {
                    container: "tabs".into(),
                    target: target.clone(),
                },
                &host(),
            )
            .unwrap_or_else(|error| {
                panic!("built-in conversion {source:?} -> {target:?} rejected: {error}")
            });
            let converted = target_plan.commit(&mut service).unwrap().document;
            validate_layout(&converted).unwrap();
            assert_eq!(
                converted
                    .panels
                    .iter()
                    .map(|panel| panel.id.clone())
                    .collect::<BTreeSet<_>>(),
                expected_panels,
                "panel identities changed during {source:?} -> {target:?}"
            );
            let root = converted.root.as_ref().unwrap();
            assert_eq!(root.id, NodeId::from("tabs"));
            let children = match &root.kind {
                NodeKind::Tabs { items, .. }
                | NodeKind::Split {
                    children: items, ..
                } => items,
                NodeKind::Extension { children, .. } => children,
                _ => panic!("conversion {source:?} -> {target:?} did not produce a container"),
            };
            assert_eq!(
                children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect::<BTreeSet<_>>(),
                expected_children,
                "child identities changed during {source:?} -> {target:?}"
            );
            if converted.revision != snapshot.document.revision {
                let undone = service.undo(converted.revision).unwrap().document;
                assert_eq!(
                    undone.root, snapshot.document.root,
                    "undo did not restore the source for {source:?} -> {target:?}"
                );
                let redone = service.redo(undone.revision).unwrap().document;
                assert_eq!(
                    redone.root, converted.root,
                    "redo did not restore the target for {source:?} -> {target:?}"
                );
            }
        }
    }
}

#[test]
fn all_builtin_destinations_require_their_host_capability_before_mutation() {
    let modalities = [
        ContainerModality::Tabbed { active_index: 1 },
        ContainerModality::Split {
            orientation: Orientation::Horizontal,
            weights: vec![],
        },
        ContainerModality::Accordion { multiple: false },
        ContainerModality::Accordion { multiple: true },
        ContainerModality::ShadeStack,
        ContainerModality::Grid { columns: 2, gap: 6 },
        ContainerModality::Spatial,
        ContainerModality::NodeGraph,
        ContainerModality::Layered,
        ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::AdaptiveGrid,
            gap: 6,
        },
        ContainerModality::ScrollableColumns {
            gap: 6,
            width: 240,
            minimum_width: 120,
            maximum_width: Some(420),
        },
    ];
    let required_capability = |modality: &ContainerModality| match modality {
        ContainerModality::Tabbed { .. } => "workspace:tabs",
        ContainerModality::Split { .. } => "workspace:docking",
        ContainerModality::Accordion { .. } => "workspace:accordion",
        ContainerModality::ShadeStack => "workspace:shade_stack",
        ContainerModality::Grid { .. } => "workspace:grid",
        ContainerModality::Spatial => "workspace:spatial",
        ContainerModality::NodeGraph => "workspace:node_graph",
        ContainerModality::Layered => "workspace:layered",
        ContainerModality::LayoutStrategy { .. } => "workspace:layout_strategy",
        ContainerModality::ScrollableColumns { .. } => "workspace:scrollable_columns",
    };

    for source in &modalities {
        for target in &modalities {
            let mut service = DockingService::new(document(), 16).unwrap();
            if !matches!(source, ContainerModality::Tabbed { .. }) {
                plan_modality_change(
                    &service.snapshot().document,
                    ModalityChange::Container {
                        container: "tabs".into(),
                        target: source.clone(),
                    },
                    &host(),
                )
                .unwrap()
                .commit(&mut service)
                .unwrap();
            }
            let before = service.snapshot();
            let mut unsupported = host();
            unsupported.supported.remove(required_capability(target));
            let error = plan_modality_change(
                &before.document,
                ModalityChange::Container {
                    container: "tabs".into(),
                    target: target.clone(),
                },
                &unsupported,
            )
            .unwrap_err();
            assert_eq!(
                error.code,
                ErrorCode::UnsupportedOperation,
                "missing capability for {source:?} -> {target:?}"
            );
            assert_eq!(
                service.snapshot(),
                before,
                "rejected conversion changed canonical state for {source:?} -> {target:?}"
            );
        }
    }
}

#[test]
fn node_graph_destination_fails_before_mutation_without_host_capability() {
    let document = document();
    let service = DockingService::new(document.clone(), 8).unwrap();
    let mut unsupported = host();
    unsupported.supported.remove("workspace:node_graph");
    let error = plan_modality_change(
        &document,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::NodeGraph,
        },
        &unsupported,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedOperation);
    assert_eq!(service.snapshot().document, document);
}

#[test]
fn layered_order_selection_switching_persistence_and_history_are_canonical() {
    let mut service = DockingService::new(document(), 16).unwrap();
    let initial = service.snapshot();
    let plan = plan_modality_change(
        &initial.document,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::Layered,
        },
        &host(),
    )
    .unwrap();
    let preview = plan.preview();
    let NodeKind::Extension {
        payload, children, ..
    } = &preview.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    let preview_config = LayeredConfiguration::parse(payload, children).unwrap();
    assert_eq!(preview_config.active, "two-node".into());
    assert_eq!(
        preview_config.back_to_front,
        vec!["one-node".into(), "home-node".into(), "two-node".into()]
    );
    assert_eq!(
        service.snapshot(),
        initial,
        "preview must not mutate canonical state"
    );

    let committed = plan.commit(&mut service).unwrap();
    let raised = service
        .transact(
            committed.document.revision,
            "raise-layer".into(),
            [Operation::RaiseLayer {
                container: "tabs".into(),
                child: "one-node".into(),
            }],
        )
        .unwrap();
    let NodeKind::Extension {
        payload, children, ..
    } = &raised.document.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    let raised_config = LayeredConfiguration::parse(payload, children).unwrap();
    assert_eq!(
        raised_config.back_to_front.last(),
        Some(&NodeId::from("one-node"))
    );
    assert_eq!(raised_config.active, "one-node".into());

    let lowered = service
        .transact(
            raised.document.revision,
            "lower-layer".into(),
            [Operation::LowerLayer {
                container: "tabs".into(),
                child: "one-node".into(),
            }],
        )
        .unwrap();
    let NodeKind::Extension {
        payload, children, ..
    } = &lowered.document.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    let lowered_config = LayeredConfiguration::parse(payload, children).unwrap();
    assert_eq!(
        lowered_config.back_to_front.first(),
        Some(&NodeId::from("one-node"))
    );

    let selected = service
        .transact(
            lowered.document.revision,
            "select-layer".into(),
            [Operation::SelectLayer {
                container: "tabs".into(),
                child: "home-node".into(),
            }],
        )
        .unwrap();
    let NodeKind::Extension {
        payload, children, ..
    } = &selected.document.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    let selected_config = LayeredConfiguration::parse(payload, children).unwrap();
    assert_eq!(
        selected_config.back_to_front.last(),
        Some(&NodeId::from("home-node"))
    );
    assert_eq!(selected_config.active, "home-node".into());

    let restored = deserialize_layout(
        &serialize_layout(&selected.document).unwrap(),
        PersistenceLimits::default(),
    )
    .unwrap();
    assert_eq!(restored.root, selected.document.root);
    let undo = service.undo(selected.document.revision).unwrap();
    assert_eq!(undo.document.root, lowered.document.root);
    let redo = service.redo(undo.document.revision).unwrap();
    assert_eq!(redo.document.root, selected.document.root);

    let tabs = apply_operation(
        &selected.document,
        Operation::ConvertContainer {
            container: "tabs".into(),
            target: ContainerModality::Tabbed { active_index: 0 },
        },
    )
    .unwrap()
    .document;
    let NodeKind::Tabs {
        active_index,
        items,
        ..
    } = &tabs.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    assert_eq!(
        items
            .iter()
            .map(|child| child.id.0.as_str())
            .collect::<Vec<_>>(),
        vec!["one-node", "two-node", "home-node"]
    );
    assert_eq!(items[*active_index].id.0, "home-node");
}

#[test]
fn layered_schema_rejects_duplicate_missing_and_stale_child_ids() {
    let children = match &document().root.unwrap().kind {
        NodeKind::Tabs { items, .. } => items.clone(),
        _ => unreachable!(),
    };
    for config in [
        LayeredConfiguration {
            version: 1,
            back_to_front: vec!["one-node".into(), "one-node".into(), "home-node".into()],
            active: "one-node".into(),
        },
        LayeredConfiguration {
            version: 1,
            back_to_front: vec!["one-node".into(), "two-node".into(), "missing".into()],
            active: "one-node".into(),
        },
        LayeredConfiguration {
            version: 2,
            back_to_front: vec!["one-node".into(), "two-node".into(), "home-node".into()],
            active: "two-node".into(),
        },
    ] {
        assert!(LayeredConfiguration::parse(&config.to_payload().unwrap(), &children).is_err());
    }
    let valid = LayeredConfiguration::from_children(&children).unwrap();
    assert!(
        LayeredConfiguration::parse_ids(
            &valid.to_payload().unwrap(),
            &["one-node".into(), "one-node".into(), "home-node".into()],
        )
        .is_err()
    );
    assert_eq!(
        plan_modality_change(
            &document(),
            ModalityChange::Container {
                container: "tabs".into(),
                target: ContainerModality::Layered,
            },
            &HostCapabilities::default(),
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedOperation
    );
}

#[test]
fn layered_profile_negotiates_provider_and_generates_stable_child_configuration() {
    let profile = builtin_workspace_profiles()
        .into_iter()
        .find(|profile| profile.id == "layered")
        .unwrap();
    let specification = WorkspaceSpecification::new(profile).unwrap();
    let assignments = std::collections::BTreeMap::from([
        ("base".into(), vec![PanelId::from("one")]),
        ("sheet".into(), vec![PanelId::from("two")]),
        ("popover".into(), vec![PanelId::from("home")]),
    ]);
    let registry = ExtensionRegistry::default();
    let mut declared_host = host();
    declared_host
        .supported
        .insert("layout-provider:doofusdock.layered".into());
    let plan = plan_profile_switch(
        &document(),
        0,
        &specification,
        &assignments,
        &registry,
        &declared_host,
    )
    .unwrap();
    let NodeKind::Extension {
        modality,
        payload,
        children,
    } = &plan.application.document.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    assert_eq!(modality.0, LAYERED_MODALITY);
    let config = LayeredConfiguration::parse(payload, children).unwrap();
    assert_eq!(config.back_to_front.len(), 3);
    assert_eq!(config.active, children.last().unwrap().id);

    let err = plan_profile_switch(
        &document(),
        0,
        &specification,
        &assignments,
        &registry,
        &host(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::UnsupportedOperation);
    assert!(err.context.contains("layout-provider:doofusdock.layered"));
}

#[test]
fn closing_active_layer_prunes_order_and_selects_remaining_top_layer() {
    let layered = apply_operation(
        &document(),
        Operation::ConvertContainer {
            container: "tabs".into(),
            target: ContainerModality::Layered,
        },
    )
    .unwrap()
    .document;
    let closed = apply_operation(
        &layered,
        Operation::ClosePanel {
            panel: "two".into(),
        },
    )
    .unwrap()
    .document;
    let NodeKind::Extension {
        payload, children, ..
    } = &closed.root.as_ref().unwrap().kind
    else {
        unreachable!()
    };
    let config = LayeredConfiguration::parse(payload, children).unwrap();
    assert_eq!(
        config.back_to_front,
        vec!["one-node".into(), "home-node".into()]
    );
    assert_eq!(config.active, "home-node".into());
}

#[test]
fn structural_roundtrips_preserve_children_and_support_history_persistence() {
    let d = document();
    let children = if let NodeKind::Tabs { items, .. } = &d.root.as_ref().unwrap().kind {
        items.clone()
    } else {
        unreachable!()
    };
    let mut service = DockingService::new(d.clone(), 32).unwrap();
    for target in [
        ContainerModality::Split {
            orientation: Orientation::Horizontal,
            weights: vec![],
        },
        ContainerModality::Accordion { multiple: false },
        ContainerModality::Split {
            orientation: Orientation::Vertical,
            weights: vec![],
        },
        ContainerModality::ShadeStack,
        ContainerModality::Tabbed { active_index: 1 },
    ] {
        let before = service.snapshot();
        let plan = plan_modality_change(
            &before.document,
            ModalityChange::Container {
                container: "tabs".into(),
                target,
            },
            &host(),
        )
        .unwrap();
        assert_eq!(service.snapshot(), before);
        let result = plan.commit(&mut service).unwrap();
        assert_eq!(result.document.revision, before.revision + 1);
        let root = result.document.root.as_ref().unwrap();
        assert_eq!(root.id.0, "tabs");
        let next_children = match &root.kind {
            NodeKind::Tabs { items, .. } => items,
            NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => children,
            _ => unreachable!(),
        };
        assert_eq!(next_children, &children);
        let restored = deserialize_layout(
            &serialize_layout(&result.document).unwrap(),
            PersistenceLimits::default(),
        )
        .unwrap();
        assert_eq!(
            serialize_layout(&restored).unwrap(),
            serialize_layout(&result.document).unwrap()
        );
        assert_eq!(restored.root, result.document.root);
        let undo = service.undo(result.document.revision).unwrap();
        assert_eq!(undo.document.root, before.document.root);
        service.redo(undo.document.revision).unwrap();
    }
}
#[test]
fn stack_expansion_selection_and_close_cleanup_are_canonical_and_atomic() {
    let d = apply_operation(
        &document(),
        Operation::ConvertContainer {
            container: "tabs".into(),
            target: ContainerModality::Accordion { multiple: false },
        },
    )
    .unwrap()
    .document;
    let d = apply_operation(
        &d,
        Operation::SetStackExpanded {
            container: "tabs".into(),
            child: "one-node".into(),
            expanded: true,
        },
    )
    .unwrap()
    .document;
    let parse = |d: &LayoutDocument| {
        let NodeKind::Extension {
            payload, children, ..
        } = &d.root.as_ref().unwrap().kind
        else {
            unreachable!()
        };
        StackConfiguration::parse(payload, children).unwrap()
    };
    assert_eq!(parse(&d).expanded, BTreeSet::from(["one-node".into()]));
    let closed = apply_operation(
        &d,
        Operation::ClosePanel {
            panel: "one".into(),
        },
    )
    .unwrap()
    .document;
    assert!(parse(&closed).expanded.is_empty());
    assert_ne!(parse(&closed).selected, Some("one-node".into()));
    validate_layout(&closed).unwrap();
    let bad = apply_operation(
        &d,
        Operation::UpdateExtensionPayload {
            container: "tabs".into(),
            payload: r#"{"version":2,"multiple":false,"expanded":[],"selected":null}"#.into(),
        },
    )
    .unwrap_err();
    assert_eq!(bad.code, ErrorCode::ConstraintViolation);
    assert_eq!(parse(&d).expanded.len(), 1);
}
#[test]
fn geometry_returns_conflicting_container_and_never_overflows() {
    let d = apply_operation(
        &document(),
        Operation::ConvertContainer {
            container: "tabs".into(),
            target: ContainerModality::ShadeStack,
        },
    )
    .unwrap()
    .document;
    let root = d.root.as_ref().unwrap();
    let bounds = Rect {
        x: 10,
        y: 20,
        width: 600,
        height: 300,
    };
    let sections = arrange_stack(root, bounds, 24, &[Size::new(100, 60); 3]).unwrap();
    assert_eq!(sections.len(), 3);
    assert_eq!(sections.iter().filter(|s| s.body.is_some()).count(), 1);
    assert_eq!(sections.last().unwrap().header.y + 24, 320);
    let error = arrange_stack(
        root,
        Rect {
            height: 100,
            ..bounds
        },
        24,
        &[Size::new(100, 60); 3],
    )
    .unwrap_err();
    assert_eq!(error.context, "tabs");
    assert!(error.message.contains("132"));
}
#[test]
fn stale_plans_capability_and_policy_rejections_do_not_change_history() {
    let d = document();
    let change = || ModalityChange::Container {
        container: "tabs".into(),
        target: ContainerModality::Split {
            orientation: Orientation::Horizontal,
            weights: vec![],
        },
    };
    assert_eq!(
        plan_modality_change(&d, change(), &HostCapabilities::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedOperation
    );
    let mut blocked = d.clone();
    Arc::make_mut(&mut blocked.panels)[0].docking.allow_split = false;
    assert!(plan_modality_change(&blocked, change(), &host()).is_err());
    let mut service = DockingService::new(d.clone(), 8).unwrap();
    let plan = plan_modality_change(&d, change(), &host()).unwrap();
    service
        .transact(
            0,
            "select".into(),
            [Operation::SelectTab {
                tabs: "tabs".into(),
                index: 0,
            }],
        )
        .unwrap();
    let before = service.snapshot();
    assert_eq!(
        plan.commit(&mut service).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(service.snapshot(), before);
}
#[test]
fn presentation_modality_changes_fail_closed_without_host_capabilities() {
    let base = document();
    let floating = apply_operation(
        &base,
        Operation::FloatPanel {
            panel: "one".into(),
            window: "native".into(),
            geometry: Rect {
                x: 20,
                y: 30,
                width: 400,
                height: 300,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
    )
    .unwrap()
    .document;
    let auto_hidden = apply_operation(
        &base,
        Operation::AutoHide {
            panel: "one".into(),
            edge: DockEdge::Left,
        },
    )
    .unwrap()
    .document;
    let focused = apply_operation(
        &base,
        Operation::Maximize {
            panel: "one".into(),
        },
    )
    .unwrap()
    .document;

    let cases = [
        (
            base.clone(),
            ModalityChange::NativeFloating {
                panel: "one".into(),
                window: "requested".into(),
                geometry: Rect {
                    x: 20,
                    y: 30,
                    width: 400,
                    height: 300,
                },
                monitor: "primary".into(),
                scale: 1.0,
            },
            "native_viewport",
        ),
        (
            base.clone(),
            ModalityChange::AutoHide {
                panel: "one".into(),
                edge: DockEdge::Left,
            },
            "workspace:auto_hide",
        ),
        (
            floating.clone(),
            ModalityChange::DockBack {
                window: "native".into(),
                target_panel: None,
            },
            "workspace:docking",
        ),
        (
            auto_hidden,
            ModalityChange::Docked {
                panel: "one".into(),
                target_tabs: "tabs".into(),
                index: 0,
            },
            "workspace:tabs",
        ),
        (
            base.clone(),
            ModalityChange::Focus {
                panel: "one".into(),
            },
            "workspace:focus",
        ),
        (focused, ModalityChange::RestoreFocus, "workspace:focus"),
    ];

    for (document, change, capability) in cases {
        let service = DockingService::new(document.clone(), 8).unwrap();
        let before = service.snapshot();
        let mut unsupported = host();
        unsupported.supported.remove(capability);
        let error = plan_modality_change(&document, change, &unsupported).unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedOperation);
        assert_eq!(error.context, capability);
        assert_eq!(service.snapshot(), before);
    }
}

#[test]
fn native_auto_hide_and_focus_switches_use_existing_atomic_operations() {
    let mut service = DockingService::new(document(), 16).unwrap();
    let changes = [
        ModalityChange::NativeFloating {
            panel: "one".into(),
            window: "native".into(),
            geometry: Rect {
                x: 20,
                y: 30,
                width: 400,
                height: 300,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
        ModalityChange::DockBack {
            window: "native".into(),
            target_panel: Some("two".into()),
        },
        ModalityChange::AutoHide {
            panel: "one".into(),
            edge: DockEdge::Left,
        },
        ModalityChange::Docked {
            panel: "one".into(),
            target_tabs: "tabs".into(),
            index: 0,
        },
        ModalityChange::Focus {
            panel: "one".into(),
        },
        ModalityChange::RestoreFocus,
    ];
    for change in changes {
        let before = service.snapshot();
        let plan = plan_modality_change(&before.document, change, &host()).unwrap();
        let result = plan.commit(&mut service).unwrap();
        assert_eq!(result.document.revision, before.revision + 1);
        assert_eq!(result.document.panels, before.document.panels);
        validate_layout(&result.document).unwrap();
    }
}

#[test]
fn negotiated_auto_hide_to_docked_transition_preserves_identity_and_history() {
    let mut service = DockingService::new(document(), 8).unwrap();
    let initial = service.snapshot();
    let hidden = plan_modality_change(
        &initial.document,
        ModalityChange::AutoHide {
            panel: "one".into(),
            edge: DockEdge::Bottom,
        },
        &host(),
    )
    .unwrap()
    .commit(&mut service)
    .unwrap();
    assert_eq!(hidden.document.auto_hide_panels.len(), 1);
    let panel_descriptor = hidden
        .document
        .descriptor(&PanelId::from("one"))
        .unwrap()
        .clone();

    let restored = plan_modality_change(
        &hidden.document,
        ModalityChange::Docked {
            panel: "one".into(),
            target_tabs: "tabs".into(),
            index: 0,
        },
        &host(),
    )
    .unwrap()
    .commit(&mut service)
    .unwrap();
    assert!(restored.document.auto_hide_panels.is_empty());
    assert_eq!(
        restored.document.descriptor(&PanelId::from("one")),
        Some(&panel_descriptor)
    );
    let serialized = serialize_layout(&restored.document).unwrap();
    let loaded = deserialize_layout(&serialized, PersistenceLimits::default()).unwrap();
    assert_eq!(serialize_layout(&loaded).unwrap(), serialized);
    assert_eq!(loaded.root, restored.document.root);
    assert_eq!(loaded.auto_hide_panels, restored.document.auto_hide_panels);

    let undone = service.undo(restored.document.revision).unwrap();
    assert_eq!(undone.document.auto_hide_panels.len(), 1);
    assert_eq!(
        undone.document.descriptor(&PanelId::from("one")),
        Some(&panel_descriptor)
    );
    let redone = service.redo(undone.document.revision).unwrap();
    assert!(redone.document.auto_hide_panels.is_empty());
    assert_eq!(
        redone.document.descriptor(&PanelId::from("one")),
        Some(&panel_descriptor)
    );
    validate_layout(&redone.document).unwrap();
}

#[test]
fn split_orientation_only_change_is_a_real_undoable_transaction() {
    let mut service = DockingService::new(document(), 8).unwrap();
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let before = service.snapshot();
        let result = plan_modality_change(
            &before.document,
            ModalityChange::Container {
                container: "tabs".into(),
                target: ContainerModality::Split {
                    orientation,
                    weights: vec![],
                },
            },
            &host(),
        )
        .unwrap()
        .commit(&mut service)
        .unwrap();
        assert_eq!(result.document.revision, before.revision + 1);
        assert!(
            matches!(service.snapshot().document.root.as_ref().unwrap().kind, NodeKind::Split { orientation: current, .. } if current == orientation)
        );
    }
    let undo = service.undo(2).unwrap();
    assert!(matches!(
        undo.document.root.as_ref().unwrap().kind,
        NodeKind::Split {
            orientation: Orientation::Horizontal,
            ..
        }
    ));
}

#[test]
fn grid_spatial_switching_retains_provider_configuration_and_subtree_identity() {
    let mut d = document();
    let tabs = d.root.take().unwrap();
    d.root = Some(std::sync::Arc::new(Node {
        id: tabs.id.clone(),
        kind: NodeKind::Extension {
            modality: "doofusdock.masonry".into(),
            payload: r#"{"columns":2,"gap":12}"#.into(),
            children: match &tabs.kind {
                NodeKind::Tabs { items, .. } => items.clone(),
                _ => unreachable!(),
            },
        },
    }));
    let original = d.root.clone();
    let host = HostCapabilities {
        supported: ["workspace:grid".into(), "workspace:spatial".into()]
            .into_iter()
            .collect(),
    };
    let mut service = DockingService::new(d, 8).unwrap();
    let spatial = plan_modality_change(
        &service.snapshot().document,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::Spatial,
        },
        &host,
    )
    .unwrap()
    .commit(&mut service)
    .unwrap()
    .document;
    assert!(
        matches!(&spatial.root.as_ref().unwrap().kind, NodeKind::Extension { modality, .. } if modality.0 == "doofusdock.spatial")
    );
    assert_eq!(spatial.root.as_ref().unwrap().id, "tabs".into());
    let grid = plan_modality_change(
        &spatial,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::Grid {
                columns: 2,
                gap: 12,
            },
        },
        &host,
    )
    .unwrap()
    .commit(&mut service)
    .unwrap()
    .document;
    assert_eq!(grid.root, original);
    let serialized = serialize_layout(&grid).unwrap();
    assert_eq!(
        deserialize_layout(&serialized, PersistenceLimits::default())
            .unwrap()
            .root,
        original
    );
}

#[test]
fn grid_spatial_node_graph_path_permutations_restore_source_provider_payloads() {
    let initial = document();
    let mut service = DockingService::new(initial.clone(), 32).unwrap();
    let convert = |service: &mut DockingService, target| {
        let snapshot = service.snapshot();
        plan_modality_change(
            &snapshot.document,
            ModalityChange::Container {
                container: "tabs".into(),
                target,
            },
            &host(),
        )
        .unwrap()
        .commit(service)
        .unwrap()
        .document
    };

    let grid = convert(
        &mut service,
        ContainerModality::Grid {
            columns: 2,
            gap: 13,
        },
    );
    let grid_root = grid.root.clone();
    let _graph = convert(&mut service, ContainerModality::NodeGraph);
    let _spatial = convert(&mut service, ContainerModality::Spatial);
    let graph_again = convert(&mut service, ContainerModality::NodeGraph);
    let grid_again = convert(&mut service, ContainerModality::Grid { columns: 5, gap: 4 });
    assert_eq!(grid_again.root, grid_root);

    let spatial = convert(&mut service, ContainerModality::Spatial);
    let spatial_root = spatial.root.clone();
    let graph = convert(&mut service, ContainerModality::NodeGraph);
    let spatial_again = convert(&mut service, ContainerModality::Spatial);
    assert_eq!(spatial_again.root, spatial_root);

    for layout in [grid_again, spatial_again] {
        let encoded = serialize_layout(&layout).unwrap();
        let decoded = deserialize_layout(&encoded, PersistenceLimits::default()).unwrap();
        assert_eq!(decoded.root, layout.root);
    }
    assert_eq!(graph.root.as_ref().unwrap().id, NodeId::from("tabs"));
    assert_eq!(graph_again.root.as_ref().unwrap().id, NodeId::from("tabs"));
}

#[test]
fn malformed_builtin_provider_payload_fails_closed_before_conversion() {
    for payload in ["{bad-json", "[]"] {
        let mut d = document();
        let tabs = d.root.take().unwrap();
        d.root = Some(std::sync::Arc::new(Node {
            id: tabs.id.clone(),
            kind: NodeKind::Extension {
                modality: "doofusdock.masonry".into(),
                payload: payload.into(),
                children: match &tabs.kind {
                    NodeKind::Tabs { items, .. } => items.clone(),
                    _ => unreachable!(),
                },
            },
        }));
        let error = match DockingService::new(d.clone(), 8) {
            Err(error) => error,
            Ok(_) => panic!("invalid provider payload unexpectedly initialized a service"),
        };
        assert_eq!(error.code, ErrorCode::InvalidLayout);
        assert_eq!(d.root.as_ref().unwrap().id, "tabs".into());
        let error = plan_modality_change(
            &d,
            ModalityChange::Container {
                container: "tabs".into(),
                target: ContainerModality::Tabbed { active_index: 0 },
            },
            &host(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidLayout);
        assert_eq!(d.root.as_ref().unwrap().id, "tabs".into());
    }
}

#[test]
fn grid_configuration_rejects_unbounded_values_before_mutation() {
    let d = document();
    let service = DockingService::new(d.clone(), 8).unwrap();
    let error = plan_modality_change(
        &d,
        ModalityChange::Container {
            container: "tabs".into(),
            target: ContainerModality::Grid {
                columns: 1000,
                gap: 500,
            },
        },
        &host(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ConstraintViolation);
    assert_eq!(service.snapshot().document, d);
}
