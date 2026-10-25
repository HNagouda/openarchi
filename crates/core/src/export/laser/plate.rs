//! The base plate: the site boundary to cut, with roads, water, green, every
//! building footprint and the buildings' ids scored on it.
//!
//! A plate larger than the bed is cut into equal tiles along straight lines,
//! each engraved "T1", "T2" and on, numbered row by row from the north-west.

use std::panic::{self, AssertUnwindSafe};

use geo::{
    BooleanOps, BoundingRect, Coord, Geometry, InteriorPoint, LineString, MapCoords,
    MultiLineString, MultiPolygon, Polygon, Rect, Simplify,
};

use super::glyphs::{centred_label, fitted_label, LABEL_MIN_MM};
use super::pieces::{self, Footprint};
use super::{Frame, LaserSettings, Part};
use crate::scene::{Layer, Scene};
use crate::sources::clip::{self, Clipped};
use crate::{Error, Result};

// #! NOTE[laser, tiles] : A plate larger than the bed is split into equal tiles along straight lines, which may cross buildings; would students rather be told to pick a smaller scale, or have the seams follow streets?
// #! NOTE[laser, terrain] : Contours are scored on the plate, not stacked as terrain layers, since a Dutch site rarely rises one board at 1:1000; stack them from Scene::terrain for hilly sites?

/// The layers scored on the plate as the outline of their merged areas, in one place.
const SCORED_AREAS: [Layer; 3] = [Layer::Road, Layer::Water, Layer::Green];
/// Contour lines are simplified like footprints, to what the laser can score.
const SIMPLIFY_MM: f64 = 0.2;
/// Tile ids are larger than building ids, so they read from arm's length.
const TILE_LABEL_MM: f64 = 4.0;

/// The plate as one part, or as tiles that each fit `usable` (width, height).
pub fn tiles(
    scene: &Scene,
    frame: Frame,
    footprints: &[Footprint],
    settings: &LaserSettings,
    usable: (f64, f64),
) -> Result<Vec<Part>> {
    let boundary = pieces::to_model(&scene.boundary, frame).ok_or_else(|| {
        Error::Invalid(format!(
            "The site is too small to cut at 1:{}.",
            settings.scale
        ))
    })?;
    let lines = scored_lines(scene, frame, footprints, settings.has_contours);
    let rect = boundary
        .bounding_rect()
        .expect("a kept boundary has points");
    let columns = (rect.width() / usable.0).ceil().max(1.0) as usize;
    let rows = (rect.height() / usable.1).ceil().max(1.0) as usize;
    if columns == 1 && rows == 1 {
        return Ok(vec![part(&[boundary], &lines, None)]);
    }

    let tile_width = rect.width() / columns as f64;
    let tile_height = rect.height() / rows as f64;
    let mut tiles = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let west = rect.min().x + column as f64 * tile_width;
            let north = rect.min().y + row as f64 * tile_height;
            let tile = Rect::new(
                Coord { x: west, y: north },
                Coord {
                    x: west + tile_width,
                    y: north + tile_height,
                },
            )
            .to_polygon();
            let cut = match clip::clip(&tile, Geometry::Polygon(boundary.clone())) {
                Clipped::Inside(geometry) => polygons(geometry),
                Clipped::Outside => continue,
                Clipped::Broken => return Err(broken()),
            };
            let scored = match clip::clip(&tile, Geometry::MultiLineString(lines.clone())) {
                Clipped::Inside(geometry) => lines_of(geometry),
                Clipped::Outside => MultiLineString::new(Vec::new()),
                Clipped::Broken => return Err(broken()),
            };
            let number = tiles.len() + 1;
            tiles.push(part(&cut, &scored, Some(&format!("T{number}"))));
        }
    }

    Ok(tiles)
}

fn broken() -> Error {
    Error::Invalid(
        "The base plate could not be split into tiles. Try a smaller scale or a larger bed."
            .to_string(),
    )
}

/// One part from its outlines and score lines in plate millimetres, moved so
/// its top-left corner sits at (0, 0), with `label` engraved in its largest piece.
fn part(cut: &[Polygon<f64>], scored: &MultiLineString<f64>, label: Option<&str>) -> Part {
    let outlines = MultiPolygon::new(cut.to_vec());
    let rect = outlines
        .bounding_rect()
        .expect("a plate part has an outline");
    let (west, north) = (rect.min().x, rect.min().y);
    let shift = |coord: Coord<f64>| Coord {
        x: coord.x - west,
        y: coord.y - north,
    };
    let shifted = outlines.map_coords(shift);

    let mut engrave: Vec<Vec<(f64, f64)>> = scored
        .map_coords(shift)
        .0
        .iter()
        .map(|line| line.0.iter().map(|coord| (coord.x, coord.y)).collect())
        .collect();
    let largest = shifted
        .0
        .iter()
        .max_by(|a, b| geo::Area::unsigned_area(*a).total_cmp(&geo::Area::unsigned_area(*b)));
    if let (Some(text), Some(polygon)) = (label, largest) {
        if let Some(point) = polygon.interior_point() {
            engrave.extend(centred_label(text, point.x(), point.y(), TILE_LABEL_MM));
        }
    }

    Part {
        cut: shifted.0.iter().flat_map(pieces::rings).collect(),
        engrave,
        width: rect.width(),
        height: rect.height(),
        area_mm2: geo::Area::unsigned_area(&shifted),
    }
}

/// Everything scored on the plate, in plate millimetres.
fn scored_lines(
    scene: &Scene,
    frame: Frame,
    footprints: &[Footprint],
    has_contours: bool,
) -> MultiLineString<f64> {
    let mut lines = Vec::new();
    for layer in SCORED_AREAS {
        let areas: Vec<Polygon<f64>> = scene
            .features
            .iter()
            .filter(|feature| feature.layer == layer)
            .flat_map(|feature| polygons(feature.geometry.clone()))
            .collect();
        for polygon in merged(areas) {
            if let Some(model) = pieces::to_model(&polygon, frame) {
                lines.extend(ring_lines(&model));
            }
        }
    }

    for footprint in footprints {
        lines.extend(ring_lines(&footprint.polygon));
        if let Some(id) = footprint.id {
            lines.extend(footprint_label(&footprint.polygon, &format!("B{id}")));
        }
    }

    if has_contours {
        for feature in &scene.features {
            if feature.layer == Layer::Contour {
                for line in lines_of(feature.geometry.clone()) {
                    lines.push(to_model_line(&line, frame));
                }
            }
        }
    }

    MultiLineString::new(lines)
}

/// The id inside its footprint, or at its smallest size over it when it fits nowhere.
fn footprint_label(polygon: &Polygon<f64>, text: &str) -> Vec<LineString<f64>> {
    let placed = fitted_label(polygon, text).or_else(|| {
        polygon
            .interior_point()
            .map(|point| ((point.x(), point.y()), LABEL_MIN_MM))
    });
    let Some(((x, y), height)) = placed else {
        return Vec::new();
    };

    centred_label(text, x, y, height)
        .into_iter()
        .map(LineString::from)
        .collect()
}

/// `polygons` merged where they touch, so a road made of many BGT pieces is
/// scored as one outline instead of a line across it at every seam.
fn merged(polygons: Vec<Polygon<f64>>) -> Vec<Polygon<f64>> {
    let mut merged = MultiPolygon::new(Vec::new());
    let mut loose = Vec::new();
    for polygon in polygons {
        // geo's boolean operations can panic on degenerate input. Such a
        // polygon is scored on its own rather than lost.
        match panic::catch_unwind(AssertUnwindSafe(|| merged.union(&polygon))) {
            Ok(union) => merged = union,
            Err(_) => loose.push(polygon),
        }
    }

    merged.0.into_iter().chain(loose).collect()
}

fn ring_lines(polygon: &Polygon<f64>) -> Vec<LineString<f64>> {
    let mut lines = vec![polygon.exterior().clone()];
    lines.extend(polygon.interiors().iter().cloned());
    lines
}

fn to_model_line(line: &LineString<f64>, frame: Frame) -> LineString<f64> {
    line.map_coords(|coord| {
        let (x, y) = frame.to_mm(coord.x, coord.y);
        Coord { x, y }
    })
    .simplify(&SIMPLIFY_MM)
}

fn polygons(geometry: Geometry<f64>) -> Vec<Polygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(multi) => multi.0,
        _ => Vec::new(),
    }
}

fn lines_of(geometry: Geometry<f64>) -> MultiLineString<f64> {
    match geometry {
        Geometry::LineString(line) => MultiLineString::new(vec![line]),
        Geometry::MultiLineString(lines) => lines,
        _ => MultiLineString::new(Vec::new()),
    }
}
