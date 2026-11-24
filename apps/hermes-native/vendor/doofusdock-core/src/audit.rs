use crate::*;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationRecord {
    #[serde(rename = "operation_id")]
    pub operation: CommandId,
    pub prior_revision: u64,
    pub new_revision: u64,
    pub actor: String,
    pub timestamp_utc: String,
    pub succeeded: bool,
    pub result_code: String,
    pub reason: String,
    pub patch_kinds: Vec<String>,
}
pub type AuditErrorSink = Arc<dyn Fn(&DockError) + Send + Sync>;
#[derive(Clone)]
pub struct AuditOptions {
    pub max_retained_records: usize,
    pub max_record_bytes: usize,
    pub max_diagnostics_bytes: usize,
    pub include_actor: bool,
    pub include_reason: bool,
    pub error_sink: Option<AuditErrorSink>,
}
impl Default for AuditOptions {
    fn default() -> Self {
        Self {
            max_retained_records: 2048,
            max_record_bytes: 16 * 1024,
            max_diagnostics_bytes: 1024 * 1024,
            include_actor: false,
            include_reason: false,
            error_sink: None,
        }
    }
}
impl AuditOptions {
    fn normalized(mut self) -> Self {
        self.max_retained_records = self.max_retained_records.max(1);
        self.max_record_bytes = self.max_record_bytes.max(128);
        self.max_diagnostics_bytes = self.max_diagnostics_bytes.max(2);
        self
    }
}
pub struct AuditStream {
    path: PathBuf,
    options: AuditOptions,
    records: Vec<OperationRecord>,
    sequence: u64,
}
impl AuditStream {
    pub fn new(path: impl Into<PathBuf>, options: AuditOptions) -> Self {
        Self {
            path: path.into(),
            options: options.normalized(),
            records: Vec::new(),
            sequence: 0,
        }
    }
    pub fn records(&self) -> &[OperationRecord] {
        &self.records
    }
    fn report(&self, error: &DockError) {
        if let Some(sink) = &self.options.error_sink {
            let _ = catch_unwind(AssertUnwindSafe(|| sink(error)));
        }
    }
    pub fn append(
        &mut self,
        before: &LayoutDocument,
        result: DockResult<TransactionResult>,
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> DockResult<()> {
        self.sequence = self.sequence.saturating_add(1);
        let mut record = OperationRecord {
            operation: CommandId::default(),
            prior_revision: before.revision,
            new_revision: before.revision,
            actor: if self.options.include_actor {
                actor.into()
            } else {
                "[redacted]".into()
            },
            timestamp_utc: timestamp_utc(),
            succeeded: false,
            result_code: String::new(),
            reason: if self.options.include_reason {
                reason.into()
            } else {
                "[redacted]".into()
            },
            patch_kinds: vec![],
        };
        match result {
            Ok(result) => {
                record.succeeded = true;
                record.operation = result.operation_id.into();
                record.new_revision = result.document.revision;
                record.result_code = "committed".into();
                record.patch_kinds = result.diff.patches.iter().map(patch_name).collect();
            }
            Err(e) => {
                record.result_code = error_code_number(e.code).to_string();
            }
        }
        if let Err(error) = self.write_record(&record) {
            self.report(&error);
            return Err(error);
        }
        self.retain(record);
        Ok(())
    }
    pub fn append_failure(
        &mut self,
        before: &LayoutDocument,
        _operation: &Operation,
        error: &DockError,
        actor: impl Into<String>,
    ) -> DockResult<()> {
        self.sequence = self.sequence.saturating_add(1);
        let record = OperationRecord {
            operation: CommandId::from(format!("failed-at-r{}-{}", before.revision, self.sequence)),
            prior_revision: before.revision,
            new_revision: before.revision,
            actor: if self.options.include_actor {
                actor.into()
            } else {
                "[redacted]".into()
            },
            timestamp_utc: timestamp_utc(),
            succeeded: false,
            result_code: error_code_number(error.code).to_string(),
            reason: if self.options.include_reason {
                error.message.clone()
            } else {
                "[redacted]".into()
            },
            patch_kinds: vec![],
        };
        if let Err(error) = self.write_record(&record) {
            self.report(&error);
            return Err(error);
        }
        self.retain(record);
        Ok(())
    }
    fn retain(&mut self, record: OperationRecord) {
        self.records.push(record);
        let excess = self
            .records
            .len()
            .saturating_sub(self.options.max_retained_records);
        if excess > 0 {
            self.records.drain(0..excess);
        }
    }
    fn write_record(&self, record: &OperationRecord) -> DockResult<()> {
        let bytes = serde_json::to_vec(record)
            .map_err(|e| DockError::new(ErrorCode::SerializationError, e.to_string(), "audit"))?;
        if bytes.len() + 1 > self.options.max_record_bytes {
            let e = DockError::new(
                ErrorCode::ResourceLimit,
                "Audit record exceeds configured byte limit.",
                (bytes.len() + 1).to_string(),
            );
            return Err(e);
        }
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    "Cannot create audit directory.",
                    e.to_string(),
                )
            })?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    "Cannot append audit record.",
                    format!("{}: {e}", self.path.display()),
                )
            })?;
        file.write_all(&bytes)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.flush())
            .map_err(|e| {
                DockError::new(
                    ErrorCode::PersistenceError,
                    "Audit append failed.",
                    format!("{}: {e}", self.path.display()),
                )
            })
    }
    pub fn diagnostics_json(&self) -> DockResult<String> {
        #[derive(Serialize)]
        struct Diagnostic<'a> {
            prior_revision: u64,
            new_revision: u64,
            succeeded: bool,
            result_code: &'a str,
            patch_kinds: &'a [String],
        }
        let items: Vec<_> = self
            .records
            .iter()
            .map(|r| Diagnostic {
                prior_revision: r.prior_revision,
                new_revision: r.new_revision,
                succeeded: r.succeeded,
                result_code: &r.result_code,
                patch_kinds: &r.patch_kinds,
            })
            .collect();
        let text = match serde_json::to_string_pretty(&items) {
            Ok(text) => text,
            Err(error) => {
                let error = DockError::new(
                    ErrorCode::SerializationError,
                    error.to_string(),
                    "diagnostics",
                );
                self.report(&error);
                return Err(error);
            }
        };
        if text.len() > self.options.max_diagnostics_bytes {
            let e = DockError::new(
                ErrorCode::ResourceLimit,
                "Diagnostics export exceeds configured byte limit.",
                text.len().to_string(),
            );
            self.report(&e);
            return Err(e);
        }
        Ok(text)
    }
}
fn patch_name(p: &DiffPatch) -> String {
    match p {
        DiffPatch::SplitOrientationChanged { .. } => "split_orientation_changed",
        DiffPatch::TabPinChanged { .. } => "tab_pin_changed",
        DiffPatch::PanelWorkspaceMembershipChanged { .. } => "panel_workspace_membership_changed",
        DiffPatch::NodeCreated { .. } => "node_created",
        DiffPatch::NodeRemoved { .. } => "node_removed",
        DiffPatch::RootChanged { .. } => "root_changed",
        DiffPatch::PanelReparented { .. } => "panel_reparented",
        DiffPatch::ContainerKindChanged { .. } => "container_kind_changed",
        DiffPatch::ContainerOrderChanged { .. } => "container_order_changed",
        DiffPatch::ActiveTabChanged { .. } => "active_tab_changed",
        DiffPatch::TabGroupMinimizedChanged { .. } => "tab_group_minimized_changed",
        DiffPatch::SplitRatioUpdated { .. } => "split_ratio_updated",
        DiffPatch::ExtensionPayloadUpdated { .. } => "extension_payload_updated",
        DiffPatch::WindowCreated { .. } => "window_created",
        DiffPatch::WindowDestroyed { .. } => "window_destroyed",
        DiffPatch::WindowGeometryUpdated { .. } => "window_geometry_updated",
        DiffPatch::WindowZOrderChanged { .. } => "window_z_order_changed",
        DiffPatch::WindowMonitorChanged { .. } => "window_monitor_changed",
        DiffPatch::WindowPictureInPictureChanged { .. } => "window_picture_in_picture_changed",
        DiffPatch::ScratchpadChanged { .. } => "scratchpad_changed",
        DiffPatch::PanelVisibilityChanged { .. } => "panel_visibility_changed",
        DiffPatch::DrawerChanged { .. } => "drawer_changed",
        DiffPatch::MaximizedPanelChanged { .. } => "maximized_panel_changed",
    }
    .into()
}
fn error_code_number(code: ErrorCode) -> u8 {
    match code {
        ErrorCode::InvalidId => 0,
        ErrorCode::DuplicateId => 1,
        ErrorCode::NotFound => 2,
        ErrorCode::InvalidLayout => 3,
        ErrorCode::ConstraintViolation => 4,
        ErrorCode::UnsupportedOperation => 5,
        ErrorCode::SerializationError => 6,
        ErrorCode::SchemaError => 7,
        ErrorCode::ResourceLimit => 8,
        ErrorCode::ExtensionError => 9,
        ErrorCode::PersistenceError => 10,
        ErrorCode::Conflict => 11,
    }
}
fn timestamp_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86400) as i64;
    let rem = seconds % 86400;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}
