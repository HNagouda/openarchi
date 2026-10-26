// The light and air round the site: the sun with soft shadows sized to the
// site, a sky light that is bluer from above than from the ground, a sky that
// pales toward the horizon, a haze that hides where the data ends, and a wide
// ground under everything. In the palette finish the sky is the sheet's
// paper and the sun is white, so the palette keeps its colours; in the
// realistic finish the sky is blue and a low sun warms.

import * as THREE from "three";
import type { SunStep, View3d } from "../lib/types";
import type { Finish } from "./materials";
import { REAL_HORIZON, REAL_ZENITH } from "./materials";
import { isUp, towardSun } from "./sunDay";

// The shadow map covers the site and the long shadows a low sun casts past it.
const SHADOW_REACH_RADII = 1.6;
const SHADOW_MAP_PX = 4096;
const SHADOW_BIAS = -0.0004;
const SHADOW_NORMAL_BIAS = 0.4;
// Texels a shadow's edge is spread over: soft, without losing a lamppost's shadow.
const SHADOW_SOFTNESS = 2.5;
// Day: a strong sun and a sky light that keeps shaded walls readable. Night: the sky alone, dimmer.
const SUN_INTENSITY = 2.6;
const SKY_DAY = 1.4;
const SKY_NIGHT = 0.4;
// A real sun is orange at the horizon and near white from this altitude up.
const LOW_SUN = "#FFB27A";
const HIGH_SUN = "#FFF5E8";
const WHITE = "#FFFFFF";
const WARM_UNTIL_DEG = 30;
// What the ground gives back to the shaded side of things: a neutral warm grey in
// the palette, so every palette is lit alike, and a real ground's brown otherwise.
const NEUTRAL_BOUNCE = "#B8B0A0";
const REAL_BOUNCE = "#8B8371";
// The haze starts this many site radii from the camera and is whole at the second.
const HAZE_FROM_RADII = 3.5;
const HAZE_TO_RADII = 14;
// The wide ground and the sky dome, in site radii; both inside the camera's far plane.
const FAR_GROUND_RADII = 30;
const SKY_DOME_RADII = 32;
// Under the site's own ground by enough that the two never flicker, even from far away.
const FAR_GROUND_DEPTH_M = -0.5;
// Past the data the realistic ground is a pale field colour, which the haze soon takes.
const REAL_FAR_GROUND = "#A3A894";
const FAR_SHADOW_OPACITY = 0.25;

export type Atmosphere = {
  group: THREE.Group;
  sun: THREE.DirectionalLight;
  setSun: (step: SunStep) => void;
  dispose: () => void;
};

/** The sky, haze, lights and far ground for `finish`, set on `scene`. */
export function createAtmosphere(scene: THREE.Scene, view: View3d, radius: number, finish: Finish, maxTexturePx: number): Atmosphere {
  const isReal = finish === "realistic";
  const horizon = isReal ? REAL_HORIZON : view.sky.horizon;
  const zenith = isReal ? REAL_ZENITH : view.sky.zenith;
  const group = new THREE.Group();
  scene.fog = new THREE.Fog(horizon, radius * HAZE_FROM_RADII, radius * HAZE_TO_RADII);
  scene.background = new THREE.Color(horizon);

  const dome = skyDome(horizon, zenith, radius * SKY_DOME_RADII);
  // The far ground is unlit and not tone mapped, so in the palette it is exactly the
  // sheet's paper and meets the haze without a seam; a second plane catches the shadows on it.
  const farSize = radius * FAR_GROUND_RADII * 2;
  const farGround = new THREE.Mesh(
    new THREE.PlaneGeometry(farSize, farSize),
    new THREE.MeshBasicMaterial({ color: isReal ? REAL_FAR_GROUND : horizon, toneMapped: false }),
  );
  farGround.position.z = FAR_GROUND_DEPTH_M;
  const farShadows = new THREE.Mesh(new THREE.PlaneGeometry(farSize, farSize), new THREE.ShadowMaterial({ opacity: FAR_SHADOW_OPACITY }));
  farShadows.position.z = FAR_GROUND_DEPTH_M / 2;
  farShadows.receiveShadow = true;

  // The palette colours the backdrop, never the light: a dark palette's sky would put the site in the dark.
  const sky = isReal ? new THREE.HemisphereLight(zenith, REAL_BOUNCE, SKY_DAY) : new THREE.HemisphereLight(WHITE, NEUTRAL_BOUNCE, SKY_DAY);
  sky.position.set(0, 0, 1);
  const sun = new THREE.DirectionalLight(WHITE, SUN_INTENSITY);
  sun.castShadow = true;
  const reach = radius * SHADOW_REACH_RADII;
  Object.assign(sun.shadow.camera, { left: -reach, right: reach, top: reach, bottom: -reach, near: 1, far: reach * 4 });
  const mapSize = Math.min(SHADOW_MAP_PX, maxTexturePx);
  sun.shadow.mapSize.set(mapSize, mapSize);
  sun.shadow.bias = SHADOW_BIAS;
  sun.shadow.normalBias = SHADOW_NORMAL_BIAS;
  sun.shadow.radius = SHADOW_SOFTNESS;
  group.add(dome, farGround, farShadows, sky, sun, sun.target);
  scene.add(group);

  const low = new THREE.Color(LOW_SUN);
  const high = new THREE.Color(HIGH_SUN);
  const setSun = (step: SunStep) => {
    const [east, north, up] = towardSun(step);
    const isDay = isUp(step);
    sun.visible = isDay;
    sky.intensity = isDay ? SKY_DAY : SKY_NIGHT;
    sun.position.set(east * reach * 2, north * reach * 2, up * reach * 2);
    if (isReal) sun.color.lerpColors(low, high, THREE.MathUtils.clamp(step.altitude_deg / WARM_UNTIL_DEG, 0, 1));
  };

  const dispose = () => {
    scene.remove(group);
    scene.fog = null;
    for (const mesh of [dome, farGround, farShadows]) {
      mesh.geometry.dispose();
      (mesh.material as THREE.Material).dispose();
    }
    sun.shadow.map?.dispose();
  };

  return { group, sun, setSun, dispose };
}

/** A sphere seen from inside, the horizon colour at and below eye level, the zenith overhead. */
function skyDome(horizon: string, zenith: string, radius: number): THREE.Mesh {
  const material = new THREE.ShaderMaterial({
    uniforms: { uHorizon: { value: new THREE.Color(horizon) }, uZenith: { value: new THREE.Color(zenith) } },
    vertexShader: `
      varying vec3 vDirection;
      void main() {
        vDirection = normalize(position);
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }`,
    fragmentShader: `
      uniform vec3 uHorizon;
      uniform vec3 uZenith;
      varying vec3 vDirection;
      void main() {
        float height = clamp(vDirection.z, 0.0, 1.0);
        gl_FragColor = vec4(mix(uHorizon, uZenith, pow(height, 0.6)), 1.0);
        #include <colorspace_fragment>
      }`,
    side: THREE.BackSide,
    depthWrite: false,
    fog: false,
    // Not tone mapped, like the haze and the background, so the horizon meets the far ground without a seam.
    toneMapped: false,
  });
  const dome = new THREE.Mesh(new THREE.SphereGeometry(radius, 32, 16), material);
  // Drawn first, behind everything.
  dome.renderOrder = -1;
  return dome;
}
