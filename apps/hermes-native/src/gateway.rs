//! Attaches to Hermes' authoritative gateway using its supported private ticket and canonical RPC.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap, HashSet},
    io::{self, Read, Write},
    net::TcpStream,
    process::Command,
    process::Stdio,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use tungstenite::{ClientRequestBuilder, Message, client::connect_with_config, http::Uri};
use zeroize::{Zeroize, Zeroizing};

const CANONICAL_PROTOCOL: &str = "hermes-gateway-v1";
const TICKET_PROTOCOL_PREFIX: &str = "hermes-gateway-ticket.";
const GATEWAY_HTTP_RESPONSE_LIMIT: u64 = 512 * 1024;

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

#[derive(Clone, Debug, Deserialize)]
pub struct SessionSummary {
    pub id: String,
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
    ArchivedSessionsLoaded {
        request_id: String,
        result: Result<Vec<SessionSummary>, String>,
    },
    CapabilitiesLoaded {
        request_id: String,
        result: Result<CapabilitiesSnapshot, String>,
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
    Disconnected(String),
}

enum GatewayCommand {
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
    LoadSessions,
    LoadArchivedSessions {
        request_id: String,
    },
    SearchSessions {
        request_id: String,
        query: String,
    },
    LoadCapabilities {
        request_id: String,
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
}

type GatewaySocket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

pub struct GatewayDiscovery {
    status_receiver: Receiver<Result<ConnectedGateway, String>>,
    event_receiver: Receiver<GatewayEvent>,
    command_sender: mpsc::Sender<GatewayCommand>,
    plugin_update_all_sender: mpsc::Sender<AgentPluginUpdateAllCommand>,
    http_sender: mpsc::Sender<GatewayHttpCommand>,
}

struct AgentPluginUpdateAllCommand {
    request_id: String,
    profile: Option<String>,
}

impl GatewayDiscovery {
    pub fn start() -> Self {
        let (status_sender, status_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        let (command_sender, command_receiver) = mpsc::channel();
        let (plugin_update_all_sender, plugin_update_all_receiver) = mpsc::channel();
        let (http_sender, http_receiver) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-gateway-session-list".into())
            .spawn(move || match attach_gateway() {
                Ok((connected, mut socket)) => {
                    let search_endpoint = connected.endpoint.clone();
                    let search_events = event_sender.clone();
                    let plugin_update_all_endpoint = connected.endpoint.clone();
                    let plugin_update_all_events = event_sender.clone();
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
                    gateway_actor(&mut socket, command_receiver, event_sender);
                }
                Err(error) => {
                    let _ = status_sender.send(Err(error));
                }
            })
            .expect("failed to start Hermes gateway discovery worker");
        let _ = http_sender.send(GatewayHttpCommand::LoadSessions);
        Self {
            status_receiver,
            event_receiver,
            command_sender,
            plugin_update_all_sender,
            http_sender,
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

    pub fn load_capabilities(&self, request_id: String) -> Result<(), String> {
        self.http_sender
            .send(GatewayHttpCommand::LoadCapabilities { request_id })
            .map_err(|_| "Hermes profile HTTP worker has closed".to_owned())
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

fn gateway_actor(
    socket: &mut GatewaySocket,
    commands: Receiver<GatewayCommand>,
    events: mpsc::Sender<GatewayEvent>,
) {
    let mut next_request_id = 5_u64;
    let mut retained_renames: HashMap<String, serde_json::Value> = HashMap::new();
    loop {
        match commands.try_recv() {
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
            GatewayHttpCommand::LoadSessions => {
                GatewayEvent::SessionsLoaded(load_sessions_http(&endpoint))
            }
            GatewayHttpCommand::LoadArchivedSessions { request_id } => {
                GatewayEvent::ArchivedSessionsLoaded {
                    request_id,
                    result: load_archived_sessions_http(&endpoint),
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
        };
        if events.send(event).is_err() {
            return;
        }
    }
}

fn load_sessions_http(endpoint: &GatewayEndpoint) -> Result<Vec<SessionSummary>, String> {
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

fn load_archived_sessions_http(endpoint: &GatewayEndpoint) -> Result<Vec<SessionSummary>, String> {
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

fn load_capabilities_http(endpoint: &GatewayEndpoint) -> Result<CapabilitiesSnapshot, String> {
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

fn set_toolset_enabled_http(
    endpoint: &GatewayEndpoint,
    name: &str,
    enabled: bool,
) -> Result<bool, String> {
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
    if !is_bare_loopback_origin(api_origin) || !url.starts_with(api_origin) {
        return Err("Hermes API request target is outside the verified loopback origin".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(5)))
        .max_redirects(0)
        .build()
        .into();
    let response = agent
        .get(url)
        .header("X-Hermes-Gateway-Ticket", ticket)
        .header("Accept", "application/json")
        .call()
        .map_err(|_| "Hermes API read request failed".to_owned())?;
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
    use serde_json::json;
    use std::net::TcpListener;

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
            "title":"Recent session",
            "started_at":1731543892.75
        }))
        .expect("decode current session list row");
        assert_eq!(session.started_at, 1_731_543_892.75);
    }

    #[test]
    fn archived_session_pages_dedupe_pinned_backfills_and_use_fixed_offsets() {
        let mut offsets = Vec::new();
        let sessions = collect_archived_session_pages(|offset| {
            offsets.push(offset);
            let rows = if offset == 0 {
                vec![
                    SessionSummary {
                        id: "archived-1".into(),
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
                        id: "live-pin".into(),
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
                        id: "archived-2".into(),
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
                        id: "live-pin".into(),
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
                        id: "archived-1".into(),
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
        assert_eq!(sessions[1].title, "");
        assert_eq!(sessions[1].source, "");
        assert_eq!(sessions[1].last_active, None);
        assert!(!sessions[1].pinned);
        assert!(!sessions[1].unread);
        server.join().expect("fixture server exits");
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
