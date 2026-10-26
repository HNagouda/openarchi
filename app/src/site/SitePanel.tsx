// The panel beside the map: finding the place, the tools to draw it, what was
// drawn, how big it is, and the fetch.

import type { ReactNode } from "react";
import { formatArea } from "../lib/format";
import type { Area, AreaSize, Place, Progress } from "../lib/types";
import { PERCENT } from "../lib/units";
import { ProgressBar } from "../ui/ProgressBar";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { Icon } from "../ui/Icon";
import { NumberField } from "../ui/NumberField";
import { Problem } from "../ui/Problem";
import { Segmented } from "../ui/Segmented";
import { SearchBox } from "./SearchBox";
import type { Shape } from "./shapes";
import type { Tool } from "./useDrawing";

// The largest circle that stays under the core's 4 km² limit is about 1128 m.
const MIN_RADIUS_M = 25;
const MAX_RADIUS_M = 1125;
const RADIUS_STEP_M = 5;

export type FetchState =
  | { state: "idle" }
  | { state: "running"; progress: Progress | null }
  | { state: "failed"; error: string };

type Props = {
  onPick: (place: Place) => void;
  /** The drawing tool in hand; picking it again puts it down. */
  tool: Tool;
  onTool: (tool: Tool) => void;
  name: string;
  onName: (name: string) => void;
  area: Area | null;
  size: AreaSize | null;
  onRadius: (radius: number) => void;
  onRadiusSeal: () => void;
  onShape: (shape: Shape) => void;
  onDelete: () => void;
  fetch: FetchState;
  onFetch: () => void;
  onCancel: () => void;
  isEditing: boolean;
};

// The drawing tools' single keys, here so the site step, the shortcuts list and the tour follow them.
export const CIRCLE_KEY = "c";
export const RECTANGLE_KEY = "r";
export const POLYGON_KEY = "p";
/** Back to moving the map, as Hand and Move are in drawing programs. */
export const PAN_KEYS = ["h", "v"];

const TOOLS: { tool: Tool; icon: "circle" | "rectangle" | "polygon"; label: string; tip: string; key: string }[] = [
  { tool: "circle", icon: "circle", label: "Circle", tip: "Drag out from the centre", key: CIRCLE_KEY },
  { tool: "rectangle", icon: "rectangle", label: "Rectangle", tip: "Drag corner to corner", key: RECTANGLE_KEY },
  { tool: "polygon", icon: "polygon", label: "Polygon", tip: "Click each corner", key: POLYGON_KEY },
];

const SHAPE_OPTIONS: { value: Shape; label: ReactNode; tip: string }[] = [
  { value: "circle", label: <Icon name="circle" />, tip: "Circle" },
  { value: "rectangle", label: <Icon name="rectangle" />, tip: "Rectangle" },
  { value: "polygon", label: <Icon name="polygon" />, tip: "Polygon" },
];

export function SitePanel({
  onPick,
  tool,
  onTool,
  name,
  onName,
  area,
  size,
  onRadius,
  onRadiusSeal,
  onShape,
  onDelete,
  fetch,
  onFetch,
  onCancel,
  isEditing,
}: Props) {
  const isTooLarge = size !== null && size.m2 > size.max_m2;
  const isOutside = size !== null && !size.in_netherlands;
  const canFetch = area !== null && !isTooLarge && fetch.state !== "running";
  const share = size ? Math.min(1, size.m2 / size.max_m2) : 0;

  return (
    <aside className="site-panel">
      <div className="site-panel-head">
        <span className="field-label">{isEditing ? "Editing site" : "New site"}</span>
        <EditableText className="site-name" value={name} placeholder="Untitled site" onCommit={onName} />
      </div>

      <div className="site-find">
        <SearchBox onPick={onPick} />
        {/* Drawing replaces the site; Shape below changes the one already drawn. */}
        <span className="field-label">{area === null ? "Draw" : "Draw again"}</span>
        <div className="draw-tools" role="toolbar" aria-label="Drawing tools">
          {TOOLS.map((entry) => (
            <Button
              key={entry.tool}
              icon={entry.icon}
              tip={entry.tip}
              shortcut={entry.key}
              isActive={tool === entry.tool}
              onClick={() => onTool(entry.tool)}
            >
              {entry.label}
            </Button>
          ))}
        </div>
      </div>

      {area === null ? (
        <p className="site-hint">Search for a place, then pick a shape and draw the site on the map.</p>
      ) : (
        <div className="site-facts">
          <div className="shape-row">
            <span className="field-label">Shape</span>
            <Segmented label="Shape" value={area.kind} options={SHAPE_OPTIONS} onChange={onShape} />
            <Button icon="trash" tip="Delete the shape" shortcut="delete" tipSide="left" onClick={onDelete} />
          </div>
          {area.kind === "circle" && (
            <div className="fact-block">
              <span className="field-label">Radius</span>
              <NumberField
                label="Radius"
                value={Math.round(area.radius_m)}
                min={MIN_RADIUS_M}
                max={MAX_RADIUS_M}
                step={RADIUS_STEP_M}
                unit="m"
                onChange={onRadius}
                onSeal={onRadiusSeal}
              />
            </div>
          )}
          <div className="fact-block">
            <div className="fact-row">
              <span className="field-label">Area</span>
              <span className={isTooLarge ? "area-value is-over tabular" : "area-value tabular"}>
                {size ? formatArea(size.m2) : "–"}
              </span>
            </div>
            <div className="area-meter" aria-hidden="true">
              <div className={isTooLarge ? "area-meter-fill is-over" : "area-meter-fill"} style={{ width: `${share * PERCENT}%` }} />
            </div>
            <span className="area-caption">
              {isTooLarge ? "Larger than the 4 km² openarchi can fetch at once. Draw a smaller site." : "Up to 4 km² per site"}
            </span>
          </div>
          {isOutside && (
            <p className="site-note">openarchi covers the Netherlands for now, and this site is outside it. Data may be missing.</p>
          )}
        </div>
      )}

      <div className="site-panel-foot">
        {fetch.state === "running" && (
          <ProgressBar
            label={fetch.progress ? fetch.progress.step : "Starting"}
            done={fetch.progress?.done ?? null}
            total={fetch.progress?.total ?? null}
          />
        )}
        {fetch.state === "failed" && <Problem title="The site couldn't be fetched." detail={fetch.error} />}
        {fetch.state === "running" ? (
          <Button variant="secondary" className="wide" onClick={onCancel}>
            Cancel
          </Button>
        ) : (
          <Button variant="primary" className="wide" disabled={!canFetch} onClick={onFetch}>
            {fetch.state === "failed" ? "Try again" : isEditing ? "Fetch again" : "Fetch site"}
          </Button>
        )}
      </div>
    </aside>
  );
}
