// The library on the left of the report editor: every block the project can
// draw, under Ratios, Analysis and Sheets, each with a small picture of
// itself. Drag one onto a page, or click it (or press Return on it) to add it
// to the middle of the current page.

import { useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from "react";
import { DRAG_THRESHOLD_PX } from "../lib/pointerDrag";
import { Icon } from "../ui/Icon";
import type { LibraryGroup, LibraryItem } from "./library";
import "./library.css";

// Every group, in the order the panel lists them.
const GROUPS: LibraryGroup[] = ["Ratios", "Analysis", "Sheets"];

type Drag = { item: LibraryItem; startX: number; startY: number; x: number; y: number; isMoving: boolean };

/** Where a drag from the library is on screen, so the editor can show where the block would land. */
export type LibraryDrag = { item: LibraryItem; clientX: number; clientY: number };

type Props = {
  items: LibraryItem[];
  /** Item key -> the block as an SVG document, drawn by the core. */
  pictures: Record<string, string>;
  /** Item key -> a sheet thumbnail as an image URL. */
  thumbnails: Record<string, string>;
  /** The palette's paper, behind every picture, so a picture's own margins disappear into it. */
  paper: string;
  isLoading: boolean;
  problems: string[];
  onAdd: (item: LibraryItem) => void;
  /** Dropped at a point on screen; the editor finds the page under it. */
  onDrop: (item: LibraryItem, clientX: number, clientY: number) => void;
  /** Every move of a drag, and null when it ends. */
  onDragOver: (drag: LibraryDrag | null) => void;
};

export function LibraryPanel({ items, pictures, thumbnails, paper, isLoading, problems, onAdd, onDrop, onDragOver }: Props) {
  const [filter, setFilter] = useState("");
  const [drag, setDrag] = useState<Drag | null>(null);
  const words = filter.trim().toLowerCase();
  const shown = items.filter((item) => words === "" || `${item.label} ${item.detail}`.toLowerCase().includes(words));

  const start = (item: LibraryItem, event: ReactPointerEvent) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ item, startX: event.clientX, startY: event.clientY, x: event.clientX, y: event.clientY, isMoving: false });
  };

  const move = (event: ReactPointerEvent) => {
    if (!drag) return;
    const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
    const isMoving = drag.isMoving || distance > DRAG_THRESHOLD_PX;
    setDrag({ ...drag, x: event.clientX, y: event.clientY, isMoving });
    if (isMoving) onDragOver({ item: drag.item, clientX: event.clientX, clientY: event.clientY });
  };

  const end = (event: ReactPointerEvent) => {
    if (!drag) return;
    if (drag.isMoving) onDrop(drag.item, event.clientX, event.clientY);
    else onAdd(drag.item);
    setDrag(null);
    onDragOver(null);
  };

  const cancel = () => {
    setDrag(null);
    onDragOver(null);
  };

  return (
    <nav className="report-library" aria-label="Blocks" style={{ "--library-paper": paper } as CSSProperties}>
      <header className="report-panel-header">
        <span className="report-panel-title">Blocks</span>
        {isLoading && <span className="report-library-loading" aria-label="Loading" />}
      </header>
      <div className="report-library-search">
        <Icon name="search" size={13} />
        <input value={filter} placeholder="Filter" aria-label="Filter blocks" onChange={(event) => setFilter(event.target.value)} />
      </div>
      <div className="report-library-list">
        {GROUPS.map((group) => {
          const entries = shown.filter((item) => item.group === group);
          if (entries.length === 0) return null;
          return (
            <section key={group} aria-label={group}>
              <h3 className="diagram-group-name">{group}</h3>
              {entries.map((item) => (
                <button
                  key={item.key}
                  type="button"
                  className={drag?.item.key === item.key && drag.isMoving ? "library-item is-dragged" : "library-item"}
                  data-tip="Drag onto a page, or click to add"
                  onPointerDown={(event) => start(item, event)}
                  onPointerMove={move}
                  onPointerUp={end}
                  onPointerCancel={cancel}
                  onKeyDown={(event) => event.key === "Enter" && onAdd(item)}
                >
                  <ItemPicture svg={item.svg ?? pictures[item.key]} thumbnail={thumbnails[item.key]} group={item.group} />
                  <span className="library-label">
                    <span className="library-name">{item.label}</span>
                    <span className="library-detail">{item.detail}</span>
                  </span>
                </button>
              ))}
            </section>
          );
        })}
        {problems.map((problem) => (
          <p key={problem} className="field-note library-problem">
            {problem}
          </p>
        ))}
      </div>
      {drag?.isMoving && (
        <div className="library-ghost" style={{ left: drag.x, top: drag.y }} aria-hidden>
          {drag.item.label}
        </div>
      )}
    </nav>
  );
}

/** The block drawn by the core, the sheet's thumbnail, or a quiet glyph until either arrives. */
function ItemPicture({ svg, thumbnail, group }: { svg?: string; thumbnail?: string; group: LibraryGroup }) {
  if (svg) return <span className="library-thumb" dangerouslySetInnerHTML={{ __html: svg }} />;
  if (thumbnail) {
    return (
      <span className="library-thumb">
        <img src={thumbnail} alt="" draggable={false} />
      </span>
    );
  }
  const glyph = { Ratios: "▤", Analysis: "≡", Sheets: "◯" }[group];
  return <span className="library-thumb library-glyph">{glyph}</span>;
}
