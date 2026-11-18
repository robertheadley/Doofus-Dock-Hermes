//! Toolkit-neutral arrangement contracts for extension-backed containers.
use crate::{DockError, DockResult, ErrorCode, Node, NodeKind, NodePtr, Point, Rect, Size};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    ops::Range,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

pub const SCROLLABLE_COLUMNS_MODALITY_ID: &str = "doofusdock.scrollable_columns";
const MAX_SCROLLABLE_COLUMN_WIDTH: i32 = 1_000_000;
const STRATEGY_MODALITIES: [&str; 8] = [
    "doofusdock.master_stack",
    "doofusdock.bsp",
    "doofusdock.spiral",
    "doofusdock.dwindle",
    "doofusdock.monocle",
    "doofusdock.equal_columns",
    "doofusdock.equal_rows",
    "doofusdock.adaptive_grid",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutStrategy {
    MasterStack,
    BinarySpacePartition,
    Spiral,
    Dwindle,
    Monocle,
    EqualColumns,
    EqualRows,
    AdaptiveGrid,
}

impl LayoutStrategy {
    pub const ALL: [Self; 8] = [
        Self::MasterStack,
        Self::BinarySpacePartition,
        Self::Spiral,
        Self::Dwindle,
        Self::Monocle,
        Self::EqualColumns,
        Self::EqualRows,
        Self::AdaptiveGrid,
    ];

    pub fn modality(self) -> &'static str {
        match self {
            Self::MasterStack => STRATEGY_MODALITIES[0],
            Self::BinarySpacePartition => STRATEGY_MODALITIES[1],
            Self::Spiral => STRATEGY_MODALITIES[2],
            Self::Dwindle => STRATEGY_MODALITIES[3],
            Self::Monocle => STRATEGY_MODALITIES[4],
            Self::EqualColumns => STRATEGY_MODALITIES[5],
            Self::EqualRows => STRATEGY_MODALITIES[6],
            Self::AdaptiveGrid => STRATEGY_MODALITIES[7],
        }
    }
}

/// Versioned deterministic arrangement policy. Child order is supplied by
/// the canonical container; this configuration never owns panel identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutStrategyConfiguration {
    pub schema_version: u32,
    pub strategy: LayoutStrategy,
    pub gap: i32,
}

impl LayoutStrategyConfiguration {
    pub fn new(strategy: LayoutStrategy, gap: i32) -> DockResult<Self> {
        let config = Self {
            schema_version: 1,
            strategy,
            gap,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(self) -> DockResult<()> {
        if self.schema_version != 1 || !(0..=256).contains(&self.gap) {
            return Err(layout_error(
                "Layout strategy schema version or gap is invalid.",
                "strategy-config",
            ));
        }
        Ok(())
    }

    pub fn to_payload(self) -> DockResult<String> {
        self.validate()?;
        serde_json::to_string(&self).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                "Layout strategy config serialization failed.",
                error.to_string(),
            )
        })
    }

    pub fn from_payload(payload: &str) -> DockResult<Self> {
        if payload.len() > 4096 {
            return Err(layout_error(
                "Layout strategy payload exceeds its limit.",
                payload.len().to_string(),
            ));
        }
        let config: Self = serde_json::from_str(payload).map_err(|error| {
            DockError::new(
                ErrorCode::SchemaError,
                "Layout strategy payload is invalid.",
                error.to_string(),
            )
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn into_node(
        self,
        id: impl Into<crate::NodeId>,
        children: Vec<NodePtr>,
    ) -> DockResult<NodePtr> {
        let payload = self.to_payload()?;
        Ok(Arc::new(Node {
            id: id.into(),
            kind: NodeKind::Extension {
                modality: self.strategy.modality().into(),
                payload,
                children,
            },
        }))
    }
}

/// Arrange a canonical ordered child list through a versioned strategy.
pub fn arrange_layout_strategy(
    config: LayoutStrategyConfiguration,
    bounds: Rect,
    children: &[ExtensionChildMetrics],
) -> DockResult<Vec<ExtensionPlacement>> {
    config.validate()?;
    if bounds.width <= 0 || bounds.height <= 0 || children.len() > 4096 {
        return Err(layout_error(
            "Strategy viewport or child count is invalid.",
            "layout-strategy",
        ));
    }
    if children.is_empty() {
        return Ok(Vec::new());
    }
    let n = children.len();
    let mut result = vec![Rect::default(); n];
    match config.strategy {
        LayoutStrategy::Monocle => result.fill(bounds),
        LayoutStrategy::EqualColumns => {
            let widths = divide_axis(bounds.width, n, config.gap)?;
            let mut x = bounds.x;
            for (index, width) in widths.into_iter().enumerate() {
                result[index] = Rect {
                    x,
                    y: bounds.y,
                    width,
                    height: bounds.height,
                };
                x = x.saturating_add(width).saturating_add(config.gap);
            }
        }
        LayoutStrategy::EqualRows => {
            let heights = divide_axis(bounds.height, n, config.gap)?;
            let mut y = bounds.y;
            for (index, height) in heights.into_iter().enumerate() {
                result[index] = Rect {
                    x: bounds.x,
                    y,
                    width: bounds.width,
                    height,
                };
                y = y.saturating_add(height).saturating_add(config.gap);
            }
        }
        LayoutStrategy::AdaptiveGrid => {
            let aspect = bounds.width as f64 / bounds.height as f64;
            let columns = ((n as f64 * aspect).sqrt().ceil() as usize).clamp(1, n);
            let rows = n.div_ceil(columns);
            let widths = divide_axis(bounds.width, columns, config.gap)?;
            let heights = divide_axis(bounds.height, rows, config.gap)?;
            for (index, rect) in result.iter_mut().enumerate() {
                let column = index % columns;
                let row = index / columns;
                *rect = Rect {
                    x: axis_offset(bounds.x, &widths, config.gap, column),
                    y: axis_offset(bounds.y, &heights, config.gap, row),
                    width: widths[column],
                    height: heights[row],
                };
            }
        }
        LayoutStrategy::MasterStack => arrange_master_stack(bounds, n, config.gap, &mut result)?,
        LayoutStrategy::BinarySpacePartition => {
            arrange_recursive_strategy(bounds, config.gap, 0, n, 0, &mut result)?;
        }
        LayoutStrategy::Spiral | LayoutStrategy::Dwindle => {
            arrange_carved_strategy(config.strategy, bounds, config.gap, 0, &mut result)?;
        }
    }
    for (index, (rect, metrics)) in result.iter().zip(children).enumerate() {
        if rect.width < metrics.minimum.width || rect.height < metrics.minimum.height {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Layout strategy cannot satisfy a child's minimum geometry.",
                format!("child={index}:rect={rect:?}:minimum={:?}", metrics.minimum),
            ));
        }
    }
    Ok(result
        .into_iter()
        .enumerate()
        .map(|(child_index, rect)| ExtensionPlacement {
            child_index,
            bounds: rect,
        })
        .collect())
}

fn divide_axis(extent: i32, count: usize, gap: i32) -> DockResult<Vec<i32>> {
    let available = i64::from(extent) - i64::from(gap) * count.saturating_sub(1) as i64;
    if count == 0 || available < count as i64 {
        return Err(layout_error(
            "Strategy gap leaves no positive size for every item.",
            format!("extent={extent}:count={count}:gap={gap}"),
        ));
    }
    let base = available / count as i64;
    let remainder = available % count as i64;
    Ok((0..count)
        .map(|index| (base + i64::from((index as i64) < remainder)) as i32)
        .collect())
}

fn axis_offset(origin: i32, extents: &[i32], gap: i32, index: usize) -> i32 {
    extents.iter().take(index).fold(origin, |offset, extent| {
        offset.saturating_add(*extent).saturating_add(gap)
    })
}

fn arrange_master_stack(
    bounds: Rect,
    count: usize,
    gap: i32,
    output: &mut [Rect],
) -> DockResult<()> {
    if count == 1 {
        output[0] = bounds;
        return Ok(());
    }
    let master_width = ((i64::from(bounds.width) * 3) / 5) as i32;
    let stack_width = bounds
        .width
        .saturating_sub(master_width)
        .saturating_sub(gap);
    let stack_heights = divide_axis(bounds.height, count - 1, gap)?;
    output[0] = Rect {
        x: bounds.x,
        y: bounds.y,
        width: master_width,
        height: bounds.height,
    };
    let mut y = bounds.y;
    for (index, height) in stack_heights.into_iter().enumerate() {
        output[index + 1] = Rect {
            x: bounds.x.saturating_add(master_width).saturating_add(gap),
            y,
            width: stack_width,
            height,
        };
        y = y.saturating_add(height).saturating_add(gap);
    }
    Ok(())
}

fn arrange_recursive_strategy(
    bounds: Rect,
    gap: i32,
    start: usize,
    end: usize,
    depth: usize,
    output: &mut [Rect],
) -> DockResult<()> {
    if end - start == 1 {
        output[start] = bounds;
        return Ok(());
    }
    let first_count = (end - start).div_ceil(2);
    let axis_horizontal = depth.is_multiple_of(2);
    let (extent, leading_gap) = if axis_horizontal {
        (bounds.width, gap)
    } else {
        (bounds.height, gap)
    };
    let first_extent = ((i64::from(extent) - i64::from(leading_gap)) / 2) as i32;
    let second_extent = extent
        .saturating_sub(leading_gap)
        .saturating_sub(first_extent);
    if first_extent <= 0 || second_extent <= 0 {
        return Err(layout_error(
            "Recursive strategy cannot split the remaining viewport.",
            format!("depth={depth}:bounds={bounds:?}"),
        ));
    }
    let (first, second) = if axis_horizontal {
        (
            Rect {
                width: first_extent,
                ..bounds
            },
            Rect {
                x: bounds.x.saturating_add(first_extent).saturating_add(gap),
                width: second_extent,
                ..bounds
            },
        )
    } else {
        (
            Rect {
                height: first_extent,
                ..bounds
            },
            Rect {
                y: bounds.y.saturating_add(first_extent).saturating_add(gap),
                height: second_extent,
                ..bounds
            },
        )
    };
    arrange_recursive_strategy(first, gap, start, start + first_count, depth + 1, output)?;
    arrange_recursive_strategy(second, gap, start + first_count, end, depth + 1, output)
}

fn arrange_carved_strategy(
    strategy: LayoutStrategy,
    mut remaining: Rect,
    gap: i32,
    index: usize,
    output: &mut [Rect],
) -> DockResult<()> {
    if index + 1 == output.len() {
        output[index] = remaining;
        return Ok(());
    }
    let direction = index % 4;
    let extent = if direction.is_multiple_of(2) {
        remaining.width
    } else {
        remaining.height
    };
    if extent <= gap + 1 {
        return Err(layout_error(
            "Spiral strategy cannot fit another panel in the remaining viewport.",
            format!("index={index}:bounds={remaining:?}"),
        ));
    }
    let ratio = if strategy == LayoutStrategy::Spiral {
        0.5
    } else {
        0.618
    };
    let carved =
        ((f64::from(extent) * ratio).round() as i32).clamp(1, extent.saturating_sub(gap + 1));
    let tile = match direction {
        0 => {
            let tile = Rect {
                width: carved,
                ..remaining
            };
            remaining.x = remaining.x.saturating_add(carved).saturating_add(gap);
            remaining.width = remaining.width.saturating_sub(carved).saturating_sub(gap);
            tile
        }
        1 => {
            let tile = Rect {
                height: carved,
                ..remaining
            };
            remaining.y = remaining.y.saturating_add(carved).saturating_add(gap);
            remaining.height = remaining.height.saturating_sub(carved).saturating_sub(gap);
            tile
        }
        2 => {
            let tile_x = remaining
                .x
                .saturating_add(remaining.width)
                .saturating_sub(carved);
            let tile = Rect {
                x: tile_x,
                width: carved,
                ..remaining
            };
            remaining.width = remaining.width.saturating_sub(carved).saturating_sub(gap);
            tile
        }
        _ => {
            let tile_y = remaining
                .y
                .saturating_add(remaining.height)
                .saturating_sub(carved);
            let tile = Rect {
                y: tile_y,
                height: carved,
                ..remaining
            };
            remaining.height = remaining.height.saturating_sub(carved).saturating_sub(gap);
            tile
        }
    };
    if remaining.width <= 0 || remaining.height <= 0 {
        return Err(layout_error(
            "Spiral strategy exhausted the viewport before placing all panels.",
            format!("index={index}:bounds={remaining:?}"),
        ));
    }
    output[index] = tile;
    arrange_carved_strategy(strategy, remaining, gap, index + 1, output)
}

/// Persistable stable ordering and sizing for horizontally scrollable columns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollableColumnsConfiguration {
    pub schema_version: u32,
    pub gap: i32,
    pub columns: Vec<ScrollableColumn>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollableColumn {
    pub child_id: crate::NodeId,
    pub width: i32,
    pub minimum_width: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_width: Option<i32>,
}

/// Transient viewport result. Keep widget state keyed by child identity even
/// when a column is outside `visible_range`; only its drawing is virtualized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollableColumnsFrame {
    pub content_width: i32,
    pub scroll_offset: i32,
    pub visible_range: Range<usize>,
    pub overscan_range: Range<usize>,
    pub placements: Vec<ExtensionPlacement>,
}

impl ScrollableColumnsConfiguration {
    pub fn new(columns: Vec<ScrollableColumn>, gap: i32) -> DockResult<Self> {
        let config = Self {
            schema_version: 1,
            gap,
            columns,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> DockResult<()> {
        if self.schema_version != 1 || self.gap < 0 || self.gap > 256 {
            return Err(DockError::new(
                ErrorCode::SchemaError,
                "Scrollable-column schema version or gap is invalid.",
                format!("version={}:gap={}", self.schema_version, self.gap),
            ));
        }
        if self.columns.len() > 4096 {
            return Err(DockError::new(
                ErrorCode::ResourceLimit,
                "Scrollable-column count exceeds its limit.",
                self.columns.len().to_string(),
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for column in &self.columns {
            if column.child_id.is_empty()
                || !ids.insert(column.child_id.clone())
                || column.minimum_width <= 0
                || column.minimum_width > MAX_SCROLLABLE_COLUMN_WIDTH
                || column.width < column.minimum_width
                || column.width > MAX_SCROLLABLE_COLUMN_WIDTH
                || column.maximum_width.is_some_and(|max| {
                    max < column.width || max <= 0 || max > MAX_SCROLLABLE_COLUMN_WIDTH
                })
            {
                return Err(DockError::new(
                    ErrorCode::SchemaError,
                    "Scrollable-column identity or width constraints are invalid.",
                    column.child_id.0.clone(),
                ));
            }
        }
        Ok(())
    }

    /// Return a validated configuration with one stable-ID column resized.
    /// A missing maximum uses the global one-million-pixel safety bound.
    pub fn resized_column(&self, child_id: &crate::NodeId, delta: i32) -> DockResult<Self> {
        self.validate()?;
        let mut updated = self.clone();
        let column = updated
            .columns
            .iter_mut()
            .find(|column| &column.child_id == child_id)
            .ok_or_else(|| {
                DockError::new(
                    ErrorCode::NotFound,
                    "Scrollable column to resize was not found.",
                    &child_id.0,
                )
            })?;
        let maximum = column.maximum_width.unwrap_or(MAX_SCROLLABLE_COLUMN_WIDTH);
        column.width = (i64::from(column.width) + i64::from(delta))
            .clamp(i64::from(column.minimum_width), i64::from(maximum))
            as i32;
        updated.validate()?;
        Ok(updated)
    }

    /// Serialize a bounded configuration for a canonical
    /// `UpdateExtensionPayload` operation.
    pub fn to_payload(&self) -> DockResult<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|error| {
            DockError::new(
                ErrorCode::SerializationError,
                format!("Scrollable-column configuration could not be serialized: {error}"),
                "scrollable-columns",
            )
        })
    }

    /// Read stable column order and widths from the canonical extension node.
    pub fn from_node(node: &Node) -> DockResult<Self> {
        let (payload, children) = match &node.kind {
            NodeKind::Extension {
                modality,
                payload,
                children,
            } if modality.0 == SCROLLABLE_COLUMNS_MODALITY_ID => (payload, children),
            _ => {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Node is not a scrollable-columns container.",
                    &node.id.0,
                ));
            }
        };
        let config: Self = serde_json::from_str(payload).map_err(|error| {
            DockError::new(
                ErrorCode::SchemaError,
                format!("Scrollable-column payload is invalid: {error}"),
                &node.id.0,
            )
        })?;
        config.validate_child_identity(children)?;
        Ok(config)
    }

    /// Replace only the serialized configuration of a canonical extension node.
    /// Child subtrees and the stable container ID are retained.
    pub fn update_node(&self, node: &Node) -> DockResult<NodePtr> {
        let children = match &node.kind {
            NodeKind::Extension {
                modality, children, ..
            } if modality.0 == SCROLLABLE_COLUMNS_MODALITY_ID => children,
            _ => {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Node is not a scrollable-columns container.",
                    &node.id.0,
                ));
            }
        };
        self.validate_child_identity(children)?;
        let payload = self.to_payload()?;
        Ok(Arc::new(Node {
            id: node.id.clone(),
            kind: NodeKind::Extension {
                modality: SCROLLABLE_COLUMNS_MODALITY_ID.into(),
                payload,
                children: children.clone(),
            },
        }))
    }

    /// Compile a configuration edit to the existing canonical, undoable
    /// extension-payload operation.
    pub fn update_operation(&self, node: &Node) -> DockResult<crate::Operation> {
        let children = match &node.kind {
            NodeKind::Extension {
                modality, children, ..
            } if modality.0 == SCROLLABLE_COLUMNS_MODALITY_ID => children,
            _ => {
                return Err(DockError::new(
                    ErrorCode::InvalidLayout,
                    "Node is not a scrollable-columns container.",
                    &node.id.0,
                ));
            }
        };
        self.validate_child_identity(children)?;
        Ok(crate::Operation::UpdateExtensionPayload {
            container: node.id.clone(),
            payload: self.to_payload()?,
        })
    }

    /// Create a canonical, persistable extension node from owned child subtrees.
    pub fn into_node(
        self,
        id: impl Into<crate::NodeId>,
        children: Vec<NodePtr>,
    ) -> DockResult<NodePtr> {
        self.validate_child_identity(&children)?;
        let payload = self.to_payload()?;
        Ok(Arc::new(Node {
            id: id.into(),
            kind: NodeKind::Extension {
                modality: SCROLLABLE_COLUMNS_MODALITY_ID.into(),
                payload,
                children,
            },
        }))
    }

    fn validate_child_identity(&self, children: &[NodePtr]) -> DockResult<()> {
        self.validate()?;
        let actual = children
            .iter()
            .map(|child| child.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let configured = self
            .columns
            .iter()
            .map(|column| column.child_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        if actual.len() != children.len() || actual != configured {
            return Err(DockError::new(
                ErrorCode::InvalidLayout,
                "Scrollable-column configuration must cover its canonical children exactly once.",
                format!(
                    "node_children={}:configured={}",
                    children.len(),
                    configured.len()
                ),
            ));
        }
        Ok(())
    }
}

/// Arrange a stable column strip and return only the visible columns plus a
/// bounded number of adjacent overscan columns. `scroll_offset` is transient
/// host/view state; configuration order and widths are the persistable state.
pub fn arrange_scrollable_columns(
    config: &ScrollableColumnsConfiguration,
    child_ids: &[crate::NodeId],
    children: &[ExtensionChildMetrics],
    viewport: Rect,
    scroll_offset: i32,
    overscan_columns: usize,
) -> DockResult<ScrollableColumnsFrame> {
    config.validate()?;
    if viewport.width <= 0 || viewport.height <= 0 || overscan_columns > 64 {
        return Err(DockError::new(
            ErrorCode::SchemaError,
            "Scrollable-column viewport or overscan is outside supported bounds.",
            format!("viewport={viewport:?}:overscan={overscan_columns}"),
        ));
    }
    if child_ids.len() != children.len() || child_ids.len() != config.columns.len() {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Scrollable-column configuration must cover every child exactly once.",
            format!(
                "configured={}:children={}:metrics={}",
                config.columns.len(),
                child_ids.len(),
                children.len()
            ),
        ));
    }
    let mut child_positions = std::collections::BTreeMap::new();
    for (index, child_id) in child_ids.iter().enumerate() {
        if child_positions.insert(child_id.clone(), index).is_some() {
            return Err(DockError::new(
                ErrorCode::DuplicateId,
                "Scrollable-column child IDs must be unique.",
                child_id.0.clone(),
            ));
        }
        if children[index].minimum.height > viewport.height {
            return Err(DockError::new(
                ErrorCode::ConstraintViolation,
                "Scrollable column cannot satisfy its minimum height.",
                child_id.0.clone(),
            ));
        }
    }
    if config
        .columns
        .iter()
        .any(|column| !child_positions.contains_key(&column.child_id))
    {
        return Err(DockError::new(
            ErrorCode::InvalidLayout,
            "Scrollable-column configuration contains an unknown child ID.",
            "scrollable-columns",
        ));
    }

    let count = config.columns.len();
    let total = config.columns.iter().try_fold(0_i64, |sum, column| {
        sum.checked_add(i64::from(column.width))
    });
    let content_width = total
        .and_then(|width| width.checked_add(i64::from(config.gap) * count.saturating_sub(1) as i64))
        .filter(|width| *width <= i64::from(i32::MAX))
        .ok_or_else(|| {
            DockError::new(
                ErrorCode::ResourceLimit,
                "Scrollable-column content width exceeds the supported range.",
                "scrollable-columns",
            )
        })? as i32;
    let max_offset = content_width.saturating_sub(viewport.width).max(0);
    let scroll_offset = scroll_offset.clamp(0, max_offset);

    let mut starts = Vec::with_capacity(count);
    let mut x = 0_i32;
    for column in &config.columns {
        starts.push(x);
        x = x.saturating_add(column.width).saturating_add(config.gap);
    }
    let mut first_visible = 0;
    while first_visible < count
        && starts[first_visible].saturating_add(config.columns[first_visible].width)
            <= scroll_offset
    {
        first_visible += 1;
    }
    let visible_end = starts
        .iter()
        .enumerate()
        .skip(first_visible)
        .find_map(|(index, start)| {
            (start.saturating_sub(scroll_offset) >= viewport.width).then_some(index)
        })
        .unwrap_or(count);
    let visible_range = first_visible..visible_end.max(first_visible);
    let range_start = first_visible.saturating_sub(overscan_columns);
    let range_end = visible_end.saturating_add(overscan_columns).min(count);
    let overscan_range = range_start..range_end.max(range_start);
    let placements = config.columns[overscan_range.clone()]
        .iter()
        .enumerate()
        .map(|(offset, column)| {
            let child_index = child_positions[&column.child_id];
            ExtensionPlacement {
                child_index,
                bounds: Rect {
                    x: viewport
                        .x
                        .saturating_add(starts[range_start + offset])
                        .saturating_sub(scroll_offset),
                    y: viewport.y,
                    width: column.width,
                    height: viewport.height,
                },
            }
        })
        .collect();
    Ok(ScrollableColumnsFrame {
        content_width,
        scroll_offset,
        visible_range,
        overscan_range,
        placements,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionChildMetrics {
    pub minimum: Size,
    pub preferred: Size,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtensionLayoutRequest<'a> {
    pub modality: &'a str,
    pub payload: &'a str,
    pub bounds: Rect,
    pub child_ids: &'a [crate::NodeId],
    pub children: &'a [ExtensionChildMetrics],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionPlacement {
    pub child_index: usize,
    pub bounds: Rect,
}

/// Tests placements from back to front, so overlapping freeform panels select the topmost child.
pub fn hit_test_extension_placements(
    placements: &[ExtensionPlacement],
    point: Point,
) -> Option<usize> {
    placements.iter().rev().find_map(|placement| {
        let rect = placement.bounds;
        (point.x >= rect.x
            && point.y >= rect.y
            && point.x < rect.x.saturating_add(rect.width)
            && point.y < rect.y.saturating_add(rect.height))
        .then_some(placement.child_index)
    })
}

/// Toolkit-neutral arrangement provider. Dynamic providers register through
/// `ExtensionRegistry` to share identity and lifecycle management.
pub trait ExtensionLayoutProvider: Send + Sync {
    fn modality(&self) -> &str;
    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>>;
}

#[derive(Clone, Default)]
/// Built-in container algorithms, available without application registration.
/// Dynamic providers belong to `ExtensionRegistry`.
pub struct ExtensionLayoutRegistry {
    providers: HashMap<String, Arc<dyn ExtensionLayoutProvider>>,
}

impl ExtensionLayoutRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        for provider in [
            Arc::new(MasonryLayout) as Arc<dyn ExtensionLayoutProvider>,
            Arc::new(SpatialLayout),
            Arc::new(NodeGraphLayout),
            Arc::new(LayeredLayout),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[0],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[1],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[2],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[3],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[4],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[5],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[6],
            }),
            Arc::new(StrategyLayout {
                modality: STRATEGY_MODALITIES[7],
            }),
        ] {
            registry
                .providers
                .insert(provider.modality().to_owned(), provider);
        }
        registry
    }

    /// Legacy compatibility shim. Built-in algorithms need no freeze operation.
    #[deprecated(note = "the built-in layout registry has no mutable lifecycle")]
    pub fn freeze(&mut self) {}

    /// Dynamic providers must be registered through `ExtensionRegistry` so
    /// they share global identity checks, activation, and shutdown lifecycle.
    #[deprecated(note = "register layout providers through ExtensionRegistry")]
    pub fn register(&mut self, provider: Arc<dyn ExtensionLayoutProvider>) -> DockResult<()> {
        Err(DockError::new(
            ErrorCode::UnsupportedOperation,
            "Dynamic layout providers must use ExtensionRegistry lifecycle management.",
            provider.modality(),
        ))
    }

    pub fn arrange(
        &self,
        request: &ExtensionLayoutRequest<'_>,
    ) -> DockResult<Vec<ExtensionPlacement>> {
        let provider = self.providers.get(request.modality).ok_or_else(|| {
            layout_error(
                "No layout provider is registered for this extension.",
                request.modality,
            )
        })?;
        arrange_layout_provider(provider.as_ref(), request)
    }
}

/// Validate and invoke a registered arrangement provider through a shared
/// bounded, panic-contained contract.
pub(crate) fn arrange_layout_provider(
    provider: &dyn ExtensionLayoutProvider,
    request: &ExtensionLayoutRequest<'_>,
) -> DockResult<Vec<ExtensionPlacement>> {
    validate_request(request)?;
    let placements =
        catch_unwind(AssertUnwindSafe(|| provider.arrange(request))).map_err(|_| {
            DockError::new(
                ErrorCode::ExtensionError,
                "Custom layout provider panicked while arranging children.",
                request.modality,
            )
        })??;
    validate_placements(request, &placements)?;
    Ok(placements)
}

pub(crate) fn validate_layout_provider_modality(id: &str) -> DockResult<()> {
    let parts: Vec<_> = id.split('.').collect();
    if parts.len() < 2
        || parts.iter().any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
        })
    {
        return Err(layout_error("Layout provider IDs must be namespaced.", id));
    }
    Ok(())
}

pub(crate) fn is_builtin_layout_modality(id: &str) -> bool {
    matches!(
        id,
        "doofusdock.masonry"
            | "doofusdock.spatial"
            | "doofusdock.node_graph"
            | "doofusdock.layered"
            | "doofusdock.master_stack"
            | "doofusdock.bsp"
            | "doofusdock.spiral"
            | "doofusdock.dwindle"
            | "doofusdock.monocle"
            | "doofusdock.equal_columns"
            | "doofusdock.equal_rows"
            | "doofusdock.adaptive_grid"
    )
}

pub(crate) fn validate_strategy_payload(modality: &str, payload: &str) -> DockResult<()> {
    let config = LayoutStrategyConfiguration::from_payload(payload)?;
    if config.strategy.modality() != modality {
        return Err(layout_error(
            "Layout strategy payload does not match its canonical modality.",
            modality,
        ));
    }
    Ok(())
}

pub(crate) fn is_strategy_modality(modality: &str) -> bool {
    STRATEGY_MODALITIES.contains(&modality)
}

fn validate_request(request: &ExtensionLayoutRequest<'_>) -> DockResult<()> {
    if request.bounds.width < 0 || request.bounds.height < 0 {
        return Err(layout_error(
            "Container bounds cannot be negative.",
            request.modality,
        ));
    }
    if request.children.len() > 4096 {
        return Err(layout_error(
            "Container child count exceeds the layout limit.",
            request.modality,
        ));
    }
    if request.children.iter().any(|m| {
        m.minimum.width < 0
            || m.minimum.height < 0
            || m.preferred.width < 0
            || m.preferred.height < 0
    }) {
        return Err(layout_error(
            "Child sizes cannot be negative.",
            request.modality,
        ));
    }
    Ok(())
}

fn validate_placements(
    request: &ExtensionLayoutRequest<'_>,
    placements: &[ExtensionPlacement],
) -> DockResult<()> {
    if placements.len() != request.children.len() {
        return Err(layout_error(
            "Layout provider must return exactly one placement per child.",
            request.modality,
        ));
    }
    let mut seen = vec![false; placements.len()];
    for placement in placements {
        let r = placement.bounds;
        let Some(slot) = seen.get_mut(placement.child_index) else {
            return Err(layout_error(
                "Layout provider returned an invalid child index.",
                request.modality,
            ));
        };
        if *slot {
            return Err(layout_error(
                "Layout provider returned a child more than once.",
                request.modality,
            ));
        }
        *slot = true;
        if r.width < 0
            || r.height < 0
            || r.width < request.children[placement.child_index].minimum.width
            || r.height < request.children[placement.child_index].minimum.height
            || r.x < request.bounds.x
            || r.y < request.bounds.y
            || r.x.saturating_add(r.width) > request.bounds.x.saturating_add(request.bounds.width)
            || r.y.saturating_add(r.height) > request.bounds.y.saturating_add(request.bounds.height)
        {
            return Err(layout_error(
                "Layout provider returned bounds outside its container.",
                request.modality,
            ));
        }
    }
    Ok(())
}

struct MasonryLayout;
impl ExtensionLayoutProvider for MasonryLayout {
    fn modality(&self) -> &str {
        "doofusdock.masonry"
    }
    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>> {
        let config = parse_config(request.payload, self.modality())?;
        let count = request.children.len();
        if count == 0 {
            return Ok(Vec::new());
        }
        let columns = config
            .get("columns")
            .and_then(Value::as_u64)
            .unwrap_or(3)
            .max(1);
        let columns = usize::try_from(columns).unwrap_or(usize::MAX);
        let columns = columns.min(count).min(request.bounds.width.max(1) as usize);
        let gap = config
            .get("gap")
            .and_then(Value::as_i64)
            .unwrap_or(8)
            .clamp(0, 256) as i32;
        let usable_width =
            (request.bounds.width - gap.saturating_mul(columns.saturating_sub(1) as i32)).max(0);
        let base = usable_width / columns as i32;
        let rem = usable_width % columns as i32;
        let mut heights = vec![0_i32; columns];
        let mut output = Vec::with_capacity(count);
        for (index, child) in request.children.iter().enumerate() {
            let col = heights
                .iter()
                .enumerate()
                .min_by_key(|(i, h)| (**h, *i))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let width = base + i32::from(col < rem as usize);
            let height = child.preferred.height.max(child.minimum.height);
            if height > request.bounds.height.saturating_sub(heights[col]) {
                return Err(layout_error(
                    "Masonry content exceeds its viewport; the host must provide scrolling or more height.",
                    self.modality(),
                ));
            }
            output.push(ExtensionPlacement {
                child_index: index,
                bounds: Rect {
                    x: request.bounds.x + col as i32 * (base + gap),
                    y: request.bounds.y + heights[col],
                    width,
                    height,
                },
            });
            heights[col] = heights[col].saturating_add(height).saturating_add(gap);
        }
        Ok(output)
    }
}

struct SpatialLayout;
impl ExtensionLayoutProvider for SpatialLayout {
    fn modality(&self) -> &str {
        "doofusdock.spatial"
    }
    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>> {
        let config = parse_config(request.payload, self.modality())?;
        let entries = config.get("placements").and_then(Value::as_array);
        let count = request.children.len();
        let mut output = Vec::with_capacity(count);
        for index in 0..count {
            let child = request.children[index];
            let (x, y, width, height) = if let Some(item) = entries.and_then(|v| v.get(index)) {
                let x = normalized(item, "x", 0.0)?;
                let y = normalized(item, "y", 0.0)?;
                let w = normalized(item, "width", 0.3)?.max(0.0);
                let h = normalized(item, "height", 0.25)?.max(0.0);
                (x, y, w, h)
            } else {
                let col = index % 3;
                let row = index / 3;
                (col as f64 * 0.32, row as f64 * 0.30, 0.30, 0.26)
            };
            let width = ((request.bounds.width as f64 * width).round() as i32)
                .max(child.minimum.width)
                .min(request.bounds.width);
            let height = ((request.bounds.height as f64 * height).round() as i32)
                .max(child.minimum.height)
                .min(request.bounds.height);
            let max_x = (request.bounds.width - width).max(0);
            let max_y = (request.bounds.height - height).max(0);
            output.push(ExtensionPlacement {
                child_index: index,
                bounds: Rect {
                    x: request.bounds.x
                        + ((request.bounds.width as f64 * x).round() as i32).clamp(0, max_x),
                    y: request.bounds.y
                        + ((request.bounds.height as f64 * y).round() as i32).clamp(0, max_y),
                    width,
                    height,
                },
            });
        }
        Ok(output)
    }
}

struct NodeGraphLayout;
impl ExtensionLayoutProvider for NodeGraphLayout {
    fn modality(&self) -> &str {
        "doofusdock.node_graph"
    }
    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>> {
        let config = parse_config(request.payload, self.modality())?;
        let nodes = config.get("nodes").and_then(Value::as_array);
        let cols = (request.bounds.width / 240).max(1) as usize;
        let width = 200.min(request.bounds.width);
        let mut output = Vec::with_capacity(request.children.len());
        for (index, child) in request.children.iter().enumerate() {
            let item = nodes.and_then(|v| v.get(index));
            let x = item
                .and_then(|v| v.get("x"))
                .and_then(Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or((index % cols) as i32 * 240);
            let y = item
                .and_then(|v| v.get("y"))
                .and_then(Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or((index / cols) as i32 * 140);
            let w = item
                .and_then(|v| v.get("width"))
                .and_then(Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(width)
                .clamp(
                    child.minimum.width.min(request.bounds.width),
                    request.bounds.width,
                );
            let h = item
                .and_then(|v| v.get("height"))
                .and_then(Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(112)
                .clamp(
                    child.minimum.height.min(request.bounds.height),
                    request.bounds.height,
                );
            let x = x.clamp(0, (request.bounds.width - w).max(0));
            let y = y.clamp(0, (request.bounds.height - h).max(0));
            output.push(ExtensionPlacement {
                child_index: index,
                bounds: Rect {
                    x: request.bounds.x + x,
                    y: request.bounds.y + y,
                    width: w,
                    height: h,
                },
            });
        }
        Ok(output)
    }
}

struct LayeredLayout;
impl ExtensionLayoutProvider for LayeredLayout {
    fn modality(&self) -> &str {
        crate::LAYERED_MODALITY
    }

    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>> {
        if request.child_ids.len() != request.children.len() {
            return Err(layout_error(
                "Layered child IDs and metrics must have equal length.",
                self.modality(),
            ));
        }
        let config = crate::LayeredConfiguration::parse_ids(request.payload, request.child_ids)?;
        let count = request.children.len();
        let step = 14_i32
            .min(request.bounds.width / count.max(1) as i32)
            .min(request.bounds.height / count.max(1) as i32);
        let mut output = Vec::with_capacity(count);
        for id in &config.back_to_front {
            let index = request
                .child_ids
                .iter()
                .position(|candidate| candidate == id)
                .ok_or_else(|| {
                    layout_error(
                        "Layered configuration references an unknown child.",
                        self.modality(),
                    )
                })?;
            let offset = step.saturating_mul(output.len() as i32);
            let metrics = request.children[index];
            let width = request
                .bounds
                .width
                .saturating_sub(offset)
                .max(0)
                .max(metrics.minimum.width.min(request.bounds.width));
            let height = request
                .bounds
                .height
                .saturating_sub(offset)
                .max(0)
                .max(metrics.minimum.height.min(request.bounds.height));
            output.push(ExtensionPlacement {
                child_index: index,
                bounds: Rect {
                    x: request.bounds.x + offset,
                    y: request.bounds.y + offset,
                    width: width.min(request.bounds.width.saturating_sub(offset)),
                    height: height.min(request.bounds.height.saturating_sub(offset)),
                },
            });
        }
        Ok(output)
    }
}

struct StrategyLayout {
    modality: &'static str,
}

impl ExtensionLayoutProvider for StrategyLayout {
    fn modality(&self) -> &str {
        self.modality
    }

    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>> {
        let config = LayoutStrategyConfiguration::from_payload(request.payload)?;
        if config.strategy.modality() != self.modality {
            return Err(layout_error(
                "Layout strategy payload does not match its canonical modality.",
                self.modality,
            ));
        }
        arrange_layout_strategy(config, request.bounds, request.children)
    }
}

fn parse_config(payload: &str, context: &str) -> DockResult<Value> {
    serde_json::from_str(payload).map_err(|error| {
        DockError::new(
            ErrorCode::InvalidLayout,
            format!("Invalid extension layout payload: {error}"),
            context,
        )
    })
}
fn normalized(value: &Value, key: &str, default: f64) -> DockResult<f64> {
    let Some(value) = value.get(key) else {
        return Ok(default);
    };
    let number = value
        .as_f64()
        .ok_or_else(|| layout_error("Spatial coordinates and sizes must be numeric.", key))?;
    if !number.is_finite() {
        return Err(layout_error("Spatial coordinates must be finite.", key));
    }
    Ok(number.clamp(0.0, 1.0))
}
fn layout_error(message: &str, context: impl Into<String>) -> DockError {
    DockError::new(ErrorCode::InvalidLayout, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;

    fn child(height: i32) -> ExtensionChildMetrics {
        ExtensionChildMetrics {
            minimum: Size::new(40, 30),
            preferred: Size::new(120, height),
        }
    }

    fn request<'a>(
        modality: &'a str,
        payload: &'a str,
        children: &'a [ExtensionChildMetrics],
    ) -> ExtensionLayoutRequest<'a> {
        ExtensionLayoutRequest {
            modality,
            payload,
            bounds: Rect {
                x: 10,
                y: 20,
                width: 600,
                height: 400,
            },
            child_ids: &[],
            children,
        }
    }

    #[test]
    fn builtin_geometry_is_repeatable_bounded_or_returns_typed_infeasibility() {
        let registry = ExtensionLayoutRegistry::with_builtins();
        let modalities = [
            "doofusdock.masonry",
            "doofusdock.spatial",
            "doofusdock.node_graph",
            crate::LAYERED_MODALITY,
        ];
        let mut seed = 0x5eed_u64;

        for case in 0..64 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let count = (seed as usize % 7) + 1;
            let ids: Vec<_> = (0..count)
                .map(|index| NodeId::from(format!("case-{case}-child-{index}")))
                .collect();
            let children: Vec<_> = (0..count)
                .map(|index| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let minimum = 8 + (seed % 48) as i32;
                    ExtensionChildMetrics {
                        minimum: Size::new(minimum, minimum / 2),
                        preferred: Size::new(minimum + 20, 24 + (index as i32 * 13)),
                    }
                })
                .collect();

            for modality in modalities {
                let layered = crate::LayeredConfiguration {
                    version: 1,
                    back_to_front: ids.clone(),
                    active: ids[case % count].clone(),
                };
                let payload = match modality {
                    "doofusdock.masonry" => {
                        format!(r#"{{"columns":{},"gap":{}}}"#, case % 5 + 1, case % 9)
                    }
                    "doofusdock.spatial" => {
                        r#"{"placements":[{"x":0.7,"y":0.8,"width":0.4,"height":0.4}]}"#.to_owned()
                    }
                    "doofusdock.node_graph" => {
                        r#"{"nodes":[{"x":2147483647,"y":2147483647,"width":240,"height":160}]}"#
                            .to_owned()
                    }
                    crate::LAYERED_MODALITY => layered.to_payload().unwrap(),
                    _ => unreachable!(),
                };
                let request = ExtensionLayoutRequest {
                    modality,
                    payload: &payload,
                    bounds: Rect {
                        x: -80,
                        y: 35,
                        width: 640,
                        height: 480,
                    },
                    child_ids: &ids,
                    children: &children,
                };
                let first = registry.arrange(&request);
                let second = registry.arrange(&request);
                assert_eq!(
                    first, second,
                    "{modality} case {case} changed across identical inputs"
                );
                if let Ok(placements) = first {
                    assert_eq!(placements.len(), count);
                    for placement in placements {
                        let bounds = placement.bounds;
                        assert!(bounds.width >= children[placement.child_index].minimum.width);
                        assert!(bounds.height >= children[placement.child_index].minimum.height);
                        assert!(bounds.x >= request.bounds.x && bounds.y >= request.bounds.y);
                        assert!(
                            bounds.x.saturating_add(bounds.width)
                                <= request.bounds.x + request.bounds.width
                        );
                        assert!(
                            bounds.y.saturating_add(bounds.height)
                                <= request.bounds.y + request.bounds.height
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn builtins_arrange_masonry_spatial_and_node_graph_children() {
        let registry = ExtensionLayoutRegistry::with_builtins();
        let children = [child(150), child(100), child(180), child(80)];

        let masonry = registry
            .arrange(&request(
                "doofusdock.masonry",
                r#"{"columns":2,"gap":10}"#,
                &children,
            ))
            .unwrap();
        assert_eq!(masonry.len(), children.len());
        assert_eq!(masonry[0].bounds.x, 10);
        assert_eq!(masonry[1].bounds.x, 315);
        assert_eq!(masonry[2].bounds.y, 130);

        let spatial = registry
            .arrange(&request(
                "doofusdock.spatial",
                r#"{"placements":[{"x":0.5,"y":0.25,"width":0.4,"height":0.5}]}"#,
                &children[..1],
            ))
            .unwrap();
        assert_eq!(
            spatial[0].bounds,
            Rect {
                x: 310,
                y: 120,
                width: 240,
                height: 200
            }
        );

        let graph = registry
            .arrange(&request(
                "doofusdock.node_graph",
                r#"{"nodes":[{"x":90,"y":60,"width":180,"height":90}]}"#,
                &children[..1],
            ))
            .unwrap();
        assert_eq!(
            graph[0].bounds,
            Rect {
                x: 100,
                y: 80,
                width: 180,
                height: 90
            }
        );
        assert_eq!(
            hit_test_extension_placements(&spatial, Point { x: 310, y: 120 }),
            Some(0)
        );
        assert_eq!(
            hit_test_extension_placements(&spatial, Point { x: 10, y: 20 }),
            None
        );
    }

    #[test]
    fn layered_layout_overlaps_in_persistent_back_to_front_order() {
        let registry = ExtensionLayoutRegistry::with_builtins();
        let ids = [
            NodeId::from("back"),
            NodeId::from("middle"),
            NodeId::from("front"),
        ];
        let children = [child(100), child(100), child(100)];
        let config = crate::LayeredConfiguration {
            version: 1,
            back_to_front: ids.to_vec(),
            active: ids[2].clone(),
        };
        let payload = config.to_payload().unwrap();
        let request = ExtensionLayoutRequest {
            modality: crate::LAYERED_MODALITY,
            payload: &payload,
            bounds: Rect {
                x: 10,
                y: 20,
                width: 600,
                height: 400,
            },
            child_ids: &ids,
            children: &children,
        };
        let placements = registry.arrange(&request).unwrap();
        assert_eq!(
            placements.iter().map(|p| p.child_index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(placements[0].bounds.x < placements[1].bounds.x);
        assert!(placements[1].bounds.x < placements[2].bounds.x);
        assert_eq!(
            hit_test_extension_placements(&placements, Point { x: 50, y: 60 }),
            Some(2),
            "the frontmost layer wins overlapping hit tests"
        );
    }

    struct InvalidProvider;
    impl ExtensionLayoutProvider for InvalidProvider {
        fn modality(&self) -> &str {
            "example.invalid"
        }
        fn arrange(
            &self,
            _request: &ExtensionLayoutRequest<'_>,
        ) -> DockResult<Vec<ExtensionPlacement>> {
            Ok(vec![ExtensionPlacement {
                child_index: 0,
                bounds: Rect {
                    x: -1,
                    y: 0,
                    width: 10,
                    height: 10,
                },
            }])
        }
    }

    #[test]
    #[allow(deprecated)]
    fn legacy_layout_registry_refuses_unmanaged_dynamic_providers() {
        let mut registry = ExtensionLayoutRegistry::with_builtins();
        assert!(registry.register(Arc::new(InvalidProvider)).is_err());
        let children = [child(50)];
        assert!(
            registry
                .arrange(&request("example.invalid", "{}", &children))
                .is_err()
        );
    }

    #[test]
    fn unknown_provider_and_invalid_geometry_fail_closed() {
        let registry = ExtensionLayoutRegistry::with_builtins();
        assert!(
            registry
                .arrange(&request("unknown.provider", "{}", &[]))
                .is_err()
        );
        assert!(
            registry
                .arrange(&request("doofusdock.spatial", "{bad-json", &[]))
                .is_err()
        );
        let mut bad = request("doofusdock.masonry", "{}", &[]);
        bad.bounds.width = -1;
        assert!(registry.arrange(&bad).is_err());
        let too_tall = [child(500)];
        assert!(
            registry
                .arrange(&request("doofusdock.masonry", "{}", &too_tall))
                .is_err()
        );
        let too_wide = [ExtensionChildMetrics {
            minimum: Size::new(700, 30),
            preferred: Size::new(700, 80),
        }];
        assert!(
            registry
                .arrange(&request("doofusdock.spatial", "{}", &too_wide))
                .is_err()
        );
    }

    fn scroll_columns(ids: &[NodeId]) -> ScrollableColumnsConfiguration {
        ScrollableColumnsConfiguration::new(
            ids.iter()
                .cloned()
                .map(|child_id| ScrollableColumn {
                    child_id,
                    width: 80,
                    minimum_width: 60,
                    maximum_width: Some(120),
                })
                .collect(),
            10,
        )
        .unwrap()
    }

    #[test]
    fn scrollable_columns_virtualize_by_stable_identity_and_keep_widths() {
        let ids: Vec<_> = (0..10)
            .map(|index| NodeId::from(format!("child-{index}")))
            .collect();
        let metrics = vec![child(50); ids.len()];
        let config = scroll_columns(&ids);
        let encoded = serde_json::to_string(&config).unwrap();
        let decoded: ScrollableColumnsConfiguration = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, config);

        let frame = arrange_scrollable_columns(
            &config,
            &ids,
            &metrics,
            Rect {
                x: 40,
                y: 20,
                width: 200,
                height: 100,
            },
            300,
            1,
        )
        .unwrap();
        assert_eq!(frame.content_width, 890);
        assert_eq!(frame.scroll_offset, 300);
        assert_eq!(frame.visible_range, 3..6);
        assert_eq!(frame.overscan_range, 2..7);
        assert_eq!(frame.placements.len(), 5);
        assert_eq!(frame.placements[0].child_index, 2);
        assert_eq!(frame.placements[0].bounds.x, -80);
        assert_eq!(frame.placements[4].child_index, 6);
        assert_eq!(frame.placements[4].bounds.width, 80);
    }

    #[test]
    fn scrollable_column_width_edits_clamp_by_stable_child_identity() {
        let ids = [NodeId::from("first"), NodeId::from("second")];
        let config = scroll_columns(&ids);
        let wider = config.resized_column(&ids[1], 1000).unwrap();
        assert_eq!(wider.columns[0].width, 80);
        assert_eq!(wider.columns[1].width, 120);

        let narrower = wider.resized_column(&ids[1], -1000).unwrap();
        assert_eq!(narrower.columns[1].width, 60);
        assert_eq!(narrower.columns[1].child_id, ids[1]);
        assert!(
            narrower
                .resized_column(&NodeId::from("missing"), 16)
                .is_err()
        );
    }

    #[test]
    fn scrollable_columns_round_trip_through_canonical_node_payload() {
        let children = vec![Node::panel("panel-a", "a"), Node::panel("panel-b", "b")];
        let ids = children
            .iter()
            .map(|child| child.id.clone())
            .collect::<Vec<_>>();
        let config = scroll_columns(&ids);
        let node = config
            .clone()
            .into_node("columns", children.clone())
            .unwrap();
        assert_eq!(
            ScrollableColumnsConfiguration::from_node(&node).unwrap(),
            config
        );

        let mut resized = config.clone();
        resized.columns[0].width = 96;
        let updated = resized.update_node(&node).unwrap();
        assert_eq!(updated.id, node.id);
        assert_eq!(
            ScrollableColumnsConfiguration::from_node(&updated).unwrap(),
            resized
        );
        let NodeKind::Extension {
            children: retained, ..
        } = &updated.kind
        else {
            panic!("updated canonical node must remain an extension container");
        };
        assert_eq!(retained, &children);

        let mut document = crate::LayoutDocument {
            root: Some(updated),
            ..crate::LayoutDocument::new("layout", "workspace")
        };
        document.panels = Arc::new(vec![
            crate::PanelDescriptor::new("a", "A"),
            crate::PanelDescriptor::new("b", "B"),
        ]);
        let restored = crate::deserialize_layout(
            &crate::serialize_layout(&document).unwrap(),
            crate::PersistenceLimits::default(),
        )
        .unwrap();
        let root = restored.root.as_deref().unwrap();
        assert_eq!(
            ScrollableColumnsConfiguration::from_node(root).unwrap(),
            resized
        );

        let mut service = crate::DockingService::new(document, 4).unwrap();
        let current_root = service.snapshot().document.root.unwrap();
        let operation = config.update_operation(&current_root).unwrap();
        service
            .transact(0, "restore-column-widths".into(), [operation])
            .unwrap();
        assert_eq!(
            ScrollableColumnsConfiguration::from_node(
                service.snapshot().document.root.as_deref().unwrap()
            )
            .unwrap(),
            config
        );
        service.undo(1).unwrap();
        assert_eq!(
            ScrollableColumnsConfiguration::from_node(
                service.snapshot().document.root.as_deref().unwrap()
            )
            .unwrap(),
            resized
        );
    }

    #[test]
    fn scrollable_columns_canonical_payload_rejects_child_identity_drift() {
        let config = scroll_columns(&[NodeId::from("a"), NodeId::from("b")]);
        assert!(
            config
                .clone()
                .into_node(
                    "columns",
                    vec![Node::panel("a", "a"), Node::panel("other", "b")]
                )
                .is_err()
        );
        let ordinary_node = Node::panel("panel", "a");
        assert!(ScrollableColumnsConfiguration::from_node(&ordinary_node).is_err());
    }

    #[test]
    fn layout_strategy_catalog_is_stable_bounded_and_respects_minimums() {
        let strategies = [
            LayoutStrategy::MasterStack,
            LayoutStrategy::BinarySpacePartition,
            LayoutStrategy::Spiral,
            LayoutStrategy::Dwindle,
            LayoutStrategy::Monocle,
            LayoutStrategy::EqualColumns,
            LayoutStrategy::EqualRows,
            LayoutStrategy::AdaptiveGrid,
        ];
        let children = vec![
            ExtensionChildMetrics {
                minimum: Size::new(1, 1),
                preferred: Size::new(30, 30)
            };
            6
        ];
        let child_ids = (0..children.len())
            .map(|index| NodeId::from(format!("strategy-{index}")))
            .collect::<Vec<_>>();
        let viewport = Rect {
            x: 10,
            y: 20,
            width: 600,
            height: 400,
        };
        let registry = ExtensionLayoutRegistry::with_builtins();
        for strategy in strategies {
            let config = LayoutStrategyConfiguration::new(strategy, 8).unwrap();
            let payload = config.to_payload().unwrap();
            assert_eq!(
                LayoutStrategyConfiguration::from_payload(&payload).unwrap(),
                config
            );
            let request = ExtensionLayoutRequest {
                modality: strategy.modality(),
                payload: &payload,
                bounds: viewport,
                child_ids: &child_ids,
                children: &children,
            };
            let first = registry.arrange(&request).unwrap();
            let second = registry.arrange(&request).unwrap();
            assert_eq!(first, second);
            assert_eq!(first.len(), children.len());
            assert!(first.iter().all(|placement| {
                placement.bounds.x >= viewport.x
                    && placement.bounds.y >= viewport.y
                    && placement.bounds.x.saturating_add(placement.bounds.width)
                        <= viewport.x + viewport.width
                    && placement.bounds.y.saturating_add(placement.bounds.height)
                        <= viewport.y + viewport.height
            }));
        }
        let too_small = vec![ExtensionChildMetrics {
            minimum: Size::new(700, 500),
            preferred: Size::default(),
        }];
        assert!(
            arrange_layout_strategy(
                LayoutStrategyConfiguration::new(LayoutStrategy::Monocle, 0).unwrap(),
                viewport,
                &too_small,
            )
            .is_err()
        );
        let payload = LayoutStrategyConfiguration::new(LayoutStrategy::MasterStack, 4)
            .unwrap()
            .to_payload()
            .unwrap();
        assert!(
            registry
                .arrange(&ExtensionLayoutRequest {
                    modality: LayoutStrategy::Spiral.modality(),
                    payload: &payload,
                    bounds: viewport,
                    child_ids: &child_ids,
                    children: &children,
                })
                .is_err()
        );

        let mut document = crate::LayoutDocument::new("strategy", "workspace");
        let node_children = vec![Node::panel("strategy-panel-node", "strategy-panel")];
        document.panels = Arc::new(vec![crate::PanelDescriptor::new(
            "strategy-panel",
            "Strategy",
        )]);
        document.root = Some(Arc::new(Node {
            id: NodeId::from("strategy-container"),
            kind: NodeKind::Extension {
                modality: LayoutStrategy::MasterStack.modality().into(),
                payload: LayoutStrategyConfiguration::new(LayoutStrategy::MasterStack, 4)
                    .unwrap()
                    .to_payload()
                    .unwrap(),
                children: node_children,
            },
        }));
        assert!(crate::validate_layout(&document).is_ok());
        let root = Arc::make_mut(document.root.as_mut().unwrap());
        let NodeKind::Extension {
            modality, payload, ..
        } = &mut root.kind
        else {
            unreachable!();
        };
        *modality = LayoutStrategy::Spiral.modality().into();
        *payload = LayoutStrategyConfiguration::new(LayoutStrategy::MasterStack, 4)
            .unwrap()
            .to_payload()
            .unwrap();
        assert!(crate::validate_layout(&document).is_err());
    }

    #[test]
    fn scrollable_columns_clamp_offset_and_respect_configured_order() {
        let ids = [NodeId::from("a"), NodeId::from("b"), NodeId::from("c")];
        let metrics = vec![child(50); ids.len()];
        let mut config = scroll_columns(&ids);
        config.columns.reverse();
        let frame = arrange_scrollable_columns(
            &config,
            &ids,
            &metrics,
            Rect {
                x: 0,
                y: 0,
                width: 400,
                height: 100,
            },
            i32::MAX,
            0,
        )
        .unwrap();
        assert_eq!(frame.content_width, 260);
        assert_eq!(frame.scroll_offset, 0);
        assert_eq!(
            frame
                .placements
                .iter()
                .map(|placement| placement.child_index)
                .collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
    }

    #[test]
    fn scrollable_columns_reject_invalid_or_unbounded_configuration() {
        let ids = [NodeId::from("a"), NodeId::from("b")];
        let metrics = vec![child(50); ids.len()];
        let config = scroll_columns(&ids);
        let viewport = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        assert!(arrange_scrollable_columns(&config, &ids[..1], &metrics, viewport, 0, 0).is_err());
        assert!(arrange_scrollable_columns(&config, &ids, &metrics, viewport, 0, 65).is_err());
        let mut duplicate = config.clone();
        duplicate.columns[1].child_id = duplicate.columns[0].child_id.clone();
        assert!(duplicate.validate().is_err());
        let mut too_wide = config;
        too_wide.columns[0].width = i32::MAX;
        assert!(arrange_scrollable_columns(&too_wide, &ids, &metrics, viewport, 0, 0).is_err());
    }
}
