//! Outdoor comfort per month: the daytime hours in each UTCI stress category,
//! once for a person in the sun and once for a person in the shade.
//!
//! Every hour of the typical year gets a mean radiant temperature from
//! `radiant` and then a UTCI from `utci`, with the PVGIS wind at 10 m and
//! humidity as measured.

use crate::area::LonLat;
use crate::sources::pvgis::Hour;
use crate::sun::position_utc;

use super::climate::{Climate, MonthClimate};
use super::radiant::{radiant_temperature, Sky};
use super::utci::{is_in_range, stress_category, utci, STRESS_LABELS};

/// The method and its assumptions in one sentence, for the chart caption and the assistant.
pub const METHOD: &str = "UTCI by the operational polynomial of Bröde et al. (2012), with the mean radiant temperature from the SolarCal model of ASHRAE 55 Appendix C: a standing person on open ground (reflectance 0.25) with the whole sky in view, the sun over their shoulder, surroundings at air temperature, and wind at 10 m";

/// The daytime hours of one month in each stress category.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MonthComfort {
    /// Hours per category of `STRESS_LABELS`, in the sun.
    pub sun_hours: Vec<u32>,
    /// The same in the shade.
    pub shade_hours: Vec<u32>,
    /// Hours whose temperatures lie outside the polynomial's fitted range.
    pub outside_range_hours: u32,
}

/// Whose comfort a count or a chart is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exposure {
    Sun,
    Shade,
}

impl Exposure {
    pub fn title(self) -> &'static str {
        match self {
            Exposure::Sun => "In the sun",
            Exposure::Shade => "In the shade",
        }
    }

    /// The month's daytime hours per stress category.
    pub fn hours(self, month: &MonthClimate) -> &[u32] {
        match self {
            Exposure::Sun => &month.comfort_sun_hours,
            Exposure::Shade => &month.comfort_shade_hours,
        }
    }
}

/// The comfort of `hours`, which are already the daytime hours of one month.
/// `irradiance_offset_hours` moves an hour to the moment its sun values belong to.
pub fn month_comfort(
    hours: &[Hour],
    location: LonLat,
    irradiance_offset_hours: f64,
) -> MonthComfort {
    let mut comfort = MonthComfort {
        sun_hours: vec![0; STRESS_LABELS.len()],
        shade_hours: vec![0; STRESS_LABELS.len()],
        outside_range_hours: 0,
    };
    for hour in hours {
        let utc_hours = hour.hour as f64 + irradiance_offset_hours;
        let sun = position_utc(location, hour.month, hour.day, utc_hours);
        let sky = Sky {
            air_c: hour.temperature_c,
            infrared_w_m2: hour.infrared_w_m2,
            diffuse_w_m2: hour.diffuse_w_m2,
            direct_normal_w_m2: hour.direct_normal_w_m2,
            sun_altitude_deg: sun.altitude_deg,
        };
        let in_sun = radiant_temperature(&sky, true);
        let in_shade = radiant_temperature(&sky, false);
        if !is_in_range(hour.temperature_c, in_sun) {
            comfort.outside_range_hours += 1;
        }

        let sun_utci = utci(hour.temperature_c, in_sun, hour.wind_m_s, hour.humidity_pct);
        let shade_utci = utci(
            hour.temperature_c,
            in_shade,
            hour.wind_m_s,
            hour.humidity_pct,
        );
        comfort.sun_hours[stress_category(sun_utci)] += 1;
        comfort.shade_hours[stress_category(shade_utci)] += 1;
    }

    comfort
}

/// The daytime hours of the whole year in each stress category.
pub fn yearly_hours(climate: &Climate, exposure: Exposure) -> Vec<u32> {
    let mut total = vec![0; STRESS_LABELS.len()];
    for month in &climate.months {
        for (category, hours) in exposure.hours(month).iter().enumerate() {
            total[category] += hours;
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    const DE_BILT: LonLat = LonLat {
        lon: 5.18,
        lat: 52.10,
    };

    fn noon(month: u32, temperature_c: f64, wind_m_s: f64, direct_normal_w_m2: f64) -> Hour {
        Hour {
            month,
            day: 15,
            hour: 11,
            temperature_c,
            humidity_pct: 70.0,
            global_w_m2: 0.0,
            direct_normal_w_m2,
            diffuse_w_m2: 120.0,
            infrared_w_m2: 330.0,
            wind_m_s,
            wind_from_deg: 225.0,
        }
    }

    #[test]
    fn a_mild_sunny_july_noon_has_no_thermal_stress_in_the_shade() {
        let comfort = month_comfort(&[noon(7, 21.0, 3.0, 600.0)], DE_BILT, 0.0);
        assert_eq!(
            STRESS_LABELS[category(&comfort.shade_hours)],
            "No thermal stress"
        );
        assert_eq!(comfort.outside_range_hours, 0);
    }

    #[test]
    fn a_windy_january_day_is_cold_stress_even_in_the_sun() {
        let comfort = month_comfort(&[noon(1, 4.0, 7.0, 300.0)], DE_BILT, 0.0);
        let found = STRESS_LABELS[category(&comfort.sun_hours)];
        assert!(found.ends_with("cold stress"), "{found}");
    }

    #[test]
    fn sun_never_feels_colder_than_shade_and_often_warmer() {
        let mut warmer = 0;
        for temperature in -5..30 {
            let hour = noon(4, temperature as f64, 2.0, 700.0);
            let comfort = month_comfort(&[hour], DE_BILT, 0.0);
            let sun = category(&comfort.sun_hours);
            let shade = category(&comfort.shade_hours);
            assert!(sun >= shade, "{temperature} °C");
            if sun > shade {
                warmer += 1;
            }
        }
        assert!(warmer >= 5, "{warmer}");
    }

    fn category(hours: &[u32]) -> usize {
        hours.iter().position(|&count| count == 1).unwrap()
    }
}
