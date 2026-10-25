//! Areas the user highlights inside the site: the plot, a proposed footprint,
//! a square to be redesigned.
//!
//! Like pins, highlights belong to the project rather than to one sheet and
//! sit in real RD New metres, so every sheet draws them where they belong,
//! whatever it frames and however its view is tilted. A highlight can also
//! become a sheet of its own: its shape turned into a sheet area
//! (`selection`), so that sheet's ratios and figures describe exactly it.

use geo::{Area as _, Centroid, Coord, LineString, Polygon};
use serde::{Deserialize, Serialize};

use crate::area::{Area, LonLat};
use crate::crs::{rd_to_wgs84, wgs84_to_rd};
use crate::pin::{self, MAX_LABEL_CHARS};
use crate::ratios::area_text;
use crate::render::is_colour;
use crate::scene::Scene;
use crate::{Error, Result};

/// Smaller than this, in square metres, is a slip of the mouse rather than an area.
pub const MIN_AREA_M2: f64 = 4.0;
/// A hand-drawn outline has a few dozen corners; more is not drawn by hand.
pub const MAX_POINTS: usize = 200;
/// Enough segments that a circle looks round at any sheet scale.
const CIRCLE_SEGMENTS: usize = 96;
/// The legend row of a highlight without a label.
pub const UNLABELLED: &str = "Highlighted area";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    /// Unique within the project: "highlight-1".
    pub id: String,
    /// May be empty; the legend then says "Highlighted area".
    #[serde(default)]
    pub label: String,
    pub shape: HighlightShape,
    #[serde(default)]
    pub outline: Outline,
    /// The colour of the outline and tint: a palette role like "@accent", or
    /// "#RRGGBB". Older highlights have none and draw in the accent.
    #[serde(default = "default_colour")]
    pub colour: String,
    /// Fades everything outside the highlight a little, so the eye goes to it.
    #[serde(default)]
    pub dims_rest: bool,
    /// Adds the area in m² to the label.
    #[serde(default)]
    pub shows_area: bool,
}

/// The shape, in RD New metres like pins, so it stays put when the site is fetched again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HighlightShape {
    Circle {
        center: [f64; 2],
        radius_m: f64,
    },
    /// Square to the RD grid, as the site step's rectangle is square to the map.
    Rectangle {
        min: [f64; 2],
        max: [f64; 2],
    },
    /// Outer ring only, not closed: the first point is not repeated at the end.
    Polygon {
        points: Vec<[f64; 2]>,
    },
}

/// What a highlight without a colour of its own is drawn in.
pub const DEFAULT_COLOUR: &str = "@accent";

fn default_colour() -> String {
    DEFAULT_COLOUR.to_string()
}

/// The few looks a highlight's edge can have.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outline {
    /// A firm line, for what exists or is decided: the plot.
    #[default]
    Solid,
    /// A dashed line, for what is proposed: a new footprint.
    Dashed,
    /// Round dots, for what is studied or uncertain: an area of intervention.
    Dotted,
}

impl HighlightShape {
    /// The outline as RD New points, a circle as a many-sided polygon. Not closed.
    pub fn ring(&self) -> Vec<(f64, f64)> {
        match self {
            HighlightShape::Circle { center, radius_m } => (0..CIRCLE_SEGMENTS)
                .map(|index| {
                    let angle = std::f64::consts::TAU * index as f64 / CIRCLE_SEGMENTS as f64;
                    (
                        center[0] + radius_m * angle.cos(),
                        center[1] + radius_m * angle.sin(),
                    )
                })
                .collect(),
            HighlightShape::Rectangle { min, max } => vec![
                (min[0], min[1]),
                (max[0], min[1]),
                (max[0], max[1]),
                (min[0], max[1]),
            ],
            HighlightShape::Polygon { points } => {
                points.iter().map(|point| (point[0], point[1])).collect()
            }
        }
    }

    /// The shape in the scene's local frame, whose (0, 0) is RD New `origin`.
    pub fn polygon_from(&self, origin: (f64, f64)) -> Polygon<f64> {
        let ring: Vec<Coord<f64>> = self
            .ring()
            .into_iter()
            .map(|(x, y)| Coord {
                x: x - origin.0,
                y: y - origin.1,
            })
            .collect();
        Polygon::new(LineString::new(ring), vec![])
    }

    /// In square metres; a circle's is exact rather than its polygon's.
    pub fn area_m2(&self) -> f64 {
        match self {
            HighlightShape::Circle { radius_m, .. } => std::f64::consts::PI * radius_m * radius_m,
            _ => self.polygon_from((0.0, 0.0)).unsigned_area(),
        }
    }

    /// The middle in RD New: a circle's centre, otherwise the centroid.
    pub fn middle(&self) -> (f64, f64) {
        if let HighlightShape::Circle { center, .. } = self {
            return (center[0], center[1]);
        }
        self.polygon_from((0.0, 0.0))
            .centroid()
            .map(|point| point.x_y())
            .unwrap_or((f64::NAN, f64::NAN))
    }

    /// A shape drawn in longitude and latitude, as an assistant passes it.
    /// A rectangle becomes the RD-square box around its two corners.
    pub fn from_area(area: &Area) -> HighlightShape {
        match area {
            Area::Circle { center, radius_m } => {
                let (x, y) = wgs84_to_rd(*center);
                HighlightShape::Circle {
                    center: [x, y],
                    radius_m: *radius_m,
                }
            }
            Area::Rectangle {
                south_west,
                north_east,
            } => {
                let (x1, y1) = wgs84_to_rd(*south_west);
                let (x2, y2) = wgs84_to_rd(*north_east);
                HighlightShape::Rectangle {
                    min: [x1.min(x2), y1.min(y2)],
                    max: [x1.max(x2), y1.max(y2)],
                }
            }
            Area::Polygon { points } => HighlightShape::Polygon {
                points: points
                    .iter()
                    .map(|point| {
                        let (x, y) = wgs84_to_rd(*point);
                        [x, y]
                    })
                    .collect(),
            },
        }
    }

    /// The shape as a sheet area in longitude and latitude, for "Open as
    /// sheet". A rectangle goes as its four corners, because a box square to
    /// RD is not quite square to longitude and latitude.
    pub fn to_area(&self) -> Area {
        match self {
            HighlightShape::Circle { center, radius_m } => Area::Circle {
                center: rd_to_wgs84(center[0], center[1]),
                radius_m: *radius_m,
            },
            _ => Area::Polygon {
                points: self
                    .ring()
                    .into_iter()
                    .map(|(x, y)| rd_to_wgs84(x, y))
                    .collect::<Vec<LonLat>>(),
            },
        }
    }

    fn coordinates(&self) -> Vec<f64> {
        match self {
            HighlightShape::Circle { center, radius_m } => vec![center[0], center[1], *radius_m],
            HighlightShape::Rectangle { min, max } => vec![min[0], min[1], max[0], max[1]],
            HighlightShape::Polygon { points } => points.iter().flatten().copied().collect(),
        }
    }
}

impl Highlight {
    /// A highlight the renderer can draw, or an error naming what is wrong.
    pub fn check(&self) -> Result<()> {
        let invalid =
            |problem: String| Err(Error::Invalid(format!("highlight {}: {problem}", self.id)));
        if self
            .shape
            .coordinates()
            .iter()
            .any(|value| !value.is_finite())
        {
            return invalid("every coordinate must be a real number".to_string());
        }
        match &self.shape {
            HighlightShape::Rectangle { min, max } if min[0] >= max[0] || min[1] >= max[1] => {
                return invalid(
                    "the rectangle's min corner must lie south-west of its max corner".to_string(),
                );
            }
            HighlightShape::Polygon { points } if points.len() < 3 || points.len() > MAX_POINTS => {
                return invalid(format!(
                    "a polygon needs 3 to {MAX_POINTS} points, got {}",
                    points.len()
                ));
            }
            _ => {}
        }
        let area = self.shape.area_m2();
        if area < MIN_AREA_M2 {
            return invalid(format!(
                "it covers {area:.1} m²; draw at least {MIN_AREA_M2} m²"
            ));
        }
        if !is_colour(&self.colour) {
            return invalid(format!(
                "its colour must look like '#RRGGBB' or a palette role like '@accent', got {:?}",
                self.colour
            ));
        }
        let length = self.label.chars().count();
        if length > MAX_LABEL_CHARS {
            return invalid(format!(
                "a label of {length} characters; keep it to {MAX_LABEL_CHARS}"
            ));
        }

        Ok(())
    }

    /// What the sheet writes beside it, the area added when asked: "Plot · 412 m²".
    pub fn caption(&self) -> String {
        if !self.shows_area {
            return self.label.clone();
        }
        let area = area_text(self.shape.area_m2());
        if self.label.is_empty() {
            area
        } else {
            format!("{} · {area}", self.label)
        }
    }

    /// The legend row's text.
    pub fn legend_label(&self) -> &str {
        if self.label.is_empty() {
            UNLABELLED
        } else {
            &self.label
        }
    }
}

/// True when the highlight's middle lies inside the fetched site, the rule
/// the app and the assistant both place highlights by.
pub fn is_on_site(scene: &Scene, highlight: &Highlight) -> bool {
    let (x, y) = highlight.shape.middle();
    pin::is_on_site(scene, x, y)
}

/// The first "highlight-N" id no highlight uses yet.
pub fn next_highlight_id(highlights: &[Highlight]) -> String {
    let mut number = highlights.len() + 1;
    loop {
        let id = format!("highlight-{number}");
        if highlights.iter().all(|highlight| highlight.id != id) {
            return id;
        }
        number += 1;
    }
}

#[cfg(test)]
mod tests;
