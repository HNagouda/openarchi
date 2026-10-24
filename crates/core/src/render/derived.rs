//! Layers worked out from the scene when a diagram draws them, instead of
//! fetched: city blocks from the street surfaces, hours of sun from building
//! and tree heights. They are computed per drawing and never stored, so an
//! old project draws them too.

use crate::blocks;
use crate::diagram::{Diagram, Scheme};
use crate::scene::{Feature, Layer, Scene};
use crate::sunlight;
use crate::view::SunMoment;

/// The computed features of every computed layer `diagram` draws.
pub fn features(scene: &Scene, diagram: &Diagram, sun: &SunMoment) -> Vec<Feature> {
    let mut features = Vec::new();
    if diagram.layers.contains(&Layer::Block) {
        features.extend(blocks::blocks(scene));
    }
    if diagram.layers.contains(&Layer::SunHours) {
        let grid = sunlight::sun_hours(scene, sun.month, sun.day);
        features.extend(sunlight::bands(&grid, &breaks(diagram, Layer::SunHours)));
    }

    features
}

/// The class breaks the diagram colours `layer` by, so the bands match its legend.
fn breaks(diagram: &Diagram, layer: Layer) -> Vec<f64> {
    let classify = diagram
        .classify
        .as_ref()
        .filter(|classify| classify.layer == layer);
    match classify.map(|classify| &classify.scheme) {
        Some(Scheme::Sequential { breaks, .. }) => breaks.clone(),
        _ => Vec::new(),
    }
}
