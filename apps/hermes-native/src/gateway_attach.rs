//! Strict boundary for the pinned `hermes gateway attach --json` local-client contract.
//!
//! This module deliberately does not open a process or a WebSocket. It only builds the
//! documented CLI arguments and validates the one-line receipt before a caller can use its
//! single-use ticket.

use serde::Deserialize;
use url::Url;
use zeroize::Zeroizing;

pub const MAX_ATTACH_RECEIPT_BYTES: usize = 64 * 1024;
const CANONICAL_PROTOCOL: &str = "hermes-gateway-v1";
const TICKET_PROTOCOL_PREFIX: &str = "hermes-gateway-ticket.";
const MAX_IDENTITY_BYTES: usize = 512;
const MAX_DETAIL_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttachMode {
    /// First local attach. May ensure/start the profile's Gateway.
    Start,
    /// Discover only; must never start a Gateway.
    Discover,
    /// Discover first and re-ensure only if the Gateway is not attachable.
    Recover,
}

/// Arguments following the `hermes` executable for the pinned public attach command.
pub fn command_args(mode: AttachMode) -> Vec<&'static str> {
    let mut args = vec!["gateway", "attach", "--json"];
    match mode {
        AttachMode::Start => args.push("--start"),
        AttachMode::Discover => {}
        AttachMode::Recover => args.push("--recover"),
    }
    args
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttachOutcome {
    Attached(AttachGrant),
    Refused(AttachRefusal),
}

#[derive(Clone, Eq, PartialEq)]
pub struct AttachGrant {
    pub endpoint: AttachEndpoint,
    pub url: String,
    pub protocol: String,
    /// The one-use credential is excluded from Debug output and zeroized on drop.
    pub ticket: Zeroizing<String>,
    pub client_code_sha: Option<String>,
    pub start_context: StartContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachRefusal {
    pub state: String,
    pub reason_code: Option<String>,
    pub detail: Option<String>,
    pub message: String,
    pub client_code_sha: Option<String>,
    pub start_context: Option<StartContext>,
}

impl std::fmt::Debug for AttachGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachGrant")
            .field("endpoint", &self.endpoint)
            .field("url", &self.url)
            .field("protocol", &self.protocol)
            .field("protocols", &"[REDACTED ticket protocol]")
            .field("ticket", &"[REDACTED]")
            .field("client_code_sha", &self.client_code_sha)
            .field("start_context", &self.start_context)
            .finish()
    }
}

impl AttachGrant {
    /// Produce the exact WebSocket subprotocol list from the parsed, validated ticket.
    /// Callers should keep this list only for the socket handshake.
    pub fn websocket_protocols(&self) -> [String; 2] {
        [
            CANONICAL_PROTOCOL.to_owned(),
            format!("{TICKET_PROTOCOL_PREFIX}{}", self.ticket.as_str()),
        ]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttachEndpoint {
    pub profile_id: String,
    pub instance_id: String,
    pub authority_epoch: u64,
    pub runtime_protocol: u32,
    pub api_origin: String,
    pub capabilities: Vec<String>,
    pub control_home: Option<String>,
    pub code_sha: Option<String>,
    pub supervisor: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartContext {
    pub windows_service_session: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAttachReceipt {
    state: String,
    reason_code: Option<String>,
    detail: Option<String>,
    endpoint: Option<AttachEndpoint>,
    client_code_sha: Option<String>,
    start_context: Option<StartContext>,
    message: Option<String>,
    url: Option<String>,
    protocol: Option<String>,
    protocols: Option<Vec<String>>,
    ticket: Option<String>,
}

#[derive(Debug)]
pub enum AttachReceiptError {
    Oversized,
    InvalidJson(String),
    InvalidField(&'static str),
    ExitCodeMismatch { expected: i32, actual: i32 },
}

impl std::fmt::Display for AttachReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Oversized => write!(
                f,
                "Hermes attach receipt exceeds the {MAX_ATTACH_RECEIPT_BYTES}-byte limit"
            ),
            Self::InvalidJson(error) => {
                write!(f, "Hermes attach receipt is not valid JSON: {error}")
            }
            Self::InvalidField(field) => write!(
                f,
                "Hermes attach receipt is missing or has invalid field `{field}`"
            ),
            Self::ExitCodeMismatch { expected, actual } => write!(
                f,
                "Hermes attach process exit code {actual} does not match receipt verdict (expected {expected})"
            ),
        }
    }
}

impl std::error::Error for AttachReceiptError {}

/// Parse one complete stdout JSON document and validate it against the pinned attach contract.
///
/// `process_exit_code` is required because a syntactically valid refusal is not a successful
/// connection. Parser-level `invalid_invocation` is the CLI's intentionally minimal refusal;
/// all normal parsed invocations include `start_context` and `client_code_sha`.
pub fn parse_attach_receipt(
    stdout: &[u8],
    process_exit_code: i32,
) -> Result<AttachOutcome, AttachReceiptError> {
    if stdout.len() > MAX_ATTACH_RECEIPT_BYTES {
        return Err(AttachReceiptError::Oversized);
    }
    let value: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|error| AttachReceiptError::InvalidJson(error.to_string()))?;
    let object = value
        .as_object()
        .ok_or(AttachReceiptError::InvalidField("root"))?;
    let state = object
        .get("state")
        .and_then(serde_json::Value::as_str)
        .ok_or(AttachReceiptError::InvalidField("state"))?;
    let invalid_invocation = state == "inaccessible"
        && object
            .get("reason_code")
            .and_then(serde_json::Value::as_str)
            == Some("invalid_invocation");
    let mut required = vec!["state", "reason_code", "endpoint"];
    if !invalid_invocation {
        required.extend(["detail", "client_code_sha", "start_context"]);
    }
    for key in required {
        if !object.contains_key(key) {
            return Err(AttachReceiptError::InvalidField(key));
        }
    }
    if let Some(endpoint) = object
        .get("endpoint")
        .and_then(serde_json::Value::as_object)
    {
        for key in [
            "profile_id",
            "instance_id",
            "authority_epoch",
            "runtime_protocol",
            "api_origin",
            "capabilities",
            "control_home",
            "code_sha",
            "supervisor",
        ] {
            if !endpoint.contains_key(key) {
                return Err(AttachReceiptError::InvalidField("endpoint"));
            }
        }
    }

    let raw: RawAttachReceipt = serde_json::from_value(value)
        .map_err(|error| AttachReceiptError::InvalidJson(error.to_string()))?;
    if !invalid_invocation && raw.start_context.is_none() {
        return Err(AttachReceiptError::InvalidField("start_context"));
    }
    if invalid_invocation && raw.endpoint.is_some() {
        return Err(AttachReceiptError::InvalidField("endpoint"));
    }
    validate_optional_text(raw.reason_code.as_deref(), "reason_code", 128)?;
    validate_optional_multiline_text(raw.detail.as_deref(), "detail", MAX_DETAIL_BYTES)?;
    validate_optional_text(raw.client_code_sha.as_deref(), "client_code_sha", 64)?;
    if let Some(sha) = raw.client_code_sha.as_deref() {
        validate_sha(sha, "client_code_sha")?;
    }

    let (expected_exit, outcome) = match raw.ticket {
        Some(ticket) => {
            let endpoint = raw
                .endpoint
                .ok_or(AttachReceiptError::InvalidField("endpoint"))?;
            if raw.message.is_some()
                || raw.url.is_none()
                || raw.protocol.is_none()
                || raw.protocols.is_none()
            {
                return Err(AttachReceiptError::InvalidField("success_shape"));
            }
            let start_context = raw
                .start_context
                .ok_or(AttachReceiptError::InvalidField("start_context"))?;
            validate_endpoint(&endpoint)?;
            if raw.state != "ready"
                && !(raw.state == "draining" && raw.reason_code.as_deref() == Some("restart_wait"))
            {
                return Err(AttachReceiptError::InvalidField("attachable_state"));
            }
            if raw.reason_code.is_some() && raw.state == "ready" {
                return Err(AttachReceiptError::InvalidField("reason_code"));
            }
            let url = raw.url.ok_or(AttachReceiptError::InvalidField("url"))?;
            validate_websocket_url(&url)?;
            let protocol = raw
                .protocol
                .ok_or(AttachReceiptError::InvalidField("protocol"))?;
            if protocol != CANONICAL_PROTOCOL {
                return Err(AttachReceiptError::InvalidField("protocol"));
            }
            validate_ticket(&ticket)?;
            let protocols = raw
                .protocols
                .ok_or(AttachReceiptError::InvalidField("protocols"))?;
            if protocols.len() != 2
                || protocols[0] != CANONICAL_PROTOCOL
                || protocols[1] != format!("{TICKET_PROTOCOL_PREFIX}{ticket}")
            {
                return Err(AttachReceiptError::InvalidField("protocols"));
            }
            let expected_url = websocket_url(&endpoint.api_origin)?;
            if url != expected_url {
                return Err(AttachReceiptError::InvalidField("url_identity"));
            }
            (
                0,
                AttachOutcome::Attached(AttachGrant {
                    endpoint,
                    url,
                    protocol,
                    ticket: Zeroizing::new(ticket),
                    client_code_sha: raw.client_code_sha,
                    start_context,
                }),
            )
        }
        None => {
            if raw.url.is_some()
                || raw.protocol.is_some()
                || raw.protocols.is_some()
                || raw.ticket.is_some()
            {
                return Err(AttachReceiptError::InvalidField("refusal_credentials"));
            }
            if let Some(endpoint) = raw.endpoint.as_ref() {
                validate_endpoint(endpoint)?;
            }
            if raw.state == "ready"
                || (raw.state == "draining" && raw.reason_code.as_deref() == Some("restart_wait"))
            {
                return Err(AttachReceiptError::InvalidField("missing_attach_ticket"));
            }
            let message = if invalid_invocation {
                raw.message
                    .unwrap_or_else(|| "gateway inaccessible: invalid_invocation".into())
            } else {
                raw.message
                    .ok_or(AttachReceiptError::InvalidField("message"))?
            };
            validate_multiline_text(&message, "message", MAX_DETAIL_BYTES)?;
            if contains_credential_marker(&message)
                || raw
                    .detail
                    .as_deref()
                    .is_some_and(contains_credential_marker)
            {
                return Err(AttachReceiptError::InvalidField(
                    "credential_bearing_refusal",
                ));
            }
            let expected_exit =
                refusal_exit_code(&raw.state, raw.reason_code.as_deref(), invalid_invocation)?;
            (
                expected_exit,
                AttachOutcome::Refused(AttachRefusal {
                    state: raw.state,
                    reason_code: raw.reason_code,
                    detail: raw.detail,
                    message,
                    client_code_sha: raw.client_code_sha,
                    start_context: raw.start_context,
                }),
            )
        }
    };
    if process_exit_code != expected_exit {
        return Err(AttachReceiptError::ExitCodeMismatch {
            expected: expected_exit,
            actual: process_exit_code,
        });
    }
    Ok(outcome)
}

fn validate_endpoint(endpoint: &AttachEndpoint) -> Result<(), AttachReceiptError> {
    validate_required_text(
        &endpoint.profile_id,
        "endpoint.profile_id",
        MAX_IDENTITY_BYTES,
    )?;
    validate_required_text(
        &endpoint.instance_id,
        "endpoint.instance_id",
        MAX_IDENTITY_BYTES,
    )?;
    if endpoint.authority_epoch == 0 || endpoint.runtime_protocol != 1 {
        return Err(AttachReceiptError::InvalidField("endpoint.identity"));
    }
    validate_api_origin(&endpoint.api_origin)?;
    if endpoint.capabilities.len() > 512 {
        return Err(AttachReceiptError::InvalidField("endpoint.capabilities"));
    }
    let mut seen = std::collections::HashSet::new();
    for capability in &endpoint.capabilities {
        validate_required_text(capability, "endpoint.capabilities[]", 128)?;
        if capability.bytes().any(|byte| byte.is_ascii_whitespace()) || !seen.insert(capability) {
            return Err(AttachReceiptError::InvalidField("endpoint.capabilities"));
        }
    }
    if let Some(path) = &endpoint.control_home {
        validate_required_text(path, "endpoint.control_home", MAX_IDENTITY_BYTES)?;
    }
    if let Some(sha) = endpoint.code_sha.as_deref() {
        validate_sha(sha, "endpoint.code_sha")?;
    }
    if !["none", "systemd", "launchd", "windows", "external"]
        .contains(&endpoint.supervisor.as_str())
    {
        return Err(AttachReceiptError::InvalidField("endpoint.supervisor"));
    }
    Ok(())
}

fn validate_api_origin(value: &str) -> Result<(), AttachReceiptError> {
    let parsed =
        Url::parse(value).map_err(|_| AttachReceiptError::InvalidField("endpoint.api_origin"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !matches!(parsed.path(), "" | "/")
        || !host_is_loopback(&parsed)
        || parsed.port_or_known_default().is_none()
    {
        return Err(AttachReceiptError::InvalidField("endpoint.api_origin"));
    }
    Ok(())
}

fn validate_websocket_url(value: &str) -> Result<(), AttachReceiptError> {
    let parsed = Url::parse(value).map_err(|_| AttachReceiptError::InvalidField("url"))?;
    if !matches!(parsed.scheme(), "ws" | "wss")
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/api/ws"
        || !host_is_loopback(&parsed)
        || parsed.port_or_known_default().is_none()
    {
        return Err(AttachReceiptError::InvalidField("url"));
    }
    Ok(())
}

fn host_is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        None => false,
    }
}

fn websocket_url(api_origin: &str) -> Result<String, AttachReceiptError> {
    let mut parsed = Url::parse(api_origin)
        .map_err(|_| AttachReceiptError::InvalidField("endpoint.api_origin"))?;
    let scheme = if parsed.scheme() == "https" {
        "wss"
    } else {
        "ws"
    };
    parsed
        .set_scheme(scheme)
        .map_err(|_| AttachReceiptError::InvalidField("endpoint.api_origin"))?;
    parsed.set_path("/api/ws");
    Ok(parsed.to_string())
}

fn validate_ticket(ticket: &str) -> Result<(), AttachReceiptError> {
    if ticket.is_empty()
        || ticket.len() > 2048
        || !ticket.bytes().all(is_http_token_byte)
        || ticket.to_ascii_lowercase().contains("bearer")
    {
        return Err(AttachReceiptError::InvalidField("ticket"));
    }
    Ok(())
}

fn is_http_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn validate_sha(value: &str, field: &'static str) -> Result<(), AttachReceiptError> {
    if !(7..=64).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AttachReceiptError::InvalidField(field));
    }
    Ok(())
}

fn validate_required_text(
    value: &str,
    field: &'static str,
    max_bytes: usize,
) -> Result<(), AttachReceiptError> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(AttachReceiptError::InvalidField(field));
    }
    Ok(())
}

fn validate_multiline_text(
    value: &str,
    field: &'static str,
    max_bytes: usize,
) -> Result<(), AttachReceiptError> {
    if value.is_empty()
        || value.len() > max_bytes
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(AttachReceiptError::InvalidField(field));
    }
    Ok(())
}

fn validate_optional_text(
    value: Option<&str>,
    field: &'static str,
    max_bytes: usize,
) -> Result<(), AttachReceiptError> {
    if let Some(value) = value {
        validate_required_text(value, field, max_bytes)?;
    }
    Ok(())
}

fn validate_optional_multiline_text(
    value: Option<&str>,
    field: &'static str,
    max_bytes: usize,
) -> Result<(), AttachReceiptError> {
    if let Some(value) = value {
        validate_multiline_text(value, field, max_bytes)?;
    }
    Ok(())
}

fn contains_credential_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains(TICKET_PROTOCOL_PREFIX) || lower.contains("bearer ") || lower.contains("token=")
}

fn refusal_exit_code(
    state: &str,
    reason_code: Option<&str>,
    invalid_invocation: bool,
) -> Result<i32, AttachReceiptError> {
    if invalid_invocation {
        return Ok(2);
    }
    if ![
        "absent",
        "starting",
        "incompatible",
        "draining",
        "inaccessible",
        "conflict",
    ]
    .contains(&state)
    {
        return Err(AttachReceiptError::InvalidField("state"));
    }
    let code = match reason_code {
        Some("deadline") => 5,
        Some(
            "authorization" | "profile_mismatch" | "gateway_elevated" | "gateway_other_session",
        ) => 4,
        _ => match state {
            "absent" | "starting" => 5,
            "incompatible" => 3,
            "draining" => 6,
            "inaccessible" | "conflict" => 7,
            _ => return Err(AttachReceiptError::InvalidField("state")),
        },
    };
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENDPOINT: &str = r#"{"profile_id":"C:\\Hermes","instance_id":"owner-1","authority_epoch":1,"runtime_protocol":1,"api_origin":"http://127.0.0.1:8765","capabilities":["session-authority-v1"],"control_home":"C:\\Hermes","code_sha":"0123456789abcdef0123456789abcdef01234567","supervisor":"windows"}"#;

    fn ready_json() -> String {
        format!(
            r#"{{"state":"ready","reason_code":null,"detail":null,"endpoint":{ENDPOINT},"client_code_sha":"0123456789abcdef0123456789abcdef01234567","start_context":{{"windows_service_session":false}},"url":"ws://127.0.0.1:8765/api/ws","protocol":"hermes-gateway-v1","protocols":["hermes-gateway-v1","hermes-gateway-ticket.abc123"],"ticket":"abc123"}}"#
        )
    }

    fn refusal_json() -> String {
        r#"{"state":"absent","reason_code":null,"detail":null,"endpoint":null,"client_code_sha":"0123456789abcdef0123456789abcdef01234567","start_context":{"windows_service_session":false},"message":"gateway absent: not ready"}"#.into()
    }

    #[test]
    fn command_arguments_cover_start_discover_and_recover() {
        assert_eq!(
            command_args(AttachMode::Start),
            ["gateway", "attach", "--json", "--start"]
        );
        assert_eq!(
            command_args(AttachMode::Discover),
            ["gateway", "attach", "--json"]
        );
        assert_eq!(
            command_args(AttachMode::Recover),
            ["gateway", "attach", "--json", "--recover"]
        );
    }

    #[test]
    fn parses_a_pinned_ready_attach_receipt() {
        let receipt = parse_attach_receipt(ready_json().as_bytes(), 0).unwrap();
        let AttachOutcome::Attached(grant) = receipt else {
            panic!("expected attached receipt")
        };
        assert_eq!(grant.endpoint.profile_id, r#"C:\Hermes"#);
        assert_eq!(grant.protocol, CANONICAL_PROTOCOL);
        assert_eq!(grant.url, "ws://127.0.0.1:8765/api/ws");
        assert_eq!(
            grant.websocket_protocols(),
            [
                "hermes-gateway-v1".to_owned(),
                "hermes-gateway-ticket.abc123".to_owned(),
            ]
        );
        assert_eq!(&*grant.ticket, "abc123");
        assert!(!format!("{grant:?}").contains("abc123"));
    }

    #[test]
    fn parses_a_normal_refusal_and_checks_exit_code() {
        let receipt = parse_attach_receipt(refusal_json().as_bytes(), 5).unwrap();
        let AttachOutcome::Refused(refusal) = receipt else {
            panic!("expected refusal")
        };
        assert_eq!(refusal.state, "absent");
        assert!(parse_attach_receipt(refusal_json().as_bytes(), 0).is_err());
    }

    #[test]
    fn accepts_the_cli_parser_minimal_invalid_invocation_refusal() {
        let json =
            br#"{"endpoint":null,"reason_code":"invalid_invocation","state":"inaccessible"}"#;
        assert!(matches!(
            parse_attach_receipt(json, 2),
            Ok(AttachOutcome::Refused(_))
        ));
    }

    #[test]
    fn rejects_malformed_oversized_and_multi_document_replies() {
        assert!(parse_attach_receipt(b"{not json", 0).is_err());
        assert!(parse_attach_receipt(format!("{} ", ready_json()).as_bytes(), 0).is_ok());
        assert!(
            parse_attach_receipt(format!("{}{}", ready_json(), ready_json()).as_bytes(), 0)
                .is_err()
        );
        assert!(matches!(
            parse_attach_receipt(&vec![b' '; MAX_ATTACH_RECEIPT_BYTES + 1], 0),
            Err(AttachReceiptError::Oversized)
        ));
    }

    #[test]
    fn rejects_identity_mismatch_ticket_mismatch_credentials_and_unknown_fields() {
        let bad_url = ready_json().replace("127.0.0.1:8765", "example.com:8765");
        assert!(parse_attach_receipt(bad_url.as_bytes(), 0).is_err());
        let bad_ticket = ready_json().replace("abc123", "bad ticket");
        assert!(parse_attach_receipt(bad_ticket.as_bytes(), 0).is_err());
        let wrong_protocol_ticket = ready_json().replace("ticket.abc123", "ticket.other");
        assert!(parse_attach_receipt(wrong_protocol_ticket.as_bytes(), 0).is_err());
        let credential_refusal = refusal_json().replace(
            "gateway absent: not ready",
            "gateway absent: bearer secret-value",
        );
        assert!(parse_attach_receipt(credential_refusal.as_bytes(), 5).is_err());
        let unknown = ready_json().replace(
            "\"ticket\":\"abc123\"",
            "\"unexpected\":true,\"ticket\":\"abc123\"",
        );
        assert!(parse_attach_receipt(unknown.as_bytes(), 0).is_err());
    }

    #[test]
    fn rejects_non_attachable_identity_and_untrusted_urls() {
        let bad_epoch = ready_json().replace("\"authority_epoch\":1", "\"authority_epoch\":0");
        assert!(parse_attach_receipt(bad_epoch.as_bytes(), 0).is_err());
        let bad_protocol = ready_json().replace("\"runtime_protocol\":1", "\"runtime_protocol\":2");
        assert!(parse_attach_receipt(bad_protocol.as_bytes(), 0).is_err());
        let bad_origin =
            ready_json().replace("http://127.0.0.1:8765", "http://user:pass@127.0.0.1:8765");
        assert!(parse_attach_receipt(bad_origin.as_bytes(), 0).is_err());
        let malformed_identity = ready_json().replace("owner-1", "");
        assert!(parse_attach_receipt(malformed_identity.as_bytes(), 0).is_err());
    }
}
