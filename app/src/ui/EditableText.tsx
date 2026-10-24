import { useEffect, useRef, useState } from "react";

type Props = {
  value: string;
  placeholder?: string;
  onCommit: (value: string) => void;
  /** Opens straight into editing, for a freshly created item. */
  startEditing?: boolean;
  className?: string;
  /** Some names may not be empty; an empty commit then restores the old value. */
  allowEmpty?: boolean;
};

/** Text that turns into a field on click. Return or a click elsewhere commits, Escape cancels. */
export function EditableText({ value, placeholder, onCommit, startEditing = false, className = "", allowEmpty = false }: Props) {
  const [isEditing, setEditing] = useState(startEditing);
  const [draft, setDraft] = useState(value);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!isEditing) setDraft(value);
  }, [value, isEditing]);

  useEffect(() => {
    if (isEditing) input.current?.select();
  }, [isEditing]);

  const commit = () => {
    setEditing(false);
    const trimmed = draft.trim();
    if (trimmed === value) return;
    if (trimmed === "" && !allowEmpty) return;
    onCommit(trimmed);
  };

  if (isEditing) {
    return (
      <input
        ref={input}
        className={`editable editable-input ${className}`}
        value={draft}
        placeholder={placeholder}
        size={Math.max(4, draft.length + 1)}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit();
          if (event.key === "Escape") {
            setDraft(value);
            setEditing(false);
          }
          event.stopPropagation();
        }}
      />
    );
  }

  return (
    <span
      className={`editable ${value ? "" : "is-placeholder"} ${className}`}
      onClick={(event) => {
        event.stopPropagation();
        setEditing(true);
      }}
      onDoubleClick={(event) => event.stopPropagation()}
      title="Click to edit"
    >
      {value || placeholder}
    </span>
  );
}
