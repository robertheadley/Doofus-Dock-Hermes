use doofusdock_core::*;
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

fn extension_profile() -> WorkspaceProfile {
    builtin_workspace_profiles()
        .into_iter()
        .find(|profile| matches!(profile.layout, ProfileLayout::Extension { .. }))
        .expect("built-in extension profile")
}

fn registry(modality: &str) -> ExtensionRegistry {
    let descriptor = ExtensionDescriptor {
        namespaced_id: modality.to_owned(),
        version: ApiVersion { major: 1, minor: 0 },
        api: CompatibilityRange {
            minimum: ApiVersion { major: 1, minor: 0 },
            maximum: ApiVersion { major: 1, minor: 0 },
        },
        ..Default::default()
    };
    let mut registry = ExtensionRegistry::default();
    registry
        .register_modality(ModalityExtension {
            descriptor,
            validate_config: Arc::new(|value: &Value| {
                if value.is_object() {
                    Ok(())
                } else {
                    Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Modality config must be an object.",
                        "test-modality",
                    ))
                }
            }),
            operations: vec!["switch".into()],
            capabilities: vec![],
            fallback: ExtensionFallbackPolicy::RejectUnavailable,
            configuration_schema: serde_json::json!({"type":"object"}),
            required_capabilities: vec![],
            incompatible_capabilities: vec![],
        })
        .unwrap();
    registry.freeze();
    registry.activate(modality).unwrap();
    registry
}

fn document_and_assignments(
    profile: &WorkspaceProfile,
) -> (LayoutDocument, BTreeMap<String, Vec<PanelId>>) {
    let mut document = LayoutDocument::new("switch-layout", "switch-workspace");
    let mut assignments = BTreeMap::new();
    let mut nodes = Vec::new();
    for (index, region) in profile.regions.iter().enumerate() {
        let id = PanelId::from(format!("panel-{index}"));
        Arc::make_mut(&mut document.panels).push(PanelDescriptor::new(id.0.as_str(), id.0.clone()));
        assignments.insert(region.id.clone(), vec![id.clone()]);
        nodes.push(Node::panel(format!("node-{index}"), id));
    }
    document.root = Some(if nodes.len() == 1 {
        nodes.remove(0)
    } else {
        Node::tabs("initial-tabs", nodes, 0)
    });
    (document, assignments)
}

#[test]
fn transient_presentation_requests_negotiate_without_mutating_canonical_layout() {
    let document = LayoutDocument::new("transient-layout", "transient-workspace");
    let host = HostCapabilities {
        supported: [
            "overlay.modal",
            "overlay.modeless",
            "overlay.drawer",
            "overlay.peek",
            "viewport:fullscreen",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    };
    let requests = [
        TransientPresentationRequest {
            id: "fullscreen-on".into(),
            title: "Fullscreen".into(),
            kind: TransientPresentationKind::Fullscreen { enabled: true },
        },
        TransientPresentationRequest {
            id: "dialog".into(),
            title: "Modal dialog".into(),
            kind: TransientPresentationKind::Overlay { modal: true },
        },
        TransientPresentationRequest {
            id: "overlay".into(),
            title: "Modeless overlay".into(),
            kind: TransientPresentationKind::Overlay { modal: false },
        },
        TransientPresentationRequest {
            id: "drawer".into(),
            title: "Right drawer".into(),
            kind: TransientPresentationKind::Drawer {
                edge: DockEdge::Right,
                modal: false,
            },
        },
        TransientPresentationRequest {
            id: "peek".into(),
            title: "Peek".into(),
            kind: TransientPresentationKind::Peek {
                edge: DockEdge::Bottom,
            },
        },
    ];
    for request in requests {
        let plan = plan_transient_presentation(&document, request.clone(), &host).unwrap();
        assert_eq!(plan.base_revision(), document.revision);
        assert_eq!(plan.preview(), &request);
        let expected_capability = match request.kind {
            TransientPresentationKind::Fullscreen { .. } => "viewport:fullscreen",
            TransientPresentationKind::Overlay { modal: true }
            | TransientPresentationKind::Drawer { modal: true, .. } => "overlay.modal",
            TransientPresentationKind::Overlay { modal: false } => "overlay.modeless",
            TransientPresentationKind::Drawer { modal: false, .. } => "overlay.drawer",
            TransientPresentationKind::Peek { .. } => "overlay.peek",
        };
        assert_eq!(plan.required_capability(), expected_capability);
        assert_eq!(plan.activate(&document).unwrap(), request);
        assert_eq!(document.revision, 0);
        validate_layout(&document).unwrap();
    }
}

#[test]
fn transient_presentation_rejects_unsupported_invalid_and_stale_requests() {
    let document = LayoutDocument::new("transient-layout", "transient-workspace");
    let request = TransientPresentationRequest {
        id: "dialog".into(),
        title: "Dialog".into(),
        kind: TransientPresentationKind::Overlay { modal: true },
    };
    let unsupported =
        plan_transient_presentation(&document, request.clone(), &HostCapabilities::default())
            .unwrap_err();
    assert_eq!(unsupported.code, ErrorCode::UnsupportedOperation);

    let host = HostCapabilities {
        supported: ["overlay.modal".into()].into_iter().collect(),
    };
    let invalid = TransientPresentationRequest {
        id: "bad/id".into(),
        ..request.clone()
    };
    assert_eq!(
        plan_transient_presentation(&document, invalid, &host)
            .unwrap_err()
            .code,
        ErrorCode::InvalidId
    );

    let plan = plan_transient_presentation(&document, request, &host).unwrap();
    let mut changed = document.clone();
    changed.revision += 1;
    assert_eq!(
        plan.activate(&changed).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(document.revision, 0);
}

#[test]
fn every_transient_presentation_kind_requires_its_declared_capability() {
    let document = LayoutDocument::new("transient-layout", "transient-workspace");
    let before = document.clone();
    let requests = [
        (
            TransientPresentationRequest {
                id: "fullscreen".into(),
                title: "Fullscreen".into(),
                kind: TransientPresentationKind::Fullscreen { enabled: true },
            },
            "viewport:fullscreen",
        ),
        (
            TransientPresentationRequest {
                id: "modal-overlay".into(),
                title: "Modal overlay".into(),
                kind: TransientPresentationKind::Overlay { modal: true },
            },
            "overlay.modal",
        ),
        (
            TransientPresentationRequest {
                id: "modeless-overlay".into(),
                title: "Modeless overlay".into(),
                kind: TransientPresentationKind::Overlay { modal: false },
            },
            "overlay.modeless",
        ),
        (
            TransientPresentationRequest {
                id: "modal-drawer".into(),
                title: "Modal drawer".into(),
                kind: TransientPresentationKind::Drawer {
                    edge: DockEdge::Left,
                    modal: true,
                },
            },
            "overlay.modal",
        ),
        (
            TransientPresentationRequest {
                id: "modeless-drawer".into(),
                title: "Modeless drawer".into(),
                kind: TransientPresentationKind::Drawer {
                    edge: DockEdge::Right,
                    modal: false,
                },
            },
            "overlay.drawer",
        ),
        (
            TransientPresentationRequest {
                id: "peek".into(),
                title: "Peek".into(),
                kind: TransientPresentationKind::Peek {
                    edge: DockEdge::Bottom,
                },
            },
            "overlay.peek",
        ),
    ];
    let all_capabilities = [
        "overlay.modal",
        "overlay.modeless",
        "overlay.drawer",
        "overlay.peek",
        "viewport:fullscreen",
    ];

    for (request, required) in requests {
        let host = HostCapabilities {
            supported: all_capabilities
                .iter()
                .filter(|capability| **capability != required)
                .map(|capability| (*capability).to_owned())
                .collect(),
        };
        let error = plan_transient_presentation(&document, request, &host).unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedOperation);
        assert_eq!(error.context, required);
        assert_eq!(document, before);
    }
}

#[test]
fn dynamic_modality_switch_negotiates_before_atomic_profile_commit() {
    let mut profile = extension_profile();
    if let ProfileLayout::Extension { modality, .. } = &mut profile.layout {
        *modality = "sample:spatial".into();
    }
    let modality = match &profile.layout {
        ProfileLayout::Extension { modality, .. } => modality.clone(),
        _ => unreachable!(),
    };
    let spec = WorkspaceSpecification::new(profile.clone()).unwrap();
    let (document, assignments) = document_and_assignments(&profile);
    let registry = registry(&modality);
    let mut service = DockingService::new(document, 16).unwrap();

    let rejected = plan_profile_switch(
        &service.snapshot().document,
        0,
        &spec,
        &assignments,
        &registry,
        &HostCapabilities::default(),
    )
    .unwrap_err();
    assert_eq!(rejected.code, ErrorCode::UnsupportedOperation);
    assert_eq!(service.snapshot().revision, 0);

    let host = HostCapabilities {
        supported: [format!("modality:{modality}")].into_iter().collect(),
    };
    let plan = plan_profile_switch(
        &service.snapshot().document,
        0,
        &spec,
        &assignments,
        &registry,
        &host,
    )
    .unwrap();
    assert_eq!(plan.modalities, vec![modality]);
    let committed = service.apply_profile_switch(0, plan).unwrap();
    assert_eq!(committed.document.revision, 1);
    assert_eq!(service.snapshot().undo_depth, 1);
    assert!(validate_layout(&committed.document).is_ok());

    let undone = service.undo(committed.document.revision).unwrap();
    assert_eq!(undone.document.revision, 2);
    assert!(validate_layout(&undone.document).is_ok());
    let redone = service.redo(undone.document.revision).unwrap();
    assert_eq!(redone.document.revision, 3);
    assert!(validate_layout(&redone.document).is_ok());
}

#[test]
fn prepared_modality_switch_rejects_stale_revision_without_mutation() {
    let mut profile = extension_profile();
    if let ProfileLayout::Extension { modality, .. } = &mut profile.layout {
        *modality = "sample:spatial".into();
    }
    let modality = match &profile.layout {
        ProfileLayout::Extension { modality, .. } => modality.clone(),
        _ => unreachable!(),
    };
    let spec = WorkspaceSpecification::new(profile.clone()).unwrap();
    let (document, assignments) = document_and_assignments(&profile);
    let registry = registry(&modality);
    let host = HostCapabilities {
        supported: [format!("modality:{modality}")].into_iter().collect(),
    };
    let mut service = DockingService::new(document, 16).unwrap();
    let plan = plan_profile_switch(
        &service.snapshot().document,
        0,
        &spec,
        &assignments,
        &registry,
        &host,
    )
    .unwrap();

    service
        .transact(
            0,
            "concurrent-layout-change".into(),
            [Operation::SelectTab {
                tabs: "initial-tabs".into(),
                index: 1,
            }],
        )
        .unwrap();
    let before = service.snapshot();
    let error = service.apply_profile_switch(0, plan).unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    let after = service.snapshot();
    assert_eq!(after.document, before.document);
    assert_eq!(after.undo_depth, before.undo_depth);
}

#[test]
fn picture_in_picture_plan_floats_and_marks_window_in_one_undoable_revision() {
    let mut document = LayoutDocument::new("pip-layout", "pip-workspace");
    Arc::make_mut(&mut document.panels).push(PanelDescriptor::new("panel", "Panel"));
    document.root = Some(Node::panel("panel-node", "panel"));
    let original = document.clone();
    let host = HostCapabilities {
        supported: ["picture_in_picture".into()].into_iter().collect(),
    };
    let plan = plan_modality_change(
        &document,
        ModalityChange::PictureInPicture {
            panel: "panel".into(),
            window: "pip-window".into(),
            geometry: Rect {
                x: 700,
                y: 60,
                width: 320,
                height: 180,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
        &host,
    )
    .unwrap();
    assert_eq!(plan.preview().revision, 1);
    assert_eq!(plan.preview().root, None);
    assert!(plan.preview().floating_windows[0].picture_in_picture);
    assert_eq!(document, original);

    let mut service = DockingService::new(document, 8).unwrap();
    plan.commit(&mut service).unwrap();
    assert_eq!(service.snapshot().revision, 1);
    assert!(service.snapshot().document.floating_windows[0].picture_in_picture);
    service.undo(1).unwrap();
    let mut expected = original;
    expected.revision = 2;
    assert_eq!(service.snapshot().document, expected);
}
