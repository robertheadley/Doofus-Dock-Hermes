//! Local, read-only project tree for the Hermes Files panel.
//!
//! Filesystem work runs on a dedicated worker so a large or unavailable
//! directory cannot stall egui. The tree is lazy: only the root and expanded
//! directories are enumerated.

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
};

const MAX_DIRECTORY_ENTRIES: usize = 1_000;
const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_PDF_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PDF_RENDER_WIDTH: i32 = 1_280;
const MAX_PDF_RENDER_HEIGHT: i32 = 1_800;
const IMAGE_EXTENSIONS: &[&str] = &["bmp", "gif", "jpeg", "jpg", "png", "svg", "webp"];
const EXCLUDED_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".cache",
    ".next",
    ".turbo",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
}

#[derive(Debug)]
struct DirectoryRequest {
    generation: u64,
    root: PathBuf,
    path: PathBuf,
}

#[derive(Debug)]
struct DirectoryResult {
    generation: u64,
    root: PathBuf,
    path: PathBuf,
    result: Result<Vec<FileEntry>, String>,
}

struct PreviewRequest {
    path: PathBuf,
    page_index: usize,
    force: bool,
}

struct PreviewResult {
    path: PathBuf,
    page_index: usize,
    result: Result<FilePreviewContent, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePreviewContent {
    pub kind: FilePreviewKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilePreviewKind {
    Text {
        text: String,
        language: String,
        truncated: bool,
    },
    Image {
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    },
    Pdf {
        page_count: usize,
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    },
}

#[derive(Default)]
pub struct FilePreviewStore {
    contents: BTreeMap<(PathBuf, usize), Result<FilePreviewContent, String>>,
    images: BTreeMap<(PathBuf, usize), egui::TextureHandle>,
    pending: BTreeSet<(PathBuf, usize)>,
    pdf_page: BTreeMap<PathBuf, usize>,
    request_tx: Option<Sender<PreviewRequest>>,
    result_rx: Option<Receiver<PreviewResult>>,
}

impl FilePreviewStore {
    pub fn request(&mut self, path: &Path) {
        let page_index = self.pdf_page.get(path).copied().unwrap_or(0);
        self.request_page(path, page_index, false);
    }

    fn request_page(&mut self, path: &Path, page_index: usize, force: bool) {
        self.ensure_worker();
        self.poll();
        let key = (path.to_path_buf(), page_index);
        if !self.contents.contains_key(&key) && !self.pending.contains(&key) {
            if self.pending.len() < 8 {
                self.pending.insert(key.clone());
                let sent = self.request_tx.as_ref().is_some_and(|tx| {
                    tx.send(PreviewRequest {
                        path: path.to_path_buf(),
                        page_index,
                        force,
                    })
                    .is_ok()
                });
                if !sent {
                    self.pending.remove(&key);
                    self.contents
                        .insert(key, Err("File preview worker stopped".into()));
                }
            }
        }
    }

    pub fn open(&mut self, ui: &mut egui::Ui, path: &Path, markdown_cache: &mut CommonMarkCache) {
        self.request(path);
        let mut force_preview = false;
        let page_index = self.pdf_page.get(path).copied().unwrap_or(0);
        let key = (path.to_path_buf(), page_index);
        let mut next_pdf_page = None;
        match self.contents.get(&key) {
            Some(Ok(content)) => match &content.kind {
                FilePreviewKind::Text {
                    text,
                    language,
                    truncated,
                } => {
                    egui::ScrollArea::both()
                        .id_salt(("hermes-file-preview", path))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.vertical(|ui| {
                                    if language == "markdown" {
                                        CommonMarkViewer::new().show(ui, markdown_cache, text);
                                    } else if let Some(markdown) =
                                        syntax_highlight_markdown(language, text)
                                    {
                                        CommonMarkViewer::new().show(ui, markdown_cache, &markdown);
                                    } else {
                                        ui.label(
                                            egui::RichText::new(text)
                                                .font(egui::FontId::monospace(12.0)),
                                        );
                                    }
                                    if *truncated {
                                        ui.label("Preview is truncated at 512 KiB.");
                                    }
                                });
                            });
                        });
                }
                FilePreviewKind::Image {
                    width,
                    height,
                    rgba,
                } => {
                    let texture = self.images.entry(key.clone()).or_insert_with(|| {
                        let image =
                            egui::ColorImage::from_rgba_unmultiplied([*width, *height], rgba);
                        ui.ctx().load_texture(
                            format!("hermes-file-preview:{}", path.display()),
                            image,
                            egui::TextureOptions::LINEAR,
                        )
                    });
                    egui::ScrollArea::both()
                        .id_salt(("hermes-file-preview-image", path))
                        .show(ui, |ui| {
                            ui.add_space(12.0);
                            ui.image(&*texture);
                        });
                }
                FilePreviewKind::Pdf {
                    page_count,
                    width,
                    height,
                    rgba,
                } => {
                    let texture = self.images.entry(key.clone()).or_insert_with(|| {
                        let image =
                            egui::ColorImage::from_rgba_unmultiplied([*width, *height], rgba);
                        ui.ctx().load_texture(
                            format!("hermes-file-preview-pdf:{}:{page_index}", path.display()),
                            image,
                            egui::TextureOptions::LINEAR,
                        )
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(page_index > 0, egui::Button::new("Previous page"))
                            .clicked()
                        {
                            next_pdf_page = Some(page_index.saturating_sub(1));
                        }
                        ui.label(format!("Page {} of {page_count}", page_index + 1));
                        if ui
                            .add_enabled(
                                page_index + 1 < *page_count,
                                egui::Button::new("Next page"),
                            )
                            .clicked()
                        {
                            next_pdf_page = Some(page_index + 1);
                        }
                    });
                    egui::ScrollArea::both()
                        .id_salt(("hermes-file-preview-pdf", path, page_index))
                        .show(ui, |ui| ui.image(&*texture));
                }
            },
            Some(Err(error)) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(format!("Unable to preview this file: {error}"));
                    if error.contains("preview limit") || error.contains("binary") {
                        if ui.button("Preview anyway").clicked() {
                            force_preview = true;
                        }
                    }
                });
            }
            None if self.pending.contains(&key) => {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading file preview…");
                });
            }
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("Preview queue is full. Close an unused preview and retry.");
                });
            }
        }
        if force_preview {
            self.request_force(path);
        }
        if let Some(page_index) = next_pdf_page {
            self.pdf_page.insert(path.to_path_buf(), page_index);
            self.request_page(path, page_index, false);
        }
    }

    fn request_force(&mut self, path: &Path) {
        let page_index = self.pdf_page.get(path).copied().unwrap_or(0);
        let key = (path.to_path_buf(), page_index);
        self.contents.remove(&key);
        self.pending.remove(&key);
        self.ensure_worker();
        if self.pending.len() >= 8 {
            self.contents
                .insert(key, Err("Preview queue is full".into()));
            return;
        }
        self.pending.insert(key.clone());
        let sent = self.request_tx.as_ref().is_some_and(|tx| {
            tx.send(PreviewRequest {
                path: path.to_path_buf(),
                page_index,
                force: true,
            })
            .is_ok()
        });
        if !sent {
            self.pending.remove(&key);
            self.contents
                .insert(key, Err("File preview worker stopped".into()));
        }
    }

    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.result_rx else {
            return false;
        };
        let mut changed = false;
        loop {
            match receiver.try_recv() {
                Ok(result) => {
                    self.pending
                        .remove(&(result.path.clone(), result.page_index));
                    self.contents
                        .insert((result.path, result.page_index), result.result);
                    changed = true;
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        changed
    }

    fn ensure_worker(&mut self) {
        if self.request_tx.is_some() {
            return;
        }
        let (request_tx, request_rx) = mpsc::channel::<PreviewRequest>();
        let (result_tx, result_rx) = mpsc::channel::<PreviewResult>();
        let spawn = thread::Builder::new()
            .name("hermes-file-preview".into())
            .spawn(move || {
                let mut pdfium: Option<Result<pdfium_render::prelude::Pdfium, String>> = None;
                while let Ok(request) = request_rx.recv() {
                    let is_pdf = request
                        .path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
                    let result = if is_pdf {
                        if pdfium.is_none() {
                            pdfium = Some(load_pdfium());
                        }
                        match pdfium.as_ref() {
                            Some(Ok(pdfium)) => read_file_preview_with_pdfium(
                                &request.path,
                                request.force,
                                request.page_index,
                                Some(pdfium),
                            ),
                            Some(Err(error)) => Err(error.clone()),
                            None => Err("PDF renderer initialization failed".into()),
                        }
                    } else {
                        read_file_preview_with_pdfium(
                            &request.path,
                            request.force,
                            request.page_index,
                            None,
                        )
                    };
                    if result_tx
                        .send(PreviewResult {
                            path: request.path,
                            page_index: request.page_index,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        if spawn.is_ok() {
            self.request_tx = Some(request_tx);
            self.result_rx = Some(result_rx);
        }
    }
}

fn syntax_highlight_markdown(language: &str, source: &str) -> Option<String> {
    let longest_run = |needle: char| {
        let mut longest = 0usize;
        let mut current = 0usize;
        for character in source.chars() {
            if character == needle {
                current += 1;
                longest = longest.max(current);
            } else {
                current = 0;
            }
        }
        longest
    };
    let (marker, longest) = if longest_run('`') <= longest_run('~') {
        ('`', longest_run('`'))
    } else {
        ('~', longest_run('~'))
    };
    let fence_length = longest.max(2) + 1;
    if fence_length > 64 {
        return None;
    }
    let fence = marker.to_string().repeat(fence_length);
    Some(format!("{fence}{language}\n{source}\n{fence}"))
}

pub struct FileBrowserState {
    root: Option<PathBuf>,
    entries: BTreeMap<PathBuf, Vec<FileEntry>>,
    expanded: BTreeSet<PathBuf>,
    pending: BTreeMap<PathBuf, u64>,
    generation: u64,
    errors: BTreeMap<PathBuf, String>,
    selected: Option<PathBuf>,
    request_tx: Sender<DirectoryRequest>,
    result_rx: Receiver<DirectoryResult>,
}

impl Default for FileBrowserState {
    fn default() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<DirectoryRequest>();
        let (result_tx, result_rx) = mpsc::channel::<DirectoryResult>();
        thread::Builder::new()
            .name("hermes-files-tree".into())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let result = read_directory(&request.path);
                    if result_tx
                        .send(DirectoryResult {
                            generation: request.generation,
                            root: request.root,
                            path: request.path,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("failed to start Hermes Files worker");
        Self {
            root: None,
            entries: BTreeMap::new(),
            expanded: BTreeSet::new(),
            pending: BTreeMap::new(),
            generation: 0,
            errors: BTreeMap::new(),
            selected: None,
            request_tx,
            result_rx,
        }
    }
}

impl FileBrowserState {
    pub fn set_root(&mut self, root: Option<PathBuf>) {
        let root = root.and_then(|path| {
            path.is_absolute()
                .then(|| path.components().collect::<PathBuf>())
        });
        if root == self.root {
            return;
        }
        self.generation = self.generation.saturating_add(1);
        self.root = root;
        self.entries.clear();
        self.expanded.clear();
        self.pending.clear();
        self.errors.clear();
        self.selected = None;
        if let Some(root) = self.root.clone() {
            self.expanded.insert(root.clone());
            self.request_directory(root);
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    pub fn refresh(&mut self) {
        if let Some(root) = self.root.clone() {
            self.generation = self.generation.saturating_add(1);
            self.entries.clear();
            self.pending.clear();
            self.errors.clear();
            self.expanded.clear();
            self.expanded.insert(root.clone());
            self.request_directory(root);
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.result_rx.try_recv() {
                Ok(result) => {
                    if self.generation != result.generation
                        || self.root.as_ref() != Some(&result.root)
                    {
                        continue;
                    }
                    self.pending.remove(&result.path);
                    match result.result {
                        Ok(entries) => {
                            self.errors.remove(&result.path);
                            self.entries.insert(result.path, entries);
                        }
                        Err(error) => {
                            self.errors.insert(result.path, error);
                        }
                    }
                    changed = true;
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        changed
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let Some(root) = self.root.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label("Open a Hermes session with a project folder to browse files.");
            });
            return None;
        };

        let opened = self.draw_directory(ui, &root, 0);
        if let Some(selected) = self.selected.clone() {
            ui.separator();
            ui.label(
                egui::RichText::new(selected.display().to_string())
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
        opened
    }

    fn request_directory(&mut self, path: PathBuf) {
        let Some(root) = self.root.clone() else {
            return;
        };
        if self.pending.len() >= 8
            || self.pending.contains_key(&path)
            || self.entries.contains_key(&path)
        {
            return;
        }
        self.errors.remove(&path);
        self.pending.insert(path.clone(), self.generation);
        if self
            .request_tx
            .send(DirectoryRequest {
                generation: self.generation,
                root,
                path: path.clone(),
            })
            .is_err()
        {
            self.pending.remove(&path);
            self.errors
                .insert(path, "File browser worker stopped".into());
        }
    }

    fn draw_directory(&mut self, ui: &mut egui::Ui, path: &Path, depth: usize) -> Option<PathBuf> {
        let Some(entries) = self.entries.get(path).cloned() else {
            if self.pending.contains_key(path) {
                ui.label("Loading…");
            } else if let Some(error) = self.errors.get(path).cloned() {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        format!("Unable to read: {error}"),
                    );
                    if ui.small_button("Retry").clicked() {
                        self.request_directory(path.to_path_buf());
                    }
                });
            }
            return None;
        };

        for entry in entries {
            let mut open_file = None;
            ui.horizontal(|ui| {
                ui.add_space((depth as f32) * 14.0);
                if entry.is_directory {
                    let open = self.expanded.contains(&entry.path);
                    let glyph = if open { "▾" } else { "▸" };
                    if ui.small_button(glyph).clicked() {
                        if open {
                            self.expanded.remove(&entry.path);
                        } else {
                            self.expanded.insert(entry.path.clone());
                            self.request_directory(entry.path.clone());
                        }
                    }
                    let response = ui.selectable_label(
                        self.selected.as_ref() == Some(&entry.path),
                        format!("{}/", entry.name),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::TreeItem,
                            true,
                            entry.path.display().to_string(),
                        )
                    });
                    if response.clicked() {
                        self.selected = Some(entry.path.clone());
                        if open {
                            self.expanded.remove(&entry.path);
                        } else {
                            self.expanded.insert(entry.path.clone());
                            self.request_directory(entry.path.clone());
                        }
                    }
                } else {
                    let response = ui.selectable_label(
                        self.selected.as_ref() == Some(&entry.path),
                        entry.name.clone(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::TreeItem,
                            true,
                            entry.path.display().to_string(),
                        )
                    });
                    if response.clicked() {
                        self.selected = Some(entry.path.clone());
                    }
                    if response.double_clicked() {
                        open_file = Some(entry.path.clone());
                    }
                }
            });
            if open_file.is_some() {
                return open_file;
            }
            if entry.is_directory && self.expanded.contains(&entry.path) {
                if let Some(opened) = self.draw_directory(ui, &entry.path, depth + 1) {
                    return Some(opened);
                }
            }
        }
        if self
            .entries
            .get(path)
            .is_some_and(|entries| entries.len() == MAX_DIRECTORY_ENTRIES)
        {
            ui.label(
                egui::RichText::new(format!(
                    "Showing at most {MAX_DIRECTORY_ENTRIES} entries in this folder."
                ))
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        }
        None
    }
}

#[cfg(test)]
fn read_file_preview(path: &Path, force: bool) -> Result<FilePreviewContent, String> {
    read_file_preview_with_pdfium(path, force, 0, None)
}

fn read_file_preview_with_pdfium(
    path: &Path,
    force: bool,
    page_index: usize,
    pdfium: Option<&pdfium_render::prelude::Pdfium>,
) -> Result<FilePreviewContent, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    if !metadata.is_file() {
        return Err("Only regular files can be previewed".into());
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        return read_image_preview(path, metadata.len());
    }
    if extension == "pdf" {
        let Some(pdfium) = pdfium else {
            return Err("PDF renderer is unavailable".into());
        };
        return read_pdf_preview(path, metadata.len(), page_index, pdfium);
    }
    if metadata.len() > MAX_PREVIEW_BYTES && !force {
        return Err(format!(
            "File exceeds the {MAX_PREVIEW_BYTES}-byte preview limit"
        ));
    }
    let file = fs::File::open(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    let mut bytes = Vec::with_capacity(MAX_PREVIEW_BYTES as usize + 1);
    file.take(MAX_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{} ({})", error, error.kind()))?;
    if bytes.len() as u64 > MAX_PREVIEW_BYTES && !force {
        return Err(format!(
            "File exceeds the {MAX_PREVIEW_BYTES}-byte preview limit"
        ));
    }
    let truncated = bytes.len() as u64 > MAX_PREVIEW_BYTES;
    bytes.truncate(MAX_PREVIEW_BYTES as usize);
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) if force => String::from_utf8_lossy(error.as_bytes()).into_owned(),
        Err(_) => return Err("File is binary or not valid UTF-8".into()),
    };
    let language = match extension.as_str() {
        "md" | "markdown" | "mdx" => "markdown",
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "json" => "json",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "html" | "htm" => "html",
        "css" => "css",
        _ => "text",
    }
    .to_owned();
    Ok(FilePreviewContent {
        kind: FilePreviewKind::Text {
            text,
            language,
            truncated,
        },
    })
}

fn pdfium_library_path() -> Result<PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let candidates = [
        exe_dir.map(|directory| directory.join("pdfium-8125.dll")),
        Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets/pdfium/chromium-8125/bin/pdfium.dll"),
        ),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
        .ok_or_else(|| "Bundled PDFium 8125 runtime is missing".into())
}

fn load_pdfium() -> Result<pdfium_render::prelude::Pdfium, String> {
    let path = pdfium_library_path()?;
    let bindings = pdfium_render::prelude::Pdfium::bind_to_library(&path)
        .map_err(|error| format!("Unable to load PDFium runtime: {error}"))?;
    Ok(pdfium_render::prelude::Pdfium::new(bindings))
}

fn read_pdf_preview(
    path: &Path,
    file_size: u64,
    page_index: usize,
    pdfium: &pdfium_render::prelude::Pdfium,
) -> Result<FilePreviewContent, String> {
    if file_size > MAX_PDF_BYTES {
        return Err(format!(
            "PDF exceeds the {MAX_PDF_BYTES}-byte preview limit"
        ));
    }
    let bytes = fs::read(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    if !bytes.starts_with(b"%PDF-") {
        return Err("Invalid PDF file header".into());
    }
    let document = pdfium
        .load_pdf_from_byte_vec(bytes, None)
        .map_err(|error| format!("Unable to open PDF: {error}"))?;
    let page_count = document.pages().len() as usize;
    if page_count == 0 {
        return Err("PDF contains no pages".into());
    }
    let page = document
        .pages()
        .get(page_index as _)
        .map_err(|error| format!("Unable to open PDF page {}: {error}", page_index + 1))?;
    let bitmap = page
        .render_with_config(
            &pdfium_render::prelude::PdfRenderConfig::new()
                .set_maximum_width(MAX_PDF_RENDER_WIDTH)
                .set_maximum_height(MAX_PDF_RENDER_HEIGHT),
        )
        .map_err(|error| format!("Unable to render PDF page {}: {error}", page_index + 1))?;
    let width = bitmap.width() as usize;
    let height = bitmap.height() as usize;
    if width == 0 || height == 0 || (width as u64) * (height as u64) > MAX_IMAGE_PIXELS {
        return Err("PDF page dimensions exceed the preview limit".into());
    }
    Ok(FilePreviewContent {
        kind: FilePreviewKind::Pdf {
            page_count,
            width,
            height,
            rgba: bitmap.as_rgba_bytes(),
        },
    })
}

fn read_image_preview(path: &Path, file_size: u64) -> Result<FilePreviewContent, String> {
    if file_size > MAX_IMAGE_BYTES {
        return Err(format!(
            "Image exceeds the {MAX_IMAGE_BYTES}-byte preview limit"
        ));
    }
    let bytes = fs::read(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
    {
        return decode_svg_preview(&bytes);
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| format!("Unable to identify image format: {error}"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some((MAX_IMAGE_PIXELS * 4).min(256 * 1024 * 1024));
    let mut reader = reader;
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| format!("Unable to decode image: {error}"))?;
    let (width, height) = (decoded.width() as u64, decoded.height() as u64);
    if width.saturating_mul(height) > MAX_IMAGE_PIXELS {
        return Err(format!(
            "Image dimensions exceed the {MAX_IMAGE_PIXELS}-pixel preview limit"
        ));
    }
    let image = decoded.to_rgba8();
    Ok(FilePreviewContent {
        kind: FilePreviewKind::Image {
            width: image.width() as usize,
            height: image.height() as usize,
            rgba: image.into_raw(),
        },
    })
}

fn decode_svg_preview(bytes: &[u8]) -> Result<FilePreviewContent, String> {
    let options = resvg::usvg::Options::default();
    // Nested mode deliberately ignores external image references. Previewing an
    // untrusted project SVG must never turn into arbitrary local-file reads.
    let tree = resvg::usvg::Tree::from_data_nested(bytes, &options)
        .map_err(|error| format!("Unable to parse SVG: {error}"))?;
    let size = tree.size();
    let width = size.width().ceil() as u64;
    let height = size.height().ceil() as u64;
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return Err("SVG dimensions exceed the supported 16384-pixel edge limit".into());
    }
    if width.saturating_mul(height) > MAX_IMAGE_PIXELS {
        return Err(format!(
            "SVG dimensions exceed the {MAX_IMAGE_PIXELS}-pixel preview limit"
        ));
    }
    let width = width as u32;
    let height = height as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("Unable to allocate {width}x{height} SVG preview"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Ok(FilePreviewContent {
        kind: FilePreviewKind::Image {
            width: width as usize,
            height: height as usize,
            rgba: pixmap.take(),
        },
    })
}

fn read_directory(path: &Path) -> Result<Vec<FileEntry>, String> {
    if !path.is_absolute() {
        return Err("Project path is not absolute".into());
    }
    let iterator = fs::read_dir(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    let mut entries = Vec::new();
    for item in iterator {
        let item = item.map_err(|error| format!("{} ({})", error, error.kind()))?;
        let name = item.file_name().to_string_lossy().into_owned();
        if EXCLUDED_NAMES.contains(&name.as_str()) {
            continue;
        }
        let file_type = item
            .file_type()
            .map_err(|error| format!("{} ({})", error, error.kind()))?;
        let is_directory = if file_type.is_dir() {
            true
        } else if file_type.is_symlink() {
            item.metadata().is_ok_and(|metadata| metadata.is_dir())
        } else {
            false
        };
        entries.push(FileEntry {
            path: item.path(),
            name,
            is_directory,
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    entries.truncate(MAX_DIRECTORY_ENTRIES);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn directory_listing_is_sorted_and_excludes_hidden_and_build_entries() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("hermes-files-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join("README.md"), "read only").unwrap();
        fs::write(root.join(".secret"), "hidden").unwrap();
        fs::write(root.join("target").join("artifact"), "ignored").unwrap();

        let entries = read_directory(&root).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["src", ".secret", "README.md"]
        );
        assert!(entries[0].is_directory);
        assert!(!entries[1].is_directory);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn browser_loads_root_asynchronously_and_rejects_stale_root_results() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let first =
            std::env::temp_dir().join(format!("hermes-files-a-{}-{nonce}", std::process::id()));
        let second =
            std::env::temp_dir().join(format!("hermes-files-b-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("old.txt"), "a").unwrap();
        fs::write(second.join("new.txt"), "b").unwrap();

        let mut state = FileBrowserState::default();
        state.set_root(Some(first.clone()));
        state.set_root(Some(second.clone()));
        for _ in 0..100 {
            state.poll();
            if state.entries.contains_key(&second) {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(state.entries.contains_key(&second));
        assert!(!state.entries.contains_key(&first));
        assert_eq!(state.entries[&second][0].name, "new.txt");
        fs::remove_dir_all(first).unwrap();
        fs::remove_dir_all(second).unwrap();
    }

    #[test]
    fn preview_reads_small_utf8_text_and_rejects_large_or_binary_files() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("hermes-preview-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let text = root.join("sample.rs");
        fs::write(&text, "fn main() {}\n").unwrap();
        let preview = read_file_preview(&text, false).unwrap();
        assert!(matches!(
            preview.kind,
            FilePreviewKind::Text { ref language, ref text, truncated: false }
                if language == "rust" && text.contains("fn main")
        ));

        let binary = root.join("image.bin");
        fs::write(&binary, [0, 159, 146, 150]).unwrap();
        assert!(
            read_file_preview(&binary, false)
                .unwrap_err()
                .contains("binary")
        );
        assert!(matches!(
            read_file_preview(&binary, true).unwrap().kind,
            FilePreviewKind::Text { ref text, .. } if text.contains('�')
        ));

        let oversized = root.join("large.txt");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_PREVIEW_BYTES + 1).unwrap();
        assert!(
            read_file_preview(&oversized, false)
                .unwrap_err()
                .contains("preview limit")
        );
        assert!(matches!(
            read_file_preview(&oversized, true).unwrap().kind,
            FilePreviewKind::Text {
                truncated: true,
                ..
            }
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn image_preview_decodes_supported_raster_content_and_rejects_oversized_input() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("hermes-image-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let image_path = root.join("preview.png");
        image::RgbaImage::from_pixel(2, 1, image::Rgba([12, 34, 56, 255]))
            .save(&image_path)
            .unwrap();
        let preview = read_file_preview(&image_path, false).unwrap();
        assert!(matches!(
            preview.kind,
            FilePreviewKind::Image { width: 2, height: 1, ref rgba }
                if rgba == &[12, 34, 56, 255, 12, 34, 56, 255]
        ));

        let oversized = root.join("large.png");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_IMAGE_BYTES + 1).unwrap();
        assert!(
            read_file_preview(&oversized, false)
                .unwrap_err()
                .contains("preview limit")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn svg_preview_renders_vectors_and_ignores_external_image_references() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hermes-svg-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let image_path = root.join("vector.svg");
        fs::write(
            &image_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10" viewBox="0 0 20 10"><rect width="20" height="10" fill="#ff0000"/><image href="file:///should-not-be-read.png" width="1" height="1"/></svg>"##,
        )
        .unwrap();
        let preview = read_file_preview(&image_path, false).unwrap();
        assert!(matches!(
            preview.kind,
            FilePreviewKind::Image { width: 20, height: 10, ref rgba }
                if rgba[0..4] == [255, 0, 0, 255]
        ));

        let malformed = root.join("broken.svg");
        fs::write(&malformed, "<svg>").unwrap();
        assert!(
            read_file_preview(&malformed, false)
                .unwrap_err()
                .contains("Unable to parse SVG")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pdf_preview_renders_selected_page_with_bounded_output() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hermes-pdf-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("sample.pdf");
        write_test_pdf(
            &path,
            &["1 0 0 rg 0 0 100 100 re f", "0 0 1 rg 0 0 100 100 re f"],
        );
        let pdfium = load_pdfium().expect("bundled PDFium runtime must load for its preview test");
        let first = read_file_preview_with_pdfium(&path, false, 0, Some(&pdfium)).unwrap();
        let second = read_file_preview_with_pdfium(&path, false, 1, Some(&pdfium)).unwrap();
        let FilePreviewKind::Pdf {
            page_count: 2,
            width,
            height,
            rgba: first_rgba,
        } = first.kind
        else {
            panic!("expected first PDF page preview");
        };
        let FilePreviewKind::Pdf {
            page_count: 2,
            rgba: second_rgba,
            ..
        } = second.kind
        else {
            panic!("expected second PDF page preview");
        };
        assert!(width <= MAX_PDF_RENDER_WIDTH as usize);
        assert!(height <= MAX_PDF_RENDER_HEIGHT as usize);
        assert_ne!(first_rgba, second_rgba);

        let malformed = root.join("bad.pdf");
        fs::write(&malformed, b"not a pdf").unwrap();
        assert!(
            read_file_preview_with_pdfium(&malformed, false, 0, Some(&pdfium))
                .unwrap_err()
                .contains("Invalid PDF file header")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn write_test_pdf(path: &Path, page_contents: &[&str]) {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Count {} /Kids [{}] >>",
                page_contents.len(),
                (0..page_contents.len())
                    .map(|index| format!("{} 0 R", 3 + index * 2))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        for (index, content) in page_contents.iter().enumerate() {
            let page_id = 3 + index * 2;
            let content_id = page_id + 1;
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents {content_id} 0 R >>"
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                content.len(),
                content
            ));
        }
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len() + 1);
        offsets.push(0usize);
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes
                .extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref_offset = bytes.len();
        bytes.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes(),
        );
        for offset in offsets.iter().skip(1) {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
                offsets.len()
            )
            .as_bytes(),
        );
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn syntax_highlight_wrapper_chooses_a_safe_fence_for_source_text() {
        let wrapped = syntax_highlight_markdown("rust", "let marker = `inline`;\n").unwrap();
        assert!(wrapped.starts_with("~~~rust\n"));
        assert!(wrapped.ends_with("\n~~~"));
        let fence_saturated = format!("{}{}", "~".repeat(65), "`".repeat(65));
        assert!(syntax_highlight_markdown("text", &fence_saturated).is_none());
    }
}
