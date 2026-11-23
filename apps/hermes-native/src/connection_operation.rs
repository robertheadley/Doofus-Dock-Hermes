//! Bounded, owner-scoped projection of Hermes connection-operation events.
//!
//! `manage_connections` and `manage_catalog` are represented by resumable
//! `connection.request` / `connection.update` events and answered with the
//! ordinary `connection.respond` RPC. This module stores only server-provided
//! metadata; submitted credential values are deliberately absent from these
//! types.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

const MAX_PAYLOAD_BYTES: usize = 256 * 1024;
const MAX_OPERATION_ID_BYTES: usize = 256;
const MAX_SESSION_ID_BYTES: usize = 256;
const MAX_TARGETS: usize = 64;
const MAX_ENV_FIELDS: usize = 64;
const MAX_LIST_ITEMS: usize = 128;
const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_URL_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTargetKind {
    Connector,
    Mcp,
    Plugin,
    Skill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTargetAction {
    Authorize,
    Connect,
    Enable,
    Install,
    Reconnect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTargetState {
    Connected,
    Expired,
    Failed,
    Initiated,
    NotConnected,
    Pending,
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionSettleReason {
    AllResolved,
    Continue,
    Deadline,
    Interrupt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionActor {
    User,
    BackendWatcher,
    Clock,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConnectionEnvField {
    pub name: String,
    pub required: bool,
    pub secret: bool,
    pub default: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConnectionLaunch {
    pub kind: String,
    pub target: String,
    /// Environment variable names only. Secret values are not part of event snapshots.
    pub env: Vec<String>,
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConnectionOperationTarget {
    pub name: String,
    pub kind: ConnectionTargetKind,
    pub action: ConnectionTargetAction,
    pub state: ConnectionTargetState,
    pub resolved: Option<bool>,
    pub detail: Option<String>,
    pub instructions: Option<String>,
    pub discovery_error: Option<String>,
    pub connect_url: Option<String>,
    pub connection_id: Option<String>,
    pub alias: Option<String>,
    pub attempt: Option<String>,
    pub required_env: Vec<ConnectionEnvField>,
    pub launch: Option<ConnectionLaunch>,
    pub tools: Vec<String>,
    pub hint: Option<String>,
    pub display: Option<String>,
    pub description: Option<String>,
    pub tier: Option<String>,
    pub platforms: Vec<String>,
    pub repo: Option<String>,
    pub sha: Option<String>,
    pub subdir: Option<String>,
    pub scan_status: Option<String>,
    pub scan_summary: Option<String>,
    pub requires_hermes: Option<String>,
    pub has_desktop_half: Option<bool>,
    pub target_profile: Option<String>,
    pub skill: Option<String>,
    pub phase: Option<String>,
    pub enabled: Option<bool>,
    pub missing_env: Vec<String>,
    pub already_installed: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConnectionDisplay {
    pub suggested: Vec<String>,
}

/// Current server-owned state of one session's setup operation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConnectionOperation {
    pub session_id: String,
    pub op_id: String,
    pub seq: u64,
    pub deadline_at: f64,
    pub timeout_seconds: f64,
    pub targets: Vec<ConnectionOperationTarget>,
    pub tool_call_id: Option<String>,
    pub display: Option<ConnectionDisplay>,
    pub settled: bool,
    pub settled_at: Option<f64>,
    pub settled_by: Option<ConnectionSettleReason>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConnectionOperationUpdate {
    pub session_id: String,
    pub op_id: String,
    pub seq: u64,
    pub deadline_at: f64,
    pub settled: bool,
    pub settled_at: Option<f64>,
    pub settled_by: Option<ConnectionSettleReason>,
    pub targets: Vec<ConnectionOperationTarget>,
    pub target: Option<String>,
    pub from: Option<ConnectionTargetState>,
    pub to: Option<ConnectionTargetState>,
    pub actor: Option<ConnectionActor>,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateApply {
    Applied,
    StaleSequence,
    WrongOperation,
    WrongSession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    PayloadTooLarge,
    InvalidShape,
    MissingField,
    InvalidValue,
    TooManyItems,
    TextTooLong,
}

impl ConnectionOperation {
    /// Parse the `connection.request` event payload and bind it to the event's
    /// enclosing runtime session. The session identity is not accepted from
    /// the nested payload.
    pub fn parse_request(payload: &Value, session_id: &str) -> Result<Self, ParseError> {
        ensure_bounded_payload(payload)?;
        let session_id = required_id(session_id, MAX_SESSION_ID_BYTES)?;
        let object = payload.as_object().ok_or(ParseError::InvalidShape)?;
        let op_id = required_string(object.get("op_id"), MAX_OPERATION_ID_BYTES)?;
        let seq = required_u64(object.get("seq"))?;
        if seq == 0 {
            return Err(ParseError::InvalidValue);
        }
        let deadline_at = required_finite_number(object.get("deadline_at"))?;
        let timeout_seconds = required_finite_number(object.get("timeout_seconds"))?;
        if timeout_seconds < 0.0 {
            return Err(ParseError::InvalidValue);
        }
        let targets = parse_targets(object.get("targets"))?;
        let tool_call_id = optional_string(object.get("tool_call_id"), MAX_OPERATION_ID_BYTES)?;
        let display = object
            .get("display")
            .filter(|value| !value.is_null())
            .map(parse_display)
            .transpose()?;

        Ok(Self {
            session_id,
            op_id,
            seq,
            deadline_at,
            timeout_seconds,
            targets,
            tool_call_id,
            display,
            settled: false,
            settled_at: None,
            settled_by: None,
        })
    }

    /// Apply only a newer full-snapshot `connection.update` for this exact
    /// operation and runtime session. State remains backend-owned.
    pub fn apply_update(&mut self, update: ConnectionOperationUpdate) -> UpdateApply {
        if self.op_id != update.op_id {
            return UpdateApply::WrongOperation;
        }
        if self.session_id != update.session_id {
            return UpdateApply::WrongSession;
        }
        if update.seq <= self.seq {
            return UpdateApply::StaleSequence;
        }

        self.seq = update.seq;
        self.deadline_at = update.deadline_at;
        self.targets = update.targets;
        self.settled = update.settled;
        self.settled_at = update.settled_at;
        self.settled_by = update.settled_by;
        UpdateApply::Applied
    }
}

impl ConnectionOperationUpdate {
    /// Parse the session-owned `connection.update` full snapshot. Account
    /// operations intentionally fail this parser and are handled elsewhere.
    pub fn parse(payload: &Value) -> Result<Self, ParseError> {
        ensure_bounded_payload(payload)?;
        let object = payload.as_object().ok_or(ParseError::InvalidShape)?;
        let owner = object
            .get("owner")
            .and_then(Value::as_object)
            .ok_or(ParseError::MissingField)?;
        if required_string(owner.get("type"), 32)? != "session" {
            return Err(ParseError::InvalidValue);
        }
        let session_id = required_string(owner.get("session_id"), MAX_SESSION_ID_BYTES)?;
        let op_id = required_string(object.get("op_id"), MAX_OPERATION_ID_BYTES)?;
        let seq = required_u64(object.get("seq"))?;
        if seq == 0 {
            return Err(ParseError::InvalidValue);
        }
        let deadline_at = required_finite_number(object.get("deadline_at"))?;
        let settled = object
            .get("settled")
            .and_then(Value::as_bool)
            .ok_or(ParseError::MissingField)?;
        let settled_at = optional_finite_number(object.get("settled_at"))?;
        let settled_by = parse_settle_reason(object.get("settled_by"))?;
        let targets = parse_targets(object.get("targets"))?;

        Ok(Self {
            session_id,
            op_id,
            seq,
            deadline_at,
            settled,
            settled_at,
            settled_by,
            targets,
            target: optional_string(object.get("target"), MAX_OPERATION_ID_BYTES)?,
            from: parse_target_state(object.get("from"))?,
            to: parse_target_state(object.get("to"))?,
            actor: parse_actor(object.get("actor"))?,
            detail: optional_string(object.get("detail"), MAX_TEXT_BYTES)?,
        })
    }
}

fn ensure_bounded_payload(value: &Value) -> Result<(), ParseError> {
    if !value.is_object() {
        return Err(ParseError::InvalidShape);
    }
    if serde_json::to_vec(value)
        .map_err(|_| ParseError::InvalidShape)?
        .len()
        > MAX_PAYLOAD_BYTES
    {
        return Err(ParseError::PayloadTooLarge);
    }
    Ok(())
}

fn required_id(value: &str, max: usize) -> Result<String, ParseError> {
    let value = value.trim();
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(ParseError::InvalidValue);
    }
    Ok(value.to_owned())
}

fn required_string(value: Option<&Value>, max: usize) -> Result<String, ParseError> {
    let value = value
        .and_then(Value::as_str)
        .ok_or(ParseError::MissingField)?;
    bounded_string(value, max, false)
}

fn optional_string(value: Option<&Value>, max: usize) -> Result<Option<String>, ParseError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(bounded_string(value, max, true)?)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

fn bounded_string(value: &str, max: usize, allow_empty: bool) -> Result<String, ParseError> {
    if value.len() > max {
        return Err(ParseError::TextTooLong);
    }
    if !allow_empty && value.trim().is_empty() {
        return Err(ParseError::InvalidValue);
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ParseError::InvalidValue);
    }
    Ok(value.to_owned())
}

fn required_u64(value: Option<&Value>) -> Result<u64, ParseError> {
    value
        .and_then(Value::as_u64)
        .ok_or(ParseError::MissingField)
}

fn required_finite_number(value: Option<&Value>) -> Result<f64, ParseError> {
    let number = value
        .and_then(Value::as_f64)
        .ok_or(ParseError::MissingField)?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(ParseError::InvalidValue)
    }
}

fn optional_finite_number(value: Option<&Value>) -> Result<Option<f64>, ParseError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(required_finite_number(Some(value))?)),
    }
}

fn parse_targets(value: Option<&Value>) -> Result<Vec<ConnectionOperationTarget>, ParseError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or(ParseError::MissingField)?;
    if values.len() > MAX_TARGETS {
        return Err(ParseError::TooManyItems);
    }
    let mut names = BTreeSet::new();
    let mut targets = Vec::with_capacity(values.len());
    for value in values {
        let target = parse_target(value)?;
        if !names.insert(target.name.clone()) {
            return Err(ParseError::InvalidValue);
        }
        targets.push(target);
    }
    Ok(targets)
}

fn parse_target(value: &Value) -> Result<ConnectionOperationTarget, ParseError> {
    let object = value.as_object().ok_or(ParseError::InvalidShape)?;
    let name = required_string(object.get("name"), MAX_OPERATION_ID_BYTES)?;
    let kind = parse_kind(object.get("kind"))?.ok_or(ParseError::MissingField)?;
    let action = parse_action(object.get("action"))?.ok_or(ParseError::MissingField)?;
    let state = parse_target_state(object.get("state"))?.ok_or(ParseError::MissingField)?;
    let required_env = parse_env_fields(object.get("required_env"))?;
    let launch = parse_launch(object.get("launch"))?;
    let tools = optional_string_list(object.get("tools"), MAX_LIST_ITEMS, MAX_OPERATION_ID_BYTES)?;
    let platforms = optional_string_list(object.get("platforms"), MAX_LIST_ITEMS, 128)?;
    let missing_env = optional_string_list(object.get("missing_env"), MAX_ENV_FIELDS, 256)?;
    let scan = object.get("scan").filter(|value| !value.is_null());
    let (scan_status, scan_summary) = match scan {
        None => (None, None),
        Some(scan) => {
            let scan = scan.as_object().ok_or(ParseError::InvalidShape)?;
            let status = optional_string(scan.get("status"), 64)?;
            if status
                .as_deref()
                .is_some_and(|status| !matches!(status, "passed" | "warnings" | "failed"))
            {
                return Err(ParseError::InvalidValue);
            }
            (
                status,
                optional_string(scan.get("summary"), MAX_TEXT_BYTES)?,
            )
        }
    };

    Ok(ConnectionOperationTarget {
        name,
        kind,
        action,
        state,
        resolved: optional_bool(object.get("resolved"))?,
        detail: optional_string(object.get("detail"), MAX_TEXT_BYTES)?,
        instructions: optional_string(object.get("instructions"), MAX_TEXT_BYTES)?,
        discovery_error: optional_string(object.get("discovery_error"), MAX_TEXT_BYTES)?,
        connect_url: optional_external_link(object.get("connect_url"))?,
        connection_id: optional_string(object.get("connection_id"), MAX_OPERATION_ID_BYTES)?,
        alias: optional_string(object.get("alias"), MAX_OPERATION_ID_BYTES)?,
        attempt: optional_string(object.get("attempt"), MAX_OPERATION_ID_BYTES)?,
        required_env,
        launch,
        tools,
        hint: optional_string(object.get("hint"), MAX_TEXT_BYTES)?,
        display: optional_string(object.get("display"), MAX_TEXT_BYTES)?,
        description: optional_string(object.get("description"), MAX_TEXT_BYTES)?,
        tier: optional_enum_string(object.get("tier"), &["official", "verified", "community"])?,
        platforms,
        repo: optional_string(object.get("repo"), MAX_URL_BYTES)?,
        sha: optional_string(object.get("sha"), 128)?,
        subdir: optional_string(object.get("subdir"), MAX_OPERATION_ID_BYTES)?,
        scan_status,
        scan_summary,
        requires_hermes: optional_string(object.get("requires_hermes"), 256)?,
        has_desktop_half: optional_bool(object.get("has_desktop_half"))?,
        target_profile: optional_string(object.get("target_profile"), MAX_OPERATION_ID_BYTES)?,
        skill: optional_string(object.get("skill"), MAX_OPERATION_ID_BYTES)?,
        phase: optional_string(object.get("phase"), 128)?,
        enabled: optional_bool(object.get("enabled"))?,
        missing_env,
        already_installed: optional_bool(object.get("already_installed"))?,
    })
}

fn parse_env_fields(value: Option<&Value>) -> Result<Vec<ConnectionEnvField>, ParseError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(Vec::new());
    };
    let fields = value.as_array().ok_or(ParseError::InvalidShape)?;
    if fields.len() > MAX_ENV_FIELDS {
        return Err(ParseError::TooManyItems);
    }
    let mut names = BTreeSet::new();
    let mut parsed = Vec::with_capacity(fields.len());
    for field in fields {
        let object = field.as_object().ok_or(ParseError::InvalidShape)?;
        let name = required_string(object.get("name"), 256)?;
        if !names.insert(name.clone()) {
            return Err(ParseError::InvalidValue);
        }
        let required = object
            .get("required")
            .and_then(Value::as_bool)
            .ok_or(ParseError::MissingField)?;
        let secret = object
            .get("secret")
            .and_then(Value::as_bool)
            .ok_or(ParseError::MissingField)?;
        parsed.push(ConnectionEnvField {
            name,
            required,
            secret,
            // A secret's declared default is not useful for display and must
            // not be retained in the UI projection, even if a server sends it.
            default: if secret {
                None
            } else {
                optional_string(object.get("default"), 4096)?
            },
            prompt: optional_string(object.get("prompt"), 4096)?,
        });
    }
    Ok(parsed)
}

fn parse_launch(value: Option<&Value>) -> Result<Option<ConnectionLaunch>, ParseError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let object = value.as_object().ok_or(ParseError::InvalidShape)?;
    let kind = required_string(object.get("kind"), 32)?;
    if !matches!(kind.as_str(), "remote" | "stdio") {
        return Err(ParseError::InvalidValue);
    }
    Ok(Some(ConnectionLaunch {
        kind,
        target: required_string(object.get("target"), MAX_URL_BYTES)?,
        env: optional_string_list(object.get("env"), MAX_ENV_FIELDS, 256)?,
        cwd: optional_string(object.get("cwd"), MAX_URL_BYTES)?,
    }))
}

fn parse_display(value: &Value) -> Result<ConnectionDisplay, ParseError> {
    let object = value.as_object().ok_or(ParseError::InvalidShape)?;
    Ok(ConnectionDisplay {
        suggested: optional_string_list(object.get("suggested"), MAX_LIST_ITEMS, 256)?,
    })
}

fn optional_string_list(
    value: Option<&Value>,
    max_items: usize,
    max_bytes: usize,
) -> Result<Vec<String>, ParseError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(Vec::new());
    };
    let values = value.as_array().ok_or(ParseError::InvalidShape)?;
    if values.len() > max_items {
        return Err(ParseError::TooManyItems);
    }
    values
        .iter()
        .map(|value| required_string(Some(value), max_bytes))
        .collect()
}

fn optional_bool(value: Option<&Value>) -> Result<Option<bool>, ParseError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

fn optional_enum_string(
    value: Option<&Value>,
    allowed: &[&str],
) -> Result<Option<String>, ParseError> {
    let value = optional_string(value, 64)?;
    if value
        .as_deref()
        .is_some_and(|value| !allowed.contains(&value))
    {
        return Err(ParseError::InvalidValue);
    }
    Ok(value)
}

fn optional_external_link(value: Option<&Value>) -> Result<Option<String>, ParseError> {
    let Some(link) = optional_string(value, MAX_URL_BYTES)? else {
        return Ok(None);
    };
    let parsed = url::Url::parse(&link).map_err(|_| ParseError::InvalidValue)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(ParseError::InvalidValue);
    }
    Ok(Some(link))
}

fn parse_kind(value: Option<&Value>) -> Result<Option<ConnectionTargetKind>, ParseError> {
    match optional_string(value, 32)?.as_deref() {
        None => Ok(None),
        Some("connector") => Ok(Some(ConnectionTargetKind::Connector)),
        Some("mcp") => Ok(Some(ConnectionTargetKind::Mcp)),
        Some("plugin") => Ok(Some(ConnectionTargetKind::Plugin)),
        Some("skill") => Ok(Some(ConnectionTargetKind::Skill)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

fn parse_action(value: Option<&Value>) -> Result<Option<ConnectionTargetAction>, ParseError> {
    match optional_string(value, 32)?.as_deref() {
        None => Ok(None),
        Some("authorize") => Ok(Some(ConnectionTargetAction::Authorize)),
        Some("connect") => Ok(Some(ConnectionTargetAction::Connect)),
        Some("enable") => Ok(Some(ConnectionTargetAction::Enable)),
        Some("install") => Ok(Some(ConnectionTargetAction::Install)),
        Some("reconnect") => Ok(Some(ConnectionTargetAction::Reconnect)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

fn parse_target_state(value: Option<&Value>) -> Result<Option<ConnectionTargetState>, ParseError> {
    match optional_string(value, 32)?.as_deref() {
        None => Ok(None),
        Some("connected") => Ok(Some(ConnectionTargetState::Connected)),
        Some("expired") => Ok(Some(ConnectionTargetState::Expired)),
        Some("failed") => Ok(Some(ConnectionTargetState::Failed)),
        Some("initiated") => Ok(Some(ConnectionTargetState::Initiated)),
        Some("not_connected") => Ok(Some(ConnectionTargetState::NotConnected)),
        Some("pending") => Ok(Some(ConnectionTargetState::Pending)),
        Some("skipped") => Ok(Some(ConnectionTargetState::Skipped)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

fn parse_settle_reason(
    value: Option<&Value>,
) -> Result<Option<ConnectionSettleReason>, ParseError> {
    match optional_string(value, 32)?.as_deref() {
        None => Ok(None),
        Some("all_resolved") => Ok(Some(ConnectionSettleReason::AllResolved)),
        Some("continue") => Ok(Some(ConnectionSettleReason::Continue)),
        Some("deadline") => Ok(Some(ConnectionSettleReason::Deadline)),
        Some("interrupt") => Ok(Some(ConnectionSettleReason::Interrupt)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

fn parse_actor(value: Option<&Value>) -> Result<Option<ConnectionActor>, ParseError> {
    match optional_string(value, 32)?.as_deref() {
        None => Ok(None),
        Some("user") => Ok(Some(ConnectionActor::User)),
        Some("backend_watcher") => Ok(Some(ConnectionActor::BackendWatcher)),
        Some("clock") => Ok(Some(ConnectionActor::Clock)),
        Some(_) => Err(ParseError::InvalidValue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> Value {
        json!({
            "op_id":"op-1",
            "seq":1,
            "deadline_at":1900000000.0,
            "timeout_seconds":120.0,
            "tool_call_id":"call-1",
            "targets":[{
                "name":"linear",
                "kind":"mcp",
                "action":"install",
                "state":"pending",
                "required_env":[{
                    "name":"LINEAR_API_KEY",
                    "required":true,
                    "secret":true,
                    "default":null,
                    "prompt":"API key"
                }],
                "launch":{"kind":"stdio","target":"npx linear-mcp","env":["LINEAR_API_KEY"]},
                "detail":"Credentials are required"
            }]
        })
    }

    fn update(op_id: &str, session_id: &str, seq: u64) -> Value {
        json!({
            "op_id":op_id,
            "seq":seq,
            "deadline_at":1900000000.0,
            "settled":false,
            "settled_at":null,
            "settled_by":null,
            "owner":{"type":"session","session_id":session_id},
            "targets":[{
                "name":"linear",
                "kind":"mcp",
                "action":"install",
                "state":"connected",
                "required_env":[]
            }]
        })
    }

    #[test]
    fn parses_session_bound_mcp_request_without_retaining_answer_values() {
        let mut payload = request();
        payload["targets"][0]["env"] = json!({"LINEAR_API_KEY":"do-not-retain"});
        let operation = ConnectionOperation::parse_request(&payload, "runtime-1").unwrap();
        assert_eq!(operation.session_id, "runtime-1");
        assert_eq!(operation.op_id, "op-1");
        assert_eq!(operation.targets[0].kind, ConnectionTargetKind::Mcp);
        assert_eq!(operation.targets[0].required_env[0].name, "LINEAR_API_KEY");
        assert!(operation.targets[0].required_env[0].secret);
        assert!(!format!("{operation:?}").contains("do-not-retain"));
    }

    #[test]
    fn rejects_malformed_operation_target_and_owner() {
        let mut malformed = request();
        malformed["targets"][0]["state"] = json!("future-state");
        assert_eq!(
            ConnectionOperation::parse_request(&malformed, "runtime-1"),
            Err(ParseError::InvalidValue)
        );
        assert_eq!(
            ConnectionOperationUpdate::parse(&json!({
                "op_id":"op-1","seq":2,"deadline_at":1.0,"settled":false,
                "owner":{"type":"account"},"targets":[]
            })),
            Err(ParseError::InvalidValue)
        );
    }

    #[test]
    fn rejects_duplicate_target_or_env_names() {
        let mut duplicate_target = request();
        let first = duplicate_target["targets"][0].clone();
        duplicate_target["targets"]
            .as_array_mut()
            .unwrap()
            .push(first);
        assert_eq!(
            ConnectionOperation::parse_request(&duplicate_target, "runtime-1"),
            Err(ParseError::InvalidValue)
        );
        let mut duplicate_env = request();
        duplicate_env["targets"][0]["required_env"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"LINEAR_API_KEY","required":true,"secret":true}));
        assert_eq!(
            ConnectionOperation::parse_request(&duplicate_env, "runtime-1"),
            Err(ParseError::InvalidValue)
        );
    }

    #[test]
    fn rejects_oversized_payloads_and_collections() {
        let mut too_large = request();
        too_large["targets"][0]["instructions"] = json!("x".repeat(MAX_PAYLOAD_BYTES));
        assert_eq!(
            ConnectionOperation::parse_request(&too_large, "runtime-1"),
            Err(ParseError::PayloadTooLarge)
        );
        let too_many = json!({
            "op_id":"op-1","seq":1,"deadline_at":1.0,"timeout_seconds":1.0,
            "targets":(0..=MAX_TARGETS).map(|index| json!({
                "name":format!("mcp-{index}"),"kind":"mcp","action":"install","state":"pending"
            })).collect::<Vec<_>>()
        });
        assert_eq!(
            ConnectionOperation::parse_request(&too_many, "runtime-1"),
            Err(ParseError::TooManyItems)
        );
    }

    #[test]
    fn applies_only_newer_same_operation_same_session_snapshots() {
        let mut operation = ConnectionOperation::parse_request(&request(), "runtime-1").unwrap();
        assert_eq!(
            operation.apply_update(
                ConnectionOperationUpdate::parse(&update("other", "runtime-1", 2)).unwrap()
            ),
            UpdateApply::WrongOperation
        );
        assert_eq!(
            operation.apply_update(
                ConnectionOperationUpdate::parse(&update("op-1", "runtime-2", 2)).unwrap()
            ),
            UpdateApply::WrongSession
        );
        assert_eq!(
            operation.apply_update(
                ConnectionOperationUpdate::parse(&update("op-1", "runtime-1", 1)).unwrap()
            ),
            UpdateApply::StaleSequence
        );
        let settled = json!({
            "op_id":"op-1","seq":2,"deadline_at":1900000000.0,"settled":true,
            "settled_at":1899999990.0,"settled_by":"all_resolved",
            "owner":{"type":"session","session_id":"runtime-1"},
            "targets":[{"name":"linear","kind":"mcp","action":"install","state":"connected"}]
        });
        assert_eq!(
            operation.apply_update(ConnectionOperationUpdate::parse(&settled).unwrap()),
            UpdateApply::Applied
        );
        assert_eq!(operation.seq, 2);
        assert!(operation.settled);
        assert_eq!(
            operation.settled_by,
            Some(ConnectionSettleReason::AllResolved)
        );
        assert_eq!(operation.targets[0].state, ConnectionTargetState::Connected);
    }
}
