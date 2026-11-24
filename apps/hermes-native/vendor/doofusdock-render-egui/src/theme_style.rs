//! Toolkit-specific mapping for core semantic theme styles.
//!
//! The caller chooses the role/state scope. This adapter changes only egui's
//! inherited style; canonical docking state and interaction policy stay owned
//! by the host and `doofusdock-core`.
use doofusdock_core::{ThemeStyleProperty, ThemeValue};
use egui::{Color32, FontFamily, FontId, Style, Vec2};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeStyleApplyError {
    TypeMismatch(ThemeStyleProperty),
    NonFinite(ThemeStyleProperty),
    OutOfRange(ThemeStyleProperty),
}

impl fmt::Display for ThemeStyleApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeMismatch(property) => {
                write!(f, "theme value type does not match {property:?}")
            }
            Self::NonFinite(property) => write!(f, "theme value for {property:?} must be finite"),
            Self::OutOfRange(property) => {
                write!(f, "theme value for {property:?} is outside egui bounds")
            }
        }
    }
}

impl std::error::Error for ThemeStyleApplyError {}

/// Apply an already resolved style map to an egui style.
///
/// `Padding` maps to button padding and `Margin` maps to egui inter-widget
/// spacing. `FontFamily` names must already be registered by the host. Values
/// are checked before the supplied style is changed.
pub fn apply_theme_style(
    style: &mut Style,
    values: &BTreeMap<ThemeStyleProperty, ThemeValue>,
) -> Result<(), ThemeStyleApplyError> {
    let mut candidate = style.clone();
    for (property, value) in values {
        match (property, value) {
            (ThemeStyleProperty::Color, ThemeValue::Color(value)) => {
                let color =
                    parse_color(value).ok_or(ThemeStyleApplyError::TypeMismatch(*property))?;
                candidate.visuals.override_text_color = Some(color);
                for_each_widget_visual(&mut candidate, |visual| visual.fg_stroke.color = color);
            }
            (ThemeStyleProperty::BackgroundColor, ThemeValue::Color(value)) => {
                let color =
                    parse_color(value).ok_or(ThemeStyleApplyError::TypeMismatch(*property))?;
                candidate.visuals.panel_fill = color;
                candidate.visuals.window_fill = color;
                for_each_widget_visual(&mut candidate, |visual| {
                    visual.bg_fill = color;
                    visual.weak_bg_fill = color;
                });
            }
            (ThemeStyleProperty::BorderColor, ThemeValue::Color(value)) => {
                let color =
                    parse_color(value).ok_or(ThemeStyleApplyError::TypeMismatch(*property))?;
                for_each_widget_visual(&mut candidate, |visual| visual.bg_stroke.color = color);
            }
            (ThemeStyleProperty::Padding, ThemeValue::Metric(metric)) => {
                let amount = metric.amount as f32;
                validate_metric(*property, metric.amount, amount)?;
                candidate.spacing.button_padding = Vec2::splat(amount);
            }
            (ThemeStyleProperty::Margin, ThemeValue::Metric(metric)) => {
                let amount = metric.amount as f32;
                validate_metric(*property, metric.amount, amount)?;
                candidate.spacing.item_spacing = Vec2::splat(amount);
            }
            (ThemeStyleProperty::BorderWidth, ThemeValue::Metric(metric)) => {
                let amount = metric.amount as f32;
                validate_metric(*property, metric.amount, amount)?;
                for_each_widget_visual(&mut candidate, |visual| visual.bg_stroke.width = amount);
            }
            (ThemeStyleProperty::BorderRadius, ThemeValue::Metric(metric)) => {
                let amount = metric.amount as f32;
                validate_metric(*property, metric.amount, amount)?;
                let radius = amount.round().clamp(0.0, 255.0) as u8;
                for_each_widget_visual(&mut candidate, |visual| {
                    visual.corner_radius = egui::CornerRadius::same(radius)
                });
            }
            (ThemeStyleProperty::FontSize, ThemeValue::Metric(metric)) => {
                let amount = metric.amount as f32;
                validate_metric(*property, metric.amount, amount)?;
                for font in candidate.text_styles.values_mut() {
                    font.size = amount;
                }
                if let Some(font) = &mut candidate.override_font_id {
                    font.size = amount;
                }
            }
            (ThemeStyleProperty::FontFamily, ThemeValue::Text(value)) => {
                if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                    return Err(ThemeStyleApplyError::TypeMismatch(*property));
                }
                let family = FontFamily::Name(value.clone().into());
                for font in candidate.text_styles.values_mut() {
                    font.family = family.clone();
                }
                let size = candidate
                    .override_font_id
                    .as_ref()
                    .map_or_else(|| FontId::default().size, |font| font.size);
                candidate.override_font_id = Some(FontId::new(size, family));
            }
            (ThemeStyleProperty::Opacity, ThemeValue::Number(value)) => {
                if !value.is_finite() {
                    return Err(ThemeStyleApplyError::NonFinite(*property));
                }
                if !(0.0..=1.0).contains(value) {
                    return Err(ThemeStyleApplyError::OutOfRange(*property));
                }
                let opacity = *value as f32;
                if !opacity.is_finite() {
                    return Err(ThemeStyleApplyError::NonFinite(*property));
                }
                if let Some(color) = candidate.visuals.override_text_color {
                    candidate.visuals.override_text_color = Some(color.gamma_multiply(opacity));
                }
                candidate.visuals.panel_fill = candidate.visuals.panel_fill.gamma_multiply(opacity);
                candidate.visuals.window_fill =
                    candidate.visuals.window_fill.gamma_multiply(opacity);
                for_each_widget_visual(&mut candidate, |visual| {
                    visual.bg_fill = visual.bg_fill.gamma_multiply(opacity);
                    visual.weak_bg_fill = visual.weak_bg_fill.gamma_multiply(opacity);
                    visual.bg_stroke.color = visual.bg_stroke.color.gamma_multiply(opacity);
                    visual.fg_stroke.color = visual.fg_stroke.color.gamma_multiply(opacity);
                });
            }
            _ => return Err(ThemeStyleApplyError::TypeMismatch(*property)),
        }
    }
    *style = candidate;
    Ok(())
}

fn validate_metric(
    property: ThemeStyleProperty,
    source: f64,
    value: f32,
) -> Result<(), ThemeStyleApplyError> {
    if !source.is_finite() || !value.is_finite() {
        return Err(ThemeStyleApplyError::NonFinite(property));
    }
    if !(0.0..=4096.0).contains(&value) {
        return Err(ThemeStyleApplyError::OutOfRange(property));
    }
    Ok(())
}

fn parse_color(value: &str) -> Option<Color32> {
    if !value.is_ascii() || !matches!(value.len(), 7 | 9) || !value.starts_with('#') {
        return None;
    }
    let channel = |start| u8::from_str_radix(&value[start..start + 2], 16).ok();
    Some(Color32::from_rgba_unmultiplied(
        channel(1)?,
        channel(3)?,
        channel(5)?,
        if value.len() == 9 { channel(7)? } else { 255 },
    ))
}

fn for_each_widget_visual(
    style: &mut Style,
    mut visit: impl FnMut(&mut egui::style::WidgetVisuals),
) {
    let widgets = &mut style.visuals.widgets;
    visit(&mut widgets.noninteractive);
    visit(&mut widgets.inactive);
    visit(&mut widgets.hovered);
    visit(&mut widgets.active);
    visit(&mut widgets.open);
}

#[cfg(test)]
mod tests {
    use super::*;
    use doofusdock_core::{
        ThemeMetric, ThemeMetricUnit, ThemeStyleContext, ThemeStyleSheet, ThemeStyleState,
    };

    #[test]
    fn applies_resolved_colors_spacing_geometry_and_opacity_to_egui_style() {
        let values = BTreeMap::from([
            (
                ThemeStyleProperty::Color,
                ThemeValue::Color("#f0e0d0".into()),
            ),
            (
                ThemeStyleProperty::BackgroundColor,
                ThemeValue::Color("#102030".into()),
            ),
            (
                ThemeStyleProperty::BorderColor,
                ThemeValue::Color("#405060".into()),
            ),
            (
                ThemeStyleProperty::Padding,
                ThemeValue::Metric(ThemeMetric {
                    amount: 7.0,
                    unit: ThemeMetricUnit::LogicalPx,
                }),
            ),
            (
                ThemeStyleProperty::BorderRadius,
                ThemeValue::Metric(ThemeMetric {
                    amount: 5.0,
                    unit: ThemeMetricUnit::LogicalPx,
                }),
            ),
            (ThemeStyleProperty::Opacity, ThemeValue::Number(0.5)),
        ]);
        let mut style = Style::default();
        apply_theme_style(&mut style, &values).unwrap();
        assert_eq!(style.spacing.button_padding, Vec2::splat(7.0));
        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            egui::CornerRadius::same(5)
        );
        assert_eq!(style.visuals.override_text_color.unwrap().a(), 128);
        assert_eq!(style.visuals.widgets.inactive.bg_fill.a(), 128);
        assert_eq!(style.visuals.widgets.inactive.bg_stroke.color.a(), 128);
    }

    #[test]
    fn invalid_style_map_does_not_partially_mutate_egui_style() {
        let mut style = Style::default();
        let original = style.clone();
        let values = BTreeMap::from([
            (
                ThemeStyleProperty::Padding,
                ThemeValue::Metric(ThemeMetric {
                    amount: 12.0,
                    unit: ThemeMetricUnit::LogicalPx,
                }),
            ),
            (ThemeStyleProperty::Opacity, ThemeValue::Number(2.0)),
        ]);
        assert_eq!(
            apply_theme_style(&mut style, &values),
            Err(ThemeStyleApplyError::OutOfRange(
                ThemeStyleProperty::Opacity
            ))
        );
        assert_eq!(style, original);
    }

    #[test]
    fn resolved_core_rules_apply_to_an_egui_style_scope() {
        let sheet = ThemeStyleSheet::parse(
            "button { color: var(--text); padding: 6px; } button:hover { background-color: #203040; }",
        )
        .unwrap();
        let context = ThemeStyleContext::new("button")
            .unwrap()
            .with_state(ThemeStyleState::Hover);
        let tokens = BTreeMap::from([("text".to_owned(), ThemeValue::Color("#f0e0d0".into()))]);
        let resolved = sheet.resolve(&context, &tokens).unwrap();
        let mut style = Style::default();

        apply_theme_style(&mut style, &resolved).unwrap();

        assert_eq!(style.spacing.button_padding, Vec2::splat(6.0));
        assert_eq!(
            style.visuals.widgets.inactive.bg_fill,
            Color32::from_rgb(32, 48, 64)
        );
        assert_eq!(
            style.visuals.widgets.inactive.fg_stroke.color,
            Color32::from_rgb(240, 224, 208)
        );
    }
}
