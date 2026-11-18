use doofusdock_core::*;
use doofusdock_presentation::*;
use std::collections::BTreeMap;
fn source() -> LayoutDocument {
    let mut doc = LayoutDocument::new("show", "show");
    let panels: Vec<_> = (0..12)
        .map(|i| PanelDescriptor::new(format!("p{i}"), format!("Panel {i}")))
        .collect();
    doc.panels = panels.into();
    doc.root = Some(Node::tabs(
        "tabs",
        (0..12)
            .map(|i| Node::panel(format!("n{i}"), format!("p{i}")))
            .collect(),
        0,
    ));
    doc
}
fn viewport() -> Box2 {
    Box2 {
        x: 0.,
        y: 0.,
        w: 1200.,
        h: 800.,
    }
}

fn transition_registry() -> ExtensionRegistry {
    use std::sync::Arc;
    let version = ApiVersion { major: 1, minor: 0 };
    let mut registry = ExtensionRegistry::default();
    registry
        .register_transition(TransitionExtension {
            descriptor: ExtensionDescriptor {
                namespaced_id: "test:duration".into(),
                version,
                api: CompatibilityRange {
                    minimum: version,
                    maximum: version,
                },
                ..ExtensionDescriptor::default()
            },
            duration: Arc::new(|request| request.duration_ms / 2),
            required_capabilities: vec![],
            incompatible_capabilities: vec![],
        })
        .unwrap();
    registry.freeze();
    registry.activate("test:duration").unwrap();
    registry
}

fn panicking_transition_registry() -> ExtensionRegistry {
    use std::sync::Arc;
    let version = ApiVersion { major: 1, minor: 0 };
    let mut registry = ExtensionRegistry::default();
    registry
        .register_transition(TransitionExtension {
            descriptor: ExtensionDescriptor {
                namespaced_id: "test:panic".into(),
                version,
                api: CompatibilityRange {
                    minimum: version,
                    maximum: version,
                },
                ..ExtensionDescriptor::default()
            },
            duration: Arc::new(|_| panic!("injected transition provider panic")),
            required_capabilities: vec![],
            incompatible_capabilities: vec![],
        })
        .unwrap();
    registry.freeze();
    registry.activate("test:panic").unwrap();
    registry
}

fn timing_policy(requested_duration_ms: u32, maximum_duration_ms: u32) -> MorphTimingPolicy {
    MorphTimingPolicy {
        transition: "test:duration".into(),
        requested_duration_ms,
        maximum_duration_ms,
        reduced_motion: false,
        host_capabilities: vec![],
        easing: Easing::Linear,
    }
}
fn target(doc: &LayoutDocument, id: &str) -> LayoutDocument {
    let profile = builtin_workspace_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap();
    let assignments: BTreeMap<_, _> = profile
        .regions
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), vec![doc.panels[i].id.clone()]))
        .collect();
    apply_workspace_profile(doc, &profile, &assignments)
        .unwrap()
        .document
}
#[test]
fn every_profile_projects_and_morphs_without_mutating_canonical() {
    let doc = source();
    let bytes = serialize_layout(&doc).unwrap();
    for profile in builtin_workspace_profiles() {
        let target = target(&doc, &profile.id);
        let morph = Morph::new(&doc, &target, viewport(), 0., 1., false).unwrap();
        for t in [0., 0.1, 0.5, 0.9, 1.] {
            let frame = morph.sample(t).unwrap();
            assert_eq!(frame.panels.len(), 12);
            assert!(
                frame.panels.values().all(|v| v.bounds.valid()
                    && v.clip.valid()
                    && (0.0..=1.0).contains(&v.opacity))
            );
        }
        assert_eq!(
            morph.sample(1.).unwrap(),
            project(&target, viewport()).unwrap()
        );
        assert_eq!(serialize_layout(&doc).unwrap(), bytes);
    }
}
#[test]
fn structural_morph_samples_compose_into_revision_bound_scene_frames() {
    let doc = source();
    let target = target(&doc, "workbench");
    let morph = Morph::new(&doc, &target, viewport(), 0.0, 1.0, false).unwrap();
    let panel = PanelId::from("p0");
    let decorative = BTreeMap::from([(panel.clone(), Affine2::translate(75.0, 20.0).unwrap())]);
    let structural = morph.sample_scene(0.5, 12, &BTreeMap::new()).unwrap();
    let frame = morph.sample_scene(0.5, 12, &decorative).unwrap();
    let base_element = structural
        .elements()
        .iter()
        .find(|element| element.panel == panel)
        .unwrap();
    let composed_element = frame
        .elements()
        .iter()
        .find(|element| element.panel == panel)
        .unwrap();
    assert_eq!(frame.revision(), doc.revision);
    assert_eq!(frame.generation(), 12);
    assert_eq!(
        composed_element.world_transform,
        decorative[&panel]
            .compose(base_element.world_transform)
            .unwrap(),
    );
    assert_eq!(
        frame
            .elements()
            .iter()
            .map(|element| &element.panel)
            .collect::<Vec<_>>(),
        structural
            .elements()
            .iter()
            .map(|element| &element.panel)
            .collect::<Vec<_>>(),
    );
    assert!(
        frame
            .hit_test(ScenePoint::default(), doc.revision + 1, 12)
            .is_err()
    );
    assert_eq!(
        doc,
        source(),
        "sampling leaves the canonical source untouched"
    );
}

#[test]
fn retarget_reverse_seek_cancel_and_single_commit() {
    let mut doc = source();
    let a = target(&doc, "workbench");
    let b = target(&doc, "chat");
    let mut morph = Morph::new(&doc, &a, viewport(), 10., 2., false).unwrap();
    let midway = morph.sample(11.).unwrap();
    morph.retarget(&b, 11., 2.).unwrap();
    assert_eq!(midway, morph.sample(11.).unwrap());
    let sought = morph.sample(12.).unwrap();
    assert_eq!(sought, morph.sample(12.).unwrap());
    let mut history = UndoStack::new(8);
    assert!(!morph.finish(12., &mut doc, &mut history).unwrap());
    assert!(morph.finish(13., &mut doc, &mut history).unwrap());
    assert!(!morph.finish(14., &mut doc, &mut history).unwrap());
    assert_eq!(history.undo_size(), 1);
    validate_layout(&doc).unwrap();
    history.undo(&mut doc).unwrap();
    let mut reverse = Morph::new(&doc, &b, viewport(), 0., 2., false).unwrap();
    let before = reverse.sample(1.).unwrap();
    reverse.reverse(1., 1.).unwrap();
    assert_eq!(before, reverse.sample(1.).unwrap());
    assert_eq!(
        reverse.sample(2.).unwrap().panels,
        project(&doc, viewport()).unwrap().panels
    );
    reverse.cancel();
    assert!(!reverse.finish(3., &mut doc, &mut history).unwrap());
}
#[test]
fn stale_targets_and_invalid_timing_are_rejected_atomically() {
    let mut doc = source();
    let b = target(&doc, "dashboard");
    let mut morph = Morph::new(&doc, &b, viewport(), 0., 1., false).unwrap();
    let mut history = UndoStack::new(8);
    history
        .execute(
            &mut doc,
            Operation::SelectTab {
                tabs: "tabs".into(),
                index: 1,
            },
        )
        .unwrap();
    let before = doc.clone();
    assert!(morph.finish(2., &mut doc, &mut history).is_err());
    assert_eq!(doc, before);
    assert!(morph.sample(f64::NAN).is_err());
    assert!(morph.retarget(&b, 2., -1.).is_err());
    assert!(Morph::new(&doc, &b, viewport(), 0., f64::INFINITY, false).is_err());
}
#[test]
fn policy_forbidden_profiles_and_replacements_do_not_mutate() {
    let mut doc = source();
    let b = target(&doc, "workbench");
    std::sync::Arc::make_mut(&mut doc.panels)[0]
        .docking
        .allow_split = false;
    let mut b = b;
    b.panels = doc.panels.clone();
    let before = doc.clone();
    assert!(Morph::new(&doc, &b, viewport(), 0., 1., false).is_err());
    assert!(UndoStack::new(8).apply_layout(&mut doc, &b).is_err());
    assert_eq!(doc, before);
    let mut doc = source();
    std::sync::Arc::make_mut(&mut doc.panels)[11].close_policy = ClosePolicy::Forbidden;
    let p = builtin_workspace_profiles()
        .into_iter()
        .find(|p| p.id == "workbench")
        .unwrap();
    let a = p
        .regions
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), vec![doc.panels[i].id.clone()]))
        .collect();
    assert!(apply_workspace_profile(&doc, &p, &a).is_err());
}
#[test]
fn reduced_motion_is_immediate_and_roundtrip_is_stable() {
    let mut doc = source();
    let b = target(&doc, "chat");
    let mut m = Morph::new(&doc, &b, viewport(), 0., 2., true).unwrap();
    assert_eq!(m.sample(0.).unwrap(), project(&b, viewport()).unwrap());
    m.finish(0., &mut doc, &mut UndoStack::new(1)).unwrap();
    let encoded = serialize_layout(&doc).unwrap();
    assert_eq!(
        serialize_layout(&deserialize_layout(&encoded, PersistenceLimits::default()).unwrap())
            .unwrap(),
        encoded
    );
}

#[test]
fn provider_timing_is_capped_reduced_and_used_for_retarget_without_jumps() {
    let doc = source();
    let workbench = target(&doc, "workbench");
    let chat = target(&doc, "chat");
    let registry = transition_registry();
    let policy = timing_policy(2_000, 600);
    let mut morph =
        Morph::new_with_provider(&doc, &workbench, viewport(), 0.0, &policy, &registry).unwrap();

    // Provider returns half of 2 seconds, then the host maximum caps it at 600 ms.
    assert_eq!(
        morph.sample(0.6).unwrap(),
        project(&workbench, viewport()).unwrap()
    );
    assert!(!morph.is_settled());

    let before_retarget = morph.sample(0.3).unwrap();
    let mut retarget_policy = timing_policy(1_000, 1_000);
    retarget_policy.easing = Easing::EaseIn;
    morph
        .retarget_with_provider(&chat, 0.3, &retarget_policy, &registry)
        .unwrap();
    assert_eq!(before_retarget, morph.sample(0.3).unwrap());
    assert_eq!(morph.easing(), Easing::EaseIn);
    assert_eq!(
        morph.sample(0.8).unwrap(),
        project(&chat, viewport()).unwrap()
    );

    let mut reduced = timing_policy(2_000, 60_000);
    reduced.reduced_motion = true;
    morph
        .retarget_with_provider(&workbench, 0.8, &reduced, &registry)
        .unwrap();
    assert_eq!(
        morph.sample(0.8).unwrap(),
        project(&workbench, viewport()).unwrap()
    );
    let before_motion_restored = morph.sample(0.8).unwrap();
    morph
        .retarget_with_provider(&chat, 0.8, &timing_policy(1_000, 1_000), &registry)
        .unwrap();
    assert_eq!(before_motion_restored, morph.sample(0.8).unwrap());
    assert_eq!(
        morph.sample(1.3).unwrap(),
        project(&chat, viewport()).unwrap()
    );
    let immediate =
        Morph::new_with_provider(&doc, &workbench, viewport(), 5.0, &reduced, &registry).unwrap();
    assert_eq!(
        immediate.sample(5.0).unwrap(),
        project(&workbench, viewport()).unwrap()
    );
}

#[test]
fn missing_provider_retarget_leaves_the_current_morph_unchanged() {
    let doc = source();
    let workbench = target(&doc, "workbench");
    let chat = target(&doc, "chat");
    let registry = transition_registry();
    let mut morph = Morph::new_with_provider(
        &doc,
        &workbench,
        viewport(),
        0.0,
        &timing_policy(1_000, 1_000),
        &registry,
    )
    .unwrap();
    let before = morph.sample(0.4).unwrap();
    let mut missing = timing_policy(1_000, 1_000);
    missing.transition = "missing:provider".into();
    assert!(
        morph
            .retarget_with_provider(&chat, 0.4, &missing, &registry)
            .is_err()
    );
    assert_eq!(before, morph.sample(0.4).unwrap());
}

#[test]
fn panicking_provider_retarget_is_contained_and_leaves_morph_unchanged() {
    let doc = source();
    let workbench = target(&doc, "workbench");
    let chat = target(&doc, "chat");
    let registry = transition_registry();
    let mut morph = Morph::new_with_provider(
        &doc,
        &workbench,
        viewport(),
        0.0,
        &timing_policy(1_000, 1_000),
        &registry,
    )
    .unwrap();
    let before = morph.sample(0.4).unwrap();
    let mut panic_policy = timing_policy(1_000, 1_000);
    panic_policy.transition = "test:panic".into();
    let error = morph
        .retarget_with_provider(&chat, 0.4, &panic_policy, &panicking_transition_registry())
        .unwrap_err();

    assert!(error.contains("Transition duration provider panicked"));
    assert_eq!(before, morph.sample(0.4).unwrap());
}
