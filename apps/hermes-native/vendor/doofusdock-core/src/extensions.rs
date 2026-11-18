use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityRange {
    pub minimum: ApiVersion,
    pub maximum: ApiVersion,
}
pub type ActivateHook = Arc<dyn Fn() -> DockResult<()> + Send + Sync>;
pub type ShutdownHook = Arc<dyn Fn() + Send + Sync>;
pub type PanelValidationHook = Arc<dyn Fn(&PanelDescriptor) -> DockResult<()> + Send + Sync>;
pub type ModalityValidationHook = Arc<dyn Fn(&Value) -> DockResult<()> + Send + Sync>;
pub type TransitionDurationHook = Arc<dyn Fn(&TransitionRequest) -> u32 + Send + Sync>;
pub type CodecEncodeHook = Arc<dyn Fn(&Value) -> DockResult<Value> + Send + Sync>;
pub type CodecDecodeHook = Arc<dyn Fn(&Value) -> DockResult<Value> + Send + Sync>;
pub type CommandExecuteHook =
    Arc<dyn Fn(&LayoutDocument, &Value) -> DockResult<TransactionResult> + Send + Sync>;
pub type HostDiffValidationHook = Arc<dyn Fn(&LayoutDiff) -> DockResult<()> + Send + Sync>;
pub type WorkspaceProfileHook = Arc<dyn Fn(&str) -> DockResult<WorkspaceProfile> + Send + Sync>;
pub type RendererCapabilityHook =
    Arc<dyn Fn() -> DockResult<RendererCapabilityReport> + Send + Sync>;
pub type PanelShellHook = Arc<dyn Fn(&PanelDescriptor) -> DockResult<PanelShellSpec> + Send + Sync>;
pub type InputProviderHook =
    Arc<dyn Fn(&LayoutDocument, &InputEvent) -> DockResult<InputIntent> + Send + Sync>;
pub type AccessibilityProviderHook =
    Arc<dyn Fn(&LayoutDocument) -> DockResult<AccessibilityTree> + Send + Sync>;
#[derive(Clone, Default)]
pub struct ExtensionDescriptor {
    pub namespaced_id: String,
    pub version: ApiVersion,
    pub api: CompatibilityRange,
    pub on_activate: Option<ActivateHook>,
    pub on_shutdown: Option<ShutdownHook>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionFallbackPolicy {
    RejectUnavailable,
    #[default]
    PreserveOpaquePayload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionKind {
    PanelProvider,
    Modality,
    Transition,
    PersistenceCodec,
    Command,
    HostAdapter,
    LayoutProvider,
    ProfileProvider,
    RendererProvider,
    PanelShellProvider,
    InputProvider,
    AccessibilityProvider,
}

/// Stable, closure-free view of registered extension capabilities for clients
/// that need to inspect a registry without executing provider callbacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtensionCapabilitySnapshot {
    pub id: String,
    pub kind: ExtensionKind,
    pub version: ApiVersion,
    pub api: CompatibilityRange,
    pub active: bool,
    pub operations: Vec<String>,
    pub provided_capabilities: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub incompatible_capabilities: Vec<String>,
    pub configuration_schema: Option<Value>,
    pub fallback: Option<ExtensionFallbackPolicy>,
    /// `None` means this provider makes no host-capability assertion.
    pub supported_on_host: Option<bool>,
    pub missing_capabilities: Vec<String>,
    pub reason_unavailable: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalityCapabilityReport {
    pub operations: Vec<String>,
    pub capabilities: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub incompatible_capabilities: Vec<String>,
    pub fallback: ExtensionFallbackPolicy,
}
#[derive(Clone)]
pub struct PanelProviderExtension {
    pub descriptor: ExtensionDescriptor,
    pub validate_panel: PanelValidationHook,
}
#[derive(Clone)]
pub struct ModalityExtension {
    pub descriptor: ExtensionDescriptor,
    pub validate_config: ModalityValidationHook,
    pub operations: Vec<String>,
    pub capabilities: Vec<String>,
    pub fallback: ExtensionFallbackPolicy,
    pub configuration_schema: Value,
    pub required_capabilities: Vec<String>,
    pub incompatible_capabilities: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct TransitionRequest {
    pub id: TransitionId,
    pub duration_ms: u32,
    pub reduced_motion: bool,
    pub host_capabilities: Vec<String>,
}
#[derive(Clone)]
pub struct TransitionExtension {
    pub descriptor: ExtensionDescriptor,
    pub duration: TransitionDurationHook,
    pub required_capabilities: Vec<String>,
    pub incompatible_capabilities: Vec<String>,
}
#[derive(Clone)]
pub struct PersistenceCodecExtension {
    pub descriptor: ExtensionDescriptor,
    pub encode: CodecEncodeHook,
    pub decode: CodecDecodeHook,
}
#[derive(Clone)]
pub struct CommandExtension {
    pub descriptor: ExtensionDescriptor,
    pub execute: CommandExecuteHook,
}
#[derive(Clone)]
pub struct HostAdapterExtension {
    pub descriptor: ExtensionDescriptor,
    pub validate_diff: HostDiffValidationHook,
}
#[derive(Clone)]
pub struct LayoutProviderExtension {
    pub descriptor: ExtensionDescriptor,
    pub provider: Arc<dyn ExtensionLayoutProvider>,
}
#[derive(Clone)]
pub struct ProfileProviderExtension {
    pub descriptor: ExtensionDescriptor,
    /// Resolve one named, toolkit-neutral workspace profile.
    pub provide_profile: WorkspaceProfileHook,
}
#[derive(Clone)]
pub struct RendererProviderExtension {
    pub descriptor: ExtensionDescriptor,
    /// Produce a bounded, closure-free renderer capability report on request.
    pub capability_report: RendererCapabilityHook,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelShellActionKind {
    Close,
    Dock,
    Float,
    Pin,
    Maximize,
    Restore,
    Custom,
}

/// Toolkit-neutral shell model. A renderer may paint these actions, while a
/// host maps selected action IDs through its canonical policy/operation path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelShellActionSpec {
    pub id: String,
    pub title_key: String,
    pub kind: PanelShellActionKind,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelShellSpec {
    pub panel_id: PanelId,
    pub title_key: String,
    pub accessible_label: String,
    pub actions: Vec<PanelShellActionSpec>,
}

#[derive(Clone)]
pub struct PanelShellProviderExtension {
    pub descriptor: ExtensionDescriptor,
    pub describe_shell: PanelShellHook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputDevice {
    Pointer,
    Keyboard,
    Touch,
    Pen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputPhase {
    Begin,
    Update,
    End,
    Cancel,
}

/// Toolkit-neutral input event after a host adapter has normalized platform
/// input. Coordinates are optional logical host coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputEvent {
    pub device: InputDevice,
    pub phase: InputPhase,
    pub target_id: Option<String>,
    pub position: Option<[f64; 2]>,
    pub key: Option<String>,
    pub modifiers: Vec<String>,
}

/// Input providers return semantic focus and canonical operations as an
/// intention. The registry validates operations against a private snapshot;
/// the host decides whether and when to commit them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputIntent {
    pub consumed: bool,
    pub focus_target: Option<String>,
    pub operations: Vec<Operation>,
}

#[derive(Clone)]
pub struct InputProviderExtension {
    pub descriptor: ExtensionDescriptor,
    pub route_input: InputProviderHook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityRole {
    Window,
    Group,
    Region,
    Tab,
    Button,
    Dialog,
    Text,
    Textbox,
    Menu,
    MenuItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityNodeSpec {
    pub id: String,
    pub role: AccessibilityRole,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub selected: bool,
    pub actions: Vec<String>,
    pub children: Vec<AccessibilityNodeSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityTree {
    pub root: AccessibilityNodeSpec,
}

#[derive(Clone)]
pub struct AccessibilityProviderExtension {
    pub descriptor: ExtensionDescriptor,
    pub describe_tree: AccessibilityProviderHook,
}

#[derive(Clone)]
pub struct ExtensionRegistry {
    api: ApiVersion,
    frozen: bool,
    order: Vec<String>,
    active: HashSet<String>,
    descriptors: HashMap<String, ExtensionDescriptor>,
    panel_providers: HashMap<String, PanelProviderExtension>,
    modalities: HashMap<String, ModalityExtension>,
    transitions: HashMap<String, TransitionExtension>,
    codecs: HashMap<String, PersistenceCodecExtension>,
    commands: HashMap<String, CommandExtension>,
    host_adapters: HashMap<String, HostAdapterExtension>,
    layout_providers: HashMap<String, LayoutProviderExtension>,
    layout_provider_by_modality: HashMap<String, String>,
    profile_providers: HashMap<String, ProfileProviderExtension>,
    renderer_providers: HashMap<String, RendererProviderExtension>,
    panel_shell_providers: HashMap<String, PanelShellProviderExtension>,
    input_providers: HashMap<String, InputProviderExtension>,
    accessibility_providers: HashMap<String, AccessibilityProviderExtension>,
}
impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self {
            api: ApiVersion { major: 1, minor: 0 },
            frozen: false,
            order: Vec::new(),
            active: HashSet::new(),
            descriptors: HashMap::new(),
            panel_providers: HashMap::new(),
            modalities: HashMap::new(),
            transitions: HashMap::new(),
            codecs: HashMap::new(),
            commands: HashMap::new(),
            host_adapters: HashMap::new(),
            layout_providers: HashMap::new(),
            layout_provider_by_modality: HashMap::new(),
            profile_providers: HashMap::new(),
            renderer_providers: HashMap::new(),
            panel_shell_providers: HashMap::new(),
            input_providers: HashMap::new(),
            accessibility_providers: HashMap::new(),
        }
    }
}
impl ExtensionRegistry {
    pub fn new(api: ApiVersion) -> Self {
        Self {
            api,
            ..Self::default()
        }
    }
    pub fn freeze(&mut self) {
        self.frozen = true;
    }
    pub fn frozen(&self) -> bool {
        self.frozen
    }
    pub fn active(&self, id: &str) -> bool {
        self.active.contains(id)
    }
    pub fn counts(&self) -> [usize; 12] {
        [
            self.panel_providers.len(),
            self.modalities.len(),
            self.transitions.len(),
            self.codecs.len(),
            self.commands.len(),
            self.host_adapters.len(),
            self.layout_providers.len(),
            self.profile_providers.len(),
            self.renderer_providers.len(),
            self.panel_shell_providers.len(),
            self.input_providers.len(),
            self.accessibility_providers.len(),
        ]
    }

    /// Return a stable, deterministic capability snapshot for all registered
    /// extensions. Host support is assessed only for modalities, whose
    /// declarations explicitly list required host capabilities.
    pub fn capability_snapshot(&self, host: &HostCapabilities) -> Vec<ExtensionCapabilitySnapshot> {
        let mut snapshots = Vec::with_capacity(self.descriptors.len());
        for id in &self.order {
            let Some(descriptor) = self.descriptors.get(id) else {
                continue;
            };
            let (kind, operations, provided, required, incompatible, schema, fallback) =
                if let Some(extension) = self.modalities.get(id) {
                    (
                        ExtensionKind::Modality,
                        extension.operations.clone(),
                        extension.capabilities.clone(),
                        extension.required_capabilities.clone(),
                        extension.incompatible_capabilities.clone(),
                        Some(extension.configuration_schema.clone()),
                        Some(extension.fallback),
                    )
                } else if self.panel_providers.contains_key(id) {
                    (
                        ExtensionKind::PanelProvider,
                        vec!["validate_panel".into()],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if let Some(extension) = self.transitions.get(id) {
                    (
                        ExtensionKind::Transition,
                        vec!["duration_ms".into()],
                        Vec::new(),
                        extension.required_capabilities.clone(),
                        extension.incompatible_capabilities.clone(),
                        None,
                        None,
                    )
                } else if self.codecs.contains_key(id) {
                    (
                        ExtensionKind::PersistenceCodec,
                        vec!["decode".into(), "encode".into()],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if self.commands.contains_key(id) {
                    (
                        ExtensionKind::Command,
                        vec![id.clone()],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if let Some(extension) = self.layout_providers.get(id) {
                    (
                        ExtensionKind::LayoutProvider,
                        vec!["arrange".into()],
                        vec![format!("layout_provider:{}", extension.provider.modality())],
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if self.profile_providers.contains_key(id) {
                    (
                        ExtensionKind::ProfileProvider,
                        vec!["get_profile".into()],
                        vec!["workspace_profile".into()],
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if self.renderer_providers.contains_key(id) {
                    (
                        ExtensionKind::RendererProvider,
                        vec!["capability_report".into()],
                        vec!["renderer_capabilities".into()],
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if self.panel_shell_providers.contains_key(id) {
                    (
                        ExtensionKind::PanelShellProvider,
                        vec!["describe_shell".into()],
                        vec!["panel_shell".into()],
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if self.input_providers.contains_key(id) {
                    (
                        ExtensionKind::InputProvider,
                        vec!["route_input".into()],
                        vec!["input_intent".into()],
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else if self.accessibility_providers.contains_key(id) {
                    (
                        ExtensionKind::AccessibilityProvider,
                        vec!["describe_tree".into()],
                        vec!["accessibility_tree".into()],
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                } else {
                    (
                        ExtensionKind::HostAdapter,
                        vec!["validate_diff".into()],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        None,
                        None,
                    )
                };
            let mut missing: Vec<_> = required
                .iter()
                .filter(|capability| !host.supported.contains(*capability))
                .cloned()
                .collect();
            missing.sort();
            let mut incompatible_present: Vec<_> = incompatible
                .iter()
                .filter(|capability| host.supported.contains(*capability))
                .cloned()
                .collect();
            incompatible_present.sort();
            let compatible = missing.is_empty() && incompatible_present.is_empty();
            let host_constraints_declared = !required.is_empty() || !incompatible.is_empty();
            let supported_on_host = (kind == ExtensionKind::Modality
                || (kind == ExtensionKind::Transition && host_constraints_declared))
                .then_some(compatible);
            let reason_unavailable = if compatible {
                None
            } else {
                Some(format!(
                    "Host capability constraints failed; missing: [{}], incompatible: [{}]",
                    missing.join(", "),
                    incompatible_present.join(", ")
                ))
            };
            let mut operations = operations;
            let mut provided = provided;
            let mut required = required;
            let mut incompatible = incompatible;
            operations.sort();
            operations.dedup();
            provided.sort();
            provided.dedup();
            required.sort();
            required.dedup();
            incompatible.sort();
            incompatible.dedup();
            snapshots.push(ExtensionCapabilitySnapshot {
                id: descriptor.namespaced_id.clone(),
                kind,
                version: descriptor.version,
                api: descriptor.api,
                active: self.active.contains(id),
                operations,
                provided_capabilities: provided,
                required_capabilities: required,
                incompatible_capabilities: incompatible,
                configuration_schema: schema,
                fallback,
                supported_on_host,
                missing_capabilities: missing,
                reason_unavailable,
            });
        }
        snapshots.sort_by(|a, b| a.id.cmp(&b.id));
        snapshots
    }
    fn require_active(&self, id: &str) -> DockResult<()> {
        if self.active.contains(id) {
            Ok(())
        } else {
            Err(DockError::new(
                ErrorCode::Conflict,
                "Extension callbacks require successful activation after registry freeze.",
                id,
            ))
        }
    }

    fn validate_descriptor(&self, d: &ExtensionDescriptor) -> DockResult<()> {
        let mut parts = d.namespaced_id.split(':');
        let valid = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
        };
        if !valid(parts.next().unwrap_or(""))
            || !valid(parts.next().unwrap_or(""))
            || parts.next().is_some()
        {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Extension IDs must use a nonempty namespace and name separated by ':'.",
                d.namespaced_id.clone(),
            ));
        }
        if d.api.minimum > d.api.maximum {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Extension API compatibility range is inverted.",
                d.namespaced_id.clone(),
            ));
        }
        if self.api < d.api.minimum || self.api > d.api.maximum {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Extension API range does not include this framework version.",
                d.namespaced_id.clone(),
            ));
        }
        Ok(())
    }
    fn register_base(&mut self, d: &ExtensionDescriptor) -> DockResult<()> {
        if self.frozen {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Extension registry is frozen.",
                d.namespaced_id.clone(),
            ));
        }
        self.validate_descriptor(d)?;
        if self.descriptors.contains_key(&d.namespaced_id) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Extension ID is already registered.",
                d.namespaced_id.clone(),
            ));
        }
        self.descriptors.insert(d.namespaced_id.clone(), d.clone());
        self.order.push(d.namespaced_id.clone());
        Ok(())
    }
    pub fn register_panel_provider(&mut self, e: PanelProviderExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.panel_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }
    pub fn register_modality(&mut self, e: ModalityExtension) -> DockResult<()> {
        for group in [
            &e.operations,
            &e.capabilities,
            &e.required_capabilities,
            &e.incompatible_capabilities,
        ] {
            validate_tokens(group, &e.descriptor.namespaced_id)?;
        }
        let required: HashSet<_> = e.required_capabilities.iter().collect();
        if e.incompatible_capabilities
            .iter()
            .any(|v| required.contains(v))
            || e.incompatible_capabilities
                .iter()
                .any(|v| e.capabilities.contains(v))
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Modality capability declarations conflict.",
                e.descriptor.namespaced_id.clone(),
            ));
        }
        let mut schema_nodes = 0;
        validate_schema_declaration(
            &e.configuration_schema,
            &format!("{}.config", e.descriptor.namespaced_id),
            0,
            &mut schema_nodes,
        )?;
        self.register_base(&e.descriptor)?;
        self.modalities
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }
    pub fn register_transition(&mut self, e: TransitionExtension) -> DockResult<()> {
        validate_tokens(&e.required_capabilities, &e.descriptor.namespaced_id)?;
        validate_tokens(&e.incompatible_capabilities, &e.descriptor.namespaced_id)?;
        let required: HashSet<_> = e.required_capabilities.iter().collect();
        if e.incompatible_capabilities
            .iter()
            .any(|capability| required.contains(capability))
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Transition capability declarations conflict.",
                e.descriptor.namespaced_id,
            ));
        }
        self.register_base(&e.descriptor)?;
        self.transitions
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }
    pub fn register_persistence_codec(&mut self, e: PersistenceCodecExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.codecs.insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }
    pub fn register_command(&mut self, e: CommandExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.commands.insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }
    pub fn register_host_adapter(&mut self, e: HostAdapterExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.host_adapters
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }
    pub fn register_layout_provider(&mut self, e: LayoutProviderExtension) -> DockResult<()> {
        let modality = e.provider.modality();
        crate::custom_layout::validate_layout_provider_modality(modality)?;
        if crate::custom_layout::is_builtin_layout_modality(modality) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "A built-in layout modality cannot be replaced by an extension provider.",
                modality,
            ));
        }
        if self.layout_provider_by_modality.contains_key(modality) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "A layout modality already has a registered provider.",
                modality,
            ));
        }
        self.register_base(&e.descriptor)?;
        self.layout_provider_by_modality
            .insert(modality.to_owned(), e.descriptor.namespaced_id.clone());
        self.layout_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }

    /// Register a lifecycle-managed source of declarative workspace profiles.
    /// Profiles remain recipes applied to the canonical document; providers do
    /// not own a parallel layout tree.
    pub fn register_profile_provider(&mut self, e: ProfileProviderExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.profile_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }

    pub fn register_renderer_provider(&mut self, e: RendererProviderExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.renderer_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }

    pub fn register_panel_shell_provider(
        &mut self,
        e: PanelShellProviderExtension,
    ) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.panel_shell_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }

    /// Obtain a validated semantic shell model from an active provider.
    pub fn describe_panel_shell(
        &self,
        provider_id: &str,
        panel: &PanelDescriptor,
    ) -> DockResult<PanelShellSpec> {
        self.require_active(provider_id)?;
        let extension = self.panel_shell_providers.get(provider_id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Panel-shell provider was not found.",
                provider_id,
            )
        })?;
        let spec = catch_unwind(AssertUnwindSafe(|| (extension.describe_shell)(panel))).map_err(
            |_| {
                DockError::new(
                    ErrorCode::ExtensionError,
                    "Panel-shell provider panicked.",
                    provider_id,
                )
            },
        )??;
        validate_panel_shell_spec(&spec, panel)?;
        Ok(spec)
    }

    pub fn register_input_provider(&mut self, e: InputProviderExtension) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.input_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }

    /// Route a normalized event through an active provider and validate its
    /// returned operation batch on an isolated canonical snapshot.
    pub fn route_input(
        &self,
        provider_id: &str,
        document: &LayoutDocument,
        event: &InputEvent,
    ) -> DockResult<InputIntent> {
        self.require_active(provider_id)?;
        validate_input_event(event)?;
        validate_layout(document)?;
        let extension = self.input_providers.get(provider_id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Input provider was not found.",
                provider_id,
            )
        })?;
        let intent = catch_unwind(AssertUnwindSafe(|| {
            (extension.route_input)(document, event)
        }))
        .map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Input provider panicked.",
                provider_id,
            )
        })??;
        validate_input_intent(document, &intent)?;
        Ok(intent)
    }

    pub fn register_accessibility_provider(
        &mut self,
        e: AccessibilityProviderExtension,
    ) -> DockResult<()> {
        self.register_base(&e.descriptor)?;
        self.accessibility_providers
            .insert(e.descriptor.namespaced_id.clone(), e);
        Ok(())
    }

    /// Request a bounded semantic tree from an active provider. OS-specific
    /// accessibility APIs remain the responsibility of a host adapter.
    pub fn accessibility_tree(
        &self,
        provider_id: &str,
        document: &LayoutDocument,
    ) -> DockResult<AccessibilityTree> {
        self.require_active(provider_id)?;
        validate_layout(document)?;
        let extension = self
            .accessibility_providers
            .get(provider_id)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Accessibility provider was not found.",
                    provider_id,
                )
            })?;
        let tree = catch_unwind(AssertUnwindSafe(|| (extension.describe_tree)(document)))
            .map_err(|_| {
                DockError::new(
                    ErrorCode::ExtensionError,
                    "Accessibility provider panicked.",
                    provider_id,
                )
            })??;
        validate_accessibility_tree(&tree)?;
        Ok(tree)
    }

    /// Request renderer facts from an active provider. Projection remains
    /// callback-free; callers explicitly attach this report to URNE.
    pub fn renderer_capability_report(
        &self,
        provider_id: &str,
    ) -> DockResult<RendererCapabilityReport> {
        self.require_active(provider_id)?;
        let extension = self.renderer_providers.get(provider_id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Renderer capability provider was not found.",
                provider_id,
            )
        })?;
        let report = catch_unwind(AssertUnwindSafe(|| (extension.capability_report)())).map_err(
            |_| {
                DockError::new(
                    ErrorCode::InvalidLayout,
                    "Renderer capability provider panicked.",
                    provider_id,
                )
            },
        )??;
        report.validate()?;
        Ok(report)
    }

    /// Resolve and validate a named profile from an active provider.
    pub fn provide_profile(
        &self,
        provider_id: &str,
        profile_id: &str,
    ) -> DockResult<WorkspaceProfile> {
        let extension = self.profile_providers.get(provider_id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Profile provider is unavailable.",
                provider_id,
            )
        })?;
        self.require_active(provider_id)?;
        if profile_id.trim().is_empty() || profile_id.len() > 256 {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Requested profile ID must be nonempty and bounded.",
                profile_id,
            ));
        }
        let profile = catch_unwind(AssertUnwindSafe(|| (extension.provide_profile)(profile_id)))
            .map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Profile provider panicked.",
                provider_id,
            )
        })??;
        if profile.id != profile_id {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Profile provider returned a different profile ID than requested.",
                format!("requested={profile_id}; returned={}", profile.id),
            ));
        }
        validate_profile(&profile)?;
        Ok(profile)
    }

    pub fn arrange_layout(
        &self,
        request: &ExtensionLayoutRequest<'_>,
    ) -> DockResult<Vec<ExtensionPlacement>> {
        let id = self
            .layout_provider_by_modality
            .get(request.modality)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "No extension layout provider is registered for this modality.",
                    request.modality,
                )
            })?;
        self.require_active(id)?;
        let extension = self.layout_providers.get(id).ok_or_else(|| {
            DockError::new(
                ErrorCode::Conflict,
                "Registered layout provider index is inconsistent.",
                id,
            )
        })?;
        crate::custom_layout::arrange_layout_provider(extension.provider.as_ref(), request)
    }

    pub fn has_layout_provider(&self, modality: &str) -> bool {
        self.layout_provider_by_modality.contains_key(modality)
    }
    pub fn activate(&mut self, id: &str) -> DockResult<()> {
        if !self.frozen {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Extensions can only activate after registry freeze.",
                id,
            ));
        }
        if self.active.contains(id) {
            return Ok(());
        }
        let d = self.descriptors.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Extension is not registered.", id)
        })?;
        if let Some(hook) = &d.on_activate {
            catch_unwind(AssertUnwindSafe(|| hook())).map_err(|_| {
                DockError::new(
                    ErrorCode::ExtensionError,
                    "Extension activation panicked.",
                    id,
                )
            })??;
        }
        self.active.insert(id.to_owned());
        Ok(())
    }
    pub fn deactivate(&mut self, id: &str) -> DockResult<()> {
        if !self.active.contains(id) {
            return Ok(());
        }
        let d = self.descriptors.get(id).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Active extension is not registered.",
                id,
            )
        })?;
        if let Some(hook) = &d.on_shutdown {
            catch_unwind(AssertUnwindSafe(|| hook())).map_err(|_| {
                DockError::new(
                    ErrorCode::ExtensionError,
                    "Extension shutdown panicked.",
                    id,
                )
            })?;
        }
        self.active.remove(id);
        Ok(())
    }
    pub fn shutdown(&mut self) -> DockResult<()> {
        let mut first = None;
        for id in self.order.clone().into_iter().rev() {
            if let Err(e) = self.deactivate(&id)
                && first.is_none()
            {
                first = Some(e);
            }
        }
        first.map_or(Ok(()), Err)
    }
    pub fn validate_panel(&self, id: &str, panel: &PanelDescriptor) -> DockResult<()> {
        let e = self.panel_providers.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Panel provider is unavailable.", id)
        })?;
        self.require_active(id)?;
        catch_unwind(AssertUnwindSafe(|| (e.validate_panel)(panel))).map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Panel provider validation panicked.",
                id,
            )
        })?
    }
    pub fn validate_modality_config(&self, id: &ModalityId, config: &Value) -> DockResult<()> {
        let e = self.modalities.get(&id.0).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Modality extension is unavailable.",
                id.0.clone(),
            )
        })?;
        self.require_active(&id.0)?;
        validate_config_value(config, &e.configuration_schema, &format!("{}.config", id.0))?;
        catch_unwind(AssertUnwindSafe(|| (e.validate_config)(config))).map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Modality configuration validator panicked.",
                id.0.clone(),
            )
        })?
    }
    pub fn modality_configuration_schema(&self, id: &ModalityId) -> DockResult<&Value> {
        self.modalities
            .get(&id.0)
            .map(|e| &e.configuration_schema)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Modality extension is unavailable.",
                    id.0.clone(),
                )
            })
    }
    pub fn modality_capabilities(&self, id: &ModalityId) -> DockResult<ModalityCapabilityReport> {
        let e = self.modalities.get(&id.0).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Modality extension is unavailable.",
                id.0.clone(),
            )
        })?;
        Ok(ModalityCapabilityReport {
            operations: e.operations.clone(),
            capabilities: e.capabilities.clone(),
            required_capabilities: e.required_capabilities.clone(),
            incompatible_capabilities: e.incompatible_capabilities.clone(),
            fallback: e.fallback,
        })
    }
    pub fn modality_supports_operation(
        &self,
        id: &ModalityId,
        operation: &str,
    ) -> DockResult<bool> {
        Ok(self
            .modality_capabilities(id)?
            .operations
            .iter()
            .any(|x| x == operation))
    }
    pub fn validate_modality_combination(&self, selected: &[ModalityId]) -> DockResult<()> {
        let mut ids = HashSet::new();
        let mut provided = HashSet::new();
        let mut extensions = Vec::new();
        for id in selected {
            if !ids.insert(id.0.as_str()) {
                return Err(DockError::new(
                    ErrorCode::DuplicateId,
                    "Modality combination contains a duplicate ID.",
                    id.0.clone(),
                ));
            }
            let e = self.modalities.get(&id.0).ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Modality in combination is unavailable.",
                    id.0.clone(),
                )
            })?;
            provided.extend(e.capabilities.iter().map(String::as_str));
            extensions.push(e);
        }
        for e in extensions {
            for required in &e.required_capabilities {
                if !provided.contains(required.as_str()) {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Required modality capability is unavailable.",
                        format!("{}.{}", e.descriptor.namespaced_id, required),
                    ));
                }
            }
            for banned in &e.incompatible_capabilities {
                if provided.contains(banned.as_str()) {
                    return Err(DockError::new(
                        ErrorCode::UnsupportedOperation,
                        "Modality combination contains an incompatible capability.",
                        format!("{}.{}", e.descriptor.namespaced_id, banned),
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn transition_duration(
        &self,
        id: &TransitionId,
        request: &TransitionRequest,
        max_ms: u32,
    ) -> DockResult<u32> {
        if request.reduced_motion || request.duration_ms == 0 {
            return Ok(0);
        }
        let e = self.transitions.get(&id.0).ok_or_else(|| {
            DockError::new(
                ErrorCode::NotFound,
                "Transition extension is unavailable.",
                id.0.clone(),
            )
        })?;
        self.require_active(&id.0)?;
        validate_tokens(&request.host_capabilities, &id.0)?;
        for required in &e.required_capabilities {
            if !request.host_capabilities.contains(required) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Transition provider requires an unavailable host capability.",
                    format!("{}.{}", id.0, required),
                ));
            }
        }
        for incompatible in &e.incompatible_capabilities {
            if request.host_capabilities.contains(incompatible) {
                return Err(DockError::new(
                    ErrorCode::UnsupportedOperation,
                    "Transition provider is incompatible with the active host capability.",
                    format!("{}.{}", id.0, incompatible),
                ));
            }
        }
        catch_unwind(AssertUnwindSafe(|| (e.duration)(request)))
            .map(|d| d.min(max_ms))
            .map_err(|_| {
                DockError::new(
                    ErrorCode::ExtensionError,
                    "Transition duration provider panicked.",
                    id.0.clone(),
                )
            })
    }
    pub fn encode_extension_state(&self, id: &str, state: &Value) -> DockResult<Value> {
        let e = self.codecs.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Persistence codec is unavailable.", id)
        })?;
        self.require_active(id)?;
        catch_unwind(AssertUnwindSafe(|| (e.encode)(state))).map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Persistence encoder panicked.",
                id,
            )
        })?
    }
    pub fn decode_extension_state(&self, id: &str, state: &Value) -> DockResult<Value> {
        let e = self.codecs.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Persistence codec is unavailable.", id)
        })?;
        self.require_active(id)?;
        catch_unwind(AssertUnwindSafe(|| (e.decode)(state))).map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Persistence decoder panicked.",
                id,
            )
        })?
    }
    pub fn execute_command(
        &self,
        id: &str,
        document: &LayoutDocument,
        arguments: &Value,
    ) -> DockResult<TransactionResult> {
        let e = self.commands.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Command provider is unavailable.", id)
        })?;
        self.require_active(id)?;
        let result = catch_unwind(AssertUnwindSafe(|| (e.execute)(document, arguments))).map_err(
            |_| DockError::new(ErrorCode::ExtensionError, "Command provider panicked.", id),
        )??;
        if result.document.id != document.id || result.document.workspace != document.workspace {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Extension command changed layout ownership.",
                id,
            ));
        }
        validate_layout(&result.document)?;
        if diff_layouts(document, &result.document) != result.diff {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Extension command returned a diff inconsistent with its target layout.",
                id,
            ));
        }
        Ok(result)
    }
    pub fn validate_host_diff(&self, id: &str, diff: &LayoutDiff) -> DockResult<()> {
        let e = self.host_adapters.get(id).ok_or_else(|| {
            DockError::new(ErrorCode::NotFound, "Host adapter is unavailable.", id)
        })?;
        self.require_active(id)?;
        catch_unwind(AssertUnwindSafe(|| (e.validate_diff)(diff))).map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Host adapter validation panicked.",
                id,
            )
        })?
    }
}
fn validate_panel_shell_spec(spec: &PanelShellSpec, panel: &PanelDescriptor) -> DockResult<()> {
    let invalid_text =
        |value: &str| value.is_empty() || value.len() > 512 || value.chars().any(char::is_control);
    if spec.panel_id != panel.id
        || invalid_text(&spec.title_key)
        || invalid_text(&spec.accessible_label)
        || spec.actions.len() > 32
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Panel-shell output must match the requested panel and contain bounded labels/actions.",
            panel.id.0.clone(),
        ));
    }
    let mut ids = HashSet::new();
    for action in &spec.actions {
        if action.id.is_empty()
            || action.id.len() > 128
            || !action
                .id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_.:-".contains(character))
            || invalid_text(&action.title_key)
            || !ids.insert(&action.id)
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Panel-shell actions must have unique bounded IDs and labels.",
                &action.id,
            ));
        }
    }
    Ok(())
}

fn validate_input_event(event: &InputEvent) -> DockResult<()> {
    let invalid_text =
        |value: &str| value.is_empty() || value.len() > 256 || value.chars().any(char::is_control);
    if event.target_id.as_deref().is_some_and(invalid_text)
        || event.key.as_deref().is_some_and(invalid_text)
        || event.modifiers.len() > 8
        || event.position.is_some_and(|position| {
            position
                .iter()
                .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
        })
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Normalized input event contains an invalid or unbounded field.",
            "input-event",
        ));
    }
    let mut modifiers = HashSet::new();
    if event.modifiers.iter().any(|modifier| {
        modifier.is_empty()
            || modifier.len() > 32
            || !modifier
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_.-".contains(character))
            || !modifiers.insert(modifier)
    }) {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Input event modifiers must be unique bounded tokens.",
            "input-event",
        ));
    }
    Ok(())
}

fn validate_input_intent(document: &LayoutDocument, intent: &InputIntent) -> DockResult<()> {
    if intent.operations.len() > 32
        || intent.focus_target.as_deref().is_some_and(|target| {
            target.is_empty() || target.len() > 256 || target.chars().any(char::is_control)
        })
        || (!intent.consumed && (!intent.operations.is_empty() || intent.focus_target.is_some()))
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Input intent contains an invalid focus target or unbounded/unconsumed actions.",
            "input-intent",
        ));
    }
    if !intent.operations.is_empty() {
        let mut preview = DockingService::new(document.clone(), 0)?;
        preview.transact(
            document.revision,
            CommandId::from("extension-input-preview"),
            intent.operations.clone(),
        )?;
    }
    Ok(())
}

fn validate_accessibility_tree(tree: &AccessibilityTree) -> DockResult<()> {
    let mut ids = HashSet::new();
    let mut node_count = 0usize;
    fn visit(
        node: &AccessibilityNodeSpec,
        depth: usize,
        ids: &mut HashSet<String>,
        node_count: &mut usize,
    ) -> DockResult<()> {
        *node_count += 1;
        let invalid_text = |value: &str, limit: usize| {
            value.is_empty() || value.len() > limit || value.chars().any(char::is_control)
        };
        if depth > 16
            || *node_count > 512
            || node.id.len() > 256
            || !valid_extension_token(&node.id)
            || !ids.insert(node.id.clone())
            || invalid_text(&node.label, 512)
            || node.description.len() > 2048
            || node.description.chars().any(char::is_control)
            || node.actions.len() > 32
            || node.children.len() > 128
        {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Accessibility tree contains invalid identity or exceeds bounded depth/size.",
                &node.id,
            ));
        }
        let mut actions = HashSet::new();
        if node.actions.iter().any(|action| {
            action.len() > 128 || !valid_extension_token(action) || !actions.insert(action)
        }) {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Accessibility actions must have unique bounded IDs.",
                &node.id,
            ));
        }
        for child in &node.children {
            visit(child, depth + 1, ids, node_count)?;
        }
        Ok(())
    }
    visit(&tree.root, 0, &mut ids, &mut node_count)
}

fn valid_extension_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.:-".contains(character))
}

fn validate_tokens(values: &[String], context: &str) -> DockResult<()> {
    let mut unique = HashSet::new();
    for value in values {
        if value.is_empty()
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
            || !unique.insert(value)
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Modality declarations must be unique nonempty tokens.",
                context,
            ));
        }
    }
    Ok(())
}
fn supported_type(t: &str) -> bool {
    matches!(
        t,
        "object" | "array" | "string" | "number" | "integer" | "boolean" | "null"
    )
}
fn matches_type(v: &Value, t: &str) -> bool {
    match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        _ => v.is_null(),
    }
}
fn validate_schema_declaration(
    s: &Value,
    path: &str,
    depth: usize,
    nodes: &mut usize,
) -> DockResult<()> {
    if !s.is_object() {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Configuration schema nodes must be objects.",
            path,
        ));
    }
    *nodes += 1;
    if *nodes > 1024 || depth > 32 {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Configuration schema exceeds structural limits.",
            path,
        ));
    }
    let object = s.as_object().unwrap();
    let allowed = [
        "type",
        "enum",
        "required",
        "additionalProperties",
        "properties",
        "items",
    ];
    if let Some(k) = object.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Configuration schema keyword is not supported.",
            format!("{path}.{k}"),
        ));
    }
    if let Some(t) = s.get("type") {
        let Some(t) = t.as_str() else {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Schema type must be a string.",
                format!("{path}.type"),
            ));
        };
        if !supported_type(t) {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Schema type is unsupported.",
                format!("{path}.type"),
            ));
        }
    }
    if let Some(values) = s.get("enum") {
        if !values.as_array().is_some_and(|v| !v.is_empty()) {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Schema enum must be nonempty array.",
                format!("{path}.enum"),
            ));
        }
        if let Some(t) = s.get("type").and_then(Value::as_str)
            && values
                .as_array()
                .unwrap()
                .iter()
                .any(|v| !matches_type(v, t))
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Schema enum item has wrong type.",
                format!("{path}.enum"),
            ));
        }
    }
    if let Some(req) = s.get("required") {
        let Some(req) = req.as_array() else {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Schema required must be an array.",
                format!("{path}.required"),
            ));
        };
        let mut seen = HashSet::new();
        for x in req {
            let Some(x) = x.as_str() else {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Schema required entries must be strings.",
                    format!("{path}.required"),
                ));
            };
            if x.is_empty() || !seen.insert(x) {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Schema required entries must be unique and nonempty.",
                    format!("{path}.required"),
                ));
            }
        }
    }
    if s.get("additionalProperties")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Schema additionalProperties must be boolean.",
            format!("{path}.additionalProperties"),
        ));
    }
    if let Some(props) = s.get("properties") {
        let Some(props) = props.as_object() else {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Schema properties must be an object.",
                format!("{path}.properties"),
            ));
        };
        for (k, v) in props {
            validate_schema_declaration(v, &format!("{path}.properties.{k}"), depth + 1, nodes)?;
        }
    }
    if let Some(items) = s.get("items") {
        validate_schema_declaration(items, &format!("{path}.items"), depth + 1, nodes)?;
    }
    Ok(())
}
fn validate_config_value(value: &Value, schema: &Value, path: &str) -> DockResult<()> {
    if let Some(t) = schema.get("type").and_then(Value::as_str)
        && !matches_type(value, t)
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Configuration value has the wrong type.",
            path,
        ));
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array)
        && !options.contains(value)
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Configuration value is outside its enum.",
            path,
        ));
    }
    if let Some(obj) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for name in required.iter().filter_map(Value::as_str) {
                if !obj.contains_key(name) {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Required configuration property is missing.",
                        format!("{path}.{name}"),
                    ));
                }
            }
        }
        for (key, value) in obj {
            let sub = schema.get("properties").and_then(|p| p.get(key));
            if let Some(sub) = sub {
                validate_config_value(value, sub, &format!("{path}.{key}"))?;
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Unknown configuration property.",
                    format!("{path}.{key}"),
                ));
            }
        }
    }
    if let (Some(values), Some(items)) = (value.as_array(), schema.get("items")) {
        for (i, x) in values.iter().enumerate() {
            validate_config_value(x, items, &format!("{path}[{i}]"))?;
        }
    }
    Ok(())
}
