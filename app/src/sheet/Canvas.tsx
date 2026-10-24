// The sheet on a neutral canvas, with pan and zoom.
//
// The SVG is laid out at the zoomed size rather than scaled with a CSS
// transform, so WebKit redraws the vectors and the sheet stays crisp at
// every zoom. Only the offset uses a transform.

import { useCallback, useEffect, useImperativeHandle, useRef, useState, type Ref } from "react";
import { clamp } from "../lib/format";

const MM_TO_PX = 96 / 25.4;
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 8;
const ZOOM_STEP = 1.25;
// Space kept around the sheet when fitting it.
const FIT_PADDING_PX = 48;
// Trackpad pinch arrives as small ctrl-wheel deltas; this turns them into a smooth factor.
const PINCH_SENSITIVITY = 0.01;

export type CanvasControls = { zoomIn: () => void; zoomOut: () => void; fit: () => void };

type Size = { width: number; height: number };
type View = { zoom: number; x: number; y: number };

/** The sheet size in CSS pixels at 100 %, read from the SVG's width and height in millimetres. */
function sheetSize(svg: string): Size | null {
  const openingTag = /<svg\b[^>]*>/i.exec(svg)?.[0] ?? "";
  const read = (name: string) => {
    const match = new RegExp(`\\b${name}="([\\d.]+)(mm|px)?"`).exec(openingTag);
    if (!match) return null;
    const value = Number(match[1]);
    return match[2] === "mm" ? value * MM_TO_PX : value;
  };
  const width = read("width");
  const height = read("height");
  if (width && height) return { width, height };

  const viewBox = /viewBox="[\d.-]+\s+[\d.-]+\s+([\d.]+)\s+([\d.]+)"/.exec(openingTag);
  if (!viewBox) return null;
  return { width: Number(viewBox[1]) * MM_TO_PX, height: Number(viewBox[2]) * MM_TO_PX };
}

type Props = {
  svg: string | null;
  controls: Ref<CanvasControls>;
  onZoom: (zoom: number) => void;
};

export function Canvas({ svg, controls, onZoom }: Props) {
  const viewport = useRef<HTMLDivElement>(null);
  const [view, setView] = useState<View>({ zoom: 1, x: 0, y: 0 });
  const [isPanning, setPanning] = useState(false);
  const size = svg ? sheetSize(svg) : null;
  const sizeKey = size ? `${Math.round(size.width)}x${Math.round(size.height)}` : "";
  const latest = useRef({ view, size });
  latest.current = { view, size };

  const fit = useCallback(() => {
    const element = viewport.current;
    const current = latest.current.size;
    if (!element || !current) return;
    const zoom = clamp(
      Math.min(
        (element.clientWidth - FIT_PADDING_PX * 2) / current.width,
        (element.clientHeight - FIT_PADDING_PX * 2) / current.height,
      ),
      MIN_ZOOM,
      MAX_ZOOM,
    );
    setView({
      zoom,
      x: (element.clientWidth - current.width * zoom) / 2,
      y: (element.clientHeight - current.height * zoom) / 2,
    });
  }, []);

  /** Zooms by `factor`, keeping the point under (`atX`, `atY`) in place. */
  const zoomBy = useCallback((factor: number, atX?: number, atY?: number) => {
    const element = viewport.current;
    if (!element) return;
    const anchorX = atX ?? element.clientWidth / 2;
    const anchorY = atY ?? element.clientHeight / 2;
    setView((current) => {
      const zoom = clamp(current.zoom * factor, MIN_ZOOM, MAX_ZOOM);
      const applied = zoom / current.zoom;
      return { zoom, x: anchorX - (anchorX - current.x) * applied, y: anchorY - (anchorY - current.y) * applied };
    });
  }, []);

  useImperativeHandle(controls, () => ({
    zoomIn: () => zoomBy(ZOOM_STEP),
    zoomOut: () => zoomBy(1 / ZOOM_STEP),
    fit,
  }));

  // Fit when the first sheet arrives and whenever the paper size changes.
  useEffect(() => {
    if (sizeKey) fit();
  }, [sizeKey, fit]);

  useEffect(() => onZoom(view.zoom), [view.zoom, onZoom]);

  // Keep the sheet fitted while the window or the inspector resizes, if it was fitted.
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    let lastWidth = element.clientWidth;
    let lastHeight = element.clientHeight;
    const observer = new ResizeObserver(() => {
      const dx = (element.clientWidth - lastWidth) / 2;
      const dy = (element.clientHeight - lastHeight) / 2;
      lastWidth = element.clientWidth;
      lastHeight = element.clientHeight;
      setView((current) => ({ ...current, x: current.x + dx, y: current.y + dy }));
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Wheel and pinch need a non-passive listener to stop the page from zooming.
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;

    // WebKit reports trackpad pinches as gesture events with a running scale;
    // other engines send ctrl-wheel. Whichever comes first wins during a pinch.
    let lastScale = 1;
    let isGesturing = false;

    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      if (event.ctrlKey || event.metaKey) {
        if (isGesturing) return;
        zoomBy(Math.exp(-event.deltaY * PINCH_SENSITIVITY), event.clientX - rect.left, event.clientY - rect.top);
        return;
      }
      setView((current) => ({ ...current, x: current.x - event.deltaX, y: current.y - event.deltaY }));
    };

    const onGestureStart = (event: Event) => {
      event.preventDefault();
      lastScale = 1;
      isGesturing = true;
    };
    const onGestureEnd = () => {
      isGesturing = false;
    };
    const onGestureChange = (event: Event) => {
      event.preventDefault();
      const gesture = event as Event & { scale: number; clientX: number; clientY: number };
      const rect = element.getBoundingClientRect();
      zoomBy(gesture.scale / lastScale, gesture.clientX - rect.left, gesture.clientY - rect.top);
      lastScale = gesture.scale;
    };

    element.addEventListener("wheel", onWheel, { passive: false });
    element.addEventListener("gesturestart", onGestureStart);
    element.addEventListener("gesturechange", onGestureChange);
    element.addEventListener("gestureend", onGestureEnd);
    return () => {
      element.removeEventListener("gestureend", onGestureEnd);
      element.removeEventListener("wheel", onWheel);
      element.removeEventListener("gesturestart", onGestureStart);
      element.removeEventListener("gesturechange", onGestureChange);
    };
  }, [zoomBy]);

  const startPan = (event: React.PointerEvent) => {
    if (event.button !== 0 && event.button !== 1) return;
    const start = { x: event.clientX, y: event.clientY, view: latest.current.view };
    setPanning(true);
    const onMove = (move: PointerEvent) => {
      setView({ ...start.view, x: start.view.x + move.clientX - start.x, y: start.view.y + move.clientY - start.y });
    };
    const onUp = () => {
      setPanning(false);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  return (
    <div ref={viewport} className={isPanning ? "canvas is-panning" : "canvas"} onPointerDown={startPan} onDoubleClick={fit}>
      {svg && size && (
        <div
          className="paper"
          style={{
            width: size.width * view.zoom,
            height: size.height * view.zoom,
            transform: `translate(${view.x}px, ${view.y}px)`,
          }}
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      )}
    </div>
  );
}
