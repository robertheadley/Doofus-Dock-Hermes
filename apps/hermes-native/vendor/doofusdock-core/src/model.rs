use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

macro_rules! string_id {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl From<&str> for $name { fn from(value: &str) -> Self { Self(value.to_owned()) } }
        impl From<String> for $name { fn from(value: String) -> Self { Self(value) } }
        impl $name { pub fn is_empty(&self) -> bool { self.0.is_empty() } }
    )+ };
}
string_id!(
    PanelId,
    NodeId,
    WindowId,
    ModalityId,
    TransitionId,
    LayoutId,
    WorkspaceId,
    CommandId
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}
impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
impl Rect {
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeConstraints {
    pub minimum: Size,
    pub maximum: Size,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
#[derive(Default)]
pub enum ClosePolicy {
    #[default]
    Allowed = 0,
    HideOnly = 1,
    Forbidden = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
#[derive(Default)]
pub enum PersistencePolicy {
    #[default]
    Persistent = 0,
    SessionOnly = 1,
    Never = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
#[derive(Default)]
pub enum ReparentCapability {
    #[default]
    Supported = 0,
    ReinitializeNativeContent = 1,
    Unsupported = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum DockEdge {
    Left = 0,
    Right = 1,
    Top = 2,
    Bottom = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidId,
    DuplicateId,
    NotFound,
    InvalidLayout,
    ConstraintViolation,
    UnsupportedOperation,
    SerializationError,
    SchemaError,
    ResourceLimit,
    ExtensionError,
    PersistenceError,
    Conflict,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockError {
    pub code: ErrorCode,
    pub message: String,
    pub context: String,
}
pub type DockResult<T> = Result<T, DockError>;
impl DockError {
    pub fn new(code: ErrorCode, message: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: context.into(),
        }
    }
}
impl std::fmt::Display for DockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.message, self.context)
    }
}
impl std::error::Error for DockError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeKind {
    #[serde(rename = "panel")]
    Panel { panel: PanelId },
    #[serde(rename = "tabs")]
    Tabs {
        #[serde(rename = "active")]
        active_index: usize,
        /// Canonical collapsed state for this whole tab group. This is
        /// additive and defaults to expanded when loading older documents.
        #[serde(default)]
        minimized: bool,
        items: Vec<NodePtr>,
    },
    #[serde(rename = "document_well")]
    DocumentWell {
        active_group: usize,
        groups: Vec<NodePtr>,
    },
    #[serde(rename = "split")]
    Split {
        orientation: Orientation,
        weights: Vec<f64>,
        children: Vec<NodePtr>,
    },
    #[serde(rename = "placeholder")]
    Placeholder { panel: PanelId, title: String },
    #[serde(rename = "extension")]
    Extension {
        modality: ModalityId,
        payload: String,
        children: Vec<NodePtr>,
    },
}
pub type NodePtr = Arc<Node>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    #[serde(flatten)]
    pub kind: NodeKind,
}
impl Node {
    pub fn panel(id: impl Into<NodeId>, panel: impl Into<PanelId>) -> NodePtr {
        Arc::new(Self {
            id: id.into(),
            kind: NodeKind::Panel {
                panel: panel.into(),
            },
        })
    }
    pub fn tabs(id: impl Into<NodeId>, items: Vec<NodePtr>, active_index: usize) -> NodePtr {
        Self::tabs_with_minimized(id, items, active_index, false)
    }
    pub fn tabs_with_minimized(
        id: impl Into<NodeId>,
        items: Vec<NodePtr>,
        active_index: usize,
        minimized: bool,
    ) -> NodePtr {
        Arc::new(Self {
            id: id.into(),
            kind: NodeKind::Tabs {
                active_index,
                minimized,
                items,
            },
        })
    }
    pub fn split(
        id: impl Into<NodeId>,
        orientation: Orientation,
        children: Vec<NodePtr>,
        weights: Vec<f64>,
    ) -> NodePtr {
        let weights = if weights.is_empty() && !children.is_empty() {
            vec![1.0 / children.len() as f64; children.len()]
        } else {
            weights
        };
        Arc::new(Self {
            id: id.into(),
            kind: NodeKind::Split {
                orientation,
                weights,
                children,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockingCapabilities {
    pub allow_tab: bool,
    pub allow_split: bool,
    pub allow_float: bool,
    pub allow_auto_hide: bool,
    pub allowed_edges: Vec<DockEdge>,
}
impl Default for DockingCapabilities {
    fn default() -> Self {
        Self {
            allow_tab: true,
            allow_split: true,
            allow_float: true,
            allow_auto_hide: true,
            allowed_edges: vec![
                DockEdge::Left,
                DockEdge::Right,
                DockEdge::Top,
                DockEdge::Bottom,
            ],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanelDescriptor {
    pub id: PanelId,
    pub title_key: String,
    pub icon_key: String,
    #[serde(rename = "min", with = "size_array")]
    pub minimum: Size,
    #[serde(rename = "max", with = "size_array")]
    pub maximum: Size,
    #[serde(default)]
    pub close_policy: ClosePolicy,
    #[serde(default)]
    pub persistence: PersistencePolicy,
    #[serde(rename = "provider")]
    pub provider_key: String,
    /// Application-defined semantic role used by rules and accessibility.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
    /// Bounded application metadata; stable keys are available to declarative rules.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// Logical workspace tags for projection; empty means shared in every context.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub workspace_memberships: BTreeSet<String>,
    #[serde(default)]
    pub reparent: ReparentCapability,
    #[serde(default)]
    pub docking: DockingCapabilities,
}
impl PanelDescriptor {
    pub fn new(id: impl Into<PanelId>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title_key: title.into(),
            icon_key: String::new(),
            minimum: Size::default(),
            maximum: Size::default(),
            close_policy: ClosePolicy::default(),
            persistence: PersistencePolicy::default(),
            provider_key: String::new(),
            role: String::new(),
            metadata: BTreeMap::new(),
            workspace_memberships: BTreeSet::new(),
            reparent: ReparentCapability::default(),
            docking: DockingCapabilities::default(),
        }
    }
}
fn default_scale() -> f64 {
    1.0
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScratchpadConfig {
    pub name: String,
    #[serde(default = "default_scratchpad_visible")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon_position: Option<Point>,
    #[serde(default)]
    pub auto_hide_on_deactivate: bool,
    /// Keep this scratchpad discoverable from other workspaces. Its canonical
    /// document remains the sole owner; summoning activates that workspace.
    #[serde(default)]
    pub pinned_across_workspaces: bool,
}

fn default_scratchpad_visible() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatingWindow {
    pub id: WindowId,
    #[serde(with = "rect_array")]
    pub geometry: Rect,
    #[serde(rename = "monitor", default)]
    pub monitor_id: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(rename = "z_order", default)]
    pub z_order_hint: i32,
    /// Persisted single-panel native picture-in-picture presentation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub picture_in_picture: bool,
    /// Optional named scratchpad metadata; visibility remains canonical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scratchpad: Option<ScratchpadConfig>,
    pub root: NodePtr,
}
fn is_false(value: &bool) -> bool {
    !value
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutoHidePanel {
    pub panel: PanelId,
    pub edge: DockEdge,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutDocument {
    pub schema_version: u32,
    #[serde(rename = "layout_id")]
    pub id: LayoutId,
    #[serde(rename = "workspace_id")]
    pub workspace: WorkspaceId,
    pub revision: u64,
    /// Field order matches the canonical C++ JSON schema for checksum compatibility.
    #[serde(default)]
    pub panels: Arc<Vec<PanelDescriptor>>,
    #[serde(default)]
    pub closed_panels: Vec<PanelId>,
    /// Durable tab pin preferences, retained while content is closed or floating.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub pinned_panels: BTreeSet<PanelId>,
    #[serde(default)]
    pub auto_hide_panels: Vec<AutoHidePanel>,
    #[serde(default)]
    pub maximized_panel: Option<PanelId>,
    pub root: Option<NodePtr>,
    #[serde(rename = "windows", default)]
    pub floating_windows: Vec<FloatingWindow>,
}
impl LayoutDocument {
    pub fn new(id: impl Into<LayoutId>, workspace: impl Into<WorkspaceId>) -> Self {
        Self {
            schema_version: 1,
            id: id.into(),
            workspace: workspace.into(),
            revision: 0,
            root: None,
            floating_windows: vec![],
            panels: Arc::new(vec![]),
            closed_panels: vec![],
            pinned_panels: BTreeSet::new(),
            auto_hide_panels: vec![],
            maximized_panel: None,
        }
    }
    pub fn descriptor(&self, panel: &PanelId) -> Option<&PanelDescriptor> {
        self.panels.iter().find(|d| &d.id == panel)
    }
}

pub fn validate_layout(doc: &LayoutDocument) -> DockResult<()> {
    if doc.id.is_empty() || doc.workspace.is_empty() {
        return Err(DockError::new(
            ErrorCode::InvalidId,
            "Layout and workspace IDs are required.",
            "document",
        ));
    }
    if doc.schema_version != 1 {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Unsupported layout schema version.",
            doc.schema_version.to_string(),
        ));
    }
    for pinned in &doc.pinned_panels {
        if doc.descriptor(pinned).is_none() {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Pinned panel is not registered.",
                &pinned.0,
            ));
        }
    }
    let mut descriptors = HashSet::new();
    for p in doc.panels.iter() {
        if p.id.is_empty() {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Panel descriptor ID cannot be empty.",
                "panels",
            ));
        }
        if !descriptors.insert(p.id.clone()) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Panel descriptors must have unique IDs.",
                p.id.0.clone(),
            ));
        }
        if p.minimum.width < 0
            || p.minimum.height < 0
            || p.maximum.width < 0
            || p.maximum.height < 0
            || (p.maximum.width != 0 && p.maximum.width < p.minimum.width)
            || (p.maximum.height != 0 && p.maximum.height < p.minimum.height)
        {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Panel size constraints are invalid.",
                p.id.0.clone(),
            ));
        }
        let edges: HashSet<_> = p.docking.allowed_edges.iter().collect();
        if edges.len() != p.docking.allowed_edges.len() {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Panel docking edges must be unique.",
                p.id.0.clone(),
            ));
        }
        for value in [&p.title_key, &p.icon_key, &p.provider_key] {
            if value.len() > 4096 {
                return Err(DockError::new(
                    ErrorCode::ResourceLimit,
                    "Panel descriptor string exceeds limit.",
                    p.id.0.clone(),
                ));
            }
        }
        if p.role.len() > 128
            || !p
                .role
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_.:-".contains(character))
            || p.metadata.len() > 64
            || p.workspace_memberships.len() > 64
            || p.workspace_memberships.iter().any(|tag| {
                tag.is_empty()
                    || tag.len() > 64
                    || !tag.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_.:-".contains(character)
                    })
            })
            || p.metadata.iter().any(|(key, value)| {
                key.is_empty()
                    || key.len() > 128
                    || !key.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_.:-".contains(character)
                    })
                    || value.len() > 1024
                    || value.chars().any(char::is_control)
            })
        {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Panel role or application metadata is invalid or exceeds its bound.",
                p.id.0.clone(),
            ));
        }
    }
    let mut nodes = HashSet::new();
    let mut live = HashSet::new();
    let mut count = 0usize;
    fn visit(
        n: &Node,
        depth: usize,
        nodes: &mut HashSet<NodeId>,
        live: &mut HashSet<PanelId>,
        count: &mut usize,
    ) -> DockResult<()> {
        *count += 1;
        if *count > 10_000 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Layout exceeds the maximum node count.",
                "tree",
            ));
        }
        if depth > 128 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Layout exceeds maximum nesting depth.",
                n.id.0.clone(),
            ));
        }
        if n.id.is_empty() {
            return Err(DockError::new(
                ErrorCode::InvalidId,
                "Node ID cannot be empty.",
                "tree",
            ));
        }
        if n.id.0.len() > 4096 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Node ID exceeds the configured string limit.",
                n.id.0.clone(),
            ));
        }
        if !nodes.insert(n.id.clone()) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Node IDs must be unique.",
                n.id.0.clone(),
            ));
        }
        let recurse = |children: &[NodePtr],
                       nodes: &mut HashSet<NodeId>,
                       live: &mut HashSet<PanelId>,
                       count: &mut usize|
         -> DockResult<()> {
            for c in children {
                visit(c, depth + 1, nodes, live, count)?;
            }
            Ok(())
        };
        match &n.kind {
            NodeKind::Panel { panel } => {
                if panel.is_empty() {
                    return Err(DockError::new(
                        ErrorCode::InvalidId,
                        "Panel ID cannot be empty.",
                        n.id.0.clone(),
                    ));
                }
                if !live.insert(panel.clone()) {
                    return Err(DockError::new(
                        ErrorCode::DuplicateId,
                        "A panel may appear only once in a layout.",
                        panel.0.clone(),
                    ));
                }
                if panel.0.len() > 4096 {
                    return Err(DockError::new(
                        ErrorCode::ResourceLimit,
                        "Panel ID exceeds the configured string limit.",
                        panel.0.clone(),
                    ));
                }
            }
            NodeKind::Tabs {
                active_index,
                items,
                ..
            } => {
                if items.is_empty() || *active_index >= items.len() {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Tab group must have an active item.",
                        n.id.0.clone(),
                    ));
                }
                recurse(items, nodes, live, count)?;
            }
            NodeKind::DocumentWell {
                active_group,
                groups,
            } => {
                if groups.is_empty()
                    || *active_group >= groups.len()
                    || groups
                        .iter()
                        .any(|g| !matches!(g.kind, NodeKind::Tabs { .. }))
                {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Document well must contain active tab groups.",
                        n.id.0.clone(),
                    ));
                }
                recurse(groups, nodes, live, count)?;
            }
            NodeKind::Split {
                weights, children, ..
            } => {
                if children.len() < 2
                    || children.len() != weights.len()
                    || weights.iter().any(|w| !w.is_finite() || *w <= 0.0)
                    || (weights.iter().sum::<f64>() - 1.0).abs() > 1e-6
                {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Split must have at least two children and finite positive weights summing to one.",
                        n.id.0.clone(),
                    ));
                }
                recurse(children, nodes, live, count)?;
            }
            NodeKind::Extension {
                modality,
                payload,
                children,
            } => {
                let payload_limit = if modality.0 == crate::SCROLLABLE_COLUMNS_MODALITY_ID {
                    1024 * 1024
                } else {
                    4096
                };
                if modality.is_empty() || modality.0.len() > 4096 || payload.len() > payload_limit {
                    return Err(DockError::new(
                        ErrorCode::ResourceLimit,
                        "Invalid extension node payload or modality.",
                        n.id.0.clone(),
                    ));
                }
                if crate::is_stack_modality(modality) {
                    crate::StackConfiguration::parse(payload, children)?;
                }
                if modality.0 == crate::LAYERED_MODALITY {
                    crate::LayeredConfiguration::parse(payload, children)?;
                }
                if modality.0 == crate::SCROLLABLE_COLUMNS_MODALITY_ID {
                    crate::ScrollableColumnsConfiguration::from_node(n)?;
                }
                if crate::custom_layout::is_strategy_modality(&modality.0) {
                    crate::custom_layout::validate_strategy_payload(&modality.0, payload)?;
                }
                if matches!(
                    modality.0.as_str(),
                    "doofusdock.masonry" | "doofusdock.spatial" | "doofusdock.node_graph"
                ) {
                    let config: serde_json::Value =
                        serde_json::from_str(payload).map_err(|error| {
                            DockError::new(
                                ErrorCode::InvalidLayout,
                                format!("Invalid built-in container payload: {error}"),
                                &n.id.0,
                            )
                        })?;
                    if !config.is_object() {
                        return Err(DockError::new(
                            ErrorCode::InvalidLayout,
                            "Built-in container payload must be a JSON object.",
                            &n.id.0,
                        ));
                    }
                }
                recurse(children, nodes, live, count)?;
            }
            NodeKind::Placeholder { panel, title } => {
                if panel.is_empty() || panel.0.len() > 4096 || title.len() > 4096 {
                    return Err(DockError::new(
                        ErrorCode::InvalidLayout,
                        "Invalid placeholder.",
                        n.id.0.clone(),
                    ));
                }
            }
        }
        Ok(())
    }
    if let Some(root) = &doc.root {
        visit(root, 0, &mut nodes, &mut live, &mut count)?;
    }
    if doc.floating_windows.len() > 64 {
        return Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Too many floating windows.",
            "windows",
        ));
    }
    let mut window_ids = HashSet::new();
    let mut scratchpad_names = HashSet::new();
    for window in &doc.floating_windows {
        if window.id.is_empty() || !window_ids.insert(window.id.clone()) {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Floating window IDs must be valid and unique.",
                window.id.0.clone(),
            ));
        }
        if window.geometry.width < 0
            || window.geometry.height < 0
            || !window.scale.is_finite()
            || window.scale <= 0.0
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Floating window geometry or scale is invalid.",
                window.id.0.clone(),
            ));
        }
        if window.picture_in_picture {
            let bounds = window.geometry;
            let aspect = bounds.width as f64 / bounds.height.max(1) as f64;
            if !matches!(&window.root.kind, NodeKind::Panel { .. })
                || !(120..=1600).contains(&bounds.width)
                || !(68..=1200).contains(&bounds.height)
                || !(0.4..=3.0).contains(&aspect)
            {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Picture-in-picture requires one panel and bounded 120×68–1600×1200 geometry with aspect ratio 0.4–3.0.",
                    window.id.0.clone(),
                ));
            }
        }
        if window.monitor_id.len() > 4096 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Monitor ID exceeds string limit.",
                window.id.0.clone(),
            ));
        }
        if let Some(scratchpad) = &window.scratchpad
            && (scratchpad.name.is_empty()
                || scratchpad.name.len() > 64
                || !scratchpad.name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "-_.".contains(character)
                })
                || !scratchpad_names.insert(scratchpad.name.as_str()))
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Scratchpad names must be unique bounded ASCII tokens.",
                scratchpad.name.clone(),
            ));
        }
        visit(&window.root, 0, &mut nodes, &mut live, &mut count)?;
    }
    for panel in &live {
        if !descriptors.contains(panel) {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Layout references a panel without a descriptor.",
                panel.0.clone(),
            ));
        }
    }
    let mut closed = HashSet::new();
    for p in &doc.closed_panels {
        if !descriptors.contains(p) || live.contains(p) || !closed.insert(p.clone()) {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Closed panels must be known, unique, and absent from live layout.",
                p.0.clone(),
            ));
        }
    }
    let mut drawers = HashSet::new();
    for d in &doc.auto_hide_panels {
        if !descriptors.contains(&d.panel)
            || live.contains(&d.panel)
            || closed.contains(&d.panel)
            || !drawers.insert(d.panel.clone())
        {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Auto-hide panels must be unique and absent from active/closed sets.",
                d.panel.0.clone(),
            ));
        }
    }
    if doc
        .maximized_panel
        .as_ref()
        .is_some_and(|p| !live.contains(p))
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Maximized panel must be present in the active layout.",
            doc.maximized_panel.as_ref().unwrap().0.clone(),
        ));
    }
    Ok(())
}

pub fn walk(node: &Node, f: &mut impl FnMut(&Node)) {
    f(node);
    match &node.kind {
        NodeKind::Tabs { items, .. } => items.iter().for_each(|n| walk(n, f)),
        NodeKind::DocumentWell { groups, .. } => groups.iter().for_each(|n| walk(n, f)),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter().for_each(|n| walk(n, f))
        }
        _ => (),
    }
}
pub fn find_node<'a>(root: &'a Node, id: &NodeId) -> Option<&'a Node> {
    if &root.id == id {
        return Some(root);
    }
    match &root.kind {
        NodeKind::Tabs { items, .. } => items.iter().find_map(|n| find_node(n, id)),
        NodeKind::DocumentWell { groups, .. } => groups.iter().find_map(|n| find_node(n, id)),
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter().find_map(|n| find_node(n, id))
        }
        _ => None,
    }
}
pub fn find_panel_leaf<'a>(root: &'a Node, panel: &PanelId) -> Option<&'a Node> {
    if matches!(&root.kind, NodeKind::Panel { panel: p } if p == panel) {
        return Some(root);
    }
    match &root.kind {
        NodeKind::Tabs { items, .. } => items.iter().find_map(|n| find_panel_leaf(n, panel)),
        NodeKind::DocumentWell { groups, .. } => {
            groups.iter().find_map(|n| find_panel_leaf(n, panel))
        }
        NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
            children.iter().find_map(|n| find_panel_leaf(n, panel))
        }
        _ => None,
    }
}
pub fn find_panel_container(doc: &LayoutDocument, panel: &PanelId) -> Option<NodeId> {
    fn descend(n: &Node, parent: Option<&NodeId>, p: &PanelId) -> Option<NodeId> {
        if matches!(&n.kind, NodeKind::Panel { panel } if panel == p) {
            return Some(parent.unwrap_or(&n.id).clone());
        }
        match &n.kind {
            NodeKind::Tabs { items, .. } => items.iter().find_map(|c| descend(c, Some(&n.id), p)),
            NodeKind::DocumentWell { groups, .. } => {
                groups.iter().find_map(|c| descend(c, Some(&n.id), p))
            }
            NodeKind::Split { children, .. } | NodeKind::Extension { children, .. } => {
                children.iter().find_map(|c| descend(c, Some(&n.id), p))
            }
            _ => None,
        }
    }
    if let Some(root) = &doc.root
        && let Some(found) = descend(root, None, panel)
    {
        return Some(found);
    }
    doc.floating_windows
        .iter()
        .find_map(|w| descend(&w.root, None, panel))
}

mod size_array {
    use super::Size;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(v: &Size, s: S) -> Result<S::Ok, S::Error> {
        [v.width, v.height].serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Size, D::Error> {
        let [width, height] = <[i32; 2]>::deserialize(d)?;
        Ok(Size { width, height })
    }
}
mod rect_array {
    use super::Rect;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(v: &Rect, s: S) -> Result<S::Ok, S::Error> {
        [v.x, v.y, v.width, v.height].serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Rect, D::Error> {
        let [x, y, width, height] = <[i32; 4]>::deserialize(d)?;
        Ok(Rect {
            x,
            y,
            width,
            height,
        })
    }
}
