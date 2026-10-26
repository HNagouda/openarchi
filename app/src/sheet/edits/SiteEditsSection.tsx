// The site edits in the inspector: the switch for the edit tool and its
// planting mode, the tree or building picked on the sheet, and every edit,
// each with the button that undoes it. Picking and dragging happen on the
// sheet; see useEditTool and EditOverlay.

import { shortcutLabel } from "../../lib/platform";
import type { Editable } from "../../lib/types";
import { Button } from "../../ui/Button";
import { Section } from "../../ui/Section";
import { Segmented } from "../../ui/Segmented";
import { CROWN_RADIUS_M } from "./editGround";
import { EDIT_KEY, type EditMode, type EditTool } from "./useEditTool";
import "./edits.css";

const MODES: { value: EditMode; label: string; tip: string }[] = [
  { value: "pick", label: "Select", tip: "Click a tree or building; drag a tree to move it" },
  { value: "plant", label: "Plant trees", tip: "Click inside the site to plant a tree" },
];

export function SiteEditsSection({ tool }: { tool: EditTool }) {
  const { selected, things } = tool;
  const tip = tool.isPlan ? "Pick trees and buildings on the sheet" : "Trees and buildings are edited on a plan view";

  return (
    <Section id="site-edits" title="Site edits">
      <div className="sheet-fields">
        <Button icon="tree" tip={tip} shortcut={EDIT_KEY} isActive={tool.isEditing} disabled={!tool.isPlan} onClick={tool.toggle}>
          {tool.isEditing ? "Done editing" : "Edit trees and buildings"}
        </Button>
        {tool.isEditing && <Segmented label="Edit tool" value={tool.mode} options={MODES} onChange={tool.setMode} />}
        {tool.isEditing && <p className="model-hint">{hint(tool.mode)}</p>}
        {selected && <Picked thing={selected} tool={tool} />}
        {things.length > 0 && (
          <div className="field-stack">
            <span className="field-label">Edits</span>
            <ul className="edit-list">
              {things.map((thing) => (
                <li key={thing.id} className={selected?.id === thing.id ? "is-selected" : ""}>
                  <button type="button" className="edit-name" title={thingLabel(thing)} onClick={() => tool.setSelected(thing)}>
                    {thingLabel(thing)}
                  </button>
                  <Button variant="plain" onClick={() => (thing.is_planted ? tool.remove(thing) : tool.restore(thing))}>
                    {thing.is_planted ? "Remove" : "Restore"}
                  </Button>
                </li>
              ))}
            </ul>
          </div>
        )}
        <p className="model-hint">
          Every sheet, the 3D view and the exports show the edits. Ratios and the analysis count the site as fetched.
        </p>
      </div>
    </Section>
  );
}

function Picked({ thing, tool }: { thing: Editable; tool: EditTool }) {
  return (
    <div className="field-stack">
      <span className="field-label">Selected</span>
      <p className="model-facts">{thingLabel(thing)}</p>
      <div className="model-actions">
        <Button icon={thing.is_removed ? "restore" : "trash"} shortcut="delete" onClick={() => tool.toggleRemoved(thing)}>
          {thing.is_removed ? "Restore" : "Remove"}
        </Button>
        {thing.surveyed_at && !thing.is_removed && (
          <Button icon="reset" onClick={() => tool.restore(thing)}>
            Put back
          </Button>
        )}
      </div>
    </div>
  );
}

function hint(mode: EditMode): string {
  if (mode === "plant") return `Click inside the site to plant a tree, drawn with a ${CROWN_RADIUS_M} m crown like every tree. Esc stops.`;
  return `Click a tree or building, then ${shortcutLabel("delete")} takes it away or brings it back. Drag a tree, or use the arrows, to move it. Removed ones show dashed.`;
}

/** "Building, built 1772, removed under the model", "Tree, moved 4.2 m", "Planted tree". */
export function thingLabel(thing: Editable): string {
  if (thing.layer === "building") {
    const built = thing.year_built ? `Building, built ${thing.year_built}` : "Building";
    return `${built}${stateLabel(thing)}`;
  }
  const name = thing.is_planted ? "Planted tree" : "Tree";
  if (thing.surveyed_at && thing.at && !thing.is_removed) {
    const metres = Math.hypot(thing.at[0] - thing.surveyed_at[0], thing.at[1] - thing.surveyed_at[1]);
    return `${name}, moved ${metres.toFixed(1)} m`;
  }
  return `${name}${stateLabel(thing)}`;
}

function stateLabel(thing: Editable): string {
  if (thing.is_under_model) return ", under the model";
  if (thing.is_removed) return ", removed";
  return "";
}
