//! The section view: an elevation along the sheet's section line.
//!
//! The ground profile comes from the terrain kept with the scene, buildings
//! and trees from the scene itself, so nothing is drawn that the plan does not
//! show. Buildings beyond the line stand in elevation, painted far to near and
//! fading with distance; then the ground, hatched below its profile; then the
//! buildings the line cuts, in solid poché. Water stands between the ground
//! and the cuts, from its level down to its bed. Heights share the scale of
//! lengths unless the user exaggerates them, which the legend then says.

use super::color;
use super::context;
use super::geometry::{mm, Projection};
use super::layout::Layout;
use super::map::SheetDrawing;
use super::map_sheet::round_scale;
use super::massing;
use super::section_draw::{self, Mass};
use super::section_legend::{self, Drawn};
use super::section_levels::{self, Frame};
use super::section_pieces::{self, Building, Cut, AIR_M, EARTH_M};
use super::section_water;
use super::solid;
use super::solid::ESTIMATED_GREY;
use super::style::{self, ResolvedLayer};
use super::typeset::Typesetter;
use super::RenderInput;
use crate::scene::{Feature, Layer};
use crate::section::cut::Stretch;
use crate::section::{SectionLine, SCALES};
use crate::theme::Theme;
use crate::Result;

pub const TITLE: &str = "Section A\u{2013}A\u{2032}";
const CLIP_ID: &str = "section-view";

pub fn render(
    input: &RenderInput,
    layout: &Layout,
    typesetter: &Typesetter,
) -> Result<SheetDrawing> {
    let scene = input.scene;
    let look = input.look;
    let pieces = section_pieces::pieces(scene, look)?;
    let line = pieces.line;
    let axis = &pieces.axis;
    let profile = &pieces.profile;
    let waters = &pieces.waters;
    let theme = input.theme;
    let mut layers = style::resolve_layers(input.diagram, theme, look)?;
    massing::lighten(&mut layers, look, theme);
    let buildings = layers
        .iter()
        .find(|resolved| resolved.layer == Layer::Building);

    let mut masses = Vec::new();
    for building in &pieces.buildings {
        let fill = building_fill(buildings, building.feature, input, theme);
        masses.push(mass_of(building, fill));
    }
    // An elevation is opaque, with nothing behind a neighbour to show through,
    // so a fainter neighbourhood moves its fill towards the paper instead.
    let neighbour_fill = color::mix(
        &context::roof(&massing::theme_fill(theme), theme),
        &theme.palette.paper,
        1.0 - look.sheet.context_opacity,
    );
    for building in &pieces.neighbours {
        masses.push(mass_of(building, neighbour_fill.clone()));
    }
    let tree_fill = tree_fill(&layers, theme);
    for tree in &pieces.trees {
        masses.push(Mass {
            stretch: tree.stretch,
            depth: tree.depth,
            base: tree.base,
            top: tree.base + solid::TREE_HEIGHT_M,
            fill: tree_fill.clone(),
            is_tree: true,
        });
    }
    // Far to near, so what is nearer covers what is behind it.
    masses.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    let cuts: Vec<&Cut> = pieces.cuts().collect();

    let lowest = pieces.lowest() - EARTH_M;
    let highest = masses
        .iter()
        .map(|mass| mass.top)
        .chain(cuts.iter().map(|cut| cut.top))
        .chain(waters.iter().map(|water| water.level_m))
        .fold(profile.highest(), f64::max)
        + AIR_M;
    let room = section_levels::side_room(typesetter);
    let frame = frame_on(layout, room, line, axis.length, lowest, highest);
    let scale = (1000.0 / frame.mm_per_m).round() as u32;

    let mut defs = section_draw::earth_pattern(&theme.palette, layout.unit);
    defs.push_str(&section_water::unknown_depth_gradient(&theme.palette));
    defs.push_str(&format!(
        r#"<clipPath id="{CLIP_ID}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#,
        mm(frame.left),
        mm(frame.top),
        mm(frame.right - frame.left),
        mm(frame.bottom - frame.top)
    ));
    let gaps: Vec<Stretch> = waters
        .iter()
        .filter(|water| !water.has_bed)
        .map(|water| water.stretch)
        .collect();
    let mut body = format!(r#"<g clip-path="url(#{CLIP_ID})">"#);
    section_draw::write_masses(&mut body, &masses, line.depth_m, &frame, theme);
    section_draw::write_ground(&mut body, profile, &gaps, &frame, theme, layout.unit);
    section_water::write(
        &mut body,
        waters,
        profile,
        &frame,
        &theme.palette,
        typesetter,
    );
    section_draw::write_cuts(&mut body, &cuts, &frame, theme);
    body.push_str("</g>\n");
    section_levels::write(&mut body, &frame, profile, &cuts, theme, typesetter);

    let seen: Vec<&Feature> = pieces
        .buildings
        .iter()
        .map(|building| building.feature)
        .collect();
    let legend = section_legend::build(
        input,
        &Drawn {
            buildings,
            seen: &seen,
            profile,
            line,
            has_neighbours: !pieces.neighbours.is_empty(),
            has_trees: !pieces.trees.is_empty(),
            tree_fill: &tree_fill,
            waters,
        },
    );
    // Pins and the app's ground points need a plan; a section has no ground plane on paper.
    let ground = Projection::new(&scene.boundary, scale, &layout.drawing);

    Ok(SheetDrawing {
        defs,
        body,
        legend: look.sheet.show_legend.then_some(legend),
        mm_per_m: frame.mm_per_m,
        scale,
        north_deg: 0.0,
        is_foreshortened: false,
        title_name: TITLE.to_string(),
        ground,
    })
}

/// Where the section goes on the paper: centred in the drawing zone at the
/// line's scale, or the largest round one that fits, with heights drawn
/// `exaggeration` times taller. A scale too large to fit is cut to the zone.
fn frame_on(
    layout: &Layout,
    room: f64,
    line: &SectionLine,
    length: f64,
    lowest: f64,
    highest: f64,
) -> Frame {
    let zone = &layout.drawing;
    let times = f64::from(line.exaggeration);
    let fitting_mm_per_m = ((zone.width - 2.0 * room) / length)
        .min((zone.height - room) / ((highest - lowest) * times));
    let scale = line
        .scale
        .unwrap_or_else(|| section_scale(fitting_mm_per_m));
    let mm_per_m = 1000.0 / f64::from(scale);
    let vertical_mm_per_m = mm_per_m * times;

    let (centre_x, centre_y) = zone.centre();
    let (start_x, end_x) = (
        centre_x - length * mm_per_m / 2.0,
        centre_x + length * mm_per_m / 2.0,
    );
    let full_top = centre_y - (highest - lowest) * vertical_mm_per_m / 2.0;
    let full_bottom = centre_y + (highest - lowest) * vertical_mm_per_m / 2.0;
    // The ruler and the A and A' labels keep their room outside the box.
    let top = full_top.max(zone.y);
    let bottom = full_bottom.min(zone.y + zone.height - room / 2.0);

    Frame {
        start_x,
        end_x,
        left: start_x.max(zone.x + room),
        right: end_x.min(zone.right() - room),
        top,
        bottom,
        highest: highest - (top - full_top) / vertical_mm_per_m,
        mm_per_m,
        vertical_mm_per_m,
    }
}

/// The largest round scale at which the section fits the drawing zone. It
/// picks from the section's own scales, the ones its scale menu offers, so a
/// fitted section can always be set back to the scale it shows.
fn section_scale(fitting_mm_per_m: f64) -> u32 {
    round_scale(1000.0 / fitting_mm_per_m, &SCALES)
}

/// A building as it stands beyond the line, in `fill`.
fn mass_of(building: &Building, fill: String) -> Mass {
    Mass {
        stretch: building.stretch,
        depth: building.depth,
        base: building.base,
        top: building.top,
        fill,
        is_tree: false,
    }
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
