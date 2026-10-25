//! What a section is made of, worked out once for the sheet and its DXF: the
//! ground profile, lowered over water nobody surveyed the bed of; the waters
//! the line crosses; each building within reach, with where the line cuts it;
//! and each tree within reach. Plain data in metres along the line and above
//! NAP, with no colours, so `section.rs` paints it and `export/dxf_section.rs`
//! writes it as entities.

use super::section_water;
use super::solid;
use crate::project::Look;
use crate::scene::{Feature, Layer, Scene};
use crate::section::cut::{self, Stretch};
use crate::section::profile::{self, Profile};
use crate::section::water::{self, Water};
use crate::section::{Axis, SectionLine};
use crate::{Error, Result};

/// Earth drawn below the lowest ground, and air above the tallest roof, in metres.
pub const EARTH_M: f64 = 4.0;
pub const AIR_M: f64 = 3.0;

pub struct Pieces<'a> {
    pub line: &'a SectionLine,
    pub axis: Axis,
    pub profile: Profile,
    pub waters: Vec<Water>,
    /// The site's buildings standing beyond the line, in scene order.
    pub buildings: Vec<Building<'a>>,
    /// The neighbourhood's, when the sheet shows it.
    pub neighbours: Vec<Building<'a>>,
    pub trees: Vec<Tree>,
}

/// A building standing beyond the line, and its cuts where the line passes through it.
pub struct Building<'a> {
    pub feature: &'a Feature,
    pub stretch: Stretch,
    /// How far behind the line its nearest part stands, in metres.
    pub depth: f64,
    pub base: f64,
    pub top: f64,
    pub cuts: Vec<Cut>,
}

/// A stretch of a building the line passes through.
pub struct Cut {
    pub stretch: Stretch,
    pub bottom: f64,
    pub top: f64,
    /// The measured height above the ground, for its label; `None` when guessed.
    pub height_m: Option<f64>,
}

pub struct Tree {
    pub stretch: Stretch,
    pub depth: f64,
    pub base: f64,
}

/// The pieces of the section `look` draws across `scene`, or why it has none.
pub fn pieces<'a>(scene: &'a Scene, look: &'a Look) -> Result<Pieces<'a>> {
    let line = look.section.as_ref().ok_or_else(|| {
        Error::Invalid(
            "This sheet has no section line yet. Draw one on the plan first.".to_string(),
        )
    })?;
    line.check()?;
    let axis = line.axis(scene.origin);
    let profile = profile::ground_profile(scene.terrain.as_ref(), scene.water_bed.as_ref(), &axis);
    let waters = water::crossed(scene, &axis);
    let profile = section_water::lowered(profile, &waters);

    let buildings = buildings_of(&scene.features, scene, line, &axis, &profile);
    let neighbours = if look.sheet.shows_context() {
        buildings_of(&scene.context, scene, line, &axis, &profile)
    } else {
        Vec::new()
    };
    let trees = trees_of(scene, line, &axis, &profile);

    Ok(Pieces {
        line,
        axis,
        profile,
        waters,
        buildings,
        neighbours,
        trees,
    })
}

impl Pieces<'_> {
    /// The lowest ground or water surface along the line.
    pub fn lowest(&self) -> f64 {
        self.waters
            .iter()
            .map(|water| water.level_m)
            .fold(self.profile.lowest(), f64::min)
    }

    /// The cuts through the site's buildings, then the neighbourhood's.
    pub fn cuts(&self) -> impl Iterator<Item = &Cut> {
        self.buildings
            .iter()
            .chain(&self.neighbours)
            .flat_map(|building| &building.cuts)
    }
}

fn buildings_of<'a>(
    features: &'a [Feature],
    scene: &Scene,
    line: &SectionLine,
    axis: &Axis,
    profile: &Profile,
) -> Vec<Building<'a>> {
    let mut buildings = Vec::new();
    for feature in features
        .iter()
        .filter(|candidate| candidate.layer == Layer::Building)
    {
        let Some(seen) = cut::beyond(&feature.geometry, axis, line.depth_m) else {
            continue;
        };
        let (height, is_estimated) = solid::building_height(feature);
        let base = ground_under(
            seen.centre,
            seen.stretch,
            profile,
            scene,
            feature.attributes.ground_m,
        );
        let top = base + height;
        let cuts = cut::cut_stretches(&feature.geometry, axis)
            .into_iter()
            .map(|stretch| {
                // Down to the ground line at either end, so the cut never floats over a slope.
                let ground = profile
                    .height_at(stretch.from)
                    .min(profile.height_at(stretch.to));
                Cut {
                    stretch,
                    bottom: base.min(ground),
                    top,
                    height_m: (!is_estimated).then_some(height),
                }
            })
            .collect();
        buildings.push(Building {
            feature,
            stretch: seen.stretch,
            depth: seen.depth,
            base,
            top,
            cuts,
        });
    }

    buildings
}

fn trees_of(scene: &Scene, line: &SectionLine, axis: &Axis, profile: &Profile) -> Vec<Tree> {
    let mut trees = Vec::new();
    for feature in scene
        .features
        .iter()
        .filter(|candidate| candidate.layer == Layer::Tree)
    {
        let Some(seen) = cut::beyond(&feature.geometry, axis, line.depth_m) else {
            continue;
        };
        trees.push(Tree {
            stretch: seen.stretch,
            depth: seen.depth,
            base: ground_under(seen.centre, seen.stretch, profile, scene, None),
        });
    }

    trees
}

/// The ground a building or tree stands on: the building's own measured
/// ground, else the terrain under its middle, else the profile. On a scene
/// without terrain everything stands at 0, on the flat ground drawn.
fn ground_under(
    centre: (f64, f64),
    stretch: Stretch,
    profile: &Profile,
    scene: &Scene,
    measured_ground: Option<f64>,
) -> f64 {
    if !profile.is_measured {
        return 0.0;
    }
    measured_ground
        .or_else(|| {
            let terrain = scene.terrain.as_ref()?;
            profile::height_at(terrain, centre.0, centre.1)
        })
        .unwrap_or_else(|| profile.height_at((stretch.from + stretch.to) / 2.0))
}

/// The outline of a water with a surveyed bed: along its surface from A' to
/// A, then back along the bed below it, never above the surface.
pub fn water_body(water: &Water, profile: &Profile) -> Vec<(f64, f64)> {
    let (from, to, level) = (water.stretch.from, water.stretch.to, water.level_m);
    let mut body = vec![(to, level), (from, level)];
    body.push((from, profile.height_at(from).min(level)));
    for &(along, height) in &profile.samples {
        if along > from && along < to {
            body.push((along, height.min(level)));
        }
    }
    body.push((to, profile.height_at(to).min(level)));

    body
}
