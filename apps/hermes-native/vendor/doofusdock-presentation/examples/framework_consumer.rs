//! GUI-free application consumer using only core and presentation APIs.
use doofusdock_core::*;
use doofusdock_presentation::{
    Box2, Easing, Morph, MorphTimingPolicy, Path2D, PathCommand2D, SceneFrame,
    SceneGroupIdentityMap, SceneNode, ScenePointerPhase, ScenePointerRouter,
    effects::{EffectContext, evaluate_scene_effects},
};
use std::{
    collections::{BTreeMap, BTreeSet},
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

/// Minimal independent-host adapter. It records the request for its own event
/// loop and reports Queued until that host observes the native outcome.
struct ConsumerFocusAdapter {
    revision: u64,
    panel: PanelId,
    queued: Vec<PanelId>,
}

impl WindowRuleHostAdapter for ConsumerFocusAdapter {
    fn apply(
        &mut self,
        intent: &ResolvedRuleAction,
        expected_revision: u64,
    ) -> DockResult<HostIntentResult> {
        if expected_revision != self.revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Consumer host received a stale window-rule intent.",
                format!("expected={expected_revision}:observed={}", self.revision),
            ));
        }
        if !matches!(
            intent.action,
            WindowRuleAction::Focus(FocusBehavior::FocusPanel)
        ) {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Consumer host example only maps panel focus intents.",
                intent.rule_id.clone(),
            ));
        }
        self.queued.push(self.panel.clone());
        Ok(HostIntentResult {
            observed_revision: self.revision,
            outcome: HostIntentOutcome::Queued,
            detail: "consumer event-loop request queued; native outcome pending".into(),
        })
    }
}

fn verify_consumer_rule_loop_suppression(
    document: &LayoutDocument,
) -> Result<(), Box<dyn std::error::Error>> {
    let panel = PanelId::from("editor");
    let workspace = document.workspace.clone();
    let mut service = DockingService::new(document.clone(), 8)?;
    let subscription = service.subscribe_events()?;
    service.observe_host_event(
        document.revision,
        "consumer-user-focus".into(),
        HostObservation::PanelFocused {
            panel: panel.clone(),
            floating_window: None,
        },
    )?;

    let rules = WindowRuleSet {
        schema_version: 1,
        rules: vec![WindowRule {
            id: "consumer-pin-after-focus".into(),
            enabled: true,
            priority: 1,
            overrides: Vec::new(),
            event_kinds: BTreeSet::from([
                FrameworkEventKind::FocusChanged,
                FrameworkEventKind::LayoutCommitted,
            ]),
            predicate: WindowRulePredicate {
                panel_id: Some(panel.clone()),
                ..WindowRulePredicate::default()
            },
            actions: vec![WindowRuleAction::SetPinned { pinned: true }],
        }],
    };
    let first_batch = service.read_events(subscription, 16)?;
    let snapshot = service.snapshot();
    let context = WindowRuleContext::for_panel(
        &snapshot.document,
        &panel,
        HostCapabilities {
            supported: BTreeSet::from(["workspace:tabs".to_owned()]),
        },
    )?;
    let mut consumer = WindowRuleEventConsumer::default();
    let triggered =
        consumer.evaluate_batch(&rules, &first_batch, std::slice::from_ref(&context))?;
    if triggered.len() != 1 {
        return Err(format!("expected one focus-triggered rule, got {}", triggered.len()).into());
    }
    let plan = WindowRulePlan::prepare(triggered[0].evaluation.clone(), &context)?;
    let rule_command = CommandId::from("consumer-rule-pin-after-focus");
    let rule_commit = plan.commit(&mut service, rule_command.clone())?;
    consumer.suppress_command_at_revision(rule_command.clone(), rule_commit.resulting_revision)?;
    if service.snapshot().document.revision != document.revision + 1 {
        return Err("focus-triggered structural rule did not commit exactly once".into());
    }
    if !service.snapshot().document.pinned_panels.contains(&panel) {
        return Err("focus rule did not pin its target panel".into());
    }

    let generated = service.read_events(subscription, 16)?;
    if !generated.events.iter().any(|event| {
        event.command == rule_command && event.kind == FrameworkEventKind::LayoutCommitted
    }) {
        return Err("rule commit event was not observed by the consumer".into());
    }
    let follow_up = consumer.evaluate_batch(&rules, &generated, &[])?;
    if !follow_up.is_empty() {
        return Err("rule-origin commit re-entered the consumer event loop".into());
    }
    if service.snapshot().document.workspace != workspace {
        return Err("rule event consumer changed workspace identity".into());
    }
    Ok(())
}

fn verify_consumer_focus_intent_echo_suppression(
    document: &LayoutDocument,
) -> Result<(), Box<dyn std::error::Error>> {
    let panel = PanelId::from("editor");
    let capabilities = HostCapabilities {
        supported: BTreeSet::from(["window:focus".to_owned()]),
    };
    let context = WindowRuleContext::for_panel(document, &panel, capabilities)?;
    let rules = WindowRuleSet {
        schema_version: 1,
        rules: vec![WindowRule {
            id: "consumer-focus-echo".into(),
            enabled: true,
            priority: 1,
            overrides: Vec::new(),
            event_kinds: BTreeSet::from([FrameworkEventKind::FocusChanged]),
            predicate: WindowRulePredicate {
                panel_id: Some(panel.clone()),
                ..WindowRulePredicate::default()
            },
            actions: vec![WindowRuleAction::Focus(FocusBehavior::FocusPanel)],
        }],
    };

    let mut service = DockingService::new(document.clone(), 8)?;
    let subscription = service.subscribe_events()?;
    service.observe_host_event(
        document.revision,
        "consumer-user-focus-before-rule".into(),
        HostObservation::PanelFocused {
            panel: panel.clone(),
            floating_window: None,
        },
    )?;
    let initial = service.read_events(subscription, 16)?;
    let mut consumer = WindowRuleEventConsumer::default();
    let triggered = consumer.evaluate_batch(&rules, &initial, std::slice::from_ref(&context))?;
    if triggered.len() != 1 {
        return Err(format!(
            "expected one user-focus rule trigger, got {}",
            triggered.len()
        )
        .into());
    }
    let plan = WindowRulePlan::prepare(triggered[0].evaluation.clone(), &context)?;
    let rule_command = CommandId::from("consumer-rule-focus-editor");
    let mut committed = plan.commit(&mut service, rule_command.clone())?;
    if service.snapshot().document.revision != document.revision {
        return Err("host-only focus intent unexpectedly mutated canonical layout".into());
    }
    // A native focus notification can arrive later, so suppression follows the
    // causal command across revisions and is installed before dispatch.
    consumer.suppress_command(rule_command.clone())?;
    let mut adapter = ConsumerFocusAdapter {
        revision: document.revision,
        panel: panel.clone(),
        queued: Vec::new(),
    };
    committed.apply_host_intents(&mut adapter)?;
    if adapter.queued != [panel.clone()] {
        return Err("focus intent was not queued by the independent host adapter".into());
    }
    if committed.acknowledged_host_intents.len() != 1
        || committed.acknowledged_host_intents[0].outcome != HostIntentOutcome::Queued
    {
        return Err("queued receipt was treated as confirmed native focus".into());
    }

    // Simulate the delayed native observation using the original causal ID.
    service.observe_host_event(
        document.revision,
        rule_command,
        HostObservation::PanelFocused {
            panel: panel.clone(),
            floating_window: None,
        },
    )?;
    let echo = service.read_events(subscription, 16)?;
    if !consumer
        .evaluate_batch(&rules, &echo, std::slice::from_ref(&context))?
        .is_empty()
    {
        return Err("rule-originated native focus echo re-entered event evaluation".into());
    }

    // The same focus fact with an unrelated user command remains eligible.
    service.observe_host_event(
        document.revision,
        "consumer-user-focus-after-rule".into(),
        HostObservation::PanelFocused {
            panel: panel.clone(),
            floating_window: None,
        },
    )?;
    let user_event = service.read_events(subscription, 16)?;
    let user_triggered =
        consumer.evaluate_batch(&rules, &user_event, std::slice::from_ref(&context))?;
    if user_triggered.len() != 1 {
        return Err(format!(
            "ordinary user focus should remain eligible, got {} triggers",
            user_triggered.len()
        )
        .into());
    }
    if service.snapshot().document != *document {
        return Err("focus event evaluation mutated the canonical document".into());
    }
    Ok(())
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

fn scene_rect(x: f64, y: f64, width: f64, height: f64) -> doofusdock_presentation::SceneRect {
    doofusdock_presentation::SceneRect {
        x,
        y,
        width,
        height,
    }
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

    // Consume the public stable group-ID binding API from an independent,
    // GUI-free application. The host retains the canonical NodeId and rebuilds
    // its snapshot-local path binding whenever it projects the scene.
    let mut scene_root = SceneNode::group(scene_rect(0.0, 0.0, 60.0, 20.0));
    let mut clipped_group = SceneNode::group(scene_rect(0.0, 0.0, 30.0, 20.0));
    clipped_group.children = vec![
        SceneNode::panel("consumer-scene-first", scene_rect(0.0, 0.0, 10.0, 10.0)),
        SceneNode::panel("consumer-scene-second", scene_rect(10.0, 0.0, 10.0, 10.0)),
    ];
    let mut ungrouped =
        SceneNode::panel("consumer-scene-ungrouped", scene_rect(0.0, 0.0, 10.0, 10.0));
    ungrouped.transform = doofusdock_presentation::Affine2::translate(40.0, 0.0)?;
    scene_root.children = vec![clipped_group, ungrouped];
    let scene_frame = SceneFrame::build(&scene_root, restored.revision, 1)?;
    let clip_path = Path2D {
        commands: vec![
            PathCommand2D::MoveTo { point: [0.0, 0.0] },
            PathCommand2D::LineTo { point: [10.0, 0.0] },
            PathCommand2D::LineTo {
                point: [10.0, 10.0],
            },
            PathCommand2D::LineTo { point: [0.0, 10.0] },
            PathCommand2D::Close,
        ],
    }
    .flatten(0.25, 16)?;
    let canonical_group_id = NodeId::from("consumer-canonical-group");
    let group_identities = SceneGroupIdentityMap::from_scene_paths(
        &scene_root,
        [(canonical_group_id.clone(), vec![0])],
    )?;
    let clipped_frame = scene_frame.with_group_mesh_clip_for_identity(
        &scene_root,
        &group_identities,
        &canonical_group_id,
        clip_path,
        restored.revision,
        1,
    )?;
    assert_eq!(
        clipped_frame.hit_test(
            doofusdock_presentation::ScenePoint { x: 5.0, y: 5.0 },
            restored.revision,
            1,
        )?,
        Some(PanelId::from("consumer-scene-first"))
    );
    assert_eq!(
        clipped_frame.hit_test(
            doofusdock_presentation::ScenePoint { x: 15.0, y: 5.0 },
            restored.revision,
            1,
        )?,
        None,
        "the mask applies to all visible descendants"
    );
    assert_eq!(
        clipped_frame.hit_test(
            doofusdock_presentation::ScenePoint { x: 45.0, y: 5.0 },
            restored.revision,
            1,
        )?,
        Some(PanelId::from("consumer-scene-ungrouped")),
        "panels outside the group keep their original behavior"
    );

    // Route a shape-masked pointer through the public host-neutral API. The
    // event loop retains this router while the application widget receives
    // panel-local coordinates, including outside its visible scene bounds.
    let pointer_mask = Path2D {
        commands: vec![
            PathCommand2D::MoveTo { point: [2.0, 2.0] },
            PathCommand2D::LineTo { point: [8.0, 2.0] },
            PathCommand2D::LineTo { point: [8.0, 8.0] },
            PathCommand2D::LineTo { point: [2.0, 8.0] },
            PathCommand2D::Close,
        ],
    }
    .flatten(0.25, 16)?;
    let pointer_masks = BTreeMap::from([(PanelId::from("consumer-scene-ungrouped"), pointer_mask)]);
    let mut pointer_router = ScenePointerRouter::default();
    let pointer_press = pointer_router.route_with_paths(
        &clipped_frame,
        ScenePointerPhase::Press,
        doofusdock_presentation::ScenePoint { x: 45.0, y: 5.0 },
        restored.revision,
        1,
        &pointer_masks,
    )?;
    let pointer_hit = pointer_press
        .target
        .expect("masked scene surface accepts press");
    assert_eq!(pointer_hit.panel, PanelId::from("consumer-scene-ungrouped"));
    assert_eq!(
        pointer_hit.local_point,
        doofusdock_presentation::ScenePoint { x: 5.0, y: 5.0 }
    );
    let pointer_move = pointer_router.route_with_paths(
        &clipped_frame,
        ScenePointerPhase::Move,
        doofusdock_presentation::ScenePoint { x: 60.0, y: 20.0 },
        restored.revision,
        1,
        &pointer_masks,
    )?;
    assert!(pointer_move.captured);
    assert_eq!(
        pointer_move.target.unwrap().local_point,
        doofusdock_presentation::ScenePoint { x: 20.0, y: 20.0 }
    );
    let pointer_release = pointer_router.route_with_paths(
        &clipped_frame,
        ScenePointerPhase::Release,
        doofusdock_presentation::ScenePoint { x: 60.0, y: 20.0 },
        restored.revision,
        1,
        &pointer_masks,
    )?;
    assert!(pointer_release.captured);
    assert!(pointer_router.cancel().is_none());

    // Exercise the public rule host-adapter boundary from an independent
    // consumer. A queued receipt is not treated as confirmed native focus.
    let focus_panel = PanelId::from("editor");
    let rule_host = HostCapabilities {
        supported: BTreeSet::from(["window:focus".to_owned()]),
    };
    let rule_context = WindowRuleContext::for_panel(&restored, &focus_panel, rule_host)?;
    let focus_rules = WindowRuleSet {
        schema_version: 1,
        rules: vec![WindowRule {
            id: "consumer-focus-editor".into(),
            enabled: true,
            priority: 1,
            overrides: Vec::new(),
            event_kinds: BTreeSet::new(),
            predicate: WindowRulePredicate::default(),
            actions: vec![WindowRuleAction::Focus(FocusBehavior::FocusPanel)],
        }],
    };
    let focus_evaluation = focus_rules.evaluate(&rule_context)?;
    let focus_plan = WindowRulePlan::prepare(focus_evaluation, &rule_context)?;
    let mut rule_service = DockingService::new(restored.clone(), 4)?;
    let mut committed_focus =
        focus_plan.commit(&mut rule_service, CommandId::from("consumer-focus-rule"))?;
    let mut focus_adapter = ConsumerFocusAdapter {
        revision: restored.revision,
        panel: focus_panel.clone(),
        queued: Vec::new(),
    };
    let stale_focus = ResolvedRuleAction {
        rule_id: "consumer-focus-editor".into(),
        priority: 1,
        action: WindowRuleAction::Focus(FocusBehavior::FocusPanel),
    };
    assert!(
        focus_adapter
            .apply(&stale_focus, restored.revision.saturating_add(1))
            .is_err()
    );
    assert!(
        focus_adapter.queued.is_empty(),
        "stale focus must not queue"
    );
    committed_focus.apply_host_intents(&mut focus_adapter)?;
    assert_eq!(focus_adapter.queued, vec![focus_panel]);
    assert_eq!(
        committed_focus.acknowledged_host_intents[0].outcome,
        HostIntentOutcome::Queued
    );
    assert_eq!(restored.revision, rule_service.snapshot().document.revision);
    verify_consumer_rule_loop_suppression(&restored)?;
    verify_consumer_focus_intent_echo_suppression(&restored)?;

    let capabilities = transitions.capability_snapshot(&host);
    assert!(capabilities.iter().any(|capability| {
        capability.id == "consumer:layout-morph" && capability.kind == ExtensionKind::Transition
    }));
    let operation_catalog = builtin_operation_catalog(&host)?;
    assert_eq!(operation_catalog.len(), 38);
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
        "framework consumer ready: revision={}, panels={}, floating_windows={}, theme={}, transition_samples={}, capabilities={}, operations={}, urne_nodes={}, scene_items={}, rule_host_queue={:?}",
        restored.revision,
        restored.panels.len(),
        restored.floating_windows.len(),
        resolved_theme.theme_id,
        midpoint.panels.len(),
        capabilities.len(),
        operation_catalog.len(),
        projection.nodes.len(),
        scene.decorative.len(),
        focus_adapter.queued
    );
    Ok(())
}
