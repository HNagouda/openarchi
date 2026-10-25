//! Several diagrams of one site as a stack of tilted plates, pulled apart so
//! each one shows, with dotted guides down the sides and each plate named at
//! the end of a leader line.
//!
//! Every plate is the full flat map of its diagram in the look's theme, drawn
//! through the same tilted projection. The stack is scaled down to the nearest
//! round scale that fits the drawing zone, never up past the sheet's own.
//!
//! The scale bar measures the plates. A tilted plate keeps its scale only for
//! lengths running left to right across the paper: depth into the plate is
//! shortened by the sine of the tilt. So the bar, like the axonometric one,
//! holds for lengths along a plate's horizontal axis, and its note says
//! "left to right". All plates share one scale, so one bar serves the stack.
//!
//! Each plate is a group carrying its plate id, its place in the stack, the
//! paper y of its centre and its left and right edge in `data-*` attributes,
//! so the app can pick a plate up and drop it elsewhere in the stack without
//! repeating this layout.

use std::fmt::Write;

use super::context::ContextArea;
use super::frame::ground_outline;
use super::geometry::{decimal, furthest_from, ground_centre, mm, Projection};
use super::layout::Layout;
use super::map::{draw, MapSpec, SheetDrawing};
use super::map_sheet::{fitted_scale, Bounds};
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
    let ids = plate_ids(&view.plates, &own_plate);
    let plates = plate_diagrams(&ids)?;

    // The plate outline at 1 mm per metre, centred on the paper origin.
    let centre = ground_centre(&input.scene.boundary);
    let unit_view = Projection::tilted(centre, 1.0, view.rotation_deg, view.tilt_deg).flat();
    let ground = ground_outline(look.sheet.frame, input.scene, centre);
    let outline: Vec<(f64, f64)> = ground
        .iter()
        .map(|&(x, y)| unit_view.to_paper(x, y))
        .collect();
    let plate_reach_m = furthest_from(ground.iter().copied(), centre);
    // Only the bottom plate stands in the neighbourhood; the plates above would only cover it.
    let bottom_context = ContextArea {
        zone: layout.drawing,
        reach_m: Some(plate_reach_m * NEIGHBOURHOOD_REACH),
    };
    let stack = Stack::new(input, layout, typesetter, &plates, &outline);

    let mut defs = String::new();
    let mut body = String::new();
    // Bottom plate first, so each plate above covers the one below.
    for (index, diagram) in plates.iter().enumerate().rev() {
        let spec = MapSpec {
            diagram,
            projection: unit_view
                .scaled_to(stack.mm_per_m)
                .moved_to(stack.plate_centre(index)),
            crop_zone: None,
            scope: format!("plate-{}-", index + 1),
            has_volume: false,
            is_plate: true,
            reach_m: None,
            context: (index == plates.len() - 1).then_some(bottom_context),
            // Every plate shows the highlights on its ground, and the top one names them.
            has_highlights: true,
            has_highlight_labels: index == 0,
        };
        let map = draw(input, &spec, typesetter)?;
        defs.push_str(&map.defs);
        let (_, centre_y) = stack.plate_centre(index);
        let _ = writeln!(
            body,
            r#"<g {} data-plate="{}" data-index="{index}" data-centre-y="{}" data-left="{}" data-right="{}">{}</g>"#,
            layer_attributes(
                &format!("plate-{}", index + 1),
                &format!("Plate {}: {}", index + 1, diagram.name)
            ),
            escape(&ids[index]),
            mm(centre_y),
            mm(stack.at(index, stack.left).0),
            mm(stack.at(index, stack.right).0),
            map.body
        );
    }
    write_labels(
        &mut body,
        &plates,
        &stack,
        typesetter,
        &input.theme.palette.ink,
    );

    Ok(SheetDrawing {
        defs,
        body: wrap_in_font(&body, typesetter),
        legend: None,
        mm_per_m: stack.mm_per_m,
        scale: stack.scale,
        north_deg: unit_view.north_angle_deg(),
        is_foreshortened: true,
        title_name: TITLE.to_string(),
        ground: unit_view
            .scaled_to(stack.mm_per_m)
            .moved_to(stack.plate_centre(0)),
    })
}

/// Where the plates go on paper: the stack fitted into the drawing zone with
/// room for the labels on the right, centred, at a round scale.
struct Stack {
    /// Where the ground centre of the top plate lands.
    first_centre: (f64, f64),
    /// From one plate's centre to the next one down.
    step: f64,
    mm_per_m: f64,
    scale: u32,
    /// The plate outline's leftmost and rightmost points at 1 mm per metre,
    /// where the guides run and the leaders start.
    left: (f64, f64),
    right: (f64, f64),
}

impl Stack {
    fn new(
        input: &RenderInput,
        layout: &Layout,
        typesetter: &Typesetter,
        plates: &[Diagram],
        outline: &[(f64, f64)],
    ) -> Stack {
        let view = &input.look.view;
        let mut bounds = Bounds::default();
        for &point in outline {
            bounds.add(point);
        }
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
            + (LEADER + LABEL_GAP) * typesetter.unit;
        let step_share = if view.stacking {
            STACKED_STEP
        } else {
            EXPLODED_STEP
        };
        let count = plates.len() as f64;
        let zone = layout.drawing;
        let fitting_mm_per_m = ((zone.width - label_width) / bounds.width())
            .min(zone.height / (bounds.height() * (1.0 + (count - 1.0) * step_share)));
        let scale = fitted_scale(input.look.sheet.scale, fitting_mm_per_m);
        let mm_per_m = 1000.0 / scale as f64;
        let step = bounds.height() * mm_per_m * step_share;
        let stack_width = bounds.width() * mm_per_m + label_width;
        let stack_height = bounds.height() * mm_per_m + (count - 1.0) * step;
        let stack_left = zone.centre().0 - stack_width / 2.0;
        let stack_top = zone.centre().1 - stack_height / 2.0;

        Stack {
            first_centre: (
                stack_left - bounds.min.0 * mm_per_m,
                stack_top - bounds.min.1 * mm_per_m,
            ),
            step,
            mm_per_m,
            scale,
            left,
            right,
        }
    }

    fn plate_centre(&self, index: usize) -> (f64, f64) {
        (
            self.first_centre.0,
            self.first_centre.1 + self.step * index as f64,
        )
    }

    /// A point of the 1 mm per metre outline, on plate `index`.
    fn at(&self, index: usize, (x, y): (f64, f64)) -> (f64, f64) {
        let (centre_x, centre_y) = self.plate_centre(index);
        (centre_x + x * self.mm_per_m, centre_y + y * self.mm_per_m)
    }
}

/// The dotted guides down both sides of the stack, and each plate's name at
/// the end of a leader from its right edge.
fn write_labels(
    body: &mut String,
    plates: &[Diagram],
    stack: &Stack,
    typesetter: &Typesetter,
    ink: &str,
) {
    let unit = typesetter.unit;
    let last = plates.len() - 1;
    let _ = write!(
        body,
        r#"<g {} fill="none" stroke="{ink}" stroke-width="{}">"#,
        layer_attributes("plate-labels", "Plate labels"),
        mm(LINE_MM)
    );
    for side in [stack.left, stack.right] {
        let (top_x, top_y) = stack.at(0, side);
        let (_, bottom_y) = stack.at(last, side);
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
        let (x, y) = stack.at(index, stack.right);
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
            body,
            end + LABEL_GAP * unit,
            baseline,
            &diagram.name,
            Kind::Label,
            None,
        );
    }
    body.push_str("</g>\n");
}

/// The labels sit outside the furniture group, so they need the theme's family too.
fn wrap_in_font(body: &str, typesetter: &Typesetter) -> String {
    format!(
        r#"<g font-family="{}">{body}</g>"#,
        escape(typesetter.face.css_stack())
    )
}

/// The plate ids, top first. With none chosen, the look's own diagram is the one plate.
fn plate_ids(chosen: &[String], fallback: &str) -> Vec<String> {
    if chosen.is_empty() {
        vec![fallback.to_string()]
    } else {
        chosen.to_vec()
    }
}

/// The diagram each plate id draws, in the same order.
fn plate_diagrams(ids: &[String]) -> Result<Vec<Diagram>> {
    let catalog = builtin_diagrams();

    ids.iter()
        .map(|id| {
            find_plate(&catalog, id).ok_or_else(|| {
                Error::Invalid(format!(
                    "explode plate {id:?} is not a diagram; use an id from list_diagrams"
                ))
            })
        })
        .collect()
}
