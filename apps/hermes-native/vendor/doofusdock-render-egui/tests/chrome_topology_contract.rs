use doofusdock_render_egui::topology::{
    BoundarySide, TopologyPalette, TopologyRole, resolve_container, resolve_sibling_topology,
};
use egui::{Color32, Pos2, Rect};

fn palette(high_contrast: bool) -> TopologyPalette {
    TopologyPalette {
        border: Color32::from_rgb(22, 32, 42),
        accent: Color32::from_rgb(90, 120, 150),
        focus: Color32::WHITE,
        corner_radius: 6.0,
        border_width: 1.0,
        high_contrast,
    }
}

#[test]
fn theme_and_topology_resolve_stable_group_outline_contracts() {
    let tabs = resolve_container(TopologyRole::Tabs, 3, palette(false)).unwrap();
    assert_eq!(tabs.role, TopologyRole::Tabs);
    assert_eq!(tabs.outline.color, palette(false).accent);
    assert_eq!(tabs.corner_radius, 6.0);

    let split = resolve_container(TopologyRole::Split, 2, palette(false)).unwrap();
    assert_eq!(split.outline.color, palette(false).border);
    assert!((split.corner_radius - 3.9).abs() < 0.001);

    let contrast = resolve_container(TopologyRole::DocumentWell, 2, palette(true)).unwrap();
    assert_eq!(contrast.outline.color, Color32::WHITE);
    assert!(contrast.outline.width >= 2.0);
}

#[test]
fn leaves_single_child_groups_and_invalid_palette_values_have_no_outline() {
    assert!(resolve_container(TopologyRole::Split, 1, palette(false)).is_none());
    let mut invalid = palette(false);
    invalid.corner_radius = f32::NAN;
    assert!(resolve_container(TopologyRole::Tabs, 2, invalid).is_none());
}

#[test]
fn sibling_topology_is_order_independent_and_shares_each_border_once() {
    let left = (
        "left".to_owned(),
        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(50.0, 50.0)),
    );
    let right = (
        "right".to_owned(),
        Rect::from_min_max(Pos2::new(54.0, 0.0), Pos2::new(104.0, 50.0)),
    );
    let first = resolve_sibling_topology(&[left.clone(), right.clone()], 4.0).unwrap();
    let reversed = resolve_sibling_topology(&[right, left], 4.0).unwrap();
    assert_eq!(first, reversed);
    assert_eq!(first.shared_boundaries.len(), 1);
    assert_eq!(first.shared_boundaries[0].first_side, BoundarySide::Right);
    assert_eq!(first.shared_boundaries[0].second_side, BoundarySide::Left);
    assert_eq!(first.exterior_boundaries.len(), 6);
}
