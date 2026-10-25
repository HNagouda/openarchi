//! A map's layers in the diagram's order: flat areas and lines, walk bands
//! and points drawn straight away, buildings and trees of a tilted view set
//! aside to stand up later, and a shadow diagram's shadows slipped in under
//! the first layer that casts them.

use std::collections::BTreeMap;

use super::drawing::{write_layer, Canvas, Drawn};
use super::legend::{self, Row};
use super::map::layer_features;
use super::map_shadows::{write_shadows, Casters, DIAGRAM_SHADOW_OPACITY};
use super::points::{self, PointLayer};
use super::solid;
use super::style::ResolvedLayer;
use super::typeset::Typesetter;
use super::walk_bands::{self, Bands};
use crate::diagram::{Diagram, Solar};
use crate::scene::{Feature, Layer};
use crate::sun::SunPosition;
use crate::theme::Theme;

/// What drawing the layers reads.
pub(super) struct LayerPass<'a, 'f> {
    pub layers: &'a [ResolvedLayer],
    pub features: &'a BTreeMap<Layer, Vec<&'f Feature>>,
    pub casters: &'a Casters<'f>,
    pub sun: &'a SunPosition,
    pub canvas: &'a Canvas<'a>,
    pub theme: &'a Theme,
    pub diagram: &'a Diagram,
    pub typesetter: &'a Typesetter,
    pub has_volume: bool,
    pub has_labels: bool,
}

/// What the legend needs to know about the drawn layers.
pub(super) struct FlatLayers {
    /// One per layer, in the layers' order.
    pub drawn: Vec<Drawn>,
    pub band_rows: Vec<Row>,
    pub transit_lines: Vec<String>,
    pub has_merged_points: bool,
}

pub(super) fn write(out: &mut String, pass: &LayerPass) -> FlatLayers {
    let palette = &pass.theme.palette;
    let is_shadow_diagram = pass.diagram.solar == Some(Solar::Shadow);
    let mut flat = FlatLayers {
        drawn: Vec::with_capacity(pass.layers.len()),
        band_rows: Vec::new(),
        transit_lines: Vec::new(),
        has_merged_points: false,
    };
    let mut has_shadows = false;
    for resolved in pass.layers {
        let layer_features = layer_features(pass.features, resolved.layer);
        let is_caster = matches!(resolved.layer, Layer::Building | Layer::Tree);
        if is_shadow_diagram && !pass.has_volume && is_caster && !has_shadows {
            write_shadows(
                out,
                pass.casters,
                pass.sun,
                pass.canvas,
                pass.theme,
                DIAGRAM_SHADOW_OPACITY,
            );
            has_shadows = true;
        }
        if pass.has_volume && is_caster {
            flat.drawn
                .push(solid::drawn(resolved, layer_features, pass.diagram));
            continue;
        }
        if resolved.layer == Layer::WalkBand {
            let bands = Bands {
                resolved,
                features: layer_features,
                canvas: pass.canvas,
                paper: &palette.paper,
                typesetter: pass.typesetter,
                has_labels: pass.has_labels,
            };
            flat.band_rows = walk_bands::write(out, &bands);
            // The bands bring their own legend rows, one per band.
            flat.drawn.push(Drawn {
                has_features: false,
                classes_present: Vec::new(),
                has_unknown: false,
            });
            continue;
        }
        if legend::is_point_layer(resolved.layer) {
            let point_layer = PointLayer {
                resolved,
                features: layer_features,
                diagram: pass.diagram,
                canvas: pass.canvas,
                paper: &palette.paper,
                typesetter: pass.typesetter,
                has_labels: pass.has_labels,
            };
            let drawn_points = points::write(out, &point_layer);
            flat.transit_lines.extend(drawn_points.lines);
            flat.has_merged_points |= drawn_points.has_merged;
            flat.drawn.push(drawn_points.drawn);
            continue;
        }
        flat.drawn.push(write_layer(
            out,
            resolved,
            layer_features,
            pass.diagram,
            pass.theme,
            pass.canvas,
        ));
    }

    flat
}
