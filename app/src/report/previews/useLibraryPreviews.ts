// The sheet's side of the library previews: one store per open project,
// warmed for the palettes its reports use a little after the site or a
// palette changes, and the report editor's side, which reads one palette.

import { useEffect, useState, useSyncExternalStore } from "react";
import type { ProjectView } from "../../lib/types";
import { createLibraryPreviews, type LibraryPreviews, type PaletteLibrary } from "./libraryPreviews";

// Let the sheet preview and the diagram thumbnails have their turn first.
const WARM_UP_DELAY_MS = 2500;

/** Owned by the sheet. `sheetThemeId` is the first sheet's palette, which reports follow by default. */
export function useLibraryPreviews(project: ProjectView, sheetThemeId: string, isEnabled: boolean): LibraryPreviews {
  const [previews] = useState(createLibraryPreviews);
  const siteKey = `${project.id} ${project.fetched_at}`;
  const themeIds = [...new Set([sheetThemeId, ...project.reports.map((report) => report.theme_id ?? sheetThemeId)])];
  const themesKey = themeIds.join(" ");

  useEffect(() => {
    if (!isEnabled) return;
    previews.reset(siteKey);
    // Stepping through palettes on the sheet only warms the one it settles on.
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
