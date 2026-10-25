//! Trees, green, the vegetation index and the heat island of a site, from
//! tiles to bands.
//!
//! These four rasters come from three services that fail on their own: the
//! RIVM tree and green maps, the PDOK infrared photo and the RIVM heat island
//! map. Each is fetched in the same tiles as noise and air quality, measured
//! inside the site, and banded at the breaks of the diagram variant that
//! draws it, like `site/environment.rs` does for those.

use std::collections::HashMap;

use geo::{BoundingRect, Coord, Polygon, Rect};

use super::download::Dataset;
use super::editions::Editions;
use super::environment::breaks_for;
use super::source::Source;
use crate::environment::Measured;
use crate::scene::{Feature, Layer};
use crate::sources::bands::{self, Gaps, Lowest};
use crate::sources::edition::Edition;
use crate::sources::raster::{self, Grid};
use crate::sources::{luchtfoto, rivm, wcs};
use crate::{forest, skyview, Result};

/// RIVM leaves cells without a tree empty and draws trees from 2.5 m, in
/// whole metres. The canopy edge is traced at 1 m, between the 0 of no tree
/// and the lowest tree, so it follows the crowns rather than the rounding.
const CANOPY_EDGE_M: f64 = 1.0;
/// Trees around the site hide sky and sun too (see `forest.rs`), so their
/// heights are fetched and kept this far past the site's box: as far as the
/// sky view searches the horizon.
pub const TREE_REACH_M: f64 = skyview::SEARCH_RADIUS_M;
/// A green class is traced as a mask of ones on zeros; its edge lies halfway.
const MASK_EDGE: f64 = 0.5;
const IN_MASK: f32 = 1.0;

/// RIVM's green class codes and the group each becomes, in one place. The
/// green cover diagram classifies by these groups.
// #! NOTE[green, palette] : The classes take the palette's category colours, so green land is yellow in Paper; give themes a green ramp or category?
pub const GREEN_GROUPS: [(f32, &str); 4] = [
    (1.0, "farmland"),
    (2.0, "natura_2000"),
    (3.0, "nature_network"),
    (4.0, "other_green"),
];

/// Every raster of this module, in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Greenery {
    TreeHeight,
    GreenClasses,
    Vegetation,
    HeatIsland,
}

pub const KINDS: [Greenery; 4] = [
    Greenery::TreeHeight,
    Greenery::GreenClasses,
    Greenery::Vegetation,
    Greenery::HeatIsland,
];

impl Greenery {
    pub fn layer(self) -> Layer {
        match self {
            Greenery::TreeHeight => Layer::TreeCanopy,
            Greenery::GreenClasses => Layer::GreenCover,
            Greenery::Vegetation => Layer::VegetationIndex,
            Greenery::HeatIsland => Layer::HeatIsland,
        }
    }

    pub fn source(self) -> Source {
        match self {
            Greenery::TreeHeight | Greenery::GreenClasses => Source::Green,
            Greenery::Vegetation => Source::Vegetation,
            Greenery::HeatIsland => Source::Heat,
        }
    }

    /// The tile cache folder of `edition` of this raster. The RIVM maps share
    /// the WCS folder name, so a tile is never fetched twice under two names.
    pub fn cache_name(self, edition: &Edition) -> String {
        match self {
            Greenery::Vegetation => format!("luchtfotocir-{}-ndvi", edition.id),
            _ => format!("wcs-{}", edition.id),
        }
    }

    pub fn fetch(self, edition: &Edition, rect: Rect<f64>) -> Result<Grid> {
        match self {
            Greenery::Vegetation => luchtfoto::fetch_tile(&edition.id, rect),
            _ => wcs::fetch(&rivm::coverage(&edition.id), rect),
        }
    }
}

/// The tiles of every raster of this module, taken out of `rasters`.
pub fn take_tiles(
    rasters: &mut HashMap<Dataset, Vec<Grid>>,
    editions: &Editions,
) -> HashMap<Greenery, Vec<Grid>> {
    KINDS
        .into_iter()
        .map(|kind| {
            let dataset = Dataset::Greenery(kind, editions.greenery(kind).clone());
            (kind, rasters.remove(&dataset).unwrap_or_default())
        })
        .collect()
}

/// The bands of one raster from its tiles. Its figures inside `boundary` go
/// into `measured`, with the raster cut to the boundary's box when they found
/// a value; the tree heights reach `TREE_REACH_M` further. Tree cells under
/// one of `buildings` are left out.
pub fn bands(
    kind: Greenery,
    tiles: &[Grid],
    boundary: &Polygon<f64>,
    buildings: &[&Polygon<f64>],
    measured: &mut Measured,
) -> Vec<Feature> {
    let Some(grid) = raster::join(tiles) else {
        return Vec::new();
    };
    if kind == Greenery::GreenClasses {
        return green_class_areas(&grid);
    }
    // The tree map took some roofs and façades for crowns, such as a 60 m "tree" on the TU/e campus.
    // #! NOTE[trees, old-sites] : Sites fetched before this keep those roof "trees" in their canopy bands and figures until fetched again; the woods that block (forest.rs) clear them when drawn
    let grid = match kind {
        Greenery::TreeHeight => forest::without_buildings(&grid, buildings.iter().copied()),
        _ => grid,
    };

    let stats = raster::stats_within(&grid, boundary);
    measured.set(kind.layer(), stats);
    if let (Some(_), Some(bbox)) = (stats, boundary.bounding_rect()) {
        let reach = match kind {
            Greenery::TreeHeight => TREE_REACH_M,
            _ => 0.0,
        };
        let kept = Rect::new(
            bbox.min() - Coord { x: reach, y: reach },
            bbox.max() + Coord { x: reach, y: reach },
        );
        measured.grids.insert(kind.layer(), grid.cropped(kept));
    }
    let (lowest, gaps) = match kind {
        // A cell without a tree is ground, not unknown.
        Greenery::TreeHeight => (Lowest::From(CANOPY_EDGE_M), Gaps::Means(0.0)),
        _ => (Lowest::Everything, Gaps::Unknown),
    };
    bands::banded(&grid, kind.layer(), &breaks_for(kind.layer()), lowest, gaps)
}

/// One area per green class, each traced on its own because the classes have
/// no order to stack them in. Each area carries its group.
fn green_class_areas(grid: &Grid) -> Vec<Feature> {
    let mut features = Vec::new();
    for (code, group) in GREEN_GROUPS {
        let mask = Grid {
            values: grid
                .values
                .iter()
                .map(|value| if *value == code { IN_MASK } else { 0.0 })
                .collect(),
            ..grid.clone()
        };
        let has_class = mask.values.contains(&IN_MASK);
        if !has_class {
            continue;
        }
        let areas = bands::banded(
            &mask,
            Layer::GreenCover,
            &[],
            Lowest::From(MASK_EDGE),
            Gaps::Unknown,
        );
        for mut area in areas {
            area.attributes.value = None;
            area.attributes.group = Some(group.to_string());
            features.push(area);
        }
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::polygon;

    /// One 250 m tile around the Westerkerk of each RIVM map, 2026-10-25.
    const TREES: &[u8] = include_bytes!("../../tests/fixtures/rivm-boomhoogte-westerkerk.tif");
    const GREEN: &[u8] = include_bytes!("../../tests/fixtures/rivm-groenkaart-westerkerk.tif");
    const HEAT: &[u8] = include_bytes!("../../tests/fixtures/rivm-hitteeiland-westerkerk.tif");

    /// The middle 150 m of the fixture tiles.
    fn site() -> Polygon<f64> {
        polygon![
            (x: 120550.0, y: 487550.0),
            (x: 120700.0, y: 487550.0),
            (x: 120700.0, y: 487700.0),
            (x: 120550.0, y: 487700.0),
        ]
    }

    fn tile(body: &[u8]) -> Grid {
        wcs::decode("RIVM", body).unwrap()
    }

    #[test]
    fn tree_heights_become_canopy_bands_and_a_canopy_share() {
        let mut measured = Measured::default();
        let features = bands(
            Greenery::TreeHeight,
            &[tile(TREES)],
            &site(),
            &[],
            &mut measured,
        );

        let values: Vec<f64> = features
            .iter()
            .map(|feature| feature.attributes.value.unwrap())
            .collect();
        assert_eq!(values[0], CANOPY_EDGE_M);
        assert!(values.contains(&10.0), "{values:?}");
        let stats = measured.get(Layer::TreeCanopy).unwrap();
        // The canal elms cover about a fifth of the blocks around the Westerkerk.
        assert!(stats.coverage > 0.1 && stats.coverage < 0.4, "{stats:?}");
        assert!(stats.max <= 25.0, "{stats:?}");
        assert!(measured.grids.contains_key(&Layer::TreeCanopy));
    }

    #[test]
    fn trees_under_a_building_footprint_are_left_out() {
        let mut measured = Measured::default();
        let roof = polygon![
            (x: 120500.0, y: 487500.0),
            (x: 120750.0, y: 487500.0),
            (x: 120750.0, y: 487750.0),
            (x: 120500.0, y: 487750.0),
        ];
        let features = bands(
            Greenery::TreeHeight,
            &[tile(TREES)],
            &site(),
            &[&roof],
            &mut measured,
        );

        assert!(features.is_empty());
    }

    #[test]
    fn green_classes_become_areas_of_their_group() {
        let mut measured = Measured::default();
        let features = bands(
            Greenery::GreenClasses,
            &[tile(GREEN)],
            &site(),
            &[],
            &mut measured,
        );

        assert!(!features.is_empty());
        for feature in &features {
            // The city centre is neither farmland nor protected nature.
            assert_eq!(feature.attributes.group.as_deref(), Some("other_green"));
            assert_eq!(feature.layer, Layer::GreenCover);
        }
        assert_eq!(measured, Measured::default());
    }

    #[test]
    fn the_heat_island_is_measured_in_degrees() {
        let mut measured = Measured::default();
        let features = bands(
            Greenery::HeatIsland,
            &[tile(HEAT)],
            &site(),
            &[],
            &mut measured,
        );

        let stats = measured.get(Layer::HeatIsland).unwrap();
        assert!(stats.mean > 1.9 && stats.mean < 2.7, "{stats:?}");
        assert_eq!(stats.coverage, 1.0);
        assert!(!features.is_empty());
    }

    #[test]
    fn no_tiles_draw_nothing_and_measure_nothing() {
        let mut measured = Measured::default();
        for kind in KINDS {
            assert!(bands(kind, &[], &site(), &[], &mut measured).is_empty());
        }
        assert_eq!(measured, Measured::default());
    }

    #[test]
    #[ignore = "asks RIVM"]
    fn the_live_heat_island_coverage_answers_in_degrees() {
        let edition = Edition {
            year: rivm::HEAT_ISLAND_YEAR,
            id: rivm::HEAT_ISLAND_ID.to_string(),
        };
        let rect = Rect::new((120500.0, 487500.0), (120750.0, 487750.0));
        let grid = Greenery::HeatIsland.fetch(&edition, rect).unwrap();
        assert_eq!(grid.cell_m, 10.0);
        assert!(grid.values.iter().all(|value| (0.0..4.0).contains(value)));
    }
}
