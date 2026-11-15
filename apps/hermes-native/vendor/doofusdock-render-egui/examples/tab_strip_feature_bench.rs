use doofusdock_render_egui::{chrome::PanelChromeStyle, tabs::*};
use egui::{Color32, FontId};
fn main() {
    let context = egui::Context::default();
    let mut items: Vec<_> = (0..10_000)
        .map(|_| TabStripItem {
            title: "Document",
            accessible_label: "Document",
            icon: None,
            modified: false,
            pinned: false,
            closable: true,
            multi_selected: false,
        })
        .collect();
    let style = PanelChromeStyle {
        titlebar: Color32::BLACK,
        title: Color32::WHITE,
        control: Color32::DARK_GRAY,
        control_hover: Color32::GRAY,
        control_text: Color32::WHITE,
        control_border: Color32::GRAY,
        focus: Color32::YELLOW,
        corner_radius: 2.0,
        font: FontId::proportional(14.0),
    };
    let mut state = TabStripState::default();
    if std::env::args().any(|arg| arg == "--pinned") {
        items[9999].pinned = true;
    }
    if std::env::args().any(|arg| arg == "--search") {
        state.query = "Document".into();
    }
    let mut samples = Vec::new();
    let mut shapes = 0;
    for frame in 0..220 {
        let start = std::time::Instant::now();
        let mut out = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            tab_strip(
                ui,
                ui.scope_id().with("bench"),
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 32.0)),
                &items,
                0,
                &mut state,
                &style,
            );
        });
        if frame >= 20 {
            samples.push(start.elapsed().as_micros());
            shapes = out.shapes.len();
        }
        out.textures_delta.clear();
    }
    samples.sort();
    println!(
        "tabs=10000 samples=200 width=800 p50_us={} p95_us={} p99_us={} shapes={} scope=egui_cpu_no_tessellation_or_OS",
        samples[100], samples[190], samples[198], shapes
    );
}
