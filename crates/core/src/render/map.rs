//! One diagram drawn into one frame. A plan is one map, an axonometric one map
//! with its buildings standing up, and an explode view one flat map per plate.

use std::collections::BTreeMap;
use std::fmt::Write;

use geo::CoordsIter;

use super::color;
use super::context::{self, ContextArea, Neighbourhood};
use super::cull;
use super::derived;
use super::drawing::{pattern_id, shadow_colour, write_layer, Canvas, Drawn};
use super::frame::{ground_outline, FrameShape};
use super::geometry::{ground_centre, Projection};
use super::highlights::{self, HighlightLayer};
use super::layout::{Layout, Rect};
use super::legend::{self, Legend, Row, Swatch};
use super::massing;
use super::patterns;
use super::points::{self, PointLayer};
use super::shadow;
use super::solid::{self, Neighbours};
use super::style::{self, ResolvedLayer};
use super::sunpath::{self, SunPath};
use super::svg::layer_attributes;
use super::typeset::Typesetter;
use super::viewpoint;
use super::walk_bands::{self, Bands};
use super::RenderInput;
use crate::diagram::{Diagram, Solar, DATE_PLACEHOLDER};
use crate::scene::{Feature, Layer, Scene};
use crate::sources::osm;
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
const MERGED_LABEL: &str = "Larger symbol: several places, or a big bicycle park";
/// Room around the furthest walk band, so its edge does not touch the frame.
const WALK_FRAME_MARGIN: f64 = 1.04;
/// Scale denominators a fitted sheet snaps to, a tilted view or a sheet area,
/// so the note reads like a real scale. Down to 1:50, for a sheet of one plot.
pub(super) const NICE_SCALES: [u32; 21] = [
    50, 100, 200, 500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7500, 10_000,
    12_500, 15_000, 20_000, 25_000, 50_000,
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
    /// A tilted view, where only lengths running left to right across the
    /// paper are at `scale`: depth shortens with the tilt and heights rise
    /// with it. The scale note says so.
    pub is_foreshortened: bool,
    pub title_name: String,
    /// The ground plane on paper, where pins stand: the plan, the tilted
    /// view at ground height, or the top plate of an explode view.
    pub ground: Projection,
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
    /// A plan zoomed out to a walk: the frame reaches this far from the
    /// centre, in metres, instead of hugging the site.
    pub reach_m: Option<f64>,
    /// Where the neighbourhood shows, when this map shows one.
    pub context: Option<ContextArea>,
    /// The project's highlights lie on this map's ground: a plan, an
    /// axonometric, or every plate of an explode view.
    pub has_highlights: bool,
    /// Their labels are written too; an explode view names them on its top plate only.
    pub has_highlight_labels: bool,
}

pub struct Map {
    pub defs: String,
    pub body: String,
    pub legend: Legend,
}

pub fn plan(input: &RenderInput, layout: &Layout, typesetter: &Typesetter) -> Result<SheetDrawing> {
    let sheet = &input.look.sheet;
    let zone = layout.drawing;
    let mut projection = Projection::new(&input.scene.boundary, sheet.scale, &zone);
    let mut scale = sheet.scale;
    let mut reach_m = None;
    if let Some(walk_reach_m) = walk_reach(input, projection.ground_centre) {
        // Zoom out until the walk fits, and never in past the sheet's own scale.
        let fitting_mm_per_m = zone.width.min(zone.height) / (2.0 * walk_reach_m);
        scale = fitted_scale(sheet.scale, fitting_mm_per_m);
        projection = projection.scaled_to(1000.0 / scale as f64);
        // The frame takes in the whole site too, but stops at the drawing zone,
        // so a site larger than the walk is cut off rather than run into the legend.
        let zone_reach_m = zone.width.min(zone.height) / 2.0 / projection.mm_per_m;
        let site_reach_m = site_reach(input.scene, projection.ground_centre);
        reach_m = Some(walk_reach_m.max(site_reach_m).min(zone_reach_m));
    }
    let spec = MapSpec {
        diagram: input.diagram,
        projection,
        crop_zone: Some(zone),
        scope: String::new(),
        has_volume: false,
        is_plate: false,
        reach_m,
        context: Some(ContextArea {
            zone,
            reach_m: None,
        }),
        has_highlights: true,
        has_highlight_labels: true,
    };
    let map = draw(input, &spec, typesetter)?;

    Ok(SheetDrawing {
        defs: map.defs,
        body: map.body,
        legend: sheet.show_legend.then_some(map.legend),
        mm_per_m: projection.mm_per_m,
        scale,
        north_deg: 0.0,
        is_foreshortened: false,
        title_name: input.diagram.name.clone(),
        ground: projection,
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
        reach_m: None,
        context: Some(ContextArea {
            zone,
            reach_m: None,
        }),
        has_highlights: true,
        has_highlight_labels: true,
    };
    let map = draw(input, &spec, typesetter)?;

    Ok(SheetDrawing {
        defs: map.defs,
        body: map.body,
        legend: sheet.show_legend.then_some(map.legend),
        mm_per_m,
        scale,
        north_deg: projection.north_angle_deg(),
        is_foreshortened: true,
        title_name: input.diagram.name.clone(),
        ground: projection,
    })
}

/// The sheet's own scale when the drawing fits at it, or the nearest round scale that does.
pub(super) fn fitted_scale(sheet_scale: u32, fitting_mm_per_m: f64) -> u32 {
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

/// A row after the classes saying what a larger symbol means.
fn add_merged_row(legend: &mut Legend, ink: &str) {
    let after_classes = legend
        .rows
        .iter()
        .rposition(|row| matches!(row.swatch, Swatch::Symbol { .. }))
        .map_or(0, |index| index + 1);
    legend.rows.insert(
        after_classes,
        Row {
            label: MERGED_LABEL.to_string(),
            swatch: Swatch::Dot {
                fill: None,
                stroke: Some(ink.to_string()),
                opacity: 1.0,
            },
        },
    );
}

/// For a walk diagram, how far its furthest drawn band reaches from `centre`,
/// with a little room. `None` for other diagrams and scenes without a walk.
fn walk_reach(input: &RenderInput, centre: (f64, f64)) -> Option<f64> {
    let minutes = f64::from(input.diagram.walk_minutes?);
    let walk = input.scene.walk.as_ref()?;
    let furthest = walk
        .bands
        .iter()
        .filter(|band| {
            band.attributes
                .walk_min
                .is_some_and(|band_min| band_min <= minutes)
        })
        .flat_map(|band| band.geometry.coords_iter())
        .map(|coord| ((coord.x - centre.0).powi(2) + (coord.y - centre.1).powi(2)).sqrt())
        .fold(0.0, f64::max);

    (furthest > 0.0).then_some(furthest * WALK_FRAME_MARGIN)
}

/// The furthest the site boundary reaches from `centre`, so a zoomed-out frame never cuts the site.
fn site_reach(scene: &Scene, centre: (f64, f64)) -> f64 {
    scene
        .boundary
        .exterior()
        .0
        .iter()
        .map(|coord| ((coord.x - centre.0).powi(2) + (coord.y - centre.1).powi(2)).sqrt())
        .fold(0.0, f64::max)
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
    let frame = match spec.reach_m {
        Some(reach_m) => FrameShape::around(look.sheet.frame, scene, &projection, reach_m, scope),
        None => FrameShape::new(
            look.sheet.frame,
            scene,
            &projection,
            spec.crop_zone.as_ref(),
            scope,
        ),
    };
    let mut layers = style::resolve_layers(diagram, theme, look)?;
    if !projection.is_plan() {
        // A drop shadow is a flat trick; tilted views cast real ones or none.
        for resolved in &mut layers {
            resolved.style.shadow_mm = 0.0;
        }
    }
    if spec.has_volume {
        massing::lighten(&mut layers, look, theme);
    }
    let location = sun::site_location(&scene.area);
    let sun = sun::position_at(location, &look.sun);
    let seen = viewpoint::compute(scene, diagram, look, frame.ground_bounds());
    let mut computed = derived::features(scene, diagram, &look.sun);
    computed.extend(seen.iter().map(|seen| seen.area.clone()));
    let features = cull::within_frame(
        features_by_layer(scene, &computed, diagram),
        frame.ground_bounds(),
        cull::caster_margin(
            tallest_building(scene).max(solid::TREE_HEIGHT_M),
            spec.has_volume || diagram.solar == Some(Solar::Shadow),
            &sun,
            &spec.projection,
        ),
    );
    let canvas = Canvas {
        projection,
        clip_id: frame.clip_id(),
        scope,
    };
    let neighbourhood = Neighbourhood::of(scene, &look.sheet, spec.context, scope);

    let mut defs = String::new();
    frame.write_defs(&mut defs);
    if let Some(neighbourhood) = &neighbourhood {
        neighbourhood.write_defs(&mut defs, &projection);
    }
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
    if let Some(neighbourhood) = &neighbourhood {
        neighbourhood.write_ground(&mut body, theme, &canvas, spec.has_volume)?;
    }
    let is_shadow_diagram = diagram.solar == Some(Solar::Shadow);
    let casters = shadow_casters(&layers, &features);
    let mut drawn = Vec::with_capacity(layers.len());
    let mut has_shadows = false;
    let mut band_rows = Vec::new();
    let mut transit_lines = Vec::new();
    let mut has_merged_points = false;
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
        if resolved.layer == Layer::WalkBand {
            let bands = Bands {
                resolved,
                features: layer_features,
                canvas: &canvas,
                paper: &palette.paper,
                typesetter,
                has_labels: !spec.is_plate,
            };
            band_rows = walk_bands::write(&mut body, &bands);
            // The bands bring their own legend rows, one per band.
            drawn.push(Drawn {
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
                diagram,
                canvas: &canvas,
                paper: &palette.paper,
                typesetter,
                has_labels: !spec.is_plate,
            };
            let drawn_points = points::write(&mut body, &point_layer);
            transit_lines.extend(drawn_points.lines);
            has_merged_points |= drawn_points.has_merged;
            drawn.push(drawn_points.drawn);
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
    let highlight_layer = HighlightLayer {
        highlights: if spec.has_highlights && !look.hide_highlights {
            highlights::shown(input.highlights, scene)
        } else {
            Vec::new()
        },
        scene,
        has_volume: spec.has_volume,
        has_labels: spec.has_highlight_labels,
        canvas: &canvas,
        palette,
        typesetter,
    };
    highlights::write_shapes(&mut body, &highlight_layer);
    if spec.has_volume {
        let solid_layer = |layer: Layer| {
            layers
                .iter()
                .find(|resolved| resolved.layer == layer)
                .map(|resolved| (resolved, layer_features(&features, layer)))
        };
        let neighbours = neighbourhood.as_ref().map(|neighbourhood| Neighbours {
            features: neighbourhood.buildings(),
            roof: context::roof(&massing::theme_fill(theme), theme),
            clip_id: &neighbourhood.clip_id,
            opacity: neighbourhood.opacity_attribute(),
        });
        solid::write(
            &mut body,
            solid_layer(Layer::Building),
            solid_layer(Layer::Tree),
            neighbours.as_ref(),
            diagram,
            theme,
            &canvas,
            &sun,
        );
    }

    highlights::write_over(&mut body, &highlight_layer);
    if let Some(seen) = &seen {
        viewpoint::write_eye(
            &mut body,
            seen,
            &canvas,
            palette,
            typesetter.unit,
            !spec.is_plate,
        );
    }

    let mut legend = legend::build(&layers, &drawn, diagram, &palette.accent);
    // The note names the year of the data, which only the scene knows.
    if legend.note.is_some() {
        legend.note = diagram
            .classify
            .as_ref()
            .and_then(|classify| classify.note_for(scene))
            .map(|note| note.replace(DATE_PLACEHOLDER, &look.sun.date_label()));
    }
    add_lines_to_mode_rows(&mut legend, &transit_lines);
    if has_merged_points {
        add_merged_row(&mut legend, &palette.ink);
    }
    let site_row = legend.rows.len() - 1;
    for (offset, row) in band_rows.into_iter().enumerate() {
        legend.rows.insert(site_row + offset, row);
    }
    legend
        .rows
        .extend(highlights::legend_rows(&highlight_layer));
    if is_shadow_diagram {
        legend.rows.push(shadow_row(&sun, look, theme));
    }
    if let Some(seen) = &seen {
        viewpoint::describe(&mut legend, seen, palette);
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

/// The scene's features by layer, with the computed ones. A walk diagram
/// takes its places, paths and bands from the walk instead, which reaches
/// past the site, keeping only what lies within its minutes.
fn features_by_layer<'a>(
    scene: &'a Scene,
    computed: &'a [Feature],
    diagram: &Diagram,
) -> BTreeMap<Layer, Vec<&'a Feature>> {
    let mut grouped: BTreeMap<Layer, Vec<&Feature>> = BTreeMap::new();
    let walk = diagram.walk_minutes.zip(scene.walk.as_ref());
    for feature in scene.features.iter().chain(computed) {
        if walk.is_some() && legend::is_point_layer(feature.layer) {
            continue;
        }
        grouped.entry(feature.layer).or_default().push(feature);
    }
    if let Some((minutes, walk)) = walk {
        let is_within = |feature: &&Feature| {
            feature
                .attributes
                .walk_min
                .is_some_and(|walk_min| walk_min <= f64::from(minutes))
        };
        let within = walk.places.iter().chain(&walk.bands).filter(is_within);
        for feature in within.chain(&walk.paths) {
            grouped.entry(feature.layer).or_default().push(feature);
        }
    }

    grouped
}

/// "Tram" -> "Tram 2 · 13 · 17": each transit mode lists the lines that call
/// at a drawn stop. A mode whose lines only call at stops of a busier mode,
/// such as buses at a tram stop, gets a row of its own without a symbol.
fn add_lines_to_mode_rows(legend: &mut Legend, lines: &[String]) {
    let mut after_symbols = legend
        .rows
        .iter()
        .rposition(|row| matches!(row.swatch, Swatch::Symbol { .. }))
        .map_or(0, |index| index + 1);
    for (_, mode) in osm::TRANSIT_GROUPS {
        let prefix = format!("{mode} ");
        let mut of_mode: Vec<String> = lines
            .iter()
            .filter(|line| line.starts_with(&prefix))
            .cloned()
            .collect();
        if of_mode.is_empty() {
            continue;
        }
        of_mode.sort_by_key(|line| osm::line_order(&line[prefix.len()..]));
        let label = format!("{mode} {}", points::line_numbers(&of_mode));
        match legend.rows.iter_mut().find(|row| row.label == mode) {
            Some(row) => row.label = label,
            None => {
                let swatch = Swatch::Dot {
                    fill: None,
                    stroke: None,
                    opacity: 0.0,
                };
                legend.rows.insert(after_symbols, Row { label, swatch });
                after_symbols += 1;
            }
        }
    }
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
