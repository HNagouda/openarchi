//! The colours the 3D view takes from the sheet it opened from, so the view
//! and the sheet read as one series. Buildings take the diagram's classes the
//! way the axonometric's roofs do (age, use, height), or the light massing
//! colour a tilted sheet stands plain buildings in. The neighbourhood is
//! muted exactly as the sheet mutes it, and the model takes the accent, as
//! it does on every sheet.

use crate::render::{
    building_height, lighten_massing, luminance, massing_fill, mix, muted_colour, resolve_colour,
    resolve_layers, roof_fill, RenderInput, ResolvedLayer, ESTIMATED_GREY,
};
use crate::scene::{Feature, Layer};
use crate::theme::Theme;
use crate::Result;

use super::Sky;

/// How far the sky overhead moves from the paper towards the palette's water.
const SKY_TINT: f64 = 0.3;
/// Plain buildings closer to the paper than this in luminance stand in their
/// outline colour instead, see `massing`.
const MIN_MASSING_CONTRAST: f64 = 0.08;

pub struct Colours<'a> {
    input: &'a RenderInput<'a>,
    /// The sheet's building layer as a tilted view resolves it, `None` when
    /// the diagram leaves buildings out or the look hides them.
    buildings: Option<ResolvedLayer>,
}

impl<'a> Colours<'a> {
    pub fn of(input: &'a RenderInput<'a>) -> Result<Colours<'a>> {
        let mut layers = resolve_layers(input.diagram, input.theme, input.look)?;
        lighten_massing(&mut layers, input.look, input.theme);
        let buildings = layers
            .into_iter()
            .find(|resolved| resolved.layer == Layer::Building);

        Ok(Colours { input, buildings })
    }

    fn theme(&self) -> &Theme {
        self.input.theme
    }

    /// The roof and walls of one of the site's buildings, greyed like the
    /// sheet's when its height is a guess.
    pub fn building(&self, feature: &Feature) -> String {
        let colour = match &self.buildings {
            Some(resolved) if resolved.classes.is_some() => {
                roof_fill(resolved, feature, self.input.diagram, self.theme())
            }
            Some(resolved) => self.massing(resolved.style.fill.as_deref()),
            None => self.massing(None),
        };
        let (_, is_estimated) = building_height(feature);
        if is_estimated {
            return mix(&colour, &self.theme().palette.unknown, ESTIMATED_GREY);
        }

        colour
    }

    /// Every building around the site, in the sheet's muted roof colour.
    pub fn neighbour(&self) -> String {
        muted_colour(&self.massing(None), &self.theme().palette.paper)
    }

    /// The colour plain buildings stand in: the sheet's own `fill`, or the
    /// theme's massing colour. A palette that draws buildings as a dark fill
    /// on dark paper with a light outline, like Ink, would leave them unseen
    /// in 3D, which draws no outlines, so those stand in the outline's colour.
    fn massing(&self, fill: Option<&str>) -> String {
        let theme = self.theme();
        let fill = fill.map_or_else(|| massing_fill(theme), str::to_string);
        let contrast = (luminance(&fill) - luminance(&theme.palette.paper)).abs();
        if contrast >= MIN_MASSING_CONTRAST {
            return fill;
        }
        theme
            .layers
            .get(&Layer::Building)
            .and_then(|style| style.stroke.as_deref())
            .and_then(|stroke| resolve_colour(&theme.palette, stroke).ok())
            .unwrap_or(fill)
    }

    /// The site's ground where no layer covers it.
    pub fn open_ground(&self) -> String {
        self.theme().palette.ground.clone()
    }

    /// A ground layer inside the site in its theme fill, `None` when the theme gives it none.
    pub fn ground(&self, layer: Layer) -> Result<Option<String>> {
        let Some(fill) = self
            .theme()
            .layers
            .get(&layer)
            .and_then(|style| style.fill.as_ref())
        else {
            return Ok(None);
        };

        Ok(Some(resolve_colour(&self.theme().palette, fill)?))
    }

    /// A ground layer around the site, muted as the sheet's neighbourhood is.
    pub fn surrounding(&self, layer: Layer) -> Result<Option<String>> {
        let paper = &self.theme().palette.paper;
        Ok(self.ground(layer)?.map(|fill| muted_colour(&fill, paper)))
    }

    /// The tree crowns: the tree layer's fill, as the tilted sheets draw them.
    pub fn trees(&self) -> Result<String> {
        Ok(self
            .ground(Layer::Tree)?
            .unwrap_or_else(|| self.theme().palette.green.clone()))
    }

    pub fn sky(&self) -> Sky {
        let palette = &self.theme().palette;
        Sky {
            horizon: palette.paper.clone(),
            zenith: mix(&palette.paper, &palette.water, SKY_TINT),
        }
    }

    pub fn model(&self) -> String {
        self.theme().palette.accent.clone()
    }
}
