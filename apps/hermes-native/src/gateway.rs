//! Attaches to Hermes' authoritative gateway using its supported private ticket and canonical RPC.

use crate::{
    artifacts::{
        ArtifactIndexSnapshot, ArtifactMessagePage, ArtifactMessagePagination, ArtifactSession,
        load_artifacts_for_sessions,
    },
    files::{FilePreviewContent, decode_artifact_image_preview},
    native_oauth::{NativeOAuthSession, native_token_needs_refresh, refresh_native_session},
    remote_probe::RemoteAuthMode,
    repo_discovery::{DiscoveredRepo, RepoScanOptions, RepoScanPolicy, scan_git_repos},
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt,
    io::{self, Read, Write},
    net::TcpStream,
    path::PathBuf,
    process::Command,
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};
use tungstenite::{ClientRequestBuilder, Message, client::connect_with_config, http::Uri};
use url::Url;
use zeroize::{Zeroize, Zeroizing};

const CANONICAL_PROTOCOL: &str = "hermes-gateway-v1";
const TICKET_PROTOCOL_PREFIX: &str = "hermes-gateway-ticket.";
const GATEWAY_HTTP_RESPONSE_LIMIT: u64 = 512 * 1024;
const PROJECT_TREE_HTTP_RESPONSE_LIMIT: u64 = 8 * 1024 * 1024;
const ARTIFACT_GATEWAY_RESPONSE_LIMIT: u64 = 64 * 1024 * 1024;
const ARTIFACT_IMAGE_RESPONSE_LIMIT: u64 = 24 * 1024 * 1024;
const ARTIFACT_SESSION_LIMIT: usize = 30;

#[derive(Clone, Debug, Deserialize)]
pub struct GatewayEndpoint {
    pub profile_id: String,
    pub instance_id: String,
    pub authority_epoch: u64,
    pub runtime_protocol: u32,
    pub api_origin: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub control_home: Option<String>,
    #[serde(skip)]
    pub(crate) remote_auth: Option<RemoteGatewayAuth>,
}

#[derive(Clone)]
pub(crate) enum RemoteGatewayAuth {
    Token(Zeroizing<String>),
    OAuth {
        session: Arc<Mutex<NativeOAuthSession>>,
        credential_store_directory: std::path::PathBuf,
    },
}

enum RemoteHttpAuth {
    SessionToken(Zeroizing<String>),
    Bearer(Zeroizing<String>),
}

impl fmt::Debug for RemoteGatewayAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Token(_) => formatter.write_str("RemoteGatewayAuth::Token([REDACTED])"),
            Self::OAuth { .. } => formatter.write_str("RemoteGatewayAuth::OAuth([REDACTED])"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct EnsureReply {
    state: String,
    #[serde(default)]
    reason_code: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    endpoint: Option<GatewayEndpoint>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SessionSummary {
    pub id: String,
    /// Exact source connection when the host supplied a connection-tagged row.
    /// `None` is reserved for legacy rows that genuinely lack that owner tag.
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default, rename = "_lineage_root_id")]
    pub lineage_root_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub preview: String,
    #[serde(default)]
    pub message_count: usize,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub started_at: f64,
    #[serde(default)]
    pub last_active: Option<f64>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub unread: bool,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandCenterSessionSnapshot {
    pub sessions: Vec<SessionSummary>,
    pub profiles_truncated: BTreeMap<String, bool>,
    pub failed_profiles: BTreeSet<String>,
    pub error: Option<String>,
}

impl SessionSummary {
    pub fn durable_session_id(&self) -> &str {
        self.lineage_root_id.as_deref().unwrap_or(&self.id)
    }

    pub fn owner_identity(
        &self,
    ) -> Result<crate::session_owner::SessionOwnerIdentity, crate::session_owner::SessionOwnerError>
    {
        crate::session_owner::SessionOwnerIdentity::new(
            self.connection_id.as_deref(),
            self.profile.as_deref(),
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SessionSearchResult {
    #[serde(default)]
    pub last_active: Option<f64>,
    #[serde(default)]
    pub lineage_root: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub session_started: Option<f64>,
    #[serde(default)]
    pub snippet: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectSessionColor {
    pub session_id: String,
    pub color: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectTreeSnapshot {
    #[serde(default)]
    pub projects: Vec<ProjectTreeProject>,
    #[serde(default)]
    pub active_id: Option<String>,
    #[serde(default)]
    pub scoped_session_ids: Vec<String>,
    #[serde(default)]
    pub errors: Vec<ProjectTreeProfileError>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectListSnapshot {
    #[serde(default)]
    pub projects: Vec<ProjectListEntry>,
    #[serde(default)]
    pub active_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct BotRosterSnapshot {
    #[serde(default)]
    pub bot_mode_protocol: Option<bool>,
    #[serde(default)]
    pub profiles: Vec<BotProfile>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct BotProfile {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

fn parse_bot_roster(value: serde_json::Value) -> Result<BotRosterSnapshot, String> {
    let profiles_value = value
        .get("profiles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Hermes returned an invalid profiles.list response".to_owned())?;
    if profiles_value.len() > 2048 {
        return Err("Hermes returned too many Bot roster rows".into());
    }
    let snapshot: BotRosterSnapshot = serde_json::from_value(value)
        .map_err(|_| "Hermes returned malformed profiles.list data".to_owned())?;
    let mut names = std::collections::BTreeSet::new();
    for profile in &snapshot.profiles {
        if !valid_bounded_tree_id(&profile.name)
            || profile.name.len() > 128
            || !names.insert(profile.name.to_ascii_lowercase())
        {
            return Err("Hermes returned an invalid or duplicate Bot profile name".into());
        }
        for field in [
            profile.description.as_deref(),
            profile.display_name.as_deref(),
            profile.handle.as_deref(),
            profile.title.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if field.len() > 2048 || field.chars().any(char::is_control) {
                return Err("Hermes returned an invalid Bot profile field".into());
            }
        }
    }
    Ok(snapshot)
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectListEntry {
    pub id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub board_slug: Option<String>,
    #[serde(default)]
    pub primary_path: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub created_at: f64,
    #[serde(default)]
    pub folders: Vec<ProjectListFolder>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectListFolder {
    pub path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub added_at: f64,
}

fn parse_project_list(value: serde_json::Value) -> Result<ProjectListSnapshot, String> {
    let projects_value = value
        .get("projects")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Hermes returned an invalid projects.list response".to_owned())?;
    if projects_value.len() > 2048 {
        return Err("Hermes returned too many project list rows".into());
    }
    let snapshot: ProjectListSnapshot = serde_json::from_value(value)
        .map_err(|_| "Hermes returned malformed projects.list data".to_owned())?;
    let mut project_ids = std::collections::BTreeSet::new();
    let mut folder_count = 0usize;
    for project in &snapshot.projects {
        if !valid_bounded_tree_id(&project.id)
            || !project_ids.insert(project.id.as_str())
            || project.slug.len() > 1024
            || project.name.len() > 1024
            || project
                .description
                .as_ref()
                .is_some_and(|value| value.len() > 16_384)
            || project.icon.as_ref().is_some_and(|value| value.len() > 256)
            || project
                .color
                .as_ref()
                .is_some_and(|value| value.len() > 128)
            || project
                .board_slug
                .as_ref()
                .is_some_and(|value| value.len() > 1024)
            || project
                .primary_path
                .as_ref()
                .is_some_and(|value| value.len() > 16_384)
            || !project.created_at.is_finite()
            || project.created_at < 0.0
        {
            return Err("Hermes returned an invalid or duplicate project list entry".into());
        }
        folder_count = folder_count.saturating_add(project.folders.len());
        if folder_count > 10_000 {
            return Err("Hermes returned too many project folders".into());
        }
        if project.folders.iter().any(|folder| {
            folder.path.is_empty()
                || folder.path.len() > 16_384
                || folder
                    .label
                    .as_ref()
                    .is_some_and(|value| value.len() > 1024)
                || !folder.added_at.is_finite()
                || folder.added_at < 0.0
        }) {
            return Err("Hermes returned an invalid project folder".into());
        }
    }
    if snapshot
        .active_id
        .as_deref()
        .is_some_and(|id| !project_ids.contains(id))
    {
        return Err("Hermes projects.list active_id did not match a project".into());
    }
    Ok(snapshot)
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectTreeProfileError {
    pub profile: String,
    pub error: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectTreeProject {
    pub id: String,
    #[serde(default, rename = "mergeKey")]
    pub merge_key: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default, rename = "isDefaultProfile")]
    pub is_default_profile: bool,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default, rename = "isAuto")]
    pub is_auto: bool,
    #[serde(default, rename = "isNoProject")]
    pub is_no_project: bool,
    #[serde(default, rename = "sessionCount")]
    pub session_count: usize,
    #[serde(default, rename = "totalTokens")]
    pub total_tokens: Option<u64>,
    #[serde(default, rename = "totalCostUsd")]
    pub total_cost_usd: Option<f64>,
    #[serde(default, rename = "lastActive")]
    pub last_active: Option<f64>,
    #[serde(default, rename = "previewSessions")]
    pub preview_sessions: Vec<SessionSummary>,
    #[serde(default, rename = "sessionIds")]
    pub session_ids: Vec<String>,
    #[serde(default)]
    pub repos: Vec<ProjectTreeRepo>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectTreeRepo {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default, rename = "sessionCount")]
    pub session_count: usize,
    #[serde(default)]
    pub groups: Vec<ProjectTreeGroup>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectTreeGroup {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default, rename = "isMain")]
    pub is_main: bool,
    #[serde(default, rename = "isHome")]
    pub is_home: bool,
    #[serde(default, rename = "isKanban")]
    pub is_kanban: bool,
    #[serde(default, rename = "isGit")]
    pub is_git: Option<bool>,
    #[serde(default)]
    pub sessions: Vec<SessionSummary>,
}

fn valid_bounded_tree_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control)
}

fn parse_project_tree(value: serde_json::Value) -> Result<ProjectTreeSnapshot, String> {
    let projects_value = value
        .get("projects")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Hermes returned an invalid projects.tree response".to_owned())?;
    if projects_value.len() > 2048 {
        return Err("Hermes returned too many project tree rows".into());
    }
    let snapshot: ProjectTreeSnapshot = serde_json::from_value(value)
        .map_err(|_| "Hermes returned malformed projects.tree data".to_owned())?;
    if snapshot.errors.len() > 256
        || snapshot.errors.iter().any(|entry| {
            !valid_bounded_tree_id(&entry.profile)
                || entry.error.is_empty()
                || entry.error.len() > 4096
        })
    {
        return Err("Hermes returned invalid project profile errors".into());
    }
    let mut project_ids = std::collections::BTreeSet::new();
    let mut total_nodes = snapshot.scoped_session_ids.len();
    if total_nodes > 50_000 {
        return Err("Hermes returned too many scoped project session IDs".into());
    }
    for project in &snapshot.projects {
        if !valid_bounded_tree_id(&project.id)
            || project.label.len() > 1024
            || !project_ids.insert(project.id.as_str())
        {
            return Err("Hermes returned an invalid or duplicate project identity".into());
        }
        total_nodes += project.session_ids.len() + project.preview_sessions.len();
        let mut repo_ids = std::collections::BTreeSet::new();
        for repo in &project.repos {
            if !valid_bounded_tree_id(&repo.id)
                || repo.label.len() > 1024
                || !repo_ids.insert(repo.id.as_str())
            {
                return Err("Hermes returned an invalid or duplicate project repository".into());
            }
            total_nodes += repo.groups.len();
            let mut group_ids = std::collections::BTreeSet::new();
            for group in &repo.groups {
                if !valid_bounded_tree_id(&group.id)
                    || group.label.len() > 1024
                    || !group_ids.insert(group.id.as_str())
                {
                    return Err("Hermes returned an invalid or duplicate project group".into());
                }
                total_nodes += group.sessions.len();
                if total_nodes > 100_000 {
                    return Err("Hermes returned too many project tree entries".into());
                }
            }
        }
    }
    if total_nodes > 100_000 {
        return Err("Hermes returned too many project tree entries".into());
    }
    if snapshot
        .active_id
        .as_deref()
        .is_some_and(|id| !valid_bounded_tree_id(id))
    {
        return Err("Hermes returned an invalid active project identity".into());
    }
    Ok(snapshot)
}

fn validate_all_profile_project_tree(snapshot: &ProjectTreeSnapshot) -> Result<(), String> {
    for project in &snapshot.projects {
        for session in &project.preview_sessions {
            if session.profile.as_deref().is_none_or(|profile| {
                !valid_bounded_tree_id(profile) || profile.eq_ignore_ascii_case("all")
            }) {
                return Err("Hermes all-profile project preview omitted its owning profile".into());
            }
        }
        for repo in &project.repos {
            for group in &repo.groups {
                for session in &group.sessions {
                    if session.profile.as_deref().is_none_or(|profile| {
                        !valid_bounded_tree_id(profile) || profile.eq_ignore_ascii_case("all")
                    }) {
                        return Err(
                            "Hermes all-profile project session omitted its owning profile".into(),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn project_tree_session_colors(snapshot: &ProjectTreeSnapshot) -> Vec<ProjectSessionColor> {
    let mut colors = std::collections::BTreeMap::<String, Option<String>>::new();
    for project in &snapshot.projects {
        for session_id in &project.session_ids {
            colors.insert(session_id.clone(), project.color.clone());
        }
        for session in &project.preview_sessions {
            colors.insert(session.id.clone(), project.color.clone());
        }
        for repo in &project.repos {
            for group in &repo.groups {
                for session in &group.sessions {
                    colors.insert(
                        session.id.clone(),
                        group.color.clone().or_else(|| project.color.clone()),
                    );
                }
            }
        }
    }
    colors
        .into_iter()
        .map(|(session_id, color)| ProjectSessionColor { session_id, color })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveSessionStatusKind {
    Idle,
    Starting,
    Waiting,
    Working,
    Streaming,
    Resuming,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveSessionStatus {
    pub runtime_session_id: String,
    pub stored_session_id: String,
    pub status: ActiveSessionStatusKind,
    pub last_active: Option<f64>,
}

fn parse_active_session_statuses(
    value: serde_json::Value,
) -> Result<Vec<ActiveSessionStatus>, String> {
    let rows = value
        .get("sessions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Hermes returned an invalid session.active_list response".to_owned())?;
    if rows.len() > 4096 {
        return Err("Hermes returned too many active session rows".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut statuses = Vec::with_capacity(rows.len());
    for row in rows {
        let runtime_session_id = row
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 512 && !id.chars().any(char::is_control))
            .ok_or_else(|| "Hermes returned an invalid active runtime session ID".to_owned())?;
        let stored_session_id = row
            .get("session_key")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 512 && !id.chars().any(char::is_control))
            .ok_or_else(|| "Hermes returned an invalid stored session ID".to_owned())?;
        if !seen.insert(runtime_session_id.to_owned()) {
            return Err("Hermes returned duplicate active runtime session IDs".into());
        }
        let status = match row
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
        {
            "idle" => ActiveSessionStatusKind::Idle,
            "starting" => ActiveSessionStatusKind::Starting,
            "waiting" => ActiveSessionStatusKind::Waiting,
            "working" => ActiveSessionStatusKind::Working,
            "streaming" => ActiveSessionStatusKind::Streaming,
            "resuming" => ActiveSessionStatusKind::Resuming,
            _ => return Err("Hermes returned an unknown active session status".into()),
        };
        let last_active = row
            .get("last_active")
            .and_then(serde_json::Value::as_f64)
            .filter(|timestamp| timestamp.is_finite() && *timestamp >= 0.0);
        statuses.push(ActiveSessionStatus {
            runtime_session_id: runtime_session_id.to_owned(),
            stored_session_id: stored_session_id.to_owned(),
            status,
            last_active,
        });
    }
    Ok(statuses)
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkillInfo {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    pub name: String,
    #[serde(default)]
    pub usage: Option<u64>,
    #[serde(default)]
    pub provenance: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ToolsetInfo {
    #[serde(default)]
    pub configured: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub label: String,
    pub name: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ToolsetEnabledResponse {
    ok: bool,
    name: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct SessionMutationSnapshot {
    session_id: String,
    exists: bool,
    runtime_revision: u64,
    runtime_generation: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SessionMutationReceipt {
    ok: bool,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CapabilitiesSnapshot {
    pub skills: Vec<SkillInfo>,
    pub toolsets: Vec<ToolsetInfo>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MessagingPlatformsSnapshot {
    pub platforms: Vec<MessagingPlatformInfo>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronJobInfo {
    pub id: String,
    /// `None` means the scheduler follows its configured default (including JSON null/omission).
    #[serde(default)]
    pub catch_up: Option<bool>,
    #[serde(default)]
    pub no_agent: bool,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub schedule_display: Option<String>,
    #[serde(default)]
    pub schedule: Option<CronJobSchedule>,
    #[serde(default)]
    pub deliver: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub latest_execution: Option<CronJobExecutionInfo>,
    #[serde(default)]
    pub live_execution: Option<CronJobExecutionInfo>,
}

/// Fields accepted by the pinned Desktop `POST /api/cron/jobs` form.
/// `None` fields are omitted from JSON, matching the Desktop's optional controls.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CronJobCreatePayload {
    pub prompt: String,
    pub schedule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catch_up: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Optional fields accepted inside the pinned Desktop `PUT` request's `updates` object.
/// The nested options on `model` and `provider` preserve omitted-vs-null semantics:
/// `None` omits a field; `Some(None)` explicitly clears an override.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CronJobUpdatePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catch_up: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<Option<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronDeliveryTarget {
    pub id: String,
    pub name: String,
    pub home_target_set: bool,
    #[serde(default)]
    pub home_env_var: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronDeliveryTargetsSnapshot {
    pub targets: Vec<CronDeliveryTarget>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronJobDeleteResult {
    pub ok: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModelOptionsSnapshot {
    #[serde(default)]
    pub providers: Vec<ModelOptionProvider>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModelOptionProvider {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub authenticated: Option<bool>,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronBlueprint {
    pub key: String,
    pub title: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub fields: Vec<CronBlueprintField>,
    pub command: String,
    #[serde(rename = "appUrl")]
    pub app_url: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub plugin: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronBlueprintField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub label: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    pub optional: bool,
    #[serde(default)]
    pub strict: Option<bool>,
    #[serde(default)]
    pub help: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CronBlueprintInstantiatePayload {
    pub blueprint: String,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronJobSchedule {
    #[serde(default)]
    pub display: Option<String>,
    #[serde(default)]
    pub expr: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronJobRunInfo {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub started_at: Option<f64>,
    #[serde(default)]
    pub last_active: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronJobExecutionInfo {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub claimed_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CronCancelResult {
    pub job_id: String,
    pub cancelled: bool,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub execution_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PairingSnapshot {
    #[serde(default)]
    pub approved: Vec<PairingUser>,
    #[serde(default)]
    pub pending: Vec<PairingUser>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PairingUser {
    #[serde(default)]
    pub age_minutes: Option<f64>,
    pub platform: String,
    #[serde(default)]
    pub request_id: Option<String>,
    pub user_id: String,
    #[serde(default)]
    pub user_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairingMutationKind {
    Approve,
    Revoke,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PairingMutationReceipt {
    pub ok: bool,
    #[serde(default)]
    pub user: Option<PairingUser>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TelegramOnboardingStart {
    pub deep_link: String,
    pub expires_at: String,
    pub pairing_id: String,
    pub qr_payload: String,
    pub suggested_username: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TelegramOnboardingStatus {
    Waiting {
        expires_at: String,
    },
    Ready {
        bot_username: Option<String>,
        expires_at: String,
        #[serde(default)]
        owner_user_id: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct TelegramOnboardingApplyReceipt {
    pub ok: bool,
    pub platform: String,
    pub needs_restart: bool,
    #[serde(default)]
    pub bot_username: Option<String>,
    #[serde(default)]
    pub restart_action: Option<String>,
    #[serde(default)]
    pub restart_error: Option<String>,
    #[serde(default)]
    pub restart_pid: Option<u32>,
    #[serde(default)]
    pub restart_started: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TelegramOnboardingCancelReceipt {
    pub ok: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GatewayActionReceipt {
    pub name: String,
    pub ok: bool,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub action_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GatewayActionStatus {
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub lines: Vec<String>,
    pub name: String,
    #[serde(default)]
    pub pid: Option<u32>,
    pub running: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterStatus {
    pub active_sessions: usize,
    pub gateway_running: bool,
    pub version: String,
    #[serde(default)]
    pub gateway_heartbeat_stale_s: Option<f64>,
    #[serde(default)]
    pub gateway_exit_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterLogs {
    pub file: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterSystemSnapshot {
    pub status: CommandCenterStatus,
    pub logs: CommandCenterLogs,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageSnapshot {
    #[serde(default)]
    pub by_model: Vec<CommandCenterUsageModel>,
    #[serde(default)]
    pub daily: Vec<CommandCenterUsageDay>,
    pub period_days: usize,
    #[serde(default)]
    pub skills: CommandCenterUsageSkills,
    #[serde(default)]
    pub tools: Option<Vec<CommandCenterUsageTool>>,
    pub totals: CommandCenterUsageTotals,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageDay {
    pub actual_cost: f64,
    pub api_calls: u64,
    pub cache_read_tokens: u64,
    pub day: String,
    pub estimated_cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub sessions: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageModel {
    pub api_calls: u64,
    pub estimated_cost: f64,
    pub input_tokens: u64,
    pub model: String,
    pub output_tokens: u64,
    pub sessions: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageSkills {
    #[serde(default)]
    pub summary: CommandCenterUsageSkillsSummary,
    #[serde(default)]
    pub top_skills: Vec<CommandCenterUsageSkill>,
}

impl Default for CommandCenterUsageSkills {
    fn default() -> Self {
        Self {
            summary: CommandCenterUsageSkillsSummary::default(),
            top_skills: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct CommandCenterUsageSkillsSummary {
    pub distinct_skills_used: u64,
    pub total_skill_actions: u64,
    pub total_skill_edits: u64,
    pub total_skill_loads: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageSkill {
    pub last_used_at: Option<f64>,
    pub manage_count: u64,
    pub percentage: f64,
    pub skill: String,
    pub total_count: u64,
    pub view_count: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageTool {
    pub count: u64,
    pub percentage: f64,
    pub tool: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterUsageTotals {
    pub total_actual_cost: f64,
    pub total_api_calls: Option<u64>,
    pub total_cache_read: Option<u64>,
    pub total_estimated_cost: f64,
    pub total_input: Option<u64>,
    pub total_output: Option<u64>,
    pub total_reasoning: Option<u64>,
    pub total_sessions: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionExportTranscript {
    pub resolved_session_id: String,
    pub messages: Vec<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCenterSystemAction {
    RestartGateway,
    UpdateHermes,
}

impl CommandCenterSystemAction {
    fn endpoint(self) -> &'static str {
        match self {
            Self::RestartGateway => "/api/gateway/restart",
            Self::UpdateHermes => "/api/hermes/update",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandCenterSystemOperation {
    StartAction { action: CommandCenterSystemAction },
    LoadActionStatus { name: String },
}

#[derive(Clone, Debug)]
pub enum CommandCenterSystemActionResult {
    ActionStarted(GatewayActionReceipt),
    ActionStatus(GatewayActionStatus),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCenterMaintenanceAction {
    Doctor,
    SecurityAudit,
    Backup,
    Curator,
}

impl CommandCenterMaintenanceAction {
    fn endpoint(self) -> &'static str {
        match self {
            Self::Doctor => "/api/ops/doctor",
            Self::SecurityAudit => "/api/ops/security-audit",
            Self::Backup => "/api/ops/backup",
            Self::Curator => "/api/curator/run",
        }
    }

    fn action_name(self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::SecurityAudit => "security-audit",
            Self::Backup => "backup",
            Self::Curator => "curator-run",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCenterMemoryResetTarget {
    Memory,
    User,
}

impl CommandCenterMemoryResetTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::User => "user",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandCenterMaintenanceOperation {
    LoadSnapshot,
    StartAction {
        action: CommandCenterMaintenanceAction,
    },
    LoadActionStatus {
        name: String,
    },
    SetCuratorPaused {
        paused: bool,
    },
    ResetMemory {
        target: CommandCenterMemoryResetTarget,
    },
    DebugShare,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CommandCenterMaintenanceCuratorStatus {
    pub enabled: bool,
    pub paused: bool,
    #[serde(default)]
    pub interval_hours: Option<f64>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub min_idle_hours: Option<f64>,
    #[serde(default)]
    pub stale_after_days: Option<f64>,
    #[serde(default)]
    pub archive_after_days: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommandCenterMemoryProvider {
    pub name: String,
    pub description: String,
    pub available: bool,
    pub configured: bool,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommandCenterMemoryFiles {
    pub memory: u64,
    pub user: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommandCenterMaintenanceMemoryStatus {
    pub active: String,
    #[serde(default)]
    pub providers: Vec<CommandCenterMemoryProvider>,
    pub builtin_files: CommandCenterMemoryFiles,
}

#[derive(Clone, Debug)]
pub struct CommandCenterMaintenanceSnapshot {
    pub curator: Result<CommandCenterMaintenanceCuratorStatus, String>,
    pub memory: Result<CommandCenterMaintenanceMemoryStatus, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommandCenterMemoryResetReceipt {
    pub ok: bool,
    #[serde(default)]
    pub deleted: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommandCenterCuratorPausedReceipt {
    pub ok: bool,
    pub paused: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CommandCenterDebugShareResponse {
    pub ok: bool,
    #[serde(default)]
    pub urls: BTreeMap<String, String>,
    #[serde(default)]
    pub failures: BTreeMap<String, String>,
    #[serde(default)]
    pub redacted: bool,
    #[serde(default)]
    pub auto_delete_seconds: Option<u64>,
}

#[derive(Clone, Debug)]
pub enum CommandCenterMaintenanceResult {
    Snapshot(CommandCenterMaintenanceSnapshot),
    ActionStarted(GatewayActionReceipt),
    ActionStatus(GatewayActionStatus),
    CuratorPaused(CommandCenterCuratorPausedReceipt),
    MemoryReset(CommandCenterMemoryResetReceipt),
    DebugShare(CommandCenterDebugShareResponse),
}

#[derive(Clone, Debug, Deserialize)]
pub struct MessagingPlatformInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub configured: bool,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub gateway_running: bool,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub docs_url: String,
    #[serde(default)]
    pub relay_fronted: bool,
    #[serde(default)]
    pub relay_exclusive: bool,
    #[serde(default)]
    pub disabled_by_relay: bool,
    #[serde(default)]
    pub env_vars: Vec<MessagingEnvVarInfo>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MessagingEnvVarInfo {
    #[serde(default)]
    pub advanced: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_list: bool,
    #[serde(default)]
    pub is_password: bool,
    #[serde(default)]
    pub is_set: bool,
    pub key: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub redacted_value: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Clone, Default, serde::Serialize)]
pub struct MessagingPlatformUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub clear_env: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MessagingPlatformUpdateResult {
    pub ok: bool,
    pub platform: String,
    #[serde(default)]
    pub hot_served: Option<bool>,
}

#[derive(Deserialize)]
struct DisplayTimestampsConfig {
    #[serde(default)]
    display: Option<DisplayTimestampsSection>,
}

#[derive(Deserialize)]
struct DisplayTimestampsSection {
    #[serde(default)]
    timestamps: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorCatalogEntry {
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorListEntry {
    pub connector: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub connection_status: Option<String>,
    #[serde(default)]
    pub status_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorAccountEntry {
    pub connection_id: String,
    pub connector: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorsSnapshot {
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub connectors: Vec<ConnectorListEntry>,
    #[serde(default)]
    pub catalog: Vec<ConnectorCatalogEntry>,
    #[serde(default)]
    pub accounts: Vec<ConnectorAccountEntry>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AgentPluginServerInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub sentence: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AgentPluginInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub portable: bool,
    #[serde(default)]
    pub catalog_name: Option<String>,
    #[serde(default)]
    pub catalog_tier: Option<String>,
    #[serde(default)]
    pub installed_sha: Option<String>,
    #[serde(default)]
    pub catalog_sha: Option<String>,
    #[serde(default)]
    pub catalog_version: Option<String>,
    #[serde(default)]
    pub update_available: Option<bool>,
    #[serde(default)]
    pub update_target: Option<String>,
    #[serde(default)]
    pub has_desktop_half: bool,
    #[serde(default)]
    pub servers: Vec<AgentPluginServerInfo>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AgentPluginsSnapshot {
    #[serde(default)]
    pub plugins: Vec<AgentPluginInfo>,
    #[serde(default)]
    pub user_count: usize,
    #[serde(default)]
    pub bundled_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct AgentPluginAutoUpdateSettings {
    pub auto_apply: bool,
    pub check_hours: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AgentPluginUpdateAllRow {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AgentPluginToggleReceipt {
    pub plugin: AgentPluginInfo,
    pub restart_required: bool,
    pub gateway_reloaded: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AgentPluginInspection {
    pub agent: bool,
    pub desktop: bool,
    pub agent_name: Option<String>,
    pub desktop_name: Option<String>,
    pub sha: Option<String>,
    pub warnings: Vec<String>,
    pub insecure: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AgentPluginInstallReceipt {
    pub plugin_name: String,
    pub enabled: bool,
    pub warnings: Vec<String>,
    pub missing_env: Vec<String>,
    pub skills: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AgentPluginScanFinding {
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub line: u64,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug)]
pub enum AgentPluginUpdateOutcome {
    Applied {
        unchanged: bool,
    },
    Consent {
        sha: String,
        delta_lines: Vec<String>,
    },
    Scan {
        verdict: String,
        findings: Vec<AgentPluginScanFinding>,
    },
}

#[derive(Clone, Debug)]
pub struct ArtifactFileDownloadRequest {
    pub request_id: String,
    pub path: String,
    pub session_id: String,
    pub profile: String,
}

#[derive(Clone, Debug)]
pub struct ArtifactImagePreviewRequest {
    pub request_id: String,
    pub artifact_id: String,
    pub path: String,
    pub session_id: String,
    pub profile: String,
}

#[derive(Clone, Debug)]
pub struct ConnectedGateway {
    pub endpoint: GatewayEndpoint,
    pub sessions: Vec<SessionSummary>,
}

#[derive(Clone, Debug)]
pub struct SessionTranscript {
    pub session_id: String,
    pub profile: Option<String>,
    pub runtime_session_id: String,
    pub message_count: usize,
    pub messages: Vec<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct OlderTranscriptPage {
    pub messages: Vec<serde_json::Value>,
    pub has_more: bool,
    pub replace_all: bool,
}

#[derive(Clone, Debug)]
pub struct CreatedSession {
    pub request_id: String,
    pub runtime_session_id: String,
    pub stored_session_id: String,
}

#[derive(Clone, Debug)]
pub struct ClarifyLockReceipt {
    pub expired: bool,
}

#[derive(Clone, Debug)]
pub enum PromptSubmitReceipt {
    Admitted {
        admission_id: String,
        status: String,
        execution_generation: Option<u64>,
    },
    VoiceStopped,
}

#[derive(Debug)]
pub enum GatewayEvent {
    SessionResumed {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        result: Result<SessionTranscript, String>,
    },
    OlderTranscriptLoaded {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        offset: usize,
        result: Result<OlderTranscriptPage, String>,
    },
    ActiveTranscriptRefreshed {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        endpoint_profile_id: String,
        runtime_session_id: String,
        result: Result<OlderTranscriptPage, String>,
    },
    PromptSubmitted {
        session_id: String,
        submission_id: String,
        result: Result<PromptSubmitReceipt, String>,
    },
    UnknownPromptResolved {
        session_id: String,
        admission_id: String,
        result: Result<String, String>,
    },
    SessionInterrupted {
        request_id: String,
        session_id: String,
        result: Result<bool, String>,
    },
    ApprovalResponded {
        request_id: String,
        session_id: String,
        prompt_id: String,
        choice: String,
        result: Result<(), String>,
    },
    ClarifyResponded {
        request_id: String,
        session_id: String,
        prompt_id: String,
        answer: String,
        result: Result<(), String>,
    },
    ClarifyLocked {
        request_id: String,
        clarify_request_id: String,
        question_id: String,
        result: Result<ClarifyLockReceipt, String>,
    },
    ServerRequestReceived {
        id: String,
        method: String,
        params: serde_json::Value,
    },
    ServerRequestAnswered {
        id: String,
        result: Result<(), String>,
    },
    SessionCreated(Result<CreatedSession, String>),
    ServerEvent(serde_json::Value),
    SessionsSearched {
        request_id: String,
        query: String,
        result: Result<Vec<SessionSearchResult>, String>,
    },
    SessionsLoaded(Result<Vec<SessionSummary>, String>),
    SessionsRefreshed {
        request_id: String,
        profile_id: String,
        result: Result<CommandCenterSessionSnapshot, String>,
    },
    CommandPaletteSessionsLoaded {
        request_id: String,
        result: Result<Vec<SessionSummary>, String>,
    },
    ArchivedSessionsLoaded {
        request_id: String,
        result: Result<Vec<SessionSummary>, String>,
    },
    ArtifactsLoaded {
        request_id: String,
        result: Result<ArtifactIndexSnapshot, String>,
    },
    ArtifactFileDownloaded {
        request_id: String,
        result: Result<Option<String>, String>,
    },
    ArtifactImagePreviewLoaded {
        request_id: String,
        artifact_id: String,
        result: Result<FilePreviewContent, String>,
    },
    CapabilitiesLoaded {
        request_id: String,
        result: Result<CapabilitiesSnapshot, String>,
    },
    MessagingPlatformsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<MessagingPlatformsSnapshot, String>,
    },
    CronJobsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<Vec<CronJobInfo>, String>,
    },
    ModelOptionsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<ModelOptionsSnapshot, String>,
    },
    CronDeliveryTargetsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<Vec<CronDeliveryTarget>, String>,
    },
    CronBlueprintsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<Vec<CronBlueprint>, String>,
    },
    CronBlueprintInstantiated {
        request_id: String,
        profile: Option<String>,
        blueprint: String,
        result: Result<CronJobInfo, String>,
    },
    CronJobCreated {
        request_id: String,
        profile: Option<String>,
        result: Result<CronJobInfo, String>,
    },
    CronJobUpdated {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        result: Result<CronJobInfo, String>,
    },
    CronJobDeleted {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        result: Result<CronJobDeleteResult, String>,
    },
    CronJobRunsLoaded {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        result: Result<Vec<CronJobRunInfo>, String>,
    },
    CronJobPauseChanged {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        paused: bool,
        result: Result<CronJobInfo, String>,
    },
    CronJobTriggered {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        result: Result<CronJobInfo, String>,
    },
    CronJobRunCancelled {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        result: Result<CronCancelResult, String>,
    },
    MessagingPlatformUpdated {
        request_id: String,
        profile: Option<String>,
        platform_id: String,
        enabled: bool,
        result: Result<MessagingPlatformUpdateResult, String>,
    },
    MessagingPlatformConfigUpdated {
        request_id: String,
        profile: Option<String>,
        platform_id: String,
        result: Result<MessagingPlatformUpdateResult, String>,
    },
    PairingLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<PairingSnapshot, String>,
    },
    PairingMutated {
        request_id: String,
        profile: Option<String>,
        platform: String,
        user_id: String,
        kind: PairingMutationKind,
        result: Result<PairingMutationReceipt, String>,
    },
    TelegramOnboardingStarted {
        request_id: String,
        profile: Option<String>,
        result: Result<TelegramOnboardingStart, String>,
    },
    TelegramOnboardingStatusLoaded {
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
        result: Result<TelegramOnboardingStatus, String>,
    },
    TelegramOnboardingApplied {
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
        result: Result<TelegramOnboardingApplyReceipt, String>,
    },
    TelegramOnboardingCanceled {
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
        result: Result<TelegramOnboardingCancelReceipt, String>,
    },
    GatewayRestartStarted {
        request_id: String,
        profile: Option<String>,
        result: Result<GatewayActionReceipt, String>,
    },
    GatewayActionStatusLoaded {
        request_id: String,
        profile: Option<String>,
        action_name: String,
        result: Result<GatewayActionStatus, String>,
    },
    CommandCenterSystemLoaded {
        request_id: String,
        profile: Option<String>,
        log_file: String,
        log_level: String,
        result: Result<CommandCenterSystemSnapshot, String>,
    },
    CommandCenterSystemAction {
        request_id: String,
        profile: Option<String>,
        operation: CommandCenterSystemOperation,
        result: Result<CommandCenterSystemActionResult, String>,
    },
    CommandCenterUsageLoaded {
        request_id: String,
        profile: Option<String>,
        period_days: usize,
        result: Result<CommandCenterUsageSnapshot, String>,
    },
    CommandCenterMaintenance {
        request_id: String,
        profile: Option<String>,
        operation: CommandCenterMaintenanceOperation,
        result: Result<CommandCenterMaintenanceResult, String>,
    },
    DisplayTimestampsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<bool, String>,
    },
    RepoScanConfigLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<RepoScanPolicy, String>,
    },
    RepoScanConfigSaved {
        request_id: String,
        profile: Option<String>,
        result: Result<RepoScanPolicy, String>,
    },
    RepoDiscoveryCompleted {
        request_id: String,
        profile: String,
        result: Result<(), String>,
    },
    ConnectorsLoaded {
        request_id: String,
        result: Result<ConnectorsSnapshot, String>,
    },
    AgentPluginsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<AgentPluginsSnapshot, String>,
    },
    ProjectSessionColorsLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<Vec<ProjectSessionColor>, String>,
    },
    ProjectTreeLoaded {
        request_id: String,
        profile: String,
        result: Result<ProjectTreeSnapshot, String>,
    },
    ProjectListLoaded {
        request_id: String,
        profile: String,
        result: Result<ProjectListSnapshot, String>,
    },
    BotRosterLoaded {
        request_id: String,
        profile: String,
        result: Result<BotRosterSnapshot, String>,
    },
    ProjectDeleted {
        request_id: String,
        project_id: String,
        profile: String,
        result: Result<ProjectListSnapshot, String>,
    },
    ProjectSessionsLoaded {
        request_id: String,
        project_id: String,
        profile: String,
        result: Result<Option<ProjectTreeProject>, String>,
    },
    ActiveSessionStatusesLoaded {
        request_id: String,
        profile: String,
        result: Result<Vec<ActiveSessionStatus>, String>,
    },
    AgentPluginAutoUpdateLoaded {
        request_id: String,
        profile: Option<String>,
        result: Result<AgentPluginAutoUpdateSettings, String>,
    },
    AgentPluginAutoUpdateSet {
        request_id: String,
        profile: Option<String>,
        enabled: bool,
        result: Result<AgentPluginAutoUpdateSettings, String>,
    },
    AgentPluginUpdateAll {
        request_id: String,
        profile: Option<String>,
        result: Result<Vec<AgentPluginUpdateAllRow>, String>,
    },
    AgentPluginToggled {
        request_id: String,
        key: String,
        enabled: bool,
        profile: Option<String>,
        result: Result<AgentPluginToggleReceipt, String>,
    },
    AgentPluginRemoved {
        request_id: String,
        name: String,
        profile: Option<String>,
        result: Result<(), String>,
    },
    AgentPluginInspected {
        request_id: String,
        catalog_name: String,
        profile: Option<String>,
        result: Result<AgentPluginInspection, String>,
    },
    AgentPluginInstalled {
        request_id: String,
        catalog_name: String,
        profile: Option<String>,
        result: Result<AgentPluginInstallReceipt, String>,
    },
    AgentPluginUpdated {
        request_id: String,
        name: String,
        profile: Option<String>,
        accept_capabilities: bool,
        force: bool,
        result: Result<AgentPluginUpdateOutcome, String>,
    },
    ToolsetEnabled {
        request_id: String,
        name: String,
        result: Result<bool, String>,
    },
    SessionPinUpdated {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        pinned: bool,
        result: Result<(), String>,
    },
    SessionUnreadUpdated {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        unread: bool,
        result: Result<(), String>,
    },
    SessionArchiveUpdated {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        archived: bool,
        result: Result<(), String>,
    },
    SessionRenamed {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        result: Result<String, String>,
    },
    SessionDeleted {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        result: Result<(), String>,
    },
    SessionExportLoaded {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        result: Result<SessionExportTranscript, String>,
    },
    Disconnected(String),
}

enum GatewayCommand {
    RecordDiscoveredRepos {
        request_id: String,
        profile: String,
        policy: RepoScanPolicy,
        repos: Vec<DiscoveredRepo>,
    },
    DiscoverRemoteRepos {
        request_id: String,
        profile: String,
    },
    LoadProjectList {
        request_id: String,
        profile: String,
    },
    LoadBotRoster {
        request_id: String,
        profile: String,
    },
    DeleteProject {
        request_id: String,
        project_id: String,
        profile: String,
    },
    LoadProjectTree {
        request_id: String,
        profile: String,
        preview_limit: usize,
    },
    LoadProjectSessions {
        request_id: String,
        project_id: String,
        profile: String,
    },
    LoadActiveSessionStatuses {
        request_id: String,
        profile: String,
    },
    LoadProjectSessionColors {
        request_id: String,
        profile: Option<String>,
    },
    LoadAgentPlugins {
        request_id: String,
        profile: Option<String>,
    },
    LoadAgentPluginAutoUpdate {
        request_id: String,
        profile: Option<String>,
    },
    SetAgentPluginAutoUpdate {
        request_id: String,
        profile: Option<String>,
        enabled: bool,
    },
    SetAgentPluginEnabled {
        request_id: String,
        key: String,
        enabled: bool,
        profile: Option<String>,
    },
    RemoveAgentPlugin {
        request_id: String,
        name: String,
        profile: Option<String>,
    },
    InspectCatalogPlugin {
        request_id: String,
        catalog_name: String,
        identifier: String,
        profile: Option<String>,
    },
    InstallCatalogPlugin {
        request_id: String,
        catalog_name: String,
        identifier: String,
        inspected_sha: String,
        profile: Option<String>,
    },
    UpdateAgentPlugin {
        request_id: String,
        name: String,
        profile: Option<String>,
        accept_capabilities: bool,
        force: bool,
    },
    ResumeSession {
        request_id: String,
        session_id: String,
        profile: Option<String>,
    },
    CreateSession {
        request_id: String,
        params: serde_json::Value,
    },
    RenameRuntimeSession {
        request_id: String,
        stored_session_id: String,
        runtime_session_id: String,
        profile: Option<String>,
        title: String,
        retry_only: bool,
    },
    SubmitPrompt {
        session_id: String,
        profile: Option<String>,
        submission_id: String,
        text: String,
    },
    ResolveUnknownPrompt {
        session_id: String,
        profile: Option<String>,
        admission_id: String,
        execution_generation: u64,
    },
    InterruptSession {
        request_id: String,
        session_id: String,
        profile: Option<String>,
    },
    RespondApproval {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        execution_generation: u64,
        prompt_id: String,
        choice: String,
    },
    RespondClarify {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        execution_generation: u64,
        prompt_id: String,
        answer: String,
    },
    LockClarify {
        request_id: String,
        clarify_request_id: String,
        question_id: String,
        profile: Option<String>,
        answer: Option<String>,
    },
    RespondServerRequest {
        id: String,
        response: Result<serde_json::Value, (i64, String)>,
    },
    LoadConnectors {
        request_id: String,
    },
}

enum GatewayHttpCommand {
    LoadOlderTranscript {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        offset: usize,
    },
    RefreshActiveTranscript {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        runtime_session_id: String,
    },
    LoadSessions,
    RefreshSessions {
        request_id: String,
        limit: usize,
    },
    LoadArchivedSessions {
        request_id: String,
    },
    LoadCommandPaletteSessions {
        request_id: String,
    },
    SearchSessions {
        request_id: String,
        query: String,
    },
    LoadCapabilities {
        request_id: String,
    },
    LoadMessagingPlatforms {
        request_id: String,
        profile: Option<String>,
    },
    LoadCronJobs {
        request_id: String,
        profile: Option<String>,
    },
    LoadModelOptions {
        request_id: String,
        profile: Option<String>,
    },
    LoadCronDeliveryTargets {
        request_id: String,
        profile: Option<String>,
    },
    LoadCronBlueprints {
        request_id: String,
        profile: Option<String>,
    },
    InstantiateCronBlueprint {
        request_id: String,
        profile: Option<String>,
        payload: CronBlueprintInstantiatePayload,
    },
    CreateCronJob {
        request_id: String,
        profile: Option<String>,
        payload: CronJobCreatePayload,
    },
    UpdateCronJob {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        updates: CronJobUpdatePayload,
    },
    DeleteCronJob {
        request_id: String,
        profile: Option<String>,
        job_id: String,
    },
    LoadCronJobRuns {
        request_id: String,
        profile: Option<String>,
        job_id: String,
    },
    SetCronJobPaused {
        request_id: String,
        profile: Option<String>,
        job_id: String,
        paused: bool,
    },
    CancelCronJobRun {
        request_id: String,
        profile: Option<String>,
        job_id: String,
    },
    SetMessagingPlatformEnabled {
        request_id: String,
        profile: Option<String>,
        platform_id: String,
        enabled: bool,
    },
    UpdateMessagingPlatformConfig {
        request_id: String,
        profile: Option<String>,
        platform_id: String,
        update: MessagingPlatformUpdate,
    },
    LoadPairing {
        request_id: String,
        profile: Option<String>,
    },
    MutatePairing {
        request_id: String,
        profile: Option<String>,
        platform: String,
        user_id: String,
        kind: PairingMutationKind,
    },
    StartTelegramOnboarding {
        request_id: String,
        profile: Option<String>,
        bot_name: Option<String>,
    },
    LoadTelegramOnboardingStatus {
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
    },
    ApplyTelegramOnboarding {
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
        allowed_user_ids: Vec<String>,
    },
    CancelTelegramOnboarding {
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
    },
    RestartGateway {
        request_id: String,
        profile: Option<String>,
    },
    LoadGatewayActionStatus {
        request_id: String,
        action_name: String,
        profile: Option<String>,
    },
    LoadCommandCenterSystem {
        request_id: String,
        profile: Option<String>,
        log_file: String,
        log_level: String,
    },
    CommandCenterSystemAction {
        request_id: String,
        profile: Option<String>,
        operation: CommandCenterSystemOperation,
    },
    LoadCommandCenterUsage {
        request_id: String,
        profile: Option<String>,
        period_days: usize,
    },
    CommandCenterMaintenance {
        request_id: String,
        profile: Option<String>,
        operation: CommandCenterMaintenanceOperation,
    },
    LoadDisplayTimestamps {
        request_id: String,
        profile: Option<String>,
    },
    LoadRepoScanConfig {
        request_id: String,
        profile: Option<String>,
    },
    SaveRepoScanConfig {
        request_id: String,
        profile: Option<String>,
        policy: RepoScanPolicy,
    },
    SetToolsetEnabled {
        request_id: String,
        name: String,
        enabled: bool,
    },
    SetSessionPinned {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        pinned: bool,
    },
    SetSessionUnread {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        unread: bool,
    },
    SetSessionArchived {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        archived: bool,
    },
    RenameSession {
        request_id: String,
        session_id: String,
        profile: Option<String>,
        title: String,
    },
    DeleteSession {
        request_id: String,
        session_id: String,
        profile: Option<String>,
    },
    ExportSession {
        request_id: String,
        session_id: String,
        profile: Option<String>,
    },
}

struct ProjectTreeHttpCommand {
    request_id: String,
    preview_limit: usize,
}

type GatewaySocket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

pub struct GatewayDiscovery {
    status_receiver: Receiver<Result<ConnectedGateway, String>>,
    event_receiver: Receiver<GatewayEvent>,
    command_sender: mpsc::Sender<GatewayCommand>,
    plugin_update_all_sender: mpsc::Sender<AgentPluginUpdateAllCommand>,
    http_sender: mpsc::Sender<GatewayHttpCommand>,
    project_tree_sender: mpsc::Sender<ProjectTreeHttpCommand>,
    cron_trigger_sender: mpsc::SyncSender<CronTriggerCommand>,
    artifact_sender: mpsc::Sender<String>,
    artifact_download_sender: mpsc::Sender<ArtifactFileDownloadRequest>,
    artifact_image_sender: mpsc::Sender<ArtifactImagePreviewRequest>,
}

#[cfg(test)]
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TestGatewayCommand {
    ResumeSession {
        request_id: String,
        session_id: String,
        profile: Option<String>,
    },
    SubmitPrompt,
    Other,
}

struct CronTriggerCommand {
    request_id: String,
    profile: Option<String>,
    job_id: String,
}

struct AgentPluginUpdateAllCommand {
    request_id: String,
    profile: Option<String>,
}

impl GatewayDiscovery {
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn for_test_session_resume() -> (
        Self,
        Receiver<TestGatewayCommand>,
        Receiver<(String, usize)>,
    ) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (observed_command_sender, observed_command_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                let observed = match command {
                    GatewayCommand::ResumeSession {
                        request_id,
                        session_id,
                        profile,
                    } => TestGatewayCommand::ResumeSession {
                        request_id,
                        session_id,
                        profile,
                    },
                    GatewayCommand::SubmitPrompt { .. } => TestGatewayCommand::SubmitPrompt,
                    _ => TestGatewayCommand::Other,
                };
                if observed_command_sender.send(observed).is_err() {
                    break;
                }
            }
        });
        let (refresh_sender, refresh_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = http_receiver.recv() {
                if let GatewayHttpCommand::RefreshSessions { request_id, limit } = command
                    && refresh_sender.send((request_id, limit)).is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            observed_command_receiver,
            refresh_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_session_refresh() -> (Self, Receiver<(String, usize)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, _command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (refresh_sender, refresh_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = http_receiver.recv() {
                if let GatewayHttpCommand::RefreshSessions { request_id, limit } = command
                    && refresh_sender.send((request_id, limit)).is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            refresh_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_active_status_refresh() -> (Self, Receiver<(String, String)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (refresh_sender, refresh_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                if let GatewayCommand::LoadActiveSessionStatuses {
                    request_id,
                    profile,
                } = command
                    && refresh_sender.send((request_id, profile)).is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            refresh_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_project_sessions() -> (Self, Receiver<(String, String, String)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                if let GatewayCommand::LoadProjectSessions {
                    request_id,
                    project_id,
                    profile,
                } = command
                    && request_sender
                        .send((request_id, project_id, profile))
                        .is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            request_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_project_tree() -> (Self, Receiver<(String, String, usize)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                if let GatewayCommand::LoadProjectTree {
                    request_id,
                    profile,
                    preview_limit,
                } = command
                    && request_sender
                        .send((request_id, profile, preview_limit))
                        .is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            request_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_all_profile_project_tree() -> (Self, Receiver<(String, usize)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, _command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, project_tree_receiver) =
            mpsc::channel::<ProjectTreeHttpCommand>();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = project_tree_receiver.recv() {
                if request_sender
                    .send((command.request_id, command.preview_limit))
                    .is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            request_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_project_list() -> (Self, Receiver<(String, String)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                if let GatewayCommand::LoadProjectList {
                    request_id,
                    profile,
                } = command
                    && request_sender.send((request_id, profile)).is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            request_receiver,
        )
    }

    #[cfg(test)]
    fn for_test_repo_discovery_commands() -> (Self, Receiver<GatewayCommand>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            command_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_project_delete() -> (Self, Receiver<(String, String, String)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, _http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (request_sender, request_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                if let GatewayCommand::DeleteProject {
                    request_id,
                    project_id,
                    profile,
                } = command
                    && request_sender
                        .send((request_id, profile, project_id))
                        .is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            request_receiver,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test_transcript_refresh()
    -> (Self, Receiver<(String, String, Option<String>, String)>) {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (_event_sender, event_receiver) = mpsc::channel();
        let (command_sender, _command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, _plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, http_receiver) = mpsc::channel();
        let (project_tree_sender, _project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, _cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, _artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, _artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, _artifact_image_receiver) = mpsc::channel();
        let (refresh_sender, refresh_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(command) = http_receiver.recv() {
                if let GatewayHttpCommand::RefreshActiveTranscript {
                    request_id,
                    session_id,
                    profile,
                    runtime_session_id,
                } = command
                    && refresh_sender
                        .send((request_id, session_id, profile, runtime_session_id))
                        .is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                status_receiver,
                event_receiver,
                command_sender,
                plugin_update_all_sender,
                http_sender,
                project_tree_sender,
                cron_trigger_sender,
                artifact_sender,
                artifact_download_sender,
                artifact_image_sender,
            },
            refresh_receiver,
        )
    }

    pub fn start() -> Self {
        Self::start_with_attach(attach_gateway, true)
    }

    pub(crate) fn start_remote(
        base_url: String,
        auth_mode: RemoteAuthMode,
        credential: RemoteGatewayAuth,
    ) -> Self {
        Self::start_with_attach(
            move || attach_remote_gateway(&base_url, auth_mode, credential),
            false,
        )
    }

    fn start_with_attach(
        attach: impl FnOnce() -> Result<(ConnectedGateway, GatewaySocket), String> + Send + 'static,
        load_sessions_http: bool,
    ) -> Self {
        let (status_sender, status_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, http_receiver) = mpsc::channel();
        let (project_tree_sender, project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, cron_trigger_receiver) = mpsc::sync_channel(8);
        let (artifact_sender, artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, artifact_image_receiver) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-gateway-session-list".into())
            .spawn(move || match attach() {
                Ok((connected, mut socket)) => {
                    let search_endpoint = connected.endpoint.clone();
                    let search_events = event_sender.clone();
                    let plugin_update_all_endpoint = connected.endpoint.clone();
                    let plugin_update_all_events = event_sender.clone();
                    let artifact_endpoint = connected.endpoint.clone();
                    let artifact_events = event_sender.clone();
                    let artifact_download_endpoint = connected.endpoint.clone();
                    let artifact_download_events = event_sender.clone();
                    let artifact_image_endpoint = connected.endpoint.clone();
                    let artifact_image_events = event_sender.clone();
                    let cron_trigger_endpoint = connected.endpoint.clone();
                    let cron_trigger_events = event_sender.clone();
                    let project_tree_endpoint = connected.endpoint.clone();
                    let project_tree_events = event_sender.clone();
                    let _ = status_sender.send(Ok(connected));
                    thread::Builder::new()
                        .name("hermes-plugin-update-all".into())
                        .spawn(move || {
                            agent_plugin_update_all_actor(
                                plugin_update_all_endpoint,
                                plugin_update_all_receiver,
                                plugin_update_all_events,
                            )
                        })
                        .expect("failed to start Agent Plugin update-all worker");
                    thread::Builder::new()
                        .name("hermes-profile-http".into())
                        .spawn(move || {
                            gateway_http_actor(search_endpoint, http_receiver, search_events)
                        })
                        .expect("failed to start Hermes session search worker");
                    thread::Builder::new()
                        .name("hermes-project-tree-aggregate".into())
                        .spawn(move || {
                            gateway_project_tree_actor(
                                project_tree_endpoint,
                                project_tree_receiver,
                                project_tree_events,
                            )
                        })
                        .expect("failed to start Hermes project tree aggregate worker");
                    thread::Builder::new()
                        .name("hermes-cron-trigger-dispatch".into())
                        .spawn(move || {
                            gateway_cron_trigger_actor(
                                cron_trigger_endpoint,
                                cron_trigger_receiver,
                                cron_trigger_events,
                            )
                        })
                        .expect("failed to start Hermes Scheduled Job trigger dispatcher");
                    thread::Builder::new()
                        .name("hermes-artifact-index".into())
                        .spawn(move || {
                            gateway_artifact_actor(
                                artifact_endpoint,
                                artifact_receiver,
                                artifact_events,
                            )
                        })
                        .expect("failed to start Hermes artifact-index worker");
                    thread::Builder::new()
                        .name("hermes-artifact-download".into())
                        .spawn(move || {
                            gateway_artifact_download_actor(
                                artifact_download_endpoint,
                                artifact_download_receiver,
                                artifact_download_events,
                            )
                        })
                        .expect("failed to start Hermes artifact-download worker");
                    thread::Builder::new()
                        .name("hermes-artifact-image-preview".into())
                        .spawn(move || {
                            gateway_artifact_image_actor(
                                artifact_image_endpoint,
                                artifact_image_receiver,
                                artifact_image_events,
                            )
                        })
                        .expect("failed to start Hermes artifact-image preview worker");
                    gateway_actor(&mut socket, command_receiver, event_sender);
                }
                Err(error) => {
                    let _ = status_sender.send(Err(error));
                }
            })
            .expect("failed to start Hermes gateway discovery worker");
        if load_sessions_http {
            let _ = http_sender.send(GatewayHttpCommand::LoadSessions);
        }
        Self {
            status_receiver,
            event_receiver,
            command_sender,
            plugin_update_all_sender,
            http_sender,
            project_tree_sender,
            cron_trigger_sender,
            artifact_sender,
            artifact_download_sender,
            artifact_image_sender,
        }
    }

    pub fn poll(&self) -> Option<Result<ConnectedGateway, String>> {
        match self.status_receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    pub fn poll_event(&self) -> Option<GatewayEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Retire this connection and stop accepting its results in the current UI.
    ///
    /// Dropping the event receiver first makes every late event send fail. Dropping
    /// the command senders then disconnects the Gateway actor and the worker
    /// channels; the socket actor observes command disconnection after its bounded
    /// read timeout. This does not join in-flight HTTP requests or an attach still
    /// blocked in network setup, so callers must still fence results by connection
    /// identity when more than one connection can be active.
    pub fn shutdown(self) {
        let Self {
            status_receiver,
            event_receiver,
            command_sender,
            plugin_update_all_sender,
            http_sender,
            project_tree_sender,
            cron_trigger_sender,
            artifact_sender,
            artifact_download_sender,
            artifact_image_sender,
        } = self;

        drop((status_receiver, event_receiver));
        drop((
            command_sender,
            plugin_update_all_sender,
            http_sender,
            project_tree_sender,
            cron_trigger_sender,
            artifact_sender,
            artifact_download_sender,
            artifact_image_sender,
        ));
    }

    pub fn resume_session(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::ResumeSession {
                request_id,
                session_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_older_transcript(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        offset: usize,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadOlderTranscript {
                request_id,
                session_id,
                profile,
                offset,
            })
            .map_err(|_| "Hermes transcript reader has closed".to_owned())
    }

    pub fn refresh_active_transcript(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        runtime_session_id: String,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::RefreshActiveTranscript {
                request_id,
                session_id,
                profile,
                runtime_session_id,
            })
            .map_err(|_| "Hermes transcript refresh reader has closed".to_owned())
    }

    /// Refresh the session list after a backend change signal. Each native
    /// window owns its own GatewayDiscovery, so every window must re-read the
    /// backend snapshot independently.
    pub fn refresh_sessions(&self, request_id: String, limit: usize) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::RefreshSessions { request_id, limit })
            .map_err(|_| "Hermes session list worker has closed".to_owned())
    }

    pub fn submit_prompt(
        &self,
        session_id: String,
        profile: Option<String>,
        submission_id: String,
        text: String,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::SubmitPrompt {
                session_id,
                profile,
                submission_id,
                text,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn resolve_unknown_prompt(
        &self,
        session_id: String,
        profile: Option<String>,
        admission_id: String,
        execution_generation: u64,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::ResolveUnknownPrompt {
                session_id,
                profile,
                admission_id,
                execution_generation,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn interrupt_session(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::InterruptSession {
                request_id,
                session_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn respond_approval(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        execution_generation: u64,
        prompt_id: String,
        choice: String,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::RespondApproval {
                request_id,
                session_id,
                profile,
                execution_generation,
                prompt_id,
                choice,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn respond_clarify(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        execution_generation: u64,
        prompt_id: String,
        answer: String,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::RespondClarify {
                request_id,
                session_id,
                profile,
                execution_generation,
                prompt_id,
                answer,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn lock_clarify(
        &self,
        request_id: String,
        clarify_request_id: String,
        question_id: String,
        profile: Option<String>,
        answer: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::LockClarify {
                request_id,
                clarify_request_id,
                question_id,
                profile,
                answer,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn respond_server_request(
        &self,
        id: String,
        response: Result<serde_json::Value, (i64, String)>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::RespondServerRequest { id, response })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn create_session(
        &self,
        request_id: String,
        params: serde_json::Value,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::CreateSession { request_id, params })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn search_sessions(&self, request_id: String, query: String) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SearchSessions { request_id, query })
            .map_err(|_| "Hermes session search worker has closed".to_owned())
    }

    pub fn load_archived_sessions(&self, request_id: String) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadArchivedSessions { request_id })
            .map_err(|_| "Hermes session list worker has closed".to_owned())
    }

    pub fn load_artifacts(&self, request_id: String) -> Result<(), String> {
        self.artifact_sender
            .send(request_id)
            .map_err(|_| "Hermes artifact-index worker has closed".to_owned())
    }

    pub fn download_artifact_file(
        &self,
        request: ArtifactFileDownloadRequest,
    ) -> Result<(), String> {
        self.artifact_download_sender
            .send(request)
            .map_err(|_| "Hermes artifact-download worker has closed".to_owned())
    }

    pub fn load_artifact_image_preview(
        &self,
        request: ArtifactImagePreviewRequest,
    ) -> Result<(), String> {
        self.artifact_image_sender
            .send(request)
            .map_err(|_| "Hermes artifact-image preview worker has closed".to_owned())
    }

    pub fn load_command_palette_sessions(&self, request_id: String) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCommandPaletteSessions { request_id })
            .map_err(|_| "Hermes session list worker has closed".to_owned())
    }

    pub fn load_capabilities(&self, request_id: String) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCapabilities { request_id })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_messaging_platforms(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadMessagingPlatforms {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_cron_jobs(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCronJobs {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_model_options(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadModelOptions {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_cron_delivery_targets(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCronDeliveryTargets {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_cron_blueprints(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCronBlueprints {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn instantiate_cron_blueprint(
        &self,
        request_id: String,
        profile: Option<String>,
        payload: CronBlueprintInstantiatePayload,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::InstantiateCronBlueprint {
                request_id,
                profile,
                payload,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn create_cron_job(
        &self,
        request_id: String,
        profile: Option<String>,
        payload: CronJobCreatePayload,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::CreateCronJob {
                request_id,
                profile,
                payload,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn update_cron_job(
        &self,
        request_id: String,
        profile: Option<String>,
        job_id: String,
        updates: CronJobUpdatePayload,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::UpdateCronJob {
                request_id,
                profile,
                job_id,
                updates,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn delete_cron_job(
        &self,
        request_id: String,
        profile: Option<String>,
        job_id: String,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::DeleteCronJob {
                request_id,
                profile,
                job_id,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_cron_job_runs(
        &self,
        request_id: String,
        profile: Option<String>,
        job_id: String,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCronJobRuns {
                request_id,
                profile,
                job_id,
            })
            .map_err(|_| "Hermes gateway HTTP worker is unavailable".into())
    }

    pub fn set_cron_job_paused(
        &self,
        request_id: String,
        profile: Option<String>,
        job_id: String,
        paused: bool,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SetCronJobPaused {
                request_id,
                profile,
                job_id,
                paused,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn trigger_cron_job(
        &self,
        request_id: String,
        profile: Option<String>,
        job_id: String,
    ) -> Result<(), String> {
        self.cron_trigger_sender
            .try_send(CronTriggerCommand {
                request_id,
                profile,
                job_id,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    "Hermes Scheduled Job trigger queue is full".to_owned()
                }
                mpsc::TrySendError::Disconnected(_) => {
                    "Hermes Scheduled Job trigger worker has closed".to_owned()
                }
            })
    }

    pub fn cancel_cron_job_run(
        &self,
        request_id: String,
        profile: Option<String>,
        job_id: String,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::CancelCronJobRun {
                request_id,
                profile,
                job_id,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn set_messaging_platform_enabled(
        &self,
        request_id: String,
        profile: Option<String>,
        platform_id: String,
        enabled: bool,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SetMessagingPlatformEnabled {
                request_id,
                profile,
                platform_id,
                enabled,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn update_messaging_platform_config(
        &self,
        request_id: String,
        profile: Option<String>,
        platform_id: String,
        update: MessagingPlatformUpdate,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::UpdateMessagingPlatformConfig {
                request_id,
                profile,
                platform_id,
                update,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_pairing(&self, request_id: String, profile: Option<String>) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadPairing {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn mutate_pairing(
        &self,
        request_id: String,
        profile: Option<String>,
        platform: String,
        user_id: String,
        kind: PairingMutationKind,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::MutatePairing {
                request_id,
                profile,
                platform,
                user_id,
                kind,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn start_telegram_onboarding(
        &self,
        request_id: String,
        profile: Option<String>,
        bot_name: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::StartTelegramOnboarding {
                request_id,
                profile,
                bot_name,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_telegram_onboarding_status(
        &self,
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadTelegramOnboardingStatus {
                request_id,
                pairing_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn apply_telegram_onboarding(
        &self,
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
        allowed_user_ids: Vec<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::ApplyTelegramOnboarding {
                request_id,
                pairing_id,
                profile,
                allowed_user_ids,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn cancel_telegram_onboarding(
        &self,
        request_id: String,
        pairing_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::CancelTelegramOnboarding {
                request_id,
                pairing_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn restart_gateway(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::RestartGateway {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_gateway_action_status(
        &self,
        request_id: String,
        action_name: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadGatewayActionStatus {
                request_id,
                action_name,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_command_center_system(
        &self,
        request_id: String,
        profile: Option<String>,
        log_file: String,
        log_level: String,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCommandCenterSystem {
                request_id,
                profile,
                log_file,
                log_level,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn command_center_system_action(
        &self,
        request_id: impl Into<String>,
        profile: impl Into<String>,
        operation: CommandCenterSystemOperation,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::CommandCenterSystemAction {
                request_id: request_id.into(),
                profile: Some(profile.into()),
                operation,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_command_center_usage(
        &self,
        request_id: String,
        profile: Option<String>,
        period_days: usize,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCommandCenterUsage {
                request_id,
                profile,
                period_days,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn command_center_maintenance(
        &self,
        request_id: impl Into<String>,
        profile: impl Into<String>,
        operation: CommandCenterMaintenanceOperation,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::CommandCenterMaintenance {
                request_id: request_id.into(),
                profile: Some(profile.into()),
                operation,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_display_timestamps(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadDisplayTimestamps {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn load_repo_scan_config(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadRepoScanConfig {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn save_repo_scan_config(
        &self,
        request_id: String,
        profile: Option<String>,
        policy: RepoScanPolicy,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SaveRepoScanConfig {
                request_id,
                profile,
                policy,
            })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
    }

    pub fn scan_and_record_repos(
        &self,
        request_id: String,
        profile: String,
        policy: RepoScanPolicy,
        home_dir: PathBuf,
        generation_state: Arc<AtomicU64>,
        generation: u64,
    ) -> Result<(), String> {
        if !valid_bounded_tree_id(&profile) || profile.eq_ignore_ascii_case("all") {
            return Err("Local repo discovery requires one explicit profile".into());
        }
        if !repo_scan_policy_is_bounded(&policy) {
            return Err("Hermes repo discovery settings exceed their bounded schema".into());
        }
        let command_sender = self.command_sender.clone();
        thread::Builder::new()
            .name("hermes-local-repo-scan".into())
            .spawn(move || {
                if generation_state.load(Ordering::Acquire) != generation {
                    return;
                }
                let repos = if policy.enabled {
                    scan_git_repos(&RepoScanOptions::new(policy.clone(), home_dir))
                } else {
                    Vec::new()
                };
                if generation_state.load(Ordering::Acquire) != generation {
                    return;
                }
                let _ = command_sender.send(GatewayCommand::RecordDiscoveredRepos {
                    request_id,
                    profile,
                    policy,
                    repos,
                });
            })
            .map_err(|_| "Could not start the Hermes repository discovery worker".to_owned())?;
        Ok(())
    }

    pub fn discover_remote_repos(&self, request_id: String, profile: String) -> Result<(), String> {
        if !valid_bounded_tree_id(&profile) || profile.eq_ignore_ascii_case("all") {
            return Err("Remote repo discovery requires one explicit profile".into());
        }
        self.command_sender
            .send(GatewayCommand::DiscoverRemoteRepos {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_connectors(&self, request_id: String) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::LoadConnectors { request_id })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_agent_plugins(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::LoadAgentPlugins {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_project_session_colors(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::LoadProjectSessionColors {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_project_tree(
        &self,
        request_id: String,
        profile: String,
        preview_limit: usize,
    ) -> Result<(), String> {
        if !(1..=2000).contains(&preview_limit) {
            return Err("Hermes project tree preview limit is outside the supported range".into());
        }
        self.command_sender
            .send(GatewayCommand::LoadProjectTree {
                request_id,
                profile,
                preview_limit,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_project_list(&self, request_id: String, profile: String) -> Result<(), String> {
        if !valid_bounded_tree_id(&profile) || profile.eq_ignore_ascii_case("all") {
            return Err("Hermes project list requires one explicit profile".into());
        }
        self.command_sender
            .send(GatewayCommand::LoadProjectList {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_bot_roster(&self, request_id: String, profile: String) -> Result<(), String> {
        if !valid_bounded_tree_id(&profile)
            || profile.len() > 128
            || profile.eq_ignore_ascii_case("all")
        {
            return Err("Hermes Bot roster requires one explicit profile".into());
        }
        self.command_sender
            .send(GatewayCommand::LoadBotRoster {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn delete_project(
        &self,
        request_id: String,
        project_id: String,
        profile: String,
    ) -> Result<(), String> {
        if !valid_bounded_tree_id(&profile)
            || profile.eq_ignore_ascii_case("all")
            || !valid_bounded_tree_id(&project_id)
        {
            return Err(
                "Hermes project deletion requires an explicit profile and project ID".into(),
            );
        }
        self.command_sender
            .send(GatewayCommand::DeleteProject {
                request_id,
                project_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_project_sessions(
        &self,
        request_id: String,
        project_id: String,
        profile: String,
    ) -> Result<(), String> {
        if !valid_bounded_tree_id(&project_id) || !valid_bounded_tree_id(&profile) {
            return Err("Hermes project session scope is invalid".into());
        }
        self.command_sender
            .send(GatewayCommand::LoadProjectSessions {
                request_id,
                project_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_all_profile_project_tree(
        &self,
        request_id: String,
        preview_limit: usize,
    ) -> Result<(), String> {
        if !(1..=2000).contains(&preview_limit) {
            return Err(
                "Hermes all-profile project tree preview limit is outside the supported range"
                    .into(),
            );
        }
        self.project_tree_sender
            .send(ProjectTreeHttpCommand {
                request_id,
                preview_limit,
            })
            .map_err(|_| "Hermes project tree HTTP worker has closed".to_owned())
    }

    pub fn load_active_session_statuses(
        &self,
        request_id: String,
        profile: String,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::LoadActiveSessionStatuses {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn load_agent_plugin_auto_update(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::LoadAgentPluginAutoUpdate {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn set_agent_plugin_auto_update(
        &self,
        request_id: String,
        profile: Option<String>,
        enabled: bool,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::SetAgentPluginAutoUpdate {
                request_id,
                profile,
                enabled,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn update_all_agent_plugins(
        &self,
        request_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.plugin_update_all_sender
            .send(AgentPluginUpdateAllCommand {
                request_id,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn set_agent_plugin_enabled(
        &self,
        request_id: String,
        key: String,
        enabled: bool,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::SetAgentPluginEnabled {
                request_id,
                key,
                enabled,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn remove_agent_plugin(
        &self,
        request_id: String,
        name: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::RemoveAgentPlugin {
                request_id,
                name,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn inspect_catalog_plugin(
        &self,
        request_id: String,
        catalog_name: String,
        identifier: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::InspectCatalogPlugin {
                request_id,
                catalog_name,
                identifier,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn install_catalog_plugin(
        &self,
        request_id: String,
        catalog_name: String,
        identifier: String,
        inspected_sha: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::InstallCatalogPlugin {
                request_id,
                catalog_name,
                identifier,
                inspected_sha,
                profile,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn update_agent_plugin(
        &self,
        request_id: String,
        name: String,
        profile: Option<String>,
        accept_capabilities: bool,
        force: bool,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::UpdateAgentPlugin {
                request_id,
                name,
                profile,
                accept_capabilities,
                force,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn set_toolset_enabled(
        &self,
        request_id: String,
        name: String,
        enabled: bool,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SetToolsetEnabled {
                request_id,
                name,
                enabled,
            })
            .map_err(|_| "Hermes capability writer has closed".to_owned())
    }

    pub fn set_session_pinned(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        pinned: bool,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SetSessionPinned {
                request_id,
                session_id,
                profile,
                pinned,
            })
            .map_err(|_| "Hermes session writer has closed".to_owned())
    }

    pub fn set_session_unread(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        unread: bool,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SetSessionUnread {
                request_id,
                session_id,
                profile,
                unread,
            })
            .map_err(|_| "Hermes session writer has closed".to_owned())
    }

    pub fn set_session_archived(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        archived: bool,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::SetSessionArchived {
                request_id,
                session_id,
                profile,
                archived,
            })
            .map_err(|_| "Hermes session writer has closed".to_owned())
    }

    pub fn rename_session(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
        title: String,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::RenameSession {
                request_id,
                session_id,
                profile,
                title,
            })
            .map_err(|_| "Hermes session writer has closed".to_owned())
    }

    pub fn delete_session(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::DeleteSession {
                request_id,
                session_id,
                profile,
            })
            .map_err(|_| "Hermes session writer has closed".to_owned())
    }

    pub fn export_session(
        &self,
        request_id: String,
        session_id: String,
        profile: Option<String>,
    ) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::ExportSession {
                request_id,
                session_id,
                profile,
            })
            .map_err(|_| "Hermes session writer has closed".to_owned())
    }

    pub fn rename_runtime_session(
        &self,
        request_id: String,
        stored_session_id: String,
        runtime_session_id: String,
        profile: Option<String>,
        title: String,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::RenameRuntimeSession {
                request_id,
                stored_session_id,
                runtime_session_id,
                profile,
                title,
                retry_only: false,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }

    pub fn retry_runtime_session_rename(
        &self,
        request_id: String,
        stored_session_id: String,
        runtime_session_id: String,
        profile: Option<String>,
        title: String,
    ) -> Result<(), String> {
        self.command_sender
            .send(GatewayCommand::RenameRuntimeSession {
                request_id,
                stored_session_id,
                runtime_session_id,
                profile,
                title,
                retry_only: true,
            })
            .map_err(|_| "Hermes gateway connection has closed".to_owned())
    }
}

fn attach_gateway() -> Result<(ConnectedGateway, GatewaySocket), String> {
    let output = Command::new("hermes")
        .args(["gateway", "ensure", "--json", "--timeout", "60"])
        .output()
        .map_err(|error| format!("Could not start Hermes gateway ensure: {error}"))?;

    if output.stdout.len() > 65_536 {
        return Err("Hermes gateway ensure returned an oversized response".into());
    }

    let stdout = Zeroizing::new(output.stdout);
    let reply: EnsureReply = serde_json::from_slice(&stdout).map_err(|error| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr.lines().last().unwrap_or_default();
        if last.is_empty() {
            format!("Hermes gateway ensure returned invalid JSON: {error}")
        } else {
            format!("Hermes gateway ensure failed: {last}")
        }
    })?;

    if !output.status.success() || reply.state != "ready" {
        let detail = reply
            .detail
            .or(reply.reason_code)
            .unwrap_or_else(|| format!("gateway state: {}", reply.state));
        return Err(detail);
    }

    let endpoint = reply
        .endpoint
        .ok_or_else(|| "Hermes reported ready without a gateway endpoint".to_owned())?;
    let mut socket = connect_gateway_endpoint(&endpoint)?;
    let rows = rpc(
        &mut socket,
        3,
        "session.list",
        serde_json::json!({ "limit": 200, "include_hidden": false }),
    )?;
    let sessions: Vec<SessionSummary> = serde_json::from_value(
        rows.get("sessions")
            .cloned()
            .ok_or_else(|| "Hermes session list response omitted sessions".to_owned())?,
    )
    .map_err(|_| "Hermes returned an invalid session list".to_owned())?;
    let mut sessions = sessions;
    for session in &mut sessions {
        if session.profile.is_none() {
            session.profile = Some(endpoint.profile_id.clone());
        }
    }
    Ok((ConnectedGateway { endpoint, sessions }, socket))
}

fn connect_gateway_endpoint(endpoint: &GatewayEndpoint) -> Result<GatewaySocket, String> {
    if endpoint.remote_auth.is_some() {
        return connect_remote_gateway_endpoint(endpoint);
    }
    validate_endpoint(endpoint)?;
    let ticket = mint_windows_ticket(&endpoint, "interactive", Some("host"))?;
    if ticket.is_empty() || ticket.len() > 2048 || !ticket.bytes().all(is_http_token_byte) {
        return Err("Hermes returned an invalid one-use gateway ticket".into());
    }
    let protocols = Zeroizing::new([
        CANONICAL_PROTOCOL.to_owned(),
        format!("{TICKET_PROTOCOL_PREFIX}{}", ticket.as_str()),
    ]);
    let url = websocket_url(&endpoint.api_origin)?;
    let expected_url = websocket_url(&endpoint.api_origin)?;
    if url != expected_url {
        return Err("Hermes returned an invalid authenticated gateway target".into());
    }

    let uri: Uri = url
        .parse()
        .map_err(|_| "Hermes returned an invalid WebSocket URI".to_owned())?;
    let request = ClientRequestBuilder::new(uri)
        .with_sub_protocol(protocols[0].clone())
        .with_sub_protocol(protocols[1].clone());
    // The gateway's canonical wire explicitly limits redirects to none. Its ticket must never be
    // replayed to a redirected origin.
    let (mut socket, response) = connect_with_config(request, None, 0)
        .map_err(|error| format!("Could not connect to the Hermes gateway: {error}"))?;
    let negotiated = response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|value| value.to_str().ok());
    if negotiated != Some(CANONICAL_PROTOCOL) {
        let _ = socket.close(None);
        return Err("Hermes gateway negotiated an unexpected WebSocket protocol".into());
    }

    set_socket_deadlines(&mut socket)?;
    wait_for_gateway_ready(&mut socket)?;
    rpc(
        &mut socket,
        1,
        "client.capabilities",
        serde_json::json!({
            "server_requests": false,
            "desktop_local": true
        }),
    )?;
    let runtime = rpc(&mut socket, 2, "runtime.describe", serde_json::json!({}))?;
    if runtime.get("instance_id").and_then(|value| value.as_str()) != Some(&endpoint.instance_id)
        || runtime.get("profile_id").and_then(|value| value.as_str()) != Some(&endpoint.profile_id)
        || runtime
            .get("authority_epoch")
            .and_then(|value| value.as_u64())
            != Some(endpoint.authority_epoch)
    {
        let _ = socket.close(None);
        return Err("Hermes gateway runtime identity did not match its attach receipt".into());
    }
    Ok(socket)
}

fn attach_remote_gateway(
    base_url: &str,
    auth_mode: RemoteAuthMode,
    credential: RemoteGatewayAuth,
) -> Result<(ConnectedGateway, GatewaySocket), String> {
    if !matches!(
        (&credential, auth_mode),
        (RemoteGatewayAuth::Token(_), RemoteAuthMode::Token)
            | (RemoteGatewayAuth::OAuth { .. }, RemoteAuthMode::OAuth)
    ) {
        return Err("Remote Hermes authentication mode did not match its saved credential".into());
    }
    let base_url = crate::remote_probe::normalize_remote_base_url(base_url)?;
    if matches!(credential, RemoteGatewayAuth::OAuth { .. }) {
        let _ = remote_access_token(&base_url, &credential)?;
    }
    let mut endpoint = GatewayEndpoint {
        profile_id: String::new(),
        instance_id: String::new(),
        authority_epoch: 0,
        runtime_protocol: 1,
        api_origin: base_url,
        capabilities: Vec::new(),
        control_home: None,
        remote_auth: Some(credential.clone()),
    };
    let mut socket = connect_remote_gateway_endpoint(&endpoint)?;
    // The web client advertises a browser/remote client. Never claim the local
    // desktop privilege bit for a remote server.
    rpc(
        &mut socket,
        1,
        "client.capabilities",
        serde_json::json!({"server_requests": false, "desktop_local": false}),
    )?;
    let runtime = rpc(&mut socket, 2, "runtime.describe", serde_json::json!({}))?;
    endpoint.profile_id = runtime
        .get("profile_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Remote Hermes runtime omitted its profile identity".to_owned())?
        .to_owned();
    endpoint.instance_id = runtime
        .get("instance_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Remote Hermes runtime omitted its instance identity".to_owned())?
        .to_owned();
    endpoint.authority_epoch = runtime
        .get("authority_epoch")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    endpoint.capabilities = runtime
        .get("capabilities")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    endpoint.remote_auth = Some(credential.clone());
    let rows = rpc(
        &mut socket,
        3,
        "session.list",
        serde_json::json!({ "limit": 200, "include_hidden": false }),
    )?;
    let mut sessions: Vec<SessionSummary> = serde_json::from_value(
        rows.get("sessions")
            .cloned()
            .ok_or_else(|| "Remote Hermes session list response omitted sessions".to_owned())?,
    )
    .map_err(|_| "Remote Hermes returned an invalid session list".to_owned())?;
    for session in &mut sessions {
        if session.profile.is_none() {
            session.profile = Some(endpoint.profile_id.clone());
        }
    }
    Ok((ConnectedGateway { endpoint, sessions }, socket))
}

fn connect_remote_gateway_endpoint(endpoint: &GatewayEndpoint) -> Result<GatewaySocket, String> {
    let auth = endpoint
        .remote_auth
        .as_ref()
        .ok_or_else(|| "Remote Hermes credentials are unavailable".to_owned())?;
    let mut ws_url = Url::parse(&endpoint.api_origin)
        .map_err(|_| "Remote Hermes gateway URL is invalid".to_owned())?;
    ws_url
        .set_scheme(if ws_url.scheme() == "https" {
            "wss"
        } else {
            "ws"
        })
        .map_err(|_| "Remote Hermes gateway URL has an unsupported scheme".to_owned())?;
    let prefix = ws_url.path().trim_end_matches('/');
    ws_url.set_path(&format!("{prefix}/api/ws"));
    let mut ticket = Zeroizing::new(String::new());
    match auth {
        RemoteGatewayAuth::Token(token) => {
            ws_url
                .query_pairs_mut()
                .append_pair("token", token.as_str());
        }
        RemoteGatewayAuth::OAuth { .. } => {
            let access_token = remote_access_token(&endpoint.api_origin, auth)?;
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(8)))
                .max_redirects(0)
                .build()
                .into();
            let response = agent
                .post(&format!(
                    "{}/api/auth/ws-ticket",
                    endpoint.api_origin.trim_end_matches('/')
                ))
                .header("Accept", "application/json")
                .header(
                    "Authorization",
                    &format!("Bearer {}", access_token.as_str()),
                )
                .send_empty()
                .map_err(|_| "Remote Hermes OAuth WebSocket ticket request failed".to_owned())?;
            if !response.status().is_success() {
                return Err(format!(
                    "Remote Hermes OAuth WebSocket ticket returned HTTP {}",
                    response.status()
                ));
            }
            let value: serde_json::Value =
                serde_json::from_reader(response.into_body().into_reader())
                    .map_err(|_| "Remote Hermes OAuth ticket response was invalid".to_owned())?;
            ticket.push_str(
                value
                    .get("ticket")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty() && value.len() <= 2048)
                    .ok_or_else(|| {
                        "Remote Hermes omitted a valid OAuth WebSocket ticket".to_owned()
                    })?,
            );
            ws_url
                .query_pairs_mut()
                .append_pair("ticket", ticket.as_str());
        }
    }
    let uri: Uri = ws_url
        .as_str()
        .parse()
        .map_err(|_| "Remote Hermes returned an invalid WebSocket target".to_owned())?;
    let request = ClientRequestBuilder::new(uri);
    let (mut socket, _) = connect_with_config(request, None, 0)
        .map_err(|_| "Could not connect to the authenticated remote Hermes gateway".to_owned())?;
    set_socket_deadlines(&mut socket)?;
    Ok(socket)
}

fn remote_access_token(
    api_origin: &str,
    auth: &RemoteGatewayAuth,
) -> Result<Zeroizing<String>, String> {
    let RemoteGatewayAuth::OAuth {
        session,
        credential_store_directory,
    } = auth
    else {
        return Err("Hermes OAuth credentials are unavailable".into());
    };
    let mut session = session
        .lock()
        .map_err(|_| "Hermes OAuth credential state is unavailable".to_owned())?;
    if native_token_needs_refresh(&session, unix_now_seconds()) {
        let refreshed = refresh_native_session(api_origin, &session)?;
        crate::native_token_store::save(credential_store_directory, api_origin, &refreshed)?;
        *session = refreshed;
    }
    Ok(Zeroizing::new(session.access_token.to_string()))
}

fn unix_now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

fn agent_plugin_update_all_actor(
    endpoint: GatewayEndpoint,
    commands: Receiver<AgentPluginUpdateAllCommand>,
    events: mpsc::Sender<GatewayEvent>,
) {
    agent_plugin_update_all_worker(commands, events, || connect_gateway_endpoint(&endpoint));
}

fn agent_plugin_update_all_worker<C>(
    commands: Receiver<AgentPluginUpdateAllCommand>,
    events: mpsc::Sender<GatewayEvent>,
    mut connect: C,
) where
    C: FnMut() -> Result<GatewaySocket, String>,
{
    let mut next_request_id = 3_u64;
    while let Ok(AgentPluginUpdateAllCommand {
        request_id,
        profile,
    }) = commands.recv()
    {
        let result = connect().and_then(|mut socket| {
            let result = update_all_agent_plugins_rpc(
                &mut socket,
                next_request_id,
                profile.as_deref(),
                Some(&events),
            );
            let _ = socket.close(None);
            result
        });
        next_request_id = next_request_id.saturating_add(1);
        if events
            .send(GatewayEvent::AgentPluginUpdateAll {
                request_id,
                profile,
                result,
            })
            .is_err()
        {
            return;
        }
    }
}

fn update_all_agent_plugins_rpc(
    socket: &mut GatewaySocket,
    request_id: u64,
    profile: Option<&str>,
    events: Option<&mpsc::Sender<GatewayEvent>>,
) -> Result<Vec<AgentPluginUpdateAllRow>, String> {
    let params = session_profile_params(serde_json::json!({"action":"update_all"}), profile)?;
    let value = rpc_with_events_timeout(
        socket,
        request_id,
        "plugins.manage",
        params,
        Duration::from_secs(600),
        events,
    )?;
    parse_agent_plugin_update_all(value)
}

fn collect_project_tree_session_colors(
    colors: &mut std::collections::BTreeMap<String, Option<String>>,
    rows: Option<&Vec<serde_json::Value>>,
    color: Option<String>,
) {
    for row in rows.into_iter().flatten() {
        if let Some(id) = row.get("id").and_then(serde_json::Value::as_str) {
            colors.insert(id.to_owned(), color.clone());
        }
    }
}

fn parse_project_session_colors(
    value: serde_json::Value,
) -> Result<Vec<ProjectSessionColor>, String> {
    let projects = value
        .get("projects")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Hermes returned an invalid projects.tree response".to_owned())?;
    let mut colors = std::collections::BTreeMap::<String, Option<String>>::new();
    for project in projects {
        let color = project
            .get("color")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if let Some(ids) = project
            .get("sessionIds")
            .and_then(serde_json::Value::as_array)
        {
            for id in ids.iter().filter_map(serde_json::Value::as_str) {
                colors.insert(id.to_owned(), color.clone());
            }
        }
        collect_project_tree_session_colors(
            &mut colors,
            project
                .get("previewSessions")
                .and_then(serde_json::Value::as_array),
            color.clone(),
        );
        if let Some(repos) = project.get("repos").and_then(serde_json::Value::as_array) {
            for repo in repos {
                if let Some(groups) = repo.get("groups").and_then(serde_json::Value::as_array) {
                    for group in groups {
                        collect_project_tree_session_colors(
                            &mut colors,
                            group.get("sessions").and_then(serde_json::Value::as_array),
                            color.clone(),
                        );
                    }
                }
            }
        }
    }
    Ok(colors
        .into_iter()
        .map(|(session_id, color)| ProjectSessionColor { session_id, color })
        .collect())
}

fn gateway_actor(
    socket: &mut GatewaySocket,
    commands: Receiver<GatewayCommand>,
    events: mpsc::Sender<GatewayEvent>,
) {
    let mut next_request_id = 5_u64;
    let mut retained_renames: HashMap<String, serde_json::Value> = HashMap::new();
    loop {
        match commands.try_recv() {
            Ok(GatewayCommand::RecordDiscoveredRepos {
                request_id,
                profile,
                policy,
                repos,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({
                        "discovery_policy": policy,
                        "repos": repos.iter().map(|repo| serde_json::json!({
                            "label": repo.label,
                            "root": repo.root,
                        })).collect::<Vec<_>>(),
                    }),
                    Some(&profile),
                )
                .and_then(|params| {
                    rpc_with_events(
                        socket,
                        next_request_id,
                        "projects.record_repos",
                        params,
                        Some(&events),
                    )
                })
                .and_then(|response| {
                    if response
                        .get("repos")
                        .and_then(serde_json::Value::as_array)
                        .is_none()
                    {
                        return Err("Hermes omitted the repository record result".into());
                    }
                    if policy.enabled
                        && response
                            .get("accepted")
                            .and_then(serde_json::Value::as_bool)
                            != Some(true)
                    {
                        return Err(
                            "Hermes rejected repository discovery for the captured policy".into(),
                        );
                    }
                    Ok(())
                });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::RepoDiscoveryCompleted {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::DiscoverRemoteRepos {
                request_id,
                profile,
            }) => {
                let result =
                    session_profile_params(serde_json::json!({"scan": true}), Some(&profile))
                        .and_then(|params| {
                            rpc_with_events(
                                socket,
                                next_request_id,
                                "projects.discover_repos",
                                params,
                                Some(&events),
                            )
                        })
                        .and_then(|response| {
                            if response
                                .get("repos")
                                .and_then(serde_json::Value::as_array)
                                .is_none()
                            {
                                Err("projects.discover_repos returned no repository list".into())
                            } else {
                                Ok(())
                            }
                        });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::RepoDiscoveryCompleted {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadProjectList {
                request_id,
                profile,
            }) => {
                let result = session_profile_params(serde_json::json!({}), Some(&profile))
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "projects.list",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(parse_project_list);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ProjectListLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadBotRoster {
                request_id,
                profile,
            }) => {
                let result = session_profile_params(serde_json::json!({}), Some(&profile))
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "profiles.list",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(parse_bot_roster);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::BotRosterLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::DeleteProject {
                request_id,
                project_id,
                profile,
            }) => {
                let result =
                    session_profile_params(serde_json::json!({"id": project_id}), Some(&profile))
                        .and_then(|params| {
                            rpc_with_events(
                                socket,
                                next_request_id,
                                "projects.delete",
                                params,
                                Some(&events),
                            )
                        })
                        .and_then(parse_project_list)
                        .and_then(|snapshot| {
                            if snapshot
                                .projects
                                .iter()
                                .any(|project| project.id == project_id)
                            {
                                Err(
                            "Hermes project delete receipt still contains the deleted project"
                                .into(),
                        )
                            } else {
                                Ok(snapshot)
                            }
                        });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ProjectDeleted {
                        request_id,
                        project_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadProjectTree {
                request_id,
                profile,
                preview_limit,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"preview_limit": preview_limit}),
                    Some(&profile),
                )
                .and_then(|params| {
                    rpc_with_events_timeout(
                        socket,
                        next_request_id,
                        "projects.tree",
                        params,
                        Duration::from_secs(60),
                        Some(&events),
                    )
                })
                .and_then(parse_project_tree);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ProjectTreeLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadProjectSessions {
                request_id,
                project_id,
                profile,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"project_id":project_id}),
                    Some(&profile),
                )
                .and_then(|params| {
                    rpc_with_events_timeout(
                        socket,
                        next_request_id,
                        "projects.project_sessions",
                        params,
                        Duration::from_secs(60),
                        Some(&events),
                    )
                })
                .and_then(|response| {
                    let Some(project) = response.get("project") else {
                        return Err("Hermes omitted the project session result".into());
                    };
                    if project.is_null() {
                        return Ok(None);
                    }
                    let tree = parse_project_tree(serde_json::json!({"projects":[project]}))?;
                    let project = tree.projects.into_iter().next().ok_or_else(|| {
                        "Hermes returned an empty project session result".to_owned()
                    })?;
                    if project.id != project_id {
                        return Err(
                            "Hermes returned project sessions for a different project".into()
                        );
                    }
                    Ok(Some(project))
                });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ProjectSessionsLoaded {
                        request_id,
                        project_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadActiveSessionStatuses {
                request_id,
                profile,
            }) => {
                let result = session_profile_params(serde_json::json!({}), Some(&profile))
                    .and_then(|params| {
                        rpc_with_events_timeout(
                            socket,
                            next_request_id,
                            "session.active_list",
                            params,
                            Duration::from_secs(10),
                            Some(&events),
                        )
                    })
                    .and_then(parse_active_session_statuses);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ActiveSessionStatusesLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadProjectSessionColors {
                request_id,
                profile,
            }) => {
                let result = session_profile_params(serde_json::json!({}), profile.as_deref())
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "projects.tree",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(parse_project_session_colors);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ProjectSessionColorsLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadAgentPlugins {
                request_id,
                profile,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({"action":"list"}),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "plugins.manage",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|value| {
                        let plugins = serde_json::from_value(
                            value
                                .get("plugins")
                                .cloned()
                                .unwrap_or_else(|| serde_json::json!([])),
                        )
                        .map_err(|_| {
                            "Hermes returned an invalid plugins.manage list response".to_owned()
                        })?;
                        Ok(AgentPluginsSnapshot {
                            plugins,
                            user_count: value
                                .get("user_count")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0) as usize,
                            bundled_count: value
                                .get("bundled_count")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0) as usize,
                        })
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginsLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadAgentPluginAutoUpdate {
                request_id,
                profile,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"action":"auto_update"}),
                    profile.as_deref(),
                )
                .and_then(|params| {
                    rpc_with_events(
                        socket,
                        next_request_id,
                        "plugins.manage",
                        params,
                        Some(&events),
                    )
                })
                .and_then(parse_agent_plugin_auto_update_settings);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginAutoUpdateLoaded {
                        request_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::SetAgentPluginAutoUpdate {
                request_id,
                profile,
                enabled,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"action":"auto_update","enable":enabled}),
                    profile.as_deref(),
                )
                .and_then(|params| {
                    rpc_with_events(
                        socket,
                        next_request_id,
                        "plugins.manage",
                        params,
                        Some(&events),
                    )
                })
                .and_then(parse_agent_plugin_auto_update_settings);
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginAutoUpdateSet {
                        request_id,
                        profile,
                        enabled,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::SetAgentPluginEnabled {
                request_id,
                key,
                enabled,
                profile,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({"action":"toggle","key":key,"enable":enabled}),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "plugins.manage",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|value| {
                        if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
                            return Err(value
                                .get("error")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("Hermes refused the plugin state change")
                                .to_owned());
                        }
                        let plugin: AgentPluginInfo =
                            serde_json::from_value(value.get("plugin").cloned().ok_or_else(
                                || "Hermes omitted the updated plugin receipt".to_owned(),
                            )?)
                            .map_err(|_| {
                                "Hermes returned an invalid updated plugin receipt".to_owned()
                            })?;
                        if plugin.key != key {
                            return Err("Hermes updated a different plugin key".into());
                        }
                        Ok(AgentPluginToggleReceipt {
                            plugin,
                            restart_required: value
                                .get("restart_required")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false),
                            gateway_reloaded: value
                                .get("gateway_reloaded")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false),
                        })
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginToggled {
                        request_id,
                        key,
                        enabled,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::RemoveAgentPlugin {
                request_id,
                name,
                profile,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"action":"remove","name":name}),
                    profile.as_deref(),
                )
                .and_then(|params| {
                    rpc_with_events(
                        socket,
                        next_request_id,
                        "plugins.manage",
                        params,
                        Some(&events),
                    )
                })
                .and_then(|value| {
                    if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
                        Ok(())
                    } else {
                        Err(value
                            .get("error")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Hermes refused to uninstall the plugin")
                            .to_owned())
                    }
                });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginRemoved {
                        request_id,
                        name,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::InspectCatalogPlugin {
                request_id,
                catalog_name,
                identifier,
                profile,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"action":"inspect","catalog_name":catalog_name,"identifier":identifier}),
                    profile.as_deref(),
                )
                .and_then(|params| {
                    rpc_with_events(socket, next_request_id, "plugins.manage", params, Some(&events))
                })
                .and_then(|value| {
                    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
                        return Err(value.get("error").and_then(serde_json::Value::as_str)
                            .unwrap_or("Hermes could not inspect this plugin source").to_owned());
                    }
                    let sha = value.get("sha").and_then(serde_json::Value::as_str)
                        .filter(|sha| sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
                        .map(str::to_ascii_lowercase);
                    Ok(AgentPluginInspection {
                        agent: value.get("agent").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        desktop: value.get("desktop").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        agent_name: value.get("agent_name").and_then(serde_json::Value::as_str).map(str::to_owned),
                        desktop_name: value.get("desktop_name").and_then(serde_json::Value::as_str).map(str::to_owned),
                        sha,
                        warnings: value.get("warnings").and_then(serde_json::Value::as_array)
                            .map(|rows| rows.iter().filter_map(serde_json::Value::as_str).map(str::to_owned).collect())
                            .unwrap_or_default(),
                        insecure: value.get("insecure").and_then(serde_json::Value::as_bool).unwrap_or(false),
                    })
                });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginInspected {
                        request_id,
                        catalog_name,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::InstallCatalogPlugin {
                request_id,
                catalog_name,
                identifier,
                inspected_sha,
                profile,
            }) => {
                let result = session_profile_params(
                    serde_json::json!({"action":"install","catalog_name":catalog_name,"identifier":identifier,
                        "enable":true,"force":false,"inspected_sha":inspected_sha}),
                    profile.as_deref(),
                )
                .and_then(|params| {
                    rpc_with_events(socket, next_request_id, "plugins.manage", params, Some(&events))
                })
                .and_then(|value| {
                    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
                        return Err(value.get("error").and_then(serde_json::Value::as_str)
                            .unwrap_or("Hermes refused to install this plugin").to_owned());
                    }
                    let plugin_name = value.get("plugin_name").and_then(serde_json::Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .ok_or_else(|| "Hermes omitted the installed plugin name".to_owned())?.to_owned();
                    let skills = value.pointer("/activation/live_now/skills")
                        .and_then(serde_json::Value::as_array)
                        .map(|rows| rows.iter().filter_map(|row| row.get("name").and_then(serde_json::Value::as_str))
                            .map(str::to_owned).collect()).unwrap_or_default();
                    Ok(AgentPluginInstallReceipt {
                        plugin_name,
                        enabled: value.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(true),
                        warnings: value.get("warnings").and_then(serde_json::Value::as_array)
                            .map(|rows| rows.iter().filter_map(serde_json::Value::as_str).map(str::to_owned).collect())
                            .unwrap_or_default(),
                        missing_env: value.get("missing_env").and_then(serde_json::Value::as_array)
                            .map(|rows| rows.iter().filter_map(serde_json::Value::as_str).map(str::to_owned).collect())
                            .unwrap_or_default(),
                        skills,
                    })
                });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginInstalled {
                        request_id,
                        catalog_name,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::UpdateAgentPlugin {
                request_id,
                name,
                profile,
                accept_capabilities,
                force,
            }) => {
                let mut update_params = serde_json::json!({"action":"update","name":name});
                if accept_capabilities {
                    update_params["accept_capabilities"] = serde_json::Value::Bool(true);
                }
                if force {
                    update_params["force"] = serde_json::Value::Bool(true);
                }
                let result = session_profile_params(update_params, profile.as_deref())
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "plugins.manage",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|value| {
                        if value
                            .get("consent_required")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                        {
                            return Ok(AgentPluginUpdateOutcome::Consent {
                                sha: value
                                    .get("sha")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("")
                                    .chars()
                                    .take(8)
                                    .collect(),
                                delta_lines: value
                                    .get("delta_lines")
                                    .and_then(serde_json::Value::as_array)
                                    .map(|rows| {
                                        rows.iter()
                                            .filter_map(serde_json::Value::as_str)
                                            .map(str::to_owned)
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                            });
                        }
                        if value
                            .get("scan_blocked")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                        {
                            let findings = serde_json::from_value(
                                value
                                    .get("scan_findings")
                                    .cloned()
                                    .unwrap_or_else(|| serde_json::json!([])),
                            )
                            .map_err(|_| {
                                "Hermes returned invalid plugin security scan findings".to_owned()
                            })?;
                            return Ok(AgentPluginUpdateOutcome::Scan {
                                verdict: value
                                    .get("scan_verdict")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("dangerous")
                                    .to_owned(),
                                findings,
                            });
                        }
                        if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
                            return Ok(AgentPluginUpdateOutcome::Applied {
                                unchanged: value
                                    .get("unchanged")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false),
                            });
                        }
                        Err(value
                            .get("error")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Hermes refused to update the plugin")
                            .to_owned())
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::AgentPluginUpdated {
                        request_id,
                        name,
                        profile,
                        accept_capabilities,
                        force,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LoadConnectors { request_id }) => {
                let result = (|| {
                    let listed = rpc_with_events(
                        socket,
                        next_request_id,
                        "connectors.list",
                        serde_json::json!({"owner":{"type":"account"}}),
                        Some(&events),
                    );
                    next_request_id = next_request_id.saturating_add(1);
                    let listed = listed?;
                    let available = listed
                        .get("available")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let connectors = serde_json::from_value(
                        listed
                            .get("connectors")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!([])),
                    )
                    .map_err(|_| {
                        "Hermes returned an invalid connectors.list response".to_owned()
                    })?;

                    let catalog_response = rpc_with_events(
                        socket,
                        next_request_id,
                        "connectors.catalog",
                        serde_json::json!({}),
                        Some(&events),
                    );
                    next_request_id = next_request_id.saturating_add(1);
                    let catalog_response = catalog_response?;
                    let catalog = serde_json::from_value(
                        catalog_response
                            .get("connectors")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!([])),
                    )
                    .map_err(|_| {
                        "Hermes returned an invalid connectors.catalog response".to_owned()
                    })?;

                    let accounts_response = rpc_with_events(
                        socket,
                        next_request_id,
                        "connectors.accounts",
                        serde_json::json!({}),
                        Some(&events),
                    );
                    next_request_id = next_request_id.saturating_add(1);
                    let accounts_response = accounts_response?;
                    let accounts = serde_json::from_value(
                        accounts_response
                            .get("accounts")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!([])),
                    )
                    .map_err(|_| {
                        "Hermes returned an invalid connectors.accounts response".to_owned()
                    })?;
                    Ok(ConnectorsSnapshot {
                        available,
                        connectors,
                        catalog,
                        accounts,
                    })
                })();
                if events
                    .send(GatewayEvent::ConnectorsLoaded { request_id, result })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::ResumeSession {
                request_id,
                session_id,
                profile,
            }) => {
                let event_profile = profile.clone();
                let result = resume_session(socket, &session_id, profile, &events);
                if events
                    .send(GatewayEvent::SessionResumed {
                        request_id,
                        session_id,
                        profile: event_profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::CreateSession { request_id, params }) => {
                let result = rpc_with_events(
                    socket,
                    next_request_id,
                    "session.create",
                    params,
                    Some(&events),
                )
                .and_then(|result| {
                    let runtime_session_id = result
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| {
                            "Hermes session.create omitted its runtime session id".to_owned()
                        })?;
                    let stored_session_id = result
                        .get("stored_session_id")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| {
                            "Hermes session.create omitted its stored session id".to_owned()
                        })?;
                    Ok(CreatedSession {
                        request_id: request_id.clone(),
                        runtime_session_id: runtime_session_id.to_owned(),
                        stored_session_id: stored_session_id.to_owned(),
                    })
                });
                next_request_id = next_request_id.saturating_add(1);
                if events.send(GatewayEvent::SessionCreated(result)).is_err() {
                    return;
                }
            }
            Ok(GatewayCommand::RenameRuntimeSession {
                request_id,
                stored_session_id,
                runtime_session_id,
                profile,
                title,
                retry_only,
            }) => {
                let result = rename_runtime_session_rpc(
                    socket,
                    &events,
                    &mut next_request_id,
                    &mut retained_renames,
                    &request_id,
                    &runtime_session_id,
                    profile.as_deref(),
                    &title,
                    retry_only,
                );
                if events
                    .send(GatewayEvent::SessionRenamed {
                        request_id,
                        session_id: stored_session_id,
                        profile,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::SubmitPrompt {
                session_id,
                profile,
                submission_id,
                text,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({"session_id":session_id,"submission_id":submission_id,"text":text}),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "prompt.submit",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|result| parse_prompt_submit_receipt(&result, &session_id));
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::PromptSubmitted {
                        session_id,
                        submission_id,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::ResolveUnknownPrompt {
                session_id,
                profile,
                admission_id,
                execution_generation,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({
                        "session_id":session_id,
                        "admission_id":admission_id,
                        "execution_generation":execution_generation
                    }),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "prompt.resolve_unknown",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|reply| {
                        if reply
                            .get("admission_id")
                            .and_then(serde_json::Value::as_str)
                            != Some(admission_id.as_str())
                            || reply
                                .pointer("/ref/session_id")
                                .and_then(serde_json::Value::as_str)
                                != Some(session_id.as_str())
                        {
                            return Err(
                                "Hermes unknown-admission receipt destination mismatch".into()
                            );
                        }
                        reply
                            .get("status")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                            .ok_or_else(|| "Hermes unknown-admission receipt omitted status".into())
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::UnknownPromptResolved {
                        session_id,
                        admission_id,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::InterruptSession {
                request_id,
                session_id,
                profile,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({"session_id":session_id}),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "session.interrupt",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|result| {
                        match result.get("status").and_then(serde_json::Value::as_str) {
                            Some("interrupted") => Ok(true),
                            Some("not_interrupted") => Ok(false),
                            _ => Err("Hermes returned an invalid session.interrupt result".into()),
                        }
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::SessionInterrupted {
                        request_id,
                        session_id,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::RespondApproval {
                request_id,
                session_id,
                profile,
                execution_generation,
                prompt_id,
                choice,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({
                        "session_id":session_id,
                        "execution_generation":execution_generation,
                        "prompt_id":prompt_id,
                        "choice":choice
                    }),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "approval.respond",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|result| {
                        let status = result.get("status").and_then(serde_json::Value::as_str);
                        let returned_prompt =
                            result.get("prompt_id").and_then(serde_json::Value::as_str);
                        if matches!(status, Some("resolved" | "already_resolved"))
                            && returned_prompt == Some(&prompt_id)
                        {
                            Ok(())
                        } else {
                            Err("Hermes returned an invalid approval response receipt".into())
                        }
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ApprovalResponded {
                        request_id,
                        session_id,
                        prompt_id,
                        choice,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::RespondClarify {
                request_id,
                session_id,
                profile,
                execution_generation,
                prompt_id,
                answer,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({
                        "session_id":session_id,
                        "execution_generation":execution_generation,
                        "prompt_id":prompt_id,
                        "answer":answer
                    }),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "clarify.respond",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|result| {
                        let status = result.get("status").and_then(serde_json::Value::as_str);
                        let returned_prompt =
                            result.get("prompt_id").and_then(serde_json::Value::as_str);
                        if matches!(status, Some("resolved" | "already_resolved"))
                            && returned_prompt == Some(prompt_id.as_str())
                        {
                            Ok(())
                        } else {
                            Err("Hermes returned an invalid clarify response receipt".into())
                        }
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ClarifyResponded {
                        request_id,
                        session_id,
                        prompt_id,
                        answer,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::LockClarify {
                request_id,
                clarify_request_id,
                question_id,
                profile,
                answer,
            }) => {
                let params = session_profile_params(
                    serde_json::json!({
                        "request_id":clarify_request_id,
                        "question_id":question_id,
                        "answer":answer
                    }),
                    profile.as_deref(),
                );
                let result = params
                    .and_then(|params| {
                        rpc_with_events(
                            socket,
                            next_request_id,
                            "clarify.lock",
                            params,
                            Some(&events),
                        )
                    })
                    .and_then(|result| {
                        match result.get("status").and_then(serde_json::Value::as_str) {
                            Some("ok") => Ok(ClarifyLockReceipt { expired: false }),
                            Some("expired") => Ok(ClarifyLockReceipt { expired: true }),
                            _ => Err("Hermes returned an invalid clarify.lock receipt".into()),
                        }
                    });
                next_request_id = next_request_id.saturating_add(1);
                if events
                    .send(GatewayEvent::ClarifyLocked {
                        request_id,
                        clarify_request_id,
                        question_id,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Ok(GatewayCommand::RespondServerRequest { id, response }) => {
                let (frame, result) = match response {
                    Ok(result) => (
                        serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}),
                        Ok(()),
                    ),
                    Err((code, message)) => (
                        serde_json::json!({
                            "jsonrpc":"2.0",
                            "id":id,
                            "error":{"code":code,"message":message}
                        }),
                        Ok(()),
                    ),
                };
                let result = socket
                    .send(Message::Text(frame.to_string().into()))
                    .map_err(|_| "Could not answer Hermes server request".to_owned())
                    .and(result);
                if events
                    .send(GatewayEvent::ServerRequestAnswered { id, result })
                    .is_err()
                {
                    return;
                }
            }
            Err(TryRecvError::Disconnected) => {
                let _ = socket.close(None);
                return;
            }
            Err(TryRecvError::Empty) => {}
        }

        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Ok(frame) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some((id, method, params)) = server_request_parts(&frame) {
                        if matches!(method.as_str(), "approval" | "clarify") {
                            let _ = events.send(GatewayEvent::ServerRequestReceived {
                                id,
                                method,
                                params,
                            });
                        } else if let Some(response) = unhandled_server_request_response(&frame) {
                            if socket
                                .send(Message::Text(response.to_string().into()))
                                .is_err()
                            {
                                let _ = events.send(GatewayEvent::Disconnected(
                                    "Hermes gateway closed while declining a server request".into(),
                                ));
                                return;
                            }
                        }
                    } else if frame.get("method").and_then(|value| value.as_str()) == Some("event")
                        && let Some(params) = frame.get("params")
                    {
                        let _ = events.send(GatewayEvent::ServerEvent(params.clone()));
                    }
                }
            }
            Ok(Message::Close(_)) => return,
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => {
                let _ = events.send(GatewayEvent::Disconnected(format!(
                    "Hermes gateway connection closed: {error}"
                )));
                return;
            }
        }
    }
}

fn parse_prompt_submit_receipt(
    reply: &serde_json::Value,
    session_id: &str,
) -> Result<PromptSubmitReceipt, String> {
    if reply
        .get("voice_stopped")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && reply.get("admission_id").is_none()
        && reply.get("ref").is_none()
    {
        return Ok(PromptSubmitReceipt::VoiceStopped);
    }
    if reply
        .pointer("/ref/session_id")
        .and_then(serde_json::Value::as_str)
        != Some(session_id)
    {
        return Err("Hermes prompt.submit receipt destination mismatch".into());
    }
    let admission_id = reply
        .get("admission_id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "Hermes prompt.submit omitted its admission ID".to_owned())?;
    let status = reply
        .get("status")
        .and_then(serde_json::Value::as_str)
        .filter(|status| matches!(*status, "queued" | "started" | "unknown" | "terminal"))
        .ok_or_else(|| "Hermes prompt.submit returned an invalid admission status".to_owned())?;
    let execution_generation = reply
        .get("execution_generation")
        .and_then(serde_json::Value::as_u64);
    if status == "unknown" && execution_generation.is_none() {
        return Err("Hermes unknown admission omitted its execution generation".into());
    }
    Ok(PromptSubmitReceipt::Admitted {
        admission_id: admission_id.to_owned(),
        status: status.to_owned(),
        execution_generation,
    })
}

fn rename_runtime_session_rpc(
    socket: &mut GatewaySocket,
    events: &mpsc::Sender<GatewayEvent>,
    next_request_id: &mut u64,
    retained: &mut HashMap<String, serde_json::Value>,
    request_id: &str,
    runtime_session_id: &str,
    profile: Option<&str>,
    title: &str,
    retry_only: bool,
) -> Result<String, String> {
    if retry_only && !retained.contains_key(request_id) {
        return Err("Original Hermes rename request is unavailable after reconnect".into());
    }
    let params = if let Some(params) = retained.get(request_id) {
        if params.get("session_id").and_then(serde_json::Value::as_str) != Some(runtime_session_id)
            || params.get("profile").and_then(serde_json::Value::as_str) != profile
            || params
                .pointer("/payload/title")
                .and_then(serde_json::Value::as_str)
                != Some(title)
        {
            return Err("Hermes retained rename request identity changed".into());
        }
        params.clone()
    } else {
        let resume_params = session_profile_params(
            serde_json::json!({"session_id": runtime_session_id, "omit_messages": true}),
            profile,
        )?;
        let snapshot = rpc_with_events(
            socket,
            *next_request_id,
            "session.resume",
            resume_params,
            Some(events),
        );
        *next_request_id = next_request_id.saturating_add(1);
        let snapshot = snapshot?;
        if snapshot
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            != Some(runtime_session_id)
        {
            return Err("Hermes rename snapshot destination mismatch".into());
        }
        let revision = snapshot
            .get("revision")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "Hermes rename snapshot omitted its revision".to_owned())?;
        let params = session_profile_params(
            serde_json::json!({
                "session_id": runtime_session_id,
                "request_id": request_id,
                "expected_revision": revision,
                "operation": "rename",
                "payload": {"title": title},
            }),
            profile,
        )?;
        retained.insert(request_id.to_owned(), params.clone());
        params
    };
    let reply = rpc_with_events(
        socket,
        *next_request_id,
        "session.mutate",
        params,
        Some(events),
    );
    *next_request_id = next_request_id.saturating_add(1);
    match reply {
        Ok(reply) => {
            if reply.get("session_id").and_then(serde_json::Value::as_str)
                != Some(runtime_session_id)
            {
                return Err("Hermes rename receipt destination mismatch".into());
            }
            let title = reply
                .get("title")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "Hermes rename receipt omitted its confirmed title".to_owned())?;
            retained.remove(request_id);
            Ok(title.to_owned())
        }
        Err(error) => {
            if error.starts_with("Hermes session.mutate refused:") {
                retained.remove(request_id);
            }
            Err(error)
        }
    }
}

fn resume_session(
    socket: &mut GatewaySocket,
    session_id: &str,
    profile: Option<String>,
    events: &mpsc::Sender<GatewayEvent>,
) -> Result<SessionTranscript, String> {
    let params = session_profile_params(
        serde_json::json!({ "session_id": session_id, "omit_messages": false }),
        profile.as_deref(),
    )?;
    let snapshot = rpc_with_events(socket, 4, "session.resume", params, Some(events))?;
    let runtime_session_id = snapshot
        .get("session_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let stored_session_id = snapshot
        .get("stored_session_id")
        .and_then(|value| value.as_str());
    if runtime_session_id != session_id && stored_session_id != Some(session_id) {
        return Err("Hermes returned a different session than requested".into());
    }
    let message_count = snapshot
        .get("message_count")
        .and_then(|value| value.as_u64())
        .unwrap_or_default() as usize;
    let messages = snapshot
        .get("messages")
        .and_then(|value| value.as_array())
        .cloned()
        .ok_or_else(|| "Hermes session snapshot omitted its transcript".to_owned())?;
    Ok(SessionTranscript {
        session_id: session_id.to_owned(),
        profile,
        runtime_session_id: runtime_session_id.to_owned(),
        message_count,
        messages,
    })
}

fn session_profile_params(
    mut params: serde_json::Value,
    profile: Option<&str>,
) -> Result<serde_json::Value, String> {
    let Some(profile) = profile else {
        return Ok(params);
    };
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes session profile scope is invalid".into());
    }
    let object = params
        .as_object_mut()
        .ok_or_else(|| "Hermes session request parameters must be an object".to_owned())?;
    object.insert(
        "profile".into(),
        serde_json::Value::String(profile.to_owned()),
    );
    Ok(params)
}

fn parse_agent_plugin_auto_update_settings(
    value: serde_json::Value,
) -> Result<AgentPluginAutoUpdateSettings, String> {
    if value.get("ok").and_then(serde_json::Value::as_bool) == Some(false) {
        return Err(value
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Hermes refused to read automatic plugin updates")
            .to_owned());
    }
    let check_hours = match value.get("check_hours") {
        None | Some(serde_json::Value::Null) => 24,
        Some(hours) => hours.as_u64().ok_or_else(|| {
            "Hermes returned an invalid automatic plugin update interval".to_owned()
        })?,
    };
    Ok(AgentPluginAutoUpdateSettings {
        auto_apply: value
            .get("auto_apply")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        check_hours,
    })
}

fn parse_agent_plugin_update_all(
    value: serde_json::Value,
) -> Result<Vec<AgentPluginUpdateAllRow>, String> {
    let rows: Vec<AgentPluginUpdateAllRow> = serde_json::from_value(
        value
            .get("results")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )
    .map_err(|_| "Hermes returned invalid plugin update-all results".to_owned())?;
    if rows.iter().any(|row| {
        row.name.is_empty()
            || row.name.len() > 512
            || row.name.chars().any(char::is_control)
            || !matches!(
                row.status.as_str(),
                "updated" | "current" | "failed" | "needs_review"
            )
            || row
                .detail
                .as_ref()
                .is_some_and(|detail| detail.len() > 16_384)
    }) {
        return Err("Hermes returned invalid plugin update-all row data".into());
    }
    Ok(rows)
}

const TICKET_CLIENT: &str = r#"
import json, sys
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
with redirect_stdout(sys.stderr):
    from hermes_cli.gateway_client import _session_ticket
    from hermes_cli.gateway_runtime_discovery import DiscoveryError, _private_node
line = sys.stdin.buffer.readline(65537)
if len(line) > 65536 or not line.endswith(b'\n'):
    raise SystemExit(2)
request = json.loads(line)
try:
    with redirect_stdout(sys.stderr):
        endpoint = SimpleNamespace(**{'control_home': None, **request['endpoint']})
        home = Path(endpoint.profile_id)
        if str(home.resolve()) != endpoint.profile_id or endpoint.runtime_protocol != 1:
            raise ValueError('invalid ticket endpoint')
        if __import__('os').name != 'nt':
            _private_node(home, kind='directory', home=True)
        ticket = _session_ticket(home, endpoint, purpose=request['purpose'], scope=request.get('scope'))
    print(json.dumps({'id': request['id'], 'ticket': ticket}), flush=True)
except DiscoveryError as exc:
    print(json.dumps({'id': request['id'], 'error': exc.reason}), flush=True)
except Exception:
    print(json.dumps({'id': request['id'], 'error': 'ticket_failed'}), flush=True)
"#;

fn mint_windows_ticket(
    endpoint: &GatewayEndpoint,
    purpose: &str,
    scope: Option<&str>,
) -> Result<Zeroizing<String>, String> {
    let runtime_output = Command::new("hermes")
        .arg("--print-runtime-command")
        .output()
        .map_err(|error| format!("Could not resolve Hermes' installed Python runtime: {error}"))?;
    if !runtime_output.status.success() || runtime_output.stdout.len() > 65_536 {
        return Err("Hermes did not report its installed Python runtime".into());
    }
    let runtime: Vec<String> = serde_json::from_slice(&runtime_output.stdout)
        .map_err(|_| "Hermes returned an invalid installed runtime command".to_owned())?;
    let Some(python) = runtime
        .first()
        .filter(|path| std::path::Path::new(path).is_absolute())
    else {
        return Err("Hermes returned an invalid Python runtime path".into());
    };
    let code = runtime
        .iter()
        .position(|argument| argument == "-c")
        .and_then(|index| runtime.get(index + 1))
        .ok_or_else(|| "Hermes runtime launcher omitted its bootstrap command".to_owned())?;
    let marker = "runpy.run_module('hermes_cli.main'";
    let (bootstrap, _) = code
        .split_once(marker)
        .ok_or_else(|| "Hermes runtime launcher bootstrap is incompatible".to_owned())?;
    let launcher =
        format!("{bootstrap}exec(compile({TICKET_CLIENT:?}, '<hermes-native-ticket>', 'exec'))");
    let endpoint_json = serde_json::json!({
        "profile_id": endpoint.profile_id,
        "instance_id": endpoint.instance_id,
        "runtime_protocol": endpoint.runtime_protocol,
        "control_home": endpoint.control_home,
    });
    let request = serde_json::json!({
        "id": 1,
        "endpoint": endpoint_json,
        "purpose": purpose,
        "scope": scope
    });
    let mut child = Command::new(python)
        .args(["-I", "-c", &launcher])
        .env("HERMES_HOME", &endpoint.profile_id)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not start Hermes' private ticket client".to_owned())?;
    let mut input = Zeroizing::new(format!("{}\n", request));
    child
        .stdin
        .take()
        .ok_or_else(|| "Hermes ticket client has no private input".to_owned())?
        .write_all(input.as_bytes())
        .map_err(|_| "Could not write to Hermes' private ticket client".to_owned())?;
    input.zeroize();
    let output = child
        .wait_with_output()
        .map_err(|_| "Hermes' private ticket client did not return".to_owned())?;
    if !output.status.success() || output.stdout.len() > 65_536 {
        return Err("Hermes' private ticket client failed".into());
    }
    #[derive(Deserialize)]
    struct TicketReply {
        ticket: Option<String>,
        error: Option<String>,
    }
    let stdout = Zeroizing::new(output.stdout);
    let mut reply: TicketReply = serde_json::from_slice(&stdout)
        .map_err(|_| "Hermes' private ticket client returned invalid data".to_owned())?;
    if let Some(ticket) = reply.ticket.take().filter(|ticket| !ticket.is_empty()) {
        return Ok(Zeroizing::new(ticket));
    }
    Err(format!(
        "Hermes refused to mint a gateway ticket ({})",
        reply.error.unwrap_or_else(|| "ticket_failed".into())
    ))
}

#[derive(Deserialize)]
struct SessionSearchResponse {
    results: Vec<SessionSearchResult>,
}

#[derive(Deserialize)]
struct SessionMessagesPageResponse {
    session_id: String,
    #[serde(default)]
    profile: Option<String>,
    messages: Vec<serde_json::Value>,
    #[serde(default)]
    pagination: Option<SessionMessagesPagination>,
}

#[derive(Deserialize)]
struct SessionMessagesPagination {
    limit: usize,
    offset: usize,
    #[serde(default)]
    order: Option<String>,
    returned: usize,
}

fn gateway_artifact_download_actor(
    endpoint: GatewayEndpoint,
    requests: Receiver<ArtifactFileDownloadRequest>,
    events: mpsc::Sender<GatewayEvent>,
) {
    while let Ok(request) = requests.recv() {
        let request_id = request.request_id.clone();
        #[cfg(windows)]
        let result = download_remote_artifact_file(&endpoint, &request, |filename| {
            let mut dialog = rfd::FileDialog::new()
                .set_title("Save File")
                .set_file_name(filename);
            if let Some(extension) = safe_filename_extension(filename) {
                let label = format!("{} File", extension.to_ascii_uppercase());
                dialog = dialog.add_filter(label, &[extension.as_str()]);
            }
            dialog.add_filter("All Files", &["*"]).save_file()
        });
        #[cfg(not(windows))]
        let result: Result<Option<PathBuf>, String> = Err(
            "Remote artifact downloads currently require the Windows native save dialog".into(),
        );
        let result = result.map(|path| path.map(|path| path.to_string_lossy().into_owned()));
        if events
            .send(GatewayEvent::ArtifactFileDownloaded { request_id, result })
            .is_err()
        {
            return;
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactImageDataUrl {
    data_url: String,
}

fn gateway_artifact_image_actor(
    endpoint: GatewayEndpoint,
    requests: Receiver<ArtifactImagePreviewRequest>,
    events: mpsc::Sender<GatewayEvent>,
) {
    while let Ok(request) = requests.recv() {
        let request_id = request.request_id.clone();
        let artifact_id = request.artifact_id.clone();
        let result = load_remote_artifact_image_preview(&endpoint, &request);
        if events
            .send(GatewayEvent::ArtifactImagePreviewLoaded {
                request_id,
                artifact_id,
                result,
            })
            .is_err()
        {
            return;
        }
    }
}

fn artifact_image_preview_url(
    endpoint: &GatewayEndpoint,
    request: &ArtifactImagePreviewRequest,
) -> Result<String, String> {
    if endpoint.remote_auth.is_none()
        || request.request_id.is_empty()
        || request.request_id.len() > 256
        || request.artifact_id.is_empty()
        || request.artifact_id.len() > 512
        || request.artifact_id.chars().any(char::is_control)
        || request.path.trim().is_empty()
        || request.path.len() > 16_384
        || request.path.chars().any(char::is_control)
        || request.session_id.is_empty()
        || request.session_id.len() > 256
        || request.session_id.chars().any(char::is_control)
        || request.profile.is_empty()
        || request.profile.len() > 128
        || request.profile.chars().any(char::is_control)
    {
        return Err("Remote artifact image scope is invalid".into());
    }
    let api_origin = endpoint.api_origin.trim_end_matches('/');
    let external_http = request
        .path
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
        || request
            .path
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
    let url = if external_http {
        let remote_image = Url::parse(&request.path)
            .map_err(|_| "Remote artifact image URL is invalid".to_owned())?;
        if !matches!(remote_image.scheme(), "http" | "https")
            || remote_image.host_str().is_none()
            || !remote_image.username().is_empty()
            || remote_image.password().is_some()
        {
            return Err("Remote artifact image URL is not an allowed HTTP target".into());
        }
        format!(
            "{api_origin}/api/media/proxy?url={}&profile={}",
            encode_query_component(&request.path),
            encode_query_component(&request.profile)
        )
    } else {
        let path = artifact_media_path(&request.path);
        format!(
            "{api_origin}/api/fs/read-data-url?path={}&session_id={}&profile={}",
            encode_query_component(&path),
            encode_query_component(&request.session_id),
            encode_query_component(&request.profile)
        )
    };
    if !api_url_is_within_origin(&endpoint.api_origin, &url) {
        return Err("Remote artifact image target is outside the configured gateway".into());
    }
    Ok(url)
}

fn artifact_media_path(path: &str) -> String {
    if !path
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
    {
        return path.to_owned();
    }
    let Some(url) = Url::parse(path).ok() else {
        return path.to_owned();
    };
    if let Ok(path) = url.to_file_path() {
        return path.to_string_lossy().into_owned();
    }
    let decoded = percent_decode_path(url.path());
    match url.host_str() {
        Some(host) if !host.is_empty() => format!("//{host}{decoded}"),
        _ => decoded,
    }
}

fn percent_decode_path(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&value[index + 1..index + 3], 16)
        {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_owned())
}

fn load_remote_artifact_image_preview(
    endpoint: &GatewayEndpoint,
    request: &ArtifactImagePreviewRequest,
) -> Result<FilePreviewContent, String> {
    let url = artifact_image_preview_url(endpoint, request)?;
    let response: ArtifactImageDataUrl =
        request_remote_gateway_json_with_limit(endpoint, &url, ARTIFACT_IMAGE_RESPONSE_LIMIT)?;
    let (metadata, encoded) = response
        .data_url
        .split_once(',')
        .ok_or_else(|| "Remote Hermes returned an invalid image data URL".to_owned())?;
    if !metadata.starts_with("data:image/") || !metadata.ends_with(";base64") {
        return Err("Remote Hermes returned an unsupported image data URL".into());
    }
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "Remote Hermes returned invalid image data".to_owned())?;
    let is_svg = metadata[11..metadata.len() - 7].eq_ignore_ascii_case("svg+xml");
    decode_artifact_image_preview(&bytes, is_svg)
}

fn artifact_download_url(
    endpoint: &GatewayEndpoint,
    request: &ArtifactFileDownloadRequest,
) -> Result<String, String> {
    if endpoint.remote_auth.is_none()
        || request.request_id.is_empty()
        || request.request_id.len() > 256
        || request.path.trim().is_empty()
        || request.path.len() > 16_384
        || request.path.chars().any(char::is_control)
        || request.session_id.is_empty()
        || request.session_id.len() > 256
        || request.session_id.chars().any(char::is_control)
        || request.profile.is_empty()
        || request.profile.len() > 128
        || request.profile.chars().any(char::is_control)
    {
        return Err("Remote artifact download scope is invalid".into());
    }
    let url = format!(
        "{}/api/fs/download?path={}&session_id={}&profile={}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(&request.path),
        encode_query_component(&request.session_id),
        encode_query_component(&request.profile)
    );
    if !api_url_is_within_origin(&endpoint.api_origin, &url) {
        return Err("Remote artifact download target is outside the configured gateway".into());
    }
    Ok(url)
}

fn safe_filename_extension(filename: &str) -> Option<String> {
    let name = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    (extension.len() <= 16
        && !extension.is_empty()
        && extension.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    .then_some(extension)
}

fn decode_header_filename(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value
        .strip_prefix("UTF-8''")
        .or_else(|| value.strip_prefix("utf-8''"))
        .unwrap_or(value)
        .trim_matches('"');
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&value[index + 1..index + 3], 16)
        {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let value = String::from_utf8(decoded).ok()?;
    let basename = value.rsplit(['/', '\\']).next()?.trim();
    if basename.is_empty()
        || basename == "."
        || basename == ".."
        || basename.chars().any(char::is_control)
    {
        return None;
    }
    Some(basename.to_owned())
}

fn artifact_download_filename(content_disposition: Option<&str>, path: &str) -> String {
    if let Some(header) = content_disposition {
        let lower = header.to_ascii_lowercase();
        if let Some(start) = lower.find("filename*=") {
            let value = header[start + "filename*=".len()..]
                .split(';')
                .next()
                .unwrap_or("");
            if let Some(name) = decode_header_filename(value) {
                return name;
            }
        }
        if let Some(start) = lower.find("filename=") {
            let value = header[start + "filename=".len()..]
                .split(';')
                .next()
                .unwrap_or("");
            if let Some(name) = decode_header_filename(value) {
                return name;
            }
        }
    }
    let fallback = path.rsplit(['/', '\\']).next().unwrap_or("download");
    let fallback = fallback.strip_prefix("file://").unwrap_or(fallback);
    let fallback = decode_header_filename(fallback).unwrap_or_else(|| "download".into());
    fallback
}

fn download_remote_artifact_file(
    endpoint: &GatewayEndpoint,
    request: &ArtifactFileDownloadRequest,
    choose_destination: impl FnOnce(&str) -> Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let url = artifact_download_url(endpoint, request)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_per_read(Some(Duration::from_secs(60)))
        .max_redirects(0)
        .build()
        .into();
    let download = match endpoint.remote_auth.as_ref() {
        Some(RemoteGatewayAuth::Token(token)) => agent
            .get(&url)
            .header("X-Hermes-Session-Token", token.as_str()),
        Some(auth @ RemoteGatewayAuth::OAuth { .. }) => {
            let token = remote_access_token(&endpoint.api_origin, auth)?;
            agent
                .get(&url)
                .header("Authorization", &format!("Bearer {}", token.as_str()))
        }
        None => return Err("Remote Hermes credentials are unavailable".into()),
    };
    let response = download
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|_| "Remote Hermes artifact download failed".to_owned())?;
    let content_disposition = response
        .headers()
        .get("content-disposition")
        .and_then(|value| value.to_str().ok());
    let filename = artifact_download_filename(content_disposition, &request.path);
    let Some(destination) = choose_destination(&filename) else {
        return Ok(None);
    };
    let mut body = response.into_body().into_reader();
    save_stream_atomically(&mut body, &destination)?;
    Ok(Some(destination))
}

fn save_stream_atomically(reader: &mut impl Read, destination: &PathBuf) -> Result<(), String> {
    use std::fs::{self, OpenOptions};
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut temporary = None;
    for _ in 0..8 {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".hermes-download-{}-{id}.part", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Could not create a temporary download file: {error}"
                ));
            }
        }
    }
    let Some((temporary_path, mut file)) = temporary else {
        return Err("Could not reserve a unique temporary download file".into());
    };
    let result = (|| {
        io::copy(reader, &mut file)
            .map_err(|error| format!("Could not receive the remote Hermes file: {error}"))?;
        file.flush()
            .map_err(|error| format!("Could not flush the remote Hermes file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not sync the remote Hermes file: {error}"))?;
        drop(file);
        fs::rename(&temporary_path, destination)
            .map_err(|error| format!("Could not install the downloaded Hermes file: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn gateway_artifact_actor(
    endpoint: GatewayEndpoint,
    requests: Receiver<String>,
    events: mpsc::Sender<GatewayEvent>,
) {
    while let Ok(request_id) = requests.recv() {
        let event = GatewayEvent::ArtifactsLoaded {
            request_id,
            result: load_artifact_index_http(&endpoint),
        };
        if events.send(event).is_err() {
            return;
        }
    }
}

fn load_artifact_index_http(endpoint: &GatewayEndpoint) -> Result<ArtifactIndexSnapshot, String> {
    let (sessions, local_ticket) = if endpoint.remote_auth.is_some() {
        let path = format!(
            "{}/api/profiles/sessions?limit={ARTIFACT_SESSION_LIMIT}&offset=0&min_messages=1&archived=exclude&order=recent&profile=all",
            endpoint.api_origin.trim_end_matches('/')
        );
        let page = request_remote_session_page(endpoint, &path)?;
        (page.sessions, None)
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err(
                "Hermes artifact indexing requires the verified loopback API origin".into(),
            );
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        let url = format!(
            "{}/api/profiles/sessions?limit={ARTIFACT_SESSION_LIMIT}&offset=0&min_messages=1&archived=exclude&order=recent&profile=all",
            endpoint.api_origin.trim_end_matches('/')
        );
        let response: SessionListResponse =
            request_gateway_json(&endpoint.api_origin, ticket.as_str(), &url)?;
        if response.sessions.len() > 200 {
            return Err("Hermes returned too many recent artifact sessions".into());
        }
        let sessions = response
            .sessions
            .into_iter()
            .map(session_summary_from_row)
            .collect();
        (sessions, Some(ticket))
    };
    if sessions.len() > 200 {
        return Err("Hermes returned too many recent artifact sessions".into());
    }
    let sessions = sessions
        .into_iter()
        .map(|mut session: SessionSummary| {
            if session.profile.is_none() {
                session.profile = Some(endpoint.profile_id.clone());
            }
            ArtifactSession {
                session_id: session.id,
                profile: session.profile,
                title: session.title,
                preview: session.preview,
                started_at: session.started_at,
                last_active: session.last_active,
            }
        })
        .collect::<Vec<_>>();
    let index = load_artifacts_for_sessions(&sessions, |session, limit, offset| {
        request_artifact_messages_page(
            endpoint,
            local_ticket.as_deref().map(String::as_str),
            session,
            limit,
            offset,
        )
    });
    Ok(index)
}

fn session_summary_from_row(session: SessionListRow) -> SessionSummary {
    SessionSummary {
        id: session.id,
        connection_id: session.connection_id,
        lineage_root_id: session.lineage_root_id,
        cwd: session.cwd,
        profile: session.profile,
        title: session.title.unwrap_or_default(),
        preview: session.preview.unwrap_or_default(),
        message_count: session.message_count,
        source: session.source.unwrap_or_default(),
        started_at: session.started_at,
        last_active: session.last_active,
        pinned: session.pinned,
        unread: session.unread,
        archived: session.archived,
    }
}

fn request_artifact_messages_page(
    endpoint: &GatewayEndpoint,
    local_ticket: Option<&str>,
    session: &ArtifactSession,
    limit: usize,
    offset: usize,
) -> Result<ArtifactMessagePage, String> {
    if session.session_id.is_empty()
        || session.session_id.len() > 256
        || session.session_id.chars().any(char::is_control)
        || limit != crate::artifacts::ARTIFACT_MESSAGE_PAGE_SIZE
        || offset > 10_000_000
        || session.profile.as_deref().is_some_and(|value| {
            value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
        })
    {
        return Err("Hermes artifact transcript request scope is invalid".into());
    }
    let mut url = format!(
        "{}/api/sessions/{}/messages?",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(&session.session_id)
    );
    if let Some(profile) = &session.profile {
        url.push_str(&format!("profile={}&", encode_query_component(profile)));
    }
    url.push_str(&format!(
        "limit={limit}&offset={offset}&order=oldest&include_compacted=true"
    ));
    let page: SessionMessagesPageResponse = if endpoint.remote_auth.is_some() {
        request_remote_gateway_json_with_limit(endpoint, &url, ARTIFACT_GATEWAY_RESPONSE_LIMIT)?
    } else {
        let ticket =
            local_ticket.ok_or_else(|| "Hermes local artifact ticket is unavailable".to_owned())?;
        request_gateway_json_with_limit(
            &endpoint.api_origin,
            ticket,
            &url,
            ARTIFACT_GATEWAY_RESPONSE_LIMIT,
        )?
    };
    if page.session_id != session.session_id {
        return Err("Hermes artifact transcript response belongs to a different session".into());
    }
    if page
        .profile
        .as_deref()
        .is_some_and(|returned| Some(returned) != session.profile.as_deref())
    {
        return Err("Hermes artifact transcript response belongs to a different profile".into());
    }
    let pagination = page.pagination.map(|page| ArtifactMessagePagination {
        limit: page.limit,
        offset: page.offset,
        order: page.order.unwrap_or_default(),
        returned: page.returned,
    });
    Ok(ArtifactMessagePage {
        messages: page.messages,
        pagination,
    })
}

fn gateway_cron_trigger_actor(
    endpoint: GatewayEndpoint,
    commands: Receiver<CronTriggerCommand>,
    events: mpsc::Sender<GatewayEvent>,
) {
    let (permit_sender, permit_receiver) = mpsc::sync_channel::<()>(8);
    for _ in 0..8 {
        if permit_sender.send(()).is_err() {
            return;
        }
    }
    while let Ok(command) = commands.recv() {
        if permit_receiver.try_recv().is_err() {
            let _ = events.send(GatewayEvent::CronJobTriggered {
                request_id: command.request_id,
                profile: command.profile,
                job_id: command.job_id,
                result: Err("Hermes Scheduled Job trigger capacity is full".into()),
            });
            continue;
        }
        let endpoint = endpoint.clone();
        let events = events.clone();
        let failed_events = events.clone();
        let permits = permit_sender.clone();
        let request_id = command.request_id;
        let profile = command.profile;
        let job_id = command.job_id;
        let failed_request_id = request_id.clone();
        let failed_profile = profile.clone();
        let failed_job_id = job_id.clone();
        let worker = thread::Builder::new()
            .name("hermes-cron-trigger-request".into())
            .spawn(move || {
                let result = trigger_cron_job_http(&endpoint, profile.as_deref(), &job_id);
                let _ = events.send(GatewayEvent::CronJobTriggered {
                    request_id,
                    profile,
                    job_id,
                    result,
                });
                let _ = permits.send(());
            });
        if worker.is_err() {
            let _ = permit_sender.send(());
            let _ = failed_events.send(GatewayEvent::CronJobTriggered {
                request_id: failed_request_id,
                profile: failed_profile,
                job_id: failed_job_id,
                result: Err("Could not start Hermes Scheduled Job trigger worker".into()),
            });
        }
    }
}

fn gateway_http_actor(
    endpoint: GatewayEndpoint,
    commands: Receiver<GatewayHttpCommand>,
    events: mpsc::Sender<GatewayEvent>,
) {
    while let Ok(command) = commands.recv() {
        // Preserve FIFO and never coalesce a write. Search is already debounced
        // by the UI; each accepted mutation must produce its own receipt.
        let event = match command {
            GatewayHttpCommand::LoadOlderTranscript {
                request_id,
                session_id,
                profile,
                offset,
            } => GatewayEvent::OlderTranscriptLoaded {
                result: load_older_transcript_http(
                    &endpoint,
                    &session_id,
                    profile.as_deref(),
                    offset,
                ),
                request_id,
                session_id,
                profile,
                offset,
            },
            GatewayHttpCommand::RefreshActiveTranscript {
                request_id,
                session_id,
                profile,
                runtime_session_id,
            } => GatewayEvent::ActiveTranscriptRefreshed {
                result: load_active_transcript_http(&endpoint, &session_id, profile.as_deref()),
                request_id,
                session_id,
                profile,
                endpoint_profile_id: endpoint.profile_id.clone(),
                runtime_session_id,
            },
            GatewayHttpCommand::LoadSessions => {
                GatewayEvent::SessionsLoaded(load_sessions_http(&endpoint))
            }
            GatewayHttpCommand::RefreshSessions { request_id, limit } => {
                GatewayEvent::SessionsRefreshed {
                    request_id,
                    profile_id: endpoint.profile_id.clone(),
                    result: load_command_center_sessions_http(&endpoint, limit),
                }
            }
            GatewayHttpCommand::LoadArchivedSessions { request_id } => {
                GatewayEvent::ArchivedSessionsLoaded {
                    request_id,
                    result: load_archived_sessions_http(&endpoint),
                }
            }
            GatewayHttpCommand::LoadCommandPaletteSessions { request_id } => {
                GatewayEvent::CommandPaletteSessionsLoaded {
                    request_id,
                    result: load_command_palette_sessions_http(&endpoint),
                }
            }
            GatewayHttpCommand::SearchSessions { request_id, query } => {
                GatewayEvent::SessionsSearched {
                    request_id,
                    query: query.clone(),
                    result: search_sessions_http(&endpoint, &query),
                }
            }
            GatewayHttpCommand::LoadCapabilities { request_id } => {
                GatewayEvent::CapabilitiesLoaded {
                    request_id,
                    result: load_capabilities_http(&endpoint),
                }
            }
            GatewayHttpCommand::LoadMessagingPlatforms {
                request_id,
                profile,
            } => GatewayEvent::MessagingPlatformsLoaded {
                request_id,
                profile: profile.clone(),
                result: load_messaging_platforms_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::LoadCronJobs {
                request_id,
                profile,
            } => GatewayEvent::CronJobsLoaded {
                request_id,
                profile: profile.clone(),
                result: load_cron_jobs_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::LoadModelOptions {
                request_id,
                profile,
            } => GatewayEvent::ModelOptionsLoaded {
                request_id,
                profile: profile.clone(),
                result: load_model_options_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::LoadCronDeliveryTargets {
                request_id,
                profile,
            } => GatewayEvent::CronDeliveryTargetsLoaded {
                request_id,
                profile: profile.clone(),
                result: load_cron_delivery_targets_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::LoadCronBlueprints {
                request_id,
                profile,
            } => GatewayEvent::CronBlueprintsLoaded {
                request_id,
                profile: profile.clone(),
                result: load_cron_blueprints_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::InstantiateCronBlueprint {
                request_id,
                profile,
                payload,
            } => GatewayEvent::CronBlueprintInstantiated {
                request_id,
                profile: profile.clone(),
                blueprint: payload.blueprint.clone(),
                result: instantiate_cron_blueprint_http(&endpoint, profile.as_deref(), &payload),
            },
            GatewayHttpCommand::CreateCronJob {
                request_id,
                profile,
                payload,
            } => GatewayEvent::CronJobCreated {
                request_id,
                profile: profile.clone(),
                result: create_cron_job_http(&endpoint, profile.as_deref(), &payload),
            },
            GatewayHttpCommand::UpdateCronJob {
                request_id,
                profile,
                job_id,
                updates,
            } => GatewayEvent::CronJobUpdated {
                request_id,
                profile: profile.clone(),
                job_id: job_id.clone(),
                result: update_cron_job_http(&endpoint, profile.as_deref(), &job_id, &updates),
            },
            GatewayHttpCommand::DeleteCronJob {
                request_id,
                profile,
                job_id,
            } => GatewayEvent::CronJobDeleted {
                request_id,
                profile: profile.clone(),
                job_id: job_id.clone(),
                result: delete_cron_job_http(&endpoint, profile.as_deref(), &job_id),
            },
            GatewayHttpCommand::LoadCronJobRuns {
                request_id,
                profile,
                job_id,
            } => GatewayEvent::CronJobRunsLoaded {
                request_id,
                profile: profile.clone(),
                job_id: job_id.clone(),
                result: load_cron_job_runs_http(&endpoint, profile.as_deref(), &job_id),
            },
            GatewayHttpCommand::SetCronJobPaused {
                request_id,
                profile,
                job_id,
                paused,
            } => GatewayEvent::CronJobPauseChanged {
                request_id,
                profile: profile.clone(),
                job_id: job_id.clone(),
                paused,
                result: set_cron_job_paused_http(&endpoint, profile.as_deref(), &job_id, paused),
            },
            GatewayHttpCommand::CancelCronJobRun {
                request_id,
                profile,
                job_id,
            } => GatewayEvent::CronJobRunCancelled {
                request_id,
                profile: profile.clone(),
                job_id: job_id.clone(),
                result: cancel_cron_job_run_http(&endpoint, profile.as_deref(), &job_id),
            },
            GatewayHttpCommand::SetMessagingPlatformEnabled {
                request_id,
                profile,
                platform_id,
                enabled,
            } => GatewayEvent::MessagingPlatformUpdated {
                request_id,
                profile: profile.clone(),
                platform_id: platform_id.clone(),
                enabled,
                result: set_messaging_platform_enabled_http(
                    &endpoint,
                    profile.as_deref(),
                    &platform_id,
                    enabled,
                ),
            },
            GatewayHttpCommand::UpdateMessagingPlatformConfig {
                request_id,
                profile,
                platform_id,
                update,
            } => GatewayEvent::MessagingPlatformConfigUpdated {
                request_id,
                profile: profile.clone(),
                platform_id: platform_id.clone(),
                result: update_messaging_platform_http(
                    &endpoint,
                    profile.as_deref(),
                    &platform_id,
                    update,
                ),
            },
            GatewayHttpCommand::LoadPairing {
                request_id,
                profile,
            } => GatewayEvent::PairingLoaded {
                request_id,
                profile: profile.clone(),
                result: load_pairing_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::MutatePairing {
                request_id,
                profile,
                platform,
                user_id,
                kind,
            } => GatewayEvent::PairingMutated {
                request_id,
                profile: profile.clone(),
                result: mutate_pairing_http(
                    &endpoint,
                    profile.as_deref(),
                    &platform,
                    &user_id,
                    kind,
                ),
                platform,
                user_id,
                kind,
            },
            GatewayHttpCommand::StartTelegramOnboarding {
                request_id,
                profile,
                bot_name,
            } => GatewayEvent::TelegramOnboardingStarted {
                request_id,
                profile: profile.clone(),
                result: start_telegram_onboarding_http(
                    &endpoint,
                    profile.as_deref(),
                    bot_name.as_deref(),
                ),
            },
            GatewayHttpCommand::LoadTelegramOnboardingStatus {
                request_id,
                pairing_id,
                profile,
            } => GatewayEvent::TelegramOnboardingStatusLoaded {
                request_id,
                pairing_id: pairing_id.clone(),
                profile: profile.clone(),
                result: load_telegram_onboarding_status_http(
                    &endpoint,
                    &pairing_id,
                    profile.as_deref(),
                ),
            },
            GatewayHttpCommand::ApplyTelegramOnboarding {
                request_id,
                pairing_id,
                profile,
                allowed_user_ids,
            } => GatewayEvent::TelegramOnboardingApplied {
                request_id,
                pairing_id: pairing_id.clone(),
                profile: profile.clone(),
                result: apply_telegram_onboarding_http(
                    &endpoint,
                    &pairing_id,
                    profile.as_deref(),
                    &allowed_user_ids,
                ),
            },
            GatewayHttpCommand::CancelTelegramOnboarding {
                request_id,
                pairing_id,
                profile,
            } => GatewayEvent::TelegramOnboardingCanceled {
                request_id,
                pairing_id: pairing_id.clone(),
                profile: profile.clone(),
                result: cancel_telegram_onboarding_http(&endpoint, &pairing_id, profile.as_deref()),
            },
            GatewayHttpCommand::RestartGateway {
                request_id,
                profile,
            } => GatewayEvent::GatewayRestartStarted {
                request_id,
                profile: profile.clone(),
                result: restart_gateway_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::LoadGatewayActionStatus {
                request_id,
                action_name,
                profile,
            } => GatewayEvent::GatewayActionStatusLoaded {
                request_id,
                profile: profile.clone(),
                action_name: action_name.clone(),
                result: load_gateway_action_status_http(
                    &endpoint,
                    &action_name,
                    profile.as_deref(),
                ),
            },
            GatewayHttpCommand::LoadCommandCenterSystem {
                request_id,
                profile,
                log_file,
                log_level,
            } => GatewayEvent::CommandCenterSystemLoaded {
                request_id,
                profile: profile.clone(),
                log_file: log_file.clone(),
                log_level: log_level.clone(),
                result: load_command_center_system_http(
                    &endpoint,
                    profile.as_deref(),
                    &log_file,
                    &log_level,
                ),
            },
            GatewayHttpCommand::CommandCenterSystemAction {
                request_id,
                profile,
                operation,
            } => GatewayEvent::CommandCenterSystemAction {
                request_id,
                profile: profile.clone(),
                result: command_center_system_action_http(
                    &endpoint,
                    profile.as_deref(),
                    &operation,
                ),
                operation,
            },
            GatewayHttpCommand::LoadCommandCenterUsage {
                request_id,
                profile,
                period_days,
            } => GatewayEvent::CommandCenterUsageLoaded {
                request_id,
                profile: profile.clone(),
                period_days,
                result: load_command_center_usage_http(&endpoint, profile.as_deref(), period_days),
            },
            GatewayHttpCommand::CommandCenterMaintenance {
                request_id,
                profile,
                operation,
            } => GatewayEvent::CommandCenterMaintenance {
                request_id,
                profile: profile.clone(),
                result: command_center_maintenance_http(&endpoint, profile.as_deref(), &operation),
                operation,
            },
            GatewayHttpCommand::LoadDisplayTimestamps {
                request_id,
                profile,
            } => GatewayEvent::DisplayTimestampsLoaded {
                request_id,
                profile: profile.clone(),
                result: load_display_timestamps_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::LoadRepoScanConfig {
                request_id,
                profile,
            } => GatewayEvent::RepoScanConfigLoaded {
                request_id,
                profile: profile.clone(),
                result: load_repo_scan_config_http(&endpoint, profile.as_deref()),
            },
            GatewayHttpCommand::SaveRepoScanConfig {
                request_id,
                profile,
                policy,
            } => GatewayEvent::RepoScanConfigSaved {
                request_id,
                profile: profile.clone(),
                result: save_repo_scan_config_http(&endpoint, profile.as_deref(), &policy),
            },
            GatewayHttpCommand::SetToolsetEnabled {
                request_id,
                name,
                enabled,
            } => GatewayEvent::ToolsetEnabled {
                request_id,
                name: name.clone(),
                result: set_toolset_enabled_http(&endpoint, &name, enabled),
            },
            GatewayHttpCommand::SetSessionPinned {
                request_id,
                session_id,
                profile,
                pinned,
            } => {
                let result = set_session_pinned_http(
                    &endpoint,
                    &session_id,
                    profile.as_deref(),
                    pinned,
                    &request_id,
                );
                GatewayEvent::SessionPinUpdated {
                    request_id,
                    session_id,
                    profile,
                    pinned,
                    result,
                }
            }
            GatewayHttpCommand::SetSessionUnread {
                request_id,
                session_id,
                profile,
                unread,
            } => {
                let result = set_session_unread_http(
                    &endpoint,
                    &session_id,
                    profile.as_deref(),
                    unread,
                    &request_id,
                );
                GatewayEvent::SessionUnreadUpdated {
                    request_id,
                    session_id,
                    profile,
                    unread,
                    result,
                }
            }
            GatewayHttpCommand::SetSessionArchived {
                request_id,
                session_id,
                profile,
                archived,
            } => {
                let result = set_session_archived_http(
                    &endpoint,
                    &session_id,
                    profile.as_deref(),
                    archived,
                    &request_id,
                );
                GatewayEvent::SessionArchiveUpdated {
                    request_id,
                    session_id,
                    profile,
                    archived,
                    result,
                }
            }
            GatewayHttpCommand::RenameSession {
                request_id,
                session_id,
                profile,
                title,
            } => GatewayEvent::SessionRenamed {
                result: rename_session_http(
                    &endpoint,
                    &session_id,
                    profile.as_deref(),
                    &title,
                    &request_id,
                ),
                request_id,
                session_id,
                profile,
            },
            GatewayHttpCommand::DeleteSession {
                request_id,
                session_id,
                profile,
            } => GatewayEvent::SessionDeleted {
                result: delete_session_http(
                    &endpoint,
                    &session_id,
                    profile.as_deref(),
                    &request_id,
                ),
                request_id,
                session_id,
                profile,
            },
            GatewayHttpCommand::ExportSession {
                request_id,
                session_id,
                profile,
            } => GatewayEvent::SessionExportLoaded {
                result: load_all_session_messages_http(&endpoint, &session_id, profile.as_deref()),
                request_id,
                session_id,
                profile,
            },
        };
        if events.send(event).is_err() {
            return;
        }
    }
}

fn gateway_project_tree_actor(
    endpoint: GatewayEndpoint,
    commands: Receiver<ProjectTreeHttpCommand>,
    events: mpsc::Sender<GatewayEvent>,
) {
    while let Ok(command) = commands.recv() {
        let result = load_all_profile_project_tree_http(&endpoint, command.preview_limit);
        if events
            .send(GatewayEvent::ProjectTreeLoaded {
                request_id: command.request_id,
                profile: endpoint.profile_id.clone(),
                result,
            })
            .is_err()
        {
            break;
        }
    }
}

fn load_all_profile_project_tree_http(
    endpoint: &GatewayEndpoint,
    preview_limit: usize,
) -> Result<ProjectTreeSnapshot, String> {
    if !(1..=2000).contains(&preview_limit) {
        return Err(
            "Hermes all-profile project tree preview limit is outside the supported range".into(),
        );
    }
    let url = format!(
        "{}/api/profiles/projects/tree?preview_limit={preview_limit}",
        endpoint.api_origin.trim_end_matches('/')
    );
    let mut bytes = Vec::new();
    if endpoint.remote_auth.is_some() {
        if !api_url_is_within_origin(&endpoint.api_origin, &url) {
            return Err("Remote Hermes project tree request target is invalid".into());
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .max_redirects(0)
            .build()
            .into();
        let auth = remote_http_auth(endpoint)?;
        let mut request = agent.get(&url).header("Accept", "application/json");
        request = match auth {
            RemoteHttpAuth::SessionToken(token) => {
                request.header("X-Hermes-Session-Token", token.as_str())
            }
            RemoteHttpAuth::Bearer(token) => {
                request.header("Authorization", &format!("Bearer {}", token.as_str()))
            }
        };
        request
            .call()
            .map_err(|_| "Remote Hermes all-profile project tree read failed".to_owned())?
            .into_body()
            .into_reader()
            .take(PROJECT_TREE_HTTP_RESPONSE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read remote Hermes project tree response".to_owned())?;
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes all-profile project tree requires verified loopback origin".into());
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(60)))
            .max_redirects(0)
            .build()
            .into();
        agent
            .get(&url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .call()
            .map_err(|_| "Hermes all-profile project tree read failed".to_owned())?
            .into_body()
            .into_reader()
            .take(PROJECT_TREE_HTTP_RESPONSE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read Hermes project tree response".to_owned())?;
    }
    if bytes.len() as u64 > PROJECT_TREE_HTTP_RESPONSE_LIMIT {
        return Err("Hermes project tree response exceeded its 8 MiB read limit".into());
    }
    let tree = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| "Hermes returned invalid all-profile project tree JSON".to_owned())
        .and_then(parse_project_tree)?;
    validate_all_profile_project_tree(&tree)?;
    Ok(tree)
}

fn load_sessions_http(endpoint: &GatewayEndpoint) -> Result<Vec<SessionSummary>, String> {
    if endpoint.remote_auth.is_some() {
        let path = format!(
            "{}/api/sessions?limit=200&offset=0&min_messages=0&archived=exclude&order=recent",
            endpoint.api_origin.trim_end_matches('/')
        );
        let mut sessions = request_remote_session_page(endpoint, &path)?.sessions;
        for session in &mut sessions {
            if session.profile.is_none() {
                session.profile = Some(endpoint.profile_id.clone());
            }
        }
        return Ok(sessions);
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes session list requires the verified loopback API origin".into());
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    let mut sessions = request_session_list(&endpoint.api_origin, ticket.as_str())?;
    for session in &mut sessions {
        if session.profile.is_none() {
            session.profile = Some(endpoint.profile_id.clone());
        }
    }
    Ok(sessions)
}

fn load_command_center_sessions_http(
    endpoint: &GatewayEndpoint,
    limit: usize,
) -> Result<CommandCenterSessionSnapshot, String> {
    const RECENTS_EXCLUDED: &str = "acp%2Ccron%2Ckanban%2Coneshot%2Csubagent%2Ctool%2Ctelegram%2Cdiscord%2Cslack%2Cmattermost%2Cmatrix%2Csignal%2Cwhatsapp%2Cbluebubbles%2Cphoton%2Chomeassistant%2Cemail%2Csms%2Cwebhook%2Capi_server%2Cweixin%2Cwecom%2Cqqbot%2Cyuanbao%2Cdingtalk%2Cfeishu";
    const MESSAGING_EXCLUDED: &str =
        "cron%2Cacp%2Ccli%2Ccodex%2Cdesktop%2Cgateway%2Ckanban%2Clocal%2Coneshot%2Ctui";
    let limit = limit.clamp(1, 2_000);
    let profile = encode_query_component(&endpoint.profile_id);
    let url = format!(
        "{}/api/profiles/sessions/sidebar?recents_profile={profile}&recents_limit={limit}&cron_limit=50&messaging_limit=100&recents_exclude={RECENTS_EXCLUDED}&messaging_exclude={MESSAGING_EXCLUDED}",
        endpoint.api_origin.trim_end_matches('/')
    );
    let response: SidebarSessionsResponse = if endpoint.remote_auth.is_some() {
        match request_remote_gateway_json_with_timeout(endpoint, &url, Duration::from_secs(60)) {
            Ok(response) => response,
            Err(error) if is_missing_sidebar_endpoint(&error) => {
                return load_command_center_sessions_legacy(endpoint, limit, &profile);
            }
            Err(error) => return Err(error),
        }
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes session list requires the verified loopback API origin".into());
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        match request_gateway_json_with_timeout(
            &endpoint.api_origin,
            ticket.as_str(),
            &url,
            Duration::from_secs(60),
        ) {
            Ok(response) => response,
            Err(error) if is_missing_sidebar_endpoint(&error) => {
                return load_command_center_sessions_legacy(endpoint, limit, &profile);
            }
            Err(error) => return Err(error),
        }
    };
    let mut rows = response.recents.sessions;
    let mut sessions = Vec::with_capacity(rows.len());
    for row in rows.drain(..) {
        let mut session = row.into_summary();
        if session.profile.is_none() {
            if endpoint.profile_id.eq_ignore_ascii_case("all") {
                return Err(format!(
                    "Hermes all-profile session {} has no profile owner",
                    session.id
                ));
            }
            session.profile = Some(endpoint.profile_id.clone());
        }
        sessions.push(session);
    }
    let mut failed_profiles = BTreeSet::new();
    for error in response.errors.iter().chain(response.recents.errors.iter()) {
        failed_profiles.insert(error.profile.clone());
    }
    failed_profiles.extend(response.profiles_failed.keys().cloned());
    failed_profiles.extend(response.recents.profiles_failed.keys().cloned());
    if response.recents.failed && failed_profiles.is_empty() {
        return Err("Hermes Command Center session list failed without profile details".into());
    }
    let error = response
        .errors
        .iter()
        .chain(response.recents.errors.iter())
        .filter(|error| !error.error.trim().is_empty())
        .map(|error| format!("{}: {}", error.profile, error.error.trim()))
        .collect::<Vec<_>>();
    Ok(CommandCenterSessionSnapshot {
        sessions,
        profiles_truncated: response.recents.profiles_truncated,
        failed_profiles,
        error: (!error.is_empty()).then(|| error.join("; ")),
    })
}

fn is_missing_sidebar_endpoint(error: &str) -> bool {
    error.contains("404")
}

fn load_command_center_sessions_legacy(
    endpoint: &GatewayEndpoint,
    limit: usize,
    encoded_profile: &str,
) -> Result<CommandCenterSessionSnapshot, String> {
    const RECENTS_EXCLUDED: &str = "acp%2Ccron%2Ckanban%2Coneshot%2Csubagent%2Ctool%2Ctelegram%2Cdiscord%2Cslack%2Cmattermost%2Cmatrix%2Csignal%2Cwhatsapp%2Cbluebubbles%2Cphoton%2Chomeassistant%2Cemail%2Csms%2Cwebhook%2Capi_server%2Cweixin%2Cwecom%2Cqqbot%2Cyuanbao%2Cdingtalk%2Cfeishu";
    let limit = limit.clamp(1, 2_000);
    let url = format!(
        "{}/api/profiles/sessions?limit={limit}&offset=0&min_messages=1&archived=exclude&order=recent&profile={encoded_profile}&exclude_sources={RECENTS_EXCLUDED}",
        endpoint.api_origin.trim_end_matches('/')
    );
    let response: SessionListResponse = if endpoint.remote_auth.is_some() {
        request_remote_gateway_json_with_timeout(endpoint, &url, Duration::from_secs(60))?
    } else {
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        request_gateway_json_with_timeout(
            &endpoint.api_origin,
            ticket.as_str(),
            &url,
            Duration::from_secs(60),
        )?
    };
    let mut sessions = response
        .sessions
        .into_iter()
        .map(SessionListRow::into_summary)
        .collect::<Vec<_>>();
    for session in &mut sessions {
        if session.profile.is_none() {
            if endpoint.profile_id.eq_ignore_ascii_case("all") {
                return Err(format!(
                    "Hermes all-profile session {} has no profile owner",
                    session.id
                ));
            }
            session.profile = Some(endpoint.profile_id.clone());
        }
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for session in &sessions {
        *counts
            .entry(session.profile.clone().unwrap_or_else(|| "default".into()))
            .or_default() += 1;
    }
    Ok(CommandCenterSessionSnapshot {
        sessions,
        profiles_truncated: counts
            .into_iter()
            .map(|(profile, count)| (profile, count >= limit))
            .collect(),
        ..Default::default()
    })
}

#[derive(Deserialize, Default)]
struct SidebarSessionsResponse {
    #[serde(default)]
    recents: SidebarSessionSlice,
    #[serde(default)]
    errors: Vec<SidebarSessionError>,
    #[serde(default)]
    profiles_failed: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Default)]
struct SidebarSessionSlice {
    #[serde(default)]
    sessions: Vec<SessionListRow>,
    #[serde(default)]
    profiles_truncated: BTreeMap<String, bool>,
    #[serde(default)]
    errors: Vec<SidebarSessionError>,
    #[serde(default)]
    profiles_failed: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    failed: bool,
}

#[derive(Deserialize)]
struct SidebarSessionError {
    profile: String,
    #[serde(default)]
    error: String,
}

fn load_command_palette_sessions_http(
    endpoint: &GatewayEndpoint,
) -> Result<Vec<SessionSummary>, String> {
    if endpoint.remote_auth.is_some() {
        let path = format!(
            "{}/api/profiles/sessions?limit=200&offset=0&min_messages=1&archived=exclude&order=recent&profile=all",
            endpoint.api_origin.trim_end_matches('/')
        );
        let mut sessions = request_remote_session_page(endpoint, &path)?.sessions;
        for session in &mut sessions {
            if session.profile.is_none() {
                session.profile = Some(endpoint.profile_id.clone());
            }
        }
        return Ok(sessions);
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err(
            "Hermes command-palette sessions require the verified loopback API origin".into(),
        );
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    request_all_profile_command_palette_sessions(
        &endpoint.api_origin,
        ticket.as_str(),
        Some(&endpoint.profile_id),
    )
}

fn request_all_profile_command_palette_sessions(
    api_origin: &str,
    ticket: &str,
    owner_profile: Option<&str>,
) -> Result<Vec<SessionSummary>, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err(
            "Hermes command-palette sessions require the verified loopback API origin".into(),
        );
    }
    let url = format!(
        "{}/api/profiles/sessions?limit=200&offset=0&min_messages=1&archived=exclude&order=recent&profile=all",
        api_origin.trim_end_matches('/')
    );
    let response: SessionListResponse = request_gateway_json(api_origin, ticket, &url)?;
    let mut sessions = response
        .sessions
        .into_iter()
        .map(|session| SessionSummary {
            id: session.id,
            connection_id: session.connection_id,
            lineage_root_id: session.lineage_root_id,
            cwd: session.cwd,
            profile: session.profile,
            title: session.title.unwrap_or_default(),
            preview: session.preview.unwrap_or_default(),
            message_count: session.message_count,
            source: session.source.unwrap_or_default(),
            started_at: session.started_at,
            last_active: session.last_active,
            pinned: session.pinned,
            unread: session.unread,
            archived: session.archived,
        })
        .collect::<Vec<_>>();
    for session in &mut sessions {
        if session.profile.is_none() {
            session.profile = owner_profile.map(str::to_owned);
        }
    }
    Ok(sessions)
}
fn load_archived_sessions_http(endpoint: &GatewayEndpoint) -> Result<Vec<SessionSummary>, String> {
    if endpoint.remote_auth.is_some() {
        return collect_archived_session_pages(|offset| {
            let path = format!(
                "{}/api/profiles/sessions?limit=500&offset={offset}&min_messages=0&archived=only&order=recent&profile=all",
                endpoint.api_origin.trim_end_matches('/')
            );
            request_remote_session_page(endpoint, &path)
        });
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes archived sessions require the verified loopback API origin".into());
    }
    collect_archived_session_pages(|offset| {
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        request_all_profile_archived_session_page(&endpoint.api_origin, ticket.as_str(), offset)
    })
}

fn collect_archived_session_pages(
    mut read_page: impl FnMut(usize) -> Result<SessionListPage, String>,
) -> Result<Vec<SessionSummary>, String> {
    const PAGE_SIZE: usize = 500;

    let mut sessions = Vec::new();
    let mut seen = HashSet::new();
    let mut archived_count = 0;
    let mut offset = 0;

    loop {
        let page = read_page(offset)?;
        let before = archived_count;
        for session in page.sessions {
            let key = format!(
                "{}\0{}",
                session.profile.as_deref().unwrap_or("default"),
                session.id
            );
            if seen.insert(key) {
                if session.archived {
                    archived_count += 1;
                }
                sessions.push(session);
            }
        }
        offset = offset
            .checked_add(PAGE_SIZE)
            .ok_or_else(|| "Hermes archived-session pagination offset overflowed".to_owned())?;
        if archived_count >= page.total
            || (offset >= page.total && archived_count == before)
            || sessions.is_empty()
        {
            return Ok(sessions);
        }
    }
}

#[derive(Deserialize)]
struct SessionListResponse {
    sessions: Vec<SessionListRow>,
    #[serde(default)]
    total: usize,
}

struct SessionListPage {
    sessions: Vec<SessionSummary>,
    total: usize,
}

#[derive(Deserialize)]
struct SessionListRow {
    id: String,
    #[serde(default)]
    connection_id: Option<String>,
    #[serde(default, rename = "_lineage_root_id")]
    lineage_root_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    preview: Option<String>,
    #[serde(default)]
    message_count: usize,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    started_at: f64,
    #[serde(default)]
    last_active: Option<f64>,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    unread: bool,
    #[serde(default)]
    archived: bool,
}

impl SessionListRow {
    fn into_summary(self) -> SessionSummary {
        SessionSummary {
            id: self.id,
            connection_id: self.connection_id,
            lineage_root_id: self.lineage_root_id,
            cwd: self.cwd,
            profile: self.profile,
            title: self.title.unwrap_or_default(),
            preview: self.preview.unwrap_or_default(),
            message_count: self.message_count,
            source: self.source.unwrap_or_default(),
            started_at: self.started_at,
            last_active: self.last_active,
            pinned: self.pinned,
            unread: self.unread,
            archived: self.archived,
        }
    }
}

fn request_session_list(api_origin: &str, ticket: &str) -> Result<Vec<SessionSummary>, String> {
    Ok(request_session_list_page(api_origin, ticket, "exclude", 0)?.sessions)
}

fn request_session_list_page(
    api_origin: &str,
    ticket: &str,
    archived: &str,
    offset: usize,
) -> Result<SessionListPage, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err("Hermes session list requires the verified loopback API origin".into());
    }
    if !matches!(archived, "exclude" | "only") {
        return Err("Hermes session list received an unsupported archive filter".into());
    }
    let url = format!(
        "{}/api/sessions?limit=200&offset={offset}&min_messages=0&archived={archived}&order=recent",
        api_origin.trim_end_matches('/')
    );
    let response: SessionListResponse = request_gateway_json(api_origin, ticket, &url)?;
    Ok(SessionListPage {
        total: response.total,
        sessions: response
            .sessions
            .into_iter()
            .map(|session| SessionSummary {
                id: session.id,
                connection_id: session.connection_id,
                lineage_root_id: session.lineage_root_id,
                cwd: session.cwd,
                profile: session.profile,
                title: session.title.unwrap_or_default(),
                preview: session.preview.unwrap_or_default(),
                message_count: session.message_count,
                source: session.source.unwrap_or_default(),
                started_at: session.started_at,
                last_active: session.last_active,
                pinned: session.pinned,
                unread: session.unread,
                archived: session.archived,
            })
            .collect(),
    })
}

fn request_all_profile_archived_session_page(
    api_origin: &str,
    ticket: &str,
    offset: usize,
) -> Result<SessionListPage, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err("Hermes archived sessions require the verified loopback API origin".into());
    }
    let url = format!(
        "{}/api/profiles/sessions?limit=500&offset={offset}&min_messages=0&archived=only&order=recent&profile=all",
        api_origin.trim_end_matches('/')
    );
    let response: SessionListResponse = request_gateway_json(api_origin, ticket, &url)?;
    Ok(SessionListPage {
        total: response.total,
        sessions: response
            .sessions
            .into_iter()
            .map(|session| SessionSummary {
                id: session.id,
                connection_id: session.connection_id,
                lineage_root_id: session.lineage_root_id,
                cwd: session.cwd,
                profile: session.profile,
                title: session.title.unwrap_or_default(),
                preview: session.preview.unwrap_or_default(),
                message_count: session.message_count,
                source: session.source.unwrap_or_default(),
                started_at: session.started_at,
                last_active: session.last_active,
                pinned: session.pinned,
                unread: session.unread,
                archived: session.archived,
            })
            .collect(),
    })
}

const TRANSCRIPT_PAGE_SIZE: usize = 120;

fn load_older_transcript_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
    offset: usize,
) -> Result<OlderTranscriptPage, String> {
    if endpoint.remote_auth.is_some() {
        if session_id.is_empty()
            || session_id.len() > 256
            || session_id.chars().any(char::is_control)
            || offset > 10_000_000
            || profile.is_some_and(|value| {
                value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
            })
        {
            return Err("Hermes transcript request scope is invalid".into());
        }
        let mut path = format!(
            "{}/api/sessions/{}/messages?",
            endpoint.api_origin.trim_end_matches('/'),
            encode_query_component(session_id)
        );
        if let Some(profile) = profile {
            path.push_str(&format!("profile={}&", encode_query_component(profile)));
        }
        path.push_str(&format!(
            "limit={TRANSCRIPT_PAGE_SIZE}&offset={offset}&order=latest&include_compacted=true"
        ));
        let page: SessionMessagesPageResponse = request_remote_gateway_json(endpoint, &path)?;
        return decode_older_transcript_page(page, session_id, profile, offset);
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes transcript reads require the verified loopback API origin".into());
    }
    if session_id.is_empty() || session_id.len() > 256 || session_id.chars().any(char::is_control) {
        return Err("Hermes transcript session id is invalid".into());
    }
    if profile.is_some_and(|value| {
        value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
    }) {
        return Err("Hermes transcript profile scope is invalid".into());
    }
    if offset > 10_000_000 {
        return Err("Hermes transcript page offset exceeds the safe limit".into());
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    request_older_transcript_page(
        &endpoint.api_origin,
        ticket.as_str(),
        session_id,
        profile,
        offset,
    )
}

fn load_active_transcript_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
) -> Result<OlderTranscriptPage, String> {
    load_older_transcript_http(endpoint, session_id, profile, 0)
}

fn load_all_session_messages_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
) -> Result<SessionExportTranscript, String> {
    const EXPORT_PAGE_SIZE: usize = 500;
    const MAX_JSON_CHARS: usize = 32_000_000;
    const MAX_EXPORT_PAGES: usize = 200;

    if session_id.is_empty() || session_id.len() > 256 || session_id.chars().any(char::is_control) {
        return Err("Hermes session export ID is invalid".into());
    }
    if profile.is_some_and(|value| {
        value.is_empty()
            || value.len() > 128
            || value.chars().any(char::is_control)
            || value.eq_ignore_ascii_case("all")
    }) {
        return Err("Hermes session export profile scope is invalid".into());
    }
    let profile_id = profile.unwrap_or(&endpoint.profile_id);
    if profile_id.eq_ignore_ascii_case("all") {
        return Err("Hermes session export requires a concrete profile".into());
    }

    let origin = endpoint.api_origin.trim_end_matches('/');
    let mut messages = Vec::new();
    let mut json_chars = 0usize;
    let mut offset = 0usize;

    for _ in 0..MAX_EXPORT_PAGES {
        let mut url = format!(
            "{origin}/api/sessions/{}/messages?",
            encode_query_component(session_id)
        );
        if !profile_id.is_empty() && profile_id != "default" {
            url.push_str(&format!("profile={}&", encode_query_component(profile_id)));
        }
        url.push_str(&format!(
            "limit={EXPORT_PAGE_SIZE}&offset={offset}&order=oldest&include_compacted=true"
        ));
        let page: SessionMessagesPageResponse =
            profile_scoped_gateway_http_request(endpoint, "GET", &url, None)?;
        if page.session_id.is_empty()
            || page.session_id.len() > 256
            || page.session_id.chars().any(char::is_control)
        {
            return Err("Hermes session export returned an invalid session ID".into());
        }
        let resolved_session_id = page.session_id;
        let page_chars = serde_json::to_string(&page.messages)
            .map(|serialized| serialized.len())
            .unwrap_or(0);
        json_chars = json_chars.saturating_add(page_chars);
        if json_chars > MAX_JSON_CHARS {
            return Err(
                "Session transcript exceeds the Desktop safe-load limit; use the Web Dashboard export for this session."
                    .into(),
            );
        }
        let page_len = page.messages.len();
        let page_limit = page.pagination.as_ref().map(|pagination| pagination.limit);
        messages.extend(page.messages);
        if page_limit.is_none()
            || page_len == 0
            || page_len < page_limit.unwrap_or(EXPORT_PAGE_SIZE)
        {
            return Ok(SessionExportTranscript {
                resolved_session_id,
                messages,
            });
        }
        offset = offset.saturating_add(page_len);
    }

    Err(
        "Session transcript exceeds the Desktop safe-load limit; use the Web Dashboard export for this session."
            .into(),
    )
}

fn request_older_transcript_page(
    api_origin: &str,
    ticket: &str,
    session_id: &str,
    profile: Option<&str>,
    offset: usize,
) -> Result<OlderTranscriptPage, String> {
    const PAGE_SIZE: usize = TRANSCRIPT_PAGE_SIZE;
    let mut url = format!(
        "{}/api/sessions/{}/messages?",
        api_origin.trim_end_matches('/'),
        encode_query_component(session_id)
    );
    if let Some(profile) = profile {
        url.push_str(&format!("profile={}&", encode_query_component(profile)));
    }
    url.push_str(&format!(
        "limit={PAGE_SIZE}&offset={offset}&order=latest&include_compacted=true"
    ));
    let page: SessionMessagesPageResponse = request_gateway_json(api_origin, ticket, &url)?;
    decode_older_transcript_page(page, session_id, profile, offset)
}

fn decode_older_transcript_page(
    page: SessionMessagesPageResponse,
    session_id: &str,
    profile: Option<&str>,
    offset: usize,
) -> Result<OlderTranscriptPage, String> {
    const PAGE_SIZE: usize = TRANSCRIPT_PAGE_SIZE;
    if page.session_id != session_id {
        return Err("Hermes transcript response belongs to a different session".into());
    }
    if page
        .profile
        .as_deref()
        .is_some_and(|returned| Some(returned) != profile)
    {
        return Err("Hermes transcript response belongs to a different profile".into());
    }
    let Some(pagination) = page.pagination else {
        // Legacy servers return the complete transcript and omit pagination.
        // The shared 512 KiB HTTP cap bounds payload size; this row cap also
        // prevents a tiny-row response from creating unbounded UI work.
        if page.messages.len() > 10_000 {
            return Err("Hermes returned too many legacy transcript messages".into());
        }
        return Ok(OlderTranscriptPage {
            messages: page.messages,
            has_more: false,
            replace_all: true,
        });
    };
    if page.messages.len() > PAGE_SIZE {
        return Err("Hermes returned too many transcript messages".into());
    }
    if pagination.limit != PAGE_SIZE
        || pagination.offset != offset
        || pagination.order.as_deref() != Some("latest")
        || pagination.returned != page.messages.len()
    {
        return Err("Hermes did not confirm the requested latest transcript page".into());
    }
    let has_more = page.messages.len() == PAGE_SIZE;
    Ok(OlderTranscriptPage {
        messages: page.messages,
        has_more,
        replace_all: false,
    })
}

fn search_sessions_http(
    endpoint: &GatewayEndpoint,
    query: &str,
) -> Result<Vec<SessionSearchResult>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    if query.len() > 512 {
        return Err("Session search query exceeds 512 bytes".into());
    }
    if endpoint.remote_auth.is_some() {
        let path = format!(
            "{}/api/sessions/search?q={}&limit=20",
            endpoint.api_origin.trim_end_matches('/'),
            encode_query_component(query)
        );
        let response: SessionSearchResponse = request_remote_gateway_json(endpoint, &path)?;
        if response.results.len() > 100 {
            return Err("Hermes returned too many session search results".into());
        }
        return Ok(response.results);
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes session search requires the verified loopback API origin".into());
    }

    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    request_session_search(&endpoint.api_origin, ticket.as_str(), query)
}

fn request_session_search(
    api_origin: &str,
    ticket: &str,
    query: &str,
) -> Result<Vec<SessionSearchResult>, String> {
    let url = format!(
        "{}/api/sessions/search?q={}&limit=20",
        api_origin.trim_end_matches('/'),
        encode_query_component(query)
    );
    let response: SessionSearchResponse = request_gateway_json(api_origin, ticket, &url)?;
    if response.results.len() > 100 {
        return Err("Hermes returned too many session search results".into());
    }
    Ok(response.results)
}

fn load_messaging_platforms_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<MessagingPlatformsSnapshot, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    let url = format!(
        "{}/api/messaging/platforms{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let snapshot: MessagingPlatformsSnapshot = if endpoint.remote_auth.is_some() {
        request_remote_gateway_json(endpoint, &url)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes messaging reads require the verified loopback API origin".into());
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        request_gateway_json(&endpoint.api_origin, ticket.as_str(), &url)?
    };
    if snapshot.platforms.len() > 256 {
        return Err("Hermes returned too many messaging platforms".into());
    }
    let mut ids = HashSet::new();
    for platform in &snapshot.platforms {
        if platform.id.trim().is_empty()
            || platform.id.len() > 256
            || platform.name.len() > 512
            || platform.description.len() > 16_384
            || !ids.insert(platform.id.as_str())
        {
            return Err("Hermes returned invalid or duplicate messaging platform rows".into());
        }
    }
    Ok(snapshot)
}

const CRON_FORM_NAME_LIMIT: usize = 1024;
const CRON_FORM_PROMPT_LIMIT: usize = 1_048_576;
const CRON_FORM_SCHEDULE_LIMIT: usize = 512;
const CRON_FORM_DELIVER_LIMIT: usize = 512;
const CRON_FORM_MODEL_LIMIT: usize = 512;
const CRON_FORM_PROVIDER_LIMIT: usize = 512;

fn cron_profile_scope<'a>(
    endpoint: &'a GatewayEndpoint,
    profile: Option<&'a str>,
) -> Result<&'a str, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes scheduled-jobs profile scope is invalid".into());
    }
    Ok(profile)
}

fn cron_job_id_is_valid(job_id: &str) -> bool {
    !job_id.trim().is_empty() && job_id.len() <= 256 && !job_id.chars().any(char::is_control)
}

fn cron_form_text_is_valid(value: &str, max_len: usize) -> bool {
    value.len() <= max_len && !value.contains('\0')
}

fn validate_cron_create_payload(payload: &CronJobCreatePayload) -> Result<(), String> {
    if payload.schedule.trim().is_empty()
        || !cron_form_text_is_valid(&payload.schedule, CRON_FORM_SCHEDULE_LIMIT)
        || !cron_form_text_is_valid(&payload.prompt, CRON_FORM_PROMPT_LIMIT)
        || payload
            .name
            .as_ref()
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_NAME_LIMIT))
        || payload
            .deliver
            .as_ref()
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_DELIVER_LIMIT))
        || payload
            .model
            .as_ref()
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_MODEL_LIMIT))
        || payload
            .provider
            .as_ref()
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_PROVIDER_LIMIT))
    {
        return Err("Hermes scheduled-job create fields are invalid or exceed their bounds".into());
    }
    Ok(())
}

fn validate_cron_update_payload(updates: &CronJobUpdatePayload) -> Result<(), String> {
    if updates
        .name
        .as_ref()
        .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_NAME_LIMIT))
        || updates
            .prompt
            .as_ref()
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_PROMPT_LIMIT))
        || updates.schedule.as_ref().is_some_and(|value| {
            value.trim().is_empty() || !cron_form_text_is_valid(value, CRON_FORM_SCHEDULE_LIMIT)
        })
        || updates
            .deliver
            .as_ref()
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_DELIVER_LIMIT))
        || updates
            .model
            .as_ref()
            .and_then(Option::as_ref)
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_MODEL_LIMIT))
        || updates
            .provider
            .as_ref()
            .and_then(Option::as_ref)
            .is_some_and(|value| !cron_form_text_is_valid(value, CRON_FORM_PROVIDER_LIMIT))
        || (updates.name.is_none()
            && updates.prompt.is_none()
            && updates.schedule.is_none()
            && updates.deliver.is_none()
            && updates.catch_up.is_none()
            && updates.model.is_none()
            && updates.provider.is_none())
    {
        return Err("Hermes scheduled-job update fields are invalid or empty".into());
    }
    Ok(())
}

fn cron_mutation_url(endpoint: &GatewayEndpoint, profile: &str, suffix: &str) -> String {
    format!(
        "{}{}{}",
        endpoint.api_origin.trim_end_matches('/'),
        suffix,
        session_profile_suffix(profile)
    )
}

fn cron_gateway_http_request<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    method: &str,
    url: &str,
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    if !matches!(method, "GET" | "POST" | "PUT" | "DELETE")
        || !api_url_is_within_origin(&endpoint.api_origin, url)
    {
        return Err("Hermes scheduled-job API target is invalid".into());
    }
    if endpoint.remote_auth.is_some() {
        return if method == "GET" {
            request_remote_gateway_json(endpoint, url)
        } else {
            request_remote_gateway_mutation(
                endpoint,
                method,
                url,
                body.unwrap_or(serde_json::Value::Null),
            )
        };
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err(
            "Hermes scheduled-job requests require the verified loopback API origin".into(),
        );
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    cron_local_gateway_http_request_with_ticket(
        &endpoint.api_origin,
        ticket.as_str(),
        method,
        url,
        body,
    )
}

fn cron_local_gateway_http_request_with_ticket<T: DeserializeOwned>(
    api_origin: &str,
    ticket: &str,
    method: &str,
    url: &str,
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    if !matches!(method, "GET" | "POST" | "PUT" | "DELETE")
        || !is_bare_loopback_origin(api_origin)
        || !api_url_is_within_origin(api_origin, url)
    {
        return Err("Hermes scheduled-job API target is invalid".into());
    }
    validate_http_ticket(ticket)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(8)))
        .max_redirects(0)
        .build()
        .into();
    let response = match method {
        "GET" => agent
            .get(url)
            .header("X-Hermes-Gateway-Ticket", ticket)
            .header("Accept", "application/json")
            .call(),
        "POST" => agent
            .post(url)
            .header("X-Hermes-Gateway-Ticket", ticket)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body.unwrap_or(serde_json::Value::Null).to_string()),
        "PUT" => agent
            .put(url)
            .header("X-Hermes-Gateway-Ticket", ticket)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body.unwrap_or(serde_json::Value::Null).to_string()),
        "DELETE" => agent
            .delete(url)
            .header("X-Hermes-Gateway-Ticket", ticket)
            .header("Accept", "application/json")
            .call(),
        _ => unreachable!("method was validated"),
    }
    .map_err(|_| "Hermes scheduled-job API request failed".to_owned())?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes scheduled-job API response".to_owned())?;
    if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
        return Err("Hermes scheduled-job API response exceeded 512 KiB".into());
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| "Hermes returned invalid scheduled-job API JSON".to_owned())
}

fn validate_cron_job_receipt(
    job: CronJobInfo,
    expected_id: Option<&str>,
) -> Result<CronJobInfo, String> {
    if !cron_job_id_is_valid(&job.id)
        || expected_id.is_some_and(|expected| job.id != expected)
        || job
            .name
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_NAME_LIMIT)
        || job
            .prompt
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_PROMPT_LIMIT)
        || job
            .script
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_PROMPT_LIMIT)
        || job
            .state
            .as_ref()
            .is_some_and(|value| value.len() > 128 || value.chars().any(char::is_control))
        || job
            .schedule_display
            .as_ref()
            .is_some_and(|value| value.len() > 2048)
        || job
            .deliver
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_DELIVER_LIMIT)
        || job
            .model
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_MODEL_LIMIT)
        || job
            .provider
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_PROVIDER_LIMIT)
        || job
            .next_run_at
            .as_ref()
            .is_some_and(|value| value.len() > 128)
        || job
            .last_run_at
            .as_ref()
            .is_some_and(|value| value.len() > 128)
        || job
            .last_error
            .as_ref()
            .is_some_and(|value| value.len() > 16_384)
        || job
            .latest_execution
            .as_ref()
            .is_some_and(|row| !cron_execution_is_valid(row))
        || job
            .live_execution
            .as_ref()
            .is_some_and(|row| !cron_execution_is_valid(row))
        || job.schedule.as_ref().is_some_and(|schedule| {
            schedule
                .display
                .as_ref()
                .is_some_and(|value| value.len() > 2048)
                || schedule
                    .expr
                    .as_ref()
                    .is_some_and(|value| value.len() > CRON_FORM_SCHEDULE_LIMIT)
                || schedule
                    .kind
                    .as_ref()
                    .is_some_and(|value| value.len() > 128)
        })
    {
        return Err("Hermes returned an invalid or mismatched Scheduled Job receipt".into());
    }
    Ok(job)
}

fn load_cron_delivery_targets_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<Vec<CronDeliveryTarget>, String> {
    let profile = cron_profile_scope(endpoint, profile)?;
    let url = cron_mutation_url(endpoint, profile, "/api/cron/delivery-targets");
    let snapshot: CronDeliveryTargetsSnapshot =
        cron_gateway_http_request(endpoint, "GET", &url, None)?;
    if snapshot.targets.len() > 256 {
        return Err("Hermes returned too many Scheduled Job delivery targets".into());
    }
    let mut identities = HashSet::new();
    for target in &snapshot.targets {
        if target.id.trim().is_empty()
            || target.id.len() > 256
            || target.id.chars().any(char::is_control)
            || target.name.len() > 512
            || target.name.chars().any(char::is_control)
            || target
                .home_env_var
                .as_ref()
                .is_some_and(|value| value.len() > 256 || value.chars().any(char::is_control))
            || !identities.insert(target.id.as_str())
        {
            return Err(
                "Hermes returned invalid or duplicate Scheduled Job delivery targets".into(),
            );
        }
    }
    Ok(snapshot.targets)
}

fn load_model_options_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<ModelOptionsSnapshot, String> {
    let profile = cron_profile_scope(endpoint, profile)?;
    let url = format!(
        "{}/api/model/options?explicit_only=1&profile={}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(profile)
    );
    let snapshot: ModelOptionsSnapshot = cron_gateway_http_request(endpoint, "GET", &url, None)?;
    if snapshot.providers.len() > 512
        || snapshot
            .model
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_MODEL_LIMIT)
        || snapshot
            .provider
            .as_ref()
            .is_some_and(|value| value.len() > CRON_FORM_PROVIDER_LIMIT)
    {
        return Err("Hermes returned invalid or oversized model options".into());
    }
    let mut providers = HashSet::new();
    for provider in &snapshot.providers {
        if provider.slug.trim().is_empty()
            || provider.slug.len() > 256
            || provider.slug.chars().any(char::is_control)
            || provider.name.trim().is_empty()
            || provider.name.len() > 512
            || provider.name.chars().any(char::is_control)
            || provider.models.len() > 10_000
            || !providers.insert(provider.slug.as_str())
        {
            return Err("Hermes returned invalid or duplicate model providers".into());
        }
        let mut models = HashSet::new();
        for model in &provider.models {
            if model.trim().is_empty()
                || model.len() > CRON_FORM_MODEL_LIMIT
                || model.chars().any(char::is_control)
                || !models.insert(model.as_str())
            {
                return Err("Hermes returned invalid or duplicate provider models".into());
            }
        }
    }
    Ok(snapshot)
}

fn load_cron_blueprints_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<Vec<CronBlueprint>, String> {
    let profile = cron_profile_scope(endpoint, profile)?;
    let url = cron_mutation_url(endpoint, profile, "/api/cron/blueprints");
    let snapshot: CronBlueprintsSnapshot = cron_gateway_http_request(endpoint, "GET", &url, None)?;
    if snapshot.blueprints.len() > 1024 {
        return Err("Hermes returned too many Scheduled Job blueprints".into());
    }
    let mut blueprint_keys = HashSet::new();
    for blueprint in &snapshot.blueprints {
        if blueprint.key.trim().is_empty()
            || blueprint.key.len() > 256
            || blueprint.key.chars().any(char::is_control)
            || blueprint.title.len() > 1024
            || blueprint.description.len() > 16_384
            || blueprint.category.len() > 256
            || blueprint.command.len() > 4096
            || blueprint.app_url.len() > 4096
            || blueprint
                .source
                .as_ref()
                .is_some_and(|value| value.len() > 128)
            || blueprint
                .plugin
                .as_ref()
                .is_some_and(|value| value.len() > 256 || value.chars().any(char::is_control))
            || blueprint.tags.len() > 64
            || blueprint
                .tags
                .iter()
                .any(|value| value.len() > 256 || value.chars().any(char::is_control))
            || blueprint.fields.len() > 128
            || !blueprint_keys.insert(blueprint.key.as_str())
        {
            return Err("Hermes returned invalid or duplicate Scheduled Job blueprints".into());
        }
        let mut field_names = HashSet::new();
        for field in &blueprint.fields {
            if field.name.trim().is_empty()
                || field.name.len() > 128
                || field.name.chars().any(char::is_control)
                || !matches!(
                    field.field_type.as_str(),
                    "enum" | "text" | "time" | "weekdays"
                )
                || field.label.len() > 512
                || field
                    .default
                    .as_ref()
                    .is_some_and(|value| value.len() > 16_384)
                || field.options.len() > 256
                || field
                    .options
                    .iter()
                    .any(|value| value.len() > 1024 || value.chars().any(char::is_control))
                || field.help.len() > 4096
                || !field_names.insert(field.name.as_str())
            {
                return Err("Hermes returned an invalid Scheduled Job blueprint field".into());
            }
        }
    }
    Ok(snapshot.blueprints)
}

#[derive(Deserialize)]
struct CronBlueprintsSnapshot {
    #[serde(default)]
    blueprints: Vec<CronBlueprint>,
}

fn instantiate_cron_blueprint_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    payload: &CronBlueprintInstantiatePayload,
) -> Result<CronJobInfo, String> {
    if payload.blueprint.trim().is_empty()
        || payload.blueprint.len() > 256
        || payload.blueprint.chars().any(char::is_control)
        || payload.values.len() > 128
        || payload.values.iter().any(|(key, value)| {
            key.trim().is_empty()
                || key.len() > 128
                || key.chars().any(char::is_control)
                || value.len() > 16_384
                || value.contains('\0')
        })
    {
        return Err(
            "Hermes Scheduled Job blueprint values are invalid or exceed their bounds".into(),
        );
    }
    let profile = cron_profile_scope(endpoint, profile)?;
    let body = serde_json::to_value(payload)
        .map_err(|_| "Could not encode Hermes Scheduled Job blueprint request".to_owned())?;
    if body.to_string().len() > 256 * 1024 {
        return Err("Hermes Scheduled Job blueprint request exceeded 256 KiB".into());
    }
    let url = cron_mutation_url(endpoint, profile, "/api/cron/blueprints/instantiate");
    let job: CronJobInfo = cron_gateway_http_request(endpoint, "POST", &url, Some(body))?;
    validate_cron_job_receipt(job, None)
}

fn create_cron_job_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    payload: &CronJobCreatePayload,
) -> Result<CronJobInfo, String> {
    let profile = cron_profile_scope(endpoint, profile)?;
    validate_cron_create_payload(payload)?;
    let body = serde_json::to_value(payload)
        .map_err(|_| "Could not encode Hermes Scheduled Job create request".to_owned())?;
    if body.to_string().len() > 256 * 1024 {
        return Err("Hermes Scheduled Job create request exceeded 256 KiB".into());
    }
    let url = cron_mutation_url(endpoint, profile, "/api/cron/jobs");
    let job: CronJobInfo = cron_gateway_http_request(endpoint, "POST", &url, Some(body))?;
    validate_cron_job_receipt(job, None)
}

fn update_cron_job_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    job_id: &str,
    updates: &CronJobUpdatePayload,
) -> Result<CronJobInfo, String> {
    if !cron_job_id_is_valid(job_id) {
        return Err("Hermes scheduled-job ID is invalid".into());
    }
    let profile = cron_profile_scope(endpoint, profile)?;
    validate_cron_update_payload(updates)?;
    let body = serde_json::json!({ "updates": updates });
    if body.to_string().len() > 256 * 1024 {
        return Err("Hermes Scheduled Job update request exceeded 256 KiB".into());
    }
    let suffix = format!("/api/cron/jobs/{}", encode_query_component(job_id));
    let url = cron_mutation_url(endpoint, profile, &suffix);
    let job: CronJobInfo = cron_gateway_http_request(endpoint, "PUT", &url, Some(body))?;
    validate_cron_job_receipt(job, Some(job_id))
}

fn delete_cron_job_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    job_id: &str,
) -> Result<CronJobDeleteResult, String> {
    if !cron_job_id_is_valid(job_id) {
        return Err("Hermes scheduled-job ID is invalid".into());
    }
    let profile = cron_profile_scope(endpoint, profile)?;
    let suffix = format!("/api/cron/jobs/{}", encode_query_component(job_id));
    let url = cron_mutation_url(endpoint, profile, &suffix);
    let result: CronJobDeleteResult = cron_gateway_http_request(endpoint, "DELETE", &url, None)?;
    if !result.ok {
        return Err("Hermes did not confirm Scheduled Job deletion".into());
    }
    Ok(result)
}

fn load_cron_jobs_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<Vec<CronJobInfo>, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes scheduled-jobs profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/cron/jobs{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let jobs: Vec<CronJobInfo> = if endpoint.remote_auth.is_some() {
        request_remote_gateway_json(endpoint, &url)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err(
                "Hermes scheduled-job reads require the verified loopback API origin".into(),
            );
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        request_gateway_json(&endpoint.api_origin, ticket.as_str(), &url)?
    };
    if jobs.len() > 10_000 {
        return Err("Hermes returned too many scheduled jobs".into());
    }
    let mut identities = HashSet::new();
    for job in &jobs {
        if job.id.is_empty()
            || job.id.len() > 256
            || job.id.chars().any(char::is_control)
            || job.name.as_ref().is_some_and(|value| value.len() > 1024)
            || job
                .prompt
                .as_ref()
                .is_some_and(|value| value.len() > 1_048_576)
            || job.state.as_ref().is_some_and(|value| value.len() > 128)
            || job
                .schedule_display
                .as_ref()
                .is_some_and(|value| value.len() > 2048)
            || job.deliver.as_ref().is_some_and(|value| value.len() > 512)
            || job.model.as_ref().is_some_and(|value| value.len() > 512)
            || job.provider.as_ref().is_some_and(|value| value.len() > 512)
            || job
                .next_run_at
                .as_ref()
                .is_some_and(|value| value.len() > 128)
            || job
                .last_run_at
                .as_ref()
                .is_some_and(|value| value.len() > 128)
            || job
                .last_error
                .as_ref()
                .is_some_and(|value| value.len() > 16_384)
            || job
                .latest_execution
                .as_ref()
                .is_some_and(|execution| !cron_execution_is_valid(execution))
            || job
                .live_execution
                .as_ref()
                .is_some_and(|execution| !cron_execution_is_valid(execution))
            || job.schedule.as_ref().is_some_and(|schedule| {
                schedule
                    .display
                    .as_ref()
                    .is_some_and(|value| value.len() > 2048)
                    || schedule
                        .expr
                        .as_ref()
                        .is_some_and(|value| value.len() > 512)
                    || schedule
                        .kind
                        .as_ref()
                        .is_some_and(|value| value.len() > 128)
            })
            || !identities.insert(job.id.as_str())
        {
            return Err("Hermes returned invalid or duplicate scheduled-job rows".into());
        }
    }
    Ok(jobs)
}

fn cron_execution_is_valid(execution: &CronJobExecutionInfo) -> bool {
    !execution.id.is_empty()
        && execution.id.len() <= 256
        && !execution.id.chars().any(char::is_control)
        && !execution.status.is_empty()
        && execution.status.len() <= 128
        && !execution.status.chars().any(char::is_control)
        && execution
            .claimed_at
            .as_ref()
            .is_none_or(|value| value.len() <= 128 && !value.chars().any(char::is_control))
        && execution
            .error
            .as_ref()
            .is_none_or(|value| value.len() <= 16_384)
}

fn trigger_cron_job_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    job_id: &str,
) -> Result<CronJobInfo, String> {
    if job_id.trim().is_empty() || job_id.len() > 256 || job_id.chars().any(char::is_control) {
        return Err("Hermes scheduled-job ID is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes scheduled-jobs profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/cron/jobs/{}/trigger{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(job_id),
        session_profile_suffix(profile)
    );
    let job: CronJobInfo =
        request_empty_gateway_post_with_timeout(endpoint, &url, Duration::from_secs(24 * 60 * 60))?;
    if job.id != job_id
        || job
            .latest_execution
            .as_ref()
            .is_some_and(|row| !cron_execution_is_valid(row))
        || job
            .live_execution
            .as_ref()
            .is_some_and(|row| !cron_execution_is_valid(row))
    {
        return Err("Hermes returned an invalid Scheduled Job trigger receipt".into());
    }
    Ok(job)
}

fn cancel_cron_job_run_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    job_id: &str,
) -> Result<CronCancelResult, String> {
    if job_id.trim().is_empty() || job_id.len() > 256 || job_id.chars().any(char::is_control) {
        return Err("Hermes scheduled-job ID is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes scheduled-jobs profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/cron/jobs/{}/cancel{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(job_id),
        session_profile_suffix(profile)
    );
    let result: CronCancelResult = request_empty_gateway_post(endpoint, &url)?;
    if result.job_id != job_id
        || result.execution_id.as_ref().is_some_and(|value| {
            value.is_empty() || value.len() > 256 || value.chars().any(char::is_control)
        })
        || result
            .detail
            .as_ref()
            .is_some_and(|value| value.len() > 16_384)
    {
        return Err("Hermes returned a mismatched Scheduled Job cancel receipt".into());
    }
    Ok(result)
}

fn load_cron_job_runs_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    job_id: &str,
) -> Result<Vec<CronJobRunInfo>, String> {
    if job_id.trim().is_empty() || job_id.len() > 256 || job_id.chars().any(char::is_control) {
        return Err("Hermes scheduled-job ID is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes scheduled-jobs profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/cron/jobs/{}/runs?limit=20&profile={}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(job_id),
        encode_query_component(profile)
    );
    let response: CronJobRunsReply = if endpoint.remote_auth.is_some() {
        request_remote_gateway_json(endpoint, &url)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err(
                "Hermes scheduled-job reads require the verified loopback API origin".into(),
            );
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        request_gateway_json(&endpoint.api_origin, ticket.as_str(), &url)?
    };
    if response.runs.len() > 20 {
        return Err("Hermes returned too many scheduled-job runs".into());
    }
    let mut identities = HashSet::new();
    for run in &response.runs {
        if run.id.is_empty()
            || run.id.len() > 512
            || run.id.chars().any(char::is_control)
            || run.title.as_ref().is_some_and(|value| value.len() > 4096)
            || run
                .preview
                .as_ref()
                .is_some_and(|value| value.len() > 16_384)
            || run.source.as_ref().is_some_and(|value| value.len() > 128)
            || run
                .started_at
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || run
                .last_active
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || !identities.insert(run.id.as_str())
        {
            return Err("Hermes returned invalid or duplicate scheduled-job run rows".into());
        }
    }
    Ok(response.runs)
}

#[derive(Deserialize)]
struct CronJobRunsReply {
    #[serde(default)]
    runs: Vec<CronJobRunInfo>,
}

fn set_cron_job_paused_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    job_id: &str,
    paused: bool,
) -> Result<CronJobInfo, String> {
    if job_id.trim().is_empty() || job_id.len() > 256 || job_id.chars().any(char::is_control) {
        return Err("Hermes scheduled-job ID is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes scheduled-jobs profile scope is invalid".into());
    }
    let action = if paused { "pause" } else { "resume" };
    let url = format!(
        "{}/api/cron/jobs/{}/{}{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(job_id),
        action,
        session_profile_suffix(profile)
    );
    let job: CronJobInfo = request_empty_gateway_post(endpoint, &url)?;
    let returned_paused = !job.enabled || job.state.as_deref() == Some("paused");
    if job.id != job_id || returned_paused != paused {
        return Err("Hermes returned a mismatched scheduled-job action receipt".into());
    }
    Ok(job)
}

fn request_empty_gateway_post<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    url: &str,
) -> Result<T, String> {
    request_empty_gateway_post_with_timeout(endpoint, url, Duration::from_secs(8))
}

fn request_empty_gateway_post_with_timeout<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    url: &str,
    timeout: Duration,
) -> Result<T, String> {
    if !api_url_is_within_origin(&endpoint.api_origin, url) {
        return Err("Hermes API target is invalid".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .build()
        .into();
    let response = if endpoint.remote_auth.is_some() {
        match remote_http_auth(endpoint)? {
            RemoteHttpAuth::SessionToken(token) => agent
                .post(url)
                .header("X-Hermes-Session-Token", token.as_str())
                .header("Accept", "application/json")
                .send_empty(),
            RemoteHttpAuth::Bearer(token) => agent
                .post(url)
                .header("Authorization", &format!("Bearer {}", token.as_str()))
                .header("Accept", "application/json")
                .send_empty(),
        }
        .map_err(|_| "Hermes scheduled-job action request failed".to_owned())?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err(
                "Hermes scheduled-job actions require the verified loopback API origin".into(),
            );
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        let local_agent: ureq::Agent = ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(timeout))
            .max_redirects(0)
            .build()
            .into();
        local_agent
            .post(url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .send_empty()
            .map_err(|_| "Hermes scheduled-job action request failed".to_owned())?
    };
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes scheduled-job action response".to_owned())?;
    if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
        return Err("Hermes scheduled-job action response exceeded 512 KiB".into());
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| "Hermes returned invalid scheduled-job action JSON".to_owned())
}

fn load_pairing_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<PairingSnapshot, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes pairing profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/pairing{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let snapshot: PairingSnapshot = if endpoint.remote_auth.is_some() {
        request_remote_gateway_json(endpoint, &url)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes pairing reads require the verified loopback API origin".into());
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        request_gateway_json(&endpoint.api_origin, ticket.as_str(), &url)?
    };
    if snapshot.pending.len() > 4096 || snapshot.approved.len() > 4096 {
        return Err("Hermes returned too many pairing rows".into());
    }
    let mut identities = HashSet::new();
    for user in snapshot.pending.iter().chain(snapshot.approved.iter()) {
        if user.platform.trim().is_empty()
            || user.platform.len() > 128
            || user.platform.chars().any(char::is_control)
            || user.user_id.trim().is_empty()
            || user.user_id.len() > 512
            || user.user_id.chars().any(char::is_control)
            || user
                .user_name
                .as_ref()
                .is_some_and(|name| name.len() > 1024)
            || user
                .age_minutes
                .is_some_and(|age| !age.is_finite() || age < 0.0)
            || user.request_id.as_ref().is_some_and(|id| {
                id.is_empty() || id.len() > 256 || id.chars().any(char::is_control)
            })
            || !identities.insert((user.platform.as_str(), user.user_id.as_str()))
        {
            return Err("Hermes returned invalid or duplicate pairing rows".into());
        }
    }
    Ok(snapshot)
}

fn validate_telegram_onboarding_start(
    result: TelegramOnboardingStart,
) -> Result<TelegramOnboardingStart, String> {
    let expires_at = time::OffsetDateTime::parse(
        &result.expires_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| "Hermes returned an invalid Telegram setup expiry".to_owned())?;
    let safe_link =
        result.deep_link.starts_with("https://t.me/") || result.deep_link.starts_with("tg://");
    if result.pairing_id.is_empty()
        || result.pairing_id.len() > 256
        || result.pairing_id.chars().any(char::is_control)
        || result.qr_payload.is_empty()
        || result.qr_payload.len() > 4096
        || result.qr_payload.chars().any(char::is_control)
        || result.suggested_username.len() > 128
        || result.suggested_username.chars().any(char::is_control)
        || !safe_link
        || result.deep_link.len() > 2048
        || expires_at <= time::OffsetDateTime::now_utc()
    {
        return Err("Hermes returned invalid Telegram setup details".into());
    }
    Ok(result)
}

fn telegram_onboarding_url(
    endpoint: &GatewayEndpoint,
    pairing_id: &str,
    profile: &str,
    suffix: &str,
) -> Result<String, String> {
    if pairing_id.is_empty()
        || pairing_id.len() > 256
        || pairing_id.chars().any(char::is_control)
        || profile.is_empty()
        || profile.len() > 128
        || profile.chars().any(char::is_control)
    {
        return Err("Hermes Telegram setup scope is invalid".into());
    }
    Ok(format!(
        "{}/api/messaging/telegram/onboarding/{}{}{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(pairing_id),
        suffix,
        session_profile_suffix(profile)
    ))
}

fn profile_scoped_gateway_http_request<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    method: &str,
    url: &str,
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    if !matches!(method, "POST" | "GET" | "PUT" | "DELETE")
        || !api_url_is_within_origin(&endpoint.api_origin, url)
    {
        return Err("Hermes API target is invalid".into());
    }
    if endpoint.remote_auth.is_some() {
        let value = match method {
            "GET" => request_remote_gateway_json(endpoint, url)?,
            "DELETE" => request_remote_gateway_mutation(
                endpoint,
                method,
                url,
                body.unwrap_or(serde_json::Value::Null),
            )?,
            _ => request_remote_gateway_mutation(
                endpoint,
                method,
                url,
                body.unwrap_or(serde_json::Value::Null),
            )?,
        };
        return Ok(value);
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes Telegram setup requires the verified loopback API origin".into());
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(8)))
        .max_redirects(0)
        .build()
        .into();
    let response = match method {
        "POST" => agent
            .post(url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body.unwrap_or(serde_json::Value::Null).to_string()),
        "PUT" => agent
            .put(url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body.unwrap_or(serde_json::Value::Null).to_string()),
        "GET" => agent
            .get(url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .call(),
        "DELETE" => agent
            .delete(url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .call(),
        _ => unreachable!("method was validated"),
    }
    .map_err(|_| "Hermes API request failed".to_owned())?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes API response".to_owned())?;
    if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
        return Err("Hermes API response exceeded 512 KiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Hermes returned invalid API JSON".to_owned())
}

fn start_telegram_onboarding_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    bot_name: Option<&str>,
) -> Result<TelegramOnboardingStart, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes Telegram setup profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/messaging/telegram/onboarding/start{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let mut body = serde_json::Map::new();
    if let Some(name) = bot_name {
        if name.len() > 128 || name.chars().any(char::is_control) {
            return Err("Telegram bot name is invalid".into());
        }
        body.insert("bot_name".into(), serde_json::Value::String(name.into()));
    }
    validate_telegram_onboarding_start(profile_scoped_gateway_http_request(
        endpoint,
        "POST",
        &url,
        Some(serde_json::Value::Object(body)),
    )?)
}

fn load_telegram_onboarding_status_http(
    endpoint: &GatewayEndpoint,
    pairing_id: &str,
    profile: Option<&str>,
) -> Result<TelegramOnboardingStatus, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    let url = telegram_onboarding_url(endpoint, pairing_id, profile, "")?;
    let status: TelegramOnboardingStatus =
        profile_scoped_gateway_http_request(endpoint, "GET", &url, None)?;
    match &status {
        TelegramOnboardingStatus::Waiting { expires_at } => {
            validate_telegram_expiry(expires_at)?;
        }
        TelegramOnboardingStatus::Ready {
            bot_username,
            expires_at,
            owner_user_id,
        } => {
            validate_telegram_expiry(expires_at)?;
            if bot_username
                .as_ref()
                .is_some_and(|name| name.len() > 128 || name.chars().any(char::is_control))
                || owner_user_id
                    .as_ref()
                    .is_some_and(|id| !valid_telegram_user_id(id))
            {
                return Err("Hermes returned invalid Telegram setup status".into());
            }
        }
    }
    Ok(status)
}

fn validate_telegram_expiry(value: &str) -> Result<time::OffsetDateTime, String> {
    let expiry = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map_err(|_| "Hermes returned an invalid Telegram setup expiry".to_owned())?;
    if expiry <= time::OffsetDateTime::now_utc() {
        return Err("Telegram setup has expired".into());
    }
    Ok(expiry)
}

fn valid_telegram_user_id(value: &str) -> bool {
    (3..=20).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn apply_telegram_onboarding_http(
    endpoint: &GatewayEndpoint,
    pairing_id: &str,
    profile: Option<&str>,
    allowed_user_ids: &[String],
) -> Result<TelegramOnboardingApplyReceipt, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if allowed_user_ids.is_empty()
        || allowed_user_ids.len() > 256
        || allowed_user_ids
            .iter()
            .any(|id| !valid_telegram_user_id(id))
    {
        return Err("Telegram allowed user IDs must contain 3 to 20 digits".into());
    }
    let unique = allowed_user_ids.iter().collect::<HashSet<_>>();
    if unique.len() != allowed_user_ids.len() {
        return Err("Telegram allowed user IDs contain duplicates".into());
    }
    let url = telegram_onboarding_url(endpoint, pairing_id, profile, "/apply")?;
    let body = serde_json::json!({"allowed_user_ids": allowed_user_ids, "profile": profile});
    let receipt: TelegramOnboardingApplyReceipt =
        profile_scoped_gateway_http_request(endpoint, "POST", &url, Some(body))?;
    if !receipt.ok || receipt.platform != "telegram" {
        return Err("Hermes did not confirm Telegram bot setup".into());
    }
    Ok(receipt)
}

fn cancel_telegram_onboarding_http(
    endpoint: &GatewayEndpoint,
    pairing_id: &str,
    profile: Option<&str>,
) -> Result<TelegramOnboardingCancelReceipt, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    let url = telegram_onboarding_url(endpoint, pairing_id, profile, "")?;
    let receipt: TelegramOnboardingCancelReceipt =
        profile_scoped_gateway_http_request(endpoint, "DELETE", &url, None)?;
    if !receipt.ok {
        return Err("Hermes did not confirm Telegram setup cancellation".into());
    }
    Ok(receipt)
}

fn mutate_pairing_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    platform: &str,
    user_id: &str,
    kind: PairingMutationKind,
) -> Result<PairingMutationReceipt, String> {
    if platform.trim().is_empty()
        || platform.len() > 128
        || platform.chars().any(char::is_control)
        || user_id.trim().is_empty()
        || user_id.len() > 512
        || user_id.chars().any(char::is_control)
    {
        return Err("Hermes pairing target is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes pairing profile scope is invalid".into());
    }
    let (path, key) = match kind {
        PairingMutationKind::Approve => ("/api/pairing/approve", "request_id"),
        PairingMutationKind::Revoke => ("/api/pairing/revoke", "user_id"),
    };
    let url = format!(
        "{}{}{}",
        endpoint.api_origin.trim_end_matches('/'),
        path,
        session_profile_suffix(profile)
    );
    let mut body = serde_json::json!({"platform": platform, "profile": profile});
    body.as_object_mut()
        .expect("pairing request body is an object")
        .insert(key.into(), serde_json::Value::String(user_id.into()));
    let receipt: PairingMutationReceipt = if endpoint.remote_auth.is_some() {
        request_remote_gateway_mutation(endpoint, "POST", &url, body)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes pairing writes require the verified loopback API origin".into());
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(5)))
            .max_redirects(0)
            .build()
            .into();
        let response = agent
            .post(&url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(|_| "Hermes pairing update request failed".to_owned())?;
        let mut bytes = Vec::new();
        response
            .into_body()
            .into_reader()
            .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read Hermes pairing update response".to_owned())?;
        if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
            return Err("Hermes pairing update response exceeded 512 KiB".into());
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "Hermes returned invalid pairing update JSON".to_owned())?
    };
    if !receipt.ok
        || (kind == PairingMutationKind::Approve
            && receipt
                .user
                .as_ref()
                .is_none_or(|user| user.platform != platform || user.user_id.trim().is_empty()))
    {
        return Err("Hermes did not confirm the requested pairing change".into());
    }
    Ok(receipt)
}

fn validate_action_name(action_name: &str) -> Result<(), String> {
    if action_name.is_empty()
        || action_name.len() > 256
        || action_name.chars().any(char::is_control)
    {
        return Err("Hermes gateway action name is invalid".into());
    }
    Ok(())
}

fn restart_gateway_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<GatewayActionReceipt, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes gateway restart profile scope is invalid".into());
    }
    let url = format!(
        "{}/api/gateway/restart{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let receipt: GatewayActionReceipt =
        profile_scoped_gateway_http_request(endpoint, "POST", &url, Some(serde_json::json!({})))?;
    validate_action_name(&receipt.name)?;
    if let Some(action_id) = receipt.action_id.as_deref() {
        validate_action_name(action_id)?;
    }
    if !receipt.ok {
        return Err("Hermes did not accept the gateway restart".into());
    }
    Ok(receipt)
}

fn load_gateway_action_status_http(
    endpoint: &GatewayEndpoint,
    action_name: &str,
    profile: Option<&str>,
) -> Result<GatewayActionStatus, String> {
    load_gateway_action_status_with_lines_http(endpoint, action_name, profile, 200)
}

fn load_gateway_action_status_with_lines_http(
    endpoint: &GatewayEndpoint,
    action_name: &str,
    profile: Option<&str>,
    lines: usize,
) -> Result<GatewayActionStatus, String> {
    validate_action_name(action_name)?;
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes gateway action profile scope is invalid".into());
    }
    let lines = lines.clamp(1, 200);
    let url = format!(
        "{}/api/actions/{}/status?lines={lines}{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(action_name),
        session_profile_suffix(profile).replacen('?', "&", 1)
    );
    let status: GatewayActionStatus =
        profile_scoped_gateway_http_request(endpoint, "GET", &url, None)?;
    if status.name != action_name || (status.running && status.exit_code.is_some()) {
        return Err("Hermes returned mismatched gateway action status".into());
    }
    Ok(status)
}

fn command_center_system_action_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    operation: &CommandCenterSystemOperation,
) -> Result<CommandCenterSystemActionResult, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty()
        || profile.len() > 128
        || profile.chars().any(char::is_control)
        || profile.eq_ignore_ascii_case("all")
    {
        return Err("Hermes Command Center System profile scope is invalid".into());
    }
    let origin = endpoint.api_origin.trim_end_matches('/');
    let profile_suffix = session_profile_suffix(profile);
    match operation {
        CommandCenterSystemOperation::StartAction { action } => {
            let receipt: GatewayActionReceipt = profile_scoped_gateway_http_request(
                endpoint,
                "POST",
                &format!("{origin}{}{profile_suffix}", action.endpoint()),
                Some(serde_json::json!({})),
            )?;
            validate_action_name(&receipt.name)?;
            if !receipt.ok
                || receipt.pid.is_some_and(|pid| pid == 0)
                || receipt
                    .action_id
                    .as_deref()
                    .is_some_and(|action_id| validate_action_name(action_id).is_err())
            {
                return Err("Hermes returned an invalid system action receipt".into());
            }
            Ok(CommandCenterSystemActionResult::ActionStarted(receipt))
        }
        CommandCenterSystemOperation::LoadActionStatus { name } => {
            let status =
                load_gateway_action_status_with_lines_http(endpoint, name, Some(profile), 180)?;
            if status.lines.len() > 180
                || status
                    .lines
                    .iter()
                    .any(|line| line.len() > 8192 || line.chars().any(|ch| ch == '\0'))
            {
                return Err("Hermes returned invalid system action output".into());
            }
            Ok(CommandCenterSystemActionResult::ActionStatus(status))
        }
    }
}

fn load_command_center_system_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    log_file: &str,
    log_level: &str,
) -> Result<CommandCenterSystemSnapshot, String> {
    const LOG_FILES: [&str; 4] = ["agent", "errors", "gateway", "desktop"];
    const LOG_LEVELS: [&str; 4] = ["ALL", "INFO", "WARNING", "ERROR"];
    if !LOG_FILES.contains(&log_file) || !LOG_LEVELS.contains(&log_level) {
        return Err("Hermes Command Center log filter is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes Command Center profile scope is invalid".into());
    }
    let origin = endpoint.api_origin.trim_end_matches('/');
    let status_url = format!("{origin}/api/status{}", session_profile_suffix(profile));
    let status: CommandCenterStatus =
        profile_scoped_gateway_http_request(endpoint, "GET", &status_url, None)?;
    if status.active_sessions > 1_000_000
        || status.version.is_empty()
        || status.version.len() > 128
        || status.version.chars().any(char::is_control)
        || status
            .gateway_heartbeat_stale_s
            .is_some_and(|age| !age.is_finite() || age < 0.0)
    {
        return Err("Hermes returned invalid Command Center status".into());
    }

    let level_query = if log_level == "ALL" {
        String::new()
    } else {
        format!("&level={}", encode_query_component(log_level))
    };
    let profile_query = session_profile_suffix(profile).replacen('?', "&", 1);
    let logs_url = format!(
        "{origin}/api/logs?file={}&lines=200{level_query}{profile_query}",
        encode_query_component(log_file)
    );
    let logs: CommandCenterLogs =
        profile_scoped_gateway_http_request(endpoint, "GET", &logs_url, None)?;
    if logs.file != log_file
        || logs.lines.len() > 200
        || logs
            .lines
            .iter()
            .any(|line| line.len() > 32_768 || line.chars().any(|c| c == '\0'))
    {
        return Err("Hermes returned invalid Command Center logs".into());
    }
    Ok(CommandCenterSystemSnapshot { status, logs })
}

fn load_command_center_usage_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    period_days: usize,
) -> Result<CommandCenterUsageSnapshot, String> {
    if ![7, 30, 90].contains(&period_days) {
        return Err("Hermes Command Center usage period is invalid".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes Command Center profile scope is invalid".into());
    }
    let profile_query = session_profile_suffix(profile).replacen('?', "&", 1);
    let url = format!(
        "{}/api/analytics/usage?days={period_days}{profile_query}",
        endpoint.api_origin.trim_end_matches('/')
    );
    let snapshot: CommandCenterUsageSnapshot =
        profile_scoped_gateway_http_request(endpoint, "GET", &url, None)?;
    if snapshot.period_days != period_days
        || snapshot.daily.len() > period_days
        || snapshot.by_model.len() > 4096
        || snapshot.skills.top_skills.len() > 4096
        || snapshot
            .tools
            .as_ref()
            .is_some_and(|tools| tools.len() > 4096)
        || !snapshot.totals.total_actual_cost.is_finite()
        || snapshot.totals.total_actual_cost < 0.0
        || !snapshot.totals.total_estimated_cost.is_finite()
        || snapshot.totals.total_estimated_cost < 0.0
    {
        return Err("Hermes returned invalid Command Center usage totals".into());
    }
    let mut days = HashSet::new();
    if snapshot.daily.iter().any(|entry| {
        entry.day.len() != 10
            || !entry
                .day
                .chars()
                .enumerate()
                .all(|(index, character)| match index {
                    4 | 7 => character == '-',
                    _ => character.is_ascii_digit(),
                })
            || !days.insert(entry.day.as_str())
            || !entry.actual_cost.is_finite()
            || entry.actual_cost < 0.0
            || !entry.estimated_cost.is_finite()
            || entry.estimated_cost < 0.0
    }) {
        return Err("Hermes returned invalid Command Center daily usage".into());
    }
    if snapshot.by_model.iter().any(|entry| {
        entry.model.trim().is_empty()
            || entry.model.len() > 512
            || entry.model.chars().any(char::is_control)
            || !entry.estimated_cost.is_finite()
            || entry.estimated_cost < 0.0
    }) || snapshot.skills.top_skills.iter().any(|entry| {
        entry.skill.trim().is_empty()
            || entry.skill.len() > 512
            || entry.skill.chars().any(char::is_control)
            || !entry.percentage.is_finite()
            || !(0.0..=100.0).contains(&entry.percentage)
            || entry
                .last_used_at
                .is_some_and(|timestamp| !timestamp.is_finite())
    }) || snapshot.tools.as_ref().is_some_and(|tools| {
        tools.iter().any(|entry| {
            entry.tool.trim().is_empty()
                || entry.tool.len() > 512
                || entry.tool.chars().any(char::is_control)
                || !entry.percentage.is_finite()
                || !(0.0..=100.0).contains(&entry.percentage)
        })
    }) {
        return Err("Hermes returned invalid Command Center usage breakdown".into());
    }
    Ok(snapshot)
}

fn command_center_maintenance_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    operation: &CommandCenterMaintenanceOperation,
) -> Result<CommandCenterMaintenanceResult, String> {
    let profile = profile.unwrap_or(&endpoint.profile_id);
    if profile.is_empty() || profile.len() > 128 || profile.chars().any(char::is_control) {
        return Err("Hermes Command Center profile scope is invalid".into());
    }
    let origin = endpoint.api_origin.trim_end_matches('/');
    let profile_suffix = session_profile_suffix(profile);
    match operation {
        CommandCenterMaintenanceOperation::LoadSnapshot => {
            let curator = load_command_center_curator_http(endpoint, profile);
            let memory = load_command_center_memory_http(endpoint, profile);
            Ok(CommandCenterMaintenanceResult::Snapshot(
                CommandCenterMaintenanceSnapshot { curator, memory },
            ))
        }
        CommandCenterMaintenanceOperation::StartAction { action } => {
            let receipt: GatewayActionReceipt = profile_scoped_gateway_http_request(
                endpoint,
                "POST",
                &format!("{origin}{}{profile_suffix}", action.endpoint()),
                Some(serde_json::json!({})),
            )?;
            if receipt.name != action.action_name() || !receipt.ok {
                return Err("Hermes returned a mismatched maintenance action receipt".into());
            }
            if receipt.pid.is_some_and(|pid| pid == 0)
                || receipt
                    .action_id
                    .as_deref()
                    .is_some_and(|action_id| validate_action_name(action_id).is_err())
            {
                return Err("Hermes returned an invalid maintenance action receipt".into());
            }
            Ok(CommandCenterMaintenanceResult::ActionStarted(receipt))
        }
        CommandCenterMaintenanceOperation::LoadActionStatus { name } => {
            if !["doctor", "security-audit", "backup", "curator-run"].contains(&name.as_str()) {
                return Err("Hermes maintenance action name is invalid".into());
            }
            let status = load_gateway_action_status_http(endpoint, name, Some(profile))?;
            if status.lines.len() > 200
                || status
                    .lines
                    .iter()
                    .any(|line| line.len() > 8192 || line.chars().any(|ch| ch == '\0'))
            {
                return Err("Hermes returned invalid maintenance action output".into());
            }
            Ok(CommandCenterMaintenanceResult::ActionStatus(status))
        }
        CommandCenterMaintenanceOperation::SetCuratorPaused { paused } => {
            let receipt: CommandCenterCuratorPausedReceipt = profile_scoped_gateway_http_request(
                endpoint,
                "PUT",
                &format!("{origin}/api/curator/paused{profile_suffix}"),
                Some(serde_json::json!({ "paused": paused })),
            )?;
            if !receipt.ok || receipt.paused != *paused {
                return Err("Hermes returned a mismatched Curator pause receipt".into());
            }
            Ok(CommandCenterMaintenanceResult::CuratorPaused(receipt))
        }
        CommandCenterMaintenanceOperation::ResetMemory { target } => {
            let receipt: CommandCenterMemoryResetReceipt = profile_scoped_gateway_http_request(
                endpoint,
                "POST",
                &format!("{origin}/api/memory/reset{profile_suffix}"),
                Some(serde_json::json!({ "target": target.as_str() })),
            )?;
            if !receipt.ok
                || receipt.deleted.len() > 2
                || receipt
                    .deleted
                    .iter()
                    .any(|name| !["MEMORY.md", "USER.md"].contains(&name.as_str()))
            {
                return Err("Hermes returned an invalid memory reset receipt".into());
            }
            Ok(CommandCenterMaintenanceResult::MemoryReset(receipt))
        }
        CommandCenterMaintenanceOperation::DebugShare => {
            let receipt: CommandCenterDebugShareResponse = profile_scoped_gateway_http_request(
                endpoint,
                "POST",
                &format!("{origin}/api/ops/debug-share{profile_suffix}"),
                Some(serde_json::json!({ "consent": true })),
            )?;
            if !receipt.ok
                || receipt.urls.len() > 16
                || receipt.failures.len() > 16
                || receipt.urls.iter().any(|(key, url)| {
                    key.trim().is_empty()
                        || key.len() > 128
                        || key.chars().any(char::is_control)
                        || url.len() > 2048
                        || !url.starts_with("https://")
                        || url.chars().any(char::is_control)
                })
                || receipt.failures.iter().any(|(key, message)| {
                    key.trim().is_empty()
                        || key.len() > 128
                        || key.chars().any(char::is_control)
                        || message.len() > 2048
                        || message.chars().any(char::is_control)
                })
            {
                return Err("Hermes returned an invalid debug-share response".into());
            }
            Ok(CommandCenterMaintenanceResult::DebugShare(receipt))
        }
    }
}

fn load_command_center_curator_http(
    endpoint: &GatewayEndpoint,
    profile: &str,
) -> Result<CommandCenterMaintenanceCuratorStatus, String> {
    let url = format!(
        "{}/api/curator{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let status: CommandCenterMaintenanceCuratorStatus =
        profile_scoped_gateway_http_request(endpoint, "GET", &url, None)?;
    if status
        .interval_hours
        .into_iter()
        .chain(status.min_idle_hours)
        .chain(status.stale_after_days)
        .chain(status.archive_after_days)
        .any(|value| !value.is_finite() || value < 0.0)
        || status
            .last_run_at
            .as_ref()
            .is_some_and(|value| value.len() > 128 || value.chars().any(char::is_control))
    {
        return Err("Hermes returned invalid Curator status".into());
    }
    Ok(status)
}

fn load_command_center_memory_http(
    endpoint: &GatewayEndpoint,
    profile: &str,
) -> Result<CommandCenterMaintenanceMemoryStatus, String> {
    let url = format!(
        "{}/api/memory{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile)
    );
    let status: CommandCenterMaintenanceMemoryStatus =
        profile_scoped_gateway_http_request(endpoint, "GET", &url, None)?;
    if status.active.len() > 128
        || status.active.chars().any(char::is_control)
        || status.providers.len() > 64
    {
        return Err("Hermes returned invalid memory provider status".into());
    }
    let mut names = HashSet::new();
    if status.providers.iter().any(|provider| {
        provider.name.trim().is_empty()
            || provider.name.len() > 128
            || provider.name.chars().any(char::is_control)
            || provider.description.len() > 2048
            || provider.description.chars().any(char::is_control)
            || !["missing", "needs_config", "ready", "unavailable"]
                .contains(&provider.status.as_str())
            || !names.insert(provider.name.to_lowercase())
    }) {
        return Err("Hermes returned invalid or duplicate memory providers".into());
    }
    Ok(status)
}

fn set_messaging_platform_enabled_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    platform_id: &str,
    enabled: bool,
) -> Result<MessagingPlatformUpdateResult, String> {
    update_messaging_platform_http(
        endpoint,
        profile,
        platform_id,
        MessagingPlatformUpdate {
            enabled: Some(enabled),
            ..MessagingPlatformUpdate::default()
        },
    )
}

fn update_messaging_platform_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    platform_id: &str,
    update: MessagingPlatformUpdate,
) -> Result<MessagingPlatformUpdateResult, String> {
    if platform_id.trim().is_empty()
        || platform_id.len() > 256
        || platform_id.chars().any(char::is_control)
    {
        return Err("Hermes messaging platform ID is invalid".into());
    }
    if update.env.len() > 256
        || update.clear_env.len() > 256
        || update.env.iter().any(|(key, value)| {
            key.trim().is_empty()
                || key.len() > 256
                || key.chars().any(char::is_control)
                || value.len() > 16_384
        })
        || update.clear_env.iter().any(|key| {
            key.trim().is_empty() || key.len() > 256 || key.chars().any(char::is_control)
        })
        || (update.enabled.is_none() && update.env.is_empty() && update.clear_env.is_empty())
    {
        return Err("Hermes messaging update is invalid or exceeds its bounds".into());
    }
    let mut cleared = HashSet::new();
    if update
        .clear_env
        .iter()
        .any(|key| !cleared.insert(key.as_str()) || update.env.contains_key(key))
    {
        return Err("Hermes messaging update contains duplicate or conflicting keys".into());
    }
    let profile = profile.unwrap_or(&endpoint.profile_id);
    let url = format!(
        "{}/api/messaging/platforms/{}{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(platform_id),
        session_profile_suffix(profile)
    );
    let body = serde_json::to_value(&update)
        .map_err(|_| "Could not encode Hermes messaging update".to_owned())?;
    if body.to_string().len() > 256 * 1024 {
        return Err("Hermes messaging update exceeded 256 KiB".into());
    }
    let result: MessagingPlatformUpdateResult = if endpoint.remote_auth.is_some() {
        request_remote_gateway_mutation(endpoint, "PUT", &url, body)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes messaging updates require the verified loopback API origin".into());
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(5)))
            .max_redirects(0)
            .build()
            .into();
        let response = agent
            .put(&url)
            .header("X-Hermes-Gateway-Ticket", ticket.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(|_| "Hermes messaging platform update request failed".to_owned())?;
        let mut bytes = Vec::new();
        response
            .into_body()
            .into_reader()
            .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read Hermes messaging update response".to_owned())?;
        if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
            return Err("Hermes messaging update response exceeded 512 KiB".into());
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "Hermes returned invalid messaging update JSON".to_owned())?
    };
    if !result.ok || result.platform != platform_id {
        return Err("Hermes did not confirm the requested messaging platform".into());
    }
    Ok(result)
}

fn load_capabilities_http(endpoint: &GatewayEndpoint) -> Result<CapabilitiesSnapshot, String> {
    if endpoint.remote_auth.is_some() {
        let origin = endpoint.api_origin.trim_end_matches('/');
        let skills: Vec<SkillInfo> =
            request_remote_gateway_json(endpoint, &format!("{origin}/api/skills"))?;
        let toolsets: Vec<ToolsetInfo> =
            request_remote_gateway_json(endpoint, &format!("{origin}/api/tools/toolsets"))?;
        if skills.len() > 10_000 || toolsets.len() > 1_000 {
            return Err("Remote Hermes returned too many capability rows".into());
        }
        return Ok(CapabilitiesSnapshot { skills, toolsets });
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes capability reads require the verified loopback API origin".into());
    }
    let skills_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&skills_ticket)?;
    let toolsets_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&toolsets_ticket)?;
    read_capabilities(
        &endpoint.api_origin,
        skills_ticket.as_str(),
        toolsets_ticket.as_str(),
    )
}

fn load_display_timestamps_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<bool, String> {
    if endpoint.remote_auth.is_some() {
        let url = format!(
            "{}/api/config{}",
            endpoint.api_origin.trim_end_matches('/'),
            session_profile_suffix(profile.unwrap_or(&endpoint.profile_id))
        );
        let config: DisplayTimestampsConfig = request_remote_gateway_json(endpoint, &url)?;
        let value = config.display.and_then(|display| display.timestamps);
        return Ok(value.as_ref().is_some_and(display_timestamps_enabled));
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err(
            "Hermes display configuration requires the verified loopback API origin".into(),
        );
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    read_display_timestamps(
        &endpoint.api_origin,
        ticket.as_str(),
        profile.unwrap_or(&endpoint.profile_id),
    )
}

fn load_repo_scan_config_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
) -> Result<RepoScanPolicy, String> {
    let profile_id = profile.unwrap_or(&endpoint.profile_id);
    let url = format!(
        "{}/api/config{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile_id)
    );
    if endpoint.remote_auth.is_some() {
        let config: serde_json::Value = request_remote_gateway_json(endpoint, &url)?;
        return Ok(repo_scan_policy_from_config(&config));
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err(
            "Hermes repo discovery settings require the verified loopback API origin".into(),
        );
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    read_repo_scan_config_local(&endpoint.api_origin, ticket.as_str(), profile_id)
}

fn read_repo_scan_config_local(
    api_origin: &str,
    ticket: &str,
    profile_id: &str,
) -> Result<RepoScanPolicy, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err(
            "Hermes repo discovery settings require the verified loopback API origin".into(),
        );
    }
    let url = format!(
        "{}/api/config{}",
        api_origin.trim_end_matches('/'),
        session_profile_suffix(profile_id)
    );
    let config: serde_json::Value = request_gateway_json(api_origin, ticket, &url)?;
    Ok(repo_scan_policy_from_config(&config))
}

fn save_repo_scan_config_http(
    endpoint: &GatewayEndpoint,
    profile: Option<&str>,
    policy: &RepoScanPolicy,
) -> Result<RepoScanPolicy, String> {
    if !repo_scan_policy_is_bounded(policy) {
        return Err("Hermes repo discovery settings exceed their bounded schema".into());
    }
    let profile_id = profile.unwrap_or(&endpoint.profile_id);
    let url = format!(
        "{}/api/config{}",
        endpoint.api_origin.trim_end_matches('/'),
        session_profile_suffix(profile_id)
    );
    // Hermes config PUT deep-merges sparse patches. Sending only this section
    // preserves all unrelated profile settings and avoids a read/overwrite race.
    let body = repo_scan_config_patch(policy);
    let response: serde_json::Value = if endpoint.remote_auth.is_some() {
        request_remote_gateway_mutation(endpoint, "PUT", &url, body)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err(
                "Hermes repo discovery settings require the verified loopback API origin".into(),
            );
        }
        let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&ticket)?;
        save_repo_scan_config_local(&endpoint.api_origin, ticket.as_str(), profile_id, policy)?
    };
    if response.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("Hermes did not confirm repo discovery settings were saved".into());
    }
    Ok(policy.clone())
}

fn repo_scan_policy_is_bounded(policy: &RepoScanPolicy) -> bool {
    serde_json::to_vec(policy)
        .is_ok_and(|encoded| encoded.len() as u64 <= GATEWAY_HTTP_RESPONSE_LIMIT)
}

fn repo_scan_config_patch(policy: &RepoScanPolicy) -> serde_json::Value {
    serde_json::json!({
        "config": {
            "desktop": {
                "repo_scan_enabled": policy.enabled,
                "repo_scan_roots": policy.roots,
                "repo_scan_exclude_paths": policy.exclude_paths,
            }
        }
    })
}

fn save_repo_scan_config_local(
    api_origin: &str,
    ticket: &str,
    profile_id: &str,
    policy: &RepoScanPolicy,
) -> Result<serde_json::Value, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err(
            "Hermes repo discovery settings require the verified loopback API origin".into(),
        );
    }
    let url = format!(
        "{}/api/config{}",
        api_origin.trim_end_matches('/'),
        session_profile_suffix(profile_id)
    );
    cron_local_gateway_http_request_with_ticket(
        api_origin,
        ticket,
        "PUT",
        &url,
        Some(repo_scan_config_patch(policy)),
    )
}

fn repo_scan_policy_from_config(config: &serde_json::Value) -> RepoScanPolicy {
    let desktop = config.get("desktop").and_then(serde_json::Value::as_object);
    let enabled = desktop
        .and_then(|desktop| desktop.get("repo_scan_enabled"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let strings = |key: &str| {
        desktop
            .and_then(|desktop| desktop.get(key))
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    RepoScanPolicy {
        enabled,
        roots: strings("repo_scan_roots"),
        exclude_paths: strings("repo_scan_exclude_paths"),
    }
}

fn read_display_timestamps(
    api_origin: &str,
    ticket: &str,
    profile_id: &str,
) -> Result<bool, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err(
            "Hermes display configuration requires the verified loopback API origin".into(),
        );
    }
    let url = format!(
        "{}/api/config{}",
        api_origin.trim_end_matches('/'),
        session_profile_suffix(profile_id)
    );
    let config: DisplayTimestampsConfig = request_gateway_json(api_origin, ticket, &url)?;
    let value = config.display.and_then(|display| display.timestamps);
    Ok(value.as_ref().is_some_and(display_timestamps_enabled))
}

fn display_timestamps_enabled(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(enabled) => *enabled,
        serde_json::Value::String(value) => value == "true",
        serde_json::Value::Number(value) => value.as_i64() == Some(1),
        _ => false,
    }
}

fn set_toolset_enabled_http(
    endpoint: &GatewayEndpoint,
    name: &str,
    enabled: bool,
) -> Result<bool, String> {
    if endpoint.remote_auth.is_some() {
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err("Hermes toolset name is invalid".into());
        }
        let url = format!(
            "{}/api/tools/toolsets/{}",
            endpoint.api_origin.trim_end_matches('/'),
            encode_query_component(name)
        );
        let response: ToolsetEnabledResponse = request_remote_gateway_mutation(
            endpoint,
            "PUT",
            &url,
            serde_json::json!({"enabled":enabled}),
        )?;
        if !response.ok || response.name != name || response.enabled != enabled {
            return Err("Hermes did not confirm the requested toolset state".into());
        }
        return Ok(response.enabled);
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes toolset updates require the verified loopback API origin".into());
    }
    let ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&ticket)?;
    request_toolset_enabled(&endpoint.api_origin, ticket.as_str(), name, enabled)
}

fn request_toolset_enabled(
    api_origin: &str,
    ticket: &str,
    name: &str,
    enabled: bool,
) -> Result<bool, String> {
    validate_http_ticket(ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err("Hermes toolset updates require the verified loopback API origin".into());
    }
    if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err("Hermes toolset name is invalid".into());
    }
    let url = format!(
        "{}/api/tools/toolsets/{}",
        api_origin.trim_end_matches('/'),
        encode_query_component(name)
    );
    let body = serde_json::json!({"enabled":enabled}).to_string();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(5)))
        .max_redirects(0)
        .build()
        .into();
    let response = agent
        .put(url.as_str())
        .header("X-Hermes-Gateway-Ticket", ticket)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .send(body)
        .map_err(|_| "Hermes toolset update request failed".to_owned())?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes toolset update response".to_owned())?;
    if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
        return Err("Hermes API response exceeded 512 KiB".into());
    }
    let response: ToolsetEnabledResponse = serde_json::from_slice(&bytes)
        .map_err(|_| "Hermes returned invalid toolset update JSON".to_owned())?;
    if !response.ok || response.name != name || response.enabled != enabled {
        return Err("Hermes did not confirm the requested toolset state".into());
    }
    Ok(response.enabled)
}

fn set_session_pinned_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
    pinned: bool,
    request_id: &str,
) -> Result<(), String> {
    set_session_boolean_http(endpoint, session_id, "pinned", pinned, request_id, profile)
}

fn set_session_unread_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
    unread: bool,
    request_id: &str,
) -> Result<(), String> {
    set_session_boolean_http(endpoint, session_id, "unread", unread, request_id, profile)
}

fn set_session_archived_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
    archived: bool,
    request_id: &str,
) -> Result<(), String> {
    set_session_boolean_http(
        endpoint, session_id, "archived", archived, request_id, profile,
    )
}

fn set_session_boolean_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    field: &str,
    value: bool,
    request_id: &str,
    profile: Option<&str>,
) -> Result<(), String> {
    if !matches!(field, "pinned" | "unread" | "archived") {
        return Err("Hermes session update field is unsupported".into());
    }
    if endpoint.remote_auth.is_some() {
        validate_session_mutation_input(session_id, request_id)?;
        let profile_id = profile.unwrap_or(&endpoint.profile_id);
        let snapshot = read_remote_session_mutation_snapshot(endpoint, session_id, profile_id)?;
        validate_session_mutation_snapshot(&snapshot, session_id)?;
        request_remote_session_patch(
            endpoint,
            session_id,
            field,
            serde_json::Value::Bool(value),
            request_id,
            snapshot.runtime_revision,
            snapshot.runtime_generation.expect("validated generation"),
            profile_id,
        )?;
        return Ok(());
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes session updates require the verified loopback API origin".into());
    }
    validate_session_mutation_input(session_id, request_id)?;
    let snapshot_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&snapshot_ticket)?;
    let profile_id = profile.unwrap_or(&endpoint.profile_id);
    let snapshot = read_session_mutation_snapshot(
        &endpoint.api_origin,
        snapshot_ticket.as_str(),
        session_id,
        profile_id,
    )?;
    if !snapshot.exists
        || snapshot.session_id != session_id
        || snapshot.runtime_generation.is_none()
        || snapshot.runtime_revision > 9_007_199_254_740_991
    {
        return Err("Hermes session mutation snapshot is stale or incomplete".into());
    }
    let patch_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    validate_http_ticket(&patch_ticket)?;
    request_session_boolean_patch(
        &endpoint.api_origin,
        patch_ticket.as_str(),
        session_id,
        field,
        value,
        request_id,
        snapshot.runtime_revision,
        snapshot.runtime_generation.expect("validated generation"),
        profile_id,
    )
}

fn rename_session_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
    title: &str,
    request_id: &str,
) -> Result<String, String> {
    if title.len() > 16_384 || title.chars().any(char::is_control) {
        return Err("Hermes session title is invalid".into());
    }
    if endpoint.remote_auth.is_some() {
        validate_session_mutation_input(session_id, request_id)?;
        let profile_id = profile.unwrap_or(&endpoint.profile_id);
        let snapshot = read_remote_session_mutation_snapshot(endpoint, session_id, profile_id)?;
        validate_session_mutation_snapshot(&snapshot, session_id)?;
        let receipt = request_remote_session_patch(
            endpoint,
            session_id,
            "title",
            serde_json::Value::String(title.to_owned()),
            request_id,
            snapshot.runtime_revision,
            snapshot.runtime_generation.expect("validated generation"),
            profile_id,
        )?;
        return receipt
            .title
            .ok_or_else(|| "Hermes session rename receipt omitted its title".into());
    }
    if !is_bare_loopback_origin(&endpoint.api_origin) {
        return Err("Hermes session updates require the verified loopback API origin".into());
    }
    validate_session_mutation_input(session_id, request_id)?;
    let profile_id = profile.unwrap_or(&endpoint.profile_id);
    let snapshot_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    let snapshot = read_session_mutation_snapshot(
        &endpoint.api_origin,
        snapshot_ticket.as_str(),
        session_id,
        profile_id,
    )?;
    if !snapshot.exists
        || snapshot.session_id != session_id
        || snapshot.runtime_generation.is_none()
        || snapshot.runtime_revision > 9_007_199_254_740_991
    {
        return Err("Hermes session mutation snapshot is stale or incomplete".into());
    }
    let patch_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
    let receipt = request_session_patch(
        &endpoint.api_origin,
        patch_ticket.as_str(),
        session_id,
        "title",
        serde_json::Value::String(title.to_owned()),
        request_id,
        snapshot.runtime_revision,
        snapshot.runtime_generation.expect("validated generation"),
        profile_id,
    )?;
    receipt
        .title
        .ok_or_else(|| "Hermes session rename receipt omitted its title".into())
}

fn delete_session_http(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile: Option<&str>,
    request_id: &str,
) -> Result<(), String> {
    validate_session_mutation_input(session_id, request_id)?;
    let profile_id = profile.unwrap_or(&endpoint.profile_id);
    if profile_id.len() > 128
        || profile_id.chars().any(char::is_control)
        || profile_id.eq_ignore_ascii_case("all")
    {
        return Err("Hermes session delete profile scope is invalid".into());
    }
    let snapshot = if endpoint.remote_auth.is_some() {
        read_remote_session_mutation_snapshot(endpoint, session_id, profile_id)?
    } else {
        if !is_bare_loopback_origin(&endpoint.api_origin) {
            return Err("Hermes session updates require the verified loopback API origin".into());
        }
        let snapshot_ticket = mint_windows_ticket(endpoint, "native-http", None)?;
        validate_http_ticket(&snapshot_ticket)?;
        read_session_mutation_snapshot(
            &endpoint.api_origin,
            snapshot_ticket.as_str(),
            session_id,
            profile_id,
        )?
    };
    validate_session_mutation_snapshot(&snapshot, session_id)?;
    let expected_generation = snapshot.runtime_generation.expect("validated generation");
    let mut query = Vec::new();
    if !profile_id.is_empty() && profile_id != "default" {
        query.push(format!("profile={}", encode_query_component(profile_id)));
    }
    query.push(format!("expected_revision={}", snapshot.runtime_revision));
    query.push(format!("expected_generation={expected_generation}"));
    query.push(format!("request_id={}", encode_query_component(request_id)));
    let url = format!(
        "{}/api/sessions/{}?{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(session_id),
        query.join("&")
    );
    let receipt: SessionMutationReceipt =
        profile_scoped_gateway_http_request(endpoint, "DELETE", &url, None)?;
    if !receipt.ok {
        return Err("Hermes did not confirm the requested session deletion".into());
    }
    Ok(())
}

fn validate_session_mutation_input(session_id: &str, request_id: &str) -> Result<(), String> {
    if session_id.is_empty() || session_id.len() > 512 || session_id.chars().any(char::is_control) {
        return Err("Hermes session ID is invalid".into());
    }
    if request_id.is_empty() || request_id.len() > 128 || request_id.chars().any(char::is_control) {
        return Err("Hermes session mutation request ID is invalid".into());
    }
    Ok(())
}

fn session_profile_suffix(profile_id: &str) -> String {
    if profile_id.is_empty() || profile_id == "default" {
        String::new()
    } else {
        format!("?profile={}", encode_query_component(profile_id))
    }
}

fn read_session_mutation_snapshot(
    api_origin: &str,
    ticket: &str,
    session_id: &str,
    profile_id: &str,
) -> Result<SessionMutationSnapshot, String> {
    validate_http_ticket(ticket)?;
    validate_session_mutation_input(session_id, "snapshot-read")?;
    if !is_bare_loopback_origin(api_origin) {
        return Err("Hermes session updates require the verified loopback API origin".into());
    }
    let path = format!(
        "{}/api/sessions/{}/mutation-snapshot{}",
        api_origin.trim_end_matches('/'),
        encode_query_component(session_id),
        session_profile_suffix(profile_id)
    );
    request_gateway_json(api_origin, ticket, &path)
}

#[allow(clippy::too_many_arguments)]
fn request_session_boolean_patch(
    api_origin: &str,
    ticket: &str,
    session_id: &str,
    field: &str,
    value: bool,
    request_id: &str,
    expected_revision: u64,
    expected_generation: u64,
    profile_id: &str,
) -> Result<(), String> {
    if !matches!(field, "pinned" | "unread" | "archived") {
        return Err("Hermes session update field is unsupported".into());
    }
    request_session_patch(
        api_origin,
        ticket,
        session_id,
        field,
        serde_json::Value::Bool(value),
        request_id,
        expected_revision,
        expected_generation,
        profile_id,
    )
    .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
fn request_session_patch(
    api_origin: &str,
    ticket: &str,
    session_id: &str,
    field: &str,
    value: serde_json::Value,
    request_id: &str,
    expected_revision: u64,
    expected_generation: u64,
    profile_id: &str,
) -> Result<SessionMutationReceipt, String> {
    validate_http_ticket(ticket)?;
    validate_session_mutation_input(session_id, request_id)?;
    if !matches!(field, "pinned" | "unread" | "archived" | "title") {
        return Err("Hermes session update field is unsupported".into());
    }
    if !is_bare_loopback_origin(api_origin) {
        return Err("Hermes session updates require the verified loopback API origin".into());
    }
    if expected_revision > 9_007_199_254_740_991 || expected_generation > 9_007_199_254_740_991 {
        return Err("Hermes session mutation identity exceeds the supported integer range".into());
    }
    let url = format!(
        "{}/api/sessions/{}",
        api_origin.trim_end_matches('/'),
        encode_query_component(session_id)
    );
    let mut body = serde_json::json!({
        "request_id": request_id,
        "expected_revision": expected_revision,
        "expected_generation": expected_generation
    });
    let archive_enabled = field == "archived" && value == serde_json::Value::Bool(true);
    body[field] = value;
    if archive_enabled {
        body["pinned"] = serde_json::Value::Bool(false);
    }
    if !profile_id.is_empty() && profile_id != "default" {
        body["profile"] = serde_json::Value::String(profile_id.to_owned());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(5)))
        .max_redirects(0)
        .build()
        .into();
    let response = agent
        .patch(url.as_str())
        .header("X-Hermes-Gateway-Ticket", ticket)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .send(body.to_string())
        .map_err(|_| "Hermes session update request failed".to_owned())?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes session update response".to_owned())?;
    if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
        return Err("Hermes API response exceeded 512 KiB".into());
    }
    let receipt: SessionMutationReceipt = serde_json::from_slice(&bytes)
        .map_err(|_| "Hermes returned invalid session update JSON".to_owned())?;
    if !receipt.ok {
        return Err("Hermes did not confirm the requested session state".into());
    }
    Ok(receipt)
}

fn read_capabilities(
    api_origin: &str,
    skills_ticket: &str,
    toolsets_ticket: &str,
) -> Result<CapabilitiesSnapshot, String> {
    validate_http_ticket(skills_ticket)?;
    validate_http_ticket(toolsets_ticket)?;
    if !is_bare_loopback_origin(api_origin) {
        return Err("Hermes capability reads require the verified loopback API origin".into());
    }
    let origin = api_origin.trim_end_matches('/');
    let skills: Vec<SkillInfo> =
        request_gateway_json(api_origin, skills_ticket, &format!("{origin}/api/skills"))?;
    if skills.len() > 10_000 {
        return Err("Hermes returned too many skills".into());
    }

    let toolsets: Vec<ToolsetInfo> = request_gateway_json(
        api_origin,
        toolsets_ticket,
        &format!("{origin}/api/tools/toolsets"),
    )?;
    if toolsets.len() > 1_000 {
        return Err("Hermes returned too many toolsets".into());
    }
    Ok(CapabilitiesSnapshot { skills, toolsets })
}

fn validate_http_ticket(ticket: &str) -> Result<(), String> {
    if ticket.is_empty() || ticket.len() > 2048 || !ticket.bytes().all(is_http_token_byte) {
        Err("Hermes returned an invalid native HTTP ticket".into())
    } else {
        Ok(())
    }
}

fn request_gateway_json<T: DeserializeOwned>(
    api_origin: &str,
    ticket: &str,
    url: &str,
) -> Result<T, String> {
    request_gateway_json_with_limit(api_origin, ticket, url, GATEWAY_HTTP_RESPONSE_LIMIT)
}

fn request_gateway_json_with_limit<T: DeserializeOwned>(
    api_origin: &str,
    ticket: &str,
    url: &str,
    response_limit: u64,
) -> Result<T, String> {
    request_gateway_json_with_timeout_and_limit(
        api_origin,
        ticket,
        url,
        Duration::from_secs(5),
        response_limit,
    )
}

fn request_gateway_json_with_timeout<T: DeserializeOwned>(
    api_origin: &str,
    ticket: &str,
    url: &str,
    timeout: Duration,
) -> Result<T, String> {
    request_gateway_json_with_timeout_and_limit(
        api_origin,
        ticket,
        url,
        timeout,
        GATEWAY_HTTP_RESPONSE_LIMIT,
    )
}

fn request_gateway_json_with_timeout_and_limit<T: DeserializeOwned>(
    api_origin: &str,
    ticket: &str,
    url: &str,
    timeout: Duration,
    response_limit: u64,
) -> Result<T, String> {
    if !is_bare_loopback_origin(api_origin) || !url.starts_with(api_origin) {
        return Err("Hermes API request target is outside the verified loopback origin".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .build()
        .into();
    let response = agent
        .get(url)
        .header("X-Hermes-Gateway-Ticket", ticket)
        .header("Accept", "application/json")
        .call()
        .map_err(|error| format!("Hermes API read request failed: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(response_limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read Hermes API response".to_owned())?;
    if bytes.len() as u64 > response_limit {
        return Err("Hermes API response exceeded its bounded read limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Hermes returned invalid API JSON".to_owned())
}

fn request_remote_gateway_json<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    url: &str,
) -> Result<T, String> {
    request_remote_gateway_json_with_limit(endpoint, url, GATEWAY_HTTP_RESPONSE_LIMIT)
}

fn request_remote_gateway_json_with_limit<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    url: &str,
    response_limit: u64,
) -> Result<T, String> {
    request_remote_gateway_json_with_timeout_and_limit(
        endpoint,
        url,
        Duration::from_secs(8),
        response_limit,
    )
}

fn request_remote_gateway_json_with_timeout<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    url: &str,
    timeout: Duration,
) -> Result<T, String> {
    request_remote_gateway_json_with_timeout_and_limit(
        endpoint,
        url,
        timeout,
        GATEWAY_HTTP_RESPONSE_LIMIT,
    )
}

fn request_remote_gateway_json_with_timeout_and_limit<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    url: &str,
    timeout: Duration,
    response_limit: u64,
) -> Result<T, String> {
    if endpoint.remote_auth.is_none() || !api_url_is_within_origin(&endpoint.api_origin, url) {
        return Err("Remote Hermes API request target is invalid".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .build()
        .into();
    let request = match endpoint.remote_auth.as_ref().expect("remote auth checked") {
        RemoteGatewayAuth::Token(token) => agent
            .get(url)
            .header("X-Hermes-Session-Token", token.as_str()),
        auth @ RemoteGatewayAuth::OAuth { .. } => {
            let token = remote_access_token(&endpoint.api_origin, auth)?;
            agent
                .get(url)
                .header("Authorization", &format!("Bearer {}", token.as_str()))
        }
    };
    let response = request
        .header("Accept", "application/json")
        .call()
        .map_err(|error| format!("Remote Hermes API read request failed: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(response_limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read remote Hermes API response".to_owned())?;
    if bytes.len() as u64 > response_limit {
        return Err("Remote Hermes API response exceeded its bounded read limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Remote Hermes returned invalid API JSON".to_owned())
}

fn remote_http_auth(endpoint: &GatewayEndpoint) -> Result<RemoteHttpAuth, String> {
    match endpoint
        .remote_auth
        .as_ref()
        .ok_or_else(|| "Remote Hermes credentials are unavailable".to_owned())?
    {
        RemoteGatewayAuth::Token(token) => Ok(RemoteHttpAuth::SessionToken(token.clone())),
        auth @ RemoteGatewayAuth::OAuth { .. } => Ok(RemoteHttpAuth::Bearer(remote_access_token(
            &endpoint.api_origin,
            auth,
        )?)),
    }
}

fn request_remote_gateway_mutation<T: DeserializeOwned>(
    endpoint: &GatewayEndpoint,
    method: &str,
    url: &str,
    body: serde_json::Value,
) -> Result<T, String> {
    if endpoint.remote_auth.is_none()
        || !api_url_is_within_origin(&endpoint.api_origin, url)
        || !matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
    {
        return Err("Remote Hermes API mutation target is invalid".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .max_redirects(0)
        .build()
        .into();
    let auth = remote_http_auth(endpoint)?;
    let body = body.to_string();
    let response = match (method, auth) {
        ("POST", RemoteHttpAuth::SessionToken(token)) => agent
            .post(url)
            .header("X-Hermes-Session-Token", token.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body),
        ("POST", RemoteHttpAuth::Bearer(token)) => agent
            .post(url)
            .header("Authorization", &format!("Bearer {}", token.as_str()))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body),
        ("PUT", RemoteHttpAuth::SessionToken(token)) => agent
            .put(url)
            .header("X-Hermes-Session-Token", token.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body),
        ("PUT", RemoteHttpAuth::Bearer(token)) => agent
            .put(url)
            .header("Authorization", &format!("Bearer {}", token.as_str()))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body),
        ("PATCH", RemoteHttpAuth::SessionToken(token)) => agent
            .patch(url)
            .header("X-Hermes-Session-Token", token.as_str())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body),
        ("PATCH", RemoteHttpAuth::Bearer(token)) => agent
            .patch(url)
            .header("Authorization", &format!("Bearer {}", token.as_str()))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send(body),
        ("DELETE", RemoteHttpAuth::SessionToken(token)) => agent
            .delete(url)
            .header("X-Hermes-Session-Token", token.as_str())
            .header("Accept", "application/json")
            .call(),
        ("DELETE", RemoteHttpAuth::Bearer(token)) => agent
            .delete(url)
            .header("Authorization", &format!("Bearer {}", token.as_str()))
            .header("Accept", "application/json")
            .call(),
        _ => unreachable!("method checked"),
    }
    .map_err(|_| "Remote Hermes API mutation request failed".to_owned())?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(GATEWAY_HTTP_RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read remote Hermes API mutation response".to_owned())?;
    if bytes.len() as u64 > GATEWAY_HTTP_RESPONSE_LIMIT {
        return Err("Remote Hermes API response exceeded 512 KiB".into());
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| "Remote Hermes returned invalid mutation JSON".to_owned())
}

fn read_remote_session_mutation_snapshot(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    profile_id: &str,
) -> Result<SessionMutationSnapshot, String> {
    let url = format!(
        "{}/api/sessions/{}/mutation-snapshot{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(session_id),
        session_profile_suffix(profile_id)
    );
    request_remote_gateway_json(endpoint, &url)
}

fn validate_session_mutation_snapshot(
    snapshot: &SessionMutationSnapshot,
    session_id: &str,
) -> Result<(), String> {
    if !snapshot.exists
        || snapshot.session_id != session_id
        || snapshot.runtime_generation.is_none()
        || snapshot.runtime_revision > 9_007_199_254_740_991
    {
        return Err("Hermes session mutation snapshot is stale or incomplete".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn request_remote_session_patch(
    endpoint: &GatewayEndpoint,
    session_id: &str,
    field: &str,
    value: serde_json::Value,
    request_id: &str,
    expected_revision: u64,
    expected_generation: u64,
    profile_id: &str,
) -> Result<SessionMutationReceipt, String> {
    validate_session_mutation_input(session_id, request_id)?;
    if !matches!(field, "pinned" | "unread" | "archived" | "title")
        || expected_revision > 9_007_199_254_740_991
        || expected_generation > 9_007_199_254_740_991
    {
        return Err("Hermes session mutation fields are invalid".into());
    }
    let url = format!(
        "{}/api/sessions/{}",
        endpoint.api_origin.trim_end_matches('/'),
        encode_query_component(session_id)
    );
    let mut body = serde_json::json!({
        "request_id": request_id,
        "expected_revision": expected_revision,
        "expected_generation": expected_generation
    });
    let archive_enabled = field == "archived" && value == serde_json::Value::Bool(true);
    body[field] = value;
    if archive_enabled {
        body["pinned"] = serde_json::Value::Bool(false);
    }
    if !profile_id.is_empty() && profile_id != "default" {
        body["profile"] = serde_json::Value::String(profile_id.to_owned());
    }
    let receipt: SessionMutationReceipt =
        request_remote_gateway_mutation(endpoint, "PATCH", &url, body)?;
    if !receipt.ok {
        return Err("Hermes did not confirm the requested session state".into());
    }
    Ok(receipt)
}

fn api_url_is_within_origin(api_origin: &str, target: &str) -> bool {
    let (Ok(origin), Ok(target)) = (Url::parse(api_origin), Url::parse(target)) else {
        return false;
    };
    if !origin.username().is_empty()
        || origin.password().is_some()
        || !target.username().is_empty()
        || target.password().is_some()
        || target.scheme() != origin.scheme()
        || target.host_str() != origin.host_str()
        || target.port_or_known_default() != origin.port_or_known_default()
        || target.fragment().is_some()
    {
        return false;
    }
    let prefix = origin.path().trim_end_matches('/');
    prefix.is_empty() || target.path() == prefix || target.path().starts_with(&format!("{prefix}/"))
}

fn request_remote_session_page(
    endpoint: &GatewayEndpoint,
    url: &str,
) -> Result<SessionListPage, String> {
    let response: SessionListResponse = request_remote_gateway_json(endpoint, url)?;
    let sessions = response
        .sessions
        .into_iter()
        .map(|session| SessionSummary {
            id: session.id,
            connection_id: session.connection_id,
            lineage_root_id: session.lineage_root_id,
            cwd: session.cwd,
            profile: session.profile,
            title: session.title.unwrap_or_default(),
            preview: session.preview.unwrap_or_default(),
            message_count: session.message_count,
            source: session.source.unwrap_or_default(),
            started_at: session.started_at,
            last_active: session.last_active,
            pinned: session.pinned,
            unread: session.unread,
            archived: session.archived,
        })
        .collect();
    Ok(SessionListPage {
        sessions,
        total: response.total,
    })
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn set_socket_deadlines(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) -> Result<(), String> {
    let read_deadline = Some(Duration::from_millis(100));
    let write_deadline = Some(Duration::from_secs(60));
    let result = match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => {
            set_stream_deadlines(stream, read_deadline, write_deadline)
        }
        tungstenite::stream::MaybeTlsStream::NativeTls(stream) => {
            set_stream_deadlines(stream.get_mut(), read_deadline, write_deadline)
        }
        _ => Err(io::Error::other("unsupported TLS stream")),
    };
    result.map_err(|_| "Could not configure the Hermes gateway deadline".to_owned())
}

fn set_stream_deadlines(
    stream: &TcpStream,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
) -> io::Result<()> {
    stream.set_read_timeout(read_timeout)?;
    stream.set_write_timeout(write_timeout)
}

fn wait_for_gateway_ready(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) && Instant::now() < deadline =>
            {
                continue;
            }
            Err(_) => return Err("Timed out waiting for Hermes gateway readiness".into()),
        };
        let Message::Text(text) = message else {
            if matches!(message, Message::Close(_)) {
                return Err("Hermes gateway closed before becoming ready".into());
            }
            continue;
        };
        let frame: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| "Hermes gateway sent invalid readiness data".to_owned())?;
        if frame.get("method").and_then(|value| value.as_str()) == Some("event")
            && frame
                .get("params")
                .and_then(|params| params.get("type"))
                .and_then(|value| value.as_str())
                == Some("gateway.ready")
        {
            return Ok(());
        }
    }
}

fn websocket_url(origin: &str) -> Result<String, String> {
    let (scheme, authority) = origin
        .split_once("://")
        .ok_or_else(|| "Hermes returned an invalid gateway origin".to_owned())?;
    let websocket_scheme = match scheme {
        "http" => "ws",
        "https" => "wss",
        _ => return Err("Hermes returned an unsupported gateway scheme".into()),
    };
    Ok(format!("{websocket_scheme}://{authority}/api/ws"))
}

fn is_http_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn rpc(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    id: u64,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    rpc_with_events(socket, id, method, params, None)
}

fn rpc_with_events(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    id: u64,
    method: &str,
    params: serde_json::Value,
    events: Option<&mpsc::Sender<GatewayEvent>>,
) -> Result<serde_json::Value, String> {
    rpc_with_events_timeout(socket, id, method, params, Duration::from_secs(60), events)
}

fn rpc_with_events_timeout(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    id: u64,
    method: &str,
    params: serde_json::Value,
    timeout: Duration,
    events: Option<&mpsc::Sender<GatewayEvent>>,
) -> Result<serde_json::Value, String> {
    let deadline = Instant::now() + timeout;
    let request =
        serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    socket
        .send(Message::Text(request.to_string().into()))
        .map_err(|_| format!("Could not send Hermes {method} request"))?;
    loop {
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) && Instant::now() < deadline =>
            {
                continue;
            }
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(format!("Hermes {method} request timed out"));
            }
            Err(_) => return Err(format!("Hermes gateway disconnected during {method}")),
        };
        let Message::Text(text) = message else {
            if matches!(message, Message::Close(_)) {
                return Err(format!("Hermes gateway closed during {method}"));
            }
            continue;
        };
        let frame: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| "Hermes gateway sent invalid JSON-RPC data".to_owned())?;
        if let Some(response) = unhandled_server_request_response(&frame) {
            socket
                .send(Message::Text(response.to_string().into()))
                .map_err(|_| "Could not decline an unsupported Hermes server request".to_owned())?;
            continue;
        }
        if frame.get("id").and_then(|value| value.as_u64()) != Some(id) {
            if frame.get("method").and_then(|value| value.as_str()) == Some("event")
                && let (Some(sender), Some(params)) = (events, frame.get("params"))
            {
                let _ = sender.send(GatewayEvent::ServerEvent(params.clone()));
            }
            continue;
        }
        if let Some(error) = frame.get("error") {
            let reason = error
                .get("data")
                .and_then(|data| data.get("reason"))
                .and_then(|reason| reason.as_str())
                .unwrap_or("request_failed");
            return Err(format!("Hermes {method} refused: {reason}"));
        }
        return frame
            .get("result")
            .cloned()
            .ok_or_else(|| format!("Hermes {method} response omitted its result"));
    }
}

fn unhandled_server_request_response(frame: &serde_json::Value) -> Option<serde_json::Value> {
    let id = frame.get("id")?.as_str()?;
    let method = frame.get("method")?.as_str()?;
    if method == "event" {
        return None;
    }
    Some(serde_json::json!({
        "jsonrpc":"2.0",
        "id":id,
        "error":{
            "code":-32601,
            "message":format!("no handler for server request: {method}")
        }
    }))
}

fn server_request_parts(frame: &serde_json::Value) -> Option<(String, String, serde_json::Value)> {
    let id = frame.get("id")?.as_str()?.to_owned();
    let method = frame.get("method")?.as_str()?.to_owned();
    if method == "event" {
        return None;
    }
    let params = frame
        .get("params")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Some((id, method, params))
}

fn validate_endpoint(endpoint: &GatewayEndpoint) -> Result<(), String> {
    let loopback = is_bare_loopback_origin(&endpoint.api_origin);
    if !loopback
        || endpoint.runtime_protocol != 1
        || endpoint.profile_id.is_empty()
        || endpoint.instance_id.is_empty()
        || endpoint.authority_epoch == 0
        || !endpoint
            .capabilities
            .iter()
            .any(|capability| capability == "session-authority-v1")
    {
        return Err("Hermes returned an incompatible or unsafe local gateway endpoint".into());
    }
    Ok(())
}

fn is_bare_loopback_origin(origin: &str) -> bool {
    let Some(authority) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    if authority.contains(['/', '?', '#', '@']) {
        return false;
    }
    let Some((host, port)) = authority.rsplit_once(':') else {
        return false;
    };
    let valid_host = host == "127.0.0.1" || host == "[::1]";
    let valid_port = port.parse::<u16>().is_ok_and(|value| value != 0);
    valid_host && valid_port
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_discovery_shutdown_closes_actor_and_rejects_late_events() {
        let (_status_sender, status_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (actor_stopped_sender, actor_stopped_receiver) = mpsc::channel();
        let actor = thread::spawn(move || {
            while command_receiver.recv().is_ok() {}
            actor_stopped_sender
                .send(())
                .expect("report actor shutdown");
        });
        let late_event_sender = event_sender.clone();

        let (plugin_update_all_sender, plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, http_receiver) = mpsc::channel();
        let (project_tree_sender, project_tree_receiver) = mpsc::channel();
        let (cron_trigger_sender, cron_trigger_receiver) = mpsc::sync_channel(1);
        let (artifact_sender, artifact_receiver) = mpsc::channel();
        let (artifact_download_sender, artifact_download_receiver) = mpsc::channel();
        let (artifact_image_sender, artifact_image_receiver) = mpsc::channel();
        drop((
            plugin_update_all_receiver,
            http_receiver,
            project_tree_receiver,
            cron_trigger_receiver,
            artifact_receiver,
            artifact_download_receiver,
            artifact_image_receiver,
        ));

        let discovery = GatewayDiscovery {
            status_receiver,
            event_receiver,
            command_sender,
            plugin_update_all_sender,
            http_sender,
            project_tree_sender,
            cron_trigger_sender,
            artifact_sender,
            artifact_download_sender,
            artifact_image_sender,
        };
        discovery.shutdown();

        actor_stopped_receiver
            .recv_timeout(Duration::from_millis(250))
            .expect("actor should observe its closed command channel");
        actor.join().expect("actor thread should terminate");
        assert!(
            late_event_sender
                .send(GatewayEvent::Disconnected("late event".into()))
                .is_err()
        );
    }

    #[test]
    fn remote_gateway_auth_debug_redacts_both_credential_kinds() {
        let token = RemoteGatewayAuth::Token(Zeroizing::new("do-not-log-token".into()));
        assert!(!format!("{token:?}").contains("do-not-log-token"));
        let oauth = RemoteGatewayAuth::OAuth {
            session: Arc::new(Mutex::new(NativeOAuthSession {
                access_token: Zeroizing::new("do-not-log-access".into()),
                refresh_token: Zeroizing::new("do-not-log-refresh".into()),
                expires_at: 0,
                provider: "fixture".into(),
                user_id: "fixture-user".into(),
            })),
            credential_store_directory: std::path::PathBuf::from("fixture-profile"),
        };
        let debug = format!("{oauth:?}");
        assert!(!debug.contains("do-not-log-access"));
        assert!(!debug.contains("do-not-log-refresh"));
    }

    #[test]
    fn remote_gateway_attach_rejects_auth_mode_mismatch_before_network_io() {
        let result = attach_remote_gateway(
            "https://gateway.example.com/hermes",
            RemoteAuthMode::OAuth,
            RemoteGatewayAuth::Token(Zeroizing::new("fixture-token".into())),
        );
        assert!(matches!(
            result,
            Err(error) if error == "Remote Hermes authentication mode did not match its saved credential"
        ));
    }

    #[test]
    fn remote_api_target_stays_within_normalized_gateway_prefix() {
        assert!(api_url_is_within_origin(
            "https://gateway.example.com/hermes",
            "https://gateway.example.com/hermes/api/sessions?limit=20"
        ));
        assert!(!api_url_is_within_origin(
            "https://gateway.example.com/hermes",
            "https://gateway.example.com/other/api/sessions"
        ));
        assert!(!api_url_is_within_origin(
            "https://gateway.example.com/hermes",
            "https://gateway.example.com.evil.test/hermes/api/sessions"
        ));
        assert!(!api_url_is_within_origin(
            "https://gateway.example.com/hermes",
            "https://gateway.example.com/hermes-evil/api/sessions"
        ));
    }

    #[test]
    fn artifact_index_reads_pinned_recent_sessions_and_oldest_pages_with_remote_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind artifact index fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            use std::io::Write as _;
            let (mut session_stream, _) = listener.accept().expect("accept session list");
            let sessions_request =
                read_http_fixture_request(&mut session_stream).to_ascii_lowercase();
            assert!(sessions_request.contains("get /hermes/api/profiles/sessions?limit=30&offset=0&min_messages=1&archived=exclude&order=recent&profile=all http/1.1"));
            assert!(
                sessions_request.contains("x-hermes-session-token: artifact-fixture-token\r\n")
            );
            let sessions = json!({"total":1,"sessions":[{
                "id":"artifact-session","profile":"work","title":"Generated illustration",
                "preview":"Create an image","started_at":1700000000.0,"last_active":1700000001.0
            }]})
            .to_string();
            write!(session_stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", sessions.len(), sessions).expect("write session list");

            let (mut messages_stream, _) = listener.accept().expect("accept transcript page");
            let messages_request =
                read_http_fixture_request(&mut messages_stream).to_ascii_lowercase();
            assert!(messages_request.contains("get /hermes/api/sessions/artifact-session/messages?profile=work&limit=100&offset=0&order=oldest&include_compacted=true http/1.1"));
            assert!(
                messages_request.contains("x-hermes-session-token: artifact-fixture-token\r\n")
            );
            let messages = json!({
                "session_id":"artifact-session","profile":"work",
                "messages":[{"role":"assistant","timestamp":1700000002.0,"content":"MEDIA:/tmp/generated.png"}],
                "pagination":{"limit":100,"offset":0,"order":"oldest","returned":1}
            }).to_string();
            write!(messages_stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", messages.len(), messages).expect("write transcript page");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "owner-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "artifact-fixture-token".into(),
            ))),
        };
        let snapshot = load_artifact_index_http(&endpoint).expect("artifact index completes");
        assert_eq!(snapshot.session_count, 1);
        assert!(snapshot.failures.is_empty());
        assert_eq!(snapshot.artifacts.len(), 1);
        assert_eq!(snapshot.artifacts[0].value, "/tmp/generated.png");
        assert_eq!(snapshot.artifacts[0].profile.as_deref(), Some("work"));
        server.join().expect("artifact HTTP fixture exits");
    }

    #[test]
    fn remote_messaging_platforms_read_preserves_profile_status_and_rejects_duplicate_rows() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind messaging fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            for (index, body) in [
                r#"{"platforms":[{"id":"telegram","name":"Telegram","description":"Telegram bot","configured":true,"enabled":true,"gateway_running":true,"state":"connected","docs_url":"https://docs.example/telegram"}]}"#,
                r#"{"platforms":[{"id":"same","name":"One"},{"id":"same","name":"Two"}]}"#,
            ].into_iter().enumerate() {
                let (mut stream, _) = listener.accept().expect("accept messaging API read");
                let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
                assert!(request.starts_with("get /hermes/api/messaging/platforms?profile=work http/1.1\r\n"));
                assert!(request.contains("x-hermes-session-token: messaging-fixture-token\r\n"));
                assert!(!request.contains("x-hermes-gateway-ticket:"));
                write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                ).expect("write messaging response");
                assert!(index < 2);
            }
            let (mut stream, _) = listener.accept().expect("accept messaging update");
            let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
            assert!(request.starts_with(
                "put /hermes/api/messaging/platforms/tele%2fgram?profile=work http/1.1\r\n"
            ));
            assert!(request.contains("x-hermes-session-token: messaging-fixture-token\r\n"));
            assert!(!request.contains("x-hermes-gateway-ticket:"));
            assert!(request.ends_with("{\"enabled\":true}"));
            let body = r#"{"ok":true,"platform":"tele/gram","hot_served":true}"#;
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            ).expect("write messaging update receipt");
            let (mut stream, _) = listener.accept().expect("accept messaging config update");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(lower.starts_with(
                "put /hermes/api/messaging/platforms/telegram?profile=work http/1.1\r\n"
            ));
            assert!(lower.contains("x-hermes-session-token: messaging-fixture-token\r\n"));
            assert!(!lower.contains("x-hermes-gateway-ticket:"));
            let body: serde_json::Value = serde_json::from_str(
                request
                    .split_once("\r\n\r\n")
                    .expect("configuration body separator")
                    .1,
            )
            .expect("parse configuration update");
            assert_eq!(body["env"]["TELEGRAM_TOKEN"], "fixture-secret");
            assert_eq!(body["env"]["TELEGRAM_ALLOW_FROM"], "111,222");
            assert_eq!(body["clear_env"][0], "OLD_CHANNEL_ID");
            assert!(body.get("enabled").is_none());
            let body = r#"{"ok":true,"platform":"telegram","hot_served":false}"#;
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            ).expect("write messaging config receipt");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "messaging-fixture-token".into(),
            ))),
        };
        let snapshot = load_messaging_platforms_http(&endpoint, Some("work"))
            .expect("profile-scoped messaging inventory");
        assert_eq!(snapshot.platforms.len(), 1);
        assert_eq!(snapshot.platforms[0].id, "telegram");
        assert_eq!(snapshot.platforms[0].state.as_deref(), Some("connected"));
        assert!(snapshot.platforms[0].enabled && snapshot.platforms[0].configured);
        let duplicate = load_messaging_platforms_http(&endpoint, Some("work"));
        assert!(matches!(duplicate, Err(error) if error.contains("duplicate messaging platform")));
        let update =
            set_messaging_platform_enabled_http(&endpoint, Some("work"), "tele/gram", true)
                .expect("profile-scoped platform update");
        assert!(update.ok && update.hot_served == Some(true));
        assert!(matches!(
            set_messaging_platform_enabled_http(&endpoint, Some("work"), "bad\nid", true),
            Err(error) if error.contains("platform ID is invalid")
        ));
        let config_update = MessagingPlatformUpdate {
            enabled: None,
            env: BTreeMap::from([
                ("TELEGRAM_TOKEN".into(), "fixture-secret".into()),
                ("TELEGRAM_ALLOW_FROM".into(), "111,222".into()),
            ]),
            clear_env: vec!["OLD_CHANNEL_ID".into()],
        };
        let receipt = update_messaging_platform_http(
            &endpoint,
            Some("work"),
            "telegram",
            config_update.clone(),
        )
        .expect("profile-scoped channel configuration update");
        assert!(receipt.ok && receipt.hot_served == Some(false));
        let conflict = MessagingPlatformUpdate {
            env: BTreeMap::from([("SAME".into(), "value".into())]),
            clear_env: vec!["SAME".into()],
            ..MessagingPlatformUpdate::default()
        };
        assert!(matches!(
            update_messaging_platform_http(&endpoint, Some("work"), "telegram", conflict),
            Err(error) if error.contains("conflicting keys")
        ));
        let oversized = MessagingPlatformUpdate {
            env: (0..17)
                .map(|index| (format!("KEY_{index}"), "x".repeat(16_000)))
                .collect(),
            ..MessagingPlatformUpdate::default()
        };
        assert!(matches!(
            update_messaging_platform_http(&endpoint, Some("work"), "telegram", oversized),
            Err(error) if error.contains("256 KiB")
        ));
        server.join().expect("messaging fixture exits");
    }

    #[test]
    fn remote_cron_jobs_read_is_profile_scoped_authenticated_and_bounded() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cron fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            for body in [
                r#"[{"id":"daily-summary","name":"Daily summary","prompt":"Summarize project activity","enabled":true,"state":"scheduled","schedule_display":"Daily at 9:00 AM","schedule":{"expr":"0 9 * * *"},"deliver":"local","model":"openai/gpt-fixture","provider":"openai","next_run_at":"2026-11-17T09:00:00Z"}]"#,
                r#"[{"id":"duplicate","name":"One"},{"id":"duplicate","name":"Two"}]"#,
            ] {
                let (mut stream, _) = listener.accept().expect("accept cron read");
                let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
                assert!(request.starts_with("get /hermes/api/cron/jobs?profile=work http/1.1\r\n"));
                assert!(request.contains("x-hermes-session-token: cron-fixture-token\r\n"));
                assert!(!request.contains("x-hermes-gateway-ticket:"));
                write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                ).expect("write cron jobs");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "cron-fixture-token".into(),
            ))),
        };
        let jobs =
            load_cron_jobs_http(&endpoint, Some("work")).expect("profile-scoped scheduled jobs");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "daily-summary");
        assert_eq!(
            jobs[0].schedule_display.as_deref(),
            Some("Daily at 9:00 AM")
        );
        assert_eq!(jobs[0].next_run_at.as_deref(), Some("2026-11-17T09:00:00Z"));
        let duplicate = load_cron_jobs_http(&endpoint, Some("work"));
        assert!(matches!(duplicate, Err(error) if error.contains("duplicate scheduled-job")));
        server.join().expect("cron fixture exits");
    }

    #[test]
    fn cron_pause_resume_actions_use_profiled_empty_posts_and_validate_receipts() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cron action fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            for (action, state, enabled) in
                [("pause", "paused", false), ("resume", "scheduled", true)]
            {
                let (mut stream, _) = listener.accept().expect("accept cron action");
                let request = read_http_fixture_request(&mut stream);
                let lower = request.to_ascii_lowercase();
                assert!(lower.starts_with(&format!(
                    "post /hermes/api/cron/jobs/job%2falpha/{action}?profile=work http/1.1\r\n"
                )));
                assert!(lower.contains("x-hermes-session-token: cron-fixture-token\r\n"));
                assert!(request.split_once("\r\n\r\n").unwrap().1.is_empty());
                let body = json!({
                    "id":"job/alpha","name":"fixture","enabled":enabled,"state":state
                })
                .to_string();
                write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                ).expect("write cron action receipt");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "cron-fixture-token".into(),
            ))),
        };
        let paused = set_cron_job_paused_http(&endpoint, Some("work"), "job/alpha", true)
            .expect("pause receipt matches the requested job");
        assert!(!paused.enabled);
        assert_eq!(paused.state.as_deref(), Some("paused"));
        let resumed = set_cron_job_paused_http(&endpoint, Some("work"), "job/alpha", false)
            .expect("resume receipt matches the requested job");
        assert!(resumed.enabled);
        assert_eq!(resumed.state.as_deref(), Some("scheduled"));
        server.join().expect("cron actions fixture exits");
    }

    #[test]
    fn cron_trigger_isolated_from_http_worker_and_cancel_settles_independently() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cron trigger fixture");
        let address = listener.local_addr().expect("cron trigger fixture address");
        let (trigger_seen_tx, trigger_seen_rx) = mpsc::channel();
        let (finish_trigger_tx, finish_trigger_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut trigger_stream, _) = listener.accept().expect("accept trigger request");
            let trigger_request = read_http_fixture_request(&mut trigger_stream);
            let lower = trigger_request.to_ascii_lowercase();
            assert!(lower.starts_with(
                "post /hermes/api/cron/jobs/job%2falpha/trigger?profile=work http/1.1\r\n"
            ));
            assert!(lower.contains("x-hermes-session-token: cron-fixture-token\r\n"));
            assert!(trigger_request.split_once("\r\n\r\n").unwrap().1.is_empty());
            trigger_seen_tx.send(()).expect("notify trigger in flight");

            let (mut cancel_stream, _) = listener.accept().expect("accept cancel request");
            let cancel_request = read_http_fixture_request(&mut cancel_stream);
            let lower = cancel_request.to_ascii_lowercase();
            assert!(lower.starts_with(
                "post /hermes/api/cron/jobs/job%2falpha/cancel?profile=work http/1.1\r\n"
            ));
            assert!(lower.contains("x-hermes-session-token: cron-fixture-token\r\n"));
            assert!(cancel_request.split_once("\r\n\r\n").unwrap().1.is_empty());
            let cancel_body = r#"{"job_id":"job/alpha","cancelled":true,"execution_id":"exec-1"}"#;
            write!(
                cancel_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                cancel_body.len(),
                cancel_body
            )
            .expect("write cancel response");

            finish_trigger_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("test releases trigger response");
            let trigger_body = r#"{"id":"job/alpha","enabled":true,"state":"scheduled","latest_execution":{"id":"exec-1","status":"cancelled","claimed_at":"2026-11-16T22:00:00Z"}}"#;
            write!(
                trigger_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                trigger_body.len(),
                trigger_body
            )
            .expect("write trigger response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "cron-fixture-token".into(),
            ))),
        };
        let (events_tx, events_rx) = mpsc::channel();
        let (trigger_tx, trigger_rx) = mpsc::sync_channel(8);
        let (http_tx, http_rx) = mpsc::channel();
        let trigger_endpoint = endpoint.clone();
        let trigger_events = events_tx.clone();
        let trigger_actor = thread::spawn(move || {
            gateway_cron_trigger_actor(trigger_endpoint, trigger_rx, trigger_events)
        });
        let http_actor = thread::spawn(move || gateway_http_actor(endpoint, http_rx, events_tx));

        trigger_tx
            .send(CronTriggerCommand {
                request_id: "trigger-request".into(),
                profile: Some("work".into()),
                job_id: "job/alpha".into(),
            })
            .expect("queue isolated trigger");
        trigger_seen_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("trigger request reaches server");
        http_tx
            .send(GatewayHttpCommand::CancelCronJobRun {
                request_id: "cancel-request".into(),
                profile: Some("work".into()),
                job_id: "job/alpha".into(),
            })
            .expect("queue independent cancel");
        let cancel_event = events_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("cancel finishes while trigger is still waiting");
        assert!(matches!(
            cancel_event,
            GatewayEvent::CronJobRunCancelled {
                request_id,
                result: Ok(CronCancelResult { cancelled: true, .. }),
                ..
            } if request_id == "cancel-request"
        ));
        finish_trigger_tx.send(()).expect("release trigger");
        let trigger_event = events_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("trigger receipt arrives after completion");
        assert!(matches!(
            trigger_event,
            GatewayEvent::CronJobTriggered {
                request_id,
                result: Ok(CronJobInfo { id, .. }),
                ..
            } if request_id == "trigger-request" && id == "job/alpha"
        ));
        drop(trigger_tx);
        drop(http_tx);
        trigger_actor.join().expect("trigger dispatcher exits");
        http_actor.join().expect("HTTP actor exits");
        server.join().expect("cron trigger fixture exits");
    }

    #[test]
    fn cron_run_history_read_is_profile_scoped_bounded_and_rejects_duplicates() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cron runs fixture");
        let address = listener.local_addr().expect("cron runs fixture address");
        let server = thread::spawn(move || {
            for body in [
                r#"{"runs":[{"id":"run-1","title":"Daily report","preview":"Done","source":"cron","started_at":1794614400,"last_active":1794614460}]}"#,
                r#"{"runs":[{"id":"same"},{"id":"same"}]}"#,
            ] {
                let (mut stream, _) = listener.accept().expect("accept cron runs");
                let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
                assert!(request.starts_with(
                    "get /hermes/api/cron/jobs/job%2falpha/runs?limit=20&profile=work http/1.1\r\n"
                ));
                assert!(request.contains("x-hermes-session-token: cron-runs-fixture-token\r\n"));
                assert!(!request.contains("x-hermes-gateway-ticket:"));
                write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                ).expect("write cron runs");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "cron-runs-fixture-token".into(),
            ))),
        };
        let runs = load_cron_job_runs_http(&endpoint, Some("work"), "job/alpha")
            .expect("profile-scoped run history");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].title.as_deref(), Some("Daily report"));
        assert_eq!(runs[0].last_active, Some(1_794_614_460.0));
        assert!(matches!(
            load_cron_job_runs_http(&endpoint, Some("work"), "job/alpha"),
            Err(error) if error.contains("duplicate scheduled-job run")
        ));
        assert!(matches!(
            load_cron_job_runs_http(&endpoint, Some("work"), "bad\nid"),
            Err(error) if error.contains("ID is invalid")
        ));
        server.join().expect("cron runs fixture exits");
    }

    #[test]
    fn remote_pairing_reads_and_mutations_preserve_profile_auth_and_target_ids() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind pairing fixture");
        let address = listener.local_addr().expect("pairing fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept pairing read");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(lower.starts_with("get /hermes/api/pairing?profile=work http/1.1\r\n"));
            assert!(lower.contains("x-hermes-session-token: pairing-fixture-token\r\n"));
            let body = json!({
                "pending":[{"platform":"telegram","request_id":"request-1","user_id":"1001","user_name":"Bee","age_minutes":4.5}],
                "approved":[{"platform":"telegram","user_id":"2002","user_name":"Kay"}]
            }).to_string();
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            ).expect("write pairing snapshot");

            let (mut stream, _) = listener.accept().expect("accept approve request");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(
                lower.starts_with("post /hermes/api/pairing/approve?profile=work http/1.1\r\n")
            );
            assert!(lower.contains("x-hermes-session-token: pairing-fixture-token\r\n"));
            let body: serde_json::Value =
                serde_json::from_str(request.split_once("\r\n\r\n").expect("approve body").1)
                    .expect("parse approve request");
            assert_eq!(body["platform"], "telegram");
            assert_eq!(body["request_id"], "request-1");
            assert_eq!(body["profile"], "work");
            let body = json!({"ok":true,"user":{"platform":"telegram","user_id":"1001","user_name":"Bee"}}).to_string();
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            ).expect("write approve receipt");

            let (mut stream, _) = listener.accept().expect("accept revoke request");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(lower.starts_with("post /hermes/api/pairing/revoke?profile=work http/1.1\r\n"));
            assert!(lower.contains("x-hermes-session-token: pairing-fixture-token\r\n"));
            let body: serde_json::Value =
                serde_json::from_str(request.split_once("\r\n\r\n").expect("revoke body").1)
                    .expect("parse revoke request");
            assert_eq!(body["platform"], "telegram");
            assert_eq!(body["user_id"], "2002");
            assert_eq!(body["profile"], "work");
            let body = r#"{"ok":true}"#;
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            ).expect("write revoke receipt");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "pairing-fixture-token".into(),
            ))),
        };
        let snapshot =
            load_pairing_http(&endpoint, Some("work")).expect("load profile-scoped pairing data");
        assert_eq!(snapshot.pending.len(), 1);
        assert_eq!(snapshot.pending[0].request_id.as_deref(), Some("request-1"));
        assert_eq!(snapshot.approved[0].user_id, "2002");
        let approved = mutate_pairing_http(
            &endpoint,
            Some("work"),
            "telegram",
            "request-1",
            PairingMutationKind::Approve,
        )
        .expect("approve pending pairing request");
        assert!(approved.ok);
        assert_eq!(approved.user.unwrap().user_id, "1001");
        let revoked = mutate_pairing_http(
            &endpoint,
            Some("work"),
            "telegram",
            "2002",
            PairingMutationKind::Revoke,
        )
        .expect("revoke approved user");
        assert!(revoked.ok);
        assert!(matches!(
            mutate_pairing_http(&endpoint, Some("work"), "telegram", "bad\nid", PairingMutationKind::Revoke),
            Err(error) if error.contains("pairing target is invalid")
        ));
        server.join().expect("pairing fixture exits");
    }

    #[test]
    fn remote_artifact_download_preserves_gateway_scope_and_atomically_saves_body() {
        use std::io::Write as _;
        use std::time::{SystemTime, UNIX_EPOCH};

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind artifact download fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept artifact download");
            let request_text = read_http_fixture_request(&mut stream);
            let lower = request_text.to_ascii_lowercase();
            assert!(lower.starts_with("get /hermes/api/fs/download?path=%2fsrv%2freports%2fimage%20one%20%231.png&session_id=session%2fid&profile=work http/1.1\r\n"));
            assert!(lower.contains("x-hermes-session-token: artifact-download-token\r\n"));
            assert!(!lower.contains("x-hermes-gateway-ticket:"));
            let body = b"new-image-body";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Disposition: attachment; filename*=UTF-8''generated%20image.png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write download headers");
            stream.write_all(body).expect("write download body");
        });
        let api_origin = format!("http://{address}/hermes");
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin,
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "artifact-download-token".into(),
            ))),
        };
        let request = ArtifactFileDownloadRequest {
            request_id: "artifact-download-1".into(),
            path: "/srv/reports/image one #1.png".into(),
            session_id: "session/id".into(),
            profile: "work".into(),
        };
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "hermes-artifact-download-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).expect("create isolated download directory");
        let destination = directory.join("saved.png");
        std::fs::write(&destination, b"old-content").expect("seed previous destination");
        let saved = download_remote_artifact_file(&endpoint, &request, |filename| {
            assert_eq!(filename, "generated image.png");
            Some(destination.clone())
        })
        .expect("remote file downloaded");
        assert_eq!(saved.as_deref(), Some(destination.as_path()));
        assert_eq!(
            std::fs::read(&destination).expect("read saved file"),
            b"new-image-body"
        );
        server.join().expect("artifact download fixture exits");
        std::fs::remove_file(&destination).expect("remove downloaded fixture file");
        std::fs::remove_dir(&directory).expect("remove isolated download directory");
    }

    #[test]
    fn remote_artifact_image_preview_uses_scoped_authenticated_read_and_bounds_thumbnail() {
        use std::io::Write as _;

        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_fn(600, 300, |x, y| image::Rgba([x as u8, y as u8, 120, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("encode image fixture");
        use base64::Engine as _;
        let data_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.get_ref())
        );
        let json_body = serde_json::json!({"dataUrl":data_url}).to_string();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind image preview fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept image preview");
            let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
            assert!(request.contains("get /hermes/api/fs/read-data-url?path=%2fsrv%2freports%2fimage%20one.png&session_id=session%2fid&profile=work http/1.1\r\n"));
            assert!(request.contains("x-hermes-session-token: image-preview-token\r\n"));
            assert!(!request.contains("x-hermes-gateway-ticket:"));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(), json_body
            ).expect("write image preview response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "image-preview-token".into(),
            ))),
        };
        let request = ArtifactImagePreviewRequest {
            request_id: "image-preview-1".into(),
            artifact_id: "artifact-1".into(),
            path: "/srv/reports/image one.png".into(),
            session_id: "session/id".into(),
            profile: "work".into(),
        };
        let preview = load_remote_artifact_image_preview(&endpoint, &request)
            .expect("remote image preview decoded");
        let crate::files::FilePreviewKind::Image {
            width,
            height,
            rgba,
        } = preview.kind
        else {
            panic!("image preview must decode to RGBA");
        };
        assert_eq!((width, height), (512, 256));
        assert_eq!(rgba.len(), width * height * 4);
        server.join().expect("image preview fixture exits");
    }

    #[test]
    fn remote_artifact_https_image_uses_profile_scoped_gateway_proxy() {
        use std::io::Write as _;

        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(8, 4, image::Rgba([30, 80, 140, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("encode proxied image fixture");
        use base64::Engine as _;
        let data_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.get_ref())
        );
        let json_body = serde_json::json!({"dataUrl":data_url}).to_string();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind media proxy fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept media proxy request");
            let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
            assert!(request.contains("get /hermes/api/media/proxy?url=https%3a%2f%2fimages.example%2fassets%2fchart.png%3fsig%3da%20b&profile=work http/1.1\r\n"));
            assert!(request.contains("x-hermes-session-token: media-proxy-token\r\n"));
            assert!(
                !request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .contains("media-proxy-token")
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(), json_body
            ).expect("write media proxy response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "media-proxy-token".into(),
            ))),
        };
        let request = ArtifactImagePreviewRequest {
            request_id: "image-proxy-1".into(),
            artifact_id: "artifact-proxy-1".into(),
            path: "https://images.example/assets/chart.png?sig=a b".into(),
            session_id: "session-1".into(),
            profile: "work".into(),
        };
        let preview = load_remote_artifact_image_preview(&endpoint, &request)
            .expect("gateway proxy returned image data");
        let crate::files::FilePreviewKind::Image { width, height, .. } = preview.kind else {
            panic!("proxy response must decode to an image");
        };
        assert_eq!((width, height), (8, 4));
        server.join().expect("media proxy fixture exits");
    }

    #[test]
    fn remote_artifact_image_scope_and_data_url_fail_closed() {
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: "https://gateway.example/hermes".into(),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new("fixture".into()))),
        };
        let mut request = ArtifactImagePreviewRequest {
            request_id: "image-preview-2".into(),
            artifact_id: "artifact-2".into(),
            path: "/tmp/image.png".into(),
            session_id: "session-1".into(),
            profile: "work".into(),
        };
        assert!(artifact_image_preview_url(&endpoint, &request).is_ok());
        request.path = "HTTPS://images.example/image.png".into();
        assert!(
            artifact_image_preview_url(&endpoint, &request)
                .expect("uppercase HTTPS is recognized")
                .contains("/api/media/proxy?")
        );
        request.path = "https://user:secret@images.example/image.png".into();
        assert!(artifact_image_preview_url(&endpoint, &request).is_err());
        request.path = "bad\npath".into();
        assert!(artifact_image_preview_url(&endpoint, &request).is_err());
        assert!(artifact_media_path("file:///tmp/report%20one.png").ends_with("report one.png"));
        assert!(decode_artifact_image_preview(b"not an image", false).is_err());
    }

    #[test]
    fn remote_artifact_download_cancel_closes_without_creating_a_file() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind canceled download fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept canceled download");
            let request_text = read_http_fixture_request(&mut stream).to_ascii_lowercase();
            assert!(request_text.contains("get /hermes/api/fs/download?"));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Disposition: attachment; filename=\"cancel.txt\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .expect("write canceled response headers");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new("fixture".into()))),
        };
        let request = ArtifactFileDownloadRequest {
            request_id: "artifact-download-cancel".into(),
            path: "/tmp/cancel.txt".into(),
            session_id: "session-1".into(),
            profile: "work".into(),
        };
        let mut picker_called = false;
        let result = download_remote_artifact_file(&endpoint, &request, |filename| {
            picker_called = true;
            assert_eq!(filename, "cancel.txt");
            None
        })
        .expect("cancel is a normal download outcome");
        assert!(picker_called);
        assert_eq!(result, None);
        server.join().expect("canceled download fixture exits");
    }

    #[test]
    fn remote_artifact_download_rejects_invalid_scope_and_sanitizes_filenames() {
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: "https://gateway.example/hermes".into(),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new("fixture".into()))),
        };
        let mut request = ArtifactFileDownloadRequest {
            request_id: "artifact-download-2".into(),
            path: "/tmp/report.txt".into(),
            session_id: "session-1".into(),
            profile: "work".into(),
        };
        assert!(artifact_download_url(&endpoint, &request).is_ok());
        request.profile = "bad\nprofile".into();
        assert!(artifact_download_url(&endpoint, &request).is_err());
        assert_eq!(
            artifact_download_filename(
                Some("attachment; filename=\"../report.txt\""),
                "/tmp/fallback.txt"
            ),
            "report.txt"
        );
        assert_eq!(
            artifact_download_filename(None, "file:///srv/reports/a%20b.pdf"),
            "a b.pdf"
        );
    }

    #[test]
    fn remote_http_read_uses_session_token_header_and_never_ticket_header() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind remote API fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept remote request");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(lower.contains("x-hermes-session-token: fixture-session-token\r\n"));
            assert!(!lower.contains("x-hermes-gateway-ticket:"));
            let body = r#"{"connected":true}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write remote fixture response");
        });
        let api_origin = format!("http://{address}/hermes");
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: api_origin.clone(),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "fixture-session-token".into(),
            ))),
        };
        let value: serde_json::Value =
            request_remote_gateway_json(&endpoint, &format!("{api_origin}/api/status"))
                .expect("authenticated remote status response");
        assert_eq!(value["connected"], true);
        server.join().expect("remote API fixture exits");
    }

    #[test]
    fn remote_http_read_uses_oauth_bearer_header() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind remote OAuth fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept OAuth read");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(lower.contains("authorization: bearer fixture-access-token\r\n"));
            assert!(!lower.contains("x-hermes-gateway-ticket:"));
            let body = r#"{"authenticated":true}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write OAuth fixture response");
        });
        let api_origin = format!("http://{address}/hermes");
        let now = unix_now_seconds();
        let endpoint = GatewayEndpoint {
            profile_id: "remote-profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: api_origin.clone(),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::OAuth {
                session: Arc::new(Mutex::new(NativeOAuthSession {
                    access_token: Zeroizing::new("fixture-access-token".into()),
                    refresh_token: Zeroizing::new("fixture-refresh-token".into()),
                    expires_at: now + 3600,
                    provider: "fixture".into(),
                    user_id: "fixture-user".into(),
                })),
                credential_store_directory: std::env::temp_dir(),
            }),
        };
        let value: serde_json::Value =
            request_remote_gateway_json(&endpoint, &format!("{api_origin}/api/status"))
                .expect("OAuth authenticated remote status response");
        assert_eq!(value["authenticated"], true);
        server.join().expect("remote OAuth fixture exits");
    }

    #[test]
    fn remote_session_pin_uses_authenticated_snapshot_then_fenced_patch() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind remote mutation fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut snapshot_stream, _) = listener.accept().expect("accept snapshot read");
            let snapshot_request = read_http_fixture_request(&mut snapshot_stream);
            let snapshot_lower = snapshot_request.to_ascii_lowercase();
            assert!(snapshot_request.starts_with(
                "GET /hermes/api/sessions/session-1/mutation-snapshot?profile=profile-a HTTP/1.1\r\n"
            ));
            assert!(snapshot_lower.contains("x-hermes-session-token: fixture-session-token\r\n"));
            let snapshot_body = r#"{"session_id":"session-1","exists":true,"runtime_revision":42,"runtime_generation":7}"#;
            write!(
                snapshot_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                snapshot_body.len(),
                snapshot_body
            )
            .expect("write mutation snapshot");

            let (mut patch_stream, _) = listener.accept().expect("accept session patch");
            let patch_request = read_http_fixture_request(&mut patch_stream);
            let patch_lower = patch_request.to_ascii_lowercase();
            assert!(patch_request.starts_with("PATCH /hermes/api/sessions/session-1 HTTP/1.1\r\n"));
            assert!(patch_lower.contains("x-hermes-session-token: fixture-session-token\r\n"));
            let body_start = patch_request.find("\r\n\r\n").expect("patch body boundary") + 4;
            let body: serde_json::Value =
                serde_json::from_str(&patch_request[body_start..]).expect("patch body JSON");
            assert_eq!(body["pinned"], true);
            assert_eq!(body["request_id"], "pin-request-1");
            assert_eq!(body["expected_revision"], 42);
            assert_eq!(body["expected_generation"], 7);
            assert_eq!(body["profile"], "profile-a");
            let receipt = r#"{"ok":true}"#;
            write!(
                patch_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                receipt.len(),
                receipt
            )
            .expect("write patch receipt");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "profile-a".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "fixture-session-token".into(),
            ))),
        };
        set_session_pinned_http(
            &endpoint,
            "session-1",
            Some("profile-a"),
            true,
            "pin-request-1",
        )
        .expect("remote pin receipt");
        server.join().expect("remote mutation fixture exits");
    }

    #[test]
    fn plugin_auto_update_settings_use_backend_interval_without_client_range_guessing() {
        let settings = parse_agent_plugin_auto_update_settings(json!({
            "auto_apply": true,
            "check_hours": 8760
        }))
        .expect("backend interval is preserved");
        assert!(settings.auto_apply);
        assert_eq!(settings.check_hours, 8760);
        assert_eq!(
            parse_agent_plugin_auto_update_settings(json!({"auto_apply":false}))
                .expect("source default interval")
                .check_hours,
            24
        );
        assert!(parse_agent_plugin_auto_update_settings(json!({"check_hours":-1})).is_err());
    }

    #[test]
    fn project_tree_session_colors_cover_authoritative_and_preview_rows() {
        let colors = parse_project_session_colors(json!({
            "projects": [
                {
                    "id": "project-a",
                    "color": "hsl(120 68% 58%)",
                    "sessionIds": ["recent-a", "archived-a"],
                    "previewSessions": [{"id":"preview-a"}],
                    "repos": [{"groups":[{"sessions":[{"id":"lane-a"}]}]}]
                },
                {"id":"project-b", "color":null, "sessionIds":["uncolored-a"]}
            ]
        }))
        .expect("valid projects.tree response");
        let colors = colors
            .into_iter()
            .map(|entry| (entry.session_id, entry.color))
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(
            colors.get("recent-a"),
            Some(&Some("hsl(120 68% 58%)".into()))
        );
        assert_eq!(
            colors.get("preview-a"),
            Some(&Some("hsl(120 68% 58%)".into()))
        );
        assert_eq!(colors.get("lane-a"), Some(&Some("hsl(120 68% 58%)".into())));
        assert_eq!(colors.get("uncolored-a"), Some(&None));
        assert!(parse_project_session_colors(json!({"unexpected":[]})).is_err());
    }

    #[test]
    fn project_tree_parser_preserves_backend_membership_and_nested_lanes() {
        let tree = parse_project_tree(json!({
            "active_id":"project-a",
            "scoped_session_ids":["session-a","session-b"],
            "projects":[{
                "id":"project-a",
                "label":"Workspace",
                "path":"C:/work",
                "color":"hsl(120 68% 58%)",
                "sessionCount":2,
                "lastActive":10.0,
                "previewSessions":[{"id":"session-a","title":"Recent"}],
                "sessionIds":["session-a","session-b"],
                "repos":[{
                    "id":"repo-a",
                    "label":"app",
                    "path":"C:/work/app",
                    "sessionCount":2,
                    "groups":[{
                        "id":"main",
                        "label":"main",
                        "path":"C:/work/app",
                        "isMain":true,
                        "isHome":true,
                        "isGit":true,
                        "sessions":[{"id":"session-a","title":"Recent"}]
                    }]
                }]
            }]
        }))
        .expect("valid authoritative project tree");
        assert_eq!(tree.active_id.as_deref(), Some("project-a"));
        assert_eq!(tree.scoped_session_ids, vec!["session-a", "session-b"]);
        assert_eq!(tree.projects[0].repos[0].groups[0].is_home, true);
        assert_eq!(
            tree.projects[0].repos[0].groups[0].sessions[0].id,
            "session-a"
        );
        assert!(
            parse_project_tree(
                json!({"projects":[{"id":"duplicate","label":"A"},{"id":"duplicate","label":"B"}]})
            )
            .is_err()
        );
        assert!(parse_project_tree(json!({"projects":[{"id":"bad\nproject"}]})).is_err());
    }

    #[test]
    fn project_list_parser_preserves_source_metadata_and_rejects_invalid_active_ids() {
        let projects = parse_project_list(json!({
            "active_id":"p_demo",
            "projects":[{"id":"p_demo","slug":"demo","name":"Demo",
                "description":"A project","icon":"folder","color":"#7357c5",
                "board_slug":"demo-board","primary_path":"C:/work/demo","archived":false,
                "created_at":12.5,"folders":[{"path":"C:/work/demo","label":"Root",
                    "is_primary":true,"added_at":2.0}]}]
        }))
        .expect("valid projects.list snapshot");
        assert_eq!(projects.active_id.as_deref(), Some("p_demo"));
        assert_eq!(projects.projects.len(), 1);
        assert_eq!(projects.projects[0].name, "Demo");
        assert_eq!(projects.projects[0].folders[0].path, "C:/work/demo");
        assert!(parse_project_list(json!({"projects":[],"active_id":"missing"})).is_err());
        assert!(
            parse_project_list(json!({"projects":[{"id":"duplicate"},{"id":"duplicate"}]}))
                .is_err()
        );
        assert!(parse_project_list(json!({"projects":[{"id":"bad\nproject"}]})).is_err());
        assert!(
            parse_project_list(json!({"projects":[{"id":"p","folders":[{"path":""}]}]})).is_err()
        );
    }

    #[test]
    fn bot_roster_parser_preserves_source_fields_and_rejects_bad_rows() {
        let roster = parse_bot_roster(json!({
            "bot_mode_protocol":true,
            "profiles":[{"name":"researcher","display_name":"Researcher",
                "description":"Research assistant","handle":"@researcher","title":"Analyst"}]
        }))
        .expect("valid profiles.list snapshot");
        assert_eq!(roster.bot_mode_protocol, Some(true));
        assert_eq!(roster.profiles.len(), 1);
        assert_eq!(roster.profiles[0].name, "researcher");
        assert_eq!(
            roster.profiles[0].display_name.as_deref(),
            Some("Researcher")
        );
        assert!(parse_bot_roster(json!({"profiles":[]})).is_ok());
        assert!(parse_bot_roster(json!({"profiles":[{"name":"bad\nname"}]})).is_err());
        assert!(
            parse_bot_roster(json!({"profiles":[{"name":"duplicate"},{"name":"duplicate"}]}))
                .is_err()
        );
        assert!(
            parse_bot_roster(json!({"profiles":[{"name":"valid","title":"bad\nfield"}]})).is_err()
        );
        assert!(parse_bot_roster(json!({"profiles":[{}]})).is_err());
    }

    #[test]
    fn bot_roster_read_uses_profiles_list_with_explicit_profile_scope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind profiles.list fixture");
        let address = listener
            .local_addr()
            .expect("profiles.list fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept profiles.list client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value =
                match socket.read().expect("read profiles.list request") {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                    other => panic!("expected profiles.list request, got {other:?}"),
                };
            assert_eq!(request["method"], "profiles.list");
            assert_eq!(request["params"], json!({"profile":"fixture-profile"}));
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket.send(Message::Text(json!({"jsonrpc":"2.0","id":id,"result":{
                "bot_mode_protocol":true,
                "profiles":[{"name":"researcher","display_name":"Researcher","title":"Analyst"}]
            }}).to_string().into())).expect("send profiles.list response");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadBotRoster {
                request_id: "bot-roster-1".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue profiles.list read");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("profiles.list result"),
            GatewayEvent::BotRosterLoaded { request_id, profile, result: Ok(snapshot) }
                if request_id == "bot-roster-1" && profile == "fixture-profile"
                    && snapshot.bot_mode_protocol == Some(true)
                    && snapshot.profiles[0].name == "researcher")
        );
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn project_list_read_uses_pinned_rpc_and_explicit_profile_scope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind projects.list fixture");
        let address = listener
            .local_addr()
            .expect("projects.list fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept projects.list client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value =
                match socket.read().expect("read projects.list request") {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                    other => panic!("expected projects.list request, got {other:?}"),
                };
            assert_eq!(request["method"], "projects.list");
            assert_eq!(request["params"], json!({"profile":"fixture-profile"}));
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{
                        "active_id":"p_demo",
                        "projects":[{"id":"p_demo","slug":"demo","name":"Demo",
                            "description":null,"icon":null,"color":null,"board_slug":null,
                            "primary_path":"C:/work/demo","archived":false,"created_at":1.0,
                            "folders":[]}]
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send projects.list response");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadProjectList {
                request_id: "project-list-1".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue projects.list read");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("projects.list result"),
            GatewayEvent::ProjectListLoaded {
                request_id,
                profile,
                result: Ok(snapshot),
            } if request_id == "project-list-1"
                && profile == "fixture-profile"
                && snapshot.active_id.as_deref() == Some("p_demo")
                && snapshot.projects[0].name == "Demo"
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn project_delete_uses_explicit_profile_rpc_and_validates_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind projects.delete fixture");
        let address = listener
            .local_addr()
            .expect("projects.delete fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept projects.delete client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value =
                match socket.read().expect("read projects.delete request") {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                    other => panic!("expected projects.delete request, got {other:?}"),
                };
            assert_eq!(request["method"], "projects.delete");
            assert_eq!(
                request["params"],
                json!({"id":"p_demo","profile":"fixture-profile"})
            );
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{
                        "active_id":null,
                        "projects":[{"id":"p_other","name":"Other"}]
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send projects.delete response");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture WebSocket");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::DeleteProject {
                request_id: "project-delete-1".into(),
                project_id: "p_demo".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue projects.delete");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("projects.delete receipt"),
            GatewayEvent::ProjectDeleted { request_id, project_id, profile, result: Ok(snapshot) }
                if request_id == "project-delete-1"
                    && project_id == "p_demo"
                    && profile == "fixture-profile"
                    && snapshot.active_id.is_none()
                    && snapshot.projects.len() == 1
                    && snapshot.projects[0].id == "p_other"
        ));
        drop(command_tx);
        actor
            .join()
            .expect("gateway actor exits after fixture close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn project_delete_rejects_synthetic_all_profile_and_invalid_ids() {
        let (gateway, requests) = GatewayDiscovery::for_test_project_delete();
        assert!(
            gateway
                .delete_project("all-1".into(), "p_demo".into(), "all".into())
                .is_err()
        );
        assert!(
            gateway
                .delete_project("invalid-1".into(), "bad\nid".into(), "profile-a".into())
                .is_err()
        );
        assert!(
            requests.try_recv().is_err(),
            "invalid mutations must not reach the command queue"
        );
    }

    #[test]
    fn active_session_status_parser_requires_bounded_unique_profile_rows() {
        let statuses = parse_active_session_statuses(json!({"sessions":[
            {"id":"runtime-1","session_key":"stored-1","status":"working","last_active":1.5},
            {"id":"runtime-2","session_key":"stored-2","status":"waiting","last_active":2.0},
            {"id":"runtime-3","session_key":"stored-3","status":"starting"}
        ]}))
        .expect("valid active-list snapshot");
        assert_eq!(statuses.len(), 3);
        assert_eq!(statuses[0].status, ActiveSessionStatusKind::Working);
        assert_eq!(statuses[1].status, ActiveSessionStatusKind::Waiting);
        assert_eq!(statuses[2].status, ActiveSessionStatusKind::Starting);
        assert_eq!(statuses[0].last_active, Some(1.5));

        for invalid in [
            json!({}),
            json!({"sessions":[{"id":"","session_key":"stored","status":"working"}]}),
            json!({"sessions":[{"id":"runtime","session_key":"stored","status":"future"}]}),
            json!({"sessions":[
                {"id":"runtime","session_key":"stored-1","status":"working"},
                {"id":"runtime","session_key":"stored-2","status":"idle"}
            ]}),
        ] {
            assert!(parse_active_session_statuses(invalid).is_err());
        }
    }

    #[test]
    fn active_session_status_read_uses_pinned_rpc_and_explicit_profile_scope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind active-list fixture");
        let address = listener.local_addr().expect("active-list fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept active-list client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read active-list request")
            {
                Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                other => panic!("expected active-list request, got {other:?}"),
            };
            assert_eq!(request["method"], "session.active_list");
            assert_eq!(request["params"], json!({"profile":"fixture-profile"}));
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"sessions":[
                        {"id":"runtime-1","session_key":"stored-1","status":"waiting","last_active":2.0}
                    ]}})
                    .to_string()
                    .into(),
                ))
                .expect("send active-list response");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadActiveSessionStatuses {
                request_id: "active-status-1".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue active-list read");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("active-list result"),
            GatewayEvent::ActiveSessionStatusesLoaded {
                request_id,
                profile,
                result: Ok(statuses),
            } if request_id == "active-status-1"
                && profile == "fixture-profile"
                && statuses == vec![ActiveSessionStatus {
                    runtime_session_id: "runtime-1".into(),
                    stored_session_id: "stored-1".into(),
                    status: ActiveSessionStatusKind::Waiting,
                    last_active: Some(2.0),
                }]
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn project_tree_color_read_uses_the_requested_profile_on_the_primary_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read tree request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                other => panic!("expected tree request, got {other:?}"),
            };
            assert_eq!(request["method"], "projects.tree");
            assert_eq!(request["params"], json!({"profile":"fixture-profile"}));
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"projects":[{
                        "id":"project-a","color":"hsl(120 68% 58%)","sessionIds":["session-a"]
                    }]}})
                    .to_string()
                    .into(),
                ))
                .expect("send project tree result");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadProjectSessionColors {
                request_id: "project-colors-1".into(),
                profile: Some("fixture-profile".into()),
            })
            .expect("queue project colors");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("project color result"),
            GatewayEvent::ProjectSessionColorsLoaded {
                request_id,
                profile: Some(profile),
                result: Ok(colors),
            } if request_id == "project-colors-1"
                && profile == "fixture-profile"
                && colors == vec![ProjectSessionColor {
                    session_id: "session-a".into(),
                    color: Some("hsl(120 68% 58%)".into()),
                }]
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn project_tree_read_uses_requested_profile_and_preview_limit() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read tree request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                other => panic!("expected tree request, got {other:?}"),
            };
            assert_eq!(request["method"], "projects.tree");
            assert_eq!(
                request["params"],
                json!({"profile":"fixture-profile","preview_limit":3})
            );
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{
                        "active_id":"project-a",
                        "scoped_session_ids":["session-a"],
                        "projects":[{"id":"project-a","label":"Fixture",
                            "sessionIds":["session-a"],"repos":[{"id":"repo-a",
                                "label":"Repository","groups":[{"id":"main",
                                    "label":"Main","isMain":true,"sessions":[]}]}]}]
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send project tree result");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadProjectTree {
                request_id: "project-tree-1".into(),
                profile: "fixture-profile".into(),
                preview_limit: 3,
            })
            .expect("queue project tree");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("project tree result"),
            GatewayEvent::ProjectTreeLoaded {
                request_id,
                profile,
                result: Ok(tree),
            } if request_id == "project-tree-1"
                && profile == "fixture-profile"
                && tree.active_id.as_deref() == Some("project-a")
                && tree.projects[0].repos[0].groups[0].is_main
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn all_profile_project_tree_uses_profile_aggregate_and_preserves_owners_and_errors() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind project tree fixture");
        let address = listener.local_addr().expect("project tree fixture address");
        let body = json!({
            "active_id": null,
            "scoped_session_ids": ["session-default", "session-work"],
            "errors": [{"profile":"broken","error":"403 denied"}],
            "projects": [{
                "id":"project-repo","mergeKey":"C:/work/repo","label":"Repo",
                "isDefaultProfile":true,"sessionCount":2,
                "previewSessions":[
                    {"id":"session-default","profile":"default","title":"Default"},
                    {"id":"session-work","profile":"work","title":"Work"}
                ],
                "repos":[{"id":"repo","label":"Repo","sessionCount":2,"groups":[
                    {"id":"main","label":"main","isMain":true,"sessions":[
                        {"id":"session-work","profile":"work","title":"Work"}
                    ]}
                ]}]
            }]
        })
        .to_string();
        let expected_body = body.clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept project tree read");
            let request = read_http_fixture_request(&mut stream);
            let lower = request.to_ascii_lowercase();
            assert!(lower.starts_with(
                "get /hermes/api/profiles/projects/tree?preview_limit=2000 http/1.1\r\n"
            ));
            assert!(lower.contains("x-hermes-session-token: all-tree-token\r\n"));
            assert!(!lower.contains("x-hermes-gateway-ticket:"));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                expected_body.len(), expected_body
            )
            .expect("write all-profile project tree");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "all".into(),
            instance_id: "aggregate-gateway".into(),
            authority_epoch: 2,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "all-tree-token".into(),
            ))),
        };
        let tree = load_all_profile_project_tree_http(&endpoint, 2000)
            .expect("aggregate tree response is accepted");
        assert_eq!(tree.active_id, None);
        assert_eq!(tree.errors.len(), 1);
        assert_eq!(tree.errors[0].profile, "broken");
        assert_eq!(tree.projects[0].merge_key.as_deref(), Some("C:/work/repo"));
        assert_eq!(
            tree.projects[0].preview_sessions[0].profile.as_deref(),
            Some("default")
        );
        assert_eq!(
            tree.projects[0].repos[0].groups[0].sessions[0]
                .profile
                .as_deref(),
            Some("work")
        );
        server.join().expect("project tree fixture exits");

        let missing_owner = json!({
            "projects":[{"id":"project","previewSessions":[{"id":"session"}]}]
        });
        let tree = parse_project_tree(missing_owner).expect("basic tree shape is valid");
        assert!(validate_all_profile_project_tree(&tree).is_err());
    }

    #[test]
    fn project_sessions_read_uses_exact_project_and_profile_scope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind project sessions fixture");
        let address = listener.local_addr().expect("project sessions address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept project sessions client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read project sessions") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse project request"),
                other => panic!("expected project session request, got {other:?}"),
            };
            assert_eq!(request["method"], "projects.project_sessions");
            assert_eq!(
                request["params"],
                json!({"project_id":"project-a","profile":"fixture-profile"})
            );
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"project":{
                        "id":"project-a","label":"Project A","sessionCount":1,
                        "repos":[{"id":"repo-a","label":"Repository","sessionCount":1,
                            "groups":[{"id":"main","label":"main","sessions":[{
                                "id":"session-a","profile":"fixture-profile","title":"Fixture"
                            }]}]}]
                    }}})
                    .to_string()
                    .into(),
                ))
                .expect("send project session result");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadProjectSessions {
                request_id: "project-sessions-1".into(),
                project_id: "project-a".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue project sessions");
        assert!(matches!(
            event_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("project sessions result"),
            GatewayEvent::ProjectSessionsLoaded {
                request_id,
                project_id,
                profile,
                result: Ok(Some(project)),
            } if request_id == "project-sessions-1"
                && project_id == "project-a"
                && profile == "fixture-profile"
                && project.repos[0].groups[0].sessions[0].id == "session-a"
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("project sessions fixture exits");
    }
    use serde_json::json;
    use std::net::TcpListener;

    fn cron_remote_fixture_endpoint(address: std::net::SocketAddr) -> GatewayEndpoint {
        GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "cron-fixture-token".into(),
            ))),
        }
    }

    #[test]
    fn cron_crud_and_delivery_targets_use_profile_scoped_authenticated_contracts() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cron CRUD fixture");
        let address = listener.local_addr().expect("cron CRUD fixture address");
        let server = thread::spawn(move || {
            let replies = [
                (
                    "GET /hermes/api/cron/delivery-targets?profile=work",
                    r#"{"targets":[{"id":"local","name":"Local (save only)","home_target_set":true,"home_env_var":null},{"id":"telegram","name":"Telegram","home_target_set":true,"home_env_var":"TELEGRAM_HOME"}]}"#,
                ),
                (
                    "POST /hermes/api/cron/jobs?profile=work",
                    r#"{"id":"daily-summary","name":"Daily summary","prompt":"Summarize activity","enabled":true,"state":"scheduled","schedule":{"expr":"0 9 * * *"},"deliver":"local","catch_up":false}"#,
                ),
                (
                    "PUT /hermes/api/cron/jobs/daily%2Fsummary?profile=work",
                    r#"{"id":"daily/summary","name":"Daily summary","prompt":"Summarize activity","enabled":true,"state":"scheduled","schedule":{"expr":"0 10 * * *"},"deliver":"telegram","no_agent":true,"script":"echo done"}"#,
                ),
                (
                    "DELETE /hermes/api/cron/jobs/daily%2Fsummary?profile=work",
                    r#"{"ok":true}"#,
                ),
            ];
            for (index, (route, body)) in replies.into_iter().enumerate() {
                let (mut stream, _) = listener.accept().expect("accept cron CRUD request");
                let request = read_http_fixture_request(&mut stream);
                let lower = request.to_ascii_lowercase();
                assert!(lower.starts_with(&format!("{} http/1.1\r\n", route.to_ascii_lowercase())));
                assert!(lower.contains("x-hermes-session-token: cron-fixture-token\r\n"));
                assert!(!lower.contains("x-hermes-gateway-ticket:"));
                let request_body = request.split_once("\r\n\r\n").unwrap().1;
                match index {
                    0 => assert!(request_body.is_empty()),
                    1 => {
                        let value: serde_json::Value = serde_json::from_str(request_body).unwrap();
                        assert_eq!(value["prompt"], "Summarize activity");
                        assert_eq!(value["schedule"], "0 9 * * *");
                        assert_eq!(value["deliver"], "local");
                        assert_eq!(value["catch_up"], false);
                        assert!(value.get("name").is_none());
                        assert!(value.get("model").is_none());
                        assert!(value.get("provider").is_none());
                    }
                    2 => {
                        let value: serde_json::Value = serde_json::from_str(request_body).unwrap();
                        assert_eq!(value["updates"]["schedule"], "0 10 * * *");
                        assert_eq!(value["updates"]["deliver"], "telegram");
                        assert!(value["updates"]["model"].is_null());
                        assert!(value["updates"]["provider"].is_null());
                        assert!(value["updates"]["catch_up"].is_null());
                        assert!(value["updates"].get("prompt").is_none());
                        assert!(value["updates"].get("name").is_none());
                    }
                    3 => assert!(request_body.is_empty()),
                    _ => unreachable!(),
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write cron CRUD response");
            }
        });
        let endpoint = cron_remote_fixture_endpoint(address);
        let targets = load_cron_delivery_targets_http(&endpoint, Some("work"))
            .expect("profile-scoped delivery targets");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[1].id, "telegram");
        assert_eq!(targets[1].home_env_var.as_deref(), Some("TELEGRAM_HOME"));

        let created = create_cron_job_http(
            &endpoint,
            Some("work"),
            &CronJobCreatePayload {
                prompt: "Summarize activity".into(),
                schedule: "0 9 * * *".into(),
                name: None,
                deliver: Some("local".into()),
                catch_up: Some(false),
                model: None,
                provider: None,
            },
        )
        .expect("create receipt");
        assert_eq!(created.id, "daily-summary");
        assert_eq!(created.catch_up, Some(false));

        let updated = update_cron_job_http(
            &endpoint,
            Some("work"),
            "daily/summary",
            &CronJobUpdatePayload {
                schedule: Some("0 10 * * *".into()),
                deliver: Some("telegram".into()),
                catch_up: Some(None),
                model: Some(None),
                provider: Some(None),
                ..CronJobUpdatePayload::default()
            },
        )
        .expect("update receipt");
        assert_eq!(updated.id, "daily/summary");
        assert_eq!(
            updated
                .schedule
                .as_ref()
                .and_then(|row| row.expr.as_deref()),
            Some("0 10 * * *")
        );
        assert!(updated.no_agent);
        assert_eq!(updated.script.as_deref(), Some("echo done"));

        let deleted =
            delete_cron_job_http(&endpoint, Some("work"), "daily/summary").expect("delete receipt");
        assert!(deleted.ok);
        server.join().expect("cron CRUD fixture exits");
    }

    #[test]
    fn model_options_and_cron_blueprints_use_pinned_profile_scoped_api_contracts() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind model and blueprint fixture");
        let address = listener
            .local_addr()
            .expect("model and blueprint fixture address");
        let server = thread::spawn(move || {
            let replies = [
                (
                    "GET /hermes/api/model/options?explicit_only=1&profile=work",
                    r#"{"providers":[{"slug":"openai","name":"OpenAI","authenticated":true,"models":["gpt-fixture"]},{"slug":"retired","name":"Retired","authenticated":false,"models":[]}],"provider":"openai","model":"gpt-fixture"}"#,
                ),
                (
                    "GET /hermes/api/cron/blueprints?profile=work",
                    r#"{"blueprints":[{"key":"teamtools:standup","title":"Team standup","description":"Weekday summary","category":"work","tags":["work"],"fields":[{"name":"time","type":"time","label":"What time?","default":"09:15","options":[],"optional":false,"strict":true,"help":"24h time"}],"command":"/blueprint teamtools:standup","appUrl":"hermes://blueprint/teamtools%3Astandup","source":"plugin","plugin":"teamtools","schedule":"{minute} {hour} * * 1-5","scheduleHuman":"weekdays at 09:15"}]}"#,
                ),
                (
                    "POST /hermes/api/cron/blueprints/instantiate?profile=work",
                    r#"{"id":"weekday-standup","name":"Team standup","prompt":"Summarize weekdays","enabled":true,"state":"scheduled","schedule":{"expr":"15 9 * * 1-5"},"deliver":"local"}"#,
                ),
                (
                    "GET /hermes/api/cron/blueprints?profile=work",
                    r#"{"blueprints":[{"key":"duplicate","title":"One","description":"","category":"","tags":[],"fields":[],"command":"/blueprint duplicate","appUrl":"hermes://blueprint/duplicate"},{"key":"duplicate","title":"Two","description":"","category":"","tags":[],"fields":[],"command":"/blueprint duplicate","appUrl":"hermes://blueprint/duplicate"}]}"#,
                ),
            ];
            for (index, (route, body)) in replies.into_iter().enumerate() {
                let (mut stream, _) = listener.accept().expect("accept model/blueprint request");
                let request = read_http_fixture_request(&mut stream);
                let lower = request.to_ascii_lowercase();
                assert!(lower.starts_with(&format!("{} http/1.1\r\n", route.to_ascii_lowercase())));
                assert!(lower.contains("x-hermes-session-token: cron-fixture-token\r\n"));
                assert!(!lower.contains("x-hermes-gateway-ticket:"));
                let request_body = request.split_once("\r\n\r\n").unwrap().1;
                match index {
                    0 | 1 | 3 => assert!(request_body.is_empty()),
                    2 => {
                        let value: serde_json::Value = serde_json::from_str(request_body).unwrap();
                        assert_eq!(value["blueprint"], "teamtools:standup");
                        assert_eq!(value["values"]["time"], "09:15");
                        assert_eq!(value["values"]["deliver"], "local");
                    }
                    _ => unreachable!(),
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write model/blueprint response");
            }
        });
        let endpoint = cron_remote_fixture_endpoint(address);
        let models =
            load_model_options_http(&endpoint, Some("work")).expect("profile-scoped model options");
        assert_eq!(models.providers.len(), 2);
        assert_eq!(models.providers[0].slug, "openai");
        assert_eq!(models.providers[0].models, ["gpt-fixture"]);
        assert_eq!(models.providers[0].authenticated, Some(true));
        assert_eq!(models.providers[1].authenticated, Some(false));

        let blueprints = load_cron_blueprints_http(&endpoint, Some("work"))
            .expect("profile-scoped blueprint catalog");
        assert_eq!(blueprints.len(), 1);
        assert_eq!(blueprints[0].key, "teamtools:standup");
        assert_eq!(
            blueprints[0].app_url,
            "hermes://blueprint/teamtools%3Astandup"
        );
        assert_eq!(blueprints[0].fields[0].field_type, "time");

        let job = instantiate_cron_blueprint_http(
            &endpoint,
            Some("work"),
            &CronBlueprintInstantiatePayload {
                blueprint: "teamtools:standup".into(),
                values: BTreeMap::from([
                    ("time".into(), "09:15".into()),
                    ("deliver".into(), "local".into()),
                ]),
            },
        )
        .expect("instantiate blueprint receipt");
        assert_eq!(job.id, "weekday-standup");
        let duplicate_blueprints = load_cron_blueprints_http(&endpoint, Some("work"));
        assert!(matches!(
            duplicate_blueprints,
            Err(error) if error.contains("duplicate Scheduled Job blueprints")
        ));
        server.join().expect("model and blueprint fixture exits");
    }

    #[test]
    fn model_options_read_uses_oauth_bearer_and_rejects_bad_catalog_rows() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind model OAuth fixture");
        let address = listener.local_addr().expect("model OAuth fixture address");
        let server = thread::spawn(move || {
            for body in [
                r#"{"providers":[{"slug":"anthropic","name":"Anthropic","authenticated":true,"models":["claude-fixture"]}]}"#,
                r#"{"providers":[{"slug":"duplicate","name":"One","models":[]},{"slug":"duplicate","name":"Two","models":[]}]}"#,
            ] {
                let (mut stream, _) = listener.accept().expect("accept model options request");
                let request = read_http_fixture_request(&mut stream).to_ascii_lowercase();
                assert!(request.starts_with(
                    "get /hermes/api/model/options?explicit_only=1&profile=work http/1.1\r\n"
                ));
                assert!(request.contains("authorization: bearer cron-fixture-access\r\n"));
                assert!(!request.contains("x-hermes-session-token:"));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write model options response");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::OAuth {
                session: Arc::new(Mutex::new(NativeOAuthSession {
                    access_token: Zeroizing::new("cron-fixture-access".into()),
                    refresh_token: Zeroizing::new("unused-refresh".into()),
                    expires_at: i64::MAX,
                    provider: "fixture".into(),
                    user_id: "fixture-user".into(),
                })),
                credential_store_directory: PathBuf::from("unused-fixture-profile"),
            }),
        };
        let models =
            load_model_options_http(&endpoint, Some("work")).expect("OAuth model catalog read");
        assert_eq!(models.providers[0].slug, "anthropic");
        let duplicate = load_model_options_http(&endpoint, Some("work"));
        assert!(matches!(duplicate, Err(error) if error.contains("duplicate model providers")));
        server.join().expect("model OAuth fixture exits");
    }

    #[test]
    fn cron_delivery_target_read_accepts_oauth_bearer_and_validates_target_rows() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cron OAuth fixture");
        let address = listener.local_addr().expect("cron OAuth fixture address");
        let server = thread::spawn(move || {
            for (method, path, body) in [
                (
                    "GET",
                    "/hermes/api/cron/delivery-targets?profile=work",
                    r#"{"targets":[{"id":"local","name":"Local","home_target_set":true,"home_env_var":null}]}"#,
                ),
                (
                    "PUT",
                    "/hermes/api/cron/jobs/job-1?profile=work",
                    r#"{"id":"job-1","enabled":true,"state":"scheduled"}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept cron OAuth request");
                let request = read_http_fixture_request(&mut stream);
                let lower = request.to_ascii_lowercase();
                assert!(
                    lower
                        .starts_with(&format!("{method} {path} http/1.1\r\n").to_ascii_lowercase())
                );
                assert!(lower.contains("authorization: bearer cron-fixture-access\r\n"));
                assert!(!lower.contains("x-hermes-session-token:"));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write OAuth cron response");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "default".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::OAuth {
                session: Arc::new(Mutex::new(NativeOAuthSession {
                    access_token: Zeroizing::new("cron-fixture-access".into()),
                    refresh_token: Zeroizing::new("unused-refresh".into()),
                    expires_at: i64::MAX,
                    provider: "fixture".into(),
                    user_id: "fixture-user".into(),
                })),
                credential_store_directory: PathBuf::from("unused-fixture-profile"),
            }),
        };
        let targets = load_cron_delivery_targets_http(&endpoint, Some("work"))
            .expect("OAuth profile-scoped delivery target list");
        assert_eq!(targets.len(), 1);
        update_cron_job_http(
            &endpoint,
            Some("work"),
            "job-1",
            &CronJobUpdatePayload {
                schedule: Some("0 9 * * *".into()),
                ..CronJobUpdatePayload::default()
            },
        )
        .expect("OAuth update uses bearer auth");
        server.join().expect("cron OAuth fixture exits");
    }

    #[test]
    fn cron_local_http_helper_uses_gateway_ticket_for_crud_methods() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local cron ticket fixture");
        let address = listener.local_addr().expect("local cron ticket address");
        let server = thread::spawn(move || {
            for (method, path, body) in [
                (
                    "GET",
                    "/api/model/options?explicit_only=1&profile=work",
                    r#"{"providers":[{"slug":"openai","name":"OpenAI","models":["fixture"]}]}"#,
                ),
                ("POST", "/api/cron/jobs?profile=work", "{\"ok\":true}"),
                ("PUT", "/api/cron/jobs/job-1?profile=work", "{\"ok\":true}"),
                (
                    "DELETE",
                    "/api/cron/jobs/job-1?profile=work",
                    "{\"ok\":true}",
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept local cron mutation");
                let request = read_http_fixture_request(&mut stream);
                let lower = request.to_ascii_lowercase();
                assert!(
                    lower
                        .starts_with(&format!("{method} {path} http/1.1\r\n").to_ascii_lowercase())
                );
                assert!(lower.contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n"));
                assert!(!lower.contains("x-hermes-session-token:"));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write local cron response");
            }
        });
        let origin = format!("http://{address}");
        for (method, path, request_body) in [
            (
                "GET",
                "/api/model/options?explicit_only=1&profile=work",
                None,
            ),
            (
                "POST",
                "/api/cron/jobs?profile=work",
                Some(json!({"schedule":"0 9 * * *"})),
            ),
            (
                "PUT",
                "/api/cron/jobs/job-1?profile=work",
                Some(json!({"updates":{"model":null}})),
            ),
            ("DELETE", "/api/cron/jobs/job-1?profile=work", None),
        ] {
            let receipt: serde_json::Value = cron_local_gateway_http_request_with_ticket(
                &origin,
                "fixture-native-http-ticket",
                method,
                &format!("{origin}{path}"),
                request_body,
            )
            .expect("ticket-authenticated request");
            if method == "GET" {
                assert_eq!(receipt["providers"][0]["slug"], "openai");
            } else {
                assert_eq!(receipt["ok"], true);
            }
        }
        server.join().expect("local cron ticket fixture exits");
    }

    #[test]
    fn cron_crud_payloads_preserve_omission_and_reject_invalid_bounds() {
        let update = CronJobUpdatePayload {
            prompt: Some("updated".into()),
            model: Some(None),
            ..CronJobUpdatePayload::default()
        };
        let value = serde_json::to_value(update).expect("serialize update");
        assert_eq!(value, json!({ "prompt": "updated", "model": null }));
        assert!(
            validate_cron_create_payload(&CronJobCreatePayload {
                prompt: "x".repeat(CRON_FORM_PROMPT_LIMIT + 1),
                schedule: "0 9 * * *".into(),
                ..CronJobCreatePayload::default()
            })
            .is_err()
        );
        assert!(validate_cron_update_payload(&CronJobUpdatePayload::default()).is_err());
        assert!(
            validate_cron_update_payload(&CronJobUpdatePayload {
                schedule: Some("  ".into()),
                ..CronJobUpdatePayload::default()
            })
            .is_err()
        );
        assert!(
            validate_cron_job_receipt(
                CronJobInfo {
                    id: "different-job".into(),
                    catch_up: None,
                    no_agent: false,
                    script: None,
                    name: None,
                    prompt: None,
                    enabled: true,
                    state: Some("scheduled".into()),
                    schedule_display: None,
                    schedule: None,
                    deliver: None,
                    model: None,
                    provider: None,
                    next_run_at: None,
                    last_run_at: None,
                    last_error: None,
                    latest_execution: None,
                    live_execution: None,
                },
                Some("expected-job")
            )
            .is_err()
        );
    }

    fn read_http_fixture_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let count = stream.read(&mut buffer).expect("read HTTP fixture request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let header_text = String::from_utf8_lossy(&request[..header_end]);
            let content_length = header_text
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        String::from_utf8(request).expect("HTTP fixture request is UTF-8")
    }

    #[test]
    fn telegram_qr_onboarding_uses_profile_scoped_authenticated_api_contract() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind Telegram fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let replies = [
                json!({
                    "deep_link":"https://t.me/BotFather?start=fixture",
                    "expires_at":"2099-01-01T00:00:00Z",
                    "pairing_id":"pair/one",
                    "qr_payload":"https://t.me/login?token=opaque",
                    "suggested_username":"HermesBot"
                })
                .to_string(),
                json!({
                    "status":"ready",
                    "bot_username":"HermesBot",
                    "expires_at":"2099-01-01T00:00:00Z",
                    "owner_user_id":"123456789"
                })
                .to_string(),
                json!({
                    "ok":true,
                    "platform":"telegram",
                    "needs_restart":true,
                    "bot_username":"HermesBot",
                    "restart_started":false
                })
                .to_string(),
                json!({"ok":true}).to_string(),
            ];
            let expected = [
                "POST /hermes/api/messaging/telegram/onboarding/start?profile=work%20profile HTTP/1.1",
                "GET /hermes/api/messaging/telegram/onboarding/pair%2Fone?profile=work%20profile HTTP/1.1",
                "POST /hermes/api/messaging/telegram/onboarding/pair%2Fone/apply?profile=work%20profile HTTP/1.1",
                "DELETE /hermes/api/messaging/telegram/onboarding/pair%2Fone?profile=work%20profile HTTP/1.1",
            ];
            for (index, body) in replies.iter().enumerate() {
                let (mut stream, _) = listener.accept().expect("accept Telegram request");
                let request = read_http_fixture_request(&mut stream);
                assert!(request.starts_with(expected[index]), "{request}");
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("x-hermes-session-token: telegram-fixture-token\r\n")
                );
                if index == 2 {
                    let request_body = request.split_once("\r\n\r\n").unwrap().1;
                    let value: serde_json::Value =
                        serde_json::from_str(request_body).expect("apply body JSON");
                    assert_eq!(value["profile"], "work profile");
                    assert_eq!(value["allowed_user_ids"], json!(["123456789", "987654321"]));
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write Telegram fixture response");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "telegram-fixture-token".into(),
            ))),
        };
        let start = start_telegram_onboarding_http(&endpoint, Some("work profile"), None)
            .expect("Telegram setup starts");
        assert_eq!(start.pairing_id, "pair/one");
        assert_eq!(start.qr_payload, "https://t.me/login?token=opaque");
        let status = load_telegram_onboarding_status_http(
            &endpoint,
            &start.pairing_id,
            Some("work profile"),
        )
        .expect("Telegram status loads");
        assert!(matches!(status, TelegramOnboardingStatus::Ready { .. }));
        let receipt = apply_telegram_onboarding_http(
            &endpoint,
            &start.pairing_id,
            Some("work profile"),
            &["123456789".into(), "987654321".into()],
        )
        .expect("Telegram bot applies");
        assert!(receipt.needs_restart);
        assert!(
            cancel_telegram_onboarding_http(&endpoint, &start.pairing_id, Some("work profile"))
                .expect("Telegram setup cancels")
                .ok
        );
        server.join().expect("Telegram fixture exits");
    }

    #[test]
    fn telegram_setup_rejects_unsafe_links_and_invalid_allowed_user_ids() {
        let result = validate_telegram_onboarding_start(TelegramOnboardingStart {
            deep_link: "https://example.invalid/credential".into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
            pairing_id: "pair-1".into(),
            qr_payload: "opaque".into(),
            suggested_username: "HermesBot".into(),
        });
        assert!(result.is_err());
        assert!(!valid_telegram_user_id("12"));
        assert!(!valid_telegram_user_id("123a"));
        assert!(valid_telegram_user_id("123456789"));
    }

    #[test]
    fn gateway_restart_and_action_status_are_profile_scoped_and_authenticated() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind gateway restart fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut restart_stream, _) = listener.accept().expect("accept restart request");
            let request = read_http_fixture_request(&mut restart_stream);
            assert!(request.starts_with(
                "POST /hermes/api/gateway/restart?profile=work%20profile HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: restart-fixture-token\r\n")
            );
            assert_eq!(request.split_once("\r\n\r\n").unwrap().1, "{}");
            let body = json!({
                "name":"restart-fixture",
                "ok":true,
                "pid":1234,
                "action_id":"restart-fixture"
            })
            .to_string();
            write!(
                restart_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write restart fixture response");

            let (mut status_stream, _) = listener.accept().expect("accept action status request");
            let request = read_http_fixture_request(&mut status_stream);
            assert!(request.starts_with(
                "GET /hermes/api/actions/restart-fixture/status?lines=200&profile=work%20profile HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: restart-fixture-token\r\n")
            );
            let body = json!({
                "name":"restart-fixture",
                "running":false,
                "exit_code":0,
                "pid":1234,
                "lines":["restart complete"]
            })
            .to_string();
            write!(
                status_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write action status fixture response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "remote-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: vec![],
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "restart-fixture-token".into(),
            ))),
        };
        let receipt = restart_gateway_http(&endpoint, Some("work profile"))
            .expect("gateway restart accepted");
        assert_eq!(receipt.name, "restart-fixture");
        let status =
            load_gateway_action_status_http(&endpoint, &receipt.name, Some("work profile"))
                .expect("gateway action status loads");
        assert!(!status.running);
        assert_eq!(status.exit_code, Some(0));
        assert!(validate_action_name("bad\naction").is_err());
        server.join().expect("gateway restart fixture exits");
    }

    #[test]
    fn command_center_system_loads_profile_scoped_status_and_filtered_logs() {
        use std::io::Write as _;

        let listener =
            TcpListener::bind("127.0.0.1:0").expect("bind Command Center system fixture");
        let address = listener
            .local_addr()
            .expect("Command Center fixture address");
        let server = thread::spawn(move || {
            let (mut status_stream, _) = listener.accept().expect("accept status read");
            let status_request = read_http_fixture_request(&mut status_stream);
            assert!(
                status_request
                    .starts_with("GET /hermes/api/status?profile=work%20profile HTTP/1.1\r\n")
            );
            assert!(
                status_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: command-center-fixture-token\r\n")
            );
            let status = json!({
                "active_sessions": 3,
                "gateway_running": true,
                "version": "2026.11.18",
                "gateway_heartbeat_stale_s": null,
                "gateway_exit_reason": null
            })
            .to_string();
            write!(
                status_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status.len(), status
            )
            .expect("write status response");

            let (mut logs_stream, _) = listener.accept().expect("accept logs read");
            let logs_request = read_http_fixture_request(&mut logs_stream);
            assert!(logs_request.starts_with(
                "GET /hermes/api/logs?file=agent&lines=200&level=ERROR&profile=work%20profile HTTP/1.1\r\n"
            ));
            assert!(
                logs_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: command-center-fixture-token\r\n")
            );
            let logs = json!({"file":"agent","lines":["ERROR fixture failure"]}).to_string();
            write!(
                logs_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                logs.len(), logs
            )
            .expect("write logs response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "command-center-instance".into(),
            authority_epoch: 4,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "command-center-fixture-token".into(),
            ))),
        };
        let snapshot =
            load_command_center_system_http(&endpoint, Some("work profile"), "agent", "ERROR")
                .expect("typed status and logs load through remote auth");
        assert_eq!(snapshot.status.active_sessions, 3);
        assert!(snapshot.status.gateway_running);
        assert_eq!(snapshot.logs.lines, ["ERROR fixture failure"]);
        assert!(
            load_command_center_system_http(&endpoint, Some("work profile"), "../agent", "ALL")
                .is_err()
        );
        server.join().expect("Command Center fixture exits");
    }

    #[test]
    fn command_center_usage_loads_bounded_analytics_for_the_explicit_profile() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind usage fixture");
        let address = listener.local_addr().expect("usage fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept usage analytics read");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with(
                "GET /hermes/api/analytics/usage?days=30&profile=work%20profile HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: usage-fixture-token\r\n")
            );
            let body = json!({
                "by_model":[{"api_calls":3,"estimated_cost":0.04,"input_tokens":120,"model":"model-fixture","output_tokens":80,"sessions":2}],
                "daily":[{"actual_cost":0.02,"api_calls":3,"cache_read_tokens":10,"day":"2026-11-18","estimated_cost":0.04,"input_tokens":120,"output_tokens":80,"reasoning_tokens":5,"sessions":2}],
                "period_days":30,
                "skills":{"summary":{"distinct_skills_used":1,"total_skill_actions":2,"total_skill_edits":0,"total_skill_loads":2},"top_skills":[{"last_used_at":1795000000,"manage_count":0,"percentage":100.0,"skill":"review","total_count":2,"view_count":2}]},
                "tools":[{"count":1,"percentage":100.0,"tool":"read_file"}],
                "totals":{"total_actual_cost":0.02,"total_api_calls":3,"total_cache_read":10,"total_estimated_cost":0.04,"total_input":120,"total_output":80,"total_reasoning":5,"total_sessions":2}
            }).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            )
            .expect("write usage analytics response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "usage-fixture-instance".into(),
            authority_epoch: 5,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "usage-fixture-token".into(),
            ))),
        };
        let snapshot = load_command_center_usage_http(&endpoint, Some("work profile"), 30)
            .expect("profile-scoped analytics load through remote auth");
        assert_eq!(snapshot.period_days, 30);
        assert_eq!(snapshot.totals.total_sessions, 2);
        assert_eq!(snapshot.daily[0].input_tokens, 120);
        assert_eq!(snapshot.by_model[0].model, "model-fixture");
        assert_eq!(snapshot.skills.top_skills[0].total_count, 2);
        assert_eq!(snapshot.tools.as_ref().unwrap()[0].tool, "read_file");
        assert!(load_command_center_usage_http(&endpoint, Some("work profile"), 14).is_err());
        server.join().expect("usage fixture exits");
    }

    #[test]
    fn command_center_maintenance_loads_snapshot_runs_actions_and_validates_receipts() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind maintenance fixture");
        let address = listener.local_addr().expect("maintenance fixture address");
        let server = thread::spawn(move || {
            let expected = [
                (
                    "GET /hermes/api/curator?profile=work%20profile HTTP/1.1\r\n",
                    json!({
                        "enabled": true,
                        "paused": false,
                        "interval_hours": 6.0,
                        "last_run_at": "2026-11-18T12:00:00Z"
                    })
                    .to_string(),
                ),
                (
                    "GET /hermes/api/memory?profile=work%20profile HTTP/1.1\r\n",
                    json!({
                        "active": "builtin",
                        "providers": [{
                            "name": "builtin",
                            "description": "Local markdown memory",
                            "available": true,
                            "configured": true,
                            "status": "ready"
                        }],
                        "builtin_files": {"memory": 512, "user": 0}
                    })
                    .to_string(),
                ),
                (
                    "POST /hermes/api/ops/doctor?profile=work%20profile HTTP/1.1\r\n",
                    json!({"ok": true, "name": "doctor", "pid": 4242}).to_string(),
                ),
                (
                    "GET /hermes/api/actions/doctor/status?lines=200&profile=work%20profile HTTP/1.1\r\n",
                    json!({
                        "name": "doctor",
                        "running": false,
                        "exit_code": 0,
                        "lines": ["Doctor check passed"]
                    })
                    .to_string(),
                ),
                (
                    "PUT /hermes/api/curator/paused?profile=work%20profile HTTP/1.1\r\n",
                    json!({"ok": true, "paused": true}).to_string(),
                ),
                (
                    "POST /hermes/api/memory/reset?profile=work%20profile HTTP/1.1\r\n",
                    json!({"ok": true, "deleted": ["MEMORY.md"]}).to_string(),
                ),
                (
                    "POST /hermes/api/ops/debug-share?profile=work%20profile HTTP/1.1\r\n",
                    json!({
                        "ok": true,
                        "urls": {"bundle": "https://share.hermes.test/bundle-1"},
                        "failures": {},
                        "redacted": true
                    })
                    .to_string(),
                ),
            ];
            for (request_line, body) in expected {
                let (mut stream, _) = listener.accept().expect("accept maintenance request");
                let request = read_http_fixture_request(&mut stream);
                assert!(
                    request.starts_with(request_line),
                    "unexpected maintenance request: {request}"
                );
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("x-hermes-session-token: maintenance-fixture-token\r\n")
                );
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write maintenance response");
            }
        });
        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "maintenance-fixture-instance".into(),
            authority_epoch: 6,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "maintenance-fixture-token".into(),
            ))),
        };
        let snapshot = command_center_maintenance_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterMaintenanceOperation::LoadSnapshot,
        )
        .expect("load maintenance snapshot");
        match snapshot {
            CommandCenterMaintenanceResult::Snapshot(snapshot) => {
                let curator = snapshot.curator.expect("curator status");
                assert!(curator.enabled);
                assert!(!curator.paused);
                let memory = snapshot.memory.expect("memory status");
                assert_eq!(memory.active, "builtin");
                assert_eq!(memory.builtin_files.memory, 512);
                assert_eq!(memory.builtin_files.user, 0);
            }
            other => panic!("expected maintenance snapshot, got {other:?}"),
        }
        let started = command_center_maintenance_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterMaintenanceOperation::StartAction {
                action: CommandCenterMaintenanceAction::Doctor,
            },
        )
        .expect("start doctor action");
        assert!(matches!(
            started,
            CommandCenterMaintenanceResult::ActionStarted(receipt) if receipt.name == "doctor"
        ));
        let status = command_center_maintenance_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterMaintenanceOperation::LoadActionStatus {
                name: "doctor".into(),
            },
        )
        .expect("load doctor action status");
        assert!(matches!(
            status,
            CommandCenterMaintenanceResult::ActionStatus(status)
                if !status.running && status.exit_code == Some(0) && status.lines == ["Doctor check passed"]
        ));
        let paused = command_center_maintenance_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterMaintenanceOperation::SetCuratorPaused { paused: true },
        )
        .expect("pause curator");
        assert!(matches!(
            paused,
            CommandCenterMaintenanceResult::CuratorPaused(receipt) if receipt.paused
        ));
        let reset = command_center_maintenance_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterMaintenanceOperation::ResetMemory {
                target: CommandCenterMemoryResetTarget::Memory,
            },
        )
        .expect("reset MEMORY.md");
        assert!(matches!(
            reset,
            CommandCenterMaintenanceResult::MemoryReset(receipt) if receipt.deleted == ["MEMORY.md"]
        ));
        let share = command_center_maintenance_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterMaintenanceOperation::DebugShare,
        )
        .expect("share debug bundle");
        assert!(matches!(
            share,
            CommandCenterMaintenanceResult::DebugShare(receipt)
                if receipt.urls.get("bundle").map(String::as_str)
                    == Some("https://share.hermes.test/bundle-1")
        ));
        assert!(
            command_center_maintenance_http(
                &endpoint,
                Some("work profile"),
                &CommandCenterMaintenanceOperation::LoadActionStatus {
                    name: "unsupported".into()
                },
            )
            .is_err()
        );
        server.join().expect("maintenance fixture exits");
    }

    #[test]
    fn session_delete_fetches_mutation_snapshot_and_sends_fenced_delete_query() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind session delete fixture");
        let address = listener
            .local_addr()
            .expect("session delete fixture address");
        let server = thread::spawn(move || {
            let (mut snapshot_stream, _) = listener.accept().expect("accept mutation snapshot GET");
            let snapshot_request = read_http_fixture_request(&mut snapshot_stream);
            assert!(snapshot_request.starts_with(
                "GET /hermes/api/sessions/sess-1/mutation-snapshot?profile=work%20profile HTTP/1.1\r\n"
            ));
            assert!(
                snapshot_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: delete-fixture-token\r\n")
            );
            let snapshot = json!({
                "session_id": "sess-1",
                "exists": true,
                "runtime_revision": 12,
                "runtime_generation": 8
            })
            .to_string();
            write!(
                snapshot_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                snapshot.len(),
                snapshot
            )
            .expect("write mutation snapshot");

            let (mut delete_stream, _) = listener.accept().expect("accept session DELETE");
            let delete_request = read_http_fixture_request(&mut delete_stream);
            assert!(delete_request.starts_with(
                "DELETE /hermes/api/sessions/sess-1?profile=work%20profile&expected_revision=12&expected_generation=8&request_id=delete-op-1 HTTP/1.1\r\n"
            ));
            assert!(
                delete_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: delete-fixture-token\r\n")
            );
            let receipt = json!({"ok": true}).to_string();
            write!(
                delete_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                receipt.len(),
                receipt
            )
            .expect("write delete receipt");
        });

        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "delete-fixture-instance".into(),
            authority_epoch: 7,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "delete-fixture-token".into(),
            ))),
        };
        delete_session_http(&endpoint, "sess-1", Some("work profile"), "delete-op-1")
            .expect("fenced session DELETE succeeds");
        assert!(delete_session_http(&endpoint, "sess-1", Some("all"), "delete-op-2").is_err());
        server.join().expect("session delete fixture exits");
    }

    #[test]
    fn session_export_pages_all_messages_in_oldest_order_and_enforces_size_limit() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind session export fixture");
        let address = listener
            .local_addr()
            .expect("session export fixture address");
        let server = thread::spawn(move || {
            let (mut page1_stream, _) = listener.accept().expect("accept export page 1 GET");
            let page1_request = read_http_fixture_request(&mut page1_stream);
            assert!(page1_request.starts_with(
                "GET /hermes/api/sessions/sess-export/messages?profile=work%20profile&limit=500&offset=0&order=oldest&include_compacted=true HTTP/1.1\r\n"
            ));
            assert!(
                page1_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: export-fixture-token\r\n")
            );
            let page1_messages: Vec<serde_json::Value> = (0..500)
                .map(|index| json!({"role": "user", "content": format!("msg-{index}")}))
                .collect();
            let page1 = json!({
                "session_id": "sess-export",
                "profile": "work profile",
                "messages": page1_messages,
                "pagination": {
                    "limit": 500,
                    "offset": 0,
                    "order": "oldest",
                    "returned": 500
                }
            })
            .to_string();
            write!(
                page1_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                page1.len(),
                page1
            )
            .expect("write export page 1");

            let (mut page2_stream, _) = listener.accept().expect("accept export page 2 GET");
            let page2_request = read_http_fixture_request(&mut page2_stream);
            assert!(page2_request.starts_with(
                "GET /hermes/api/sessions/sess-export/messages?profile=work%20profile&limit=500&offset=500&order=oldest&include_compacted=true HTTP/1.1\r\n"
            ));
            let page2 = json!({
                "session_id": "sess-export",
                "profile": "work profile",
                "messages": [{"role": "assistant", "content": "msg-500"}],
                "pagination": {
                    "limit": 500,
                    "offset": 500,
                    "order": "oldest",
                    "returned": 1
                }
            })
            .to_string();
            write!(
                page2_stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                page2.len(),
                page2
            )
            .expect("write export page 2");
        });

        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "export-fixture-instance".into(),
            authority_epoch: 8,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "export-fixture-token".into(),
            ))),
        };
        let transcript =
            load_all_session_messages_http(&endpoint, "sess-export", Some("work profile"))
                .expect("paged session export succeeds");
        assert_eq!(transcript.resolved_session_id, "sess-export");
        assert_eq!(transcript.messages.len(), 501);
        assert_eq!(transcript.messages[0]["content"], "msg-0");
        assert_eq!(transcript.messages[500]["content"], "msg-500");
        assert!(load_all_session_messages_http(&endpoint, "sess-export", Some("all")).is_err());
        server.join().expect("session export fixture exits");
    }

    #[test]
    fn command_center_system_actions_start_restart_and_update_and_poll_180_line_status() {
        use std::io::Write as _;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind system action fixture");
        let address = listener
            .local_addr()
            .expect("system action fixture address");
        let server = thread::spawn(move || {
            let expected = [
                (
                    "POST /hermes/api/gateway/restart?profile=work%20profile HTTP/1.1\r\n",
                    json!({"ok": true, "name": "gateway-restart", "pid": 3131}).to_string(),
                ),
                (
                    "POST /hermes/api/hermes/update?profile=work%20profile HTTP/1.1\r\n",
                    json!({"ok": true, "name": "hermes-update", "pid": 4141}).to_string(),
                ),
                (
                    "GET /hermes/api/actions/hermes-update/status?lines=180&profile=work%20profile HTTP/1.1\r\n",
                    json!({
                        "name": "hermes-update",
                        "running": false,
                        "exit_code": 0,
                        "lines": ["Hermes update complete"]
                    })
                    .to_string(),
                ),
            ];
            for (request_line, body) in expected {
                let (mut stream, _) = listener.accept().expect("accept system action request");
                let request = read_http_fixture_request(&mut stream);
                assert!(
                    request.starts_with(request_line),
                    "unexpected system action request: {request}"
                );
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("x-hermes-session-token: system-action-token\r\n")
                );
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write system action response");
            }
        });

        let endpoint = GatewayEndpoint {
            profile_id: "work profile".into(),
            instance_id: "system-action-instance".into(),
            authority_epoch: 9,
            runtime_protocol: 1,
            api_origin: format!("http://{address}/hermes"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "system-action-token".into(),
            ))),
        };
        let restart = command_center_system_action_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterSystemOperation::StartAction {
                action: CommandCenterSystemAction::RestartGateway,
            },
        )
        .expect("start gateway restart");
        assert!(matches!(
            restart,
            CommandCenterSystemActionResult::ActionStarted(receipt) if receipt.name == "gateway-restart"
        ));
        let update = command_center_system_action_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterSystemOperation::StartAction {
                action: CommandCenterSystemAction::UpdateHermes,
            },
        )
        .expect("start hermes update");
        assert!(matches!(
            update,
            CommandCenterSystemActionResult::ActionStarted(receipt) if receipt.name == "hermes-update"
        ));
        let status = command_center_system_action_http(
            &endpoint,
            Some("work profile"),
            &CommandCenterSystemOperation::LoadActionStatus {
                name: "hermes-update".into(),
            },
        )
        .expect("poll hermes update status with 180 lines");
        assert!(matches!(
            status,
            CommandCenterSystemActionResult::ActionStatus(status)
                if !status.running && status.exit_code == Some(0) && status.lines == ["Hermes update complete"]
        ));
        server.join().expect("system action fixture exits");
    }

    #[test]
    fn rpc_call_replies_method_not_found_for_unhandled_server_request_and_continues() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read client RPC") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse client RPC"),
                other => panic!("expected client RPC, got {other:?}"),
            };
            let rpc_id = request["id"].as_u64().expect("outgoing RPC ID");
            assert_eq!(request["method"], "runtime.describe");
            socket
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0",
                        "id":"server-request-1",
                        "method":"unsupported.preview",
                        "params":{"session_id":"runtime-1"}
                    })
                    .to_string()
                    .into(),
                ))
                .expect("send inbound server request");
            let reply: serde_json::Value = match socket.read().expect("read decline reply") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse decline reply"),
                other => panic!("expected decline reply, got {other:?}"),
            };
            assert_eq!(reply["jsonrpc"], "2.0");
            assert_eq!(reply["id"], "server-request-1");
            assert_eq!(reply["error"]["code"], -32601);
            assert_eq!(
                reply["error"]["message"],
                "no handler for server request: unsupported.preview"
            );
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":rpc_id,"result":{"instance_id":"fixture"}})
                        .to_string()
                        .into(),
                ))
                .expect("complete original RPC");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("configure client read timeout");
        }
        let (events, received_events) = mpsc::channel();
        let result = rpc_with_events(
            &mut socket,
            17,
            "runtime.describe",
            json!({}),
            Some(&events),
        )
        .expect("original RPC should continue after declining the unsupported method");
        assert_eq!(result["instance_id"], "fixture");
        assert!(received_events.try_recv().is_err());
        server.join().expect("fixture server exits");
    }

    #[test]
    fn idle_actor_routes_approval_server_request_and_returns_result_on_same_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            socket
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0",
                        "id":"approval-wire-1",
                        "method":"approval",
                        "params":{"session_id":"runtime-1","request_id":"approval-1"}
                    })
                    .to_string()
                    .into(),
                ))
                .expect("send approval server request");
            let response: serde_json::Value = match socket.read().expect("read server reply") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse server reply"),
                other => panic!("expected JSON-RPC response, got {other:?}"),
            };
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], "approval-wire-1");
            assert_eq!(response["result"], json!({"choice":"once"}));
            socket.close(None).expect("close fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("configure client read timeout");
        }
        let (commands, command_receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_receiver, events));
        let request = event_receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("approval request reaches the UI event channel");
        assert!(matches!(
            request,
            GatewayEvent::ServerRequestReceived { id, method, params }
                if id == "approval-wire-1"
                    && method == "approval"
                    && params["session_id"] == "runtime-1"
        ));
        commands
            .send(GatewayCommand::RespondServerRequest {
                id: "approval-wire-1".into(),
                response: Ok(json!({"choice":"once"})),
            })
            .expect("queue approval response");
        let receipt = event_receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("server request response receipt");
        assert!(matches!(
            receipt,
            GatewayEvent::ServerRequestAnswered { id, result: Ok(()) }
                if id == "approval-wire-1"
        ));
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn idle_actor_routes_batch_clarify_server_request_and_returns_answers() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            socket
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0",
                        "id":"clarify-wire-1",
                        "method":"clarify",
                        "params":{"session_id":"runtime-1","questions":[{"qid":"q1","question":"Choose"}]}
                    })
                    .to_string()
                    .into(),
                ))
                .expect("send clarify server request");
            let response: serde_json::Value = match socket.read().expect("read server reply") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse server reply"),
                other => panic!("expected JSON-RPC response, got {other:?}"),
            };
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], "clarify-wire-1");
            assert_eq!(response["result"]["answers"]["q1"], "Desktop");
            socket.close(None).expect("close fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("configure client read timeout");
        }
        let (commands, command_receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_receiver, events));
        let request = event_receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("clarify request reaches the UI event channel");
        assert!(matches!(
            request,
            GatewayEvent::ServerRequestReceived { id, method, params }
                if id == "clarify-wire-1"
                    && method == "clarify"
                    && params["session_id"] == "runtime-1"
        ));
        commands
            .send(GatewayCommand::RespondServerRequest {
                id: "clarify-wire-1".into(),
                response: Ok(json!({"answers":{"q1":"Desktop"}})),
            })
            .expect("queue clarify response");
        let receipt = event_receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("server request response receipt");
        assert!(matches!(
            receipt,
            GatewayEvent::ServerRequestAnswered { id, result: Ok(()) }
                if id == "clarify-wire-1"
        ));
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn clarify_lock_rpc_preserves_question_order_and_accepts_the_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read clarify.lock") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse clarify.lock"),
                other => panic!("expected clarify.lock request, got {other:?}"),
            };
            assert_eq!(request["method"], "clarify.lock");
            assert_eq!(
                request["params"],
                json!({
                    "request_id":"clarify-wire-1",
                    "question_id":"q2",
                    "answer":"[\"Unit\",\"UI\"]"
                })
            );
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":request["id"],"result":{"status":"ok","remaining":["q3"]}})
                        .to_string()
                        .into(),
                ))
                .expect("send clarify.lock receipt");
            socket.close(None).expect("close fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("configure client read timeout");
        }
        let (commands, command_receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_receiver, events));
        commands
            .send(GatewayCommand::LockClarify {
                request_id: "lock-1".into(),
                clarify_request_id: "clarify-wire-1".into(),
                question_id: "q2".into(),
                profile: None,
                answer: Some("[\"Unit\",\"UI\"]".into()),
            })
            .expect("queue clarify.lock request");
        let receipt = event_receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("clarify.lock result reaches the UI event channel");
        assert!(matches!(
            receipt,
            GatewayEvent::ClarifyLocked {
                request_id,
                clarify_request_id,
                question_id,
                result: Ok(ClarifyLockReceipt { expired: false })
            } if request_id == "lock-1"
                && clarify_request_id == "clarify-wire-1"
                && question_id == "q2"
        ));
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn session_list_preserves_fractional_started_at_for_sidebar_recency() {
        let session: SessionSummary = serde_json::from_value(json!({
            "id":"session-1",
            "connection_id":"homelab",
            "_lineage_root_id":"lineage-root-1",
            "title":"Recent session",
            "started_at":1731543892.75
        }))
        .expect("decode current session list row");
        assert_eq!(session.started_at, 1_731_543_892.75);
        assert_eq!(session.connection_id.as_deref(), Some("homelab"));
        assert_eq!(session.lineage_root_id.as_deref(), Some("lineage-root-1"));
        assert_eq!(session.durable_session_id(), "lineage-root-1");
        let owner = session
            .owner_identity()
            .expect("valid stored-session owner");
        assert_eq!(owner.connection_id(), Some("homelab"));
        assert_eq!(owner.profile(), "default");
    }

    #[test]
    fn archived_session_pages_dedupe_pinned_backfills_and_use_fixed_offsets() {
        let mut offsets = Vec::new();
        let sessions = collect_archived_session_pages(|offset| {
            offsets.push(offset);
            let rows = if offset == 0 {
                vec![
                    SessionSummary {
                        cwd: None,
                        id: "archived-1".into(),
                        connection_id: None,
                        lineage_root_id: None,
                        profile: Some("alpha".into()),
                        title: "Older conversation".into(),
                        preview: String::new(),
                        message_count: 3,
                        source: "desktop".into(),
                        started_at: 1.0,
                        last_active: None,
                        pinned: false,
                        unread: false,
                        archived: true,
                    },
                    SessionSummary {
                        cwd: None,
                        id: "live-pin".into(),
                        connection_id: None,
                        lineage_root_id: None,
                        profile: Some("alpha".into()),
                        title: "Pinned live row".into(),
                        preview: String::new(),
                        message_count: 1,
                        source: "desktop".into(),
                        started_at: 2.0,
                        last_active: None,
                        pinned: true,
                        unread: false,
                        archived: false,
                    },
                ]
            } else {
                vec![
                    SessionSummary {
                        cwd: None,
                        id: "archived-2".into(),
                        connection_id: None,
                        lineage_root_id: None,
                        profile: Some("beta".into()),
                        title: "Newest archived conversation".into(),
                        preview: String::new(),
                        message_count: 4,
                        source: "desktop".into(),
                        started_at: 3.0,
                        last_active: None,
                        pinned: false,
                        unread: true,
                        archived: true,
                    },
                    SessionSummary {
                        cwd: None,
                        id: "live-pin".into(),
                        connection_id: None,
                        lineage_root_id: None,
                        profile: Some("alpha".into()),
                        title: "Pinned live row".into(),
                        preview: String::new(),
                        message_count: 1,
                        source: "desktop".into(),
                        started_at: 2.0,
                        last_active: None,
                        pinned: true,
                        unread: false,
                        archived: false,
                    },
                    SessionSummary {
                        cwd: None,
                        id: "archived-1".into(),
                        connection_id: None,
                        lineage_root_id: None,
                        profile: Some("beta".into()),
                        title: "Same stored ID in another profile".into(),
                        preview: String::new(),
                        message_count: 2,
                        source: "desktop".into(),
                        started_at: 4.0,
                        last_active: None,
                        pinned: false,
                        unread: false,
                        archived: true,
                    },
                ]
            };
            Ok(SessionListPage {
                sessions: rows,
                total: 3,
            })
        })
        .expect("read every archived page");

        assert_eq!(offsets, [0, 500]);
        assert_eq!(
            sessions
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            ["archived-1", "live-pin", "archived-2", "archived-1"]
        );
        assert_eq!(
            sessions
                .iter()
                .filter(|session| session.id == "archived-1")
                .map(|session| session.profile.as_deref())
                .collect::<Vec<_>>(),
            [Some("alpha"), Some("beta")]
        );
    }

    #[test]
    fn archived_session_list_requests_only_archived_rows_at_the_requested_offset() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind archived fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept archived request");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with(
                "GET /api/profiles/sessions?limit=500&offset=500&min_messages=0&archived=only&order=recent&profile=all HTTP/1.1\r\n"
            ));
            let body = json!({"total":201,"sessions":[{
                "id":"archived-201",
                "profile":"beta",
                "title":"Archived row",
                "archived":true
            }]})
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write archived fixture response");
        });
        let page = request_all_profile_archived_session_page(
            &format!("http://{address}"),
            "fixture-native-http-ticket",
            500,
        )
        .expect("decode archived page");
        assert_eq!(page.total, 201);
        assert_eq!(page.sessions.len(), 1);
        assert_eq!(page.sessions[0].id, "archived-201");
        assert!(page.sessions[0].archived);
        assert_eq!(page.sessions[0].profile.as_deref(), Some("beta"));
        server.join().expect("fixture server exits");
    }

    #[test]
    fn command_palette_session_list_uses_pinned_all_profile_query() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind palette session fixture");
        let address = listener.local_addr().expect("palette fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept palette session request");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with(
                "GET /api/profiles/sessions?limit=200&offset=0&min_messages=1&archived=exclude&order=recent&profile=all HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
            );
            let body = json!({"total":1,"sessions":[{
                "id":"20261115_120000_abcdef",
                "profile":"work",
                "title":"Palette result",
                "preview":"searchable preview",
                "message_count":3,
                "pinned":true,
                "archived":false},{"id":"20261115_120001_abcdef","title":"Legacy owner row","message_count":2}]}).to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write palette session fixture response");
        });
        let sessions = request_all_profile_command_palette_sessions(
            &format!("http://{address}"),
            "fixture-native-http-ticket",
            Some("active"),
        )
        .expect("read palette session result");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].profile.as_deref(), Some("work"));
        assert_eq!(sessions[0].title, "Palette result");
        assert_eq!(sessions[0].preview, "searchable preview");
        assert!(sessions[0].pinned);
        assert_eq!(sessions[1].profile.as_deref(), Some("active"));
        assert_eq!(sessions[1].title, "Legacy owner row");
        server.join().expect("palette fixture server exits");
    }
    #[test]
    fn profile_session_list_reads_last_active_with_a_native_http_ticket() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept session list request");
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let count = stream.read(&mut buffer).expect("read HTTP request");
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("request is ASCII");
            assert!(request.starts_with(
                "GET /api/sessions?limit=200&offset=0&min_messages=0&archived=exclude&order=recent HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
            );
            let body = json!({"sessions":[{
                "id":"session-recent",
                "_lineage_root_id":"lineage-recent-root",
                "cwd":"C:/Users/test/project",
                "title":"Recent session",
                "preview":"Latest message",
                "message_count":8,
                "source":"desktop",
                "started_at":1731543800.0,
                "last_active":1731543892.75,
                "pinned":true,
                "unread":true
            },{
                "id":"session-legacy",
                "title":null,
                "preview":null,
                "message_count":1,
                "source":null,
                "started_at":1731543700.0,
                "last_active":null
            }]})
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write session list fixture response");
        });

        let sessions =
            request_session_list(&format!("http://{address}"), "fixture-native-http-ticket")
                .expect("decode profile session list");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].last_active, Some(1_731_543_892.75));
        assert!(sessions[0].pinned);
        assert!(sessions[0].unread);
        assert_eq!(sessions[0].started_at, 1_731_543_800.0);
        assert_eq!(
            sessions[0].lineage_root_id.as_deref(),
            Some("lineage-recent-root")
        );
        assert_eq!(sessions[0].cwd.as_deref(), Some("C:/Users/test/project"));
        assert_eq!(sessions[1].title, "");
        assert_eq!(sessions[1].source, "");
        assert_eq!(sessions[1].last_active, None);
        assert!(!sessions[1].pinned);
        assert!(!sessions[1].unread);
        server.join().expect("fixture server exits");
    }

    #[test]
    fn command_center_sidebar_uses_profile_owned_window_and_truncation_metadata() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind sidebar fixture");
        let address = listener.local_addr().expect("sidebar fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept sidebar request");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("GET /api/profiles/sessions/sidebar?recents_profile=all&recents_limit=100&cron_limit=50&messaging_limit=100&recents_exclude=acp%2Ccron%2Ckanban%2Coneshot%2Csubagent%2Ctool%2Ctelegram%2Cdiscord%2Cslack%2Cmattermost%2Cmatrix%2Csignal%2Cwhatsapp%2Cbluebubbles%2Cphoton%2Chomeassistant%2Cemail%2Csms%2Cwebhook%2Capi_server%2Cweixin%2Cwecom%2Cqqbot%2Cyuanbao%2Cdingtalk%2Cfeishu&messaging_exclude=cron%2Cacp%2Ccli%2Ccodex%2Cdesktop%2Cgateway%2Ckanban%2Clocal%2Coneshot%2Ctui HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: fixture-session-token\r\n")
            );
            let body = json!({
                "recents": {
                    "sessions": [
                        {"id":"a1","profile":"profile-a","title":"A one","message_count":2},
                        {"id":"b1","profile":"profile-b","title":"B one","message_count":1}
                    ],
                    "profiles_truncated":{"profile-a":true,"profile-b":false},
                    "errors":[{"profile":"profile-c","error":"state database unavailable"}]
                },
                "errors":[],
                "profiles_failed":{"profile-c":{"failed":true,"retry":true}}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write sidebar fixture response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "all".into(),
            instance_id: "sidebar-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "fixture-session-token".into(),
            ))),
        };
        let snapshot = load_command_center_sessions_http(&endpoint, 100)
            .expect("read cross-profile sidebar snapshot");
        assert_eq!(snapshot.sessions.len(), 2);
        assert_eq!(snapshot.sessions[0].profile.as_deref(), Some("profile-a"));
        assert_eq!(snapshot.profiles_truncated.get("profile-a"), Some(&true));
        assert_eq!(snapshot.profiles_truncated.get("profile-b"), Some(&false));
        assert!(snapshot.failed_profiles.contains("profile-c"));
        assert!(
            snapshot
                .error
                .as_deref()
                .unwrap()
                .contains("state database unavailable")
        );
        server.join().expect("sidebar fixture server exits");
    }

    #[test]
    fn command_center_sidebar_falls_back_to_legacy_list_when_batch_route_is_missing() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind legacy sidebar fixture");
        let address = listener.local_addr().expect("legacy sidebar address");
        let server = thread::spawn(move || {
            let (mut batch_stream, _) = listener.accept().expect("accept batch request");
            let batch_request = read_http_fixture_request(&mut batch_stream);
            assert!(batch_request.starts_with("GET /api/profiles/sessions/sidebar?"));
            write!(
                batch_stream,
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .expect("write route-missing response");
            drop(batch_stream);

            let (mut legacy_stream, _) = listener.accept().expect("accept legacy request");
            let legacy_request = read_http_fixture_request(&mut legacy_stream);
            assert!(legacy_request.starts_with("GET /api/profiles/sessions?limit=2&offset=0&min_messages=1&archived=exclude&order=recent&profile=all&exclude_sources=acp%2Ccron%2Ckanban%2Coneshot%2Csubagent%2Ctool"));
            let body = json!({"sessions":[
                {"id":"a1","profile":"profile-a","message_count":1},
                {"id":"a2","profile":"profile-a","message_count":1},
                {"id":"b1","profile":"profile-b","message_count":1}
            ],"total":3})
            .to_string();
            write!(legacy_stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write legacy list response");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "all".into(),
            instance_id: "legacy-sidebar-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "fixture-session-token".into(),
            ))),
        };
        let snapshot = load_command_center_sessions_http(&endpoint, 2)
            .expect("legacy endpoint fallback loads rows");
        assert_eq!(snapshot.sessions.len(), 3);
        assert_eq!(snapshot.profiles_truncated.get("profile-a"), Some(&true));
        assert_eq!(snapshot.profiles_truncated.get("profile-b"), Some(&false));
        server.join().expect("legacy sidebar fixture server exits");
    }

    #[test]
    fn display_timestamps_config_read_uses_profile_scoped_source_api() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind display-config fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept display-config request");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("GET /api/config?profile=work%20profile HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
            );
            let body = json!({
                "display":{"timestamps":true},
                "model":{"provider":"ignored without deserializing secrets"}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write display-config fixture response");
        });
        let enabled = read_display_timestamps(
            &format!("http://{address}"),
            "fixture-native-http-ticket",
            "work profile",
        )
        .expect("read source display config");
        assert!(enabled);
        server.join().expect("fixture server exits");
    }

    #[test]
    fn repo_scan_config_defaults_and_filters_malformed_values() {
        assert_eq!(
            repo_scan_policy_from_config(&json!({})),
            RepoScanPolicy::default()
        );
        assert_eq!(
            repo_scan_policy_from_config(&json!({
                "desktop": {
                    "repo_scan_enabled": false,
                    "repo_scan_roots": ["/work", 7, "~/code"],
                    "repo_scan_exclude_paths": ["/work/vendor", null]
                },
                "model": {"api_key": "must-not-be-copied"}
            })),
            RepoScanPolicy {
                enabled: false,
                roots: vec!["/work".into(), "~/code".into()],
                exclude_paths: vec!["/work/vendor".into()],
            }
        );
    }

    #[test]
    fn repo_scan_config_save_uses_profile_scoped_sparse_merge_patch() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind repo config fixture");
        let address = listener.local_addr().expect("repo config fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept repo config PUT");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("PUT /api/config?profile=work%20profile HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
            );
            let body = request
                .split_once("\r\n\r\n")
                .expect("config request body")
                .1;
            let value: serde_json::Value = serde_json::from_str(body).expect("sparse config JSON");
            assert_eq!(value["config"]["desktop"]["repo_scan_enabled"], true);
            assert_eq!(
                value["config"]["desktop"]["repo_scan_roots"],
                json!(["/work"])
            );
            assert_eq!(
                value["config"]["desktop"]["repo_scan_exclude_paths"],
                json!(["/work/vendor"])
            );
            assert!(value["config"]["model"].is_null());
            let response = r#"{"ok":true}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response)
                .expect("write repo config receipt");
        });
        let saved = save_repo_scan_config_local(
            &format!("http://{address}"),
            "fixture-native-http-ticket",
            "work profile",
            &RepoScanPolicy {
                enabled: true,
                roots: vec!["/work".into()],
                exclude_paths: vec!["/work/vendor".into()],
            },
        )
        .expect("profile-scoped config patch");
        assert_eq!(saved["ok"], true);
        server.join().expect("repo config fixture exits");
    }

    #[test]
    fn repo_scan_config_read_uses_verified_loopback_ticket_and_profile() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind repo config read fixture");
        let address = listener
            .local_addr()
            .expect("repo config read fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept repo config GET");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("GET /api/config?profile=work%20profile HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
            );
            let body = json!({"desktop":{"repo_scan_enabled":false,"repo_scan_roots":["/work"]}})
                .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write repo config response");
        });
        let policy = read_repo_scan_config_local(
            &format!("http://{address}"),
            "fixture-native-http-ticket",
            "work profile",
        )
        .expect("profile-scoped config read");
        assert_eq!(policy.enabled, false);
        assert_eq!(policy.roots, ["/work"]);
        assert!(policy.exclude_paths.is_empty());
        server.join().expect("repo config read fixture exits");
    }

    #[test]
    fn local_repo_scan_is_profile_scoped_and_discards_stale_generation() {
        let (gateway, commands) = GatewayDiscovery::for_test_repo_discovery_commands();
        let temp = std::env::temp_dir().join(format!(
            "hermes-repo-generation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let repo = temp.join("configured-root").join("repo");
        std::fs::create_dir_all(repo.join(".git")).expect("create isolated repo fixture");
        std::fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/main\n")
            .expect("write valid repository HEAD");
        let policy = RepoScanPolicy {
            enabled: true,
            roots: vec![temp.join("configured-root").to_string_lossy().into_owned()],
            exclude_paths: vec![],
        };
        let generation_state = Arc::new(AtomicU64::new(2));
        gateway
            .scan_and_record_repos(
                "stale-request".into(),
                "work".into(),
                policy.clone(),
                temp.clone(),
                Arc::clone(&generation_state),
                1,
            )
            .expect("start stale scan worker");
        assert!(commands.recv_timeout(Duration::from_millis(40)).is_err());

        gateway
            .scan_and_record_repos(
                "current-request".into(),
                "work".into(),
                policy,
                temp.clone(),
                Arc::clone(&generation_state),
                2,
            )
            .expect("start current scan worker");
        match commands
            .recv_timeout(Duration::from_secs(2))
            .expect("current scan submits host record")
        {
            GatewayCommand::RecordDiscoveredRepos {
                request_id,
                profile,
                repos,
                ..
            } => {
                assert_eq!(request_id, "current-request");
                assert_eq!(profile, "work");
                assert_eq!(repos.len(), 1);
                assert_eq!(repos[0].label, "repo");
            }
            _ => panic!("unexpected repository discovery command"),
        }
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn remote_repo_discovery_refuses_all_profile_scope() {
        let (gateway, commands) = GatewayDiscovery::for_test_repo_discovery_commands();
        assert!(
            gateway
                .discover_remote_repos("all-request".into(), "all".into())
                .is_err()
        );
        gateway
            .discover_remote_repos("work-request".into(), "work".into())
            .expect("queue concrete-profile remote scan");
        assert!(matches!(
            commands.try_recv(),
            Ok(GatewayCommand::DiscoverRemoteRepos { request_id, profile })
                if request_id == "work-request" && profile == "work"
        ));
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn remote_repo_discovery_uses_source_params_and_rejects_missing_repo_list() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind repo discovery fixture");
        let address = listener
            .local_addr()
            .expect("repo discovery fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept repo discovery client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            for (index, response) in [
                json!({"repos":[{"id":"repo-a","root":"/work/repo-a"}]}),
                json!({"accepted":false}),
            ]
            .into_iter()
            .enumerate()
            {
                let request: serde_json::Value = match socket.read().expect("read discovery RPC") {
                    Message::Text(text) => {
                        serde_json::from_str(&text).expect("parse discovery RPC")
                    }
                    other => panic!("expected discovery RPC, got {other:?}"),
                };
                assert_eq!(request["method"], "projects.discover_repos");
                assert_eq!(
                    request["params"],
                    json!({"scan":true,"profile":"fixture-profile"})
                );
                let id = request["id"].as_u64().expect("numeric discovery RPC id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":response})
                            .to_string()
                            .into(),
                    ))
                    .expect("send discovery result");
                assert_eq!(index, id as usize - 5);
            }
            socket.close(None).expect("close discovery fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("configure fixture read timeout");
        }
        let (commands, command_receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_receiver, events));

        commands
            .send(GatewayCommand::DiscoverRemoteRepos {
                request_id: "remote-scan-ok".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue successful remote scan");
        assert!(matches!(
            event_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("successful remote scan event"),
            GatewayEvent::RepoDiscoveryCompleted {
                request_id,
                profile,
                result: Ok(()),
            } if request_id == "remote-scan-ok" && profile == "fixture-profile"
        ));

        commands
            .send(GatewayCommand::DiscoverRemoteRepos {
                request_id: "remote-scan-malformed".into(),
                profile: "fixture-profile".into(),
            })
            .expect("queue malformed remote scan");
        assert!(matches!(
            event_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("malformed remote scan event"),
            GatewayEvent::RepoDiscoveryCompleted {
                request_id,
                profile,
                result: Err(error),
            } if request_id == "remote-scan-malformed"
                && profile == "fixture-profile"
                && error.contains("no repository list")
        ));
        drop(commands);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("repo discovery fixture exits");
    }

    #[test]
    fn display_timestamps_config_matches_hermes_truthy_contract() {
        for value in [json!(true), json!("true"), json!(1)] {
            assert!(display_timestamps_enabled(&value), "{value}");
        }
        for value in [
            json!(false),
            json!("false"),
            json!("True"),
            json!(0),
            json!("1"),
            json!(null),
        ] {
            assert!(!display_timestamps_enabled(&value), "{value}");
        }
        let missing: DisplayTimestampsConfig = serde_json::from_value(json!({})).unwrap();
        assert!(
            !missing
                .display
                .and_then(|display| display.timestamps)
                .as_ref()
                .is_some_and(display_timestamps_enabled)
        );
    }

    #[test]
    fn session_pin_uses_snapshot_fencing_and_confirms_the_patch() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut snapshot_stream, _) = listener.accept().expect("accept snapshot request");
            let snapshot_request = read_http_fixture_request(&mut snapshot_stream);
            assert!(snapshot_request.starts_with(
                "GET /api/sessions/session-42/mutation-snapshot?profile=work HTTP/1.1\r\n"
            ));
            assert!(
                snapshot_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-snapshot-ticket\r\n")
            );
            let snapshot = json!({
                "session_id":"session-42",
                "exists":true,
                "runtime_revision":17,
                "runtime_generation":9
            })
            .to_string();
            write!(snapshot_stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", snapshot.len(), snapshot)
                .expect("write mutation snapshot");

            let (mut patch_stream, _) = listener.accept().expect("accept pin patch");
            let patch_request = read_http_fixture_request(&mut patch_stream);
            assert!(patch_request.starts_with("PATCH /api/sessions/session-42 HTTP/1.1\r\n"));
            assert!(
                patch_request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-patch-ticket\r\n")
            );
            let body = patch_request
                .split_once("\r\n\r\n")
                .expect("patch body separator")
                .1;
            let body: serde_json::Value = serde_json::from_str(body).expect("parse pin body");
            assert_eq!(body["pinned"], true);
            assert_eq!(body["request_id"], "pin-op-1");
            assert_eq!(body["expected_revision"], 17);
            assert_eq!(body["expected_generation"], 9);
            assert_eq!(body["profile"], "work");
            let receipt = json!({"ok":true}).to_string();
            write!(patch_stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", receipt.len(), receipt)
                .expect("write patch receipt");
        });

        let origin = format!("http://{address}");
        let snapshot = read_session_mutation_snapshot(
            &origin,
            "fixture-snapshot-ticket",
            "session-42",
            "work",
        )
        .expect("read mutation snapshot");
        assert!(snapshot.exists);
        assert_eq!(snapshot.runtime_revision, 17);
        request_session_boolean_patch(
            &origin,
            "fixture-patch-ticket",
            "session-42",
            "pinned",
            true,
            "pin-op-1",
            snapshot.runtime_revision,
            snapshot.runtime_generation.expect("fixture generation"),
            "work",
        )
        .expect("confirm pin mutation");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn session_unread_patch_uses_the_shared_fenced_mutation_contract() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept unread patch");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("PATCH /api/sessions/session-42 HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-unread-ticket\r\n")
            );
            let body = request
                .split_once("\r\n\r\n")
                .expect("patch body separator")
                .1;
            let body: serde_json::Value = serde_json::from_str(body).expect("parse unread body");
            assert_eq!(body["unread"], true);
            assert_eq!(body["request_id"], "unread-op-1");
            assert_eq!(body["expected_revision"], 18);
            assert_eq!(body["expected_generation"], 10);
            let receipt = json!({"ok":true}).to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", receipt.len(), receipt)
                .expect("write unread receipt");
        });
        request_session_boolean_patch(
            &format!("http://{address}"),
            "fixture-unread-ticket",
            "session-42",
            "unread",
            true,
            "unread-op-1",
            18,
            10,
            "default",
        )
        .expect("confirm unread mutation");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn session_rename_patch_scopes_profile_and_returns_confirmed_title() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept rename patch");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("PATCH /api/sessions/session-42 HTTP/1.1\r\n"));
            let body: serde_json::Value = serde_json::from_str(
                request
                    .split_once("\r\n\r\n")
                    .expect("patch body separator")
                    .1,
            )
            .expect("parse rename body");
            assert_eq!(body["title"], "New title");
            assert_eq!(body["profile"], "work");
            assert_eq!(body["request_id"], "rename-op-1");
            assert_eq!(body["expected_revision"], 22);
            assert_eq!(body["expected_generation"], 14);
            let receipt = json!({"ok":true,"title":"New title"}).to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", receipt.len(), receipt)
                .expect("write rename receipt");
        });
        let receipt = request_session_patch(
            &format!("http://{address}"),
            "fixture-rename-ticket",
            "session-42",
            "title",
            serde_json::Value::String("New title".into()),
            "rename-op-1",
            22,
            14,
            "work",
        )
        .expect("confirm rename mutation");
        assert_eq!(receipt.title.as_deref(), Some("New title"));
        server.join().expect("fixture server exits");
    }

    #[test]
    fn archiving_a_session_also_clears_its_pinned_state() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept archive patch");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with("PATCH /api/sessions/session-42 HTTP/1.1\r\n"));
            let body = request
                .split_once("\r\n\r\n")
                .expect("patch body separator")
                .1;
            let body: serde_json::Value = serde_json::from_str(body).expect("parse archive body");
            assert_eq!(body["archived"], true);
            assert_eq!(body["pinned"], false);
            assert_eq!(body["request_id"], "archive-op-1");
            assert_eq!(body["profile"], "work");
            let receipt = json!({"ok":true}).to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", receipt.len(), receipt)
                .expect("write archive receipt");
        });
        request_session_boolean_patch(
            &format!("http://{address}"),
            "fixture-archive-ticket",
            "session-42",
            "archived",
            true,
            "archive-op-1",
            19,
            11,
            "work",
        )
        .expect("confirm archive mutation");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn older_transcript_page_uses_profile_scope_and_validates_latest_offset() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept transcript read");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with(
                "GET /api/sessions/session%2F42/messages?profile=work%20profile&limit=120&offset=120&order=latest&include_compacted=true HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-transcript-ticket\r\n")
            );
            let body = json!({
                "session_id":"session/42",
                "profile":"work profile",
                "messages":[{"role":"assistant","content":"older"}],
                "pagination":{"limit":120,"offset":120,"order":"latest","returned":1}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write transcript page");
        });
        let page = request_older_transcript_page(
            &format!("http://{address}"),
            "fixture-transcript-ticket",
            "session/42",
            Some("work profile"),
            120,
        )
        .expect("validate older transcript page");
        assert_eq!(
            page.messages,
            vec![json!({"role":"assistant","content":"older"})]
        );
        assert!(!page.has_more);
        assert!(!page.replace_all);
        server.join().expect("fixture server exits");
    }

    #[test]
    fn active_transcript_refresh_reads_the_profile_scoped_latest_page_at_offset_zero() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept transcript refresh");
            let request = read_http_fixture_request(&mut stream);
            assert!(request.starts_with(
                "GET /api/sessions/stored-session/messages?profile=work&limit=120&offset=0&order=latest&include_compacted=true HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-session-token: fixture-remote-transcript-token\r\n")
            );
            let body = json!({
                "session_id":"stored-session",
                "profile":"work",
                "messages":[{"id":1,"role":"assistant","content":"latest"}],
                "pagination":{"limit":120,"offset":0,"order":"latest","returned":1}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write latest transcript page");
        });
        let endpoint = GatewayEndpoint {
            profile_id: "work".into(),
            instance_id: "fixture-instance".into(),
            authority_epoch: 1,
            runtime_protocol: 1,
            api_origin: format!("http://{address}"),
            capabilities: Vec::new(),
            control_home: None,
            remote_auth: Some(RemoteGatewayAuth::Token(Zeroizing::new(
                "fixture-remote-transcript-token".into(),
            ))),
        };
        let page = load_active_transcript_http(&endpoint, "stored-session", Some("work"))
            .expect("read only active transcript refresh");
        assert_eq!(page.messages[0]["content"], "latest");
        assert!(!page.replace_all);
        server.join().expect("fixture server exits");
    }

    #[test]
    fn older_transcript_page_rejects_a_different_profile_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept transcript read");
            let _request = read_http_fixture_request(&mut stream);
            let body = json!({
                "session_id":"session-42",
                "profile":"personal",
                "messages":[],
                "pagination":{"limit":120,"offset":120,"order":"latest","returned":0}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write transcript page");
        });
        let result = request_older_transcript_page(
            &format!("http://{address}"),
            "fixture-transcript-ticket",
            "session-42",
            Some("work"),
            120,
        );
        assert!(result.unwrap_err().contains("different profile"));
        server.join().expect("fixture server exits");
    }

    #[test]
    fn older_transcript_page_rejects_backend_that_did_not_apply_latest_order() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept transcript read");
            let _request = read_http_fixture_request(&mut stream);
            let body = json!({
                "session_id":"session-42",
                "messages":[{"role":"user","content":"wrong page"}],
                "pagination":{"limit":120,"offset":120,"order":"oldest","returned":1}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write transcript page");
        });
        let result = request_older_transcript_page(
            &format!("http://{address}"),
            "fixture-transcript-ticket",
            "session-42",
            None,
            120,
        );
        assert!(result.unwrap_err().contains("did not confirm"));
        server.join().expect("fixture server exits");
    }

    #[test]
    fn legacy_transcript_response_without_pagination_replaces_with_full_history() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept transcript read");
            let _request = read_http_fixture_request(&mut stream);
            let messages = (0..121)
                .map(|index| json!({"role":"user","content":format!("legacy row {index}")}))
                .collect::<Vec<_>>();
            let body = json!({
                "session_id":"session-42",
                "messages":messages
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write legacy transcript");
        });
        let page = request_older_transcript_page(
            &format!("http://{address}"),
            "fixture-transcript-ticket",
            "session-42",
            None,
            120,
        )
        .expect("accept legacy complete transcript");
        assert!(page.replace_all);
        assert!(!page.has_more);
        assert_eq!(
            page.messages.len(),
            121,
            "legacy complete response may exceed one page"
        );
        server.join().expect("fixture server exits");
    }

    #[test]
    fn full_history_search_uses_scoped_ticket_and_decodes_ranked_results() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept HTTP request");
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let count = stream.read(&mut buffer).expect("read HTTP request");
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("HTTP request is ASCII");
            assert!(request.starts_with(
                "GET /api/sessions/search?q=needle%20%26%20tea&limit=20 HTTP/1.1\r\n"
            ));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
            );
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("accept: application/json\r\n")
            );
            let body = json!({"results":[{
                "session_id":"ranked-1",
                "title":"Ranked hit",
                "snippet":"before >>>needle<<< after",
                "last_active":1731543892.75,
                "session_started":1731543800.0,
                "lineage_root":"root-1",
                "model":"fixture-model",
                "role":"assistant",
                "source":"desktop"
            }]})
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                .expect("write HTTP fixture response");
        });

        let results = request_session_search(
            &format!("http://{address}"),
            "fixture-native-http-ticket",
            "needle & tea",
        )
        .expect("decode full-history search result");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "ranked-1");
        assert_eq!(results[0].lineage_root.as_deref(), Some("root-1"));
        assert_eq!(results[0].last_active, Some(1_731_543_892.75));
        assert_eq!(results[0].snippet, "before >>>needle<<< after");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn capabilities_read_uses_two_profile_scoped_one_use_tickets() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            for (path, ticket, body) in [
                (
                    "/api/skills",
                    "fixture-skills-ticket",
                    json!([{
                        "name":"web-research",
                        "category":"research",
                        "description":"Research the web",
                        "enabled":true,
                        "usage":12,
                        "provenance":"hub"
                    }])
                    .to_string(),
                ),
                (
                    "/api/tools/toolsets",
                    "fixture-toolsets-ticket",
                    json!([{
                        "name":"web",
                        "label":"Web Search",
                        "description":"Search the web",
                        "enabled":true,
                        "configured":true,
                        "tools":["web_search","web_extract"]
                    }])
                    .to_string(),
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept capability request");
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                loop {
                    let count = stream.read(&mut buffer).expect("read HTTP request");
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).expect("HTTP request is ASCII");
                assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains(&format!("x-hermes-gateway-ticket: {ticket}\r\n"))
                );
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                    .expect("write capability response");
            }
        });

        let snapshot = read_capabilities(
            &format!("http://{address}"),
            "fixture-skills-ticket",
            "fixture-toolsets-ticket",
        )
        .expect("read capabilities through the local HTTP fixture");
        assert_eq!(snapshot.skills.len(), 1);
        assert_eq!(snapshot.skills[0].name, "web-research");
        assert_eq!(snapshot.skills[0].provenance.as_deref(), Some("hub"));
        assert_eq!(snapshot.toolsets.len(), 1);
        assert_eq!(snapshot.toolsets[0].label, "Web Search");
        assert_eq!(snapshot.toolsets[0].tools, ["web_search", "web_extract"]);
        server.join().expect("capability fixture server exits");
    }

    #[test]
    fn toolset_update_uses_profile_ticket_put_and_validates_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture HTTP endpoint");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            for response_enabled in [false, true] {
                let (mut stream, _) = listener.accept().expect("accept toolset update");
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                let mut body_range = None;
                loop {
                    let count = stream.read(&mut buffer).expect("read toolset request");
                    if count == 0 {
                        panic!("toolset request closed before the full body arrived");
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if let Some(offset) =
                        request.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        if body_range.is_none() {
                            let body_offset = offset + 4;
                            let headers = String::from_utf8_lossy(&request[..offset]);
                            let content_length = headers
                                .lines()
                                .find_map(|line| {
                                    let (key, value) = line.split_once(':')?;
                                    key.eq_ignore_ascii_case("content-length").then(|| {
                                        value.trim().parse::<usize>().expect("content length")
                                    })
                                })
                                .expect("content length header");
                            body_range = Some((body_offset, content_length));
                        }
                        let (body_offset, content_length) = body_range.expect("parsed headers");
                        if request.len() >= body_offset + content_length {
                            break;
                        }
                    }
                }
                let (body_offset, content_length) = body_range.expect("complete HTTP request");
                let request_text = String::from_utf8_lossy(&request);
                assert!(
                    request_text.starts_with("PUT /api/tools/toolsets/web%20search HTTP/1.1\r\n")
                );
                assert!(
                    request_text
                        .to_ascii_lowercase()
                        .contains("x-hermes-gateway-ticket: fixture-native-http-ticket\r\n")
                );
                assert!(
                    request_text
                        .to_ascii_lowercase()
                        .contains("content-type: application/json\r\n")
                );
                let body: serde_json::Value =
                    serde_json::from_slice(&request[body_offset..body_offset + content_length])
                        .expect("parse toggle body");
                assert_eq!(body, json!({"enabled":false}));
                let response = json!({
                    "ok":true,
                    "name":"web search",
                    "enabled":response_enabled
                })
                .to_string();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response)
                    .expect("write toolset response");
            }
        });

        let origin = format!("http://{address}");
        let updated =
            request_toolset_enabled(&origin, "fixture-native-http-ticket", "web search", false)
                .expect("decode toolset update receipt");
        assert!(!updated);
        let mismatched =
            request_toolset_enabled(&origin, "fixture-native-http-ticket", "web search", false);
        assert!(
            mismatched.is_err(),
            "mismatched state receipt must fail closed"
        );
        server.join().expect("toolset fixture server exits");
    }

    #[test]
    fn create_then_submit_uses_runtime_and_stored_session_ids_for_their_distinct_roles() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let create: serde_json::Value = match socket.read().expect("read create request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse create request"),
                other => panic!("expected create request, got {other:?}"),
            };
            assert_eq!(create["method"], "session.create");
            assert_eq!(create["params"]["source"], "desktop");
            assert_eq!(create["params"]["cols"], 96);
            assert_eq!(create["params"]["profile"], "fixture-profile");
            assert_eq!(create["params"]["request_id"], "create-1");
            let create_rpc_id = create["id"].as_u64().expect("create RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":create_rpc_id,"result":{
                        "session_id":"runtime-1","stored_session_id":"stored-1"
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("acknowledge create");

            let submit: serde_json::Value = match socket.read().expect("read submit request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse submit request"),
                other => panic!("expected submit request, got {other:?}"),
            };
            assert_eq!(submit["method"], "prompt.submit");
            assert_eq!(
                submit["params"],
                json!({"session_id":"runtime-1","profile":"fixture-profile","submission_id":"submit-1","text":"hello Hermes"})
            );
            let submit_rpc_id = submit["id"].as_u64().expect("submit RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":submit_rpc_id,"result":{
                        "admission_id":"admission-1","ref":{"session_id":"runtime-1"},
                        "status":"started","sequence":1,"outcome":null,"authority_epoch":1,
                        "execution_generation":1
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("acknowledge submit");
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::CreateSession {
                request_id: "create-1".into(),
                params: json!({"cols":96,"source":"desktop","profile":"fixture-profile","request_id":"create-1"}),
            })
            .expect("queue session create");
        let created = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("session created event");
        let GatewayEvent::SessionCreated(Ok(created)) = created else {
            panic!("expected successful created session event");
        };
        assert_eq!(created.request_id, "create-1");
        assert_eq!(created.runtime_session_id, "runtime-1");
        assert_eq!(created.stored_session_id, "stored-1");
        command_tx
            .send(GatewayCommand::SubmitPrompt {
                session_id: created.runtime_session_id,
                profile: Some("fixture-profile".into()),
                submission_id: "submit-1".into(),
                text: "hello Hermes".into(),
            })
            .expect("queue initial prompt");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("prompt result"),
            GatewayEvent::PromptSubmitted { session_id, submission_id, result: Ok(PromptSubmitReceipt::Admitted { admission_id, status, .. }) }
                if session_id == "runtime-1" && submission_id == "submit-1" && admission_id == "admission-1" && status == "started"
        ));

        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn connectors_read_uses_the_pinned_catalog_list_and_accounts_rpc_contracts() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            for (expected_method, response) in [
                (
                    "connectors.list",
                    json!({"available":true,"connectors":[{
                        "connector":"linear","enabled":true,"connected":true,
                        "connection_status":"active","status_reason":null,"gateway_disabled_tools":[]
                    }]}),
                ),
                (
                    "connectors.catalog",
                    json!({"connectors":[{
                        "slug":"linear","name":"Linear","description":"Project work","category":"Productivity"
                    }]}),
                ),
                (
                    "connectors.accounts",
                    json!({"accounts":[{
                        "connection_id":"connection-1","connector":"linear","status":"active",
                        "label":"Linear workspace","alias":"Engineering","active":true,
                        "disabled":false,"created_at":"2026-11-15T00:00:00Z","updated_at":"2026-11-15T00:00:00Z"
                    }]}),
                ),
            ] {
                let request: serde_json::Value =
                    match socket.read().expect("read connector request") {
                        Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                        other => panic!("expected connector request, got {other:?}"),
                    };
                assert_eq!(request["method"], expected_method);
                if expected_method == "connectors.list" {
                    assert_eq!(request["params"]["owner"], json!({"type":"account"}));
                }
                let id = request["id"].as_u64().expect("numeric RPC id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":response})
                            .to_string()
                            .into(),
                    ))
                    .expect("send connector response");
            }
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadConnectors {
                request_id: "connectors-1".into(),
            })
            .expect("queue connector inventory read");
        let event = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("connector inventory result");
        let GatewayEvent::ConnectorsLoaded {
            request_id,
            result: Ok(snapshot),
        } = event
        else {
            panic!("expected successful connector snapshot");
        };
        assert_eq!(request_id, "connectors-1");
        assert!(snapshot.available);
        assert_eq!(snapshot.connectors[0].connector, "linear");
        assert_eq!(snapshot.catalog[0].name, "Linear");
        assert_eq!(snapshot.accounts[0].alias.as_deref(), Some("Engineering"));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn plugin_auto_update_read_and_write_preserve_profile_scope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            for (enabled, auto_apply) in [(None, false), (Some(true), true)] {
                let request: serde_json::Value = match socket.read().expect("read plugin setting") {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                    other => panic!("expected setting request, got {other:?}"),
                };
                assert_eq!(request["method"], "plugins.manage");
                assert_eq!(request["params"]["profile"], "fixture-profile");
                assert_eq!(request["params"]["action"], "auto_update");
                if let Some(enabled) = enabled {
                    assert_eq!(request["params"]["enable"], enabled);
                } else {
                    assert!(request["params"].get("enable").is_none());
                }
                let id = request["id"].as_u64().expect("numeric RPC id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":{"auto_apply":auto_apply,"check_hours":12}})
                            .to_string()
                            .into(),
                    ))
                    .expect("send auto-update receipt");
            }
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadAgentPluginAutoUpdate {
                request_id: "auto-read".into(),
                profile: Some("fixture-profile".into()),
            })
            .expect("queue auto-update read");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("read result"),
            GatewayEvent::AgentPluginAutoUpdateLoaded {
                request_id,
                profile: Some(profile),
                result: Ok(AgentPluginAutoUpdateSettings { auto_apply: false, check_hours: 12 }),
            } if request_id == "auto-read" && profile == "fixture-profile"
        ));
        command_tx
            .send(GatewayCommand::SetAgentPluginAutoUpdate {
                request_id: "auto-write".into(),
                profile: Some("fixture-profile".into()),
                enabled: true,
            })
            .expect("queue auto-update write");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("write result"),
            GatewayEvent::AgentPluginAutoUpdateSet {
                request_id,
                profile: Some(profile),
                enabled: true,
                result: Ok(AgentPluginAutoUpdateSettings { auto_apply: true, check_hours: 12 }),
            } if request_id == "auto-write" && profile == "fixture-profile"
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn plugin_update_all_uses_profile_scope_and_preserves_summary_rows() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read update-all request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse request"),
                other => panic!("expected update-all request, got {other:?}"),
            };
            assert_eq!(request["method"], "plugins.manage");
            assert_eq!(
                request["params"],
                json!({"action":"update_all","profile":"fixture-profile"})
            );
            let id = request["id"].as_u64().expect("numeric RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"results":[
                        {"name":"Weather Tools","status":"updated"},
                        {"name":"Research Helper","status":"needs_review","detail":"Capabilities changed"},
                        {"name":"Browser Helper","status":"failed","detail":"Repository unavailable"}
                    ]}})
                    .to_string()
                    .into(),
                ))
                .expect("send update-all result");
            socket.close(None).expect("close fixture WebSocket");
        });
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let fixture_url = format!("ws://{address}/api/ws");
        let actor = thread::spawn(move || {
            agent_plugin_update_all_worker(command_rx, event_tx, || {
                tungstenite::connect(&fixture_url)
                    .map(|(socket, _)| socket)
                    .map_err(|error| format!("fixture connect failed: {error}"))
            })
        });
        command_tx
            .send(AgentPluginUpdateAllCommand {
                request_id: "update-all-1".into(),
                profile: Some("fixture-profile".into()),
            })
            .expect("queue update-all");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("update-all receipt"),
            GatewayEvent::AgentPluginUpdateAll { request_id, profile:Some(profile), result:Ok(rows) }
                if request_id == "update-all-1" && profile == "fixture-profile" && rows.len() == 3
                    && rows[0].status == "updated" && rows[1].status == "needs_review"
                    && rows[2].detail.as_deref() == Some("Repository unavailable")
        ));
        drop(command_tx);
        actor.join().expect("update-all actor exits");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn long_plugin_update_all_does_not_block_primary_gateway_actor() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let (update_started_tx, update_started_rx) = mpsc::channel();
        let (release_update_tx, release_update_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (main_stream, _) = listener.accept().expect("accept primary connection");
            let main_socket = tungstenite::accept(main_stream).expect("accept primary WebSocket");
            let (update_stream, _) = listener.accept().expect("accept update connection");
            let update_socket =
                tungstenite::accept(update_stream).expect("accept update WebSocket");

            let main = thread::spawn(move || {
                let mut socket = main_socket;
                let request: serde_json::Value = match socket.read().expect("read list request") {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse list request"),
                    other => panic!("expected list request, got {other:?}"),
                };
                assert_eq!(request["method"], "plugins.manage");
                assert_eq!(request["params"], json!({"action":"list","profile":"work"}));
                let id = request["id"].as_u64().expect("list RPC id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":{"plugins":[],"user_count":0,"bundled_count":0}})
                            .to_string()
                            .into(),
                    ))
                    .expect("send list result");
                socket.close(None).expect("close primary connection");
            });

            let update = thread::spawn(move || {
                let mut socket = update_socket;
                let request: serde_json::Value = match socket.read().expect("read update request") {
                    Message::Text(text) => {
                        serde_json::from_str(&text).expect("parse update request")
                    }
                    other => panic!("expected update request, got {other:?}"),
                };
                assert_eq!(request["method"], "plugins.manage");
                assert_eq!(
                    request["params"],
                    json!({"action":"update_all","profile":"work"})
                );
                update_started_tx
                    .send(())
                    .expect("signal long update started");
                release_update_rx.recv().expect("release long update");
                let id = request["id"].as_u64().expect("update RPC id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":{"results":[]}})
                            .to_string()
                            .into(),
                    ))
                    .expect("send update result");
                socket.close(None).expect("close update connection");
            });

            main.join().expect("primary fixture handler exits");
            update.join().expect("update fixture handler exits");
        });

        let fixture_url = format!("ws://{address}/api/ws");
        let (mut primary_socket, _) =
            tungstenite::connect(&fixture_url).expect("connect primary fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = primary_socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure primary fixture read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let primary_events = event_tx.clone();
        let primary_actor =
            thread::spawn(move || gateway_actor(&mut primary_socket, command_rx, primary_events));
        let update_url = fixture_url.clone();
        let update_events = event_tx;
        let update_actor = thread::spawn(move || {
            agent_plugin_update_all_worker(update_rx, update_events, || {
                tungstenite::connect(&update_url)
                    .map(|(socket, _)| socket)
                    .map_err(|error| format!("fixture connect failed: {error}"))
            })
        });

        update_tx
            .send(AgentPluginUpdateAllCommand {
                request_id: "long-update".into(),
                profile: Some("work".into()),
            })
            .expect("queue long update");
        update_started_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("long update entered server handler");

        let primary_request_started = Instant::now();
        command_tx
            .send(GatewayCommand::LoadAgentPlugins {
                request_id: "parallel-list".into(),
                profile: Some("work".into()),
            })
            .expect("queue primary plugin list");
        let primary_result = event_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("primary list result arrives while update is held");
        let primary_latency_ms = primary_request_started.elapsed().as_secs_f64() * 1000.0;
        println!("primary_plugin_list_latency_ms={primary_latency_ms:.3}");
        assert!(matches!(
            primary_result,
            GatewayEvent::AgentPluginsLoaded { request_id, profile:Some(profile), result:Ok(snapshot) }
                if request_id == "parallel-list" && profile == "work" && snapshot.plugins.is_empty()
        ));

        let update_job_started = Instant::now();
        release_update_tx.send(()).expect("release held update");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).expect("update result arrives"),
            GatewayEvent::AgentPluginUpdateAll { request_id, profile:Some(profile), result:Ok(rows) }
                if request_id == "long-update" && profile == "work" && rows.is_empty()
        ));
        println!(
            "held_update_rpc_duration_ms={:.3}",
            update_job_started.elapsed().as_secs_f64() * 1000.0
        );
        drop(command_tx);
        drop(update_tx);
        primary_actor.join().expect("primary actor exits");
        update_actor.join().expect("update actor exits");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn agent_plugins_use_profile_scoped_list_and_keyed_toggle_contracts() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let list: serde_json::Value = match socket.read().expect("read plugin list request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse list request"),
                other => panic!("expected list request, got {other:?}"),
            };
            assert_eq!(list["method"], "plugins.manage");
            assert_eq!(list["params"], json!({"action":"list","profile":"work"}));
            let list_id = list["id"].as_u64().expect("list RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":list_id,"result":{
                        "user_count":1,"bundled_count":0,"plugins":[{
                            "name":"Legacy Helper","key":"legacy/helper","version":"1.2.0",
                            "description":"Profile plugin","source":"user","status":"disabled",
                            "portable":false,"has_desktop_half":false,"servers":[]
                        }]
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send list result");

            let toggle: serde_json::Value = match socket.read().expect("read plugin toggle request")
            {
                Message::Text(text) => serde_json::from_str(&text).expect("parse toggle"),
                other => panic!("expected toggle request, got {other:?}"),
            };
            assert_eq!(toggle["method"], "plugins.manage");
            assert_eq!(
                toggle["params"],
                json!({"action":"toggle","key":"legacy/helper","enable":true,"profile":"work"})
            );
            let toggle_id = toggle["id"].as_u64().expect("toggle RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":toggle_id,"result":{
                        "ok":true,"restart_required":false,"gateway_reloaded":true,
                        "plugin":{"name":"Legacy Helper","key":"legacy/helper","version":"1.2.0",
                            "description":"Profile plugin","source":"user","status":"enabled",
                            "portable":false,"has_desktop_half":false,"servers":[]}
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send toggle result");
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::LoadAgentPlugins {
                request_id: "plugin-list-1".into(),
                profile: Some("work".into()),
            })
            .expect("queue plugin list");
        let loaded = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("plugin inventory result");
        assert!(matches!(
            loaded,
            GatewayEvent::AgentPluginsLoaded { request_id, profile: Some(profile), result: Ok(snapshot) }
                if request_id == "plugin-list-1"
                    && profile == "work"
                    && snapshot.plugins.len() == 1
                    && snapshot.plugins[0].key == "legacy/helper"
                    && snapshot.user_count == 1
        ));
        command_tx
            .send(GatewayCommand::SetAgentPluginEnabled {
                request_id: "plugin-toggle-1".into(),
                key: "legacy/helper".into(),
                enabled: true,
                profile: Some("work".into()),
            })
            .expect("queue profile plugin toggle");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("toggle receipt"),
            GatewayEvent::AgentPluginToggled { request_id, key, enabled: true,
                profile: Some(profile),
                result: Ok(AgentPluginToggleReceipt { plugin, restart_required: false, gateway_reloaded: true }) }
                if request_id == "plugin-toggle-1" && key == "legacy/helper" && profile == "work" && plugin.status == "enabled"
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn catalog_plugin_install_is_pinned_to_the_inspected_commit_and_profile() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let inspected_sha = "e404b6816b20237a82f6fbe2749b5a576cf75dba";
        let server_sha = inspected_sha.to_owned();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let inspect: serde_json::Value = match socket.read().expect("read inspect") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse inspect"),
                other => panic!("expected inspect request, got {other:?}"),
            };
            assert_eq!(inspect["method"], "plugins.manage");
            assert_eq!(
                inspect["params"],
                json!({
                    "action":"inspect","catalog_name":"sample-plugin",
                    "identifier":"https://github.com/example/sample#plugin",
                    "profile":"work"
                })
            );
            let id = inspect["id"].as_u64().expect("inspect RPC id");
            socket.send(Message::Text(json!({"jsonrpc":"2.0","id":id,"result":{
                "ok":true,"agent":true,"desktop":false,"agent_name":"sample-plugin",
                "sha":server_sha,"warnings":["Review requested permissions"],"insecure":false
            }}).to_string().into())).expect("reply to inspect");

            let install: serde_json::Value = match socket.read().expect("read install") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse install"),
                other => panic!("expected install request, got {other:?}"),
            };
            assert_eq!(install["method"], "plugins.manage");
            assert_eq!(
                install["params"],
                json!({
                    "action":"install","catalog_name":"sample-plugin",
                    "identifier":"https://github.com/example/sample#plugin",
                    "enable":true,"force":false,"inspected_sha":inspected_sha,"profile":"work"
                })
            );
            let id = install["id"].as_u64().expect("install RPC id");
            socket.send(Message::Text(json!({"jsonrpc":"2.0","id":id,"result":{
                "ok":true,"plugin_name":"sample-plugin","enabled":true,
                "warnings":[],"missing_env":[],"activation":{"live_now":{"skills":[{"name":"sample:lookup"}]}}
            }}).to_string().into())).expect("reply to install");
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::InspectCatalogPlugin {
                request_id: "inspect-1".into(),
                catalog_name: "sample-plugin".into(),
                identifier: "https://github.com/example/sample#plugin".into(),
                profile: Some("work".into()),
            })
            .expect("queue inspect");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("inspection result"),
            GatewayEvent::AgentPluginInspected { request_id, catalog_name, profile:Some(profile),
                result:Ok(AgentPluginInspection { agent:true, desktop:false, sha:Some(sha), insecure:false, .. }) }
                if request_id == "inspect-1" && catalog_name == "sample-plugin" && profile == "work" && sha == inspected_sha)
        );
        command_tx
            .send(GatewayCommand::InstallCatalogPlugin {
                request_id: "install-1".into(),
                catalog_name: "sample-plugin".into(),
                identifier: "https://github.com/example/sample#plugin".into(),
                inspected_sha: inspected_sha.into(),
                profile: Some("work".into()),
            })
            .expect("queue install");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("install result"),
            GatewayEvent::AgentPluginInstalled { request_id, catalog_name, profile:Some(profile),
                result:Ok(AgentPluginInstallReceipt { plugin_name, enabled:true, skills, .. }) }
                if request_id == "install-1" && catalog_name == "sample-plugin" && profile == "work"
                    && plugin_name == "sample-plugin" && skills == vec!["sample:lookup"])
        );
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn agent_plugin_update_preserves_consent_and_scan_override_sequence() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let expected = [
                (
                    json!({"action":"update","name":"Weather","profile":"work"}),
                    json!({"consent_required":true,"sha":"abcdef0123456789","delta_lines":["+ tool: weather.alerts"]}),
                ),
                (
                    json!({"action":"update","name":"Weather","accept_capabilities":true,"profile":"work"}),
                    json!({"scan_blocked":true,"scan_verdict":"caution","scan_findings":[{"severity":"high","category":"execution","file":"plugin.py","line":7,"description":"dynamic execution"}]}),
                ),
                (
                    json!({"action":"update","name":"Weather","accept_capabilities":true,"force":true,"profile":"work"}),
                    json!({"ok":true,"unchanged":false}),
                ),
            ];
            for (expected_params, result) in expected {
                let request: serde_json::Value = match socket.read().expect("read update request") {
                    Message::Text(text) => {
                        serde_json::from_str(&text).expect("parse update request")
                    }
                    other => panic!("expected update request, got {other:?}"),
                };
                assert_eq!(request["method"], "plugins.manage");
                assert_eq!(request["params"], expected_params);
                let id = request["id"].as_u64().expect("update RPC id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":result})
                            .to_string()
                            .into(),
                    ))
                    .expect("reply to update");
            }
            socket.close(None).expect("close fixture WebSocket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        let issue = |request_id: &str, accept_capabilities: bool, force: bool| {
            GatewayCommand::UpdateAgentPlugin {
                request_id: request_id.into(),
                name: "Weather".into(),
                profile: Some("work".into()),
                accept_capabilities,
                force,
            }
        };
        command_tx
            .send(issue("update-1", false, false))
            .expect("queue initial update");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("consent review"),
            GatewayEvent::AgentPluginUpdated { request_id, accept_capabilities:false, force:false,
                result:Ok(AgentPluginUpdateOutcome::Consent { sha, delta_lines, .. }), .. }
                if request_id == "update-1" && sha == "abcdef01" && delta_lines == vec!["+ tool: weather.alerts"])
        );
        command_tx
            .send(issue("update-2", true, false))
            .expect("queue consented update");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("scan review"),
            GatewayEvent::AgentPluginUpdated { request_id, accept_capabilities:true, force:false,
                result:Ok(AgentPluginUpdateOutcome::Scan { verdict, findings }), .. }
                if request_id == "update-2" && verdict == "caution" && findings.len() == 1 && findings[0].file == "plugin.py")
        );
        command_tx
            .send(issue("update-3", true, true))
            .expect("queue explicit scan override");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("update receipt"),
            GatewayEvent::AgentPluginUpdated { request_id, accept_capabilities:true, force:true,
                result:Ok(AgentPluginUpdateOutcome::Applied { unchanged:false }), .. } if request_id == "update-3")
        );
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn agent_plugin_remove_uses_profile_scoped_manage_contract() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket
                .read()
                .expect("read plugin remove request")
            {
                Message::Text(text) => serde_json::from_str(&text).expect("parse remove request"),
                other => panic!("expected remove request, got {other:?}"),
            };
            assert_eq!(request["method"], "plugins.manage");
            assert_eq!(
                request["params"],
                json!({"action":"remove","name":"Legacy Helper","profile":"work"})
            );
            let id = request["id"].as_u64().expect("remove RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"ok":true}})
                        .to_string()
                        .into(),
                ))
                .expect("send removal result");
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::RemoveAgentPlugin {
                request_id: "plugin-remove-1".into(),
                name: "Legacy Helper".into(),
                profile: Some("work".into()),
            })
            .expect("queue profile plugin removal");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("removal receipt"),
            GatewayEvent::AgentPluginRemoved { request_id, name, profile: Some(profile), result: Ok(()) }
                if request_id == "plugin-remove-1" && name == "Legacy Helper" && profile == "work"
        ));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn prompt_receipt_rejects_wrong_destination_and_accepts_voice_stop() {
        let wrong = json!({
            "admission_id":"admission-1","ref":{"session_id":"other"},"status":"started"
        });
        assert_eq!(
            parse_prompt_submit_receipt(&wrong, "runtime-1")
                .expect_err("receipt for another session must not settle this submission"),
            "Hermes prompt.submit receipt destination mismatch"
        );
        assert!(matches!(
            parse_prompt_submit_receipt(&json!({"voice_stopped":true}), "runtime-1"),
            Ok(PromptSubmitReceipt::VoiceStopped)
        ));
        assert!(parse_prompt_submit_receipt(&json!({"accepted":true}), "runtime-1").is_err());
    }

    #[test]
    fn unknown_admission_resolution_uses_generation_profile_and_exact_admission() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read resolve request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse resolve request"),
                other => panic!("expected resolve request, got {other:?}"),
            };
            assert_eq!(request["method"], "prompt.resolve_unknown");
            assert_eq!(
                request["params"],
                json!({
                    "session_id":"runtime-1","profile":"work",
                    "admission_id":"admission-lost","execution_generation":12
                })
            );
            let id = request["id"].as_u64().expect("RPC id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{
                        "session_id":"runtime-1","ref":{"session_id":"runtime-1"},
                        "admission_id":"admission-lost","status":"terminal"
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send resolve receipt");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        let (commands, receiver) = mpsc::channel();
        let (events, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, receiver, events));
        commands
            .send(GatewayCommand::ResolveUnknownPrompt {
                session_id: "runtime-1".into(),
                profile: Some("work".into()),
                admission_id: "admission-lost".into(),
                execution_generation: 12,
            })
            .expect("queue resolve");
        assert!(
            matches!(event_rx.recv_timeout(Duration::from_secs(3)).expect("resolve event"),
            GatewayEvent::UnknownPromptResolved { session_id, admission_id, result:Ok(status) }
                if session_id=="runtime-1" && admission_id=="admission-lost" && status=="terminal")
        );
        drop(commands);
        actor.join().expect("actor exits");
        server.join().expect("fixture exits");
    }

    #[test]
    fn prompt_submit_keeps_the_runtime_session_scope_and_forwards_stream_events() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read prompt request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse prompt request"),
                other => panic!("expected text request, got {other:?}"),
            };
            assert_eq!(request["method"], "prompt.submit");
            assert_eq!(
                request["params"],
                json!({"session_id":"runtime-1","submission_id":"submission-1","text":"hello Hermes"})
            );
            let id = request["id"].as_u64().expect("JSON-RPC request id");
            for (kind, payload) in [
                ("message.start", json!({"turn_id":"turn-1"})),
                ("message.delta", json!({"text":"Hello "})),
            ] {
                socket
                    .send(Message::Text(
                        json!({
                            "jsonrpc":"2.0","method":"event",
                            "params":{"type":kind,"session_id":"runtime-1","payload":payload}
                        })
                        .to_string()
                        .into(),
                    ))
                    .expect("send stream event");
            }
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{
                        "admission_id":"admission-1","ref":{"session_id":"runtime-1"},
                        "status":"started","sequence":1,"outcome":null,"authority_epoch":1,
                        "execution_generation":1
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("acknowledge prompt");
            socket.send(Message::Text(json!({
                "jsonrpc":"2.0","method":"event",
                "params":{"type":"message.complete","session_id":"runtime-1","payload":{"admission_id":"admission-1","text":"Hello Hermes!","status":"complete"}}
            }).to_string().into())).expect("send completion");
            socket.close(None).expect("close fixture WebSocket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::SubmitPrompt {
                session_id: "runtime-1".into(),
                profile: None,
                submission_id: "submission-1".into(),
                text: "hello Hermes".into(),
            })
            .expect("queue prompt");

        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(
                event_rx
                    .recv_timeout(Duration::from_secs(10))
                    .expect("gateway event"),
            );
        }
        assert!(
            matches!(&seen[0], GatewayEvent::ServerEvent(event) if event["type"] == "message.start")
        );
        assert!(
            matches!(&seen[1], GatewayEvent::ServerEvent(event) if event["type"] == "message.delta")
        );
        assert!(
            matches!(&seen[2], GatewayEvent::PromptSubmitted { session_id, submission_id, result: Ok(PromptSubmitReceipt::Admitted { admission_id, status, .. }) } if session_id == "runtime-1" && submission_id == "submission-1" && admission_id == "admission-1" && status == "started")
        );
        assert!(
            matches!(&seen[3], GatewayEvent::ServerEvent(event) if event["type"] == "message.complete")
        );

        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn session_resume_routes_profile_and_preserves_it_in_transcript() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let request: serde_json::Value = match socket.read().expect("read resume request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse resume request"),
                other => panic!("expected resume request, got {other:?}"),
            };
            assert_eq!(request["method"], "session.resume");
            assert_eq!(request["params"]["session_id"], "stored-1");
            assert_eq!(request["params"]["profile"], "work");
            let id = request["id"].as_u64().expect("JSON-RPC request id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{
                        "session_id":"runtime-1","stored_session_id":"stored-1",
                        "message_count":1,"messages":[{"role":"assistant","content":"hi"}]
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send resume snapshot");
            socket.close(None).expect("close fixture socket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        let (events, _event_receiver) = mpsc::channel();
        let transcript = resume_session(&mut socket, "stored-1", Some("work".into()), &events)
            .expect("resume scoped session");
        assert_eq!(transcript.session_id, "stored-1");
        assert_eq!(transcript.profile.as_deref(), Some("work"));
        assert_eq!(transcript.runtime_session_id, "runtime-1");
        assert_eq!(transcript.message_count, 1);
        assert_eq!(transcript.messages[0]["content"], "hi");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn session_resume_streams_inflight_events_and_recovers_after_refusal() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");

            let first: serde_json::Value = match socket.read().expect("read first resume") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse first resume"),
                other => panic!("expected first resume request, got {other:?}"),
            };
            assert_eq!(first["method"], "session.resume");
            assert_eq!(first["params"]["session_id"], "stored-live-1");
            assert_eq!(first["params"]["profile"], "work");
            socket
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0", "id":first["id"],
                        "error":{"code":-32000,"message":"temporarily unavailable","data":{"reason":"temporarily_unavailable"}}
                    })
                    .to_string()
                    .into(),
                ))
                .expect("refuse first resume");

            let retry: serde_json::Value = match socket.read().expect("read resume retry") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse resume retry"),
                other => panic!("expected resume retry, got {other:?}"),
            };
            assert_eq!(retry["method"], "session.resume");
            assert_eq!(retry["params"]["session_id"], "stored-live-1");
            assert_eq!(retry["params"]["profile"], "work");
            let retry_id = retry["id"].as_u64().expect("JSON-RPC retry id");

            for (kind, payload) in [
                ("message.start", json!({"turn_id":"turn-live-1"})),
                ("message.delta", json!({"text":"Resumed response"})),
            ] {
                socket
                    .send(Message::Text(
                        json!({
                            "jsonrpc":"2.0", "method":"event",
                            "params":{"type":kind,"session_id":"runtime-live-1","payload":payload}
                        })
                        .to_string()
                        .into(),
                    ))
                    .expect("send in-flight event while resume is pending");
            }
            socket
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0", "id":retry_id,
                        "result":{
                            "session_id":"runtime-live-1", "stored_session_id":"stored-live-1",
                            "message_count":2,
                            "messages":[{"role":"assistant","content":"Resumed response"}]
                        }
                    })
                    .to_string()
                    .into(),
                ))
                .expect("return resumed transcript snapshot");
            socket
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0", "method":"event",
                        "params":{"type":"message.complete","session_id":"runtime-live-1","payload":{"turn_id":"turn-live-1"}}
                    })
                    .to_string()
                    .into(),
                ))
                .expect("send in-flight completion after resume snapshot");
            socket.close(None).expect("close fixture socket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("configure client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        command_tx
            .send(GatewayCommand::ResumeSession {
                request_id: "resume-first".into(),
                session_id: "stored-live-1".into(),
                profile: Some("work".into()),
            })
            .expect("queue first resume");
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(3)).expect("resume refusal"),
            GatewayEvent::SessionResumed {
                request_id, session_id, profile, result: Err(error)
            } if request_id == "resume-first"
                && session_id == "stored-live-1"
                && profile.as_deref() == Some("work")
                && error.contains("temporarily_unavailable")
        ));

        command_tx
            .send(GatewayCommand::ResumeSession {
                request_id: "resume-retry".into(),
                session_id: "stored-live-1".into(),
                profile: Some("work".into()),
            })
            .expect("queue retry after refusal");
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(
                event_rx
                    .recv_timeout(Duration::from_secs(3))
                    .expect("retry stream or resume event"),
            );
        }
        assert!(matches!(
            &seen[0],
            GatewayEvent::ServerEvent(event) if event["type"] == "message.start"
                && event["session_id"] == "runtime-live-1"
        ));
        assert!(matches!(
            &seen[1],
            GatewayEvent::ServerEvent(event) if event["type"] == "message.delta"
                && event["payload"]["text"] == "Resumed response"
        ));
        assert!(matches!(
            &seen[2],
            GatewayEvent::SessionResumed {
                request_id, session_id, profile,
                result: Ok(transcript)
            } if request_id == "resume-retry"
                && session_id == "stored-live-1"
                && profile.as_deref() == Some("work")
                && transcript.runtime_session_id == "runtime-live-1"
                && transcript.message_count == 2
        ));
        assert!(matches!(
            &seen[3],
            GatewayEvent::ServerEvent(event) if event["type"] == "message.complete"
        ));

        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn runtime_session_title_uses_profile_and_retains_stored_identity() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let resume: serde_json::Value = match socket.read().expect("read rename snapshot") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse resume request"),
                other => panic!("expected resume request, got {other:?}"),
            };
            assert_eq!(resume["method"], "session.resume");
            assert_eq!(resume["params"]["session_id"], "runtime-1");
            assert_eq!(resume["params"]["profile"], "work");
            assert_eq!(resume["params"]["omit_messages"], true);
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":resume["id"],"result":{
                        "session_id":"runtime-1","revision":7
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send rename snapshot");
            let request: serde_json::Value = match socket.read().expect("read rename mutation") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse mutation"),
                other => panic!("expected mutation, got {other:?}"),
            };
            assert_eq!(request["method"], "session.mutate");
            assert_eq!(request["params"]["session_id"], "runtime-1");
            assert_eq!(request["params"]["profile"], "work");
            assert_eq!(request["params"]["expected_revision"], 7);
            assert_eq!(request["params"]["operation"], "rename");
            assert_eq!(request["params"]["payload"]["title"], "Draft name");
            assert_eq!(request["params"]["request_id"], "rename-1");
            let id = request["id"].as_u64().expect("JSON-RPC request id");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"session_id":"runtime-1","revision":8,"title":"Draft name"}})
                        .to_string()
                        .into(),
                ))
                .expect("send title receipt");
            socket.close(None).expect("close fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        let (commands, command_receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_receiver, events));
        commands
            .send(GatewayCommand::RenameRuntimeSession {
                request_id: "rename-1".into(),
                stored_session_id: "stored-1".into(),
                runtime_session_id: "runtime-1".into(),
                profile: Some("work".into()),
                title: "Draft name".into(),
                retry_only: false,
            })
            .expect("queue title request");
        assert!(matches!(
            event_receiver.recv_timeout(Duration::from_secs(3)).expect("title event"),
            GatewayEvent::SessionRenamed { session_id, profile, result: Ok(title), .. }
                if session_id == "stored-1" && profile.as_deref() == Some("work") && title == "Draft name"
        ));
        drop(commands);
        actor.join().expect("gateway actor exits");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn uncertain_rename_retries_exact_canonical_mutation() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            let resume: serde_json::Value = match socket.read().expect("read snapshot request") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse snapshot request"),
                other => panic!("expected snapshot request, got {other:?}"),
            };
            assert_eq!(resume["method"], "session.resume");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":resume["id"],"result":{
                        "session_id":"runtime-1","revision":7
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send snapshot");
            let first: serde_json::Value = match socket.read().expect("read first mutation") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse first mutation"),
                other => panic!("expected mutation, got {other:?}"),
            };
            assert_eq!(first["method"], "session.mutate");
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":first["id"],"result":{
                        "session_id":"runtime-1","revision":8
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send incomplete receipt");
            let second: serde_json::Value = match socket.read().expect("read exact retry") {
                Message::Text(text) => serde_json::from_str(&text).expect("parse retry"),
                other => panic!("expected retry mutation, got {other:?}"),
            };
            assert_eq!(second["method"], "session.mutate");
            assert_eq!(second["params"], first["params"]);
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":second["id"],"result":{
                        "session_id":"runtime-1","revision":8,"title":"Draft name"
                    }})
                    .to_string()
                    .into(),
                ))
                .expect("send exact retry receipt");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture");
        let (events, _) = mpsc::channel();
        let mut retained = HashMap::new();
        let mut rpc_id = 5;
        let first = rename_runtime_session_rpc(
            &mut socket,
            &events,
            &mut rpc_id,
            &mut retained,
            "rename-1",
            "runtime-1",
            Some("work"),
            "Draft name",
            false,
        );
        assert_eq!(
            first.expect_err("incomplete receipt is uncertain"),
            "Hermes rename receipt omitted its confirmed title"
        );
        assert!(retained.contains_key("rename-1"));
        let title = rename_runtime_session_rpc(
            &mut socket,
            &events,
            &mut rpc_id,
            &mut retained,
            "rename-1",
            "runtime-1",
            Some("work"),
            "Draft name",
            true,
        )
        .expect("exact retry confirms title");
        assert_eq!(title, "Draft name");
        assert!(!retained.contains_key("rename-1"));
        assert_eq!(
            rename_runtime_session_rpc(
                &mut socket,
                &events,
                &mut rpc_id,
                &mut retained,
                "rename-1",
                "runtime-1",
                Some("work"),
                "Draft name",
                true,
            )
            .expect_err("retry without retained tuple must stop before a new snapshot"),
            "Original Hermes rename request is unavailable after reconnect"
        );
        server.join().expect("fixture server exits");
    }

    #[test]
    fn session_interrupt_uses_runtime_id_and_accepts_already_idle_result() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            for (expected_session, status) in [
                ("runtime-active", "interrupted"),
                ("runtime-finished", "not_interrupted"),
            ] {
                let request: serde_json::Value = match socket.read().expect("read interrupt") {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse interrupt"),
                    other => panic!("expected text request, got {other:?}"),
                };
                assert_eq!(request["method"], "session.interrupt");
                assert_eq!(request["params"], json!({"session_id":expected_session}));
                let id = request["id"].as_u64().expect("JSON-RPC request id");
                socket
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":{"status":status}})
                            .to_string()
                            .into(),
                    ))
                    .expect("respond to interrupt");
            }
            socket.close(None).expect("close fixture socket");
        });

        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        for (request_id, session_id) in [
            ("interrupt-active", "runtime-active"),
            ("interrupt-finished", "runtime-finished"),
        ] {
            command_tx
                .send(GatewayCommand::InterruptSession {
                    request_id: request_id.into(),
                    session_id: session_id.into(),
                    profile: None,
                })
                .expect("queue interrupt");
        }

        for (request_id, session_id, interrupted) in [
            ("interrupt-active", "runtime-active", true),
            ("interrupt-finished", "runtime-finished", false),
        ] {
            let event = event_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("interrupt result event");
            assert!(matches!(
                event,
                GatewayEvent::SessionInterrupted {
                    request_id: actual_request,
                    session_id: actual_session,
                    result: Ok(actual_interrupted),
                } if actual_request == request_id
                    && actual_session == session_id
                    && actual_interrupted == interrupted
            ));
        }
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn approval_response_is_generation_scoped_and_rejects_mismatched_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            for (prompt_id, response_prompt_id) in [("prompt-1", "prompt-1"), ("prompt-2", "stale")]
            {
                let request: serde_json::Value =
                    match socket.read().expect("read approval response") {
                        Message::Text(text) => serde_json::from_str(&text).expect("parse response"),
                        other => panic!("expected text request, got {other:?}"),
                    };
                assert_eq!(request["method"], "approval.respond");
                assert_eq!(
                    request["params"],
                    json!({
                        "session_id":"runtime-session",
                        "execution_generation":7,
                        "prompt_id":prompt_id,
                        "choice":"once"
                    })
                );
                let id = request["id"].as_u64().expect("JSON-RPC request id");
                socket
                    .send(Message::Text(
                        json!({
                            "jsonrpc":"2.0","id":id,
                            "result":{"status":"resolved","prompt_id":response_prompt_id}
                        })
                        .to_string()
                        .into(),
                    ))
                    .expect("answer approval response");
            }
            socket.close(None).expect("close fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        for (request_id, prompt_id) in [("approval-1", "prompt-1"), ("approval-2", "prompt-2")] {
            command_tx
                .send(GatewayCommand::RespondApproval {
                    request_id: request_id.into(),
                    session_id: "runtime-session".into(),
                    profile: None,
                    execution_generation: 7,
                    prompt_id: prompt_id.into(),
                    choice: "once".into(),
                })
                .expect("queue approval response");
        }
        let first = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("first approval result");
        assert!(matches!(first, GatewayEvent::ApprovalResponded {
            request_id, prompt_id, choice, result: Ok(()), ..
        } if request_id == "approval-1" && prompt_id == "prompt-1" && choice == "once"));
        let second = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("second approval result");
        assert!(matches!(second, GatewayEvent::ApprovalResponded {
            request_id, prompt_id, result: Err(_), ..
        } if request_id == "approval-2" && prompt_id == "prompt-2"));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }

    #[test]
    fn clarify_response_is_generation_scoped_and_validates_prompt_receipt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture gateway");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket");
            for (prompt_id, answer, response_prompt_id) in [
                ("question-1", "[\"red\",\"blue\"]", "question-1"),
                ("question-2", "", "stale"),
            ] {
                let request: serde_json::Value = match socket.read().expect("read clarify response")
                {
                    Message::Text(text) => serde_json::from_str(&text).expect("parse response"),
                    other => panic!("expected text request, got {other:?}"),
                };
                assert_eq!(request["method"], "clarify.respond");
                assert_eq!(
                    request["params"],
                    json!({
                        "session_id":"runtime-session",
                        "execution_generation":11,
                        "prompt_id":prompt_id,
                        "answer":answer
                    })
                );
                let id = request["id"].as_u64().expect("JSON-RPC request id");
                socket
                    .send(Message::Text(
                        json!({
                            "jsonrpc":"2.0","id":id,
                            "result":{"status":"resolved","prompt_id":response_prompt_id}
                        })
                        .to_string()
                        .into(),
                    ))
                    .expect("answer clarify response");
            }
            socket.close(None).expect("close fixture socket");
        });
        let (mut socket, _) =
            tungstenite::connect(format!("ws://{address}/api/ws")).expect("connect fixture client");
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(50)))
                .expect("configure fixture client read timeout");
        }
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let actor = thread::spawn(move || gateway_actor(&mut socket, command_rx, event_tx));
        for (request_id, prompt_id, answer) in [
            ("clarify-1", "question-1", "[\"red\",\"blue\"]"),
            ("clarify-2", "question-2", ""),
        ] {
            command_tx
                .send(GatewayCommand::RespondClarify {
                    request_id: request_id.into(),
                    session_id: "runtime-session".into(),
                    profile: None,
                    execution_generation: 11,
                    prompt_id: prompt_id.into(),
                    answer: answer.into(),
                })
                .expect("queue clarify response");
        }
        let first = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("first clarify result");
        assert!(matches!(first, GatewayEvent::ClarifyResponded {
            request_id, prompt_id, answer, result: Ok(()), ..
        } if request_id == "clarify-1" && prompt_id == "question-1" && answer == "[\"red\",\"blue\"]"));
        let second = event_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("second clarify result");
        assert!(matches!(second, GatewayEvent::ClarifyResponded {
            request_id, prompt_id, result: Err(_), ..
        } if request_id == "clarify-2" && prompt_id == "question-2"));
        drop(command_tx);
        actor.join().expect("gateway actor exits after peer close");
        server.join().expect("fixture server exits");
    }
}
