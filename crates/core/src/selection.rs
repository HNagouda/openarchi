//! A sheet opened from a highlight: a smaller area inside the fetched site
//! that the sheet frames and clips to.
//!
//! The scene is cut to the smaller area and then drawn, measured and exported
//! exactly like a whole site, so there is no second renderer to drift, the
//! drawing zooms to the area, and the ratios and the measured noise, air and
//! flood figures describe it. The walk is kept whole, because it starts at the
//! site centre and never stopped at the site edge either. The rest of the site
//! joins the neighbourhood, so a zoomed-in sheet shows it muted around the area
//! rather than a blank gap before the ring.

use std::borrow::Cow;
use std::collections::BTreeSet;

use geo::{Area as _, BooleanOps, Contains, Coord, MapCoords, Polygon};

use crate::area::Area;
use crate::environment;
use crate::scene::{Feature, Layer, Scene};
use crate::site::boundary_in_rd;
use crate::sources::clip::{self, Clipped};
use crate::{Error, Result};

/// A sheet area must keep at least this much of the site, in square
/// metres: less is a slip of the mouse, not an area worth a sheet.
const MIN_AREA_M2: f64 = 25.0;

/// The scene as `selection` frames it, or the whole scene when there is none.
pub fn framed_or_whole<'a>(scene: &'a Scene, selection: Option<&Area>) -> Result<Cow<'a, Scene>> {
    match selection {
        Some(selection) => Ok(Cow::Owned(framed(scene, selection)?)),
        None => Ok(Cow::Borrowed(scene)),
    }
}

/// `scene` cut to `selection`. The part of the selection outside the site is
/// dropped, since there is no data there to draw.
pub fn framed(scene: &Scene, selection: &Area) -> Result<Scene> {
    let (origin_x, origin_y) = scene.origin;
    let drawn = boundary_in_rd(selection)?.map_coords(|coord| Coord {
        x: coord.x - origin_x,
        y: coord.y - origin_y,
    });
    let boundary = largest_part(&scene.boundary, &drawn).ok_or_else(|| {
        Error::Invalid(
            "The highlight lies outside the fetched site. Draw it inside the site.".to_string(),
        )
    })?;

    let mut framed = scene.clone();
    framed.area = selection.clone();
    framed.features = clipped(&scene.features, &boundary);
    framed.context = surroundings(scene, &boundary);
    framed.measured = environment::measured_within(&scene.measured, &boundary);
    framed.boundary = boundary;
    Ok(framed)
}

/// The biggest piece where the site and the selection overlap, if it is big enough to draw.
fn largest_part(site: &Polygon<f64>, selection: &Polygon<f64>) -> Option<Polygon<f64>> {
    site.intersection(selection)
        .0
        .into_iter()
        .filter(|part| part.unsigned_area() >= MIN_AREA_M2)
        .max_by(|a, b| a.unsigned_area().total_cmp(&b.unsigned_area()))
}

fn clipped(features: &[Feature], boundary: &Polygon<f64>) -> Vec<Feature> {
    let mut kept = Vec::new();
    for feature in features {
        // A shape too broken to cut was already counted when the site was fetched.
        if let Clipped::Inside(geometry) = clip::clip(boundary, feature.geometry.clone()) {
            kept.push(Feature {
                geometry,
                ..feature.clone()
            });
        }
    }

    kept
}

/// The neighbourhood of a framed sheet: the ring around the site, and every
/// site feature of the ring's layers that reaches outside `boundary`. A scene
/// fetched without a neighbourhood keeps none, as its whole-site sheets do.
fn surroundings(scene: &Scene, boundary: &Polygon<f64>) -> Vec<Feature> {
    let layers: BTreeSet<Layer> = scene.context.iter().map(|feature| feature.layer).collect();
    let mut around = scene.context.clone();
    for feature in &scene.features {
        if layers.contains(&feature.layer) && !boundary.contains(&feature.geometry) {
            around.push(feature.clone());
        }
    }

    around
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crs::rd_to_wgs84;
    use crate::scene::{Attributes, Layer};
    use crate::sources::raster::{stats_within, Grid, Stats};
    use geo::{Geometry, Point};

    const ORIGIN: (f64, f64) = (120_900.0, 487_400.0);
    /// 10 m cells over the whole 200 m site.
    const CELLS: usize = 20;
    const CELL_M: f64 = 10.0;
    const SLOPE: Stats = Stats {
        min: 0.0,
        mean: 1.0,
        max: 4.0,
        coverage: 1.0,
    };

    /// A raster over the site whose cells each hold their own number, so every area has its own mean.
    fn gradient() -> Grid {
        Grid {
            west: -100.0,
            north: 100.0,
            cell_m: CELL_M,
            width: CELLS,
            height: CELLS,
            values: (0..CELLS * CELLS).map(|index| index as f32).collect(),
        }
    }

    /// A flood raster where only the north-east quarter of the site floods.
    fn north_east_flood() -> Grid {
        let mut flood = gradient();
        for row in 0..CELLS {
            for column in 0..CELLS {
                if row >= CELLS / 2 || column < CELLS / 2 {
                    flood.values[row * CELLS + column] = f32::NAN;
                }
            }
        }

        flood
    }

    /// A 200 m square site around the origin, with one tree inside and one near its corner.
    fn site() -> Scene {
        let square: Polygon<f64> = Polygon::new(
            vec![
                (-100.0, -100.0),
                (100.0, -100.0),
                (100.0, 100.0),
                (-100.0, 100.0),
            ]
            .into(),
            vec![],
        );
        let tree = |x: f64, y: f64| Feature {
            layer: Layer::Tree,
            geometry: Geometry::Point(Point::new(x, y)),
            attributes: Attributes::default(),
        };
        Scene {
            area: Area::Circle {
                center: rd_to_wgs84(ORIGIN.0, ORIGIN.1),
                radius_m: 100.0,
            },
            crs: "EPSG:28992".to_string(),
            origin: ORIGIN,
            boundary: square,
            features: vec![tree(0.0, 0.0), tree(90.0, 90.0)],
            credits: Vec::new(),
            fetched_at: String::new(),
            warnings: Vec::new(),
            measured: Default::default(),
            walk: None,
            editions: Default::default(),
            context: vec![tree(300.0, 0.0)],
            terrain: None,
            water_bed: None,
            water_levels: Vec::new(),
            solar: None,
            wind: None,
        }
    }

    fn circle_at(x: f64, y: f64, radius_m: f64) -> Area {
        Area::Circle {
            center: rd_to_wgs84(ORIGIN.0 + x, ORIGIN.1 + y),
            radius_m,
        }
    }

    #[test]
    fn a_selection_keeps_only_the_features_inside_it() {
        let framed = framed(&site(), &circle_at(0.0, 0.0, 30.0)).unwrap();
        assert_eq!(framed.features.len(), 1);
        let area = framed.boundary.unsigned_area();
        let circle = std::f64::consts::PI * 30.0 * 30.0;
        assert!((area - circle).abs() / circle < 0.01, "{area}");
    }

    #[test]
    fn a_selection_reaching_past_the_site_is_cut_to_it() {
        let framed = framed(&site(), &circle_at(100.0, 100.0, 40.0)).unwrap();
        // A quarter of the circle lies inside the site's corner.
        let quarter = std::f64::consts::PI * 40.0 * 40.0 / 4.0;
        let area = framed.boundary.unsigned_area();
        assert!((area - quarter).abs() / quarter < 0.02, "{area}");
        assert_eq!(framed.features.len(), 1);
    }

    #[test]
    fn a_selection_outside_the_site_is_a_readable_error() {
        let error = framed(&site(), &circle_at(500.0, 0.0, 30.0)).unwrap_err();
        assert!(error.to_string().contains("outside the fetched site"));
    }

    #[test]
    fn the_rest_of_the_site_joins_the_neighbourhood() {
        let framed = framed(&site(), &circle_at(0.0, 0.0, 30.0)).unwrap();
        // The ring's own tree, and the site's tree in the corner, outside the selection.
        assert_eq!(framed.context.len(), 2);
    }

    #[test]
    fn a_sheet_area_measures_its_figures_again_from_the_kept_rasters() {
        let mut scene = site();
        let (noise, flood) = (gradient(), north_east_flood());
        scene.measured.road_noise_db = stats_within(&noise, &scene.boundary);
        scene.measured.flood_depth_m = stats_within(&flood, &scene.boundary);
        scene.measured.slope_percent = Some(SLOPE);
        scene.measured.grids.insert(Layer::RoadNoise, noise.clone());
        scene.measured.grids.insert(Layer::Flood, flood);

        let framed = framed(&scene, &circle_at(-50.0, -50.0, 30.0)).unwrap();
        let measured = &framed.measured;
        assert_eq!(
            measured.road_noise_db,
            stats_within(&noise, &framed.boundary)
        );
        assert_ne!(measured.road_noise_db, scene.measured.road_noise_db);
        // The south-west corner stays dry.
        assert_eq!(measured.flood_depth_m, None);
        // The slope raster is never kept.
        assert_eq!(measured.slope_percent, Some(SLOPE));
        assert_eq!(measured.whole_site, vec![Layer::Slope]);
    }

    #[test]
    fn an_old_scene_keeps_its_whole_site_figures_and_says_so() {
        let mut scene = site();
        scene.measured.no2_ug_m3 = stats_within(&gradient(), &scene.boundary);
        scene.measured.slope_percent = Some(SLOPE);

        let framed = framed(&scene, &circle_at(-50.0, -50.0, 30.0)).unwrap();
        assert_eq!(framed.measured.no2_ug_m3, scene.measured.no2_ug_m3);
        assert_eq!(framed.measured.whole_site, vec![Layer::No2, Layer::Slope]);
        assert!(scene.measured.whole_site.is_empty());
    }

    #[test]
    fn no_selection_borrows_the_whole_scene() {
        let scene = site();
        assert!(matches!(
            framed_or_whole(&scene, None).unwrap(),
            Cow::Borrowed(_)
        ));
    }
}
