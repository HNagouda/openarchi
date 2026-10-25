//! Legend rows only a map knows to add: what a larger symbol means, and the
//! transit lines that call at the drawn stops.

use super::legend::{Legend, Row, Swatch};
use super::points;
use crate::sources::osm;

const MERGED_LABEL: &str = "Larger symbol: several places, or a big bicycle park";

/// A row after the classes saying what a larger symbol means.
pub(super) fn add_merged_row(legend: &mut Legend, ink: &str) {
    let after_classes = legend
        .rows
        .iter()
        .rposition(|row| matches!(row.swatch, Swatch::Symbol { .. }))
        .map_or(0, |index| index + 1);
    legend.rows.insert(
        after_classes,
        Row {
            label: MERGED_LABEL.to_string(),
            swatch: Swatch::Dot {
                fill: None,
                stroke: Some(ink.to_string()),
                opacity: 1.0,
            },
        },
    );
}

/// "Tram" -> "Tram 2 · 13 · 17": each transit mode lists the lines that call
/// at a drawn stop. A mode whose lines only call at stops of a busier mode,
/// such as buses at a tram stop, gets a row of its own without a symbol.
pub(super) fn add_lines_to_mode_rows(legend: &mut Legend, lines: &[String]) {
    let mut after_symbols = legend
        .rows
        .iter()
        .rposition(|row| matches!(row.swatch, Swatch::Symbol { .. }))
        .map_or(0, |index| index + 1);
    for (_, mode) in osm::TRANSIT_GROUPS {
        let prefix = format!("{mode} ");
        let mut of_mode: Vec<String> = lines
            .iter()
            .filter(|line| line.starts_with(&prefix))
            .cloned()
            .collect();
        if of_mode.is_empty() {
            continue;
        }
        of_mode.sort_by_key(|line| osm::line_order(&line[prefix.len()..]));
        let label = format!("{mode} {}", points::line_numbers(&of_mode));
        match legend.rows.iter_mut().find(|row| row.label == mode) {
            Some(row) => row.label = label,
            None => {
                let swatch = Swatch::Dot {
                    fill: None,
                    stroke: None,
                    opacity: 0.0,
                };
                legend.rows.insert(after_symbols, Row { label, swatch });
                after_symbols += 1;
            }
        }
    }
}
