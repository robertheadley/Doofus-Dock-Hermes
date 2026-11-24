use doofusdock_render_egui::svg::{SvgRenderOptions, rasterize_svg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">
        <circle class="accent" cx="12" cy="12" r="10"/>
    </svg>"##;
    let image = rasterize_svg(
        source,
        SvgRenderOptions {
            output_size: [48, 48],
            style_sheet: Some(".accent { fill: #f29b62 !important; }"),
        },
    )?;
    assert_eq!(image.size, [48, 48]);
    println!(
        "Rendered SVG to {}x{} egui pixels",
        image.width(),
        image.height()
    );
    Ok(())
}
