//! The sheet's title block: the eyebrow, the title and the subtitle, each set
//! as a text group the app can select, drag and edit.

use super::furniture::Furniture;
use super::text;
use super::typeset::{Kind, Typesetter};
use crate::sheet::{Sheet, SUBTITLE_ID, TITLE_ID};

// Spacing at A3, in millimetres; multiplied by the layout unit.

const EYEBROW_GAP: f64 = 3.4;
const SUBTITLE_GAP: f64 = 3.0;
const MAX_TITLE_LINES: usize = 3;
const MAX_SUBTITLE_LINES: usize = 3;

/// The title and subtitle the user has not moved, stacked from `top` across
/// `width`. Returns the y below the last line, or `top` when nothing stayed.
pub(super) fn write_title_block(
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
pub(super) fn write_title(
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
pub(super) fn write_subtitle(
    out: &mut String,
    furniture: &Furniture,
    x: f64,
    top: f64,
    width: f64,
) -> f64 {
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
pub(super) fn has_title_in_layout(sheet: &Sheet) -> bool {
    sheet.title_position.is_none()
        || (sheet.subtitle_position.is_none() && !sheet.subtitle.trim().is_empty())
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
