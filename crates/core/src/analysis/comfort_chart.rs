//! The outdoor comfort chart: for each month, the share of daytime hours in
//! each UTCI stress category, stacked from cold at the bottom to hot at the top.

use super::charts::{axes, LEGEND_GAP_MM, STACK_BAR_SHARE};
use super::climate::Climate;
use super::comfort::Exposure;
use super::figure::{figure, legend, rect, text, Figure, Paint, INSET_MM, LABEL_MM};
use super::utci::{NO_STRESS, STRESS_EDGES_C, STRESS_LABELS};
use crate::render::mix;

/// Where the chart's title sits above the plot.
const TITLE_BASELINE_MM: f64 = INSET_MM + LABEL_MM;

pub fn comfort(climate: &Climate, exposure: Exposure, paint: &Paint) -> Figure {
    let axes = axes(0.0, 100.0);
    let mut out = String::new();
    text(
        &mut out,
        axes.left,
        TITLE_BASELINE_MM,
        LABEL_MM,
        600,
        &paint.ink,
        "start",
        &paint.label(exposure.title()),
    );
    let bottom = axes.draw(&mut out, &[0.0, 50.0, 100.0], " %", paint);
    let colours = stress_colours(paint);

    let width = axes.column_width() * STACK_BAR_SHARE;
    for (index, month) in climate.months.iter().enumerate() {
        let hours = exposure.hours(month);
        let total: u32 = hours.iter().sum();
        let mut cumulative = 0.0;
        for (category, count) in hours.iter().enumerate() {
            let share = *count as f64 / total.max(1) as f64 * 100.0;
            let top = axes.y(cumulative + share);
            rect(
                &mut out,
                axes.x(index) - width / 2.0,
                top,
                width,
                axes.y(cumulative) - top,
                &colours[category],
            );
            cumulative += share;
        }
    }

    // The two charts sit one above the other, so only the lower one carries
    // the legend. Only the categories the site has: a temperate year never
    // reaches the extremes.
    if exposure == Exposure::Sun {
        return figure("comfort-sun", bottom + INSET_MM, out, paint);
    }
    let items: Vec<(String, String)> = occurring_categories(climate)
        .into_iter()
        .map(|category| (legend_label(category), colours[category].clone()))
        .collect();
    let height = legend(&mut out, axes.left, bottom + LEGEND_GAP_MM, &items, paint) + INSET_MM;
    figure("comfort-shade", height, out, paint)
}

/// Cold to hot: greys in the theme's own tint for the five cold categories,
/// darker the colder, then green for no thermal stress, then the ramp and the
/// accent for heat, darkening towards ink, so the comfortable middle stands out.
pub fn stress_colours(paint: &Paint) -> Vec<String> {
    vec![
        paint.ink_at(0.95),
        paint.ink_at(0.8),
        paint.ink_at(0.64),
        paint.ink_at(0.46),
        paint.ink_at(0.26),
        // Green pulled a fifth towards ink, so a dark theme's deep green still reads.
        paint.visible(&mix(&paint.green, &paint.ink, 0.2), &paint.ramp_at(0.2)),
        paint.ramp_at(0.45),
        paint.accent.clone(),
        mix(&paint.accent, &paint.ink, 0.35),
        mix(&paint.accent, &paint.ink, 0.6),
    ]
}

/// The categories with at least one hour, in the sun or in the shade.
fn occurring_categories(climate: &Climate) -> Vec<usize> {
    (0..STRESS_LABELS.len())
        .filter(|&category| {
            climate.months.iter().any(|month| {
                month.comfort_sun_hours[category] + month.comfort_shade_hours[category] > 0
            })
        })
        .collect()
}

/// "Moderate cold −13 to 0", "No thermal stress 9 to 26", "Strong heat 32 to 38".
fn legend_label(category: usize) -> String {
    let name = if category == NO_STRESS {
        STRESS_LABELS[category]
    } else {
        STRESS_LABELS[category].trim_end_matches(" stress")
    };
    let edges = STRESS_EDGES_C;
    let range = if category == 0 {
        format!("below {}", signed(edges[0]))
    } else if category == edges.len() {
        format!("above {}", signed(edges[edges.len() - 1]))
    } else {
        format!(
            "{} to {}",
            signed(edges[category - 1]),
            signed(edges[category])
        )
    };

    format!("{name} {range}")
}

/// -13.0 -> "−13", with a true minus sign.
fn signed(value: f64) -> String {
    if value < 0.0 {
        format!("\u{2212}{:.0}", -value)
    } else {
        format!("{value:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_labels_carry_their_utci_range() {
        assert_eq!(legend_label(0), "Extreme cold below \u{2212}40");
        assert_eq!(legend_label(3), "Moderate cold \u{2212}13 to 0");
        assert_eq!(legend_label(NO_STRESS), "No thermal stress 9 to 26");
        assert_eq!(legend_label(9), "Extreme heat above 46");
    }
}
