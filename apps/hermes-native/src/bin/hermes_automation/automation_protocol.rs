use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use hermes_native::app::{HermesNativeApp, RegisteredUiAction, registered_ui_action_catalog};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet, VecDeque},
    env,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    process,
    rc::Rc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PROTOCOL_VERSION: u64 = 1;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_TEXT_CHARS: usize = 16 * 1024;
const EVENT_LIMIT: usize = 128;
const REQUEST_CACHE_LIMIT: usize = 128;
const REQUEST_ID_LIMIT: usize = 10_000;

type UiHarness<'a> = Harness<'a>;
type UiApp = Rc<RefCell<HermesNativeApp>>;

#[derive(Default)]
struct ProtocolState {
    generation: u64,
    next_event_id: u64,
    events: VecDeque<Value>,
    request_order: VecDeque<String>,
    requests: BTreeMap<String, CachedRequest>,
    seen_request_ids: HashSet<String>,
}

struct CachedRequest {
    canonical_request: String,
    response: Value,
}

pub fn run_stdio() -> Result<(), String> {
    let profile = super::TemporaryProfile::create()?;
    let profile_path = profile.0.clone();
    let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
        profile_path,
    )));
    let frame_app = app.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
    let mut protocol = ProtocolState::default();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    let ready = json!({
        "version": PROTOCOL_VERSION,
        "type": "ready",
        "protocol": "hermes-native-agent-ui/1",
        "transport": "inherited-stdio-capability",
        "process_id": process::id(),
        "gateway": "disabled",
        "state_generation": protocol.generation,
        "state": snapshot(&app, protocol.generation),
    });
    write_message(&mut writer, &ready)?;

    loop {
        let line = match read_bounded_line(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                write_message(
                    &mut writer,
                    &failure(Value::Null, "request_too_large", error.to_string()),
                )?;
                break;
            }
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let request = match serde_json::from_slice::<Value>(&line) {
            Ok(value) => value,
            Err(error) => {
                write_message(
                    &mut writer,
                    &failure(Value::Null, "invalid_json", error.to_string()),
                )?;
                continue;
            }
        };
        let is_close = request
            .get("method")
            .and_then(Value::as_str)
            .is_some_and(|method| method == "protocol.close");
        let response = protocol.handle(request, &mut harness, &app);
        write_message(&mut writer, &response)?;
        if is_close {
            break;
        }
    }
    Ok(())
}

impl ProtocolState {
    fn handle(&mut self, request: Value, harness: &mut UiHarness<'_>, app: &UiApp) -> Value {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let Some(id_text) = id.as_str() else {
            return failure(id, "invalid_request", "request id must be a string".into());
        };
        if id_text.is_empty() || id_text.len() > 128 {
            return failure(
                id,
                "invalid_request",
                "request id must contain 1 to 128 bytes".into(),
            );
        }
        let canonical_request = match serde_json::to_string(&request) {
            Ok(value) => value,
            Err(error) => return failure(id, "invalid_request", error.to_string()),
        };
        if let Some(cached) = self.requests.get(id_text) {
            if cached.canonical_request == canonical_request {
                return cached.response.clone();
            }
            return failure(
                id,
                "request_id_conflict",
                "request id was already used with a different payload".into(),
            );
        }
        if self.seen_request_ids.contains(id_text) {
            return failure(
                id,
                "request_replay_expired",
                "request id was already processed and its cached response expired".into(),
            );
        }
        if self.seen_request_ids.len() >= REQUEST_ID_LIMIT {
            return failure(
                id,
                "request_limit_reached",
                "protocol instance reached its 10000 unique request limit".into(),
            );
        }
        self.seen_request_ids.insert(id_text.to_owned());

        let response = self.dispatch(&id, &request, harness, app);
        self.cache_request(id_text.to_owned(), canonical_request, response.clone());
        response
    }

    fn dispatch(
        &mut self,
        id: &Value,
        request: &Value,
        harness: &mut UiHarness<'_>,
        app: &UiApp,
    ) -> Value {
        if request.get("version").and_then(Value::as_u64) != Some(PROTOCOL_VERSION) {
            return failure(
                id.clone(),
                "unsupported_version",
                "protocol version must be 1".into(),
            );
        }
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            return failure(
                id.clone(),
                "invalid_request",
                "method must be a string".into(),
            );
        };
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        if !params.is_object() {
            return failure(
                id.clone(),
                "invalid_request",
                "params must be an object".into(),
            );
        }
        let request_id = id.as_str().unwrap_or_default();
        match method {
            "health.get" => success(
                id.clone(),
                json!({
                    "status": "ready",
                    "protocol": "hermes-native-agent-ui/1",
                    "process_id": process::id(),
                    "state_generation": self.generation,
                    "gateway": "disabled",
                    "build_version": env!("CARGO_PKG_VERSION"),
                }),
            ),
            "capabilities.list" => success(id.clone(), capabilities()),
            "app.actions.list" => success(id.clone(), registered_ui_action_catalog()),
            "state.get" => success(id.clone(), snapshot(app, self.generation)),
            "ui.tree" => success(id.clone(), describe_node(harness.root())),
            "app.action.execute" => {
                let Some(action_value) = params.get("action").cloned() else {
                    return failure(id.clone(), "invalid_argument", "action is required".into());
                };
                let action: RegisteredUiAction = match serde_json::from_value(action_value) {
                    Ok(action) => action,
                    Err(error) => {
                        return failure(
                            id.clone(),
                            "invalid_action",
                            format!("action schema validation failed: {error}"),
                        );
                    }
                };
                if let Err(error) = self.check_generation(&params) {
                    return failure(id.clone(), error.0, error.1);
                }
                let before_generation = self.generation;
                let before_state = snapshot(app, before_generation);
                if let Err(error) = app.borrow_mut().execute_registered_action(action.clone()) {
                    self.record_event(id, method, "failed", before_generation, before_generation);
                    return failure(id.clone(), "action_rejected", error);
                }
                harness.run();
                self.generation = self.generation.saturating_add(1);
                let after_state = snapshot(app, self.generation);
                self.record_event(id, method, "succeeded", before_generation, self.generation);
                success(
                    id.clone(),
                    json!({
                        "request_id": request_id,
                        "correlation_id": request_id,
                        "trace_id": format!("ui-{request_id}"),
                        "actor": "stdio-peer",
                        "permission_class": "ui-interaction",
                        "command": "app.action.execute",
                        "action": action,
                        "state_generation_before": before_generation,
                        "state_generation_after": self.generation,
                        "before": before_state,
                        "after": after_state,
                        "outcome": "succeeded"
                    }),
                )
            }
            "events.list" => success(
                id.clone(),
                json!({"events": self.events.iter().cloned().collect::<Vec<_>>() }),
            ),
            "diagnostics.bundle" => success(
                id.clone(),
                json!({
                    "protocol": "hermes-native-agent-ui/1",
                    "process_id": process::id(),
                    "state": snapshot(app, self.generation),
                    "events": self.events.iter().cloned().collect::<Vec<_>>(),
                    "event_count": self.events.len(),
                    "request_cache_entries": self.requests.len(),
                    "unique_request_ids": self.seen_request_ids.len(),
                    "gateway": "disabled",
                    "native_window": false
                }),
            ),
            "actions.click" | "actions.right_click" => {
                let (role, name) = match resolve_locator(harness, &params, "") {
                    Ok(locator) => locator,
                    Err((code, message)) => return failure(id.clone(), code, message),
                };
                if let Err(error) = self.check_generation(&params) {
                    return failure(id.clone(), error.0, error.1);
                }
                let before_generation = self.generation;
                let before_state = snapshot(app, before_generation);
                let matches = matching_nodes(harness, &role, &name);
                if matches.len() != 1 {
                    let (code, message) = match matches.len() {
                        0 => ("target_not_found", format!("no {role} named {name:?}")),
                        _ => (
                            "ambiguous_target",
                            format!("{} {role} controls are named {name:?}", matches.len()),
                        ),
                    };
                    self.record_event(id, method, "failed", before_generation, before_generation);
                    return failure(id.clone(), code, message);
                }
                if matches[0].accesskit_node().is_disabled() {
                    return failure(
                        id.clone(),
                        "target_disabled",
                        format!("{role} named {name:?} is disabled"),
                    );
                }
                if method == "actions.click" {
                    matches[0].click();
                } else {
                    let position = matches[0].rect().center();
                    harness.event(egui::Event::PointerMoved(position));
                    harness.run();
                    harness.event(egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Secondary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    });
                    harness.run();
                    harness.event(egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Secondary,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    });
                    harness.run();
                }
                harness.run();
                self.generation = self.generation.saturating_add(1);
                let after_state = snapshot(app, self.generation);
                self.record_event(id, method, "succeeded", before_generation, self.generation);
                success(
                    id.clone(),
                    json!({
                        "request_id": request_id,
                        "correlation_id": request_id,
                        "trace_id": format!("ui-{request_id}"),
                        "actor": "stdio-peer",
                        "permission_class": "ui-interaction",
                        "command": if method == "actions.click" { "ui.click" } else { "ui.right_click" },
                        "target": {"role": role, "name": name},
                        "state_generation_before": before_generation,
                        "state_generation_after": self.generation,
                        "before": before_state,
                        "after": after_state,
                        "outcome": "succeeded",
                    }),
                )
            }
            "actions.type" => {
                let (role, name) = match resolve_locator(harness, &params, "") {
                    Ok(locator) => locator,
                    Err((code, message)) => return failure(id.clone(), code, message),
                };
                let Some(text) = string_param(&params, "text") else {
                    return failure(id.clone(), "invalid_argument", "text is required".into());
                };
                if text.chars().count() > MAX_TEXT_CHARS {
                    return failure(
                        id.clone(),
                        "payload_too_large",
                        "text exceeds 16384 characters".into(),
                    );
                }
                if let Err(error) = self.check_generation(&params) {
                    return failure(id.clone(), error.0, error.1);
                }
                let before_generation = self.generation;
                let before_state = snapshot(app, before_generation);
                let matches = matching_nodes(harness, &role, &name);
                if matches.len() != 1 {
                    let (code, message) = match matches.len() {
                        0 => ("target_not_found", format!("no {role} named {name:?}")),
                        _ => (
                            "ambiguous_target",
                            format!("{} {role} controls are named {name:?}", matches.len()),
                        ),
                    };
                    self.record_event(id, method, "failed", before_generation, before_generation);
                    return failure(id.clone(), code, message);
                }
                if matches[0].accesskit_node().is_disabled() {
                    return failure(
                        id.clone(),
                        "target_disabled",
                        format!("{role} named {name:?} is disabled"),
                    );
                }
                matches[0].type_text(text);
                harness.run();
                self.generation = self.generation.saturating_add(1);
                let after_state = snapshot(app, self.generation);
                self.record_event(id, method, "succeeded", before_generation, self.generation);
                success(
                    id.clone(),
                    json!({
                        "request_id": request_id,
                        "correlation_id": request_id,
                        "trace_id": format!("ui-{request_id}"),
                        "actor": "stdio-peer",
                        "permission_class": "ui-interaction",
                        "command": "ui.type",
                        "target": {"role": role, "name": name},
                        "characters": text.chars().count(),
                        "state_generation_before": before_generation,
                        "state_generation_after": self.generation,
                        "before": before_state,
                        "after": after_state,
                        "outcome": "succeeded",
                    }),
                )
            }
            "actions.drag" => {
                let (role, name) = match resolve_locator(harness, &params, "") {
                    Ok(locator) => locator,
                    Err((code, message)) => return failure(id.clone(), code, message),
                };
                let to_role = string_param(&params, "to_role");
                let to_name = string_param(&params, "to_name");
                let to_test_id = string_param(&params, "to_test_id");
                let has_coordinates = params.get("to_x").is_some() || params.get("to_y").is_some();
                let locator_destination = match (to_role, to_name, to_test_id) {
                    (None, None, None) if has_coordinates => None,
                    (_, _, _) if !has_coordinates => match resolve_locator(harness, &params, "to_")
                    {
                        Ok(locator) => Some(locator),
                        Err((code, message)) => return failure(id.clone(), code, message),
                    },
                    _ => {
                        return failure(
                            id.clone(),
                            "invalid_argument",
                            "provide one destination locator or both to_x/to_y".into(),
                        );
                    }
                };
                let steps = params.get("steps").and_then(Value::as_u64).unwrap_or(8);
                if !(1..=32).contains(&steps) {
                    return failure(
                        id.clone(),
                        "invalid_argument",
                        "steps must be between 1 and 32".into(),
                    );
                }
                if let Err(error) = self.check_generation(&params) {
                    return failure(id.clone(), error.0, error.1);
                }
                let before_generation = self.generation;
                let before_state = snapshot(app, before_generation);
                let matches = matching_nodes(harness, &role, &name);
                if matches.len() != 1 {
                    let (code, message) = match matches.len() {
                        0 => ("target_not_found", format!("no {role} named {name:?}")),
                        _ => (
                            "ambiguous_target",
                            format!("{} {role} controls are named {name:?}", matches.len()),
                        ),
                    };
                    self.record_event(id, method, "failed", before_generation, before_generation);
                    return failure(id.clone(), code, message);
                }
                if matches[0].accesskit_node().is_disabled() {
                    return failure(
                        id.clone(),
                        "target_disabled",
                        format!("{role} named {name:?} is disabled"),
                    );
                }
                let start = matches[0].rect().center();
                let (end, destination_target) = if let Some((target_role, target_name)) =
                    locator_destination
                {
                    let targets = matching_nodes(harness, &target_role, &target_name);
                    if targets.len() != 1 {
                        let (code, message) = if targets.is_empty() {
                            (
                                "target_not_found",
                                format!("no destination {target_role} named {target_name:?}"),
                            )
                        } else {
                            (
                                "ambiguous_target",
                                format!(
                                    "{} destination {target_role} controls are named {target_name:?}",
                                    targets.len()
                                ),
                            )
                        };
                        return failure(id.clone(), code, message);
                    }
                    if targets[0].accesskit_node().is_disabled() {
                        return failure(
                            id.clone(),
                            "target_disabled",
                            format!("destination {target_role} named {target_name:?} is disabled"),
                        );
                    }
                    (
                        targets[0].rect().center(),
                        Some(json!({"role":target_role,"name":target_name})),
                    )
                } else {
                    let Some(x) = params.get("to_x").and_then(Value::as_f64) else {
                        return failure(
                            id.clone(),
                            "invalid_argument",
                            "to_x must be a finite number".into(),
                        );
                    };
                    let Some(y) = params.get("to_y").and_then(Value::as_f64) else {
                        return failure(
                            id.clone(),
                            "invalid_argument",
                            "to_y must be a finite number".into(),
                        );
                    };
                    if !x.is_finite()
                        || !y.is_finite()
                        || x.abs() > f32::MAX as f64
                        || y.abs() > f32::MAX as f64
                    {
                        return failure(
                            id.clone(),
                            "invalid_argument",
                            "drag destination is outside numeric bounds".into(),
                        );
                    }
                    (egui::pos2(x as f32, y as f32), None)
                };
                let screen = harness.ctx.viewport_rect();
                if !screen.contains(start) || !screen.contains(end) {
                    return failure(
                        id.clone(),
                        "invalid_argument",
                        "drag source and destination must be inside the visible UI bounds".into(),
                    );
                }
                harness.event(egui::Event::PointerMoved(start));
                harness.run();
                harness.event(egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                });
                harness.run();
                for step in 1..=steps {
                    let position = start.lerp(end, step as f32 / steps as f32);
                    harness.event(egui::Event::PointerMoved(position));
                    harness.run();
                }
                harness.event(egui::Event::PointerButton {
                    pos: end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                });
                harness.run();
                self.generation = self.generation.saturating_add(1);
                let after_state = snapshot(app, self.generation);
                self.record_event(id, method, "succeeded", before_generation, self.generation);
                success(
                    id.clone(),
                    json!({
                        "request_id": request_id,
                        "correlation_id": request_id,
                        "trace_id": format!("ui-{request_id}"),
                        "actor": "stdio-peer",
                        "permission_class": "ui-interaction",
                        "command": "ui.drag",
                        "source": {"role": role, "name": name, "bounds": {"x": start.x, "y": start.y}},
                        "destination": {"x": end.x, "y": end.y, "target": destination_target},
                        "steps": steps,
                        "state_generation_before": before_generation,
                        "state_generation_after": self.generation,
                        "before": before_state,
                        "after": after_state,
                        "outcome": "succeeded",
                    }),
                )
            }
            "actions.select" => {
                let (role, name) = match resolve_locator(harness, &params, "") {
                    Ok(locator) => locator,
                    Err((code, message)) => return failure(id.clone(), code, message),
                };
                let Some(option) = string_param(&params, "option") else {
                    return failure(id.clone(), "invalid_argument", "option is required".into());
                };
                if let Err(error) = self.check_generation(&params) {
                    return failure(id.clone(), error.0, error.1);
                }
                let before_generation = self.generation;
                let before_state = snapshot(app, before_generation);
                let selectors = matching_nodes(harness, &role, &name);
                if selectors.len() != 1 {
                    let (code, message) = if selectors.is_empty() {
                        ("target_not_found", format!("no {role} named {name:?}"))
                    } else {
                        (
                            "ambiguous_target",
                            format!("{} {role} controls are named {name:?}", selectors.len()),
                        )
                    };
                    return failure(id.clone(), code, message);
                }
                if selectors[0].accesskit_node().is_disabled() {
                    return failure(
                        id.clone(),
                        "target_disabled",
                        format!("{role} named {name:?} is disabled"),
                    );
                }
                selectors[0].click();
                harness.run();
                let options = matching_labels(harness, option);
                if options.len() != 1 {
                    let (code, message) = if options.is_empty() {
                        (
                            "option_not_found",
                            format!("no visible option named {option:?}"),
                        )
                    } else {
                        (
                            "ambiguous_option",
                            format!("{} visible options are named {option:?}", options.len()),
                        )
                    };
                    return failure(id.clone(), code, message);
                }
                if options[0].accesskit_node().is_disabled() {
                    return failure(
                        id.clone(),
                        "option_disabled",
                        format!("option {option:?} is disabled"),
                    );
                }
                options[0].click();
                harness.run();
                self.generation = self.generation.saturating_add(1);
                let after_state = snapshot(app, self.generation);
                self.record_event(id, method, "succeeded", before_generation, self.generation);
                success(
                    id.clone(),
                    json!({"request_id":request_id,"correlation_id":request_id,"trace_id":format!("ui-{request_id}"),"actor":"stdio-peer","permission_class":"ui-interaction","command":"ui.select","target":{"role":role,"name":name,"option":option},"state_generation_before":before_generation,"state_generation_after":self.generation,"before":before_state,"after":after_state,"outcome":"succeeded"}),
                )
            }
            "actions.keypress" => {
                let Some(key_name) = string_param(&params, "key") else {
                    return failure(id.clone(), "invalid_argument", "key is required".into());
                };
                let Some(key) = parse_key(key_name) else {
                    return failure(
                        id.clone(),
                        "invalid_argument",
                        format!("unsupported key: {key_name}"),
                    );
                };
                let modifiers = match parse_modifiers(params.get("modifiers")) {
                    Ok(modifiers) => modifiers,
                    Err(message) => return failure(id.clone(), "invalid_argument", message),
                };
                if let Err(error) = self.check_generation(&params) {
                    return failure(id.clone(), error.0, error.1);
                }
                let before_generation = self.generation;
                let before_state = snapshot(app, before_generation);
                harness.key_press_modifiers(modifiers, key);
                harness.run();
                self.generation = self.generation.saturating_add(1);
                let after_state = snapshot(app, self.generation);
                self.record_event(id, method, "succeeded", before_generation, self.generation);
                success(
                    id.clone(),
                    json!({"request_id":request_id,"correlation_id":request_id,"trace_id":format!("ui-{request_id}"),"actor":"stdio-peer","permission_class":"ui-interaction","command":"ui.keypress","key":key_name,"modifiers":modifier_names(&modifiers),"state_generation_before":before_generation,"state_generation_after":self.generation,"before":before_state,"after":after_state,"outcome":"succeeded"}),
                )
            }
            "ui.wait_for" => {
                let (role, name) = match resolve_locator(harness, &params, "") {
                    Ok(locator) => locator,
                    Err((code, message)) => return failure(id.clone(), code, message),
                };
                let timeout_ms = params
                    .get("timeout_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if timeout_ms > 5_000 {
                    return failure(
                        id.clone(),
                        "invalid_argument",
                        "timeout_ms must be between 0 and 5000".into(),
                    );
                }
                let started = Instant::now();
                loop {
                    let matches = matching_nodes(harness, &role, &name);
                    if matches.len() == 1 {
                        return success(
                            id.clone(),
                            json!({"role":role,"name":name,"visible":true,"state_generation":self.generation,"waited_ms":started.elapsed().as_millis()}),
                        );
                    }
                    if matches.len() > 1 {
                        return failure(
                            id.clone(),
                            "ambiguous_target",
                            format!("{} {role} controls are named {name:?}", matches.len()),
                        );
                    }
                    if started.elapsed() >= Duration::from_millis(timeout_ms) {
                        return failure(
                            id.clone(),
                            "wait_timeout",
                            format!("no visible {role} named {name:?} within {timeout_ms} ms"),
                        );
                    }
                    harness.run();
                    thread::sleep(Duration::from_millis(10));
                }
            }
            "screenshots.capture" => {
                let Some(filename) = string_param(&params, "filename") else {
                    return failure(
                        id.clone(),
                        "invalid_argument",
                        "filename is required".into(),
                    );
                };
                match capture_screenshot(harness, filename) {
                    Ok(result) => {
                        self.record_event(
                            id,
                            method,
                            "succeeded",
                            self.generation,
                            self.generation,
                        );
                        success(id.clone(), result)
                    }
                    Err((code, message)) => {
                        self.record_event(id, method, "failed", self.generation, self.generation);
                        failure(id.clone(), code, message)
                    }
                }
            }
            "protocol.close" => success(id.clone(), json!({"closed": true})),
            _ => failure(
                id.clone(),
                "unsupported_method",
                format!("unsupported method: {method}"),
            ),
        }
    }

    fn check_generation(&self, params: &Value) -> Result<(), (&'static str, String)> {
        let expected = params
            .get("expected_state_generation")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                (
                    "invalid_argument",
                    "expected_state_generation is required for UI actions".into(),
                )
            })?;
        if expected != self.generation {
            return Err((
                "stale_state",
                format!(
                    "expected state generation {expected}; current generation is {}",
                    self.generation
                ),
            ));
        }
        Ok(())
    }

    fn record_event(
        &mut self,
        id: &Value,
        method: &str,
        outcome: &str,
        before_generation: u64,
        after_generation: u64,
    ) {
        self.next_event_id = self.next_event_id.saturating_add(1);
        self.events.push_back(json!({
            "event_id": self.next_event_id,
            "timestamp_unix_ms": unix_time_ms(),
            "request_id": id,
            "correlation_id": id,
            "trace_id": format!("ui-{}", id.as_str().unwrap_or_default()),
            "type": "agent_ui.action.completed",
            "method": method,
            "outcome": outcome,
            "state_generation_before": before_generation,
            "state_generation_after": after_generation,
        }));
        while self.events.len() > EVENT_LIMIT {
            self.events.pop_front();
        }
    }

    fn cache_request(&mut self, id: String, canonical_request: String, response: Value) {
        self.request_order.push_back(id.clone());
        self.requests.insert(
            id,
            CachedRequest {
                canonical_request,
                response,
            },
        );
        while self.request_order.len() > REQUEST_CACHE_LIMIT {
            if let Some(expired) = self.request_order.pop_front() {
                self.requests.remove(&expired);
            }
        }
    }
}

fn matching_nodes<'tree>(
    harness: &'tree UiHarness<'_>,
    role: &str,
    name: &str,
) -> Vec<egui_kittest::Node<'tree>> {
    let wanted_role = role.to_owned();
    let wanted_name = name.to_owned();
    harness
        .query_all_by(move |node| {
            node.label().as_deref() == Some(wanted_name.as_str())
                && format!("{:?}", node.role()) == wanted_role
        })
        .collect()
}

fn matching_test_id_nodes<'tree>(
    harness: &'tree UiHarness<'_>,
    test_id: &str,
) -> Vec<egui_kittest::Node<'tree>> {
    let test_id = test_id.to_owned();
    harness
        .query_all_by(move |node| {
            let role = format!("{:?}", node.role());
            node.label()
                .as_deref()
                .and_then(|name| stable_test_id(&role, name))
                .as_deref()
                == Some(test_id.as_str())
        })
        .collect()
}

fn resolve_locator(
    harness: &UiHarness<'_>,
    params: &Value,
    prefix: &str,
) -> Result<(String, String), (&'static str, String)> {
    let test_id_key = format!("{prefix}test_id");
    let role_key = format!("{prefix}role");
    let name_key = format!("{prefix}name");
    let test_id = string_param(params, &test_id_key);
    let role = string_param(params, &role_key);
    let name = string_param(params, &name_key);
    match (test_id, role, name) {
        (Some(test_id), None, None) if !test_id.is_empty() && test_id.len() <= 128 => {
            let matches = matching_test_id_nodes(harness, test_id);
            match matches.as_slice() {
                [node] => Ok((
                    format!("{:?}", node.accesskit_node().role()),
                    node.accesskit_node().label().unwrap_or_default(),
                )),
                [] => Err((
                    "target_not_found",
                    format!("no control has test_id {test_id:?}"),
                )),
                _ => Err((
                    "ambiguous_target",
                    format!("{} controls have test_id {test_id:?}", matches.len()),
                )),
            }
        }
        (None, Some(role), Some(name)) => Ok((role.to_owned(), name.to_owned())),
        (Some(_), _, _) => Err((
            "invalid_argument",
            format!("provide {test_id_key} or both {role_key}/{name_key}"),
        )),
        (None, _, _) => Err((
            "invalid_argument",
            format!("provide both {role_key} and {name_key}"),
        )),
    }
}

fn matching_labels<'tree>(
    harness: &'tree UiHarness<'_>,
    name: &str,
) -> Vec<egui_kittest::Node<'tree>> {
    let wanted = name.to_owned();
    harness
        .query_all_by(move |node| node.label().as_deref() == Some(wanted.as_str()))
        .collect()
}

fn parse_key(name: &str) -> Option<egui::Key> {
    use egui::Key;
    Some(match name.to_ascii_uppercase().as_str() {
        "ENTER" => Key::Enter,
        "ESCAPE" => Key::Escape,
        "TAB" => Key::Tab,
        "SPACE" => Key::Space,
        "BACKSPACE" => Key::Backspace,
        "DELETE" => Key::Delete,
        "ARROWUP" | "UP" => Key::ArrowUp,
        "ARROWDOWN" | "DOWN" => Key::ArrowDown,
        "ARROWLEFT" | "LEFT" => Key::ArrowLeft,
        "ARROWRIGHT" | "RIGHT" => Key::ArrowRight,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PAGEUP" => Key::PageUp,
        "PAGEDOWN" => Key::PageDown,
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,
        _ => return None,
    })
}

fn parse_modifiers(value: Option<&Value>) -> Result<egui::Modifiers, String> {
    let Some(value) = value else {
        return Ok(egui::Modifiers::NONE);
    };
    let Some(values) = value.as_array() else {
        return Err("modifiers must be an array of modifier names".into());
    };
    if values.len() > 4 {
        return Err("modifiers accepts at most four names".into());
    }
    let mut names = HashSet::new();
    let mut modifiers = egui::Modifiers::NONE;
    for value in values {
        let Some(name) = value.as_str() else {
            return Err("each modifier must be a string".into());
        };
        let normalized = name.to_ascii_uppercase();
        if !names.insert(normalized.clone()) {
            return Err(format!("duplicate modifier: {name}"));
        }
        match normalized.as_str() {
            "ALT" => modifiers.alt = true,
            "CTRL" | "CONTROL" => modifiers.ctrl = true,
            "SHIFT" => modifiers.shift = true,
            "COMMAND" | "CMD" => modifiers.command = true,
            "META" => modifiers.mac_cmd = true,
            _ => return Err(format!("unsupported modifier: {name}")),
        }
    }
    Ok(modifiers)
}

fn modifier_names(modifiers: &egui::Modifiers) -> Vec<&'static str> {
    let mut names = Vec::new();
    if modifiers.alt {
        names.push("ALT");
    }
    if modifiers.ctrl {
        names.push("CTRL");
    }
    if modifiers.shift {
        names.push("SHIFT");
    }
    if modifiers.command {
        names.push("COMMAND");
    }
    if modifiers.mac_cmd {
        names.push("META");
    }
    names
}

fn describe_node(node: egui_kittest::Node<'_>) -> Value {
    let access = node.accesskit_node();
    let role = format!("{:?}", access.role());
    let name = access.label();
    let test_id = name.as_deref().and_then(|name| stable_test_id(&role, name));
    let bounds = access.bounding_box().map(
        |rect| json!({"x":rect.x0,"y":rect.y0,"width":rect.x1-rect.x0,"height":rect.y1-rect.y0}),
    );
    let children = node.children().map(describe_node).collect::<Vec<_>>();
    json!({
        "id": format!("{:?}", access.id()),
        "role": role,
        "name": name,
        "test_id": test_id,
        "description": access.description(),
        "value": access.value(),
        "disabled": access.is_disabled(),
        "focused": node.is_focused(),
        "bounds": bounds,
        "children": children
    })
}

fn stable_test_id(role: &str, name: &str) -> Option<String> {
    let fixed = match (role, name) {
        ("Group", "Sessions") => "panel.sessions",
        ("Tab", "New session") => "tab.workspace.new-session",
        ("Tab", "Dock tab Sessions") => "dock.tab.sessions",
        ("Tab", "Dock tab New session") => "dock.tab.workspace",
        ("Button", "Settings") => "app.settings.open",
        ("ComboBox", "Interface mode") => "app.interface-mode",
        ("ComboBox", "Layout profile") => "app.layout-profile",
        ("Button", "Review") => "pane.review.toggle",
        ("Button", "Files") => "pane.files.toggle",
        ("Button", "Terminal") => "pane.terminal.toggle",
        ("Button", "+  New session") => "session.create",
        ("Button", "Capabilities") => "route.capabilities",
        ("Button", "Messaging") => "route.messaging",
        ("Button", "Artifacts") => "route.artifacts",
        ("Button", "Scheduled jobs") => "route.cron",
        ("Button", "Show archived sessions") => "sessions.show-archived",
        ("Button", "Show active sessions") => "sessions.show-active",
        ("TextInput", "Search sessions") => "sessions.search",
        ("MultilineTextInput", "Message composer") => "chat.composer",
        ("Button", "Send") => "chat.send",
        ("Button", "Stop response") | ("Button", "Stopping response") => "chat.stop",
        ("Button", "Show earlier messages") | ("Button", "Loading earlier messages") => {
            "chat.transcript.show-earlier"
        }
        ("Button", "Connect to Hermes") => "gateway.connect",
        ("Button", "Install Hermes locally") => "gateway.install",
        ("Button", "Close") => "overlay.close",
        ("Button", "Skills") => "capabilities.tab.skills",
        ("Button", "Plugins") => "capabilities.tab.plugins",
        ("Button", "Connectors") => "capabilities.tab.connectors",
        ("Button", "Toolsets") => "capabilities.tab.toolsets",
        ("TextInput", "Search capabilities") => "capabilities.search",
        ("Button", "Refresh") => "capabilities.refresh",
        _ => "",
    };
    if !fixed.is_empty() {
        return Some(fixed.to_owned());
    }
    if role == "Button" {
        if let Some((_, scoped)) = name.rsplit_once(", profile ")
            && let Some((profile, session_id)) = scoped.split_once(", session ")
            && valid_session_test_id(profile)
            && valid_session_test_id(session_id)
        {
            return Some(format!("session.row.{profile}.{session_id}"));
        }
        if let Some(rest) = name.strip_prefix("Session actions profile ")
            && let Some((profile, session_id)) = rest.split_once(" session ")
            && valid_session_test_id(profile)
            && valid_session_test_id(session_id)
        {
            return Some(format!("session.actions.{profile}.{session_id}"));
        }
        for (label, prefix) in [
            ("Pin profile ", "session.pin."),
            ("Unpin profile ", "session.pin."),
            ("Mark unread profile ", "session.unread."),
            ("Mark read profile ", "session.unread."),
            ("Archive profile ", "session.archive."),
            ("Unarchive profile ", "session.archive."),
        ] {
            if let Some(rest) = name.strip_prefix(label)
                && let Some((profile, session_id)) = rest.split_once(" session ")
                && valid_session_test_id(profile)
                && valid_session_test_id(session_id)
            {
                return Some(format!("{prefix}{profile}.{session_id}"));
            }
        }
        if let Some((_, session_id)) = name.rsplit_once(", session ") {
            if valid_session_test_id(session_id) {
                return Some(format!("session.row.{session_id}"));
            }
        }
        if let Some(session_id) = name.strip_prefix("Session actions session ") {
            if valid_session_test_id(session_id) {
                return Some(format!("session.actions.{session_id}"));
            }
        }
        if let Some(session_id) = name
            .strip_prefix("Pin session ")
            .or_else(|| name.strip_prefix("Unpin session "))
        {
            if valid_session_test_id(session_id) {
                return Some(format!("session.pin.{session_id}"));
            }
        }
        if let Some(session_id) = name
            .strip_prefix("Mark unread session ")
            .or_else(|| name.strip_prefix("Mark read session "))
        {
            if valid_session_test_id(session_id) {
                return Some(format!("session.unread.{session_id}"));
            }
        }
        if let Some(session_id) = name
            .strip_prefix("Archive session ")
            .or_else(|| name.strip_prefix("Unarchive session "))
        {
            if valid_session_test_id(session_id) {
                return Some(format!("session.archive.{session_id}"));
            }
        }
    }
    None
}

fn valid_session_test_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id.len() <= 112
        && session_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_.".contains(ch))
}

fn snapshot(app: &UiApp, generation: u64) -> Value {
    let mut state = app.borrow().automation_state();
    if let Some(object) = state.as_object_mut() {
        object.insert("state_generation".into(), json!(generation));
    }
    state
}

fn capabilities() -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "transport": {"kind":"inherited_stdio_capability","network_listener":false},
        "app_actions": registered_ui_action_catalog(),
        "capabilities": [
            {"id":"health.get","method":"health.get","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"capabilities.list","method":"capabilities.list","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"state.get","method":"state.get","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"ui.tree","method":"ui.tree","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"app.actions.list","method":"app.actions.list","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"app.action.execute","method":"app.action.execute","permission":"ui_interaction","implemented":true,"verification":"headless_app_core"},
            {"id":"events.list","method":"events.list","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"diagnostics.bundle","method":"diagnostics.bundle","permission":"read_only","implemented":true,"verification":"runtime"},
            {"id":"ui.click","method":"actions.click","permission":"ui_interaction","implemented":true,"verification":"headless_semantic"},
            {"id":"ui.right_click","method":"actions.right_click","permission":"ui_interaction","implemented":true,"verification":"headless_pointer_events"},
            {"id":"ui.type","method":"actions.type","permission":"ui_interaction","implemented":true,"verification":"headless_semantic"},
            {"id":"ui.drag","method":"actions.drag","permission":"ui_interaction","implemented":true,"verification":"headless_pointer_events"},
            {"id":"ui.select","method":"actions.select","permission":"ui_interaction","implemented":true,"verification":"headless_semantic"},
            {"id":"ui.keypress","method":"actions.keypress","permission":"ui_interaction","implemented":true,"verification":"headless_semantic"},
            {"id":"ui.wait_for","method":"ui.wait_for","permission":"read_only","implemented":true,"verification":"headless_semantic"},
            {"id":"screenshot.capture","method":"screenshots.capture","permission":"artifact_write","implemented":true,"verification":"headless_render"},
            {"id":"native_window_control","method":null,"permission":"unsupported","implemented":false,"verification":"not_tested"},
            {"id":"stable_test_id_locator","method":"actions.click/actions.right_click/actions.type/actions.select/actions.drag/ui.wait_for","permission":"ui_interaction","implemented":true,"verification":"headless_semantic"}
        ]
    })
}

fn capture_screenshot(
    harness: &mut UiHarness<'_>,
    filename: &str,
) -> Result<Value, (&'static str, String)> {
    let path = Path::new(filename);
    if filename.is_empty()
        || filename.len() > 160
        || path.components().count() != 1
        || path.file_name().and_then(|part| part.to_str()) != Some(filename)
        || path.extension().and_then(|part| part.to_str()) != Some("png")
    {
        return Err((
            "invalid_filename",
            "filename must be a single .png name inside artifacts/agent-ui".into(),
        ));
    }
    let artifact_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("artifacts/agent-ui");
    std::fs::create_dir_all(&artifact_dir).map_err(|error| {
        (
            "artifact_error",
            format!("create evidence directory: {error}"),
        )
    })?;
    let root = artifact_dir.canonicalize().map_err(|error| {
        (
            "artifact_error",
            format!("resolve evidence directory: {error}"),
        )
    })?;
    let destination = root.join(filename);
    if destination.exists() {
        return Err((
            "artifact_exists",
            "refusing to overwrite existing screenshot evidence".into(),
        ));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ("clock_error", error.to_string()))?
        .as_nanos();
    let temporary = root.join(format!(".capture-{}-{nonce}.tmp.png", process::id()));
    let image = harness.render().map_err(|error| ("render_failed", error))?;
    image
        .save(&temporary)
        .map_err(|error| ("render_failed", format!("write temporary PNG: {error}")))?;
    let validation = std::fs::read(&temporary)
        .map_err(|error| ("artifact_error", format!("validate temporary PNG: {error}")))?;
    if !validation.starts_with(b"\x89PNG\r\n\x1a\n") {
        let _ = std::fs::remove_file(&temporary);
        return Err(("artifact_invalid", "renderer did not produce a PNG".into()));
    }
    std::fs::rename(&temporary, &destination).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        (
            "artifact_error",
            format!("commit screenshot atomically: {error}"),
        )
    })?;
    Ok(json!({
        "path": destination,
        "width": image.width(),
        "height": image.height(),
        "bytes": validation.len(),
        "permission_class": "artifact_write",
        "outcome": "succeeded"
    }))
}

fn read_bounded_line(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        if line.len().saturating_add(take) > MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request exceeds 65536 bytes",
            ));
        }
        let has_newline = available.get(take - 1) == Some(&b'\n');
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if has_newline {
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
    }
}

fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), String> {
    let encoded = serde_json::to_vec(message).map_err(|error| error.to_string())?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err("response exceeds 256 KiB bound".into());
    }
    writer
        .write_all(&encoded)
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|error| format!("write bridge response: {error}"))
}

fn string_param<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

fn success(id: Value, result: Value) -> Value {
    json!({"version":PROTOCOL_VERSION,"id":id,"status":"succeeded","result":result})
}

fn failure(id: Value, code: &str, message: String) -> Value {
    json!({
        "version":PROTOCOL_VERSION,
        "id":id,
        "status":"failed",
        "error":{"code":code,"message":message}
    })
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_app_action_and_human_widget_share_the_registered_handler() {
        let profile = super::super::TemporaryProfile::create().expect("isolated profile");
        let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
            profile.0.clone(),
        )));
        let frame_app = app.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
        let mut protocol = ProtocolState::default();

        let registry = protocol.handle(
            json!({"version":1,"id":"action-catalog","method":"app.actions.list","params":{}}),
            &mut harness,
            &app,
        );
        assert_eq!(registry["status"], "succeeded");
        assert_eq!(registry["result"]["actions"].as_array().unwrap().len(), 12);
        assert!(
            registry["result"]["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["type"] == "open_session")
        );
        assert!(
            registry["result"]["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["type"] == "set_toolset_enabled")
        );
        assert!(
            registry["result"]["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["type"] == "stop_generation")
        );
        assert!(
            registry["result"]["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["type"] == "respond_approval")
        );
        assert!(
            registry["result"]["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["type"] == "respond_clarify")
        );
        assert!(
            registry["result"]["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["type"] == "close_active_tab")
        );

        let capability_response = protocol.handle(
            json!({"version":1,"id":"capability-catalog","method":"capabilities.list","params":{}}),
            &mut harness,
            &app,
        );
        let capability_ids = capability_response["result"]["capabilities"]
            .as_array()
            .expect("capability catalog")
            .iter()
            .map(|entry| entry["id"].as_str().expect("capability id"))
            .collect::<Vec<_>>();
        assert_eq!(
            capability_ids.iter().collect::<HashSet<_>>().len(),
            capability_ids.len(),
            "capability IDs must be unique"
        );

        let action = protocol.handle(
            json!({
                "version": 1,
                "id": "typed-toggle-files",
                "method": "app.action.execute",
                "params": {
                    "action": {"type":"toggle_pane","panel":"files"},
                    "expected_state_generation": 0
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(action["status"], "succeeded");
        assert_eq!(action["result"]["after"]["panels"]["files"], true);

        let human_path = protocol.handle(
            json!({
                "version": 1,
                "id": "human-toggle-files",
                "method": "actions.click",
                "params": {
                    "test_id": "pane.files.toggle",
                    "expected_state_generation": 1
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(human_path["status"], "succeeded");
        assert_eq!(human_path["result"]["after"]["panels"]["files"], false);

        let rejected = protocol.handle(
            json!({
                "version": 1,
                "id": "reject-unregistered-pane",
                "method": "app.action.execute",
                "params": {
                    "action": {"type":"toggle_pane","panel":"workspace"},
                    "expected_state_generation": 2
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(rejected["status"], "failed");
        assert_eq!(rejected["error"]["code"], "action_rejected");
        assert_eq!(app.borrow().automation_state()["panels"]["files"], false);

        let rejected_toolset = protocol.handle(
            json!({
                "version": 1,
                "id": "reject-offline-toolset-change",
                "method": "app.action.execute",
                "params": {
                    "action": {"type":"set_toolset_enabled","name":"web","enabled":true},
                    "expected_state_generation": 2
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(rejected_toolset["status"], "failed");
        assert_eq!(rejected_toolset["error"]["code"], "action_rejected");
        assert_eq!(app.borrow().automation_state()["panels"]["files"], false);

        let rejected_stop = protocol.handle(
            json!({
                "version": 1,
                "id": "reject-offline-stop",
                "method": "app.action.execute",
                "params": {
                    "action": {"type":"stop_generation"},
                    "expected_state_generation": 2
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(rejected_stop["status"], "failed");
        assert_eq!(rejected_stop["error"]["code"], "action_rejected");
        assert_eq!(app.borrow().automation_state()["panels"]["files"], false);
        drop(app);
        std::fs::remove_dir_all(profile.0.clone()).expect("remove isolated test profile");
    }

    #[test]
    fn stable_test_ids_are_emitted_and_work_as_action_locators() {
        assert_eq!(
            stable_test_id("Button", "Stop response").as_deref(),
            Some("chat.stop")
        );
        assert_eq!(
            stable_test_id("Button", "Stopping response").as_deref(),
            Some("chat.stop")
        );
        assert_eq!(
            stable_test_id("Button", "Show earlier messages").as_deref(),
            Some("chat.transcript.show-earlier")
        );
        assert_eq!(
            stable_test_id("Button", "Loading earlier messages").as_deref(),
            Some("chat.transcript.show-earlier")
        );
        assert_eq!(
            stable_test_id("Button", "Release notes, session sess-42").as_deref(),
            Some("session.row.sess-42")
        );
        assert_eq!(
            stable_test_id("Button", "Session actions session sess-42").as_deref(),
            Some("session.actions.sess-42")
        );
        assert_eq!(
            stable_test_id("Button", "Pin session sess-42").as_deref(),
            Some("session.pin.sess-42")
        );
        assert_eq!(
            stable_test_id("Button", "Mark unread session sess-42").as_deref(),
            Some("session.unread.sess-42")
        );
        assert_eq!(
            stable_test_id("Button", "Archive session sess-42").as_deref(),
            Some("session.archive.sess-42")
        );
        assert_eq!(
            stable_test_id("Button", "Show archived sessions").as_deref(),
            Some("sessions.show-archived")
        );
        assert_eq!(
            stable_test_id("Button", "Show active sessions").as_deref(),
            Some("sessions.show-active")
        );
        assert_eq!(
            stable_test_id(
                "Button",
                "Archived planning, profile work, session old-session-9"
            )
            .as_deref(),
            Some("session.row.work.old-session-9")
        );
        assert_eq!(
            stable_test_id("Button", "Unarchive profile work session old-session-9").as_deref(),
            Some("session.archive.work.old-session-9")
        );
        let profile = super::super::TemporaryProfile::create().expect("isolated profile");
        let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
            profile.0.clone(),
        )));
        let frame_app = app.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
        let tree = describe_node(harness.root());
        assert_eq!(
            matching_test_id_nodes(&harness, "route.capabilities").len(),
            1
        );
        let tree_text = tree.to_string();
        assert!(tree_text.contains("sessions.search"));
        assert!(tree_text.contains("app.interface-mode"));

        let mut protocol = ProtocolState::default();
        let response = protocol.handle(
            json!({
                "version": 1,
                "id": "stable-route-click",
                "method": "actions.click",
                "params": {
                    "test_id": "route.capabilities",
                    "expected_state_generation": 0
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(response["status"], "succeeded");
        assert_eq!(
            response["result"]["after"]["route"],
            "/capabilities?tab=skills"
        );
        assert_eq!(response["result"]["target"]["name"], "Capabilities");
        let capabilities_tree = describe_node(harness.root()).to_string();
        assert!(capabilities_tree.contains("capabilities.tab.toolsets"));
        assert!(capabilities_tree.contains("capabilities.search"));

        let toolsets = protocol.handle(
            json!({
                "version": 1,
                "id": "stable-capabilities-tab",
                "method": "actions.click",
                "params": {
                    "test_id": "capabilities.tab.toolsets",
                    "expected_state_generation": 1
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(toolsets["status"], "succeeded");
        assert_eq!(
            toolsets["result"]["after"]["route"],
            "/capabilities?tab=toolsets"
        );

        let typed = protocol.handle(
            json!({
                "version": 1,
                "id": "stable-search-type",
                "method": "actions.type",
                "params": {
                    "test_id": "capabilities.search",
                    "text": "needle",
                    "expected_state_generation": 2
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(typed["status"], "succeeded");
        assert_eq!(typed["result"]["target"]["name"], "Search capabilities");

        let waited = protocol.handle(
            json!({
                "version": 1,
                "id": "stable-search-wait",
                "method": "ui.wait_for",
                "params": {"test_id":"capabilities.search","timeout_ms":0}
            }),
            &mut harness,
            &app,
        );
        assert_eq!(waited["status"], "succeeded");
        assert_eq!(waited["result"]["visible"], true);

        let selected = protocol.handle(
            json!({
                "version": 1,
                "id": "stable-mode-select",
                "method": "actions.select",
                "params": {
                    "test_id": "app.interface-mode",
                    "option": "Simple",
                    "expected_state_generation": 3
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(selected["status"], "succeeded");
        assert_eq!(selected["result"]["target"]["name"], "Interface mode");
        drop(app);
        std::fs::remove_dir_all(profile.0.clone()).expect("remove isolated test profile");
    }

    #[test]
    fn pointer_drag_and_dock_tab_selection_use_the_live_ui_path() {
        let profile = super::super::TemporaryProfile::create().expect("isolated profile");
        let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
            profile.0.clone(),
        )));
        let frame_app = app.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
        let mut protocol = ProtocolState::default();

        let invalid = protocol.handle(
            json!({
                "version": 1,
                "id": "invalid-drag",
                "method": "actions.drag",
                "params": {
                    "role": "Group",
                    "name": "Sessions",
                    "to_x": 880.0,
                    "to_y": 480.0,
                    "steps": 33,
                    "expected_state_generation": 0
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(invalid["status"], "failed");
        assert_eq!(invalid["error"]["code"], "invalid_argument");

        let dragged = protocol.handle(
            json!({
                "version": 1,
                "id": "center-drag",
                "method": "actions.drag",
                "params": {
                    "test_id": "panel.sessions",
                    "to_x": 880.0,
                    "to_y": 480.0,
                    "steps": 10,
                    "expected_state_generation": 0
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(dragged["status"], "succeeded");
        assert_eq!(dragged["result"]["state_generation_after"], 1);
        let groups = &dragged["result"]["after"]["dock_tab_groups"];
        assert_eq!(groups[0]["panels"][0], "workspace");
        assert_eq!(groups[0]["panels"][1], "sessions");
        assert_eq!(groups[0]["active_index"], 1);

        assert_eq!(
            matching_nodes(&harness, "Tab", "Dock tab Sessions").len(),
            1,
            "the inactive docked panel must remain visibly reachable"
        );

        assert_eq!(
            matching_nodes(&harness, "Tab", "Dock tab New session").len(),
            1,
            "workspace dock tab must be a unique semantic destination"
        );
        let reordered = protocol.handle(
            json!({
                "version": 1,
                "id": "reorder-sessions-tab",
                "method": "actions.drag",
                "params": {
                    "test_id": "dock.tab.sessions",
                    "to_test_id": "dock.tab.workspace",
                    "steps": 10,
                    "expected_state_generation": 1
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(reordered["status"], "succeeded", "{reordered}");
        let reordered_group = &reordered["result"]["after"]["dock_tab_groups"][0];
        assert_eq!(reordered_group["panels"], json!(["sessions", "workspace"]));
        assert_eq!(reordered_group["active_index"], 0);
        assert_eq!(reordered["result"]["after"]["layout_revision"], 3);

        let switched = protocol.handle(
            json!({
                "version": 1,
                "id": "select-workspace-tab",
                "method": "actions.click",
                "params": {
                    "role": "Tab",
                    "name": "Dock tab New session",
                    "expected_state_generation": 2
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(switched["status"], "succeeded");
        assert_eq!(
            switched["result"]["after"]["dock_tab_groups"][0]["active_index"],
            1
        );

        let right_clicked = protocol.handle(
            json!({
                "version": 1,
                "id": "open-dock-tab-menu",
                "method": "actions.right_click",
                "params": {
                    "test_id": "dock.tab.workspace",
                    "expected_state_generation": 3
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(right_clicked["status"], "succeeded", "{right_clicked}");
        assert_eq!(right_clicked["result"]["command"], "ui.right_click");
        assert_eq!(right_clicked["result"]["state_generation_after"], 4);
        assert!(
            !matching_labels(&harness, "Close all").is_empty(),
            "the agent's real secondary-pointer path must open the same DoofusDock context menu"
        );
        assert!(
            protocol.handle(
                json!({"version":1,"id":"right-click-capability","method":"capabilities.list","params":{}}),
                &mut harness,
                &app,
            )["result"]["capabilities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["id"] == "ui.right_click"),
            "secondary-pointer interaction must be discoverable"
        );

        drop(harness);
        drop(app);
        drop(profile);
    }

    #[test]
    fn modifier_keypress_routes_command_w_through_the_shared_close_handler() {
        let profile = super::super::TemporaryProfile::create().expect("isolated profile");
        let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
            profile.0.clone(),
        )));
        app.borrow_mut()
            .execute_registered_action(RegisteredUiAction::OpenSession {
                session_id: "keyboard-close-session".into(),
                profile: Some("work".into()),
            })
            .expect("open an isolated session tab");
        let frame_app = app.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
        let mut protocol = ProtocolState::default();
        let session_panel = app.borrow().automation_state()["layout_document"]["panels"]
            .as_array()
            .expect("layout panel descriptors")
            .iter()
            .find(|panel| {
                panel["role"] == "hermes_session_tile"
                    && panel["metadata"]["hermes.session.id"] == "keyboard-close-session"
            })
            .and_then(|panel| panel["id"].as_str())
            .expect("profile-scoped session pane")
            .to_owned();

        let invalid = protocol.handle(
            json!({
                "version":1,
                "id":"reject-unsupported-modifier",
                "method":"actions.keypress",
                "params":{"key":"W","modifiers":["SUPER"],"expected_state_generation":0}
            }),
            &mut harness,
            &app,
        );
        assert_eq!(invalid["status"], "failed");
        assert_eq!(invalid["error"]["code"], "invalid_argument");

        let closed = protocol.handle(
            json!({
                "version":1,
                "id":"command-w-close-session-tab",
                "method":"actions.keypress",
                "params":{"key":"W","modifiers":["COMMAND"],"expected_state_generation":0}
            }),
            &mut harness,
            &app,
        );
        assert_eq!(closed["status"], "succeeded", "{closed}");
        assert_eq!(closed["result"]["modifiers"], json!(["COMMAND"]));
        assert!(
            closed["result"]["after"]["layout_document"]["closed_panels"]
                .as_array()
                .is_some_and(|panels| panels.iter().any(|panel| panel == &session_panel)),
            "Command+W from the agent must invoke the human close-tab handler"
        );
        drop(harness);
        drop(app);
        drop(profile);
    }

    #[test]
    fn pointer_drag_edge_drop_commits_a_canonical_split() {
        let profile = super::super::TemporaryProfile::create().expect("isolated profile");
        let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
            profile.0.clone(),
        )));
        let frame_app = app.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
        let mut protocol = ProtocolState::default();
        let response = protocol.handle(
            json!({
                "version": 1,
                "id": "left-edge-drag",
                "method": "actions.drag",
                "params": {
                    "role": "Group",
                    "name": "Sessions",
                    "to_x": 350.0,
                    "to_y": 480.0,
                    "steps": 10,
                    "expected_state_generation": 0
                }
            }),
            &mut harness,
            &app,
        );
        assert_eq!(response["status"], "succeeded");
        let state = app.borrow().automation_state();
        assert_eq!(state["layout_document"]["root"]["type"], "split");
        assert_eq!(
            state["layout_document"]["root"]["orientation"],
            "horizontal"
        );
        assert_eq!(
            state["layout_document"]["root"]["weights"],
            json!([0.4, 0.6])
        );
        assert_eq!(state["dock_tab_groups"], json!([]));
        drop(harness);
        drop(app);
        drop(profile);
    }

    #[test]
    fn pointer_drag_resizes_a_split_through_the_canonical_transaction() {
        let profile = super::super::TemporaryProfile::create().expect("isolated profile");
        let app = Rc::new(RefCell::new(HermesNativeApp::new_for_automation(
            profile.0.clone(),
        )));
        let frame_app = app.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui(move |ui| frame_app.borrow_mut().draw(ui));
        let mut protocol = ProtocolState::default();
        let before = app.borrow().automation_state();
        let initial_weights = before["layout_document"]["root"]["weights"].clone();
        let split_id = before["layout_document"]["root"]["id"]
            .as_str()
            .expect("root split id");
        let handle_name = format!("Resize split {split_id} divider 1");
        let request = json!({
            "version": 1,
            "id": "resize-split",
            "method": "actions.drag",
            "params": {
                "role": "Button",
                "name": handle_name,
                "to_x": 430.0,
                "to_y": 480.0,
                "steps": 10,
                "expected_state_generation": 0
            }
        });
        let response = protocol.handle(request, &mut harness, &app);
        assert_eq!(response["status"], "succeeded");
        let after = &response["result"]["after"];
        let resized_weights = &after["layout_document"]["root"]["weights"];
        assert_ne!(*resized_weights, initial_weights);
        let weight_sum = resized_weights[0].as_f64().expect("first split weight")
            + resized_weights[1].as_f64().expect("second split weight");
        assert!((weight_sum - 1.0).abs() < 1e-9);
        assert!(
            after["layout_revision"].as_u64().unwrap()
                > before["layout_revision"].as_u64().unwrap()
        );
        assert!(after["persistence_error"].is_null());
        assert_eq!(response["result"]["command"], "ui.drag");
        drop(harness);
        drop(app);
        drop(profile);
    }
}
