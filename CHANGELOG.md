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

### Sites

- Fetching a site again after moving, resizing or reshaping it only downloads what is new. A 300 m site nudged by 50 m comes back in under half a second.
- Downloaded map data is kept for 30 days and shared between projects, so a site next to one you already fetched is quick too. The cache stays under 1 GB.
- A large site downloads faster: a 1 km radius site takes about 12 s instead of about 26 s.

### Your work

- Project files are about six times smaller, and large sites open in a fraction of the time and memory. Projects from 0.1.0 still open and are stored the new way on their next save.

### Text on the sheet

- The title and subtitle can be set in any of the three bundled fonts (Inter, Source Serif 4, IBM Plex Mono), bold or italic, at a size in points, aligned left, centre or right, in any colour and opacity.
- Drag the title or subtitle anywhere on the sheet. The legend moves up into the space it left, and the toolbar's reset button puts it back in the automatic layout.
- Press T to add your own text box. Double-click any text to edit it in place, use the arrow keys to nudge it (Shift for 10 mm), and Delete removes a text box. Every change undoes with ⌘Z.
- Bold and italic print with real bold and italic faces in PDF and PNG, not a slanted copy.
- An assistant can add, restyle, move and delete text through the new `edit_text` tool.

### Colours

- Every colour in the inspector opens a picker with a saturation square, hue and opacity sliders, hex and RGB fields, the theme's palette and your recent colours. All of it works from the keyboard.
- A text colour picked from the theme's palette stays linked to the theme, so it changes with the theme.

### Under the hood

- Project files move to format 2. Files from 0.1.0 open as before, with their title in the automatic layout.

Sheets can now show the site in three dimensions and the sun over it.

### Views

- The View section in the inspector switches a sheet between Plan, Axonometric and Explode, and every view exports to SVG, PDF and PNG like a plan.
- Axonometric tilts the site and stands every building up at its measured height. Roofs keep the diagram's colours, so building age, height and use read on the roofs. Walls are lit by the sun at the sheet's date and time, buildings cast soft shadows, and trees stand up as a trunk and a crown. A slider turns the site, and the north arrow turns with it.
- Buildings with no measured height stand at 9 m and are greyed, so the guess never passes for data.
- Explode stacks several diagrams of the site as tilted plates with dotted guides down the sides and a label on each plate. Add, remove and reorder the plates in the inspector, and switch on Stacked close for a tighter stack. The stack shrinks to fit the paper.

### Sun

- Two new diagrams. Sun path draws the sun's arcs on 21 June, the equinox and 21 December over the site, with hour marks and the range of the sun through the year. Shadow draws the shadows of buildings and trees at a chosen date and time.
- The date and time default to 21 March at 13:00, the equinox near solar noon, and can be changed in the inspector or by an assistant.

### Assistants

- `set_look` takes `view` and `sun`, so an assistant can switch views, pick the plates and set the moment for the sun diagrams.

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
