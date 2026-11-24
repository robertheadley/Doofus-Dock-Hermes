//! Bounded bridge from panel-local egui output to retained scene textures.
//!
//! Hosts keep panel contexts and platform output separate, call [`SceneEguiPanelRenderer::update_panel`]
//! for each visible panel, then pass [`SceneEguiPanelRenderer::surface_textures`] to
//! `scene::paint_scene_surfaces`. The host still owns input routing and platform commands.
use crate::raster::{MAX_PIXELS, Rasterizer};
use doofusdock_core::PanelId;
use egui::{Color32, Context, FullOutput, TextureHandle, TextureId, TextureOptions};
use std::collections::{BTreeMap, BTreeSet};

/// Aggregate bound for retained panel-context textures, measured in source pixels.
pub const MAX_SCENE_PANEL_TEXTURE_PIXELS: usize = 16 * 1024 * 1024;
/// Aggregate bound for retained rasterized panel surfaces, measured in physical pixels.
pub const MAX_SCENE_PANEL_SURFACE_PIXELS: usize = 16 * 1024 * 1024;
/// Bound for retained panel IDs, including currently hidden panels kept by the host.
pub const MAX_SCENE_PANEL_SURFACES: usize = 256;

#[derive(Default)]
pub struct SceneEguiPanelRenderer {
    rasterizers: BTreeMap<PanelId, Rasterizer>,
    textures: BTreeMap<PanelId, TextureHandle>,
    surface_pixels: BTreeMap<PanelId, usize>,
}

impl SceneEguiPanelRenderer {
    /// Apply one context's texture delta, tessellate its widget shapes, and
    /// upload the bounded software-rasterized result to the host context.
    /// `FullOutput::platform_output` remains host-owned and is not consumed.
    pub fn update_panel(
        &mut self,
        host_context: &Context,
        panel: &PanelId,
        panel_context: &Context,
        output: &mut FullOutput,
        size: [usize; 2],
    ) -> Result<TextureId, String> {
        let result = self.update_panel_inner(host_context, panel, panel_context, output, size);
        // The adapter owns this context's texture delta even on rejection; it
        // must not be silently applied a second time by another renderer.
        output.textures_delta.clear();
        result
    }

    fn update_panel_inner(
        &mut self,
        host_context: &Context,
        panel: &PanelId,
        panel_context: &Context,
        output: &FullOutput,
        size: [usize; 2],
    ) -> Result<TextureId, String> {
        let Some(pixels) = size[0].checked_mul(size[1]) else {
            return Err("Panel surface dimensions overflow".into());
        };
        if size[0] == 0 || size[1] == 0 || pixels > MAX_PIXELS {
            return Err("Panel surface is empty or exceeds the renderer pixel bound".into());
        }
        let current_pixels = self.surface_pixels.get(panel).copied().unwrap_or_default();
        let other_surface_pixels = self
            .surface_pixels
            .values()
            .sum::<usize>()
            .saturating_sub(current_pixels);
        if other_surface_pixels.saturating_add(pixels) > MAX_SCENE_PANEL_SURFACE_PIXELS {
            return Err("Aggregate panel surface pixel budget exceeded".into());
        }
        if !self.rasterizers.contains_key(panel)
            && self.rasterizers.len() >= MAX_SCENE_PANEL_SURFACES
        {
            return Err("Retained panel surface count limit exceeded".into());
        }

        let projected_panel_textures = if let Some(rasterizer) = self.rasterizers.get(panel) {
            rasterizer.projected_texture_pixels_after(&output.textures_delta)?
        } else {
            Rasterizer::default().projected_texture_pixels_after(&output.textures_delta)?
        };
        let other_texture_pixels = self
            .rasterizers
            .iter()
            .filter(|(id, _)| *id != panel)
            .map(|(_, renderer)| renderer.texture_pixels())
            .sum::<usize>();
        if other_texture_pixels.saturating_add(projected_panel_textures)
            > MAX_SCENE_PANEL_TEXTURE_PIXELS
        {
            return Err("Aggregate panel-context texture pixel budget exceeded".into());
        }
        let rasterizer = self.rasterizers.entry(panel.clone()).or_default();
        rasterizer.update(&output.textures_delta)?;
        let jobs = panel_context.tessellate(output.shapes.clone(), output.pixels_per_point);
        let mut rgb = vec![0u32; pixels];
        rasterizer
            .paint(&jobs, output.pixels_per_point, size, &mut rgb)
            .map_err(|error| format!("Panel output rasterization failed: {error}"))?;
        rasterizer.free(&output.textures_delta);

        let image = egui::ColorImage::new(
            size,
            rgb.into_iter()
                .map(|pixel| {
                    Color32::from_rgb(
                        ((pixel >> 16) & 0xff) as u8,
                        ((pixel >> 8) & 0xff) as u8,
                        (pixel & 0xff) as u8,
                    )
                })
                .collect(),
        );
        let texture = if let Some(texture) = self.textures.get_mut(panel) {
            texture.set(image, TextureOptions::NEAREST);
            texture.clone()
        } else {
            let texture = host_context.load_texture(
                format!("doofusdock-scene-panel:{}", panel.0),
                image,
                TextureOptions::NEAREST,
            );
            self.textures.insert(panel.clone(), texture.clone());
            texture
        };
        self.surface_pixels.insert(panel.clone(), pixels);
        Ok(texture.id())
    }

    /// Texture IDs ready for revision-bound `paint_scene_surfaces` calls.
    pub fn surface_textures(&self) -> BTreeMap<PanelId, TextureId> {
        self.textures
            .iter()
            .map(|(panel, texture)| (panel.clone(), texture.id()))
            .collect()
    }

    /// Remove texture and partial atlas state after the host retires a panel.
    pub fn remove_panel(&mut self, panel: &PanelId) -> bool {
        let had_surface = self.surface_pixels.remove(panel).is_some();
        let had_rasterizer = self.rasterizers.remove(panel).is_some();
        let had_texture = self.textures.remove(panel).is_some();
        had_surface || had_rasterizer || had_texture
    }

    /// Retain renderer state only for host-owned panel identities.
    pub fn retain_panels(&mut self, live_panels: &BTreeSet<PanelId>) {
        self.surface_pixels
            .retain(|panel, _| live_panels.contains(panel));
        self.rasterizers
            .retain(|panel, _| live_panels.contains(panel));
        self.textures.retain(|panel, _| live_panels.contains(panel));
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        panel_contexts::SceneEguiPanelContexts, raster::Rasterizer, scene::paint_scene_surfaces,
    };
    use doofusdock_presentation::{SceneFrame, SceneNode, SceneRect};
    use egui::{Event, Id, RawInput, TextEdit};

    fn bounds(x: f64, y: f64, width: f64, height: f64) -> SceneRect {
        SceneRect {
            x,
            y,
            width,
            height,
        }
    }

    fn panel_output(
        contexts: &mut SceneEguiPanelContexts,
        panel: &PanelId,
        text: &mut String,
        events: Vec<Event>,
    ) -> FullOutput {
        contexts.run_panel_ui(
            panel,
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(96.0, 48.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                ui.add(TextEdit::singleline(text).id(Id::unique("same-panel-editor")));
                ui.label(text.as_str());
            },
        )
    }

    fn compose(
        context: &Context,
        frame: &SceneFrame,
        renderer: &SceneEguiPanelRenderer,
    ) -> (FullOutput, usize) {
        let mut rendered = 0;
        let output = context.run_ui(
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(192.0, 48.0),
                )),
                ..Default::default()
            },
            |ui| {
                rendered =
                    paint_scene_surfaces(ui.painter(), frame, &renderer.surface_textures(), 4, 9)
                        .unwrap()
                        .rendered;
            },
        );
        (output, rendered)
    }

    fn rasterize_root(
        context: &Context,
        rasterizer: &mut Rasterizer,
        mut output: FullOutput,
        size: [usize; 2],
    ) -> Vec<u32> {
        rasterizer.update(&output.textures_delta).unwrap();
        let jobs = context.tessellate(output.shapes, output.pixels_per_point);
        let mut pixels = vec![0u32; size[0] * size[1]];
        rasterizer
            .paint(&jobs, output.pixels_per_point, size, &mut pixels)
            .unwrap();
        rasterizer.free(&output.textures_delta);
        output.textures_delta.clear();
        pixels
    }

    #[test]
    fn oversized_panel_surface_rejects_before_allocating_texture_state() {
        let panel = PanelId::from("oversized");
        let mut renderer = SceneEguiPanelRenderer::default();
        let mut output = FullOutput::default();
        let result = renderer.update_panel(
            &Context::default(),
            &panel,
            &Context::default(),
            &mut output,
            [MAX_PIXELS + 1, 1],
        );
        assert!(result.is_err());
        assert!(renderer.is_empty());
        assert!(renderer.rasterizers.is_empty());
        assert!(output.textures_delta.set.is_empty());
    }

    #[test]
    fn panel_widget_output_renders_through_revision_bound_scene_surfaces() {
        let left = PanelId::from("left");
        let right = PanelId::from("right");
        let mut contexts = SceneEguiPanelContexts::default();
        let mut renderer = SceneEguiPanelRenderer::default();
        let host_context = Context::default();
        let left_context = contexts.context(&left).clone();
        let right_context = contexts.context(&right).clone();
        let mut left_text = String::new();
        let mut right_text = String::new();
        let mut first_left = panel_output(&mut contexts, &left, &mut left_text, vec![]);
        let mut first_right = panel_output(&mut contexts, &right, &mut right_text, vec![]);
        renderer
            .update_panel(
                &host_context,
                &left,
                &left_context,
                &mut first_left,
                [96, 48],
            )
            .unwrap();
        renderer
            .update_panel(
                &host_context,
                &right,
                &right_context,
                &mut first_right,
                [96, 48],
            )
            .unwrap();

        let mut root = SceneNode::group(bounds(0.0, 0.0, 192.0, 48.0));
        root.children = vec![
            SceneNode::panel(left.clone(), bounds(0.0, 0.0, 96.0, 48.0)),
            SceneNode::panel(right.clone(), bounds(96.0, 0.0, 96.0, 48.0)),
        ];
        let frame = SceneFrame::build(&root, 4, 9).unwrap();
        let (before_output, before_rendered) = compose(&host_context, &frame, &renderer);
        assert_eq!(before_rendered, 2);
        let mut root_rasterizer = Rasterizer::default();
        let before_pixels = rasterize_root(
            &host_context,
            &mut root_rasterizer,
            before_output,
            [192, 48],
        );

        contexts
            .context(&left)
            .memory_mut(|memory| memory.request_focus(Id::unique("same-panel-editor")));
        let mut changed_left = panel_output(
            &mut contexts,
            &left,
            &mut left_text,
            vec![Event::Text("changed".into())],
        );
        let mut unchanged_right = panel_output(&mut contexts, &right, &mut right_text, vec![]);
        assert_eq!(left_text, "changed");
        assert!(right_text.is_empty());
        renderer
            .update_panel(
                &host_context,
                &left,
                &left_context,
                &mut changed_left,
                [96, 48],
            )
            .unwrap();
        renderer
            .update_panel(
                &host_context,
                &right,
                &right_context,
                &mut unchanged_right,
                [96, 48],
            )
            .unwrap();
        let (after_output, after_rendered) = compose(&host_context, &frame, &renderer);
        assert_eq!(after_rendered, 2);
        let after_pixels =
            rasterize_root(&host_context, &mut root_rasterizer, after_output, [192, 48]);
        let mut left_changed = false;
        for row in 0..48 {
            let offset = row * 192;
            left_changed |= before_pixels[offset..offset + 96] != after_pixels[offset..offset + 96];
            assert_eq!(
                &before_pixels[offset + 96..offset + 192],
                &after_pixels[offset + 96..offset + 192],
                "the non-target panel surface changed at row {row}"
            );
        }
        assert!(left_changed, "the edited panel surface did not change");
        assert_eq!(contexts.len(), 2);
        assert_eq!(renderer.len(), 2);
    }
}
