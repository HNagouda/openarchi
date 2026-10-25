// What sits over the selected text on the sheet: an outline, the formatting
// toolbar above it, and a text field in its place while it is being edited.

import { useLayoutEffect, useRef, useState } from "react";
import type { Look } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { useLive, type LiveValue } from "../liveValue";
import { groupFor, readTexts, type SheetText } from "./sheetTexts";
import { TextToolbar } from "./TextToolbar";
import { isTextBox, positionOf, styleOf, textOf, withPosition, withStyle, withText, withoutBox } from "./textEdits";
import type { TextDrag } from "./useTextTool";

const MM_PER_PT = 25.4 / 72;
// Room between the text and its outline, and between the outline and the toolbar.
const OUTLINE_PAD_PX = 4;
const TOOLBAR_GAP_PX = 8;
const TOOLBAR_HEIGHT_PX = 36;
// The field is at least this wide, so a short word still has room to grow.
const MIN_EDITOR_WIDTH_PX = 120;

type Props = {
  view: PaperView;
  svg: string;
  look: Look;
  selectedId: string | null;
  editingId: string | null;
  /** The selected one's offset while it is dragged. */
  drag: LiveValue<TextDrag | null>;
  /** The diagram name, which an empty title shows instead. */
  diagramName: string;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
  onEditEnd: () => void;
  onDeleted: () => void;
};

export function TextOverlay({ view, svg, look, selectedId, editingId, drag, diagramName, onLook, onSeal, onEditEnd, onDeleted }: Props) {
  const [texts, setTexts] = useState<SheetText[]>([]);

  // Read the texts once the SVG is in the page, again whenever the canvas moves or zooms.
  useLayoutEffect(() => {
    setTexts(readTexts(view.getPaper()));
  }, [svg, view]);
  const dragged = useLive(drag);

  const text = texts.find((entry) => entry.id === selectedId);
  if (!text) return null;

  const px = view.pxPerMm;
  const offset = dragged?.id === text.id ? dragged : { dx: 0, dy: 0 };
  const outline = {
    left: (text.box.x + offset.dx) * px - OUTLINE_PAD_PX,
    top: (text.box.y + offset.dy) * px - OUTLINE_PAD_PX,
    width: text.box.width * px + 2 * OUTLINE_PAD_PX,
    height: text.box.height * px + 2 * OUTLINE_PAD_PX,
  };
  const isEditing = editingId === text.id;

  return (
    <>
      <div className={isEditing ? "text-outline is-editing" : "text-outline"} style={outline} />
      {isEditing ? (
        <TextEditor
          key={text.id}
          text={text}
          initial={textOf(look, text.id)}
          placeholder={text.id === "title" ? diagramName : ""}
          view={view}
          onCommit={(value) => {
            const isEmptyBox = isTextBox(text.id) && value.trim() === "";
            // Same key as the add, so a new box and its first words undo together.
            onLook(isEmptyBox ? withoutBox(look, text.id) : withText(look, text.id, value), `text:${text.id}`);
            onSeal();
            if (isEmptyBox) onDeleted();
            onEditEnd();
          }}
          onCancel={onEditEnd}
        />
      ) : (
        !dragged && (
          <div className="text-toolbar-slot" style={toolbarPlace(view, outline)}>
            <TextToolbar
              text={text}
              style={styleOf(look, text.id)}
              isMoved={positionOf(look, text.id) !== null}
              isBox={isTextBox(text.id)}
              onStyle={(patch, mergeKey) => onLook(withStyle(look, text.id, patch), mergeKey)}
              onSeal={onSeal}
              onBackToLayout={() => onLook(withPosition(look, text.id, null))}
              onDelete={() => {
                onLook(withoutBox(look, text.id));
                onDeleted();
              }}
            />
          </div>
        )
      )}
    </>
  );
}

/** Above the outline, or below it when the top of the canvas is in the way. */
function toolbarPlace(view: PaperView, outline: { left: number; top: number; height: number }) {
  const paper = view.getPaper();
  const canvasTop = paper?.parentElement?.getBoundingClientRect().top ?? 0;
  const paperTop = paper?.getBoundingClientRect().top ?? 0;
  const above = outline.top - TOOLBAR_GAP_PX - TOOLBAR_HEIGHT_PX;
  const fitsAbove = paperTop + above >= canvasTop + TOOLBAR_GAP_PX;
  return { left: outline.left, top: fitsAbove ? above : outline.top + outline.height + TOOLBAR_GAP_PX };
}

type EditorProps = {
  text: SheetText;
  initial: string;
  placeholder: string;
  view: PaperView;
  onCommit: (value: string) => void;
  onCancel: () => void;
};

/** A field set in the text's own type, in its place. Return applies, Shift-Return adds a line, Escape cancels. */
function TextEditor({ text, initial, placeholder, view, onCommit, onCancel }: EditorProps) {
  const [draft, setDraft] = useState(initial);
  const field = useRef<HTMLTextAreaElement>(null);
  const isDone = useRef(false);
  // The paper as it was when editing began; the canvas re-rendering must not restart the edit.
  const paper = useRef(view.getPaper());

  // Hide the drawn text while its field covers it, and select the words to type over them.
  useLayoutEffect(() => {
    const group = groupFor(paper.current, text.id);
    group?.style.setProperty("visibility", "hidden");
    field.current?.focus();
    field.current?.select();
    return () => {
      group?.style.removeProperty("visibility");
    };
  }, [text.id]);

  const finish = (value: string | null) => {
    if (isDone.current) return;
    isDone.current = true;
    if (value === null || value === initial) onCancel();
    else onCommit(value);
  };

  const px = view.pxPerMm;
  const fontSize = text.sizePt * MM_PER_PT * px;
  const lines = Math.max(1, draft.split("\n").length);
  const width = Math.max(MIN_EDITOR_WIDTH_PX, text.box.width * px + fontSize * 2);
  const left = { left: text.x * px, centre: text.x * px - width / 2, right: text.x * px - width }[text.align];

  return (
    <textarea
      ref={field}
      className="text-editor"
      value={draft}
      placeholder={placeholder}
      rows={lines}
      spellCheck={false}
      style={{
        left,
        top: text.box.y * px - OUTLINE_PAD_PX,
        width,
        fontFamily: `"${text.family}"`,
        fontSize,
        fontWeight: text.isBold ? 600 : 400,
        fontStyle: text.isItalic ? "italic" : "normal",
        textAlign: text.align === "centre" ? "center" : text.align,
        color: text.colour,
      }}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => finish(draft)}
      onPointerDown={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          finish(draft);
        }
        if (event.key === "Escape") finish(null);
      }}
    />
  );
}
