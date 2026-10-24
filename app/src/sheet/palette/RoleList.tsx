// Which colour each role took and why. "Auto" leaves a role to the rule in
// crates/core/src/palette/roles.rs; a swatch pins it to exactly that colour.

import type { PaletteSwatch, Role, RoleColour } from "../../lib/palette";

/** What each role paints on a sheet, in the words the editor uses. */
const ROLE_LABELS: Record<Role, { name: string; paints: string }> = {
  paper: { name: "Paper", paints: "the sheet" },
  ink: { name: "Ink", paints: "text and fine lines" },
  figure: { name: "Figure", paints: "buildings" },
  accent: { name: "Accent", paints: "site outline and highlights" },
  water: { name: "Water", paints: "canals, rivers and ponds" },
  green: { name: "Green", paints: "parks and planting" },
};

type Props = {
  roles: RoleColour[];
  swatches: PaletteSwatch[];
  onPin: (role: Role, index: number | null) => void;
};

export function RoleList({ roles, swatches, onPin }: Props) {
  return (
    <ul className="role-list">
      {roles.map((colour) => {
        const label = ROLE_LABELS[colour.role];
        return (
          <li key={colour.role} className="role-row">
            <span className="role-chip" style={{ background: colour.hex }} aria-hidden="true" />
            <span className="role-text">
              <span className="role-name">
                {label.name} <span className="role-paints">{label.paints}</span>
              </span>
              <span className="role-reason">
                {colour.hex}, {colour.reason}
              </span>
            </span>
            <span className="role-pins" role="radiogroup" aria-label={`${label.name} colour`}>
              <button
                type="button"
                role="radio"
                aria-checked={!colour.is_pinned}
                className={colour.is_pinned ? "role-auto" : "role-auto is-on"}
                data-tip="Let the rule choose"
                onClick={() => onPin(colour.role, null)}
              >
                Auto
              </button>
              {swatches.map((swatch, index) => {
                const isOn = colour.is_pinned && colour.swatch === index;
                return (
                  <button
                    key={index}
                    type="button"
                    role="radio"
                    aria-checked={isOn}
                    aria-label={`${label.name} ${swatch.hex}`}
                    className={isOn ? "role-pin is-on" : "role-pin"}
                    style={{ background: swatch.hex }}
                    onClick={() => onPin(colour.role, index)}
                  />
                );
              })}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
