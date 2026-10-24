//! Mean radiant temperature outdoors: the air temperature, raised by the sun
//! a person absorbs and lowered by the heat they lose to a cold sky.
//!
//! The sun part is the SolarCal model of ASHRAE Standard 55 Normative Appendix C
//! (Arens et al. 2015, "Modeling the comfort effects of short-wave solar
//! radiation indoors", Building and Environment 88: 3–9), with the open sky
//! in place of a window. The sky part takes the sky temperature from the
//! long-wave radiation the weather data measures. Both follow
//! `outdoor_sky_heat_exch` in Ladybug Tools' `ladybug_comfort/solarcal.py`,
//! which the tests compare against.
//!
//! The person is assumed to stand on open ground with the whole sky in view,
//! and the ground and walls around them are taken to be at air temperature,
//! as Ladybug does when surface temperatures are unknown. "In the shade" blocks
//! the direct beam only; diffuse sky light, ground reflection and the cold
//! sky still reach the person.

/// A standing person: the share of the body surface that exchanges radiation
/// with the surroundings (ASHRAE 55 Appendix C).
const EFFECTIVE_AREA_SHARE: f64 = 0.725;
/// The radiative heat transfer coefficient of ASHRAE 55 Appendix C, W/m²K.
const RADIATIVE_COEFFICIENT: f64 = 6.012;
/// ASHRAE 55 Appendix C's default short-wave absorptivity of skin and
/// clothing, and the usual long-wave emissivity of a clothed body.
const BODY_ABSORPTIVITY: f64 = 0.7;
const BODY_EMISSIVITY: f64 = 0.95;
/// Grass or dry bare soil, Ladybug's default for outdoor ground.
const GROUND_REFLECTANCE: f64 = 0.25;
/// The sun's horizontal angle from the person's face: 135° is sun over the
/// shoulder from behind, Ladybug's default, as people turn from the glare.
const SUN_FROM_FACE_DEG: f64 = 135.0;
/// Below this sun altitude Ladybug counts no short-wave gain.
const MIN_SUN_ALTITUDE_DEG: f64 = 2.0;
/// The whole sky in view, as on open ground.
const SKY_VIEW: f64 = 1.0;
/// The Stefan–Boltzmann constant as Ladybug and EnergyPlus write it, W/m²K⁴.
const STEFAN_BOLTZMANN: f64 = 5.6697e-8;
const KELVIN: f64 = 273.15;

/// ASHRAE 55 Appendix C's projected area factors f_p for a standing person:
/// one row per 15° of sun angle from the face (0° to 180°), one column per
/// 15° of sun altitude (0° to 90°). The same table as pythermalcomfort's
/// `solar_gain`.
const PROJECTED_AREA: [[f64; 7]; 13] = [
    [0.35, 0.35, 0.314, 0.258, 0.206, 0.144, 0.082],
    [0.342, 0.342, 0.31, 0.252, 0.2, 0.14, 0.082],
    [0.33, 0.33, 0.3, 0.244, 0.19, 0.132, 0.082],
    [0.31, 0.31, 0.275, 0.228, 0.175, 0.124, 0.082],
    [0.283, 0.283, 0.251, 0.208, 0.16, 0.114, 0.082],
    [0.252, 0.252, 0.228, 0.188, 0.15, 0.108, 0.082],
    [0.23, 0.23, 0.214, 0.18, 0.148, 0.108, 0.082],
    [0.242, 0.242, 0.222, 0.18, 0.153, 0.112, 0.082],
    [0.274, 0.274, 0.245, 0.203, 0.165, 0.116, 0.082],
    [0.304, 0.304, 0.27, 0.22, 0.174, 0.121, 0.082],
    [0.328, 0.328, 0.29, 0.234, 0.183, 0.125, 0.082],
    [0.344, 0.344, 0.304, 0.244, 0.19, 0.128, 0.082],
    [0.347, 0.347, 0.308, 0.246, 0.191, 0.128, 0.082],
];
const TABLE_STEP_DEG: f64 = 15.0;

/// One hour of weather as the radiation sees it.
#[derive(Debug, Clone, Copy)]
pub struct Sky {
    pub air_c: f64,
    /// Long-wave radiation from the sky onto a horizontal plane, W/m².
    pub infrared_w_m2: f64,
    pub diffuse_w_m2: f64,
    pub direct_normal_w_m2: f64,
    pub sun_altitude_deg: f64,
}

/// Mean radiant temperature in °C for a person in the sun, or in the shade
/// of something that blocks only the direct beam.
pub fn radiant_temperature(sky: &Sky, is_in_sun: bool) -> f64 {
    sky.air_c + sun_gain_k(sky, is_in_sun) + sky_loss_k(sky)
}

/// How much the absorbed sun raises the mean radiant temperature, in K.
fn sun_gain_k(sky: &Sky, is_in_sun: bool) -> f64 {
    if sky.sun_altitude_deg < MIN_SUN_ALTITUDE_DEG {
        return 0.0;
    }

    let altitude = sky.sun_altitude_deg.min(90.0);
    let global = sky.diffuse_w_m2 + sky.direct_normal_w_m2 * altitude.to_radians().sin();
    let exposed = if is_in_sun { 1.0 } else { 0.0 };
    let direct = EFFECTIVE_AREA_SHARE
        * projected_area(altitude, SUN_FROM_FACE_DEG)
        * exposed
        * sky.direct_normal_w_m2;
    let diffuse = 0.5 * SKY_VIEW * EFFECTIVE_AREA_SHARE * sky.diffuse_w_m2;
    let reflected = 0.5 * SKY_VIEW * EFFECTIVE_AREA_SHARE * global * GROUND_REFLECTANCE;
    let effective_radiant_field =
        (direct + diffuse + reflected) * BODY_ABSORPTIVITY / BODY_EMISSIVITY;

    effective_radiant_field / (EFFECTIVE_AREA_SHARE * RADIATIVE_COEFFICIENT)
}

/// How much the sky, colder than the air, lowers it, in K: half the body
/// faces the sky and half the ground.
fn sky_loss_k(sky: &Sky) -> f64 {
    let sky_c = (sky.infrared_w_m2 / (BODY_EMISSIVITY * STEFAN_BOLTZMANN)).powf(0.25) - KELVIN;
    0.5 * SKY_VIEW * (sky_c - sky.air_c)
}

/// f_p from the table, interpolated between its rows and columns.
fn projected_area(altitude_deg: f64, from_face_deg: f64) -> f64 {
    let last_column = PROJECTED_AREA[0].len() - 1;
    let last_row = PROJECTED_AREA.len() - 1;
    let column = (altitude_deg / TABLE_STEP_DEG).clamp(0.0, last_column as f64);
    let row = (from_face_deg / TABLE_STEP_DEG).clamp(0.0, last_row as f64);
    let left = (column.floor() as usize).min(last_column - 1);
    let top = (row.floor() as usize).min(last_row - 1);
    let across = column - left as f64;
    let down = row - top as f64;

    let upper = PROJECTED_AREA[top][left] * (1.0 - across) + PROJECTED_AREA[top][left + 1] * across;
    let lower =
        PROJECTED_AREA[top + 1][left] * (1.0 - across) + PROJECTED_AREA[top + 1][left + 1] * across;
    upper * (1.0 - down) + lower * down
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sunny_afternoon_matches_ladybug() {
        // ladybug-comfort, tests/solarcal_test.py, test_outdoor_sky_heat_exch:
        // outdoor_sky_heat_exch(22, 380, 200, 380, 45) gives an MRT of 48.9847 °C.
        // Ladybug reads f_p from the original SolarCal splines, this module
        // from the ASHRAE 55 table, so the two differ by a few hundredths of a degree.
        let sky = Sky {
            air_c: 22.0,
            infrared_w_m2: 380.0,
            diffuse_w_m2: 200.0,
            direct_normal_w_m2: 380.0,
            sun_altitude_deg: 45.0,
        };
        let found = radiant_temperature(&sky, true);
        assert!((found - 48.9847).abs() < 0.1, "{found}");
        // The same test's parts: the sky alone lowers it by 2.6661 K.
        assert!((sky_loss_k(&sky) + 2.6661).abs() < 1e-3);
    }

    #[test]
    fn a_clear_night_matches_ladybug() {
        // ladybug-comfort, tests/solarcal_test.py: outdoor_sky_heat_exch(18, 330, 0, 0, 0)
        // gives an MRT of 12.3120 °C.
        let sky = Sky {
            air_c: 18.0,
            infrared_w_m2: 330.0,
            diffuse_w_m2: 0.0,
            direct_normal_w_m2: 0.0,
            sun_altitude_deg: -20.0,
        };
        assert!((radiant_temperature(&sky, true) - 12.3120).abs() < 1e-3);
    }

    #[test]
    fn shade_takes_away_the_direct_beam_only() {
        let sky = Sky {
            air_c: 22.0,
            infrared_w_m2: 380.0,
            diffuse_w_m2: 200.0,
            direct_normal_w_m2: 380.0,
            sun_altitude_deg: 45.0,
        };
        let sun = radiant_temperature(&sky, true);
        let shade = radiant_temperature(&sky, false);
        assert!(sun - shade > 5.0, "{sun} {shade}");
        assert!(shade > 22.0 + sky_loss_k(&sky), "diffuse light still warms");
    }

    #[test]
    fn the_projected_area_table_is_read_at_its_corners_and_between() {
        assert_eq!(projected_area(0.0, 0.0), 0.35);
        assert_eq!(projected_area(90.0, 180.0), 0.082);
        assert_eq!(projected_area(45.0, 135.0), 0.22);
        assert!((projected_area(52.5, 135.0) - (0.22 + 0.174) / 2.0).abs() < 1e-12);
    }
}
