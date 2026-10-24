//! Buildings as stacks of identical footprint pieces, each with its id.

use geo::{
    Area, BoundingRect, Coord, Geometry, LineString, MapCoords, MinimumRotatedRect, Point, Polygon,
    Rotate, Simplify,
};

use super::glyphs::{self, centred_label, fitted_label, LABEL_MIN_MM};
use super::{Frame, LaserSettings, Part};
use crate::render::DEFAULT_HEIGHT_M;
use crate::scene::{Layer, Scene};

/// Wiggles in a footprint smaller than this, in model millimetres, are finer
/// than a laser cuts cleanly, so the outline is simplified to this tolerance.
const SIMPLIFY_MM: f64 = 0.2;
/// A footprint narrower than this on the model is left out: a piece that
/// small is lost in the laser's grate or burns through.
const MIN_PIECE_MM: f64 = 2.0;
/// Ids run in rows this many ground metres tall, north to south and west to
/// east in each row, so B12 lies near B11 on the base plate.
const ROW_M: f64 = 25.0;
const QUARTER_TURN_DEG: f64 = 90.0;
/// A label that fits inside no footprint is engraved this far below its piece.
const LABEL_GAP_MM: f64 = 0.5;

// #! NOTE[laser, kerf] : Outlines are cut on the line, without kerf compensation, so pieces come out about 0.1 mm small on each side; the laser software usually offers an offset if a shop wants one.

/// A building's footprint on the base plate, in plate millimetres, with the
/// id its pieces carry. Footprints too small to cut have no id.
#[derive(Debug, Clone)]
pub struct Footprint {
    pub id: Option<usize>,
    pub polygon: Polygon<f64>,
}

pub struct Buildings {
    pub pieces: Vec<Part>,
    pub footprints: Vec<Footprint>,
    pub building_count: usize,
    pub dropped: usize,
    pub estimated_heights: usize,
}

/// How many boards make up a building `height_m` tall: never fewer than one,
/// so every building stands on the model. 12 m at 1:1000 in 2 mm board -> 6.
pub fn layer_count(height_m: f64, scale: u32, thickness_mm: f64) -> u32 {
    let layers = (height_m * 1000.0 / scale as f64 / thickness_mm).round();
    layers.max(1.0) as u32
}

/// Every building of the scene as pieces, and its footprint for the plate.
pub fn buildings(scene: &Scene, frame: Frame, settings: &LaserSettings) -> Buildings {
    // (footprint in mm, height in metres, whether the height is the default guess)
    let mut shapes: Vec<(Polygon<f64>, f64, bool)> = Vec::new();
    for feature in &scene.features {
        if feature.layer != Layer::Building {
            continue;
        }
        let (height, is_estimated) = match feature.attributes.height_m {
            Some(height) if height > 0.0 => (height, false),
            _ => (DEFAULT_HEIGHT_M, true),
        };
        for polygon in polygons(&feature.geometry) {
            if let Some(model) = to_model(polygon, frame) {
                shapes.push((model, height, is_estimated));
            }
        }
    }
    let row_mm = ROW_M * frame.mm_per_m;
    shapes.sort_by(|a, b| {
        let (row_a, x_a) = reading_order(&a.0, row_mm);
        let (row_b, x_b) = reading_order(&b.0, row_mm);
        row_a.total_cmp(&row_b).then(x_a.total_cmp(&x_b))
    });

    let mut buildings = Buildings {
        pieces: Vec::new(),
        footprints: Vec::new(),
        building_count: 0,
        dropped: 0,
        estimated_heights: 0,
    };
    for (polygon, height, is_estimated) in shapes {
        if !is_big_enough(&polygon) {
            buildings.dropped += 1;
            buildings.footprints.push(Footprint { id: None, polygon });
            continue;
        }
        buildings.building_count += 1;
        let id = buildings.building_count;
        if is_estimated {
            buildings.estimated_heights += 1;
        }
        let layers = layer_count(height, settings.scale, settings.thickness_mm);
        buildings.pieces.extend(stack(&polygon, id, layers));
        buildings.footprints.push(Footprint {
            id: Some(id),
            polygon,
        });
    }

    buildings
}

fn polygons(geometry: &Geometry<f64>) -> Vec<&Polygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(multi) => multi.0.iter().collect(),
        _ => Vec::new(),
    }
}

/// A ground polygon in model millimetres, simplified to what the laser can
/// cut. `None` when nothing worth cutting is left.
pub fn to_model(polygon: &Polygon<f64>, frame: Frame) -> Option<Polygon<f64>> {
    let model = polygon
        .map_coords(|coord| {
            let (x, y) = frame.to_mm(coord.x, coord.y);
            Coord { x, y }
        })
        .simplify(&SIMPLIFY_MM);
    if model.exterior().0.len() < 4 || model.unsigned_area() == 0.0 {
        return None;
    }
    let holes: Vec<LineString<f64>> = model
        .interiors()
        .iter()
        .filter(|ring| ring.0.len() >= 4)
        .cloned()
        .collect();

    Some(Polygon::new(model.exterior().clone(), holes))
}

/// The row and the x of a footprint's centre, the order ids are given in.
fn reading_order(polygon: &Polygon<f64>, row_mm: f64) -> (f64, f64) {
    let centre = polygon
        .bounding_rect()
        .map(|rect| rect.center())
        .unwrap_or_default();
    ((centre.y / row_mm).floor(), centre.x)
}

fn is_big_enough(polygon: &Polygon<f64>) -> bool {
    polygon
        .bounding_rect()
        .is_some_and(|rect| rect.width() >= MIN_PIECE_MM && rect.height() >= MIN_PIECE_MM)
}

/// `layers` identical pieces of `footprint`, ids "B12·1/5" to "B12·5/5".
fn stack(footprint: &Polygon<f64>, id: usize, layers: u32) -> Vec<Part> {
    let footprint = lying_flat(footprint);
    let rect = footprint
        .bounding_rect()
        .expect("a kept footprint has points");
    let shifted = footprint.map_coords(|coord| Coord {
        x: coord.x - rect.min().x,
        y: coord.y - rect.min().y,
    });
    let cut = rings(&shifted);
    let area_mm2 = shifted.unsigned_area();
    // The last label is the widest, so whatever holds it holds them all.
    let widest = format!("B{id}·{layers}/{layers}");
    let inside = fitted_label(&shifted, &widest);

    let mut pieces = Vec::new();
    for layer in 1..=layers {
        let text = format!("B{id}·{layer}/{layers}");
        let (width, height, engrave) = match inside {
            Some(((x, y), size)) => (
                rect.width(),
                rect.height(),
                centred_label(&text, x, y, size),
            ),
            None => {
                let top = rect.height() + LABEL_GAP_MM;
                let label_width = glyphs::text_width(&text, LABEL_MIN_MM);
                (
                    rect.width().max(label_width),
                    top + LABEL_MIN_MM,
                    glyphs::strokes(&text, 0.0, top, LABEL_MIN_MM),
                )
            }
        };
        pieces.push(Part {
            cut: cut.clone(),
            engrave,
            width,
            height,
            area_mm2,
        });
    }

    pieces
}

/// `footprint` turned so its tightest enclosing rectangle lies square and
/// wide. Streets rarely run north-south, and a footprint left at its angle
/// takes a bounding box up to twice its size on the bed. Turning a piece
/// changes nothing for gluing: its outline still matches the plate's.
fn lying_flat(footprint: &Polygon<f64>) -> Polygon<f64> {
    let Some(tightest) = footprint.minimum_rotated_rect() else {
        return footprint.clone();
    };
    let corners = &tightest.exterior().0;
    let (start, end) = (corners[0], corners[1]);
    let angle = (end.y - start.y).atan2(end.x - start.x).to_degrees();
    let turned = footprint.rotate_around_point(-angle, Point::new(0.0, 0.0));

    let is_tall = turned
        .bounding_rect()
        .is_some_and(|rect| rect.height() > rect.width());
    if is_tall {
        turned.rotate_around_point(QUARTER_TURN_DEG, Point::new(0.0, 0.0))
    } else {
        turned
    }
}

/// Every ring of a polygon as closed point lists, holes first so the laser
/// cuts them before the piece falls free.
pub fn rings(polygon: &Polygon<f64>) -> Vec<Vec<(f64, f64)>> {
    let mut rings: Vec<Vec<(f64, f64)>> = polygon
        .interiors()
        .iter()
        .map(|ring| ring.0.iter().map(|coord| (coord.x, coord.y)).collect())
        .collect();
    rings.push(
        polygon
            .exterior()
            .0
            .iter()
            .map(|coord| (coord.x, coord.y))
            .collect(),
    );

    rings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side_mm: f64) -> Polygon<f64> {
        Polygon::new(
            vec![
                (0.0, 0.0),
                (side_mm, 0.0),
                (side_mm, side_mm),
                (0.0, side_mm),
            ]
            .into(),
            vec![],
        )
    }

    #[test]
    fn a_building_gets_one_board_per_thickness_of_its_height() {
        // 12 m at 1:1000 is 12 mm, six 2 mm boards or four 3 mm ones.
        assert_eq!(layer_count(12.0, 1000, 2.0), 6);
        assert_eq!(layer_count(12.0, 1000, 3.0), 4);
        // 13 m in 2 mm board is 6.5 boards, rounded half away from zero.
        assert_eq!(layer_count(13.0, 1000, 2.0), 7);
        // 3 m at 1:2500 is 1.2 mm, still one board so the building stands.
        assert_eq!(layer_count(3.0, 2500, 2.0), 1);
        assert_eq!(layer_count(0.2, 2500, 3.0), 1);
    }

    #[test]
    fn every_layer_of_a_stack_is_the_same_piece_with_its_own_id() {
        let pieces = stack(&square(20.0), 12, 5);
        assert_eq!(pieces.len(), 5);
        for piece in &pieces {
            assert_eq!(piece.cut, pieces[0].cut);
            assert!((piece.width - 20.0).abs() < 1e-9 && (piece.height - 20.0).abs() < 1e-9);
        }
        assert_ne!(pieces[0].engrave, pieces[4].engrave);
    }

    #[test]
    fn a_label_too_wide_for_its_piece_goes_below_it() {
        let pieces = stack(&square(3.0), 120, 12);
        let piece = &pieces[0];
        assert!(piece.height > 3.0 + LABEL_GAP_MM);
        let lowest = piece
            .engrave
            .iter()
            .flatten()
            .map(|p| p.1)
            .fold(0.0, f64::max);
        assert!((lowest - piece.height).abs() < 1e-9);
    }

    #[test]
    fn a_slanted_footprint_is_turned_to_lie_square_and_wide() {
        // A 4 by 10 rectangle standing at 30 degrees.
        let slanted = Polygon::new(
            vec![(0.0, 0.0), (4.0, 0.0), (4.0, 10.0), (0.0, 10.0)].into(),
            vec![],
        )
        .rotate_around_point(30.0, Point::new(0.0, 0.0));
        let rect = lying_flat(&slanted).bounding_rect().unwrap();
        assert!((rect.width() - 10.0).abs() < 1e-6, "{rect:?}");
        assert!((rect.height() - 4.0).abs() < 1e-6, "{rect:?}");
    }

    #[test]
    fn wiggles_below_the_laser_resolution_are_simplified_away() {
        let frame = Frame {
            mm_per_m: 1.0,
            west: 0.0,
            north: 10.0,
        };
        // A 10 m square at 1:1000 with a 5 cm notch in one side, 0.05 mm on the model.
        let notched = Polygon::new(
            vec![
                (0.0, 0.0),
                (5.0, 0.0),
                (5.0, 0.05),
                (5.1, 0.05),
                (5.1, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
            ]
            .into(),
            vec![],
        );
        let model = to_model(&notched, frame).unwrap();
        assert_eq!(model.exterior().0.len(), 5);
    }
}
