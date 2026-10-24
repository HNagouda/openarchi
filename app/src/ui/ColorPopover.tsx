// A floating panel anchored to a swatch: below it when there is room, above
// it otherwise, always inside the window. A click outside or Escape closes it,
// and focus goes back to the swatch.

import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

// Space between the swatch and the panel, and kept free at the window edges.
const GAP_PX = 6;
const EDGE_PX = 8;

type Props = {
  anchor: HTMLElement | null;
  label: string;
  onClose: () => void;
  children: ReactNode;
};

export function ColorPopover({ anchor, label, onClose, children }: Props) {
  const panel = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState({ left: -9999, top: -9999 });
  const latestClose = useRef(onClose);
  latestClose.current = onClose;

  useLayoutEffect(() => {
    const element = panel.current;
    if (!anchor || !element) return;
    const target = anchor.getBoundingClientRect();
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    const below = target.bottom + GAP_PX;
    const top = below + height <= window.innerHeight - EDGE_PX ? below : Math.max(EDGE_PX, target.top - GAP_PX - height);
    const left = Math.min(Math.max(EDGE_PX, target.left), window.innerWidth - width - EDGE_PX);
    setPlace({ left, top });
  }, [anchor]);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    panel.current?.querySelector<HTMLElement>("[tabindex], input, button")?.focus();

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (panel.current?.contains(target) || anchor?.contains(target)) return;
      latestClose.current();
    };
    // Capture, so Escape closes the picker without also clearing a selection behind it.
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      latestClose.current();
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
      previous?.focus();
    };
  }, [anchor]);

  return createPortal(
    // data-modal-open keeps single-key shortcuts (1 to 9, [ and ]) from firing while the picker has focus.
    <div ref={panel} className="color-popover" role="dialog" aria-label={label} data-modal-open style={place}>
      {children}
    </div>,
    document.body,
  );
}
