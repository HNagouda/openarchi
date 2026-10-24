// Small, pure edits to the texts on a sheet: the title, the subtitle and the
// user's text boxes, all addressed by the ids the core writes into the SVG.

import type { Look, Position, TextBox, TextStyle } from "../../lib/types";

export const TITLE_ID = "title";
export const SUBTITLE_ID = "subtitle";
const NEW_TEXT = "Text";

export function isTextBox(id: string): boolean {
  return id !== TITLE_ID && id !== SUBTITLE_ID;
}

export function textOf(look: Look, id: string): string {
  if (id === TITLE_ID) return look.sheet.title;
  if (id === SUBTITLE_ID) return look.sheet.subtitle;
  return look.sheet.texts.find((text) => text.id === id)?.text ?? "";
}

export function styleOf(look: Look, id: string): TextStyle {
  if (id === TITLE_ID) return look.sheet.title_style;
  if (id === SUBTITLE_ID) return look.sheet.subtitle_style;
  return look.sheet.texts.find((text) => text.id === id)?.style ?? {};
}

export function positionOf(look: Look, id: string): Position | null {
  if (id === TITLE_ID) return look.sheet.title_position;
  if (id === SUBTITLE_ID) return look.sheet.subtitle_position;
  return look.sheet.texts.find((text) => text.id === id)?.position ?? null;
}

export function withText(look: Look, id: string, text: string): Look {
  if (id === TITLE_ID) return { ...look, sheet: { ...look.sheet, title: text } };
  if (id === SUBTITLE_ID) return { ...look, sheet: { ...look.sheet, subtitle: text } };
  return withBox(look, id, (box) => ({ ...box, text }));
}

/** Sets or clears style fields; `null` clears one back to the theme. */
export function withStyle(look: Look, id: string, patch: TextStyle): Look {
  const merged: TextStyle = { ...styleOf(look, id), ...patch };
  const style = Object.fromEntries(Object.entries(merged).filter(([, value]) => value !== null && value !== undefined)) as TextStyle;
  if (id === TITLE_ID) return { ...look, sheet: { ...look.sheet, title_style: style } };
  if (id === SUBTITLE_ID) return { ...look, sheet: { ...look.sheet, subtitle_style: style } };
  return withBox(look, id, (box) => ({ ...box, style }));
}

/** `null` puts the title or subtitle back in the automatic layout. */
export function withPosition(look: Look, id: string, position: Position | null): Look {
  if (id === TITLE_ID) return { ...look, sheet: { ...look.sheet, title_position: position } };
  if (id === SUBTITLE_ID) return { ...look, sheet: { ...look.sheet, subtitle_position: position } };
  return withBox(look, id, (box) => ({ ...box, position }));
}

/** A new text box at `position`, and its id. */
export function withNewBox(look: Look, position: Position): { look: Look; id: string } {
  const id = nextBoxId(look.sheet.texts);
  const box: TextBox = { id, text: NEW_TEXT, style: {}, position };
  return { look: { ...look, sheet: { ...look.sheet, texts: [...look.sheet.texts, box] } }, id };
}

export function withoutBox(look: Look, id: string): Look {
  return { ...look, sheet: { ...look.sheet, texts: look.sheet.texts.filter((text) => text.id !== id) } };
}

function withBox(look: Look, id: string, change: (box: TextBox) => TextBox): Look {
  const texts = look.sheet.texts.map((text) => (text.id === id ? change(text) : text));
  return { ...look, sheet: { ...look.sheet, texts } };
}

/** "text-N" with the first N no box uses, the same rule the core follows. */
function nextBoxId(texts: TextBox[]): string {
  let number = texts.length + 1;
  while (texts.some((text) => text.id === `text-${number}`)) number += 1;
  return `text-${number}`;
}
