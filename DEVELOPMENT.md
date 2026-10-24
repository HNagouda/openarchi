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
  src/site/             the drawn boundary, and fetching tiles from the cache or PDOK
  src/sources/          one module per data service (BGT, BAG, BRK, AHN, OpenStreetMap), shared paging,
                        the 250 m tile grid and the tile cache
  src/crs.rs            WGS84 to RD New and back
  src/geocode.rs        address search through PDOK Locatieserver
  src/diagram.rs        what a sheet shows
  src/theme.rs          how a sheet looks
  src/sheet.rs          paper, scale, frame and furniture
  src/view.rs           plan, axonometric or explode, and the moment the sun is drawn for
  src/sun.rs            where the sun stands, from the site's position and a Dutch clock time
  src/walk.rs           walk bands from the site centre and the 15 minute neighbourhood
  src/render/           scene + diagram + theme + sheet -> SVG, in every view
  src/ratios.rs         how the site divides up (land use, coverage, floor area ratio), as CSV or an SVG of charts
  src/analysis.rs       location, site summary, climate and rain, with design notes and charts in the theme
  src/export/           SVG -> PDF and PNG, scene -> DXF
  src/catalog.rs        built-in diagrams and themes, and the user's own themes
  src/catalog/          finding a diagram by id, and the old ids of diagrams folded into variants
  src/project.rs        one site plus the look the user chose
  src/library.rs        project files, backups and Recently Deleted
  assets/               diagram and theme JSON, and the bundled OFL fonts
  examples/             fetch_site and render_samples, for working on data and looks
crates/mcp/             openarchi-mcp, a stdio MCP server over the core
app/                    the desktop app
  src-tauri/            Tauri commands, each a thin call into the core
  src/                  React front end, one folder per screen: home, site, sheet, settings, tour
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
| A new layer | `Layer` in `scene.rs` (the one list), then a style in every theme |
| A new raster from a WCS | a `Coverage` in its source module, fetched by `sources/wcs.rs`, turned into bands in `site/environment.rs` with the breaks of the diagram that draws it |
| A new diagram | a JSON file in `crates/core/assets/diagrams/` with its `group`, listed in `catalog.rs` next to its group |
| Another measure for an existing diagram | a variant in that diagram's JSON; a diagram folded into another keeps its old id in `catalog/lookup.rs` |
| A new theme | a JSON file in `crates/core/assets/themes/`, listed in `catalog.rs` |
| Sheet furniture (legend, scale bar) | `crates/core/src/render/` |
| A new export format | `crates/core/src/export/` |
| A new analysis section or design note | `crates/core/src/analysis/`: data in its own module, the section in `sections.rs`, the rule in `notes.rs` |
| A new assistant tool | `crates/mcp/src/server.rs` |
| A screen or control | `app/src/<screen>/`, with the Tauri command in `app/src-tauri/src/commands/` |

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
cargo run --release -p openarchi-core --example analysis_samples # every analysis section in every theme, target/analysis-samples/
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

The scene inside a project is stored as zstd-compressed JSON in base64 (`project/packed_scene.rs`), because it is nearly the whole file and its JSON shrinks about sixfold. The rest of the file stays readable JSON, so the project list never unpacks a scene.

Downloaded data lives in `<library>/cache/`, shared by every project. Each source is fetched in tiles of a fixed 250 m grid over RD New, and each tile is kept for 30 days, so redrawing a site, or drawing the one next door, only downloads the tiles it has not seen. The cache deletes its oldest tiles past 1 GB. The `fetch_site` example uses `target/tile-cache` instead, so working on the data layer never touches your library.
