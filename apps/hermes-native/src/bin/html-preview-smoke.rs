#![cfg(windows)]

use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use eframe::egui;
use wry::{
    WebView, WebViewBuilder,
    dpi::{LogicalPosition, LogicalSize},
};

const EXPECTED_TITLE: &str = "asset-loaded:16";

struct SmokeApp {
    webview: WebView,
    title_rx: Receiver<String>,
    output_path: PathBuf,
    deadline: Instant,
    observed_titles: Vec<String>,
    result: Option<Result<String, String>>,
}

impl eframe::App for SmokeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        while let Ok(title) = self.title_rx.try_recv() {
            self.observed_titles.push(title.clone());
            if title == EXPECTED_TITLE {
                self.result = Some(Ok(title));
                let _ = self.webview.set_visible(false);
                break;
            }
        }
        if self.result.is_none() && Instant::now() >= self.deadline {
            self.result = Some(Err(format!(
                "Timed out waiting for WebView2 document title {EXPECTED_TITLE:?}; observed titles {:?}; current URL {:?}",
                self.observed_titles,
                self.webview.url()
            )));
        }
        if let Some(result) = &self.result {
            let record = match result {
                Ok(title) => {
                    serde_json::json!({"ok":true,"document_title":title,"checks":["WebView2 child initialized","local HTML loaded","JavaScript executed","relative SVG asset loaded"]})
                }
                Err(error) => serde_json::json!({"ok":false,"error":error}),
            };
            let _ = atomic_write(&self.output_path, record.to_string().as_bytes());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ctx.request_repaint_after(Duration::from_millis(25));
        }
    }
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
    let root =
        std::env::temp_dir().join(format!("hermes-html-preview-smoke-{}", std::process::id()));
    fs::create_dir_all(&root).map_err(|error| format!("create HTML fixture: {error}"))?;
    fs::write(
        root.join("asset.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"><circle cx=\"8\" cy=\"8\" r=\"7\" fill=\"blue\"/></svg>",
    )
    .map_err(|error| format!("write relative asset: {error}"))?;
    fs::write(
        root.join("index.html"),
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>waiting</title></head><body><img src=\"asset.svg\" onload=\"document.title='asset-loaded:'+this.naturalWidth\"></body></html>",
    )
    .map_err(|error| format!("write HTML document: {error}"))?;
    let url = file_url(&root.join("index.html"))?;
    let (title_tx, title_rx) = mpsc::channel();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hermes HTML preview smoke")
            .with_inner_size([640.0, 480.0])
            .with_taskbar(false)
            .with_visible(true),
        ..Default::default()
    };
    let result_path = output_path.clone();
    let run_result = eframe::run_native(
        "Hermes HTML preview smoke",
        options,
        Box::new(move |cc| {
            let bounds = wry::Rect {
                position: LogicalPosition::new(0.0, 0.0).into(),
                size: LogicalSize::new(640.0, 480.0).into(),
            };
            let webview = WebViewBuilder::new()
                .with_url(url)
                .with_bounds(bounds)
                .with_visible(true)
                .with_document_title_changed_handler(move |title| {
                    let _ = title_tx.send(title);
                })
                .build_as_child(cc)
                .map_err(|error| format!("create WebView2 child: {error}"))?;
            Ok(Box::new(SmokeApp {
                webview,
                title_rx,
                output_path: result_path,
                deadline: Instant::now() + Duration::from_secs(15),
                observed_titles: Vec::new(),
                result: None,
            }))
        }),
    );
    let _ = fs::remove_dir_all(&root);
    run_result.map_err(|error| format!("run hidden WebView2 smoke: {error}"))?;
    let result: serde_json::Value = serde_json::from_slice(
        &fs::read(&output_path).map_err(|error| format!("read smoke result: {error}"))?,
    )
    .map_err(|error| format!("parse smoke result: {error}"))?;
    if result["ok"] != true {
        return Err(result["error"]
            .as_str()
            .unwrap_or("WebView2 smoke failed")
            .to_owned());
    }
    println!("{}", result);
    Ok(())
}

fn file_url(path: &std::path::Path) -> Result<String, String> {
    let path = path
        .canonicalize()
        .map_err(|error| format!("canonicalize HTML fixture path: {error}"))?;
    let mut path = path.to_string_lossy().replace('\\', "/");
    if let Some(unc_path) = path.strip_prefix("//?/UNC/") {
        path = format!("//{unc_path}");
    } else if let Some(drive_path) = path.strip_prefix("//?/") {
        path = drive_path.to_owned();
    }
    let is_unc = path.starts_with("//");
    let encoded = path
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect::<String>();
    if is_unc {
        Ok(format!("file://{encoded}"))
    } else {
        Ok(format!("file:///{}", encoded.trim_start_matches('/')))
    }
}

fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes).map_err(|error| format!("write smoke result: {error}"))?;
    serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|error| format!("validate smoke result: {error}"))?;
    fs::rename(&temp_path, path).map_err(|error| format!("publish smoke result: {error}"))
}
