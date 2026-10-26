//! The site as the computed layers see it with a model on it: the model
//! standing as one more building, on a scene that already has what it
//! replaces taken away (`crate::edits::edited`). Hours of sun, sky view,
//! radiation and wind then count the design without knowing about models at all.

use geo::{Geometry, MultiPolygon};

use super::{footprint_on_site, ground_m, Model};
use crate::scene::{Attributes, Feature, Layer, Scene};

// #! NOTE[3d, analysis] : The view diagram's line of sight, sections and the explode view still see the site without the model
/// `scene` with the model standing on it.
pub fn scene_with_model(scene: &Scene, model: &Model) -> Scene {
    let mut with_model = scene.clone();
    with_model.features.push(model_as_building(model, scene));
    with_model
}

/// The model as a building of its footprint and its highest point, standing
/// on its base level, or on the terrain under it like any building.
// #! NOTE[3d, analysis] : The computed layers see the model as one prism at its highest point, so a pitched roof shades a little more than it should
fn model_as_building(model: &Model, scene: &Scene) -> Feature {
    Feature {
        layer: Layer::Building,
        geometry: Geometry::MultiPolygon(MultiPolygon::new(footprint_on_site(model, scene))),
        attributes: Attributes {
            height_m: Some(model.shape().height),
            ground_m: model.placement.base_m.or_else(|| ground_m(model, scene)),
            ..Attributes::default()
        },
    }
}
