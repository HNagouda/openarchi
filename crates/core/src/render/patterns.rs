//! Hatches and dot screens as SVG `<pattern>` tiles.
//!
//! The layer's flat fill is painted into the tile behind the marks, so a
//! patterned layer is one fill on one set of paths instead of two stacked copies.

use super::geometry::{decimal, mm};
use crate::theme::Pattern;

/// The `<pattern>` element for `pattern`, drawn over `background` when the layer has a fill.
pub fn pattern_def(id: &str, pattern: &Pattern, background: Option<&str>) -> String {
    let (spacing, angle, marks) = match pattern {
        Pattern::Hatch {
            angle_deg,
            spacing_mm,
            stroke_mm,
            color,
        } => {
            let half = spacing_mm / 2.0;
            let line = horizontal_line(*spacing_mm, half, *stroke_mm, color);
            (*spacing_mm, *angle_deg, line)
        }
        Pattern::CrossHatch {
            angle_deg,
            spacing_mm,
            stroke_mm,
            color,
        } => {
            let half = spacing_mm / 2.0;
            let lines = format!(
                r#"{}<path d="M{h} 0V{s}" stroke="{color}" stroke-width="{w}"/>"#,
                horizontal_line(*spacing_mm, half, *stroke_mm, color),
                h = mm(half),
                s = mm(*spacing_mm),
                w = mm(*stroke_mm),
            );
            (*spacing_mm, *angle_deg, lines)
        }
        Pattern::Dots {
            spacing_mm,
            radius_mm,
            color,
        } => {
            // A staggered screen: one dot in the centre and a quarter in each corner.
            let mut dots = String::new();
            for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.5, 0.5)] {
                dots.push_str(&format!(
                    r#"<circle cx="{}" cy="{}" r="{}" fill="{color}"/>"#,
                    mm(x * spacing_mm),
                    mm(y * spacing_mm),
                    mm(*radius_mm)
                ));
            }
            (*spacing_mm, 0.0, dots)
        }
    };
    let size = mm(spacing);
    let background_rect = match background {
        Some(fill) => format!(r#"<rect width="{size}" height="{size}" fill="{fill}"/>"#),
        None => String::new(),
    };
    let transform = if angle == 0.0 {
        String::new()
    } else {
        format!(r#" patternTransform="rotate({})""#, decimal(angle))
    };

    format!(
        r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{size}" height="{size}"{transform}>{background_rect}{marks}</pattern>"#
    )
}

fn horizontal_line(spacing: f64, y: f64, stroke: f64, color: &str) -> String {
    format!(
        r#"<path d="M0 {y}H{s}" stroke="{color}" stroke-width="{w}"/>"#,
        y = mm(y),
        s = mm(spacing),
        w = mm(stroke)
    )
}
