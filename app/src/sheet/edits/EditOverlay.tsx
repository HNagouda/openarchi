// The site edits over the sheet while the edit tool is on: removed trees and
// buildings as dashed ghosts, so they can be found and brought back, a moved
// tree's line from where the survey has it, planted trees ringed, and the
// selection. The sheet itself already draws the edited site; these marks only
// show what changed. Points go through the sheet's ground points, as pins do.

import type { Editable } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { useLive } from "../liveValue";
import { rdToPaper, type Point } from "../paper";
import { CROWN_RADIUS_M } from "./editGround";
import type { EditTool } from "./useEditTool";
import "./edits.css";

// A crown is a speck on a large site; its marks stay at least this large on screen.
const MIN_MARK_PX = 6;

type Props = {
  view: PaperView;
  tool: EditTool;
};

export function EditOverlay({ view, tool }: Props) {
  const drag = useLive(tool.drag);
  const toPaper = rdToPaper(view.getPaper());
  if (!tool.isEditing || !toPaper) return null;

  const px = view.pxPerMm;
  const screen = ([x, y]: [number, number]): Point => {
    const paper = toPaper({ x, y });
    return { x: paper.x * px, y: paper.y * px };
  };
  const origin = toPaper({ x: 0, y: 0 });
  const across = toPaper({ x: CROWN_RADIUS_M, y: 0 });
  const crownPx = Math.max(MIN_MARK_PX, Math.hypot(across.x - origin.x, across.y - origin.y) * px);
  const outline = (thing: Editable) =>
    thing.outline.map((ring) => "M" + ring.map((corner) => {
      const point = screen(corner);
      return `${point.x},${point.y}`;
    }).join("L") + "Z").join("");
  const selected = tool.selected;

  return (
    <svg className="edit-overlay" aria-label="Site edits">
      {tool.things.map((thing) => (
        <EditMark key={thing.id} thing={thing} screen={screen} crownPx={crownPx} outline={outline} />
      ))}
      {selected?.layer === "building" && <path className="edit-selected" d={outline(selected)} />}
      {selected?.layer === "tree" && selected.at && <SelectedTree at={screen(selected.at)} crownPx={crownPx} />}
      {drag && <circle className="edit-drag" cx={screen([drag.at.x, drag.at.y]).x} cy={screen([drag.at.x, drag.at.y]).y} r={crownPx} />}
    </svg>
  );
}

type MarkProps = {
  thing: Editable;
  screen: (point: [number, number]) => Point;
  crownPx: number;
  outline: (thing: Editable) => string;
};

function EditMark({ thing, screen, crownPx, outline }: MarkProps) {
  if (thing.layer === "building") return <path className="edit-ghost" d={outline(thing)} />;
  if (!thing.at) return null;
  const at = screen(thing.at);
  if (thing.is_removed) return <circle className="edit-ghost" cx={at.x} cy={at.y} r={crownPx} />;
  if (thing.surveyed_at) {
    const from = screen(thing.surveyed_at);
    return (
      <g>
        <circle className="edit-ghost" cx={from.x} cy={from.y} r={crownPx} />
        <line className="edit-moved" x1={from.x} y1={from.y} x2={at.x} y2={at.y} />
      </g>
    );
  }
  return <circle className="edit-planted" cx={at.x} cy={at.y} r={crownPx} />;
}

function SelectedTree({ at, crownPx }: { at: Point; crownPx: number }) {
  return <circle className="edit-selected" cx={at.x} cy={at.y} r={crownPx + 3} />;
}
