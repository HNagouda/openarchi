// The panel beside the map: what was drawn, how big it is, and the fetch.

import { formatArea } from "../lib/format";
import type { Area, AreaSize, Progress } from "../lib/types";
import { Button, Keys } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { NumberField } from "../ui/NumberField";
import { Problem } from "../ui/Problem";

// The largest circle that stays under the core's 4 km² limit is about 1128 m.
const MIN_RADIUS_M = 25;
const MAX_RADIUS_M = 1125;
const RADIUS_STEP_M = 5;

export type FetchState =
  | { state: "idle" }
  | { state: "running"; progress: Progress | null }
  | { state: "failed"; error: string };

type Props = {
  name: string;
  onName: (name: string) => void;
  area: Area | null;
  size: AreaSize | null;
  onRadius: (radius: number) => void;
  fetch: FetchState;
  onFetch: () => void;
  onCancel: () => void;
  isEditing: boolean;
};

const SHAPE_NAMES = { circle: "Circle", rectangle: "Rectangle", polygon: "Polygon" };

export function SitePanel({ name, onName, area, size, onRadius, fetch, onFetch, onCancel, isEditing }: Props) {
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

      {area === null ? (
        <div className="site-hint">
          <p>Search for a place, then draw the site.</p>
          <ul className="hint-keys">
            <li>
              <Keys shortcut="c" /> Circle from its centre
            </li>
            <li>
              <Keys shortcut="r" /> Rectangle corner to corner
            </li>
            <li>
              <Keys shortcut="p" /> Polygon, click each corner
            </li>
          </ul>
        </div>
      ) : (
        <div className="site-facts">
          <div className="fact-row">
            <span className="field-label">Shape</span>
            <span>{SHAPE_NAMES[area.kind]}</span>
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
              <div className={isTooLarge ? "area-meter-fill is-over" : "area-meter-fill"} style={{ width: `${share * 100}%` }} />
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
          <div className="fetch-progress">
            <div className="fetch-progress-row">
              <span>{fetch.progress ? fetch.progress.step : "Starting"}</span>
              {fetch.progress && (
                <span className="tabular fetch-count">
                  {fetch.progress.done} of {fetch.progress.total}
                </span>
              )}
            </div>
            <div className="progress-track">
              <div
                className={fetch.progress ? "progress-fill" : "progress-fill is-waiting"}
                style={{ width: fetch.progress ? `${(fetch.progress.done / Math.max(1, fetch.progress.total)) * 100}%` : "30%" }}
              />
            </div>
          </div>
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
