//! The site as triangles for the app's live 3D view: the ground, every
//! building standing at its height, the neighbourhood, the trees, the
//! imported model, and where the sun stands through the day, all in the
//! colours of the sheet the view opened from (`view3d/colours.rs`).
//!
//! The view only uploads these and moves a light, so its buildings are the
//! prisms the axonometric sheet draws (`render/solid.rs`) and its sun is the
//! one the shadow diagram casts with (`sun.rs`). It is a way of looking, not a
//! sheet: nothing printed comes from it.
//
// #! NOTE[3d, terrain] : The ground is flat, as on the sheets; AHN's grid is in `Scene::terrain`, but draping the ground layers over it means cutting every area into the grid's cells, which was left for a site with real relief
// #! NOTE[3d, owner] : "360 view" is an orbit around the site here; is an equirectangular panorama from a point on the site wanted too?

mod colours;
pub mod photo;
mod surfaces;

use std::collections::BTreeMap;

use geo::{Polygon, Simplify};
use serde::Serialize;

use crate::floors::STOREY_HEIGHT_M;
use crate::model;
use crate::render::{building_height, tree_size, RenderInput};
use crate::scene::{polygons, Feature, Layer, Scene};
use crate::sun;
use crate::view::SunMoment;
use crate::viewshed;
use crate::Result;

use self::colours::Colours;
use self::surfaces::{extrude, flat, Triangles};

/// Ground layers bottom to top, each lifted a little above the one under it so
/// they do not flicker where they overlap.
const GROUND_LAYERS: [Layer; 13] = [
    Layer::Unpaved,
    Layer::Paved,
    Layer::Yard,
    Layer::Green,
    Layer::Verge,
    Layer::WaterBank,
    Layer::Water,
    Layer::RailArea,
    Layer::Road,
    Layer::Parking,
    Layer::Cycleway,
    Layer::Footpath,
    Layer::Bridge,
];
const GROUND_STEP_M: f64 = 0.03;
/// Outlines lose corners closer than this to the line between their
/// neighbours: footpaths and kerbs are surveyed to the centimetre, which a
/// view of the whole site cannot show, and it halves what the view receives.
const SIMPLIFY_M: f64 = 0.15;
/// The sun's place is given for every this many minutes of the clock day,
/// the step the hours of sun are counted in (`sunlight.rs`).
pub const SUN_STEP_MINUTES: u32 = 10;
const MINUTES_PER_DAY: u32 = 24 * 60;

/// Everything the 3D view draws, in the scene's local metres: x east, y north, z up.
#[derive(Debug, Clone, Serialize)]
pub struct View3d {
    /// The furthest point of the site from its centre, for framing the camera.
    pub radius_m: f64,
    /// Where a standing eye is, the View diagram's (`viewshed.rs`).
    pub eye_height_m: f64,
    /// The storey the floor counts assume (`floors.rs`), for window rows.
    pub storey_height_m: f64,
    pub sky: Sky,
    /// The site's open ground first, then each ground layer on top.
    pub ground: Vec<Surface>,
    /// The ground layers of the neighbourhood, muted.
    pub surroundings: Vec<Surface>,
    /// The site's buildings, one surface per colour the diagram gives them.
    pub buildings: Vec<Surface>,
    /// The buildings around the site, outside the boundary.
    pub neighbours: Surface,
    pub model: Option<Surface>,
    pub trees: Trees,
}

/// The sky in the sheet's palette: the paper at the horizon, so the far
/// ground fades into the sheet, and a little of the water colour overhead.
#[derive(Debug, Clone, Serialize)]
pub struct Sky {
    pub horizon: String,
    pub zenith: String,
}

/// Triangles in one colour.
#[derive(Debug, Clone, Serialize)]
pub struct Surface {
    pub colour: String,
    /// The ground layer these triangles are, so a view in real colours can
    /// paint water as water; `None` for buildings and the site's open ground.
    pub layer: Option<Layer>,
    /// For buildings, how many triangles each building has, in order, so a
    /// view can vary its facades building by building. Empty otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<u32>,
    /// Every corner of every triangle as three f32 (x, y, z), little-endian,
    /// in base64: a large site crosses to the window as a few MB of text
    /// rather than tens of MB of JSON numbers.
    pub corners: String,
    pub triangle_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Trees {
    pub colour: String,
    /// Where each tree stands, x and y.
    pub at: Vec<[f32; 2]>,
    pub height_m: f64,
    pub crown_m: f64,
}

/// Where the sun stands through one day, at the site.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SunDay {
    pub month: u32,
    pub day: u32,
    /// Every `SUN_STEP_MINUTES` of the Dutch clock day, midnight to midnight.
    pub steps: Vec<SunStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SunStep {
    /// Minutes after midnight on the clock, summer time included.
    pub minute: u32,
    pub azimuth_deg: f64,
    pub altitude_deg: f64,
}

/// The sheet's scene, its neighbourhood and the model as the 3D view draws
/// them, in the sheet's diagram and palette.
pub fn view(input: &RenderInput) -> Result<View3d> {
    let scene = input.scene;
    let colours = Colours::of(input)?;
    let model = input.model;
    let is_standing = |feature: &&Feature| {
        feature.layer == Layer::Building && !model.is_some_and(|model| model.replaces(feature))
    };
    let neighbours = scene.context.iter().filter(is_standing);
    let (height_m, crown_m) = tree_size();

    Ok(View3d {
        radius_m: radius_m(scene),
        eye_height_m: viewshed::EYE_HEIGHT_M,
        storey_height_m: STOREY_HEIGHT_M,
        sky: colours.sky(),
        ground: ground(scene, &colours)?,
        surroundings: surroundings(scene, &colours)?,
        buildings: buildings(scene.features.iter().filter(is_standing), &colours),
        neighbours: prisms(&colours.neighbour(), neighbours),
        model: model.map(|model| {
            Surface::of(
                &colours.model(),
                None,
                Triangles::from(model::triangles_on_site(model, scene)),
            )
        }),
        trees: Trees {
            colour: colours.trees()?,
            at: trees(scene),
            height_m,
            crown_m,
        },
    })
}

/// The site's open ground, then every ground layer the palette fills, each a
/// little above the one under it.
fn ground(scene: &Scene, colours: &Colours) -> Result<Vec<Surface>> {
    let mut surfaces = vec![Surface::of(
        &colours.open_ground(),
        None,
        flat(std::slice::from_ref(&scene.boundary), 0.0),
    )];
    for (index, layer) in GROUND_LAYERS.iter().enumerate() {
        let Some(colour) = colours.ground(*layer)? else {
            continue;
        };
        let pieces = outlines(&scene.features, *layer);
        if !pieces.is_empty() {
            let triangles = flat(&pieces, lift(index));
            surfaces.push(Surface::of(&colour, Some(*layer), triangles));
        }
    }

    Ok(surfaces)
}

/// The ground layers of the neighbourhood, muted, at the heights the site's own take.
fn surroundings(scene: &Scene, colours: &Colours) -> Result<Vec<Surface>> {
    let mut surfaces = Vec::new();
    for (index, layer) in GROUND_LAYERS.iter().enumerate() {
        let Some(colour) = colours.surrounding(*layer)? else {
            continue;
        };
        let pieces = outlines(&scene.context, *layer);
        if !pieces.is_empty() {
            let triangles = flat(&pieces, lift(index));
            surfaces.push(Surface::of(&colour, Some(*layer), triangles));
        }
    }

    Ok(surfaces)
}

/// The buildings as prisms, gathered by colour so each colour is one surface.
fn buildings<'a>(features: impl Iterator<Item = &'a Feature>, colours: &Colours) -> Vec<Surface> {
    let mut by_colour: BTreeMap<String, Vec<&Feature>> = BTreeMap::new();
    for feature in features {
        by_colour
            .entry(colours.building(feature))
            .or_default()
            .push(feature);
    }

    by_colour
        .into_iter()
        .map(|(colour, features)| prisms(&colour, features.into_iter()))
        .collect()
}

fn outlines(features: &[Feature], layer: Layer) -> Vec<Polygon<f64>> {
    features
        .iter()
        .filter(|feature| feature.layer == layer)
        .flat_map(|feature| polygons(&feature.geometry).into_iter())
        .map(|polygon| polygon.simplify(&SIMPLIFY_M))
        .collect()
}

/// The height of the ground layer at `index` of `GROUND_LAYERS`.
fn lift(index: usize) -> f64 {
    (index + 1) as f64 * GROUND_STEP_M
}

/// The sun at the site on `month`/`day`, every `SUN_STEP_MINUTES` of the clock.
pub fn sun_day(scene: &Scene, month: u32, day: u32) -> SunDay {
    let location = sun::site_location(&scene.area);
    let steps = (0..=MINUTES_PER_DAY)
        .step_by(SUN_STEP_MINUTES as usize)
        .map(|minute| {
            let moment = SunMoment {
                month,
                day,
                hour: minute / 60,
                minute: minute % 60,
            };
            let position = sun::position_at(location, &moment);
            SunStep {
                minute,
                azimuth_deg: position.azimuth_deg,
                altitude_deg: position.altitude_deg,
            }
        })
        .collect();

    SunDay { month, day, steps }
}

impl Surface {
    fn of(colour: &str, layer: Option<Layer>, triangles: Triangles) -> Surface {
        Surface {
            colour: colour.to_string(),
            layer,
            pieces: Vec::new(),
            triangle_count: triangles.count(),
            corners: triangles.packed(),
        }
    }
}

/// Buildings stood up at their heights, as one surface in `colour`.
fn prisms<'a>(colour: &str, features: impl Iterator<Item = &'a Feature>) -> Surface {
    let mut triangles = Triangles::default();
    let mut pieces = Vec::new();
    for feature in features {
        let before = triangles.count();
        let (height, _) = building_height(feature);
        for polygon in polygons(&feature.geometry) {
            extrude(&mut triangles, &polygon.simplify(&SIMPLIFY_M), height);
        }
        pieces.push((triangles.count() - before) as u32);
    }

    Surface {
        pieces,
        ..Surface::of(colour, None, triangles)
    }
}

fn trees(scene: &Scene) -> Vec<[f32; 2]> {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Tree)
        .filter_map(|feature| match feature.geometry {
            geo::Geometry::Point(point) => Some([point.x() as f32, point.y() as f32]),
            _ => None,
        })
        .collect()
}

fn radius_m(scene: &Scene) -> f64 {
    scene
        .boundary
        .exterior()
        .0
        .iter()
        .map(|c| c.x.hypot(c.y))
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests;
