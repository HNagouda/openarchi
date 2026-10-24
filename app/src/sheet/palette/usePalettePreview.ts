// The core's view of a draft: how it filled each role, its warnings, and the
// open sheet drawn in it. Asked again a moment after the last edit, so a drag
// in the colour picker does not queue a render per pixel.

import { useEffect, useState } from "react";
import { errorText } from "../../lib/ipc";
import { checkPalette, renderPalettePreview, type PaletteCheck, type PaletteDraft } from "../../lib/palette";
import type { Look } from "../../lib/types";

const SETTLE_MS = 160;

export function usePalettePreview(draft: PaletteDraft, look: Look) {
  const [check, setCheck] = useState<PaletteCheck | null>(null);
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isCurrent = true;
    const timer = window.setTimeout(() => {
      Promise.all([checkPalette(draft), renderPalettePreview(look, draft)])
        .then(([checked, drawn]) => {
          if (!isCurrent) return;
          setCheck(checked);
          setSvg(drawn);
          setError(null);
        })
        .catch((reason) => isCurrent && setError(errorText(reason)));
    }, SETTLE_MS);
    return () => {
      isCurrent = false;
      window.clearTimeout(timer);
    };
  }, [draft, look]);

  return { check, svg, error };
}
