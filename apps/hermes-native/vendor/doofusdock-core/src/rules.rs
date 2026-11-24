//! Versioned, deterministic window-rule matching and transaction preparation.
//!
//! Rules inspect an immutable context and produce a revision-bound plan. They
//! never mutate `LayoutDocument`; supported structural effects become ordinary
//! `Operation`s and use `DockingService`'s atomic transaction path.
use crate::{
    CommandId, ContainerModality, DockError, DockResult, DockingService, ErrorCode, EventBatch,
    FrameworkEvent, FrameworkEventKind, HostCapabilities, LayoutDocument,
    LayoutStrategyConfiguration, ModalityId, Node, NodeId, NodeKind, Operation, PanelId, Rect,
    SCROLLABLE_COLUMNS_MODALITY_ID, ScrollableColumnsConfiguration, Size, TransactionResult,
    TransitionId, WorkspaceId, validate_layout,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_RULE_BYTES: usize = 1024 * 1024;
const MAX_RULES: usize = 256;
const MAX_ACTIONS_PER_RULE: usize = 32;
const MAX_RULE_EVENT_KINDS: usize = 16;
const MAX_CONSUMER_EVENT_HISTORY: usize = 1024;
const MAX_SUPPRESSED_RULE_COMMANDS: usize = 256;
const MAX_RULE_FILE_BYTES: usize = MAX_RULE_BYTES + 4096;
static RULE_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct WindowRuleStore {
    directory: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowRuleFile {
    schema_version: u32,
    checksum: String,
    rules: WindowRuleSet,
}

impl WindowRuleStore {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn save(&self, workspace: &WorkspaceId, rules: &WindowRuleSet) -> DockResult<()> {
        rules.validate()?;
        let path = self.path_for(workspace)?;
        fs::create_dir_all(&self.directory)
            .map_err(|error| persistence_error("Create rule directory", error, &path))?;
        let encoded = encode_rule_file(rules)?;

        let backup = backup_path(&path);
        if path.exists() && read_rule_file(&path).is_ok() {
            let backup_temporary = temporary_path(&backup);
            write_new_file(
                &backup_temporary,
                &fs::read(&path)
                    .map_err(|error| persistence_error("Read previous rule file", error, &path))?,
            )?;
            crate::atomic_replace(&backup_temporary, &backup).map_err(|error| {
                let _ = fs::remove_file(&backup_temporary);
                persistence_error("Replace known-good rule backup", error, &backup)
            })?;
        }

        let temporary = temporary_path(&path);
        write_new_file(&temporary, &encoded)?;
        crate::atomic_replace(&temporary, &path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            persistence_error("Atomically replace rule file", error, &path)
        })
    }

    pub fn load(&self, workspace: &WorkspaceId) -> DockResult<WindowRuleSet> {
        read_rule_file(&self.path_for(workspace)?)
    }

    pub fn recover(&self, workspace: &WorkspaceId) -> DockResult<WindowRuleSet> {
        let primary = self.path_for(workspace)?;
        match read_rule_file(&primary) {
            Ok(rules) => Ok(rules),
            Err(primary_error) => {
                if fs::metadata(&primary)
                    .is_ok_and(|metadata| metadata.len() > MAX_RULE_FILE_BYTES as u64)
                {
                    return Err(DockError::new(
                        ErrorCode::ResourceLimit,
                        "Window-rule primary exceeds its byte limit; backup recovery was not attempted.",
                        primary.display().to_string(),
                    ));
                }
                if let Ok(bytes) = fs::read(&primary)
                    && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
                    && let Some(schema) = value.get("schema_version")
                {
                    let Some(version) = schema.as_u64() else {
                        return Err(DockError::new(
                            ErrorCode::SchemaError,
                            "Window-rule schema version is invalid; backup recovery was not attempted.",
                            schema.to_string(),
                        ));
                    };
                    if version > 1 {
                        return Err(DockError::new(
                            ErrorCode::SchemaError,
                            "Window-rule schema is newer than supported; backup recovery was not attempted.",
                            version.to_string(),
                        ));
                    }
                }
                let backup = backup_path(&primary);
                let rules = read_rule_file(&backup).map_err(|backup_error| {
                    DockError::new(
                        ErrorCode::PersistenceError,
                        format!(
                            "Rule recovery failed: primary={primary_error}; backup={backup_error}"
                        ),
                        workspace.0.clone(),
                    )
                })?;
                self.save(workspace, &rules)?;
                Ok(rules)
            }
        }
    }

    fn path_for(&self, workspace: &WorkspaceId) -> DockResult<PathBuf> {
        if !valid_token(&workspace.0, 128) {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Unsafe workspace ID for window-rule persistence.",
                workspace.0.clone(),
            ));
        }
        Ok(self
            .directory
            .join(format!("window-rules-{}.json", workspace.0)))
    }
}

pub(crate) fn encode_rule_file(rules: &WindowRuleSet) -> DockResult<Vec<u8>> {
    let rule_bytes = serde_json::to_vec(rules).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "window-rules-store",
        )
    })?;
    let file = WindowRuleFile {
        schema_version: 1,
        checksum: format!("{:016x}", checksum64(&rule_bytes)),
        rules: rules.clone(),
    };
    let bytes = serde_json::to_vec(&file).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "window-rules-store",
        )
    })?;
    if bytes.len() > MAX_RULE_FILE_BYTES {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Window-rule file exceeds its byte limit.",
            bytes.len().to_string(),
        ));
    }
    Ok(bytes)
}

fn read_rule_file(path: &Path) -> DockResult<WindowRuleSet> {
    let metadata =
        fs::metadata(path).map_err(|error| persistence_error("Inspect rule file", error, path))?;
    if metadata.len() > MAX_RULE_FILE_BYTES as u64 {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Window-rule file exceeds its byte limit.",
            path.display().to_string(),
        ));
    }
    let bytes = fs::read(path).map_err(|error| persistence_error("Read rule file", error, path))?;
    let json = std::str::from_utf8(&bytes).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            path.display().to_string(),
        )
    })?;
    decode_window_rule_file_json(json)
}

pub(crate) fn decode_window_rule_file_json(json: &str) -> DockResult<WindowRuleSet> {
    if json.len() > MAX_RULE_FILE_BYTES {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Window-rule file exceeds its byte limit.",
            json.len().to_string(),
        ));
    }
    let file: WindowRuleFile = serde_json::from_str(json).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "window-rules-file",
        )
    })?;
    if file.schema_version != 1 {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Unsupported window-rule file schema.",
            file.schema_version.to_string(),
        ));
    }
    let encoded_rules = serde_json::to_vec(&file.rules).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "window-rules-store",
        )
    })?;
    if file.checksum != format!("{:016x}", checksum64(&encoded_rules)) {
        return Err(DockError::new(
            ErrorCode::PersistenceError,
            "Window-rule checksum does not match.",
            "window-rules-file.checksum",
        ));
    }
    let rules_json = serde_json::to_string(&file.rules).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "window-rules-file.rules",
        )
    })?;
    WindowRuleSet::from_json(&rules_json)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> DockResult<()> {
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    result.map_err(|error| {
        let _ = fs::remove_file(path);
        persistence_error("Write temporary rule file", error, path)
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    let sequence = RULE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("window-rules");
    path.with_file_name(format!(
        "{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ))
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn checksum64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn persistence_error(step: &str, error: std::io::Error, path: &Path) -> DockError {
    DockError::new(
        ErrorCode::PersistenceError,
        format!("{step} failed: {error}"),
        path.display().to_string(),
    )
}

fn default_enabled() -> bool {
    true
}

fn default_rule_schema() -> u32 {
    1
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowRuleSet {
    #[serde(default = "default_rule_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub rules: Vec<WindowRule>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowRule {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub overrides: Vec<String>,
    /// Empty means the rule participates in explicit/manual evaluations only.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub event_kinds: BTreeSet<FrameworkEventKind>,
    #[serde(default)]
    pub predicate: WindowRulePredicate,
    pub actions: Vec<WindowRuleAction>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowRulePredicate {
    #[serde(default)]
    pub panel_id: Option<PanelId>,
    #[serde(default)]
    pub panel_role: Option<String>,
    #[serde(default)]
    pub panel_metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
    #[serde(default)]
    pub workspace_tags_all: BTreeSet<String>,
    #[serde(default)]
    pub workspace_tags_any: BTreeSet<String>,
    #[serde(default)]
    pub group_id: Option<NodeId>,
    #[serde(default)]
    pub modality: Option<ModalityId>,
    #[serde(default)]
    pub focused: Option<bool>,
    #[serde(default)]
    pub monitor_role: Option<String>,
    #[serde(default)]
    pub viewport_class: Option<String>,
    #[serde(default)]
    pub user_metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub host_capabilities_all: BTreeSet<String>,
    #[serde(default)]
    pub host_capabilities_any: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WindowRuleAction {
    Placement(PlacementIntent),
    PreferredSize(Size),
    MinimumSize(Size),
    MaximumSize(Size),
    AssignMonitor { monitor: String },
    WorkspaceMembership { workspaces: BTreeSet<WorkspaceId> },
    AutoHide { edge: crate::DockEdge },
    SetPinned { pinned: bool },
    Focus(FocusBehavior),
    ZOrder(ZOrderIntent),
    DefaultModality(ContainerModality),
    TransitionPreference { transition: TransitionId },
    SelectLayout { layout: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "placement", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlacementIntent {
    Root,
    Center {
        target_panel: PanelId,
    },
    CenterGroup {
        panels: Vec<PanelId>,
        target_panel: PanelId,
    },
    Split {
        target: NodeId,
        orientation: crate::Orientation,
        before: bool,
        ratio: f64,
    },
    Float {
        window: crate::WindowId,
        geometry: Rect,
        monitor: String,
        scale: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusBehavior {
    Preserve,
    FocusPanel,
    FocusWorkspace,
    DoNotFocus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZOrderIntent {
    Normal,
    Raise,
    Lower,
    AlwaysOnTop,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowRuleContext {
    pub revision: u64,
    pub panel_id: PanelId,
    pub panel_role: String,
    pub panel_metadata: BTreeMap<String, String>,
    pub workspace_id: WorkspaceId,
    pub workspace_tags: BTreeSet<String>,
    pub group_id: Option<NodeId>,
    pub modality: Option<ModalityId>,
    pub focused: bool,
    pub monitor_role: Option<String>,
    pub viewport_class: Option<String>,
    pub user_metadata: BTreeMap<String, String>,
    pub host_capabilities: HostCapabilities,
}

/// Host-observed predicates kept separate from canonical panel/workspace data.
/// A host supplies these by the exact workspace revision and panel identity
/// when evaluating triggered rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowRuleHostContext {
    pub group_id: Option<NodeId>,
    pub modality: Option<ModalityId>,
    pub focused: bool,
    pub monitor_role: Option<String>,
    pub viewport_class: Option<String>,
    pub user_metadata: BTreeMap<String, String>,
}

impl WindowRuleContext {
    pub fn for_panel(
        document: &LayoutDocument,
        panel: &PanelId,
        host_capabilities: HostCapabilities,
    ) -> DockResult<Self> {
        validate_layout(document)?;
        host_capabilities.validate()?;
        let descriptor = document.descriptor(panel).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Window-rule context panel is not registered.",
                panel.0.clone(),
            )
        })?;
        Ok(Self {
            revision: document.revision,
            panel_id: panel.clone(),
            panel_role: descriptor.role.clone(),
            panel_metadata: descriptor.metadata.clone(),
            workspace_id: document.workspace.clone(),
            workspace_tags: descriptor.workspace_memberships.clone(),
            group_id: None,
            modality: None,
            focused: false,
            monitor_role: None,
            viewport_class: None,
            user_metadata: BTreeMap::new(),
            host_capabilities,
        })
    }

    /// Capture every registered panel against one immutable document revision.
    pub fn for_document(
        document: &LayoutDocument,
        host_capabilities: HostCapabilities,
    ) -> DockResult<Vec<Self>> {
        validate_layout(document)?;
        host_capabilities.validate()?;
        Ok(document
            .panels
            .iter()
            .map(|descriptor| Self {
                revision: document.revision,
                panel_id: descriptor.id.clone(),
                panel_role: descriptor.role.clone(),
                panel_metadata: descriptor.metadata.clone(),
                workspace_id: document.workspace.clone(),
                workspace_tags: descriptor.workspace_memberships.clone(),
                group_id: None,
                modality: None,
                focused: false,
                monitor_role: None,
                viewport_class: None,
                user_metadata: BTreeMap::new(),
                host_capabilities: host_capabilities.clone(),
            })
            .collect())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleDiagnosticKind {
    Disabled,
    NotMatched,
    Matched,
    Overridden,
    Conflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowRuleDiagnostic {
    pub rule_id: String,
    pub kind: RuleDiagnosticKind,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedRuleAction {
    pub rule_id: String,
    pub priority: i32,
    pub action: WindowRuleAction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowRuleEvaluation {
    pub base_revision: u64,
    pub matched_rules: Vec<String>,
    pub actions: Vec<ResolvedRuleAction>,
    pub diagnostics: Vec<WindowRuleDiagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowRulePlan {
    pub base_revision: u64,
    pub panel: PanelId,
    pub canonical_operations: Vec<Operation>,
    pub host_intents: Vec<ResolvedRuleAction>,
    pub required_capabilities: BTreeSet<String>,
    pub missing_capabilities: BTreeSet<String>,
    pub diagnostics: Vec<WindowRuleDiagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowRuleCommit {
    pub transaction: Option<TransactionResult>,
    /// Revision after the canonical portion of this plan completed.
    pub resulting_revision: u64,
    pub host_intents: Vec<ResolvedRuleAction>,
    pub acknowledged_host_intents: Vec<HostIntentAcknowledgement>,
    acknowledged: Vec<bool>,
    pub diagnostics: Vec<WindowRuleDiagnostic>,
}

/// Poll-driven event bridge. It evaluates only rules that explicitly declare
/// the event kind and never executes host callbacks or commits operations.
#[derive(Clone, Debug, Default)]
pub struct WindowRuleEventConsumer {
    processed: VecDeque<u64>,
    suppressed_commands: VecDeque<(CommandId, Option<u64>)>,
    last_sequence: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TriggeredRuleEvaluation {
    pub event_sequence: u64,
    pub event_kind: FrameworkEventKind,
    pub panel: PanelId,
    pub evaluation: WindowRuleEvaluation,
}

impl WindowRuleEventConsumer {
    /// Mark a command ID as rule-originated at every revision. Use this for a
    /// causal host-action chain whose observations may span revisions. A
    /// command ID must uniquely identify that causal action while retained;
    /// use `suppress_command_at_revision` for a single-transaction effect.
    pub fn suppress_command(&mut self, command: CommandId) -> DockResult<()> {
        self.suppress_command_scope(command, None)
    }

    /// Mark a rule-originated command at its resulting canonical revision.
    /// Every fact emitted by that transaction is consumed without rule
    /// reevaluation, while later reuse of the same command ID is unaffected.
    pub fn suppress_command_at_revision(
        &mut self,
        command: CommandId,
        revision: u64,
    ) -> DockResult<()> {
        self.suppress_command_scope(command, Some(revision))
    }

    fn suppress_command_scope(
        &mut self,
        command: CommandId,
        revision: Option<u64>,
    ) -> DockResult<()> {
        if command.is_empty() || command.0.len() > 128 {
            return Err(rule_error("Suppressed command ID is invalid.", &command.0));
        }
        let entry = (command, revision);
        if !self.suppressed_commands.contains(&entry) {
            self.suppressed_commands.push_back(entry);
            while self.suppressed_commands.len() > MAX_SUPPRESSED_RULE_COMMANDS {
                self.suppressed_commands.pop_front();
            }
        }
        Ok(())
    }

    /// Evaluate a batch against contexts captured at each event's exact
    /// canonical revision. Overflow and snapshot drift fail closed.
    pub fn evaluate_batch(
        &mut self,
        rules: &WindowRuleSet,
        batch: &EventBatch,
        contexts: &[WindowRuleContext],
    ) -> DockResult<Vec<TriggeredRuleEvaluation>> {
        rules.validate()?;
        if batch.lost_events > 0 {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Rule event evaluation cannot continue after event loss.",
                batch.lost_events.to_string(),
            ));
        }
        if batch.events.len() > 256 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Rule event batch exceeds its bounded event limit.",
                batch.events.len().to_string(),
            ));
        }

        let mut staged_processed = self.processed.clone();
        let mut prior_sequence = None;
        let mut results = Vec::new();
        for event in &batch.events {
            validate_rule_event(event)?;
            if prior_sequence.is_some_and(|sequence| event.sequence <= sequence) {
                return Err(rule_error(
                    "Rule event batch must be strictly sequence ordered.",
                    &event.sequence.to_string(),
                ));
            }
            prior_sequence = Some(event.sequence);
            if event.sequence <= self.last_sequence || staged_processed.contains(&event.sequence) {
                continue;
            }
            let suppressed = self.suppressed_commands.iter().any(|(command, revision)| {
                command == &event.command
                    && revision.is_none_or(|revision| revision == event.revision)
            });
            if !suppressed {
                let matching_contexts = contexts
                    .iter()
                    .filter(|context| context.revision == event.revision)
                    .collect::<Vec<_>>();
                if matching_contexts.is_empty()
                    && rules
                        .rules
                        .iter()
                        .any(|rule| rule.event_kinds.contains(&event.kind))
                {
                    return Err(DockError::new(
                        ErrorCode::Conflict,
                        "Triggered rule evaluation requires a context from the event revision.",
                        format!("sequence={}:revision={}", event.sequence, event.revision),
                    ));
                }
                let event_panel = event
                    .payload
                    .get("panel")
                    .map(|panel| PanelId::from(panel.clone()));
                for context in matching_contexts.into_iter().filter(|context| {
                    event_panel
                        .as_ref()
                        .is_none_or(|panel| panel == &context.panel_id)
                }) {
                    let mut event_context = context.clone();
                    if event.kind == FrameworkEventKind::FocusChanged {
                        event_context.focused = true;
                    }
                    let evaluation = rules.evaluate_for_event(&event_context, event.kind)?;
                    if !evaluation.matched_rules.is_empty() {
                        results.push(TriggeredRuleEvaluation {
                            event_sequence: event.sequence,
                            event_kind: event.kind,
                            panel: context.panel_id.clone(),
                            evaluation,
                        });
                    }
                }
            }
            staged_processed.push_back(event.sequence);
            while staged_processed.len() > MAX_CONSUMER_EVENT_HISTORY {
                staged_processed.pop_front();
            }
        }
        self.processed = staged_processed;
        if let Some(sequence) = prior_sequence {
            self.last_sequence = self.last_sequence.max(sequence);
        }
        Ok(results)
    }
}

fn validate_rule_event(event: &FrameworkEvent) -> DockResult<()> {
    if event.sequence == 0
        || event.id.0 != event.sequence
        || (event.revision == 0
            && !matches!(
                event.kind,
                FrameworkEventKind::FocusChanged
                    | FrameworkEventKind::NativeViewportFocused
                    | FrameworkEventKind::NativeViewportCloseRequested
                    | FrameworkEventKind::NativeViewportGeometryObserved
                    | FrameworkEventKind::NativeRootViewportGeometryObserved
                    | FrameworkEventKind::MonitorChanged
            ))
    {
        return Err(rule_error(
            "Rule event identity or revision is invalid.",
            &event.sequence.to_string(),
        ));
    }
    if event.command.is_empty() || event.command.0.len() > 128 || event.payload.len() > 32 {
        return Err(rule_error(
            "Rule event exceeds its validated bounds.",
            &event.sequence.to_string(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostIntentOutcome {
    /// The host reports that the requested effect has taken effect.
    Applied,
    /// The host accepted the request for dispatch, but the native result has
    /// not yet been observed. This is an adapter receipt, not success proof.
    Queued,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostIntentAcknowledgement {
    pub index: usize,
    pub rule_id: String,
    pub observed_revision: u64,
    pub outcome: HostIntentOutcome,
    pub detail: String,
}

/// Result reported by a host after it handles one rule-derived intention.
/// `Queued` is terminal for adapter retry (the command must not be replayed),
/// while its native effect remains unconfirmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostIntentResult {
    pub observed_revision: u64,
    pub outcome: HostIntentOutcome,
    pub detail: String,
}

/// Toolkit-independent adapter boundary for effects the canonical layout
/// model does not own. Implementations run after the canonical batch commits.
pub trait WindowRuleHostAdapter {
    fn apply(
        &mut self,
        intent: &ResolvedRuleAction,
        expected_revision: u64,
    ) -> DockResult<HostIntentResult>;
}

impl WindowRuleSet {
    pub fn validate(&self) -> DockResult<()> {
        if self.schema_version != 1 || self.rules.len() > MAX_RULES {
            return Err(DockError::new(
                if self.rules.len() > MAX_RULES {
                    ErrorCode::ResourceLimit
                } else {
                    ErrorCode::SchemaError
                },
                "Window-rule schema version or rule count is unsupported.",
                format!("version={}:count={}", self.schema_version, self.rules.len()),
            ));
        }
        let mut rules = BTreeMap::new();
        for rule in &self.rules {
            if !valid_token(&rule.id, 128)
                || rule.priority.unsigned_abs() > 1_000_000
                || rule.actions.is_empty()
                || rule.actions.len() > MAX_ACTIONS_PER_RULE
                || rule.event_kinds.len() > MAX_RULE_EVENT_KINDS
                || rule.overrides.len() > MAX_RULES
            {
                return Err(rule_error(
                    "Rule ID, priority, actions, or overrides are invalid.",
                    &rule.id,
                ));
            }
            if rules.insert(rule.id.as_str(), rule).is_some() {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Window-rule IDs must be unique.",
                    &rule.id,
                ));
            }
            validate_predicate(&rule.predicate, &rule.id)?;
            let mut keys = BTreeSet::new();
            for action in &rule.actions {
                validate_action(action, &rule.id)?;
                if !keys.insert(action.conflict_key()) {
                    return Err(rule_error(
                        "A rule cannot define conflicting actions for the same policy.",
                        &rule.id,
                    ));
                }
            }
            let mut overrides = BTreeSet::new();
            for other in &rule.overrides {
                if other == &rule.id || !valid_token(other, 128) || !overrides.insert(other) {
                    return Err(rule_error(
                        "Rule override references must be unique valid IDs and cannot target itself.",
                        &rule.id,
                    ));
                }
            }
        }
        for rule in &self.rules {
            for other in &rule.overrides {
                if !rules.contains_key(other.as_str()) {
                    return Err(rule_error(
                        "Rule override references an unknown rule.",
                        &rule.id,
                    ));
                }
            }
        }
        validate_override_acyclic(&self.rules)?;
        Ok(())
    }

    pub fn to_json(&self) -> DockResult<String> {
        self.validate()?;
        let json = serde_json::to_string(self).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "window-rules-export",
            )
        })?;
        if json.len() > MAX_RULE_BYTES {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Window-rule export exceeds its byte limit.",
                json.len().to_string(),
            ));
        }
        Ok(json)
    }

    /// Import version 0 snapshots by applying version-1 defaults, then validate.
    pub fn from_json(json: &str) -> DockResult<Self> {
        if json.len() > MAX_RULE_BYTES {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Window-rule import exceeds its byte limit.",
                json.len().to_string(),
            ));
        }
        let mut rules: Self = serde_json::from_str(json).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "window-rules-import",
            )
        })?;
        if rules.schema_version == 0 {
            rules.schema_version = 1;
        }
        rules.validate()?;
        Ok(rules)
    }

    pub fn evaluate(&self, context: &WindowRuleContext) -> DockResult<WindowRuleEvaluation> {
        self.evaluate_filtered(context, None)
    }

    pub fn evaluate_for_event(
        &self,
        context: &WindowRuleContext,
        kind: FrameworkEventKind,
    ) -> DockResult<WindowRuleEvaluation> {
        self.evaluate_filtered(context, Some(kind))
    }

    fn evaluate_filtered(
        &self,
        context: &WindowRuleContext,
        event_kind: Option<FrameworkEventKind>,
    ) -> DockResult<WindowRuleEvaluation> {
        self.validate()?;
        context.host_capabilities.validate()?;
        if (!context.panel_role.is_empty() && !valid_token(&context.panel_role, 128))
            || !valid_rule_metadata(&context.panel_metadata)
            || !valid_rule_metadata(&context.user_metadata)
        {
            return Err(rule_error(
                "Rule evaluation context metadata is invalid or too large.",
                &context.panel_id.0,
            ));
        }
        let mut diagnostics = Vec::new();
        let mut matching = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                diagnostics.push(diagnostic(
                    &rule.id,
                    RuleDiagnosticKind::Disabled,
                    "Rule is disabled.",
                ));
            } else if event_kind.is_some_and(|kind| !rule.event_kinds.contains(&kind)) {
                diagnostics.push(diagnostic(
                    &rule.id,
                    RuleDiagnosticKind::NotMatched,
                    "Rule is not subscribed to this event kind.",
                ));
            } else if predicate_matches(&rule.predicate, context) {
                matching.push(rule);
            } else {
                diagnostics.push(diagnostic(
                    &rule.id,
                    RuleDiagnosticKind::NotMatched,
                    "Predicate did not match.",
                ));
            }
        }
        matching.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
        let matched_rules = matching.iter().map(|rule| rule.id.clone()).collect();
        for rule in &matching {
            diagnostics.push(diagnostic(
                &rule.id,
                RuleDiagnosticKind::Matched,
                "Predicate matched.",
            ));
        }
        let mut by_key: BTreeMap<String, Vec<(&WindowRule, &WindowRuleAction)>> = BTreeMap::new();
        for rule in &matching {
            for action in &rule.actions {
                by_key
                    .entry(action.conflict_key())
                    .or_default()
                    .push((rule, action));
            }
        }
        let mut actions = Vec::new();
        for (key, candidates) in by_key {
            let max_priority = candidates
                .iter()
                .map(|(rule, _)| rule.priority)
                .max()
                .unwrap_or(i32::MIN);
            let top: Vec<_> = candidates
                .iter()
                .filter(|(rule, _)| rule.priority == max_priority)
                .copied()
                .collect();
            let unoverridden: Vec<_> = top
                .iter()
                .copied()
                .filter(|(candidate, _)| {
                    !top.iter().any(|(other, _)| {
                        other.id != candidate.id && other.overrides.contains(&candidate.id)
                    })
                })
                .collect();
            let winner = if unoverridden.len() == 1 {
                Some(unoverridden[0])
            } else if !unoverridden.is_empty()
                && unoverridden
                    .iter()
                    .all(|(_, action)| *action == unoverridden[0].1)
            {
                unoverridden
                    .into_iter()
                    .min_by(|(a, _), (b, _)| a.id.cmp(&b.id))
            } else {
                None
            };
            if let Some((winner, action)) = winner {
                actions.push(ResolvedRuleAction {
                    rule_id: winner.id.clone(),
                    priority: winner.priority,
                    action: action.clone(),
                });
                for (rule, _) in candidates.iter().filter(|(rule, _)| rule.id != winner.id) {
                    diagnostics.push(diagnostic(
                        &rule.id,
                        RuleDiagnosticKind::Overridden,
                        format!("Policy '{key}' resolved to rule '{}'.", winner.id),
                    ));
                }
            } else {
                for (rule, _) in &candidates {
                    diagnostics.push(diagnostic(
                        &rule.id,
                        RuleDiagnosticKind::Conflict,
                        format!("Conflicting top-priority actions for policy '{key}'."),
                    ));
                }
            }
        }
        actions.sort_by(|a, b| {
            a.action
                .conflict_key()
                .cmp(&b.action.conflict_key())
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });
        diagnostics.sort_by(|a, b| {
            a.rule_id
                .cmp(&b.rule_id)
                .then_with(|| a.kind.cmp(&b.kind))
                .then_with(|| a.detail.cmp(&b.detail))
        });
        Ok(WindowRuleEvaluation {
            base_revision: context.revision,
            matched_rules,
            actions,
            diagnostics,
        })
    }
}

impl WindowRuleAction {
    fn conflict_key(&self) -> String {
        match self {
            Self::Placement(_) | Self::AutoHide { .. } => "placement".into(),
            Self::PreferredSize(_) => "preferred_size".into(),
            Self::MinimumSize(_) => "minimum_size".into(),
            Self::MaximumSize(_) => "maximum_size".into(),
            Self::AssignMonitor { .. } => "monitor".into(),
            Self::WorkspaceMembership { .. } => "workspace_membership".into(),
            Self::SetPinned { .. } => "pinned".into(),
            Self::Focus(_) => "focus".into(),
            Self::ZOrder(_) => "z_order".into(),
            Self::DefaultModality(_) => "modality".into(),
            Self::TransitionPreference { .. } => "transition".into(),
            Self::SelectLayout { .. } => "layout".into(),
        }
    }
}

impl WindowRulePlan {
    pub fn prepare(
        evaluation: WindowRuleEvaluation,
        context: &WindowRuleContext,
    ) -> DockResult<Self> {
        if evaluation.base_revision != context.revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Rule evaluation context changed before preparation.",
                format!(
                    "evaluation={}:context={}",
                    evaluation.base_revision, context.revision
                ),
            ));
        }
        validate_resolved_size_actions(&evaluation.actions, &context.panel_id)?;
        let mut canonical_operations = Vec::new();
        let mut host_intents = Vec::new();
        let mut required_capabilities = BTreeSet::new();
        for resolved in &evaluation.actions {
            let panel = context.panel_id.clone();
            match &resolved.action {
                WindowRuleAction::Placement(PlacementIntent::Root) => {
                    required_capabilities.insert("workspace:docking".into());
                    canonical_operations.push(Operation::RootPanel { panel })
                }
                WindowRuleAction::Placement(PlacementIntent::Center { target_panel }) => {
                    required_capabilities.insert("workspace:tabs".into());
                    canonical_operations.push(Operation::CenterPanel {
                        panel,
                        target_panel: target_panel.clone(),
                    })
                }
                WindowRuleAction::Placement(PlacementIntent::Split {
                    target,
                    orientation,
                    before,
                    ratio,
                }) => {
                    required_capabilities.insert("workspace:docking".into());
                    canonical_operations.push(Operation::SplitPanel {
                        panel,
                        target: target.clone(),
                        orientation: *orientation,
                        before: *before,
                        ratio: *ratio,
                    });
                }
                WindowRuleAction::Placement(PlacementIntent::CenterGroup {
                    panels,
                    target_panel,
                }) => {
                    required_capabilities.insert("workspace:tabs".into());
                    canonical_operations.push(Operation::CenterPanelGroup {
                        panels: panels.clone(),
                        target_panel: target_panel.clone(),
                    });
                }
                WindowRuleAction::Placement(PlacementIntent::Float {
                    window,
                    geometry,
                    monitor,
                    scale,
                }) => {
                    required_capabilities.insert("native_viewport".into());
                    canonical_operations.push(Operation::FloatPanel {
                        panel,
                        window: window.clone(),
                        geometry: *geometry,
                        monitor: monitor.clone(),
                        scale: *scale,
                    });
                }
                WindowRuleAction::AutoHide { edge } => {
                    required_capabilities.insert("workspace:auto_hide".into());
                    canonical_operations.push(Operation::AutoHide { panel, edge: *edge });
                }
                WindowRuleAction::SetPinned { pinned } => {
                    required_capabilities.insert("workspace:tabs".into());
                    canonical_operations.push(Operation::SetTabPinned {
                        panel,
                        pinned: *pinned,
                    })
                }
                WindowRuleAction::AssignMonitor { .. } => {
                    required_capabilities.insert("monitor:assign".into());
                    host_intents.push(resolved.clone());
                }
                WindowRuleAction::WorkspaceMembership { workspaces } => {
                    required_capabilities.insert("workspace:membership".into());
                    canonical_operations.push(Operation::SetPanelWorkspaceMemberships {
                        panel,
                        tags: workspaces
                            .iter()
                            .map(|workspace| workspace.0.clone())
                            .collect(),
                    });
                }
                WindowRuleAction::PreferredSize(_)
                | WindowRuleAction::MinimumSize(_)
                | WindowRuleAction::MaximumSize(_) => {
                    required_capabilities.insert("panel:size_policy".into());
                    host_intents.push(resolved.clone());
                }
                WindowRuleAction::Focus(_) => {
                    required_capabilities.insert("window:focus".into());
                    host_intents.push(resolved.clone());
                }
                WindowRuleAction::ZOrder(_) => {
                    required_capabilities.insert("window:z_order".into());
                    host_intents.push(resolved.clone());
                }
                WindowRuleAction::DefaultModality(modality) => {
                    required_capabilities.insert(modality_capability(modality).to_owned());
                    if let Some(group) = &context.group_id {
                        canonical_operations.push(Operation::ConvertContainer {
                            container: group.clone(),
                            target: modality.clone(),
                        });
                    } else {
                        host_intents.push(resolved.clone());
                    }
                }
                WindowRuleAction::TransitionPreference { transition } => {
                    required_capabilities.insert(format!("transition:{}", transition.0));
                    host_intents.push(resolved.clone());
                }
                WindowRuleAction::SelectLayout { layout } => {
                    required_capabilities.insert(format!("layout:{layout}"));
                    host_intents.push(resolved.clone());
                }
            }
        }
        let missing_capabilities = required_capabilities
            .difference(&context.host_capabilities.supported)
            .cloned()
            .collect();
        Ok(Self {
            base_revision: evaluation.base_revision,
            panel: context.panel_id.clone(),
            canonical_operations,
            host_intents,
            required_capabilities,
            missing_capabilities,
            diagnostics: evaluation.diagnostics,
        })
    }

    /// Commit all compiled canonical effects in one revision-checked batch.
    /// Host intents are returned for the adapter; callers must not treat them
    /// as applied until the host reports success.
    pub fn commit(
        self,
        service: &mut DockingService,
        command: CommandId,
    ) -> DockResult<WindowRuleCommit> {
        if !self.missing_capabilities.is_empty() {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Window-rule plan requires unavailable host capabilities.",
                self.missing_capabilities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == RuleDiagnosticKind::Conflict)
        {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Conflicting window rules must be resolved before commit.",
                self.panel.0.clone(),
            ));
        }
        let snapshot = service.snapshot();
        if snapshot.revision != self.base_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Window-rule plan became stale before commit.",
                format!("plan={}:service={}", self.base_revision, snapshot.revision),
            ));
        }
        let operations = self
            .canonical_operations
            .into_iter()
            .filter(|operation| !operation_already_satisfied(&snapshot.document, operation))
            .collect::<Vec<_>>();
        let transaction = if operations.is_empty() {
            None
        } else {
            Some(service.transact(self.base_revision, command, operations)?)
        };
        let resulting_revision = transaction
            .as_ref()
            .map_or(self.base_revision, |result| result.document.revision);
        let acknowledged = vec![false; self.host_intents.len()];
        Ok(WindowRuleCommit {
            transaction,
            resulting_revision,
            host_intents: self.host_intents,
            acknowledged_host_intents: Vec::new(),
            acknowledged,
            diagnostics: self.diagnostics,
        })
    }

    /// Commit a rule plan against application-owned document/history state.
    /// This mirrors the service-backed commit contract for hosts whose
    /// canonical state is already owned outside `DockingService`; callers
    /// remain responsible for publishing any surrounding host/service events.
    pub fn commit_document(
        self,
        document: &mut LayoutDocument,
        history: &mut crate::UndoStack,
        command: CommandId,
    ) -> DockResult<WindowRuleCommit> {
        if !self.missing_capabilities.is_empty() {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Window-rule plan requires unavailable host capabilities.",
                self.missing_capabilities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == RuleDiagnosticKind::Conflict)
        {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Conflicting window rules must be resolved before commit.",
                self.panel.0.clone(),
            ));
        }
        if document.revision != self.base_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Window-rule plan became stale before commit.",
                format!("plan={}:document={}", self.base_revision, document.revision),
            ));
        }
        let operations = self
            .canonical_operations
            .into_iter()
            .filter(|operation| !operation_already_satisfied(document, operation))
            .collect::<Vec<_>>();
        let transaction = if operations.is_empty() {
            None
        } else {
            Some(DockingService::transact_document(
                document,
                history,
                self.base_revision,
                command,
                operations,
            )?)
        };
        let resulting_revision = transaction
            .as_ref()
            .map_or(document.revision, |result| result.document.revision);
        let acknowledged = vec![false; self.host_intents.len()];
        Ok(WindowRuleCommit {
            transaction,
            resulting_revision,
            host_intents: self.host_intents,
            acknowledged_host_intents: Vec::new(),
            acknowledged,
            diagnostics: self.diagnostics,
        })
    }
}

impl WindowRuleCommit {
    /// Record the host's result for one deterministic intent slot. This only
    /// acknowledges the adapter outcome; it does not mutate canonical state.
    pub fn acknowledge_host_intent(
        &mut self,
        index: usize,
        observed_revision: u64,
        outcome: HostIntentOutcome,
        detail: impl Into<String>,
    ) -> DockResult<&HostIntentAcknowledgement> {
        let intent = self.host_intents.get(index).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Window-rule host intent index is not present in this commit.",
                index.to_string(),
            )
        })?;
        if observed_revision != self.resulting_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Window-rule host acknowledgement is stale.",
                format!(
                    "expected={}:observed={observed_revision}",
                    self.resulting_revision
                ),
            ));
        }
        if self.acknowledged[index] {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Window-rule host intent has already been acknowledged.",
                index.to_string(),
            ));
        }
        let detail = detail.into();
        if detail.len() > 512 || detail.chars().any(char::is_control) {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Window-rule host acknowledgement detail is invalid or exceeds 512 bytes.",
                index.to_string(),
            ));
        }
        let acknowledgement = HostIntentAcknowledgement {
            index,
            rule_id: intent.rule_id.clone(),
            observed_revision,
            outcome,
            detail,
        };
        self.acknowledged[index] = true;
        self.acknowledged_host_intents.push(acknowledgement);
        Ok(self
            .acknowledged_host_intents
            .last()
            .expect("just appended"))
    }

    /// Dispatch each unacknowledged host effect in stable rule/action order.
    /// This runs outside the canonical transaction. Successful earlier
    /// acknowledgements are retained if a later adapter call fails, so retry
    /// resumes at the first still-pending intent without replaying completed
    /// host effects.
    pub fn apply_host_intents(
        &mut self,
        adapter: &mut impl WindowRuleHostAdapter,
    ) -> DockResult<()> {
        for index in 0..self.host_intents.len() {
            if self.acknowledged[index] {
                continue;
            }
            let result = adapter.apply(&self.host_intents[index], self.resulting_revision)?;
            self.acknowledge_host_intent(
                index,
                result.observed_revision,
                result.outcome,
                result.detail,
            )?;
        }
        Ok(())
    }

    pub fn all_host_intents_acknowledged(&self) -> bool {
        self.acknowledged.iter().all(|acknowledged| *acknowledged)
    }
}

fn operation_already_satisfied(document: &LayoutDocument, operation: &Operation) -> bool {
    match operation {
        Operation::SetTabPinned { panel, pinned } => {
            document.pinned_panels.contains(panel) == *pinned
        }
        Operation::SetPanelWorkspaceMemberships { panel, tags } => document
            .descriptor(panel)
            .is_some_and(|descriptor| &descriptor.workspace_memberships == tags),
        Operation::AutoHide { panel, edge } => document
            .auto_hide_panels
            .iter()
            .any(|item| item.panel == *panel && item.edge == *edge),
        Operation::RootPanel { panel } => matches!(
            document.root.as_deref().map(|node| &node.kind),
            Some(NodeKind::Panel { panel: current }) if current == panel
        ),
        Operation::FloatPanel {
            panel,
            window,
            geometry,
            monitor,
            scale,
        } => document
            .floating_windows
            .iter()
            .find(|item| item.id == *window)
            .is_some_and(|item| {
                contains_panel(&item.root, panel)
                    && item.geometry == *geometry
                    && item.monitor_id == *monitor
                    && (item.scale - scale).abs() < f64::EPSILON
            }),
        Operation::CenterPanel {
            panel,
            target_panel,
        } => {
            document
                .root
                .as_deref()
                .is_some_and(|root| share_tabs(root, panel, target_panel))
                && !document.floating_windows.iter().any(|item| {
                    contains_panel(&item.root, panel) || contains_panel(&item.root, target_panel)
                })
        }
        Operation::CenterPanelGroup {
            panels,
            target_panel,
        } => {
            panels.iter().all(|panel| {
                document
                    .root
                    .as_deref()
                    .is_some_and(|root| share_tabs(root, panel, target_panel))
            }) && !document.floating_windows.iter().any(|item| {
                panels.iter().any(|panel| contains_panel(&item.root, panel))
                    || contains_panel(&item.root, target_panel)
            })
        }
        Operation::ConvertContainer { container, target } => document
            .root
            .as_deref()
            .and_then(|root| crate::find_node(root, container))
            .is_some_and(|node| modality_matches(&node.kind, target)),
        _ => false,
    }
}

fn share_tabs(node: &Node, first: &PanelId, second: &PanelId) -> bool {
    match &node.kind {
        NodeKind::Tabs { items, .. } => {
            let has_first = items.iter().any(|child| contains_panel(child, first));
            let has_second = items.iter().any(|child| contains_panel(child, second));
            (has_first && has_second) || items.iter().any(|child| share_tabs(child, first, second))
        }
        NodeKind::DocumentWell { groups, .. }
        | NodeKind::Split {
            children: groups, ..
        } => groups.iter().any(|child| share_tabs(child, first, second)),
        NodeKind::Extension { children, .. } => children
            .iter()
            .any(|child| share_tabs(child, first, second)),
        _ => false,
    }
}

fn contains_panel(node: &Node, panel: &PanelId) -> bool {
    match &node.kind {
        NodeKind::Panel { panel: current } | NodeKind::Placeholder { panel: current, .. } => {
            current == panel
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => items.iter().any(|child| contains_panel(child, panel)),
    }
}

fn modality_matches(node: &NodeKind, target: &ContainerModality) -> bool {
    match (node, target) {
        (NodeKind::Tabs { .. }, ContainerModality::Tabbed { .. })
        | (NodeKind::Split { .. }, ContainerModality::Split { .. })
        | (NodeKind::DocumentWell { .. }, ContainerModality::Accordion { .. })
        | (NodeKind::DocumentWell { .. }, ContainerModality::ShadeStack) => true,
        (
            NodeKind::Extension {
                modality,
                payload,
                children,
            },
            target,
        ) => {
            let target_id = match target {
                ContainerModality::Grid { .. } => "doofusdock.masonry",
                ContainerModality::Spatial => "doofusdock.spatial",
                ContainerModality::NodeGraph => "doofusdock.node_graph",
                ContainerModality::Layered => "doofusdock.layered",
                ContainerModality::LayoutStrategy { strategy, .. } => strategy.modality(),
                ContainerModality::ScrollableColumns { .. } => SCROLLABLE_COLUMNS_MODALITY_ID,
                _ => return false,
            };
            modality.0 == target_id
                && match target {
                    ContainerModality::Grid { columns, gap } => {
                        serde_json::from_str::<serde_json::Value>(payload).is_ok_and(|config| {
                            config.get("columns").and_then(serde_json::Value::as_u64)
                                == Some(*columns as u64)
                                && config.get("gap").and_then(serde_json::Value::as_u64)
                                    == Some(*gap as u64)
                        })
                    }
                    ContainerModality::LayoutStrategy { strategy, gap } => {
                        LayoutStrategyConfiguration::from_payload(payload)
                            .is_ok_and(|config| config.strategy == *strategy && config.gap == *gap)
                    }
                    ContainerModality::ScrollableColumns {
                        gap,
                        width,
                        minimum_width,
                        maximum_width,
                    } => ScrollableColumnsConfiguration::from_node(&Node {
                        id: NodeId::from("rule-target"),
                        kind: NodeKind::Extension {
                            modality: modality.clone(),
                            payload: payload.clone(),
                            children: children.clone(),
                        },
                    })
                    .is_ok_and(|config| {
                        config.gap == *gap
                            && config.columns.iter().all(|column| {
                                column.width == *width
                                    && column.minimum_width == *minimum_width
                                    && column.maximum_width == *maximum_width
                            })
                    }),
                    _ => serde_json::from_str::<serde_json::Value>(payload).is_ok(),
                }
        }
        _ => false,
    }
}

fn predicate_matches(predicate: &WindowRulePredicate, context: &WindowRuleContext) -> bool {
    predicate
        .panel_id
        .as_ref()
        .is_none_or(|value| value == &context.panel_id)
        && predicate
            .panel_role
            .as_ref()
            .is_none_or(|value| value == &context.panel_role)
        && map_contains(&context.panel_metadata, &predicate.panel_metadata)
        && predicate
            .workspace_id
            .as_ref()
            .is_none_or(|value| value == &context.workspace_id)
        && predicate
            .workspace_tags_all
            .is_subset(&context.workspace_tags)
        && (predicate.workspace_tags_any.is_empty()
            || !predicate
                .workspace_tags_any
                .is_disjoint(&context.workspace_tags))
        && predicate
            .group_id
            .as_ref()
            .is_none_or(|value| context.group_id.as_ref() == Some(value))
        && predicate
            .modality
            .as_ref()
            .is_none_or(|value| context.modality.as_ref() == Some(value))
        && predicate
            .focused
            .is_none_or(|value| value == context.focused)
        && predicate
            .monitor_role
            .as_ref()
            .is_none_or(|value| context.monitor_role.as_ref() == Some(value))
        && predicate
            .viewport_class
            .as_ref()
            .is_none_or(|value| context.viewport_class.as_ref() == Some(value))
        && map_contains(&context.user_metadata, &predicate.user_metadata)
        && predicate
            .host_capabilities_all
            .is_subset(&context.host_capabilities.supported)
        && (predicate.host_capabilities_any.is_empty()
            || !predicate
                .host_capabilities_any
                .is_disjoint(&context.host_capabilities.supported))
}

fn map_contains(actual: &BTreeMap<String, String>, expected: &BTreeMap<String, String>) -> bool {
    expected
        .iter()
        .all(|(key, value)| actual.get(key) == Some(value))
}

fn valid_rule_metadata(metadata: &BTreeMap<String, String>) -> bool {
    metadata.len() <= 64
        && metadata.iter().all(|(key, value)| {
            valid_token(key, 128) && value.len() <= 1024 && !value.chars().any(char::is_control)
        })
}

fn validate_resolved_size_actions(
    actions: &[ResolvedRuleAction],
    panel: &PanelId,
) -> DockResult<()> {
    let mut preferred = None;
    let mut minimum = None;
    let mut maximum = None;
    for resolved in actions {
        match resolved.action {
            WindowRuleAction::PreferredSize(size) => preferred = Some(size),
            WindowRuleAction::MinimumSize(size) => minimum = Some(size),
            WindowRuleAction::MaximumSize(size) => maximum = Some(size),
            _ => {}
        }
    }
    let invalid = minimum
        .zip(maximum)
        .is_some_and(|(min, max)| min.width > max.width || min.height > max.height)
        || preferred
            .zip(minimum)
            .is_some_and(|(pref, min)| pref.width < min.width || pref.height < min.height)
        || preferred
            .zip(maximum)
            .is_some_and(|(pref, max)| pref.width > max.width || pref.height > max.height);
    if invalid {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Resolved window-rule size policies conflict.",
            panel.0.clone(),
        ));
    }
    Ok(())
}

fn validate_predicate(predicate: &WindowRulePredicate, context: &str) -> DockResult<()> {
    if predicate.panel_metadata.len() > 64
        || predicate.user_metadata.len() > 64
        || predicate.workspace_tags_all.len() > 64
        || predicate.workspace_tags_any.len() > 64
        || predicate.host_capabilities_all.len() > 64
        || predicate.host_capabilities_any.len() > 64
    {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Window-rule predicate exceeds a field limit.",
            context,
        ));
    }
    for (key, value) in predicate
        .panel_metadata
        .iter()
        .chain(&predicate.user_metadata)
    {
        if !valid_token(key, 128) || value.len() > 1024 || value.chars().any(char::is_control) {
            return Err(rule_error(
                "Rule metadata predicate is invalid or too large.",
                context,
            ));
        }
    }
    for field in [
        predicate.panel_role.as_ref(),
        predicate.monitor_role.as_ref(),
        predicate.viewport_class.as_ref(),
    ] {
        if field.is_some_and(|value| !valid_token(value, 128)) {
            return Err(rule_error("Rule predicate token is invalid.", context));
        }
    }
    for value in predicate
        .workspace_tags_all
        .iter()
        .chain(&predicate.workspace_tags_any)
        .chain(&predicate.host_capabilities_all)
        .chain(&predicate.host_capabilities_any)
    {
        if !valid_token(value, 256) {
            return Err(rule_error(
                "Rule predicate tag or capability is invalid.",
                context,
            ));
        }
    }
    Ok(())
}

fn validate_action(action: &WindowRuleAction, context: &str) -> DockResult<()> {
    let valid_size = |size: Size| {
        size.width > 0 && size.height > 0 && size.width <= 1_000_000 && size.height <= 1_000_000
    };
    match action {
        WindowRuleAction::Placement(PlacementIntent::Center { target_panel })
            if target_panel.is_empty() =>
        {
            return Err(rule_error(
                "Center placement requires a target panel.",
                context,
            ));
        }
        WindowRuleAction::Placement(PlacementIntent::CenterGroup {
            panels,
            target_panel,
        }) if panels.is_empty()
            || panels.len() > 64
            || !valid_token(&target_panel.0, 128)
            || panels.iter().any(|panel| !valid_token(&panel.0, 128))
            || panels.iter().collect::<BTreeSet<_>>().len() != panels.len()
            || panels.contains(target_panel) =>
        {
            return Err(rule_error(
                "Center-group placement has invalid or duplicate panel IDs.",
                context,
            ));
        }
        WindowRuleAction::Placement(PlacementIntent::Split { target, ratio, .. })
            if target.is_empty() || !ratio.is_finite() || !(0.05..=0.95).contains(ratio) =>
        {
            return Err(rule_error(
                "Split placement target or ratio is invalid.",
                context,
            ));
        }
        WindowRuleAction::Placement(PlacementIntent::Float {
            window,
            geometry,
            monitor,
            scale,
        }) if window.is_empty()
            || geometry.width <= 0
            || geometry.height <= 0
            || geometry.width > 1_000_000
            || geometry.height > 1_000_000
            || !valid_token(monitor, 256)
            || !scale.is_finite()
            || !(0.25..=8.0).contains(scale) =>
        {
            return Err(rule_error("Floating placement is invalid.", context));
        }
        WindowRuleAction::PreferredSize(size)
        | WindowRuleAction::MinimumSize(size)
        | WindowRuleAction::MaximumSize(size)
            if !valid_size(*size) =>
        {
            return Err(rule_error("Rule size policy is invalid.", context));
        }
        WindowRuleAction::AssignMonitor { monitor } if !valid_token(monitor, 256) => {
            return Err(rule_error("Monitor assignment token is invalid.", context));
        }
        WindowRuleAction::WorkspaceMembership { workspaces }
            if workspaces.is_empty()
                || workspaces.len() > 64
                || workspaces.iter().any(|id| !valid_token(&id.0, 128)) =>
        {
            return Err(rule_error(
                "Workspace membership action is empty or invalid.",
                context,
            ));
        }
        WindowRuleAction::TransitionPreference { transition }
            if !valid_token(&transition.0, 128) =>
        {
            return Err(rule_error("Transition preference ID is invalid.", context));
        }
        WindowRuleAction::SelectLayout { layout } if !valid_token(layout, 128) => {
            return Err(rule_error("Layout selection ID is invalid.", context));
        }
        WindowRuleAction::DefaultModality(modality)
            if !valid_token(modality_capability(modality), 256)
                || matches!(modality, ContainerModality::Grid { columns, gap }
                    if !(1..=64).contains(columns) || *gap > 256)
                || matches!(modality, ContainerModality::Split { weights, .. }
                    if weights.is_empty() || weights.len() > 64
                        || weights.iter().any(|value| !value.is_finite() || *value <= 0.0))
                || matches!(modality, ContainerModality::LayoutStrategy { strategy, gap }
                    if LayoutStrategyConfiguration::new(*strategy, *gap).is_err())
                || matches!(modality, ContainerModality::ScrollableColumns {
                    gap, width, minimum_width, maximum_width
                } if ScrollableColumnsConfiguration::new(vec![crate::ScrollableColumn {
                    child_id: NodeId::from("rule-validation"),
                    width: *width,
                    minimum_width: *minimum_width,
                    maximum_width: *maximum_width,
                }], *gap).is_err()) =>
        {
            return Err(rule_error(
                "Default modality capability is invalid.",
                context,
            ));
        }
        _ => {}
    }
    Ok(())
}

fn validate_override_acyclic(rules: &[WindowRule]) -> DockResult<()> {
    fn visit(
        id: &str,
        by_id: &BTreeMap<&str, &WindowRule>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visited.contains(id) {
            return true;
        }
        if !visiting.insert(id.to_owned()) {
            return false;
        }
        if let Some(rule) = by_id.get(id) {
            for target in &rule.overrides {
                if !visit(target, by_id, visiting, visited) {
                    return false;
                }
            }
        }
        visiting.remove(id);
        visited.insert(id.to_owned());
        true
    }
    let by_id: BTreeMap<_, _> = rules.iter().map(|rule| (rule.id.as_str(), rule)).collect();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in by_id.keys() {
        if !visit(id, &by_id, &mut visiting, &mut visited) {
            return Err(rule_error("Window-rule overrides contain a cycle.", id));
        }
    }
    Ok(())
}

fn valid_token(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.:-".contains(character))
}

fn modality_capability(modality: &ContainerModality) -> &'static str {
    match modality {
        ContainerModality::Tabbed { .. } => "workspace:tabs",
        ContainerModality::Split { .. } => "workspace:docking",
        ContainerModality::Accordion { .. } => "workspace:accordion",
        ContainerModality::ShadeStack => "workspace:shade_stack",
        ContainerModality::Grid { .. } => "workspace:grid",
        ContainerModality::Spatial => "workspace:spatial",
        ContainerModality::NodeGraph => "workspace:node_graph",
        ContainerModality::Layered => "workspace:layered",
        ContainerModality::LayoutStrategy { .. } => "workspace:layout_strategy",
        ContainerModality::ScrollableColumns { .. } => "workspace:scrollable_columns",
    }
}

fn rule_error(message: &str, context: &str) -> DockError {
    DockError::new(ErrorCode::SchemaError, message, context)
}

fn diagnostic(
    rule_id: &str,
    kind: RuleDiagnosticKind,
    detail: impl Into<String>,
) -> WindowRuleDiagnostic {
    WindowRuleDiagnostic {
        rule_id: rule_id.to_owned(),
        kind,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn document() -> LayoutDocument {
        let mut document = LayoutDocument::new("rules", "workbench");
        let mut descriptor = crate::PanelDescriptor::new("editor", "Editor");
        descriptor.role = "editor".into();
        descriptor.metadata.insert("language".into(), "rust".into());
        document.panels = vec![descriptor].into();
        document.root = Some(crate::Node::panel("editor-node", "editor"));
        document
    }

    fn context() -> WindowRuleContext {
        WindowRuleContext::for_panel(
            &document(),
            &PanelId::from("editor"),
            HostCapabilities::default(),
        )
        .unwrap()
    }

    fn rule(id: &str, priority: i32, action: WindowRuleAction) -> WindowRule {
        WindowRule {
            id: id.into(),
            enabled: true,
            priority,
            overrides: vec![],
            event_kinds: BTreeSet::new(),
            predicate: WindowRulePredicate::default(),
            actions: vec![action],
        }
    }

    #[test]
    fn event_rule_consumer_is_revision_bound_deduplicated_and_suppressible() {
        let mut context = context();
        context.revision = 7;
        let mut event_rule = rule("on_create", 1, WindowRuleAction::SetPinned { pinned: true });
        event_rule
            .event_kinds
            .insert(FrameworkEventKind::PanelCreated);
        let rules = WindowRuleSet {
            schema_version: 1,
            rules: vec![event_rule],
        };
        let event = FrameworkEvent {
            id: crate::EventId(11),
            sequence: 11,
            revision: 7,
            kind: FrameworkEventKind::PanelCreated,
            command: CommandId::from("create-editor"),
            payload: BTreeMap::new(),
        };
        let batch = EventBatch {
            events: vec![event.clone()],
            lost_events: 0,
        };
        let mut consumer = WindowRuleEventConsumer::default();
        let triggered = consumer
            .evaluate_batch(&rules, &batch, std::slice::from_ref(&context))
            .unwrap();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].event_sequence, 11);
        assert_eq!(triggered[0].evaluation.matched_rules, vec!["on_create"]);
        assert!(
            consumer
                .evaluate_batch(&rules, &batch, std::slice::from_ref(&context))
                .unwrap()
                .is_empty()
        );

        let mut guarded = WindowRuleEventConsumer::default();
        guarded
            .suppress_command(CommandId::from("create-editor"))
            .unwrap();
        assert!(
            guarded
                .evaluate_batch(&rules, &batch, std::slice::from_ref(&context))
                .unwrap()
                .is_empty()
        );
        let mut drifted_context = context.clone();
        drifted_context.revision = 6;
        let drift_event = FrameworkEvent {
            id: crate::EventId(12),
            sequence: 12,
            revision: 7,
            kind: FrameworkEventKind::PanelCreated,
            command: CommandId::from("create-editor-2"),
            payload: BTreeMap::new(),
        };
        let drift_batch = EventBatch {
            events: vec![drift_event.clone()],
            lost_events: 0,
        };
        assert!(
            guarded
                .evaluate_batch(&rules, &drift_batch, &[drifted_context.clone()])
                .is_err()
        );
        drifted_context.revision = 7;
        assert_eq!(
            guarded
                .evaluate_batch(&rules, &drift_batch, &[drifted_context])
                .unwrap()
                .len(),
            1,
            "revision failure must not consume the event cursor"
        );
        assert!(
            guarded
                .evaluate_batch(
                    &rules,
                    &EventBatch {
                        events: vec![drift_event],
                        lost_events: 1,
                    },
                    &[],
                )
                .is_err()
        );
    }

    #[test]
    fn revision_scoped_suppression_keeps_same_command_eligible_in_later_revision() {
        let mut context = context();
        context.revision = 7;
        let mut event_rule = rule("on_create", 1, WindowRuleAction::SetPinned { pinned: true });
        event_rule
            .event_kinds
            .insert(FrameworkEventKind::PanelCreated);
        let rules = WindowRuleSet {
            schema_version: 1,
            rules: vec![event_rule],
        };
        let command = CommandId::from("reused-rule-command");
        let make_event = |sequence, revision| FrameworkEvent {
            id: crate::EventId(sequence),
            sequence,
            revision,
            kind: FrameworkEventKind::PanelCreated,
            command: command.clone(),
            payload: BTreeMap::new(),
        };
        let mut consumer = WindowRuleEventConsumer::default();
        consumer
            .suppress_command_at_revision(command.clone(), 7)
            .unwrap();

        let same_commit_facts = EventBatch {
            events: vec![make_event(11, 7), make_event(12, 7)],
            lost_events: 0,
        };
        assert!(
            consumer
                .evaluate_batch(&rules, &same_commit_facts, std::slice::from_ref(&context))
                .unwrap()
                .is_empty()
        );

        context.revision = 8;
        let later_reuse = EventBatch {
            events: vec![make_event(13, 8)],
            lost_events: 0,
        };
        let triggered = consumer
            .evaluate_batch(&rules, &later_reuse, &[context])
            .unwrap();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].event_sequence, 13);
    }

    #[test]
    fn rule_consumer_accepts_revision_bound_host_geometry_monitor_and_transition_facts() {
        let revision_zero = context();
        let mut revision_one = revision_zero.clone();
        revision_one.revision = 1;
        let mut event_rule = rule(
            "on_host_fact",
            1,
            WindowRuleAction::SetPinned { pinned: true },
        );
        event_rule.event_kinds = BTreeSet::from([
            FrameworkEventKind::NativeViewportGeometryObserved,
            FrameworkEventKind::NativeRootViewportGeometryObserved,
            FrameworkEventKind::MonitorChanged,
            FrameworkEventKind::TransitionStarted,
            FrameworkEventKind::TransitionCompleted,
        ]);
        let rules = WindowRuleSet {
            schema_version: 1,
            rules: vec![event_rule],
        };
        let events = [
            FrameworkEvent {
                id: crate::EventId(1),
                sequence: 1,
                revision: 0,
                kind: FrameworkEventKind::NativeViewportGeometryObserved,
                command: CommandId::from("native-geometry"),
                payload: BTreeMap::from([
                    ("window".into(), "float-window".into()),
                    ("geometry".into(), "-1200,80,500,360".into()),
                ]),
            },
            FrameworkEvent {
                id: crate::EventId(2),
                sequence: 2,
                revision: 0,
                kind: FrameworkEventKind::NativeRootViewportGeometryObserved,
                command: CommandId::from("native-root-geometry"),
                payload: BTreeMap::from([("geometry".into(), "0,0,1280,720".into())]),
            },
            FrameworkEvent {
                id: crate::EventId(3),
                sequence: 3,
                revision: 0,
                kind: FrameworkEventKind::MonitorChanged,
                command: CommandId::from("monitor-change"),
                payload: BTreeMap::from([
                    ("monitor".into(), "DISPLAY2".into()),
                    ("connected".into(), "false".into()),
                ]),
            },
            FrameworkEvent {
                id: crate::EventId(4),
                sequence: 4,
                revision: 1,
                kind: FrameworkEventKind::TransitionStarted,
                command: CommandId::from("transition-start"),
                payload: BTreeMap::from([("transition".into(), "morph-1".into())]),
            },
            FrameworkEvent {
                id: crate::EventId(5),
                sequence: 5,
                revision: 1,
                kind: FrameworkEventKind::TransitionCompleted,
                command: CommandId::from("transition-complete"),
                payload: BTreeMap::from([("transition".into(), "morph-1".into())]),
            },
        ];
        let mut consumer = WindowRuleEventConsumer::default();
        let results = consumer
            .evaluate_batch(
                &rules,
                &EventBatch {
                    events: events.to_vec(),
                    lost_events: 0,
                },
                &[revision_zero, revision_one],
            )
            .unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(
            results
                .iter()
                .map(|result| result.event_kind)
                .collect::<Vec<_>>(),
            vec![
                FrameworkEventKind::NativeViewportGeometryObserved,
                FrameworkEventKind::NativeRootViewportGeometryObserved,
                FrameworkEventKind::MonitorChanged,
                FrameworkEventKind::TransitionStarted,
                FrameworkEventKind::TransitionCompleted,
            ]
        );
        assert!(
            results
                .iter()
                .all(|result| { result.evaluation.matched_rules == vec!["on_host_fact"] })
        );
    }

    #[test]
    fn focus_event_rule_context_is_scoped_to_the_reported_panel() {
        let mut first = context();
        first.revision = 9;
        let mut second = first.clone();
        second.panel_id = PanelId::from("focused-panel");
        let mut focus_rule = rule(
            "focused-event",
            1,
            WindowRuleAction::SetPinned { pinned: true },
        );
        focus_rule
            .event_kinds
            .insert(FrameworkEventKind::FocusChanged);
        focus_rule.predicate.focused = Some(true);
        let rules = WindowRuleSet {
            schema_version: 1,
            rules: vec![focus_rule],
        };
        let event = FrameworkEvent {
            id: crate::EventId(4),
            sequence: 4,
            revision: 9,
            kind: FrameworkEventKind::FocusChanged,
            command: CommandId::from("focused-event"),
            payload: BTreeMap::from([("panel".into(), "focused-panel".into())]),
        };
        let triggered = WindowRuleEventConsumer::default()
            .evaluate_batch(
                &rules,
                &EventBatch {
                    events: vec![event],
                    lost_events: 0,
                },
                &[first, second],
            )
            .unwrap();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].panel, PanelId::from("focused-panel"));
        assert_eq!(triggered[0].evaluation.matched_rules, vec!["focused-event"]);
    }

    #[test]
    fn rule_evaluation_is_deterministic_and_matches_typed_context_fields() {
        let mut context = context();
        context.workspace_tags.extend(["dev".into(), "rust".into()]);
        context
            .user_metadata
            .insert("project".into(), "dock".into());
        context
            .host_capabilities
            .supported
            .insert("window:create".into());
        let mut predicate = WindowRulePredicate {
            panel_role: Some("editor".into()),
            ..WindowRulePredicate::default()
        };
        predicate
            .panel_metadata
            .insert("language".into(), "rust".into());
        predicate.workspace_tags_all.insert("dev".into());
        predicate.workspace_tags_any.insert("rust".into());
        predicate
            .user_metadata
            .insert("project".into(), "dock".into());
        predicate
            .host_capabilities_all
            .insert("window:create".into());
        let mut editor_rule = rule(
            "editor-float",
            4,
            WindowRuleAction::Placement(PlacementIntent::Root),
        );
        editor_rule.predicate = predicate;
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![editor_rule],
        };
        assert_eq!(
            set.evaluate(&context).unwrap(),
            set.evaluate(&context).unwrap()
        );
        assert_eq!(
            set.evaluate(&context).unwrap().matched_rules,
            vec!["editor-float"]
        );
    }

    #[test]
    fn panel_rule_metadata_is_bounded_and_schema_validated() {
        let mut invalid_role = document();
        Arc::make_mut(&mut invalid_role.panels)[0].role = "editor\nforged".into();
        assert!(validate_layout(&invalid_role).is_err());

        let mut invalid_key = document();
        Arc::make_mut(&mut invalid_key.panels)[0]
            .metadata
            .insert("language/name".into(), "rust".into());
        assert!(validate_layout(&invalid_key).is_err());

        let mut invalid_value = document();
        Arc::make_mut(&mut invalid_value.panels)[0]
            .metadata
            .insert("language".into(), "rust\u{7}".into());
        assert!(validate_layout(&invalid_value).is_err());
    }

    #[test]
    fn workspace_membership_rules_commit_canonically_and_repeat_without_new_revision() {
        let mut document = LayoutDocument::new("membership-rule", "workbench");
        document.panels = Arc::new(vec![crate::PanelDescriptor::new("editor", "Editor")]);
        document.root = Some(crate::Node::panel("editor-node", "editor"));
        let host = HostCapabilities {
            supported: BTreeSet::from(["workspace:membership".into()]),
        };
        let context =
            WindowRuleContext::for_panel(&document, &PanelId::from("editor"), host).unwrap();
        let rule_set = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "membership",
                1,
                WindowRuleAction::WorkspaceMembership {
                    workspaces: BTreeSet::from([
                        WorkspaceId::from("work"),
                        WorkspaceId::from("research"),
                    ]),
                },
            )],
        };
        let plan = WindowRulePlan::prepare(rule_set.evaluate(&context).unwrap(), &context).unwrap();
        assert!(matches!(
            plan.canonical_operations.as_slice(),
            [Operation::SetPanelWorkspaceMemberships { .. }]
        ));
        assert!(plan.host_intents.is_empty());
        let mut service = DockingService::new(document, 4).unwrap();
        let committed = plan
            .commit(&mut service, CommandId::from("membership-rule"))
            .unwrap();
        assert_eq!(committed.resulting_revision, 1);
        assert_eq!(
            committed
                .transaction
                .unwrap()
                .document
                .descriptor(&PanelId::from("editor"))
                .unwrap()
                .workspace_memberships,
            BTreeSet::from(["work".to_string(), "research".to_string()])
        );
        let current = service.snapshot().document;
        let next_context = WindowRuleContext::for_panel(
            &current,
            &PanelId::from("editor"),
            HostCapabilities {
                supported: BTreeSet::from(["workspace:membership".into()]),
            },
        )
        .unwrap();
        assert_eq!(
            next_context.workspace_tags,
            BTreeSet::from(["work".to_string(), "research".to_string()])
        );
        let repeated =
            WindowRulePlan::prepare(rule_set.evaluate(&next_context).unwrap(), &next_context)
                .unwrap()
                .commit(&mut service, CommandId::from("membership-rule-retry"))
                .unwrap();
        assert!(repeated.transaction.is_none());
        assert_eq!(service.snapshot().document.revision, 1);
    }

    #[test]
    fn group_placement_rules_compile_to_one_reversible_canonical_batch() {
        let mut document = LayoutDocument::new("group-rules", "workbench");
        document.panels = Arc::new(
            ["a", "b", "target"]
                .into_iter()
                .map(|id| crate::PanelDescriptor::new(id, id))
                .collect(),
        );
        document.root = Some(crate::Node::split(
            "root",
            crate::Orientation::Horizontal,
            vec![
                crate::Node::panel("a-node", "a"),
                crate::Node::panel("b-node", "b"),
                crate::Node::panel("target-node", "target"),
            ],
            vec![1.0 / 3.0; 3],
        ));
        let host = HostCapabilities {
            supported: BTreeSet::from(["workspace:tabs".into()]),
        };
        let context = WindowRuleContext::for_panel(&document, &PanelId::from("a"), host).unwrap();
        let rules = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "center-group",
                1,
                WindowRuleAction::Placement(PlacementIntent::CenterGroup {
                    panels: vec![PanelId::from("a"), PanelId::from("b")],
                    target_panel: PanelId::from("target"),
                }),
            )],
        };
        let plan = WindowRulePlan::prepare(rules.evaluate(&context).unwrap(), &context).unwrap();
        assert!(matches!(
            plan.canonical_operations.as_slice(),
            [Operation::CenterPanelGroup { .. }]
        ));
        let mut service = DockingService::new(document.clone(), 4).unwrap();
        assert_eq!(
            plan.commit(&mut service, CommandId::from("group-rule"))
                .unwrap()
                .transaction
                .unwrap()
                .document
                .revision,
            1
        );
        service.undo(1).unwrap();
        let restored = service.snapshot().document;
        assert_eq!(restored.revision, 2);
        let mut expected = document;
        expected.revision = restored.revision;
        assert_eq!(restored, expected);
    }

    #[test]
    fn priorities_overrides_and_equal_priority_conflicts_are_explicit() {
        let low = rule("low", 1, WindowRuleAction::SetPinned { pinned: false });
        let high = rule("high", 8, WindowRuleAction::SetPinned { pinned: true });
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![low.clone(), high],
        };
        let evaluation = set.evaluate(&context()).unwrap();
        assert_eq!(evaluation.actions[0].rule_id, "high");
        assert!(
            evaluation
                .diagnostics
                .iter()
                .any(|item| item.rule_id == "low" && item.kind == RuleDiagnosticKind::Overridden)
        );

        let mut override_rule = rule("override", 8, WindowRuleAction::SetPinned { pinned: true });
        override_rule.overrides = vec!["conflict".into()];
        let conflict = rule("conflict", 8, WindowRuleAction::SetPinned { pinned: false });
        let overridden = WindowRuleSet {
            schema_version: 1,
            rules: vec![conflict, override_rule],
        }
        .evaluate(&context())
        .unwrap();
        assert_eq!(overridden.actions[0].rule_id, "override");

        let a = rule("a", 8, WindowRuleAction::SetPinned { pinned: true });
        let b = rule("b", 8, WindowRuleAction::SetPinned { pinned: false });
        let conflicted = WindowRuleSet {
            schema_version: 1,
            rules: vec![a, b],
        }
        .evaluate(&context())
        .unwrap();
        assert!(conflicted.actions.is_empty());
        assert_eq!(
            conflicted
                .diagnostics
                .iter()
                .filter(|item| item.kind == RuleDiagnosticKind::Conflict)
                .count(),
            2
        );
    }

    #[test]
    fn disabled_rules_and_nonmatching_predicates_do_not_produce_actions() {
        let mut disabled = rule("disabled", 10, WindowRuleAction::SetPinned { pinned: true });
        disabled.enabled = false;
        let mut mismatch = rule("mismatch", 10, WindowRuleAction::SetPinned { pinned: true });
        mismatch.predicate.panel_role = Some("console".into());
        let evaluation = WindowRuleSet {
            schema_version: 1,
            rules: vec![disabled, mismatch],
        }
        .evaluate(&context())
        .unwrap();
        assert!(evaluation.actions.is_empty());
        assert!(
            evaluation
                .diagnostics
                .iter()
                .any(|item| item.kind == RuleDiagnosticKind::Disabled)
        );
        assert!(
            evaluation
                .diagnostics
                .iter()
                .any(|item| item.kind == RuleDiagnosticKind::NotMatched)
        );
    }

    #[test]
    fn import_export_migrates_version_zero_and_rejects_cycles_and_unknown_fields() {
        let legacy = include_str!("../tests/fixtures/window_rules_schema_v0.json");
        assert_eq!(WindowRuleSet::from_json(legacy).unwrap().schema_version, 1);
        let unknown = r#"{"schema_version":1,"rules":[],"surprise":true}"#;
        assert!(WindowRuleSet::from_json(unknown).is_err());
        let mut a = rule("a", 0, WindowRuleAction::SetPinned { pinned: true });
        let mut b = rule("b", 0, WindowRuleAction::SetPinned { pinned: false });
        a.overrides = vec!["b".into()];
        b.overrides = vec!["a".into()];
        assert!(
            WindowRuleSet {
                schema_version: 1,
                rules: vec![a, b]
            }
            .validate()
            .is_err()
        );
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule("pin", 1, WindowRuleAction::SetPinned { pinned: true })],
        };
        assert_eq!(
            WindowRuleSet::from_json(&set.to_json().unwrap()).unwrap(),
            set
        );
    }

    #[test]
    fn document_rule_commit_uses_existing_history_and_is_idempotent() {
        let mut document = document();
        let mut host = HostCapabilities::default();
        host.supported.insert("workspace:membership".into());
        let context =
            WindowRuleContext::for_panel(&document, &PanelId::from("editor"), host).unwrap();
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "membership",
                1,
                WindowRuleAction::WorkspaceMembership {
                    workspaces: BTreeSet::from([WorkspaceId::from("analysis")]),
                },
            )],
        };
        let plan = WindowRulePlan::prepare(set.evaluate(&context).unwrap(), &context).unwrap();
        let mut history = crate::UndoStack::new(8);
        let committed = plan
            .commit_document(
                &mut document,
                &mut history,
                CommandId::from("rule-membership"),
            )
            .unwrap();
        assert!(committed.transaction.is_some());
        assert_eq!(document.revision, 1);
        assert!(
            document
                .descriptor(&PanelId::from("editor"))
                .unwrap()
                .workspace_memberships
                .contains("analysis")
        );
        assert_eq!(history.undo_size(), 1);
        history.undo(&mut document).unwrap();
        assert!(
            document
                .descriptor(&PanelId::from("editor"))
                .unwrap()
                .workspace_memberships
                .is_empty()
        );
        history.redo(&mut document).unwrap();
        let context = WindowRuleContext::for_panel(
            &document,
            &PanelId::from("editor"),
            HostCapabilities {
                supported: BTreeSet::from(["workspace:membership".into()]),
            },
        )
        .unwrap();
        let plan = WindowRulePlan::prepare(set.evaluate(&context).unwrap(), &context).unwrap();
        let revision = document.revision;
        let committed = plan
            .commit_document(
                &mut document,
                &mut history,
                CommandId::from("rule-membership-repeat"),
            )
            .unwrap();
        assert!(committed.transaction.is_none());
        assert_eq!(document.revision, revision);
        assert_eq!(history.undo_size(), 1);
    }

    #[test]
    fn plan_compiles_one_atomic_canonical_batch_and_rejects_stale_or_unsupported_work() {
        let document = document();
        let host = HostCapabilities {
            supported: BTreeSet::from(["native_viewport".into(), "workspace:tabs".into()]),
        };
        let rule_context =
            WindowRuleContext::for_panel(&document, &PanelId::from("editor"), host).unwrap();
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![
                rule(
                    "float",
                    5,
                    WindowRuleAction::Placement(PlacementIntent::Float {
                        window: crate::WindowId::from("rules-float"),
                        geometry: Rect {
                            x: 40,
                            y: 50,
                            width: 320,
                            height: 240,
                        },
                        monitor: "display-1".into(),
                        scale: 1.0,
                    }),
                ),
                rule("pin", 4, WindowRuleAction::SetPinned { pinned: true }),
            ],
        };
        let plan =
            WindowRulePlan::prepare(set.evaluate(&rule_context).unwrap(), &rule_context).unwrap();
        assert_eq!(plan.canonical_operations.len(), 2);
        let mut service = DockingService::new(document, 8).unwrap();
        let committed = plan
            .commit(&mut service, CommandId::from("rule-batch"))
            .unwrap();
        assert_eq!(committed.transaction.unwrap().document.revision, 1);

        let second_context = WindowRuleContext::for_panel(
            &service.snapshot().document,
            &PanelId::from("editor"),
            HostCapabilities {
                supported: BTreeSet::from(["native_viewport".into(), "workspace:tabs".into()]),
            },
        )
        .unwrap();
        let second_plan =
            WindowRulePlan::prepare(set.evaluate(&second_context).unwrap(), &second_context)
                .unwrap();
        assert!(
            second_plan
                .commit(&mut service, CommandId::from("rule-batch-repeat"))
                .unwrap()
                .transaction
                .is_none()
        );
        assert_eq!(service.snapshot().revision, 1);

        let stale_context = context();
        let stale_eval = set.evaluate(&stale_context).unwrap();
        let mut changed = stale_context;
        changed.revision += 1;
        assert!(WindowRulePlan::prepare(stale_eval, &changed).is_err());

        let mut unsupported_context = context();
        unsupported_context.host_capabilities.supported.clear();
        let floating = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "float",
                1,
                WindowRuleAction::Placement(PlacementIntent::Float {
                    window: crate::WindowId::from("float-1"),
                    geometry: Rect {
                        x: 0,
                        y: 0,
                        width: 300,
                        height: 200,
                    },
                    monitor: "display-1".into(),
                    scale: 1.0,
                }),
            )],
        };
        let unsupported = WindowRulePlan::prepare(
            floating.evaluate(&unsupported_context).unwrap(),
            &unsupported_context,
        )
        .unwrap();
        assert!(
            unsupported
                .commit(
                    &mut DockingService::new(context_document(), 4).unwrap(),
                    CommandId::from("unsupported")
                )
                .is_err()
        );
    }

    #[test]
    fn host_intents_require_revision_bound_single_acknowledgements() {
        let document = document();
        let mut host = HostCapabilities::default();
        host.supported.insert("window:focus".into());
        let context =
            WindowRuleContext::for_panel(&document, &PanelId::from("editor"), host).unwrap();
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "focus",
                1,
                WindowRuleAction::Focus(FocusBehavior::FocusPanel),
            )],
        };
        let plan = WindowRulePlan::prepare(set.evaluate(&context).unwrap(), &context).unwrap();
        assert!(plan.canonical_operations.is_empty());
        let mut service = DockingService::new(document, 4).unwrap();
        let mut committed = plan
            .commit(&mut service, CommandId::from("rule-focus"))
            .unwrap();
        assert_eq!(committed.resulting_revision, 0);
        assert!(!committed.all_host_intents_acknowledged());
        assert!(
            committed
                .acknowledge_host_intent(0, 1, HostIntentOutcome::Applied, "focused")
                .is_err()
        );
        assert!(
            committed
                .acknowledge_host_intent(0, 0, HostIntentOutcome::Applied, "bad\nreceipt")
                .is_err()
        );
        let receipt = committed
            .acknowledge_host_intent(0, 0, HostIntentOutcome::Applied, "focused")
            .unwrap();
        assert_eq!(receipt.rule_id, "focus");
        assert_eq!(receipt.observed_revision, 0);
        assert!(committed.all_host_intents_acknowledged());
        assert!(
            committed
                .acknowledge_host_intent(0, 0, HostIntentOutcome::Applied, "duplicate")
                .is_err()
        );
        assert!(
            committed
                .acknowledge_host_intent(1, 0, HostIntentOutcome::Applied, "unknown")
                .is_err()
        );
    }

    #[test]
    fn host_adapter_dispatch_resumes_after_partial_ack_without_replaying_success() {
        struct Adapter {
            calls: usize,
            fail_on_call: Option<usize>,
        }
        impl WindowRuleHostAdapter for Adapter {
            fn apply(
                &mut self,
                _intent: &ResolvedRuleAction,
                expected_revision: u64,
            ) -> DockResult<HostIntentResult> {
                self.calls += 1;
                if self.fail_on_call == Some(self.calls) {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "test adapter failure",
                        self.calls.to_string(),
                    ));
                }
                Ok(HostIntentResult {
                    observed_revision: expected_revision,
                    outcome: if self.calls == 1 {
                        HostIntentOutcome::Queued
                    } else {
                        HostIntentOutcome::Applied
                    },
                    detail: format!("receipt-{}", self.calls),
                })
            }
        }

        let document = document();
        let mut host = HostCapabilities::default();
        host.supported.insert("window:focus".into());
        host.supported.insert("window:z_order".into());
        let context =
            WindowRuleContext::for_panel(&document, &PanelId::from("editor"), host).unwrap();
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "host-effects",
                1,
                WindowRuleAction::Focus(FocusBehavior::FocusPanel),
            )],
        };
        let mut evaluation = set.evaluate(&context).unwrap();
        evaluation.actions.push(ResolvedRuleAction {
            rule_id: "host-effects".into(),
            priority: 1,
            action: WindowRuleAction::ZOrder(ZOrderIntent::Raise),
        });
        let plan = WindowRulePlan::prepare(evaluation, &context).unwrap();
        let mut service = DockingService::new(document, 4).unwrap();
        let mut committed = plan
            .commit(&mut service, CommandId::from("adapter-test"))
            .unwrap();
        let mut adapter = Adapter {
            calls: 0,
            fail_on_call: Some(2),
        };

        assert!(committed.apply_host_intents(&mut adapter).is_err());
        assert_eq!(adapter.calls, 2);
        assert_eq!(committed.acknowledged_host_intents.len(), 1);
        assert_eq!(
            committed.acknowledged_host_intents[0].outcome,
            HostIntentOutcome::Queued,
            "a queued command is a dispatch receipt, not proof of native completion"
        );
        assert!(!committed.all_host_intents_acknowledged());

        adapter.fail_on_call = None;
        committed.apply_host_intents(&mut adapter).unwrap();
        assert_eq!(
            adapter.calls, 3,
            "the first successful intent must not replay"
        );
        assert!(committed.all_host_intents_acknowledged());
        assert_eq!(committed.acknowledged_host_intents.len(), 2);
    }

    #[test]
    fn rule_store_atomically_saves_and_recovers_the_last_known_good_snapshot() {
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-window-rules-{}-{}",
            std::process::id(),
            RULE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let store = WindowRuleStore::new(&directory);
        let workspace = WorkspaceId::from("workbench");
        let first = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "first",
                1,
                WindowRuleAction::SetPinned { pinned: true },
            )],
        };
        let second = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "second",
                2,
                WindowRuleAction::SetPinned { pinned: false },
            )],
        };
        store.save(&workspace, &first).unwrap();
        store.save(&workspace, &second).unwrap();
        let path = store.path_for(&workspace).unwrap();
        fs::write(&path, b"damaged rules").unwrap();
        assert_eq!(store.recover(&workspace).unwrap(), first);
        assert_eq!(store.load(&workspace).unwrap(), first);
        assert!(store.load(&WorkspaceId::from("../escape")).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn future_window_rule_schema_does_not_fall_back_to_older_backup() {
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-window-rules-future-{}-{}",
            std::process::id(),
            RULE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let store = WindowRuleStore::new(&directory);
        let workspace = WorkspaceId::from("workbench");
        let first = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "first",
                1,
                WindowRuleAction::SetPinned { pinned: true },
            )],
        };
        let second = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "second",
                2,
                WindowRuleAction::SetPinned { pinned: false },
            )],
        };
        store.save(&workspace, &first).unwrap();
        store.save(&workspace, &second).unwrap();
        let primary = store.path_for(&workspace).unwrap();
        let backup = backup_path(&primary);
        let backup_bytes = fs::read(&backup).unwrap();
        let mut future: serde_json::Value =
            serde_json::from_slice(&fs::read(&primary).unwrap()).unwrap();
        future["schema_version"] = serde_json::Value::from(2);
        let future_bytes = serde_json::to_vec(&future).unwrap();
        fs::write(&primary, &future_bytes).unwrap();

        let error = store.recover(&workspace).unwrap_err();
        assert_eq!(error.code, ErrorCode::SchemaError);
        assert!(error.message.contains("newer than supported"));
        assert_eq!(fs::read(&primary).unwrap(), future_bytes);
        assert_eq!(fs::read(&backup).unwrap(), backup_bytes);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_rule_replacement_preserves_the_existing_persisted_file() {
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-window-rules-invalid-{}-{}",
            std::process::id(),
            RULE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let store = WindowRuleStore::new(&directory);
        let workspace = WorkspaceId::from("workbench");
        let original = WindowRuleSet {
            schema_version: 1,
            rules: vec![rule(
                "keep",
                1,
                WindowRuleAction::SetPinned { pinned: true },
            )],
        };
        store.save(&workspace, &original).unwrap();
        let mut invalid = original.clone();
        invalid.schema_version = 9;
        assert!(store.save(&workspace, &invalid).is_err());
        assert_eq!(store.load(&workspace).unwrap(), original);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn resolved_minimum_and_maximum_policies_must_be_compatible() {
        let context = context();
        let set = WindowRuleSet {
            schema_version: 1,
            rules: vec![
                rule(
                    "minimum",
                    2,
                    WindowRuleAction::MinimumSize(Size::new(800, 600)),
                ),
                rule(
                    "maximum",
                    2,
                    WindowRuleAction::MaximumSize(Size::new(400, 300)),
                ),
            ],
        };
        let evaluation = set.evaluate(&context).unwrap();
        assert!(WindowRulePlan::prepare(evaluation, &context).is_err());
    }

    fn context_document() -> LayoutDocument {
        document()
    }
}
