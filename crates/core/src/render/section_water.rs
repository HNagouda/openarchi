//! The waters a section crosses: a body of water from its surface down to the
//! surveyed bed, or fading out downwards where nobody surveyed the bed, with
//! the surface as a line and, at a published target level, the usual level
//! mark. What each water is made of is worked out in `section/water.rs`.

use std::fmt::Write;

use super::color;
use super::geometry::{decimal, mm};
use super::section_levels::{self, Frame};
use super::section_pieces;
use super::svg::layer_attributes;
use super::typeset::{Kind, Typesetter};
use crate::section::profile::Profile;
use crate::section::water::Water;
use crate::theme::Palette;

pub const UNKNOWN_DEPTH_ID: &str = "section-water-depth-unknown";
/// The surface line: the water colour moved this far towards ink.
const SURFACE_SHADE: f64 = 0.45;
const SURFACE_MM: f64 = 0.3;
/// The level mark, an open triangle standing on the surface, in millimetres at A3.
const MARK_MM: f64 = 1.6;
const MARK_LABEL_GAP: f64 = 0.8;
/// The level's baseline sits this share of the mark's size above the surface.
const MARK_LABEL_RISE: f64 = 0.15;
/// Where nobody surveyed the bed the water fades to this opacity at the bottom of the frame.
const UNKNOWN_DEPTH_END_OPACITY: f64 = 0.0;

/// The fade a water without a surveyed bed is filled with, for it and its legend row.
pub fn unknown_depth_gradient(palette: &Palette) -> String {
    format!(
        r#"<linearGradient id="{UNKNOWN_DEPTH_ID}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{water}"/><stop offset="1" stop-color="{water}" stop-opacity="{}"/></linearGradient>"#,
        decimal(UNKNOWN_DEPTH_END_OPACITY),
        water = palette.water
    )
}

pub fn surface_colour(palette: &Palette) -> String {
    color::mix(&palette.water, &palette.ink, SURFACE_SHADE)
}

pub fn write(
    out: &mut String,
    waters: &[Water],
    profile: &Profile,
    frame: &Frame,
    palette: &Palette,
    typesetter: &Typesetter,
) {
    if waters.is_empty() {
        return;
    }
    let unit = typesetter.unit;
    let surface = surface_colour(palette);
    let _ = write!(out, "<g {}>", layer_attributes("section-water", "Water"));
    for water in waters {
        let (left, right) = (frame.x(water.stretch.from), frame.x(water.stretch.to));
        let top = frame.y(water.level_m);
        if water.has_bed {
            let _ = write!(
                out,
                r#"<path d="{}" fill="{}"/>"#,
                body_down_to_bed(water, profile, frame),
                palette.water
            );
        } else {
            let _ = write!(
                out,
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="url(#{UNKNOWN_DEPTH_ID})"/>"#,
                mm(left),
                mm(top),
                mm(right - left),
                mm((frame.bottom - top).max(0.0))
            );
        }
        let _ = write!(
            out,
            r#"<path d="M{} {top}H{}" fill="none" stroke="{surface}" stroke-width="{}"/>"#,
            mm(left),
            mm(right),
            mm(SURFACE_MM * unit),
            top = mm(top)
        );
        if water.level_name.is_some() {
            write_level_mark(
                out,
                (left + right) / 2.0,
                top,
                water.level_m,
                &surface,
                typesetter,
            );
        }
    }
    out.push_str("</g>\n");
}

/// `profile` with the ground over each water without a surveyed bed lowered
/// to the water's surface, dropping straight down at the banks. Bridged from
/// bank to bank, it would otherwise stand above the water as solid earth.
pub fn lowered(profile: Profile, waters: &[Water]) -> Profile {
    let gaps: Vec<&Water> = waters.iter().filter(|water| !water.has_bed).collect();
    if gaps.is_empty() {
        return profile;
    }
    let mut samples = Vec::with_capacity(profile.samples.len() + 4 * gaps.len());
    for &(along, height) in &profile.samples {
        let inside = gaps
            .iter()
            .find(|water| along > water.stretch.from && along < water.stretch.to);
        match inside {
            Some(water) => samples.push((along, height.min(water.level_m))),
            None => samples.push((along, height)),
        }
    }
    for water in gaps {
        let (from, to) = (water.stretch.from, water.stretch.to);
        let level = water.level_m;
        samples.push((from, profile.height_at(from)));
        samples.push((from, profile.height_at(from).min(level)));
        samples.push((to, profile.height_at(to).min(level)));
        samples.push((to, profile.height_at(to)));
    }
    // Stable, so the bank stays before the drop at the near end and after it at the far end.
    samples.sort_by(|a, b| a.0.total_cmp(&b.0));

    Profile { samples, ..profile }
}

/// From the surface along A' to A, back along the bed below it.
fn body_down_to_bed(water: &Water, profile: &Profile, frame: &Frame) -> String {
    let body = section_pieces::water_body(water, profile);
    let (start, along_surface) = (body[0], body[1]);
    let mut path = format!(
        "M{} {}H{}",
        mm(frame.x(start.0)),
        mm(frame.y(start.1)),
        mm(frame.x(along_surface.0))
    );
    for &(along, height) in &body[2..] {
        let _ = write!(path, "L{} {}", mm(frame.x(along)), mm(frame.y(height)));
    }
    path.push('Z');

    path
}

/// An open triangle standing on its point at the surface, and the level beside it: "▽ −0.4".
fn write_level_mark(
    out: &mut String,
    x: f64,
    surface_y: f64,
    level_m: f64,
    colour: &str,
    typesetter: &Typesetter,
) {
    let size = MARK_MM * typesetter.unit;
    let _ = write!(
        out,
        r#"<path d="M{} {}L{} {}L{} {}Z" fill="none" stroke="{colour}" stroke-width="{}" stroke-linejoin="round"/>"#,
        mm(x),
        mm(surface_y),
        mm(x - size / 2.0),
        mm(surface_y - size),
        mm(x + size / 2.0),
        mm(surface_y - size),
        mm(SURFACE_MM * typesetter.unit)
    );
    let mut label = String::new();
    typesetter.write(
        &mut label,
        x + size / 2.0 + MARK_LABEL_GAP * typesetter.unit,
        surface_y - size * MARK_LABEL_RISE,
        &section_levels::precise_level(level_m),
        Kind::Small,
        None,
    );
    let _ = write!(out, r#"<g stroke="none" fill="{colour}">{label}</g>"#);
}
