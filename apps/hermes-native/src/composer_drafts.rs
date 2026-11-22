//! Bounded, atomic persistence for plain text composer drafts.
//!
//! Draft identities are already scoped by the app to a Gateway connection,
//! profile and stored session. Only text is persisted; attachment payloads and
//! temporary upload references remain in the live session view.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const STORE_DIRECTORY: &str = "composer-drafts";
const FILE_PREFIX: &str = "draft-";
const FILE_SUFFIX: &str = ".json";
const VERSION: u8 = 1;
const MAX_DRAFTS: usize = 50;
const MAX_IDENTITY_BYTES: usize = 4096;
const MAX_TEXT_BYTES: usize = 256 * 1024;
const MAX_RECORD_BYTES: u64 = (MAX_TEXT_BYTES + 8192) as u64;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct DraftRecord {
    version: u8,
    identity_key: String,
    text: String,
    updated_at_unix_millis: u64,
}

impl DraftRecord {
    fn validate(&self) -> Result<(), String> {
        if self.version != VERSION
            || self.identity_key.is_empty()
            || self.identity_key.len() > MAX_IDENTITY_BYTES
            || self.text.is_empty()
            || self.text.len() > MAX_TEXT_BYTES
        {
            return Err("Composer draft failed its version, identity or size bounds".into());
        }
        Ok(())
    }
}

fn store_directory(data_directory: &Path) -> PathBuf {
    data_directory.join(STORE_DIRECTORY)
}

fn entry_path(data_directory: &Path, identity_key: &str) -> Result<PathBuf, String> {
    if identity_key.is_empty() || identity_key.len() > MAX_IDENTITY_BYTES {
        return Err("Composer draft identity is invalid".into());
    }
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(identity_key.as_bytes());
    let name = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(store_directory(data_directory).join(format!("{FILE_PREFIX}{name}{FILE_SUFFIX}")))
}

fn ensure_directory(data_directory: &Path) -> Result<PathBuf, String> {
    let directory = store_directory(data_directory);
    if directory.exists()
        && fs::symlink_metadata(&directory)
            .map_err(|error| format!("inspect composer draft store: {error}"))?
            .file_type()
            .is_symlink()
    {
        return Err("Composer draft store cannot be a symbolic link".into());
    }
    fs::create_dir_all(&directory)
        .map_err(|error| format!("create composer draft store: {error}"))?;
    Ok(directory)
}

fn read_record(path: &Path) -> Result<DraftRecord, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect composer draft: {error}"))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_RECORD_BYTES {
        return Err("Composer draft entry is not a bounded regular file".into());
    }
    let bytes = fs::read(path).map_err(|error| format!("read composer draft: {error}"))?;
    let record: DraftRecord = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode composer draft: {error}"))?;
    record.validate()?;
    Ok(record)
}

/// Loads text only when the complete connection/profile/session owner matches.
pub fn load(data_directory: &Path, identity_key: &str) -> Result<Option<String>, String> {
    let path = entry_path(data_directory, identity_key)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("Composer draft entry cannot be a symbolic link".into());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("inspect composer draft: {error}")),
    }
    let record = read_record(&path)?;
    if record.identity_key != identity_key {
        return Err("Composer draft belongs to a different owner".into());
    }
    Ok(Some(record.text))
}

/// Atomically stores nonempty text or clears the scoped draft when text is empty.
pub fn save(data_directory: &Path, identity_key: &str, text: &str) -> Result<(), String> {
    let path = entry_path(data_directory, identity_key)?;
    if text.is_empty() {
        return clear(data_directory, identity_key);
    }
    if text.len() > MAX_TEXT_BYTES {
        return Err("Composer draft exceeds the 256 KiB persistence limit".into());
    }
    let directory = ensure_directory(data_directory)?;
    let updated_at_unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("read composer draft clock: {error}"))?
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let record = DraftRecord {
        version: VERSION,
        identity_key: identity_key.to_owned(),
        text: text.to_owned(),
        updated_at_unix_millis,
    };
    record.validate()?;
    let bytes =
        serde_json::to_vec(&record).map_err(|error| format!("encode composer draft: {error}"))?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err("Composer draft record exceeds its storage limit".into());
    }
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("json.tmp-{}-{sequence}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("create composer draft temporary file: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("persist composer draft: {error}"))?;
        let readback = fs::read(&temporary)
            .map_err(|error| format!("validate composer draft temporary file: {error}"))?;
        let verified: DraftRecord = serde_json::from_slice(&readback)
            .map_err(|error| format!("decode composer draft temporary file: {error}"))?;
        verified.validate()?;
        if verified != record {
            return Err("Composer draft temporary file did not match its source".into());
        }
        fs::rename(&temporary, &path)
            .map_err(|error| format!("atomically replace composer draft: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    prune(data_directory, &directory, identity_key)
}

pub fn clear(data_directory: &Path, identity_key: &str) -> Result<(), String> {
    let path = entry_path(data_directory, identity_key)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("clear composer draft: {error}")),
    }
}

fn prune(data_directory: &Path, directory: &Path, newest_identity: &str) -> Result<(), String> {
    let entries =
        fs::read_dir(directory).map_err(|error| format!("list composer drafts: {error}"))?;
    let mut records = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("read composer draft entry: {error}"))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !file_name.starts_with(FILE_PREFIX) || !file_name.ends_with(FILE_SUFFIX) {
            continue;
        }
        let metadata = entry
            .file_type()
            .map_err(|error| format!("inspect composer draft entry: {error}"))?;
        if !metadata.is_file() {
            continue;
        }
        let record = read_record(&entry.path())?;
        if entry_path(data_directory, &record.identity_key)? != entry.path() {
            return Err("Composer draft filename does not match its owner".into());
        }
        records.push((
            record.updated_at_unix_millis,
            record.identity_key,
            entry.path(),
        ));
    }
    records.sort_by(|left, right| {
        let left_is_newest = left.1 == newest_identity;
        let right_is_newest = right.1 == newest_identity;
        right_is_newest
            .cmp(&left_is_newest)
            .then_with(|| right.0.cmp(&left.0))
            .then_with(|| left.1.cmp(&right.1))
    });
    for (_, _, path) in records.into_iter().skip(MAX_DRAFTS) {
        fs::remove_file(path).map_err(|error| format!("prune old composer draft: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    static TEST_ID: AtomicUsize = AtomicUsize::new(1);

    fn temporary_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "hermes-composer-drafts-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn composer_drafts_round_trip_by_full_owner_and_clear_atomically() {
        let directory = temporary_directory();
        save(
            &directory,
            "connection-a|profile-work|session-1",
            "keep this draft",
        )
        .unwrap();
        assert_eq!(
            load(&directory, "connection-a|profile-work|session-1").unwrap(),
            Some("keep this draft".into())
        );
        assert_eq!(
            load(&directory, "connection-a|profile-personal|session-1").unwrap(),
            None,
            "the same session ID in another profile is isolated"
        );
        save(&directory, "connection-a|profile-work|session-1", "").unwrap();
        assert_eq!(
            load(&directory, "connection-a|profile-work|session-1").unwrap(),
            None
        );
        assert!(
            load(&directory, "connection-a|profile-personal|session-1")
                .unwrap()
                .is_none()
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn composer_draft_store_caps_at_fifty_and_keeps_the_current_write() {
        let directory = temporary_directory();
        for index in 0..=MAX_DRAFTS {
            let identity = format!("owner-{index}");
            save(&directory, &identity, &format!("draft-{index}")).unwrap();
        }
        let store = store_directory(&directory);
        let count = fs::read_dir(&store).unwrap().count();
        assert_eq!(count, MAX_DRAFTS);
        assert_eq!(
            load(&directory, "owner-50").unwrap(),
            Some("draft-50".into())
        );
        assert_eq!(load(&directory, "owner-0").unwrap(), None);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_or_oversized_draft_fails_closed_without_returning_text() {
        let directory = temporary_directory();
        let path = entry_path(&directory, "owner").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{not json").unwrap();
        assert!(load(&directory, "owner").is_err());
        fs::remove_dir_all(directory).unwrap();

        let directory = temporary_directory();
        assert!(save(&directory, "owner", &"x".repeat(MAX_TEXT_BYTES + 1)).is_err());
        assert!(load(&directory, "owner").unwrap().is_none());
        if directory.exists() {
            fs::remove_dir_all(directory).unwrap();
        }
    }
}
