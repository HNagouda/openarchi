// The site edits as the sheet's undo history changes them: the same rules as
// `SiteEdits` in crates/core/src/edits.rs, so the app and an assistant edit
// alike. Every function returns new edits and leaves the old ones alone.

import type { SiteEdits, TreeAt } from "../../lib/types";

// Planted trees are named "planted-1", "planted-2" and so on, as PLANTED_PREFIX in the core.
export const PLANTED_PREFIX = "planted-";

export const NO_EDITS: SiteEdits = {};

const removedOf = (edits: SiteEdits) => edits.removed ?? [];
const movedOf = (edits: SiteEdits) => edits.moved ?? [];
const plantedOf = (edits: SiteEdits) => edits.planted ?? [];

export function hasEdits(edits: SiteEdits): boolean {
  return removedOf(edits).length + movedOf(edits).length + plantedOf(edits).length > 0;
}

export function isRemoved(edits: SiteEdits, id: string): boolean {
  return removedOf(edits).some((removed) => removed.id === id);
}

/** `id` taken away by hand. A planted tree is simply gone. */
export function withRemoved(edits: SiteEdits, id: string): SiteEdits {
  if (id.startsWith(PLANTED_PREFIX)) return { ...edits, planted: plantedOf(edits).filter((tree) => tree.id !== id) };
  if (isRemoved(edits, id)) return edits;
  return { ...edits, removed: [...removedOf(edits), { id }] };
}

/** `id` back as the survey has it: no longer removed, and a moved tree back in its place. */
export function withRestored(edits: SiteEdits, id: string): SiteEdits {
  return {
    ...edits,
    removed: removedOf(edits).filter((removed) => removed.id !== id),
    moved: movedOf(edits).filter((tree) => tree.id !== id),
  };
}

/** Tree `id` standing at an RD New point. */
export function withMoved(edits: SiteEdits, id: string, x: number, y: number): SiteEdits {
  const moved: TreeAt = { id, x, y };
  const isPlanted = id.startsWith(PLANTED_PREFIX);
  const trees = isPlanted ? plantedOf(edits) : movedOf(edits);
  const others = trees.filter((tree) => tree.id !== id);
  const next = trees.some((tree) => tree.id === id) ? trees.map((tree) => (tree.id === id ? moved : tree)) : [...others, moved];
  return isPlanted ? { ...edits, planted: next } : { ...edits, moved: next };
}

/** A new tree at an RD New point, and its id. */
export function withPlanted(edits: SiteEdits, x: number, y: number): { edits: SiteEdits; id: string } {
  const planted = plantedOf(edits);
  let number = planted.length + 1;
  while (planted.some((tree) => tree.id === `${PLANTED_PREFIX}${number}`)) number += 1;
  const id = `${PLANTED_PREFIX}${number}`;
  return { edits: { ...edits, planted: [...planted, { id, x, y }] }, id };
}

/** What the model stands on picked afresh: `ids` taken away under it, the rest of its picks back. Hand removals stay. */
export function withModelPicks(edits: SiteEdits, ids: string[]): SiteEdits {
  const kept = withoutModelPicks(edits);
  const added = ids.filter((id) => !isRemoved(kept, id)).map((id) => ({ id, is_under_model: true }));
  return { ...kept, removed: [...removedOf(kept), ...added] };
}

/** Everything the model took away back, for when the model goes. */
export function withoutModelPicks(edits: SiteEdits): SiteEdits {
  return { ...edits, removed: removedOf(edits).filter((removed) => !removed.is_under_model) };
}

/** `id`, something the model stands on, taken away under it or kept, as its box was ticked. */
export function withTicked(edits: SiteEdits, id: string, isTicked: boolean): SiteEdits {
  if (!isTicked) return { ...edits, removed: removedOf(edits).filter((removed) => removed.id !== id) };
  if (isRemoved(edits, id)) return edits;
  return { ...edits, removed: [...removedOf(edits), { id, is_under_model: true }] };
}
