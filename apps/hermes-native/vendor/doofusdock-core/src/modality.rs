//! Typed structural changes and revision-bound presentation placement plans.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ACCORDION_MODALITY: &str = "doofusdock.accordion";
pub const SHADE_STACK_MODALITY: &str = "doofusdock.shade_stack";
pub const LAYERED_MODALITY: &str = "doofusdock.layered";

/// Persistent order and active layer for a stacked, overlapping container.
/// `back_to_front` is an exact permutation of the container's child IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredConfiguration {
    pub version: u32,
    pub back_to_front: Vec<NodeId>,
    pub active: NodeId,
}

impl LayeredConfiguration {
    pub fn parse(payload: &str, children: &[NodePtr]) -> DockResult<Self> {
        let ids: Vec<_> = children.iter().map(|child| child.id.clone()).collect();
        Self::parse_ids(payload, &ids)
    }

    pub fn parse_ids(payload: &str, child_ids: &[NodeId]) -> DockResult<Self> {
        let config: Self = serde_json::from_str(payload)
            .map_err(|e| DockError::new(ErrorCode::SchemaError, e.to_string(), "layered-config"))?;
        let child_id_count = child_ids.len();
        let child_ids: BTreeSet<_> = child_ids.iter().cloned().collect();
        let order_ids: BTreeSet<_> = config.back_to_front.iter().cloned().collect();
        if config.version != 1
            || child_ids.is_empty()
            || child_ids.len() != child_id_count
            || child_ids.len() > 4096
            || child_ids.len() != config.back_to_front.len()
            || order_ids.len() != config.back_to_front.len()
            || order_ids != child_ids
            || !child_ids.contains(&config.active)
        {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Layered version, active child, or back-to-front order is invalid.",
                "layered-config",
            ));
        }
        Ok(config)
    }

    pub fn from_children(children: &[NodePtr]) -> DockResult<Self> {
        let config = Self {
            version: 1,
            back_to_front: children.iter().map(|child| child.id.clone()).collect(),
            active: children
                .last()
                .map(|child| child.id.clone())
                .ok_or_else(|| {
                    DockError::new(
                        ErrorCode::ConstraintViolation,
                        "Cannot layer an empty container.",
                        "layered-config",
                    )
                })?,
        };
        Self::parse(&config.to_payload()?, children)
    }

    pub fn to_payload(&self) -> DockResult<String> {
        serde_json::to_string(self).map_err(|e| {
            DockError::new(
                ErrorCode::SerializationError,
                e.to_string(),
                "layered-config",
            )
        })
    }

    pub fn raise(&mut self, child: &NodeId) -> DockResult<()> {
        let Some(index) = self.back_to_front.iter().position(|id| id == child) else {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Layered child not found.",
                &child.0,
            ));
        };
        self.back_to_front.remove(index);
        self.back_to_front.push(child.clone());
        self.active = child.clone();
        Ok(())
    }

    pub fn lower(&mut self, child: &NodeId) -> DockResult<()> {
        let Some(index) = self.back_to_front.iter().position(|id| id == child) else {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Layered child not found.",
                &child.0,
            ));
        };
        self.back_to_front.remove(index);
        self.back_to_front.insert(0, child.clone());
        self.active = child.clone();
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackConfiguration {
    pub version: u32,
    pub multiple: bool,
    pub expanded: BTreeSet<NodeId>,
    pub selected: Option<NodeId>,
}
impl StackConfiguration {
    pub fn parse(payload: &str, children: &[NodePtr]) -> DockResult<Self> {
        let config: Self = serde_json::from_str(payload)
            .map_err(|e| DockError::new(ErrorCode::SchemaError, e.to_string(), "stack-config"))?;
        let ids: BTreeSet<_> = children.iter().map(|c| c.id.clone()).collect();
        if config.version != 1
            || (!config.multiple && config.expanded.len() > 1)
            || !config.expanded.is_subset(&ids)
            || config.selected.as_ref().is_some_and(|id| !ids.contains(id))
        {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Stack version, expansion mode, or child identity is invalid.",
                "stack-config",
            ));
        }
        Ok(config)
    }
    pub fn to_payload(&self) -> DockResult<String> {
        serde_json::to_string(self).map_err(|e| {
            DockError::new(ErrorCode::SerializationError, e.to_string(), "stack-config")
        })
    }
}
pub fn is_stack_modality(id: &ModalityId) -> bool {
    matches!(id.0.as_str(), ACCORDION_MODALITY | SHADE_STACK_MODALITY)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "configuration",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ContainerModality {
    Tabbed {
        active_index: usize,
    },
    Split {
        orientation: Orientation,
        weights: Vec<f64>,
    },
    Accordion {
        multiple: bool,
    },
    ShadeStack,
    Grid {
        columns: usize,
        gap: u32,
    },
    Spatial,
    NodeGraph,
    Layered,
    LayoutStrategy {
        strategy: LayoutStrategy,
        gap: i32,
    },
    ScrollableColumns {
        gap: i32,
        width: i32,
        minimum_width: i32,
        maximum_width: Option<i32>,
    },
}

pub(crate) fn converted_container(node: &Node, target: &ContainerModality) -> DockResult<NodeKind> {
    if let NodeKind::Extension {
        modality, payload, ..
    } = &node.kind
        && matches!(
            modality.0.as_str(),
            "doofusdock.masonry" | "doofusdock.spatial" | "doofusdock.node_graph"
        )
    {
        let config: serde_json::Value = serde_json::from_str(payload).map_err(|error| {
            DockError::new(
                ErrorCode::InvalidLayout,
                format!("Invalid built-in container payload: {error}"),
                &node.id.0,
            )
        })?;
        if !config.is_object() {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Built-in container payload must be a JSON object.",
                &node.id.0,
            ));
        }
    }
    if let NodeKind::Extension {
        modality,
        payload,
        children,
    } = &node.kind
        && let Ok(config) = serde_json::from_str::<serde_json::Value>(payload)
    {
        match (modality.0.as_str(), target) {
            ("doofusdock.masonry", ContainerModality::Grid { columns, gap }) => {
                if !(1..=64).contains(columns) || *gap > 256 {
                    return Err(DockError::new(
                        ErrorCode::ConstraintViolation,
                        "Grid columns must be 1–64 and gap at most 256.",
                        &node.id.0,
                    ));
                }
                let mut next = config.clone();
                next["columns"] = serde_json::json!(columns);
                next["gap"] = serde_json::json!(gap);
                return Ok(NodeKind::Extension {
                    modality: modality.clone(),
                    payload: next.to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.spatial", ContainerModality::Spatial) => {
                return Ok(NodeKind::Extension {
                    modality: modality.clone(),
                    payload: payload.clone(),
                    children: children.clone(),
                });
            }
            ("doofusdock.masonry", ContainerModality::Spatial) => {
                if let Some(saved) = config
                    .get("_spatial_restore")
                    .and_then(serde_json::Value::as_str)
                {
                    return Ok(NodeKind::Extension {
                        modality: "doofusdock.spatial".into(),
                        payload: saved.to_owned(),
                        children: children.clone(),
                    });
                }
                let columns = config
                    .get("columns")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(3)
                    .max(1) as usize;
                let count = children.len();
                let cols = columns.min(count.max(1));
                let rows = count.div_ceil(cols);
                let mut placements = Vec::with_capacity(count);
                for index in 0..count {
                    let col = index % cols;
                    let row = index / cols;
                    placements.push(serde_json::json!({"x": col as f64 / cols as f64, "y": row as f64 / rows as f64,
                        "width": 1.0 / cols as f64, "height": 1.0 / rows as f64}));
                }
                let spatial = serde_json::json!({"placements":placements,"_grid_restore":payload});
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.spatial".into(),
                    payload: spatial.to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.masonry", ContainerModality::NodeGraph) => {
                if let Some(saved) = config
                    .get("_node_graph_restore")
                    .and_then(serde_json::Value::as_str)
                {
                    return Ok(NodeKind::Extension {
                        modality: "doofusdock.node_graph".into(),
                        payload: saved.to_owned(),
                        children: children.clone(),
                    });
                }
                let graph = serde_json::json!({"_grid_restore": payload});
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.node_graph".into(),
                    payload: graph.to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.spatial", ContainerModality::Grid { columns, gap }) => {
                if let Some(saved) = config
                    .get("_grid_restore")
                    .and_then(serde_json::Value::as_str)
                {
                    return Ok(NodeKind::Extension {
                        modality: "doofusdock.masonry".into(),
                        payload: saved.to_owned(),
                        children: children.clone(),
                    });
                }
                let grid =
                    serde_json::json!({"columns":columns,"gap":gap,"_spatial_restore":payload});
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.masonry".into(),
                    payload: grid.to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.spatial", ContainerModality::NodeGraph) => {
                if let Some(saved) = config
                    .get("_node_graph_restore")
                    .and_then(serde_json::Value::as_str)
                {
                    return Ok(NodeKind::Extension {
                        modality: "doofusdock.node_graph".into(),
                        payload: saved.to_owned(),
                        children: children.clone(),
                    });
                }
                let graph = serde_json::json!({"_spatial_restore": payload});
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.node_graph".into(),
                    payload: graph.to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.node_graph", ContainerModality::Grid { columns, gap }) => {
                if let Some(saved) = config
                    .get("_grid_restore")
                    .and_then(serde_json::Value::as_str)
                {
                    return Ok(NodeKind::Extension {
                        modality: "doofusdock.masonry".into(),
                        payload: saved.to_owned(),
                        children: children.clone(),
                    });
                }
                if !(1..=64).contains(columns) || *gap > 256 {
                    return Err(DockError::new(
                        ErrorCode::ConstraintViolation,
                        "Grid columns must be 1–64 and gap at most 256.",
                        &node.id.0,
                    ));
                }
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.masonry".into(),
                    payload: serde_json::json!({
                        "columns": columns,
                        "gap": gap,
                        "_node_graph_restore": payload
                    })
                    .to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.node_graph", ContainerModality::Spatial) => {
                if let Some(saved) = config
                    .get("_spatial_restore")
                    .and_then(serde_json::Value::as_str)
                {
                    return Ok(NodeKind::Extension {
                        modality: "doofusdock.spatial".into(),
                        payload: saved.to_owned(),
                        children: children.clone(),
                    });
                }
                let count = children.len();
                let columns = (count as f64).sqrt().ceil().max(1.0) as usize;
                let rows = count.div_ceil(columns);
                let mut placements = Vec::with_capacity(count);
                for index in 0..count {
                    let col = index % columns;
                    let row = index / columns;
                    placements.push(serde_json::json!({
                        "x": col as f64 / columns as f64,
                        "y": row as f64 / rows as f64,
                        "width": 1.0 / columns as f64,
                        "height": 1.0 / rows as f64
                    }));
                }
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.spatial".into(),
                    payload: serde_json::json!({
                        "placements": placements,
                        "_node_graph_restore": payload
                    })
                    .to_string(),
                    children: children.clone(),
                });
            }
            ("doofusdock.node_graph", ContainerModality::NodeGraph) => {
                return Ok(NodeKind::Extension {
                    modality: "doofusdock.node_graph".into(),
                    payload: payload.clone(),
                    children: children.clone(),
                });
            }
            (LAYERED_MODALITY, ContainerModality::Layered) => {
                let config = LayeredConfiguration::parse(payload, children)?;
                return Ok(NodeKind::Extension {
                    modality: LAYERED_MODALITY.into(),
                    payload: config.to_payload()?,
                    children: children.clone(),
                });
            }
            _ => {}
        }
    }
    let children = match &node.kind {
        NodeKind::Tabs { items, .. } => items,
        NodeKind::Split { children, .. } => children,
        NodeKind::Extension {
            modality, children, ..
        } if is_stack_modality(modality)
            || matches!(
                modality.0.as_str(),
                LAYERED_MODALITY
                    | "doofusdock.node_graph"
                    | "doofusdock.masonry"
                    | "doofusdock.spatial"
                    | SCROLLABLE_COLUMNS_MODALITY_ID
                    | "doofusdock.master_stack"
                    | "doofusdock.bsp"
                    | "doofusdock.spiral"
                    | "doofusdock.dwindle"
                    | "doofusdock.monocle"
                    | "doofusdock.equal_columns"
                    | "doofusdock.equal_rows"
                    | "doofusdock.adaptive_grid"
            ) =>
        {
            children
        }
        _ => {
            return Err(DockError::new(
                ErrorCode::UnsupportedOperation,
                "Conversion supports tab, split, accordion and shade-stack containers.",
                &node.id.0,
            ));
        }
    };
    let layered_state = match &node.kind {
        NodeKind::Extension {
            modality, payload, ..
        } if modality.0 == LAYERED_MODALITY => {
            Some(LayeredConfiguration::parse(payload, children)?)
        }
        _ => None,
    };
    let ordered_children: Vec<_> = if let Some(config) = &layered_state {
        config
            .back_to_front
            .iter()
            .filter_map(|id| children.iter().find(|child| &child.id == id).cloned())
            .collect()
    } else {
        children.clone()
    };
    if children.is_empty() {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Cannot convert an empty container.",
            &node.id.0,
        ));
    }
    Ok(match target {
        ContainerModality::Tabbed { active_index } => {
            if let Some(config) = &layered_state {
                let selected_index = ordered_children
                    .iter()
                    .position(|child| child.id == config.active)
                    .unwrap_or(*active_index);
                NodeKind::Tabs {
                    active_index: selected_index.min(ordered_children.len().saturating_sub(1)),
                    minimized: false,
                    items: ordered_children.clone(),
                }
            } else {
                NodeKind::Tabs {
                    active_index: *active_index,
                    minimized: false,
                    items: ordered_children.clone(),
                }
            }
        }
        ContainerModality::Split {
            orientation,
            weights,
        } => NodeKind::Split {
            orientation: *orientation,
            weights: if weights.is_empty() {
                vec![1.0 / children.len() as f64; children.len()]
            } else {
                weights.clone()
            },
            children: ordered_children.clone(),
        },
        ContainerModality::Grid { columns, gap } => {
            if !(1..=64).contains(columns) || *gap > 256 {
                return Err(DockError::new(
                    ErrorCode::ConstraintViolation,
                    "Grid columns must be 1–64 and gap at most 256.",
                    &node.id.0,
                ));
            }
            NodeKind::Extension {
                modality: "doofusdock.masonry".into(),
                payload: serde_json::json!({"columns":columns,"gap":gap}).to_string(),
                children: ordered_children.clone(),
            }
        }
        ContainerModality::Spatial => NodeKind::Extension {
            modality: "doofusdock.spatial".into(),
            payload: "{}".into(),
            children: ordered_children.clone(),
        },
        ContainerModality::NodeGraph => NodeKind::Extension {
            modality: "doofusdock.node_graph".into(),
            payload: "{}".into(),
            children: ordered_children.clone(),
        },
        ContainerModality::Layered => {
            let mut config = LayeredConfiguration::from_children(&ordered_children)?;
            let active = match &node.kind {
                NodeKind::Tabs {
                    active_index,
                    items,
                    ..
                } => items.get(*active_index),
                NodeKind::Extension {
                    modality,
                    payload,
                    children,
                } if is_stack_modality(modality) => StackConfiguration::parse(payload, children)?
                    .selected
                    .and_then(|id| children.iter().find(|child| child.id == id)),
                _ => None,
            };
            if let Some(active) = active {
                config.raise(&active.id)?;
            }
            NodeKind::Extension {
                modality: LAYERED_MODALITY.into(),
                payload: config.to_payload()?,
                children: ordered_children.clone(),
            }
        }
        ContainerModality::LayoutStrategy { strategy, gap } => {
            let config = LayoutStrategyConfiguration::new(*strategy, *gap)?;
            NodeKind::Extension {
                modality: strategy.modality().into(),
                payload: config.to_payload()?,
                children: ordered_children.clone(),
            }
        }
        ContainerModality::ScrollableColumns {
            gap,
            width,
            minimum_width,
            maximum_width,
        } => {
            let columns = ordered_children
                .iter()
                .map(|child| ScrollableColumn {
                    child_id: child.id.clone(),
                    width: *width,
                    minimum_width: *minimum_width,
                    maximum_width: *maximum_width,
                })
                .collect();
            let config = ScrollableColumnsConfiguration::new(columns, *gap)?;
            NodeKind::Extension {
                modality: SCROLLABLE_COLUMNS_MODALITY_ID.into(),
                payload: config.to_payload()?,
                children: ordered_children.clone(),
            }
        }
        ContainerModality::Accordion { multiple: _ } | ContainerModality::ShadeStack => {
            let multiple = match target {
                ContainerModality::Accordion { multiple } => *multiple,
                _ => true,
            };
            let selected = if let Some(config) = &layered_state {
                children.iter().find(|child| child.id == config.active)
            } else {
                match &node.kind {
                    NodeKind::Tabs { active_index, .. } => children.get(*active_index),
                    NodeKind::Extension {
                        modality,
                        payload,
                        children,
                    } if is_stack_modality(modality) => {
                        StackConfiguration::parse(payload, children)?
                            .selected
                            .and_then(|id| children.iter().find(|child| child.id == id))
                    }
                    _ => children.first(),
                }
            }
            .map(|n| n.id.clone());
            let config = StackConfiguration {
                version: 1,
                multiple,
                expanded: selected.iter().cloned().collect(),
                selected,
            };
            NodeKind::Extension {
                modality: match target {
                    ContainerModality::ShadeStack => SHADE_STACK_MODALITY,
                    _ => ACCORDION_MODALITY,
                }
                .into(),
                payload: config.to_payload()?,
                children: ordered_children.clone(),
            }
        }
    })
}

/// Header and optional body geometry. Collapsed content stays canonical but has
/// no input/focus surface. Hosts may provide scrolling when this returns a conflict.
#[derive(Clone, Debug, PartialEq)]
pub struct StackSection {
    pub child: NodeId,
    pub header: Rect,
    pub body: Option<Rect>,
}
pub fn arrange_stack(
    node: &Node,
    bounds: Rect,
    header_height: i32,
    minimum_bodies: &[Size],
) -> DockResult<Vec<StackSection>> {
    let NodeKind::Extension {
        modality,
        payload,
        children,
    } = &node.kind
    else {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Expected a stack container.",
            &node.id.0,
        ));
    };
    if !is_stack_modality(modality) {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Expected a stack modality.",
            &modality.0,
        ));
    }
    let config = StackConfiguration::parse(payload, children)?;
    if header_height <= 0
        || bounds.width < 0
        || bounds.height < 0
        || minimum_bodies.len() != children.len()
        || minimum_bodies.iter().any(|s| s.width < 0 || s.height < 0)
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Invalid stack geometry or child metrics.",
            &node.id.0,
        ));
    }
    let headers = i64::from(header_height) * children.len() as i64;
    let minimum_height: i64 = children
        .iter()
        .zip(minimum_bodies)
        .filter(|(c, _)| config.expanded.contains(&c.id))
        .map(|(_, s)| i64::from(s.height))
        .sum();
    if headers + minimum_height > i64::from(bounds.height)
        || children
            .iter()
            .zip(minimum_bodies)
            .any(|(c, s)| config.expanded.contains(&c.id) && s.width > bounds.width)
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            format!(
                "Stack needs at least {} px height and expanded child widths within {} px; available height is {} px.",
                headers + minimum_height,
                bounds.width,
                bounds.height
            ),
            &node.id.0,
        ));
    }
    if bounds.x.checked_add(bounds.width).is_none() || bounds.y.checked_add(bounds.height).is_none()
    {
        return Err(DockError::new(
            ErrorCode::ConstraintViolation,
            "Stack coordinates overflow.",
            &node.id.0,
        ));
    }
    let spare = bounds.height - (headers + minimum_height) as i32;
    let count = config.expanded.len() as i32;
    let mut expanded_index = 0;
    let mut y = bounds.y;
    let mut result = Vec::with_capacity(children.len());
    for (child, min) in children.iter().zip(minimum_bodies) {
        let header = Rect {
            x: bounds.x,
            y,
            width: bounds.width,
            height: header_height,
        };
        y += header_height;
        let body = if config.expanded.contains(&child.id) {
            let height = min.height + spare / count + i32::from(expanded_index < spare % count);
            expanded_index += 1;
            let body = Rect {
                x: bounds.x,
                y,
                width: bounds.width,
                height,
            };
            y += height;
            Some(body)
        } else {
            None
        };
        result.push(StackSection {
            child: child.id.clone(),
            header,
            body,
        });
    }
    Ok(result)
}

#[derive(Clone, Debug)]
pub enum ModalityChange {
    Container {
        container: NodeId,
        target: ContainerModality,
    },
    NativeFloating {
        panel: PanelId,
        window: WindowId,
        geometry: Rect,
        monitor: String,
        scale: f64,
    },
    PictureInPicture {
        panel: PanelId,
        window: WindowId,
        geometry: Rect,
        monitor: String,
        scale: f64,
    },
    DockBack {
        window: WindowId,
        target_panel: Option<PanelId>,
    },
    AutoHide {
        panel: PanelId,
        edge: DockEdge,
    },
    Docked {
        panel: PanelId,
        target_tabs: NodeId,
        index: usize,
    },
    Focus {
        panel: PanelId,
    },
    RestoreFocus,
}
#[derive(Clone, Debug)]
pub struct ModalitySwitchPlan {
    source: LayoutDocument,
    preview: LayoutDocument,
    operations: Vec<Operation>,
}
impl ModalitySwitchPlan {
    pub fn preview(&self) -> &LayoutDocument {
        &self.preview
    }
    pub fn base_revision(&self) -> u64 {
        self.source.revision
    }

    pub fn commit(self, service: &mut DockingService) -> DockResult<TransactionResult> {
        if service.snapshot().document != self.source {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Modality plan source changed; prepare a new preview.",
                &self.source.id.0,
            ));
        }
        service.transact(
            self.source.revision,
            "switch-modality".into(),
            self.operations,
        )
    }
}
/// Negotiation is explicit. Hosts advertise only implemented rendering/input
/// contracts. Unsupported destinations fail before any canonical mutation.
pub fn plan_modality_change(
    document: &LayoutDocument,
    change: ModalityChange,
    host: &HostCapabilities,
) -> DockResult<ModalitySwitchPlan> {
    validate_layout(document)?;
    host.validate()?;
    let (required, operations) = match change {
        ModalityChange::Container { container, target } => {
            match &target {
                ContainerModality::Grid { columns, gap }
                    if !(1..=64).contains(columns) || *gap > 256 =>
                {
                    return Err(DockError::new(
                        ErrorCode::ConstraintViolation,
                        "Grid columns must be 1–64 and gap at most 256.",
                        &container.0,
                    ));
                }
                ContainerModality::LayoutStrategy { strategy, gap } => {
                    LayoutStrategyConfiguration::new(*strategy, *gap)?;
                }
                ContainerModality::ScrollableColumns {
                    gap,
                    width,
                    minimum_width,
                    maximum_width,
                } => {
                    ScrollableColumnsConfiguration::new(
                        vec![ScrollableColumn {
                            child_id: NodeId::from("target-validation"),
                            width: *width,
                            minimum_width: *minimum_width,
                            maximum_width: *maximum_width,
                        }],
                        *gap,
                    )?;
                }
                _ => {}
            }
            let required = match target {
                ContainerModality::Tabbed { .. } => "workspace:tabs",
                ContainerModality::Split { .. } => "workspace:docking",
                ContainerModality::Accordion { .. } => "workspace:accordion",
                ContainerModality::ShadeStack => "workspace:shade_stack",
                ContainerModality::Grid { .. } => "workspace:grid",
                ContainerModality::Spatial => "workspace:spatial",
                ContainerModality::NodeGraph => "workspace:node_graph",
                ContainerModality::Layered => "workspace:layered",
                ContainerModality::LayoutStrategy { .. } => "workspace:layout_strategy",
                ContainerModality::ScrollableColumns { .. } => "workspace:scrollable_columns",
            };
            (
                required,
                vec![Operation::ConvertContainer { container, target }],
            )
        }
        ModalityChange::NativeFloating {
            panel,
            window,
            geometry,
            monitor,
            scale,
        } => (
            "native_viewport",
            vec![Operation::FloatPanel {
                panel,
                window,
                geometry,
                monitor,
                scale,
            }],
        ),
        ModalityChange::PictureInPicture {
            panel,
            window,
            geometry,
            monitor,
            scale,
        } => (
            "picture_in_picture",
            vec![
                Operation::FloatPanel {
                    panel,
                    window: window.clone(),
                    geometry,
                    monitor,
                    scale,
                },
                Operation::SetPictureInPicture {
                    window,
                    enabled: true,
                },
            ],
        ),
        ModalityChange::DockBack {
            window,
            target_panel,
        } => (
            "workspace:docking",
            vec![Operation::DockFloatingWindow {
                window,
                target_panel,
            }],
        ),
        ModalityChange::AutoHide { panel, edge } => (
            "workspace:auto_hide",
            vec![Operation::AutoHide { panel, edge }],
        ),
        ModalityChange::Docked {
            panel,
            target_tabs,
            index,
        } => (
            "workspace:tabs",
            vec![Operation::PinPanel {
                panel,
                target_tabs,
                index,
            }],
        ),
        ModalityChange::Focus { panel } => ("workspace:focus", vec![Operation::Maximize { panel }]),
        ModalityChange::RestoreFocus => ("workspace:focus", vec![Operation::RestoreMaximized]),
    };
    if !host.supported.contains(required) {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Host has not declared the required modality behavior.",
            required,
        ));
    }
    let mut preview_source = document.clone();
    let mut preview_history = UndoStack::new(1);
    let preview = DockingService::transact_document(
        &mut preview_source,
        &mut preview_history,
        document.revision,
        "preview-modality".into(),
        operations.clone(),
    )?
    .document;
    Ok(ModalitySwitchPlan {
        source: document.clone(),
        preview,
        operations,
    })
}

/// Transient presentation state that a host may show without changing the
/// canonical layout or adding persistence fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransientPresentationKind {
    /// Toggle the host viewport's exclusive full-display presentation.
    /// This is ephemeral host state and does not change canonical layout.
    Fullscreen {
        enabled: bool,
    },
    Overlay {
        modal: bool,
    },
    Drawer {
        edge: DockEdge,
        modal: bool,
    },
    Peek {
        edge: DockEdge,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransientPresentationRequest {
    pub id: String,
    pub title: String,
    pub kind: TransientPresentationKind,
}

/// Capability-negotiated, revision-bound host presentation request.
/// Activation returns an intent; it does not mutate or persist layout state.
#[derive(Clone, Debug)]
pub struct TransientPresentationPlan {
    source: LayoutDocument,
    request: TransientPresentationRequest,
    required_capability: &'static str,
}

impl TransientPresentationPlan {
    pub fn preview(&self) -> &TransientPresentationRequest {
        &self.request
    }

    pub fn base_revision(&self) -> u64 {
        self.source.revision
    }

    pub fn required_capability(&self) -> &'static str {
        self.required_capability
    }

    /// Produces a host intent only while the exact planned source snapshot is
    /// still current. The host owns activation and eventual dismissal.
    pub fn activate(self, current: &LayoutDocument) -> DockResult<TransientPresentationRequest> {
        if current != &self.source {
            return Err(DockError::new(
                ErrorCode::Conflict,
                "Transient presentation source changed; prepare a new request.",
                &self.source.id.0,
            ));
        }
        Ok(self.request)
    }
}

pub fn plan_transient_presentation(
    document: &LayoutDocument,
    request: TransientPresentationRequest,
    host: &HostCapabilities,
) -> DockResult<TransientPresentationPlan> {
    validate_layout(document)?;
    host.validate()?;
    if request.id.is_empty()
        || request.id.len() > 256
        || !request
            .id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.:-".contains(character))
        || request.title.trim().is_empty()
        || request.title.len() > 4096
        || request.title.chars().any(char::is_control)
    {
        return Err(DockError::new(
            ErrorCode::InvalidId,
            "Transient presentation ID or title is invalid.",
            &request.id,
        ));
    }
    let required_capability = match request.kind {
        TransientPresentationKind::Fullscreen { .. } => "viewport:fullscreen",
        TransientPresentationKind::Overlay { modal: true }
        | TransientPresentationKind::Drawer { modal: true, .. } => "overlay.modal",
        TransientPresentationKind::Overlay { modal: false } => "overlay.modeless",
        TransientPresentationKind::Drawer { modal: false, .. } => "overlay.drawer",
        TransientPresentationKind::Peek { .. } => "overlay.peek",
    };
    if !host.supported.contains(required_capability) {
        return Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Host has not declared the requested transient presentation behavior.",
            required_capability,
        ));
    }
    Ok(TransientPresentationPlan {
        source: document.clone(),
        request,
        required_capability,
    })
}
