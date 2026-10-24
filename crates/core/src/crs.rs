//! Conversion between WGS84 (what the map shows) and RD New (what Dutch data uses).
//!
//! Owned by the data layer. Accurate to well under a metre across the Netherlands,
//! which is enough to crop a site; the data itself arrives in RD New already.

use crate::area::LonLat;

/// WGS84 longitude/latitude to RD New x/y in metres.
pub fn wgs84_to_rd(point: LonLat) -> (f64, f64) {
    let _ = point;
    todo!("data layer")
}

/// RD New x/y in metres to WGS84 longitude/latitude.
pub fn rd_to_wgs84(x: f64, y: f64) -> LonLat {
    let _ = (x, y);
    todo!("data layer")
}
