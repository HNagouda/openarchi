// Edits to a palette draft, each returning a new draft. Pins point at swatches
// by index, so moving or removing a swatch moves the pins with it.

import { ROLES, type PaletteDraft, type RoleChoice } from "../../lib/palette";

/** The most and fewest swatches a palette holds, as crates/core/src/palette/parse.rs allows. */
export const MAX_SWATCHES = 10;
export const MIN_SWATCHES = 2;

/** Where a swatch at `index` ends up when the one at `from` moves to `to`. */
function movedIndex(index: number, from: number, to: number): number {
  if (index === from) return to;
  if (from < index && index <= to) return index - 1;
  if (to <= index && index < from) return index + 1;
  return index;
}

function remapRoles(roles: RoleChoice, map: (index: number) => number | null): RoleChoice {
  const next: RoleChoice = {};
  for (const role of ROLES) {
    const pinned = roles[role];
    if (pinned === undefined || pinned === null) continue;
    const moved = map(pinned);
    if (moved !== null) next[role] = moved;
  }
  return next;
}

export function moveSwatch(draft: PaletteDraft, from: number, to: number): PaletteDraft {
  if (from === to || to < 0 || to >= draft.swatches.length) return draft;
  const swatches = [...draft.swatches];
  const [moved] = swatches.splice(from, 1);
  swatches.splice(to, 0, moved);
  return { ...draft, swatches, roles: remapRoles(draft.roles, (index) => movedIndex(index, from, to)) };
}

/** A role pinned to the removed swatch goes back to the rule. */
export function removeSwatch(draft: PaletteDraft, removed: number): PaletteDraft {
  if (draft.swatches.length <= MIN_SWATCHES) return draft;
  const swatches = draft.swatches.filter((_, index) => index !== removed);
  const roles = remapRoles(draft.roles, (index) => {
    if (index === removed) return null;
    return index > removed ? index - 1 : index;
  });
  return { ...draft, swatches, roles };
}

export function addSwatch(draft: PaletteDraft, hex: string): PaletteDraft {
  if (draft.swatches.length >= MAX_SWATCHES) return draft;
  return { ...draft, swatches: [...draft.swatches, { hex }] };
}

export function recolourSwatch(draft: PaletteDraft, index: number, hex: string): PaletteDraft {
  const swatches = draft.swatches.map((swatch, at) => (at === index ? { ...swatch, hex } : swatch));
  return { ...draft, swatches };
}

/** `null` hands the role back to the rule. */
export function pinRole(draft: PaletteDraft, role: (typeof ROLES)[number], index: number | null): PaletteDraft {
  const roles = { ...draft.roles };
  if (index === null) delete roles[role];
  else roles[role] = index;
  return { ...draft, roles };
}
