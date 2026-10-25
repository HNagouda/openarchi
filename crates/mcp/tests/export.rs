//! Protocol tests for the tools that write things out: render_preview, export and the reports.

use openarchi_core::library::Library;
use openarchi_core::scene::Scene;
use serde_json::{json, Value};
use tempfile::TempDir;

mod common;

use common::{fetched_westermarkt, Server, WESTERMARKT_SCENE};

#[test]
fn preview_before_a_fetch_says_to_fetch_first() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err("render_preview", json!({ "id": id }));
    assert!(message.contains("fetch_site"), "{message}");
}

#[test]
fn a_report_laid_out_in_the_app_is_listed_and_exports_as_pdf_svg_and_png() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERMARKT_SCENE).unwrap()).unwrap();
    project.scene = Some(scene);
    project.reports = serde_json::from_value(json!([{
        "id": "report-1",
        "name": "Westermarkt",
        "show_page_numbers": true,
        "pages": [
            { "id": "page-1", "paper": "a4", "landscape": false, "blocks": [
                { "id": "block-1", "x_mm": 20.0, "y_mm": 20.0, "width_mm": 170.0, "height_mm": 120.0,
                  "content": { "kind": "sheet", "diagram_id": "figure-ground" } },
            ] },
            { "id": "page-2", "paper": "a3", "landscape": true, "blocks": [
                { "id": "block-2", "x_mm": 20.0, "y_mm": 20.0, "width_mm": 120.0, "height_mm": 80.0,
                  "content": { "kind": "ratios", "group": "land_use", "style": "bars" } },
                { "id": "block-3", "x_mm": 150.0, "y_mm": 20.0, "width_mm": 120.0, "height_mm": 30.0,
                  "content": { "kind": "text", "text": "Notes for the client" } },
            ] },
        ],
    }]))
    .unwrap();
    library.save(&mut project).unwrap();

    let listed = server.call_ok("list_reports", json!({ "id": id }));
    assert_eq!(listed["reports"][0]["id"], json!("report-1"));
    assert_eq!(listed["reports"][0]["page_count"], json!(2));
    assert_eq!(
        listed["reports"][0]["background"],
        Value::Null,
        "only what an assistant needs is listed"
    );

    let path = server.library.path().join("westermarkt-report.pdf");
    let written = server.call_ok(
        "export_report",
        json!({ "id": id, "report_id": "report-1", "path": path }),
    );
    assert_eq!(written["pages"], json!(2));
    let pdf = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.starts_with("%PDF"));
    assert!(text.contains("/Count 2"));

    let again = server.call_err(
        "export_report",
        json!({ "id": id, "report_id": "report-1", "path": path }),
    );
    assert!(again.contains("overwrite"), "{again}");
    let unknown = server.call_err(
        "export_report",
        json!({ "id": id, "report_id": "report-9", "path": path, "overwrite": true }),
    );
    assert!(unknown.contains("report-1"), "{unknown}");

    // SVG and PNG write a file per page, numbered next to the path asked for.
    let svg_path = server.library.path().join("westermarkt-report.svg");
    let svgs = server.call_ok(
        "export_report",
        json!({ "id": id, "report_id": "report-1", "format": "svg", "path": svg_path }),
    );
    assert_eq!(svgs["files"].as_array().unwrap().len(), 2);
    let first =
        std::fs::read_to_string(server.library.path().join("westermarkt-report-1.svg")).unwrap();
    assert!(first.contains("<svg"));
    assert!(server
        .library
        .path()
        .join("westermarkt-report-2.svg")
        .exists());

    let png_path = server.library.path().join("westermarkt-report.png");
    server.call_ok(
        "export_report",
        json!({ "id": id, "report_id": "report-1", "format": "png", "dpi": 72, "path": png_path }),
    );
    let png = std::fs::read(server.library.path().join("westermarkt-report-1.png")).unwrap();
    assert_eq!(&png[1..4], b"PNG");

    let mismatch = server.call_err(
        "export_report",
        json!({ "id": id, "report_id": "report-1", "format": "png", "path": path, "overwrite": true }),
    );
    assert!(mismatch.contains(".png"), "{mismatch}");
    let numbered_again = server.call_err(
        "export_report",
        json!({ "id": id, "report_id": "report-1", "format": "svg", "path": svg_path }),
    );
    assert!(
        numbered_again.contains("westermarkt-report-1.svg"),
        "{numbered_again}"
    );
}

#[test]
fn export_dxf_follows_the_sheet_marks_and_its_section_view() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    server.call_ok(
        "add_highlight",
        json!({
            "id": id,
            "label": "Plot",
            "outline": "dashed",
            "shape": { "kind": "circle", "center": { "lon": 4.8840, "lat": 52.3745 }, "radius_m": 20.0 },
        }),
    );
    server.call_ok(
        "set_section",
        json!({
            "id": id,
            "from": { "lon": 4.8825, "lat": 52.3745 },
            "to": { "lon": 4.8855, "lat": 52.3745 },
        }),
    );
    let out = TempDir::new().unwrap();

    let plan_path = out.path().join("plan.dxf");
    server.call_ok(
        "export",
        json!({ "id": id, "format": "dxf", "path": plan_path }),
    );
    let plan = std::fs::read_to_string(&plan_path).unwrap();
    assert!(plan.contains("\nOA-HIGHLIGHT-DASHED-ACCENT\n") && plan.contains("\nPlot\n"));
    assert!(plan.contains("\nOA-SECTION-LINE\n") && plan.contains("\nOA-BUILDING\n"));

    server.call_ok(
        "set_look",
        json!({ "id": id, "view": { "mode": "section" } }),
    );
    let section_path = out.path().join("section.dxf");
    server.call_ok(
        "export",
        json!({ "id": id, "format": "dxf", "path": section_path }),
    );
    let section = std::fs::read_to_string(&section_path).unwrap();
    assert!(section.contains("\nOA-SECTION-CUT\n") && section.contains("\nOA-SECTION-GROUND\n"));
    assert!(
        !section.contains("\nOA-SITE\n"),
        "a section has no plan in it"
    );
}

#[test]
fn export_dxf_holds_the_shown_layers_unless_asked_for_all() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let out = TempDir::new().unwrap();
    let declared = |path: &std::path::Path| {
        std::fs::read_to_string(path)
            .unwrap()
            .matches("\n  0\nLAYER\n")
            .count()
    };

    let shown_path = out.path().join("shown.dxf");
    server.call_ok(
        "export",
        json!({ "id": id, "format": "dxf", "path": shown_path }),
    );
    let all_path = out.path().join("all.dxf");
    server.call_ok(
        "export",
        json!({ "id": id, "format": "dxf", "dxf_layers": "all", "path": all_path }),
    );
    assert!(declared(&shown_path) < declared(&all_path));

    let unknown = server.call_err(
        "export",
        json!({ "id": id, "format": "dxf", "dxf_layers": "some", "path": out.path().join("x.dxf") }),
    );
    assert!(unknown.contains("some"), "{unknown}");
}

#[test]
fn export_writes_the_whole_series_into_a_folder_as_the_app_does() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let out = TempDir::new().unwrap();

    let written = server.call_ok(
        "export",
        json!({ "id": id, "format": "dxf", "series": true, "folder": out.path() }),
    );
    let files = written["files"].as_array().unwrap();
    let drawings: usize = openarchi_core::catalog::builtin_diagrams()
        .iter()
        .map(|diagram| diagram.drawings().len())
        .sum();
    assert_eq!(files.len(), drawings);
    assert!(
        files
            .iter()
            .any(|file| file.as_str().unwrap().ends_with("-figure-ground.dxf")),
        "{written}"
    );
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), drawings);

    let again = server.call_err(
        "export",
        json!({ "id": id, "format": "dxf", "series": true, "folder": out.path() }),
    );
    assert!(again.contains("overwrite"), "{again}");
    let no_folder = server.call_err(
        "export",
        json!({ "id": id, "format": "dxf", "series": true }),
    );
    assert!(no_folder.contains("folder is needed"), "{no_folder}");
    let no_path = server.call_err("export", json!({ "id": id, "format": "dxf" }));
    assert!(no_path.contains("path is needed"), "{no_path}");
}

#[test]
fn export_refuses_a_relative_path() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err(
        "export",
        json!({ "id": id, "format": "svg", "path": "site.svg" }),
    );
    assert!(message.contains("absolute"), "{message}");
}

#[test]
fn render_preview_returns_a_png_image() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let result = server.call("render_preview", json!({ "id": id, "width_px": 400 }));

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["content"][0]["type"], json!("image"));
    assert_eq!(result["content"][0]["mimeType"], json!("image/png"));
}

#[test]
fn sky_view_draws_on_an_old_site_and_solar_radiation_asks_for_a_fresh_fetch() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let text_of = |result: &Value| -> Vec<String> {
        result["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|block| block["text"].as_str().map(str::to_string))
            .collect()
    };

    server.call_ok("set_look", json!({ "id": id, "diagram_id": "sky-view" }));
    let sky = server.call("render_preview", json!({ "id": id, "width_px": 300 }));
    server.call_ok(
        "set_look",
        json!({ "id": id, "diagram_id": "solar-radiation" }),
    );
    let solar = server.call("render_preview", json!({ "id": id, "width_px": 300 }));

    assert_eq!(sky["isError"], json!(false), "{sky}");
    assert!(!text_of(&sky).iter().any(|text| text.contains("sky view")));
    // The fixture was fetched before a fetch kept the typical year.
    assert_eq!(solar["isError"], json!(false), "{solar}");
    assert!(text_of(&solar)
        .iter()
        .any(|text| text.contains("solar radiation") && text.contains("Fetch the site again")));
}

#[test]
fn export_writes_every_format() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let out = TempDir::new().unwrap();

    for format in ["svg", "pdf", "png", "dxf"] {
        let path = out.path().join(format!("westermarkt.{format}"));
        server.call_ok(
            "export",
            json!({ "id": id, "format": format, "path": path }),
        );
        assert!(path.metadata().unwrap().len() > 0, "{format} is empty");
    }
}
