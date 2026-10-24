//! Groups from the environment and BAG units: how much of the site lies in
//! each noise band, how many dwellings the buildings hold, and how large the
//! buildings are by their BAG floor area.

use super::buildings::{classified, diagram_classify};
use super::{binned, finish, row, Group, Unit};
use crate::render::{build_classes, class_of};
use crate::scene::{Feature, Layer, Scene};
use crate::sources::bands::visible_areas;
use crate::theme::Theme;
use crate::Result;

/// Diagrams whose classes these groups reuse, so the numbers match their legends.
const NOISE_DIAGRAM: &str = "traffic-noise";
const DWELLINGS_DIAGRAM: &str = "households";

/// Floor area classes: a house, a large house or small block, a block, a
/// large block or school, and a complex.
const FLOOR_AREA_BREAKS: [f64; 5] = [100.0, 250.0, 1000.0, 2500.0, 10_000.0];
const FLOOR_AREA_LABELS: [&str; 6] = [
    "Under 100 m²",
    "100–250 m²",
    "250–1,000 m²",
    "1,000–2,500 m²",
    "2,500–10,000 m²",
    "10,000 m² or more",
];

pub fn groups(scene: &Scene, buildings: &[&Feature], theme: &Theme) -> Result<Vec<Group>> {
    let floor_areas: Vec<Option<f64>> = buildings
        .iter()
        .map(|building| building.attributes.floor_area_m2)
        .collect();
    let floor_area_note = "BAG records usable floor area per unit, which is about a fifth smaller than gross floor area.";

    Ok(vec![
        noise(scene, theme)?,
        classified(
            "dwellings",
            buildings,
            &diagram_classify(DWELLINGS_DIAGRAM)?,
            theme,
        )?,
        finish(
            "floor_area",
            "Floor area per building",
            Unit::Buildings,
            binned(&floor_areas, &FLOOR_AREA_BREAKS, &FLOOR_AREA_LABELS, theme)?,
            Some(floor_area_note.to_string()),
        ),
    ])
}

/// The site's area in each road traffic noise band.
fn noise(scene: &Scene, theme: &Theme) -> Result<Group> {
    let classify = diagram_classify(NOISE_DIAGRAM)?;
    let classes = build_classes(&classify, theme)?;
    let bands: Vec<&Feature> = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::RoadNoise)
        .collect();

    let mut areas = vec![0.0; classes.classes.len()];
    for (value, area) in visible_areas(&bands) {
        let band = bands
            .iter()
            .find(|band| band.attributes.value == Some(value));
        if let Some(index) = band.and_then(|band| class_of(band, &classify)) {
            areas[index] += area;
        }
    }
    let rows = classes
        .classes
        .iter()
        .zip(areas)
        .map(|(class, area)| row(&class.label, area, class.fill.clone()))
        .collect();

    Ok(finish(
        "road_noise",
        "Road traffic noise",
        Unit::SquareMetres,
        rows,
        classify.note.clone(),
    ))
}
