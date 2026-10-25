// The imported model: importing it, reading it in another unit, removing it,
// and moving and turning it on the sheet.
//
// Which model is in effect and where it stands live in the sheet's undo
// history, so every change undoes like any other edit. The app keeps every
// model read this session by key, and this hook hands it the choice after each
// change; the preview draws again once the app has it. Like pins, a drag only
// changes the history when it is dropped, and the drop picks again the
// buildings the model replaces.

import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { coveredBuildings, errorText, importModel, rereadModel, setModel } from "../../lib/ipc";
import { usePlainKeys } from "../../lib/keys";
import type { ModelChoice, ModelUnit, ModelView, Placement, ProjectView, UpAxis } from "../../lib/types";
import { useToast } from "../../ui/Toasts";
import { useLiveValue } from "../liveValue";
import { replacedBy, sizeLabel } from "./modelEdits";

export const MODEL_KEY = "m";
// The extensions the core reads; see READABLE_FORMATS in crates/core/src/model.rs.
const MODEL_EXTENSIONS = ["obj", "glb", "gltf"];
// Arrow keys move a placed model this far, or the larger step with Shift.
const NUDGE_M = 1;
const BIG_NUDGE_M = 10;

type Options = {
  project: ProjectView;
  choice: ModelChoice | null;
  onModel: (next: ModelChoice | null, mergeKey?: string) => void;
  onSeal: () => void;
  /** The app holds the new choice: draw the sheet again. */
  onApplied: () => void;
  isEnabled: boolean;
  toast: ReturnType<typeof useToast>;
  onUndo: () => void;
};

export function useModelTool({ project, choice, onModel, onSeal, onApplied, isEnabled, toast, onUndo }: Options) {
  const [models, setModels] = useState<Record<string, ModelView>>(() => (project.model ? { [project.model.key]: project.model } : {}));
  const [isPlacing, setPlacing] = useState(false);
  // The placement while the footprint is dragged or turned, until it is dropped.
  const draft = useLiveValue<Placement | null>(null);
  const applied = useRef(JSON.stringify(choice));
  // Each placement is one undo step, though it changes the history twice.
  const placements = useRef(0);
  const latest = useRef(choice);
  latest.current = choice;
  const model = choice ? models[choice.key] ?? null : null;

  // A model the app sent again after an assistant's edit is known by its new key.
  useEffect(() => {
    const fresh = project.model;
    if (fresh) setModels((known) => ({ ...known, [fresh.key]: fresh }));
  }, [project.model]);

  useEffect(() => {
    const wanted = JSON.stringify(choice);
    if (wanted === applied.current) return;
    applied.current = wanted;
    setModel(choice)
      .then(onApplied)
      .catch((error) => toast(`Couldn't place the model. ${errorText(error)}`, { isError: true }));
  }, [choice, onApplied, toast]);

  useEffect(() => {
    if (!isEnabled || !choice) setPlacing(false);
  }, [isEnabled, choice]);

  const keep = (view: ModelView) => setModels((known) => ({ ...known, [view.key]: view }));

  const importFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "3D model", extensions: MODEL_EXTENSIONS }] });
    if (typeof path !== "string") return;
    try {
      // A model imported again, after a change in Archicad, stands where the last one stood.
      const view = await importModel(path, null, null, latest.current?.placement ?? null);
      keep(view);
      onModel({ key: view.key, placement: view.placement, replaced: view.replaced });
      toast(`Imported ${view.file_name}, ${sizeLabel(view)}`);
    } catch (error) {
      toast(`Couldn't import the model. ${errorText(error)}`, { isError: true });
    }
  };

  const reread = async (unit: ModelUnit, up: UpAxis) => {
    const current = latest.current;
    if (!current) return;
    try {
      const view = await rereadModel(current.key, unit, up);
      keep(view);
      onModel({ ...current, key: view.key });
    } catch (error) {
      toast(`Couldn't read the model that way. ${errorText(error)}`, { isError: true });
    }
  };

  const remove = () => {
    onModel(null);
    toast("Removed the model", { action: { label: "Undo", run: onUndo } });
  };

  /** Stands the model at `placement` and picks again what it replaces, as one undo step, or part of the step `mergeKey` names. */
  const place = async (placement: Placement, mergeKey?: string) => {
    const current = latest.current;
    if (!current) return;
    placements.current += 1;
    mergeKey ??= `model-place-${placements.current}`;
    onModel({ ...current, placement }, mergeKey);
    try {
      const covered = await coveredBuildings({ ...current, placement });
      onModel({ ...current, placement, replaced: replacedBy(covered) }, mergeKey);
    } catch (error) {
      toast(`Couldn't find the buildings under the model. ${errorText(error)}`, { isError: true });
    }
    draft.set(null);
  };

  const togglePlacing = () => {
    if (isEnabled && latest.current) setPlacing(!isPlacing);
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === MODEL_KEY && !event.repeat) togglePlacing();
    if (key === "Escape") setPlacing(false);
    const steps: Record<string, [number, number]> = { ArrowUp: [0, 1], ArrowDown: [0, -1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] };
    const current = latest.current;
    if (!isPlacing || !current || !steps[key]) return;
    event.preventDefault();
    const metres = event.shiftKey ? BIG_NUDGE_M : NUDGE_M;
    const [east, north] = steps[key];
    place({ ...current.placement, x: current.placement.x + east * metres, y: current.placement.y + north * metres });
  }, isEnabled);

  return { model, models, choice, isPlacing, draft, togglePlacing, importFile, reread, remove, place, onReplaced: onModel, onSeal };
}

export type ModelTool = ReturnType<typeof useModelTool>;
