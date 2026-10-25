//! Layers worked out from the scene when a diagram draws them, instead of
//! fetched: city blocks from the street surfaces, hours of sun from building
//! and tree heights, the sky view and the yearly solar radiation from building
//! heights and terrain, and the wind from building footprints and the
//! typical year. They are computed per drawing and never stored, so an old
//! project draws them too, except the two that need the kept typical year.

use crate::blocks;
use crate::diagram::{Diagram, Scheme};
use crate::project::Look;
use crate::radiation;
use crate::scene::{Feature, Layer, Scene};
use crate::skyview;
use crate::sunlight;
use crate::wind;

/// The computed features of every computed layer `diagram` draws.
pub fn features(scene: &Scene, diagram: &Diagram, look: &Look) -> Vec<Feature> {
    let sun = &look.sun;
    let mut features = Vec::new();
    if diagram.layers.contains(&Layer::Block) {
        features.extend(blocks::blocks(scene));
    }
    if diagram.layers.contains(&Layer::SunHours) {
        let grid = sunlight::sun_hours(scene, sun.month, sun.day);
        features.extend(sunlight::bands(&grid, &breaks(diagram, Layer::SunHours)));
    }
    if diagram.layers.contains(&Layer::SkyView) {
        let grid = skyview::sky_view(scene);
        features.extend(skyview::bands(&grid, &breaks(diagram, Layer::SkyView)));
    }
    // A scene without its typical year draws nothing here; the missing-data note says why.
    if let Some(solar) = scene.solar.as_ref() {
        if diagram.layers.contains(&Layer::SolarRadiation) {
            let grid = radiation::yearly(scene, solar);
            features.extend(radiation::bands(
                &grid,
                &breaks(diagram, Layer::SolarRadiation),
            ));
        }
    }
    // Likewise without the kept wind; a plan draws it, the app and set_look keep it off tilted views.
    if diagram.layers.contains(&Layer::Wind) {
        if let Some(prevailing) = wind::prevailing(scene, look.wind_season) {
            let flow = wind::flow(scene, prevailing.from_deg);
            features.extend(wind::features(&flow, &breaks(diagram, Layer::Wind)));
        }
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
