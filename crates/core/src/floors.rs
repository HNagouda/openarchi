//! Storeys and gross floor area of a building, estimated from its roof height.
//!
//! None of the sources a scene draws on records how many storeys a building
//! has: BAG keeps footprints, years and units, AHN keeps heights. So storeys
//! are the roof height over a typical floor-to-floor height, and gross floor
//! area is the footprint times those storeys. Both are estimates, and every
//! legend and ratio that shows them says so.

use geo::Area;

use crate::scene::Feature;

/// Floor-to-floor height used to estimate storeys from roof height, the usual
/// figure for housing. The Dutch building code asks at least 2.6 m clear in
/// living rooms, and a floor adds about 0.4 m, so 3 m. Offices and old ground
/// floors are taller, but the roof height also includes a pitched roof, and
/// rounding to the nearest storey evens the two out.
// #! NOTE[floors, country] : One height for every use; offices (about 3.6–3.9 m floor to floor) come out a storey or two high in a tall office block
pub const STOREY_HEIGHT_M: f64 = 3.0;

/// 13.4 m -> 4 storeys. Never less than one.
pub fn estimated_floors(height_m: f64) -> f64 {
    (height_m / STOREY_HEIGHT_M).round().max(1.0)
}

/// The storeys of a building, or `None` when its height is unknown.
pub fn floors_of(building: &Feature) -> Option<f64> {
    building.attributes.height_m.map(estimated_floors)
}

/// Footprint times storeys, in m², or `None` when the height is unknown.
pub fn gross_floor_area_m2(building: &Feature) -> Option<f64> {
    Some(building.geometry.unsigned_area() * floors_of(building)?)
}

#[cfg(test)]
mod tests {
    use geo::{polygon, Geometry};

    use super::*;
    use crate::scene::{Attributes, Layer};

    fn building(height_m: Option<f64>) -> Feature {
        Feature {
            layer: Layer::Building,
            geometry: Geometry::Polygon(polygon![
                (x: 0.0, y: 0.0),
                (x: 10.0, y: 0.0),
                (x: 10.0, y: 8.0),
                (x: 0.0, y: 8.0),
            ]),
            attributes: Attributes {
                height_m,
                ..Attributes::default()
            },
        }
    }

    #[test]
    fn floors_round_to_the_nearest_and_never_drop_below_one() {
        assert_eq!(estimated_floors(13.4), 4.0);
        assert_eq!(estimated_floors(4.6), 2.0);
        assert_eq!(estimated_floors(4.4), 1.0);
        assert_eq!(estimated_floors(0.4), 1.0);
    }

    #[test]
    fn gross_floor_area_is_footprint_times_floors() {
        assert_eq!(gross_floor_area_m2(&building(Some(9.2))), Some(240.0));
    }

    #[test]
    fn every_legend_that_names_the_storey_height_names_this_one() {
        let height = format!("{STOREY_HEIGHT_M} m");
        let notes: Vec<String> = crate::catalog::builtin_diagrams()
            .iter()
            .flat_map(|diagram| diagram.drawings())
            .filter_map(|(_, drawing)| drawing.classify?.note)
            .filter(|note| note.contains("per floor") || note.contains("m each"))
            .collect();

        assert!(notes.len() >= 3);
        assert!(notes.iter().all(|note| note.contains(&height)), "{notes:?}");
    }

    #[test]
    fn a_building_without_a_height_has_no_floors() {
        assert_eq!(floors_of(&building(None)), None);
        assert_eq!(gross_floor_area_m2(&building(None)), None);
    }
}
