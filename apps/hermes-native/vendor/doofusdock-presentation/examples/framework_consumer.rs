//! GUI-free application consumer using only core and presentation APIs.
use doofusdock_core::*;
use doofusdock_presentation::{
    Box2, Easing, Morph, MorphTimingPolicy,
    effects::{EffectContext, evaluate_scene_effects},
};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

const WORKBENCH_THEME: &str = include_str!("../../../themes/doofusdock-workbench/theme.json");

struct TemporaryDirectory(std::path::PathBuf);

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn transition_registry() -> Result<ExtensionRegistry, DockError> {
    let version = ApiVersion { major: 1, minor: 0 };
    let mut registry = ExtensionRegistry::default();
    registry.register_transition(TransitionExtension {
        descriptor: ExtensionDescriptor {
            namespaced_id: "consumer:layout-morph".into(),
            version,
            api: CompatibilityRange {
                minimum: version,
                maximum: version,
            },
            ..ExtensionDescriptor::default()
        },
        duration: Arc::new(|request| request.duration_ms),
        required_capabilities: vec![],
        incompatible_capabilities: vec![],
    })?;
    registry.freeze();
    registry.activate("consumer:layout-morph")?;
    Ok(registry)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a canonical workspace and declare its semantic regions through a
    // reusable built-in profile.
    let mut source = LayoutDocument::new("consumer-workspace", "consumer-app");
    let panels = ["navigation", "editor", "inspector", "output"];
    Arc::make_mut(&mut source.panels).extend(
        panels
            .iter()
            .map(|id| PanelDescriptor::new(*id, format!("{} panel", id))),
    );
    source.root = Some(Node::tabs(
        "initial-tabs",
        panels
            .iter()
            .map(|id| Node::panel(format!("{id}-node"), *id))
            .collect(),
        1,
    ));

    let profile = builtin_workspace_profiles()
        .into_iter()
        .find(|profile| profile.id == "workbench")
        .expect("workbench profile is part of the public built-in catalog");
    let assignments: BTreeMap<_, _> = [
        ("navigation", "navigation"),
        ("main", "editor"),
        ("inspector", "inspector"),
        ("output", "output"),
    ]
    .into_iter()
    .map(|(region, panel)| (region.to_owned(), vec![PanelId::from(panel)]))
    .collect();
    let profile_application = apply_workspace_profile(&source, &profile, &assignments)?;

    // Resolve a toolkit-neutral transition request without changing canonical
    // state. The host is free to render these snapshots with its own adapter.
    let transitions = transition_registry()?;
    let policy = MorphTimingPolicy {
        transition: "consumer:layout-morph".into(),
        requested_duration_ms: 500,
        maximum_duration_ms: 500,
        reduced_motion: false,
        host_capabilities: vec![],
        easing: Easing::Smooth,
    };
    let morph = Morph::new_with_provider(
        &source,
        &profile_application.document,
        Box2 {
            x: 0.0,
            y: 0.0,
            w: 1440.0,
            h: 900.0,
        },
        0.0,
        &policy,
        &transitions,
    )?;
    let midpoint = morph.sample(0.25)?;
    assert!(!midpoint.panels.is_empty());
    assert_eq!(
        source.revision, 0,
        "transition sampling is presentation-only"
    );

    // Apply the profile, negotiate a structural modality change, then float,
    // undo/redo, and dock a panel through the canonical service.
    let mut service = DockingService::new(source, 32)?;
    service.apply_profile(0, &profile, &assignments)?;
    let host = HostCapabilities {
        supported: [
            "workspace:tabs".to_owned(),
            "window:native".to_owned(),
            "native_viewport".to_owned(),
        ]
        .into_iter()
        .collect(),
    };
    let snapshot = service.snapshot();
    let root_id = snapshot.document.root.as_ref().unwrap().id.clone();
    let layered = plan_modality_change(
        &snapshot.document,
        ModalityChange::Container {
            container: root_id,
            target: ContainerModality::Tabbed { active_index: 0 },
        },
        &host,
    )?
    .commit(&mut service)?;

    let floated = service.transact(
        layered.document.revision,
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
    let undone = service.undo(floated.document.revision)?;
    let redone = service.redo(undone.document.revision)?;
    let docked = service.transact(
        redone.document.revision,
        "dock-inspector".into(),
        [Operation::DockFloatingWindow {
            window: "inspector-window".into(),
            target_panel: Some("editor".into()),
        }],
    )?;
    validate_layout(&docked.document)?;

    // Apply a bundled theme through the public resolver. The host consumes the
    // resolved tokens; this example intentionally starts no GUI.
    let manifest = ThemeManifest::parse_json(WORKBENCH_THEME)?;
    let resolved_theme = resolve_theme(
        &manifest,
        "dark",
        &BTreeMap::new(),
        &ThemeAdapterCapabilities::default(),
        None,
    )?;
    assert!(!resolved_theme.tokens.is_empty());

    // Persist and restore canonical state, then expose read-only host and URNE
    // capability projections to an independent application.
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "doofusdock-framework-consumer-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir(&directory)?;
    let _temporary_directory = TemporaryDirectory(directory.clone());
    let mut store = LayoutStore::new(&directory, PersistenceLimits::default());
    store.save(&docked.document)?;
    let restored = store.recover(&docked.document.id)?;
    assert_eq!(
        serialize_layout(&restored)?,
        serialize_layout(&docked.document)?
    );

    let capabilities = transitions.capability_snapshot(&host);
    assert!(capabilities.iter().any(|capability| {
        capability.id == "consumer:layout-morph" && capability.kind == ExtensionKind::Transition
    }));
    let operation_catalog = builtin_operation_catalog(&host)?;
    assert_eq!(operation_catalog.len(), 35);
    assert!(operation_catalog.iter().any(|operation| {
        operation.name == "float_panel"
            && operation.host_available
            && operation.request_schema["properties"]["operation"]["const"] == "float_panel"
    }));
    let projection = UrneProjection::from_layout_with_capabilities(
        &restored,
        &transitions,
        "consumer-native-host",
        &host,
    )?;
    assert!(!projection.nodes.is_empty());
    let scene = evaluate_scene_effects(
        "wall_of_windows",
        EffectContext {
            time_seconds: 1.25,
            progress: 0.4,
            viewport: Box2 {
                x: 0.0,
                y: 0.0,
                w: 1440.0,
                h: 900.0,
            },
            seed: 42,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.2,
            canonical_doc: &restored,
        },
    )?;
    assert_eq!(scene.decorative.len(), 50);
    let scene_after_seek = evaluate_scene_effects(
        "wall_of_windows",
        EffectContext {
            time_seconds: 0.3,
            progress: 0.1,
            viewport: scene.viewport,
            seed: 42,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.0,
            canonical_doc: &restored,
        },
    )?;
    let scene_repeat = evaluate_scene_effects(
        "wall_of_windows",
        EffectContext {
            time_seconds: 1.25,
            progress: 0.4,
            viewport: scene.viewport,
            seed: 42,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.2,
            canonical_doc: &restored,
        },
    )?;
    assert_ne!(scene, scene_after_seek, "effects support stateless seek");
    assert_eq!(scene, scene_repeat, "effect samples are deterministic");
    assert_eq!(restored.revision, 6, "scene evaluation is read-only");
    println!(
        "framework consumer ready: revision={}, panels={}, floating_windows={}, theme={}, transition_samples={}, capabilities={}, operations={}, urne_nodes={}, scene_items={}",
        restored.revision,
        restored.panels.len(),
        restored.floating_windows.len(),
        resolved_theme.theme_id,
        midpoint.panels.len(),
        capabilities.len(),
        operation_catalog.len(),
        projection.nodes.len(),
        scene.decorative.len()
    );
    Ok(())
}
