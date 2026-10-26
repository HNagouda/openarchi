// How each thing in the 3D view is painted. In the palette finish everything
// keeps the sheet's colours. In the realistic finish buildings get brick and
// plaster facades with rows of windows a storey apart, darker roofs, and
// water a sheen of the sky. All of it is worked out in the shaders from the
// position of each pixel, so no image is downloaded or shipped.
//
// Both finishes darken the foot of every wall a little: less sky reaches the
// bottom of a street, and that cheap baked shade grounds the buildings
// without a screen-space occlusion pass.

import * as THREE from "three";
import type { Layer } from "../lib/types";
import type { PhotoLevel } from "./photo";

export type Finish = "palette" | "realistic";

// Facade tones of Dutch streets: red, dark and buff brick, white plaster, grey render.
const FACADES = ["#9C6B56", "#7E5C4E", "#BFA586", "#DDD6CA", "#B1ADA5"];
// Roofs are mostly dark tiles or bitumen, a little of the facade showing through.
const ROOF = "#5C5854";
const ROOF_SHARE_OF_FACADE = 0.25;
const GLASS = "#26303A";
// Bays between windows vary per building between these, in metres.
const BAY_MIN_M = 2.2;
const BAY_MAX_M = 3.4;
// A window's share of its bay, and its sill and head as shares of the storey.
const WINDOW_SHARE = 0.5;
const SILL_SHARE = 0.3;
const HEAD_SHARE = 0.78;
// No windows in the top of a wall: the parapet or the eaves.
const PARAPET_M = 0.8;
// The foot of a wall is this much darker, fading out over this height.
const FOOT_SHADE = 0.72;
const FOOT_M = 2.5;
// How much of the sky a level water surface shows at a grazing angle.
const WATER_SHEEN = 0.55;

// The ground in real colours, for where no aerial photo is shown.
export const REAL_GROUND: Partial<Record<Layer, string>> = {
  road: "#5E5F61",
  parking: "#6B6B6A",
  // The Netherlands paves its cycle paths in red asphalt.
  cycleway: "#8C5A4E",
  footpath: "#A39D92",
  paved: "#99948B",
  bridge: "#7E786F",
  rail_area: "#6C6457",
  green: "#6E8A4C",
  verge: "#7B8F55",
  yard: "#8D887B",
  unpaved: "#9B8E73",
  water: "#5D7680",
  water_bank: "#8A8576",
};
export const REAL_OPEN_GROUND = "#86906F";
export const REAL_TREE = "#58763A";
export const REAL_TRUNK = "#594A3C";
export const REAL_MODEL = "#EDEAE4";
// A clear Dutch sky, pale at the horizon.
export const REAL_HORIZON = "#D8E2EA";
export const REAL_ZENITH = "#7FA4CC";

type BuildingOptions = {
  finish: Finish;
  storeyM: number;
  hasFacades: boolean;
  /** The aerial photo's levels, site first: realistic roofs take their colour from it. */
  photo: PhotoLevel[] | null;
};

/** A building surface: its colour in the palette, or brick and windows in the realistic finish. */
export function buildingMaterial(colour: string, { finish, storeyM, hasFacades, photo }: BuildingOptions): THREE.MeshLambertMaterial {
  const material = new THREE.MeshLambertMaterial({ color: colour });
  const isFacaded = finish === "realistic" && hasFacades;
  const roofPhoto = isFacaded && photo && photo.length > 0 ? photo : null;
  material.onBeforeCompile = (shader) => {
    shader.uniforms.uFacades = { value: FACADES.map((tone) => new THREE.Color(tone)) };
    shader.uniforms.uRoof = { value: new THREE.Color(ROOF) };
    shader.uniforms.uGlass = { value: new THREE.Color(GLASS) };
    shader.uniforms.uStorey = { value: storeyM };
    if (isFacaded) shader.defines = { ...shader.defines, FACADES: "" };
    if (roofPhoto) {
      // The site's level and the widest; the same one twice when there is only one.
      const around = roofPhoto[roofPhoto.length - 1];
      shader.defines = { ...shader.defines, ROOF_PHOTO: "" };
      shader.uniforms.uSitePhoto = { value: roofPhoto[0].texture };
      shader.uniforms.uSiteRect = { value: roofPhoto[0].rect };
      shader.uniforms.uAroundPhoto = { value: around.texture };
      shader.uniforms.uAroundRect = { value: around.rect };
    }
    shader.vertexShader = withSitePosition(shader.vertexShader, `attribute float aTop;\nattribute float aSeed;\nvarying float vTop;\nvarying float vSeed;`, `vTop = aTop;\nvSeed = aSeed;`);
    shader.fragmentShader = shader.fragmentShader
      .replace("#include <common>", `#include <common>\n${SITE_VARYINGS}\n${FACADE_FUNCTIONS}`)
      .replace("#include <color_fragment>", `#include <color_fragment>\n${BUILDING_COLOUR}`);
  };
  // Every palette colour compiles the same program; only the finish changes it.
  material.customProgramCacheKey = () => (roofPhoto ? "building-roof-photo" : isFacaded ? "building-facades" : "building");
  return material;
}

/** Water: flat in the palette, with a sheen of the sky in the realistic finish. */
export function waterMaterial(colour: string, finish: Finish, sky: string, opacity = 1): THREE.MeshLambertMaterial {
  const material = new THREE.MeshLambertMaterial({ color: colour, transparent: opacity < 1, opacity });
  if (finish !== "realistic") return material;
  material.onBeforeCompile = (shader) => {
    shader.uniforms.uSky = { value: new THREE.Color(sky) };
    shader.vertexShader = withSitePosition(shader.vertexShader, "", "");
    shader.fragmentShader = shader.fragmentShader
      .replace("#include <common>", `#include <common>\n${SITE_VARYINGS}\nuniform vec3 uSky;`)
      .replace("#include <emissivemap_fragment>", `#include <emissivemap_fragment>\n${WATER_SHEEN_GLSL}`);
  };
  material.customProgramCacheKey = () => "water-sheen";
  return material;
}

/** Per-corner attributes a building surface's shader reads: the top of its building and a seed per building. */
export function buildingAttributes(geometry: THREE.BufferGeometry, pieces: number[] | undefined) {
  const position = geometry.getAttribute("position");
  const corners = position.count;
  const tops = new Float32Array(corners);
  const seeds = new Float32Array(corners);
  // Walls run from the ground to the roof, so a triangle's highest corner is its building's height.
  for (let first = 0; first < corners; first += 3) {
    const top = Math.max(position.getZ(first), position.getZ(first + 1), position.getZ(first + 2));
    tops.fill(top, first, first + 3);
  }
  let corner = 0;
  (pieces ?? []).forEach((triangles, index) => {
    seeds.fill(seedOf(index), corner, corner + triangles * 3);
    corner += triangles * 3;
  });
  geometry.setAttribute("aTop", new THREE.BufferAttribute(tops, 1));
  geometry.setAttribute("aSeed", new THREE.BufferAttribute(seeds, 1));
}

/** 0 to 1, spread evenly enough that neighbours rarely match. */
export function seedOf(index: number): number {
  const value = Math.sin(index * 12.9898 + 78.233) * 43758.5453;
  return value - Math.floor(value);
}

/** Adds the site position and normal to a vertex shader, with `declarations` and `assignments` of its own. */
function withSitePosition(vertexShader: string, declarations: string, assignments: string): string {
  return vertexShader
    .replace("#include <common>", `#include <common>\n${SITE_VARYINGS}\n${declarations}`)
    .replace("#include <project_vertex>", `#include <project_vertex>\nvSitePosition = (modelMatrix * vec4(transformed, 1.0)).xyz;\nvSiteNormal = normalize(mat3(modelMatrix) * objectNormal);\n${assignments}`);
}

const SITE_VARYINGS = `varying vec3 vSitePosition;\nvarying vec3 vSiteNormal;`;

const FACADE_FUNCTIONS = `
uniform vec3 uFacades[${FACADES.length}];
uniform vec3 uRoof;
uniform vec3 uGlass;
uniform float uStorey;
varying float vTop;
varying float vSeed;

// How much of this pixel is window: rows a storey apart, bays along the wall.
// Too far away to draw them crisply, it is their average share instead.
float windowShare(vec3 p, vec3 n) {
  vec2 along = normalize(vec2(-n.y, n.x));
  float u = dot(p.xy, along);
  float bay = mix(${BAY_MIN_M.toFixed(2)}, ${BAY_MAX_M.toFixed(2)}, fract(vSeed * 13.7));
  float acrossBay = abs(fract(u / bay) - 0.5);
  float level = p.z / uStorey;
  float inStorey = fract(level);
  float blurU = fwidth(u) / bay;
  float blurV = fwidth(level);
  float halfWidth = ${(WINDOW_SHARE / 2).toFixed(3)};
  float across = 1.0 - smoothstep(halfWidth - blurU, halfWidth + blurU, acrossBay);
  float up = smoothstep(${SILL_SHARE.toFixed(3)} - blurV, ${SILL_SHARE.toFixed(3)} + blurV, inStorey)
    * (1.0 - smoothstep(${HEAD_SHARE.toFixed(3)} - blurV, ${HEAD_SHARE.toFixed(3)} + blurV, inStorey));
  float detail = 1.0 - smoothstep(0.2, 0.5, max(blurU, blurV));
  float average = ${(WINDOW_SHARE * (HEAD_SHARE - SILL_SHARE)).toFixed(3)};
  float belowEaves = 1.0 - step(vTop - ${PARAPET_M.toFixed(2)}, p.z);
  return mix(average, across * up, detail) * belowEaves;
}

#ifdef ROOF_PHOTO
uniform sampler2D uSitePhoto;
uniform vec4 uSiteRect;
uniform sampler2D uAroundPhoto;
uniform vec4 uAroundRect;

// The roof as the aerial photo saw it: from the site's finer photo where it covers, else the neighbourhood's.
vec3 photographedRoof(vec2 at) {
  vec2 inSite = (at - uSiteRect.xy) / uSiteRect.zw;
  bool isInSite = all(greaterThanEqual(inSite, vec2(0.0))) && all(lessThanEqual(inSite, vec2(1.0)));
  vec2 inAround = (at - uAroundRect.xy) / uAroundRect.zw;
  return isInSite ? texture2D(uSitePhoto, inSite).rgb : texture2D(uAroundPhoto, inAround).rgb;
}
#endif

vec3 facadeTone() {
  int index = int(floor(vSeed * ${FACADES.length.toFixed(1)}));
  float brightness = 0.9 + 0.2 * fract(vSeed * 7.31);
  return uFacades[index] * brightness;
}
`;

const BUILDING_COLOUR = `
float isWall = 1.0 - step(0.5, abs(vSiteNormal.z));
#ifdef FACADES
  vec3 tone = facadeTone();
  vec3 roof = mix(uRoof, tone, ${ROOF_SHARE_OF_FACADE.toFixed(2)});
  #ifdef ROOF_PHOTO
    roof = photographedRoof(vSitePosition.xy);
  #endif
  diffuseColor.rgb = mix(roof, tone, isWall);
  diffuseColor.rgb = mix(diffuseColor.rgb, uGlass, windowShare(vSitePosition, vSiteNormal) * isWall);
#endif
float foot = mix(${FOOT_SHADE.toFixed(2)}, 1.0, smoothstep(0.0, ${FOOT_M.toFixed(1)}, vSitePosition.z));
diffuseColor.rgb *= mix(1.0, foot, isWall);
`;

// A Schlick-like rise toward grazing angles, rippled a little so it is not a flat sheet.
const WATER_SHEEN_GLSL = `
float facing = abs(normalize(cameraPosition - vSitePosition).z);
float ripple = 0.8 + 0.2 * sin(vSitePosition.x * 0.9 + 2.0 * sin(vSitePosition.y * 0.6));
totalEmissiveRadiance += uSky * pow(1.0 - facing, 3.0) * ${WATER_SHEEN.toFixed(2)} * ripple;
`;
