//! The downloaded tiles put back together per source, and the features cut to
//! the boundary and moved into the scene's frame.

use std::collections::{HashMap, HashSet};

use geo::{Geometry, MapCoords, Polygon, Rect};

use super::download::{Dataset, Download, Payload};
use super::editions::Editions;
use super::environment::{Raster, RIVM_RASTERS};
use super::source::{Source, BASE_SOURCES, SOURCES};
use super::{context, environment, greenery, to_local_centimetres};
use crate::environment::Measured;
use crate::scene::{Credit, Feature, Layer};
use crate::sources::ahn::{self, CellTile, Elevation};
use crate::sources::clip::{self, Clipped};
use crate::sources::osm::{self, Osm};
use crate::sources::pdok::Record;
use crate::sources::raster::{self, Grid};
use crate::sources::{bag, bag_units, bgt, brk, cbs, heights};
use crate::Error;

/// The downloaded tiles put back together, per source.
#[derive(Default)]
pub(super) struct Joined {
    pub(super) features: Vec<Feature>,
    pub(super) elevation: Option<Elevation>,
    pub(super) osm: Option<Osm>,
    pub(super) credits: Vec<Credit>,
    pub(super) warnings: Vec<String>,
    pub(super) measured: Measured,
    /// At least one of `BASE_SOURCES` arrived.
    pub(super) has_base: bool,
    /// The neighbourhood around the site, in RD New and not cut yet.
    pub(super) context: Vec<Feature>,
    /// The neighbourhood's bare ground in 2 m cells, RD New, when every tile of it arrived.
    pub(super) context_terrain: Option<Grid>,
}

/// Joins the tiles of each source. A source missing even one tile is left out
/// whole, because a layer with a hole in it would look like real emptiness.
pub(super) fn join(
    mut downloads: Vec<Download>,
    bbox: Rect<f64>,
    boundary: &Polygon<f64>,
    editions: &Editions,
) -> Joined {
    // Tiles finish in any order; sorting keeps the scene the same every time.
    downloads.sort_by_key(|download| (download.tile.row, download.tile.column));

    let mut failed: HashMap<Source, Error> = HashMap::new();
    let mut records: HashMap<Dataset, Vec<Record>> = HashMap::new();
    let mut cells: HashMap<Dataset, Vec<CellTile>> = HashMap::new();
    let mut rasters: HashMap<Dataset, Vec<Grid>> = HashMap::new();
    let mut osm_body: Option<Vec<u8>> = None;
    for download in downloads {
        match download.payload {
            Ok(Payload::Records(tile_records)) => records
                .entry(download.dataset)
                .or_default()
                .extend(tile_records),
            Ok(Payload::Cells(tile_cells)) => cells
                .entry(download.dataset)
                .or_default()
                .push((download.tile.rect(), tile_cells)),
            Ok(Payload::Raster(grid)) => rasters.entry(download.dataset).or_default().push(grid),
            Ok(Payload::Osm(body)) => osm_body = Some(body),
            Err(error) => {
                failed.entry(download.dataset.source()).or_insert(error);
            }
        }
    }

    let mut joined = Joined::default();
    // Before the site takes its records: the neighbourhood reads the site's
    // tiles too. Without all of them it would have a hole that looks real.
    let has_context = [Source::Context, Source::Bgt, Source::Bag]
        .iter()
        .all(|source| !failed.contains_key(source));
    if has_context {
        // Joined before the neighbourhood takes its rasters, for the section profile.
        joined.context_terrain = rasters
            .get(&Dataset::ContextTerrain)
            .and_then(|tiles| raster::join(tiles));
        joined.context = context::features(&records, &mut rasters);
    }
    let greenery_tiles = greenery::take_tiles(&mut rasters, editions);
    let mut take = |dataset| unique(records.remove(&dataset).unwrap_or_default());
    // Units first, so the buildings can count them.
    let units = (!failed.contains_key(&Source::BagUnits))
        .then(|| bag_units::totals_per_building(&take(Dataset::BagUnits)));
    let mut bands_of = |raster: Raster, joined: &mut Joined| {
        let dataset = Dataset::Raster(raster, editions.raster(raster).clone());
        let tiles = rasters.remove(&dataset).unwrap_or_default();
        let features = environment::raster_bands(raster, &tiles, boundary, &mut joined.measured);
        joined.features.extend(features);
    };
    for source in SOURCES {
        if let Some(error) = failed.remove(&source) {
            joined
                .warnings
                .push(format!("{} could not be fetched: {error}", source.step()));
            continue;
        }

        match source {
            Source::Bgt => {
                for collection in bgt::COLLECTIONS {
                    let tile_records = take(Dataset::Bgt(collection));
                    joined
                        .features
                        .extend(bgt::to_features(collection, tile_records));
                }
            }
            Source::Bag => joined
                .features
                .extend(bag::to_features(take(Dataset::Bag), units.as_ref())),
            Source::Brk => joined.features.extend(brk::to_features(take(Dataset::Brk))),
            Source::Ahn => {
                let surface = cells.remove(&Dataset::AhnSurface).unwrap_or_default();
                let terrain = cells.remove(&Dataset::AhnTerrain).unwrap_or_default();
                joined.elevation = Some(Elevation {
                    surface: ahn::assemble(bbox, &surface),
                    terrain: ahn::assemble(bbox, &terrain),
                });
            }
            Source::Rivm => {
                for raster in RIVM_RASTERS {
                    bands_of(raster, &mut joined);
                }
            }
            Source::Liwo => bands_of(Raster::Flood, &mut joined),
            Source::Cbs => {
                let years = cbs::years(editions.population).map(|year| take(Dataset::Cbs(year)));
                joined.features.extend(cbs::to_features(years.into()));
            }
            Source::Green | Source::Vegetation | Source::Heat => {
                // The site's and the neighbourhood's buildings, joined above, to clear the tree map's roofs.
                let buildings = footprints(&joined);
                let buildings: Vec<&Polygon<f64>> = buildings.iter().collect();
                for kind in greenery::KINDS
                    .iter()
                    .filter(|kind| kind.source() == source)
                {
                    let tiles = greenery_tiles.get(kind).map(Vec::as_slice).unwrap_or(&[]);
                    let features =
                        greenery::bands(*kind, tiles, boundary, &buildings, &mut joined.measured);
                    joined.features.extend(features);
                }
            }
            Source::BagUnits | Source::Context => {}
            Source::Osm => match osm::parse(&osm_body.take().unwrap_or_default()) {
                Ok(parsed) => joined.osm = Some(parsed),
                Err(error) => {
                    joined
                        .warnings
                        .push(format!("{} could not be read: {error}", source.step()));
                    continue;
                }
            },
        }
        joined.has_base |= BASE_SOURCES.contains(&source);
        joined.credits.extend(source.credit());
    }

    joined
}

/// Each PDOK feature once. A feature crossing a tile edge comes back from
/// every tile it touches, always under the same id.
// #! NOTE[cache, dedupe] : Does PDOK give an edited feature a new id? Then tiles cached days apart could show both versions
pub(super) fn unique(records: Vec<Record>) -> Vec<Record> {
    let mut seen = HashSet::new();
    records
        .into_iter()
        .filter(|record| match &record.id {
            Some(id) => seen.insert(id.clone()),
            None => true,
        })
        .collect()
}

/// Every building footprint joined so far, in the site and around it.
fn footprints(joined: &Joined) -> Vec<Polygon<f64>> {
    joined
        .features
        .iter()
        .chain(&joined.context)
        .filter(|feature| feature.layer == Layer::Building)
        .flat_map(|feature| match &feature.geometry {
            Geometry::Polygon(polygon) => vec![polygon.clone()],
            Geometry::MultiPolygon(multi) => multi.0.clone(),
            _ => Vec::new(),
        })
        .collect()
}

// Heights use the whole footprint, before clipping, so a building cut by the
// boundary keeps the height of the whole building.
pub(super) fn add_building_heights(features: &mut [Feature], elevation: &Elevation) {
    for feature in features
        .iter_mut()
        .filter(|feature| feature.layer == Layer::Building)
    {
        if let Geometry::Polygon(footprint) = &feature.geometry {
            let (height_m, ground_m) = heights::building_height(footprint, elevation);
            feature.attributes.height_m = height_m;
            feature.attributes.ground_m = ground_m;
        }
    }
}

/// Features cut to `boundary` and moved into the local frame, plus how many
/// had shapes too broken to cut.
pub(super) fn clip_and_localise(
    features: Vec<Feature>,
    boundary: &Polygon<f64>,
    origin: (f64, f64),
) -> (Vec<Feature>, usize) {
    let mut kept = Vec::with_capacity(features.len());
    let mut unclippable = 0;
    for feature in features {
        match clip::clip(boundary, feature.geometry) {
            Clipped::Inside(geometry) => kept.push(Feature {
                geometry: geometry.map_coords(|coord| to_local_centimetres(coord, origin)),
                ..feature
            }),
            Clipped::Outside => {}
            Clipped::Broken => unclippable += 1,
        }
    }

    (kept, unclippable)
}
