//! Building heights from AHN: roof level from the surface model, ground level
//! from the terrain just outside the footprint.

// geo 0.29 deprecates EuclideanDistance, but its replacement only measures point
// to point. The contract pins geo 0.29.
#[allow(deprecated)]
use geo::EuclideanDistance;
use geo::{BoundingRect, Contains, Point, Polygon, Rect};

use crate::sources::ahn::Elevation;

/// The 70th percentile of the surface cells inside a footprint reads as "the
/// roof": above eaves and courtyards, below chimneys and the ridge of a pitched roof.
const ROOF_PERCENTILE: f64 = 0.7;
/// Ground is the median terrain in this band around the footprint. The terrain
/// model has no values under the building itself.
const GROUND_RING_M: f64 = 2.0;

/// Height of the roof above the surrounding ground, and that ground above NAP.
/// Either is `None` when AHN has no usable cells for this footprint.
pub fn building_height(
    footprint: &Polygon<f64>,
    elevation: &Elevation,
) -> (Option<f64>, Option<f64>) {
    let Some(rect) = footprint.bounding_rect() else {
        return (None, None);
    };
    let around = Rect::new(
        (rect.min().x - GROUND_RING_M, rect.min().y - GROUND_RING_M),
        (rect.max().x + GROUND_RING_M, rect.max().y + GROUND_RING_M),
    );

    let surface = &elevation.surface;
    let terrain = &elevation.terrain;
    let (columns, rows) = surface.cells_within(around);
    let mut roof = Vec::new();
    let mut ground = Vec::new();
    for row in rows {
        for column in columns.clone() {
            let centre = Point::from(surface.cell_centre(column, row));
            if footprint.contains(&centre) {
                roof.push(surface.value(column, row));
            } else if is_near(&centre, footprint) {
                ground.push(terrain.value(column, row));
            }
        }
    }

    let roof_m = percentile(roof, ROOF_PERCENTILE);
    let ground_m = percentile(ground, 0.5);
    let height_m = match (roof_m, ground_m) {
        (Some(roof_m), Some(ground_m)) => Some(round_to_cm(roof_m - ground_m)),
        _ => None,
    };

    (height_m, ground_m.map(round_to_cm))
}

#[allow(deprecated)]
fn is_near(centre: &Point<f64>, footprint: &Polygon<f64>) -> bool {
    centre.euclidean_distance(footprint) <= GROUND_RING_M
}

/// The value at `fraction` (0 to 1) of the sorted finite values, nearest rank.
/// [1, 2, 3, 4, 5] at 0.5 -> 3.
pub fn percentile(mut values: Vec<f32>, fraction: f64) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }

    values.sort_by(f32::total_cmp);
    let rank = (fraction * (values.len() - 1) as f64).round() as usize;
    Some(f64::from(values[rank]))
}

// AHN is accurate to a few centimetres; more digits only pretend.
fn round_to_cm(metres: f64) -> f64 {
    (metres * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::ahn::Grid;
    use geo::polygon;

    #[test]
    fn the_median_of_five_values_is_the_middle_one() {
        assert_eq!(percentile(vec![5.0, 1.0, 3.0, 2.0, 4.0], 0.5), Some(3.0));
    }

    #[test]
    fn no_data_cells_are_ignored_and_all_missing_gives_none() {
        assert_eq!(percentile(vec![f32::NAN, 2.0, f32::NAN], 0.7), Some(2.0));
        assert_eq!(percentile(vec![f32::NAN], 0.5), None);
    }

    #[test]
    fn the_70th_percentile_ignores_a_tall_chimney() {
        let mut roof = vec![10.0; 9];
        roof.push(25.0);
        assert_eq!(percentile(roof, ROOF_PERCENTILE), Some(10.0));
    }

    #[test]
    fn a_ten_metre_box_on_flat_ground_is_ten_metres_tall() {
        // 40 m square grid at 0.5 m, ground at 1 m NAP, a 10 by 10 m building at its centre.
        let size = 80;
        let footprint = polygon![(x: 15.0, y: 15.0), (x: 25.0, y: 15.0), (x: 25.0, y: 25.0), (x: 15.0, y: 25.0)];
        let mut surface = Grid {
            west: 0.0,
            north: 40.0,
            cell_m: 0.5,
            width: size,
            height: size,
            values: vec![1.0; size * size],
        };
        let mut terrain = surface.clone();
        for row in 0..size {
            for column in 0..size {
                if footprint.contains(&Point::from(surface.cell_centre(column, row))) {
                    surface.values[row * size + column] = 11.0;
                    terrain.values[row * size + column] = f32::NAN;
                }
            }
        }

        let (height_m, ground_m) = building_height(&footprint, &Elevation { surface, terrain });
        assert_eq!(height_m, Some(10.0));
        assert_eq!(ground_m, Some(1.0));
    }
}
