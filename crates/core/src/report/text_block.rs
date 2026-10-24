//! A text block: the user's words, set like a sheet's text box and wrapped
//! at the block's width.
//!
//! The text is written as the same data-carrying group the sheet uses, so the
//! app reads the resolved family, size and colour off the block's SVG for its
//! formatting bar. It never shrinks to fit: words that run past the bottom of
//! the box stay readable, and the user makes the box taller.

use super::draw::Drawing;
use crate::render::{close_group, open_group, write_lines, StyledText, TextKind, Typesetter};
use crate::sheet::TextStyle;
use crate::theme::Theme;
use crate::Result;

/// Report text is set at the A3 sheet's type scale, whatever the page size.
const TYPE_UNIT: f64 = 1.0;
/// The id the app finds the text group by.
const GROUP_ID: &str = "report-text";

pub fn draw(
    text: &str,
    style: &TextStyle,
    width: f64,
    height: f64,
    theme: &Theme,
) -> Result<Drawing> {
    let palette = &theme.palette;
    let typesetter = Typesetter::new(&theme.typography, TYPE_UNIT, &palette.ink, &palette.accent);
    let styled = StyledText::new(&typesetter, TextKind::Note, style, palette)?;

    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(styled.wrap(line, width));
        }
    }

    let mut body = String::new();
    let x = styled.anchor_x(0.0, width);
    open_group(&mut body, &styled, GROUP_ID, x, 0.0);
    write_lines(&mut body, &styled, &lines, x, styled.cap_height());
    close_group(&mut body);

    Ok(Drawing {
        body,
        width,
        height,
    })
}
