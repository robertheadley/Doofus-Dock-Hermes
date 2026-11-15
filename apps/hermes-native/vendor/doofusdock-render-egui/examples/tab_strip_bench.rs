use doofusdock_render_egui::chrome::PanelChromeStyle;
use doofusdock_render_egui::tabs::{TabStripItem, TabStripState, tab_strip};
use egui::{Align2, Color32, FontId, Rect, Sense, Vec2};
use std::time::Instant;

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
}

fn measure(mut render: impl FnMut(), iterations: usize) -> Summary {
    for _ in 0..50 {
        render();
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        render();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    samples.sort_by(f64::total_cmp);
    Summary {
        median_us: samples[samples.len() / 2],
        p95_us: samples[(samples.len() * 95) / 100],
    }
}

fn main() {
    let job_started = Instant::now();
    let setup_started = Instant::now();
    let style = PanelChromeStyle {
        titlebar: Color32::DARK_GRAY,
        title: Color32::WHITE,
        control: Color32::GRAY,
        control_hover: Color32::LIGHT_BLUE,
        control_text: Color32::WHITE,
        control_border: Color32::BLACK,
        focus: Color32::YELLOW,
        corner_radius: 3.0,
        font: FontId::proportional(14.0),
    };
    let context = egui::Context::default();
    let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(900.0, 32.0));
    eprintln!("harness_setup_ms={}", setup_started.elapsed().as_millis());
    println!(
        "scenario,tabs,reference_all_median_us,reference_all_p95_us,bounded_median_us,bounded_p95_us"
    );

    for count in [10, 100, 1000] {
        let titles: Vec<_> = (0..count)
            .map(|index| format!("Document {index}"))
            .collect();
        let items: Vec<_> = titles
            .iter()
            .map(|title| TabStripItem {
                title,
                accessible_label: title,
                icon: None,
                modified: false,
                pinned: false,
                closable: true,
                multi_selected: false,
            })
            .collect();

        let reference_context = context.clone();
        let reference_titles = &titles;
        let reference = measure(
            || {
                let mut frame = reference_context.run_ui(
                    egui::RawInput::default(),
                    &mut |ui: &mut egui::Ui| {
                        let reference_id = ui.scope_id().with(("reference-tabs", count));
                        for (index, title) in reference_titles.iter().enumerate() {
                            let tab_rect = Rect::from_min_size(
                                egui::pos2(index as f32 * 94.0, 0.0),
                                Vec2::new(90.0, 28.0),
                            );
                            let response = ui.interact(
                                tab_rect,
                                reference_id.with(index),
                                Sense::click_and_drag(),
                            );
                            ui.painter().rect_filled(
                                tab_rect,
                                3.0,
                                if response.hovered() {
                                    style.control_hover
                                } else {
                                    style.control
                                },
                            );
                            ui.painter().with_clip_rect(tab_rect.shrink(4.0)).text(
                                tab_rect.left_center(),
                                Align2::LEFT_CENTER,
                                title,
                                style.font.clone(),
                                style.title,
                            );
                        }
                    },
                );
                frame.textures_delta.clear();
            },
            500,
        );

        let bounded_context = context.clone();
        let mut state = TabStripState::default();
        let bounded = measure(
            || {
                let mut frame =
                    bounded_context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
                        let _ = tab_strip(
                            ui,
                            ui.scope_id().with(("bounded-tabs", count)),
                            rect,
                            &items,
                            0,
                            &mut state,
                            &style,
                        );
                    });
                frame.textures_delta.clear();
            },
            500,
        );
        println!(
            "egui_widget_evaluation,{count},{:.3},{:.3},{:.3},{:.3}",
            reference.median_us, reference.p95_us, bounded.median_us, bounded.p95_us
        );
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
        "metric_scope=egui in-memory widget evaluation; no tessellation, renderer, GPU, or OS event loop; event-loop lag is not applicable"
    );
}
