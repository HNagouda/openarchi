//! The shape the drawing is cropped to, and the site boundary drawn on top.
//!
//! The frame is worked out on the ground, in metres, and then projected, so the
//! same circle becomes an ellipse and the same square a parallelogram when the
//! view is tilted.

use std::f64::consts::TAU;
use std::fmt::Write;

use geo::{BooleanOps, BoundingRect, Polygon, Rect as GroundRect};

use super::geometry::{boundary_reach, decimal, mm, polygon_path, push_paper_points, Projection};
use super::layout::Rect;
use super::svg::layer_attributes;
use crate::area::Area;
use crate::scene::Scene;
use crate::sheet::Frame;
use crate::theme::Palette;

/// The site outline in the accent colour: strong enough to find, thin enough to stay a line.
pub(super) const SITE_STROKE_MM: f64 = 0.5;
/// The frame edge in ink, a hairline.
const FRAME_STROKE_MM: f64 = 0.2;
const FRAME_STROKE_OPACITY: f64 = 0.7;
/// Points on a circle frame when it has to be a polygon: for fitting and outlines.
const CIRCLE_POINTS: usize = 96;

#[derive(Debug, Clone)]
enum Shape {
    /// Centre and radius on the ground, in metres.
    Circle { centre: (f64, f64), radius: f64 },
    /// Corners on the ground, in metres, counterclockwise from the south-west.
    Square { corners: [(f64, f64); 4] },
    /// Path data on paper, and half the smaller side of the site in metres.
    Outline { path: String, half: f64 },
}

#[derive(Debug, Clone)]
pub struct FrameShape {
    shape: Shape,
    projection: Projection,
    boundary_path: String,
    clip_id: String,
    /// The frame had to shrink to fit the drawing zone, so part of the site is cut off.
    is_cropped: bool,
    /// The frame edge differs from the site outline and gets its own hairline.
    has_own_edge: bool,
    /// The box on the ground, in metres, that holds everything the frame
    /// shows; `None` when the frame shows the whole site.
    ground_bounds: Option<GroundRect<f64>>,
}

impl FrameShape {
    /// `crop_zone` is the drawing zone a plan must stay inside; tilted views
    /// are scaled to fit beforehand and pass `None`. `scope` keeps ids unique
    /// when one sheet holds several frames.
    pub fn new(
        frame: Frame,
        scene: &Scene,
        projection: &Projection,
        crop_zone: Option<&Rect>,
        scope: &str,
    ) -> FrameShape {
        let projection = *projection;
        let boundary_path = polygon_path(&scene.boundary, &projection);
        let centre = projection.ground_centre;
        let (half_width, half_height) = half_extent(&scene.boundary);
        // The most the frame may reach from the centre, in ground metres, before the zone cuts it.
        let (reach_x, reach_y) = match crop_zone {
            Some(zone) => (
                zone.width / 2.0 / projection.mm_per_m,
                zone.height / 2.0 / projection.mm_per_m,
            ),
            None => (f64::INFINITY, f64::INFINITY),
        };
        let clip_id = format!("{scope}frame-clip");

        match frame {
            Frame::Circle => {
                let site_radius = boundary_reach(&scene.boundary, centre);
                let radius = site_radius.min(reach_x).min(reach_y);
                let is_cropped = radius < site_radius - 0.01;
                let is_circle_site = matches!(scene.area, Area::Circle { .. });
                FrameShape {
                    shape: Shape::Circle { centre, radius },
                    projection,
                    boundary_path,
                    clip_id,
                    is_cropped,
                    has_own_edge: is_cropped || !is_circle_site,
                    ground_bounds: Some(box_around(centre, radius, radius)),
                }
            }
            Frame::Square => {
                let site_half = half_width.min(half_height);
                let half = site_half.min(reach_x).min(reach_y);
                FrameShape {
                    shape: Shape::Square {
                        corners: square_corners(centre, half),
                    },
                    projection,
                    boundary_path,
                    clip_id,
                    is_cropped: half < half_width.max(half_height) - 0.01,
                    has_own_edge: true,
                    ground_bounds: Some(box_around(centre, half, half)),
                }
            }
            Frame::Area => {
                let is_cropped = half_width > reach_x || half_height > reach_y;
                // Too big for the zone: crop the outline itself, so the frame stays one plain shape.
                let path = match crop_zone {
                    Some(zone) if is_cropped => cropped_outline(&scene.boundary, &projection, zone),
                    _ => boundary_path.clone(),
                };
                FrameShape {
                    shape: Shape::Outline {
                        path,
                        half: half_width.min(half_height),
                    },
                    projection,
                    boundary_path,
                    clip_id,
                    is_cropped,
                    has_own_edge: is_cropped,
                    ground_bounds: is_cropped.then(|| box_around(centre, reach_x, reach_y)),
                }
            }
        }
    }

    /// A frame `reach_m` around the centre rather than around the site, for a
    /// plan zoomed out to a walk. The site keeps its outline inside it. An
    /// area frame becomes a circle, since a walk has no drawn shape to follow.
    pub fn around(
        frame: Frame,
        scene: &Scene,
        projection: &Projection,
        reach_m: f64,
        scope: &str,
    ) -> FrameShape {
        let projection = *projection;
        let centre = projection.ground_centre;
        let shape = match frame {
            Frame::Square => Shape::Square {
                corners: square_corners(centre, reach_m),
            },
            Frame::Circle | Frame::Area => Shape::Circle {
                centre,
                radius: reach_m,
            },
        };

        FrameShape {
            shape,
            projection,
            boundary_path: polygon_path(&scene.boundary, &projection),
            clip_id: format!("{scope}frame-clip"),
            is_cropped: false,
            has_own_edge: true,
            ground_bounds: Some(box_around(centre, reach_m, reach_m)),
        }
    }

    pub fn clip_id(&self) -> &str {
        &self.clip_id
    }

    pub fn ground_bounds(&self) -> Option<GroundRect<f64>> {
        self.ground_bounds
    }

    pub fn write_defs(&self, out: &mut String) {
        let _ = write!(
            out,
            r#"<clipPath id="{}">{}</clipPath>"#,
            self.clip_id,
            self.element("")
        );
    }

    /// The open ground inside the frame, under every layer.
    pub fn write_ground(&self, out: &mut String, ground: &str, id: &str) {
        let _ = writeln!(
            out,
            r#"<g {} clip-path="url(#{})">{}</g>"#,
            layer_attributes(id, "Ground"),
            self.clip_id,
            self.element(&format!(r#" fill="{ground}""#))
        );
    }

    /// The site outline in the accent colour, and the frame edge when it is not the same line.
    pub fn write_outlines(&self, out: &mut String, palette: &Palette, id: &str) {
        let _ = write!(out, "<g {}>", layer_attributes(id, "Site"));
        if self.has_own_edge {
            out.push_str(&self.edge(&palette.ink));
        }
        if !self.boundary_path.is_empty() {
            // Uncropped, the outline sits on the frame edge and would lose its outer half to the clip.
            let clip = if self.is_cropped {
                format!(r#" clip-path="url(#{})""#, self.clip_id)
            } else {
                String::new()
            };
            let _ = write!(
                out,
                r#"<path id="{id}-boundary" d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"{clip}/>"#,
                self.boundary_path,
                palette.accent,
                mm(SITE_STROKE_MM)
            );
        }
        out.push_str("</g>\n");
    }

    /// The frame edge alone, as a hairline in `colour`.
    pub fn edge(&self, colour: &str) -> String {
        let edge = format!(
            r#" fill="none" stroke="{colour}" stroke-width="{}" stroke-opacity="{}""#,
            mm(FRAME_STROKE_MM),
            decimal(FRAME_STROKE_OPACITY)
        );
        self.element(&edge)
    }

    /// The largest circle the frame holds, on the ground: centre and radius in metres.
    pub fn horizon(&self) -> ((f64, f64), f64) {
        match &self.shape {
            Shape::Circle { centre, radius } => (*centre, *radius),
            Shape::Square { corners } => {
                let half = (corners[1].0 - corners[0].0) / 2.0;
                (self.projection.ground_centre, half)
            }
            Shape::Outline { half, .. } => (self.projection.ground_centre, *half),
        }
    }

    fn element(&self, attributes: &str) -> String {
        let projection = &self.projection;
        match &self.shape {
            Shape::Circle { centre, radius } => {
                let (x, y) = projection.to_paper(centre.0, centre.1);
                let rx = radius * projection.mm_per_m;
                if projection.is_plan() {
                    format!(
                        r#"<circle cx="{}" cy="{}" r="{}"{attributes}/>"#,
                        mm(x),
                        mm(y),
                        mm(rx)
                    )
                } else {
                    // A turned circle is still a circle, so only the foreshortening shows.
                    format!(
                        r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}"{attributes}/>"#,
                        mm(x),
                        mm(y),
                        mm(rx),
                        mm(rx * projection.squash)
                    )
                }
            }
            Shape::Square { corners } => {
                let points: Vec<(f64, f64)> = corners
                    .iter()
                    .map(|&(x, y)| projection.to_paper(x, y))
                    .collect();
                let mut data = String::new();
                push_paper_points(&mut data, &points, true);
                format!(r#"<path d="{data}"{attributes}/>"#)
            }
            Shape::Outline { path, .. } => format!(r#"<path d="{path}"{attributes}/>"#),
        }
    }
}

/// The frame's outline on the ground, in metres, before any cropping. Tilted
/// views fit their scale to this.
pub fn ground_outline(frame: Frame, scene: &Scene, centre: (f64, f64)) -> Vec<(f64, f64)> {
    match frame {
        Frame::Circle => {
            let radius = boundary_reach(&scene.boundary, centre);
            (0..CIRCLE_POINTS)
                .map(|index| {
                    let angle = index as f64 / CIRCLE_POINTS as f64 * TAU;
                    (
                        centre.0 + radius * angle.cos(),
                        centre.1 + radius * angle.sin(),
                    )
                })
                .collect()
        }
        Frame::Square => {
            let (half_width, half_height) = half_extent(&scene.boundary);
            square_corners(centre, half_width.min(half_height)).to_vec()
        }
        Frame::Area => scene
            .boundary
            .exterior()
            .0
            .iter()
            .map(|c| (c.x, c.y))
            .collect(),
    }
}

/// Half the boundary's width and height, in metres.
fn half_extent(boundary: &Polygon<f64>) -> (f64, f64) {
    boundary
        .bounding_rect()
        .map(|rect| (rect.width() / 2.0, rect.height() / 2.0))
        .unwrap_or((0.0, 0.0))
}

/// The ground box `half_width` and `half_height` metres around `centre`.
fn box_around(centre: (f64, f64), half_width: f64, half_height: f64) -> GroundRect<f64> {
    let (x, y) = centre;
    GroundRect::new(
        (x - half_width, y - half_height),
        (x + half_width, y + half_height),
    )
}

fn square_corners(centre: (f64, f64), half: f64) -> [(f64, f64); 4] {
    let (x, y) = centre;
    [
        (x - half, y - half),
        (x + half, y - half),
        (x + half, y + half),
        (x - half, y + half),
    ]
}

/// The boundary cut to the drawing zone, as path data.
fn cropped_outline(boundary: &Polygon<f64>, projection: &Projection, zone: &Rect) -> String {
    let corners = [
        projection.to_ground(zone.x, zone.bottom()),
        projection.to_ground(zone.right(), zone.bottom()),
        projection.to_ground(zone.right(), zone.y),
        projection.to_ground(zone.x, zone.y),
    ];
    let mut ring = corners.to_vec();
    ring.push(corners[0]);
    let zone_on_ground = Polygon::new(ring.into(), vec![]);

    boundary
        .intersection(&zone_on_ground)
        .0
        .iter()
        .map(|part| polygon_path(part, projection))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_corners_run_counterclockwise_from_the_south_west() {
        let corners = square_corners((0.0, 0.0), 1.0);
        assert_eq!(corners[0], (-1.0, -1.0));
        assert_eq!(corners[2], (1.0, 1.0));
    }
}
