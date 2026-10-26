//! The live data a report's blocks are drawn from, gathered once per render.
//!
//! Only what the blocks ask for is computed: the ratios when a ratios block
//! is placed, and each analysis part one of its sections is placed from. An
//! analysis part that cannot be fetched does not fail the report; its blocks
//! say so on the page instead.

use crate::analysis::{self, Part, Section};
use std::borrow::Cow;

use crate::diagram::Diagram;
use crate::edits::{self, SiteEdits};
use crate::highlight::Highlight;
use crate::model::Model;
use crate::pin::Pin;
use crate::project::Look;
use crate::ratios::{self, Group, Ratios};
use crate::scene::Scene;
use crate::sources::cache::TileCache;
use crate::theme::Theme;
use crate::Result;

use super::Content;

pub struct ReportData<'a> {
    /// The site as fetched, which ratios and analysis count.
    pub scene: &'a Scene,
    /// The site with the project's edits, which sheet blocks draw like the sheets.
    pub edited: Cow<'a, Scene>,
    /// The project's look. Sheet blocks keep its paper, view, sun and text.
    pub look: &'a Look,
    /// The project's pins. Sheet blocks show them, as every sheet does unless
    /// its look hides them.
    pub pins: &'a [Pin],
    /// The project's highlights, shown like the pins.
    pub highlights: &'a [Highlight],
    /// The project's imported model, shown on sheet blocks like on its sheets.
    pub model: Option<&'a Model>,
    pub diagrams: &'a [Diagram],
    /// The palette the report is drawn in.
    pub theme: &'a Theme,
    ratios: Option<Ratios>,
    sections: Vec<Section>,
    /// Parts that could not be made, with why.
    failures: Vec<(Part, String)>,
}

impl<'a> ReportData<'a> {
    /// Computes what `contents` need. Climate and rain read the tile cache,
    /// and on first use wait on PVGIS and KNMI.
    #[allow(clippy::too_many_arguments)]
    pub fn gather<'c>(
        scene: &'a Scene,
        look: &'a Look,
        pins: &'a [Pin],
        highlights: &'a [Highlight],
        model: Option<&'a Model>,
        edits: &SiteEdits,
        diagrams: &'a [Diagram],
        theme: &'a Theme,
        cache: &TileCache,
        contents: impl IntoIterator<Item = &'c Content> + Clone,
    ) -> Result<ReportData<'a>> {
        let needs_ratios = contents
            .clone()
            .into_iter()
            .any(|content| matches!(content, Content::Ratios { .. }));
        let ratios = if needs_ratios {
            Some(ratios::compute(scene, theme)?)
        } else {
            None
        };

        let mut sections = Vec::new();
        let mut failures = Vec::new();
        for part in parts_needed(contents) {
            match analysis::kept_part(part, scene, theme, cache) {
                Ok(found) => sections.extend(found.iter().cloned()),
                Err(error) => failures.push((part, error.to_string())),
            }
        }

        Ok(ReportData {
            scene,
            edited: edits::edited(scene, edits),
            look,
            pins,
            highlights,
            model,
            diagrams,
            theme,
            ratios,
            sections,
            failures,
        })
    }

    /// The section with `id`, or a sentence for the page saying why it is missing.
    pub fn section(&self, id: &str) -> std::result::Result<&Section, String> {
        if let Some(section) = self.sections.iter().find(|section| section.id == id) {
            return Ok(section);
        }
        let part = part_of(id);
        match self.failures.iter().find(|(failed, _)| *failed == part) {
            Some((_, error)) => Err(format!("{} could not be fetched: {error}", part.title())),
            None => Err(format!("This site has no analysis section “{id}”.")),
        }
    }

    /// The ratios group with `id`, or a sentence for the page saying why it is missing.
    pub fn group(&self, id: &str) -> std::result::Result<&Group, String> {
        self.ratios
            .iter()
            .flat_map(|ratios| ratios.groups.iter())
            .find(|group| group.id == id)
            .ok_or_else(|| format!("This site has no ratios for “{id}”."))
    }
}

/// The analysis parts `contents` draw from, each once, in report order.
pub fn parts_needed<'c>(contents: impl IntoIterator<Item = &'c Content>) -> Vec<Part> {
    let mut parts = Vec::new();
    for content in contents {
        let section = match content {
            Content::AnalysisFigure { section, .. }
            | Content::AnalysisFacts { section }
            | Content::AnalysisNotes { section } => section,
            _ => continue,
        };
        let part = part_of(section);
        if !parts.contains(&part) {
            parts.push(part);
        }
    }
    parts.sort_by_key(|part| analysis::PARTS.iter().position(|known| known == part));

    parts
}

/// Which part makes a section. Every climate section (temperature, comfort,
/// wind, sun, humidity) comes from the one PVGIS year.
fn part_of(section: &str) -> Part {
    match section {
        "location" => Part::Location,
        "site" => Part::Site,
        "rain" => Part::Rain,
        _ => Part::Climate,
    }
}
