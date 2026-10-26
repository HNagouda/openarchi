// The 3D view's renderer, camera and light. It draws only when something
// changed (the camera, the sun, the size) or while the sun or the orbit is
// playing, so an open view that nobody touches costs no processor time.
//
// One directional light is the sun, with a shadow map over the site; a soft
// sky light keeps the shaded sides readable. The camera has z up, as the
// core's frame has it; navigation.ts moves it.

import * as THREE from "three";
import type { SunStep, View3d } from "../lib/types";
import { createNavigation, type Mode, type Navigation, type SheetAngle } from "./navigation";
import { disposeGroup, siteGroup } from "./scene";
import { isUp, towardSun } from "./sunDay";

const FIELD_OF_VIEW_DEG = 35;
// Near enough for an eye at 1.7 m to see the pavement at its feet.
const NEAR_M = 0.2;
const FAR_RADII = 40;
const MIN_RADIUS_M = 10;
// The shadow map covers the site and the long shadows a low sun casts past it.
const SHADOW_REACH_RADII = 1.6;
const SHADOW_MAP_PX = 4096;
const SHADOW_BIAS = -0.0004;
const SHADOW_NORMAL_BIAS = 0.4;
// Day: a strong sun and a soft sky. Night: the sky alone, dimmer.
const SUN_INTENSITY = 2.6;
const SKY_DAY = 1.1;
const SKY_NIGHT = 0.35;
const SKY_COLOUR = "#FFFFFF";
const GROUND_BOUNCE_COLOUR = "#B8B0A0";
// One turn of the orbit takes this long.
const ORBIT_SECONDS = 30;
// Never more pixels than this per CSS pixel, so a large display does not drown a small GPU.
const MAX_PIXEL_RATIO = 2;

export type Stage = {
  setSun: (sun: SunStep) => void;
  /** Turns the camera round the site until called with false. */
  setOrbiting: (isOrbiting: boolean) => void;
  /** Calls `tick` with the seconds since the last frame on every frame until it returns false. */
  animate: (tick: (seconds: number) => boolean) => void;
  /** The current picture as base64 PNG, without the data URL prefix. */
  capture: () => string;
  /** The camera turned round the site centre by `degrees`, for frames of an orbit. */
  turnCamera: (degrees: number) => void;
  navigation: Navigation;
  dispose: () => void;
};

export function createStage(host: HTMLElement, view: View3d, angle: SheetAngle, onMode: (mode: Mode) => void): Stage {
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  // Shadows follow the sun, not the camera, so turning the view reuses them.
  renderer.shadowMap.autoUpdate = false;
  // The first picture makes the map, so no frame samples a shadow that is not there yet.
  renderer.shadowMap.needsUpdate = true;
  host.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(view.sky);
  const site = siteGroup(view);
  scene.add(site);

  const radius = Math.max(view.radius_m, MIN_RADIUS_M);
  const camera = new THREE.PerspectiveCamera(FIELD_OF_VIEW_DEG, 1, NEAR_M, radius * FAR_RADII);
  camera.up.set(0, 0, 1);

  const sky = new THREE.HemisphereLight(SKY_COLOUR, GROUND_BOUNCE_COLOUR, SKY_DAY);
  scene.add(sky);
  const sun = new THREE.DirectionalLight(SKY_COLOUR, SUN_INTENSITY);
  sun.castShadow = true;
  const reach = radius * SHADOW_REACH_RADII;
  Object.assign(sun.shadow.camera, { left: -reach, right: reach, top: reach, bottom: -reach, near: 1, far: reach * 4 });
  const mapSize = Math.min(SHADOW_MAP_PX, renderer.capabilities.maxTextureSize);
  sun.shadow.mapSize.set(mapSize, mapSize);
  sun.shadow.bias = SHADOW_BIAS;
  sun.shadow.normalBias = SHADOW_NORMAL_BIAS;
  scene.add(sun, sun.target);

  // The animation frame that is running, 0 when none is.
  let loop = 0;
  let isFramePending = false;
  const draw = () => {
    isFramePending = false;
    renderer.render(scene, camera);
  };
  const requestDraw = () => {
    // A running animation draws every frame anyway.
    if (isFramePending || loop !== 0) return;
    isFramePending = true;
    requestAnimationFrame(draw);
  };

  const resize = () => {
    const { clientWidth, clientHeight } = host;
    if (clientWidth === 0 || clientHeight === 0) return;
    renderer.setSize(clientWidth, clientHeight);
    camera.aspect = clientWidth / clientHeight;
    camera.updateProjectionMatrix();
    requestDraw();
  };
  const watcher = new ResizeObserver(resize);
  watcher.observe(host);
  resize();

  // One loop for every animation, stopped as soon as none wants another frame.
  let ticks: ((seconds: number) => boolean)[] = [];
  let lastTime: number | null = null;
  const frame = (time: number) => {
    const seconds = lastTime === null ? 0 : (time - lastTime) / 1000;
    lastTime = time;
    ticks = ticks.filter((tick) => tick(seconds));
    renderer.render(scene, camera);
    loop = ticks.length > 0 ? requestAnimationFrame(frame) : 0;
    if (loop === 0) lastTime = null;
  };
  const animate = (tick: (seconds: number) => boolean) => {
    ticks.push(tick);
    if (loop === 0) loop = requestAnimationFrame(frame);
  };

  const navigation = createNavigation({
    camera,
    canvas: renderer.domElement,
    site,
    radius,
    eyeHeightM: view.eye_height_m,
    angle,
    requestDraw,
    animate,
    onMode,
  });

  let isOrbiting = false;
  const turnCamera = (degrees: number) => navigation.turn(degrees);
  const setOrbiting = (next: boolean) => {
    if (next === isOrbiting) return;
    isOrbiting = next;
    if (!next) return;
    animate((seconds) => {
      if (!isOrbiting) return false;
      turnCamera((-360 * seconds) / ORBIT_SECONDS);
      return true;
    });
  };

  const setSun = (step: SunStep) => {
    const [east, north, up] = towardSun(step);
    const isDay = isUp(step);
    sun.visible = isDay;
    sky.intensity = isDay ? SKY_DAY : SKY_NIGHT;
    sun.position.set(east * reach * 2, north * reach * 2, up * reach * 2);
    renderer.shadowMap.needsUpdate = true;
    requestDraw();
  };

  const capture = () => {
    // Drawn and read in one go, so the buffer still holds the picture.
    renderer.render(scene, camera);
    return renderer.domElement.toDataURL("image/png").split(",")[1];
  };

  // A hidden window stops turning; the next change draws it again.
  const onVisibility = () => {
    if (document.hidden) setOrbiting(false);
  };
  document.addEventListener("visibilitychange", onVisibility);

  const dispose = () => {
    isOrbiting = false;
    ticks = [];
    cancelAnimationFrame(loop);
    document.removeEventListener("visibilitychange", onVisibility);
    watcher.disconnect();
    navigation.dispose();
    disposeGroup(site);
    renderer.dispose();
    renderer.domElement.remove();
  };

  return { setSun, setOrbiting, animate, capture, turnCamera, navigation, dispose };
}
