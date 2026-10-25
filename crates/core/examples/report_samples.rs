//! Lays out a three-page sample report of the Westermarkt fixture and writes
//! it in a few palettes as the PDF the app exports and a PNG per page, for
//! looking at report pages while working on them.
//!
//! cargo run --release -p openarchi-core --example report_samples   # writes target/report-samples/
//!
//! The analysis reads the answers recorded in tests/fixtures/analysis-cache,
//! so no service is asked.

use std::fs;
use std::path::Path;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::project::Look;
use openarchi_core::report::{
    export, Block, Content, Page, RatiosStyle, Report, ReportData, ReportFormat, DEFAULT_BACKGROUND,
};
use openarchi_core::scene::Scene;
use openarchi_core::sheet::{Align, Paper, Sheet, TextStyle};
use openarchi_core::sources::cache::TileCache;
use openarchi_core::sources::grid::Tile;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const RECORDED: &str = "tests/fixtures/analysis-cache/tiles-v1";
const OUT_DIR: &str = "../../target/report-samples";
/// A light, a dark and a coloured palette.
const PALETTES: [&str; 3] = ["paper", "midnight", "terracotta"];
const PNG_DPI: u32 = 110;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(manifest.join(FIXTURE)).expect("read the Westermarkt fixture");
    let scene: Scene = serde_json::from_str(&text).expect("parse the Westermarkt fixture");
    let cache_dir = tempfile::TempDir::new().expect("a temporary cache");
    let cache = TileCache::at(cache_dir.path());
    seed(&cache, &manifest.join(RECORDED));
    let out_dir = manifest.join(OUT_DIR);
    fs::create_dir_all(&out_dir).expect("create the samples folder");

    let look = Look {
        sheet: Sheet {
            title: "Westermarkt".to_string(),
            subtitle: "Amsterdam, around the Westerkerk".to_string(),
            ..Sheet::default()
        },
        ..Look::default()
    };
    let diagrams = builtin_diagrams();
    let report = sample();
    let contents: Vec<&Content> = report
        .pages
        .iter()
        .flat_map(|page| page.blocks.iter().map(|block| &block.content))
        .collect();
    for theme in builtin_themes()
        .into_iter()
        .filter(|theme| PALETTES.contains(&theme.id.as_str()))
    {
        let data = ReportData::gather(
            &scene,
            &look,
            &[],
            &[],
            &diagrams,
            &theme,
            &cache,
            contents.clone(),
        )
        .expect("gather the report's data");
        let stem = out_dir.join(format!("westermarkt--{}", theme.id));
        export(
            &report,
            &data,
            ReportFormat::Pdf,
            PNG_DPI,
            &stem.with_extension("pdf"),
        )
        .expect("write the PDF");
        export(
            &report,
            &data,
            ReportFormat::Png,
            PNG_DPI,
            &stem.with_extension("png"),
        )
        .expect("write the PNGs");
        export(
            &report,
            &data,
            ReportFormat::Svg,
            PNG_DPI,
            &stem.with_extension("svg"),
        )
        .expect("write the SVGs");
    }

    println!("samples in {}", out_dir.display());
}

/// A title page, a page of numbers and a page of sheets.
fn sample() -> Report {
    let title_page = Page {
        id: "page-1".to_string(),
        paper: Paper::A4,
        landscape: false,
        blocks: vec![
            block(
                "block-1",
                20.0,
                22.0,
                170.0,
                16.0,
                text("Westermarkt", 30.0, true),
            ),
            block(
                "block-2",
                20.0,
                38.0,
                170.0,
                8.0,
                text(
                    "Site analysis, Amsterdam, around the Westerkerk",
                    11.0,
                    false,
                ),
            ),
            block("block-3", 20.0, 52.0, 170.0, 120.0, sheet("figure-ground")),
            block(
                "block-4",
                20.0,
                182.0,
                170.0,
                42.0,
                Content::AnalysisFacts {
                    section: "site".to_string(),
                },
            ),
            block(
                "block-5",
                20.0,
                232.0,
                82.0,
                48.0,
                Content::AnalysisFacts {
                    section: "location".to_string(),
                },
            ),
            block(
                "block-6",
                108.0,
                232.0,
                82.0,
                48.0,
                Content::AnalysisNotes {
                    section: "site".to_string(),
                },
            ),
        ],
    };
    let numbers_page = Page {
        id: "page-2".to_string(),
        paper: Paper::A4,
        landscape: true,
        blocks: vec![
            block(
                "block-7",
                15.0,
                15.0,
                125.0,
                75.0,
                ratios("land_use", RatiosStyle::Bars),
            ),
            block(
                "block-8",
                155.0,
                15.0,
                125.0,
                60.0,
                ratios("building_use", RatiosStyle::Pie),
            ),
            block(
                "block-9",
                15.0,
                100.0,
                125.0,
                90.0,
                ratios("year_built", RatiosStyle::Table),
            ),
            block(
                "block-10",
                155.0,
                85.0,
                60.0,
                80.0,
                figure("wind", "wind-roses"),
            ),
            block(
                "block-11",
                220.0,
                85.0,
                60.0,
                105.0,
                Content::AnalysisNotes {
                    section: "wind".to_string(),
                },
            ),
        ],
    };
    let sheets_page = Page {
        id: "page-3".to_string(),
        paper: Paper::A3,
        landscape: true,
        blocks: vec![
            block("block-12", 20.0, 20.0, 185.0, 131.0, sheet("building-age")),
            block("block-13", 215.0, 20.0, 185.0, 131.0, sheet("landscape")),
            block("block-14", 20.0, 165.0, 120.0, 90.0, figure("temperature", "temperature")),
            block("block-15", 150.0, 165.0, 120.0, 110.0, Content::AnalysisNotes { section: "temperature".to_string() }),
            block("block-16", 280.0, 165.0, 120.0, 60.0, text("Age and landscape side by side: the oldest blocks line the canals, the newer ones fill the gaps behind them.", 10.0, false)),
        ],
    };

    Report {
        id: "report-1".to_string(),
        name: "Westermarkt".to_string(),
        theme_id: None,
        background: DEFAULT_BACKGROUND.to_string(),
        show_page_numbers: true,
        pages: vec![title_page, numbers_page, sheets_page],
    }
}

fn block(id: &str, x_mm: f64, y_mm: f64, width_mm: f64, height_mm: f64, content: Content) -> Block {
    Block {
        id: id.to_string(),
        x_mm,
        y_mm,
        width_mm,
        height_mm,
        content,
    }
}

fn text(words: &str, size_pt: f64, bold: bool) -> Content {
    Content::Text {
        text: words.to_string(),
        style: TextStyle {
            size_pt: Some(size_pt),
            bold: Some(bold),
            align: Some(Align::Left),
            ..TextStyle::default()
        },
    }
}

fn sheet(diagram_id: &str) -> Content {
    Content::Sheet {
        diagram_id: diagram_id.to_string(),
        variant: None,
    }
}

fn ratios(group: &str, style: RatiosStyle) -> Content {
    Content::Ratios {
        group: group.to_string(),
        style,
    }
}

fn figure(section: &str, figure: &str) -> Content {
    Content::AnalysisFigure {
        section: section.to_string(),
        figure: figure.to_string(),
    }
}

/// Fills `cache` with every recorded answer, written fresh so none counts as stale.
fn seed(cache: &TileCache, recorded: &Path) {
    for dataset in fs::read_dir(recorded).expect("the recorded analysis answers") {
        let dataset = dataset.expect("a dataset folder").path();
        let name = dataset
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        for file in fs::read_dir(&dataset).expect("the dataset's files") {
            let path = file.expect("a recorded answer").path();
            let stem = path
                .file_stem()
                .expect("a stem")
                .to_string_lossy()
                .to_string();
            let (column, row) = stem.split_once('_').expect("a file named column_row");
            let tile = Tile {
                column: column.parse().expect("a column"),
                row: row.parse().expect("a row"),
            };
            let bytes =
                zstd::decode_all(fs::read(&path).expect("read").as_slice()).expect("unpack");
            cache.write(&name, tile, &bytes).expect("seed the cache");
        }
    }
}
