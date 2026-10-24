// A colour picker: a saturation and brightness square over a hue slider, hex
// and RGB fields, an optional opacity slider, the theme's palette and the
// colours used recently. Every part works from the keyboard: the square with
// the arrow keys, the sliders as native range inputs.

import { createContext, useEffect, useRef, useState } from "react";
import { hexToRgb, hsvToRgb, parseHex, readRecentColours, rgbToHex, rgbToHsv, type Hsv, type Rgb } from "./colour";

/** A colour the picker offers as one click: a theme role like "@accent", or a hex. */
export type Swatch = { value: string; colour: string; label: string };

/** The current theme's palette, provided by the sheet workspace for every picker inside it. */
export const PaletteSwatches = createContext<Swatch[]>([]);

const HUE_MAX = 360;
const PERCENT = 100;
const CHANNEL_MAX = 255;
// Arrow keys move the square by 1 %, with Shift by 10 %.
const KEY_STEP = 0.01;
const KEY_STEP_LARGE = 0.1;

type Props = {
  /** The colour shown, "#RRGGBB". */
  value: string;
  /** A hex, or a swatch's value when `acceptsRoles` and a swatch was clicked. */
  onChange: (value: string) => void;
  /** Theme roles can be kept as roles ("@accent"), so the colour follows theme changes. */
  acceptsRoles?: boolean;
  /** The swatch value currently chosen, to mark it. */
  selected?: string | null;
  swatches?: Swatch[];
  /** 0 to 1. The opacity slider shows only when both are given. */
  opacity?: number;
  onOpacity?: (opacity: number) => void;
};

export function ColorPicker({ value, onChange, acceptsRoles = false, selected = null, swatches = [], opacity, onOpacity }: Props) {
  const [hsv, setHsv] = useState<Hsv>(() => rgbToHsv(hexToRgb(value)));
  const [hexDraft, setHexDraft] = useState(value);
  const [recent] = useState(readRecentColours);
  const square = useRef<HTMLDivElement>(null);

  // Follow outside changes, but keep the hue the user chose when grey makes it ambiguous.
  useEffect(() => {
    setHexDraft(value);
    setHsv((current) => (rgbToHex(hsvToRgb(current)) === value ? current : rgbToHsv(hexToRgb(value))));
  }, [value]);

  const choose = (next: Hsv) => {
    setHsv(next);
    onChange(rgbToHex(hsvToRgb(next)));
  };

  const chooseRgb = (rgb: Rgb) => onChange(rgbToHex(rgb));

  const pickInSquare = (clientX: number, clientY: number) => {
    const rect = square.current?.getBoundingClientRect();
    if (!rect) return;
    const s = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    const v = 1 - Math.min(1, Math.max(0, (clientY - rect.top) / rect.height));
    choose({ ...hsv, s, v });
  };

  const startSquareDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    event.currentTarget.focus();
    pickInSquare(event.clientX, event.clientY);
  };

  const moveInSquare = (event: React.KeyboardEvent) => {
    const step = event.shiftKey ? KEY_STEP_LARGE : KEY_STEP;
    const clamp = (number: number) => Math.min(1, Math.max(0, number));
    const moves: Record<string, Partial<Hsv>> = {
      ArrowLeft: { s: clamp(hsv.s - step) },
      ArrowRight: { s: clamp(hsv.s + step) },
      ArrowUp: { v: clamp(hsv.v + step) },
      ArrowDown: { v: clamp(hsv.v - step) },
    };
    const move = moves[event.key];
    if (!move) return;
    event.preventDefault();
    choose({ ...hsv, ...move });
  };

  const commitHex = () => {
    const parsed = parseHex(hexDraft);
    if (parsed) onChange(parsed);
    else setHexDraft(value);
  };

  const rgb = hexToRgb(value);
  const hueColour = rgbToHex(hsvToRgb({ h: hsv.h, s: 1, v: 1 }));
  const hasOpacity = opacity !== undefined && onOpacity !== undefined;

  const squareView = (
    <div
      ref={square}
      className="color-square"
      style={{ backgroundColor: hueColour }}
      tabIndex={0}
      role="slider"
      aria-label="Saturation and brightness"
      aria-valuetext={`Saturation ${Math.round(hsv.s * PERCENT)}%, brightness ${Math.round(hsv.v * PERCENT)}%`}
      aria-valuenow={Math.round(hsv.s * PERCENT)}
      onPointerDown={startSquareDrag}
      onPointerMove={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) pickInSquare(event.clientX, event.clientY);
      }}
      onKeyDown={moveInSquare}
    >
      <span className="color-square-thumb" style={{ left: `${hsv.s * PERCENT}%`, top: `${(1 - hsv.v) * PERCENT}%`, background: value }} />
    </div>
  );

  return (
    <div className="color-picker">
      {squareView}
      <input
        className="color-range color-hue"
        type="range"
        min={0}
        max={HUE_MAX}
        value={Math.round(hsv.h)}
        aria-label="Hue"
        onChange={(event) => choose({ ...hsv, h: Number(event.target.value) })}
      />
      {hasOpacity && (
        <input
          className="color-range color-opacity"
          type="range"
          min={0}
          max={PERCENT}
          value={Math.round(opacity * PERCENT)}
          aria-label="Opacity"
          style={{ ["--range-colour" as string]: value }}
          onChange={(event) => onOpacity(Number(event.target.value) / PERCENT)}
        />
      )}

      <div className="color-fields">
        <label className="color-field-cell color-hex">
          <input
            className="tabular"
            value={hexDraft}
            spellCheck={false}
            aria-label="Hex"
            onChange={(event) => setHexDraft(event.target.value)}
            onBlur={commitHex}
            onKeyDown={(event) => {
              if (event.key === "Enter") commitHex();
            }}
          />
          <span>Hex</span>
        </label>
        <ChannelField label="R" value={rgb.r} onChange={(r) => chooseRgb({ ...rgb, r })} />
        <ChannelField label="G" value={rgb.g} onChange={(g) => chooseRgb({ ...rgb, g })} />
        <ChannelField label="B" value={rgb.b} onChange={(b) => chooseRgb({ ...rgb, b })} />
        {hasOpacity && (
          <ChannelField label="%" max={PERCENT} value={Math.round(opacity * PERCENT)} onChange={(percent) => onOpacity(percent / PERCENT)} />
        )}
      </div>

      {swatches.length > 0 && (
        <SwatchRow
          title="Palette"
          swatches={swatches}
          selected={selected}
          onPick={(swatch) => onChange(acceptsRoles ? swatch.value : swatch.colour)}
        />
      )}
      {recent.length > 0 && (
        <SwatchRow
          title="Recent"
          swatches={recent.map((hex) => ({ value: hex, colour: hex, label: hex }))}
          selected={selected}
          onPick={(swatch) => onChange(swatch.colour)}
        />
      )}
    </div>
  );
}

/** A 0 to 255 channel (or a percentage) that commits on Return or blur. */
function ChannelField({ label, value, max = CHANNEL_MAX, onChange }: { label: string; value: number; max?: number; onChange: (value: number) => void }) {
  const [draft, setDraft] = useState(String(value));

  useEffect(() => setDraft(String(value)), [value]);

  const commit = () => {
    const typed = Number(draft);
    if (draft.trim() === "" || Number.isNaN(typed)) setDraft(String(value));
    else onChange(Math.round(Math.min(max, Math.max(0, typed))));
  };

  return (
    <label className="color-field-cell">
      <input
        className="tabular"
        inputMode="numeric"
        value={draft}
        aria-label={label === "%" ? "Opacity percent" : label}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit();
        }}
      />
      <span>{label}</span>
    </label>
  );
}

function SwatchRow({ title, swatches, selected, onPick }: { title: string; swatches: Swatch[]; selected: string | null; onPick: (swatch: Swatch) => void }) {
  return (
    <div className="color-swatches" role="group" aria-label={title}>
      <span className="color-swatches-title">{title}</span>
      <div className="color-swatches-row">
        {swatches.map((swatch) => (
          <button
            key={swatch.value}
            type="button"
            className={swatch.value === selected ? "color-swatch is-selected" : "color-swatch"}
            style={{ background: swatch.colour }}
            data-tip={swatch.label}
            aria-label={swatch.label}
            aria-pressed={swatch.value === selected}
            onClick={() => onPick(swatch)}
          />
        ))}
      </div>
    </div>
  );
}
