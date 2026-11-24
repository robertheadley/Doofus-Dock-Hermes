//! egui textured-mesh adapter for presentation cube transitions.
//!
//! Hosts capture outgoing/incoming content into textures and decide when input
//! and focus move to the incoming layout. This module only paints the transient
//! projected faces and never changes canonical docking state.
use doofusdock_presentation::{CubeFaceKind, CubeProjectedFace, CubeTransitionFrame};
use egui::{Color32, Mesh, Painter, Pos2, Rect, TextureId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeTransitionPaintError {
    InvalidOpacity,
    InvalidProjectedVertex,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CubeTransitionPaintReport {
    pub drawn_faces: usize,
    pub bounds: Option<Rect>,
}

/// Build one egui mesh for each visible face in the frame's back-to-front order.
/// Texture IDs are ordered `[outgoing, incoming]`.
pub fn cube_transition_meshes(
    frame: &CubeTransitionFrame,
    textures: [TextureId; 2],
    opacity: f32,
) -> Result<Vec<Mesh>, CubeTransitionPaintError> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(CubeTransitionPaintError::InvalidOpacity);
    }
    let mut meshes = Vec::with_capacity(2);
    for kind in &frame.draw_order {
        let face = match kind {
            CubeFaceKind::Outgoing => &frame.outgoing,
            CubeFaceKind::Incoming => &frame.incoming,
        };
        if !face.visible {
            continue;
        }
        let texture = match kind {
            CubeFaceKind::Outgoing => textures[0],
            CubeFaceKind::Incoming => textures[1],
        };
        meshes.push(face_mesh(face, texture, opacity)?);
    }
    Ok(meshes)
}

/// Paint projected outgoing and incoming snapshot textures through the painter's
/// active clip. Callers retain ownership of texture lifetime and focus policy.
pub fn paint_cube_transition(
    painter: &Painter,
    frame: &CubeTransitionFrame,
    textures: [TextureId; 2],
    opacity: f32,
) -> Result<CubeTransitionPaintReport, CubeTransitionPaintError> {
    let meshes = cube_transition_meshes(frame, textures, opacity)?;
    let mut report = CubeTransitionPaintReport::default();
    for mesh in meshes {
        let rect = mesh_bounds(&mesh)?;
        report.bounds = Some(report.bounds.map_or(rect, |bounds| bounds.union(rect)));
        report.drawn_faces += 1;
        painter.add(mesh);
    }
    Ok(report)
}

fn face_mesh(
    face: &CubeProjectedFace,
    texture: TextureId,
    opacity: f32,
) -> Result<Mesh, CubeTransitionPaintError> {
    let mut mesh = Mesh::with_texture(texture);
    mesh.vertices.reserve(4);
    for vertex in face.vertices {
        let x = vertex.position.x as f32;
        let y = vertex.position.y as f32;
        if !x.is_finite()
            || !y.is_finite()
            || x.abs() > 1_000_000.0
            || y.abs() > 1_000_000.0
            || vertex
                .uv
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(CubeTransitionPaintError::InvalidProjectedVertex);
        }
        mesh.vertices.push(egui::epaint::Vertex {
            pos: Pos2::new(x, y),
            uv: Pos2::new(vertex.uv[0], vertex.uv[1]),
            color: Color32::WHITE.gamma_multiply(opacity),
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    if !mesh.is_valid() {
        return Err(CubeTransitionPaintError::InvalidProjectedVertex);
    }
    Ok(mesh)
}

fn mesh_bounds(mesh: &Mesh) -> Result<Rect, CubeTransitionPaintError> {
    let Some(first) = mesh.vertices.first() else {
        return Err(CubeTransitionPaintError::InvalidProjectedVertex);
    };
    let mut min = first.pos;
    let mut max = first.pos;
    for vertex in mesh.vertices.iter().skip(1) {
        min.x = min.x.min(vertex.pos.x);
        min.y = min.y.min(vertex.pos.y);
        max.x = max.x.max(vertex.pos.x);
        max.y = max.y.max(vertex.pos.y);
    }
    Ok(Rect::from_min_max(min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use doofusdock_presentation::{
        Box2, CubeRotationAxis, CubeRotationDirection, CubeTransition3D,
    };

    fn frame(progress: f64) -> CubeTransitionFrame {
        CubeTransition3D::new(
            Box2 {
                x: 10.0,
                y: 20.0,
                w: 160.0,
                h: 90.0,
            },
            Box2 {
                x: 5.0,
                y: 15.0,
                w: 220.0,
                h: 130.0,
            },
            CubeRotationAxis::Horizontal,
            CubeRotationDirection::Forward,
        )
        .unwrap()
        .sample(progress)
        .unwrap()
    }

    #[test]
    fn cube_frames_emit_texture_meshes_with_expected_uvs_and_visibility() {
        let outgoing = TextureId::User(10);
        let incoming = TextureId::User(11);
        let start = cube_transition_meshes(&frame(0.0), [outgoing, incoming], 1.0).unwrap();
        let middle = cube_transition_meshes(&frame(0.5), [outgoing, incoming], 0.75).unwrap();
        let end = cube_transition_meshes(&frame(1.0), [outgoing, incoming], 1.0).unwrap();
        assert_eq!(start.len(), 1);
        assert_eq!(start[0].texture_id, outgoing);
        assert_eq!(middle.len(), 2);
        assert_eq!(end.len(), 1);
        assert_eq!(end[0].texture_id, incoming);
        assert_eq!(end[0].indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(end[0].vertices[0].uv, Pos2::new(0.0, 0.0));
        assert_eq!(middle[0].vertices[0].color.a(), 191);
        assert!(mesh_bounds(&middle[0]).unwrap().is_finite());
    }

    #[test]
    fn cube_mesh_builder_rejects_bad_opacity_and_tampered_projection() {
        assert_eq!(
            cube_transition_meshes(
                &frame(0.5),
                [TextureId::User(1), TextureId::User(2)],
                f32::NAN
            ),
            Err(CubeTransitionPaintError::InvalidOpacity)
        );
        let mut bad = frame(0.5);
        bad.incoming.vertices[0].position.x = f64::INFINITY;
        assert_eq!(
            cube_transition_meshes(&bad, [TextureId::User(1), TextureId::User(2)], 1.0),
            Err(CubeTransitionPaintError::InvalidProjectedVertex)
        );
    }

    #[test]
    fn painter_accepts_snapshot_textures_in_headless_egui_context() {
        let frame = frame(0.5);
        let mut report = None;
        egui::__run_test_ui(|ui| {
            report = Some(
                paint_cube_transition(
                    ui.painter(),
                    &frame,
                    [TextureId::User(10), TextureId::User(11)],
                    1.0,
                )
                .unwrap(),
            );
        });
        assert_eq!(report.unwrap().drawn_faces, 2);
    }
}
