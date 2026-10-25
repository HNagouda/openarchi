# Developing openarchi

openarchi is a Rust core library with two thin shells on top: a Tauri desktop app and an MCP server. Everything a user sees comes out of the core, so the app and an AI assistant always agree on what a site looks like.

## The one idea: the SVG is the truth

A sheet is rendered once, by the core, as an SVG document sized in millimetres. The app shows that exact SVG as its preview. PDF and PNG are converted from the same SVG, and the MCP server's preview is a PNG of it too. There is no second renderer that could drift, so what you see is what you export. DXF is the one exception: CAD users want real layers and real coordinates rather than paint, so it is written from the scene directly.

Styling is split in two on purpose. A **diagram** decides which layers show and how buildings are classified (by age, use, height), and knows nothing about colour. A **theme** decides how things look, using named palette roles like `@figure` and `@muted` instead of raw colours. Any diagram works with any theme, and changing a theme restyles the whole series at once. The app and the user docs call a theme a palette; the code, the JSON files and the MCP tools keep the name theme. `crates/core/src/diagram.rs` spells out the exact order in which the two combine.

Sheet text follows the same idea. The core writes every title, subtitle and text box as a group carrying its id, anchor and resolved style in `data-*` attributes (`crates/core/src/render/text.rs`). The app reads those back off the SVG to select, drag and edit text, so it never has to guess where the layout put the title.

## Layout

```text
crates/core/            the library: no window, no async
  src/area.rs           the site the user drew, in lon/lat
  src/scene.rs          a fetched site: every feature in RD New metres, and the Layer list
  src/site.rs           build_scene: fetch every tile, join, clip
  src/site/             the drawn boundary, the neighbourhood around it, fetching tiles from the cache or PDOK, joining and clipping them (join.rs), and the water bed and levels for sections
  src/sources/          one module per data service (BGT, BAG, BRK, AHN, the infrared photo and the
                        boundaries from PDOK, RIVM, LIWO, CBS, PVGIS, KNMI, Rijkswaterstaat,
                        OpenStreetMap), shared paging, the 250 m tile grid and the tile cache
  src/crs.rs            WGS84 to RD New and back
  src/geocode.rs        address search through PDOK Locatieserver
  src/diagram.rs        what a sheet shows
  src/theme.rs          how a sheet looks
  src/sheet.rs          paper, scale, frame and furniture
  src/view.rs           plan, axonometric, explode or section, and the moment the sun is drawn for
  src/section.rs        a sheet's section line, and section/: the ground profile, what the cut meets and the water it crosses
  src/sun.rs            where the sun stands, from the site's position and a Dutch clock time
  src/walk.rs           walk bands from the site centre and the 15 minute neighbourhood
  src/floors.rs         floors and gross floor area, estimated from roof height
  src/blocks.rs         city blocks cut from the street surfaces, with their coverage and floor space
  src/memo.rs           the last few computed grids and analysis parts, kept so a redraw of the same scene does not work them out again
  src/meteo.rs          compass sectors, the calm limit and the seasons, shared by the climate analysis and the wind
  src/grid.rs           the square grid every computed layer below lays over the site: ground and roof cells, filling shapes, values back into bands
  src/sunlight.rs       hours of direct sun on a grid over the site
  src/skyview.rs        the share of the sky each ground cell sees, from the horizon in 32 directions
  src/radiation.rs      yearly sunlight on the ground and the roofs, from the typical year kept with the scene, in classes set per site
  src/viewshed.rs       the ground in view from a sheet's viewpoint, by line of sight over buildings, woods and terrain
  src/forest.rs         the trees dense enough to block sight, sky and sun, by the FAO definition of forest
  src/wind.rs           the prevailing wind through the site in plan: wind/ solves the 2D flow and draws its lines
  src/render/           scene + diagram + theme + sheet -> SVG, in every view
  src/ratios.rs         how the site divides up (land use, coverage, floor area ratio), as CSV or an SVG of charts
  src/analysis.rs       location, site summary, climate and rain, with design notes and charts in the theme
  src/report.rs         reports: pages of blocks that point at ratios, analysis, sheets or text, drawn in a theme
  src/export/           SVG -> PDF and PNG, scene -> DXF (a plan, or a section sheet's section), scene -> laser cutting files (laser/)
  src/catalog.rs        built-in diagrams and themes, and the user's own themes
  src/palette.rs        a user's palette: a few swatches turned into a whole theme, with a contrast check
  src/catalog/          finding a diagram by id, the old ids of diagrams folded into variants, and
                        styles for the layers a saved theme is older than (missing_layers.rs)
  src/project.rs        one site plus its sheets (looks), pins, highlights and reports
  src/pin.rs            landmarks in RD New metres, shown on every sheet
  src/highlight.rs      areas marked inside the site in RD New metres, shown on every sheet
  src/selection.rs      a sheet's area, opened from a highlight: the scene cut to it
  src/library.rs        project files, backups and Recently Deleted
  src/environment.rs    noise, air, flood, slope and people as numbers, for get_environment
  src/attribution.rs    the source names under each ratio group and analysis section
  assets/               diagram and theme JSON, and the bundled OFL fonts
  examples/             fetch_site and the *_samples, *_timing and app_fixtures tools, for working on data, looks and speed (see Running it)
crates/mcp/             openarchi-mcp, a stdio MCP server over the core
app/                    the desktop app
  src-tauri/            Tauri commands, each a thin call into the core
  src/                  React front end, one folder per screen: home, site, sheet, report, settings, tour; shared lib/, ui/ and styles/; dev/ holds the browser mock
scripts/mcp.sh          call any MCP tool from the shell
```

## Reading order

1. `crates/core/src/scene.rs` and `area.rs`, for the data model.
2. `crates/core/src/diagram.rs` and `theme.rs`, for how styling resolves.
3. `crates/core/src/site.rs`, then one source such as `sources/bag.rs`.
4. `crates/core/src/render/mod.rs`, then `layout.rs` and `legend.rs`.
5. `crates/core/src/library.rs`, for how projects are kept safe.
6. `app/src-tauri/src/commands/` and `app/src/sheet/`, for how the app drives it.

## Where things go

| Change | Where |
| --- | --- |
| A new data source or country | a module in `crates/core/src/sources/`, called from `site.rs` |
| A source published in yearly editions | a function in its source module that asks the service for the newest one, called through `sources/edition.rs` from `site/editions.rs`, with the year in `Scene::editions` and `{year}` in the legend note |
| A new layer | `Layer` in `scene.rs` (the one list), then a style in every theme |
| A layer worked out from the scene when drawn, never stored | a module in `crates/core/src/`, called from `render/derived.rs` when a diagram draws its layer, and kept in `memo.rs` when it takes more than a few milliseconds |
| A new raster from a WCS | a `Coverage` in its source module, fetched by `sources/wcs.rs`, turned into bands in `site/environment.rs` (noise, air, flood, slope) or `site/greenery.rs` (trees, green, vegetation, heat) with the breaks of the diagram that draws it |
| A new diagram | a JSON file in `crates/core/assets/diagrams/` with its `group`, listed in `catalog.rs` next to its group |
| Another measure for an existing diagram | a variant in that diagram's JSON; a diagram folded into another keeps its old id in `catalog/lookup.rs` |
| A new theme | a JSON file in `crates/core/assets/themes/`, listed in `catalog.rs` |
| How a palette of swatches becomes a theme (import formats, the role rule, contrast checks, mixed colours) | `crates/core/src/palette/`; the editor is `app/src/sheet/palette/` |
| Sheet furniture (legend, scale bar) | `crates/core/src/render/` |
| Something drawn on top of every view, like pins | a module in `crates/core/src/render/`, placed with `SheetDrawing::ground` |
| Something drawn on the ground of every map, like highlights | a module in `crates/core/src/render/`, called from `map::draw` for maps whose `MapSpec::has_highlights` is set |
| A new view mode | a `ViewMode` in `view.rs`, drawn by a module in `crates/core/src/render/` that returns a `SheetDrawing`, like `render/section.rs` |
| A new export format | `crates/core/src/export/` |
| A new analysis section or design note | `crates/core/src/analysis/`: data in its own module, the section in `sections.rs` (a climate section's facts in `sections/climate_facts.rs`), the rule in `notes.rs` |
| A new kind of report block | a `Content` variant in `crates/core/src/report.rs`, drawn in `report/draw.rs`, offered in `app/src/report/library.ts` |
| A new assistant tool | the module for its area in `crates/mcp/src/` (`styles.rs`, `figures.rs`, `export.rs`, `sheets.rs` and so on), with its input in the matching `*_inputs.rs` (or beside the tool when the module has only one) and its router joined in `server.rs`; its protocol test goes in the matching file in `crates/mcp/tests/`, with the shared harness in `tests/common/mod.rs` |
| A screen or control | `app/src/<screen>/`, with the Tauri command in `app/src-tauri/src/commands/` |
| The app icon | `app/src-tauri/icons/icon.svg`; `pnpm tauri icon src-tauri/icons/icon.svg` in `app/` makes every size from it, and only the files `tauri.conf.json` lists are kept |

## Running it

You need Rust (stable), Node 22 and pnpm.

```sh
cargo test --workspace                       # everything, offline
cargo test -p openarchi-core -- --ignored    # also the live PDOK tests

cd app && pnpm install
OPENARCHI_LIBRARY_DIR=../.scratch/library pnpm tauri dev
```

`OPENARCHI_LIBRARY_DIR` points the app and the MCP server at a throwaway folder, so working on openarchi never touches your real projects. `.scratch/` is ignored by git.

To work on looks without the app, fetch a site once and render every diagram in every theme:

```sh
cargo run --release -p openarchi-core --example fetch_site -- 4.8840 52.3745 300 .scratch/site.json
cargo run --release -p openarchi-core --example render_samples   # writes target/render-samples/
cargo run --release -p openarchi-core --example palette_samples  # every diagram in a palette made from swatches, target/render-samples/palette/
cargo run --release -p openarchi-core --example laser_samples    # laser files and a PNG of each bed, target/laser-samples/
cargo run --release -p openarchi-core --example dxf_samples -- <library folder> <project id>  # a DXF per diagram, a section and a highlight's sheet, target/dxf-samples/
cargo run --release -p openarchi-core --example analysis_samples # every analysis section in every theme, and the A4 PDF, target/analysis-samples/
cargo run --release -p openarchi-core --example section_samples -- .scratch/site.json westermarkt  # a section in every theme, target/render-samples/sections/
cargo run --release -p openarchi-core --example report_samples   # a sample report in a few palettes, as PDF and PNGs, target/report-samples/
cargo run --release -p openarchi-core --example highlight_samples # highlights in every theme and view, target/render-samples/highlights/
cargo run --release -p openarchi-core --example pin_samples       # every kind of pin in every theme and view, target/render-samples/pins/
cargo run --release -p openarchi-core --example netherlands_outline  # rebuilds the outline of the country on the locator map, assets/places/
```

When the app feels slow, time it before changing anything. `render_timing` loads a saved project and prints, per diagram, how long the preview takes to draw, how large its SVG is and how long its thumbnail takes. The SVG size matters as much as the time: the app parses and paints every byte on every change.

```sh
cargo run --release -p openarchi-core --example render_timing -- <library folder> <project id>
VIEW=axonometric SELECT_RADIUS=150 DUMP_DIR=.scratch/svgs cargo run --release -p openarchi-core --example render_timing -- <library folder> <project id>
```

Then time what WebKit does with those SVGs, which on a large site is most of an edit. `scripts/paint-timing.swift` (macOS) shows each dumped SVG in a WebKit view the size of the sheet and prints how long it takes to appear; an empty sheet takes about 33 ms. On a 1.1 km site at 1:5650 the core writes a sheet in about 75 ms and WebKit takes 250 to 370 ms to paint it. What costs is the amount of geometry and the number of elements that carry a fill pattern, not the element count as such, which is why each layer draws all its areas as one path.

```sh
swiftc -O scripts/paint-timing.swift -o target/paint-timing
target/paint-timing .scratch/svgs/*.svg
```

`compute_timing` times the layers worked out when drawn (the woods that block, the hours of sun, the sky view, the solar radiation, the city blocks, the wind solve and the view from the site centre) on scenes written by `fetch_site`, so a slow one shows up before it reaches the app. It times the work itself; the app keeps each result between redraws (`memo.rs`).

`library_timing` times the project list, the check whether someone else saved a project, and a save the way the app saves after an edit. It saves into the library's backups, so give it a copy: `cargo run --release -p openarchi-core --example library_timing -- <copy of a library folder>`.

```sh
cargo run --release -p openarchi-core --example compute_timing -- .scratch/site.json
```

To work on the front end in a plain browser, render the fixtures once and start Vite on its own. Outside Tauri, `app/src/dev/mockIpc.ts` answers every command from those fixtures, so the gallery, the sheet and the site step all work without the Rust side. This is how the screenshots in `docs/screenshots/` are taken. The mock is loaded only in dev, so it never reaches a release build.

```sh
cargo run --release -p openarchi-core --example app_fixtures   # writes app/dev-fixtures/
cd app && pnpm dev                                             # then open http://localhost:1420
```

To drive the core the way an assistant does:

```sh
scripts/mcp.sh list_projects '{}'
scripts/mcp.sh render_preview '{"id": "<project id>"}'   # prints the path of the PNG
```

## Saved data

Projects are JSON files with a `format_version`. When the shape changes, bump `PROJECT_FORMAT_VERSION` in `project.rs` and add a step to `library/migrate.rs`, so every older file still opens. Themes have their own `THEME_FORMAT_VERSION` for the same reason, because people share them.

The scene inside a project is stored as zstd-compressed JSON in base64 (`project/packed_scene.rs`), because it is nearly the whole file and its JSON shrinks five- to sixfold as stored text. The rest of the file stays readable JSON, so the project list never unpacks a scene. The scene is written last, so the list and the check before every save read only the first few kB of each file; a file saved before that reads whole and is rewritten in the new order on its next save.

Downloaded data lives in `<library>/cache/`, shared by every project. Each source is fetched in tiles of a fixed 250 m grid over RD New, and each tile is kept for 30 days, so redrawing a site, or drawing the one next door, only downloads the tiles it has not seen. The cache deletes its oldest tiles past 1 GB. It also keeps which edition of RIVM, CBS, the infrared photo and the Rijkswaterstaat bed survey is newest, for a day, and each edition's tiles in a folder of their own, so a new year is downloaded fresh. The `fetch_site` example uses `target/tile-cache` instead, so working on the data layer never touches your library.
