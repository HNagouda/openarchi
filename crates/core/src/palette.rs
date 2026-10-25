//! A user's own palette: a few swatches, imported from a palette generator or
//! picked by hand, turned into a whole theme that works on every diagram.
//!
//! A theme needs far more than five colours: nine roles, a ramp, a water ramp,
//! categories and a fill for every layer. The swatches fill the main roles by
//! one plain rule (`roles.rs`), the user can pin any role to another swatch,
//! and everything else is mixed from those roles (`derive.rs`), the way the
//! built-in themes are designed. The swatches and pins are kept in the theme
//! file, so the palette editor can open it again.

mod colour;
mod derive;
mod parse;
mod roles;

use serde::{Deserialize, Serialize};

use crate::theme::{Theme, Typography};
use crate::Result;

use colour::contrast_ratio;
// Shared with the catalog, which gives a saved palette the layers added after it.
pub(crate) use derive::{layer_style, template_theme};
pub use parse::parse;
use parse::MAX_SWATCHES;
use roles::resolve;
pub use roles::RoleColour;

/// WCAG 2.1 success criterion 1.4.3: body text needs 4.5:1 against its background.
pub const TEXT_CONTRAST: f64 = 4.5;
/// WCAG 2.1 success criterion 1.4.11: graphics need 3:1 to be told apart from their background.
pub const GRAPHIC_CONTRAST: f64 = 3.0;

/// One colour of a palette, "#RRGGBB", with the name its generator gave it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Swatch {
    pub hex: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Which swatch the user pinned to each role, by index. `None` lets the rule choose.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RoleChoice {
    #[serde(default)]
    pub paper: Option<usize>,
    #[serde(default)]
    pub ink: Option<usize>,
    #[serde(default)]
    pub figure: Option<usize>,
    #[serde(default)]
    pub accent: Option<usize>,
    #[serde(default)]
    pub water: Option<usize>,
    #[serde(default)]
    pub green: Option<usize>,
}

/// What a palette made in the app was built from. Kept in its theme file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaletteSource {
    pub swatches: Vec<Swatch>,
    #[serde(default)]
    pub roles: RoleChoice,
}

/// A palette as the editor holds it, before it is saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaletteDraft {
    pub name: String,
    pub swatches: Vec<Swatch>,
    #[serde(default)]
    pub roles: RoleChoice,
    /// The type of the theme it was duplicated from; Inter otherwise.
    #[serde(default)]
    pub typography: Option<Typography>,
    /// The id of the theme it was duplicated from, so an unchanged copy keeps
    /// that theme's hand-made look; see `check_copy`.
    #[serde(default)]
    pub based_on: Option<String>,
}

/// The theme a draft becomes, how each role was filled, and what reads badly.
#[derive(Debug, Clone, Serialize)]
pub struct PaletteCheck {
    pub theme: Theme,
    pub roles: Vec<RoleColour>,
    /// Plain sentences, each naming the colours and the WCAG ratio they miss.
    pub warnings: Vec<String>,
    /// Text in ink meets its WCAG ratio on the paper. The editor refuses to
    /// save a palette whose text cannot be read; faint buildings or a faint
    /// accent only warn, since a pale figure can be a deliberate look.
    pub is_readable: bool,
}

/// The whole theme for `draft`, saved under `id`, with its contrast warnings.
pub fn check(draft: &PaletteDraft, id: &str) -> Result<PaletteCheck> {
    let roles = resolve(&draft.swatches, &draft.roles)?;
    let theme = derive::theme(draft, id, &roles)?;
    let warnings = contrast_warnings(&theme);
    let is_readable = is_readable(&theme);

    Ok(PaletteCheck {
        theme,
        roles,
        warnings,
        is_readable,
    })
}

/// Like `check`, for a draft duplicated from `base`. While its swatches and
/// pins are still the ones `draft_from(base)` gave it, the theme is a copy of
/// `base` under the new id and name: a built-in theme's hand-tuned roads,
/// ramps and categories are worth more than mixed ones. Any change switches
/// to the mixed theme.
pub fn check_copy(draft: &PaletteDraft, id: &str, base: &Theme) -> Result<PaletteCheck> {
    let mixed = check(draft, id)?;
    let untouched = draft_from(base, &draft.name);
    let is_unchanged = untouched.swatches == draft.swatches && untouched.roles == draft.roles;
    if !is_unchanged {
        return Ok(mixed);
    }

    let theme = Theme {
        id: id.to_string(),
        name: mixed.theme.name.clone(),
        author: mixed.theme.author.clone(),
        source: mixed.theme.source.clone(),
        ..base.clone()
    };
    let warnings = contrast_warnings(&theme);
    let is_readable = is_readable(&theme);
    Ok(PaletteCheck {
        theme,
        roles: mixed.roles,
        warnings,
        is_readable,
    })
}

/// A draft holding `theme`'s main colours, each pinned to its role, so
/// duplicating a built-in palette starts from what it looks like.
pub fn draft_from(theme: &Theme, name: &str) -> PaletteDraft {
    if let Some(source) = &theme.source {
        return PaletteDraft {
            name: name.to_string(),
            swatches: source.swatches.clone(),
            roles: source.roles.clone(),
            typography: Some(theme.typography.clone()),
            based_on: Some(theme.id.clone()),
        };
    }

    let palette = &theme.palette;
    let mut swatches: Vec<Swatch> = Vec::new();
    let roles = RoleChoice {
        paper: Some(swatch_index(&mut swatches, &palette.paper)),
        ink: Some(swatch_index(&mut swatches, &palette.ink)),
        figure: Some(swatch_index(&mut swatches, &palette.figure)),
        accent: Some(swatch_index(&mut swatches, &palette.accent)),
        water: Some(swatch_index(&mut swatches, &palette.water)),
        green: Some(swatch_index(&mut swatches, &palette.green)),
    };
    for category in &theme.categories {
        if swatches.len() >= MAX_SWATCHES {
            break;
        }
        swatch_index(&mut swatches, category);
    }

    PaletteDraft {
        name: name.to_string(),
        swatches,
        roles,
        typography: Some(theme.typography.clone()),
        based_on: Some(theme.id.clone()),
    }
}

/// Where `hex` is among `swatches`, added at the end when it is new.
fn swatch_index(swatches: &mut Vec<Swatch>, hex: &str) -> usize {
    let hex = hex.to_ascii_uppercase();
    if let Some(index) = swatches.iter().position(|swatch| swatch.hex == hex) {
        return index;
    }
    swatches.push(Swatch { hex, name: None });
    swatches.len() - 1
}

/// Text in ink on paper, which a sheet cannot do without.
fn is_readable(theme: &Theme) -> bool {
    let palette = &theme.palette;
    contrast_ratio(&palette.ink, &palette.paper) >= TEXT_CONTRAST
}

/// Text in ink on paper, the buildings and the accent line against paper.
fn contrast_warnings(theme: &Theme) -> Vec<String> {
    let palette = &theme.palette;
    let mut warnings = Vec::new();
    let text = contrast_ratio(&palette.ink, &palette.paper);
    if text < TEXT_CONTRAST {
        warnings.push(format!(
            "Text in ink {} on paper {} has a contrast of {text:.1}:1, below the {TEXT_CONTRAST}:1 WCAG asks for small text. Pick a darker ink or a lighter paper.",
            palette.ink, palette.paper
        ));
    }
    let figure = contrast_ratio(&palette.figure, &palette.paper);
    if figure < GRAPHIC_CONTRAST {
        warnings.push(format!(
            "Buildings in {} stand out weakly from the paper ({figure:.1}:1, WCAG asks {GRAPHIC_CONTRAST}:1 for graphics).",
            palette.figure
        ));
    }
    let accent = contrast_ratio(&palette.accent, &palette.paper);
    if accent < GRAPHIC_CONTRAST {
        warnings.push(format!(
            "The accent {} is faint on the paper ({accent:.1}:1), so the site outline will be hard to see.",
            palette.accent
        ));
    }

    warnings
}

#[cfg(test)]
mod tests;
