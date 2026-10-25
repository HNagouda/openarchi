//! How a section's parts are painted: what stands beyond the line, fading
//! with distance; the earth below the ground profile, hatched; and the
//! buildings the line cuts, in solid poché. Where each part goes is worked
//! out in `section.rs`.

use std::fmt::Write;

use super::color;
use super::geometry::{decimal, mm};
use super::patterns;
use super::section_levels::Frame;
use super::solid;
use super::svg::layer_attributes;
use crate::section::cut::Stretch;
use crate::section::profile::Profile;
use crate::theme::{Palette, Pattern, Theme};

/// What stands beyond the line moves towards the paper: this far right at
/// the line, so it never reads as cut, and this much more at the far edge.
const NEAR_FADE: f64 = 0.25;
const DEPTH_FADE: f64 = 0.45;
/// The earth below the profile: the ground colour moved this far towards ink, hatched.
const EARTH_SHADE: f64 = 0.14;
const HATCH_SHADE: f64 = 0.5;
const HATCH_ANGLE_DEG: f64 = 45.0;
const HATCH_SPACING_MM: f64 = 1.1;
const HATCH_STROKE_MM: f64 = 0.1;
const PROFILE_STROKE_MM: f64 = 0.45;
const EDGE_STROKE_MM: f64 = 0.1;
const EDGE_OPACITY: f64 = 0.35;
const TRUNK_MM: f64 = 0.18;
pub const HATCH_ID: &str = "section-earth";

/// Something standing beyond the line, as it is drawn.
pub(super) struct Mass {
    pub stretch: Stretch,
    pub depth: f64,
    pub base: f64,
    pub top: f64,
    pub fill: String,
    pub is_tree: bool,
}

/// A stretch of a building the line passes through.
pub(super) struct Cut {
    pub stretch: Stretch,
    pub bottom: f64,
    pub top: f64,
    /// The measured height above the ground, for its label; `None` when guessed.
    pub height_m: Option<f64>,
}

/// `fill` as it stands beyond the line, `share` of the way to the far edge (0 to 1).
pub fn beyond_fill(fill: &str, paper: &str, share: f64) -> String {
    color::mix(fill, paper, NEAR_FADE + DEPTH_FADE * share)
}

/// The cut's solid poché: the palette's ink, its strongest colour against the
/// paper. The figure colour would match the buildings beyond in a palette
/// whose buildings are not lightened for standing up.
pub fn poche(palette: &Palette) -> &str {
    &palette.ink
}

/// The hatched earth as a `<pattern>`, for the ground and its legend row.
pub fn earth_pattern(palette: &Palette, unit: f64) -> String {
    let earth = color::mix(&palette.ground, &palette.ink, EARTH_SHADE);
    let hatch = Pattern::Hatch {
        angle_deg: HATCH_ANGLE_DEG,
        spacing_mm: HATCH_SPACING_MM * unit,
        stroke_mm: HATCH_STROKE_MM * unit,
        color: color::mix(&earth, &palette.ink, HATCH_SHADE),
    };
    patterns::pattern_def(HATCH_ID, &hatch, Some(&earth))
}

pub fn write_masses(out: &mut String, masses: &[Mass], depth_m: f64, frame: &Frame, theme: &Theme) {
    let paper = &theme.palette.paper;
    let _ = write!(
        out,
        r#"<g {} stroke="{}" stroke-width="{}" stroke-opacity="{}">"#,
        layer_attributes("section-beyond", "Beyond the section"),
        theme.palette.ink,
        mm(EDGE_STROKE_MM),
        decimal(EDGE_OPACITY)
    );
    for mass in masses {
        let fill = beyond_fill(&mass.fill, paper, mass.depth / depth_m);
        if mass.is_tree {
            write_tree(out, mass, &fill, frame);
            continue;
        }
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}"/>"#,
            mm(frame.x(mass.stretch.from)),
            mm(frame.y(mass.top)),
            mm((mass.stretch.to - mass.stretch.from) * frame.mm_per_m),
            mm((mass.top - mass.base) * frame.vertical_mm_per_m)
        );
    }
    out.push_str("</g>\n");
}

/// A trunk and a round crown, the tree of the axonometric seen from the side.
fn write_tree(out: &mut String, mass: &Mass, fill: &str, frame: &Frame) {
    let (height, crown) = solid::tree_size();
    let x = frame.x(mass.stretch.from);
    let crown_centre = mass.base + height - crown;
    let _ = write!(
        out,
        r#"<path d="M{x} {}V{}" stroke-width="{}" stroke-opacity="1"/><ellipse cx="{x}" cy="{}" rx="{}" ry="{}" fill="{fill}"/>"#,
        mm(frame.y(mass.base)),
        mm(frame.y(crown_centre)),
        mm(TRUNK_MM),
        mm(frame.y(crown_centre)),
        mm(crown * frame.mm_per_m),
        mm(crown * frame.vertical_mm_per_m),
        x = mm(x)
    );
}

/// The earth below the profile, hatched, and the profile itself as the
/// heaviest line, broken over `gaps`: waters whose bed nobody surveyed, where
/// the profile is only a bridge between the banks.
pub fn write_ground(
    out: &mut String,
    profile: &Profile,
    gaps: &[Stretch],
    frame: &Frame,
    theme: &Theme,
    unit: f64,
) {
    let mut outline = String::new();
    for (index, &(along, height)) in profile.samples.iter().enumerate() {
        let command = if index == 0 { 'M' } else { 'L' };
        let _ = write!(
            outline,
            "{command}{} {}",
            mm(frame.x(along)),
            mm(frame.y(height))
        );
    }
    let _ = write!(
        out,
        r#"<g {}><path d="{outline}V{}H{}Z" fill="url(#{HATCH_ID})"/><path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"/></g>"#,
        layer_attributes("section-ground", "Ground"),
        mm(frame.bottom),
        mm(frame.start_x),
        broken_line(profile, gaps, frame),
        theme.palette.ink,
        mm(PROFILE_STROKE_MM * unit)
    );
    out.push('\n');
}

/// The profile as path data, lifting the pen from one end of each gap to the other.
fn broken_line(profile: &Profile, gaps: &[Stretch], frame: &Frame) -> String {
    let point = |along: f64| {
        format!(
            "{} {}",
            mm(frame.x(along)),
            mm(frame.y(profile.height_at(along)))
        )
    };
    let gap_at = |along: f64| gaps.iter().find(|gap| along > gap.from && along < gap.to);
    let mut line = String::new();
    let mut is_lifted = true;
    for &(along, _) in &profile.samples {
        match gap_at(along) {
            Some(gap) if !is_lifted => {
                let _ = write!(line, "L{}", point(gap.from));
                is_lifted = true;
            }
            Some(_) => {}
            None if is_lifted => {
                let start = gaps
                    .iter()
                    .filter(|gap| gap.to <= along)
                    .map(|gap| gap.to)
                    .fold(f64::NEG_INFINITY, f64::max)
                    .max(profile.samples[0].0);
                let _ = write!(line, "M{}", point(start));
                if start < along {
                    let _ = write!(line, "L{}", point(along));
                }
                is_lifted = false;
            }
            None => {
                let _ = write!(line, "L{}", point(along));
            }
        }
    }

    line
}

/// The buildings the line passes through, in solid poché.
pub fn write_cuts(out: &mut String, cuts: &[Cut], frame: &Frame, theme: &Theme) {
    let _ = write!(
        out,
        r#"<g {} fill="{}">"#,
        layer_attributes("section-cut", "Cut"),
        poche(&theme.palette)
    );
    for cut in cuts {
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}"/>"#,
            mm(frame.x(cut.stretch.from)),
            mm(frame.y(cut.top)),
            mm((cut.stretch.to - cut.stretch.from) * frame.mm_per_m),
            mm((cut.top - cut.bottom) * frame.vertical_mm_per_m)
        );
    }
    out.push_str("</g>\n");
}
