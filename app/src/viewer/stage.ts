// The 3D view's renderer, its site and its loop. It draws only when something
// changed (the camera, the sun, the size, the finish) or while the sun, the
// orbit or a move is playing, so an open view that nobody touches costs no
// processor time. atmosphere.ts lights it, navigation.ts moves the camera,
// scene.ts builds what it shows.
//
// Colours go through a neutral tone map, which leaves a palette colour as it
// is up to bright highlights and only rolls those off, rather than the film
// curves that would shift every hue of a palette.

import * as THREE from "three";
import type { AerialPhoto, SunStep, View3d } from "../lib/types";
import { createAtmosphere, type Atmosphere } from "./atmosphere";
import type { Finish } from "./materials";
import { createNavigation, type Mode, type Navigation, type SheetAngle } from "./navigation";
import { photoGroup, type PhotoLevel } from "./photo";
import { disposeGroup, siteGroup } from "./scene";

const FIELD_OF_VIEW_DEG = 35;
// The near plane follows the camera's height: close enough at eye level to see the
// pavement at one's feet, further from the air, so the depth buffer keeps ground
// layers a few centimetres apart from each other at any distance.
const NEAR_SHARE_OF_HEIGHT = 0.05;
const MIN_NEAR_M = 0.2;
const MAX_NEAR_M = 20;
const FAR_RADII = 40;
const MIN_RADIUS_M = 10;
// One turn of the orbit takes this long.
const ORBIT_SECONDS = 30;
// Never more pixels than this per CSS pixel, so a large display does not drown a small GPU.
const MAX_PIXEL_RATIO = 2;

export type Stage = {
  setSun: (sun: SunStep) => void;
  /** Paints the site in the sheet's palette or in real colours. */
  setFinish: (finish: Finish) => void;
  /** Lays the aerial photo on the ground, shown in the realistic finish. */
  setPhoto: (photo: AerialPhoto) => Promise<void>;
  /** Turns the camera round the site until called with false. */
  setOrbiting: (isOrbiting: boolean) => void;
  /** Calls `tick` with the seconds since the last frame on every frame until it returns false. */
  animate: (tick: (seconds: number) => boolean) => void;
  /** The current picture as a canvas, drawn just now. */
  picture: () => HTMLCanvasElement;
  /** The camera turned round the site centre by `degrees`, for frames of an orbit. */
  turnCamera: (degrees: number) => void;
  navigation: Navigation;
  dispose: () => void;
};

type Options = {
  view: View3d;
  finish: Finish;
  angle: SheetAngle;
  onMode: (mode: Mode) => void;
};

export function createStage(host: HTMLElement, { view, finish, angle, onMode }: Options): Stage {
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO));
  renderer.toneMapping = THREE.NeutralToneMapping;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  // Shadows follow the sun, not the camera, so turning the view reuses them.
  renderer.shadowMap.autoUpdate = false;
  // The first picture makes the map, so no frame samples a shadow that is not there yet.
  renderer.shadowMap.needsUpdate = true;
  host.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  const radius = Math.max(view.radius_m, MIN_RADIUS_M);
  // The site sits in a holder of its own, so changing the finish swaps what is inside.
  const world = new THREE.Group();
  scene.add(world);
  let shownFinish = finish;
  let photo: { group: THREE.Group; levels: PhotoLevel[] } | null = null;
  // A photo still decoding when the view closes is thrown away.
  let isDisposed = false;
  let site = siteGroup(view, finish, null);
  world.add(site);
  const maxTexturePx = renderer.capabilities.maxTextureSize;
  let atmosphere: Atmosphere = createAtmosphere(scene, view, radius, finish, maxTexturePx);
  let lastSun: SunStep | null = null;

  const camera = new THREE.PerspectiveCamera(FIELD_OF_VIEW_DEG, 1, MIN_NEAR_M, radius * FAR_RADII);
  camera.up.set(0, 0, 1);

  // The animation frame that is running, 0 when none is.
  let loop = 0;
  let isFramePending = false;
  const render = () => {
    const near = THREE.MathUtils.clamp(camera.position.z * NEAR_SHARE_OF_HEIGHT, MIN_NEAR_M, MAX_NEAR_M);
    if (near !== camera.near) {
      camera.near = near;
      camera.updateProjectionMatrix();
    }
    renderer.render(scene, camera);
  };
  const draw = () => {
    isFramePending = false;
    render();
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
    render();
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
    site: world,
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
    lastSun = step;
    atmosphere.setSun(step);
    renderer.shadowMap.needsUpdate = true;
    requestDraw();
  };

  /** Builds the site again for the finish shown, on the photo when there is one to show. */
  const rebuild = () => {
    const shownPhoto = shownFinish === "realistic" ? photo : null;
    world.remove(site);
    disposeGroup(site);
    site = siteGroup(view, shownFinish, shownPhoto?.levels ?? null);
    world.add(site);
    if (photo) photo.group.visible = shownPhoto !== null;
    renderer.shadowMap.needsUpdate = true;
    requestDraw();
  };

  const setFinish = (next: Finish) => {
    if (next === shownFinish) return;
    shownFinish = next;
    atmosphere.dispose();
    atmosphere = createAtmosphere(scene, view, radius, next, maxTexturePx);
    if (lastSun) atmosphere.setSun(lastSun);
    rebuild();
  };

  const setPhoto = async (aerial: AerialPhoto) => {
    const built = await photoGroup(aerial, renderer.capabilities.getMaxAnisotropy());
    if (isDisposed) {
      disposeGroup(built.group);
      return;
    }
    if (photo) {
      world.remove(photo.group);
      disposeGroup(photo.group);
    }
    photo = built;
    world.add(photo.group);
    rebuild();
  };

  // Drawn and returned in one go, so the canvas still holds the picture when it is read.
  const picture = () => {
    render();
    return renderer.domElement;
  };

  // A hidden window stops turning; the next change draws it again.
  const onVisibility = () => {
    if (document.hidden) setOrbiting(false);
  };
  document.addEventListener("visibilitychange", onVisibility);

  const dispose = () => {
    isDisposed = true;
    isOrbiting = false;
    ticks = [];
    cancelAnimationFrame(loop);
    document.removeEventListener("visibilitychange", onVisibility);
    watcher.disconnect();
    navigation.dispose();
    atmosphere.dispose();
    disposeGroup(site);
    if (photo) disposeGroup(photo.group);
    renderer.dispose();
    renderer.domElement.remove();
  };

  return { setSun, setFinish, setPhoto, setOrbiting, animate, picture, turnCamera, navigation, dispose };
}
