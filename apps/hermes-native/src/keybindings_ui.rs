//! Headless-testable settings panel for supported Hermes Native shortcuts.
//!
//! The panel deliberately renders only `keybindings::NATIVE_ACTIONS`. It does
//! not create controls for upstream actions without native handlers.

use crate::keybindings::{
    ActionDefinition, KeyPlatform, KeybindingStore, NATIVE_ACTIONS, normalize_combo,
};
use egui::{Event, Key, Modifiers, Ui};
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct KeybindingsPanel {
    query: String,
    collapsed: BTreeSet<String>,
    capture_action: Option<String>,
    error: Option<String>,
}

impl KeybindingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_capturing(&self) -> bool {
        self.capture_action.is_some()
    }

    pub fn capture_action(&self) -> Option<&str> {
        self.capture_action.as_deref()
    }

    pub fn cancel_capture(&mut self) {
        self.capture_action = None;
    }

    pub fn show(&mut self, ui: &mut Ui, store: &mut KeybindingStore, platform: KeyPlatform) {
        self.capture_next_combo(ui, store, platform);

        ui.heading("Keyboard shortcuts");
        ui.label("Search and customize shortcuts for supported native actions.");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search shortcuts")
                    .desired_width((ui.available_width() - 140.0).max(120.0)),
            )
            .accessible_name("Search keybindings");
            if ui
                .button("Reset all")
                .accessible_name("Reset all keybindings")
                .clicked()
            {
                self.cancel_capture();
                self.set_result(store.reset_all());
            }
        });

        if let Some(action_id) = self.capture_action.as_deref() {
            ui.label(format!(
                "Press a supported key combo for {}…",
                action_name(action_id)
            ))
            .accessible_name(format!("Recording binding for {action_id}"));
        }
        if let Some(error) = self.error.as_deref() {
            ui.colored_label(egui::Color32::LIGHT_RED, error)
                .accessible_name("Keybinding error");
        }

        let query = self.query.trim().to_ascii_lowercase();
        let mut categories: Vec<&'static str> = Vec::new();
        for action in NATIVE_ACTIONS {
            if !categories.contains(&action.category) {
                categories.push(action.category);
            }
        }

        for category in categories {
            let actions: Vec<ActionDefinition> = NATIVE_ACTIONS
                .iter()
                .copied()
                .filter(|action| action.category == category)
                .filter(|action| {
                    query.is_empty()
                        || action.id.to_ascii_lowercase().contains(&query)
                        || action_name(action.id).to_ascii_lowercase().contains(&query)
                        || category.to_ascii_lowercase().contains(&query)
                })
                .collect();
            if actions.is_empty() {
                continue;
            }

            let searching = !query.is_empty();
            let open = searching || !self.collapsed.contains(category);
            let category_label = category_name(category);
            let toggle = ui
                .button(format!(
                    "{} {}",
                    if open { "v" } else { ">" },
                    category_label
                ))
                .accessible_name(format!("Toggle category {category}"));
            if toggle.clicked() && !searching {
                if open {
                    self.collapsed.insert(category.to_owned());
                } else {
                    self.collapsed.remove(category);
                }
            }
            if open {
                for action in actions {
                    self.action_row(ui, store, platform, action);
                }
            }
        }
    }

    fn action_row(
        &mut self,
        ui: &mut Ui,
        store: &mut KeybindingStore,
        platform: KeyPlatform,
        action: ActionDefinition,
    ) {
        let combos = match store.bindings_for(action.id) {
            Ok(combos) => combos,
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error)
                    .accessible_name(format!("Binding error for {}", action.id));
                return;
            }
        };
        let conflicts = match store.conflicts_for(action.id) {
            Ok(conflicts) => conflicts,
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error)
                    .accessible_name(format!("Conflict check error for {}", action.id));
                Vec::new()
            }
        };
        let armed = self.capture_action.as_deref() == Some(action.id);
        let mut record = false;
        let mut clear = false;
        let mut reset = false;

        ui.horizontal(|ui| {
            ui.add(
                egui::Button::new(action_name(action.id))
                    .frame(false)
                    .sense(egui::Sense::hover()),
            )
            .accessible_name(format!("Action {}", action.id));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                reset = ui
                    .add_enabled(
                        store.overrides().contains_key(action.id),
                        egui::Button::new("Reset"),
                    )
                    .accessible_name(format!("Reset binding for {}", action.id))
                    .clicked();
                clear = ui
                    .add_enabled(!combos.is_empty(), egui::Button::new("Clear"))
                    .accessible_name(format!("Clear binding for {}", action.id))
                    .clicked();
                record = ui
                    .button(if armed { "Recording…" } else { "Record" })
                    .accessible_name(format!("Record binding for {}", action.id))
                    .clicked();
                let display = if armed {
                    "Press key…".to_owned()
                } else if combos.is_empty() {
                    "Unbound".to_owned()
                } else {
                    combos.join("  ·  ")
                };
                ui.label(display)
                    .accessible_name(format!("Bindings for {}", action.id));
            });
        });
        if !conflicts.is_empty() {
            let names = conflicts
                .iter()
                .map(|id| action_name(id))
                .collect::<Vec<_>>()
                .join(", ");
            ui.add(
                egui::Button::new(
                    egui::RichText::new(format!("Conflict with {names}"))
                        .color(egui::Color32::YELLOW),
                )
                .frame(false)
                .sense(egui::Sense::hover()),
            )
            .accessible_name(format!("Conflict: {}", conflicts.join(", ")));
        }

        if record {
            self.capture_action = Some(action.id.to_owned());
            self.error = None;
        } else if clear {
            self.cancel_capture();
            self.set_result(store.clear_binding(action.id));
        } else if reset {
            self.cancel_capture();
            self.set_result(store.reset_binding(action.id));
        }

        // Keep the current platform argument live in this component's API: it
        // controls how captured physical Control is represented in the combo.
        let _ = platform;
    }

    fn capture_next_combo(
        &mut self,
        ui: &mut Ui,
        store: &mut KeybindingStore,
        platform: KeyPlatform,
    ) {
        let Some(action_id) = self.capture_action.clone() else {
            return;
        };
        let cancel = ui.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        repeat: false,
                        ..
                    }
                )
            })
        });
        if cancel {
            self.cancel_capture();
            self.error = None;
            return;
        }
        let captured = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } => key_event_to_combo(*key, *modifiers, platform),
                _ => None,
            })
        });
        let Some(combo) = captured else {
            return;
        };
        match normalize_combo(&combo, platform) {
            Ok(combo) => {
                self.capture_action = None;
                self.error = None;
                self.set_result(store.set_binding(&action_id, &[combo.as_str()]));
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn set_result(&mut self, result: Result<(), String>) {
        self.error = result.err();
    }
}

pub(crate) fn key_event_to_combo(
    key: Key,
    modifiers: Modifiers,
    platform: KeyPlatform,
) -> Option<String> {
    let base = key_token(key)?;
    let mut parts = Vec::new();
    match platform {
        KeyPlatform::Mac => {
            if modifiers.mac_cmd || modifiers.command {
                parts.push("mod");
            }
            if modifiers.ctrl {
                parts.push("ctrl");
            }
        }
        KeyPlatform::Other => {
            if modifiers.ctrl || modifiers.command {
                parts.push("mod");
            }
        }
    }
    if modifiers.alt {
        parts.push("alt");
    }
    if modifiers.shift {
        parts.push("shift");
    }
    parts.push(base);
    Some(parts.join("+"))
}

fn key_token(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::A => "a",
        Key::B => "b",
        Key::C => "c",
        Key::D => "d",
        Key::E => "e",
        Key::F => "f",
        Key::G => "g",
        Key::H => "h",
        Key::I => "i",
        Key::J => "j",
        Key::K => "k",
        Key::L => "l",
        Key::M => "m",
        Key::N => "n",
        Key::O => "o",
        Key::P => "p",
        Key::Q => "q",
        Key::R => "r",
        Key::S => "s",
        Key::T => "t",
        Key::U => "u",
        Key::V => "v",
        Key::W => "w",
        Key::X => "x",
        Key::Y => "y",
        Key::Z => "z",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash => "/",
        Key::Backslash => "\\",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::Backtick => "`",
        Key::Minus => "-",
        Key::Equals => "=",
        Key::Space => "space",
        Key::Enter => "enter",
        Key::Escape => "escape",
        Key::Backspace => "backspace",
        Key::Delete => "delete",
        Key::Tab => "tab",
        Key::PageUp => "pageup",
        Key::PageDown => "pagedown",
        Key::ArrowUp => "up",
        Key::ArrowDown => "down",
        Key::ArrowLeft => "left",
        Key::ArrowRight => "right",
        Key::F1 => "f1",
        Key::F2 => "f2",
        Key::F3 => "f3",
        Key::F4 => "f4",
        Key::F5 => "f5",
        Key::F6 => "f6",
        Key::F7 => "f7",
        Key::F8 => "f8",
        Key::F9 => "f9",
        Key::F10 => "f10",
        Key::F11 => "f11",
        Key::F12 => "f12",
        _ => return None,
    })
}

fn category_name(category: &str) -> &'static str {
    match category {
        "navigation" => "Navigation",
        "session" => "Session",
        "view" => "View",
        _ => "Other",
    }
}

fn action_name(action_id: &str) -> String {
    match action_id {
        "nav.commandPalette" => "Open command palette".to_owned(),
        "view.findInPage" => "Find in transcript".to_owned(),
        "view.findNext" => "Find next".to_owned(),
        "view.findPrevious" => "Find previous".to_owned(),
        "session.next" => "Next session tab".to_owned(),
        "session.prev" => "Previous session tab".to_owned(),
        "view.closeTab" => "Close active tab".to_owned(),
        "view.reopenTab" => "Reopen closed tab".to_owned(),
        "view.toggleSidebar" => "Toggle sessions sidebar".to_owned(),
        "view.toggleRightSidebar" => "Toggle files sidebar".to_owned(),
        _ => action_id.to_owned(),
    }
}

#[cfg(all(test, feature = "agent-automation"))]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use std::{
        cell::RefCell,
        fs,
        path::PathBuf,
        rc::Rc,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestStore {
        directory: PathBuf,
        store: Rc<RefCell<KeybindingStore>>,
    }

    impl TestStore {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let directory = std::env::temp_dir().join(format!(
                "hermes-native-keybindings-ui-{}-{stamp}",
                std::process::id()
            ));
            fs::create_dir_all(&directory).unwrap();
            let store = Rc::new(RefCell::new(
                KeybindingStore::load(directory.join("keybindings.json"), KeyPlatform::Other)
                    .unwrap(),
            ));
            Self { directory, store }
        }
    }

    impl Drop for TestStore {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }

    fn harness(
        panel: Rc<RefCell<KeybindingsPanel>>,
        store: Rc<RefCell<KeybindingStore>>,
    ) -> egui_kittest::Harness<'static> {
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(720.0, 900.0))
            .build_ui(move |ui| {
                panel
                    .borrow_mut()
                    .show(ui, &mut store.borrow_mut(), KeyPlatform::Other)
            })
    }

    #[test]
    fn search_filters_actions_and_category_toggle_hides_rows() {
        let state = TestStore::new();
        let panel = Rc::new(RefCell::new(KeybindingsPanel::new()));
        let mut harness = harness(panel, state.store.clone());
        assert!(
            harness
                .query_by_label("Record binding for nav.commandPalette")
                .is_some()
        );
        harness.get_by_label("Toggle category navigation").click();
        harness.run();
        assert!(
            harness
                .query_by_label("Record binding for nav.commandPalette")
                .is_none()
        );

        harness
            .get_by_label("Search keybindings")
            .type_text("find previous");
        harness.run();
        assert!(
            harness
                .query_by_label("Record binding for view.findPrevious")
                .is_some()
        );
        assert!(
            harness
                .query_by_label("Record binding for nav.commandPalette")
                .is_none()
        );
    }

    #[test]
    fn armed_capture_records_a_supported_combo_and_clear_reset_work() {
        let state = TestStore::new();
        let panel = Rc::new(RefCell::new(KeybindingsPanel::new()));
        let mut harness = harness(panel.clone(), state.store.clone());
        harness
            .get_by_label("Record binding for nav.commandPalette")
            .click();
        harness.run();
        assert!(panel.borrow().is_capturing());
        harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::X);
        harness.run();
        assert!(!panel.borrow().is_capturing());
        assert_eq!(
            state
                .store
                .borrow()
                .bindings_for("nav.commandPalette")
                .unwrap(),
            ["mod+x"]
        );

        harness
            .get_by_label("Clear binding for nav.commandPalette")
            .click();
        harness.run();
        assert!(
            state
                .store
                .borrow()
                .bindings_for("nav.commandPalette")
                .unwrap()
                .is_empty()
        );
        harness
            .get_by_label("Reset binding for nav.commandPalette")
            .click();
        harness.run();
        assert_eq!(
            state
                .store
                .borrow()
                .bindings_for("nav.commandPalette")
                .unwrap(),
            ["mod+k", "mod+p"]
        );
    }

    #[test]
    fn escape_cancels_armed_capture_without_changing_the_binding() {
        let state = TestStore::new();
        let panel = Rc::new(RefCell::new(KeybindingsPanel::new()));
        let mut harness = harness(panel.clone(), state.store.clone());
        harness
            .get_by_label("Record binding for nav.commandPalette")
            .click();
        harness.run();
        assert!(panel.borrow().is_capturing());
        harness.key_press(egui::Key::Escape);
        harness.run();
        assert!(!panel.borrow().is_capturing());
        assert_eq!(
            state
                .store
                .borrow()
                .bindings_for("nav.commandPalette")
                .expect("defaults still resolve"),
            vec!["mod+k", "mod+p"]
        );
        drop(harness);
    }

    #[test]
    fn conflict_warning_is_visible_and_reset_all_clears_overrides() {
        let state = TestStore::new();
        state
            .store
            .borrow_mut()
            .set_binding("view.findInPage", &["ctrl+k"])
            .unwrap();
        let panel = Rc::new(RefCell::new(KeybindingsPanel::new()));
        let mut harness = harness(panel, state.store.clone());
        assert!(
            harness
                .query_by_label("Conflict: nav.commandPalette")
                .is_some()
        );
        harness.get_by_label("Reset all keybindings").click();
        harness.run();
        assert!(state.store.borrow().overrides().is_empty());
        assert!(
            harness
                .query_by_label("Conflict: nav.commandPalette")
                .is_none()
        );
    }

    #[test]
    fn unmodified_text_key_is_ignored_until_record_is_armed() {
        let state = TestStore::new();
        let panel = Rc::new(RefCell::new(KeybindingsPanel::new()));
        let mut harness = harness(panel, state.store.clone());
        harness.key_press(egui::Key::X);
        harness.run();
        assert!(state.store.borrow().overrides().is_empty());

        harness
            .get_by_label("Record binding for nav.commandPalette")
            .click();
        harness.run();
        harness.key_press(egui::Key::X);
        harness.run();
        assert_eq!(
            state
                .store
                .borrow()
                .bindings_for("nav.commandPalette")
                .unwrap(),
            ["x"]
        );
    }
}
