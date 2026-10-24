//! KNMI daily precipitation from the nearest weather station.
//!
//! PVGIS has no rain, so rain comes from the KNMI station network. Only about
//! thirty stations measure it over a long run, so a site gets the nearest of
//! them and the chart says which one and how far away it is.

use crate::area::LonLat;
use crate::scene::Credit;
use crate::sources::http;
use crate::{Error, Result};

const SERVICE: &str = "KNMI daggegevens";
const DAILY_URL: &str = "https://www.daggegevens.knmi.nl/klimatologie/daggegevens";
/// Daily precipitation total, in tenths of a millimetre.
const PRECIPITATION: &str = "RH";
/// What KNMI writes for a day with less than 0.05 mm.
const TRACE_VALUE: i64 = -1;
const TENTHS: f64 = 10.0;

pub const CACHE_DATASET: &str = "knmi-daily-rain";

const EARTH_RADIUS_KM: f64 = 6_371.0;

pub fn credit() -> Credit {
    Credit {
        name: "KNMI daily weather data".to_string(),
        license: "CC BY 4.0".to_string(),
        url: "https://www.knmi.nl/nederland-nu/klimatologie/daggegevens".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Station {
    pub id: u32,
    pub name: &'static str,
    pub location: LonLat,
}

/// Every KNMI station with at least 25 complete years of daily rain between
/// 1996 and 2025, checked against the service on 2026-10-24, in one place.
/// Positions are the ones KNMI lists in the header of its answers. Wind-only
/// and coastal stations without a rain gauge are left out.
pub const STATIONS: [Station; 30] = [
    station(235, "De Kooy", 4.781, 52.928),
    station(240, "Schiphol", 4.790, 52.318),
    station(249, "Berkhout", 4.979, 52.644),
    station(251, "Hoorn (Terschelling)", 5.346, 53.392),
    station(260, "De Bilt", 5.180, 52.100),
    station(267, "Stavoren", 5.384, 52.898),
    station(269, "Lelystad", 5.520, 52.458),
    station(270, "Leeuwarden", 5.752, 53.224),
    station(273, "Marknesse", 5.888, 52.703),
    station(275, "Deelen", 5.873, 52.056),
    station(277, "Lauwersoog", 6.200, 53.413),
    station(278, "Heino", 6.259, 52.435),
    station(279, "Hoogeveen", 6.574, 52.750),
    station(280, "Groningen Eelde", 6.585, 53.125),
    station(283, "Hupsel", 6.657, 52.069),
    station(286, "Nieuw Beerta", 7.150, 53.196),
    station(290, "Twenthe", 6.891, 52.274),
    station(310, "Vlissingen", 3.596, 51.442),
    station(319, "Westdorpe", 3.861, 51.226),
    station(323, "Wilhelminadorp", 3.884, 51.527),
    station(330, "Hoek van Holland", 4.122, 51.992),
    station(344, "Rotterdam", 4.447, 51.962),
    station(348, "Cabauw", 4.926, 51.970),
    station(350, "Gilze-Rijen", 4.936, 51.566),
    station(356, "Herwijnen", 5.146, 51.859),
    station(370, "Eindhoven", 5.377, 51.451),
    station(375, "Volkel", 5.707, 51.659),
    station(377, "Ell", 5.763, 51.198),
    station(380, "Maastricht", 5.762, 50.906),
    station(391, "Arcen", 6.197, 51.498),
];

const fn station(id: u32, name: &'static str, lon: f64, lat: f64) -> Station {
    Station {
        id,
        name,
        location: LonLat { lon, lat },
    }
}

/// One day's rain. `None` when the station did not report it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Day {
    pub year: i32,
    pub month: u32,
    pub millimetres: Option<f64>,
}

/// The nearest station to `location` and its distance in kilometres.
pub fn nearest(location: LonLat) -> (Station, f64) {
    let mut best = (STATIONS[0], f64::INFINITY);
    for candidate in STATIONS {
        let distance = distance_km(location, candidate.location);
        if distance < best.1 {
            best = (candidate, distance);
        }
    }

    best
}

/// The raw CSV for every day from 1 January `first_year` to 31 December `last_year`.
pub fn fetch(station_id: u32, first_year: i32, last_year: i32) -> Result<Vec<u8>> {
    let station = station_id.to_string();
    let start = format!("{first_year}0101");
    let end = format!("{last_year}1231");
    http::get(
        SERVICE,
        DAILY_URL,
        &[
            ("stns", station.as_str()),
            ("vars", PRECIPITATION),
            ("start", start.as_str()),
            ("end", end.as_str()),
        ],
    )
}

/// Rows look like "  240,19960107,   30": station, date, tenths of a millimetre.
/// Comment lines start with '#'.
pub fn parse(bytes: &[u8]) -> Result<Vec<Day>> {
    let text = String::from_utf8_lossy(bytes);
    let mut days = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        days.push(parse_row(line)?);
    }

    if days.is_empty() {
        return Err(Error::Data {
            service: SERVICE.to_string(),
            detail: "the answer held no days".to_string(),
        });
    }
    Ok(days)
}

fn parse_row(line: &str) -> Result<Day> {
    let bad_row = || Error::Data {
        service: SERVICE.to_string(),
        detail: format!("expected a row like '240,19960107,30', got {line:?}"),
    };
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    let [_, date, value] = fields.as_slice() else {
        return Err(bad_row());
    };
    let year = date
        .get(0..4)
        .and_then(|text| text.parse().ok())
        .ok_or_else(bad_row)?;
    let month = date
        .get(4..6)
        .and_then(|text| text.parse().ok())
        .ok_or_else(bad_row)?;
    let millimetres = match value.parse::<i64>() {
        Ok(TRACE_VALUE) => Some(0.0),
        Ok(tenths) => Some(tenths as f64 / TENTHS),
        Err(_) if value.is_empty() => None,
        Err(_) => return Err(bad_row()),
    };

    Ok(Day {
        year,
        month,
        millimetres,
    })
}

/// Great-circle distance; at Dutch distances the earth may as well be a sphere.
fn distance_km(from: LonLat, to: LonLat) -> f64 {
    let (lat1, lat2) = (from.lat.to_radians(), to.lat.to_radians());
    let half_dlat = (lat2 - lat1) / 2.0;
    let half_dlon = (to.lon - from.lon).to_radians() / 2.0;
    let a = half_dlat.sin().powi(2) + lat1.cos() * lat2.cos() * half_dlon.sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_westerkerk_is_nearest_to_schiphol() {
        let (station, distance) = nearest(LonLat {
            lon: 4.884,
            lat: 52.3745,
        });
        assert_eq!(station.id, 240);
        assert!((distance - 8.9).abs() < 0.5, "{distance}");
    }

    #[test]
    fn rows_become_days_in_millimetres_and_traces_count_as_dry() {
        let text = "# STN,YYYYMMDD,   RH\n  240,19960106,   -1\n  240,19960107,   30\n  209,20250101,     \n";
        let days = parse(text.as_bytes()).unwrap();
        assert_eq!(days.len(), 3);
        assert_eq!(days[0].millimetres, Some(0.0));
        assert_eq!(days[1].millimetres, Some(3.0));
        assert_eq!((days[1].year, days[1].month), (1996, 1));
        assert_eq!(days[2].millimetres, None);
    }

    #[test]
    fn an_html_error_page_is_a_data_error() {
        assert!(parse(b"<!DOCTYPE html>\n<html>").is_err());
    }
}
