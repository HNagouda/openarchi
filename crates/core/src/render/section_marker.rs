//! A sheet's section line on its plan or tilted view, as the usual cut
//! marker: a long-dash line, a heavy tick at each end, arrows showing which
//! way the section looks, and the labels A and A'. It is drawn in the
//! palette's one accent on a paper edge, so it reads over a dark figure-ground
//! as well as over open ground.
//!
//! It is one group carrying the paper position of both ends in data
//! attributes, as pins do, so the app can pick up an end and drag it on the
//! very SVG it shows.

use std::fmt::Write;

use super::geometry::{mm, Projection};
use super::typeset::{Kind, Typesetter};
use crate::scene::Scene;
use crate::section::SectionLine;
use crate::theme::Palette;

// Sizes at A3, in millimetres; multiplied by the layout unit.
const LINE_MM: f64 = 0.3;
const HALO_MM: f64 = 0.9;
/// Long dash, gap, dot, gap: the centre line of a cut.
pub const DASHES: [f64; 4] = [6.0, 1.2, 0.8, 1.2];
pub const TICK_LENGTH: f64 = 5.0;
const TICK_MM: f64 = 1.0;
pub const ARROW_LENGTH: f64 = 4.4;
pub const ARROW_HALF_WIDTH: f64 = 1.9;
/// The paper edge around ticks and arrows, so they read on a dark figure too.
const MARK_HALO_MM: f64 = 0.7;
pub const LABEL_GAP: f64 = 3.2;
const LABEL_HALO_MM: f64 = 0.8;
/// A ground step long enough that its paper direction is exact to the hundredth.
const PROBE_M: f64 = 10.0;

pub fn write(
    out: &mut String,
    line: &SectionLine,
    scene: &Scene,
    ground: &Projection,
    palette: &Palette,
    typesetter: &Typesetter,
) {
    let unit = typesetter.unit;
    let axis = line.axis(scene.origin);
    let start = ground.to_paper(axis.start.0, axis.start.1);
    let end_ground = axis.point_at(axis.length, 0.0);
    let end = ground.to_paper(end_ground.0, end_ground.1);
    let along = unit_vector(start, end);
    let probe = axis.point_at(0.0, PROBE_M);
    let looking = unit_vector(start, ground.to_paper(probe.0, probe.1));

    let _ = write!(
        out,
        r#"<g id="section-line" data-section="A" data-start-x="{}" data-start-y="{}" data-end-x="{}" data-end-y="{}" fill="none" stroke-linecap="butt">"#,
        mm(start.0),
        mm(start.1),
        mm(end.0),
        mm(end.1)
    );
    let path = format!(
        "M{} {}L{} {}",
        mm(start.0),
        mm(start.1),
        mm(end.0),
        mm(end.1)
    );
    let dashes: Vec<String> = DASHES.iter().map(|dash| mm(dash * unit)).collect();
    let _ = write!(
        out,
        r#"<path d="{path}" stroke="{}" stroke-width="{}" stroke-opacity="0.8"/><path d="{path}" stroke="{}" stroke-width="{}" stroke-dasharray="{}"/>"#,
        palette.paper,
        mm(HALO_MM * unit),
        palette.accent,
        mm(LINE_MM * unit),
        dashes.join(" ")
    );
    write_end(out, start, along, looking, palette, unit);
    write_end(out, end, (-along.0, -along.1), looking, palette, unit);

    let _ = write!(
        out,
        r#"<g stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke">"#,
        palette.paper,
        mm(LABEL_HALO_MM * unit)
    );
    let half_cap = typesetter.cap_height(Kind::PinLabel) / 2.0;
    let gap = LABEL_GAP * unit;
    let a_at = (start.0 - along.0 * gap, start.1 - along.1 * gap + half_cap);
    let b_at = (end.0 + along.0 * gap, end.1 + along.1 * gap + half_cap);
    typesetter.write(out, a_at.0, a_at.1, "A", Kind::PinLabel, Some("middle"));
    typesetter.write(
        out,
        b_at.0,
        b_at.1,
        "A\u{2032}",
        Kind::PinLabel,
        Some("middle"),
    );
    out.push_str("</g></g>\n");
}

/// The heavy tick running inwards from one end, and the arrow pointing where the section looks.
fn write_end(
    out: &mut String,
    at: (f64, f64),
    inwards: (f64, f64),
    looking: (f64, f64),
    palette: &Palette,
    unit: f64,
) {
    let tick = TICK_LENGTH * unit;
    let tip = (
        at.0 + looking.0 * ARROW_LENGTH * unit,
        at.1 + looking.1 * ARROW_LENGTH * unit,
    );
    let half = ARROW_HALF_WIDTH * unit;
    let tick_path = format!(
        "M{} {}L{} {}",
        mm(at.0),
        mm(at.1),
        mm(at.0 + inwards.0 * tick),
        mm(at.1 + inwards.1 * tick)
    );
    let arrow_path = format!(
        "M{} {}L{} {}L{} {}Z",
        mm(at.0 + inwards.0 * half),
        mm(at.1 + inwards.1 * half),
        mm(tip.0),
        mm(tip.1),
        mm(at.0 - inwards.0 * half),
        mm(at.1 - inwards.1 * half)
    );
    let halo = MARK_HALO_MM * unit;
    let _ = write!(
        out,
        r#"<path d="{tick_path}" stroke="{paper}" stroke-width="{}"/><path d="{arrow_path}" fill="{paper}" stroke="{paper}" stroke-width="{}" stroke-linejoin="round"/><path d="{tick_path}" stroke="{ink}" stroke-width="{}"/><path d="{arrow_path}" fill="{ink}" stroke="none"/>"#,
        mm(TICK_MM * unit + 2.0 * halo),
        mm(2.0 * halo),
        mm(TICK_MM * unit),
        paper = palette.paper,
        ink = palette.accent
    );
}

fn unit_vector(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy).max(f64::EPSILON);
    (dx / length, dy / length)
}
