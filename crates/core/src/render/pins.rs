//! The project's pins, drawn over the drawing as small markers with labels.
//!
//! A pin sits on the ground: in a plan where it is, in an axonometric at
//! ground height, and in an explode view on the top plate. Each pin is one
//! group carrying its id, icon, label and paper anchor as data attributes, the
//! way sheet text is written, so the app can select, drag and relabel a pin on
//! the very SVG it shows.
//!
//! Pins stay out of the legend. Each one already carries its label where it
//! is, and the legend explains what the data colours mean; mixing hand-placed
//! landmarks into it would blur that.

use std::f64::consts::TAU;
use std::fmt::Write;

use super::geometry::{decimal, mm, push_paper_points, Projection};
use super::svg::{escape, layer_attributes};
use super::typeset::{Kind, Typesetter};
use crate::pin::{is_on_site, number_of, Pin, PinIcon};
use crate::scene::Scene;
use crate::theme::Palette;

// Sizes at A3, in millimetres; multiplied by the layout unit.
/// The map pin's head: its radius and how far its centre stands above the point.
const HEAD_RADIUS: f64 = 2.2;
const HEAD_HEIGHT: f64 = 5.4;
const HEAD_HOLE: f64 = 0.8;
const STAR_OUTER: f64 = 2.8;
const STAR_INNER: f64 = 1.2;
const STAR_POINTS: usize = 5;
const FLAG_HEIGHT: f64 = 6.0;
const FLAG_WIDTH: f64 = 3.9;
const FLAG_DEPTH: f64 = 2.4;
const POLE_MM: f64 = 0.35;
const DOT_RADIUS: f64 = 1.4;
const DISC_RADIUS: f64 = 2.7;
/// The number in a disc, as a share of the disc's radius.
const NUMBER_SIZE: f64 = 1.15;
/// The paper-coloured edge that keeps a marker apart from what lies under it.
const HALO_MM: f64 = 0.4;
const LABEL_HALO_MM: f64 = 0.6;
const LABEL_GAP: f64 = 1.4;
/// How far apart the ground points the app places pins by are, in metres.
const REFERENCE_M: f64 = 1000.0;

pub struct PinLayer<'a> {
    pub pins: &'a [Pin],
    /// The scene as the sheet frames it. Pins outside its boundary are left
    /// off the sheet: there is nothing drawn under them.
    pub scene: &'a Scene,
    /// The ground plane on paper.
    pub ground: Projection,
    pub palette: &'a Palette,
    pub typesetter: &'a Typesetter,
}

pub fn write(out: &mut String, layer: &PinLayer) {
    let shown: Vec<(&Pin, (f64, f64))> = layer
        .pins
        .iter()
        .filter(|pin| is_on_site(layer.scene, pin.x, pin.y))
        .map(|pin| {
            let (origin_x, origin_y) = layer.scene.origin;
            (
                pin,
                layer.ground.to_paper(pin.x - origin_x, pin.y - origin_y),
            )
        })
        .collect();
    if shown.is_empty() {
        return;
    }

    let _ = write!(
        out,
        r#"<g {} font-family="{}">"#,
        layer_attributes("pins", "Pins"),
        escape(layer.typesetter.face.css_stack())
    );
    for (pin, at) in shown {
        write_pin(out, layer, pin, at);
    }
    out.push_str("</g>\n");
}

fn write_pin(out: &mut String, layer: &PinLayer, pin: &Pin, (x, y): (f64, f64)) {
    let unit = layer.typesetter.unit;
    let palette = layer.palette;
    let icon = match pin.icon {
        PinIcon::Pin => "pin",
        PinIcon::Star => "star",
        PinIcon::Flag => "flag",
        PinIcon::Dot => "dot",
        PinIcon::Number => "number",
    };
    let _ = write!(
        out,
        r#"<g data-pin="{}" data-icon="{icon}" data-label="{}" data-x="{}" data-y="{}">"#,
        escape(&pin.id),
        escape(&pin.label),
        mm(x),
        mm(y)
    );
    let halo = format!(
        r#" stroke="{}" stroke-width="{}" stroke-linejoin="round""#,
        palette.paper,
        mm(HALO_MM * unit)
    );
    let accent = &palette.accent;
    // Where the label's left edge and middle go, beside the marker's widest part.
    let (reach, middle) = match pin.icon {
        PinIcon::Pin => {
            let head = y - HEAD_HEIGHT * unit;
            out.push_str(&teardrop(
                (x, y),
                unit,
                &format!(r#" fill="{accent}"{halo}"#),
            ));
            let _ = write!(
                out,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"#,
                mm(x),
                mm(head),
                mm(HEAD_HOLE * unit),
                palette.paper
            );
            (HEAD_RADIUS * unit, head)
        }
        PinIcon::Star => {
            out.push_str(&star((x, y), unit, &format!(r#" fill="{accent}"{halo}"#)));
            (STAR_OUTER * unit, y)
        }
        PinIcon::Flag => {
            write_flag(out, (x, y), unit, &palette.ink, accent, &halo);
            (
                FLAG_WIDTH * unit,
                y - (FLAG_HEIGHT - FLAG_DEPTH / 2.0) * unit,
            )
        }
        PinIcon::Dot => {
            let _ = write!(
                out,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{accent}"{halo}/>"#,
                mm(x),
                mm(y),
                mm(DOT_RADIUS * unit)
            );
            (DOT_RADIUS * unit, y)
        }
        PinIcon::Number => {
            let number = number_of(layer.pins, &pin.id).unwrap_or(0);
            write_disc(out, (x, y), unit, number, palette, &halo);
            (DISC_RADIUS * unit, y)
        }
    };
    if !pin.label.is_empty() {
        write_label(out, layer, &pin.label, x + reach + LABEL_GAP * unit, middle);
    }
    out.push_str("</g>");
}

/// The classic map pin: a round head on a point, the point exactly at the place.
fn teardrop((x, y): (f64, f64), unit: f64, attributes: &str) -> String {
    let radius = HEAD_RADIUS * unit;
    let height = HEAD_HEIGHT * unit;
    // The sides run from the point to where they touch the head's circle.
    let touch = (radius / height).acos();
    let side_x = radius * touch.sin();
    let side_y = y - height + radius * touch.cos();
    format!(
        r#"<path d="M{} {}L{} {}A{r} {r} 0 1 1 {} {}Z"{attributes}/>"#,
        mm(x),
        mm(y),
        mm(x - side_x),
        mm(side_y),
        mm(x + side_x),
        mm(side_y),
        r = mm(radius)
    )
}

fn star((x, y): (f64, f64), unit: f64, attributes: &str) -> String {
    let corners: Vec<(f64, f64)> = (0..STAR_POINTS * 2)
        .map(|index| {
            let reach = if index % 2 == 0 {
                STAR_OUTER
            } else {
                STAR_INNER
            } * unit;
            let angle = TAU * index as f64 / (STAR_POINTS * 2) as f64;
            (x + reach * angle.sin(), y - reach * angle.cos())
        })
        .collect();
    let mut data = String::new();
    push_paper_points(&mut data, &corners, true);
    format!(r#"<path d="{data}"{attributes}/>"#)
}

/// A pole in ink standing on the place, with a pennant in the accent colour.
fn write_flag(
    out: &mut String,
    (x, y): (f64, f64),
    unit: f64,
    ink: &str,
    accent: &str,
    halo: &str,
) {
    let top = y - FLAG_HEIGHT * unit;
    let pennant = [
        (x, top),
        (x + FLAG_WIDTH * unit, top + FLAG_DEPTH * unit / 2.0),
        (x, top + FLAG_DEPTH * unit),
    ];
    let mut data = String::new();
    push_paper_points(&mut data, &pennant, true);
    let _ = write!(
        out,
        r#"<path d="M{} {}V{}" stroke="{ink}" stroke-width="{}" stroke-linecap="round"/><path d="{data}" fill="{accent}"{halo}/>"#,
        mm(x),
        mm(y),
        mm(top),
        mm(POLE_MM * unit)
    );
}

fn write_disc(
    out: &mut String,
    (x, y): (f64, f64),
    unit: f64,
    number: usize,
    palette: &Palette,
    halo: &str,
) {
    let radius = DISC_RADIUS * unit;
    let size = radius * NUMBER_SIZE;
    let _ = write!(
        out,
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}"{halo}/><text x="{}" y="{}" font-size="{}" font-weight="600" fill="{}" text-anchor="middle">{number}</text>"#,
        mm(x),
        mm(y),
        mm(radius),
        palette.accent,
        mm(x),
        // The digits' cap height is about 0.72 of the size; this centres them.
        mm(y + size * 0.36),
        mm(size),
        palette.paper
    );
}

/// The label in ink on a paper halo, so it reads over any fill.
fn write_label(out: &mut String, layer: &PinLayer, label: &str, x: f64, middle: f64) {
    let typesetter = layer.typesetter;
    let baseline = middle + typesetter.cap_height(Kind::PinLabel) / 2.0;
    let _ = write!(
        out,
        r#"<g stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke">"#,
        layer.palette.paper,
        mm(LABEL_HALO_MM * typesetter.unit)
    );
    typesetter.write(out, x, baseline, label, Kind::PinLabel, None);
    out.push_str("</g>");
}

/// Three ground points far apart, each as "paper x, paper y, RD x, RD y",
/// for the app to turn a dropped pin into RD New metres. The ground plane
/// maps onto the paper linearly in every view, so three points fix it, and
/// at a kilometre apart the hundredth-millimetre rounding costs millimetres.
pub fn ground_points(ground: &Projection, origin: (f64, f64)) -> String {
    let local = [(0.0, 0.0), (REFERENCE_M, 0.0), (0.0, REFERENCE_M)];
    let mut parts = Vec::new();
    for (x, y) in local {
        let (paper_x, paper_y) = ground.to_paper(x, y);
        parts.push(mm(paper_x));
        parts.push(mm(paper_y));
        parts.push(decimal(x + origin.0));
        parts.push(decimal(y + origin.1));
    }

    parts.join(" ")
}
