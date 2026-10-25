//! Resolves each diagram layer to one concrete style, in the order the
//! `diagram` module documents: theme, mute, figure, classify, user overrides.

use super::classify::{self, Classes};
use super::color;
use crate::diagram::Diagram;
use crate::project::Look;
use crate::scene::Layer;
use crate::theme::{LayerStyle, Pattern, Theme};
use crate::Result;

/// Stroke in the fill colour on figure layers. Neighbouring tiles of a planar
/// map would otherwise show hairline seams where antialiasing meets.
const FIGURE_SEAM_MM: f64 = 0.25;
/// The ink edge of a blank layer: enough to find a building, too fine to compete with the ground.
const BLANK_EDGE_MM: f64 = 0.1;
const BLANK_TINT: f64 = 0.08;
/// A fine diagonal hatch over a blank layer, so it never reads as the
/// lightest class of the ground under it.
const BLANK_HATCH_ANGLE_DEG: f64 = 45.0;
const BLANK_HATCH_SPACING_MM: f64 = 0.7;
const BLANK_HATCH_STROKE_MM: f64 = 0.08;
const BLANK_HATCH_SHARE: f64 = 0.4;

/// Why a layer looks the way it does. The legend groups rows by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Drawn in its theme style.
    Theme,
    /// Pushed into the background with `@muted`.
    Muted,
    /// Fused into the solid figure of an inverse figure-ground.
    Figure,
    /// Coloured per feature from the ramp or categories.
    Classified,
}

#[derive(Debug, Clone)]
pub struct ResolvedLayer {
    pub layer: Layer,
    /// Every colour already "#RRGGBB". For a classified layer, `fill` is unused.
    pub style: LayerStyle,
    pub role: Role,
    pub classes: Option<Classes>,
}

/// Every drawn layer of the diagram, bottom to top. Layers the theme has no
/// style for, or the user hid, are left out.
pub fn resolve_layers(diagram: &Diagram, theme: &Theme, look: &Look) -> Result<Vec<ResolvedLayer>> {
    let mut resolved = Vec::new();
    for &layer in &diagram.layers {
        let Some(theme_style) = theme.layers.get(&layer) else {
            continue;
        };
        let overrides = look.overrides.get(&layer);
        if overrides.and_then(|o| o.hidden) == Some(true) {
            continue;
        }

        let mut style = theme_style.clone();
        let mut role = Role::Theme;
        let classified = diagram
            .classify
            .as_ref()
            .filter(|classify| classify.layer == layer);
        let is_kept = diagram.emphasis == Some(layer) || diagram.highlight.contains(&layer);
        let is_figure = diagram.as_figure.contains(&layer);
        let is_blank = diagram.blank.contains(&layer);

        let is_quieted = !is_kept && !is_figure && !is_blank && classified.is_none();
        if diagram.mute_others && is_quieted {
            mute(&mut style);
            role = Role::Muted;
        }
        if is_figure {
            make_figure(&mut style);
            role = Role::Figure;
        }
        if is_blank {
            make_blank(&mut style, theme);
        }
        let mut classes = match classified {
            Some(classify) => {
                role = Role::Classified;
                Some(classify::build(classify, theme)?)
            }
            None => None,
        };
        if let Some(overrides) = overrides {
            if overrides.fill.is_some() {
                // A user fill wins over the classes, so the layer turns flat.
                style.fill = overrides.fill.clone();
                style.pattern = None;
                classes = None;
                role = Role::Theme;
            }
            if overrides.stroke.is_some() {
                style.stroke = overrides.stroke.clone();
            }
            if let Some(stroke_mm) = overrides.stroke_mm {
                style.stroke_mm = stroke_mm;
            }
            if let Some(opacity) = overrides.opacity {
                style.opacity = opacity.clamp(0.0, 1.0);
            }
        }

        resolve_colours(&mut style, theme)?;
        resolved.push(ResolvedLayer {
            layer,
            style,
            role,
            classes,
        });
    }

    Ok(resolved)
}

/// Muted layers lose their patterns and shadows too: texture would pull the
/// eye back to the layers the diagram is trying to quiet.
fn mute(style: &mut LayerStyle) {
    let had_fill = style.fill.is_some() || style.pattern.is_some();
    style.fill = had_fill.then(|| "@muted".to_string());
    style.stroke = style.stroke.as_ref().map(|_| "@muted".to_string());
    style.pattern = None;
    style.shadow_mm = 0.0;
}

fn make_figure(style: &mut LayerStyle) {
    style.fill = Some("@figure".to_string());
    style.stroke = Some("@figure".to_string());
    style.stroke_mm = FIGURE_SEAM_MM;
    style.dash_mm.clear();
    style.opacity = 1.0;
    style.pattern = None;
    style.shadow_mm = 0.0;
}

/// Paper lifted a little towards ink, so a blank solid still stands out from
/// the paper in a tilted view, on dark palettes too, and hatched in plan.
fn make_blank(style: &mut LayerStyle, theme: &Theme) {
    let palette = &theme.palette;
    style.fill = Some(color::mix(&palette.paper, &palette.ink, BLANK_TINT));
    style.stroke = Some("@ink".to_string());
    style.stroke_mm = BLANK_EDGE_MM;
    style.dash_mm.clear();
    style.opacity = 1.0;
    style.pattern = Some(Pattern::Hatch {
        angle_deg: BLANK_HATCH_ANGLE_DEG,
        spacing_mm: BLANK_HATCH_SPACING_MM,
        stroke_mm: BLANK_HATCH_STROKE_MM,
        color: color::mix(&palette.paper, &palette.ink, BLANK_HATCH_SHARE),
    });
    style.shadow_mm = 0.0;
}

fn resolve_colours(style: &mut LayerStyle, theme: &Theme) -> Result<()> {
    if let Some(fill) = &style.fill {
        style.fill = Some(color::resolve(&theme.palette, fill)?);
    }
    if let Some(stroke) = &style.stroke {
        style.stroke = Some(color::resolve(&theme.palette, stroke)?);
    }
    if let Some(pattern) = &mut style.pattern {
        let colour = match pattern {
            Pattern::Hatch { color, .. }
            | Pattern::CrossHatch { color, .. }
            | Pattern::Dots { color, .. } => color,
        };
        *colour = color::resolve(&theme.palette, colour)?;
    }

    Ok(())
}
