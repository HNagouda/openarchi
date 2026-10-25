//! The sun path drawn over the site: the sky folded flat onto the frame.
//!
//! The frame's circle is the horizon and its centre the point straight
//! overhead. A sun position lands at its compass bearing, at a distance from
//! the centre proportional to how far it is below the zenith, so a sun at 30
//! degrees sits two thirds of the way out. Hours are winter time (CET) all
//! year, which keeps each hour's line smooth across the switch to summer time.

use std::fmt::Write;

use super::color;
use super::geometry::{decimal, mm, push_paper_points, Projection};
use super::legend::{Row, Swatch};
use super::sunpath_guides::{write_guides, write_hour_lines};
use super::typeset::{Kind, Typesetter};
use crate::area::LonLat;
use crate::sun::{position_utc, SunPosition, STANDARD_OFFSET_HOURS};
use crate::theme::Palette;

const SUMMER_SOLSTICE: (u32, u32) = (6, 21);
/// The March equinox; the September one draws the same arc.
const EQUINOX: (u32, u32) = (3, 20);
const WINTER_SOLSTICE: (u32, u32) = (12, 21);
const SAMPLE_MINUTES: usize = 5;
const MINUTES_PER_DAY: usize = 24 * 60;

// Line weights and marks on paper, in millimetres.
const ARC_MM: f64 = 0.45;
pub(super) const GUIDE_MM: f64 = 0.12;
const HOUR_DOT_RADIUS: f64 = 0.55;
const SUN_RADIUS: f64 = 1.5;
const LABEL_GAP: f64 = 2.2;
const BAND_OPACITY: f64 = 0.1;
/// How far winter's arc moves from the water colour towards ink, so it reads on every theme.
const WINTER_INK_SHARE: f64 = 0.45;

/// The colours of the three arcs, from the theme.
struct Inks {
    summer: String,
    equinox: String,
    winter: String,
}

pub struct SunPath<'a> {
    pub location: LonLat,
    /// Centre and radius of the horizon on the ground, in metres.
    pub horizon: ((f64, f64), f64),
    pub projection: &'a Projection,
    pub palette: &'a Palette,
    /// The look's moment, marked with a dot when the sun is up.
    pub sun: SunPosition,
    pub sun_label: String,
    /// Keeps ids unique when several maps share a sheet.
    pub scope: &'a str,
    /// Labels on the arcs and the compass ring; left off small explode plates.
    pub has_labels: bool,
}

/// Draws the sun path and returns its legend rows.
pub fn write(out: &mut String, path: &SunPath, typesetter: &Typesetter) -> Vec<Row> {
    let inks = Inks {
        summer: path.palette.accent.clone(),
        equinox: path.palette.ink.clone(),
        winter: color::mix(&path.palette.water, &path.palette.ink, WINTER_INK_SHARE),
    };
    let ink = &path.palette.ink;
    let _ = write!(
        out,
        r#"<g id="{}sun-path" inkscape:groupmode="layer" inkscape:label="Sun path" font-family="{}" fill="none" stroke-linecap="round" stroke-linejoin="round">"#,
        path.scope,
        typesetter.face.css_stack().replace('\'', "&apos;")
    );

    let summer = day_arc(path, SUMMER_SOLSTICE);
    let equinox = day_arc(path, EQUINOX);
    let winter = day_arc(path, WINTER_SOLSTICE);
    write_band(out, path, &summer, &winter, &inks.summer);
    write_guides(out, path, typesetter, ink);
    write_hour_lines(out, path, ink);
    for (arc, colour) in [
        (&summer, &inks.summer),
        (&equinox, &inks.equinox),
        (&winter, &inks.winter),
    ] {
        let _ = write!(
            out,
            r#"<path d="{}" stroke="{colour}" stroke-width="{}"/>"#,
            paper_path(path, arc, false),
            mm(ARC_MM)
        );
    }
    for (date, colour, is_outward) in [
        (SUMMER_SOLSTICE, &inks.summer, false),
        (EQUINOX, &inks.equinox, false),
        (WINTER_SOLSTICE, &inks.winter, true),
    ] {
        write_hour_marks(out, path, typesetter, date, colour, is_outward);
    }
    if path.sun.is_up() {
        let (x, y) = to_paper(path, &path.sun);
        let _ = write!(
            out,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
            mm(x),
            mm(y),
            mm(SUN_RADIUS),
            path.palette.accent,
            path.palette.paper,
            mm(GUIDE_MM * 2.0)
        );
    }
    out.push_str("</g>\n");

    legend_rows(path, &inks)
}

fn legend_rows(path: &SunPath, inks: &Inks) -> Vec<Row> {
    let line = |label: &str, stroke: &str| Row {
        label: label.to_string(),
        swatch: Swatch::Line {
            stroke: stroke.to_string(),
            stroke_mm: ARC_MM,
            dash_mm: Vec::new(),
            opacity: 1.0,
        },
    };
    let mut rows = vec![
        line("Summer solstice, 21 June", &inks.summer),
        line("Equinox, 20 March and 22 September", &inks.equinox),
        line("Winter solstice, 21 December", &inks.winter),
        Row {
            label: "Range of the sun through the year".to_string(),
            swatch: Swatch::Area {
                fill: path.palette.accent.clone(),
                stroke: None,
                stroke_mm: 0.0,
                opacity: BAND_OPACITY * 2.0,
            },
        },
    ];
    if path.sun.is_up() {
        rows.push(Row {
            label: format!("Sun on {}", path.sun_label),
            swatch: Swatch::Dot {
                fill: Some(path.palette.accent.clone()),
                stroke: None,
                opacity: 1.0,
            },
        });
    }

    rows
}

/// Sun positions every few minutes while the sun is up, sunrise to sunset.
fn day_arc(path: &SunPath, (month, day): (u32, u32)) -> Vec<SunPosition> {
    let samples: Vec<SunPosition> = (0..=MINUTES_PER_DAY / SAMPLE_MINUTES)
        .map(|step| {
            let utc_hours = (step * SAMPLE_MINUTES) as f64 / 60.0;
            position_utc(path.location, month, day, utc_hours)
        })
        .collect();

    let mut arc = Vec::new();
    for pair in samples.windows(2) {
        let (before, after) = (pair[0], pair[1]);
        if before.is_up() {
            arc.push(before);
        }
        // Sunrise or sunset between two samples: add the point on the horizon.
        if before.is_up() != after.is_up() {
            let share = before.altitude_deg / (before.altitude_deg - after.altitude_deg);
            arc.push(SunPosition {
                azimuth_deg: before.azimuth_deg + (after.azimuth_deg - before.azimuth_deg) * share,
                altitude_deg: 0.0,
            });
        }
    }

    arc
}

/// The sky position on the paper.
fn to_paper(path: &SunPath, position: &SunPosition) -> (f64, f64) {
    let ((centre_x, centre_y), radius) = path.horizon;
    let distance = radius * (90.0 - position.altitude_deg.max(0.0)) / 90.0;
    let azimuth = position.azimuth_deg.to_radians();
    path.projection.to_paper(
        centre_x + distance * azimuth.sin(),
        centre_y + distance * azimuth.cos(),
    )
}

pub(super) fn paper_path(path: &SunPath, positions: &[SunPosition], closed: bool) -> String {
    let points: Vec<(f64, f64)> = positions.iter().map(|p| to_paper(path, p)).collect();
    let mut data = String::new();
    push_paper_points(&mut data, &points, closed);
    data
}

/// The sky between the summer and winter arcs, closed along the horizon.
fn write_band(
    out: &mut String,
    path: &SunPath,
    summer: &[SunPosition],
    winter: &[SunPosition],
    fill: &str,
) {
    let (Some(summer_set), Some(winter_set)) = (summer.last(), winter.last()) else {
        return;
    };
    let (Some(winter_rise), Some(summer_rise)) = (winter.first(), summer.first()) else {
        return;
    };
    let mut outline: Vec<SunPosition> = summer.to_vec();
    outline.extend(horizon_between(
        summer_set.azimuth_deg,
        winter_set.azimuth_deg,
    ));
    outline.extend(winter.iter().rev().copied());
    outline.extend(horizon_between(
        winter_rise.azimuth_deg,
        summer_rise.azimuth_deg,
    ));
    let _ = write!(
        out,
        r#"<path d="{}" fill="{fill}" fill-opacity="{}" stroke="none"/>"#,
        paper_path(path, &outline, true),
        decimal(BAND_OPACITY)
    );
}

/// Points along the horizon from one bearing to another, the short way round.
fn horizon_between(from_deg: f64, to_deg: f64) -> Vec<SunPosition> {
    let steps = ((to_deg - from_deg).abs() / 3.0).ceil().max(1.0) as usize;
    (0..=steps)
        .map(|step| SunPosition {
            azimuth_deg: from_deg + (to_deg - from_deg) * step as f64 / steps as f64,
            altitude_deg: 0.0,
        })
        .collect()
}

/// A dot at each whole hour on one day's arc, numbered when labels are on.
fn write_hour_marks(
    out: &mut String,
    path: &SunPath,
    typesetter: &Typesetter,
    (month, day): (u32, u32),
    colour: &str,
    is_outward: bool,
) {
    let (centre, _) = path.horizon;
    let centre_paper = path.projection.to_paper(centre.0, centre.1);
    for hour in 0..24 {
        let position = position_utc(
            path.location,
            month,
            day,
            hour as f64 - STANDARD_OFFSET_HOURS,
        );
        if !position.is_up() {
            continue;
        }
        let (x, y) = to_paper(path, &position);
        let _ = write!(
            out,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{colour}" stroke-width="{}"/>"#,
            mm(x),
            mm(y),
            mm(HOUR_DOT_RADIUS),
            path.palette.paper,
            mm(GUIDE_MM * 1.5)
        );
        // Equinox hours sit between the other two arcs and would crowd them.
        if !path.has_labels || (month, day) == EQUINOX {
            continue;
        }
        let (dx, dy) = (x - centre_paper.0, y - centre_paper.1);
        let length = (dx * dx + dy * dy).sqrt().max(1e-6);
        let direction = if is_outward { 1.0 } else { -1.0 };
        let gap = LABEL_GAP * typesetter.unit;
        let (label_x, label_y) = (
            x + direction * dx / length * gap,
            y + direction * dy / length * gap,
        );
        let baseline = label_y + typesetter.cap_height(Kind::Small) / 2.0;
        typesetter.write(
            out,
            label_x,
            baseline,
            &format!("{hour:02}"),
            Kind::Small,
            Some("middle"),
        );
    }
}
