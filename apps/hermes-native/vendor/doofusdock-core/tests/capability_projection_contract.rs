use doofusdock_core::*;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;

fn descriptor(id: &str) -> ExtensionDescriptor {
    ExtensionDescriptor {
        namespaced_id: id.into(),
        version: ApiVersion { major: 2, minor: 1 },
        api: CompatibilityRange {
            minimum: ApiVersion { major: 1, minor: 0 },
            maximum: ApiVersion { major: 2, minor: 5 },
        },
        ..ExtensionDescriptor::default()
    }
}

fn registry() -> ExtensionRegistry {
    let mut registry = ExtensionRegistry::new(ApiVersion { major: 2, minor: 0 });
    registry
        .register_panel_provider(PanelProviderExtension {
            descriptor: descriptor("panels:editor"),
            validate_panel: Arc::new(|_| Ok(())),
        })
        .unwrap();
    registry
        .register_modality(ModalityExtension {
            descriptor: descriptor("layout:overlay"),
            validate_config: Arc::new(|_| Ok(())),
            operations: vec!["present".into(), "dismiss".into()],
            capabilities: vec!["overlay_surface".into()],
            fallback: ExtensionFallbackPolicy::PreserveOpaquePayload,
            configuration_schema: json!({"type":"object","properties":{"edge":{"type":"string"}}}),
            required_capabilities: vec!["pointer_input".into()],
            incompatible_capabilities: vec!["exclusive_fullscreen".into()],
        })
        .unwrap();
    registry
        .register_modality(ModalityExtension {
            descriptor: descriptor("window:native"),
            validate_config: Arc::new(|_| Ok(())),
            operations: vec!["float".into(), "dock_back".into()],
            capabilities: vec!["native_window".into()],
            fallback: ExtensionFallbackPolicy::RejectUnavailable,
            configuration_schema: json!({"type":"object"}),
            required_capabilities: vec!["native_viewport".into()],
            incompatible_capabilities: Vec::new(),
        })
        .unwrap();
    registry
        .register_command(CommandExtension {
            descriptor: descriptor("commands:activate"),
            execute: Arc::new(|_, _| {
                Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Test command is introspection-only.",
                    "commands:activate",
                ))
            }),
        })
        .unwrap();
    registry
        .register_transition(TransitionExtension {
            descriptor: descriptor("transitions:fade"),
            duration: Arc::new(|request| request.duration_ms),
            required_capabilities: vec!["vector_timeline".into()],
            incompatible_capabilities: vec!["exclusive_fullscreen".into()],
        })
        .unwrap();
    registry
        .register_persistence_codec(PersistenceCodecExtension {
            descriptor: descriptor("codecs:json"),
            encode: Arc::new(|value| Ok(value.clone())),
            decode: Arc::new(|value| Ok(value.clone())),
        })
        .unwrap();
    registry
        .register_host_adapter(HostAdapterExtension {
            descriptor: descriptor("hosts:egui"),
            validate_diff: Arc::new(|_| Ok(())),
        })
        .unwrap();
    registry
}

fn host() -> HostCapabilities {
    HostCapabilities {
        supported: BTreeSet::from(["native_viewport".into(), "native_window".into()]),
    }
}

fn layout() -> LayoutDocument {
    let mut document = LayoutDocument::new("capability-layout", "workspace");
    document.panels = Arc::new(vec![PanelDescriptor::new("editor", "Editor")]);
    document.root = Some(Node::panel("editor-node", "editor"));
    document
}

#[test]
fn capability_snapshot_is_sorted_typed_and_reports_missing_host_requirements() {
    let snapshot = registry().capability_snapshot(&host());
    assert_eq!(
        snapshot
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "codecs:json",
            "commands:activate",
            "hosts:egui",
            "layout:overlay",
            "panels:editor",
            "transitions:fade",
            "window:native",
        ]
    );
    let panel = snapshot
        .iter()
        .find(|entry| entry.id == "panels:editor")
        .unwrap();
    assert_eq!(panel.kind, ExtensionKind::PanelProvider);
    assert_eq!(panel.operations, vec!["validate_panel"]);
    let transition = snapshot
        .iter()
        .find(|entry| entry.id == "transitions:fade")
        .unwrap();
    assert_eq!(transition.kind, ExtensionKind::Transition);
    assert_eq!(transition.operations, vec!["duration_ms"]);
    assert_eq!(transition.supported_on_host, Some(false));
    assert_eq!(transition.missing_capabilities, vec!["vector_timeline"]);
    assert!(
        transition
            .reason_unavailable
            .as_deref()
            .unwrap()
            .contains("vector_timeline")
    );
    let codec = snapshot
        .iter()
        .find(|entry| entry.id == "codecs:json")
        .unwrap();
    assert_eq!(codec.kind, ExtensionKind::PersistenceCodec);
    assert_eq!(codec.operations, vec!["decode", "encode"]);
    let host_adapter = snapshot
        .iter()
        .find(|entry| entry.id == "hosts:egui")
        .unwrap();
    assert_eq!(host_adapter.kind, ExtensionKind::HostAdapter);
    assert_eq!(host_adapter.operations, vec!["validate_diff"]);
    let overlay = snapshot
        .iter()
        .find(|entry| entry.id == "layout:overlay")
        .unwrap();
    assert_eq!(overlay.kind, ExtensionKind::Modality);
    assert_eq!(overlay.supported_on_host, Some(false));
    assert_eq!(overlay.missing_capabilities, vec!["pointer_input"]);
    assert!(
        overlay
            .reason_unavailable
            .as_deref()
            .unwrap()
            .contains("pointer_input")
    );
    assert_eq!(
        overlay.configuration_schema.as_ref().unwrap()["properties"]["edge"]["type"],
        "string"
    );
    assert_eq!(
        overlay.fallback,
        Some(ExtensionFallbackPolicy::PreserveOpaquePayload)
    );
    let native = snapshot
        .iter()
        .find(|entry| entry.id == "window:native")
        .unwrap();
    assert_eq!(native.supported_on_host, Some(true));
    assert!(native.missing_capabilities.is_empty());
    assert_eq!(snapshot[0].kind, ExtensionKind::PersistenceCodec);
}

#[test]
fn capability_urne_projection_is_deterministic_and_has_no_dangling_edges() {
    let document = layout();
    let registry = registry();
    let host = host();
    let first =
        UrneProjection::from_layout_with_capabilities(&document, &registry, "windows-egui", &host)
            .unwrap();
    let second =
        UrneProjection::from_layout_with_capabilities(&document, &registry, "windows-egui", &host)
            .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.version, URNE_VERSION);
    let ids: BTreeSet<_> = first.nodes.iter().map(|node| node.id.as_str()).collect();
    assert!(
        first
            .edges
            .iter()
            .all(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
    );
    assert!(first.edges.iter().any(|edge| {
        edge.edge_type == "supports_operation" && edge.target.starts_with("operation:")
    }));
    let mut indegree = ids
        .iter()
        .map(|id| (*id, 0usize))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut outgoing = std::collections::BTreeMap::<&str, Vec<&str>>::new();
    for edge in &first.edges {
        *indegree.get_mut(edge.target.as_str()).unwrap() += 1;
        outgoing
            .entry(edge.source.as_str())
            .or_default()
            .push(edge.target.as_str());
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(id) = ready.pop() {
        visited += 1;
        for target in outgoing.get(id).into_iter().flatten() {
            let degree = indegree.get_mut(target).unwrap();
            *degree -= 1;
            if *degree == 0 {
                ready.push(target);
            }
        }
    }
    assert_eq!(
        visited,
        ids.len(),
        "URNE relationship graph must be acyclic"
    );
    let transition = first
        .nodes
        .iter()
        .find(|node| node.node_type == "transition_capability")
        .unwrap();
    assert_eq!(transition.metadata["operation"], "duration_ms");
    assert_eq!(transition.metadata["host_availability"], "unsupported");
    assert_eq!(transition.metadata["request_schema"]["type"], "object");
    assert_eq!(
        transition.metadata["request_schema"]["additionalProperties"],
        false
    );
    assert!(
        transition.metadata["request_schema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "host_capabilities")
    );
    assert_eq!(
        transition.metadata["missing_host_capabilities"][0],
        "vector_timeline"
    );
    assert!(
        transition.metadata["request_fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "host_capabilities")
    );
    assert_eq!(transition.metadata["callback_invoked"], false);
    assert!(
        first.edges.iter().any(|edge| {
            edge.edge_type == "provides_transition" && edge.target == transition.id
        })
    );
    assert!(
        first
            .edges
            .iter()
            .any(|edge| { edge.edge_type == "requires" && edge.target.starts_with("capability:") })
    );
    assert!(first.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_host_capability" && diagnostic.node_id.starts_with("provider:")
    }));
    assert!(
        first
            .nodes
            .iter()
            .any(|node| node.id.starts_with("provider:") && node.node_type == "extension_provider")
    );
    let native_ops = first
        .nodes
        .iter()
        .find(|node| {
            node.node_type == "builtin_operation_support"
                && node.metadata["operation_family_id"] == "floating_window_control"
        })
        .unwrap();
    assert_eq!(native_ops.metadata["core_available"], true);
    assert_eq!(native_ops.metadata["host_available"], true);
    assert_eq!(native_ops.metadata["missing_host_capabilities"], json!([]));
    assert!(
        native_ops.metadata["applicable_target_kinds"]
            .as_array()
            .unwrap()
            .contains(&json!("window"))
    );
    assert!(
        !native_ops.metadata["constraints"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        native_ops.metadata["request_encoding"]["discriminator"],
        "operation"
    );
    assert_eq!(
        native_ops.metadata["operation_schemas"]["float_panel"]["properties"]["operation"]["const"],
        "float_panel"
    );
    let layout_ops = first
        .nodes
        .iter()
        .find(|node| {
            node.node_type == "builtin_operation_support"
                && node.metadata["operation_family_id"] == "layout_edit"
        })
        .unwrap();
    assert_eq!(layout_ops.metadata["host_available"], true);
    assert_eq!(
        layout_ops.metadata["operation_schemas"]["close_panel"]["properties"]["operation"]["const"],
        "close_panel"
    );
    assert!(first.edges.iter().any(|edge| {
        edge.edge_type == "supports_operation"
            && edge.source == native_ops.id
            && edge.target.starts_with("builtin-operation:")
    }));
}

#[test]
fn built_in_support_reports_missing_native_and_recovery_requirements() {
    let host = HostCapabilities::default();
    let support = builtin_operation_support(&host).unwrap();
    let floating = support
        .iter()
        .find(|entry| entry.id == "floating_window_control")
        .unwrap();
    assert!(floating.core_available);
    assert!(!floating.host_available);
    assert_eq!(
        floating.required_any_host_capabilities,
        vec!["native_viewport", "embedded_viewport"]
    );
    assert_eq!(
        floating.missing_host_capabilities,
        vec!["embedded_viewport", "native_viewport"]
    );
    assert!(floating.reason.contains("native_viewport"));

    let recovery = support
        .iter()
        .find(|entry| entry.id == "floating_window_recovery")
        .unwrap();
    assert_eq!(
        recovery.missing_host_capabilities,
        vec!["monitor_dpi_query", "native_viewport"]
    );
    let edit = support
        .iter()
        .find(|entry| entry.id == "layout_edit")
        .unwrap();
    assert!(edit.core_available && edit.host_available);
    assert!(edit.missing_host_capabilities.is_empty());

    let embedded = HostCapabilities {
        supported: BTreeSet::from(["embedded_viewport".into()]),
    };
    let embedded_support = builtin_operation_support(&embedded).unwrap();
    let floating = embedded_support
        .iter()
        .find(|entry| entry.id == "floating_window_control")
        .unwrap();
    assert!(floating.host_available);
    assert!(floating.missing_host_capabilities.is_empty());
}

#[test]
fn built_in_support_catalog_covers_every_operation_variant_once() {
    let mut actual: Vec<_> = builtin_operation_support(&HostCapabilities::default())
        .unwrap()
        .into_iter()
        .flat_map(|family| family.operation_names)
        .collect();
    let mut expected = vec![
        "set_tab_pinned",
        "convert_container",
        "set_stack_expanded",
        "raise_layer",
        "lower_layer",
        "select_layer",
        "tab_panel",
        "center_panel",
        "center_panel_group",
        "swap_panels",
        "root_panel",
        "split_panel",
        "float_panel",
        "float_group",
        "reorder_panel",
        "close_panel",
        "close_panel_group",
        "reopen_panel",
        "resize_split",
        "set_group_minimized",
        "auto_hide",
        "pin_panel",
        "maximize",
        "restore_maximized",
        "select_tab",
        "set_picture_in_picture",
        "set_scratchpad",
        "activate_document_group",
        "update_extension_payload",
        "add_panel",
        "add_group",
        "update_floating_window",
        "activate_floating_window",
        "recover_floating_window",
        "dock_floating_window",
        "close_floating_window",
        "clear_layout",
    ];
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn built_in_operation_json_schema_matches_the_tagged_wire_shape() {
    let schema = builtin_operation_json_schema();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    let branches = schema["oneOf"].as_array().unwrap();
    let mut schema_operations: Vec<_> = branches
        .iter()
        .map(|branch| {
            branch["properties"]["operation"]["const"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    let mut catalog_operations: Vec<_> = builtin_operation_support(&HostCapabilities::default())
        .unwrap()
        .into_iter()
        .flat_map(|family| family.operation_names)
        .collect();
    schema_operations.sort();
    catalog_operations.sort();
    assert_eq!(schema_operations, catalog_operations);

    let restore = branches
        .iter()
        .find(|branch| branch["properties"]["operation"]["const"] == "restore_maximized")
        .unwrap();
    assert_eq!(restore["required"], serde_json::json!(["operation"]));
    assert!(restore["properties"].get("arguments").is_none());
    let float = branches
        .iter()
        .find(|branch| branch["properties"]["operation"]["const"] == "float_panel")
        .unwrap();
    assert_eq!(
        float["properties"]["arguments"]["properties"]["geometry"],
        serde_json::json!({"$ref": "#/$defs/Rect"})
    );
    let center_group = branches
        .iter()
        .find(|branch| branch["properties"]["operation"]["const"] == "center_panel_group")
        .unwrap();
    assert_eq!(
        center_group["properties"]["arguments"]["properties"]["panels"]["minItems"],
        1
    );
    assert_eq!(
        center_group["properties"]["arguments"]["properties"]["panels"]["maxItems"],
        1023
    );
    assert_eq!(
        schema["$defs"]["Rect"],
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer"},
                "y": {"type": "integer"},
                "width": {"type": "integer"},
                "height": {"type": "integer"}
            },
            "required": ["x", "y", "width", "height"],
            "additionalProperties": true
        })
    );
    assert_eq!(
        serde_json::to_value(Operation::FloatPanel {
            panel: "panel-a".into(),
            window: "window-a".into(),
            geometry: Rect {
                x: 1,
                y: 2,
                width: 3,
                height: 4
            },
            monitor: "monitor-a".to_owned(),
            scale: 1.0,
        })
        .unwrap()["arguments"]["geometry"],
        serde_json::json!({"x": 1, "y": 2, "width": 3, "height": 4})
    );
    assert_eq!(
        serde_json::to_value(Operation::ConvertContainer {
            container: "container-a".into(),
            target: ContainerModality::ShadeStack,
        })
        .unwrap()["arguments"]["target"],
        serde_json::json!({"kind": "shade_stack"})
    );
    assert_eq!(
        serde_json::to_value(ContainerModality::NodeGraph).unwrap(),
        serde_json::json!({"kind": "node_graph"})
    );
    assert_eq!(
        serde_json::to_value(ContainerModality::LayoutStrategy {
            strategy: LayoutStrategy::BinarySpacePartition,
            gap: 8,
        })
        .unwrap(),
        serde_json::json!({
            "kind": "layout_strategy",
            "configuration": {"strategy": "binary_space_partition", "gap": 8}
        })
    );
    assert_eq!(
        serde_json::to_value(ContainerModality::ScrollableColumns {
            gap: 12,
            width: 260,
            minimum_width: 120,
            maximum_width: Some(520),
        })
        .unwrap(),
        serde_json::json!({
            "kind": "scrollable_columns",
            "configuration": {
                "gap": 12,
                "width": 260,
                "minimum_width": 120,
                "maximum_width": 520
            }
        })
    );
}

#[test]
fn operation_catalog_joins_request_schema_with_per_operation_host_support() {
    let catalog = builtin_operation_catalog(&HostCapabilities::default()).unwrap();
    assert_eq!(catalog.len(), 37);
    assert!(catalog.windows(2).all(|pair| pair[0].name < pair[1].name));

    let close = catalog
        .iter()
        .find(|entry| entry.name == "close_panel")
        .unwrap();
    assert_eq!(close.family, "layout_edit");
    assert!(close.core_available);
    assert!(close.host_available);
    assert!(close.missing_host_capabilities.is_empty());
    assert_eq!(
        close.request_schema["properties"]["operation"]["const"],
        "close_panel"
    );

    let float = catalog
        .iter()
        .find(|entry| entry.name == "float_panel")
        .unwrap();
    assert_eq!(float.family, "floating_window_control");
    assert!(float.core_available);
    assert!(!float.host_available);
    assert_eq!(
        float.missing_host_capabilities,
        vec!["embedded_viewport", "native_viewport"]
    );
    assert!(!float.reason.is_empty());
    let wire = serde_json::to_vec(&catalog).unwrap();
    let restored: Vec<BuiltinOperationDescriptor> = serde_json::from_slice(&wire).unwrap();
    assert_eq!(restored, catalog);
}

#[test]
fn renderer_report_projects_supported_paths_and_effect_limits_deterministically() {
    let report = RendererCapabilityReport {
        renderer_id: "doofusdock-render-egui".into(),
        observations: vec![
            RendererCapabilityObservation {
                capability_id: "vector_chrome".into(),
                availability: RendererCapabilityAvailability::Available,
                details: "Vector chrome is the baseline.".into(),
            },
            RendererCapabilityObservation {
                capability_id: "application_shader_effects".into(),
                availability: RendererCapabilityAvailability::NotImplemented,
                details: "No application shader effect pipeline is implemented.".into(),
            },
        ],
    };
    report.validate().unwrap();
    let first = UrneProjection::from_layout(&layout())
        .unwrap()
        .with_renderer_report(&report)
        .unwrap();
    let second = UrneProjection::from_layout(&layout())
        .unwrap()
        .with_renderer_report(&report)
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first
            .clone()
            .with_renderer_report(&report)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let effect = first
        .nodes
        .iter()
        .find(|node| {
            node.node_type == "renderer_capability"
                && node.metadata["capability_id"] == "application_shader_effects"
        })
        .unwrap();
    assert_eq!(effect.metadata["availability"], "not_implemented");
    assert!(
        first
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "renderer_capability_not_implemented")
    );
    let ids: BTreeSet<_> = first.nodes.iter().map(|node| node.id.as_str()).collect();
    assert!(
        first
            .edges
            .iter()
            .all(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
    );
}

#[test]
fn renderer_report_rejects_duplicate_and_unbounded_observations() {
    let mut report = RendererCapabilityReport {
        renderer_id: "renderer".into(),
        observations: vec![RendererCapabilityObservation {
            capability_id: "vector_chrome".into(),
            availability: RendererCapabilityAvailability::Available,
            details: "available".into(),
        }],
    };
    report.observations.push(report.observations[0].clone());
    assert_eq!(
        report.validate().unwrap_err().code,
        ErrorCode::InvalidLayout
    );
    report.observations.clear();
    report.observations.push(RendererCapabilityObservation {
        capability_id: "invalid capability".into(),
        availability: RendererCapabilityAvailability::Available,
        details: "bad id".into(),
    });
    assert_eq!(
        report.validate().unwrap_err().code,
        ErrorCode::InvalidLayout
    );
}

#[test]
fn operation_preflight_is_read_only_and_separates_core_from_host_support() {
    let document = layout();
    let before = document.clone();
    let operation = Operation::FloatPanel {
        panel: "editor".into(),
        window: "editor-window".into(),
        geometry: Rect {
            x: 10,
            y: 20,
            width: 400,
            height: 300,
        },
        monitor: "primary".into(),
        scale: 1.0,
    };
    let unsupported =
        preflight_operation(&document, &operation, &HostCapabilities::default()).unwrap();
    assert!(unsupported.core_available);
    assert!(!unsupported.host_available);
    assert!(!unsupported.supported);
    assert_eq!(
        unsupported.missing_host_capabilities,
        vec!["embedded_viewport", "native_viewport"]
    );
    assert!(
        unsupported
            .reason
            .contains("at least one of these capabilities")
    );
    assert_eq!(
        unsupported.target_ids,
        vec!["panel:editor", "window:editor-window"]
    );
    assert_eq!(unsupported.source_revision, document.revision);
    assert_eq!(document, before);
    assert_eq!(
        serde_json::from_value::<OperationPreflightReport>(
            serde_json::to_value(&unsupported).unwrap()
        )
        .unwrap(),
        unsupported
    );

    let supported = preflight_operation(
        &document,
        &operation,
        &HostCapabilities {
            supported: BTreeSet::from(["native_viewport".into()]),
        },
    )
    .unwrap();
    assert!(supported.supported);
    assert!(supported.rejection_code.is_none());
    supported.validate().unwrap();

    let embedded = preflight_operation(
        &document,
        &operation,
        &HostCapabilities {
            supported: BTreeSet::from(["embedded_viewport".into()]),
        },
    )
    .unwrap();
    assert!(embedded.supported);
}

#[test]
fn operation_preflight_reports_policy_errors_and_projects_with_revision_binding() {
    let document = layout();
    let invalid = preflight_operation(
        &document,
        &Operation::ClosePanel {
            panel: "missing-panel".into(),
        },
        &HostCapabilities::default(),
    )
    .unwrap();
    assert!(!invalid.core_available);
    assert_eq!(invalid.rejection_code.as_deref(), Some("not_found"));
    assert!(invalid.reason.contains("Panel descriptor"));

    let operation = Operation::FloatPanel {
        panel: "editor".into(),
        window: "editor-window".into(),
        geometry: Rect {
            x: 10,
            y: 20,
            width: 400,
            height: 300,
        },
        monitor: "primary".into(),
        scale: 1.0,
    };
    let report = preflight_operation(&document, &operation, &HostCapabilities::default()).unwrap();
    let projection = UrneProjection::from_layout(&document)
        .unwrap()
        .with_operation_preflight_report(&report)
        .unwrap();
    assert!(
        projection
            .nodes
            .iter()
            .any(|node| node.node_type == "operation_preflight"
                && node.metadata["supported"] == false)
    );
    assert!(projection.edges.iter().any(|edge| {
        edge.edge_type == "missing_host_capability" && edge.target.starts_with("capability:")
    }));
    let mut changed = document.clone();
    changed.revision += 1;
    assert_eq!(
        UrneProjection::from_layout(&changed)
            .unwrap()
            .with_operation_preflight_report(&report)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
}

#[test]
fn host_report_urne_separates_api_availability_from_verification_evidence() {
    let document = layout();
    let registry = registry();
    let host = HostCapabilities {
        supported: BTreeSet::from(["native_viewport".into(), "native_close_requests".into()]),
    };
    let report = HostCapabilityReport {
        host_id: "windows-egui".into(),
        observations: vec![
            HostCapabilityObservation {
                capability_id: "native_viewport".into(),
                availability: HostCapabilityAvailability::Available,
                evidence: HostCapabilityEvidence::PhysicalSmokeTest,
                details: "Detached viewport observed in the single-display Windows smoke run."
                    .into(),
            },
            HostCapabilityObservation {
                capability_id: "native_close_requests".into(),
                availability: HostCapabilityAvailability::Available,
                evidence: HostCapabilityEvidence::SyntheticContract,
                details: "Canonical close request path is contract-tested.".into(),
            },
            HostCapabilityObservation {
                capability_id: "native_cross_viewport_drag".into(),
                availability: HostCapabilityAvailability::NotImplemented,
                evidence: HostCapabilityEvidence::NotVerified,
                details: "Cross-viewport drag is not implemented.".into(),
            },
        ],
    };
    let first =
        UrneProjection::from_layout_with_host_report(&document, &registry, &host, &report).unwrap();
    let second =
        UrneProjection::from_layout_with_host_report(&document, &registry, &host, &report).unwrap();
    assert_eq!(first, second);
    let native = first
        .nodes
        .iter()
        .find(|node| node.node_type == "capability" && node.label == "native_viewport")
        .unwrap();
    assert_eq!(native.metadata["availability"], "available");
    assert_eq!(
        native.metadata["verification_evidence"],
        "physical_smoke_test"
    );
    let unsupported = first
        .nodes
        .iter()
        .find(|node| node.node_type == "capability" && node.label == "native_cross_viewport_drag")
        .unwrap();
    assert_eq!(unsupported.metadata["availability"], "not_implemented");
    assert!(first.edges.iter().any(|edge| {
        edge.edge_type == "reports" && edge.target.starts_with("host-capability-report:")
    }));
    let ids: BTreeSet<_> = first.nodes.iter().map(|node| node.id.as_str()).collect();
    assert!(
        first
            .edges
            .iter()
            .all(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
    );
}

#[test]
fn host_report_rejects_capability_availability_mismatch_and_duplicates() {
    let report = HostCapabilityReport {
        host_id: "test-host".into(),
        observations: vec![HostCapabilityObservation {
            capability_id: "native_viewport".into(),
            availability: HostCapabilityAvailability::Available,
            evidence: HostCapabilityEvidence::ApiDeclared,
            details: "available".into(),
        }],
    };
    let absent = HostCapabilities::default();
    assert_eq!(
        report.validate(&absent).unwrap_err().code,
        ErrorCode::InvalidLayout
    );
    let mismatched = HostCapabilities {
        supported: BTreeSet::from(["native_viewport".into()]),
    };
    let mut duplicate = report.clone();
    duplicate
        .observations
        .push(duplicate.observations[0].clone());
    assert_eq!(
        duplicate.validate(&mismatched).unwrap_err().code,
        ErrorCode::InvalidLayout
    );
}

#[test]
fn capability_projection_rejects_an_unidentified_host_context() {
    let error = UrneProjection::from_capabilities(&registry(), "", &host()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidId);
}

#[test]
fn capability_projection_rejects_invalid_or_unbounded_host_capabilities() {
    let invalid = HostCapabilities {
        supported: BTreeSet::from(["pointer input".into()]),
    };
    let error =
        UrneProjection::from_capabilities(&registry(), "windows-egui", &invalid).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidLayout);

    let oversized = HostCapabilities {
        supported: BTreeSet::from(["x".repeat(257)]),
    };
    let error =
        UrneProjection::from_capabilities(&registry(), "windows-egui", &oversized).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidLayout);
}

#[test]
fn profile_catalog_urne_projection_contains_only_declared_profiles_and_regions() {
    let mut profiles = builtin_workspace_profiles();
    profiles.truncate(2);
    let mut catalog = WorkspaceProfileCatalog::from_profiles(profiles).unwrap();
    catalog.default_profile = catalog.profiles.keys().next().cloned();
    let first = UrneProjection::from_profile_catalog(&catalog).unwrap();
    let second = UrneProjection::from_profile_catalog(&catalog).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first
            .nodes
            .iter()
            .filter(|node| node.node_type == "layout_profile")
            .count(),
        2
    );
    assert!(
        first
            .nodes
            .iter()
            .any(|node| node.node_type == "semantic_region")
    );
    assert!(
        first
            .edges
            .iter()
            .any(|edge| edge.edge_type == "declares_region")
    );
    let ids: BTreeSet<_> = first.nodes.iter().map(|node| node.id.as_str()).collect();
    assert!(
        first
            .edges
            .iter()
            .all(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
    );
}
