//! The typical year of sunlight kept with a scene, for the solar radiation
//! diagram, which is drawn without asking any service.
//!
//! It is the same PVGIS year the analysis reads for its climate, from the same
//! cache cell, so a site whose climate was looked at fetches nothing new.

use crate::analysis::cached;
use crate::radiation::{self, SolarYear};
use crate::scene::Scene;
use crate::sources::cache::TileCache;
use crate::sources::grid::Tile;
use crate::sources::pvgis;
use crate::Result;

/// Adds the site's typical year from the cache or PVGIS, or a warning when
/// it cannot be had; the rest of the scene stands without it.
pub fn keep(scene: &mut Scene, cache: &TileCache) {
    keep_with(scene, cache, &pvgis::fetch);
}

pub fn keep_with(scene: &mut Scene, cache: &TileCache, fetch: &dyn Fn(Tile) -> Result<Vec<u8>>) {
    match solar_year(scene, cache, fetch) {
        Ok((solar, warning)) => {
            scene.solar = Some(solar);
            scene.warnings.extend(warning);
            scene.credits.push(radiation::credit());
        }
        Err(error) => scene.warnings.push(format!(
            "The typical year of sunlight could not be fetched, so solar radiation cannot be drawn: {error}"
        )),
    }
}

fn solar_year(
    scene: &Scene,
    cache: &TileCache,
    fetch: &dyn Fn(Tile) -> Result<Vec<u8>>,
) -> Result<(SolarYear, Option<String>)> {
    let cell = pvgis::cell_of(scene.origin);
    let (bytes, warning) = cached(cache, pvgis::CACHE_DATASET, cell, || {
        let bytes = fetch(cell)?;
        // Only an answer that parses is worth keeping for a month.
        pvgis::parse(&bytes)?;
        Ok(bytes)
    })?;
    let solar = SolarYear::from_tmy(&pvgis::parse(&bytes)?)?;

    Ok((solar, warning))
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
    fn a_fetched_year_is_kept_with_its_credit_for_the_radiation_only() {
        let folder = TempDir::new().unwrap();
        let mut scene = westermarkt();
        keep_with(&mut scene, &TileCache::at(folder.path()), &|_| recorded());

        let solar = scene.solar.unwrap();
        assert_eq!(solar.months.len(), 12);
        let credit = scene.credits.last().unwrap();
        assert!(credit.name.contains("PVGIS"));
        assert_eq!(credit.layers, vec![Layer::SolarRadiation]);
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
        assert_eq!(scene.warnings.len(), warnings_before + 1);
        assert!(scene.warnings.last().unwrap().contains("solar radiation"));
    }
}
