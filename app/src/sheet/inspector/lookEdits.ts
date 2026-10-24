// Small, pure edits to a Look, so inspector controls stay one line each.

import type { Layer, LayerOverride, Look, Palette, Sheet, SunMoment, Theme, View } from "../../lib/types";

export function withSheet(look: Look, patch: Partial<Sheet>): Look {
  return { ...look, sheet: { ...look.sheet, ...patch } };
}

export function withView(look: Look, patch: Partial<View>): Look {
  return { ...look, view: { ...look.view, ...patch } };
}

export function withSun(look: Look, patch: Partial<SunMoment>): Look {
  return { ...look, sun: { ...look.sun, ...patch } };
}

/** The plate at `index` swapped with its neighbour `by` places away; out of range does nothing. */
export function withPlateMoved(look: Look, index: number, by: number): Look {
  const target = index + by;
  const plates = [...look.view.plates];
  if (target < 0 || target >= plates.length) return look;
  [plates[index], plates[target]] = [plates[target], plates[index]];
  return withView(look, { plates });
}

/** Sets or clears override fields; `null` clears one. An empty override is removed entirely. */
export function withOverride(look: Look, layer: Layer, patch: LayerOverride): Look {
  const merged: LayerOverride = { ...look.overrides[layer], ...patch };
  const kept = Object.fromEntries(
    Object.entries(merged).filter(([, value]) => value !== null && value !== undefined),
  ) as LayerOverride;
  const overrides = { ...look.overrides };
  if (Object.keys(kept).length === 0) delete overrides[layer];
  else overrides[layer] = kept;
  return { ...look, overrides };
}

/** "@figure" -> the theme's figure colour, for showing a swatch. The renderer does the real resolving. */
export function swatchColour(value: string | null | undefined, palette: Palette): string | null {
  if (!value) return null;
  if (value.startsWith("@")) return palette[value.slice(1) as keyof Palette] ?? null;
  return value;
}

/** The colours a theme card shows, from paper to accent. */
export function paletteStrip(theme: Theme): string[] {
  const { paper, figure, accent, water, green } = theme.palette;
  return [paper, figure, accent, water, green];
}
