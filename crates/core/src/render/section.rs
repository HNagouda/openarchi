//! The section view: an elevation along the sheet's section line.
//!
//! The ground profile comes from the terrain kept with the scene, buildings
//! and trees from the scene itself, so nothing is drawn that the plan does not
//! show. Buildings beyond the line stand in elevation, painted far to near and
//! fading with distance; then the ground, hatched below its profile; then the
//! buildings the line cuts, in solid poché. Heights and lengths share one
//! scale: nothing is exaggerated.

use super::color;
use super::context;
use super::geometry::Projection;
use super::layout::Layout;
use super::legend::{self, Legend, Row, Swatch};
use super::map::SheetDrawing;
use super::massing;
use super::section_draw::{self, Cut, Mass};
use super::section_levels::{self, Frame};
use super::solid;
use super::style::{self, ResolvedLayer, Role};
use super::typeset::Typesetter;
use super::RenderInput;
use crate::scene::{Feature, Layer};
use crate::section::cut::{self, Stretch};
use crate::section::profile::{self, Profile};
use crate::section::Axis;
use crate::theme::Theme;
use crate::{Error, Result};

pub const TITLE: &str = "Section A\u{2013}A\u{2032}";
/// Scales a section snaps to, largest first is not needed: the first that fits wins.
const SCALES: [u32; 16] = [
    100, 200, 250, 500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 5000, 7500, 10_000, 20_000,
];
/// Earth drawn below the lowest ground, and air above the tallest roof, in metres.
const EARTH_M: f64 = 4.0;
const AIR_M: f64 = 3.0;
/// How far a building of unknown height moves towards the unknown colour, as in the axonometric.
const ESTIMATED_GREY: f64 = 0.65;
/// Legend swatches of what stands beyond show it halfway to the far edge.
const MIDDLE_DEPTH: f64 = 0.5;
const FLAT_NOTE: &str = "Ground drawn flat: this site was fetched before ground heights were kept. Fetch it again for the terrain.";

pub fn render(
    input: &RenderInput,
    layout: &Layout,
    typesetter: &Typesetter,
) -> Result<SheetDrawing> {
    let scene = input.scene;
    let look = input.look;
    let line = look.section.as_ref().ok_or_else(|| {
        Error::Invalid(
            "This sheet has no section line yet. Draw one on the plan first.".to_string(),
        )
    })?;
    line.check()?;
    let axis = line.axis(scene.origin);
    let profile = profile::ground_profile(scene.terrain.as_ref(), &axis);
    let theme = input.theme;
    let mut layers = style::resolve_layers(input.diagram, theme, look)?;
    massing::lighten(&mut layers, look, theme);
    let buildings = layers
        .iter()
        .find(|resolved| resolved.layer == Layer::Building);

    let mut site_buildings: Vec<&Feature> = layer_of(&scene.features, Layer::Building);
    let neighbours = if look.sheet.show_context && look.sheet.context_opacity > 0.0 {
        layer_of(&scene.context, Layer::Building)
    } else {
        Vec::new()
    };
    let trees = layer_of(&scene.features, Layer::Tree);
    let mut masses = Vec::new();
    let mut cuts = Vec::new();
    for feature in &site_buildings {
        let fill = building_fill(buildings, feature, input, theme);
        add_building(
            feature,
            fill,
            &axis,
            line.depth_m,
            &profile,
            scene,
            &mut masses,
            &mut cuts,
        );
    }
    // An elevation is opaque, with nothing behind a neighbour to show through,
    // so a fainter neighbourhood moves its fill towards the paper instead.
    let neighbour_fill = color::mix(
        &context::roof(&massing::theme_fill(theme), theme),
        &theme.palette.paper,
        1.0 - look.sheet.context_opacity,
    );
    let site_masses = masses.len();
    for feature in &neighbours {
        let fill = neighbour_fill.clone();
        add_building(
            feature,
            fill,
            &axis,
            line.depth_m,
            &profile,
            scene,
            &mut masses,
            &mut cuts,
        );
    }
    let has_neighbours = masses.len() > site_masses;
    let tree_fill = tree_fill(&layers, theme);
    for feature in &trees {
        if let Some(seen) = cut::beyond(&feature.geometry, &axis, line.depth_m) {
            let base = ground_under(seen.centre, seen.stretch, &profile, scene, None);
            masses.push(Mass {
                stretch: seen.stretch,
                depth: seen.depth,
                base,
                top: base + solid::TREE_HEIGHT_M,
                fill: tree_fill.clone(),
                is_tree: true,
            });
        }
    }
    // Far to near, so what is nearer covers what is behind it.
    masses.sort_by(|a, b| b.depth.total_cmp(&a.depth));

    let lowest = profile.lowest() - EARTH_M;
    let highest = masses
        .iter()
        .map(|mass| mass.top)
        .chain(cuts.iter().map(|cut| cut.top))
        .fold(profile.highest(), f64::max)
        + AIR_M;
    let room = section_levels::side_room(typesetter);
    let fitting_mm_per_m = ((layout.drawing.width - 2.0 * room) / axis.length)
        .min((layout.drawing.height - room) / (highest - lowest));
    let scale = section_scale(fitting_mm_per_m);
    let mm_per_m = 1000.0 / scale as f64;
    let (centre_x, centre_y) = layout.drawing.centre();
    let frame = Frame {
        left: centre_x - axis.length * mm_per_m / 2.0,
        right: centre_x + axis.length * mm_per_m / 2.0,
        top: centre_y - (highest - lowest) * mm_per_m / 2.0,
        bottom: centre_y + (highest - lowest) * mm_per_m / 2.0,
        highest,
        mm_per_m,
    };

    let defs = section_draw::earth_pattern(&theme.palette, layout.unit);
    let mut body = String::new();
    section_draw::write_masses(&mut body, &masses, line.depth_m, &frame, theme);
    section_draw::write_ground(&mut body, &profile, &frame, theme, layout.unit);
    section_draw::write_cuts(&mut body, &cuts, &frame, theme);
    section_levels::write(&mut body, &frame, &profile, &cuts, theme, typesetter);

    let has_trees = masses.iter().any(|mass| mass.is_tree);
    site_buildings.retain(|feature| cut::beyond(&feature.geometry, &axis, line.depth_m).is_some());
    let legend = legend_for(
        input,
        buildings,
        &site_buildings,
        &profile,
        line.depth_m,
        has_neighbours,
        has_trees,
        &tree_fill,
    );
    // Pins and the app's ground points need a plan; a section has no ground plane on paper.
    let ground = Projection::new(&scene.boundary, scale, &layout.drawing);

    Ok(SheetDrawing {
        defs,
        body,
        legend: look.sheet.show_legend.then_some(legend),
        mm_per_m,
        scale,
        north_deg: 0.0,
        is_foreshortened: false,
        title_name: TITLE.to_string(),
        ground,
    })
}

/// The largest round scale at which the section fits the drawing zone.
// #! NOTE[sections, scale] : A section picks its own scale to fill the sheet rather than the sheet's scale. Should it follow the sheet's scale like a plan?
fn section_scale(fitting_mm_per_m: f64) -> u32 {
    let needed = 1000.0 / fitting_mm_per_m;
    SCALES
        .iter()
        .copied()
        .find(|&scale| scale as f64 >= needed)
        .unwrap_or_else(|| (needed / 1000.0).ceil() as u32 * 1000)
}

fn layer_of(features: &[Feature], layer: Layer) -> Vec<&Feature> {
    features
        .iter()
        .filter(|feature| feature.layer == layer)
        .collect()
}

/// A building's elevation beyond the line, and its cut where the line passes through it.
#[allow(clippy::too_many_arguments)]
fn add_building(
    feature: &Feature,
    fill: String,
    axis: &Axis,
    depth_m: f64,
    profile: &Profile,
    scene: &crate::scene::Scene,
    masses: &mut Vec<Mass>,
    cuts: &mut Vec<Cut>,
) {
    let (height, is_estimated) = solid::building_height(feature);
    let measured_ground = feature.attributes.ground_m;
    let stretches = cut::cut_stretches(&feature.geometry, axis);
    let Some(seen) = cut::beyond(&feature.geometry, axis, depth_m) else {
        return;
    };
    let base = ground_under(seen.centre, seen.stretch, profile, scene, measured_ground);
    masses.push(Mass {
        stretch: seen.stretch,
        depth: seen.depth,
        base,
        top: base + height,
        fill,
        is_tree: false,
    });
    for stretch in stretches {
        // Down to the ground line at either end, so the poché never floats over a slope.
        let ground = profile
            .height_at(stretch.from)
            .min(profile.height_at(stretch.to));
        cuts.push(Cut {
            stretch,
            bottom: base.min(ground),
            top: base + height,
            height_m: (!is_estimated).then_some(height),
        });
    }
}

/// The ground a building or tree stands on: the building's own measured
/// ground, else the terrain under its middle, else the profile. On a scene
/// without terrain everything stands at 0, on the flat ground drawn.
fn ground_under(
    centre: (f64, f64),
    stretch: Stretch,
    profile: &Profile,
    scene: &crate::scene::Scene,
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

/// The diagram's colour for a building: its class, or the light massing of a
/// plain building, greyed when its height is a guess.
fn building_fill(
    buildings: Option<&ResolvedLayer>,
    feature: &Feature,
    input: &RenderInput,
    theme: &Theme,
) -> String {
    let fill = match buildings {
        Some(resolved) => solid::roof_fill(resolved, feature, input.diagram, theme),
        None => massing::theme_fill(theme),
    };
    if solid::building_height(feature).1 {
        color::mix(&fill, &theme.palette.unknown, ESTIMATED_GREY)
    } else {
        fill
    }
}

/// The crown colour: the tree's fill in the diagram or theme, or for a theme
/// that draws trees as outlines, the outline's colour.
fn tree_fill(layers: &[ResolvedLayer], theme: &Theme) -> String {
    let resolved = layers.iter().find(|resolved| resolved.layer == Layer::Tree);
    let style = theme.layers.get(&Layer::Tree);
    let theme_colour = style
        .and_then(|style| style.fill.as_deref().or(style.stroke.as_deref()))
        .and_then(|colour| color::resolve(&theme.palette, colour).ok());
    resolved
        .and_then(|resolved| resolved.style.fill.clone())
        .or(theme_colour)
        .unwrap_or_else(|| theme.palette.green.clone())
}

/// The diagram's building classes when it has them, then what a section shows.
#[allow(clippy::too_many_arguments)]
fn legend_for(
    input: &RenderInput,
    buildings: Option<&ResolvedLayer>,
    seen: &[&Feature],
    profile: &Profile,
    depth_m: f64,
    has_neighbours: bool,
    has_trees: bool,
    tree_fill: &str,
) -> Legend {
    let palette = &input.theme.palette;
    let classified = buildings.filter(|resolved| resolved.role == Role::Classified);
    let mut legend = match classified {
        Some(resolved) => {
            let drawn = solid::drawn(resolved, seen, input.diagram);
            let mut legend = legend::build(
                std::slice::from_ref(resolved),
                &[drawn],
                input.diagram,
                &palette.accent,
            );
            legend.rows.retain(|row| row.label != legend::SITE_LABEL);
            // The note names the year of the data, which only the scene knows.
            if legend.note.is_some() {
                legend.note = input
                    .diagram
                    .classify
                    .as_ref()
                    .and_then(|classify| classify.note_for(input.scene));
            }
            legend
        }
        None => Legend {
            title: TITLE.to_string(),
            note: None,
            rows: vec![area_row(
                "Buildings beyond",
                &section_draw::beyond_fill(
                    &massing::theme_fill(input.theme),
                    &palette.paper,
                    MIDDLE_DEPTH,
                ),
            )],
        },
    };
    legend
        .rows
        .push(area_row("Cut", section_draw::poche(palette)));
    if has_neighbours {
        let roof = context::roof(&massing::theme_fill(input.theme), input.theme);
        let fill = section_draw::beyond_fill(&roof, &palette.paper, MIDDLE_DEPTH);
        legend.rows.push(area_row("Neighbourhood beyond", &fill));
    }
    if has_trees {
        legend.rows.push(Row {
            label: "Trees beyond".to_string(),
            swatch: Swatch::Dot {
                fill: Some(tree_fill.to_string()),
                // The crowns have the elevation's ink edge, which a white crown needs to show.
                stroke: Some(palette.ink.clone()),
                opacity: 1.0,
            },
        });
    }
    legend.rows.push(area_row(
        "Ground",
        &format!("url(#{})", section_draw::HATCH_ID),
    ));
    // The flat ground matters more than a class note: it says the drawing is not the terrain.
    if !profile.is_measured {
        legend.note = Some(FLAT_NOTE.to_string());
    } else if legend.note.is_none() {
        legend.note = Some(format!(
            "Heights in metres above NAP. Seen up to {depth_m:.0} m beyond the line."
        ));
    }

    legend
}

fn area_row(label: &str, fill: &str) -> Row {
    Row {
        label: label.to_string(),
        swatch: Swatch::Area {
            fill: fill.to_string(),
            stroke: None,
            stroke_mm: 0.0,
            opacity: 1.0,
        },
    }
}
