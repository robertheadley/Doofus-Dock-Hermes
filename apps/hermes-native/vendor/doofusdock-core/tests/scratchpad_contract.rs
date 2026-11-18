use doofusdock_core::{
    DockingService, FloatingWindow, LayoutDocument, Node, Operation, PanelDescriptor, Point, Rect,
    ScratchpadConfig, WindowId,
};

fn document() -> LayoutDocument {
    let mut document = LayoutDocument::new("scratchpads", "workspace");
    document.panels = vec![PanelDescriptor::new("terminal", "Terminal")].into();
    document.floating_windows.push(FloatingWindow {
        id: WindowId::from("terminal-window"),
        geometry: Rect {
            x: 100,
            y: 140,
            width: 720,
            height: 480,
        },
        monitor_id: "display-1".into(),
        scale: 1.0,
        z_order_hint: 0,
        picture_in_picture: false,
        scratchpad: None,
        root: Node::panel("terminal-node", "terminal"),
    });
    document
}

#[test]
fn named_scratchpad_visibility_and_summon_geometry_are_canonical_and_reversible() {
    let mut service = DockingService::new(document(), 16).unwrap();
    let subscription = service.subscribe_events().unwrap();
    let config = ScratchpadConfig {
        name: "terminal".into(),
        visible: false,
        summon_position: Some(Point { x: 40, y: 60 }),
        auto_hide_on_deactivate: false,
        pinned_across_workspaces: false,
    };
    let configured = service
        .transact(
            0,
            "configure-terminal-scratchpad".into(),
            [Operation::SetScratchpad {
                window: "terminal-window".into(),
                config: Some(config.clone()),
            }],
        )
        .unwrap();
    assert_eq!(configured.document.revision, 1);
    assert!(configured.diff.patches.iter().any(|patch| matches!(
        patch,
        doofusdock_core::DiffPatch::ScratchpadChanged {
            config: Some(value), ..
        } if value == &config
    )));
    let events = service.read_events(subscription, 16).unwrap();
    assert!(events.events.iter().any(|event| {
        event.kind == doofusdock_core::FrameworkEventKind::ScratchpadChanged
            && event
                .payload
                .get("visible")
                .is_some_and(|visible| visible == "false")
    }));

    let mut visible = config.clone();
    visible.visible = true;
    let summoned = service
        .transact(
            1,
            "summon-terminal-scratchpad".into(),
            [Operation::SetScratchpad {
                window: "terminal-window".into(),
                config: Some(visible.clone()),
            }],
        )
        .unwrap();
    assert_eq!(summoned.document.floating_windows[0].geometry.x, 40);
    assert_eq!(summoned.document.floating_windows[0].geometry.y, 60);
    assert_eq!(
        summoned.document.floating_windows[0].scratchpad,
        Some(visible.clone())
    );
    let repeated_summon = service
        .transact(
            2,
            "summon-terminal-scratchpad-again".into(),
            [Operation::SetScratchpad {
                window: "terminal-window".into(),
                config: Some(visible.clone()),
            }],
        )
        .unwrap();
    assert!(repeated_summon.diff.is_empty());
    assert_eq!(repeated_summon.document.revision, 2);

    let mut hidden = visible;
    hidden.visible = false;
    let dismissed = service
        .transact(
            2,
            "dismiss-terminal-scratchpad".into(),
            [Operation::SetScratchpad {
                window: "terminal-window".into(),
                config: Some(hidden.clone()),
            }],
        )
        .unwrap();
    assert_eq!(dismissed.document.floating_windows[0].geometry.x, 40);
    assert_eq!(
        dismissed.document.floating_windows[0].scratchpad,
        Some(hidden.clone())
    );
    let repeated = service
        .transact(
            3,
            "dismiss-terminal-scratchpad-again".into(),
            [Operation::SetScratchpad {
                window: "terminal-window".into(),
                config: Some(hidden),
            }],
        )
        .unwrap();
    assert!(repeated.diff.is_empty());
    assert_eq!(repeated.document.revision, 3);
}

#[test]
fn scratchpad_names_are_unique_and_legacy_floating_windows_default_to_none() {
    let mut doc = document();
    let first = ScratchpadConfig {
        name: "one".into(),
        visible: true,
        summon_position: None,
        auto_hide_on_deactivate: false,
        pinned_across_workspaces: false,
    };
    doc.floating_windows[0].scratchpad = Some(first.clone());
    let mut duplicate = doc.floating_windows[0].clone();
    duplicate.id = "second-window".into();
    duplicate.root = Node::panel("second-terminal", "terminal");
    doc.floating_windows.push(duplicate);
    assert!(doofusdock_core::validate_layout(&doc).is_err());

    let mut legacy = document();
    let mut encoded = serde_json::to_value(&legacy).unwrap();
    encoded["windows"][0]
        .as_object_mut()
        .unwrap()
        .remove("scratchpad");
    legacy = serde_json::from_value(encoded).unwrap();
    assert_eq!(legacy.floating_windows[0].scratchpad, None);
}

#[test]
fn legacy_scratchpad_config_defaults_workspace_pin_to_false() {
    let value = serde_json::json!({
        "name": "terminal",
        "visible": true,
        "summon_position": null,
        "auto_hide_on_deactivate": false
    });
    let decoded: ScratchpadConfig = serde_json::from_value(value).unwrap();
    assert!(!decoded.pinned_across_workspaces);
}
