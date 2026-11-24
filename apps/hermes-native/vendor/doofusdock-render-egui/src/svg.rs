//! Bounded SVG-to-egui rasterization for vector visuals.
//!
//! Parsing uses `usvg` through `resvg` with a resource resolver that rejects
//! external string references and bounds embedded raster resources. This module
//! creates pixels only and does not execute SVG scripts or perform network or
//! filesystem resource discovery.

use doofusdock_presentation::{Easing, Tween, TweenValue};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    sync::Arc,
};

const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_STYLESHEET_BYTES: usize = 64 * 1024;
const MAX_RENDER_DIMENSION: u32 = 4096;
const MAX_RENDER_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_EMBEDDED_IMAGE_DIMENSION: usize = 8192;
const MAX_EMBEDDED_IMAGE_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_SVG_ANIMATION_TRACKS: usize = 128;

/// Error returned when an SVG cannot be safely loaded or rasterized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgError {
    pub message: String,
    pub context: String,
}

impl SvgError {
    fn new(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            context: context.into(),
        }
    }
}

impl std::fmt::Display for SvgError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} [{}]", self.message, self.context)
    }
}

impl std::error::Error for SvgError {}

/// Raster output size and an optional CSS stylesheet applied during SVG parse.
/// Re-rendering with a different stylesheet supports dynamic visual states.
#[derive(Clone, Copy, Debug)]
pub struct SvgRenderOptions<'a> {
    /// Output pixel dimensions. The caller chooses these for its DPI scale.
    pub output_size: [u32; 2],
    /// Optional bounded stylesheet, for example a theme state override.
    pub style_sheet: Option<&'a str>,
}

/// Supported SVG properties for a typed, seekable style tween.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SvgAnimatedProperty {
    Fill,
    Stroke,
    Color,
    Opacity,
    StrokeWidth,
}

impl SvgAnimatedProperty {
    fn css_name(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Stroke => "stroke",
            Self::Color => "color",
            Self::Opacity => "opacity",
            Self::StrokeWidth => "stroke-width",
        }
    }
}

/// Typed endpoints for the SVG properties animated by this module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SvgAnimatedValue {
    Color(egui::Color32),
    Number(f64),
}

impl TweenValue for SvgAnimatedValue {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        let t = progress.clamp(0.0, 1.0);
        match (from, to) {
            (Self::Color(from), Self::Color(to)) => {
                let from = from.to_srgba_unmultiplied();
                let to = to.to_srgba_unmultiplied();
                let channel = |index: usize| {
                    (f64::from(from[index]) + (f64::from(to[index]) - f64::from(from[index])) * t)
                        .round()
                        .clamp(0.0, 255.0) as u8
                };
                Self::Color(egui::Color32::from_rgba_unmultiplied(
                    channel(0),
                    channel(1),
                    channel(2),
                    channel(3),
                ))
            }
            (Self::Number(from), Self::Number(to)) => Self::Number(from + (to - from) * t),
            _ => *from,
        }
    }
}

/// Stable-ID-targeted SVG style transition sampled by the presentation Tween.
/// Selector text is generated from the validated ID, never accepted as raw CSS.
#[derive(Clone, Copy, Debug)]
pub struct SvgStyleTween<'a> {
    pub target_id: &'a str,
    pub property: SvgAnimatedProperty,
    pub from: SvgAnimatedValue,
    pub to: SvgAnimatedValue,
    pub start_seconds: f64,
    pub duration_seconds: f64,
    pub easing: Easing,
    pub reduced_motion: bool,
}

impl SvgStyleTween<'_> {
    fn stylesheet_at(&self, time_seconds: f64, base: Option<&str>) -> Result<String, SvgError> {
        if !valid_svg_id(self.target_id) {
            return Err(SvgError::new(
                "SVG animation target must be a simple stable element ID.",
                "target_id",
            ));
        }
        let values_match = matches!(
            (self.property, self.from, self.to),
            (
                SvgAnimatedProperty::Fill
                    | SvgAnimatedProperty::Stroke
                    | SvgAnimatedProperty::Color,
                SvgAnimatedValue::Color(_),
                SvgAnimatedValue::Color(_)
            ) | (
                SvgAnimatedProperty::Opacity,
                SvgAnimatedValue::Number(_),
                SvgAnimatedValue::Number(_)
            ) | (
                SvgAnimatedProperty::StrokeWidth,
                SvgAnimatedValue::Number(_),
                SvgAnimatedValue::Number(_)
            )
        );
        if !values_match {
            return Err(SvgError::new(
                "SVG animation endpoint types do not match the selected property.",
                "property",
            ));
        }
        for value in [self.from, self.to] {
            if let SvgAnimatedValue::Number(number) = value {
                let max = match self.property {
                    SvgAnimatedProperty::Opacity => 1.0,
                    SvgAnimatedProperty::StrokeWidth => 256.0,
                    _ => 0.0,
                };
                if !number.is_finite() || number < 0.0 || number > max {
                    return Err(SvgError::new(
                        "SVG animation number is outside the property's supported range.",
                        "value",
                    ));
                }
            }
        }
        let tween = Tween::new(
            self.from,
            self.to,
            self.start_seconds,
            self.duration_seconds,
            self.easing,
            self.reduced_motion,
        )
        .map_err(|error| SvgError::new(error, "timing"))?;
        let value = tween
            .sample(time_seconds)
            .map_err(|error| SvgError::new(error, "time"))?;
        let css_value = match value {
            SvgAnimatedValue::Color(color) => {
                let [r, g, b, a] = color.to_srgba_unmultiplied();
                format!("rgba({r}, {g}, {b}, {:.6})", f64::from(a) / 255.0)
            }
            SvgAnimatedValue::Number(number) => format!("{number:.6}"),
        };
        let animated_rule = format!(
            "#{} {{ {}: {} !important; }}",
            self.target_id,
            self.property.css_name(),
            css_value
        );
        let stylesheet = match base {
            Some(base) if !base.is_empty() => format!("{base}\n{animated_rule}"),
            _ => animated_rule,
        };
        if stylesheet.len() > MAX_STYLESHEET_BYTES {
            return Err(SvgError::new(
                "Combined SVG stylesheet exceeds the 64 KiB limit.",
                "style_sheet",
            ));
        }
        Ok(stylesheet)
    }
}

fn valid_svg_id(id: &str) -> bool {
    let mut bytes = id.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Rasterize an SVG at one deterministic point in a typed style transition.
/// Sampling uses the presentation crate's bounded `Tween` and accepts absolute
/// time, so callers can scrub, reverse, or re-render individual frames.
pub fn rasterize_svg_animated(
    source: &str,
    options: SvgRenderOptions<'_>,
    animation: SvgStyleTween<'_>,
    time_seconds: f64,
) -> Result<egui::ColorImage, SvgError> {
    rasterize_svg_timeline(source, options, &[animation], time_seconds)
}

/// Rasterize multiple independently timed property tracks over one SVG sample.
/// Duplicate `(element ID, property)` tracks are rejected; at most 128 tracks
/// and the existing 64 KiB combined stylesheet bound are allowed.
pub fn rasterize_svg_timeline(
    source: &str,
    options: SvgRenderOptions<'_>,
    tracks: &[SvgStyleTween<'_>],
    time_seconds: f64,
) -> Result<egui::ColorImage, SvgError> {
    if tracks.len() > MAX_SVG_ANIMATION_TRACKS {
        return Err(SvgError::new(
            "SVG animation exceeds the 128-track limit.",
            "tracks",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut stylesheet = options.style_sheet.unwrap_or_default().to_owned();
    for track in tracks {
        if !seen.insert((track.target_id, track.property)) {
            return Err(SvgError::new(
                "SVG animation has duplicate tracks for the same element property.",
                "tracks",
            ));
        }
        let rule = track.stylesheet_at(time_seconds, None)?;
        if !stylesheet.is_empty() {
            stylesheet.push('\n');
        }
        stylesheet.push_str(&rule);
        if stylesheet.len() > MAX_STYLESHEET_BYTES {
            return Err(SvgError::new(
                "Combined SVG stylesheet exceeds the 64 KiB limit.",
                "style_sheet",
            ));
        }
    }
    let style_sheet = (!stylesheet.is_empty()).then_some(stylesheet.as_str());
    rasterize_svg(
        source,
        SvgRenderOptions {
            output_size: options.output_size,
            style_sheet,
        },
    )
}

/// Parse an inline or generated SVG and return a premultiplied egui image.
///
/// The SVG is contained in the requested canvas while preserving its aspect
/// ratio. External `<image>` resources are ignored by the nested parser.
pub fn rasterize_svg(
    source: &str,
    options: SvgRenderOptions<'_>,
) -> Result<egui::ColorImage, SvgError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(SvgError::new(
            "SVG source exceeds the 2 MiB limit.",
            "source",
        ));
    }
    validate_output(options.output_size)?;
    if options
        .style_sheet
        .is_some_and(|style_sheet| style_sheet.len() > MAX_STYLESHEET_BYTES)
    {
        return Err(SvgError::new(
            "SVG stylesheet exceeds the 64 KiB limit.",
            "style_sheet",
        ));
    }

    let parser_options = resvg::usvg::Options {
        style_sheet: options.style_sheet.map(str::to_owned),
        image_href_resolver: resvg::usvg::ImageHrefResolver {
            resolve_data: Box::new(|mime, data: Arc<Vec<u8>>, _| {
                if data.len() > MAX_SOURCE_BYTES {
                    return None;
                }
                let size = imagesize::blob_size(&data).ok()?;
                let pixels = (size.width as u64).checked_mul(size.height as u64)?;
                if size.width == 0
                    || size.height == 0
                    || size.width > MAX_EMBEDDED_IMAGE_DIMENSION
                    || size.height > MAX_EMBEDDED_IMAGE_DIMENSION
                    || pixels > MAX_EMBEDDED_IMAGE_PIXELS
                {
                    return None;
                }
                match mime {
                    "image/jpeg" | "image/jpg" => Some(resvg::usvg::ImageKind::JPEG(data)),
                    "image/png" => Some(resvg::usvg::ImageKind::PNG(data)),
                    "image/gif" => Some(resvg::usvg::ImageKind::GIF(data)),
                    "image/webp" => Some(resvg::usvg::ImageKind::WEBP(data)),
                    _ => None,
                }
            }),
            resolve_string: Box::new(|_, _| None),
        },
        ..resvg::usvg::Options::default()
    };
    let tree = resvg::usvg::Tree::from_data(source.as_bytes(), &parser_options)
        .map_err(|error| SvgError::new(format!("SVG parse failed: {error}"), "source"))?;
    let source_size = tree.size();
    let source_width = source_size.width();
    let source_height = source_size.height();
    if !source_width.is_finite()
        || !source_height.is_finite()
        || source_width <= 0.0
        || source_height <= 0.0
    {
        return Err(SvgError::new(
            "SVG has an invalid intrinsic size.",
            "viewBox",
        ));
    }

    let [width, height] = options.output_size;
    let scale = (width as f32 / source_width).min(height as f32 / source_height);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(SvgError::new(
            "SVG transform is outside the supported range.",
            "output_size",
        ));
    }
    let drawn_width = source_width * scale;
    let drawn_height = source_height * scale;
    let translate_x = (width as f32 - drawn_width) * 0.5;
    let translate_y = (height as f32 - drawn_height) * 0.5;
    let transform =
        resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, translate_x, translate_y);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| SvgError::new("Could not allocate bounded SVG output.", "output_size"))?;
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(egui::ColorImage::from_rgba_premultiplied(
        [width as usize, height as usize],
        pixmap.data(),
    ))
}

/// Read and rasterize a selected SVG file without following a symlink at the
/// file path. SVG-linked external resources remain disabled by the resolver.
pub fn rasterize_svg_file(
    path: &Path,
    options: SvgRenderOptions<'_>,
) -> Result<egui::ColorImage, SvgError> {
    let source = read_svg_file(path)?;
    rasterize_svg(&source, options)
}

/// Rasterize a bounded regular SVG file at a typed animation time sample.
pub fn rasterize_svg_file_animated(
    path: &Path,
    options: SvgRenderOptions<'_>,
    animation: SvgStyleTween<'_>,
    time_seconds: f64,
) -> Result<egui::ColorImage, SvgError> {
    rasterize_svg_file_timeline(path, options, &[animation], time_seconds)
}

/// Rasterize a bounded regular SVG file with multiple timed style tracks.
pub fn rasterize_svg_file_timeline(
    path: &Path,
    options: SvgRenderOptions<'_>,
    tracks: &[SvgStyleTween<'_>],
    time_seconds: f64,
) -> Result<egui::ColorImage, SvgError> {
    let source = read_svg_file(path)?;
    rasterize_svg_timeline(&source, options, tracks, time_seconds)
}

fn read_svg_file(path: &Path) -> Result<String, SvgError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        SvgError::new(
            format!("Could not inspect SVG file: {error}"),
            path.display().to_string(),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SvgError::new(
            "SVG path must be a regular file, not a symbolic link.",
            path.display().to_string(),
        ));
    }
    if metadata.len() > MAX_SOURCE_BYTES as u64 {
        return Err(SvgError::new(
            "SVG file exceeds the 2 MiB limit.",
            path.display().to_string(),
        ));
    }
    let file = File::open(path).map_err(|error| {
        SvgError::new(
            format!("Could not open SVG file: {error}"),
            path.display().to_string(),
        )
    })?;
    let mut bytes = Vec::new();
    file.take((MAX_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            SvgError::new(
                format!("Could not read SVG file: {error}"),
                path.display().to_string(),
            )
        })?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(SvgError::new(
            "SVG file exceeds the 2 MiB limit.",
            path.display().to_string(),
        ));
    }
    std::str::from_utf8(&bytes)
        .map(str::to_owned)
        .map_err(|error| {
            SvgError::new(
                format!("SVG file is not UTF-8: {error}"),
                path.display().to_string(),
            )
        })
}

fn validate_output([width, height]: [u32; 2]) -> Result<(), SvgError> {
    let pixel_count = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > MAX_RENDER_DIMENSION
        || height > MAX_RENDER_DIMENSION
        || pixel_count > MAX_RENDER_PIXELS
    {
        return Err(SvgError::new(
            "SVG output dimensions must be non-zero, at most 4096 per side, and at most 16 Mi pixels.",
            format!("{width}x{height}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn options(style_sheet: Option<&str>) -> SvgRenderOptions<'_> {
        SvgRenderOptions {
            output_size: [48, 48],
            style_sheet,
        }
    }

    #[test]
    fn inline_svg_rasterizes_with_transparency_and_dynamic_stylesheet() {
        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12">
            <rect id="mark" x="0" y="0" width="24" height="12"/>
        </svg>"##;
        let base =
            rasterize_svg(source, options(Some("rect { fill: #0000ff !important; }"))).unwrap();
        let styled =
            rasterize_svg(source, options(Some("rect { fill: #ff0000 !important; }"))).unwrap();
        assert_eq!(base.size, [48, 48]);
        assert_eq!(base.pixels[24 * 48 + 24], egui::Color32::BLUE);
        assert_eq!(styled.pixels[24 * 48 + 24], egui::Color32::RED);
        assert_eq!(base.pixels[0], egui::Color32::TRANSPARENT);
    }

    #[test]
    fn svg_style_animation_uses_bounded_seekable_presentation_tween() {
        use doofusdock_presentation::Easing;

        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
            <rect id="animated" width="24" height="24" fill="#000000"/>
        </svg>"##;
        let animation = SvgStyleTween {
            target_id: "animated",
            property: SvgAnimatedProperty::Fill,
            from: SvgAnimatedValue::Color(egui::Color32::RED),
            to: SvgAnimatedValue::Color(egui::Color32::BLUE),
            start_seconds: 0.0,
            duration_seconds: 2.0,
            easing: Easing::Linear,
            reduced_motion: false,
        };
        let at = |time| {
            rasterize_svg_animated(source, options(None), animation, time)
                .unwrap()
                .pixels[24 * 48 + 24]
        };
        assert_eq!(at(0.0), egui::Color32::RED);
        assert_eq!(at(1.0), egui::Color32::from_rgb(128, 0, 128));
        assert_eq!(at(2.0), egui::Color32::BLUE);
        assert_eq!(at(1.0), at(1.0));

        let reduced = SvgStyleTween {
            reduced_motion: true,
            ..animation
        };
        assert_eq!(
            rasterize_svg_animated(source, options(None), reduced, 0.0)
                .unwrap()
                .pixels[24 * 48 + 24],
            egui::Color32::BLUE
        );
        let hostile_selector = SvgStyleTween {
            target_id: "animated } rect { display: none",
            ..animation
        };
        assert!(rasterize_svg_animated(source, options(None), hostile_selector, 0.5).is_err());
    }

    #[test]
    fn svg_style_timeline_composes_tracks_and_rejects_ambiguous_work() {
        use doofusdock_presentation::Easing;

        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
            <rect id="left" x="0" y="0" width="12" height="24"/>
            <rect id="right" x="12" y="0" width="12" height="24"/>
        </svg>"##;
        let tracks = [
            SvgStyleTween {
                target_id: "left",
                property: SvgAnimatedProperty::Fill,
                from: SvgAnimatedValue::Color(egui::Color32::BLACK),
                to: SvgAnimatedValue::Color(egui::Color32::RED),
                start_seconds: 0.0,
                duration_seconds: 2.0,
                easing: Easing::Linear,
                reduced_motion: false,
            },
            SvgStyleTween {
                target_id: "right",
                property: SvgAnimatedProperty::Fill,
                from: SvgAnimatedValue::Color(egui::Color32::BLACK),
                to: SvgAnimatedValue::Color(egui::Color32::BLUE),
                start_seconds: 0.0,
                duration_seconds: 2.0,
                easing: Easing::Linear,
                reduced_motion: false,
            },
        ];
        let image = rasterize_svg_timeline(source, options(None), &tracks, 2.0).unwrap();
        assert_eq!(image.pixels[24 * 48 + 12], egui::Color32::RED);
        assert_eq!(image.pixels[24 * 48 + 36], egui::Color32::BLUE);
        assert_eq!(
            rasterize_svg_timeline(source, options(None), &tracks, 1.0).unwrap(),
            rasterize_svg_timeline(source, options(None), &tracks, 1.0).unwrap()
        );
        assert!(
            rasterize_svg_timeline(source, options(None), &[tracks[0], tracks[0]], 1.0).is_err()
        );
        assert!(
            rasterize_svg_timeline(
                source,
                options(None),
                &vec![tracks[0]; MAX_SVG_ANIMATION_TRACKS + 1],
                1.0
            )
            .is_err()
        );
    }

    #[test]
    fn svg_parser_ignores_external_image_resources() {
        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
            <image href="file:///this-path-must-never-be-read.png" width="24" height="24"/>
        </svg>"##;
        let image = rasterize_svg(source, options(None)).unwrap();
        assert!(
            image
                .pixels
                .iter()
                .all(|pixel| *pixel == egui::Color32::TRANSPARENT)
        );
    }

    #[test]
    fn svg_composes_bounded_embedded_raster_assets() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use image::ImageEncoder as _;

        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&[0, 255, 0, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        let encoded = STANDARD.encode(png);
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
                <image href="data:image/png;base64,{encoded}" width="24" height="24"/>
            </svg>"#
        );
        let image = rasterize_svg(&source, options(None)).unwrap();
        assert_eq!(image.pixels[24 * 48 + 24], egui::Color32::GREEN);

        let mut oversized_png = Vec::new();
        let oversized_rgba = vec![0; (MAX_EMBEDDED_IMAGE_DIMENSION + 1) * 4];
        image::codecs::png::PngEncoder::new(&mut oversized_png)
            .write_image(
                &oversized_rgba,
                (MAX_EMBEDDED_IMAGE_DIMENSION + 1) as u32,
                1,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        let oversized_source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
                <image href="data:image/png;base64,{}" width="24" height="24"/>
            </svg>"#,
            STANDARD.encode(oversized_png)
        );
        let blank = rasterize_svg(&oversized_source, options(None)).unwrap();
        assert!(
            blank
                .pixels
                .iter()
                .all(|pixel| *pixel == egui::Color32::TRANSPARENT)
        );
    }

    #[test]
    fn svg_input_and_output_limits_fail_before_raster_allocation() {
        let too_large = " ".repeat(MAX_SOURCE_BYTES + 1);
        assert!(rasterize_svg(&too_large, options(None)).is_err());
        assert!(
            rasterize_svg(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>",
                SvgRenderOptions {
                    output_size: [0, 1],
                    style_sheet: None,
                }
            )
            .is_err()
        );
        assert!(validate_output([4096, 4096]).is_ok());
        assert!(validate_output([4096, 4097]).is_err());
    }

    #[test]
    fn file_backed_svg_is_bounded_and_rejects_symlink_targets() {
        let directory = std::env::temp_dir().join(format!(
            "doofusdock-svg-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("generated.svg");
        fs::write(
            &path,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"><rect id=\"mark\" width=\"4\" height=\"4\" fill=\"#00ff00\"/></svg>",
        )
        .unwrap();
        let image = rasterize_svg_file(&path, options(None)).unwrap();
        assert_eq!(image.pixels[24 * 48 + 24], egui::Color32::GREEN);
        let animated = rasterize_svg_file_animated(
            &path,
            options(None),
            SvgStyleTween {
                target_id: "mark",
                property: SvgAnimatedProperty::Fill,
                from: SvgAnimatedValue::Color(egui::Color32::RED),
                to: SvgAnimatedValue::Color(egui::Color32::BLUE),
                start_seconds: 0.0,
                duration_seconds: 1.0,
                easing: doofusdock_presentation::Easing::Linear,
                reduced_motion: false,
            },
            1.0,
        )
        .unwrap();
        assert_eq!(animated.pixels[24 * 48 + 24], egui::Color32::BLUE);

        let link = directory.join("linked.svg");
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_file(&path, &link);
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&path, &link);
        if link_result.is_ok() {
            assert!(rasterize_svg_file(&link, options(None)).is_err());
            assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        }
        assert!(rasterize_svg_file(&directory, options(None)).is_err());

        let oversized_path = directory.join("oversized.svg");
        fs::write(&oversized_path, vec![b' '; MAX_SOURCE_BYTES + 1]).unwrap();
        assert!(rasterize_svg_file(&oversized_path, options(None)).is_err());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn persisted_theme_svg_asset_resolves_and_rasterizes_headlessly() {
        use doofusdock_core::{
            ThemeAdapterCapabilities, ThemeAsset, ThemeAssetFormat, ThemeAssetFormatName,
            ThemeManifest, ThemeSvgStyleBinding, ThemeSvgStyleProperty, ThemeTiling,
            ThemeTransparency, ThemeValue, inspect_theme_assets, resolve_theme,
        };
        use std::collections::{BTreeMap, BTreeSet};

        let directory = std::env::temp_dir().join(format!(
            "doofusdock-theme-svg-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(directory.join("assets")).unwrap();
        let path = directory.join("assets/mark.svg");
        fs::write(
            &path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" fill="#00ff00"/></svg>"##,
        )
        .unwrap();

        let mut manifest = ThemeManifest::parse_json(include_str!(
            "../../../themes/doofusdock-workbench/theme.json"
        ))
        .expect("bundled theme is a valid source manifest");
        manifest.assets.push(ThemeAsset {
            id: "vector-mark".into(),
            path: "assets/mark.svg".into(),
            format: ThemeAssetFormat::Svg,
            source_width: 24,
            source_height: 24,
            scale: 1.0,
            transparency: ThemeTransparency::Alpha,
            color_key: None,
            nine_slice: None,
            edge_tiling: ThemeTiling::Stretch,
            center_tiling: ThemeTiling::Stretch,
            provenance: None,
        });
        let grip = &mut manifest
            .components
            .get_mut("splitter")
            .unwrap()
            .states
            .get_mut("grip")
            .unwrap();
        grip.tokens.insert(
            "svg.fill".into(),
            ThemeValue::Reference("color.accent".into()),
        );
        grip.svg_bindings.push(ThemeSvgStyleBinding {
            asset_id: "vector-mark".into(),
            selector: "rect".into(),
            property: ThemeSvgStyleProperty::Fill,
            token: "svg.fill".into(),
        });
        manifest.validate().unwrap();
        let report = inspect_theme_assets(&manifest, &directory).unwrap();
        assert!(report.is_verified("vector-mark"));
        let capabilities = ThemeAdapterCapabilities {
            supported_features: BTreeSet::from(["custom_chrome".into()]),
            supported_asset_formats: BTreeSet::from([ThemeAssetFormatName::Svg]),
            supports_raster_assets: false,
        };
        let resolved = resolve_theme(
            &manifest,
            "dark",
            &BTreeMap::new(),
            &capabilities,
            Some(&report),
        )
        .unwrap();
        assert!(
            resolved
                .assets
                .iter()
                .any(|asset| asset.id == "vector-mark" && asset.enabled)
        );

        let style_sheet = resolved
            .svg_stylesheet("vector-mark", "splitter", "grip")
            .unwrap();
        assert_eq!(style_sheet, "rect { fill: #F28C28; }\n");
        let image = rasterize_svg_file(
            &path,
            SvgRenderOptions {
                output_size: [48, 48],
                style_sheet: Some(&style_sheet),
            },
        )
        .unwrap();
        assert_eq!(
            image.pixels[24 * 48 + 24],
            egui::Color32::from_rgb(242, 140, 40)
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
