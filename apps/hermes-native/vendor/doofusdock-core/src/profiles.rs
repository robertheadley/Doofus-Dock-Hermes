//! Reusable, toolkit-independent workspace profile templates.
//!
//! A profile is an ephemeral recipe that instantiates the one canonical
//! `LayoutDocument`. It is not a second persisted widget tree. Semantic region
//! identity is encoded in stable node IDs and can be resolved after ordinary
//! layout edits by following the region's anchor panel.
use crate::{
    DockEdge, DockError, DockResult, ErrorCode, LayoutDocument, Node, NodeId, NodeKind, NodePtr,
    Orientation, PanelId, validate_layout,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRegion {
    pub id: String,
    pub title_key: String,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ProfileLayout {
    Region(String),
    Split {
        id: String,
        orientation: Orientation,
        weights: Vec<f64>,
        children: Vec<ProfileLayout>,
    },
    Tabs {
        id: String,
        active_index: usize,
        regions: Vec<ProfileLayout>,
    },
    /// Toolkit-neutral custom container declaration. The adapter/provider for
    /// `modality` owns geometry and interaction; the core preserves its payload.
    Extension {
        id: String,
        modality: String,
        payload: String,
        regions: Vec<ProfileLayout>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProfile {
    pub id: String,
    pub version: u32,
    pub category: String,
    pub title_key: String,
    pub regions: Vec<ProfileRegion>,
    pub layout: ProfileLayout,
    /// Transient host presentation intent; it does not add a second saved tree.
    pub presentation: ProfilePresentation,
}

/// Toolkit-neutral presentation behavior that a host applies after resolving the
/// profile's semantic regions in the canonical layout document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ProfilePresentation {
    Standard,
    Overlay {
        main_region: String,
        overlay_region: String,
        modal: bool,
    },
    SlideIn {
        main_region: String,
        panel_region: String,
        edge: DockEdge,
    },
    ModelessWindow {
        main_region: String,
        dialog_region: String,
    },
    Focus {
        region: String,
    },
}

impl ProfilePresentation {
    fn semantic_regions(&self) -> Vec<&str> {
        match self {
            Self::Standard => Vec::new(),
            Self::Overlay {
                main_region,
                overlay_region,
                ..
            } => vec![main_region, overlay_region],
            Self::SlideIn {
                main_region,
                panel_region,
                ..
            } => vec![main_region, panel_region],
            Self::ModelessWindow {
                main_region,
                dialog_region,
            } => vec![main_region, dialog_region],
            Self::Focus { region } => vec![region],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileApplication {
    pub document: LayoutDocument,
    pub diff: crate::LayoutDiff,
    pub operation_id: crate::CommandId,
    /// Initial node IDs are useful for adapter setup; use
    /// `resolve_profile_region` again after user layout changes.
    pub semantic_regions: BTreeMap<String, NodeId>,
    /// Adapter instruction for transient modal/overlay/focus presentation.
    pub presentation: ProfilePresentation,
}

/// Instantiate a profile over the host's registered panel descriptors.
/// Missing optional regions are removed and split weights are renormalized.
/// Unassigned panels remain registered and are recorded as closed. Existing
/// floating/drawer/maximized presentation is replaced by the selected profile.
pub fn apply_workspace_profile(
    document: &LayoutDocument,
    profile: &WorkspaceProfile,
    assignments: &BTreeMap<String, Vec<PanelId>>,
) -> DockResult<ProfileApplication> {
    validate_profile(profile)?;
    validate_layout(document)?;

    let region_specs: BTreeMap<_, _> = profile
        .regions
        .iter()
        .map(|region| (region.id.as_str(), region))
        .collect();
    for key in assignments.keys() {
        if !region_specs.contains_key(key.as_str()) {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Profile assignment references an unknown semantic region.",
                key.clone(),
            ));
        }
    }
    let known_panels: BTreeSet<_> = document
        .panels
        .iter()
        .map(|panel| panel.id.clone())
        .collect();
    let mut assigned = BTreeSet::new();
    for (region, panels) in assignments {
        for panel in panels {
            if !known_panels.contains(panel) {
                return Err(DockError::new(
                    ErrorCode::NotFound,
                    "Profile assignment references an unregistered panel.",
                    panel.0.clone(),
                ));
            }
            if !assigned.insert(panel.clone()) {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "A panel can be assigned to only one profile region.",
                    format!("{}:{region}", panel.0),
                ));
            }
        }
    }
    for region in &profile.regions {
        if region.required && assignments.get(&region.id).is_none_or(Vec::is_empty) {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "A required profile region has no assigned panel.",
                region.id.clone(),
            ));
        }
    }

    let mut next = document.clone();
    let mut semantic_regions = BTreeMap::new();
    let root = build_profile_node(
        &profile.id,
        &profile.layout,
        assignments,
        &mut semantic_regions,
    )?;
    next.root = root;
    next.floating_windows.clear();
    next.auto_hide_panels.clear();
    next.maximized_panel = None;
    next.closed_panels = known_panels.difference(&assigned).cloned().collect();
    next.revision = document.revision;
    crate::validate_layout_replacement(document, &next)?;
    let diff = crate::diff_layouts(document, &next);
    if !diff.is_empty() {
        next.revision = document.revision.checked_add(1).ok_or_else(|| {
            DockError::new(
                ErrorCode::ResourceLimit,
                "Layout revision cannot be incremented.",
                document.id.0.clone(),
            )
        })?;
    }
    let presentation = if profile
        .presentation
        .semantic_regions()
        .iter()
        .all(|region| semantic_regions.contains_key(*region))
    {
        profile.presentation.clone()
    } else {
        ProfilePresentation::Standard
    };
    Ok(ProfileApplication {
        document: next,
        diff,
        operation_id: format!("profile:{}", profile.id).into(),
        semantic_regions,
        presentation,
    })
}

/// Resolve a profile's semantic region after drag, tab, split, float, or other
/// supported edits. If its original group node was replaced, the anchor panel
/// identifies the current containing node.
pub fn resolve_profile_region(
    document: &LayoutDocument,
    profile_id: &str,
    region_id: &str,
) -> Option<NodeId> {
    let prefix = format!("profile-region:{}:{}:", hex(profile_id), hex(region_id));
    let mut anchor_hex = None;
    fn visit(node: &Node, prefix: &str, anchor_hex: &mut Option<String>) -> Option<NodeId> {
        if node.id.0.starts_with(prefix) {
            *anchor_hex = node.id.0.rsplit(':').next().map(str::to_owned);
            return Some(node.id.clone());
        }
        for child in node_children(node) {
            if let Some(found) = visit(child, prefix, anchor_hex) {
                return Some(found);
            }
        }
        None
    }
    for root in document
        .root
        .iter()
        .chain(document.floating_windows.iter().map(|window| &window.root))
    {
        if let Some(found) = visit(root, &prefix, &mut anchor_hex) {
            return Some(found);
        }
    }
    let anchor_hex = anchor_hex?;
    let panel = document
        .panels
        .iter()
        .find(|panel| hex(&panel.id.0) == anchor_hex)?;
    crate::find_panel_container(document, &panel.id)
}

pub fn validate_profile(profile: &WorkspaceProfile) -> DockResult<()> {
    if profile.id.trim().is_empty()
        || profile.id.len() > 256
        || profile.version == 0
        || profile.category.len() > 256
        || profile.title_key.len() > 4096
        || profile.regions.len() > 256
    {
        return Err(DockError::new(
            ErrorCode::InvalidId,
            "Profile ID and positive version are required.",
            profile.id.clone(),
        ));
    }
    let mut ids = BTreeSet::new();
    for region in &profile.regions {
        if region.id.trim().is_empty()
            || region.id.len() > 256
            || region.title_key.len() > 4096
            || !ids.insert(region.id.as_str())
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Profile region IDs must be valid and unique.",
                region.id.clone(),
            ));
        }
    }
    let declared_regions: BTreeSet<_> = profile
        .regions
        .iter()
        .map(|region| region.id.as_str())
        .collect();
    let mut seen_presentation_regions = BTreeSet::new();
    for region in profile.presentation.semantic_regions() {
        if !declared_regions.contains(region) || !seen_presentation_regions.insert(region) {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Profile presentation references an unknown or repeated semantic region.",
                region,
            ));
        }
    }
    fn inspect(
        node: &ProfileLayout,
        declared: &BTreeSet<&str>,
        seen: &mut BTreeSet<String>,
        containers: &mut BTreeSet<String>,
        depth: usize,
    ) -> DockResult<()> {
        if depth > 128 || seen.len() + containers.len() > 10_000 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Profile tree exceeds its nesting or node limit.",
                depth.to_string(),
            ));
        }
        match node {
            ProfileLayout::Region(id) => {
                if !declared.contains(id.as_str()) || !seen.insert(id.clone()) {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Profile layout references an unknown or repeated region.",
                        id.clone(),
                    ));
                }
            }
            ProfileLayout::Split {
                id,
                weights,
                children,
                ..
            } => {
                if id.trim().is_empty()
                    || id.len() > 256
                    || !containers.insert(id.clone())
                    || children.len() < 2
                    || weights.len() != children.len()
                    || !weights
                        .iter()
                        .all(|weight| weight.is_finite() && *weight > 0.0)
                    || !weights.iter().sum::<f64>().is_finite()
                    || weights.iter().sum::<f64>() <= 0.0
                {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Profile split requires a stable ID, two or more children, and positive finite weights.",
                        id.clone(),
                    ));
                }
                for child in children {
                    inspect(child, declared, seen, containers, depth + 1)?;
                }
            }
            ProfileLayout::Tabs {
                id,
                active_index,
                regions,
            } => {
                if id.trim().is_empty()
                    || id.len() > 256
                    || !containers.insert(id.clone())
                    || regions.len() < 2
                    || *active_index >= regions.len()
                {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Profile tab group requires a stable ID and a valid active item.",
                        id.clone(),
                    ));
                }
                for child in regions {
                    inspect(child, declared, seen, containers, depth + 1)?;
                }
            }
            ProfileLayout::Extension {
                id,
                modality,
                payload,
                regions,
            } => {
                if id.trim().is_empty()
                    || id.len() > 256
                    || !containers.insert(id.clone())
                    || modality.trim().is_empty()
                    || modality.len() > 256
                    || payload.len() > 4096
                    || regions.is_empty()
                {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Profile extension requires a unique ID, modality, bounded payload, and at least one region.",
                        id.clone(),
                    ));
                }
                for child in regions {
                    inspect(child, declared, seen, containers, depth + 1)?;
                }
            }
        }
        Ok(())
    }
    let declared: BTreeSet<_> = profile
        .regions
        .iter()
        .map(|region| region.id.as_str())
        .collect();
    let mut used = BTreeSet::new();
    let mut containers = BTreeSet::new();
    inspect(&profile.layout, &declared, &mut used, &mut containers, 0)?;
    if profile
        .regions
        .iter()
        .any(|region| region.required && !used.contains(&region.id))
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Every required semantic region must appear in the profile layout.",
            profile.id.clone(),
        ));
    }
    Ok(())
}

/// Versioned, host-neutral declarative workspace recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSpecification {
    pub schema_version: u32,
    pub profile: WorkspaceProfile,
}

impl WorkspaceSpecification {
    pub const CURRENT_SCHEMA: u32 = 1;

    pub fn new(profile: WorkspaceProfile) -> DockResult<Self> {
        validate_profile(&profile)?;
        Ok(Self {
            schema_version: Self::CURRENT_SCHEMA,
            profile,
        })
    }

    pub fn to_json(&self) -> DockResult<String> {
        validate_workspace_specification(self)?;
        serde_json::to_string_pretty(self).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "workspace-spec",
            )
        })
    }

    pub fn from_json(json: &str, maximum_bytes: usize) -> DockResult<Self> {
        if json.len() > maximum_bytes {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Workspace specification exceeds the configured byte limit.",
                json.len().to_string(),
            ));
        }
        let specification: Self = serde_json::from_str(json).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "workspace-spec",
            )
        })?;
        validate_workspace_specification(&specification)?;
        Ok(specification)
    }

    pub fn resolve(
        &self,
        document: &LayoutDocument,
        assignments: &BTreeMap<String, Vec<PanelId>>,
    ) -> DockResult<ProfileApplication> {
        validate_workspace_specification(self)?;
        apply_workspace_profile(document, &self.profile, assignments)
    }
}

pub fn validate_workspace_specification(specification: &WorkspaceSpecification) -> DockResult<()> {
    if specification.schema_version != WorkspaceSpecification::CURRENT_SCHEMA {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Unsupported workspace specification schema version.",
            specification.schema_version.to_string(),
        ));
    }
    validate_profile(&specification.profile)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProfileCatalog {
    pub schema_version: u32,
    pub default_profile: Option<String>,
    pub profiles: BTreeMap<String, WorkspaceProfile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileComparison {
    pub same: bool,
    pub metadata_changed: bool,
    pub layout_changed: bool,
    pub added_regions: Vec<String>,
    pub removed_regions: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostCapabilities {
    pub supported: BTreeSet<String>,
}

impl HostCapabilities {
    /// Validate the deterministic, bounded capability vocabulary supplied by a host.
    pub fn validate(&self) -> DockResult<()> {
        const MAX_CAPABILITIES: usize = 1024;
        const MAX_TOKEN_LENGTH: usize = 256;
        if self.supported.len() > MAX_CAPABILITIES {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Host capability set exceeds the supported limit.",
                format!("count={} limit={MAX_CAPABILITIES}", self.supported.len()),
            ));
        }
        for capability in &self.supported {
            if capability.is_empty()
                || capability.len() > MAX_TOKEN_LENGTH
                || !capability.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "_.:-".contains(character)
                })
            {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Host capabilities must be bounded nonempty ASCII tokens.",
                    capability,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostCapabilityAvailability {
    Available,
    Unavailable,
    NotImplemented,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostCapabilityEvidence {
    ApiDeclared,
    SyntheticContract,
    PhysicalSmokeTest,
    NotVerified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCapabilityObservation {
    pub capability_id: String,
    pub availability: HostCapabilityAvailability,
    pub evidence: HostCapabilityEvidence,
    pub details: String,
}

/// Structured host support and evidence. Availability is separate from
/// evidence so an API declaration is not mistaken for verified behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCapabilityReport {
    pub host_id: String,
    pub observations: Vec<HostCapabilityObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererCapabilityAvailability {
    Available,
    Unavailable,
    NotImplemented,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RendererCapabilityObservation {
    pub capability_id: String,
    pub availability: RendererCapabilityAvailability,
    pub details: String,
}

/// Renderer support facts supplied by the renderer adapter. The core stores
/// and projects this report without depending on a GUI or renderer crate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RendererCapabilityReport {
    pub renderer_id: String,
    pub observations: Vec<RendererCapabilityObservation>,
}

impl RendererCapabilityReport {
    pub fn validate(&self) -> DockResult<()> {
        if self.renderer_id.is_empty()
            || self.renderer_id.len() > 4096
            || self.renderer_id.chars().any(char::is_control)
            || self.observations.len() > 512
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Renderer capability report identity or observation count is invalid.",
                &self.renderer_id,
            ));
        }
        let mut ids = BTreeSet::new();
        for observation in &self.observations {
            if observation.capability_id.is_empty()
                || observation.capability_id.len() > 256
                || !observation.capability_id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "_.:-".contains(character)
                })
                || observation.details.len() > 4096
                || observation.details.chars().any(char::is_control)
                || !ids.insert(&observation.capability_id)
            {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Renderer capability observations must have unique bounded IDs and details.",
                    &observation.capability_id,
                ));
            }
        }
        Ok(())
    }
}

/// Toolkit-neutral description of one deterministic scene effect exposed to
/// capability discovery. The schema documents the accepted effect context;
/// it does not execute provider code or imply GPU/shader support.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectCapabilityObservation {
    pub effect_id: String,
    pub availability: RendererCapabilityAvailability,
    pub schema: serde_json::Value,
    pub provided_capabilities: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub details: String,
}

/// Versioned-graph input supplied by a presentation adapter for pure scene
/// effects. URNE stores schemas and relationships only; effect evaluation
/// remains in the presentation layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectCapabilityReport {
    pub provider_id: String,
    pub observations: Vec<EffectCapabilityObservation>,
}

impl EffectCapabilityReport {
    pub fn validate(&self) -> DockResult<()> {
        if self.provider_id.is_empty()
            || self.provider_id.len() > 256
            || !valid_capability_id(&self.provider_id)
            || self.observations.is_empty()
            || self.observations.len() > 128
        {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Effect capability report identity or observation count is invalid.",
                &self.provider_id,
            ));
        }
        let mut effect_ids = BTreeSet::new();
        for observation in &self.observations {
            let mut schema_nodes = 0usize;
            if !bounded_effect_schema(&observation.schema, 0, &mut schema_nodes) {
                return Err(DockError::new(
                    ErrorCode::ResourceLimit,
                    "Effect request schema exceeds its depth, node, or collection bounds.",
                    &observation.effect_id,
                ));
            }
            let schema_bytes = serde_json::to_vec(&observation.schema).map_err(|error| {
                DockError::new(
                    ErrorCode::InvalidLayout,
                    "Effect capability schema could not be serialized.",
                    error.to_string(),
                )
            })?;
            if observation.effect_id.is_empty()
                || observation.effect_id.len() > 128
                || !valid_capability_id(&observation.effect_id)
                || !effect_ids.insert(&observation.effect_id)
                || !observation.schema.is_object()
                || schema_bytes.len() > 16 * 1024
                || observation.details.len() > 4096
                || observation.details.chars().any(char::is_control)
                || observation.provided_capabilities.len() > 32
                || observation.required_capabilities.len() > 32
            {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Effect capability observations must have unique bounded IDs, object schemas, and bounded details.",
                    &observation.effect_id,
                ));
            }
            for capabilities in [
                &observation.provided_capabilities,
                &observation.required_capabilities,
            ] {
                let mut unique = BTreeSet::new();
                if capabilities.iter().any(|capability| {
                    capability.len() > 256
                        || !valid_capability_id(capability)
                        || !unique.insert(capability)
                }) {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Effect capability relationships must be valid, unique IDs.",
                        &observation.effect_id,
                    ));
                }
            }
        }
        Ok(())
    }
}

fn bounded_effect_schema(value: &serde_json::Value, depth: usize, nodes: &mut usize) -> bool {
    *nodes += 1;
    if depth > 16 || *nodes > 1024 {
        return false;
    }
    match value {
        serde_json::Value::Object(object) => {
            object.len() <= 128
                && object.iter().all(|(key, value)| {
                    key.len() <= 256 && bounded_effect_schema(value, depth + 1, nodes)
                })
        }
        serde_json::Value::Array(array) => {
            array.len() <= 256
                && array
                    .iter()
                    .all(|value| bounded_effect_schema(value, depth + 1, nodes))
        }
        serde_json::Value::String(text) => text.len() <= 4096,
        _ => true,
    }
}

fn valid_capability_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.:-".contains(character))
}

impl HostCapabilityReport {
    pub fn validate(&self, host: &HostCapabilities) -> DockResult<()> {
        host.validate()?;
        if self.host_id.is_empty() || self.host_id.len() > 4096 || self.observations.len() > 1024 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Host capability report exceeds its ID or observation limit.",
                &self.host_id,
            ));
        }
        let mut ids = BTreeSet::new();
        for observation in &self.observations {
            let id = &observation.capability_id;
            if id.is_empty()
                || id.len() > 256
                || !id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "_.:-".contains(character)
                })
                || !ids.insert(id)
                || observation.details.len() > 4096
                || observation.details.chars().any(char::is_control)
            {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Host capability observation is invalid or duplicated.",
                    id,
                ));
            }
            match observation.availability {
                HostCapabilityAvailability::Available if !host.supported.contains(id) => {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Available host report entry is absent from HostCapabilities.",
                        id,
                    ));
                }
                HostCapabilityAvailability::Unavailable
                | HostCapabilityAvailability::NotImplemented
                    if host.supported.contains(id) =>
                {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Unavailable host report entry is declared as supported.",
                        id,
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileModalityDiagnostic {
    pub modality: String,
    pub missing_capabilities: Vec<String>,
    pub fallback: crate::ExtensionFallbackPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileSwitchPlan {
    pub base_revision: u64,
    pub application: ProfileApplication,
    pub modalities: Vec<String>,
    pub diagnostics: Vec<ProfileModalityDiagnostic>,
}

/// Resolve a declarative layout and negotiate its required modality providers
/// before committing any canonical state.
pub fn plan_profile_switch(
    document: &LayoutDocument,
    expected_revision: u64,
    specification: &WorkspaceSpecification,
    assignments: &BTreeMap<String, Vec<PanelId>>,
    registry: &crate::ExtensionRegistry,
    host: &HostCapabilities,
) -> DockResult<ProfileSwitchPlan> {
    host.validate()?;
    if document.revision != expected_revision {
        return Err(DockError::new(
            ErrorCode::Conflict,
            "Workspace revision changed before the modality switch was planned.",
            format!("expected={expected_revision}:actual={}", document.revision),
        ));
    }
    validate_workspace_specification(specification)?;
    let application = specification.resolve(document, assignments)?;
    let mut selected = BTreeSet::new();
    fn collect(node: &ProfileLayout, out: &mut BTreeSet<String>) {
        match node {
            ProfileLayout::Region(_) => {}
            ProfileLayout::Split { children, .. }
            | ProfileLayout::Tabs {
                regions: children, ..
            } => {
                children.iter().for_each(|child| collect(child, out));
            }
            ProfileLayout::Extension {
                modality, regions, ..
            } => {
                out.insert(modality.clone());
                regions.iter().for_each(|child| collect(child, out));
            }
        }
    }
    collect(&specification.profile.layout, &mut selected);
    let modalities: Vec<_> = selected.into_iter().collect();
    let ids: Vec<_> = modalities
        .iter()
        .filter(|id| id.contains(':'))
        .cloned()
        .map(crate::ModalityId::from)
        .collect();
    registry.validate_modality_combination(&ids)?;

    fn validate_configs(
        node: &ProfileLayout,
        registry: &crate::ExtensionRegistry,
    ) -> DockResult<()> {
        match node {
            ProfileLayout::Region(_) => Ok(()),
            ProfileLayout::Split { children, .. }
            | ProfileLayout::Tabs {
                regions: children, ..
            } => {
                for child in children {
                    validate_configs(child, registry)?;
                }
                Ok(())
            }
            ProfileLayout::Extension {
                modality,
                payload,
                regions,
                ..
            } => {
                if modality.contains(':') {
                    let config: serde_json::Value =
                        serde_json::from_str(payload).map_err(|error| {
                            DockError::new(
                                ErrorCode::SerializationError,
                                error.to_string(),
                                modality,
                            )
                        })?;
                    registry.validate_modality_config(
                        &crate::ModalityId::from(modality.clone()),
                        &config,
                    )?;
                }
                for child in regions {
                    validate_configs(child, registry)?;
                }
                Ok(())
            }
        }
    }
    validate_configs(&specification.profile.layout, registry)?;

    let mut diagnostics = Vec::new();
    for modality in &modalities {
        let (fallback, required_capabilities, host_key) = if modality.contains(':') {
            let id = crate::ModalityId::from(modality.clone());
            let report = registry.modality_capabilities(&id)?;
            (
                report.fallback,
                report.required_capabilities,
                format!("modality:{modality}"),
            )
        } else if matches!(
            modality.as_str(),
            "doofusdock.masonry"
                | "doofusdock.spatial"
                | "doofusdock.node_graph"
                | "doofusdock.layered"
        ) {
            (
                crate::ExtensionFallbackPolicy::RejectUnavailable,
                Vec::new(),
                format!("layout-provider:{modality}"),
            )
        } else {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Profile references an unknown modality provider.",
                modality,
            ));
        };
        let mut missing = Vec::new();
        if !host.supported.contains(&host_key) {
            missing.push(host_key);
        }
        missing.extend(
            required_capabilities
                .iter()
                .filter(|capability| !host.supported.contains(*capability))
                .cloned(),
        );
        missing.sort();
        missing.dedup();
        if !missing.is_empty() && fallback == crate::ExtensionFallbackPolicy::RejectUnavailable {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Host cannot provide a required profile modality capability.",
                format!("{modality}:{}", missing.join(",")),
            ));
        }
        diagnostics.push(ProfileModalityDiagnostic {
            modality: modality.clone(),
            missing_capabilities: missing,
            fallback,
        });
    }

    let presentation_requirement = match &specification.profile.presentation {
        ProfilePresentation::Standard => None,
        ProfilePresentation::Overlay { modal: true, .. } => Some("overlay.modal"),
        ProfilePresentation::Overlay { modal: false, .. } => Some("overlay.modeless"),
        ProfilePresentation::SlideIn { .. } => Some("overlay.slide-in"),
        ProfilePresentation::ModelessWindow { .. } => Some("window.modeless"),
        ProfilePresentation::Focus { .. } => Some("window.focus"),
    };
    if let Some(capability) = presentation_requirement
        && !host.supported.contains(capability)
    {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Host does not support the requested profile presentation mode.",
            capability,
        ));
    }

    Ok(ProfileSwitchPlan {
        base_revision: expected_revision,
        application,
        modalities,
        diagnostics,
    })
}

impl Default for WorkspaceProfileCatalog {
    fn default() -> Self {
        Self {
            schema_version: 1,
            default_profile: None,
            profiles: BTreeMap::new(),
        }
    }
}

impl WorkspaceProfileCatalog {
    pub const CURRENT_SCHEMA: u32 = 1;

    pub fn from_profiles(profiles: impl IntoIterator<Item = WorkspaceProfile>) -> DockResult<Self> {
        let mut catalog = Self::default();
        for profile in profiles {
            catalog.insert(profile)?;
        }
        validate_profile_catalog(&catalog)?;
        Ok(catalog)
    }

    pub fn insert(&mut self, profile: WorkspaceProfile) -> DockResult<()> {
        validate_profile(&profile)?;
        if self.profiles.contains_key(&profile.id) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Workspace profile ID already exists.",
                profile.id,
            ));
        }
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub fn duplicate(
        &mut self,
        source_id: &str,
        new_id: impl Into<String>,
        title_key: impl Into<String>,
    ) -> DockResult<()> {
        let new_id = new_id.into();
        if self.profiles.contains_key(&new_id) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Workspace profile ID already exists.",
                new_id,
            ));
        }
        let source = self.profiles.get(source_id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Source profile was not found.",
                source_id,
            )
        })?;
        let mut duplicate = source.clone();
        duplicate.id = new_id;
        duplicate.title_key = title_key.into();
        duplicate.version = duplicate.version.checked_add(1).ok_or_else(|| {
            DockError::new(
                ErrorCode::ResourceLimit,
                "Profile version overflow.",
                source_id,
            )
        })?;
        self.insert(duplicate)
    }

    pub fn rename(
        &mut self,
        old_id: &str,
        new_id: impl Into<String>,
        title_key: impl Into<String>,
    ) -> DockResult<()> {
        let new_id = new_id.into();
        if old_id != new_id && self.profiles.contains_key(&new_id) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Workspace profile ID already exists.",
                new_id,
            ));
        }
        let mut profile = self.profiles.get(old_id).cloned().ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Workspace profile was not found.",
                old_id,
            )
        })?;
        profile.id = new_id.clone();
        profile.title_key = title_key.into();
        validate_profile(&profile)?;
        self.profiles.remove(old_id);
        self.profiles.insert(new_id.clone(), profile);
        if self.default_profile.as_deref() == Some(old_id) {
            self.default_profile = Some(new_id);
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> DockResult<WorkspaceProfile> {
        if self.default_profile.as_deref() == Some(id) {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Choose another default profile before removing the current default.",
                id,
            ));
        }
        self.profiles.remove(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Workspace profile was not found.", id)
        })
    }

    pub fn set_default(&mut self, id: Option<&str>) -> DockResult<()> {
        if let Some(id) = id
            && !self.profiles.contains_key(id)
        {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Default workspace profile was not found.",
                id,
            ));
        }
        self.default_profile = id.map(str::to_owned);
        Ok(())
    }

    pub fn export(&self, id: &str) -> DockResult<String> {
        let profile = self.profiles.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Workspace profile was not found.", id)
        })?;
        WorkspaceSpecification::new(profile.clone())?.to_json()
    }

    pub fn import(&mut self, json: &str, maximum_bytes: usize) -> DockResult<()> {
        let specification = WorkspaceSpecification::from_json(json, maximum_bytes)?;
        self.insert(specification.profile)
    }

    pub fn to_json(&self) -> DockResult<String> {
        validate_profile_catalog(self)?;
        serde_json::to_string_pretty(self).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "profile-catalog",
            )
        })
    }

    /// Load schema 0's documented list form and migrate it in memory to schema 1.
    pub fn from_json(json: &str, maximum_bytes: usize) -> DockResult<Self> {
        if json.len() > maximum_bytes {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Profile catalog exceeds the configured byte limit.",
                json.len().to_string(),
            ));
        }
        let value: serde_json::Value = serde_json::from_str(json).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                error.to_string(),
                "profile-catalog",
            )
        })?;
        match value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
        {
            Some(0) => {
                #[derive(Deserialize)]
                struct LegacyCatalog {
                    schema_version: u32,
                    profiles: Vec<WorkspaceProfile>,
                }
                let legacy: LegacyCatalog = serde_json::from_value(value).map_err(|error| {
                    DockError::new(
                        ErrorCode::SerializationError,
                        error.to_string(),
                        "profile-catalog-v0",
                    )
                })?;
                debug_assert_eq!(legacy.schema_version, 0);
                Self::from_profiles(legacy.profiles)
            }
            Some(1) => {
                let catalog: Self = serde_json::from_value(value).map_err(|error| {
                    DockError::new(
                        ErrorCode::SerializationError,
                        error.to_string(),
                        "profile-catalog-v1",
                    )
                })?;
                validate_profile_catalog(&catalog)?;
                Ok(catalog)
            }
            Some(version) => Err(DockError::new(
                ErrorCode::SchemaError,
                "Unsupported profile catalog schema version.",
                version.to_string(),
            )),
            None => Err(DockError::new(
                ErrorCode::SchemaError,
                "Profile catalog schema version is required.",
                "profile-catalog",
            )),
        }
    }

    pub fn compare(&self, left: &str, right: &str) -> DockResult<ProfileComparison> {
        let left = self
            .profiles
            .get(left)
            .ok_or_else(|| DockError::new(ErrorCode::NotFound, "Profile was not found.", left))?;
        let right = self
            .profiles
            .get(right)
            .ok_or_else(|| DockError::new(ErrorCode::NotFound, "Profile was not found.", right))?;
        let left_regions: BTreeSet<_> = left
            .regions
            .iter()
            .map(|region| region.id.as_str())
            .collect();
        let right_regions: BTreeSet<_> = right
            .regions
            .iter()
            .map(|region| region.id.as_str())
            .collect();
        let added_regions = right_regions
            .difference(&left_regions)
            .map(|id| (*id).to_owned())
            .collect();
        let removed_regions = left_regions
            .difference(&right_regions)
            .map(|id| (*id).to_owned())
            .collect();
        let metadata_changed = left.category != right.category
            || left.title_key != right.title_key
            || left.presentation != right.presentation;
        let layout_changed = left.layout != right.layout || left_regions != right_regions;
        Ok(ProfileComparison {
            same: !metadata_changed && !layout_changed,
            metadata_changed,
            layout_changed,
            added_regions,
            removed_regions,
        })
    }

    pub fn reset(
        &mut self,
        defaults: impl IntoIterator<Item = WorkspaceProfile>,
    ) -> DockResult<()> {
        let replacement = Self::from_profiles(defaults)?;
        *self = replacement;
        Ok(())
    }
}

pub fn validate_profile_catalog(catalog: &WorkspaceProfileCatalog) -> DockResult<()> {
    if catalog.schema_version != WorkspaceProfileCatalog::CURRENT_SCHEMA {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Unsupported profile catalog schema version.",
            catalog.schema_version.to_string(),
        ));
    }
    for (id, profile) in &catalog.profiles {
        validate_profile(profile)?;
        if id != &profile.id {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Profile catalog key does not match the profile ID.",
                id.clone(),
            ));
        }
    }
    if let Some(default) = &catalog.default_profile
        && !catalog.profiles.contains_key(default)
    {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Profile catalog default references an unknown profile.",
            default.clone(),
        ));
    }
    Ok(())
}

pub fn save_profile_catalog(
    store: &crate::WorkspacePreferencesStore,
    workspace: &crate::WorkspaceId,
    catalog: &WorkspaceProfileCatalog,
) -> DockResult<()> {
    validate_profile_catalog(catalog)?;
    let value = serde_json::to_value(catalog).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "profile-catalog",
        )
    })?;
    store.save(workspace, &value)
}

pub fn load_profile_catalog(
    store: &crate::WorkspacePreferencesStore,
    workspace: &crate::WorkspaceId,
) -> DockResult<WorkspaceProfileCatalog> {
    let value = store.load(workspace)?;
    let json = serde_json::to_string(&value).map_err(|error| {
        DockError::new(
            ErrorCode::SerializationError,
            error.to_string(),
            "profile-catalog",
        )
    })?;
    WorkspaceProfileCatalog::from_json(&json, 8 * 1024 * 1024)
}

/// The built-in recipe seed set. Applications can clone and customize any
/// profile or define arbitrary profiles using the same tree contract.
pub fn builtin_workspace_profiles() -> Vec<WorkspaceProfile> {
    let mut profiles = vec![
        profile(
            "workbench",
            "workspaces",
            &["navigation", "main", "inspector", "output"],
            split(
                "root",
                Orientation::Horizontal,
                vec![
                    region("navigation"),
                    split(
                        "center",
                        Orientation::Horizontal,
                        vec![region("main"), region("inspector")],
                        vec![0.75, 0.25],
                    ),
                    region("output"),
                ],
                vec![0.20, 0.60, 0.20],
            ),
        ),
        profile(
            "document-workspace",
            "workspaces",
            &["outline", "editor", "inspector", "output"],
            split(
                "root",
                Orientation::Horizontal,
                vec![
                    region("outline"),
                    split(
                        "editor-inspector",
                        Orientation::Horizontal,
                        vec![region("editor"), region("inspector")],
                        vec![0.75, 0.25],
                    ),
                    region("output"),
                ],
                vec![0.18, 0.62, 0.20],
            ),
        ),
        profile(
            "email-three-pane",
            "communication",
            &["folders", "messages", "reading-pane"],
            split(
                "root",
                Orientation::Horizontal,
                vec![
                    region("folders"),
                    region("messages"),
                    region("reading-pane"),
                ],
                vec![0.18, 0.32, 0.50],
            ),
        ),
        profile(
            "chat",
            "communication",
            &["rooms", "conversation", "participants", "composer"],
            split(
                "root",
                Orientation::Horizontal,
                vec![
                    region("rooms"),
                    split(
                        "conversation-stack",
                        Orientation::Vertical,
                        vec![region("conversation"), region("composer")],
                        vec![0.82, 0.18],
                    ),
                    region("participants"),
                ],
                vec![0.20, 0.60, 0.20],
            ),
        ),
        profile(
            "browse-preview",
            "navigation",
            &["navigation", "items", "preview"],
            split(
                "root",
                Orientation::Horizontal,
                vec![region("navigation"), region("items"), region("preview")],
                vec![0.22, 0.34, 0.44],
            ),
        ),
        profile(
            "guided-task",
            "navigation",
            &["steps", "content", "help"],
            split(
                "root",
                Orientation::Horizontal,
                vec![region("steps"), region("content"), region("help")],
                vec![0.20, 0.60, 0.20],
            ),
        ),
        profile(
            "dashboard",
            "data",
            &["overview", "activity", "alerts", "details"],
            split(
                "root",
                Orientation::Vertical,
                vec![
                    split(
                        "top",
                        Orientation::Horizontal,
                        vec![region("overview"), region("activity")],
                        vec![0.5, 0.5],
                    ),
                    split(
                        "bottom",
                        Orientation::Horizontal,
                        vec![region("alerts"), region("details")],
                        vec![0.5, 0.5],
                    ),
                ],
                vec![0.5, 0.5],
            ),
        ),
        profile(
            "data-explorer",
            "data",
            &["query", "results", "details"],
            split(
                "root",
                Orientation::Horizontal,
                vec![region("query"), region("results"), region("details")],
                vec![0.22, 0.50, 0.28],
            ),
        ),
        profile(
            "media-design",
            "media",
            &["library", "surface", "timeline", "inspector"],
            split(
                "root",
                Orientation::Horizontal,
                vec![
                    region("library"),
                    split(
                        "surface-timeline",
                        Orientation::Vertical,
                        vec![region("surface"), region("timeline")],
                        vec![0.68, 0.32],
                    ),
                    region("inspector"),
                ],
                vec![0.22, 0.56, 0.22],
            ),
        ),
    ];
    profiles.extend(additional_workspace_profiles());
    profiles
}

fn profile(id: &str, category: &str, regions: &[&str], layout: ProfileLayout) -> WorkspaceProfile {
    WorkspaceProfile {
        id: id.into(),
        version: 1,
        category: category.into(),
        title_key: format!("profile.{id}"),
        regions: regions
            .iter()
            .map(|id| ProfileRegion {
                id: (*id).into(),
                title_key: format!("region.{id}"),
                required: matches!(
                    *id,
                    "main"
                        | "editor"
                        | "conversation"
                        | "content"
                        | "overview"
                        | "results"
                        | "surface"
                        | "detail"
                        | "reading-pane"
                        | "preview"
                        | "canvas"
                        | "table"
                        | "sheet"
                        | "board"
                        | "calendar"
                        | "map"
                        | "gallery"
                        | "product-grid"
                        | "timeline"
                        | "player"
                        | "viewport"
                        | "graph"
                        | "diff"
                        | "terminal"
                        | "metrics"
                        | "feed"
                        | "search-results"
                ),
            })
            .collect(),
        layout,
        presentation: match id {
            "overlay-panel" => ProfilePresentation::Overlay {
                main_region: "main".into(),
                overlay_region: "overlay".into(),
                modal: false,
            },
            "modal-dialog" => ProfilePresentation::Overlay {
                main_region: "main".into(),
                overlay_region: "dialog".into(),
                modal: true,
            },
            "modeless-dialog" => ProfilePresentation::ModelessWindow {
                main_region: "main".into(),
                dialog_region: "dialog".into(),
            },
            "slide-in-panel" => ProfilePresentation::SlideIn {
                main_region: "main".into(),
                panel_region: "slide-in".into(),
                edge: DockEdge::Right,
            },
            "focus-full-screen" => ProfilePresentation::Focus {
                region: "main".into(),
            },
            _ => ProfilePresentation::Standard,
        },
    }
}
fn region(id: &str) -> ProfileLayout {
    ProfileLayout::Region(id.into())
}
fn split(
    id: &str,
    orientation: Orientation,
    children: Vec<ProfileLayout>,
    weights: Vec<f64>,
) -> ProfileLayout {
    ProfileLayout::Split {
        id: id.into(),
        orientation,
        weights,
        children,
    }
}

#[derive(Clone, Copy)]
enum RecipeShape {
    Single,
    SideBySide,
    TopAndMain,
    MainAndBottom,
    ThreeColumn,
    NestedWorkbench,
    Grid2x2,
    Tabs,
    Extension,
    Conversation,
    TimelineEditor,
}

fn additional_workspace_profiles() -> Vec<WorkspaceProfile> {
    use RecipeShape::*;
    let definitions: &[(&str, &str, RecipeShape, &[&str])] = &[
        ("single-pane", "navigation", Single, &["main"]),
        (
            "sidebar-navigation",
            "navigation",
            SideBySide,
            &["navigation", "main"],
        ),
        (
            "top-navigation",
            "navigation",
            TopAndMain,
            &["navigation", "main"],
        ),
        (
            "bottom-navigation",
            "navigation",
            MainAndBottom,
            &["main", "navigation"],
        ),
        (
            "navigation-rail",
            "navigation",
            SideBySide,
            &["rail", "main"],
        ),
        (
            "tabbed-navigation",
            "navigation",
            Tabs,
            &["main", "activity", "details"],
        ),
        (
            "tree-navigation",
            "navigation",
            ThreeColumn,
            &["tree", "main", "inspector"],
        ),
        (
            "breadcrumb-navigation",
            "navigation",
            TopAndMain,
            &["breadcrumb", "main"],
        ),
        ("hub-and-spoke", "navigation", SideBySide, &["hub", "tasks"]),
        (
            "master-detail",
            "navigation",
            SideBySide,
            &["master", "detail"],
        ),
        ("list-detail", "navigation", SideBySide, &["list", "detail"]),
        (
            "wizard-stepper",
            "navigation",
            ThreeColumn,
            &["steps", "content", "help"],
        ),
        (
            "canvas",
            "workspaces",
            ThreeColumn,
            &["tools", "canvas", "inspector"],
        ),
        (
            "inspector-properties",
            "workspaces",
            SideBySide,
            &["main", "inspector"],
        ),
        (
            "multi-column-workspace",
            "workspaces",
            ThreeColumn,
            &["navigation", "main", "details"],
        ),
        (
            "split-view",
            "workspaces",
            SideBySide,
            &["main", "secondary"],
        ),
        (
            "nested-split-view",
            "workspaces",
            NestedWorkbench,
            &["navigation", "main", "inspector", "output"],
        ),
        (
            "tabbed-panels",
            "workspaces",
            Tabs,
            &["main", "secondary", "output"],
        ),
        (
            "docked-panels",
            "workspaces",
            ThreeColumn,
            &["navigation", "main", "inspector"],
        ),
        (
            "auto-hide-panels",
            "workspaces",
            SideBySide,
            &["main", "edge-panels"],
        ),
        (
            "floating-panels",
            "workspaces",
            SideBySide,
            &["main", "utility-panels"],
        ),
        ("overlay-panel", "workspaces", Tabs, &["main", "overlay"]),
        (
            "slide-in-panel",
            "workspaces",
            SideBySide,
            &["main", "slide-in"],
        ),
        (
            "modal-dialog",
            "workspaces",
            SideBySide,
            &["main", "dialog"],
        ),
        (
            "modeless-dialog",
            "workspaces",
            SideBySide,
            &["main", "dialog"],
        ),
        ("focus-full-screen", "workspaces", Single, &["main"]),
        (
            "irc-layout",
            "communication",
            Conversation,
            &["servers", "conversation", "users", "composer"],
        ),
        (
            "threaded-conversation",
            "communication",
            ThreeColumn,
            &["threads", "conversation", "replies"],
        ),
        (
            "inbox-layout",
            "communication",
            ThreeColumn,
            &["folders", "messages", "reading-pane"],
        ),
        (
            "presence-collaboration",
            "communication",
            ThreeColumn,
            &["main", "participants", "activity"],
        ),
        (
            "feed-timeline",
            "communication",
            SideBySide,
            &["navigation", "feed"],
        ),
        (
            "table-grid",
            "data",
            ThreeColumn,
            &["filters", "table", "inspector"],
        ),
        (
            "spreadsheet",
            "data",
            NestedWorkbench,
            &["sheets", "main", "formula-bar", "inspector"],
        ),
        (
            "card-grid",
            "data",
            SideBySide,
            &["filters", "product-grid"],
        ),
        (
            "gallery",
            "data",
            ThreeColumn,
            &["folders", "gallery", "preview"],
        ),
        ("kanban-board", "data", SideBySide, &["filters", "board"]),
        (
            "calendar",
            "data",
            ThreeColumn,
            &["calendar-list", "calendar", "agenda"],
        ),
        (
            "timeline-gantt",
            "data",
            TimelineEditor,
            &["tasks", "main", "timeline", "inspector"],
        ),
        (
            "map-based",
            "data",
            ThreeColumn,
            &["filters", "map", "results"],
        ),
        (
            "catalog-storefront",
            "data",
            ThreeColumn,
            &["categories", "product-grid", "details"],
        ),
        (
            "search-results",
            "data",
            ThreeColumn,
            &["filters", "search-results", "preview"],
        ),
        (
            "tree-table",
            "data",
            ThreeColumn,
            &["tree", "table", "inspector"],
        ),
        ("media-player", "media", SideBySide, &["player", "queue"]),
        (
            "playlist-library",
            "media",
            SideBySide,
            &["library", "player"],
        ),
        (
            "video-editing",
            "media",
            TimelineEditor,
            &["media-bin", "preview", "timeline", "inspector"],
        ),
        (
            "audio-mixing",
            "media",
            TimelineEditor,
            &["tracks", "mixer", "timeline", "effects"],
        ),
        (
            "image-editor",
            "media",
            ThreeColumn,
            &["tools", "canvas", "layers"],
        ),
        (
            "3d-editor",
            "media",
            TimelineEditor,
            &["scene-tree", "viewport", "timeline", "inspector"],
        ),
        (
            "node-graph",
            "media",
            Extension,
            &["node-library", "graph", "inspector"],
        ),
        ("code-diff", "media", SideBySide, &["source", "diff"]),
        (
            "terminal-console",
            "media",
            ThreeColumn,
            &["sessions", "terminal", "output"],
        ),
        (
            "monitoring-command-center",
            "media",
            Grid2x2,
            &["metrics", "activity", "alerts", "details"],
        ),
        ("one-column", "structure", Single, &["main"]),
        (
            "two-column",
            "structure",
            SideBySide,
            &["main", "secondary"],
        ),
        (
            "three-column",
            "structure",
            ThreeColumn,
            &["navigation", "main", "details"],
        ),
        (
            "grid",
            "structure",
            Grid2x2,
            &["top-left", "top-right", "bottom-left", "bottom-right"],
        ),
        (
            "masonry",
            "structure",
            Extension,
            &["column-one", "column-two", "column-three", "column-four"],
        ),
        (
            "asymmetric",
            "structure",
            ThreeColumn,
            &["primary", "secondary", "tertiary"],
        ),
        (
            "responsive-adaptive",
            "structure",
            ThreeColumn,
            &["navigation", "main", "details"],
        ),
        (
            "card-based",
            "structure",
            Grid2x2,
            &["card-one", "card-two", "card-three", "card-four"],
        ),
        (
            "layered",
            "structure",
            Extension,
            &["base", "sheet", "popover"],
        ),
        (
            "spatial",
            "structure",
            Extension,
            &[
                "surface-one",
                "surface-two",
                "surface-three",
                "surface-four",
            ],
        ),
    ];
    definitions
        .iter()
        .map(|(id, category, shape, regions)| {
            let layout = match shape {
                Single => region(regions[0]),
                SideBySide => split(
                    "root",
                    Orientation::Horizontal,
                    regions.iter().map(|id| region(id)).collect(),
                    normalized_weights(regions.len()),
                ),
                TopAndMain => split(
                    "root",
                    Orientation::Vertical,
                    regions.iter().map(|id| region(id)).collect(),
                    vec![0.15, 0.85],
                ),
                MainAndBottom => split(
                    "root",
                    Orientation::Vertical,
                    regions.iter().map(|id| region(id)).collect(),
                    vec![0.82, 0.18],
                ),
                ThreeColumn => split(
                    "root",
                    Orientation::Horizontal,
                    regions.iter().map(|id| region(id)).collect(),
                    vec![0.22, 0.50, 0.28],
                ),
                Tabs => ProfileLayout::Tabs {
                    id: "root".into(),
                    active_index: 0,
                    regions: regions.iter().map(|id| region(id)).collect(),
                },
                Extension => ProfileLayout::Extension {
                    id: "root".into(),
                    modality: match *id {
                        "masonry" => "doofusdock.masonry",
                        "spatial" => "doofusdock.spatial",
                        "layered" => "doofusdock.layered",
                        _ => "doofusdock.node_graph",
                    }
                    .into(),
                    payload: match *id {
                        "masonry" => r#"{"packing":"masonry","columns":4}"#,
                        "spatial" => r#"{"coordinate_space":"workspace","placement":"freeform"}"#,
                        "layered" => "{}",
                        _ => r#"{"canvas":"node_graph"}"#,
                    }
                    .into(),
                    regions: regions.iter().map(|id| region(id)).collect(),
                },
                Grid2x2 => split(
                    "root",
                    Orientation::Vertical,
                    vec![
                        split(
                            "row-top",
                            Orientation::Horizontal,
                            vec![region(regions[0]), region(regions[1])],
                            vec![0.5, 0.5],
                        ),
                        split(
                            "row-bottom",
                            Orientation::Horizontal,
                            vec![region(regions[2]), region(regions[3])],
                            vec![0.5, 0.5],
                        ),
                    ],
                    vec![0.5, 0.5],
                ),
                NestedWorkbench => split(
                    "root",
                    Orientation::Horizontal,
                    vec![
                        region(regions[0]),
                        split(
                            "center",
                            Orientation::Horizontal,
                            vec![region(regions[1]), region(regions[3])],
                            vec![0.72, 0.28],
                        ),
                        region(regions[2]),
                    ],
                    vec![0.18, 0.62, 0.20],
                ),
                Conversation => split(
                    "root",
                    Orientation::Horizontal,
                    vec![
                        region(regions[0]),
                        split(
                            "conversation-stack",
                            Orientation::Vertical,
                            vec![region(regions[1]), region(regions[3])],
                            vec![0.82, 0.18],
                        ),
                        region(regions[2]),
                    ],
                    vec![0.20, 0.60, 0.20],
                ),
                TimelineEditor => split(
                    "root",
                    Orientation::Horizontal,
                    vec![
                        region(regions[0]),
                        split(
                            "surface-timeline",
                            Orientation::Vertical,
                            vec![region(regions[1]), region(regions[2])],
                            vec![0.68, 0.32],
                        ),
                        region(regions[3]),
                    ],
                    vec![0.22, 0.56, 0.22],
                ),
            };
            profile(id, category, regions, layout)
        })
        .collect()
}

fn normalized_weights(count: usize) -> Vec<f64> {
    vec![1.0 / count as f64; count]
}

fn build_profile_node(
    profile_id: &str,
    layout: &ProfileLayout,
    assignments: &BTreeMap<String, Vec<PanelId>>,
    semantic_regions: &mut BTreeMap<String, NodeId>,
) -> DockResult<Option<NodePtr>> {
    match layout {
        ProfileLayout::Region(region_id) => {
            let Some(panels) = assignments
                .get(region_id)
                .filter(|panels| !panels.is_empty())
            else {
                return Ok(None);
            };
            let root_id = profile_region_node_id(profile_id, region_id, &panels[0]);
            let children: Vec<_> = panels
                .iter()
                .map(|panel| Node::panel(panel_node_id(panel), panel.clone()))
                .collect();
            let root = if children.len() == 1 {
                Node::panel(root_id.clone(), panels[0].clone())
            } else {
                Arc::new(Node {
                    id: root_id.clone(),
                    kind: NodeKind::Tabs {
                        active_index: 0,
                        minimized: false,
                        items: children,
                    },
                })
            };
            semantic_regions.insert(region_id.clone(), root_id);
            Ok(Some(root))
        }
        ProfileLayout::Split {
            id,
            orientation,
            weights,
            children,
        } => {
            let mut nodes = Vec::new();
            let mut kept_weights = Vec::new();
            for (child, weight) in children.iter().zip(weights) {
                if let Some(node) =
                    build_profile_node(profile_id, child, assignments, semantic_regions)?
                {
                    nodes.push(node);
                    kept_weights.push(*weight);
                }
            }
            match nodes.len() {
                0 => Ok(None),
                1 => Ok(nodes.pop()),
                _ => {
                    let sum: f64 = kept_weights.iter().sum();
                    let normalized = kept_weights
                        .into_iter()
                        .map(|weight| weight / sum)
                        .collect();
                    Ok(Some(Node::split(
                        profile_container_id(profile_id, id),
                        *orientation,
                        nodes,
                        normalized,
                    )))
                }
            }
        }
        ProfileLayout::Tabs {
            id,
            active_index,
            regions,
        } => {
            let mut items = Vec::new();
            let mut remapped_active = 0;
            for (index, region) in regions.iter().enumerate() {
                if let Some(node) =
                    build_profile_node(profile_id, region, assignments, semantic_regions)?
                {
                    if index < *active_index {
                        remapped_active += 1;
                    }
                    items.push(node);
                }
            }
            match items.len() {
                0 => Ok(None),
                1 => Ok(items.pop()),
                _ => {
                    let active_index = remapped_active.min(items.len() - 1);
                    Ok(Some(Node::tabs(
                        profile_container_id(profile_id, id),
                        items,
                        active_index,
                    )))
                }
            }
        }
        ProfileLayout::Extension {
            id,
            modality,
            payload,
            regions,
        } => {
            let mut children = Vec::new();
            for region in regions {
                if let Some(node) =
                    build_profile_node(profile_id, region, assignments, semantic_regions)?
                {
                    children.push(node);
                }
            }
            if children.is_empty() {
                Ok(None)
            } else {
                let payload = if modality == crate::LAYERED_MODALITY {
                    crate::LayeredConfiguration::from_children(&children)?.to_payload()?
                } else {
                    payload.clone()
                };
                Ok(Some(Arc::new(Node {
                    id: profile_container_id(profile_id, id),
                    kind: NodeKind::Extension {
                        modality: modality.clone().into(),
                        payload,
                        children,
                    },
                })))
            }
        }
    }
}

fn node_children(node: &Node) -> &[NodePtr] {
    match &node.kind {
        NodeKind::Tabs { items, .. } => items,
        NodeKind::DocumentWell { groups, .. } => groups,
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => children,
        NodeKind::Panel { .. } | NodeKind::Placeholder { .. } => &[],
    }
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn profile_region_node_id(profile: &str, region: &str, anchor: &PanelId) -> NodeId {
    format!(
        "profile-region:{}:{}:{}",
        hex(profile),
        hex(region),
        hex(&anchor.0)
    )
    .into()
}
fn profile_container_id(profile: &str, container: &str) -> NodeId {
    format!("profile-container:{}:{}", hex(profile), hex(container)).into()
}
fn panel_node_id(panel: &PanelId) -> NodeId {
    format!("profile-panel:{}", hex(&panel.0)).into()
}
