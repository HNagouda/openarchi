//! Place groups from OpenStreetMap: businesses and facilities on the site by
//! group, and transit stops within a 10 minute walk by mode. Classes and
//! colours come from the diagrams that draw them, so a chart matches its sheet.

use super::buildings::diagram_classify;
use super::{finish, row, Group, Unit};
use crate::render::{build_classes, class_of};
use crate::scene::{Feature, Layer, Scene};
use crate::theme::Theme;
use crate::Result;

const BUSINESS_DIAGRAM: &str = "business";
const FACILITY_DIAGRAM: &str = "facility";
const TRANSIT_DIAGRAM: &str = "transit";
/// The walk the transit group counts stops within, the transit diagram's own.
const TRANSIT_WALK_MIN: f64 = 10.0;

pub fn groups(scene: &Scene, theme: &Theme) -> Result<Vec<Group>> {
    let on_site = |layer: Layer| -> Vec<&Feature> {
        scene
            .features
            .iter()
            .filter(|feature| feature.layer == layer)
            .collect()
    };
    let stops_within: Vec<&Feature> = scene
        .walk
        .iter()
        .flat_map(|walk| &walk.places)
        .filter(|place| place.layer == Layer::TransitStop)
        .filter(|place| {
            place
                .attributes
                .walk_min
                .is_some_and(|minutes| minutes <= TRANSIT_WALK_MIN)
        })
        .collect();

    Ok(vec![
        counted(
            "businesses",
            "Businesses",
            Unit::Places,
            &on_site(Layer::Business),
            BUSINESS_DIAGRAM,
            theme,
        )?,
        counted(
            "facilities",
            "Facilities",
            Unit::Places,
            &on_site(Layer::Facility),
            FACILITY_DIAGRAM,
            theme,
        )?,
        counted(
            "transit_stops_10_min",
            "Transit stops within a 10 minute walk",
            Unit::Stops,
            &stops_within,
            TRANSIT_DIAGRAM,
            theme,
        )?,
    ])
}

/// Counts `places` into the classes of `diagram`, with its labels and colours.
fn counted(
    id: &str,
    title: &str,
    unit: Unit,
    places: &[&Feature],
    diagram: &str,
    theme: &Theme,
) -> Result<Group> {
    let classify = diagram_classify(diagram)?;
    let classes = build_classes(&classify, theme)?;
    let mut counts = vec![0.0; classes.classes.len()];
    for place in places {
        if let Some(index) = class_of(place, &classify) {
            let last = counts.len() - 1;
            counts[index.min(last)] += 1.0;
        }
    }
    let rows = classes
        .classes
        .iter()
        .zip(counts)
        .map(|(class, count)| row(&class.label, count, class.fill.clone()))
        .collect();

    Ok(finish(id, title, unit, rows, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::builtin_themes;
    use crate::scene::Attributes;
    use geo::{Geometry, Point};

    fn place(layer: Layer, group: &str) -> Feature {
        Feature {
            layer,
            geometry: Geometry::Point(Point::new(0.0, 0.0)),
            attributes: Attributes {
                group: Some(group.to_string()),
                ..Attributes::default()
            },
        }
    }

    #[test]
    fn places_are_counted_in_the_classes_of_their_diagram() {
        let theme = &builtin_themes()[0];
        let places = [
            place(Layer::Business, "shop"),
            place(Layer::Business, "shop"),
            place(Layer::Business, "food_drink"),
        ];
        let places: Vec<&Feature> = places.iter().collect();

        let group = counted(
            "businesses",
            "Businesses",
            Unit::Places,
            &places,
            "business",
            theme,
        )
        .unwrap();

        assert_eq!(group.total, 3.0);
        assert_eq!(group.rows[0].label, "Food and drink");
        assert_eq!(group.rows[1].label, "Shops");
        assert_eq!(group.rows[1].value, 2.0);
    }
}
