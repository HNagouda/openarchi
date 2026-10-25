//! An analysis section's facts or design notes, laid out at a block's width.

use super::draw::Drawing;
use crate::analysis::{fact_grid, heading, note_list, small, Paint, Section};

/// The note a section without design notes shows, so the block is not left empty.
const NO_NOTES: &str = "No design notes for this section.";
/// The room between a section's notes, warnings and source line.
const GAP_MM: f64 = 2.0;

/// The section title over its facts in a grid; a long value takes as many
/// columns as it needs and moves to the next row when this one is full.
pub fn facts(section: &Section, width: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let y = heading(&mut body, &section.title, 0.0, 0.0, width, paint);
    let height = fact_grid(&mut body, &section.facts, 0.0, y, width, paint);

    Drawing {
        body,
        width,
        height,
    }
}

/// The section title over its design notes, any warnings, and the line
/// saying where its numbers come from.
pub fn notes(section: &Section, width: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let mut y = heading(&mut body, &section.title, 0.0, 0.0, width, paint);
    if section.notes.is_empty() {
        y = small(&mut body, NO_NOTES, 0.0, y, width, paint);
    }
    y = note_list(&mut body, &section.notes, 0.0, y, width, paint);
    for warning in &section.warnings {
        y += GAP_MM;
        y = small(&mut body, warning, 0.0, y, width, paint);
    }
    y += GAP_MM;
    y = small(&mut body, &section.source, 0.0, y, width, paint);

    Drawing {
        body,
        width,
        height: y,
    }
}
