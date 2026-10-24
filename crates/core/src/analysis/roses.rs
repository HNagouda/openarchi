//! Wind roses by season and the radiation rose, drawn in the theme.
//!
//! Both are compass charts with north up: a wedge points the way the wind
//! comes from, or the way the sun shines from, and its length is the share
//! of hours or the energy that arrives from there.

use std::f64::consts::TAU;
use std::fmt::Write;

use super::climate::{Climate, SECTORS, WIND_LABELS};
use super::figure::{
    figure, legend, text, Figure, Paint, HAIRLINE_MM, INSET_MM, LABEL_MM, SMALL_MM, WIDTH_MM,
};
use super::notes::winter_half_text;

const WIND_RADIUS_MM: f64 = 12.0;
/// The calm circle in the middle of a wind rose.
const CALM_RADIUS_MM: f64 = 1.4;
const WIND_RING_STEP: f64 = 0.05;
const SUN_RADIUS_MM: f64 = 22.0;
const SUN_RING_STEP_KWH: f64 = 50.0;
/// A wedge fills this share of its sector, leaving a gap to the next.
const WEDGE_SHARE: f64 = 0.82;
/// From the rose's edge to the centre of a compass letter.
const COMPASS_GAP_MM: f64 = 2.6;
const SEASON_TITLE_MM: f64 = 4.0;
const ROSE_ROW_GAP_MM: f64 = 4.0;
const LEGEND_GAP_MM: f64 = 3.0;

/// The ramp positions of the wind speed bands, light to strong. The light
/// end is skipped because in some themes it is close to the paper.
const WIND_BAND_RAMP: [f64; 4] = [0.3, 0.55, 0.8, 1.0];

const COMPASS: [(&str, f64); 4] = [("N", 0.0), ("E", 90.0), ("S", 180.0), ("W", 270.0)];

pub fn wind(climate: &Climate, paint: &Paint) -> Figure {
    let largest = climate
        .wind
        .iter()
        .flat_map(|rose| rose.sectors.iter().map(|sector| sector.total()))
        .fold(0.0, f64::max);
    let outer = (largest / WIND_RING_STEP).ceil().max(1.0) * WIND_RING_STEP;
    let colours: Vec<String> = WIND_BAND_RAMP.iter().map(|&at| paint.ramp_at(at)).collect();

    let mut out = String::new();
    let column = (WIDTH_MM - 2.0 * INSET_MM) / 2.0;
    let row_height = SEASON_TITLE_MM + 2.0 * (WIND_RADIUS_MM + COMPASS_GAP_MM) + ROSE_ROW_GAP_MM;
    for (index, rose) in climate.wind.iter().enumerate() {
        let left = INSET_MM + (index % 2) as f64 * column;
        let top = INSET_MM + (index / 2) as f64 * row_height;
        let title = format!("{} · {}", paint.label(&rose.season), rose.months);
        text(
            &mut out,
            left + column / 2.0,
            top + SMALL_MM,
            SMALL_MM,
            600,
            &paint.ink,
            "middle",
            &title,
        );
        let centre = (
            left + column / 2.0,
            top + SEASON_TITLE_MM + COMPASS_GAP_MM + WIND_RADIUS_MM,
        );
        rings(
            &mut out,
            centre,
            WIND_RADIUS_MM,
            outer,
            WIND_RING_STEP,
            paint,
        );

        for (sector_index, sector) in rose.sectors.iter().enumerate() {
            let mut from = 0.0;
            for (band, share) in sector.shares.iter().enumerate() {
                let to = from + share;
                let inner = CALM_RADIUS_MM + from / outer * (WIND_RADIUS_MM - CALM_RADIUS_MM);
                let outer_radius = CALM_RADIUS_MM + to / outer * (WIND_RADIUS_MM - CALM_RADIUS_MM);
                wedge(
                    &mut out,
                    centre,
                    sector_index,
                    inner,
                    outer_radius,
                    &colours[band],
                );
                from = to;
            }
        }
        let _ = writeln!(
            out,
            r#"<circle cx="{:.2}" cy="{:.2}" r="{CALM_RADIUS_MM}" fill="{}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
            centre.0, centre.1, paint.paper, paint.secondary
        );
        compass(&mut out, centre, WIND_RADIUS_MM, paint);
        let calm = format!("calm {:.0} %", rose.calm_share * 100.0);
        text(
            &mut out,
            centre.0 + WIND_RADIUS_MM,
            centre.1 + WIND_RADIUS_MM + 1.0,
            SMALL_MM * 0.9,
            400,
            &paint.secondary,
            "end",
            &calm,
        );
    }

    let mut items: Vec<(String, String)> = WIND_LABELS
        .iter()
        .zip(&colours)
        .map(|(label, colour)| (label.to_string(), colour.clone()))
        .collect();
    items.push((
        format!("Rings every {:.0} % of hours", WIND_RING_STEP * 100.0),
        String::new(),
    ));
    let legend_top = INSET_MM + 2.0 * row_height - ROSE_ROW_GAP_MM + LEGEND_GAP_MM;
    let height = legend(&mut out, INSET_MM, legend_top, &items, paint) + INSET_MM;
    figure("wind-roses", height, out, paint)
}

pub fn sun(climate: &Climate, paint: &Paint) -> Figure {
    let largest = climate
        .sun
        .iter()
        .map(|sector| sector.winter_kwh_m2 + sector.summer_kwh_m2)
        .fold(0.0, f64::max);
    let outer = (largest / SUN_RING_STEP_KWH).ceil().max(1.0) * SUN_RING_STEP_KWH;
    let winter = paint.accent.clone();
    let summer = paint.ramp_at(0.35);

    let mut out = String::new();
    let centre = (
        WIDTH_MM / 2.0,
        INSET_MM + COMPASS_GAP_MM + SUN_RADIUS_MM + 1.0,
    );
    rings(
        &mut out,
        centre,
        SUN_RADIUS_MM,
        outer,
        SUN_RING_STEP_KWH,
        paint,
    );
    for (index, sector) in climate.sun.iter().enumerate() {
        let winter_radius = sector.winter_kwh_m2 / outer * SUN_RADIUS_MM;
        let year_radius = (sector.winter_kwh_m2 + sector.summer_kwh_m2) / outer * SUN_RADIUS_MM;
        wedge(&mut out, centre, index, 0.0, winter_radius, &winter);
        wedge(&mut out, centre, index, winter_radius, year_radius, &summer);
    }
    compass(&mut out, centre, SUN_RADIUS_MM, paint);
    let scale = format!("{:.0} kWh/m²", outer);
    text(
        &mut out,
        centre.0 + 1.0,
        centre.1 - SUN_RADIUS_MM + SMALL_MM + 0.6,
        SMALL_MM * 0.9,
        400,
        &paint.secondary,
        "start",
        &scale,
    );

    let items = [
        (format!("Direct sun {}", winter_half_text()), winter),
        ("Apr to Sep".to_string(), summer),
        (
            format!("Rings every {SUN_RING_STEP_KWH:.0} kWh/m²"),
            String::new(),
        ),
    ];
    let legend_top = centre.1 + SUN_RADIUS_MM + COMPASS_GAP_MM + LEGEND_GAP_MM;
    let height = legend(&mut out, INSET_MM, legend_top, &items, paint) + INSET_MM;
    figure("radiation-rose", height, out, paint)
}

/// Concentric rings every `step` up to `outer`, the outermost one at `radius`.
fn rings(out: &mut String, centre: (f64, f64), radius: f64, outer: f64, step: f64, paint: &Paint) {
    let count = (outer / step).round() as usize;
    for ring in 1..=count {
        let ring_radius = radius * ring as f64 / count as f64;
        let _ = writeln!(
            out,
            r#"<circle cx="{:.2}" cy="{:.2}" r="{ring_radius:.2}" fill="none" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
            centre.0, centre.1, paint.faint
        );
    }
    for (_, degrees) in COMPASS {
        let (x, y) = polar(centre, radius, degrees.to_radians());
        super::figure::line(out, centre, (x, y), &paint.faint, HAIRLINE_MM);
    }
}

fn compass(out: &mut String, centre: (f64, f64), radius: f64, paint: &Paint) {
    for (letter, degrees) in COMPASS {
        let (x, y) = polar(centre, radius + COMPASS_GAP_MM, degrees.to_radians());
        text(
            out,
            x,
            y + LABEL_MM * 0.35,
            SMALL_MM,
            600,
            &paint.secondary,
            "middle",
            letter,
        );
    }
}

/// A ring segment for sector `index` (0 is north) between two radii.
fn wedge(out: &mut String, centre: (f64, f64), index: usize, inner: f64, outer: f64, fill: &str) {
    if outer - inner < 0.01 {
        return;
    }
    let sector = TAU / SECTORS as f64;
    let middle = index as f64 * sector;
    let start = middle - sector * WEDGE_SHARE / 2.0;
    let end = middle + sector * WEDGE_SHARE / 2.0;
    let (outer_start, outer_end) = (polar(centre, outer, start), polar(centre, outer, end));
    let (inner_start, inner_end) = (polar(centre, inner, start), polar(centre, inner, end));
    let _ = writeln!(
        out,
        r#"<path d="M{:.2} {:.2}A{outer:.2} {outer:.2} 0 0 1 {:.2} {:.2}L{:.2} {:.2}A{inner:.2} {inner:.2} 0 0 0 {:.2} {:.2}Z" fill="{fill}"/>"#,
        outer_start.0,
        outer_start.1,
        outer_end.0,
        outer_end.1,
        inner_end.0,
        inner_end.1,
        inner_start.0,
        inner_start.1
    );
}

/// The point `radius` from `centre` at `angle` radians clockwise from north.
fn polar(centre: (f64, f64), radius: f64, angle: f64) -> (f64, f64) {
    (
        centre.0 + radius * angle.sin(),
        centre.1 - radius * angle.cos(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn east_lies_to_the_right_of_the_centre() {
        let (x, y) = polar((10.0, 10.0), 5.0, 90f64.to_radians());
        assert!((x - 15.0).abs() < 1e-9 && (y - 10.0).abs() < 1e-9);
    }
}
