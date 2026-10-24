//! Reports of the recorded Westermarkt scene, offline: the analysis reads a
//! temporary tile cache filled with the answers checked in under
//! tests/fixtures/analysis-cache, so no service is asked.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::compose::scope_ids;
use super::draw::inner_svg;
use super::*;
use crate::analysis::Part;
use crate::catalog::{builtin_diagrams, builtin_themes};
use crate::project::{Look, Project};
use crate::scene::Scene;
use crate::sheet::Align;
use crate::sources::cache::TileCache;
use crate::sources::grid::Tile;
use crate::theme::Theme;

const SCENE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westermarkt.scene.json"
);
const RECORDED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/analysis-cache/tiles-v1"
);

fn westermarkt() -> Scene {
    let json = fs::read_to_string(SCENE).expect("fixture is checked in");
    serde_json::from_str(&json).expect("fixture is a serialised Scene")
}

fn theme(id: &str) -> Theme {
    builtin_themes()
        .into_iter()
        .find(|theme| theme.id == id)
        .unwrap()
}

/// A cache holding every recorded answer, written fresh so none counts as stale.
fn recorded_cache() -> (TempDir, TileCache) {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());
    for dataset in fs::read_dir(RECORDED).unwrap() {
        let dataset = dataset.unwrap().path();
        let name = dataset.file_name().unwrap().to_string_lossy().to_string();
        for file in fs::read_dir(&dataset).unwrap() {
            let path = file.unwrap().path();
            let bytes = zstd::decode_all(fs::read(&path).unwrap().as_slice()).unwrap();
            cache.write(&name, tile_of(&path), &bytes).unwrap();
        }
    }

    (folder, cache)
}

/// "24_97.zst" -> column 24, row 97.
fn tile_of(path: &Path) -> Tile {
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let (column, row) = stem.split_once('_').unwrap();
    Tile {
        column: column.parse().unwrap(),
        row: row.parse().unwrap(),
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

fn page(id: &str, blocks: Vec<Block>) -> Page {
    Page {
        id: id.to_string(),
        paper: Paper::A4,
        landscape: false,
        blocks,
    }
}

fn report(pages: Vec<Page>) -> Report {
    Report {
        id: "report-1".to_string(),
        name: "Westermarkt".to_string(),
        theme_id: None,
        background: DEFAULT_BACKGROUND.to_string(),
        show_page_numbers: true,
        pages,
    }
}

fn ratios(group: &str, style: RatiosStyle) -> Content {
    Content::Ratios {
        group: group.to_string(),
        style,
    }
}

fn sheet(diagram_id: &str) -> Content {
    Content::Sheet {
        diagram_id: diagram_id.to_string(),
        variant: None,
    }
}

/// Renders every page of `report` in `theme_id` from the recorded scene.
fn render(report: &Report, theme_id: &str) -> Vec<String> {
    let scene = westermarkt();
    let look = Look::default();
    let diagrams = builtin_diagrams();
    let theme = theme(theme_id);
    let (_folder, cache) = recorded_cache();
    let contents = report
        .pages
        .iter()
        .flat_map(|page| page.blocks.iter().map(|block| &block.content));
    let data = ReportData::gather(&scene, &look, &diagrams, &theme, &cache, contents).unwrap();
    pages_svg(report, &data).unwrap()
}

#[test]
fn a_project_saved_before_reports_opens_with_none() {
    let project = Project {
        format_version: crate::project::PROJECT_FORMAT_VERSION,
        id: "p".to_string(),
        name: "P".to_string(),
        created: String::new(),
        modified: String::new(),
        area: crate::area::Area::Circle {
            center: crate::area::LonLat {
                lon: 4.88,
                lat: 52.37,
            },
            radius_m: 100.0,
        },
        scene: None,
        look: Look::default(),
        reports: Vec::new(),
    };
    let mut json = serde_json::to_value(&project).unwrap();
    json.as_object_mut().unwrap().remove("reports");

    let read: Project = serde_json::from_value(json).unwrap();

    assert!(read.reports.is_empty());
}

#[test]
fn a_block_names_its_kind_in_snake_case() {
    let json = serde_json::to_value(ratios("land_use", RatiosStyle::Pie)).unwrap();

    assert_eq!(json["kind"], "ratios");
    assert_eq!(json["style"], "pie");
}

#[test]
fn a_page_is_sized_in_millimetres_and_turns_for_landscape() {
    let mut a3 = page("page-1", Vec::new());
    a3.paper = Paper::A3;
    a3.landscape = true;

    assert_eq!(a3.size_mm(), (420.0, 297.0));
}

#[test]
fn every_block_is_a_named_layer_at_its_position() {
    let report = report(vec![page(
        "page-1",
        vec![
            block(
                "block-1",
                20.0,
                30.0,
                80.0,
                60.0,
                ratios("land_use", RatiosStyle::Bars),
            ),
            block(
                "block-2",
                110.0,
                30.0,
                80.0,
                60.0,
                ratios("land_use", RatiosStyle::Table),
            ),
        ],
    )]);

    let svg = &render(&report, "paper")[0];

    assert!(
        svg.contains(r#"width="210mm" height="297mm""#),
        "{svg:.300}"
    );
    assert!(svg.contains(r#"id="block-1" inkscape:groupmode="layer" inkscape:label="Ratios: land_use" transform="translate(20.00 30.00)""#));
    assert!(svg.contains(r#"id="block-2""#));
    assert!(svg.contains(">1 / 1</text>"), "the page number is drawn");
}

#[test]
fn two_sheets_on_one_page_keep_their_own_clip_paths() {
    let report = report(vec![page(
        "page-1",
        vec![
            block("block-1", 10.0, 10.0, 90.0, 64.0, sheet("figure-ground")),
            block("block-2", 110.0, 10.0, 90.0, 64.0, sheet("building-age")),
        ],
    )]);

    let svg = &render(&report, "paper")[0];

    assert!(svg.contains(r#"id="block-1-frame-clip""#));
    assert!(svg.contains(r#"id="block-2-frame-clip""#));
    assert!(svg.contains("url(#block-2-frame-clip)"));
    assert!(!svg.contains("url(#frame-clip)"));
}

#[test]
fn blocks_follow_the_palette_they_are_drawn_in() {
    let report = report(vec![page(
        "page-1",
        vec![block(
            "block-1",
            10.0,
            10.0,
            90.0,
            60.0,
            ratios("land_use", RatiosStyle::Pie),
        )],
    )]);

    let paper = &render(&report, "paper")[0];
    let midnight = &render(&report, "midnight")[0];

    assert!(paper.contains(&theme("paper").palette.paper));
    assert!(midnight.contains(&theme("midnight").palette.paper));
    assert_ne!(paper, midnight);
}

#[test]
fn a_missing_section_says_so_on_the_page_instead_of_failing() {
    let content = Content::AnalysisFacts {
        section: "nowhere".to_string(),
    };
    let report = report(vec![page(
        "page-1",
        vec![block("block-1", 10.0, 10.0, 90.0, 30.0, content)],
    )]);

    let svg = &render(&report, "paper")[0];

    assert!(svg.contains("stroke-dasharray"));
    assert!(svg.contains("no analysis section “nowhere”"), "{svg}");
}

#[test]
fn analysis_blocks_draw_facts_notes_and_figures() {
    let section = |name: &str| name.to_string();
    let report = report(vec![page(
        "page-1",
        vec![
            block(
                "block-1",
                10.0,
                10.0,
                190.0,
                30.0,
                Content::AnalysisFacts {
                    section: section("site"),
                },
            ),
            block(
                "block-2",
                10.0,
                50.0,
                90.0,
                80.0,
                Content::AnalysisFigure {
                    section: section("wind"),
                    figure: "wind-roses".to_string(),
                },
            ),
            block(
                "block-3",
                110.0,
                50.0,
                90.0,
                80.0,
                Content::AnalysisNotes {
                    section: section("wind"),
                },
            ),
        ],
    )]);

    let svg = &render(&report, "paper")[0];

    assert!(svg.contains(">Coverage</text>"), "the site facts are drawn");
    assert!(svg.contains(">—</text>"), "the wind notes are drawn");
    assert!(
        svg.contains("PVGIS typical year"),
        "the source line is drawn"
    );
}

#[test]
fn text_wraps_at_the_box_width_and_keeps_its_style() {
    let style = TextStyle {
        size_pt: Some(12.0),
        align: Some(Align::Centre),
        colour: Some("@accent".to_string()),
        ..TextStyle::default()
    };
    let content = Content::Text {
        text: "A long line of words that cannot fit on one line of a narrow box".to_string(),
        style,
    };
    let report = report(vec![page(
        "page-1",
        vec![block("block-1", 10.0, 10.0, 40.0, 40.0, content)],
    )]);

    let svg = &render(&report, "paper")[0];

    assert!(svg.matches("<text x=").count() > 2, "{svg}");
    assert!(svg.contains(r#"data-align="centre""#));
    assert!(svg.contains(&format!(r#"fill="{}""#, theme("paper").palette.accent)));
}

#[test]
fn a_bad_block_is_refused_with_its_id() {
    let tiny = block("block-9", 0.0, 0.0, 1.0, 40.0, sheet("context"));
    let report = report(vec![page("page-1", vec![tiny])]);

    let error = report.check().unwrap_err().to_string();

    assert!(error.contains("block-9"), "{error}");
}

#[test]
fn only_the_parts_a_report_places_are_fetched() {
    let contents = [
        Content::AnalysisFigure {
            section: "wind".to_string(),
            figure: "wind-roses".to_string(),
        },
        Content::AnalysisNotes {
            section: "temperature".to_string(),
        },
        Content::AnalysisFacts {
            section: "location".to_string(),
        },
        sheet("context"),
    ];

    assert_eq!(parts_needed(&contents), vec![Part::Location, Part::Climate]);
}

#[test]
fn ids_and_their_references_take_the_block_prefix() {
    let body = r##"<clipPath id="clip"/><g clip-path="url(#clip)"><use href="#mark"/></g><g data-id="x"/>"##;

    let scoped = scope_ids(body, "block-3");

    assert_eq!(
        scoped,
        r##"<clipPath id="block-3-clip"/><g clip-path="url(#block-3-clip)"><use href="#block-3-mark"/></g><g data-id="x"/>"##
    );
}

#[test]
fn the_document_wrapper_is_taken_off_a_sheet() {
    let document = "<?xml version=\"1.0\"?>\n<svg width=\"1mm\">\n<rect/>\n</svg>\n";

    assert_eq!(inner_svg(document), "\n<rect/>\n");
}

#[test]
fn a_three_page_report_exports_as_one_pdf_and_a_png_per_page() {
    let scene = westermarkt();
    let look = Look::default();
    let diagrams = builtin_diagrams();
    let theme = theme("paper");
    let (_folder, cache) = recorded_cache();
    let report = report(vec![
        page(
            "page-1",
            vec![block(
                "block-1",
                20.0,
                20.0,
                170.0,
                120.0,
                sheet("figure-ground"),
            )],
        ),
        page(
            "page-2",
            vec![block(
                "block-2",
                20.0,
                20.0,
                170.0,
                80.0,
                ratios("land_use", RatiosStyle::Bars),
            )],
        ),
        page("page-3", Vec::new()),
    ]);
    let contents = report
        .pages
        .iter()
        .flat_map(|page| page.blocks.iter().map(|block| &block.content));
    let data = ReportData::gather(&scene, &look, &diagrams, &theme, &cache, contents).unwrap();
    let folder = TempDir::new().unwrap();

    let pdf = export(
        &report,
        &data,
        ReportFormat::Pdf,
        72,
        &folder.path().join("r.pdf"),
    )
    .unwrap();
    let pngs = export(
        &report,
        &data,
        ReportFormat::Png,
        30,
        &folder.path().join("r.png"),
    )
    .unwrap();

    let text = String::from_utf8_lossy(&fs::read(&pdf[0]).unwrap()).to_string();
    assert!(text.starts_with("%PDF"));
    assert!(text.contains("/Count 3"));
    let names: Vec<String> = pngs
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["r-1.png", "r-2.png", "r-3.png"]);
}
