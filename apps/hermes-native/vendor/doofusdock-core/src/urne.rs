//! Lossless URNE v2 projection of canonical DoofusDock layouts.
//!
//! URNE is a projection format, not the authoritative layout store.  This module
//! keeps the layout document authoritative and emits the existing URNE envelope
//! (`nodes`, `edges`, `diagnostics`, `sources`) with the shared UI-composition
//! node vocabulary. DoofusDock-specific layout state lives in node metadata.
use crate::{DockError, LayoutDocument, Node, NodeKind, NodePtr, validate_layout};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};

pub const URNE_ENGINE: &str = "urne";
pub const URNE_VERSION: u32 = 2;
const SOURCE: &str = "doofusdock.layout";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UrneNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub state: String,
    pub source: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source_ref: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parent_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub child_ids: Vec<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub metadata: Map<String, Value>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub provenance: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UrneEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub metadata: Map<String, Value>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub provenance: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UrneDiagnostic {
    pub code: String,
    pub message: String,
    pub source: String,
    pub node_id: String,
}

/// Shape-compatible with Doofus Hermes `ProjectionResult` v1.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UrneProjection {
    /// Version of the serialized projection envelope.
    pub version: u32,
    pub nodes: Vec<UrneNode>,
    pub edges: Vec<UrneEdge>,
    pub diagnostics: Vec<UrneDiagnostic>,
    pub sources: BTreeMap<String, String>,
}

impl UrneProjection {
    /// Produce a deterministic URNE graph from a validated canonical layout.
    /// UI objects use URNE's workspace/layout/region/panel/window vocabulary.
    pub fn from_layout(document: &LayoutDocument) -> Result<Self, DockError> {
        validate_layout(document)?;
        let workspace_id = format!("workspace:{}", hex_key(&document.workspace.0));
        let scope = format!(
            "{}:{}",
            hex_key(&document.workspace.0),
            hex_key(&document.id.0)
        );
        let layout_id = format!("layout:{scope}");
        let mut projection = Self {
            version: URNE_VERSION,
            nodes: Vec::new(),
            edges: Vec::new(),
            diagnostics: Vec::new(),
            sources: BTreeMap::from([(
                SOURCE.to_owned(),
                "validated canonical LayoutDocument (read-only projection)".to_owned(),
            )]),
        };
        let mut workspace_metadata = Map::new();
        workspace_metadata.insert("dock_kind".into(), Value::String("workspace".into()));
        workspace_metadata.insert("source_status".into(), Value::String("active".into()));
        workspace_metadata.insert(
            "workspace_id".into(),
            Value::String(document.workspace.0.clone()),
        );
        projection.nodes.push(urne_node(
            "workspace",
            workspace_id.clone(),
            document.workspace.0.clone(),
            document.workspace.0.clone(),
            vec![],
            vec![layout_id.clone()],
            workspace_metadata,
        ));
        let mut metadata = Map::new();
        metadata.insert("dock_kind".into(), Value::String("layout".into()));
        metadata.insert("source_status".into(), Value::String("canonical".into()));
        metadata.insert(
            "workspace_id".into(),
            Value::String(document.workspace.0.clone()),
        );
        metadata.insert("layout_id".into(), Value::String(document.id.0.clone()));
        metadata.insert(
            "layout_schema_version".into(),
            Value::from(document.schema_version),
        );
        metadata.insert("revision".into(), Value::from(document.revision));
        projection.nodes.push(urne_node(
            "layout",
            layout_id.clone(),
            document.id.0.clone(),
            document.id.0.clone(),
            vec![],
            vec![],
            metadata,
        ));

        let panels: BTreeMap<_, _> = document.panels.iter().map(|p| (&p.id, p)).collect();
        for panel in document.panels.iter() {
            let id = panel_id(&scope, &panel.id.0);
            let mut metadata = Map::new();
            metadata.insert("dock_kind".into(), Value::String("panel".into()));
            metadata.insert("source_status".into(), Value::String("registered".into()));
            metadata.insert("panel_id".into(), Value::String(panel.id.0.clone()));
            metadata.insert("title_key".into(), Value::String(panel.title_key.clone()));
            metadata.insert("icon_key".into(), Value::String(panel.icon_key.clone()));
            metadata.insert(
                "provider_key".into(),
                Value::String(panel.provider_key.clone()),
            );
            metadata.insert(
                "minimum".into(),
                serde_json::json!([panel.minimum.width, panel.minimum.height]),
            );
            metadata.insert(
                "maximum".into(),
                serde_json::json!([panel.maximum.width, panel.maximum.height]),
            );
            metadata.insert("close_policy".into(), Value::from(panel.close_policy as u8));
            metadata.insert(
                "persistence_policy".into(),
                Value::from(panel.persistence as u8),
            );
            metadata.insert(
                "reparent_capability".into(),
                Value::from(panel.reparent as u8),
            );
            metadata.insert("docking".into(), serde_json::json!({
                "allow_tab": panel.docking.allow_tab,
                "allow_split": panel.docking.allow_split,
                "allow_float": panel.docking.allow_float,
                "allow_auto_hide": panel.docking.allow_auto_hide,
                "allowed_edges": panel.docking.allowed_edges.iter().map(|edge| edge_name(*edge)).collect::<Vec<_>>(),
            }));
            projection.nodes.push(urne_node(
                "panel",
                id,
                panel.title_key.clone(),
                panel.id.0.clone(),
                vec![],
                vec![],
                metadata,
            ));
        }
        projection
            .edges
            .push(edge(&workspace_id, &layout_id, Map::new()));

        if let Some(root) = &document.root {
            project_node(root, &scope, &panels, &mut projection)?;
            let root_id = node_id(&scope, root);
            projection
                .edges
                .push(edge(&layout_id, &root_id, Map::new()));
        }
        for (index, window) in document.floating_windows.iter().enumerate() {
            let window_id = format!("window:{scope}:{}", hex_key(&window.id.0));
            let mut metadata = Map::new();
            metadata.insert("dock_kind".into(), Value::String("floating_window".into()));
            metadata.insert("source_status".into(), Value::String("open".into()));
            metadata.insert("window_id".into(), Value::String(window.id.0.clone()));
            metadata.insert(
                "geometry".into(),
                serde_json::json!([
                    window.geometry.x,
                    window.geometry.y,
                    window.geometry.width,
                    window.geometry.height
                ]),
            );
            metadata.insert(
                "monitor_id".into(),
                Value::String(window.monitor_id.clone()),
            );
            metadata.insert("scale".into(), Value::from(window.scale));
            metadata.insert("z_order_hint".into(), Value::from(window.z_order_hint));
            if let Some(scratchpad) = &window.scratchpad {
                metadata.insert(
                    "scratchpad_name".into(),
                    Value::String(scratchpad.name.clone()),
                );
                metadata.insert("scratchpad_visible".into(), Value::Bool(scratchpad.visible));
                metadata.insert(
                    "scratchpad_auto_hide".into(),
                    Value::Bool(scratchpad.auto_hide_on_deactivate),
                );
            }
            projection.nodes.push(urne_node(
                "window",
                window_id.clone(),
                format!("Floating window {}", index + 1),
                window.id.0.clone(),
                vec![layout_id.clone()],
                vec![],
                metadata,
            ));
            projection
                .edges
                .push(edge(&layout_id, &window_id, Map::new()));
            project_node(&window.root, &scope, &panels, &mut projection)?;
            let root_id = node_id(&scope, &window.root);
            projection
                .edges
                .push(edge(&window_id, &root_id, Map::new()));
        }
        for closed in &document.closed_panels {
            add_panel_state(&mut projection, &scope, closed, "closed")?;
            projection
                .edges
                .push(edge(&layout_id, &panel_id(&scope, &closed.0), Map::new()));
        }
        for hidden in &document.auto_hide_panels {
            add_panel_state(&mut projection, &scope, &hidden.panel, "auto_hide")?;
            projection.edges.push(edge(
                &layout_id,
                &panel_id(&scope, &hidden.panel.0),
                Map::new(),
            ));
            if let Some(panel_node) = projection
                .nodes
                .iter_mut()
                .find(|n| n.id == panel_id(&scope, &hidden.panel.0))
            {
                panel_node.metadata.insert(
                    "auto_hide_edge".into(),
                    Value::String(format!("{:?}", hidden.edge).to_lowercase()),
                );
            }
        }
        if let Some(panel) = &document.maximized_panel
            && let Some(layout) = projection.nodes.iter_mut().find(|n| n.id == layout_id)
        {
            layout
                .metadata
                .insert("maximized_panel_id".into(), Value::String(panel.0.clone()));
        }
        normalize_links(&mut projection);
        projection.nodes.sort_by(|a, b| {
            (a.node_type.as_str(), a.id.as_str()).cmp(&(b.node_type.as_str(), b.id.as_str()))
        });
        projection.edges.sort_by(|a, b| {
            (a.edge_type.as_str(), a.source.as_str(), a.target.as_str()).cmp(&(
                b.edge_type.as_str(),
                b.source.as_str(),
                b.target.as_str(),
            ))
        });
        projection.diagnostics.sort_by(|a, b| {
            (&a.code, &a.node_id, &a.message).cmp(&(&b.code, &b.node_id, &b.message))
        });
        Ok(projection)
    }

    /// Add a validated window-rule policy to a workspace projection. Rules
    /// remain a separate policy source and are never presented as committed
    /// layout facts.
    pub fn with_window_rules(
        mut self,
        workspace: &crate::WorkspaceId,
        rules: &crate::WindowRuleSet,
    ) -> Result<Self, DockError> {
        rules.validate()?;
        let workspace_node = self.nodes.iter().find(|node| {
            node.metadata.get("dock_kind").and_then(Value::as_str) == Some("workspace")
                && node.metadata.get("workspace_id").and_then(Value::as_str)
                    == Some(workspace.0.as_str())
        });
        let Some(workspace_node) = workspace_node else {
            return Err(DockError::new(
                crate::ErrorCode::NotFound,
                "Window-rule projection requires the matching workspace node.",
                &workspace.0,
            ));
        };
        let workspace_node_id = workspace_node.id.clone();
        let set_id = format!("window_rule_set:{}", hex_key(&workspace.0));
        if self.nodes.iter().any(|node| node.id == set_id) {
            return Err(DockError::new(
                crate::ErrorCode::Conflict,
                "A window-rule set is already attached to this workspace projection.",
                &workspace.0,
            ));
        }
        let mut set_metadata = Map::new();
        set_metadata.insert("dock_kind".into(), Value::String("window_rule_set".into()));
        set_metadata.insert(
            "source_status".into(),
            Value::String("validated_policy".into()),
        );
        set_metadata.insert("workspace_id".into(), Value::String(workspace.0.clone()));
        set_metadata.insert("schema_version".into(), Value::from(rules.schema_version));
        set_metadata.insert("rule_count".into(), Value::from(rules.rules.len()));
        let mut set_node = urne_node(
            "policy_set",
            set_id.clone(),
            format!("Window rules ({})", workspace.0),
            workspace.0.clone(),
            vec![],
            vec![],
            set_metadata,
        );
        set_node.source = "doofusdock.window_rules".into();
        self.nodes.push(set_node);
        self.edges
            .push(typed_edge(&workspace_node_id, &set_id, "has_policy"));

        let rule_ids = rules
            .rules
            .iter()
            .map(|rule| {
                (
                    rule.id.as_str(),
                    format!(
                        "window_rule:{}:{}",
                        hex_key(&workspace.0),
                        hex_key(&rule.id)
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for rule in &rules.rules {
            let id = &rule_ids[rule.id.as_str()];
            let mut metadata = Map::new();
            metadata.insert("dock_kind".into(), Value::String("window_rule".into()));
            metadata.insert("enabled".into(), Value::Bool(rule.enabled));
            metadata.insert("priority".into(), Value::from(rule.priority));
            metadata.insert(
                "predicate".into(),
                serde_json::to_value(&rule.predicate).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::SerializationError,
                        "Window rule predicate projection failed.",
                        error.to_string(),
                    )
                })?,
            );
            metadata.insert(
                "event_kinds".into(),
                serde_json::to_value(&rule.event_kinds).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::SerializationError,
                        "Window rule event triggers could not be projected.",
                        error.to_string(),
                    )
                })?,
            );
            metadata.insert(
                "actions".into(),
                serde_json::to_value(&rule.actions).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::SerializationError,
                        "Window rule action projection failed.",
                        error.to_string(),
                    )
                })?,
            );
            metadata.insert(
                "override_ids".into(),
                serde_json::to_value(&rule.overrides).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::SerializationError,
                        "Window rule override projection failed.",
                        error.to_string(),
                    )
                })?,
            );
            let mut rule_node = urne_node(
                "policy",
                id.clone(),
                rule.id.clone(),
                rule.id.clone(),
                vec![],
                vec![],
                metadata,
            );
            rule_node.source = "doofusdock.window_rules".into();
            self.nodes.push(rule_node);
            self.edges.push(typed_edge(&set_id, id, "contains_rule"));
        }
        for rule in &rules.rules {
            let source = &rule_ids[rule.id.as_str()];
            for overridden in &rule.overrides {
                let target = &rule_ids[overridden.as_str()];
                self.edges
                    .push(typed_edge(source, target, "overrides_rule"));
            }
        }
        self.sources.insert(
            "doofusdock.window_rules".into(),
            "validated declarative policy definitions (read-only projection)".into(),
        );
        finish_projection(&mut self, "window-rule-projection")?;
        Ok(self)
    }

    /// Attach workspace lifecycle metadata separately from each canonical
    /// workspace layout tree.
    pub fn with_workspace_catalog(
        mut self,
        catalog: &crate::WorkspaceCatalog,
    ) -> Result<Self, DockError> {
        catalog.validate()?;
        let catalog_id = "workspace_catalog:v1".to_owned();
        if self.nodes.iter().any(|node| node.id == catalog_id) {
            return Err(DockError::new(
                crate::ErrorCode::Conflict,
                "A workspace catalog is already attached to this projection.",
                catalog_id,
            ));
        }
        let mut metadata = Map::new();
        metadata.insert(
            "dock_kind".into(),
            Value::String("workspace_catalog".into()),
        );
        metadata.insert("schema_version".into(), Value::from(catalog.schema_version));
        metadata.insert(
            "workspace_count".into(),
            Value::from(catalog.workspaces.len()),
        );
        metadata.insert(
            "active_workspace_id".into(),
            catalog
                .active
                .as_ref()
                .map_or(Value::Null, |workspace| Value::String(workspace.0.clone())),
        );
        self.nodes.push(urne_node(
            "workspace_catalog",
            catalog_id.clone(),
            "Workspaces".into(),
            "workspace_catalog".into(),
            vec![],
            vec![],
            metadata,
        ));
        for (order, id) in catalog.order.iter().enumerate() {
            let workspace = &catalog.workspaces[id];
            let node_id = format!("workspace_context:{}", hex_key(&id.0));
            let mut metadata = Map::new();
            metadata.insert(
                "dock_kind".into(),
                Value::String("workspace_context".into()),
            );
            metadata.insert("workspace_id".into(), Value::String(id.0.clone()));
            metadata.insert("archived".into(), Value::Bool(workspace.archived));
            metadata.insert("order".into(), Value::from(order));
            if let Some(template) = &workspace.template_source {
                metadata.insert("template_source".into(), Value::String(template.0.clone()));
            }
            self.nodes.push(urne_node(
                "workspace_context",
                node_id.clone(),
                workspace.title.clone(),
                id.0.clone(),
                vec![],
                vec![],
                metadata,
            ));
            self.edges
                .push(typed_edge(&catalog_id, &node_id, "contains_workspace"));
            if let Some(template) = &workspace.template_source {
                let source_id = format!("workspace_context:{}", hex_key(&template.0));
                self.edges
                    .push(typed_edge(&node_id, &source_id, "uses_layout_template"));
            }
        }
        self.sources.insert(
            "doofusdock.workspaces".into(),
            "validated workspace lifecycle catalog (read-only projection)".into(),
        );
        finish_projection(&mut self, "workspace-catalog-projection")?;
        Ok(self)
    }

    /// Attach a bounded immutable event batch to the matching workspace as a
    /// separate operational-facts projection. These event nodes are derived
    /// observations, never canonical layout state.
    pub fn with_framework_events(
        mut self,
        workspace: &crate::WorkspaceId,
        events: &[crate::FrameworkEvent],
    ) -> Result<Self, DockError> {
        if events.len() > 256 {
            return Err(DockError::new(
                crate::ErrorCode::ResourceLimit,
                "Framework event projection exceeds its 256-event limit.",
                events.len().to_string(),
            ));
        }
        let workspace_node = self.nodes.iter().find(|node| {
            node.metadata.get("dock_kind").and_then(Value::as_str) == Some("workspace")
                && node.metadata.get("workspace_id").and_then(Value::as_str)
                    == Some(workspace.0.as_str())
        });
        let Some(workspace_node) = workspace_node else {
            return Err(DockError::new(
                crate::ErrorCode::NotFound,
                "Framework event projection requires the matching workspace node.",
                &workspace.0,
            ));
        };
        let workspace_node_id = workspace_node.id.clone();
        let stream_id = format!("framework_event_stream:{}", hex_key(&workspace.0));
        if self.nodes.iter().any(|node| node.id == stream_id) {
            return Err(DockError::new(
                crate::ErrorCode::Conflict,
                "Framework events are already attached to this workspace projection.",
                &workspace.0,
            ));
        }
        let mut sequence_ids = HashSet::new();
        for event in events {
            if event.id.0 == 0
                || event.sequence == 0
                || event.id.0 != event.sequence
                || event.revision == 0
                || event.command.is_empty()
                || event.command.0.len() > 128
                || !sequence_ids.insert(event.sequence)
                || event.payload.len() > 32
                || event.payload.iter().any(|(key, value)| {
                    key.is_empty()
                        || key.len() > 64
                        || !key
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || "_.:-".contains(ch))
                        || value.len() > 512
                        || value.chars().any(char::is_control)
                })
            {
                return Err(DockError::new(
                    crate::ErrorCode::SchemaError,
                    "Framework event identity or payload is invalid.",
                    event.sequence.to_string(),
                ));
            }
        }
        let mut stream_metadata = Map::new();
        stream_metadata.insert(
            "dock_kind".into(),
            Value::String("framework_event_stream".into()),
        );
        stream_metadata.insert(
            "source_status".into(),
            Value::String("immutable_facts".into()),
        );
        stream_metadata.insert("workspace_id".into(), Value::String(workspace.0.clone()));
        stream_metadata.insert("event_count".into(), Value::from(events.len()));
        let mut stream = urne_node(
            "event_stream",
            stream_id.clone(),
            format!("Framework events ({})", workspace.0),
            workspace.0.clone(),
            vec![],
            vec![],
            stream_metadata,
        );
        stream.source = "doofusdock.events".into();
        self.nodes.push(stream);
        self.edges.push(typed_edge(
            &workspace_node_id,
            &stream_id,
            "has_event_stream",
        ));
        for event in events {
            let id = format!(
                "framework_event:{}:{}",
                hex_key(&workspace.0),
                event.sequence
            );
            let mut metadata = Map::new();
            metadata.insert("dock_kind".into(), Value::String("framework_event".into()));
            metadata.insert("event_id".into(), Value::from(event.id.0));
            metadata.insert("sequence".into(), Value::from(event.sequence));
            metadata.insert("revision".into(), Value::from(event.revision));
            metadata.insert(
                "event_kind".into(),
                Value::String(framework_event_kind_token(event.kind).into()),
            );
            metadata.insert("command_id".into(), Value::String(event.command.0.clone()));
            metadata.insert(
                "payload".into(),
                serde_json::to_value(&event.payload).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::SerializationError,
                        "Framework event payload projection failed.",
                        error.to_string(),
                    )
                })?,
            );
            let mut node = urne_node(
                "event",
                id.clone(),
                format!(
                    "{} · r{}",
                    framework_event_kind_token(event.kind),
                    event.revision
                ),
                event.command.0.clone(),
                vec![],
                vec![],
                metadata,
            );
            node.state = "committed".into();
            node.source = "doofusdock.events".into();
            self.nodes.push(node);
            self.edges
                .push(typed_edge(&stream_id, &id, "contains_event"));
        }
        self.sources.insert(
            "doofusdock.events".into(),
            "bounded immutable facts polled from the canonical service (read-only projection)"
                .into(),
        );
        finish_projection(&mut self, "framework-event-projection")?;
        Ok(self)
    }

    /// Project registered extension capabilities together with an explicit host
    /// capability set. Host availability is a read-only assessment, not a
    /// promise that a provider is currently active or its callbacks will succeed.
    pub fn from_layout_with_capabilities(
        document: &LayoutDocument,
        registry: &crate::ExtensionRegistry,
        host_id: &str,
        host: &crate::HostCapabilities,
    ) -> Result<Self, DockError> {
        let mut projection = Self::from_layout(document)?;
        append_capability_projection(&mut projection, registry, host_id, host)?;
        finish_projection(&mut projection, &document.id.0)?;
        Ok(projection)
    }

    /// Project a structured host report alongside provider capabilities.
    /// Availability and verification evidence remain separate URNE fields.
    pub fn from_layout_with_host_report(
        document: &LayoutDocument,
        registry: &crate::ExtensionRegistry,
        host: &crate::HostCapabilities,
        report: &crate::HostCapabilityReport,
    ) -> Result<Self, DockError> {
        report.validate(host)?;
        let mut projection =
            Self::from_layout_with_capabilities(document, registry, &report.host_id, host)?;
        append_host_capability_report(&mut projection, report)?;
        finish_projection(&mut projection, &document.id.0)?;
        Ok(projection)
    }

    /// Add renderer capability facts supplied by an adapter. This method has
    /// no renderer dependency and never derives support from theme values.
    pub fn with_renderer_report(
        mut self,
        report: &crate::RendererCapabilityReport,
    ) -> Result<Self, DockError> {
        report.validate()?;
        append_renderer_capability_report(&mut self, report)?;
        finish_projection(&mut self, &report.renderer_id)?;
        Ok(self)
    }

    /// Add a bounded, schema-bearing scene-effect capability catalog. The
    /// projection records provider/effect/capability relationships without
    /// invoking an effect or claiming a renderer can present its output.
    pub fn with_effect_capability_report(
        mut self,
        report: &crate::EffectCapabilityReport,
    ) -> Result<Self, DockError> {
        report.validate()?;
        append_effect_capability_report(&mut self, report)?;
        finish_projection(&mut self, &report.provider_id)?;
        Ok(self)
    }

    /// Add a document-bound operation preflight result to this projection.
    /// A stale result cannot be attached to a different layout revision.
    pub fn with_operation_preflight_report(
        mut self,
        report: &crate::OperationPreflightReport,
    ) -> Result<Self, DockError> {
        report.validate()?;
        append_operation_preflight(&mut self, report)?;
        finish_projection(&mut self, &report.document_id)?;
        Ok(self)
    }

    /// Project only provider/host capability relationships when an application
    /// has no current layout document to inspect.
    pub fn from_capabilities(
        registry: &crate::ExtensionRegistry,
        host_id: &str,
        host: &crate::HostCapabilities,
    ) -> Result<Self, DockError> {
        let mut projection = Self {
            version: URNE_VERSION,
            nodes: Vec::new(),
            edges: Vec::new(),
            diagnostics: Vec::new(),
            sources: BTreeMap::new(),
        };
        append_capability_projection(&mut projection, registry, host_id, host)?;
        finish_projection(&mut projection, host_id)?;
        Ok(projection)
    }

    /// Project a saved profile catalog and its semantic regions without
    /// deriving profile data from a currently instantiated layout.
    pub fn from_profile_catalog(
        catalog: &crate::WorkspaceProfileCatalog,
    ) -> Result<Self, DockError> {
        crate::validate_profile_catalog(catalog)?;
        let catalog_id = format!("profile-catalog:v{}", catalog.schema_version);
        let mut projection = Self {
            version: URNE_VERSION,
            nodes: vec![urne_node(
                "profile_catalog",
                catalog_id.clone(),
                "Workspace profiles".into(),
                catalog_id.clone(),
                vec![],
                vec![],
                serde_json::json!({
                    "dock_kind": "profile_catalog",
                    "schema_version": catalog.schema_version,
                    "default_profile": catalog.default_profile,
                    "profile_count": catalog.profiles.len(),
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            )],
            edges: Vec::new(),
            diagnostics: Vec::new(),
            sources: BTreeMap::from([(
                "doofusdock.profiles".to_owned(),
                "validated WorkspaceProfileCatalog (read-only projection)".to_owned(),
            )]),
        };
        for profile in catalog.profiles.values() {
            let profile_id = format!("layout_profile:{}", hex_key(&profile.id));
            let mut metadata = Map::new();
            metadata.insert("dock_kind".into(), Value::String("layout_profile".into()));
            metadata.insert("profile_id".into(), Value::String(profile.id.clone()));
            metadata.insert("version".into(), Value::from(profile.version));
            metadata.insert("category".into(), Value::String(profile.category.clone()));
            metadata.insert("title_key".into(), Value::String(profile.title_key.clone()));
            metadata.insert(
                "is_default".into(),
                Value::Bool(catalog.default_profile.as_deref() == Some(profile.id.as_str())),
            );
            metadata.insert(
                "presentation".into(),
                serde_json::to_value(&profile.presentation).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::InvalidLayout,
                        "Profile presentation could not be projected.",
                        error.to_string(),
                    )
                })?,
            );
            metadata.insert(
                "layout_specification".into(),
                serde_json::to_value(&profile.layout).map_err(|error| {
                    DockError::new(
                        crate::ErrorCode::InvalidLayout,
                        "Profile layout specification could not be projected.",
                        error.to_string(),
                    )
                })?,
            );
            projection.nodes.push(urne_node(
                "layout_profile",
                profile_id.clone(),
                profile.title_key.clone(),
                profile.id.clone(),
                vec![catalog_id.clone()],
                vec![],
                metadata,
            ));
            projection
                .edges
                .push(typed_edge(&catalog_id, &profile_id, "contains"));
            for region in &profile.regions {
                let region_id = format!(
                    "profile_region:{}:{}",
                    hex_key(&profile.id),
                    hex_key(&region.id)
                );
                let metadata = serde_json::json!({
                    "dock_kind": "semantic_region",
                    "region_id": region.id,
                    "required": region.required,
                    "title_key": region.title_key,
                })
                .as_object()
                .cloned()
                .unwrap_or_default();
                projection.nodes.push(urne_node(
                    "semantic_region",
                    region_id.clone(),
                    region.title_key.clone(),
                    region.id.clone(),
                    vec![profile_id.clone()],
                    vec![],
                    metadata,
                ));
                projection
                    .edges
                    .push(typed_edge(&profile_id, &region_id, "declares_region"));
            }
        }
        if projection.nodes.len() > crate::theme::MAX_URNE_NODES {
            return Err(DockError::new(
                crate::ErrorCode::ResourceLimit,
                "Profile URNE projection exceeds 4096 nodes.",
                catalog_id,
            ));
        }
        finish_projection(&mut projection, "profile-catalog")?;
        Ok(projection)
    }

    /// Project a validated theme package and its deterministic resolved snapshot.
    /// The manifest remains authoritative; binary asset bytes are never copied into URNE.
    pub fn from_theme(
        manifest: &crate::ThemeManifest,
        resolved: &crate::ResolvedTheme,
    ) -> Result<Self, crate::ThemeError> {
        manifest.validate()?;
        resolved.validate_snapshot()?;
        if resolved.theme_id != manifest.id
            || resolved.manifest_fingerprint != manifest.fingerprint()?
            || !manifest.variants.contains_key(&resolved.variant_id)
        {
            return Err(crate::ThemeError::new(
                crate::ThemeErrorCode::IncompatibleSnapshot,
                "Resolved theme snapshot does not match this validated manifest.",
                manifest.id.clone(),
            ));
        }
        let theme_id = format!("theme:{}", hex_key(&manifest.id));
        let mut projection = Self {
            version: URNE_VERSION,
            nodes: Vec::new(),
            edges: Vec::new(),
            diagnostics: Vec::new(),
            sources: BTreeMap::from([(
                "doofusdock.theme.package".into(),
                "validated versioned theme manifest (read-only projection)".into(),
            )]),
        };

        let mut package_metadata = serde_json::to_value(&manifest.metadata)
            .map_err(|error| theme_urne_error(error.to_string(), "metadata"))?
            .as_object()
            .cloned()
            .unwrap_or_default();
        package_metadata.insert(
            "schema_version".into(),
            Value::from(manifest.schema_version),
        );
        package_metadata.insert(
            "manifest_fingerprint".into(),
            Value::String(resolved.manifest_fingerprint.clone()),
        );
        package_metadata.insert(
            "resolved_variant".into(),
            Value::String(resolved.variant_id.clone()),
        );
        package_metadata.insert(
            "resolved_fingerprint".into(),
            Value::String(resolved.fingerprint.clone()),
        );
        package_metadata.insert(
            "provenance".into(),
            serde_json::to_value(&manifest.provenance)
                .map_err(|error| theme_urne_error(error.to_string(), "provenance"))?,
        );
        projection.nodes.push(theme_urne_node(
            "theme_package",
            theme_id.clone(),
            manifest.metadata.name.clone(),
            manifest.id.clone(),
            package_metadata,
        ));

        for (variant_id, variant) in &manifest.variants {
            let variant_node_id = format!(
                "theme-variant:{}:{}",
                hex_key(&manifest.id),
                hex_key(variant_id)
            );
            let mut metadata = Map::new();
            metadata.insert("variant_id".into(), Value::String(variant_id.clone()));
            metadata.insert("label".into(), Value::String(variant.label.clone()));
            metadata.insert(
                "extends".into(),
                variant
                    .extends
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            metadata.insert(
                "selected".into(),
                Value::Bool(variant_id == &resolved.variant_id),
            );
            projection.nodes.push(theme_urne_node(
                "theme_variant",
                variant_node_id.clone(),
                variant.label.clone(),
                variant_id.clone(),
                metadata,
            ));
            projection
                .edges
                .push(edge(&theme_id, &variant_node_id, Map::new()));
            for (token_id, value) in &variant.tokens {
                let node_id = format!(
                    "theme-token:{}:variant:{}:{}",
                    hex_key(&manifest.id),
                    hex_key(variant_id),
                    hex_key(token_id)
                );
                let mut metadata = Map::new();
                metadata.insert("token_id".into(), Value::String(token_id.clone()));
                metadata.insert("layer".into(), Value::String(variant_id.clone()));
                metadata.insert(
                    "value".into(),
                    serde_json::to_value(value)
                        .map_err(|error| theme_urne_error(error.to_string(), token_id))?,
                );
                projection.nodes.push(theme_urne_node(
                    "theme_token",
                    node_id.clone(),
                    token_id.clone(),
                    token_id.clone(),
                    metadata,
                ));
                projection
                    .edges
                    .push(edge(&variant_node_id, &node_id, Map::new()));
            }
        }

        for (token_id, value) in &manifest.tokens {
            let node_id = format!(
                "theme-token:{}:base:{}",
                hex_key(&manifest.id),
                hex_key(token_id)
            );
            let mut metadata = Map::new();
            metadata.insert("token_id".into(), Value::String(token_id.clone()));
            metadata.insert("layer".into(), Value::String("base".into()));
            metadata.insert(
                "value".into(),
                serde_json::to_value(value)
                    .map_err(|error| theme_urne_error(error.to_string(), token_id))?,
            );
            projection.nodes.push(theme_urne_node(
                "theme_token",
                node_id.clone(),
                token_id.clone(),
                token_id.clone(),
                metadata,
            ));
            projection.edges.push(edge(&theme_id, &node_id, Map::new()));
        }

        for (component_id, component) in &manifest.components {
            let component_node_id = format!(
                "theme-component:{}:base:{}",
                hex_key(&manifest.id),
                hex_key(component_id)
            );
            let mut metadata = Map::new();
            metadata.insert("component_id".into(), Value::String(component_id.clone()));
            metadata.insert("layer".into(), Value::String("base".into()));
            projection.nodes.push(theme_urne_node(
                "theme_component",
                component_node_id.clone(),
                component_id.clone(),
                component_id.clone(),
                metadata,
            ));
            projection
                .edges
                .push(edge(&theme_id, &component_node_id, Map::new()));
            add_theme_states(
                &mut projection,
                &manifest.id,
                "base",
                component_id,
                component,
                &component_node_id,
            )?;
        }
        for (variant_id, variant) in &manifest.variants {
            let parent_id = format!(
                "theme-variant:{}:{}",
                hex_key(&manifest.id),
                hex_key(variant_id)
            );
            for (component_id, component) in &variant.components {
                let component_node_id = format!(
                    "theme-component:{}:{}:{}",
                    hex_key(&manifest.id),
                    hex_key(variant_id),
                    hex_key(component_id)
                );
                let mut metadata = Map::new();
                metadata.insert("component_id".into(), Value::String(component_id.clone()));
                metadata.insert("layer".into(), Value::String(variant_id.clone()));
                projection.nodes.push(theme_urne_node(
                    "theme_component",
                    component_node_id.clone(),
                    component_id.clone(),
                    component_id.clone(),
                    metadata,
                ));
                projection
                    .edges
                    .push(edge(&parent_id, &component_node_id, Map::new()));
                add_theme_states(
                    &mut projection,
                    &manifest.id,
                    variant_id,
                    component_id,
                    component,
                    &component_node_id,
                )?;
            }
        }

        for (parameter_id, parameter) in &manifest.parameters {
            let node_id = format!(
                "theme-parameter:{}:{}",
                hex_key(&manifest.id),
                hex_key(parameter_id)
            );
            let mut metadata = serde_json::to_value(parameter)
                .map_err(|error| theme_urne_error(error.to_string(), parameter_id))?
                .as_object()
                .cloned()
                .unwrap_or_default();
            metadata.insert("parameter_id".into(), Value::String(parameter_id.clone()));
            projection.nodes.push(theme_urne_node(
                "theme_parameter",
                node_id.clone(),
                parameter_id.clone(),
                parameter_id.clone(),
                metadata,
            ));
            projection.edges.push(edge(&theme_id, &node_id, Map::new()));
        }
        for asset in &manifest.assets {
            let node_id = format!(
                "theme-asset:{}:{}",
                hex_key(&manifest.id),
                hex_key(&asset.id)
            );
            let mut metadata = serde_json::to_value(asset)
                .map_err(|error| theme_urne_error(error.to_string(), &asset.id))?
                .as_object()
                .cloned()
                .unwrap_or_default();
            metadata.remove("provenance");
            metadata.insert("asset_id".into(), Value::String(asset.id.clone()));
            metadata.insert(
                "provenance".into(),
                asset
                    .provenance
                    .as_ref()
                    .map(|source| serde_json::to_value(source).unwrap_or(Value::Null))
                    .unwrap_or(Value::Null),
            );
            metadata.insert(
                "resolved_enabled".into(),
                Value::Bool(
                    resolved
                        .assets
                        .iter()
                        .any(|active| active.id == asset.id && active.enabled),
                ),
            );
            projection.nodes.push(theme_urne_node(
                "theme_asset",
                node_id.clone(),
                asset.id.clone(),
                asset.path.clone(),
                metadata,
            ));
            projection.edges.push(edge(&theme_id, &node_id, Map::new()));
        }
        for dependency in &manifest.dependencies {
            let node_id = format!(
                "theme-dependency:{}:{}",
                hex_key(&manifest.id),
                hex_key(&dependency.id)
            );
            let mut metadata = serde_json::to_value(dependency)
                .map_err(|error| theme_urne_error(error.to_string(), &dependency.id))?
                .as_object()
                .cloned()
                .unwrap_or_default();
            metadata.insert("dependency_id".into(), Value::String(dependency.id.clone()));
            projection.nodes.push(theme_urne_node(
                "theme_dependency",
                node_id.clone(),
                dependency.id.clone(),
                dependency.id.clone(),
                metadata,
            ));
            projection.edges.push(edge(&theme_id, &node_id, Map::new()));
        }

        let validation_id = format!(
            "theme-validation:{}:{}",
            hex_key(&manifest.id),
            hex_key(&resolved.variant_id)
        );
        let mut validation_metadata = Map::new();
        validation_metadata.insert(
            "variant_id".into(),
            Value::String(resolved.variant_id.clone()),
        );
        validation_metadata.insert(
            "contrast".into(),
            serde_json::to_value(&resolved.contrast)
                .map_err(|error| theme_urne_error(error.to_string(), "contrast"))?,
        );
        validation_metadata.insert(
            "diagnostics".into(),
            serde_json::to_value(&resolved.diagnostics)
                .map_err(|error| theme_urne_error(error.to_string(), "diagnostics"))?,
        );
        validation_metadata.insert(
            "value_sources".into(),
            serde_json::to_value(&resolved.value_sources)
                .map_err(|error| theme_urne_error(error.to_string(), "value_sources"))?,
        );
        validation_metadata.insert(
            "resolved_tokens".into(),
            serde_json::to_value(&resolved.tokens)
                .map_err(|error| theme_urne_error(error.to_string(), "resolved_tokens"))?,
        );
        validation_metadata.insert(
            "resolved_components".into(),
            serde_json::to_value(&resolved.components)
                .map_err(|error| theme_urne_error(error.to_string(), "resolved_components"))?,
        );
        validation_metadata.insert(
            "parameter_values".into(),
            serde_json::to_value(&resolved.parameters)
                .map_err(|error| theme_urne_error(error.to_string(), "parameter_values"))?,
        );
        validation_metadata.insert(
            "resolved_fingerprint".into(),
            Value::String(resolved.fingerprint.clone()),
        );
        projection.nodes.push(theme_urne_node(
            "theme_validation",
            validation_id.clone(),
            "Resolved theme validation".into(),
            resolved.fingerprint.clone(),
            validation_metadata,
        ));
        projection
            .edges
            .push(edge(&theme_id, &validation_id, Map::new()));

        if projection.nodes.len() > crate::theme::MAX_URNE_NODES {
            return Err(crate::ThemeError::new(
                crate::ThemeErrorCode::ResourceLimit,
                "Theme URNE projection exceeds 4096 nodes.",
                manifest.id.clone(),
            ));
        }
        for node in &mut projection.nodes {
            node.source = "doofusdock.theme".into();
        }
        projection.edges.sort_by(|a, b| {
            (a.edge_type.as_str(), a.source.as_str(), a.target.as_str()).cmp(&(
                b.edge_type.as_str(),
                b.source.as_str(),
                b.target.as_str(),
            ))
        });
        normalize_links(&mut projection);
        Ok(projection)
    }
}

fn theme_urne_error(message: String, context: &str) -> crate::ThemeError {
    crate::ThemeError::new(crate::ThemeErrorCode::InvalidJson, message, context)
}

fn theme_urne_node(
    node_type: &str,
    id: String,
    label: String,
    source_ref: String,
    metadata: Map<String, Value>,
) -> UrneNode {
    let mut node = urne_node(node_type, id, label, source_ref, vec![], vec![], metadata);
    node.source = "doofusdock.theme".into();
    node
}

fn add_theme_states(
    projection: &mut UrneProjection,
    theme_id: &str,
    layer: &str,
    component_id: &str,
    component: &crate::ThemeComponent,
    parent_id: &str,
) -> Result<(), crate::ThemeError> {
    for (state_id, state) in &component.states {
        let id = format!(
            "theme-state:{}:{}:{}:{}",
            hex_key(theme_id),
            hex_key(layer),
            hex_key(component_id),
            hex_key(state_id)
        );
        let metadata = serde_json::to_value(state)
            .map_err(|error| theme_urne_error(error.to_string(), state_id))?
            .as_object()
            .cloned()
            .unwrap_or_default();
        projection.nodes.push(theme_urne_node(
            "theme_state",
            id.clone(),
            state_id.clone(),
            state_id.clone(),
            metadata,
        ));
        projection.edges.push(edge(parent_id, &id, Map::new()));
    }
    Ok(())
}

fn framework_event_kind_token(kind: crate::FrameworkEventKind) -> &'static str {
    use crate::FrameworkEventKind as Kind;
    match kind {
        Kind::LayoutCommitted => "layout_committed",
        Kind::LayoutRestored => "layout_restored",
        Kind::PanelCreated => "panel_created",
        Kind::PanelClosed => "panel_closed",
        Kind::PanelVisibilityChanged => "panel_visibility_changed",
        Kind::PanelWorkspaceMembershipChanged => "panel_workspace_membership_changed",
        Kind::GroupMinimizedChanged => "group_minimized_changed",
        Kind::ScratchpadChanged => "scratchpad_changed",
        Kind::PanelMoved => "panel_moved",
        Kind::PanelDrawerChanged => "panel_drawer_changed",
        Kind::GroupChanged => "group_changed",
        Kind::WindowFloated => "window_floated",
        Kind::WindowDocked => "window_docked",
        Kind::WindowGeometryChanged => "window_geometry_changed",
        Kind::PictureInPictureChanged => "picture_in_picture_changed",
        Kind::WindowZOrderChanged => "window_z_order_changed",
        Kind::WindowMonitorChanged => "window_monitor_changed",
        Kind::FocusChanged => "focus_changed",
        Kind::NativeViewportFocused => "native_viewport_focused",
        Kind::NativeViewportCloseRequested => "native_viewport_close_requested",
        Kind::NativeViewportGeometryObserved => "native_viewport_geometry_observed",
        Kind::NativeRootViewportGeometryObserved => "native_root_viewport_geometry_observed",
        Kind::WorkspaceActivated => "workspace_activated",
        Kind::ModalityChanged => "modality_changed",
        Kind::MonitorChanged => "monitor_changed",
        Kind::TransitionStarted => "transition_started",
        Kind::TransitionCompleted => "transition_completed",
    }
}

fn urne_node(
    node_type: &str,
    id: String,
    label: String,
    source_ref: String,
    parent_ids: Vec<String>,
    child_ids: Vec<String>,
    metadata: Map<String, Value>,
) -> UrneNode {
    UrneNode {
        id,
        node_type: node_type.into(),
        label,
        state: "unknown".into(),
        source: SOURCE.into(),
        source_ref,
        parent_ids,
        child_ids,
        metadata,
        provenance: Map::new(),
    }
}
fn edge(source: &str, target: &str, metadata: Map<String, Value>) -> UrneEdge {
    UrneEdge {
        source: source.into(),
        target: target.into(),
        edge_type: "contains".into(),
        metadata,
        provenance: Map::new(),
    }
}
fn typed_edge(source: &str, target: &str, edge_type: &str) -> UrneEdge {
    UrneEdge {
        source: source.into(),
        target: target.into(),
        edge_type: edge_type.into(),
        metadata: Map::new(),
        provenance: Map::new(),
    }
}
fn finish_projection(projection: &mut UrneProjection, context: &str) -> Result<(), DockError> {
    if projection.nodes.len() > crate::theme::MAX_URNE_NODES
        || projection.edges.len() > crate::theme::MAX_URNE_NODES * 2
    {
        return Err(DockError::new(
            crate::ErrorCode::ResourceLimit,
            "URNE projection exceeds its bounded node or edge budget.",
            context,
        ));
    }
    projection.nodes.sort_by(|a, b| {
        (a.node_type.as_str(), a.id.as_str()).cmp(&(b.node_type.as_str(), b.id.as_str()))
    });
    projection.edges.sort_by(|a, b| {
        (a.edge_type.as_str(), a.source.as_str(), a.target.as_str()).cmp(&(
            b.edge_type.as_str(),
            b.source.as_str(),
            b.target.as_str(),
        ))
    });
    projection
        .diagnostics
        .sort_by(|a, b| (&a.code, &a.node_id, &a.message).cmp(&(&b.code, &b.node_id, &b.message)));
    normalize_links(projection);
    if !projection_is_acyclic(projection) {
        return Err(DockError::new(
            crate::ErrorCode::InvalidLayout,
            "URNE projection relationships must form an acyclic graph.",
            context,
        ));
    }
    Ok(())
}

fn projection_is_acyclic(projection: &UrneProjection) -> bool {
    let mut indegree = projection
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), 0usize))
        .collect::<BTreeMap<_, _>>();
    if indegree.len() != projection.nodes.len() {
        return false;
    }
    let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();
    for relation in &projection.edges {
        if !indegree.contains_key(relation.source.as_str()) {
            return false;
        }
        let Some(degree) = indegree.get_mut(relation.target.as_str()) else {
            return false;
        };
        *degree += 1;
        outgoing
            .entry(relation.source.as_str())
            .or_default()
            .push(relation.target.as_str());
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(id) = ready.pop() {
        visited += 1;
        for target in outgoing.get(id).into_iter().flatten() {
            let Some(degree) = indegree.get_mut(target) else {
                return false;
            };
            *degree -= 1;
            if *degree == 0 {
                ready.push(target);
            }
        }
    }
    visited == projection.nodes.len()
}

fn append_capability_projection(
    projection: &mut UrneProjection,
    registry: &crate::ExtensionRegistry,
    host_id: &str,
    host: &crate::HostCapabilities,
) -> Result<(), DockError> {
    host.validate()?;
    if host_id.is_empty() || host_id.len() > 4096 {
        return Err(DockError::new(
            crate::ErrorCode::InvalidId,
            "Capability projection requires a nonempty bounded host ID.",
            host_id,
        ));
    }
    projection.sources.insert(
        "doofusdock.capabilities".into(),
        "ExtensionRegistry declarations evaluated against supplied HostCapabilities".into(),
    );
    let host_node = format!("host:{}", hex_key(host_id));
    projection.nodes.push(urne_node(
        "host",
        host_node.clone(),
        host_id.into(),
        host_id.into(),
        vec![],
        vec![],
        serde_json::json!({
            "dock_kind": "host_capability_context",
            "host_id": host_id,
            "capability_count": host.supported.len(),
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    ));

    let mut capability_ids = BTreeMap::<String, String>::new();
    let mut operation_ids = HashSet::new();
    for name in &host.supported {
        let id = format!("capability:{}", hex_key(name));
        capability_ids.insert(name.clone(), id.clone());
        projection.nodes.push(urne_node(
            "capability",
            id.clone(),
            name.clone(),
            name.clone(),
            vec![host_node.clone()],
            vec![],
            serde_json::json!({
                "dock_kind": "host_capability",
                "capability_id": name,
                "available_on_host": true,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ));
        projection
            .edges
            .push(typed_edge(&host_node, &id, "supports"));
    }

    for snapshot in registry.capability_snapshot(host) {
        let provider_id = format!("provider:{}", hex_key(&snapshot.id));
        let missing: Vec<_> = snapshot.missing_capabilities.clone();
        let mut metadata = serde_json::to_value(&snapshot)
            .map_err(|error| {
                DockError::new(
                    crate::ErrorCode::InvalidLayout,
                    "Extension capability snapshot could not be serialized.",
                    error.to_string(),
                )
            })?
            .as_object()
            .cloned()
            .unwrap_or_default();
        metadata.insert(
            "dock_kind".into(),
            Value::String("extension_provider".into()),
        );
        projection.nodes.push(urne_node(
            "extension_provider",
            provider_id.clone(),
            snapshot.id.clone(),
            snapshot.id.clone(),
            vec![host_node.clone()],
            vec![],
            metadata,
        ));
        projection
            .edges
            .push(typed_edge(&host_node, &provider_id, "evaluates"));
        if snapshot.kind == crate::ExtensionKind::Transition {
            let transition_id = format!("transition:{}", hex_key(&snapshot.id));
            let host_availability = match snapshot.supported_on_host {
                Some(true) => "supported",
                Some(false) => "unsupported",
                None => "not_assessed",
            };
            projection.nodes.push(urne_node(
                "transition_capability",
                transition_id.clone(),
                snapshot.id.clone(),
                snapshot.id.clone(),
                vec![provider_id.clone()],
                vec![],
                serde_json::json!({
                    "dock_kind": "registered_transition",
                    "transition_id": snapshot.id,
                    "provider_id": snapshot.id,
                    "provider_version": snapshot.version,
                    "operation": "duration_ms",
                    "request_fields": ["duration_ms", "reduced_motion", "host_capabilities"],
                    "request_schema": {
                        "type": "object",
                        "required": ["duration_ms", "reduced_motion", "host_capabilities"],
                        "additionalProperties": false,
                        "properties": {
                            "duration_ms": {"type": "integer", "minimum": 0, "maximum": 60000},
                            "reduced_motion": {"type": "boolean"},
                            "host_capabilities": {"type": "array", "items": {"type": "string", "maxLength": 256}, "maxItems": 128}
                        }
                    },
                    "host_availability": host_availability,
                    "missing_host_capabilities": snapshot.missing_capabilities.clone(),
                    "host_reason_unavailable": snapshot.reason_unavailable.clone(),
                    "callback_invoked": false,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            ));
            projection.edges.push(typed_edge(
                &provider_id,
                &transition_id,
                "provides_transition",
            ));
        }
        for (values, relationship) in [
            (&snapshot.provided_capabilities, "provides"),
            (&snapshot.required_capabilities, "requires"),
            (&snapshot.incompatible_capabilities, "incompatible_with"),
        ] {
            for name in values {
                let capability_id = capability_ids.entry(name.clone()).or_insert_with(|| {
                    let id = format!("capability:{}", hex_key(name));
                    projection.nodes.push(urne_node(
                        "capability",
                        id.clone(),
                        name.clone(),
                        name.clone(),
                        vec![],
                        vec![],
                        serde_json::json!({
                            "dock_kind": "declared_capability",
                            "capability_id": name,
                            "available_on_host": host.supported.contains(name),
                        })
                        .as_object()
                        .cloned()
                        .unwrap_or_default(),
                    ));
                    id
                });
                projection
                    .edges
                    .push(typed_edge(&provider_id, capability_id, relationship));
            }
        }
        for operation in &snapshot.operations {
            let operation_id = format!("operation:{}", hex_key(operation));
            if operation_ids.insert(operation_id.clone()) {
                projection.nodes.push(urne_node(
                    "operation",
                    operation_id.clone(),
                    operation.clone(),
                    operation.clone(),
                    vec![],
                    vec![],
                    serde_json::json!({
                        "dock_kind": "registered_operation",
                        "operation_id": operation,
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
                ));
            }
            projection.edges.push(typed_edge(
                &provider_id,
                &operation_id,
                "supports_operation",
            ));
        }
        for name in missing {
            projection.diagnostics.push(UrneDiagnostic {
                code: "missing_host_capability".into(),
                message: format!(
                    "Host {host_id} lacks required capability {name} for {}.",
                    snapshot.id
                ),
                source: "doofusdock.capabilities".into(),
                node_id: provider_id.clone(),
            });
        }
    }
    append_builtin_operation_projection(projection, &host_node, host)?;
    Ok(())
}

fn append_builtin_operation_projection(
    projection: &mut UrneProjection,
    host_node: &str,
    host: &crate::HostCapabilities,
) -> Result<(), DockError> {
    projection.sources.insert(
        "doofusdock.builtin_operations".into(),
        "built-in operation families evaluated against supplied HostCapabilities".into(),
    );
    let operation_catalog = crate::builtin_operation_catalog(host)?;
    for support in crate::builtin_operation_support(host)? {
        let mut operation_schemas = Map::new();
        for name in &support.operation_names {
            let descriptor = operation_catalog
                .iter()
                .find(|entry| entry.name == *name)
                .ok_or_else(|| {
                    DockError::new(
                        crate::ErrorCode::InvalidLayout,
                        "Built-in operation catalog is missing a support-family member.",
                        name,
                    )
                })?;
            operation_schemas.insert(name.clone(), descriptor.request_schema.clone());
        }
        let support_id = format!(
            "builtin-operation-support:{}:{}",
            host_node.strip_prefix("host:").unwrap_or_default(),
            hex_key(&support.id)
        );
        projection.nodes.push(urne_node(
            "builtin_operation_support",
            support_id.clone(),
            support.id.clone(),
            support.id.clone(),
            vec![host_node.to_owned()],
            vec![],
            serde_json::json!({
                "dock_kind": "builtin_operation_support",
                "operation_family_id": support.id,
                "operation_names": support.operation_names,
                "core_available": support.core_available,
                "host_available": support.host_available,
                "required_host_capabilities": support.required_host_capabilities,
                "required_any_host_capabilities": support.required_any_host_capabilities,
                "missing_host_capabilities": support.missing_host_capabilities,
                "applicable_target_kinds": support.applicable_target_kinds,
                "constraints": support.constraints,
                "operation_schemas": operation_schemas,
                "request_encoding": {
                    "format": "serde_adjacent_tagged_enum",
                    "discriminator": "operation",
                    "arguments_field": "arguments",
                    "unknown_fields": "rejected",
                },
                "reason": support.reason,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ));
        projection.edges.push(typed_edge(
            host_node,
            &support_id,
            "evaluates_operation_support",
        ));
        for name in &support.operation_names {
            let operation_id = format!("builtin-operation:{}", hex_key(name));
            if !projection.nodes.iter().any(|node| node.id == operation_id) {
                projection.nodes.push(urne_node(
                    "operation",
                    operation_id.clone(),
                    name.clone(),
                    name.clone(),
                    vec![],
                    vec![],
                    serde_json::json!({
                        "dock_kind": "builtin_operation",
                        "operation_id": name,
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
                ));
            }
            projection
                .edges
                .push(typed_edge(&support_id, &operation_id, "supports_operation"));
        }
        for capability in &support.required_host_capabilities {
            let capability_id = format!("capability:{}", hex_key(capability));
            if !projection.nodes.iter().any(|node| node.id == capability_id) {
                projection.nodes.push(urne_node(
                    "capability",
                    capability_id.clone(),
                    capability.clone(),
                    capability.clone(),
                    vec![],
                    vec![],
                    serde_json::json!({
                        "dock_kind": "declared_capability",
                        "capability_id": capability,
                        "available_on_host": host.supported.contains(capability),
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
                ));
            }
            projection.edges.push(typed_edge(
                &support_id,
                &capability_id,
                "requires_host_capability",
            ));
        }
        if !support.host_available {
            projection.diagnostics.push(UrneDiagnostic {
                code: "built_in_operation_unavailable_on_host".into(),
                message: support.reason,
                source: "doofusdock.builtin_operations".into(),
                node_id: support_id,
            });
        }
    }
    Ok(())
}

fn append_host_capability_report(
    projection: &mut UrneProjection,
    report: &crate::HostCapabilityReport,
) -> Result<(), DockError> {
    projection.sources.insert(
        "doofusdock.host_diagnostics".into(),
        "Host API availability paired with explicit synthetic or physical evidence".into(),
    );
    let host_id = format!("host:{}", hex_key(&report.host_id));
    let Some(host_node) = projection.nodes.iter_mut().find(|node| node.id == host_id) else {
        return Err(DockError::new(
            crate::ErrorCode::NotFound,
            "Host diagnostic report has no matching capability context.",
            &report.host_id,
        ));
    };
    host_node.metadata.insert(
        "diagnostic_count".into(),
        Value::from(report.observations.len()),
    );
    host_node
        .metadata
        .insert("diagnostic_schema_version".into(), Value::from(1));

    for observation in &report.observations {
        let capability_node_id = format!("capability:{}", hex_key(&observation.capability_id));
        if let Some(capability_node) = projection
            .nodes
            .iter_mut()
            .find(|node| node.id == capability_node_id)
        {
            capability_node.metadata.insert(
                "availability".into(),
                Value::String(availability_name(observation.availability).into()),
            );
            capability_node.metadata.insert(
                "verification_evidence".into(),
                Value::String(evidence_name(observation.evidence).into()),
            );
            capability_node.metadata.insert(
                "evidence_details".into(),
                Value::String(observation.details.clone()),
            );
        } else {
            projection.nodes.push(urne_node(
                "capability",
                capability_node_id.clone(),
                observation.capability_id.clone(),
                observation.capability_id.clone(),
                vec![host_id.clone()],
                vec![],
                serde_json::json!({
                    "dock_kind": "host_capability_observation",
                    "capability_id": observation.capability_id,
                    "available_on_host": observation.availability == crate::HostCapabilityAvailability::Available,
                    "availability": availability_name(observation.availability),
                    "verification_evidence": evidence_name(observation.evidence),
                    "evidence_details": observation.details,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            ));
            projection.edges.push(typed_edge(
                &host_id,
                &capability_node_id,
                "reports_capability",
            ));
        }

        let diagnostic_id = format!(
            "host-capability-report:{}:{}",
            hex_key(&report.host_id),
            hex_key(&observation.capability_id)
        );
        projection.nodes.push(urne_node(
            "host_capability_diagnostic",
            diagnostic_id.clone(),
            observation.capability_id.clone(),
            observation.capability_id.clone(),
            vec![host_id.clone()],
            vec![],
            serde_json::json!({
                "dock_kind": "host_capability_diagnostic",
                "capability_id": observation.capability_id,
                "availability": availability_name(observation.availability),
                "verification_evidence": evidence_name(observation.evidence),
                "details": observation.details,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ));
        projection
            .edges
            .push(typed_edge(&host_id, &diagnostic_id, "reports"));
        projection.edges.push(typed_edge(
            &diagnostic_id,
            &capability_node_id,
            "describes_capability",
        ));
    }
    Ok(())
}

fn append_renderer_capability_report(
    projection: &mut UrneProjection,
    report: &crate::RendererCapabilityReport,
) -> Result<(), DockError> {
    projection.sources.insert(
        "doofusdock.renderer_capabilities".into(),
        "renderer adapter declarations (read-only capability report)".into(),
    );
    let renderer_id = format!("renderer:{}", hex_key(&report.renderer_id));
    if projection.nodes.iter().any(|node| node.id == renderer_id) {
        return Err(DockError::new(
            crate::ErrorCode::Conflict,
            "Renderer capability report is already present in this URNE projection.",
            &report.renderer_id,
        ));
    }
    projection.nodes.push(urne_node(
        "renderer",
        renderer_id.clone(),
        report.renderer_id.clone(),
        report.renderer_id.clone(),
        vec![],
        vec![],
        serde_json::json!({
            "dock_kind": "renderer_capability_context",
            "renderer_id": report.renderer_id,
            "capability_count": report.observations.len(),
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    ));
    for observation in &report.observations {
        let availability = renderer_availability_name(observation.availability);
        let capability_id = format!(
            "renderer-capability:{}:{}",
            hex_key(&report.renderer_id),
            hex_key(&observation.capability_id)
        );
        projection.nodes.push(urne_node(
            "renderer_capability",
            capability_id.clone(),
            observation.capability_id.clone(),
            observation.capability_id.clone(),
            vec![renderer_id.clone()],
            vec![],
            serde_json::json!({
                "dock_kind": "renderer_capability",
                "capability_id": observation.capability_id,
                "availability": availability,
                "details": observation.details,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ));
        projection.edges.push(typed_edge(
            &renderer_id,
            &capability_id,
            "reports_capability",
        ));
        if observation.availability != crate::RendererCapabilityAvailability::Available {
            let diagnostic_id = format!(
                "renderer-diagnostic:{}:{}",
                hex_key(&report.renderer_id),
                hex_key(&observation.capability_id)
            );
            projection.nodes.push(urne_node(
                "renderer_capability_diagnostic",
                diagnostic_id.clone(),
                observation.capability_id.clone(),
                observation.capability_id.clone(),
                vec![renderer_id.clone()],
                vec![],
                serde_json::json!({
                    "dock_kind": "renderer_capability_diagnostic",
                    "availability": availability,
                    "details": observation.details,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            ));
            projection.edges.push(typed_edge(
                &renderer_id,
                &diagnostic_id,
                "reports_limitation",
            ));
            projection.edges.push(typed_edge(
                &diagnostic_id,
                &capability_id,
                "describes_capability",
            ));
            projection.diagnostics.push(UrneDiagnostic {
                code: match observation.availability {
                    crate::RendererCapabilityAvailability::Available => unreachable!(),
                    crate::RendererCapabilityAvailability::Unavailable => {
                        "renderer_capability_unavailable"
                    }
                    crate::RendererCapabilityAvailability::NotImplemented => {
                        "renderer_capability_not_implemented"
                    }
                }
                .into(),
                message: observation.details.clone(),
                source: "doofusdock.renderer_capabilities".into(),
                node_id: diagnostic_id,
            });
        }
    }
    Ok(())
}

fn append_effect_capability_report(
    projection: &mut UrneProjection,
    report: &crate::EffectCapabilityReport,
) -> Result<(), DockError> {
    projection.sources.insert(
        "doofusdock.effect_capabilities".into(),
        "presentation effect catalog (read-only schema and capability report)".into(),
    );
    let provider_id = format!("effect-provider:{}", hex_key(&report.provider_id));
    if projection.nodes.iter().any(|node| node.id == provider_id) {
        return Err(DockError::new(
            crate::ErrorCode::Conflict,
            "Effect capability report is already present in this URNE projection.",
            &report.provider_id,
        ));
    }
    projection.nodes.push(urne_node(
        "effect_provider",
        provider_id.clone(),
        report.provider_id.clone(),
        report.provider_id.clone(),
        vec![],
        vec![],
        serde_json::json!({
            "dock_kind": "scene_effect_provider",
            "provider_id": report.provider_id,
            "effect_count": report.observations.len(),
            "callback_invoked": false,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    ));

    let mut capability_ids = projection
        .nodes
        .iter()
        .filter(|node| node.node_type == "capability")
        .filter_map(|node| {
            node.metadata
                .get("capability_id")
                .and_then(Value::as_str)
                .map(|name| (name.to_owned(), node.id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    for observation in &report.observations {
        let effect_id = format!(
            "effect-capability:{}:{}",
            hex_key(&report.provider_id),
            hex_key(&observation.effect_id)
        );
        projection.nodes.push(urne_node(
            "effect_capability",
            effect_id.clone(),
            observation.effect_id.clone(),
            observation.effect_id.clone(),
            vec![provider_id.clone()],
            vec![],
            serde_json::json!({
                "dock_kind": "scene_effect_capability",
                "provider_id": report.provider_id,
                "effect_id": observation.effect_id,
                "availability": renderer_availability_name(observation.availability),
                "request_schema": observation.schema,
                "provided_capabilities": observation.provided_capabilities,
                "required_capabilities": observation.required_capabilities,
                "details": observation.details,
                "callback_invoked": false,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ));
        projection
            .edges
            .push(typed_edge(&provider_id, &effect_id, "provides_effect"));
        for (values, relationship) in [
            (&observation.provided_capabilities, "provides"),
            (&observation.required_capabilities, "requires"),
        ] {
            for name in values {
                let capability_id = capability_ids.entry(name.clone()).or_insert_with(|| {
                    let id = format!("capability:{}", hex_key(name));
                    projection.nodes.push(urne_node(
                        "capability",
                        id.clone(),
                        name.clone(),
                        name.clone(),
                        vec![],
                        vec![],
                        serde_json::json!({
                            "dock_kind": "declared_effect_capability",
                            "capability_id": name,
                        })
                        .as_object()
                        .cloned()
                        .unwrap_or_default(),
                    ));
                    id
                });
                projection
                    .edges
                    .push(typed_edge(&effect_id, capability_id, relationship));
            }
        }
    }
    Ok(())
}

fn append_operation_preflight(
    projection: &mut UrneProjection,
    report: &crate::OperationPreflightReport,
) -> Result<(), DockError> {
    let Some(layout_node) = projection.nodes.iter().find(|node| {
        node.node_type == "layout"
            && node.metadata.get("layout_id") == Some(&Value::String(report.document_id.clone()))
    }) else {
        return Err(DockError::new(
            crate::ErrorCode::NotFound,
            "Operation preflight document is not present in this projection.",
            &report.document_id,
        ));
    };
    if layout_node.metadata.get("revision") != Some(&Value::from(report.source_revision)) {
        return Err(DockError::new(
            crate::ErrorCode::Conflict,
            "Operation preflight belongs to a different layout revision.",
            format!(
                "report={}:projection={:?}",
                report.source_revision,
                layout_node.metadata.get("revision")
            ),
        ));
    }
    let layout_id = layout_node.id.clone();
    let preflight_id = format!(
        "operation-preflight:{}:{}:{}",
        hex_key(&report.document_id),
        report.source_revision,
        hex_key(&report.operation_id)
    );
    if projection.nodes.iter().any(|node| node.id == preflight_id) {
        return Err(DockError::new(
            crate::ErrorCode::Conflict,
            "Operation preflight report is already present in this projection.",
            &report.operation_id,
        ));
    }
    let operation_id = format!("builtin-operation:{}", hex_key(&report.operation_id));
    if !projection.nodes.iter().any(|node| node.id == operation_id) {
        projection.nodes.push(urne_node(
            "operation",
            operation_id.clone(),
            report.operation_id.clone(),
            report.operation_id.clone(),
            vec![],
            vec![],
            serde_json::json!({
                "dock_kind": "builtin_operation",
                "operation_id": report.operation_id,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        ));
    }
    projection.nodes.push(urne_node(
        "operation_preflight",
        preflight_id.clone(),
        report.operation_id.clone(),
        report.operation_id.clone(),
        vec![layout_id.clone()],
        vec![],
        serde_json::json!({
            "dock_kind": "operation_preflight",
            "document_id": report.document_id,
            "source_revision": report.source_revision,
            "operation_id": report.operation_id,
            "target_ids": report.target_ids,
            "core_available": report.core_available,
            "host_available": report.host_available,
            "supported": report.supported,
            "required_host_capabilities": report.required_host_capabilities,
            "required_any_host_capabilities": report.required_any_host_capabilities,
            "missing_host_capabilities": report.missing_host_capabilities,
            "rejection_code": report.rejection_code,
            "reason": report.reason,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    ));
    projection
        .edges
        .push(typed_edge(&layout_id, &preflight_id, "preflights"));
    projection
        .edges
        .push(typed_edge(&preflight_id, &operation_id, "checks_operation"));
    for capability in &report.missing_host_capabilities {
        let capability_id = format!("capability:{}", hex_key(capability));
        if !projection.nodes.iter().any(|node| node.id == capability_id) {
            projection.nodes.push(urne_node(
                "capability",
                capability_id.clone(),
                capability.clone(),
                capability.clone(),
                vec![],
                vec![],
                serde_json::json!({
                    "dock_kind": "declared_capability",
                    "capability_id": capability,
                    "available_on_host": false,
                })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            ));
        }
        projection.edges.push(typed_edge(
            &preflight_id,
            &capability_id,
            "missing_host_capability",
        ));
    }
    if !report.supported {
        let (code, source) = if !report.core_available {
            ("operation_preflight_rejected", "doofusdock.operations")
        } else {
            (
                "operation_host_capability_missing",
                "doofusdock.capabilities",
            )
        };
        projection.diagnostics.push(UrneDiagnostic {
            code: code.into(),
            message: report.reason.clone(),
            source: source.into(),
            node_id: preflight_id,
        });
    }
    Ok(())
}

fn renderer_availability_name(value: crate::RendererCapabilityAvailability) -> &'static str {
    match value {
        crate::RendererCapabilityAvailability::Available => "available",
        crate::RendererCapabilityAvailability::Unavailable => "unavailable",
        crate::RendererCapabilityAvailability::NotImplemented => "not_implemented",
    }
}

fn availability_name(value: crate::HostCapabilityAvailability) -> &'static str {
    match value {
        crate::HostCapabilityAvailability::Available => "available",
        crate::HostCapabilityAvailability::Unavailable => "unavailable",
        crate::HostCapabilityAvailability::NotImplemented => "not_implemented",
    }
}

fn evidence_name(value: crate::HostCapabilityEvidence) -> &'static str {
    match value {
        crate::HostCapabilityEvidence::ApiDeclared => "api_declared",
        crate::HostCapabilityEvidence::SyntheticContract => "synthetic_contract",
        crate::HostCapabilityEvidence::PhysicalSmokeTest => "physical_smoke_test",
        crate::HostCapabilityEvidence::NotVerified => "not_verified",
    }
}

fn node_id(scope: &str, node: &Node) -> String {
    format!("region:{scope}:node:{}", hex_key(&node.id.0))
}
fn panel_id(scope: &str, panel: &str) -> String {
    format!("panel:{scope}:{}", hex_key(panel))
}
fn hex_key(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn edge_name(edge: crate::DockEdge) -> &'static str {
    match edge {
        crate::DockEdge::Left => "left",
        crate::DockEdge::Right => "right",
        crate::DockEdge::Top => "top",
        crate::DockEdge::Bottom => "bottom",
    }
}
fn normalize_links(projection: &mut UrneProjection) {
    for node in &mut projection.nodes {
        node.parent_ids.clear();
        node.child_ids.clear();
    }
    let indexes: BTreeMap<_, _> = projection
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    for edge in &projection.edges {
        if let Some(parent_index) = indexes.get(&edge.source) {
            projection.nodes[*parent_index]
                .child_ids
                .push(edge.target.clone());
        }
        if let Some(child_index) = indexes.get(&edge.target) {
            projection.nodes[*child_index]
                .parent_ids
                .push(edge.source.clone());
        }
    }
    for node in &mut projection.nodes {
        node.parent_ids.sort();
        node.parent_ids.dedup();
        node.child_ids.sort();
        node.child_ids.dedup();
    }
}
fn add_panel_state(
    projection: &mut UrneProjection,
    scope: &str,
    panel: &crate::PanelId,
    status: &str,
) -> Result<(), DockError> {
    let id = panel_id(scope, &panel.0);
    let node = projection
        .nodes
        .iter_mut()
        .find(|n| n.id == id)
        .ok_or_else(|| {
            DockError::new(
                crate::ErrorCode::NotFound,
                "URNE projection references an unknown panel.",
                panel.0.clone(),
            )
        })?;
    node.metadata
        .insert("source_status".into(), Value::String(status.into()));
    Ok(())
}
fn project_node(
    node: &NodePtr,
    scope: &str,
    panels: &BTreeMap<&crate::PanelId, &crate::PanelDescriptor>,
    projection: &mut UrneProjection,
) -> Result<(), DockError> {
    let id = node_id(scope, node);
    let mut metadata = Map::new();
    metadata.insert(
        "dock_kind".into(),
        Value::String(
            match &node.kind {
                NodeKind::Panel { .. } => "panel_leaf",
                NodeKind::Tabs { .. } => "tabs",
                NodeKind::DocumentWell { .. } => "document_well",
                NodeKind::Split { .. } => "split",
                NodeKind::Placeholder { .. } => "placeholder",
                NodeKind::Extension { .. } => "extension",
            }
            .into(),
        ),
    );
    metadata.insert("source_status".into(), Value::String("present".into()));
    metadata.insert("layout_node_id".into(), Value::String(node.id.0.clone()));
    let (label, children): (String, Vec<NodePtr>) = match &node.kind {
        NodeKind::Panel { panel } => {
            let descriptor = panels.get(panel).ok_or_else(|| {
                DockError::new(
                    crate::ErrorCode::NotFound,
                    "Layout panel has no descriptor.",
                    panel.0.clone(),
                )
            })?;
            metadata.insert("panel_id".into(), Value::String(panel.0.clone()));
            (descriptor.title_key.clone(), vec![])
        }
        NodeKind::Tabs {
            active_index,
            items,
            minimized,
        } => {
            metadata.insert("active_index".into(), Value::from(*active_index));
            metadata.insert("minimized".into(), Value::from(*minimized));
            ("Tab group".into(), items.clone())
        }
        NodeKind::DocumentWell {
            active_group,
            groups,
        } => {
            metadata.insert("active_group".into(), Value::from(*active_group));
            ("Document groups".into(), groups.clone())
        }
        NodeKind::Split {
            orientation,
            weights,
            children,
        } => {
            metadata.insert(
                "orientation".into(),
                Value::String(format!("{:?}", orientation).to_lowercase()),
            );
            metadata.insert(
                "weights".into(),
                serde_json::to_value(weights).unwrap_or(Value::Null),
            );
            ("Split".into(), children.clone())
        }
        NodeKind::Placeholder { panel, title } => {
            metadata.insert("panel_id".into(), Value::String(panel.0.clone()));
            (title.clone(), vec![])
        }
        NodeKind::Extension {
            modality,
            payload,
            children,
        } => {
            metadata.insert("modality".into(), Value::String(modality.0.clone()));
            metadata.insert("payload".into(), Value::String(payload.clone()));
            ("Extension container".into(), children.clone())
        }
    };
    let child_ids: Vec<_> = children.iter().map(|child| node_id(scope, child)).collect();
    projection.nodes.push(urne_node(
        "region",
        id.clone(),
        label,
        node.id.0.clone(),
        vec![],
        child_ids.clone(),
        metadata,
    ));
    if let NodeKind::Panel { panel } = &node.kind {
        let target = panel_id(scope, &panel.0);
        let mut relation = Map::new();
        relation.insert("relation".into(), Value::String("panel_reference".into()));
        projection.edges.push(edge(&id, &target, relation));
    }
    for (child, child_id) in children.iter().zip(child_ids) {
        project_node(child, scope, panels, projection)?;
        projection.edges.push(edge(&id, &child_id, Map::new()));
    }
    Ok(())
}

#[cfg(test)]
mod projection_contract_tests {
    use super::*;

    #[test]
    fn finish_projection_rejects_cycles_and_dangling_relationships() {
        let mut cyclic = UrneProjection {
            version: URNE_VERSION,
            nodes: vec![
                urne_node(
                    "test",
                    "a".into(),
                    "A".into(),
                    "a".into(),
                    vec![],
                    vec![],
                    Map::new(),
                ),
                urne_node(
                    "test",
                    "b".into(),
                    "B".into(),
                    "b".into(),
                    vec![],
                    vec![],
                    Map::new(),
                ),
            ],
            edges: vec![typed_edge("a", "b", "links"), typed_edge("b", "a", "links")],
            diagnostics: Vec::new(),
            sources: BTreeMap::new(),
        };
        let cycle_error = finish_projection(&mut cyclic, "cycle-fixture").unwrap_err();
        assert_eq!(cycle_error.code, crate::ErrorCode::InvalidLayout);
        assert!(cycle_error.message.contains("acyclic"));

        let mut dangling = UrneProjection {
            version: URNE_VERSION,
            nodes: vec![urne_node(
                "test",
                "a".into(),
                "A".into(),
                "a".into(),
                vec![],
                vec![],
                Map::new(),
            )],
            edges: vec![typed_edge("a", "missing", "links")],
            diagnostics: Vec::new(),
            sources: BTreeMap::new(),
        };
        assert_eq!(
            finish_projection(&mut dangling, "dangling-fixture")
                .unwrap_err()
                .code,
            crate::ErrorCode::InvalidLayout
        );
    }

    #[test]
    fn window_rules_project_as_separate_deterministic_workspace_policy() {
        let document = LayoutDocument::new("layout", "workbench");
        let rules = crate::WindowRuleSet {
            schema_version: 1,
            rules: vec![crate::WindowRule {
                id: "pin-editor".into(),
                enabled: true,
                priority: 10,
                overrides: Vec::new(),
                event_kinds: [crate::FrameworkEventKind::PanelCreated].into(),
                predicate: crate::WindowRulePredicate {
                    panel_role: Some("editor".into()),
                    ..crate::WindowRulePredicate::default()
                },
                actions: vec![crate::WindowRuleAction::SetPinned { pinned: true }],
            }],
        };
        let first = UrneProjection::from_layout(&document)
            .unwrap()
            .with_window_rules(&document.workspace, &rules)
            .unwrap();
        let second = UrneProjection::from_layout(&document)
            .unwrap()
            .with_window_rules(&document.workspace, &rules)
            .unwrap();
        assert_eq!(first, second);
        assert!(first.sources.contains_key("doofusdock.window_rules"));
        let rule_node = first
            .nodes
            .iter()
            .find(|node| {
                node.metadata.get("dock_kind").and_then(Value::as_str) == Some("window_rule")
            })
            .unwrap();
        assert_eq!(rule_node.source, "doofusdock.window_rules");
        assert_eq!(
            rule_node.metadata.get("event_kinds"),
            Some(&serde_json::json!(["panel_created"]))
        );
        assert_eq!(
            rule_node.metadata.get("enabled").and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            first
                .edges
                .iter()
                .any(|edge| edge.edge_type == "contains_rule")
        );
        assert!(
            UrneProjection::from_layout(&document)
                .unwrap()
                .with_window_rules(&"missing".into(), &rules)
                .is_err()
        );
    }

    #[test]
    fn workspace_catalog_projects_as_deterministic_lifecycle_metadata() {
        let document = LayoutDocument::new("layout", "work");
        let mut catalog = crate::WorkspaceCatalog::default();
        let work = crate::WorkspaceId::from("work");
        catalog.create(work.clone(), "Work").unwrap();
        catalog
            .duplicate_layout_template(&work, "research".into(), "Research")
            .unwrap();
        let first = UrneProjection::from_layout(&document)
            .unwrap()
            .with_workspace_catalog(&catalog)
            .unwrap();
        let second = UrneProjection::from_layout(&document)
            .unwrap()
            .with_workspace_catalog(&catalog)
            .unwrap();
        assert_eq!(first, second);
        assert!(first.sources.contains_key("doofusdock.workspaces"));
        assert!(first.edges.iter().any(|edge| {
            edge.edge_type == "uses_layout_template"
                && edge.source.starts_with("workspace_context:")
        }));
        assert!(first.nodes.iter().any(|node| {
            node.metadata.get("workspace_id").and_then(Value::as_str) == Some("research")
                && node.metadata.get("template_source").and_then(Value::as_str) == Some("work")
        }));
    }

    #[test]
    fn framework_events_project_as_deterministic_immutable_workspace_facts() {
        let document = LayoutDocument::new("layout", "workbench");
        let mut hub = crate::EventHub::new(8).unwrap();
        let first = hub
            .publish(
                3,
                crate::FrameworkEventKind::PanelMoved,
                "move-panels".into(),
                BTreeMap::from([("panel".into(), "alpha".into())]),
            )
            .unwrap()
            .unwrap();
        let second = hub
            .publish(
                3,
                crate::FrameworkEventKind::PanelMoved,
                "move-panels".into(),
                BTreeMap::from([("panel".into(), "beta".into())]),
            )
            .unwrap()
            .unwrap();
        let geometry = hub
            .publish(
                3,
                crate::FrameworkEventKind::NativeViewportGeometryObserved,
                "native-geometry".into(),
                BTreeMap::from([
                    ("window".into(), "floating".into()),
                    ("geometry".into(), "-1280,120,640,480".into()),
                ]),
            )
            .unwrap()
            .unwrap();
        let root_geometry = hub
            .publish(
                3,
                crate::FrameworkEventKind::NativeRootViewportGeometryObserved,
                "native-root-geometry".into(),
                BTreeMap::from([("geometry".into(), "0,0,1280,720".into())]),
            )
            .unwrap()
            .unwrap();
        let transition_started = hub
            .publish(
                3,
                crate::FrameworkEventKind::TransitionStarted,
                "transition-start".into(),
                BTreeMap::from([("transition".into(), "morph-1".into())]),
            )
            .unwrap()
            .unwrap();
        let transition_completed = hub
            .publish(
                3,
                crate::FrameworkEventKind::TransitionCompleted,
                "transition-complete".into(),
                BTreeMap::from([("transition".into(), "morph-1".into())]),
            )
            .unwrap()
            .unwrap();
        let pip_changed = hub
            .publish(
                3,
                crate::FrameworkEventKind::PictureInPictureChanged,
                "pip-change".into(),
                BTreeMap::from([
                    ("window".into(), "floating".into()),
                    ("picture_in_picture".into(), "true".into()),
                ]),
            )
            .unwrap()
            .unwrap();
        let window_monitor_changed = hub
            .publish(
                3,
                crate::FrameworkEventKind::WindowMonitorChanged,
                "window-monitor-change".into(),
                BTreeMap::from([
                    ("window".into(), "floating".into()),
                    ("monitor".into(), "display-2".into()),
                    ("scale".into(), "1.25".into()),
                ]),
            )
            .unwrap()
            .unwrap();
        let events = [
            first,
            second,
            geometry,
            root_geometry,
            transition_started,
            transition_completed,
            pip_changed,
            window_monitor_changed,
        ];
        let first_projection = UrneProjection::from_layout(&document)
            .unwrap()
            .with_framework_events(&document.workspace, &events)
            .unwrap();
        let second_projection = UrneProjection::from_layout(&document)
            .unwrap()
            .with_framework_events(&document.workspace, &events)
            .unwrap();
        assert_eq!(first_projection, second_projection);
        assert!(first_projection.sources.contains_key("doofusdock.events"));
        let event_nodes = first_projection
            .nodes
            .iter()
            .filter(|node| {
                node.metadata.get("dock_kind").and_then(Value::as_str) == Some("framework_event")
            })
            .collect::<Vec<_>>();
        assert_eq!(event_nodes.len(), 8);
        assert!(event_nodes.iter().any(|node| {
            node.metadata.get("event_kind").and_then(Value::as_str) == Some("panel_moved")
                && node.metadata.get("revision").and_then(Value::as_u64) == Some(3)
        }));
        assert!(
            first_projection
                .edges
                .iter()
                .any(|edge| edge.edge_type == "contains_event")
        );
        for token in [
            "native_viewport_geometry_observed",
            "native_root_viewport_geometry_observed",
            "transition_started",
            "transition_completed",
            "picture_in_picture_changed",
            "window_monitor_changed",
        ] {
            assert!(event_nodes.iter().any(|node| {
                node.metadata.get("event_kind").and_then(Value::as_str) == Some(token)
            }));
        }
        assert!(
            UrneProjection::from_layout(&document)
                .unwrap()
                .with_framework_events(&"missing".into(), &events)
                .is_err()
        );

        let mut invalid = events.clone();
        invalid[0].payload.insert("bad\nkey".into(), "value".into());
        assert!(
            UrneProjection::from_layout(&document)
                .unwrap()
                .with_framework_events(&document.workspace, &invalid)
                .is_err()
        );
    }
}
