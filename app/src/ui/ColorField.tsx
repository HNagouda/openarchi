import { useEffect, useState } from "react";

const HEX_PATTERN = /^#?([0-9a-f]{6})$/i;

type Props = {
  /** The colour shown, "#RRGGBB"; null shows an empty, struck-through swatch. */
  value: string | null;
  /** True when the user overrode the theme here; the reset button appears. */
  isOverridden: boolean;
  onChange: (hex: string) => void;
  onReset: () => void;
  label: string;
};

/** A swatch that opens the system colour picker, plus a typeable hex value. */
export function ColorField({ value, isOverridden, onChange, onReset, label }: Props) {
  const [draft, setDraft] = useState(value ?? "");

  useEffect(() => setDraft(value ?? ""), [value]);

  const commit = () => {
    const match = HEX_PATTERN.exec(draft.trim());
    if (match) onChange(`#${match[1].toUpperCase()}`);
    else setDraft(value ?? "");
  };

  return (
    <div className="color-field">
      <span className="field-label">{label}</span>
      <label className={value ? "swatch" : "swatch is-empty"} style={{ background: value ?? undefined }}>
        <input
          type="color"
          value={value ?? "#000000"}
          aria-label={`${label} colour`}
          onChange={(event) => onChange(event.target.value.toUpperCase())}
        />
      </label>
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
