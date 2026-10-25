//! The direct beam over a year: for every hour of each month's average day,
//! a cell is lit when the straight line from it towards the sun clears the
//! height surface of `skyview::Surface`, the buildings, dense trees and
//! terrain. The line is walked out one cell at a time, the way the sky view
//! walks out to the horizon, until it rises above the highest point of the
//! surface or leaves it. A ground cell looks from the ground, so the floor of
//! a wood is in the shade; a roof cell looks from its roof, so a taller
//! building next door shades a lower roof.

use std::thread;

use super::{joined, SolarMonth, SolarYear, AVERAGE_DAYS};
use crate::area::LonLat;
use crate::grid::{Cell, Raster, Surface};
use crate::render::shadow_per_metre;
use crate::sun::{self, SunPosition};

/// What a month's direct beam needs to know about the site.
pub(super) struct Lit<'a> {
    pub raster: &'a Raster,
    pub cells: &'a [Cell],
    pub surface: &'a Surface,
    pub location: LonLat,
    pub offset_hours: f64,
}

/// The direct beam on each cell over the year, Wh/m² on a level surface,
/// one month per thread.
pub(super) fn direct_beam(lit: &Lit, solar: &SolarYear) -> Vec<f64> {
    thread::scope(|scope| {
        let months: Vec<_> = solar
            .months
            .iter()
            // Zipped rather than indexed, so a stored year with too many months cannot overrun.
            .zip(AVERAGE_DAYS)
            .enumerate()
            .map(|(index, (month, day))| {
                scope.spawn(move || month_beam(lit, index as u32 + 1, day, month))
            })
            .collect();
        let mut direct_wh = vec![0.0; lit.raster.len()];
        for month in months {
            for (total, wh) in direct_wh.iter_mut().zip(joined(month)) {
                *total += wh;
            }
        }
        direct_wh
    })
}

/// The direct beam on level ground that nothing shades, Wh/m² over the year,
/// summed exactly as `direct_beam` sums a lit cell.
pub(super) fn open_beam(location: LonLat, solar: &SolarYear) -> f64 {
    let offset_hours = solar.irradiance_offset_hours;
    let mut total = 0.0;
    for (index, (month, day)) in solar.months.iter().zip(AVERAGE_DAYS).enumerate() {
        let hours = sunny_hours(location, offset_hours, index as u32 + 1, day, month);
        total += hours.iter().map(|(_, beam_wh)| beam_wh).sum::<f64>();
    }

    total
}

/// One month's direct beam on each cell: DNI times the sine of the sun's
/// altitude on the month's average day, for every hour the cell is lit,
/// times the days in the month.
fn month_beam(lit: &Lit, month_number: u32, day: u32, month: &SolarMonth) -> Vec<f64> {
    let raster = lit.raster;
    let mut direct_wh = vec![0.0; raster.len()];
    let solar_hours = sunny_hours(lit.location, lit.offset_hours, month_number, day, month);
    for (sun, beam_wh) in solar_hours {
        let azimuth = sun.azimuth_deg.to_radians();
        let toward_sun = (azimuth.sin(), azimuth.cos());
        let rise_per_metre = sun.altitude_deg.to_radians().tan();
        for (index, cell) in lit.cells.iter().enumerate() {
            let point = raster.centre(index);
            let base = match cell {
                Cell::Ground => lit.surface.ground_at(point),
                Cell::Roof => lit.surface.height_at(point),
                Cell::Outside => continue,
            };
            let ray = Ray {
                point,
                base,
                toward_sun,
                rise_per_metre,
            };
            if is_lit(lit.surface, raster.cell_m, &ray) {
                direct_wh[index] += beam_wh;
            }
        }
    }

    direct_wh
}

/// The line from a cell towards the sun.
struct Ray {
    point: (f64, f64),
    base: f64,
    toward_sun: (f64, f64),
    rise_per_metre: f64,
}

/// Nothing on the surface stands above the line from the cell to the sun.
fn is_lit(surface: &Surface, step_m: f64, ray: &Ray) -> bool {
    let mut distance = step_m;
    loop {
        let line_m = ray.base + distance * ray.rise_per_metre;
        if line_m >= surface.highest {
            return true;
        }
        let at = (
            ray.point.0 + ray.toward_sun.0 * distance,
            ray.point.1 + ray.toward_sun.1 * distance,
        );
        if surface.is_beyond(at) {
            return true;
        }
        if surface.height_at(at) > line_m {
            return false;
        }
        distance += step_m;
    }
}

/// Every hour of the month's average day with a direct beam, as the sun and
/// the beam on a level surface over the month, Wh/m².
fn sunny_hours(
    location: LonLat,
    offset_hours: f64,
    month_number: u32,
    day: u32,
    month: &SolarMonth,
) -> Vec<(SunPosition, f64)> {
    let mut hours = Vec::new();
    for (hour, &direct_normal) in month.direct_normal_w_m2.iter().enumerate() {
        if direct_normal <= 0.0 {
            continue;
        }
        let sun = sun::position_utc(location, month_number, day, hour as f64 + offset_hours);
        // A sun too low to cast a drawn shadow only grazes the ground, as in the hours of sun.
        if shadow_per_metre(&sun).is_none() {
            continue;
        }
        let beam_wh = direct_normal * sun.altitude_deg.to_radians().sin() * f64::from(month.days);
        hours.push((sun, beam_wh));
    }

    hours
}
