// The imported model: importing it, reading it in another unit, removing it,
// and moving and turning it on the sheet.
//
// Which model is in effect and where it stands live in the sheet's undo
// history with the site edits, so every change undoes like any other edit.
// The app keeps every model read this session by key, and useAppliedSite hands
// it the choice after each change. Like pins, a drag only changes the history
// when it is dropped, and the drop picks again the buildings and trees the
// model replaces, as site edits under the model.

import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { errorText, importModel, rereadModel, underModel } from "../../lib/ipc";
import { usePlainKeys } from "../../lib/keys";
import type { ModelChoice, ModelUnit, ModelView, Placement, ProjectView, SiteEdits, UpAxis } from "../../lib/types";
import { useToast } from "../../ui/Toasts";
import { useLiveValue } from "../liveValue";
import { withModelPicks, withoutModelPicks } from "../edits/siteEdits";
import { sizeLabel } from "./modelEdits";

export const MODEL_KEY = "m";
// The extensions the core reads; see READABLE_FORMATS in crates/core/src/model.rs.
const MODEL_EXTENSIONS = ["obj", "glb", "gltf"];
// Arrow keys move a placed model this far, or the larger step with Shift.
const NUDGE_M = 1;
const BIG_NUDGE_M = 10;

type Options = {
  project: ProjectView;
  choice: ModelChoice | null;
  edits: SiteEdits;
  /** The model and the site edits, which change together when the model moves. */
  onModel: (next: ModelChoice | null, edits: SiteEdits, mergeKey?: string) => void;
  onEdits: (next: SiteEdits) => void;
  onSeal: () => void;
  isEnabled: boolean;
  toast: ReturnType<typeof useToast>;
  onUndo: () => void;
};

export function useModelTool({ project, choice, edits, onModel, onEdits, onSeal, isEnabled, toast, onUndo }: Options) {
  const [models, setModels] = useState<Record<string, ModelView>>(() => (project.model ? { [project.model.key]: project.model } : {}));
  const [isPlacing, setPlacing] = useState(false);
  // The placement while the footprint is dragged or turned, until it is dropped.
  const draft = useLiveValue<Placement | null>(null);
  // Each placement is one undo step, though it changes the history twice.
  const placements = useRef(0);
  const latest = useRef(choice);
  latest.current = choice;
  const latestEdits = useRef(edits);
  latestEdits.current = edits;
  const model = choice ? models[choice.key] ?? null : null;

  // A model the app sent again after an assistant's edit is known by its new key.
  useEffect(() => {
    const fresh = project.model;
    if (fresh) setModels((known) => ({ ...known, [fresh.key]: fresh }));
  }, [project.model]);

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
      const imported = { key: view.key, placement: view.placement };
      const under = await underModel(imported, latestEdits.current);
      onModel(imported, withModelPicks(latestEdits.current, under.picked));
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
      onModel({ ...current, key: view.key }, latestEdits.current);
    } catch (error) {
      toast(`Couldn't read the model that way. ${errorText(error)}`, { isError: true });
    }
  };

  const remove = () => {
    onModel(null, withoutModelPicks(latestEdits.current));
    toast("Removed the model", { action: { label: "Undo", run: onUndo } });
  };

  /** Stands the model at `placement` and picks again what it replaces, as one undo step, or part of the step `mergeKey` names. */
  const place = async (placement: Placement, mergeKey?: string) => {
    const current = latest.current;
    if (!current) return;
    placements.current += 1;
    const placementNumber = placements.current;
    mergeKey ??= `model-place-${placementNumber}`;
    const placed = { ...current, placement };
    onModel(placed, latestEdits.current, mergeKey);
    try {
      const under = await underModel(placed, latestEdits.current);
      // A newer placement asked while this one was answered wins; this answer would move the model back.
      if (placementNumber !== placements.current) return;
      onModel(placed, withModelPicks(latestEdits.current, under.picked), mergeKey);
    } catch (error) {
      toast(`Couldn't find the buildings and trees under the model. ${errorText(error)}`, { isError: true });
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

  return { model, models, choice, edits, isPlacing, draft, togglePlacing, importFile, reread, remove, place, onEdits, onSeal };
}

export type ModelTool = ReturnType<typeof useModelTool>;
