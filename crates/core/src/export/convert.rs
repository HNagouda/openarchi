//! The rendered SVG to PDF and PNG, with only the bundled fonts available.

use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Pixmap, Transform};
use usvg::{fontdb, Options, Tree};

use crate::render::BUNDLED_FONTS;
use crate::{Error, Result};

/// usvg measures SVG lengths in CSS pixels, 96 to the inch.
const CSS_DPI: f64 = 96.0;
/// A0 at 300 dpi is about 140 million pixels, half a gigabyte of RAM. Beyond
/// this, PDF is the format to use.
const MAX_PNG_PIXELS: u64 = 150_000_000;

/// The bundled fonts, loaded once per process.
fn font_database() -> Arc<fontdb::Database> {
    static DATABASE: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    DATABASE
        .get_or_init(|| {
            let mut database = fontdb::Database::new();
            for (_, _, _, bytes) in BUNDLED_FONTS {
                database.load_font_data(bytes.to_vec());
            }
            database.set_sans_serif_family("Inter");
            database.set_serif_family("Source Serif 4");
            database.set_monospace_family("IBM Plex Mono");
            Arc::new(database)
        })
        .clone()
}

fn parse(svg: &str) -> Result<Tree> {
    let options = Options {
        fontdb: font_database(),
        font_family: "Inter".to_string(),
        ..Options::default()
    };
    Tree::from_str(svg, &options)
        .map_err(|error| Error::Invalid(format!("could not read the rendered sheet: {error}")))
}

pub fn pdf(svg: &str) -> Result<Vec<u8>> {
    let tree = parse(svg)?;
    // Telling svg2pdf the same 96 dpi usvg used keeps millimetres millimetres on the PDF page.
    let page = svg2pdf::PageOptions {
        dpi: CSS_DPI as f32,
    };
    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), page)
        .map_err(|error| Error::Invalid(format!("could not convert the sheet to PDF: {error}")))
}

pub fn png_at_dpi(svg: &str, dpi: u32) -> Result<Vec<u8>> {
    let tree = parse(svg)?;
    rasterise(&tree, dpi as f64 / CSS_DPI)
}

pub fn png_at_width(svg: &str, width_px: u32) -> Result<Vec<u8>> {
    let tree = parse(svg)?;
    rasterise(&tree, width_px as f64 / tree.size().width() as f64)
}

fn rasterise(tree: &Tree, zoom: f64) -> Result<Vec<u8>> {
    let width = (tree.size().width() as f64 * zoom).round().max(1.0) as u32;
    let height = (tree.size().height() as f64 * zoom).round().max(1.0) as u32;
    if width as u64 * height as u64 > MAX_PNG_PIXELS {
        return Err(Error::Invalid(format!(
            "a {width} × {height} px PNG is too large to render; lower the resolution or export a PDF"
        )));
    }

    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| Error::Invalid(format!("cannot make a {width} × {height} px image")))?;
    resvg::render(
        tree,
        Transform::from_scale(zoom as f32, zoom as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| Error::Invalid(format!("could not encode the PNG: {error}")))
}
