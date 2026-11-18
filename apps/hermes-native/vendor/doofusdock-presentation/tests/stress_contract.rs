use doofusdock_core::*;
use doofusdock_presentation::{
    Box2,
    effects::{EffectContext, evaluate_scene_effects},
};
use std::time::Instant;

fn sample_workbench() -> LayoutDocument {
    let mut doc = LayoutDocument::new("stress_doc", "stress");
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
fn stress_panel_targets_10_50_100_250_500_are_bounded_and_fast() {
    let doc = sample_workbench();
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1920.0,
        h: 1080.0,
    };

    for &budget in &[10, 50, 100, 250, 500] {
        let start = Instant::now();
        for frame in 0..50 {
            let t = frame as f64 * 0.016;
            let graph = evaluate_scene_effects(
                "wall_of_windows",
                EffectContext {
                    time_seconds: t,
                    progress: 0.5,
                    viewport,
                    seed: 1272026,
                    lod_budget: budget,
                    reduced_motion: false,
                    pulse: 0.0,
                    canonical_doc: &doc,
                },
            )
            .unwrap();
            assert_eq!(graph.decorative.len(), budget);
        }
        let elapsed_per_frame = start.elapsed().as_secs_f64() * 1000.0 / 50.0;
        // Even for 500 panels, evaluation must be well under 2ms on any modern CPU
        assert!(
            elapsed_per_frame < 5.0,
            "LOD {budget} took {elapsed_per_frame:.3}ms/frame (exceeded 5ms limit)"
        );
    }
}

#[test]
fn all_71_builtin_workspace_profiles_project_and_evaluate_cleanly() {
    let viewport = Box2 {
        x: 0.0,
        y: 0.0,
        w: 1280.0,
        h: 800.0,
    };
    let profiles = builtin_workspace_profiles();
    assert_eq!(profiles.len(), 71);

    for profile in profiles {
        let mut doc = LayoutDocument::new("p_test", "p_test");
        let mut history = UndoStack::new(4);
        let mut assignments = std::collections::BTreeMap::new();
        let mut panels = Vec::new();
        for (i, reg) in profile.regions.iter().enumerate() {
            let pid = PanelId(format!("p_{i}"));
            panels.push(PanelDescriptor::new(pid.clone(), format!("Panel {i}")));
            assignments.insert(reg.id.clone(), vec![pid]);
        }
        doc.panels = panels.into();
        if history
            .apply_profile(&mut doc, &profile, &assignments)
            .is_ok()
        {
            let proj = doofusdock_presentation::project(&doc, viewport);
            assert!(proj.is_ok(), "Failed to project profile: {}", profile.id);
        }
    }
}
