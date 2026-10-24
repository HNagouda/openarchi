//! A sheet's sub-selection: a smaller area inside the fetched site that the
//! sheet frames and clips to.
//!
//! The scene is cut to the smaller area and then drawn, measured and exported
//! exactly like a whole site, so there is no second renderer to drift, the
//! drawing zooms to the area, and the ratios describe it. The walk is kept
//! whole, because it starts at the site centre and never stopped at the site
//! edge either.

use std::borrow::Cow;

use geo::{Area as _, BooleanOps, Coord, MapCoords, Polygon};

use crate::area::Area;
use crate::scene::{Feature, Scene};
use crate::site::boundary_in_rd;
use crate::sources::clip::{self, Clipped};
use crate::{Error, Result};

/// A sub-selection must keep at least this much of the site, in square
/// metres: less is a slip of the mouse, not an area worth a sheet.
const MIN_AREA_M2: f64 = 25.0;

/// The scene as `selection` frames it, or the whole scene when there is none.
pub fn framed_or_whole<'a>(scene: &'a Scene, selection: Option<&Area>) -> Result<Cow<'a, Scene>> {
    match selection {
        Some(selection) => Ok(Cow::Owned(framed(scene, selection)?)),
        None => Ok(Cow::Borrowed(scene)),
    }
}

// #! NOTE[selection, environment] : Measured noise, air and flood figures in `scene.measured` still describe the whole site
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
            "The sub-selection lies outside the fetched site. Draw it inside the site.".to_string(),
        )
    })?;

    let mut framed = scene.clone();
    framed.area = selection.clone();
    framed.features = clipped(&scene.features, &boundary);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crs::rd_to_wgs84;
    use crate::scene::{Attributes, Layer};
    use geo::{Geometry, Point};

    const ORIGIN: (f64, f64) = (120_900.0, 487_400.0);

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
            context: Vec::new(),
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
    fn no_selection_borrows_the_whole_scene() {
        let scene = site();
        assert!(matches!(
            framed_or_whole(&scene, None).unwrap(),
            Cow::Borrowed(_)
        ));
    }
}
