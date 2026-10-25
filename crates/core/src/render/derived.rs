//! Layers worked out from the scene when a diagram draws them, instead of
//! fetched: city blocks from the street surfaces, hours of sun from building
//! and tree heights, the sky view and the yearly solar radiation from building
//! heights and terrain, and the wind from building footprints and the
//! typical year. They are never stored, so an old project draws them too,
//! except the two that need the kept typical year, and each is kept in
//! `memo.rs` for the next drawing of the same scene. With a model on the
//! site, the layers that read buildings work on the scene with the model in
//! place of the buildings it replaces.

use crate::blocks;
use crate::diagram::{Diagram, Scheme};
use crate::memo::{self, SceneKey};
use crate::model::{self, Model};
use crate::project::Look;
use crate::radiation;
use crate::scene::{Feature, Layer, Scene};
use crate::skyview;
use crate::sunlight;
use crate::wind;

/// The computed layers that read the buildings, and so see the model.
const BUILDING_LAYERS: [Layer; 4] = [
    Layer::SunHours,
    Layer::SkyView,
    Layer::SolarRadiation,
    Layer::Wind,
];

/// The computed features of every computed layer `diagram` draws.
pub fn features(
    scene: &Scene,
    diagram: &Diagram,
    look: &Look,
    model: Option<&Model>,
) -> Vec<Feature> {
    let sun = &look.sun;
    let mut features = Vec::new();
    if diagram.layers.contains(&Layer::Block) {
        let blocks = memo::BLOCKS.kept_or(SceneKey::of(scene), || blocks::blocks(scene));
        features.extend(blocks.iter().cloned());
    }

    // Only copied when a layer below reads it, since a large site is a few megabytes.
    let reads_buildings = BUILDING_LAYERS
        .iter()
        .any(|layer| diagram.layers.contains(layer));
    let with_model = model
        .filter(|_| reads_buildings)
        .map(|model| model::scene_with_model(scene, model));
    let scene = with_model.as_ref().unwrap_or(scene);
    if diagram.layers.contains(&Layer::SunHours) {
        let key = (SceneKey::of(scene), sun.month, sun.day);
        let grid = memo::SUN_HOURS.kept_or(key, || sunlight::sun_hours(scene, sun.month, sun.day));
        features.extend(sunlight::bands(&grid, &breaks(diagram, Layer::SunHours)));
    }
    if diagram.layers.contains(&Layer::SkyView) {
        let grid = memo::SKY_VIEWS.kept_or(SceneKey::of(scene), || skyview::sky_view(scene));
        features.extend(skyview::bands(&grid, &breaks(diagram, Layer::SkyView)));
    }
    // A scene without its typical year draws nothing here; the missing-data note says why.
    if let Some(solar) = scene.solar.as_ref() {
        if diagram.layers.contains(&Layer::SolarRadiation) {
            let grid =
                memo::RADIATION.kept_or(SceneKey::of(scene), || radiation::yearly(scene, solar));
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
