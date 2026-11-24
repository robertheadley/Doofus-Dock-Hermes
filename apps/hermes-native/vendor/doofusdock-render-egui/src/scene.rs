//! Texture-backed rendering for transformed, toolkit-neutral scene frames.
use doofusdock_presentation::SceneSurfaceGeometry;
use doofusdock_presentation::{SceneFrame, SceneSurfaceMeshGeometry};
use egui::{Color32, Mesh, Painter, TextureId, epaint::Vertex, pos2};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneSurfaceRenderReport {
    pub rendered: usize,
    pub missing_surfaces: usize,
    pub clipped_away: usize,
}

/// Paint host-owned offscreen panel textures through scene transforms and clips.
/// The caller remains responsible for producing and retaining each texture.
pub fn paint_scene_surfaces(
    painter: &Painter,
    frame: &SceneFrame,
    surfaces: &BTreeMap<doofusdock_core::PanelId, TextureId>,
    revision: u64,
    generation: u64,
) -> doofusdock_core::DockResult<SceneSurfaceRenderReport> {
    let mut report = SceneSurfaceRenderReport::default();
    if frame.requires_indexed_surface_mesh() {
        for surface in frame.surface_mesh_geometry(revision, generation)? {
            let Some(texture) = surfaces.get(&surface.panel).copied() else {
                report.missing_surfaces += 1;
                continue;
            };
            let mesh = mesh_for_surface_mesh(texture, &surface);
            if mesh.indices.is_empty() {
                report.clipped_away += 1;
                continue;
            }
            painter.add(egui::Shape::mesh(mesh));
            report.rendered += 1;
        }
    } else {
        for surface in frame.surface_geometry(revision, generation)? {
            let Some(texture) = surfaces.get(&surface.panel).copied() else {
                report.missing_surfaces += 1;
                continue;
            };
            let mesh = mesh_for_geometry(texture, &surface);
            if mesh.indices.is_empty() {
                report.clipped_away += 1;
                continue;
            }
            painter.add(egui::Shape::mesh(mesh));
            report.rendered += 1;
        }
    }
    Ok(report)
}

fn mesh_for_geometry(texture: TextureId, geometry: &SceneSurfaceGeometry) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    if geometry.vertices.len() < 3 {
        return mesh;
    }
    let color = Color32::from_white_alpha((geometry.opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
    mesh.vertices.reserve(geometry.vertices.len());
    for vertex in &geometry.vertices {
        mesh.vertices.push(Vertex {
            pos: pos2(vertex.position.x as f32, vertex.position.y as f32),
            uv: pos2(vertex.uv.x as f32, vertex.uv.y as f32),
            color,
        });
    }
    for index in 1..geometry.vertices.len() - 1 {
        mesh.indices
            .extend_from_slice(&[0, index as u32, (index + 1) as u32]);
    }
    mesh
}

fn mesh_for_surface_mesh(texture: TextureId, geometry: &SceneSurfaceMeshGeometry) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    let color = Color32::from_white_alpha((geometry.opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
    mesh.vertices.reserve(geometry.vertices.len());
    for vertex in &geometry.vertices {
        mesh.vertices.push(Vertex {
            pos: pos2(vertex.position.x as f32, vertex.position.y as f32),
            uv: pos2(vertex.uv.x as f32, vertex.uv.y as f32),
            color,
        });
    }
    mesh.indices.extend_from_slice(&geometry.indices);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use doofusdock_presentation::{Affine2, SceneNode, ScenePoint, SceneRect, SceneSurfaceVertex};

    #[test]
    fn texture_mesh_preserves_projected_positions_uvs_and_opacity() {
        let geometry = SceneSurfaceGeometry {
            panel: "surface".into(),
            opacity: 0.5,
            vertices: vec![
                SceneSurfaceVertex {
                    position: ScenePoint { x: 10.0, y: 20.0 },
                    uv: ScenePoint { x: 0.0, y: 0.0 },
                },
                SceneSurfaceVertex {
                    position: ScenePoint { x: 30.0, y: 20.0 },
                    uv: ScenePoint { x: 1.0, y: 0.0 },
                },
                SceneSurfaceVertex {
                    position: ScenePoint { x: 30.0, y: 40.0 },
                    uv: ScenePoint { x: 1.0, y: 1.0 },
                },
                SceneSurfaceVertex {
                    position: ScenePoint { x: 10.0, y: 40.0 },
                    uv: ScenePoint { x: 0.0, y: 1.0 },
                },
            ],
        };
        let mesh = mesh_for_geometry(TextureId::Managed(7), &geometry);
        assert_eq!(mesh.texture_id, TextureId::Managed(7));
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.vertices[1].pos, pos2(30.0, 20.0));
        assert_eq!(mesh.vertices[2].uv, pos2(1.0, 1.0));
        assert_eq!(mesh.vertices[0].color.a(), 128);
    }

    #[test]
    fn public_painter_path_reports_host_surfaces_and_rejects_stale_frames() {
        let mut panel = SceneNode::panel(
            "editor",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
        );
        panel.transform = Affine2::translate(4.0, 6.0).unwrap();
        let frame = SceneFrame::build(&panel, 3, 9).unwrap();
        let surfaces = BTreeMap::from([(
            doofusdock_core::PanelId::from("editor"),
            TextureId::Managed(2),
        )]);
        let context = egui::Context::default();
        let mut report = None;
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            report = Some(paint_scene_surfaces(ui.painter(), &frame, &surfaces, 3, 9).unwrap());
            assert!(paint_scene_surfaces(ui.painter(), &frame, &surfaces, 4, 9).is_err());
        });
        output.textures_delta.clear();
        assert_eq!(
            report,
            Some(SceneSurfaceRenderReport {
                rendered: 1,
                missing_surfaces: 0,
                clipped_away: 0,
            })
        );
    }

    #[test]
    fn curved_scene_clip_is_present_in_the_painted_surface_mesh() {
        use doofusdock_core::PanelId;
        use doofusdock_presentation::{Path2D, PathCommand2D};

        let mut panel = SceneNode::panel(
            "circle-clipped",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 10.0,
            },
        );
        panel.transform = Affine2::translate(5.0, 3.0).unwrap();
        let frame = SceneFrame::build(&panel, 12, 4).unwrap();
        let k = 0.552_284_749_830_793_6;
        let clip = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [14.0, 5.0] },
                PathCommand2D::CubicTo {
                    control1: [14.0, 5.0 + 4.0 * k],
                    control2: [10.0 + 4.0 * k, 9.0],
                    point: [10.0, 9.0],
                },
                PathCommand2D::CubicTo {
                    control1: [10.0 - 4.0 * k, 9.0],
                    control2: [6.0, 5.0 + 4.0 * k],
                    point: [6.0, 5.0],
                },
                PathCommand2D::CubicTo {
                    control1: [6.0, 5.0 - 4.0 * k],
                    control2: [10.0 - 4.0 * k, 1.0],
                    point: [10.0, 1.0],
                },
                PathCommand2D::CubicTo {
                    control1: [10.0 + 4.0 * k, 1.0],
                    control2: [14.0, 5.0 - 4.0 * k],
                    point: [14.0, 5.0],
                },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.1, 128)
        .unwrap();
        let frame = frame
            .with_panel_clip_paths(
                &BTreeMap::from([(PanelId::from("circle-clipped"), clip.clone())]),
                12,
                4,
            )
            .unwrap();
        let surfaces = BTreeMap::from([(PanelId::from("circle-clipped"), TextureId::Managed(3))]);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_scene_surfaces(ui.painter(), &frame, &surfaces, 12, 4).unwrap(),
                SceneSurfaceRenderReport {
                    rendered: 1,
                    missing_surfaces: 0,
                    clipped_away: 0,
                }
            );
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("circle-clipped host surface should paint a mesh");
        output.textures_delta.clear();
        assert!(mesh.vertices.len() > 4);
        assert!(mesh.indices.len() > 6);
        let max_radius = mesh
            .vertices
            .iter()
            .map(|vertex| {
                let x = vertex.pos.x as f64 - 5.0 - 10.0;
                let y = vertex.pos.y as f64 - 3.0 - 5.0;
                (x * x + y * y).sqrt()
            })
            .fold(0.0_f64, f64::max);
        assert!(max_radius <= 4.15, "max clipped radius={max_radius}");
    }

    #[test]
    fn concave_panel_clip_uses_indexed_mesh_surface_painter() {
        use doofusdock_presentation::{Path2D, PathCommand2D};

        let panel = SceneNode::panel(
            "l-shaped",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
        );
        let frame = SceneFrame::build(&panel, 7, 9).unwrap();
        let clip = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 0.0] },
                PathCommand2D::LineTo { point: [20.0, 5.0] },
                PathCommand2D::LineTo { point: [5.0, 5.0] },
                PathCommand2D::LineTo { point: [5.0, 20.0] },
                PathCommand2D::LineTo { point: [0.0, 20.0] },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        let frame = frame
            .with_panel_mesh_clip_paths(&BTreeMap::from([("l-shaped".into(), clip)]), 7, 9)
            .unwrap();
        let surfaces = BTreeMap::from([("l-shaped".into(), TextureId::Managed(12))]);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            let report = paint_scene_surfaces(ui.painter(), &frame, &surfaces, 7, 9).unwrap();
            assert_eq!(report.rendered, 1);
            assert_eq!(report.clipped_away, 0);
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .expect("concave panel texture should paint an indexed mesh");
        output.textures_delta.clear();
        assert!(mesh.vertices.len() > 4);
        assert_eq!(mesh.indices.len() % 3, 0);
    }

    #[test]
    fn shared_group_clip_paints_indexed_meshes_for_each_member_surface() {
        use doofusdock_core::PanelId;
        use doofusdock_presentation::{Path2D, PathCommand2D, SceneGroupClip};
        use std::collections::BTreeSet;

        let mut root = SceneNode::group(SceneRect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 20.0,
        });
        let mut right = SceneNode::panel(
            "group-right",
            SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
        );
        right.transform = Affine2::translate(20.0, 0.0).unwrap();
        root.children = vec![
            SceneNode::panel(
                "group-left",
                SceneRect {
                    x: 0.0,
                    y: 0.0,
                    width: 20.0,
                    height: 20.0,
                },
            ),
            right,
        ];
        let frame = SceneFrame::build(&root, 11, 5).unwrap();
        let donut = Path2D {
            commands: vec![
                PathCommand2D::MoveTo { point: [0.0, 0.0] },
                PathCommand2D::LineTo { point: [40.0, 0.0] },
                PathCommand2D::LineTo {
                    point: [40.0, 20.0],
                },
                PathCommand2D::LineTo { point: [0.0, 20.0] },
                PathCommand2D::Close,
                PathCommand2D::MoveTo { point: [15.0, 5.0] },
                PathCommand2D::LineTo { point: [25.0, 5.0] },
                PathCommand2D::LineTo {
                    point: [25.0, 15.0],
                },
                PathCommand2D::LineTo {
                    point: [15.0, 15.0],
                },
                PathCommand2D::Close,
            ],
        }
        .flatten(0.25, 32)
        .unwrap();
        let frame = frame
            .with_group_mesh_clip_paths(
                &[SceneGroupClip {
                    panels: BTreeSet::from([
                        PanelId::from("group-left"),
                        PanelId::from("group-right"),
                    ]),
                    path: donut.clone(),
                    world_transform: Affine2::IDENTITY,
                }],
                11,
                5,
            )
            .unwrap();
        let surfaces = BTreeMap::from([
            (PanelId::from("group-left"), TextureId::Managed(20)),
            (PanelId::from("group-right"), TextureId::Managed(21)),
        ]);
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            assert_eq!(
                paint_scene_surfaces(ui.painter(), &frame, &surfaces, 11, 5).unwrap(),
                SceneSurfaceRenderReport {
                    rendered: 2,
                    missing_surfaces: 0,
                    clipped_away: 0,
                }
            );
        });
        let meshes = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .collect::<Vec<_>>();
        output.textures_delta.clear();
        assert_eq!(meshes.len(), 2);
        for mesh in meshes {
            assert!(!mesh.indices.is_empty());
            assert_eq!(mesh.indices.len() % 3, 0);
            for triangle in mesh.indices.as_chunks::<3>().0 {
                let center = [
                    triangle
                        .iter()
                        .map(|index| f64::from(mesh.vertices[*index as usize].pos.x))
                        .sum::<f64>()
                        / 3.0,
                    triangle
                        .iter()
                        .map(|index| f64::from(mesh.vertices[*index as usize].pos.y))
                        .sum::<f64>()
                        / 3.0,
                ];
                assert!(donut.contains(center).unwrap());
            }
        }
    }

    #[test]
    fn structural_morph_scene_samples_render_through_application_surfaces() {
        use doofusdock_core::{LayoutDocument, Node, NodeKind, Orientation, PanelDescriptor};
        use doofusdock_presentation::{Box2, Morph};

        let mut source = LayoutDocument::new("morph-surfaces", "workspace");
        source.panels = vec![
            PanelDescriptor::new("left", "Left"),
            PanelDescriptor::new("right", "Right"),
        ]
        .into();
        source.root = Some(Node::split(
            "root",
            Orientation::Horizontal,
            vec![
                Node::panel("left-node", "left"),
                Node::panel("right-node", "right"),
            ],
            vec![0.3, 0.7],
        ));
        let canonical_before = source.clone();
        let mut target = source.clone();
        let mut root = target.root.as_deref().unwrap().clone();
        let NodeKind::Split { weights, .. } = &mut root.kind else {
            panic!("fixture root should be a split");
        };
        weights.swap(0, 1);
        target.root = Some(std::sync::Arc::new(root));

        let morph = Morph::new(
            &source,
            &target,
            Box2 {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
            0.0,
            1.0,
            false,
        )
        .unwrap();
        let frame = morph.sample_scene(0.5, 9, &BTreeMap::new()).unwrap();
        let surfaces = BTreeMap::from([
            ("left".into(), TextureId::Managed(10)),
            ("right".into(), TextureId::Managed(11)),
        ]);
        let context = egui::Context::default();
        let mut report = None;
        let mut output = context.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            report = Some(
                paint_scene_surfaces(ui.painter(), &frame, &surfaces, source.revision, 9).unwrap(),
            );
        });
        output.textures_delta.clear();

        assert_eq!(frame.revision(), source.revision);
        assert_eq!(frame.generation(), 9);
        assert_eq!(
            report,
            Some(SceneSurfaceRenderReport {
                rendered: 2,
                missing_surfaces: 0,
                clipped_away: 0,
            })
        );
        assert_eq!(source, canonical_before);
    }
}
