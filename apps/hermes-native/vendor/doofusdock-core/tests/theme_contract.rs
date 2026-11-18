use doofusdock_core::*;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const SAMPLE: &str = include_str!("../../../themes/doofusdock-workbench/theme.json");
const NOCTUA: &str = include_str!("../../../themes/doofusdock-noctua/theme.json");
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn sample() -> ThemeManifest {
    ThemeManifest::parse_json(SAMPLE).expect("sample theme must validate")
}

#[test]
fn package_editor_rejects_invalid_edits_without_mutation_and_round_trips_saved_packages() {
    let mut editor = ThemePackageEditor::import_json(SAMPLE).expect("editor imports valid package");
    assert!(!editor.dirty());
    editor.select_variant("high_contrast").unwrap();
    assert_eq!(editor.selected_variant(), "high_contrast");

    editor
        .set_base_token("effect.dither.enabled", ThemeValue::Boolean(true))
        .unwrap();
    assert!(editor.dirty());
    let valid_draft = editor.manifest().clone();
    assert_eq!(
        editor
            .set_base_token("color.surface.panel", ThemeValue::Number(7.0))
            .unwrap_err()
            .code,
        ThemeErrorCode::TypeMismatch
    );
    assert_eq!(editor.manifest(), &valid_draft, "rejected edit is atomic");
    assert!(editor.select_variant("missing").is_err());
    assert_eq!(editor.selected_variant(), "high_contrast");
    assert_eq!(
        ThemePackageEditor::import_json(&editor.export_json().unwrap())
            .unwrap()
            .manifest(),
        &valid_draft
    );

    let package = TempPackage::new();
    let path = package.0.join("authored-theme.json");
    fs::write(&path, b"previous package").unwrap();
    editor.save(&path).expect("save to explicit package path");
    assert!(!editor.dirty());
    let loaded = ThemePackageEditor::load(&path).expect("load saved package");
    assert_eq!(loaded.manifest(), &valid_draft);

    editor
        .set_base_token("effect.dither.enabled", ThemeValue::Boolean(false))
        .unwrap();
    assert!(editor.dirty());
    editor.revert();
    assert!(!editor.dirty());
    assert_eq!(editor.manifest(), &valid_draft);
}

#[test]
fn theme_package_recovery_restores_only_a_checksummed_known_good_manifest() {
    let package = TempPackage::new();
    let path = package.0.join("theme.json");
    let mut editor = ThemePackageEditor::import_json(SAMPLE).unwrap();
    editor
        .set_base_token("color.surface.panel", ThemeValue::Color("#FAFAFA".into()))
        .unwrap();
    let known_good = editor.manifest().clone();
    editor.save(&path).unwrap();

    editor
        .set_base_token("color.surface.panel", ThemeValue::Color("#EEEEEE".into()))
        .unwrap();
    editor.save(&path).unwrap();

    let backup = package.0.join("theme.json.bak");
    assert_eq!(
        ThemeManifest::parse_json(&fs::read_to_string(&backup).unwrap()).unwrap(),
        known_good
    );
    fs::write(&path, b"corrupt primary theme package").unwrap();

    let recovered = ThemePackageEditor::recover(&path).unwrap();
    assert_eq!(recovered.manifest(), &known_good);
    assert_eq!(
        ThemePackageEditor::load(&path).unwrap().manifest(),
        &known_good
    );
    assert_eq!(
        ThemeManifest::parse_json(&fs::read_to_string(&backup).unwrap()).unwrap(),
        known_good,
        "recovery retains its known-good source"
    );
}

#[test]
fn package_editor_failed_save_preserves_existing_target() {
    let package = TempPackage::new();
    let path = package.0.join("authored-theme.json");
    let original = b"keep this exact file on save failure";
    fs::write(&path, original).unwrap();
    let mut editor = ThemePackageEditor::new(sample()).unwrap();
    // A missing parent makes the atomic write fail after draft serialization.
    let missing = package.0.join("missing").join("theme.json");
    assert!(editor.save(&missing).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(&package.0).unwrap().count(), 1);
}

fn capabilities() -> ThemeAdapterCapabilities {
    ThemeAdapterCapabilities {
        supported_features: BTreeSet::from(["custom_chrome".into()]),
        supported_asset_formats: BTreeSet::from([ThemeAssetFormatName::Png]),
        supports_raster_assets: true,
    }
}

struct TempPackage(PathBuf);

impl TempPackage {
    fn new() -> Self {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("doofusdock-theme-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create temporary theme package");
        Self(path)
    }
}

impl Drop for TempPackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn crc32(bytes: &[u8]) -> u32 {
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

fn png_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    chunk.extend_from_slice(kind);
    chunk.extend_from_slice(payload);
    chunk.extend_from_slice(&crc32(&chunk[4..]).to_be_bytes());
    chunk
}

fn tiny_png() -> Vec<u8> {
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut header = Vec::from(1_u32.to_be_bytes());
    header.extend_from_slice(&1_u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    png.extend(png_chunk(b"IHDR", &header));
    png.extend(png_chunk(b"IDAT", &[1, 2, 3, 4]));
    png.extend(png_chunk(b"IEND", &[]));
    png
}

fn test_asset() -> ThemeAsset {
    ThemeAsset {
        id: "panel-frame".into(),
        path: "assets/frame.png".into(),
        format: ThemeAssetFormat::Png,
        source_width: 1,
        source_height: 1,
        scale: 1.0,
        transparency: ThemeTransparency::Alpha,
        color_key: None,
        nine_slice: Some(ThemeNineSlice {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }),
        edge_tiling: ThemeTiling::Stretch,
        center_tiling: ThemeTiling::Stretch,
        provenance: None,
    }
}

#[test]
fn token_only_sample_resolves_all_variants_deterministically_and_checks_contrast() {
    let manifest = sample();
    assert!(
        manifest.assets.is_empty(),
        "proof theme must work with all assets disabled"
    );
    let json = manifest.to_json().unwrap();
    let envelope: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope["format_version"], 1);
    assert!(
        envelope["checksum"]
            .as_str()
            .unwrap()
            .starts_with("fnv1a64:")
    );
    assert_eq!(ThemeManifest::parse_json(&json).unwrap(), manifest);
    let mut corrupted = envelope;
    corrupted["manifest"]["id"] = Value::String("modified-after-save".into());
    assert_eq!(
        ThemeManifest::parse_json(&corrupted.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::InvalidJson,
        "checksum mismatch must fail before a modified package is accepted"
    );
    for variant in ["dark", "light", "high_contrast"] {
        let first =
            resolve_theme(&manifest, variant, &BTreeMap::new(), &capabilities(), None).unwrap();
        let second =
            resolve_theme(&manifest, variant, &BTreeMap::new(), &capabilities(), None).unwrap();
        assert_eq!(first, second);
        assert!(first.contrast.iter().all(|pair| pair.passed), "{variant}");
        assert!(first.fingerprint.starts_with("fnv1a64:"));
        assert_eq!(first.theme_id, manifest.id);
    }
}

#[test]
fn noctua_tracker_package_resolves_all_variants_and_declares_complete_chrome_states() {
    let manifest = ThemeManifest::parse_json(NOCTUA).expect("Noctua package parses");
    manifest.validate().expect("Noctua package validates");
    assert_eq!(manifest.id, "doofusdock-noctua");
    assert!(
        manifest.assets.is_empty(),
        "native vector chrome has no image dependency"
    );
    for token in [
        "color.chrome.control",
        "color.chrome.control_hover",
        "color.chrome.control_pressed",
        "color.chrome.control_disabled",
        "color.chrome.focus_ring",
        "metric.border.width",
        "metric.bevel.width",
        "metric.spacing.dense",
        "metric.control.height",
        "metric.control.padding",
        "metric.font.chrome",
        "text.typography.chrome",
    ] {
        assert!(manifest.tokens.contains_key(token), "missing token {token}");
    }
    for (component, states) in [
        (
            "button",
            &["normal", "hover", "pressed", "focused", "disabled"][..],
        ),
        (
            "window_control",
            &["normal", "hover", "pressed", "focused", "disabled"][..],
        ),
        (
            "menu",
            &["normal", "hover", "focused", "selected", "disabled"][..],
        ),
        ("scrollbar", &["normal", "hover", "dragged", "disabled"][..]),
        (
            "tab",
            &[
                "normal", "hover", "pressed", "focused", "selected", "disabled",
            ][..],
        ),
    ] {
        let definition = manifest
            .components
            .get(component)
            .expect("required component exists");
        for state in states {
            assert!(
                definition.states.contains_key(*state),
                "missing {component}.{state}"
            );
        }
    }
    for variant in ["dark", "light", "high_contrast"] {
        let first =
            resolve_theme(&manifest, variant, &BTreeMap::new(), &capabilities(), None).unwrap();
        let second =
            resolve_theme(&manifest, variant, &BTreeMap::new(), &capabilities(), None).unwrap();
        assert_eq!(first, second, "{variant} must resolve deterministically");
        assert!(
            first.contrast.iter().all(|pair| pair.passed),
            "{variant} contrast"
        );
        assert!(
            first.assets.is_empty(),
            "{variant} has token-only fallbacks"
        );
    }
}

#[test]
fn inheritance_component_overrides_parameters_and_adapter_fallback_follow_declared_order() {
    let manifest = sample();
    let mut overrides = BTreeMap::new();
    overrides.insert("bevel_width".into(), 4.0);
    let resolved = resolve_theme(
        &manifest,
        "light",
        &overrides,
        &ThemeAdapterCapabilities::default(),
        None,
    )
    .unwrap();
    assert_eq!(
        resolved.tokens["color.surface.panel"],
        ThemeValue::Color("#FFFFFF".into())
    );
    assert_eq!(
        resolved.tokens["metric.bevel.width"],
        ThemeValue::Metric(ThemeMetric {
            amount: 4.0,
            unit: ThemeMetricUnit::LogicalPx
        })
    );
    assert_eq!(
        resolved.value_sources["metric.bevel.width"],
        "parameter:bevel_width"
    );
    assert_eq!(
        resolved.components["splitter"]["grip"]["foreground"],
        ThemeValue::Color("#56677A".into())
    );
    assert!(
        resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == ThemeDiagnosticCode::FallbackApplied)
    );
}

#[test]
fn schema_unknown_fields_version_limits_and_ids_are_rejected() {
    let mut value: Value = serde_json::from_str(SAMPLE).unwrap();
    value["unexpected"] = Value::Bool(true);
    assert_eq!(
        ThemeManifest::parse_json(&value.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::InvalidJson
    );
    value.as_object_mut().unwrap().remove("unexpected");
    value["schema_version"] = Value::from(2);
    assert_eq!(
        ThemeManifest::parse_json(&value.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::UnsupportedSchemaVersion
    );
    assert_eq!(
        ThemeManifest::parse_json(&format!(
            "{}{}{}",
            "{".repeat(66),
            "\"x\":0",
            "}".repeat(66)
        ))
        .unwrap_err()
        .code,
        ThemeErrorCode::ResourceLimit
    );
    assert_eq!(
        ThemeManifest::parse_json(&" ".repeat(1024 * 1024 + 1))
            .unwrap_err()
            .code,
        ThemeErrorCode::ResourceLimit
    );
    let mut manifest = sample();
    manifest
        .tokens
        .insert("Color.Bad".into(), ThemeValue::Boolean(true));
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidIdentifier
    );
}

#[test]
fn variant_cycles_missing_parents_and_type_changing_overrides_are_rejected() {
    let mut manifest = sample();
    manifest.variants.get_mut("dark").unwrap().extends = Some("light".into());
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidInheritance
    );

    let mut manifest = sample();
    manifest.variants.get_mut("light").unwrap().extends = Some("missing".into());
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidInheritance
    );

    let mut manifest = sample();
    manifest
        .variants
        .get_mut("light")
        .unwrap()
        .tokens
        .insert("color.surface.panel".into(), ThemeValue::Number(2.0));
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::TypeMismatch
    );
}

#[test]
fn token_references_reject_missing_ids_cycles_and_component_type_errors() {
    let mut manifest = sample();
    manifest.tokens.insert(
        "color.surface.panel".into(),
        ThemeValue::Reference("color.missing".into()),
    );
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidReference
    );

    let mut manifest = sample();
    manifest.tokens.insert(
        "color.surface.base".into(),
        ThemeValue::Reference("color.surface.panel".into()),
    );
    manifest.tokens.insert(
        "color.surface.panel".into(),
        ThemeValue::Reference("color.surface.base".into()),
    );
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::ReferenceCycle
    );

    let mut manifest = sample();
    manifest
        .components
        .get_mut("tab")
        .unwrap()
        .states
        .get_mut("normal")
        .unwrap()
        .tokens
        .insert(
            "unknown-slot".into(),
            ThemeValue::Reference("color.not-there".into()),
        );
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidReference
    );
}

#[test]
fn parameter_defaults_overrides_units_steps_and_ranges_are_enforced() {
    let manifest = sample();
    let defaults =
        resolve_theme(&manifest, "dark", &BTreeMap::new(), &capabilities(), None).unwrap();
    assert_eq!(defaults.parameters["bevel_width"], 2.0);
    let mut override_value = BTreeMap::from([("bevel_width".into(), 3.0)]);
    assert_eq!(
        resolve_theme(&manifest, "dark", &override_value, &capabilities(), None)
            .unwrap()
            .parameters["bevel_width"],
        3.0
    );
    override_value.insert("bevel_width".into(), 9.0);
    assert_eq!(
        resolve_theme(&manifest, "dark", &override_value, &capabilities(), None)
            .unwrap_err()
            .code,
        ThemeErrorCode::InvalidParameter
    );
    override_value.insert("bevel_width".into(), 2.5);
    assert_eq!(
        resolve_theme(&manifest, "dark", &override_value, &capabilities(), None)
            .unwrap_err()
            .code,
        ThemeErrorCode::InvalidParameter
    );
    override_value.clear();
    override_value.insert("not_declared".into(), 1.0);
    assert_eq!(
        resolve_theme(&manifest, "dark", &override_value, &capabilities(), None)
            .unwrap_err()
            .code,
        ThemeErrorCode::InvalidParameter
    );

    let mut manifest = sample();
    manifest.parameters.get_mut("bevel_width").unwrap().unit = ThemeMetricUnit::SourcePx;
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::TypeMismatch
    );
}

#[test]
fn wcag_contrast_known_values_alpha_and_critical_failure_are_reported() {
    assert!((color_contrast_ratio("#000000", "#FFFFFF").unwrap() - 21.0).abs() < 1e-9);
    assert!(
        (color_contrast_ratio("#777777", "#FFFFFF").unwrap() - 4.478_089_453_577_214).abs() < 1e-9
    );
    assert!(color_contrast_ratio("#000000", "#FFFFFF80").is_err());
    assert!(
        (color_contrast_ratio("#00000080", "#FFFFFF").unwrap() - 4.004_106_956_614_851_5).abs()
            < 1e-9
    );

    let mut manifest = sample();
    manifest.variants.get_mut("light").unwrap().tokens.insert(
        "color.text.primary".into(),
        ThemeValue::Color("#FFFFFF".into()),
    );
    let error =
        resolve_theme(&manifest, "light", &BTreeMap::new(), &capabilities(), None).unwrap_err();
    assert_eq!(error.code, ThemeErrorCode::InsufficientContrast);
    assert!(error.context.starts_with("contrast_pairs."));
    assert!(error.message.contains("target is"));

    let mut manifest = sample();
    manifest
        .components
        .get_mut("tab")
        .unwrap()
        .states
        .get_mut("selected")
        .unwrap()
        .tokens
        .insert("foreground".into(), ThemeValue::Color("#FFFFFF".into()));
    let error =
        resolve_theme(&manifest, "dark", &BTreeMap::new(), &capabilities(), None).unwrap_err();
    assert_eq!(error.code, ThemeErrorCode::InsufficientContrast);
    assert_eq!(error.context, "contrast_pairs.selected-tab-text");
}

#[test]
fn assets_validate_paths_slices_crc_dimensions_and_degrade_to_tokens() {
    let mut manifest = sample();
    let mut asset = test_asset();
    asset.path = "../secret.png".into();
    manifest.assets.push(asset.clone());
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidAsset
    );
    asset.path = "assets/frame.png".into();
    asset.nine_slice = Some(ThemeNineSlice {
        left: 1,
        top: 0,
        right: 1,
        bottom: 0,
    });
    manifest.assets = vec![asset.clone()];
    assert_eq!(
        manifest.validate().unwrap_err().code,
        ThemeErrorCode::InvalidAsset
    );

    asset.nine_slice = Some(ThemeNineSlice {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    });
    manifest.assets = vec![asset.clone()];
    let state = manifest
        .components
        .get_mut("splitter")
        .unwrap()
        .states
        .get_mut("grip")
        .unwrap();
    state.requires = vec!["asset-panel-frame".into()];
    state
        .tokens
        .insert("decoration".into(), ThemeValue::Asset("panel-frame".into()));
    state.fallback = Some(BTreeMap::from([(
        "decoration".into(),
        ThemeValue::Reference("color.surface.raised".into()),
    )]));
    manifest.validate().unwrap();

    let package = TempPackage::new();
    fs::create_dir_all(package.0.join("assets")).unwrap();
    let report = inspect_theme_assets(&manifest, &package.0).unwrap();
    let unavailable = resolve_theme(
        &manifest,
        "dark",
        &BTreeMap::new(),
        &capabilities(),
        Some(&report),
    )
    .unwrap();
    assert!(!unavailable.assets[0].enabled);
    assert_eq!(
        unavailable.components["splitter"]["grip"]["decoration"],
        ThemeValue::Color("#303947".into())
    );
    assert!(
        unavailable
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == ThemeDiagnosticCode::FallbackApplied)
    );

    fs::write(package.0.join("assets/frame.png"), tiny_png()).unwrap();
    let report = inspect_theme_assets(&manifest, &package.0).unwrap();
    assert!(report.is_verified("panel-frame"));
    let available = resolve_theme(
        &manifest,
        "dark",
        &BTreeMap::new(),
        &capabilities(),
        Some(&report),
    )
    .unwrap();
    assert!(available.assets[0].enabled);
    assert_eq!(
        available.components["splitter"]["grip"]["decoration"],
        ThemeValue::Asset("panel-frame".into())
    );

    let mut corrupt = tiny_png();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    fs::write(package.0.join("assets/frame.png"), corrupt).unwrap();
    let report = inspect_theme_assets(&manifest, &package.0).unwrap();
    assert!(!report.is_verified("panel-frame"));
}

#[test]
fn adapter_omits_unsupported_states_and_assets_with_typed_fallback_diagnostics() {
    let manifest = sample();
    let resolved = resolve_theme(
        &manifest,
        "dark",
        &BTreeMap::new(),
        &ThemeAdapterCapabilities::default(),
        None,
    )
    .unwrap();
    assert_eq!(
        resolved.components["splitter"]["grip"]["foreground"],
        ThemeValue::Color("#667386".into())
    );
    assert!(
        resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == ThemeDiagnosticCode::FallbackApplied)
    );
}

#[test]
fn theme_urne_projection_is_deterministic_bounded_and_has_no_dangling_edges_or_binary_assets() {
    let manifest = sample();
    let resolved =
        resolve_theme(&manifest, "dark", &BTreeMap::new(), &capabilities(), None).unwrap();
    let first = UrneProjection::from_theme(&manifest, &resolved).unwrap();
    let second = UrneProjection::from_theme(&manifest, &resolved).unwrap();
    assert_eq!(first, second);
    assert!(first.nodes.len() < 4096);
    let ids: BTreeSet<_> = first.nodes.iter().map(|node| node.id.as_str()).collect();
    for edge in &first.edges {
        assert!(ids.contains(edge.source.as_str()));
        assert!(ids.contains(edge.target.as_str()));
    }
    let types: BTreeSet<_> = first
        .nodes
        .iter()
        .map(|node| node.node_type.as_str())
        .collect();
    for expected in [
        "theme_package",
        "theme_variant",
        "theme_token",
        "theme_component",
        "theme_state",
        "theme_parameter",
        "theme_validation",
    ] {
        assert!(
            types.contains(expected),
            "missing URNE node type {expected}"
        );
    }
    let json = serde_json::to_vec(&first).unwrap();
    assert!(
        !json
            .windows(8)
            .any(|window| window == [137, 80, 78, 71, 13, 10, 26, 10])
    );
    assert!(first.sources.contains_key("doofusdock.theme.package"));

    let mut mismatched = resolved;
    mismatched.variant_id = "light".into();
    assert_eq!(
        UrneProjection::from_theme(&manifest, &mismatched)
            .unwrap_err()
            .code,
        ThemeErrorCode::IncompatibleSnapshot
    );
}
