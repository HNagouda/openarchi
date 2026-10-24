// Changes to the project's pins. Every function returns a new list and leaves
// the old one alone, so each result can go straight onto the undo stack.

import type { Pin, PinIcon } from "../../lib/types";

// The markers in the order the palette offers them, in one place; crates/core/src/pin.rs has the same list.
export const PIN_ICONS: { icon: PinIcon; label: string }[] = [
  { icon: "pin", label: "Pin" },
  { icon: "star", label: "Star" },
  { icon: "flag", label: "Flag" },
  { icon: "dot", label: "Dot" },
  { icon: "number", label: "Numbered marker" },
];

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
