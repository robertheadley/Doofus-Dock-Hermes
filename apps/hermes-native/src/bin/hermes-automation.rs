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
    let mut plugins_catalog_fixture = false;
    let mut plugin_install_review_fixture = false;
    let mut plugin_update_scan_review_fixture = false;
    let mut plugin_auto_update_fixture = false;
    let mut plugin_update_all_report_fixture = false;
    let mut connectors_fixture = false;
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
            "--plugins-catalog-fixture" => plugins_catalog_fixture = true,
            "--plugin-install-review-fixture" => plugin_install_review_fixture = true,
            "--plugin-update-scan-review-fixture" => plugin_update_scan_review_fixture = true,
            "--plugin-auto-update-fixture" => plugin_auto_update_fixture = true,
            "--plugin-update-all-report-fixture" => plugin_update_all_report_fixture = true,
            "--connectors-fixture" => connectors_fixture = true,
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
        plugins_catalog_fixture,
        plugin_install_review_fixture,
        plugin_update_scan_review_fixture,
        plugin_auto_update_fixture,
        plugin_update_all_report_fixture,
        connectors_fixture,
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
Usage: hermes-automation [--viewport WIDTHxHEIGHT] [--markdown-fixture | --first-run-setup-fixture | --long-session-fixture | --transcript-find-fixture | --transcript-timestamps-fixture | --session-pane-fixture | --recent-session-switcher-fixture | --plugins-catalog-fixture | --plugin-install-review-fixture | --plugin-update-scan-review-fixture | --plugin-auto-update-fixture | --plugin-update-all-report-fixture | --connectors-fixture | --files-preview-fixture | --image-preview-fixture | --svg-preview-fixture | --pdf-preview-fixture | --pdf-scroll-fixture | --html-preview-fixture] [--inspect] [--click ACCESSIBLE_NAME] [--scroll-up ACCESSIBLE_NAME | --scroll-down ACCESSIBLE_NAME] [--type ACCESSIBLE_NAME TEXT | --type-role ROLE ACCESSIBLE_NAME TEXT] [--screenshot PATH]...
Actions run in order against an isolated offline Hermes Native UI. No native window is opened and no Hermes gateway is started.
Accessible actions use egui's AccessKit UI tree and normal widget input handling.");
}
