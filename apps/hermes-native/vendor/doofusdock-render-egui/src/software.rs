//! Native input/window loop with CPU framebuffer presentation; never creates a GPU context.
use crate::raster::Rasterizer;
use egui::{ClippedPrimitive, epaint::Primitive};
use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};
#[derive(Debug)]
enum Event {
    Repaint {
        when: Instant,
        requested_delay_us: u64,
    },
    Access(egui_winit::accesskit_winit::Event),
}
impl From<egui_winit::accesskit_winit::Event> for Event {
    fn from(e: egui_winit::accesskit_winit::Event) -> Self {
        Self::Access(e)
    }
}
struct Host<F: FnMut(&mut egui::Ui), A: Fn() -> Option<(u64, u64)>> {
    ui: F,
    title: String,
    ctx: egui::Context,
    state: Option<egui_winit::State>,
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    raster: Rasterizer,
    retained_pixels: Vec<u32>,
    previous_jobs: Option<Vec<ClippedPrimitive>>,
    previous_scale: Option<f32>,
    previous_size: Option<[usize; 2]>,
    proxy: EventLoopProxy<Event>,
    error: Option<String>,
    next: Instant,
    expected_repaint: Instant,
    frames: u64,
    close_after: Option<u64>,
    run_for: Option<Duration>,
    frame_sample_limit: usize,
    started: Instant,
    process_cpu_start_ms: Option<u64>,
    first_presented_ms: Option<u64>,
    frame_times_us: Vec<u64>,
    ui_times_us: Vec<u64>,
    tessellation_times_us: Vec<u64>,
    raster_times_us: Vec<u64>,
    present_times_us: Vec<u64>,
    repaint_lags_us: Vec<u64>,
    output_repaint_delays_us: Vec<Option<u64>>,
    callback_repaint_delays_us: Vec<u64>,
    last_surface_probe: Vec<u32>,
    allocation_snapshot: A,
    allocation_deltas: Vec<(u64, u64)>,
    working_set_samples: Vec<(u64, u64, u64)>,
}

const MIN_REPAINT_DELAY: Duration = Duration::from_millis(1);
const MAX_REPAINT_DELAY: Duration = Duration::from_secs(60);
const MAX_PARTIAL_DAMAGE_FRACTION: f64 = 0.25;

fn primitive_mesh_equal(a: &Primitive, b: &Primitive) -> bool {
    match (a, b) {
        (Primitive::Mesh(a), Primitive::Mesh(b)) => {
            a.texture_id == b.texture_id && a.indices == b.indices && a.vertices == b.vertices
        }
        _ => false,
    }
}

/// Returns `None` when a conservative full redraw is required; an empty region
/// means the retained frame is unchanged. Clip bounds include every changed old
/// and new job, so intersecting layers can be replayed in canonical order.
fn damage_region(
    previous: &[ClippedPrimitive],
    current: &[ClippedPrimitive],
    scale: f32,
    size: [usize; 2],
) -> Option<[usize; 4]> {
    if !scale.is_finite() || scale <= 0.0 || previous.len() != current.len() {
        return None;
    }
    let mut damage: Option<egui::Rect> = None;
    for (old, new) in previous.iter().zip(current) {
        if !matches!(old.primitive, Primitive::Mesh(_))
            || !matches!(new.primitive, Primitive::Mesh(_))
        {
            return None;
        }
        let unchanged =
            old.clip_rect == new.clip_rect && primitive_mesh_equal(&old.primitive, &new.primitive);
        if unchanged {
            continue;
        }
        let old_bounds = old.clip_rect;
        let new_bounds = new.clip_rect;
        damage = Some(damage.map_or(old_bounds.union(new_bounds), |bounds| {
            bounds.union(old_bounds).union(new_bounds)
        }));
    }
    let Some(damage) = damage else {
        return Some([0, 0, 0, 0]);
    };
    let region = [
        (damage.min.x * scale).floor().max(0.0).min(size[0] as f32) as usize,
        (damage.min.y * scale).floor().max(0.0).min(size[1] as f32) as usize,
        (damage.max.x * scale).ceil().max(0.0).min(size[0] as f32) as usize,
        (damage.max.y * scale).ceil().max(0.0).min(size[1] as f32) as usize,
    ];
    let area = region[2].saturating_sub(region[0]) * region[3].saturating_sub(region[1]);
    let screen_area = size[0].saturating_mul(size[1]);
    if screen_area == 0 || area as f64 > screen_area as f64 * MAX_PARTIAL_DAMAGE_FRACTION {
        None
    } else {
        Some(region)
    }
}

fn bounded_repaint_delay(delay: Duration) -> Duration {
    delay.max(MIN_REPAINT_DELAY).min(MAX_REPAINT_DELAY)
}

fn scheduled_repaint_delay(requested: Duration, bounded_frame_run: bool) -> Duration {
    if bounded_frame_run {
        MIN_REPAINT_DELAY
    } else {
        bounded_repaint_delay(requested)
    }
}

fn requested_repaint_deadline(requested_at: Instant, delay: Duration) -> Instant {
    requested_at + bounded_repaint_delay(delay)
}

fn percentile(values: &[u64], rank: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[((sorted.len() * rank).div_ceil(100)).saturating_sub(1)]
}

fn repaint_lateness_us(actual: Instant, scheduled: Instant) -> u64 {
    actual.saturating_duration_since(scheduled).as_micros() as u64
}

fn earlier_repaint_deadline(current: Instant, requested: Instant) -> Instant {
    current.min(requested)
}

fn allocation_percentile(samples: &[(u64, u64)], bytes: bool, rank: usize) -> u64 {
    let values: Vec<_> = samples
        .iter()
        .map(|sample| if bytes { sample.1 } else { sample.0 })
        .collect();
    percentile(&values, rank)
}

fn finite_repaint_delay_us(delay: Duration) -> Option<u64> {
    (delay != Duration::MAX).then(|| delay.as_micros().min(u64::MAX as u128) as u64)
}

fn optional_percentile(values: &[Option<u64>], rank: usize) -> u64 {
    percentile(&values.iter().flatten().copied().collect::<Vec<_>>(), rank)
}

fn should_forward_window_event_to_egui(event: &WindowEvent) -> bool {
    !matches!(event, WindowEvent::RedrawRequested)
}

#[cfg(windows)]
fn current_process_working_set_bytes() -> Option<(u64, u64)> {
    use std::ffi::c_void;

    #[repr(C)]
    struct ProcessMemoryCountersEx {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCountersEx,
            size: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCountersEx {
        cb: std::mem::size_of::<ProcessMemoryCountersEx>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
        private_usage: 0,
    };
    let counters_size = counters.cb;
    // SAFETY: the pseudo-handle and initialized ABI-compatible counter buffer are valid here.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters_size) };
    (ok != 0).then_some((
        counters.working_set_size as u64,
        counters.peak_working_set_size as u64,
    ))
}

#[cfg(not(windows))]
fn current_process_working_set_bytes() -> Option<(u64, u64)> {
    None
}

#[cfg(windows)]
fn current_process_cpu_ms() -> Option<u64> {
    use std::ffi::c_void;

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }

    let mut creation = FileTime { low: 0, high: 0 };
    let mut exit = FileTime { low: 0, high: 0 };
    let mut kernel = FileTime { low: 0, high: 0 };
    let mut user = FileTime { low: 0, high: 0 };
    // SAFETY: GetCurrentProcess returns a valid pseudo-handle and all FILETIME outputs are writable.
    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    (ok != 0).then_some(
        (((u64::from(kernel.high) << 32) | u64::from(kernel.low))
            + ((u64::from(user.high) << 32) | u64::from(user.low)))
            / 10_000,
    )
}

#[cfg(not(windows))]
fn current_process_cpu_ms() -> Option<u64> {
    None
}

fn sample_surface_pixels(pixels: &[u32], size: [usize; 2]) -> Vec<u32> {
    const GRID: usize = 9;
    let [width, height] = size;
    if width == 0 || height == 0 || width.checked_mul(height) != Some(pixels.len()) {
        return Vec::new();
    }
    let xs = std::array::from_fn::<_, GRID, _>(|index| (width - 1) * index / (GRID - 1));
    let ys = std::array::from_fn::<_, GRID, _>(|index| (height - 1) * index / (GRID - 1));
    ys.into_iter()
        .flat_map(|y| xs.map(move |x| pixels[y * width + x]))
        .collect()
}

pub fn run(
    title: &str,
    ui: impl FnMut(&mut egui::Ui),
    close_after: Option<u64>,
) -> Result<(), String> {
    run_internal(title, ui, close_after, None, || None)
}

/// Run the CPU/softbuffer host and sample caller-owned process allocation counters per frame.
/// The sampler is queried before UI work and after presentation; it should be allocation-free.
pub fn run_with_allocation_snapshot(
    title: &str,
    ui: impl FnMut(&mut egui::Ui),
    close_after: Option<u64>,
    allocation_snapshot: impl Fn() -> Option<(u64, u64)>,
) -> Result<(), String> {
    run_internal(title, ui, close_after, None, allocation_snapshot)
}

/// Run the native CPU/softbuffer host for a bounded idle interval.
pub fn run_for_duration(
    title: &str,
    ui: impl FnMut(&mut egui::Ui),
    duration: Duration,
    allocation_snapshot: impl Fn() -> Option<(u64, u64)>,
) -> Result<(), String> {
    run_internal(
        title,
        ui,
        None,
        Some(duration.clamp(Duration::from_secs(1), Duration::from_secs(60))),
        allocation_snapshot,
    )
}

fn run_internal(
    title: &str,
    ui: impl FnMut(&mut egui::Ui),
    close_after: Option<u64>,
    run_for: Option<Duration>,
    allocation_snapshot: impl Fn() -> Option<(u64, u64)>,
) -> Result<(), String> {
    let event_loop = EventLoop::<Event>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let proxy = event_loop.create_proxy();
    let ctx = egui::Context::default();
    let repaint = proxy.clone();
    ctx.set_request_repaint_callback(move |info| {
        let when = requested_repaint_deadline(Instant::now(), info.delay);
        let requested_delay_us = info.delay.as_micros().min(u64::MAX as u128) as u64;
        let _ = repaint.send_event(Event::Repaint {
            when,
            requested_delay_us,
        });
    });
    let initial_deadline = Instant::now();
    let sample_limit = close_after
        .map(|limit| limit.min(10_000) as usize)
        .or_else(|| run_for.map(|_| 4_096))
        .unwrap_or_default();
    let mut host = Host {
        ui,
        title: title.into(),
        ctx,
        state: None,
        window: None,
        surface: None,
        raster: Rasterizer::default(),
        retained_pixels: Vec::new(),
        previous_jobs: None,
        previous_scale: None,
        previous_size: None,
        proxy,
        error: None,
        next: initial_deadline,
        expected_repaint: initial_deadline,
        frames: 0,
        close_after,
        run_for,
        frame_sample_limit: sample_limit,
        started: Instant::now(),
        process_cpu_start_ms: current_process_cpu_ms(),
        first_presented_ms: None,
        frame_times_us: Vec::with_capacity(sample_limit),
        ui_times_us: Vec::with_capacity(sample_limit),
        tessellation_times_us: Vec::with_capacity(sample_limit),
        raster_times_us: Vec::with_capacity(sample_limit),
        present_times_us: Vec::with_capacity(sample_limit),
        repaint_lags_us: Vec::with_capacity(sample_limit),
        output_repaint_delays_us: Vec::with_capacity(sample_limit),
        callback_repaint_delays_us: Vec::with_capacity(sample_limit),
        last_surface_probe: Vec::new(),
        allocation_snapshot,
        allocation_deltas: Vec::with_capacity(sample_limit),
        working_set_samples: Vec::with_capacity((sample_limit / 10 + 2).min(1_002)),
    };
    event_loop.run_app(&mut host).map_err(|e| e.to_string())?;
    if let Some(error) = host.error {
        Err(error)
    } else {
        println!(
            "software_host: presented {} frames through CPU/softbuffer",
            host.frames
        );
        if close_after.is_some() || run_for.is_some() {
            let elapsed_ms = host.started.elapsed().as_secs_f64() * 1000.0;
            let cpu_ms = host
                .process_cpu_start_ms
                .zip(current_process_cpu_ms())
                .map(|(start, end)| end.saturating_sub(start));
            let mut receipt = serde_json::json!({
                "schema_version": 3,
                "scenario": if run_for.is_some() { "software_host_idle" } else { "software_host_bounded_frames" },
                "frames_presented": host.frames,
                "elapsed_ms": elapsed_ms,
                "process_cpu_ms": cpu_ms,
                "process_cpu_percent": cpu_ms.map(|value| value as f64 / elapsed_ms.max(1.0) * 100.0),
                "first_presented_ms": host.first_presented_ms,
                "job_wall_ms": host.started.elapsed().as_millis(),
                "frame_duration_p50_us": percentile(&host.frame_times_us, 50),
                "frame_duration_p95_us": percentile(&host.frame_times_us, 95),
                "frame_duration_p99_us": percentile(&host.frame_times_us, 99),
                "ui_p50_p95_p99_us": [
                    percentile(&host.ui_times_us, 50),
                    percentile(&host.ui_times_us, 95),
                    percentile(&host.ui_times_us, 99)
                ],
                "tessellation_p50_p95_p99_us": [
                    percentile(&host.tessellation_times_us, 50),
                    percentile(&host.tessellation_times_us, 95),
                    percentile(&host.tessellation_times_us, 99)
                ],
                "raster_p50_p95_p99_us": [
                    percentile(&host.raster_times_us, 50),
                    percentile(&host.raster_times_us, 95),
                    percentile(&host.raster_times_us, 99)
                ],
                "present_p50_p95_p99_us": [
                    percentile(&host.present_times_us, 50),
                    percentile(&host.present_times_us, 95),
                    percentile(&host.present_times_us, 99)
                ],
                "event_loop_lag_p50_us": percentile(&host.repaint_lags_us, 50),
                "event_loop_lag_p95_us": percentile(&host.repaint_lags_us, 95),
                "event_loop_lag_p99_us": percentile(&host.repaint_lags_us, 99),
                "frame_samples": host.frame_times_us.len(),
                "repaint_wake_samples": host.repaint_lags_us.len(),
                "frame_allocation_samples": host.allocation_deltas.len(),
                "allocation_requests_per_frame_p50": allocation_percentile(&host.allocation_deltas, false, 50),
                "allocation_requests_per_frame_p95": allocation_percentile(&host.allocation_deltas, false, 95),
                "allocation_requests_per_frame_p99": allocation_percentile(&host.allocation_deltas, false, 99),
                "requested_bytes_per_frame_p50": allocation_percentile(&host.allocation_deltas, true, 50),
                "requested_bytes_per_frame_p95": allocation_percentile(&host.allocation_deltas, true, 95),
                "requested_bytes_per_frame_p99": allocation_percentile(&host.allocation_deltas, true, 99),
                "working_set_sample_count": host.working_set_samples.len(),
                "working_set_first_bytes": host.working_set_samples.first().map(|sample| sample.1),
                "working_set_last_bytes": host.working_set_samples.last().map(|sample| sample.1),
                "working_set_peak_bytes": host.working_set_samples.iter().map(|sample| sample.2).max(),
                "working_set_curve_frame_bytes": host.working_set_samples,
                "last_frame_surface_probe_rgb_u32": host.last_surface_probe,
                "last_frame_surface_probe_unique_colors": host
                    .last_surface_probe
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                "measurement_scope": "egui UI, tessellation, CPU rasterization, softbuffer copy and present"
            });
            let repaint_diagnostics = serde_json::json!({
                "egui_output_repaint_delay_p50_us": optional_percentile(&host.output_repaint_delays_us, 50),
                "egui_output_repaint_delay_p95_us": optional_percentile(&host.output_repaint_delays_us, 95),
                "egui_output_repaint_delay_p99_us": optional_percentile(&host.output_repaint_delays_us, 99),
                "egui_output_no_repaint_frames": host.output_repaint_delays_us.iter().filter(|delay| delay.is_none()).count(),
                "repaint_callback_requests": host.callback_repaint_delays_us.len(),
                "repaint_callback_delay_p50_us": percentile(&host.callback_repaint_delays_us, 50),
                "repaint_callback_delay_p95_us": percentile(&host.callback_repaint_delays_us, 95),
                "repaint_callback_delay_p99_us": percentile(&host.callback_repaint_delays_us, 99)
            });
            receipt
                .as_object_mut()
                .expect("software host receipt must be an object")
                .extend(
                    repaint_diagnostics
                        .as_object()
                        .expect("repaint diagnostics must be an object")
                        .clone(),
                );
            println!("software_host_receipt={receipt}");
        }
        Ok(())
    }
}
impl<F: FnMut(&mut egui::Ui), A: Fn() -> Option<(u64, u64)>> Host<F, A> {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        eprintln!("Software renderer failed: {error}");
        self.error = Some(error);
        event_loop.exit();
    }
    fn draw(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let frame_started = Instant::now();
        if self.frames > 0
            && self.frame_sample_limit > 0
            && self.repaint_lags_us.len() < self.frame_sample_limit
        {
            self.repaint_lags_us
                .push(repaint_lateness_us(frame_started, self.expected_repaint));
        }
        let allocations_before = (self.allocation_snapshot)();
        let window = self.window.as_ref().unwrap();
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let state = self.state.as_mut().unwrap();
        egui_winit::update_viewport_info(
            state
                .egui_input_mut()
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default(),
            &self.ctx,
            window,
            self.frames == 0,
        );
        let input = state.take_egui_input(window);
        let ui_started = Instant::now();
        let mut output = self.ctx.run_ui(input, &mut self.ui);
        let ui_elapsed = ui_started.elapsed().as_micros() as u64;
        if self.frame_sample_limit > 0
            && self.output_repaint_delays_us.len() < self.frame_sample_limit
        {
            let delay = output
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map(|viewport| viewport.repaint_delay)
                .unwrap_or(Duration::from_secs(1));
            self.output_repaint_delays_us
                .push(finite_repaint_delay_us(delay));
        }
        state.handle_platform_output_with_event_loop(window, event_loop, output.platform_output);
        if let Some(viewport) = output.viewport_output.get(&egui::ViewportId::ROOT) {
            let info = state
                .egui_input_mut()
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default();
            let mut actions = Vec::new();
            egui_winit::process_viewport_commands(
                &self.ctx,
                info,
                viewport.commands.clone(),
                window,
                &mut actions,
            );
            if info.events.contains(&egui::ViewportEvent::Close) {
                event_loop.exit();
            }
        }
        let tessellation_started = Instant::now();
        let texture_invalidation =
            !output.textures_delta.set.is_empty() || !output.textures_delta.free.is_empty();
        self.raster.update(&output.textures_delta)?;
        let jobs = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let tessellation_elapsed = tessellation_started.elapsed().as_micros() as u64;
        if size.width as usize * size.height as usize > crate::raster::MAX_PIXELS {
            return Err("Framebuffer exceeds 8,294,400 pixel budget; reduce window size".into());
        }
        let surface = self.surface.as_mut().unwrap();
        surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .map_err(|e| e.to_string())?;
        let pixel_size = [size.width as usize, size.height as usize];
        if self.retained_pixels.len() != pixel_size[0] * pixel_size[1] {
            self.retained_pixels = vec![0; pixel_size[0] * pixel_size[1]];
            self.previous_jobs = None;
        }
        let full_invalidation = texture_invalidation
            || self.previous_size != Some(pixel_size)
            || self.previous_scale != Some(output.pixels_per_point);
        let damage = if full_invalidation {
            None
        } else {
            self.previous_jobs.as_deref().and_then(|previous| {
                damage_region(previous, &jobs, output.pixels_per_point, pixel_size)
            })
        };
        let raster_started = Instant::now();
        let mut buffer = surface.buffer_mut().map_err(|e| e.to_string())?;
        if let Some(region) = damage {
            if region[0] < region[2] && region[1] < region[3] {
                self.raster.paint_region(
                    &jobs,
                    output.pixels_per_point,
                    pixel_size,
                    region,
                    &mut self.retained_pixels,
                )?;
            }
        } else {
            self.raster.paint(
                &jobs,
                output.pixels_per_point,
                pixel_size,
                &mut self.retained_pixels,
            )?;
        }
        buffer.copy_from_slice(&self.retained_pixels);
        self.previous_jobs = Some(jobs);
        self.previous_scale = Some(output.pixels_per_point);
        self.previous_size = Some(pixel_size);
        window.pre_present_notify();
        let raster_elapsed = raster_started.elapsed().as_micros() as u64;
        if self
            .close_after
            .is_some_and(|limit| self.frames.saturating_add(1) == limit)
        {
            self.last_surface_probe =
                sample_surface_pixels(&buffer, [size.width as usize, size.height as usize]);
        }
        let present_started = Instant::now();
        buffer.present().map_err(|e| e.to_string())?;
        let present_elapsed = present_started.elapsed().as_micros() as u64;
        self.raster.free(&output.textures_delta);
        output.textures_delta.clear();
        let allocation_delta = allocations_before.zip((self.allocation_snapshot)()).map(
            |((allocations_before, bytes_before), (allocations_after, bytes_after))| {
                (
                    allocations_after.saturating_sub(allocations_before),
                    bytes_after.saturating_sub(bytes_before),
                )
            },
        );
        if self.frames == 0 {
            self.first_presented_ms = Some(self.started.elapsed().as_millis() as u64);
        }
        self.frames += 1;
        if self.frame_sample_limit > 0 && self.frame_times_us.len() < self.frame_sample_limit {
            self.frame_times_us
                .push(frame_started.elapsed().as_micros() as u64);
            self.ui_times_us.push(ui_elapsed);
            self.tessellation_times_us.push(tessellation_elapsed);
            self.raster_times_us.push(raster_elapsed);
            self.present_times_us.push(present_elapsed);
            if let Some(sample) = allocation_delta {
                self.allocation_deltas.push(sample);
            }
            if (self.frames.is_multiple_of(10)
                || self.close_after == Some(self.frames)
                || self
                    .run_for
                    .is_some_and(|duration| self.started.elapsed() >= duration))
                && let Some((working, peak)) = current_process_working_set_bytes()
            {
                self.working_set_samples.push((self.frames, working, peak));
            }
        }
        if self.close_after.is_some_and(|limit| self.frames >= limit) {
            event_loop.exit();
        }
        let requested_delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.repaint_delay)
            .unwrap_or(Duration::from_secs(1));
        let delay = scheduled_repaint_delay(requested_delay, self.close_after.is_some());
        self.expected_repaint = Instant::now() + delay;
        self.next = self.expected_repaint;
        Ok(())
    }
}
impl<F: FnMut(&mut egui::Ui), A: Fn() -> Option<(u64, u64)>> ApplicationHandler<Event>
    for Host<F, A>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title(&self.title)
                            .with_visible(false)
                            .with_inner_size(winit::dpi::LogicalSize::new(1200., 800.)),
                    )
                    .map_err(|e| e.to_string())?,
            );
            let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
            let surface =
                softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
            let mut state = egui_winit::State::new(
                self.ctx.clone(),
                egui::ViewportId::ROOT,
                event_loop,
                Some(window.scale_factor() as f32),
                window.theme(),
                Some(4096),
            );
            state.init_accesskit(event_loop, &window, self.proxy.clone());
            window.set_visible(true);
            self.surface = Some(surface);
            self.state = Some(state);
            if self.frame_sample_limit > 0
                && let Some((working, peak)) = current_process_working_set_bytes()
            {
                self.working_set_samples.push((0, working, peak));
            }
            window.request_redraw();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(e) = result {
            self.fail(event_loop, e);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
            return;
        }
        // RedrawRequested is the host's render notification, not fresh input for egui.
        // Forwarding it through egui-winit can return `repaint = true`, which queues
        // another redraw and keeps an otherwise idle software window at full CPU.
        let is_redraw = !should_forward_window_event_to_egui(&event);
        if !is_redraw && let (Some(window), Some(state)) = (&self.window, &mut self.state) {
            let response = state.on_window_event(window, &event);
            if response.repaint {
                window.request_redraw();
            }
        }
        if is_redraw && let Err(e) = self.draw(event_loop) {
            self.fail(event_loop, e);
        }
    }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::Repaint {
                when,
                requested_delay_us,
            } => {
                self.next = earlier_repaint_deadline(self.next, when);
                if self.frame_sample_limit > 0
                    && self.callback_repaint_delays_us.len() < self.frame_sample_limit
                {
                    self.callback_repaint_delays_us.push(requested_delay_us);
                }
            }
            Event::Access(event) => {
                use egui_winit::accesskit_winit::WindowEvent;
                match event.window_event {
                    WindowEvent::InitialTreeRequested => self.ctx.enable_accesskit(),
                    WindowEvent::ActionRequested(request) => {
                        if let Some(state) = &mut self.state {
                            state.on_accesskit_action_request(request);
                        }
                    }
                    WindowEvent::AccessibilityDeactivated => self.ctx.disable_accesskit(),
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if self
            .run_for
            .is_some_and(|duration| now.duration_since(self.started) >= duration)
        {
            if self.frame_sample_limit > 0
                && let Some((working, peak)) = current_process_working_set_bytes()
            {
                self.working_set_samples.push((self.frames, working, peak));
            }
            event_loop.exit();
            return;
        }
        if now >= self.next {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            self.next = now + MIN_REPAINT_DELAY;
        }
        let next = self
            .run_for
            .map(|duration| self.next.min(self.started + duration))
            .unwrap_or(self.next);
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }
}

#[cfg(test)]
mod repaint_tests {
    use super::{
        MAX_REPAINT_DELAY, MIN_REPAINT_DELAY, allocation_percentile, bounded_repaint_delay,
        damage_region, earlier_repaint_deadline, finite_repaint_delay_us, optional_percentile,
        repaint_lateness_us, requested_repaint_deadline, sample_surface_pixels,
        scheduled_repaint_delay, should_forward_window_event_to_egui,
    };
    use std::time::Duration;
    use winit::event::WindowEvent;

    fn damage_job(rect: egui::Rect, color: egui::Color32) -> egui::ClippedPrimitive {
        let mut mesh = egui::Mesh::default();
        mesh.add_colored_rect(rect, color);
        egui::ClippedPrimitive {
            clip_rect: rect,
            primitive: egui::epaint::Primitive::Mesh(mesh),
        }
    }

    #[test]
    fn software_damage_bounds_are_stable_conservative_and_size_limited() {
        let old = vec![damage_job(
            egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(80.0, 60.0)),
            egui::Color32::WHITE,
        )];
        assert_eq!(
            damage_region(&old, &old, 1.0, [800, 600]),
            Some([0, 0, 0, 0])
        );

        let new = vec![damage_job(
            egui::Rect::from_min_max(egui::pos2(20.0, 30.0), egui::pos2(100.0, 90.0)),
            egui::Color32::LIGHT_BLUE,
        )];
        assert_eq!(
            damage_region(&old, &new, 1.0, [800, 600]),
            Some([10, 20, 100, 90])
        );

        let broad = vec![damage_job(
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(500.0, 400.0)),
            egui::Color32::WHITE,
        )];
        assert_eq!(damage_region(&old, &broad, 1.0, [800, 600]), None);
        assert_eq!(
            damage_region(&old, &new, 1.5, [800, 600]),
            Some([15, 30, 150, 135])
        );
    }

    #[test]
    fn software_host_preserves_egui_idle_delay_instead_of_forcing_60hz() {
        assert_eq!(
            bounded_repaint_delay(Duration::from_secs(1)),
            Duration::from_secs(1)
        );
        assert_eq!(
            bounded_repaint_delay(Duration::from_secs(10)),
            Duration::from_secs(10)
        );
        assert_eq!(bounded_repaint_delay(Duration::ZERO), MIN_REPAINT_DELAY);
        assert_eq!(
            bounded_repaint_delay(Duration::from_secs(120)),
            MAX_REPAINT_DELAY
        );
        assert_eq!(
            scheduled_repaint_delay(Duration::from_secs(1), false),
            Duration::from_secs(1)
        );
        assert_eq!(
            scheduled_repaint_delay(Duration::from_secs(1), true),
            MIN_REPAINT_DELAY
        );
    }

    #[test]
    fn surface_readback_samples_a_spread_across_the_frame() {
        assert_eq!(
            sample_surface_pixels(&(1..=81).collect::<Vec<u32>>(), [9, 9]),
            (1..=81).collect::<Vec<u32>>()
        );
        assert!(sample_surface_pixels(&[1, 2, 3], [2, 2]).is_empty());
    }

    #[test]
    fn allocation_receipt_percentiles_report_requests_and_bytes_per_frame() {
        let samples = [(1, 10), (4, 40), (2, 20), (3, 30)];
        assert_eq!(allocation_percentile(&samples, false, 50), 2);
        assert_eq!(allocation_percentile(&samples, false, 99), 4);
        assert_eq!(allocation_percentile(&samples, true, 50), 20);
        assert_eq!(allocation_percentile(&samples, true, 99), 40);
        assert_eq!(allocation_percentile(&[], false, 99), 0);
    }

    #[test]
    fn repaint_lateness_is_sampled_against_the_requested_deadline() {
        let scheduled = std::time::Instant::now();
        assert_eq!(repaint_lateness_us(scheduled, scheduled), 0);
        assert_eq!(
            repaint_lateness_us(scheduled, scheduled + Duration::from_millis(1)),
            0
        );
        assert!(repaint_lateness_us(scheduled + Duration::from_millis(4), scheduled) >= 4_000);
    }

    #[test]
    fn delayed_repaint_requests_do_not_preempt_the_current_deadline() {
        let now = std::time::Instant::now();
        let current = now + Duration::from_secs(1);
        let earlier_request = now + Duration::from_millis(250);
        let later_request = now + Duration::from_secs(2);
        assert_eq!(
            earlier_repaint_deadline(current, earlier_request),
            earlier_request
        );
        assert_eq!(earlier_repaint_deadline(current, later_request), current);
    }

    #[test]
    fn repaint_callback_deadline_preserves_requested_delay_with_bounds() {
        let now = std::time::Instant::now();
        assert_eq!(
            requested_repaint_deadline(now, Duration::from_secs(1)),
            now + Duration::from_secs(1)
        );
        assert_eq!(
            requested_repaint_deadline(now, Duration::ZERO),
            now + MIN_REPAINT_DELAY
        );
    }

    #[test]
    fn redraw_notifications_are_not_requeued_as_egui_input() {
        assert!(!should_forward_window_event_to_egui(
            &WindowEvent::RedrawRequested
        ));
        assert!(should_forward_window_event_to_egui(&WindowEvent::Focused(
            true
        )));
    }

    #[test]
    fn never_repaint_is_reported_separately_from_finite_delays() {
        assert_eq!(finite_repaint_delay_us(Duration::MAX), None);
        assert_eq!(
            finite_repaint_delay_us(Duration::from_millis(250)),
            Some(250_000)
        );
        assert_eq!(optional_percentile(&[None, Some(10), Some(20)], 50), 10);
        assert_eq!(optional_percentile(&[None, None], 95), 0);
    }
}
