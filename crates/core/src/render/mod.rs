//! Resolves a scene, diagram, theme and sheet into one SVG document.
//!
//! This SVG is the single source of truth: the app shows it as the preview, and
//! PDF and PNG are converted from it. It is sized in millimetres with a viewBox
//! in millimetres, so one user unit is one millimetre of paper. The tilted
//! views are drawn here too, as plain shapes, so they export like a plan does.

mod classify;
mod color;
mod drawing;
mod explode;
mod fonts;
mod frame;
mod furniture;
mod geometry;
mod layout;
mod legend;
mod map;
mod order;
mod patterns;
mod points;
mod shadow;
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
use crate::project::Look;
use crate::scene::Scene;
use crate::sheet::Sheet;
use crate::theme::Theme;
use crate::view::ViewMode;
use crate::Result;

// Shared with the catalog (validating themes) and export (fonts, CAD layer names).
pub(crate) use color::{is_hex, role, ROLES};
pub(crate) use drawing::layer_slug;
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

use furniture::Furniture;
use geometry::mm;
use layout::Layout;
use text::SheetTexts;
use typeset::Typesetter;

/// Everything the renderer needs, already resolved from ids.
pub struct RenderInput<'a> {
    pub scene: &'a Scene,
    pub diagram: &'a Diagram,
    pub theme: &'a Theme,
    pub look: &'a Look,
}

/// The full sheet as an SVG document, sized in millimetres. A diagram with
/// variants is drawn as the look's variant.
pub fn render_svg(input: &RenderInput) -> Result<String> {
    let look = input.look;
    let drawn = input.diagram.drawn_as(look.variant.as_deref());
    let input = &RenderInput {
        diagram: &drawn,
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
    };

    let mut out = String::with_capacity(estimated_size(drawing.body.len()));
    write_header(&mut out, &layout);
    let _ = writeln!(out, "<defs>{}</defs>", drawing.defs);
    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{}" height="{}" fill="{}"/>"#,
        mm(layout.width),
        mm(layout.height),
        palette.paper
    );
    out.push_str(&drawing.body);

    // The furniture describes the drawing as drawn: a fitted scale, a turned north.
    let sheet = Sheet {
        scale: drawing.scale,
        show_scale_bar: look.sheet.show_scale_bar && drawing.has_scale_bar,
        ..look.sheet.clone()
    };
    let texts = SheetTexts::resolve(&sheet, &typesetter, palette)?;
    let furniture = Furniture {
        texts: &texts,
        sheet: &sheet,
        scene: input.scene,
        layers: &input.diagram.layers,
        diagram_name: &drawing.title_name,
        palette,
        mm_per_m: drawing.mm_per_m,
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
