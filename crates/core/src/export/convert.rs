//! The rendered SVG to PDF and PNG, with only the bundled fonts available.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, TextStr};

use resvg::tiny_skia::{Pixmap, Transform};
use usvg::{fontdb, Options, Tree};

use crate::render::BUNDLED_FONTS;
use crate::{Error, Result};

/// usvg measures SVG lengths in CSS pixels, 96 to the inch.
const CSS_DPI: f64 = 96.0;
/// PDF measures in points, 72 to the inch.
const PDF_DPI: f64 = 72.0;
/// The name each page's drawing goes by in its resources.
const PAGE_DRAWING: Name<'static> = Name(b"Page");
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

/// Several pages into one PDF. svg2pdf turns each page into a drawing (an
/// XObject); each gets fresh object numbers and is drawn across its own page.
pub fn pdf_pages(svgs: &[String], title: &str) -> Result<Vec<u8>> {
    let mut next = Ref::new(1);
    let catalog = next.bump();
    let page_tree = next.bump();
    let mut pdf = Pdf::new();
    let mut page_refs = Vec::new();

    for svg in svgs {
        let tree = parse(svg)?;
        let (chunk, drawing) = svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default())
            .map_err(|error| Error::Invalid(format!("could not convert a page to PDF: {error}")))?;
        let mut renumbered = HashMap::new();
        let chunk = chunk.renumber(|old| *renumbered.entry(old).or_insert_with(|| next.bump()));
        let drawing = renumbered[&drawing];

        let points = PDF_DPI / CSS_DPI;
        let width = (tree.size().width() as f64 * points) as f32;
        let height = (tree.size().height() as f64 * points) as f32;
        let page_ref = next.bump();
        let content_ref = next.bump();
        let mut page = pdf.page(page_ref);
        page.media_box(Rect::new(0.0, 0.0, width, height));
        page.parent(page_tree);
        page.contents(content_ref);
        page.resources().x_objects().pair(PAGE_DRAWING, drawing);
        page.finish();

        let mut content = Content::new();
        content
            .transform([width, 0.0, 0.0, height, 0.0, 0.0])
            .x_object(PAGE_DRAWING);
        pdf.stream(content_ref, &content.finish());
        pdf.extend(&chunk);
        page_refs.push(page_ref);
    }

    pdf.catalog(catalog).pages(page_tree);
    let count = page_refs.len() as i32;
    pdf.pages(page_tree).kids(page_refs).count(count);
    pdf.document_info(next.bump())
        .title(TextStr(title))
        .producer(TextStr("openarchi"));

    Ok(pdf.finish())
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
