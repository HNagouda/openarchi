//! The wind of a typical year, cut down to what the wind diagram needs and
//! small enough to keep in a scene: for each month, the hours the wind came
//! from each of 16 compass sectors, and the calm hours.
//!
//! The sectors and the calm limit are the wind roses' (`meteo.rs`), so the
//! diagram's prevailing wind is the longest wedge of the matching rose.

use serde::{Deserialize, Serialize};

use crate::meteo::{incomplete_year, sector_of, CALM_M_S, SECTORS, SECTOR_DEG, SECTOR_NAMES};
use crate::sources::pvgis::Tmy;
use crate::Result;

const MONTHS: usize = 12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindYear {
    /// The years the typical months were picked from, as the source names them: 2005 and 2023.
    pub first_year: i32,
    pub last_year: i32,
    /// January to December.
    pub months: Vec<WindMonth>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindMonth {
    /// Hours with wind from each sector, north first and clockwise, calm hours left out.
    pub hours_from: Vec<u32>,
    /// Hours below Beaufort 2, which have no direction worth drawing.
    pub calm_hours: u32,
}

/// The direction the wind comes from most often in some months.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prevailing {
    /// The middle of the sector, degrees clockwise from north: 225 for the SW.
    pub from_deg: f64,
    /// "SW".
    pub name: &'static str,
    /// Share of all the months' hours, calm ones included, 0 to 1.
    pub share: f64,
}

impl WindYear {
    /// The hours of a typical year counted by month and sector. Fails on a
    /// year that lacks a month, as the solar year does.
    pub fn from_tmy(tmy: &Tmy) -> Result<WindYear> {
        let mut months = vec![
            WindMonth {
                hours_from: vec![0; SECTORS],
                calm_hours: 0,
            };
            MONTHS
        ];
        for hour in &tmy.hours {
            let Some(month) = months.get_mut((hour.month as usize).wrapping_sub(1)) else {
                return Err(incomplete_year(format!("an hour in month {}", hour.month)));
            };
            if hour.wind_m_s < CALM_M_S {
                month.calm_hours += 1;
            } else {
                month.hours_from[sector_of(hour.wind_from_deg)] += 1;
            }
        }
        if let Some(empty) = months.iter().position(|month| month.total() == 0) {
            return Err(incomplete_year(format!("no hours in month {}", empty + 1)));
        }

        Ok(WindYear {
            first_year: tmy.first_year,
            last_year: tmy.last_year,
            months,
        })
    }

    /// The prevailing wind over `months` (1 to 12). `None` when every hour was calm.
    pub fn prevailing(&self, months: &[u32]) -> Option<Prevailing> {
        let chosen: Vec<&WindMonth> = months
            .iter()
            .filter_map(|&month| self.months.get((month as usize).wrapping_sub(1)))
            .collect();
        let mut hours_from = [0u32; SECTORS];
        for month in &chosen {
            for (total, hours) in hours_from.iter_mut().zip(&month.hours_from) {
                *total += hours;
            }
        }
        let all_hours: u32 = chosen.iter().map(|month| month.total()).sum();
        // The first of equal sectors, so the answer never depends on iteration order.
        let (sector, &most) = hours_from
            .iter()
            .enumerate()
            .rev()
            .max_by_key(|(_, &hours)| hours)?;
        if most == 0 {
            return None;
        }

        Some(Prevailing {
            from_deg: sector as f64 * SECTOR_DEG,
            name: SECTOR_NAMES[sector],
            share: f64::from(most) / f64::from(all_hours),
        })
    }
}

impl WindMonth {
    fn total(&self) -> u32 {
        self.calm_hours + self.hours_from.iter().sum::<u32>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::LonLat;
    use crate::sources::pvgis::Hour;

    fn hour(month: u32, wind_m_s: f64, wind_from_deg: f64) -> Hour {
        Hour {
            month,
            day: 1,
            hour: 0,
            temperature_c: 10.0,
            humidity_pct: 80.0,
            global_w_m2: 0.0,
            direct_normal_w_m2: 0.0,
            diffuse_w_m2: 0.0,
            infrared_w_m2: 300.0,
            wind_m_s,
            wind_from_deg,
        }
    }

    /// A west wind every month, with a stronger pull from the north-east in January.
    fn year() -> Tmy {
        let mut hours = Vec::new();
        for month in 1..=12 {
            hours.push(hour(month, 4.0, 268.0));
            hours.push(hour(month, 4.0, 272.0));
            hours.push(hour(month, 0.5, 90.0));
        }
        for _ in 0..3 {
            hours.push(hour(1, 6.0, 44.0));
        }
        Tmy {
            location: LonLat {
                lon: 4.88,
                lat: 52.37,
            },
            elevation_m: 0.0,
            first_year: 2005,
            last_year: 2023,
            irradiance_offset_hours: 0.0,
            hours,
        }
    }

    #[test]
    fn calm_hours_have_no_direction_and_the_rest_fall_in_their_sector() {
        let wind = WindYear::from_tmy(&year()).unwrap();
        let january = &wind.months[0];
        assert_eq!(january.calm_hours, 1);
        assert_eq!(january.hours_from[12], 2);
        assert_eq!(january.hours_from[2], 3);
    }

    #[test]
    fn the_prevailing_wind_depends_on_the_months_asked_for() {
        let wind = WindYear::from_tmy(&year()).unwrap();
        let january = wind.prevailing(&[1]).unwrap();
        let all_year = wind.prevailing(&(1..=12).collect::<Vec<_>>()).unwrap();

        assert_eq!((january.name, january.from_deg), ("NE", 45.0));
        assert!((january.share - 0.5).abs() < 1e-9);
        assert_eq!((all_year.name, all_year.from_deg), ("W", 270.0));
    }

    #[test]
    fn a_year_without_a_month_is_refused() {
        let mut tmy = year();
        tmy.hours.retain(|hour| hour.month != 7);
        assert!(WindYear::from_tmy(&tmy).is_err());
    }
}
