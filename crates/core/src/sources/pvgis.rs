//! PVGIS typical meteorological year (TMY) from the EU Joint Research Centre:
//! 8,760 hourly rows of temperature, humidity, sun and wind for one point.
//!
//! A TMY strings together the most typical calendar month of 2005 to 2023,
//! so it describes ordinary weather, not extremes. The radiation comes from
//! satellites (SARAH-3) and the rest from ERA5 reanalysis, a grid of about
//! 30 km, so neighbouring sites get the same climate. That is why the cache
//! keeps one year per `CELL_M` square rather than per site.

use serde::Deserialize;

use crate::area::LonLat;
use crate::crs::{rd_to_wgs84, wgs84_to_rd};
use crate::scene::Credit;
use crate::sources::grid::Tile;
use crate::sources::http;
use crate::{Error, Result};

const SERVICE: &str = "PVGIS";
const TMY_URL: &str = "https://re.jrc.ec.europa.eu/api/v5_3/tmy";

/// One TMY per square of this size. ERA5 cells are about 30 km and PVGIS
/// interpolates between them, so within 5 km the year barely changes.
pub const CELL_M: f64 = 5_000.0;
/// The cache folder, named with the cell size so a change of cell starts afresh.
pub const CACHE_DATASET: &str = "pvgis-tmy-5km";

/// Coordinates sent to PVGIS, in decimal places. 0.001 degree is about 100 m.
const COORDINATE_DECIMALS: i32 = 3;

pub fn credit() -> Credit {
    Credit {
        name: "PVGIS typical meteorological year, European Commission JRC".to_string(),
        license: "Free reuse with acknowledgement (Commission Decision 2011/833/EU)".to_string(),
        url: "https://joint-research-centre.ec.europa.eu/photovoltaic-geographical-information-system-pvgis_en".to_string(),
        layers: Vec::new(),
    }
}

/// One hour of the typical year. Times are UTC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hour {
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    /// Air temperature 2 m above ground, °C.
    pub temperature_c: f64,
    /// Relative humidity, %.
    pub humidity_pct: f64,
    /// Global irradiance on a horizontal plane (GHI), W/m², which over one hour is Wh/m².
    pub global_w_m2: f64,
    /// Direct irradiance on a plane facing the sun (DNI), W/m².
    pub direct_normal_w_m2: f64,
    /// Diffuse irradiance on a horizontal plane (DHI), W/m².
    pub diffuse_w_m2: f64,
    /// Long-wave (infrared) radiation from the sky onto a horizontal plane, W/m².
    pub infrared_w_m2: f64,
    /// Wind speed 10 m above ground, m/s.
    pub wind_m_s: f64,
    /// Where the wind comes from, degrees clockwise from north.
    pub wind_from_deg: f64,
}

/// The parsed year and where PVGIS says it is for.
#[derive(Debug, Clone)]
pub struct Tmy {
    pub location: LonLat,
    pub elevation_m: f64,
    /// The years the typical months were picked from: 2005 and 2023.
    pub first_year: i32,
    pub last_year: i32,
    /// Hours to add to a row's time to get the moment its sun values belong to.
    pub irradiance_offset_hours: f64,
    pub hours: Vec<Hour>,
}

/// The cache cell that holds the year for `point` (RD New).
pub fn cell_of(point: (f64, f64)) -> Tile {
    Tile {
        column: (point.0 / CELL_M).floor() as i64,
        row: (point.1 / CELL_M).floor() as i64,
    }
}

/// The raw PVGIS answer for the centre of `cell`, so every site in the cell
/// gets the same year from the cache.
pub fn fetch(cell: Tile) -> Result<Vec<u8>> {
    let centre = rd_to_wgs84(
        (cell.column as f64 + 0.5) * CELL_M,
        (cell.row as f64 + 0.5) * CELL_M,
    );
    let lat = rounded(centre.lat);
    let lon = rounded(centre.lon);
    http::get(
        SERVICE,
        TMY_URL,
        &[
            ("lat", lat.as_str()),
            ("lon", lon.as_str()),
            ("outputformat", "json"),
        ],
    )
}

/// The cell of a site given in longitude and latitude.
pub fn cell_of_location(location: LonLat) -> Tile {
    cell_of(wgs84_to_rd(location))
}

pub fn parse(bytes: &[u8]) -> Result<Tmy> {
    let answer: Answer = serde_json::from_slice(bytes).map_err(|error| Error::Data {
        service: SERVICE.to_string(),
        detail: error.to_string(),
    })?;
    let hours = answer
        .outputs
        .tmy_hourly
        .iter()
        .map(to_hour)
        .collect::<Result<Vec<Hour>>>()?;
    let location = answer.inputs.location;
    let meteo = answer.inputs.meteo_data;

    Ok(Tmy {
        location: LonLat {
            lon: location.longitude,
            lat: location.latitude,
        },
        elevation_m: location.elevation,
        first_year: meteo.year_min,
        last_year: meteo.year_max,
        irradiance_offset_hours: location.irradiance_time_offset.unwrap_or(0.0),
        hours,
    })
}

fn rounded(degrees: f64) -> String {
    format!("{degrees:.*}", COORDINATE_DECIMALS as usize)
}

#[derive(Deserialize)]
struct Answer {
    inputs: Inputs,
    outputs: Outputs,
}

#[derive(Deserialize)]
struct Inputs {
    location: Location,
    meteo_data: MeteoData,
}

#[derive(Deserialize)]
struct Location {
    latitude: f64,
    longitude: f64,
    elevation: f64,
    irradiance_time_offset: Option<f64>,
}

#[derive(Deserialize)]
struct MeteoData {
    year_min: i32,
    year_max: i32,
}

#[derive(Deserialize)]
struct Outputs {
    tmy_hourly: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    #[serde(rename = "time(UTC)")]
    time: String,
    #[serde(rename = "T2m")]
    temperature: f64,
    #[serde(rename = "RH")]
    humidity: f64,
    #[serde(rename = "G(h)")]
    global: f64,
    #[serde(rename = "Gb(n)")]
    direct_normal: f64,
    #[serde(rename = "Gd(h)")]
    diffuse: f64,
    #[serde(rename = "IR(h)")]
    infrared: f64,
    #[serde(rename = "WS10m")]
    wind_speed: f64,
    #[serde(rename = "WD10m")]
    wind_direction: f64,
}

/// "20190101:1200" is 1 January, 12:00 UTC. The year differs per month and is not needed.
fn to_hour(row: &Row) -> Result<Hour> {
    let bad_time = || Error::Data {
        service: SERVICE.to_string(),
        detail: format!("expected a time like '20190101:1200', got {:?}", row.time),
    };
    let digits = row
        .time
        .get(4..8)
        .zip(row.time.get(9..11))
        .ok_or_else(bad_time)?;
    let number = |text: &str| text.parse::<u32>().map_err(|_| bad_time());

    Ok(Hour {
        month: number(&digits.0[0..2])?,
        day: number(&digits.0[2..4])?,
        hour: number(digits.1)?,
        temperature_c: row.temperature,
        humidity_pct: row.humidity,
        global_w_m2: row.global,
        direct_normal_w_m2: row.direct_normal,
        diffuse_w_m2: row.diffuse,
        infrared_w_m2: row.infrared,
        wind_m_s: row.wind_speed,
        wind_from_deg: row.wind_direction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "inputs": {
            "location": {"latitude": 52.375, "longitude": 4.884, "elevation": 10.0, "irradiance_time_offset": 0.1839},
            "meteo_data": {"radiation_db": "PVGIS-SARAH3", "meteo_db": "ERA5", "year_min": 2005, "year_max": 2023}
        },
        "outputs": {
            "months_selected": [{"month": 1, "year": 2019}],
            "tmy_hourly": [
                {"time(UTC)": "20190101:1200", "T2m": 4.13, "RH": 92.15, "G(h)": 208.0, "Gb(n)": 402.73,
                 "Gd(h)": 102.0, "IR(h)": 257.6, "WS10m": 2.41, "WD10m": 239.0, "SP": 100620.0}
            ]
        }
    }"#;

    #[test]
    fn a_tmy_row_becomes_an_hour_in_utc() {
        let tmy = parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(tmy.first_year, 2005);
        assert_eq!(tmy.irradiance_offset_hours, 0.1839);
        let hour = tmy.hours[0];
        assert_eq!((hour.month, hour.day, hour.hour), (1, 1, 12));
        assert_eq!(hour.direct_normal_w_m2, 402.73);
        assert_eq!(hour.diffuse_w_m2, 102.0);
        assert_eq!(hour.infrared_w_m2, 257.6);
        assert_eq!(hour.wind_from_deg, 239.0);
    }

    #[test]
    fn neighbouring_sites_share_a_cell() {
        assert_eq!(
            cell_of((120_733.0, 487_521.0)),
            cell_of((121_900.0, 486_100.0))
        );
        assert_ne!(
            cell_of((120_733.0, 487_521.0)),
            cell_of((125_100.0, 487_521.0))
        );
    }
}
