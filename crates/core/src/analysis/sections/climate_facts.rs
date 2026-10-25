//! The facts of each climate section: temperature, outdoor comfort, wind, sun
//! and humidity.

use super::fact;
use crate::analysis::climate::{sector_name, Climate};
use crate::analysis::comfort::{yearly_hours, Exposure};
use crate::analysis::figure::percent;
use crate::analysis::utci::NO_STRESS;
use crate::analysis::Fact;
use crate::ratios::grouped;

pub(super) fn temperature_facts(climate: &Climate) -> Vec<Fact> {
    let count = climate.months.len().max(1) as f64;
    let annual = climate.months.iter().map(|month| month.mean_c).sum::<f64>() / count;
    let warmest = climate
        .months
        .iter()
        .map(|month| month.mean_daily_max_c)
        .fold(f64::NEG_INFINITY, f64::max);
    let coldest = climate
        .months
        .iter()
        .map(|month| month.mean_daily_min_c)
        .fold(f64::INFINITY, f64::min);
    vec![
        fact("Annual mean", format!("{annual:.1} °C")),
        fact("Warmest highs", format!("{warmest:.0} °C")),
        fact("Coldest lows", format!("{coldest:.0} °C")),
    ]
}

pub(super) fn comfort_facts(climate: &Climate) -> Vec<Fact> {
    let sun = yearly_hours(climate, Exposure::Sun);
    let shade = yearly_hours(climate, Exposure::Shade);
    let total = sun.iter().sum::<u32>().max(1) as f64;
    let cold_in_shade: u32 = shade[..NO_STRESS].iter().sum();
    let heat_in_sun: u32 = sun[NO_STRESS + 1..].iter().sum();
    vec![
        fact(
            "No thermal stress, sun",
            format!(
                "{} of daytime hours",
                percent(sun[NO_STRESS] as f64 / total)
            ),
        ),
        fact(
            "No thermal stress, shade",
            format!(
                "{} of daytime hours",
                percent(shade[NO_STRESS] as f64 / total)
            ),
        ),
        fact(
            "Cold stress, shade",
            format!("{} of daytime hours", percent(cold_in_shade as f64 / total)),
        ),
        fact("Heat stress, sun", format!("{heat_in_sun} hours a year")),
    ]
}

pub(super) fn wind_facts(climate: &Climate) -> Vec<Fact> {
    let mut facts = Vec::new();
    for rose in &climate.wind {
        let Some(prevailing) = rose
            .sectors
            .iter()
            .max_by(|a, b| a.total().total_cmp(&b.total()))
        else {
            continue;
        };
        facts.push(fact(
            &rose.season,
            format!("{}, {:.1} m/s mean", prevailing.from, rose.mean_speed_m_s),
        ));
    }

    facts
}

pub(super) fn sun_facts(climate: &Climate) -> Vec<Fact> {
    let global: f64 = climate.months.iter().map(|month| month.global_kwh_m2).sum();
    let direct: f64 = climate
        .months
        .iter()
        .map(|month| month.direct_normal_kwh_m2)
        .sum();
    let strongest = climate
        .sun
        .iter()
        .enumerate()
        .max_by(|a, b| {
            (a.1.winter_kwh_m2 + a.1.summer_kwh_m2)
                .total_cmp(&(b.1.winter_kwh_m2 + b.1.summer_kwh_m2))
        })
        .map(|(index, _)| sector_name(index as f64 * 22.5))
        .unwrap_or("S");
    vec![
        fact("Global, per year", format!("{} kWh/m²", grouped(global))),
        fact("Direct, per year", format!("{} kWh/m²", grouped(direct))),
        fact("Most direct sun from", strongest.to_string()),
    ]
}

pub(super) fn humidity_facts(climate: &Climate) -> Vec<Fact> {
    let count = climate.months.len().max(1) as f64;
    let annual = climate
        .months
        .iter()
        .map(|month| month.humidity_pct)
        .sum::<f64>()
        / count;
    let driest = climate
        .months
        .iter()
        .map(|month| month.humidity_pct)
        .fold(f64::INFINITY, f64::min);
    let dampest = climate
        .months
        .iter()
        .map(|month| month.humidity_pct)
        .fold(f64::NEG_INFINITY, f64::max);
    vec![
        fact("Annual mean", format!("{annual:.0} %")),
        fact("Monthly range", format!("{driest:.0}–{dampest:.0} %")),
    ]
}
