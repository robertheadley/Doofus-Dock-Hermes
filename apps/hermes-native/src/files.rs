//! Local, read-only project tree for the Hermes Files panel.
//!
//! Filesystem work runs on a dedicated worker so a large or unavailable
//! directory cannot stall egui. The tree is lazy: only the root and expanded
//! directories are enumerated.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
};

const MAX_DIRECTORY_ENTRIES: usize = 1_000;
const MAX_PREVIEW_BYTES: u64 = 1_048_576;
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
}

struct PreviewResult {
    path: PathBuf,
    result: Result<FilePreviewContent, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePreviewContent {
    pub text: String,
    pub language: String,
}

#[derive(Default)]
pub struct FilePreviewStore {
    contents: BTreeMap<PathBuf, Result<FilePreviewContent, String>>,
    pending: BTreeSet<PathBuf>,
    request_tx: Option<Sender<PreviewRequest>>,
    result_rx: Option<Receiver<PreviewResult>>,
}

impl FilePreviewStore {
    pub fn request(&mut self, path: &Path) {
        self.ensure_worker();
        self.poll();
        if !self.contents.contains_key(path) && !self.pending.contains(path) {
            if self.pending.len() < 8 {
                self.pending.insert(path.to_path_buf());
                let sent = self.request_tx.as_ref().is_some_and(|tx| {
                    tx.send(PreviewRequest {
                        path: path.to_path_buf(),
                    })
                    .is_ok()
                });
                if !sent {
                    self.pending.remove(path);
                    self.contents.insert(
                        path.to_path_buf(),
                        Err("File preview worker stopped".into()),
                    );
                }
            }
        }
    }

    pub fn open(&mut self, ui: &mut egui::Ui, path: &Path) {
        self.request(path);
        match self.contents.get(path) {
            Some(Ok(content)) => {
                ui.label(
                    egui::RichText::new(path.display().to_string())
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                ui.separator();
                egui::ScrollArea::both()
                    .id_salt(("hermes-file-preview", path))
                    .show(ui, |ui| {
                        if content.language == "markdown" {
                            ui.label(egui::RichText::new(&content.text).size(13.0));
                        } else {
                            ui.label(
                                egui::RichText::new(&content.text)
                                    .font(egui::FontId::monospace(12.0)),
                            );
                        }
                    });
            }
            Some(Err(error)) => {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("Unable to preview this file: {error}"));
                });
            }
            None if self.pending.contains(path) => {
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
    }

    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.result_rx else {
            return false;
        };
        let mut changed = false;
        loop {
            match receiver.try_recv() {
                Ok(result) => {
                    self.pending.remove(&result.path);
                    self.contents.insert(result.path, result.result);
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
                while let Ok(request) = request_rx.recv() {
                    let result = read_file_preview(&request.path);
                    if result_tx
                        .send(PreviewResult {
                            path: request.path,
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

fn read_file_preview(path: &Path) -> Result<FilePreviewContent, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    if !metadata.is_file() {
        return Err("Only regular files can be previewed".into());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(format!(
            "File exceeds the {MAX_PREVIEW_BYTES}-byte preview limit"
        ));
    }
    let bytes = fs::read(path).map_err(|error| format!("{} ({})", error, error.kind()))?;
    if bytes.len() as u64 > MAX_PREVIEW_BYTES {
        return Err(format!(
            "File exceeds the {MAX_PREVIEW_BYTES}-byte preview limit"
        ));
    }
    let text =
        String::from_utf8(bytes).map_err(|_| "File is binary or not valid UTF-8".to_owned())?;
    let language = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
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
    Ok(FilePreviewContent { text, language })
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
        let preview = read_file_preview(&text).unwrap();
        assert_eq!(preview.language, "rust");
        assert!(preview.text.contains("fn main"));

        let binary = root.join("image.bin");
        fs::write(&binary, [0, 159, 146, 150]).unwrap();
        assert!(read_file_preview(&binary).unwrap_err().contains("binary"));

        let oversized = root.join("large.txt");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_PREVIEW_BYTES + 1).unwrap();
        assert!(
            read_file_preview(&oversized)
                .unwrap_err()
                .contains("preview limit")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
