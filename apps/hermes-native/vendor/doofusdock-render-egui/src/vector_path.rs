//! Bounded solid-fill painting for toolkit-neutral compound paths.
use doofusdock_core::{DockError, DockResult, ErrorCode, PanelId};
use doofusdock_presentation::{FlattenedPath2D, PathBooleanOp2D, ShapeError};
use doofusdock_presentation::{
    Path2D, PathCommand2D, SceneFrame, ScenePoint, SceneSurfaceGeometry,
};
use egui::{Color32, Mesh, Painter, epaint::Vertex, pos2};
use std::collections::BTreeMap;

const MAX_SCENE_CLIP_SEGMENTS: usize = 2048;
const MAX_PATH_STROKE_WIDTH: f64 = 4096.0;

/// Stroke every closed contour in a bounded flattened path. The path remains
/// presentation geometry; egui's active painter clip controls viewport clipping.
/// Returns the number of contours submitted. Width must be finite and in
/// `0..=4096`; zero width and empty paths are no-ops.
pub fn paint_path_stroke(
    painter: &Painter,
    path: &FlattenedPath2D,
    width: f64,
    color: Color32,
) -> Result<usize, ShapeError> {
    if !width.is_finite() || !(0.0..=MAX_PATH_STROKE_WIDTH).contains(&width) {
        return Err(ShapeError::InvalidStrokeWidth);
    }
    if width == 0.0 {
        return Ok(0);
    }
    let vertex_count = path.contours().iter().map(Vec::len).sum::<usize>();
    if vertex_count > MAX_SCENE_CLIP_SEGMENTS {
        return Err(ShapeError::TooManySegments);
    }
    let mut rendered = 0;
    for contour in path.contours() {
        if contour.len() < 3 {
            return Err(ShapeError::InvalidPath);
        }
        let points = contour
            .iter()
            .map(|[x, y]| pos2(*x as f32, *y as f32))
            .collect();
        painter.add(egui::Shape::Path(egui::epaint::PathShape::closed_line(
            points,
            egui::Stroke::new(width as f32, color),
        )));
        rendered += 1;
    }
    Ok(rendered)
}

/// Paint one even-odd path fill using bounded presentation tessellation.
/// Painter clipping remains controlled by egui's current clip rectangle.
/// Returns the number of triangles submitted; an empty path submits no shape.
pub fn paint_path_fill(
    painter: &Painter,
    path: &FlattenedPath2D,
    color: Color32,
) -> Result<usize, ShapeError> {
    let triangles = path.tessellate()?;
    let count = triangles.len();
    if count == 0 {
        return Ok(0);
    }
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(count.saturating_mul(3));
    mesh.indices.reserve(count.saturating_mul(3));
    for triangle in triangles {
        let base = u32::try_from(mesh.vertices.len()).map_err(|_| ShapeError::TooManySegments)?;
        for [x, y] in triangle {
            mesh.vertices.push(Vertex {
                pos: pos2(x as f32, y as f32),
                uv: pos2(0.0, 0.0),
                color,
            });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    painter.add(egui::Shape::mesh(mesh));
    Ok(count)
}

/// Paint a bounded even-odd path using a spatial linear color gradient.
/// Colors are interpolated per mesh vertex; endpoints must be finite and
/// distinct. Painter clipping remains controlled by egui's current clip rect.
pub fn paint_path_fill_linear_gradient(
    painter: &Painter,
    path: &FlattenedPath2D,
    start: [f64; 2],
    end: [f64; 2],
    start_color: Color32,
    end_color: Color32,
) -> Result<usize, ShapeError> {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let length_squared = dx * dx + dy * dy;
    if !start.into_iter().chain(end).all(f64::is_finite)
        || !length_squared.is_finite()
        || length_squared <= f64::EPSILON
    {
        return Err(ShapeError::InvalidGradient);
    }
    let triangles = path.tessellate()?;
    let count = triangles.len();
    if count == 0 {
        return Ok(0);
    }
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(count.saturating_mul(3));
    mesh.indices.reserve(count.saturating_mul(3));
    for triangle in triangles {
        let base = u32::try_from(mesh.vertices.len()).map_err(|_| ShapeError::TooManySegments)?;
        for [x, y] in triangle {
            let amount =
                (((x - start[0]) * dx + (y - start[1]) * dy) / length_squared).clamp(0.0, 1.0);
            mesh.vertices.push(Vertex {
                pos: pos2(x as f32, y as f32),
                uv: pos2(0.0, 0.0),
                color: interpolate_color(start_color, end_color, amount),
            });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    painter.add(egui::Shape::mesh(mesh));
    Ok(count)
}

fn interpolate_color(start: Color32, end: Color32, amount: f64) -> Color32 {
    let interpolate =
        |left: u8, right: u8| (left as f64 + (right as f64 - left as f64) * amount).round() as u8;
    Color32::from_rgba_premultiplied(
        interpolate(start.r(), end.r()),
        interpolate(start.g(), end.g()),
        interpolate(start.b(), end.b()),
        interpolate(start.a(), end.a()),
    )
}

/// Intersect a fill with a second validated path, then paint the clipped mesh.
/// Both paths use the presentation crate's bounded simple-ring topology rules.
pub fn paint_path_fill_clipped(
    painter: &Painter,
    path: &FlattenedPath2D,
    clip_path: &FlattenedPath2D,
    color: Color32,
) -> Result<usize, ShapeError> {
    let clipped = path.boolean(clip_path, PathBooleanOp2D::Intersection)?;
    paint_path_fill(painter, &clipped, color)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScenePathFillRenderReport {
    pub rendered: usize,
    pub clipped_away: usize,
}

/// A panel-local flattened path and linear-gradient paint description for
/// revision-bound scene rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePathLinearGradient {
    pub path: FlattenedPath2D,
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub start_color: Color32,
    pub end_color: Color32,
}

/// Paint panel-local path fills through a revision-bound scene frame. Paths
/// follow each panel's composed transform and are clipped to its visible
/// surface after inherited scene clips are applied.
pub fn paint_scene_path_fills(
    painter: &Painter,
    frame: &SceneFrame,
    paths: &BTreeMap<PanelId, FlattenedPath2D>,
    color: Color32,
    revision: u64,
    generation: u64,
) -> DockResult<ScenePathFillRenderReport> {
    for panel in paths.keys() {
        if !frame
            .elements()
            .iter()
            .any(|element| &element.panel == panel)
        {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Scene path fill targets a panel absent from the frame.",
                &panel.0,
            ));
        }
    }
    let geometry = frame.surface_geometry(revision, generation)?;
    let surfaces = geometry
        .iter()
        .map(|surface| (surface.panel.clone(), surface))
        .collect::<BTreeMap<_, _>>();
    let mut report = ScenePathFillRenderReport::default();
    for (panel, path) in paths {
        let element = frame
            .elements()
            .iter()
            .find(|element| &element.panel == panel)
            .expect("path panel was validated above");
        let Some(surface) = surfaces.get(panel).copied() else {
            report.clipped_away += 1;
            continue;
        };
        let transformed = path
            .transformed(element.world_transform)
            .map_err(scene_path_error)?;
        let clip_path = surface_path(surface).map_err(scene_path_error)?;
        let color = color.gamma_multiply(element.opacity as f32);
        let triangles = paint_path_fill_clipped(painter, &transformed, &clip_path, color)
            .map_err(scene_path_error)?;
        if triangles == 0 {
            report.clipped_away += 1;
        } else {
            report.rendered += 1;
        }
    }
    Ok(report)
}

/// Paint panel-local path gradients through a revision-bound scene frame.
/// Both path geometry and gradient endpoints follow the composed panel
/// transform; inherited clips and panel opacity apply to the resulting mesh.
pub fn paint_scene_path_linear_gradient_fills(
    painter: &Painter,
    frame: &SceneFrame,
    fills: &BTreeMap<PanelId, ScenePathLinearGradient>,
    revision: u64,
    generation: u64,
) -> DockResult<ScenePathFillRenderReport> {
    for panel in fills.keys() {
        if !frame
            .elements()
            .iter()
            .any(|element| &element.panel == panel)
        {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Scene path gradient targets a panel absent from the frame.",
                &panel.0,
            ));
        }
    }
    let geometry = frame.surface_geometry(revision, generation)?;
    let surfaces = geometry
        .iter()
        .map(|surface| (surface.panel.clone(), surface))
        .collect::<BTreeMap<_, _>>();
    let mut report = ScenePathFillRenderReport::default();
    for (panel, fill) in fills {
        let element = frame
            .elements()
            .iter()
            .find(|element| &element.panel == panel)
            .expect("gradient panel was validated above");
        let Some(surface) = surfaces.get(panel).copied() else {
            report.clipped_away += 1;
            continue;
        };
        let transformed = fill
            .path
            .transformed(element.world_transform)
            .map_err(scene_path_error)?;
        let clip_path = surface_path(surface).map_err(scene_path_error)?;
        let clipped = transformed
            .boolean(&clip_path, PathBooleanOp2D::Intersection)
            .map_err(scene_path_error)?;
        let transform_gradient_point = |[x, y]: [f64; 2]| {
            let point = element.world_transform.transform_point(ScenePoint { x, y });
            [point.x, point.y]
        };
        let start = transform_gradient_point(fill.start);
        let end = transform_gradient_point(fill.end);
        let start_color = fill.start_color.gamma_multiply(element.opacity as f32);
        let end_color = fill.end_color.gamma_multiply(element.opacity as f32);
        let triangles =
            paint_path_fill_linear_gradient(painter, &clipped, start, end, start_color, end_color)
                .map_err(scene_path_error)?;
        if triangles == 0 {
            report.clipped_away += 1;
        } else {
            report.rendered += 1;
        }
    }
    Ok(report)
}

/// Paint panel-local closed-contour strokes as bounded fill bands. Stroke width
/// is measured in panel-local units before the composed scene transform; the
/// visible surface clip and panel opacity apply to the resulting mesh.
pub fn paint_scene_path_strokes(
    painter: &Painter,
    frame: &SceneFrame,
    paths: &BTreeMap<PanelId, FlattenedPath2D>,
    width: f64,
    color: Color32,
    revision: u64,
    generation: u64,
) -> DockResult<ScenePathFillRenderReport> {
    if !width.is_finite() || !(0.0..=MAX_PATH_STROKE_WIDTH).contains(&width) {
        return Err(scene_path_error(ShapeError::InvalidStrokeWidth));
    }
    for panel in paths.keys() {
        if !frame
            .elements()
            .iter()
            .any(|element| &element.panel == panel)
        {
            return Err(DockError::new(
                ErrorCode::NotFound,
                "Scene path stroke targets a panel absent from the frame.",
                &panel.0,
            ));
        }
    }
    let geometry = frame.surface_geometry(revision, generation)?;
    let surfaces = geometry
        .iter()
        .map(|surface| (surface.panel.clone(), surface))
        .collect::<BTreeMap<_, _>>();
    let mut report = ScenePathFillRenderReport::default();
    for (panel, path) in paths {
        let element = frame
            .elements()
            .iter()
            .find(|element| &element.panel == panel)
            .expect("stroke panel was validated above");
        let Some(surface) = surfaces.get(panel).copied() else {
            report.clipped_away += 1;
            continue;
        };
        if width == 0.0 {
            continue;
        }
        let half_width = width / 2.0;
        let outer = path.offset(half_width).map_err(scene_path_error)?;
        let inner = path.offset(-half_width).map_err(scene_path_error)?;
        let stroke = outer
            .boolean(&inner, PathBooleanOp2D::Difference)
            .map_err(scene_path_error)?;
        let transformed = stroke
            .transformed(element.world_transform)
            .map_err(scene_path_error)?;
        let clip_path = surface_path(surface).map_err(scene_path_error)?;
        let color = color.gamma_multiply(element.opacity as f32);
        let triangles = paint_path_fill_clipped(painter, &transformed, &clip_path, color)
            .map_err(scene_path_error)?;
        if triangles == 0 {
            report.clipped_away += 1;
        } else {
            report.rendered += 1;
        }
    }
    Ok(report)
}

fn surface_path(surface: &SceneSurfaceGeometry) -> Result<FlattenedPath2D, ShapeError> {
    if surface.vertices.len() < 3 || surface.vertices.len() > MAX_SCENE_CLIP_SEGMENTS - 1 {
        return Err(ShapeError::TooManySegments);
    }
    let first = surface.vertices[0].position;
    let mut commands = Vec::with_capacity(surface.vertices.len() + 1);
    commands.push(PathCommand2D::MoveTo {
        point: [first.x, first.y],
    });
    commands.extend(
        surface
            .vertices
            .iter()
            .skip(1)
            .map(|vertex| PathCommand2D::LineTo {
                point: [vertex.position.x, vertex.position.y],
            }),
    );
    commands.push(PathCommand2D::Close);
    Path2D { commands }.flatten(0.5, MAX_SCENE_CLIP_SEGMENTS)
}

fn scene_path_error(error: ShapeError) -> DockError {
    DockError::new(
        ErrorCode::InvalidLayout,
        "Scene path fill geometry failed validation or exceeded its work limit.",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use doofusdock_presentation::{Path2D, PathCommand2D};

    #[test]
    fn path_stroke_paints_each_closed_contour_and_rejects_unbounded_width() {
        let path = Path2D {
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
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_path_stroke(ui.painter(), &path, 2.5, Color32::GREEN).unwrap(),
                2
            );
            assert_eq!(
                paint_path_stroke(ui.painter(), &path, 0.0, Color32::GREEN).unwrap(),
                0
            );
            assert_eq!(
                paint_path_stroke(ui.painter(), &path, f64::INFINITY, Color32::GREEN),
                Err(ShapeError::InvalidStrokeWidth)
            );
            assert_eq!(
                paint_path_stroke(ui.painter(), &path, 4096.5, Color32::GREEN),
                Err(ShapeError::InvalidStrokeWidth)
            );
        });
        let paths = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Path(path) => Some(path),
                _ => None,
            })
            .collect::<Vec<_>>();
        output.textures_delta.clear();
        assert_eq!(paths.len(), 2);
        assert!(
            paths
                .iter()
                .all(|path| path.closed && path.fill == Color32::TRANSPARENT)
        );
        assert!(paths.iter().all(|path| path.stroke.width == 2.5));
    }

    #[test]
    fn path_fill_paints_bounded_even_odd_mesh_and_empty_result_is_a_noop() {
        let path = Path2D {
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
        let empty = Path2D {
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
        .unwrap()
        .boolean(
            &Path2D {
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
            .unwrap(),
            doofusdock_presentation::PathBooleanOp2D::Difference,
        )
        .unwrap();

        let context = egui::Context::default();
        let mut count = 0;
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            count = paint_path_fill(ui.painter(), &path, Color32::from_rgb(20, 40, 60)).unwrap();
            assert_eq!(
                paint_path_fill(ui.painter(), &empty, Color32::WHITE).unwrap(),
                0
            );
        });
        assert!(
            count >= 8,
            "hole-aware mesh should contain multiple triangles"
        );
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Mesh(mesh) if mesh.indices.len() == count * 3
        )));
        output.textures_delta.clear();
    }

    #[test]
    fn linear_gradient_fill_interpolates_vertex_colors_and_rejects_zero_length() {
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
        let start_color = Color32::from_rgb(255, 0, 0);
        let end_color = Color32::from_rgb(0, 0, 255);
        let context = egui::Context::default();
        let mut count = 0;
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            count = paint_path_fill_linear_gradient(
                ui.painter(),
                &path,
                [0.0, 0.0],
                [10.0, 0.0],
                start_color,
                end_color,
            )
            .unwrap();
            assert_eq!(
                paint_path_fill_linear_gradient(
                    ui.painter(),
                    &path,
                    [1.0, 1.0],
                    [1.0, 1.0],
                    start_color,
                    end_color,
                ),
                Err(ShapeError::InvalidGradient)
            );
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("gradient submits an egui mesh");
        assert!(count > 0);
        assert!(
            mesh.vertices
                .iter()
                .any(|vertex| { vertex.pos.x == 0.0 && vertex.color == start_color })
        );
        assert!(
            mesh.vertices
                .iter()
                .any(|vertex| { vertex.pos.x == 10.0 && vertex.color == end_color })
        );
        output.textures_delta.clear();
    }

    #[test]
    fn clipped_path_fill_mesh_stays_inside_the_intersection() {
        fn rectangle(x: f64, right: f64) -> FlattenedPath2D {
            Path2D {
                commands: vec![
                    PathCommand2D::MoveTo { point: [x, 0.0] },
                    PathCommand2D::LineTo {
                        point: [right, 0.0],
                    },
                    PathCommand2D::LineTo {
                        point: [right, 10.0],
                    },
                    PathCommand2D::LineTo { point: [x, 10.0] },
                    PathCommand2D::Close,
                ],
            }
            .flatten(0.5, 32)
            .unwrap()
        }

        let subject = rectangle(0.0, 10.0);
        let clip = rectangle(5.0, 15.0);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_path_fill_clipped(ui.painter(), &subject, &clip, Color32::WHITE).unwrap(),
                2
            );
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("clipped path produces a mesh");
        assert_eq!(mesh.indices.len(), 6);
        assert!(mesh.vertices.iter().all(|vertex| {
            (5.0..=10.0).contains(&vertex.pos.x) && (0.0..=10.0).contains(&vertex.pos.y)
        }));
        output.textures_delta.clear();
    }

    #[test]
    fn scene_path_fill_follows_composed_transform_and_inherited_clip() {
        use doofusdock_presentation::{Affine2, SceneNode, SceneRect};
        use std::collections::BTreeMap;

        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 30.0,
            height: 20.0,
        });
        root.clip = Some(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 20.0,
        });
        let mut panel = SceneNode::panel(
            "editor",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 10.0,
            },
        );
        panel.transform = Affine2::translate(3.0, 4.0).unwrap();
        panel.opacity = 0.5;
        root.children.push(panel);
        let frame = SceneFrame::build(&root, 4, 9)
            .unwrap()
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("editor"),
                    Affine2::translate(2.0, 1.0).unwrap(),
                )]),
                4,
                9,
            )
            .unwrap();
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [20.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();
        let paths = BTreeMap::from([(PanelId::from("editor"), path)]);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_scene_path_fills(ui.painter(), &frame, &paths, Color32::WHITE, 4, 9).unwrap(),
                ScenePathFillRenderReport {
                    rendered: 1,
                    clipped_away: 0,
                }
            );
            assert!(
                paint_scene_path_fills(ui.painter(), &frame, &paths, Color32::WHITE, 5, 9).is_err()
            );
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("visible scene path should produce a mesh");
        assert_eq!(mesh.indices.len(), 6);
        assert!(mesh.vertices.iter().all(|vertex| {
            (5.0..=10.0).contains(&vertex.pos.x) && (5.0..=15.0).contains(&vertex.pos.y)
        }));
        assert!(mesh.vertices.iter().all(|vertex| vertex.color.a() == 128));
        output.textures_delta.clear();
    }

    #[test]
    fn scene_path_gradient_follows_transform_clip_opacity_and_revision() {
        use doofusdock_presentation::{Affine2, SceneNode, SceneRect};
        use std::collections::BTreeMap;

        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 30.0,
            height: 20.0,
        });
        root.clip = Some(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 20.0,
        });
        let mut panel = SceneNode::panel(
            "editor",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 10.0,
            },
        );
        panel.transform = Affine2::translate(3.0, 4.0).unwrap();
        panel.opacity = 0.5;
        root.children.push(panel);
        let frame = SceneFrame::build(&root, 4, 9)
            .unwrap()
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("editor"),
                    Affine2::translate(2.0, 1.0).unwrap(),
                )]),
                4,
                9,
            )
            .unwrap();
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [20.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();
        let start_color = Color32::RED;
        let end_color = Color32::BLUE;
        let fills = BTreeMap::from([(
            PanelId::from("editor"),
            ScenePathLinearGradient {
                path,
                start: [0.0, 0.0],
                end: [20.0, 0.0],
                start_color,
                end_color,
            },
        )]);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_scene_path_linear_gradient_fills(ui.painter(), &frame, &fills, 4, 9).unwrap(),
                ScenePathFillRenderReport {
                    rendered: 1,
                    clipped_away: 0,
                }
            );
            assert!(
                paint_scene_path_linear_gradient_fills(ui.painter(), &frame, &fills, 5, 9).is_err()
            );
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("visible scene gradient should produce a mesh");
        output.textures_delta.clear();
        assert_eq!(mesh.indices.len(), 6);
        assert!(mesh.vertices.iter().all(|vertex| {
            (5.0..=10.0).contains(&vertex.pos.x) && (5.0..=15.0).contains(&vertex.pos.y)
        }));
        assert!(mesh.vertices.iter().all(|vertex| vertex.color.a() == 128));
        assert!(
            mesh.vertices.iter().any(
                |vertex| vertex.pos.x == 5.0 && vertex.color == start_color.gamma_multiply(0.5)
            )
        );
        assert!(mesh.vertices.iter().any(|vertex| {
            vertex.pos.x > 5.0 && vertex.color != start_color.gamma_multiply(0.5)
        }));
    }

    #[test]
    fn scene_path_stroke_transforms_clips_and_applies_opacity() {
        use doofusdock_presentation::{Affine2, SceneNode, SceneRect};
        use std::collections::BTreeMap;

        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 30.0,
            height: 20.0,
        });
        root.clip = Some(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 20.0,
        });
        let mut panel = SceneNode::panel(
            "editor",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 10.0,
            },
        );
        panel.transform = Affine2::translate(3.0, 4.0).unwrap();
        panel.opacity = 0.5;
        root.children.push(panel);
        let frame = SceneFrame::build(&root, 4, 9)
            .unwrap()
            .with_panel_transforms(
                &BTreeMap::from([(
                    PanelId::from("editor"),
                    Affine2::translate(2.0, 1.0).unwrap(),
                )]),
                4,
                9,
            )
            .unwrap();
        let path = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [20.0, 10.0],
                },
                PathCommand2D::LineTo { point: [0.0, 10.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.5, 32)
        .unwrap();
        let paths = BTreeMap::from([(PanelId::from("editor"), path)]);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_scene_path_strokes(ui.painter(), &frame, &paths, 2.0, Color32::WHITE, 4, 9)
                    .unwrap(),
                ScenePathFillRenderReport {
                    rendered: 1,
                    clipped_away: 0,
                }
            );
            assert!(
                paint_scene_path_strokes(ui.painter(), &frame, &paths, 2.0, Color32::WHITE, 5, 9)
                    .is_err()
            );
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("visible clipped scene stroke should produce a mesh");
        output.textures_delta.clear();
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|vertex| {
            (5.0..=10.0).contains(&vertex.pos.x) && (5.0..=15.0).contains(&vertex.pos.y)
        }));
        assert!(mesh.vertices.iter().all(|vertex| vertex.color.a() == 128));
    }
}
