//! Credential-free remote Hermes gateway discovery for the first-run setup.

use crate::native_oauth::{NativeOAuthSession, native_token_needs_refresh, refresh_native_session};
use serde_json::Value;
use std::{
    io::Read,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};
use tungstenite::{ClientRequestBuilder, client::connect_with_config, http::Uri};
use url::Url;
use zeroize::{Zeroize, Zeroizing};

const MAX_REMOTE_STATUS_BYTES: u64 = 512 * 1024;
const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteAuthProvider {
    pub name: String,
    pub display_name: String,
    pub supports_password: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteProbeReport {
    pub base_url: String,
    pub auth_mode: RemoteAuthMode,
    pub supports_native_pkce: bool,
    pub version: Option<String>,
    pub providers: Vec<RemoteAuthProvider>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteAuthMode {
    Token,
    OAuth,
}

#[derive(Debug)]
pub struct RemoteProbeResult {
    pub request_id: String,
    pub result: Result<RemoteProbeReport, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteConnectionTestReport {
    pub base_url: String,
    pub version: Option<String>,
}

#[derive(Debug)]
pub struct RemoteConnectionTestResult {
    pub request_id: String,
    pub result: Result<RemoteConnectionTestReport, String>,
    pub oauth_session: Option<NativeOAuthSession>,
}

struct RemoteProbeRequest {
    request_id: String,
    raw_url: String,
}

struct RemoteConnectionTestRequest {
    request_id: String,
    raw_url: String,
    credential: RemoteTestCredential,
}

enum RemoteTestCredential {
    SessionToken(Zeroizing<String>),
    OAuthBearer(Zeroizing<String>),
    OAuthSession(NativeOAuthSession),
}

impl RemoteTestCredential {
    fn secret(&self) -> &str {
        match self {
            Self::SessionToken(value) | Self::OAuthBearer(value) => value,
            Self::OAuthSession(session) => &session.access_token,
        }
    }

    fn zeroize(&mut self) {
        match self {
            Self::SessionToken(value) | Self::OAuthBearer(value) => value.zeroize(),
            Self::OAuthSession(session) => {
                session.access_token.zeroize();
                session.refresh_token.zeroize();
            }
        }
    }
}

/// Runs the pinned Desktop's token-authenticated HTTP and WebSocket test off the UI thread.
pub struct RemoteConnectionTestWorker {
    requests: Sender<RemoteConnectionTestRequest>,
    results: Receiver<RemoteConnectionTestResult>,
}

impl RemoteConnectionTestWorker {
    pub fn start() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<RemoteConnectionTestRequest>();
        let (result_tx, result_rx) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-remote-connection-test".into())
            .spawn(move || {
                while let Ok(mut request) = request_rx.recv() {
                    while let Ok(newer) = request_rx.try_recv() {
                        request.credential.zeroize();
                        request = newer;
                    }
                    let result = match &mut request.credential {
                        RemoteTestCredential::OAuthSession(session)
                            if native_token_needs_refresh(session, unix_now_seconds()) =>
                        {
                            match refresh_native_session(&request.raw_url, session) {
                                Ok(refreshed) => {
                                    *session = refreshed;
                                    test_remote_connection(
                                        &request.raw_url,
                                        &RemoteTestCredential::OAuthBearer(Zeroizing::new(
                                            session.access_token.to_string(),
                                        )),
                                    )
                                }
                                Err(error) => Err(error),
                            }
                        }
                        RemoteTestCredential::OAuthSession(session) => test_remote_connection(
                            &request.raw_url,
                            &RemoteTestCredential::OAuthBearer(Zeroizing::new(
                                session.access_token.to_string(),
                            )),
                        ),
                        credential => test_remote_connection(&request.raw_url, credential),
                    };
                    let oauth_session = match &request.credential {
                        RemoteTestCredential::OAuthSession(session) => Some(session.clone()),
                        _ => None,
                    };
                    request.credential.zeroize();
                    if result_tx
                        .send(RemoteConnectionTestResult {
                            request_id: request.request_id,
                            result,
                            oauth_session,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("remote connection test worker thread must start");
        Self {
            requests: request_tx,
            results: result_rx,
        }
    }

    pub fn test_token(
        &self,
        request_id: String,
        raw_url: String,
        token: Zeroizing<String>,
    ) -> Result<(), String> {
        self.requests
            .send(RemoteConnectionTestRequest {
                request_id,
                raw_url,
                credential: RemoteTestCredential::SessionToken(token),
            })
            .map_err(|_| "Remote connection test worker has closed".to_owned())
    }

    pub fn test_oauth(
        &self,
        request_id: String,
        raw_url: String,
        session: NativeOAuthSession,
    ) -> Result<(), String> {
        self.requests
            .send(RemoteConnectionTestRequest {
                request_id,
                raw_url,
                credential: RemoteTestCredential::OAuthSession(session),
            })
            .map_err(|_| "Remote connection test worker has closed".to_owned())
    }

    pub fn poll(&self) -> Option<RemoteConnectionTestResult> {
        let mut latest = None;
        loop {
            match self.results.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

fn unix_now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

/// Serializes URL probes off the UI thread and discards queued stale targets.
pub struct RemoteProbeWorker {
    requests: Sender<RemoteProbeRequest>,
    results: Receiver<RemoteProbeResult>,
}

impl RemoteProbeWorker {
    pub fn start() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<RemoteProbeRequest>();
        let (result_tx, result_rx) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-remote-gateway-probe".into())
            .spawn(move || {
                while let Ok(mut request) = request_rx.recv() {
                    while let Ok(newer) = request_rx.try_recv() {
                        request = newer;
                    }
                    let result = probe_remote_gateway(&request.raw_url);
                    if result_tx
                        .send(RemoteProbeResult {
                            request_id: request.request_id,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("remote gateway probe worker thread must start");
        Self {
            requests: request_tx,
            results: result_rx,
        }
    }

    pub fn probe(&self, request_id: String, raw_url: String) -> Result<(), String> {
        self.requests
            .send(RemoteProbeRequest {
                request_id,
                raw_url,
            })
            .map_err(|_| "Remote gateway probe worker has closed".to_owned())
    }

    pub fn poll(&self) -> Option<RemoteProbeResult> {
        let mut latest = None;
        loop {
            match self.results.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

pub fn normalize_remote_base_url(raw_url: &str) -> Result<String, String> {
    let value = raw_url.trim();
    if value.is_empty() {
        return Err("Remote gateway URL is required".into());
    }
    let value = if value.contains("://") {
        value.to_owned()
    } else {
        format!("http://{value}")
    };
    let mut url =
        Url::parse(&value).map_err(|error| format!("Remote gateway URL is invalid: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Remote gateway URL must use http or https".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Remote gateway URL must contain a host and no embedded credentials".into());
    }
    let valid_host = match url.host() {
        Some(url::Host::Domain(host)) => {
            !host.is_empty()
                && host != "."
                && !host.starts_with('.')
                && !host.ends_with('.')
                && host
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        }
        Some(url::Host::Ipv4(_) | url::Host::Ipv6(_)) => true,
        None => false,
    };
    if !valid_host || url.port().is_some_and(|port| port == 0) {
        return Err("Remote gateway URL must contain a valid host and port".into());
    }
    url.set_query(None);
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Ok(url.to_string().trim_end_matches('/').to_owned())
}

fn probe_remote_gateway(raw_url: &str) -> Result<RemoteProbeReport, String> {
    let base_url = normalize_remote_base_url(raw_url)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REMOTE_PROBE_TIMEOUT))
        .max_redirects(0)
        .build()
        .into();
    let status = request_public_json(&agent, &format!("{base_url}/api/status"))?;
    let auth_mode = if status
        .get("auth_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        RemoteAuthMode::OAuth
    } else {
        RemoteAuthMode::Token
    };
    let providers = if auth_mode == RemoteAuthMode::OAuth {
        request_public_json(&agent, &format!("{base_url}/api/auth/providers"))
            .ok()
            .and_then(|value| value.get("providers").and_then(Value::as_array).cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|provider| {
                let name = provider.get("name")?.as_str()?.trim();
                if name.is_empty() {
                    return None;
                }
                let display_name = provider
                    .get("display_name")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or(name);
                Some(RemoteAuthProvider {
                    name: name.to_owned(),
                    display_name: display_name.to_owned(),
                    supports_password: provider
                        .get("supports_password")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(RemoteProbeReport {
        base_url,
        auth_mode,
        supports_native_pkce: status
            .get("auth_flows")
            .and_then(Value::as_array)
            .is_some_and(|flows| {
                flows
                    .iter()
                    .any(|flow| flow.as_str() == Some("native_pkce"))
            }),
        version: status
            .get("version")
            .and_then(Value::as_str)
            .filter(|version| !version.trim().is_empty())
            .map(str::to_owned),
        providers,
    })
}

fn test_remote_connection(
    raw_url: &str,
    credential: &RemoteTestCredential,
) -> Result<RemoteConnectionTestReport, String> {
    let token = credential.secret();
    if token.trim().is_empty() {
        return Err("Paste the Hermes session token before testing this connection".into());
    }
    if token.len() > 16 * 1024 || token.chars().any(char::is_control) {
        return Err("Hermes session token is invalid".into());
    }
    let base_url = normalize_remote_base_url(raw_url)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REMOTE_PROBE_TIMEOUT))
        .max_redirects(0)
        .build()
        .into();
    let mut status_request = agent
        .get(&format!("{base_url}/api/status"))
        .header("Accept", "application/json");
    status_request = match credential {
        RemoteTestCredential::SessionToken(token) => {
            status_request.header("X-Hermes-Session-Token", token.as_str())
        }
        RemoteTestCredential::OAuthBearer(token) => {
            status_request.header("Authorization", &format!("Bearer {}", token.as_str()))
        }
        RemoteTestCredential::OAuthSession(session) => status_request.header(
            "Authorization",
            &format!("Bearer {}", session.access_token.as_str()),
        ),
    };
    let status_response = status_request
        .call()
        .map_err(|error| format!("Authenticated Hermes HTTP check failed: {error}"))?;
    if !status_response.status().is_success() {
        return Err(format!(
            "Authenticated Hermes HTTP check returned {}",
            status_response.status()
        ));
    }
    let status = read_json_response(status_response.into_body())?;

    let mut ws_url =
        Url::parse(&base_url).map_err(|_| "Remote gateway URL is invalid".to_owned())?;
    ws_url
        .set_scheme(if ws_url.scheme() == "https" {
            "wss"
        } else {
            "ws"
        })
        .map_err(|_| "Remote gateway URL has an unsupported WebSocket scheme".to_owned())?;
    let prefix = ws_url.path().trim_end_matches('/');
    ws_url.set_path(&format!("{prefix}/api/ws"));
    match credential {
        RemoteTestCredential::SessionToken(token) => {
            ws_url.query_pairs_mut().append_pair("token", token);
        }
        RemoteTestCredential::OAuthBearer(token) => {
            let ticket_response = agent
                .post(&format!("{base_url}/api/auth/ws-ticket"))
                .header("Accept", "application/json")
                .header("Authorization", &format!("Bearer {}", token.as_str()))
                .send_empty()
                .map_err(|error| {
                    format!("HTTP connected, but OAuth WebSocket-ticket request failed: {error}")
                })?;
            if !ticket_response.status().is_success() {
                return Err(format!(
                    "OAuth WebSocket-ticket request returned HTTP {}",
                    ticket_response.status()
                ));
            }
            let ticket = read_json_response(ticket_response.into_body())?
                .get("ticket")
                .and_then(Value::as_str)
                .filter(|ticket| !ticket.is_empty() && ticket.len() <= 2048)
                .ok_or_else(|| "Hermes OAuth response omitted a valid WebSocket ticket".to_owned())?
                .to_owned();
            ws_url.query_pairs_mut().append_pair("ticket", &ticket);
        }
        RemoteTestCredential::OAuthSession(session) => {
            let ticket_response = agent
                .post(&format!("{base_url}/api/auth/ws-ticket"))
                .header("Accept", "application/json")
                .header(
                    "Authorization",
                    &format!("Bearer {}", session.access_token.as_str()),
                )
                .send_empty()
                .map_err(|_| {
                    "HTTP connected, but OAuth WebSocket-ticket request failed".to_owned()
                })?;
            if !ticket_response.status().is_success() {
                return Err(format!(
                    "OAuth WebSocket-ticket request returned HTTP {}",
                    ticket_response.status()
                ));
            }
            let ticket = read_json_response(ticket_response.into_body())?
                .get("ticket")
                .and_then(Value::as_str)
                .filter(|ticket| !ticket.is_empty() && ticket.len() <= 2048)
                .ok_or_else(|| "Hermes OAuth response omitted a valid WebSocket ticket".to_owned())?
                .to_owned();
            ws_url.query_pairs_mut().append_pair("ticket", &ticket);
        }
    }
    let uri: Uri = ws_url
        .as_str()
        .parse()
        .map_err(|_| "Remote gateway returned an invalid WebSocket URL".to_owned())?;
    let request = ClientRequestBuilder::new(uri);
    let (mut socket, _) = connect_with_config(request, None, 0)
        .map_err(|_| "HTTP connected, but Hermes WebSocket test failed".to_owned())?;
    let _ = socket.close(None);

    Ok(RemoteConnectionTestReport {
        base_url,
        version: status
            .get("version")
            .and_then(Value::as_str)
            .filter(|version| !version.trim().is_empty())
            .map(str::to_owned),
    })
}

fn read_json_response(body: ureq::Body) -> Result<Value, String> {
    let mut bytes = Vec::new();
    body.into_reader()
        .take(MAX_REMOTE_STATUS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read Hermes response: {error}"))?;
    if bytes.len() as u64 > MAX_REMOTE_STATUS_BYTES {
        return Err("Hermes response exceeded 512 KiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Hermes gateway returned invalid JSON".to_owned())
}

fn request_public_json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    let response = agent
        .get(url)
        .header("Accept", "application/json")
        .call()
        .map_err(|error| format!("Hermes gateway probe failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Hermes gateway probe returned HTTP {}",
            response.status()
        ));
    }
    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_REMOTE_STATUS_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("Could not read Hermes gateway probe response: {error}"))?;
    if body.len() as u64 > MAX_REMOTE_STATUS_BYTES {
        return Err("Hermes gateway probe response exceeded 512 KiB".into());
    }
    serde_json::from_slice(&body)
        .map_err(|_| "Hermes gateway probe returned invalid JSON".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
    };

    #[test]
    fn normalizes_remote_gateway_urls_like_the_pinned_desktop() {
        assert_eq!(
            normalize_remote_base_url("gateway.example.com/hermes///?ignored=1#fragment").unwrap(),
            "http://gateway.example.com/hermes"
        );
        assert_eq!(
            normalize_remote_base_url("https://gateway.example.com/hermes/").unwrap(),
            "https://gateway.example.com/hermes"
        );
    }

    #[test]
    fn rejects_unsupported_schemes_and_embedded_credentials() {
        assert!(normalize_remote_base_url("ftp://gateway.example.com").is_err());
        assert!(normalize_remote_base_url("https://user:secret@gateway.example.com").is_err());
        assert!(normalize_remote_base_url("").is_err());
    }

    #[test]
    fn probes_auth_mode_and_optional_provider_without_credentials() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local gateway fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for (expected_path, body) in [
                (
                    "/hermes/api/status",
                    r#"{"auth_required":true,"auth_flows":["native_pkce"],"version":"2026.11"}"#,
                ),
                (
                    "/hermes/api/auth/providers",
                    r#"{"providers":[{"name":"nous","display_name":"Nous Research","supports_password":false}]}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept public probe");
                let mut reader = BufReader::new(stream.try_clone().expect("clone fixture stream"));
                let mut request = String::new();
                reader.read_line(&mut request).expect("read request line");
                assert!(request.contains(expected_path), "{request}");
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).expect("read request header");
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                    requests.push(header.to_ascii_lowercase());
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write public probe response");
            }
            requests
        });

        let worker = RemoteProbeWorker::start();
        worker
            .probe(
                "probe-fixture-1".into(),
                format!("http://{address}/hermes/"),
            )
            .expect("queue remote probe");
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let result = loop {
            if let Some(result) = worker.poll() {
                break result;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "remote probe worker timed out"
            );
            thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(result.request_id, "probe-fixture-1");
        let report = result.result.expect("probe fixture");
        let requests = server.join().expect("fixture server");
        assert_eq!(report.base_url, format!("http://{address}/hermes"));
        assert_eq!(report.auth_mode, RemoteAuthMode::OAuth);
        assert!(report.supports_native_pkce);
        assert_eq!(report.version.as_deref(), Some("2026.11"));
        assert_eq!(report.providers.len(), 1);
        assert_eq!(report.providers[0].display_name, "Nous Research");
        assert!(
            requests
                .iter()
                .all(|header| !header.starts_with("authorization:"))
        );
        assert!(
            requests
                .iter()
                .all(|header| !header.starts_with("x-hermes-session-token:"))
        );
    }

    #[test]
    fn token_connection_test_checks_authenticated_http_and_websocket() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local gateway fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept status request");
            let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
            let mut request_line = String::new();
            reader
                .read_line(&mut request_line)
                .expect("read status request");
            let mut headers = String::new();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).expect("read status header");
                if header == "\r\n" || header.is_empty() {
                    break;
                }
                headers.push_str(&header.to_ascii_lowercase());
            }
            assert!(request_line.contains("/hermes/api/status"));
            assert!(headers.contains("x-hermes-session-token: fixture-secret"));
            let body = r#"{"version":"2026.11"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write status response");
            drop(stream);

            let (stream, _) = listener.accept().expect("accept websocket request");
            let websocket_observed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let observed = websocket_observed.clone();
            let mut socket = tungstenite::accept_hdr(
                stream,
                move |request: &tungstenite::handshake::server::Request,
                      response: tungstenite::handshake::server::Response| {
                    let valid = request.uri().path() == "/hermes/api/ws"
                        && request
                            .uri()
                            .query()
                            .is_some_and(|query| query == "token=fixture-secret");
                    observed.store(valid, std::sync::atomic::Ordering::SeqCst);
                    Ok(response)
                },
            )
            .expect("accept authenticated websocket probe");
            let valid = websocket_observed.load(std::sync::atomic::Ordering::SeqCst);
            let _ = socket.close(None);
            valid
        });

        let report = test_remote_connection(
            &format!("http://{address}/hermes"),
            &RemoteTestCredential::SessionToken(Zeroizing::new("fixture-secret".into())),
        )
        .expect("both authenticated transport checks pass");
        assert_eq!(report.base_url, format!("http://{address}/hermes"));
        assert_eq!(report.version.as_deref(), Some("2026.11"));
        assert!(server.join().expect("fixture server"));
    }

    #[test]
    fn token_connection_test_rejects_missing_or_malformed_secrets_before_network() {
        let empty = RemoteTestCredential::SessionToken(Zeroizing::new(" ".into()));
        let malformed = RemoteTestCredential::SessionToken(Zeroizing::new("bad\ntoken".into()));
        assert!(test_remote_connection("http://127.0.0.1:1", &empty).is_err());
        assert!(test_remote_connection("http://127.0.0.1:1", &malformed).is_err());
    }

    #[test]
    fn oauth_connection_test_mints_single_use_ticket_before_websocket() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local gateway fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let mut observed = Vec::new();
            for (path, response_body) in [
                ("/hermes/api/status", r#"{"version":"2026.11"}"#),
                (
                    "/hermes/api/auth/ws-ticket",
                    r#"{"ticket":"single-use-ticket"}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept oauth HTTP request");
                let mut reader = BufReader::new(stream.try_clone().expect("clone fixture stream"));
                let mut request_line = String::new();
                reader
                    .read_line(&mut request_line)
                    .expect("read request line");
                assert!(request_line.contains(path), "{request_line}");
                let mut headers = String::new();
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).expect("read request header");
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                    headers.push_str(&header.to_ascii_lowercase());
                }
                assert!(headers.contains("authorization: bearer access-fixture"));
                observed.push(request_line);
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response_body.len(), response_body).expect("write oauth response");
            }
            let (stream, _) = listener.accept().expect("accept oauth websocket");
            let was_valid = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let check = was_valid.clone();
            let mut socket = tungstenite::accept_hdr(
                stream,
                move |request: &tungstenite::handshake::server::Request,
                      response: tungstenite::handshake::server::Response| {
                    check.store(
                        request.uri().path() == "/hermes/api/ws"
                            && request.uri().query() == Some("ticket=single-use-ticket"),
                        std::sync::atomic::Ordering::SeqCst,
                    );
                    Ok(response)
                },
            )
            .expect("accept ticket-authenticated websocket");
            let valid = was_valid.load(std::sync::atomic::Ordering::SeqCst);
            let _ = socket.close(None);
            (observed, valid)
        });

        let report = test_remote_connection(
            &format!("http://{address}/hermes"),
            &RemoteTestCredential::OAuthBearer(Zeroizing::new("access-fixture".into())),
        )
        .expect("oauth HTTP, ticket and WebSocket checks pass");
        assert_eq!(report.version.as_deref(), Some("2026.11"));
        let (http_requests, websocket_ok) = server.join().expect("fixture server");
        assert_eq!(http_requests.len(), 2);
        assert!(websocket_ok);
    }

    #[test]
    fn oauth_worker_refreshes_expiring_session_before_test_and_returns_rotation() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind gateway fixture");
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut paths_and_auth = Vec::new();
            for (expected_path, response_body) in [
                (
                    "/hermes/auth/native/refresh",
                    r#"{"access_token":"access-after","refresh_token":"refresh-after","expires_at":2000000000,"provider":"nous","user_id":"user-1"}"#,
                ),
                ("/hermes/api/status", r#"{"version":"2026.11"}"#),
                (
                    "/hermes/api/auth/ws-ticket",
                    r#"{"ticket":"rotated-session-ticket"}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept OAuth request");
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                assert!(request_line.contains(expected_path), "{request_line}");
                let mut headers = String::new();
                let mut body_len = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                    let lower = header.to_ascii_lowercase();
                    if let Some(length) = lower.strip_prefix("content-length: ") {
                        body_len = length.trim().parse::<usize>().unwrap();
                    }
                    headers.push_str(&lower);
                }
                if expected_path.ends_with("/refresh") {
                    let mut body = vec![0; body_len];
                    reader.read_exact(&mut body).unwrap();
                    let request: Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(request["refresh_token"], "refresh-before");
                    assert_eq!(request["provider"], "nous");
                } else {
                    assert!(headers.contains("authorization: bearer access-after"));
                }
                paths_and_auth.push((request_line, headers));
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response_body.len(), response_body).unwrap();
            }
            let (stream, _) = listener.accept().expect("accept OAuth websocket");
            let mut socket = tungstenite::accept_hdr(
                stream,
                |request: &tungstenite::handshake::server::Request,
                 response: tungstenite::handshake::server::Response| {
                    assert_eq!(request.uri().path(), "/hermes/api/ws");
                    assert_eq!(request.uri().query(), Some("ticket=rotated-session-ticket"));
                    Ok(response)
                },
            )
            .unwrap();
            let _ = socket.close(None);
            paths_and_auth
        });

        let worker = RemoteConnectionTestWorker::start();
        worker
            .test_oauth(
                "refresh-fixture".into(),
                format!("http://{address}/hermes"),
                NativeOAuthSession {
                    access_token: Zeroizing::new("access-before".into()),
                    refresh_token: Zeroizing::new("refresh-before".into()),
                    expires_at: 1,
                    provider: "nous".into(),
                    user_id: "user-1".into(),
                },
            )
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let result = loop {
            if let Some(result) = worker.poll() {
                break result;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "remote test worker timed out"
            );
            thread::sleep(Duration::from_millis(10));
        };
        let report = result.result.expect("refreshed connection test succeeds");
        assert_eq!(report.version.as_deref(), Some("2026.11"));
        let session = result.oauth_session.expect("rotated OAuth session");
        assert_eq!(&*session.access_token, "access-after");
        assert_eq!(&*session.refresh_token, "refresh-after");
        let calls = server.join().expect("gateway fixture");
        assert_eq!(calls.len(), 3);
        assert!(calls[0].0.contains("/auth/native/refresh"));
        assert!(calls[1].1.contains("authorization: bearer access-after"));
        assert!(calls[2].1.contains("authorization: bearer access-after"));
    }
}
