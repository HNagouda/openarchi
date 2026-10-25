//! The sunlight the ground receives over a year, in kWh/m², on a grid over
//! the site.
//!
//! Global irradiation on a level surface is its direct and its diffuse part.
//! The direct beam counts only while the cell lies outside every shadow, with
//! the shadows cast exactly as the hours of sun cast them (`sunlight.rs`). The
//! diffuse light comes from a sky taken as equally bright everywhere, the
//! isotropic sky of Liu & Jordan ("The interrelationship and characteristic
//! distribution of direct, diffuse and total solar radiation", Solar Energy 4,
//! 1960), so a cell receives the open-ground diffuse light times its sky view
//! factor (`skyview.rs`).
//!
//! The light is a typical meteorological year (`year.rs`), kept with the scene
//! when it is fetched. Rather than 8,760 hours, each month is one average day,
//! the mean of each hour of the month, with the sun where it stands on Klein's
//! recommended average day of that month ("Calculation of monthly average
//! insolation on tilted surfaces", Solar Energy 19, 1977), times the days in
//! the month. That is 12 days of hourly shadows instead of 365.
//!
//! Only open ground is counted. Roofs are left out, because the shadow
//! casting finds shadows on the ground and not on a lower roof.

mod year;

use std::thread;

use crate::area::LonLat;
use crate::render::shadow_per_metre;
use crate::scene::{Credit, Feature, Layer, Scene};
use crate::skyview;
use crate::sources::pvgis;
use crate::sun;
use crate::sunlight::raster::Raster;
use crate::sunlight::{banded, casters, cells, raster_over, shade, Caster, Cell};
pub use year::{SolarMonth, SolarYear};

/// Klein's recommended average day of each month, January first: the day
/// whose extraterrestrial radiation is nearest the month's mean.
const AVERAGE_DAYS: [u32; 12] = [17, 16, 16, 15, 15, 11, 17, 16, 15, 15, 14, 10];
const WH_PER_KWH: f64 = 1000.0;

/// Yearly irradiation per grid cell, kWh/m². Cells outside the site or under a roof have none.
pub struct RadiationGrid {
    raster: Raster,
    kwh_m2: Vec<Option<f64>>,
}

/// The yearly global irradiation on every ground cell of the site, from the
/// typical year `solar`, on the sky view factor's grid. The sky view and each
/// month's shadows stand on their own, so they are worked out side by side.
pub fn yearly(scene: &Scene, solar: &SolarYear) -> RadiationGrid {
    let raster = raster_over(&scene.boundary, skyview::MAX_CELLS);
    let cells = cells(scene, &raster);
    let (sky, direct_wh) = thread::scope(|scope| {
        let sky = scope.spawn(|| skyview::factors(scene, &raster, &cells));
        let direct_wh = direct_beam(scene, solar, &raster, &cells);
        (joined(sky), direct_wh)
    });
    let open_diffuse_wh: f64 = solar
        .months
        .iter()
        .map(|month| f64::from(month.days) * month.diffuse_w_m2.iter().sum::<f64>())
        .sum();

    let kwh_m2 = sky
        .iter()
        .zip(&direct_wh)
        .map(|(factor, direct)| {
            factor.map(|factor| (direct + open_diffuse_wh * factor) / WH_PER_KWH)
        })
        .collect();
    RadiationGrid { raster, kwh_m2 }
}

/// The PVGIS credit, printed only on sheets that draw the radiation.
pub fn credit() -> Credit {
    Credit {
        layers: vec![Layer::SolarRadiation],
        ..pvgis::credit()
    }
}

/// The grid as bands of `breaks`, each carrying the lower edge of its class.
pub fn bands(grid: &RadiationGrid, breaks: &[f64]) -> Vec<Feature> {
    banded(&grid.raster, &grid.kwh_m2, breaks, Layer::SolarRadiation)
}

/// The direct beam on each cell over the year, Wh/m² on a level surface,
/// one month per thread.
fn direct_beam(scene: &Scene, solar: &SolarYear, raster: &Raster, cells: &[Cell]) -> Vec<f64> {
    let location = sun::site_location(&scene.area);
    // Trees are left out, as in the sky view factor; see the note in skyview.rs.
    let buildings: Vec<Caster> = casters(scene)
        .into_iter()
        .filter(|caster| matches!(caster, Caster::Building { .. }))
        .collect();
    let lit = Lit {
        raster,
        cells,
        buildings: &buildings,
        location,
        offset_hours: solar.irradiance_offset_hours,
    };

    thread::scope(|scope| {
        let months: Vec<_> = solar
            .months
            .iter()
            // Zipped rather than indexed, so a stored year with too many months cannot overrun.
            .zip(AVERAGE_DAYS)
            .enumerate()
            .map(|(index, (month, day))| {
                let lit = &lit;
                scope.spawn(move || month_beam(lit, index as u32 + 1, day, month))
            })
            .collect();
        let mut direct_wh = vec![0.0; raster.len()];
        for month in months {
            for (total, wh) in direct_wh.iter_mut().zip(joined(month)) {
                *total += wh;
            }
        }
        direct_wh
    })
}

/// What a month's direct beam needs to know about the site.
struct Lit<'a> {
    raster: &'a Raster,
    cells: &'a [Cell],
    buildings: &'a [Caster],
    location: LonLat,
    offset_hours: f64,
}

/// One month's direct beam on each cell: DNI times the sine of the sun's
/// altitude on the month's average day, for every hour the cell is lit,
/// times the days in the month.
fn month_beam(lit: &Lit, month_number: u32, day: u32, month: &SolarMonth) -> Vec<f64> {
    let raster = lit.raster;
    let mut shaded_at = vec![usize::MAX; raster.len()];
    let mut direct_wh = vec![0.0; raster.len()];
    for (hour, &direct_normal) in month.direct_normal_w_m2.iter().enumerate() {
        if direct_normal <= 0.0 {
            continue;
        }
        let utc_hours = hour as f64 + lit.offset_hours;
        let sun = sun::position_utc(lit.location, month_number, day, utc_hours);
        // A sun too low to cast a drawn shadow only grazes the ground, as in the hours of sun.
        let Some(per_metre) = shadow_per_metre(&sun) else {
            continue;
        };
        for caster in lit.buildings {
            shade(caster, per_metre, &sun, raster, |index| {
                shaded_at[index] = hour
            });
        }

        let beam_wh = direct_normal * sun.altitude_deg.to_radians().sin() * f64::from(month.days);
        for index in 0..raster.len() {
            if lit.cells[index] == Cell::Ground && shaded_at[index] != hour {
                direct_wh[index] += beam_wh;
            }
        }
    }

    direct_wh
}

/// The answer of a thread. Every failure is a value, so a panic is a bug and goes on up.
fn joined<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    handle
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

#[cfg(test)]
mod tests;
