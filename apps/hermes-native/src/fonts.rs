use egui::{FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use std::sync::Arc;

/// Install the user's chosen typefaces while retaining egui's bundled fallback
/// fonts for symbols, emoji, and glyphs outside either typeface's coverage.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());

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
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_jetbrains_ui_and_fira_code_families_with_fallbacks() {
        let ctx = egui::Context::default();
        install(&ctx);
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
    }
}
