// The site as three.js meshes, from the triangles the core sends: the ground
// in the palette's colours, buildings and the model in plain light greys, and
// trees as a trunk and a round crown. Everything casts or catches shadows.
// The frame is the core's: x east, y north, z up, in metres.

import * as THREE from "three";
import type { Surface, View3d } from "../lib/types";

// Corners per triangle, and numbers per corner.
const CORNERS = 3;
const TREE_CROWN_DETAIL = 1;
const TRUNK_RADIUS_M = 0.25;
const TRUNK_SIDES = 6;
const TRUNK_COLOUR = "#6B5B4B";

/** Every mesh of the site in one group, ready to add to a scene. */
export function siteGroup(view: View3d): THREE.Group {
  const group = new THREE.Group();
  for (const surface of view.ground) {
    // The ground only catches shadows: its pieces are flat and cannot cast any.
    group.add(surfaceMesh(surface, { castsShadow: false, isDoubleSided: false }));
  }
  group.add(surfaceMesh(view.buildings, { castsShadow: true, isDoubleSided: false }));
  if (view.neighbours.triangle_count > 0) group.add(surfaceMesh(view.neighbours, { castsShadow: true, isDoubleSided: false }));
  // An exported model may face some triangles inward; both sides are drawn so none goes missing.
  if (view.model) group.add(surfaceMesh(view.model, { castsShadow: true, isDoubleSided: true }));
  if (view.trees.at.length > 0) group.add(trees(view.trees));
  return group;
}

function surfaceMesh(surface: Surface, { castsShadow, isDoubleSided }: { castsShadow: boolean; isDoubleSided: boolean }): THREE.Mesh {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(corners(surface.corners), CORNERS));
  // Each triangle has corners of its own, so these normals are the triangles' own: flat faces.
  geometry.computeVertexNormals();
  const material = new THREE.MeshLambertMaterial({ color: surface.colour, side: isDoubleSided ? THREE.DoubleSide : THREE.FrontSide });
  const mesh = new THREE.Mesh(geometry, material);
  mesh.castShadow = castsShadow;
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
function trees(trees: View3d["trees"]): THREE.Group {
  const crownGeometry = new THREE.IcosahedronGeometry(trees.crown_m, TREE_CROWN_DETAIL);
  const trunkHeight = trees.height_m - trees.crown_m;
  const trunkGeometry = new THREE.CylinderGeometry(TRUNK_RADIUS_M, TRUNK_RADIUS_M, trunkHeight, TRUNK_SIDES);
  // three.js stands a cylinder along y; the site has z up.
  trunkGeometry.rotateX(Math.PI / 2);
  const crowns = new THREE.InstancedMesh(crownGeometry, new THREE.MeshLambertMaterial({ color: trees.colour, flatShading: true }), trees.at.length);
  const trunks = new THREE.InstancedMesh(trunkGeometry, new THREE.MeshLambertMaterial({ color: TRUNK_COLOUR }), trees.at.length);
  const place = new THREE.Matrix4();
  trees.at.forEach(([x, y], index) => {
    crowns.setMatrixAt(index, place.makeTranslation(x, y, trees.height_m - trees.crown_m));
    trunks.setMatrixAt(index, place.makeTranslation(x, y, trunkHeight / 2));
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
