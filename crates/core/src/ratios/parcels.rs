//! Parcel groups: how much of each parcel is built over (coverage), and how
//! much floor it holds for its size (floor area ratio), counted per parcel.
//!
//! Parcels and buildings are both clipped to the site, so a parcel the
//! boundary cuts is measured on its inside part, with the buildings on it.

use std::panic::{self, AssertUnwindSafe};

use geo::{Area, BooleanOps, BoundingRect, Geometry, Intersects, MultiPolygon, Rect};

use super::{binned, finish, row, Group, Row, Unit};
use crate::floors::estimated_floors;
use crate::render::resolve_colour;
use crate::scene::{Feature, Layer, Scene};
use crate::theme::Theme;
use crate::Result;

/// BAG outlines and BRK boundaries are surveyed apart and disagree by
/// centimetres. An overlap smaller than this is that, not a building on the parcel.
const MIN_OVERLAP_M2: f64 = 1.0;
/// Slivers where the site boundary just touches a parcel are not counted.
const MIN_PARCEL_M2: f64 = 1.0;

/// Coverage in quarters: plain to read, and the same on every site.
const COVERAGE_BREAKS: [f64; 3] = [0.25, 0.5, 0.75];
const COVERAGE_LABELS: [&str; 4] = ["Under 25 %", "25–50 %", "50–75 %", "75–100 %"];

/// Floor area ratio classes, roughly: detached houses, terraces, perimeter
/// blocks, dense city blocks, and inner-city or tower plots.
const FAR_BREAKS: [f64; 4] = [0.5, 1.0, 2.0, 3.0];
const FAR_LABELS: [&str; 5] = ["Under 0.5", "0.5–1", "1–2", "2–3", "3 or more"];

const NO_BUILDING_LABEL: &str = "No building";

/// One building, ready to be measured against many parcels.
struct Footprint {
    shape: MultiPolygon<f64>,
    rect: Rect<f64>,
    floors: Option<f64>,
}

/// What one parcel holds.
struct Measured {
    /// Built-over share of the parcel, 0 to 1. 0 means no building.
    coverage: f64,
    /// `None` when a building on it has no height.
    floor_area_ratio: Option<f64>,
}

pub fn groups(scene: &Scene, buildings: &[&Feature], theme: &Theme) -> Result<Vec<Group>> {
    let footprints: Vec<Footprint> = buildings
        .iter()
        .filter_map(|building| footprint(building))
        .collect();
    let mut measured = Vec::new();
    let mut unmeasurable = 0;
    for parcel in scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Parcel)
    {
        let Some(shape) = area_shape(&parcel.geometry) else {
            continue;
        };
        if shape.unsigned_area() < MIN_PARCEL_M2 {
            continue;
        }
        let (parcel_measured, failures) = measure(&shape, &footprints);
        measured.push(parcel_measured);
        unmeasurable += failures;
    }

    let note = (unmeasurable > 0).then(|| {
        format!("{unmeasurable} building outlines were too broken to measure and are left out.")
    });
    let built: Vec<&Measured> = measured
        .iter()
        .filter(|parcel| parcel.coverage > 0.0)
        .collect();
    let empty_count = (measured.len() - built.len()) as f64;
    let empty_fill = resolve_colour(&theme.palette, "@muted")?;

    let coverages: Vec<Option<f64>> = built.iter().map(|parcel| Some(parcel.coverage)).collect();
    let ratios: Vec<Option<f64>> = built.iter().map(|parcel| parcel.floor_area_ratio).collect();
    let coverage_rows = with_empty_row(
        row(NO_BUILDING_LABEL, empty_count, empty_fill.clone()),
        binned(&coverages, &COVERAGE_BREAKS, &COVERAGE_LABELS, theme)?,
    );
    let ratio_rows = with_empty_row(
        row(NO_BUILDING_LABEL, empty_count, empty_fill),
        binned(&ratios, &FAR_BREAKS, &FAR_LABELS, theme)?,
    );

    Ok(vec![
        finish(
            "coverage",
            "Building coverage",
            Unit::Parcels,
            coverage_rows,
            note.clone(),
        ),
        finish(
            "floor_area_ratio",
            "Floor area ratio",
            Unit::Parcels,
            ratio_rows,
            note,
        ),
    ])
}

fn with_empty_row(empty: Row, rows: Vec<Row>) -> Vec<Row> {
    std::iter::once(empty).chain(rows).collect()
}

/// The parcel's built-over area and floor area, from every building whose box
/// touches it. Also returns how many overlaps could not be computed.
fn measure(parcel: &MultiPolygon<f64>, footprints: &[Footprint]) -> (Measured, usize) {
    let parcel_area = parcel.unsigned_area();
    let Some(parcel_rect) = parcel.bounding_rect() else {
        return (empty_parcel(), 0);
    };

    let mut built_m2 = 0.0;
    let mut floor_m2 = 0.0;
    let mut is_height_missing = false;
    let mut failures = 0;
    for footprint in footprints
        .iter()
        .filter(|footprint| footprint.rect.intersects(&parcel_rect))
    {
        let Some(overlap) = overlap_m2(parcel, &footprint.shape) else {
            failures += 1;
            continue;
        };
        if overlap < MIN_OVERLAP_M2 {
            continue;
        }
        built_m2 += overlap;
        match footprint.floors {
            Some(floors) => floor_m2 += overlap * floors,
            None => is_height_missing = true,
        }
    }

    let measured = Measured {
        // Overlapping BAG outlines can add up to a little over the parcel.
        coverage: (built_m2 / parcel_area).min(1.0),
        floor_area_ratio: (!is_height_missing).then_some(floor_m2 / parcel_area),
    };
    (measured, failures)
}

fn empty_parcel() -> Measured {
    Measured {
        coverage: 0.0,
        floor_area_ratio: Some(0.0),
    }
}

/// Area of the overlap, or `None` when geo cannot cut the two shapes. Its
/// boolean operations can panic on degenerate rings, as in `sources::clip`.
fn overlap_m2(parcel: &MultiPolygon<f64>, building: &MultiPolygon<f64>) -> Option<f64> {
    panic::catch_unwind(AssertUnwindSafe(|| {
        parcel.intersection(building).unsigned_area()
    }))
    .ok()
}

fn footprint(building: &Feature) -> Option<Footprint> {
    let shape = area_shape(&building.geometry)?;
    let rect = shape.bounding_rect()?;
    Some(Footprint {
        shape,
        rect,
        floors: building.attributes.height_m.map(estimated_floors),
    })
}

fn area_shape(geometry: &Geometry<f64>) -> Option<MultiPolygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => Some(MultiPolygon::new(vec![polygon.clone()])),
        Geometry::MultiPolygon(polygons) => Some(polygons.clone()),
        _ => None,
    }
}
