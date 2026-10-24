//! Pages as SVG documents: the background, every block fitted into its box,
//! and the page number.
//!
//! The app draws each block from `block_svg` at the block's position over
//! `backdrop_svg`, which is the page without its blocks. `page_svg` puts
//! the very same block bodies at the same positions, so the editor shows what
//! the export contains.

use std::fmt::Write;

use super::data::ReportData;
use super::draw::{draw, Drawing};
use super::{Block, Content, Page, Report};
use crate::analysis::{text, Paint};
use crate::render::{escape, resolve_colour};
use crate::theme::Theme;
use crate::{Error, Result};

/// Where the page number sits, from the page's bottom-right corner.
const NUMBER_RIGHT_MM: f64 = 12.0;
const NUMBER_BOTTOM_MM: f64 = 8.0;
const NUMBER_MM: f64 = 2.5;
/// A scale this close to 1 is left out of the transform.
const SCALE_EPSILON: f64 = 1e-6;

/// Page `index` with its blocks, numbered when the report asks for it.
pub fn page_svg(report: &Report, index: usize, data: &ReportData) -> Result<String> {
    let page = page_at(report, index)?;
    let paint = Paint::new(data.theme)?;
    let mut body = String::new();
    for block in &page.blocks {
        let drawing = draw(
            &block.content,
            block.width_mm,
            block.height_mm,
            data,
            &paint,
        )?;
        let _ = write!(
            body,
            r#"<g id="{}" inkscape:groupmode="layer" inkscape:label="{}" transform="translate({:.2} {:.2})">"#,
            escape(&block.id),
            escape(&label(&block.content)),
            block.x_mm,
            block.y_mm
        );
        body.push_str(&fitted(&drawing, block, &paint));
        body.push_str("</g>\n");
    }

    document(report, index, page, &body, data.theme)
}

/// Every page of the report, in order.
pub fn pages_svg(report: &Report, data: &ReportData) -> Result<Vec<String>> {
    (0..report.pages.len())
        .map(|index| page_svg(report, index, data))
        .collect()
}

/// Page `index` with nothing on it but its background and page number.
pub fn backdrop_svg(report: &Report, index: usize, theme: &Theme) -> Result<String> {
    let page = page_at(report, index)?;
    document(report, index, page, "", theme)
}

/// One block on its own, `width_mm` × `height_mm`, with no background, for
/// the editor to lay over the backdrop.
pub fn block_svg(block: &Block, data: &ReportData) -> Result<String> {
    let paint = Paint::new(data.theme)?;
    let drawing = draw(
        &block.content,
        block.width_mm,
        block.height_mm,
        data,
        &paint,
    )?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.2}mm" height="{h:.2}mm" viewBox="0 0 {w:.2} {h:.2}" overflow="visible">"#,
        w = block.width_mm,
        h = block.height_mm,
    );
    out.push_str(&fitted(&drawing, block, &paint));
    out.push_str("</svg>\n");

    Ok(out)
}

fn page_at(report: &Report, index: usize) -> Result<&Page> {
    report.pages.get(index).ok_or_else(|| {
        Error::Invalid(format!(
            "report {} has {} pages, so there is no page {}",
            report.id,
            report.pages.len(),
            index + 1
        ))
    })
}

fn document(
    report: &Report,
    index: usize,
    page: &Page,
    body: &str,
    theme: &Theme,
) -> Result<String> {
    let paint = Paint::new(theme)?;
    let background = resolve_colour(&theme.palette, &report.background)?;
    let (width, height) = page.size_mm();
    let mut out = String::with_capacity(body.len() + 1024);
    let _ = writeln!(
        out,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="{width}mm" height="{height}mm" viewBox="0 0 {width} {height}" font-family="{}">"#,
        paint.face.css_stack()
    );
    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{width}" height="{height}" fill="{background}"/>"#
    );
    out.push_str(body);
    if report.show_page_numbers {
        let number = format!("{} / {}", index + 1, report.pages.len());
        let x = width - NUMBER_RIGHT_MM;
        let y = height - NUMBER_BOTTOM_MM;
        text(
            &mut out,
            x,
            y,
            NUMBER_MM,
            400,
            &paint.secondary,
            "end",
            &number,
        );
    }
    out.push_str("</svg>\n");

    Ok(out)
}

/// The drawing scaled to fit the block, centred across it and set at its top,
/// in the palette's type, with its ids made unique to the block.
fn fitted(drawing: &Drawing, block: &Block, paint: &Paint) -> String {
    let scale = (block.width_mm / drawing.width).min(block.height_mm / drawing.height);
    let offset = (block.width_mm - drawing.width * scale) / 2.0;
    let mut out = String::with_capacity(drawing.body.len() + 256);
    let _ = write!(out, r#"<g font-family="{}""#, paint.face.css_stack());
    if (scale - 1.0).abs() > SCALE_EPSILON || offset.abs() > SCALE_EPSILON {
        let _ = write!(
            out,
            r#" transform="translate({offset:.3} 0) scale({scale:.5})""#
        );
    }
    out.push('>');
    out.push_str(&scope_ids(&drawing.body, &block.id));
    out.push_str("</g>");

    out
}

/// Prefixes every id, and every reference to one, with the block's id. Two
/// sheets on a page both define "frame-clip"; without this the second would
/// be clipped by the first one's circle.
pub(super) fn scope_ids(body: &str, prefix: &str) -> String {
    body.replace(r#" id=""#, &format!(r#" id="{prefix}-"#))
        .replace("url(#", &format!("url(#{prefix}-"))
        .replace(r##"href="#"##, &format!(r##"href="#{prefix}-"##))
}

/// The block's name in a layers panel: "Ratios: land_use".
fn label(content: &Content) -> String {
    match content {
        Content::Ratios { group, .. } => format!("Ratios: {group}"),
        Content::AnalysisFigure { section, figure } => format!("Analysis: {section}, {figure}"),
        Content::AnalysisFacts { section } => format!("Analysis: {section}, facts"),
        Content::AnalysisNotes { section } => format!("Analysis: {section}, notes"),
        Content::Sheet { diagram_id, .. } => format!("Sheet: {diagram_id}"),
        Content::Text { text, .. } => {
            let first = text.lines().next().unwrap_or_default();
            format!("Text: {first}")
        }
    }
}
