// The sheet's zoom pill. The readout follows the zoom on its own, so a pinch
// renders only this, not the workspace.

import type { RefObject } from "react";
import { ZoomControl } from "../ui/ZoomControl";
import type { CanvasControls } from "./Canvas";
import { useLive, type LiveValue } from "./liveValue";

type Props = {
  zoom: LiveValue<number>;
  canvas: RefObject<CanvasControls | null>;
};

export function SheetZoomControl({ zoom, canvas }: Props) {
  const shown = useLive(zoom);

  return (
    <ZoomControl
      zoom={shown}
      fitTip="Fit sheet"
      onZoomIn={() => canvas.current?.zoomIn()}
      onZoomOut={() => canvas.current?.zoomOut()}
      onFit={() => canvas.current?.fit()}
    />
  );
}
