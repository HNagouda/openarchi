//! The editions a site is fetched in, found before its tiles, because each
//! edition's tiles are fetched and cached on their own.
//!
//! RIVM and CBS are asked at the same time, each through `sources::edition`,
//! which keeps the answers for a day: the first fetch of a day waits about a
//! second for RIVM's list, later ones not at all. LIWO cannot be asked and has
//! one edition.

use std::collections::BTreeMap;
use std::panic;
use std::thread;

use super::environment::{Raster, RIVM_RASTERS};
use crate::scene::Layer;
use crate::sources::cache::TileCache;
use crate::sources::edition::{self, Edition};
use crate::sources::{cbs, liwo, rivm};

/// Names of the answers kept in the tile cache.
const RIVM_ANSWER: &str = "rivm";
const CBS_ANSWER: &str = "cbs";

/// The edition of every dataset that has more than one.
#[derive(Debug, Clone, PartialEq)]
pub struct Editions {
    pub road_noise: Edition,
    pub all_noise: Edition,
    pub no2: Edition,
    pub pm10: Edition,
    pub pm25: Edition,
    pub flood: Edition,
    /// The newest CBS year with squares; figures it lacks come from the year before.
    pub population: i32,
}

impl Editions {
    /// The editions named in code, without asking any service.
    #[cfg(test)]
    pub fn known() -> Editions {
        Editions {
            road_noise: rivm::known(&rivm::ROAD_NOISE),
            all_noise: rivm::known(&rivm::ALL_NOISE),
            no2: rivm::known(&rivm::NO2),
            pm10: rivm::known(&rivm::PM10),
            pm25: rivm::known(&rivm::PM25),
            flood: liwo::edition(),
            population: cbs::KNOWN_YEAR,
        }
    }

    pub fn raster(&self, raster: Raster) -> &Edition {
        match raster {
            Raster::RoadNoise => &self.road_noise,
            Raster::AllNoise => &self.all_noise,
            Raster::No2 => &self.no2,
            Raster::Pm10 => &self.pm10,
            Raster::Pm25 => &self.pm25,
            Raster::Flood => &self.flood,
        }
    }

    /// The year each layer's data describes, which the scene keeps.
    pub fn years(&self) -> BTreeMap<Layer, i32> {
        let mut years = BTreeMap::new();
        for raster in RIVM_RASTERS {
            years.insert(raster.layer(), self.raster(raster).year);
        }
        years.insert(Layer::Flood, self.flood.year);
        years.insert(Layer::Population, self.population);

        years
    }
}

/// The newest editions, from the services or from the cache, and a warning
/// for each source whose newest edition could not be found.
pub fn find(cache: &TileCache) -> (Editions, Vec<String>) {
    let ((rivm_ids, rivm_error), (population, cbs_error)) = thread::scope(|scope| {
        let cbs =
            scope.spawn(|| edition::resolve(cache, CBS_ANSWER, cbs::latest_year, cbs::KNOWN_YEAR));
        let rivm = edition::resolve(cache, RIVM_ANSWER, rivm::list_ids, known_rivm_ids());
        // `resolve` returns every failure as a value, so a panic is a bug and goes on up.
        let cbs = cbs
            .join()
            .unwrap_or_else(|payload| panic::resume_unwind(payload));
        (rivm, cbs)
    });

    let mut warnings = Vec::new();
    let mut rivm_edition = |map: rivm::Map| -> Edition {
        match rivm::newest(&map, &rivm_ids) {
            Some(edition) => edition,
            None => {
                let known = rivm::known(&map);
                warnings.push(format!(
                    "RIVM no longer lists a map like {}, so that edition was asked for anyway.",
                    known.id
                ));
                known
            }
        }
    };
    let editions = Editions {
        road_noise: rivm_edition(rivm::ROAD_NOISE),
        all_noise: rivm_edition(rivm::ALL_NOISE),
        no2: rivm_edition(rivm::NO2),
        pm10: rivm_edition(rivm::PM10),
        pm25: rivm_edition(rivm::PM25),
        flood: liwo::edition(),
        population,
    };

    if let Some(error) = rivm_error {
        warnings.push(format!(
            "RIVM could not be asked for its newest maps, so noise is from {} and air quality from {}, which may not be the newest: {error}",
            editions.road_noise.year, editions.no2.year
        ));
    }
    if let Some(error) = cbs_error {
        warnings.push(format!(
            "CBS could not be asked for its newest year, so population is from {}, which may not be the newest: {error}",
            editions.population
        ));
    }

    (editions, warnings)
}

fn known_rivm_ids() -> Vec<String> {
    rivm::MAPS.map(|map| map.known.to_string()).to_vec()
}

/// The years of the editions every scene was fetched in before editions were
/// kept in the scene, when they were pinned in code. Frozen: changing a year
/// here would change what old projects say about their own data.
pub fn years_before_editions_were_kept() -> BTreeMap<Layer, i32> {
    BTreeMap::from([
        (Layer::RoadNoise, 2022),
        (Layer::AllNoise, 2022),
        (Layer::No2, 2024),
        (Layer::Pm10, 2024),
        (Layer::Pm25, 2024),
        (Layer::Flood, 2025),
        (Layer::Population, 2024),
    ])
}
