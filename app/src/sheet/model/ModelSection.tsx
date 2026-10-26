// The imported model in the inspector: what it is, the unit and axis it was
// read with, its turn and base, and the buildings and trees it replaces.
// Moving it happens on the sheet; see ModelOverlay.

import { useEffect, useState } from "react";
import { errorText, underModel } from "../../lib/ipc";
import { shortcutLabel } from "../../lib/platform";
import type { CoveredBuilding, CoveredTree, ModelUnit, UnderModel, UpAxis } from "../../lib/types";
import { Button } from "../../ui/Button";
import { NumberField } from "../../ui/NumberField";
import { Section } from "../../ui/Section";
import { Segmented } from "../../ui/Segmented";
import { Toggle } from "../../ui/Toggle";
import { isRemoved, withTicked } from "../edits/siteEdits";
import { sizeLabel } from "./modelEdits";
import { MODEL_KEY, type ModelTool } from "./useModelTool";
import { PERCENT } from "../../lib/units";
import "./model.css";

const TURN_MERGE_KEY = "model-turn";
const BASE_MERGE_KEY = "model-base";
const FULL_TURN_DEG = 360;
// A base level in metres above NAP: the lowest polder to well above any Dutch dune.
const LOWEST_BASE_M = -10;
const HIGHEST_BASE_M = 50;
const BASE_STEP_M = 0.1;

const UNITS: { value: ModelUnit; label: string }[] = [
  { value: "metre", label: "m" },
  { value: "centimetre", label: "cm" },
  { value: "millimetre", label: "mm" },
];
const AXES: { value: UpAxis; label: string; tip: string }[] = [
  { value: "z", label: "Z up", tip: "As some CAD exports write it" },
  { value: "y", label: "Y up", tip: "As OBJ, glTF and Archicad write it" },
];

export function ModelSection({ tool }: { tool: ModelTool }) {
  const { model, choice } = tool;
  if (!model || !choice) {
    return (
      <Section id="model" title="Model">
        <div className="sheet-fields">
          <p className="model-hint">
            Place your design on the site. In Archicad use File › Save As › Wavefront (.obj); SketchUp, Rhino and Blender export glTF (.glb).
          </p>
          <div className="model-actions">
            <Button icon="cube" variant="secondary" onClick={tool.importFile}>
              Import model…
            </Button>
          </div>
        </div>
      </Section>
    );
  }

  const placement = choice.placement;
  const isOnTerrain = placement.base_m === null || placement.base_m === undefined;
  const turnTo = (rotation_deg: number) => tool.place({ ...placement, rotation_deg }, TURN_MERGE_KEY);
  const baseAt = (base_m: number | null) => tool.place({ ...placement, base_m }, BASE_MERGE_KEY);

  return (
    <Section id="model" title="Model">
      <div className="sheet-fields">
        <p className="model-facts">
          {model.file_name}
          <br />
          {sizeLabel(model)}, {model.triangles.toLocaleString()} triangles
        </p>
        <label className="field-stack">
          <span className="field-label">Read in</span>
          <Segmented label="Unit" value={model.unit} options={UNITS} onChange={(unit) => tool.reread(unit, model.up)} />
        </label>
        <Segmented label="Up axis" value={model.up} options={AXES} onChange={(up) => tool.reread(model.unit, up)} />
        <Button icon="move" tip="Drag the model on the sheet; arrows move it a metre" shortcut={MODEL_KEY} isActive={tool.isPlacing} onClick={tool.togglePlacing}>
          {tool.isPlacing ? "Done placing" : "Move on the sheet"}
        </Button>
        <div className="field-stack">
          <span className="field-label">Turn</span>
          <NumberField label="Turn" value={placement.rotation_deg} min={0} max={FULL_TURN_DEG} step={1} unit="°" onChange={turnTo} onSeal={tool.onSeal} />
        </div>
        <Toggle label="Stands on the terrain" isOn={isOnTerrain} onChange={(isOn) => baseAt(isOn ? null : model.ground_m ?? 0)} />
        {!isOnTerrain && (
          <div className="field-stack">
            <span className="field-label">Base, metres above NAP</span>
            <NumberField
              label="Base, metres above NAP"
              value={placement.base_m ?? 0}
              min={LOWEST_BASE_M}
              max={HIGHEST_BASE_M}
              step={BASE_STEP_M}
              unit="m"
              onChange={baseAt}
              onSeal={tool.onSeal}
            />
          </div>
        )}
        <ReplacedList tool={tool} />
        <p className="model-hint">
          {shortcutLabel(MODEL_KEY).toUpperCase()} moves the model on the sheet. Moving it picks again the buildings it covers more than half of, and the trees within a crown (3 m) of it.
        </p>
        <div className="model-actions">
          <Button icon="cube" onClick={tool.importFile}>
            Import again…
          </Button>
          <Button icon="trash" onClick={tool.remove}>
            Remove
          </Button>
        </div>
      </div>
    </Section>
  );
}

const NOTHING_UNDER: UnderModel = { buildings: [], trees: [], picked: [] };

/** The buildings and trees under the model, ticked when it replaces them. */
function ReplacedList({ tool }: { tool: ModelTool }) {
  const { choice, edits } = tool;
  const [under, setUnder] = useState<UnderModel>(NOTHING_UNDER);
  const [error, setError] = useState<string | null>(null);
  // Only a new place, or a tree moved or planted, changes what stands under
  // the model; ticking a box or removing something does not.
  const placementKey = JSON.stringify(choice && [choice.key, choice.placement, edits.moved, edits.planted]);

  useEffect(() => {
    if (!choice) return;
    let isCurrent = true;
    underModel(choice, edits)
      .then((found) => {
        if (!isCurrent) return;
        setUnder(found);
        setError(null);
      })
      .catch((reason) => {
        if (isCurrent) setError(errorText(reason));
      });
    return () => {
      isCurrent = false;
    };
  }, [placementKey]);

  if (!choice) return null;
  if (error) return <p className="model-hint">{error}</p>;
  if (under.buildings.length + under.trees.length === 0) return <p className="model-hint">No existing buildings or trees under the model.</p>;
  const box = (id: string, label: string) => (
    <li key={id}>
      <label>
        <input type="checkbox" checked={isRemoved(edits, id)} onChange={(event) => tool.onEdits(withTicked(edits, id, event.target.checked))} />
        <span>{label}</span>
      </label>
    </li>
  );
  return (
    <div className="field-stack">
      <span className="field-label">Replaces</span>
      <ul className="model-replaced">
        {under.buildings.map((building) => box(building.id, buildingLabel(building)))}
        {under.trees.map((tree) => box(tree.id, treeLabel(tree)))}
      </ul>
    </div>
  );
}

/** "Tree, 2.1 m from the model" or "Planted tree, inside the model". */
function treeLabel(tree: CoveredTree): string {
  const name = tree.is_planted ? "Planted tree" : "Tree";
  return tree.distance_m === 0 ? `${name}, inside the model` : `${name}, ${tree.distance_m.toFixed(1)} m from the model`;
}

/** "Built 1772, 15.8 m tall, 92 % under the model". */
function buildingLabel(building: CoveredBuilding): string {
  const parts = [
    building.year_built ? `Built ${building.year_built}` : "Year unknown",
    building.height_m ? `${building.height_m.toFixed(1)} m tall` : null,
    `${Math.round(building.share * PERCENT)} % under the model`,
  ];
  return parts.filter(Boolean).join(", ");
}
