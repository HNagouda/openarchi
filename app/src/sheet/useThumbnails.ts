// Diagram column pictures: every diagram in the current theme and sheet, rendered
// one at a time in the background after the preview has had its turn.

import { useEffect, useState } from "react";
import { renderThumbnail } from "../lib/ipc";
import type { Diagram, Look, Sheet } from "../lib/types";

// Thumbnails are about 64 px wide in the column; twice that keeps them sharp on Retina.
const THUMBNAIL_WIDTH_PX = 160;
// Let the preview render first after a change.
const START_DELAY_MS = 450;

/** The sheet with its title, legend and other text left off. Text cannot be
 * read at thumbnail size, and leaving it out means typing a title does not
 * redraw every picture. */
function withoutText(sheet: Sheet): Sheet {
  return {
    ...sheet,
    title: "",
    subtitle: "",
    texts: [],
    show_legend: false,
    show_north_arrow: false,
    show_scale_bar: false,
    show_credits: false,
    title_style: {},
    subtitle_style: {},
    title_position: null,
    subtitle_position: null,
  };
}

/** Diagram id -> PNG data URL. Old pictures stay until their replacement is ready. */
export function useThumbnails(look: Look, diagrams: Diagram[], revision: number, isEnabled: boolean) {
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});

  // Everything but the diagram, its variant, the sheet's text and the view mode: the pictures only change when this changes.
  const { diagram_id: _diagram, variant: _variant, ...rest } = look;
  const shared = { ...rest, sheet: withoutText(look.sheet), view: { ...look.view, mode: "plan" as const } };
  const key = JSON.stringify(shared);

  useEffect(() => {
    if (!isEnabled || diagrams.length === 0) return;
    let isCancelled = false;

    const renderAll = async () => {
      for (const diagram of diagrams) {
        if (isCancelled) return;
        try {
          // Every picture shows its plan: an explode view stacks the same plates whatever the
          // diagram, and a tilted view of a large site takes ten times as long to draw.
          // A diagram with variants is shown as its first.
          const url = await renderThumbnail({ ...shared, diagram_id: diagram.id, variant: null }, THUMBNAIL_WIDTH_PX);
          if (!isCancelled) setThumbnails((current) => ({ ...current, [diagram.id]: url }));
        } catch {
          // A diagram that cannot render keeps its placeholder; the preview reports the error.
        }
      }
    };

    const timer = window.setTimeout(renderAll, START_DELAY_MS);
    return () => {
      isCancelled = true;
      window.clearTimeout(timer);
    };
    // `key` stands for everything in `look` except the diagram.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, diagrams, revision, isEnabled]);

  return thumbnails;
}
