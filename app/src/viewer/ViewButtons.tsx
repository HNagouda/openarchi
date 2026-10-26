// The named views over the 3D picture, and a line saying what the pointer
// does while choosing where to stand or walking.

import { Button } from "../ui/Button";
import { MAX_WALK_FIELD_OF_VIEW_DEG, MIN_WALK_FIELD_OF_VIEW_DEG, type Mode, type Navigation } from "./navigation";
import { PERCENT } from "../lib/units";

const FIELD_OF_VIEW_STEP_DEG = 1;

const HINTS: Record<Mode, string | null> = {
  orbit: null,
  pick: "Click where to stand · Esc to cancel",
  walk: "Drag to look round · WASD or arrows to walk, Q and E to turn · Esc to step back",
};

type Props = {
  navigation: Navigation | null;
  mode: Mode;
  /** The walk's field of view in degrees; a slider sets it while walking, as a pinch does. */
  walkFieldOfView: number;
  onWalkFieldOfView: (degrees: number) => void;
};

export function ViewButtons({ navigation, mode, walkFieldOfView, onWalkFieldOfView }: Props) {
  const fill = ((walkFieldOfView - MIN_WALK_FIELD_OF_VIEW_DEG) / (MAX_WALK_FIELD_OF_VIEW_DEG - MIN_WALK_FIELD_OF_VIEW_DEG)) * PERCENT;
  const isDisabled = navigation === null;
  const hint = HINTS[mode];
  return (
    <>
      <div className="viewer-views" role="toolbar" aria-label="Views">
        <Button icon="fit" tip="The whole site, from where the view opened" disabled={isDisabled} onClick={() => navigation?.fit()}>
          Whole site
        </Button>
        <Button tip="Straight down, north up" disabled={isDisabled} onClick={() => navigation?.top()}>
          Top
        </Button>
        <Button tip="The sheet's own turn and tilt" disabled={isDisabled} onClick={() => navigation?.sheetAngle()}>
          Sheet angle
        </Button>
        <Button icon="eye" tip="Stand on the site at eye height, then drag to look round" isActive={mode !== "orbit"} disabled={isDisabled} onClick={() => (mode === "orbit" ? navigation?.pickStandpoint() : navigation?.leave())}>
          Walk
        </Button>
      </div>
      {hint && <div className="viewer-hint">{hint}</div>}
      {mode === "walk" && (
        <label className="viewer-lens" data-tip="Narrow like a long lens, or wide to take in a street; pinching does the same">
          <span>Field of view</span>
          <input
            className="slider"
            type="range"
            aria-label="Field of view"
            min={MIN_WALK_FIELD_OF_VIEW_DEG}
            max={MAX_WALK_FIELD_OF_VIEW_DEG}
            step={FIELD_OF_VIEW_STEP_DEG}
            value={walkFieldOfView}
            onChange={(event) => onWalkFieldOfView(Number(event.target.value))}
            style={{ ["--fill" as string]: `${fill}%` }}
          />
          <span className="tabular">{Math.round(walkFieldOfView)}°</span>
        </label>
      )}
    </>
  );
}
