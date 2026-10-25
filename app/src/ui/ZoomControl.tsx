// The zoom pill in the corner of the sheet, the report pages and the site map.
// The sheet and the report show the zoom in the middle and fit on a click of
// it; the map has no zoom percentage worth reading, so it shows a fit button.

import { shortcutLabel } from "../lib/platform";
import { PERCENT } from "../lib/units";
import { Button } from "./Button";

type Props = {
  /** The zoom as a fraction, 1 for 100 %. Left out, the middle is a fit icon instead of the readout. */
  zoom?: number;
  fitTip: string;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFit: () => void;
};

export function ZoomControl({ zoom, fitTip, onZoomIn, onZoomOut, onFit }: Props) {
  return (
    <div className="zoom-control">
      <Button icon="zoomOut" tip="Zoom out" shortcut="mod+-" tipSide="above" onClick={onZoomOut} />
      {zoom === undefined ? (
        <Button icon="fit" tip={fitTip} shortcut="mod+0" tipSide="above" onClick={onFit} />
      ) : (
        <button type="button" className="zoom-value tabular" data-tip={fitTip} data-tip-key={shortcutLabel("mod+0")} data-tip-side="above" onClick={onFit}>
          {Math.round(zoom * PERCENT)}%
        </button>
      )}
      <Button icon="zoomIn" tip="Zoom in" shortcut="mod+=" tipSide="above" onClick={onZoomIn} />
    </div>
  );
}
