//! The site the user drew, in longitude and latitude, exactly as they drew it.

use serde::{Deserialize, Serialize};

/// Largest area we fetch in one go. A 4 km² site at 0.5 m elevation resolution
/// is already about 64 MB of raster, and diagrams stop reading well beyond it.
pub const MAX_AREA_M2: f64 = 4_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Area {
    Circle { center: LonLat, radius_m: f64 },
    Rectangle { south_west: LonLat, north_east: LonLat },
    /// Outer ring only, not closed: the first point is not repeated at the end.
    Polygon { points: Vec<LonLat> },
}
