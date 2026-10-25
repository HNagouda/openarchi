//! A plan or an axonometric sheet: one map fitted to the drawing zone, at the
//! sheet's own scale or the nearest round scale that fits.

use geo::CoordsIter;

use super::context::ContextArea;
use super::frame::ground_outline;
use super::geometry::{boundary_reach, furthest_from, ground_centre, Projection};
use super::layout::Layout;
use super::map::{draw, tallest_building, MapSpec, SheetDrawing};
use super::typeset::Typesetter;
use super::RenderInput;
use crate::scene::Layer;
use crate::Result;

/// Room around the furthest walk band, so its edge does not touch the frame.
const WALK_FRAME_MARGIN: f64 = 1.04;
/// Scale denominators a fitted sheet snaps to, a tilted view or a sheet area,
/// so the note reads like a real scale. Down to 1:50, for a sheet of one plot.
pub(super) const NICE_SCALES: [u32; 21] = [
    50, 100, 200, 500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7500, 10_000,
    12_500, 15_000, 20_000, 25_000, 50_000,
];

pub fn plan(input: &RenderInput, layout: &Layout, typesetter: &Typesetter) -> Result<SheetDrawing> {
    let sheet = &input.look.sheet;
    let zone = layout.drawing;
    let mut projection = Projection::new(&input.scene.boundary, sheet.scale, &zone);
    let mut scale = sheet.scale;
    let mut reach_m = None;
    if let Some(walk_reach_m) = walk_reach(input, projection.ground_centre) {
        // Zoom out until the walk fits, and never in past the sheet's own scale.
        let fitting_mm_per_m = zone.width.min(zone.height) / (2.0 * walk_reach_m);
        scale = fitted_scale(sheet.scale, fitting_mm_per_m);
        projection = projection.scaled_to(1000.0 / scale as f64);
        // The frame takes in the whole site too, but stops at the drawing zone,
        // so a site larger than the walk is cut off rather than run into the legend.
        let zone_reach_m = zone.width.min(zone.height) / 2.0 / projection.mm_per_m;
        let site_reach_m = boundary_reach(&input.scene.boundary, projection.ground_centre);
        reach_m = Some(walk_reach_m.max(site_reach_m).min(zone_reach_m));
    }
    let spec = MapSpec {
        diagram: input.diagram,
        projection,
        crop_zone: Some(zone),
        scope: String::new(),
        has_volume: false,
        is_plate: false,
        reach_m,
        context: Some(ContextArea {
            zone,
            reach_m: None,
        }),
        has_highlights: true,
        has_highlight_labels: true,
    };
    let map = draw(input, &spec, typesetter)?;

    Ok(SheetDrawing {
        defs: map.defs,
        body: map.body,
        legend: sheet.show_legend.then_some(map.legend),
        mm_per_m: projection.mm_per_m,
        scale,
        north_deg: 0.0,
        is_foreshortened: false,
        title_name: input.diagram.name.clone(),
        ground: projection,
    })
}

pub fn axonometric(
    input: &RenderInput,
    layout: &Layout,
    typesetter: &Typesetter,
) -> Result<SheetDrawing> {
    let sheet = &input.look.sheet;
    let view = &input.look.view;
    let scene = input.scene;
    let centre = ground_centre(&scene.boundary);
    let unit_view = Projection::tilted(centre, 1.0, view.rotation_deg, view.tilt_deg);
    let tallest = if input.diagram.layers.contains(&Layer::Building) {
        tallest_building(scene)
    } else {
        0.0
    };

    // The frame's outline at ground level and at the tallest roof, at 1 mm per metre.
    let mut bounds = Bounds::default();
    for (x, y) in ground_outline(sheet.frame, scene, centre) {
        bounds.add(unit_view.to_paper_at(x, y, 0.0));
        bounds.add(unit_view.to_paper_at(x, y, tallest));
    }
    let zone = layout.drawing;
    let fitting_mm_per_m = (zone.width / bounds.width()).min(zone.height / bounds.height());
    let scale = fitted_scale(sheet.scale, fitting_mm_per_m);
    let mm_per_m = 1000.0 / scale as f64;
    let (middle_x, middle_y) = bounds.middle();
    let (zone_x, zone_y) = zone.centre();
    let projection = unit_view
        .scaled_to(mm_per_m)
        .moved_to((zone_x - middle_x * mm_per_m, zone_y - middle_y * mm_per_m));

    let spec = MapSpec {
        diagram: input.diagram,
        projection,
        crop_zone: None,
        scope: String::new(),
        has_volume: true,
        is_plate: false,
        reach_m: None,
        context: Some(ContextArea {
            zone,
            reach_m: None,
        }),
        has_highlights: true,
        has_highlight_labels: true,
    };
    let map = draw(input, &spec, typesetter)?;

    Ok(SheetDrawing {
        defs: map.defs,
        body: map.body,
        legend: sheet.show_legend.then_some(map.legend),
        mm_per_m,
        scale,
        north_deg: projection.north_angle_deg(),
        is_foreshortened: true,
        title_name: input.diagram.name.clone(),
        ground: projection,
    })
}

/// The sheet's own scale when the drawing fits at it, or the nearest round scale that does.
pub(super) fn fitted_scale(sheet_scale: u32, fitting_mm_per_m: f64) -> u32 {
    let needed = 1000.0 / fitting_mm_per_m;
    if sheet_scale as f64 >= needed {
        return sheet_scale;
    }

    round_scale(needed, &NICE_SCALES)
}

/// The first of `scales` at least as large as `needed`, else `needed` rounded
/// up to the next thousand: 1:830 -> 1:1000 from the nice scales.
pub(super) fn round_scale(needed: f64, scales: &[u32]) -> u32 {
    scales
        .iter()
        .copied()
        .find(|&scale| scale as f64 >= needed)
        .unwrap_or_else(|| (needed / 1000.0).ceil() as u32 * 1000)
}

/// For a walk diagram, how far its furthest drawn band reaches from `centre`,
/// with a little room. `None` for other diagrams and scenes without a walk.
fn walk_reach(input: &RenderInput, centre: (f64, f64)) -> Option<f64> {
    let minutes = f64::from(input.diagram.walk_minutes?);
    let walk = input.scene.walk.as_ref()?;
    let drawn_points = walk
        .bands
        .iter()
        .filter(|band| {
            band.attributes
                .walk_min
                .is_some_and(|band_min| band_min <= minutes)
        })
        .flat_map(|band| band.geometry.coords_iter())
        .map(|coord| (coord.x, coord.y));
    let furthest = furthest_from(drawn_points, centre);

    (furthest > 0.0).then_some(furthest * WALK_FRAME_MARGIN)
}

/// Paper bounds of a set of points.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: (f64, f64),
    pub max: (f64, f64),
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            min: (f64::INFINITY, f64::INFINITY),
            max: (f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
}

impl Bounds {
    pub fn add(&mut self, (x, y): (f64, f64)) {
        self.min = (self.min.0.min(x), self.min.1.min(y));
        self.max = (self.max.0.max(x), self.max.1.max(y));
    }

    pub fn width(&self) -> f64 {
        (self.max.0 - self.min.0).max(1e-9)
    }

    pub fn height(&self) -> f64 {
        (self.max.1 - self.min.1).max(1e-9)
    }

    pub fn middle(&self) -> (f64, f64) {
        (
            (self.min.0 + self.max.0) / 2.0,
            (self.min.1 + self.max.1) / 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_that_fits_keeps_the_sheet_scale() {
        assert_eq!(fitted_scale(2500, 0.5), 2500);
    }

    #[test]
    fn a_drawing_too_big_snaps_to_the_next_round_scale() {
        // 0.36 mm per metre needs 1:2778, and the next round scale is 1:3000.
        assert_eq!(fitted_scale(2500, 0.36), 3000);
    }
}
