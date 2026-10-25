//! The neighbourhood around the site, drawn muted outside the boundary.
//!
//! It has no colours of its own. Each layer keeps its theme style, loses most
//! of its colour and moves towards the paper, with thinner lines, so the
//! neighbourhood fits every palette, dark ones included, and never competes
//! with the site. It is clipped to the drawing zone, so it can never reach the
//! furniture beside it, and it stays out of the legend.

use std::collections::BTreeMap;
use std::fmt::Write;

use super::color;
use super::drawing::{layer_slug, Canvas};
use super::geometry::{decimal, mm, paper_shapes, PaperShapes, Projection};
use super::layout::Rect;
use super::svg::{layer_attributes, paint_attributes};
use crate::scene::{Feature, Layer, Scene};
use crate::sheet::Sheet;
use crate::theme::{LayerStyle, Theme};
use crate::Result;

/// How far every colour moves towards its own grey, then towards the paper.
const DESATURATION: f64 = 0.35;
const FADE: f64 = 0.66;
/// Lines are this share of their theme width.
const STROKE_SHARE: f64 = 0.6;
const LABEL: &str = "Neighbourhood";

/// Where a map's neighbourhood may show: inside the drawing zone on paper,
/// and on an explode plate only within `reach_m` of the site centre, so the
/// bottom plate widens into a disc rather than a cut-off sheet.
#[derive(Debug, Clone, Copy)]
pub struct ContextArea {
    pub zone: Rect,
    pub reach_m: Option<f64>,
}

/// The neighbourhood of one map and the clip that keeps it in its area.
#[derive(Debug, Clone)]
pub struct Neighbourhood<'a> {
    pub features: &'a [Feature],
    pub clip_id: String,
    /// The sheet's `context_opacity`, 0 to 1.
    pub opacity: f64,
    area: ContextArea,
}

impl<'a> Neighbourhood<'a> {
    /// `None` when the sheet hides the neighbourhood or the scene has none.
    pub fn of(
        scene: &'a Scene,
        sheet: &Sheet,
        area: Option<ContextArea>,
        scope: &str,
    ) -> Option<Neighbourhood<'a>> {
        let area = area?;
        if !sheet.shows_context() || scene.context.is_empty() {
            return None;
        }

        Some(Neighbourhood {
            features: &scene.context,
            clip_id: format!("{scope}context-clip"),
            opacity: sheet.context_opacity,
            area,
        })
    }

    pub fn write_defs(&self, out: &mut String, projection: &Projection) {
        let zone = &self.area.zone;
        let _ = write!(
            out,
            r#"<clipPath id="{}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#,
            self.clip_id,
            mm(zone.x),
            mm(zone.y),
            mm(zone.width),
            mm(zone.height)
        );
        if let Some(reach_m) = self.area.reach_m {
            // A circle on the ground, which a tilted plate shows as an ellipse.
            let (x, y) =
                projection.to_paper(projection.ground_centre.0, projection.ground_centre.1);
            let radius = reach_m * projection.mm_per_m;
            let _ = write!(
                out,
                r#"<clipPath id="{}-reach"><ellipse cx="{}" cy="{}" rx="{}" ry="{}"/></clipPath>"#,
                self.clip_id,
                mm(x),
                mm(y),
                mm(radius),
                mm(radius * projection.squash)
            );
        }
    }

    /// Every flat layer, bottom to top. Buildings too, unless they stand up
    /// with the site's own in a tilted view.
    pub fn write_ground(
        &self,
        out: &mut String,
        theme: &Theme,
        canvas: &Canvas,
        has_volume: bool,
    ) -> Result<()> {
        let mut by_layer: BTreeMap<Layer, Vec<&Feature>> = BTreeMap::new();
        for feature in self.features {
            by_layer.entry(feature.layer).or_default().push(feature);
        }

        let _ = write!(
            out,
            r#"<g {} clip-path="url(#{})" fill-rule="evenodd"{}>"#,
            layer_attributes(&format!("{}context", canvas.scope), LABEL),
            self.clip_id,
            self.opacity_attribute()
        );
        if self.area.reach_m.is_some() {
            let _ = write!(out, r#"<g clip-path="url(#{}-reach)">"#, self.clip_id);
        }
        for layer in Layer::ALL {
            if has_volume && layer == Layer::Building {
                continue;
            }
            let (Some(features), Some(style)) = (by_layer.get(&layer), theme.layers.get(&layer))
            else {
                continue;
            };
            let style = muted(style, theme)?;
            let _ = write!(
                out,
                r#"<g id="{}context-{}"{}>"#,
                canvas.scope,
                layer_slug(layer),
                paint_attributes(&style, style.fill.as_deref())
            );
            write_shapes(out, features, canvas);
            out.push_str("</g>");
        }
        if self.area.reach_m.is_some() {
            out.push_str("</g>");
        }
        out.push_str("</g>\n");

        Ok(())
    }

    /// ` opacity="0.4"`, or nothing at full opacity, so the usual sheet stays byte for byte the same.
    pub fn opacity_attribute(&self) -> String {
        if self.opacity < 1.0 {
            format!(r#" opacity="{}""#, decimal(self.opacity))
        } else {
            String::new()
        }
    }

    /// The buildings, for a tilted view to stand up among the site's own.
    pub fn buildings(&self) -> Vec<&'a Feature> {
        self.features
            .iter()
            .filter(|feature| feature.layer == Layer::Building)
            .collect()
    }
}

/// The roof of a standing neighbour: `massing`, the fill the site's own
/// buildings stand in, muted like everything else around the site.
pub fn roof(massing: &str, theme: &Theme) -> String {
    fade(massing, &theme.palette.paper)
}

/// A theme style made quiet: faded colours, thinner lines, no pattern or shadow.
fn muted(style: &LayerStyle, theme: &Theme) -> Result<LayerStyle> {
    let paper = &theme.palette.paper;
    let faded = |value: &Option<String>| -> Result<Option<String>> {
        match value {
            Some(value) => Ok(Some(fade(&color::resolve(&theme.palette, value)?, paper))),
            None => Ok(None),
        }
    };

    Ok(LayerStyle {
        fill: faded(&style.fill)?,
        stroke: faded(&style.stroke)?,
        stroke_mm: style.stroke_mm * STROKE_SHARE,
        dash_mm: style.dash_mm.clone(),
        opacity: style.opacity,
        pattern: None,
        radius_mm: style.radius_mm,
        shadow_mm: 0.0,
    })
}

/// "#C2532E" on "#F4F1EA" -> a pale, greyed terracotta.
fn fade(colour: &str, paper: &str) -> String {
    let grey = color::to_hex({
        let level = color::luminance(colour) * 255.0;
        (level, level, level)
    });
    let greyed = color::mix(colour, &grey, DESATURATION);
    color::mix(&greyed, paper, FADE)
}

fn write_shapes(out: &mut String, features: &[&Feature], canvas: &Canvas) {
    for feature in features {
        let mut shapes = PaperShapes::default();
        paper_shapes(&feature.geometry, &canvas.projection, &mut shapes);
        for area in &shapes.areas {
            let _ = write!(out, r#"<path d="{area}"/>"#);
        }
        for line in &shapes.lines {
            let _ = write!(out, r#"<path d="{line}" fill="none"/>"#);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_faded_colour_lies_between_the_colour_and_the_paper() {
        let paper = "#F4F1EA";
        let faded = fade("#2D2A26", paper);
        let lightness = color::luminance(&faded);
        assert!(lightness > color::luminance("#2D2A26"));
        assert!(lightness < color::luminance(paper));
    }

    #[test]
    fn on_a_dark_sheet_a_light_line_fades_towards_the_dark_paper() {
        let faded = fade("#F2F1EE", "#131417");
        assert!(color::luminance(&faded) < color::luminance("#F2F1EE"));
    }
}
