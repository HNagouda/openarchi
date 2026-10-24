// The line across the explode stack that shows where a dragged plate will
// land: through the middle of the slot it will take, edge to edge.

import type { PaperView } from "../Canvas";
import type { PlateDrag } from "./usePlateDrag";
import "./plates.css";

type Props = { view: PaperView; drag: PlateDrag | null };

export function PlateDropLine({ view, drag }: Props) {
  if (!drag || drag.to === drag.from) return null;

  const slot = drag.plates[drag.to];
  const px = view.pxPerMm;
  return (
    <div
      className="plate-drop-line"
      style={{ left: slot.left * px, top: slot.centreY * px, width: (slot.right - slot.left) * px }}
      aria-hidden="true"
    />
  );
}
