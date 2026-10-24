// What the report library lists, per palette, with a picture of every block,
// gathered in the background while the user works on the sheet, so the
// library is ready the moment the report editor opens.
//
// It lives outside React state on purpose: a picture arriving must not
// re-render the sheet. Only the report editor subscribes to it.

import { errorText, getAnalysis, getRatios, renderReportBlock } from "../../lib/ipc";
import type { AnalysisPart, AnalysisSection, Ratios, ReportBlock } from "../../lib/types";
import { analysisItems, ratiosItems, type LibraryItem } from "../library";

// Every analysis part, in report order, in one place.
const PARTS: AnalysisPart[] = ["location", "site", "climate", "rain"];
// A pause before each picture, so this work never crowds out the sheet
// preview or a block the user is placing.
const PICTURE_GAP_MS = 120;

export type PaletteLibrary = {
  ratios: Ratios | null;
  parts: Partial<Record<AnalysisPart, AnalysisSection[]>>;
  /** Every part that has arrived, in report order. */
  sections: AnalysisSection[];
  isLoading: boolean;
  /** Parts that could not be fetched, as sentences. */
  problems: string[];
  /** Library item key -> the block drawn by the core at its starting size. */
  pictures: Record<string, string>;
};

export type LibraryPreviews = {
  /** Starts again from nothing for another project or a fresh fetch of the site. */
  reset: (key: string) => void;
  /** Fills these palettes in the background, in this order, replacing the last list. */
  warm: (themeIds: string[]) => void;
  /** Fills this palette before any other: the report editor is showing it. */
  want: (themeId: string) => void;
  library: (themeId: string) => PaletteLibrary | undefined;
  subscribe: (listener: () => void) => () => void;
};

const EMPTY: PaletteLibrary = { ratios: null, parts: {}, sections: [], isLoading: true, problems: [], pictures: {} };

export function createLibraryPreviews(): LibraryPreviews {
  let key = "";
  // Bumped by every reset, so work begun for the old project drops its results.
  let generation = 0;
  let libraries = new Map<string, PaletteLibrary>();
  let complete = new Set<string>();
  let wanted: string | null = null;
  let warmList: string[] = [];
  let isRunning = false;
  const listeners = new Set<() => void>();

  const update = (themeId: string, change: (library: PaletteLibrary) => PaletteLibrary) => {
    const next = new Map(libraries);
    next.set(themeId, change(libraries.get(themeId) ?? EMPTY));
    libraries = next;
    listeners.forEach((listener) => listener());
  };

  /** The palette to work on now: the wanted one, then the warm list, skipping finished ones. */
  const nextPalette = () => {
    const order = wanted ? [wanted, ...warmList] : warmList;
    return order.find((themeId) => !complete.has(themeId)) ?? null;
  };

  const fetchData = async (themeId: string, started: number) => {
    const settle = (change: (library: PaletteLibrary) => PaletteLibrary) => {
      if (started === generation) update(themeId, change);
    };
    const complain = (what: string, reason: unknown) => settle((library) => ({ ...library, problems: [...library.problems, `${what}: ${errorText(reason)}`] }));

    update(themeId, () => EMPTY);
    const ratios = getRatios(themeId, null)
      .then((found) => settle((library) => ({ ...library, ratios: found })))
      .catch((reason) => complain("Ratios", reason));
    const parts = PARTS.map((part) =>
      getAnalysis(part, themeId)
        .then((found) =>
          settle((library) => {
            const next = { ...library.parts, [part]: found };
            return { ...library, parts: next, sections: PARTS.flatMap((entry) => next[entry] ?? []) };
          }),
        )
        .catch((reason) => complain(part[0].toUpperCase() + part.slice(1), reason)),
    );
    await Promise.all([ratios, ...parts]);
    settle((library) => ({ ...library, isLoading: false }));
  };

  const fill = async (themeId: string, started: number) => {
    const known = libraries.get(themeId);
    if (!known || known.isLoading) await fetchData(themeId, started);

    for (const item of itemsToDraw(libraries.get(themeId) ?? EMPTY)) {
      // Stop when the project changed or the editor asked for another palette; the rest waits its turn.
      if (started !== generation || nextPalette() !== themeId) return;
      if (libraries.get(themeId)?.pictures[item.key]) continue;
      await pause(PICTURE_GAP_MS);
      try {
        const svg = await renderReportBlock(themeId, previewBlock(item));
        if (started === generation) update(themeId, (library) => ({ ...library, pictures: { ...library.pictures, [item.key]: svg } }));
      } catch (reason) {
        // The item keeps its glyph; placing it on a page shows the same error in full.
        console.warn(`No library picture for ${item.key}. ${errorText(reason)}`);
      }
    }
    if (started === generation) complete.add(themeId);
  };

  const run = async () => {
    if (isRunning) return;
    isRunning = true;
    const started = generation;
    let themeId = nextPalette();
    while (themeId !== null && started === generation) {
      await fill(themeId, started);
      themeId = nextPalette();
    }
    isRunning = false;
    // A reset during the work: begin again for the new project.
    if (started !== generation) run();
  };

  return {
    reset: (next) => {
      if (next === key) return;
      key = next;
      generation += 1;
      libraries = new Map();
      complete = new Set();
      listeners.forEach((listener) => listener());
    },
    warm: (themeIds) => {
      warmList = themeIds;
      run();
    },
    want: (themeId) => {
      wanted = themeId;
      run();
    },
    library: (themeId) => libraries.get(themeId),
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** The items the core draws a picture for: analysis figures already come with theirs. */
function itemsToDraw(library: PaletteLibrary): LibraryItem[] {
  const items = [...(library.ratios ? ratiosItems(library.ratios) : []), ...analysisItems(library.sections)];
  return items.filter((item) => item.svg === undefined);
}

/** The item as a block at its starting size, which is what dropping it gives. */
function previewBlock(item: LibraryItem): ReportBlock {
  return { id: `preview-${item.key}`, x_mm: 0, y_mm: 0, width_mm: item.size.width, height_mm: item.size.height, content: item.content };
}

function pause(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}
