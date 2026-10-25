//! Writes what the app's dev-only IPC mock serves: the catalog, the
//! Westermarkt fixture's area and layers, every diagram in every theme as
//! an SVG preview and a PNG thumbnail, the ratios and analysis, and the report
//! blocks drawn from them. One palette made from a few swatches stands in for
//! the user's own, with what the palette editor asks for. With it the front
//! end runs in a plain
//! browser, which is how the screenshots in docs/screenshots are taken.
//!
//! cargo run --release -p openarchi-core --example app_fixtures   # writes app/dev-fixtures/

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use openarchi_core::analysis::{self, Part, PARTS};
use openarchi_core::area::{Area, LonLat};
use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::export::render_png;
use openarchi_core::palette::{self, PaletteCheck, PaletteDraft, RoleChoice};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::report::{block_svg, Block, Content, RatiosStyle, ReportData};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::section::{SectionLine, DEFAULT_DEPTH_M};
use openarchi_core::sheet::Sheet;
use openarchi_core::sources::cache::TileCache;
use openarchi_core::sources::grid::Tile;
use openarchi_core::theme::Theme;
use openarchi_core::view::{View, ViewMode};
use openarchi_core::{ratios, selection};
use serde_json::json;

const FIXTURE: &str = "tests/fixtures/westermarkt.scene.json";
const RECORDED_ANALYSIS: &str = "tests/fixtures/analysis-cache/tiles-v1";
/// Noise, air, flood, slope, population and dwellings for the inner 120 m of
/// the same site, so the environment diagrams have something to show.
const ENVIRONMENT_FIXTURE: &str = "tests/fixtures/westerkerk-environment.scene.json";
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
/// Wide enough for a gallery card on a Retina screen.
const THUMBNAIL_WIDTH_PX: u32 = 560;
/// The user's own palette in the mock: five colours from a palette generator.
const SAMPLE_PALETTE: &str = "f79f79, f7d08a, e3f09b, 87b6a7, 5b5941";
const SAMPLE_PALETTE_ID: &str = "westermarkt-sample";
const SAMPLE_PALETTE_NAME: &str = "Westermarkt sample";
/// Half the section line the mock's section preview is cut along.
const SECTION_HALF_LENGTH_M: f64 = 100.0;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("../../app/dev-fixtures");
    fs::create_dir_all(out_dir.join("previews")).expect("create the previews folder");
    fs::create_dir_all(out_dir.join("thumbs")).expect("create the thumbs folder");

    let text = fs::read_to_string(manifest.join(FIXTURE)).expect("read the Westermarkt fixture");
    let mut scene: Scene = serde_json::from_str(&text).expect("parse the Westermarkt fixture");
    let text = fs::read_to_string(manifest.join(ENVIRONMENT_FIXTURE))
        .expect("read the environment fixture");
    let environment: Scene = serde_json::from_str(&text).expect("parse the environment fixture");
    add_environment(&mut scene, environment);
    write_catalog(&scene, &out_dir);
    write_palette(&out_dir);
    let (_cache_dir, cache) = recorded_cache();
    write_analysis(&scene, &cache, &out_dir);
    write_report_blocks(&scene, &cache, &out_dir);

    let sheet = Sheet {
        title: "Westermarkt".to_string(),
        subtitle: "Amsterdam, around the Westerkerk".to_string(),
        ..Sheet::default()
    };
    // A diagram with variants gets a picture per variant, "air-quality-pm10--paper",
    // the names the mock asks for.
    let drawings: Vec<_> = builtin_diagrams()
        .into_iter()
        .flat_map(|diagram| {
            diagram
                .drawings()
                .into_iter()
                .map(move |(variant, _)| (diagram.clone(), variant))
        })
        .collect();
    for (diagram, variant) in drawings {
        for theme in fixture_themes() {
            let look = Look {
                diagram_id: diagram.id.clone(),
                variant: variant.clone(),
                theme_id: theme.id.clone(),
                sheet: sheet.clone(),
                ..Look::default()
            };
            let input = RenderInput {
                scene: &scene,
                diagram: &diagram,
                theme: &theme,
                look: &look,
                pins: &[],
            };
            let stem = match &variant {
                Some(variant) => format!("{}-{variant}", diagram.id),
                None => diagram.id.clone(),
            };
            let name = format!("{stem}--{}", theme.id);
            let svg = render_svg(&input).expect("render the preview");
            fs::write(out_dir.join(format!("previews/{name}.svg")), svg)
                .expect("write the preview");
            let png = render_png(&input, THUMBNAIL_WIDTH_PX).expect("render the thumbnail");
            fs::write(out_dir.join(format!("thumbs/{name}.png")), png)
                .expect("write the thumbnail");
        }
    }

    write_explode_previews(&scene, &sheet, &out_dir);
    write_section_previews(&scene, &sheet, &out_dir);
    write_ratios(&scene, &out_dir);

    println!("fixtures in {}", out_dir.display());
}

/// The built-in themes, then the sample palette, as the app lists the user's own after them.
fn fixture_themes() -> Vec<Theme> {
    let mut themes = builtin_themes();
    themes.push(sample_palette().theme);
    themes
}

/// The sample palette as the editor checks it, with the rule filling every role.
fn sample_palette() -> PaletteCheck {
    let draft = PaletteDraft {
        name: SAMPLE_PALETTE_NAME.to_string(),
        swatches: palette::parse(SAMPLE_PALETTE).expect("read the sample palette"),
        roles: RoleChoice::default(),
        typography: None,
        based_on: None,
    };
    palette::check(&draft, SAMPLE_PALETTE_ID).expect("build the sample palette")
}

/// What the palette editor asks for: the swatches an import finds, the check
/// of the sample, and of every palette a draft of it and its unchanged check,
/// "palette/draft--paper.json" and "palette/check--paper.json".
fn write_palette(out_dir: &Path) {
    let folder = out_dir.join("palette");
    fs::create_dir_all(&folder).expect("create the palette folder");
    let swatches = palette::parse(SAMPLE_PALETTE).expect("read the sample palette");
    fs::write(
        folder.join("swatches.json"),
        serde_json::to_string(&swatches).expect("serialise the swatches"),
    )
    .expect("write the swatches");
    fs::write(
        folder.join("check.json"),
        serde_json::to_string(&sample_palette()).expect("serialise the check"),
    )
    .expect("write the check");
    for theme in fixture_themes() {
        let draft = palette::draft_from(&theme, &theme.name);
        let unchanged = palette::check_copy(&draft, &theme.id, &theme).expect("check the draft");
        fs::write(
            folder.join(format!("check--{}.json", theme.id)),
            serde_json::to_string(&unchanged).expect("serialise the check"),
        )
        .expect("write the check");
        fs::write(
            folder.join(format!("draft--{}.json", theme.id)),
            serde_json::to_string(&draft).expect("serialise the draft"),
        )
        .expect("write the draft");
    }
}

/// The ratios in every theme, as `get_ratios` returns them, "ratios/paper.json".
fn write_ratios(scene: &Scene, out_dir: &Path) {
    let folder = out_dir.join("ratios");
    fs::create_dir_all(&folder).expect("create the ratios folder");
    for theme in fixture_themes() {
        let ratios = ratios::compute(scene, &theme).expect("work out the ratios");
        fs::write(
            folder.join(format!("{}.json", theme.id)),
            serde_json::to_string(&ratios).expect("serialise the ratios"),
        )
        .expect("write the ratios");
    }
}

/// The explode view with its default plates in every theme, "explode--paper",
/// so the mock can show plates to drag. It cannot reorder them; the app can.
fn write_explode_previews(scene: &Scene, sheet: &Sheet, out_dir: &Path) {
    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == "context")
        .expect("the context diagram");
    for theme in fixture_themes() {
        let look = Look {
            diagram_id: diagram.id.clone(),
            theme_id: theme.id.clone(),
            sheet: sheet.clone(),
            view: View {
                mode: ViewMode::Explode,
                ..View::default()
            },
            ..Look::default()
        };
        let input = RenderInput {
            scene,
            diagram,
            theme: &theme,
            look: &look,
            pins: &[],
        };
        let svg = render_svg(&input).expect("render the explode preview");
        fs::write(
            out_dir.join(format!("previews/explode--{}.svg", theme.id)),
            svg,
        )
        .expect("write the explode preview");
    }
}

/// The section view in every theme, "section--paper", along one line west to
/// east through the site centre, which the mock shows whatever line is drawn.
/// The fixture predates kept terrain, so its ground is flat.
fn write_section_previews(scene: &Scene, sheet: &Sheet, out_dir: &Path) {
    let diagrams = builtin_diagrams();
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == "context")
        .expect("the context diagram");
    let (x, y) = scene.origin;
    let line = SectionLine {
        start: (x - SECTION_HALF_LENGTH_M, y),
        end: (x + SECTION_HALF_LENGTH_M, y),
        depth_m: DEFAULT_DEPTH_M,
        scale: None,
        exaggeration: 1,
    };
    for theme in fixture_themes() {
        let look = Look {
            diagram_id: diagram.id.clone(),
            theme_id: theme.id.clone(),
            sheet: sheet.clone(),
            view: View {
                mode: ViewMode::Section,
                ..View::default()
            },
            section: Some(line.clone()),
            ..Look::default()
        };
        let input = RenderInput {
            scene,
            diagram,
            theme: &theme,
            look: &look,
            pins: &[],
        };
        let svg = render_svg(&input).expect("render the section preview");
        fs::write(
            out_dir.join(format!("previews/section--{}.svg", theme.id)),
            svg,
        )
        .expect("write the section preview");
    }
}

/// A cache holding the answers recorded in tests/fixtures/analysis-cache, so
/// no service is asked.
fn recorded_cache() -> (tempfile::TempDir, TileCache) {
    let cache_dir = tempfile::TempDir::new().expect("a temporary cache");
    let cache = TileCache::at(cache_dir.path());
    let recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join(RECORDED_ANALYSIS);
    for dataset in fs::read_dir(&recorded).expect("the recorded analysis answers") {
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
            // Written fresh, so the cache never takes a recorded answer for a stale one.
            cache.write(&name, tile, &bytes).expect("seed the cache");
        }
    }

    (cache_dir, cache)
}

/// Every analysis part in every theme, as `get_analysis` returns it.
fn write_analysis(scene: &Scene, cache: &TileCache, out_dir: &Path) {
    let folder = out_dir.join("analysis");
    fs::create_dir_all(&folder).expect("create the analysis folder");

    for theme in fixture_themes() {
        for part in PARTS {
            let sections =
                analysis::part(part, scene, None, &theme, cache).expect("draw the analysis part");
            let part_name = serde_json::to_value(part).expect("a part name");
            let file = folder.join(format!(
                "{}--{}.json",
                part_name.as_str().expect("text"),
                theme.id
            ));
            fs::write(file, serde_json::to_string(&sections).expect("serialise"))
                .expect("write the part");
        }

        // The mock serves this for any sheet with a sub-selection.
        let sheet_area = selection::framed(scene, &sheet_area()).expect("frame the sheet area");
        let sections = analysis::part(Part::Site, scene, Some(&sheet_area), &theme, cache)
            .expect("draw the sheet area's site part");
        let file = folder.join(format!("site-area--{}.json", theme.id));
        fs::write(file, serde_json::to_string(&sections).expect("serialise"))
            .expect("write the part");
    }
}

/// 60 m around the Westerkerk, inside the Westermarkt fixture.
fn sheet_area() -> Area {
    Area::Circle {
        center: LonLat {
            lon: 4.8840,
            lat: 52.3745,
        },
        radius_m: 60.0,
    }
}

/// The environment layers, dwellings and credits of `environment` on `scene`.
/// Both fixtures are centred on the Westerkerk, so their frames match.
fn add_environment(scene: &mut Scene, environment: Scene) {
    let mut units = HashMap::new();
    for feature in environment.features {
        if feature.layer == Layer::Building {
            units.insert(feature.attributes.source_id.clone(), feature.attributes);
        } else if !matches!(feature.layer, Layer::Water | Layer::Contour) {
            scene.features.push(feature);
        }
    }
    for building in scene
        .features
        .iter_mut()
        .filter(|feature| feature.layer == Layer::Building)
    {
        if let Some(attributes) = units.get(&building.attributes.source_id) {
            building.attributes.dwellings = attributes.dwellings;
            building.attributes.floor_area_m2 = attributes.floor_area_m2;
        }
    }
    let known: BTreeSet<String> = scene.credits.iter().map(|c| c.name.clone()).collect();
    let new_credits: Vec<_> = environment
        .credits
        .into_iter()
        .filter(|credit| !known.contains(&credit.name))
        .collect();
    scene.credits.extend(new_credits);
    scene.measured = environment.measured;
}

/// The catalog as the app's `get_catalog` returns it, plus the fixture's area and layers.
fn write_catalog(scene: &Scene, out_dir: &Path) {
    let layers: Vec<_> = Layer::ALL
        .iter()
        .map(|layer| json!({ "id": layer, "label": layer.label() }))
        .collect();
    let present: BTreeSet<Layer> = scene.features.iter().map(|feature| feature.layer).collect();
    let fixture = json!({
        "catalog": {
            "diagrams": builtin_diagrams(),
            "themes": fixture_themes(),
            "warnings": [],
            "layers": layers,
        },
        "area": scene.area,
        "credits": scene.credits,
        "layers_present": present,
    });
    fs::write(
        out_dir.join("fixture.json"),
        serde_json::to_string(&fixture).expect("serialise the fixture"),
    )
    .expect("write the fixture");
}

/// Every ratios group in every chart style, and the facts and notes of every
/// analysis section, in every theme, as `render_report_block` draws them at
/// the size the library starts them at: "ratios-land_use-bars--paper.svg",
/// "figure-wind-wind-roses--paper.svg", "facts-site--paper.svg",
/// "notes-wind--paper.svg". Sheets are not written: the mock takes them from
/// the previews.
fn write_report_blocks(scene: &Scene, cache: &TileCache, out_dir: &Path) {
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
        let data = ReportData::gather(scene, &look, &[], &diagrams, &theme, cache, contents)
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
