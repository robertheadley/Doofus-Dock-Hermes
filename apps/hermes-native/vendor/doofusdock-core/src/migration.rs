//! Read-only, version-aware previews for persisted DoofusDock data.
//!
//! Hosts own their data directories, so this API accepts bytes read by the
//! caller and never writes files. A host can gather previews for its known
//! stores, show the complete plan, and only then choose a checkpoint/commit
//! workflow.

use crate::{
    DockError, DockResult, ErrorCode, LayoutDocument, LayoutId, PersistenceLimits, WindowRuleSet,
    WorkspaceCatalog, WorkspaceProfileCatalog, deserialize_layout, serialize_layout,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MIGRATION_PREVIEW_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedDataFormat {
    Layout,
    ProfileCatalog,
    WindowRules,
    WorkspaceCatalog,
    WorkspacePreferences,
    /// Checksummed workspace preferences containing extension-owned state.
    /// Requires an activated extension registry for preview and apply.
    ExtensionState,
    ThemePackage,
}

impl PersistedDataFormat {
    fn current_version(self) -> u64 {
        match self {
            Self::Layout => u64::from(crate::CURRENT_LAYOUT_SCHEMA),
            Self::ThemePackage => u64::from(crate::THEME_SCHEMA_VERSION),
            _ => 1,
        }
    }

    fn maximum_bytes(self) -> usize {
        match self {
            Self::Layout
            | Self::ProfileCatalog
            | Self::WorkspaceCatalog
            | Self::WorkspacePreferences
            | Self::ExtensionState => MIGRATION_PREVIEW_LIMIT,
            Self::WindowRules => 1024 * 1024,
            Self::ThemePackage => 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationPreview {
    pub format: PersistedDataFormat,
    pub source_version: u64,
    pub target_version: u64,
    pub requires_migration: bool,
    /// Stable, non-cryptographic fingerprint for comparing the inspected input.
    /// It is not an integrity or authenticity check.
    pub source_fingerprint: String,
    /// Validated target representation. The caller decides whether and when to
    /// persist it after creating a whole-data checkpoint.
    pub target_json: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension_migrations: Vec<ExtensionMigrationReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionMigrationReceipt {
    pub extension_id: String,
    pub source_schema: u32,
    pub target_schema: u32,
}

/// Validate a supported persisted value, perform its existing version adapter
/// in memory, and return the target representation without modifying storage.
pub fn preview_persisted_migration(
    format: PersistedDataFormat,
    source_json: &str,
) -> DockResult<MigrationPreview> {
    if format == PersistedDataFormat::ExtensionState {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Extension state preview requires an activated ExtensionRegistry.",
            "format=extension_state;registry=missing",
        ));
    }
    if source_json.len() > format.maximum_bytes() {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Persisted value exceeds the migration preview byte limit.",
            format!("format={format:?};bytes={}", source_json.len()),
        ));
    }
    let source: Value = serde_json::from_str(source_json).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            format!("migration-preview:{format:?}"),
        )
    })?;
    let source_version = detect_version(format, &source)?;
    if source_version > format.current_version() {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Persisted value is newer than this framework supports; preview stopped without downgrade.",
            format!("format={format:?};version={source_version}"),
        ));
    }
    let target_json = match format {
        PersistedDataFormat::Layout => {
            validate_layout_json_fields(&source)?;
            let limits = PersistenceLimits {
                maximum_bytes: format.maximum_bytes(),
                ..PersistenceLimits::default()
            };
            let document: LayoutDocument = deserialize_layout(source_json, limits)?;
            serialize_layout(&document)?
        }
        PersistedDataFormat::ProfileCatalog => migrate_profile_catalog(format, &source)?,
        PersistedDataFormat::WindowRules => {
            if source.get("rules").is_some_and(Value::is_object) && source.get("checksum").is_some()
            {
                let rules = crate::rules::decode_window_rule_file_json(source_json)?;
                String::from_utf8(crate::rules::encode_rule_file(&rules)?).map_err(|error| {
                    DockError::new(
                        ErrorCode::SerializationError,
                        error.to_string(),
                        "migration-preview:window-rules",
                    )
                })?
            } else {
                WindowRuleSet::from_json(source_json)?.to_json()?
            }
        }
        PersistedDataFormat::WorkspaceCatalog => migrate_workspace_catalog(format, &source)?,
        PersistedDataFormat::WorkspacePreferences => {
            crate::persistence::validate_workspace_preferences_envelope(
                source_json,
                PersistenceLimits {
                    maximum_bytes: format.maximum_bytes(),
                    ..PersistenceLimits::default()
                },
            )?;
            source_json.to_owned()
        }
        PersistedDataFormat::ExtensionState => unreachable!("handled before generic preview"),
        PersistedDataFormat::ThemePackage => crate::ThemePackageEditor::import_json(source_json)
            .and_then(|editor| editor.export_json())
            .map_err(|error| {
                DockError::new(
                    ErrorCode::SchemaError,
                    error.to_string(),
                    "migration-preview:theme-package",
                )
            })?,
    };
    let target: Value = serde_json::from_str(&target_json).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:target",
        )
    })?;
    let target_version = detect_version(format, &target)?;
    if target_version != format.current_version() {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Migration adapter did not produce the current schema version.",
            format!("format={format:?};version={target_version}"),
        ));
    }
    Ok(MigrationPreview {
        format,
        source_version,
        target_version,
        requires_migration: source_version != target_version,
        source_fingerprint: fingerprint(source_json.as_bytes()),
        target_json,
        extension_migrations: Vec::new(),
    })
}

/// Layout serde intentionally supports old schemas with defaults, but serde's
/// default behavior also ignores unknown object fields. A migration preview
/// must not silently erase fields added by an application or a newer writer,
/// so check the known structural keys before adapting the document. Opaque
/// extension payload strings and panel metadata maps remain application-owned.
fn validate_layout_json_fields(source: &Value) -> DockResult<()> {
    fn object_keys(value: &Value, allowed: &[&str], path: &str) -> DockResult<()> {
        let object = value.as_object().ok_or_else(|| {
            DockError::new(ErrorCode::SchemaError, "Expected a JSON object.", path)
        })?;
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(DockError::new(
                    ErrorCode::SchemaError,
                    "Unknown persisted layout field; refusing a lossy migration.",
                    format!("{path}.{key}"),
                ));
            }
        }
        Ok(())
    }
    fn visit_node(node: &Value, path: &str) -> DockResult<()> {
        let kind = node.get("type").and_then(Value::as_str).ok_or_else(|| {
            DockError::new(ErrorCode::SchemaError, "Layout node type is missing.", path)
        })?;
        let fields: &[&str] = match kind {
            "panel" => &["panel"],
            "tabs" => &["active", "minimized", "items"],
            "document_well" => &["active_group", "groups"],
            "split" => &["orientation", "weights", "children"],
            "placeholder" => &["panel", "title"],
            "extension" => &["modality", "payload", "children"],
            _ => {
                return Err(DockError::new(
                    ErrorCode::SchemaError,
                    "Unknown layout node type; refusing a lossy migration.",
                    format!("{path}.type={kind}"),
                ));
            }
        };
        let mut allowed = vec!["id", "type"];
        allowed.extend_from_slice(fields);
        object_keys(node, &allowed, path)?;
        let child_key = match kind {
            "tabs" => Some("items"),
            "document_well" => Some("groups"),
            "split" | "extension" => Some("children"),
            _ => None,
        };
        if let Some(child_key) = child_key
            && let Some(children) = node.get(child_key).and_then(Value::as_array)
        {
            for (index, child) in children.iter().enumerate() {
                visit_node(child, &format!("{path}.{child_key}[{index}]"))?;
            }
        }
        Ok(())
    }
    const LAYOUT_FIELDS: &[&str] = &[
        "schema_version",
        "layout_id",
        "workspace_id",
        "revision",
        "checksum",
        "panels",
        "closed_panels",
        "pinned_panels",
        "auto_hide_panels",
        "maximized_panel",
        "root",
        "windows",
    ];
    object_keys(source, LAYOUT_FIELDS, "layout")?;
    if let Some(panels) = source.get("panels").and_then(Value::as_array) {
        const PANEL_FIELDS: &[&str] = &[
            "id",
            "title_key",
            "icon_key",
            "min",
            "max",
            "close_policy",
            "persistence",
            "provider",
            "role",
            "metadata",
            "workspace_memberships",
            "reparent",
            "docking",
        ];
        for (index, panel) in panels.iter().enumerate() {
            let path = format!("layout.panels[{index}]");
            object_keys(panel, PANEL_FIELDS, &path)?;
            if let Some(docking) = panel.get("docking") {
                object_keys(
                    docking,
                    &[
                        "allow_tab",
                        "allow_split",
                        "allow_float",
                        "allow_auto_hide",
                        "allowed_edges",
                    ],
                    &format!("{path}.docking"),
                )?;
            }
        }
    }
    if let Some(hides) = source.get("auto_hide_panels").and_then(Value::as_array) {
        for (index, hide) in hides.iter().enumerate() {
            object_keys(
                hide,
                &["panel", "edge"],
                &format!("layout.auto_hide_panels[{index}]"),
            )?;
        }
    }
    if let Some(root) = source.get("root").filter(|value| !value.is_null()) {
        visit_node(root, "layout.root")?;
    }
    if let Some(windows) = source.get("windows").and_then(Value::as_array) {
        for (index, window) in windows.iter().enumerate() {
            let path = format!("layout.windows[{index}]");
            object_keys(
                window,
                &[
                    "id",
                    "geometry",
                    "monitor",
                    "scale",
                    "z_order",
                    "picture_in_picture",
                    "scratchpad",
                    "root",
                ],
                &path,
            )?;
            if let Some(scratchpad) = window.get("scratchpad").filter(|value| !value.is_null()) {
                object_keys(
                    scratchpad,
                    &[
                        "name",
                        "visible",
                        "summon_position",
                        "auto_hide_on_deactivate",
                        "pinned_across_workspaces",
                    ],
                    &format!("{path}.scratchpad"),
                )?;
            }
            if let Some(root) = window.get("root") {
                visit_node(root, &format!("{path}.root"))?;
            }
        }
    }
    Ok(())
}

/// Preview extension-owned states embedded in a checksummed workspace
/// preferences envelope. Unknown providers and inactive migration chains fail closed.
pub fn preview_extension_state_migration(
    registry: &crate::ExtensionRegistry,
    source_json: &str,
) -> DockResult<MigrationPreview> {
    if source_json.len() > PersistedDataFormat::ExtensionState.maximum_bytes() {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Persisted value exceeds the migration preview byte limit.",
            format!("format=ExtensionState;bytes={}", source_json.len()),
        ));
    }
    crate::persistence::validate_workspace_preferences_envelope(
        source_json,
        PersistenceLimits {
            maximum_bytes: PersistedDataFormat::ExtensionState.maximum_bytes(),
            ..PersistenceLimits::default()
        },
    )?;
    let mut root: Value = serde_json::from_str(source_json).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:extension-state",
        )
    })?;
    let extensions = root
        .get_mut("preferences")
        .and_then(Value::as_object_mut)
        .and_then(|preferences| preferences.get_mut("extensions"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::SchemaError,
                "Workspace preferences do not contain an extensions object.",
                "preferences.extensions",
            )
        })?;
    let mut receipts = Vec::new();
    for (id, entry) in extensions {
        let schema = entry
            .get("schema_version")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::SchemaError,
                    "Extension state schema_version is missing or malformed.",
                    id,
                )
            })?;
        let data = entry.get("data").ok_or_else(|| {
            DockError::new(
                ErrorCode::SchemaError,
                "Extension state data is missing.",
                id,
            )
        })?;
        let target_schema = registry
            .persistence_codec_schema_version(id)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "No registered versioned persistence codec for extension state.",
                    id,
                )
            })?;
        let migrated = registry.migrate_persistence_codec_state(id, schema, data)?;
        if schema != target_schema {
            let object = entry
                .as_object_mut()
                .expect("extension entry validated as object");
            object.insert("schema_version".into(), Value::from(target_schema));
            object.insert("data".into(), migrated);
            receipts.push(ExtensionMigrationReceipt {
                extension_id: id.clone(),
                source_schema: schema,
                target_schema,
            });
        }
    }
    let target_json = if receipts.is_empty() {
        source_json.to_owned()
    } else {
        let checksum = crate::persistence::checksum_layout_json(
            &serde_json::to_string(&root).map_err(|error| {
                DockError::new(
                    ErrorCode::SerializationError,
                    error.to_string(),
                    "extension-state",
                )
            })?,
        )?;
        root.as_object_mut()
            .expect("validated envelope object")
            .insert("checksum".into(), Value::String(checksum));
        serde_json::to_string(&root).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "extension-state",
            )
        })?
    };
    if target_json.len() > PersistedDataFormat::ExtensionState.maximum_bytes() {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Migrated extension state exceeds the migration preview byte limit.",
            format!("bytes={}", target_json.len()),
        ));
    }
    Ok(MigrationPreview {
        format: PersistedDataFormat::ExtensionState,
        source_version: 1,
        target_version: 1,
        requires_migration: !receipts.is_empty(),
        source_fingerprint: fingerprint(source_json.as_bytes()),
        target_json,
        extension_migrations: receipts,
    })
}

fn detect_version(format: PersistedDataFormat, value: &Value) -> DockResult<u64> {
    let version = match format {
        PersistedDataFormat::ProfileCatalog => value
            .get("preferences")
            .filter(|_| value.get("workspace_id").is_some())
            .and_then(|preferences| preferences.get("schema_version"))
            .or_else(|| value.get("schema_version")),
        PersistedDataFormat::WorkspaceCatalog => value
            .get("preferences")
            .filter(|_| value.get("workspace_id").is_some())
            .and_then(|preferences| preferences.get("schema_version"))
            .or_else(|| value.get("schema_version")),
        PersistedDataFormat::ThemePackage => value
            .get("manifest")
            .and_then(|manifest| manifest.get("schema_version"))
            .or_else(|| value.get("schema_version")),
        PersistedDataFormat::WindowRules => value
            .get("rules")
            .and_then(|rules| rules.get("schema_version"))
            .or_else(|| value.get("schema_version")),
        _ => value.get("schema_version"),
    };
    version.and_then(Value::as_u64).ok_or_else(|| {
        DockError::new(
            ErrorCode::SchemaError,
            "Persisted value has a missing or malformed schema version.",
            format!("format={format:?}"),
        )
    })
}

/// Migrate either the standalone profile-catalog JSON format or the actual
/// checksummed preferences envelope used by `save_profile_catalog`.
fn migrate_profile_catalog(format: PersistedDataFormat, source: &Value) -> DockResult<String> {
    let Some(preferences) = source
        .get("preferences")
        .filter(|_| source.get("workspace_id").is_some())
    else {
        return WorkspaceProfileCatalog::from_json(
            &serde_json::to_string(source).map_err(|error| {
                DockError::new(
                    ErrorCode::SerializationError,
                    error.to_string(),
                    "migration-preview:profile-catalog",
                )
            })?,
            format.maximum_bytes(),
        )?
        .to_json();
    };

    let source_json = serde_json::to_string(source).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:profile-envelope",
        )
    })?;
    crate::persistence::validate_workspace_preferences_envelope(
        &source_json,
        PersistenceLimits {
            maximum_bytes: format.maximum_bytes(),
            ..PersistenceLimits::default()
        },
    )?;
    let profile_json = serde_json::to_string(preferences).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:profile-payload",
        )
    })?;
    let migrated_json =
        WorkspaceProfileCatalog::from_json(&profile_json, format.maximum_bytes())?.to_json()?;
    let migrated_profiles: Value = serde_json::from_str(&migrated_json).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:profile-target",
        )
    })?;
    rewrite_preferences_envelope(source, migrated_profiles, "profile-envelope")
}

/// Validate a standalone workspace catalog or the actual persisted envelope
/// written by `save_workspace_catalog` and return current-schema JSON.
fn migrate_workspace_catalog(format: PersistedDataFormat, source: &Value) -> DockResult<String> {
    let Some(preferences) = source
        .get("preferences")
        .filter(|_| source.get("workspace_id").is_some())
    else {
        let json = serde_json::to_string(source).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "migration-preview:workspace-catalog",
            )
        })?;
        return WorkspaceCatalog::from_json(&json, format.maximum_bytes())?.to_json();
    };

    let envelope_json = serde_json::to_string(source).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:workspace-catalog-envelope",
        )
    })?;
    crate::persistence::validate_workspace_preferences_envelope(
        &envelope_json,
        PersistenceLimits {
            maximum_bytes: format.maximum_bytes(),
            ..PersistenceLimits::default()
        },
    )?;
    let catalog_json = serde_json::to_string(preferences).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:workspace-catalog-payload",
        )
    })?;
    let catalog = WorkspaceCatalog::from_json(&catalog_json, format.maximum_bytes())?;
    let migrated_catalog: Value = serde_json::from_str(&catalog.to_json()?).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "migration-preview:workspace-catalog-target",
        )
    })?;
    rewrite_preferences_envelope(source, migrated_catalog, "workspace-catalog-envelope")
}

fn rewrite_preferences_envelope(
    source: &Value,
    preferences: Value,
    context: &str,
) -> DockResult<String> {
    let mut envelope = source.clone();
    {
        let object = envelope.as_object_mut().ok_or_else(|| {
            DockError::new(
                ErrorCode::SchemaError,
                "Persisted preferences envelope must be a JSON object.",
                context,
            )
        })?;
        object.insert("preferences".into(), preferences);
        object.remove("checksum");
    }
    let without_checksum = serde_json::to_string(&envelope).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            format!("migration-preview:{context}-target"),
        )
    })?;
    let checksum = crate::checksum_layout_json(&without_checksum)?;
    envelope
        .as_object_mut()
        .expect("the preferences envelope object was validated above")
        .insert("checksum".into(), Value::String(checksum));
    serde_json::to_string(&envelope).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            format!("migration-preview:{context}-target"),
        )
    })
}

fn fingerprint(bytes: &[u8]) -> String {
    let mut hash = 14695981039346656037u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("fnv1a64:{hash:016x}")
}

/// Convenience validation for callers that want a layout-specific preview
/// while retaining the concrete document identity for their own report.
pub fn preview_layout_migration(source_json: &str) -> DockResult<(LayoutId, MigrationPreview)> {
    let preview = preview_persisted_migration(PersistedDataFormat::Layout, source_json)?;
    let target: LayoutDocument = deserialize_layout(
        &preview.target_json,
        PersistenceLimits {
            maximum_bytes: MIGRATION_PREVIEW_LIMIT,
            ..PersistenceLimits::default()
        },
    )?;
    Ok((target.id, preview))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        fs::read_to_string(path).expect("fixture should be readable")
    }

    #[test]
    fn preview_migrates_legacy_layout_profile_and_rules_without_mutating_input() {
        let mut legacy_layout: Value =
            serde_json::from_str(&fixture("layout_schema_v0.json")).unwrap();
        let checksum = crate::checksum_layout_json(&legacy_layout.to_string()).unwrap();
        legacy_layout["checksum"] = Value::String(checksum);
        let cases = [
            (PersistedDataFormat::Layout, legacy_layout.to_string()),
            (
                PersistedDataFormat::ProfileCatalog,
                fixture("profile_catalog_schema_v0.json"),
            ),
            (
                PersistedDataFormat::WindowRules,
                fixture("window_rules_schema_v0.json"),
            ),
        ];
        for (format, source) in cases {
            let before = source.clone();
            let preview = preview_persisted_migration(format, &source).unwrap();
            assert_eq!(preview.source_version, 0);
            assert_eq!(preview.target_version, format.current_version());
            assert!(preview.requires_migration);
            assert_eq!(source, before);
            assert_ne!(preview.target_json, source);
            if format == PersistedDataFormat::Layout {
                let migrated: crate::LayoutDocument = crate::deserialize_layout(
                    &preview.target_json,
                    crate::PersistenceLimits::default(),
                )
                .unwrap();
                let root = migrated
                    .root
                    .as_ref()
                    .expect("legacy extension root survives");
                assert_eq!(root.id.0, "legacy-extension-node");
                match &root.kind {
                    crate::NodeKind::Extension {
                        modality,
                        payload,
                        children,
                    } => {
                        assert_eq!(modality.0, "vendor.unknown-layout-v4");
                        assert_eq!(
                            payload,
                            r#"{"future_extension":{"opaque":[1,{"keep":"exact"}]}}"#
                        );
                        assert_eq!(children[0].id.0, "legacy-extension-panel-node");
                    }
                    other => panic!("expected the opaque extension node, got {other:?}"),
                }
                assert_eq!(migrated.panels.len(), 2);
                assert_eq!(migrated.floating_windows[0].id.0, "legacy-float");
            }
            let repeated = preview_persisted_migration(format, &preview.target_json).unwrap();
            assert!(!repeated.requires_migration);
            assert_eq!(repeated.target_json, preview.target_json);
        }
    }

    #[test]
    fn layout_preview_rejects_unknown_structural_fields_without_erasing_extension_data() {
        fn checked_layout(mut value: Value) -> String {
            let checksum = crate::checksum_layout_json(&value.to_string()).unwrap();
            value["checksum"] = Value::String(checksum);
            value.to_string()
        }
        let mut value: Value = serde_json::from_str(&fixture("layout_schema_v0.json")).unwrap();
        value["future_application_metadata"] = serde_json::json!({"keep": [1, 2, 3]});
        let changed = checked_layout(value.clone());
        let error = preview_persisted_migration(PersistedDataFormat::Layout, &changed)
            .expect_err("unknown structural data must fail closed");
        assert_eq!(error.code, ErrorCode::SchemaError);
        assert!(error.context.contains("future_application_metadata"));
        assert_eq!(
            value["future_application_metadata"],
            serde_json::json!({"keep": [1, 2, 3]})
        );

        let mut value: Value = serde_json::from_str(&fixture("layout_schema_v0.json")).unwrap();
        value["panels"][0]["future_panel_setting"] = Value::String("retain".into());
        let error =
            preview_persisted_migration(PersistedDataFormat::Layout, &checked_layout(value))
                .expect_err("unknown nested structural data must fail closed");
        assert!(error.context.contains("future_panel_setting"));

        let cases = [
            ("node", "root", "future_node_setting"),
            ("docking", "panels", "future_docking_setting"),
            ("scratchpad", "windows", "future_scratchpad_setting"),
        ];
        for (kind, container, key) in cases {
            let mut value: Value = serde_json::from_str(&fixture("layout_schema_v0.json")).unwrap();
            match kind {
                "node" => value["root"][key] = Value::Bool(true),
                "docking" => {
                    let mut docking = serde_json::json!({});
                    docking[key] = Value::Bool(true);
                    value["panels"][0]["docking"] = docking;
                }
                "scratchpad" => {
                    let mut scratchpad = serde_json::json!({"name": "legacy"});
                    scratchpad[key] = Value::Bool(true);
                    value["windows"][0]["scratchpad"] = scratchpad;
                }
                _ => unreachable!(),
            }
            let error =
                preview_persisted_migration(PersistedDataFormat::Layout, &checked_layout(value))
                    .unwrap_err();
            assert!(error.context.contains(key), "{container}: {error}");
        }

        // Unknown modality payloads are the supported extension point and must
        // remain byte-for-byte content through the existing adapter.
        let original =
            checked_layout(serde_json::from_str(&fixture("layout_schema_v0.json")).unwrap());
        let preview = preview_persisted_migration(PersistedDataFormat::Layout, &original).unwrap();
        let target: Value = serde_json::from_str(&preview.target_json).unwrap();
        assert_eq!(
            target["root"]["payload"],
            Value::String(r#"{"future_extension":{"opaque":[1,{"keep":"exact"}]}}"#.into())
        );
    }

    #[test]
    fn preview_migrates_profile_catalog_in_its_persisted_preferences_envelope() {
        let workspace = crate::WorkspaceId::from("legacy-profile-store");
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-profile-envelope-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{}.json", workspace.0));
        let store =
            crate::WorkspacePreferencesStore::new(&directory, crate::PersistenceLimits::default());

        let legacy_catalog: Value =
            serde_json::from_str(&fixture("profile_catalog_schema_v0.json")).unwrap();
        let mut envelope = serde_json::json!({
            "schema_version": 1,
            "workspace_id": workspace.0,
            "preferences": legacy_catalog,
            "host_metadata": { "owner": "legacy-host" }
        });
        envelope["checksum"] =
            Value::String(crate::checksum_layout_json(&envelope.to_string()).unwrap());
        let source = serde_json::to_string(&envelope).unwrap();
        fs::write(&path, &source).unwrap();

        let preview =
            preview_persisted_migration(PersistedDataFormat::ProfileCatalog, &source).unwrap();
        assert_eq!(preview.source_version, 0);
        assert_eq!(
            preview.target_version,
            WorkspaceProfileCatalog::CURRENT_SCHEMA as u64
        );
        assert!(preview.requires_migration);
        let migrated: Value = serde_json::from_str(&preview.target_json).unwrap();
        assert_eq!(migrated["workspace_id"], workspace.0);
        assert_eq!(migrated["host_metadata"]["owner"], "legacy-host");
        assert_eq!(migrated["preferences"]["schema_version"], 1);
        crate::persistence::validate_workspace_preferences_envelope(
            &preview.target_json,
            crate::PersistenceLimits::default(),
        )
        .unwrap();

        let mut corrupt = envelope.clone();
        corrupt["preferences"]["profiles"][0]["title_key"] = Value::String("tampered".into());
        let corrupt_bytes = serde_json::to_string(&corrupt).unwrap();
        assert_eq!(
            preview_persisted_migration(PersistedDataFormat::ProfileCatalog, &corrupt_bytes)
                .unwrap_err()
                .code,
            ErrorCode::SerializationError
        );
        assert_eq!(
            serde_json::from_str::<Value>(&corrupt_bytes).unwrap()["preferences"]["profiles"][0]["title_key"],
            "tampered"
        );

        let mut future = envelope.clone();
        future["preferences"]["schema_version"] = Value::from(2);
        future["checksum"] =
            Value::String(crate::checksum_layout_json(&future.to_string()).unwrap());
        assert_eq!(
            preview_persisted_migration(PersistedDataFormat::ProfileCatalog, &future.to_string())
                .unwrap_err()
                .code,
            ErrorCode::SchemaError
        );
        assert_eq!(fs::read(&path).unwrap(), source.as_bytes());

        fs::write(&path, &preview.target_json).unwrap();
        let loaded = crate::load_profile_catalog(&store, &workspace).unwrap();
        assert_eq!(
            loaded.schema_version,
            WorkspaceProfileCatalog::CURRENT_SCHEMA
        );
        assert_eq!(
            loaded.profiles["legacy-workbench"].title_key,
            "Legacy workbench"
        );
        assert_eq!(loaded.profiles["legacy-workbench"].regions[0].id, "main");
        assert!(
            !preview_persisted_migration(PersistedDataFormat::ProfileCatalog, &preview.target_json)
                .unwrap()
                .requires_migration
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn previous_commit_profile_store_fixture_loads_through_current_adapter() {
        let source = fixture("profile_store_source_f59ee94.json");
        let preview = preview_persisted_migration(PersistedDataFormat::ProfileCatalog, &source)
            .expect("historical saved-profile envelope should remain readable");
        assert_eq!(preview.source_version, 1);
        assert_eq!(
            preview.target_version,
            WorkspaceProfileCatalog::CURRENT_SCHEMA as u64
        );
        assert!(!preview.requires_migration);
        crate::persistence::validate_workspace_preferences_envelope(
            &source,
            PersistenceLimits::default(),
        )
        .unwrap();

        let workspace = crate::WorkspaceId::from("fixture-host");
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-prior-profile-fixture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("fixture-host.json"), source).unwrap();
        let store = crate::WorkspacePreferencesStore::new(&directory, PersistenceLimits::default());
        let catalog = crate::load_profile_catalog(&store, &workspace).unwrap();
        assert!(catalog.profiles.contains_key("workbench"));
        assert_eq!(catalog.profiles.len(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preview_migrates_layout_serialized_by_original_rust_baseline() {
        let source = fixture("layout_schema_v1_baseline_6987eff.json");
        let preview = preview_persisted_migration(PersistedDataFormat::Layout, &source).unwrap();
        assert_eq!(preview.source_version, 1);
        assert_eq!(preview.target_version, crate::CURRENT_LAYOUT_SCHEMA as u64);
        assert!(preview.requires_migration);

        let migrated =
            crate::deserialize_layout(&preview.target_json, crate::PersistenceLimits::default())
                .unwrap();
        assert_eq!(migrated.schema_version, crate::CURRENT_LAYOUT_SCHEMA);
        assert_eq!(migrated.id.0, "workspace-default");
        assert_eq!(migrated.workspace.0, "default");
        assert_eq!(migrated.revision, 12);
        assert_eq!(migrated.panels.len(), 2);
        assert_eq!(migrated.panels[0].id.0, "console");
        assert_eq!(migrated.panels[1].id.0, "editor");
        assert_eq!(migrated.root.as_ref().unwrap().id.0, "previous-root-editor");
        let floating = &migrated.floating_windows[0];
        assert_eq!(floating.id.0, "previous-float-console");
        assert_eq!(
            floating.geometry,
            crate::Rect {
                x: 120,
                y: 80,
                width: 720,
                height: 480
            }
        );
        assert_eq!(floating.monitor_id, "display-left");
        assert_eq!(floating.scale, 1.25);
        assert_eq!(floating.z_order_hint, 3);
        assert_eq!(floating.root.id.0, "previous-float-console-panel");

        let repeated =
            preview_persisted_migration(PersistedDataFormat::Layout, &preview.target_json).unwrap();
        assert!(!repeated.requires_migration);
        assert_eq!(repeated.target_json, preview.target_json);
    }

    #[test]
    fn preview_rejects_future_schema_and_oversized_input() {
        let future = r#"{"schema_version":3}"#;
        let error =
            preview_persisted_migration(PersistedDataFormat::ProfileCatalog, future).unwrap_err();
        assert_eq!(error.code, ErrorCode::SchemaError);

        let oversized = " ".repeat(PersistedDataFormat::WindowRules.maximum_bytes() + 1);
        let error =
            preview_persisted_migration(PersistedDataFormat::WindowRules, &oversized).unwrap_err();
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn preview_theme_outputs_current_manifest_without_writing_files() {
        let source = include_str!("../../../themes/doofusdock-noctua/theme.json").to_owned();
        let before = source.clone();
        let preview =
            preview_persisted_migration(PersistedDataFormat::ThemePackage, &source).unwrap();
        assert_eq!(preview.source_version, 1);
        assert_eq!(preview.target_version, crate::THEME_SCHEMA_VERSION as u64);
        assert!(preview.requires_migration);
        assert_eq!(source, before);
    }

    #[test]
    fn previous_commit_theme_manifest_fixture_keeps_its_tokens_and_variants() {
        let source = fixture("theme_manifest_source_05da1c4.json");
        let preview =
            preview_persisted_migration(PersistedDataFormat::ThemePackage, &source).unwrap();
        assert_eq!(preview.source_version, 1);
        assert_eq!(preview.target_version, crate::THEME_SCHEMA_VERSION as u64);
        assert!(preview.requires_migration);
        let historical = crate::ThemeManifest::parse_json(&source).unwrap();
        let migrated = crate::ThemeManifest::parse_json(&preview.target_json).unwrap();
        assert_eq!(historical.id, "doofusdock-noctua");
        assert_eq!(migrated.tokens, historical.tokens);
        assert_eq!(migrated.variants, historical.variants);
        assert_eq!(migrated.components, historical.components);
    }

    #[test]
    fn preview_preferences_preserves_checksummed_user_metadata_and_rejects_corruption() {
        let mut envelope = serde_json::json!({
            "schema_version": 1,
            "workspace_id": "workspace-1",
            "preferences": {"extension.custom": {"user_value": "retain-me"}},
            "host_metadata": {"owner": "consumer"}
        });
        let checksum = crate::checksum_layout_json(&envelope.to_string()).unwrap();
        envelope["checksum"] = Value::String(checksum);
        let source = envelope.to_string();
        let preview =
            preview_persisted_migration(PersistedDataFormat::WorkspacePreferences, &source)
                .unwrap();
        assert_eq!(preview.source_version, 1);
        assert!(!preview.requires_migration);
        assert_eq!(
            serde_json::from_str::<Value>(&preview.target_json).unwrap(),
            envelope
        );

        envelope["preferences"]["extension.custom"]["user_value"] = Value::String("changed".into());
        let corrupted = envelope.to_string();
        let error =
            preview_persisted_migration(PersistedDataFormat::WorkspacePreferences, &corrupted)
                .unwrap_err();
        assert_eq!(error.code, ErrorCode::SerializationError);
    }

    #[test]
    fn preview_validates_persisted_window_rule_envelope_and_preserves_its_checksum() {
        let rules = WindowRuleSet {
            schema_version: 1,
            rules: Vec::new(),
        };
        let source = String::from_utf8(crate::rules::encode_rule_file(&rules).unwrap()).unwrap();
        let preview =
            preview_persisted_migration(PersistedDataFormat::WindowRules, &source).unwrap();
        assert_eq!(preview.source_version, 1);
        assert!(!preview.requires_migration);
        assert_eq!(
            crate::rules::decode_window_rule_file_json(&preview.target_json).unwrap(),
            rules
        );

        let mut corrupt: Value = serde_json::from_str(&source).unwrap();
        corrupt["checksum"] = Value::String("0000000000000000".into());
        let error =
            preview_persisted_migration(PersistedDataFormat::WindowRules, &corrupt.to_string())
                .unwrap_err();
        assert_eq!(error.code, ErrorCode::PersistenceError);
    }

    #[test]
    fn previous_commit_window_rule_store_fixture_loads_through_current_adapter() {
        let source = fixture("window_rules_store_source_c7e7804.json");
        let preview =
            preview_persisted_migration(PersistedDataFormat::WindowRules, &source).unwrap();
        assert_eq!(preview.source_version, 1);
        assert_eq!(preview.target_version, 1);
        assert!(!preview.requires_migration);
        assert_eq!(preview.source_fingerprint, fingerprint(source.as_bytes()));

        let workspace = crate::WorkspaceId::from("fixture-host");
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-prior-rule-fixture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("window-rules-fixture-host.json"), &source).unwrap();
        let store = crate::WindowRuleStore::new(&directory);
        let rules = store.load(&workspace).unwrap();
        assert_eq!(rules.rules.len(), 1);
        assert_eq!(rules.rules[0].id, "prior-rule");
        assert!(matches!(
            rules.rules[0].actions.as_slice(),
            [crate::WindowRuleAction::SetPinned { pinned: true }]
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preview_workspace_catalog_validates_and_serializes_current_catalog() {
        let catalog = WorkspaceCatalog::default();
        let source = catalog.to_json().unwrap();
        let preview =
            preview_persisted_migration(PersistedDataFormat::WorkspaceCatalog, &source).unwrap();
        assert_eq!(
            preview.source_version,
            WorkspaceCatalog::CURRENT_SCHEMA as u64
        );
        assert!(!preview.requires_migration);
        assert_eq!(
            WorkspaceCatalog::from_json(&preview.target_json, MIGRATION_PREVIEW_LIMIT).unwrap(),
            catalog
        );
    }

    #[test]
    fn preview_workspace_catalog_accepts_its_persisted_preferences_envelope() {
        let owner = crate::WorkspaceId::from("workspace-catalog-owner");
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-workspace-envelope-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{}.json", owner.0));
        let store =
            crate::WorkspacePreferencesStore::new(&directory, crate::PersistenceLimits::default());

        let mut catalog = WorkspaceCatalog::default();
        catalog
            .create(crate::WorkspaceId::from("default"), "Default")
            .unwrap();
        let catalog_value = serde_json::to_value(&catalog).unwrap();
        let mut envelope = serde_json::json!({
            "schema_version": 1,
            "workspace_id": owner.0,
            "preferences": catalog_value,
            "host_metadata": { "owner": "workspace-host" }
        });
        envelope["checksum"] =
            Value::String(crate::checksum_layout_json(&envelope.to_string()).unwrap());
        let source = serde_json::to_string(&envelope).unwrap();
        fs::write(&path, &source).unwrap();

        let preview =
            preview_persisted_migration(PersistedDataFormat::WorkspaceCatalog, &source).unwrap();
        assert_eq!(
            preview.source_version,
            WorkspaceCatalog::CURRENT_SCHEMA as u64
        );
        assert_eq!(
            preview.target_version,
            WorkspaceCatalog::CURRENT_SCHEMA as u64
        );
        assert!(!preview.requires_migration);
        let migrated: Value = serde_json::from_str(&preview.target_json).unwrap();
        assert_eq!(migrated["host_metadata"]["owner"], "workspace-host");
        crate::persistence::validate_workspace_preferences_envelope(
            &preview.target_json,
            crate::PersistenceLimits::default(),
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), source.as_bytes());
        assert_eq!(
            crate::load_workspace_catalog(&store, &owner, MIGRATION_PREVIEW_LIMIT).unwrap(),
            catalog
        );
        fs::write(&path, &preview.target_json).unwrap();
        assert_eq!(
            crate::load_workspace_catalog(&store, &owner, MIGRATION_PREVIEW_LIMIT).unwrap(),
            catalog
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
