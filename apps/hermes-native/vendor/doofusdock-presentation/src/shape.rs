//! Bounded, toolkit-neutral 2D shape primitives for transient presentation and hit testing.
//!
//! These values do not alter canonical docking bounds or native OS window shapes.

use crate::{Affine2, Box2, ScenePoint};
use geo::algorithm::bool_ops::{BooleanOps, FillRule, OpType};
use geo::algorithm::buffer::{BufferStyle, LineJoin};
use geo::algorithm::validation::Validation;
use geo::{
    Area, Buffer, ChaikinSmoothing, Contains, Coord, Intersects, LineString, MultiPolygon, Point,
    Polygon, TriangulateEarcut,
};
use serde::{Deserialize, Serialize};
use std::fmt;

const MAX_COORDINATE: f64 = 1_000_000.0;
const MAX_POLYGON_VERTICES: usize = 4096;
const MAX_BOOLEAN_VERTICES: usize = 2048;
const MAX_FILL_TRIANGLES: usize = 16_384;
const MAX_PATH_SMOOTHING_ITERATIONS: u8 = 8;
const MAX_PATH_COMMANDS: usize = 4096;
const MAX_PATH_DEPTH: u8 = 16;
const EDGE_EPSILON: f64 = 1.0e-9;
const MIN_SUPERELLIPSE_EXPONENT: f64 = 1.0;
const MAX_SUPERELLIPSE_EXPONENT: f64 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapeError {
    InvalidBounds,
    InvalidRadius,
    InvalidEllipse,
    InvalidExponent,
    InvalidInterpolation,
    InvalidTransform,
    IncompatibleGeometry,
    InvalidPolygon,
    InvalidPoint,
    TooManyVertices,
    InvalidBooleanResult,
    InvalidOffset,
    InvalidPath,
    InvalidGradient,
    InvalidStrokeWidth,
    InvalidTolerance,
    TooManySegments,
    SubdivisionLimit,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => {
                f.write_str("shape bounds must be finite, bounded, and nonnegative")
            }
            Self::InvalidRadius => {
                f.write_str("corner radii must be finite, nonnegative, and fit the bounds")
            }
            Self::InvalidEllipse => {
                f.write_str("ellipse center and radii must be finite, bounded, and positive")
            }
            Self::InvalidExponent => {
                f.write_str("superellipse exponent must be finite and within 1..=8")
            }
            Self::InvalidInterpolation => {
                f.write_str("shape interpolation amount must be finite and within 0..=1")
            }
            Self::InvalidTransform => {
                f.write_str("path transform must be finite and invertible")
            }
            Self::IncompatibleGeometry => {
                f.write_str("shape interpolation requires matching primitive variants and topology")
            }
            Self::InvalidPolygon => {
                f.write_str("polygon needs three distinct finite vertices and nonzero area")
            }
            Self::InvalidPoint => {
                f.write_str("hit-test point must be finite and within the coordinate limit")
            }
            Self::TooManyVertices => f.write_str("polygon vertex count exceeds the shape limit"),
            Self::InvalidBooleanResult => {
                f.write_str("boolean geometry output is invalid or exceeds coordinate limits")
            }
            Self::InvalidOffset => {
                f.write_str("path offset distance must be finite and within coordinate limits")
            }
            Self::InvalidPath => f.write_str("path commands must form closed, valid contours"),
            Self::InvalidGradient => {
                f.write_str("gradient endpoints must be finite and define a nonzero vector")
            }
            Self::InvalidStrokeWidth => {
                f.write_str("path stroke width must be finite and within 0..=4096")
            }
            Self::InvalidTolerance => {
                f.write_str("path flattening tolerance must be finite and bounded")
            }
            Self::TooManySegments => f.write_str("flattened path exceeds the segment limit"),
            Self::SubdivisionLimit => f.write_str("Bezier path cannot meet the requested tolerance within the subdivision depth limit"),
        }
    }
}

impl std::error::Error for ShapeError {}

/// Commands for a closed compound Bézier path. Each contour starts with
/// `MoveTo` and must end with `Close`; multiple contours use even-odd fill.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathCommand2D {
    MoveTo {
        point: [f64; 2],
    },
    LineTo {
        point: [f64; 2],
    },
    QuadraticTo {
        control: [f64; 2],
        point: [f64; 2],
    },
    CubicTo {
        control1: [f64; 2],
        control2: [f64; 2],
        point: [f64; 2],
    },
    Close,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Path2D {
    pub commands: Vec<PathCommand2D>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlattenedPath2D {
    contours: Vec<Vec<[f64; 2]>>,
    bounds: Box2,
}

/// Set operation applied to two bounded, flattened compound paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathBooleanOp2D {
    Union,
    Intersection,
    Difference,
    ExclusiveOr,
}

impl FlattenedPath2D {
    pub fn contours(&self) -> &[Vec<[f64; 2]>] {
        &self.contours
    }

    pub fn bounds(&self) -> Box2 {
        self.bounds
    }

    /// Transform all contours and revalidate simple even-odd topology. The
    /// vertex budget matches the planar Boolean API so transformed paths can
    /// be safely used for clipped scene painting.
    pub fn transformed(&self, transform: Affine2) -> Result<Self, ShapeError> {
        transform
            .inverse()
            .map_err(|_| ShapeError::InvalidTransform)?;
        let vertices = self.contours.iter().map(Vec::len).sum::<usize>();
        if vertices > MAX_BOOLEAN_VERTICES {
            return Err(ShapeError::TooManySegments);
        }
        if self.contours.is_empty() {
            return Ok(Self {
                contours: Vec::new(),
                bounds: Box2::default(),
            });
        }
        let contours = self
            .contours
            .iter()
            .map(|contour| {
                contour
                    .iter()
                    .map(|[x, y]| {
                        let point = transform.transform_point(ScenePoint { x: *x, y: *y });
                        let point = [point.x, point.y];
                        if point_valid(point) {
                            Ok(point)
                        } else {
                            Err(ShapeError::InvalidBooleanResult)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for [x, y] in contours.iter().flatten() {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
        let transformed = Self {
            contours,
            bounds: Box2 {
                x: min_x,
                y: min_y,
                w: max_x - min_x,
                h: max_y - min_y,
            },
        };
        validate_bounds(transformed.bounds).map_err(|_| ShapeError::InvalidBooleanResult)?;
        boolean_path_geometry(&transformed)?;
        Ok(transformed)
    }

    pub fn contains(&self, point: [f64; 2]) -> Result<bool, ShapeError> {
        if !point_valid(point) {
            return Err(ShapeError::InvalidPoint);
        }
        let mut inside = false;
        for contour in &self.contours {
            if polygon_contains(contour, point) {
                if contour
                    .iter()
                    .zip(contour.iter().cycle().skip(1))
                    .take(contour.len())
                    .any(|(a, b)| point_on_segment(point, *a, *b))
                {
                    return Ok(true);
                }
                inside = !inside;
            }
        }
        Ok(inside)
    }

    /// Combine this even-odd filled path with another path using bounded planar
    /// Boolean geometry. Curves must be flattened before this call; the output
    /// is a deterministic, bounded set of straight contours.
    pub fn boolean(&self, other: &Self, operation: PathBooleanOp2D) -> Result<Self, ShapeError> {
        let left = boolean_path_geometry(self)?;
        let right = boolean_path_geometry(other)?;
        let result = match operation {
            PathBooleanOp2D::Union => {
                left.boolean_op_with_fill_rule(&right, OpType::Union, FillRule::EvenOdd)
            }
            PathBooleanOp2D::Intersection => {
                left.boolean_op_with_fill_rule(&right, OpType::Intersection, FillRule::EvenOdd)
            }
            PathBooleanOp2D::Difference => {
                left.boolean_op_with_fill_rule(&right, OpType::Difference, FillRule::EvenOdd)
            }
            PathBooleanOp2D::ExclusiveOr => {
                left.boolean_op_with_fill_rule(&right, OpType::Xor, FillRule::EvenOdd)
            }
        };
        flattened_from_boolean_result(result)
    }

    /// Offset this even-odd filled path in local 2D coordinates. Positive
    /// distances expand the filled region; negative distances contract it.
    /// Bevel joins keep generated geometry simpler than rounded joins. Output
    /// is revalidated and must fit the existing path and coordinate limits.
    pub fn offset(&self, distance: f64) -> Result<Self, ShapeError> {
        if !distance.is_finite() || distance.abs() > MAX_COORDINATE {
            return Err(ShapeError::InvalidOffset);
        }
        let geometry = boolean_path_geometry(self)?;
        if distance == 0.0 {
            return Ok(self.clone());
        }
        let offset =
            geometry.buffer_with_style(BufferStyle::new(distance).line_join(LineJoin::Bevel));
        flattened_from_boolean_result(offset)
    }

    /// Apply bounded Chaikin corner cutting to every contour. Each iteration
    /// doubles the unique vertex count, so the output budget is checked before
    /// geometry allocation. The smoothed result is revalidated; if smoothing
    /// collapses or crosses a hole, the operation fails closed.
    pub fn smoothed(&self, iterations: u8) -> Result<Self, ShapeError> {
        if iterations > MAX_PATH_SMOOTHING_ITERATIONS {
            return Err(ShapeError::TooManySegments);
        }
        let geometry = boolean_path_geometry(self)?;
        if iterations == 0 {
            return Ok(self.clone());
        }
        let multiplier = 1usize
            .checked_shl(u32::from(iterations))
            .ok_or(ShapeError::TooManySegments)?;
        let unique_vertices = geometry
            .0
            .iter()
            .flat_map(|polygon| std::iter::once(polygon.exterior()).chain(polygon.interiors()))
            .try_fold(0usize, |total, ring| {
                total.checked_add(ring.0.len().saturating_sub(1))
            })
            .ok_or(ShapeError::TooManySegments)?;
        if unique_vertices
            .checked_mul(multiplier)
            .is_none_or(|count| count > MAX_BOOLEAN_VERTICES)
        {
            return Err(ShapeError::TooManySegments);
        }
        flattened_from_boolean_result(geometry.chaikin_smoothing(usize::from(iterations)))
    }

    /// Tessellate the validated even-odd fill into bounded triangles. Holes
    /// remain empty; returned triangles are transient paint geometry and do
    /// not alter path hit testing or canonical layout state.
    pub fn tessellate(&self) -> Result<Vec<[[f64; 2]; 3]>, ShapeError> {
        let geometry = boolean_path_geometry(self)?;
        let mut triangles = Vec::new();
        for polygon in &geometry.0 {
            for triangle in polygon.earcut_triangles() {
                if triangles.len() >= MAX_FILL_TRIANGLES {
                    return Err(ShapeError::TooManySegments);
                }
                let points =
                    [triangle.v1(), triangle.v2(), triangle.v3()].map(|point| [point.x, point.y]);
                if points.iter().any(|point| !point_valid(*point))
                    || triangle_area(points).abs() <= EDGE_EPSILON
                {
                    return Err(ShapeError::InvalidBooleanResult);
                }
                triangles.push(points);
            }
        }
        Ok(triangles)
    }
}

impl Path2D {
    /// Flatten the path with adaptive De Casteljau subdivision. `max_segments`
    /// bounds the total output across all contours.
    pub fn flatten(
        &self,
        tolerance: f64,
        max_segments: usize,
    ) -> Result<FlattenedPath2D, ShapeError> {
        if !tolerance.is_finite() || !(0.01..=64.0).contains(&tolerance) {
            return Err(ShapeError::InvalidTolerance);
        }
        if !(1..=65_536).contains(&max_segments) {
            return Err(ShapeError::TooManySegments);
        }
        if self.commands.is_empty() || self.commands.len() > MAX_PATH_COMMANDS {
            return Err(ShapeError::InvalidPath);
        }
        let mut contours: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current: Option<[f64; 2]> = None;
        let mut start: Option<[f64; 2]> = None;
        let mut contour: Vec<[f64; 2]> = Vec::new();
        let mut bounds_points = Vec::new();
        let mut segment_count = 0usize;
        for command in &self.commands {
            match *command {
                PathCommand2D::MoveTo { point } => {
                    if current.is_some() || !point_valid(point) {
                        return Err(ShapeError::InvalidPath);
                    }
                    current = Some(point);
                    start = Some(point);
                    contour.push(point);
                    bounds_points.push(point);
                }
                PathCommand2D::LineTo { point } => {
                    current.ok_or(ShapeError::InvalidPath)?;
                    if !point_valid(point) {
                        return Err(ShapeError::InvalidPath);
                    }
                    contour.push(point);
                    bounds_points.push(point);
                    current = Some(point);
                    segment_count += 1;
                }
                PathCommand2D::QuadraticTo { control, point } => {
                    let from = current.ok_or(ShapeError::InvalidPath)?;
                    if !point_valid(control) || !point_valid(point) {
                        return Err(ShapeError::InvalidPath);
                    }
                    bounds_points.push(control);
                    bounds_points.push(point);
                    flatten_quadratic(
                        from,
                        control,
                        point,
                        tolerance,
                        0,
                        &mut contour,
                        &mut segment_count,
                        max_segments,
                    )?;
                    current = Some(point);
                }
                PathCommand2D::CubicTo {
                    control1,
                    control2,
                    point,
                } => {
                    let from = current.ok_or(ShapeError::InvalidPath)?;
                    if !point_valid(control1) || !point_valid(control2) || !point_valid(point) {
                        return Err(ShapeError::InvalidPath);
                    }
                    bounds_points.extend([control1, control2, point]);
                    flatten_cubic(
                        from,
                        control1,
                        control2,
                        point,
                        tolerance,
                        0,
                        &mut contour,
                        &mut segment_count,
                        max_segments,
                    )?;
                    current = Some(point);
                }
                PathCommand2D::Close => {
                    let first = start.take().ok_or(ShapeError::InvalidPath)?;
                    if current.is_none() || contour.len() < 3 {
                        return Err(ShapeError::InvalidPath);
                    }
                    if contour.last() != Some(&first) {
                        contour.push(first);
                        segment_count += 1;
                    }
                    if segment_count > max_segments {
                        return Err(ShapeError::TooManySegments);
                    }
                    let mut polygon = contour.clone();
                    if polygon.first() == polygon.last() {
                        polygon.pop();
                    }
                    validate_polygon(&polygon)?;
                    contours.push(std::mem::take(&mut contour));
                    current = None;
                }
            }
            if segment_count > max_segments {
                return Err(ShapeError::TooManySegments);
            }
        }
        if current.is_some() || contours.is_empty() || bounds_points.is_empty() {
            return Err(ShapeError::InvalidPath);
        }
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for [x, y] in bounds_points {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let bounds = Box2 {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        };
        validate_bounds(bounds)?;
        Ok(FlattenedPath2D { contours, bounds })
    }

    /// Flatten and combine two compound paths. `tolerance` and
    /// `max_segments` use the same bounds as [`Path2D::flatten`]. Boolean
    /// processing additionally caps each input and the output at 2,048 total
    /// vertices to bound CPU and memory use.
    pub fn boolean(
        &self,
        other: &Self,
        operation: PathBooleanOp2D,
        tolerance: f64,
        max_segments: usize,
    ) -> Result<FlattenedPath2D, ShapeError> {
        self.flatten(tolerance, max_segments)?
            .boolean(&other.flatten(tolerance, max_segments)?, operation)
    }

    /// Interpolate matching closed path command streams and validate the
    /// sampled topology before returning the transient path.
    pub fn interpolate(
        &self,
        target: &Self,
        amount: f64,
        tolerance: f64,
        max_segments: usize,
    ) -> Result<Self, ShapeError> {
        if !amount.is_finite() || !(0.0..=1.0).contains(&amount) {
            return Err(ShapeError::InvalidInterpolation);
        }
        if self.commands.len() != target.commands.len() {
            return Err(ShapeError::IncompatibleGeometry);
        }
        let mix_point = |a: [f64; 2], b: [f64; 2]| {
            [a[0] + (b[0] - a[0]) * amount, a[1] + (b[1] - a[1]) * amount]
        };
        let commands = self
            .commands
            .iter()
            .zip(&target.commands)
            .map(|(from, to)| match (*from, *to) {
                (PathCommand2D::MoveTo { point: a }, PathCommand2D::MoveTo { point: b }) => {
                    Ok(PathCommand2D::MoveTo {
                        point: mix_point(a, b),
                    })
                }
                (PathCommand2D::LineTo { point: a }, PathCommand2D::LineTo { point: b }) => {
                    Ok(PathCommand2D::LineTo {
                        point: mix_point(a, b),
                    })
                }
                (
                    PathCommand2D::QuadraticTo {
                        control: control_a,
                        point: point_a,
                    },
                    PathCommand2D::QuadraticTo {
                        control: control_b,
                        point: point_b,
                    },
                ) => Ok(PathCommand2D::QuadraticTo {
                    control: mix_point(control_a, control_b),
                    point: mix_point(point_a, point_b),
                }),
                (
                    PathCommand2D::CubicTo {
                        control1: control1_a,
                        control2: control2_a,
                        point: point_a,
                    },
                    PathCommand2D::CubicTo {
                        control1: control1_b,
                        control2: control2_b,
                        point: point_b,
                    },
                ) => Ok(PathCommand2D::CubicTo {
                    control1: mix_point(control1_a, control1_b),
                    control2: mix_point(control2_a, control2_b),
                    point: mix_point(point_a, point_b),
                }),
                (PathCommand2D::Close, PathCommand2D::Close) => Ok(PathCommand2D::Close),
                _ => Err(ShapeError::IncompatibleGeometry),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sampled = Self { commands };
        let flattened = sampled.flatten(tolerance, max_segments)?;
        boolean_path_geometry(&flattened)?;
        Ok(sampled)
    }
}

fn boolean_path_geometry(path: &FlattenedPath2D) -> Result<MultiPolygon<f64>, ShapeError> {
    let vertex_count = path.contours.iter().map(Vec::len).sum::<usize>();
    if vertex_count > MAX_BOOLEAN_VERTICES {
        return Err(ShapeError::TooManySegments);
    }
    let rings = path
        .contours
        .iter()
        .map(|contour| {
            if contour.len() < 4 || contour.first() != contour.last() {
                return Err(ShapeError::InvalidPath);
            }
            let coordinates = contour
                .iter()
                .map(|[x, y]| Coord { x: *x, y: *y })
                .collect();
            let ring = LineString::new(coordinates);
            if !ring.is_valid() {
                return Err(ShapeError::InvalidPath);
            }
            Ok(ring)
        })
        .collect::<Result<Vec<_>, ShapeError>>()?;
    for left in 0..rings.len() {
        for right in left + 1..rings.len() {
            if rings[left].intersects(&rings[right]) {
                return Err(ShapeError::InvalidPath);
            }
        }
    }

    let polygons = rings
        .iter()
        .cloned()
        .map(|ring| Polygon::new(ring, Vec::new()))
        .collect::<Vec<_>>();
    let areas = polygons
        .iter()
        .map(Polygon::unsigned_area)
        .collect::<Vec<_>>();
    let mut parents = vec![None; polygons.len()];
    for child in 0..polygons.len() {
        let point = polygons[child]
            .exterior()
            .0
            .first()
            .ok_or(ShapeError::InvalidPath)?;
        let point = Point::new(point.x, point.y);
        parents[child] = (0..polygons.len())
            .filter(|&candidate| {
                candidate != child
                    && areas[candidate] > areas[child]
                    && polygons[candidate].contains(&point)
            })
            .min_by(|&left, &right| areas[left].total_cmp(&areas[right]));
    }
    let depths = parents
        .iter()
        .enumerate()
        .map(|(index, _)| contour_depth(index, &parents))
        .collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::new();
    for exterior in 0..polygons.len() {
        if depths[exterior] % 2 != 0 {
            continue;
        }
        let interiors = (0..polygons.len())
            .filter(|&candidate| parents[candidate] == Some(exterior))
            .filter(|&candidate| depths[candidate] % 2 == 1)
            .map(|candidate| polygons[candidate].exterior().clone())
            .collect();
        result.push(Polygon::new(
            polygons[exterior].exterior().clone(),
            interiors,
        ));
    }
    let result = MultiPolygon(result);
    if !result.is_valid() {
        return Err(ShapeError::InvalidPath);
    }
    Ok(result)
}

fn contour_depth(index: usize, parents: &[Option<usize>]) -> Result<usize, ShapeError> {
    let mut depth = 0usize;
    let mut current = parents[index];
    while let Some(parent) = current {
        depth = depth.checked_add(1).ok_or(ShapeError::InvalidPath)?;
        if depth > parents.len() {
            return Err(ShapeError::InvalidPath);
        }
        current = parents[parent];
    }
    Ok(depth)
}

fn flattened_from_boolean_result(result: MultiPolygon<f64>) -> Result<FlattenedPath2D, ShapeError> {
    if !result.is_valid() {
        return Err(ShapeError::InvalidBooleanResult);
    }
    let mut contours = Vec::new();
    let mut vertex_count = 0usize;
    for polygon in result.0 {
        for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
            let mut vertices = ring
                .0
                .iter()
                .map(|coordinate| [coordinate.x, coordinate.y])
                .collect::<Vec<_>>();
            if vertices.first() == vertices.last() {
                vertices.pop();
            }
            if vertices.len() < 3 {
                continue;
            }
            validate_polygon(&vertices).map_err(|_| ShapeError::InvalidBooleanResult)?;
            canonicalize_contour(&mut vertices);
            vertex_count = vertex_count.saturating_add(vertices.len());
            if vertex_count + contours.len() * 2 + 2 > MAX_PATH_COMMANDS
                || vertex_count > MAX_BOOLEAN_VERTICES
            {
                return Err(ShapeError::TooManySegments);
            }
            vertices.push(vertices[0]);
            contours.push(vertices);
        }
    }
    contours.sort_by(|left, right| compare_contours(left, right));
    if contours.is_empty() {
        return Ok(FlattenedPath2D {
            contours,
            bounds: Box2::default(),
        });
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for [x, y] in contours.iter().flatten() {
        if !point_valid([*x, *y]) {
            return Err(ShapeError::InvalidBooleanResult);
        }
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let bounds = Box2 {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    };
    validate_bounds(bounds).map_err(|_| ShapeError::InvalidBooleanResult)?;
    Ok(FlattenedPath2D { contours, bounds })
}

fn canonicalize_contour(vertices: &mut [[f64; 2]]) {
    let signed_area = vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .take(vertices.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>();
    if signed_area < 0.0 {
        vertices.reverse();
    }
    if let Some(first) = vertices
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1])))
        .map(|(index, _)| index)
    {
        vertices.rotate_left(first);
    }
}

fn compare_contours(a: &[[f64; 2]], b: &[[f64; 2]]) -> std::cmp::Ordering {
    a.first()
        .zip(b.first())
        .map(|(a, b)| a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1])))
        .unwrap_or_else(|| a.len().cmp(&b.len()))
        .then_with(|| a.len().cmp(&b.len()))
}

fn validate_polygon(vertices: &[[f64; 2]]) -> Result<(), ShapeError> {
    if vertices.len() < 3 || vertices.iter().any(|p| !point_valid(*p)) {
        return Err(ShapeError::InvalidPath);
    }
    let area2 = vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .take(vertices.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>();
    if !area2.is_finite() || area2.abs() <= EDGE_EPSILON {
        return Err(ShapeError::InvalidPath);
    }
    Ok(())
}

fn triangle_area(points: [[f64; 2]; 3]) -> f64 {
    let [a, b, c] = points;
    ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) * 0.5
}

fn distance_to_chord(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx.hypot(dy);
    if length <= EDGE_EPSILON {
        return squared_distance(point, a).sqrt();
    }
    ((point[0] - a[0]) * dy - (point[1] - a[1]) * dx).abs() / length
}

fn mid(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}

#[allow(clippy::too_many_arguments)]
fn flatten_quadratic(
    a: [f64; 2],
    c: [f64; 2],
    b: [f64; 2],
    tolerance: f64,
    depth: u8,
    out: &mut Vec<[f64; 2]>,
    count: &mut usize,
    limit: usize,
) -> Result<(), ShapeError> {
    let flat = distance_to_chord(c, a, b) <= tolerance;
    if flat {
        if *count >= limit {
            return Err(ShapeError::TooManySegments);
        }
        out.push(b);
        *count += 1;
        return Ok(());
    }
    if depth >= MAX_PATH_DEPTH {
        return Err(ShapeError::SubdivisionLimit);
    }
    let ac = mid(a, c);
    let cb = mid(c, b);
    let center = mid(ac, cb);
    flatten_quadratic(a, ac, center, tolerance, depth + 1, out, count, limit)?;
    flatten_quadratic(center, cb, b, tolerance, depth + 1, out, count, limit)
}

#[allow(clippy::too_many_arguments)]
fn flatten_cubic(
    a: [f64; 2],
    c1: [f64; 2],
    c2: [f64; 2],
    b: [f64; 2],
    tolerance: f64,
    depth: u8,
    out: &mut Vec<[f64; 2]>,
    count: &mut usize,
    limit: usize,
) -> Result<(), ShapeError> {
    let flat = distance_to_chord(c1, a, b).max(distance_to_chord(c2, a, b)) <= tolerance;
    if flat {
        if *count >= limit {
            return Err(ShapeError::TooManySegments);
        }
        out.push(b);
        *count += 1;
        return Ok(());
    }
    if depth >= MAX_PATH_DEPTH {
        return Err(ShapeError::SubdivisionLimit);
    }
    let a1 = mid(a, c1);
    let c12 = mid(c1, c2);
    let c2b = mid(c2, b);
    let left_mid = mid(a1, c12);
    let right_mid = mid(c12, c2b);
    let center = mid(left_mid, right_mid);
    flatten_cubic(
        a,
        a1,
        left_mid,
        center,
        tolerance,
        depth + 1,
        out,
        count,
        limit,
    )?;
    flatten_cubic(
        center,
        right_mid,
        c2b,
        b,
        tolerance,
        depth + 1,
        out,
        count,
        limit,
    )
}

/// A closed 2D surface. Rounded-rectangle radii are ordered top-left, top-right,
/// bottom-right, bottom-left. Polygon hit testing uses the even-odd rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape2D {
    Rect {
        bounds: Box2,
    },
    RoundedRect {
        bounds: Box2,
        radii: [f64; 4],
    },
    Circle {
        center: [f64; 2],
        radius: f64,
    },
    Ellipse {
        center: [f64; 2],
        radii: [f64; 2],
    },
    /// Axis-aligned superellipse: exponent 2 is an ellipse and exponent 4 is a squircle.
    Superellipse {
        center: [f64; 2],
        radii: [f64; 2],
        exponent: f64,
    },
    Polygon {
        vertices: Vec<[f64; 2]>,
    },
}

impl Shape2D {
    pub fn validate(&self) -> Result<(), ShapeError> {
        match self {
            Self::Rect { bounds } => validate_bounds(*bounds),
            Self::RoundedRect { bounds, radii } => {
                validate_bounds(*bounds)?;
                if radii.iter().any(|r| !r.is_finite() || *r < 0.0) {
                    return Err(ShapeError::InvalidRadius);
                }
                if radii[0] + radii[1] > bounds.w
                    || radii[3] + radii[2] > bounds.w
                    || radii[0] + radii[3] > bounds.h
                    || radii[1] + radii[2] > bounds.h
                {
                    return Err(ShapeError::InvalidRadius);
                }
                Ok(())
            }
            Self::Circle { center, radius } => {
                if !point_valid(*center)
                    || !radius.is_finite()
                    || *radius <= 0.0
                    || *radius > MAX_COORDINATE
                    || center[0].abs() + radius > MAX_COORDINATE
                    || center[1].abs() + radius > MAX_COORDINATE
                {
                    return Err(ShapeError::InvalidEllipse);
                }
                Ok(())
            }
            Self::Ellipse { center, radii } => {
                if !point_valid(*center)
                    || radii
                        .iter()
                        .any(|r| !r.is_finite() || *r <= 0.0 || *r > MAX_COORDINATE)
                    || center[0].abs() + radii[0] > MAX_COORDINATE
                    || center[1].abs() + radii[1] > MAX_COORDINATE
                {
                    return Err(ShapeError::InvalidEllipse);
                }
                Ok(())
            }
            Self::Superellipse {
                center,
                radii,
                exponent,
            } => {
                if !point_valid(*center)
                    || radii
                        .iter()
                        .any(|r| !r.is_finite() || *r <= 0.0 || *r > MAX_COORDINATE)
                    || center[0].abs() + radii[0] > MAX_COORDINATE
                    || center[1].abs() + radii[1] > MAX_COORDINATE
                {
                    return Err(ShapeError::InvalidEllipse);
                }
                if !exponent.is_finite()
                    || !(MIN_SUPERELLIPSE_EXPONENT..=MAX_SUPERELLIPSE_EXPONENT).contains(exponent)
                {
                    return Err(ShapeError::InvalidExponent);
                }
                Ok(())
            }
            Self::Polygon { vertices } => {
                if vertices.len() > MAX_POLYGON_VERTICES {
                    return Err(ShapeError::TooManyVertices);
                }
                if vertices.len() < 3 || vertices.iter().any(|point| !point_valid(*point)) {
                    return Err(ShapeError::InvalidPolygon);
                }
                let mut ordered = vertices.clone();
                ordered.sort_by(|a, b| a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1])));
                let unique = ordered.windows(2).all(|pair| pair[0] != pair[1]);
                let area2 = vertices
                    .iter()
                    .zip(vertices.iter().cycle().skip(1))
                    .take(vertices.len())
                    .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                    .sum::<f64>();
                if !unique || !area2.is_finite() || area2.abs() <= EDGE_EPSILON {
                    return Err(ShapeError::InvalidPolygon);
                }
                Ok(())
            }
        }
    }

    /// Returns a conservative axis-aligned bound. Call `validate` first when
    /// shapes can come from untrusted or persisted input.
    pub fn bounds(&self) -> Result<Box2, ShapeError> {
        self.validate()?;
        match self {
            Self::Rect { bounds } | Self::RoundedRect { bounds, .. } => Ok(*bounds),
            Self::Circle { center, radius } => Ok(Box2 {
                x: center[0] - radius,
                y: center[1] - radius,
                w: radius * 2.0,
                h: radius * 2.0,
            }),
            Self::Ellipse { center, radii } => Ok(Box2 {
                x: center[0] - radii[0],
                y: center[1] - radii[1],
                w: radii[0] * 2.0,
                h: radii[1] * 2.0,
            }),
            Self::Superellipse { center, radii, .. } => Ok(Box2 {
                x: center[0] - radii[0],
                y: center[1] - radii[1],
                w: radii[0] * 2.0,
                h: radii[1] * 2.0,
            }),
            Self::Polygon { vertices } => {
                let (mut min_x, mut max_x, mut min_y, mut max_y) = (
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                );
                for [x, y] in vertices {
                    min_x = min_x.min(*x);
                    max_x = max_x.max(*x);
                    min_y = min_y.min(*y);
                    max_y = max_y.max(*y);
                }
                Ok(Box2 {
                    x: min_x,
                    y: min_y,
                    w: max_x - min_x,
                    h: max_y - min_y,
                })
            }
        }
    }

    /// Boundary points count as hits. Invalid shapes and nonfinite query points
    /// return a typed error instead of producing a misleading hit result.
    pub fn contains(&self, point: [f64; 2]) -> Result<bool, ShapeError> {
        self.validate()?;
        self.contains_validated(point)
    }

    /// Linearly interpolate matching primitive geometry without changing
    /// canonical layout state. Polygon contours must have equal vertex counts;
    /// any invalid intermediate geometry is rejected after interpolation.
    pub fn interpolate(&self, target: &Self, amount: f64) -> Result<Self, ShapeError> {
        self.validate()?;
        target.validate()?;
        if !amount.is_finite() || !(0.0..=1.0).contains(&amount) {
            return Err(ShapeError::InvalidInterpolation);
        }
        let mix = |a: f64, b: f64| a + (b - a) * amount;
        let mix_point = |a: [f64; 2], b: [f64; 2]| [mix(a[0], b[0]), mix(a[1], b[1])];
        let result = match (self, target) {
            (Self::Rect { bounds: a }, Self::Rect { bounds: b }) => Self::Rect {
                bounds: mix_bounds(*a, *b, &mix),
            },
            (
                Self::RoundedRect {
                    bounds: a_bounds,
                    radii: a_radii,
                },
                Self::RoundedRect {
                    bounds: b_bounds,
                    radii: b_radii,
                },
            ) => Self::RoundedRect {
                bounds: mix_bounds(*a_bounds, *b_bounds, &mix),
                radii: std::array::from_fn(|index| mix(a_radii[index], b_radii[index])),
            },
            (
                Self::Circle {
                    center: a_center,
                    radius: a_radius,
                },
                Self::Circle {
                    center: b_center,
                    radius: b_radius,
                },
            ) => Self::Circle {
                center: mix_point(*a_center, *b_center),
                radius: mix(*a_radius, *b_radius),
            },
            (
                Self::Ellipse {
                    center: a_center,
                    radii: a_radii,
                },
                Self::Ellipse {
                    center: b_center,
                    radii: b_radii,
                },
            ) => Self::Ellipse {
                center: mix_point(*a_center, *b_center),
                radii: mix_point(*a_radii, *b_radii),
            },
            (
                Self::Superellipse {
                    center: a_center,
                    radii: a_radii,
                    exponent: a_exponent,
                },
                Self::Superellipse {
                    center: b_center,
                    radii: b_radii,
                    exponent: b_exponent,
                },
            ) => Self::Superellipse {
                center: mix_point(*a_center, *b_center),
                radii: mix_point(*a_radii, *b_radii),
                exponent: mix(*a_exponent, *b_exponent),
            },
            (Self::Polygon { vertices: a }, Self::Polygon { vertices: b })
                if a.len() == b.len() =>
            {
                Self::Polygon {
                    vertices: a.iter().zip(b).map(|(a, b)| mix_point(*a, *b)).collect(),
                }
            }
            _ => return Err(ShapeError::IncompatibleGeometry),
        };
        result.validate()?;
        Ok(result)
    }

    fn contains_validated(&self, point: [f64; 2]) -> Result<bool, ShapeError> {
        if !point_valid(point) {
            return Err(ShapeError::InvalidPoint);
        }
        let [x, y] = point;
        Ok(match self {
            Self::Rect { bounds } => point_in_box(*bounds, x, y),
            Self::RoundedRect { bounds, radii } => rounded_rect_contains(*bounds, *radii, x, y),
            Self::Circle { center, radius } => {
                squared_distance(point, *center) <= radius * radius + EDGE_EPSILON
            }
            Self::Ellipse { center, radii } => {
                let dx = (x - center[0]) / radii[0];
                let dy = (y - center[1]) / radii[1];
                dx * dx + dy * dy <= 1.0 + EDGE_EPSILON
            }
            Self::Superellipse {
                center,
                radii,
                exponent,
            } => {
                let dx = ((x - center[0]) / radii[0]).abs().powf(*exponent);
                let dy = ((y - center[1]) / radii[1]).abs().powf(*exponent);
                dx + dy <= 1.0 + EDGE_EPSILON
            }
            Self::Polygon { vertices } => polygon_contains(vertices, point),
        })
    }
}

/// Immutable validated shape for repeated pointer hit tests. Construct once
/// when accepting a shape, then reuse without repeating structural validation.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedShape2D {
    shape: Shape2D,
    bounds: Box2,
}

impl ValidatedShape2D {
    pub fn new(shape: Shape2D) -> Result<Self, ShapeError> {
        let bounds = shape.bounds()?;
        Ok(Self { shape, bounds })
    }

    pub fn shape(&self) -> &Shape2D {
        &self.shape
    }

    pub fn bounds(&self) -> Box2 {
        self.bounds
    }

    pub fn contains(&self, point: [f64; 2]) -> Result<bool, ShapeError> {
        self.shape.contains_validated(point)
    }
}

fn validate_bounds(bounds: Box2) -> Result<(), ShapeError> {
    if ![bounds.x, bounds.y, bounds.w, bounds.h]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE)
        || bounds.w < 0.0
        || bounds.h < 0.0
        || (bounds.x + bounds.w).abs() > MAX_COORDINATE
        || (bounds.y + bounds.h).abs() > MAX_COORDINATE
    {
        return Err(ShapeError::InvalidBounds);
    }
    Ok(())
}

fn mix_bounds(a: Box2, b: Box2, mix: &impl Fn(f64, f64) -> f64) -> Box2 {
    Box2 {
        x: mix(a.x, b.x),
        y: mix(a.y, b.y),
        w: mix(a.w, b.w),
        h: mix(a.h, b.h),
    }
}

fn point_valid([x, y]: [f64; 2]) -> bool {
    x.is_finite() && y.is_finite() && x.abs() <= MAX_COORDINATE && y.abs() <= MAX_COORDINATE
}

fn point_in_box(bounds: Box2, x: f64, y: f64) -> bool {
    x >= bounds.x && y >= bounds.y && x <= bounds.x + bounds.w && y <= bounds.y + bounds.h
}

fn squared_distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

fn rounded_rect_contains(bounds: Box2, radii: [f64; 4], x: f64, y: f64) -> bool {
    if !point_in_box(bounds, x, y) {
        return false;
    }
    let corners = [
        (
            bounds.x + radii[0],
            bounds.y + radii[0],
            radii[0],
            x < bounds.x + radii[0] && y < bounds.y + radii[0],
        ),
        (
            bounds.x + bounds.w - radii[1],
            bounds.y + radii[1],
            radii[1],
            x > bounds.x + bounds.w - radii[1] && y < bounds.y + radii[1],
        ),
        (
            bounds.x + bounds.w - radii[2],
            bounds.y + bounds.h - radii[2],
            radii[2],
            x > bounds.x + bounds.w - radii[2] && y > bounds.y + bounds.h - radii[2],
        ),
        (
            bounds.x + radii[3],
            bounds.y + bounds.h - radii[3],
            radii[3],
            x < bounds.x + radii[3] && y > bounds.y + bounds.h - radii[3],
        ),
    ];
    corners.iter().all(|(cx, cy, radius, active)| {
        !active
            || *radius == 0.0
            || squared_distance([x, y], [*cx, *cy]) <= radius * radius + EDGE_EPSILON
    })
}

fn polygon_contains(vertices: &[[f64; 2]], point: [f64; 2]) -> bool {
    let mut inside = false;
    for (a, b) in vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .take(vertices.len())
    {
        if point_on_segment(point, *a, *b) {
            return true;
        }
        if (a[1] > point[1]) != (b[1] > point[1]) {
            let crossing_x = (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0];
            if point[0] < crossing_x {
                inside = !inside;
            }
        }
    }
    inside
}

fn point_on_segment(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let px = point[0] - a[0];
    let py = point[1] - a[1];
    let cross = px * dy - py * dx;
    let roundoff = f64::EPSILON * 8.0 * (1.0 + px.abs() * dy.abs() + py.abs() * dx.abs());
    if cross.abs() > EDGE_EPSILON.max(roundoff) {
        return false;
    }
    let coordinate_epsilon = EDGE_EPSILON.max(roundoff / dx.abs().max(dy.abs()).max(1.0));
    point[0] >= a[0].min(b[0]) - coordinate_epsilon
        && point[0] <= a[0].max(b[0]) + coordinate_epsilon
        && point[1] >= a[1].min(b[1]) - coordinate_epsilon
        && point[1] <= a[1].max(b[1]) + coordinate_epsilon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_validate_boundaries_and_hit_tests() {
        let rect = Shape2D::Rect {
            bounds: Box2 {
                x: 0.0,
                y: 0.0,
                w: 20.0,
                h: 10.0,
            },
        };
        assert!(rect.contains([20.0, 10.0]).unwrap());
        assert!(!rect.contains([20.1, 10.0]).unwrap());

        let rounded = Shape2D::RoundedRect {
            bounds: Box2 {
                x: 0.0,
                y: 0.0,
                w: 20.0,
                h: 10.0,
            },
            radii: [4.0; 4],
        };
        assert!(!rounded.contains([0.0, 0.0]).unwrap());
        assert!(rounded.contains([4.0, 0.0]).unwrap());

        let circle = Shape2D::Circle {
            center: [3.0, 4.0],
            radius: 5.0,
        };
        assert!(circle.contains([6.0, 8.0]).unwrap());
        assert!(!circle.contains([6.1, 8.0]).unwrap());

        let ellipse = Shape2D::Ellipse {
            center: [0.0, 0.0],
            radii: [4.0, 2.0],
        };
        assert!(ellipse.contains([4.0, 0.0]).unwrap());
        assert!(!ellipse.contains([0.0, 2.1]).unwrap());

        let squircle = Shape2D::Superellipse {
            center: [0.0, 0.0],
            radii: [10.0, 10.0],
            exponent: 4.0,
        };
        assert!(squircle.contains([8.0, 8.0]).unwrap());
        assert!(!squircle.contains([9.0, 9.0]).unwrap());
        assert_eq!(
            squircle.bounds().unwrap(),
            Box2 {
                x: -10.0,
                y: -10.0,
                w: 20.0,
                h: 20.0,
            }
        );
        assert_eq!(
            Shape2D::Superellipse {
                center: [0.0, 0.0],
                radii: [1.0, 1.0],
                exponent: 0.5,
            }
            .validate(),
            Err(ShapeError::InvalidExponent)
        );

        let polygon = Shape2D::Polygon {
            vertices: vec![[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]],
        };
        assert!(polygon.contains([5.0, 3.0]).unwrap());
        assert!(polygon.contains([5.0, 8.0]).unwrap());
        assert!(!polygon.contains([9.0, 7.0]).unwrap());
        assert_eq!(
            polygon.bounds().unwrap(),
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 8.0
            }
        );

        let large_polygon = Shape2D::Polygon {
            vertices: vec![
                [900_000.0, 900_000.0],
                [950_000.0, 900_000.0],
                [950_000.0, 950_000.0],
                [900_000.0, 950_000.0],
            ],
        };
        assert!(large_polygon.contains([925_000.0, 900_000.0]).unwrap());
    }

    #[test]
    fn malformed_shapes_and_nonfinite_queries_fail_closed() {
        assert_eq!(
            Shape2D::Circle {
                center: [0.0, 0.0],
                radius: 0.0
            }
            .validate(),
            Err(ShapeError::InvalidEllipse)
        );
        assert_eq!(
            Shape2D::RoundedRect {
                bounds: Box2 {
                    x: 0.0,
                    y: 0.0,
                    w: 4.0,
                    h: 4.0
                },
                radii: [3.0; 4]
            }
            .validate(),
            Err(ShapeError::InvalidRadius)
        );
        assert_eq!(
            Shape2D::Rect {
                bounds: Box2 {
                    x: MAX_COORDINATE,
                    y: 0.0,
                    w: 1.0,
                    h: 1.0,
                }
            }
            .validate(),
            Err(ShapeError::InvalidBounds)
        );
        assert_eq!(
            Shape2D::Polygon {
                vertices: vec![[0.0, 0.0]; MAX_POLYGON_VERTICES + 1]
            }
            .validate(),
            Err(ShapeError::TooManyVertices)
        );
        assert_eq!(
            Shape2D::Polygon {
                vertices: vec![[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]
            }
            .validate(),
            Err(ShapeError::InvalidPolygon)
        );
        let rect = Shape2D::Rect {
            bounds: Box2 {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
        };
        assert_eq!(
            rect.contains([f64::NAN, 0.0]),
            Err(ShapeError::InvalidPoint)
        );
    }

    #[test]
    fn shape_interpolation_is_bounded_variant_safe_and_validates_results() {
        let start = Shape2D::Rect {
            bounds: Box2 {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 20.0,
            },
        };
        let target = Shape2D::Rect {
            bounds: Box2 {
                x: 20.0,
                y: 10.0,
                w: 30.0,
                h: 40.0,
            },
        };
        assert_eq!(start.interpolate(&target, 0.0).unwrap(), start);
        assert_eq!(start.interpolate(&target, 1.0).unwrap(), target);
        assert_eq!(
            start.interpolate(&target, 0.5).unwrap(),
            Shape2D::Rect {
                bounds: Box2 {
                    x: 10.0,
                    y: 5.0,
                    w: 20.0,
                    h: 30.0,
                }
            }
        );
        assert_eq!(
            start.interpolate(&target, f64::NAN),
            Err(ShapeError::InvalidInterpolation)
        );
        assert_eq!(
            start.interpolate(
                &Shape2D::Circle {
                    center: [0.0, 0.0],
                    radius: 1.0,
                },
                0.5,
            ),
            Err(ShapeError::IncompatibleGeometry)
        );

        let triangle = Shape2D::Polygon {
            vertices: vec![[0.0, 0.0], [2.0, 0.0], [0.0, 2.0]],
        };
        let reversed = Shape2D::Polygon {
            vertices: vec![[0.0, 0.0], [0.0, 2.0], [2.0, 0.0]],
        };
        assert_eq!(
            triangle.interpolate(&reversed, 0.5),
            Err(ShapeError::InvalidPolygon)
        );
    }

    #[test]
    fn validated_shape_reuses_checked_geometry_and_shape_json_rejects_unknown_fields() {
        let shape = Shape2D::Polygon {
            vertices: vec![[0.0, 0.0], [8.0, 0.0], [4.0, 6.0]],
        };
        let validated = ValidatedShape2D::new(shape.clone()).unwrap();
        assert_eq!(validated.shape(), &shape);
        assert_eq!(validated.bounds().w, 8.0);
        assert!(validated.contains([4.0, 2.0]).unwrap());
        assert!(
            serde_json::from_str::<Shape2D>(
                r#"{"shape":"circle","center":[0,0],"radius":4,"future":true}"#
            )
            .is_err()
        );
    }

    #[test]
    fn bezier_path_flattens_with_bounded_error_and_compound_even_odd_fill() {
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::CubicTo {
                    control1: [14.0, 0.0],
                    control2: [14.0, 10.0],
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        };
        let flattened = path.flatten(0.1, 64).unwrap();
        assert!(flattened.contours()[0].len() > 5);
        assert_eq!(flattened.bounds().x, 0.0);
        assert_eq!(flattened.bounds().w, 14.0);
        assert!(flattened.contains([5.0, 5.0]).unwrap());
        assert!(flattened.contains([0.0, 0.0]).unwrap());
        assert!(!flattened.contains([15.0, 5.0]).unwrap());

        let compound = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo { point: [3.0, 3.0] },
                PathCommand2D::LineTo { point: [7.0, 3.0] },
                PathCommand2D::LineTo { point: [7.0, 7.0] },
                PathCommand2D::LineTo { point: [3.0, 7.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(1.0, 32)
        .unwrap();
        assert!(compound.contains([1.0, 1.0]).unwrap());
        assert!(!compound.contains([5.0, 5.0]).unwrap());

        let quadratic = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::QuadraticTo {
                    control: [5.0, -5.0],
                    point: [10.0, 0.0],
                },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.1, 64)
        .unwrap();
        assert!(quadratic.bounds().y < 0.0);
        assert!(quadratic.contains([5.0, 4.0]).unwrap());
    }

    #[test]
    fn bezier_path_rejects_open_invalid_or_over_budget_paths() {
        let open = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [1.0, 0.0] },
                PathCommand2D::LineTo { point: [0.0, 1.0] },
            ],
        };
        assert_eq!(open.flatten(1.0, 32).unwrap_err(), ShapeError::InvalidPath);
        assert_eq!(
            open.flatten(f64::NAN, 32).unwrap_err(),
            ShapeError::InvalidTolerance
        );

        let closed = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        };
        assert_eq!(
            closed.flatten(1.0, 2).unwrap_err(),
            ShapeError::TooManySegments
        );
    }

    #[test]
    fn path_interpolation_requires_matching_commands_and_valid_midpoint_topology() {
        fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Path2D {
            Path2D {
                commands: vec![
                    PathCommand2D::MoveTo { point: [x, y] },
                    PathCommand2D::LineTo { point: [x + w, y] },
                    PathCommand2D::LineTo {
                        point: [x + w, y + h],
                    },
                    PathCommand2D::LineTo { point: [x, y + h] },
                    PathCommand2D::Close,
                ],
            }
        }

        let from = rectangle(0.0, 0.0, 10.0, 10.0);
        let to = rectangle(10.0, 0.0, 10.0, 20.0);
        assert_eq!(from.interpolate(&to, 0.0, 0.5, 64).unwrap(), from);
        assert_eq!(from.interpolate(&to, 1.0, 0.5, 64).unwrap(), to);
        let middle = from.interpolate(&to, 0.5, 0.5, 64).unwrap();
        assert!(
            middle
                .flatten(0.5, 64)
                .unwrap()
                .contains([10.0, 5.0])
                .unwrap()
        );
        assert_eq!(
            from.interpolate(&to, f64::NAN, 0.5, 64),
            Err(ShapeError::InvalidInterpolation)
        );
        assert_eq!(
            from.interpolate(&to, 0.5, 0.5, 2),
            Err(ShapeError::TooManySegments)
        );

        let mut incompatible = to.clone();
        incompatible.commands[1] = PathCommand2D::QuadraticTo {
            control: [20.0, 0.0],
            point: [20.0, 0.0],
        };
        assert_eq!(
            from.interpolate(&incompatible, 0.5, 0.5, 64),
            Err(ShapeError::IncompatibleGeometry)
        );

        let reversed = rectangle(0.0, 0.0, 10.0, 10.0);
        let mut reverse_winding = reversed.clone();
        reverse_winding.commands = vec![
            PathCommand2D::MoveTo { point: [0.0, 0.0] },
            PathCommand2D::LineTo { point: [0.0, 10.0] },
            PathCommand2D::LineTo {
                point: [10.0, 10.0],
            },
            PathCommand2D::LineTo { point: [10.0, 0.0] },
            PathCommand2D::Close,
        ];
        assert_eq!(
            reversed.interpolate(&reverse_winding, 0.5, 0.5, 64),
            Err(ShapeError::InvalidPath)
        );

        let quadratic_path = |x: f64, control_y: f64| Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [x, 0.0] },
                PathCommand2D::QuadraticTo {
                    control: [x + 5.0, control_y],
                    point: [x + 10.0, 0.0],
                },
                PathCommand2D::LineTo {
                    point: [x + 10.0, -10.0],
                },
                PathCommand2D::LineTo { point: [x, -10.0] },
                PathCommand2D::Close,
            ],
        };
        let quadratic = quadratic_path(0.0, 10.0)
            .interpolate(&quadratic_path(10.0, 20.0), 0.5, 0.25, 256)
            .unwrap();
        assert_eq!(
            quadratic.commands[1],
            PathCommand2D::QuadraticTo {
                control: [10.0, 15.0],
                point: [15.0, 0.0],
            }
        );

        let cubic_path = |x: f64, control_y: f64| Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [x, 0.0] },
                PathCommand2D::CubicTo {
                    control1: [x + 2.5, control_y],
                    control2: [x + 7.5, control_y],
                    point: [x + 10.0, 0.0],
                },
                PathCommand2D::LineTo {
                    point: [x + 10.0, -10.0],
                },
                PathCommand2D::LineTo { point: [x, -10.0] },
                PathCommand2D::Close,
            ],
        };
        let cubic = cubic_path(0.0, 10.0)
            .interpolate(&cubic_path(10.0, 20.0), 0.5, 0.25, 256)
            .unwrap();
        assert_eq!(
            cubic.commands[1],
            PathCommand2D::CubicTo {
                control1: [7.5, 15.0],
                control2: [12.5, 15.0],
                point: [15.0, 0.0],
            }
        );
    }

    #[test]
    fn compound_path_boolean_operations_are_even_odd_bounded_and_deterministic() {
        fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Path2D {
            Path2D {
                commands: vec![
                    PathCommand2D::MoveTo { point: [x, y] },
                    PathCommand2D::LineTo { point: [x + w, y] },
                    PathCommand2D::LineTo {
                        point: [x + w, y + h],
                    },
                    PathCommand2D::LineTo { point: [x, y + h] },
                    PathCommand2D::Close,
                ],
            }
        }

        let left = rectangle(0.0, 0.0, 10.0, 10.0);
        let right = rectangle(5.0, 5.0, 10.0, 10.0);
        let union = left
            .boolean(&right, PathBooleanOp2D::Union, 0.5, 64)
            .unwrap();
        let intersection = left
            .boolean(&right, PathBooleanOp2D::Intersection, 0.5, 64)
            .unwrap();
        let difference = left
            .boolean(&right, PathBooleanOp2D::Difference, 0.5, 64)
            .unwrap();
        let exclusive_or = left
            .boolean(&right, PathBooleanOp2D::ExclusiveOr, 0.5, 64)
            .unwrap();

        assert!(union.contains([2.0, 2.0]).unwrap());
        assert!(union.contains([7.0, 7.0]).unwrap());
        assert!(union.contains([12.0, 12.0]).unwrap());
        assert!(!union.contains([20.0, 20.0]).unwrap());
        assert!(intersection.contains([7.0, 7.0]).unwrap());
        assert!(!intersection.contains([2.0, 2.0]).unwrap());
        assert!(!intersection.contains([12.0, 12.0]).unwrap());
        assert!(difference.contains([2.0, 2.0]).unwrap());
        assert!(!difference.contains([7.0, 7.0]).unwrap());
        assert!(exclusive_or.contains([2.0, 2.0]).unwrap());
        assert!(!exclusive_or.contains([7.0, 7.0]).unwrap());
        assert!(exclusive_or.contains([12.0, 12.0]).unwrap());
        assert_eq!(
            left.boolean(&right, PathBooleanOp2D::Union, 0.5, 64)
                .unwrap(),
            union,
            "identical bounded inputs produce stable contour ordering"
        );
        for contour in union.contours() {
            assert_eq!(contour.first(), contour.last());
        }

        let empty = rectangle(30.0, 30.0, 2.0, 2.0);
        let no_overlap = left
            .boolean(&empty, PathBooleanOp2D::Intersection, 0.5, 64)
            .unwrap();
        assert!(no_overlap.contours().is_empty());
        assert!(!no_overlap.contains([1.0, 1.0]).unwrap());
        assert_eq!(no_overlap.bounds(), Box2::default());
    }

    #[test]
    fn compound_path_offset_and_smoothing_are_bounded_and_revalidated() {
        let square = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();

        let expanded = square.offset(2.0).unwrap();
        assert_eq!(
            expanded.bounds(),
            Box2 {
                x: -2.0,
                y: -2.0,
                w: 14.0,
                h: 14.0,
            }
        );
        assert!(expanded.contains([-1.0, 5.0]).unwrap());
        assert!(square.offset(f64::NAN).is_err());
        assert!(square.offset(MAX_COORDINATE + 1.0).is_err());

        let contracted = square.offset(-2.0).unwrap();
        assert!(contracted.contains([5.0, 5.0]).unwrap());
        assert!(!contracted.contains([1.0, 5.0]).unwrap());
        assert!(square.offset(-6.0).unwrap().contours().is_empty());

        let smoothed = square.smoothed(1).unwrap();
        assert_eq!(smoothed.contours()[0].len(), 9);
        assert!(smoothed.contains([5.0, 5.0]).unwrap());
        assert!(!smoothed.contains([0.0, 0.0]).unwrap());
        assert_eq!(square.smoothed(0).unwrap(), square);
        assert_eq!(
            square.smoothed(MAX_PATH_SMOOTHING_ITERATIONS + 1),
            Err(ShapeError::TooManySegments)
        );

        let donut = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [20.0, 20.0],
                },
                PathCommand2D::LineTo { point: [0.0, 20.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo { point: [5.0, 5.0] },
                PathCommand2D::LineTo { point: [15.0, 5.0] },
                PathCommand2D::LineTo {
                    point: [15.0, 15.0],
                },
                PathCommand2D::LineTo { point: [5.0, 15.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 64)
        .unwrap();
        let expanded_donut = donut.offset(2.0).unwrap();
        assert!(expanded_donut.contains([6.0, 10.0]).unwrap());
        assert!(!expanded_donut.contains([10.0, 10.0]).unwrap());
        let smooth_donut = donut.smoothed(1).unwrap();
        assert!(smooth_donut.contains([2.0, 10.0]).unwrap());
        assert!(!smooth_donut.contains([10.0, 10.0]).unwrap());

        let mut contour: Vec<_> = (0..1024)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / 1024.0;
                [100.0 + angle.cos() * 10.0, 100.0 + angle.sin() * 10.0]
            })
            .collect();
        contour.push(contour[0]);
        let many_vertices = FlattenedPath2D {
            contours: vec![contour],
            bounds: Box2 {
                x: 90.0,
                y: 90.0,
                w: 20.0,
                h: 20.0,
            },
        };
        assert_eq!(
            many_vertices.smoothed(2),
            Err(ShapeError::TooManySegments),
            "predicted output over the vertex cap is rejected before smoothing allocation"
        );
    }

    #[test]
    fn compound_path_fill_tessellation_preserves_holes_and_is_bounded() {
        fn area(triangle: [[f64; 2]; 3]) -> f64 {
            triangle_area(triangle).abs()
        }

        let square = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();
        let triangles = square.tessellate().unwrap();
        assert_eq!(triangles.len(), 2);
        assert!((triangles.iter().copied().map(area).sum::<f64>() - 100.0).abs() < 1.0e-9);
        assert_eq!(triangles, square.tessellate().unwrap());

        let donut = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo { point: [2.0, 2.0] },
                PathCommand2D::LineTo { point: [8.0, 2.0] },
                PathCommand2D::LineTo { point: [8.0, 8.0] },
                PathCommand2D::LineTo { point: [2.0, 8.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();
        let triangles = donut.tessellate().unwrap();
        assert!(triangles.len() >= 8);
        assert!((triangles.iter().copied().map(area).sum::<f64>() - 64.0).abs() < 1.0e-9);
        for triangle in triangles {
            let centroid = [
                (triangle[0][0] + triangle[1][0] + triangle[2][0]) / 3.0,
                (triangle[0][1] + triangle[1][1] + triangle[2][1]) / 3.0,
            ];
            assert!(
                !(2.0 < centroid[0] && centroid[0] < 8.0 && 2.0 < centroid[1] && centroid[1] < 8.0)
            );
        }

        let empty = square
            .boolean(&square, PathBooleanOp2D::Difference)
            .unwrap();
        assert!(empty.tessellate().unwrap().is_empty());
    }

    #[test]
    fn flattened_path_transform_preserves_hits_and_rejects_singular_geometry() {
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();
        let moved = path
            .transformed(Affine2::translate(7.0, -3.0).unwrap())
            .unwrap();
        assert!(moved.contains([12.0, 2.0]).unwrap());
        assert!(!moved.contains([2.0, 2.0]).unwrap());
        assert_eq!(
            moved.bounds(),
            Box2 {
                x: 7.0,
                y: -3.0,
                w: 10.0,
                h: 10.0
            }
        );

        let singular = Affine2::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(
            path.transformed(singular),
            Err(ShapeError::InvalidTransform)
        );
    }

    #[test]
    fn compound_path_boolean_preserves_even_odd_holes_and_enforces_work_bound() {
        fn rectangle_commands(x: f64, y: f64, size: f64) -> Vec<PathCommand2D> {
            vec![
                PathCommand2D::MoveTo { point: [x, y] },
                PathCommand2D::LineTo {
                    point: [x + size, y],
                },
                PathCommand2D::LineTo {
                    point: [x + size, y + size],
                },
                PathCommand2D::LineTo {
                    point: [x, y + size],
                },
                PathCommand2D::Close,
            ]
        }

        let mut donut_commands = rectangle_commands(0.0, 0.0, 20.0);
        donut_commands.extend(rectangle_commands(5.0, 5.0, 10.0));
        let donut = Path2D {
            commands: donut_commands,
        };
        let clip = Path2D {
            commands: rectangle_commands(-1.0, -1.0, 22.0),
        };
        let clipped = donut
            .boolean(&clip, PathBooleanOp2D::Intersection, 0.5, 64)
            .unwrap();
        assert!(clipped.contains([2.0, 2.0]).unwrap());
        assert!(!clipped.contains([10.0, 10.0]).unwrap());

        let crossing_contours = Path2D {
            commands: {
                let mut commands = rectangle_commands(0.0, 0.0, 10.0);
                commands.extend(rectangle_commands(5.0, 5.0, 10.0));
                commands
            },
        };
        assert_eq!(
            crossing_contours.boolean(&clip, PathBooleanOp2D::Union, 0.5, 64),
            Err(ShapeError::InvalidPath)
        );

        let self_crossing = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [10.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::LineTo { point: [10.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [12.0, 15.0],
                },
                PathCommand2D::Close,
            ],
        };
        assert_eq!(
            self_crossing.boolean(&clip, PathBooleanOp2D::Union, 0.5, 64),
            Err(ShapeError::InvalidPath)
        );

        let oversized = Path2D {
            commands: vec![PathCommand2D::MoveTo { point: [0.0, 0.0] }]
                .into_iter()
                .chain((0..MAX_BOOLEAN_VERTICES).map(|index| {
                    let angle = index as f64 * std::f64::consts::TAU / MAX_BOOLEAN_VERTICES as f64;
                    PathCommand2D::LineTo {
                        point: [100.0 + angle.cos(), 100.0 + angle.sin()],
                    }
                }))
                .chain(std::iter::once(PathCommand2D::Close))
                .collect(),
        };
        assert_eq!(
            oversized.boolean(&clip, PathBooleanOp2D::Union, 0.5, 8_192),
            Err(ShapeError::TooManySegments)
        );
    }
}
