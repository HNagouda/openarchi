// A selected text block's formatting bar, floating above it as on the sheet,
// and the field that edits its words in place after a double-click.
//
// The bar shows the style as the core resolved it, read off the block's own
// SVG, so "palette" values read as what is actually printed.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReportBlock, TextStyle } from "../lib/types";
import { readTexts, type SheetText } from "../sheet/text/sheetTexts";
import { TextToolbar } from "../sheet/text/TextToolbar";
import "../sheet/text/text.css";

const MM_PER_PT = 25.4 / 72;
// Room between the block and the bar above it.
const TOOLBAR_GAP_PX = 8;
const TOOLBAR_HEIGHT_PX = 36;
// What the bar shows before the block's picture has arrived.
const FALLBACK_SIZE_PT = 10;

type TextBlock = ReportBlock & { content: { kind: "text"; text: string; style: TextStyle } };

type Props = {
  block: TextBlock;
  pxPerMm: number;
  isEditing: boolean;
  onStyle: (patch: TextStyle, mergeKey?: string) => void;
  onSeal: () => void;
  onText: (text: string) => void;
  onEditEnd: () => void;
  onDelete: () => void;
};

export function TextBlockTools({ block, pxPerMm, isEditing, onStyle, onSeal, onText, onEditEnd, onDelete }: Props) {
  const resolved = resolvedText(block);
  const left = block.x_mm * pxPerMm;
  const top = block.y_mm * pxPerMm;

  if (isEditing) {
    return <BlockEditor block={block} text={resolved} pxPerMm={pxPerMm} onCommit={onText} onDone={onEditEnd} />;
  }
  return (
    <div className="text-toolbar-slot" style={{ left, top: Math.max(0, top - TOOLBAR_GAP_PX - TOOLBAR_HEIGHT_PX) }}>
      <TextToolbar
        text={resolved}
        style={block.content.style}
        isMoved={false}
        isBox
        onStyle={(patch, mergeKey) => onStyle({ ...block.content.style, ...patch }, mergeKey)}
        onSeal={onSeal}
        onBackToLayout={() => undefined}
        onDelete={onDelete}
      />
    </div>
  );
}

/** The style the core drew the block in, from the data attributes on its picture. */
function resolvedText(block: TextBlock): SheetText {
  const element = document.querySelector(`[data-block="${CSS.escape(block.id)}"]`);
  const [drawn] = readTexts(element);
  if (drawn) return drawn;
  const style = block.content.style;
  return {
    id: block.id,
    x: block.x_mm,
    y: block.y_mm,
    box: { x: block.x_mm, y: block.y_mm, width: block.width_mm, height: block.height_mm },
    align: style.align ?? "left",
    family: style.family ?? "Inter",
    sizePt: style.size_pt ?? FALLBACK_SIZE_PT,
    isBold: style.bold ?? false,
    isItalic: style.italic ?? false,
    colour: "#1f1d1a",
    opacity: style.opacity ?? 1,
  };
}

type EditorProps = { block: TextBlock; text: SheetText; pxPerMm: number; onCommit: (text: string) => void; onDone: () => void };

/** The block's words in its own type, in its box. Return applies, Shift-Return adds a line, Escape cancels. */
function BlockEditor({ block, text, pxPerMm, onCommit, onDone }: EditorProps) {
  const [draft, setDraft] = useState(block.content.text);
  const field = useRef<HTMLTextAreaElement>(null);
  const isDone = useRef(false);

  useLayoutEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);

  // A click elsewhere on the page ends editing by removing the field, often
  // before its blur arrives, so the words are kept on the way out as well.
  const latest = useRef({ draft, onCommit });
  latest.current = { draft, onCommit };
  useEffect(() => {
    const original = block.content.text;
    return () => {
      if (isDone.current || latest.current.draft === original) return;
      isDone.current = true;
      latest.current.onCommit(latest.current.draft);
    };
    // Only on the way out: the original words are the ones editing began with.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const finish = (value: string | null) => {
    if (isDone.current) return;
    isDone.current = true;
    if (value !== null && value !== block.content.text) onCommit(value);
    onDone();
  };

  return (
    <textarea
      ref={field}
      className="text-editor report-text-editor"
      value={draft}
      spellCheck={false}
      style={{
        left: block.x_mm * pxPerMm,
        top: block.y_mm * pxPerMm,
        width: block.width_mm * pxPerMm,
        height: block.height_mm * pxPerMm,
        fontFamily: `"${text.family}"`,
        fontSize: text.sizePt * MM_PER_PT * pxPerMm,
        fontWeight: text.isBold ? 600 : 400,
        fontStyle: text.isItalic ? "italic" : "normal",
        textAlign: text.align === "centre" ? "center" : text.align,
        color: text.colour,
      }}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => finish(draft)}
      onPointerDown={(event) => event.stopPropagation()}
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
