// The theme's colours as the swatches every colour picker on the sheet offers.

import type { Palette } from "../lib/types";
import type { Swatch } from "../ui/ColorPicker";

// The palette roles a colour picker offers on this sheet, in one place.
const SWATCH_ROLES: (keyof Palette)[] = ["ink", "paper", "figure", "accent", "water", "green", "muted", "ground"];

/** The theme's palette as picker swatches. Picking one for text keeps the role, so it follows theme changes. */
export function paletteSwatches(palette: Palette): Swatch[] {
  return SWATCH_ROLES.map((role) => ({ value: `@${role}`, colour: palette[role], label: role[0].toUpperCase() + role.slice(1) }));
}
