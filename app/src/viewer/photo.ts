// The aerial photo as ground: each level's JPEG tiles drawn side by side on
// one canvas, which becomes the texture of a flat plane exactly over those
// tiles. The site's finer level lies on top of the neighbourhood's coarser one,
// and each fades out at its edges into what lies under it.
//
// The photo already holds a day's light, so it is shown as it is, unlit; a
// clear plane over it catches the view's own shadows and darkens it there.
//
// #! NOTE[3d, photo] : The photo was taken on a summer day, so it carries that day's shadows under the view's own; a shadow-free photo does not exist for the whole country
// #! NOTE[3d, photo] : A level's texture is the tiles' size, not always a power of two; WebGL2 mipmaps any size, which every supported webview has

import * as THREE from "three";
import type { AerialPhoto } from "../lib/types";

// The neighbourhood's photo lies under the site's, both under the site's water
// and above the far ground, far enough apart not to flicker from any distance.
const LEVEL_DEPTHS_M = [0, -0.15];
const SHADOWS_DEPTH_M = 0.02;
// How dark the view's shadows make the photo: lighter than on plain ground, as
// the photo's own shade already darkens the north side of everything.
const SHADOW_OPACITY = 0.35;
// Each level fades to nothing over this many metres at its edges.
const EDGE_FADE_M = 60;

/** One level's texture and where it lies, x and y of its south-west corner, then its width and height. */
export type PhotoLevel = { texture: THREE.Texture; rect: THREE.Vector4 };

/** The photo's levels as textured planes, the site first, and their textures for the roofs. */
export async function photoGroup(photo: AerialPhoto, maxAnisotropy: number): Promise<{ group: THREE.Group; levels: PhotoLevel[] }> {
  const group = new THREE.Group();
  const levels: PhotoLevel[] = [];
  for (const [index, level] of photo.levels.entries()) {
    const texture = new THREE.CanvasTexture(await stitched(level));
    texture.colorSpace = THREE.SRGBColorSpace;
    // Sharper at the low angles a walk looks at the ground from.
    texture.anisotropy = maxAnisotropy;
    const width = level.columns * level.tile_m;
    const height = level.rows * level.tile_m;
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(width, height), fadingPhoto(texture, width, height));
    plane.position.set(level.west + width / 2, level.south + height / 2, LEVEL_DEPTHS_M[index] ?? LEVEL_DEPTHS_M[LEVEL_DEPTHS_M.length - 1]);
    group.add(plane);
    levels.push({ texture, rect: new THREE.Vector4(level.west, level.south, width, height) });
  }

  // The widest level is the last; its plane catches every shadow over the photo.
  const widest = photo.levels[photo.levels.length - 1];
  const width = widest.columns * widest.tile_m;
  const height = widest.rows * widest.tile_m;
  const shadows = new THREE.Mesh(new THREE.PlaneGeometry(width, height), new THREE.ShadowMaterial({ opacity: SHADOW_OPACITY }));
  shadows.position.set(widest.west + width / 2, widest.south + height / 2, SHADOWS_DEPTH_M);
  shadows.receiveShadow = true;
  group.add(shadows);
  return { group, levels };
}

/** The photo as it is, not tone mapped, fading out within `EDGE_FADE_M` of its edges. */
function fadingPhoto(texture: THREE.Texture, width: number, height: number): THREE.MeshBasicMaterial {
  const material = new THREE.MeshBasicMaterial({ map: texture, transparent: true, toneMapped: false });
  material.onBeforeCompile = (shader) => {
    shader.uniforms.uSize = { value: new THREE.Vector2(width, height) };
    shader.fragmentShader = shader.fragmentShader
      .replace("#include <common>", "#include <common>\nuniform vec2 uSize;")
      .replace(
        "#include <map_fragment>",
        `#include <map_fragment>
        vec2 fromEdge = min(vMapUv, 1.0 - vMapUv) * uSize;
        diffuseColor.a *= smoothstep(0.0, ${EDGE_FADE_M.toFixed(1)}, min(fromEdge.x, fromEdge.y));`,
      );
  };
  // Keyed by size: a program shared between levels would keep the first level's size.
  material.customProgramCacheKey = () => `fading-photo-${width}x${height}`;
  return material;
}

/** One canvas with every tile in its place, north at the top. */
async function stitched(level: AerialPhoto["levels"][number]): Promise<HTMLCanvasElement> {
  const canvas = document.createElement("canvas");
  canvas.width = level.columns * level.tile_px;
  canvas.height = level.rows * level.tile_px;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("The aerial photo could not be drawn.");
  const images = await Promise.all(level.tiles.map((tile) => decoded(tile.jpeg)));
  level.tiles.forEach((tile, index) => {
    const top = (level.rows - 1 - tile.row) * level.tile_px;
    context.drawImage(images[index], tile.column * level.tile_px, top, level.tile_px, level.tile_px);
  });
  return canvas;
}

async function decoded(jpeg: string): Promise<HTMLImageElement> {
  const image = new Image();
  image.src = `data:image/jpeg;base64,${jpeg}`;
  await image.decode();
  return image;
}
