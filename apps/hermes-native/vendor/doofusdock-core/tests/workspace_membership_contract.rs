use doofusdock_core::{
    LayoutDocument, Node, Operation, PanelDescriptor, PanelId, apply_operation, validate_layout,
};
use std::collections::BTreeSet;

#[test]
fn panel_workspace_memberships_are_atomic_persistent_and_idempotent() {
    let mut document = LayoutDocument::new("workspace-tags", "workspace-main");
    document.panels = std::sync::Arc::new(vec![
        PanelDescriptor::new("editor", "Editor"),
        PanelDescriptor::new("shared", "Shared"),
    ]);
    document.root = Some(Node::tabs(
        "tabs",
        vec![
            Node::panel("editor-node", "editor"),
            Node::panel("shared-node", "shared"),
        ],
        0,
    ));
    validate_layout(&document).unwrap();

    let tags = BTreeSet::from(["work".to_string(), "research".to_string()]);
    let result = apply_operation(
        &document,
        Operation::SetPanelWorkspaceMemberships {
            panel: PanelId::from("editor"),
            tags: tags.clone(),
        },
    )
    .unwrap();
    assert_eq!(result.document.revision, document.revision + 1);
    assert_eq!(
        result
            .document
            .descriptor(&PanelId::from("editor"))
            .unwrap()
            .workspace_memberships,
        tags
    );
    assert_eq!(
        result.document.panels.len(),
        2,
        "membership must not clone canonical content ownership"
    );
    assert!(
        matches!(result.diff.patches.as_slice(), [doofusdock_core::DiffPatch::PanelWorkspaceMembershipChanged { panel, .. }] if panel.0 == "editor")
    );

    let repeated = apply_operation(
        &result.document,
        Operation::SetPanelWorkspaceMemberships {
            panel: PanelId::from("editor"),
            tags: BTreeSet::from(["research".to_string(), "work".to_string()]),
        },
    )
    .unwrap();
    assert!(repeated.diff.is_empty());
    assert_eq!(repeated.document.revision, result.document.revision);

    let encoded = serde_json::to_vec(&result.document).unwrap();
    let restored: LayoutDocument = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, result.document);
}

#[test]
fn invalid_or_unknown_memberships_fail_without_mutating_the_source() {
    let mut document = LayoutDocument::new("workspace-tags", "workspace-main");
    document.panels = std::sync::Arc::new(vec![PanelDescriptor::new("editor", "Editor")]);
    document.root = Some(Node::panel("editor-node", "editor"));
    let before = document.clone();
    assert!(
        apply_operation(
            &document,
            Operation::SetPanelWorkspaceMemberships {
                panel: PanelId::from("missing"),
                tags: BTreeSet::new()
            },
        )
        .is_err()
    );
    assert!(
        apply_operation(
            &document,
            Operation::SetPanelWorkspaceMemberships {
                panel: PanelId::from("editor"),
                tags: BTreeSet::from(["bad tag".to_string()])
            },
        )
        .is_err()
    );
    assert_eq!(document, before);
}
