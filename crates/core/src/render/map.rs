//! One diagram drawn into one frame. A plan is one map, an axonometric one map
//! with its buildings standing up, and an explode view one flat map per plate.

use std::collections::BTreeMap;
use std::fmt::Write;

use super::context::{self, ContextArea, Neighbourhood};
use super::cull;
use super::derived;
use super::drawing::{pattern_id, Canvas};
use super::frame::FrameShape;
use super::geometry::Projection;
use super::highlights::{self, HighlightLayer};
use super::layout::Rect;
use super::legend::{self, Legend};
use super::map_layers::{self, FlatLayers, LayerPass};
use super::map_legend::{add_lines_to_mode_rows, add_merged_row};
use super::map_shadows::{
    shadow_casters, shadow_row, write_shadows, AMBIENT_SHADOW_OPACITY, DIAGRAM_SHADOW_OPACITY,
};
use super::massing;
use super::patterns;
use super::proposal;
use super::solid::{self, Neighbours};
use super::style::{self, ResolvedLayer};
use super::sunpath::{self, SunPath};
use super::svg::layer_attributes;
use super::typeset::Typesetter;
use super::viewpoint::{self, Seen};
use super::RenderInput;
use crate::diagram::{Diagram, Solar, DATE_PLACEHOLDER, WIND_PLACEHOLDER};
use crate::model::Model;
use crate::scene::{Feature, Layer, Scene};
use crate::sun::{self, SunPosition};
use crate::theme::Theme;
use crate::wind;
use crate::Result;

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

pub(super) fn tallest_building(scene: &Scene) -> f64 {
    scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .map(|feature| solid::building_height(feature).0)
        .fold(0.0, f64::max)
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
    let layers = resolved_layers(input, spec)?;
    let location = sun::site_location(&scene.area);
    let sun = sun::position_at(location, &look.sun);
    let seen = viewpoint::compute(scene, diagram, look, input.pins, frame.ground_bounds());
    let mut computed = derived::features(scene, diagram, look);
    computed.extend(seen.iter().map(|seen| seen.area.clone()));
    let features = cull::within_frame(
        features_by_layer(scene, &computed, diagram, input.model),
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
    let defs = map_defs(&frame, neighbourhood.as_ref(), &layers, &projection, scope);

    let mut body = String::new();
    frame.write_ground(&mut body, &palette.ground, &format!("{scope}ground"));
    if let Some(neighbourhood) = &neighbourhood {
        neighbourhood.write_ground(&mut body, theme, &canvas, spec.has_volume)?;
    }
    let model = proposal::placed(input, &layers);
    let casters = shadow_casters(&layers, &features, model.as_ref());
    let pass = LayerPass {
        layers: &layers,
        features: &features,
        casters: &casters,
        sun: &sun,
        canvas: &canvas,
        theme,
        diagram,
        typesetter,
        has_volume: spec.has_volume,
        has_labels: !spec.is_plate,
    };
    let flat = map_layers::write(&mut body, &pass);

    if spec.has_volume {
        let opacity = if diagram.solar == Some(Solar::Shadow) {
            DIAGRAM_SHADOW_OPACITY
        } else {
            AMBIENT_SHADOW_OPACITY
        };
        write_shadows(&mut body, &casters, &sun, &canvas, theme, opacity);
    }
    if let Some(model) = model.as_ref().filter(|_| !spec.has_volume) {
        proposal::write_plan(&mut body, model, &canvas, theme);
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
            opacity: neighbourhood.opacity,
        });
        solid::write(
            &mut body,
            solid_layer(Layer::Building),
            solid_layer(Layer::Tree),
            neighbours.as_ref(),
            model.as_ref(),
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

    let mut legend = map_legend(
        input,
        diagram,
        &layers,
        flat,
        &highlight_layer,
        seen.as_ref(),
        &sun,
    );
    if model.is_some() {
        legend.rows.push(proposal::legend_row(theme));
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

/// The diagram's layers in the look's theme, as this map draws them.
fn resolved_layers(input: &RenderInput, spec: &MapSpec) -> Result<Vec<ResolvedLayer>> {
    let mut layers = style::resolve_layers(spec.diagram, input.theme, input.look)?;
    if !spec.projection.is_plan() {
        // A drop shadow is a flat trick; tilted views cast real ones or none.
        for resolved in &mut layers {
            resolved.style.shadow_mm = 0.0;
        }
    }
    if spec.has_volume {
        massing::lighten(&mut layers, input.look, input.theme);
    }

    Ok(layers)
}

/// The frame's clip, the neighbourhood's and every layer's fill pattern.
fn map_defs(
    frame: &FrameShape,
    neighbourhood: Option<&Neighbourhood>,
    layers: &[ResolvedLayer],
    projection: &Projection,
    scope: &str,
) -> String {
    let mut defs = String::new();
    frame.write_defs(&mut defs);
    if let Some(neighbourhood) = neighbourhood {
        neighbourhood.write_defs(&mut defs, projection);
    }
    for resolved in layers {
        if let Some(pattern) = &resolved.style.pattern {
            defs.push_str(&patterns::pattern_def(
                &pattern_id(scope, resolved.layer),
                pattern,
                resolved.style.fill.as_deref(),
            ));
        }
    }

    defs
}

/// The legend of the drawn layers, with the rows only this map knows to add:
/// the walk bands, transit lines, highlights, shadow time and the view.
fn map_legend(
    input: &RenderInput,
    diagram: &Diagram,
    layers: &[ResolvedLayer],
    flat: FlatLayers,
    highlight_layer: &HighlightLayer,
    seen: Option<&Seen>,
    sun: &SunPosition,
) -> Legend {
    let scene = input.scene;
    let look = input.look;
    let palette = &input.theme.palette;
    let mut legend = legend::build(layers, &flat.drawn, diagram, &palette.accent);
    // The note names the year of the data, which only the scene knows.
    if legend.note.is_some() {
        legend.note = diagram
            .classify
            .as_ref()
            .and_then(|classify| classify.note_for(scene))
            .map(|note| note.replace(DATE_PLACEHOLDER, &look.sun.date_label()))
            .map(|note| note.replace(WIND_PLACEHOLDER, &wind::described(scene, look.wind_season)));
    }
    add_lines_to_mode_rows(&mut legend, &flat.transit_lines);
    if flat.has_merged_points {
        add_merged_row(&mut legend, &palette.ink);
    }
    let site_row = legend.rows.len() - 1;
    for (offset, row) in flat.band_rows.into_iter().enumerate() {
        legend.rows.insert(site_row + offset, row);
    }
    legend.rows.extend(highlights::legend_rows(highlight_layer));
    if diagram.solar == Some(Solar::Shadow) {
        legend.rows.push(shadow_row(sun, look, input.theme));
    }
    if let Some(seen) = seen {
        viewpoint::describe(&mut legend, seen, palette);
    }

    legend
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

pub(super) fn layer_features<'a, 'b>(
    features: &'b BTreeMap<Layer, Vec<&'a Feature>>,
    layer: Layer,
) -> &'b [&'a Feature] {
    features.get(&layer).map(Vec::as_slice).unwrap_or(&[])
}

/// The scene's features by layer, with the computed ones, less the buildings
/// the model replaces. A walk diagram takes its places, paths and bands from
/// the walk instead, which reaches past the site, keeping only what lies within its minutes.
fn features_by_layer<'a>(
    scene: &'a Scene,
    computed: &'a [Feature],
    diagram: &Diagram,
    model: Option<&Model>,
) -> BTreeMap<Layer, Vec<&'a Feature>> {
    let mut grouped: BTreeMap<Layer, Vec<&Feature>> = BTreeMap::new();
    let walk = diagram.walk_minutes.zip(scene.walk.as_ref());
    for feature in scene.features.iter().chain(computed) {
        if walk.is_some() && legend::is_point_layer(feature.layer) {
            continue;
        }
        if model.is_some_and(|model| model.replaces(feature)) {
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
