use doofusdock_core::*;
use doofusdock_presentation::{
    Box2,
    effects::{
        DecorativeKind, DeterministicRng, EffectContext, built_in_effect_capability_report,
        evaluate_scene_effects,
    },
};

fn sample_workbench() -> LayoutDocument {
    let mut doc = LayoutDocument::new("show", "show");
    let p1 = PanelDescriptor::new("editor", "Editor");
    let p2 = PanelDescriptor::new("console", "Console");
    doc.panels = vec![p1, p2].into();
    doc.root = Some(Node::split(
        "root-split",
        Orientation::Horizontal,
        vec![
            Node::panel("n1", PanelId("editor".into())),
            Node::panel("n2", PanelId("console".into())),
        ],
        vec![0.6, 0.4],
    ));
    doc
}

#[test]
fn built_in_effect_capabilities_project_as_bounded_schema_relationships() {
    let document = sample_workbench();
    let report = built_in_effect_capability_report();
    report.validate().unwrap();
    let first = UrneProjection::from_layout(&document)
        .unwrap()
        .with_effect_capability_report(&report)
        .unwrap();
    let second = UrneProjection::from_layout(&document)
        .unwrap()
        .with_effect_capability_report(&report)
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.version, URNE_VERSION);
    let effects = first
        .nodes
        .iter()
        .filter(|node| node.node_type == "effect_capability")
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 8);
    let awakening = effects
        .iter()
        .find(|node| node.metadata["effect_id"] == "awakening")
        .unwrap();
    assert_eq!(awakening.metadata["availability"], "available");
    assert_eq!(awakening.metadata["callback_invoked"], false);
    assert_eq!(awakening.metadata["request_schema"]["type"], "object");
    assert!(
        awakening.metadata["request_schema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "canonical_doc")
    );
    assert!(
        first
            .edges
            .iter()
            .any(|edge| { edge.edge_type == "provides_effect" && edge.target == awakening.id })
    );
    assert!(
        first
            .edges
            .iter()
            .any(|edge| { edge.edge_type == "requires" && edge.source == awakening.id })
    );

    let mut malformed = report;
    malformed.observations[1].effect_id = malformed.observations[0].effect_id.clone();
    assert!(malformed.validate().is_err());
    let mut oversized = built_in_effect_capability_report();
    oversized.observations[0].schema = serde_json::json!({"payload": "x".repeat(17 * 1024)});
    assert!(oversized.validate().is_err());
    let mut nested_schema = serde_json::Value::Null;
    for _ in 0..18 {
        nested_schema = serde_json::json!({"nested": nested_schema});
    }
    let mut too_deep = built_in_effect_capability_report();
    too_deep.observations[0].schema = nested_schema;
    assert!(too_deep.validate().is_err());
}

#[test]
fn deterministic_rng_is_reproducible_and_well_distributed() {
    let mut rng1 = DeterministicRng::new(12345);
    let mut rng2 = DeterministicRng::new(12345);
    for _ in 0..100 {
        assert_eq!(rng1.next_u32(), rng2.next_u32());
        assert_eq!(rng1.next_f64(), rng2.next_f64());
    }

    let mut rng = DeterministicRng::new(9999);
    for _ in 0..100 {
        let val = rng.range_f64(10.0, 50.0);
        assert!((10.0..=50.0).contains(&val));
    }
}

#[test]
fn awakening_materialize_scales_and_fades_in() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };

    let start_graph = evaluate_scene_effects(
        "awakening",
        EffectContext {
            time_seconds: 0.0,
            progress: 0.0,
            viewport,
            seed: 42,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.0,
            canonical_doc: &doc,
        },
    )
    .unwrap();

    let end_graph = evaluate_scene_effects(
        "awakening",
        EffectContext {
            time_seconds: 3.0,
            progress: 1.0,
            viewport,
            seed: 42,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.0,
            canonical_doc: &doc,
        },
    )
    .unwrap();

    let p_start = start_graph.panels.get(&PanelId("editor".into())).unwrap();
    let p_end = end_graph.panels.get(&PanelId("editor".into())).unwrap();

    assert_eq!(p_start.opacity, 0.0);
    assert_eq!(p_end.opacity, 1.0);
    assert!(p_start.bounds.w < p_end.bounds.w);
}

#[test]
fn awakening_window_geometry_ignores_audio_pulses_and_grows_monotonically() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };
    let mut previous_width = 0.0;

    for progress in [0.0, 0.05, 0.1, 0.2, 0.4, 0.7, 1.0] {
        let frame = |pulse| {
            evaluate_scene_effects(
                "awakening",
                EffectContext {
                    time_seconds: progress * 4.0,
                    progress,
                    viewport,
                    seed: 42,
                    lod_budget: 50,
                    reduced_motion: false,
                    pulse,
                    canonical_doc: &doc,
                },
            )
            .unwrap()
        };
        let quiet = frame(0.0);
        let transient = frame(1.0);
        assert_eq!(quiet.panels, transient.panels, "progress={progress}");

        let width = quiet
            .panels
            .get(&PanelId("editor".into()))
            .unwrap()
            .bounds
            .w;
        assert!(width >= previous_width, "progress={progress}");
        previous_width = width;
    }
}

#[test]
fn wall_of_windows_respects_lod_budgets_and_promotes_extracted() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1200.0,
        h: 800.0,
    };

    for &budget in &[10, 50, 100, 250, 500] {
        let graph = evaluate_scene_effects(
            "wall_of_windows",
            EffectContext {
                time_seconds: 10.0,
                progress: 0.8,
                viewport,
                seed: 1272026,
                lod_budget: budget,
                reduced_motion: false,
                pulse: 0.5,
                canonical_doc: &doc,
            },
        )
        .unwrap();

        assert_eq!(graph.decorative.len(), budget);
        let extracted_count = graph.decorative.iter().filter(|d| d.is_extracted).count();
        assert_eq!(extracted_count, 2);
        let promoted = graph.decorative.iter().find(|d| d.is_extracted).unwrap();
        assert_eq!(promoted.kind, DecorativeKind::WindowCard);
        assert_eq!(
            promoted.z,
            graph.decorative.iter().map(|item| item.z).max().unwrap()
        );
        assert!(
            graph
                .decorative
                .iter()
                .filter(|item| item.is_extracted)
                .any(|item| item.bounds.w > 100.0)
        );
    }
}

#[test]
fn solitaire_victory_creates_ballistic_bounces_and_bounded_ghost_trails() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };

    let graph = evaluate_scene_effects(
        "solitaire_victory",
        EffectContext {
            time_seconds: 50.0,
            progress: 0.5,
            viewport,
            seed: 777,
            lod_budget: 100,
            reduced_motion: false,
            pulse: 0.0,
            canonical_doc: &doc,
        },
    )
    .unwrap();

    // The full deck has 52 distinct faces. The remaining budget supplies trails.
    let cards: Vec<_> = graph
        .decorative
        .iter()
        .filter(|d| d.kind == DecorativeKind::WindowCard)
        .collect();
    assert_eq!(cards.len(), 52);
    assert_eq!(
        cards
            .iter()
            .map(|c| &c.title)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        52
    );
    for c in cards {
        assert!(c.bounds.w < viewport.w * 0.09 && c.bounds.y + c.bounds.h <= viewport.h + 1e-8);
    }
    assert!(graph.decorative.len() <= 100);
    let trails: Vec<_> = graph
        .decorative
        .iter()
        .filter(|d| d.kind == DecorativeKind::GhostTrail)
        .collect();
    assert!(!trails.is_empty());
    assert!(trails.len() <= 128); // Explicit bound per R023
}

#[test]
fn magnetic_assembly_converges_to_canonical_layout() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };

    let _start_graph = evaluate_scene_effects(
        "magnetic_assembly",
        EffectContext {
            time_seconds: 23.0,
            progress: 0.0,
            viewport,
            seed: 101,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.0,
            canonical_doc: &doc,
        },
    )
    .unwrap();

    let end_graph = evaluate_scene_effects(
        "magnetic_assembly",
        EffectContext {
            time_seconds: 32.0,
            progress: 1.0,
            viewport,
            seed: 101,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.0,
            canonical_doc: &doc,
        },
    )
    .unwrap();

    let base_proj = doofusdock_presentation::project(&doc, viewport).unwrap();
    let end_editor = end_graph.panels.get(&PanelId("editor".into())).unwrap();
    let base_editor = base_proj.panels.get(&PanelId("editor".into())).unwrap();

    // At progress 1.0, magnetic assembly perfectly aligns with canonical layout
    assert!((end_editor.bounds.x - base_editor.bounds.x).abs() < 1e-6);
    assert!((end_editor.bounds.y - base_editor.bounds.y).abs() < 1e-6);
    assert!((end_editor.bounds.w - base_editor.bounds.w).abs() < 1e-6);
    assert!((end_editor.bounds.h - base_editor.bounds.h).abs() < 1e-6);
}

#[test]
fn kinetic_storm_fissions_and_fuses_fragments() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };

    let graph = evaluate_scene_effects(
        "kinetic_storm",
        EffectContext {
            time_seconds: 65.0,
            progress: 0.5,
            viewport,
            seed: 404,
            lod_budget: 50,
            reduced_motion: false,
            pulse: 0.2,
            canonical_doc: &doc,
        },
    )
    .unwrap();

    let fragments: Vec<_> = graph
        .decorative
        .iter()
        .filter(|d| d.kind == DecorativeKind::PanelFragment)
        .collect();
    assert_eq!(fragments.len(), 8); // 2 panels * 4 fragments
}

#[test]
fn reduced_motion_and_error_handling() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };

    // Reduced motion immediately sets full opacity and skips trails
    let graph = evaluate_scene_effects(
        "awakening",
        EffectContext {
            time_seconds: 0.0,
            progress: 0.0,
            viewport,
            seed: 1,
            lod_budget: 50,
            reduced_motion: true,
            pulse: 0.0,
            canonical_doc: &doc,
        },
    )
    .unwrap();
    assert_eq!(
        graph.panels.get(&PanelId("editor".into())).unwrap().opacity,
        1.0
    );

    // Invalid parameters fail gracefully
    assert!(
        evaluate_scene_effects(
            "awakening",
            EffectContext {
                time_seconds: 0.0,
                progress: 1.5,
                viewport,
                seed: 1,
                lod_budget: 50,
                reduced_motion: false,
                pulse: 0.0,
                canonical_doc: &doc,
            },
        )
        .is_err()
    );

    assert!(
        evaluate_scene_effects(
            "awakening",
            EffectContext {
                time_seconds: 0.0,
                progress: f64::NAN,
                viewport,
                seed: 1,
                lod_budget: 50,
                reduced_motion: false,
                pulse: 0.0,
                canonical_doc: &doc,
            },
        )
        .is_err()
    );
}

#[test]
fn effect_context_rejects_non_finite_time_and_unbounded_pulse() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 600.0,
    };
    let evaluate = |time_seconds, pulse| {
        evaluate_scene_effects(
            "wall_of_windows",
            EffectContext {
                time_seconds,
                progress: 0.5,
                viewport,
                seed: 1,
                lod_budget: 50,
                reduced_motion: false,
                pulse,
                canonical_doc: &doc,
            },
        )
    };

    assert!(evaluate(f64::NAN, 0.5).is_err());
    assert!(evaluate(f64::INFINITY, 0.5).is_err());
    assert!(evaluate(0.0, f64::NAN).is_err());
    assert!(evaluate(0.0, -0.01).is_err());
    assert!(evaluate(0.0, 1.01).is_err());
    assert!(evaluate(0.0, 0.0).is_ok());
    assert!(evaluate(0.0, 1.0).is_ok());
}

#[test]
fn morph_focus_regeneration_and_recovery_have_distinct_visible_states() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 91.0,
        y: 77.0,
        w: 935.0,
        h: 541.0,
    };
    let scene = |id, progress| {
        evaluate_scene_effects(
            id,
            EffectContext {
                time_seconds: 42.0,
                progress,
                viewport,
                seed: 1,
                lod_budget: 100,
                reduced_motion: false,
                pulse: 0.3,
                canonical_doc: &doc,
            },
        )
        .unwrap()
    };
    let pose = |beat: f64| scene("great_morph", (beat + 0.001) / 8.0);
    let editor = PanelId::from("editor");
    let poses: Vec<_> = (0..4)
        .map(|i| pose(i as f64).panels[&editor].bounds)
        .collect();
    for (i, b) in poses.iter().enumerate() {
        assert!(!poses[..i].contains(b));
    }
    assert!(poses[2].w > poses[1].w);
    assert!(pose(2.001).panels[&editor].bounds.w > viewport.w * 0.70);
    for (beat, count) in [(0.8, 2), (1.8, 4), (2.8, 8), (3.8, 16)] {
        assert_eq!(
            scene("infinite_regeneration", beat / 32.0).decorative.len(),
            count
        );
    }
    assert!(scene("infinite_regeneration", 7.99 / 32.0).decorative.len() < 16);
    let broken = scene("impossible_recovery", 0.05);
    assert_eq!(broken.decorative.len(), 8);
    assert!(broken.panels.values().all(|p| p.opacity == 0.0));
    assert_eq!(
        scene("impossible_recovery", 1.0).panels,
        doofusdock_presentation::project(&doc, viewport)
            .unwrap()
            .panels
    );
    for progress in [0.0, 0.3, 0.75, 0.99] {
        let storm = scene("kinetic_storm", progress);
        let mut center = (0.0, 0.0);
        for item in &storm.decorative {
            let b = item.bounds;
            assert!((b.w / b.h - 16.0 / 9.0).abs() < 1e-9);
            center.0 += b.x + b.w * 0.5;
            center.1 += b.y + b.h * 0.5;
        }
        let n = storm.decorative.len() as f64;
        assert!((center.0 / n - viewport.x - viewport.w * 0.5).abs() < 1e-6);
        assert!((center.1 / n - viewport.y - viewport.h * 0.5).abs() < 1e-6);
    }
}

#[test]
fn wall_spotlight_handoff_keeps_center_and_ratio_without_geometry_pop() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 13.0,
        y: 27.0,
        w: 1280.0,
        h: 720.0,
    };
    let graph = |p| {
        evaluate_scene_effects(
            "wall_of_windows",
            EffectContext {
                time_seconds: 0.0,
                progress: p,
                viewport,
                seed: 11,
                lod_budget: 50,
                reduced_motion: false,
                pulse: 0.0,
                canonical_doc: &doc,
            },
        )
        .unwrap()
    };
    let at = |items: &[doofusdock_presentation::effects::DecorativeItem], index| {
        items
            .iter()
            .find(|item| item.id == format!("wall_item_{index}"))
            .unwrap()
            .clone()
    };
    let before = graph(0.5 - 1e-7);
    let after = graph(0.5 + 1e-7);
    assert_eq!(
        before
            .decorative
            .iter()
            .filter(|item| item.is_extracted)
            .count(),
        2
    );
    assert_eq!(
        after
            .decorative
            .iter()
            .filter(|item| item.is_extracted)
            .count(),
        2
    );
    let old = before
        .decorative
        .iter()
        .find(|item| item.is_extracted && item.opacity > 0.5)
        .unwrap();
    let entering = after
        .decorative
        .iter()
        .find(|item| item.is_extracted && item.opacity > 0.5)
        .unwrap();
    let center = Box2 {
        x: viewport.x + viewport.w * 0.18,
        y: viewport.y + viewport.h * 0.18,
        w: viewport.w * 0.64,
        h: viewport.h * 0.64,
    };
    assert!((old.bounds.x - center.x).abs() < 0.01);
    assert!((entering.bounds.x - center.x).abs() < 0.01);
    assert!((entering.bounds.w / entering.bounds.h - 16.0 / 9.0).abs() < 0.01);
    let outgoing_index = old
        .id
        .strip_prefix("wall_item_")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let outgoing = at(&before.decorative, outgoing_index);
    let resumed = at(&after.decorative, outgoing_index);
    assert!(outgoing.bounds.x.is_finite() && resumed.bounds.x.is_finite());
    assert!(outgoing.opacity > 0.99);
    assert!(resumed.opacity > 0.99);
    assert!((resumed.bounds.x - center.x).abs() < 0.01);
}
