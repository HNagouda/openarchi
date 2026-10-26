// The site as three.js meshes, from the triangles the core sends: the ground,
// the buildings, the neighbourhood, the model, and trees as a trunk and a
// round crown. The palette finish keeps the sheet's colours; the realistic
// finish paints the same shapes in real ones (materials.ts). Everything casts
// or catches shadows. The frame is the core's: x east, y north, z up, in metres.

import * as THREE from "three";
import type { Surface, View3d } from "../lib/types";
import {
  buildingAttributes,
  buildingMaterial,
  type Finish,
  REAL_GROUND,
  REAL_MODEL,
  REAL_OPEN_GROUND,
  REAL_TREE,
  REAL_TRUNK,
  REAL_ZENITH,
  seedOf,
  waterMaterial,
} from "./materials";

// Corners per triangle, and numbers per corner.
const CORNERS = 3;
const TREE_CROWN_DETAIL = 2;
const TRUNK_RADIUS_M = 0.25;
const TRUNK_SIDES = 6;
const TRUNK_COLOUR = "#6B5B4B";
// Real trees differ: crowns this much larger or smaller, and a little lighter or darker.
const CROWN_SIZE_SPREAD = 0.3;
const CROWN_LIGHTNESS_SPREAD = 0.12;
const CROWN_HUE_SPREAD = 0.03;

/** Every mesh of the site in one group, ready to add to a scene. */
export function siteGroup(view: View3d, finish: Finish): THREE.Group {
  const group = new THREE.Group();
  for (const surface of [...view.ground, ...view.surroundings]) group.add(groundMesh(surface, finish));
  const buildingOptions = { finish, storeyM: view.storey_height_m, hasFacades: true };
  for (const surface of [...view.buildings, view.neighbours]) {
    if (surface.triangle_count === 0) continue;
    const mesh = surfaceMesh(surface, buildingMaterial(surface.colour, buildingOptions));
    buildingAttributes(mesh.geometry, surface.pieces);
    mesh.castShadow = true;
    group.add(mesh);
  }
  if (view.model) {
    const colour = finish === "realistic" ? REAL_MODEL : view.model.colour;
    const material = buildingMaterial(colour, { ...buildingOptions, hasFacades: false });
    // An exported model may face some triangles inward; both sides are drawn so none goes missing.
    material.side = THREE.DoubleSide;
    const mesh = surfaceMesh(view.model, material);
    mesh.castShadow = true;
    group.add(mesh);
  }
  if (view.trees.at.length > 0) group.add(trees(view.trees, finish));
  return group;
}

/** A ground surface: it only catches shadows, being flat. */
function groundMesh(surface: Surface, finish: Finish): THREE.Mesh {
  const isReal = finish === "realistic";
  const colour = !isReal ? surface.colour : surface.layer ? (REAL_GROUND[surface.layer] ?? REAL_OPEN_GROUND) : REAL_OPEN_GROUND;
  const material = surface.layer === "water" ? waterMaterial(colour, finish, REAL_ZENITH) : new THREE.MeshLambertMaterial({ color: colour });
  return surfaceMesh(surface, material);
}

function surfaceMesh(surface: Surface, material: THREE.Material): THREE.Mesh {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(corners(surface.corners), CORNERS));
  // Each triangle has corners of its own, so these normals are the triangles' own: flat faces.
  geometry.computeVertexNormals();
  const mesh = new THREE.Mesh(geometry, material);
  mesh.receiveShadow = true;
  return mesh;
}

/** The core's base64 corners as the float array a buffer attribute takes. */
function corners(packed: string): Float32Array {
  const text = atob(packed);
  const bytes = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index += 1) bytes[index] = text.charCodeAt(index);
  return new Float32Array(bytes.buffer);
}

/** One drawing for every crown and one for every trunk, each tree a copy moved into place. */
function trees(trees: View3d["trees"], finish: Finish): THREE.Group {
  const isReal = finish === "realistic";
  const crownGeometry = new THREE.IcosahedronGeometry(trees.crown_m, TREE_CROWN_DETAIL);
  const trunkHeight = trees.height_m - trees.crown_m;
  const trunkGeometry = new THREE.CylinderGeometry(TRUNK_RADIUS_M, TRUNK_RADIUS_M, trunkHeight, TRUNK_SIDES);
  // three.js stands a cylinder along y; the site has z up.
  trunkGeometry.rotateX(Math.PI / 2);
  trunkGeometry.translate(0, 0, trunkHeight / 2);
  const crownColour = isReal ? REAL_TREE : trees.colour;
  const crowns = new THREE.InstancedMesh(crownGeometry, new THREE.MeshLambertMaterial({ color: isReal ? "#FFFFFF" : crownColour }), trees.at.length);
  const trunks = new THREE.InstancedMesh(trunkGeometry, new THREE.MeshLambertMaterial({ color: isReal ? REAL_TRUNK : TRUNK_COLOUR }), trees.at.length);
  const place = new THREE.Matrix4();
  const shade = new THREE.Color();
  trees.at.forEach(([x, y], index) => {
    // The palette draws every tree alike, as the sheets do; real trees vary.
    const seed = seedOf(index);
    const size = isReal ? 1 + (seed - 0.5) * CROWN_SIZE_SPREAD * 2 : 1;
    const crownZ = trunkHeight * size;
    crowns.setMatrixAt(index, place.makeScale(size, size, size).setPosition(x, y, crownZ));
    trunks.setMatrixAt(index, place.makeScale(size, size, size).setPosition(x, y, 0));
    if (isReal) {
      shade.set(crownColour).offsetHSL((seedOf(index + 1) - 0.5) * CROWN_HUE_SPREAD, 0, (seedOf(index + 2) - 0.5) * CROWN_LIGHTNESS_SPREAD);
      crowns.setColorAt(index, shade);
    }
  });
  for (const mesh of [crowns, trunks]) {
    mesh.castShadow = true;
    mesh.receiveShadow = true;
  }
  const group = new THREE.Group();
  group.add(crowns, trunks);
  return group;
}

/** Frees the GPU memory of everything under `group`. */
export function disposeGroup(group: THREE.Group) {
  group.traverse((object) => {
    if (object instanceof THREE.Mesh) {
      object.geometry.dispose();
      const materials = Array.isArray(object.material) ? object.material : [object.material];
      materials.forEach((material) => material.dispose());
    }
  });
}
