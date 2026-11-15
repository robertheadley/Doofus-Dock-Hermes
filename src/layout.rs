use doofusdock_core::{
    DockingService, LayoutDocument, LayoutId, LayoutStore, Node, Orientation, PanelDescriptor,
    PersistenceLimits,
};
use doofusdock_presentation::{Box2, Snapshot, project};
use std::path::PathBuf;

pub const PANEL_EXPLORER: &str = "hermes.explorer";
pub const PANEL_CHAT: &str = "hermes.chat";
pub const PANEL_INSPECTOR: &str = "hermes.inspector";
pub const PANEL_TERMINAL: &str = "hermes.terminal";

pub struct Workspace {
    pub dock: DockingService,
    store: LayoutStore,
    pub persistence_error: Option<String>,
}

impl Workspace {
    pub fn load() -> Result<Self, String> {
        let mut store = LayoutStore::new(layout_directory(), PersistenceLimits::default());
        let layout_id = LayoutId::from("hermes-main");
        let document = match store.load(&layout_id) {
            Ok(document) => document,
            Err(_) => {
                let document = default_document();
                store.save(&document).map_err(|error| error.to_string())?;
                document
            }
        };
        let dock = DockingService::new(document, 128).map_err(|error| error.to_string())?;
        Ok(Self {
            dock,
            store,
            persistence_error: None,
        })
    }

    pub fn snapshot(&self, bounds: Box2) -> Result<Snapshot, String> {
        project(&self.dock.snapshot().document, bounds)
    }

    pub fn save(&mut self) {
        let document = self.dock.snapshot().document;
        match self.store.save(&document) {
            Ok(()) => self.persistence_error = None,
            Err(error) => self.persistence_error = Some(error.to_string()),
        }
    }
}

fn layout_directory() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("HermesDoofusDock").join("layouts")
}

fn default_document() -> LayoutDocument {
    let mut document = LayoutDocument::new("hermes-main", "hermes-default");
    let panels = [
        (PANEL_EXPLORER, "Files & Tasks"),
        (PANEL_CHAT, "Chat"),
        (PANEL_INSPECTOR, "Preview & Doofus"),
        (PANEL_TERMINAL, "Terminal · Tests · Logs"),
    ];
    let descriptors = panels
        .iter()
        .map(|(id, title)| {
            let mut descriptor = PanelDescriptor::new(*id, *title);
            descriptor.minimum = doofusdock_core::Size::new(220, 120);
            descriptor
        })
        .collect::<Vec<_>>();
    std::sync::Arc::make_mut(&mut document.panels).extend(descriptors);

    let explorer = Node::panel("node-explorer", PANEL_EXPLORER);
    let chat = Node::panel("node-chat", PANEL_CHAT);
    let inspector = Node::panel("node-inspector", PANEL_INSPECTOR);
    let terminal = Node::panel("node-terminal", PANEL_TERMINAL);
    let top = Node::split(
        "node-top-row",
        Orientation::Horizontal,
        vec![explorer, chat, inspector],
        vec![0.22, 0.53, 0.25],
    );
    document.root = Some(Node::split(
        "node-workbench",
        Orientation::Vertical,
        vec![top, terminal],
        vec![0.74, 0.26],
    ));
    document
}
