//! Stable application-facing control facade over the canonical layout model.
//!
//! UI commands and programmatic callers use the same optimistic, validated
//! transaction path. The service owns no renderer or operating-system state.
use crate::{
    CommandId, DiffPatch, DockError, DockResult, ErrorCode, LayoutDiff, LayoutDocument, Operation,
    TransactionResult, UndoStack, WorkspaceProfile, validate_layout,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ControlSnapshot {
    pub revision: u64,
    pub document: LayoutDocument,
    pub undo_depth: usize,
    pub redo_depth: usize,
}

/// Deterministic single-writer controller for a DoofusDock workspace.
pub struct DockingService {
    document: LayoutDocument,
    history: UndoStack,
    events: crate::EventHub,
}

impl DockingService {
    pub(crate) fn staged_clone(&self) -> Self {
        Self {
            document: self.document.clone(),
            history: self.history.clone(),
            events: self.events.clone(),
        }
    }

    /// Execute a transaction over application-owned state through the same
    /// revision and validation contract used by the owning service instance.
    pub fn transact_document(
        document: &mut LayoutDocument,
        history: &mut UndoStack,
        expected_revision: u64,
        command: CommandId,
        operations: impl IntoIterator<Item = Operation>,
    ) -> DockResult<TransactionResult> {
        Self::check_document_revision(document, expected_revision, "transact")?;
        history.execute_batch(document, command, operations)
    }

    pub fn apply_profile_document(
        document: &mut LayoutDocument,
        history: &mut UndoStack,
        expected_revision: u64,
        profile: &WorkspaceProfile,
        assignments: &BTreeMap<String, Vec<crate::PanelId>>,
    ) -> DockResult<crate::ProfileApplication> {
        Self::check_document_revision(document, expected_revision, "apply-profile")?;
        history.apply_profile(document, profile, assignments)
    }

    pub fn apply_layout_document(
        document: &mut LayoutDocument,
        history: &mut UndoStack,
        expected_revision: u64,
        target: &LayoutDocument,
    ) -> DockResult<TransactionResult> {
        Self::check_document_revision(document, expected_revision, "apply-layout")?;
        history.apply_layout(document, target)
    }

    pub fn undo_document(
        document: &mut LayoutDocument,
        history: &mut UndoStack,
        expected_revision: u64,
    ) -> DockResult<TransactionResult> {
        Self::check_document_revision(document, expected_revision, "undo")?;
        history.undo(document)
    }

    pub fn redo_document(
        document: &mut LayoutDocument,
        history: &mut UndoStack,
        expected_revision: u64,
    ) -> DockResult<TransactionResult> {
        Self::check_document_revision(document, expected_revision, "redo")?;
        history.redo(document)
    }

    pub fn new(document: LayoutDocument, history_capacity: usize) -> DockResult<Self> {
        validate_layout(&document)?;
        Ok(Self {
            document,
            history: UndoStack::new(history_capacity),
            events: crate::EventHub::new(256).expect("built-in event capacity is valid"),
        })
    }

    pub fn snapshot(&self) -> ControlSnapshot {
        ControlSnapshot {
            revision: self.document.revision,
            document: self.document.clone(),
            undo_depth: self.history.undo_size(),
            redo_depth: self.history.redo_size(),
        }
    }

    /// Execute an atomic command if the caller's observed revision is current.
    pub fn transact(
        &mut self,
        expected_revision: u64,
        command: CommandId,
        operations: impl IntoIterator<Item = Operation>,
    ) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "transact")?;
        let result = self
            .history
            .execute_batch(&mut self.document, command, operations)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutCommitted)?;
        Ok(result)
    }

    pub fn apply_profile(
        &mut self,
        expected_revision: u64,
        profile: &WorkspaceProfile,
        assignments: &BTreeMap<String, Vec<crate::PanelId>>,
    ) -> DockResult<crate::ProfileApplication> {
        self.check_revision(expected_revision, "apply-profile")?;
        let application = self
            .history
            .apply_profile(&mut self.document, profile, assignments)?;
        if !application.diff.is_empty() {
            let command = application.operation_id.clone();
            self.events.publish(
                application.document.revision,
                crate::FrameworkEventKind::WorkspaceActivated,
                command.clone(),
                BTreeMap::new(),
            )?;
            self.publish_diff_events(application.document.revision, command, &application.diff)?;
        }
        Ok(application)
    }

    pub fn apply_profile_switch(
        &mut self,
        expected_revision: u64,
        plan: crate::ProfileSwitchPlan,
    ) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "apply-profile-switch")?;
        if plan.base_revision != expected_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Profile switch plan was prepared for a different revision.",
                format!("plan={}:expected={expected_revision}", plan.base_revision),
            ));
        }
        let result = self
            .history
            .apply_layout(&mut self.document, &plan.application.document)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutCommitted)?;
        Ok(result)
    }

    /// Apply a complete target document as one undoable canonical operation.
    pub fn apply_layout(
        &mut self,
        expected_revision: u64,
        target: &LayoutDocument,
    ) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "apply-layout")?;
        let result = self.history.apply_layout(&mut self.document, target)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutCommitted)?;
        Ok(result)
    }

    pub fn undo(&mut self, expected_revision: u64) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "undo")?;
        let result = self.history.undo(&mut self.document)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutRestored)?;
        Ok(result)
    }

    pub fn redo(&mut self, expected_revision: u64) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "redo")?;
        let result = self.history.redo(&mut self.document)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutRestored)?;
        Ok(result)
    }

    pub(crate) fn apply_transfer_layout(
        &mut self,
        expected_revision: u64,
        target: &LayoutDocument,
        transfer_id: &str,
    ) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "workspace-transfer")?;
        let result = self
            .history
            .apply_transfer_layout(&mut self.document, target, transfer_id)?;
        if !result.diff.is_empty() {
            self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutCommitted)?;
        }
        Ok(result)
    }

    pub(crate) fn undo_transfer(
        &mut self,
        expected_revision: u64,
        transfer_id: &str,
    ) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "undo-workspace-transfer")?;
        let result = self
            .history
            .undo_transfer(&mut self.document, transfer_id)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutRestored)?;
        Ok(result)
    }

    pub(crate) fn redo_transfer(
        &mut self,
        expected_revision: u64,
        transfer_id: &str,
    ) -> DockResult<TransactionResult> {
        self.check_revision(expected_revision, "redo-workspace-transfer")?;
        let result = self
            .history
            .redo_transfer(&mut self.document, transfer_id)?;
        self.publish_committed_event(&result, crate::FrameworkEventKind::LayoutRestored)?;
        Ok(result)
    }

    pub(crate) fn top_undo_transfer_id(&self) -> Option<&str> {
        self.history.top_undo_transfer_id()
    }

    pub(crate) fn top_redo_transfer_id(&self) -> Option<&str> {
        self.history.top_redo_transfer_id()
    }

    pub(crate) fn contains_transfer_history(&self, transfer_id: &str) -> bool {
        self.history.contains_transfer_id(transfer_id)
    }

    pub(crate) fn discard_transfer_redo(&mut self, transfer_id: &str) {
        self.history.discard_transfer_redo(transfer_id);
    }

    pub fn subscribe_events(&mut self) -> DockResult<crate::EventSubscriptionId> {
        self.events.subscribe()
    }

    pub fn read_events(
        &mut self,
        subscription: crate::EventSubscriptionId,
        limit: usize,
    ) -> DockResult<crate::EventBatch> {
        self.events.read(subscription, limit)
    }

    pub fn unsubscribe_events(&mut self, subscription: crate::EventSubscriptionId) -> bool {
        self.events.unsubscribe(subscription)
    }

    /// Publish an immutable host observation only when both its canonical
    /// revision and reported panel placement still match the service snapshot.
    pub fn observe_host_event(
        &mut self,
        expected_revision: u64,
        command: CommandId,
        observation: crate::HostObservation,
    ) -> DockResult<Option<crate::FrameworkEvent>> {
        self.check_revision(expected_revision, "observe-host-event")?;
        match observation {
            crate::HostObservation::PanelFocused {
                panel,
                floating_window,
            } => {
                if self.document.descriptor(&panel).is_none() {
                    return Err(DockError::new(
                        ErrorCode::NotFound,
                        "Host focus observation references an unknown panel.",
                        panel.0,
                    ));
                }
                let observed_host = if self
                    .document
                    .root
                    .as_deref()
                    .is_some_and(|root| crate::find_panel_leaf(root, &panel).is_some())
                {
                    None
                } else {
                    self.document
                        .floating_windows
                        .iter()
                        .find(|window| crate::find_panel_leaf(&window.root, &panel).is_some())
                        .map(|window| window.id.clone())
                };
                if observed_host != floating_window {
                    return Err(DockError::new(
                        ErrorCode::Conflict,
                        "Host focus observation does not match the panel's current placement.",
                        format!(
                            "panel={}:expected_host={:?}:reported_host={:?}",
                            panel.0, observed_host, floating_window
                        ),
                    ));
                }
                let mut payload = BTreeMap::new();
                payload.insert("panel".into(), panel.0);
                if let Some(window) = floating_window {
                    payload.insert("window".into(), window.0);
                }
                self.events.publish(
                    self.document.revision,
                    crate::FrameworkEventKind::FocusChanged,
                    command,
                    payload,
                )
            }
            crate::HostObservation::NativeViewportFocused { window } => {
                if let Some(window) = &window {
                    self.require_native_window(window)?;
                }
                let mut payload = BTreeMap::new();
                match window {
                    Some(window) => {
                        payload.insert("window".into(), window.0);
                    }
                    None => {
                        payload.insert("viewport".into(), "root".into());
                    }
                }
                self.events.publish(
                    self.document.revision,
                    crate::FrameworkEventKind::NativeViewportFocused,
                    command,
                    payload,
                )
            }
            crate::HostObservation::NativeViewportCloseRequested { window } => {
                self.require_native_window(&window)?;
                self.events.publish(
                    self.document.revision,
                    crate::FrameworkEventKind::NativeViewportCloseRequested,
                    command,
                    BTreeMap::from([("window".into(), window.0)]),
                )
            }
            crate::HostObservation::NativeViewportGeometryObserved { window, geometry } => {
                self.require_native_window(&window)?;
                let mut payload = BTreeMap::new();
                payload.insert("window".into(), window.0);
                payload.insert(
                    "geometry".into(),
                    format!(
                        "{},{},{},{}",
                        geometry.x, geometry.y, geometry.width, geometry.height
                    ),
                );
                self.events.publish(
                    self.document.revision,
                    crate::FrameworkEventKind::NativeViewportGeometryObserved,
                    command,
                    payload,
                )
            }
            crate::HostObservation::NativeRootViewportGeometryObserved { geometry } => {
                if geometry.width <= 0
                    || geometry.height <= 0
                    || geometry.width > 1_000_000
                    || geometry.height > 1_000_000
                {
                    return Err(DockError::new(
                        ErrorCode::SchemaError,
                        "Root viewport geometry must have positive, bounded dimensions.",
                        format!("{}x{}", geometry.width, geometry.height),
                    ));
                }
                self.events.publish(
                    self.document.revision,
                    crate::FrameworkEventKind::NativeRootViewportGeometryObserved,
                    command,
                    BTreeMap::from([(
                        "geometry".into(),
                        format!(
                            "{},{},{},{}",
                            geometry.x, geometry.y, geometry.width, geometry.height
                        ),
                    )]),
                )
            }
            crate::HostObservation::MonitorChanged {
                monitor,
                work_area,
                scale_milli,
                primary,
                connected,
            } => {
                if monitor.trim().is_empty()
                    || monitor.len() > 256
                    || scale_milli == 0
                    || scale_milli > 10_000
                    || work_area.width <= 0
                    || work_area.height <= 0
                {
                    return Err(DockError::new(
                        ErrorCode::SchemaError,
                        "Monitor observation requires a stable identity, positive work area, and bounded positive scale.",
                        monitor,
                    ));
                }
                let mut payload = BTreeMap::new();
                payload.insert("monitor".into(), monitor);
                payload.insert(
                    "work_area".into(),
                    format!(
                        "{},{},{},{}",
                        work_area.x, work_area.y, work_area.width, work_area.height
                    ),
                );
                payload.insert(
                    "scale".into(),
                    format!("{:.3}", scale_milli as f64 / 1000.0),
                );
                payload.insert("primary".into(), primary.to_string());
                payload.insert("connected".into(), connected.to_string());
                self.events.publish(
                    self.document.revision,
                    crate::FrameworkEventKind::MonitorChanged,
                    command,
                    payload,
                )
            }
            crate::HostObservation::TransitionStarted { transition } => self
                .publish_transition_observation(
                    transition,
                    command,
                    crate::FrameworkEventKind::TransitionStarted,
                ),
            crate::HostObservation::TransitionCompleted { transition } => self
                .publish_transition_observation(
                    transition,
                    command,
                    crate::FrameworkEventKind::TransitionCompleted,
                ),
        }
    }

    fn publish_transition_observation(
        &mut self,
        transition: crate::TransitionId,
        command: CommandId,
        kind: crate::FrameworkEventKind,
    ) -> DockResult<Option<crate::FrameworkEvent>> {
        if transition.is_empty() || transition.0.len() > 128 {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Transition observation requires a bounded stable identity.",
                transition.0,
            ));
        }
        self.events.publish(
            self.document.revision,
            kind,
            command,
            BTreeMap::from([("transition".into(), transition.0)]),
        )
    }

    fn require_native_window(&self, window: &crate::WindowId) -> DockResult<()> {
        if self
            .document
            .floating_windows
            .iter()
            .any(|entry| &entry.id == window)
        {
            return Ok(());
        }
        Err(DockError::new(
            ErrorCode::NotFound,
            "Native viewport observation references an unknown floating window.",
            &window.0,
        ))
    }

    fn publish_committed_event(
        &mut self,
        result: &TransactionResult,
        kind: crate::FrameworkEventKind,
    ) -> DockResult<()> {
        if !result.diff.is_empty() {
            let command = CommandId::from(result.operation_id.clone());
            self.events.publish(
                result.document.revision,
                kind,
                command.clone(),
                BTreeMap::new(),
            )?;
            self.publish_diff_events(result.document.revision, command, &result.diff)?;
        }
        Ok(())
    }

    fn publish_diff_events(
        &mut self,
        revision: u64,
        command: CommandId,
        diff: &LayoutDiff,
    ) -> DockResult<()> {
        for patch in &diff.patches {
            if let Some((kind, payload)) = event_for_patch(patch) {
                self.events
                    .publish(revision, kind, command.clone(), payload.clone())?;
                if matches!(
                    patch,
                    DiffPatch::PanelVisibilityChanged { visible: false, .. }
                ) {
                    self.events.publish(
                        revision,
                        crate::FrameworkEventKind::PanelClosed,
                        command.clone(),
                        payload,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn check_revision(&self, expected: u64, context: &str) -> DockResult<()> {
        Self::check_document_revision(&self.document, expected, context)
    }

    fn check_document_revision(
        document: &LayoutDocument,
        expected: u64,
        context: &str,
    ) -> DockResult<()> {
        if expected == document.revision {
            return Ok(());
        }
        Err(DockError::new(
            ErrorCode::Conflict,
            "The workspace changed since this command was prepared.",
            format!("{context}:expected={expected}:actual={}", document.revision),
        ))
    }
}

fn event_for_patch(
    patch: &DiffPatch,
) -> Option<(crate::FrameworkEventKind, BTreeMap<String, String>)> {
    use crate::FrameworkEventKind as Kind;
    let mut payload = BTreeMap::new();
    let kind = match patch {
        DiffPatch::PanelReparented {
            panel,
            from_parent,
            to_parent,
            from_window,
            to_window,
            ..
        } => {
            payload.insert("panel".into(), panel.0.clone());
            payload.insert("from_parent".into(), from_parent.0.clone());
            payload.insert("to_parent".into(), to_parent.0.clone());
            if let Some(window) = from_window {
                payload.insert("from_window".into(), window.0.clone());
            }
            if let Some(window) = to_window {
                payload.insert("to_window".into(), window.0.clone());
            }
            if from_parent.0.is_empty() {
                Kind::PanelCreated
            } else {
                match (from_window, to_window) {
                    (None, Some(_)) => Kind::WindowFloated,
                    (Some(_), None) => Kind::WindowDocked,
                    _ => Kind::PanelMoved,
                }
            }
        }
        DiffPatch::PanelVisibilityChanged { panel, visible } => {
            payload.insert("panel".into(), panel.0.clone());
            payload.insert("visible".into(), visible.to_string());
            Kind::PanelVisibilityChanged
        }
        DiffPatch::PanelWorkspaceMembershipChanged { panel, tags } => {
            payload.insert("panel".into(), panel.0.clone());
            payload.insert("tag_count".into(), tags.len().to_string());
            payload.insert(
                "tags".into(),
                tags.iter()
                    .take(6)
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            );
            Kind::PanelWorkspaceMembershipChanged
        }
        DiffPatch::GroupMinimizedChanged { group, minimized } => {
            payload.insert("group".into(), group.0.clone());
            payload.insert("minimized".into(), minimized.to_string());
            Kind::GroupMinimizedChanged
        }
        DiffPatch::DrawerChanged {
            panel,
            auto_hidden,
            edge,
        } => {
            payload.insert("panel".into(), panel.0.clone());
            payload.insert("auto_hidden".into(), auto_hidden.to_string());
            payload.insert("edge".into(), format!("{edge:?}").to_ascii_lowercase());
            Kind::PanelDrawerChanged
        }
        DiffPatch::WindowCreated { window } => {
            payload.insert("window".into(), window.0.clone());
            Kind::WindowFloated
        }
        DiffPatch::WindowDestroyed { window } => {
            payload.insert("window".into(), window.0.clone());
            Kind::WindowDocked
        }
        DiffPatch::WindowGeometryUpdated { window, geometry } => {
            payload.insert("window".into(), window.0.clone());
            payload.insert(
                "geometry".into(),
                format!(
                    "{},{},{},{}",
                    geometry.x, geometry.y, geometry.width, geometry.height
                ),
            );
            Kind::WindowGeometryChanged
        }
        DiffPatch::WindowZOrderChanged { window, z_order } => {
            payload.insert("window".into(), window.0.clone());
            payload.insert("z_order".into(), z_order.to_string());
            Kind::WindowZOrderChanged
        }
        DiffPatch::WindowMonitorChanged {
            window,
            monitor,
            scale,
        } => {
            payload.insert("window".into(), window.0.clone());
            payload.insert("monitor".into(), monitor.clone());
            payload.insert("scale".into(), scale.to_string());
            Kind::WindowMonitorChanged
        }
        DiffPatch::WindowPictureInPictureChanged { window, enabled } => {
            payload.insert("window".into(), window.0.clone());
            payload.insert("picture_in_picture".into(), enabled.to_string());
            Kind::PictureInPictureChanged
        }
        DiffPatch::ScratchpadChanged { window, config } => {
            payload.insert("window".into(), window.0.clone());
            payload.insert(
                "visible".into(),
                config
                    .as_ref()
                    .is_some_and(|config| config.visible)
                    .to_string(),
            );
            if let Some(config) = config {
                payload.insert("name".into(), config.name.clone());
                payload.insert(
                    "auto_hide".into(),
                    config.auto_hide_on_deactivate.to_string(),
                );
            }
            Kind::ScratchpadChanged
        }
        DiffPatch::ActiveTabChanged { tabs, active_index } => {
            payload.insert("container".into(), tabs.0.clone());
            payload.insert("active_index".into(), active_index.to_string());
            Kind::FocusChanged
        }
        DiffPatch::MaximizedPanelChanged { panel } => {
            if let Some(panel) = panel {
                payload.insert("panel".into(), panel.0.clone());
            }
            Kind::FocusChanged
        }
        DiffPatch::ExtensionPayloadUpdated {
            container,
            modality,
            ..
        } => {
            payload.insert("container".into(), container.0.clone());
            payload.insert("modality".into(), modality.0.clone());
            Kind::ModalityChanged
        }
        DiffPatch::ContainerKindChanged { id, kind } => {
            payload.insert("container".into(), id.0.clone());
            payload.insert("kind".into(), format!("{kind:?}").to_ascii_lowercase());
            Kind::ModalityChanged
        }
        DiffPatch::NodeCreated { id, .. } => {
            payload.insert("node".into(), id.0.clone());
            payload.insert("change".into(), "created".into());
            Kind::GroupChanged
        }
        DiffPatch::NodeRemoved { id } => {
            payload.insert("node".into(), id.0.clone());
            payload.insert("change".into(), "removed".into());
            Kind::GroupChanged
        }
        DiffPatch::RootChanged { old_root, new_root } => {
            if let Some(id) = old_root {
                payload.insert("old_root".into(), id.0.clone());
            }
            if let Some(id) = new_root {
                payload.insert("new_root".into(), id.0.clone());
            }
            Kind::GroupChanged
        }
        DiffPatch::ContainerOrderChanged {
            parent,
            ordered_children,
        } => {
            payload.insert("parent".into(), parent.0.clone());
            payload.insert("child_count".into(), ordered_children.len().to_string());
            payload.insert(
                "children".into(),
                ordered_children
                    .iter()
                    .take(3)
                    .map(|child| child.0.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );
            Kind::GroupChanged
        }
        DiffPatch::SplitOrientationChanged { split, orientation } => {
            payload.insert("split".into(), split.0.clone());
            payload.insert(
                "orientation".into(),
                format!("{orientation:?}").to_ascii_lowercase(),
            );
            Kind::GroupChanged
        }
        DiffPatch::SplitRatioUpdated { split, weights } => {
            payload.insert("split".into(), split.0.clone());
            payload.insert("weight_count".into(), weights.len().to_string());
            payload.insert(
                "weights".into(),
                weights
                    .iter()
                    .take(20)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
            Kind::GroupChanged
        }
        DiffPatch::TabPinChanged { panel, pinned } => {
            payload.insert("panel".into(), panel.0.clone());
            payload.insert("pinned".into(), pinned.to_string());
            Kind::GroupChanged
        }
    };
    Some((kind, payload))
}

#[cfg(test)]
mod event_projection_tests {
    use super::*;
    use crate::{DiffNodeKind, DockEdge, Node, Orientation, ScratchpadConfig};
    use std::collections::BTreeSet;

    #[test]
    fn every_diff_patch_family_maps_to_a_typed_framework_fact() {
        use crate::FrameworkEventKind as Kind;
        let node = Node::panel("node", "panel");
        let window = crate::WindowId::from("window");
        let patches = vec![
            (
                DiffPatch::SplitOrientationChanged {
                    split: "split".into(),
                    orientation: Orientation::Horizontal,
                },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::TabPinChanged {
                    panel: "panel".into(),
                    pinned: true,
                },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::PanelWorkspaceMembershipChanged {
                    panel: "panel".into(),
                    tags: BTreeSet::from(["work".into()]),
                },
                Kind::PanelWorkspaceMembershipChanged,
            ),
            (
                DiffPatch::NodeCreated {
                    id: "node".into(),
                    node: node.clone(),
                },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::NodeRemoved { id: "node".into() },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::RootChanged {
                    old_root: None,
                    new_root: Some("node".into()),
                },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::PanelReparented {
                    panel: "panel".into(),
                    from_parent: "old".into(),
                    to_parent: "new".into(),
                    insertion_index: 0,
                    from_window: None,
                    to_window: None,
                },
                Kind::PanelMoved,
            ),
            (
                DiffPatch::PanelReparented {
                    panel: "panel".into(),
                    from_parent: "".into(),
                    to_parent: "tabs".into(),
                    insertion_index: 0,
                    from_window: None,
                    to_window: None,
                },
                Kind::PanelCreated,
            ),
            (
                DiffPatch::ContainerKindChanged {
                    id: "container".into(),
                    kind: DiffNodeKind::Split,
                },
                Kind::ModalityChanged,
            ),
            (
                DiffPatch::ContainerOrderChanged {
                    parent: "container".into(),
                    ordered_children: vec!["node".into()],
                },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::ActiveTabChanged {
                    tabs: "tabs".into(),
                    active_index: 0,
                },
                Kind::FocusChanged,
            ),
            (
                DiffPatch::SplitRatioUpdated {
                    split: "split".into(),
                    weights: vec![0.5, 0.5],
                },
                Kind::GroupChanged,
            ),
            (
                DiffPatch::ExtensionPayloadUpdated {
                    container: "container".into(),
                    modality: "extension".into(),
                    payload: "{}".into(),
                },
                Kind::ModalityChanged,
            ),
            (
                DiffPatch::WindowCreated {
                    window: window.clone(),
                },
                Kind::WindowFloated,
            ),
            (
                DiffPatch::WindowDestroyed {
                    window: window.clone(),
                },
                Kind::WindowDocked,
            ),
            (
                DiffPatch::WindowGeometryUpdated {
                    window: window.clone(),
                    geometry: crate::Rect {
                        x: 1,
                        y: 2,
                        width: 320,
                        height: 240,
                    },
                },
                Kind::WindowGeometryChanged,
            ),
            (
                DiffPatch::WindowZOrderChanged {
                    window: window.clone(),
                    z_order: 1,
                },
                Kind::WindowZOrderChanged,
            ),
            (
                DiffPatch::WindowMonitorChanged {
                    window: window.clone(),
                    monitor: "display-1".into(),
                    scale: 1.0,
                },
                Kind::WindowMonitorChanged,
            ),
            (
                DiffPatch::WindowPictureInPictureChanged {
                    window: window.clone(),
                    enabled: true,
                },
                Kind::PictureInPictureChanged,
            ),
            (
                DiffPatch::ScratchpadChanged {
                    window: window.clone(),
                    config: Some(ScratchpadConfig {
                        name: "terminal".into(),
                        visible: true,
                        summon_position: None,
                        auto_hide_on_deactivate: false,
                        pinned_across_workspaces: false,
                    }),
                },
                Kind::ScratchpadChanged,
            ),
            (
                DiffPatch::PanelVisibilityChanged {
                    panel: "panel".into(),
                    visible: true,
                },
                Kind::PanelVisibilityChanged,
            ),
            (
                DiffPatch::DrawerChanged {
                    panel: "panel".into(),
                    auto_hidden: true,
                    edge: DockEdge::Left,
                },
                Kind::PanelDrawerChanged,
            ),
            (
                DiffPatch::MaximizedPanelChanged {
                    panel: Some("panel".into()),
                },
                Kind::FocusChanged,
            ),
        ];

        for (patch, expected_kind) in patches {
            let (kind, payload) = event_for_patch(&patch).expect("every diff patch emits a fact");
            assert_eq!(kind, expected_kind, "patch={patch:?}");
            assert!(!payload.is_empty(), "patch={patch:?}");
        }

        for (from_window, to_window, expected_kind) in [
            (None, Some(window.clone()), Kind::WindowFloated),
            (Some(window.clone()), None, Kind::WindowDocked),
        ] {
            let (kind, payload) = event_for_patch(&DiffPatch::PanelReparented {
                panel: "panel".into(),
                from_parent: "old".into(),
                to_parent: "new".into(),
                insertion_index: 0,
                from_window,
                to_window,
            })
            .expect("floating transitions emit a fact");
            assert_eq!(kind, expected_kind);
            assert!(payload.contains_key("panel"));
        }
    }
}
