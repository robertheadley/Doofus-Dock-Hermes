use doofusdock_render_egui::raster::Rasterizer;
use egui::{
    ClippedPrimitive, Color32, Context, Mesh, RawInput, Rect, TextureId, epaint::Primitive, pos2,
    vec2,
};
use std::time::Instant;

const SIZE: [usize; 2] = [1200, 800];
const FRAMES_PER_RUN: usize = 40;

fn popup(index: usize) -> Rect {
    let x = 100.0 + (index % 20) as f32 * 40.0;
    let y = 100.0 + (index % 7) as f32 * 24.0;
    Rect::from_min_size(pos2(x, y), vec2(160.0, 100.0))
}

fn rect_job(rect: Rect, clip_rect: Rect, color: Color32) -> ClippedPrimitive {
    let mut mesh = Mesh {
        texture_id: TextureId::Managed(0),
        ..Default::default()
    };
    mesh.add_colored_rect(rect, color);
    ClippedPrimitive {
        clip_rect,
        primitive: Primitive::Mesh(mesh),
    }
}

fn jobs(index: usize) -> Vec<ClippedPrimitive> {
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(SIZE[0] as f32, SIZE[1] as f32));
    vec![
        rect_job(screen, screen, Color32::from_rgb(24, 32, 44)),
        rect_job(
            Rect::from_min_size(pos2(20.0, 20.0), vec2(1160.0, 760.0)),
            screen,
            Color32::from_rgba_premultiplied(12, 18, 28, 90),
        ),
        rect_job(
            Rect::from_min_size(pos2(60.0, 50.0), vec2(1080.0, 700.0)),
            screen,
            Color32::from_rgba_premultiplied(15, 24, 38, 80),
        ),
        rect_job(
            Rect::from_min_size(pos2(80.0, 80.0), vec2(1040.0, 640.0)),
            Rect::from_min_max(pos2(80.0, 80.0), pos2(1120.0, 720.0)),
            Color32::from_rgba_premultiplied(8, 14, 24, 110),
        ),
        rect_job(
            popup(index),
            screen,
            if index.is_multiple_of(2) {
                Color32::from_rgb(30, 140, 220)
            } else {
                Color32::from_rgb(40, 110, 210)
            },
        ),
    ]
}

fn percentile(samples: &[u64], percentile: usize) -> u64 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[((sorted.len() - 1) * percentile).div_ceil(100)]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::default();
    let mut rasterizer = Rasterizer::default();
    let texture_output = context.run_ui(RawInput::default(), |_| {});
    rasterizer.update(&texture_output.textures_delta)?;
    let scale = 1.0;
    let initial_jobs = jobs(0);
    let mut retained = vec![0; SIZE[0] * SIZE[1]];
    rasterizer.paint(&initial_jobs, scale, SIZE, &mut retained)?;
    let mut baseline_output = vec![0; retained.len()];
    let mut candidate_output = vec![0; retained.len()];
    let mut baseline_us = Vec::with_capacity(FRAMES_PER_RUN * 3);
    let mut candidate_us = Vec::with_capacity(FRAMES_PER_RUN * 3);
    let job_started = Instant::now();

    for repetition in 0..3 {
        let mut previous = 0;
        rasterizer.paint(&initial_jobs, scale, SIZE, &mut retained)?;
        for step in 1..=FRAMES_PER_RUN {
            let index = repetition * FRAMES_PER_RUN + step;
            let jobs = jobs(index);

            let started = Instant::now();
            rasterizer.paint(&jobs, scale, SIZE, &mut baseline_output)?;
            baseline_us.push(started.elapsed().as_micros() as u64);

            let old = popup(previous);
            let new = popup(index);
            let damage = old.union(new).expand(1.0);
            let region = [
                (damage.min.x * scale).floor().max(0.0) as usize,
                (damage.min.y * scale).floor().max(0.0) as usize,
                (damage.max.x * scale).ceil().min(SIZE[0] as f32) as usize,
                (damage.max.y * scale).ceil().min(SIZE[1] as f32) as usize,
            ];
            let started = Instant::now();
            rasterizer.paint_region(&jobs, scale, SIZE, region, &mut retained)?;
            candidate_output.copy_from_slice(&retained);
            candidate_us.push(started.elapsed().as_micros() as u64);

            if candidate_output != baseline_output {
                let first = candidate_output
                    .iter()
                    .zip(&baseline_output)
                    .position(|(a, b)| a != b)
                    .unwrap_or_default();
                return Err(format!(
                    "partial output diverged at frame {index}, first pixel ({}, {}): {:06x} != {:06x}, region={region:?}",
                    first % SIZE[0], first / SIZE[0], candidate_output[first], baseline_output[first]
                ).into());
            }
            previous = index;
        }
    }
    let wall = job_started.elapsed();
    let changed_pixels = ((SIZE[0] * SIZE[1]) as f64 * 0.035).round() as usize;
    println!(
        "{{\"schema_version\":1,\"scenario\":\"1200x800 four-layer clipped translucent UI with moving 160x100 opaque popup\",\"repetitions\":3,\"frames_per_repetition\":{FRAMES_PER_RUN},\"pixel_equivalence\":true,\"changed_region_estimate_pixels\":{changed_pixels},\"full_redraw_p50_p95_p99_us\":[{},{},{}],\"damage_plus_full_copy_p50_p95_p99_us\":[{},{},{}],\"job_wall_ms\":{},\"retained_and_present_buffers_bytes\":{},\"event_loop_lag\":\"not applicable; in-process renderer microbenchmark\",\"startup\":\"not isolated; excluded from per-frame samples\",\"cpu_and_working_set\":\"not instrumented in this example\",\"surface_present\":\"not measured; damage path includes full framebuffer copy required before software surface presentation\"}}",
        percentile(&baseline_us, 50),
        percentile(&baseline_us, 95),
        percentile(&baseline_us, 99),
        percentile(&candidate_us, 50),
        percentile(&candidate_us, 95),
        percentile(&candidate_us, 99),
        wall.as_millis(),
        (retained.len() * 2 * std::mem::size_of::<u32>())
    );
    Ok(())
}
