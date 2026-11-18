//! Revision-bound transient drag sessions.
//!
//! A drag preview is host-owned and never changes canonical layout state. The
//! host commits the returned target through `DockingService::transact` only if
//! the workspace revision still matches the session's starting revision.
use crate::{DockError, DockResult, ErrorCode, PanelId};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragCancelReason {
    Escape,
    PointerCaptureLost,
    HostFocusLost,
    ViewportClosed,
    SourceRemoved,
    TargetRemoved,
    RevisionConflict,
    PointerLeftClient,
    NativeMoveInterrupted,
    Explicit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DragSession<T> {
    base_revision: u64,
    panels: Vec<PanelId>,
    preview_target: Option<T>,
    cancelled: bool,
    cancel_reason: Option<DragCancelReason>,
}

impl<T> DragSession<T> {
    pub fn begin(base_revision: u64, panels: Vec<PanelId>) -> DockResult<Self> {
        let unique: BTreeSet<_> = panels.iter().collect();
        if panels.is_empty() || unique.len() != panels.len() {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "A drag session requires at least one uniquely identified panel.",
                "drag-session",
            ));
        }
        Ok(Self {
            base_revision,
            panels,
            preview_target: None,
            cancelled: false,
            cancel_reason: None,
        })
    }

    pub fn base_revision(&self) -> u64 {
        self.base_revision
    }

    pub fn panels(&self) -> &[PanelId] {
        &self.panels
    }

    pub fn preview_target(&self) -> Option<&T> {
        self.preview_target.as_ref()
    }

    pub fn update_preview(&mut self, observed_revision: u64, target: Option<T>) -> DockResult<()> {
        if self.cancelled {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "A cancelled drag session cannot be updated.",
                format!("drag-revision={}", self.base_revision),
            ));
        }
        self.ensure_revision(observed_revision)?;
        self.preview_target = target;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.cancel_with_reason(DragCancelReason::Explicit);
    }

    pub fn cancel_with_reason(&mut self, reason: DragCancelReason) {
        self.cancelled = true;
        self.cancel_reason = Some(reason);
        self.preview_target = None;
    }

    pub fn cancel_reason(&self) -> Option<DragCancelReason> {
        self.cancel_reason
    }

    /// Consume the session and return the accepted target, if any.
    /// A cancelled or targetless drag completes as `Ok(None)`.
    pub fn commit(self, observed_revision: u64) -> DockResult<Option<T>> {
        self.ensure_revision(observed_revision)?;
        if self.cancelled {
            return Ok(None);
        }
        Ok(self.preview_target)
    }

    fn ensure_revision(&self, observed_revision: u64) -> DockResult<()> {
        if self.base_revision != observed_revision {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "The workspace changed during the drag session.",
                format!(
                    "drag-revision={}:observed-revision={observed_revision}",
                    self.base_revision
                ),
            ));
        }
        Ok(())
    }
}
