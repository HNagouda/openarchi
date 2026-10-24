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
mod drawing;
mod explode;
mod fit;
mod fonts;
mod frame;
mod furniture;
mod geometry;
mod layout;
mod legend;
mod lighting;
mod map;
mod massing;
mod order;
mod patterns;
mod pins;
mod points;
mod section;
mod section_draw;
mod section_levels;
mod section_marker;
mod shadow;
mod simplify;
mod solid;
mod style;
mod sunpath;
mod svg;
mod symbols;
mod text;
mod tools;
mod typeset;
mod walk_bands;

use std::fmt::Write;

use crate::diagram::Diagram;
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
pub(crate) use color::{mix, resolve as resolve_colour, rgb, sample_ramp};
pub(crate) use fonts::Face;
pub use solid::DEFAULT_HEIGHT_M;
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
}

/// The full sheet as an SVG document, sized in millimetres. A diagram with
/// variants is drawn as the look's variant, and a look with a sub-selection
/// draws the scene cut to it.
pub fn render_svg(input: &RenderInput) -> Result<String> {
    let look = input.look;
    let drawn = input.diagram.drawn_as(look.variant.as_deref());
    let scene = selection::framed_or_whole(input.scene, look.selection.as_ref())?;
    let input = &RenderInput {
        diagram: &drawn,
        scene: &scene,
        ..*input
    };
    if look.sheet.scale == 0 {
        return Err(crate::Error::Invalid(
            "scale must be at least 1, as in 1:2500".to_string(),
        ));
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
        ViewMode::Plan => map::plan(input, &layout, &typesetter)?,
        ViewMode::Axonometric => map::axonometric(input, &layout, &typesetter)?,
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
        r#"<rect id="paper" width="{}" height="{}" fill="{}" data-ground-points="{}"/>"#,
        mm(layout.width),
        mm(layout.height),
        palette.paper,
        ground_points
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
