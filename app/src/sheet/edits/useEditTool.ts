// Editing the site's trees and buildings on a plan sheet: pick one with a
// click, Delete takes it away or brings it back, drag a tree to move it, and
// in planting mode a click plants a new tree.
//
// The edits live in the sheet's undo history, so every change undoes like any
// other edit. Like pins, a drag changes the history only when it is dropped.
// What lies under the pointer comes from the core, which knows the site; the
// tool only turns paper millimetres into RD New metres for it.

import { useEffect, useRef, useState } from "react";
import { errorText, pickOnSite, siteEditThings } from "../../lib/ipc";
import { usePlainKeys } from "../../lib/keys";
import type { Editable, Look, SiteEdits } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { followPointer } from "../../lib/pointerDrag";
import { paperToRd, type Point } from "../paper";
import { CROWN_RADIUS_M, isOnSite, metresPerMm } from "./editGround";
import { hasEdits, withMoved, withPlanted, withRemoved, withRestored } from "./siteEdits";

export const EDIT_KEY = "e";
// A press this close to a tree, in paper millimetres, still picks it, on top of its crown.
const TREE_HIT_MM = 2;
// Arrow keys move a selected tree this far, or the larger step with Shift, as they move the model.
const NUDGE_M = 1;
const BIG_NUDGE_M = 10;
const ARROWS: Record<string, [number, number]> = { ArrowUp: [0, 1], ArrowDown: [0, -1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] };

export type EditMode = "pick" | "plant";
/** A tree on its way to a new place, in RD New metres. */
export type TreeDrag = { id: string; at: Point };

type Options = {
  edits: SiteEdits;
  onEdits: (next: SiteEdits) => void;
  look: Look;
  svg: string | null;
  isEnabled: boolean;
  onRefused: (message: string) => void;
};

export function useEditTool({ edits, onEdits, look, svg, isEnabled, onRefused }: Options) {
  const [isEditing, setEditing] = useState(false);
  const [mode, setMode] = useState<EditMode>("pick");
  const [selected, setSelected] = useState<Editable | null>(null);
  const [things, setThings] = useState<Editable[]>([]);
  const drag = useLiveValue<TreeDrag | null>(null);
  const latest = useRef(edits);
  latest.current = edits;
  // Callers pass a new function each render; the list below must not refetch for that.
  const refuse = useRef(onRefused);
  refuse.current = onRefused;
  const isPlan = look.view.mode === "plan";

  useEffect(() => drag.set(null), [drag, svg]);
  useEffect(() => {
    if (!isEnabled || !isPlan) setEditing(false);
  }, [isEnabled, isPlan]);
  useEffect(() => {
    if (!isEditing) setSelected(null);
  }, [isEditing]);

  // What every edit names, for the list in the inspector and the marks on the sheet.
  useEffect(() => {
    if (!hasEdits(edits)) {
      setThings((current) => (current.length === 0 ? current : []));
      return;
    }
    let isCurrent = true;
    siteEditThings(edits)
      .then((found) => isCurrent && setThings(found))
      .catch((error) => refuse.current(`Couldn't list the site edits. ${errorText(error)}`));
    return () => {
      isCurrent = false;
    };
  }, [edits]);

  // The selection follows undo and redo: as an edit names it, or as the survey
  // has it. The list arrives after each change of the edits, so it alone is watched.
  useEffect(() => {
    setSelected((current) => current && (things.find((thing) => thing.id === current.id) ?? unedited(current, latest.current)));
  }, [things]);

  const toggle = () => {
    if (!isEnabled) return;
    if (!isPlan) {
      onRefused("Trees and buildings are edited on a plan view.");
      return;
    }
    setEditing(!isEditing);
  };

  /** Takes the thing away, or brings it back when it is gone. */
  const toggleRemoved = (thing: Editable) => {
    onEdits(thing.is_removed ? withRestored(latest.current, thing.id) : withRemoved(latest.current, thing.id));
  };

  const restore = (thing: Editable) => onEdits(withRestored(latest.current, thing.id));

  /** Takes the thing away for good when it is planted, however it stands now. */
  const remove = (thing: Editable) => onEdits(withRemoved(latest.current, thing.id));

  const plantAt = (paper: Element | null, at: Point) => {
    if (!isOnSite(paper, at)) {
      onRefused("Trees are planted inside the outline of the site.");
      return;
    }
    const planted = withPlanted(latest.current, at.x, at.y);
    onEdits(planted.edits);
    setSelected(plantedTree(planted.id, at));
  };

  const moveTo = (paper: Element | null, tree: Editable, at: Point): boolean => {
    if (!isOnSite(paper, at)) {
      onRefused("Trees stand inside the outline of the site.");
      return false;
    }
    onEdits(withMoved(latest.current, tree.id, at.x, at.y));
    return true;
  };

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    const paper = view.getPaper();
    const toRd = paperToRd(paper);
    const perMm = metresPerMm(paper);
    if (!toRd || !perMm) return false;
    event.preventDefault();
    const at = toRd(view.toMm(event.clientX, event.clientY));
    if (mode === "plant") {
      plantAt(paper, at);
      return true;
    }

    const reachM = Math.max(CROWN_RADIUS_M, perMm * TREE_HIT_MM);
    let found: Editable | null = null;
    const picking = pickOnSite(latest.current, at.x, at.y, reachM).then((picked) => (found = picked));
    const isMovable = (thing: Editable | null): thing is Editable => thing?.layer === "tree" && !thing.is_removed;
    followPointer(event, {
      onDrag: (move) => {
        if (isMovable(found)) drag.set({ id: found.id, at: toRd(view.toMm(move.clientX, move.clientY)) });
      },
      onDrop: async (up, isDragged) => {
        const picked = await picking.catch(() => null);
        setSelected(picked);
        if (!isDragged || !isMovable(picked)) return;
        const isMoved = moveTo(paper, picked, toRd(view.toMm(up.clientX, up.clientY)));
        if (!isMoved) drag.set(null);
      },
    });
    return true;
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === EDIT_KEY && !event.repeat) toggle();
    if (!isEditing) return;
    if (key === "Escape") {
      if (selected) setSelected(null);
      else setEditing(false);
    }
    if ((key === "Delete" || key === "Backspace") && selected && !event.repeat) toggleRemoved(selected);
    const step = ARROWS[key];
    if (step && selected?.layer === "tree" && selected.at && !selected.is_removed) {
      event.preventDefault();
      const metres = event.shiftKey ? BIG_NUDGE_M : NUDGE_M;
      const paper = document.querySelector(".paper");
      moveTo(paper, selected, { x: selected.at[0] + step[0] * metres, y: selected.at[1] + step[1] * metres });
    }
  }, isEnabled);

  return { isEditing, isPlan, mode, setMode, selected, setSelected, things, drag, toggle, toggleRemoved, restore, remove, onPaperPointerDown };
}

export type EditTool = ReturnType<typeof useEditTool>;

/** A thing no edit names any more, as the survey has it, or null for a planted tree that is gone. */
function unedited(thing: Editable, edits: SiteEdits): Editable | null {
  if (thing.is_planted) {
    const tree = edits.planted?.find((planted) => planted.id === thing.id);
    return tree ? { ...thing, at: [tree.x, tree.y], is_removed: false } : null;
  }
  return { ...thing, at: thing.surveyed_at ?? thing.at, surveyed_at: null, is_removed: false, is_under_model: false };
}

function plantedTree(id: string, at: Point): Editable {
  return {
    id,
    layer: "tree",
    kind: null,
    year_built: null,
    height_m: null,
    at: [at.x, at.y],
    surveyed_at: null,
    outline: [],
    is_removed: false,
    is_under_model: false,
    is_planted: true,
  };
}
