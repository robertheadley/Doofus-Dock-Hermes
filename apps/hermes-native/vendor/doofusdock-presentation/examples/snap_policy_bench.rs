use doofusdock_core::PanelId;
use doofusdock_presentation::{Box2, SnapKind, SnapSettings, Snapshot, SpatialIndex, Visual};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn scene(count: usize) -> Snapshot {
    let mut snapshot = Snapshot::default();
    for index in 0..count {
        let bounds = Box2 {
            x: (index * 500) as f64,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        snapshot.panels.insert(
            PanelId::from(format!("panel-{index:04}")),
            Visual {
                bounds,
                clip: bounds,
                opacity: 1.0,
                z: 0,
            },
        );
    }
    snapshot
}

fn p50(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn main() {
    const QUERIES: usize = 1_000;
    const REPETITIONS: usize = 15;
    println!("objects,snap_default_p50_ns,snap_equal_spacing_p50_ns");
    for count in [10, 50, 100, 250, 500, 1_000] {
        let snapshot = scene(count);
        let index = SpatialIndex::build(&snapshot, 1, 1, 128.0).unwrap();
        let moving = Box2 {
            x: 218.0,
            y: 20.0,
            w: 180.0,
            h: 40.0,
        };
        if count >= 2 {
            let equal = index
                .snap_bounds_with_spacing(
                    moving,
                    None,
                    SnapSettings {
                        threshold_points: 12.0,
                        ..SnapSettings::default()
                    },
                    Some(20),
                )
                .unwrap();
            assert_eq!(equal.bounds.x, 210.0);
            assert!(
                equal
                    .guides
                    .iter()
                    .any(|guide| guide.kind == SnapKind::EqualSpacing)
            );
        }
        let mut baseline = Vec::with_capacity(REPETITIONS);
        let mut equal_spacing = Vec::with_capacity(REPETITIONS);
        for _ in 0..REPETITIONS {
            let start = Instant::now();
            for _ in 0..QUERIES {
                black_box(
                    index
                        .snap_bounds(moving, None, SnapSettings::default())
                        .unwrap(),
                );
            }
            baseline.push(start.elapsed());

            let start = Instant::now();
            for _ in 0..QUERIES {
                black_box(
                    index
                        .snap_bounds_with_spacing(moving, None, SnapSettings::default(), Some(20))
                        .unwrap(),
                );
            }
            equal_spacing.push(start.elapsed());
        }
        let ns_per_query = |elapsed: Duration| elapsed.as_secs_f64() * 1e9 / QUERIES as f64;
        println!(
            "{count},{:.1},{:.1}",
            ns_per_query(p50(&mut baseline)),
            ns_per_query(p50(&mut equal_spacing)),
        );
    }
}
