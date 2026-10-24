// The app's icon set: 16 px, 1.5 px strokes in the current text colour.

const PATHS: Record<string, string> = {
  back: "M10 3.5 5.5 8l4.5 4.5",
  chevronRight: "M6 3.5 10.5 8 6 12.5",
  chevronDown: "M3.5 6 8 10.5 12.5 6",
  chevronUpDown: "M5 6 8 3l3 3M5 10l3 3 3-3",
  plus: "M8 3v10M3 8h10",
  close: "M4 4l8 8M12 4l-8 8",
  check: "M3.5 8.5l3 3 6-7",
  search: "M7 12A5 5 0 1 0 7 2a5 5 0 0 0 0 10zM10.6 10.6 14 14",
  circle: "M8 13.5A5.5 5.5 0 1 0 8 2.5a5.5 5.5 0 0 0 0 11z",
  rectangle: "M2.5 4h11v8h-11z",
  polygon: "M8 2.5l5.5 4-2 7h-7l-2-7z",
  square: "M3 3h10v10H3z",
  area: "M3 5.5 7.5 2.5 13 4.5 12 11.5 6 13.5 2.5 10z",
  eye: "M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8zM8 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
  eyeOff: "M2.5 2.5l11 11M6.6 4C7 3.7 7.5 3.5 8 3.5c4 0 6.5 4.5 6.5 4.5a12 12 0 0 1-1.8 2.3M4.3 5.3A12 12 0 0 0 1.5 8S4 12.5 8 12.5c1 0 2-.3 2.8-.8",
  undo: "M5.5 4 2.5 7l3 3M2.5 7H10a3.5 3.5 0 0 1 0 7H8",
  redo: "M10.5 4l3 3-3 3M13.5 7H6a3.5 3.5 0 0 0 0 7h2",
  export: "M8 10V2.5M5 5.5l3-3 3 3M3 9v3.5h10V9",
  sidebar: "M2.5 3h11v10h-11zM10 3v10",
  settings: "M8 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM8 1.8v1.7M8 12.5v1.7M14.2 8h-1.7M3.5 8H1.8M12.4 3.6l-1.2 1.2M4.8 11.2l-1.2 1.2M12.4 12.4l-1.2-1.2M4.8 4.8 3.6 3.6",
  help: "M8 14.5a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13zM6.2 6.2a1.9 1.9 0 0 1 3.7.6c0 1.2-1.9 1.6-1.9 2.7M8 11.4v.1",
  trash: "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 9h5.8l.6-9",
  restore: "M3 8a5 5 0 1 0 1.5-3.5M3 2.5v2.5h2.5",
  map: "M8 14s4.5-4.2 4.5-7.5a4.5 4.5 0 0 0-9 0C3.5 9.8 8 14 8 14zM8 8a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z",
  zoomIn: "M8 3v10M3 8h10",
  zoomOut: "M3 8h10",
  fit: "M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10",
  assistant: "M8 2.5l1.3 3.2L12.5 7l-3.2 1.3L8 11.5 6.7 8.3 3.5 7l3.2-1.3zM12.5 11l.6 1.4 1.4.6-1.4.6-.6 1.4-.6-1.4-1.4-.6 1.4-.6z",
  folder: "M2 4.5V12.5h12V5.5H8L6.5 4H2z",
  reset: "M3 8a5 5 0 1 0 1.5-3.5M3 2.5v2.5h2.5",
  more: "M4 8h.01M8 8h.01M12 8h.01",
  keyboard: "M1.5 4h13v8h-13zM4 6.5h.01M6.5 6.5h.01M9 6.5h.01M11.5 6.5h.01M5 9.5h6",
  text: "M3.5 4V3h9v1M8 3v10M6 13h4",
  alignLeft: "M2.5 4h11M2.5 7h7M2.5 10h11M2.5 13h7",
  alignCentre: "M2.5 4h11M4.5 7h7M2.5 10h11M4.5 13h7",
  alignRight: "M2.5 4h11M6.5 7h7M2.5 10h11M6.5 13h7",
};

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  const isDots = name === "more";
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={isDots ? 2.2 : 1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
