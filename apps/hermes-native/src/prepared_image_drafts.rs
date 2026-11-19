use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const STORE_DIRECTORY: &str = "prepared-image-drafts";
const MAX_DRAFTS: usize = 128;
const MAX_DRAFT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_DRAFT_TEXT_BYTES: usize = 128 * 1024;
const MAX_ATTACHMENTS: usize = 64;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedImageAttachment {
    pub path: String,
    pub source_path: String,
    pub name: String,
    pub mime: String,
    pub ref_text: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedImageDraft {
    pub version: u8,
    pub id: String,
    pub connection_id: String,
    pub profile: Option<String>,
    pub stored_session_id: String,
    pub text: String,
    pub attachments: Vec<PreparedImageAttachment>,
    pub created_at_unix_seconds: u64,
}

impl PreparedImageDraft {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.id.is_empty()
            || self.id.len() > 256
            || self.connection_id.is_empty()
            || self.connection_id.len() > 256
            || self
                .profile
                .as_ref()
                .is_some_and(|profile| profile.len() > 256)
            || self.stored_session_id.is_empty()
            || self.stored_session_id.len() > 512
            || self.text.len() > MAX_DRAFT_TEXT_BYTES
            || self.attachments.is_empty()
            || self.attachments.len() > MAX_ATTACHMENTS
            || self.attachments.iter().any(|attachment| {
                attachment.path.is_empty()
                    || attachment.path.len() > 4096
                    || attachment.source_path.is_empty()
                    || attachment.source_path.len() > 4096
                    || attachment.name.is_empty()
                    || attachment.name.len() > 1024
                    || attachment.mime.len() > 256
                    || attachment
                        .ref_text
                        .as_ref()
                        .is_some_and(|text| text.len() > 8192)
            })
        {
            return Err("Prepared image draft failed its size or identity bounds".into());
        }
        Ok(())
    }
}

fn store_directory(data_directory: &Path) -> PathBuf {
    data_directory.join(STORE_DIRECTORY)
}

fn entry_path(data_directory: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.len() > 256 {
        return Err("Prepared image draft ID is invalid".into());
    }
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(id.as_bytes());
    let name = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(store_directory(data_directory).join(format!("draft-{name}.json")))
}

pub fn load(data_directory: &Path) -> Result<Vec<PreparedImageDraft>, String> {
    let directory = store_directory(data_directory);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not list prepared image drafts: {error}")),
    };
    let mut drafts = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("Could not read prepared image draft entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect prepared image draft entry: {error}"))?;
        if !file_type.is_file()
            || !entry.file_name().to_string_lossy().starts_with("draft-")
            || !entry.file_name().to_string_lossy().ends_with(".json")
        {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Could not inspect prepared image draft size: {error}"))?;
        if metadata.len() > MAX_DRAFT_BYTES {
            return Err("Prepared image draft store contains an oversized entry".into());
        }
        let bytes = fs::read(entry.path())
            .map_err(|error| format!("Could not read prepared image draft: {error}"))?;
        let draft: PreparedImageDraft = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Prepared image draft is invalid JSON: {error}"))?;
        draft.validate()?;
        drafts.push(draft);
        if drafts.len() > MAX_DRAFTS {
            return Err("Prepared image draft store exceeds the 128-entry limit".into());
        }
    }
    drafts.sort_by(|left, right| {
        right
            .created_at_unix_seconds
            .cmp(&left.created_at_unix_seconds)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(drafts)
}

pub fn write(data_directory: &Path, draft: &PreparedImageDraft) -> Result<(), String> {
    draft.validate()?;
    let directory = store_directory(data_directory);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create prepared image draft store: {error}"))?;
    let path = entry_path(data_directory, &draft.id)?;
    let bytes = serde_json::to_vec(draft)
        .map_err(|error| format!("Could not encode prepared image draft: {error}"))?;
    if bytes.len() as u64 > MAX_DRAFT_BYTES {
        return Err("Prepared image draft exceeds its 4 MiB limit".into());
    }
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("json.tmp-{}-{sequence}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Could not create prepared image draft temp file: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("Could not persist prepared image draft: {error}"))?;
        let readback = fs::read(&temporary).map_err(|error| {
            format!("Could not validate prepared image draft temp file: {error}")
        })?;
        let verified: PreparedImageDraft = serde_json::from_slice(&readback).map_err(|error| {
            format!("Prepared image draft temp file failed validation: {error}")
        })?;
        verified.validate()?;
        fs::rename(&temporary, &path)
            .map_err(|error| format!("Could not atomically replace prepared image draft: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    let current_count = load(data_directory)?.len();
    if current_count > MAX_DRAFTS {
        let _ = remove(data_directory, &draft.id);
        return Err("Prepared image draft store exceeds the 128-entry limit".into());
    }
    Ok(())
}

pub fn remove(data_directory: &Path, id: &str) -> Result<(), String> {
    let path = entry_path(data_directory, id)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Could not retire prepared image draft: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "hermes-prepared-image-drafts-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn sample() -> PreparedImageDraft {
        PreparedImageDraft {
            version: 1,
            id: "submission-draft-1".into(),
            connection_id: "connection-a".into(),
            profile: Some("profile-a".into()),
            stored_session_id: "stored-session-a".into(),
            text: "Describe this".into(),
            attachments: vec![PreparedImageAttachment {
                path: "attachments/capture.png".into(),
                source_path: "C:\\Users\\alice\\Pictures\\capture.png".into(),
                name: "capture.png".into(),
                mime: "image/png".into(),
                ref_text: None,
            }],
            created_at_unix_seconds: 100,
        }
    }

    #[test]
    fn prepared_image_drafts_round_trip_and_retire_atomically() {
        let directory = temp_directory();
        let mut draft = sample();
        write(&directory, &draft).unwrap();
        assert_eq!(load(&directory).unwrap(), [draft.clone()]);
        draft.text = "Updated after restoring the draft".into();
        draft.created_at_unix_seconds += 1;
        write(&directory, &draft).unwrap();
        assert_eq!(load(&directory).unwrap(), [draft.clone()]);
        remove(&directory, &draft.id).unwrap();
        assert!(load(&directory).unwrap().is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn prepared_image_draft_rejects_invalid_identity_and_oversized_text() {
        let directory = temp_directory();
        let mut draft = sample();
        draft.connection_id.clear();
        assert!(write(&directory, &draft).is_err());
        draft = sample();
        draft.text = "x".repeat(MAX_DRAFT_TEXT_BYTES + 1);
        assert!(write(&directory, &draft).is_err());
        assert!(!directory.exists());
    }
}
