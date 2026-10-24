//! One laser bed as DXF: a CUT and an ENGRAVE layer, in millimetres.
//!
//! Written with the same R12 pieces as the site DXF. Laser software that
//! reads DXF assigns its cut and score settings per layer or per colour, so
//! both are set: CUT is red and ENGRAVE blue, as in the SVG.

use geo::{Coord, LineString};

use super::Part;
use crate::export::dxf::{coordinate, group, section, write_polyline};

const CUT_LAYER: &str = "CUT";
const ENGRAVE_LAYER: &str = "ENGRAVE";
/// AutoCAD colour index 1: red.
const CUT_COLOUR: u8 = 1;
/// AutoCAD colour index 5: blue.
const ENGRAVE_COLOUR: u8 = 5;

// #! NOTE[laser, dxf] : R12 has no unit setting, so the laser software must be told to import the DXF as millimetres; is that a problem at the workshop?
/// `sheet` holds (index into `parts`, x, y) for each piece on this bed.
pub fn write(parts: &[Part], sheet: &[(usize, f64, f64)], bed: (f64, f64)) -> String {
    let mut out = String::new();
    section(&mut out, "HEADER");
    group(&mut out, 9, "$ACADVER");
    group(&mut out, 1, "AC1009");
    group(&mut out, 9, "$EXTMIN");
    coordinate(&mut out, 0.0, 0.0, 0.0);
    group(&mut out, 9, "$EXTMAX");
    coordinate(&mut out, bed.0, bed.1, 0.0);
    group(&mut out, 0, "ENDSEC");

    section(&mut out, "TABLES");
    group(&mut out, 0, "TABLE");
    group(&mut out, 2, "LAYER");
    group(&mut out, 70, "2");
    for (name, colour) in [(CUT_LAYER, CUT_COLOUR), (ENGRAVE_LAYER, ENGRAVE_COLOUR)] {
        group(&mut out, 0, "LAYER");
        group(&mut out, 2, name);
        group(&mut out, 70, "0");
        group(&mut out, 62, &colour.to_string());
        group(&mut out, 6, "CONTINUOUS");
    }
    group(&mut out, 0, "ENDTAB");
    group(&mut out, 0, "ENDSEC");

    // Score lines first, then cuts, as in the SVG.
    section(&mut out, "ENTITIES");
    for &(index, x, y) in sheet {
        for line in &parts[index].engrave {
            let placed = on_bed(line, (x, y), bed.1);
            write_polyline(&mut out, &placed, ENGRAVE_LAYER, 0.0, false, (0.0, 0.0));
        }
    }
    for &(index, x, y) in sheet {
        for ring in &parts[index].cut {
            let placed = on_bed(ring, (x, y), bed.1);
            write_polyline(&mut out, &placed, CUT_LAYER, 0.0, true, (0.0, 0.0));
        }
    }
    group(&mut out, 0, "ENDSEC");
    group(&mut out, 0, "EOF");

    out
}

/// Piece millimetres (y down) to bed millimetres with y up, as CAD expects.
fn on_bed(points: &[(f64, f64)], offset: (f64, f64), bed_height: f64) -> LineString<f64> {
    points
        .iter()
        .map(|&(x, y)| Coord {
            x: x + offset.0,
            y: bed_height - (y + offset.1),
        })
        .collect()
}
