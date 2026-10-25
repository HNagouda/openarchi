//! Resolves a scene, diagram, theme and sheet into one SVG document.
//!
//! This SVG is the single source of truth: the app shows it as the preview, and
//! PDF and PNG are converted from it. It is sized in millimetres with a viewBox
//! in millimetres, so one user unit is one millimetre of paper. The tilted
//! views are drawn here too, as plain shapes, so they export like a plan does.

mod classify;
mod color;
mod context;
mod cull;
mod derived;
mod drawing;
mod explode;
mod fit;
mod fonts;
mod frame;
mod furniture;
mod geometry;
mod highlights;
mod layout;
mod legend;
mod legend_draw;
mod lighting;
mod map;
mod map_legend;
mod map_shadows;
mod map_sheet;
mod massing;
mod missing;
mod order;
mod patterns;
mod pins;
mod points;
mod section;
mod section_draw;
mod section_legend;
mod section_levels;
mod section_marker;
mod section_water;
mod shadow;
mod simplify;
mod solid;
mod solid_draw;
mod style;
mod sunpath;
mod sunpath_guides;
mod svg;
mod symbols;
mod text;
mod title_block;
mod tools;
mod typeset;
mod viewpoint;
mod walk_bands;

use std::fmt::Write;

use crate::diagram::Diagram;
use crate::highlight::Highlight;
use crate::pin::Pin;
use crate::project::Look;
use crate::scene::{Layer, Scene};
use crate::selection;
use crate::sheet::Sheet;
use crate::theme::Theme;
use crate::view::ViewMode;
use crate::Result;

// Shared with the catalog (validating themes) and export (fonts, CAD layer names).
pub(crate) use color::{is_hex, role, ROLES};
pub(crate) use drawing::layer_slug;
pub use fit::fitting_scale;
pub use fonts::FAMILIES as FONT_FAMILIES;
pub(crate) use fonts::{is_bundled as is_bundled_font, BUNDLED as BUNDLED_FONTS};
// Shared with ratios, so its rows take the same classes and colours as the diagrams.
pub(crate) use classify::{build as build_classes, class_of};
// Shared with the DXF export, so CAD gets the classes, computed layers,
// highlights and section the sheet draws, worked out the same way.
pub(crate) use classify::Classes;
pub(crate) use color::{mix, resolve as resolve_colour, rgb, sample_ramp};
pub(crate) use derived::features as computed_features;
pub(crate) use fonts::Face;
pub(crate) use highlights::shown as shown_highlights;
pub(crate) use section::ground_under as section_ground_under;
pub(crate) use section_water::lowered as lowered_over_water;
pub(crate) use solid::tree_size;
pub use solid::DEFAULT_HEIGHT_M;
// Shared with the sunlight hours, so they cast exactly the shadows the shadow diagram draws.
pub(crate) use shadow::{shadow_per_metre, signed_area, tree_shadow_ring};
pub(crate) use solid::building_height;
pub(crate) use svg::{escape, layer_attributes};
// Shared with the analysis pages, which wrap their notes like sheet text.
pub(crate) use color::luminance;
pub(crate) use fonts::{text_width, wrap as wrap_text};
// Shared with report text blocks, so they are set exactly like sheet text.
pub(crate) use text::{close_group, open_group, write_lines, StyledText};
pub(crate) use typeset::{Kind as TextKind, Typesetter};

use furniture::Furniture;
use geometry::mm;
use layout::Layout;
use pins::PinLayer;
use text::SheetTexts;

/// What a section draws whatever the diagram, so their sources are credited.
const SECTION_LAYERS: [Layer; 2] = [Layer::Building, Layer::Tree];

/// Everything the renderer needs, already resolved from ids.
pub struct RenderInput<'a> {
    pub scene: &'a Scene,
    pub diagram: &'a Diagram,
    pub theme: &'a Theme,
    pub look: &'a Look,
    /// The project's pins, in RD New metres. Drawn unless the look hides them.
    pub pins: &'a [Pin],
    /// The project's highlights, in RD New metres. Drawn unless the look hides them.
    pub highlights: &'a [Highlight],
}

/// The full sheet as an SVG document, sized in millimetres. A diagram with
/// variants is drawn as the look's variant, and a look with a sub-selection
/// draws the scene cut to it.
/// What the sheet asks for that its scene lacks, as the sentences the app
/// shows over the sheet: an old fetch without a diagram's source, or without
/// the neighbourhood. Empty when the sheet has all it needs.
pub fn missing_data(input: &RenderInput) -> Result<Vec<String>> {
    let look = input.look;
    // The look's variant decides which layer the diagram is about.
    let drawn = input.diagram.drawn_as(look.variant.as_deref());
    let scene = selection::framed_or_whole(input.scene, look.selection.as_ref())?;
    Ok(missing::notes(&scene, &drawn, look))
}

/// The look as `diagram` can draw it: a plan-only diagram turns a tilted or
/// exploded view into its plan. The app and set_look never ask for one; a
/// look from elsewhere still draws.
fn planned(diagram: &Diagram, look: &Look) -> Look {
    let is_tilted = matches!(look.view.mode, ViewMode::Axonometric | ViewMode::Explode);
    let mut planned = look.clone();
    if diagram.plan_only && is_tilted {
        planned.view.mode = ViewMode::Plan;
    }
    planned
}

pub fn render_svg(input: &RenderInput) -> Result<String> {
    let drawn = input.diagram.drawn_as(input.look.variant.as_deref());
    let look = &planned(&drawn, input.look);
    let scene = selection::framed_or_whole(input.scene, look.selection.as_ref())?;
    let input = &RenderInput {
        diagram: &drawn,
        scene: &scene,
        look,
        ..*input
    };
    if look.sheet.scale == 0 {
        return Err(crate::Error::Invalid(
            "scale must be at least 1, as in 1:2500".to_string(),
        ));
    }
    if !(0.0..=1.0).contains(&look.sheet.context_opacity) {
        return Err(crate::Error::Invalid(format!(
            "context_opacity must be between 0 and 1, got {}",
            look.sheet.context_opacity
        )));
    }
    if let Some(problem) = look.sun.problem() {
        return Err(crate::Error::Invalid(format!("sun: {problem}")));
    }
    let palette = &input.theme.palette;
    let layout = Layout::new(&look.sheet);
    let typesetter = Typesetter::new(
        &input.theme.typography,
        layout.unit,
        &palette.ink,
        &palette.accent,
    );
    let drawing = match look.view.mode {
        ViewMode::Plan => map_sheet::plan(input, &layout, &typesetter)?,
        ViewMode::Axonometric => map_sheet::axonometric(input, &layout, &typesetter)?,
        ViewMode::Explode => explode::render(input, &layout, &typesetter)?,
        ViewMode::Section => section::render(input, &layout, &typesetter)?,
    };
    // A section has no ground plane on paper, so pins and the section's own marker stay off it.
    let is_section = look.view.mode == ViewMode::Section;
    let ground_points = if is_section {
        String::new()
    } else {
        pins::ground_points(&drawing.ground, input.scene.origin)
    };

    let mut out = String::with_capacity(estimated_size(drawing.body.len()));
    write_header(&mut out, &layout);
    let _ = writeln!(out, "<defs>{}</defs>", drawing.defs);
    // The ground's place on the paper, so the app can turn a drop into a pin.
    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{}" height="{}" fill="{}" data-ground-points="{}" data-missing="{}"/>"#,
        mm(layout.width),
        mm(layout.height),
        palette.paper,
        ground_points,
        svg::escape(&missing::notes(input.scene, input.diagram, look).join("\n"))
    );
    out.push_str(&drawing.body);
    if let Some(line) = look.section.as_ref().filter(|_| !is_section) {
        section_marker::write(
            &mut out,
            line,
            input.scene,
            &drawing.ground,
            palette,
            &typesetter,
        );
    }
    if !look.hide_pins && !is_section {
        let pin_layer = PinLayer {
            pins: input.pins,
            scene: input.scene,
            ground: drawing.ground,
            palette,
            typesetter: &typesetter,
        };
        pins::write(&mut out, &pin_layer);
    }

    // The furniture describes the drawing as drawn: a fitted scale, a turned north.
    let sheet = Sheet {
        scale: drawing.scale,
        show_north_arrow: look.sheet.show_north_arrow && !is_section,
        ..look.sheet.clone()
    };
    let texts = SheetTexts::resolve(&sheet, &typesetter, palette)?;
    let furniture = Furniture {
        texts: &texts,
        sheet: &sheet,
        scene: input.scene,
        layers: if is_section {
            &SECTION_LAYERS
        } else {
            &input.diagram.layers
        },
        diagram_name: &drawing.title_name,
        palette,
        mm_per_m: drawing.mm_per_m,
        is_foreshortened: drawing.is_foreshortened,
        north_deg: drawing.north_deg,
        legend: drawing.legend,
    };
    furniture::write(&mut out, &furniture, &layout, &typesetter);
    out.push_str("</svg>\n");

    Ok(out)
}

fn write_header(out: &mut String, layout: &Layout) {
    let width = mm(layout.width);
    let height = mm(layout.height);
    let _ = writeln!(
        out,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="{width}mm" height="{height}mm" viewBox="0 0 {width} {height}">"#
    );
}

/// The drawing plus room for the header and furniture, so the buffer rarely has to grow.
fn estimated_size(body_len: usize) -> usize {
    16_384 + body_len
}
