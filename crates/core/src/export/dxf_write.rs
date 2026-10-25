//! The R12 pieces every DXF the app writes is made of: group codes, polylines,
//! circles, text, and a drawing that collects its layers as entities land on
//! them, so the layer table always matches what the file holds.
//!
//! DXF R12 (AC1009) is the oldest flavour every CAD and GIS program still
//! opens, and the only one that needs no object handles. It predates Unicode,
//! so text outside ASCII is written as `\U+XXXX`, the escape AutoCAD reads
//! back as the character. ezdxf 1.4 keeps the escape as typed.

use std::fmt::Write;

use geo::{Geometry, LineString, Polygon};

/// Points become circles of this radius on the ground: a tree, a stop, a shop.
const POINT_RADIUS_M: f64 = 1.0;
const CONTINUOUS: &str = "CONTINUOUS";
/// R12 layer names may be at most this long.
const MAX_LAYER_NAME: usize = 31;
/// Stands in for a character R12 cannot spell even as `\U+XXXX`.
const UNSPELLABLE: char = '?';

/// A named line pattern, lengths in drawing units: a dash is positive, a gap
/// negative and a dot zero, as the LTYPE table wants them.
pub struct Linetype {
    pub name: &'static str,
    pub description: &'static str,
    pub pattern: Vec<f64>,
}

struct CadLayer {
    name: String,
    colour: u8,
    linetype: &'static str,
}

/// Entities written so far, and the layers they sit on in order of first use.
pub struct Drawing {
    pub entities: String,
    layers: Vec<CadLayer>,
}

impl Drawing {
    pub fn new() -> Drawing {
        Drawing {
            entities: String::with_capacity(1 << 16),
            layers: Vec::new(),
        }
    }

    /// Declares a layer in a solid line; the first colour given for a name stays.
    pub fn layer(&mut self, name: &str, colour: u8) {
        self.layer_with_line(name, colour, CONTINUOUS);
    }

    pub fn layer_with_line(&mut self, name: &str, colour: u8, linetype: &'static str) {
        if self.layers.iter().any(|layer| layer.name == name) {
            return;
        }
        self.layers.push(CadLayer {
            name: name.to_string(),
            colour,
            linetype,
        });
    }

    /// The whole file: header with `extent` (min x, min y, max x, max y), the
    /// line and layer tables, then the entities.
    pub fn finish(self, extent: (f64, f64, f64, f64), linetypes: &[Linetype]) -> String {
        let mut out = String::with_capacity(self.entities.len() + 8192);
        section(&mut out, "HEADER");
        group(&mut out, 9, "$ACADVER");
        group(&mut out, 1, "AC1009");
        group(&mut out, 9, "$EXTMIN");
        coordinate(&mut out, extent.0, extent.1, 0.0);
        group(&mut out, 9, "$EXTMAX");
        coordinate(&mut out, extent.2, extent.3, 0.0);
        group(&mut out, 0, "ENDSEC");

        section(&mut out, "TABLES");
        write_linetypes(&mut out, linetypes);
        group(&mut out, 0, "TABLE");
        group(&mut out, 2, "LAYER");
        group(&mut out, 70, &self.layers.len().to_string());
        for layer in &self.layers {
            group(&mut out, 0, "LAYER");
            group(&mut out, 2, &layer.name);
            group(&mut out, 70, "0");
            group(&mut out, 62, &layer.colour.to_string());
            group(&mut out, 6, layer.linetype);
        }
        group(&mut out, 0, "ENDTAB");
        group(&mut out, 0, "ENDSEC");

        section(&mut out, "ENTITIES");
        out.push_str(&self.entities);
        group(&mut out, 0, "ENDSEC");
        group(&mut out, 0, "EOF");

        out
    }
}

fn write_linetypes(out: &mut String, linetypes: &[Linetype]) {
    group(out, 0, "TABLE");
    group(out, 2, "LTYPE");
    group(out, 70, &(linetypes.len() + 1).to_string());
    group(out, 0, "LTYPE");
    group(out, 2, CONTINUOUS);
    group(out, 70, "0");
    group(out, 3, "Solid line");
    group(out, 72, "65");
    group(out, 73, "0");
    group(out, 40, "0.0");
    for linetype in linetypes {
        let total: f64 = linetype.pattern.iter().map(|length| length.abs()).sum();
        group(out, 0, "LTYPE");
        group(out, 2, linetype.name);
        group(out, 70, "0");
        group(out, 3, linetype.description);
        group(out, 72, "65");
        group(out, 73, &linetype.pattern.len().to_string());
        group(out, 40, &format!("{total:.3}"));
        for length in &linetype.pattern {
            group(out, 49, &format!("{length:.3}"));
        }
    }
    group(out, 0, "ENDTAB");
}

/// "6–10 m" -> "6-10-M": upper case, anything but letters and digits a dash,
/// cut to the 31 characters R12 allows in a layer name.
pub fn cad_name(text: &str) -> String {
    let mut name = String::new();
    for letter in text.to_uppercase().chars() {
        if letter.is_ascii_alphanumeric() {
            name.push(letter);
        } else if !name.ends_with('-') {
            name.push('-');
        }
    }

    let name: String = name
        .trim_matches('-')
        .chars()
        .take(MAX_LAYER_NAME)
        .collect();
    name.trim_end_matches('-').to_string()
}

pub fn write_geometry(
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
        Geometry::Point(point) => write_circle(
            out,
            point.x() + shift.0,
            point.y() + shift.1,
            POINT_RADIUS_M,
            layer,
        ),
        Geometry::MultiPoint(multi) => {
            for point in &multi.0 {
                write_circle(
                    out,
                    point.x() + shift.0,
                    point.y() + shift.1,
                    POINT_RADIUS_M,
                    layer,
                );
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
pub fn write_polygon(
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

pub fn write_polyline(
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

/// An open or closed polyline through plain (x, y) points, already in drawing units.
pub fn write_points(out: &mut String, points: &[(f64, f64)], layer: &str, is_closed: bool) {
    let line: LineString<f64> = points.iter().copied().collect();
    write_polyline(out, &line, layer, 0.0, is_closed, (0.0, 0.0));
}

pub fn write_circle(out: &mut String, x: f64, y: f64, radius: f64, layer: &str) {
    group(out, 0, "CIRCLE");
    group(out, 8, layer);
    coordinate(out, x, y, 0.0);
    group(out, 40, &format!("{radius:.3}"));
}

pub fn write_point(out: &mut String, x: f64, y: f64, layer: &str) {
    group(out, 0, "POINT");
    group(out, 8, layer);
    coordinate(out, x, y, 0.0);
}

/// One line of text, its baseline starting at (x, y), or centred on x when `is_centred`.
pub fn write_text(
    out: &mut String,
    (x, y): (f64, f64),
    height: f64,
    text: &str,
    layer: &str,
    is_centred: bool,
) {
    group(out, 0, "TEXT");
    group(out, 8, layer);
    coordinate(out, x, y, 0.0);
    group(out, 40, &format!("{height:.3}"));
    group(out, 1, &spelled(text));
    if is_centred {
        // Horizontal justification 1 is centre; R12 then places the text by its second point.
        group(out, 72, "1");
        let _ = write!(out, " 11\n{x:.3}\n 21\n{y:.3}\n 31\n0.000\n");
    }
}

/// "Plot · 412 m²" -> "Plot \U+00B7 412 m\U+00B2", on one line.
fn spelled(text: &str) -> String {
    let mut spelled = String::with_capacity(text.len());
    for letter in text.chars() {
        if letter == '\n' || letter == '\r' {
            spelled.push(' ');
        } else if letter.is_ascii() {
            spelled.push(letter);
        } else if (letter as u32) <= 0xFFFF {
            let _ = write!(spelled, "\\U+{:04X}", letter as u32);
        } else {
            spelled.push(UNSPELLABLE);
        }
    }

    spelled
}

pub fn section(out: &mut String, name: &str) {
    group(out, 0, "SECTION");
    group(out, 2, name);
}

/// One group code and its value, the two-line unit DXF is made of.
pub fn group(out: &mut String, code: u16, value: &str) {
    let _ = write!(out, "{code:>3}\n{value}\n");
}

/// Millimetre precision: metres to three decimals.
pub fn coordinate(out: &mut String, x: f64, y: f64, z: f64) {
    let _ = write!(out, " 10\n{x:.3}\n 20\n{y:.3}\n 30\n{z:.3}\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_labels_become_r12_layer_names() {
        assert_eq!(cad_name("6–10 m"), "6-10-M");
        assert_eq!(cad_name("30 m and taller"), "30-M-AND-TALLER");
        assert_eq!(cad_name(&"x".repeat(40)).len(), MAX_LAYER_NAME);
    }

    #[test]
    fn text_outside_ascii_is_spelled_as_unicode_escapes() {
        assert_eq!(spelled("Plot · 412 m²"), "Plot \\U+00B7 412 m\\U+00B2");
        assert_eq!(spelled("A\u{2032}"), "A\\U+2032");
        assert_eq!(spelled("two\nlines"), "two lines");
    }

    #[test]
    fn a_layer_is_declared_once_with_its_first_colour() {
        let mut drawing = Drawing::new();
        drawing.layer("OA-SITE", 1);
        drawing.layer("OA-SITE", 3);
        let file = drawing.finish((0.0, 0.0, 1.0, 1.0), &[]);

        assert_eq!(file.matches("\nOA-SITE\n").count(), 1);
        assert!(file.contains("OA-SITE\n 70\n0\n 62\n1\n"));
    }
}
