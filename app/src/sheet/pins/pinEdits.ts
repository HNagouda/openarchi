// Changes to the project's pins. Every function returns a new list and leaves
// the old one alone, so each result can go straight onto the undo stack.

import type { Pin, PinIcon, PinSize } from "../../lib/types";

// The markers in the order the palette offers them, in one place; crates/core/src/pin.rs has the same list.
// The numbered marker is no longer offered, because a white number in a red
// disc reads as an error badge; pins that already are one still draw.
export const PIN_ICONS: { icon: PinIcon; label: string }[] = [
  { icon: "pin", label: "Pin" },
  { icon: "star", label: "Star" },
  { icon: "flag", label: "Flag" },
  { icon: "dot", label: "Dot" },
];

// The sizes in the order the pin bar offers them, smallest first, in one place;
// crates/core/src/pin.rs has the same list.
export const PIN_SIZES: { size: PinSize; label: string; name: string }[] = [
  { size: "small", label: "S", name: "Small" },
  { size: "medium", label: "M", name: "Medium" },
  { size: "large", label: "L", name: "Large" },
  { size: "extra_large", label: "XL", name: "Extra large" },
];

/** The next size up or down from `size`, staying at the ends. Older pins have no size and count as medium. */
export function steppedSize(size: PinSize | undefined, direction: 1 | -1): PinSize {
  const index = PIN_SIZES.findIndex((entry) => entry.size === (size ?? "medium"));
  const next = Math.max(0, Math.min(PIN_SIZES.length - 1, index + direction));
  return PIN_SIZES[next].size;
}

/** The first "pin-N" id no pin uses yet, as the core picks it. */
export function nextPinId(pins: Pin[]): string {
  let number = pins.length + 1;
  while (pins.some((pin) => pin.id === `pin-${number}`)) number += 1;
  return `pin-${number}`;
}

export function withPin(pins: Pin[], pin: Pin): Pin[] {
  return [...pins, pin];
}

export function withChangedPin(pins: Pin[], id: string, patch: Partial<Omit<Pin, "id">>): Pin[] {
  return pins.map((pin) => (pin.id === id ? { ...pin, ...patch } : pin));
}

export function withoutPin(pins: Pin[], id: string): Pin[] {
  return pins.filter((pin) => pin.id !== id);
}
