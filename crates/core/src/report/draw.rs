//! One block's content drawn at its natural size, ready to be fitted into its box.
//!
//! Text-like content (facts, notes, tables, bars, text) is laid out at the
//! box's width, so it keeps its print size and only shrinks when it runs
//! taller than the box. Pictures (figures, pies, sheets) keep their own
//! proportions and are scaled to the box.

use std::fmt::Write;

use super::data::ReportData;
use super::{analysis_blocks, ratio_blocks, text_block, Content};
use crate::analysis::Paint;
use crate::catalog::current_id;
use crate::project::Look;
use crate::render::{escape, render_svg, wrap_text, RenderInput};
use crate::Result;

/// The placeholder's message size and the gap around it.
const MESSAGE_MM: f64 = 2.8;
const MESSAGE_LINE_MM: f64 = 4.0;
const MESSAGE_INSET_MM: f64 = 4.0;
const DASH_MM: f64 = 1.2;
const OUTLINE_MM: f64 = 0.25;

/// A drawing with its top-left at 0, 0, in millimetres.
pub(super) struct Drawing {
    pub body: String,
    pub width: f64,
    pub height: f64,
}

pub(super) fn draw(
    content: &Content,
    width: f64,
    height: f64,
    data: &ReportData,
    paint: &Paint,
) -> Result<Drawing> {
    let found = match content {
        Content::Ratios { group, style } => data
            .group(group)
            .map(|group| ratio_blocks::draw(group, *style, width, paint)),
        Content::AnalysisFigure { section, figure } => data.section(section).and_then(|found| {
            found
                .figures
                .iter()
                .find(|candidate| candidate.id == *figure)
                .map(|figure| Drawing {
                    body: figure.body.clone(),
                    width: figure.width_mm,
                    height: figure.height_mm,
                })
                .ok_or_else(|| format!("{} has no figure “{figure}”.", found.title))
        }),
        Content::AnalysisFacts { section } => data
            .section(section)
            .map(|section| analysis_blocks::facts(section, width, paint)),
        Content::AnalysisNotes { section } => data
            .section(section)
            .map(|section| analysis_blocks::notes(section, width, paint)),
        Content::Sheet {
            diagram_id,
            variant,
        } => sheet(diagram_id, variant.as_deref(), data)?,
        Content::Text { text, style } => {
            Ok(text_block::draw(text, style, width, height, data.theme)?)
        }
    };

    Ok(match found {
        Ok(drawing) => drawing,
        Err(message) => placeholder(&message, width, height, paint),
    })
}

/// The project's sheet in another diagram and the report's palette, without
/// its document wrapper. An unknown diagram is a placeholder, not an error.
fn sheet(
    diagram_id: &str,
    variant: Option<&str>,
    data: &ReportData,
) -> Result<std::result::Result<Drawing, String>> {
    let (id, variant) = current_id(diagram_id, variant);
    let Some(diagram) = data.diagrams.iter().find(|diagram| diagram.id == id) else {
        return Ok(Err(format!("There is no diagram “{diagram_id}”.")));
    };
    let look = Look {
        diagram_id: id,
        variant,
        theme_id: data.theme.id.clone(),
        ..data.look.clone()
    };
    let input = RenderInput {
        scene: data.scene,
        diagram,
        theme: data.theme,
        look: &look,
        pins: data.pins,
    };
    let svg = render_svg(&input)?;
    let (width, height) = look.sheet.size_mm();

    Ok(Ok(Drawing {
        body: inner_svg(&svg).to_string(),
        width,
        height,
    }))
}

/// What lies between a document's opening `<svg …>` tag and its closing tag.
pub(super) fn inner_svg(document: &str) -> &str {
    let start = document
        .find("<svg")
        .and_then(|open| document[open..].find('>').map(|end| open + end + 1))
        .unwrap_or(0);
    let end = document.rfind("</svg>").unwrap_or(document.len());

    document.get(start..end).unwrap_or_default()
}

/// A dashed box with the reason the block has nothing to show, so a missing
/// part is visible on the page instead of silently empty.
fn placeholder(message: &str, width: f64, height: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let _ = writeln!(
        body,
        r#"<rect x="{half:.2}" y="{half:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="{OUTLINE_MM}" stroke-dasharray="{DASH_MM} {DASH_MM}"/>"#,
        width - OUTLINE_MM,
        height - OUTLINE_MM,
        paint.faint,
        half = OUTLINE_MM / 2.0,
    );
    let text_width = (width - 2.0 * MESSAGE_INSET_MM).max(MESSAGE_MM);
    let lines = wrap_text(
        message,
        &paint.face,
        400,
        false,
        MESSAGE_MM,
        0.0,
        text_width,
    );
    let mut baseline = MESSAGE_INSET_MM + MESSAGE_MM;
    for line in lines {
        let _ = writeln!(
            body,
            r#"<text x="{MESSAGE_INSET_MM}" y="{baseline:.2}" font-size="{MESSAGE_MM}" fill="{}">{}</text>"#,
            paint.secondary,
            escape(&line)
        );
        baseline += MESSAGE_LINE_MM;
    }

    Drawing {
        body,
        width,
        height,
    }
}
