//! Per-user encrypted persistence for native OAuth sessions.
//!
//! The store is isolated under Hermes Native's own profile directory. Windows
//! DPAPI protects the serialized token set; only base URLs and ciphertext are
//! written to disk. It never reads or writes the installed Hermes profile.

use crate::{native_oauth::NativeOAuthSession, remote_probe::normalize_remote_base_url};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
use zeroize::{Zeroize, Zeroizing};

const STORE_FILE: &str = "native-oauth-tokens.json";
const MAX_STORE_BYTES: u64 = 4 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub fn save(
    profile_directory: &Path,
    base_url: &str,
    session: &NativeOAuthSession,
) -> Result<(), String> {
    let mut value = json!({
        "credentialType": "oauth",
        "accessToken": session.access_token.as_str(),
        "refreshToken": session.refresh_token.as_str(),
        "expiresAt": session.expires_at,
        "provider": session.provider,
        "userId": session.user_id,
    });
    save_value(profile_directory, base_url, &mut value)
}

pub fn save_session_token(
    profile_directory: &Path,
    base_url: &str,
    token: &str,
) -> Result<(), String> {
    if token.trim().is_empty() || token.len() > 16 * 1024 || token.chars().any(char::is_control) {
        return Err("Hermes session token is invalid".into());
    }
    let mut value = json!({"credentialType": "session_token", "sessionToken": token});
    save_value(profile_directory, base_url, &mut value)
}

fn save_value(profile_directory: &Path, base_url: &str, value: &mut Value) -> Result<(), String> {
    let mut plaintext = Zeroizing::new(
        serde_json::to_vec(value)
            .map_err(|_| "Could not encode secure gateway credential".to_owned())?,
    );
    wipe_value(value);
    let base_url = normalize_remote_base_url(base_url)?;
    let path = profile_directory.join(STORE_FILE);
    let mut store = read_store(&path)?;
    let encrypted = protect(&plaintext)?;
    plaintext.zeroize();
    store.insert(base_url, STANDARD.encode(encrypted));
    let bytes = serde_json::to_vec_pretty(&store)
        .map_err(|_| "Could not encode secure gateway credential store".to_owned())?;
    atomic_write(&path, &bytes)
}

pub fn load(
    profile_directory: &Path,
    base_url: &str,
) -> Result<Option<NativeOAuthSession>, String> {
    let base_url = normalize_remote_base_url(base_url)?;
    let path = profile_directory.join(STORE_FILE);
    let store = read_store(&path)?;
    let Some(ciphertext) = store.get(&base_url) else {
        return Ok(None);
    };
    let mut ciphertext = STANDARD
        .decode(ciphertext)
        .map_err(|_| "Stored OAuth token envelope is invalid".to_owned())?;
    let plaintext = unprotect(&ciphertext);
    ciphertext.zeroize();
    let mut plaintext = Zeroizing::new(plaintext?);
    let mut value: Value = serde_json::from_slice(&plaintext)
        .map_err(|_| "Stored OAuth token set is invalid".to_owned())?;
    plaintext.zeroize();
    if value
        .get("credentialType")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind != "oauth")
    {
        wipe_value(&mut value);
        return Ok(None);
    }
    let session = session_from_value(&value);
    wipe_value(&mut value);
    session.map(Some)
}

pub fn load_session_token(
    profile_directory: &Path,
    base_url: &str,
) -> Result<Option<Zeroizing<String>>, String> {
    let base_url = normalize_remote_base_url(base_url)?;
    let path = profile_directory.join(STORE_FILE);
    let store = read_store(&path)?;
    let Some(ciphertext) = store.get(&base_url) else {
        return Ok(None);
    };
    let mut ciphertext = STANDARD
        .decode(ciphertext)
        .map_err(|_| "Stored credential envelope is invalid".to_owned())?;
    let plaintext = unprotect(&ciphertext);
    ciphertext.zeroize();
    let mut plaintext = Zeroizing::new(plaintext?);
    let mut value: Value = serde_json::from_slice(&plaintext)
        .map_err(|_| "Stored gateway credential is invalid".to_owned())?;
    plaintext.zeroize();
    let token = if value.get("credentialType").and_then(Value::as_str) == Some("session_token") {
        value
            .get("sessionToken")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .map(|token| Zeroizing::new(token.to_owned()))
    } else {
        None
    };
    wipe_value(&mut value);
    Ok(token)
}

pub fn remove(profile_directory: &Path, base_url: &str) -> Result<bool, String> {
    let base_url = normalize_remote_base_url(base_url)?;
    let path = profile_directory.join(STORE_FILE);
    let mut store = read_store(&path)?;
    if store.remove(&base_url).is_none() {
        return Ok(false);
    }
    let bytes = serde_json::to_vec_pretty(&store)
        .map_err(|_| "Could not encode secure OAuth token store".to_owned())?;
    atomic_write(&path, &bytes)?;
    Ok(true)
}

fn session_from_value(value: &Value) -> Result<NativeOAuthSession, String> {
    let access = value
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Stored OAuth token set omitted its access token".to_owned())?;
    Ok(NativeOAuthSession {
        access_token: Zeroizing::new(access.to_owned()),
        refresh_token: Zeroizing::new(
            value
                .get("refreshToken")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ),
        expires_at: value.get("expiresAt").and_then(Value::as_i64).unwrap_or(0),
        provider: value
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        user_id: value
            .get("userId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    })
}

fn wipe_value(value: &mut Value) {
    match value {
        Value::String(text) => text.zeroize(),
        Value::Array(items) => items.iter_mut().for_each(wipe_value),
        Value::Object(object) => object.values_mut().for_each(wipe_value),
        _ => {}
    }
}

fn read_store(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(_) => return Err("Could not read secure OAuth token store".into()),
    };
    if bytes.len() as u64 > MAX_STORE_BYTES {
        return Err("Secure OAuth token store exceeds 4 MiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Secure OAuth token store is invalid".into())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Secure OAuth token store has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|_| "Could not create Hermes Native profile directory".to_owned())?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("json.tmp-{}-{sequence}", std::process::id()));
    let write_result = (|| {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Could not create temporary secure OAuth store".to_owned())?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "Could not write temporary secure OAuth store".to_owned())?;
        let check = fs::read(&temporary)
            .map_err(|_| "Could not verify temporary secure OAuth store".to_owned())?;
        let _: BTreeMap<String, String> = serde_json::from_slice(&check)
            .map_err(|_| "Temporary secure OAuth store failed validation".to_owned())?;
        fs::rename(&temporary, path)
            .map_err(|_| "Could not atomically replace secure OAuth token store".to_owned())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

#[cfg(windows)]
fn protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData},
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(plaintext.len())
            .map_err(|_| "OAuth token set is too large".to_owned())?,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("Windows could not protect OAuth tokens for this user".into());
    }
    let encrypted =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        LocalFree(output.pbData.cast());
    }
    Ok(encrypted)
}

#[cfg(not(windows))]
fn protect(_plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err("Secure OAuth token persistence is not supported on this platform".into())
}

#[cfg(windows)]
fn unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(ciphertext.len())
            .map_err(|_| "OAuth token envelope is too large".to_owned())?,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("Windows could not unlock OAuth tokens for this user".into());
    }
    let plaintext =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        LocalFree(output.pbData.cast());
    }
    Ok(plaintext)
}

#[cfg(not(windows))]
fn unprotect(_ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err("Secure OAuth token persistence is not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn dpapi_token_store_round_trips_without_plaintext_on_disk() {
        let directory = std::env::temp_dir().join(format!(
            "hermes-native-token-store-test-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let session = NativeOAuthSession {
            access_token: Zeroizing::new("fixture-access-secret".into()),
            refresh_token: Zeroizing::new("fixture-refresh-secret".into()),
            expires_at: 2_000_000_000,
            provider: "fixture-provider".into(),
            user_id: "fixture-user".into(),
        };
        let base_url = "http://localhost:9123/hermes";
        save(&directory, base_url, &session).expect("save encrypted tokens");
        let file = fs::read_to_string(directory.join(STORE_FILE)).unwrap();
        assert!(!file.contains("fixture-access-secret"));
        assert!(!file.contains("fixture-refresh-secret"));
        let rotated = NativeOAuthSession {
            access_token: Zeroizing::new("rotated-access-secret".into()),
            refresh_token: Zeroizing::new("rotated-refresh-secret".into()),
            expires_at: 2_000_000_100,
            provider: "fixture-provider".into(),
            user_id: "fixture-user".into(),
        };
        save(&directory, base_url, &rotated).expect("replace existing encrypted tokens");
        let loaded = load(&directory, "http://localhost:9123/hermes/")
            .expect("load protected tokens")
            .expect("saved session");
        assert_eq!(&*loaded.access_token, "rotated-access-secret");
        assert_eq!(&*loaded.refresh_token, "rotated-refresh-secret");
        let file = fs::read_to_string(directory.join(STORE_FILE)).unwrap();
        assert!(!file.contains("rotated-access-secret"));
        assert!(!file.contains("rotated-refresh-secret"));
        assert!(remove(&directory, base_url).expect("remove tokens"));
        assert!(load(&directory, base_url).unwrap().is_none());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    #[cfg(windows)]
    fn dpapi_store_separates_static_session_tokens_from_oauth_sessions() {
        let directory = std::env::temp_dir().join(format!(
            "hermes-native-static-token-test-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let base_url = "https://gateway.example.com/hermes";
        save_session_token(&directory, base_url, "fixture-static-session-token")
            .expect("save encrypted session token");
        let file = fs::read_to_string(directory.join(STORE_FILE)).unwrap();
        assert!(!file.contains("fixture-static-session-token"));
        assert_eq!(
            load_session_token(&directory, base_url)
                .unwrap()
                .as_deref()
                .map(String::as_str),
            Some("fixture-static-session-token")
        );
        assert!(load(&directory, base_url).unwrap().is_none());

        let session = NativeOAuthSession {
            access_token: Zeroizing::new("oauth-access".into()),
            refresh_token: Zeroizing::new("oauth-refresh".into()),
            expires_at: 2_000_000_000,
            provider: "fixture-provider".into(),
            user_id: "fixture-user".into(),
        };
        save(&directory, base_url, &session).expect("replace with encrypted OAuth session");
        assert!(load_session_token(&directory, base_url).unwrap().is_none());
        assert_eq!(
            &*load(&directory, base_url).unwrap().unwrap().access_token,
            "oauth-access"
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn token_store_rejects_corrupt_envelopes_without_panicking() {
        let directory = std::env::temp_dir().join(format!(
            "hermes-native-token-corrupt-test-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join(STORE_FILE),
            br#"{"http://localhost:9123":"not-base64!"}"#,
        )
        .unwrap();
        assert!(load(&directory, "http://localhost:9123").is_err());
        let _ = fs::remove_dir_all(directory);
    }
}
