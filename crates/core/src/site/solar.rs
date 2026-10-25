//! The typical year kept with a scene, for the solar radiation and wind
//! diagrams, which are drawn without asking any service.
//!
//! It is the same PVGIS year the analysis reads for its climate, from the same
//! cache cell, so a site whose climate was looked at fetches nothing new. Each
//! diagram keeps only its cut-down part: monthly sunlight, and monthly hours
//! of wind by direction.

use crate::analysis::cached;
use crate::radiation::{self, SolarYear};
use crate::scene::Scene;
use crate::sources::cache::TileCache;
use crate::sources::grid::Tile;
use crate::sources::pvgis::{self, Tmy};
use crate::wind::{self, WindYear};
use crate::Result;

/// Adds the site's typical sunlight and wind from the cache or PVGIS, or a
/// warning when they cannot be had; the rest of the scene stands without them.
pub fn keep(scene: &mut Scene, cache: &TileCache) {
    keep_with(scene, cache, &pvgis::fetch);
}

pub fn keep_with(scene: &mut Scene, cache: &TileCache, fetch: &dyn Fn(Tile) -> Result<Vec<u8>>) {
    let (tmy, warning) = match typical_year(scene, cache, fetch) {
        Ok(found) => found,
        Err(error) => {
            scene.warnings.push(format!(
                "The typical year of weather could not be fetched, so solar radiation and wind cannot be drawn: {error}"
            ));
            return;
        }
    };
    scene.warnings.extend(warning);
    match SolarYear::from_tmy(&tmy) {
        Ok(solar) => {
            scene.solar = Some(solar);
            scene.credits.push(radiation::credit());
        }
        Err(error) => scene
            .warnings
            .push(format!("Solar radiation cannot be drawn: {error}")),
    }
    match WindYear::from_tmy(&tmy) {
        Ok(year) => {
            scene.wind = Some(year);
            scene.credits.push(wind::credit());
        }
        Err(error) => scene
            .warnings
            .push(format!("Wind cannot be drawn: {error}")),
    }
}

fn typical_year(
    scene: &Scene,
    cache: &TileCache,
    fetch: &dyn Fn(Tile) -> Result<Vec<u8>>,
) -> Result<(Tmy, Option<String>)> {
    let cell = pvgis::cell_of(scene.origin);
    let (bytes, warning) = cached(cache, pvgis::CACHE_DATASET, cell, || {
        let bytes = fetch(cell)?;
        // Only an answer that parses is worth keeping for a month.
        pvgis::parse(&bytes)?;
        Ok(bytes)
    })?;

    Ok((pvgis::parse(&bytes)?, warning))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::scene::Layer;
    use crate::Error;

    const SCENE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/westermarkt.scene.json"
    );
    const RECORDED_YEAR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/analysis-cache/tiles-v1/pvgis-tmy-5km/24_97.zst"
    );

    fn westermarkt() -> Scene {
        serde_json::from_str(&fs::read_to_string(SCENE).unwrap()).unwrap()
    }

    fn recorded() -> Result<Vec<u8>> {
        Ok(zstd::decode_all(fs::read(RECORDED_YEAR).unwrap().as_slice()).unwrap())
    }

    #[test]
    fn a_fetched_year_is_kept_with_one_credit_for_the_radiation_and_one_for_the_wind() {
        let folder = TempDir::new().unwrap();
        let mut scene = westermarkt();
        keep_with(&mut scene, &TileCache::at(folder.path()), &|_| recorded());

        assert_eq!(scene.solar.unwrap().months.len(), 12);
        assert_eq!(scene.wind.unwrap().months.len(), 12);
        let credits = &scene.credits[scene.credits.len() - 2..];
        assert!(credits.iter().all(|credit| credit.name.contains("PVGIS")));
        assert_eq!(credits[0].layers, vec![Layer::SolarRadiation]);
        assert_eq!(credits[1].layers, vec![Layer::Wind]);
    }

    #[test]
    fn an_unreachable_service_leaves_a_warning_and_no_year() {
        let folder = TempDir::new().unwrap();
        let mut scene = westermarkt();
        let warnings_before = scene.warnings.len();
        let down = |_| {
            Err(Error::Network {
                service: "PVGIS".to_string(),
                detail: "503".to_string(),
            })
        };
        keep_with(&mut scene, &TileCache::at(folder.path()), &down);

        assert!(scene.solar.is_none());
        assert!(scene.wind.is_none());
        assert_eq!(scene.warnings.len(), warnings_before + 1);
        let warning = scene.warnings.last().unwrap();
        assert!(warning.contains("solar radiation") && warning.contains("wind"));
    }
}
