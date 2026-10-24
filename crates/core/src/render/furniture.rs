//! Everything on the sheet that is not the map: title, legend, north arrow,
//! scale bar, credits and the openarchi mark.
//!
//! All of it stays inside the furniture zone from `layout`, until the user
//! moves the title or subtitle out of it. Each block is
//! measured before it is placed, and the legend gets whatever height is left.

use std::fmt::Write;

use super::geometry::{decimal, mm};
use super::layout::{Layout, Placement, Rect};
use super::legend::{self, Legend};
use super::svg::layer_attributes;
use super::text::{self, SheetTexts};
use super::tools::{write_tools, TOOLS_HEIGHT};
use super::typeset::{Kind, Typesetter};
use crate::scene::Scene;
use crate::sheet::{Sheet, SUBTITLE_ID, TITLE_ID};
use crate::theme::Palette;

const MARK_TEXT: &str = "openarchi";

// Spacing at A3, in millimetres; multiplied by the layout unit.
const EYEBROW_GAP: f64 = 3.4;
const SUBTITLE_GAP: f64 = 3.0;
const TITLE_TO_LEGEND: f64 = 9.0;
const RULE_TO_LEGEND: f64 = 5.0;
const LEGEND_TO_TOOLS: f64 = 8.0;
const BAND_PADDING_TOP: f64 = 6.0;
const BAND_COLUMN_GAP: f64 = 9.0;
/// Shares of the band's width for the title column and the tools column.
const BAND_TITLE_SHARE: f64 = 0.32;
const BAND_TOOLS_SHARE: f64 = 0.24;
const CREDITS_GAP: f64 = 2.6;
const TOOLS_TO_RULE: f64 = 5.0;
const RULE_TO_CREDITS: f64 = 4.0;
const RULE_STROKE_MM: f64 = 0.15;
const RULE_OPACITY: f64 = 0.35;
const MAX_TITLE_LINES: usize = 3;
const MAX_SUBTITLE_LINES: usize = 3;

const MARK_RING_RADIUS: f64 = 0.95;

pub struct Furniture<'a> {
    pub sheet: &'a Sheet,
    pub scene: &'a Scene,
    pub diagram_name: &'a str,
    pub palette: &'a Palette,
    pub mm_per_m: f64,
    pub legend: Option<Legend>,
    pub texts: &'a SheetTexts<'a>,
}

/// The furniture layer, then the text layer above everything: the title or
/// subtitle once the user moved it, and the user's own text boxes.
///
/// A moved title leaves the furniture zone, and the legend moves up into the
/// space it left. The furniture never makes room around free text, because a
/// legend that jumps while the user drags a title is worse than an overlap they
/// can see and fix by moving the text again.
pub fn write(out: &mut String, furniture: &Furniture, layout: &Layout, typesetter: &Typesetter) {
    let _ = write!(
        out,
        r#"<g {} font-family="{}">"#,
        layer_attributes("furniture", "Furniture"),
        typesetter.face.css_stack().replace('\'', "&apos;")
    );
    match layout.placement {
        Placement::Column => write_column(out, furniture, &layout.furniture, typesetter),
        Placement::Band => write_band(out, furniture, &layout.furniture, typesetter),
    }
    out.push_str("</g>\n");

    let sheet = furniture.sheet;
    let title_width = title_width(layout);
    text::open_layer(out);
    if let Some(position) = sheet.title_position {
        write_title(
            out,
            furniture,
            typesetter,
            position.x_mm,
            position.y_mm,
            title_width,
        );
    }
    if let Some(position) = sheet.subtitle_position {
        write_subtitle(out, furniture, position.x_mm, position.y_mm, title_width);
    }
    text::write_boxes(out, &furniture.texts.boxes, &layout.drawing, layout.unit);
    text::close_group(out);
    out.push('\n');
}

/// Title at the top, legend in the middle, tools and credits at the foot.
fn write_column(out: &mut String, furniture: &Furniture, area: &Rect, typesetter: &Typesetter) {
    let unit = typesetter.unit;
    let title_bottom = write_title_block(out, furniture, typesetter, area.x, area.y, area.width);
    let foot_top = write_foot(out, furniture, typesetter, area, area.x);

    let Some(legend) = &furniture.legend else {
        return;
    };
    // With the title and subtitle both moved away, the legend starts at the top of the column.
    let rule_y = if title_bottom > area.y {
        title_bottom + TITLE_TO_LEGEND * unit
    } else {
        area.y
    };
    let legend_top = rule_y + RULE_TO_LEGEND * unit;
    let legend_area = Rect {
        x: area.x,
        y: legend_top,
        width: area.width,
        height: foot_top - LEGEND_TO_TOOLS * unit - legend_top,
    };
    if legend_area.height > 0.0 {
        write_rule(out, area.x, area.right(), rule_y, &furniture.palette.ink);
        legend::write(out, legend, typesetter, &legend_area, 2);
    }
}

/// A rule along the top, then title, legend and tools side by side, credits along the foot.
fn write_band(out: &mut String, furniture: &Furniture, area: &Rect, typesetter: &Typesetter) {
    let unit = typesetter.unit;
    let gap = BAND_COLUMN_GAP * unit;
    let tools_width = area.width * BAND_TOOLS_SHARE;
    // With the title and subtitle both moved away, the legend takes their column too.
    let title_column = if has_title_in_layout(furniture.sheet) {
        area.width * BAND_TITLE_SHARE + gap
    } else {
        0.0
    };
    let legend_width = area.width - title_column - tools_width - gap;
    let top = area.y + BAND_PADDING_TOP * unit;

    write_rule(out, area.x, area.right(), area.y, &furniture.palette.ink);
    write_title_block(
        out,
        furniture,
        typesetter,
        area.x,
        top,
        area.width * BAND_TITLE_SHARE,
    );
    let credits_top = write_credits_and_mark(
        out,
        furniture,
        typesetter,
        area.x,
        area.bottom(),
        area.width * 0.7,
        area.right(),
    );

    if let Some(legend) = &furniture.legend {
        let legend_area = Rect {
            x: area.x + title_column,
            y: top,
            width: legend_width,
            height: credits_top - CREDITS_GAP * unit - top,
        };
        legend::write(out, legend, typesetter, &legend_area, 3);
    }
    let tools_x = area.right() - tools_width;
    write_tools(out, furniture, typesetter, tools_x, top, tools_width);
}

/// The title and subtitle the user has not moved, stacked from `top` across
/// `width`. Returns the y below the last line, or `top` when nothing stayed.
fn write_title_block(
    out: &mut String,
    furniture: &Furniture,
    typesetter: &Typesetter,
    x: f64,
    top: f64,
    width: f64,
) -> f64 {
    let sheet = furniture.sheet;
    let subtitle = &furniture.texts.subtitle;
    let mut y = top;
    if sheet.title_position.is_none() {
        let anchor_x = furniture.texts.title.anchor_x(x, width);
        y = write_title(out, furniture, typesetter, anchor_x, top, width);
    }
    if sheet.subtitle_position.is_none() && !sheet.subtitle.trim().is_empty() {
        // Under a title, the gap is measured baseline to baseline like the rest of the block.
        let subtitle_top = if y > top {
            y + SUBTITLE_GAP * typesetter.unit + subtitle.line_pitch() - subtitle.cap_height()
        } else {
            top
        };
        let anchor_x = subtitle.anchor_x(x, width);
        y = write_subtitle(out, furniture, anchor_x, subtitle_top, width);
    }

    y
}

/// The eyebrow and title as one group anchored at (`x`, `top`). Returns the last baseline.
fn write_title(
    out: &mut String,
    furniture: &Furniture,
    typesetter: &Typesetter,
    x: f64,
    top: f64,
    width: f64,
) -> f64 {
    let sheet = furniture.sheet;
    let styled = &furniture.texts.title;
    // With no title of their own, the diagram name becomes the title.
    let (eyebrow, title) = if sheet.title.trim().is_empty() {
        (None, furniture.diagram_name)
    } else {
        (Some(furniture.diagram_name), sheet.title.as_str())
    };

    text::open_group(out, styled, TITLE_ID, x, top);
    let mut y = top;
    if let Some(eyebrow) = eyebrow {
        y += typesetter.cap_height(Kind::Eyebrow);
        typesetter.write(out, x, y, eyebrow, Kind::Eyebrow, None);
        y += EYEBROW_GAP * typesetter.unit;
    }
    let lines = limited(styled.wrap(title, width), MAX_TITLE_LINES);
    let last_baseline = text::write_lines(out, styled, &lines, x, y + styled.cap_height());
    text::close_group(out);

    last_baseline
}

/// The subtitle as one group anchored at (`x`, `top`). Returns the last baseline.
fn write_subtitle(out: &mut String, furniture: &Furniture, x: f64, top: f64, width: f64) -> f64 {
    let styled = &furniture.texts.subtitle;
    let lines = limited(
        styled.wrap(&furniture.sheet.subtitle, width),
        MAX_SUBTITLE_LINES,
    );
    if lines.is_empty() {
        return top;
    }

    text::open_group(out, styled, SUBTITLE_ID, x, top);
    let last_baseline = text::write_lines(out, styled, &lines, x, top + styled.cap_height());
    text::close_group(out);

    last_baseline
}

/// True while the title or a subtitle still sits in the furniture zone.
fn has_title_in_layout(sheet: &Sheet) -> bool {
    sheet.title_position.is_none()
        || (sheet.subtitle_position.is_none() && !sheet.subtitle.trim().is_empty())
}

/// The width the title wraps to, in the layout and after it has been moved.
fn title_width(layout: &Layout) -> f64 {
    match layout.placement {
        Placement::Column => layout.furniture.width,
        Placement::Band => layout.furniture.width * BAND_TITLE_SHARE,
    }
}

/// Tools, a rule, credits and the mark, stacked up from the bottom of `area`. Returns the top of the stack.
fn write_foot(
    out: &mut String,
    furniture: &Furniture,
    typesetter: &Typesetter,
    area: &Rect,
    x: f64,
) -> f64 {
    let unit = typesetter.unit;
    let credits_top = write_credits_and_mark(
        out,
        furniture,
        typesetter,
        x,
        area.bottom(),
        area.width,
        area.right(),
    );
    let rule_y = credits_top - RULE_TO_CREDITS * unit;
    let has_tools = furniture.sheet.show_north_arrow || furniture.sheet.show_scale_bar;
    if !has_tools {
        return rule_y;
    }

    write_rule(out, x, area.right(), rule_y, &furniture.palette.ink);
    let tools_top = rule_y - TOOLS_TO_RULE * unit - TOOLS_HEIGHT * unit;
    write_tools(out, furniture, typesetter, x, tools_top, area.width);

    tools_top
}

/// Credits wrapped above `bottom`, with the mark on the last line at the right. Returns the top.
fn write_credits_and_mark(
    out: &mut String,
    furniture: &Furniture,
    typesetter: &Typesetter,
    x: f64,
    bottom: f64,
    credits_width: f64,
    right: f64,
) -> f64 {
    let unit = typesetter.unit;
    let mark_width = typesetter.width(MARK_TEXT, Kind::Mark) + 3.0 * MARK_RING_RADIUS * unit;
    write_mark(
        out,
        typesetter,
        right - mark_width,
        bottom,
        &furniture.palette.ink,
    );

    let text = credits_text(furniture.scene);
    if !furniture.sheet.show_credits || text.is_empty() {
        return bottom - typesetter.cap_height(Kind::Mark);
    }
    let width = credits_width.min(right - x - mark_width - CREDITS_GAP * unit);
    let lines = typesetter.wrap(&text, Kind::Credit, width);
    let pitch = typesetter.line_pitch(Kind::Credit);
    let first_baseline = bottom - pitch * (lines.len() as f64 - 1.0);
    for (index, line) in lines.iter().enumerate() {
        typesetter.write(
            out,
            x,
            first_baseline + pitch * index as f64,
            line,
            Kind::Credit,
            None,
        );
    }

    first_baseline - typesetter.cap_height(Kind::Credit)
}

/// A small ring, the circle-around-a-site motif, and the name.
fn write_mark(out: &mut String, typesetter: &Typesetter, x: f64, baseline: f64, ink: &str) {
    let unit = typesetter.unit;
    let radius = MARK_RING_RADIUS * unit;
    let centre_y = baseline - typesetter.cap_height(Kind::Mark) / 2.0;
    let _ = write!(
        out,
        r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{ink}" stroke-width="{}" opacity="0.45"/>"#,
        mm(x + radius),
        mm(centre_y),
        mm(radius),
        mm(0.2 * unit)
    );
    typesetter.write(out, x + 3.0 * radius, baseline, MARK_TEXT, Kind::Mark, None);
}

/// "Sources: BGT, Kadaster, CC BY 4.0; ... Retrieved 2026-10-24."
fn credits_text(scene: &Scene) -> String {
    if scene.credits.is_empty() {
        return String::new();
    }
    let sources: Vec<String> = scene
        .credits
        .iter()
        .map(|credit| format!("{} ({})", credit.name, credit.license))
        .collect();
    let date: String = scene.fetched_at.chars().take(10).collect();
    let retrieved = if date.is_empty() {
        String::new()
    } else {
        format!(" Retrieved {date}.")
    };

    format!("Sources: {}.{retrieved}", sources.join("; "))
}

fn write_rule(out: &mut String, from_x: f64, to_x: f64, y: f64, ink: &str) {
    let _ = write!(
        out,
        r#"<path d="M{} {y}H{}" stroke="{ink}" stroke-width="{}" opacity="{}"/>"#,
        mm(from_x),
        mm(to_x),
        mm(RULE_STROKE_MM),
        decimal(RULE_OPACITY),
        y = mm(y)
    );
}

/// The first `max` lines, the last one ending in an ellipsis if text was cut.
fn limited(mut lines: Vec<String>, max: usize) -> Vec<String> {
    if lines.len() > max {
        lines.truncate(max);
        if let Some(last) = lines.last_mut() {
            last.push('\u{2026}');
        }
    }

    lines
}
