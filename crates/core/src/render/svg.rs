//! Small pieces of SVG text shared by the drawing and the furniture.

use super::geometry::{decimal, mm};
use crate::theme::LayerStyle;

/// Text safe inside an element or a double-quoted attribute.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }

    escaped
}

/// Group attributes for a top-level layer that Illustrator, Inkscape and
/// Affinity all list by name in their layers panel.
pub fn layer_attributes(id: &str, label: &str) -> String {
    format!(
        r#"id="{id}" inkscape:groupmode="layer" inkscape:label="{}""#,
        escape(label)
    )
}

/// ` fill=".." stroke=".." ...` for a style, with `fill` given separately so
/// classes and patterns can swap it.
pub fn paint_attributes(style: &LayerStyle, fill: Option<&str>) -> String {
    let mut attributes = String::new();
    match fill {
        Some(fill) => attributes.push_str(&format!(r#" fill="{fill}""#)),
        None => attributes.push_str(r#" fill="none""#),
    }
    match &style.stroke {
        Some(stroke) if style.stroke_mm > 0.0 => {
            attributes.push_str(&format!(
                r#" stroke="{stroke}" stroke-width="{}" stroke-linejoin="round""#,
                mm(style.stroke_mm)
            ));
            // Round caps soften line ends; on dashes they would lengthen every dash.
            if style.dash_mm.is_empty() {
                attributes.push_str(r#" stroke-linecap="round""#);
            } else {
                let dashes: Vec<String> = style.dash_mm.iter().map(|&length| mm(length)).collect();
                attributes.push_str(&format!(r#" stroke-dasharray="{}""#, dashes.join(" ")));
            }
        }
        _ => attributes.push_str(r#" stroke="none""#),
    }
    if style.opacity < 1.0 {
        attributes.push_str(&format!(r#" opacity="{}""#, decimal(style.opacity)));
    }

    attributes
}
