//! The faint guides under the sun path: altitude rings, the compass ticks and
//! bearings round the horizon, and each whole hour's line through the half year.

use std::fmt::Write;

use super::geometry::{decimal, mm};
use super::sunpath::{paper_path, SunPath, GUIDE_MM};
use super::typeset::{Kind, Typesetter};
use crate::sun::{position_utc, SunPosition, STANDARD_OFFSET_HOURS};

/// The 21st of each month from winter to summer, for the hour lines.
const HOUR_LINE_DATES: [(u32, u32); 7] = [
    (12, 21),
    (1, 21),
    (2, 21),
    (3, 21),
    (4, 21),
    (5, 21),
    (6, 21),
];
/// Altitude rings drawn as faint guides.
const ALTITUDE_RINGS_DEG: [f64; 2] = [30.0, 60.0];
const RING_POINTS: usize = 120;

// Line weights and marks on paper, in millimetres.
const HOUR_LINE_MM: f64 = 0.16;
const TICK_SHORT: f64 = 0.9;
const TICK_LONG: f64 = 2.0;
/// Compass bearings sit further out, clear of the horizon line and its ticks.
const COMPASS_GAP: f64 = 4.0;
const GUIDE_OPACITY: f64 = 0.35;

/// Altitude rings, the compass ticks round the horizon and their bearings.
pub(super) fn write_guides(out: &mut String, path: &SunPath, typesetter: &Typesetter, ink: &str) {
    let _ = write!(
        out,
        r#"<g stroke="{ink}" stroke-width="{}" stroke-opacity="{}">"#,
        mm(GUIDE_MM),
        decimal(GUIDE_OPACITY)
    );
    for altitude in ALTITUDE_RINGS_DEG {
        let ring: Vec<SunPosition> = (0..RING_POINTS)
            .map(|index| SunPosition {
                azimuth_deg: index as f64 / RING_POINTS as f64 * 360.0,
                altitude_deg: altitude,
            })
            .collect();
        let _ = write!(
            out,
            r#"<path d="{}" stroke-dasharray="0.6 0.9"/>"#,
            paper_path(path, &ring, true)
        );
    }
    let mm_per_m = path.projection.mm_per_m;
    let radius = path.horizon.1;
    for degree in (0..360).step_by(5) {
        let length = if degree % 15 == 0 {
            TICK_LONG
        } else {
            TICK_SHORT
        };
        let inner_altitude = 90.0 * length / (radius * mm_per_m);
        let tick = [
            SunPosition {
                azimuth_deg: degree as f64,
                altitude_deg: 0.0,
            },
            SunPosition {
                azimuth_deg: degree as f64,
                altitude_deg: inner_altitude,
            },
        ];
        let _ = write!(out, r#"<path d="{}"/>"#, paper_path(path, &tick, false));
    }
    out.push_str("</g>");
    if !path.has_labels {
        return;
    }

    for degree in (0..360).step_by(30) {
        let label = match degree {
            0 => "N".to_string(),
            90 => "E".to_string(),
            180 => "S".to_string(),
            270 => "W".to_string(),
            _ => degree.to_string(),
        };
        // Just outside the horizon, by the label gap.
        let outside = -90.0 * COMPASS_GAP * typesetter.unit / (radius * mm_per_m);
        let (x, y) = to_paper_unclamped(path, degree as f64, outside);
        let kind = if degree % 90 == 0 {
            Kind::Heading
        } else {
            Kind::Small
        };
        let baseline = y + typesetter.cap_height(kind) / 2.0;
        typesetter.write(out, x, baseline, &label, kind, Some("middle"));
    }
}

/// Like `to_paper`, but lets a negative altitude land outside the horizon, for labels.
fn to_paper_unclamped(path: &SunPath, azimuth_deg: f64, altitude_deg: f64) -> (f64, f64) {
    let ((centre_x, centre_y), radius) = path.horizon;
    let distance = radius * (90.0 - altitude_deg) / 90.0;
    let azimuth = azimuth_deg.to_radians();
    path.projection.to_paper(
        centre_x + distance * azimuth.sin(),
        centre_y + distance * azimuth.cos(),
    )
}

/// Each whole hour's position through the half year, as a dotted line.
pub(super) fn write_hour_lines(out: &mut String, path: &SunPath, ink: &str) {
    let _ = write!(
        out,
        r#"<g stroke="{ink}" stroke-width="{}" stroke-opacity="{}" stroke-dasharray="0.1 0.9">"#,
        mm(HOUR_LINE_MM),
        decimal(GUIDE_OPACITY * 1.6)
    );
    for hour in 0..24 {
        let utc_hours = hour as f64 - STANDARD_OFFSET_HOURS;
        let positions: Vec<SunPosition> = HOUR_LINE_DATES
            .iter()
            .map(|&(month, day)| position_utc(path.location, month, day, utc_hours))
            .collect();
        // Only the stretch where the sun is up at this hour on every date in a run.
        for run in positions.split(|position| !position.is_up()) {
            if run.len() >= 2 {
                let _ = write!(out, r#"<path d="{}"/>"#, paper_path(path, run, false));
            }
        }
    }
    out.push_str("</g>");
}
