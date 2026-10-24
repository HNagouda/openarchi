import { useEffect, useState } from "react";
import { clamp, parseTyped } from "../lib/format";

type Props = {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  /** Called when a drag ends or a typed value is committed: the end of one undo step. */
  onSeal?: () => void;
  /** How the typed field shows the value: 0.35 -> "35". */
  display?: (value: number) => string;
  /** The inverse of `display`, from the typed number back to a value. */
  read?: (typed: number) => number;
  unit?: string;
  /** A log slider spreads 1:500 to 1:10000 evenly instead of crowding the small end. */
  isLogarithmic?: boolean;
  label?: string;
};

const SLIDER_STEPS = 1000;

/** A slider with a typeable number beside it. Typed values accept "0,5" and are clamped. */
export function NumberField({
  value,
  min,
  max,
  step,
  onChange,
  onSeal,
  display = (shown) => String(shown),
  read = (typed) => typed,
  unit,
  isLogarithmic = false,
  label,
}: Props) {
  const [draft, setDraft] = useState(display(value));
  const [isTyping, setTyping] = useState(false);

  useEffect(() => {
    if (!isTyping) setDraft(display(value));
  }, [value, isTyping, display]);

  const toPosition = (raw: number) =>
    isLogarithmic
      ? (Math.log(raw / min) / Math.log(max / min)) * SLIDER_STEPS
      : ((raw - min) / (max - min)) * SLIDER_STEPS;
  const fromPosition = (position: number) => {
    const fraction = position / SLIDER_STEPS;
    const raw = isLogarithmic ? min * Math.pow(max / min, fraction) : min + fraction * (max - min);
    return clamp(Math.round(raw / step) * step, min, max);
  };

  const commitTyped = () => {
    setTyping(false);
    const typed = parseTyped(draft);
    if (typed === null) {
      setDraft(display(value));
      return;
    }
    const next = clamp(Math.round(read(typed) / step) * step, min, max);
    onChange(next);
    onSeal?.();
    setDraft(display(next));
  };

  return (
    <div className="number-field">
      <input
        type="range"
        className="slider"
        aria-label={label}
        min={0}
        max={SLIDER_STEPS}
        value={toPosition(value)}
        onChange={(event) => onChange(fromPosition(Number(event.target.value)))}
        onPointerUp={() => onSeal?.()}
        onKeyUp={() => onSeal?.()}
        style={{ ["--fill" as string]: `${(toPosition(value) / SLIDER_STEPS) * 100}%` }}
      />
      <label className="number-box">
        <input
          className="number-input tabular"
          value={draft}
          aria-label={label}
          onFocus={(event) => {
            setTyping(true);
            event.target.select();
          }}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commitTyped}
          onKeyDown={(event) => {
            if (event.key === "Enter") (event.target as HTMLInputElement).blur();
            if (event.key === "Escape") {
              setDraft(display(value));
              setTyping(false);
              (event.target as HTMLInputElement).blur();
            }
          }}
        />
        {unit && <span className="number-unit">{unit}</span>}
      </label>
    </div>
  );
}
