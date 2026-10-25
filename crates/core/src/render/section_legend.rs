//! A section's legend: the diagram's building classes when it has them, then
//! what a section shows, and a note on what the drawing does not measure.

use super::context;
use super::legend::{self, Legend, Row, Swatch};
use super::massing;
use super::section::TITLE;
use super::section_draw;
use super::section_water;
use super::solid;
use super::style::{ResolvedLayer, Role};
use super::RenderInput;
use crate::scene::Feature;
use crate::section::profile::Profile;
use crate::section::water::Water;
use crate::section::SectionLine;

/// Legend swatches of what stands beyond show it halfway to the far edge.
const MIDDLE_DEPTH: f64 = 0.5;
const FLAT_NOTE: &str = "Ground drawn flat: this site was fetched before ground heights were kept. Fetch it again for the terrain.";

/// What the section drew, which is what its legend lists.
pub(super) struct Drawn<'a> {
    pub buildings: Option<&'a ResolvedLayer>,
    /// The site's buildings standing beyond the line.
    pub seen: &'a [&'a Feature],
    pub profile: &'a Profile,
    pub line: &'a SectionLine,
    pub has_neighbours: bool,
    pub has_trees: bool,
    pub tree_fill: &'a str,
    pub waters: &'a [Water],
}

pub(super) fn build(input: &RenderInput, drawn: &Drawn) -> Legend {
    let palette = &input.theme.palette;
    let classified = drawn
        .buildings
        .filter(|resolved| resolved.role == Role::Classified);
    let mut legend = match classified {
        Some(resolved) => {
            let shown = solid::drawn(resolved, drawn.seen, input.diagram);
            let mut legend = legend::build(
                std::slice::from_ref(resolved),
                &[shown],
                input.diagram,
                &palette.accent,
            );
            legend.rows.retain(|row| row.label != legend::SITE_LABEL);
            // The note names the year of the data, which only the scene knows.
            if legend.note.is_some() {
                legend.note = input
                    .diagram
                    .classify
                    .as_ref()
                    .and_then(|classify| classify.note_for(input.scene));
            }
            legend
        }
        None => Legend {
            title: TITLE.to_string(),
            note: None,
            rows: vec![area_row(
                "Buildings beyond",
                &section_draw::beyond_fill(
                    &massing::theme_fill(input.theme),
                    &palette.paper,
                    MIDDLE_DEPTH,
                ),
            )],
        },
    };
    legend
        .rows
        .push(area_row("Cut", section_draw::poche(palette)));
    if drawn.has_neighbours {
        let roof = context::roof(&massing::theme_fill(input.theme), input.theme);
        let fill = section_draw::beyond_fill(&roof, &palette.paper, MIDDLE_DEPTH);
        legend.rows.push(area_row("Neighbourhood beyond", &fill));
    }
    if drawn.has_trees {
        legend.rows.push(Row {
            label: "Trees beyond".to_string(),
            swatch: Swatch::Dot {
                fill: Some(drawn.tree_fill.to_string()),
                // The crowns have the elevation's ink edge, which a white crown needs to show.
                stroke: Some(palette.ink.clone()),
                opacity: 1.0,
            },
        });
    }
    legend.rows.push(area_row(
        "Ground",
        &format!("url(#{})", section_draw::HATCH_ID),
    ));
    legend.rows.extend(water_rows(drawn.waters, palette));

    // The flat ground matters more than a class note: it says the drawing is not the terrain.
    let note = if !drawn.profile.is_measured {
        FLAT_NOTE.to_string()
    } else {
        legend.note.take().unwrap_or_else(|| {
            format!(
                "Heights in metres above NAP. Seen up to {:.0} m beyond the line.",
                drawn.line.depth_m
            )
        })
    };
    legend.note = Some(match drawn.line.exaggeration {
        1 => note,
        times => format!("Heights drawn {times}\u{00d7} taller than lengths. {note}"),
    });

    legend
}

/// One row per kind of water drawn, each saying where its level and depth come from.
fn water_rows(waters: &[Water], palette: &crate::theme::Palette) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut published: Vec<f64> = waters
        .iter()
        .filter(|water| water.level_name.is_some())
        .map(|water| water.level_m)
        .collect();
    published.sort_by(f64::total_cmp);
    published.dedup();
    if !published.is_empty() {
        let levels: Vec<String> = published
            .iter()
            .map(|level| super::section_levels::precise_level(*level))
            .collect();
        rows.push(area_row(
            &format!("Water at target level {} m NAP", levels.join(", ")),
            &palette.water,
        ));
    }
    if waters.iter().any(|water| water.level_name.is_none()) {
        rows.push(area_row(
            "Water at its bank, no published level",
            &palette.water,
        ));
    }
    if waters.iter().any(|water| !water.has_bed) {
        rows.push(area_row(
            "Water depth not surveyed",
            &format!("url(#{})", section_water::UNKNOWN_DEPTH_ID),
        ));
    }

    rows
}

fn area_row(label: &str, fill: &str) -> Row {
    Row {
        label: label.to_string(),
        swatch: Swatch::Area {
            fill: fill.to_string(),
            stroke: None,
            stroke_mm: 0.0,
            opacity: 1.0,
        },
    }
}
