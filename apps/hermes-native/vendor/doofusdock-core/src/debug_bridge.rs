//! Transport-neutral, versioned debug requests for trusted host adapters.
//!
//! This module is a protocol dispatcher, not a network listener or an
//! authentication system. A host must authenticate callers before creating a
//! `DebugBridgePrincipal`, then enforce its local transport policy. The wire
//! schema contains no credential or permission fields, so requests cannot
//! grant themselves access. Code running inside the host process remains part
//! of the trusted computing base.

use crate::{
    AuditStream, CommandId, DockError, DockResult, DockingService, ErrorCode, LayoutDocument,
    Operation,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const DEBUG_BRIDGE_PROTOCOL_VERSION: u16 = 1;
pub const MAX_DEBUG_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_DEBUG_RESPONSE_BYTES: usize = 1024 * 1024;
pub const MAX_DEBUG_REQUEST_ID_BYTES: usize = 128;
pub const MAX_DEBUG_ACTOR_BYTES: usize = 128;
pub const MAX_DEBUG_OPERATIONS: usize = 32;

/// The capabilities granted by an authenticated host to one debug caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugBridgePermission {
    InspectLayout,
    MutateLayout,
}

/// A transport-authenticated identity and its explicit per-session grants.
///
/// The constructor does not authenticate the identity or enforce access to the
/// host process. A host adapter must call it only after authenticating the
/// transport caller and assigning that identity's explicit grants. Never
/// derive these permissions from fields in a debug request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugBridgePrincipal {
    actor: String,
    permissions: BTreeSet<DebugBridgePermission>,
}

impl DebugBridgePrincipal {
    pub fn new(
        actor: impl Into<String>,
        permissions: impl IntoIterator<Item = DebugBridgePermission>,
    ) -> DockResult<Self> {
        let actor = actor.into();
        if actor.trim().is_empty() || actor.len() > MAX_DEBUG_ACTOR_BYTES {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Debug principal actor must be non-empty and bounded.",
                actor.len().to_string(),
            ));
        }
        Ok(Self {
            actor,
            permissions: permissions.into_iter().collect(),
        })
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }

    pub fn has_permission(&self, permission: DebugBridgePermission) -> bool {
        self.permissions.contains(&permission)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugBridgeRequest {
    pub protocol_version: u16,
    pub request_id: String,
    pub command: DebugBridgeCommand,
}

/// Host-owned reads are represented explicitly and fail closed in the core
/// dispatcher until a host adapter supplies their live snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugHostQuery {
    Capabilities,
    ActiveTheme,
    PresentationAndAnimation,
    FocusAndInputRouting,
    PerformanceCounters,
    ValidationFailures,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DebugBridgeCommand {
    InspectLayout,
    ApplyOperations {
        expected_revision: u64,
        command_id: CommandId,
        operations: Vec<Operation>,
    },
    InspectHost {
        query: DebugHostQuery,
    },
    PreviewThemePackage,
    PreviewTransition,
    ApplyHostOperation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugBridgeResponse {
    pub protocol_version: u16,
    pub request_id: Option<String>,
    pub result: Result<DebugBridgePayload, DebugBridgeFault>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DebugBridgePayload {
    Layout(DebugLayoutSnapshot),
    Mutation(DebugMutationReceipt),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugLayoutSnapshot {
    pub revision: u64,
    pub document: LayoutDocument,
    pub undo_depth: usize,
    pub redo_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugMutationReceipt {
    pub operation_id: String,
    pub base_revision: u64,
    pub revision: u64,
    pub patch_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugBridgeFault {
    pub code: String,
    pub message: String,
    pub context: String,
}

impl DebugBridgeFault {
    fn from_error(error: &DockError) -> Self {
        Self {
            code: error_code_name(error.code).to_owned(),
            message: error.message.clone(),
            context: error.context.clone(),
        }
    }
}

/// Executes the core-owned subset of the development protocol.
///
/// Mutations are simulated against a private service clone first. The bounded
/// audit record is appended to the supplied `AuditStream` before the same
/// revision-checked command is applied to the live service, so an audit-write
/// failure cannot silently permit a debug mutation. `AuditStream` is
/// in-memory when configured with an empty path; hosts requiring durable audit
/// must supply a stream with a persistent path. The caller owns the stream's
/// lifetime and persistence policy.
pub struct DebugBridge<'a> {
    service: &'a mut DockingService,
    audit: &'a mut AuditStream,
}

impl<'a> DebugBridge<'a> {
    pub fn new(service: &'a mut DockingService, audit: &'a mut AuditStream) -> Self {
        Self { service, audit }
    }

    /// Parse and dispatch one bounded JSON request, always returning a
    /// versioned JSON result when the response itself fits the response bound.
    pub fn dispatch_json(
        &mut self,
        bytes: &[u8],
        principal: &DebugBridgePrincipal,
    ) -> DockResult<Vec<u8>> {
        if bytes.len() > MAX_DEBUG_REQUEST_BYTES {
            return self.encode_response(response_error(
                None,
                DockError::new(
                    ErrorCode::ResourceLimit,
                    "Debug request exceeds the protocol byte limit.",
                    bytes.len().to_string(),
                ),
            ));
        }

        let request: DebugBridgeRequest = match parse_debug_request(bytes) {
            Ok(request) => request,
            Err(error) => {
                return self.encode_response(response_error(
                    None,
                    DockError::new(
                        ErrorCode::SchemaError,
                        "Debug request is not valid protocol JSON.",
                        error.to_string(),
                    ),
                ));
            }
        };

        let request_id = request.request_id.clone();
        if let Err(error) = validate_request(&request) {
            return self.encode_response(response_error(Some(request_id), error));
        }
        if request.protocol_version != DEBUG_BRIDGE_PROTOCOL_VERSION {
            return self.encode_response(response_error(
                Some(request_id),
                DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Debug protocol version is not supported.",
                    request.protocol_version.to_string(),
                ),
            ));
        }

        match request.command {
            DebugBridgeCommand::InspectLayout => {
                if !principal.has_permission(DebugBridgePermission::InspectLayout) {
                    return self.encode_response(response_error(
                        Some(request_id),
                        permission_error("inspect_layout"),
                    ));
                }
                let snapshot = self.service.snapshot();
                self.encode_response(DebugBridgeResponse {
                    protocol_version: DEBUG_BRIDGE_PROTOCOL_VERSION,
                    request_id: Some(request_id),
                    result: Ok(DebugBridgePayload::Layout(DebugLayoutSnapshot {
                        revision: snapshot.revision,
                        document: snapshot.document,
                        undo_depth: snapshot.undo_depth,
                        redo_depth: snapshot.redo_depth,
                    })),
                })
            }
            DebugBridgeCommand::ApplyOperations {
                expected_revision,
                command_id,
                operations,
            } => self.apply_operations(
                request_id,
                principal,
                expected_revision,
                command_id,
                operations,
            ),
            DebugBridgeCommand::InspectHost { query } => self.encode_response(response_error(
                Some(request_id),
                unsupported_host_query(query),
            )),
            DebugBridgeCommand::PreviewThemePackage => self.encode_response(response_error(
                Some(request_id),
                unsupported_host_action("preview_theme_package"),
            )),
            DebugBridgeCommand::PreviewTransition => self.encode_response(response_error(
                Some(request_id),
                unsupported_host_action("preview_transition"),
            )),
            DebugBridgeCommand::ApplyHostOperation => self.encode_response(response_error(
                Some(request_id),
                unsupported_host_action("apply_host_operation"),
            )),
        }
    }

    fn apply_operations(
        &mut self,
        request_id: String,
        principal: &DebugBridgePrincipal,
        expected_revision: u64,
        command_id: CommandId,
        operations: Vec<Operation>,
    ) -> DockResult<Vec<u8>> {
        let before = self.service.snapshot().document;
        if !principal.has_permission(DebugBridgePermission::MutateLayout) {
            return self.audit_rejected_mutation(
                request_id,
                &before,
                permission_error("apply_operations"),
                principal.actor(),
            );
        }
        if operations.is_empty() || operations.len() > MAX_DEBUG_OPERATIONS {
            let error = DockError::new(
                ErrorCode::ResourceLimit,
                "Debug mutation must contain between one and the operation limit.",
                operations.len().to_string(),
            );
            return self.audit_rejected_mutation(request_id, &before, error, principal.actor());
        }
        if before.revision != expected_revision {
            let error = DockError::new(
                ErrorCode::Conflict,
                "Debug mutation expected a stale layout revision.",
                format!("expected={expected_revision};actual={}", before.revision),
            );
            return self.audit_rejected_mutation(request_id, &before, error, principal.actor());
        }

        let mut staged = self.service.staged_clone();
        let staged_result = match staged.transact(
            expected_revision,
            command_id.clone(),
            operations.iter().cloned(),
        ) {
            Ok(result) => result,
            Err(error) => {
                return self.audit_rejected_mutation(request_id, &before, error, principal.actor());
            }
        };

        if let Err(error) = self.audit.append(
            &before,
            Ok(staged_result.clone()),
            principal.actor(),
            "debug bridge apply operations",
        ) {
            return self.encode_response(response_error(Some(request_id), error));
        }

        let committed = match self
            .service
            .transact(expected_revision, command_id, operations)
        {
            Ok(result) => result,
            Err(error) => {
                let _ = self.audit.append(
                    &before,
                    Err(error.clone()),
                    principal.actor(),
                    "debug bridge commit after staged validation",
                );
                return self.encode_response(response_error(Some(request_id), error));
            }
        };
        let payload = DebugBridgePayload::Mutation(DebugMutationReceipt {
            operation_id: committed.operation_id,
            base_revision: expected_revision,
            revision: committed.document.revision,
            patch_count: committed.diff.patches.len(),
        });
        self.encode_response(DebugBridgeResponse {
            protocol_version: DEBUG_BRIDGE_PROTOCOL_VERSION,
            request_id: Some(request_id),
            result: Ok(payload),
        })
    }

    fn audit_rejected_mutation(
        &mut self,
        request_id: String,
        before: &LayoutDocument,
        error: DockError,
        actor: &str,
    ) -> DockResult<Vec<u8>> {
        match self.audit.append(
            before,
            Err(error.clone()),
            actor,
            "debug bridge rejected mutation",
        ) {
            Ok(()) => self.encode_response(response_error(Some(request_id), error)),
            Err(audit_error) => self.encode_response(response_error(Some(request_id), audit_error)),
        }
    }

    fn encode_response(&self, response: DebugBridgeResponse) -> DockResult<Vec<u8>> {
        let bytes = serde_json::to_vec(&response).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                "Could not serialize debug response.",
                error.to_string(),
            )
        })?;
        if bytes.len() > MAX_DEBUG_RESPONSE_BYTES {
            return serde_json::to_vec(&response_error(
                response.request_id,
                DockError::new(
                    ErrorCode::ResourceLimit,
                    "Debug response exceeds the protocol byte limit.",
                    bytes.len().to_string(),
                ),
            ))
            .map_err(|error| {
                DockError::new(
                    ErrorCode::SerializationError,
                    "Could not serialize bounded debug error response.",
                    error.to_string(),
                )
            });
        }
        Ok(bytes)
    }
}

fn validate_request(request: &DebugBridgeRequest) -> DockResult<()> {
    if request.request_id.trim().is_empty()
        || request.request_id.len() > MAX_DEBUG_REQUEST_ID_BYTES
        || request.request_id.chars().any(char::is_control)
    {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Debug request ID must be non-empty, bounded, and contain no control characters.",
            request.request_id.len().to_string(),
        ));
    }
    Ok(())
}

fn parse_debug_request(bytes: &[u8]) -> DockResult<DebugBridgeRequest> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        DockError::new(
            ErrorCode::SchemaError,
            "Debug request is not valid protocol JSON.",
            error.to_string(),
        )
    })?;
    let root = value.as_object().ok_or_else(|| {
        DockError::new(
            ErrorCode::SchemaError,
            "Debug request must be a JSON object.",
            "request",
        )
    })?;
    reject_unknown_keys(
        root,
        &["protocol_version", "request_id", "command"],
        "request",
    )?;
    let command = root
        .get("command")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::SchemaError,
                "Debug command must be a tagged JSON object.",
                "command",
            )
        })?;
    let kind = command
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::SchemaError,
                "Debug command requires a string type tag.",
                "command.type",
            )
        })?;
    let allowed: &[&str] = match kind {
        "inspect_layout"
        | "preview_theme_package"
        | "preview_transition"
        | "apply_host_operation" => &["type"],
        "apply_operations" => &["type", "expected_revision", "command_id", "operations"],
        "inspect_host" => &["type", "query"],
        _ => {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Debug command type is not supported by this protocol version.",
                kind,
            ));
        }
    };
    reject_unknown_keys(command, allowed, "command")?;
    serde_json::from_value(value).map_err(|error| {
        DockError::new(
            ErrorCode::SchemaError,
            "Debug request fields do not match the protocol schema.",
            error.to_string(),
        )
    })
}

fn reject_unknown_keys(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    context: &str,
) -> DockResult<()> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Debug request contains an unknown field.",
            format!("{context}.{key}"),
        ));
    }
    Ok(())
}

fn response_error(request_id: Option<String>, error: DockError) -> DebugBridgeResponse {
    DebugBridgeResponse {
        protocol_version: DEBUG_BRIDGE_PROTOCOL_VERSION,
        request_id,
        result: Err(DebugBridgeFault::from_error(&error)),
    }
}

fn permission_error(action: &str) -> DockError {
    DockError::new(
        ErrorCode::UnsupportedOperation,
        "Debug principal lacks the permission required for this request.",
        action,
    )
}

fn unsupported_host_query(query: DebugHostQuery) -> DockError {
    DockError::new(
        ErrorCode::UnsupportedOperation,
        "This host-specific inspection is not available from the core dispatcher.",
        format!("{query:?}"),
    )
}

fn unsupported_host_action(action: &str) -> DockError {
    DockError::new(
        ErrorCode::UnsupportedOperation,
        "This host-specific debug action is not available from the core dispatcher.",
        action,
    )
}

fn error_code_name(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::InvalidId => "invalid_id",
        ErrorCode::DuplicateId => "duplicate_id",
        ErrorCode::NotFound => "not_found",
        ErrorCode::InvalidLayout => "invalid_layout",
        ErrorCode::ConstraintViolation => "constraint_violation",
        ErrorCode::UnsupportedOperation => "unsupported_operation",
        ErrorCode::SerializationError => "serialization_error",
        ErrorCode::SchemaError => "schema_error",
        ErrorCode::ResourceLimit => "resource_limit",
        ErrorCode::ExtensionError => "extension_error",
        ErrorCode::PersistenceError => "persistence_error",
        ErrorCode::Conflict => "conflict",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Node, PanelDescriptor};
    use std::sync::Arc;

    fn service() -> DockingService {
        let mut document = LayoutDocument::new("debug-layout", "debug-workspace");
        Arc::make_mut(&mut document.panels).push(PanelDescriptor::new("panel-a", "Panel A"));
        document.root = Some(Node::tabs(
            "tabs",
            vec![Node::panel("panel-a-node", "panel-a")],
            0,
        ));
        DockingService::new(document, 8).unwrap()
    }

    fn audit() -> AuditStream {
        AuditStream::new(
            "",
            crate::AuditOptions {
                include_actor: true,
                include_reason: true,
                ..crate::AuditOptions::default()
            },
        )
    }

    fn request(command: DebugBridgeCommand) -> Vec<u8> {
        serde_json::to_vec(&DebugBridgeRequest {
            protocol_version: DEBUG_BRIDGE_PROTOCOL_VERSION,
            request_id: "test-request".into(),
            command,
        })
        .unwrap()
    }

    fn principal(permissions: &[DebugBridgePermission]) -> DebugBridgePrincipal {
        DebugBridgePrincipal::new("test-agent", permissions.iter().copied()).unwrap()
    }

    fn response(bytes: &[u8]) -> DebugBridgeResponse {
        serde_json::from_slice(bytes).unwrap()
    }

    #[test]
    fn read_only_principal_inspects_layout_without_audit_or_mutation() {
        let mut service = service();
        let before = service.snapshot();
        let mut audit = audit();
        let output = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(
                &request(DebugBridgeCommand::InspectLayout),
                &principal(&[DebugBridgePermission::InspectLayout]),
            )
            .unwrap();
        let decoded = response(&output);
        assert_eq!(decoded.protocol_version, DEBUG_BRIDGE_PROTOCOL_VERSION);
        assert_eq!(decoded.request_id.as_deref(), Some("test-request"));
        assert!(matches!(decoded.result, Ok(DebugBridgePayload::Layout(_))));
        assert_eq!(service.snapshot(), before);
        assert!(audit.records().is_empty());
    }

    #[test]
    fn mutation_requires_explicit_permission_and_records_denial() {
        let mut service = service();
        let before = service.snapshot();
        let mut audit = audit();
        let output = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(
                &request(DebugBridgeCommand::ApplyOperations {
                    expected_revision: 0,
                    command_id: "debug-pin".into(),
                    operations: vec![Operation::SetTabPinned {
                        panel: "panel-a".into(),
                        pinned: true,
                    }],
                }),
                &principal(&[DebugBridgePermission::InspectLayout]),
            )
            .unwrap();
        let decoded = response(&output);
        assert!(matches!(decoded.result, Err(DebugBridgeFault { .. })));
        assert_eq!(decoded.result.unwrap_err().code, "unsupported_operation");
        assert_eq!(service.snapshot(), before);
        assert_eq!(audit.records().len(), 1);
        assert!(!audit.records()[0].succeeded);
        assert_eq!(audit.records()[0].actor, "test-agent");
    }

    #[test]
    fn authorized_mutation_is_revision_bound_and_audited_before_commit() {
        let mut service = service();
        let mut audit = audit();
        let caller = principal(&[
            DebugBridgePermission::InspectLayout,
            DebugBridgePermission::MutateLayout,
        ]);
        let output = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(
                &request(DebugBridgeCommand::ApplyOperations {
                    expected_revision: 0,
                    command_id: "debug-pin".into(),
                    operations: vec![Operation::SetTabPinned {
                        panel: "panel-a".into(),
                        pinned: true,
                    }],
                }),
                &caller,
            )
            .unwrap();
        let decoded = response(&output);
        let Ok(DebugBridgePayload::Mutation(receipt)) = decoded.result else {
            panic!("expected mutation receipt");
        };
        assert_eq!(receipt.base_revision, 0);
        assert_eq!(receipt.revision, 1);
        assert_eq!(receipt.patch_count, 1);
        assert!(
            service
                .snapshot()
                .document
                .pinned_panels
                .contains(&crate::PanelId::from("panel-a"))
        );
        assert_eq!(audit.records().len(), 1);
        assert!(audit.records()[0].succeeded);

        let stale = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(
                &request(DebugBridgeCommand::ApplyOperations {
                    expected_revision: 0,
                    command_id: "debug-stale".into(),
                    operations: vec![Operation::SetTabPinned {
                        panel: "panel-a".into(),
                        pinned: false,
                    }],
                }),
                &caller,
            )
            .unwrap();
        assert_eq!(response(&stale).result.unwrap_err().code, "conflict");
        assert!(
            service
                .snapshot()
                .document
                .pinned_panels
                .contains(&crate::PanelId::from("panel-a"))
        );
        assert_eq!(audit.records().len(), 2);
        assert!(!audit.records()[1].succeeded);
    }

    #[test]
    fn unknown_versions_oversized_requests_and_host_actions_fail_closed() {
        let mut service = service();
        let mut audit = audit();
        let caller = principal(&[
            DebugBridgePermission::InspectLayout,
            DebugBridgePermission::MutateLayout,
        ]);
        let host_only_commands = [
            DebugBridgeCommand::InspectHost {
                query: DebugHostQuery::PerformanceCounters,
            },
            DebugBridgeCommand::PreviewThemePackage,
            DebugBridgeCommand::PreviewTransition,
            DebugBridgeCommand::ApplyHostOperation,
        ];
        for command in host_only_commands {
            let unsupported = DebugBridge::new(&mut service, &mut audit)
                .dispatch_json(&request(command), &caller)
                .unwrap();
            assert_eq!(
                response(&unsupported).result.unwrap_err().code,
                "unsupported_operation"
            );
        }

        let mut too_new = request(DebugBridgeCommand::InspectLayout);
        let mut parsed: DebugBridgeRequest = serde_json::from_slice(&too_new).unwrap();
        parsed.protocol_version += 1;
        too_new = serde_json::to_vec(&parsed).unwrap();
        let unsupported_version = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(&too_new, &caller)
            .unwrap();
        assert_eq!(
            response(&unsupported_version).result.unwrap_err().code,
            "unsupported_operation"
        );

        let oversized = vec![b' '; MAX_DEBUG_REQUEST_BYTES + 1];
        let oversized = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(&oversized, &caller)
            .unwrap();
        assert_eq!(
            response(&oversized).result.unwrap_err().code,
            "resource_limit"
        );
        assert!(audit.records().is_empty());
    }

    #[test]
    fn schema_and_batch_bounds_reject_unknown_fields_and_excess_operations() {
        let mut service = service();
        let mut audit = audit();
        let caller = principal(&[
            DebugBridgePermission::InspectLayout,
            DebugBridgePermission::MutateLayout,
        ]);
        let unknown = br#"{"protocol_version":1,"request_id":"x","permissions":["mutate_layout"],"command":{"type":"inspect_layout","shell":"bad"}}"#;
        let rejected = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(unknown, &caller)
            .unwrap();
        assert_eq!(response(&rejected).result.unwrap_err().code, "schema_error");

        let forged = br#"{"protocol_version":1,"request_id":"x","command":{"type":"inspect_layout","permissions":["mutate_layout"]}}"#;
        let rejected = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(forged, &caller)
            .unwrap();
        assert_eq!(response(&rejected).result.unwrap_err().code, "schema_error");

        let before = service.snapshot();
        let excess = request(DebugBridgeCommand::ApplyOperations {
            expected_revision: 0,
            command_id: "too-many".into(),
            operations: (0..=MAX_DEBUG_OPERATIONS)
                .map(|_| Operation::SetTabPinned {
                    panel: "panel-a".into(),
                    pinned: true,
                })
                .collect(),
        });
        let rejected = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(&excess, &caller)
            .unwrap();
        assert_eq!(
            response(&rejected).result.unwrap_err().code,
            "resource_limit"
        );
        assert_eq!(service.snapshot(), before);
        assert_eq!(audit.records().len(), 1);
        assert!(!audit.records()[0].succeeded);
    }

    #[test]
    fn failed_audit_write_prevents_the_live_mutation() {
        let mut service = service();
        let before = service.snapshot();
        let parent_file = std::env::temp_dir().join(format!(
            "doofusdock-debug-audit-file-{}.tmp",
            std::process::id()
        ));
        std::fs::write(&parent_file, b"file cannot be an audit directory").unwrap();
        let audit_path = parent_file.join("audit.jsonl");
        let mut audit = AuditStream::new(audit_path, crate::AuditOptions::default());
        let output = DebugBridge::new(&mut service, &mut audit)
            .dispatch_json(
                &request(DebugBridgeCommand::ApplyOperations {
                    expected_revision: 0,
                    command_id: "debug-audit-failure".into(),
                    operations: vec![Operation::SetTabPinned {
                        panel: "panel-a".into(),
                        pinned: true,
                    }],
                }),
                &principal(&[DebugBridgePermission::MutateLayout]),
            )
            .unwrap();
        assert_eq!(
            response(&output).result.unwrap_err().code,
            "persistence_error"
        );
        assert_eq!(service.snapshot(), before);
        std::fs::remove_file(parent_file).unwrap();
    }
}
