//! Several diagrams of one site as a stack of tilted plates, pulled apart so
//! each one shows, with dotted guides down the sides and each plate named at
//! the end of a leader line.
//!
//! Every plate is the full flat map of its diagram in the look's theme, drawn
//! through the same tilted projection. The stack is scaled to fit the drawing
//! zone, so the sheet's own scale is a ceiling here rather than a promise.

use std::fmt::Write;

use super::context::ContextArea;
use super::frame::ground_outline;
use super::geometry::{decimal, ground_centre, mm, Projection};
use super::layout::Layout;
use super::map::{draw, Bounds, MapSpec, SheetDrawing};
use super::svg::{escape, layer_attributes};
use super::typeset::{Kind, Typesetter};
use super::RenderInput;
use crate::catalog::{builtin_diagrams, find_plate, plate_id};
use crate::diagram::Diagram;
use crate::{Error, Result};

/// The step from one plate to the next, as a share of a plate's height on paper.
const EXPLODED_STEP: f64 = 0.62;
const STACKED_STEP: f64 = 0.24;
const TITLE: &str = "Exploded view";

// Sizes at A3, in millimetres; multiplied by the layout unit.
const LEADER: f64 = 14.0;
const LABEL_GAP: f64 = 2.0;
const DOT_RADIUS: f64 = 0.55;
const LINE_MM: f64 = 0.15;
const GUIDE_DASH: &str = "0.2 1";
const GUIDE_OPACITY: f64 = 0.6;
/// How far the bottom plate's neighbourhood reaches, as a multiple of the plate's own reach.
const NEIGHBOURHOOD_REACH: f64 = 1.6;

pub fn render(
    input: &RenderInput,
    layout: &Layout,
    typesetter: &Typesetter,
) -> Result<SheetDrawing> {
    let look = input.look;
    let view = &look.view;
    let own_plate = plate_id(&look.diagram_id, look.variant.as_deref());
    let plates = plate_diagrams(&view.plates, &own_plate)?;
    let unit = typesetter.unit;
    let ink = &input.theme.palette.ink;

    // The plate outline at 1 mm per metre, centred on the paper origin.
    let centre = ground_centre(&input.scene.boundary);
    let unit_view = Projection::tilted(centre, 1.0, view.rotation_deg, view.tilt_deg).flat();
    let ground = ground_outline(look.sheet.frame, input.scene, centre);
    let outline: Vec<(f64, f64)> = ground
        .iter()
        .map(|&(x, y)| unit_view.to_paper(x, y))
        .collect();
    let plate_reach_m = ground
        .iter()
        .map(|&(x, y)| ((x - centre.0).powi(2) + (y - centre.1).powi(2)).sqrt())
        .fold(0.0, f64::max);
    // Only the bottom plate stands in the neighbourhood; the plates above would only cover it.
    let bottom_context = ContextArea {
        zone: layout.drawing,
        reach_m: Some(plate_reach_m * NEIGHBOURHOOD_REACH),
    };
    let mut bounds = Bounds::default();
    for &point in &outline {
        bounds.add(point);
    }
    // The leftmost and rightmost points, where the guides run and the leaders start.
    let left = outline
        .iter()
        .copied()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or_default();
    let right = outline
        .iter()
        .copied()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or_default();

    let label_width = plates
        .iter()
        .map(|diagram| typesetter.width(&diagram.name, Kind::Label))
        .fold(0.0, f64::max)
        + (LEADER + LABEL_GAP) * unit;
    let step_share = if view.stacking {
        STACKED_STEP
    } else {
        EXPLODED_STEP
    };
    let count = plates.len() as f64;
    let zone = layout.drawing;
    let mm_per_m = (1000.0 / look.sheet.scale as f64)
        .min((zone.width - label_width) / bounds.width())
        .min(zone.height / (bounds.height() * (1.0 + (count - 1.0) * step_share)));
    let step = bounds.height() * mm_per_m * step_share;
    let stack_width = bounds.width() * mm_per_m + label_width;
    let stack_height = bounds.height() * mm_per_m + (count - 1.0) * step;
    let stack_left = zone.centre().0 - stack_width / 2.0;
    let stack_top = zone.centre().1 - stack_height / 2.0;
    // Where the ground centre of the top plate lands.
    let first_centre = (
        stack_left - bounds.min.0 * mm_per_m,
        stack_top - bounds.min.1 * mm_per_m,
    );
    let plate_centre = |index: usize| (first_centre.0, first_centre.1 + step * index as f64);
    let at = |index: usize, (x, y): (f64, f64)| {
        let (centre_x, centre_y) = plate_centre(index);
        (centre_x + x * mm_per_m, centre_y + y * mm_per_m)
    };

    let mut defs = String::new();
    let mut body = String::new();
    // Bottom plate first, so each plate above covers the one below.
    for (index, diagram) in plates.iter().enumerate().rev() {
        let spec = MapSpec {
            diagram,
            projection: unit_view.scaled_to(mm_per_m).moved_to(plate_centre(index)),
            crop_zone: None,
            scope: format!("plate-{}-", index + 1),
            has_volume: false,
            is_plate: true,
            reach_m: None,
            context: (index == plates.len() - 1).then_some(bottom_context),
        };
        let map = draw(input, &spec, typesetter)?;
        defs.push_str(&map.defs);
        let _ = writeln!(
            body,
            "<g {}>{}</g>",
            layer_attributes(
                &format!("plate-{}", index + 1),
                &format!("Plate {}: {}", index + 1, diagram.name)
            ),
            map.body
        );
    }

    let last = plates.len() - 1;
    let _ = write!(
        body,
        r#"<g {} fill="none" stroke="{ink}" stroke-width="{}">"#,
        layer_attributes("plate-labels", "Plate labels"),
        mm(LINE_MM)
    );
    for side in [left, right] {
        let (top_x, top_y) = at(0, side);
        let (_, bottom_y) = at(last, side);
        let _ = write!(
            body,
            r#"<path d="M{} {}V{}" stroke-dasharray="{GUIDE_DASH}" stroke-opacity="{}"/>"#,
            mm(top_x),
            mm(top_y),
            mm(bottom_y),
            decimal(GUIDE_OPACITY)
        );
    }
    for (index, diagram) in plates.iter().enumerate() {
        let (x, y) = at(index, right);
        let end = x + LEADER * unit;
        let _ = write!(
            body,
            r#"<path d="M{} {y}H{}"/><circle cx="{}" cy="{y}" r="{}" fill="{ink}" stroke="none"/>"#,
            mm(x),
            mm(end),
            mm(x),
            mm(DOT_RADIUS * unit),
            y = mm(y)
        );
        let baseline = y + typesetter.cap_height(Kind::Label) / 2.0;
        typesetter.write(
            &mut body,
            end + LABEL_GAP * unit,
            baseline,
            &diagram.name,
            Kind::Label,
            None,
        );
    }
    body.push_str("</g>\n");

    Ok(SheetDrawing {
        defs,
        body: wrap_in_font(&body, typesetter),
        legend: None,
        mm_per_m,
        scale: look.sheet.scale,
        north_deg: unit_view.north_angle_deg(),
        has_scale_bar: false,
        title_name: TITLE.to_string(),
    })
}

/// The labels sit outside the furniture group, so they need the theme's family too.
fn wrap_in_font(body: &str, typesetter: &Typesetter) -> String {
    format!(
        r#"<g font-family="{}">{body}</g>"#,
        escape(typesetter.face.css_stack())
    )
}

/// The plates' diagrams, top first. With none chosen, the look's own diagram is the one plate.
fn plate_diagrams(ids: &[String], fallback: &str) -> Result<Vec<Diagram>> {
    let catalog = builtin_diagrams();
    let wanted: Vec<&str> = if ids.is_empty() {
        vec![fallback]
    } else {
        ids.iter().map(String::as_str).collect()
    };

    wanted
        .into_iter()
        .map(|id| {
            find_plate(&catalog, id).ok_or_else(|| {
                Error::Invalid(format!(
                    "explode plate {id:?} is not a diagram; use an id from list_diagrams"
                ))
            })
        })
        .collect()
}
