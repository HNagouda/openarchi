// The tabs along the bottom of the workspace, one per sheet, the way a
// spreadsheet lists its sheets. Click to switch, double-click to rename, drag
// to reorder, right-click (or the dots on the open tab) for the rest.
//
// Tabs are sheets and nothing else: reports and the analysis have their own
// places, so every tab opens a sheet.

import { useRef, useState, type ReactNode } from "react";
import { Button } from "../../ui/Button";
import { Icon } from "../../ui/Icon";
import "./tabs.css";

export type Tab = { key: string; label: string };

// A press that moves less than this is a click, not a drag.
const DRAG_THRESHOLD_PX = 4;

type Props = {
  tabs: Tab[];
  activeIndex: number;
  onSelect: (index: number) => void;
  onAdd: () => void;
  onRename: (index: number, name: string) => void;
  onDuplicate: (index: number) => void;
  onDelete: (index: number) => void;
  onMove: (from: number, to: number) => void;
  /** Tools on the right of the bar. */
  children?: ReactNode;
};

type Drag = { from: number; to: number; dx: number };
type Menu = { index: number; x: number; y: number };

export function TabBar({ tabs, activeIndex, onSelect, onAdd, onRename, onDuplicate, onDelete, onMove, children }: Props) {
  const [renaming, setRenaming] = useState<number | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [menu, setMenu] = useState<Menu | null>(null);
  const elements = useRef<(HTMLDivElement | null)[]>([]);
  // The dragged tab moves with the pointer, so letting go clicks it too; that click must not select by its old place.
  const wasDragged = useRef(false);
  const canDelete = tabs.length > 1;

  /** The slot a tab dragged to `clientX` lands in: before the first tab whose middle lies right of it. */
  const slotAt = (clientX: number, from: number) => {
    let slot = 0;
    elements.current.forEach((element, index) => {
      if (!element || index === from) return;
      const rect = element.getBoundingClientRect();
      if (clientX > rect.left + rect.width / 2) slot = index < from ? index + 1 : index;
    });
    return slot;
  };

  const startDrag = (event: React.PointerEvent, index: number) => {
    if (event.button !== 0 || renaming !== null) return;
    wasDragged.current = false;
    const startX = event.clientX;
    let isDragging = false;
    const onPointerMove = (move: PointerEvent) => {
      const dx = move.clientX - startX;
      if (!isDragging && Math.abs(dx) < DRAG_THRESHOLD_PX) return;
      isDragging = true;
      setDrag({ from: index, to: slotAt(move.clientX, index), dx });
    };
    const onPointerUp = (up: PointerEvent) => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      setDrag(null);
      if (!isDragging) return;
      wasDragged.current = true;
      onMove(index, slotAt(up.clientX, index));
    };
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  };

  const onTabKey = (event: React.KeyboardEvent, index: number) => {
    // The workspace's own arrow and Delete keys must not also fire.
    event.stopPropagation();
    if (event.key === "Enter" || event.key === "F2") {
      event.preventDefault();
      setRenaming(index);
    }
    if ((event.key === "Delete" || event.key === "Backspace") && canDelete && !event.repeat) onDelete(index);
    const step = { ArrowLeft: -1, ArrowRight: 1 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const target = index + step;
    if (target < 0 || target >= tabs.length) return;
    // Alt with an arrow moves the tab; an arrow alone moves the focus.
    if (event.altKey) onMove(index, target);
    else elements.current[target]?.querySelector<HTMLButtonElement>(".sheet-tab-button")?.focus();
  };

  const openMenu = (event: React.MouseEvent, index: number) => {
    event.preventDefault();
    const rect = event.currentTarget.getBoundingClientRect();
    setMenu({ index, x: event.type === "contextmenu" ? event.clientX : rect.left, y: rect.top });
  };

  return (
    <div className="tab-bar" role="tablist" aria-label="Sheets">
      <div className="tab-strip">
        {tabs.map((tab, index) => {
          const isActive = index === activeIndex;
          const isDragged = drag?.from === index;
          // The line showing where the dragged tab lands: before the tab it pushes right, after the one it pushes left.
          const isTarget = drag !== null && !isDragged && drag.to === index;
          const dropSide = isTarget ? (drag.to < drag.from ? "drop-before" : "drop-after") : "";
          const classes = ["sheet-tab", isActive ? "is-active" : "", isDragged ? "is-dragged" : "", dropSide];
          return (
            <div
              key={tab.key}
              ref={(element) => {
                elements.current[index] = element;
              }}
              className={classes.filter(Boolean).join(" ")}
              style={isDragged ? { transform: `translateX(${drag.dx}px)` } : undefined}
              onPointerDown={(event) => startDrag(event, index)}
              onContextMenu={(event) => openMenu(event, index)}
            >
              {renaming === index ? (
                <RenameField
                  value={tab.label}
                  onDone={(name) => {
                    setRenaming(null);
                    if (name !== null && name !== tab.label) onRename(index, name);
                  }}
                />
              ) : (
                <button
                  type="button"
                  role="tab"
                  aria-selected={isActive}
                  className="sheet-tab-button"
                  tabIndex={isActive ? 0 : -1}
                  onClick={() => {
                    if (!wasDragged.current) onSelect(index);
                  }}
                  onDoubleClick={() => setRenaming(index)}
                  onKeyDown={(event) => onTabKey(event, index)}
                >
                  {tab.label}
                </button>
              )}
              {isActive && renaming !== index && (
                <button type="button" className="sheet-tab-more" aria-label="Sheet actions" onPointerDown={(event) => event.stopPropagation()} onClick={(event) => openMenu(event, index)}>
                  <Icon name="chevronUp" size={12} />
                </button>
              )}
            </div>
          );
        })}
        <Button icon="plus" tip="New sheet" tipSide="above" className="tab-add" onClick={onAdd} />
      </div>
      <div className="tab-bar-tools">{children}</div>
      {menu && (
        <TabMenu
          menu={menu}
          canDelete={canDelete}
          onClose={() => setMenu(null)}
          onRename={() => setRenaming(menu.index)}
          onDuplicate={() => onDuplicate(menu.index)}
          onDelete={() => onDelete(menu.index)}
        />
      )}
    </div>
  );
}

/** The tab's name as a field. Return or a click elsewhere keeps it, Escape cancels. */
function RenameField({ value, onDone }: { value: string; onDone: (name: string | null) => void }) {
  const [draft, setDraft] = useState(value);
  const isDone = useRef(false);
  const finish = (name: string | null) => {
    if (isDone.current) return;
    isDone.current = true;
    onDone(name === null || name.trim() === "" ? null : name.trim());
  };

  return (
    <input
      className="sheet-tab-field"
      value={draft}
      size={Math.max(6, draft.length + 1)}
      autoFocus
      onFocus={(event) => event.target.select()}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => finish(draft)}
      onPointerDown={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") finish(draft);
        if (event.key === "Escape") finish(null);
      }}
    />
  );
}

type MenuProps = {
  menu: Menu;
  canDelete: boolean;
  onClose: () => void;
  onRename: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
};

/** Rename, duplicate and delete, opening upwards from the tab. */
function TabMenu({ menu, canDelete, onClose, onRename, onDuplicate, onDelete }: MenuProps) {
  const run = (action: () => void) => () => {
    onClose();
    action();
  };

  return (
    <div className="tab-menu-scrim" onPointerDown={onClose} onContextMenu={(event) => event.preventDefault()}>
      <div className="tab-menu" role="menu" style={{ left: menu.x, bottom: window.innerHeight - menu.y + 4 }} onPointerDown={(event) => event.stopPropagation()}>
        <button type="button" role="menuitem" onClick={run(onRename)}>
          Rename
        </button>
        <button type="button" role="menuitem" onClick={run(onDuplicate)}>
          Duplicate
        </button>
        <button type="button" role="menuitem" className="is-danger" disabled={!canDelete} onClick={run(onDelete)}>
          Delete
        </button>
      </div>
    </div>
  );
}
