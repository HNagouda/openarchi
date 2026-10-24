//! Building groups: use, floors, height, year built and footprint, counted per building.

use geo::Area;

use super::{binned, finish, row, Group, Unit, UNKNOWN_LABEL};
use crate::catalog;
use crate::diagram::Classify;
use crate::render::{build_classes, class_of};
use crate::scene::Feature;
use crate::theme::Theme;
use crate::{Error, Result};

/// Floor to floor height used to estimate floors from roof height. Dutch
/// housing has at least 2.6 m clear plus about 0.4 m of floor, so 3 m. Offices
/// and old ground floors are taller, but the roof height also includes the
/// pitched roof, and rounding to the nearest floor evens the two out.
pub const STOREY_HEIGHT_M: f64 = 3.0;

/// Floor classes by how a building is lived in: houses, walk-up blocks and
/// canal houses, mid-rise where lifts start, taller blocks, and high-rise.
const FLOOR_BREAKS: [f64; 4] = [3.0, 5.0, 7.0, 11.0];
const FLOOR_LABELS: [&str; 5] = [
    "1–2 floors",
    "3–4 floors",
    "5–6 floors",
    "7–10 floors",
    "11 floors or more",
];

/// Footprint classes: sheds and garages, a house, a canal house or villa, a
/// small apartment building, a block, and halls.
const FOOTPRINT_BREAKS: [f64; 5] = [50.0, 100.0, 250.0, 500.0, 1000.0];
const FOOTPRINT_LABELS: [&str; 6] = [
    "Under 50 m²",
    "50–100 m²",
    "100–250 m²",
    "250–500 m²",
    "500–1,000 m²",
    "1,000 m² or more",
];

/// Diagrams whose classes the building groups reuse, so the numbers match their legends.
const USE_DIAGRAM: &str = "building-use";
const AGE_DIAGRAM: &str = "building-age";
const HEIGHT_DIAGRAM: &str = "building-height";

pub fn groups(buildings: &[&Feature], theme: &Theme) -> Result<Vec<Group>> {
    let floors: Vec<Option<f64>> = buildings
        .iter()
        .map(|building| building.attributes.height_m.map(estimated_floors))
        .collect();
    let footprints: Vec<Option<f64>> = buildings
        .iter()
        .map(|building| Some(building.geometry.unsigned_area()))
        .collect();
    let floors_note =
        format!("Floors are estimated from roof height at {STOREY_HEIGHT_M} m per floor.");

    Ok(vec![
        classified(
            "building_use",
            buildings,
            &diagram_classify(USE_DIAGRAM)?,
            theme,
        )?,
        finish(
            "floors",
            "Floors",
            Unit::Buildings,
            binned(&floors, &FLOOR_BREAKS, &FLOOR_LABELS, theme)?,
            Some(floors_note),
        ),
        classified(
            "height",
            buildings,
            &diagram_classify(HEIGHT_DIAGRAM)?,
            theme,
        )?,
        classified(
            "year_built",
            buildings,
            &diagram_classify(AGE_DIAGRAM)?,
            theme,
        )?,
        finish(
            "footprint",
            "Footprint area",
            Unit::Buildings,
            binned(&footprints, &FOOTPRINT_BREAKS, &FOOTPRINT_LABELS, theme)?,
            None,
        ),
    ])
}

/// 13.4 m -> 4 floors. Never less than one.
pub fn estimated_floors(height_m: f64) -> f64 {
    (height_m / STOREY_HEIGHT_M).round().max(1.0)
}

/// Footprint times estimated floors, summed. Buildings without a height are left out.
pub fn floor_area_m2(buildings: &[&Feature]) -> f64 {
    buildings
        .iter()
        .filter_map(|building| {
            let floors = estimated_floors(building.attributes.height_m?);
            Some(building.geometry.unsigned_area() * floors)
        })
        .sum()
}

/// Counts buildings into a diagram's classes, with its labels and colours.
/// Mixed use falls in the class the diagram draws it in.
pub(super) fn classified(
    id: &str,
    buildings: &[&Feature],
    classify: &Classify,
    theme: &Theme,
) -> Result<Group> {
    let classes = build_classes(classify, theme)?;
    let mut counts = vec![0.0; classes.classes.len()];
    let mut unknown = 0.0;
    for building in buildings {
        match class_of(building, classify) {
            Some(index) => counts[index] += 1.0,
            None => unknown += 1.0,
        }
    }

    let mut rows: Vec<_> = classes
        .classes
        .iter()
        .zip(counts)
        .map(|(class, count)| row(&class.label, count, class.fill.clone()))
        .collect();
    rows.push(row(UNKNOWN_LABEL, unknown, classes.unknown_fill));

    Ok(finish(id, &classify.title, Unit::Buildings, rows, None))
}

/// The classes of a built-in diagram, or of one variant: "noise:road".
pub(super) fn diagram_classify(id: &str) -> Result<Classify> {
    catalog::find_plate(&catalog::builtin_diagrams(), id)
        .and_then(|diagram| diagram.classify)
        .ok_or_else(|| Error::Invalid(format!("built-in diagram {id:?} has no classes")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_round_to_the_nearest_and_never_drop_below_one() {
        assert_eq!(estimated_floors(13.4), 4.0);
        assert_eq!(estimated_floors(4.6), 2.0);
        assert_eq!(estimated_floors(0.4), 1.0);
    }
}
