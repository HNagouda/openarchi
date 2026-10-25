//! How the prevailing wind moves through the site at street level, in plan.
//!
//! The wind comes from the site's prevailing direction in the typical year
//! kept with the scene (`year.rs`), all year or in one season. The flow
//! around the building footprints of the site and its neighbourhood is
//! solved in two dimensions with the lattice Boltzmann method (`lattice.rs`)
//! and read back on a grid over the site's open ground, as the local speed
//! over the incoming speed: the wind speed ratio of pedestrian wind studies.
//!
//! A plan cannot let air over the roofs, so every bit of it squeezes through
//! the gaps between buildings: speeds in the gaps come out higher, and calm
//! behind buildings larger, than in the real three-dimensional flow. The
//! diagram is good for comparing sheltered and windy places on one site, not
//! for comfort criteria, and its legend says so.

mod field;
pub mod lattice;
mod lines;
mod memo;
mod year;

use std::sync::Arc;

use geo::{BoundingRect, Geometry, LineString, MultiPolygon, Polygon};
use serde::{Deserialize, Serialize};

use crate::analysis::climate::SEASONS;
use crate::scene::{Attributes, Credit, Feature, Layer, Scene};
use crate::sources::pvgis;
use crate::sunlight::raster::Raster;
use crate::sunlight::{banded, cells};
use field::Field;
use lines::Line;
pub use year::{Prevailing, WindMonth, WindYear};

const PERCENT: f64 = 100.0;
/// The flow is blended between its cells, so the site is read on a grid this
/// share of a flow cell, for bands that follow the streets more smoothly.
const SITE_CELL_SHARE: f64 = 0.5;
/// But never finer than the hours of sun's grid.
const FINEST_SITE_CELL_M: f64 = 2.0;

/// The months the prevailing wind is taken from. A sheet keeps its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindSeason {
    #[default]
    Year,
    Winter,
    Spring,
    Summer,
    Autumn,
}

impl WindSeason {
    pub fn is_year(&self) -> bool {
        *self == WindSeason::Year
    }

    /// January is 1.
    pub fn months(self) -> Vec<u32> {
        match self.season() {
            Some((_, _, months)) => months.to_vec(),
            None => (1..=12).collect(),
        }
    }

    /// "all year", or "in winter (Dec–Feb)".
    pub fn label(self) -> String {
        match self.season() {
            Some((name, months, _)) => format!("in {} ({months})", name.to_lowercase()),
            None => "all year".to_string(),
        }
    }

    /// The meteorological season of the wind roses, so both name the same months.
    fn season(self) -> Option<(&'static str, &'static str, [u32; 3])> {
        let index = match self {
            WindSeason::Year => return None,
            WindSeason::Winter => 0,
            WindSeason::Spring => 1,
            WindSeason::Summer => 2,
            WindSeason::Autumn => 3,
        };
        Some(SEASONS[index])
    }
}

/// The flow over the site, ready to draw.
pub struct WindFlow {
    /// A grid over the site, in the scene's frame.
    raster: Raster,
    /// The wind speed ratio of each open ground cell; `None` outside the site or under a roof.
    ratios: Vec<Option<f64>>,
    lines: Vec<Line>,
    arrows: Vec<[(f64, f64); 3]>,
}

/// The PVGIS credit, printed only on sheets that draw the wind.
pub fn credit() -> Credit {
    Credit {
        layers: vec![Layer::Wind],
        ..pvgis::credit()
    }
}

/// The prevailing wind of the scene's typical year in `season`. `None` for a
/// scene fetched before the wind was kept, or a typical year without wind.
pub fn prevailing(scene: &Scene, season: WindSeason) -> Option<Prevailing> {
    scene.wind.as_ref()?.prevailing(&season.months())
}

/// How the legend names the wind: "from the SW, most common
/// all year (24% of hours)". A scene without the kept wind draws none, and the
/// missing-data note says why.
pub fn described(scene: &Scene, season: WindSeason) -> String {
    match prevailing(scene, season) {
        Some(prevailing) => format!(
            "from the {}, most common {} ({:.0}% of hours)",
            prevailing.name,
            season.label(),
            prevailing.share * PERCENT
        ),
        None => format!("from its most common direction {}", season.label()),
    }
}

/// The flow for wind from `from_deg`, solved once and kept for the next drawing.
pub fn flow(scene: &Scene, from_deg: f64) -> Arc<WindFlow> {
    let (raster, downwind, obstacles) = Field::grid(scene, from_deg);
    let key = memo::Key {
        from_deg,
        boundary: scene.boundary.clone(),
        raster,
        solid: obstacles.solid.clone(),
    };
    memo::kept_or(key, || {
        let field = Field::solve(raster, downwind, obstacles);
        over_site(scene, &field)
    })
}

/// The wind speed ratio in bands of `breaks`, on the wind layer, and the
/// flow lines with their arrowheads, on the wind line layer.
pub fn features(flow: &WindFlow, breaks: &[f64]) -> Vec<Feature> {
    let mut features = banded(&flow.raster, &flow.ratios, breaks, Layer::Wind);
    // Lines are drawn as thin ribbons, so the theme can edge them in the
    // paper colour and they read over the darkest band as over the lightest.
    let ribbons = flow.lines.iter().map(lines::ribbon);
    let arrows = flow.arrows.iter().map(|corners| corners.to_vec());
    let shapes = ribbons
        .chain(arrows)
        .map(|ring| Polygon::new(LineString::from(ring), Vec::new()))
        .collect();
    features.push(Feature {
        layer: Layer::WindLine,
        geometry: Geometry::MultiPolygon(MultiPolygon::new(shapes)),
        attributes: Attributes::default(),
    });
    features
}

/// The solved flow read on a grid of the flow's own fineness over the site.
fn over_site(scene: &Scene, field: &Field) -> WindFlow {
    let cell_m = (field.cell_m() * SITE_CELL_SHARE).max(FINEST_SITE_CELL_M);
    let raster = site_raster(scene, cell_m);
    let cells = cells(scene, &raster);
    let ratios = (0..raster.len())
        .map(|index| {
            if cells[index] != crate::sunlight::Cell::Ground {
                return None;
            }
            let (x, y) = field.at(raster.centre(index)).unwrap_or((0.0, 0.0));
            Some(x.hypot(y))
        })
        .collect();
    let (lines, arrows) = lines::flow_lines(field, &raster, &cells);

    WindFlow {
        raster,
        ratios,
        lines,
        arrows,
    }
}

fn site_raster(scene: &Scene, cell_m: f64) -> Raster {
    let Some(bounds) = scene.boundary.bounding_rect() else {
        return Raster {
            min: (0.0, 0.0),
            cell_m,
            columns: 0,
            rows: 0,
        };
    };
    Raster {
        min: (bounds.min().x, bounds.min().y),
        cell_m,
        columns: (bounds.width() / cell_m).ceil() as usize,
        rows: (bounds.height() / cell_m).ceil() as usize,
    }
}
