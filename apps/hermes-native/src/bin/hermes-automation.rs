use egui::Vec2;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use hermes_native::app::{HermesNativeApp, RegisteredUiAction, registered_ui_action_catalog};
use serde_json::{Value, json};
use std::{env, path::PathBuf, process};

#[path = "hermes_automation/automation_protocol.rs"]
mod automation_protocol;

#[derive(Debug)]
enum Action {
    Inspect,
    Click(String),
    Scroll(String, bool),
    Type(String, String),
    TypeByRole(String, String, String),
    Screenshot(PathBuf),
}

pub(crate) struct TemporaryProfile(pub(crate) PathBuf);

impl TemporaryProfile {
    pub(crate) fn create() -> Result<Self, String> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("system clock is before UNIX epoch: {error}"))?
            .as_nanos();
        let path = env::temp_dir().join(format!("HermesNative-Agent-{}-{nonce}", process::id()));
        std::fs::create_dir(&path)
            .map_err(|error| format!("create isolated profile {}: {error}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for TemporaryProfile {
    fn drop(&mut self) {
        let temp_root = env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| env::temp_dir());
        let parent = self.0.parent().and_then(|path| path.canonicalize().ok());
        let name_is_scoped = self
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("HermesNative-Agent-"));
        if parent.as_ref() == Some(&temp_root) && name_is_scoped {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                eprintln!("hermes-automation: cleanup {}: {error}", self.0.display());
            }
        } else {
            eprintln!(
                "hermes-automation: refused to remove unscoped temporary profile {}",
                self.0.display()
            );
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("hermes-automation: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.as_slice() == ["--native-window"] {
        return run_native_window(false);
    }
    if args.as_slice() == ["--native-window", "--stdio"] {
        return run_native_window(true);
    }
    if args.as_slice() == ["--stdio"] {
        return automation_protocol::run_stdio();
    }
    if args.iter().any(|arg| arg == "--stdio") {
        return Err("--stdio cannot be combined with one-shot UI arguments".into());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") || args.is_empty() {
        print_help();
        return Ok(());
    }
    let mut width = 1440.0_f32;
    let mut height = 900.0_f32;
    let mut actions = Vec::new();
    let mut markdown_fixture = false;
    let mut first_run_setup_fixture = false;
    let mut long_session_fixture = false;
    let mut transcript_find_fixture = false;
    let mut transcript_timestamps_fixture = false;
    let mut session_pane_fixture = false;
    let mut recent_session_switcher_fixture = false;
    let mut recent_session_reorder_fixture = false;
    let mut command_center_sessions_fixture = false;
    let mut plugins_catalog_fixture = false;
    let mut plugin_install_review_fixture = false;
    let mut plugin_update_scan_review_fixture = false;
    let mut plugin_auto_update_fixture = false;
    let mut plugin_update_all_report_fixture = false;
    let mut connectors_fixture = false;
    let mut artifacts_fixture = false;
    let mut files_preview_fixture = false;
    let mut image_preview_fixture = false;
    let mut svg_preview_fixture = false;
    let mut pdf_preview_fixture = false;
    let mut pdf_scroll_fixture = false;
    let mut html_preview_fixture = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--viewport" => {
                let value = take(&args, &mut index, "--viewport")?;
                let (w, h) = value
                    .split_once('x')
                    .ok_or("viewport must be WIDTHxHEIGHT")?;
                width = w.parse().map_err(|_| "invalid viewport width")?;
                height = h.parse().map_err(|_| "invalid viewport height")?;
                if !(320.0..=7680.0).contains(&width) || !(240.0..=4320.0).contains(&height) {
                    return Err("viewport is outside supported bounds".into());
                }
            }
            "--inspect" => actions.push(Action::Inspect),
            "--markdown-fixture" => markdown_fixture = true,
            "--first-run-setup-fixture" => first_run_setup_fixture = true,
            "--long-session-fixture" => long_session_fixture = true,
            "--transcript-find-fixture" => transcript_find_fixture = true,
            "--transcript-timestamps-fixture" => transcript_timestamps_fixture = true,
            "--session-pane-fixture" => session_pane_fixture = true,
            "--recent-session-switcher-fixture" => recent_session_switcher_fixture = true,
            "--recent-session-reorder-fixture" => recent_session_reorder_fixture = true,
            "--command-center-sessions-fixture" => command_center_sessions_fixture = true,
            "--plugins-catalog-fixture" => plugins_catalog_fixture = true,
            "--plugin-install-review-fixture" => plugin_install_review_fixture = true,
            "--plugin-update-scan-review-fixture" => plugin_update_scan_review_fixture = true,
            "--plugin-auto-update-fixture" => plugin_auto_update_fixture = true,
            "--plugin-update-all-report-fixture" => plugin_update_all_report_fixture = true,
            "--connectors-fixture" => connectors_fixture = true,
            "--artifacts-fixture" => artifacts_fixture = true,
            "--files-preview-fixture" => files_preview_fixture = true,
            "--image-preview-fixture" => image_preview_fixture = true,
            "--svg-preview-fixture" => svg_preview_fixture = true,
            "--pdf-preview-fixture" => pdf_preview_fixture = true,
            "--pdf-scroll-fixture" => pdf_scroll_fixture = true,
            "--html-preview-fixture" => html_preview_fixture = true,
            "--click" => actions.push(Action::Click(take(&args, &mut index, "--click")?)),
            "--scroll-down" => actions.push(Action::Scroll(
                take(&args, &mut index, "--scroll-down")?,
                true,
            )),
            "--scroll-up" => actions.push(Action::Scroll(
                take(&args, &mut index, "--scroll-up")?,
                false,
            )),
            "--type" => {
                let label = take(&args, &mut index, "--type")?;
                let text = take(&args, &mut index, "--type text")?;
                actions.push(Action::Type(label, text));
            }
            "--type-role" => {
                let role = take(&args, &mut index, "--type-role")?;
                let label = take(&args, &mut index, "--type-role accessible name")?;
                let text = take(&args, &mut index, "--type-role text")?;
                actions.push(Action::TypeByRole(role, label, text));
            }
            "--screenshot" => actions.push(Action::Screenshot(PathBuf::from(take(
                &args,
                &mut index,
                "--screenshot",
            )?))),
            other => return Err(format!("unknown option: {other}")),
        }
        index += 1;
    }
    if [
        markdown_fixture,
        first_run_setup_fixture,
        long_session_fixture,
        transcript_find_fixture,
        transcript_timestamps_fixture,
        session_pane_fixture,
        recent_session_switcher_fixture,
        recent_session_reorder_fixture,
        command_center_sessions_fixture,
        plugins_catalog_fixture,
        plugin_install_review_fixture,
        plugin_update_scan_review_fixture,
        plugin_auto_update_fixture,
        plugin_update_all_report_fixture,
        connectors_fixture,
        artifacts_fixture,
        files_preview_fixture,
        image_preview_fixture,
        svg_preview_fixture,
        pdf_preview_fixture,
        pdf_scroll_fixture,
        html_preview_fixture,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
        > 1
    {
        return Err("UI fixtures are mutually exclusive".into());
    }
    let profile = TemporaryProfile::create()?;
    let profile_path = profile.0.clone();
    let app = std::rc::Rc::new(std::cell::RefCell::new(
        HermesNativeApp::new_for_automation(profile_path.clone()),
    ));
    if markdown_fixture {
        app.borrow_mut().load_markdown_fixture_for_automation();
    } else if first_run_setup_fixture {
        app.borrow_mut()
            .load_first_run_setup_fixture_for_automation();
    } else if long_session_fixture {
        app.borrow_mut().load_long_session_fixture_for_automation();
    } else if transcript_find_fixture {
        app.borrow_mut()
            .load_transcript_find_fixture_for_automation();
    } else if transcript_timestamps_fixture {
        app.borrow_mut()
            .load_transcript_timestamps_fixture_for_automation();
    } else if session_pane_fixture {
        app.borrow_mut().load_session_pane_fixture_for_automation();
    } else if recent_session_switcher_fixture {
        app.borrow_mut()
            .load_recent_session_switcher_fixture_for_automation();
    } else if recent_session_reorder_fixture {
        app.borrow_mut()
            .load_recent_session_reorder_fixture_for_automation();
    } else if command_center_sessions_fixture {
        app.borrow_mut()
            .load_command_center_sessions_fixture_for_automation();
    } else if plugins_catalog_fixture {
        app.borrow_mut()
            .load_plugin_catalog_fixture_for_automation();
    } else if plugin_install_review_fixture {
        app.borrow_mut()
            .load_catalog_plugin_install_review_fixture_for_automation();
    } else if plugin_update_scan_review_fixture {
        app.borrow_mut()
            .load_plugin_update_scan_review_fixture_for_automation();
    } else if plugin_auto_update_fixture {
        app.borrow_mut()
            .load_plugin_auto_update_fixture_for_automation();
    } else if plugin_update_all_report_fixture {
        app.borrow_mut()
            .load_plugin_update_all_report_fixture_for_automation();
    } else if connectors_fixture {
        app.borrow_mut().load_connectors_fixture_for_automation();
    } else if artifacts_fixture {
        app.borrow_mut().load_artifacts_fixture_for_automation()?;
    } else if files_preview_fixture {
        app.borrow_mut().load_files_preview_fixture_for_automation(
            &profile_path.join("files-preview-project"),
            false,
            false,
            false,
            false,
            false,
        )?;
    } else if image_preview_fixture {
        app.borrow_mut().load_files_preview_fixture_for_automation(
            &profile_path.join("image-preview-project"),
            true,
            false,
            false,
            false,
            false,
        )?;
    } else if svg_preview_fixture {
        app.borrow_mut().load_files_preview_fixture_for_automation(
            &profile_path.join("svg-preview-project"),
            false,
            true,
            false,
            false,
            false,
        )?;
    } else if pdf_preview_fixture {
        app.borrow_mut().load_files_preview_fixture_for_automation(
            &profile_path.join("pdf-preview-project"),
            false,
            false,
            true,
            false,
            false,
        )?;
    } else if pdf_scroll_fixture {
        app.borrow_mut().load_files_preview_fixture_for_automation(
            &profile_path.join("pdf-scroll-project"),
            false,
            false,
            true,
            true,
            false,
        )?;
    } else if html_preview_fixture {
        app.borrow_mut().load_files_preview_fixture_for_automation(
            &profile_path.join("html-preview-project"),
            false,
            false,
            false,
            false,
            true,
        )?;
    }
    let app_for_frame = app.clone();
    let mut builder = Harness::builder().with_size(Vec2::new(width, height));
    if recent_session_switcher_fixture {
        builder = builder.with_context_setup(|ctx| {
            ctx.input_mut(|input| input.modifiers = egui::Modifiers::CTRL);
        });
    }
    let mut harness = builder.build_ui(move |ui| app_for_frame.borrow_mut().draw(ui));
    if artifacts_fixture {
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            harness.step();
        }
    }
    for action in actions {
        match action {
            Action::Inspect => {
                let tree = describe_node(harness.root());
                println!(
                    "{}",
                    json!({"ok":true,"action":"inspect","profile":profile_path,"tree":tree})
                );
            }
            Action::Click(label) => {
                let node = harness
                    .query_by_label(&label)
                    .ok_or_else(|| format!("no accessible control named {label:?}"))?;
                let role = format!("{:?}", node.accesskit_node().role());
                node.click();
                harness.run();
                println!(
                    "{}",
                    json!({"ok":true,"action":"click","label":label,"role":role})
                );
            }
            Action::Scroll(label, down) => {
                let node = harness
                    .query_by_label(&label)
                    .ok_or_else(|| format!("no accessible node named {label:?}"))?;
                if down {
                    node.scroll_down();
                } else {
                    node.scroll_up();
                }
                harness.run();
                println!(
                    "{}",
                    json!({"ok":true,"action":if down { "scroll_down" } else { "scroll_up" },"label":label,"distance_points":100})
                );
            }
            Action::Type(label, text) => {
                let role;
                {
                    let node = harness
                        .query_by_label(&label)
                        .ok_or_else(|| format!("no accessible control named {label:?}"))?;
                    role = format!("{:?}", node.accesskit_node().role());
                    node.click();
                }
                harness.run();
                let node = harness
                    .query_by_label(&label)
                    .ok_or_else(|| format!("control {label:?} disappeared after focus"))?;
                node.type_text(&text);
                harness.run();
                println!(
                    "{}",
                    json!({"ok":true,"action":"type","label":label,"role":role,"characters":text.chars().count()})
                );
            }
            Action::TypeByRole(role, label, text) => {
                {
                    let mut matches = harness.query_all_by(|node| {
                        format!("{:?}", node.role()) == role
                            && node.label().as_deref() == Some(label.as_str())
                    });
                    let node = matches
                        .next()
                        .ok_or_else(|| format!("no accessible {role} control named {label:?}"))?;
                    if matches.next().is_some() {
                        return Err(format!(
                            "multiple accessible {role} controls named {label:?}"
                        ));
                    }
                    node.click();
                }
                harness.run();
                {
                    let mut matches = harness.query_all_by(|node| {
                        format!("{:?}", node.role()) == role
                            && node.label().as_deref() == Some(label.as_str())
                    });
                    let node = matches.next().ok_or_else(|| {
                        format!("{role} control named {label:?} disappeared after focus")
                    })?;
                    if matches.next().is_some() {
                        return Err(format!(
                            "multiple accessible {role} controls named {label:?} after focus"
                        ));
                    }
                    node.type_text(&text);
                }
                harness.run();
                println!(
                    "{}",
                    json!({"ok":true,"action":"type","role":role,"label":label,"characters":text.chars().count()})
                );
            }
            Action::Screenshot(path) => {
                let image = harness
                    .render()
                    .map_err(|error| format!("render screenshot: {error}"))?;
                image
                    .save(&path)
                    .map_err(|error| format!("write screenshot {}: {error}", path.display()))?;
                println!(
                    "{}",
                    json!({"ok":true,"action":"screenshot","path":path,"width":image.width(),"height":image.height()})
                );
            }
        }
    }
    Ok(())
}

fn take(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} requires a value"))
}

fn describe_node(node: egui_kittest::Node<'_>) -> Value {
    let access = node.accesskit_node();
    let children = node.children().map(describe_node).collect::<Vec<_>>();
    json!({
        "role": format!("{:?}", access.role()),
        "name": access.label(),
        "value": access.value(),
        "children": children
    })
}

fn print_help() {
    println!("Hermes Native background UI agent runner
Usage: hermes-automation [--native-window] [--viewport WIDTHxHEIGHT] [--markdown-fixture | --first-run-setup-fixture | --long-session-fixture | --transcript-find-fixture | --transcript-timestamps-fixture | --session-pane-fixture | --recent-session-switcher-fixture | --recent-session-reorder-fixture | --command-center-sessions-fixture | --plugins-catalog-fixture | --plugin-install-review-fixture | --plugin-update-scan-review-fixture | --plugin-auto-update-fixture | --plugin-update-all-report-fixture | --connectors-fixture | --artifacts-fixture | --files-preview-fixture | --image-preview-fixture | --svg-preview-fixture | --pdf-preview-fixture | --pdf-scroll-fixture | --html-preview-fixture] [--inspect] [--click ACCESSIBLE_NAME] [--scroll-up ACCESSIBLE_NAME | --scroll-down ACCESSIBLE_NAME] [--type ACCESSIBLE_NAME TEXT | --type-role ROLE ACCESSIBLE_NAME TEXT] [--screenshot PATH]...
Actions run in order against an isolated offline Hermes Native UI. No native window is opened and no Hermes gateway is started.
Accessible actions use egui's AccessKit UI tree and normal widget input handling.
--native-window opens the actual app off-screen and inactive on Windows, with a temporary profile and gateway disabled. Add --stdio for generation-checked egui input and renderer screenshot requests; inspect controls through Windows UI Automation.");
}

#[cfg(windows)]
fn run_native_window(with_stdio: bool) -> Result<(), String> {
    use std::{
        io::{BufRead, BufReader, BufWriter, Write},
        sync::mpsc,
        thread,
    };

    let profile = TemporaryProfile::create()?;
    let profile_dir = profile.0.clone();
    let (request_tx, request_rx) = mpsc::sync_channel::<Value>(64);
    let (response_tx, response_rx) = mpsc::channel::<Value>();
    let repaint_context = std::sync::Arc::new(std::sync::Mutex::new(None::<egui::Context>));
    if with_stdio {
        let response_tx_for_reader = response_tx.clone();
        thread::Builder::new()
            .name("hermes-native-agent-stdout".into())
            .spawn(move || {
                let stdout = std::io::stdout();
                let mut writer = BufWriter::new(stdout.lock());
                for mut response in response_rx {
                    if let Some(object) = response.as_object_mut() {
                        object.insert("version".into(), serde_json::json!(1));
                    }
                    if serde_json::to_writer(&mut writer, &response).is_err()
                        || writer.write_all(b"\n").is_err()
                        || writer.flush().is_err()
                    {
                        break;
                    }
                }
            })
            .map_err(|error| format!("start native agent response writer: {error}"))?;
        let repaint_context_for_reader = repaint_context.clone();
        thread::Builder::new()
            .name("hermes-native-agent-stdin".into())
            .spawn(move || {
                let stdin = std::io::stdin();
                let mut reader = BufReader::new(stdin.lock());
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) if line.len() > 64 * 1024 => break,
                        Ok(_) => match serde_json::from_str::<Value>(&line) {
                            Ok(request) => {
                                if request_tx.send(request).is_err() {
                                    break;
                                }
                                if let Ok(context) = repaint_context_for_reader.lock()
                                    && let Some(context) = context.as_ref()
                                {
                                    context.request_repaint();
                                }
                            }
                            Err(error) => {
                                let _ = response_tx_for_reader.send(serde_json::json!({
                                    "id": null,
                                    "status": "failed",
                                    "error": {"code":"invalid_json","message":error.to_string()}
                                }));
                            }
                        },
                    }
                }
            })
            .map_err(|error| format!("start native agent request reader: {error}"))?;
        response_tx
            .send(serde_json::json!({
                "type":"ready",
                "version":1,
                "protocol":"hermes-native-window/1",
                "process_id":process::id(),
                "gateway":"disabled",
                "state_generation":0
            }))
            .map_err(|error| format!("publish native agent readiness: {error}"))?;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hermes Native Agent Window")
            .with_inner_size([1440.0, 900.0])
            .with_position(egui::pos2(-32000.0, -32000.0))
            .with_taskbar(false)
            .with_active(false)
            .with_visible(true),
        ..Default::default()
    };
    eframe::run_native(
        "Hermes Native Agent Window",
        options,
        Box::new(move |cc| {
            if let Ok(mut repaint_context) = repaint_context.lock() {
                *repaint_context = Some(cc.egui_ctx.clone());
            }
            let hwnd = native_window_handle(cc).map_err(|error| {
                Box::new(std::io::Error::other(error)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            set_window_no_activate(hwnd).map_err(|error| {
                Box::new(std::io::Error::other(error)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            Ok(Box::new(NativeAgentWindow {
                app: HermesNativeApp::new_for_native_automation(cc, profile_dir),
                hwnd,
                control: with_stdio.then(|| NativeControl {
                    requests: request_rx,
                    responses: response_tx,
                    generation: 0,
                    seen: std::collections::HashSet::new(),
                    closed: false,
                    pending_click: None,
                    pending_tree: None,
                    pending_action: None,
                    last_tree: None,
                }),
                pending_responses: Vec::new(),
                pending_screenshots: Vec::new(),
            }))
        }),
    )
    .map_err(|error| format!("run isolated Hermes Native agent window: {error}"))?;
    drop(profile);
    Ok(())
}

#[cfg(not(windows))]
fn run_native_window(_with_stdio: bool) -> Result<(), String> {
    Err("--native-window is currently supported only on Windows".into())
}

#[cfg(windows)]
struct NativeAgentWindow {
    app: HermesNativeApp,
    hwnd: *mut std::ffi::c_void,
    control: Option<NativeControl>,
    pending_responses: Vec<serde_json::Value>,
    pending_screenshots: Vec<(serde_json::Value, u64)>,
}

#[cfg(windows)]
struct NativeControl {
    requests: std::sync::mpsc::Receiver<serde_json::Value>,
    responses: std::sync::mpsc::Sender<serde_json::Value>,
    generation: u64,
    seen: std::collections::HashSet<String>,
    closed: bool,
    pending_click: Option<PendingNativeClick>,
    pending_tree: Option<PendingNativeTree>,
    pending_action: Option<PendingNativeSemanticAction>,
    last_tree: Option<NativeTreeSnapshot>,
}

#[cfg(windows)]
struct PendingNativeClick {
    request_id: serde_json::Value,
    point: egui::Pos2,
    x: f64,
    y: f64,
}

#[cfg(windows)]
struct PendingNativeTree {
    request_id: serde_json::Value,
    state_generation: u64,
    receiver: std::sync::mpsc::Receiver<Result<serde_json::Value, String>>,
}

#[cfg(windows)]
struct NativeTreeSnapshot {
    state_generation: u64,
    tree: serde_json::Value,
}

#[cfg(windows)]
struct PendingNativeSemanticAction {
    request_id: serde_json::Value,
    request: egui::accesskit::ActionRequest,
    action_name: String,
    target: serde_json::Value,
    before_route: serde_json::Value,
    before_state: serde_json::Value,
}

#[cfg(windows)]
#[derive(Default)]
struct NativeInputBatch {
    responses: Vec<serde_json::Value>,
    screenshots: Vec<(serde_json::Value, u64)>,
}

#[cfg(windows)]
impl NativeControl {
    fn process_input(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        app: &mut HermesNativeApp,
        hwnd: usize,
    ) -> NativeInputBatch {
        use egui::{Event, PointerButton, pos2};
        use serde_json::{Value, json};
        use std::sync::mpsc::TryRecvError;

        let mut batch = NativeInputBatch::default();
        if let Some(pending) = &self.pending_tree {
            match pending.receiver.try_recv() {
                Ok(Ok(tree)) => {
                    let pending = self.pending_tree.take().expect("pending tree exists");
                    self.last_tree = Some(NativeTreeSnapshot {
                        state_generation: pending.state_generation,
                        tree: tree.clone(),
                    });
                    batch.responses.push(json!({
                        "id":pending.request_id,
                        "status":"succeeded",
                        "result":{
                            "state_generation":pending.state_generation,
                            "tree":tree,
                            "window_foreground_requested":false
                        }
                    }));
                }
                Ok(Err(message)) => {
                    let pending = self.pending_tree.take().expect("pending tree exists");
                    batch.responses.push(json!({
                        "id":pending.request_id,
                        "status":"failed",
                        "error":{"code":"accessibility_tree_unavailable","message":message,"state_generation":pending.state_generation}
                    }));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return batch,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let pending = self.pending_tree.take().expect("pending tree exists");
                    batch.responses.push(json!({
                        "id":pending.request_id,
                        "status":"failed",
                        "error":{"code":"accessibility_worker_stopped","message":"native UI Automation query worker stopped before returning a tree","state_generation":pending.state_generation}
                    }));
                }
            }
        }
        if let Some(click) = self.pending_click.take() {
            input.focused = true;
            input.events.push(Event::PointerButton {
                pos: click.point,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            });
            self.generation = self.generation.saturating_add(1);
            batch.responses.push(json!({
                "id":click.request_id,
                "status":"failed",
                "error":{
                    "code":"target_effect_unverified",
                    "message":"synthetic pointer events were dispatched, but the target effect was not verified",
                    "input_dispatched":true,
                    "state_generation":self.generation,
                    "x":click.x,
                    "y":click.y,
                    "window_foreground_requested":false
                }
            }));
            return batch;
        }
        for _ in 0..32 {
            let request = match self.requests.try_recv() {
                Ok(request) => request,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            let Some(id_text) = id.as_str() else {
                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_request","message":"id must be a string"}}));
                continue;
            };
            if id_text.is_empty() || id_text.len() > 128 {
                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_request","message":"id must be 1 to 128 UTF-8 bytes"}}));
                continue;
            }
            if self.seen.contains(id_text) {
                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"duplicate_request","message":"request id was already used"}}));
                continue;
            }
            if self.seen.len() >= 4096 {
                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"request_limit","message":"native host reached its 4096 unique-request limit; restart the isolated process"}}));
                continue;
            }
            self.seen.insert(id_text.to_owned());
            if request.get("version").and_then(Value::as_u64) != Some(1) {
                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"unsupported_version","message":"version must be 1"}}));
                continue;
            }
            match request.get("method").and_then(Value::as_str) {
                Some("health.get") => batch.responses.push(json!({"id":id,"status":"succeeded","result":{"process_id":process::id(),"state_generation":self.generation,"gateway":"disabled","window":"offscreen-inactive"}})),
                Some("state.get") => {
                    let (title, path) = app.automation_route();
                    let mut snapshot = app.automation_state();
                    if let Some(object) = snapshot.as_object_mut() {
                        object.insert("state_generation".into(), json!(self.generation));
                    }
                    batch.responses.push(json!({
                        "id":id,
                        "status":"succeeded",
                        "result":{"state_generation":self.generation,"route":{"title":title,"path":path},"snapshot":snapshot}
                    }));
                }
                Some("diagnostics.bundle") => {
                    let (title, path) = app.automation_route();
                    let mut snapshot = app.automation_state();
                    if let Some(object) = snapshot.as_object_mut() {
                        object.insert("state_generation".into(), json!(self.generation));
                    }
                    batch.responses.push(json!({
                        "id":id,
                        "status":"succeeded",
                        "result":{
                            "protocol":"hermes-native-window/1",
                            "process_id":process::id(),
                            "window":"offscreen-inactive",
                            "gateway":"disabled",
                            "native_window":true,
                            "route":{"title":title,"path":path},
                            "state":snapshot
                        }
                    }));
                }
                Some("app.actions.list") => batch.responses.push(json!({
                    "id":id,
                    "status":"succeeded",
                    "result":registered_ui_action_catalog()
                })),
                Some("ui.tree") => {
                    let expected = request["params"]["expected_state_generation"].as_u64();
                    if expected.is_some_and(|expected| expected != self.generation) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}}));
                        continue;
                    }
                    let state_generation = self.generation;
                    let (sender, receiver) = std::sync::mpsc::channel();
                    let repaint_ctx = ctx.clone();
                    let spawn_result = std::thread::Builder::new()
                        .name("hermes-native-uia-tree".into())
                        .spawn(move || {
                            let _ = sender.send(read_native_uia_tree(hwnd));
                            repaint_ctx.request_repaint();
                        });
                    if let Err(error) = spawn_result {
                        batch.responses.push(json!({
                            "id":id,
                            "status":"failed",
                            "error":{"code":"accessibility_worker_start_failed","message":format!("start native UI Automation tree query: {error}")}
                        }));
                        continue;
                    }
                    self.pending_tree = Some(PendingNativeTree {
                        request_id: id,
                        state_generation,
                        receiver,
                    });
                    return batch;
                }
                Some("ui.action") => {
                    let params = &request["params"];
                    if params["expected_state_generation"].as_u64() != Some(self.generation) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}}));
                        continue;
                    }
                    let Some(snapshot) = &self.last_tree else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"tree_required","message":"query ui.tree at the current state_generation before resolving a native locator","state_generation":self.generation}}));
                        continue;
                    };
                    if snapshot.state_generation != self.generation {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_tree","message":"query ui.tree again after the last state change","state_generation":self.generation}}));
                        continue;
                    }
                    if snapshot.tree["truncated"].as_bool() == Some(true) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"tree_truncated","message":"native locator actions require a complete UI Automation tree snapshot","state_generation":self.generation}}));
                        continue;
                    }
                    let Some(action_name) = params["action"].as_str() else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"action must be click, focus, set_value or replace_selected_text"}}));
                        continue;
                    };
                    let locator = native_tree_locator(params);
                    let locator = match locator {
                        Ok(locator) => locator,
                        Err(message) => {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_locator","message":message}}));
                            continue;
                        }
                    };
                    let nodes = snapshot.tree["nodes"].as_array().expect("validated tree node array");
                    let matches = nodes.iter().filter(|node| native_node_matches(node, &locator)).collect::<Vec<_>>();
                    if matches.len() != 1 {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":if matches.is_empty() { "target_not_found" } else { "target_ambiguous" },"message":format!("native locator matched {} controls; exactly one is required",matches.len()),"state_generation":self.generation}}));
                        continue;
                    }
                    let target = matches[0];
                    if target["enabled"].as_bool() != Some(true) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_disabled","message":"native locator target is disabled or unavailable","state_generation":self.generation}}));
                        continue;
                    }
                    let Some(node_id) = target["accesskit_node_id"].as_u64().filter(|node_id| *node_id != 0) else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_not_actionable","message":"native control has no AccessKit node ID in the current tree","state_generation":self.generation}}));
                        continue;
                    };
                    let actions = target["actions"].as_array().cloned().unwrap_or_default();
                    let supports_action = |name: &str| actions.iter().any(|action| action.as_str() == Some(name));
                    if action_name == "replace_selected_text" {
                        // SAFETY: this high-entropy ID came from the current egui AccessKit tree.
                        let widget_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
                        if ctx.memory(|memory| memory.focused()) != Some(widget_id) {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_not_focused","message":"focus this text control through ui.action before replacing selected text","state_generation":self.generation}}));
                            continue;
                        }
                    }
                    let (action, data) = match action_name {
                        "focus" if supports_action("focus") => {
                            (egui::accesskit::Action::Focus, None)
                        }
                        "click" if supports_action("click") => (egui::accesskit::Action::Click, None),
                        "set_value" if supports_action("set_value") => {
                            let Some(value) = params["value"].as_str() else {
                                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"set_value requires a string value"}}));
                                continue;
                            };
                            if value.chars().count() > 16_384 {
                                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"input_too_large","message":"value exceeds 16384 Unicode characters"}}));
                                continue;
                            }
                            (egui::accesskit::Action::SetValue, Some(egui::accesskit::ActionData::Value(value.to_owned().into_boxed_str())))
                        }
                        "replace_selected_text" if supports_action("replace_selected_text") => {
                            let Some(value) = params["value"].as_str() else {
                                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"replace_selected_text requires a string value"}}));
                                continue;
                            };
                            if value.chars().count() > 16_384 {
                                batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"input_too_large","message":"value exceeds 16384 Unicode characters"}}));
                                continue;
                            }
                            (egui::accesskit::Action::ReplaceSelectedText, Some(egui::accesskit::ActionData::Value(value.to_owned().into_boxed_str())))
                        }
                        "click" | "focus" | "set_value" | "replace_selected_text" => {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"action_unsupported","message":format!("target does not expose the native pattern required for {action_name}"),"state_generation":self.generation}}));
                            continue;
                        }
                        _ => {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"unsupported_action","message":"action must be click, focus, set_value or replace_selected_text"}}));
                            continue;
                        }
                    };
                    let accesskit_request = egui::accesskit::ActionRequest {
                        action,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        target_node: egui::accesskit::NodeId(node_id),
                        data,
                    };
                    let (before_title, before_path) = app.automation_route();
                    self.pending_action = Some(PendingNativeSemanticAction {
                        request_id: id,
                        request: accesskit_request.clone(),
                        action_name: action_name.to_owned(),
                        target: json!({
                            "role":target["role"],
                            "name":target["name"],
                            "automation_id":target["automation_id"],
                            "accesskit_node_id":node_id,
                            "bounds":target["bounds"]
                        }),
                        before_route: json!({"title":before_title,"path":before_path}),
                        before_state: app.automation_state(),
                    });
                    input.events.push(egui::Event::AccessKitActionRequest(accesskit_request));
                    ctx.request_repaint();
                    return batch;
                }
                Some("ui.keypress") => {
                    let params = &request["params"];
                    if params["expected_state_generation"].as_u64() != Some(self.generation) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}}));
                        continue;
                    }
                    let Some(key_name) = params["key"].as_str() else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"key must be a supported key name"}}));
                        continue;
                    };
                    let Some(key) = automation_protocol::parse_key(key_name) else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"key must be Enter, Escape, Tab, Space, Backspace, Delete, an arrow, Home, End, PageUp, PageDown or A-Z"}}));
                        continue;
                    };
                    let modifiers = match automation_protocol::parse_modifiers(params.get("modifiers")) {
                        Ok(modifiers) => modifiers,
                        Err(message) => {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":message}}));
                            continue;
                        }
                    };
                    input.focused = true;
                    input.events.extend([
                        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers },
                        Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers },
                    ]);
                    self.generation = self.generation.saturating_add(1);
                    self.last_tree = None;
                    batch.responses.push(json!({
                        "id":id,
                        "status":"failed",
                        "error":{
                            "code":"target_effect_unverified",
                            "message":"keyboard input was dispatched without activating the native window; read state.get to verify its effect before retrying",
                            "input_dispatched":true,
                            "key":key_name,
                            "modifiers":params.get("modifiers").cloned().unwrap_or_else(|| json!([])),
                            "state_generation":self.generation
                        }
                    }));
                }
                Some("ui.scroll") => {
                    let params = &request["params"];
                    if params["expected_state_generation"].as_u64() != Some(self.generation) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}}));
                        continue;
                    }
                    let Some(snapshot) = &self.last_tree else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"tree_required","message":"query ui.tree at the current state_generation before resolving a native scroll target","state_generation":self.generation}}));
                        continue;
                    };
                    if snapshot.state_generation != self.generation {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_tree","message":"query ui.tree again after the last state change","state_generation":self.generation}}));
                        continue;
                    }
                    if snapshot.tree["truncated"].as_bool() == Some(true) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"tree_truncated","message":"native scrolling requires a complete UI Automation tree snapshot","state_generation":self.generation}}));
                        continue;
                    }
                    let delta_x = params["delta_x"].as_f64();
                    let delta_y = params["delta_y"].as_f64();
                    let (Some(delta_x), Some(delta_y)) = (delta_x, delta_y) else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"delta_x and delta_y must be finite numbers"}}));
                        continue;
                    };
                    if !delta_x.is_finite() || !delta_y.is_finite() || (delta_x == 0.0 && delta_y == 0.0) || delta_x.abs() > 3000.0 || delta_y.abs() > 3000.0 {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"scroll deltas must be finite, non-zero and within 3000 points"}}));
                        continue;
                    }
                    let locator = match native_tree_locator(params) {
                        Ok(locator) => locator,
                        Err(message) => {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_locator","message":message}}));
                            continue;
                        }
                    };
                    let nodes = snapshot.tree["nodes"].as_array().expect("validated tree node array");
                    let matches = nodes.iter().filter(|node| native_node_matches(node, &locator)).collect::<Vec<_>>();
                    if matches.len() != 1 {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":if matches.is_empty() { "target_not_found" } else { "target_ambiguous" },"message":format!("native scroll locator matched {} controls; exactly one is required",matches.len()),"state_generation":self.generation}}));
                        continue;
                    }
                    let target = matches[0];
                    if target["enabled"].as_bool() != Some(true) || target["bounds_valid"].as_bool() == Some(false) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_not_actionable","message":"native scroll target is disabled or has invalid bounds","state_generation":self.generation}}));
                        continue;
                    }
                    let Some(root) = nodes.first() else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"tree_invalid","message":"native UI Automation tree has no root node","state_generation":self.generation}}));
                        continue;
                    };
                    let rect_number = |node: &Value, key: &str| node["bounds"][key].as_f64();
                    let (Some(root_x), Some(root_y), Some(root_width), Some(root_height), Some(target_x), Some(target_y), Some(target_width), Some(target_height)) = (
                        rect_number(root, "x"), rect_number(root, "y"), rect_number(root, "width"), rect_number(root, "height"),
                        rect_number(target, "x"), rect_number(target, "y"), rect_number(target, "width"), rect_number(target, "height"),
                    ) else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_has_no_bounds","message":"native scroll target or window root has no finite bounds","state_generation":self.generation}}));
                        continue;
                    };
                    if root_width <= 0.0 || root_height <= 0.0 || target_width <= 0.0 || target_height <= 0.0 {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_has_no_bounds","message":"native scroll target or window root has zero-area bounds","state_generation":self.generation}}));
                        continue;
                    }
                    let scale = f64::from(ctx.pixels_per_point());
                    if !scale.is_finite() || scale <= 0.0 {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_viewport_scale","message":"native viewport scale is not finite and positive","state_generation":self.generation}}));
                        continue;
                    }
                    let x = (target_x - root_x + target_width / 2.0) / scale;
                    let y = (target_y - root_y + target_height / 2.0) / scale;
                    if !x.is_finite() || !y.is_finite() || !(0.0..=1440.0).contains(&x) || !(0.0..=900.0).contains(&y) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"target_outside_viewport","message":"native scroll target center is outside the logical window viewport","state_generation":self.generation}}));
                        continue;
                    }
                    let target_receipt = json!({"role":target["role"],"name":target["name"],"automation_id":target["automation_id"],"bounds":{"x":x,"y":y}});
                    input.focused = true;
                    input.events.extend([
                        Event::PointerMoved(pos2(x as f32, y as f32)),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(delta_x as f32, delta_y as f32),
                            phase: egui::TouchPhase::Move,
                            source: egui::MouseWheelSource::Wheel,
                            modifiers: Default::default(),
                        },
                    ]);
                    self.generation = self.generation.saturating_add(1);
                    self.last_tree = None;
                    batch.responses.push(json!({
                        "id":id,
                        "status":"failed",
                        "error":{
                            "code":"target_effect_unverified",
                            "message":"wheel input was dispatched without activating the native window; inspect state or capture a screenshot before retrying",
                            "input_dispatched":true,
                            "target":target_receipt,
                            "delta_x":delta_x,
                            "delta_y":delta_y,
                            "state_generation":self.generation
                        }
                    }));
                }
                Some("ui.wheel") => {
                    let params = &request["params"];
                    if params["expected_state_generation"].as_u64() != Some(self.generation) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}}));
                        continue;
                    }
                    let (Some(x), Some(y), Some(delta_x), Some(delta_y)) = (
                        params["x"].as_f64(), params["y"].as_f64(),
                        params["delta_x"].as_f64(), params["delta_y"].as_f64(),
                    ) else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"x, y, delta_x and delta_y must be finite numbers"}}));
                        continue;
                    };
                    if !x.is_finite() || !y.is_finite() || !(0.0..=1440.0).contains(&x) || !(0.0..=900.0).contains(&y) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_coordinates","message":"x/y must be finite and within the 1440x900 logical viewport"}}));
                        continue;
                    }
                    if !delta_x.is_finite() || !delta_y.is_finite() || (delta_x == 0.0 && delta_y == 0.0) || delta_x.abs() > 3000.0 || delta_y.abs() > 3000.0 {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"wheel deltas must be finite, non-zero and within 3000 points"}}));
                        continue;
                    }
                    let point = pos2(x as f32, y as f32);
                    input.focused = true;
                    input.events.extend([
                        Event::PointerMoved(point),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(delta_x as f32, delta_y as f32),
                            phase: egui::TouchPhase::Move,
                            source: egui::MouseWheelSource::Wheel,
                            modifiers: Default::default(),
                        },
                    ]);
                    self.generation = self.generation.saturating_add(1);
                    self.last_tree = None;
                    batch.responses.push(json!({
                        "id":id,
                        "status":"failed",
                        "error":{
                            "code":"target_effect_unverified",
                            "message":"wheel input was dispatched at the requested logical viewport point without activating the native window; inspect state or capture a screenshot before retrying",
                            "input_dispatched":true,
                            "x":x,
                            "y":y,
                            "delta_x":delta_x,
                            "delta_y":delta_y,
                            "state_generation":self.generation
                        }
                    }));
                }
                Some("app.action.execute") => {
                    let params = &request["params"];
                    if params["expected_state_generation"].as_u64() != Some(self.generation) {
                        batch.responses.push(json!({
                            "id":id,
                            "status":"failed",
                            "error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}
                        }));
                        continue;
                    }
                    let Some(action_value) = params.get("action").cloned() else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_argument","message":"action is required"}}));
                        continue;
                    };
                    let action: RegisteredUiAction = match serde_json::from_value(action_value) {
                        Ok(action) => action,
                        Err(error) => {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_action","message":error.to_string()}}));
                            continue;
                        }
                    };
                    let (before_title, before_path) = app.automation_route();
                    let before_state = app.automation_state();
                    if let Err(error) = app.execute_registered_action(action.clone()) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"action_rejected","message":error}}));
                        continue;
                    }
                    self.generation = self.generation.saturating_add(1);
                    let (after_title, after_path) = app.automation_route();
                    let mut after_state = app.automation_state();
                    if let Some(object) = after_state.as_object_mut() {
                        object.insert("state_generation".into(), json!(self.generation));
                    }
                    batch.responses.push(json!({
                        "id":id,
                        "status":"succeeded",
                        "result":{
                            "state_generation_before":self.generation.saturating_sub(1),
                            "state_generation_after":self.generation,
                            "action":action,
                            "before":{"title":before_title,"path":before_path},
                            "after":{"title":after_title,"path":after_path},
                            "state_before":before_state,
                            "state_after":after_state,
                            "outcome":"shared_app_handler_succeeded"
                        }
                    }));
                }
                Some("protocol.close") => {
                    self.closed = true;
                    batch.responses.push(json!({"id":id,"status":"succeeded","result":{"state_generation":self.generation}}));
                }
                Some("screenshot.capture") => {
                    batch.screenshots.push((id, self.generation));
                }
                Some("ui.click") => {
                    let params = &request["params"];
                    let expected = params["expected_state_generation"].as_u64();
                    let x = params["x"].as_f64();
                    let y = params["y"].as_f64();
                    if expected != Some(self.generation) {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"stale_generation","message":"expected_state_generation did not match","state_generation":self.generation}}));
                    } else if let (Some(x), Some(y)) = (x, y) {
                        if !x.is_finite() || !y.is_finite() || !(0.0..=1440.0).contains(&x) || !(0.0..=900.0).contains(&y) {
                            batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_coordinates","message":"x/y must be finite and within the 1440x900 logical viewport"}}));
                            continue;
                        }
                        let point = pos2(x as f32, y as f32);
                        input.focused = true;
                        input.events.extend([
                            Event::PointerMoved(point),
                            Event::PointerButton {
                                pos: point,
                                button: PointerButton::Primary,
                                pressed: true,
                                modifiers: Default::default(),
                            },
                        ]);
                        self.pending_click = Some(PendingNativeClick {
                            request_id: id,
                            point,
                            x,
                            y,
                        });
                        ctx.request_repaint();
                        return batch;
                    } else {
                        batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_coordinates","message":"x/y must be finite and within the 1440x900 logical viewport"}}));
                    }
                }
                Some(method) => batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"unsupported_method","message":format!("unsupported native method {method}")}})),
                None => batch.responses.push(json!({"id":id,"status":"failed","error":{"code":"invalid_request","message":"method must be a string"}})),
            }
        }
        batch
    }
}

#[cfg(windows)]
fn encode_native_screenshot(image: &egui::ColorImage) -> Result<(String, String), String> {
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use sha2::{Digest, Sha256};
    use std::io::Cursor;

    let width =
        u32::try_from(image.size[0]).map_err(|error| format!("invalid image width: {error}"))?;
    let height =
        u32::try_from(image.size[1]).map_err(|error| format!("invalid image height: {error}"))?;
    let mut rgba = Vec::with_capacity(image.pixels.len().saturating_mul(4));
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    let image = RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "renderer returned an invalid RGBA pixel buffer".to_owned())?;
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("encode renderer PNG: {error}"))?;
    let png = cursor.into_inner();
    if png.len() > 12 * 1024 * 1024 {
        return Err("renderer PNG exceeded the 12 MiB response limit".into());
    }
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("PNG encoder returned an invalid signature".into());
    }
    let sha256 = format!("{:x}", Sha256::digest(&png));
    Ok((encode_base64(&png), sha256))
}

#[cfg(windows)]
fn native_tree_locator(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let automation_id = params
        .get("automation_id")
        .and_then(serde_json::Value::as_str);
    let role = params.get("role").and_then(serde_json::Value::as_str);
    let name = params.get("name").and_then(serde_json::Value::as_str);
    match (automation_id, role, name) {
        (Some(id), None, None) if !id.is_empty() => Ok(serde_json::json!({"automation_id":id})),
        (None, Some(role), Some(name)) if !role.is_empty() && !name.is_empty() => {
            Ok(serde_json::json!({"role":role,"name":name}))
        }
        (Some(_), _, _) => {
            Err("provide automation_id by itself, or provide both role and name".into())
        }
        (None, Some(_), None) | (None, None, Some(_)) => {
            Err("role and name must be provided together".into())
        }
        _ => Err("provide a nonempty automation_id or an exact role and name".into()),
    }
}

#[cfg(windows)]
fn native_node_matches(node: &serde_json::Value, locator: &serde_json::Value) -> bool {
    if let Some(automation_id) = locator
        .get("automation_id")
        .and_then(serde_json::Value::as_str)
    {
        node.get("automation_id")
            .and_then(serde_json::Value::as_str)
            == Some(automation_id)
    } else {
        node.get("role").and_then(serde_json::Value::as_str)
            == locator.get("role").and_then(serde_json::Value::as_str)
            && node.get("name").and_then(serde_json::Value::as_str)
                == locator.get("name").and_then(serde_json::Value::as_str)
    }
}

#[cfg(windows)]
fn read_native_uia_tree(hwnd: usize) -> Result<serde_json::Value, String> {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    use serde_json::Value;

    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| "SystemRoot is not set; cannot locate Windows PowerShell".to_owned())?;
    let powershell = std::path::PathBuf::from(system_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if !powershell.is_file() {
        return Err(format!(
            "Windows PowerShell was not found at {}",
            powershell.display()
        ));
    }

    let script = format!(
        "$targetHwnd = [Int64]{};\n{}",
        hwnd,
        include_str!("hermes_automation/native_uia_tree.ps1")
    );
    let encoded: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = encode_base64(&encoded);
    let mut command = Command::new(powershell);
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = command
        .spawn()
        .map_err(|error| format!("start read-only UI Automation query: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("UI Automation tree query exceeded its 8-second limit".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("poll UI Automation query process: {error}"));
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("collect UI Automation query output: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            format!("UI Automation query exited with {}", output.status)
        } else {
            format!(
                "UI Automation query exited with {}: {detail}",
                output.status
            )
        });
    }
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("decode UI Automation tree JSON: {error}"))?;
    if value.get("root").and_then(Value::as_u64).is_none()
        || !value.get("nodes").is_some_and(Value::is_array)
    {
        return Err("UI Automation query returned an invalid tree shape".into());
    }
    if output.stdout.len() > 256 * 1024 {
        return Err("UI Automation tree response exceeded 256 KiB".into());
    }
    Ok(value)
}

#[cfg(windows)]
fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(TABLE[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(windows)]
fn native_window_handle(cc: &eframe::CreationContext<'_>) -> Result<*mut std::ffi::c_void, String> {
    use wry::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let window_handle = cc
        .window_handle()
        .map_err(|error| format!("get native window handle: {error}"))?;
    let hwnd = match window_handle.as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut std::ffi::c_void,
        other => return Err(format!("expected Win32 window handle, got {other:?}")),
    };
    Ok(hwnd)
}

#[cfg(windows)]
fn set_window_no_activate(hwnd: *mut std::ffi::c_void) -> Result<(), String> {
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_NOACTIVATE: isize = 0x0800_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetWindowLongPtrW(window: *mut std::ffi::c_void, index: i32) -> isize;
        fn SetWindowLongPtrW(window: *mut std::ffi::c_void, index: i32, value: isize) -> isize;
        fn SetWindowPos(
            window: *mut std::ffi::c_void,
            insert_after: *mut std::ffi::c_void,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }

    // SAFETY: eframe supplies the live HWND for the primary Windows viewport.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if style & WS_EX_NOACTIVATE == 0 {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE);
            if SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            ) == 0
            {
                return Err("could not apply the non-activating window style".into());
            }
            if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_NOACTIVATE == 0 {
                return Err("Windows did not retain WS_EX_NOACTIVATE".into());
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
impl eframe::App for NativeAgentWindow {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if let Some(control) = &mut self.control {
            let batch = control.process_input(_ctx, raw_input, &mut self.app, self.hwnd as usize);
            self.pending_responses.extend(batch.responses);
            self.pending_screenshots.extend(batch.screenshots);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Err(error) = set_window_no_activate(self.hwnd) {
            eprintln!("hermes-automation: {error}");
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let ctx = ui.ctx().clone();
        for (request_id, generation) in std::mem::take(&mut self.pending_screenshots) {
            let response_tx = self
                .control
                .as_ref()
                .expect("screenshot requires native control")
                .responses
                .clone();
            let screenshot_ctx = ctx.clone();
            ctx.request_screenshot(move |image| {
                let encoded = encode_native_screenshot(&image);
                let pixels_per_point = screenshot_ctx.pixels_per_point();
                let response = match encoded {
                    Ok((png_base64, sha256)) => serde_json::json!({
                        "id":request_id,
                        "status":"succeeded",
                        "result":{
                            "state_generation":generation,
                            "mime_type":"image/png",
                            "width":image.size[0],
                            "height":image.size[1],
                            "pixels_per_point":pixels_per_point,
                            "logical_width":image.size[0] as f32 / pixels_per_point,
                            "logical_height":image.size[1] as f32 / pixels_per_point,
                            "sha256":sha256,
                            "png_base64":png_base64,
                            "capture":"native-egui-renderer",
                            "window_foreground_requested":false
                        }
                    }),
                    Err(message) => serde_json::json!({
                        "id":request_id,
                        "status":"failed",
                        "error":{"code":"screenshot_failed","message":message}
                    }),
                };
                let _ = response_tx.send(response);
                screenshot_ctx.request_repaint();
            });
        }
        self.app.draw(ui);
        if let Some(control) = &mut self.control {
            if let Some(action) = control.pending_action.take() {
                let handled = match action.request.action {
                    egui::accesskit::Action::Click => {
                        let node_id = action.request.target_node.0;
                        if node_id == 0 {
                            false
                        } else {
                            // SAFETY: this high-entropy ID came from the current egui AccessKit tree.
                            let widget_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
                            ctx.read_response(widget_id)
                                .is_some_and(|response| response.clicked())
                        }
                    }
                    egui::accesskit::Action::Focus => {
                        let node_id = action.request.target_node.0;
                        if node_id == 0 {
                            false
                        } else {
                            // SAFETY: this high-entropy ID came from the current egui AccessKit tree.
                            let widget_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
                            ctx.memory(|memory| memory.focused()) == Some(widget_id)
                        }
                    }
                    egui::accesskit::Action::SetValue => {
                        let expected_value = match &action.request.data {
                            Some(egui::accesskit::ActionData::Value(value)) => value.as_ref(),
                            _ => "",
                        };
                        let expected_name = action.target["name"].as_str();
                        ctx.output(|output| {
                            output.events.iter().any(|event| match event {
                                egui::output::OutputEvent::ValueChanged(info) => {
                                    info.role == egui::Role::TextInput
                                        && info.hint_text.as_deref() == expected_name
                                        && info.current_text_value.as_deref()
                                            == Some(expected_value)
                                }
                                _ => false,
                            })
                        })
                    }
                    egui::accesskit::Action::ReplaceSelectedText => {
                        let inserted_value = match &action.request.data {
                            Some(egui::accesskit::ActionData::Value(value)) => value.as_ref(),
                            _ => "",
                        };
                        let expected_name = action.target["name"].as_str();
                        ctx.output(|output| {
                            output.events.iter().any(|event| match event {
                                egui::output::OutputEvent::ValueChanged(info) => {
                                    info.role == egui::Role::TextInput
                                        && info.hint_text.as_deref() == expected_name
                                        && info.current_text_value.as_deref()
                                            != info.prev_text_value.as_deref()
                                        && (inserted_value.is_empty()
                                            || info.current_text_value.as_deref().is_some_and(
                                                |value| value.contains(inserted_value),
                                            ))
                                }
                                _ => false,
                            })
                        })
                    }
                    _ => false,
                };
                let generation_before = control.generation;
                control.generation = control.generation.saturating_add(1);
                control.last_tree = None;
                let (after_title, after_path) = self.app.automation_route();
                let mut after_state = self.app.automation_state();
                if let Some(object) = after_state.as_object_mut() {
                    object.insert(
                        "state_generation".into(),
                        serde_json::json!(control.generation),
                    );
                }
                let response = if handled {
                    serde_json::json!({
                        "id":action.request_id,
                        "status":"succeeded",
                        "result":{
                            "state_generation_before":generation_before,
                            "state_generation_after":control.generation,
                            "action":action.action_name,
                            "target":action.target,
                            "before":{"route":action.before_route,"state":action.before_state},
                            "after":{"route":{"title":after_title,"path":after_path},"state":after_state},
                            "outcome":"target_accesskit_action_handled",
                            "window_foreground_requested":false
                        }
                    })
                } else {
                    serde_json::json!({
                        "id":action.request_id,
                        "status":"failed",
                        "error":{
                            "code":"target_effect_unverified",
                            "message":"the semantic action was dispatched, but the target widget did not confirm handling it",
                            "input_dispatched":true,
                            "state_generation":control.generation,
                            "state_before":action.before_state,
                            "state_after":after_state,
                            "window_foreground_requested":false
                        }
                    })
                };
                let _ = control.responses.send(response);
            }
            let pointer_state = ctx.input(|input| {
                let position = input
                    .pointer
                    .latest_pos()
                    .map(|point| serde_json::json!({"x":point.x,"y":point.y}));
                serde_json::json!({
                    "primary_clicked":input.pointer.primary_clicked(),
                    "primary_pressed":input.pointer.primary_down(),
                    "position":position
                })
            });
            for mut response in std::mem::take(&mut self.pending_responses) {
                if response
                    .pointer("/result/input")
                    .and_then(serde_json::Value::as_str)
                    .is_some()
                {
                    response["result"]["egui_pointer"] = pointer_state.clone();
                }
                let _ = control.responses.send(response);
            }
            if control.closed {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
