// Moving round the site. Drag turns the view round the point in the middle,
// right-drag or shift-drag slides it over the ground, the wheel and a pinch
// come closer to whatever is under the pointer, and a two-finger swipe on a
// trackpad slides. WASD or the arrows move, Q and E turn. Named views fly the
// camera to the whole site, straight down, the sheet's own tilted angle, or an
// eye on the ground: click where to stand, then drag to look round.
//
// OrbitControls does the dragging and its damping; everything else moves the
// camera and its target here and asks it to settle. Nothing runs between
// moves, so the view still draws only when something changed.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { isTrackpadScroll, wheelPixels } from "./wheel";

// The opening view: from the south-west and above, far enough to see the site whole.
const START_BEARING_DEG = 210;
const START_ELEVATION_DEG = 35;
const START_DISTANCE_RADII = 3.2;
// Straight down, with this much room round the site.
const TOP_MARGIN = 1.15;
// The target may wander this many site radii from the centre, the camera this far from the target.
const MAX_TARGET_RADII = 2.5;
const MAX_DISTANCE_RADII = 12;
const MIN_DISTANCE_M = 3;
// Just above the horizon, so the camera never looks up at the ground from below it.
const MAX_POLAR_ANGLE = Math.PI / 2 - 0.03;
// The camera keeps at least this far above the ground plane.
const MIN_CAMERA_Z_M = 0.5;
// Low damping keeps a drag crisp: most of the motion lands in the first few frames.
const DAMPING = 0.18;
// Zoom: how much one pixel of wheel or pinch brings the camera closer.
const WHEEL_ZOOM_PER_PIXEL = 0.0015;
const PINCH_ZOOM_PER_PIXEL = 0.01;
// Keys: a share of the distance to the target per second, and degrees per second.
const KEY_MOVE_SHARE_PER_SECOND = 0.6;
const KEY_TURN_DEG_PER_SECOND = 60;
const FAST_FACTOR = 4;
// Walking: the pace of a walk, how far ahead the eye looks, and how fast a drag turns it.
const WALK_M_PER_SECOND = 1.4;
const WALK_LOOK_AHEAD_M = 1;
const WALK_WHEEL_M_PER_PIXEL = 0.05;
const LOOK_RAD_PER_PIXEL = 0.004;
const MAX_LOOK_PITCH = THREE.MathUtils.degToRad(80);
// A press that moves less than this is a click, not a drag.
const CLICK_SLOP_PX = 4;
// Named views fly there in this long.
const FLY_SECONDS = 0.6;
// A surface this level or more counts as somewhere to stand.
const STANDABLE_NORMAL_Z = 0.5;
const UP = new THREE.Vector3(0, 0, 1);

export type Mode = "orbit" | "pick" | "walk";

export type SheetAngle = { rotationDeg: number; tiltDeg: number };

export type Navigation = {
  fit: () => void;
  top: () => void;
  /** The sheet's tilted view: the same turn and tilt, seen in perspective. */
  sheetAngle: () => void;
  /** The next click on the site stands the eye there. */
  pickStandpoint: () => void;
  /** Back to looking at the site from outside; false when already there. */
  leave: () => boolean;
  /** Turns the camera round the target, or the eye where it stands. */
  turn: (degrees: number) => void;
  dispose: () => void;
};

type Options = {
  camera: THREE.PerspectiveCamera;
  canvas: HTMLCanvasElement;
  /** What a click or the pointer can land on. */
  site: THREE.Object3D;
  radius: number;
  eyeHeightM: number;
  angle: SheetAngle;
  requestDraw: () => void;
  animate: (tick: (seconds: number) => boolean) => void;
  onMode: (mode: Mode) => void;
};

export function createNavigation(options: Options): Navigation {
  const { camera, canvas, radius } = options;
  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = DAMPING;
  controls.screenSpacePanning = false;
  controls.maxTargetRadius = radius * MAX_TARGET_RADII;
  let mode: Mode = "orbit";
  applyLimits();
  placeAt(...startView());

  // Damping needs an update every frame until the camera comes to rest.
  let isSettling = false;
  const settle = () => {
    if (isSettling) return;
    isSettling = true;
    options.animate(() => {
      isSettling = controls.update();
      return isSettling;
    });
  };
  controls.addEventListener("change", () => {
    options.requestDraw();
    settle();
  });

  function setMode(next: Mode) {
    mode = next;
    canvas.style.cursor = next === "pick" ? "crosshair" : "";
    applyLimits();
    options.onMode(next);
  }

  function applyLimits() {
    const isWalking = mode === "walk";
    controls.enableRotate = !isWalking;
    controls.enableZoom = !isWalking;
    controls.maxPolarAngle = isWalking ? Math.PI : MAX_POLAR_ANGLE;
    controls.minDistance = isWalking ? WALK_LOOK_AHEAD_M / 2 : MIN_DISTANCE_M;
    controls.maxDistance = radius * MAX_DISTANCE_RADII;
  }

  function placeAt(position: THREE.Vector3, target: THREE.Vector3) {
    camera.position.copy(position);
    controls.target.copy(target);
    camera.position.z = Math.max(camera.position.z, MIN_CAMERA_Z_M);
    controls.update();
    options.requestDraw();
  }

  /** Eases the camera and target from where they are to where they should be. */
  function flyTo(position: THREE.Vector3, target: THREE.Vector3) {
    const fromPosition = camera.position.clone();
    const fromTarget = controls.target.clone();
    let elapsed = 0;
    options.animate((seconds) => {
      elapsed = Math.min(elapsed + seconds, FLY_SECONDS);
      const share = THREE.MathUtils.smootherstep(elapsed, 0, FLY_SECONDS);
      camera.position.lerpVectors(fromPosition, position, share);
      controls.target.lerpVectors(fromTarget, target, share);
      controls.update();
      return elapsed < FLY_SECONDS;
    });
  }

  function viewFrom(bearingDeg: number, elevationDeg: number, distance: number): [THREE.Vector3, THREE.Vector3] {
    const bearing = THREE.MathUtils.degToRad(bearingDeg);
    const elevation = THREE.MathUtils.degToRad(elevationDeg);
    const position = new THREE.Vector3(
      Math.sin(bearing) * Math.cos(elevation) * distance,
      Math.cos(bearing) * Math.cos(elevation) * distance,
      Math.sin(elevation) * distance,
    );
    return [position, new THREE.Vector3()];
  }

  function startView() {
    return viewFrom(START_BEARING_DEG, START_ELEVATION_DEG, radius * START_DISTANCE_RADII);
  }

  function goTo([position, target]: [THREE.Vector3, THREE.Vector3]) {
    setMode("orbit");
    flyTo(position, target);
  }

  const fit = () => goTo(startView());
  const top = () => {
    const halfField = THREE.MathUtils.degToRad(camera.fov / 2);
    const fitted = Math.max(1, 1 / camera.aspect);
    const height = (radius * TOP_MARGIN * fitted) / Math.tan(halfField);
    // A hair south of straight down, so north stays up the screen.
    goTo([new THREE.Vector3(0, -height * 0.001, height), new THREE.Vector3()]);
  };
  const sheetAngle = () => {
    // The sheet turns the ground counterclockwise, which moves the viewer clockwise from the south.
    goTo(viewFrom(180 + options.angle.rotationDeg, options.angle.tiltDeg, radius * START_DISTANCE_RADII));
  };

  // * ====================================================================
  // * === pointer: where it lands, clicks, the wheel and a pinch
  // * ====================================================================

  const raycaster = new THREE.Raycaster();
  const ground = new THREE.Plane(UP, 0);

  /** The site under a screen point, or the ground plane past its edge. */
  function landing(clientX: number, clientY: number): THREE.Intersection | { point: THREE.Vector3; face: null } | null {
    const box = canvas.getBoundingClientRect();
    const pointer = new THREE.Vector2(((clientX - box.left) / box.width) * 2 - 1, -((clientY - box.top) / box.height) * 2 + 1);
    raycaster.setFromCamera(pointer, camera);
    const [hit] = raycaster.intersectObject(options.site, true);
    if (hit) return hit;
    const point = raycaster.ray.intersectPlane(ground, new THREE.Vector3());
    return point ? { point, face: null } : null;
  }

  /** Scales the camera and its target about the point under the pointer, which keeps that point still on screen. */
  function zoomAt(clientX: number, clientY: number, amount: number) {
    const point = landing(clientX, clientY)?.point ?? controls.target.clone();
    const scale = Math.exp(amount);
    const distance = camera.position.distanceTo(controls.target) * scale;
    if (distance < controls.minDistance || distance > controls.maxDistance) return;
    const position = point.clone().add(camera.position.clone().sub(point).multiplyScalar(scale));
    const target = point.clone().add(controls.target.clone().sub(point).multiplyScalar(scale));
    placeAt(position, target);
  }

  /** Slides the camera and target over the ground by screen pixels, as if the ground were dragged. */
  function slideBy(rightPx: number, downPx: number) {
    const distance = camera.position.distanceTo(controls.target);
    const metresPerPixel = (2 * distance * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2))) / canvas.clientHeight;
    const [forward, right] = groundAxes();
    const shift = right.multiplyScalar(rightPx * metresPerPixel).add(forward.multiplyScalar(-downPx * metresPerPixel));
    placeAt(camera.position.clone().add(shift), controls.target.clone().add(shift));
  }

  const onWheel = (event: WheelEvent) => {
    event.preventDefault();
    // Handled here rather than by OrbitControls, which would zoom on a trackpad swipe too.
    event.stopPropagation();
    const [x, y] = wheelPixels(event);
    if (mode === "walk") walkBy(-y * WALK_WHEEL_M_PER_PIXEL, 0);
    else if (event.ctrlKey) zoomAt(event.clientX, event.clientY, y * PINCH_ZOOM_PER_PIXEL);
    else if (isTrackpadScroll(event)) slideBy(x, y);
    else zoomAt(event.clientX, event.clientY, y * WHEEL_ZOOM_PER_PIXEL);
  };
  // Capturing on the parent runs before OrbitControls' own listener on the canvas.
  const host = canvas.parentElement ?? canvas;
  host.addEventListener("wheel", onWheel, { capture: true, passive: false });

  // Safari and WKWebView send a trackpad pinch as gesture events with a scale, not as a wheel.
  type Gesture = UIEvent & { scale: number; clientX: number; clientY: number };
  let lastScale = 1;
  const onGestureStart = (event: Event) => {
    event.preventDefault();
    lastScale = 1;
  };
  const onGestureChange = (event: Event) => {
    event.preventDefault();
    const gesture = event as Gesture;
    if (mode !== "walk") zoomAt(gesture.clientX, gesture.clientY, Math.log(lastScale / gesture.scale));
    lastScale = gesture.scale;
  };
  canvas.addEventListener("gesturestart", onGestureStart);
  canvas.addEventListener("gesturechange", onGestureChange);

  // Clicks pick the standpoint; drags look round while walking.
  let press: { x: number; y: number; lastX: number; lastY: number } | null = null;
  const onPointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    press = { x: event.clientX, y: event.clientY, lastX: event.clientX, lastY: event.clientY };
  };
  const onPointerMove = (event: PointerEvent) => {
    if (!press || mode !== "walk") return;
    look(event.clientX - press.lastX, event.clientY - press.lastY);
    press.lastX = event.clientX;
    press.lastY = event.clientY;
  };
  const onPointerUp = (event: PointerEvent) => {
    if (!press) return;
    const isClick = Math.hypot(event.clientX - press.x, event.clientY - press.y) < CLICK_SLOP_PX;
    press = null;
    if (isClick && mode === "pick") standAt(event.clientX, event.clientY);
  };
  canvas.addEventListener("pointerdown", onPointerDown);
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);

  // * ====================================================================
  // * === walking at eye height
  // * ====================================================================

  const pickStandpoint = () => setMode("pick");

  function standAt(clientX: number, clientY: number) {
    const hit = landing(clientX, clientY);
    if (!hit) return;
    // Ground and roofs; a wall is no place to stand.
    const levelness = hit.face ? Math.abs(hit.face.normal.z) : 1;
    if (levelness < STANDABLE_NORMAL_Z) return;
    const [forward] = groundAxes();
    const eye = hit.point.clone().setZ(hit.point.z + options.eyeHeightM);
    setMode("walk");
    flyTo(eye, eye.clone().add(forward.multiplyScalar(WALK_LOOK_AHEAD_M)));
  }

  /** Turns the eye by a drag of screen pixels: the scene follows the pointer. */
  function look(rightPx: number, downPx: number) {
    const direction = controls.target.clone().sub(camera.position).normalize();
    const heading = Math.atan2(direction.x, direction.y) - rightPx * LOOK_RAD_PER_PIXEL;
    const pitch = THREE.MathUtils.clamp(Math.asin(direction.z) + downPx * LOOK_RAD_PER_PIXEL, -MAX_LOOK_PITCH, MAX_LOOK_PITCH);
    const ahead = new THREE.Vector3(Math.sin(heading) * Math.cos(pitch), Math.cos(heading) * Math.cos(pitch), Math.sin(pitch));
    controls.target.copy(camera.position).add(ahead.multiplyScalar(WALK_LOOK_AHEAD_M));
    controls.update();
    options.requestDraw();
  }

  function walkBy(forwardM: number, rightM: number) {
    const [forward, right] = groundAxes();
    const shift = forward.multiplyScalar(forwardM).add(right.multiplyScalar(rightM));
    placeAt(camera.position.clone().add(shift), controls.target.clone().add(shift));
  }

  const leave = () => {
    if (mode === "orbit") return false;
    if (mode === "pick") setMode("orbit");
    else fit();
    return true;
  };

  // * ====================================================================
  // * === keys
  // * ====================================================================

  const held = new Set<string>();
  let isMoving = false;
  const onKeyDown = (event: KeyboardEvent) => {
    if (isTyping(event) || event.metaKey || event.ctrlKey || event.altKey) return;
    if (!MOVES[event.code]) return;
    event.preventDefault();
    held.add(event.code);
    if (isMoving) return;
    isMoving = true;
    options.animate(moveFor);
  };
  const onKeyUp = (event: KeyboardEvent) => held.delete(event.code);
  const onBlur = () => held.clear();
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("keyup", onKeyUp);
  window.addEventListener("blur", onBlur);

  function moveFor(seconds: number): boolean {
    if (held.size === 0) {
      isMoving = false;
      return false;
    }
    let forward = 0;
    let right = 0;
    let turn = 0;
    for (const code of held) {
      const [f, r, t] = MOVES[code];
      forward += f;
      right += r;
      turn += t;
    }
    const isFast = held.has("ShiftLeft") || held.has("ShiftRight");
    const speed = mode === "walk" ? WALK_M_PER_SECOND : camera.position.distanceTo(controls.target) * KEY_MOVE_SHARE_PER_SECOND;
    const step = speed * seconds * (isFast ? FAST_FACTOR : 1);
    if (forward !== 0 || right !== 0) walkBy(forward * step, right * step);
    if (turn !== 0) turnBy(turn * KEY_TURN_DEG_PER_SECOND * seconds);
    return true;
  }

  // * ====================================================================
  // * === turning, and the ground's own directions
  // * ====================================================================

  /** Positive degrees turn the view to the left. */
  function turnBy(degrees: number) {
    const angle = THREE.MathUtils.degToRad(degrees);
    if (mode === "walk") {
      const ahead = controls.target.clone().sub(camera.position).applyAxisAngle(UP, angle);
      controls.target.copy(camera.position).add(ahead);
    } else {
      // Turning the camera clockwise round the target swings the view to the left.
      camera.position.sub(controls.target).applyAxisAngle(UP, -angle).add(controls.target);
    }
    controls.update();
    options.requestDraw();
  }

  /** Forward and right along the ground, as the camera faces. */
  function groundAxes(): [THREE.Vector3, THREE.Vector3] {
    const forward = controls.target.clone().sub(camera.position).setZ(0);
    if (forward.lengthSq() === 0) camera.getWorldDirection(forward).setZ(0);
    if (forward.lengthSq() === 0) forward.set(0, 1, 0);
    forward.normalize();
    const right = new THREE.Vector3(forward.y, -forward.x, 0);
    return [forward, right];
  }

  const dispose = () => {
    host.removeEventListener("wheel", onWheel, { capture: true });
    canvas.removeEventListener("gesturestart", onGestureStart);
    canvas.removeEventListener("gesturechange", onGestureChange);
    canvas.removeEventListener("pointerdown", onPointerDown);
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("keyup", onKeyUp);
    window.removeEventListener("blur", onBlur);
    controls.dispose();
  };

  return { fit, top, sheetAngle, pickStandpoint, leave, turn: (degrees) => turnBy(-degrees), dispose };
}

// What each key does: forward, right, turn left. Every key that moves, in one place.
const MOVES: Record<string, [number, number, number]> = {
  KeyW: [1, 0, 0],
  ArrowUp: [1, 0, 0],
  KeyS: [-1, 0, 0],
  ArrowDown: [-1, 0, 0],
  KeyA: [0, -1, 0],
  ArrowLeft: [0, -1, 0],
  KeyD: [0, 1, 0],
  ArrowRight: [0, 1, 0],
  KeyQ: [0, 0, 1],
  KeyE: [0, 0, -1],
  ShiftLeft: [0, 0, 0],
  ShiftRight: [0, 0, 0],
};

/** True while the user types into a field, where keys are not for moving. */
function isTyping(event: KeyboardEvent): boolean {
  const target = event.target;
  return target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement;
}
