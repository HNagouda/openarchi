// The live preview: the core's SVG for the current look.
//
// Only one render runs at a time. Changes that arrive meanwhile collapse
// into one follow-up render of the newest look, and the old picture stays
// up until the new one is ready, so the sheet never flickers.

import { useEffect, useRef, useState } from "react";
import { errorText, renderPreview } from "../lib/ipc";
import type { Look } from "../lib/types";

// Short enough to feel live, long enough to skip the middle of a fast drag.
const PREVIEW_DELAY_MS = 40;
// Only show the rendering hint when a render is noticeably slow.
const SLOW_RENDER_MS = 350;

export function usePreview(look: Look, revision: number, isEnabled: boolean) {
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSlow, setSlow] = useState(false);
  const isRunning = useRef(false);
  const wanted = useRef<Look | null>(null);

  const runNext = async () => {
    if (isRunning.current || !wanted.current) return;
    const target = wanted.current;
    wanted.current = null;
    isRunning.current = true;
    const slowTimer = window.setTimeout(() => setSlow(true), SLOW_RENDER_MS);
    try {
      setSvg(await renderPreview(target));
      setError(null);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      window.clearTimeout(slowTimer);
      setSlow(false);
      isRunning.current = false;
      runNext();
    }
  };

  const key = JSON.stringify(look);
  useEffect(() => {
    if (!isEnabled) return;
    const timer = window.setTimeout(() => {
      wanted.current = look;
      runNext();
    }, PREVIEW_DELAY_MS);
    return () => window.clearTimeout(timer);
    // `key` stands for `look`; runNext only reads refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, revision, isEnabled]);

  return { svg, error, isSlow };
}
