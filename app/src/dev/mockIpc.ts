// A stand-in for the Rust side, so the front end runs in a plain browser for
// screenshots. main.tsx loads it only in dev and only outside Tauri, so it never
// reaches a release build. The pictures come from app/dev-fixtures, written by
// `cargo run --release -p openarchi-core --example app_fixtures`.

import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { chosenVariant } from "../lib/diagrams";
import type { Area, Catalog, Highlight, Credit, LaserSettings, Layer, Look, Pin, ProjectSummary, ProjectView, Report, ReportBlock, Sheet, SunMoment, View } from "../lib/types";
import { answerPalette, withPaletteEdits } from "./mockPalettes";
import { mockHighlightArea, withMockHighlights } from "./mockHighlights";
import { answerModel, mockModelView, withMockModel } from "./mockModel";
import { withMockPins } from "./mockPins";
import { withMockSection } from "./mockSection";
import { withMockViewPoint } from "./mockViewPoint";
import { mockBackdrop, mockBlockFixture, mockTextBlock } from "./mockReports";

const FIXTURES = "/dev-fixtures";
const MAX_AREA_M2 = 4_000_000;
// How long the mock pretends PVGIS and KNMI take, so the loading state shows.
const SLOW_PART_MS = 1200;

type Fixture = { catalog: Catalog; area: Area; credits: Credit[]; layers_present: Layer[] };

const SHEET: Sheet = {
  paper: "a3",
  landscape: true,
  scale: 2500,
  frame: "circle",
  title: "Westermarkt",
  subtitle: "Amsterdam, around the Westerkerk",
  show_legend: true,
  show_north_arrow: true,
  show_scale_bar: true,
  show_credits: true,
  show_context: true,
  context_opacity: 1,
  title_style: {},
  subtitle_style: {},
  title_position: null,
  subtitle_position: null,
  texts: [],
};

// The core's defaults from crates/core/src/view.rs.
const VIEW: View = {
  mode: "plan",
  rotation_deg: 0,
  tilt_deg: 35,
  plates: ["building-use", "building-age", "landscape", "movement", "context"],
  stacking: false,
};
const SUN: SunMoment = { month: 3, day: 21, hour: 13, minute: 0 };

function look(diagram_id: string, theme_id: string, patch: Partial<Sheet> = {}): Look {
  return { diagram_id, theme_id, sheet: { ...SHEET, ...patch }, overrides: {}, view: VIEW, sun: SUN };
}

const PROJECTS: ProjectSummary[] = [
  summary("westermarkt", "Westermarkt", "2026-10-24T10:12:00Z", look("building-age", "paper")),
  summary("domplein", "Domplein, Utrecht", "2026-10-23T16:40:00Z", look("context", "ink", { scale: 2000 })),
  summary("blauwe-loper", "Blauwe Loper", "2026-10-22T09:05:00Z", look("figure-ground", "mono", { paper: "a2" })),
  summary("polder", "Polder edge study", "2026-10-20T14:30:00Z", look("landscape", "polder", { scale: 5000 })),
  summary("night", "Jordaan by night", "2026-10-18T19:55:00Z", look("building-use", "midnight", { paper: "a1", scale: 1000 })),
  { id: "katendrecht", name: "Katendrecht, Rotterdam", modified: "2026-10-17T11:00:00Z", has_scene: false, look: null },
];

const TRASH: ProjectSummary[] = [
  { id: "old", name: "First try", modified: "2026-10-12T08:00:00Z", has_scene: true, look: look("parcels", "pastel") },
];

function summary(id: string, name: string, modified: string, projectLook: Look): ProjectSummary {
  return { id, name, modified, has_scene: true, look: projectLook };
}

const fixture: Fixture = await fetch(`${FIXTURES}/fixture.json`).then((response) => response.json());
// The open project's sheet tabs, pins and highlights, as save_sheets last left them.
let openSheets: Look[] = [PROJECTS[0].look!];
let openPins: Pin[] = [];
let openHighlights: Highlight[] = [];
// The fixtures are drawn at the default scale; a sheet framing part of the site pretends to zoom in.
const MOCK_SCALE = 2500;
const MOCK_SELECTION_SCALE = 1000;
let openReports: Report[] = [];

function view(project: ProjectSummary): ProjectView {
  return {
    id: project.id,
    name: project.name,
    modified: project.modified,
    area: fixture.area,
    look: project.look ?? look("context", "paper"),
    sheets: openSheets,
    pins: openPins,
    highlights: openHighlights,
    has_scene: project.has_scene,
    fetched_at: project.has_scene ? project.modified : null,
    warnings: [],
    credits: fixture.credits,
    layers_present: fixture.layers_present,
    reports: openReports,
    model: mockModelView(),
  };
}

/** The fixture a look shows: "air-quality-pm10--paper", a variant named as app_fixtures writes it. */
function fixtureName(shown: Look) {
  const diagram = fixture.catalog.diagrams.find((entry) => entry.id === shown.diagram_id);
  const variant = diagram ? chosenVariant(diagram, shown.variant) : null;
  const drawing = variant ? `${shown.diagram_id}-${variant.id}` : shown.diagram_id;
  return `${drawing}--${shown.theme_id}`;
}

const picture = (lookToShow: Look) => `${FIXTURES}/thumbs/${fixtureName(lookToShow)}.png`;

function circleArea(area: Area) {
  return area.kind === "circle" ? Math.PI * area.radius_m * area.radius_m : 0;
}

type Args = Record<string, unknown>;

async function answer(command: string, args: Args) {
  const project = PROJECTS.find((entry) => entry.id === args.id);
  const palette = await answerPalette(command, args, fixture.catalog, (shown) => answer("render_preview", { look: shown }));
  if (palette !== undefined) return palette;
  const model = answerModel(command, args);
  if (model !== undefined) return model;
  // The save and folder dialogs answer at once, as if the user chose the Desktop.
  if (command === "plugin:dialog|save") return `/Users/you/Desktop/${(args.options as { defaultPath?: string }).defaultPath ?? "openarchi"}`;
  if (command === "plugin:dialog|open") return "/Users/you/Desktop";
  if (command === "library_path") return "/Users/you/Documents/openarchi";
  if (command === "list_projects") return PROJECTS;
  if (command === "list_trash") return TRASH;
  if (command === "get_catalog") return withPaletteEdits(fixture.catalog);
  if (command === "project_thumbnail") return project?.has_scene && project.look ? picture(project.look) : null;
  if (command === "open_project" && project) {
    openSheets = [project.look ?? openSheets[0]];
    openPins = [];
    openHighlights = [];
    return view(project);
  }
  if (command === "current_project") return view({ ...PROJECTS[0], look: openSheets[0] });
  if (command === "save_sheets") {
    openSheets = args.sheets as Look[];
    openPins = args.pins as Pin[];
    openHighlights = (args.highlights as Highlight[] | undefined) ?? [];
    return new Date().toISOString();
  }
  if (command === "highlight_area") return mockHighlightArea(args.highlight as Highlight);
  if (command === "fitting_scale") return (args.look as Look).selection ? MOCK_SELECTION_SCALE : MOCK_SCALE;
  if (command === "render_preview") {
    const shown = args.look as Look;
    // The explode fixture has the default plates in their default order, whatever the look asks for,
    // and the section fixture is cut along one fixed line, whatever line is drawn.
    const isFixedView = shown.view.mode === "explode" || shown.view.mode === "section";
    const name = isFixedView ? `${shown.view.mode}--${shown.theme_id}` : fixtureName(shown);
    const svg = await fetch(`${FIXTURES}/previews/${name}.svg`).then((response) => response.text());
    if (shown.view.mode === "section") return svg;
    const pins = (args.pins as Pin[] | undefined) ?? [];
    const withLine = withMockSection(withMockViewPoint(svg, shown, pins), shown);
    // The explode fixture's ground points belong to its top plate, so the mock draws highlights there only;
    // the core draws them on every plate.
    const highlights = shown.hide_highlights ? [] : ((args.highlights as Highlight[] | undefined) ?? []);
    const withHighlights = withMockHighlights(withMockModel(withLine), highlights);
    return shown.hide_pins ? withHighlights : withMockPins(withHighlights, pins);
  }
  if (command === "render_thumbnail") return picture(args.look as Look);
  if (command === "search_places") {
    return [{ label: "Westermarkt 20, 1016DK Amsterdam", kind: "adres", location: fixture.area.kind === "circle" ? fixture.area.center : null }];
  }
  if (command === "measure_area") return { m2: circleArea(args.area as Area), max_m2: MAX_AREA_M2, in_netherlands: true };
  if (command === "create_project") return view({ id: "new", name: String(args.name), modified: new Date().toISOString(), has_scene: false, look: null });
  if (command === "export_file_name") return "westermarkt.svg";
  if (command === "get_ratios") return fetch(`${FIXTURES}/ratios/${args.themeId}.json`).then((response) => response.json());
  if (command === "get_analysis") {
    // Climate and rain wait on PVGIS and KNMI in the app, so the mock waits too.
    const isSlow = args.part === "climate" || args.part === "rain";
    if (isSlow) await new Promise((resolve) => setTimeout(resolve, SLOW_PART_MS));
    // One sheet area stands in for every other, as with the zoomed-in scale.
    const name = args.part === "site" && args.selection ? "site-area" : args.part;
    return fetch(`${FIXTURES}/analysis/${name}--${args.themeId}.json`).then((response) => response.json());
  }
  if (command === "save_reports") {
    openReports = args.reports as Report[];
    return new Date().toISOString();
  }
  if (command === "render_report_block") return reportBlock(args.themeId as string, args.block as ReportBlock);
  if (command === "render_report_backdrop") {
    const report = args.report as Report;
    const theme = fixture.catalog.themes.find((entry) => entry.id === (report.theme_id ?? openSheets[0].theme_id));
    return theme ? mockBackdrop(report, args.index as number, theme) : null;
  }
  if (command === "laser_summary") return mockLaserSummary(args.settings as LaserSettings);
  if (command === "export_laser") {
    const summary = await mockLaserSummary(args.settings as LaserSettings);
    const files = Array.from({ length: summary.sheet_count }, (_, index) => `${args.folder}/westermarkt-laser-${index + 1}.svg`);
    return { files, summary };
  }
  if (command === "report_file_name") return `westermarkt.${args.format}`;
  if (command === "analysis_file_name") return args.sectionId ? `westermarkt-${args.sectionId}.svg` : "westermarkt-analysis.pdf";
  // Saves, renames, window theme and the like: nothing to show for them here.
  return null;
}

/** The summary app_fixtures wrote for the nearest preset scale, whatever the other settings. */
async function mockLaserSummary(settings: LaserSettings) {
  const scale = [500, 1000, 2500].reduce((best, preset) => (Math.abs(preset - settings.scale) < Math.abs(best - settings.scale) ? preset : best));
  return fetch(`${FIXTURES}/laser/${scale}.json`).then((response) => response.json());
}

/** A block as the core would draw it, from the fixtures: sheets from the previews, the rest from report-blocks. */
async function reportBlock(themeId: string, block: ReportBlock): Promise<string> {
  const content = block.content;
  const theme = fixture.catalog.themes.find((entry) => entry.id === themeId);
  if (content.kind === "text") return theme ? mockTextBlock(block, theme) : "";
  if (content.kind === "sheet") {
    const shown = { ...openSheets[0], diagram_id: content.diagram_id, variant: content.variant ?? null, theme_id: themeId };
    return fetch(`${FIXTURES}/previews/${fixtureName(shown)}.svg`).then((response) => response.text());
  }
  return fetch(`${FIXTURES}/report-blocks/${mockBlockFixture(content)}--${themeId}.svg`).then((response) => response.text());
}

mockWindows("main");
mockIPC((command, args) => answer(command, (args ?? {}) as Args), { shouldMockEvents: true });

// The menu bar is native, so a screenshot script sends its commands through this instead.
Object.assign(window, { menu: (id: string) => emit("menu", id) });
