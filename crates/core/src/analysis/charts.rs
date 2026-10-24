//! The month-by-month charts and the ground cover bar, drawn in the theme.
//!
//! Colours come from the ramp, ink and accent, which every theme keeps
//! distinct from its paper. Water and green are used only through
//! `Paint::visible`, because some themes paint them in the paper colour.

use std::fmt::Write;

use super::climate::Climate;
use super::figure::{
    figure, legend, line, percent, rect, text, ticks, Figure, MonthAxes, Paint, HAIRLINE_MM,
    INSET_MM, LINE_MM, SMALL_MM, WIDTH_MM,
};
use super::rain::Rain;
use crate::ratios::Group;
use crate::render::mix;

/// Room left of the plot for the value labels.
const AXIS_LABEL_MM: f64 = 9.0;
const PLOT_HEIGHT_MM: f64 = 34.0;
/// Space above the plot for the labels over the bars.
const HEADROOM_MM: f64 = 4.5;
pub(super) const LEGEND_GAP_MM: f64 = 3.2;

/// The temperatures the bar colours run between, light end to strong end:
/// a cold Dutch winter month to a warm summer one.
const COLD_MEAN_C: f64 = -2.0;
const WARM_MEAN_C: f64 = 20.0;
const TEMPERATURE_STEP_C: f64 = 5.0;
/// A daily range bar takes this share of its month's column.
const RANGE_BAR_SHARE: f64 = 0.4;
pub(super) const STACK_BAR_SHARE: f64 = 0.62;
const PAIR_BAR_SHARE: f64 = 0.36;

const HUMIDITY_FLOOR_PCT: f64 = 50.0;
const HUMIDITY_STEP_PCT: f64 = 10.0;
const HUMIDITY_FILL_OPACITY: f64 = 0.35;
const DOT_RADIUS_MM: f64 = 0.7;
const RADIATION_STEP_KWH: f64 = 50.0;
const RAIN_STEP_MM: f64 = 25.0;

const GROUND_BAR_MM: f64 = 5.0;

pub fn temperature(climate: &Climate, paint: &Paint) -> Figure {
    let low = climate
        .months
        .iter()
        .map(|month| month.mean_daily_min_c)
        .fold(f64::INFINITY, f64::min);
    let high = climate
        .months
        .iter()
        .map(|month| month.mean_daily_max_c)
        .fold(f64::NEG_INFINITY, f64::max);
    let axes = axes(
        (low / TEMPERATURE_STEP_C).floor() * TEMPERATURE_STEP_C,
        (high / TEMPERATURE_STEP_C).ceil() * TEMPERATURE_STEP_C,
    );
    let mut out = String::new();
    let bottom = axes.draw(
        &mut out,
        &ticks(axes.min, axes.max, TEMPERATURE_STEP_C),
        "°",
        paint,
    );
    if axes.min < 0.0 {
        line(
            &mut out,
            (axes.left, axes.y(0.0)),
            (axes.right, axes.y(0.0)),
            &paint.secondary,
            HAIRLINE_MM,
        );
    }

    let width = axes.column_width() * RANGE_BAR_SHARE;
    for (index, month) in climate.months.iter().enumerate() {
        let x = axes.x(index);
        let top = axes.y(month.mean_daily_max_c);
        let fill = paint.ramp_at((month.mean_c - COLD_MEAN_C) / (WARM_MEAN_C - COLD_MEAN_C));
        let _ = writeln!(
            out,
            r#"<rect x="{:.2}" y="{top:.2}" width="{width:.2}" height="{:.2}" rx="{:.2}" fill="{fill}"/>"#,
            x - width / 2.0,
            axes.y(month.mean_daily_min_c) - top,
            width / 2.0
        );
        let mean_y = axes.y(month.mean_c);
        line(
            &mut out,
            (x - width * 0.85, mean_y),
            (x + width * 0.85, mean_y),
            &paint.ink,
            LINE_MM,
        );
        let label = format!("{:.0}", month.mean_daily_max_c);
        text(
            &mut out,
            x,
            top - 1.0,
            SMALL_MM,
            400,
            &paint.secondary,
            "middle",
            &label,
        );
    }

    let items = [
        ("Mean daily low to high".to_string(), paint.ramp_at(0.5)),
        ("Monthly mean".to_string(), paint.ink.clone()),
    ];
    let height = legend(&mut out, axes.left, bottom + LEGEND_GAP_MM, &items, paint) + INSET_MM;
    figure("temperature", height, out, paint)
}

pub fn humidity(climate: &Climate, paint: &Paint) -> Figure {
    let axes = axes(HUMIDITY_FLOOR_PCT, 100.0);
    let mut out = String::new();
    let bottom = axes.draw(
        &mut out,
        &ticks(axes.min, axes.max, HUMIDITY_STEP_PCT),
        " %",
        paint,
    );

    let points: Vec<(f64, f64)> = climate
        .months
        .iter()
        .enumerate()
        .map(|(index, month)| (axes.x(index), axes.y(month.humidity_pct)))
        .collect();
    let path: Vec<String> = points
        .iter()
        .map(|(x, y)| format!("{x:.2} {y:.2}"))
        .collect();
    let fill = paint.visible(&paint.water, &paint.ramp_at(0.4));
    let _ = writeln!(
        out,
        r#"<path d="M{:.2} {:.2}L{}L{:.2} {:.2}Z" fill="{fill}" opacity="{HUMIDITY_FILL_OPACITY}"/>"#,
        points[0].0,
        axes.bottom,
        path.join("L"),
        points[points.len() - 1].0,
        axes.bottom
    );
    let _ = writeln!(
        out,
        r#"<path d="M{}" fill="none" stroke="{}" stroke-width="{LINE_MM}" stroke-linejoin="round"/>"#,
        path.join("L"),
        paint.ink
    );
    for (index, (x, y)) in points.iter().enumerate() {
        let _ = writeln!(
            out,
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="{DOT_RADIUS_MM}" fill="{}"/>"#,
            paint.ink
        );
        let label = format!("{:.0}", climate.months[index].humidity_pct);
        text(
            &mut out,
            *x,
            y - 1.6,
            SMALL_MM,
            400,
            &paint.secondary,
            "middle",
            &label,
        );
    }

    let items = [("Mean relative humidity".to_string(), paint.ink.clone())];
    let height = legend(&mut out, axes.left, bottom + LEGEND_GAP_MM, &items, paint) + INSET_MM;
    figure("humidity", height, out, paint)
}

pub fn radiation(climate: &Climate, paint: &Paint) -> Figure {
    let high = climate
        .months
        .iter()
        .map(|month| month.global_kwh_m2.max(month.direct_normal_kwh_m2))
        .fold(0.0, f64::max);
    let axes = axes(0.0, (high / RADIATION_STEP_KWH).ceil() * RADIATION_STEP_KWH);
    let mut out = String::new();
    let bottom = axes.draw(
        &mut out,
        &ticks(axes.min, axes.max, RADIATION_STEP_KWH),
        "",
        paint,
    );
    let global = paint.ramp_at(0.45);
    let direct = paint.ramp_at(0.95);

    let width = axes.column_width() * PAIR_BAR_SHARE;
    for (index, month) in climate.months.iter().enumerate() {
        let x = axes.x(index);
        for (offset, value, fill) in [
            (-width, month.global_kwh_m2, &global),
            (0.0, month.direct_normal_kwh_m2, &direct),
        ] {
            let top = axes.y(value);
            rect(&mut out, x + offset, top, width, axes.bottom - top, fill);
        }
    }

    let items = [
        ("Global horizontal, kWh/m²".to_string(), global),
        ("Direct normal, kWh/m²".to_string(), direct),
    ];
    let height = legend(&mut out, axes.left, bottom + LEGEND_GAP_MM, &items, paint) + INSET_MM;
    figure("radiation", height, out, paint)
}

pub fn rain(rain: &Rain, paint: &Paint) -> Figure {
    let high = rain
        .months
        .iter()
        .map(|month| month.mean_mm)
        .fold(0.0, f64::max);
    let axes = axes(0.0, (high / RAIN_STEP_MM).ceil() * RAIN_STEP_MM);
    let mut out = String::new();
    let month_labels_y = axes.draw(
        &mut out,
        &ticks(axes.min, axes.max, RAIN_STEP_MM),
        "",
        paint,
    );
    let fill = rain_colour(paint);

    let width = axes.column_width() * STACK_BAR_SHARE;
    let days_y = month_labels_y + SMALL_MM + 1.6;
    for (index, month) in rain.months.iter().enumerate() {
        let x = axes.x(index);
        let top = axes.y(month.mean_mm);
        rect(
            &mut out,
            x - width / 2.0,
            top,
            width,
            axes.bottom - top,
            &fill,
        );
        let label = format!("{:.0}", month.mean_mm);
        text(
            &mut out,
            x,
            top - 1.0,
            SMALL_MM,
            400,
            &paint.secondary,
            "middle",
            &label,
        );
        let days = format!("{:.0}", month.rain_days);
        text(
            &mut out, x, days_y, SMALL_MM, 600, &paint.ink, "middle", &days,
        );
    }
    text(
        &mut out,
        axes.left - 1.2,
        days_y,
        SMALL_MM,
        400,
        &paint.secondary,
        "end",
        "days",
    );

    let items = [
        ("Mean monthly total, mm".to_string(), fill),
        (
            "Rain days of 1 mm or more, in figures".to_string(),
            paint.ink.clone(),
        ),
    ];
    let height = legend(&mut out, axes.left, days_y + LEGEND_GAP_MM, &items, paint) + INSET_MM;
    figure("rain", height, out, paint)
}

/// The ground cover as one bar in the land use colours, labelled with shares.
pub fn ground(land_use: &Group, paint: &Paint) -> Figure {
    let mut out = String::new();
    let left = INSET_MM;
    let width = WIDTH_MM - 2.0 * INSET_MM;
    let top = INSET_MM;
    let mut x = left;
    for row in &land_use.rows {
        let span = row.share * width;
        rect(&mut out, x, top, span, GROUND_BAR_MM, &row.fill);
        x += span;
    }
    let _ = writeln!(
        out,
        r#"<rect x="{left}" y="{top}" width="{width:.2}" height="{GROUND_BAR_MM}" fill="none" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
        paint.faint
    );

    let items: Vec<(String, String)> = land_use
        .rows
        .iter()
        .map(|row| {
            (
                format!("{} {}", row.label, percent(row.share)),
                row.fill.clone(),
            )
        })
        .collect();
    let height = legend(
        &mut out,
        left,
        top + GROUND_BAR_MM + LEGEND_GAP_MM,
        &items,
        paint,
    ) + INSET_MM;
    figure("ground", height, out, paint)
}

/// Water pulled a quarter towards ink, so pale water still reads as a bar.
pub fn rain_colour(paint: &Paint) -> String {
    paint.visible(&mix(&paint.water, &paint.ink, 0.25), &paint.ramp_at(0.6))
}

/// The plot area every month chart shares, for values from `min` to `max`.
pub(super) fn axes(min: f64, max: f64) -> MonthAxes {
    MonthAxes {
        left: INSET_MM + AXIS_LABEL_MM,
        right: WIDTH_MM - INSET_MM,
        top: INSET_MM + HEADROOM_MM,
        bottom: INSET_MM + HEADROOM_MM + PLOT_HEIGHT_MM,
        min,
        max: if max > min { max } else { min + 1.0 },
    }
}
