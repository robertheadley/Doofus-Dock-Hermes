#![cfg(windows)]

use std::{
    ffi::c_void,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use eframe::egui;
use hermes_native::app::HermesNativeApp;
use wry::raw_window_handle::HasWindowHandle;

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
    screenshot_path: PathBuf,
    window_handle: isize,
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
            let screenshot = if result.is_ok() {
                match native_capture::capture(self.window_handle, &self.screenshot_path) {
                    Ok((width, height)) => serde_json::json!({
                        "path": self.screenshot_path,
                        "width": width,
                        "height": height
                    }),
                    Err(error) => serde_json::json!({"error": error}),
                }
            } else {
                serde_json::Value::Null
            };
            let record = match result {
                Ok(()) => serde_json::json!({
                    "ok": true,
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
    let screenshot_path = output_path.with_extension("png");
    let profile_for_app = profile_dir.clone();
    let fixture_for_app = fixture_root.clone();
    let preview_path = fixture_root.join("preview.html");
    let run_result = eframe::run_native(
        "Hermes HTML preview smoke",
        options,
        Box::new(move |cc| {
            let window_handle = match cc
                .window_handle()
                .map_err(|error| format!("get native screenshot window handle: {error}"))?
                .as_raw()
            {
                wry::raw_window_handle::RawWindowHandle::Win32(handle) => handle.hwnd.get(),
                _ => return Err("HTML preview smoke requires a Win32 window".into()),
            };
            let mut hermes = HermesNativeApp::new_for_native_automation(cc, profile_for_app);
            hermes.load_html_preview_fixture_for_native_smoke(&fixture_for_app)?;
            Ok(Box::new(SmokeApp {
                hermes,
                preview_path,
                output_path: result_path,
                screenshot_path,
                window_handle,
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

mod native_capture {
    use super::*;
    use std::mem;

    const PW_RENDERFULLCONTENT: u32 = 2;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_pixels_per_meter: i32,
        y_pixels_per_meter: i32,
        colors_used: u32,
        colors_important: u32,
    }

    #[repr(C)]
    struct BitmapInfo {
        header: BitmapInfoHeader,
        colors: [u32; 1],
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetWindowRect(hwnd: *mut c_void, rect: *mut Rect) -> i32;
        fn GetDC(hwnd: *mut c_void) -> *mut c_void;
        fn ReleaseDC(hwnd: *mut c_void, dc: *mut c_void) -> i32;
        fn PrintWindow(hwnd: *mut c_void, dc: *mut c_void, flags: u32) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(dc: *mut c_void) -> *mut c_void;
        fn CreateCompatibleBitmap(dc: *mut c_void, width: i32, height: i32) -> *mut c_void;
        fn SelectObject(dc: *mut c_void, object: *mut c_void) -> *mut c_void;
        fn GetDIBits(
            dc: *mut c_void,
            bitmap: *mut c_void,
            start: u32,
            lines: u32,
            bits: *mut c_void,
            info: *mut BitmapInfo,
            usage: u32,
        ) -> i32;
        fn DeleteObject(object: *mut c_void) -> i32;
        fn DeleteDC(dc: *mut c_void) -> i32;
    }

    pub(super) fn capture(hwnd_value: isize, path: &std::path::Path) -> Result<(u32, u32), String> {
        let hwnd = hwnd_value as *mut c_void;
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return Err("GetWindowRect failed".into());
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if !(320..=7680).contains(&width) || !(240..=4320).contains(&height) {
            return Err(format!("unexpected window dimensions {width}x{height}"));
        }
        let window_dc = unsafe { GetDC(hwnd) };
        if window_dc.is_null() {
            return Err("GetDC failed".into());
        }
        let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
        let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
        if memory_dc.is_null() || bitmap.is_null() {
            if !bitmap.is_null() {
                unsafe { DeleteObject(bitmap) };
            }
            if !memory_dc.is_null() {
                unsafe { DeleteDC(memory_dc) };
            }
            unsafe { ReleaseDC(hwnd, window_dc) };
            return Err("could not allocate screenshot bitmap".into());
        }

        let old_bitmap = unsafe { SelectObject(memory_dc, bitmap) };
        let printed = unsafe { PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) };
        unsafe { SelectObject(memory_dc, old_bitmap) };
        let mut pixels = vec![0_u8; width as usize * height as usize * 4];
        let mut info = BitmapInfo {
            header: BitmapInfoHeader {
                size: mem::size_of::<BitmapInfoHeader>() as u32,
                width,
                height: -height,
                planes: 1,
                bit_count: 32,
                compression: 0,
                size_image: pixels.len() as u32,
                x_pixels_per_meter: 0,
                y_pixels_per_meter: 0,
                colors_used: 0,
                colors_important: 0,
            },
            colors: [0],
        };
        let lines = unsafe {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                pixels.as_mut_ptr().cast(),
                &mut info,
                0,
            )
        };
        unsafe {
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
        }
        if printed == 0 || lines != height {
            return Err(format!(
                "PrintWindow/GetDIBits failed (print={printed}, lines={lines})"
            ));
        }
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        let image = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or("could not assemble screenshot pixels")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create screenshot folder: {error}"))?;
        }
        let temp_path = path.with_extension("png.tmp");
        image
            .save_with_format(&temp_path, image::ImageFormat::Png)
            .map_err(|error| format!("encode screenshot PNG: {error}"))?;
        let decoded = image::ImageReader::open(&temp_path)
            .map_err(|error| format!("open screenshot PNG for validation: {error}"))?
            .with_guessed_format()
            .map_err(|error| format!("detect screenshot PNG format: {error}"))?
            .decode()
            .map_err(|error| format!("validate screenshot PNG: {error}"))?;
        if decoded.width() != width as u32 || decoded.height() != height as u32 {
            let _ = fs::remove_file(&temp_path);
            return Err("screenshot dimensions did not validate".into());
        }
        let _ = fs::remove_file(path);
        fs::rename(&temp_path, path).map_err(|error| format!("publish screenshot: {error}"))?;
        Ok((width as u32, height as u32))
    }
}
