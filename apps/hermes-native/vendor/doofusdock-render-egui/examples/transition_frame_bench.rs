//! Headless egui snapshot-render benchmark. Measures adapter, widget layout,
//! shape generation, tessellation, and instrumented allocation volume; it does
//! not measure GPU/software surface submission, OS events, or display scanout.
use doofusdock_core::PanelId;
use doofusdock_presentation::{Box2, Snapshot, Visual};
use doofusdock_render_egui::transition::{
    render_visual_snapshot, render_visual_snapshot_with_scratch,
};
use egui::{Color32, Rect, pos2, vec2};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct CountingAllocator;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static REQUESTED_BYTES: AtomicU64 = AtomicU64::new(0);

// SAFETY: each method delegates to System with unchanged pointers and layouts.
// The relaxed counters observe requests only and do not affect allocation.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        REQUESTED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded unchanged to System.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        REQUESTED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to System.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[cfg(windows)]
#[allow(non_snake_case)]
mod process_metrics {
    use std::ffi::c_void;

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    struct MemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_nonpaged_pool_usage: usize,
        quota_nonpaged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            handle: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    #[link(name = "Psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            handle: *mut c_void,
            counters: *mut MemoryCounters,
            size: u32,
        ) -> i32;
    }

    fn ticks(time: &FileTime) -> u64 {
        (u64::from(time.high) << 32) | u64::from(time.low)
    }

    pub fn cpu_ms() -> Option<u64> {
        unsafe {
            let mut creation = FileTime { low: 0, high: 0 };
            let mut exit = FileTime { low: 0, high: 0 };
            let mut kernel = FileTime { low: 0, high: 0 };
            let mut user = FileTime { low: 0, high: 0 };
            if GetProcessTimes(
                GetCurrentProcess(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            ) == 0
            {
                None
            } else {
                Some((ticks(&kernel) + ticks(&user)) / 10_000)
            }
        }
    }

    pub fn peak_working_set_bytes() -> Option<u64> {
        unsafe {
            let mut counters = std::mem::zeroed::<MemoryCounters>();
            counters.cb = std::mem::size_of::<MemoryCounters>() as u32;
            if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) == 0 {
                None
            } else {
                Some(counters.peak_working_set_size as u64)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Summary {
    median_us: f64,
    p95_us: f64,
    p99_us: f64,
    allocations_per_frame: f64,
    requested_bytes_per_frame: f64,
    maximum_allocations_per_frame: u64,
}

fn measure(mut render: impl FnMut(), iterations: usize) -> Summary {
    for _ in 0..50 {
        render();
    }
    let mut samples = Vec::with_capacity(iterations);
    let mut allocations = Vec::with_capacity(iterations);
    let mut requested_bytes = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
        let bytes_before = REQUESTED_BYTES.load(Ordering::Relaxed);
        let started = Instant::now();
        render();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
        allocations.push(ALLOCATIONS.load(Ordering::Relaxed) - allocations_before);
        requested_bytes.push(REQUESTED_BYTES.load(Ordering::Relaxed) - bytes_before);
    }
    samples.sort_by(f64::total_cmp);
    Summary {
        median_us: samples[samples.len() / 2],
        p95_us: samples[(samples.len() * 95) / 100],
        p99_us: samples[(samples.len() * 99) / 100],
        allocations_per_frame: allocations.iter().sum::<u64>() as f64 / iterations as f64,
        requested_bytes_per_frame: requested_bytes.iter().sum::<u64>() as f64 / iterations as f64,
        maximum_allocations_per_frame: allocations.into_iter().max().unwrap_or_default(),
    }
}

fn fixture(count: usize) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let columns = 25usize;
    for index in 0..count {
        let x = (index % columns) as f64 * 76.0;
        let y = (index / columns) as f64 * 42.0;
        snapshot.panels.insert(
            PanelId::from(format!("panel-{index:04}")),
            Visual {
                bounds: Box2 {
                    x,
                    y,
                    w: 72.0,
                    h: 38.0,
                },
                clip: Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 1900.0,
                    h: 1000.0,
                },
                opacity: if index % 11 == 0 { 0.85 } else { 1.0 },
                z: index as i32,
            },
        );
    }
    snapshot
}

fn main() {
    let job_started = Instant::now();
    let context = egui::Context::default();
    let viewport = Rect::from_min_size(pos2(0.0, 0.0), vec2(1900.0, 1000.0));
    println!(
        "panels,path,median_us,p95_us,p99_us,allocations_per_frame,requested_bytes_per_frame,max_allocations_per_frame,shapes,tessellated_primitives,rendered,skipped_invisible"
    );

    for count in [10, 50, 100, 250, 500] {
        let snapshot = fixture(count);
        let mut visual_scratch = Vec::with_capacity(count);
        for buffered in [false, true] {
            let last_shapes = std::cell::Cell::new(0usize);
            let last_primitives = std::cell::Cell::new(0usize);
            let last_rendered = std::cell::Cell::new(0usize);
            let last_skipped = std::cell::Cell::new(0usize);
            let summary = measure(
                || {
                    let output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(viewport),
                            ..egui::RawInput::default()
                        },
                        &mut |ui: &mut egui::Ui| {
                            let render_content = |ui: &mut egui::Ui, id: &str, _: &Visual| {
                                let rect = ui.max_rect().shrink(2.0);
                                ui.painter().rect_filled(
                                    rect,
                                    3.0,
                                    Color32::from_rgb((id.len() * 13 % 180 + 40) as u8, 72, 116),
                                );
                                ui.painter().text(
                                    rect.left_top() + vec2(4.0, 4.0),
                                    egui::Align2::LEFT_TOP,
                                    id,
                                    egui::FontId::proportional(12.0),
                                    Color32::WHITE,
                                );
                            };
                            let report = if buffered {
                                render_visual_snapshot_with_scratch(
                                    ui,
                                    &snapshot,
                                    viewport,
                                    &mut visual_scratch,
                                    render_content,
                                )
                            } else {
                                render_visual_snapshot(ui, &snapshot, viewport, render_content)
                            };
                            last_rendered.set(report.rendered);
                            last_skipped.set(report.skipped_invisible);
                        },
                    );
                    last_shapes.set(output.shapes.len());
                    let primitives = context.tessellate(output.shapes, output.pixels_per_point);
                    last_primitives.set(primitives.len());
                },
                500,
            );
            println!(
                "{count},{},{:.3},{:.3},{:.3},{:.2},{:.0},{},{},{},{},{}",
                if buffered {
                    "reused_scratch"
                } else {
                    "allocated_each_frame"
                },
                summary.median_us,
                summary.p95_us,
                summary.p99_us,
                summary.allocations_per_frame,
                summary.requested_bytes_per_frame,
                summary.maximum_allocations_per_frame,
                last_shapes.get(),
                last_primitives.get(),
                last_rendered.get(),
                last_skipped.get()
            );
        }
    }

    eprintln!("benchmark_job_ms={}", job_started.elapsed().as_millis());
    #[cfg(windows)]
    eprintln!(
        "process_cpu_ms={};peak_working_set_bytes={}",
        process_metrics::cpu_ms().unwrap_or_default(),
        process_metrics::peak_working_set_bytes().unwrap_or_default()
    );
    #[cfg(not(windows))]
    eprintln!("process_cpu_ms=unavailable;peak_working_set_bytes=unavailable");
    eprintln!(
        "startup_ms=not_isolated;allocations=global_allocator_instrumented;idle_cpu=not_measured;event_loop_lag=not_applicable"
    );
    eprintln!(
        "metric_scope=egui snapshot adapter plus in-memory layout, shape generation, and tessellation; excludes GPU/software surface presentation, OS event loop, and display scanout; global allocator counters add atomic instrumentation overhead"
    );
}
