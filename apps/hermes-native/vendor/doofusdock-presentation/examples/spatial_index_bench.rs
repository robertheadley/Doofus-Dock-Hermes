use doofusdock_core::PanelId;
use doofusdock_presentation::{Box2, SnapSettings, Snapshot, SpatialIndex, Visual};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn scene(count: usize) -> Snapshot {
    let mut snapshot = Snapshot::default();
    for i in 0..count {
        let column = i % 40;
        let row = i / 40;
        let bounds = Box2 {
            x: (column * 240) as f64,
            y: (row * 160) as f64,
            w: 220.0,
            h: 140.0,
        };
        snapshot.panels.insert(
            PanelId::from(format!("panel-{i:04}")),
            Visual {
                bounds,
                clip: bounds,
                opacity: 1.0,
                z: (i % 7) as i32,
            },
        );
    }
    snapshot
}

fn percentile_us(samples: &mut [Duration]) -> f64 {
    samples.sort_unstable();
    samples[samples.len() / 2].as_secs_f64() * 1e6
}

fn percentile_duration(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn main() {
    const QUERIES: usize = 5_000;
    const REPETITIONS: usize = 15;
    const DRAG_FRAMES: usize = 60;
    println!(
        "objects,build_p50_us,point_query_p50_ns,point_linear_p50_ns,rect_query_p50_ns,rect_linear_p50_ns,rebuild_p50_us,drag_rebuild_frame_p50_us,drag_cached_frame_p50_us"
    );
    for count in [10, 50, 100, 250, 500, 1_000] {
        let snapshot = scene(count);
        let mut build_samples = Vec::with_capacity(REPETITIONS);
        let mut rebuild_samples = Vec::with_capacity(REPETITIONS);
        let index = SpatialIndex::build(&snapshot, 1, 1, 256.0).unwrap();
        for generation in 0..REPETITIONS {
            let start = Instant::now();
            black_box(SpatialIndex::build(&snapshot, 1, generation as u64, 256.0).unwrap());
            build_samples.push(start.elapsed());
            let start = Instant::now();
            black_box(SpatialIndex::build(&snapshot, 1, generation as u64 + 100, 256.0).unwrap());
            rebuild_samples.push(start.elapsed());
        }

        let points: Vec<_> = (0..QUERIES)
            .map(|query| {
                (
                    ((query * 37) % (40 * 240)) as f64 + 55.0,
                    ((query * 53) % ((count.div_ceil(40)) * 160).max(1)) as f64 + 35.0,
                )
            })
            .collect();
        let rects: Vec<_> = points
            .iter()
            .map(|(x, y)| Box2 {
                x: *x,
                y: *y,
                w: 180.0,
                h: 100.0,
            })
            .collect();
        for ((x, y), rect) in points.iter().zip(&rects) {
            let expected_point = snapshot
                .panels
                .iter()
                .filter(|(_, v)| {
                    v.opacity >= 0.5 && v.bounds.contains(*x, *y) && v.clip.contains(*x, *y)
                })
                .max_by(|(ida, a), (idb, b)| a.z.cmp(&b.z).then_with(|| idb.cmp(ida)))
                .map(|(id, _)| id.clone());
            assert_eq!(index.point_containing(*x, *y), expected_point);
            let expected_rect: Vec<_> = snapshot
                .panels
                .iter()
                .filter(|(_, v)| {
                    v.opacity >= 0.5
                        && v.bounds.x < rect.x + rect.w
                        && rect.x < v.bounds.x + v.bounds.w
                        && v.bounds.y < rect.y + rect.h
                        && rect.y < v.bounds.y + v.bounds.h
                })
                .map(|(id, _)| id.clone())
                .collect();
            assert_eq!(index.intersecting(*rect).unwrap(), expected_rect);
        }

        let mut indexed_point = Vec::with_capacity(REPETITIONS);
        let mut linear_point = Vec::with_capacity(REPETITIONS);
        let mut indexed_rect = Vec::with_capacity(REPETITIONS);
        let mut linear_rect = Vec::with_capacity(REPETITIONS);
        for _ in 0..REPETITIONS {
            let start = Instant::now();
            for (x, y) in &points {
                black_box(index.point_containing(*x, *y));
            }
            indexed_point.push(start.elapsed());
            let start = Instant::now();
            for (x, y) in &points {
                black_box(
                    snapshot
                        .panels
                        .iter()
                        .filter(|(_, v)| {
                            v.opacity >= 0.5 && v.bounds.contains(*x, *y) && v.clip.contains(*x, *y)
                        })
                        .max_by(|(ida, a), (idb, b)| a.z.cmp(&b.z).then_with(|| idb.cmp(ida))),
                );
            }
            linear_point.push(start.elapsed());
            let start = Instant::now();
            for rect in &rects {
                black_box(index.intersecting(*rect).unwrap());
            }
            indexed_rect.push(start.elapsed());
            let start = Instant::now();
            for rect in &rects {
                black_box(
                    snapshot
                        .panels
                        .iter()
                        .filter(|(_, v)| {
                            v.opacity >= 0.5
                                && v.bounds.x < rect.x + rect.w
                                && rect.x < v.bounds.x + v.bounds.w
                                && v.bounds.y < rect.y + rect.h
                                && rect.y < v.bounds.y + v.bounds.h
                        })
                        .count(),
                );
            }
            linear_rect.push(start.elapsed());
        }
        let drag_rects: Vec<_> = (0..DRAG_FRAMES)
            .map(|frame| Box2 {
                x: 218.0 + (frame % 30) as f64,
                y: 20.0 + (frame % 5) as f64,
                w: 180.0,
                h: 40.0,
            })
            .collect();
        let drag_points: Vec<_> = (0..DRAG_FRAMES)
            .map(|frame| (55.0 + (frame % 20) as f64, 35.0))
            .collect();
        let mut rebuild_drag_samples = Vec::with_capacity(REPETITIONS);
        let mut cached_drag_samples = Vec::with_capacity(REPETITIONS);
        for repetition in 0..REPETITIONS {
            let start = Instant::now();
            for (frame, (rect, (x, y))) in drag_rects.iter().zip(&drag_points).enumerate() {
                let frame_index = SpatialIndex::build(
                    &snapshot,
                    1,
                    (repetition * DRAG_FRAMES + frame + 2) as u64,
                    128.0,
                )
                .unwrap();
                let snapped = frame_index
                    .snap_bounds_with_spacing(*rect, None, SnapSettings::default(), Some(6))
                    .unwrap();
                black_box((snapped, frame_index.point_containing(*x, *y)));
            }
            rebuild_drag_samples.push(start.elapsed());

            let start = Instant::now();
            for (rect, (x, y)) in drag_rects.iter().zip(&drag_points) {
                let snapped = index
                    .snap_bounds_with_spacing(*rect, None, SnapSettings::default(), Some(6))
                    .unwrap();
                black_box((snapped, index.point_containing(*x, *y)));
            }
            cached_drag_samples.push(start.elapsed());
        }
        let first_rect = drag_rects[0];
        let first_point = drag_points[0];
        let cached_snap = index
            .snap_bounds_with_spacing(first_rect, None, SnapSettings::default(), Some(6))
            .unwrap();
        let rebuilt_index = SpatialIndex::build(&snapshot, 1, 999, 128.0).unwrap();
        let rebuilt_snap = rebuilt_index
            .snap_bounds_with_spacing(first_rect, None, SnapSettings::default(), Some(6))
            .unwrap();
        assert_eq!(cached_snap.bounds, rebuilt_snap.bounds);
        assert_eq!(cached_snap.guides, rebuilt_snap.guides);
        assert_eq!(
            index.point_containing(first_point.0, first_point.1),
            rebuilt_index.point_containing(first_point.0, first_point.1)
        );
        let per_drag_frame_us =
            |duration: Duration| duration.as_secs_f64() * 1e6 / DRAG_FRAMES as f64;
        let per_query_ns = |duration: Duration| duration.as_secs_f64() * 1e9 / QUERIES as f64;
        println!(
            "{count},{:.2},{:.1},{:.1},{:.1},{:.1},{:.2},{:.2},{:.2}",
            percentile_us(&mut build_samples),
            per_query_ns(percentile_duration(&mut indexed_point)),
            per_query_ns(percentile_duration(&mut linear_point)),
            per_query_ns(percentile_duration(&mut indexed_rect)),
            per_query_ns(percentile_duration(&mut linear_rect)),
            percentile_us(&mut rebuild_samples),
            per_drag_frame_us(percentile_duration(&mut rebuild_drag_samples)),
            per_drag_frame_us(percentile_duration(&mut cached_drag_samples)),
        );
    }
}
