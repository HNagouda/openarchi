import type { ReactNode } from "react";

type Option<T> = { value: T; label: ReactNode; tip?: string; isDisabled?: boolean };

/** A row of mutually exclusive choices, like a macOS segmented control. */
export function Segmented<T extends string | number | boolean>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: Option<T>[];
  onChange: (value: T) => void;
  label?: string;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          className={option.value === value ? "segment is-selected" : "segment"}
          disabled={option.isDisabled}
          data-tip={option.tip}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
