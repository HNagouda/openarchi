# Changelog

What changed in each version of openarchi, newest first.

## 0.2.0 (unreleased)

### Sites

- Click a drawn site to select it, drag it to move it, and drag its handles to resize it. A polygon's corners move one by one, and pressing the dot between two corners adds a new one.
- Delete or the bin in the side panel removes the drawn site.
- ⌘Z and ⇧⌘Z undo and redo every change to the site: drawing, moving, resizing, deleting and typing a radius.
- Switch a drawn site between circle, rectangle and polygon. It keeps its centre and its size across.
- A fit button and ⌘0 bring the map back to the drawn site.
- The place search and the Circle, Rectangle and Polygon tools sit at the top of the side panel instead of floating over the map, so they never cover the site you are drawing. The tools are named, and C, R and P still pick them.

### Looks

- openarchi has its own look, taken from a drafting room: warm paper and ink in light, a graphite table in dark. Project names and section titles are set in Source Serif, scales, paper sizes and values in IBM Plex Mono, the same type the sheets use.
- The project gallery shows each project as a print lying on a desk, with its diagram, scale and paper size underneath, and a blank sheet as the first tile for a new site.
- Settings has a fourth appearance, AMOLED, with true black surfaces. The chosen appearance is applied before the window first paints, so it no longer flashes light on launch.

### Sheets

- Every layer row shows a chevron, the same one the section headers use, so it is clear the row opens to its fill, line and opacity. The eye to hide a layer sits at the end of the row.
- Themes are now called palettes in the app.
- Palettes are picked from a compact menu, like choosing a font: the closed control shows the current palette and its colours, and the open menu lists every palette with its colours, your own palettes in their own group. [ and ] still step through them.
- Diagrams are picked from the filmstrip along the bottom only. The inspector no longer repeats them, which leaves it room for the palette, layers, view and sheet.

### Sheet tabs and pins

- A project holds several sheets, each a tab along the bottom of the workspace with its own diagram, palette, view and text. The + adds one, a double-click renames it, dragging reorders it, and the menu on the open tab duplicates or deletes it. Deleting says so with an Undo, and every tab change undoes like any other edit. Undoing or redoing a tab move keeps the sheet you were on in view.
- Tabs hold sheets only. Reports open from the top bar, and the analysis sits in the inspector.
- Ctrl+Tab and Ctrl+Shift+Tab, or Ctrl+PageDown and Ctrl+PageUp, step through the tabs, and on a Mac ⌘⌥→ and ⌘⌥← do too.
- A sheet can show part of the site: highlight that part and open it as a sheet (see Highlights below). The sheet clips to that area at the largest round scale it fits, and its ratios describe only that area. With the neighbourhood on, the rest of the site is drawn muted around it.
- On a sheet that shows part of the site, the measured road and total noise, NO₂, PM10, PM2.5 and flood depth describe that part too. A fetch now keeps those rasters, cut to the site, which makes a project file about 1 % bigger. The steepest slope stays the whole site's, because its 0.5 m terrain raster would be far bigger than everything else together. Sites fetched before this keep their whole-site figures and say so in the site analysis; fetching the site again measures each sheet's own area. Assistants get a sheet's figures from `get_environment` with `tab`, and `whole_site` names the figures that still describe the whole site.
- Landmarks can be pinned. Drag a pin, star, flag or dot from the tab bar onto the sheet, type its label, and it shows on every sheet, in plan, axonometric and explode views, in the palette's accent colour. Click a pin to change its marker or delete it, drag it to move it, and the eye in the tab bar hides the pins on one sheet. Pins export to DXF on their own OA-PINS layer. The numbered marker is gone from the tab bar, because a number in a red disc read like an error badge; projects that already have numbered pins still show them.
- Assistants can list and add sheets and list, add, move and remove pins, and work on any tab.
- Projects made before tabs open as one sheet, unchanged.

### Highlights and pin sizes

- Mark part of the site as a highlight: the plot, a proposed house, a square to redesign. The rectangle, circle and polygon in the tab bar draw one on the sheet, a rectangle dragged from corner to corner, a circle from its centre outwards, a polygon a click per corner and Return or a double-click to finish. H turns the tool on, and Return on a shape button places one of a house's size in the middle of the view.
- A highlight is drawn by the core as a light tint and an outline, solid for what is there, dashed for what is proposed or dotted for what is studied, with its label above it and in the legend. It is in the palette's accent until you pick another colour from the swatch in its bar: a palette colour, which follows the palette, or any hex. The legend swatch shows its colour and its line. It can add its area in m² to the label, and "dim the rest" fades everything outside it a little. In an axonometric it lies on the ground under the buildings, and the buildings standing in it keep their colour while the rest fade. In an explode view it lies on every plate and is named on the top one, where the pins are. A section leaves it off.
- Click a highlight to select it, drag it to move it, drag its handles to reshape it, and use the arrow keys to nudge it, + and − to grow or shrink it, Return to label it and Delete to remove it. Every change undoes. This works in plan, axonometric and explode views; in an explode view you edit it on the top plate. Its middle has to lie inside the sheet's area, and it has to cover at least 4 m².
- Open as sheet, in a highlight's bar, adds a tab whose area is exactly that highlight, so its ratios and figures describe it, and it keeps drawing the highlight. This is now the one way to a sheet of part of the site; the Whole site button and its map are gone from the tab bar. Projects that already have a sheet of part of the site open unchanged. The eye beside the shape buttons hides the highlights on one sheet.
- A sheet fits to round scales down to 1:200, 1:100 and 1:50, so a sheet opened from a single plot fills the paper. The Scale field in the inspector goes down to 1:50 too, and the scale bar takes 1, 2 and 2.5 m steps.
- Pins come in four sizes, S, M, L and XL, from the pin's bar, or − and + with a pin selected. The label grows more slowly than the marker. Older pins are medium.
- Assistants can list, add, edit and remove highlights with their colour and line, open one as a sheet, give pins a size, and show or hide pins and highlights per sheet with `show_on_sheet`. `add_sheet` no longer takes a `selection`; an assistant gets a sheet of part of the site from `open_highlight_as_sheet`, as the app does.

### Diagram column

- The diagrams moved from the filmstrip along the bottom to a column on the left of the sheet, grouped under Context and built form, Movement and places, Environment, People, and Sun and shade. Each one shows a small picture in the current palette and its name, and the one on the sheet is marked.
- Air quality is one diagram with a switch between NO₂, PM10 and PM2.5, and Noise is one diagram with a switch between road traffic and all sources. The switch sits under the selected diagram, and ← and → flip between the measures. That brings the column from 24 entries to 21.
- ↑ and ↓ step through every diagram. With a text box selected the arrows still nudge the text. The 1 to 9 keys and the numbers beside the diagrams are gone, since nine keys never reached all 21 diagrams.
- D, the button next to the back arrow and View > Toggle Diagrams show or hide the column, and the app remembers the choice. In a window narrower than 1000 px the column lies over the sheet instead of squeezing it.
- Exporting the whole series writes one file per measure, such as `westermarkt-air-quality-pm10.pdf`, so the series still holds every sheet it held before.
- Projects saved on PM10, PM2.5, traffic noise or noise from all sources open on the same measure of the folded diagram, and so do explode plates. A plate can now show any one measure, like Air quality, PM2.5.
- For assistants, `list_diagrams` gives each diagram its `group` and `variants`, `set_look` takes a `variant`, and plates take `"air-quality:pm25"`. The old ids still work in `set_look`.

### Places, transit and walking

- Four new diagrams from OpenStreetMap. Business shows shops, food and drink, offices, hotels and services on the site. Facilities shows education, culture, civic buildings and places of worship, health care, sport and play, car parking and bicycle parking. Transit shows the bus, tram, metro and train stops within a 10 minute walk, each named with the bus, tram, metro and ferry lines that call there. Train stations show their name only, without service numbers. Walkability shows how far you get on foot in 5, 10 and 15 minutes, with the facilities inside each band.
- Each group has its own shape as well as its colour, so the diagrams still read in Mono. Places of one group that would touch on paper merge into one larger symbol, so a busy shopping street does not turn into a smear. Bicycle parking is drawn as a light ring, and a bicycle park with its capacity mapped grows with it, so a station's bicycle park stands out from a single rack.
- Transit and Walkability zoom out until the walk fits the sheet, for example to 1:10 000 for the 15 minute walk around the Westerkerk, with the site outlined in the middle.
- Walk times are worked out on your own computer over the OpenStreetMap footpath network at 1.4 m/s, from the site centre. Benches, bins and vending machines are left out.
- The Ratios view counts businesses and facilities on the site by group, car and bicycle parking on their own, and transit stops within a 10 minute walk by mode.
- Sheets that show OpenStreetMap data credit "© OpenStreetMap contributors (ODbL 1.0)". Other sheets leave the credit off.
- OpenStreetMap is asked once per site, and the answer is kept with the other downloaded data, so a nudged site does not ask again. When it is busy or down, the site still arrives without these layers and says why.
- Assistants can read the walk counts with the new `get_neighbourhood` tool. Car and bicycle parking are counted apart from the facilities, so the site summary never lists parking as a missing facility.

### Analysis

- The inspector has an Analysis view next to Style and Ratios: a scrolling report on where the site is, what is on it, and its climate.
- Location gives the nearest address, the municipality and province, the coordinates in degrees and in RD, and a small map of the country with a dot and of the municipality with the site drawn in it. The country outline ships with the app.
- Site sums up the area, buildings, coverage, floor area ratio, mean and tallest height, median year built, most common use and the shares of green, water and paved ground, with the same numbers as the Ratios view. It adds road noise, nitrogen dioxide, the worst-case flood, the steepest slope, residents and dwellings when the site has them, and counts the facilities and shops within a 15 minute walk and the transit stops within 5 minutes when the walk was fetched.
- Climate comes from a PVGIS typical year for the site: monthly temperature with the mean daily low and high, outdoor comfort as hours per month from cold to hot, wind roses for each season, monthly sun on a flat roof and towards the sun, where the direct sun comes from in winter and summer, and humidity.
- Rain is thirty years of monthly totals and rain days from the nearest of thirty KNMI stations, named on the chart with its distance.
- Each section ends with a few design notes written from fixed rules, such as which side the winter wind comes from and what that means for entrances, when outdoor space is comfortable, and how long a 10 m building's shadow is at midwinter noon.
- Charts are drawn in the sheet's palette and change with it. Export one section or the whole analysis as an SVG or PDF page.
- Climate and rain are fetched once per site and kept with the other downloaded data. Location and site appear at once; climate and rain show their own loading and error state, and the rest of the analysis works without them.
- Assistants can read the numbers and the notes with the new `get_analysis` tool.
- On a sheet that shows part of the site, the Site section sums up that area, like the Ratios view does: its area, buildings, ground cover, noise, air and flood. The line under the Site heading says so, names any figure that still describes the whole site, such as the steepest slope, and notes that places and stops within walking distance are counted from the site's centre. Location, climate and rain stay on the whole site. Exporting the analysis from that sheet, and `get_analysis` with a `tab`, follow the same area.

### Outdoor comfort and the A4 report

- Outdoor comfort is now the real UTCI (Universal Thermal Climate Index), the felt temperature that counts sun, wind and humidity as well as the air. It uses the operational polynomial of Bröde et al. (2012), the one the official UTCI software, Ladybug Tools and pythermalcomfort use, and is checked against pythermalcomfort's published validation table.
- The sun's share comes from the SolarCal model of ASHRAE Standard 55, Appendix C: a standing person on open ground, sun over the shoulder, with the long-wave radiation of the sky from the PVGIS year making clear nights and winter days feel colder.
- The comfort section shows two charts, in the sun and in the shade, with daytime hours in the ten UTCI stress categories from extreme cold to extreme heat. The caption names the method and every assumption.
- For the Westermarkt, summer days are mostly free of thermal stress in sun or shade, and nine in ten winter daytime hours in the shade bring moderate or strong cold stress.
- The comfort notes read from the new numbers: the months that are comfortable in the sun, how many hours a year sunshine takes out of cold stress, how cold winter stays even in the sun, and how many hours bring heat stress in the sun against the shade.
- The whole analysis exports as a PDF of A4 pages: a title on the first page, a running header with the project and the address, page numbers, sections kept whole on one page, figures beside their notes, and the data credits with licences and links on the last page. One section, or the whole analysis as SVG, still exports as one page.
- `get_analysis` gives the comfort hours in the sun and in the shade per month, the names of the ten categories, and the method in one sentence.

### Noise, air, flood and people

- Nine new diagrams from Dutch government data. Traffic noise and Noise, all sources show the RIVM noise map in 5 dB bands from under 45 to 70 dB and louder. Air quality shows the yearly mean of nitrogen dioxide, and two more show PM10 and PM2.5, each in bands around the WHO guideline. Flood depth shows how deep water could stand in LIWO's worst case, a flood with a chance under 1 in 30,000 a year. Slope shows how steep the ground is in percent, from the same AHN terrain the contours use. Population shows residents per CBS 100 m square, and Dwellings colours each building by how many homes BAG counts in it.
- The bands are drawn as smooth areas, not as the grid cells they come from, so they export to Illustrator and DXF as shapes you can edit. In DXF each band sits at the height of its lower edge, so 55 dB or 0.5 m can be read off the outline.
- Each legend says the unit, the source and its year under its title, for example "Lden in dB, the day, evening and night average. RIVM, 2022."
- Sheets credit RIVM, Rijkswaterstaat (LIWO) and CBS only when they show their data.
- The Ratios view adds how much of the site lies in each road noise band, dwellings per building, floor area per building from BAG, and next to the estimated floor area ratio a second one from BAG floor areas, with the site's dwellings and residents. Both show up in the CSV and SVG export too.
- Assistants can read noise, air quality, flood depth, slope, residents and dwellings for a site with the new `get_environment` tool.
- A source that does not answer is tried twice more, the second time after a longer pause, before the site arrives without it.

### Trees, green and heat

- Trees and green is a new diagram with three measures. Trees shows every tree of 2.5 m and taller by its height, from the RIVM tree map. Green shows where grass, shrubs and trees are, split by whether the land is farmland, Natura 2000, the Nature Network or none of these. NDVI shows how green the ground reads in the newest summer infrared aerial photo from PDOK, as an approximate vegetation index; the legend says it is approximate because the photo is colour-corrected, not calibrated.
- Tree cells that a building footprint reaches into are left out of the canopy, its bands and the canopy share. The RIVM tree map read some roofs and façades as crowns, such as a 60 m "tree" beside a TU/e tower. A fetch now also keeps the tree heights 150 m past the site, for the woods that block sight, sky and sun (see Sky view and solar radiation). Sites fetched before this keep the old canopy until they are fetched again.
- The Green legend now says plainly that its classes describe how the land is protected (farmland, Natura 2000, the Nature Network or none), not what kind of green it is.
- Heat island is a new diagram: how much warmer the air over the site is than in the countryside around, on average over June to August, in bands of 0.5 °C, from RIVM's heat island map.
- The Analysis site summary adds the share of the site under tree canopy and the mean heat island.
- Assistants get the canopy share, the mean vegetation index and the mean heat island from `get_environment`, each with its year.
- Sites fetched before this say so on these diagrams and ask to be fetched again. Sheets credit RIVM and Beeldmateriaal Nederland only when they show their data.

### Newest editions

- Noise, air quality and population come from the newest year RIVM and CBS publish, found when the site is fetched instead of fixed in openarchi. When RIVM publishes its 2023 noise map, the next fetch uses it and the legend says "RIVM, 2023."
- A saved project keeps the years it was fetched with, so its legends, ratios, Analysis tab and `get_environment` go on naming the same years until you fetch the site again. Projects from before this change show the years they were fetched with too.
- When RIVM or CBS cannot be asked which year is newest, the site arrives with the last year they named and a warning that it may not be the newest.
- Asking takes about a second, once a day; every other fetch that day reuses the answer.

### Laser cutting

- The export sheet has a Laser choice: files a laser cutter takes directly, for a physical site model of the sheet. A sheet that frames part of the site makes a model of that part only.
- Pick the model's scale (1:500, 1:1000, 1:2500 or any other; it starts from the sheet's scale when that is a usual model scale, else 1:1000), the board (2 mm board, 3 mm MDF or any thickness), the laser bed (600 × 400, 900 × 600, 1000 × 600 mm or your own) and which parts to cut: the base plate, the buildings, and contour lines on the plate when the site has them.
- The base plate is the site boundary, with roads, water, green and every building footprint scored on it, so you know where to glue. A plate larger than the bed is cut into equal tiles along straight lines, engraved T1, T2 and on.
- Each building is a stack of identical footprint pieces, one per board thickness of its height: 12 m at 1:1000 in 2 mm board is six pieces. Every piece is engraved with an id like B12·3/5, and the plate engraves B12 at its footprint. Ids run north to south in rows and west to east within a row.
- Outlines are simplified to 0.2 mm on the model, finer than a laser cuts cleanly. Buildings narrower than 2 mm on the model are left out of the pieces but still scored on the plate. Pieces are turned to lie square on the bed, packed with a 2 mm gap and a 5 mm margin, and the same site always packs the same way.
- One SVG per bed, sized in real millimetres with no transforms: cuts are pure red (#FF0000) hairlines, scores pure blue (#0000FF) hairlines, holes cut before outlines. DXF writes the same with CUT and ENGRAVE layers.
- Before you choose a folder the dialog says what the files will hold: beds, pieces, square metres of board, the plate's size and tiles, and how many buildings were left out or had no measured height.
- Assistants can do the same with `export_laser`, which takes the same settings and can report the summary without writing.

### Plates and layout

- Plates in the explode view are reordered by dragging: by the grip in the inspector's plate list, where the other rows slide aside to show where it lands. Dragging the plates on the sheet itself was slow and is gone. Alt+↑ and Alt+↓ move a focused row in the list. Each move is one undo step.
- The explode view has a scale bar again. The stack is drawn at the nearest round scale that fits, such as 1:5000, and the note reads "Scale 1:5000 left to right, on A3", because a tilted plate is only at scale across the paper; lengths running into it are shortened by the tilt. The axonometric note says "left to right" for the same reason.
- The map in the site step shows a metric scale bar in the bottom left corner.
- The diagram column and the inspector can be dragged wider or narrower by their inner edge, or moved with the arrow keys once the edge is focused. Double-click the edge to reset it. The app remembers both widths. In a wider inspector the ratios and analysis charts, numbers and notes grow with it, and the summary figures spread over three columns; a wider diagram column shows bigger pictures.

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

### Your own palettes

- New under the palette menu makes a palette of your own from a few colours. Pick each swatch on a hue wheel with a saturation square inside it, or paste what a palette generator exports: hex codes, a link with the colours in it, or its SVG, JSON or XML file, dropped or chosen. Two to ten swatches; drag one along the row to reorder it, and Delete removes it.
- openarchi gives every swatch a job by one plain rule: the lightest colour becomes the paper (softened to a pale tint when it is colourful), the darkest the ink, the most colourful the accent, and the colours nearest to blue and green the water and green. Each role says which swatch it took and why, and a click pins it to another swatch instead. Everything else a sheet needs, the ramps, categories and every layer, is mixed from those roles.
- The editor checks contrast as you go and names the colours that read badly, with their ratio against the WCAG 2.1 thresholds: 4.5:1 for text, 3:1 for buildings and the site outline.
- The open sheet is drawn live in the palette while you edit it, and ⌘Z inside the editor undoes your last change to the palette.
- Duplicate starts a palette from the current one. A copy of a built-in palette keeps its hand-made look until you change a colour.
- Your palettes sit under the built-in eight in the menu and work on every diagram, view and the neighbourhood. Edit it again later to rename it or change its swatches and pins, or delete it: it moves to Recently Deleted, and Undo in the toast brings it back.
- Copy a palette's swatches as JSON, CSV or a hex list, to use them elsewhere.
- Assistants can import a palette with `import_palette` and list yours with `list_user_palettes`.
- Every ratio group says under its title where its numbers come from, with the edition year where it is known, and the ratios CSV has a source column.

### Neighbourhood

- Every sheet can show the neighbourhood around the site: buildings, roads, water and green outside the boundary, drawn in the sheet's palette but greyed towards the paper with thinner lines, so the site stays the thing you look at. It fills the drawing area and never reaches the title, legend, scale bar or credits column.
- In the axonometric the neighbours stand up at their measured height among the site's own buildings, in a pale roof colour and with softer light. In an explode view the bottom plate widens into a disc of neighbourhood under the stack.
- The neighbourhood never shows up in the legend, the ratios or the analysis. Those still describe only the drawn site.
- Neighbourhood in the View section of the inspector, under Plan, Axo and Explode, turns it on and off, and ⌘Z undoes it. On a sheet that shows part of the site it also hides the rest of the site drawn muted around that part. New sheets start with it on. Projects from before have no neighbourhood until the site is fetched again, and open with it off.
- A fetch also downloads the neighbourhood: 250 to 500 m around the site, depending on its size. It reuses the tiles along the site's edge and runs alongside the OpenStreetMap request, so a fetch takes about as long as before.
- DXF exports put the neighbourhood on layers of its own, `OA-CONTEXT-BUILDING`, `OA-CONTEXT-ROAD` and so on, in grey.
- `set_look` takes `sheet.show_context`.
- Neighbourhood opacity, a slider under the Neighbourhood toggle, lets the ground show through the neighbourhood from 100 % down to 10 %, so it can sit further back behind the site. In the axonometric each neighbour fades as one block, so its walls never show through its own roof. It is one undo step per drag, and `set_look` takes it as `sheet.context_opacity`, from 0 to 1.

### Reports

- Reports in the sheet's top bar opens a report editor over the sheet. A report is pages you lay out yourself from the project's ratios, analysis charts, facts and design notes, whole sheets in any diagram, and your own headings and text.
- The library on the left lists every block the project can draw. Drag one onto a page, or click it to add it to the middle of the current page. On the page, drag a block to move it, with snap lines to the page and the other blocks, and drag its handles to resize it. The inspector takes exact positions and sizes in millimetres.
- Every block is drawn live from the project's data in the report's palette, which follows the sheet's unless you pick another. Changing the palette or fetching the site again redraws the whole report.
- Ratios blocks switch between bars, a pie and a table. Sheet blocks switch diagram and measure. Double-click a text block to edit its words, and the bar above it sets the font, size, weight, alignment and colour, as on the sheet.
- Pages are A4 to A1, portrait or landscape, each on its own. Drag a page by its number to move it, or use the arrows in its header. The blocks of a page are listed top first in the inspector, and dragging a row restacks it.
- The arrow keys nudge the selected block by 1 mm (with ⇧ by 10 mm), ⌘D duplicates it, ⌘] and ⌘[ bring it forward or send it back, and Delete removes it. ⌘Z undoes every change to a report, including adding and deleting whole reports.
- Export writes one PDF of every page, or an SVG or a PNG per page. The SVG keeps each block as a named layer. A report can show page numbers and lie on the palette's paper, its ground colour or plain white.
- Reports are saved in the project as you go. Projects from before open with none.
- For assistants, `list_reports` lists a project's reports with each page's paper and blocks, and `export_report` writes one as a PDF, or as an SVG or PNG per page.

### Report editor

- Millimetre rulers run along the top and the left of the pages, as in a word processor, and follow zoom and scroll. They measure from the corner of the current page and mark the selected block's extent on both.
- While a block is moved, resized or dragged in from the library, its extent lights up on both rulers and a label under it gives its position and size in millimetres. A block from the library shows a dashed outline of where it will land before you let go.
- Headings, text and captions are no longer in the library. Add heading, Add text and Add caption in the page section of the inspector, or **T**, put one across the current page under the last block, selected and ready to type. Reports with text blocks from before open as they were.
- The library shows a real picture of every block: ratios charts, facts and design notes as well as the analysis charts and sheets. The pictures sit on the palette's paper, so charts of every proportion read as small pages instead of slivers. They are drawn in the background while you work on the sheet, one at a time and after the preview, so the library is full when you open it.
- The library and the inspector resize by dragging their inner edge, as on the sheet, and keep their widths. Double-click an edge to put it back.
- The way back to the sheet is a labelled Sheet button at the top left. **Esc** only lets go of the selected block, so the reports are never left by accident.

### Speed

- A sheet that shows part of a large site no longer carries the parts its frame hides, and path points too fine to see on paper are left out. The context sheet of a 3.8 km² site went from 6.4 MB to 0.9 MB and draws the same, so every change reaches the screen sooner.
- Dragging the scale slider redraws the sheet while it moves, instead of waiting for the pointer to rest. Stepping back to a diagram or undoing shows the earlier sheet at once.
- Panning moves the drawn sheet instead of painting it again, and zooming scales it smoothly, sharpening it once the zoom rests.
- Saving no longer packs the whole site again each time; it packs it once after opening or fetching. A sheet area is cut from the site once instead of on every redraw.
- The diagram column's pictures always show the plan, leave out the sheet's text and ignore turning or tilting, so typing a title or turning the view no longer redraws all 21 of them.
- `pnpm tauri dev` builds its dependencies optimised, so a development build renders and saves close to release speed.

### Fixes

- A polygon drawn clockwise showed the area of the rest of the earth (510 million km²); it now shows its own.
- A walk diagram (walkability, transit, facilities) on a site larger than its walk drew its frame past the drawing zone and over the legend; the frame now stops at the zone and cuts the site off there.
- Pin labels are set larger than the map's own labels, so a landmark's name reads at a glance.
- Assistants follow the app's rules: they can no longer add the numbered pin the app dropped, and `import_palette` no longer saves a palette whose text cannot be read.

### A calmer palette editor

- The palette editor fills the window instead of a fixed 1180 px box, so the live sheet is as large as the screen allows, at 1280 px and at 2560 px alike.
- The right side follows the short path most palettes take: paste hex codes or drop a file, look at the swatches, name the palette, save. Swatches are larger, and a click on one opens its colour picker; another click closes it.
- Which swatch paints which role, and copying the palette as JSON, CSV or a hex list, now wait behind one "Fine-tune which colour paints what" disclosure. The hue wheel is gone; the picker's saturation square and hue slider do the same job.
- A palette whose text misses its WCAG ratio on the paper can no longer be saved: Save stays grey and says to fix the text contrast. Faint buildings or a faint accent only warn, since a pale figure can be a deliberate look.
- The Ratios view now shows the source line under every group's title, and the Analysis tab moved its source line up under each section title to match.

### Old sites

- A diagram whose data the site was fetched without, such as transit on a site fetched before openarchi read OpenStreetMap, says so in a note over the sheet with a Fetch again button, instead of drawing only the base map. So does the Neighbourhood toggle on a site fetched before the neighbourhood was kept. A site that truly has no flood risk or no shops is not mistaken for one fetched without the source.
- `render_preview` returns the same notes as text next to the picture.
- An assistant fetching the open site again no longer loses its new data when the app saves next: the app takes the newer fetch and keeps your own changes to the sheets.

### Floors, blocks and sunlight hours

- Three new diagrams worked out from data every site already has, so they need no new download and work on projects made before them.
- Floors colours each building by its number of floors, from its roof height at 3 m per floor: 1, 2, 3–4, 5–8 and 9 or more. Its Floor area variant colours each building by its gross floor area, its footprint times its floors. The Floors chart in the ratios now uses the same classes as the diagram, and the ratios panel and CSV give the site's total gross floor area.
- City blocks cuts the site into the blocks its streets, paths, rail and water leave, so a road that stops at the site edge still cuts and a circle closes its blocks at its edge. Its GSI variant colours each block by how much of it the buildings cover, and its FSI variant by its floor area over its area, both as Spacematrix defines them.
- Sunlight hours shows how many hours of direct sun the ground gets over the sheet's sun date, past the shadows of the buildings, trees and woods in the site and around it, on a 2 m grid. Woods block sun hours too: the edge of a wood casts the shadow of its canopy height, and the ground under it is in shade all day. The legend names the date and the method, a sun-on-ground study after BRE BR 209, which asks for two hours on 21 March over half of a garden. More sun reads brighter and shade darker in every palette, and buildings are drawn blank and finely hatched, so no roof reads as a class. On this diagram the time field shows only in the axonometric view, which is lit at one moment.
- All three draw in plan, axonometric and explode views and on a sheet that shows part of the site, and assistants find them through `list_diagrams`.
- Your own palettes saved before a layer existed now draw it too: a palette made from swatches gets the style its own colours give that layer, any other the Paper style in its own colours. The file on disk is not changed.

### Sections

- Cut a section through the site. The section tool in the tab bar, or S, draws a line on the sheet with a drag or two clicks. It shows on the plan, the axonometric and the explode view as a cut marker in the palette's accent: a long-dash line, a heavy tick and an arrow at each end, and the labels A and A'. Drag either end to move it, click it and press Delete to remove it; every change undoes like any other edit.
- The fourth view, Section, draws the elevation along the line, seen from A towards A' and looking to the left of the line. The ground is the measured profile, hatched below; buildings the line passes through are cut in solid poché in the palette's ink, with their measured height over the roof; buildings and trees beyond the line stand in elevation, fading with distance up to a depth you set (60 m to start). A ruler beside it marks the height in metres above NAP and a dashed line marks NAP itself. The section picks the largest round scale that fits the sheet, from 1:100 up, or the scale you pick in the inspector; what does not fit is cut off at the edge of the drawing. Heights can be drawn two or five times taller than lengths for a flat site, and the legend then says so.
- A diagram that colours buildings colours them in the section too, with its legend, so a building age section shows the ages of the street wall behind the cut.
- Flip in the inspector turns the section round, and Remove takes the line away. The Section view is greyed out until a sheet has a line.
- A section works on a sheet that shows part of the site: what lies outside the area is drawn as neighbourhood.
- A fetch now keeps the bare ground of AHN over the site and its neighbourhood in 4 m cells, to the decimetre, for the profile. That makes a project file about 3 % bigger, some 40 kB for a 200 m site. Sites fetched before this draw a flat ground and say so in the legend until they are fetched again.
- Water the section crosses stands at its target level where Rijkswaterstaat publishes one (−0.4 m NAP for the Amsterdam canals), and otherwise at its lower bank. It reaches down to the bed Rijkswaterstaat surveyed, as on the Waal, or fades out downwards where nobody did, as in the Amsterdam canals. The legend says which. A fetch now asks Rijkswaterstaat for both, only for tiles with water.
- SVG, PDF and PNG export a section like any sheet, and so does DXF (see DXF below).
- Assistants draw, change, flip and clear a sheet's line, and set its scale and exaggeration, with `set_section`, and `set_look` shows the section view once a line is there.

### Sky view and solar radiation

- Sky view factor shows how much of the sky each point of the ground sees past the buildings and terrain around it, from 0 at the foot of a shaft to 1 on open land. It is the cosine-weighted factor of Johnson & Watson, found from the horizon in 32 directions within 150 m as Zakšek, Oštir and Kokalj describe, and the legend says so. Buildings in the site and its neighbourhood count as blocks of their roof height, and the terrain counts where the site kept it. It needs no new data, so it works on projects made before it.
- Trees hide the sky only where they are dense, like a wood. "Dense" is the forest of the FAO Forest Resources Assessment: more than 0.5 ha of trees taller than 5 m, and a strip only when it is more than 20 m wide. On the RIVM tree map that means patches of tree cells over 5 m, at least 20 m across and 0.5 ha in all, each standing solid up to its canopy. A street tree or a row of canal elms is see-through. The sky view, the solar radiation, the View diagram and the sunlight hours share this rule, and the legends name it. A site without the tree map has no blocking trees. The Shadow and Sunlight hours diagrams still cast every single tree's shadow as well.
- Solar radiation shows how much sunlight the ground and the roofs receive in a typical year, in kWh/m²: the direct sun for every hour the line from the cell to the sun clears the buildings, woods and terrain, plus the light of the sky it sees, weighted by its sky view factor. A roof is counted from its own height, so a taller building next door shades a lower roof; on a plan the buildings are drawn as outlines over it, and in a tilted view they stand up blank as before. The light comes from the PVGIS typical year the climate analysis already uses, hour by hour on one average day per month after Klein. Open ground in Amsterdam comes out at about 1,040 kWh/m².
- The radiation's classes are set per site, as shares of what open ground receives there in its own typical year (under 30 %, 30 to 50 %, up to 95 % or more), so a sunnier country still spreads over every band. The legend gives each class in kWh/m² with its share, and names the open-ground total. It takes about 40 to 70 ms on the test sites.
- A legend note can now run to three lines instead of two, so the longer method notes are no longer cut off.
- A fetch now keeps that typical year with the site, about 600 numbers, from the same cache cell as the analysis. A site fetched before this says in a note over the sheet that the solar radiation needs a fresh fetch, and the PVGIS credit is printed only on sheets that draw it.
- Both draw in bands under a legend like the sunlight hours, in every palette, in plan, axonometric and explode views and on a sheet that shows part of the site, and assistants find them through `list_diagrams`.

### Wind

- A Wind diagram shows how the site's prevailing wind moves through it at street level: the ground in bands of how fast the air moves as a share of the incoming wind, from "under 20%, sheltered" to "100% or more, windier than open ground", with flow lines and arrowheads that point downwind.
- The flow is worked out in two dimensions round the building footprints of the site and 200 m of neighbourhood, with the lattice Boltzmann method (D2Q9 with the BGK collision of Qian, d'Humières & Lallemand, 1992), on a grid of about 4 m. The flow lines are spaced evenly after Jobard & Lefer. The legend names the method and says plainly what it leaves out: no air over the roofs and no turbulence model, so it is for comparing sheltered and windy places on one site, not for comfort criteria.
- Because a plan cannot let air over the roofs, the gaps between buildings come out windier and dense old centres calmer than they are. At the Domplein in Utrecht almost the whole site reads as sheltered.
- The wind comes from the most common direction in the site's typical year, the longest wedge of the wind rose in the analysis. The View section picks the whole year or one season, and the legend names the direction and its share of hours, like "from the SW, most common all year (12% of hours)". The choice undoes like any other edit.
- The Wind diagram is drawn in plan only. Its Axo and Explode buttons are greyed out, it is not offered as an explode plate, and choosing it on a tilted sheet turns the sheet to its plan.
- A fetch now keeps the typical year's wind with the site too: the hours from each of 16 directions per month, 204 numbers, from the same PVGIS answer as the solar radiation. A site fetched before this says in a note over the sheet that the wind needs a fresh fetch.
- Solving takes about half a second on a large site, so it is kept for the last few sites and seasons: changing the palette, the title or a thumbnail draws it again in a few milliseconds. Large sites draw on a coarser grid instead of slowing down.
- A legend note may now run to three lines instead of two.
- Assistants choose the season with `set_wind_season`, and `set_look` refuses the axonometric and explode views and explode plates for the Wind diagram, as the app does.

### DXF

- A DXF now holds what the sheet shows, not only the fetched data. A diagram that colours a layer by class writes one CAD layer per legend class, so a building height sheet arrives as `OA-BUILDING-UNDER-6-M`, `OA-BUILDING-6-10-M` and so on, with `OA-BUILDING-UNKNOWN` for buildings without a height. Computed layers the diagram draws, city blocks and hours of sun, come along on `OA-BLOCK` and `OA-SUN-HOURS-...`.
- The Export dialog asks which layers a plan DXF holds. As on the sheet, the default, writes the layers the sheet shows, without the ones you hid, and the neighbourhood only when the sheet shows it, so the file matches what you are looking at. Every layer writes every fetched layer and the whole neighbourhood, whatever the sheet shows. The dialog remembers the choice. A section sheet's DXF is its section either way.
- Each CAD layer takes the colour it has on the sheet: the palette's fill for the layer, or the class colour for a legend class, including colours you gave a layer yourself, so the drawing reads like the sheet. The site outline, the section line and pins are in the palette's accent, highlight captions in its ink, and the neighbourhood stays grey. R12 only knows the 255 AutoCAD index colours, so each colour becomes the nearest of those by the CIEDE2000 colour difference, and each legend class keeps a colour of its own even when a pale ramp puts two classes near one index colour.
- Highlights are in the DXF on a layer per line and colour, such as `OA-HIGHLIGHT-ACCENT`, `OA-HIGHLIGHT-DASHED-ACCENT` or `OA-HIGHLIGHT-DOTTED-2E7D32`, each with a matching line type and the highlight's own colour, and their captions on `OA-HIGHLIGHT-LABEL`. The section line is on `OA-SECTION-LINE` as a dash-dot line, with its ticks, arrows and the A and A' labels on `OA-SECTION-MARK`. Dashes, labels and pin labels are sized for the sheet's scale, so the DXF plotted at that scale looks like the sheet.
- A section sheet exports its section as DXF, in metres along the line from A and metres NAP: the ground, the buildings the line cuts, what stands beyond it, trees, water from its level to its bed, and a dashed NAP line with the target level of each water written beside it. Heights are true, whatever the sheet exaggerates them by.
- Text outside plain ASCII, such as m² or an accented street name, is written as the `\U+00B2` escapes R12 uses for it instead of raw UTF-8, which older readers garbled.
- Assistants export the whole series with `export` and `series: true` into a `folder`, as the dialog does, and `export_laser` with `check_only` no longer needs a folder.
- Assistants choose the same layers with `export` and `dxf_layers`, `shown` or `all`.

### View

- A new View diagram tints the ground a person standing at a viewpoint can see, over the site and the whole drawing, with the eye drawn where it stands. The legend names the method, the eye height and how much of the site's open ground is in view.
- It is worked out by line of sight (the R2 viewshed of Franklin and Ray, 1994) past every building of the site and its neighbourhood at its roof height, standing on the kept terrain where the site has it. The eye stands 1.7 m up, the eye level of an average Dutch adult. Woods block the view as solid stands (the FAO forest rule under Sky view and solar radiation); a single tree is left out, because a standing eye sees the ground under and past its crown. An eye placed in a wood looks from its floor.
- The eye starts on the street or square nearest the site centre. The Viewpoint section of the inspector stands it on one of the project's pins, where it follows the pin when the pin moves and goes back to the centre when the pin is deleted, or back at the centre, V or Place on the sheet puts it where you click, and the eye can be dragged on the sheet. Every move undoes. A viewpoint on a roof looks from the roof.
- The view draws in plan, axonometric and on an explode plate. It takes a few milliseconds, so it redraws with every change.
- Assistants set the viewpoint with `set_view_point`, tied to a pin the same way, and `list_sheets` shows it with the pin it stands on.

### Computed layers, tidied

- Sunlight hours, sky view, solar radiation and city blocks are worked out once per site and kept, as the wind already was, so typing a title or changing the palette on such a sheet no longer works them out again. Redrawing the sunlight hours of the Westermarkt went from about 130 ms to 17 ms, the solar radiation from 48 ms to 19 ms and the city blocks from 28 ms to 8 ms.
- The report editor draws the location, climate and rain of a site once per palette, not once per block, so a page of climate blocks opens and edits faster.
- The view, the sky view and the solar radiation look over one height surface, so they agree on what stands where. A roof stands at its own measured ground level plus its height, the ground under buildings and water is filled in from the streets around it, and where two footprints overlap the taller one counts. On the test sites this moves under 2 % of the ground from one sky view or radiation class to the next; the view did not change.
- An exported ratios page draws its secondary text in the same grey as the analysis charts and the report's ratio blocks, a shade darker than before.
- A highlight's caption writes its area the way the ratios and the site summary do: 5,000 m² rather than 5000 m².

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
