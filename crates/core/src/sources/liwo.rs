//! LIWO, the national flood information service of Rijkswaterstaat: the
//! deepest water any flood scenario reaches, through its GeoServer WCS.
//!
//! LIWO publishes this map in five probability classes, Kaart1 to Kaart5. Its
//! method report (Deltares, "Technische uitgangspunten en methode LIWO
//! afgeleide kaarten", 21 November 2025, table 2-1) numbers them: 1 large
//! chance (more than 1 in 30 a year), 2 medium (1 in 30 to 1 in 300), 3 small
//! (1 in 300 to 1 in 3,000), 4 very small (1 in 3,000 to 1 in 30,000) and 5
//! extremely small (less than 1 in 30,000). Each map includes every scenario
//! of the classes above it, so Kaart5 is the most complete picture of where
//! water can reach, and the one a site analysis shows. The WMS layers carry no
//! abstract, so the report is the only source for this numbering.

use crate::scene::{Credit, Layer};
use crate::sources::wcs::Coverage;

/// Maximum water depth in metres, 5 m cells. Dry cells are nodata, not zero.
pub const FLOOD_DEPTH: Coverage = Coverage {
    service: "LIWO",
    url: "https://basisinformatie-overstromingen.nl/geoserver/LIWO_Basis/ows",
    id: "LIWO_Basis__MaximaleWaterdiepteNederland_Kaart5",
    axes: ("X", "Y"),
};

/// LIWO release 2025.1.2 is the one the method report describes.
pub const FLOOD_YEAR: i32 = 2025;

// #! NOTE[flood, licence] : The LIWO capabilities say "Fees none, AccessConstraints none" and name no licence; ask liwo@rws.nl which one applies
pub fn credit() -> Credit {
    Credit {
        name: "Rijkswaterstaat LIWO, maximum flood depth".to_string(),
        license: "no access constraints".to_string(),
        url: "https://basisinformatie-overstromingen.nl".to_string(),
        layers: vec![Layer::Flood],
    }
}
