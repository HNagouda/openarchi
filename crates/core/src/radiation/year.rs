//! The sunlight of a typical year, cut down to what the radiation diagram
//! needs and small enough to keep in a scene: for each month, the mean
//! direct and diffuse irradiance at each hour of the day.
//!
//! The full typical meteorological year has 8,760 hourly rows. Averaging each
//! hour of the day over its month keeps the month's total diffuse light
//! exactly and its direct light as the sun stands on that month's average day,
//! which is how `radiation.rs` sums it.

use serde::{Deserialize, Serialize};

use crate::meteo::incomplete_year;
use crate::sources::pvgis::Tmy;
use crate::Result;

const MONTHS: usize = 12;
const HOURS_PER_DAY: usize = 24;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarYear {
    /// The years the typical months were picked from, as the source names them: 2005 and 2023.
    pub first_year: i32,
    pub last_year: i32,
    /// Hours to add to an hour of the day to get the moment its sun values belong to.
    pub irradiance_offset_hours: f64,
    /// January to December.
    pub months: Vec<SolarMonth>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarMonth {
    /// Days the month has in the typical year.
    pub days: u32,
    /// Mean direct irradiance on a plane facing the sun (DNI), W/m², per UTC hour 0 to 23.
    pub direct_normal_w_m2: Vec<f64>,
    /// Mean diffuse irradiance on a level plane (DHI), W/m², per UTC hour 0 to 23.
    pub diffuse_w_m2: Vec<f64>,
}

impl SolarYear {
    /// The monthly means of a typical year. Fails on a year that lacks a month,
    /// rather than drawing a total that quietly leaves it out.
    pub fn from_tmy(tmy: &Tmy) -> Result<SolarYear> {
        let mut direct = [[0.0; HOURS_PER_DAY]; MONTHS];
        let mut diffuse = [[0.0; HOURS_PER_DAY]; MONTHS];
        let mut days = [0u32; MONTHS];
        for hour in &tmy.hours {
            let month = hour.month as usize;
            let hour_of_day = hour.hour as usize;
            if !(1..=MONTHS).contains(&month) || hour_of_day >= HOURS_PER_DAY {
                return Err(incomplete_year(format!(
                    "an hour on month {} at {}:00",
                    hour.month, hour.hour
                )));
            }
            direct[month - 1][hour_of_day] += hour.direct_normal_w_m2;
            diffuse[month - 1][hour_of_day] += hour.diffuse_w_m2;
            if hour_of_day == 0 {
                days[month - 1] += 1;
            }
        }
        if let Some(empty) = days.iter().position(|&count| count == 0) {
            return Err(incomplete_year(format!("no hours in month {}", empty + 1)));
        }

        let months = (0..MONTHS)
            .map(|month| {
                let count = f64::from(days[month]);
                SolarMonth {
                    days: days[month],
                    direct_normal_w_m2: direct[month].iter().map(|sum| sum / count).collect(),
                    diffuse_w_m2: diffuse[month].iter().map(|sum| sum / count).collect(),
                }
            })
            .collect();
        Ok(SolarYear {
            first_year: tmy.first_year,
            last_year: tmy.last_year,
            irradiance_offset_hours: tmy.irradiance_offset_hours,
            months,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::LonLat;
    use crate::sources::pvgis::Hour;

    fn hour(month: u32, day: u32, hour: u32, direct: f64, diffuse: f64) -> Hour {
        Hour {
            month,
            day,
            hour,
            temperature_c: 10.0,
            humidity_pct: 80.0,
            global_w_m2: 0.0,
            direct_normal_w_m2: direct,
            diffuse_w_m2: diffuse,
            infrared_w_m2: 300.0,
            wind_m_s: 3.0,
            wind_from_deg: 225.0,
        }
    }

    /// Every hour of the year with the same light, except two January noons.
    fn year() -> Tmy {
        let mut hours = Vec::new();
        for month in 1..=12 {
            for day in 1..=28 {
                for hour_of_day in 0..24 {
                    hours.push(hour(month, day, hour_of_day, 100.0, 50.0));
                }
            }
        }
        hours[12].direct_normal_w_m2 = 400.0;
        hours[24 + 12].direct_normal_w_m2 = 600.0;

        Tmy {
            location: LonLat {
                lon: 4.9,
                lat: 52.4,
            },
            elevation_m: 0.0,
            first_year: 2005,
            last_year: 2023,
            irradiance_offset_hours: 0.18,
            hours,
        }
    }

    #[test]
    fn each_hour_of_the_day_is_averaged_over_its_month() {
        let solar = SolarYear::from_tmy(&year()).unwrap();
        let january = &solar.months[0];

        assert_eq!(solar.months.len(), 12);
        assert_eq!(january.days, 28);
        assert_eq!(january.diffuse_w_m2[3], 50.0);
        // (400 + 600 + 26 × 100) / 28.
        assert!((january.direct_normal_w_m2[12] - 3600.0 / 28.0).abs() < 1e-9);
    }

    #[test]
    fn a_year_without_a_month_is_refused() {
        let mut tmy = year();
        tmy.hours.retain(|hour| hour.month != 7);
        let error = SolarYear::from_tmy(&tmy).unwrap_err().to_string();

        assert!(error.contains("no hours in month 7"), "{error}");
    }
}
