use egui::{ColorImage, Context, Id};
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

const STAGING_DIRECTORY: &str = "clipboard-images";
const STAGED_FILE_PREFIX: &str = "clipboard-hermes-native-clipboard-image-";
const DROPPED_FILE_DIRECTORY: &str = "dropped-files";
const DROPPED_FILE_DIRECTORY_PREFIX: &str = "drop-";
const MAX_PIXELS: usize = 16 * 1024 * 1024;
const MAX_PNG_BYTES: usize = 16 * 1024 * 1024;
const MAX_DROPPED_FILE_BYTES: usize = 16 * 1024 * 1024;

pub fn take_focused_image_event(ctx: &Context, widget_id: Id) -> Option<Arc<ColorImage>> {
    if ctx.memory(|memory| memory.focused()) != Some(widget_id) {
        return None;
    }
    ctx.input_mut(|input| {
        let position = input
            .raw
            .events
            .iter()
            .position(|event| matches!(event, egui::Event::PasteImage(_)))?;
        match input.raw.events.remove(position) {
            egui::Event::PasteImage(image) => Some(image),
            _ => None,
        }
    })
}

pub fn stage_png(
    data_directory: &Path,
    operation_id: &str,
    image: &ColorImage,
) -> Result<PathBuf, String> {
    if operation_id.is_empty()
        || operation_id.len() > 160
        || !operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Clipboard image staging ID is invalid".into());
    }
    let [width, height] = image.size;
    let pixel_count = width
        .checked_mul(height)
        .filter(|count| *count > 0 && *count <= MAX_PIXELS)
        .ok_or_else(|| "Clipboard image exceeds the 16 megapixel limit".to_owned())?;
    if width > u32::MAX as usize || height > u32::MAX as usize || image.pixels.len() != pixel_count
    {
        return Err("Clipboard image dimensions or pixel data are invalid".into());
    }

    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    let rgba = RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| "Clipboard image pixel data could not be decoded".to_owned())?;
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(rgba)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|error| format!("Could not encode clipboard image as PNG: {error}"))?;
    if png.len() > MAX_PNG_BYTES {
        return Err("Clipboard image is larger than Hermes' 16 MiB attachment limit".into());
    }

    let directory = data_directory.join(STAGING_DIRECTORY);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create clipboard image staging directory: {error}"))?;
    let destination = directory.join(format!("{STAGED_FILE_PREFIX}{operation_id}.png"));
    let temporary = directory.join(format!("{STAGED_FILE_PREFIX}{operation_id}.tmp"));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Could not create clipboard image staging file: {error}"))?;
        file.write_all(&png)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("Could not persist clipboard image: {error}"))?;
        let bytes = fs::read(&temporary)
            .map_err(|error| format!("Could not verify staged clipboard image: {error}"))?;
        let decoded = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
            .map_err(|error| format!("Staged clipboard image failed PNG validation: {error}"))?;
        if decoded.width() != width as u32 || decoded.height() != height as u32 {
            return Err("Staged clipboard image dimensions changed during encoding".into());
        }
        if bytes.len() > MAX_PNG_BYTES {
            return Err("Staged clipboard image exceeds Hermes' 16 MiB limit".into());
        }
        fs::rename(&temporary, &destination).map_err(|error| {
            format!("Could not atomically publish staged clipboard image: {error}")
        })
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result?;
    Ok(destination)
}

pub fn stage_dropped_file_bytes(
    data_directory: &Path,
    operation_id: &str,
    supplied_name: &str,
    mime: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    if operation_id.is_empty()
        || operation_id.len() > 160
        || !operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Dropped-file staging ID is invalid".into());
    }
    if bytes.len() > MAX_DROPPED_FILE_BYTES {
        return Err("Dropped file is larger than Hermes' 16 MiB attachment limit".into());
    }
    let name = sanitized_dropped_file_name(supplied_name, mime, bytes);
    let staging_root = data_directory.join(DROPPED_FILE_DIRECTORY);
    let directory = staging_root.join(format!("{DROPPED_FILE_DIRECTORY_PREFIX}{operation_id}"));
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create dropped-file staging directory: {error}"))?;
    let destination = directory.join(&name);
    let temporary = directory.join(format!(".{name}.tmp"));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Could not create dropped-file staging file: {error}"))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("Could not persist dropped file: {error}"))?;
        let readback = fs::read(&temporary)
            .map_err(|error| format!("Could not verify staged dropped file: {error}"))?;
        if readback.len() > MAX_DROPPED_FILE_BYTES || readback != bytes {
            return Err("Staged dropped file failed byte or size validation".into());
        }
        fs::rename(&temporary, &destination)
            .map_err(|error| format!("Could not atomically publish staged dropped file: {error}"))
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
        let _ = fs::remove_dir(&directory);
    }
    write_result?;
    Ok(destination)
}

fn sanitized_dropped_file_name(supplied_name: &str, mime: &str, bytes: &[u8]) -> String {
    let basename = supplied_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    let mut name = basename
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
            {
                '_'
            } else {
                character
            }
        })
        .take(180)
        .collect::<String>();
    name = name.trim_end_matches([' ', '.']).to_owned();
    if name.is_empty() || name == "." || name == ".." {
        name = "attachment".into();
    }
    if Path::new(&name).extension().is_none() {
        let extension = match mime.to_ascii_lowercase().as_str() {
            "image/png" => Some("png"),
            "image/jpeg" => Some("jpg"),
            "image/gif" => Some("gif"),
            "image/webp" => Some("webp"),
            "image/bmp" => Some("bmp"),
            _ => image::guess_format(bytes)
                .ok()
                .and_then(|format| match format {
                    ImageFormat::Png => Some("png"),
                    ImageFormat::Jpeg => Some("jpg"),
                    ImageFormat::Gif => Some("gif"),
                    ImageFormat::WebP => Some("webp"),
                    ImageFormat::Bmp => Some("bmp"),
                    _ => None,
                }),
        };
        if let Some(extension) = extension {
            name.push('.');
            name.push_str(extension);
        } else if name == "attachment" {
            name.push_str(".bin");
        }
    }
    name
}

pub fn remove_staged(data_directory: &Path, path: &Path) -> Result<bool, String> {
    let directory = data_directory.join(STAGING_DIRECTORY);
    let clipboard_file = path.parent() == Some(directory.as_path())
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(STAGED_FILE_PREFIX) && name.ends_with(".png"));
    let dropped_root = data_directory.join(DROPPED_FILE_DIRECTORY);
    let dropped_parent = path.parent();
    let dropped_file = dropped_parent
        .and_then(Path::parent)
        .is_some_and(|parent| parent == dropped_root)
        && dropped_parent
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.strip_prefix(DROPPED_FILE_DIRECTORY_PREFIX)
                    .is_some_and(|id| {
                        !id.is_empty()
                            && id.len() <= 160
                            && id.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
                            })
                    })
            });
    if !clipboard_file && !dropped_file {
        return Ok(false);
    }
    match fs::remove_file(path) {
        Ok(()) => {
            if dropped_file && let Some(parent) = path.parent() {
                let _ = fs::remove_dir(parent);
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Could not remove staged clipboard image: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, RawInput, vec2};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "hermes-clipboard-image-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn image_event_is_taken_only_when_composer_has_focus() {
        let context = Context::default();
        let id = Id::unique("clipboard-image-test-composer");
        let image = Arc::new(ColorImage::new([1, 1], vec![Color32::WHITE]));
        let mut observed = false;
        context
            .run_ui(
                RawInput {
                    events: vec![egui::Event::PasteImage(image.clone())],
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx();
                    ctx.memory_mut(|memory| memory.request_focus(id));
                    let received = take_focused_image_event(ctx, id);
                    observed = received.as_deref() == Some(image.as_ref());
                },
            )
            .drop_without_applying_deltas();
        assert!(observed);
    }

    #[test]
    fn image_event_is_left_alone_when_another_widget_owns_focus() {
        let context = Context::default();
        let composer_id = Id::unique("clipboard-image-test-composer");
        let other_id = Id::unique("clipboard-image-test-other");
        let image = Arc::new(ColorImage::new([1, 1], vec![Color32::WHITE]));
        let mut preserved = false;
        context
            .run_ui(
                RawInput {
                    events: vec![egui::Event::PasteImage(image)],
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx();
                    ctx.memory_mut(|memory| memory.request_focus(other_id));
                    assert!(take_focused_image_event(ctx, composer_id).is_none());
                    preserved = ctx.input(|input| {
                        input
                            .raw
                            .events
                            .iter()
                            .any(|event| matches!(event, egui::Event::PasteImage(_)))
                    });
                },
            )
            .drop_without_applying_deltas();
        assert!(preserved);
    }

    #[test]
    fn stages_a_valid_png_and_removes_only_its_own_file() {
        let root = temporary_directory("stage");
        let image = ColorImage::new([2, 1], vec![Color32::RED, Color32::BLUE]);
        let staged = stage_png(&root, "operation-1", &image).unwrap();
        let decoded =
            image::load_from_memory_with_format(&fs::read(&staged).unwrap(), ImageFormat::Png)
                .unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 1));
        assert!(!remove_staged(&root, Path::new("C:/outside/image.png")).unwrap());
        assert!(remove_staged(&root, &staged).unwrap());
        assert!(!staged.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stages_pathless_dropped_bytes_with_a_safe_name_and_removes_the_owned_file() {
        let root = temporary_directory("drop-bytes");
        let image = ColorImage::new([1, 1], vec![Color32::WHITE]);
        let mut png = std::io::Cursor::new(Vec::new());
        image::write_buffer_with_format(
            &mut png,
            &image.pixels[0].to_array(),
            1,
            1,
            image::ColorType::Rgba8,
            ImageFormat::Png,
        )
        .unwrap();
        let png = png.into_inner();

        let staged =
            stage_dropped_file_bytes(&root, "drop-1", "C:\\unsafe?.png", "", &png).unwrap();
        assert_eq!(staged.file_name().unwrap(), "unsafe_.png");
        assert_eq!(fs::read(&staged).unwrap(), png);
        assert!(remove_staged(&root, &staged).unwrap());
        assert!(!staged.exists());
        assert!(!staged.parent().unwrap().exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn infers_image_extension_for_extensionless_pathless_drops() {
        let root = temporary_directory("drop-image-extension");
        let image = ColorImage::new([1, 1], vec![Color32::WHITE]);
        let mut png = std::io::Cursor::new(Vec::new());
        image::write_buffer_with_format(
            &mut png,
            &image.pixels[0].to_array(),
            1,
            1,
            image::ColorType::Rgba8,
            ImageFormat::Png,
        )
        .unwrap();
        let png = png.into_inner();
        let staged = stage_dropped_file_bytes(&root, "drop-2", "", "", &png).unwrap();
        assert_eq!(staged.file_name().unwrap(), "attachment.png");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_pathless_drop_outside_size_and_id_limits() {
        let root = temporary_directory("drop-limits");
        assert!(stage_dropped_file_bytes(&root, "../escape", "x.txt", "", b"x").is_err());
        assert!(
            stage_dropped_file_bytes(
                &root,
                "drop-3",
                "x.txt",
                "",
                &vec![0; MAX_DROPPED_FILE_BYTES + 1],
            )
            .is_err()
        );
        assert!(!root.exists());
    }

    #[test]
    fn rejects_invalid_ids_and_oversized_image_dimensions_before_encoding() {
        let root = temporary_directory("limits");
        let image = ColorImage {
            size: [4097, 4097],
            pixels: Vec::new(),
            source_size: vec2(4097.0, 4097.0),
        };
        assert!(
            stage_png(
                &root,
                "../bad",
                &ColorImage::new([1, 1], vec![Color32::WHITE])
            )
            .is_err()
        );
        assert!(
            stage_png(&root, "operation-2", &image)
                .unwrap_err()
                .contains("16 megapixel")
        );
        assert!(!root.exists());
    }
}
