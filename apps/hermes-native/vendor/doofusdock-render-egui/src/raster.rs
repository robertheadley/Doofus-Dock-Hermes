//! Bounded CPU rendering of egui's premultiplied sRGBA triangle meshes.
use egui::{
    ClippedPrimitive, Color32, TextureId, TexturesDelta,
    epaint::{ImageData, Primitive},
};
use std::collections::HashMap;

pub const MAX_PIXELS: usize = 3840 * 2160;
pub const MAX_TEXTURE_PIXELS: usize = 16 * 1024 * 1024;
#[derive(Default)]
pub struct Rasterizer {
    textures: HashMap<TextureId, Texture>,
}
struct Texture {
    size: [usize; 2],
    pixels: Vec<Color32>,
}
impl Rasterizer {
    pub fn update(&mut self, delta: &TexturesDelta) -> Result<(), String> {
        for (id, image) in delta
            .set
            .iter()
            .flat_map(|(id, images)| images.iter().map(move |image| (id, image)))
        {
            let ImageData::Color(data) = &image.image;
            let [w, h] = data.size;
            if w == 0
                || h == 0
                || w > 8192
                || h > 8192
                || w.checked_mul(h) != Some(data.pixels.len())
            {
                return Err("Invalid texture dimensions".into());
            }
            if let Some([x, y]) = image.pos {
                let texture = self
                    .textures
                    .get_mut(id)
                    .ok_or("Partial texture update without base")?;
                if x.checked_add(w).is_none_or(|v| v > texture.size[0])
                    || y.checked_add(h).is_none_or(|v| v > texture.size[1])
                {
                    return Err("Partial texture update out of bounds".into());
                }
                for row in 0..h {
                    let offset = (y + row) * texture.size[0] + x;
                    texture.pixels[offset..offset + w]
                        .copy_from_slice(&data.pixels[row * w..(row + 1) * w]);
                }
            } else {
                let used: usize = self
                    .textures
                    .iter()
                    .filter(|(key, _)| *key != id)
                    .map(|(_, v)| v.pixels.len())
                    .sum();
                if used + w * h > MAX_TEXTURE_PIXELS {
                    return Err("64 MiB texture budget exceeded".into());
                }
                self.textures.insert(
                    *id,
                    Texture {
                        size: data.size,
                        pixels: data.pixels.clone(),
                    },
                );
            }
        }
        Ok(())
    }
    pub fn free(&mut self, delta: &TexturesDelta) {
        for id in &delta.free {
            self.textures.remove(id);
        }
    }
    pub fn paint(
        &self,
        jobs: &[ClippedPrimitive],
        scale: f32,
        size: [usize; 2],
        pixels: &mut [u32],
    ) -> Result<(), String> {
        self.paint_region(jobs, scale, size, [0, 0, size[0], size[1]], pixels)
    }

    /// Redraw one physical-pixel region over a retained framebuffer. The caller
    /// must include every changed old/new surface bound and force `paint` for
    /// resize, theme, texture, scale, or otherwise unknown invalidation.
    pub fn paint_region(
        &self,
        jobs: &[ClippedPrimitive],
        scale: f32,
        size: [usize; 2],
        region: [usize; 4],
        pixels: &mut [u32],
    ) -> Result<(), String> {
        let [w, h] = size;
        if !scale.is_finite()
            || scale <= 0.0
            || w.checked_mul(h) != Some(pixels.len())
            || pixels.len() > MAX_PIXELS
        {
            return Err("Invalid or oversized framebuffer (maximum 3840x2160 pixels)".into());
        }
        let region = [
            region[0].min(w),
            region[1].min(h),
            region[2].min(w),
            region[3].min(h),
        ];
        if region[0] >= region[2] || region[1] >= region[3] {
            return Ok(());
        }
        for y in region[1]..region[3] {
            pixels[y * w + region[0]..y * w + region[2]].fill(0x10151d);
        }
        for job in jobs {
            let Primitive::Mesh(mesh) = &job.primitive else {
                return Err("GPU callback unsupported: provide baseline chrome fallback".into());
            };
            let tex = self
                .textures
                .get(&mesh.texture_id)
                .ok_or("Mesh references missing texture")?;
            let clip = job.clip_rect;
            let bounds = [
                ((clip.min.x * scale).floor().max(0.0) as usize).max(region[0]),
                ((clip.min.y * scale).floor().max(0.0) as usize).max(region[1]),
                ((clip.max.x * scale).ceil().max(0.0).min(w as f32) as usize).min(region[2]),
                ((clip.max.y * scale).ceil().max(0.0).min(h as f32) as usize).min(region[3]),
            ];
            if !mesh.is_valid() {
                return Err("Invalid mesh index".into());
            }
            for indices in mesh.indices.as_chunks::<3>().0 {
                let mut v = [
                    mesh.vertices[indices[0] as usize],
                    mesh.vertices[indices[1] as usize],
                    mesh.vertices[indices[2] as usize],
                ];
                for p in &mut v {
                    p.pos.x *= scale;
                    p.pos.y *= scale;
                }
                let edge = |a: egui::Pos2, b: egui::Pos2, p: egui::Pos2| {
                    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
                };
                let mut area = edge(v[0].pos, v[1].pos, v[2].pos);
                if !area.is_finite() || area.abs() < 0.0001 {
                    continue;
                }
                if area < 0.0 {
                    v.swap(1, 2);
                    area = -area;
                }
                let minx = v
                    .iter()
                    .map(|p| p.pos.x)
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .max(bounds[0] as f32) as usize;
                let miny = v
                    .iter()
                    .map(|p| p.pos.y)
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .max(bounds[1] as f32) as usize;
                let maxx = v
                    .iter()
                    .map(|p| p.pos.x)
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .max(0.0)
                    .min(bounds[2] as f32) as usize;
                let maxy = v
                    .iter()
                    .map(|p| p.pos.y)
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .max(0.0)
                    .min(bounds[3] as f32) as usize;
                let top_left =
                    |a: egui::Pos2, b: egui::Pos2| b.y < a.y || (b.y == a.y && b.x > a.x);
                let tl = [
                    top_left(v[1].pos, v[2].pos),
                    top_left(v[2].pos, v[0].pos),
                    top_left(v[0].pos, v[1].pos),
                ];
                let inv_area = 1.0 / area;
                let constant_sample = (v[0].uv == v[1].uv
                    && v[1].uv == v[2].uv
                    && v[0].color == v[1].color
                    && v[1].color == v[2].color)
                    .then(|| {
                        let texture = sample(tex, v[0].uv.x, v[0].uv.y);
                        let source: [f32; 4] = std::array::from_fn(|channel| {
                            v[0].color[channel] as f32 * texture[channel] / 255.0
                        });
                        (source, 1.0 - source[3] / 255.0)
                    });
                let edges = [
                    (v[1].pos, v[2].pos),
                    (v[2].pos, v[0].pos),
                    (v[0].pos, v[1].pos),
                ];
                let edge_dx = edges.map(|(a, b)| -(b.y - a.y));
                let edge_dy = edges.map(|(a, b)| b.x - a.x);
                let mut row_edge = {
                    let p = egui::pos2(minx as f32 + 0.5, miny as f32 + 0.5);
                    edges.map(|(a, b)| edge(a, b, p))
                };
                if let Some((source, inv)) = constant_sample {
                    let opaque_color = (inv == 0.0).then(|| {
                        let r = source[0].round().clamp(0.0, 255.0) as u32;
                        let g = source[1].round().clamp(0.0, 255.0) as u32;
                        let b = source[2].round().clamp(0.0, 255.0) as u32;
                        (r << 16) | (g << 8) | b
                    });
                    for y in miny..maxy {
                        let mut e = row_edge;
                        for x in minx..maxx {
                            if e[0] >= 0.0
                                && (e[0] != 0.0 || tl[0])
                                && e[1] >= 0.0
                                && (e[1] != 0.0 || tl[1])
                                && e[2] >= 0.0
                                && (e[2] != 0.0 || tl[2])
                            {
                                let index = y * w + x;
                                if let Some(color) = opaque_color {
                                    pixels[index] = color;
                                } else {
                                    let dst = pixels[index];
                                    let r = (source[0] + ((dst >> 16) & 255) as f32 * inv)
                                        .round()
                                        .clamp(0.0, 255.0)
                                        as u32;
                                    let g = (source[1] + ((dst >> 8) & 255) as f32 * inv)
                                        .round()
                                        .clamp(0.0, 255.0)
                                        as u32;
                                    let b = (source[2] + (dst & 255) as f32 * inv)
                                        .round()
                                        .clamp(0.0, 255.0)
                                        as u32;
                                    pixels[index] = (r << 16) | (g << 8) | b;
                                }
                            }
                            e[0] += edge_dx[0];
                            e[1] += edge_dx[1];
                            e[2] += edge_dx[2];
                        }
                        row_edge[0] += edge_dy[0];
                        row_edge[1] += edge_dy[1];
                        row_edge[2] += edge_dy[2];
                    }
                } else {
                    for y in miny..maxy {
                        let mut e = row_edge;
                        for x in minx..maxx {
                            if e[0] >= 0.0
                                && (e[0] != 0.0 || tl[0])
                                && e[1] >= 0.0
                                && (e[1] != 0.0 || tl[1])
                                && e[2] >= 0.0
                                && (e[2] != 0.0 || tl[2])
                            {
                                let b = [e[0] * inv_area, e[1] * inv_area, e[2] * inv_area];
                                let u = (0..3).map(|i| v[i].uv.x * b[i]).sum::<f32>();
                                let t = (0..3).map(|i| v[i].uv.y * b[i]).sum::<f32>();
                                let color = sample(tex, u, t);
                                let mut c = [0.0; 4];
                                for channel in 0..4 {
                                    c[channel] = (0..3)
                                        .map(|i| v[i].color[channel] as f32 * b[i])
                                        .sum::<f32>()
                                        * color[channel]
                                        / 255.0;
                                }
                                let dst = pixels[y * w + x];
                                let inv = 1.0 - c[3] / 255.0;
                                let r = (c[0] + ((dst >> 16) & 255) as f32 * inv)
                                    .round()
                                    .clamp(0.0, 255.0)
                                    as u32;
                                let g = (c[1] + ((dst >> 8) & 255) as f32 * inv)
                                    .round()
                                    .clamp(0.0, 255.0)
                                    as u32;
                                let b = (c[2] + (dst & 255) as f32 * inv).round().clamp(0.0, 255.0)
                                    as u32;
                                pixels[y * w + x] = (r << 16) | (g << 8) | b;
                            }
                            e[0] += edge_dx[0];
                            e[1] += edge_dx[1];
                            e[2] += edge_dx[2];
                        }
                        row_edge[0] += edge_dy[0];
                        row_edge[1] += edge_dy[1];
                        row_edge[2] += edge_dy[2];
                    }
                }
            }
        }
        Ok(())
    }
}
fn sample(tex: &Texture, u: f32, v: f32) -> [f32; 4] {
    let x = (u * tex.size[0] as f32 - 0.5).clamp(0.0, (tex.size[0] - 1) as f32);
    let y = (v * tex.size[1] as f32 - 0.5).clamp(0.0, (tex.size[1] - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(tex.size[0] - 1), (y0 + 1).min(tex.size[1] - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x, y| tex.pixels[y * tex.size[0] + x];
    std::array::from_fn(|c| {
        (at(x0, y0)[c] as f32 * (1.0 - fx) + at(x1, y0)[c] as f32 * fx) * (1.0 - fy)
            + (at(x0, y1)[c] as f32 * (1.0 - fx) + at(x1, y1)[c] as f32 * fx) * fy
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textured_quad_has_no_double_blended_diagonal_and_obeys_clip() {
        let mut r = Rasterizer::default();
        r.textures.insert(
            TextureId::Managed(0),
            Texture {
                size: [1, 1],
                pixels: vec![Color32::WHITE],
            },
        );
        let mut mesh = egui::epaint::Mesh::default();
        mesh.add_colored_rect(
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(4., 4.)),
            Color32::from_rgba_premultiplied(128, 0, 0, 128),
        );
        let job = ClippedPrimitive {
            clip_rect: egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(3., 4.)),
            primitive: Primitive::Mesh(mesh),
        };
        let mut buf = vec![0; 16];
        r.paint(&[job], 1., [4, 4], &mut buf).unwrap();
        assert_eq!(buf[0], buf[1]);
        assert_eq!(buf[0], buf[5]);
        assert_eq!(buf[0], 0x880a0e);
        assert_eq!(buf[3], 0x10151d);
        assert_ne!(buf[0], buf[3]);
    }

    #[test]
    fn opaque_constant_color_quad_fills_each_pixel_once() {
        let mut rasterizer = Rasterizer::default();
        rasterizer.textures.insert(
            TextureId::Managed(0),
            Texture {
                size: [1, 1],
                pixels: vec![Color32::WHITE],
            },
        );
        let mut mesh = egui::epaint::Mesh::default();
        mesh.add_colored_rect(
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(4., 4.)),
            Color32::from_rgb(30, 60, 90),
        );
        let job = ClippedPrimitive {
            clip_rect: egui::Rect::EVERYTHING,
            primitive: Primitive::Mesh(mesh),
        };
        let mut pixels = vec![0; 16];
        rasterizer.paint(&[job], 1., [4, 4], &mut pixels).unwrap();
        assert!(pixels.iter().all(|pixel| *pixel == 0x1e3c5a));
    }

    #[test]
    fn varying_texture_uvs_still_sample_each_pixel_correctly() {
        let mut rasterizer = Rasterizer::default();
        rasterizer.textures.insert(
            TextureId::Managed(0),
            Texture {
                size: [2, 2],
                pixels: vec![Color32::RED, Color32::GREEN, Color32::BLUE, Color32::WHITE],
            },
        );
        let mut mesh = egui::epaint::Mesh {
            texture_id: TextureId::Managed(0),
            ..Default::default()
        };
        mesh.add_rect_with_uv(
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(2., 2.)),
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
            Color32::WHITE,
        );
        let job = ClippedPrimitive {
            clip_rect: egui::Rect::EVERYTHING,
            primitive: Primitive::Mesh(mesh),
        };
        let mut pixels = vec![0; 4];
        rasterizer.paint(&[job], 1., [2, 2], &mut pixels).unwrap();
        assert_eq!(pixels, [0xff0000, 0x00ff00, 0x0000ff, 0xffffff]);
    }

    #[test]
    fn damaged_region_replays_overlap_and_alpha_and_matches_full_redraw() {
        let mut rasterizer = Rasterizer::default();
        rasterizer.textures.insert(
            TextureId::Managed(0),
            Texture {
                size: [1, 1],
                pixels: vec![Color32::WHITE],
            },
        );
        let build_jobs = |popup: egui::Rect, theme: Color32| {
            let mut background = egui::epaint::Mesh::default();
            background.add_colored_rect(
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(32.0, 24.0)),
                Color32::from_rgb(24, 32, 44),
            );
            let mut overlay = egui::epaint::Mesh::default();
            overlay.add_colored_rect(
                egui::Rect::from_min_max(egui::pos2(2.0, 2.0), egui::pos2(28.0, 20.0)),
                Color32::from_rgba_premultiplied(20, 80, 30, 128),
            );
            let mut dialog = egui::epaint::Mesh::default();
            dialog.add_colored_rect(
                popup,
                Color32::from_rgba_premultiplied(theme.r() / 2, theme.g() / 2, theme.b() / 2, 128),
            );
            vec![
                ClippedPrimitive {
                    clip_rect: egui::Rect::from_min_max(
                        egui::pos2(0.0, 0.0),
                        egui::pos2(32.0, 24.0),
                    ),
                    primitive: Primitive::Mesh(background),
                },
                ClippedPrimitive {
                    clip_rect: egui::Rect::from_min_max(
                        egui::pos2(1.0, 1.0),
                        egui::pos2(29.0, 22.0),
                    ),
                    primitive: Primitive::Mesh(overlay),
                },
                ClippedPrimitive {
                    clip_rect: egui::Rect::from_min_max(
                        egui::pos2(0.0, 0.0),
                        egui::pos2(32.0, 24.0),
                    ),
                    primitive: Primitive::Mesh(dialog),
                },
            ]
        };
        let old_popup = egui::Rect::from_min_max(egui::pos2(4.0, 4.0), egui::pos2(12.0, 12.0));
        let new_popup = egui::Rect::from_min_max(egui::pos2(19.0, 13.0), egui::pos2(29.0, 21.0));
        let old_jobs = build_jobs(old_popup, Color32::from_rgb(240, 80, 40));
        let new_jobs = build_jobs(new_popup, Color32::from_rgb(40, 120, 240));
        let mut retained = vec![0; 32 * 24];
        rasterizer
            .paint(&old_jobs, 1.0, [32, 24], &mut retained)
            .unwrap();

        let mut damaged = retained.clone();
        rasterizer
            .paint_region(&new_jobs, 1.0, [32, 24], [4, 4, 29, 21], &mut damaged)
            .unwrap();
        let mut full = vec![0; 32 * 24];
        rasterizer
            .paint(&new_jobs, 1.0, [32, 24], &mut full)
            .unwrap();

        assert_eq!(damaged, full);
    }

    #[test]
    fn full_paint_remains_resize_and_theme_invalidation_fallback() {
        let mut rasterizer = Rasterizer::default();
        rasterizer.textures.insert(
            TextureId::Managed(0),
            Texture {
                size: [1, 1],
                pixels: vec![Color32::WHITE],
            },
        );
        let theme_job = |color| {
            let mut mesh = egui::epaint::Mesh::default();
            mesh.add_colored_rect(
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(4.0, 4.0)),
                color,
            );
            ClippedPrimitive {
                clip_rect: egui::Rect::EVERYTHING,
                primitive: Primitive::Mesh(mesh),
            }
        };
        let mut small = vec![0; 4 * 4];
        rasterizer
            .paint(
                &[theme_job(Color32::from_rgb(20, 40, 60))],
                1.0,
                [4, 4],
                &mut small,
            )
            .unwrap();
        assert!(small.iter().all(|pixel| *pixel == 0x14283c));

        rasterizer
            .paint(
                &[theme_job(Color32::from_rgb(60, 40, 20))],
                1.0,
                [4, 4],
                &mut small,
            )
            .unwrap();
        assert!(small.iter().all(|pixel| *pixel == 0x3c2814));

        let mut resized = vec![0; 6 * 5];
        rasterizer
            .paint(
                &[theme_job(Color32::from_rgb(60, 40, 20))],
                1.0,
                [6, 5],
                &mut resized,
            )
            .unwrap();
        assert!(resized[..4].iter().all(|pixel| *pixel == 0x3c2814));
        assert!(resized.contains(&0x10151d));
    }

    #[test]
    fn resource_and_missing_texture_errors_are_explicit() {
        let r = Rasterizer::default();
        assert!(r.paint(&[], f32::NAN, [1, 1], &mut [0]).is_err());
        let job = ClippedPrimitive {
            clip_rect: egui::Rect::EVERYTHING,
            primitive: Primitive::Mesh(Default::default()),
        };
        assert!(
            r.paint(&[job], 1., [1, 1], &mut [0])
                .unwrap_err()
                .contains("missing texture")
        );
    }
}
