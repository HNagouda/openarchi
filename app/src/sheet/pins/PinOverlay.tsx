// What sits over the selected pin: an outline, a small bar to change its
// marker or delete it, and a field for its label while it is being edited.

import { useLayoutEffect, useRef, useState } from "react";
import type { Pin } from "../../lib/types";
import { Button } from "../../ui/Button";
import type { PaperView } from "../Canvas";
import { useLive, type LiveValue } from "../liveValue";
import { PinGlyph } from "./PinGlyph";
import { PIN_ICONS, PIN_SIZES, withChangedPin, withoutPin } from "./pinEdits";
import { readPins, type SheetPin } from "./sheetPins";
import type { PinDrag } from "./usePinTool";

// Room between the pin and its outline, and between the outline and the bar.
const OUTLINE_PAD_PX = 4;
const TOOLBAR_GAP_PX = 8;
const TOOLBAR_HEIGHT_PX = 36;
const LABEL_FIELD_WIDTH_PX = 180;
// The same limit the core enforces.
const MAX_LABEL_CHARS = 80;

type Props = {
  view: PaperView;
  svg: string;
  pins: Pin[];
  selectedId: string | null;
  editingId: string | null;
  /** The selected one's offset while it is dragged. */
  drag: LiveValue<PinDrag | null>;
  onPins: (next: Pin[], mergeKey?: string) => void;
  onSeal: () => void;
  onEdit: () => void;
  onEditEnd: () => void;
  onDeleted: () => void;
};

export function PinOverlay({ view, svg, pins, selectedId, editingId, drag, onPins, onSeal, onEdit, onEditEnd, onDeleted }: Props) {
  const [drawn, setDrawn] = useState<SheetPin[]>([]);

  // Read the pins once the SVG is in the page, again whenever the canvas moves or zooms.
  useLayoutEffect(() => {
    setDrawn(readPins(view.getPaper()));
  }, [svg, view]);
  const dragged = useLive(drag);

  const shown = drawn.find((entry) => entry.id === selectedId);
  const pin = pins.find((entry) => entry.id === selectedId);
  if (!shown || !pin) return null;

  const px = view.pxPerMm;
  const offset = dragged?.id === shown.id ? dragged : { dx: 0, dy: 0 };
  const outline = {
    left: (shown.box.x + offset.dx) * px - OUTLINE_PAD_PX,
    top: (shown.box.y + offset.dy) * px - OUTLINE_PAD_PX,
    width: shown.box.width * px + 2 * OUTLINE_PAD_PX,
    height: shown.box.height * px + 2 * OUTLINE_PAD_PX,
  };
  const isEditing = editingId === shown.id;

  return (
    <>
      <div className="pin-outline" style={outline} />
      {isEditing && (
        <LabelField
          key={shown.id}
          initial={pin.label}
          style={{ left: outline.left, top: outline.top + outline.height + TOOLBAR_GAP_PX, width: LABEL_FIELD_WIDTH_PX }}
          onCommit={(label) => {
            // Same key as the placing, so a new pin and its label undo together.
            onPins(withChangedPin(pins, pin.id, { label }), `pin:${pin.id}`);
            onSeal();
            onEditEnd();
          }}
          onCancel={() => {
            // Close the placing step, so a later label edit undoes on its own.
            onSeal();
            onEditEnd();
          }}
        />
      )}
      {!isEditing && !dragged && (
        <div className="pin-toolbar-slot" style={{ left: outline.left, top: Math.max(0, outline.top - TOOLBAR_GAP_PX - TOOLBAR_HEIGHT_PX) }}>
          <div className="pin-toolbar" role="toolbar" aria-label="Pin" onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}>
            {PIN_ICONS.map(({ icon, label }) => (
              <button
                key={icon}
                type="button"
                className={icon === pin.icon ? "pin-icon-choice is-active" : "pin-icon-choice"}
                aria-label={label}
                data-tip={label}
                onClick={() => onPins(withChangedPin(pins, pin.id, { icon }))}
              >
                <PinGlyph icon={icon} />
              </button>
            ))}
            <span className="toolbar-divider" />
            {PIN_SIZES.map(({ size, label, name }) => (
              <button
                key={size}
                type="button"
                className={size === (pin.size ?? "medium") ? "pin-size-choice is-active" : "pin-size-choice"}
                aria-label={name}
                data-tip={name}
                // The smallest and largest name the keys that step through the sizes.
                data-tip-key={size === "small" ? "−" : size === "extra_large" ? "+" : undefined}
                onClick={() => onPins(withChangedPin(pins, pin.id, { size }))}
              >
                {label}
              </button>
            ))}
            <span className="toolbar-divider" />
            <Button icon="text" tip="Edit the label" shortcut="enter" onClick={onEdit} />
            <Button
              icon="trash"
              tip="Delete the pin"
              shortcut="delete"
              onClick={() => {
                onPins(withoutPin(pins, pin.id));
                onDeleted();
              }}
            />
          </div>
        </div>
      )}
    </>
  );
}

type FieldProps = {
  initial: string;
  style: React.CSSProperties;
  onCommit: (label: string) => void;
  onCancel: () => void;
};

/** The label as a field under the pin. Return applies, Escape cancels, an empty label leaves the marker alone. */
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
      className="pin-label-field"
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
