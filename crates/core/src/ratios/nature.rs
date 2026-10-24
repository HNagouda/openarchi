//! Green and water groups: area per BGT type, in shades of the theme's green and water.

use std::collections::BTreeMap;

use geo::Area;

use super::{finish, row, Group, Row, Unit};
use crate::render::{mix, resolve_colour};
use crate::scene::{Layer, Scene};
use crate::theme::Theme;
use crate::Result;

/// Green classes in display order. `green_label` maps BGT types onto them.
const GREEN_LABELS: [&str; 7] = [
    "Planted areas",
    "Grass",
    "Woodland",
    "Shrubs",
    "Nature",
    "Farmland",
    "Other green",
];
const VERGES_LABEL: &str = "Verges";

/// Water classes in display order. `water_label` maps BGT types onto them.
const WATER_LABELS: [&str; 5] = [
    "Canals and streams",
    "Lakes and ponds",
    "Sea",
    "Ditches",
    "Other water",
];
const BANKS_LABEL: &str = "Banks";

/// How far each further row moves from the theme colour towards its ink,
/// so neighbouring rows of one colour stay apart.
const SHADE_STEP: f64 = 0.18;

pub fn green(scene: &Scene, theme: &Theme) -> Result<Group> {
    let mut by_label: BTreeMap<&str, f64> = BTreeMap::new();
    let mut tree_count = 0;
    for feature in &scene.features {
        let label = match feature.layer {
            Layer::Green => green_label(feature.attributes.kind.as_deref()),
            Layer::Verge => VERGES_LABEL,
            Layer::Tree => {
                tree_count += 1;
                continue;
            }
            _ => continue,
        };
        *by_label.entry(label).or_insert(0.0) += feature.geometry.unsigned_area();
    }

    let labels: Vec<&str> = GREEN_LABELS.into_iter().chain([VERGES_LABEL]).collect();
    let base = resolve_colour(&theme.palette, "@green")?;
    let rows = shaded_rows(&labels, &by_label, &base, theme)?;
    let note = (tree_count > 0)
        .then(|| format!("Plus {tree_count} single trees, which are points rather than areas."));

    Ok(finish("green", "Green", Unit::SquareMetres, rows, note))
}

pub fn water(scene: &Scene, theme: &Theme) -> Result<Group> {
    let mut by_label: BTreeMap<&str, f64> = BTreeMap::new();
    for feature in &scene.features {
        let label = match feature.layer {
            Layer::Water => water_label(feature.attributes.kind.as_deref()),
            Layer::WaterBank => BANKS_LABEL,
            _ => continue,
        };
        *by_label.entry(label).or_insert(0.0) += feature.geometry.unsigned_area();
    }

    let labels: Vec<&str> = WATER_LABELS.into_iter().chain([BANKS_LABEL]).collect();
    let base = resolve_colour(&theme.palette, "@water")?;
    let rows = shaded_rows(&labels, &by_label, &base, theme)?;

    Ok(finish("water", "Water", Unit::SquareMetres, rows, None))
}

/// One row per label in order, in shades of `base`. Only rows with a value
/// take a shade, so the first row present always has the theme colour itself.
fn shaded_rows(
    labels: &[&str],
    values: &BTreeMap<&str, f64>,
    base: &str,
    theme: &Theme,
) -> Result<Vec<Row>> {
    let ink = resolve_colour(&theme.palette, "@ink")?;
    let present = labels.iter().filter(|label| values.contains_key(*label));
    let rows = present
        .enumerate()
        .map(|(index, label)| {
            let fill = mix(base, &ink, index as f64 * SHADE_STEP);
            row(label, values[label], fill)
        })
        .collect();

    Ok(rows)
}

/// BGT begroeidterreindeel `fysiek_voorkomen` -> a green class.
fn green_label(kind: Option<&str>) -> &'static str {
    match kind.unwrap_or_default() {
        "groenvoorziening" => "Planted areas",
        "grasland agrarisch" | "grasland overig" => "Grass",
        "loofbos" | "naaldbos" | "gemengd bos" | "houtwal" => "Woodland",
        "struiken" => "Shrubs",
        "heide" | "moeras" | "rietland" | "kwelder" | "duin" => "Nature",
        "fruitteelt" | "boomteelt" | "bouwland" => "Farmland",
        _ => "Other green",
    }
}

/// BGT waterdeel `type` -> a water class.
fn water_label(kind: Option<&str>) -> &'static str {
    match kind.unwrap_or_default() {
        "waterloop" => "Canals and streams",
        "watervlakte" => "Lakes and ponds",
        "zee" => "Sea",
        "greppel, droge sloot" => "Ditches",
        _ => "Other water",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_bgt_types_fall_in_the_other_class() {
        assert_eq!(green_label(Some("loofbos")), "Woodland");
        assert_eq!(green_label(Some("maanlandschap")), "Other green");
        assert_eq!(water_label(None), "Other water");
    }
}
