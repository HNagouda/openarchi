//! The measuring marks of a section: a level ruler beside it, the NAP datum
//! across it, the height of each building it cuts, and the A and A' ends.

use std::fmt::Write;

use super::geometry::{decimal, mm};
use super::section_draw::Cut;
use super::typeset::{Kind, Typesetter};
use crate::section::profile::Profile;
use crate::theme::Theme;

// Sizes at A3, in millimetres; multiplied by the layout unit.
/// Room beside the section for the ruler and its numbers, on each side.
const SIDE_ROOM: f64 = 16.0;
const RULER_GAP: f64 = 4.0;
const TICK: f64 = 1.2;
const TICK_LABEL_GAP: f64 = 0.8;
/// Ruler marks stand at least this far apart on the paper.
const MIN_TICK_SPACING: f64 = 5.0;
const END_LABEL_GAP: f64 = 2.4;
const HEIGHT_LABEL_GAP: f64 = 1.0;
const DATUM_LABEL_GAP: f64 = 1.0;
const LINE_MM: f64 = 0.15;
const DATUM_OPACITY: f64 = 0.45;
const DATUM_DASH: (f64, f64) = (1.2, 0.8);
const HALO_MM: f64 = 0.6;
/// Rounding on the paper that still counts an end as inside the box.
const EDGE_SLACK_MM: f64 = 0.01;
/// Steps between ruler marks, in metres: round numbers to read heights off.
const STEPS_M: [f64; 8] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0];

/// Where the section lies on the paper: the paper x of its ends, the box of
/// it that shows, and the height at that box's top edge. At a scale too large
/// for the sheet the ends lie outside the box, and the drawing is clipped to it.
pub struct Frame {
    /// Paper x of A and of A'.
    pub start_x: f64,
    pub end_x: f64,
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    /// The height, in metres, the top edge stands for.
    pub highest: f64,
    /// Paper per metre along the line.
    pub mm_per_m: f64,
    /// Paper per metre of height: `mm_per_m` times the exaggeration.
    pub vertical_mm_per_m: f64,
}

impl Frame {
    /// Paper x of a point `along` metres from A.
    pub fn x(&self, along: f64) -> f64 {
        self.start_x + along * self.mm_per_m
    }

    /// Paper y of a height in metres.
    pub fn y(&self, height: f64) -> f64 {
        self.top + (self.highest - height) * self.vertical_mm_per_m
    }

    fn lowest(&self) -> f64 {
        self.highest - (self.bottom - self.top) / self.vertical_mm_per_m
    }
}

/// The paper kept free on each side of a section for the ruler, in millimetres.
pub fn side_room(typesetter: &Typesetter) -> f64 {
    SIDE_ROOM * typesetter.unit
}

pub fn write(
    out: &mut String,
    frame: &Frame,
    profile: &Profile,
    cuts: &[Cut],
    theme: &Theme,
    typesetter: &Typesetter,
) {
    let ink = &theme.palette.ink;
    let _ = write!(
        out,
        r#"<g id="section-levels" stroke="{ink}" stroke-width="{}" fill="none">"#,
        mm(LINE_MM * typesetter.unit)
    );
    if profile.is_measured {
        write_datum(out, frame, typesetter);
    }
    write_ruler(out, frame, profile.is_measured, typesetter);
    out.push_str("</g>\n");

    let _ = write!(
        out,
        r#"<g id="section-labels" stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke">"#,
        theme.palette.paper,
        mm(HALO_MM * typesetter.unit)
    );
    write_heights(out, frame, cuts, typesetter);
    let baseline =
        frame.bottom + END_LABEL_GAP * typesetter.unit + typesetter.cap_height(Kind::Heading);
    // An end outside the box is off the sheet, and so is its label.
    if frame.start_x >= frame.left - EDGE_SLACK_MM {
        typesetter.write(out, frame.start_x, baseline, "A", Kind::Heading, None);
    }
    if frame.end_x <= frame.right + EDGE_SLACK_MM {
        typesetter.write(
            out,
            frame.end_x,
            baseline,
            "A\u{2032}",
            Kind::Heading,
            Some("end"),
        );
    }
    out.push_str("</g>\n");
}

/// A dashed line across the section at NAP 0, when the section reaches it.
fn write_datum(out: &mut String, frame: &Frame, typesetter: &Typesetter) {
    if !(frame.lowest()..=frame.highest).contains(&0.0) {
        return;
    }
    let unit = typesetter.unit;
    let y = frame.y(0.0);
    let _ = write!(
        out,
        r#"<path d="M{} {y}H{}" stroke-opacity="{}" stroke-dasharray="{} {}"/>"#,
        mm(frame.left),
        mm(frame.right),
        decimal(DATUM_OPACITY),
        mm(DATUM_DASH.0 * unit),
        mm(DATUM_DASH.1 * unit),
        y = mm(y)
    );
    let mut label = String::new();
    let baseline = y + typesetter.cap_height(Kind::Small) / 2.0;
    typesetter.write(
        &mut label,
        frame.right + DATUM_LABEL_GAP * unit,
        baseline,
        "NAP",
        Kind::Small,
        None,
    );
    let _ = write!(out, r#"<g stroke="none">{label}</g>"#);
}

/// A vertical ruler left of the section with a mark every round step of height.
fn write_ruler(out: &mut String, frame: &Frame, is_measured: bool, typesetter: &Typesetter) {
    let unit = typesetter.unit;
    let step = STEPS_M
        .iter()
        .copied()
        .find(|step| step * frame.vertical_mm_per_m >= MIN_TICK_SPACING * unit)
        .unwrap_or(STEPS_M[STEPS_M.len() - 1]);
    let first = (frame.lowest() / step).ceil() as i64;
    let last = (frame.highest / step).floor() as i64;
    if last <= first {
        return;
    }
    let x = frame.left - RULER_GAP * unit;
    let _ = write!(
        out,
        r#"<path d="M{} {}V{}"/>"#,
        mm(x),
        mm(frame.y(first as f64 * step)),
        mm(frame.y(last as f64 * step))
    );

    let mut labels = String::new();
    let label_x = x - (TICK + TICK_LABEL_GAP) * unit;
    let half_cap = typesetter.cap_height(Kind::Small) / 2.0;
    for mark in first..=last {
        let height = mark as f64 * step;
        let y = frame.y(height);
        let _ = write!(
            out,
            r#"<path d="M{} {}H{}"/>"#,
            mm(x - TICK * unit),
            mm(y),
            mm(x)
        );
        typesetter.write(
            &mut labels,
            label_x,
            y + half_cap,
            &level(height),
            Kind::Small,
            Some("end"),
        );
    }
    // Above the ruler: what its numbers measure.
    let unit_label = if is_measured { "m NAP" } else { "m" };
    let caption_y = frame.y(last as f64 * step) - 2.0 * half_cap - TICK_LABEL_GAP * unit;
    typesetter.write(
        &mut labels,
        x,
        caption_y,
        unit_label,
        Kind::Small,
        Some("end"),
    );
    let _ = write!(out, r#"<g stroke="none">{labels}</g>"#);
}

/// Each cut building's measured height above its ground, over its roof and
/// any taller cut roof beside it, skipping a label that would run into the
/// one before it.
fn write_heights(out: &mut String, frame: &Frame, cuts: &[Cut], typesetter: &Typesetter) {
    let mut sorted: Vec<&Cut> = cuts.iter().filter(|cut| cut.height_m.is_some()).collect();
    sorted.sort_by(|a, b| a.stretch.from.total_cmp(&b.stretch.from));
    let mut free_from = f64::NEG_INFINITY;
    for cut in sorted {
        let Some(height) = cut.height_m else {
            continue;
        };
        let text = format!("{height:.1} m");
        let width = typesetter.width(&text, Kind::Small);
        let middle = frame.x((cut.stretch.from + cut.stretch.to) / 2.0);
        let is_off_the_sheet = middle < frame.left || middle > frame.right;
        if is_off_the_sheet || middle - width / 2.0 < free_from {
            continue;
        }
        let (left, right) = (middle - width / 2.0, middle + width / 2.0);
        let roof = cuts
            .iter()
            .filter(|other| frame.x(other.stretch.from) < right && frame.x(other.stretch.to) > left)
            .map(|other| other.top)
            .fold(cut.top, f64::max);
        let baseline = frame.y(roof) - HEIGHT_LABEL_GAP * typesetter.unit;
        typesetter.write(out, middle, baseline, &text, Kind::Small, Some("middle"));
        free_from = middle + width / 2.0 + HEIGHT_LABEL_GAP * typesetter.unit;
    }
}

/// "+7.95", "−0.40": a level to the centimetre, as a water level is marked.
pub fn precise_level(height: f64) -> String {
    if height > 0.0 {
        format!("+{height:.2}")
    } else if height < 0.0 {
        format!("\u{2212}{:.2}", -height)
    } else {
        "0.00".to_string()
    }
}

/// "+5", "0", "−2": a level as a section marks it.
fn level(height: f64) -> String {
    if height > 0.0 {
        format!("+{height:.0}")
    } else if height < 0.0 {
        format!("\u{2212}{:.0}", -height)
    } else {
        "0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_signed_like_a_drawing_marks_them() {
        assert_eq!(level(5.0), "+5");
        assert_eq!(level(0.0), "0");
        assert_eq!(level(-2.0), "\u{2212}2");
    }

    #[test]
    fn heights_map_onto_the_paper_with_the_top_edge_highest() {
        let frame = Frame {
            start_x: 10.0,
            end_x: 60.0,
            left: 10.0,
            right: 110.0,
            top: 20.0,
            bottom: 60.0,
            highest: 30.0,
            mm_per_m: 1.0,
            vertical_mm_per_m: 2.0,
        };
        assert_eq!(frame.y(30.0), 20.0);
        assert_eq!(frame.y(10.0), 60.0);
        assert_eq!(frame.lowest(), 10.0);
        assert_eq!(frame.x(50.0), 60.0);
    }
}
