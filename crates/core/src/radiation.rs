//! The sunlight the ground and the roofs receive over a year, in kWh/m², on a
//! grid over the site.
//!
//! Global irradiation on a level surface is its direct and its diffuse part.
//! The direct beam counts only while the line from the cell towards the sun
//! clears the buildings, the dense trees and the terrain (`beam.rs`). The
//! diffuse light comes from a sky taken as equally bright everywhere, the
//! isotropic sky of Liu & Jordan ("The interrelationship and characteristic
//! distribution of direct, diffuse and total solar radiation", Solar Energy 4,
//! 1960), so a cell receives the open-ground diffuse light times its sky view
//! factor (`skyview.rs`). Both read one height surface, so they agree on what
//! blocks: buildings as prisms of their roof height, dense trees as solid
//! stands of their canopy height (`forest.rs`; a single tree lets the light
//! through), and the terrain where the scene kept it. A roof is counted from
//! its own height, so a taller building next door shades a lower roof.
//!
//! The light is a typical meteorological year (`year.rs`), kept with the scene
//! when it is fetched. Rather than 8,760 hours, each month is one average day,
//! the mean of each hour of the month, with the sun where it stands on Klein's
//! recommended average day of that month ("Calculation of monthly average
//! insolation on tilted surfaces", Solar Energy 19, 1977), times the days in
//! the month. That is 12 days of hourly sun positions instead of 365.
//!
//! The classes are shares of what open ground receives on the site, see `classes.rs`.

mod beam;
mod classes;
mod year;

use std::thread;

use crate::scene::{Credit, Feature, Layer, Scene};
use crate::skyview;
use crate::sources::pvgis;
use crate::sun;
use crate::sunlight::raster::Raster;
use crate::sunlight::{banded, cells, raster_over};
use beam::Lit;
pub use classes::sited;
pub use year::{SolarMonth, SolarYear};

/// Klein's recommended average day of each month, January first: the day
/// whose extraterrestrial radiation is nearest the month's mean.
const AVERAGE_DAYS: [u32; 12] = [17, 16, 16, 15, 15, 11, 17, 16, 15, 15, 14, 10];
const WH_PER_KWH: f64 = 1000.0;

/// Yearly irradiation per grid cell, kWh/m². Cells outside the site have none.
pub struct RadiationGrid {
    raster: Raster,
    kwh_m2: Vec<Option<f64>>,
}

/// The yearly global irradiation on every ground and roof cell of the site,
/// from the typical year `solar`, on the sky view factor's grid. The sky view
/// and the direct beam stand on their own, so they are worked out side by side.
pub fn yearly(scene: &Scene, solar: &SolarYear) -> RadiationGrid {
    let raster = raster_over(&scene.boundary, skyview::MAX_CELLS);
    let cells = cells(scene, &raster);
    let surface = skyview::surface(scene, &raster, skyview::SEARCH_RADIUS_M);
    let lit = Lit {
        raster: &raster,
        cells: &cells,
        surface: &surface,
        location: sun::site_location(&scene.area),
        offset_hours: solar.irradiance_offset_hours,
    };
    let (sky, direct_wh) = thread::scope(|scope| {
        let sky = scope.spawn(|| skyview::factors(&surface, &raster, &cells));
        let direct_wh = beam::direct_beam(&lit, solar);
        (joined(sky), direct_wh)
    });

    let open_diffuse_wh = open_diffuse(solar);
    let kwh_m2 = sky
        .iter()
        .zip(&direct_wh)
        .map(|(factor, direct)| {
            factor.map(|factor| (direct + open_diffuse_wh * factor) / WH_PER_KWH)
        })
        .collect();
    RadiationGrid { raster, kwh_m2 }
}

/// What level ground that nothing shades receives over the typical year,
/// kWh/m²: the top of the scale the classes are shares of.
pub fn open_ground_kwh(scene: &Scene, solar: &SolarYear) -> f64 {
    let location = sun::site_location(&scene.area);
    (beam::open_beam(location, solar) + open_diffuse(solar)) / WH_PER_KWH
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

/// The diffuse light on level ground under the whole sky over the year, Wh/m².
fn open_diffuse(solar: &SolarYear) -> f64 {
    solar
        .months
        .iter()
        .map(|month| f64::from(month.days) * month.diffuse_w_m2.iter().sum::<f64>())
        .sum()
}

/// The answer of a thread. Every failure is a value, so a panic is a bug and goes on up.
fn joined<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    handle
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

#[cfg(test)]
mod tests;
