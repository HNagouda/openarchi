//! Monthly rain at the nearest KNMI station, averaged over full years.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::sources::knmi::{Day, Station};

/// Thirty years, the length of a WMO climate normal.
pub const YEARS: i32 = 30;
/// WMO climate normals count a day with at least 1 mm as a rain day; less
/// is dew or drizzle that does not keep anyone indoors.
pub const RAIN_DAY_MM: f64 = 1.0;

#[derive(Debug, Clone, Serialize)]
pub struct Rain {
    pub station: Station,
    pub distance_km: f64,
    pub first_year: i32,
    pub last_year: i32,
    /// The fewest complete years any month was averaged over.
    pub complete_years: u32,
    /// January first.
    pub months: Vec<RainMonth>,
    pub annual_mm: f64,
    pub annual_rain_days: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RainMonth {
    pub month: u32,
    /// The mean total for this month over its complete years.
    pub mean_mm: f64,
    /// The mean number of days with at least `RAIN_DAY_MM`.
    pub rain_days: f64,
}

/// Averages every calendar month over the years it was fully reported in. A
/// month with a missing day is left out rather than counted as dry.
pub fn from_days(
    station: Station,
    distance_km: f64,
    first_year: i32,
    last_year: i32,
    days: &[Day],
) -> Rain {
    // (year, month) -> (total mm, rain days, reported days, missing days)
    let mut by_month: BTreeMap<(i32, u32), (f64, u32, u32, u32)> = BTreeMap::new();
    for day in days {
        let entry = by_month.entry((day.year, day.month)).or_default();
        match day.millimetres {
            Some(millimetres) => {
                entry.0 += millimetres;
                entry.2 += 1;
                if millimetres >= RAIN_DAY_MM {
                    entry.1 += 1;
                }
            }
            None => entry.3 += 1,
        }
    }

    let mut months = Vec::with_capacity(12);
    let mut complete_years = u32::MAX;
    for month in 1..=12 {
        let complete: Vec<(f64, u32)> = by_month
            .iter()
            .filter(|((year, number), counts)| {
                *number == month && counts.3 == 0 && counts.2 == days_in(*year, month)
            })
            .map(|(_, counts)| (counts.0, counts.1))
            .collect();
        let years = complete.len().max(1) as f64;
        complete_years = complete_years.min(complete.len() as u32);
        months.push(RainMonth {
            month,
            mean_mm: complete.iter().map(|month| month.0).sum::<f64>() / years,
            rain_days: complete.iter().map(|month| month.1 as f64).sum::<f64>() / years,
        });
    }

    Rain {
        station,
        distance_km,
        first_year,
        last_year,
        complete_years,
        annual_mm: months.iter().map(|month| month.mean_mm).sum(),
        annual_rain_days: months.iter().map(|month| month.rain_days).sum(),
        months,
    }
}

fn days_in(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::knmi::STATIONS;

    fn february(year: i32, millimetres: Option<f64>, count: u32) -> Vec<Day> {
        (0..count)
            .map(|_| Day {
                year,
                month: 2,
                millimetres,
            })
            .collect()
    }

    #[test]
    fn a_month_with_a_missing_day_is_left_out_of_the_mean() {
        let mut days = february(2023, Some(2.0), 28);
        days.extend(february(2024, Some(1.0), 29));
        let mut gap = february(2025, Some(5.0), 27);
        gap.extend(february(2025, None, 1));
        days.extend(gap);

        let rain = from_days(STATIONS[1], 9.0, 2023, 2025, &days);
        let february = &rain.months[1];
        assert_eq!(february.mean_mm, (56.0 + 29.0) / 2.0);
        assert_eq!(february.rain_days, (28.0 + 29.0) / 2.0);
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert_eq!(days_in(2024, 2), 29);
        assert_eq!(days_in(1900, 2), 28);
        assert_eq!(days_in(2000, 2), 29);
    }
}
