use doofusdock_core::*;
use std::{collections::BTreeMap, sync::Arc, time::SystemTime};

struct TemporaryDirectory(std::path::PathBuf);

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn descriptor(id: &str, role: &str, region: &str) -> PanelDescriptor {
    let mut panel = PanelDescriptor::new(id, format!("{role} panel"));
    panel.role = role.into();
    panel.provider_key = format!("hermes::{role}");
    panel.metadata.insert("region".into(), region.into());
    panel
}

fn find_session_tab(node: &Node) -> Option<(&Node, &[NodePtr], usize)> {
    match &node.kind {
        NodeKind::Tabs {
            items,
            active_index,
            ..
        } if items.iter().any(|item| {
            matches!(&item.kind, NodeKind::Panel { panel } if panel.0 == "chat")
        }) && items.iter().any(|item| {
            matches!(&item.kind, NodeKind::Panel { panel } if panel.0 == "session-profile-dev")
        }) => Some((node, items, *active_index)),
        NodeKind::Tabs { items, .. }
        | NodeKind::Split { children: items, .. }
        | NodeKind::Extension { children: items, .. }
        | NodeKind::DocumentWell { groups: items, .. } => {
            items.iter().find_map(|item| find_session_tab(item))
        }
        _ => None,
    }
}

fn save_and_recover(store: &mut LayoutStore, document: &LayoutDocument) -> LayoutDocument {
    store
        .save(document)
        .expect("persist canonical workbench state");
    store
        .recover(&document.id)
        .expect("recover canonical workbench state")
}

#[test]
fn hermes_shaped_workbench_keeps_profile_session_identity_across_dock_lifecycle() {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "doofusdock-hermes-workbench-{}-{unique}",
        std::process::id()
    ));
    let _temporary_directory = TemporaryDirectory(directory.clone());
    let mut store = LayoutStore::new(&directory, PersistenceLimits::default());

    let mut document = LayoutDocument::new("hermes-workbench", "hermes-main");
    let mut panels = vec![
        descriptor("navigation", "file-tree", "navigation"),
        descriptor("chat", "chat", "workspace"),
        descriptor("inspector", "review", "inspector"),
        descriptor("output", "terminal", "output"),
        descriptor("session-profile-dev", "session-transcript", "workspace"),
    ];
    let session = panels
        .iter_mut()
        .find(|panel| panel.id.0 == "session-profile-dev")
        .expect("session descriptor exists");
    session
        .metadata
        .insert("profile_id".into(), "developer".into());
    session
        .metadata
        .insert("session_id".into(), "session-7".into());
    session
        .metadata
        .insert("content_kind".into(), "transcript-and-composer".into());
    document.panels = Arc::new(panels);
    document.root = Some(Node::split(
        "hermes-four-zone-root",
        Orientation::Horizontal,
        vec![
            Node::panel("navigation-leaf", "navigation"),
            Node::panel("chat-leaf", "chat"),
            Node::panel("inspector-leaf", "inspector"),
            Node::panel("output-leaf", "output"),
            Node::panel("session-leaf", "session-profile-dev"),
        ],
        vec![0.16, 0.44, 0.18, 0.14, 0.08],
    ));
    validate_layout(&document).expect("initial four-zone workbench is valid");

    // A center drop models the application's drag-to-tab path from its
    // profile-scoped session pane onto the active chat workspace panel.
    let centered = apply_operation(
        &document,
        Operation::CenterPanel {
            panel: "session-profile-dev".into(),
            target_panel: "chat".into(),
        },
    )
    .expect("session pane can be centered over chat")
    .document;
    let (tabs, tab_items, active_index) =
        find_session_tab(centered.root.as_deref().expect("centered root exists"))
            .expect("chat and session share a tab group after center drop");
    assert_eq!(tabs.id.0, "center-tabs-1-chat");
    assert_eq!(
        tab_items
            .iter()
            .map(|item| match &item.kind {
                NodeKind::Panel { panel } => panel.0.as_str(),
                _ => panic!("consumer tab entries remain panel leaves"),
            })
            .collect::<Vec<_>>(),
        ["chat", "session-profile-dev"]
    );
    assert_eq!(active_index, 1, "drop selects the moved session pane");
    let centered = save_and_recover(&mut store, &centered);
    assert_eq!(
        find_session_tab(centered.root.as_deref().unwrap())
            .unwrap()
            .2,
        1,
        "selected session tab survives persistence reload"
    );

    let floated = apply_operation(
        &centered,
        Operation::FloatPanel {
            panel: "session-profile-dev".into(),
            window: "session-window-dev".into(),
            geometry: Rect {
                x: 120,
                y: 80,
                width: 720,
                height: 540,
            },
            monitor: "primary".into(),
            scale: 1.0,
        },
    )
    .expect("profile session pane can be detached")
    .document;
    assert_eq!(floated.floating_windows.len(), 1);
    assert!(floated.root.as_deref().unwrap().id.0 != "center-tabs-1-chat");
    let floated = save_and_recover(&mut store, &floated);
    assert!(
        find_panel_leaf(
            floated.floating_windows[0].root.as_ref(),
            &PanelId::from("session-profile-dev")
        )
        .is_some()
    );

    let docked = apply_operation(
        &floated,
        Operation::DockFloatingWindow {
            window: "session-window-dev".into(),
            target_panel: Some("chat".into()),
        },
    )
    .expect("session window can redock into chat")
    .document;
    assert!(docked.floating_windows.is_empty());
    let docked = save_and_recover(&mut store, &docked);
    let (_, tab_items, active_index) = find_session_tab(docked.root.as_deref().unwrap())
        .expect("redocked session and chat share a tab group after reload");
    assert_eq!(active_index, 1);
    assert_eq!(tab_items.len(), 2);
    assert_eq!(
        tab_items
            .iter()
            .map(|item| match &item.kind {
                NodeKind::Panel { panel } => panel.0.as_str(),
                _ => panic!("redocked consumer tabs remain panel leaves"),
            })
            .collect::<Vec<_>>(),
        ["chat", "session-profile-dev"]
    );

    let restored_descriptor = docked
        .descriptor(&PanelId::from("session-profile-dev"))
        .expect("session descriptor identity survives every transition");
    assert_eq!(restored_descriptor.role, "session-transcript");
    assert_eq!(
        restored_descriptor.provider_key,
        "hermes::session-transcript"
    );
    assert_eq!(
        restored_descriptor.metadata,
        BTreeMap::from([
            ("content_kind".into(), "transcript-and-composer".into()),
            ("profile_id".into(), "developer".into()),
            ("region".into(), "workspace".into()),
            ("session_id".into(), "session-7".into()),
        ])
    );
    validate_layout(&docked).expect("restored workbench remains valid");
}
