// The live preview: the core's SVG for the current look and the project's pins and highlights.
//
// Only one render runs at a time. Changes that arrive meanwhile collapse
// into one follow-up render of the newest look, and the old picture stays
// up until the new one is ready, so the sheet never flickers. That queue
// is the only throttle: a slider drag redraws as fast as renders finish,
// rather than waiting for the pointer to rest.
//
// The last few pictures are kept, so stepping back to a diagram or undoing
// shows the earlier sheet at once.

import { useEffect, useRef, useState } from "react";
import { errorText, renderPreview } from "../lib/ipc";
import type { Highlight, Look, Pin } from "../lib/types";

// Enough for stepping through neighbouring diagrams and a few undos; a large sheet is a few MB.
const KEPT_PICTURES = 8;
// Only show the rendering hint when a render is noticeably slow.
const SLOW_RENDER_MS = 350;

type Wanted = { key: string; look: Look; pins: Pin[]; highlights: Highlight[] };

export function usePreview(look: Look, pins: Pin[], highlights: Highlight[], revision: number, isEnabled: boolean) {
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSlow, setSlow] = useState(false);
  const isRunning = useRef(false);
  const wanted = useRef<Wanted | null>(null);
  // Key -> SVG, oldest first. Cleared when the revision changes, since the same look then draws differently.
  const kept = useRef(new Map<string, string>());
  const keptRevision = useRef(revision);
  const currentKey = useRef("");

  const runNext = async () => {
    if (isRunning.current || !wanted.current) return;
    const target = wanted.current;
    wanted.current = null;
    isRunning.current = true;
    const slowTimer = window.setTimeout(() => setSlow(true), SLOW_RENDER_MS);
    try {
      const drawn = await renderPreview(target.look, target.pins, target.highlights);
      keep(target.key, drawn);
      // A render overtaken by a newer one still shows while that one runs, so a
      // drag stays live; one overtaken by a kept picture would only undo it.
      if (target.key === currentKey.current || wanted.current !== null) setSvg(drawn);
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

  const keep = (key: string, drawn: string) => {
    kept.current.delete(key);
    kept.current.set(key, drawn);
    if (kept.current.size > KEPT_PICTURES) {
      const oldest = kept.current.keys().next().value;
      if (oldest !== undefined) kept.current.delete(oldest);
    }
  };

  // A sheet that hides pins or highlights does not redraw when they change,
  // unless its View eye stands on a pin and follows it.
  const isPinFree = look.hide_pins && !look.view_pin;
  const key = JSON.stringify([look, isPinFree ? null : pins, look.hide_highlights ? null : highlights]);
  useEffect(() => {
    if (!isEnabled) return;
    currentKey.current = key;
    if (keptRevision.current !== revision) {
      kept.current.clear();
      keptRevision.current = revision;
    }
    const earlier = kept.current.get(key);
    if (earlier !== undefined) {
      // Drop any render still wanted for a look that is no longer current.
      wanted.current = null;
      keep(key, earlier);
      setSvg(earlier);
      return;
    }
    wanted.current = { key, look, pins, highlights };
    runNext();
    // `key` stands for `look`, `pins` and `highlights`; runNext only reads refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, revision, isEnabled]);

  return { svg, error, isSlow };
}
