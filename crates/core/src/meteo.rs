//! The weather terms the climate analysis and the computed layers share:
//! compass sectors, the calm limit, the meteorological seasons, and what a
//! typical year that falls short is called. The wind roses and the wind
//! diagram read the same sectors, so the diagram's prevailing wind is the
//! longest wedge of the matching rose.

use crate::Error;

/// Below this wind speed at 10 m the air is calm: Beaufort 0 and 1 (WMO).
pub const CALM_M_S: f64 = 1.6;

pub const SECTORS: usize = 16;
pub const SECTOR_NAMES: [&str; SECTORS] = [
    "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW",
    "NNW",
];
/// The width of one sector, and the step from one sector's middle to the next.
pub const SECTOR_DEG: f64 = 360.0 / SECTORS as f64;

/// Meteorological seasons: (name, months, first and last month).
pub const SEASONS: [(&str, &str, [u32; 3]); 4] = [
    ("Winter", "Dec–Feb", [12, 1, 2]),
    ("Spring", "Mar–May", [3, 4, 5]),
    ("Summer", "Jun–Aug", [6, 7, 8]),
    ("Autumn", "Sep–Nov", [9, 10, 11]),
];

/// 225° -> 10, the index of "SW".
pub fn sector_of(degrees: f64) -> usize {
    ((degrees.rem_euclid(360.0) / SECTOR_DEG).round() as usize) % SECTORS
}

/// 225° -> "SW".
pub fn sector_name(degrees: f64) -> &'static str {
    SECTOR_NAMES[sector_of(degrees)]
}

/// The error for a PVGIS typical year with a month or an hour missing, the
/// same whichever part of the year is being read.
pub fn incomplete_year(detail: String) -> Error {
    Error::Data {
        service: "PVGIS".to_string(),
        detail: format!("expected a full typical year, got {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_round_to_the_nearest_of_sixteen() {
        assert_eq!(sector_name(0.0), "N");
        assert_eq!(sector_name(350.0), "N");
        assert_eq!(sector_name(225.0), "SW");
        assert_eq!(sector_name(-90.0), "W");
    }
}
