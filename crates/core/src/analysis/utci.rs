//! The Universal Thermal Climate Index (UTCI): how warm the outdoor weather
//! feels to a walking person, in °C, from air temperature, mean radiant
//! temperature, wind and humidity.
//!
//! This is the operational polynomial of Bröde et al. (2012), "Deriving the
//! operational procedure for the Universal Thermal Climate Index (UTCI)", Int J
//! Biometeorol 56: 481–494. Its 210 coefficients are those of the official
//! UTCI_a002 Fortran code from utci.org, which Ladybug Tools and pythermalcomfort
//! also use; they were copied mechanically from Ladybug's `utci.py`, not typed.
//! The tests check the result against pythermalcomfort's published validation table.

/// The ten stress categories of the UTCI assessment scale (Błażejczyk et al.
/// 2013, "An introduction to the Universal Thermal Climate Index", Geographia
/// Polonica 86: 5–10), coldest first, and the UTCI edges between them in °C.
/// Every category list the analysis uses lives here.
pub const STRESS_LABELS: [&str; 10] = [
    "Extreme cold stress",
    "Very strong cold stress",
    "Strong cold stress",
    "Moderate cold stress",
    "Slight cold stress",
    "No thermal stress",
    "Moderate heat stress",
    "Strong heat stress",
    "Very strong heat stress",
    "Extreme heat stress",
];
pub const STRESS_EDGES_C: [f64; 9] = [-40.0, -27.0, -13.0, 0.0, 9.0, 26.0, 32.0, 38.0, 46.0];
/// The index of "No thermal stress" in `STRESS_LABELS`.
pub const NO_STRESS: usize = 5;

/// The ranges the polynomial was fitted over (Bröde et al. 2012, table 1).
/// Wind is measured 10 m above the ground.
const AIR_MIN_C: f64 = -50.0;
const AIR_MAX_C: f64 = 50.0;
const RADIANT_DIFFERENCE_MIN_K: f64 = -30.0;
const RADIANT_DIFFERENCE_MAX_K: f64 = 70.0;
const WIND_MIN_M_S: f64 = 0.5;
const WIND_MAX_M_S: f64 = 17.0;

const KELVIN: f64 = 273.15;
const HPA_PER_KPA: f64 = 10.0;

/// UTCI in °C. Wind outside 0.5 to 17 m/s is clamped to that range, as the
/// reference code does; `is_in_range` says whether the temperatures are.
pub fn utci(air_c: f64, radiant_c: f64, wind_m_s: f64, humidity_pct: f64) -> f64 {
    let wind = wind_m_s.clamp(WIND_MIN_M_S, WIND_MAX_M_S);
    let vapour_kpa = saturation_vapour_pressure_hpa(air_c) * humidity_pct / 100.0 / HPA_PER_KPA;
    polynomial(air_c, radiant_c - air_c, wind, vapour_kpa)
}

/// Whether air and mean radiant temperature lie inside the polynomial's range.
pub fn is_in_range(air_c: f64, radiant_c: f64) -> bool {
    let difference = radiant_c - air_c;
    (AIR_MIN_C..=AIR_MAX_C).contains(&air_c)
        && (RADIANT_DIFFERENCE_MIN_K..=RADIANT_DIFFERENCE_MAX_K).contains(&difference)
}

/// The index into `STRESS_LABELS`. An edge belongs to the colder category,
/// as in pythermalcomfort: 26.0 is still no thermal stress.
pub fn stress_category(utci_c: f64) -> usize {
    STRESS_EDGES_C.iter().filter(|&&edge| utci_c > edge).count()
}

/// Saturation vapour pressure over water in hPa, by the Hardy (1998) ITS-90
/// formula the UTCI reference code uses.
fn saturation_vapour_pressure_hpa(air_c: f64) -> f64 {
    const G: [f64; 7] = [
        -2836.5744,
        -6028.076559,
        19.54263612,
        -0.02737830188,
        0.000016261698,
        7.0229056e-10,
        -1.8680009e-13,
    ];
    let kelvin = air_c + KELVIN;
    let mut exponent = 2.7150305 * kelvin.ln();
    for (index, coefficient) in G.iter().enumerate() {
        exponent += coefficient * kelvin.powi(index as i32 - 2);
    }

    exponent.exp() * 0.01
}

/// The sixth-order polynomial in air temperature `ta` (°C), mean radiant minus
/// air temperature `d` (K), wind `va` (m/s at 10 m) and vapour pressure `pa` (kPa).
#[allow(clippy::excessive_precision)]
fn polynomial(ta: f64, d: f64, va: f64, pa: f64) -> f64 {
    let (ta2, ta3, ta4, ta5, ta6) = (ta.powi(2), ta.powi(3), ta.powi(4), ta.powi(5), ta.powi(6));
    let (va2, va3, va4, va5, va6) = (va.powi(2), va.powi(3), va.powi(4), va.powi(5), va.powi(6));
    let (d2, d3, d4, d5, d6) = (d.powi(2), d.powi(3), d.powi(4), d.powi(5), d.powi(6));
    let (pa2, pa3, pa4, pa5, pa6) = (pa.powi(2), pa.powi(3), pa.powi(4), pa.powi(5), pa.powi(6));

    ta + 0.607562052 - 0.0227712343 * ta + 8.06470249e-4 * ta2
        - 1.54271372e-4 * ta3
        - 3.24651735e-6 * ta4
        + 7.32602852e-8 * ta5
        + 1.35959073e-9 * ta6
        - 2.25836520 * va
        + 0.0880326035 * ta * va
        + 0.00216844454 * ta2 * va
        - 1.53347087e-5 * ta3 * va
        - 5.72983704e-7 * ta4 * va
        - 2.55090145e-9 * ta5 * va
        - 0.751269505 * va2
        - 0.00408350271 * ta * va2
        - 5.21670675e-5 * ta2 * va2
        + 1.94544667e-6 * ta3 * va2
        + 1.14099531e-8 * ta4 * va2
        + 0.158137256 * va3
        - 6.57263143e-5 * ta * va3
        + 2.22697524e-7 * ta2 * va3
        - 4.16117031e-8 * ta3 * va3
        - 0.0127762753 * va4
        + 9.66891875e-6 * ta * va4
        + 2.52785852e-9 * ta2 * va4
        + 4.56306672e-4 * va5
        - 1.74202546e-7 * ta * va5
        - 5.91491269e-6 * va6
        + 0.398374029 * d
        + 1.83945314e-4 * ta * d
        - 1.73754510e-4 * ta2 * d
        - 7.60781159e-7 * ta3 * d
        + 3.77830287e-8 * ta4 * d
        + 5.43079673e-10 * ta5 * d
        - 0.0200518269 * va * d
        + 8.92859837e-4 * ta * va * d
        + 3.45433048e-6 * ta2 * va * d
        - 3.77925774e-7 * ta3 * va * d
        - 1.69699377e-9 * ta4 * va * d
        + 1.69992415e-4 * va2 * d
        - 4.99204314e-5 * ta * va2 * d
        + 2.47417178e-7 * ta2 * va2 * d
        + 1.07596466e-8 * ta3 * va2 * d
        + 8.49242932e-5 * va3 * d
        + 1.35191328e-6 * ta * va3 * d
        - 6.21531254e-9 * ta2 * va3 * d
        - 4.99410301e-6 * va4 * d
        - 1.89489258e-8 * ta * va4 * d
        + 8.15300114e-8 * va5 * d
        + 7.55043090e-4 * d2
        - 5.65095215e-5 * ta * d2
        - 4.52166564e-7 * ta2 * d2
        + 2.46688878e-8 * ta3 * d2
        + 2.42674348e-10 * ta4 * d2
        + 1.54547250e-4 * va * d2
        + 5.24110970e-6 * ta * va * d2
        - 8.75874982e-8 * ta2 * va * d2
        - 1.50743064e-9 * ta3 * va * d2
        - 1.56236307e-5 * va2 * d2
        - 1.33895614e-7 * ta * va2 * d2
        + 2.49709824e-9 * ta2 * va2 * d2
        + 6.51711721e-7 * va3 * d2
        + 1.94960053e-9 * ta * va3 * d2
        - 1.00361113e-8 * va4 * d2
        - 1.21206673e-5 * d3
        - 2.18203660e-7 * ta * d3
        + 7.51269482e-9 * ta2 * d3
        + 9.79063848e-11 * ta3 * d3
        + 1.25006734e-6 * va * d3
        - 1.81584736e-9 * ta * va * d3
        - 3.52197671e-10 * ta2 * va * d3
        - 3.36514630e-8 * va2 * d3
        + 1.35908359e-10 * ta * va2 * d3
        + 4.17032620e-10 * va3 * d3
        - 1.30369025e-9 * d4
        + 4.13908461e-10 * ta * d4
        + 9.22652254e-12 * ta2 * d4
        - 5.08220384e-9 * va * d4
        - 2.24730961e-11 * ta * va * d4
        + 1.17139133e-10 * va2 * d4
        + 6.62154879e-10 * d5
        + 4.03863260e-13 * ta * d5
        + 1.95087203e-12 * va * d5
        - 4.73602469e-12 * d6
        + 5.12733497 * pa
        - 0.312788561 * ta * pa
        - 0.0196701861 * ta2 * pa
        + 9.99690870e-4 * ta3 * pa
        + 9.51738512e-6 * ta4 * pa
        - 4.66426341e-7 * ta5 * pa
        + 0.548050612 * va * pa
        - 0.00330552823 * ta * va * pa
        - 0.00164119440 * ta2 * va * pa
        - 5.16670694e-6 * ta3 * va * pa
        + 9.52692432e-7 * ta4 * va * pa
        - 0.0429223622 * va2 * pa
        + 0.00500845667 * ta * va2 * pa
        + 1.00601257e-6 * ta2 * va2 * pa
        - 1.81748644e-6 * ta3 * va2 * pa
        - 1.25813502e-3 * va3 * pa
        - 1.79330391e-4 * ta * va3 * pa
        + 2.34994441e-6 * ta2 * va3 * pa
        + 1.29735808e-4 * va4 * pa
        + 1.29064870e-6 * ta * va4 * pa
        - 2.28558686e-6 * va5 * pa
        - 0.0369476348 * d * pa
        + 0.00162325322 * ta * d * pa
        - 3.14279680e-5 * ta2 * d * pa
        + 2.59835559e-6 * ta3 * d * pa
        - 4.77136523e-8 * ta4 * d * pa
        + 8.64203390e-3 * va * d * pa
        - 6.87405181e-4 * ta * va * d * pa
        - 9.13863872e-6 * ta2 * va * d * pa
        + 5.15916806e-7 * ta3 * va * d * pa
        - 3.59217476e-5 * va2 * d * pa
        + 3.28696511e-5 * ta * va2 * d * pa
        - 7.10542454e-7 * ta2 * va2 * d * pa
        - 1.24382300e-5 * va3 * d * pa
        - 7.38584400e-9 * ta * va3 * d * pa
        + 2.20609296e-7 * va4 * d * pa
        - 7.32469180e-4 * d2 * pa
        - 1.87381964e-5 * ta * d2 * pa
        + 4.80925239e-6 * ta2 * d2 * pa
        - 8.75492040e-8 * ta3 * d2 * pa
        + 2.77862930e-5 * va * d2 * pa
        - 5.06004592e-6 * ta * va * d2 * pa
        + 1.14325367e-7 * ta2 * va * d2 * pa
        + 2.53016723e-6 * va2 * d2 * pa
        - 1.72857035e-8 * ta * va2 * d2 * pa
        - 3.95079398e-8 * va3 * d2 * pa
        - 3.59413173e-7 * d3 * pa
        + 7.04388046e-7 * ta * d3 * pa
        - 1.89309167e-8 * ta2 * d3 * pa
        - 4.79768731e-7 * va * d3 * pa
        + 7.96079978e-9 * ta * va * d3 * pa
        + 1.62897058e-9 * va2 * d3 * pa
        + 3.94367674e-8 * d4 * pa
        - 1.18566247e-9 * ta * d4 * pa
        + 3.34678041e-10 * va * d4 * pa
        - 1.15606447e-10 * d5 * pa
        - 2.80626406 * pa2
        + 0.548712484 * ta * pa2
        - 0.00399428410 * ta2 * pa2
        - 9.54009191e-4 * ta3 * pa2
        + 1.93090978e-5 * ta4 * pa2
        - 0.308806365 * va * pa2
        + 0.0116952364 * ta * va * pa2
        + 4.95271903e-4 * ta2 * va * pa2
        - 1.90710882e-5 * ta3 * va * pa2
        + 0.00210787756 * va2 * pa2
        - 6.98445738e-4 * ta * va2 * pa2
        + 2.30109073e-5 * ta2 * va2 * pa2
        + 4.17856590e-4 * va3 * pa2
        - 1.27043871e-5 * ta * va3 * pa2
        - 3.04620472e-6 * va4 * pa2
        + 0.0514507424 * d * pa2
        - 0.00432510997 * ta * d * pa2
        + 8.99281156e-5 * ta2 * d * pa2
        - 7.14663943e-7 * ta3 * d * pa2
        - 2.66016305e-4 * va * d * pa2
        + 2.63789586e-4 * ta * va * d * pa2
        - 7.01199003e-6 * ta2 * va * d * pa2
        - 1.06823306e-4 * va2 * d * pa2
        + 3.61341136e-6 * ta * va2 * d * pa2
        + 2.29748967e-7 * va3 * d * pa2
        + 3.04788893e-4 * d2 * pa2
        - 6.42070836e-5 * ta * d2 * pa2
        + 1.16257971e-6 * ta2 * d2 * pa2
        + 7.68023384e-6 * va * d2 * pa2
        - 5.47446896e-7 * ta * va * d2 * pa2
        - 3.59937910e-8 * va2 * d2 * pa2
        - 4.36497725e-6 * d3 * pa2
        + 1.68737969e-7 * ta * d3 * pa2
        + 2.67489271e-8 * va * d3 * pa2
        + 3.23926897e-9 * d4 * pa2
        - 0.0353874123 * pa3
        - 0.221201190 * ta * pa3
        + 0.0155126038 * ta2 * pa3
        - 2.63917279e-4 * ta3 * pa3
        + 0.0453433455 * va * pa3
        - 0.00432943862 * ta * va * pa3
        + 1.45389826e-4 * ta2 * va * pa3
        + 2.17508610e-4 * va2 * pa3
        - 6.66724702e-5 * ta * va2 * pa3
        + 3.33217140e-5 * va3 * pa3
        - 0.00226921615 * d * pa3
        + 3.80261982e-4 * ta * d * pa3
        - 5.45314314e-9 * ta2 * d * pa3
        - 7.96355448e-4 * va * d * pa3
        + 2.53458034e-5 * ta * va * d * pa3
        - 6.31223658e-6 * va2 * d * pa3
        + 3.02122035e-4 * d2 * pa3
        - 4.77403547e-6 * ta * d2 * pa3
        + 1.73825715e-6 * va * d2 * pa3
        - 4.09087898e-7 * d3 * pa3
        + 0.614155345 * pa4
        - 0.0616755931 * ta * pa4
        + 0.00133374846 * ta2 * pa4
        + 0.00355375387 * va * pa4
        - 5.13027851e-4 * ta * va * pa4
        + 1.02449757e-4 * va2 * pa4
        - 0.00148526421 * d * pa4
        - 4.11469183e-5 * ta * d * pa4
        - 6.80434415e-6 * va * d * pa4
        - 9.77675906e-6 * d2 * pa4
        + 0.0882773108 * pa5
        - 0.00301859306 * ta * pa5
        + 0.00104452989 * va * pa5
        + 2.47090539e-4 * d * pa5
        + 0.00148348065 * pa6
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The validation table pythermalcomfort, jsthermalcomfort and the R
    /// package check their UTCI against: ts_utci.json, version 1.0.0, in the
    /// FedericoTartarini/validation-data-comfort-models repository. Each row is
    /// (air °C, mean radiant °C, humidity %, wind m/s at 10 m, UTCI °C), and the
    /// table's own tolerance is 0.1 K.
    const VALIDATION_TABLE: [(f64, f64, f64, f64, f64); 9] = [
        (25.0, 27.0, 50.0, 1.0, 25.2),
        (19.0, 24.0, 50.0, 1.0, 20.0),
        (19.0, 14.0, 50.0, 1.0, 16.8),
        (27.0, 22.0, 50.0, 1.0, 25.5),
        (27.0, 22.0, 50.0, 10.0, 20.0),
        (27.0, 22.0, 50.0, 16.0, 15.8),
        (30.0, 27.0, 50.0, 1.0, 29.6),
        (9.0, 9.0, 50.0, 1.0, 8.7),
        (25.0, 25.0, 50.0, 1.0, 24.6),
    ];
    const VALIDATION_TOLERANCE_K: f64 = 0.1;

    #[test]
    fn utci_matches_the_pythermalcomfort_validation_table() {
        for (air, radiant, humidity, wind, expected) in VALIDATION_TABLE {
            let found = utci(air, radiant, wind, humidity);
            assert!(
                (found - expected).abs() <= VALIDATION_TOLERANCE_K,
                "UTCI({air}, {radiant}, {wind}, {humidity}) = {found:.3}, expected {expected}"
            );
        }
    }

    #[test]
    fn utci_matches_ladybug_tools_unrounded() {
        // ladybug-comfort, tests/utci_test.py, test_utci. Ladybug runs the same
        // polynomial, so the values agree far closer than its 1 % tolerance.
        assert!((utci(20.0, 20.0, 3.0, 50.0) - 16.24224).abs() < 1e-3);
        assert!((utci(30.0, 30.0, 0.5, 90.0) - 35.511294).abs() < 1e-3);
    }

    #[test]
    fn wind_outside_the_fitted_range_is_clamped() {
        assert_eq!(utci(10.0, 10.0, 0.0, 70.0), utci(10.0, 10.0, 0.5, 70.0));
        assert_eq!(utci(10.0, 10.0, 30.0, 70.0), utci(10.0, 10.0, 17.0, 70.0));
    }

    #[test]
    fn stress_categories_follow_the_assessment_scale() {
        assert_eq!(STRESS_LABELS[stress_category(-41.0)], "Extreme cold stress");
        assert_eq!(STRESS_LABELS[stress_category(-5.0)], "Moderate cold stress");
        assert_eq!(STRESS_LABELS[stress_category(26.0)], "No thermal stress");
        assert_eq!(STRESS_LABELS[stress_category(26.1)], "Moderate heat stress");
        assert_eq!(STRESS_LABELS[stress_category(47.0)], "Extreme heat stress");
        assert_eq!(STRESS_LABELS[NO_STRESS], "No thermal stress");
    }

    #[test]
    fn radiant_temperature_far_above_the_air_is_outside_the_polynomial() {
        assert!(is_in_range(20.0, 60.0));
        assert!(!is_in_range(20.0, 95.0));
        assert!(!is_in_range(55.0, 55.0));
    }
}
