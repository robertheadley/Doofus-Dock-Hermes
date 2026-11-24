use doofusdock_core::{
    ThemeAdapterCapabilities, ThemeAsset, ThemeAssetFormat, ThemeAssetFormatName, ThemeManifest,
    ThemePackageEditor, ThemeSvgStyleBinding, ThemeSvgStyleProperty, ThemeTiling,
    ThemeTransparency, ThemeValue, inspect_theme_assets, resolve_theme,
};
use doofusdock_render_egui::svg::{SvgRenderOptions, rasterize_svg_file};
use std::{collections::BTreeMap, collections::BTreeSet, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manifest = ThemeManifest::parse_json(include_str!(
        "../../../themes/doofusdock-workbench/theme.json"
    ))?;
    manifest.assets.push(ThemeAsset {
        id: "sample-vector-mark".into(),
        path: "examples/assets/theme-vector-mark.svg".into(),
        format: ThemeAssetFormat::Svg,
        source_width: 24,
        source_height: 24,
        scale: 1.0,
        transparency: ThemeTransparency::Alpha,
        color_key: None,
        nine_slice: None,
        edge_tiling: ThemeTiling::Stretch,
        center_tiling: ThemeTiling::Stretch,
        provenance: None,
    });
    let mut editor = ThemePackageEditor::new(manifest)?;
    let mut draft = editor.manifest().clone();
    draft
        .components
        .get_mut("splitter")
        .unwrap()
        .states
        .get_mut("grip")
        .unwrap()
        .tokens
        .insert(
            "svg.fill".into(),
            ThemeValue::Reference("color.accent".into()),
        );
    editor.replace_draft(draft)?;
    let mut bindings = editor.manifest().components["splitter"].states["grip"]
        .svg_bindings
        .clone();
    bindings.push(ThemeSvgStyleBinding {
        asset_id: "sample-vector-mark".into(),
        selector: "circle".into(),
        property: ThemeSvgStyleProperty::Fill,
        token: "svg.fill".into(),
    });
    editor.set_component_state_svg_bindings("splitter", "grip", bindings)?;
    let manifest = editor.manifest().clone();
    manifest.validate()?;

    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let report = inspect_theme_assets(&manifest, &package_root)?;
    let capabilities = ThemeAdapterCapabilities {
        supported_features: BTreeSet::from(["custom_chrome".into()]),
        supported_asset_formats: BTreeSet::from([ThemeAssetFormatName::Svg]),
        supports_raster_assets: false,
    };
    let resolved = resolve_theme(
        &manifest,
        "dark",
        &BTreeMap::new(),
        &capabilities,
        Some(&report),
    )?;
    let asset = resolved
        .assets
        .iter()
        .find(|asset| asset.id == "sample-vector-mark")
        .ok_or("resolved SVG asset is missing")?;
    if !asset.enabled {
        return Err("theme SVG asset did not pass verification and capability negotiation".into());
    }

    let path = package_root.join(&asset.path);
    let style_sheet = resolved.svg_stylesheet("sample-vector-mark", "splitter", "grip")?;
    let image = rasterize_svg_file(
        &path,
        SvgRenderOptions {
            output_size: [48, 48],
            style_sheet: Some(&style_sheet),
        },
    )?;
    println!(
        "theme={}, asset={}, enabled={}, rendered={}x{}",
        resolved.theme_id, asset.id, asset.enabled, image.size[0], image.size[1]
    );
    Ok(())
}
