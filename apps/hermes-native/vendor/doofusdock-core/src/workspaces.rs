//! Bounded metadata catalog for host-managed canonical workspace documents.
//!
//! The catalog stores workspace identity and lifecycle only. Each workspace's
//! `LayoutDocument` remains the sole authority for panel ownership and layout.
use crate::{
    DockError, DockResult, ErrorCode, LayoutDocument, WorkspaceId, WorkspacePreferencesStore,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

const MAX_WORKSPACES: usize = 256;
const MAX_WORKSPACE_ID_BYTES: usize = 128;
const MAX_WORKSPACE_TITLE_BYTES: usize = 256;
const MAX_RETAINED_RULE_REVISIONS: usize = 16;
const MAX_RETAINED_FOCUS_HISTORY: usize = 16;
const MAX_TRACKED_WORKSPACE_TRANSFERS: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInfo {
    pub id: WorkspaceId,
    pub title: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_source: Option<WorkspaceId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCatalog {
    #[serde(default = "current_workspace_catalog_schema")]
    pub schema_version: u32,
    pub active: Option<WorkspaceId>,
    /// Stable display order for both active and archived workspaces.
    pub order: Vec<WorkspaceId>,
    pub workspaces: BTreeMap<WorkspaceId, WorkspaceInfo>,
}

fn current_workspace_catalog_schema() -> u32 {
    1
}

impl Default for WorkspaceCatalog {
    fn default() -> Self {
        Self {
            schema_version: 1,
            active: None,
            order: Vec::new(),
            workspaces: BTreeMap::new(),
        }
    }
}

impl WorkspaceCatalog {
    pub const CURRENT_SCHEMA: u32 = 1;

    pub fn validate(&self) -> DockResult<()> {
        if self.schema_version != Self::CURRENT_SCHEMA || self.workspaces.len() > MAX_WORKSPACES {
            return Err(workspace_error(
                if self.workspaces.len() > MAX_WORKSPACES {
                    ErrorCode::ResourceLimit
                } else {
                    ErrorCode::SchemaError
                },
                "Workspace catalog schema or size is unsupported.",
                format!(
                    "schema={}:count={}",
                    self.schema_version,
                    self.workspaces.len()
                ),
            ));
        }
        if self.order.len() != self.workspaces.len() {
            return Err(workspace_error(
                ErrorCode::SchemaError,
                "Workspace order must contain every catalog entry exactly once.",
                format!(
                    "order={}:entries={}",
                    self.order.len(),
                    self.workspaces.len()
                ),
            ));
        }
        let mut ordered = BTreeSet::new();
        for id in &self.order {
            if !ordered.insert(id) || !self.workspaces.contains_key(id) {
                return Err(workspace_error(
                    ErrorCode::SchemaError,
                    "Workspace order contains a duplicate or unknown ID.",
                    &id.0,
                ));
            }
        }
        for (id, info) in &self.workspaces {
            validate_id(id)?;
            validate_title(&info.title)?;
            if id != &info.id {
                return Err(workspace_error(
                    ErrorCode::SchemaError,
                    "Workspace catalog key does not match its entry ID.",
                    &id.0,
                ));
            }
            if let Some(source) = &info.template_source
                && (source == id || !self.workspaces.contains_key(source))
            {
                return Err(workspace_error(
                    ErrorCode::NotFound,
                    "Workspace template source does not exist.",
                    &source.0,
                ));
            }
        }
        if let Some(active) = &self.active
            && self
                .workspaces
                .get(active)
                .is_none_or(|workspace| workspace.archived)
        {
            return Err(workspace_error(
                ErrorCode::SchemaError,
                "Active workspace must reference a non-archived entry.",
                &active.0,
            ));
        }
        Ok(())
    }

    pub fn create(&mut self, id: WorkspaceId, title: impl Into<String>) -> DockResult<()> {
        self.mutate(|catalog| {
            catalog.insert(WorkspaceInfo {
                id,
                title: title.into(),
                archived: false,
                template_source: None,
            })
        })
    }

    /// Add a new workspace catalog entry tied to a layout-template source.
    /// The host instantiates the source document under the new workspace ID.
    pub fn duplicate_layout_template(
        &mut self,
        source: &WorkspaceId,
        id: WorkspaceId,
        title: impl Into<String>,
    ) -> DockResult<()> {
        self.mutate(|catalog| {
            if !catalog.workspaces.contains_key(source) {
                return Err(workspace_error(
                    ErrorCode::NotFound,
                    "Workspace template source does not exist.",
                    &source.0,
                ));
            }
            catalog.insert(WorkspaceInfo {
                id,
                title: title.into(),
                archived: false,
                template_source: Some(source.clone()),
            })
        })
    }

    pub fn rename(&mut self, id: &WorkspaceId, title: impl Into<String>) -> DockResult<()> {
        let title = title.into();
        self.mutate(|catalog| {
            validate_title(&title)?;
            let workspace = catalog.workspaces.get_mut(id).ok_or_else(|| {
                workspace_error(ErrorCode::NotFound, "Workspace was not found.", &id.0)
            })?;
            workspace.title = title;
            Ok(())
        })
    }

    pub fn activate(&mut self, id: &WorkspaceId) -> DockResult<bool> {
        let changed = self.active.as_ref() != Some(id);
        self.mutate(|catalog| {
            let workspace = catalog.workspaces.get(id).ok_or_else(|| {
                workspace_error(ErrorCode::NotFound, "Workspace was not found.", &id.0)
            })?;
            if workspace.archived {
                return Err(workspace_error(
                    ErrorCode::ConstraintViolation,
                    "An archived workspace cannot be activated.",
                    &id.0,
                ));
            }
            catalog.active = Some(id.clone());
            Ok(())
        })?;
        Ok(changed)
    }

    pub fn reorder(&mut self, id: &WorkspaceId, index: usize) -> DockResult<bool> {
        let old = self.order.iter().position(|candidate| candidate == id);
        let changed = old.is_some_and(|old| old != index);
        self.mutate(|catalog| {
            let old = catalog
                .order
                .iter()
                .position(|candidate| candidate == id)
                .ok_or_else(|| {
                    workspace_error(ErrorCode::NotFound, "Workspace was not found.", &id.0)
                })?;
            if index >= catalog.order.len() {
                return Err(workspace_error(
                    ErrorCode::InvalidLayout,
                    "Workspace order index is out of bounds.",
                    index.to_string(),
                ));
            }
            let entry = catalog.order.remove(old);
            catalog.order.insert(index, entry);
            Ok(())
        })?;
        Ok(changed)
    }

    pub fn archive(&mut self, id: &WorkspaceId) -> DockResult<()> {
        self.mutate(|catalog| {
            let workspace = catalog.workspaces.get_mut(id).ok_or_else(|| {
                workspace_error(ErrorCode::NotFound, "Workspace was not found.", &id.0)
            })?;
            if workspace.archived {
                return Ok(());
            }
            if catalog.active.as_ref() == Some(id) {
                let replacement = catalog.order.iter().find(|candidate| {
                    *candidate != id
                        && catalog
                            .workspaces
                            .get(*candidate)
                            .is_some_and(|entry| !entry.archived)
                });
                let Some(replacement) = replacement.cloned() else {
                    return Err(workspace_error(
                        ErrorCode::ConstraintViolation,
                        "The last active workspace cannot be archived.",
                        &id.0,
                    ));
                };
                catalog.active = Some(replacement);
            }
            catalog.workspaces.get_mut(id).unwrap().archived = true;
            Ok(())
        })
    }

    pub fn restore(&mut self, id: &WorkspaceId) -> DockResult<bool> {
        let was_archived = self.workspaces.get(id).is_some_and(|entry| entry.archived);
        self.mutate(|catalog| {
            let workspace = catalog.workspaces.get_mut(id).ok_or_else(|| {
                workspace_error(ErrorCode::NotFound, "Workspace was not found.", &id.0)
            })?;
            workspace.archived = false;
            Ok(())
        })?;
        Ok(was_archived)
    }

    /// Remove only when the owning canonical document has no retained content.
    pub fn delete(&mut self, id: &WorkspaceId, document: &LayoutDocument) -> DockResult<()> {
        self.mutate(|catalog| {
            if catalog.active.as_ref() == Some(id) {
                return Err(workspace_error(
                    ErrorCode::ConstraintViolation,
                    "The active workspace cannot be deleted.",
                    &id.0,
                ));
            }
            if &document.workspace != id
                || document.root.is_some()
                || !document.floating_windows.is_empty()
                || !document.auto_hide_panels.is_empty()
                || !document.closed_panels.is_empty()
                || !document.panels.is_empty()
            {
                return Err(workspace_error(
                    ErrorCode::ConstraintViolation,
                    "Workspace deletion requires its matching empty canonical document.",
                    &id.0,
                ));
            }
            if catalog.workspaces.remove(id).is_none() {
                return Err(workspace_error(
                    ErrorCode::NotFound,
                    "Workspace was not found.",
                    &id.0,
                ));
            }
            catalog.order.retain(|candidate| candidate != id);
            for entry in catalog.workspaces.values_mut() {
                if entry.template_source.as_ref() == Some(id) {
                    entry.template_source = None;
                }
            }
            Ok(())
        })
    }

    pub fn to_json(&self) -> DockResult<String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| {
            workspace_error(
                ErrorCode::SerializationError,
                "Workspace catalog encoding failed.",
                error.to_string(),
            )
        })
    }

    pub fn from_json(json: &str, maximum_bytes: usize) -> DockResult<Self> {
        if json.len() > maximum_bytes {
            return Err(workspace_error(
                ErrorCode::ResourceLimit,
                "Workspace catalog exceeds its byte limit.",
                json.len().to_string(),
            ));
        }
        let catalog: Self = serde_json::from_str(json).map_err(|error| {
            workspace_error(
                ErrorCode::SerializationError,
                "Workspace catalog decoding failed.",
                error.to_string(),
            )
        })?;
        catalog.validate()?;
        Ok(catalog)
    }

    fn insert(&mut self, info: WorkspaceInfo) -> DockResult<()> {
        validate_id(&info.id)?;
        validate_title(&info.title)?;
        if self.workspaces.len() >= MAX_WORKSPACES {
            return Err(workspace_error(
                ErrorCode::ResourceLimit,
                "Workspace catalog has reached its entry limit.",
                self.workspaces.len().to_string(),
            ));
        }
        if self.workspaces.contains_key(&info.id) {
            return Err(workspace_error(
                ErrorCode::DuplicateId,
                "Workspace ID already exists.",
                &info.id.0,
            ));
        }
        self.order.push(info.id.clone());
        self.workspaces.insert(info.id.clone(), info);
        if self.active.is_none() {
            self.active = self.order.first().cloned();
        }
        Ok(())
    }

    fn mutate(&mut self, action: impl FnOnce(&mut Self) -> DockResult<()>) -> DockResult<()> {
        let mut candidate = self.clone();
        action(&mut candidate)?;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
}

/// Host-owned collection of per-workspace services with exactly one active
/// document at a time. Each workspace keeps its own undo/event history.
pub struct WorkspaceDocumentHost {
    catalog: WorkspaceCatalog,
    services: BTreeMap<WorkspaceId, crate::DockingService>,
    focus_history: BTreeMap<WorkspaceId, VecDeque<crate::PanelId>>,
    rule_context_documents: BTreeMap<WorkspaceId, VecDeque<LayoutDocument>>,
    rule_event_subscriptions: BTreeMap<WorkspaceId, crate::EventSubscriptionId>,
    rule_event_consumers: BTreeMap<WorkspaceId, crate::WindowRuleEventConsumer>,
    transfer_endpoints: BTreeMap<String, WorkspaceTransferHistory>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceActivation {
    pub previous_workspace: WorkspaceId,
    pub active_workspace: WorkspaceId,
    pub snapshot: crate::ControlSnapshot,
    /// Last observed focused panel if it still exists in this document.
    pub focus_restore: Option<crate::PanelId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceScratchpadTransfer {
    pub transfer_id: String,
    pub source_workspace: WorkspaceId,
    pub target_workspace: WorkspaceId,
    pub window: crate::WindowId,
    pub panels: Vec<crate::PanelId>,
    pub source: crate::TransactionResult,
    pub target: crate::TransactionResult,
}

#[derive(Clone)]
struct WorkspaceTransferHistory {
    source: WorkspaceId,
    target: WorkspaceId,
    source_focus_before: VecDeque<crate::PanelId>,
    target_focus_before: VecDeque<crate::PanelId>,
    source_focus_after: VecDeque<crate::PanelId>,
    target_focus_after: VecDeque<crate::PanelId>,
}

impl WorkspaceDocumentHost {
    pub fn new(
        catalog: WorkspaceCatalog,
        documents: impl IntoIterator<Item = LayoutDocument>,
        history_capacity: usize,
    ) -> DockResult<Self> {
        catalog.validate()?;
        let mut services = BTreeMap::new();
        let mut rule_context_documents = BTreeMap::new();
        let mut rule_event_subscriptions = BTreeMap::new();
        let mut rule_event_consumers = BTreeMap::new();
        for document in documents {
            let workspace = document.workspace.clone();
            if !catalog.workspaces.contains_key(&workspace) {
                return Err(workspace_error(
                    ErrorCode::NotFound,
                    "Workspace document is not registered in the catalog.",
                    &workspace.0,
                ));
            }
            let mut service = crate::DockingService::new(document.clone(), history_capacity)?;
            let subscription = service.subscribe_events()?;
            retain_rule_context_document(&mut rule_context_documents, document);
            rule_event_subscriptions.insert(workspace.clone(), subscription);
            rule_event_consumers
                .insert(workspace.clone(), crate::WindowRuleEventConsumer::default());
            if services.insert(workspace.clone(), service).is_some() {
                return Err(workspace_error(
                    ErrorCode::DuplicateId,
                    "Only one canonical document may be registered per workspace.",
                    workspace.0,
                ));
            }
        }
        let active = catalog.active.as_ref().ok_or_else(|| {
            workspace_error(
                ErrorCode::ConstraintViolation,
                "Workspace document host requires an active workspace.",
                "catalog.active",
            )
        })?;
        if !services.contains_key(active) {
            return Err(workspace_error(
                ErrorCode::NotFound,
                "Active workspace has no loaded canonical document.",
                &active.0,
            ));
        }
        Ok(Self {
            catalog,
            services,
            focus_history: BTreeMap::new(),
            rule_context_documents,
            rule_event_subscriptions,
            rule_event_consumers,
            transfer_endpoints: BTreeMap::new(),
        })
    }

    pub fn catalog(&self) -> &WorkspaceCatalog {
        &self.catalog
    }

    pub fn active_workspace(&self) -> &WorkspaceId {
        self.catalog
            .active
            .as_ref()
            .expect("validated host always has an active workspace")
    }

    pub fn snapshot(&self) -> crate::ControlSnapshot {
        self.services[self.active_workspace()].snapshot()
    }

    pub fn snapshots(&self) -> BTreeMap<WorkspaceId, crate::ControlSnapshot> {
        self.services
            .iter()
            .map(|(workspace, service)| (workspace.clone(), service.snapshot()))
            .collect()
    }

    /// Return names that identify exactly one pinned scratchpad in a loaded,
    /// non-archived workspace. Ambiguous names are omitted from pickers and
    /// remain rejected by `summon_pinned_scratchpad`.
    pub fn pinned_scratchpad_names(&self) -> Vec<String> {
        let mut counts = BTreeMap::<String, usize>::new();
        for (workspace, service) in &self.services {
            if self
                .catalog
                .workspaces
                .get(workspace)
                .is_none_or(|info| info.archived)
            {
                continue;
            }
            for window in &service.snapshot().document.floating_windows {
                if let Some(scratchpad) = &window.scratchpad
                    && scratchpad.pinned_across_workspaces
                {
                    *counts.entry(scratchpad.name.clone()).or_default() += 1;
                }
            }
        }
        counts
            .into_iter()
            .filter_map(|(name, count)| (count == 1).then_some(name))
            .collect()
    }

    /// Rebuild the bounded, revision-exact rule contexts retained by this host.
    /// Requests older than the retention window fail closed.
    pub fn rule_contexts_for_revision(
        &self,
        workspace: &WorkspaceId,
        revision: u64,
        host_capabilities: crate::HostCapabilities,
    ) -> DockResult<Vec<crate::WindowRuleContext>> {
        let document = self
            .rule_context_documents
            .get(workspace)
            .and_then(|documents| {
                documents
                    .iter()
                    .find(|document| document.revision == revision)
            })
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Rule context revision is outside the retained workspace history.",
                    format!("workspace={}:revision={revision}", workspace.0),
                )
            })?;
        crate::WindowRuleContext::for_document(document, host_capabilities)
    }

    /// Poll and evaluate only the events owned by this workspace host. This
    /// method never commits rule actions or invokes host callbacks.
    pub fn poll_rule_events(
        &mut self,
        workspace: &WorkspaceId,
        rules: &crate::WindowRuleSet,
        host_capabilities: crate::HostCapabilities,
        limit: usize,
    ) -> DockResult<Vec<crate::TriggeredRuleEvaluation>> {
        let subscription = self
            .rule_event_subscriptions
            .get(workspace)
            .cloned()
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace rule-event subscription was not found.",
                    &workspace.0,
                )
            })?;
        let batch = self
            .services
            .get_mut(workspace)
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace has no loaded event service.",
                    &workspace.0,
                )
            })?
            .read_events(subscription, limit)?;
        let revisions = batch
            .events
            .iter()
            .map(|event| event.revision)
            .collect::<BTreeSet<_>>();
        let mut contexts = Vec::new();
        for revision in revisions {
            contexts.extend(self.rule_contexts_for_revision(
                workspace,
                revision,
                host_capabilities.clone(),
            )?);
        }
        self.rule_event_consumers
            .get_mut(workspace)
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace rule-event consumer was not found.",
                    &workspace.0,
                )
            })?
            .evaluate_batch(rules, &batch, &contexts)
    }

    pub fn transact(
        &mut self,
        expected_workspace: &WorkspaceId,
        expected_revision: u64,
        command: crate::CommandId,
        operations: impl IntoIterator<Item = crate::Operation>,
    ) -> DockResult<crate::TransactionResult> {
        self.ensure_active_workspace(expected_workspace, "Transaction")?;
        let result = self
            .active_service_mut()
            .transact(expected_revision, command, operations)?;
        self.retain_active_rule_context_revision(result.document.clone());
        Ok(result)
    }

    /// Apply a complete target document to the active workspace as one undoable operation.
    pub fn apply_layout(
        &mut self,
        expected_revision: u64,
        target: &LayoutDocument,
    ) -> DockResult<crate::TransactionResult> {
        if target.workspace != *self.active_workspace() {
            return Err(workspace_error(
                ErrorCode::Conflict,
                "Target layout belongs to a different active workspace.",
                format!(
                    "expected={}:actual={}",
                    self.active_workspace().0,
                    target.workspace.0
                ),
            ));
        }
        let result = self
            .active_service_mut()
            .apply_layout(expected_revision, target)?;
        self.retain_active_rule_context_revision(result.document.clone());
        Ok(result)
    }

    pub fn undo(
        &mut self,
        expected_workspace: &WorkspaceId,
        expected_revision: u64,
    ) -> DockResult<crate::TransactionResult> {
        self.ensure_active_workspace(expected_workspace, "Undo")?;
        let active = self.active_workspace().clone();
        let active_service = &self.services[&active];
        if let Some(transfer_id) = active_service.top_undo_transfer_id().map(str::to_owned) {
            let transfer = self
                .transfer_endpoints
                .get(&transfer_id)
                .cloned()
                .ok_or_else(|| {
                    workspace_error(
                        ErrorCode::Conflict,
                        "Paired scratchpad transfer metadata is unavailable; refusing a one-sided undo.",
                        &transfer_id,
                    )
                })?;
            let source_snapshot = self.services[&transfer.source].snapshot();
            let target_snapshot = self.services[&transfer.target].snapshot();
            if source_snapshot.revision == expected_revision && active == transfer.source
                || target_snapshot.revision == expected_revision && active == transfer.target
            {
                if self.services[&transfer.source].top_undo_transfer_id()
                    != Some(transfer_id.as_str())
                    || self.services[&transfer.target].top_undo_transfer_id()
                        != Some(transfer_id.as_str())
                {
                    return Err(workspace_error(
                        ErrorCode::Conflict,
                        "Both workspace histories must reach the same transfer before undo.",
                        &transfer_id,
                    ));
                }
                let mut source = self.services[&transfer.source].staged_clone();
                let mut target = self.services[&transfer.target].staged_clone();
                let source_result = source.undo_transfer(source_snapshot.revision, &transfer_id)?;
                let target_result = target.undo_transfer(target_snapshot.revision, &transfer_id)?;
                self.services.insert(transfer.source.clone(), source);
                self.services.insert(transfer.target.clone(), target);
                self.focus_history.insert(
                    transfer.source.clone(),
                    transfer.source_focus_before.clone(),
                );
                self.focus_history.insert(
                    transfer.target.clone(),
                    transfer.target_focus_before.clone(),
                );
                retain_rule_context_document(
                    &mut self.rule_context_documents,
                    source_result.document.clone(),
                );
                retain_rule_context_document(
                    &mut self.rule_context_documents,
                    target_result.document.clone(),
                );
                return Ok(if active == transfer.source {
                    source_result
                } else {
                    target_result
                });
            }
        }
        let result = self.active_service_mut().undo(expected_revision)?;
        self.retain_active_rule_context_revision(result.document.clone());
        Ok(result)
    }

    pub fn redo(
        &mut self,
        expected_workspace: &WorkspaceId,
        expected_revision: u64,
    ) -> DockResult<crate::TransactionResult> {
        self.ensure_active_workspace(expected_workspace, "Redo")?;
        let active = self.active_workspace().clone();
        let active_service = &self.services[&active];
        if let Some(transfer_id) = active_service.top_redo_transfer_id().map(str::to_owned) {
            let transfer = self
                .transfer_endpoints
                .get(&transfer_id)
                .cloned()
                .ok_or_else(|| {
                    workspace_error(
                        ErrorCode::Conflict,
                        "Paired scratchpad transfer metadata is unavailable; refusing a one-sided redo.",
                        &transfer_id,
                    )
                })?;
            let source_snapshot = self.services[&transfer.source].snapshot();
            let target_snapshot = self.services[&transfer.target].snapshot();
            if source_snapshot.revision == expected_revision && active == transfer.source
                || target_snapshot.revision == expected_revision && active == transfer.target
            {
                let source_has_transfer =
                    self.services[&transfer.source].contains_transfer_history(&transfer_id);
                let target_has_transfer =
                    self.services[&transfer.target].contains_transfer_history(&transfer_id);
                if !source_has_transfer || !target_has_transfer {
                    let mut source = self.services[&transfer.source].staged_clone();
                    let mut target = self.services[&transfer.target].staged_clone();
                    source.discard_transfer_redo(&transfer_id);
                    target.discard_transfer_redo(&transfer_id);
                    self.services.insert(transfer.source.clone(), source);
                    self.services.insert(transfer.target.clone(), target);
                    self.transfer_endpoints.remove(&transfer_id);
                } else if self.services[&transfer.source].top_redo_transfer_id()
                    != Some(transfer_id.as_str())
                    || self.services[&transfer.target].top_redo_transfer_id()
                        != Some(transfer_id.as_str())
                {
                    return Err(workspace_error(
                        ErrorCode::Conflict,
                        "Both workspace histories must reach the same transfer before redo.",
                        &transfer_id,
                    ));
                } else {
                    let mut source = self.services[&transfer.source].staged_clone();
                    let mut target = self.services[&transfer.target].staged_clone();
                    let source_result =
                        source.redo_transfer(source_snapshot.revision, &transfer_id)?;
                    let target_result =
                        target.redo_transfer(target_snapshot.revision, &transfer_id)?;
                    self.services.insert(transfer.source.clone(), source);
                    self.services.insert(transfer.target.clone(), target);
                    self.focus_history
                        .insert(transfer.source.clone(), transfer.source_focus_after.clone());
                    self.focus_history
                        .insert(transfer.target.clone(), transfer.target_focus_after.clone());
                    retain_rule_context_document(
                        &mut self.rule_context_documents,
                        source_result.document.clone(),
                    );
                    retain_rule_context_document(
                        &mut self.rule_context_documents,
                        target_result.document.clone(),
                    );
                    return Ok(if active == transfer.source {
                        source_result
                    } else {
                        target_result
                    });
                }
            }
        }
        let result = self.active_service_mut().redo(expected_revision)?;
        self.retain_active_rule_context_revision(result.document.clone());
        Ok(result)
    }

    pub fn apply_profile(
        &mut self,
        expected_workspace: &WorkspaceId,
        expected_revision: u64,
        profile: &crate::WorkspaceProfile,
        assignments: &BTreeMap<String, Vec<crate::PanelId>>,
    ) -> DockResult<crate::ProfileApplication> {
        self.ensure_active_workspace(expected_workspace, "Profile application")?;
        let result =
            self.active_service_mut()
                .apply_profile(expected_revision, profile, assignments)?;
        self.retain_active_rule_context_revision(result.document.clone());
        Ok(result)
    }

    pub fn commit_window_rule(
        &mut self,
        expected_workspace: &WorkspaceId,
        plan: crate::WindowRulePlan,
        command: crate::CommandId,
    ) -> DockResult<crate::WindowRuleCommit> {
        self.ensure_active_workspace(expected_workspace, "Window-rule commit")?;
        let result = plan.commit(self.active_service_mut(), command)?;
        self.retain_active_rule_context_revision(self.snapshot().document);
        Ok(result)
    }

    /// Explicitly commit one event-triggered rule evaluation for the active
    /// workspace. The plan is rebuilt against the exact retained event
    /// revision; the rule-origin command is staged for feedback suppression
    /// and becomes active only if the canonical commit succeeds.
    pub fn commit_triggered_rule(
        &mut self,
        workspace: &WorkspaceId,
        triggered: crate::TriggeredRuleEvaluation,
        host_capabilities: crate::HostCapabilities,
        command: crate::CommandId,
    ) -> DockResult<crate::WindowRuleCommit> {
        if self.active_workspace() != workspace {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Triggered rule belongs to an inactive workspace.",
                format!(
                    "requested={}:active={}",
                    workspace.0,
                    self.active_workspace().0
                ),
            ));
        }
        let context = self
            .rule_contexts_for_revision(
                workspace,
                triggered.evaluation.base_revision,
                host_capabilities,
            )?
            .into_iter()
            .find(|context| context.panel_id == triggered.panel)
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Triggered rule panel is not present in its retained event revision.",
                    &triggered.panel.0,
                )
            })?;
        let plan = crate::WindowRulePlan::prepare(triggered.evaluation, &context)?;
        let mut staged_consumer = self
            .rule_event_consumers
            .get(workspace)
            .cloned()
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace rule-event consumer was not found.",
                    &workspace.0,
                )
            })?;
        staged_consumer.suppress_command(command.clone())?;
        let result = plan.commit(
            self.services.get_mut(workspace).ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace document service was not found.",
                    &workspace.0,
                )
            })?,
            command,
        )?;
        self.rule_event_consumers
            .insert(workspace.clone(), staged_consumer);
        self.retain_active_rule_context_revision(self.snapshot().document);
        Ok(result)
    }

    /// Replace the active document after an explicit load/reset. The target
    /// stays in the same workspace and starts at a newer revision with fresh
    /// history/event cursors.
    pub fn replace_active_document(
        &mut self,
        expected_workspace: &WorkspaceId,
        expected_revision: u64,
        mut document: LayoutDocument,
        history_capacity: usize,
    ) -> DockResult<crate::ControlSnapshot> {
        self.ensure_active_workspace(expected_workspace, "Document replacement")?;
        let workspace = self.active_workspace().clone();
        let service = self
            .services
            .get(&workspace)
            .expect("active service exists");
        let current = service.snapshot();
        if current.revision != expected_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Workspace document changed before replacement.",
                format!("expected={expected_revision}:actual={}", current.revision),
            ));
        }
        if document.workspace != workspace {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Replacement document belongs to a different workspace.",
                format!("expected={}:actual={}", workspace.0, document.workspace.0),
            ));
        }
        document.revision = current.revision.checked_add(1).ok_or_else(|| {
            workspace_error(
                ErrorCode::ResourceLimit,
                "Workspace revision overflow prevents document replacement.",
                &workspace.0,
            )
        })?;
        let mut replacement = crate::DockingService::new(document, history_capacity)?;
        let subscription = replacement.subscribe_events()?;
        self.services.insert(workspace.clone(), replacement);
        self.rule_event_subscriptions
            .insert(workspace.clone(), subscription);
        self.rule_event_consumers
            .insert(workspace.clone(), crate::WindowRuleEventConsumer::default());
        self.retain_active_rule_context_revision(self.services[&workspace].snapshot().document);
        Ok(self.services[&workspace].snapshot())
    }

    pub fn record_focus(
        &mut self,
        expected_workspace: &WorkspaceId,
        expected_revision: u64,
        panel: crate::PanelId,
    ) -> DockResult<()> {
        if self.active_workspace() != expected_workspace {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Focus observation belongs to an inactive workspace.",
                format!(
                    "expected={}:active={}",
                    expected_workspace.0,
                    self.active_workspace().0
                ),
            ));
        }
        let snapshot = self.snapshot();
        if snapshot.revision != expected_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Focus observation was reported for a stale workspace revision.",
                format!("expected={expected_revision}:actual={}", snapshot.revision),
            ));
        }
        if snapshot.document.descriptor(&panel).is_none()
            || !panel_is_focusable(&snapshot.document, &panel)
        {
            return Err(workspace_error(
                ErrorCode::NotFound,
                "Focused panel is not placed in the active workspace.",
                &panel.0,
            ));
        }
        let history = self
            .focus_history
            .entry(expected_workspace.clone())
            .or_default();
        history.retain(|candidate| candidate != &panel);
        history.push_front(panel);
        history.truncate(MAX_RETAINED_FOCUS_HISTORY);
        Ok(())
    }

    /// Return the most recently focused placed panel, optionally skipping the
    /// panel being dismissed. Focus history is transient host state, bounded
    /// per workspace, and never changes the canonical document.
    pub fn focus_restore_candidate(
        &self,
        workspace: &WorkspaceId,
        excluding: Option<&crate::PanelId>,
    ) -> Option<crate::PanelId> {
        let excluded = excluding.cloned().into_iter().collect();
        self.focus_restore_candidate_excluding(workspace, &excluded)
    }

    /// Return the most recent still-placed focus target while skipping a set
    /// of panels, such as all members of a closing scratchpad window.
    pub fn focus_restore_candidate_excluding(
        &self,
        workspace: &WorkspaceId,
        excluded: &BTreeSet<crate::PanelId>,
    ) -> Option<crate::PanelId> {
        let document = self.services.get(workspace)?.snapshot().document;
        self.focus_history.get(workspace)?.iter().find_map(|panel| {
            (!excluded.contains(panel)
                && document.descriptor(panel).is_some()
                && panel_is_focusable(&document, panel))
            .then(|| panel.clone())
        })
    }

    /// Activate and summon the unique pinned scratchpad with `name`, wherever
    /// its canonical window is owned. This changes workspace activation and
    /// visibility in one host call while preserving one document/window/panel
    /// identity; it never copies content between workspace documents.
    pub fn summon_pinned_scratchpad(
        &mut self,
        expected_active: &WorkspaceId,
        name: &str,
        command: crate::CommandId,
    ) -> DockResult<WorkspaceActivation> {
        let current = self.active_workspace().clone();
        if &current != expected_active {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Pinned scratchpad summon was prepared from a stale active workspace.",
                format!("expected={}:actual={}", expected_active.0, current.0),
            ));
        }
        if name.is_empty() || name.len() > 64 {
            return Err(workspace_error(
                ErrorCode::ConstraintViolation,
                "Pinned scratchpad name must be a nonempty token of at most 64 bytes.",
                name,
            ));
        }

        let mut matches = self.services.iter().filter_map(|(workspace, service)| {
            service
                .snapshot()
                .document
                .floating_windows
                .iter()
                .find(|window| {
                    window.scratchpad.as_ref().is_some_and(|scratchpad| {
                        scratchpad.name == name && scratchpad.pinned_across_workspaces
                    })
                })
                .map(|window| (workspace.clone(), window.id.clone(), window.root.clone()))
        });
        let Some((target, window, root)) = matches.next() else {
            return Err(workspace_error(
                ErrorCode::NotFound,
                "Pinned scratchpad was not found in any loaded workspace.",
                name,
            ));
        };
        if matches.next().is_some() {
            return Err(workspace_error(
                ErrorCode::Conflict,
                "Pinned scratchpad name is ambiguous across loaded workspaces.",
                name,
            ));
        }

        let catalog_info = self.catalog.workspaces.get(&target).ok_or_else(|| {
            workspace_error(
                ErrorCode::NotFound,
                "Scratchpad workspace was not found.",
                &target.0,
            )
        })?;
        if catalog_info.archived {
            return Err(workspace_error(
                ErrorCode::ConstraintViolation,
                "Pinned scratchpad belongs to an archived workspace.",
                &target.0,
            ));
        }
        let mut next_catalog = self.catalog.clone();
        next_catalog.activate(&target)?;
        let service = self.services.get_mut(&target).ok_or_else(|| {
            workspace_error(
                ErrorCode::NotFound,
                "Pinned scratchpad workspace has no loaded document.",
                &target.0,
            )
        })?;
        let snapshot = service.snapshot();
        let existing = snapshot
            .document
            .floating_windows
            .iter()
            .find(|candidate| candidate.id == window)
            .and_then(|candidate| candidate.scratchpad.clone())
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::Conflict,
                    "Pinned scratchpad changed while its summon was being prepared.",
                    &window.0,
                )
            })?;
        let mut visible = existing;
        visible.visible = true;
        let result = service.transact(
            snapshot.revision,
            command,
            [
                crate::Operation::SetScratchpad {
                    window: window.clone(),
                    config: Some(visible),
                },
                crate::Operation::ActivateFloatingWindow { window },
            ],
        )?;
        self.catalog = next_catalog;
        retain_rule_context_document(&mut self.rule_context_documents, result.document.clone());
        Ok(WorkspaceActivation {
            previous_workspace: current,
            active_workspace: target,
            snapshot: self.services[self.active_workspace()].snapshot(),
            focus_restore: first_panel_in_node(&root),
        })
    }

    /// Move the unique pinned scratchpad named `name` into another loaded
    /// workspace. The source and destination documents commit together, retain
    /// the same window/panel/node identities, and share a paired undo/redo edge.
    pub fn move_pinned_scratchpad(
        &mut self,
        expected_active: &WorkspaceId,
        expected_source_revision: u64,
        expected_target_revision: u64,
        name: &str,
        target_workspace: &WorkspaceId,
        command: crate::CommandId,
    ) -> DockResult<WorkspaceScratchpadTransfer> {
        self.prune_transfer_endpoints();
        if self.active_workspace() != expected_active {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Scratchpad move was prepared from a stale active workspace.",
                format!(
                    "expected={}:actual={}",
                    expected_active.0,
                    self.active_workspace().0
                ),
            ));
        }
        if name.is_empty() || name.len() > 64 || command.is_empty() || command.0.len() > 128 {
            return Err(workspace_error(
                ErrorCode::SchemaError,
                "Scratchpad move requires a valid name and bounded command ID.",
                name,
            ));
        }
        if self.transfer_endpoints.contains_key(&command.0) {
            return Err(workspace_error(
                ErrorCode::DuplicateId,
                "Workspace transfer command ID has already been used.",
                &command.0,
            ));
        }
        if self.transfer_endpoints.len() >= MAX_TRACKED_WORKSPACE_TRANSFERS {
            return Err(workspace_error(
                ErrorCode::ResourceLimit,
                "Workspace transfer history reached its bounded tracking limit.",
                MAX_TRACKED_WORKSPACE_TRANSFERS.to_string(),
            ));
        }
        if !self
            .catalog
            .workspaces
            .get(target_workspace)
            .is_some_and(|info| !info.archived)
            || !self.services.contains_key(target_workspace)
        {
            return Err(workspace_error(
                ErrorCode::NotFound,
                "Scratchpad target must be a loaded, non-archived workspace.",
                &target_workspace.0,
            ));
        }

        let mut owner = None;
        for (workspace, service) in &self.services {
            let Some(info) = self.catalog.workspaces.get(workspace) else {
                continue;
            };
            if info.archived {
                continue;
            }
            for window in &service.snapshot().document.floating_windows {
                if window
                    .scratchpad
                    .as_ref()
                    .is_some_and(|config| config.name == name && config.pinned_across_workspaces)
                {
                    if owner.is_some() {
                        return Err(workspace_error(
                            ErrorCode::Conflict,
                            "Pinned scratchpad name is ambiguous across loaded workspaces.",
                            name,
                        ));
                    }
                    owner = Some((workspace.clone(), window.clone()));
                }
            }
        }
        let (source_workspace, window) = owner.ok_or_else(|| {
            workspace_error(
                ErrorCode::NotFound,
                "Pinned scratchpad was not found in any loaded workspace.",
                name,
            )
        })?;
        if &source_workspace == target_workspace {
            return Err(workspace_error(
                ErrorCode::ConstraintViolation,
                "Pinned scratchpad already belongs to the requested workspace.",
                &target_workspace.0,
            ));
        }

        let source_snapshot = self.services[&source_workspace].snapshot();
        let target_snapshot = self.services[target_workspace].snapshot();
        if source_snapshot.revision != expected_source_revision
            || target_snapshot.revision != expected_target_revision
        {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Scratchpad move was prepared from stale document revisions.",
                format!(
                    "source={}:{} expected={expected_source_revision};target={}:{} expected={expected_target_revision}",
                    source_workspace.0,
                    source_snapshot.revision,
                    target_workspace.0,
                    target_snapshot.revision
                ),
            ));
        }
        let mut panel_ids = BTreeSet::new();
        let mut node_ids = BTreeSet::new();
        collect_subtree_identity(&window.root, &mut node_ids, &mut panel_ids);
        if panel_ids.is_empty() {
            return Err(workspace_error(
                ErrorCode::InvalidLayout,
                "A scratchpad transfer requires at least one canonical panel.",
                name,
            ));
        }
        for (workspace, service) in &self.services {
            if workspace == &source_workspace {
                continue;
            }
            let document = service.snapshot().document;
            if panel_ids
                .iter()
                .any(|panel| document.descriptor(panel).is_some())
            {
                return Err(workspace_error(
                    ErrorCode::DuplicateId,
                    "A transferred panel identity is already registered in another loaded workspace.",
                    name,
                ));
            }
        }
        let mut target_node_ids = BTreeSet::new();
        let mut target_panel_ids = BTreeSet::new();
        if let Some(root) = &target_snapshot.document.root {
            collect_subtree_identity(root, &mut target_node_ids, &mut target_panel_ids);
        }
        for target_window in &target_snapshot.document.floating_windows {
            collect_subtree_identity(
                &target_window.root,
                &mut target_node_ids,
                &mut target_panel_ids,
            );
            if target_window.id == window.id
                || target_window
                    .scratchpad
                    .as_ref()
                    .is_some_and(|config| config.name == name)
            {
                return Err(workspace_error(
                    ErrorCode::DuplicateId,
                    "Target workspace already owns this window identity or scratchpad name.",
                    name,
                ));
            }
        }
        if node_ids.iter().any(|id| target_node_ids.contains(id))
            || panel_ids.iter().any(|id| target_panel_ids.contains(id))
            || panel_ids
                .iter()
                .any(|id| target_snapshot.document.descriptor(id).is_some())
        {
            return Err(workspace_error(
                ErrorCode::DuplicateId,
                "Target workspace already owns a transferred node or panel identity.",
                name,
            ));
        }

        let descriptors = source_snapshot
            .document
            .panels
            .iter()
            .filter(|descriptor| panel_ids.contains(&descriptor.id))
            .cloned()
            .collect::<Vec<_>>();
        if descriptors.len() != panel_ids.len() {
            return Err(workspace_error(
                ErrorCode::InvalidLayout,
                "Scratchpad subtree contains a panel without exactly one source descriptor.",
                name,
            ));
        }
        let mut source_next = source_snapshot.document.clone();
        let mut target_next = target_snapshot.document.clone();
        source_next
            .floating_windows
            .retain(|candidate| candidate.id != window.id);
        source_next.panels = Arc::new(
            source_next
                .panels
                .iter()
                .filter(|descriptor| !panel_ids.contains(&descriptor.id))
                .cloned()
                .collect(),
        );
        source_next
            .pinned_panels
            .retain(|panel| !panel_ids.contains(panel));
        source_next
            .closed_panels
            .retain(|panel| !panel_ids.contains(panel));
        source_next
            .auto_hide_panels
            .retain(|drawer| !panel_ids.contains(&drawer.panel));
        let moved_maximized = source_next
            .maximized_panel
            .as_ref()
            .is_some_and(|panel| panel_ids.contains(panel));
        if moved_maximized && target_next.maximized_panel.is_some() {
            return Err(workspace_error(
                ErrorCode::Conflict,
                "Target workspace already has a maximized panel; refusing to lose maximize state.",
                name,
            ));
        }
        if moved_maximized {
            target_next.maximized_panel = source_next.maximized_panel.take();
        }
        target_next.panels = Arc::new(
            target_next
                .panels
                .iter()
                .cloned()
                .chain(descriptors)
                .collect(),
        );
        for panel in source_snapshot
            .document
            .pinned_panels
            .intersection(&panel_ids)
        {
            target_next.pinned_panels.insert(panel.clone());
        }
        target_next.floating_windows.push(window.clone());

        crate::validate_layout(&source_next)?;
        crate::validate_layout(&target_next)?;
        let mut next_catalog = self.catalog.clone();
        next_catalog.activate(target_workspace)?;
        let mut source_service = self.services[&source_workspace].staged_clone();
        let mut target_service = self.services[target_workspace].staged_clone();
        let source_result = source_service.apply_transfer_layout(
            expected_source_revision,
            &source_next,
            &command.0,
        )?;
        let target_result = target_service.apply_transfer_layout(
            expected_target_revision,
            &target_next,
            &command.0,
        )?;
        if source_result.diff.is_empty() || target_result.diff.is_empty() {
            return Err(workspace_error(
                ErrorCode::Conflict,
                "Scratchpad move did not produce changes in both canonical documents.",
                &command.0,
            ));
        }

        self.services
            .insert(source_workspace.clone(), source_service);
        self.services
            .insert(target_workspace.clone(), target_service);
        let panels = panel_ids.into_iter().collect::<Vec<_>>();
        let source_focus_before = self
            .focus_history
            .get(&source_workspace)
            .cloned()
            .unwrap_or_default();
        let target_focus_before = self
            .focus_history
            .get(target_workspace)
            .cloned()
            .unwrap_or_default();
        let mut source_focus = self
            .focus_history
            .remove(&source_workspace)
            .unwrap_or_default();
        let mut moved_focus = VecDeque::new();
        source_focus.retain(|panel| {
            if panels.contains(panel) {
                moved_focus.push_back(panel.clone());
                false
            } else {
                true
            }
        });
        self.focus_history
            .insert(source_workspace.clone(), source_focus);
        if !moved_focus.is_empty() {
            let target_focus = self
                .focus_history
                .entry(target_workspace.clone())
                .or_default();
            for panel in moved_focus.into_iter().rev() {
                target_focus.retain(|existing| existing != &panel);
                target_focus.push_front(panel);
            }
            target_focus.truncate(MAX_RETAINED_FOCUS_HISTORY);
        }
        self.transfer_endpoints.insert(
            command.0.clone(),
            WorkspaceTransferHistory {
                source: source_workspace.clone(),
                target: target_workspace.clone(),
                source_focus_before,
                target_focus_before,
                source_focus_after: self
                    .focus_history
                    .get(&source_workspace)
                    .cloned()
                    .unwrap_or_default(),
                target_focus_after: self
                    .focus_history
                    .get(target_workspace)
                    .cloned()
                    .unwrap_or_default(),
            },
        );
        retain_rule_context_document(
            &mut self.rule_context_documents,
            source_result.document.clone(),
        );
        retain_rule_context_document(
            &mut self.rule_context_documents,
            target_result.document.clone(),
        );
        self.catalog = next_catalog;
        Ok(WorkspaceScratchpadTransfer {
            transfer_id: command.0,
            source_workspace,
            target_workspace: target_workspace.clone(),
            window: window.id,
            panels,
            source: source_result,
            target: target_result,
        })
    }

    pub fn activate(
        &mut self,
        expected_active: &WorkspaceId,
        target: &WorkspaceId,
    ) -> DockResult<WorkspaceActivation> {
        let current = self.active_workspace().clone();
        if &current != expected_active {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Workspace activation was prepared from a stale active workspace.",
                format!("expected={}:actual={}", expected_active.0, current.0),
            ));
        }
        let info = self.catalog.workspaces.get(target).ok_or_else(|| {
            workspace_error(ErrorCode::NotFound, "Workspace was not found.", &target.0)
        })?;
        if info.archived {
            return Err(workspace_error(
                ErrorCode::ConstraintViolation,
                "Archived workspace cannot be activated.",
                &target.0,
            ));
        }
        let service = self.services.get(target).ok_or_else(|| {
            workspace_error(
                ErrorCode::NotFound,
                "Workspace has no loaded canonical document.",
                &target.0,
            )
        })?;
        self.catalog.activate(target)?;
        let snapshot = service.snapshot();
        let focus_restore = self.focus_restore_candidate(target, None);
        Ok(WorkspaceActivation {
            previous_workspace: current,
            active_workspace: target.clone(),
            snapshot,
            focus_restore,
        })
    }

    /// Create an empty canonical document and activate it as a new workspace.
    pub fn create_workspace(
        &mut self,
        expected_active: &WorkspaceId,
        id: WorkspaceId,
        title: impl Into<String>,
    ) -> DockResult<WorkspaceActivation> {
        let current = self.active_workspace().clone();
        if &current != expected_active {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Workspace creation was prepared from a stale active workspace.",
                format!("expected={}:actual={}", expected_active.0, current.0),
            ));
        }
        let mut catalog = self.catalog.clone();
        catalog.create(id.clone(), title)?;
        catalog.activate(&id)?;
        let document = LayoutDocument::new(format!("workspace-{}", id.0), id.0.clone());
        let mut service = crate::DockingService::new(document, 64)?;
        let snapshot = service.snapshot();
        let subscription = service.subscribe_events()?;
        self.catalog = catalog;
        self.services.insert(id.clone(), service);
        self.rule_event_subscriptions
            .insert(id.clone(), subscription);
        self.rule_event_consumers
            .insert(id.clone(), crate::WindowRuleEventConsumer::default());
        retain_rule_context_document(&mut self.rule_context_documents, snapshot.document.clone());
        Ok(WorkspaceActivation {
            previous_workspace: current,
            active_workspace: id,
            snapshot,
            focus_restore: None,
        })
    }

    pub fn rename_workspace(
        &mut self,
        id: &WorkspaceId,
        title: impl Into<String>,
    ) -> DockResult<()> {
        self.catalog.rename(id, title)
    }

    /// Archive the active workspace and atomically activate another loaded workspace.
    pub fn archive_active_workspace(
        &mut self,
        expected_active: &WorkspaceId,
    ) -> DockResult<WorkspaceActivation> {
        let current = self.active_workspace().clone();
        if &current != expected_active {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Workspace archive was prepared from a stale active workspace.",
                format!("expected={}:actual={}", expected_active.0, current.0),
            ));
        }
        let mut catalog = self.catalog.clone();
        catalog.archive(&current)?;
        let target = catalog.active.clone().ok_or_else(|| {
            workspace_error(
                ErrorCode::ConstraintViolation,
                "Archiving left the workspace catalog without an active document.",
                &current.0,
            )
        })?;
        let service = self.services.get(&target).ok_or_else(|| {
            workspace_error(
                ErrorCode::NotFound,
                "Replacement workspace has no loaded canonical document.",
                &target.0,
            )
        })?;
        let snapshot = service.snapshot();
        let focus_restore = self.focus_restore_candidate(&target, None);
        self.catalog = catalog;
        Ok(WorkspaceActivation {
            previous_workspace: current,
            active_workspace: target,
            snapshot,
            focus_restore,
        })
    }

    pub fn restore_workspace(&mut self, id: &WorkspaceId) -> DockResult<bool> {
        self.catalog.restore(id)
    }

    /// Delete only an inactive workspace whose canonical document retains no content.
    pub fn delete_workspace(&mut self, id: &WorkspaceId) -> DockResult<()> {
        let service = self.services.get(id).ok_or_else(|| {
            workspace_error(
                ErrorCode::NotFound,
                "Workspace has no loaded canonical document.",
                &id.0,
            )
        })?;
        let mut catalog = self.catalog.clone();
        catalog.delete(id, &service.snapshot().document)?;
        self.catalog = catalog;
        self.services.remove(id);
        self.focus_history.remove(id);
        self.rule_context_documents.remove(id);
        self.rule_event_subscriptions.remove(id);
        self.rule_event_consumers.remove(id);
        Ok(())
    }

    pub fn subscribe_events(&mut self) -> DockResult<crate::EventSubscriptionId> {
        let active = self.active_workspace().clone();
        self.subscribe_workspace_events(&active)
    }

    pub fn subscribe_workspace_events(
        &mut self,
        workspace: &WorkspaceId,
    ) -> DockResult<crate::EventSubscriptionId> {
        self.services
            .get_mut(workspace)
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace has no loaded event service.",
                    &workspace.0,
                )
            })?
            .subscribe_events()
    }

    pub fn read_events(
        &mut self,
        subscription: crate::EventSubscriptionId,
        limit: usize,
    ) -> DockResult<crate::EventBatch> {
        let active = self.active_workspace().clone();
        self.read_workspace_events(&active, subscription, limit)
    }

    pub fn read_workspace_events(
        &mut self,
        workspace: &WorkspaceId,
        subscription: crate::EventSubscriptionId,
        limit: usize,
    ) -> DockResult<crate::EventBatch> {
        self.services
            .get_mut(workspace)
            .ok_or_else(|| {
                workspace_error(
                    ErrorCode::NotFound,
                    "Workspace has no loaded event service.",
                    &workspace.0,
                )
            })?
            .read_events(subscription, limit)
    }

    pub fn observe_host_event(
        &mut self,
        workspace: &WorkspaceId,
        expected_revision: u64,
        command: crate::CommandId,
        observation: crate::HostObservation,
    ) -> DockResult<Option<crate::FrameworkEvent>> {
        if self.active_workspace() != workspace {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Host observation belongs to an inactive workspace.",
                format!(
                    "observed={}:active={}",
                    workspace.0,
                    self.active_workspace().0
                ),
            ));
        }
        self.active_service_mut()
            .observe_host_event(expected_revision, command, observation)
    }

    fn active_service_mut(&mut self) -> &mut crate::DockingService {
        let active = self.active_workspace().clone();
        self.services
            .get_mut(&active)
            .expect("validated host keeps the active workspace document loaded")
    }

    fn ensure_active_workspace(
        &self,
        expected_workspace: &WorkspaceId,
        action: &str,
    ) -> DockResult<()> {
        let active = self.active_workspace();
        if active == expected_workspace {
            return Ok(());
        }
        Err(DockError::new(
            ErrorCode::Conflict,
            format!("{action} belongs to an inactive workspace."),
            format!("expected={}:active={}", expected_workspace.0, active.0),
        ))
    }

    fn prune_transfer_endpoints(&mut self) {
        let services = &self.services;
        self.transfer_endpoints.retain(|transfer_id, transfer| {
            services
                .get(&transfer.source)
                .is_some_and(|service| service.contains_transfer_history(transfer_id))
                || services
                    .get(&transfer.target)
                    .is_some_and(|service| service.contains_transfer_history(transfer_id))
        });
    }

    fn retain_active_rule_context_revision(&mut self, document: LayoutDocument) {
        let workspace = document.workspace.clone();
        let documents = self.rule_context_documents.entry(workspace).or_default();
        retain_rule_context_document_in_history(documents, document);
    }
}

fn retain_rule_context_document(
    histories: &mut BTreeMap<WorkspaceId, VecDeque<LayoutDocument>>,
    document: LayoutDocument,
) {
    retain_rule_context_document_in_history(
        histories.entry(document.workspace.clone()).or_default(),
        document,
    );
}

fn retain_rule_context_document_in_history(
    history: &mut VecDeque<LayoutDocument>,
    document: LayoutDocument,
) {
    if history
        .back()
        .is_some_and(|retained| retained.revision == document.revision)
    {
        *history.back_mut().expect("back exists") = document;
        return;
    }
    history.push_back(document);
    while history.len() > MAX_RETAINED_RULE_REVISIONS {
        history.pop_front();
    }
}

fn panel_is_focusable(document: &LayoutDocument, panel: &crate::PanelId) -> bool {
    document
        .root
        .as_deref()
        .is_some_and(|root| crate::find_panel_leaf(root, panel).is_some())
        || document.floating_windows.iter().any(|window| {
            window
                .scratchpad
                .as_ref()
                .is_none_or(|scratchpad| scratchpad.visible)
                && crate::find_panel_leaf(&window.root, panel).is_some()
        })
}

fn collect_subtree_identity(
    node: &crate::Node,
    nodes: &mut BTreeSet<crate::NodeId>,
    panels: &mut BTreeSet<crate::PanelId>,
) {
    nodes.insert(node.id.clone());
    match &node.kind {
        crate::NodeKind::Panel { panel } | crate::NodeKind::Placeholder { panel, .. } => {
            panels.insert(panel.clone());
        }
        crate::NodeKind::Tabs { items, .. } => {
            for child in items {
                collect_subtree_identity(child, nodes, panels);
            }
        }
        crate::NodeKind::DocumentWell { groups, .. } => {
            for child in groups {
                collect_subtree_identity(child, nodes, panels);
            }
        }
        crate::NodeKind::Split { children, .. } | crate::NodeKind::Extension { children, .. } => {
            for child in children {
                collect_subtree_identity(child, nodes, panels);
            }
        }
    }
}

fn first_panel_in_node(node: &crate::Node) -> Option<crate::PanelId> {
    match &node.kind {
        crate::NodeKind::Panel { panel } => Some(panel.clone()),
        crate::NodeKind::Tabs {
            active_index,
            items,
            ..
        } => items
            .get(*active_index)
            .and_then(|active| first_panel_in_node(active)),
        crate::NodeKind::DocumentWell {
            active_group,
            groups,
        } => groups
            .get(*active_group)
            .and_then(|active| first_panel_in_node(active)),
        crate::NodeKind::Split { children, .. } | crate::NodeKind::Extension { children, .. } => {
            children.iter().find_map(|child| first_panel_in_node(child))
        }
        crate::NodeKind::Placeholder { .. } => None,
    }
}

pub fn save_workspace_catalog(
    store: &WorkspacePreferencesStore,
    owner: &WorkspaceId,
    catalog: &WorkspaceCatalog,
) -> DockResult<()> {
    catalog.validate()?;
    let value = serde_json::to_value(catalog).map_err(|error| {
        workspace_error(
            ErrorCode::SerializationError,
            "Workspace catalog encoding failed.",
            error.to_string(),
        )
    })?;
    store.save(owner, &value)
}

pub fn load_workspace_catalog(
    store: &WorkspacePreferencesStore,
    owner: &WorkspaceId,
    maximum_bytes: usize,
) -> DockResult<WorkspaceCatalog> {
    let value: Value = store.load(owner)?;
    decode_workspace_catalog(value, maximum_bytes)
}

pub fn recover_workspace_catalog(
    store: &WorkspacePreferencesStore,
    owner: &WorkspaceId,
    maximum_bytes: usize,
) -> DockResult<WorkspaceCatalog> {
    let value: Value = store.recover(owner)?;
    decode_workspace_catalog(value, maximum_bytes)
}

fn decode_workspace_catalog(value: Value, maximum_bytes: usize) -> DockResult<WorkspaceCatalog> {
    let json = serde_json::to_string(&value).map_err(|error| {
        workspace_error(
            ErrorCode::SerializationError,
            "Workspace catalog decoding failed.",
            error.to_string(),
        )
    })?;
    WorkspaceCatalog::from_json(&json, maximum_bytes)
}

fn validate_id(id: &WorkspaceId) -> DockResult<()> {
    if id.0.is_empty()
        || id.0.len() > MAX_WORKSPACE_ID_BYTES
        || !id
            .0
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err(workspace_error(
            ErrorCode::InvalidId,
            "Workspace ID is invalid or unbounded.",
            &id.0,
        ));
    }
    Ok(())
}

fn validate_title(title: &str) -> DockResult<()> {
    if title.trim().is_empty()
        || title.len() > MAX_WORKSPACE_TITLE_BYTES
        || title.chars().any(char::is_control)
    {
        return Err(workspace_error(
            ErrorCode::InvalidLayout,
            "Workspace title is invalid or unbounded.",
            title,
        ));
    }
    Ok(())
}

fn workspace_error(code: ErrorCode, message: &str, context: impl Into<String>) -> DockError {
    DockError::new(code, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_document(workspace: &str, panel: &str) -> LayoutDocument {
        let mut document = LayoutDocument::new(format!("{workspace}-layout"), workspace);
        std::sync::Arc::make_mut(&mut document.panels)
            .push(crate::PanelDescriptor::new(panel, panel));
        document.root = Some(crate::Node::tabs(
            format!("{workspace}-tabs"),
            vec![crate::Node::panel(format!("{panel}-node"), panel)],
            0,
        ));
        document
    }

    #[test]
    fn document_host_retains_bounded_exact_revision_rule_contexts() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 32)
                .unwrap();

        let initial = host
            .rule_contexts_for_revision(&work, 0, crate::HostCapabilities::default())
            .unwrap();
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].revision, 0);
        assert_eq!(initial[0].panel_id, crate::PanelId::from("editor"));

        let committed = host
            .transact(
                &work,
                0,
                "pin-editor-for-context".into(),
                [crate::Operation::SetTabPinned {
                    panel: "editor".into(),
                    pinned: true,
                }],
            )
            .unwrap();
        assert_eq!(
            host.rule_contexts_for_revision(
                &work,
                committed.document.revision,
                crate::HostCapabilities::default(),
            )
            .unwrap()[0]
                .revision,
            1
        );
        host.undo(&work, 1).unwrap();

        for index in 0..MAX_RETAINED_RULE_REVISIONS {
            host.transact(
                &work,
                host.snapshot().revision,
                format!("bounded-context-{index}").into(),
                [crate::Operation::SetTabPinned {
                    panel: "editor".into(),
                    pinned: index % 2 == 0,
                }],
            )
            .unwrap();
        }
        assert!(
            host.rule_contexts_for_revision(&work, 0, crate::HostCapabilities::default())
                .is_err()
        );
        assert!(
            host.rule_contexts_for_revision(
                &work,
                host.snapshot().revision,
                crate::HostCapabilities::default(),
            )
            .is_ok()
        );
    }

    #[test]
    fn workspace_host_polls_rule_events_with_exact_revision_contexts() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 8).unwrap();
        let rules = crate::WindowRuleSet {
            schema_version: 1,
            rules: vec![crate::WindowRule {
                id: "event-pin".into(),
                enabled: true,
                priority: 1,
                overrides: Vec::new(),
                event_kinds: BTreeSet::from([
                    crate::FrameworkEventKind::FocusChanged,
                    crate::FrameworkEventKind::LayoutCommitted,
                ]),
                predicate: crate::WindowRulePredicate {
                    panel_id: Some("editor".into()),
                    ..crate::WindowRulePredicate::default()
                },
                actions: vec![crate::WindowRuleAction::SetPinned { pinned: true }],
            }],
        };
        host.observe_host_event(
            &work,
            0,
            "initial-focus".into(),
            crate::HostObservation::PanelFocused {
                panel: "editor".into(),
                floating_window: None,
            },
        )
        .unwrap();
        host.transact(
            &work,
            0,
            "user-pin".into(),
            [crate::Operation::SetTabPinned {
                panel: "editor".into(),
                pinned: true,
            }],
        )
        .unwrap();

        let triggered = host
            .poll_rule_events(&work, &rules, crate::HostCapabilities::default(), 16)
            .unwrap();
        assert_eq!(triggered.len(), 2);
        assert_eq!(
            triggered[0].event_kind,
            crate::FrameworkEventKind::FocusChanged
        );
        assert_eq!(triggered[0].evaluation.base_revision, 0);
        assert_eq!(
            triggered[1].event_kind,
            crate::FrameworkEventKind::LayoutCommitted
        );
        assert_eq!(triggered[1].evaluation.base_revision, 1);
        assert!(
            host.poll_rule_events(&work, &rules, crate::HostCapabilities::default(), 16,)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn workspace_host_explicitly_commits_triggered_rule_and_suppresses_follow_up() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 8).unwrap();
        let rules = crate::WindowRuleSet {
            schema_version: 1,
            rules: vec![crate::WindowRule {
                id: "event-pin".into(),
                enabled: true,
                priority: 1,
                overrides: Vec::new(),
                event_kinds: BTreeSet::from([
                    crate::FrameworkEventKind::FocusChanged,
                    crate::FrameworkEventKind::LayoutCommitted,
                ]),
                predicate: crate::WindowRulePredicate {
                    panel_id: Some("editor".into()),
                    ..crate::WindowRulePredicate::default()
                },
                actions: vec![crate::WindowRuleAction::SetPinned { pinned: true }],
            }],
        };
        let host_capabilities = crate::HostCapabilities {
            supported: BTreeSet::from(["workspace:tabs".into()]),
        };
        host.observe_host_event(
            &work,
            0,
            "event-pin-focus".into(),
            crate::HostObservation::PanelFocused {
                panel: "editor".into(),
                floating_window: None,
            },
        )
        .unwrap();
        let triggered = host
            .poll_rule_events(&work, &rules, host_capabilities.clone(), 16)
            .unwrap()
            .pop()
            .expect("focus observation should trigger the explicit rule");
        assert_eq!(triggered.evaluation.base_revision, 0);

        let committed = host
            .commit_triggered_rule(
                &work,
                triggered,
                host_capabilities.clone(),
                "rule-origin-event-pin".into(),
            )
            .unwrap();
        assert_eq!(committed.resulting_revision, 1);
        assert!(
            host.snapshot()
                .document
                .pinned_panels
                .contains(&crate::PanelId::from("editor"))
        );
        assert!(
            host.poll_rule_events(&work, &rules, host_capabilities, 16)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn stale_triggered_rule_does_not_suppress_the_intervening_user_commit() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 8).unwrap();
        let rules = crate::WindowRuleSet {
            schema_version: 1,
            rules: vec![crate::WindowRule {
                id: "event-pin".into(),
                enabled: true,
                priority: 1,
                overrides: Vec::new(),
                event_kinds: BTreeSet::from([
                    crate::FrameworkEventKind::FocusChanged,
                    crate::FrameworkEventKind::LayoutCommitted,
                ]),
                predicate: crate::WindowRulePredicate {
                    panel_id: Some("editor".into()),
                    ..crate::WindowRulePredicate::default()
                },
                actions: vec![crate::WindowRuleAction::SetPinned { pinned: true }],
            }],
        };
        let host_capabilities = crate::HostCapabilities {
            supported: BTreeSet::from(["workspace:tabs".into()]),
        };
        host.observe_host_event(
            &work,
            0,
            "stale-rule-focus".into(),
            crate::HostObservation::PanelFocused {
                panel: "editor".into(),
                floating_window: None,
            },
        )
        .unwrap();
        let triggered = host
            .poll_rule_events(&work, &rules, host_capabilities.clone(), 16)
            .unwrap()
            .pop()
            .unwrap();

        host.transact(
            &work,
            0,
            "rule-origin-stale".into(),
            [crate::Operation::SetTabPinned {
                panel: "editor".into(),
                pinned: true,
            }],
        )
        .unwrap();
        assert!(
            host.commit_triggered_rule(
                &work,
                triggered,
                host_capabilities.clone(),
                "rule-origin-stale".into(),
            )
            .is_err()
        );
        let follow_up = host
            .poll_rule_events(&work, &rules, host_capabilities, 16)
            .unwrap();
        assert_eq!(follow_up.len(), 1);
        assert_eq!(
            follow_up[0].event_kind,
            crate::FrameworkEventKind::LayoutCommitted
        );
    }

    #[test]
    fn document_host_switches_sessions_restores_focus_and_keeps_histories_separate() {
        let work = WorkspaceId::from("work");
        let research = WorkspaceId::from("research");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(research.clone(), "Research").unwrap();
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [
                workspace_document("work", "editor"),
                workspace_document("research", "editor"),
            ],
            8,
        )
        .unwrap();

        host.record_focus(&work, 0, "editor".into()).unwrap();
        let changed = host
            .transact(
                &work,
                0,
                "pin-work-editor".into(),
                [crate::Operation::SetTabPinned {
                    panel: "editor".into(),
                    pinned: true,
                }],
            )
            .unwrap();
        assert_eq!(changed.document.revision, 1);
        assert!(host.record_focus(&work, 0, "editor".into()).is_err());
        host.record_focus(&work, 1, "editor".into()).unwrap();

        let research_activation = host.activate(&work, &research).unwrap();
        assert_eq!(research_activation.previous_workspace, work);
        assert_eq!(research_activation.active_workspace, research);
        assert_eq!(research_activation.snapshot.document.revision, 0);
        assert_eq!(research_activation.focus_restore, None);
        host.record_focus(&research, 0, "editor".into()).unwrap();
        assert!(host.activate(&work, &research).is_err());

        let work_activation = host.activate(&research, &work).unwrap();
        assert_eq!(work_activation.snapshot.document.revision, 1);
        assert_eq!(work_activation.snapshot.undo_depth, 1);
        assert_eq!(work_activation.focus_restore, Some("editor".into()));
        let undone = host.undo(&work, 1).unwrap();
        assert!(undone.document.pinned_panels.is_empty());
        assert_eq!(host.active_workspace(), &work);
    }

    #[test]
    fn document_host_rejects_stale_mutations_after_same_revision_workspace_switch() {
        let work = WorkspaceId::from("work");
        let research = WorkspaceId::from("research");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(research.clone(), "Research").unwrap();
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [
                workspace_document("work", "editor"),
                workspace_document("research", "editor"),
            ],
            8,
        )
        .unwrap();

        host.transact(
            &work,
            0,
            "pin-work-editor".into(),
            [crate::Operation::SetTabPinned {
                panel: "editor".into(),
                pinned: true,
            }],
        )
        .unwrap();
        host.activate(&work, &research).unwrap();
        host.transact(
            &research,
            0,
            "pin-research-editor".into(),
            [crate::Operation::SetTabPinned {
                panel: "editor".into(),
                pinned: true,
            }],
        )
        .unwrap();
        let before_stale_calls = host.snapshot();
        assert_eq!(before_stale_calls.document.revision, 1);

        assert!(
            host.transact(
                &work,
                1,
                "stale-work-transaction".into(),
                [crate::Operation::SetTabPinned {
                    panel: "editor".into(),
                    pinned: false,
                }],
            )
            .is_err()
        );
        assert!(host.undo(&work, 1).is_err());
        assert!(
            host.replace_active_document(
                &work,
                1,
                workspace_document("work", "replacement-editor"),
                8,
            )
            .is_err()
        );
        assert_eq!(host.snapshot(), before_stale_calls);

        let undone = host.undo(&research, 1).unwrap();
        assert_eq!(undone.document.revision, 2);
        assert!(undone.document.pinned_panels.is_empty());
        let before_stale_redo = host.snapshot();
        assert!(host.redo(&work, 2).is_err());
        assert_eq!(host.snapshot(), before_stale_redo);

        let redone = host.redo(&research, 2).unwrap();
        assert_eq!(redone.document.revision, 3);
        assert!(redone.document.pinned_panels.contains(&"editor".into()));
    }

    #[test]
    fn document_host_focus_history_restores_previous_placed_panel() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut document = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut document.panels)
            .push(crate::PanelDescriptor::new("terminal", "Terminal"));
        document.root = Some(crate::Node::tabs(
            "work-tabs",
            vec![
                crate::Node::panel("editor-node", "editor"),
                crate::Node::panel("terminal-node", "terminal"),
            ],
            0,
        ));
        let mut host = WorkspaceDocumentHost::new(catalog, [document], 8).unwrap();

        host.record_focus(&work, 0, "editor".into()).unwrap();
        host.record_focus(&work, 0, "terminal".into()).unwrap();
        host.record_focus(&work, 0, "editor".into()).unwrap();
        assert_eq!(
            host.focus_restore_candidate(&work, Some(&crate::PanelId::from("editor"))),
            Some(crate::PanelId::from("terminal"))
        );
        assert_eq!(
            host.focus_restore_candidate(&work, None),
            Some(crate::PanelId::from("editor"))
        );

        let replacement = workspace_document("work", "terminal");
        host.replace_active_document(&work, 0, replacement, 8)
            .unwrap();
        assert_eq!(
            host.focus_restore_candidate(&work, None),
            Some(crate::PanelId::from("terminal")),
            "stale focus history must be skipped when a replacement removes a panel"
        );
    }

    #[test]
    fn pinned_scratchpad_summon_activates_its_owner_without_copying_content() {
        let work = WorkspaceId::from("work");
        let chat = WorkspaceId::from("chat");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(chat.clone(), "Chat").unwrap();
        catalog.activate(&chat).unwrap();

        let mut owner = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut owner.panels)
            .push(crate::PanelDescriptor::new("terminal", "Terminal"));
        owner.floating_windows.push(crate::FloatingWindow {
            id: "terminal-window".into(),
            geometry: crate::Rect {
                x: 40,
                y: 60,
                width: 640,
                height: 420,
            },
            monitor_id: "primary".into(),
            scale: 1.0,
            z_order_hint: 0,
            picture_in_picture: false,
            scratchpad: Some(crate::ScratchpadConfig {
                name: "terminal".into(),
                visible: false,
                summon_position: Some(crate::Point { x: 40, y: 60 }),
                auto_hide_on_deactivate: true,
                pinned_across_workspaces: true,
            }),
            root: crate::Node::panel("terminal-node", "terminal"),
        });
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [owner, workspace_document("chat", "chat-editor")],
            8,
        )
        .unwrap();

        let activation = host
            .summon_pinned_scratchpad(&chat, "terminal", "summon-global-terminal".into())
            .unwrap();
        assert_eq!(activation.previous_workspace, chat);
        assert_eq!(activation.active_workspace, work);
        assert_eq!(activation.focus_restore, Some("terminal".into()));
        let snapshot = host.snapshot();
        assert_eq!(snapshot.document.floating_windows.len(), 1);
        assert_eq!(
            snapshot.document.floating_windows[0].id,
            "terminal-window".into()
        );
        assert_eq!(
            snapshot.document.floating_windows[0].root,
            crate::Node::panel("terminal-node", "terminal")
        );
        let config = snapshot.document.floating_windows[0]
            .scratchpad
            .as_ref()
            .unwrap();
        assert!(config.visible);
        assert!(config.pinned_across_workspaces);
        assert_eq!(host.snapshots()[&chat].document.floating_windows.len(), 0);
    }

    #[test]
    fn pinned_scratchpad_move_preserves_identity_and_pairs_undo_redo() {
        let work = WorkspaceId::from("work");
        let chat = WorkspaceId::from("chat");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(chat.clone(), "Chat").unwrap();
        catalog.activate(&chat).unwrap();

        let mut owner = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut owner.panels)
            .push(crate::PanelDescriptor::new("terminal", "Terminal"));
        let original_window = crate::FloatingWindow {
            id: "terminal-window".into(),
            geometry: crate::Rect {
                x: -120,
                y: 80,
                width: 640,
                height: 420,
            },
            monitor_id: "primary".into(),
            scale: 1.25,
            z_order_hint: 4,
            picture_in_picture: false,
            scratchpad: Some(crate::ScratchpadConfig {
                name: "terminal".into(),
                visible: false,
                summon_position: Some(crate::Point { x: -120, y: 80 }),
                auto_hide_on_deactivate: true,
                pinned_across_workspaces: true,
            }),
            root: crate::Node::panel("terminal-node", "terminal"),
        };
        owner.floating_windows.push(original_window.clone());
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [owner, workspace_document("chat", "chat-editor")],
            8,
        )
        .unwrap();
        host.focus_history.insert(
            work.clone(),
            VecDeque::from([
                crate::PanelId::from("terminal"),
                crate::PanelId::from("editor"),
            ]),
        );
        host.focus_history.insert(
            chat.clone(),
            VecDeque::from([crate::PanelId::from("chat-editor")]),
        );
        let source_focus_before = host.focus_history[&work].clone();
        let target_focus_before = host.focus_history[&chat].clone();

        let transfer = host
            .move_pinned_scratchpad(
                &chat,
                0,
                0,
                "terminal",
                &chat,
                "move-terminal-to-chat".into(),
            )
            .unwrap();
        assert_eq!(transfer.panels, vec![crate::PanelId::from("terminal")]);
        assert_eq!(transfer.window, original_window.id);
        assert_eq!(host.active_workspace(), &chat);
        let moved = host.snapshots();
        assert!(moved[&work].document.floating_windows.is_empty());
        assert!(
            moved[&work]
                .document
                .descriptor(&"terminal".into())
                .is_none()
        );
        assert_eq!(
            moved[&chat].document.floating_windows,
            vec![original_window.clone()]
        );
        assert!(
            moved[&chat]
                .document
                .descriptor(&"terminal".into())
                .is_some()
        );
        assert_eq!(
            host.focus_history[&work],
            VecDeque::from([crate::PanelId::from("editor")])
        );
        assert_eq!(
            host.focus_history[&chat],
            VecDeque::from([
                crate::PanelId::from("terminal"),
                crate::PanelId::from("chat-editor")
            ])
        );

        host.undo(&chat, moved[&chat].revision).unwrap();
        let undone = host.snapshots();
        assert_eq!(
            undone[&work].document.floating_windows,
            vec![original_window.clone()]
        );
        assert!(undone[&chat].document.floating_windows.is_empty());
        assert!(
            undone[&work]
                .document
                .descriptor(&"terminal".into())
                .is_some()
        );
        assert!(
            undone[&chat]
                .document
                .descriptor(&"terminal".into())
                .is_none()
        );
        assert_eq!(host.focus_history[&work], source_focus_before);
        assert_eq!(host.focus_history[&chat], target_focus_before);

        host.redo(&chat, undone[&chat].revision).unwrap();
        let redone = host.snapshots();
        assert!(redone[&work].document.floating_windows.is_empty());
        assert_eq!(
            redone[&chat].document.floating_windows,
            vec![original_window]
        );
        assert!(
            redone[&chat]
                .document
                .descriptor(&"terminal".into())
                .is_some()
        );
        assert!(
            redone[&work]
                .document
                .descriptor(&"terminal".into())
                .is_none()
        );
        assert_eq!(
            host.focus_history[&chat],
            VecDeque::from([
                crate::PanelId::from("terminal"),
                crate::PanelId::from("chat-editor")
            ])
        );
    }

    #[test]
    fn pinned_scratchpad_move_rejects_target_identity_collision_atomically() {
        let work = WorkspaceId::from("work");
        let chat = WorkspaceId::from("chat");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(chat.clone(), "Chat").unwrap();
        catalog.activate(&chat).unwrap();
        let mut owner = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut owner.panels)
            .push(crate::PanelDescriptor::new("terminal", "Terminal"));
        owner.floating_windows.push(crate::FloatingWindow {
            id: "terminal-window".into(),
            geometry: crate::Rect {
                x: 0,
                y: 0,
                width: 400,
                height: 300,
            },
            monitor_id: "primary".into(),
            scale: 1.0,
            z_order_hint: 0,
            picture_in_picture: false,
            scratchpad: Some(crate::ScratchpadConfig {
                name: "terminal".into(),
                visible: true,
                summon_position: None,
                auto_hide_on_deactivate: false,
                pinned_across_workspaces: true,
            }),
            root: crate::Node::panel("terminal-node", "terminal"),
        });
        let mut host =
            WorkspaceDocumentHost::new(catalog, [owner, workspace_document("chat", "terminal")], 8)
                .unwrap();
        let before = host.snapshots();

        assert!(
            host.move_pinned_scratchpad(
                &chat,
                before[&work].revision,
                before[&chat].revision,
                "terminal",
                &chat,
                "collision-terminal-move".into(),
            )
            .is_err()
        );
        assert_eq!(host.active_workspace(), &chat);
        assert_eq!(host.snapshots(), before);
    }

    #[test]
    fn pinned_scratchpad_move_requires_undo_capacity_and_keeps_documents_unchanged() {
        let work = WorkspaceId::from("work");
        let chat = WorkspaceId::from("chat");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(chat.clone(), "Chat").unwrap();
        catalog.activate(&chat).unwrap();
        let mut owner = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut owner.panels)
            .push(crate::PanelDescriptor::new("terminal", "Terminal"));
        owner.floating_windows.push(crate::FloatingWindow {
            id: "terminal-window".into(),
            geometry: crate::Rect {
                x: 0,
                y: 0,
                width: 400,
                height: 300,
            },
            monitor_id: "primary".into(),
            scale: 1.0,
            z_order_hint: 0,
            picture_in_picture: false,
            scratchpad: Some(crate::ScratchpadConfig {
                name: "terminal".into(),
                visible: true,
                summon_position: None,
                auto_hide_on_deactivate: false,
                pinned_across_workspaces: true,
            }),
            root: crate::Node::panel("terminal-node", "terminal"),
        });
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [owner, workspace_document("chat", "chat-editor")],
            0,
        )
        .unwrap();
        let before = host.snapshots();
        let result = host.move_pinned_scratchpad(
            &chat,
            before[&work].revision,
            before[&chat].revision,
            "terminal",
            &chat,
            "zero-history-terminal-move".into(),
        );
        assert!(result.is_err());
        assert_eq!(host.snapshots(), before);
        assert_eq!(host.active_workspace(), &chat);
    }

    #[test]
    fn new_workspace_commit_invalidates_the_other_side_of_transfer_redo() {
        let work = WorkspaceId::from("work");
        let chat = WorkspaceId::from("chat");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(chat.clone(), "Chat").unwrap();
        catalog.activate(&chat).unwrap();
        let mut owner = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut owner.panels)
            .push(crate::PanelDescriptor::new("terminal", "Terminal"));
        owner.floating_windows.push(crate::FloatingWindow {
            id: "terminal-window".into(),
            geometry: crate::Rect {
                x: 0,
                y: 0,
                width: 400,
                height: 300,
            },
            monitor_id: "primary".into(),
            scale: 1.0,
            z_order_hint: 0,
            picture_in_picture: false,
            scratchpad: Some(crate::ScratchpadConfig {
                name: "terminal".into(),
                visible: true,
                summon_position: None,
                auto_hide_on_deactivate: false,
                pinned_across_workspaces: true,
            }),
            root: crate::Node::panel("terminal-node", "terminal"),
        });
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [owner, workspace_document("chat", "chat-editor")],
            8,
        )
        .unwrap();
        host.move_pinned_scratchpad(
            &chat,
            0,
            0,
            "terminal",
            &chat,
            "invalidate-redo-terminal-move".into(),
        )
        .unwrap();
        host.undo(&chat, host.snapshot().revision).unwrap();

        host.activate(&chat, &work).unwrap();
        host.transact(
            &work,
            host.snapshot().revision,
            "work-after-transfer-undo".into(),
            [crate::Operation::SetTabPinned {
                panel: "editor".into(),
                pinned: true,
            }],
        )
        .unwrap();
        host.activate(&work, &chat).unwrap();
        let before_redo = host.snapshots();
        assert!(host.redo(&chat, before_redo[&chat].revision).is_err());
        let after_redo = host.snapshots();
        assert_eq!(
            after_redo[&work].document.floating_windows.len(),
            before_redo[&work].document.floating_windows.len()
        );
        assert_eq!(
            after_redo[&chat].document.floating_windows.len(),
            before_redo[&chat].document.floating_windows.len()
        );
        assert!(
            after_redo[&work]
                .document
                .pinned_panels
                .contains(&"editor".into())
        );
    }

    #[test]
    fn pinned_scratchpad_summon_rejects_ambiguous_names_without_switching() {
        let work = WorkspaceId::from("work");
        let chat = WorkspaceId::from("chat");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(chat.clone(), "Chat").unwrap();
        catalog.activate(&chat).unwrap();
        let mut first = workspace_document("work", "work-panel");
        let mut second = workspace_document("chat", "chat-panel");
        for document in [&mut first, &mut second] {
            let panel = format!("{}-scratch", document.workspace.0);
            std::sync::Arc::make_mut(&mut document.panels)
                .push(crate::PanelDescriptor::new(panel.clone(), "Scratch"));
            document.floating_windows.push(crate::FloatingWindow {
                id: format!("{}-window", document.workspace.0).into(),
                geometry: crate::Rect {
                    x: 0,
                    y: 0,
                    width: 300,
                    height: 200,
                },
                monitor_id: "primary".into(),
                scale: 1.0,
                z_order_hint: 0,
                picture_in_picture: false,
                scratchpad: Some(crate::ScratchpadConfig {
                    name: "duplicate".into(),
                    visible: false,
                    summon_position: None,
                    auto_hide_on_deactivate: false,
                    pinned_across_workspaces: true,
                }),
                root: crate::Node::panel(format!("{panel}-node"), panel),
            });
        }
        let mut host = WorkspaceDocumentHost::new(catalog, [first, second], 8).unwrap();
        assert!(
            host.summon_pinned_scratchpad(&chat, "duplicate", "ambiguous-summon".into())
                .is_err()
        );
        assert_eq!(host.active_workspace(), &chat);
        assert_eq!(host.snapshot().revision, 0);
    }

    #[test]
    fn document_host_focus_restore_skips_hidden_scratchpad_panels() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut document = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut document.panels)
            .push(crate::PanelDescriptor::new("scratch", "Scratch"));
        document.floating_windows.push(crate::FloatingWindow {
            id: "scratch-window".into(),
            geometry: crate::Rect {
                x: 20,
                y: 20,
                width: 400,
                height: 300,
            },
            monitor_id: "primary".into(),
            scale: 1.0,
            z_order_hint: 0,
            picture_in_picture: false,
            scratchpad: Some(crate::ScratchpadConfig {
                name: "scratch".into(),
                visible: true,
                summon_position: None,
                auto_hide_on_deactivate: false,
                pinned_across_workspaces: false,
            }),
            root: crate::Node::panel("scratch-node", "scratch"),
        });
        let mut host = WorkspaceDocumentHost::new(catalog, [document], 8).unwrap();
        host.record_focus(&work, 0, "editor".into()).unwrap();
        host.record_focus(&work, 0, "scratch".into()).unwrap();

        host.transact(
            &work,
            0,
            "hide-scratchpad-for-focus-restore".into(),
            [crate::Operation::SetScratchpad {
                window: "scratch-window".into(),
                config: Some(crate::ScratchpadConfig {
                    name: "scratch".into(),
                    visible: false,
                    summon_position: None,
                    auto_hide_on_deactivate: false,
                    pinned_across_workspaces: false,
                }),
            }],
        )
        .unwrap();
        assert_eq!(
            host.focus_restore_candidate(&work, None),
            Some(crate::PanelId::from("editor"))
        );
        assert!(host.record_focus(&work, 1, "scratch".into()).is_err());
    }

    #[test]
    fn document_host_requires_registered_documents_and_rejects_inactive_observations() {
        let work = WorkspaceId::from("work");
        let research = WorkspaceId::from("research");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(research.clone(), "Research").unwrap();
        assert!(
            WorkspaceDocumentHost::new(catalog.clone(), [workspace_document("work", "editor")], 4,)
                .unwrap()
                .activate(&work, &research)
                .is_err()
        );

        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [
                workspace_document("work", "editor"),
                workspace_document("research", "editor"),
            ],
            4,
        )
        .unwrap();
        assert!(
            host.observe_host_event(
                &research,
                0,
                "inactive-focus".into(),
                crate::HostObservation::PanelFocused {
                    panel: "editor".into(),
                    floating_window: None,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn document_host_replacement_is_revision_bound_workspace_local_and_resets_history() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 4).unwrap();
        host.transact(
            &work,
            0,
            "pin-editor".into(),
            [crate::Operation::SetTabPinned {
                panel: "editor".into(),
                pinned: true,
            }],
        )
        .unwrap();
        assert!(
            host.replace_active_document(&work, 0, workspace_document("work", "new-editor"), 4)
                .is_err()
        );
        assert!(
            host.replace_active_document(&work, 1, workspace_document("other", "new-editor"), 4)
                .is_err()
        );
        let snapshot = host
            .replace_active_document(&work, 1, workspace_document("work", "new-editor"), 4)
            .unwrap();
        assert_eq!(snapshot.revision, 2);
        assert_eq!(snapshot.undo_depth, 0);
        assert!(snapshot.document.descriptor(&"new-editor".into()).is_some());
    }

    #[test]
    fn document_host_apply_layout_is_active_workspace_bound_and_undoable() {
        let work = WorkspaceId::from("work");
        let other = WorkspaceId::from("other");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(other, "Other").unwrap();
        let mut work_document = workspace_document("work", "editor");
        std::sync::Arc::make_mut(&mut work_document.panels)
            .push(crate::PanelDescriptor::new("notes", "notes"));
        work_document.root = Some(crate::Node::tabs(
            "work-tabs",
            vec![
                crate::Node::panel("editor-node", "editor"),
                crate::Node::panel("notes-node", "notes"),
            ],
            0,
        ));
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [work_document.clone(), workspace_document("other", "notes")],
            4,
        )
        .unwrap();
        let mut target = work_document;
        target.root = Some(crate::Node::tabs(
            "work-tabs",
            vec![
                crate::Node::panel("editor-node", "editor"),
                crate::Node::panel("notes-node", "notes"),
            ],
            1,
        ));
        host.apply_layout(0, &target).unwrap();
        assert_eq!(host.snapshot().document.root, target.root);
        assert_eq!(host.snapshot().undo_depth, 1);
        host.undo(&work, 1).unwrap();
        assert_eq!(
            host.snapshot().document.root,
            Some(crate::Node::tabs(
                "work-tabs",
                vec![
                    crate::Node::panel("editor-node", "editor"),
                    crate::Node::panel("notes-node", "notes"),
                ],
                0,
            ))
        );

        let other_target = workspace_document("other", "notes");
        assert!(host.apply_layout(2, &other_target).is_err());
        assert_eq!(
            host.snapshot().document.workspace,
            WorkspaceId::from("work")
        );
    }

    #[test]
    fn document_host_creates_an_empty_workspace_and_rejects_stale_creation() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 4).unwrap();
        assert!(
            host.create_workspace(
                &WorkspaceId::from("stale"),
                WorkspaceId::from("new-room"),
                "New room"
            )
            .is_err()
        );
        let activation = host
            .create_workspace(&work, WorkspaceId::from("new-room"), "New room")
            .unwrap();
        assert_eq!(activation.active_workspace, WorkspaceId::from("new-room"));
        assert_eq!(
            activation.snapshot.document.workspace,
            activation.active_workspace
        );
        assert!(activation.snapshot.document.panels.is_empty());
        assert!(activation.snapshot.document.root.is_none());
        assert_eq!(activation.snapshot.undo_depth, 0);
        host.rename_workspace(&activation.active_workspace, "Research room")
            .unwrap();
        assert_eq!(
            host.catalog().workspaces[&activation.active_workspace].title,
            "Research room"
        );
    }

    #[test]
    fn document_host_archive_switches_to_loaded_workspace_and_can_restore() {
        let work = WorkspaceId::from("work");
        let research = WorkspaceId::from("research");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        catalog.create(research.clone(), "Research").unwrap();
        let mut host = WorkspaceDocumentHost::new(
            catalog,
            [
                workspace_document("work", "editor"),
                workspace_document("research", "notes"),
            ],
            4,
        )
        .unwrap();
        let activation = host.archive_active_workspace(&work).unwrap();
        assert_eq!(activation.active_workspace, research);
        assert!(host.catalog().workspaces[&work].archived);
        assert!(host.archive_active_workspace(&work).is_err());
        assert!(host.restore_workspace(&work).unwrap());
        assert!(!host.catalog().workspaces[&work].archived);
        assert!(!host.restore_workspace(&work).unwrap());
    }

    #[test]
    fn document_host_deletes_only_an_inactive_empty_workspace() {
        let work = WorkspaceId::from("work");
        let mut catalog = WorkspaceCatalog::default();
        catalog.create(work.clone(), "Work").unwrap();
        let mut host =
            WorkspaceDocumentHost::new(catalog, [workspace_document("work", "editor")], 4).unwrap();
        let created = host
            .create_workspace(&work, WorkspaceId::from("empty"), "Empty")
            .unwrap();
        assert!(host.delete_workspace(&created.active_workspace).is_err());
        let activation = host
            .archive_active_workspace(&created.active_workspace)
            .unwrap();
        assert_eq!(activation.active_workspace, work);
        host.delete_workspace(&WorkspaceId::from("empty")).unwrap();
        assert!(
            !host
                .catalog()
                .workspaces
                .contains_key(&WorkspaceId::from("empty"))
        );
        assert!(host.snapshots().contains_key(&WorkspaceId::from("work")));
    }

    #[test]
    fn workspace_catalog_lifecycle_is_atomic_ordered_and_template_aware() {
        let mut catalog = WorkspaceCatalog::default();
        let work = WorkspaceId::from("work");
        let research = WorkspaceId::from("research");
        catalog.create(work.clone(), "Work").unwrap();
        catalog
            .duplicate_layout_template(&work, research.clone(), "Research")
            .unwrap();
        assert_eq!(catalog.active, Some(work.clone()));
        assert_eq!(
            catalog.workspaces[&research].template_source,
            Some(work.clone())
        );
        assert!(catalog.activate(&research).unwrap());
        assert!(catalog.reorder(&research, 0).unwrap());
        catalog.rename(&research, "Research and notes").unwrap();
        catalog.archive(&research).unwrap();
        assert_eq!(catalog.active, Some(work.clone()));
        assert!(catalog.workspaces[&research].archived);
        assert!(catalog.restore(&research).unwrap());

        let before = catalog.clone();
        assert!(catalog.reorder(&research, 99).is_err());
        assert_eq!(catalog, before, "invalid catalog changes are atomic");
        assert!(catalog.to_json().is_ok());
    }

    #[test]
    fn workspace_delete_requires_matching_empty_document_and_persistence_roundtrips() {
        let mut catalog = WorkspaceCatalog::default();
        let work = WorkspaceId::from("work");
        let research = WorkspaceId::from("research");
        catalog.create(work.clone(), "Work").unwrap();
        catalog
            .duplicate_layout_template(&work, research.clone(), "Research")
            .unwrap();
        let occupied = LayoutDocument::new("research-layout", research.clone());
        let mut occupied = occupied;
        occupied.panels = vec![crate::PanelDescriptor::new("editor", "Editor")].into();
        let before = catalog.clone();
        assert!(catalog.delete(&research, &occupied).is_err());
        assert_eq!(catalog, before);

        let empty = LayoutDocument::new("research-layout", research.clone());
        catalog.delete(&research, &empty).unwrap();
        assert_eq!(catalog.order, vec![work.clone()]);
        assert_eq!(catalog.workspaces[&work].template_source, None);

        let directory = std::env::temp_dir().join(format!(
            "doofusdock-workspace-catalog-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let store = WorkspacePreferencesStore::new(directory.clone(), Default::default());
        save_workspace_catalog(&store, &WorkspaceId::from("application"), &catalog).unwrap();
        assert_eq!(
            load_workspace_catalog(&store, &WorkspaceId::from("application"), 1024 * 1024).unwrap(),
            catalog
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
