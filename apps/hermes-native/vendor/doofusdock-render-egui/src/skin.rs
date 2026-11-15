//! Nine-slice PNG chrome shared by GPU and software adapters.
use egui::{Color32, Rect, TextureHandle};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Component, Path},
};
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Insets {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkinSpec {
    pub png: String,
    pub insets: Insets,
}
#[derive(Clone)]
pub struct Skin {
    texture: TextureHandle,
    size: [usize; 2],
    insets: Insets,
}
#[derive(Clone, Copy)]
pub enum State {
    Normal,
    Hover,
    Focus,
    Disabled,
}
fn installed_skin_id() -> egui::Id {
    egui::Id::unique("doofusdock-installed-raster-skin")
}
impl Skin {
    pub fn load(ctx: &egui::Context, root: &Path, spec: &SkinSpec) -> Result<Self, String> {
        let relative = Path::new(&spec.png);
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || relative.as_os_str().is_empty()
        {
            return Err("Skin path must be relative without traversal".into());
        }
        let base = root.canonicalize().map_err(|e| format!("Skin root: {e}"))?;
        let path = base
            .join(relative)
            .canonicalize()
            .map_err(|e| format!("Skin PNG: {e}"))?;
        if !path.starts_with(&base) {
            return Err("Skin escapes package root".into());
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Skin PNG exceeds 8 MiB".into());
        }
        let mut reader =
            image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(2048);
        limits.max_image_height = Some(2048);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        let img = reader
            .decode()
            .map_err(|e| format!("Skin decode: {e}"))?
            .to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        validate(size, spec.insets)?;
        let texture = ctx.load_texture(
            "doofusdock-nine-slice",
            egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        Ok(Self {
            texture,
            size,
            insets: spec.insets,
        })
    }
    /// Original 12x12 raster art generated from integer colors; no third-party assets.
    pub fn builtin(ctx: &egui::Context) -> Self {
        let mut pixels = vec![Color32::from_rgb(28, 43, 62); 144];
        for y in 0..12 {
            for x in 0..12 {
                pixels[y * 12 + x] = if x == 0 || y == 0 || x == 11 || y == 11 {
                    Color32::from_rgb(52, 111, 134)
                } else if x < 3 || y < 3 {
                    Color32::from_rgb(39, 66, 87)
                } else {
                    pixels[y * 12 + x]
                };
            }
        }
        let texture = ctx.load_texture(
            "doofusdock-original-skin",
            egui::ColorImage::new([12, 12], pixels),
            egui::TextureOptions::LINEAR,
        );
        Self {
            texture,
            size: [12, 12],
            insets: Insets {
                left: 3,
                top: 3,
                right: 3,
                bottom: 3,
            },
        }
    }
    pub fn paint(&self, painter: &egui::Painter, rect: Rect, state: State) {
        let tint = match state {
            State::Normal => Color32::WHITE,
            State::Hover => Color32::from_rgb(180, 230, 255),
            State::Focus => Color32::from_rgb(150, 255, 200),
            State::Disabled => Color32::from_gray(110),
        };
        for (dest, uv) in slices(rect, self.size, self.insets) {
            painter.image(self.texture.id(), dest, uv, tint);
        }
    }
}
fn validate(size: [usize; 2], i: Insets) -> Result<(), String> {
    if size.contains(&0)
        || size.iter().any(|v| *v > 2048)
        || u64::from(i.left) + u64::from(i.right) >= size[0] as u64
        || u64::from(i.top) + u64::from(i.bottom) >= size[1] as u64
    {
        Err("Invalid nine-slice insets or image dimensions".into())
    } else {
        Ok(())
    }
}
pub fn slices(rect: Rect, size: [usize; 2], i: Insets) -> Vec<(Rect, Rect)> {
    if validate(size, i).is_err() || !rect.is_positive() || !rect.is_finite() {
        return vec![];
    }
    let sx = (rect.width() / (i.left + i.right).max(1) as f32).min(1.0);
    let sy = (rect.height() / (i.top + i.bottom).max(1) as f32).min(1.0);
    let xs = [
        rect.min.x,
        rect.min.x + i.left as f32 * sx,
        rect.max.x - i.right as f32 * sx,
        rect.max.x,
    ];
    let ys = [
        rect.min.y,
        rect.min.y + i.top as f32 * sy,
        rect.max.y - i.bottom as f32 * sy,
        rect.max.y,
    ];
    let us = [
        0.,
        i.left as f32 / size[0] as f32,
        1. - i.right as f32 / size[0] as f32,
        1.,
    ];
    let vs = [
        0.,
        i.top as f32 / size[1] as f32,
        1. - i.bottom as f32 / size[1] as f32,
        1.,
    ];
    let mut result = Vec::with_capacity(9);
    for y in 0..3 {
        for x in 0..3 {
            result.push((
                Rect::from_min_max(egui::pos2(xs[x], ys[y]), egui::pos2(xs[x + 1], ys[y + 1])),
                Rect::from_min_max(egui::pos2(us[x], vs[y]), egui::pos2(us[x + 1], vs[y + 1])),
            ));
        }
    }
    result
}
pub fn install(ctx: &egui::Context, skin: Option<Skin>) {
    ctx.data_mut(|d| d.insert_temp(installed_skin_id(), skin));
}
pub fn paint_panel(ui: &egui::Ui, rect: Rect, radius: f32, color: Color32) {
    let skin = ui
        .ctx()
        .data(|d| d.get_temp::<Option<Skin>>(installed_skin_id()))
        .flatten();
    if let Some(skin) = skin {
        let state = if !ui.is_enabled() {
            State::Disabled
        } else if ui.memory(|m| m.focused().is_some()) {
            State::Focus
        } else if ui.rect_contains_pointer(rect) {
            State::Hover
        } else {
            State::Normal
        };
        skin.paint(ui.painter(), rect, state);
    } else {
        ui.painter().rect_filled(rect, radius, color);
    }
}
/// Install a caller-selected skin, falling back visibly to vector chrome on error.
pub fn configure(ctx: &egui::Context, builtin: bool, package: Option<&Path>) -> Option<String> {
    let result = if let Some(root) = package {
        (|| -> Result<Skin, String> {
            let mut bytes = Vec::new();
            std::fs::File::open(root.join("skin.json"))
                .map_err(|e| e.to_string())?
                .take(16385)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 16384 {
                return Err("Skin manifest exceeds 16 KiB".into());
            }
            let spec: SkinSpec = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            Skin::load(ctx, root, &spec)
        })()
        .map(Some)
    } else if builtin {
        Ok(Some(Skin::builtin(ctx)))
    } else {
        Ok(None)
    };
    match result {
        Ok(skin) => {
            install(ctx, skin);
            None
        }
        Err(error) => {
            install(ctx, None);
            Some(format!("Skin unavailable; using vector chrome: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestPackage(PathBuf);
    impl TestPackage {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "doofusdock-skin-test-{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, bytes: &[u8]) {
            let destination = self.0.join(name);
            let temporary = self.0.join(format!("{name}.tmp"));
            let mut file = fs::File::create(&temporary).unwrap();
            use std::io::Write;
            file.write_all(bytes).unwrap();
            file.sync_all().unwrap();
            fs::rename(temporary, destination).unwrap();
        }
    }
    impl Drop for TestPackage {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn skin_spec(png: &str, insets: Insets) -> Vec<u8> {
        serde_json::to_vec(&SkinSpec {
            png: png.into(),
            insets,
        })
        .unwrap()
    }

    fn draw_installed_skin(ctx: &egui::Context) -> egui::FullOutput {
        let panel = Rect::from_min_size(egui::pos2(2.0, 2.0), egui::vec2(40.0, 36.0));
        ctx.run_ui(egui::RawInput::default(), &mut |ui: &mut egui::Ui| {
            paint_panel(ui, panel, 3.0, Color32::from_rgb(7, 19, 31));
        })
    }

    #[test]
    fn nine_slices_cover_small_and_large_destinations() {
        for s in [2., 40., 400.] {
            let r = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(s, s));
            let parts = slices(
                r,
                [12, 12],
                Insets {
                    left: 3,
                    top: 3,
                    right: 3,
                    bottom: 3,
                },
            );
            assert_eq!(parts.len(), 9);
            let area: f32 = parts.iter().map(|(d, _)| d.area()).sum();
            assert!((area - s * s).abs() < 0.01);
            for (d, uv) in parts {
                assert!(d.width() >= 0. && d.height() >= 0.);
                assert!(uv.min.x >= 0. && uv.max.x <= 1.);
            }
        }
    }
    #[test]
    fn rejects_bad_insets_and_traversal_before_io() {
        assert!(
            validate(
                [12, 12],
                Insets {
                    left: u32::MAX,
                    top: 3,
                    right: 3,
                    bottom: 3
                }
            )
            .is_err()
        );
        let spec = SkinSpec {
            png: "../escape.png".into(),
            insets: Insets {
                left: 1,
                top: 1,
                right: 1,
                bottom: 1,
            },
        };
        assert!(
            Skin::load(&egui::Context::default(), Path::new("."), &spec)
                .err()
                .unwrap()
                .contains("traversal")
        );
    }

    #[test]
    fn valid_package_skin_is_retrieved_and_rasterized_by_the_shared_mesh_path() {
        let package = TestPackage::new();
        let image = image::RgbaImage::from_pixel(12, 12, image::Rgba([240, 80, 24, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        package.write("chrome.png", png.get_ref());
        package.write(
            "skin.json",
            &skin_spec(
                "chrome.png",
                Insets {
                    left: 3,
                    top: 3,
                    right: 3,
                    bottom: 3,
                },
            ),
        );

        let ctx = egui::Context::default();
        assert!(configure(&ctx, false, Some(&package.0)).is_none());
        let mut output = draw_installed_skin(&ctx);
        assert!(!output.textures_delta.set.is_empty());
        let shapes = std::mem::take(&mut output.shapes);
        let jobs = ctx.tessellate(shapes, output.pixels_per_point);
        assert!(
            jobs.iter()
                .any(|job| matches!(job.primitive, egui::epaint::Primitive::Mesh(_)))
        );

        let mut raster = crate::raster::Rasterizer::default();
        raster.update(&output.textures_delta).unwrap();
        let mut pixels = vec![0; 64 * 64];
        raster.paint(&jobs, 1.0, [64, 64], &mut pixels).unwrap();
        assert!(pixels.iter().any(|pixel| *pixel != 0x10151d));
        output.textures_delta.clear();
    }

    #[test]
    fn invalid_package_skin_reports_reason_and_paints_vector_fallback() {
        let package = TestPackage::new();
        package.write("chrome.png", b"not a PNG");
        package.write(
            "skin.json",
            &skin_spec(
                "chrome.png",
                Insets {
                    left: 3,
                    top: 3,
                    right: 3,
                    bottom: 3,
                },
            ),
        );
        let ctx = egui::Context::default();
        let diagnostic = configure(&ctx, false, Some(&package.0)).unwrap();
        assert!(diagnostic.contains("using vector chrome"));
        assert!(diagnostic.contains("Skin decode"));

        let mut output = draw_installed_skin(&ctx);
        let target = Rect::from_min_size(egui::pos2(2.0, 2.0), egui::vec2(40.0, 36.0));
        assert!(output.shapes.iter().any(|clipped| match &clipped.shape {
            egui::epaint::Shape::Rect(rect) => rect.rect == target,
            _ => false,
        }));
        output.textures_delta.clear();
    }

    #[test]
    fn package_manifest_and_dimensions_are_bounded_with_explicit_fallback_diagnostics() {
        let package = TestPackage::new();
        package.write("skin.json", &vec![b' '; 16 * 1024 + 1]);
        let ctx = egui::Context::default();
        let diagnostic = configure(&ctx, false, Some(&package.0)).unwrap();
        assert!(diagnostic.contains("using vector chrome"));
        assert!(diagnostic.contains("Skin manifest exceeds 16 KiB"));

        let oversized = TestPackage::new();
        let image = image::RgbaImage::new(2049, 1);
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        oversized.write("chrome.png", png.get_ref());
        oversized.write(
            "skin.json",
            &skin_spec(
                "chrome.png",
                Insets {
                    left: 1,
                    top: 0,
                    right: 1,
                    bottom: 0,
                },
            ),
        );
        let diagnostic = configure(&ctx, false, Some(&oversized.0)).unwrap();
        assert!(diagnostic.contains("using vector chrome"));
        assert!(diagnostic.contains("Skin decode"));

        let oversized_file = TestPackage::new();
        oversized_file.write("chrome.png", &vec![0u8; 8 * 1024 * 1024 + 1]);
        oversized_file.write(
            "skin.json",
            &skin_spec(
                "chrome.png",
                Insets {
                    left: 1,
                    top: 1,
                    right: 1,
                    bottom: 1,
                },
            ),
        );
        let diagnostic = configure(&ctx, false, Some(&oversized_file.0)).unwrap();
        assert!(diagnostic.contains("using vector chrome"));
        assert!(diagnostic.contains("Skin PNG exceeds 8 MiB"));
    }
}
