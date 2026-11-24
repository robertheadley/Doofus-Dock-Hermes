use doofusdock_core::{
    LayoutDocument, ThemeStyleContext, ThemeStyleProperty, ThemeStyleSheet, ThemeStyleState,
    ThemeValue,
};
use doofusdock_presentation::{Easing, ThemeStyleFrame, ThemeStyleTransition};
use std::collections::BTreeMap;

fn resolve(source: &str, hover: bool) -> ThemeStyleFrame {
    let mut context = ThemeStyleContext::new("button")
        .unwrap()
        .with_class("toolbar")
        .unwrap();
    if hover {
        context = context.with_state(ThemeStyleState::Hover);
    }
    let sheet = ThemeStyleSheet::parse(source).unwrap();
    ThemeStyleFrame::new(sheet.resolve(&context, &BTreeMap::new()).unwrap()).unwrap()
}

fn normal_and_hover() -> (ThemeStyleFrame, ThemeStyleFrame) {
    let source = r#"
        .toolbar { color: #000000; padding: 4px; opacity: 0.2; font-family: "Segoe UI"; }
        .toolbar:hover { color: #ffffff; padding: 12px; opacity: 1; font-family: "Consolas"; }
    "#;
    (resolve(source, false), resolve(source, true))
}

#[test]
fn resolved_component_state_styles_interpolate_with_stable_compatible_keys() {
    let (normal, hover) = normal_and_hover();
    let transition =
        ThemeStyleTransition::new(normal, hover, 2.0, 1.0, Easing::Linear, false).unwrap();

    assert_eq!(
        transition.properties(),
        &[
            ThemeStyleProperty::Color,
            ThemeStyleProperty::Padding,
            ThemeStyleProperty::Opacity,
        ]
    );
    let before = transition.sample(2.0).unwrap();
    let middle = transition.sample(2.5).unwrap();
    let after = transition.sample(3.0).unwrap();
    assert_eq!(
        before.values().keys().collect::<Vec<_>>(),
        middle.values().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        middle.values().keys().collect::<Vec<_>>(),
        after.values().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        middle.get(ThemeStyleProperty::Color),
        Some(&ThemeValue::Color("#808080FF".to_owned()))
    );
    assert_eq!(
        middle.get(ThemeStyleProperty::Padding),
        Some(&ThemeValue::Metric(doofusdock_core::ThemeMetric {
            amount: 8.0,
            unit: doofusdock_core::ThemeMetricUnit::LogicalPx,
        }))
    );
    assert_eq!(middle.get(ThemeStyleProperty::FontFamily), None);
    assert_eq!(after.get(ThemeStyleProperty::FontFamily), None);
    assert!(transition.is_complete(3.0).unwrap());
}

#[test]
fn interruption_retargets_from_the_exact_sample_and_rejects_incompatible_targets() {
    let (normal, hover) = normal_and_hover();
    let mut transition =
        ThemeStyleTransition::new(normal, hover, 0.0, 1.0, Easing::EaseInOut, false).unwrap();
    let current = transition.sample(0.4).unwrap();

    let mut invalid_values = current.values().clone();
    invalid_values.remove(&ThemeStyleProperty::Padding);
    let invalid_target = ThemeStyleFrame::new(invalid_values).unwrap();
    assert!(
        transition
            .retarget(invalid_target, 0.4, 0.5, Easing::Linear)
            .is_err()
    );
    assert_eq!(transition.sample(0.4).unwrap(), current);

    let mut target_values = current.values().clone();
    target_values.insert(
        ThemeStyleProperty::Color,
        ThemeValue::Color("#0000FFFF".to_owned()),
    );
    let target = ThemeStyleFrame::new(target_values).unwrap();
    transition
        .retarget(target, 0.4, 0.5, Easing::Linear)
        .unwrap();
    assert_eq!(transition.sample(0.4).unwrap(), current);
    assert!(!transition.is_complete(0.6).unwrap());
    assert!(transition.is_complete(0.9).unwrap());
}

#[test]
fn metric_properties_with_different_units_are_omitted_from_the_stable_track_set() {
    let source = r#"
        .toolbar { padding: 4px; opacity: 0.2; }
        .toolbar:hover { padding: 12; opacity: 1; }
    "#;
    let transition = ThemeStyleTransition::new(
        resolve(source, false),
        resolve(source, true),
        0.0,
        1.0,
        Easing::Linear,
        false,
    )
    .unwrap();

    assert_eq!(transition.properties(), &[ThemeStyleProperty::Opacity]);
    for time in [0.0, 0.5, 1.0] {
        assert_eq!(
            transition
                .sample(time)
                .unwrap()
                .values()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![ThemeStyleProperty::Opacity]
        );
    }
}

#[test]
fn reduced_motion_settles_the_transition_without_changing_canonical_layout() {
    let document = LayoutDocument::new("style", "style");
    let original = document.clone();
    let (normal, hover) = normal_and_hover();
    let transition = ThemeStyleTransition::new(
        normal,
        hover,
        4.0,
        2.0,
        Easing::Spring {
            damping: 8.0,
            frequency: 2.5,
        },
        true,
    )
    .unwrap();

    assert_eq!(transition.duration(), 0.0);
    assert_eq!(
        transition
            .sample(3.99)
            .unwrap()
            .get(ThemeStyleProperty::Color),
        Some(&ThemeValue::Color("#000000FF".to_owned()))
    );
    assert_eq!(
        transition
            .sample(4.0)
            .unwrap()
            .get(ThemeStyleProperty::Color),
        Some(&ThemeValue::Color("#FFFFFFFF".to_owned()))
    );
    assert!(transition.is_complete(4.0).unwrap());
    assert_eq!(document, original);
}

#[test]
fn parsed_css_transition_specs_schedule_property_specific_existing_tweens() {
    let source = r#"
        .toolbar { color: #000000; opacity: 0.2; transition: color 200ms linear, opacity 0.4s ease-in; }
        .toolbar:hover { color: #FFFFFFFF; opacity: 1; }
    "#;
    let sheet = ThemeStyleSheet::parse(source).unwrap();
    let mut normal_context = ThemeStyleContext::new("button")
        .unwrap()
        .with_class("toolbar")
        .unwrap();
    let mut hover_context = normal_context.clone().with_state(ThemeStyleState::Hover);
    let tokens = BTreeMap::new();
    let normal = ThemeStyleFrame::new(sheet.resolve(&normal_context, &tokens).unwrap()).unwrap();
    let hover = ThemeStyleFrame::new(sheet.resolve(&hover_context, &tokens).unwrap()).unwrap();
    let specs = sheet.resolve_transitions(&normal_context).unwrap();

    let color = ThemeStyleTransition::from_style_spec(
        &normal,
        &hover,
        3.0,
        specs[&ThemeStyleProperty::Color],
        false,
    )
    .unwrap()
    .unwrap();
    let opacity = ThemeStyleTransition::from_style_spec(
        &normal,
        &hover,
        3.0,
        specs[&ThemeStyleProperty::Opacity],
        false,
    )
    .unwrap()
    .unwrap();

    assert_eq!(color.properties(), &[ThemeStyleProperty::Color]);
    assert_eq!(color.duration(), 0.2);
    assert_eq!(opacity.properties(), &[ThemeStyleProperty::Opacity]);
    assert_eq!(opacity.duration(), 0.4);
    assert_eq!(
        color.sample(3.1).unwrap().get(ThemeStyleProperty::Color),
        Some(&ThemeValue::Color("#808080FF".into()))
    );
    let ThemeValue::Number(opacity_at_midpoint) = opacity
        .sample(3.2)
        .unwrap()
        .get(ThemeStyleProperty::Opacity)
        .cloned()
        .unwrap()
    else {
        panic!("opacity track should produce a number");
    };
    assert!((opacity_at_midpoint - 0.4).abs() < 1e-12);

    let incompatible = r#"
        .toolbar { padding: 4px; transition: padding 200ms ease; }
        .toolbar:hover { padding: 12; }
    "#;
    let incompatible_sheet = ThemeStyleSheet::parse(incompatible).unwrap();
    normal_context = ThemeStyleContext::new("button")
        .unwrap()
        .with_class("toolbar")
        .unwrap();
    hover_context = normal_context.clone().with_state(ThemeStyleState::Hover);
    let from = ThemeStyleFrame::new(
        incompatible_sheet
            .resolve(&normal_context, &tokens)
            .unwrap(),
    )
    .unwrap();
    let to =
        ThemeStyleFrame::new(incompatible_sheet.resolve(&hover_context, &tokens).unwrap()).unwrap();
    let spec = incompatible_sheet
        .resolve_transitions(&normal_context)
        .unwrap()[&ThemeStyleProperty::Padding];
    assert!(
        ThemeStyleTransition::from_style_spec(&from, &to, 0.0, spec, false)
            .unwrap()
            .is_none()
    );
}
