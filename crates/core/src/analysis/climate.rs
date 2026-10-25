//! The climate of a site, worked out from the PVGIS typical year: monthly
//! temperature, humidity and sun, outdoor comfort, wind by season, and where
//! the direct sun comes from.

use std::collections::BTreeMap;

use serde::Serialize;

use super::comfort::{self, month_comfort, MonthComfort};
use super::utci::STRESS_LABELS;
use crate::area::LonLat;
use crate::meteo::{sector_of, CALM_M_S, SEASONS, SECTORS, SECTOR_NAMES};
use crate::sources::pvgis::{Hour, Tmy};
use crate::sun::{position_utc, STANDARD_OFFSET_HOURS};

/// The hours people use outdoor space, in Dutch winter time: 8:00 to 22:00.
pub const DAY_START_HOUR: u32 = 8;
pub const DAY_END_HOUR: u32 = 22;

/// Wind speed band edges in m/s at 10 m: the Beaufort scale (WMO). Below the
/// first edge is calm (Beaufort 0 and 1), drawn as the rose's centre.
pub const WIND_EDGES_M_S: [f64; 4] = [CALM_M_S, 3.4, 5.5, 8.0];
pub const WIND_LABELS: [&str; 4] = [
    "Light, Bft 2",
    "Gentle, Bft 3",
    "Moderate, Bft 4",
    "Fresh and up, Bft 5+",
];
/// The half of the year with low sun, October to March, for the radiation rose.
pub const WINTER_HALF: [u32; 6] = [10, 11, 12, 1, 2, 3];

const WH_PER_KWH: f64 = 1000.0;

#[derive(Debug, Clone, Serialize)]
pub struct Climate {
    /// Where PVGIS computed the year, the centre of a 5 km cell.
    pub location: LonLat,
    pub elevation_m: f64,
    pub first_year: i32,
    pub last_year: i32,
    /// January first.
    pub months: Vec<MonthClimate>,
    /// How the comfort hours were worked out, in one sentence.
    pub comfort_method: String,
    /// The UTCI stress categories the comfort hours count, coldest first.
    pub comfort_categories: Vec<String>,
    /// Daytime hours outside the range the UTCI polynomial was fitted over.
    pub comfort_outside_range_hours: u32,
    pub wind: Vec<WindRose>,
    /// Direct sun by the direction it comes from, north first.
    pub sun: Vec<SunSector>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthClimate {
    /// 1 for January.
    pub month: u32,
    pub mean_c: f64,
    /// The mean of each day's highest hour.
    pub mean_daily_max_c: f64,
    pub mean_daily_min_c: f64,
    pub humidity_pct: f64,
    /// Global horizontal irradiation (GHI) over the month.
    pub global_kwh_m2: f64,
    /// Direct normal irradiation (DNI) over the month.
    pub direct_normal_kwh_m2: f64,
    /// Hours from 8:00 to 22:00 in each of `comfort_categories`, for a person in the sun.
    pub comfort_sun_hours: Vec<u32>,
    /// The same for a person in the shade.
    pub comfort_shade_hours: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindRose {
    pub season: String,
    pub months: String,
    pub hours: u32,
    /// Share of the season's hours below the first band, 0 to 1.
    pub calm_share: f64,
    pub mean_speed_m_s: f64,
    /// North first, clockwise.
    pub sectors: Vec<WindSector>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindSector {
    /// "SW".
    pub from: String,
    /// Share of the season's hours in each band of `WIND_LABELS`, 0 to 1.
    pub shares: Vec<f64>,
}

impl WindSector {
    pub fn total(&self) -> f64 {
        self.shares.iter().sum()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SunSector {
    /// "S".
    pub from: String,
    /// Direct normal irradiation arriving from this direction, October to March.
    pub winter_kwh_m2: f64,
    /// The same, April to September.
    pub summer_kwh_m2: f64,
}

pub fn from_tmy(tmy: &Tmy) -> Climate {
    let mut months = Vec::new();
    let mut outside_range_hours = 0;
    for month in 1..=12 {
        let hours = hours_in(tmy, &[month]);
        let comfort = daytime_comfort(&hours, tmy);
        outside_range_hours += comfort.outside_range_hours;
        months.push(month_climate(month, &hours, comfort));
    }
    let wind = SEASONS
        .iter()
        .map(|(season, months, members)| wind_rose(season, months, &hours_in(tmy, members)))
        .collect();

    Climate {
        location: tmy.location,
        elevation_m: tmy.elevation_m,
        first_year: tmy.first_year,
        last_year: tmy.last_year,
        months,
        comfort_method: comfort::METHOD.to_string(),
        comfort_categories: STRESS_LABELS
            .iter()
            .map(|label| label.to_string())
            .collect(),
        comfort_outside_range_hours: outside_range_hours,
        wind,
        sun: sun_sectors(tmy),
    }
}

/// Which band of `edges` a value falls in: n edges make n + 1 bands.
pub fn band_of(value: f64, edges: &[f64]) -> usize {
    edges.iter().filter(|&&edge| value >= edge).count()
}

fn hours_in(tmy: &Tmy, months: &[u32]) -> Vec<Hour> {
    tmy.hours
        .iter()
        .filter(|hour| months.contains(&hour.month))
        .copied()
        .collect()
}

fn month_climate(month: u32, hours: &[Hour], comfort: MonthComfort) -> MonthClimate {
    let count = hours.len().max(1) as f64;
    let mean = |value: fn(&Hour) -> f64| hours.iter().map(value).sum::<f64>() / count;

    // Days are UTC days; an hour's shift moves a daily extreme very rarely.
    let mut days: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
    for hour in hours {
        let day = days
            .entry(hour.day)
            .or_insert((f64::INFINITY, f64::NEG_INFINITY));
        day.0 = day.0.min(hour.temperature_c);
        day.1 = day.1.max(hour.temperature_c);
    }
    let day_count = days.len().max(1) as f64;

    MonthClimate {
        month,
        mean_c: mean(|hour| hour.temperature_c),
        mean_daily_max_c: days.values().map(|day| day.1).sum::<f64>() / day_count,
        mean_daily_min_c: days.values().map(|day| day.0).sum::<f64>() / day_count,
        humidity_pct: mean(|hour| hour.humidity_pct),
        global_kwh_m2: hours.iter().map(|hour| hour.global_w_m2).sum::<f64>() / WH_PER_KWH,
        direct_normal_kwh_m2: hours
            .iter()
            .map(|hour| hour.direct_normal_w_m2)
            .sum::<f64>()
            / WH_PER_KWH,
        comfort_sun_hours: comfort.sun_hours,
        comfort_shade_hours: comfort.shade_hours,
    }
}

fn daytime_comfort(hours: &[Hour], tmy: &Tmy) -> MonthComfort {
    let daytime: Vec<Hour> = hours
        .iter()
        .filter(|hour| is_daytime(hour))
        .copied()
        .collect();
    month_comfort(&daytime, tmy.location, tmy.irradiance_offset_hours)
}

fn is_daytime(hour: &Hour) -> bool {
    let local = (hour.hour as f64 + STANDARD_OFFSET_HOURS) as u32 % 24;
    (DAY_START_HOUR..DAY_END_HOUR).contains(&local)
}

fn wind_rose(season: &str, months: &str, hours: &[Hour]) -> WindRose {
    let count = hours.len().max(1) as f64;
    let mut shares = vec![vec![0.0; WIND_LABELS.len()]; SECTORS];
    let mut calm = 0.0;
    for hour in hours {
        let band = band_of(hour.wind_m_s, &WIND_EDGES_M_S);
        if band == 0 {
            calm += 1.0 / count;
        } else {
            shares[sector_of(hour.wind_from_deg)][band - 1] += 1.0 / count;
        }
    }

    WindRose {
        season: season.to_string(),
        months: months.to_string(),
        hours: hours.len() as u32,
        calm_share: calm,
        mean_speed_m_s: hours.iter().map(|hour| hour.wind_m_s).sum::<f64>() / count,
        sectors: shares
            .into_iter()
            .enumerate()
            .map(|(index, shares)| WindSector {
                from: SECTOR_NAMES[index].to_string(),
                shares,
            })
            .collect(),
    }
}

/// Direct sun summed by the sun's compass direction at each hour.
fn sun_sectors(tmy: &Tmy) -> Vec<SunSector> {
    let mut winter = [0.0; SECTORS];
    let mut summer = [0.0; SECTORS];
    for hour in tmy
        .hours
        .iter()
        .filter(|hour| hour.direct_normal_w_m2 > 0.0)
    {
        let utc_hours = hour.hour as f64 + tmy.irradiance_offset_hours;
        let sun = position_utc(tmy.location, hour.month, hour.day, utc_hours);
        if !sun.is_up() {
            continue;
        }
        let sector = sector_of(sun.azimuth_deg);
        let kwh = hour.direct_normal_w_m2 / WH_PER_KWH;
        if WINTER_HALF.contains(&hour.month) {
            winter[sector] += kwh;
        } else {
            summer[sector] += kwh;
        }
    }

    (0..SECTORS)
        .map(|index| SunSector {
            from: SECTOR_NAMES[index].to_string(),
            winter_kwh_m2: winter[index],
            summer_kwh_m2: summer[index],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hour(month: u32, day: u32, hour: u32, temperature_c: f64, wind: (f64, f64)) -> Hour {
        Hour {
            month,
            day,
            hour,
            temperature_c,
            humidity_pct: 80.0,
            global_w_m2: 100.0,
            direct_normal_w_m2: 0.0,
            diffuse_w_m2: 100.0,
            infrared_w_m2: 300.0,
            wind_m_s: wind.0,
            wind_from_deg: wind.1,
        }
    }

    #[test]
    fn bands_count_the_edges_a_value_has_passed() {
        assert_eq!(band_of(1.0, &WIND_EDGES_M_S), 0);
        assert_eq!(band_of(3.4, &WIND_EDGES_M_S), 2);
        assert_eq!(band_of(12.0, &WIND_EDGES_M_S), 4);
    }

    #[test]
    fn a_month_has_daily_extremes() {
        let hours = vec![
            hour(1, 1, 3, 2.0, (2.0, 0.0)),
            hour(1, 1, 12, 8.0, (2.0, 0.0)),
            hour(1, 2, 3, 4.0, (2.0, 0.0)),
            hour(1, 2, 12, 12.0, (2.0, 0.0)),
        ];
        let month = month_climate(1, &hours, MonthComfort::default());
        assert_eq!(month.mean_c, 6.5);
        assert_eq!(month.mean_daily_max_c, 10.0);
        assert_eq!(month.mean_daily_min_c, 3.0);
        assert_eq!(month.global_kwh_m2, 0.4);
    }

    #[test]
    fn calm_hours_go_to_the_centre_and_the_rest_to_their_sector() {
        let hours = vec![
            hour(1, 1, 0, 5.0, (0.5, 200.0)),
            hour(1, 1, 1, 5.0, (6.0, 225.0)),
            hour(1, 1, 2, 5.0, (6.0, 230.0)),
            hour(1, 1, 3, 5.0, (9.0, 90.0)),
        ];
        let rose = wind_rose("Winter", "Dec–Feb", &hours);
        assert_eq!(rose.calm_share, 0.25);
        assert_eq!(rose.sectors[10].from, "SW");
        assert_eq!(rose.sectors[10].shares, vec![0.0, 0.0, 0.5, 0.0]);
        assert_eq!(rose.sectors[4].shares, vec![0.0, 0.0, 0.0, 0.25]);
    }
}
