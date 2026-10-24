//! What every analysis chart shares: its size, the theme colours it draws
//! with, and small pieces of SVG.
//!
//! A figure is drawn at `WIDTH_MM` with one user unit per millimetre, like a
//! sheet. The inspector shows it at the panel's width, which is why the
//! labels are large for print: at 76 mm wide a 2.9 mm label is about 10 px on
//! screen and 8 pt on paper. Export pages scale figures up to their column.

use std::fmt::Write;

use serde::Serialize;

use crate::render::{escape, luminance, mix, resolve_colour, text_width, Face};
use crate::theme::Theme;
use crate::Result;

pub const WIDTH_MM: f64 = 76.0;
pub const LABEL_MM: f64 = 2.9;
pub const SMALL_MM: f64 = 2.5;
/// Inside the figure's edge, where nothing but the paper goes.
pub const INSET_MM: f64 = 3.0;
pub const HAIRLINE_MM: f64 = 0.15;
pub const LINE_MM: f64 = 0.4;

pub const MONTH_INITIALS: [&str; 12] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];

/// How far secondary text and grid lines move from ink towards paper.
const SECONDARY_SHARE: f64 = 0.42;
const FAINT_SHARE: f64 = 0.82;
/// Below this difference in luminance a colour does not show on the paper.
/// Mono paints water and green in the paper colour on purpose, for its plans;
/// a chart needs them visible, so it takes a fallback instead.
const MIN_LUMINANCE_STEP: f64 = 0.05;

const SWATCH_MM: f64 = 2.4;
const LEGEND_GAP_MM: f64 = 3.0;
const LEGEND_ROW_MM: f64 = 4.2;
/// Below this a share is written with one decimal.
const SMALL_SHARE: f64 = 0.01;

#[derive(Debug, Clone, Serialize)]
pub struct Figure {
    /// Stable within a section: "temperature", "wind-roses".
    pub id: String,
    pub width_mm: f64,
    pub height_mm: f64,
    /// A standalone SVG document with the paper behind it.
    pub svg: String,
    /// The drawing without the document around it, for export pages to place.
    #[serde(skip)]
    pub body: String,
}

/// The theme colours a chart draws with, resolved once.
pub struct Paint {
    pub paper: String,
    pub ink: String,
    pub secondary: String,
    pub faint: String,
    pub accent: String,
    pub water: String,
    pub green: String,
    pub ground: String,
    pub muted: String,
    ramp: Vec<String>,
    pub face: Face,
    pub is_uppercase: bool,
}

impl Paint {
    pub fn new(theme: &Theme) -> Result<Paint> {
        let palette = &theme.palette;
        let ramp = theme
            .ramp
            .iter()
            .map(|value| resolve_colour(palette, value))
            .collect::<Result<Vec<String>>>()?;
        Ok(Paint {
            paper: palette.paper.clone(),
            ink: palette.ink.clone(),
            secondary: mix(&palette.ink, &palette.paper, SECONDARY_SHARE),
            faint: mix(&palette.ink, &palette.paper, FAINT_SHARE),
            accent: palette.accent.clone(),
            water: palette.water.clone(),
            green: palette.green.clone(),
            ground: palette.ground.clone(),
            muted: palette.muted.clone(),
            ramp,
            face: Face::for_family(&theme.typography.family),
            is_uppercase: theme.typography.uppercase_labels,
        })
    }

    /// The ramp at `position`, 0 the light end and 1 the strong end.
    pub fn ramp_at(&self, position: f64) -> String {
        let last = (self.ramp.len() - 1) as f64;
        let place = position.clamp(0.0, 1.0) * last;
        let lower = place.floor() as usize;
        let upper = (lower + 1).min(self.ramp.len() - 1);
        mix(&self.ramp[lower], &self.ramp[upper], place - lower as f64)
    }

    /// `colour`, or `fallback` when `colour` would vanish into the paper.
    pub fn visible(&self, colour: &str, fallback: &str) -> String {
        if (luminance(colour) - luminance(&self.paper)).abs() < MIN_LUMINANCE_STEP {
            fallback.to_string()
        } else {
            colour.to_string()
        }
    }

    /// `share` of the way from paper to ink: a grey in the theme's own tint.
    pub fn ink_at(&self, share: f64) -> String {
        mix(&self.paper, &self.ink, share)
    }

    pub fn label(&self, text: &str) -> String {
        if self.is_uppercase {
            text.to_uppercase()
        } else {
            text.to_string()
        }
    }
}

/// Wraps a drawing `height_mm` tall into a figure.
pub fn figure(id: &str, height_mm: f64, body: String, paint: &Paint) -> Figure {
    let mut svg = String::with_capacity(body.len() + 400);
    let _ = writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH_MM}mm" height="{height_mm:.1}mm" viewBox="0 0 {WIDTH_MM} {height_mm:.1}" font-family="{}">"#,
        paint.face.css_stack()
    );
    let _ = writeln!(
        svg,
        r#"<rect width="{WIDTH_MM}" height="{height_mm:.1}" fill="{}"/>"#,
        paint.paper
    );
    svg.push_str(&body);
    svg.push_str("</svg>\n");

    Figure {
        id: id.to_string(),
        width_mm: WIDTH_MM,
        height_mm,
        svg,
        body,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn text(
    out: &mut String,
    x: f64,
    y: f64,
    size: f64,
    weight: u16,
    fill: &str,
    anchor: &str,
    content: &str,
) {
    let _ = writeln!(
        out,
        r#"<text x="{x:.2}" y="{y:.2}" font-size="{size}" font-weight="{weight}" fill="{fill}" text-anchor="{anchor}">{}</text>"#,
        escape(content)
    );
}

pub fn line(out: &mut String, from: (f64, f64), to: (f64, f64), stroke: &str, width: f64) {
    let _ = writeln!(
        out,
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{stroke}" stroke-width="{width}" stroke-linecap="round"/>"#,
        from.0, from.1, to.0, to.1
    );
}

pub fn rect(out: &mut String, x: f64, y: f64, width: f64, height: f64, fill: &str) {
    let _ = writeln!(
        out,
        r#"<rect x="{x:.2}" y="{y:.2}" width="{:.2}" height="{:.2}" fill="{fill}"/>"#,
        width.max(0.0),
        height.max(0.0)
    );
}

/// Swatches with labels in rows from `(x, y)`, wrapping at the figure's
/// inset. An item with an empty colour is a plain note without a swatch.
/// Returns the y below the last row.
pub fn legend(out: &mut String, x: f64, y: f64, items: &[(String, String)], paint: &Paint) -> f64 {
    let right = WIDTH_MM - INSET_MM;
    let mut cursor = x;
    let mut baseline = y + SMALL_MM;
    for (label, fill) in items {
        let swatch = if fill.is_empty() {
            0.0
        } else {
            SWATCH_MM + 1.2
        };
        let width = swatch + text_width(label, &paint.face, 400, false, SMALL_MM, 0.0);
        if cursor + width > right && cursor > x {
            cursor = x;
            baseline += LEGEND_ROW_MM;
        }
        if !fill.is_empty() {
            let _ = writeln!(
                out,
                r#"<rect x="{cursor:.2}" y="{:.2}" width="{SWATCH_MM}" height="{SWATCH_MM}" fill="{fill}" stroke="{}" stroke-width="0.1"/>"#,
                baseline - SWATCH_MM + 0.15,
                paint.faint
            );
        }
        text(
            out,
            cursor + swatch,
            baseline,
            SMALL_MM,
            400,
            &paint.secondary,
            "start",
            label,
        );
        cursor += width + LEGEND_GAP_MM;
    }

    baseline + 1.5
}

/// The plot area of a chart with one column per month, and how values map onto it.
pub struct MonthAxes {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub min: f64,
    pub max: f64,
}

impl MonthAxes {
    pub fn column_width(&self) -> f64 {
        (self.right - self.left) / 12.0
    }

    /// The centre of month `index`, 0 for January.
    pub fn x(&self, index: usize) -> f64 {
        self.left + (index as f64 + 0.5) * self.column_width()
    }

    pub fn y(&self, value: f64) -> f64 {
        let share = (value - self.min) / (self.max - self.min);
        self.bottom - share.clamp(0.0, 1.0) * (self.bottom - self.top)
    }

    /// Grid lines at `ticks` labelled with `unit`, and the month initials
    /// below. Returns the y below the month labels.
    pub fn draw(&self, out: &mut String, ticks: &[f64], unit: &str, paint: &Paint) -> f64 {
        for &tick in ticks {
            let y = self.y(tick);
            line(
                out,
                (self.left, y),
                (self.right, y),
                &paint.faint,
                HAIRLINE_MM,
            );
            let label = format!("{tick:.0}{unit}");
            text(
                out,
                self.left - 1.2,
                y + SMALL_MM * 0.35,
                SMALL_MM,
                400,
                &paint.secondary,
                "end",
                &label,
            );
        }
        let label_y = self.bottom + SMALL_MM + 1.4;
        for (index, initial) in MONTH_INITIALS.iter().enumerate() {
            text(
                out,
                self.x(index),
                label_y,
                SMALL_MM,
                400,
                &paint.secondary,
                "middle",
                initial,
            );
        }

        label_y
    }
}

/// 0.184 -> "18 %", 0.004 -> "0.4 %": a small share keeps a decimal so it does not read as none.
pub fn percent(share: f64) -> String {
    if share > 0.0 && share < SMALL_SHARE {
        format!("{:.1} %", share * 100.0)
    } else {
        format!("{:.0} %", share * 100.0)
    }
}

/// Grid values from `min` to `max` every `step`, both ends included when they land on a step.
pub fn ticks(min: f64, max: f64, step: f64) -> Vec<f64> {
    let first = (min / step).ceil() as i64;
    let last = (max / step).floor() as i64;
    (first..=last).map(|index| index as f64 * step).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_land_on_whole_steps_inside_the_range() {
        assert_eq!(ticks(-3.0, 22.0, 5.0), vec![0.0, 5.0, 10.0, 15.0, 20.0]);
        assert_eq!(ticks(0.0, 10.0, 5.0), vec![0.0, 5.0, 10.0]);
    }
}
