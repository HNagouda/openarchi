// Dev only: site edits in a plain browser. Picking, the list and the marks
// work from every tree and building of the fixture (dev-fixtures/site-things.json),
// following the rules of crates/core/src/edits/. The previews are rendered
// ahead of time, so on a plan the mock takes removed trees out of the tree
// layer, moves moved ones and draws planted ones, and paints a removed
// building in the paper colour; the 3D view's trees follow too. The core
// leaves removed buildings out of every view; the mock cannot, off the plan.

import type { Editable, Look, ModelChoice, SiteEdits, TreeAt, UnderModel, View3d } from "../lib/types";
import { rdToPaper } from "./mockPins";

type Tree = { id: string; kind: string | null; at: [number, number] };
type Building = { id: string; year_built: number | null; height_m: number | null; outline: [number, number][][] };
type Things = { origin: [number, number]; trees: Tree[]; buildings: Building[] };
type Fixture = { covered: UnderModel["buildings"]; footprint: [number, number][][] };

const things: Things = await fetch("/dev-fixtures/site-things.json").then((response) => response.json());
const model: Fixture = await fetch("/dev-fixtures/model.json").then((response) => response.json());
// A fixture tree's circle, in paper millimetres, matches a tree within this distance.
const SAME_CIRCLE_MM = 0.05;
// The core's rules for what the model replaces: REPLACED_SHARE and the crown radius.
const REPLACED_SHARE = 0.5;
const CROWN_RADIUS_M = 3;
let current: SiteEdits = {};

/** The mock's answer to a site edit command, or undefined for any other command. */
export function answerEdits(command: string, args: Record<string, unknown>): unknown {
  if (command === "set_site_edits") {
    current = args.edits as SiteEdits;
    return null;
  }
  if (command === "pick_on_site") return pick(args.edits as SiteEdits, args.x as number, args.y as number, args.reachM as number);
  if (command === "site_edit_things") return editedThings(args.edits as SiteEdits);
  if (command === "under_model") return underModel(args.choice as ModelChoice, args.edits as SiteEdits);
  return undefined;
}

export const mockEdits = () => current;

/** Every tree where the edits stand it, planted ones included, removed ones too. */
function standing(edits: SiteEdits): (Tree & { is_planted: boolean })[] {
  const moved = (id: string) => edits.moved?.find((tree) => tree.id === id);
  const surveyed = things.trees.map((tree) => {
    const to = moved(tree.id);
    return { ...tree, at: to ? ([to.x, to.y] as [number, number]) : tree.at, is_planted: false };
  });
  const planted = (edits.planted ?? []).map((tree) => ({ id: tree.id, kind: null, at: [tree.x, tree.y] as [number, number], is_planted: true }));
  return [...surveyed, ...planted];
}

const isRemoved = (edits: SiteEdits, id: string) => (edits.removed ?? []).some((removed) => removed.id === id);
const isUnderModel = (edits: SiteEdits, id: string) => (edits.removed ?? []).some((removed) => removed.id === id && removed.is_under_model === true);

function treeThing(edits: SiteEdits, tree: Tree & { is_planted: boolean }): Editable {
  const surveyed = things.trees.find((entry) => entry.id === tree.id);
  const isMoved = edits.moved?.some((moved) => moved.id === tree.id) ?? false;
  return {
    id: tree.id,
    layer: "tree",
    kind: tree.kind,
    year_built: null,
    height_m: null,
    at: tree.at,
    surveyed_at: isMoved && surveyed ? surveyed.at : null,
    outline: [],
    is_removed: isRemoved(edits, tree.id),
    is_under_model: isUnderModel(edits, tree.id),
    is_planted: tree.is_planted,
  };
}

function buildingThing(edits: SiteEdits, building: Building): Editable {
  return {
    ...building,
    layer: "building",
    kind: null,
    at: null,
    surveyed_at: null,
    is_removed: isRemoved(edits, building.id),
    is_under_model: isUnderModel(edits, building.id),
    is_planted: false,
  };
}

function pick(edits: SiteEdits, x: number, y: number, reachM: number): Editable | null {
  const near = standing(edits)
    .map((tree) => ({ tree, distance: Math.hypot(tree.at[0] - x, tree.at[1] - y) }))
    .filter(({ distance }) => distance <= reachM)
    .sort((a, b) => a.distance - b.distance)[0];
  if (near) return treeThing(edits, near.tree);
  const building = things.buildings.find((entry) => entry.outline.some((ring) => isInside(ring, x, y)));
  return building ? buildingThing(edits, building) : null;
}

function editedThings(edits: SiteEdits): Editable[] {
  const trees = standing(edits);
  const removed = (edits.removed ?? []).flatMap((entry) => {
    const tree = trees.find((candidate) => candidate.id === entry.id);
    if (tree) return [treeThing(edits, tree)];
    const building = things.buildings.find((candidate) => candidate.id === entry.id);
    return building ? [buildingThing(edits, building)] : [];
  });
  const changed = trees
    .filter((tree) => tree.is_planted || edits.moved?.some((moved) => moved.id === tree.id))
    .filter((tree) => !removed.some((thing) => thing.id === tree.id))
    .map((tree) => treeThing(edits, tree));
  return [...removed, ...changed];
}

/** The fixture's buildings under the model wherever it stands, and the trees by the crown rule where it does. */
function underModel(choice: ModelChoice, edits: SiteEdits): UnderModel {
  const angle = (choice.placement.rotation_deg * Math.PI) / 180;
  const rings = model.footprint.map((ring) =>
    ring.map(([x, y]) => [choice.placement.x + x * Math.cos(angle) + y * Math.sin(angle), choice.placement.y - x * Math.sin(angle) + y * Math.cos(angle)] as [number, number]),
  );
  const trees = standing(edits)
    .map((tree) => ({ tree, distance: rings.some((ring) => isInside(ring, ...tree.at)) ? 0 : Math.min(...rings.map((ring) => ringDistance(ring, tree.at))) }))
    .filter(({ distance }) => distance <= CROWN_RADIUS_M)
    .map(({ tree, distance }) => ({ id: tree.id, distance_m: distance, is_planted: tree.is_planted }));
  const picked = [...model.covered.filter((building) => building.share > REPLACED_SHARE).map((building) => building.id), ...trees.map((tree) => tree.id)];
  return { buildings: model.covered, trees, picked };
}

function ringDistance(ring: [number, number][], [x, y]: [number, number]): number {
  let nearest = Infinity;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [ax, ay] = ring[j];
    const [bx, by] = ring[i];
    const length = (bx - ax) ** 2 + (by - ay) ** 2;
    const along = length === 0 ? 0 : Math.max(0, Math.min(1, ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / length));
    nearest = Math.min(nearest, Math.hypot(x - (ax + along * (bx - ax)), y - (ay + along * (by - ay))));
  }
  return nearest;
}

function isInside(ring: [number, number][], x: number, y: number): boolean {
  let isIn = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [xi, yi] = ring[i];
    const [xj, yj] = ring[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) isIn = !isIn;
  }
  return isIn;
}

/** A plan fixture with the mock's edits drawn in, as near as a finished SVG allows. */
export function withMockEdits(svg: string, look: Look): string {
  const toPaper = rdToPaper(svg);
  const edits = current;
  if (!toPaper || look.view.mode !== "plan") return svg;
  const paperOf = ([x, y]: [number, number]) => toPaper(x, y);
  let edited = svg;
  const trees = /<g[^>]*inkscape:label="Trees"[^>]*>([\s\S]*?)<\/g>/.exec(svg);
  if (trees) {
    const radius = /<circle [^>]*r="([\d.]+)"/.exec(trees[1])?.[1] ?? "1";
    const circleAt = (at: [number, number]) => {
      const [cx, cy] = paperOf(at);
      return `<circle cx="${cx.toFixed(2)}" cy="${cy.toFixed(2)}" r="${radius}"/>`;
    };
    let circles = trees[1];
    for (const tree of things.trees) {
      const isGone = isRemoved(edits, tree.id);
      const moved = edits.moved?.find((entry) => entry.id === tree.id);
      if (!isGone && !moved) continue;
      const [cx, cy] = paperOf(tree.at);
      circles = circles.replace(/<circle cx="([\d.]+)" cy="([\d.]+)" r="[\d.]+"\/>/g, (circle, x, y) => {
        const isThis = Math.abs(Number(x) - cx) < SAME_CIRCLE_MM && Math.abs(Number(y) - cy) < SAME_CIRCLE_MM;
        if (!isThis) return circle;
        return isGone || !moved ? "" : circleAt([moved.x, moved.y]);
      });
    }
    const planted = (edits.planted ?? []).filter((tree: TreeAt) => !isRemoved(edits, tree.id));
    circles += planted.map((tree) => circleAt([tree.x, tree.y])).join("");
    edited = edited.replace(trees[1], circles);
  }
  const paperFill = /id="paper"[^>]*fill="([^"]+)"/.exec(svg)?.[1] ?? "#fff";
  const covers = things.buildings
    .filter((building) => isRemoved(edits, building.id))
    .map((building) => building.outline.map((ring) => "M" + ring.map((corner) => paperOf(corner).join(",")).join("L") + "Z").join(""));
  const buildings = edited.indexOf('id="layer-building"');
  if (covers.length === 0 || buildings === -1) return edited;
  // Right over the buildings, so pins, text and the legend stay on top.
  const end = edited.indexOf("</g>", buildings) + "</g>".length;
  const cover = `<path d="${covers.join("")}" fill="${paperFill}" clip-path="url(#frame-clip)"/>`;
  return edited.slice(0, end) + cover + edited.slice(end);
}

/** The 3D view with its trees where the mock's edits have them. */
export function withMockEditedTrees(view: View3d): View3d {
  const [east, north] = things.origin;
  const at = standing(current)
    .filter((tree) => !isRemoved(current, tree.id))
    .map((tree) => [tree.at[0] - east, tree.at[1] - north] as [number, number]);
  return { ...view, trees: { ...view.trees, at } };
}
