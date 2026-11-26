use base64::Engine;
use serde_json::{Map, Value};
use std::fmt;
use zeroize::{Zeroize, Zeroizing};

const MAX_IMPORT_BYTES: usize = 256 * 1024;
const MAX_ENTRIES: usize = 64;
const MAX_NAME_BYTES: usize = 128;
const MAX_TOKENS: usize = 512;
const CURSOR_PREFIX: &str = "cursor://anysphere.cursor-deeplink/mcp/install?";

/// MCP setup data prepared for a review-before-save workflow. Configuration
/// values may contain secrets, so Debug and summary output are redacted.
#[derive(PartialEq)]
pub struct McpImportEntry {
    pub name: String,
    pub config: Value,
}

impl fmt::Debug for McpImportEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpImportEntry")
            .field("name", &self.name)
            .field("config", &"[redacted]")
            .finish()
    }
}

impl McpImportEntry {
    /// Preview-safe label containing only the name and transport kind.
    pub fn summary(&self) -> String {
        let transport = self
            .config
            .get("transport")
            .and_then(Value::as_str)
            .or_else(|| self.config.get("url").map(|_| "http"))
            .unwrap_or("stdio");
        format!("{} · {}", self.name, transport)
    }

    /// Zeroize config string values and remove them after apply or cancel.
    pub fn clear_sensitive(&mut self) {
        zeroize_value(&mut self.config);
        self.config = Value::Null;
    }
}

impl Drop for McpImportEntry {
    fn drop(&mut self) {
        self.clear_sensitive();
    }
}

fn zeroize_value(value: &mut Value) {
    match value {
        Value::String(value) => value.zeroize(),
        Value::Array(values) => {
            for value in values.iter_mut() {
                zeroize_value(value);
            }
            values.clear();
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                zeroize_value(value);
            }
            values.clear();
        }
        _ => {}
    }
}

/// Owns an intermediate JSON tree containing imported settings. The tree is
/// zeroized on every return path unless ownership is explicitly transferred
/// into an `McpImportEntry`.
struct SensitiveJsonValue(Value);

impl Drop for SensitiveJsonValue {
    fn drop(&mut self) {
        zeroize_value(&mut self.0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpImportError {
    EmptyInput,
    InputTooLarge,
    InvalidText,
    InvalidJson,
    InvalidConfiguration,
    TooManyEntries,
    UnrecognizedInput,
}

impl fmt::Display for McpImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyInput => "Paste an MCP server configuration or setup command",
            Self::InputTooLarge => "MCP import is larger than the 256 KiB limit",
            Self::InvalidText => "MCP import contains unsupported control characters",
            Self::InvalidJson => "MCP import contains invalid JSON",
            Self::InvalidConfiguration => "MCP import contains an invalid server configuration",
            Self::TooManyEntries => "MCP import contains more than 64 server entries",
            Self::UnrecognizedInput => "Could not recognize an MCP server configuration",
        })
    }
}

impl std::error::Error for McpImportError {}

/// Parse JSON MCP config, setup commands, HTTP URLs, or Cursor install links.
/// Does not launch commands, access the network, or print imported values.
pub fn parse_mcp_import(text: &str) -> Result<Vec<McpImportEntry>, McpImportError> {
    if text.trim().is_empty() {
        return Err(McpImportError::EmptyInput);
    }
    if text.len() > MAX_IMPORT_BYTES {
        return Err(McpImportError::InputTooLarge);
    }
    if text
        .chars()
        .any(|ch| ch == '\0' || (ch.is_control() && !matches!(ch, '\n' | '\r' | '\t')))
    {
        return Err(McpImportError::InvalidText);
    }

    let trimmed = text.trim();
    if trimmed
        .get(..CURSOR_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(CURSOR_PREFIX))
    {
        return parse_cursor_link(trimmed);
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return parse_json(trimmed);
    }

    let folded = text.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut entries = Vec::new();
    for line in folded
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some(entry) = parse_setup_line(line)? {
            entries.push(entry);
            if entries.len() > MAX_ENTRIES {
                return Err(McpImportError::TooManyEntries);
            }
        }
    }
    if entries.is_empty() {
        Err(McpImportError::UnrecognizedInput)
    } else {
        Ok(entries)
    }
}

fn parse_json(text: &str) -> Result<Vec<McpImportEntry>, McpImportError> {
    let value =
        SensitiveJsonValue(serde_json::from_str(text).map_err(|_| McpImportError::InvalidJson)?);
    let object = value
        .0
        .as_object()
        .ok_or(McpImportError::InvalidConfiguration)?;
    if is_server_shape(object) {
        let config = normalize_and_validate(object)?;
        let name = infer_name(&config);
        return Ok(vec![McpImportEntry { name, config }]);
    }

    let entries = object
        .get("mcpServers")
        .or_else(|| object.get("mcp_servers"))
        .and_then(Value::as_object)
        .unwrap_or(object);
    if entries.is_empty() || entries.len() > MAX_ENTRIES {
        return Err(if entries.len() > MAX_ENTRIES {
            McpImportError::TooManyEntries
        } else {
            McpImportError::InvalidConfiguration
        });
    }

    let mut parsed = Vec::with_capacity(entries.len());
    for (name, config) in entries {
        let config = config
            .as_object()
            .filter(|config| is_server_shape(config))
            .ok_or(McpImportError::InvalidConfiguration)?;
        parsed.push(McpImportEntry {
            name: validate_name(name)?,
            config: normalize_and_validate(config)?,
        });
    }
    Ok(parsed)
}

fn is_server_shape(config: &Map<String, Value>) -> bool {
    config
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(valid_text)
        || config
            .get("url")
            .and_then(Value::as_str)
            .is_some_and(is_http_url)
}

fn normalize_and_validate(config: &Map<String, Value>) -> Result<Value, McpImportError> {
    let mut normalized = SensitiveJsonValue(Value::Object(config.clone()));
    let normalized = normalized
        .0
        .as_object_mut()
        .expect("normalized MCP configuration is an object");
    if !normalized.contains_key("transport") {
        if let Some(kind) = normalized.remove("type") {
            if kind
                .as_str()
                .is_some_and(|value| matches!(value, "stdio" | "http" | "sse"))
            {
                normalized.insert("transport".into(), kind);
            } else {
                return Err(McpImportError::InvalidConfiguration);
            }
        }
    } else {
        normalized.remove("type");
    }
    if let Some(command) = normalized.get("command") {
        if !command.as_str().is_some_and(valid_text) {
            return Err(McpImportError::InvalidConfiguration);
        }
    }
    if let Some(url) = normalized.get("url") {
        if !url.as_str().is_some_and(is_http_url) {
            return Err(McpImportError::InvalidConfiguration);
        }
    }
    if !normalized.contains_key("command") && !normalized.contains_key("url") {
        return Err(McpImportError::InvalidConfiguration);
    }
    if let Some(args) = normalized.get("args") {
        if !args.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|item| item.as_str().is_some_and(valid_text))
        }) {
            return Err(McpImportError::InvalidConfiguration);
        }
    }
    for field in ["env", "headers"] {
        if let Some(values) = normalized.get(field) {
            let values = values
                .as_object()
                .ok_or(McpImportError::InvalidConfiguration)?;
            if values.iter().any(|(key, value)| {
                key.is_empty()
                    || key.chars().any(char::is_control)
                    || !value
                        .as_str()
                        .is_some_and(|v| !v.chars().any(char::is_control))
            }) {
                return Err(McpImportError::InvalidConfiguration);
            }
        }
    }
    if let Some(transport) = normalized.get("transport") {
        if !transport
            .as_str()
            .is_some_and(|value| matches!(value, "stdio" | "http" | "sse"))
        {
            return Err(McpImportError::InvalidConfiguration);
        }
    }
    Ok(Value::Object(std::mem::take(normalized)))
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn validate_name(name: &str) -> Result<String, McpImportError> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control) {
        return Err(McpImportError::InvalidConfiguration);
    }
    Ok(name.into())
}

fn is_http_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || authority.chars().any(char::is_whitespace)
    {
        return false;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| !host.is_empty())
        && url.username().is_empty()
        && url.password().is_none()
}
fn parse_setup_line(line: &str) -> Result<Option<McpImportEntry>, McpImportError> {
    if is_http_url(line) {
        return Ok(Some(McpImportEntry {
            name: name_from_url(line),
            config: serde_json::json!({"url": line}),
        }));
    }
    let Some(tokens) = tokenize(line) else {
        return Err(McpImportError::InvalidConfiguration);
    };
    if tokens.is_empty() || tokens.len() > MAX_TOKENS {
        return Err(McpImportError::InvalidConfiguration);
    }
    if tokens.len() >= 3 && tokens[0] == "claude" && tokens[1] == "mcp" && tokens[2] == "add" {
        return parse_claude_add(&tokens).map(Some);
    }
    if tokens.len() >= 3 && tokens[0] == "hermes" && tokens[1] == "mcp" && tokens[2] == "add" {
        return parse_hermes_add(&tokens).map(Some);
    }
    if matches!(
        tokens[0].as_str(),
        "npx" | "bunx" | "uvx" | "node" | "docker"
    ) {
        return Ok(Some(McpImportEntry {
            name: infer_command_name(&tokens[0], &tokens[1..]),
            config: serde_json::json!({"command":tokens[0],"args":tokens[1..]}),
        }));
    }
    Ok(None)
}

fn parse_claude_add(tokens: &[String]) -> Result<McpImportEntry, McpImportError> {
    let mut name = None;
    let mut transport = None;
    let mut env = Map::new();
    let mut headers = Map::new();
    let mut rest: Option<Vec<String>> = None;
    let mut i = 3;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "--" => {
                rest = Some(tokens[i + 1..].to_vec());
                break;
            }
            "--transport" | "-t" => {
                i += 1;
                transport = Some(
                    tokens
                        .get(i)
                        .ok_or(McpImportError::InvalidConfiguration)?
                        .clone(),
                );
            }
            "--env" | "-e" => {
                i += 1;
                let (key, value) = split_pair(
                    tokens.get(i).ok_or(McpImportError::InvalidConfiguration)?,
                    '=',
                )?;
                env.insert(key, Value::String(value));
            }
            "--header" | "-H" => {
                i += 1;
                let (key, value) = split_pair(
                    tokens.get(i).ok_or(McpImportError::InvalidConfiguration)?,
                    ':',
                )?;
                headers.insert(key, Value::String(value.trim().into()));
            }
            "--scope" | "-s" => i += 1,
            flag if flag.starts_with('-') => {}
            positional if name.is_none() => name = Some(positional.to_owned()),
            _ => {
                rest = Some(tokens[i..].to_vec());
                break;
            }
        }
        i += 1;
    }

    let name = validate_name(
        name.as_deref()
            .ok_or(McpImportError::InvalidConfiguration)?,
    )?;
    let rest = rest.ok_or(McpImportError::InvalidConfiguration)?;
    if rest.is_empty() {
        return Err(McpImportError::InvalidConfiguration);
    }
    let mut config = Map::new();
    if rest.len() == 1 && is_http_url(&rest[0]) {
        config.insert("url".into(), Value::String(rest[0].clone()));
        if let Some(transport) = transport {
            config.insert("transport".into(), Value::String(transport));
        }
        if !headers.is_empty() {
            config.insert("headers".into(), Value::Object(headers));
        }
    } else {
        config.insert("command".into(), Value::String(rest[0].clone()));
        config.insert(
            "args".into(),
            Value::Array(rest[1..].iter().cloned().map(Value::String).collect()),
        );
        if !env.is_empty() {
            config.insert("env".into(), Value::Object(env));
        }
    }
    Ok(McpImportEntry {
        name,
        config: normalize_and_validate(&config)?,
    })
}

fn parse_hermes_add(tokens: &[String]) -> Result<McpImportEntry, McpImportError> {
    let mut name = None;
    let mut command = None;
    let mut url = None;
    let mut auth = None;
    let mut env = Map::new();
    let mut args = Vec::new();
    let mut i = 3;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "--args" => {
                args.extend(tokens[i + 1..].iter().cloned());
                if args.first().is_some_and(|arg| arg == "--") {
                    args.remove(0);
                }
                break;
            }
            "--url" | "--command" | "--auth" => {
                let flag = tokens[i].as_str();
                i += 1;
                let value = tokens
                    .get(i)
                    .ok_or(McpImportError::InvalidConfiguration)?
                    .clone();
                match flag {
                    "--url" => url = Some(value),
                    "--command" => command = Some(value),
                    _ => auth = Some(value),
                }
            }
            "--env" => {
                i += 1;
                while i < tokens.len() && !tokens[i].starts_with('-') {
                    let (key, value) = split_pair(&tokens[i], '=')?;
                    env.insert(key, Value::String(value));
                    i += 1;
                }
                continue;
            }
            "--connect-timeout" | "--preset" => {
                i += 1;
                if i >= tokens.len() {
                    return Err(McpImportError::InvalidConfiguration);
                }
            }
            flag if flag.starts_with('-') => {}
            positional if name.is_none() => name = Some(positional.to_owned()),
            _ => {}
        }
        i += 1;
    }
    let name = validate_name(
        name.as_deref()
            .ok_or(McpImportError::InvalidConfiguration)?,
    )?;
    let mut config = Map::new();
    if let Some(url) = url {
        config.insert("url".into(), Value::String(url));
    }
    if let Some(command) = command {
        config.insert("command".into(), Value::String(command));
    }
    if !args.is_empty() {
        config.insert(
            "args".into(),
            Value::Array(args.into_iter().map(Value::String).collect()),
        );
    }
    if !env.is_empty() {
        config.insert("env".into(), Value::Object(env));
    }
    if let Some(auth) = auth {
        config.insert("auth".into(), Value::String(auth));
    }
    Ok(McpImportEntry {
        name,
        config: normalize_and_validate(&config)?,
    })
}

fn split_pair(pair: &str, delimiter: char) -> Result<(String, String), McpImportError> {
    let (key, value) = pair
        .split_once(delimiter)
        .ok_or(McpImportError::InvalidConfiguration)?;
    let key = key.trim();
    if key.is_empty() || key.chars().any(char::is_control) {
        return Err(McpImportError::InvalidConfiguration);
    }
    Ok((key.into(), value.into()))
}

fn tokenize(line: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut saw_quote = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else if ch == '\\' && active == '"' {
                current.push(chars.next()?);
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
            saw_quote = true;
        } else if ch.is_whitespace() {
            if !current.is_empty() || saw_quote {
                tokens.push(std::mem::take(&mut current));
                saw_quote = false;
                if tokens.len() > MAX_TOKENS {
                    return None;
                }
            }
        } else {
            current.push(ch);
        }
    }
    if quote.is_some() {
        return None;
    }
    if !current.is_empty() || saw_quote {
        tokens.push(current);
    }
    (tokens.len() <= MAX_TOKENS).then_some(tokens)
}

fn infer_name(config: &Value) -> String {
    if let Some(url) = config.get("url").and_then(Value::as_str) {
        return name_from_url(url);
    }
    let command = config
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("server");
    let args = config
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    infer_command_name(command, &args)
}
fn infer_command_name(command: &str, args: &[String]) -> String {
    if command == "docker" {
        let mut i = usize::from(args.first().is_some_and(|arg| arg == "run"));
        let value_flags = [
            "--entrypoint",
            "--env",
            "--label",
            "--mount",
            "--name",
            "--network",
            "--platform",
            "--publish",
            "--pull",
            "--user",
            "--volume",
            "--workdir",
            "-e",
            "-l",
            "-p",
            "-u",
            "-v",
            "-w",
        ];
        while i < args.len() {
            let arg = &args[i];
            if arg.starts_with('-') {
                i += if value_flags.contains(&arg.as_str()) {
                    2
                } else {
                    1
                };
                continue;
            }
            return cleanup_name(
                arg.rsplit('/')
                    .next()
                    .unwrap_or(arg)
                    .split(':')
                    .next()
                    .unwrap_or(arg),
            );
        }
        return "docker".into();
    }

    let candidate = if command == "node" {
        args.iter().find(|arg| !arg.starts_with('-'))
    } else {
        let mut i = 0;
        while i < args.len() {
            if args[i].starts_with('-') {
                if matches!(args[i].as_str(), "--from" | "--package" | "-p") {
                    i += 1;
                }
            } else {
                return package_name(&args[i]);
            }
            i += 1;
        }
        None
    };
    candidate
        .map(|arg| {
            if command == "node" {
                cleanup_name(arg.rsplit(['/', '\\']).next().unwrap_or(arg))
            } else {
                package_name(arg)
            }
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| command.into())
}

fn package_name(spec: &str) -> String {
    let mut bare = spec.split("==").next().unwrap_or(spec);
    if let Some(at) = bare.rfind('@').filter(|index| *index > 0) {
        bare = &bare[..at];
    }
    cleanup_name(bare.rsplit('/').next().unwrap_or(bare))
}

fn cleanup_name(raw: &str) -> String {
    let mut name = raw.trim_end_matches(['.', '/', '\\']).to_owned();
    for extension in [".cjs", ".js", ".mjs", ".py", ".ts"] {
        if name.to_ascii_lowercase().ends_with(extension) {
            name.truncate(name.len() - extension.len());
            break;
        }
    }
    for prefix in ["mcp-server-", "server-", "mcp-"] {
        if name.starts_with(prefix) {
            name.drain(..prefix.len());
            break;
        }
    }
    for suffix in ["-mcp-server", "-mcp", "-server"] {
        if name.ends_with(suffix) {
            name.truncate(name.len() - suffix.len());
            break;
        }
    }
    if name.is_empty() {
        "server".into()
    } else {
        name
    }
}

fn name_from_url(raw: &str) -> String {
    let host = url::Url::parse(raw)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_default();
    let mut labels = host
        .trim_matches(['[', ']'])
        .split('.')
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    while labels.len() > 1 && matches!(labels[0], "api" | "mcp" | "www") {
        labels.remove(0);
    }
    labels
        .first()
        .map(|value| (*value).to_owned())
        .unwrap_or_else(|| "server".into())
}
fn parse_cursor_link(link: &str) -> Result<Vec<McpImportEntry>, McpImportError> {
    let query = link
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or(McpImportError::InvalidConfiguration)?;
    let mut name = None;
    let mut encoded = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match percent_decode(key).as_deref() {
            Some("name") => name = percent_decode(value),
            Some("config") => encoded = percent_decode(value),
            _ => {}
        }
    }
    let name = validate_name(
        name.as_deref()
            .ok_or(McpImportError::InvalidConfiguration)?,
    )?;
    let encoded = encoded.ok_or(McpImportError::InvalidConfiguration)?;
    let bytes = Zeroizing::new(
        base64::engine::general_purpose::STANDARD
            .decode(encoded.as_bytes())
            .or_else(|_| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded.as_bytes())
            })
            .map_err(|_| McpImportError::InvalidConfiguration)?,
    );
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(McpImportError::InputTooLarge);
    }
    let value = SensitiveJsonValue(
        serde_json::from_slice(&bytes).map_err(|_| McpImportError::InvalidConfiguration)?,
    );
    let config = value
        .0
        .as_object()
        .filter(|config| is_server_shape(config))
        .ok_or(McpImportError::InvalidConfiguration)?;
    Ok(vec![McpImportEntry {
        name,
        config: normalize_and_validate(config)?,
    }])
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                let hi = *bytes.get(index + 1)?;
                let lo = *bytes.get(index + 2)?;
                decoded.push(
                    hex_value(hi)?
                        .checked_mul(16)?
                        .checked_add(hex_value(lo)?)?,
                );
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<McpImportEntry> {
        parse_mcp_import(text).expect("valid test import")
    }

    #[test]
    fn parses_wrapped_and_bare_json_maps() {
        let wrapped = parse(
            r#"{"mcpServers":{"fetch":{"command":"uvx","args":["mcp-server-fetch"]},"linear":{"url":"https://mcp.linear.app/sse"}}}"#,
        );
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].name, "fetch");
        assert_eq!(wrapped[1].name, "linear");
        assert_eq!(wrapped[1].config["url"], "https://mcp.linear.app/sse");
        let bare = parse(r#"{"context7":{"type":"http","url":"https://mcp.context7.com/mcp"}}"#);
        assert_eq!(bare[0].config["transport"], "http");
        assert!(!bare[0].config.get("type").is_some());
    }

    #[test]
    fn infers_name_from_single_server_object() {
        let parsed =
            parse(r#"{"command":"npx","args":["-y","@modelcontextprotocol/server-github"]}"#);
        assert_eq!(parsed[0].name, "github");
    }

    #[test]
    fn rejects_invalid_oversized_and_unrecognized_input() {
        assert_eq!(
            parse_mcp_import("").unwrap_err(),
            McpImportError::EmptyInput
        );
        assert_eq!(
            parse_mcp_import(r#"{"mcpServers":{"x":{"args":[]}}}"#).unwrap_err(),
            McpImportError::InvalidConfiguration
        );
        assert_eq!(
            parse_mcp_import("rm -rf /").unwrap_err(),
            McpImportError::UnrecognizedInput
        );
        assert_eq!(
            parse_mcp_import(&"x".repeat(MAX_IMPORT_BYTES + 1)).unwrap_err(),
            McpImportError::InputTooLarge
        );
    }

    #[test]
    fn parses_quoted_npx_uvx_and_shell_continuation() {
        let npx = parse(r#"npx -y @modelcontextprotocol/server-filesystem "/My Files/docs""#);
        assert_eq!(npx[0].name, "filesystem");
        assert_eq!(npx[0].config["args"][2], "/My Files/docs");
        assert_eq!(parse("uvx mcp-server-fetch")[0].name, "fetch");
        assert_eq!(parse("npx -y \\\n @scope/server-test")[0].name, "test");
    }

    #[test]
    fn parses_bunx_node_docker_and_multiple_commands() {
        let entries = parse(
            "bunx @scope/server-memory@latest\nnode ./servers/weather.js --port 3010\ndocker run -i --rm -e GITHUB_TOKEN=YOUR_KEY mcp/github",
        );
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "memory");
        assert_eq!(entries[1].name, "weather");
        assert_eq!(entries[2].name, "github");
        assert_eq!(entries[2].config["args"][4], "GITHUB_TOKEN=YOUR_KEY");
    }

    #[test]
    fn parses_claude_stdio_http_and_headers() {
        let stdio = parse(
            "claude mcp add airtable -e AIRTABLE_API_KEY=YOUR_KEY -- npx -y airtable-mcp-server",
        );
        assert_eq!(stdio[0].name, "airtable");
        assert_eq!(stdio[0].config["env"]["AIRTABLE_API_KEY"], "YOUR_KEY");
        let http = parse("claude mcp add --transport http context7 https://mcp.context7.com/mcp");
        assert_eq!(http[0].config["transport"], "http");
        let header = parse(
            r#"claude mcp add --transport sse api -H "Authorization: Bearer TOKEN_HERE" https://api.example.com/sse"#,
        );
        assert_eq!(
            header[0].config["headers"]["Authorization"],
            "Bearer TOKEN_HERE"
        );
    }

    #[test]
    fn parses_hermes_flags_and_bare_urls() {
        let entry = parse(
            "hermes mcp add fetch --command uvx --env TOKEN=YOUR_KEY --args mcp-server-fetch",
        );
        assert_eq!(entry[0].config["command"], "uvx");
        assert_eq!(entry[0].config["args"][0], "mcp-server-fetch");
        assert_eq!(entry[0].config["env"]["TOKEN"], "YOUR_KEY");
        assert_eq!(parse("https://mcp.linear.app/sse")[0].name, "linear");
    }

    #[test]
    fn cursor_link_values_are_redacted_and_clearable() {
        let secret = "top-secret-header";
        let config =
            serde_json::json!({"url":"https://example.com","headers":{"Authorization":secret}});
        let encoded = base64::engine::general_purpose::STANDARD.encode(config.to_string());
        let link = format!("{CURSOR_PREFIX}name=example&config={encoded}");
        let mut entry = parse(&link).remove(0);
        assert!(!format!("{entry:?}").contains(secret));
        assert_eq!(entry.summary(), "example · http");
        entry.clear_sensitive();
        assert_eq!(entry.config, Value::Null);
    }

    #[test]
    fn rejects_bad_quote_and_bad_url_without_echoing_input() {
        assert_eq!(
            parse_mcp_import(r#"npx package "unterminated"#).unwrap_err(),
            McpImportError::InvalidConfiguration
        );
        assert_eq!(
            parse_mcp_import("ftp://example.com/thing").unwrap_err(),
            McpImportError::UnrecognizedInput
        );
        assert_eq!(
            parse_mcp_import("http:///missing-host").unwrap_err(),
            McpImportError::UnrecognizedInput
        );
        assert_eq!(
            McpImportError::InvalidConfiguration
                .to_string()
                .contains("TOKEN"),
            false
        );
    }
}
