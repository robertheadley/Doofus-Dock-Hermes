use egui::{FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use std::sync::Arc;

/// Install the user's chosen typefaces while retaining egui's bundled fallback
/// fonts for symbols, emoji, and glyphs outside either typeface's coverage.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
    // Catalogs, transcripts and tool output can contain arbitrary Unicode.
    // Uncovered characters should render as tofu, never panic through egui.
    ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);

    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut style = (*ctx.style_of(theme)).clone();
        for text_style in [TextStyle::Heading, TextStyle::Button] {
            if let Some(font) = style.text_styles.get_mut(&text_style) {
                *font = FontId::new(font.size, FontFamily::Name("jetbrains_mono_medium".into()));
            }
        }
        ctx.set_style_of(theme, style);
    }
}

fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "jetbrains_mono_regular".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains_mono_medium".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono/JetBrainsMono-Medium.ttf"
        ))),
    );
    fonts.font_data.insert(
        "firacode_regular".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/FiraCode/FiraCode-Regular.ttf"
        ))),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "jetbrains_mono_regular".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "firacode_regular".into());
    fonts.families.insert(
        FontFamily::Name("jetbrains_mono_medium".into()),
        vec!["jetbrains_mono_medium".into()],
    );

    #[cfg(target_os = "windows")]
    add_windows_cjk_fallback(&mut fonts);
    fonts
}

/// Use installed Windows script faces only as glyph fallbacks. The user-chosen
/// JetBrains Mono and Fira Code remain primary throughout the UI.
#[cfg(target_os = "windows")]
fn add_windows_cjk_fallback(fonts: &mut FontDefinitions) {
    for (name, file) in [
        ("windows_cjk_japanese", "YuGothR.ttc"),
        ("windows_cjk_simplified_chinese", "simsun.ttc"),
        ("windows_cjk_korean", "malgun.ttf"),
        ("windows_thai", "LeelawUI.ttf"),
        ("windows_indic", "Nirmala.ttc"),
        ("windows_multiscript", "segoeui.ttf"),
    ] {
        let path = std::path::Path::new(r"C:\Windows\Fonts").join(file);
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts.font_data.insert(
            name.into(),
            Arc::new(FontData::from_blob(Arc::new(bytes), 0)),
        );
        for family in [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name("jetbrains_mono_medium".into()),
        ] {
            fonts.families.entry(family).or_default().push(name.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_jetbrains_ui_and_fira_code_families_with_fallbacks() {
        let ctx = egui::Context::default();
        install(&ctx);
        assert_eq!(ctx.missing_glyph_policy(), egui::MissingGlyphPolicy::Tofu);
        let style = ctx.style_of(egui::Theme::Dark);
        assert!(matches!(
            style.text_styles[&TextStyle::Heading].family,
            FontFamily::Name(ref name) if name.as_ref() == "jetbrains_mono_medium"
        ));
        assert!(matches!(
            style.text_styles[&TextStyle::Body].family,
            FontFamily::Proportional
        ));
        let fonts = font_definitions();
        assert!(
            fonts.families[&FontFamily::Proportional]
                .contains(&"jetbrains_mono_regular".to_owned())
        );
        assert!(fonts.families[&FontFamily::Monospace].contains(&"firacode_regular".to_owned()));
        assert!(fonts.font_data.contains_key("Ubuntu-Light"));
        #[cfg(target_os = "windows")]
        {
            assert!(fonts.font_data.contains_key("windows_cjk_japanese"));
            assert!(
                fonts
                    .font_data
                    .contains_key("windows_cjk_simplified_chinese")
            );
            assert!(fonts.font_data.contains_key("windows_cjk_korean"));
            assert!(fonts.font_data.contains_key("windows_thai"));
            assert!(fonts.font_data.contains_key("windows_indic"));
            assert!(fonts.font_data.contains_key("windows_multiscript"));
        }
    }
}
