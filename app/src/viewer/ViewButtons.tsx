// The named views over the 3D picture, and a line saying what the pointer
// does while choosing where to stand or walking.

import { Button } from "../ui/Button";
import type { Mode, Navigation } from "./navigation";

const HINTS: Record<Mode, string | null> = {
  orbit: null,
  pick: "Click where to stand · Esc to cancel",
  walk: "Drag to look round · WASD or arrows to walk, Q and E to turn · Esc to step back",
};

type Props = {
  navigation: Navigation | null;
  mode: Mode;
};

export function ViewButtons({ navigation, mode }: Props) {
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
    </>
  );
}
