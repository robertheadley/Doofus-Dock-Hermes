//! Bounded CSS-inspired theme rules over typed theme tokens.
//!
//! This parser intentionally supports a deterministic subset rather than web CSS.
//! It cannot change docking geometry, input, focus, or application behavior.
use crate::{ThemeError, ThemeErrorCode, ThemeMetric, ThemeMetricUnit, ThemeValue};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_STYLESHEET_BYTES: usize = 64 * 1024;
const MAX_RULES: usize = 256;
const MAX_SELECTORS_PER_RULE: usize = 16;
const MAX_DECLARATIONS_PER_RULE: usize = 32;
const MAX_DECLARATIONS_TOTAL: usize = 4096;
const MAX_SELECTOR_LENGTH: usize = 128;
const MAX_STYLE_VALUE_LENGTH: usize = 256;
const MAX_TRANSITION_TRACKS: usize = 10;
const MAX_TRANSITION_SECONDS: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThemeStyleProperty {
    Color,
    BackgroundColor,
    BorderColor,
    Padding,
    Margin,
    BorderWidth,
    BorderRadius,
    FontSize,
    FontFamily,
    Opacity,
}

/// CSS-inspired easing keywords accepted by the bounded `transition` subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeStyleEasing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// One property-specific, host-scheduled style transition declaration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeStyleTransitionSpec {
    property: ThemeStyleProperty,
    duration_seconds: f64,
    easing: ThemeStyleEasing,
}

impl ThemeStyleTransitionSpec {
    pub fn property(self) -> ThemeStyleProperty {
        self.property
    }

    pub fn duration_seconds(self) -> f64 {
        self.duration_seconds
    }

    pub fn easing(self) -> ThemeStyleEasing {
        self.easing
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThemeStyleState {
    Hover,
    Focus,
    Active,
    Pressed,
    Disabled,
    Selected,
    Loading,
    Attention,
    Unsaved,
    Error,
    Dragging,
    DropTarget,
    Pinned,
    AutoHidden,
    Floating,
    Maximized,
}

impl ThemeStyleState {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "hover" => Self::Hover,
            "focus" => Self::Focus,
            "active" => Self::Active,
            "pressed" => Self::Pressed,
            "disabled" => Self::Disabled,
            "selected" => Self::Selected,
            "loading" => Self::Loading,
            "attention" => Self::Attention,
            "unsaved" => Self::Unsaved,
            "error" => Self::Error,
            "dragging" => Self::Dragging,
            "drop-target" => Self::DropTarget,
            "pinned" => Self::Pinned,
            "auto-hidden" => Self::AutoHidden,
            "floating" => Self::Floating,
            "maximized" => Self::Maximized,
            _ => return None,
        })
    }
}

/// Semantic identity and host-reported state used to resolve theme rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeStyleContext {
    pub id: Option<String>,
    pub role: Option<String>,
    pub classes: BTreeSet<String>,
    pub states: BTreeSet<ThemeStyleState>,
    /// Optional logical viewport width used by bounded responsive rules.
    pub viewport_width: Option<f64>,
}

impl ThemeStyleContext {
    pub fn new(role: impl Into<String>) -> Result<Self, ThemeError> {
        let role = role.into();
        validate_selector_name(&role, "component role")?;
        Ok(Self {
            role: Some(role),
            ..Self::default()
        })
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Result<Self, ThemeError> {
        let id = id.into();
        validate_selector_name(&id, "component id")?;
        self.id = Some(id);
        Ok(self)
    }

    pub fn with_class(mut self, class: impl Into<String>) -> Result<Self, ThemeError> {
        let class = class.into();
        validate_selector_name(&class, "component class")?;
        self.classes.insert(class);
        Ok(self)
    }

    pub fn with_state(mut self, state: ThemeStyleState) -> Self {
        self.states.insert(state);
        self
    }

    pub fn with_viewport_width(mut self, width: f64) -> Result<Self, ThemeError> {
        if !width.is_finite() || !(1.0..=1_000_000.0).contains(&width) {
            return Err(style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme viewport width is outside its supported bounds.",
                width.to_string(),
            ));
        }
        self.viewport_width = Some(width);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum StyleValue {
    Literal(ThemeValue),
    Token(String),
}

#[derive(Clone, Debug, PartialEq)]
struct StyleDeclaration {
    property: ThemeStyleProperty,
    value: StyleValue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct StyleSelector {
    role: Option<String>,
    id: Option<String>,
    class: Option<String>,
    state: Option<ThemeStyleState>,
}

impl StyleSelector {
    fn specificity(&self) -> u8 {
        u8::from(self.id.is_some()) * 100
            + u8::from(self.class.is_some()) * 10
            + u8::from(self.state.is_some()) * 10
            + u8::from(self.role.is_some())
    }

    fn matches(&self, context: &ThemeStyleContext) -> bool {
        self.id
            .as_ref()
            .is_none_or(|id| context.id.as_ref() == Some(id))
            && self
                .role
                .as_ref()
                .is_none_or(|role| context.role.as_ref() == Some(role))
            && self
                .class
                .as_ref()
                .is_none_or(|class| context.classes.contains(class))
            && self
                .state
                .is_none_or(|state| context.states.contains(&state))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct StyleRule {
    selectors: Vec<StyleSelector>,
    declarations: Vec<StyleDeclaration>,
    transitions: Option<Vec<ThemeStyleTransitionSpec>>,
    declaration_count: usize,
    source_order: usize,
    min_width: Option<f64>,
    max_width: Option<f64>,
}

/// A validated, deterministic subset of CSS-like semantic style rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeStyleSheet {
    rules: Vec<StyleRule>,
}

impl ThemeStyleSheet {
    /// Parse simple semantic selectors with typed declarations and optional
    /// single-level viewport-width media rules. Other at-rules, combinators,
    /// nested rules, and unknown properties are rejected rather than ignored.
    pub fn parse(source: &str) -> Result<Self, ThemeError> {
        Self::parse_inner(source, true)
    }

    fn parse_inner(source: &str, allow_media: bool) -> Result<Self, ThemeError> {
        if source.len() > MAX_STYLESHEET_BYTES {
            return Err(style_error(
                ThemeErrorCode::ResourceLimit,
                "Theme stylesheet exceeds its byte limit.",
                source.len().to_string(),
            ));
        }
        let source = strip_comments(source)?;
        let mut rules = Vec::new();
        let mut total_declarations = 0usize;
        let mut cursor = 0usize;
        while cursor < source.len() {
            cursor = skip_space(&source, cursor);
            if cursor == source.len() {
                break;
            }
            let open = source[cursor..]
                .find('{')
                .map(|offset| cursor + offset)
                .ok_or_else(|| {
                    style_error(
                        ThemeErrorCode::InvalidParameter,
                        "Theme rule is missing an opening brace.",
                        format!("offset={cursor}"),
                    )
                })?;
            if source[cursor..open].contains('}') {
                return Err(style_error(
                    ThemeErrorCode::InvalidParameter,
                    "Unexpected closing brace in theme stylesheet.",
                    format!("offset={cursor}"),
                ));
            }
            let selector_text = source[cursor..open].trim();
            if selector_text.is_empty() {
                return Err(style_error(
                    ThemeErrorCode::InvalidParameter,
                    "Theme stylesheet selector is empty.",
                    selector_text,
                ));
            }
            if selector_text.starts_with('@') {
                let (min_width, max_width) = parse_media_condition(selector_text)?;
                if !allow_media {
                    return Err(style_error(
                        ThemeErrorCode::InvalidParameter,
                        "Nested viewport media rules are unsupported.",
                        selector_text,
                    ));
                }
                let close = matching_brace(&source, open)?;
                let nested = Self::parse_inner(&source[open + 1..close], false)?;
                total_declarations = total_declarations
                    .checked_add(
                        nested
                            .rules
                            .iter()
                            .map(|rule| rule.declaration_count)
                            .sum::<usize>(),
                    )
                    .filter(|count| *count <= MAX_DECLARATIONS_TOTAL)
                    .ok_or_else(|| {
                        style_error(
                            ThemeErrorCode::ResourceLimit,
                            "Theme stylesheet declaration count exceeds its limit.",
                            MAX_DECLARATIONS_TOTAL.to_string(),
                        )
                    })?;
                if rules.len() + nested.rules.len() > MAX_RULES {
                    return Err(style_error(
                        ThemeErrorCode::ResourceLimit,
                        "Theme stylesheet rule count exceeds its limit.",
                        MAX_RULES.to_string(),
                    ));
                }
                for mut rule in nested.rules {
                    rule.source_order = rules.len();
                    rule.min_width = min_width;
                    rule.max_width = max_width;
                    rules.push(rule);
                }
                cursor = close + 1;
                continue;
            }
            let close_relative = source[open + 1..].find('}').ok_or_else(|| {
                style_error(
                    ThemeErrorCode::InvalidParameter,
                    "Theme rule is missing a closing brace.",
                    format!("offset={open}"),
                )
            })?;
            let close = open + 1 + close_relative;
            let body = &source[open + 1..close];
            if body.contains('{') {
                return Err(style_error(
                    ThemeErrorCode::InvalidParameter,
                    "Nested theme rules are unsupported.",
                    format!("offset={open}"),
                ));
            }
            let selectors = selector_text
                .split(',')
                .map(str::trim)
                .map(parse_selector)
                .collect::<Result<Vec<_>, _>>()?;
            if selectors.is_empty() || selectors.len() > MAX_SELECTORS_PER_RULE {
                return Err(style_error(
                    ThemeErrorCode::ResourceLimit,
                    "Theme rule selector count exceeds its limit.",
                    selectors.len().to_string(),
                ));
            }
            let parsed = parse_declarations(body)?;
            if parsed.declarations.is_empty() && parsed.transitions.is_none()
                || parsed.declaration_count > MAX_DECLARATIONS_PER_RULE
            {
                return Err(style_error(
                    ThemeErrorCode::ResourceLimit,
                    "Theme rule declaration count is empty or exceeds its limit.",
                    parsed.declaration_count.to_string(),
                ));
            }
            total_declarations = total_declarations
                .checked_add(parsed.declaration_count)
                .filter(|count| *count <= MAX_DECLARATIONS_TOTAL)
                .ok_or_else(|| {
                    style_error(
                        ThemeErrorCode::ResourceLimit,
                        "Theme stylesheet declaration count exceeds its limit.",
                        MAX_DECLARATIONS_TOTAL.to_string(),
                    )
                })?;
            if rules.len() >= MAX_RULES {
                return Err(style_error(
                    ThemeErrorCode::ResourceLimit,
                    "Theme stylesheet rule count exceeds its limit.",
                    MAX_RULES.to_string(),
                ));
            }
            rules.push(StyleRule {
                selectors,
                declarations: parsed.declarations,
                transitions: parsed.transitions,
                declaration_count: parsed.declaration_count,
                source_order: rules.len(),
                min_width: None,
                max_width: None,
            });
            cursor = close + 1;
        }
        Ok(Self { rules })
    }

    /// Resolve matched rules against a previously resolved theme token map.
    /// Specificity wins first; equal-specificity declarations follow source order.
    pub fn resolve(
        &self,
        context: &ThemeStyleContext,
        tokens: &BTreeMap<String, ThemeValue>,
    ) -> Result<BTreeMap<ThemeStyleProperty, ThemeValue>, ThemeError> {
        validate_context(context)?;
        let mut matched = self
            .rules
            .iter()
            .filter_map(|rule| {
                let width_matches = match (rule.min_width, rule.max_width, context.viewport_width) {
                    (None, None, _) => true,
                    (Some(min), max, Some(width)) => {
                        width >= min && max.is_none_or(|max| width <= max)
                    }
                    (None, Some(max), Some(width)) => width <= max,
                    _ => false,
                };
                if !width_matches {
                    return None;
                }
                rule.selectors
                    .iter()
                    .filter(|selector| selector.matches(context))
                    .map(StyleSelector::specificity)
                    .max()
                    .map(|specificity| (specificity, rule))
            })
            .collect::<Vec<_>>();
        matched.sort_by_key(|(specificity, rule)| (*specificity, rule.source_order));
        let mut resolved = BTreeMap::new();
        for (_, rule) in matched {
            for declaration in &rule.declarations {
                let value = match &declaration.value {
                    StyleValue::Literal(value) => value.clone(),
                    StyleValue::Token(token) => tokens.get(token).cloned().ok_or_else(|| {
                        style_error(
                            ThemeErrorCode::InvalidReference,
                            "Theme stylesheet references an unavailable token.",
                            token,
                        )
                    })?,
                };
                validate_property_value(declaration.property, &value)?;
                resolved.insert(declaration.property, value);
            }
        }
        Ok(resolved)
    }

    /// Resolve the winning `transition` declaration for a component context.
    /// The declaration is a bounded comma-separated list of explicit
    /// interpolable properties, duration (`ms` or `s`), and an optional easing
    /// keyword. It schedules no work and does not mutate style or layout state.
    pub fn resolve_transitions(
        &self,
        context: &ThemeStyleContext,
    ) -> Result<BTreeMap<ThemeStyleProperty, ThemeStyleTransitionSpec>, ThemeError> {
        validate_context(context)?;
        let mut matched = self
            .rules
            .iter()
            .filter_map(|rule| {
                let width_matches = match (rule.min_width, rule.max_width, context.viewport_width) {
                    (None, None, _) => true,
                    (Some(min), max, Some(width)) => {
                        width >= min && max.is_none_or(|max| width <= max)
                    }
                    (None, Some(max), Some(width)) => width <= max,
                    _ => false,
                };
                if !width_matches {
                    return None;
                }
                rule.selectors
                    .iter()
                    .filter(|selector| selector.matches(context))
                    .map(StyleSelector::specificity)
                    .max()
                    .map(|specificity| (specificity, rule))
            })
            .collect::<Vec<_>>();
        matched.sort_by_key(|(specificity, rule)| (*specificity, rule.source_order));
        let mut resolved = BTreeMap::new();
        for (_, rule) in matched {
            if let Some(transitions) = &rule.transitions {
                resolved = transitions
                    .iter()
                    .map(|spec| (spec.property, *spec))
                    .collect();
            }
        }
        Ok(resolved)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

fn matching_brace(source: &str, open: usize) -> Result<usize, ThemeError> {
    let mut depth = 0usize;
    for (offset, byte) in source.as_bytes().iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(offset);
                }
            }
            _ => {}
        }
    }
    Err(style_error(
        ThemeErrorCode::InvalidParameter,
        "Theme media rule is missing a closing brace.",
        format!("offset={open}"),
    ))
}

fn parse_media_condition(source: &str) -> Result<(Option<f64>, Option<f64>), ThemeError> {
    let condition = source
        .strip_prefix("@media ")
        .and_then(|value| value.strip_prefix('('))
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| {
            style_error(
                ThemeErrorCode::InvalidParameter,
                "Only simple viewport-width media rules are supported.",
                source,
            )
        })?;
    let (name, value) = condition.split_once(':').ok_or_else(|| {
        style_error(
            ThemeErrorCode::InvalidParameter,
            "Media condition must specify a viewport width.",
            source,
        )
    })?;
    let value = value.trim().strip_suffix("px").ok_or_else(|| {
        style_error(
            ThemeErrorCode::InvalidParameter,
            "Viewport media widths must use logical px.",
            source,
        )
    })?;
    let width = value.parse::<f64>().map_err(|_| {
        style_error(
            ThemeErrorCode::TypeMismatch,
            "Viewport media width must be numeric.",
            source,
        )
    })?;
    if !width.is_finite() || !(1.0..=1_000_000.0).contains(&width) {
        return Err(style_error(
            ThemeErrorCode::InvalidParameter,
            "Viewport media width is outside its supported bounds.",
            source,
        ));
    }
    match name.trim() {
        "min-width" => Ok((Some(width), None)),
        "max-width" => Ok((None, Some(width))),
        _ => Err(style_error(
            ThemeErrorCode::InvalidParameter,
            "Only min-width and max-width media conditions are supported.",
            source,
        )),
    }
}

fn strip_comments(source: &str) -> Result<String, ThemeError> {
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("/*") {
        output.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        let end = tail.find("*/").ok_or_else(|| {
            style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme stylesheet comment is not terminated.",
                format!("offset={}", source.len() - rest.len() + start),
            )
        })?;
        output.push(' ');
        rest = &tail[end + 2..];
    }
    output.push_str(rest);
    Ok(output)
}

fn skip_space(source: &str, mut cursor: usize) -> usize {
    while cursor < source.len() {
        let Some(character) = source[cursor..].chars().next() else {
            break;
        };
        if !character.is_whitespace() {
            break;
        }
        cursor += character.len_utf8();
    }
    cursor
}

fn parse_selector(source: &str) -> Result<StyleSelector, ThemeError> {
    if source.is_empty()
        || source.len() > MAX_SELECTOR_LENGTH
        || source.chars().any(char::is_whitespace)
    {
        return Err(style_error(
            ThemeErrorCode::InvalidParameter,
            "Theme selector is empty, too long, or contains unsupported whitespace.",
            source,
        ));
    }
    let mut selector = StyleSelector::default();
    let mut main = source;
    if let Some(colon) = source.find(':') {
        let state = &source[colon + 1..];
        if state.contains(':') {
            return Err(style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme selectors may contain one state suffix.",
                source,
            ));
        }
        selector.state = Some(ThemeStyleState::parse(state).ok_or_else(|| {
            style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme selector uses an unsupported component state.",
                state,
            )
        })?);
        main = &source[..colon];
    }
    if main.is_empty() {
        return Ok(selector);
    }
    let (target, name) = match main.as_bytes()[0] {
        b'#' => ("id", &main[1..]),
        b'.' => ("class", &main[1..]),
        _ => ("role", main),
    };
    validate_selector_name(name, target)?;
    match target {
        "id" => selector.id = Some(name.to_owned()),
        "class" => selector.class = Some(name.to_owned()),
        _ => selector.role = Some(name.to_owned()),
    }
    Ok(selector)
}

struct ParsedDeclarations {
    declarations: Vec<StyleDeclaration>,
    transitions: Option<Vec<ThemeStyleTransitionSpec>>,
    declaration_count: usize,
}

fn parse_declarations(source: &str) -> Result<ParsedDeclarations, ThemeError> {
    let mut declarations = Vec::new();
    let mut transitions = None;
    let mut declaration_count = 0usize;
    for item in source
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let (property, value) = item.split_once(':').ok_or_else(|| {
            style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme declaration must contain a property and value.",
                item,
            )
        })?;
        let property = property.trim();
        let value_text = value.trim();
        if value_text.is_empty() || value_text.len() > MAX_STYLE_VALUE_LENGTH {
            return Err(style_error(
                ThemeErrorCode::ResourceLimit,
                "Theme declaration value is empty or too long.",
                property,
            ));
        }
        declaration_count = declaration_count.saturating_add(1);
        if property == "transition" {
            transitions = Some(parse_transition_specs(value_text)?);
            continue;
        }
        let property = ThemeStyleProperty::parse(property)?;
        let value = if let Some(token) = value_text
            .strip_prefix("var(--")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            validate_selector_name(token, "theme token reference")?;
            StyleValue::Token(token.to_owned())
        } else {
            let literal = parse_literal(property, value_text)?;
            validate_property_value(property, &literal)?;
            StyleValue::Literal(literal)
        };
        declarations.push(StyleDeclaration { property, value });
    }
    Ok(ParsedDeclarations {
        declarations,
        transitions,
        declaration_count,
    })
}

fn parse_transition_specs(source: &str) -> Result<Vec<ThemeStyleTransitionSpec>, ThemeError> {
    let mut specs = Vec::new();
    let mut seen = BTreeSet::new();
    for item in source.split(',').map(str::trim) {
        if item.is_empty() || specs.len() >= MAX_TRANSITION_TRACKS {
            return Err(style_error(
                ThemeErrorCode::ResourceLimit,
                "Theme transition list is empty or exceeds its track limit.",
                MAX_TRANSITION_TRACKS.to_string(),
            ));
        }
        let mut fields = item.split_ascii_whitespace();
        let property_name = fields.next().unwrap_or_default();
        let property = ThemeStyleProperty::parse(property_name)?;
        if property == ThemeStyleProperty::FontFamily {
            return Err(style_error(
                ThemeErrorCode::InvalidParameter,
                "Discrete font-family values cannot be interpolated by theme transitions.",
                property_name,
            ));
        }
        if !seen.insert(property) {
            return Err(style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme transition list contains a duplicate property.",
                property_name,
            ));
        }
        let duration = fields.next().ok_or_else(|| {
            style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme transition entry must include a duration.",
                item,
            )
        })?;
        let duration_seconds = parse_transition_duration(duration)?;
        let easing = match fields.next().unwrap_or("ease") {
            "linear" => ThemeStyleEasing::Linear,
            "ease" => ThemeStyleEasing::Ease,
            "ease-in" => ThemeStyleEasing::EaseIn,
            "ease-out" => ThemeStyleEasing::EaseOut,
            "ease-in-out" => ThemeStyleEasing::EaseInOut,
            other => {
                return Err(style_error(
                    ThemeErrorCode::InvalidParameter,
                    "Theme transition easing keyword is unsupported.",
                    other,
                ));
            }
        };
        if fields.next().is_some() {
            return Err(style_error(
                ThemeErrorCode::InvalidParameter,
                "Theme transition entry has unsupported extra values or delay.",
                item,
            ));
        }
        specs.push(ThemeStyleTransitionSpec {
            property,
            duration_seconds,
            easing,
        });
    }
    Ok(specs)
}

fn parse_transition_duration(source: &str) -> Result<f64, ThemeError> {
    let (amount, multiplier) = if let Some(value) = source.strip_suffix("ms") {
        (value, 0.001)
    } else if let Some(value) = source.strip_suffix('s') {
        (value, 1.0)
    } else {
        return Err(style_error(
            ThemeErrorCode::InvalidParameter,
            "Theme transition duration must use ms or s units.",
            source,
        ));
    };
    let amount = amount.parse::<f64>().map_err(|_| {
        style_error(
            ThemeErrorCode::TypeMismatch,
            "Theme transition duration must be numeric.",
            source,
        )
    })?;
    let duration = amount * multiplier;
    if !duration.is_finite() || !(0.0..=MAX_TRANSITION_SECONDS).contains(&duration) {
        return Err(style_error(
            ThemeErrorCode::InvalidParameter,
            "Theme transition duration is outside 0..=10 seconds.",
            source,
        ));
    }
    Ok(duration)
}

impl ThemeStyleProperty {
    fn parse(name: &str) -> Result<Self, ThemeError> {
        Ok(match name {
            "color" => Self::Color,
            "background-color" => Self::BackgroundColor,
            "border-color" => Self::BorderColor,
            "padding" => Self::Padding,
            "margin" => Self::Margin,
            "border-width" => Self::BorderWidth,
            "border-radius" => Self::BorderRadius,
            "font-size" => Self::FontSize,
            "font-family" => Self::FontFamily,
            "opacity" => Self::Opacity,
            _ => {
                return Err(style_error(
                    ThemeErrorCode::InvalidParameter,
                    "Theme stylesheet property is unsupported.",
                    name,
                ));
            }
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::BackgroundColor => "background-color",
            Self::BorderColor => "border-color",
            Self::Padding => "padding",
            Self::Margin => "margin",
            Self::BorderWidth => "border-width",
            Self::BorderRadius => "border-radius",
            Self::FontSize => "font-size",
            Self::FontFamily => "font-family",
            Self::Opacity => "opacity",
        }
    }
}

fn parse_literal(property: ThemeStyleProperty, source: &str) -> Result<ThemeValue, ThemeError> {
    if matches!(
        property,
        ThemeStyleProperty::Color
            | ThemeStyleProperty::BackgroundColor
            | ThemeStyleProperty::BorderColor
    ) {
        return if valid_color(source) {
            Ok(ThemeValue::Color(source.to_ascii_lowercase()))
        } else {
            Err(style_error(
                ThemeErrorCode::TypeMismatch,
                "Theme color must use #RRGGBB or #RRGGBBAA notation.",
                source,
            ))
        };
    }
    if property == ThemeStyleProperty::FontFamily {
        let value = source
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or_else(|| {
                style_error(
                    ThemeErrorCode::TypeMismatch,
                    "Font family must be a quoted text value.",
                    source,
                )
            })?;
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(style_error(
                ThemeErrorCode::InvalidParameter,
                "Font family is empty or outside its bounds.",
                source,
            ));
        }
        return Ok(ThemeValue::Text(value.to_owned()));
    }
    if property == ThemeStyleProperty::Opacity {
        let value = source.parse::<f64>().map_err(|_| {
            style_error(
                ThemeErrorCode::TypeMismatch,
                "Opacity must be a finite number in 0..=1.",
                source,
            )
        })?;
        return Ok(ThemeValue::Number(value));
    }
    let (amount_source, unit) = if let Some(amount) = source.strip_suffix("px") {
        (amount, ThemeMetricUnit::LogicalPx)
    } else {
        (source, ThemeMetricUnit::Unitless)
    };
    let amount = amount_source.parse::<f64>().map_err(|_| {
        style_error(
            ThemeErrorCode::TypeMismatch,
            "Metric values must be a number with optional px units.",
            source,
        )
    })?;
    Ok(ThemeValue::Metric(ThemeMetric { amount, unit }))
}

fn validate_property_value(
    property: ThemeStyleProperty,
    value: &ThemeValue,
) -> Result<(), ThemeError> {
    let valid = match (property, value) {
        (
            ThemeStyleProperty::Color
            | ThemeStyleProperty::BackgroundColor
            | ThemeStyleProperty::BorderColor,
            ThemeValue::Color(color),
        ) => valid_color(color),
        (ThemeStyleProperty::Opacity, ThemeValue::Number(number)) => {
            number.is_finite() && (0.0..=1.0).contains(number)
        }
        (ThemeStyleProperty::FontFamily, ThemeValue::Text(text)) => {
            !text.is_empty() && text.len() <= 128 && !text.chars().any(char::is_control)
        }
        (
            ThemeStyleProperty::Padding
            | ThemeStyleProperty::Margin
            | ThemeStyleProperty::BorderWidth
            | ThemeStyleProperty::BorderRadius
            | ThemeStyleProperty::FontSize,
            ThemeValue::Metric(metric),
        ) => {
            metric.amount.is_finite()
                && (0.0..=4096.0).contains(&metric.amount)
                && (property != ThemeStyleProperty::FontSize || metric.amount >= 1.0)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(style_error(
            ThemeErrorCode::TypeMismatch,
            "Resolved theme value does not match the style property's supported type or bounds.",
            property.name(),
        ))
    }
}

fn valid_color(value: &str) -> bool {
    value.is_ascii()
        && matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_context(context: &ThemeStyleContext) -> Result<(), ThemeError> {
    if context
        .viewport_width
        .is_some_and(|width| !width.is_finite() || !(1.0..=1_000_000.0).contains(&width))
    {
        return Err(style_error(
            ThemeErrorCode::InvalidParameter,
            "Theme viewport width is outside its supported bounds.",
            format!("{:?}", context.viewport_width),
        ));
    }
    if let Some(role) = &context.role {
        validate_selector_name(role, "component role")?;
    }
    if let Some(id) = &context.id {
        validate_selector_name(id, "component id")?;
    }
    for class in &context.classes {
        validate_selector_name(class, "component class")?;
    }
    Ok(())
}

fn validate_selector_name(name: &str, context: &str) -> Result<(), ThemeError> {
    if name.is_empty()
        || name.len() > 64
        || !name.as_bytes()[0].is_ascii_lowercase() && !name.as_bytes()[0].is_ascii_digit()
        || !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        || name.ends_with(['.', '_', '-'])
        || name.contains("..")
    {
        return Err(style_error(
            ThemeErrorCode::InvalidIdentifier,
            "Theme selector and token names must use bounded lowercase ASCII identifiers.",
            format!("{context}: {name}"),
        ));
    }
    Ok(())
}

fn style_error(
    code: ThemeErrorCode,
    message: impl Into<String>,
    context: impl Into<String>,
) -> ThemeError {
    ThemeError::new(code, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stylesheet_resolves_typed_tokens_states_and_specificity_deterministically() {
        let sheet = ThemeStyleSheet::parse(
            r#"
                /* Role defaults */
                button { color: var(--text-primary); padding: 8px; opacity: 1; }
                .danger { color: #AA0000; }
                button:hover { padding: 10px; }
                #save-button:disabled { opacity: 0.5; }
            "#,
        )
        .unwrap();
        let context = ThemeStyleContext::new("button")
            .unwrap()
            .with_id("save-button")
            .unwrap()
            .with_class("danger")
            .unwrap()
            .with_state(ThemeStyleState::Hover)
            .with_state(ThemeStyleState::Disabled);
        let tokens = BTreeMap::from([(
            "text-primary".to_owned(),
            ThemeValue::Color("#F0F0F0".to_owned()),
        )]);
        let first = sheet.resolve(&context, &tokens).unwrap();
        let second = sheet.resolve(&context, &tokens).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first[&ThemeStyleProperty::Color],
            ThemeValue::Color("#aa0000".into())
        );
        assert_eq!(
            first[&ThemeStyleProperty::Padding],
            ThemeValue::Metric(ThemeMetric {
                amount: 10.0,
                unit: ThemeMetricUnit::LogicalPx,
            })
        );
        assert_eq!(first[&ThemeStyleProperty::Opacity], ThemeValue::Number(0.5));
    }

    #[test]
    fn stylesheet_resolves_property_transition_specs_with_cascade_and_viewport_rules() {
        let sheet = ThemeStyleSheet::parse(
            r#"
                button { color: #000000; transition: color 200ms linear, opacity 0.4s ease-in; }
                button:hover { transition: color 120ms ease-out; }
                @media (min-width: 600px) {
                    #save:hover { transition: padding 1.25s ease-in-out; }
                }
            "#,
        )
        .unwrap();
        let base_context = ThemeStyleContext::new("button").unwrap();
        let base = sheet.resolve_transitions(&base_context).unwrap();
        assert_eq!(base.len(), 2);
        assert_eq!(base[&ThemeStyleProperty::Color].duration_seconds(), 0.2);
        assert_eq!(
            base[&ThemeStyleProperty::Color].easing(),
            ThemeStyleEasing::Linear
        );
        assert_eq!(base[&ThemeStyleProperty::Opacity].duration_seconds(), 0.4);
        assert_eq!(
            base[&ThemeStyleProperty::Opacity].easing(),
            ThemeStyleEasing::EaseIn
        );

        let hover_context = base_context.clone().with_state(ThemeStyleState::Hover);
        let hover = sheet.resolve_transitions(&hover_context).unwrap();
        assert_eq!(hover.len(), 1);
        assert_eq!(hover[&ThemeStyleProperty::Color].duration_seconds(), 0.12);
        assert_eq!(
            hover[&ThemeStyleProperty::Color].easing(),
            ThemeStyleEasing::EaseOut
        );

        let wide_hover = hover_context
            .with_id("save")
            .unwrap()
            .with_viewport_width(800.0)
            .unwrap();
        let wide = sheet.resolve_transitions(&wide_hover).unwrap();
        assert_eq!(wide.len(), 1);
        assert_eq!(wide[&ThemeStyleProperty::Padding].duration_seconds(), 1.25);
        assert_eq!(
            wide[&ThemeStyleProperty::Padding].easing(),
            ThemeStyleEasing::EaseInOut
        );
    }

    #[test]
    fn stylesheet_rejects_unbounded_or_discrete_transition_declarations() {
        for source in [
            "button { transition: font-family 200ms ease; }",
            "button { transition: all 200ms ease; }",
            "button { transition: color 200ms ease, color 300ms linear; }",
            "button { transition: color 11s ease; }",
            "button { transition: color 200ms spring; }",
            "button { transition: color 200ms ease 20ms; }",
            "button { transition: color 200; }",
        ] {
            assert!(ThemeStyleSheet::parse(source).is_err(), "accepted {source}");
        }
        let too_many = (0..=MAX_TRANSITION_TRACKS)
            .map(|index| format!("opacity {}ms linear", 10 + index))
            .collect::<Vec<_>>()
            .join(", ");
        assert!(ThemeStyleSheet::parse(&format!("button {{ transition: {too_many}; }}")).is_err());
    }

    #[test]
    fn stylesheet_rejects_unsupported_css_nested_rules_and_invalid_values() {
        assert!(ThemeStyleSheet::parse("button { transform: rotate(3deg); }").is_err());
        assert!(
            ThemeStyleSheet::parse("@media (orientation: landscape) { button { opacity: 1; } }")
                .is_err()
        );
        let mut nested = "button { opacity: 1; }".to_owned();
        for _ in 0..256 {
            nested = format!("@media (min-width: 400px) {{ {nested} }}");
        }
        assert!(ThemeStyleSheet::parse(&nested).is_err());
        assert!(ThemeStyleSheet::parse("button { color: red; }").is_err());
        assert!(ThemeStyleSheet::parse("button { padding: -1px; }").is_err());
        assert!(ThemeStyleSheet::parse("button { opacity: 2; }").is_err());
        assert!(ThemeStyleSheet::parse("button { font-size: 0px; }").is_err());
        assert!(
            ThemeStyleSheet::parse("button { color: var(--missing); }")
                .unwrap()
                .resolve(&ThemeStyleContext::new("button").unwrap(), &BTreeMap::new())
                .is_err()
        );
    }

    #[test]
    fn stylesheet_resolves_bounded_viewport_width_rules() {
        let sheet = ThemeStyleSheet::parse(
            "button { padding: 8px; } @media (max-width: 480px) { button { padding: 4px; } } @media (min-width: 900px) { button { padding: 12px; } }",
        ).unwrap();
        let tokens = BTreeMap::new();
        let resolve = |width| {
            sheet
                .resolve(
                    &ThemeStyleContext::new("button")
                        .unwrap()
                        .with_viewport_width(width)
                        .unwrap(),
                    &tokens,
                )
                .unwrap()
        };
        assert_eq!(
            resolve(400.0)[&ThemeStyleProperty::Padding],
            ThemeValue::Metric(ThemeMetric {
                amount: 4.0,
                unit: ThemeMetricUnit::LogicalPx
            })
        );
        assert_eq!(
            resolve(700.0)[&ThemeStyleProperty::Padding],
            ThemeValue::Metric(ThemeMetric {
                amount: 8.0,
                unit: ThemeMetricUnit::LogicalPx
            })
        );
        assert_eq!(
            resolve(1200.0)[&ThemeStyleProperty::Padding],
            ThemeValue::Metric(ThemeMetric {
                amount: 12.0,
                unit: ThemeMetricUnit::LogicalPx
            })
        );
        assert_eq!(
            sheet
                .resolve(&ThemeStyleContext::new("button").unwrap(), &tokens)
                .unwrap()[&ThemeStyleProperty::Padding],
            ThemeValue::Metric(ThemeMetric {
                amount: 8.0,
                unit: ThemeMetricUnit::LogicalPx
            })
        );
        assert!(
            ThemeStyleContext::new("button")
                .unwrap()
                .with_viewport_width(f64::NAN)
                .is_err()
        );
        assert!(
            ThemeStyleSheet::parse("@media (min-width: -1px) { button { opacity: 1; } }").is_err()
        );
    }

    #[test]
    fn stylesheet_enforces_rule_declaration_and_identifier_bounds() {
        assert!(ThemeStyleSheet::parse("#Upper { opacity: 1; }").is_err());
        assert!(ThemeStyleSheet::parse("button:hover:active { opacity: 1; }").is_err());
        let oversized = format!("button {{ font-family: \"{}\"; }}", "x".repeat(129));
        assert!(ThemeStyleSheet::parse(&oversized).is_err());
        let oversized_sheet = " ".repeat(MAX_STYLESHEET_BYTES + 1);
        assert_eq!(
            ThemeStyleSheet::parse(&oversized_sheet).unwrap_err().code,
            ThemeErrorCode::ResourceLimit
        );
    }
}
