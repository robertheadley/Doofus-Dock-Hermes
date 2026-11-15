use egui::Vec2;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use hermes_native::app::HermesNativeApp;
use serde_json::{Value, json};
use std::{env, path::PathBuf, process};

#[path = "hermes_automation/automation_protocol.rs"]
mod automation_protocol;

#[derive(Debug)]
enum Action {
    Inspect,
    Click(String),
    Type(String, String),
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
            "--click" => actions.push(Action::Click(take(&args, &mut index, "--click")?)),
            "--type" => {
                let label = take(&args, &mut index, "--type")?;
                let text = take(&args, &mut index, "--type text")?;
                actions.push(Action::Type(label, text));
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
    let profile = TemporaryProfile::create()?;
    let profile_path = profile.0.clone();
    let app = std::rc::Rc::new(std::cell::RefCell::new(
        HermesNativeApp::new_for_automation(profile_path.clone()),
    ));
    if markdown_fixture {
        app.borrow_mut().load_markdown_fixture_for_automation();
    }
    let app_for_frame = app.clone();
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, height))
        .build_ui(move |ui| app_for_frame.borrow_mut().draw(ui));
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
            Action::Type(label, text) => {
                let node = harness
                    .query_by_label(&label)
                    .ok_or_else(|| format!("no accessible control named {label:?}"))?;
                let role = format!("{:?}", node.accesskit_node().role());
                node.type_text(&text);
                harness.run();
                println!(
                    "{}",
                    json!({"ok":true,"action":"type","label":label,"role":role,"characters":text.chars().count()})
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
Usage: hermes-automation [--viewport WIDTHxHEIGHT] [--markdown-fixture] [--inspect] [--click ACCESSIBLE_NAME] [--type ACCESSIBLE_NAME TEXT] [--screenshot PATH]...
Actions run in order against an isolated offline Hermes Native UI. No native window is opened and no Hermes gateway is started.
Accessible actions use egui's AccessKit UI tree and normal widget input handling.");
}
