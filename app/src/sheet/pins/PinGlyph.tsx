// The pin markers as small icons for the palette and the pin toolbar. They
// echo the shapes the core draws on the sheet (crates/core/src/render/pins.rs);
// the sheet itself always shows the core's own drawing.

import type { PinIcon } from "../../lib/types";

export function PinGlyph({ icon, size = 16 }: { icon: PinIcon; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
      {icon === "pin" && (
        <>
          <path d="M8 15 4.6 8.4A4 4 0 1 1 11.4 8.4Z" fill="currentColor" />
          <circle cx="8" cy="6.2" r="1.5" fill="var(--surface-raised)" />
        </>
      )}
      {icon === "star" && <path d="M8 1.8l1.8 4.2 4.4.3-3.4 2.9 1.1 4.4L8 11.2l-3.9 2.4 1.1-4.4-3.4-2.9 4.4-.3z" fill="currentColor" />}
      {icon === "flag" && (
        <>
          <path d="M4.5 14.5V2" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
          <path d="M4.5 2.2 12.5 5 4.5 7.8z" fill="currentColor" />
        </>
      )}
      {icon === "dot" && <circle cx="8" cy="8" r="3.4" fill="currentColor" />}
      {icon === "number" && (
        <>
          <circle cx="8" cy="8" r="6" fill="currentColor" />
          <text x="8" y="11" textAnchor="middle" fontSize="8.5" fontWeight="600" fill="var(--surface-raised)">
            1
          </text>
        </>
      )}
    </svg>
  );
}
