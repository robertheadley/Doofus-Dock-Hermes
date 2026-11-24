//! Versioned, toolkit-neutral theme packages and deterministic resolution.
//!
//! Themes are declarative presentation data. They never own layout, input, focus, or
//! application behavior. Asset files stay outside the manifest and URNE projection.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const THEME_SCHEMA_VERSION: u32 = 3;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const THEME_PACKAGE_FORMAT_VERSION: u32 = 1;
const THEME_STYLE_PACKAGE_FORMAT_VERSION: u32 = 1;
const MAX_THEME_STYLE_PACKAGE_BYTES: usize = 128 * 1024;
const MAX_JSON_DEPTH: usize = 64;
const MAX_STRING_LENGTH: usize = 4096;
const MAX_TOKENS: usize = 512;
const MAX_VARIANTS: usize = 16;
const MAX_VARIANT_DEPTH: usize = 8;
const MAX_COMPONENTS: usize = 128;
const MAX_STATES_PER_COMPONENT: usize = 32;
const MAX_SVG_BINDINGS_PER_STATE: usize = 64;
const MAX_TOTAL_SVG_BINDINGS: usize = 2048;
const MAX_PARAMETERS: usize = 128;
const MAX_ASSETS: usize = 128;
const MAX_ASSET_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SVG_ASSET_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SVG_NODES: usize = 50_000;
const MAX_ASSET_DIMENSION: u32 = 8192;
const MAX_DEPENDENCIES: usize = 64;
const MAX_CONTRAST_PAIRS: usize = 128;
pub(crate) const MAX_URNE_NODES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeErrorCode {
    InvalidJson,
    UnsupportedSchemaVersion,
    InvalidIdentifier,
    DuplicateIdentifier,
    ResourceLimit,
    InvalidInheritance,
    InvalidReference,
    ReferenceCycle,
    TypeMismatch,
    InvalidParameter,
    InvalidAsset,
    InsufficientContrast,
    IncompatibleSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeError {
    pub code: ThemeErrorCode,
    pub message: String,
    pub context: String,
}

impl ThemeError {
    pub(crate) fn new(
        code: ThemeErrorCode,
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            context: context.into(),
        }
    }
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.message, self.context)
    }
}

impl std::error::Error for ThemeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeDiagnosticCode {
    AssetUnverified,
    AssetUnavailable,
    UnsupportedAssetFormat,
    UnsupportedFeature,
    FallbackApplied,
    DecorativeAssetOmitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDiagnostic {
    pub code: ThemeDiagnosticCode,
    pub severity: ThemeDiagnosticSeverity,
    pub message: String,
    pub context: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub id: String,
    pub metadata: ThemePackageMetadata,
    pub tokens: BTreeMap<String, ThemeValue>,
    pub variants: BTreeMap<String, ThemeVariant>,
    #[serde(default)]
    pub components: BTreeMap<String, ThemeComponent>,
    #[serde(default)]
    pub parameters: BTreeMap<String, ThemeParameter>,
    #[serde(default)]
    pub assets: Vec<ThemeAsset>,
    #[serde(default)]
    pub dependencies: Vec<ThemeDependency>,
    #[serde(default)]
    pub contrast_pairs: Vec<ThemeContrastPair>,
    pub provenance: ThemeProvenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePackageMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeProvenance {
    pub kind: ThemeProvenanceKind,
    pub source: String,
    #[serde(default)]
    pub generator: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeProvenanceKind {
    HandAuthored,
    GeneratedAssets,
    Adapted,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeVariant {
    pub label: String,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub tokens: BTreeMap<String, ThemeValue>,
    #[serde(default)]
    pub components: BTreeMap<String, ThemeComponent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeComponent {
    pub states: BTreeMap<String, ThemeComponentState>,
}

/// Visual treatment for a semantic state supplied by the host. No behavior is encoded here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeComponentState {
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub tokens: BTreeMap<String, ThemeValue>,
    #[serde(default)]
    pub fallback: Option<BTreeMap<String, ThemeValue>>,
    #[serde(default)]
    pub svg_bindings: Vec<ThemeSvgStyleBinding>,
}

/// Binds one typed component-state token to a restricted property on a simple SVG selector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeSvgStyleBinding {
    pub asset_id: String,
    pub selector: String,
    pub property: ThemeSvgStyleProperty,
    /// Key in the containing component state's `tokens` map.
    pub token: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeSvgStyleProperty {
    Fill,
    Stroke,
    Color,
    Opacity,
    StrokeWidth,
}

/// V1 supports one deliberately narrow mapping: a bounded metric parameter replaces
/// the numeric component of a metric token while retaining its declared unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeParameter {
    pub kind: ThemeParameterKind,
    pub default: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub unit: ThemeMetricUnit,
    pub target_token: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeParameterKind {
    Metric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMetricUnit {
    LogicalPx,
    SourcePx,
    Unitless,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ThemeValue {
    Color(String),
    Metric(ThemeMetric),
    Number(f64),
    Boolean(bool),
    Text(String),
    Reference(String),
    Asset(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeMetric {
    pub amount: f64,
    pub unit: ThemeMetricUnit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeAsset {
    pub id: String,
    pub path: String,
    pub format: ThemeAssetFormat,
    pub source_width: u32,
    pub source_height: u32,
    pub scale: f32,
    pub transparency: ThemeTransparency,
    #[serde(default)]
    pub color_key: Option<String>,
    #[serde(default)]
    pub nine_slice: Option<ThemeNineSlice>,
    pub edge_tiling: ThemeTiling,
    pub center_tiling: ThemeTiling,
    #[serde(default)]
    pub provenance: Option<ThemeAssetProvenance>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAssetFormat {
    Png,
    Svg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeTransparency {
    Alpha,
    ColorKey,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeTiling {
    Stretch,
    Tile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeNineSlice {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeAssetProvenance {
    pub kind: ThemeProvenanceKind,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDependency {
    pub id: String,
    pub version_requirement: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeContrastPair {
    pub id: String,
    pub foreground: String,
    pub background: String,
    pub target: ThemeContrastTarget,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeContrastTarget {
    NormalTextAa,
    LargeTextAa,
    NonTextAa,
    NormalTextAaa,
    LargeTextAaa,
}

impl ThemeContrastTarget {
    pub fn minimum_ratio(self) -> f64 {
        match self {
            Self::NormalTextAa => 4.5,
            Self::LargeTextAa | Self::NonTextAa => 3.0,
            Self::NormalTextAaa => 7.0,
            Self::LargeTextAaa => 4.5,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeAdapterCapabilities {
    #[serde(default)]
    pub supported_features: BTreeSet<String>,
    #[serde(default)]
    pub supported_asset_formats: BTreeSet<ThemeAssetFormatName>,
    #[serde(default)]
    pub supports_raster_assets: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAssetFormatName {
    Png,
    Svg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTheme {
    pub theme_id: String,
    pub variant_id: String,
    pub manifest_fingerprint: String,
    pub tokens: BTreeMap<String, ThemeValue>,
    pub components: BTreeMap<String, BTreeMap<String, BTreeMap<String, ThemeValue>>>,
    #[serde(default)]
    pub svg_bindings: BTreeMap<String, BTreeMap<String, Vec<ThemeSvgStyleBinding>>>,
    pub parameters: BTreeMap<String, f64>,
    pub assets: Vec<ResolvedThemeAsset>,
    pub contrast: Vec<ThemeContrastResult>,
    pub diagnostics: Vec<ThemeDiagnostic>,
    /// Stable FNV-1a fingerprint for reproducibility checks, not a signature.
    pub fingerprint: String,
    /// Effective value provenance keyed by token or component/state/property path.
    pub value_sources: BTreeMap<String, String>,
}

impl ResolvedTheme {
    /// Build a bounded stylesheet for one resolved component state and SVG asset.
    /// The returned CSS contains only validated selectors, allowlisted properties,
    /// and typed resolved token values.
    pub fn svg_stylesheet(
        &self,
        asset_id: &str,
        component_id: &str,
        state_id: &str,
    ) -> Result<String, ThemeError> {
        self.validate_snapshot()?;
        let asset = self
            .assets
            .iter()
            .find(|asset| asset.id == asset_id && asset.format == ThemeAssetFormat::Svg)
            .ok_or_else(|| {
                ThemeError::new(
                    ThemeErrorCode::InvalidReference,
                    "SVG stylesheet requested for an undeclared vector asset.",
                    format!("assets.{asset_id}"),
                )
            })?;
        if !asset.enabled {
            return Ok(String::new());
        }
        let Some(bindings) = self
            .svg_bindings
            .get(component_id)
            .and_then(|states| states.get(state_id))
        else {
            return Ok(String::new());
        };
        let Some(tokens) = self
            .components
            .get(component_id)
            .and_then(|states| states.get(state_id))
        else {
            return Ok(String::new());
        };
        let mut stylesheet = String::new();
        for binding in bindings
            .iter()
            .filter(|binding| binding.asset_id == asset_id)
        {
            let Some(value) = tokens.get(&binding.token) else {
                continue;
            };
            let value = svg_css_value(binding.property, value)?;
            let property = match binding.property {
                ThemeSvgStyleProperty::Fill => "fill",
                ThemeSvgStyleProperty::Stroke => "stroke",
                ThemeSvgStyleProperty::Color => "color",
                ThemeSvgStyleProperty::Opacity => "opacity",
                ThemeSvgStyleProperty::StrokeWidth => "stroke-width",
            };
            stylesheet.push_str(&binding.selector);
            stylesheet.push_str(" { ");
            stylesheet.push_str(property);
            stylesheet.push_str(": ");
            stylesheet.push_str(&value);
            stylesheet.push_str("; }\n");
            if stylesheet.len() > 64 * 1024 {
                return Err(ThemeError::new(
                    ThemeErrorCode::ResourceLimit,
                    "Resolved SVG stylesheet exceeds the 64 KiB limit.",
                    format!("components.{component_id}.states.{state_id}"),
                ));
            }
        }
        Ok(stylesheet)
    }

    /// Confirm the reproducible content fingerprint before a host consumes or projects this value.
    pub fn validate_snapshot(&self) -> Result<(), ThemeError> {
        let mut canonical = self.clone();
        let expected = std::mem::take(&mut canonical.fingerprint);
        let bytes = serde_json::to_vec(&canonical).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                "resolved_theme",
            )
        })?;
        if fnv1a64(&bytes) != expected && legacy_resolved_theme_fingerprint(&canonical) != expected
        {
            return Err(ThemeError::new(
                ThemeErrorCode::IncompatibleSnapshot,
                "Resolved theme fingerprint does not match its contents.",
                self.theme_id.clone(),
            ));
        }
        Ok(())
    }
}

/// Recreate the canonical shape used before SVG bindings were added to resolved snapshots.
/// This keeps persisted snapshots loadable while allowing the new field to default empty.
#[derive(Serialize)]
struct LegacyResolvedThemeFingerprint<'a> {
    theme_id: &'a str,
    variant_id: &'a str,
    manifest_fingerprint: &'a str,
    tokens: &'a BTreeMap<String, ThemeValue>,
    components: &'a BTreeMap<String, BTreeMap<String, BTreeMap<String, ThemeValue>>>,
    parameters: &'a BTreeMap<String, f64>,
    assets: &'a [ResolvedThemeAsset],
    contrast: &'a [ThemeContrastResult],
    diagnostics: &'a [ThemeDiagnostic],
    fingerprint: &'a str,
    value_sources: &'a BTreeMap<String, String>,
}

fn legacy_resolved_theme_fingerprint(theme: &ResolvedTheme) -> String {
    let legacy = LegacyResolvedThemeFingerprint {
        theme_id: &theme.theme_id,
        variant_id: &theme.variant_id,
        manifest_fingerprint: &theme.manifest_fingerprint,
        tokens: &theme.tokens,
        components: &theme.components,
        parameters: &theme.parameters,
        assets: &theme.assets,
        contrast: &theme.contrast,
        diagnostics: &theme.diagnostics,
        fingerprint: &theme.fingerprint,
        value_sources: &theme.value_sources,
    };
    let bytes = serde_json::to_vec(&legacy).expect("legacy snapshot serialization is infallible");
    fnv1a64(&bytes)
}

#[cfg(test)]
mod resolved_snapshot_compatibility_tests {
    use super::*;

    #[test]
    fn previous_resolved_snapshot_without_svg_bindings_still_validates() {
        let mut snapshot = ResolvedTheme {
            theme_id: "legacy".into(),
            variant_id: "dark".into(),
            manifest_fingerprint: "manifest-fingerprint".into(),
            tokens: BTreeMap::new(),
            components: BTreeMap::new(),
            svg_bindings: BTreeMap::new(),
            parameters: BTreeMap::new(),
            assets: Vec::new(),
            contrast: Vec::new(),
            diagnostics: Vec::new(),
            fingerprint: String::new(),
            value_sources: BTreeMap::new(),
        };
        snapshot.fingerprint = legacy_resolved_theme_fingerprint(&snapshot);

        let mut previous_json = serde_json::to_value(&snapshot).unwrap();
        previous_json
            .as_object_mut()
            .unwrap()
            .remove("svg_bindings");
        let restored: ResolvedTheme = serde_json::from_value(previous_json).unwrap();

        assert!(restored.svg_bindings.is_empty());
        restored
            .validate_snapshot()
            .expect("previous snapshot fingerprint remains valid");
    }
}

fn svg_css_value(
    property: ThemeSvgStyleProperty,
    value: &ThemeValue,
) -> Result<String, ThemeError> {
    match (property, value) {
        (
            ThemeSvgStyleProperty::Fill
            | ThemeSvgStyleProperty::Stroke
            | ThemeSvgStyleProperty::Color,
            ThemeValue::Color(color),
        ) if parse_color(color).is_some() => Ok(color.clone()),
        (ThemeSvgStyleProperty::Opacity, ThemeValue::Number(number))
            if number.is_finite() && (0.0..=1.0).contains(number) =>
        {
            Ok(number.to_string())
        }
        (ThemeSvgStyleProperty::StrokeWidth, ThemeValue::Number(number))
            if number.is_finite() && *number >= 0.0 =>
        {
            Ok(number.to_string())
        }
        (ThemeSvgStyleProperty::Opacity, ThemeValue::Metric(metric))
            if metric.amount.is_finite()
                && metric.unit == ThemeMetricUnit::Unitless
                && (0.0..=1.0).contains(&metric.amount) =>
        {
            Ok(metric.amount.to_string())
        }
        (ThemeSvgStyleProperty::StrokeWidth, ThemeValue::Metric(metric))
            if metric.amount.is_finite() && metric.amount >= 0.0 =>
        {
            let unit = match metric.unit {
                ThemeMetricUnit::LogicalPx => "px",
                ThemeMetricUnit::SourcePx | ThemeMetricUnit::Unitless => "",
            };
            Ok(format!("{}{unit}", metric.amount))
        }
        _ => Err(ThemeError::new(
            ThemeErrorCode::TypeMismatch,
            "Resolved theme value cannot be represented by the selected SVG CSS property.",
            "svg_stylesheet",
        )),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedThemeAsset {
    pub id: String,
    pub path: String,
    pub format: ThemeAssetFormat,
    pub source_width: u32,
    pub source_height: u32,
    pub scale: f32,
    pub transparency: ThemeTransparency,
    pub nine_slice: Option<ThemeNineSlice>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeContrastResult {
    pub pair_id: String,
    pub measured_ratio: f64,
    pub required_ratio: f64,
    pub passed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeAssetReport {
    manifest_id: String,
    verified_asset_ids: BTreeSet<String>,
    diagnostics: Vec<ThemeDiagnostic>,
}

impl ThemeAssetReport {
    pub fn is_verified(&self, asset_id: &str) -> bool {
        self.verified_asset_ids.contains(asset_id)
    }

    pub fn diagnostics(&self) -> &[ThemeDiagnostic] {
        &self.diagnostics
    }
}

impl ThemeManifest {
    pub fn parse_json(json: &str) -> Result<Self, ThemeError> {
        if json.len() > MAX_MANIFEST_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme manifest exceeds the 1 MiB byte limit.",
                "manifest",
            ));
        }
        check_json_depth(json)?;
        let mut document: serde_json::Value = serde_json::from_str(json).map_err(|error| {
            ThemeError::new(ThemeErrorCode::InvalidJson, error.to_string(), "manifest")
        })?;
        let manifest_value = if document.get("manifest").is_some() {
            let (expected, manifest) = {
                let Some(object) = document.as_object_mut() else {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        "Theme package envelope must be an object.",
                        "package_envelope",
                    ));
                };
                if object.len() != 3
                    || object
                        .get("format_version")
                        .and_then(serde_json::Value::as_u64)
                        != Some(u64::from(THEME_PACKAGE_FORMAT_VERSION))
                {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        "Theme package envelope is malformed or uses an unsupported format version.",
                        "package_envelope",
                    ));
                }
                let expected = object
                    .get("checksum")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        ThemeError::new(
                            ThemeErrorCode::InvalidJson,
                            "Theme package checksum is missing.",
                            "package_envelope.checksum",
                        )
                    })?
                    .to_owned();
                object.remove("checksum");
                let manifest = object.get("manifest").cloned().ok_or_else(|| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        "Theme package manifest is missing.",
                        "package_envelope.manifest",
                    )
                })?;
                (expected, manifest)
            };
            if theme_value_checksum(&document)? != expected {
                return Err(ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    "Theme package checksum does not match its contents.",
                    "package_envelope.checksum",
                ));
            }
            manifest
        } else {
            // Direct manifests are the unwrapped legacy package format. They
            // remain readable and are upgraded to a checksummed envelope on save.
            document
        };
        let manifest_value = migrate_theme_manifest_value(manifest_value)?;
        let manifest: Self = serde_json::from_value(manifest_value).map_err(|error| {
            ThemeError::new(ThemeErrorCode::InvalidJson, error.to_string(), "manifest")
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<String, ThemeError> {
        self.validate()?;
        let manifest = serde_json::to_value(self).map_err(|error| {
            ThemeError::new(ThemeErrorCode::InvalidJson, error.to_string(), "manifest")
        })?;
        let mut envelope = serde_json::json!({
            "format_version": THEME_PACKAGE_FORMAT_VERSION,
            "manifest": manifest
        });
        let checksum = theme_value_checksum(&envelope)?;
        envelope
            .as_object_mut()
            .expect("theme package envelope is an object")
            .insert("checksum".into(), serde_json::Value::String(checksum));
        let json = serde_json::to_string_pretty(&envelope).map_err(|error| {
            ThemeError::new(ThemeErrorCode::InvalidJson, error.to_string(), "manifest")
        })?;
        if json.len() > MAX_MANIFEST_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Checksummed theme package exceeds the 1 MiB manifest limit.",
                "manifest",
            ));
        }
        Ok(json)
    }

    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.schema_version != THEME_SCHEMA_VERSION {
            return Err(ThemeError::new(
                ThemeErrorCode::UnsupportedSchemaVersion,
                format!(
                    "Theme schema version {} is unsupported; expected {THEME_SCHEMA_VERSION}.",
                    self.schema_version
                ),
                "schema_version",
            ));
        }
        check_id(&self.id, "theme.id")?;
        check_string(&self.metadata.name, "metadata.name")?;
        check_string(&self.metadata.version, "metadata.version")?;
        check_string(&self.metadata.description, "metadata.description")?;
        check_string(&self.metadata.license, "metadata.license")?;
        for author in &self.metadata.authors {
            check_string(author, "metadata.authors")?;
        }
        if let Some(homepage) = &self.metadata.homepage {
            check_string(homepage, "metadata.homepage")?;
        }
        if self.tokens.is_empty() || self.tokens.len() > MAX_TOKENS {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme must declare 1 to 512 base tokens.",
                "tokens",
            ));
        }
        if self.variants.len() < 3 || self.variants.len() > MAX_VARIANTS {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme must declare light, dark, and high_contrast variants (3 to 16 total).",
                "variants",
            ));
        }
        for required in ["light", "dark", "high_contrast"] {
            if !self.variants.contains_key(required) {
                return Err(ThemeError::new(
                    ThemeErrorCode::InvalidReference,
                    "Required light, dark, or high_contrast variant is missing.",
                    format!("variants.{required}"),
                ));
            }
        }
        if self.components.len() > MAX_COMPONENTS
            || self.parameters.len() > MAX_PARAMETERS
            || self.assets.len() > MAX_ASSETS
            || self.dependencies.len() > MAX_DEPENDENCIES
            || self.contrast_pairs.len() > MAX_CONTRAST_PAIRS
        {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme exceeds a component, parameter, asset, dependency, or contrast-pair limit.",
                "manifest",
            ));
        }
        if self.contrast_pairs.is_empty() {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Theme must declare at least one critical contrast pair.",
                "contrast_pairs",
            ));
        }
        for (id, value) in &self.tokens {
            check_id(id, "tokens.id")?;
            validate_value(value, &format!("tokens.{id}"))?;
        }
        let mut total_tokens = self.tokens.len();
        for (variant_id, variant) in &self.variants {
            check_id(variant_id, "variants.id")?;
            check_string(&variant.label, &format!("variants.{variant_id}.label"))?;
            if let Some(parent) = &variant.extends {
                check_id(parent, &format!("variants.{variant_id}.extends"))?;
            }
            total_tokens = total_tokens.saturating_add(variant.tokens.len());
            for (id, value) in &variant.tokens {
                check_id(id, "variant.tokens.id")?;
                validate_value(value, &format!("variants.{variant_id}.tokens.{id}"))?;
            }
            validate_components(
                &variant.components,
                &format!("variants.{variant_id}.components"),
            )?;
        }
        if total_tokens > MAX_TOKENS * (MAX_VARIANTS + 1) {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme exceeds the total token-definition limit.",
                "variants.tokens",
            ));
        }
        validate_components(&self.components, "components")?;
        let total_svg_bindings: usize = self
            .components
            .values()
            .chain(
                self.variants
                    .values()
                    .flat_map(|variant| variant.components.values()),
            )
            .flat_map(|component| component.states.values())
            .map(|state| state.svg_bindings.len())
            .fold(0usize, usize::saturating_add);
        if total_svg_bindings > MAX_TOTAL_SVG_BINDINGS {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme exceeds the total SVG style binding limit of 2048.",
                "components.svg_bindings",
            ));
        }
        validate_variant_chain(self)?;
        for (id, parameter) in &self.parameters {
            check_id(id, "parameters.id")?;
            check_id(
                &parameter.target_token,
                &format!("parameters.{id}.target_token"),
            )?;
            if !parameter.minimum.is_finite()
                || !parameter.maximum.is_finite()
                || !parameter.default.is_finite()
                || !parameter.step.is_finite()
                || parameter.minimum > parameter.maximum
                || parameter.step <= 0.0
                || parameter.default < parameter.minimum
                || parameter.default > parameter.maximum
                || (((parameter.default - parameter.minimum) / parameter.step)
                    - ((parameter.default - parameter.minimum) / parameter.step).round())
                .abs()
                    > 1e-8
            {
                return Err(ThemeError::new(
                    ThemeErrorCode::InvalidParameter,
                    "Parameter default, range, or step is invalid.",
                    format!("parameters.{id}"),
                ));
            }
        }
        let mut asset_ids = BTreeSet::new();
        let mut asset_formats = BTreeMap::new();
        for asset in &self.assets {
            validate_asset(asset)?;
            if !asset_ids.insert(&asset.id) {
                return Err(ThemeError::new(
                    ThemeErrorCode::DuplicateIdentifier,
                    "Asset IDs must be unique.",
                    asset.id.clone(),
                ));
            }
            asset_formats.insert(asset.id.as_str(), asset.format);
        }
        for (component_id, component) in self.components.iter().chain(
            self.variants
                .values()
                .flat_map(|variant| variant.components.iter()),
        ) {
            for (state_id, state) in &component.states {
                for requirement in &state.requires {
                    if let Some(asset_id) = requirement.strip_prefix("asset-")
                        && !asset_ids.contains(&asset_id.to_owned())
                    {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidReference,
                            "Component state requires an asset that is not declared by this package.",
                            format!(
                                "components.{component_id}.states.{state_id}.requires:{requirement}"
                            ),
                        ));
                    }
                }
                validate_svg_bindings(
                    state,
                    &asset_formats,
                    &self.tokens,
                    &format!("components.{component_id}.states.{state_id}"),
                )?;
            }
        }
        for dependency in &self.dependencies {
            check_id(&dependency.id, "dependencies.id")?;
            check_string(
                &dependency.version_requirement,
                "dependencies.version_requirement",
            )?;
            if dependency.id == self.id {
                return Err(ThemeError::new(
                    ThemeErrorCode::InvalidReference,
                    "A theme cannot depend on itself.",
                    dependency.id.clone(),
                ));
            }
        }
        let mut pair_ids = BTreeSet::new();
        for pair in &self.contrast_pairs {
            check_id(&pair.id, "contrast_pairs.id")?;
            check_id(
                &pair.foreground,
                &format!("contrast_pairs.{}.foreground", pair.id),
            )?;
            check_id(
                &pair.background,
                &format!("contrast_pairs.{}.background", pair.id),
            )?;
            if pair.component.is_some() != pair.state.is_some() {
                return Err(ThemeError::new(
                    ThemeErrorCode::InvalidReference,
                    "Component contrast pairs must specify both component and state, or neither.",
                    format!("contrast_pairs.{}", pair.id),
                ));
            }
            if let Some(component) = &pair.component {
                check_id(component, &format!("contrast_pairs.{}.component", pair.id))?;
            }
            if let Some(state) = &pair.state {
                check_id(state, &format!("contrast_pairs.{}.state", pair.id))?;
            }
            if !pair_ids.insert(&pair.id) {
                return Err(ThemeError::new(
                    ThemeErrorCode::DuplicateIdentifier,
                    "Contrast pair IDs must be unique.",
                    pair.id.clone(),
                ));
            }
        }
        check_string(&self.provenance.source, "provenance.source")?;
        if let Some(value) = &self.provenance.generator {
            check_string(value, "provenance.generator")?;
        }
        if let Some(value) = &self.provenance.prompt {
            check_string(value, "provenance.prompt")?;
        }
        for (key, value) in &self.provenance.inputs {
            check_id(key, "provenance.inputs.key")?;
            check_string(value, "provenance.inputs.value")?;
        }
        // All references and declared assets must resolve in every variant after merge.
        for variant_id in self.variants.keys() {
            let layer = effective_layer(self, variant_id)?;
            let resolved = resolve_token_map(&layer.tokens)?;
            validate_value_references(&layer, &resolved, &asset_ids)?;
        }
        for variant_id in self.variants.keys() {
            let layer = effective_layer(self, variant_id)?;
            let resolved = resolve_token_map(&layer.tokens)?;
            for (id, parameter) in &self.parameters {
                match resolved.get(&parameter.target_token) {
                    Some(ThemeValue::Metric(metric)) if metric.unit == parameter.unit => {}
                    Some(ThemeValue::Metric(_)) => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::TypeMismatch,
                            "Parameter unit does not match its target metric token.",
                            format!("parameters.{id}@{variant_id}"),
                        ));
                    }
                    Some(_) => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::TypeMismatch,
                            "Metric parameter target must be a metric token.",
                            format!("parameters.{id}@{variant_id}"),
                        ));
                    }
                    None => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidReference,
                            "Parameter target token does not exist.",
                            format!("parameters.{id}@{variant_id}"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<String, ThemeError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            ThemeError::new(ThemeErrorCode::InvalidJson, error.to_string(), "manifest")
        })?;
        Ok(fnv1a64(&bytes))
    }
}

fn migrate_theme_manifest_value(
    mut value: serde_json::Value,
) -> Result<serde_json::Value, ThemeError> {
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::UnsupportedSchemaVersion,
                "Theme manifest has a missing or malformed schema version.",
                "schema_version",
            )
        })?;
    if version < THEME_SCHEMA_VERSION && theme_has_svg_bindings(&value) {
        return Err(ThemeError::new(
            ThemeErrorCode::UnsupportedSchemaVersion,
            "SVG style bindings require theme schema v3.",
            "components.svg_bindings",
        ));
    }
    match version {
        1 => {
            if value
                .get("assets")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|assets| {
                    assets.iter().any(|asset| {
                        asset
                            .get("format")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|format| format != "png")
                    })
                })
            {
                return Err(ThemeError::new(
                    ThemeErrorCode::UnsupportedSchemaVersion,
                    "Theme schema v1 supports PNG assets only; vector assets require schema v2 or later.",
                    "assets",
                ));
            }
            // V1 packages contain the same typed fields but only supported PNG assets.
            // The adapter preserves every serialized value and promotes the schema marker.
            value
                .as_object_mut()
                .expect("versioned theme manifest must be an object")
                .insert(
                    "schema_version".into(),
                    serde_json::Value::from(THEME_SCHEMA_VERSION),
                );
        }
        2 => {
            value
                .as_object_mut()
                .expect("versioned theme manifest must be an object")
                .insert(
                    "schema_version".into(),
                    serde_json::Value::from(THEME_SCHEMA_VERSION),
                );
        }
        THEME_SCHEMA_VERSION => {}
        _ => {
            return Err(ThemeError::new(
                ThemeErrorCode::UnsupportedSchemaVersion,
                format!(
                    "Theme schema version {version} is unsupported; expected a supported version up to {THEME_SCHEMA_VERSION}."
                ),
                "schema_version",
            ));
        }
    }
    Ok(value)
}

fn theme_has_svg_bindings(value: &serde_json::Value) -> bool {
    let components_have_bindings = |components: Option<&serde_json::Value>| {
        components
            .and_then(serde_json::Value::as_object)
            .into_iter()
            .flat_map(serde_json::Map::values)
            .filter_map(|component| component.get("states"))
            .filter_map(serde_json::Value::as_object)
            .flat_map(serde_json::Map::values)
            .any(|state| {
                state
                    .as_object()
                    .is_some_and(|state| state.contains_key("svg_bindings"))
            })
    };
    components_have_bindings(value.get("components"))
        || value
            .get("variants")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|variants| {
                variants
                    .values()
                    .any(|variant| components_have_bindings(variant.get("components")))
            })
}

#[derive(Clone)]
struct ThemeLayer {
    tokens: BTreeMap<String, ThemeValue>,
    components: BTreeMap<String, ThemeComponent>,
    token_sources: BTreeMap<String, String>,
}

/// Resolve one manifest deterministically. Missing/unverified decorative assets are omitted;
/// token values remain available so a host can render a token-only fallback.
pub fn resolve_theme(
    manifest: &ThemeManifest,
    variant_id: &str,
    parameter_overrides: &BTreeMap<String, f64>,
    capabilities: &ThemeAdapterCapabilities,
    asset_report: Option<&ThemeAssetReport>,
) -> Result<ResolvedTheme, ThemeError> {
    manifest.validate()?;
    let manifest_fingerprint = manifest.fingerprint()?;
    let mut layer = effective_layer(manifest, variant_id)?;
    let mut tokens = resolve_token_map(&layer.tokens)?;
    let mut applied_parameters = BTreeMap::new();
    for id in parameter_overrides.keys() {
        if !manifest.parameters.contains_key(id) {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidParameter,
                "Override references an undeclared parameter.",
                format!("parameters.{id}"),
            ));
        }
    }
    for (id, parameter) in &manifest.parameters {
        let value = *parameter_overrides.get(id).unwrap_or(&parameter.default);
        if !value.is_finite() || value < parameter.minimum || value > parameter.maximum {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidParameter,
                "Parameter value is outside its finite declared range.",
                format!("parameters.{id}"),
            ));
        }
        let steps = (value - parameter.minimum) / parameter.step;
        if (steps - steps.round()).abs() > 1e-8 {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidParameter,
                "Parameter value does not align to its declared step.",
                format!("parameters.{id}"),
            ));
        }
        let Some(ThemeValue::Metric(metric)) = tokens.get_mut(&parameter.target_token) else {
            return Err(ThemeError::new(
                ThemeErrorCode::TypeMismatch,
                "Parameter target is not a resolved metric token.",
                format!("parameters.{id}"),
            ));
        };
        metric.amount = value;
        layer
            .token_sources
            .insert(parameter.target_token.clone(), format!("parameter:{id}"));
        applied_parameters.insert(id.clone(), value);
    }
    let verified_assets = asset_report
        .filter(|report| report.manifest_id == manifest.id)
        .map(|report| &report.verified_asset_ids);
    let mut diagnostics = Vec::new();
    if let Some(report) = asset_report {
        if report.manifest_id != manifest.id {
            return Err(ThemeError::new(
                ThemeErrorCode::IncompatibleSnapshot,
                "Asset report belongs to a different theme manifest.",
                "assets",
            ));
        }
        diagnostics.extend(report.diagnostics.iter().cloned());
    }
    let mut resolved_assets = Vec::new();
    for asset in &manifest.assets {
        let supported = match asset.format {
            ThemeAssetFormat::Png => {
                capabilities.supports_raster_assets
                    && capabilities
                        .supported_asset_formats
                        .contains(&ThemeAssetFormatName::Png)
            }
            ThemeAssetFormat::Svg => capabilities
                .supported_asset_formats
                .contains(&ThemeAssetFormatName::Svg),
        };
        let verified = verified_assets
            .map(|ids| ids.contains(&asset.id))
            .unwrap_or(false);
        let enabled = supported && verified;
        if !supported {
            diagnostics.push(diagnostic(
                ThemeDiagnosticCode::UnsupportedAssetFormat,
                &format!("assets.{}", asset.id),
                "Host does not support this decorative asset format; token-only styling is used.",
            ));
        } else if !verified {
            diagnostics.push(diagnostic(ThemeDiagnosticCode::AssetUnverified, &format!("assets.{}", asset.id), "Asset was not verified from its package; host must omit it and use token-only styling."));
        }
        resolved_assets.push(ResolvedThemeAsset {
            id: asset.id.clone(),
            path: asset.path.clone(),
            format: asset.format,
            source_width: asset.source_width,
            source_height: asset.source_height,
            scale: asset.scale,
            transparency: asset.transparency,
            nine_slice: asset.nine_slice,
            enabled,
        });
    }
    let asset_ids: BTreeSet<_> = resolved_assets
        .iter()
        .filter(|asset| asset.enabled)
        .map(|asset| asset.id.as_str())
        .collect();
    let mut resolved_components = BTreeMap::new();
    let mut resolved_svg_bindings = BTreeMap::new();
    let mut value_sources = layer.token_sources;
    for (component_id, component) in &layer.components {
        let mut states = BTreeMap::new();
        let mut svg_states = BTreeMap::new();
        for (state_id, state) in &component.states {
            let unsupported: Vec<_> = state
                .requires
                .iter()
                .filter(|feature| match feature.strip_prefix("asset-") {
                    Some(asset_id) => !asset_ids.contains(asset_id),
                    None => !capabilities.supported_features.contains(*feature),
                })
                .cloned()
                .collect();
            let (values, source) = if unsupported.is_empty() {
                (
                    &state.tokens,
                    format!("component:{component_id}/{state_id}"),
                )
            } else if let Some(fallback) = &state.fallback {
                diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::FallbackApplied,
                    &format!("components.{component_id}.states.{state_id}"),
                    &format!(
                        "Unsupported features [{}]; declared fallback styles are active.",
                        unsupported.join(", ")
                    ),
                ));
                (fallback, format!("fallback:{component_id}/{state_id}"))
            } else {
                diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::UnsupportedFeature,
                    &format!("components.{component_id}.states.{state_id}"),
                    &format!(
                        "Unsupported features [{}]; this decorative state style is omitted.",
                        unsupported.join(", ")
                    ),
                ));
                continue;
            };
            let mut resolved_values = BTreeMap::new();
            for (property, value) in values {
                if let ThemeValue::Asset(asset_id) = value {
                    if !asset_ids.contains(asset_id.as_str()) {
                        diagnostics.push(diagnostic(ThemeDiagnosticCode::DecorativeAssetOmitted, &format!("components.{component_id}.states.{state_id}.{property}"), "Referenced decorative asset is unavailable; the host should render the token fallback."));
                        continue;
                    }
                    resolved_values.insert(property.clone(), value.clone());
                } else {
                    let resolved = resolve_value(value, &tokens)?;
                    resolved_values.insert(property.clone(), resolved);
                }
                value_sources.insert(
                    format!("component:{component_id}/{state_id}/{property}"),
                    source.clone(),
                );
            }
            let active_bindings = state
                .svg_bindings
                .iter()
                .filter(|binding| {
                    asset_ids.contains(binding.asset_id.as_str())
                        && resolved_values.contains_key(&binding.token)
                })
                .cloned()
                .collect();
            svg_states.insert(state_id.clone(), active_bindings);
            states.insert(state_id.clone(), resolved_values);
        }
        resolved_components.insert(component_id.clone(), states);
        resolved_svg_bindings.insert(component_id.clone(), svg_states);
    }
    let contrast = check_contrast_pairs_with_components(manifest, &tokens, &resolved_components)?;
    if let Some(failed) = contrast.iter().find(|result| !result.passed) {
        return Err(ThemeError::new(
            ThemeErrorCode::InsufficientContrast,
            format!(
                "Critical contrast pair measures {:.2}:1; target is {:.2}:1.",
                failed.measured_ratio, failed.required_ratio
            ),
            format!("contrast_pairs.{}", failed.pair_id),
        ));
    }
    diagnostics.sort_by(|a, b| {
        (&a.context, a.code as u8, &a.message).cmp(&(&b.context, b.code as u8, &b.message))
    });
    let mut resolved = ResolvedTheme {
        theme_id: manifest.id.clone(),
        variant_id: variant_id.to_owned(),
        manifest_fingerprint,
        tokens,
        components: resolved_components,
        svg_bindings: resolved_svg_bindings,
        parameters: applied_parameters,
        assets: resolved_assets,
        contrast,
        diagnostics,
        fingerprint: String::new(),
        value_sources,
    };
    let canonical = serde_json::to_vec(&resolved).map_err(|error| {
        ThemeError::new(
            ThemeErrorCode::InvalidJson,
            error.to_string(),
            "resolved_theme",
        )
    })?;
    resolved.fingerprint = fnv1a64(&canonical);
    Ok(resolved)
}

/// Validate bounded decorative PNG and SVG assets without executing them. Any failure is
/// reported per asset and leaves it out of `verified_asset_ids`; tokens remain available.
pub fn inspect_theme_assets(
    manifest: &ThemeManifest,
    package_root: &Path,
) -> Result<ThemeAssetReport, ThemeError> {
    manifest.validate()?;
    let mut report = ThemeAssetReport {
        manifest_id: manifest.id.clone(),
        ..ThemeAssetReport::default()
    };
    let root = match package_root.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            for asset in &manifest.assets {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    &format!("Theme package root cannot be opened: {error}"),
                ));
            }
            return Ok(report);
        }
    };
    for asset in &manifest.assets {
        let relative = match safe_asset_relative_path(&asset.path) {
            Ok(path) => path,
            Err(error) => {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    &error.message,
                ));
                continue;
            }
        };
        let path = root.join(relative);
        let canonical = match path.canonicalize() {
            Ok(path) if path.starts_with(&root) => path,
            Ok(_) => {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    "Asset symlink resolves outside the theme package.",
                ));
                continue;
            }
            Err(error) => {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    &format!("Decorative asset is missing or unreadable: {error}"),
                ));
                continue;
            }
        };
        let metadata = match canonical.metadata() {
            Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_ASSET_BYTES => metadata,
            Ok(_) => {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    "Asset must be a regular file no larger than 8 MiB.",
                ));
                continue;
            }
            Err(error) => {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    &format!("Cannot inspect decorative asset: {error}"),
                ));
                continue;
            }
        };
        let maximum_bytes = match asset.format {
            ThemeAssetFormat::Png => MAX_ASSET_BYTES,
            ThemeAssetFormat::Svg => MAX_SVG_ASSET_BYTES,
        };
        if metadata.len() > maximum_bytes {
            report.diagnostics.push(diagnostic(
                ThemeDiagnosticCode::AssetUnavailable,
                &format!("assets.{}", asset.id),
                "Asset exceeds the format-specific byte limit.",
            ));
            continue;
        }
        if asset.format == ThemeAssetFormat::Png && metadata.len() < 24 {
            report.diagnostics.push(diagnostic(
                ThemeDiagnosticCode::AssetUnavailable,
                &format!("assets.{}", asset.id),
                "PNG file is too short to contain its signature and IHDR dimensions.",
            ));
            continue;
        }
        let bytes = match File::open(&canonical).and_then(|mut file| {
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            file.read_to_end(&mut bytes).map(|_| bytes)
        }) {
            Ok(bytes) => bytes,
            Err(error) => {
                report.diagnostics.push(diagnostic(
                    ThemeDiagnosticCode::AssetUnavailable,
                    &format!("assets.{}", asset.id),
                    &format!("Cannot read decorative asset: {error}"),
                ));
                continue;
            }
        };
        let validation = match asset.format {
            ThemeAssetFormat::Png => validate_png(&bytes, asset),
            ThemeAssetFormat::Svg => validate_svg(&bytes),
        };
        if let Err(message) = validation {
            report.diagnostics.push(diagnostic(
                ThemeDiagnosticCode::AssetUnavailable,
                &format!("assets.{}", asset.id),
                message,
            ));
            continue;
        }
        report.verified_asset_ids.insert(asset.id.clone());
    }
    report
        .diagnostics
        .sort_by(|a, b| (&a.context, &a.message).cmp(&(&b.context, &b.message)));
    Ok(report)
}

fn validate_svg(bytes: &[u8]) -> Result<(), &'static str> {
    let source = std::str::from_utf8(bytes).map_err(|_| "SVG must be valid UTF-8 XML.")?;
    let lowered = source.to_ascii_lowercase();
    if lowered.contains("<!doctype") || lowered.contains("<!entity") {
        return Err("SVG document types and entity declarations are rejected.");
    }
    let document = roxmltree::Document::parse(source).map_err(|_| "SVG XML is malformed.")?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" {
        return Err("Asset document root must be an SVG element.");
    }
    let view_box = root
        .attribute("viewBox")
        .ok_or("SVG must declare a bounded viewBox.")?;
    let dimensions: Vec<_> = view_box
        .split_ascii_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<_, _>>()
        .map_err(|_| "SVG viewBox must contain four finite numbers.")?;
    if dimensions.len() != 4
        || dimensions.iter().any(|value| !value.is_finite())
        || dimensions[2] <= 0.0
        || dimensions[3] <= 0.0
        || dimensions[2] > f64::from(MAX_ASSET_DIMENSION)
        || dimensions[3] > f64::from(MAX_ASSET_DIMENSION)
    {
        return Err("SVG viewBox dimensions are outside the supported bounds.");
    }

    let mut nodes = 0usize;
    for node in document.descendants() {
        if node.is_element() {
            nodes += 1;
            if nodes > MAX_SVG_NODES {
                return Err("SVG element count exceeds the configured limit.");
            }
            let name = node.tag_name().name();
            if name.eq_ignore_ascii_case("script") || name.eq_ignore_ascii_case("foreignObject") {
                return Err("SVG scripts and foreignObject content are rejected.");
            }
            for attribute in node.attributes() {
                let attribute_name = attribute.name().to_ascii_lowercase();
                let value = attribute.value();
                if attribute_name.starts_with("on") {
                    return Err("SVG event-handler attributes are rejected.");
                }
                if attribute_name == "href" && !safe_svg_href(value) {
                    return Err("SVG external resource references are rejected.");
                }
                if !safe_svg_css_references(value) {
                    return Err("SVG external CSS imports and resource references are rejected.");
                }
            }
        } else if node.is_text()
            && node
                .parent_element()
                .is_some_and(|parent| parent.tag_name().name().eq_ignore_ascii_case("style"))
            && !safe_svg_css_references(node.text().unwrap_or_default())
        {
            return Err("SVG external CSS imports and resource references are rejected.");
        }
    }
    Ok(())
}

fn safe_svg_href(value: &str) -> bool {
    let value = value.trim();
    if value.starts_with('#') {
        return true;
    }
    let lowered = value.to_ascii_lowercase();
    [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/jpg;base64,",
        "data:image/gif;base64,",
        "data:image/webp;base64,",
    ]
    .iter()
    .any(|prefix| lowered.starts_with(prefix))
}

fn safe_svg_css_references(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("@import") {
        return false;
    }
    let mut remaining = lowered.as_str();
    while let Some(index) = remaining.find("url") {
        remaining = &remaining[index + 3..];
        let after_space = remaining.trim_start();
        if let Some(argument) = after_space.strip_prefix('(') {
            let Some(end) = argument.find(')') else {
                return false;
            };
            let target = argument[..end].trim().trim_matches(['"', '\'']);
            if !target.starts_with('#') {
                return false;
            }
            remaining = &argument[end + 1..];
        }
    }
    true
}

fn validate_png(bytes: &[u8], asset: &ThemeAsset) -> Result<(), &'static str> {
    const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.len() < 8 || bytes[..8] != SIGNATURE {
        return Err("Asset does not contain a PNG signature.");
    }
    let mut offset = 8_usize;
    let mut has_header = false;
    let mut has_image_data = false;
    let mut has_end = false;
    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err("PNG ends in a truncated chunk header or checksum.");
        }
        let length = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("PNG chunk length"),
        ) as usize;
        let chunk_end = offset
            .checked_add(12)
            .and_then(|base| base.checked_add(length))
            .ok_or("PNG chunk length overflows the file bounds.")?;
        if chunk_end > bytes.len() {
            return Err("PNG chunk extends beyond the end of the file.");
        }
        let kind = &bytes[offset + 4..offset + 8];
        let data = &bytes[offset + 8..offset + 8 + length];
        let expected_crc = u32::from_be_bytes(
            bytes[offset + 8 + length..chunk_end]
                .try_into()
                .expect("PNG chunk CRC"),
        );
        if png_crc32(&bytes[offset + 4..offset + 8 + length]) != expected_crc {
            return Err("PNG chunk checksum is invalid.");
        }
        if !has_header {
            if kind != b"IHDR" || length != 13 {
                return Err("PNG must start with a 13-byte IHDR chunk.");
            }
            let width = u32::from_be_bytes(data[0..4].try_into().expect("PNG width"));
            let height = u32::from_be_bytes(data[4..8].try_into().expect("PNG height"));
            if width == 0
                || height == 0
                || width != asset.source_width
                || height != asset.source_height
                || width > MAX_ASSET_DIMENSION
                || height > MAX_ASSET_DIMENSION
            {
                return Err("PNG dimensions do not match metadata or exceed the 8192-pixel limit.");
            }
            if data[10] != 0 || data[11] != 0 || data[12] > 1 {
                return Err("PNG uses an unsupported compression, filter, or interlace method.");
            }
            has_header = true;
        } else if kind == b"IHDR" {
            return Err("PNG contains more than one IHDR chunk.");
        }
        if kind == b"IDAT" {
            has_image_data = true;
        }
        offset = chunk_end;
        if kind == b"IEND" {
            if length != 0 || offset != bytes.len() {
                return Err("PNG IEND chunk is malformed or trailing bytes remain.");
            }
            has_end = true;
            break;
        }
    }
    if !has_header || !has_image_data || !has_end {
        return Err("PNG is missing a required IHDR, IDAT, or IEND chunk.");
    }
    Ok(())
}

fn png_crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Calculate every declared critical pair using WCAG 2.x sRGB relative luminance.
pub fn check_contrast_pairs(
    manifest: &ThemeManifest,
    tokens: &BTreeMap<String, ThemeValue>,
) -> Result<Vec<ThemeContrastResult>, ThemeError> {
    check_contrast_pairs_with_components(manifest, tokens, &BTreeMap::new())
}

pub fn check_contrast_pairs_with_components(
    manifest: &ThemeManifest,
    tokens: &BTreeMap<String, ThemeValue>,
    components: &BTreeMap<String, BTreeMap<String, BTreeMap<String, ThemeValue>>>,
) -> Result<Vec<ThemeContrastResult>, ThemeError> {
    let mut reports = Vec::with_capacity(manifest.contrast_pairs.len());
    for pair in &manifest.contrast_pairs {
        let (foreground, background) =
            if let (Some(component), Some(state)) = (&pair.component, &pair.state) {
                let styles = components
                    .get(component)
                    .and_then(|component| component.get(state))
                    .ok_or_else(|| {
                        ThemeError::new(
                            ThemeErrorCode::InvalidReference,
                            "Critical component state is missing from the resolved theme.",
                            format!("contrast_pairs.{}.component:{component}/{state}", pair.id),
                        )
                    })?;
                (styles.get(&pair.foreground), styles.get(&pair.background))
            } else {
                (tokens.get(&pair.foreground), tokens.get(&pair.background))
            };
        let foreground = foreground.ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Critical foreground value is missing from the resolved theme.",
                format!("contrast_pairs.{}.foreground", pair.id),
            )
        })?;
        let background = background.ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Critical background value is missing from the resolved theme.",
                format!("contrast_pairs.{}.background", pair.id),
            )
        })?;
        let foreground = color_channels(foreground, &pair.id)?;
        let background = color_channels(background, &pair.id)?;
        if background[3] < 0.999_999 {
            return Err(ThemeError::new(
                ThemeErrorCode::TypeMismatch,
                "Contrast background must be opaque.",
                format!("contrast_pairs.{}.background", pair.id),
            ));
        }
        let composited = [0, 1, 2].map(|index| {
            foreground[index] * foreground[3] + background[index] * (1.0 - foreground[3])
        });
        let ratio = contrast_ratio(
            luminance(composited),
            luminance([background[0], background[1], background[2]]),
        );
        let required = pair.target.minimum_ratio();
        reports.push(ThemeContrastResult {
            pair_id: pair.id.clone(),
            measured_ratio: ratio,
            required_ratio: required,
            passed: ratio + 1e-9 >= required,
        });
    }
    Ok(reports)
}

pub fn color_contrast_ratio(foreground: &str, background: &str) -> Result<f64, ThemeError> {
    let fg = parse_color(foreground).ok_or_else(|| {
        ThemeError::new(
            ThemeErrorCode::TypeMismatch,
            "Expected a #RRGGBB or #RRGGBBAA color.",
            "foreground",
        )
    })?;
    let bg = parse_color(background).ok_or_else(|| {
        ThemeError::new(
            ThemeErrorCode::TypeMismatch,
            "Expected a #RRGGBB or #RRGGBBAA color.",
            "background",
        )
    })?;
    if bg[3] < 0.999_999 {
        return Err(ThemeError::new(
            ThemeErrorCode::TypeMismatch,
            "Contrast background must be opaque.",
            "background",
        ));
    }
    let composited = [0, 1, 2].map(|index| fg[index] * fg[3] + bg[index] * (1.0 - fg[3]));
    Ok(contrast_ratio(
        luminance(composited),
        luminance([bg[0], bg[1], bg[2]]),
    ))
}

fn diagnostic(code: ThemeDiagnosticCode, context: &str, message: &str) -> ThemeDiagnostic {
    ThemeDiagnostic {
        code,
        severity: ThemeDiagnosticSeverity::Warning,
        message: message.into(),
        context: context.into(),
    }
}

fn validate_components(
    components: &BTreeMap<String, ThemeComponent>,
    context: &str,
) -> Result<(), ThemeError> {
    if components.len() > MAX_COMPONENTS {
        return Err(ThemeError::new(
            ThemeErrorCode::ResourceLimit,
            "Component count exceeds 128.",
            context,
        ));
    }
    for (component_id, component) in components {
        check_id(component_id, &format!("{context}.{component_id}"))?;
        if component.states.is_empty() || component.states.len() > MAX_STATES_PER_COMPONENT {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Component must declare 1 to 32 semantic states.",
                format!("{context}.{component_id}.states"),
            ));
        }
        for (state_id, state) in &component.states {
            check_id(state_id, &format!("{context}.{component_id}.states"))?;
            if state.tokens.len() > MAX_TOKENS
                || state
                    .fallback
                    .as_ref()
                    .is_some_and(|fallback| fallback.len() > MAX_TOKENS)
                || state.svg_bindings.len() > MAX_SVG_BINDINGS_PER_STATE
            {
                return Err(ThemeError::new(
                    ThemeErrorCode::ResourceLimit,
                    "State style exceeds the token count limit.",
                    format!("{context}.{component_id}.{state_id}"),
                ));
            }
            for feature in &state.requires {
                check_id(
                    feature,
                    &format!("{context}.{component_id}.{state_id}.requires"),
                )?;
            }
            for (property, value) in &state.tokens {
                check_id(property, "component state property")?;
                validate_value(
                    value,
                    &format!("{context}.{component_id}.{state_id}.{property}"),
                )?;
            }
            if let Some(fallback) = &state.fallback {
                for (property, value) in fallback {
                    check_id(property, "component fallback property")?;
                    validate_value(
                        value,
                        &format!("{context}.{component_id}.{state_id}.fallback.{property}"),
                    )?;
                }
            }
            for binding in &state.svg_bindings {
                check_id(&binding.asset_id, "svg_binding.asset_id")?;
                check_id(&binding.token, "svg_binding.token")?;
                if !valid_svg_selector(&binding.selector) {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidAsset,
                        "SVG style selector must be one bounded tag, class, or ID token.",
                        format!("{context}.{component_id}.{state_id}.svg_bindings.selector"),
                    ));
                }
                if !state.tokens.contains_key(&binding.token) {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidReference,
                        "SVG style binding token must name a token in its component state.",
                        format!("{context}.{component_id}.{state_id}.svg_bindings.token"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn valid_svg_selector(selector: &str) -> bool {
    let selector = selector.strip_prefix(['.', '#']).unwrap_or(selector);
    !selector.is_empty()
        && selector.len() <= 128
        && selector
            .bytes()
            .enumerate()
            .all(|(index, byte)| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'_' => true,
                b'0'..=b'9' => index > 0,
                b'-' => index > 0,
                _ => false,
            })
}

fn validate_svg_bindings(
    state: &ThemeComponentState,
    asset_formats: &BTreeMap<&str, ThemeAssetFormat>,
    base_tokens: &BTreeMap<String, ThemeValue>,
    context: &str,
) -> Result<(), ThemeError> {
    let resolved_tokens = resolve_token_map(base_tokens)?;
    for binding in &state.svg_bindings {
        if asset_formats.get(binding.asset_id.as_str()) != Some(&ThemeAssetFormat::Svg) {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "SVG style binding must reference a declared SVG asset.",
                format!("{context}.svg_bindings.{}", binding.asset_id),
            ));
        }
        let value = state.tokens.get(&binding.token).ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "SVG style binding token is missing from its component state.",
                format!("{context}.svg_bindings.{}", binding.token),
            )
        })?;
        let resolved = resolve_value(value, &resolved_tokens)?;
        if !svg_value_matches(binding.property, &resolved) {
            return Err(ThemeError::new(
                ThemeErrorCode::TypeMismatch,
                "SVG style property and bound token value have incompatible types.",
                format!("{context}.svg_bindings.{}", binding.token),
            ));
        }
    }
    Ok(())
}

fn svg_value_matches(property: ThemeSvgStyleProperty, value: &ThemeValue) -> bool {
    match property {
        ThemeSvgStyleProperty::Fill
        | ThemeSvgStyleProperty::Stroke
        | ThemeSvgStyleProperty::Color => {
            matches!(value, ThemeValue::Color(_))
        }
        ThemeSvgStyleProperty::Opacity => match value {
            ThemeValue::Number(number) => (0.0..=1.0).contains(number),
            ThemeValue::Metric(metric) => {
                metric.unit == ThemeMetricUnit::Unitless && (0.0..=1.0).contains(&metric.amount)
            }
            _ => false,
        },
        ThemeSvgStyleProperty::StrokeWidth => match value {
            ThemeValue::Number(number) => *number >= 0.0,
            ThemeValue::Metric(metric) => metric.amount >= 0.0,
            _ => false,
        },
    }
}

fn validate_value(value: &ThemeValue, context: &str) -> Result<(), ThemeError> {
    match value {
        ThemeValue::Color(value) => {
            if parse_color(value).is_none() {
                return Err(ThemeError::new(
                    ThemeErrorCode::TypeMismatch,
                    "Color must be #RRGGBB or #RRGGBBAA.",
                    context,
                ));
            }
        }
        ThemeValue::Metric(metric) => {
            if !metric.amount.is_finite() || metric.amount < 0.0 {
                return Err(ThemeError::new(
                    ThemeErrorCode::TypeMismatch,
                    "Metric must be finite and nonnegative.",
                    context,
                ));
            }
        }
        ThemeValue::Number(value) => {
            if !value.is_finite() {
                return Err(ThemeError::new(
                    ThemeErrorCode::TypeMismatch,
                    "Number must be finite.",
                    context,
                ));
            }
        }
        ThemeValue::Text(value) => check_string(value, context)?,
        ThemeValue::Reference(id) | ThemeValue::Asset(id) => check_id(id, context)?,
        ThemeValue::Boolean(_) => (),
    }
    Ok(())
}

fn validate_asset(asset: &ThemeAsset) -> Result<(), ThemeError> {
    check_id(&asset.id, "assets.id")?;
    safe_asset_relative_path(&asset.path)?;
    if asset.source_width == 0
        || asset.source_height == 0
        || asset.source_width > MAX_ASSET_DIMENSION
        || asset.source_height > MAX_ASSET_DIMENSION
        || !asset.scale.is_finite()
        || asset.scale <= 0.0
        || asset.scale > 8.0
    {
        return Err(ThemeError::new(
            ThemeErrorCode::InvalidAsset,
            "Asset dimensions or scale are outside the supported bounds.",
            format!("assets.{}", asset.id),
        ));
    }
    match (asset.transparency, &asset.color_key) {
        (ThemeTransparency::ColorKey, Some(key))
            if asset.format == ThemeAssetFormat::Png && parse_color(key).is_some() => {}
        (ThemeTransparency::ColorKey, _) => {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidAsset,
                "Color-key transparency requires a valid #RRGGBB color key.",
                format!("assets.{}", asset.id),
            ));
        }
        (_, None) => (),
        (_, Some(_)) => {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidAsset,
                "A color key is valid only for color-key transparency.",
                format!("assets.{}", asset.id),
            ));
        }
    }
    if let Some(slice) = asset.nine_slice
        && (slice.left.saturating_add(slice.right) > asset.source_width
            || slice.top.saturating_add(slice.bottom) > asset.source_height)
    {
        return Err(ThemeError::new(
            ThemeErrorCode::InvalidAsset,
            "9-slice insets exceed the source image dimensions.",
            format!("assets.{}.nine_slice", asset.id),
        ));
    }
    if let Some(provenance) = &asset.provenance {
        if let Some(tool) = &provenance.tool {
            check_string(tool, "asset.provenance.tool")?;
        }
        if let Some(prompt) = &provenance.prompt {
            check_string(prompt, "asset.provenance.prompt")?;
        }
        for (key, value) in &provenance.inputs {
            check_id(key, "asset.provenance.inputs.key")?;
            check_string(value, "asset.provenance.inputs.value")?;
        }
    }
    Ok(())
}

fn safe_asset_relative_path(value: &str) -> Result<PathBuf, ThemeError> {
    check_string(value, "asset.path")?;
    if value.is_empty() || value.contains('\\') || value.starts_with('/') || value.contains(':') {
        return Err(ThemeError::new(
            ThemeErrorCode::InvalidAsset,
            "Asset path must be a package-relative slash-separated path.",
            value,
        ));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ThemeError::new(
            ThemeErrorCode::InvalidAsset,
            "Absolute, traversal, and non-normal asset paths are rejected.",
            value,
        ));
    }
    Ok(path.to_path_buf())
}

fn validate_variant_chain(manifest: &ThemeManifest) -> Result<(), ThemeError> {
    for start in manifest.variants.keys() {
        let mut seen = BTreeSet::new();
        let mut current = start.as_str();
        for depth in 0..=MAX_VARIANT_DEPTH {
            if !seen.insert(current) {
                return Err(ThemeError::new(
                    ThemeErrorCode::InvalidInheritance,
                    "Variant inheritance contains a cycle.",
                    format!("variants.{start}"),
                ));
            }
            let variant = manifest.variants.get(current).ok_or_else(|| {
                ThemeError::new(
                    ThemeErrorCode::InvalidInheritance,
                    "Variant inherits from a missing variant.",
                    format!("variants.{start}.extends:{current}"),
                )
            })?;
            match variant.extends.as_deref() {
                None => break,
                Some(_parent) if depth == MAX_VARIANT_DEPTH => {
                    return Err(ThemeError::new(
                        ThemeErrorCode::ResourceLimit,
                        "Variant inheritance exceeds depth 8.",
                        format!("variants.{start}"),
                    ));
                }
                Some(parent) if !manifest.variants.contains_key(parent) => {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidInheritance,
                        "Variant inherits from a missing variant.",
                        format!("variants.{start}.extends:{parent}"),
                    ));
                }
                Some(parent) => current = parent,
            }
        }
    }
    Ok(())
}

fn effective_layer(manifest: &ThemeManifest, variant_id: &str) -> Result<ThemeLayer, ThemeError> {
    if !manifest.variants.contains_key(variant_id) {
        return Err(ThemeError::new(
            ThemeErrorCode::InvalidReference,
            "Selected theme variant does not exist.",
            format!("variants.{variant_id}"),
        ));
    }
    let mut chain = Vec::new();
    let mut current = variant_id;
    loop {
        let variant = manifest.variants.get(current).ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidInheritance,
                "Variant parent does not exist.",
                current,
            )
        })?;
        chain.push(current.to_owned());
        if let Some(parent) = variant.extends.as_deref() {
            current = parent;
        } else {
            break;
        }
        if chain.len() > MAX_VARIANT_DEPTH {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Variant inheritance exceeds depth 8.",
                variant_id,
            ));
        }
    }
    chain.reverse();
    let mut layer = ThemeLayer {
        tokens: manifest.tokens.clone(),
        components: manifest.components.clone(),
        token_sources: manifest
            .tokens
            .keys()
            .map(|id| (id.clone(), "base".into()))
            .collect(),
    };
    for id in chain {
        let previous = resolve_token_map(&layer.tokens)?;
        let variant = &manifest.variants[&id];
        for (token_id, value) in &variant.tokens {
            if let Some(existing) = layer.tokens.get(token_id) {
                ensure_compatible_type(
                    existing,
                    value,
                    &format!("variants.{id}.tokens.{token_id}"),
                )?;
            }
            layer.tokens.insert(token_id.clone(), value.clone());
            layer
                .token_sources
                .insert(token_id.clone(), format!("variant:{id}"));
        }
        merge_components(&mut layer.components, &variant.components);
        let current = resolve_token_map(&layer.tokens)?;
        for (token_id, old_value) in &previous {
            if let Some(new_value) = current.get(token_id)
                && value_kind(old_value) != value_kind(new_value)
            {
                return Err(ThemeError::new(
                    ThemeErrorCode::TypeMismatch,
                    "Variant resolution changes the type of an inherited token.",
                    format!("variants.{id}.tokens.{token_id}"),
                ));
            }
        }
    }
    Ok(layer)
}

fn merge_components(
    target: &mut BTreeMap<String, ThemeComponent>,
    overrides: &BTreeMap<String, ThemeComponent>,
) {
    for (component_id, component) in overrides {
        let base = target
            .entry(component_id.clone())
            .or_insert_with(|| ThemeComponent {
                states: BTreeMap::new(),
            });
        for (state_id, state) in &component.states {
            let existing =
                base.states
                    .entry(state_id.clone())
                    .or_insert_with(|| ThemeComponentState {
                        requires: Vec::new(),
                        tokens: BTreeMap::new(),
                        fallback: None,
                        svg_bindings: Vec::new(),
                    });
            existing.requires.extend(state.requires.iter().cloned());
            existing.requires.sort();
            existing.requires.dedup();
            existing.tokens.extend(state.tokens.clone());
            if state.fallback.is_some() {
                existing.fallback = state.fallback.clone();
            }
            existing.svg_bindings.extend(state.svg_bindings.clone());
        }
    }
}

fn ensure_compatible_type(
    existing: &ThemeValue,
    replacement: &ThemeValue,
    context: &str,
) -> Result<(), ThemeError> {
    let old = value_kind(existing);
    let new = value_kind(replacement);
    if old != "reference" && new != "reference" && old != new {
        return Err(ThemeError::new(
            ThemeErrorCode::TypeMismatch,
            "Variant override changes the declared token type.",
            context,
        ));
    }
    Ok(())
}

fn value_kind(value: &ThemeValue) -> &'static str {
    match value {
        ThemeValue::Color(_) => "color",
        ThemeValue::Metric(_) => "metric",
        ThemeValue::Number(_) => "number",
        ThemeValue::Boolean(_) => "boolean",
        ThemeValue::Text(_) => "text",
        ThemeValue::Reference(_) => "reference",
        ThemeValue::Asset(_) => "asset",
    }
}

fn resolve_token_map(
    tokens: &BTreeMap<String, ThemeValue>,
) -> Result<BTreeMap<String, ThemeValue>, ThemeError> {
    let mut output = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for id in tokens.keys() {
        resolve_token(id, tokens, &mut output, &mut visiting)?;
    }
    Ok(output)
}

fn resolve_token(
    id: &str,
    source: &BTreeMap<String, ThemeValue>,
    resolved: &mut BTreeMap<String, ThemeValue>,
    visiting: &mut BTreeSet<String>,
) -> Result<ThemeValue, ThemeError> {
    if let Some(value) = resolved.get(id) {
        return Ok(value.clone());
    }
    let value = source.get(id).ok_or_else(|| {
        ThemeError::new(
            ThemeErrorCode::InvalidReference,
            "Token reference points to a missing token.",
            id,
        )
    })?;
    if !visiting.insert(id.to_owned()) {
        return Err(ThemeError::new(
            ThemeErrorCode::ReferenceCycle,
            "Token references contain a cycle.",
            id,
        ));
    }
    let result = match value {
        ThemeValue::Reference(target) => resolve_token(target, source, resolved, visiting)?,
        ThemeValue::Metric(metric) if metric.amount.is_finite() && metric.amount >= 0.0 => {
            value.clone()
        }
        ThemeValue::Number(number) if number.is_finite() => value.clone(),
        ThemeValue::Color(color) if parse_color(color).is_some() => value.clone(),
        ThemeValue::Text(text) if text.len() <= MAX_STRING_LENGTH => value.clone(),
        ThemeValue::Asset(asset) => ThemeValue::Asset(asset.clone()),
        ThemeValue::Boolean(_) => value.clone(),
        _ => {
            return Err(ThemeError::new(
                ThemeErrorCode::TypeMismatch,
                "Token value is invalid after reference resolution.",
                id,
            ));
        }
    };
    visiting.remove(id);
    resolved.insert(id.to_owned(), result.clone());
    Ok(result)
}

fn resolve_value(
    value: &ThemeValue,
    tokens: &BTreeMap<String, ThemeValue>,
) -> Result<ThemeValue, ThemeError> {
    match value {
        ThemeValue::Reference(id) => tokens.get(id).cloned().ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Component style references a missing token.",
                id,
            )
        }),
        other => Ok(other.clone()),
    }
}

fn validate_value_references(
    layer: &ThemeLayer,
    tokens: &BTreeMap<String, ThemeValue>,
    assets: &BTreeSet<&String>,
) -> Result<(), ThemeError> {
    for (component, definition) in &layer.components {
        for (state, style) in &definition.states {
            for (property, value) in style
                .tokens
                .iter()
                .chain(style.fallback.iter().flat_map(|values| values.iter()))
            {
                match value {
                    ThemeValue::Reference(id) if !tokens.contains_key(id) => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidReference,
                            "Component style references an unknown semantic token.",
                            format!("components.{component}.{state}.{property}:{id}"),
                        ));
                    }
                    ThemeValue::Asset(id) if !assets.contains(id) => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidReference,
                            "Component style references an unknown decorative asset.",
                            format!("components.{component}.{state}.{property}:{id}"),
                        ));
                    }
                    _ => (),
                }
            }
        }
    }
    Ok(())
}

fn check_json_depth(json: &str) -> Result<(), ThemeError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in json.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_JSON_DEPTH {
                    return Err(ThemeError::new(
                        ThemeErrorCode::ResourceLimit,
                        "Theme JSON nesting exceeds 64 levels.",
                        "manifest",
                    ));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => (),
        }
    }
    Ok(())
}

fn check_id(value: &str, context: &str) -> Result<(), ThemeError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        || !value.as_bytes()[0].is_ascii_lowercase() && !value.as_bytes()[0].is_ascii_digit()
        || value.ends_with(['.', '_', '-'])
        || value.contains("..")
        || value.contains("--")
        || value.contains("__")
    {
        return Err(ThemeError::new(
            ThemeErrorCode::InvalidIdentifier,
            "Identifier must use lowercase ASCII letters, digits, and non-repeated dot, underscore, or hyphen separators.",
            context,
        ));
    }
    Ok(())
}

fn check_string(value: &str, context: &str) -> Result<(), ThemeError> {
    if value.len() > MAX_STRING_LENGTH {
        return Err(ThemeError::new(
            ThemeErrorCode::ResourceLimit,
            "String exceeds 4096 bytes.",
            context,
        ));
    }
    Ok(())
}

fn parse_color(value: &str) -> Option<[f64; 4]> {
    if !value.is_ascii() || !value.starts_with('#') || !matches!(value.len(), 7 | 9) {
        return None;
    }
    let byte = |index: usize| {
        u8::from_str_radix(&value[index..index + 2], 16)
            .ok()
            .map(|value| value as f64 / 255.0)
    };
    Some([
        byte(1)?,
        byte(3)?,
        byte(5)?,
        if value.len() == 9 { byte(7)? } else { 1.0 },
    ])
}

fn color_channels(value: &ThemeValue, context: &str) -> Result<[f64; 4], ThemeError> {
    match value {
        ThemeValue::Color(value) => parse_color(value).ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::TypeMismatch,
                "Contrast pair must reference color tokens.",
                context,
            )
        }),
        _ => Err(ThemeError::new(
            ThemeErrorCode::TypeMismatch,
            "Contrast pair must reference color tokens.",
            context,
        )),
    }
}

fn luminance(color: [f64; 3]) -> f64 {
    let linear = |channel: f64| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
}

fn contrast_ratio(first: f64, second: f64) -> f64 {
    let lighter = first.max(second);
    let darker = first.min(second);
    (lighter + 0.05) / (darker + 0.05)
}

fn fnv1a64(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(14695981039346656037_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(1099511628211)
    });
    format!("fnv1a64:{hash:016x}")
}

fn theme_value_checksum(value: &serde_json::Value) -> Result<String, ThemeError> {
    let canonical = serde_json::to_vec(value).map_err(|error| {
        ThemeError::new(
            ThemeErrorCode::InvalidJson,
            error.to_string(),
            "package_envelope.checksum",
        )
    })?;
    Ok(fnv1a64(&canonical))
}

static THEME_EDIT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeStylePackage {
    theme_id: String,
    source: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThemeStylePackagePayload {
    theme_id: String,
    source: String,
}

impl ThemeStylePackage {
    /// Create a bounded stylesheet package associated with one theme identity.
    /// The stylesheet is separate from ThemeManifest so older hosts can keep
    /// loading the manifest and use its token-only fallback unchanged.
    pub fn new(theme_id: impl Into<String>, source: impl Into<String>) -> Result<Self, ThemeError> {
        let package = Self {
            theme_id: theme_id.into(),
            source: source.into(),
        };
        package.validate()?;
        Ok(package)
    }

    pub fn theme_id(&self) -> &str {
        &self.theme_id
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Resolve rules against the already resolved token map for the selected
    /// theme variant. Identity mismatch fails closed so a host can retain its
    /// ordinary token-based chrome fallback.
    pub fn resolve(
        &self,
        theme_id: &str,
        context: &crate::ThemeStyleContext,
        resolved_tokens: &BTreeMap<String, ThemeValue>,
    ) -> Result<BTreeMap<crate::ThemeStyleProperty, ThemeValue>, ThemeError> {
        if self.theme_id != theme_id {
            return Err(ThemeError::new(
                ThemeErrorCode::IncompatibleSnapshot,
                "Theme stylesheet belongs to a different theme package.",
                format!("expected={theme_id};actual={}", self.theme_id),
            ));
        }
        crate::ThemeStyleSheet::parse(&self.source)?.resolve(context, resolved_tokens)
    }

    pub fn to_json(&self) -> Result<String, ThemeError> {
        self.validate()?;
        let mut envelope = serde_json::json!({
            "format_version": THEME_STYLE_PACKAGE_FORMAT_VERSION,
            "package": {
                "theme_id": self.theme_id,
                "source": self.source,
            }
        });
        let checksum = theme_value_checksum(&envelope)?;
        envelope
            .as_object_mut()
            .expect("theme style package envelope is an object")
            .insert("checksum".into(), serde_json::Value::String(checksum));
        let json = serde_json::to_string_pretty(&envelope).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                "theme_style_package",
            )
        })?;
        if json.len() > MAX_THEME_STYLE_PACKAGE_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme style package exceeds its 128 KiB limit.",
                "theme_style_package",
            ));
        }
        Ok(json)
    }

    pub fn parse_json(json: &str) -> Result<Self, ThemeError> {
        if json.len() > MAX_THEME_STYLE_PACKAGE_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme style package exceeds its 128 KiB limit.",
                "theme_style_package",
            ));
        }
        check_json_depth(json)?;
        let mut envelope: serde_json::Value = serde_json::from_str(json).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                "theme_style_package",
            )
        })?;
        let object = envelope.as_object_mut().ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                "Theme style package envelope must be an object.",
                "theme_style_package",
            )
        })?;
        if object.len() != 3 {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidJson,
                "Theme style package envelope has missing or unknown fields.",
                "theme_style_package",
            ));
        }
        let version = object
            .get("format_version")
            .and_then(serde_json::Value::as_u64);
        if version != Some(u64::from(THEME_STYLE_PACKAGE_FORMAT_VERSION)) {
            return Err(ThemeError::new(
                ThemeErrorCode::UnsupportedSchemaVersion,
                "Theme style package format version is unsupported.",
                format!("format_version={version:?}"),
            ));
        }
        let expected_checksum = object
            .get("checksum")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    "Theme style package checksum is missing.",
                    "theme_style_package.checksum",
                )
            })?
            .to_owned();
        object.remove("checksum");
        if theme_value_checksum(&envelope)? != expected_checksum {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidJson,
                "Theme style package checksum does not match its contents.",
                "theme_style_package.checksum",
            ));
        }
        let payload: ThemeStylePackagePayload =
            serde_json::from_value(envelope.get("package").cloned().ok_or_else(|| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    "Theme style package payload is missing.",
                    "theme_style_package.package",
                )
            })?)
            .map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    "theme_style_package.package",
                )
            })?;
        Self::new(payload.theme_id, payload.source)
    }

    /// Load one explicitly selected sidecar. This does not search arbitrary
    /// paths in a theme directory or alter the canonical theme manifest.
    pub fn load(path: &Path, expected_theme_id: &str) -> Result<Self, ThemeError> {
        let file = File::open(path).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                path.display().to_string(),
            )
        })?;
        let mut bytes = Vec::new();
        file.take((MAX_THEME_STYLE_PACKAGE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    path.display().to_string(),
                )
            })?;
        if bytes.len() > MAX_THEME_STYLE_PACKAGE_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme style package exceeds its 128 KiB limit.",
                path.display().to_string(),
            ));
        }
        let json = String::from_utf8(bytes).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                path.display().to_string(),
            )
        })?;
        let package = Self::parse_json(&json)?;
        package.require_theme(expected_theme_id)?;
        Ok(package)
    }

    /// Atomically save a sidecar, preserving its previous valid bytes as
    /// `.json.bak`. An unreadable, corrupt, future-version, or other-theme
    /// destination is left untouched for explicit recovery.
    pub fn save(&self, path: &Path) -> Result<(), ThemeError> {
        let json = self.to_json()?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let sequence = THEME_EDIT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".theme-style-{}-{sequence}.tmp",
            std::process::id()
        ));
        let backup = path.with_extension("json.bak");
        let backup_temp = parent.join(format!(
            ".theme-style-backup-{}-{sequence}.tmp",
            std::process::id()
        ));
        let result = (|| {
            for candidate in [path, backup.as_path()] {
                match fs::symlink_metadata(candidate) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidJson,
                            "Theme style save refuses symbolic-link package or backup paths.",
                            candidate.display().to_string(),
                        ));
                    }
                    Ok(metadata) if !metadata.is_file() => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidJson,
                            "Theme style save requires regular package and backup files.",
                            candidate.display().to_string(),
                        ));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(ThemeError::new(
                            ThemeErrorCode::InvalidJson,
                            error.to_string(),
                            candidate.display().to_string(),
                        ));
                    }
                }
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        error.to_string(),
                        temporary.display().to_string(),
                    )
                })?;
            file.write_all(json.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|error| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        error.to_string(),
                        temporary.display().to_string(),
                    )
                })?;
            if Self::load(&temporary, &self.theme_id)? != *self {
                return Err(ThemeError::new(
                    ThemeErrorCode::IncompatibleSnapshot,
                    "Staged theme style package differs from the validated source.",
                    temporary.display().to_string(),
                ));
            }
            if path.exists() {
                let current = Self::load(path, &self.theme_id)?;
                if current.theme_id != self.theme_id {
                    return Err(ThemeError::new(
                        ThemeErrorCode::IncompatibleSnapshot,
                        "Refusing to replace a style package owned by another theme.",
                        path.display().to_string(),
                    ));
                }
                let current_bytes = std::fs::read(path).map_err(|error| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        error.to_string(),
                        path.display().to_string(),
                    )
                })?;
                let mut backup_file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&backup_temp)
                    .map_err(|error| {
                        ThemeError::new(
                            ThemeErrorCode::InvalidJson,
                            error.to_string(),
                            backup_temp.display().to_string(),
                        )
                    })?;
                backup_file
                    .write_all(&current_bytes)
                    .and_then(|_| backup_file.sync_all())
                    .map_err(|error| {
                        ThemeError::new(
                            ThemeErrorCode::InvalidJson,
                            error.to_string(),
                            backup_temp.display().to_string(),
                        )
                    })?;
                if Self::load(&backup_temp, &self.theme_id)? != current {
                    return Err(ThemeError::new(
                        ThemeErrorCode::IncompatibleSnapshot,
                        "Staged theme style backup differs from the known-good package.",
                        backup_temp.display().to_string(),
                    ));
                }
                crate::persistence::atomic_replace(&backup_temp, &backup).map_err(|error| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        error.to_string(),
                        backup.display().to_string(),
                    )
                })?;
            }
            crate::persistence::atomic_replace(&temporary, path).map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    path.display().to_string(),
                )
            })
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
            let _ = std::fs::remove_file(&backup_temp);
        }
        result
    }

    fn validate(&self) -> Result<(), ThemeError> {
        check_id(&self.theme_id, "theme_style_package.theme_id")?;
        if self.source.len() > crate::theme_style::MAX_STYLESHEET_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme stylesheet exceeds its 64 KiB byte limit.",
                "theme_style_package.source",
            ));
        }
        crate::ThemeStyleSheet::parse(&self.source)?;
        Ok(())
    }

    fn require_theme(&self, expected_theme_id: &str) -> Result<(), ThemeError> {
        if self.theme_id != expected_theme_id {
            return Err(ThemeError::new(
                ThemeErrorCode::IncompatibleSnapshot,
                "Theme style package belongs to another theme.",
                format!("expected={expected_theme_id};actual={}", self.theme_id),
            ));
        }
        Ok(())
    }
}

fn preserve_known_good_theme_backup(path: &Path) -> Result<(), ThemeError> {
    if !path.exists() {
        return Ok(());
    }
    let bytes = std::fs::read(path).map_err(|error| {
        ThemeError::new(
            ThemeErrorCode::InvalidJson,
            format!("Could not read the existing theme before saving: {error}"),
            path.display().to_string(),
        )
    })?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Ok(());
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Ok(());
    };
    let Ok(manifest) = ThemeManifest::parse_json(text) else {
        // Do not replace a good recovery copy with a damaged or unvalidated
        // primary. The save may repair that primary while preserving backup.
        return Ok(());
    };
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let backup = path.with_extension("json.bak");
    let sequence = THEME_EDIT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".theme-backup-{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    temporary.display().to_string(),
                )
            })?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    temporary.display().to_string(),
                )
            })?;
        let staged = std::fs::read(&temporary).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                temporary.display().to_string(),
            )
        })?;
        let staged_manifest =
            ThemeManifest::parse_json(std::str::from_utf8(&staged).map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    temporary.display().to_string(),
                )
            })?)?;
        if staged_manifest != manifest {
            return Err(ThemeError::new(
                ThemeErrorCode::IncompatibleSnapshot,
                "Staged theme recovery copy differs from the validated package.",
                temporary.display().to_string(),
            ));
        }
        crate::persistence::atomic_replace(&temporary, &backup).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                backup.display().to_string(),
            )
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Validated in-memory authoring session. Edits validate a candidate copy first;
/// a rejected token or variant never damages the previous draft.
#[derive(Clone, Debug)]
pub struct ThemePackageEditor {
    saved: ThemeManifest,
    draft: ThemeManifest,
    selected_variant: String,
}
impl ThemePackageEditor {
    pub fn new(manifest: ThemeManifest) -> Result<Self, ThemeError> {
        manifest.validate()?;
        let selected_variant = manifest.variants.keys().next().cloned().unwrap_or_default();
        Ok(Self {
            saved: manifest.clone(),
            draft: manifest,
            selected_variant,
        })
    }
    pub fn import_json(json: &str) -> Result<Self, ThemeError> {
        Self::new(ThemeManifest::parse_json(json)?)
    }
    pub fn manifest(&self) -> &ThemeManifest {
        &self.draft
    }
    pub fn selected_variant(&self) -> &str {
        &self.selected_variant
    }
    pub fn select_variant(&mut self, variant: &str) -> Result<(), ThemeError> {
        if !self.draft.variants.contains_key(variant) {
            return Err(ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Theme variant was not found.",
                format!("variants.{variant}"),
            ));
        }
        self.selected_variant = variant.to_owned();
        Ok(())
    }
    pub fn set_base_token(&mut self, token: &str, value: ThemeValue) -> Result<(), ThemeError> {
        let mut candidate = self.draft.clone();
        candidate.tokens.insert(token.to_owned(), value);
        candidate.validate()?;
        self.draft = candidate;
        Ok(())
    }
    pub fn set_variant_token(
        &mut self,
        variant: &str,
        token: &str,
        value: ThemeValue,
    ) -> Result<(), ThemeError> {
        let mut candidate = self.draft.clone();
        let layer = candidate.variants.get_mut(variant).ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Theme variant was not found.",
                format!("variants.{variant}"),
            )
        })?;
        layer.tokens.insert(token.to_owned(), value);
        candidate.validate()?;
        self.draft = candidate;
        Ok(())
    }
    /// Replace SVG style bindings on one base component state atomically.
    pub fn set_component_state_svg_bindings(
        &mut self,
        component_id: &str,
        state_id: &str,
        bindings: Vec<ThemeSvgStyleBinding>,
    ) -> Result<(), ThemeError> {
        let mut candidate = self.draft.clone();
        let state = candidate
            .components
            .get_mut(component_id)
            .and_then(|component| component.states.get_mut(state_id))
            .ok_or_else(|| {
                ThemeError::new(
                    ThemeErrorCode::InvalidReference,
                    "Base theme component state was not found.",
                    format!("components.{component_id}.states.{state_id}"),
                )
            })?;
        state.svg_bindings = bindings;
        candidate.validate()?;
        self.draft = candidate;
        Ok(())
    }
    /// Replace SVG style bindings on a variant's component state atomically.
    pub fn set_variant_component_state_svg_bindings(
        &mut self,
        variant_id: &str,
        component_id: &str,
        state_id: &str,
        bindings: Vec<ThemeSvgStyleBinding>,
    ) -> Result<(), ThemeError> {
        let mut candidate = self.draft.clone();
        let variant = candidate.variants.get_mut(variant_id).ok_or_else(|| {
            ThemeError::new(
                ThemeErrorCode::InvalidReference,
                "Theme variant was not found.",
                format!("variants.{variant_id}"),
            )
        })?;
        let state = variant
            .components
            .get_mut(component_id)
            .and_then(|component| component.states.get_mut(state_id))
            .ok_or_else(|| {
                ThemeError::new(
                    ThemeErrorCode::InvalidReference,
                    "Variant theme component state was not found.",
                    format!("variants.{variant_id}.components.{component_id}.states.{state_id}"),
                )
            })?;
        state.svg_bindings = bindings;
        candidate.validate()?;
        self.draft = candidate;
        Ok(())
    }
    pub fn replace_draft(&mut self, manifest: ThemeManifest) -> Result<(), ThemeError> {
        manifest.validate()?;
        self.selected_variant = if manifest.variants.contains_key(&self.selected_variant) {
            self.selected_variant.clone()
        } else {
            manifest.variants.keys().next().cloned().unwrap_or_default()
        };
        self.draft = manifest;
        Ok(())
    }
    pub fn reset(&mut self) {
        self.draft = self.saved.clone();
        if !self.draft.variants.contains_key(&self.selected_variant) {
            self.selected_variant = self
                .draft
                .variants
                .keys()
                .next()
                .cloned()
                .unwrap_or_default();
        }
    }
    pub fn revert(&mut self) {
        self.reset();
    }
    pub fn dirty(&self) -> bool {
        self.draft != self.saved
    }
    pub fn export_json(&self) -> Result<String, ThemeError> {
        self.draft.to_json()
    }
    /// Save only to the caller-selected package path. A sibling temporary file
    /// is synced and reparsed before atomic replacement. Existing files survive
    /// validation or write failure unchanged.
    pub fn save(&mut self, path: &Path) -> Result<(), ThemeError> {
        let json = self.draft.to_json()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let seq = THEME_EDIT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".theme-edit-{}.{}.tmp", std::process::id(), seq));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        e.to_string(),
                        temp.display().to_string(),
                    )
                })?;
            file.write_all(json.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|e| {
                    ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        e.to_string(),
                        temp.display().to_string(),
                    )
                })?;
            let check = std::fs::read_to_string(&temp).map_err(|e| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    e.to_string(),
                    temp.display().to_string(),
                )
            })?;
            if ThemeManifest::parse_json(&check)? != self.draft {
                return Err(ThemeError::new(
                    ThemeErrorCode::IncompatibleSnapshot,
                    "Saved theme differs from validated draft.",
                    temp.display().to_string(),
                ));
            }
            preserve_known_good_theme_backup(path)?;
            crate::persistence::atomic_replace(&temp, path).map_err(|e| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result?;
        self.saved = self.draft.clone();
        Ok(())
    }
    fn read_source(path: &Path) -> Result<String, ThemeError> {
        let file = File::open(path).map_err(|e| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                e.to_string(),
                path.display().to_string(),
            )
        })?;
        let mut bytes = Vec::new();
        file.take((MAX_MANIFEST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme exceeds the 1 MiB manifest limit.",
                path.display().to_string(),
            ));
        }
        String::from_utf8(bytes).map_err(|e| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                e.to_string(),
                path.display().to_string(),
            )
        })
    }

    fn validate_upgrade_targets(path: &Path) -> Result<(), ThemeError> {
        let backup = path.with_extension("json.bak");
        for candidate in [path, backup.as_path()] {
            match fs::symlink_metadata(candidate) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        "Theme in-place upgrade refuses symbolic-link package or backup paths.",
                        candidate.display().to_string(),
                    ));
                }
                Ok(metadata) if !metadata.is_file() => {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        "Theme in-place upgrade requires regular package and backup files.",
                        candidate.display().to_string(),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(ThemeError::new(
                        ThemeErrorCode::InvalidJson,
                        format!("Could not inspect theme upgrade target: {error}"),
                        candidate.display().to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Load without changing the package on disk.
    pub fn load(path: &Path) -> Result<Self, ThemeError> {
        Self::import_json(&Self::read_source(path)?)
    }

    /// Load a package and upgrade a direct-manifest package or older supported
    /// theme schema to the current checksummed envelope. The old bytes are kept
    /// in the sibling `.json.bak`; asset files are not modified.
    pub fn load_and_upgrade(path: &Path) -> Result<(Self, bool), ThemeError> {
        Self::validate_upgrade_targets(path)?;
        let source = Self::read_source(path)?;
        let root: serde_json::Value = serde_json::from_str(&source).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                path.display().to_string(),
            )
        })?;
        let mut editor = Self::import_json(&source)?;
        let source_manifest = root.get("manifest").unwrap_or(&root);
        let source_schema = source_manifest
            .get("schema_version")
            .and_then(serde_json::Value::as_u64);
        let upgraded = root.get("manifest").is_none()
            || source_schema != Some(u64::from(THEME_SCHEMA_VERSION));
        if upgraded {
            editor.save(path)?;
        }
        Ok((editor, upgraded))
    }

    /// Restore the last valid package from its sibling `.json.bak` file.
    /// The backup is parsed and checksum-validated before the primary is
    /// atomically replaced; the backup itself is retained for further recovery.
    pub fn recover(path: &Path) -> Result<Self, ThemeError> {
        if fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_MANIFEST_BYTES as u64) {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme primary exceeds the 1 MiB manifest limit; backup recovery was not attempted.",
                path.display().to_string(),
            ));
        }
        if let Ok(bytes) = fs::read(path)
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        {
            let version_fields = [
                (value.get("format_version"), THEME_PACKAGE_FORMAT_VERSION),
                (
                    value
                        .get("manifest")
                        .and_then(|manifest| manifest.get("schema_version")),
                    THEME_SCHEMA_VERSION,
                ),
                (value.get("schema_version"), THEME_SCHEMA_VERSION),
            ];
            for (version, supported) in version_fields {
                if let Some(version) = version {
                    let Some(version) = version.as_u64() else {
                        return Err(ThemeError::new(
                            ThemeErrorCode::UnsupportedSchemaVersion,
                            "Theme primary has a malformed version field; backup recovery was not attempted.",
                            path.display().to_string(),
                        ));
                    };
                    if version > u64::from(supported) {
                        return Err(ThemeError::new(
                            ThemeErrorCode::UnsupportedSchemaVersion,
                            format!(
                                "Theme primary version {version} is newer than supported; backup recovery was not attempted."
                            ),
                            path.display().to_string(),
                        ));
                    }
                }
            }
        }
        let backup = path.with_extension("json.bak");
        let file = File::open(&backup).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                format!("Theme primary is invalid and its recovery backup is unavailable: {error}"),
                backup.display().to_string(),
            )
        })?;
        let mut bytes = Vec::new();
        file.take((MAX_MANIFEST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                ThemeError::new(
                    ThemeErrorCode::InvalidJson,
                    error.to_string(),
                    backup.display().to_string(),
                )
            })?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(ThemeError::new(
                ThemeErrorCode::ResourceLimit,
                "Theme recovery backup exceeds the 1 MiB manifest limit.",
                backup.display().to_string(),
            ));
        }
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            ThemeError::new(
                ThemeErrorCode::InvalidJson,
                error.to_string(),
                backup.display().to_string(),
            )
        })?;
        let mut editor = Self::import_json(text)?;
        editor.save(path)?;
        Ok(editor)
    }
}
