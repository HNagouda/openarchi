// The project's sheets and pins as one value, so one undo stack covers adding,
// renaming, reordering and deleting tabs as well as every edit on a sheet.
// Every function returns a new workbook and leaves the old one alone.

import type { Look, Pin } from "../../lib/types";

export type Workbook = { sheets: Look[]; pins: Pin[] };

const COPY_SUFFIX = " copy";

/** The name a tab shows: its own, or "Sheet 2" by its place. */
export function sheetName(look: Look, index: number): string {
  return look.name?.trim() || `Sheet ${index + 1}`;
}

/** Every tab keeps the name it shows now, so moving or deleting a tab does not rename the others. */
function withNamesKept(book: Workbook): Workbook {
  return { ...book, sheets: book.sheets.map((sheet, at) => ({ ...sheet, name: sheetName(sheet, at) })) };
}

export function withSheet(book: Workbook, index: number, look: Look): Workbook {
  return { ...book, sheets: book.sheets.map((sheet, at) => (at === index ? look : sheet)) };
}

/** A new last sheet in the look of sheet `from`, showing the whole site. */
export function withNewSheet(book: Workbook, from: number): Workbook {
  const source = book.sheets[from];
  const fresh: Look = { ...source, name: unusedName(book), selection: null, hide_pins: false };
  return { ...book, sheets: [...book.sheets, fresh] };
}

/** A copy of sheet `index`, right after it. */
export function withDuplicate(book: Workbook, index: number): Workbook {
  const source = book.sheets[index];
  const copy: Look = { ...source, name: sheetName(source, index) + COPY_SUFFIX };
  const sheets = [...book.sheets];
  sheets.splice(index + 1, 0, copy);
  return { ...book, sheets };
}

/** The workbook without sheet `index`. The last sheet stays: a project always has one. */
export function withoutSheet(book: Workbook, index: number): Workbook {
  if (book.sheets.length <= 1) return book;
  const named = withNamesKept(book);
  return { ...named, sheets: named.sheets.filter((_, at) => at !== index) };
}

export function withRenamed(book: Workbook, index: number, name: string): Workbook {
  return withSheet(book, index, { ...book.sheets[index], name: name.trim() });
}

/** Sheet `from` moved so it ends up at `to`. */
export function withMoved(book: Workbook, from: number, to: number): Workbook {
  if (from === to) return book;
  const sheets = [...withNamesKept(book).sheets];
  const [moved] = sheets.splice(from, 1);
  sheets.splice(to, 0, moved);
  return { ...book, sheets };
}

/** A sheet as its tab shows it, so the same sheet compares equal before and after a move names it. */
function sheetKey(sheet: Look, index: number): string {
  return `${sheetName(sheet, index)}\n${JSON.stringify({ ...sheet, name: undefined })}`;
}

/**
 * Where sheet `shown` of `before` sits in `after`, when `after` holds the same
 * sheets in another order, as undoing or redoing a tab move leaves them.
 * Null for any other change.
 */
export function movedTo(before: Workbook, after: Workbook, shown: number): number | null {
  if (before.sheets.length !== after.sheets.length) return null;
  const beforeKeys = before.sheets.map(sheetKey);
  const afterKeys = after.sheets.map(sheetKey);
  const isSameOrder = beforeKeys.every((key, at) => key === afterKeys[at]);
  const isSameSheets = [...beforeKeys].sort().join("\n") === [...afterKeys].sort().join("\n");
  if (isSameOrder || !isSameSheets) return null;

  const to = afterKeys.indexOf(beforeKeys[shown]);
  return to === -1 ? null : to;
}

/** "Sheet 3", or the next number no tab already shows. */
function unusedName(book: Workbook): string {
  const names = book.sheets.map(sheetName);
  let number = book.sheets.length + 1;
  while (names.includes(`Sheet ${number}`)) number += 1;
  return `Sheet ${number}`;
}
