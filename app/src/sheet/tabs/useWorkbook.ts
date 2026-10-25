// The open project's sheets, pins and highlights, the tab that is showing, and
// every way they change. They share one undo stack, so deleting a tab undoes
// like any other edit. The look of the showing tab is what the rest of the
// workspace edits, through `changeLook`, exactly as before there were tabs.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, fittingScale, saveSheets } from "../../lib/ipc";
import { useHistory } from "../../lib/history";
import type { Area, Highlight, Look, Pin, ProjectView } from "../../lib/types";
import { useToast } from "../../ui/Toasts";
import { withoutLostViewPins } from "../viewpoint/viewPointEdits";
import { movedTo, sheetName, withAreaSheet, withDuplicate, withMoved, withNewSheet, withoutSheet, withRenamed, withSheet, type Workbook } from "./workbook";

/** The sheets, pins and highlights a project view holds; a view from before tabs has only its look. */
export function workbookOf(project: ProjectView): Workbook {
  const sheets = project.sheets?.length ? project.sheets : [project.look];
  return { sheets, pins: project.pins ?? [], highlights: project.highlights ?? [] };
}

/** Writes every sheet, pin and highlight of the open project. */
export const saveWorkbook = (book: Workbook) => saveSheets(book.sheets, book.pins, book.highlights);

/** The first sheet that differs between two workbooks of the same length, for undo to show it. */
function changedSheet(before: Workbook, after: Workbook): number | null {
  if (before.sheets.length !== after.sheets.length) return null;
  const index = after.sheets.findIndex((sheet, at) => JSON.stringify(sheet) !== JSON.stringify(before.sheets[at]));
  return index === -1 ? null : index;
}

export function useWorkbook(project: ProjectView) {
  const history = useHistory<Workbook>(workbookOf(project));
  const [active, setActive] = useState(0);
  const toast = useToast();
  const book = history.present;
  const index = Math.min(active, book.sheets.length - 1);
  const look = book.sheets[index];
  const latest = useRef({ book, index });
  latest.current = { book, index };

  // After an undo or redo, show the sheet it changed. A step that only
  // reorders the tabs keeps the sheet that was showing, wherever it lands.
  const stepFrom = useRef<{ book: Workbook; index: number } | null>(null);
  useEffect(() => {
    const from = stepFrom.current;
    if (!from) return;
    stepFrom.current = null;
    const shown = movedTo(from.book, book, from.index) ?? changedSheet(from.book, book);
    if (shown !== null) setActive(shown);
  }, [book]);

  const undo = useCallback(() => {
    stepFrom.current = latest.current;
    history.undo();
  }, [history]);

  const redo = useCallback(() => {
    stepFrom.current = latest.current;
    history.redo();
  }, [history]);

  const changeLook = useCallback(
    (next: Look, mergeKey?: string) => {
      const current = latest.current;
      history.change(withSheet(current.book, current.index, next), mergeKey);
    },
    [history],
  );

  // One step with the sheets, so undoing a deleted pin ties the eye back onto it.
  const changePins = useCallback(
    (pins: Pin[], mergeKey?: string) => {
      const current = latest.current.book;
      history.change({ ...current, pins, sheets: withoutLostViewPins(current.sheets, pins) }, mergeKey);
    },
    [history],
  );

  const changeHighlights = useCallback(
    (highlights: Highlight[], mergeKey?: string) => history.change({ ...latest.current.book, highlights }, mergeKey),
    [history],
  );

  /** A new tab after the others, in the showing sheet's look, framing `selection` at the scale that fills the paper. */
  const addAreaSheet = async (name: string, selection: Area) => {
    const { book: current, index: from } = latest.current;
    try {
      const scale = await fittingScale({ ...current.sheets[from], selection });
      // The tab may have changed while the scale was worked out; add to the newest workbook.
      const newest = latest.current.book;
      history.change(withAreaSheet(newest, from, name, selection, scale));
      setActive(newest.sheets.length);
    } catch (error) {
      toast(`Couldn't open that area as a sheet. ${errorText(error)}`, { isError: true });
    }
  };

  const addSheet = () => {
    history.change(withNewSheet(book, index));
    setActive(book.sheets.length);
  };

  const duplicateSheet = (at: number) => {
    history.change(withDuplicate(book, at));
    setActive(at + 1);
  };

  const deleteSheet = (at: number) => {
    if (book.sheets.length <= 1) return;
    const name = sheetName(book.sheets[at], at);
    history.change(withoutSheet(book, at));
    if (at <= index) setActive(Math.max(0, index - 1));
    toast(`Deleted “${name}”`, { action: { label: "Undo", run: undo } });
  };

  const renameSheet = (at: number, name: string) => history.change(withRenamed(book, at, name));

  const moveSheet = (from: number, to: number) => {
    history.change(withMoved(book, from, to));
    // The showing tab stays the showing tab wherever it ends up.
    if (index === from) setActive(to);
    else if (from < index && to >= index) setActive(index - 1);
    else if (from > index && to <= index) setActive(index + 1);
  };

  return {
    book,
    look,
    index,
    setActive,
    canUndo: history.canUndo,
    canRedo: history.canRedo,
    undo,
    redo,
    seal: history.seal,
    replace: history.replace,
    changeLook,
    changePins,
    changeHighlights,
    addAreaSheet,
    addSheet,
    duplicateSheet,
    deleteSheet,
    renameSheet,
    moveSheet,
  };
}
