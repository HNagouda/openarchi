//! The view diagram's part of a map: the ground in view from the sheet's
//! viewpoint, the eye drawn where it stands, and the legend that says how
//! the view was found and how much of the site it takes in.
//!
//! The view is worked out over the map's own frame, so it reaches as far as
//! the drawing does. It lies on the ground like any measured layer, so a plan,
//! an axonometric and an explode plate all draw it. The eye is one group
//! carrying its paper position in data attributes, as pins do, so the app can
//! pick it up and drag it on a plan or a tilted view; a plate draws it plain.

use std::fmt::Write;

use geo::{BoundingRect, Rect};

use super::drawing::Canvas;
use super::geometry::mm;
use super::legend::{Legend, Row, Swatch, SITE_LABEL};
use super::svg::layer_attributes;
use crate::diagram::Diagram;
use crate::pin::Pin;
use crate::project::Look;
use crate::scene::{Feature, Layer, Scene};
use crate::section;
use crate::theme::Palette;
use crate::viewshed::{self, EYE_HEIGHT_M};

const LEGEND_TITLE: &str = "Ground in view";
const PERCENT: f64 = 100.0;
// Sizes at A3, in millimetres; multiplied by the layout unit.
/// Half the eye's width and half its height, an almond twice as wide as tall.
const EYE_HALF_WIDTH: f64 = 2.4;
const EYE_HALF_HEIGHT: f64 = 1.25;
const PUPIL_RADIUS: f64 = 0.7;
const EYE_LINE_MM: f64 = 0.35;
/// The paper edge that keeps the eye apart from a dark figure under it.
const HALO_MM: f64 = 0.9;

/// The view from the sheet's viewpoint, ready to draw.
pub struct Seen {
    /// The open ground in view, on the viewshed layer.
    pub area: Feature,
    /// Where the eye stands, in the scene's frame.
    pub eye: (f64, f64),
    share: Option<f64>,
    is_on_roof: bool,
    has_terrain: bool,
}

/// The view, for a diagram that draws the viewshed layer. `extent` is the
/// ground the map's frame shows, in the scene's frame. `pins` are the
/// project's, so an eye standing on a pin follows it.
pub fn compute(
    scene: &Scene,
    diagram: &Diagram,
    look: &Look,
    pins: &[Pin],
    extent: Option<Rect<f64>>,
) -> Option<Seen> {
    if !diagram.layers.contains(&Layer::Viewshed) {
        return None;
    }
    let (origin_x, origin_y) = scene.origin;
    let eye = look
        .eye_among(pins)
        .map(|(x, y)| (x - origin_x, y - origin_y));
    let extent = extent
        .or_else(|| section::reach(scene))
        .or_else(|| scene.boundary.bounding_rect())?;
    let view = viewshed::viewshed(scene, eye, extent);

    Some(Seen {
        area: view.area(),
        eye: view.eye,
        share: view.share_of_site(),
        is_on_roof: view.is_on_roof,
        has_terrain: view.has_terrain(),
    })
}

/// The eye, an almond with a pupil in the palette's accent on a paper edge.
pub fn write_eye(
    out: &mut String,
    seen: &Seen,
    canvas: &Canvas,
    palette: &Palette,
    unit: f64,
    has_handle: bool,
) {
    let (x, y) = canvas.projection.to_paper(seen.eye.0, seen.eye.1);
    let (half_width, half_height) = (EYE_HALF_WIDTH * unit, EYE_HALF_HEIGHT * unit);
    let almond = format!(
        "M{} {}Q{} {} {} {}Q{} {} {} {}Z",
        mm(x - half_width),
        mm(y),
        mm(x),
        mm(y - 2.0 * half_height),
        mm(x + half_width),
        mm(y),
        mm(x),
        mm(y + 2.0 * half_height),
        mm(x - half_width),
        mm(y)
    );
    // Only the eye the app can drag carries the plain id and its paper position.
    let attributes = if has_handle {
        format!(
            r#"{} data-x="{}" data-y="{}""#,
            layer_attributes("view-point", "Viewpoint"),
            mm(x),
            mm(y)
        )
    } else {
        layer_attributes(&format!("{}view-point", canvas.scope), "Viewpoint")
    };

    let _ = write!(
        out,
        r#"<g {attributes}><path d="{almond}" fill="{}" stroke="{}" stroke-width="{}" stroke-linejoin="round"/><path d="{almond}" fill="{}" stroke="{}" stroke-width="{}" stroke-linejoin="round"/><circle cx="{}" cy="{}" r="{}" fill="{}"/></g>"#,
        palette.paper,
        palette.paper,
        mm(HALO_MM * unit),
        palette.paper,
        palette.accent,
        mm(EYE_LINE_MM * unit),
        mm(x),
        mm(y),
        mm(PUPIL_RADIUS * unit),
        palette.accent,
    );
}

/// Names the legend after the view, notes the method and the share of the
/// site in view, and adds the eye above the site boundary.
pub fn describe(legend: &mut Legend, seen: &Seen, palette: &Palette) {
    legend.title = LEGEND_TITLE.to_string();
    legend.note = Some(note(seen));
    let stands_on = if seen.is_on_roof { "roof" } else { "ground" };
    let eye_row = Row {
        label: format!("Viewpoint, eye {EYE_HEIGHT_M} m above the {stands_on}"),
        swatch: Swatch::Dot {
            fill: Some(palette.accent.clone()),
            stroke: Some(palette.paper.clone()),
            opacity: 1.0,
        },
    };
    let site_row = legend
        .rows
        .iter()
        .position(|row| row.label == SITE_LABEL)
        .unwrap_or(legend.rows.len());
    legend.rows.insert(site_row, eye_row);
}

/// "Line of sight past building heights and the terrain (R2 viewshed,
/// Franklin & Ray 1994). 62% of the site's open ground is in view."
fn note(seen: &Seen) -> String {
    let surface = if seen.has_terrain {
        "building heights and the terrain"
    } else {
        "building heights on flat ground"
    };
    let method = format!("Line of sight past {surface} (R2 viewshed, Franklin & Ray 1994).");
    match seen.share {
        Some(share) => format!(
            "{method} {:.0}% of the site's open ground is in view.",
            share * PERCENT
        ),
        None => method,
    }
}

#[cfg(test)]
mod tests {
    use geo::{Geometry, MultiPolygon};

    use super::*;
    use crate::scene::Attributes;

    fn seen(share: Option<f64>, has_terrain: bool) -> Seen {
        Seen {
            area: Feature {
                layer: Layer::Viewshed,
                geometry: Geometry::MultiPolygon(MultiPolygon::new(Vec::new())),
                attributes: Attributes::default(),
            },
            eye: (0.0, 0.0),
            share,
            is_on_roof: false,
            has_terrain,
        }
    }

    #[test]
    fn the_note_names_the_method_and_the_share_in_view() {
        assert_eq!(
            note(&seen(Some(0.624), true)),
            "Line of sight past building heights and the terrain (R2 viewshed, Franklin & Ray 1994). 62% of the site's open ground is in view."
        );
        assert!(note(&seen(None, false))
            .ends_with("on flat ground (R2 viewshed, Franklin & Ray 1994)."));
    }
}
