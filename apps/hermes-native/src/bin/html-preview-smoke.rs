#![cfg(windows)]

use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use eframe::egui;
use hermes_native::app::HermesNativeApp;

const EXPECTED_TITLE: &str = "asset-loaded:16";

#[derive(Clone, Copy, Debug)]
enum SmokePhase {
    WaitingForVisible,
    WaitingForHidden,
    WaitingForReopen,
}

struct SmokeApp {
    hermes: HermesNativeApp,
    preview_path: PathBuf,
    output_path: PathBuf,
    phase: SmokePhase,
    deadline: Instant,
    result: Option<Result<(), String>>,
    initial_state: Option<serde_json::Value>,
    hidden_state: Option<serde_json::Value>,
    reopened_state: Option<serde_json::Value>,
}

impl eframe::App for SmokeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.hermes.draw(ui);
        let ctx = ui.ctx();
        if self.result.is_none() {
            if let Some(state) = self.hermes.html_preview_state_for_automation() {
                match self.phase {
                    SmokePhase::WaitingForVisible if is_healthy_preview(&state) => {
                        self.initial_state = Some(state);
                        self.hermes
                            .set_html_preview_mode_for_automation(&self.preview_path, false);
                        self.phase = SmokePhase::WaitingForHidden;
                        self.deadline = Instant::now() + Duration::from_secs(3);
                    }
                    SmokePhase::WaitingForHidden if state["visible"] == false => {
                        self.hidden_state = Some(state);
                        self.hermes
                            .set_html_preview_mode_for_automation(&self.preview_path, true);
                        self.phase = SmokePhase::WaitingForReopen;
                        self.deadline = Instant::now() + Duration::from_secs(3);
                    }
                    SmokePhase::WaitingForReopen if is_healthy_preview(&state) => {
                        self.reopened_state = Some(state);
                        self.result = Some(Ok(()));
                    }
                    _ => {}
                }
            }
            if self.result.is_none() && Instant::now() >= self.deadline {
                self.result = Some(Err(format!(
                    "Timed out during {:?}; visible state {:?}",
                    self.phase,
                    self.hermes.html_preview_state_for_automation(),
                )));
            }
        }
        if let Some(result) = &self.result {
            let record = match result {
                Ok(()) => serde_json::json!({
                    "ok": true,
                    "document_title": EXPECTED_TITLE,
                    "preview_states": {
                        "initial": self.initial_state,
                        "hidden": self.hidden_state,
                        "reopened": self.reopened_state,
                    },
                    "checks": [
                        "HermesNativeApp Files fixture opened the HTML preview pane",
                        "FilePreviewStore created its child WebView2 inside the active Dock preview",
                        "WebView2 is visible within non-zero Dock pane bounds",
                        "local HTML document loaded",
                        "document JavaScript executed",
                        "relative 16x16 SVG asset loaded",
                        "WebView2 hid when rendered preview mode was left",
                        "WebView2 re-opened when rendered preview mode was restored",
                    ]
                }),
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            };
            let _ = atomic_write(&self.output_path, record.to_string().as_bytes());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ctx.request_repaint_after(Duration::from_millis(25));
        }
    }
}

fn is_healthy_preview(state: &serde_json::Value) -> bool {
    state["document_title"].as_str() == Some(EXPECTED_TITLE)
        && state["visible"] == true
        && state["error"].is_null()
        && state["bounds"].as_array().is_some_and(|values| {
            values.len() == 4
                && values[2].as_f64().is_some_and(|width| width > 100.0)
                && values[3].as_f64().is_some_and(|height| height > 100.0)
        })
}

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let output_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("Usage: html-preview-smoke RESULT_JSON_PATH")?;
    if args.next().is_some() {
        return Err("Usage: html-preview-smoke RESULT_JSON_PATH".into());
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create result directory: {error}"))?;
    }
    let _ = fs::remove_file(&output_path);

    let root = unique_temp_dir("hermes-html-preview-app-smoke")?;
    fs::create_dir(&root).map_err(|error| format!("create isolated smoke root: {error}"))?;
    let fixture_root = root.join("project");
    let profile_dir = root.join("profile");
    fs::create_dir(&profile_dir).map_err(|error| format!("create isolated profile: {error}"))?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hermes HTML preview smoke")
            .with_inner_size([1440.0, 900.0])
            .with_taskbar(false)
            .with_visible(true),
        ..Default::default()
    };
    let result_path = output_path.clone();
    let profile_for_app = profile_dir.clone();
    let fixture_for_app = fixture_root.clone();
    let preview_path = fixture_root.join("preview.html");
    let run_result = eframe::run_native(
        "Hermes HTML preview smoke",
        options,
        Box::new(move |cc| {
            let mut hermes = HermesNativeApp::new_for_native_automation(cc, profile_for_app);
            hermes.load_html_preview_fixture_for_native_smoke(&fixture_for_app)?;
            Ok(Box::new(SmokeApp {
                hermes,
                preview_path,
                output_path: result_path,
                deadline: Instant::now() + Duration::from_secs(20),
                phase: SmokePhase::WaitingForVisible,
                result: None,
                initial_state: None,
                hidden_state: None,
                reopened_state: None,
            }))
        }),
    );
    let _ = fs::remove_dir_all(&root);
    run_result.map_err(|error| format!("run Hermes HTML preview smoke: {error}"))?;
    let result: serde_json::Value = serde_json::from_slice(
        &fs::read(&output_path).map_err(|error| format!("read smoke result: {error}"))?,
    )
    .map_err(|error| format!("parse smoke result: {error}"))?;
    if result["ok"] != true {
        return Err(result["error"]
            .as_str()
            .unwrap_or("Hermes HTML preview smoke failed")
            .to_owned());
    }
    println!("{}", result);
    Ok(())
}

fn unique_temp_dir(prefix: &str) -> Result<PathBuf, String> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("read system clock: {error}"))?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id())))
}

fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes).map_err(|error| format!("write smoke result: {error}"))?;
    serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|error| format!("validate smoke result: {error}"))?;
    fs::rename(&temp_path, path).map_err(|error| format!("publish smoke result: {error}"))
}
