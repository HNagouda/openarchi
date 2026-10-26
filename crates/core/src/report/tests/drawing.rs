//! Drawing and exporting report blocks: analysis, text, references between
//! ids, and the files a report is written as. Shares the recorded scene and
//! cache of the tests around it.

use super::*;

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
    let data = ReportData::gather(
        &scene,
        &look,
        &[],
        &[],
        None,
        &SiteEdits::default(),
        &diagrams,
        &theme,
        &cache,
        contents,
    )
    .unwrap();
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

#[test]
fn a_sheet_block_shows_the_projects_pins() {
    let scene = westermarkt();
    let look = Look::default();
    let diagrams = builtin_diagrams();
    let theme = theme("paper");
    let (_folder, cache) = recorded_cache();
    let pins = [crate::pin::Pin {
        id: "pin-1".to_string(),
        icon: crate::pin::PinIcon::Star,
        label: "Entrance".to_string(),
        x: scene.origin.0,
        y: scene.origin.1,
        size: crate::pin::PinSize::Medium,
    }];
    let report = report(vec![page(
        "page-1",
        vec![block("block-1", 10.0, 10.0, 180.0, 120.0, sheet("context"))],
    )]);
    let contents = [&report.pages[0].blocks[0].content];

    let data = ReportData::gather(
        &scene,
        &look,
        &pins,
        &[],
        None,
        &SiteEdits::default(),
        &diagrams,
        &theme,
        &cache,
        contents,
    )
    .unwrap();
    let svg = page_svg(&report, 0, &data).unwrap();

    assert!(svg.contains(r#"data-pin="pin-1""#));
}
