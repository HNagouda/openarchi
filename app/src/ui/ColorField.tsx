import { useContext, useEffect, useState } from "react";
import { ColorPicker, PaletteSwatches } from "./ColorPicker";
import { ColorSwatch } from "./ColorSwatch";
import { parseHex } from "./colour";

// What the picker starts from when the field has no colour yet.
const EMPTY_START = "#000000";

type Props = {
  /** The colour shown, "#RRGGBB"; null shows an empty, struck-through swatch. */
  value: string | null;
  /** True when the user overrode the theme here; the reset button appears. */
  isOverridden: boolean;
  onChange: (hex: string) => void;
  onReset: () => void;
  /** Called when the picker closes: the end of one undo step. */
  onSeal?: () => void;
  label: string;
};

/** A swatch that opens the colour picker, plus a typeable hex value. */
export function ColorField({ value, isOverridden, onChange, onReset, onSeal, label }: Props) {
  const [draft, setDraft] = useState(value ?? "");
  const palette = useContext(PaletteSwatches);

  useEffect(() => setDraft(value ?? ""), [value]);

  const commit = () => {
    const parsed = parseHex(draft);
    if (parsed) onChange(parsed);
    else setDraft(value ?? "");
  };

  return (
    <div className="color-field">
      <span className="field-label">{label}</span>
      <ColorSwatch colour={value} label={`${label} colour`} onSeal={onSeal}>
        <ColorPicker value={value ?? EMPTY_START} swatches={palette} selected={value} onChange={onChange} />
      </ColorSwatch>
      <input
        className="hex-input tabular"
        value={draft}
        placeholder="None"
        spellCheck={false}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") (event.target as HTMLInputElement).blur();
          if (event.key === "Escape") {
            setDraft(value ?? "");
            (event.target as HTMLInputElement).blur();
          }
        }}
      />
      <button
        type="button"
        className={isOverridden ? "reset-button" : "reset-button is-hidden"}
        data-tip="Back to the theme's colour"
        data-tip-side="left"
        onClick={onReset}
        aria-label="Reset"
      >
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
          <path d="M3 8a5 5 0 1 0 1.5-3.5M3 2.5v2.5h2.5" />
        </svg>
      </button>
    </div>
  );
}
