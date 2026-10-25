//! What a sheet asks for that its scene does not have, so the app can say why
//! a diagram looks empty instead of showing a quiet map.
//!
//! A scene keeps only what was fetched, with the code of that day. A site
//! fetched before a source was added, or while the source was down, has no
//! features of it, and its diagrams draw only the base map. Telling that apart
//! from a site that truly has nothing (no flood risk, no shops) needs the
//! scene's own record of what was fetched: the walk network for OpenStreetMap,
//! and a measured figure for each raster.

use crate::diagram::Diagram;
use crate::project::Look;
use crate::scene::{Layer, Scene};
use crate::view::ViewMode;

/// Every layer measured from a raster, the ones `Measured` keeps a figure for.
const RASTERS: [Layer; 7] = [
    Layer::RoadNoise,
    Layer::AllNoise,
    Layer::No2,
    Layer::Pm10,
    Layer::Pm25,
    Layer::Flood,
    Layer::Slope,
];

/// One sentence per thing the sheet wants and the scene lacks, empty when nothing is missing.
pub fn notes(scene: &Scene, diagram: &Diagram, look: &Look) -> Vec<String> {
    let mut notes = Vec::new();
    // An explode view draws several diagrams; its plates say for themselves what they show.
    if look.view.mode != ViewMode::Explode {
        if let Some(layer) = subject(diagram).filter(|layer| is_missing(scene, *layer)) {
            notes.push(format!(
                "This site has no {} data yet. It was fetched before openarchi read that source, or the source could not be reached. Fetch the site again to add it.",
                layer.label().to_lowercase()
            ));
        }
    }
    if look.sheet.show_context && scene.context.is_empty() {
        notes.push("This site was fetched before openarchi kept the neighbourhood around it. Fetch the site again to show it.".to_string());
    }

    notes
}

/// The layer a diagram is about: the one it classifies, or else the one it emphasises.
fn subject(diagram: &Diagram) -> Option<Layer> {
    diagram
        .classify
        .as_ref()
        .map(|classify| classify.layer)
        .or(diagram.emphasis)
}

fn is_missing(scene: &Scene, layer: Layer) -> bool {
    let has_features = scene.features.iter().any(|feature| feature.layer == layer);
    if has_features {
        return false;
    }
    match layer {
        // OpenStreetMap layers come in one request with the walk network.
        Layer::Business
        | Layer::Facility
        | Layer::TransitStop
        | Layer::WalkPath
        | Layer::WalkBand => scene.walk.is_none(),
        // A site without flood risk has no flood figure either, so one raster
        // proves nothing; a scene with no measured raster at all was fetched before them.
        Layer::RoadNoise
        | Layer::AllNoise
        | Layer::No2
        | Layer::Pm10
        | Layer::Pm25
        | Layer::Flood
        | Layer::Slope => RASTERS
            .iter()
            .all(|raster| scene.measured.get(*raster).is_none()),
        // A site can have no trees, no green or no band of a raster, so only
        // the credit, which a scene gets for each source it read, tells.
        Layer::TreeCanopy | Layer::GreenCover | Layer::VegetationIndex | Layer::HeatIsland => {
            !scene
                .credits
                .iter()
                .any(|credit| credit.layers.contains(&layer))
        }
        // CBS cells cover the whole country, so none at all means the source was never read.
        Layer::Population => true,
        // Worked out when drawn, but from the typical year kept at fetch time.
        Layer::SolarRadiation => scene.solar.is_none(),
        Layer::Wind => scene.wind.is_none(),
        _ => false,
    }
}
