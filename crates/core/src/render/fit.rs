//! The scale at which a sheet's area fills its drawing zone, for a sheet whose
//! sub-selection just changed: a small area at the whole site's 1:2500 would
//! be a stamp in the middle of the paper.

use super::frame::ground_outline;
use super::geometry::ground_centre;
use super::layout::Layout;
use super::map::{Bounds, NICE_SCALES};
use crate::project::Look;
use crate::scene::Scene;
use crate::selection;
use crate::Result;

/// The largest round scale at which the look's area, in its frame, fits the
/// drawing zone. Tilted views shrink further on their own when they need to.
pub fn fitting_scale(scene: &Scene, look: &Look) -> Result<u32> {
    let scene = selection::framed_or_whole(scene, look.selection.as_ref())?;
    let centre = ground_centre(&scene.boundary);
    let mut bounds = Bounds::default();
    for point in ground_outline(look.sheet.frame, &scene, centre) {
        bounds.add(point);
    }
    let zone = Layout::new(&look.sheet).drawing;
    let fitting_mm_per_m = (zone.width / bounds.width()).min(zone.height / bounds.height());
    let needed = 1000.0 / fitting_mm_per_m;

    Ok(NICE_SCALES
        .iter()
        .copied()
        .find(|&scale| scale as f64 >= needed)
        .unwrap_or_else(|| (needed / 1000.0).ceil() as u32 * 1000))
}
