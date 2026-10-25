// The sheet's side of the library previews: one store per open project,
// warmed in the background for the palettes its reports use, and the report
// editor's side, which reads one palette.

import { useEffect, useState, useSyncExternalStore } from "react";
import type { ProjectView } from "../../lib/types";
import { createLibraryPreviews, type LibraryPreviews, type PaletteLibrary } from "./libraryPreviews";

// Let the sheet preview and the diagram thumbnails have their turn first.
const WARM_UP_DELAY_MS = 2500;
// How long a newly chosen sheet palette must stay before its library is warmed.
// Stepping through palettes with [ and ] rests a few seconds on each, and
// warming every one of them would cost the core work nobody asked for.
const SHEET_PALETTE_SETTLE_MS = 30_000;

/** Owned by the sheet. `sheetThemeId` is the first sheet's palette, which reports follow by default. */
export function useLibraryPreviews(project: ProjectView, sheetThemeId: string, isEnabled: boolean): LibraryPreviews {
  const [previews] = useState(createLibraryPreviews);
  // The palette the project opened with counts as settled; a new one has to stay a while first.
  const [settledThemeId, setSettledThemeId] = useState(sheetThemeId);
  const siteKey = `${project.id} ${project.fetched_at}`;
  const themeIds = [...new Set([settledThemeId, ...project.reports.flatMap((report) => (report.theme_id ? [report.theme_id] : []))])];
  const themesKey = themeIds.join(" ");

  useEffect(() => {
    const timer = window.setTimeout(() => setSettledThemeId(sheetThemeId), SHEET_PALETTE_SETTLE_MS);
    return () => window.clearTimeout(timer);
  }, [sheetThemeId]);

  useEffect(() => {
    if (!isEnabled) return;
    previews.reset(siteKey);
    const timer = window.setTimeout(() => previews.warm(themeIds), WARM_UP_DELAY_MS);
    return () => window.clearTimeout(timer);
    // `themesKey` stands for `themeIds`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [previews, siteKey, themesKey, isEnabled]);

  return previews;
}

/** The library in one palette, asked for first because it is on screen. */
export function usePaletteLibrary(previews: LibraryPreviews, themeId: string): PaletteLibrary | undefined {
  useEffect(() => {
    previews.want(themeId);
  }, [previews, themeId]);
  return useSyncExternalStore(previews.subscribe, () => previews.library(themeId));
}
