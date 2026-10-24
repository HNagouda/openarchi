# Developing openarchi

openarchi is a Rust core library with two thin shells on top: a Tauri desktop app and an MCP server. Everything a user sees comes out of the core, so the app and an AI assistant always agree on what a site looks like.

## The one idea: the SVG is the truth

A sheet is rendered once, by the core, as an SVG document sized in millimetres. The app shows that exact SVG as its preview. PDF and PNG are converted from the same SVG, and the MCP server's preview is a PNG of it too. There is no second renderer that could drift, so what you see is what you export. DXF is the one exception: CAD users want real layers and real coordinates rather than paint, so it is written from the scene directly.

Styling is split in two on purpose. A **diagram** decides which layers show and how buildings are classified (by age, use, height), and knows nothing about colour. A **theme** decides how things look, using named palette roles like `@figure` and `@muted` instead of raw colours. Any diagram works with any theme, and changing a theme restyles the whole series at once. `crates/core/src/diagram.rs` spells out the exact order in which the two combine.

## Layout

```text
crates/core/            the library: no window, no async
  src/area.rs           the site the user drew, in lon/lat
  src/scene.rs          a fetched site: every feature in RD New metres, and the Layer list
  src/site.rs           build_scene: fetch every source in parallel, clip, join
  src/sources/          one module per data service (BGT, BAG, BRK, AHN) plus shared paging
  src/crs.rs            WGS84 to RD New and back
  src/geocode.rs        address search through PDOK Locatieserver
  src/diagram.rs        what a sheet shows
  src/theme.rs          how a sheet looks
  src/sheet.rs          paper, scale, frame and furniture
  src/render/           scene + diagram + theme + sheet -> SVG
  src/export/           SVG -> PDF and PNG, scene -> DXF
  src/catalog.rs        built-in diagrams and themes, and the user's own themes
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
| A new diagram | a JSON file in `crates/core/assets/diagrams/`, listed in `catalog.rs` |
| A new theme | a JSON file in `crates/core/assets/themes/`, listed in `catalog.rs` |
| Sheet furniture (legend, scale bar) | `crates/core/src/render/` |
| A new export format | `crates/core/src/export/` |
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
```

To drive the core the way an assistant does:

```sh
scripts/mcp.sh list_projects '{}'
scripts/mcp.sh render_preview '{"id": "<project id>"}'   # prints the path of the PNG
```

## Saved data

Projects are JSON files with a `format_version`. When the shape changes, bump `PROJECT_FORMAT_VERSION` in `project.rs` and add a step to `library/migrate.rs`, so every older file still opens. Themes have their own `THEME_FORMAT_VERSION` for the same reason, because people share them.

#! NOTE[size, scene] : A 1 km site is about 73 MB of JSON; compress the stored scene before sites get bigger

## Building the installers

CI builds them, so you rarely need to. `dev-build.yml` and `release.yml` both call `installers.yml`, which builds a universal `.dmg` on macOS and a `.msi` and setup `.exe` on Windows. It first runs `scripts/build-mcp-sidecar.sh`, which builds `openarchi-mcp` into `app/src-tauri/binaries/`, and then merges `app/src-tauri/tauri.installer.conf.json` over the normal config. That file adds the server as a sidecar (`bundle.externalBin`), so it lands next to the app's program, and ad-hoc signs the Mac app. It is kept out of `tauri.conf.json` because a sidecar must exist before the app compiles, which would make `pnpm tauri dev` build the server first.

To build one locally on an Apple Silicon Mac:

```sh
scripts/build-mcp-sidecar.sh aarch64-apple-darwin
cd app && pnpm tauri build --target aarch64-apple-darwin --config src-tauri/tauri.installer.conf.json
```

Releases come from tags. Set the version in `Cargo.toml` and `app/src-tauri/tauri.conf.json`, give it a section in `CHANGELOG.md`, merge to `main`, then tag that commit as `v<version>` and push the tag. `release.yml` refuses a tag that disagrees with either version or has no changelog section.
