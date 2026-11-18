//! Per-user, cross-process session/transcript invalidation hints.
//!
//! Event files carry only a stored session ID and message count for transcript
//! invalidations, never message content or credentials. The backend remains
//! canonical; a peer receipt asks a window to re-read its own scoped data.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

const EVENT_PREFIX: &str = "session-peer-v2-";
const EVENT_SUFFIX: &str = ".json";
const EVENT_VERSION: u8 = 2;
const MAX_EVENT_BYTES: u64 = 512;
const MAX_EVENT_FILES: usize = 512;
const EVENT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PeerPing {
    version: u8,
    id: String,
    event: PeerEvent,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PeerEvent {
    SessionsChanged,
    TranscriptChanged {
        stored_session_id: String,
        message_count: usize,
    },
}

#[derive(Debug)]
pub struct SessionPeerSync {
    directory: PathBuf,
    seen: BTreeSet<String>,
}

impl SessionPeerSync {
    pub fn open(profile_directory: &Path) -> Result<Self, String> {
        if profile_directory.exists()
            && fs::symlink_metadata(profile_directory)
                .map_err(|error| format!("inspect Hermes Native profile directory: {error}"))?
                .file_type()
                .is_symlink()
        {
            return Err("Hermes Native profile directory cannot be a symbolic link".into());
        }
        fs::create_dir_all(profile_directory)
            .map_err(|error| format!("create Hermes Native profile directory: {error}"))?;
        let directory = profile_directory.join("session-peer-events");
        fs::create_dir_all(&directory)
            .map_err(|error| format!("create session peer event directory: {error}"))?;
        if fs::symlink_metadata(&directory)
            .map_err(|error| format!("inspect session peer event directory: {error}"))?
            .file_type()
            .is_symlink()
        {
            return Err("session peer event directory cannot be a symbolic link".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|error| format!("restrict session peer event directory: {error}"))?;
        }
        let mut sync = Self {
            directory,
            seen: BTreeSet::new(),
        };
        sync.prune_old_events();
        // Existing events predate this process. The initial session read is
        // authoritative, so replaying these old invalidations is unnecessary.
        sync.seen = sync
            .scan_ids(false)?
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        Ok(sync)
    }

    /// Atomically publishes one opaque hint and suppresses it in this sender.
    pub fn broadcast(&mut self) -> Result<(), String> {
        self.broadcast_event(PeerEvent::SessionsChanged)
    }

    pub fn broadcast_transcript_changed(
        &mut self,
        stored_session_id: String,
        message_count: usize,
    ) -> Result<(), String> {
        if stored_session_id.is_empty() || stored_session_id.len() > 256 {
            return Err("transcript peer event has an invalid stored session ID".into());
        }
        self.broadcast_event(PeerEvent::TranscriptChanged {
            stored_session_id,
            message_count,
        })
    }

    fn broadcast_event(&mut self, event: PeerEvent) -> Result<(), String> {
        let id = random_id()?;
        let record = PeerPing {
            version: EVENT_VERSION,
            id: id.clone(),
            event,
        };
        let bytes = serde_json::to_vec(&record)
            .map_err(|error| format!("encode session peer event: {error}"))?;
        if bytes.len() as u64 > MAX_EVENT_BYTES {
            return Err("session peer event exceeds the size limit".into());
        }
        let final_path = self.directory.join(event_name(&id));
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .directory
            .join(format!(".tmp-{}-{sequence}-{id}", std::process::id()));
        let result: Result<(), String> = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| format!("create session peer event receipt: {error}"))?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| format!("write session peer event receipt: {error}"))?;
            validate_event(&temporary, &id)?;
            fs::rename(&temporary, &final_path)
                .map_err(|error| format!("publish session peer event receipt: {error}"))?;
            validate_event(&final_path, &id)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        self.seen.insert(id);
        self.prune_old_events();
        Ok(())
    }

    /// Returns each new validated peer invalidation once.
    pub fn poll(&mut self) -> Result<Vec<PeerEvent>, String> {
        let records = self.scan_ids(true)?;
        let mut events = Vec::new();
        let present = records
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        for (id, valid) in records {
            if valid && !self.seen.contains(&id) {
                let path = self.directory.join(event_name(&id));
                if let Ok(event) = read_event(&path, &id) {
                    events.push(event.event);
                    self.seen.insert(id);
                }
            }
        }
        self.seen.retain(|id| present.contains(id));
        Ok(events)
    }

    fn scan_ids(&self, validate_records: bool) -> Result<Vec<(String, bool)>, String> {
        let entries = fs::read_dir(&self.directory)
            .map_err(|error| format!("scan session peer event directory: {error}"))?;
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| format!("read session peer event entry: {error}"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(id) = parse_event_name(&name) else {
                continue;
            };
            let metadata = match fs::symlink_metadata(entry.path()) {
                Ok(metadata) if metadata.file_type().is_file() => metadata,
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("inspect session peer event receipt: {error}")),
            };
            if metadata.len() > MAX_EVENT_BYTES {
                records.push((id, false));
                continue;
            }
            let valid = !validate_records || validate_event(&entry.path(), &id).is_ok();
            records.push((id, valid));
            if records.len() > MAX_EVENT_FILES {
                return Err("session peer event directory exceeds its bounded scan limit".into());
            }
        }
        Ok(records)
    }

    fn prune_old_events(&self) {
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return;
        };
        let now = SystemTime::now();
        let mut files = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let id = parse_event_name(&entry.file_name().to_string_lossy())?;
                let path = entry.path();
                let metadata = fs::symlink_metadata(entry.path()).ok()?;
                if !metadata.file_type().is_file() {
                    return None;
                }
                Some((id, path, metadata.modified().ok()?))
            })
            .collect::<Vec<_>>();
        files.sort_by_key(|(_, _, modified)| *modified);
        let remove_count = files.len().saturating_sub(MAX_EVENT_FILES / 2);
        for (index, (_, path, modified)) in files.into_iter().enumerate() {
            let old = now
                .duration_since(modified)
                .is_ok_and(|age| age > EVENT_TTL);
            if old || index < remove_count {
                let _ = fs::remove_file(path);
            }
        }
        // Crash-left temporary files are not peer events. Remove only those
        // older than the same conservative retention window.
        if let Ok(entries) = fs::read_dir(&self.directory) {
            for entry in entries.filter_map(Result::ok) {
                if !entry.file_name().to_string_lossy().starts_with(".tmp-") {
                    continue;
                }
                if entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|modified| now.duration_since(modified).ok())
                    .is_some_and(|age| age > EVENT_TTL)
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn event_name(id: &str) -> String {
    format!("{EVENT_PREFIX}{id}{EVENT_SUFFIX}")
}

fn parse_event_name(name: &str) -> Option<String> {
    let id = name
        .strip_prefix(EVENT_PREFIX)?
        .strip_suffix(EVENT_SUFFIX)?;
    (id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| id.to_ascii_lowercase())
}

fn validate_event(path: &Path, expected_id: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect session peer event receipt: {error}"))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_EVENT_BYTES {
        return Err("session peer event receipt is not a bounded regular file".into());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("read session peer event receipt: {error}"))?;
    decode_event(&bytes, expected_id)?;
    Ok(())
}

fn read_event(path: &Path, expected_id: &str) -> Result<PeerPing, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect session peer event receipt: {error}"))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_EVENT_BYTES {
        return Err("session peer event receipt is not a bounded regular file".into());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("read session peer event receipt: {error}"))?;
    decode_event(&bytes, expected_id)
}

fn decode_event(bytes: &[u8], expected_id: &str) -> Result<PeerPing, String> {
    let event: PeerPing = serde_json::from_slice(bytes)
        .map_err(|error| format!("decode session peer event receipt: {error}"))?;
    if event.version != EVENT_VERSION || event.id != expected_id {
        return Err("session peer event receipt has an unknown version or mismatched ID".into());
    }
    if let PeerEvent::TranscriptChanged {
        stored_session_id, ..
    } = &event.event
        && (stored_session_id.is_empty() || stored_session_id.len() > 256)
    {
        return Err("session peer event has an invalid stored session ID".into());
    }
    Ok(event)
}

fn random_id() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("generate session peer event ID: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn temporary_profile(label: &str) -> PathBuf {
        let id = random_id().expect("unique test profile ID");
        std::env::temp_dir().join(format!("hermes-native-peer-{label}-{id}"))
    }

    #[test]
    fn peer_receives_one_opaque_ping_and_sender_does_not_receive_its_own() {
        let profile = temporary_profile("peers");
        let mut sender = SessionPeerSync::open(&profile).expect("open sender");
        let mut peer = SessionPeerSync::open(&profile).expect("open peer");
        sender.broadcast().expect("publish ping");
        assert!(sender.poll().expect("sender poll").is_empty());
        assert_eq!(
            peer.poll().expect("peer poll"),
            vec![PeerEvent::SessionsChanged]
        );
        assert!(peer.poll().expect("duplicate peer poll").is_empty());
        let files = fs::read_dir(profile.join("session-peer-events"))
            .expect("event directory")
            .filter_map(Result::ok)
            .filter(|entry| parse_event_name(&entry.file_name().to_string_lossy()).is_some())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        let bytes = fs::read(files[0].path()).expect("event file");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("event JSON");
        assert_eq!(value.as_object().expect("event object").len(), 3);
        assert!(value.get("session_id").is_none());
        let _ = fs::remove_dir_all(profile);
    }

    #[test]
    fn transcript_notification_is_typed_bounded_and_contains_no_transcript_content() {
        let profile = temporary_profile("transcript");
        let mut sender = SessionPeerSync::open(&profile).expect("open sender");
        let mut peer = SessionPeerSync::open(&profile).expect("open peer");
        sender
            .broadcast_transcript_changed("stored-1".into(), 42)
            .expect("publish transcript hint");
        assert_eq!(
            peer.poll().expect("receive transcript hint"),
            vec![PeerEvent::TranscriptChanged {
                stored_session_id: "stored-1".into(),
                message_count: 42,
            }]
        );
        let entry = fs::read_dir(profile.join("session-peer-events"))
            .expect("event directory")
            .filter_map(Result::ok)
            .find(|entry| parse_event_name(&entry.file_name().to_string_lossy()).is_some())
            .expect("transcript event file");
        let bytes = fs::read(entry.path()).expect("event payload");
        let text = String::from_utf8(bytes).expect("JSON text");
        assert!(text.contains("stored-1"));
        assert!(!text.contains("message content"));
        assert!(
            sender
                .broadcast_transcript_changed(String::new(), 0)
                .is_err()
        );
        let _ = fs::remove_dir_all(profile);
    }

    #[test]
    fn startup_ignores_old_events_and_poll_ignores_partial_and_unknown_records() {
        let profile = temporary_profile("recovery");
        let directory = profile.join("session-peer-events");
        fs::create_dir_all(&directory).expect("event directory");
        let stale = "01".repeat(16);
        fs::write(
            directory.join(event_name(&stale)),
            br#"{"version":2,"id":"01010101010101010101010101010101","event":{"kind":"sessions_changed"}}"#,
        )
        .expect("stale valid event");
        fs::write(directory.join(".tmp-crashed"), b"partial").expect("crash-left temporary record");
        let mut peer = SessionPeerSync::open(&profile).expect("open peer");
        assert!(peer.poll().expect("stale events not replayed").is_empty());
        let unknown = "02".repeat(16);
        fs::write(
            directory.join(event_name(&unknown)),
            br#"{"version":2,"id":"02020202020202020202020202020202","event":{"kind":"unknown"}}"#,
        )
        .expect("unknown version record");
        let malformed = "03".repeat(16);
        fs::write(directory.join(event_name(&malformed)), b"{").expect("malformed final record");
        assert!(peer.poll().expect("invalid records ignored").is_empty());
        let fresh = "04".repeat(16);
        fs::write(
            directory.join(event_name(&fresh)),
            br#"{"version":2,"id":"04040404040404040404040404040404","event":{"kind":"sessions_changed"}}"#,
        )
        .expect("fresh peer receipt");
        assert_eq!(peer.poll().expect("fresh event observed").len(), 1);
        let _ = fs::remove_dir_all(profile);
    }

    #[test]
    fn concurrent_writers_publish_distinct_peer_events() {
        let profile = temporary_profile("concurrent");
        let mut reader = SessionPeerSync::open(&profile).expect("open reader before writers");
        let mut workers = (0..8)
            .map(|_| {
                let profile = profile.clone();
                std::thread::spawn(move || {
                    let mut peer = SessionPeerSync::open(&profile).expect("open peer sender");
                    peer.broadcast().expect("publish peer event");
                })
            })
            .collect::<Vec<_>>();
        for worker in workers.drain(..) {
            worker.join().expect("writer joined");
        }
        assert_eq!(
            reader
                .poll()
                .expect("concurrent peer events observed")
                .len(),
            8
        );
        assert!(reader.poll().expect("events delivered once").is_empty());
        let files = fs::read_dir(profile.join("session-peer-events"))
            .expect("event directory")
            .filter_map(Result::ok)
            .filter(|entry| parse_event_name(&entry.file_name().to_string_lossy()).is_some())
            .count();
        assert_eq!(files, 8);
        let _ = fs::remove_dir_all(profile);
    }

    #[test]
    fn separate_processes_share_peer_events_without_a_payload_channel() {
        const TEST_DIRECTORY: &str = "HERMES_NATIVE_PEER_TEST_DIRECTORY";
        if let Some(directory) = std::env::var_os(TEST_DIRECTORY) {
            let mut sender = SessionPeerSync::open(Path::new(&directory))
                .expect("open child-process peer sender");
            sender
                .broadcast()
                .expect("child process publishes peer event");
            return;
        }

        let profile = temporary_profile("processes");
        let mut reader = SessionPeerSync::open(&profile).expect("open parent peer reader");
        for _ in 0..2 {
            let output = Command::new(std::env::current_exe().expect("test executable"))
                .arg("--exact")
                .arg("session_peer_sync::tests::separate_processes_share_peer_events_without_a_payload_channel")
                .env(TEST_DIRECTORY, &profile)
                .output()
                .expect("launch isolated sender process");
            assert!(
                output.status.success(),
                "child process failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert_eq!(reader.poll().expect("read child-process events").len(), 2);
        assert!(
            reader
                .poll()
                .expect("child-process events are consumed once")
                .is_empty()
        );
        let receipts = fs::read_dir(profile.join("session-peer-events"))
            .expect("event directory")
            .filter_map(Result::ok)
            .filter(|entry| parse_event_name(&entry.file_name().to_string_lossy()).is_some())
            .count();
        assert_eq!(receipts, 2);
        let _ = fs::remove_dir_all(profile);
    }
}
