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
- Themes are now called palettes in the app.
- Palettes are picked from a compact menu, like choosing a font: the closed control shows the current palette and its colours, and the open menu lists every palette with its colours, your own palettes in their own group. [ and ] still step through them.
- Diagrams are picked from the filmstrip along the bottom only. The inspector no longer repeats them, which leaves it room for the palette, layers, view and sheet.

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
