import { useEffect, useRef, useState } from "react";
import { clamp, parseTyped, snapToStep } from "../lib/format";
import { PERCENT } from "../lib/units";

type BoxProps = {
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
  /** Shown after the number: "mm", "pt", "%". */
  unit?: string;
  /** Shown before the number, for a row of short fields like X, Y, W and H. */
  prefix?: string;
  label?: string;
  tip?: string;
  className?: string;
};

type FieldProps = Omit<BoxProps, "prefix" | "tip" | "className"> & {
  /** A log slider spreads 1:500 to 1:10000 evenly instead of crowding the small end. */
  isLogarithmic?: boolean;
};

const SLIDER_STEPS = 1000;

/** A slider with a typeable number beside it. */
export function NumberField({ isLogarithmic = false, ...box }: FieldProps) {
  const { value, min, max, step, onChange, onSeal, label } = box;

  const toPosition = (raw: number) =>
    isLogarithmic
      ? (Math.log(raw / min) / Math.log(max / min)) * SLIDER_STEPS
      : ((raw - min) / (max - min)) * SLIDER_STEPS;
  const fromPosition = (position: number) => {
    const fraction = position / SLIDER_STEPS;
    const raw = isLogarithmic ? min * Math.pow(max / min, fraction) : min + fraction * (max - min);
    return clamp(snapToStep(raw, step), min, max);
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
        style={{ ["--fill" as string]: `${(toPosition(value) / SLIDER_STEPS) * PERCENT}%` }}
      />
      <NumberBox {...box} />
    </div>
  );
}

/** A typeable number on its own. Typed values accept "0,5", are clamped and snap to the step; Return or leaving the field applies them, Escape puts the old one back. */
export function NumberBox({
  value,
  min,
  max,
  step,
  onChange,
  onSeal,
  display = (shown) => String(shown),
  read = (typed) => typed,
  unit,
  prefix,
  label,
  tip,
  className = "",
}: BoxProps) {
  const [draft, setDraft] = useState(display(value));
  const [isTyping, setTyping] = useState(false);
  // Escape blurs the field too, and the blur must not then apply what was typed.
  const isCancelled = useRef(false);

  // Only a new value resets the field. Right after a commit the typed number
  // stays until the owner's value catches up, which for a text size takes a redraw.
  useEffect(() => {
    if (!isTyping) setDraft(display(value));
  }, [value]);

  const commitTyped = () => {
    setTyping(false);
    const typed = isCancelled.current ? null : parseTyped(draft);
    isCancelled.current = false;
    if (typed === null) {
      setDraft(display(value));
      return;
    }
    const next = clamp(snapToStep(read(typed), step), min, max);
    // An unchanged value is not an edit, so it leaves no undo step behind.
    if (next !== value) onChange(next);
    onSeal?.();
    setDraft(display(next));
  };

  return (
    <label className={`number-box ${className}`.trim()} data-tip={tip}>
      {prefix && <span className="number-unit">{prefix}</span>}
      <input
        className="number-input tabular"
        inputMode="decimal"
        value={draft}
        aria-label={label}
        onFocus={(event) => {
          setTyping(true);
          event.target.select();
        }}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commitTyped}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") {
            isCancelled.current = true;
            event.currentTarget.blur();
          }
        }}
      />
      {unit && <span className="number-unit">{unit}</span>}
    </label>
  );
}
