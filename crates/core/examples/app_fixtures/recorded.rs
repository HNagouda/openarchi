use std::fs;
use std::path::Path;

use openarchi_core::analysis::{self, Part, PARTS};
use openarchi_core::area::{Area, LonLat};
use openarchi_core::radiation::{self, SolarYear};
use openarchi_core::scene::Scene;
use openarchi_core::selection;
use openarchi_core::sources::cache::TileCache;
use openarchi_core::sources::grid::Tile;
use openarchi_core::sources::pvgis;

use crate::palettes::fixture_themes;

const RECORDED_ANALYSIS: &str = "tests/fixtures/analysis-cache/tiles-v1";

/// A cache holding the answers recorded in tests/fixtures/analysis-cache, so
/// no service is asked.
/// The typical year a fetch keeps today, which the fixture predates, from the recorded PVGIS answer.
pub(crate) fn add_solar_year(scene: &mut Scene, cache: &TileCache) {
    let bytes = cache
        .read(pvgis::CACHE_DATASET, pvgis::cell_of(scene.origin))
        .expect("read the cache")
        .expect("the recorded PVGIS year");
    let tmy = pvgis::parse(&bytes).expect("parse the recorded PVGIS year");
    scene.solar = Some(SolarYear::from_tmy(&tmy).expect("a full typical year"));
    scene.credits.push(radiation::credit());
}

pub(crate) fn recorded_cache() -> (tempfile::TempDir, TileCache) {
    let cache_dir = tempfile::TempDir::new().expect("a temporary cache");
    let cache = TileCache::at(cache_dir.path());
    let recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join(RECORDED_ANALYSIS);
    for dataset in fs::read_dir(&recorded).expect("the recorded analysis answers") {
        let dataset = dataset.expect("a dataset folder").path();
        let name = dataset
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        for file in fs::read_dir(&dataset).expect("the dataset's files") {
            let path = file.expect("a recorded answer").path();
            let stem = path
                .file_stem()
                .expect("a stem")
                .to_string_lossy()
                .to_string();
            let (column, row) = stem.split_once('_').expect("a file named column_row");
            let tile = Tile {
                column: column.parse().expect("a column"),
                row: row.parse().expect("a row"),
            };
            let bytes =
                zstd::decode_all(fs::read(&path).expect("read").as_slice()).expect("unpack");
            // Written fresh, so the cache never takes a recorded answer for a stale one.
            cache.write(&name, tile, &bytes).expect("seed the cache");
        }
    }

    (cache_dir, cache)
}

/// Every analysis part in every theme, as `get_analysis` returns it.
pub(crate) fn write_analysis(scene: &Scene, cache: &TileCache, out_dir: &Path) {
    let folder = out_dir.join("analysis");
    fs::create_dir_all(&folder).expect("create the analysis folder");

    for theme in fixture_themes() {
        for part in PARTS {
            let sections =
                analysis::part(part, scene, None, &theme, cache).expect("draw the analysis part");
            let part_name = serde_json::to_value(part).expect("a part name");
            let file = folder.join(format!(
                "{}--{}.json",
                part_name.as_str().expect("text"),
                theme.id
            ));
            fs::write(file, serde_json::to_string(&sections).expect("serialise"))
                .expect("write the part");
        }

        // The mock serves this for any sheet with a sub-selection.
        let sheet_area = selection::framed(scene, &sheet_area()).expect("frame the sheet area");
        let sections = analysis::part(Part::Site, scene, Some(&sheet_area), &theme, cache)
            .expect("draw the sheet area's site part");
        let file = folder.join(format!("site-area--{}.json", theme.id));
        fs::write(file, serde_json::to_string(&sections).expect("serialise"))
            .expect("write the part");
    }
}

/// 60 m around the Westerkerk, inside the Westermarkt fixture.
pub(crate) fn sheet_area() -> Area {
    Area::Circle {
        center: LonLat {
            lon: 4.8840,
            lat: 52.3745,
        },
        radius_m: 60.0,
    }
}
