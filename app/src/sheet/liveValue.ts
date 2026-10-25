// A value that changes on every pointer move or zoom step: a pin's offset
// while it is dragged, a section line being drawn, the zoom. It is kept out of
// the sheet workspace's React state, so a change renders only the overlay or
// readout that shows it, through `useLive`, and not the diagram column, the
// inspector and the tabs as well.

import { useState, useSyncExternalStore } from "react";

export type LiveValue<T> = {
  get: () => T;
  set: (next: T) => void;
  subscribe: (onChange: () => void) => () => void;
};

/** A live value that lasts as long as the component that made it. */
export function useLiveValue<T>(initial: T): LiveValue<T> {
  const [live] = useState(() => liveValue(initial));
  return live;
}

/** The current value, rendering again whenever it changes. */
export function useLive<T>(live: LiveValue<T>): T {
  return useSyncExternalStore(live.subscribe, live.get);
}

function liveValue<T>(initial: T): LiveValue<T> {
  let value = initial;
  const listeners = new Set<() => void>();
  return {
    get: () => value,
    set: (next) => {
      if (next === value) return;
      value = next;
      listeners.forEach((listener) => listener());
    },
    subscribe: (onChange) => {
      listeners.add(onChange);
      return () => listeners.delete(onChange);
    },
  };
}
