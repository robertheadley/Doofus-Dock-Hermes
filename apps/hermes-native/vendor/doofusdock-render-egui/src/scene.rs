//! Texture-backed rendering for transformed, toolkit-neutral scene frames.
use doofusdock_presentation::{SceneFrame, SceneSurfaceGeometry};
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
    let geometry = frame.surface_geometry(revision, generation)?;
    let mut report = SceneSurfaceRenderReport::default();
    for surface in geometry {
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
