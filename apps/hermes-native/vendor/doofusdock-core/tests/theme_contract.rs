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
fn loading_legacy_theme_upgrades_with_backup_and_preserves_assets() {
    let package = TempPackage::new();
    let path = package.0.join("theme.json");
    let asset_directory = package.0.join("assets");
    fs::create_dir_all(&asset_directory).unwrap();
    let asset_path = asset_directory.join("frame.png");
    let asset_bytes = b"theme asset bytes must remain untouched";
    fs::write(&asset_path, asset_bytes).unwrap();

    let mut manifest = sample();
    manifest.assets.push(ThemeAsset {
        id: "frame".into(),
        path: "assets/frame.png".into(),
        format: ThemeAssetFormat::Png,
        source_width: 1,
        source_height: 1,
        scale: 1.0,
        transparency: ThemeTransparency::Alpha,
        color_key: None,
        nine_slice: None,
        edge_tiling: ThemeTiling::Stretch,
        center_tiling: ThemeTiling::Stretch,
        provenance: None,
    });
    manifest.validate().unwrap();
    let legacy_source = serde_json::to_vec(&manifest).unwrap();
    fs::write(&path, &legacy_source).unwrap();

    let read_only = ThemePackageEditor::load(&path).unwrap();
    assert_eq!(read_only.manifest(), &manifest);
    assert_eq!(fs::read(&path).unwrap(), legacy_source);

    let (upgraded, migrated) = ThemePackageEditor::load_and_upgrade(&path).unwrap();
    assert!(migrated);
    assert_eq!(upgraded.manifest(), &manifest);
    let current_bytes = fs::read(&path).unwrap();
    let current_value: Value = serde_json::from_slice(&current_bytes).unwrap();
    assert_eq!(current_value["format_version"], 1);
    assert_eq!(
        ThemeManifest::parse_json(std::str::from_utf8(&current_bytes).unwrap()).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        legacy_source
    );
    assert_eq!(fs::read(&asset_path).unwrap(), asset_bytes);

    let (reloaded, migrated_again) = ThemePackageEditor::load_and_upgrade(&path).unwrap();
    assert!(!migrated_again);
    assert_eq!(reloaded.manifest(), &manifest);
    assert_eq!(fs::read(&path).unwrap(), current_bytes);
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        legacy_source
    );
    assert_eq!(fs::read(&asset_path).unwrap(), asset_bytes);

    let future_path = package.0.join("future-theme.json");
    let mut future = manifest.clone();
    future.schema_version += 1;
    let future_bytes = serde_json::to_vec(&future).unwrap();
    fs::write(&future_path, &future_bytes).unwrap();
    assert!(ThemePackageEditor::load_and_upgrade(&future_path).is_err());
    assert_eq!(fs::read(&future_path).unwrap(), future_bytes);
    assert!(!future_path.with_extension("json.bak").exists());

    let corrupt_path = package.0.join("corrupt-theme.json");
    let mut corrupt: Value = serde_json::from_slice(&current_bytes).unwrap();
    corrupt["checksum"] = Value::String("fnv1a64:0000000000000000".into());
    let corrupt_bytes = serde_json::to_vec(&corrupt).unwrap();
    fs::write(&corrupt_path, &corrupt_bytes).unwrap();
    assert!(ThemePackageEditor::load_and_upgrade(&corrupt_path).is_err());
    assert_eq!(fs::read(&corrupt_path).unwrap(), corrupt_bytes);
    assert!(!corrupt_path.with_extension("json.bak").exists());

    fs::write(&path, b"corrupt primary after upgrade").unwrap();
    let recovered = ThemePackageEditor::recover(&path).unwrap();
    assert_eq!(recovered.manifest(), &manifest);
    assert_eq!(
        ThemePackageEditor::load(&path).unwrap().manifest(),
        &manifest
    );
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        legacy_source
    );
    assert_eq!(fs::read(&asset_path).unwrap(), asset_bytes);
}

#[test]
fn theme_schema_v1_migrates_read_only_then_upgrades_atomically_with_exact_backup() {
    let package = TempPackage::new();
    let path = package.0.join("noctua.json");
    let legacy_bytes = NOCTUA.as_bytes().to_vec();
    fs::write(&path, &legacy_bytes).unwrap();

    let loaded = ThemePackageEditor::load(&path).unwrap();
    assert_eq!(loaded.manifest().schema_version, THEME_SCHEMA_VERSION);
    assert_eq!(loaded.manifest().id, "doofusdock-noctua");
    assert_eq!(fs::read(&path).unwrap(), legacy_bytes);

    let (upgraded, migrated) = ThemePackageEditor::load_and_upgrade(&path).unwrap();
    assert!(migrated);
    assert_eq!(upgraded.manifest().schema_version, THEME_SCHEMA_VERSION);
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        legacy_bytes
    );
    let current_bytes = fs::read(&path).unwrap();
    let envelope: Value = serde_json::from_slice(&current_bytes).unwrap();
    assert_eq!(envelope["format_version"], 1);
    assert_eq!(envelope["manifest"]["schema_version"], THEME_SCHEMA_VERSION);

    let (reloaded, migrated_again) = ThemePackageEditor::load_and_upgrade(&path).unwrap();
    assert!(!migrated_again);
    assert_eq!(reloaded.manifest(), upgraded.manifest());
    assert_eq!(fs::read(&path).unwrap(), current_bytes);
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        legacy_bytes
    );

    let envelope_path = package.0.join("checksummed-v1.json");
    let legacy_manifest: Value = serde_json::from_str(NOCTUA).unwrap();
    let mut legacy_envelope = serde_json::json!({
        "format_version": 1,
        "manifest": legacy_manifest,
    });
    let legacy_checksum = checksum_layout_json(&legacy_envelope.to_string()).unwrap();
    legacy_envelope["checksum"] = Value::String(legacy_checksum);
    let legacy_envelope_bytes = serde_json::to_vec_pretty(&legacy_envelope).unwrap();
    fs::write(&envelope_path, &legacy_envelope_bytes).unwrap();

    let (envelope_upgrade, envelope_migrated) =
        ThemePackageEditor::load_and_upgrade(&envelope_path).unwrap();
    assert!(envelope_migrated);
    assert_eq!(
        envelope_upgrade.manifest().schema_version,
        THEME_SCHEMA_VERSION
    );
    assert_eq!(
        fs::read(envelope_path.with_extension("json.bak")).unwrap(),
        legacy_envelope_bytes
    );
    let envelope_current: Value =
        serde_json::from_slice(&fs::read(&envelope_path).unwrap()).unwrap();
    assert_eq!(
        envelope_current["manifest"]["schema_version"],
        THEME_SCHEMA_VERSION
    );
}

#[test]
fn in_place_theme_upgrade_rejects_symlink_and_non_file_targets() {
    let package = TempPackage::new();
    let source = serde_json::to_vec(&sample()).unwrap();

    let backup_directory_path = package.0.join("backup-directory.json");
    fs::write(&backup_directory_path, &source).unwrap();
    fs::create_dir(backup_directory_path.with_extension("json.bak")).unwrap();
    let error = ThemePackageEditor::load_and_upgrade(&backup_directory_path).unwrap_err();
    assert_eq!(error.code, ThemeErrorCode::InvalidJson);
    assert_eq!(fs::read(&backup_directory_path).unwrap(), source);

    let target = package.0.join("theme-target.json");
    fs::write(&target, &source).unwrap();
    let link = package.0.join("theme-link.json");
    #[cfg(windows)]
    let symlink_result = std::os::windows::fs::symlink_file(&target, &link);
    #[cfg(unix)]
    let symlink_result = std::os::unix::fs::symlink(&target, &link);
    if symlink_result.is_ok() {
        let error = ThemePackageEditor::load_and_upgrade(&link).unwrap_err();
        assert_eq!(error.code, ThemeErrorCode::InvalidJson);
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), source);
    }
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
fn future_theme_schema_does_not_fall_back_to_an_older_backup() {
    let package = TempPackage::new();
    let path = package.0.join("theme.json");
    let mut editor = ThemePackageEditor::import_json(SAMPLE).unwrap();
    editor.save(&path).unwrap();
    editor
        .set_base_token("color.surface.panel", ThemeValue::Color("#EEEEEE".into()))
        .unwrap();
    editor.save(&path).unwrap();

    let backup = package.0.join("theme.json.bak");
    let backup_bytes = fs::read(&backup).unwrap();
    let mut future = editor.manifest().clone();
    future.schema_version = THEME_SCHEMA_VERSION + 1;
    let future_bytes = serde_json::to_vec(&future).unwrap();
    fs::write(&path, &future_bytes).unwrap();

    let error = ThemePackageEditor::recover(&path).unwrap_err();
    assert_eq!(error.code, ThemeErrorCode::UnsupportedSchemaVersion);
    assert!(error.message.contains("newer than supported"));
    assert_eq!(fs::read(&path).unwrap(), future_bytes);
    assert_eq!(fs::read(&backup).unwrap(), backup_bytes);
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

#[test]
fn stylesheet_package_round_trips_resolves_and_preserves_previous_valid_source() {
    let first = ThemeStylePackage::new(
        "doofusdock-workbench",
        "button { color: var(--text-primary); padding: 8px; }",
    )
    .unwrap();
    let json = first.to_json().unwrap();
    assert_eq!(ThemeStylePackage::parse_json(&json).unwrap(), first);

    let context = ThemeStyleContext::new("button").unwrap();
    let tokens = BTreeMap::from([(
        "text-primary".to_owned(),
        ThemeValue::Color("#f0f0f0".to_owned()),
    )]);
    let resolved = first
        .resolve("doofusdock-workbench", &context, &tokens)
        .unwrap();
    assert_eq!(
        resolved[&ThemeStyleProperty::Color],
        ThemeValue::Color("#f0f0f0".to_owned())
    );
    assert_eq!(
        first
            .resolve("another-theme", &context, &tokens)
            .unwrap_err()
            .code,
        ThemeErrorCode::IncompatibleSnapshot
    );

    let package = TempPackage::new();
    let path = package.0.join("theme-style.json");
    first.save(&path).unwrap();
    assert_eq!(
        ThemeStylePackage::load(&path, "doofusdock-workbench").unwrap(),
        first
    );

    let second = ThemeStylePackage::new(
        "doofusdock-workbench",
        "button { color: #aabbcc; padding: 10px; }",
    )
    .unwrap();
    second.save(&path).unwrap();
    assert_eq!(
        ThemeStylePackage::load(&path, "doofusdock-workbench").unwrap(),
        second
    );
    assert_eq!(
        ThemeStylePackage::load(&path.with_extension("json.bak"), "doofusdock-workbench").unwrap(),
        first
    );
    assert_eq!(
        ThemeStylePackage::load(&path, "another-theme")
            .unwrap_err()
            .code,
        ThemeErrorCode::IncompatibleSnapshot
    );
}

#[test]
fn stylesheet_package_rejects_tampering_future_formats_and_corrupt_replacement_targets() {
    let package =
        ThemeStylePackage::new("doofusdock-workbench", "button { opacity: 0.5; }").unwrap();
    let mut tampered: Value = serde_json::from_str(&package.to_json().unwrap()).unwrap();
    tampered["package"]["source"] = Value::String("button { opacity: 1; }".into());
    assert_eq!(
        ThemeStylePackage::parse_json(&tampered.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::InvalidJson
    );
    tampered["format_version"] = Value::from(99);
    assert_eq!(
        ThemeStylePackage::parse_json(&tampered.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::UnsupportedSchemaVersion
    );
    assert!(ThemeStylePackage::new("doofusdock-workbench", "button { color: red; }").is_err());

    let directory = TempPackage::new();
    let path = directory.0.join("theme-style.json");
    fs::write(&path, b"corrupt existing sidecar").unwrap();
    assert!(package.save(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"corrupt existing sidecar");
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let marker = path.join("preserve.txt");
    fs::write(&marker, b"directory target must remain untouched").unwrap();
    assert!(package.save(&path).is_err());
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"directory target must remain untouched"
    );
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
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
    value["schema_version"] = Value::from(THEME_SCHEMA_VERSION + 1);
    assert_eq!(
        ThemeManifest::parse_json(&value.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::UnsupportedSchemaVersion
    );
    let mut mislabeled_v1_svg: Value = serde_json::from_str(SAMPLE).unwrap();
    mislabeled_v1_svg["assets"] = serde_json::json!([{ "format": "svg" }]);
    assert_eq!(
        ThemeManifest::parse_json(&mislabeled_v1_svg.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::UnsupportedSchemaVersion,
        "a schema-v1 manifest cannot smuggle in a schema-v2 asset kind"
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
fn svg_theme_assets_are_bounded_verified_negotiated_and_fail_closed() {
    const VECTOR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 16"><defs><linearGradient id="g"><stop stop-color="#fff"/></linearGradient></defs><rect width="32" height="16" fill="url(#g)"/></svg>"##;
    let package = TempPackage::new();
    fs::create_dir_all(package.0.join("assets")).unwrap();
    let asset_path = package.0.join("assets/vector.svg");
    fs::write(&asset_path, VECTOR).unwrap();

    let mut manifest = sample();
    manifest.assets = vec![ThemeAsset {
        id: "panel-vector".into(),
        path: "assets/vector.svg".into(),
        format: ThemeAssetFormat::Svg,
        source_width: 32,
        source_height: 16,
        scale: 1.0,
        transparency: ThemeTransparency::Alpha,
        color_key: None,
        nine_slice: None,
        edge_tiling: ThemeTiling::Stretch,
        center_tiling: ThemeTiling::Stretch,
        provenance: None,
    }];
    let state = manifest
        .components
        .get_mut("splitter")
        .unwrap()
        .states
        .get_mut("grip")
        .unwrap();
    state.requires = vec!["asset-panel-vector".into()];
    state.tokens.insert(
        "decoration".into(),
        ThemeValue::Asset("panel-vector".into()),
    );
    state.fallback = Some(BTreeMap::from([(
        "decoration".into(),
        ThemeValue::Reference("color.surface.raised".into()),
    )]));
    state.tokens.insert(
        "svg.fill".into(),
        ThemeValue::Reference("color.surface.raised".into()),
    );
    state.svg_bindings.push(ThemeSvgStyleBinding {
        asset_id: "panel-vector".into(),
        selector: ".grip-fill".into(),
        property: ThemeSvgStyleProperty::Fill,
        token: "svg.fill".into(),
    });
    state
        .tokens
        .insert("svg.stroke_width".into(), ThemeValue::Number(2.0));
    state.svg_bindings.push(ThemeSvgStyleBinding {
        asset_id: "panel-vector".into(),
        selector: ".grip-fill".into(),
        property: ThemeSvgStyleProperty::StrokeWidth,
        token: "svg.stroke_width".into(),
    });
    manifest.validate().unwrap();
    let mut editor = ThemePackageEditor::new(manifest.clone()).unwrap();
    let editor_before = editor.manifest().clone();
    let mut invalid_bindings = editor_before.components["splitter"].states["grip"]
        .svg_bindings
        .clone();
    invalid_bindings[0].selector = ".safe, script".into();
    assert!(
        editor
            .set_component_state_svg_bindings("splitter", "grip", invalid_bindings)
            .is_err()
    );
    assert_eq!(editor.manifest(), &editor_before);
    let changed_bindings = editor_before.components["splitter"].states["grip"]
        .svg_bindings
        .iter()
        .take(1)
        .cloned()
        .collect();
    editor
        .set_component_state_svg_bindings("splitter", "grip", changed_bindings)
        .unwrap();
    assert!(editor.dirty());

    let mut variant_draft = manifest.clone();
    variant_draft
        .variants
        .get_mut("dark")
        .unwrap()
        .components
        .insert(
            "splitter".into(),
            ThemeComponent {
                states: BTreeMap::from([(
                    "grip".into(),
                    ThemeComponentState {
                        requires: Vec::new(),
                        tokens: BTreeMap::from([(
                            "svg.fill".into(),
                            ThemeValue::Reference("color.accent".into()),
                        )]),
                        fallback: None,
                        svg_bindings: Vec::new(),
                    },
                )]),
            },
        );
    let mut variant_editor = ThemePackageEditor::new(variant_draft).unwrap();
    let variant_binding = ThemeSvgStyleBinding {
        asset_id: "panel-vector".into(),
        selector: "circle".into(),
        property: ThemeSvgStyleProperty::Fill,
        token: "svg.fill".into(),
    };
    variant_editor
        .set_variant_component_state_svg_bindings("dark", "splitter", "grip", vec![variant_binding])
        .unwrap();
    assert!(variant_editor.dirty());
    let encoded = manifest.to_json().unwrap();
    assert_eq!(ThemeManifest::parse_json(&encoded).unwrap(), manifest);

    let report = inspect_theme_assets(&manifest, &package.0).unwrap();
    assert!(report.is_verified("panel-vector"));
    let svg_capabilities = ThemeAdapterCapabilities {
        supported_features: BTreeSet::new(),
        supported_asset_formats: BTreeSet::from([ThemeAssetFormatName::Svg]),
        supports_raster_assets: false,
    };
    let resolved = resolve_theme(
        &manifest,
        "dark",
        &BTreeMap::new(),
        &svg_capabilities,
        Some(&report),
    )
    .unwrap();
    assert!(resolved.assets[0].enabled);
    assert!(
        !resolved.svg_bindings["splitter"]["grip"].is_empty(),
        "resolved bindings: {:?}; resolved component values: {:?}; source bindings: {:?}",
        resolved.svg_bindings,
        resolved.components["splitter"]["grip"],
        manifest.components["splitter"].states["grip"].svg_bindings
    );
    assert_eq!(
        resolved.components["splitter"]["grip"]["decoration"],
        ThemeValue::Asset("panel-vector".into())
    );
    assert_eq!(
        resolved
            .svg_stylesheet("panel-vector", "splitter", "grip")
            .unwrap(),
        ".grip-fill { fill: #303947; }\n.grip-fill { stroke-width: 2; }\n"
    );

    let mut unsafe_selector = manifest.clone();
    unsafe_selector
        .components
        .get_mut("splitter")
        .unwrap()
        .states
        .get_mut("grip")
        .unwrap()
        .svg_bindings[0]
        .selector = ".grip, script".into();
    assert_eq!(
        unsafe_selector.validate().unwrap_err().code,
        ThemeErrorCode::InvalidAsset
    );
    let mut unsafe_value = manifest.clone();
    unsafe_value
        .components
        .get_mut("splitter")
        .unwrap()
        .states
        .get_mut("grip")
        .unwrap()
        .tokens
        .insert(
            "svg.fill".into(),
            ThemeValue::Text("url(https://example.invalid)".into()),
        );
    assert_eq!(
        unsafe_value.validate().unwrap_err().code,
        ThemeErrorCode::TypeMismatch
    );

    for (source, reason) in [
        (
            r#"<svg viewBox="0 0 32 16"><image href="https://example.invalid/image.png"/></svg>"#,
            "external href",
        ),
        (
            r#"<svg viewBox="0 0 32 16"><script>alert(1)</script></svg>"#,
            "script element",
        ),
        (
            r#"<svg viewBox="0 0 32 16"><style>@import url(https://example.invalid/a.css)</style></svg>"#,
            "external stylesheet",
        ),
        (
            r#"<!DOCTYPE svg [<!ENTITY x SYSTEM "file:///private">]><svg viewBox="0 0 32 16"/>"#,
            "entity declaration",
        ),
    ] {
        fs::write(&asset_path, source).unwrap();
        let report = inspect_theme_assets(&manifest, &package.0).unwrap();
        assert!(
            !report.is_verified("panel-vector"),
            "{reason} must fail closed"
        );
        let unsupported = resolve_theme(
            &manifest,
            "dark",
            &BTreeMap::new(),
            &svg_capabilities,
            Some(&report),
        )
        .unwrap();
        assert!(!unsupported.assets[0].enabled, "{reason} must not resolve");
        assert_eq!(
            unsupported.components["splitter"]["grip"]["decoration"],
            ThemeValue::Color("#303947".into()),
            "{reason} must keep the token fallback"
        );
        assert!(
            unsupported
                .svg_stylesheet("panel-vector", "splitter", "grip")
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn theme_schema_v2_adapts_to_v3_without_losing_existing_theme_values() {
    let mut legacy: Value = serde_json::from_str(SAMPLE).unwrap();
    legacy["schema_version"] = Value::from(2);
    if let Some(components) = legacy.get_mut("components").and_then(Value::as_object_mut) {
        for component in components.values_mut() {
            if let Some(states) = component.get_mut("states").and_then(Value::as_object_mut) {
                for state in states.values_mut() {
                    state.as_object_mut().unwrap().remove("svg_bindings");
                }
            }
        }
    }
    let legacy_json = serde_json::to_string(&legacy).unwrap();
    let upgraded = ThemeManifest::parse_json(&legacy_json).unwrap();
    let current = sample();
    assert_eq!(upgraded.schema_version, THEME_SCHEMA_VERSION);
    assert_eq!(upgraded.id, current.id);
    assert_eq!(upgraded.tokens, current.tokens);
    assert_eq!(upgraded.variants, current.variants);
    assert_eq!(upgraded.assets, current.assets);
    assert!(
        upgraded
            .components
            .values()
            .flat_map(|component| component.states.values())
            .all(|state| state.svg_bindings.is_empty())
    );
    legacy["components"]["splitter"]["states"]["grip"]["svg_bindings"] = serde_json::json!([]);
    assert_eq!(
        ThemeManifest::parse_json(&legacy.to_string())
            .unwrap_err()
            .code,
        ThemeErrorCode::UnsupportedSchemaVersion
    );

    let package = TempPackage::new();
    let path = package.0.join("theme-v2.json");
    legacy["components"]["splitter"]["states"]["grip"]
        .as_object_mut()
        .unwrap()
        .remove("svg_bindings");
    let original_bytes = serde_json::to_vec_pretty(&legacy).unwrap();
    fs::write(&path, &original_bytes).unwrap();
    let (upgraded_editor, migrated) = ThemePackageEditor::load_and_upgrade(&path).unwrap();
    assert!(migrated);
    assert_eq!(
        upgraded_editor.manifest().schema_version,
        THEME_SCHEMA_VERSION
    );
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        original_bytes
    );
    assert_eq!(
        ThemePackageEditor::load(&path).unwrap().manifest(),
        upgraded_editor.manifest()
    );
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
