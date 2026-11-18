use doofusdock_core::*;
use doofusdock_presentation::*;
use std::sync::Arc;

#[test]
fn minimized_tab_group_projects_a_restore_rail_and_removes_panel_hit_targets() {
    let mut document = LayoutDocument::new("minimize-projection", "workspace");
    Arc::make_mut(&mut document.panels).extend([
        PanelDescriptor::new("editor", "Editor"),
        PanelDescriptor::new("files", "Files"),
    ]);
    document.root = Some(Node::split(
        "root",
        Orientation::Horizontal,
        vec![
            Node::tabs("group", vec![Node::panel("editor-node", "editor")], 0),
            Node::panel("files-node", "files"),
        ],
        vec![0.4, 0.6],
    ));
    document = apply_operation(
        &document,
        Operation::SetGroupMinimized {
            group: "group".into(),
            minimized: true,
        },
    )
    .unwrap()
    .document;

    let snapshot = project(
        &document,
        Box2 {
            x: 0.0,
            y: 0.0,
            w: 500.0,
            h: 300.0,
        },
    )
    .unwrap();
    let rail = snapshot.group_rails.get(&NodeId::from("group")).unwrap();
    assert_eq!(rail.panel_ids, vec![PanelId::from("editor")]);
    assert_eq!(rail.active_index, 0);
    assert_eq!(rail.bounds.w, GROUP_RAIL_EXTENT);
    assert_eq!(snapshot.panels[&PanelId::from("editor")].opacity, 0.0);
    assert_eq!(snapshot.hit_test(20.0, 100.0), None);
    assert_eq!(
        snapshot.hit_test(120.0, 100.0),
        Some(PanelId::from("files"))
    );
}
