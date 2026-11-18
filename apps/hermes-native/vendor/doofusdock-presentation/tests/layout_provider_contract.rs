use doofusdock_core::*;
use doofusdock_presentation::{Box2, project_with_extensions};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CustomLayout(Arc<AtomicUsize>);
impl ExtensionLayoutProvider for CustomLayout {
    fn modality(&self) -> &str {
        "consumer.custom_layout"
    }

    fn arrange(&self, request: &ExtensionLayoutRequest<'_>) -> DockResult<Vec<ExtensionPlacement>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(request
            .children
            .iter()
            .enumerate()
            .map(|(child_index, _)| ExtensionPlacement {
                child_index,
                bounds: Rect {
                    x: request.bounds.x + 12,
                    y: request.bounds.y + 8,
                    width: request.bounds.width - 24,
                    height: request.bounds.height - 16,
                },
            })
            .collect())
    }
}

#[test]
fn application_projection_uses_only_activated_extension_layout_provider() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = ExtensionRegistry::default();
    registry
        .register_layout_provider(LayoutProviderExtension {
            descriptor: ExtensionDescriptor {
                namespaced_id: "consumer:arranger".into(),
                version: ApiVersion { major: 1, minor: 0 },
                api: CompatibilityRange {
                    minimum: ApiVersion { major: 1, minor: 0 },
                    maximum: ApiVersion { major: 1, minor: 0 },
                },
                ..ExtensionDescriptor::default()
            },
            provider: Arc::new(CustomLayout(calls.clone())),
        })
        .unwrap();

    let mut document = LayoutDocument::new("custom-layout", "custom-layout-workspace");
    Arc::make_mut(&mut document.panels).push(PanelDescriptor::new("panel", "Panel"));
    document.root = Some(Arc::new(Node {
        id: "custom-root".into(),
        kind: NodeKind::Extension {
            modality: "consumer.custom_layout".into(),
            payload: "{}".into(),
            children: vec![Node::panel("panel-node", "panel")],
        },
    }));
    let bounds = Box2 {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };

    let inactive = project_with_extensions(&document, bounds, &registry).unwrap_err();
    assert!(inactive.contains("successful activation"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    registry.freeze();
    registry.activate("consumer:arranger").unwrap();
    let projected = project_with_extensions(&document, bounds, &registry).unwrap();
    let panel = projected.panels.get(&PanelId::from("panel")).unwrap();
    assert_eq!(
        panel.bounds,
        Box2 {
            x: 12.0,
            y: 8.0,
            w: 776.0,
            h: 584.0
        }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
