use doofusdock_core::{LayoutDocument, Node, PanelDescriptor};
use doofusdock_presentation::{Box2, project_with_workspace_tags};
use std::collections::BTreeSet;

#[test]
fn tag_projection_filters_visibility_without_changing_canonical_panel_identity() {
    let mut document = LayoutDocument::new("workspace-projection", "workspace-main");
    let mut work = PanelDescriptor::new("work-panel", "Work");
    work.workspace_memberships.insert("work".into());
    let mut research = PanelDescriptor::new("research-panel", "Research");
    research.workspace_memberships.insert("research".into());
    document.panels = std::sync::Arc::new(vec![
        work,
        research,
        PanelDescriptor::new("shared", "Shared"),
    ]);
    document.root = Some(Node::tabs(
        "tabs",
        vec![
            Node::panel("work-node", "work-panel"),
            Node::panel("research-node", "research-panel"),
            Node::panel("shared-node", "shared"),
        ],
        0,
    ));
    let tags = BTreeSet::from(["work".to_string()]);

    let projected = project_with_workspace_tags(
        &document,
        Box2 {
            x: 0.0,
            y: 0.0,
            w: 1200.0,
            h: 800.0,
        },
        &tags,
    )
    .unwrap();
    assert_eq!(
        projected
            .panels
            .keys()
            .map(|id| id.0.as_str())
            .collect::<Vec<_>>(),
        vec!["shared", "work-panel"]
    );
    assert_eq!(document.panels.len(), 3);
    assert_eq!(document.root.as_ref().unwrap().id.0, "tabs");
}
