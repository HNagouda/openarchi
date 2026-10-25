// What pins and highlights both float over the sheet: the label field in
// place of the mark's label while it is edited. The toolbar above a selected
// mark shares its styles, `.paper-toolbar` in overlay-bar.css.

import { useLayoutEffect, useRef, useState } from "react";
import "./overlay-bar.css";

// The same limit the core enforces for pins and highlights (crates/core/src/pin.rs).
const MAX_LABEL_CHARS = 80;

type FieldProps = {
  initial: string;
  style: React.CSSProperties;
  onCommit: (label: string) => void;
  onCancel: () => void;
};

/** The label as a field under the mark. Return applies, Escape cancels, an unchanged label leaves the mark alone. */
export function LabelField({ initial, style, onCommit, onCancel }: FieldProps) {
  const [draft, setDraft] = useState(initial);
  const field = useRef<HTMLInputElement>(null);
  const isDone = useRef(false);

  useLayoutEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);

  const finish = (value: string | null) => {
    if (isDone.current) return;
    isDone.current = true;
    if (value === null || value.trim() === initial) onCancel();
    else onCommit(value.trim());
  };

  return (
    <input
      ref={field}
      className="paper-label-field"
      value={draft}
      placeholder="Label"
      maxLength={MAX_LABEL_CHARS}
      spellCheck={false}
      style={style}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => finish(draft)}
      onPointerDown={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") finish(draft);
        if (event.key === "Escape") finish(null);
      }}
    />
  );
}
