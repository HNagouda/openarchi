// Filmstrip pictures: every diagram in the current theme and sheet, rendered
// one at a time in the background after the preview has had its turn.

import { useEffect, useState } from "react";
import { renderThumbnail } from "../lib/ipc";
import type { Diagram, Look } from "../lib/types";

// Thumbnails are about 120 px wide; twice that keeps them sharp on Retina.
const THUMBNAIL_WIDTH_PX = 260;
// Let the preview render first after a change.
const START_DELAY_MS = 450;

/** Diagram id -> PNG data URL. Old pictures stay until their replacement is ready. */
export function useThumbnails(look: Look, diagrams: Diagram[], revision: number, isEnabled: boolean) {
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});

  // Everything but the diagram: the strip only changes when this changes.
  const { diagram_id: _ignored, ...shared } = look;
  const key = JSON.stringify(shared);

  useEffect(() => {
    if (!isEnabled || diagrams.length === 0) return;
    let isCancelled = false;

    const renderAll = async () => {
      for (const diagram of diagrams) {
        if (isCancelled) return;
        try {
          const url = await renderThumbnail({ ...look, diagram_id: diagram.id }, THUMBNAIL_WIDTH_PX);
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
