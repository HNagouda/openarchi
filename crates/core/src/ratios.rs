//! Ratios: how a site divides up, each group a set of shares that add up to 100 %.
//!
//! Every group answers one question: what covers the ground, what the
//! buildings are used for, how densely each parcel is built. Areas come from
//! the scene's features, which are already clipped to the site boundary.
//!
//! Bins are fixed classes, never quantiles. Quantile breaks move with every
//! site, so two sites could not be compared and the labels would be odd
//! numbers. Height and age use the very breaks of their diagrams, so a chart
//! reads the same as the legend on the sheet next to it.
//!
//! Colours come from the theme the way the diagrams take them: land use from
//! the layer fills, building use from the categories, binned groups from the
//! ramp. Changing the theme recolours the charts with the sheets.

mod buildings;
mod chart;
mod csv;
mod nature;
mod parcels;

use std::collections::BTreeMap;

use geo::Area;
use serde::Serialize;

use crate::render::{resolve_colour, sample_ramp};
use crate::scene::{Feature, Layer, Scene};
use crate::theme::Theme;
use crate::Result;

pub use chart::{chart_svg, ChartStyle};
// Shared with the analysis, so both write areas and counts the same way.
pub(crate) use chart::{area_text, grouped};
pub use csv::to_csv;

/// What the ground is used for, and the layers each class sums, in one place.
/// Bridges, tracks, trees, parcels and contours lie on top of the ground
/// rather than covering it, so they are left out. The first layer of each
/// class gives its colour.
const LAND_USE: [(&str, &[Layer]); 7] = [
    ("Buildings", &[Layer::Building]),
    (
        "Streets and paths",
        &[
            Layer::Road,
            Layer::Footpath,
            Layer::Cycleway,
            Layer::Parking,
            Layer::RailArea,
        ],
    ),
    ("Paved open space", &[Layer::Paved]),
    ("Yards and gardens", &[Layer::Yard]),
    ("Green", &[Layer::Green, Layer::Verge]),
    ("Water", &[Layer::Water, Layer::WaterBank]),
    ("Bare ground", &[Layer::Unpaved]),
];

/// The movement surfaces, one row each.
const TRANSPORT: [(&str, Layer); 5] = [
    ("Roads", Layer::Road),
    ("Parking", Layer::Parking),
    ("Cycleways", Layer::Cycleway),
    ("Footpaths", Layer::Footpath),
    ("Rail", Layer::RailArea),
];

/// BAG buildings and BGT surfaces are surveyed apart, so their sum misses
/// the site area by a little either way. A gap smaller than this share of the
/// site is survey noise, not land worth a row.
const UNMAPPED_MIN_SHARE: f64 = 0.005;
const UNMAPPED_LABEL: &str = "Unmapped";

/// Fallback fill for a layer the theme does not draw.
const NO_FILL: &str = "@muted";

/// The row for values that are missing, such as a building without a height.
const UNKNOWN_LABEL: &str = "Unknown";

#[derive(Debug, Clone, Serialize)]
pub struct Ratios {
    pub site_area_m2: f64,
    pub building_count: usize,
    /// Building footprint over site area, 0 to 1.
    pub coverage: f64,
    /// Estimated floor area over site area. Buildings without a height are left out.
    pub floor_area_ratio: f64,
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    /// Stable, snake_case: "land_use", "building_use".
    pub id: String,
    pub title: String,
    pub unit: Unit,
    /// The sum of the rows, in `unit`.
    pub total: f64,
    pub rows: Vec<Row>,
    /// One plain sentence the rows cannot say, such as how floors were estimated.
    pub note: Option<String>,
}

/// What a group's values measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    SquareMetres,
    Buildings,
    Parcels,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub label: String,
    pub value: f64,
    /// 0 to 1 of the group total.
    pub share: f64,
    /// "#RRGGBB" from the theme.
    pub fill: String,
}

/// Every group the scene has data for, coloured by `theme`. Groups without
/// any data, such as water on a dry site, are left out.
pub fn compute(scene: &Scene, theme: &Theme) -> Result<Ratios> {
    let site_area_m2 = scene.boundary.unsigned_area();
    let areas = area_per_layer(scene);
    let buildings: Vec<&Feature> = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .collect();

    let mut groups = vec![
        land_use(&areas, site_area_m2, theme)?,
        transport(&areas, theme)?,
    ];
    groups.extend(buildings::groups(&buildings, theme)?);
    groups.extend(parcels::groups(scene, &buildings, theme)?);
    groups.push(nature::green(scene, theme)?);
    groups.push(nature::water(scene, theme)?);
    // #! TODO[ratios, bag] : Add a households group once the scene holds BAG verblijfsobjecten
    groups.retain(|group| group.total > 0.0);

    let footprint_m2 = areas.get(&Layer::Building).copied().unwrap_or(0.0);
    Ok(Ratios {
        site_area_m2,
        building_count: buildings.len(),
        coverage: footprint_m2 / site_area_m2,
        floor_area_ratio: buildings::floor_area_m2(&buildings) / site_area_m2,
        groups,
    })
}

/// Rows with no value are dropped, and the rest get their share of the total.
fn finish(id: &str, title: &str, unit: Unit, rows: Vec<Row>, note: Option<String>) -> Group {
    let rows: Vec<Row> = rows.into_iter().filter(|row| row.value > 0.0).collect();
    let total: f64 = rows.iter().map(|row| row.value).sum();
    let rows = rows
        .into_iter()
        .map(|row| Row {
            share: row.value / total,
            ..row
        })
        .collect();

    Group {
        id: id.to_string(),
        title: title.to_string(),
        unit,
        total,
        rows,
        note,
    }
}

/// A row before `finish` has worked out its share.
fn row(label: &str, value: f64, fill: String) -> Row {
    Row {
        label: label.to_string(),
        value,
        share: 0.0,
        fill,
    }
}

/// Counts values into fixed classes: n breaks make n + 1 classes, coloured
/// along the theme's ramp, then an unknown row for `None`.
fn binned(
    values: &[Option<f64>],
    breaks: &[f64],
    labels: &[&str],
    theme: &Theme,
) -> Result<Vec<Row>> {
    let mut counts = vec![0.0; labels.len()];
    let mut unknown = 0.0;
    for value in values {
        match value {
            Some(value) => counts[breaks.iter().filter(|&&limit| *value >= limit).count()] += 1.0,
            None => unknown += 1.0,
        }
    }

    let mut rows: Vec<Row> = labels
        .iter()
        .zip(counts)
        .zip(ramp(theme, labels.len())?)
        .map(|((label, count), fill)| row(label, count, fill))
        .collect();
    rows.push(row(
        UNKNOWN_LABEL,
        unknown,
        resolve_colour(&theme.palette, "@unknown")?,
    ));

    Ok(rows)
}

/// `count` colours spread along the theme's ramp, light to strong.
fn ramp(theme: &Theme, count: usize) -> Result<Vec<String>> {
    let stops = theme
        .ramp
        .iter()
        .map(|value| resolve_colour(&theme.palette, value))
        .collect::<Result<Vec<String>>>()?;
    Ok(sample_ramp(&stops, count))
}

fn land_use(areas: &BTreeMap<Layer, f64>, site_area_m2: f64, theme: &Theme) -> Result<Group> {
    let mut rows = Vec::new();
    for (label, layers) in LAND_USE {
        let value = layers.iter().filter_map(|layer| areas.get(layer)).sum();
        rows.push(row(label, value, layer_fill(theme, layers[0])?));
    }

    let mapped_m2: f64 = rows.iter().map(|row| row.value).sum();
    let unmapped_m2 = site_area_m2 - mapped_m2;
    if unmapped_m2 > site_area_m2 * UNMAPPED_MIN_SHARE {
        rows.push(row(
            UNMAPPED_LABEL,
            unmapped_m2,
            resolve_colour(&theme.palette, "@unknown")?,
        ));
    }

    let note = "Buildings come from BAG and the ground from BGT, so the shares are of their sum, which can differ slightly from the site area.";
    Ok(finish(
        "land_use",
        "Land use",
        Unit::SquareMetres,
        rows,
        Some(note.to_string()),
    ))
}

fn transport(areas: &BTreeMap<Layer, f64>, theme: &Theme) -> Result<Group> {
    let mut rows = Vec::new();
    for (label, layer) in TRANSPORT {
        let value = areas.get(&layer).copied().unwrap_or(0.0);
        rows.push(row(label, value, layer_fill(theme, layer)?));
    }

    Ok(finish(
        "transport",
        "Transport",
        Unit::SquareMetres,
        rows,
        None,
    ))
}

fn area_per_layer(scene: &Scene) -> BTreeMap<Layer, f64> {
    let mut areas = BTreeMap::new();
    for feature in &scene.features {
        *areas.entry(feature.layer).or_insert(0.0) += feature.geometry.unsigned_area();
    }

    areas
}

/// The theme's fill for `layer` as "#RRGGBB", so a land use row matches the map.
fn layer_fill(theme: &Theme, layer: Layer) -> Result<String> {
    let fill = theme
        .layers
        .get(&layer)
        .and_then(|style| style.fill.as_deref())
        .unwrap_or(NO_FILL);
    resolve_colour(&theme.palette, fill)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_drops_empty_rows_and_shares_add_up_to_one() {
        let rows = vec![
            row("a", 30.0, "#000000".to_string()),
            row("b", 0.0, "#000000".to_string()),
            row("c", 10.0, "#000000".to_string()),
        ];
        let group = finish("test", "Test", Unit::Buildings, rows, None);
        assert_eq!(group.rows.len(), 2);
        assert_eq!(group.total, 40.0);
        assert!((group.rows[0].share - 0.75).abs() < 1e-12);
    }
}
