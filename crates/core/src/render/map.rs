//! One diagram drawn into one frame. A plan is one map, an axonometric one map
//! with its buildings standing up, and an explode view one flat map per plate.

use std::collections::BTreeMap;
use std::fmt::Write;

use super::color;
use super::drawing::{pattern_id, shadow_colour, write_layer, Canvas};
use super::frame::{ground_outline, FrameShape};
use super::geometry::{ground_centre, Projection};
use super::layout::{Layout, Rect};
use super::legend::{self, Legend, Row, Swatch};
use super::patterns;
use super::shadow;
use super::solid;
use super::style::{self, ResolvedLayer, Role};
use super::sunpath::{self, SunPath};
use super::svg::layer_attributes;
use super::typeset::Typesetter;
use super::RenderInput;
use crate::diagram::{Diagram, Solar};
use crate::scene::{Feature, Layer, Scene};
use crate::sun::{self, SunPosition};
use crate::theme::Theme;
use crate::Result;

/// Shadows in the shadow diagram: clearly there, but the ground still reads through.
const DIAGRAM_SHADOW_OPACITY: f64 = 0.32;
/// Shadows that only light a tilted view: soft, so they model without shouting.
const AMBIENT_SHADOW_OPACITY: f64 = 0.2;
/// A cool blue-grey the ambient shadows lean towards, the colour of shade outdoors.
const COOL_SHADE: &str = "#4A5878";
const COOL_SHARE: f64 = 0.35;
/// Sheets at least this light count as light; dark sheets already have light massing.
const LIGHT_SHEET_LUMINANCE: f64 = 0.5;
/// Building fills darker than this are lightened in a tilted view on a light sheet.
const DARK_MASSING_LUMINANCE: f64 = 0.4;
/// How far a dark building fill moves towards the paper colour.
const MASSING_LIGHTEN: f64 = 0.82;
/// Scale denominators a fitted axonometric snaps to, so the note reads like a real scale.
const NICE_SCALES: [u32; 18] = [
    500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7500, 10_000, 12_500, 15_000,
    20_000, 25_000, 50_000,
];

/// What a sheet's drawing zone ends up holding, for the furniture to describe.
pub struct SheetDrawing {
    pub defs: String,
    pub body: String,
    pub legend: Option<Legend>,
    pub mm_per_m: f64,
    /// The scale the drawing is actually at, which a tilted view may have reduced to fit.
    pub scale: u32,
    pub north_deg: f64,
    pub has_scale_bar: bool,
    pub title_name: String,
}

pub struct MapSpec<'a> {
    pub diagram: &'a Diagram,
    pub projection: Projection,
    /// The zone a plan must stay inside. Tilted views are fitted beforehand.
    pub crop_zone: Option<Rect>,
    pub scope: String,
    /// Buildings and trees stand up, lit and casting soft shadows.
    pub has_volume: bool,
    /// A plate in an explode view: a plain frame edge and no labels.
    pub is_plate: bool,
}

pub struct Map {
    pub defs: String,
    pub body: String,
    pub legend: Legend,
}

pub fn plan(input: &RenderInput, layout: &Layout, typesetter: &Typesetter) -> Result<SheetDrawing> {
    let sheet = &input.look.sheet;
    let projection = Projection::new(&input.scene.boundary, sheet.scale, &layout.drawing);
    let spec = MapSpec {
        diagram: input.diagram,
        projection,
        crop_zone: Some(layout.drawing),
        scope: String::new(),
        has_volume: false,
        is_plate: false,
    };
    let map = draw(input, &spec, typesetter)?;

    Ok(SheetDrawing {
        defs: map.defs,
        body: map.body,
        legend: sheet.show_legend.then_some(map.legend),
        mm_per_m: projection.mm_per_m,
        scale: sheet.scale,
        north_deg: 0.0,
        has_scale_bar: true,
        title_name: input.diagram.name.clone(),
    })
}

pub fn axonometric(
    input: &RenderInput,
    layout: &Layout,
    typesetter: &Typesetter,
) -> Result<SheetDrawing> {
    let sheet = &input.look.sheet;
    let view = &input.look.view;
    let scene = input.scene;
    let centre = ground_centre(&scene.boundary);
    let unit_view = Projection::tilted(centre, 1.0, view.rotation_deg, view.tilt_deg);
    let tallest = if input.diagram.layers.contains(&Layer::Building) {
        tallest_building(scene)
    } else {
        0.0
    };

    // The frame's outline at ground level and at the tallest roof, at 1 mm per metre.
    let mut bounds = Bounds::default();
    for (x, y) in ground_outline(sheet.frame, scene, centre) {
        bounds.add(unit_view.to_paper_at(x, y, 0.0));
        bounds.add(unit_view.to_paper_at(x, y, tallest));
    }
    let zone = layout.drawing;
    let fitting_mm_per_m = (zone.width / bounds.width()).min(zone.height / bounds.height());
    let scale = fitted_scale(sheet.scale, fitting_mm_per_m);
    let mm_per_m = 1000.0 / scale as f64;
    let (middle_x, middle_y) = bounds.middle();
    let (zone_x, zone_y) = zone.centre();
    let projection = unit_view
        .scaled_to(mm_per_m)
        .moved_to((zone_x - middle_x * mm_per_m, zone_y - middle_y * mm_per_m));

    let spec = MapSpec {
        diagram: input.diagram,
        projection,
        crop_zone: None,
        scope: String::new(),
        has_volume: true,
        is_plate: false,
    };
    let map = draw(input, &spec, typesetter)?;

    Ok(SheetDrawing {
        defs: map.defs,
        body: map.body,
        legend: sheet.show_legend.then_some(map.legend),
        mm_per_m,
        scale,
        north_deg: projection.north_angle_deg(),
        has_scale_bar: true,
        title_name: input.diagram.name.clone(),
    })
}

/// The sheet's own scale when the drawing fits at it, or the nearest round scale that does.
fn fitted_scale(sheet_scale: u32, fitting_mm_per_m: f64) -> u32 {
    let needed = 1000.0 / fitting_mm_per_m;
    if sheet_scale as f64 >= needed {
        return sheet_scale;
    }

    NICE_SCALES
        .iter()
        .copied()
        .find(|&scale| scale as f64 >= needed)
        .unwrap_or_else(|| (needed / 1000.0).ceil() as u32 * 1000)
}

fn tallest_building(scene: &Scene) -> f64 {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .map(|feature| solid::building_height(feature).0)
        .fold(0.0, f64::max)
}

/// Paper bounds of a set of points.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: (f64, f64),
    pub max: (f64, f64),
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            min: (f64::INFINITY, f64::INFINITY),
            max: (f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
}

impl Bounds {
    pub fn add(&mut self, (x, y): (f64, f64)) {
        self.min = (self.min.0.min(x), self.min.1.min(y));
        self.max = (self.max.0.max(x), self.max.1.max(y));
    }

    pub fn width(&self) -> f64 {
        (self.max.0 - self.min.0).max(1e-9)
    }

    pub fn height(&self) -> f64 {
        (self.max.1 - self.min.1).max(1e-9)
    }

    pub fn middle(&self) -> (f64, f64) {
        (
            (self.min.0 + self.max.0) / 2.0,
            (self.min.1 + self.max.1) / 2.0,
        )
    }
}

/// Draws one diagram into one frame: ground, layers, shadows, solids, sun path.
pub fn draw(input: &RenderInput, spec: &MapSpec, typesetter: &Typesetter) -> Result<Map> {
    let scene = input.scene;
    let theme = input.theme;
    let look = input.look;
    let diagram = spec.diagram;
    let palette = &theme.palette;
    let scope = spec.scope.as_str();
    let projection = spec.projection;
    let frame = FrameShape::new(
        look.sheet.frame,
        scene,
        &projection,
        spec.crop_zone.as_ref(),
        scope,
    );
    let mut layers = style::resolve_layers(diagram, theme, look)?;
    if !projection.is_plan() {
        // A drop shadow is a flat trick; tilted views cast real ones or none.
        for resolved in &mut layers {
            resolved.style.shadow_mm = 0.0;
        }
    }
    if spec.has_volume {
        lighten_massing(&mut layers, look, theme);
    }
    let features = features_by_layer(scene);
    let canvas = Canvas {
        projection,
        clip_id: frame.clip_id(),
        scope,
    };
    let location = sun::site_location(&scene.area);
    let sun = sun::position_at(location, &look.sun);

    let mut defs = String::new();
    frame.write_defs(&mut defs);
    for resolved in &layers {
        if let Some(pattern) = &resolved.style.pattern {
            defs.push_str(&patterns::pattern_def(
                &pattern_id(scope, resolved.layer),
                pattern,
                resolved.style.fill.as_deref(),
            ));
        }
    }

    let mut body = String::new();
    frame.write_ground(&mut body, &palette.ground, &format!("{scope}ground"));
    let is_shadow_diagram = diagram.solar == Some(Solar::Shadow);
    let casters = shadow_casters(&layers, &features);
    let mut drawn = Vec::with_capacity(layers.len());
    let mut has_shadows = false;
    for resolved in &layers {
        let layer_features = layer_features(&features, resolved.layer);
        let is_caster = matches!(resolved.layer, Layer::Building | Layer::Tree);
        if is_shadow_diagram && !spec.has_volume && is_caster && !has_shadows {
            write_shadows(
                &mut body,
                &casters,
                &sun,
                &canvas,
                theme,
                DIAGRAM_SHADOW_OPACITY,
            );
            has_shadows = true;
        }
        if spec.has_volume && is_caster {
            drawn.push(solid::drawn(resolved, layer_features, diagram));
            continue;
        }
        drawn.push(write_layer(
            &mut body,
            resolved,
            layer_features,
            diagram,
            theme,
            &canvas,
        ));
    }

    if spec.has_volume {
        let opacity = if is_shadow_diagram {
            DIAGRAM_SHADOW_OPACITY
        } else {
            AMBIENT_SHADOW_OPACITY
        };
        write_shadows(&mut body, &casters, &sun, &canvas, theme, opacity);
    }
    write_outlines(&mut body, &frame, spec, theme);
    if spec.has_volume {
        let solid_layer = |layer: Layer| {
            layers
                .iter()
                .find(|resolved| resolved.layer == layer)
                .map(|resolved| (resolved, layer_features(&features, layer)))
        };
        solid::write(
            &mut body,
            solid_layer(Layer::Building),
            solid_layer(Layer::Tree),
            diagram,
            theme,
            &canvas,
            &sun,
        );
    }

    let mut legend = legend::build(&layers, &drawn, diagram, &palette.accent);
    if is_shadow_diagram {
        legend.rows.push(shadow_row(&sun, look, theme));
    }
    if diagram.solar == Some(Solar::SunPath) {
        legend.rows.retain(|row| row.label != legend::SITE_LABEL);
        let path = SunPath {
            location,
            horizon: frame.horizon(),
            projection: &projection,
            palette,
            sun,
            sun_label: look.sun.label(),
            scope,
            has_labels: !spec.is_plate,
        };
        legend
            .rows
            .extend(sunpath::write(&mut body, &path, typesetter));
    }

    Ok(Map { defs, body, legend })
}

/// Standing buildings in a dark figure colour turn into one heavy mass on a
/// light sheet, so unclassified ones become a light neutral instead and
/// colour stays for diagrams that classify. A fill the user set is kept.
fn lighten_massing(layers: &mut [ResolvedLayer], look: &crate::project::Look, theme: &Theme) {
    let paper = &theme.palette.paper;
    let has_user_fill = look
        .overrides
        .get(&Layer::Building)
        .is_some_and(|edit| edit.fill.is_some());
    if has_user_fill || color::luminance(paper) < LIGHT_SHEET_LUMINANCE {
        return;
    }
    for resolved in layers.iter_mut() {
        let is_plain = matches!(resolved.role, Role::Theme | Role::Figure);
        if resolved.layer != Layer::Building || !is_plain {
            continue;
        }
        if let Some(fill) = &resolved.style.fill {
            if color::luminance(fill) < DARK_MASSING_LUMINANCE {
                resolved.style.fill = Some(color::mix(fill, paper, MASSING_LIGHTEN));
            }
        }
    }
}

fn write_outlines(out: &mut String, frame: &FrameShape, spec: &MapSpec, theme: &Theme) {
    let id = format!("{}site", spec.scope);
    // A plate, and the sun path whose horizon is the frame, get a plain ink edge.
    if spec.is_plate || spec.diagram.solar == Some(Solar::SunPath) {
        let _ = writeln!(
            out,
            "<g {}>{}</g>",
            layer_attributes(&id, "Frame"),
            frame.edge(&theme.palette.ink)
        );
    } else {
        frame.write_outlines(out, &theme.palette, &id);
    }
}

/// Buildings and trees the diagram draws, which are the ones that cast shadows.
fn shadow_casters<'a>(
    layers: &[ResolvedLayer],
    features: &BTreeMap<Layer, Vec<&'a Feature>>,
) -> Vec<&'a Feature> {
    layers
        .iter()
        .filter(|resolved| matches!(resolved.layer, Layer::Building | Layer::Tree))
        .flat_map(|resolved| layer_features(features, resolved.layer).iter().copied())
        .collect()
}

fn write_shadows(
    out: &mut String,
    casters: &[&Feature],
    sun: &SunPosition,
    canvas: &Canvas,
    theme: &Theme,
    opacity: f64,
) {
    let colour = color_for_shadows(theme, opacity);
    let _ = write!(
        out,
        r#"<g {} clip-path="url(#{})" stroke="none" fill="{colour}">"#,
        layer_attributes(&format!("{}shadows", canvas.scope), "Shadows"),
        canvas.clip_id
    );
    shadow::write(
        out,
        casters,
        sun,
        &canvas.projection,
        &shadow::opacity_attribute(opacity),
    );
    out.push_str("</g>\n");
}

/// Diagram shadows in the theme's shadow colour; the softer ambient ones lean cool.
fn color_for_shadows(theme: &Theme, opacity: f64) -> String {
    let base = shadow_colour(theme);
    if opacity < DIAGRAM_SHADOW_OPACITY {
        color::mix(&base, COOL_SHADE, COOL_SHARE)
    } else {
        base
    }
}

fn shadow_row(sun: &SunPosition, look: &crate::project::Look, theme: &Theme) -> Row {
    let label = if sun.altitude_deg < shadow::MIN_SUN_ALTITUDE_DEG {
        format!("No sun on {}", look.sun.label())
    } else {
        format!("Shadow on {}", look.sun.label())
    };
    Row {
        label,
        swatch: Swatch::Area {
            fill: shadow_colour(theme),
            stroke: None,
            stroke_mm: 0.0,
            opacity: DIAGRAM_SHADOW_OPACITY,
        },
    }
}

fn layer_features<'a, 'b>(
    features: &'b BTreeMap<Layer, Vec<&'a Feature>>,
    layer: Layer,
) -> &'b [&'a Feature] {
    features.get(&layer).map(Vec::as_slice).unwrap_or(&[])
}

fn features_by_layer(scene: &Scene) -> BTreeMap<Layer, Vec<&Feature>> {
    let mut grouped: BTreeMap<Layer, Vec<&Feature>> = BTreeMap::new();
    for feature in &scene.features {
        grouped.entry(feature.layer).or_default().push(feature);
    }

    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_that_fits_keeps_the_sheet_scale() {
        assert_eq!(fitted_scale(2500, 0.5), 2500);
    }

    #[test]
    fn a_drawing_too_big_snaps_to_the_next_round_scale() {
        // 0.36 mm per metre needs 1:2778, and the next round scale is 1:3000.
        assert_eq!(fitted_scale(2500, 0.36), 3000);
    }
}
