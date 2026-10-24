//! The shape the drawing is cropped to, and the site boundary drawn on top.

use std::fmt::Write;

use geo::{BooleanOps, Polygon};

use super::geometry::{decimal, mm, polygon_path, Projection};
use super::layout::Rect;
use super::svg::layer_attributes;
use crate::area::Area;
use crate::scene::Scene;
use crate::sheet::Frame;
use crate::theme::Palette;

pub const FRAME_CLIP_ID: &str = "frame-clip";
/// The site outline in the accent colour: strong enough to find, thin enough to stay a line.
const SITE_STROKE_MM: f64 = 0.5;
/// The frame edge in ink, a hairline.
const FRAME_STROKE_MM: f64 = 0.2;
const FRAME_STROKE_OPACITY: f64 = 0.7;

#[derive(Debug, Clone)]
enum Shape {
    Circle { centre: (f64, f64), radius: f64 },
    Square { x: f64, y: f64, side: f64 },
    Outline { path: String },
}

#[derive(Debug, Clone)]
pub struct FrameShape {
    shape: Shape,
    boundary_path: String,
    /// The frame had to shrink to fit the drawing zone, so part of the site is cut off.
    is_cropped: bool,
    /// The frame edge differs from the site outline and gets its own hairline.
    has_own_edge: bool,
}

impl FrameShape {
    pub fn new(frame: Frame, scene: &Scene, projection: &Projection, zone: &Rect) -> FrameShape {
        let boundary_path = polygon_path(&scene.boundary, projection);
        let corners: Vec<(f64, f64)> = scene
            .boundary
            .exterior()
            .0
            .iter()
            .map(|c| projection.to_paper(c.x, c.y))
            .collect();
        let centre = projection.paper_centre;
        let (min_x, max_x, min_y, max_y) = extent(&corners, centre);

        match frame {
            Frame::Circle => {
                let site_radius = corners
                    .iter()
                    .map(|&(x, y)| ((x - centre.0).powi(2) + (y - centre.1).powi(2)).sqrt())
                    .fold(0.0, f64::max);
                let radius = site_radius.min(zone.width / 2.0).min(zone.height / 2.0);
                let is_cropped = radius < site_radius - 0.01;
                let is_circle_site = matches!(scene.area, Area::Circle { .. });
                FrameShape {
                    shape: Shape::Circle { centre, radius },
                    boundary_path,
                    is_cropped,
                    has_own_edge: is_cropped || !is_circle_site,
                }
            }
            Frame::Square => {
                let site_side = (max_x - min_x).min(max_y - min_y);
                let side = site_side.min(zone.width).min(zone.height);
                FrameShape {
                    shape: Shape::Square {
                        x: centre.0 - side / 2.0,
                        y: centre.1 - side / 2.0,
                        side,
                    },
                    boundary_path,
                    is_cropped: side < (max_x - min_x).max(max_y - min_y) - 0.01,
                    has_own_edge: true,
                }
            }
            Frame::Area => {
                let is_cropped = min_x < zone.x
                    || max_x > zone.right()
                    || min_y < zone.y
                    || max_y > zone.bottom();
                // Too big for the zone: crop the outline itself, so the frame stays one plain shape.
                let path = if is_cropped {
                    cropped_outline(&scene.boundary, projection, zone)
                } else {
                    boundary_path.clone()
                };
                FrameShape {
                    shape: Shape::Outline { path },
                    boundary_path,
                    is_cropped,
                    has_own_edge: is_cropped,
                }
            }
        }
    }

    pub fn write_defs(&self, out: &mut String) {
        let _ = write!(
            out,
            r#"<clipPath id="{FRAME_CLIP_ID}">{}</clipPath>"#,
            self.element("")
        );
    }

    /// The open ground inside the frame, under every layer.
    pub fn write_ground(&self, out: &mut String, ground: &str) {
        let _ = writeln!(
            out,
            r#"<g {} clip-path="url(#{FRAME_CLIP_ID})">{}</g>"#,
            layer_attributes("ground", "Ground"),
            self.element(&format!(r#" fill="{ground}""#))
        );
    }

    /// The site outline in the accent colour, and the frame edge when it is not the same line.
    pub fn write_outlines(&self, out: &mut String, palette: &Palette) {
        let _ = write!(out, "<g {}>", layer_attributes("site", "Site"));
        if self.has_own_edge {
            let edge = format!(
                r#" fill="none" stroke="{}" stroke-width="{}" stroke-opacity="{}""#,
                palette.ink,
                mm(FRAME_STROKE_MM),
                decimal(FRAME_STROKE_OPACITY)
            );
            out.push_str(&self.element(&edge));
        }
        if !self.boundary_path.is_empty() {
            // Uncropped, the outline sits on the frame edge and would lose its outer half to the clip.
            let clip = if self.is_cropped {
                format!(r#" clip-path="url(#{FRAME_CLIP_ID})""#)
            } else {
                String::new()
            };
            let _ = write!(
                out,
                r#"<path id="site-boundary" d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"{clip}/>"#,
                self.boundary_path,
                palette.accent,
                mm(SITE_STROKE_MM)
            );
        }
        out.push_str("</g>\n");
    }

    fn element(&self, attributes: &str) -> String {
        match &self.shape {
            Shape::Circle { centre, radius } => {
                format!(
                    r#"<circle cx="{}" cy="{}" r="{}"{attributes}/>"#,
                    mm(centre.0),
                    mm(centre.1),
                    mm(*radius)
                )
            }
            Shape::Square { x, y, side } => {
                format!(
                    r#"<rect x="{}" y="{}" width="{s}" height="{s}"{attributes}/>"#,
                    mm(*x),
                    mm(*y),
                    s = mm(*side)
                )
            }
            Shape::Outline { path } => format!(r#"<path d="{path}"{attributes}/>"#),
        }
    }
}

/// The boundary cut to the drawing zone, as path data.
fn cropped_outline(boundary: &Polygon<f64>, projection: &Projection, zone: &Rect) -> String {
    let (left, top) = projection.to_ground(zone.x, zone.y);
    let (right, bottom) = projection.to_ground(zone.right(), zone.bottom());
    let zone_on_ground = Polygon::new(
        vec![
            (left, bottom),
            (right, bottom),
            (right, top),
            (left, top),
            (left, bottom),
        ]
        .into(),
        vec![],
    );

    boundary
        .intersection(&zone_on_ground)
        .0
        .iter()
        .map(|part| polygon_path(part, projection))
        .collect()
}

/// Min and max x and y of the points, or the centre alone when there are none.
fn extent(points: &[(f64, f64)], centre: (f64, f64)) -> (f64, f64, f64, f64) {
    let mut bounds = (centre.0, centre.0, centre.1, centre.1);
    for (index, &(x, y)) in points.iter().enumerate() {
        if index == 0 {
            bounds = (x, x, y, y);
        }
        bounds = (
            bounds.0.min(x),
            bounds.1.max(x),
            bounds.2.min(y),
            bounds.3.max(y),
        );
    }

    bounds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_of_no_points_is_the_centre() {
        assert_eq!(extent(&[], (5.0, 6.0)), (5.0, 5.0, 6.0, 6.0));
    }
}
