// The keys that move the 3D view: WASD or the arrows move, Q and E turn,
// Shift goes faster. Held keys move on every frame until let go, through the
// stage's animation loop, so nothing runs when no key is down.

// Shift multiplies every move by this.
const FAST_FACTOR = 4;

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

type Options = {
  animate: (tick: (seconds: number) => boolean) => void;
  /** Moves by the held keys' sum, each -1 to 1 times the fast factor, for `seconds`. */
  move: (forward: number, right: number, turn: number, seconds: number) => void;
};

/** Starts listening; returns the function that stops. */
export function listenToMoveKeys({ animate, move }: Options): () => void {
  const held = new Set<string>();
  let isMoving = false;

  const step = (seconds: number): boolean => {
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
    const factor = held.has("ShiftLeft") || held.has("ShiftRight") ? FAST_FACTOR : 1;
    move(forward * factor, right * factor, turn, seconds);
    return true;
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (isTyping(event) || event.metaKey || event.ctrlKey || event.altKey) return;
    if (!MOVES[event.code]) return;
    event.preventDefault();
    held.add(event.code);
    if (isMoving) return;
    isMoving = true;
    animate(step);
  };
  const onKeyUp = (event: KeyboardEvent) => held.delete(event.code);
  // A key let go while the window is in the background never sends its keyup.
  const onBlur = () => held.clear();
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("keyup", onKeyUp);
  window.addEventListener("blur", onBlur);

  return () => {
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("keyup", onKeyUp);
    window.removeEventListener("blur", onBlur);
  };
}

/** True while the user types into a field, where keys are not for moving. */
function isTyping(event: KeyboardEvent): boolean {
  const target = event.target;
  return target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement;
}
