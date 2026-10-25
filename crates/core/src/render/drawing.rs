//! The map itself: one named SVG layer per scene layer, clipped to the frame.

use std::fmt::Write;

use super::classify;
use super::color;

use super::geometry::{decimal, mm, paper_shapes, PaperShapes, Projection};
use super::style::ResolvedLayer;
use super::svg::{escape, layer_attributes, paint_attributes};
use crate::diagram::Diagram;
use crate::scene::{Feature, Layer};
use crate::theme::Theme;

/// Building shadows are the darker of ink and black, laid over whatever is
/// below at this opacity so they read on roads, water and green alike.
const SHADOW_OPACITY: f64 = 0.16;
/// Point symbols with no radius in the theme still need to be visible.
const DEFAULT_POINT_RADIUS_MM: f64 = 0.6;

/// What a layer ended up showing, for the legend.
#[derive(Debug, Clone)]
pub struct Drawn {
    pub has_features: bool,
    /// For a classified layer: which classes have at least one feature.
    pub classes_present: Vec<bool>,
    pub has_unknown: bool,
}

/// "rail_area" -> "rail-area", the same name themes and diagrams use.
pub fn layer_slug(layer: Layer) -> String {
    let name = serde_json::to_value(layer)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string));
    name.unwrap_or_default().replace('_', "-")
}

pub fn pattern_id(scope: &str, layer: Layer) -> String {
    format!("{scope}pattern-{}", layer_slug(layer))
}

/// Where one map goes: its projection, the clip of its frame, and a prefix
/// that keeps ids unique when a sheet holds several maps.
#[derive(Debug, Clone, Copy)]
pub struct Canvas<'a> {
    pub projection: Projection,
    pub clip_id: &'a str,
    pub scope: &'a str,
}

pub fn write_layer(
    out: &mut String,
    resolved: &ResolvedLayer,
    features: &[&Feature],
    diagram: &Diagram,
    theme: &Theme,
    canvas: &Canvas,
) -> Drawn {
    let projection = &canvas.projection;
    let scope = canvas.scope;
    let slug = format!("{scope}{}", layer_slug(resolved.layer));
    let classes = resolved.classes.as_ref();
    let classify = diagram
        .classify
        .as_ref()
        .filter(|classify| classify.layer == resolved.layer);
    let mut drawn = Drawn {
        has_features: !features.is_empty(),
        classes_present: vec![false; classes.map_or(0, |c| c.classes.len())],
        has_unknown: false,
    };
    if features.is_empty() {
        return drawn;
    }

    let style = &resolved.style;
    let fill = if style.pattern.is_some() {
        Some(format!("url(#{})", pattern_id(scope, resolved.layer)))
    } else {
        style.fill.clone()
    };
    let _ = write!(
        out,
        r#"<g {} clip-path="url(#{})" fill-rule="evenodd"{}>"#,
        layer_attributes(
            &format!("{scope}layer-{}", layer_slug(resolved.layer)),
            resolved.layer.label()
        ),
        canvas.clip_id,
        paint_attributes(style, fill.as_deref())
    );
    if resolved.style.shadow_mm > 0.0 {
        write_shadow(
            out,
            &slug,
            features,
            theme,
            projection,
            resolved.style.shadow_mm,
        );
    }

    match (classes, classify) {
        (Some(classes), Some(classify)) => {
            // One sub-group per class, so each class can be selected as a whole in a drawing program.
            let mut slots: Vec<Vec<&Feature>> = vec![Vec::new(); classes.classes.len()];
            let mut unknown: Vec<&Feature> = Vec::new();
            for &feature in features {
                match classify::class_of(feature, classify) {
                    Some(index) if index < slots.len() => slots[index].push(feature),
                    _ => unknown.push(feature),
                }
            }
            for (index, (class, members)) in classes.classes.iter().zip(&slots).enumerate() {
                if members.is_empty() {
                    continue;
                }
                drawn.classes_present[index] = true;
                write_class_group(
                    out,
                    &format!("{slug}-class-{}", index + 1),
                    &class.label,
                    &class.fill,
                    members,
                    style.radius_mm,
                    projection,
                );
            }
            if !unknown.is_empty() {
                drawn.has_unknown = true;
                write_class_group(
                    out,
                    &format!("{slug}-unknown"),
                    &classes.unknown_label,
                    &classes.unknown_fill,
                    &unknown,
                    style.radius_mm,
                    projection,
                );
            }
        }
        _ => write_features(out, features, style.radius_mm, projection),
    }

    out.push_str("</g>\n");
    drawn
}

fn write_class_group(
    out: &mut String,
    id: &str,
    label: &str,
    fill: &str,
    features: &[&Feature],
    radius_mm: f64,
    projection: &Projection,
) {
    let _ = write!(
        out,
        r#"<g id="{id}" inkscape:label="{}" fill="{fill}">"#,
        escape(label)
    );
    write_features(out, features, radius_mm, projection);
    out.push_str("</g>");
}

fn write_features(
    out: &mut String,
    features: &[&Feature],
    radius_mm: f64,
    projection: &Projection,
) {
    let radius = if radius_mm > 0.0 {
        radius_mm
    } else {
        DEFAULT_POINT_RADIUS_MM
    };
    let shapes = write_areas_and_lines(out, features, projection);
    for &(x, y) in &shapes.points {
        let _ = write!(
            out,
            r#"<circle cx="{}" cy="{}" r="{}"/>"#,
            mm(x),
            mm(y),
            mm(radius)
        );
    }
}

// #! NOTE[export, svg] : Exports keep one compound path per layer too, decided 2026-10-25; one path per feature would need an export-only switch
/// Every area of `features` as one path and every line as another, and the
/// points left for the caller. WebKit sets up a fill pattern again for every
/// element it paints, so a large site drawn one path per feature took over
/// half a second to show. Nonzero filling keeps overlapping areas whole,
/// since their rings all turn the same way (`polygon_path`).
pub fn write_areas_and_lines(
    out: &mut String,
    features: &[&Feature],
    projection: &Projection,
) -> PaperShapes {
    let mut shapes = PaperShapes::default();
    for feature in features {
        paper_shapes(&feature.geometry, projection, &mut shapes);
    }
    if !shapes.areas.is_empty() {
        let _ = write!(
            out,
            r#"<path d="{}" fill-rule="nonzero"/>"#,
            shapes.areas.concat()
        );
    }
    if !shapes.lines.is_empty() {
        let _ = write!(out, r#"<path d="{}" fill="none"/>"#, shapes.lines.concat());
    }
    shapes
}

/// The darker of ink and black, so a translucent shadow reads on every theme.
pub fn shadow_colour(theme: &Theme) -> String {
    let ink = color::resolve(&theme.palette, "@ink").unwrap_or_else(|_| "#000000".to_string());
    if color::luminance(&ink) < 0.5 {
        ink
    } else {
        "#000000".to_string()
    }
}

/// All footprints as one path, shifted south-east under the buildings.
fn write_shadow(
    out: &mut String,
    slug: &str,
    features: &[&Feature],
    theme: &Theme,
    projection: &Projection,
    offset_mm: f64,
) {
    let mut shapes = PaperShapes::default();
    for feature in features {
        paper_shapes(&feature.geometry, projection, &mut shapes);
    }
    if shapes.areas.is_empty() {
        return;
    }
    let shadow = shadow_colour(theme);

    let _ = write!(
        out,
        r#"<path id="{slug}-shadow" d="{}" fill="{shadow}" fill-rule="nonzero" stroke="none" opacity="{}" transform="translate({offset} {offset})"/>"#,
        shapes.areas.concat(),
        decimal(SHADOW_OPACITY),
        offset = mm(offset_mm)
    );
}
