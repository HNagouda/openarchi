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

/** The plate at `from` taken out and put back at `to`, the others closing up; out of range does nothing. */
export function withPlateMoved(look: Look, from: number, to: number): Look {
  const plates = [...look.view.plates];
  const isInRange = from >= 0 && from < plates.length && to >= 0 && to < plates.length;
  if (!isInRange || from === to) return look;
  const [moved] = plates.splice(from, 1);
  plates.splice(to, 0, moved);
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
