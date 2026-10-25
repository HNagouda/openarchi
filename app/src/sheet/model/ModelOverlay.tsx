// The model's footprint over the sheet while it is being placed: drag the
// outline to move it, drag the round handle to turn it. Paper points go
// through the sheet's ground points into RD New metres, so the model can be
// placed on a plan or a tilted view alike.

import type { PaperView } from "../Canvas";
import { useLive } from "../liveValue";
import { paperToRd, rdToPaper, type Point } from "../paper";
import { followPointer } from "../../lib/pointerDrag";
import { bearingDeg, footprintOnSite, normalisedTurn } from "./modelEdits";
import type { ModelTool } from "./useModelTool";
import "./model.css";

// The turning handle stands this far north of the footprint, in paper millimetres.
const HANDLE_GAP_MM = 6;
const HANDLE_RADIUS_PX = 6;

type Props = {
  view: PaperView;
  tool: ModelTool;
};

export function ModelOverlay({ view, tool }: Props) {
  const draft = useLive(tool.draft);
  const { model, choice } = tool;
  const paper = view.getPaper();
  const toPaper = rdToPaper(paper);
  const toRd = paperToRd(paper);
  if (!tool.isPlacing || !model || !choice || !toPaper || !toRd) return null;

  const placement = draft ?? choice.placement;
  const px = view.pxPerMm;
  const rings = footprintOnSite(model, placement).map((ring) => ring.map(toPaper));
  const outline = rings.map((ring) => "M" + ring.map((point) => `${point.x * px},${point.y * px}`).join("L") + "Z").join("");
  const centre = toPaper({ x: placement.x, y: placement.y });
  const top = Math.min(...rings.flat().map((point) => point.y));
  const handle = { x: centre.x, y: top - HANDLE_GAP_MM };

  const pointerRd = (event: { clientX: number; clientY: number }): Point => toRd(view.toMm(event.clientX, event.clientY));

  const move = (event: React.PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const start = pointerRd(event);
    const from = choice.placement;
    const at = (pointer: PointerEvent): Point => {
      const now = pointerRd(pointer);
      return { x: from.x + now.x - start.x, y: from.y + now.y - start.y };
    };
    followPointer(event, {
      onDrag: (pointer) => tool.draft.set({ ...from, ...at(pointer) }),
      onDrop: (pointer, isDragged) => {
        if (isDragged) tool.place({ ...from, ...at(pointer) });
      },
    });
  };

  const turn = (event: React.PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const from = choice.placement;
    const startBearing = bearingDeg(from, pointerRd(event));
    // Whole degrees, as the inspector's field shows them.
    const turned = (pointer: PointerEvent) => ({
      ...from,
      rotation_deg: Math.round(normalisedTurn(from.rotation_deg + bearingDeg(from, pointerRd(pointer)) - startBearing)),
    });
    followPointer(event, {
      onDrag: (pointer) => tool.draft.set(turned(pointer)),
      onDrop: (pointer, isDragged) => {
        if (isDragged) tool.place(turned(pointer));
      },
    });
  };

  return (
    <svg className="model-overlay" aria-label="The model: drag to move it, drag the handle to turn it">
      <path className="model-outline" d={outline} onPointerDown={move} />
      <line className="model-handle-arm" x1={centre.x * px} y1={centre.y * px} x2={handle.x * px} y2={handle.y * px} />
      <circle className="model-handle" cx={handle.x * px} cy={handle.y * px} r={HANDLE_RADIUS_PX} onPointerDown={turn} />
    </svg>
  );
}
