//! Where the sun stands in the sky, for the sun path and shadow diagrams,
//! the hours of sun, the solar radiation and the light of the axonometric.
//!
//! Uses the NOAA solar calculator equations (after Meeus, "Astronomical
//! Algorithms"): good to about 0.01 degree between 1800 and 2100, far finer
//! than a site drawing needs. Refraction near the horizon is left out, so the
//! sun rises a few minutes later than an almanac says.
//!
//! Times are Dutch clock time, with summer time by the EU rule. The sun repeats
//! its yearly path to within a fraction of a degree, so every moment is taken
//! in one reference year rather than asking the user for a year.

use crate::area::{Area, LonLat};
use crate::view::SunMoment;

/// The year every moment is placed in. Only summer time and leap days depend on it.
pub const REFERENCE_YEAR: i32 = 2026;
/// Central European Time, the Netherlands in winter.
pub const STANDARD_OFFSET_HOURS: f64 = 1.0;
const SUMMER_OFFSET_HOURS: f64 = 2.0;
/// Julian day of 2000-01-01 12:00 UTC, the J2000 epoch.
const J2000: f64 = 2_451_545.0;
const DAYS_PER_CENTURY: f64 = 36_525.0;

/// The sun's position seen from the site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunPosition {
    /// Degrees clockwise from north: 90 east, 180 south.
    pub azimuth_deg: f64,
    /// Degrees above the horizon; negative at night.
    pub altitude_deg: f64,
}

impl SunPosition {
    pub fn is_up(&self) -> bool {
        self.altitude_deg > 0.0
    }
}

/// The middle of the drawn area, where the sun is computed for.
pub fn site_location(area: &Area) -> LonLat {
    match area {
        Area::Circle { center, .. } => *center,
        Area::Rectangle {
            south_west,
            north_east,
        } => LonLat {
            lon: (south_west.lon + north_east.lon) / 2.0,
            lat: (south_west.lat + north_east.lat) / 2.0,
        },
        Area::Polygon { points } => {
            let count = points.len().max(1) as f64;
            LonLat {
                lon: points.iter().map(|p| p.lon).sum::<f64>() / count,
                lat: points.iter().map(|p| p.lat).sum::<f64>() / count,
            }
        }
    }
}

/// The sun at a Dutch clock time, summer time included.
pub fn position_at(location: LonLat, moment: &SunMoment) -> SunPosition {
    let clock_hours = moment.hour as f64 + moment.minute as f64 / 60.0;
    let offset = if is_summer_time(moment.month, moment.day, clock_hours) {
        SUMMER_OFFSET_HOURS
    } else {
        STANDARD_OFFSET_HOURS
    };
    position_utc(location, moment.month, moment.day, clock_hours - offset)
}

/// The sun on a day of the reference year at `utc_hours` (may run below 0 or past 24).
pub fn position_utc(location: LonLat, month: u32, day: u32, utc_hours: f64) -> SunPosition {
    let julian_day = julian_day(REFERENCE_YEAR, month, day) + utc_hours / 24.0;
    let century = (julian_day - J2000) / DAYS_PER_CENTURY;

    let mean_longitude =
        (280.46646 + century * (36000.76983 + century * 0.0003032)).rem_euclid(360.0);
    let mean_anomaly = 357.52911 + century * (35999.05029 - 0.0001537 * century);
    let eccentricity = 0.016708634 - century * (0.000042037 + 0.0000001267 * century);
    let anomaly = mean_anomaly.to_radians();
    let centre = anomaly.sin() * (1.914602 - century * (0.004817 + 0.000014 * century))
        + (2.0 * anomaly).sin() * (0.019993 - 0.000101 * century)
        + (3.0 * anomaly).sin() * 0.000289;
    let omega = (125.04 - 1934.136 * century).to_radians();
    let apparent_longitude =
        (mean_longitude + centre - 0.00569 - 0.00478 * omega.sin()).to_radians();
    let mean_obliquity = 23.0
        + (26.0 + (21.448 - century * (46.815 + century * (0.00059 - century * 0.001813))) / 60.0)
            / 60.0;
    let obliquity = (mean_obliquity + 0.00256 * omega.cos()).to_radians();
    let declination = (obliquity.sin() * apparent_longitude.sin()).asin();

    let y = (obliquity / 2.0).tan().powi(2);
    let longitude = mean_longitude.to_radians();
    let equation_of_time_min = 4.0
        * (y * (2.0 * longitude).sin() - 2.0 * eccentricity * anomaly.sin()
            + 4.0 * eccentricity * y * anomaly.sin() * (2.0 * longitude).cos()
            - 0.5 * y * y * (4.0 * longitude).sin()
            - 1.25 * eccentricity * eccentricity * (2.0 * anomaly).sin())
        .to_degrees();
    let true_solar_min =
        (utc_hours * 60.0 + equation_of_time_min + 4.0 * location.lon).rem_euclid(1440.0);
    let hour_angle = (true_solar_min / 4.0 - 180.0).to_radians();

    let latitude = location.lat.to_radians();
    let cos_zenith =
        latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * hour_angle.cos();
    let zenith = cos_zenith.clamp(-1.0, 1.0).acos();
    let altitude_deg = 90.0 - zenith.to_degrees();

    let denominator = latitude.cos() * zenith.sin();
    let azimuth_deg = if denominator.abs() < 1e-9 {
        180.0
    } else {
        let cos_azimuth =
            ((latitude.sin() * zenith.cos() - declination.sin()) / denominator).clamp(-1.0, 1.0);
        let angle = cos_azimuth.acos().to_degrees();
        if hour_angle > 0.0 {
            (angle + 180.0).rem_euclid(360.0)
        } else {
            (540.0 - angle).rem_euclid(360.0)
        }
    };

    SunPosition {
        azimuth_deg,
        altitude_deg,
    }
}

/// EU summer time: from 01:00 UTC on the last Sunday of March to 01:00 UTC on
/// the last Sunday of October.
pub fn is_summer_time(month: u32, day: u32, clock_hours: f64) -> bool {
    let start = last_sunday(REFERENCE_YEAR, 3);
    let end = last_sunday(REFERENCE_YEAR, 10);
    // The switch happens at 02:00 CET in March and 03:00 CEST in October.
    let switch_clock_hours = 1.0 + STANDARD_OFFSET_HOURS;
    match month {
        4..=9 => true,
        3 => day > start || (day == start && clock_hours >= switch_clock_hours),
        10 => day < end || (day == end && clock_hours < switch_clock_hours + 1.0),
        _ => false,
    }
}

/// The date of the last Sunday of a month.
fn last_sunday(year: i32, month: u32) -> u32 {
    let last_day = if matches!(month, 4 | 6 | 9 | 11) {
        30
    } else {
        31
    };
    let weekday = day_of_week(year, month, last_day);
    last_day - weekday
}

/// 0 for Sunday through 6 for Saturday (Sakamoto's method).
fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let year = if month < 3 { year - 1 } else { year };
    let value =
        year + year / 4 - year / 100 + year / 400 + OFFSETS[(month - 1) as usize] + day as i32;
    value.rem_euclid(7) as u32
}

/// Julian day at 00:00 UTC of a Gregorian date.
fn julian_day(year: i32, month: u32, day: u32) -> f64 {
    let (year, month) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let century = (year as f64 / 100.0).floor();
    let leap_correction = 2.0 - century + (century / 4.0).floor();

    (365.25 * (year as f64 + 4716.0)).floor()
        + (30.6001 * (month as f64 + 1.0)).floor()
        + day as f64
        + leap_correction
        - 1524.5
}

#[cfg(test)]
mod tests {
    use super::*;

    const AMSTERDAM: LonLat = LonLat {
        lon: 4.9041,
        lat: 52.3676,
    };

    /// The highest the sun gets on a day, sampled every minute.
    fn noon_altitude(month: u32, day: u32) -> f64 {
        (0..24 * 60)
            .map(|minute| position_utc(AMSTERDAM, month, day, minute as f64 / 60.0).altitude_deg)
            .fold(f64::MIN, f64::max)
    }

    #[test]
    fn summer_solstice_noon_is_about_61_degrees_in_amsterdam() {
        // 90 - 52.37 + 23.44.
        assert!((noon_altitude(6, 21) - 61.07).abs() < 0.1);
    }

    #[test]
    fn winter_solstice_noon_is_about_14_degrees_in_amsterdam() {
        // 90 - 52.37 - 23.44.
        assert!((noon_altitude(12, 21) - 14.19).abs() < 0.1);
    }

    #[test]
    fn equinox_noon_is_the_colatitude() {
        // The 2026 March equinox falls on 20 March, 14:46 UTC.
        assert!((noon_altitude(3, 20) - (90.0 - AMSTERDAM.lat)).abs() < 0.2);
    }

    #[test]
    fn solar_noon_in_june_is_about_13_42_summer_time() {
        // 12:00 UTC, less 19.6 minutes for 4.9 degrees east, plus 1.8 minutes
        // for the equation of time, is 11:42 UTC or 13:42 CEST.
        let moment = SunMoment {
            month: 6,
            day: 21,
            hour: 13,
            minute: 42,
        };
        let position = position_at(AMSTERDAM, &moment);
        assert!((position.azimuth_deg - 180.0).abs() < 0.5, "{position:?}");
    }

    #[test]
    fn the_sun_rises_in_the_north_east_in_june() {
        // Around 05:20 CEST, at an azimuth near 50 degrees.
        let moment = SunMoment {
            month: 6,
            day: 21,
            hour: 5,
            minute: 30,
        };
        let position = position_at(AMSTERDAM, &moment);
        assert!(position.is_up());
        assert!((40.0..60.0).contains(&position.azimuth_deg), "{position:?}");
    }

    #[test]
    fn summer_time_starts_on_the_last_sunday_of_march_2026() {
        assert_eq!(last_sunday(2026, 3), 29);
        assert_eq!(last_sunday(2026, 10), 25);
        assert!(!is_summer_time(3, 21, 13.0));
        assert!(is_summer_time(3, 29, 12.0));
        assert!(is_summer_time(6, 21, 12.0));
        assert!(!is_summer_time(10, 26, 12.0));
    }
}
