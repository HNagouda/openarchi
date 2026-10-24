//! The whole analysis as A4 portrait pages, for the PDF export.
//!
//! Each section is drawn once as a block: its heading and facts across the
//! page, then the figures in a left column and the caption and notes beside
//! them. Blocks are then dealt onto pages, a block that does not fit going to
//! the next page whole. Every page after the first carries a running header
//! with the project and the site, every page a page number, and the last page
//! the data credits. The single-section exports keep the one-page layout of `page`.

use std::fmt::Write;

use super::a4_blocks::{credits_block, failures_block, section_block, Block};
use super::figure::{text, Paint};
use super::Section;
use crate::theme::Theme;
use crate::Result;

const PAGE_WIDTH_MM: f64 = 210.0;
const PAGE_HEIGHT_MM: f64 = 297.0;
const MARGIN_MM: f64 = 20.0;
pub(super) const CONTENT_MM: f64 = PAGE_WIDTH_MM - 2.0 * MARGIN_MM;
/// Where the running header and the page number sit, and where content may run.
const HEADER_BASELINE_MM: f64 = 13.0;
const HEADER_RULE_MM: f64 = 15.5;
const CONTENT_TOP_MM: f64 = 24.0;
const FOOTER_BASELINE_MM: f64 = PAGE_HEIGHT_MM - 10.0;
const CONTENT_BOTTOM_MM: f64 = PAGE_HEIGHT_MM - 18.0;

const KICKER_MM: f64 = 3.2;
const TITLE_MM: f64 = 11.0;
const SITE_LINE_MM: f64 = 3.8;
const TITLE_RULE_MM: f64 = 0.6;
const HEADER_MM: f64 = 2.5;
pub(super) const HAIRLINE_MM: f64 = 0.2;
const SECTION_GAP_MM: f64 = 12.0;

/// The pages as SVG documents, 210 × 297 mm each, in order.
pub fn report_pages(
    sections: &[Section],
    failures: &[(String, String)],
    theme: &Theme,
    project_name: &str,
) -> Result<Vec<String>> {
    let paint = Paint::new(theme)?;
    let site = site_line(sections);
    let mut blocks: Vec<Block> = sections
        .iter()
        .map(|section| section_block(section, &paint))
        .collect();
    if !failures.is_empty() {
        blocks.push(failures_block(failures, &paint));
    }
    blocks.push(credits_block(sections, &paint));

    let (title, first_top) = title_block(project_name, &site, &paint);
    let mut pages = vec![title];
    let mut y = first_top;
    for block in blocks {
        let available = CONTENT_BOTTOM_MM - CONTENT_TOP_MM;
        if y + block.height > CONTENT_BOTTOM_MM && y > CONTENT_TOP_MM {
            pages.push(String::new());
            y = CONTENT_TOP_MM;
        }
        // A block taller than a whole page is drawn smaller rather than cut.
        let scale = if block.height > available {
            available / block.height
        } else {
            1.0
        };
        let page = pages.last_mut().expect("there is always a page");
        let _ = writeln!(
            page,
            r#"<g transform="translate({MARGIN_MM} {y:.2}) scale({scale:.4})">"#
        );
        page.push_str(&block.body);
        page.push_str("</g>\n");
        y += block.height * scale + SECTION_GAP_MM;
    }

    let count = pages.len();
    Ok(pages
        .iter()
        .enumerate()
        .map(|(index, body)| page_document(body, index + 1, count, project_name, &site, &paint))
        .collect())
}

/// "Westermarkt 1, Amsterdam" from the location section, or its coordinates.
fn site_line(sections: &[Section]) -> String {
    let Some(location) = sections.iter().find(|section| section.id == "location") else {
        return String::new();
    };
    let fact = |label: &str| {
        location
            .facts
            .iter()
            .find(|fact| fact.label == label)
            .map(|fact| fact.value.clone())
    };
    match (fact("Address"), fact("Municipality")) {
        (Some(address), Some(town)) if address.ends_with(&town) => address,
        (Some(address), Some(town)) => format!("{address}, {town}"),
        _ => fact("Coordinates").unwrap_or_default(),
    }
}

/// The first page's heading. Returns its body and the y below it.
fn title_block(project_name: &str, site: &str, paint: &Paint) -> (String, f64) {
    let mut out = String::new();
    let mut y = CONTENT_TOP_MM;
    text(
        &mut out,
        MARGIN_MM,
        y,
        KICKER_MM,
        600,
        &paint.accent,
        "start",
        &paint.label("Site analysis"),
    );
    y += TITLE_MM + 2.0;
    text(
        &mut out,
        MARGIN_MM,
        y,
        TITLE_MM,
        paint.face.title_weight,
        &paint.ink,
        "start",
        project_name,
    );
    y += SITE_LINE_MM + 3.0;
    text(
        &mut out,
        MARGIN_MM,
        y,
        SITE_LINE_MM,
        400,
        &paint.secondary,
        "start",
        site,
    );
    y += 5.0;
    let _ = writeln!(
        out,
        r#"<rect x="{MARGIN_MM}" y="{y:.2}" width="{CONTENT_MM}" height="{TITLE_RULE_MM}" fill="{}"/>"#,
        paint.ink
    );

    (out, y + TITLE_RULE_MM + SECTION_GAP_MM)
}

/// A finished page: paper, the running header after the first page, the
/// content, and the page number.
fn page_document(
    body: &str,
    number: usize,
    count: usize,
    project_name: &str,
    site: &str,
    paint: &Paint,
) -> String {
    let mut out = String::with_capacity(body.len() + 1024);
    let _ = writeln!(
        out,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="{PAGE_WIDTH_MM}mm" height="{PAGE_HEIGHT_MM}mm" viewBox="0 0 {PAGE_WIDTH_MM} {PAGE_HEIGHT_MM}" font-family="{}">"#,
        paint.face.css_stack()
    );
    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{PAGE_WIDTH_MM}" height="{PAGE_HEIGHT_MM}" fill="{}"/>"#,
        paint.paper
    );
    if number > 1 {
        let left = format!("{project_name} · {}", paint.label("Site analysis"));
        text(
            &mut out,
            MARGIN_MM,
            HEADER_BASELINE_MM,
            HEADER_MM,
            600,
            &paint.ink,
            "start",
            &left,
        );
        let right = PAGE_WIDTH_MM - MARGIN_MM;
        text(
            &mut out,
            right,
            HEADER_BASELINE_MM,
            HEADER_MM,
            400,
            &paint.secondary,
            "end",
            site,
        );
        let _ = writeln!(
            out,
            r#"<line x1="{MARGIN_MM}" y1="{HEADER_RULE_MM}" x2="{right}" y2="{HEADER_RULE_MM}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
            paint.faint
        );
    }
    out.push_str(body);
    let number_text = format!("{number} / {count}");
    text(
        &mut out,
        PAGE_WIDTH_MM - MARGIN_MM,
        FOOTER_BASELINE_MM,
        HEADER_MM,
        400,
        &paint.secondary,
        "end",
        &number_text,
    );
    out.push_str("</svg>\n");

    out
}
