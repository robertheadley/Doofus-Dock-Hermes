//! Transient interpolation for compatible resolved theme styles.
//!
//! The caller resolves component states with `ThemeStyleSheet` and owns when a
//! transition starts, is retargeted, or is applied to widgets. This adapter
//! reuses the presentation tween lifecycle and never mutates canonical layout.
use crate::{Easing, Tween, TweenValue};
use doofusdock_core::{
    ThemeMetric, ThemeStyleEasing, ThemeStyleProperty, ThemeStyleTransitionSpec, ThemeValue,
};
use std::collections::BTreeMap;
use std::fmt;

/// A bounded set of resolved style values for one host-owned component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeStyleFrame {
    values: BTreeMap<ThemeStyleProperty, ThemeValue>,
}

impl ThemeStyleFrame {
    /// Creates a frame from values returned by `ThemeStyleSheet::resolve`.
    /// Invalid or out-of-range values fail closed before interpolation.
    pub fn new(
        values: BTreeMap<ThemeStyleProperty, ThemeValue>,
    ) -> Result<Self, ThemeStyleTransitionError> {
        if values.len() > 10 {
            return Err(ThemeStyleTransitionError::InvalidFrame);
        }
        for (property, value) in &values {
            if !valid_style_value(*property, value) {
                return Err(ThemeStyleTransitionError::InvalidFrame);
            }
        }
        Ok(Self { values })
    }

    pub fn values(&self) -> &BTreeMap<ThemeStyleProperty, ThemeValue> {
        &self.values
    }

    pub fn get(&self, property: ThemeStyleProperty) -> Option<&ThemeValue> {
        self.values.get(&property)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A presentation-only transition between compatible resolved component styles.
/// Its property set is fixed at construction and remains stable across samples.
#[derive(Clone, Debug)]
pub struct ThemeStyleTransition {
    tween: Tween<ThemeStyleFrame>,
    properties: Vec<ThemeStyleProperty>,
}

impl ThemeStyleTransition {
    /// Creates a property-specific transition from a parsed CSS-inspired
    /// `ThemeStyleTransitionSpec`. Returns `Ok(None)` when the property is
    /// absent or discrete/incompatible; callers can then apply the destination
    /// value at their chosen discrete boundary.
    pub fn from_style_spec(
        from: &ThemeStyleFrame,
        to: &ThemeStyleFrame,
        start: f64,
        spec: ThemeStyleTransitionSpec,
        reduced_motion: bool,
    ) -> Result<Option<Self>, ThemeStyleTransitionError> {
        let property = spec.property();
        let (Some(from_value), Some(to_value)) = (from.get(property), to.get(property)) else {
            return Ok(None);
        };
        if !compatible_value_pair(property, from_value, to_value) {
            return Ok(None);
        }
        let from = ThemeStyleFrame::new(BTreeMap::from([(property, from_value.clone())]))?;
        let to = ThemeStyleFrame::new(BTreeMap::from([(property, to_value.clone())]))?;
        let easing = match spec.easing() {
            ThemeStyleEasing::Linear => Easing::Linear,
            ThemeStyleEasing::Ease => Easing::CubicBezier {
                x1: 0.25,
                y1: 0.1,
                x2: 0.25,
                y2: 1.0,
            },
            ThemeStyleEasing::EaseIn => Easing::EaseIn,
            ThemeStyleEasing::EaseOut => Easing::EaseOut,
            ThemeStyleEasing::EaseInOut => Easing::EaseInOut,
        };
        Self::new(
            from,
            to,
            start,
            spec.duration_seconds(),
            easing,
            reduced_motion,
        )
        .map(Some)
    }

    /// Builds tracks only for properties present at both endpoints with a
    /// compatible value type and, for metrics, the same unit. Discrete or
    /// incompatible values are omitted rather than assigned invented behavior.
    pub fn new(
        from: ThemeStyleFrame,
        to: ThemeStyleFrame,
        start: f64,
        duration: f64,
        easing: Easing,
        reduced_motion: bool,
    ) -> Result<Self, ThemeStyleTransitionError> {
        let (from, to, properties) = compatible_frames(from, to);
        let tween = Tween::new(from, to, start, duration, easing, reduced_motion)
            .map_err(ThemeStyleTransitionError::InvalidTween)?;
        Ok(Self { tween, properties })
    }

    pub fn sample(&self, time: f64) -> Result<ThemeStyleFrame, ThemeStyleTransitionError> {
        self.tween
            .sample(time)
            .map_err(ThemeStyleTransitionError::InvalidTween)
    }

    /// Retargets without a visual jump. Every currently animated property must
    /// still have the same supported value type and metric unit.
    pub fn retarget(
        &mut self,
        to: ThemeStyleFrame,
        time: f64,
        duration: f64,
        easing: Easing,
    ) -> Result<(), ThemeStyleTransitionError> {
        let current = self
            .tween
            .sample(time)
            .map_err(ThemeStyleTransitionError::InvalidTween)?;
        let mut target_values = BTreeMap::new();
        for property in &self.properties {
            let value = to
                .values
                .get(property)
                .ok_or(ThemeStyleTransitionError::IncompatibleRetarget)?;
            if !compatible_value_pair(*property, &current.values[property], value) {
                return Err(ThemeStyleTransitionError::IncompatibleRetarget);
            }
            target_values.insert(*property, value.clone());
        }
        self.tween
            .retarget(
                ThemeStyleFrame {
                    values: target_values,
                },
                time,
                duration,
                easing,
            )
            .map_err(ThemeStyleTransitionError::InvalidTween)
    }

    pub fn reverse(&mut self, time: f64, duration: f64) -> Result<(), ThemeStyleTransitionError> {
        self.tween
            .reverse(time, duration)
            .map_err(ThemeStyleTransitionError::InvalidTween)
    }

    pub fn is_complete(&self, time: f64) -> Result<bool, ThemeStyleTransitionError> {
        self.tween
            .is_complete(time)
            .map_err(ThemeStyleTransitionError::InvalidTween)
    }

    pub fn cancel(&mut self) {
        self.tween.cancel();
    }

    pub fn is_settled(&self) -> bool {
        self.tween.is_settled()
    }

    pub fn start_time(&self) -> f64 {
        self.tween.start_time()
    }

    pub fn duration(&self) -> f64 {
        self.tween.duration()
    }

    pub fn properties(&self) -> &[ThemeStyleProperty] {
        &self.properties
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeStyleTransitionError {
    InvalidFrame,
    IncompatibleRetarget,
    InvalidTween(String),
}

impl fmt::Display for ThemeStyleTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame => formatter.write_str("resolved theme style frame is invalid"),
            Self::IncompatibleRetarget => formatter.write_str(
                "theme style retarget must preserve the active animated property types and units",
            ),
            Self::InvalidTween(error) => write!(formatter, "theme style transition: {error}"),
        }
    }
}

impl std::error::Error for ThemeStyleTransitionError {}

impl TweenValue for ThemeStyleFrame {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        let amount = progress.clamp(0.0, 1.0);
        let values = from
            .values
            .iter()
            .filter_map(|(property, from_value)| {
                let to_value = to.values.get(property)?;
                interpolate_value(*property, from_value, to_value, amount)
                    .map(|value| (*property, value))
            })
            .collect();
        Self { values }
    }
}

fn compatible_frames(
    from: ThemeStyleFrame,
    to: ThemeStyleFrame,
) -> (ThemeStyleFrame, ThemeStyleFrame, Vec<ThemeStyleProperty>) {
    let mut from_values = BTreeMap::new();
    let mut to_values = BTreeMap::new();
    for (property, from_value) in from.values {
        if let Some(to_value) = to.values.get(&property)
            && compatible_value_pair(property, &from_value, to_value)
        {
            from_values.insert(property, from_value);
            to_values.insert(property, to_value.clone());
        }
    }
    let properties = from_values.keys().copied().collect();
    (
        ThemeStyleFrame {
            values: from_values,
        },
        ThemeStyleFrame { values: to_values },
        properties,
    )
}

fn compatible_value_pair(property: ThemeStyleProperty, from: &ThemeValue, to: &ThemeValue) -> bool {
    match (property, from, to) {
        (
            ThemeStyleProperty::Color
            | ThemeStyleProperty::BackgroundColor
            | ThemeStyleProperty::BorderColor,
            ThemeValue::Color(from),
            ThemeValue::Color(to),
        ) => parse_rgba(from).is_some() && parse_rgba(to).is_some(),
        (_, ThemeValue::Metric(from), ThemeValue::Metric(to)) => from.unit == to.unit,
        (ThemeStyleProperty::Opacity, ThemeValue::Number(_), ThemeValue::Number(_)) => true,
        // Font-family changes and any future discrete values intentionally step
        // outside this transition API instead of inventing interpolation rules.
        _ => false,
    }
}

fn interpolate_value(
    property: ThemeStyleProperty,
    from: &ThemeValue,
    to: &ThemeValue,
    amount: f64,
) -> Option<ThemeValue> {
    match (property, from, to) {
        (
            ThemeStyleProperty::Color
            | ThemeStyleProperty::BackgroundColor
            | ThemeStyleProperty::BorderColor,
            ThemeValue::Color(from),
            ThemeValue::Color(to),
        ) => {
            let from = parse_rgba(from)?;
            let to = parse_rgba(to)?;
            Some(ThemeValue::Color(format_rgba(std::array::from_fn(
                |index| from[index] + (to[index] - from[index]) * amount,
            ))))
        }
        (_, ThemeValue::Metric(from), ThemeValue::Metric(to)) if from.unit == to.unit => {
            let minimum = if property == ThemeStyleProperty::FontSize {
                1.0
            } else {
                0.0
            };
            Some(ThemeValue::Metric(ThemeMetric {
                amount: (from.amount + (to.amount - from.amount) * amount).clamp(minimum, 4096.0),
                unit: from.unit,
            }))
        }
        (ThemeStyleProperty::Opacity, ThemeValue::Number(from), ThemeValue::Number(to)) => Some(
            ThemeValue::Number((from + (to - from) * amount).clamp(0.0, 1.0)),
        ),
        _ => None,
    }
}

fn valid_style_value(property: ThemeStyleProperty, value: &ThemeValue) -> bool {
    match (property, value) {
        (
            ThemeStyleProperty::Color
            | ThemeStyleProperty::BackgroundColor
            | ThemeStyleProperty::BorderColor,
            ThemeValue::Color(color),
        ) => parse_rgba(color).is_some(),
        (ThemeStyleProperty::Opacity, ThemeValue::Number(value)) => {
            value.is_finite() && (0.0..=1.0).contains(value)
        }
        (ThemeStyleProperty::FontFamily, ThemeValue::Text(value)) => {
            !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
        }
        (
            ThemeStyleProperty::Padding
            | ThemeStyleProperty::Margin
            | ThemeStyleProperty::BorderWidth
            | ThemeStyleProperty::BorderRadius,
            ThemeValue::Metric(metric),
        ) => metric.amount.is_finite() && (0.0..=4096.0).contains(&metric.amount),
        (ThemeStyleProperty::FontSize, ThemeValue::Metric(metric)) => {
            metric.amount.is_finite() && (1.0..=4096.0).contains(&metric.amount)
        }
        _ => false,
    }
}

fn parse_rgba(color: &str) -> Option<[f64; 4]> {
    if !color.is_ascii() || !color.starts_with('#') || !matches!(color.len(), 7 | 9) {
        return None;
    }
    let parse = |index| u8::from_str_radix(&color[index..index + 2], 16).ok();
    Some([
        parse(1)? as f64 / 255.0,
        parse(3)? as f64 / 255.0,
        parse(5)? as f64 / 255.0,
        if color.len() == 9 {
            parse(7)? as f64 / 255.0
        } else {
            1.0
        },
    ])
}

fn format_rgba(rgba: [f64; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}
