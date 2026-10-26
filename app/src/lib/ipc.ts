// One typed function per Rust command. The Rust names are in
// src-tauri/src/commands; errors arrive as plain sentences.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AerialPhoto,
  AnalysisPart,
  AnalysisSection,
  Area,
  AreaSize,
  Catalog,
  ChartStyle,
  Editable,
  DxfLayers,
  ExportFormat,
  LaserExport,
  LaserSettings,
  LaserSummary,
  Look,
  ModelChoice,
  ModelUnit,
  ModelView,
  Placement,
  UpAxis,
  Highlight,
  Pin,
  Place,
  Progress,
  ProjectSummary,
  ProjectView,
  Ratios,
  Report,
  ReportBlock,
  ReportFormat,
  SiteEdits,
  SunDay,
  UnderModel,
  View3d,
} from "./types";

export const libraryPath = () => invoke<string>("library_path");
export const listProjects = () => invoke<ProjectSummary[]>("list_projects");
export const listTrash = () => invoke<ProjectSummary[]>("list_trash");
export const createProject = (name: string, area: Area) =>
  invoke<ProjectView>("create_project", { name, area });
export const openProject = (id: string) => invoke<ProjectView>("open_project", { id });
export const reloadProject = () => invoke<ProjectView>("reload_project");
export const closeProject = () => invoke<void>("close_project");
/** Every sheet in tab order, and the project's pins, highlights and site edits. Returns the new modified time. */
export const saveSheets = (sheets: Look[], pins: Pin[], highlights: Highlight[], edits: SiteEdits) =>
  invoke<string>("save_sheets", { sheets, pins, highlights, edits });
/** The sheet area a highlight becomes when it opens as a sheet, in longitude and latitude. */
export const highlightArea = (highlight: Highlight) => invoke<Area>("highlight_area", { highlight });
/** The largest round scale at which the look's area, or the sheet's area when it has one, fills the sheet. */
export const fittingScale = (look: Look) => invoke<number>("fitting_scale", { look });
export const renameProject = (id: string, name: string) =>
  invoke<void>("rename_project", { id, name });
export const deleteProject = (id: string) => invoke<void>("delete_project", { id });
export const restoreProject = (id: string) => invoke<void>("restore_project", { id });

export const getCatalog = () => invoke<Catalog>("get_catalog");
export const renderPreview = (look: Look, pins: Pin[], highlights: Highlight[]) =>
  invoke<string>("render_preview", { look, pins, highlights });
export const renderThumbnail = (look: Look, width: number) =>
  invoke<string>("render_thumbnail", { look, width });
export const projectThumbnail = (id: string, modified: string, width: number) =>
  invoke<string | null>("project_thumbnail", { id, modified, width });

export const searchPlaces = (query: string) => invoke<Place[]>("search_places", { query });
export const measureArea = (area: Area) => invoke<AreaSize>("measure_area", { area });
export const fetchSite = (area: Area) => invoke<number>("fetch_site", { area });
export const cancelFetch = () => invoke<void>("cancel_fetch");
export const currentProject = () => invoke<ProjectView>("current_project");

export const exportSheet = (look: Look, pins: Pin[], highlights: Highlight[], format: ExportFormat, dpi: number, dxfLayers: DxfLayers, path: string) =>
  invoke<string>("export_sheet", { look, pins, highlights, format, dpi, dxfLayers, path });
export const exportSeries = (look: Look, pins: Pin[], highlights: Highlight[], format: ExportFormat, dpi: number, dxfLayers: DxfLayers, folder: string) =>
  invoke<string[]>("export_series", { look, pins, highlights, format, dpi, dxfLayers, folder });
export const laserSummary = (look: Look, settings: LaserSettings) => invoke<LaserSummary>("laser_summary", { look, settings });
export const exportLaser = (look: Look, settings: LaserSettings, folder: string) =>
  invoke<LaserExport>("export_laser", { look, settings, folder });
export const revealPath = (path: string) => invoke<void>("reveal_path", { path });
export const openLink = (url: string) => invoke<void>("open_link", { url });

export type FetchProgressEvent = { generation: number; progress: Progress };
export type FetchFinishedEvent = { generation: number; error: string | null };
// Sent before each file of a series export; `done` counts the files already written.
export type ExportProgressEvent = { done: number; total: number; file: string };

export const onFetchProgress = (handler: (event: FetchProgressEvent) => void): Promise<UnlistenFn> =>
  listen<FetchProgressEvent>("fetch-progress", (event) => handler(event.payload));
export const onFetchFinished = (handler: (event: FetchFinishedEvent) => void): Promise<UnlistenFn> =>
  listen<FetchFinishedEvent>("fetch-finished", (event) => handler(event.payload));
export const onExportProgress = (handler: (event: ExportProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ExportProgressEvent>("export-progress", (event) => handler(event.payload));
export const onLibraryChanged = (handler: (id: string) => void): Promise<UnlistenFn> =>
  listen<{ id: string }>("library-changed", (event) => handler(event.payload.id));
export const onMenu = (handler: (id: string) => void): Promise<UnlistenFn> =>
  listen<string>("menu", (event) => handler(event.payload));

/** Tauri rejects with the command's error string; anything else becomes one. */
export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong.";
}

export const exportFileName = (diagramId: string, variant: string | null, format: ExportFormat) =>
  invoke<string>("export_file_name", { diagramId, variant, format });

/** With a sheet area (a highlight opened as a sheet), the ratios of that smaller area. */
export const getRatios = (themeId: string, selection: Area | null) => invoke<Ratios>("get_ratios", { themeId, selection });
export const ratiosFileName = () => invoke<string>("ratios_file_name");
/** CSV or SVG, by the extension of `path`. */
export const exportRatios = (themeId: string, selection: Area | null, style: ChartStyle, path: string) =>
  invoke<string>("export_ratios", { themeId, selection, style, path });

/** With a sheet area, the site part sums up that smaller area; the other parts stay on the whole site. */
export const getAnalysis = (part: AnalysisPart, themeId: string, selection: Area | null) =>
  invoke<AnalysisSection[]>("get_analysis", { part, themeId, selection });
/** With a section id, that section's name; without, the whole analysis's. */
export const analysisFileName = (sectionId: string | null) => invoke<string>("analysis_file_name", { sectionId });
/** SVG or PDF, by the extension of `path`. Without a section id, the whole analysis. */
export const exportAnalysis = (themeId: string, selection: Area | null, sectionId: string | null, path: string) =>
  invoke<string>("export_analysis", { themeId, selection, sectionId, path });

/** Reads a model file and keeps it under a new key; it takes effect through `setModel`. */
export const importModel = (path: string, unit: ModelUnit | null, up: UpAxis | null, placement: Placement | null) =>
  invoke<ModelView>("import_model", { path, unit, up, placement });
/** The same model read in another unit or with another axis up, kept under a new key. */
export const rereadModel = (key: string, unit: ModelUnit, up: UpAxis) => invoke<ModelView>("reread_model", { key, unit, up });
/** Puts a model in effect where the choice places it, or takes it away with null. */
export const setModel = (choice: ModelChoice | null) => invoke<void>("set_model", { choice });
/** The buildings and trees under the model where `choice` places it, with the trees where `edits` have them. */
export const underModel = (choice: ModelChoice, edits: SiteEdits) => invoke<UnderModel>("under_model", { choice, edits });

/** Puts the site edits in effect for every sheet, the 3D view and the exports. */
export const setSiteEdits = (edits: SiteEdits) => invoke<void>("set_site_edits", { edits });
/** The tree within `reachM` of an RD New point, or the building under it, removed ones too. */
export const pickOnSite = (edits: SiteEdits, x: number, y: number, reachM: number) =>
  invoke<Editable | null>("pick_on_site", { edits, x, y, reachM });
/** Every tree and building the edits name, for the edit tool's list and marks. */
export const siteEditThings = (edits: SiteEdits) => invoke<Editable[]>("site_edit_things", { edits });

/** The open site, its neighbours and the model as triangles, in a palette's ground colours. */
export const view3d = (look: Look) => invoke<View3d>("view_3d", { look });
export const aerialPhoto = () => invoke<AerialPhoto>("aerial_photo");
export const sunDay = (month: number, day: number) => invoke<SunDay>("sun_day", { month, day });
/** Writes a base64 PNG the 3D view took to `path`; returns the path. */
export const saveViewPng = (path: string, png: string) => invoke<string>("save_view_png", { path, png });

export const saveReports = (reports: Report[]) => invoke<string>("save_reports", { reports });
/** One block on its own, `width_mm` by `height_mm`, for the report editor to place. */
export const renderReportBlock = (themeId: string, block: ReportBlock) =>
  invoke<string>("render_report_block", { themeId, block });
/** A page with only its background and page number. */
export const renderReportBackdrop = (report: Report, index: number) =>
  invoke<string>("render_report_backdrop", { report, index });
export const reportFileName = (reportName: string, format: ReportFormat) =>
  invoke<string>("report_file_name", { reportName, format });
/** One PDF, or an SVG or PNG per page next to `path`. Returns the files written. */
export const exportReport = (report: Report, format: ReportFormat, dpi: number, path: string) =>
  invoke<string[]>("export_report", { report, format, dpi, path });
