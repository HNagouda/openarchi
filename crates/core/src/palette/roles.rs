//! Which swatch fills which role, by one rule a person can check by eye.
//!
//! Lightness, chroma (how colourful) and hue are OKLCH, see `colour.rs`.
//!
//! 1. Paper is the lightest quiet, light swatch (chroma at most 0.05,
//!    lightness at least 0.85), or else the lightest swatch, softened to a
//!    pale tint so a colourful paper never floods the sheet.
//! 2. Ink is the darkest swatch left.
//! 3. Figure, the buildings, is the darkest swatch left that stands out from
//!    the paper by WCAG's 3:1 for graphics. When none does, it is the ink.
//! 4. Accent is the most colourful swatch left.
//! 5. Water and green take the swatch nearest in hue to blue (240°) and to
//!    green (140°), softened towards the paper because they cover large areas.
//!    With no swatch within 60° of that hue, they become a quiet grey mixed
//!    from paper and ink.
//!
//! A role the user pinned to a swatch takes it exactly, and the rule fills
//! the other roles around it.

use serde::Serialize;

use super::colour::{hue_distance, lch, Lch};
use super::{RoleChoice, Swatch};
use crate::render::mix;
use crate::{Error, Result};

/// The roles a palette fills from its swatches, in the order the editor lists them.
pub const RESOLVED_ROLES: [&str; 6] = ["paper", "ink", "figure", "accent", "water", "green"];

/// Up to this chroma a swatch reads as a neutral, fit to be paper.
const QUIET_CHROMA: f64 = 0.05;
/// From this lightness a quiet swatch reads as paper rather than as a grey.
const PAPER_MIN_LIGHTNESS: f64 = 0.85;
/// Below this chroma a hue is noise, so a grey never counts as blue or green.
const HUE_MIN_CHROMA: f64 = 0.03;
const WATER_HUE_DEG: f64 = 240.0;
const GREEN_HUE_DEG: f64 = 140.0;
const HUE_TOLERANCE_DEG: f64 = 60.0;
/// How far a colourful automatic paper moves towards white.
const PAPER_TINT: f64 = 0.72;
/// How far automatic water and green move towards the paper.
const AREA_SOFTENING: f64 = 0.45;
/// The quiet greys for water and green without a matching swatch, from paper towards ink.
const GREY_WATER: f64 = 0.2;
const GREY_GREEN: f64 = 0.13;
const WHITE: &str = "#FFFFFF";

/// How one role was filled.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoleColour {
    /// "paper", "ink" and so on, as in `RESOLVED_ROLES`.
    pub role: String,
    /// The colour the theme uses, "#RRGGBB".
    pub hex: String,
    /// The swatch it came from; `None` for a grey mixed from paper and ink.
    pub swatch: Option<usize>,
    pub is_pinned: bool,
    /// One short phrase for the editor: "lightest swatch, softened".
    pub reason: String,
}

/// Fills the six roles from `swatches`, keeping every pin in `choice`.
pub fn resolve(swatches: &[Swatch], choice: &RoleChoice) -> Result<Vec<RoleColour>> {
    check_pins(swatches, choice)?;
    let colours: Vec<Lch> = swatches.iter().map(|swatch| lch(&swatch.hex)).collect();
    let pinned: Vec<usize> = [
        choice.paper,
        choice.ink,
        choice.figure,
        choice.accent,
        choice.water,
        choice.green,
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut taken = pinned.clone();

    let paper = match choice.paper {
        Some(index) => pinned_role("paper", swatches, index),
        None => automatic_paper(swatches, &colours, &taken),
    };
    take(&mut taken, paper.swatch);

    let ink = match choice.ink {
        Some(index) => pinned_role("ink", swatches, index),
        None => {
            let index = darkest(&colours, &taken).unwrap_or(0);
            from_swatch("ink", swatches, index, "darkest swatch")
        }
    };
    take(&mut taken, ink.swatch);

    let figure = match choice.figure {
        Some(index) => pinned_role("figure", swatches, index),
        None => automatic_figure(swatches, &colours, &taken, &paper, &ink),
    };
    take(&mut taken, figure.swatch);

    let accent = match choice.accent {
        Some(index) => pinned_role("accent", swatches, index),
        None => automatic_accent(swatches, &colours, &taken, &paper, &ink),
    };
    take(&mut taken, accent.swatch);

    let (water, green) = water_and_green(swatches, &colours, choice, &paper, &ink, &taken);
    Ok(vec![paper, ink, figure, accent, water, green])
}

fn check_pins(swatches: &[Swatch], choice: &RoleChoice) -> Result<()> {
    if swatches.is_empty() {
        return Err(Error::Invalid(
            "a palette needs at least one swatch".to_string(),
        ));
    }
    let pins = [
        ("paper", choice.paper),
        ("ink", choice.ink),
        ("figure", choice.figure),
        ("accent", choice.accent),
        ("water", choice.water),
        ("green", choice.green),
    ];
    for (role, pin) in pins {
        if let Some(index) = pin.filter(|index| *index >= swatches.len()) {
            return Err(Error::Invalid(format!(
                "{role} is pinned to swatch {index}, but the palette has {} swatches",
                swatches.len()
            )));
        }
    }

    Ok(())
}

fn automatic_paper(swatches: &[Swatch], colours: &[Lch], taken: &[usize]) -> RoleColour {
    let free: Vec<usize> = free_indices(colours, taken);
    let quiet: Vec<usize> = free
        .iter()
        .copied()
        .filter(|index| is_paper_like(&colours[*index]))
        .collect();
    let pool = if quiet.is_empty() { &free } else { &quiet };
    let index = lightest_of(colours, pool).unwrap_or(0);

    if is_paper_like(&colours[index]) {
        return from_swatch("paper", swatches, index, "lightest quiet swatch");
    }
    RoleColour {
        role: "paper".to_string(),
        hex: mix(&swatches[index].hex, WHITE, PAPER_TINT),
        swatch: Some(index),
        is_pinned: false,
        reason: "lightest swatch, softened to a pale tint".to_string(),
    }
}

fn is_paper_like(colour: &Lch) -> bool {
    colour.chroma <= QUIET_CHROMA && colour.lightness >= PAPER_MIN_LIGHTNESS
}

fn automatic_figure(
    swatches: &[Swatch],
    colours: &[Lch],
    taken: &[usize],
    paper: &RoleColour,
    ink: &RoleColour,
) -> RoleColour {
    let standing_out: Vec<usize> = free_indices(colours, taken)
        .into_iter()
        .filter(|index| {
            super::contrast_ratio(&swatches[*index].hex, &paper.hex) >= super::GRAPHIC_CONTRAST
        })
        .collect();
    match darkest_of(colours, &standing_out) {
        Some(index) => from_swatch(
            "figure",
            swatches,
            index,
            "darkest swatch left that stands out from the paper",
        ),
        None => RoleColour {
            role: "figure".to_string(),
            reason: "the ink, as no other swatch stands out from the paper".to_string(),
            is_pinned: false,
            ..ink.clone()
        },
    }
}

fn automatic_accent(
    swatches: &[Swatch],
    colours: &[Lch],
    taken: &[usize],
    paper: &RoleColour,
    ink: &RoleColour,
) -> RoleColour {
    let free = free_indices(colours, taken);
    let besides_paper: Vec<usize> = (0..colours.len())
        .filter(|index| Some(*index) != paper.swatch)
        .collect();
    let pool = if free.is_empty() {
        &besides_paper
    } else {
        &free
    };
    match most_colourful(colours, pool) {
        Some(index) => from_swatch("accent", swatches, index, "most colourful swatch left"),
        None => RoleColour {
            role: "accent".to_string(),
            reason: "the ink, as the palette has no other swatch".to_string(),
            is_pinned: false,
            ..ink.clone()
        },
    }
}

/// Water and green each take the nearest swatch in hue. When both want the
/// same one, the nearer role keeps it and the other looks again without it.
fn water_and_green(
    swatches: &[Swatch],
    colours: &[Lch],
    choice: &RoleChoice,
    paper: &RoleColour,
    ink: &RoleColour,
    taken: &[usize],
) -> (RoleColour, RoleColour) {
    // Paper and ink are too light and too dark for large areas; the rest may be shared.
    let blocked: Vec<usize> = [paper.swatch, ink.swatch].into_iter().flatten().collect();
    let pinned_elsewhere: Vec<usize> = taken
        .iter()
        .copied()
        .filter(|index| Some(*index) != choice.water && Some(*index) != choice.green)
        .collect();
    let nearest = |target: f64, also_blocked: Option<usize>| {
        nearest_hue(colours, target, &blocked, also_blocked, &pinned_elsewhere)
    };

    let mut water_pick = nearest(WATER_HUE_DEG, None);
    let mut green_pick = nearest(GREEN_HUE_DEG, None);
    if choice.water.is_none() && choice.green.is_none() {
        if let (Some(water), Some(green)) = (water_pick, green_pick) {
            if water.0 == green.0 && water.1 <= green.1 {
                green_pick = nearest(GREEN_HUE_DEG, Some(water.0));
            } else if water.0 == green.0 {
                water_pick = nearest(WATER_HUE_DEG, Some(green.0));
            }
        }
    }

    let water = match choice.water {
        Some(index) => pinned_role("water", swatches, index),
        None => area_role(
            "water", swatches, water_pick, paper, ink, GREY_WATER, "blue",
        ),
    };
    let green = match choice.green {
        Some(index) => pinned_role("green", swatches, index),
        None => area_role(
            "green", swatches, green_pick, paper, ink, GREY_GREEN, "green",
        ),
    };
    (water, green)
}

fn area_role(
    role: &str,
    swatches: &[Swatch],
    pick: Option<(usize, f64)>,
    paper: &RoleColour,
    ink: &RoleColour,
    grey_share: f64,
    hue_name: &str,
) -> RoleColour {
    match pick {
        Some((index, _)) => RoleColour {
            role: role.to_string(),
            hex: mix(&swatches[index].hex, &paper.hex, AREA_SOFTENING),
            swatch: Some(index),
            is_pinned: false,
            reason: format!("swatch nearest to {hue_name}, softened towards the paper"),
        },
        None => RoleColour {
            role: role.to_string(),
            hex: mix(&paper.hex, &ink.hex, grey_share),
            swatch: None,
            is_pinned: false,
            reason: format!("a quiet grey, as no swatch is near {hue_name}"),
        },
    }
}

/// The swatch nearest in hue to `target`, and how far it is, within the tolerance.
fn nearest_hue(
    colours: &[Lch],
    target: f64,
    blocked: &[usize],
    also_blocked: Option<usize>,
    pinned_elsewhere: &[usize],
) -> Option<(usize, f64)> {
    (0..colours.len())
        .filter(|index| !blocked.contains(index) && Some(*index) != also_blocked)
        .filter(|index| !pinned_elsewhere.contains(index))
        .filter(|index| colours[*index].chroma >= HUE_MIN_CHROMA)
        .map(|index| (index, hue_distance(colours[index].hue_deg, target)))
        .filter(|(_, distance)| *distance <= HUE_TOLERANCE_DEG)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

// * ====================================================================
// * === utils
// * ====================================================================

fn pinned_role(role: &str, swatches: &[Swatch], index: usize) -> RoleColour {
    RoleColour {
        is_pinned: true,
        ..from_swatch(role, swatches, index, "your choice")
    }
}

fn from_swatch(role: &str, swatches: &[Swatch], index: usize, reason: &str) -> RoleColour {
    RoleColour {
        role: role.to_string(),
        hex: swatches[index].hex.clone(),
        swatch: Some(index),
        is_pinned: false,
        reason: reason.to_string(),
    }
}

fn take(taken: &mut Vec<usize>, swatch: Option<usize>) {
    if let Some(index) = swatch {
        taken.push(index);
    }
}

fn free_indices(colours: &[Lch], taken: &[usize]) -> Vec<usize> {
    (0..colours.len())
        .filter(|index| !taken.contains(index))
        .collect()
}

fn darkest(colours: &[Lch], taken: &[usize]) -> Option<usize> {
    let free = free_indices(colours, taken);
    darkest_of(colours, &free)
        .or_else(|| darkest_of(colours, &(0..colours.len()).collect::<Vec<_>>()))
}

fn lightest_of(colours: &[Lch], pool: &[usize]) -> Option<usize> {
    pool.iter()
        .copied()
        .max_by(|a, b| colours[*a].lightness.total_cmp(&colours[*b].lightness))
}

fn darkest_of(colours: &[Lch], pool: &[usize]) -> Option<usize> {
    pool.iter()
        .copied()
        .min_by(|a, b| colours[*a].lightness.total_cmp(&colours[*b].lightness))
}

fn most_colourful(colours: &[Lch], pool: &[usize]) -> Option<usize> {
    pool.iter()
        .copied()
        .max_by(|a, b| colours[*a].chroma.total_cmp(&colours[*b].chroma))
}
