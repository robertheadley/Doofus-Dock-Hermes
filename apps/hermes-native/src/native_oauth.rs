//! RFC 8252 loopback plus PKCE login for OAuth-capable Hermes gateways.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use url::Url;
use zeroize::{Zeroize, Zeroizing};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const TOKEN_POST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RESPONSE_BYTES: u64 = 512 * 1024;
const TOKEN_REFRESH_SKEW_SECS: i64 = 60;

#[derive(Clone)]
pub struct NativeOAuthSession {
    pub access_token: Zeroizing<String>,
    pub refresh_token: Zeroizing<String>,
    pub expires_at: i64,
    pub provider: String,
    pub user_id: String,
}

pub fn native_token_needs_refresh(session: &NativeOAuthSession, now_unix_secs: i64) -> bool {
    session.expires_at <= 0 || now_unix_secs >= session.expires_at - TOKEN_REFRESH_SKEW_SECS
}

/// Rotate a native OAuth session through the pinned gateway broker endpoint.
/// The refresh token stays in memory and is never placed in URLs or diagnostics.
pub fn refresh_native_session(
    base_url: &str,
    session: &NativeOAuthSession,
) -> Result<NativeOAuthSession, String> {
    if session.refresh_token.is_empty() {
        return Err("Hermes OAuth session has no refresh token; sign in again".into());
    }
    let normalized = crate::remote_probe::normalize_remote_base_url(base_url)?;
    let mut base = Url::parse(&format!("{normalized}/"))
        .map_err(|_| "Remote gateway URL is invalid".to_owned())?;
    base.set_path(&format!("{}auth/native/refresh", base.path()));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TOKEN_POST_TIMEOUT))
        .max_redirects(0)
        .build()
        .into();
    let response = agent
        .post(base.as_str())
        .header("Accept", "application/json")
        .send_json(json!({
            "refresh_token": session.refresh_token.as_str(),
            "provider": session.provider,
        }))
        .map_err(|_| "Hermes OAuth token refresh failed".to_owned())?;
    parse_token_response(response.into_body().into_reader())
}

fn parse_token_response(body: impl Read) -> Result<NativeOAuthSession, String> {
    let mut bytes = Vec::new();
    body.take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes OAuth token response".to_owned())?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        bytes.zeroize();
        return Err("Hermes OAuth token response exceeded 512 KiB".into());
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "Hermes OAuth token response was invalid JSON".to_owned())?;
    bytes.zeroize();
    session_from_value(&value)
}

fn session_from_value(value: &Value) -> Result<NativeOAuthSession, String> {
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Hermes OAuth token response omitted access_token".to_owned())?;
    Ok(NativeOAuthSession {
        access_token: Zeroizing::new(access_token.to_owned()),
        refresh_token: Zeroizing::new(
            value
                .get("refresh_token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ),
        expires_at: value.get("expires_at").and_then(Value::as_i64).unwrap_or(0),
        provider: value
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        user_id: value
            .get("user_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    })
}

impl fmt::Debug for NativeOAuthSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeOAuthSession")
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("provider", &self.provider)
            .field("user_id", &self.user_id)
            .finish()
    }
}

#[derive(Debug)]
pub struct NativeOAuthLoginResult {
    pub request_id: String,
    pub result: Result<NativeOAuthSession, String>,
}

struct LoginRequest {
    request_id: String,
    base_url: String,
    provider: Option<String>,
}

/// One-at-a-time RFC 8252 login worker. Results are fenced by request ID in the UI.
pub struct NativeOAuthLoginWorker {
    requests: Sender<LoginRequest>,
    results: Receiver<NativeOAuthLoginResult>,
}

impl NativeOAuthLoginWorker {
    pub fn start() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<LoginRequest>();
        let (result_tx, result_rx) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-native-oauth-login".into())
            .spawn(move || {
                while let Ok(mut request) = request_rx.recv() {
                    while let Ok(newer) = request_rx.try_recv() {
                        request.base_url.zeroize();
                        request = newer;
                    }
                    let result = run_native_login(
                        &request.base_url,
                        request.provider.as_deref(),
                        LOGIN_TIMEOUT,
                        open_system_browser,
                    );
                    request.base_url.zeroize();
                    if result_tx
                        .send(NativeOAuthLoginResult {
                            request_id: request.request_id,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("native OAuth login worker thread must start");
        Self {
            requests: request_tx,
            results: result_rx,
        }
    }

    pub fn login(
        &self,
        request_id: String,
        base_url: String,
        provider: Option<String>,
    ) -> Result<(), String> {
        self.requests
            .send(LoginRequest {
                request_id,
                base_url,
                provider,
            })
            .map_err(|_| "Native OAuth login worker has closed".to_owned())
    }

    pub fn poll(&self) -> Option<NativeOAuthLoginResult> {
        let mut latest = None;
        loop {
            match self.results.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

fn run_native_login<F>(
    raw_base_url: &str,
    provider: Option<&str>,
    timeout: Duration,
    open_browser: F,
) -> Result<NativeOAuthSession, String>
where
    F: FnOnce(&str) -> Result<(), String>,
{
    let normalized = crate::remote_probe::normalize_remote_base_url(raw_base_url)?;
    let base = Url::parse(&format!("{normalized}/"))
        .map_err(|_| "Remote gateway URL is invalid".to_owned())?;
    if !matches!(base.scheme(), "http" | "https") || base.host_str().is_none() {
        return Err("Remote gateway must use HTTP or HTTPS".into());
    }
    let mut random = [0u8; 32];
    getrandom::fill(&mut random)
        .map_err(|_| "Could not generate OAuth security values".to_owned())?;
    let verifier = Zeroizing::new(URL_SAFE_NO_PAD.encode(random));
    random.zeroize();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut state_random = [0u8; 24];
    getrandom::fill(&mut state_random)
        .map_err(|_| "Could not generate OAuth security values".to_owned())?;
    let state = Zeroizing::new(URL_SAFE_NO_PAD.encode(state_random));
    state_random.zeroize();

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("Could not start local OAuth callback: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|_| "Could not configure the local OAuth callback".to_owned())?;
    let address = listener
        .local_addr()
        .map_err(|_| "Could not read the local OAuth callback address".to_owned())?;
    let redirect_uri = format!("http://127.0.0.1:{}/callback", address.port());
    let mut authorize = base.clone();
    authorize.set_path(&format!("{}auth/native/authorize", base.path()));
    authorize
        .query_pairs_mut()
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state);
    if let Some(provider) = provider.filter(|value| !value.trim().is_empty()) {
        authorize
            .query_pairs_mut()
            .append_pair("provider", provider);
    }
    open_browser(authorize.as_str())?;

    let deadline = Instant::now() + timeout;
    let code = loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                let request = read_callback_request(&mut stream)?;
                let parsed = Url::parse(&format!("http://127.0.0.1{request}"))
                    .map_err(|_| "OAuth callback URL is invalid".to_owned())?;
                let got_state = parsed
                    .query_pairs()
                    .find(|(key, _)| key == "state")
                    .map(|(_, value)| value.into_owned())
                    .unwrap_or_default();
                let error = parsed
                    .query_pairs()
                    .find(|(key, _)| key == "error")
                    .map(|(_, value)| value.into_owned());
                let code = parsed
                    .query_pairs()
                    .find(|(key, _)| key == "code")
                    .map(|(_, value)| value.into_owned());
                let body = b"<!doctype html><meta charset=utf-8><title>Hermes sign-in</title><p>Sign-in finished. You can close this tab.</p>";
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
                if let Some(error) = error {
                    return Err(format!("Hermes gateway rejected sign-in: {error}"));
                }
                if got_state != *state {
                    return Err("OAuth callback state mismatch".into());
                }
                break code.filter(|code| !code.is_empty()).ok_or_else(|| {
                    "OAuth callback did not include an authorization code".to_owned()
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("Native sign-in timed out before the browser returned".into());
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(format!("OAuth callback listener failed: {error}")),
        }
    };

    let mut token_url = base;
    token_url.set_path(&format!("{}auth/native/token", token_url.path()));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TOKEN_POST_TIMEOUT))
        .max_redirects(0)
        .build()
        .into();
    let body = agent
        .post(token_url.as_str())
        .header("Accept", "application/json")
        .send_json(json!({ "code": code, "code_verifier": verifier.as_str() }))
        .map_err(|error| format!("Hermes OAuth token exchange failed: {error}"))?
        .into_body()
        .into_reader()
        .take(MAX_RESPONSE_BYTES + 1);
    parse_token_response(body)
}

fn read_callback_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while request.len() < 16 * 1024 {
        stream
            .read_exact(&mut byte)
            .map_err(|_| "Could not read OAuth callback request".to_owned())?;
        request.push(byte[0]);
        if request.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    if request.len() >= 16 * 1024 {
        return Err("OAuth callback request was too large".into());
    }
    let first = std::str::from_utf8(&request)
        .map_err(|_| "OAuth callback request was invalid".to_owned())?
        .lines()
        .next()
        .ok_or_else(|| "OAuth callback request was empty".to_owned())?;
    let mut parts = first.split_whitespace();
    if parts.next() != Some("GET") {
        return Err("OAuth callback used an unsupported HTTP method".into());
    }
    let target = parts
        .next()
        .ok_or_else(|| "OAuth callback request target was missing".to_owned())?;
    if !target.starts_with("/callback?") {
        return Err("OAuth callback path was invalid".into());
    }
    Ok(target.to_owned())
}

fn open_system_browser(url: &str) -> Result<(), String> {
    open::that(url).map_err(|error| format!("Could not open the system browser: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader},
        net::TcpStream,
        sync::mpsc,
    };

    #[test]
    fn pkce_flow_validates_state_and_redeems_code() {
        let (browser_tx, browser_rx) = mpsc::channel::<String>();
        let server = TcpListener::bind("127.0.0.1:0").expect("bind fake gateway");
        let address = server.local_addr().unwrap();
        let exchange = thread::spawn(move || {
            let (mut stream, _) = server.accept().expect("accept token exchange");
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with("POST /base/auth/native/token "));
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" {
                    break;
                }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length: ") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0u8; length];
            reader.read_exact(&mut body).unwrap();
            let request: Value = serde_json::from_slice(&body).unwrap();
            let verifier = request["code_verifier"].as_str().unwrap();
            assert_eq!(
                URL_SAFE_NO_PAD
                    .encode(Sha256::digest(verifier.as_bytes()))
                    .len(),
                43
            );
            let response = r#"{"access_token":"access-fixture","refresh_token":"refresh-fixture","expires_at":2000000000,"provider":"nous","user_id":"user-1"}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
        });

        let login = thread::spawn(move || {
            run_native_login(
                &format!("http://{address}/base"),
                Some("nous"),
                Duration::from_secs(3),
                |authorize| {
                    browser_tx.send(authorize.to_owned()).unwrap();
                    Ok(())
                },
            )
        });
        let authorize =
            Url::parse(&browser_rx.recv_timeout(Duration::from_secs(2)).unwrap()).unwrap();
        assert!(authorize.path().ends_with("/base/auth/native/authorize"));
        assert_eq!(
            authorize
                .query_pairs()
                .find(|(key, _)| key == "provider")
                .unwrap()
                .1,
            "nous"
        );
        let callback = authorize
            .query_pairs()
            .find(|(key, _)| key == "redirect_uri")
            .unwrap()
            .1
            .into_owned();
        let state = authorize
            .query_pairs()
            .find(|(key, _)| key == "state")
            .unwrap()
            .1
            .into_owned();
        let callback_url = Url::parse(&callback).unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", callback_url.port().unwrap())).unwrap();
        write!(
            stream,
            "GET /callback?code=auth-code&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
        )
        .unwrap();
        let session = login.join().unwrap().expect("native OAuth login succeeds");
        exchange.join().unwrap();
        assert_eq!(&*session.access_token, "access-fixture");
        assert_eq!(&*session.refresh_token, "refresh-fixture");
        assert_eq!(session.provider, "nous");
    }

    #[test]
    fn callback_rejects_state_mismatch_and_debug_redacts_tokens() {
        let error = run_native_login(
            "http://127.0.0.1:9119/hermes",
            None,
            Duration::from_secs(2),
            |authorize| {
                let authorize = Url::parse(authorize).unwrap();
                let redirect = authorize
                    .query_pairs()
                    .find(|(key, _)| key == "redirect_uri")
                    .unwrap()
                    .1
                    .into_owned();
                let redirect = Url::parse(&redirect).unwrap();
                let mut stream = TcpStream::connect(("127.0.0.1", redirect.port().unwrap()))
                    .map_err(|error| error.to_string())?;
                write!(stream, "GET /callback?code=forged&state=wrong-state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
                    .map_err(|error| error.to_string())?;
                Ok(())
            },
        )
        .expect_err("mismatched state must not redeem authorization code");
        assert_eq!(error, "OAuth callback state mismatch");

        let session = NativeOAuthSession {
            access_token: Zeroizing::new("secret-access".into()),
            refresh_token: Zeroizing::new("secret-refresh".into()),
            expires_at: 0,
            provider: "fixture".into(),
            user_id: "user".into(),
        };
        let debug = format!("{session:?}");
        assert!(!debug.contains("secret-access"));
        assert!(!debug.contains("secret-refresh"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn token_refresh_uses_gateway_endpoint_and_rotates_the_session() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind refresh fixture");
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept refresh request");
            let mut reader = std::io::BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).expect("read request line");
            assert!(line.starts_with("POST /hermes/auth/native/refresh "));
            let mut body_len = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).expect("read request header");
                if header == "\r\n" || header.is_empty() {
                    break;
                }
                if let Some(length) = header.to_ascii_lowercase().strip_prefix("content-length: ") {
                    body_len = length.trim().parse::<usize>().expect("content length");
                }
            }
            let mut body = vec![0; body_len];
            reader.read_exact(&mut body).expect("read request body");
            let request: Value = serde_json::from_slice(&body).expect("decode refresh request");
            assert_eq!(request["refresh_token"], "refresh-before");
            assert_eq!(request["provider"], "nous");
            let response = r#"{"access_token":"access-after","refresh_token":"refresh-after","expires_at":2000000000,"provider":"nous","user_id":"user-1"}"#;
            write!(reader.get_mut(), "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).expect("write response");
        });
        let before = NativeOAuthSession {
            access_token: Zeroizing::new("access-before".into()),
            refresh_token: Zeroizing::new("refresh-before".into()),
            expires_at: 1,
            provider: "nous".into(),
            user_id: "user-1".into(),
        };
        assert!(native_token_needs_refresh(&before, 100));
        let after = refresh_native_session(&format!("http://{address}/hermes"), &before)
            .expect("rotated token response");
        server.join().expect("refresh fixture server");
        assert_eq!(&*after.access_token, "access-after");
        assert_eq!(&*after.refresh_token, "refresh-after");
        assert_eq!(after.expires_at, 2_000_000_000);
        let debug = format!("{after:?}");
        assert!(!debug.contains("access-after"));
        assert!(!debug.contains("refresh-after"));
    }
}
