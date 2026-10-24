//! Turns a drawn area into a scene: fetch every source, clip, clean, join.

use crate::area::Area;
use crate::scene::Scene;
use crate::Result;

/// One step of a fetch, for a progress bar: "Buildings", 2 of 7.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Progress {
    pub step: String,
    pub done: usize,
    pub total: usize,
}

/// Fetches and assembles everything inside `area`. Fails only when the area is
/// invalid or every source failed; a single failed source becomes a warning.
pub fn build_scene(area: &Area, on_progress: &dyn Fn(Progress)) -> Result<Scene> {
    let _ = (area, on_progress);
    todo!("data layer")
}
