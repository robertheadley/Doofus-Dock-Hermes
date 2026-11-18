//! A minimal native-application consumer using only DoofusDock's public core API.
use doofusdock_core::*;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut document = LayoutDocument::new("consumer-workspace", "consumer-app");
    Arc::make_mut(&mut document.panels).extend([
        PanelDescriptor::new("explorer", "Explorer"),
        PanelDescriptor::new("editor", "Editor"),
    ]);
    document.root = Some(Node::tabs(
        "main-tabs",
        vec![
            Node::panel("explorer-node", "explorer"),
            Node::panel("editor-node", "editor"),
        ],
        1,
    ));

    let mut workspace = DockingService::new(document, 16)?;
    let structural_host = HostCapabilities {
        supported: ["workspace:tabs".into(), "workspace:docking".into()]
            .into_iter()
            .collect(),
    };
    for target in [
        ContainerModality::Split {
            orientation: Orientation::Horizontal,
            weights: vec![],
        },
        ContainerModality::Tabbed { active_index: 1 },
    ] {
        let snapshot = workspace.snapshot();
        let plan = plan_modality_change(
            &snapshot.document,
            ModalityChange::Container {
                container: "main-tabs".into(),
                target,
            },
            &structural_host,
        )?;
        assert_eq!(snapshot, workspace.snapshot());
        plan.commit(&mut workspace)?;
    }
    let composed = workspace.transact(
        workspace.snapshot().revision,
        "add-inspector".into(),
        [Operation::AddGroup {
            descriptor: PanelDescriptor::new("inspector", "Inspector"),
            target: Some("main-tabs".into()),
            orientation: Orientation::Vertical,
            before: false,
            ratio: 0.25,
        }],
    )?;

    let floated = workspace.transact(
        composed.document.revision,
        "float-inspector".into(),
        [Operation::FloatPanel {
            panel: "inspector".into(),
            window: "inspector-window".into(),
            geometry: Rect {
                x: 80,
                y: 60,
                width: 520,
                height: 420,
            },
            monitor: "primary".into(),
            scale: 1.0,
        }],
    )?;
    assert_eq!(floated.document.floating_windows.len(), 1);

    let undone = workspace.undo(floated.document.revision)?;
    assert!(undone.document.floating_windows.is_empty());
    let redone = workspace.redo(undone.document.revision)?;
    assert_eq!(redone.document.floating_windows.len(), 1);

    let docked = workspace.transact(
        redone.document.revision,
        "dock-inspector".into(),
        [Operation::DockFloatingWindow {
            window: "inspector-window".into(),
            target_panel: Some("editor".into()),
        }],
    )?;
    validate_layout(&docked.document)?;

    // Transient presentation is negotiated and activated without committing a
    // layout revision or entering canonical undo history.
    let transient_host = HostCapabilities {
        supported: ["overlay.modal".into(), "overlay.drawer".into()]
            .into_iter()
            .collect(),
    };
    let before_surface = workspace.snapshot();
    let surface_plan = plan_transient_presentation(
        &before_surface.document,
        TransientPresentationRequest {
            id: "consumer-inspector-drawer".into(),
            title: "Inspector".into(),
            kind: TransientPresentationKind::Drawer {
                edge: DockEdge::Right,
                modal: true,
            },
        },
        &transient_host,
    )?;
    assert_eq!(surface_plan.preview().id, "consumer-inspector-drawer");
    let surface_intent = surface_plan.activate(&before_surface.document)?;
    assert_eq!(
        surface_intent.kind,
        TransientPresentationKind::Drawer {
            edge: DockEdge::Right,
            modal: true,
        }
    );
    assert_eq!(before_surface, workspace.snapshot());

    // Public consumers can select the existing node-graph provider through the
    // negotiated core API and preserve its payload over spatial persistence.
    let modality_host = HostCapabilities {
        supported: ["workspace:node_graph".into(), "workspace:spatial".into()]
            .into_iter()
            .collect(),
    };
    let graph_plan = plan_modality_change(
        &workspace.snapshot().document,
        ModalityChange::Container {
            container: "main-tabs".into(),
            target: ContainerModality::NodeGraph,
        },
        &modality_host,
    )?;
    let node_graph = graph_plan.commit(&mut workspace)?.document;
    let spatial = plan_modality_change(
        &node_graph,
        ModalityChange::Container {
            container: "main-tabs".into(),
            target: ContainerModality::Spatial,
        },
        &modality_host,
    )?
    .commit(&mut workspace)?
    .document;
    let persisted_spatial = serialize_layout(&spatial)?;
    let restored_spatial = deserialize_layout(&persisted_spatial, PersistenceLimits::default())?;
    let graph_again = plan_modality_change(
        &spatial,
        ModalityChange::Container {
            container: "main-tabs".into(),
            target: ContainerModality::NodeGraph,
        },
        &modality_host,
    )?
    .commit(&mut workspace)?
    .document;
    assert_eq!(restored_spatial.root, spatial.root);
    assert_eq!(graph_again.root, node_graph.root);

    // A public consumer can enter persisted PiP through the same negotiated
    // transaction path, save/restore it, and undo/redo it as one step.
    let pip_host = HostCapabilities {
        supported: ["picture_in_picture".into()].into_iter().collect(),
    };
    let before_pip = workspace.snapshot();
    let pip_plan = plan_modality_change(
        &before_pip.document,
        ModalityChange::PictureInPicture {
            panel: "inspector".into(),
            window: "consumer-pip".into(),
            geometry: Rect {
                x: 720,
                y: 80,
                width: 320,
                height: 180,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
        &pip_host,
    )?;
    assert_eq!(workspace.snapshot(), before_pip);
    let pip = pip_plan.commit(&mut workspace)?.document;
    assert!(pip.floating_windows[0].picture_in_picture);
    let persisted_pip = serialize_layout(&pip)?;
    let restored_pip = deserialize_layout(&persisted_pip, PersistenceLimits::default())?;
    assert!(restored_pip.floating_windows[0].picture_in_picture);
    let undone_pip = workspace.undo(pip.revision)?;
    assert!(undone_pip.document.floating_windows.is_empty());
    let redone_pip = workspace.redo(undone_pip.document.revision)?;
    assert!(redone_pip.document.floating_windows[0].picture_in_picture);
    let docked_pip = workspace.transact(
        redone_pip.document.revision,
        "dock-pip-inspector".into(),
        [Operation::DockFloatingWindow {
            window: "consumer-pip".into(),
            target_panel: Some("editor".into()),
        }],
    )?;
    validate_layout(&docked_pip.document)?;

    let operation_schema = builtin_operation_json_schema();
    assert_eq!(operation_schema["oneOf"].as_array().unwrap().len(), 35);

    let serialized = serialize_layout(&docked_pip.document)?;
    let restored = deserialize_layout(&serialized, PersistenceLimits::default())?;
    assert_eq!(restored.panels.len(), 3);
    assert!(restored.floating_windows.is_empty());

    let host = HostCapabilities {
        supported: ["keyboard_input".into(), "pointer_input".into()]
            .into_iter()
            .collect(),
    };
    let extensions = ExtensionRegistry::default();
    let graph = UrneProjection::from_layout_with_capabilities(
        &restored,
        &extensions,
        "consumer-egui-host",
        &host,
    )?;

    println!(
        "consumer workspace ready: revision={}, panels={}, floating_windows={}, urne_nodes={}",
        restored.revision,
        restored.panels.len(),
        restored.floating_windows.len(),
        graph.nodes.len()
    );
    Ok(())
}
