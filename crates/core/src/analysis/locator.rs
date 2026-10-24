//! The locator map: the country with a dot for the site, and beside it the
//! municipality with the site drawn in it.

use std::fmt::Write;

use geo::{BoundingRect, Coord, MultiPolygon, Polygon, Rect};

use super::figure::{figure, line, text, Figure, Paint, HAIRLINE_MM, INSET_MM, SMALL_MM, WIDTH_MM};
use super::location::{country, Location};

const MAP_HEIGHT_MM: f64 = 44.0;
const PANEL_GAP_MM: f64 = 6.0;
const OUTLINE_MM: f64 = 0.25;
const SITE_DOT_MM: f64 = 1.1;
/// A site smaller than this on the municipality map gets a ring around it,
/// so a 300 m circle in a 20 km municipality can still be found.
const MIN_SITE_MM: f64 = 2.0;
const SITE_RING_MM: f64 = 2.4;
const CAPTION_GAP_MM: f64 = 4.0;

/// Where a map panel sits on the figure and how RD metres map onto it.
struct Panel {
    left: f64,
    top: f64,
    scale: f64,
    min_x: f64,
    max_y: f64,
}

impl Panel {
    /// Fits `bounds` into a box `width` by `height` at (`left`, `top`), centred.
    fn fit(bounds: Rect<f64>, left: f64, top: f64, width: f64, height: f64) -> Panel {
        let scale = (width / bounds.width()).min(height / bounds.height());
        Panel {
            left: left + (width - bounds.width() * scale) / 2.0,
            top: top + (height - bounds.height() * scale) / 2.0,
            scale,
            min_x: bounds.min().x,
            max_y: bounds.max().y,
        }
    }

    fn point(&self, coord: Coord<f64>) -> (f64, f64) {
        (
            self.left + (coord.x - self.min_x) * self.scale,
            self.top + (self.max_y - coord.y) * self.scale,
        )
    }

    fn path(&self, polygons: &MultiPolygon<f64>) -> String {
        let mut path = String::new();
        for polygon in polygons {
            for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
                for (index, coord) in ring.coords().enumerate() {
                    let (x, y) = self.point(*coord);
                    let command = if index == 0 { 'M' } else { 'L' };
                    let _ = write!(path, "{command}{x:.2} {y:.2}");
                }
                path.push('Z');
            }
        }

        path
    }
}

pub fn locator(location: &Location, paint: &Paint) -> Figure {
    let land = paint.visible(&paint.ground, &paint.muted);
    let country = country();
    let mut out = String::new();
    let Some(country_bounds) = country.bounding_rect() else {
        return figure("locator", MAP_HEIGHT_MM, out, paint);
    };

    // The country takes the width its shape needs at the map height; the municipality gets the rest.
    let country_width = MAP_HEIGHT_MM * country_bounds.width() / country_bounds.height();
    let top = INSET_MM;
    let left_panel = Panel::fit(country_bounds, INSET_MM, top, country_width, MAP_HEIGHT_MM);
    shape(
        &mut out,
        &left_panel.path(country),
        &land,
        &paint.ink,
        OUTLINE_MM * 0.6,
    );
    if let Some(outline) = &location.municipality_outline {
        shape(
            &mut out,
            &left_panel.path(outline),
            &paint.secondary,
            "none",
            0.0,
        );
    }
    let (dot_x, dot_y) = left_panel.point(Coord {
        x: location.rd.0,
        y: location.rd.1,
    });
    dot(&mut out, (dot_x, dot_y), paint);

    let right_left = INSET_MM + country_width + PANEL_GAP_MM;
    let right_width = WIDTH_MM - INSET_MM - right_left;
    let site = MultiPolygon::new(vec![location.site.clone()]);
    let local_bounds = location
        .municipality_outline
        .as_ref()
        .and_then(BoundingRect::bounding_rect)
        .or_else(|| site.bounding_rect());
    if let Some(bounds) = local_bounds {
        let right_panel = Panel::fit(bounds, right_left, top, right_width, MAP_HEIGHT_MM);
        if let Some(outline) = &location.municipality_outline {
            shape(
                &mut out,
                &right_panel.path(outline),
                &land,
                &paint.ink,
                OUTLINE_MM,
            );
        }
        draw_site(&mut out, &right_panel, &location.site, paint);
        line(
            &mut out,
            (dot_x, dot_y),
            (right_left - 1.0, dot_y.clamp(top, top + MAP_HEIGHT_MM)),
            &paint.secondary,
            HAIRLINE_MM,
        );
    }

    let caption_y = top + MAP_HEIGHT_MM + CAPTION_GAP_MM;
    text(
        &mut out,
        INSET_MM + country_width / 2.0,
        caption_y,
        SMALL_MM,
        400,
        &paint.secondary,
        "middle",
        &paint.label("Netherlands"),
    );
    let local_name = location
        .municipality
        .clone()
        .unwrap_or_else(|| "Site".to_string());
    text(
        &mut out,
        right_left + right_width / 2.0,
        caption_y,
        SMALL_MM,
        400,
        &paint.secondary,
        "middle",
        &paint.label(&local_name),
    );

    figure("locator", caption_y + INSET_MM, out, paint)
}

fn draw_site(out: &mut String, panel: &Panel, site: &Polygon<f64>, paint: &Paint) {
    let site_shape = MultiPolygon::new(vec![site.clone()]);
    shape(out, &panel.path(&site_shape), &paint.accent, "none", 0.0);
    let Some(bounds) = site.bounding_rect() else {
        return;
    };
    if bounds.width() * panel.scale < MIN_SITE_MM {
        let (x, y) = panel.point(bounds.center());
        let _ = writeln!(
            out,
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="{SITE_RING_MM}" fill="none" stroke="{}" stroke-width="{OUTLINE_MM}"/>"#,
            paint.accent
        );
    }
}

fn dot(out: &mut String, (x, y): (f64, f64), paint: &Paint) {
    let _ = writeln!(
        out,
        r#"<circle cx="{x:.2}" cy="{y:.2}" r="{SITE_DOT_MM}" fill="{}" stroke="{}" stroke-width="{OUTLINE_MM}"/>"#,
        paint.accent, paint.paper
    );
}

fn shape(out: &mut String, path: &str, fill: &str, stroke: &str, width: f64) {
    let _ = writeln!(
        out,
        r#"<path d="{path}" fill="{fill}" fill-rule="evenodd" stroke="{stroke}" stroke-width="{width}" stroke-linejoin="round"/>"#
    );
}
