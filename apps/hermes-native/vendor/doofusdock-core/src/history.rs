use crate::*;
use std::collections::VecDeque;
#[derive(Clone, Debug)]
pub struct UndoStack {
    capacity: usize,
    undo: VecDeque<HistoryEntry>,
    redo: VecDeque<HistoryEntry>,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    document: LayoutDocument,
    transfer_id: Option<String>,
}
impl UndoStack {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
        }
    }
    pub fn undo_size(&self) -> usize {
        self.undo.len()
    }
    pub fn redo_size(&self) -> usize {
        self.redo.len()
    }
    pub fn execute(
        &mut self,
        doc: &mut LayoutDocument,
        op: Operation,
    ) -> DockResult<TransactionResult> {
        let result = apply_operation(doc, op)?;
        if !result.diff.is_empty() {
            if self.capacity > 0 {
                self.undo.push_back(HistoryEntry {
                    document: doc.clone(),
                    transfer_id: None,
                });
                while self.undo.len() > self.capacity {
                    self.undo.pop_front();
                }
            }
            self.redo.clear();
            *doc = result.document.clone();
        }
        Ok(result)
    }
    /// Apply several operations as one atomic, undoable revision.
    ///
    /// Every operation is validated against a private candidate snapshot. If
    /// any operation fails, the caller's document and history remain unchanged.
    pub fn execute_batch(
        &mut self,
        doc: &mut LayoutDocument,
        command: CommandId,
        operations: impl IntoIterator<Item = Operation>,
    ) -> DockResult<TransactionResult> {
        let mut operations: Vec<_> = operations.into_iter().collect();
        if operations.is_empty() || command.is_empty() {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "A transaction requires a command ID and at least one operation.",
                command.0,
            ));
        }
        if operations.len() == 1 {
            let mut result = self.execute(doc, operations.pop().expect("one operation"))?;
            result.operation_id = command.0;
            return Ok(result);
        }
        let before = doc.clone();
        let mut candidate = before.clone();
        for operation in operations {
            candidate = apply_operation(&candidate, operation)?.document;
        }
        let diff = diff_layouts(&before, &candidate);
        if !diff.is_empty() {
            candidate.revision = before.revision.checked_add(1).ok_or_else(|| {
                DockError::new(
                    ErrorCode::ResourceLimit,
                    "Layout revision overflow.",
                    before.id.0.clone(),
                )
            })?;
            validate_layout(&candidate)?;
            if self.capacity > 0 {
                self.undo.push_back(HistoryEntry {
                    document: before,
                    transfer_id: None,
                });
                while self.undo.len() > self.capacity {
                    self.undo.pop_front();
                }
            }
            self.redo.clear();
            *doc = candidate.clone();
        }
        Ok(TransactionResult {
            document: candidate,
            diff,
            operation_id: command.0,
        })
    }
    pub fn apply_profile(
        &mut self,
        doc: &mut LayoutDocument,
        profile: &WorkspaceProfile,
        assignments: &std::collections::BTreeMap<String, Vec<PanelId>>,
    ) -> DockResult<ProfileApplication> {
        let result = apply_workspace_profile(doc, profile, assignments)?;
        if !result.diff.is_empty() {
            if self.capacity > 0 {
                self.undo.push_back(HistoryEntry {
                    document: doc.clone(),
                    transfer_id: None,
                });
                while self.undo.len() > self.capacity {
                    self.undo.pop_front();
                }
            }
            self.redo.clear();
            *doc = result.document.clone();
        }
        Ok(result)
    }
    /// Accept a complete presentation target once, retaining ordinary undo semantics.
    pub fn apply_layout(
        &mut self,
        doc: &mut LayoutDocument,
        target: &LayoutDocument,
    ) -> DockResult<TransactionResult> {
        validate_layout_replacement(doc, target)?;
        let mut next = target.clone();
        next.revision = doc.revision;
        let diff = diff_layouts(doc, &next);
        if !diff.is_empty() {
            next.revision = doc.revision.checked_add(1).ok_or_else(|| {
                DockError::new(
                    ErrorCode::ResourceLimit,
                    "Revision overflow.",
                    "replace-layout",
                )
            })?;
            if self.capacity > 0 {
                self.undo.push_back(HistoryEntry {
                    document: doc.clone(),
                    transfer_id: None,
                });
                while self.undo.len() > self.capacity {
                    self.undo.pop_front();
                }
            }
            self.redo.clear();
            *doc = next.clone();
        }
        Ok(TransactionResult {
            document: next,
            diff,
            operation_id: "replace-layout".into(),
        })
    }
    pub fn undo(&mut self, doc: &mut LayoutDocument) -> DockResult<TransactionResult> {
        let entry = self
            .undo
            .pop_back()
            .ok_or_else(|| DockError::new(ErrorCode::NotFound, "Undo history is empty.", "undo"))?;
        if entry.transfer_id.is_some() {
            self.undo.push_back(entry);
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Workspace transfer history must be undone through its owning workspace host.",
                doc.workspace.0.clone(),
            ));
        }
        self.redo.push_back(HistoryEntry {
            document: doc.clone(),
            transfer_id: None,
        });
        let mut restored = entry.document;
        restored.revision = doc.revision + 1;
        let diff = diff_layouts(doc, &restored);
        *doc = restored.clone();
        Ok(TransactionResult {
            document: restored,
            diff,
            operation_id: "undo".into(),
        })
    }
    pub fn redo(&mut self, doc: &mut LayoutDocument) -> DockResult<TransactionResult> {
        let entry = self
            .redo
            .pop_back()
            .ok_or_else(|| DockError::new(ErrorCode::NotFound, "Redo history is empty.", "redo"))?;
        if entry.transfer_id.is_some() {
            self.redo.push_back(entry);
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Workspace transfer history must be redone through its owning workspace host.",
                doc.workspace.0.clone(),
            ));
        }
        self.undo.push_back(HistoryEntry {
            document: doc.clone(),
            transfer_id: None,
        });
        let mut restored = entry.document;
        restored.revision = doc.revision + 1;
        let diff = diff_layouts(doc, &restored);
        *doc = restored.clone();
        Ok(TransactionResult {
            document: restored,
            diff,
            operation_id: "redo".into(),
        })
    }

    /// Replace one side of a host-coordinated transfer while tagging its undo
    /// entry so a workspace host can restore both documents together.
    pub(crate) fn apply_transfer_layout(
        &mut self,
        doc: &mut LayoutDocument,
        target: &LayoutDocument,
        transfer_id: &str,
    ) -> DockResult<TransactionResult> {
        if self.capacity == 0 {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Workspace transfer requires undo capacity in both participating workspaces.",
                transfer_id,
            ));
        }
        if transfer_id.is_empty() || transfer_id.len() > 128 {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Workspace transfer ID is invalid.",
                transfer_id,
            ));
        }
        validate_layout(doc)?;
        validate_layout(target)?;
        if doc.workspace != target.workspace || doc.id != target.id {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "A transfer replacement must retain its document and workspace identity.",
                format!(
                    "source={}:{} target={}:{}",
                    doc.workspace.0, doc.id.0, target.workspace.0, target.id.0
                ),
            ));
        }
        let mut next = target.clone();
        next.revision = doc.revision;
        let diff = diff_layouts(doc, &next);
        if !diff.is_empty() {
            next.revision = doc.revision.checked_add(1).ok_or_else(|| {
                DockError::new(ErrorCode::ResourceLimit, "Revision overflow.", transfer_id)
            })?;
            validate_layout(&next)?;
            self.undo.push_back(HistoryEntry {
                document: doc.clone(),
                transfer_id: Some(transfer_id.to_owned()),
            });
            while self.undo.len() > self.capacity {
                self.undo.pop_front();
            }
            self.redo.clear();
            *doc = next.clone();
        }
        Ok(TransactionResult {
            document: next,
            diff,
            operation_id: transfer_id.to_owned(),
        })
    }

    pub(crate) fn undo_transfer(
        &mut self,
        doc: &mut LayoutDocument,
        transfer_id: &str,
    ) -> DockResult<TransactionResult> {
        let entry = self.undo.pop_back().ok_or_else(|| {
            DockError::new(
                ErrorCode::Conflict,
                "Paired transfer is absent from undo history.",
                transfer_id,
            )
        })?;
        if entry.transfer_id.as_deref() != Some(transfer_id) {
            self.undo.push_back(entry);
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Paired transfer is not the next undo entry.",
                transfer_id,
            ));
        }
        let before = doc.clone();
        let mut restored = entry.document;
        restored.revision = doc.revision.checked_add(1).ok_or_else(|| {
            DockError::new(ErrorCode::ResourceLimit, "Revision overflow.", transfer_id)
        })?;
        validate_layout(&restored)?;
        self.redo.push_back(HistoryEntry {
            document: before.clone(),
            transfer_id: Some(transfer_id.to_owned()),
        });
        let diff = diff_layouts(&before, &restored);
        *doc = restored.clone();
        Ok(TransactionResult {
            document: restored,
            diff,
            operation_id: transfer_id.to_owned(),
        })
    }

    pub(crate) fn redo_transfer(
        &mut self,
        doc: &mut LayoutDocument,
        transfer_id: &str,
    ) -> DockResult<TransactionResult> {
        let entry = self.redo.pop_back().ok_or_else(|| {
            DockError::new(
                ErrorCode::Conflict,
                "Paired transfer is absent from redo history.",
                transfer_id,
            )
        })?;
        if entry.transfer_id.as_deref() != Some(transfer_id) {
            self.redo.push_back(entry);
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Paired transfer is not the next redo entry.",
                transfer_id,
            ));
        }
        let before = doc.clone();
        let mut restored = entry.document;
        restored.revision = doc.revision.checked_add(1).ok_or_else(|| {
            DockError::new(ErrorCode::ResourceLimit, "Revision overflow.", transfer_id)
        })?;
        validate_layout(&restored)?;
        self.undo.push_back(HistoryEntry {
            document: before.clone(),
            transfer_id: Some(transfer_id.to_owned()),
        });
        while self.undo.len() > self.capacity {
            self.undo.pop_front();
        }
        let diff = diff_layouts(&before, &restored);
        *doc = restored.clone();
        Ok(TransactionResult {
            document: restored,
            diff,
            operation_id: transfer_id.to_owned(),
        })
    }

    pub(crate) fn top_undo_transfer_id(&self) -> Option<&str> {
        self.undo.back()?.transfer_id.as_deref()
    }

    pub(crate) fn top_redo_transfer_id(&self) -> Option<&str> {
        self.redo.back()?.transfer_id.as_deref()
    }

    pub(crate) fn contains_transfer_id(&self, transfer_id: &str) -> bool {
        self.undo
            .iter()
            .chain(self.redo.iter())
            .any(|entry| entry.transfer_id.as_deref() == Some(transfer_id))
    }

    pub(crate) fn discard_transfer_redo(&mut self, transfer_id: &str) {
        self.redo
            .retain(|entry| entry.transfer_id.as_deref() != Some(transfer_id));
    }
}
