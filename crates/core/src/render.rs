//! Resolves a scene, diagram, theme and sheet into one SVG document.
//!
//! This SVG is the single source of truth for 2D: the app shows it as the
//! preview, and PDF and PNG are converted from it.

use crate::diagram::Diagram;
use crate::project::Look;
use crate::scene::Scene;
use crate::theme::Theme;
use crate::Result;

/// Everything the renderer needs, already resolved from ids.
pub struct RenderInput<'a> {
    pub scene: &'a Scene,
    pub diagram: &'a Diagram,
    pub theme: &'a Theme,
    pub look: &'a Look,
}

/// The full sheet as an SVG document, sized in millimetres.
pub fn render_svg(input: &RenderInput) -> Result<String> {
    let _ = input;
    todo!("render layer")
}
