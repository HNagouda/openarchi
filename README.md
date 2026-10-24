<div align="center">

<img src="app/src-tauri/icons/128x128.png" width="96" alt="openarchi icon">

# openarchi

**Site analysis diagrams for architects, free and open source, on Mac and Windows.**

Draw a site, and get a whole series of clean, styled diagrams ready for a board or a portfolio.

<img src="docs/screenshots/workspace-light.png" alt="The openarchi workspace showing a building age diagram of Domplein, Utrecht, with the diagram picker, theme cards and a filmstrip of every diagram" width="900">

</div>

## Why this exists

Every architecture project starts with a site analysis, and most students and offices still make those diagrams by hand. You trace the map in Illustrator or wrestle QGIS, then spend hours cleaning up shapes that don't close and roads that are only lines. It should take a minute: draw an area, get a series of styled diagrams.

openarchi does exactly that, and it runs on your own computer, stores your projects as files you own, and has no limits beyond what the data services allow. It starts with the Netherlands, because Dutch open data is unusually good: every road surface, canal and garden is mapped as a real shape, so the drawings come out clean without any cleanup.

In full transparency, openarchi is vibe coded. I described what I wanted and [Claude Code](https://claude.com/claude-code) wrote most of the code, while I tested it and steered it. It's open source, so every line is there to read.

## What it does

- **Nine diagrams from one site.** Context, figure-ground, open space, building use, building age, building height, landscape, movement and parcels. Each one answers one question, and the filmstrip at the bottom shows the whole series at once.
- **Themes that keep a series together.** Eight built-in themes (Paper, Ink, Mono, Polder, Blueprint, Terracotta, Midnight, Pastel) restyle every diagram at once, so a set of sheets reads as one family. Tweak any layer's fill, line or opacity on top.
- **Real data, cleanly drawn.** Buildings with year built, use and height, road surfaces at their true width, water, green, trees, parcels and terrain contours, from the Dutch government's open registers.
- **Sheets ready to print.** A3 to A0 or a square for social posts, at a real scale like 1:2500, with a legend, north arrow, scale bar and the data credits.
- **Exports that open properly.** SVG and PDF with named layers for Illustrator, Affinity and InDesign, PNG at any resolution, and DXF in real Dutch coordinates (RD New) for AutoCAD and Rhino.
- **Hard to lose work.** Projects save as you go, every save keeps a backup, ⌘Z undoes every change, and deleted projects wait in Recently Deleted.
- **Works with your AI assistant.** Connect Claude or another assistant over MCP and ask it to make a site, try diagrams and themes, or design a new theme. Its changes show up live in the open app. See [docs/mcp.md](docs/mcp.md).

<table>
  <tr>
    <td><img src="docs/screenshots/context--paper.png" alt="Context diagram of Westermarkt, Amsterdam, in the Paper theme"></td>
    <td><img src="docs/screenshots/building-age--midnight.png" alt="Building age diagram of Westermarkt in the Midnight theme"></td>
  </tr>
  <tr>
    <td align="center">Context, in Paper</td>
    <td align="center">Building age, in Midnight</td>
  </tr>
</table>

## Using it

1. **Draw a site.** Press ⌘N, search for an address, and draw a circle, rectangle or polygon around it (C, R or P). Sites can be up to 4 km².
2. **Fetch it.** openarchi downloads the buildings, surfaces, parcels and terrain for that area. A 300 m circle takes a few seconds.
3. **Pick a diagram and a theme.** Keys 1 to 9 switch diagrams, and [ and ] step through themes. The sheet updates as you go.
4. **Export.** Press ⌘E for this diagram or the whole series, as SVG, PDF, PNG or DXF.

The first time you open openarchi, a short tour walks through this. Help > Show Tour opens it again.

## Shortcuts

On Windows, Ctrl takes the place of ⌘.

- **⌘N** starts a new project, **⌘O** or **Return** opens the selected one, and **Delete** moves it to Recently Deleted.
- **C**, **R** and **P** draw a circle, rectangle or polygon. **Return** finishes a polygon and **Esc** stops drawing.
- **1 to 9** switch diagrams, and **[** and **]** step through themes.
- **⌘Z** undoes the last change and **⇧⌘Z** redoes it.
- **⌘E** exports, **I** hides or shows the inspector, and **⌘=**, **⌘−** and **⌘0** zoom the sheet.
- **⌘,** opens settings and **⌘/** lists every shortcut.

## Where the data comes from

Everything comes from open Dutch government data through [PDOK](https://www.pdok.nl): the large-scale topography map (BGT) for road surfaces, water, green and trees, the building register (BAG) for footprints, year built and use, the cadastral map (BRK) for parcels, and the national elevation model (AHN) for building heights and contours. BGT, BAG and AHN are public domain. BRK is CC BY 4.0, which is why every sheet prints its credits by default.

## Questions

**Is it really free?** Yes. No subscription, no account, no cap on sites.

**Does it work outside the Netherlands?** Not yet. Other countries need their own data sources, and each one is a small, self-contained module, so this is a good place to contribute.

**Found a bug, or have an idea?** [Open an issue](https://github.com/HNagouda/openarchi/issues/new/choose).

## For developers

openarchi is a Rust core with a Tauri and React app on top. [DEVELOPMENT.md](DEVELOPMENT.md) explains how it fits together and how to build it.

## License

Free and open source under the [GNU General Public License v3.0](LICENSE).
