// The zoom buttons and readout in the corner of the canvas. The readout
// follows the zoom on its own, so a pinch renders only this, not the workspace.

import type { RefObject } from "react";
import { shortcutLabel } from "../lib/platform";
import { Button } from "../ui/Button";
import type { CanvasControls } from "./Canvas";
import { useLive, type LiveValue } from "./liveValue";

const PERCENT = 100;

type Props = {
  zoom: LiveValue<number>;
  canvas: RefObject<CanvasControls | null>;
};

export function ZoomControl({ zoom, canvas }: Props) {
  const shown = useLive(zoom);

  return (
    <div className="zoom-control">
      <Button icon="zoomOut" tip="Zoom out" shortcut="mod+-" tipSide="above" onClick={() => canvas.current?.zoomOut()} />
      <button type="button" className="zoom-value tabular" data-tip="Fit sheet" data-tip-key={shortcutLabel("mod+0")} data-tip-side="above" onClick={() => canvas.current?.fit()}>
        {Math.round(shown * PERCENT)}%
      </button>
      <Button icon="zoomIn" tip="Zoom in" shortcut="mod+=" tipSide="above" onClick={() => canvas.current?.zoomIn()} />
    </div>
  );
}
