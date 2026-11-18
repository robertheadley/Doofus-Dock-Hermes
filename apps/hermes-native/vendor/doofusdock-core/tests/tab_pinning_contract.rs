use doofusdock_core::*;
use std::sync::Arc;
#[test]
fn tab_pins_are_idempotent_reversible_and_backward_compatible() {
    let mut d = LayoutDocument::new("layout", "workspace");
    Arc::make_mut(&mut d.panels).push(PanelDescriptor::new("one", "One"));
    d.root = Some(Node::panel("node", "one"));
    let old = serialize_layout(&d).unwrap();
    assert!(!old.contains("pinned_panels"));
    let restored = deserialize_layout(&old, PersistenceLimits::default()).unwrap();
    assert!(restored.pinned_panels.is_empty());
    let mut service = DockingService::new(d, 16).unwrap();
    let set = || Operation::SetTabPinned {
        panel: "one".into(),
        pinned: true,
    };
    service.transact(0, "pin".into(), [set()]).unwrap();
    let pinned = service.snapshot();
    assert_eq!(pinned.undo_depth, 1);
    service.transact(1, "pin-again".into(), [set()]).unwrap();
    assert_eq!(pinned, service.snapshot());
    let restored = deserialize_layout(
        &serialize_layout(&pinned.document).unwrap(),
        PersistenceLimits::default(),
    )
    .unwrap();
    assert!(restored.pinned_panels.contains(&PanelId::from("one")));
    let closed = service
        .transact(
            1,
            "close".into(),
            [Operation::ClosePanel {
                panel: "one".into(),
            }],
        )
        .unwrap();
    assert!(
        closed
            .document
            .pinned_panels
            .contains(&PanelId::from("one"))
    );
    service.undo(2).unwrap();
    service.undo(3).unwrap();
    assert!(service.snapshot().document.pinned_panels.is_empty());
    service.redo(4).unwrap();
    let before = service.snapshot();
    assert!(
        service
            .transact(
                5,
                "bad-pin".into(),
                [Operation::SetTabPinned {
                    panel: "unknown".into(),
                    pinned: true
                }]
            )
            .is_err()
    );
    assert_eq!(before, service.snapshot());
}
