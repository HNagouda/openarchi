// A block's picture, drawn by the core at the block's size in the report's
// palette. Moving a block never redraws it; a new size, palette or content
// does. Pictures are kept per editor, so undo and redo show old ones at once.

import { createContext, useContext, useEffect, useState } from "react";
import { errorText, renderReportBlock } from "../lib/ipc";
import type { ReportBlock } from "../lib/types";

// A resize sends many sizes; draw the one it settles on.
const RENDER_DELAY_MS = 120;

export type BlockPictures = Map<string, string>;

/** The editor's pictures, keyed by everything that changes one. */
export const BlockPicturesContext = createContext<BlockPictures>(new Map());

/**
 * `lookKey` stands for the sheet's look, which sheet blocks follow: a change
 * of paper, view or text redraws them.
 */
export function useBlockSvg(themeId: string, block: ReportBlock, lookKey: string) {
  const pictures = useContext(BlockPicturesContext);
  const sheetPart = block.content.kind === "sheet" ? lookKey : "";
  const key = JSON.stringify([themeId, block.content, block.width_mm, block.height_mm, sheetPart]);
  const [svg, setSvg] = useState<string | null>(() => pictures.get(key) ?? null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const known = pictures.get(key);
    if (known) {
      setSvg(known);
      setError(null);
      return;
    }
    let isCancelled = false;
    const timer = window.setTimeout(async () => {
      try {
        const drawn = await renderReportBlock(themeId, block);
        pictures.set(key, drawn);
        if (!isCancelled) {
          setSvg(drawn);
          setError(null);
        }
      } catch (reason) {
        if (!isCancelled) setError(errorText(reason));
      }
    }, RENDER_DELAY_MS);

    return () => {
      isCancelled = true;
      window.clearTimeout(timer);
    };
    // `key` stands for the parts of `block` that change its picture; its position does not.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, pictures]);

  return { svg, error };
}
