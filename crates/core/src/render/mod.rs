//! Resolves a scene, diagram, theme and sheet into one SVG document.
//!
//! This SVG is the single source of truth for 2D: the app shows it as the
//! preview, and PDF and PNG are converted from it. It is sized in millimetres
//! with a viewBox in millimetres, so one user unit is one millimetre of paper.

mod classify;
mod color;
mod drawing;
mod fonts;
mod frame;
mod furniture;
mod geometry;
mod layout;
mod legend;
mod patterns;
mod style;
mod svg;
mod tools;
mod typeset;

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::diagram::Diagram;
use crate::project::Look;
use crate::scene::{Feature, Layer, Scene};
use crate::theme::Theme;
use crate::Result;

// Shared with the catalog (validating themes) and export (fonts, CAD layer names).
pub(crate) use color::{is_hex, role, ROLES};
pub(crate) use drawing::layer_slug;
pub(crate) use fonts::{is_bundled as is_bundled_font, BUNDLED as BUNDLED_FONTS};
// Shared with ratios, so its rows take the same classes and colours as the diagrams.
pub(crate) use classify::{build as build_classes, class_of};
pub(crate) use color::{mix, resolve as resolve_colour, sample_ramp};
pub(crate) use fonts::Face;
pub(crate) use svg::{escape, layer_attributes};

use drawing::{pattern_id, write_layer};
use frame::FrameShape;
use furniture::Furniture;
use geometry::{mm, Projection};
use layout::Layout;
use typeset::Typesetter;

/// Everything the renderer needs, already resolved from ids.
pub struct RenderInput<'a> {
    pub scene: &'a Scene,
    pub diagram: &'a Diagram,
    pub theme: &'a Theme,
    pub look: &'a Look,
}

/// The full sheet as an SVG document, sized in millimetres.
pub fn render_svg(input: &RenderInput) -> Result<String> {
    let sheet = &input.look.sheet;
    if sheet.scale == 0 {
        return Err(crate::Error::Invalid(
            "scale must be at least 1, as in 1:2500".to_string(),
        ));
    }
    let palette = &input.theme.palette;
    let layout = Layout::new(sheet);
    let projection = Projection::new(&input.scene.boundary, sheet.scale, &layout.drawing);
    let frame = FrameShape::new(sheet.frame, input.scene, &projection, &layout.drawing);
    let layers = style::resolve_layers(input.diagram, input.theme, input.look)?;
    let features = features_by_layer(input.scene);

    let mut out = String::with_capacity(estimated_size(input.scene));
    write_header(&mut out, &layout);
    out.push_str("<defs>");
    frame.write_defs(&mut out);
    for resolved in &layers {
        if let Some(pattern) = &resolved.style.pattern {
            out.push_str(&patterns::pattern_def(
                &pattern_id(resolved.layer),
                pattern,
                resolved.style.fill.as_deref(),
            ));
        }
    }
    out.push_str("</defs>\n");

    let _ = writeln!(
        out,
        r#"<rect id="paper" width="{}" height="{}" fill="{}"/>"#,
        mm(layout.width),
        mm(layout.height),
        palette.paper
    );
    frame.write_ground(&mut out, &palette.ground);
    let mut drawn = Vec::with_capacity(layers.len());
    for resolved in &layers {
        let layer_features = features
            .get(&resolved.layer)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        drawn.push(write_layer(
            &mut out,
            resolved,
            layer_features,
            input.diagram,
            input.theme,
            &projection,
        ));
    }
    frame.write_outlines(&mut out, palette);

    let typesetter = Typesetter::new(
        &input.theme.typography,
        layout.unit,
        &palette.ink,
        &palette.accent,
    );
    let furniture = Furniture {
        sheet,
        scene: input.scene,
        diagram_name: &input.diagram.name,
        palette,
        mm_per_m: projection.mm_per_m,
        legend: sheet
            .show_legend
            .then(|| legend::build(&layers, &drawn, input.diagram, &palette.accent)),
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

fn features_by_layer(scene: &Scene) -> BTreeMap<Layer, Vec<&Feature>> {
    let mut grouped: BTreeMap<Layer, Vec<&Feature>> = BTreeMap::new();
    for feature in &scene.features {
        grouped.entry(feature.layer).or_default().push(feature);
    }

    grouped
}

/// About 120 bytes of SVG per feature, so the buffer rarely has to grow.
fn estimated_size(scene: &Scene) -> usize {
    16_384 + scene.features.len() * 120
}
