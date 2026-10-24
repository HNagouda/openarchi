// One typed function per Rust command. The Rust names are in
// src-tauri/src/commands; errors arrive as plain sentences.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AnalysisPart,
  AnalysisSection,
  Area,
  AreaSize,
  Catalog,
  ChartStyle,
  ExportFormat,
  Look,
  Place,
  Progress,
  ProjectSummary,
  ProjectView,
  Ratios,
  Report,
  ReportBlock,
  ReportFormat,
} from "./types";

export const libraryPath = () => invoke<string>("library_path");
export const listProjects = () => invoke<ProjectSummary[]>("list_projects");
export const listTrash = () => invoke<ProjectSummary[]>("list_trash");
export const createProject = (name: string, area: Area) =>
  invoke<ProjectView>("create_project", { name, area });
export const openProject = (id: string) => invoke<ProjectView>("open_project", { id });
export const reloadProject = () => invoke<ProjectView>("reload_project");
export const closeProject = () => invoke<void>("close_project");
export const saveLook = (look: Look) => invoke<string>("save_look", { look });
export const renameProject = (id: string, name: string) =>
  invoke<void>("rename_project", { id, name });
export const deleteProject = (id: string) => invoke<void>("delete_project", { id });
export const restoreProject = (id: string) => invoke<void>("restore_project", { id });

export const getCatalog = () => invoke<Catalog>("get_catalog");
export const renderPreview = (look: Look) => invoke<string>("render_preview", { look });
export const renderThumbnail = (look: Look, width: number) =>
  invoke<string>("render_thumbnail", { look, width });
export const projectThumbnail = (id: string, modified: string, width: number) =>
  invoke<string | null>("project_thumbnail", { id, modified, width });

export const searchPlaces = (query: string) => invoke<Place[]>("search_places", { query });
export const measureArea = (area: Area) => invoke<AreaSize>("measure_area", { area });
export const fetchSite = (area: Area) => invoke<number>("fetch_site", { area });
export const cancelFetch = () => invoke<void>("cancel_fetch");
export const currentProject = () => invoke<ProjectView>("current_project");

export const exportSheet = (look: Look, format: ExportFormat, dpi: number, path: string) =>
  invoke<string>("export_sheet", { look, format, dpi, path });
export const exportSeries = (look: Look, format: ExportFormat, dpi: number, folder: string) =>
  invoke<string[]>("export_series", { look, format, dpi, folder });
export const revealPath = (path: string) => invoke<void>("reveal_path", { path });
export const openLink = (url: string) => invoke<void>("open_link", { url });

export type FetchProgressEvent = { generation: number; progress: Progress };
export type FetchFinishedEvent = { generation: number; error: string | null };

export const onFetchProgress = (handler: (event: FetchProgressEvent) => void): Promise<UnlistenFn> =>
  listen<FetchProgressEvent>("fetch-progress", (event) => handler(event.payload));
export const onFetchFinished = (handler: (event: FetchFinishedEvent) => void): Promise<UnlistenFn> =>
  listen<FetchFinishedEvent>("fetch-finished", (event) => handler(event.payload));
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

export const getRatios = (themeId: string) => invoke<Ratios>("get_ratios", { themeId });
export const ratiosFileName = () => invoke<string>("ratios_file_name");
/** CSV or SVG, by the extension of `path`. */
export const exportRatios = (themeId: string, style: ChartStyle, path: string) =>
  invoke<string>("export_ratios", { themeId, style, path });

export const getAnalysis = (part: AnalysisPart, themeId: string) =>
  invoke<AnalysisSection[]>("get_analysis", { part, themeId });
/** With a section id, that section's name; without, the whole analysis's. */
export const analysisFileName = (sectionId: string | null) => invoke<string>("analysis_file_name", { sectionId });
/** SVG or PDF, by the extension of `path`. Without a section id, the whole analysis. */
export const exportAnalysis = (themeId: string, sectionId: string | null, path: string) =>
  invoke<string>("export_analysis", { themeId, sectionId, path });

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
