mod app;
mod layout;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hermes · DoofusDock")
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([900.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hermes · DoofusDock",
        options,
        Box::new(|cc| Ok(Box::new(app::HermesApp::new(cc)))),
    )
}
