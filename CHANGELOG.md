# Changelog

What changed in each version of openarchi, newest first.

## 0.2.0 (unreleased)

### Sites

- Click a drawn site to select it, drag it to move it, and drag its handles to resize it. A polygon's corners move one by one, and pressing the dot between two corners adds a new one.
- Delete or the bin in the side panel removes the drawn site.
- ⌘Z and ⇧⌘Z undo and redo every change to the site: drawing, moving, resizing, deleting and typing a radius.
- Switch a drawn site between circle, rectangle and polygon. It keeps its centre and its size across.
- A fit button and ⌘0 bring the map back to the drawn site.

### Looks

- openarchi has its own look, taken from a drafting room: warm paper and ink in light, a graphite table in dark. Project names and section titles are set in Source Serif, scales, paper sizes and values in IBM Plex Mono, the same type the sheets use.
- The project gallery shows each project as a print lying on a desk, with its diagram, scale and paper size underneath, and a blank sheet as the first tile for a new site.
- Settings has a fourth appearance, AMOLED, with true black surfaces. The chosen appearance is applied before the window first paints, so it no longer flashes light on launch.

### Sheets

- Every layer row shows a chevron, the same one the section headers use, so it is clear the row opens to its fill, line and opacity. The eye to hide a layer sits at the end of the row.
- Themes are picked from a compact menu, like choosing a font: the closed control shows the current theme and its colours, and the open menu lists every theme with its colours, your own themes in their own group. [ and ] still step through them.

### Ratios

- The inspector has a Ratios view next to Style: how the site divides up, as bars or pies. Land use and transport by area, building use, floors, height, year built and footprint by building, building coverage and floor area ratio by parcel, and green and water by type.
- Classes are fixed, so two sites compare directly, and height and year built use the same classes as their diagrams. Colours come from the current theme, so the numbers match the sheet.
- Floors are estimated from building height at 3 m per floor.
- Export the ratios as a CSV table or as an SVG page of charts in the sheet's theme.
- Assistants can read the same numbers with the `get_ratios` tool.

## 0.1.0 (unreleased)

The first version: draw a site anywhere in the Netherlands and turn it into a series of styled site analysis diagrams.

### Sites

- Search an address and draw a circle, rectangle or polygon of up to 4 km². The radius is a value you can type.
- openarchi fetches buildings with year built, use and height, road surfaces at their true width, water, green, trees, parcels and terrain contours from PDOK, with a progress bar.
- If one data service fails, the site still opens without that layer and says what is missing.

### Diagrams and themes

- Nine diagrams: context, figure-ground, open space, building use, building age, building height, landscape, movement and parcels. Keys 1 to 9 switch between them.
- Eight themes: Paper, Ink, Mono, Polder, Blueprint, Terracotta, Midnight and Pastel. [ and ] step through them, and every diagram changes with the theme.
- Any layer's fill, line and opacity can be changed on top of the theme.
- The filmstrip shows every diagram of the site in the current theme.

### Sheets and export

- A4 to A0 or square, portrait or landscape, at a scale from 1:500 to 1:10000, framed as a circle, a square or the drawn shape.
- The legend, north arrow, scale bar and data credits are laid out automatically and never overlap the drawing.
- ⌘E exports this diagram or the whole series as SVG, PDF, PNG or DXF. SVG and PDF keep named layers, and DXF uses real RD New coordinates.

### Your work

- Projects save as you go, and every save keeps a backup of the version before it.
- ⌘Z and ⇧⌘Z undo and redo every change to a sheet.
- Deleted projects wait in Recently Deleted until you restore them.

### Assistants

- `openarchi-mcp` lets an AI assistant create sites, switch diagrams and themes, design new themes, see a preview and export. Changes it makes show up live in the open app.
