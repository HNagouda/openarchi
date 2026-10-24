//! The scene as a DXF drawing: one CAD layer per scene layer, in real RD New metres.
//! The neighbourhood around the site goes on layers of its own, "OA-CONTEXT-BUILDING"
//! and so on, in one grey, so it can be switched off or locked as a whole.
//!
//! Written as DXF R12 (AC1009), the oldest flavour every CAD and GIS program
//! still opens, and the only one that needs no object handles. No styling
//! beyond a layer colour: CAD users bring their own.

use std::collections::BTreeSet;
use std::fmt::Write;

use geo::{Geometry, LineString, Polygon};

use crate::render::layer_slug;
use crate::scene::{Layer, Scene};
use crate::sources::bands::BANDED;

/// Trees become circles of this radius on the ground, a symbol rather than a crown.
const TREE_RADIUS_M: f64 = 1.0;
const SITE_LAYER: &str = "OA-SITE";
/// AutoCAD colour index for the site outline: red.
const SITE_COLOUR: u8 = 1;
/// AutoCAD colour index for every neighbourhood layer: grey, the usual colour of context.
const CONTEXT_COLOUR: u8 = 8;

/// The scene, with its neighbourhood when `has_context`.
pub fn write(scene: &Scene, has_context: bool) -> String {
    let context = if has_context {
        scene.context.as_slice()
    } else {
        &[]
    };
    let mut out = String::with_capacity(4096 + (scene.features.len() + context.len()) * 400);
    let shift = scene.origin;
    let context_layers: BTreeSet<Layer> = context.iter().map(|feature| feature.layer).collect();
    write_header(&mut out, scene);
    write_layer_table(&mut out, &context_layers);

    section(&mut out, "ENTITIES");
    for feature in &scene.features {
        let layer = cad_layer(feature.layer);
        // A band sits at the height of its lower edge, as a contour sits at its
        // level, so a CAD user can read 55 dB or 0.5 m off the outline.
        let elevation = if feature.layer == Layer::Contour {
            feature.attributes.ground_m.unwrap_or(0.0)
        } else if BANDED.contains(&feature.layer) {
            feature.attributes.value.unwrap_or(0.0)
        } else {
            0.0
        };
        write_geometry(&mut out, &feature.geometry, &layer, elevation, shift);
    }
    for feature in context {
        let layer = context_cad_layer(feature.layer);
        write_geometry(&mut out, &feature.geometry, &layer, 0.0, shift);
    }
    write_polygon(&mut out, &scene.boundary, SITE_LAYER, 0.0, shift);
    group(&mut out, 0, "ENDSEC");
    group(&mut out, 0, "EOF");

    out
}

/// Layer::RailArea -> "OA-RAIL-AREA".
fn cad_layer(layer: Layer) -> String {
    format!("OA-{}", layer_slug(layer).to_uppercase())
}

/// Layer::Building -> "OA-CONTEXT-BUILDING".
fn context_cad_layer(layer: Layer) -> String {
    format!("OA-CONTEXT-{}", layer_slug(layer).to_uppercase())
}

/// AutoCAD colour index per layer, so the drawing reads before anyone restyles it.
fn colour_index(layer: Layer) -> u8 {
    match layer {
        Layer::Building => 7,
        Layer::Road | Layer::Parking | Layer::Bridge => 8,
        Layer::Footpath | Layer::Paved | Layer::Unpaved | Layer::Yard => 9,
        Layer::Cycleway => 11,
        Layer::RailArea | Layer::RailTrack => 252,
        Layer::Water | Layer::WaterBank => 5,
        Layer::Green | Layer::Verge | Layer::Tree => 3,
        Layer::Parcel => 6,
        Layer::Contour => 30,
        Layer::RoadNoise | Layer::AllNoise => 40,
        Layer::No2 | Layer::Pm10 | Layer::Pm25 => 50,
        Layer::Flood => 150,
        Layer::Slope => 32,
        Layer::Population => 200,
        Layer::Business | Layer::Facility | Layer::TransitStop => 1,
        Layer::WalkPath | Layer::WalkBand => 40,
    }
}

fn write_header(out: &mut String, scene: &Scene) {
    let (min_x, min_y, max_x, max_y) = extent(scene);
    section(out, "HEADER");
    group(out, 9, "$ACADVER");
    group(out, 1, "AC1009");
    group(out, 9, "$EXTMIN");
    coordinate(out, min_x, min_y, 0.0);
    group(out, 9, "$EXTMAX");
    coordinate(out, max_x, max_y, 0.0);
    group(out, 0, "ENDSEC");
}

fn write_layer_table(out: &mut String, context_layers: &BTreeSet<Layer>) {
    section(out, "TABLES");
    group(out, 0, "TABLE");
    group(out, 2, "LTYPE");
    group(out, 70, "1");
    group(out, 0, "LTYPE");
    group(out, 2, "CONTINUOUS");
    group(out, 70, "0");
    group(out, 3, "Solid line");
    group(out, 72, "65");
    group(out, 73, "0");
    group(out, 40, "0.0");
    group(out, 0, "ENDTAB");

    group(out, 0, "TABLE");
    group(out, 2, "LAYER");
    group(
        out,
        70,
        &(Layer::ALL.len() + context_layers.len() + 1).to_string(),
    );
    let layers = Layer::ALL
        .iter()
        .map(|&layer| (cad_layer(layer), colour_index(layer)));
    let context = context_layers
        .iter()
        .map(|&layer| (context_cad_layer(layer), CONTEXT_COLOUR));
    let site = [(SITE_LAYER.to_string(), SITE_COLOUR)];
    for (name, colour) in layers.chain(context).chain(site) {
        group(out, 0, "LAYER");
        group(out, 2, &name);
        group(out, 70, "0");
        group(out, 62, &colour.to_string());
        group(out, 6, "CONTINUOUS");
    }
    group(out, 0, "ENDTAB");
    group(out, 0, "ENDSEC");
}

fn write_geometry(
    out: &mut String,
    geometry: &Geometry<f64>,
    layer: &str,
    elevation: f64,
    shift: (f64, f64),
) {
    match geometry {
        Geometry::Polygon(polygon) => write_polygon(out, polygon, layer, elevation, shift),
        Geometry::MultiPolygon(multi) => {
            for polygon in &multi.0 {
                write_polygon(out, polygon, layer, elevation, shift);
            }
        }
        Geometry::Rect(rect) => write_polygon(out, &rect.to_polygon(), layer, elevation, shift),
        Geometry::Triangle(triangle) => {
            write_polygon(out, &triangle.to_polygon(), layer, elevation, shift)
        }
        Geometry::LineString(line) => write_polyline(out, line, layer, elevation, false, shift),
        Geometry::MultiLineString(multi) => {
            for line in &multi.0 {
                write_polyline(out, line, layer, elevation, false, shift);
            }
        }
        Geometry::Line(line) => write_polyline(
            out,
            &LineString::from(vec![line.start, line.end]),
            layer,
            elevation,
            false,
            shift,
        ),
        Geometry::Point(point) => write_circle(out, point.x(), point.y(), layer, shift),
        Geometry::MultiPoint(multi) => {
            for point in &multi.0 {
                write_circle(out, point.x(), point.y(), layer, shift);
            }
        }
        Geometry::GeometryCollection(collection) => {
            for member in &collection.0 {
                write_geometry(out, member, layer, elevation, shift);
            }
        }
    }
}

/// Outer ring and holes as separate closed polylines on the same layer.
fn write_polygon(
    out: &mut String,
    polygon: &Polygon<f64>,
    layer: &str,
    elevation: f64,
    shift: (f64, f64),
) {
    write_polyline(out, polygon.exterior(), layer, elevation, true, shift);
    for interior in polygon.interiors() {
        write_polyline(out, interior, layer, elevation, true, shift);
    }
}

fn write_polyline(
    out: &mut String,
    line: &LineString<f64>,
    layer: &str,
    elevation: f64,
    is_closed: bool,
    shift: (f64, f64),
) {
    let mut points = line.0.as_slice();
    // A closed polyline repeats no point; the flag closes it.
    if is_closed && points.len() > 1 && points.first() == points.last() {
        points = &points[..points.len() - 1];
    }
    if points.len() < 2 {
        return;
    }

    group(out, 0, "POLYLINE");
    group(out, 8, layer);
    group(out, 66, "1");
    coordinate(out, 0.0, 0.0, elevation);
    group(out, 70, if is_closed { "1" } else { "0" });
    for point in points {
        group(out, 0, "VERTEX");
        group(out, 8, layer);
        coordinate(out, point.x + shift.0, point.y + shift.1, elevation);
    }
    group(out, 0, "SEQEND");
    group(out, 8, layer);
}

fn write_circle(out: &mut String, x: f64, y: f64, layer: &str, shift: (f64, f64)) {
    group(out, 0, "CIRCLE");
    group(out, 8, layer);
    coordinate(out, x + shift.0, y + shift.1, 0.0);
    group(out, 40, &format!("{TREE_RADIUS_M:.3}"));
}

/// Real-world extent of the boundary, for the header.
fn extent(scene: &Scene) -> (f64, f64, f64, f64) {
    let (shift_x, shift_y) = scene.origin;
    let mut bounds = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for point in &scene.boundary.exterior().0 {
        bounds = (
            bounds.0.min(point.x),
            bounds.1.min(point.y),
            bounds.2.max(point.x),
            bounds.3.max(point.y),
        );
    }
    if bounds.0 > bounds.2 {
        return (shift_x, shift_y, shift_x, shift_y);
    }

    (
        bounds.0 + shift_x,
        bounds.1 + shift_y,
        bounds.2 + shift_x,
        bounds.3 + shift_y,
    )
}

fn section(out: &mut String, name: &str) {
    group(out, 0, "SECTION");
    group(out, 2, name);
}

/// One group code and its value, the two-line unit DXF is made of.
fn group(out: &mut String, code: u16, value: &str) {
    let _ = write!(out, "{code:>3}\n{value}\n");
}

/// Millimetre precision: RD New metres to three decimals.
fn coordinate(out: &mut String, x: f64, y: f64, z: f64) {
    let _ = write!(out, " 10\n{x:.3}\n 20\n{y:.3}\n 30\n{z:.3}\n");
}
