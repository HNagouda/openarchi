// Millimetre rulers along the top and the left of the desk, as in a word
// processor, measured from the corner of one page: the current page, or the
// page a block is being dragged on. They follow zoom and scroll, and mark the
// extent of the selected block, strongly while it moves.
//
// Scrolling only moves the rulers: they measure the page themselves, so the
// editor does not re-render on every scroll.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import type { Rect, Size } from "../geometry";
import "./rulers.css";

/** How thick a ruler is; rulers.css keeps the pages clear of it. */
export const RULER_PX = 20;
// Tick spacings in millimetres, finest first.
const STEPS_MM = [1, 2, 5, 10, 20, 50, 100, 200];
// The finest spacing that still reads as a scale, and the room a number like "420" needs.
const MIN_TICK_PX = 5;
const MIN_LABEL_PX = 34;
const TICK_PX = { minor: 4, half: 7, major: 11 };
const LABEL_INSET_PX = 2;
// Numbers sit in the outer half of a ruler, clear of the longest ticks.
const LABEL_BASELINE_PX = 9;

/** A block's box on the measured page; `isMoving` while it is dragged, resized or dropped. */
export type Extent = { rect: Rect; isMoving: boolean };

type Props = {
  /** The desk, which scrolls the pages; scroll events are caught on their way down to it. */
  desk: RefObject<HTMLElement | null>;
  pageIndex: number;
  page: Size;
  pxPerMm: number;
  extent: Extent | null;
};

type Origin = { x: number; y: number };

export function Rulers({ desk, pageIndex, page, pxPerMm, extent }: Props) {
  const top = useRef<HTMLDivElement>(null);
  const left = useRef<HTMLDivElement>(null);
  const [origin, setOrigin] = useState<Origin | null>(null);

  const measure = useCallback(() => {
    const paper = desk.current?.querySelector(`[data-report-page="${pageIndex}"]`);
    if (!paper || !top.current || !left.current) return;
    const box = paper.getBoundingClientRect();
    const x = Math.round(box.left - top.current.getBoundingClientRect().left);
    const y = Math.round(box.top - left.current.getBoundingClientRect().top);
    setOrigin((current) => (current?.x === x && current.y === y ? current : { x, y }));
  }, [desk, pageIndex]);

  // After every render: a zoom, a new page or a column resize all move the page.
  useLayoutEffect(measure);

  useEffect(() => {
    const element = desk.current;
    if (!element) return;
    let frame = 0;
    const onScroll = () => {
      window.cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(measure);
    };
    const observer = new ResizeObserver(onScroll);
    observer.observe(element);
    element.addEventListener("scroll", onScroll, { capture: true, passive: true });
    return () => {
      window.cancelAnimationFrame(frame);
      observer.disconnect();
      element.removeEventListener("scroll", onScroll, { capture: true });
    };
  }, [desk, measure]);

  const minor = STEPS_MM.find((step) => step * pxPerMm >= MIN_TICK_PX) ?? STEPS_MM[STEPS_MM.length - 1];
  const major = STEPS_MM.find((step) => step * pxPerMm >= MIN_LABEL_PX && step % minor === 0) ?? STEPS_MM[STEPS_MM.length - 1];
  const scale = { minor, major, pxPerMm };

  return (
    <>
      <div className="report-ruler is-top" ref={top} aria-hidden>
        {origin && (
          <svg width="100%" height="100%">
            <Scale axis="x" start={origin.x} length={page.width} scale={scale} extent={extent && { from: extent.rect.x, to: extent.rect.x + extent.rect.width, isMoving: extent.isMoving }} />
          </svg>
        )}
      </div>
      <div className="report-ruler is-left" ref={left} aria-hidden>
        {origin && (
          <svg width="100%" height="100%">
            <Scale axis="y" start={origin.y} length={page.height} scale={scale} extent={extent && { from: extent.rect.y, to: extent.rect.y + extent.rect.height, isMoving: extent.isMoving }} />
          </svg>
        )}
      </div>
      <div className="report-ruler-corner" aria-hidden>
        mm
      </div>
    </>
  );
}

type ScaleProps = {
  axis: "x" | "y";
  /** Where the page's edge is along the ruler, in pixels. */
  start: number;
  /** The page's width or height in millimetres. */
  length: number;
  scale: { minor: number; major: number; pxPerMm: number };
  extent: { from: number; to: number; isMoving: boolean } | null;
};

/** One ruler's paper, ticks, numbers and the marked extent, drawn along `axis`. */
function Scale({ axis, start, length, scale, extent }: ScaleProps) {
  const { minor, major, pxPerMm } = scale;
  const along = (mm: number) => start + mm * pxPerMm;
  // A rectangle `from` to `to` along the ruler, the full thickness across it.
  const band = (from: number, to: number) =>
    axis === "x" ? { x: along(from), y: 0, width: (to - from) * pxPerMm, height: RULER_PX } : { x: 0, y: along(from), width: RULER_PX, height: (to - from) * pxPerMm };

  const ticks = [];
  for (let mm = 0; mm <= length; mm += minor) {
    const isMajor = mm % major === 0;
    const size = isMajor ? TICK_PX.major : mm % (major / 2) === 0 ? TICK_PX.half : TICK_PX.minor;
    const at = along(mm);
    ticks.push(
      axis === "x" ? (
        <line key={mm} x1={at} x2={at} y1={RULER_PX - size} y2={RULER_PX} />
      ) : (
        <line key={mm} y1={at} y2={at} x1={RULER_PX - size} x2={RULER_PX} />
      ),
    );
    if (isMajor && mm > 0) {
      ticks.push(
        axis === "x" ? (
          <text key={`label${mm}`} x={at + LABEL_INSET_PX} y={LABEL_BASELINE_PX}>
            {mm}
          </text>
        ) : (
          // Turned to run along the ruler, reading upwards, just below its tick.
          <text key={`label${mm}`} x={LABEL_BASELINE_PX} y={at + LABEL_INSET_PX} textAnchor="end" transform={`rotate(-90 ${LABEL_BASELINE_PX} ${at + LABEL_INSET_PX})`}>
            {mm}
          </text>
        ),
      );
    }
  }

  return (
    <>
      <rect className="report-ruler-paper" {...band(0, length)} />
      {extent && <rect className={extent.isMoving ? "report-ruler-extent is-moving" : "report-ruler-extent"} {...band(extent.from, extent.to)} />}
      <g className="report-ruler-ticks">{ticks}</g>
    </>
  );
}
