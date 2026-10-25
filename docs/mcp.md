# Using openarchi from an assistant

openarchi comes with `openarchi-mcp`, a small program that lets an assistant such as Claude work in your openarchi projects. It speaks the Model Context Protocol (MCP), the standard way assistants connect to tools on your computer. Once it is connected, you can ask for a site analysis in plain words and the assistant does the clicking for you.

## What can I ask for?

Anything you would do in the app, one request at a time or all at once. For example:

> Make a figure-ground of 200 m around Westermarkt 20 in Amsterdam, A3 landscape, and export it as a PDF to my Desktop.

The assistant finds the address, creates a project with a circle around it, downloads the site data, picks the diagram and theme, looks at a preview to check the result, and writes the file. Other things that work well:

- "Which buildings around the Oude Kerk are older than 1700? Show me the building age diagram."
- "Summarise this site: how many buildings, how tall, what are they used for?"
- "How much of this site is built over, and what is the floor area ratio of the parcels around the square?"
- "Where does the winter wind come from on this site, and which months are comfortable to sit outside?"
- "Design a theme in warm brick and sage tones and use it for all my Rotterdam projects."
- "Export the Westermarkt project as DXF for AutoCAD."
- "Export my Westermarkt report as a PDF to the Desktop."
- "Save each page of the Westermarkt report as a PNG for the slides."

The assistant can also see the sheet it is working on, because the server renders a preview image for it. That is what lets it fix a cramped title or a colour that does not read before you ever open the file.

## Does it use my real projects?

Yes. The server uses the same library folder as the app, so a project the assistant makes shows up in the app's project list, and it can open the projects you made yourself. Nothing is lost by letting it edit: every save keeps the previous version as a backup, and deleting moves a project to Recently Deleted, where the app or the assistant can bring it back.

The library lives in `~/Library/Application Support/openarchi` on macOS and `%APPDATA%\openarchi` on Windows. Set `OPENARCHI_LIBRARY_DIR` to point the server at a different folder, for example to try it out without touching your projects.

## Where is the program?

Every installer puts `openarchi-mcp` next to the app's own program, so installing openarchi installs the server too.

- **Mac:** `/Applications/openarchi.app/Contents/MacOS/openarchi-mcp`, if you dragged openarchi into Applications.
- **Windows:** `openarchi-mcp.exe` in the folder openarchi was installed to. That is `%LOCALAPPDATA%\openarchi` for the setup `.exe` and `C:\Program Files\openarchi` for the `.msi`, unless you picked another folder while installing.

When you build from source, `cargo build --release -p openarchi-mcp` puts it at `target/release/openarchi-mcp` (`openarchi-mcp.exe` on Windows).

## How do I connect Claude Code?

Run this once, with the full path to the program:

```sh
claude mcp add openarchi -- /path/to/openarchi-mcp
```

Start a new Claude Code session and `/mcp` lists openarchi with its tools.

## How do I connect Claude Desktop?

Open Settings, then Developer, then Edit Config. That opens `claude_desktop_config.json`. Add openarchi under `mcpServers` and restart Claude Desktop:

```json
{
  "mcpServers": {
    "openarchi": {
      "command": "/path/to/openarchi-mcp"
    }
  }
}
```

On Windows, write the path with double backslashes, like `"C:\\Program Files\\openarchi\\openarchi-mcp.exe"`.

## Which tools does the assistant get?

| Tool | What it does |
| --- | --- |
| `search_places` | Finds addresses and places in the Netherlands |
| `list_projects`, `get_project` | Lists projects, and shows one with a summary of its site |
| `create_project` | Makes a project from a circle, rectangle or polygon |
| `fetch_site` | Downloads buildings, roads, water, green and terrain into the project |
| `list_diagrams`, `list_themes` | Shows the diagrams, with their group and variants, and the themes to choose from |
| `set_look` | Changes the diagram and its variant, theme, sheet, single layers, the view and the sun's date and time, on the first sheet tab or the one `tab` names |
| `list_sheets`, `add_sheet` | Lists a project's sheet tabs, and adds one copied from another, optionally framing a smaller area inside the site that it zooms to |
| `list_pins`, `add_pin`, `move_pin`, `remove_pin` | Marks landmarks with a pin, star, flag or dot and a label, shown on every sheet that does not hide pins |
| `set_section` | Draws, moves, flips or clears a sheet's section line, from A to A' in longitude and latitude, and sets how far beyond it the section shows buildings and trees, its scale and how much taller heights are drawn |
| `edit_text` | Adds, restyles, moves or deletes text on the sheet: the title, the subtitle and text boxes |
| `render_preview` | Renders the sheet as an image the assistant can see |
| `get_ratios` | Gives the numbers behind the site: land use, transport, building use, floors, height, age and footprint, coverage and floor area ratio per parcel, green and water, as shares that add up to 100 % |
| `get_environment` | Gives the site's road and total noise, yearly NO₂, PM10 and PM2.5, deepest flood water and the share of the site it reaches, steepest slope, residents, households and dwellings, each with its source year; with `tab`, of that sheet's smaller area |
| `get_neighbourhood` | Counts the businesses, facilities, car and bicycle parking, and transit stops within a 5, 10 and 15 minute walk of the site, by group |
| `get_analysis` | Gives the site analysis: address, municipality and province, a site summary, the climate from a PVGIS typical year (temperature, outdoor comfort as UTCI hours per stress category in the sun and in the shade, wind by season, sun, humidity), 30 years of rain from the nearest KNMI station, and design notes for each section; with `tab`, the site summary of that sheet's smaller area, while location, climate and rain stay on the whole site |
| `export` | Writes SVG, PDF, PNG or DXF |
| `list_reports` | Lists the reports laid out in the app, with each page's paper and the blocks placed on it |
| `export_report` | Writes one report as a PDF with a page per report page, or as an SVG or PNG per page, drawn from the project's current data |
| `create_theme` | Saves a new theme, which the app picks up too |
| `import_palette` | Turns a palette generator's export (hex codes, JSON, SVG swatches, XML or a link) into a theme, with the roles it chose and any contrast warnings; a palette whose text cannot be read is not saved, as in the app |
| `list_user_palettes` | Lists the palettes the user made or imported, with their swatches |
| `delete_project`, `restore_project` | Moves a project to Recently Deleted and back |

## What is a variant?

Some diagrams answer one question with a choice of measure. Air quality has the variants `no2`, `pm10` and `pm25`, and noise has `road` and `all`. `list_diagrams` lists them under `variants`, each with its own layers and classes, and a diagram with variants has no layers of its own. `set_look` takes `variant` next to `diagram_id`, like `{"diagram_id": "air-quality", "variant": "pm25"}`. The variant stays while the diagram does, and a new diagram without a variant starts on its first. The ids from before the fold, such as `air-quality-pm10` and `traffic-noise`, still work and land on the variant they became.

## How does an assistant switch to a tilted view?

`set_look` takes a `view` and a `sun` next to the sheet fields, and like the rest of the look only the fields you pass change. `view.mode` is `plan`, `axonometric` (the site tilted, with buildings at their measured height), `explode` (several diagrams stacked as tilted plates) or `section` (along the line `set_section` drew). `rotation_deg` turns the site on the paper, `tilt_deg` sets how steeply the viewer looks down (15 to 80, 35 by default), `plates` lists the diagram ids to stack with the top plate first, and `stacking: true` packs the plates closer. A plate of a diagram with variants shows its first, and `"air-quality:pm25"` picks one.

`sheet.show_context` turns the neighbourhood around the site on or off: the buildings, roads, water and green outside the boundary, drawn greyed in the palette. New projects start with it on, and projects from before it start with it off and have no neighbourhood until their site is fetched again. It never enters the legend, the ratios or the analysis.

`sun` is a day and a Dutch clock time, summer time included. The sun path and shadow diagrams are drawn for it, and the axonometric uses it to light its walls and cast its shadows. It starts at 21 March, 13:00, the equinox near solar noon.

```json
{
  "id": "2026-10-24-westermarkt-3f9a",
  "view": { "mode": "explode", "plates": ["building-use", "building-age", "landscape", "context"] },
  "sun": { "month": 6, "day": 21, "hour": 17 }
}
```

## How does an assistant cut a section?

A section needs a line first, as in the app, where the Section view is greyed out until one is drawn. `set_section` takes `from` (A) and `to` (A') as longitude and latitude, both inside the site or its neighbourhood and at least 5 m apart, and `depth_m` for how far beyond the line buildings and trees are drawn (60 m to start, at most 500). The section looks to the left of the line as you walk from A to A'; `flip: true` swaps the ends to look the other way. `scale` picks the scale as N in 1:N from the same list as the app's Scale menu (100 to 20000), and `0` goes back to the largest that fits the sheet; `exaggeration` draws heights 1, 2 or 5 times taller, as the Heights buttons do. Then `set_look` with `view.mode` `section` shows it, and `render_preview` draws it. `clear: true` removes the line and puts a sheet in the section view back on its plan. `list_sheets` shows each tab's line.

```json
{ "id": "2026-10-24-westermarkt-3f9a", "from": { "lon": 4.8825, "lat": 52.3745 }, "to": { "lon": 4.8855, "lat": 52.3745 }, "depth_m": 80 }
```

## Trying it from a terminal

`scripts/mcp.sh` calls one tool and prints the answer, which is handy when working on the server. It uses `.scratch/library` unless `OPENARCHI_LIBRARY_DIR` says otherwise, and saves preview images under `.scratch/mcp/`.

```sh
scripts/mcp.sh tools/list
scripts/mcp.sh list_projects '{}'
scripts/mcp.sh --release render_preview '{"id": "2026-10-24-westermarkt-3f9a"}'
```
