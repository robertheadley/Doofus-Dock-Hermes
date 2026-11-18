use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Host-aware support summary for a stable family of built-in operations.
/// Core availability describes the canonical model API; host availability
/// describes whether the supplied host declares the capabilities needed to
/// realize that operation family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinOperationSupport {
    pub id: String,
    pub operation_names: Vec<String>,
    pub required_host_capabilities: Vec<String>,
    pub required_any_host_capabilities: Vec<String>,
    pub missing_host_capabilities: Vec<String>,
    pub core_available: bool,
    pub host_available: bool,
    pub applicable_target_kinds: Vec<String>,
    pub constraints: Vec<String>,
    pub reason: String,
}

/// Operation-level, host-aware catalog entry with its strict request schema.
/// Family policy is repeated on each operation so clients can discover one
/// command without having to join multiple registry responses themselves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuiltinOperationDescriptor {
    pub name: String,
    pub family: String,
    pub request_schema: serde_json::Value,
    pub required_host_capabilities: Vec<String>,
    pub required_any_host_capabilities: Vec<String>,
    pub missing_host_capabilities: Vec<String>,
    pub core_available: bool,
    pub host_available: bool,
    pub applicable_target_kinds: Vec<String>,
    pub constraints: Vec<String>,
    pub reason: String,
}

/// Read-only result for checking one proposed operation against a particular
/// document snapshot and host declaration. It never commits or changes history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationPreflightReport {
    pub document_id: String,
    pub source_revision: u64,
    pub operation_id: String,
    pub target_ids: Vec<String>,
    pub core_available: bool,
    pub host_available: bool,
    pub supported: bool,
    pub required_host_capabilities: Vec<String>,
    pub required_any_host_capabilities: Vec<String>,
    pub missing_host_capabilities: Vec<String>,
    pub rejection_code: Option<String>,
    pub reason: String,
}

impl OperationPreflightReport {
    pub fn validate(&self) -> DockResult<()> {
        const ERROR_CODES: &[&str] = &[
            "invalid_id",
            "duplicate_id",
            "not_found",
            "invalid_layout",
            "constraint_violation",
            "unsupported_operation",
            "serialization_error",
            "schema_error",
            "resource_limit",
            "extension_error",
            "persistence_error",
            "conflict",
        ];
        let invalid = self.document_id.is_empty()
            || self.document_id.len() > 4096
            || self.document_id.chars().any(char::is_control)
            || self.operation_id.is_empty()
            || self.operation_id.len() > 256
            || !self.operation_id.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
            || self.target_ids.len() > MAX_PREFLIGHT_TARGETS
            || self.target_ids.iter().any(|target| {
                target.is_empty() || target.len() > 4096 || target.chars().any(char::is_control)
            })
            || self.missing_host_capabilities.len() > 1024
            || self.missing_host_capabilities.iter().any(|capability| {
                capability.is_empty()
                    || capability.len() > 256
                    || !capability.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_.:-".contains(character)
                    })
            })
            || self.required_host_capabilities.len() > 1024
            || self.required_any_host_capabilities.len() > 1024
            || self
                .required_host_capabilities
                .iter()
                .chain(self.required_any_host_capabilities.iter())
                .any(|capability| {
                    capability.is_empty()
                        || capability.len() > 256
                        || !capability.chars().all(|character| {
                            character.is_ascii_alphanumeric() || "_.:-".contains(character)
                        })
                })
            || self.reason.is_empty()
            || self.reason.len() > 4096
            || self.reason.chars().any(char::is_control)
            || self.supported != (self.core_available && self.host_available)
            || (self.core_available && self.rejection_code.is_some())
            || (!self.core_available
                && !self
                    .rejection_code
                    .as_deref()
                    .is_some_and(|code| ERROR_CODES.contains(&code)))
            || (self.host_available && !self.missing_host_capabilities.is_empty())
            || (!self.host_available && self.missing_host_capabilities.is_empty());
        if invalid {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Operation preflight report is inconsistent or exceeds its bounds.",
                &self.operation_id,
            ));
        }
        let unique: std::collections::BTreeSet<_> = self.target_ids.iter().collect();
        if unique.len() != self.target_ids.len() {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Operation preflight targets must be unique.",
                &self.operation_id,
            ));
        }
        Ok(())
    }
}

const MAX_PREFLIGHT_TARGETS: usize = 1024;
type OperationFamilyDefinition = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static str],
);

/// Return the built-in operation families and explain any host requirements
/// that are not declared by `host`.
pub fn builtin_operation_support(
    host: &HostCapabilities,
) -> DockResult<Vec<BuiltinOperationSupport>> {
    host.validate()?;
    const FAMILIES: &[OperationFamilyDefinition] = &[
        (
            "layout_edit",
            &[
                "set_tab_pinned",
                "convert_container",
                "set_stack_expanded",
                "raise_layer",
                "lower_layer",
                "select_layer",
                "tab_panel",
                "center_panel",
                "center_panel_group",
                "swap_panels",
                "root_panel",
                "split_panel",
                "reorder_panel",
                "close_panel",
                "close_panel_group",
                "reopen_panel",
                "resize_split",
                "set_group_minimized",
                "auto_hide",
                "pin_panel",
                "maximize",
                "restore_maximized",
                "select_tab",
                "activate_document_group",
                "add_panel",
                "add_group",
                "clear_layout",
            ],
            &[],
            &[],
        ),
        (
            "floating_window_control",
            &[
                "float_panel",
                "float_group",
                "set_scratchpad",
                "update_floating_window",
                "activate_floating_window",
                "dock_floating_window",
                "close_floating_window",
            ],
            &[],
            &["native_viewport", "embedded_viewport"],
        ),
        (
            "floating_window_picture_in_picture",
            &["set_picture_in_picture"],
            &["picture_in_picture"],
            &[],
        ),
        (
            "floating_window_recovery",
            &["recover_floating_window"],
            &["native_viewport", "monitor_dpi_query"],
            &[],
        ),
        (
            "extension_payload_update",
            &["update_extension_payload"],
            &[],
            &[],
        ),
    ];
    let mut reports: Vec<_> = FAMILIES
        .iter()
        .map(|(id, names, required, required_any)| {
            let required_host_capabilities: Vec<_> =
                required.iter().map(|value| (*value).to_owned()).collect();
            let required_any_host_capabilities: Vec<_> = required_any
                .iter()
                .map(|value| (*value).to_owned())
                .collect();
            let missing_required: Vec<_> = required_host_capabilities
                .iter()
                .filter(|capability| !host.supported.contains(*capability))
                .cloned()
                .collect();
            let missing_alternative_group = !required_any_host_capabilities.is_empty()
                && !required_any_host_capabilities
                    .iter()
                    .any(|capability| host.supported.contains(capability));
            let mut missing_host_capabilities = missing_required.clone();
            if missing_alternative_group {
                missing_host_capabilities.extend(required_any_host_capabilities.iter().cloned());
            }
            missing_host_capabilities.sort();
            missing_host_capabilities.dedup();
            let host_available = missing_host_capabilities.is_empty();
            let (target_kinds, constraints): (&[&str], &[&str]) = match *id {
                "layout_edit" => (
                    &["panel", "container", "window", "workspace"],
                    &[
                        "Canonical layout validation must pass.",
                        "Panel close, reparent, docking-edge, and provider policies apply.",
                        "Every accepted change is committed through the revision-checked service.",
                    ],
                ),
                "floating_window_control" => (
                    &["panel", "group", "window"],
                    &[
                        "Panel descriptor must permit floating where a panel is moved.",
                        "The host must declare either native_viewport or embedded_viewport.",
                        "Close and dock-back operations preserve forbidden-close groups atomically.",
                    ],
                ),
                "floating_window_picture_in_picture" => (
                    &["window", "panel"],
                    &[
                        "The host must declare picture_in_picture.",
                        "The floating window must contain exactly one panel with bounded geometry.",
                        "The native adapter fixes size and raises the PiP viewport above ordinary windows.",
                    ],
                ),
                "floating_window_recovery" => (
                    &["window", "monitor"],
                    &[
                        "The host must declare native_viewport and monitor_dpi_query.",
                        "Recovery geometry and monitor identity are validated before commit.",
                        "Recovery preserves window content when host recreation fails.",
                    ],
                ),
                "extension_payload_update" => (
                    &["container"],
                    &[
                        "The extension container and payload must pass registered schema validation.",
                        "Payload updates are bounded and become ordinary undoable transactions.",
                    ],
                ),
                _ => unreachable!("static operation family has metadata"),
            };
            let reason = if host_available {
                "Core operation family is available; the host declares all required capabilities."
                    .to_owned()
            } else {
                let mut clauses = Vec::new();
                if !missing_required.is_empty() {
                    clauses.push(format!(
                        "all of these capabilities are required: {}",
                        missing_required.join(", ")
                    ));
                }
                if missing_alternative_group {
                    clauses.push(format!(
                        "at least one of these capabilities is required: {}",
                        required_any_host_capabilities.join(", ")
                    ));
                }
                format!(
                    "Core operation family is available, but the host lacks {}.",
                    clauses.join("; ")
                )
            };
            BuiltinOperationSupport {
                id: (*id).to_owned(),
                operation_names: names.iter().map(|value| (*value).to_owned()).collect(),
                required_host_capabilities,
                required_any_host_capabilities,
                missing_host_capabilities,
                core_available: true,
                host_available,
                applicable_target_kinds: target_kinds
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                constraints: constraints
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                reason,
            }
        })
        .collect();
    reports.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(reports)
}

/// Generate the strict JSON Schema for the public tagged `Operation` request.
/// This schema describes wire shape; document policy and host support are
/// still evaluated by [`preflight_operation`].
pub fn builtin_operation_json_schema() -> serde_json::Value {
    const OPERATIONS: &[(&str, &[(&str, bool)])] = &[
        ("set_tab_pinned", &[("panel", true), ("pinned", true)]),
        (
            "convert_container",
            &[("container", true), ("target", true)],
        ),
        (
            "set_stack_expanded",
            &[("container", true), ("child", true), ("expanded", true)],
        ),
        ("raise_layer", &[("container", true), ("child", true)]),
        ("lower_layer", &[("container", true), ("child", true)]),
        ("select_layer", &[("container", true), ("child", true)]),
        (
            "tab_panel",
            &[("panel", true), ("target_tabs", true), ("index", true)],
        ),
        ("center_panel", &[("panel", true), ("target_panel", true)]),
        (
            "center_panel_group",
            &[("panels", true), ("target_panel", true)],
        ),
        ("swap_panels", &[("first", true), ("second", true)]),
        ("root_panel", &[("panel", true)]),
        (
            "split_panel",
            &[
                ("panel", true),
                ("target", true),
                ("orientation", true),
                ("before", true),
                ("ratio", true),
            ],
        ),
        (
            "float_panel",
            &[
                ("panel", true),
                ("window", true),
                ("geometry", true),
                ("monitor", true),
                ("scale", true),
            ],
        ),
        (
            "float_group",
            &[
                ("group", true),
                ("window", true),
                ("geometry", true),
                ("monitor", true),
                ("scale", true),
            ],
        ),
        (
            "reorder_panel",
            &[("panel", true), ("container", true), ("index", true)],
        ),
        ("close_panel", &[("panel", true)]),
        ("close_panel_group", &[("panels", true)]),
        (
            "reopen_panel",
            &[("panel", true), ("target_tabs", true), ("index", true)],
        ),
        ("resize_split", &[("split", true), ("weights", true)]),
        (
            "set_group_minimized",
            &[("group", true), ("minimized", true)],
        ),
        ("auto_hide", &[("panel", true), ("edge", true)]),
        (
            "pin_panel",
            &[("panel", true), ("target_tabs", true), ("index", true)],
        ),
        ("maximize", &[("panel", true)]),
        ("restore_maximized", &[]),
        ("select_tab", &[("tabs", true), ("index", true)]),
        (
            "activate_document_group",
            &[("well", true), ("index", true)],
        ),
        (
            "update_extension_payload",
            &[("container", true), ("payload", true)],
        ),
        ("add_panel", &[("descriptor", true), ("target", false)]),
        (
            "add_group",
            &[
                ("descriptor", true),
                ("target", false),
                ("orientation", true),
                ("before", true),
                ("ratio", true),
            ],
        ),
        (
            "update_floating_window",
            &[("window", true), ("geometry", true)],
        ),
        ("set_scratchpad", &[("window", true), ("config", true)]),
        (
            "set_picture_in_picture",
            &[("window", true), ("enabled", true)],
        ),
        ("activate_floating_window", &[("window", true)]),
        (
            "recover_floating_window",
            &[
                ("window", true),
                ("geometry", true),
                ("monitor_id", true),
                ("scale", true),
            ],
        ),
        (
            "dock_floating_window",
            &[("window", true), ("target_panel", false)],
        ),
        ("close_floating_window", &[("window", true)]),
        ("clear_layout", &[]),
    ];
    let mut branches = Vec::with_capacity(OPERATIONS.len());
    for (operation, fields) in OPERATIONS {
        let mut properties = serde_json::Map::from_iter([(
            "operation".into(),
            serde_json::json!({"const": operation}),
        )]);
        let mut required = vec![serde_json::Value::String("operation".into())];
        if !fields.is_empty() {
            let mut argument_properties = serde_json::Map::new();
            let mut argument_required = Vec::new();
            for (field, is_required) in *fields {
                argument_properties
                    .insert((*field).into(), operation_field_schema(operation, field));
                if *is_required {
                    argument_required.push((*field).to_owned());
                }
            }
            properties.insert(
                "arguments".into(),
                serde_json::json!({
                    "type": "object",
                    "properties": argument_properties,
                    "required": argument_required,
                    "additionalProperties": false,
                }),
            );
            required.push(serde_json::Value::String("arguments".into()));
        }
        branches.push(serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        }));
    }
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "DoofusDock Operation",
        "description": "Strict wire-shape schema. Use preflight_operation for document and host support.",
        "oneOf": branches,
        "$defs": operation_schema_definitions(),
    })
}

/// Return every built-in operation with its strict wire schema and the
/// host-capability explanation for its support family. The returned list is
/// stable by operation name and contains no document-specific preflight data.
pub fn builtin_operation_catalog(
    host: &HostCapabilities,
) -> DockResult<Vec<BuiltinOperationDescriptor>> {
    let support = builtin_operation_support(host)?;
    let schema = builtin_operation_json_schema();
    let branches = schema["oneOf"].as_array().ok_or_else(|| {
        DockError::new(
            ErrorCode::InvalidLayout,
            "Built-in operation schema is missing its branch list.",
            "builtin_operation_catalog",
        )
    })?;
    let mut request_schemas = std::collections::BTreeMap::new();
    for branch in branches {
        let name = branch["properties"]["operation"]
            .get("const")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::InvalidLayout,
                    "Built-in operation schema contains an unidentified branch.",
                    "builtin_operation_catalog",
                )
            })?;
        if request_schemas
            .insert(name.to_owned(), branch.clone())
            .is_some()
        {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Built-in operation schema contains a duplicate operation.",
                name,
            ));
        }
    }

    let mut descriptors = Vec::with_capacity(request_schemas.len());
    for family in support {
        for name in family.operation_names {
            let request_schema = request_schemas.remove(&name).ok_or_else(|| {
                DockError::new(
                    ErrorCode::InvalidLayout,
                    "Built-in operation support metadata has no request schema.",
                    &name,
                )
            })?;
            descriptors.push(BuiltinOperationDescriptor {
                name,
                family: family.id.clone(),
                request_schema,
                required_host_capabilities: family.required_host_capabilities.clone(),
                required_any_host_capabilities: family.required_any_host_capabilities.clone(),
                missing_host_capabilities: family.missing_host_capabilities.clone(),
                core_available: family.core_available,
                host_available: family.host_available,
                applicable_target_kinds: family.applicable_target_kinds.clone(),
                constraints: family.constraints.clone(),
                reason: family.reason.clone(),
            });
        }
    }
    if let Some(unmatched) = request_schemas.keys().next() {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Built-in operation schema has no support family.",
            unmatched,
        ));
    }
    descriptors.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(descriptors)
}

fn operation_field_schema(operation: &str, field: &str) -> serde_json::Value {
    use serde_json::json;
    match field {
        "descriptor" => json!({"$ref": "#/$defs/PanelDescriptor"}),
        "geometry" => json!({"$ref": "#/$defs/Rect"}),
        "config" if operation == "set_scratchpad" => json!({
            "anyOf": [
                {"$ref": "#/$defs/ScratchpadConfig"},
                {"type": "null"}
            ]
        }),
        "target" if operation == "convert_container" => {
            json!({"$ref": "#/$defs/ContainerModality"})
        }
        "target" if matches!(operation, "add_panel" | "add_group") => {
            json!({"type": ["string", "null"], "minLength": 1})
        }
        "target_panel" if operation == "dock_floating_window" => {
            json!({"type": ["string", "null"], "minLength": 1})
        }
        "panel" | "first" | "second" | "target_tabs" | "target_panel" | "container" | "child"
        | "target" | "group" | "window" | "split" | "tabs" | "well" | "monitor" | "monitor_id" => {
            json!({"type": "string", "minLength": 1, "maxLength": 4096})
        }
        "panels" => json!({
            "type": "array", "minItems": 1,
            "maxItems": if operation == "center_panel_group" { MAX_PREFLIGHT_TARGETS - 1 } else { MAX_PREFLIGHT_TARGETS },
            "items": {"type": "string", "minLength": 1, "maxLength": 4096}
        }),
        "pinned" | "expanded" | "before" | "enabled" | "minimized" => {
            json!({"type": "boolean"})
        }
        "orientation" => json!({"$ref": "#/$defs/Orientation"}),
        "ratio" => json!({
            "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1
        }),
        "scale" => json!({"type": "number", "exclusiveMinimum": 0}),
        "index" => json!({"type": "integer", "minimum": 0}),
        "edge" => json!({"$ref": "#/$defs/DockEdge"}),
        "weights" => json!({
            "type": "array", "minItems": 2, "maxItems": 256,
            "items": {"type": "number", "exclusiveMinimum": 0}
        }),
        "payload" => json!({"type": "string", "maxLength": 4096}),
        _ => json!({}),
    }
}

fn operation_schema_definitions() -> serde_json::Value {
    serde_json::json!({
        "Orientation": {"type": "string", "enum": ["horizontal", "vertical"]},
        "DockEdge": {"type": "integer", "enum": [0, 1, 2, 3]},
        "Rect": {
            "type": "object",
            "properties": {
                "x": {"type": "integer"}, "y": {"type": "integer"},
                "width": {"type": "integer"}, "height": {"type": "integer"}
            },
            "required": ["x", "y", "width", "height"],
            "additionalProperties": true
        },
        "Point": {
            "type": "object",
            "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}},
            "required": ["x", "y"],
            "additionalProperties": true
        },
        "ScratchpadConfig": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 64, "pattern": "^[A-Za-z0-9_.-]+$"},
                "visible": {"type": "boolean"},
                "summon_position": {"anyOf": [{"$ref": "#/$defs/Point"}, {"type": "null"}]},
                "auto_hide_on_deactivate": {"type": "boolean"},
                "pinned_across_workspaces": {"type": "boolean"}
            },
            "required": ["name"],
            "additionalProperties": true
        },
        "SizeArray": {
            "type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2
        },
        "DockingCapabilities": {
            "type": "object",
            "properties": {
                "allow_tab": {"type": "boolean"}, "allow_split": {"type": "boolean"},
                "allow_float": {"type": "boolean"}, "allow_auto_hide": {"type": "boolean"},
                "allowed_edges": {"type": "array", "items": {"$ref": "#/$defs/DockEdge"}}
            },
            "additionalProperties": true
        },
        "PanelDescriptor": {
            "type": "object",
            "properties": {
                "id": {"type": "string", "minLength": 1},
                "title_key": {"type": "string"}, "icon_key": {"type": "string"},
                "min": {"$ref": "#/$defs/SizeArray"}, "max": {"$ref": "#/$defs/SizeArray"},
                "close_policy": {"type": "integer", "enum": [0, 1, 2]},
                "persistence": {"type": "integer", "enum": [0, 1, 2]},
                "provider": {"type": "string"},
                "reparent": {"type": "integer", "enum": [0, 1, 2]},
                "docking": {"$ref": "#/$defs/DockingCapabilities"}
            },
            "required": ["id", "title_key", "icon_key", "min", "max", "provider"],
            "additionalProperties": true
        },
        "ContainerModality": {
            "oneOf": [
                {"type": "object", "properties": {"kind": {"const": "tabbed"}, "configuration": {"type": "object", "properties": {"active_index": {"type": "integer", "minimum": 0}}, "required": ["active_index"], "additionalProperties": false}}, "required": ["kind", "configuration"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "split"}, "configuration": {"type": "object", "properties": {"orientation": {"$ref": "#/$defs/Orientation"}, "weights": {"type": "array", "items": {"type": "number", "exclusiveMinimum": 0}}}, "required": ["orientation", "weights"], "additionalProperties": false}}, "required": ["kind", "configuration"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "accordion"}, "configuration": {"type": "object", "properties": {"multiple": {"type": "boolean"}}, "required": ["multiple"], "additionalProperties": false}}, "required": ["kind", "configuration"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "shade_stack"}}, "required": ["kind"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "grid"}, "configuration": {"type": "object", "properties": {"columns": {"type": "integer", "minimum": 1, "maximum": 64}, "gap": {"type": "integer", "minimum": 0, "maximum": 256}}, "required": ["columns", "gap"], "additionalProperties": false}}, "required": ["kind", "configuration"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "spatial"}}, "required": ["kind"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "node_graph"}}, "required": ["kind"], "additionalProperties": false},
                {"type": "object", "properties": {"kind": {"const": "layered"}}, "required": ["kind"], "additionalProperties": false}
            ]
        }
    })
}

/// Check a proposed operation against the canonical document and host without
/// applying it. Core validation uses the same operation path as execution;
/// native availability is assessed from declared host capability IDs.
pub fn preflight_operation(
    document: &LayoutDocument,
    operation: &Operation,
    host: &HostCapabilities,
) -> DockResult<OperationPreflightReport> {
    host.validate()?;
    let operation_id = operation.id().to_owned();
    let target_ids = operation_target_ids(operation)?;
    let family = builtin_operation_support(host)?
        .into_iter()
        .find(|family| {
            family
                .operation_names
                .iter()
                .any(|name| name == &operation_id)
        })
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Built-in operation is missing from the support catalog.",
                &operation_id,
            )
        })?;
    let check = apply_operation(document, operation.clone());
    let (core_available, rejection_code, core_reason) = match check {
        Ok(_) => (true, None, None),
        Err(error) => (
            false,
            Some(error_code_name(error.code).to_owned()),
            Some(error.message),
        ),
    };
    let host_available = family.missing_host_capabilities.is_empty();
    let reason = if let Some(reason) = core_reason {
        reason
    } else if !host_available {
        family.reason.clone()
    } else {
        "Operation passes core validation and the host declares its required capabilities."
            .to_owned()
    };
    Ok(OperationPreflightReport {
        document_id: document.id.0.clone(),
        source_revision: document.revision,
        operation_id,
        target_ids,
        core_available,
        host_available,
        supported: core_available && host_available,
        required_host_capabilities: family.required_host_capabilities,
        required_any_host_capabilities: family.required_any_host_capabilities,
        missing_host_capabilities: family.missing_host_capabilities,
        rejection_code,
        reason,
    })
}

fn operation_name(operation: &Operation) -> &'static str {
    match operation {
        Operation::SetTabPinned { .. } => "set_tab_pinned",
        Operation::SetPanelWorkspaceMemberships { .. } => "set_panel_workspace_memberships",
        Operation::ConvertContainer { .. } => "convert_container",
        Operation::SetStackExpanded { .. } => "set_stack_expanded",
        Operation::RaiseLayer { .. } => "raise_layer",
        Operation::LowerLayer { .. } => "lower_layer",
        Operation::SelectLayer { .. } => "select_layer",
        Operation::TabPanel { .. } => "tab_panel",
        Operation::CenterPanel { .. } => "center_panel",
        Operation::CenterPanelGroup { .. } => "center_panel_group",
        Operation::SwapPanels { .. } => "swap_panels",
        Operation::RootPanel { .. } => "root_panel",
        Operation::SplitPanel { .. } => "split_panel",
        Operation::FloatPanel { .. } => "float_panel",
        Operation::FloatGroup { .. } => "float_group",
        Operation::ReorderPanel { .. } => "reorder_panel",
        Operation::ClosePanel { .. } => "close_panel",
        Operation::ClosePanelGroup { .. } => "close_panel_group",
        Operation::ReopenPanel { .. } => "reopen_panel",
        Operation::ResizeSplit { .. } => "resize_split",
        Operation::SetGroupMinimized { .. } => "set_group_minimized",
        Operation::AutoHide { .. } => "auto_hide",
        Operation::PinPanel { .. } => "pin_panel",
        Operation::Maximize { .. } => "maximize",
        Operation::RestoreMaximized => "restore_maximized",
        Operation::SelectTab { .. } => "select_tab",
        Operation::ActivateDocumentGroup { .. } => "activate_document_group",
        Operation::UpdateExtensionPayload { .. } => "update_extension_payload",
        Operation::AddPanel { .. } => "add_panel",
        Operation::AddGroup { .. } => "add_group",
        Operation::UpdateFloatingWindow { .. } => "update_floating_window",
        Operation::SetScratchpad { .. } => "set_scratchpad",
        Operation::SetPictureInPicture { .. } => "set_picture_in_picture",
        Operation::ActivateFloatingWindow { .. } => "activate_floating_window",
        Operation::RecoverFloatingWindow { .. } => "recover_floating_window",
        Operation::DockFloatingWindow { .. } => "dock_floating_window",
        Operation::CloseFloatingWindow { .. } => "close_floating_window",
        Operation::ClearLayout => "clear_layout",
    }
}

fn operation_target_ids(operation: &Operation) -> DockResult<Vec<String>> {
    let mut targets = match operation {
        Operation::SetTabPinned { panel, .. }
        | Operation::SetPanelWorkspaceMemberships { panel, .. }
        | Operation::RootPanel { panel }
        | Operation::ClosePanel { panel }
        | Operation::AutoHide { panel, .. }
        | Operation::Maximize { panel } => vec![format!("panel:{}", panel.0)],
        Operation::TabPanel {
            panel, target_tabs, ..
        }
        | Operation::ReopenPanel {
            panel, target_tabs, ..
        }
        | Operation::PinPanel {
            panel, target_tabs, ..
        } => vec![
            format!("panel:{}", panel.0),
            format!("container:{}", target_tabs.0),
        ],
        Operation::CenterPanel {
            panel,
            target_panel,
        } => vec![
            format!("panel:{}", panel.0),
            format!("panel:{}", target_panel.0),
        ],
        Operation::CenterPanelGroup {
            panels,
            target_panel,
        } => {
            let mut targets: Vec<_> = panels
                .iter()
                .map(|panel| format!("panel:{}", panel.0))
                .collect();
            targets.push(format!("panel:{}", target_panel.0));
            targets
        }
        Operation::SwapPanels { first, second } => {
            vec![format!("panel:{}", first.0), format!("panel:{}", second.0)]
        }
        Operation::SplitPanel { panel, target, .. } => {
            vec![format!("panel:{}", panel.0), format!("node:{}", target.0)]
        }
        Operation::FloatPanel { panel, window, .. } => {
            vec![format!("panel:{}", panel.0), format!("window:{}", window.0)]
        }
        Operation::ReorderPanel {
            panel, container, ..
        } => vec![
            format!("panel:{}", panel.0),
            format!("container:{}", container.0),
        ],
        Operation::ConvertContainer { container, .. }
        | Operation::UpdateExtensionPayload { container, .. } => {
            vec![format!("container:{}", container.0)]
        }
        Operation::SetStackExpanded {
            container, child, ..
        }
        | Operation::RaiseLayer { container, child }
        | Operation::LowerLayer { container, child }
        | Operation::SelectLayer { container, child } => vec![
            format!("container:{}", container.0),
            format!("node:{}", child.0),
        ],
        Operation::SetGroupMinimized { group, .. } => vec![format!("group:{}", group.0)],
        Operation::FloatGroup { group, window, .. } => {
            vec![format!("group:{}", group.0), format!("window:{}", window.0)]
        }
        Operation::ClosePanelGroup { panels } => panels
            .iter()
            .map(|panel| format!("panel:{}", panel.0))
            .collect(),
        Operation::ResizeSplit { split, .. } => vec![format!("container:{}", split.0)],
        Operation::SelectTab { tabs, .. } => vec![format!("container:{}", tabs.0)],
        Operation::ActivateDocumentGroup { well, .. } => vec![format!("container:{}", well.0)],
        Operation::AddPanel { descriptor, target } => {
            let mut values = vec![format!("panel:{}", descriptor.id.0)];
            values.extend(
                target
                    .iter()
                    .map(|target| format!("container:{}", target.0)),
            );
            values
        }
        Operation::AddGroup {
            descriptor, target, ..
        } => {
            let mut values = vec![format!("group:{}", descriptor.id.0)];
            values.extend(
                target
                    .iter()
                    .map(|target| format!("container:{}", target.0)),
            );
            values
        }
        Operation::UpdateFloatingWindow { window, .. }
        | Operation::SetScratchpad { window, .. }
        | Operation::SetPictureInPicture { window, .. }
        | Operation::ActivateFloatingWindow { window }
        | Operation::RecoverFloatingWindow { window, .. }
        | Operation::CloseFloatingWindow { window } => vec![format!("window:{}", window.0)],
        Operation::DockFloatingWindow {
            window,
            target_panel,
        } => {
            let mut values = vec![format!("window:{}", window.0)];
            values.extend(
                target_panel
                    .iter()
                    .map(|panel| format!("panel:{}", panel.0)),
            );
            values
        }
        Operation::RestoreMaximized | Operation::ClearLayout => Vec::new(),
    };
    if targets.len() > MAX_PREFLIGHT_TARGETS {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Operation preflight target list exceeds the supported limit.",
            format!("count={} limit={MAX_PREFLIGHT_TARGETS}", targets.len()),
        ));
    }
    if let Some(target) = targets
        .iter()
        .find(|target| target.len() > 4096 || target.chars().any(char::is_control))
    {
        return Err(DockError::new(
            ErrorCode::InvalidId,
            "Operation preflight target IDs must be bounded and printable.",
            target,
        ));
    }
    targets.sort();
    targets.dedup();
    Ok(targets)
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "arguments",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Operation {
    SetTabPinned {
        panel: PanelId,
        pinned: bool,
    },
    SetPanelWorkspaceMemberships {
        panel: PanelId,
        tags: BTreeSet<String>,
    },
    ConvertContainer {
        container: NodeId,
        target: ContainerModality,
    },
    SetStackExpanded {
        container: NodeId,
        child: NodeId,
        expanded: bool,
    },
    RaiseLayer {
        container: NodeId,
        child: NodeId,
    },
    LowerLayer {
        container: NodeId,
        child: NodeId,
    },
    SelectLayer {
        container: NodeId,
        child: NodeId,
    },
    TabPanel {
        panel: PanelId,
        target_tabs: NodeId,
        index: usize,
    },
    CenterPanel {
        panel: PanelId,
        target_panel: PanelId,
    },
    /// Move an ordered, unique selection into a tab group in one transaction.
    CenterPanelGroup {
        panels: Vec<PanelId>,
        target_panel: PanelId,
    },
    /// Exchange the panel identities at two active layout leaves atomically.
    SwapPanels {
        first: PanelId,
        second: PanelId,
    },
    RootPanel {
        panel: PanelId,
    },
    SplitPanel {
        panel: PanelId,
        target: NodeId,
        orientation: Orientation,
        before: bool,
        ratio: f64,
    },
    FloatPanel {
        panel: PanelId,
        window: WindowId,
        geometry: Rect,
        monitor: String,
        scale: f64,
    },
    FloatGroup {
        group: NodeId,
        window: WindowId,
        geometry: Rect,
        monitor: String,
        scale: f64,
    },
    ReorderPanel {
        panel: PanelId,
        container: NodeId,
        index: usize,
    },
    ClosePanel {
        panel: PanelId,
    },
    /// Atomically park all supplied panels in the reopenable closed set.
    /// Used for group-close UI so one forbidden member cannot partially close a group.
    ClosePanelGroup {
        panels: Vec<PanelId>,
    },
    ReopenPanel {
        panel: PanelId,
        target_tabs: NodeId,
        index: usize,
    },
    ResizeSplit {
        split: NodeId,
        weights: Vec<f64>,
    },
    SetGroupMinimized {
        group: NodeId,
        minimized: bool,
    },
    AutoHide {
        panel: PanelId,
        edge: DockEdge,
    },
    PinPanel {
        panel: PanelId,
        target_tabs: NodeId,
        index: usize,
    },
    Maximize {
        panel: PanelId,
    },
    RestoreMaximized,
    SelectTab {
        tabs: NodeId,
        index: usize,
    },
    ActivateDocumentGroup {
        well: NodeId,
        index: usize,
    },
    UpdateExtensionPayload {
        container: NodeId,
        payload: String,
    },
    AddPanel {
        descriptor: PanelDescriptor,
        target: Option<NodeId>,
    },
    AddGroup {
        descriptor: PanelDescriptor,
        target: Option<NodeId>,
        orientation: Orientation,
        before: bool,
        ratio: f64,
    },
    UpdateFloatingWindow {
        window: WindowId,
        geometry: Rect,
    },
    SetScratchpad {
        window: WindowId,
        config: Option<ScratchpadConfig>,
    },
    SetPictureInPicture {
        window: WindowId,
        enabled: bool,
    },
    ActivateFloatingWindow {
        window: WindowId,
    },
    RecoverFloatingWindow {
        window: WindowId,
        geometry: Rect,
        monitor_id: String,
        scale: f64,
    },
    DockFloatingWindow {
        window: WindowId,
        target_panel: Option<PanelId>,
    },
    /// Close a floating window and park its panels in the reopenable closed set.
    /// The entire window is rejected atomically if any member is non-closable.
    CloseFloatingWindow {
        window: WindowId,
    },
    ClearLayout,
}

impl Operation {
    /// Stable snake-case operation discriminator used by capability reports
    /// and the internally tagged Serde request representation.
    pub fn id(&self) -> &'static str {
        operation_name(self)
    }
}

fn descriptor<'a>(doc: &'a LayoutDocument, p: &PanelId) -> DockResult<&'a PanelDescriptor> {
    doc.descriptor(p).ok_or_else(|| {
        DockError::new(
            ErrorCode::NotFound,
            "Panel descriptor was not registered.",
            p.0.clone(),
        )
    })
}
fn edge_allowed(d: &PanelDescriptor, e: DockEdge, split: bool) -> bool {
    (if split {
        d.docking.allow_split
    } else {
        d.docking.allow_auto_hide
    }) && d.docking.allowed_edges.contains(&e)
}
fn walk_mut(n: &mut NodePtr, id: &NodeId, f: &mut impl FnMut(&mut Node) -> bool) -> bool {
    let node = std::sync::Arc::make_mut(n);
    if &node.id == id {
        return f(node);
    }
    match &mut node.kind {
        NodeKind::Tabs { items, .. } => items.iter_mut().any(|c| walk_mut(c, id, f)),
        NodeKind::DocumentWell { groups, .. } => groups.iter_mut().any(|c| walk_mut(c, id, f)),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter_mut().any(|c| walk_mut(c, id, f))
        }
        _ => false,
    }
}
fn replace(doc: &mut LayoutDocument, id: &NodeId, mut f: impl FnMut(&mut Node) -> bool) -> bool {
    if let Some(n) = doc.root.as_mut()
        && walk_mut(n, id, &mut f)
    {
        return true;
    }
    for w in &mut doc.floating_windows {
        if walk_mut(&mut w.root, id, &mut f) {
            return true;
        }
    }
    false
}
fn extract(n: NodePtr, p: &PanelId) -> (Option<NodePtr>, Option<NodePtr>) {
    match &n.kind {
        NodeKind::Panel { panel } if panel == p => (None, Some(n)),
        NodeKind::Tabs {
            active_index,
            items,
            minimized,
        } => {
            let mut kept = Vec::with_capacity(items.len());
            let mut taken = None;
            let mut active = *active_index;
            for (i, child) in items.iter().cloned().enumerate() {
                if taken.is_some() {
                    kept.push(child);
                    continue;
                }
                let (next, got) = extract(child, p);
                if let Some(got) = got {
                    taken = Some(got);
                    if i < active {
                        active -= 1
                    } else if i == active {
                        active = active.min(kept.len())
                    }
                }
                if let Some(next) = next {
                    kept.push(next)
                }
            }
            if taken.is_none() {
                return (Some(n), None);
            }
            if kept.is_empty() {
                (None, taken)
            } else if kept.len() == 1 {
                (kept.pop(), taken)
            } else {
                active = active.min(kept.len() - 1);
                (
                    Some(Node::tabs_with_minimized(
                        n.id.clone(),
                        kept,
                        active,
                        *minimized,
                    )),
                    taken,
                )
            }
        }
        NodeKind::Split {
            orientation,
            weights,
            children,
        } => {
            let mut kept = Vec::with_capacity(children.len());
            let mut ws = Vec::with_capacity(weights.len());
            let mut taken = None;
            for (child, weight) in children.iter().cloned().zip(weights.iter().copied()) {
                if taken.is_some() {
                    kept.push(child);
                    ws.push(weight);
                    continue;
                }
                let (next, got) = extract(child, p);
                if got.is_some() {
                    taken = got
                }
                if let Some(next) = next {
                    kept.push(next);
                    ws.push(weight)
                }
            }
            if taken.is_none() {
                return (Some(n), None);
            }
            if kept.is_empty() {
                (None, taken)
            } else if kept.len() == 1 {
                (kept.pop(), taken)
            } else {
                let total: f64 = ws.iter().sum();
                if total > 0.0 {
                    ws.iter_mut().for_each(|w| *w /= total)
                }
                (
                    Some(Node::split(n.id.clone(), *orientation, kept, ws)),
                    taken,
                )
            }
        }
        NodeKind::DocumentWell {
            active_group,
            groups,
        } => {
            let mut kept = Vec::with_capacity(groups.len());
            let mut active = *active_group;
            let mut taken = None;
            for (i, child) in groups.iter().cloned().enumerate() {
                if taken.is_some() {
                    kept.push(child);
                    continue;
                }
                let (next, got) = extract(child, p);
                if got.is_some() {
                    taken = got
                }
                if let Some(mut next) = next {
                    if !matches!(next.kind, NodeKind::Tabs { .. }) {
                        next = Node::tabs(next.id.clone(), vec![next], 0)
                    }
                    kept.push(next)
                } else if i < active {
                    active -= 1
                }
            }
            if taken.is_none() {
                return (Some(n), None);
            }
            if kept.is_empty() {
                (None, taken)
            } else {
                active = active.min(kept.len() - 1);
                (
                    Some(std::sync::Arc::new(Node {
                        id: n.id.clone(),
                        kind: NodeKind::DocumentWell {
                            active_group: active,
                            groups: kept,
                        },
                    })),
                    taken,
                )
            }
        }
        NodeKind::Extension {
            modality,
            payload,
            children,
        } => {
            let mut kept = Vec::with_capacity(children.len());
            let mut taken = None;
            for child in children.iter().cloned() {
                if taken.is_some() {
                    kept.push(child);
                    continue;
                }
                let (next, got) = extract(child, p);
                if got.is_some() {
                    taken = got
                }
                if let Some(next) = next {
                    kept.push(next)
                }
            }
            if taken.is_none() {
                return (Some(n), None);
            }
            let mut next_payload = payload.clone();
            if is_stack_modality(modality) {
                if kept.is_empty() {
                    return (None, taken);
                }
                if let Ok(mut config) = StackConfiguration::parse(payload, children) {
                    config
                        .expanded
                        .retain(|id| kept.iter().any(|c| &c.id == id));
                    if config
                        .selected
                        .as_ref()
                        .is_some_and(|id| !kept.iter().any(|c| &c.id == id))
                    {
                        config.selected = kept.first().map(|c| c.id.clone());
                    }
                    if let Ok(payload) = config.to_payload() {
                        next_payload = payload;
                    }
                }
            }
            if modality.0 == LAYERED_MODALITY {
                if kept.is_empty() {
                    return (None, taken);
                }
                if let Ok(mut config) = LayeredConfiguration::parse(payload, children) {
                    config
                        .back_to_front
                        .retain(|id| kept.iter().any(|c| &c.id == id));
                    if !kept.iter().any(|child| child.id == config.active)
                        && let Some(top) = config.back_to_front.last()
                    {
                        config.active = top.clone();
                    }
                    if let Ok(payload) = config.to_payload() {
                        next_payload = payload;
                    }
                }
            }
            (
                Some(std::sync::Arc::new(Node {
                    id: n.id.clone(),
                    kind: NodeKind::Extension {
                        modality: modality.clone(),
                        payload: next_payload,
                        children: kept,
                    },
                })),
                taken,
            )
        }
        _ => (Some(n), None),
    }
}
fn take_panel(doc: &mut LayoutDocument, p: &PanelId) -> Option<NodePtr> {
    if let Some(root) = doc.root.take() {
        let (next, taken) = extract(root, p);
        doc.root = next;
        if taken.is_some() {
            return taken;
        }
    }
    let mut i = 0;
    while i < doc.floating_windows.len() {
        let root = doc.floating_windows[i].root.clone();
        let (next, taken) = extract(root, p);
        if let Some(next) = next {
            doc.floating_windows[i].root = next
        } else {
            doc.floating_windows.remove(i);
        }
        if taken.is_some() {
            return taken;
        }
        i += 1;
    }
    None
}
fn take_for_move(doc: &mut LayoutDocument, p: &PanelId) -> Option<NodePtr> {
    if let Some(node) = take_panel(doc, p) {
        return Some(node);
    }
    let i = doc.auto_hide_panels.iter().position(|d| &d.panel == p)?;
    doc.auto_hide_panels.remove(i);
    Some(Node::panel(format!("drawer-{}", p.0), p.clone()))
}
fn live(doc: &LayoutDocument, p: &PanelId) -> bool {
    find_panel_container(doc, p).is_some()
}
fn insert_node(
    n: &mut NodePtr,
    target: &NodeId,
    insert: Option<NodePtr>,
    f: &mut impl FnMut(&mut Node, Option<NodePtr>) -> bool,
) -> bool {
    let node = std::sync::Arc::make_mut(n);
    if &node.id == target {
        return f(node, insert);
    }
    match &mut node.kind {
        NodeKind::Tabs { items, .. } => items
            .iter_mut()
            .any(|c| insert_node(c, target, insert.clone(), f)),
        NodeKind::DocumentWell { groups, .. } => groups
            .iter_mut()
            .any(|c| insert_node(c, target, insert.clone(), f)),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => children
            .iter_mut()
            .any(|c| insert_node(c, target, insert.clone(), f)),
        _ => false,
    }
}
fn insert_at(
    doc: &mut LayoutDocument,
    target: &NodeId,
    node: NodePtr,
    mut f: impl FnMut(&mut Node, NodePtr) -> bool,
) -> bool {
    let insert = Some(node);
    if let Some(root) = doc.root.as_mut()
        && insert_node(root, target, insert.clone(), &mut |n, x| {
            x.is_some_and(|x| f(n, x))
        })
    {
        return true;
    }
    for w in &mut doc.floating_windows {
        if insert_node(&mut w.root, target, insert.clone(), &mut |n, x| {
            x.is_some_and(|x| f(n, x))
        }) {
            return true;
        }
    }
    false
}
fn finish(
    before: &LayoutDocument,
    mut after: LayoutDocument,
    op: &str,
) -> DockResult<TransactionResult> {
    validate_layout(&after)?;
    let diff = diff_layouts(before, &after);
    if !diff.is_empty() {
        after.revision = before.revision.checked_add(1).ok_or_else(|| {
            DockError::new(
                ErrorCode::ConstraintViolation,
                "Layout revision overflow.",
                before.id.0.clone(),
            )
        })?
    }
    Ok(TransactionResult {
        document: after,
        diff,
        operation_id: if op.is_empty() {
            "noop".into()
        } else {
            op.into()
        },
    })
}
fn permitted_move<'a>(doc: &'a LayoutDocument, p: &PanelId) -> DockResult<&'a PanelDescriptor> {
    let d = descriptor(doc, p)?;
    if !live(doc, p) && !doc.auto_hide_panels.iter().any(|x| &x.panel == p) {
        return Err(DockError::new(
            ErrorCode::NotFound,
            "Panel is not in the active layout or auto-hide drawer.",
            p.0.clone(),
        ));
    }
    Ok(d)
}
fn find_tab_member(node: &Node, panel: &PanelId) -> Option<(NodeId, usize)> {
    match &node.kind {
        NodeKind::Tabs { items, .. } => {
            for (index, child) in items.iter().enumerate() {
                if matches!(&child.kind, NodeKind::Panel { panel: id } if id == panel) {
                    return Some((node.id.clone(), index));
                }
                if let Some(found) = find_tab_member(child, panel) {
                    return Some(found);
                }
            }
            None
        }
        NodeKind::DocumentWell { groups, .. } => {
            groups.iter().find_map(|n| find_tab_member(n, panel))
        }
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter().find_map(|n| find_tab_member(n, panel))
        }
        _ => None,
    }
}

fn find_panel_leaf_in_document<'a>(doc: &'a LayoutDocument, panel: &PanelId) -> Option<&'a Node> {
    doc.root
        .as_deref()
        .and_then(|root| find_panel_leaf(root, panel))
        .or_else(|| {
            doc.floating_windows
                .iter()
                .find_map(|window| find_panel_leaf(&window.root, panel))
        })
}

fn find_tab_member_in_document(doc: &LayoutDocument, panel: &PanelId) -> Option<(NodeId, usize)> {
    doc.root
        .as_deref()
        .and_then(|root| find_tab_member(root, panel))
        .or_else(|| {
            doc.floating_windows
                .iter()
                .find_map(|window| find_tab_member(&window.root, panel))
        })
}

#[derive(Clone, Copy)]
enum LayerAction {
    Raise,
    Lower,
    Select,
}

fn set_layer_state(
    doc: &LayoutDocument,
    container: NodeId,
    child: NodeId,
    action: LayerAction,
) -> DockResult<TransactionResult> {
    let node = all_find(doc, &container).ok_or_else(|| {
        DockError::new(
            ErrorCode::NotFound,
            "Layered container not found.",
            &container.0,
        )
    })?;
    let NodeKind::Extension {
        modality,
        payload,
        children,
    } = &node.kind
    else {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Expected layered container.",
            &container.0,
        ));
    };
    if modality.0 != LAYERED_MODALITY {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Expected layered container.",
            &container.0,
        ));
    }
    let mut config = LayeredConfiguration::parse(payload, children)?;
    if !children.iter().any(|node| node.id == child) {
        return Err(DockError::new(
            ErrorCode::NotFound,
            "Layered child not found.",
            &child.0,
        ));
    }
    match action {
        LayerAction::Raise => config.raise(&child)?,
        LayerAction::Lower => config.lower(&child)?,
        LayerAction::Select => config.raise(&child)?,
    }
    apply_operation(
        doc,
        Operation::UpdateExtensionPayload {
            container,
            payload: config.to_payload()?,
        },
    )
}

pub fn apply_operation(doc: &LayoutDocument, op: Operation) -> DockResult<TransactionResult> {
    match op {
        Operation::SetTabPinned { panel, pinned } => {
            descriptor(doc, &panel)?;
            let mut next = doc.clone();
            if pinned {
                next.pinned_panels.insert(panel);
            } else {
                next.pinned_panels.remove(&panel);
            }
            finish(doc, next, "set-tab-pinned")
        }
        Operation::SetPanelWorkspaceMemberships { panel, tags } => {
            descriptor(doc, &panel)?;
            if tags.len() > 64
                || tags.iter().any(|tag| {
                    tag.is_empty()
                        || tag.len() > 64
                        || !tag.chars().all(|character| {
                            character.is_ascii_alphanumeric() || "_.:-".contains(character)
                        })
                })
            {
                return Err(DockError::new(
                    ErrorCode::SchemaError,
                    "Panel workspace tags are invalid or exceed their bound.",
                    panel.0,
                ));
            }
            let mut next = doc.clone();
            let target = std::sync::Arc::make_mut(&mut next.panels)
                .iter_mut()
                .find(|descriptor| descriptor.id == panel)
                .expect("validated panel descriptor exists");
            target.workspace_memberships = tags;
            finish(doc, next, "set-panel-workspace-memberships")
        }
        Operation::ConvertContainer { container, target } => {
            let node = all_find(doc, &container).ok_or_else(|| {
                DockError::new(ErrorCode::NotFound, "Container not found.", &container.0)
            })?;
            let kind = crate::modality::converted_container(node, &target)?;
            let mut next = doc.clone();
            replace(&mut next, &container, |node| {
                node.kind = kind.clone();
                true
            });
            validate_layout_replacement(doc, &next)?;
            finish(doc, next, "convert-container")
        }
        Operation::SetStackExpanded {
            container,
            child,
            expanded,
        } => {
            let node = all_find(doc, &container).ok_or_else(|| {
                DockError::new(ErrorCode::NotFound, "Stack not found.", &container.0)
            })?;
            let NodeKind::Extension {
                modality,
                payload,
                children,
            } = &node.kind
            else {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Expected stack container.",
                    &container.0,
                ));
            };
            if !is_stack_modality(modality) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Expected stack modality.",
                    &container.0,
                ));
            }
            let mut config = StackConfiguration::parse(payload, children)?;
            if !children.iter().any(|n| n.id == child) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Stack child not found.",
                    &child.0,
                ));
            }
            config.selected = Some(child.clone());
            if expanded {
                if !config.multiple {
                    config.expanded.clear();
                }
                config.expanded.insert(child);
            } else {
                config.expanded.remove(&child);
            }
            apply_operation(
                doc,
                Operation::UpdateExtensionPayload {
                    container,
                    payload: config.to_payload()?,
                },
            )
        }
        Operation::RaiseLayer { container, child } => {
            set_layer_state(doc, container, child, LayerAction::Raise)
        }
        Operation::LowerLayer { container, child } => {
            set_layer_state(doc, container, child, LayerAction::Lower)
        }
        Operation::SelectLayer { container, child } => {
            set_layer_state(doc, container, child, LayerAction::Select)
        }
        Operation::TabPanel {
            panel,
            target_tabs,
            index,
        } => {
            let d = descriptor(doc, &panel)?;
            if !d.docking.allow_tab {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids tab docking.",
                    panel.0,
                ));
            }
            permitted_move(doc, &panel)?;
            let mut next = doc.clone();
            let moving = take_for_move(&mut next, &panel).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Panel could not be detached.",
                    panel.0.clone(),
                )
            })?;
            let ok = insert_at(&mut next, &target_tabs, moving, |n, x| {
                if let NodeKind::Tabs {
                    items,
                    active_index,
                    ..
                } = &mut n.kind
                {
                    if index > items.len() {
                        false
                    } else {
                        items.insert(index, x);
                        *active_index = index;
                        true
                    }
                } else {
                    false
                }
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Target tab group was not found after detaching the panel.",
                    target_tabs.0,
                ));
            }
            finish(doc, next, "dock-as-tab")
        }
        Operation::CenterPanel {
            panel,
            target_panel,
        } => {
            if panel == target_panel {
                return Err(DockError::new(
                    ErrorCode::Conflict,
                    "A panel cannot be docked as a tab of itself.",
                    panel.0,
                ));
            }
            if !descriptor(doc, &panel)?.docking.allow_tab {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids tab docking.",
                    panel.0,
                ));
            }
            let target_leaf_id = find_panel_leaf_in_document(doc, &target_panel)
                .map(|node| node.id.clone())
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Target panel is not active in the primary layout.",
                        target_panel.0.clone(),
                    )
                })?;
            permitted_move(doc, &panel)?;
            let mut next = doc.clone();
            let moving = take_for_move(&mut next, &panel).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Panel could not be detached.",
                    panel.0.clone(),
                )
            })?;
            if let Some((tabs_id, _)) = find_tab_member_in_document(&next, &target_panel) {
                let mut inserted = false;
                replace(&mut next, &tabs_id, |node| {
                    if let NodeKind::Tabs {
                        items,
                        active_index,
                        ..
                    } = &mut node.kind
                        && let Some(at) = items.iter().position(|child| matches!(&child.kind, NodeKind::Panel { panel: id } if id == &target_panel)) {
                            items.insert(at + 1, moving.clone()); *active_index = at + 1; inserted = true; return true;
                        }
                    false
                });
                if inserted {
                    return finish(doc, next, "center-panel");
                }
            }
            let target = all_find(&next, &target_leaf_id)
                .map(|node| std::sync::Arc::new(node.clone()))
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Target disappeared during center docking.",
                        target_panel.0.clone(),
                    )
                })?;
            let wrapper = Node::tabs(
                format!("center-tabs-{}-{}", doc.revision + 1, target_panel.0),
                vec![target, moving],
                1,
            );
            if !replace(&mut next, &target_leaf_id, |node| {
                *node = (*wrapper).clone();
                true
            }) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Target disappeared during center docking.",
                    target_panel.0,
                ));
            }
            finish(doc, next, "center-panel")
        }
        Operation::CenterPanelGroup {
            panels,
            target_panel,
        } => {
            if panels.is_empty() || panels.len() > MAX_PREFLIGHT_TARGETS - 1 {
                return Err(DockError::new(
                    ErrorCode::ResourceLimit,
                    "A centered panel group must contain between 1 and 1023 panels.",
                    "center_panel_group",
                ));
            }
            let unique: std::collections::BTreeSet<_> = panels.iter().collect();
            if unique.len() != panels.len() {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "A centered panel group cannot contain duplicate panel IDs.",
                    "center_panel_group",
                ));
            }
            if unique.contains(&target_panel) {
                return Err(DockError::new(
                    ErrorCode::Conflict,
                    "The target panel cannot also be part of the moved selection.",
                    target_panel.0,
                ));
            }
            for panel in &panels {
                if !descriptor(doc, panel)?.docking.allow_tab {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Panel policy forbids tab docking.",
                        panel.0.clone(),
                    ));
                }
                permitted_move(doc, panel)?;
            }
            let target_leaf_id = find_panel_leaf_in_document(doc, &target_panel)
                .map(|node| node.id.clone())
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Target panel is not active in the primary layout.",
                        target_panel.0.clone(),
                    )
                })?;

            let mut next = doc.clone();
            let mut moving = Vec::with_capacity(panels.len());
            for panel in &panels {
                moving.push(take_for_move(&mut next, panel).ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "A selected panel could not be detached.",
                        panel.0.clone(),
                    )
                })?);
            }

            if let Some((tabs_id, _)) = find_tab_member_in_document(&next, &target_panel) {
                let mut inserted = false;
                replace(&mut next, &tabs_id, |node| {
                    if let NodeKind::Tabs {
                        items,
                        active_index,
                        ..
                    } = &mut node.kind
                        && let Some(at) = items.iter().position(|child| {
                            matches!(
                                &child.kind,
                                NodeKind::Panel { panel: id } if id == &target_panel
                            )
                        })
                    {
                        let insertion = at + 1;
                        let count = moving.len();
                        for (offset, child) in moving.iter().cloned().enumerate() {
                            items.insert(insertion + offset, child);
                        }
                        *active_index = insertion + count - 1;
                        inserted = true;
                        return true;
                    }
                    false
                });
                if inserted {
                    return finish(doc, next, "center-panel-group");
                }
            }

            let target = all_find(&next, &target_leaf_id)
                .map(|node| std::sync::Arc::new(node.clone()))
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Target disappeared during group center docking.",
                        target_panel.0.clone(),
                    )
                })?;
            let mut children = Vec::with_capacity(moving.len() + 1);
            children.push(target);
            children.extend(moving);
            let active_index = children.len() - 1;
            let wrapper = Node::tabs(
                format!("center-tabs-{}-{}", doc.revision + 1, target_panel.0),
                children,
                active_index,
            );
            if !replace(&mut next, &target_leaf_id, |node| {
                *node = (*wrapper).clone();
                true
            }) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Target disappeared during group center docking.",
                    target_panel.0,
                ));
            }
            finish(doc, next, "center-panel-group")
        }
        Operation::SwapPanels { first, second } => {
            if first == second {
                return Err(DockError::new(
                    ErrorCode::Conflict,
                    "A panel cannot be swapped with itself.",
                    first.0,
                ));
            }
            for panel in [&first, &second] {
                let descriptor = permitted_move(doc, panel)?;
                if descriptor.reparent != ReparentCapability::Supported {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Panel swap requires fully supported content reparenting.",
                        panel.0.clone(),
                    ));
                }
            }
            let leaf_id = |panel: &PanelId| {
                doc.root
                    .as_ref()
                    .and_then(|root| find_panel_leaf(root, panel))
                    .map(|node| node.id.clone())
                    .or_else(|| {
                        doc.floating_windows.iter().find_map(|window| {
                            find_panel_leaf(&window.root, panel).map(|node| node.id.clone())
                        })
                    })
            };
            let first_leaf = leaf_id(&first).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "First panel is not active.",
                    first.0.clone(),
                )
            })?;
            let second_leaf = leaf_id(&second).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Second panel is not active.",
                    second.0.clone(),
                )
            })?;
            let mut next = doc.clone();
            let first_replaced = replace(&mut next, &first_leaf, |node| {
                if let NodeKind::Panel { panel } = &mut node.kind {
                    *panel = second.clone();
                    true
                } else {
                    false
                }
            });
            let second_replaced = replace(&mut next, &second_leaf, |node| {
                if let NodeKind::Panel { panel } = &mut node.kind {
                    *panel = first.clone();
                    true
                } else {
                    false
                }
            });
            if !first_replaced || !second_replaced {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "A panel leaf disappeared during the swap.",
                    format!("{}:{}", first.0, second.0),
                ));
            }
            finish(doc, next, "swap-panels")
        }
        Operation::RootPanel { panel } => {
            if doc.root.is_some() {
                return Err(DockError::new(
                    ErrorCode::Conflict,
                    "Root placement requires an empty host.",
                    panel.0,
                ));
            }
            descriptor(doc, &panel)?;
            permitted_move(doc, &panel)?;
            let mut next = doc.clone();
            next.root = take_for_move(&mut next, &panel);
            finish(doc, next, "dock-as-root")
        }
        Operation::SplitPanel {
            panel,
            target,
            orientation,
            before,
            ratio,
        } => {
            if !ratio.is_finite() || ratio <= 0.0 || ratio >= 1.0 {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Split ratio must be between zero and one.",
                    target.0,
                ));
            }
            let d = descriptor(doc, &panel)?;
            let edge = match (orientation, before) {
                (Orientation::Horizontal, true) => DockEdge::Left,
                (Orientation::Horizontal, false) => DockEdge::Right,
                (Orientation::Vertical, true) => DockEdge::Top,
                (Orientation::Vertical, false) => DockEdge::Bottom,
            };
            if !edge_allowed(d, edge, true) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids requested split edge.",
                    panel.0,
                ));
            }
            permitted_move(doc, &panel)?;
            let collapsed_tab_sibling = all_find(doc, &target).and_then(|node| match &node.kind {
                NodeKind::Tabs { items, .. } if items.len() == 2 => items
                    .iter()
                    .find(|item| !matches!(&item.kind, NodeKind::Panel { panel: p } if p == &panel))
                    .map(|item| item.id.clone()),
                _ => None,
            });
            let mut next = doc.clone();
            let moving = take_for_move(&mut next, &panel).unwrap();
            let effective_target = if all_find(&next, &target).is_some() {
                target.clone()
            } else {
                collapsed_tab_sibling.unwrap_or_else(|| target.clone())
            };
            let id = NodeId(format!("split-{}", doc.revision + 1));
            let ok = insert_at(&mut next, &effective_target, moving, |n, x| {
                let old = std::sync::Arc::new(n.clone());
                let (children, weights) = if before {
                    (vec![x, old], vec![ratio, 1.0 - ratio])
                } else {
                    (vec![old, x], vec![1.0 - ratio, ratio])
                };
                *n = (*Node::split(id.clone(), orientation, children, weights)).clone();
                true
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Split target was not found.",
                    target.0,
                ));
            }
            finish(doc, next, "split-panel")
        }
        Operation::FloatPanel {
            panel,
            window,
            geometry,
            monitor,
            scale,
        } => {
            let d = descriptor(doc, &panel)?;
            if !d.docking.allow_float {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids floating.",
                    panel.0,
                ));
            }
            if window.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidId,
                    "Window ID cannot be empty.",
                    "float",
                ));
            }
            if doc.floating_windows.iter().any(|w| w.id == window) {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Floating window already exists.",
                    window.0,
                ));
            }
            permitted_move(doc, &panel)?;
            let mut next = doc.clone();
            if next.maximized_panel.as_ref() == Some(&panel) {
                next.maximized_panel = None;
            }
            let root = take_for_move(&mut next, &panel).unwrap();
            let z = next.floating_windows.len() as i32;
            next.floating_windows.push(FloatingWindow {
                id: window,
                geometry,
                monitor_id: monitor,
                scale,
                z_order_hint: z,
                picture_in_picture: false,
                scratchpad: None,
                root,
            });
            finish(doc, next, "float-panel")
        }
        Operation::FloatGroup {
            group,
            window,
            geometry,
            monitor,
            scale,
        } => {
            let group_node = all_find(doc, &group).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Dock group was not found.",
                    group.0.clone(),
                )
            })?;
            if !matches!(&group_node.kind, NodeKind::Tabs { .. }) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Only a tab group can be floated as a group.",
                    group.0,
                ));
            }
            if window.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidId,
                    "Window ID cannot be empty.",
                    "float-group",
                ));
            }
            if doc.floating_windows.iter().any(|entry| entry.id == window) {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Floating window already exists.",
                    window.0,
                ));
            }
            let mut panels = Vec::new();
            collect_node_panels(group_node, &mut panels);
            if panels.is_empty() {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Dock group contains no panels.",
                    group.0,
                ));
            }
            for panel in &panels {
                let descriptor = permitted_move(doc, panel)?;
                if !descriptor.docking.allow_float {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Panel policy forbids floating.",
                        panel.0.clone(),
                    ));
                }
            }
            let root = std::sync::Arc::new(group_node.clone());
            let mut next = doc.clone();
            for panel in &panels {
                take_panel(&mut next, panel).ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Panel disappeared while floating group.",
                        panel.0.clone(),
                    )
                })?;
                if next.maximized_panel.as_ref() == Some(panel) {
                    next.maximized_panel = None;
                }
            }
            let z = next
                .floating_windows
                .iter()
                .map(|entry| entry.z_order_hint)
                .max()
                .unwrap_or(-1)
                .saturating_add(1);
            next.floating_windows.push(FloatingWindow {
                id: window,
                geometry,
                monitor_id: monitor,
                scale,
                z_order_hint: z,
                picture_in_picture: false,
                scratchpad: None,
                root,
            });
            finish(doc, next, "float-group")
        }
        Operation::ReorderPanel {
            panel,
            container,
            index,
        } => {
            let d = descriptor(doc, &panel)?;
            let n = all_find(doc, &container).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Container not found.",
                    container.0.clone(),
                )
            })?;
            let count = match &n.kind {
                NodeKind::Tabs { items, .. } => items.len(),
                NodeKind::Split { children, .. } => children.len(),
                _ => {
                    return Err(DockError::new(
                        ErrorCode::NotFound,
                        "Target must be tabs or split.",
                        container.0,
                    ));
                }
            };
            if index >= count {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Reorder index outside container.",
                    index.to_string(),
                ));
            }
            if matches!(n.kind, NodeKind::Tabs { .. }) && !d.docking.allow_tab
                || matches!(n.kind, NodeKind::Split { .. }) && !d.docking.allow_split
            {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids reorder.",
                    panel.0,
                ));
            }
            let mut next = doc.clone();
            let mut ok = false;
            replace(&mut next, &container, |n| {
                let list = match &mut n.kind {
                    NodeKind::Tabs { items, .. } => items,
                    NodeKind::Split { children, .. } => children,
                    _ => return false,
                };
                let from = list
                    .iter()
                    .position(|c| matches!(&c.kind,NodeKind::Panel{panel:p} if p==&panel));
                if let Some(from) = from {
                    let item = list.remove(from);
                    let dst = if index > from { index - 1 } else { index };
                    list.insert(dst, item);
                    if let NodeKind::Tabs { active_index, .. } = &mut n.kind {
                        *active_index = dst
                    }
                    ok = true
                }
                true
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Panel must be a direct child of reorder target.",
                    panel.0,
                ));
            }
            finish(doc, next, "reorder-panel")
        }
        Operation::ClosePanel { panel } => {
            let d = descriptor(doc, &panel)?;
            if d.close_policy == ClosePolicy::Forbidden {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel cannot be closed.",
                    panel.0,
                ));
            }
            let mut next = doc.clone();
            if take_panel(&mut next, &panel).is_none() {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Panel is not open.",
                    panel.0,
                ));
            }
            if next.maximized_panel.as_ref() == Some(&panel) {
                next.maximized_panel = None;
            }
            next.auto_hide_panels.retain(|x| x.panel != panel);
            if !next.closed_panels.contains(&panel) {
                next.closed_panels.push(panel.clone())
            }
            finish(doc, next, "close-panel")
        }
        Operation::ClosePanelGroup { panels } => {
            if panels.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Cannot close an empty panel group.",
                    "panels",
                ));
            }
            let unique: std::collections::BTreeSet<_> = panels.iter().collect();
            if unique.len() != panels.len() {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Panel group contains duplicate panel identifiers.",
                    "panels",
                ));
            }
            for panel in &panels {
                if descriptor(doc, panel)?.close_policy == ClosePolicy::Forbidden {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Panel group contains a panel that cannot be closed.",
                        panel.0.clone(),
                    ));
                }
            }
            let mut next = doc.clone();
            for panel in panels {
                if take_panel(&mut next, &panel).is_none() {
                    return Err(DockError::new(
                        ErrorCode::NotFound,
                        "Panel group contains a panel that is not open.",
                        panel.0.clone(),
                    ));
                }
                if next.maximized_panel.as_ref() == Some(&panel) {
                    next.maximized_panel = None;
                }
                next.auto_hide_panels.retain(|entry| entry.panel != panel);
                if !next.closed_panels.contains(&panel) {
                    next.closed_panels.push(panel);
                }
            }
            finish(doc, next, "close-panel-group")
        }
        Operation::ReopenPanel {
            panel,
            target_tabs,
            index,
        } => {
            if !doc.closed_panels.contains(&panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Panel is not closed.",
                    panel.0,
                ));
            }
            let mut next = doc.clone();
            next.closed_panels.retain(|p| p != &panel);
            let node = Node::panel(
                format!("reopen-{}-{}", next.revision + 1, panel.0),
                panel.clone(),
            );
            if next.root.is_none() && (target_tabs.is_empty() || target_tabs.0 == "root") {
                next.root = Some(node);
                return finish(doc, next, "reopen-panel");
            }
            let tab_group_id = format!("reopen-tabs-{}-{}", next.revision + 1, panel.0);
            let ok = insert_at(&mut next, &target_tabs, node, |n, x| match &mut n.kind {
                NodeKind::Tabs {
                    items,
                    active_index,
                    ..
                } => {
                    if index > items.len() {
                        false
                    } else {
                        items.insert(index, x);
                        *active_index = index;
                        true
                    }
                }
                NodeKind::Panel { .. } => {
                    if index > 1 {
                        false
                    } else {
                        let existing = std::sync::Arc::new(n.clone());
                        let items = if index == 0 {
                            vec![x, existing]
                        } else {
                            vec![existing, x]
                        };
                        *n = (*Node::tabs(tab_group_id.clone(), items, index)).clone();
                        true
                    }
                }
                _ => false,
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Reopen target tabs not found.",
                    target_tabs.0,
                ));
            }
            finish(doc, next, "reopen-panel")
        }
        Operation::ResizeSplit { split, weights } => {
            let mut next = doc.clone();
            let mut valid = false;
            let ok = replace(&mut next, &split, |n| {
                if let NodeKind::Split {
                    weights: current, ..
                } = &mut n.kind
                {
                    if weights.len() != current.len() {
                        return false;
                    }
                    *current = weights.clone();
                    valid = true;
                    true
                } else {
                    false
                }
            });
            if !ok || !valid {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Split node not found or invalid weights.",
                    split.0,
                ));
            }
            finish(doc, next, "resize-split")
        }
        Operation::SetGroupMinimized { group, minimized } => {
            let node = all_find(doc, &group).ok_or_else(|| {
                DockError::new(ErrorCode::NotFound, "Tab group not found.", group.0.clone())
            })?;
            if !matches!(node.kind, NodeKind::Tabs { .. }) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Only a tab group can be minimized.",
                    group.0,
                ));
            }
            let mut next = doc.clone();
            let changed = replace(&mut next, &group, |node| {
                if let NodeKind::Tabs {
                    minimized: state, ..
                } = &mut node.kind
                {
                    *state = minimized;
                    true
                } else {
                    false
                }
            });
            if !changed {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Tab group disappeared before minimize operation.",
                    group.0,
                ));
            }
            finish(doc, next, "set-group-minimized")
        }
        Operation::AutoHide { panel, edge } => {
            let d = descriptor(doc, &panel)?;
            if !edge_allowed(d, edge, false) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids auto-hide edge.",
                    panel.0,
                ));
            }
            if !live(doc, &panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Only a docked panel may be auto-hidden.",
                    panel.0,
                ));
            }
            let mut next = doc.clone();
            if next.maximized_panel.as_ref() == Some(&panel) {
                next.maximized_panel = None;
            }
            take_panel(&mut next, &panel);
            next.auto_hide_panels.push(AutoHidePanel {
                panel: panel.clone(),
                edge,
            });
            finish(doc, next, "auto-hide")
        }
        Operation::PinPanel {
            panel,
            target_tabs,
            index,
        } => {
            let d = descriptor(doc, &panel)?;
            if !d.docking.allow_tab {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids tab docking.",
                    panel.0,
                ));
            }
            if !doc.auto_hide_panels.iter().any(|x| x.panel == panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Panel is not auto-hidden.",
                    panel.0,
                ));
            }
            let target = find_node(
                doc.root.as_ref().ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Pin target not found.",
                        target_tabs.0.clone(),
                    )
                })?,
                &target_tabs,
            )
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Pin target not found.",
                    target_tabs.0.clone(),
                )
            })?;
            if matches!(target.kind, NodeKind::Tabs { .. }) {
                return apply_operation(
                    doc,
                    Operation::TabPanel {
                        panel,
                        target_tabs,
                        index,
                    },
                );
            }
            if doc.root.as_ref().is_none_or(|r| r.id != target.id) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Pin target must be tabs or primary root.",
                    target_tabs.0,
                ));
            }
            if index > 1 {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Root pin index must be zero or one.",
                    index.to_string(),
                ));
            }
            let mut next = doc.clone();
            let moved = take_for_move(&mut next, &panel).unwrap();
            let old = next.root.take().unwrap();
            let items = if index == 0 {
                vec![moved, old]
            } else {
                vec![old, moved]
            };
            next.root = Some(Node::tabs(format!("pin-tabs-{}", panel.0), items, index));
            finish(doc, next, "pin-panel")
        }
        Operation::Maximize { panel } => {
            if !live(doc, &panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Only an open panel can be maximized.",
                    panel.0,
                ));
            }
            let mut next = doc.clone();
            next.maximized_panel = Some(panel);
            finish(doc, next, "maximize")
        }
        Operation::RestoreMaximized => {
            let mut next = doc.clone();
            next.maximized_panel = None;
            finish(doc, next, "restore-maximized")
        }
        Operation::SelectTab { tabs, index } => {
            let n = all_find(doc, &tabs).ok_or_else(|| {
                DockError::new(ErrorCode::NotFound, "Tab group not found.", tabs.0.clone())
            })?;
            let count = if let NodeKind::Tabs { items, .. } = &n.kind {
                items.len()
            } else {
                0
            };
            if index >= count {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Tab index out of range.",
                    index.to_string(),
                ));
            }
            let mut next = doc.clone();
            let ok = replace(&mut next, &tabs, |n| {
                if let NodeKind::Tabs { active_index, .. } = &mut n.kind {
                    *active_index = index;
                    true
                } else {
                    false
                }
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Tab group not found.",
                    tabs.0,
                ));
            }
            finish(doc, next, "select-tab")
        }
        Operation::ActivateDocumentGroup { well, index } => {
            let n = all_find(doc, &well).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Document well not found.",
                    well.0.clone(),
                )
            })?;
            let count = if let NodeKind::DocumentWell { groups, .. } = &n.kind {
                groups.len()
            } else {
                0
            };
            if index >= count {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Document group index out of range.",
                    index.to_string(),
                ));
            }
            let mut next = doc.clone();
            let ok = replace(&mut next, &well, |n| {
                if let NodeKind::DocumentWell { active_group, .. } = &mut n.kind {
                    *active_group = index;
                    true
                } else {
                    false
                }
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Document well not found.",
                    well.0,
                ));
            }
            finish(doc, next, "activate-document-group")
        }
        Operation::UpdateExtensionPayload { container, payload } => {
            let mut next = doc.clone();
            let ok = replace(&mut next, &container, |node| {
                if let NodeKind::Extension {
                    payload: current, ..
                } = &mut node.kind
                {
                    *current = payload.clone();
                    true
                } else {
                    false
                }
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Extension container was not found.",
                    container.0,
                ));
            }
            finish(doc, next, "update-extension-payload")
        }
        Operation::AddPanel { descriptor, target } => {
            if descriptor.id.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidId,
                    "Panel descriptor ID cannot be empty.",
                    "add-panel",
                ));
            }
            if doc.descriptor(&descriptor.id).is_some() {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Panel descriptor already exists.",
                    descriptor.id.0,
                ));
            }
            if !descriptor.docking.allow_tab && doc.root.is_some() {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids tab docking.",
                    descriptor.id.0,
                ));
            }
            let panel = descriptor.id.clone();
            let mut next = doc.clone();
            std::sync::Arc::make_mut(&mut next.panels).push(descriptor);
            let panel_node = Node::panel(
                format!("panel-node-{}-{}", doc.revision + 1, panel.0),
                panel.clone(),
            );
            if next.root.is_none() && target.is_none() {
                next.root = Some(panel_node);
                return finish(doc, next, "add-panel");
            }
            let target_id = target
                .or_else(|| next.root.as_deref().and_then(first_dock_target))
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "No dockable target group found for new panel.",
                        panel.0.clone(),
                    )
                })?;
            let tab_group_id = format!("tabs-{}-{}", doc.revision + 1, panel.0);
            let ok = insert_at(&mut next, &target_id, panel_node, |n, x| {
                match &mut n.kind {
                    NodeKind::Tabs {
                        items,
                        active_index,
                        ..
                    } => {
                        items.push(x);
                        *active_index = items.len() - 1;
                        true
                    }
                    NodeKind::Panel { .. } => {
                        let existing = std::sync::Arc::new(n.clone());
                        *n = (*Node::tabs(tab_group_id.clone(), vec![existing, x], 1)).clone();
                        true
                    }
                    _ => false,
                }
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Target group was not found for new panel.",
                    target_id.0,
                ));
            }
            finish(doc, next, "add-panel")
        }
        Operation::AddGroup {
            descriptor,
            target,
            orientation,
            before,
            ratio,
        } => {
            if descriptor.id.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidId,
                    "Panel descriptor ID cannot be empty.",
                    "add-group",
                ));
            }
            if doc.descriptor(&descriptor.id).is_some() {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Panel descriptor already exists.",
                    descriptor.id.0,
                ));
            }
            if !ratio.is_finite() || ratio <= 0.0 || ratio >= 1.0 {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Split ratio must be between zero and one.",
                    descriptor.id.0,
                ));
            }
            let edge = match (orientation, before) {
                (Orientation::Horizontal, true) => DockEdge::Left,
                (Orientation::Horizontal, false) => DockEdge::Right,
                (Orientation::Vertical, true) => DockEdge::Top,
                (Orientation::Vertical, false) => DockEdge::Bottom,
            };
            if !edge_allowed(&descriptor, edge, true) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Panel policy forbids requested split edge.",
                    descriptor.id.0,
                ));
            }
            let panel = descriptor.id.clone();
            let mut next = doc.clone();
            std::sync::Arc::make_mut(&mut next.panels).push(descriptor);
            let panel_node = Node::panel(
                format!("panel-node-{}-{}", doc.revision + 1, panel.0),
                panel.clone(),
            );
            if next.root.is_none() && target.is_none() {
                next.root = Some(panel_node);
                return finish(doc, next, "add-group");
            }
            let target_id = target
                .or_else(|| next.root.as_ref().map(|r| r.id.clone()))
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Split target was not found.",
                        panel.0.clone(),
                    )
                })?;
            let split_id = NodeId(format!("split-{}-{}", doc.revision + 1, panel.0));
            let ok = insert_at(&mut next, &target_id, panel_node, |n, x| {
                let old = std::sync::Arc::new(n.clone());
                let (children, weights) = if before {
                    (vec![x, old], vec![ratio, 1.0 - ratio])
                } else {
                    (vec![old, x], vec![1.0 - ratio, ratio])
                };
                *n = (*Node::split(split_id.clone(), orientation, children, weights)).clone();
                true
            });
            if !ok {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Split target was not found.",
                    target_id.0,
                ));
            }
            finish(doc, next, "add-group")
        }
        Operation::UpdateFloatingWindow { window, geometry } => {
            if geometry.width <= 0 || geometry.height <= 0 {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Floating window width and height must be positive.",
                    window.0,
                ));
            }
            let mut next = doc.clone();
            let Some(entry) = next.floating_windows.iter_mut().find(|w| w.id == window) else {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Floating window was not found.",
                    window.0,
                ));
            };
            entry.geometry = geometry;
            promote_window(&mut next.floating_windows, &window)?;
            finish(doc, next, "update-floating-window")
        }
        Operation::SetScratchpad { window, config } => {
            let mut next = doc.clone();
            let Some(entry) = next
                .floating_windows
                .iter_mut()
                .find(|entry| entry.id == window)
            else {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Scratchpad target floating window was not found.",
                    window.0,
                ));
            };
            let becoming_visible = config.as_ref().is_some_and(|config| {
                config.visible
                    && entry
                        .scratchpad
                        .as_ref()
                        .is_none_or(|previous| !previous.visible)
            });
            if becoming_visible
                && let Some(position) = config.as_ref().and_then(|config| config.summon_position)
            {
                entry.geometry.x = position.x;
                entry.geometry.y = position.y;
            }
            entry.scratchpad = config;
            if becoming_visible {
                promote_window(&mut next.floating_windows, &window)?;
            }
            finish(doc, next, "set-scratchpad")
        }
        Operation::SetPictureInPicture { window, enabled } => {
            let mut next = doc.clone();
            let Some(entry) = next
                .floating_windows
                .iter_mut()
                .find(|entry| entry.id == window)
            else {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Floating window was not found.",
                    window.0,
                ));
            };
            if enabled {
                let bounds = entry.geometry;
                let aspect = bounds.width as f64 / bounds.height.max(1) as f64;
                if !matches!(&entry.root.kind, NodeKind::Panel { .. })
                    || !(120..=1600).contains(&bounds.width)
                    || !(68..=1200).contains(&bounds.height)
                    || !(0.4..=3.0).contains(&aspect)
                {
                    return Err(DockError::new(
                        ErrorCode::ConstraintViolation,
                        "Picture-in-picture requires one panel and bounded geometry.",
                        window.0,
                    ));
                }
            }
            entry.picture_in_picture = enabled;
            finish(doc, next, "set-picture-in-picture")
        }
        Operation::ActivateFloatingWindow { window } => {
            let mut next = doc.clone();
            promote_window(&mut next.floating_windows, &window)?;
            finish(doc, next, "activate-floating-window")
        }
        Operation::RecoverFloatingWindow {
            window,
            geometry,
            monitor_id,
            scale,
        } => {
            if geometry.width <= 0
                || geometry.height <= 0
                || monitor_id.trim().is_empty()
                || !scale.is_finite()
                || scale <= 0.0
            {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Recovered floating-window state is invalid.",
                    window.0,
                ));
            }
            let mut next = doc.clone();
            let Some(entry) = next
                .floating_windows
                .iter_mut()
                .find(|entry| entry.id == window)
            else {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Floating window was not found.",
                    window.0,
                ));
            };
            entry.geometry = geometry;
            entry.monitor_id = monitor_id;
            entry.scale = scale;
            finish(doc, next, "recover-floating-window")
        }
        Operation::DockFloatingWindow {
            window,
            target_panel,
        } => {
            let source = doc
                .floating_windows
                .iter()
                .find(|entry| entry.id == window)
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Floating window was not found.",
                        window.0.clone(),
                    )
                })?;
            let source_root = source.root.clone();
            let mut panels = Vec::new();
            collect_node_panels(&source_root, &mut panels);
            if panels.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Floating window has no dockable panels.",
                    window.0,
                ));
            }
            let mut root_panels = Vec::new();
            if let Some(root) = &doc.root {
                collect_node_panels(root, &mut root_panels);
            }
            let target = target_panel.or_else(|| root_panels.first().cloned());
            if let Some(target_id) = &target
                && !root_panels.contains(target_id)
            {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Dock target must be an active panel in the main root.",
                    target_id.0.clone(),
                ));
            }
            for panel in &panels {
                let descriptor = permitted_move(doc, panel)?;
                if !descriptor.docking.allow_tab {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Panel policy forbids tab docking.",
                        panel.0.clone(),
                    ));
                }
            }
            let mut next = doc.clone();
            if let Some(target_panel) = target {
                let target_leaf_id = next
                    .root
                    .as_ref()
                    .and_then(|root| find_panel_leaf(root, &target_panel))
                    .map(|node| node.id.clone())
                    .ok_or_else(|| {
                        DockError::new(
                            ErrorCode::NotFound,
                            "Dock target disappeared before group insertion.",
                            target_panel.0.clone(),
                        )
                    })?;
                let insertion = next
                    .root
                    .as_ref()
                    .and_then(|root| find_tab_member(root, &target_panel));
                let mut inserted = false;
                if let Some((tabs_id, index)) = insertion {
                    replace(&mut next, &tabs_id, |node| {
                        if let NodeKind::Tabs {
                            items,
                            active_index,
                            ..
                        } = &mut node.kind
                        {
                            items.insert(index + 1, source_root.clone());
                            *active_index = index + 1;
                            inserted = true;
                            true
                        } else {
                            false
                        }
                    });
                } else if let Some(target) = next
                    .root
                    .as_ref()
                    .and_then(|root| find_node(root, &target_leaf_id))
                    .cloned()
                {
                    let wrapper = Node::tabs(
                        format!("dock-window-tabs-{}-{}", doc.revision + 1, target_panel.0),
                        vec![std::sync::Arc::new(target), source_root.clone()],
                        1,
                    );
                    inserted = replace(&mut next, &target_leaf_id, |node| {
                        *node = (*wrapper).clone();
                        true
                    });
                }
                if !inserted {
                    return Err(DockError::new(
                        ErrorCode::NotFound,
                        "Dock target disappeared during group insertion.",
                        target_panel.0,
                    ));
                }
            } else {
                if next.root.is_some() {
                    return Err(DockError::new(
                        ErrorCode::Conflict,
                        "A floating group requires an empty root when no dock target is provided.",
                        window.0,
                    ));
                }
                next.root = Some(source_root);
            }
            next.floating_windows.retain(|entry| entry.id != window);
            if next.floating_windows.iter().any(|entry| entry.id == window) {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Dock operation did not consume the floating window.",
                    window.0,
                ));
            }
            next.revision = doc.revision;
            finish(doc, next, "dock-floating-window")
        }
        Operation::CloseFloatingWindow { window } => {
            let source = doc
                .floating_windows
                .iter()
                .find(|entry| entry.id == window)
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::NotFound,
                        "Floating window was not found.",
                        window.0.clone(),
                    )
                })?;
            let mut panels = Vec::new();
            collect_node_panels(&source.root, &mut panels);
            if panels.is_empty() {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Floating window has no closable panels.",
                    window.0,
                ));
            }
            for panel in &panels {
                let descriptor = descriptor(doc, panel)?;
                if descriptor.close_policy == ClosePolicy::Forbidden {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Floating window contains a panel that cannot be closed.",
                        panel.0.clone(),
                    ));
                }
            }
            let mut next = doc.clone();
            next.floating_windows.retain(|entry| entry.id != window);
            for panel in panels {
                if next.maximized_panel.as_ref() == Some(&panel) {
                    next.maximized_panel = None;
                }
                next.auto_hide_panels.retain(|entry| entry.panel != panel);
                if !next.closed_panels.contains(&panel) {
                    next.closed_panels.push(panel);
                }
            }
            finish(doc, next, "close-floating-window")
        }
        Operation::ClearLayout => {
            let mut open_panels = Vec::new();
            if let Some(root) = &doc.root {
                collect_node_panels(root, &mut open_panels);
            }
            for window in &doc.floating_windows {
                collect_node_panels(&window.root, &mut open_panels);
            }
            for drawer in &doc.auto_hide_panels {
                if !open_panels.contains(&drawer.panel) {
                    open_panels.push(drawer.panel.clone());
                }
            }
            for panel in &open_panels {
                if descriptor(doc, panel)?.close_policy == ClosePolicy::Forbidden {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Cannot clear layout while a non-closable panel is active.",
                        panel.0.clone(),
                    ));
                }
            }
            let mut next = doc.clone();
            next.root = None;
            next.floating_windows.clear();
            next.auto_hide_panels.clear();
            next.maximized_panel = None;
            for panel in open_panels {
                if !next.closed_panels.contains(&panel) {
                    next.closed_panels.push(panel);
                }
            }
            finish(doc, next, "clear-layout")
        }
    }
}
fn first_dock_target(node: &Node) -> Option<NodeId> {
    match &node.kind {
        NodeKind::Tabs { .. } | NodeKind::Panel { .. } => Some(node.id.clone()),
        NodeKind::DocumentWell {
            active_group,
            groups,
        } => groups
            .get(*active_group)
            .or_else(|| groups.first())
            .and_then(|g| first_dock_target(g)),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter().find_map(|c| first_dock_target(c))
        }
        NodeKind::Placeholder { .. } => None,
    }
}
fn collect_node_panels(node: &Node, out: &mut Vec<PanelId>) {
    match &node.kind {
        NodeKind::Panel { panel } => {
            if !out.contains(panel) {
                out.push(panel.clone());
            }
        }
        NodeKind::Tabs { items, .. }
        | NodeKind::DocumentWell { groups: items, .. }
        | NodeKind::Split {
            children: items, ..
        }
        | NodeKind::Extension {
            children: items, ..
        } => {
            for item in items {
                collect_node_panels(item, out);
            }
        }
        NodeKind::Placeholder { .. } => {}
    }
}
fn promote_window(windows: &mut [FloatingWindow], target: &WindowId) -> DockResult<()> {
    if !windows.iter().any(|window| &window.id == target) {
        return Err(DockError::new(
            ErrorCode::NotFound,
            "Floating window was not found.",
            target.0.clone(),
        ));
    }
    let mut order: Vec<_> = windows.iter().map(|window| window.id.clone()).collect();
    order.sort_by(|left, right| {
        let left_z = windows
            .iter()
            .find(|window| &window.id == left)
            .map_or(0, |window| window.z_order_hint);
        let right_z = windows
            .iter()
            .find(|window| &window.id == right)
            .map_or(0, |window| window.z_order_hint);
        left_z.cmp(&right_z).then_with(|| left.cmp(right))
    });
    order.retain(|window| window != target);
    order.push(target.clone());
    for (index, id) in order.iter().enumerate() {
        if let Some(window) = windows.iter_mut().find(|window| &window.id == id) {
            window.z_order_hint = i32::try_from(index).map_err(|_| {
                DockError::new(
                    ErrorCode::ResourceLimit,
                    "Floating window z-order exceeds the supported range.",
                    id.0.clone(),
                )
            })?;
        }
    }
    Ok(())
}
fn all_find<'a>(doc: &'a LayoutDocument, id: &NodeId) -> Option<&'a Node> {
    if let Some(r) = &doc.root
        && let Some(n) = find_node(r, id)
    {
        return Some(n);
    }
    doc.floating_windows
        .iter()
        .find_map(|w| find_node(&w.root, id))
}
fn find_node<'a>(n: &'a Node, id: &NodeId) -> Option<&'a Node> {
    crate::find_node(n, id)
}
