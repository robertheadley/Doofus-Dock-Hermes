mod app;
mod layout;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hermes")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([420.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Hermes",
        options,
        Box::new(|cc| Ok(Box::new(app::HermesNativeApp::new(cc)))),
    )
}
