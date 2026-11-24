#![cfg(windows)]

use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

use eframe::egui;
use hermes_native::app::HermesNativeApp;
use sha2::{Digest, Sha256};

const EXPECTED_TITLE: &str = "asset-loaded:16";

#[link(name = "user32")]
unsafe extern "system" {
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
    fn GetWindowThreadProcessId(window: *mut std::ffi::c_void, process_id: *mut u32) -> u32;
}

fn foreground_process_id() -> Option<u32> {
    // SAFETY: These User32 calls only query the current foreground window and its owner.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() {
            return None;
        }
        let mut process_id = 0;
        (GetWindowThreadProcessId(window, &mut process_id) != 0).then_some(process_id)
    }
}

#[derive(Clone, Copy, Debug)]
enum SmokePhase {
    WaitingForVisible,
    WaitingForHidden,
    WaitingForReopen,
}

struct SmokeApp {
    hermes: HermesNativeApp,
    window_mode: serde_json::Value,
    preview_path: PathBuf,
    output_path: PathBuf,
    screenshot_path: PathBuf,
    screenshot_tx: Sender<Result<(u32, u32), String>>,
    screenshot_rx: Receiver<Result<(u32, u32), String>>,
    screenshot_requested: bool,
    screenshot_result: Option<Result<(u32, u32), String>>,
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
        if self.result.as_ref().is_some_and(Result::is_ok) {
            if !self.screenshot_requested {
                self.screenshot_requested = true;
                let sender = self.screenshot_tx.clone();
                let path = self.screenshot_path.clone();
                ctx.request_screenshot(move |image| {
                    let _ = sender.send(save_egui_screenshot(&path, &image));
                });
                self.deadline = Instant::now() + Duration::from_secs(5);
            }
            if let Ok(capture) = self.screenshot_rx.try_recv() {
                self.screenshot_result = Some(capture);
            } else if Instant::now() >= self.deadline {
                self.screenshot_result = Some(Err(
                    "egui did not return the requested viewport screenshot".into(),
                ));
            }
        }
        let screenshot_ready = self
            .result
            .as_ref()
            .is_some_and(|result| result.is_err() || self.screenshot_result.is_some());
        if screenshot_ready {
            let result = self
                .result
                .as_ref()
                .expect("screenshot result has a smoke result");
            let screenshot = match &self.screenshot_result {
                Some(Ok((width, height))) => serde_json::json!({
                    "path": self.screenshot_path,
                    "width": width,
                    "height": height,
                    "method": "egui renderer viewport framebuffer"
                }),
                Some(Err(error)) => serde_json::json!({"error": error}),
                None => serde_json::Value::Null,
            };
            let record = match result {
                Ok(()) => serde_json::json!({
                    "ok": true,
                    "native_window": self.window_mode,
                    "document_title": EXPECTED_TITLE,
                    "screenshot": screenshot,
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
    let screenshot_path = output_path.with_extension("png");
    let sidecar_path = output_path.with_extension("png.json");
    let staging_path = output_path.with_extension("capture.json");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create result directory: {error}"))?;
    }
    if output_path.exists()
        || screenshot_path.exists()
        || sidecar_path.exists()
        || staging_path.exists()
    {
        return Err(format!(
            "refusing to overwrite smoke evidence: {}, {}, {} or {} already exists",
            output_path.display(),
            screenshot_path.display(),
            sidecar_path.display(),
            staging_path.display()
        ));
    }

    let root = unique_temp_dir("hermes-html-preview-app-smoke")?;
    fs::create_dir(&root).map_err(|error| format!("create isolated smoke root: {error}"))?;
    let window_mode = serde_json::json!({
        "mode": "offscreen_inactive",
        "position": [-32000, -32000],
        "visible": true,
        "active_requested": false,
        "taskbar": false,
        "input_desktop_switched": false
    });
    let fixture_root = root.join("project");
    let profile_dir = root.join("profile");
    fs::create_dir(&profile_dir).map_err(|error| format!("create isolated profile: {error}"))?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hermes HTML preview smoke")
            .with_inner_size([1440.0, 900.0])
            .with_position(egui::pos2(-32000.0, -32000.0))
            .with_taskbar(false)
            .with_active(false)
            .with_visible(true),
        ..Default::default()
    };
    let result_path = staging_path.clone();
    let profile_for_app = profile_dir.clone();
    let fixture_for_app = fixture_root.clone();
    let preview_path = fixture_root.join("preview.html");
    let screenshot_path_for_app = screenshot_path.clone();
    let window_mode_for_app = window_mode.clone();
    let foreground_before = foreground_process_id();
    let app_process_id = std::process::id();
    let run_result = eframe::run_native(
        "Hermes HTML preview smoke",
        options,
        Box::new(move |cc| {
            let mut hermes = HermesNativeApp::new_for_native_automation(cc, profile_for_app);
            hermes.load_html_preview_fixture_for_native_smoke(&fixture_for_app)?;
            let (screenshot_tx, screenshot_rx) = mpsc::channel();
            Ok(Box::new(SmokeApp {
                hermes,
                window_mode: window_mode_for_app,
                preview_path,
                output_path: result_path,
                screenshot_path: screenshot_path_for_app,
                screenshot_tx,
                screenshot_rx,
                screenshot_requested: false,
                screenshot_result: None,
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
    let mut result: serde_json::Value = serde_json::from_slice(
        &fs::read(&staging_path).map_err(|error| format!("read smoke result: {error}"))?,
    )
    .map_err(|error| format!("parse smoke result: {error}"))?;
    let foreground_after = foreground_process_id();
    result["foreground_evidence"] = serde_json::json!({
        "before_process_id": foreground_before,
        "after_process_id": foreground_after,
        "smoke_process_id": app_process_id,
        "smoke_became_foreground": foreground_after == Some(app_process_id),
        "foreground_process_unchanged": foreground_before.is_some() && foreground_before == foreground_after
    });
    if foreground_after == Some(app_process_id) {
        result["ok"] = serde_json::Value::Bool(false);
        result["error"] = serde_json::Value::String(
            "off-screen smoke process unexpectedly became foreground".into(),
        );
    }
    fs::remove_file(&staging_path)
        .map_err(|error| format!("remove smoke result staging file: {error}"))?;
    let staging_temp_path = staging_path.with_extension("json.tmp");
    if staging_temp_path.exists() {
        fs::remove_file(&staging_temp_path)
            .map_err(|error| format!("remove smoke result temp: {error}"))?;
    }
    atomic_write(&output_path, result.to_string().as_bytes())?;
    if result["ok"] != true {
        return Err(result["error"]
            .as_str()
            .unwrap_or("Hermes HTML preview smoke failed")
            .to_owned());
    }
    let screenshot_bytes = fs::read(&screenshot_path)
        .map_err(|error| format!("read screenshot for provenance: {error}"))?;
    let smoke_state_bytes = fs::read(&output_path)
        .map_err(|error| format!("read smoke state for provenance: {error}"))?;
    let executable =
        std::env::current_exe().map_err(|error| format!("resolve smoke executable: {error}"))?;
    let executable_bytes = fs::read(&executable)
        .map_err(|error| format!("read smoke executable for provenance: {error}"))?;
    let target_commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|error| format!("read target commit: {error}"))?;
    if !target_commit.status.success() {
        return Err("git rev-parse HEAD failed while recording screenshot provenance".into());
    }
    let target_commit = String::from_utf8_lossy(&target_commit.stdout)
        .trim()
        .to_owned();
    let metadata = serde_json::json!({
        "version": 1,
        "hermes_source_sha": "8783054ef6cbafd86f10892241436862c1171a66",
        "doofusdock_sha": "f23ed02a99c7f8c57f83164d98feadb17cc4ed87",
        "target_commit": target_commit,
        "target_dirty": true,
        "smoke_binary_sha256": format!("{:x}", Sha256::digest(executable_bytes)),
        "smoke_state_json_sha256": format!("{:x}", Sha256::digest(smoke_state_bytes)),
        "screenshot_sha256": format!("{:x}", Sha256::digest(screenshot_bytes)),
        "dimensions": {"width": 2160, "height": 1350},
        "route": "/files-preview-fixture/preview.html",
        "state": "HermesNativeApp Files HTML Preview; child WebView loaded relative SVG, hid, and reopened",
        "capture_method": "egui renderer framebuffer; separate native WebView2 child HWND is not included",
        "foreground_evidence": result["foreground_evidence"],
        "gateway": "disabled",
        "profile": "temporary isolated profile",
        "timestamp_utc": time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|error| format!("format provenance timestamp: {error}"))?
    });
    atomic_write(&sidecar_path, metadata.to_string().as_bytes())?;
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
    serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|error| format!("validate smoke result: {error}"))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| format!("create smoke result temp: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("write smoke result temp: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("flush smoke result temp: {error}"))?;
    drop(file);
    fs::hard_link(&temp_path, path)
        .map_err(|error| format!("publish smoke result without overwrite: {error}"))?;
    fs::remove_file(temp_path).map_err(|error| format!("remove smoke result temp: {error}"))
}

fn save_egui_screenshot(
    path: &std::path::Path,
    screenshot: &egui::ColorImage,
) -> Result<(u32, u32), String> {
    let [width, height] = screenshot.size;
    if !(320..=7680).contains(&width) || !(240..=4320).contains(&height) {
        return Err(format!(
            "unexpected egui screenshot dimensions {width}x{height}"
        ));
    }
    let bytes = screenshot
        .pixels
        .iter()
        .flat_map(|color| color.to_array())
        .collect::<Vec<_>>();
    let image = image::RgbaImage::from_raw(width as u32, height as u32, bytes)
        .ok_or("could not assemble egui screenshot pixels")?;
    let parent = path
        .parent()
        .ok_or("screenshot output path has no parent directory")?;
    fs::create_dir_all(parent).map_err(|error| format!("create screenshot directory: {error}"))?;
    let temp_path = path.with_extension("png.tmp");
    let _reservation = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| format!("reserve screenshot temp path: {error}"))?;
    drop(_reservation);
    image
        .save_with_format(&temp_path, image::ImageFormat::Png)
        .map_err(|error| format!("encode egui screenshot: {error}"))?;
    let decoded = image::ImageReader::open(&temp_path)
        .map_err(|error| format!("open egui screenshot for validation: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("detect egui screenshot format: {error}"))?
        .decode()
        .map_err(|error| format!("validate egui screenshot: {error}"))?;
    if decoded.width() != width as u32 || decoded.height() != height as u32 {
        let _ = fs::remove_file(&temp_path);
        return Err("egui screenshot dimensions did not validate".into());
    }
    fs::hard_link(&temp_path, path)
        .map_err(|error| format!("publish egui screenshot without overwrite: {error}"))?;
    fs::remove_file(&temp_path).map_err(|error| format!("remove screenshot temp: {error}"))?;
    Ok((width as u32, height as u32))
}
