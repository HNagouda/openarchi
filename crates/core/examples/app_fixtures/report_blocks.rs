use std::fs;
use std::path::Path;

use openarchi_core::analysis::{self, PARTS};
use openarchi_core::catalog::builtin_diagrams;
use openarchi_core::edits::SiteEdits;
use openarchi_core::project::Look;
use openarchi_core::ratios;
use openarchi_core::report::{block_svg, Block, Content, RatiosStyle, ReportData};
use openarchi_core::scene::Scene;
use openarchi_core::sources::cache::TileCache;

use crate::palettes::fixture_themes;

/// The starting block sizes of app/src/report/library.ts, so a mock block
/// fills its box the way a drawn one does.
const CHART_WIDTH_MM: f64 = 120.0;
const FIGURE_WIDTH_MM: f64 = 90.0;
const RATIOS_HEADER_MM: f64 = 7.1;
const BAR_ROW_MM: f64 = 7.6;
const TABLE_ROW_MM: f64 = 5.4;
const PIE_ROW_MM: f64 = 5.0;
const PIE_DIAMETER_MM: f64 = 43.0;
const FACTS_WIDTH_MM: f64 = 170.0;
const FACTS_HEADER_MM: f64 = 9.9;
const FACT_ROW_MM: f64 = 10.0;
const FACT_COLUMNS: usize = 4;
const NOTES_SIZE_MM: (f64, f64) = (90.0, 70.0);

/// Every ratios group in every chart style, and the facts and notes of every
/// analysis section, in every theme, as `render_report_block` draws them at
/// the size the library starts them at: "ratios-land_use-bars--paper.svg",
/// "figure-wind-wind-roses--paper.svg", "facts-site--paper.svg",
/// "notes-wind--paper.svg". Sheets are not written: the mock takes them from
/// the previews.
pub(crate) fn write_report_blocks(scene: &Scene, cache: &TileCache, out_dir: &Path) {
    let folder = out_dir.join("report-blocks");
    fs::create_dir_all(&folder).expect("create the report blocks folder");
    let look = Look::default();
    let diagrams = builtin_diagrams();
    for theme in fixture_themes() {
        let computed = ratios::compute(scene, &theme).expect("compute the ratios");
        let mut blocks = Vec::new();
        for group in &computed.groups {
            let rows = group.rows.len() as f64;
            let styles = [
                ("bars", RatiosStyle::Bars, rows * BAR_ROW_MM),
                (
                    "pie",
                    RatiosStyle::Pie,
                    PIE_DIAMETER_MM.max(rows * PIE_ROW_MM),
                ),
                ("table", RatiosStyle::Table, rows * TABLE_ROW_MM),
            ];
            for (name, style, rows_mm) in styles {
                let content = Content::Ratios {
                    group: group.id.clone(),
                    style,
                };
                let size = (CHART_WIDTH_MM, RATIOS_HEADER_MM + rows_mm);
                blocks.push((format!("ratios-{}-{name}", group.id), content, size));
            }
        }
        for part in PARTS {
            for section in analysis::part(part, scene, None, &theme, cache).expect("draw the part")
            {
                for figure in &section.figures {
                    let content = Content::AnalysisFigure {
                        section: section.id.clone(),
                        figure: figure.id.clone(),
                    };
                    let height = FIGURE_WIDTH_MM * figure.height_mm / figure.width_mm;
                    let name = format!("figure-{}-{}", section.id, figure.id);
                    blocks.push((name, content, (FIGURE_WIDTH_MM, height)));
                }
                let rows = section.facts.len().div_ceil(FACT_COLUMNS) as f64;
                let facts = Content::AnalysisFacts {
                    section: section.id.clone(),
                };
                let size = (FACTS_WIDTH_MM, FACTS_HEADER_MM + rows * FACT_ROW_MM);
                blocks.push((format!("facts-{}", section.id), facts, size));
                let notes = Content::AnalysisNotes {
                    section: section.id.clone(),
                };
                blocks.push((format!("notes-{}", section.id), notes, NOTES_SIZE_MM));
            }
        }

        let contents: Vec<&Content> = blocks.iter().map(|(_, content, _)| content).collect();
        let data = ReportData::gather(
            scene,
            &look,
            &[],
            &[],
            None,
            &SiteEdits::default(),
            &diagrams,
            &theme,
            cache,
            contents,
        )
        .expect("gather the report data");
        for (name, content, (width_mm, height_mm)) in &blocks {
            let block = Block {
                id: "block-1".to_string(),
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: *width_mm,
                height_mm: *height_mm,
                content: content.clone(),
            };
            let svg = block_svg(&block, &data).expect("draw the block");
            fs::write(folder.join(format!("{name}--{}.svg", theme.id)), svg)
                .expect("write the block");
        }
    }
}
