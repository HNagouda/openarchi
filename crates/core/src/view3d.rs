//! The site as triangles for the app's live 3D view: the ground in the
//! palette's colours, every building standing at its height, the trees, the
//! imported model, and where the sun stands through the day.
//!
//! The view only uploads these and moves a light, so its buildings are the
//! prisms the axonometric sheet draws (`render/solid.rs`) and its sun is the
//! one the shadow diagram casts with (`sun.rs`). It is a way of looking, not a
//! sheet: nothing printed comes from it.
//
// #! NOTE[3d, terrain] : The ground is flat, as on the sheets; a site with real relief would want the terrain grid as a surface
// #! NOTE[3d, owner] : "360 view" is an orbit around the site here; is an equirectangular panorama from a point on the site wanted too?

mod surfaces;

use geo::Simplify;
use serde::Serialize;

use crate::model::{self, Model};
use crate::render::{building_height, resolve_colour, tree_size};
use crate::scene::{polygons, Feature, Layer, Scene};
use crate::sun;
use crate::theme::Theme;
use crate::view::SunMoment;
use crate::Result;

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
/// Buildings and the model in neutral greys, so light and shadow do the
/// modelling; the model lighter, so it reads as the new thing on the site.
// #! NOTE[3d, owner] : The 3D view draws buildings grey and the model white rather than in the palette; should the model take the accent here too?
const BUILDING_COLOUR: &str = "#C9C4BA";
const NEIGHBOUR_COLOUR: &str = "#DDD9D1";
const MODEL_COLOUR: &str = "#FAF8F4";
/// The sun's place is given for every this many minutes of the clock day,
/// the step the hours of sun are counted in (`sunlight.rs`).
pub const SUN_STEP_MINUTES: u32 = 10;
const MINUTES_PER_DAY: u32 = 24 * 60;

/// Everything the 3D view draws, in the scene's local metres: x east, y north, z up.
#[derive(Debug, Clone, Serialize)]
pub struct View3d {
    /// The furthest point of the site from its centre, for framing the camera.
    pub radius_m: f64,
    /// The paper colour, behind everything.
    pub sky: String,
    pub ground: Vec<Surface>,
    pub buildings: Surface,
    /// The buildings around the site, outside the boundary.
    pub neighbours: Surface,
    pub model: Option<Surface>,
    pub trees: Trees,
}

/// Triangles in one colour.
#[derive(Debug, Clone, Serialize)]
pub struct Surface {
    pub colour: String,
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

/// The scene, its neighbourhood and the model as the 3D view draws them in `theme`.
pub fn view(scene: &Scene, theme: &Theme, model: Option<&Model>) -> Result<View3d> {
    let mut ground = vec![Surface::of(
        &theme.palette.ground,
        flat(std::slice::from_ref(&scene.boundary), 0.0),
    )];
    for (index, layer) in GROUND_LAYERS.iter().enumerate() {
        let Some(fill) = theme
            .layers
            .get(layer)
            .and_then(|style| style.fill.as_ref())
        else {
            continue;
        };
        let pieces: Vec<_> = scene
            .features
            .iter()
            .filter(|feature| feature.layer == *layer)
            .flat_map(|feature| polygons(&feature.geometry).into_iter())
            .map(|polygon| polygon.simplify(&SIMPLIFY_M))
            .collect();
        if pieces.is_empty() {
            continue;
        }
        let lift = (index + 1) as f64 * GROUND_STEP_M;
        ground.push(Surface::of(
            &resolve_colour(&theme.palette, fill)?,
            flat(&pieces, lift),
        ));
    }

    let is_standing = |feature: &&Feature| {
        feature.layer == Layer::Building && !model.is_some_and(|model| model.replaces(feature))
    };
    let buildings = scene.features.iter().filter(is_standing);
    let neighbours = scene.context.iter().filter(is_standing);
    let (height_m, crown_m) = tree_size();

    Ok(View3d {
        radius_m: radius_m(scene),
        sky: theme.palette.paper.clone(),
        ground,
        buildings: Surface::of(BUILDING_COLOUR, prisms(buildings)),
        neighbours: Surface::of(NEIGHBOUR_COLOUR, prisms(neighbours)),
        model: model.map(|model| {
            Surface::of(
                MODEL_COLOUR,
                Triangles::from(model::triangles_on_site(model, scene)),
            )
        }),
        trees: Trees {
            colour: theme.palette.green.clone(),
            at: trees(scene),
            height_m,
            crown_m,
        },
    })
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
    fn of(colour: &str, triangles: Triangles) -> Surface {
        Surface {
            colour: colour.to_string(),
            triangle_count: triangles.count(),
            corners: triangles.packed(),
        }
    }
}

fn prisms<'a>(features: impl Iterator<Item = &'a Feature>) -> Triangles {
    let mut triangles = Triangles::default();
    for feature in features {
        let (height, _) = building_height(feature);
        for polygon in polygons(&feature.geometry) {
            extrude(&mut triangles, &polygon.simplify(&SIMPLIFY_M), height);
        }
    }
    triangles
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
