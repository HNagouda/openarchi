//! Rasters turned into banded areas a drawing program can edit: noise in 5 dB
//! steps, flood depth in depth classes, and so on.
//!
//! Each band is drawn as everything at or above its lower edge, and the bands
//! are stacked lowest first. So a band edge that smoothing moved a little is
//! hidden under the next band, instead of leaving a hairline gap down to the
//! ground the way side-by-side bands would. Seen from above, every place shows
//! exactly its own band.
//!
//! The raster is resampled to fine cells and blurred before the band edges are
//! traced, then the edges are thinned and rounded, so a 25 m air quality grid
//! reads as drawn curves rather than as stairs.

use std::collections::BTreeMap;

use contour::ContourBuilder;
use geo::{Area, ChaikinSmoothing, Geometry, MultiPolygon, Polygon, Simplify};

use crate::scene::{Attributes, Feature, Layer};
use crate::sources::contours::{filled, smoothed};
use crate::sources::raster::Grid;

/// Every layer drawn as stacked bands, in one place. The same ten as
/// `environment::MEASURED_LAYERS` today, so a new one is likely needed in both.
pub const BANDED: [Layer; 10] = [
    Layer::RoadNoise,
    Layer::AllNoise,
    Layer::No2,
    Layer::Pm10,
    Layer::Pm25,
    Layer::Flood,
    Layer::Slope,
    Layer::TreeCanopy,
    Layer::VegetationIndex,
    Layer::HeatIsland,
];

/// Band edges are traced on cells this size. Finer than every source, coarse
/// enough that a 1 km site stays well under a second.
const FINE_CELL_M: f64 = 2.0;
/// Bends closer than this to a straight line are dropped before rounding.
const SIMPLIFY_M: f64 = 0.4;
const ROUNDING_PASSES: usize = 2;
/// Islands and holes smaller than this are specks, not places.
const MIN_PART_M2: f64 = 30.0;
/// A band showing less of itself than this inside the site is left out, so
/// the legend does not list a band nobody can see.
const MIN_VISIBLE_M2: f64 = 25.0;

/// What a missing cell means.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gaps {
    /// Unknown: filled in from the cells around it.
    Unknown,
    /// A known value, as for flood depth, where a missing cell is dry land.
    Means(f32),
}

/// Where the lowest band starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lowest {
    /// Every cell with a value is in a band; below the first break is a band too.
    Everything,
    /// Only values from here up are drawn, as for flood depth above zero.
    From(f64),
}

/// The bands of `grid` at `breaks`, as features of `layer` in RD New, each
/// with the lower edge of its band as `value`, lowest band first.
pub fn banded(
    grid: &Grid,
    layer: Layer,
    breaks: &[f64],
    lowest: Lowest,
    gaps: Gaps,
) -> Vec<Feature> {
    let complete = match gaps {
        Gaps::Unknown => filled(grid),
        Gaps::Means(value) => Grid {
            values: grid
                .values
                .iter()
                .map(|cell| if cell.is_nan() { value } else { *cell })
                .collect(),
            ..grid.clone()
        },
    };
    let finite = complete
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite());
    let Some(minimum) = finite.reduce(f32::min).map(f64::from) else {
        return Vec::new();
    };

    let fine = smoothed(&resampled(&complete, FINE_CELL_M));
    let mut edges = Vec::new();
    match lowest {
        // Just under the lowest value, so the first area covers the whole grid.
        Lowest::Everything => edges.push((minimum - 1.0, minimum.min(breaks[0] - 1.0))),
        Lowest::From(edge) => edges.push((edge, edge)),
    }
    edges.extend(breaks.iter().map(|&edge| (edge, edge)));

    let thresholds: Vec<f64> = edges.iter().map(|&(threshold, _)| threshold).collect();
    let areas = at_or_above(&fine, &thresholds);
    areas
        .into_iter()
        .zip(edges)
        .filter_map(|(area, (_, value))| {
            let area = drawn(area);
            (!area.0.is_empty()).then(|| Feature {
                layer,
                geometry: Geometry::MultiPolygon(area),
                attributes: Attributes {
                    value: Some(value),
                    ..Attributes::default()
                },
            })
        })
        .collect()
}

/// The bands of each banded layer that show inside the site, once clipped.
/// Every other feature passes through.
pub fn without_hidden_bands(features: Vec<Feature>) -> Vec<Feature> {
    let mut hidden = vec![false; features.len()];
    for layer in BANDED {
        let mut bands: Vec<(usize, f64, f64)> = features
            .iter()
            .enumerate()
            .filter(|(_, feature)| feature.layer == layer)
            .map(|(index, feature)| {
                let value = feature.attributes.value.unwrap_or(f64::NEG_INFINITY);
                (index, value, feature.geometry.unsigned_area())
            })
            .collect();
        bands.sort_by(|a, b| a.1.total_cmp(&b.1));
        for pair in bands.windows(2) {
            if pair[0].2 - pair[1].2 < MIN_VISIBLE_M2 {
                hidden[pair[0].0] = true;
            }
        }
    }

    features
        .into_iter()
        .zip(hidden)
        .filter(|(_, is_hidden)| !is_hidden)
        .map(|(feature, _)| feature)
        .collect()
}

/// The area each band shows on its own, lowest band first, as (lower edge, m²).
pub fn visible_areas(bands: &[&Feature]) -> Vec<(f64, f64)> {
    let mut by_edge: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for band in bands {
        let value = band.attributes.value.unwrap_or(f64::NEG_INFINITY);
        // Keyed in thousandths, so the map can sort the edges.
        let entry = by_edge
            .entry((value * 1000.0) as i64)
            .or_insert((value, 0.0));
        entry.1 += band.geometry.unsigned_area();
    }

    let stacked: Vec<(f64, f64)> = by_edge.into_values().collect();
    stacked
        .iter()
        .enumerate()
        .map(|(index, &(value, area))| {
            let above = stacked.get(index + 1).map_or(0.0, |next| next.1);
            (value, (area - above).max(0.0))
        })
        .collect()
}

/// `grid` resampled to `cell_m` by bilinear interpolation, over the same extent.
fn resampled(grid: &Grid, cell_m: f64) -> Grid {
    if grid.cell_m <= cell_m {
        return grid.clone();
    }
    let width = (grid.width as f64 * grid.cell_m / cell_m).round() as usize;
    let height = (grid.height as f64 * grid.cell_m / cell_m).round() as usize;
    let mut values = Vec::with_capacity(width * height);
    for row in 0..height {
        for column in 0..width {
            // In source cells, measured from the first cell's centre.
            let u = ((column as f64 + 0.5) * cell_m / grid.cell_m - 0.5)
                .clamp(0.0, (grid.width - 1) as f64);
            let v = ((row as f64 + 0.5) * cell_m / grid.cell_m - 0.5)
                .clamp(0.0, (grid.height - 1) as f64);
            values.push(bilinear(grid, u, v));
        }
    }

    Grid {
        cell_m,
        width,
        height,
        values,
        ..*grid
    }
}

fn bilinear(grid: &Grid, u: f64, v: f64) -> f32 {
    let (left, top) = (u.floor() as usize, v.floor() as usize);
    let (right, bottom) = (
        (left + 1).min(grid.width - 1),
        (top + 1).min(grid.height - 1),
    );
    let (across, down) = ((u - left as f64) as f32, (v - top as f64) as f32);
    let upper = grid.value(left, top) * (1.0 - across) + grid.value(right, top) * across;
    let lower = grid.value(left, bottom) * (1.0 - across) + grid.value(right, bottom) * across;

    upper * (1.0 - down) + lower * down
}

/// For each threshold, the area where `grid` is at or above it, in RD New.
fn at_or_above(grid: &Grid, thresholds: &[f64]) -> Vec<MultiPolygon<f64>> {
    // The contour builder counts rows upwards, so the rows go in from the south.
    let mut values = Vec::with_capacity(grid.values.len());
    for row in (0..grid.height).rev() {
        for column in 0..grid.width {
            values.push(f64::from(grid.value(column, row)));
        }
    }
    let builder = ContourBuilder::new(grid.width, grid.height, true)
        .x_origin(grid.west + grid.cell_m / 2.0)
        .y_origin(grid.south() + grid.cell_m / 2.0)
        .x_step(grid.cell_m)
        .y_step(grid.cell_m);
    let Ok(contours) = builder.contours(&values, thresholds) else {
        return vec![MultiPolygon::new(Vec::new()); thresholds.len()];
    };

    contours
        .into_iter()
        .map(|contour| contour.into_inner().0)
        .collect()
}

/// A traced area thinned, rounded and cleared of specks.
fn drawn(area: MultiPolygon<f64>) -> MultiPolygon<f64> {
    let rounded = area
        .simplify(&SIMPLIFY_M)
        .chaikin_smoothing(ROUNDING_PASSES);
    let parts = rounded
        .0
        .into_iter()
        .filter(|part| part.unsigned_area() >= MIN_PART_M2)
        .map(|part| {
            let (exterior, holes) = part.into_inner();
            let holes = holes
                .into_iter()
                .filter(|hole| {
                    Polygon::new(hole.clone(), Vec::new()).unsigned_area() >= MIN_PART_M2
                })
                .collect();
            Polygon::new(exterior, holes)
        })
        .collect();

    MultiPolygon::new(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 200 by 100 m at 10 m cells, rising 1 per 10 m eastwards from 40 to 59.
    fn ramp_east() -> Grid {
        let (width, height) = (20, 10);
        Grid {
            west: 0.0,
            north: 100.0,
            cell_m: 10.0,
            width,
            height,
            values: (0..width * height)
                .map(|index| 40.0 + (index % width) as f32)
                .collect(),
        }
    }

    #[test]
    fn each_band_covers_everything_from_its_edge_eastwards() {
        let bands = banded(
            &ramp_east(),
            Layer::RoadNoise,
            &[45.0, 50.0, 55.0],
            Lowest::Everything,
            Gaps::Unknown,
        );
        let values: Vec<f64> = bands
            .iter()
            .map(|band| band.attributes.value.unwrap())
            .collect();
        assert_eq!(values, vec![40.0, 45.0, 50.0, 55.0]);

        // The cell centred on x = 105 holds 50, so the 50 dB band starts there.
        let Geometry::MultiPolygon(area) = &bands[2].geometry else {
            panic!("a band is an area")
        };
        let west_edge = area
            .0
            .iter()
            .flat_map(|part| part.exterior().0.iter())
            .filter(|point| point.y > 30.0 && point.y < 70.0)
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        assert!(
            (west_edge - 105.0).abs() < 3.0,
            "50 dB band starts at x = {west_edge}"
        );
        assert!(bands[0].geometry.unsigned_area() > bands[1].geometry.unsigned_area());
    }

    #[test]
    fn dry_cells_count_as_zero_and_draw_nothing() {
        let mut grid = ramp_east();
        grid.values = grid
            .values
            .iter()
            .map(|&value| {
                if value < 50.0 {
                    f32::NAN
                } else {
                    (value - 50.0) / 4.0
                }
            })
            .collect();
        let bands = banded(
            &grid,
            Layer::Flood,
            &[0.5, 1.0, 2.0],
            Lowest::From(0.01),
            Gaps::Means(0.0),
        );
        assert_eq!(bands[0].attributes.value, Some(0.01));
        // Water from about x = 100 eastwards, never over the dry western half.
        assert!(bands[0].geometry.unsigned_area() < 110.0 * 100.0);
        assert!(bands[0].geometry.unsigned_area() > 80.0 * 100.0);
    }

    #[test]
    fn a_band_hidden_under_the_next_one_is_dropped_and_shares_add_up() {
        let bands = banded(
            &ramp_east(),
            Layer::RoadNoise,
            &[45.0, 50.0, 55.0],
            Lowest::Everything,
            Gaps::Unknown,
        );
        let total: f64 = visible_areas(&bands.iter().collect::<Vec<_>>())
            .iter()
            .map(|(_, area)| area)
            .sum();
        assert!((total - bands[0].geometry.unsigned_area()).abs() < 1.0);

        let mut doubled = bands.clone();
        let mut copy = bands[3].clone();
        copy.attributes.value = Some(54.0);
        doubled.push(copy);
        let kept = without_hidden_bands(doubled);
        assert_eq!(kept.len(), 4);
        assert!(kept.iter().all(|band| band.attributes.value != Some(54.0)));
    }
}
