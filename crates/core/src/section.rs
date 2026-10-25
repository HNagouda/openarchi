//! A section through the site: a line the user draws on the plan of a sheet,
//! and what a vertical cut along it meets.
//!
//! The line sits in real RD New metres, like pins, rather than in the scene's
//! local frame. Fetching the site again moves the scene's origin, and a line
//! kept relative to it would quietly move with it.
//!
//! The viewer stands on the line and looks to its left, seen from A towards
//! A': with A on the left of the drawing and A' on the right, as a section is
//! read. Swapping the ends turns the view round.

pub mod cut;
pub mod profile;
pub mod water;

use geo::{BoundingRect, Rect};
use serde::{Deserialize, Serialize};

use crate::scene::Scene;
use crate::{Error, Result};

/// How far past the line buildings and trees are drawn in elevation, unless
/// the user sets it. Two or three street blocks deep: enough to read the
/// street wall behind the cut, not so deep that the far side of town crowds it.
pub const DEFAULT_DEPTH_M: f64 = 60.0;
pub const MAX_DEPTH_M: f64 = 500.0;
/// A shorter line is a slip of the mouse, not a section worth drawing.
pub const MIN_LENGTH_M: f64 = 5.0;
/// Every scale a section can be drawn at, as 1:N, in one place. The app's
/// scale menu offers the same list.
pub const SCALES: [u32; 16] = [
    100, 200, 250, 500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 5000, 7500, 10_000, 20_000,
];
/// How many times heights may be drawn taller than lengths. A flat site reads
/// as a thin strip at true proportions; a few round factors keep it honest.
pub const EXAGGERATIONS: [u32; 3] = [1, 2, 5];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionLine {
    /// The end labelled A, in RD New metres.
    pub start: (f64, f64),
    /// The end labelled A'.
    pub end: (f64, f64),
    /// How far past the line, in metres, the elevation reaches.
    #[serde(default = "default_depth")]
    pub depth_m: f64,
    /// The scale the section is drawn at, 1:`scale`, one of `SCALES`. `None`
    /// picks the largest that fits the sheet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    /// Heights drawn this many times taller than lengths, one of `EXAGGERATIONS`.
    #[serde(
        default = "true_proportions",
        skip_serializing_if = "is_true_proportions"
    )]
    pub exaggeration: u32,
}

fn default_depth() -> f64 {
    DEFAULT_DEPTH_M
}

fn true_proportions() -> u32 {
    1
}

fn is_true_proportions(exaggeration: &u32) -> bool {
    *exaggeration == 1
}

impl SectionLine {
    /// A line the renderer can draw, or an error naming what is wrong.
    pub fn check(&self) -> Result<()> {
        let numbers = [self.start.0, self.start.1, self.end.0, self.end.1];
        if numbers.iter().any(|number| !number.is_finite()) {
            return Err(Error::Invalid(
                "a section line needs two points with finite coordinates".to_string(),
            ));
        }
        let length = (self.end.0 - self.start.0).hypot(self.end.1 - self.start.1);
        if length < MIN_LENGTH_M {
            return Err(Error::Invalid(format!(
                "a section line must be at least {MIN_LENGTH_M} m long, got {length:.1} m"
            )));
        }
        if !(self.depth_m > 0.0 && self.depth_m <= MAX_DEPTH_M) {
            return Err(Error::Invalid(format!(
                "a section's depth must be more than 0 and at most {MAX_DEPTH_M} m, got {}",
                self.depth_m
            )));
        }

        if let Some(scale) = self.scale {
            if !SCALES.contains(&scale) {
                return Err(Error::Invalid(format!(
                    "a section's scale must be one of {SCALES:?}, got {scale}"
                )));
            }
        }
        if !EXAGGERATIONS.contains(&self.exaggeration) {
            return Err(Error::Invalid(format!(
                "a section's heights can be exaggerated {EXAGGERATIONS:?} times, got {}",
                self.exaggeration
            )));
        }

        Ok(())
    }

    /// The line in a scene's frame, which is RD New minus `origin`.
    pub fn axis(&self, origin: (f64, f64)) -> Axis {
        let start = (self.start.0 - origin.0, self.start.1 - origin.1);
        let (dx, dy) = (self.end.0 - self.start.0, self.end.1 - self.start.1);
        let length = dx.hypot(dy).max(f64::EPSILON);
        let along = (dx / length, dy / length);
        Axis {
            start,
            along,
            // A quarter turn counterclockwise: the left of A -> A'.
            looking: (-along.1, along.0),
            length,
        }
    }

    /// The same line with A and A' swapped, which looks the other way.
    pub fn flipped(&self) -> SectionLine {
        SectionLine {
            start: self.end,
            end: self.start,
            ..self.clone()
        }
    }
}

/// True when both ends of `line` lie where the scene has data to draw: inside
/// the kept terrain, which covers the neighbourhood, or for a scene without
/// it inside the box around the site and its neighbourhood.
pub fn lies_within(scene: &Scene, line: &SectionLine) -> bool {
    let reach = match &scene.terrain {
        Some(grid) => Some(Rect::new(
            (grid.west, grid.south()),
            (grid.east(), grid.north),
        )),
        None => scene
            .context
            .iter()
            .filter_map(|feature| feature.geometry.bounding_rect())
            .chain(scene.boundary.bounding_rect())
            .reduce(|a, b| {
                Rect::new(
                    (a.min().x.min(b.min().x), a.min().y.min(b.min().y)),
                    (a.max().x.max(b.max().x), a.max().y.max(b.max().y)),
                )
            }),
    };
    let Some(reach) = reach else {
        return false;
    };
    let (origin_x, origin_y) = scene.origin;
    [line.start, line.end].iter().all(|&(x, y)| {
        let (x, y) = (x - origin_x, y - origin_y);
        x >= reach.min().x && x <= reach.max().x && y >= reach.min().y && y <= reach.max().y
    })
}

/// A section line in the scene's frame, with the two directions a section needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis {
    pub start: (f64, f64),
    /// Unit vector from A towards A'.
    pub along: (f64, f64),
    /// Unit vector the viewer looks in, square to the line.
    pub looking: (f64, f64),
    pub length: f64,
}

impl Axis {
    /// (along, depth) of a point: metres from A along the line, and metres in
    /// front of the viewer, negative behind them. (10, 3) is 10 m from A, 3 m beyond the line.
    pub fn station(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.start.0, y - self.start.1);
        (
            dx * self.along.0 + dy * self.along.1,
            dx * self.looking.0 + dy * self.looking.1,
        )
    }

    /// The point `along` metres from A and `depth` metres beyond the line.
    pub fn point_at(&self, along: f64, depth: f64) -> (f64, f64) {
        (
            self.start.0 + along * self.along.0 + depth * self.looking.0,
            self.start.1 + along * self.along.1 + depth * self.looking.1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn west_to_east() -> SectionLine {
        SectionLine {
            start: (1000.0, 2000.0),
            end: (1100.0, 2000.0),
            depth_m: DEFAULT_DEPTH_M,
            scale: None,
            exaggeration: 1,
        }
    }

    #[test]
    fn a_line_from_west_to_east_looks_north() {
        let axis = west_to_east().axis((1000.0, 2000.0));
        assert_eq!(axis.looking, (-0.0, 1.0));
        assert_eq!(axis.station(30.0, 5.0), (30.0, 5.0));
        assert_eq!(axis.point_at(30.0, 5.0), (30.0, 5.0));
    }

    #[test]
    fn a_flipped_line_looks_the_other_way() {
        let axis = west_to_east().flipped().axis((1000.0, 2000.0));
        let (along, depth) = axis.station(30.0, 5.0);
        assert!((along - 70.0).abs() < 1e-9);
        assert!((depth + 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_line_without_depth_reads_with_the_default() {
        let line: SectionLine =
            serde_json::from_str(r#"{"start": [0, 0], "end": [50, 0]}"#).unwrap();
        assert_eq!(line.depth_m, DEFAULT_DEPTH_M);
        assert_eq!(line.scale, None);
        assert_eq!(line.exaggeration, 1);
    }

    #[test]
    fn a_short_line_and_a_deep_one_are_refused() {
        let short = SectionLine {
            end: (1002.0, 2000.0),
            ..west_to_east()
        };
        let deep = SectionLine {
            depth_m: 900.0,
            ..west_to_east()
        };
        let odd_scale = SectionLine {
            scale: Some(300),
            ..west_to_east()
        };
        let odd_exaggeration = SectionLine {
            exaggeration: 3,
            ..west_to_east()
        };
        assert!(short.check().is_err());
        assert!(deep.check().is_err());
        assert!(odd_scale.check().is_err());
        assert!(odd_exaggeration.check().is_err());
        assert!(west_to_east().check().is_ok());
    }
}
