//! RIVM's national maps of noise and air quality, from the Atlas Leefomgeving WCS.
//!
//! The service also offers "actueel" aliases that follow the newest year, but
//! the legend has to say which year it shows, so the dated coverages are named
//! here and their years with them. The capabilities document gives each alias
//! the dated id it points at as its first keyword, which is how to check for a
//! newer year.

use crate::scene::{Credit, Layer};
use crate::sources::wcs::Coverage;

const SERVICE: &str = "RIVM";
const WCS_URL: &str = "https://data.rivm.nl/geo/wcs";
const AXES: (&str, &str) = ("X", "Y");

// #! TODO[noise, air] : Move to the next dated coverages when the "actueel" aliases point past 2022 (noise) and 2024 (air)
/// The year the noise maps model, which the legends quote.
pub const NOISE_YEAR: i32 = 2022;
/// The year the air quality maps describe.
pub const AIR_YEAR: i32 = 2024;

/// Road traffic noise, Lden in dB, 10 m cells, stored as whole decibels.
pub const ROAD_NOISE: Coverage = rivm("alo__rivm_20250101_Geluid_lden_wegverkeer_2022");
/// Every source together (road, rail, air, industry, wind turbines), Lden in dB.
pub const ALL_NOISE: Coverage = rivm("alo__rivm_20250801_Geluid_lden_allebronnen_2022");
/// Annual means in µg/m³, 25 m cells on a grid offset from round numbers.
pub const NO2: Coverage = rivm("alo__rivm_nsl_20260401_gm_NO22024");
pub const PM10: Coverage = rivm("alo__rivm_nsl_20260401_gm_PM102024");
pub const PM25: Coverage = rivm("alo__rivm_nsl_20260401_gm_PM252024");

const fn rivm(id: &'static str) -> Coverage {
    Coverage {
        service: SERVICE,
        url: WCS_URL,
        id,
        axes: AXES,
    }
}

// data.overheid.nl lists both the noise and the air quality maps as public
// domain, with RIVM as the owner. The WCS capabilities say nothing about it.
pub fn credit() -> Credit {
    Credit {
        name: "RIVM Atlas Leefomgeving, noise and air quality maps".to_string(),
        license: "Public Domain Mark 1.0".to_string(),
        url: "https://www.atlasleefomgeving.nl".to_string(),
        layers: vec![
            Layer::RoadNoise,
            Layer::AllNoise,
            Layer::No2,
            Layer::Pm10,
            Layer::Pm25,
        ],
    }
}
