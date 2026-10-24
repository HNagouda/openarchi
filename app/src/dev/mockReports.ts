// What the dev mock answers for the report editor without the core: page
// backdrops and text blocks drawn here roughly, and the names of the block
// fixtures app_fixtures writes. Only for screenshots in a plain browser.

import type { BlockContent, Report, ReportBlock, Theme } from "../lib/types";
import { pageSize } from "../report/edits";

const MM_PER_PT = 25.4 / 72;
// Text sizes when a style leaves them out: the sheet's text box at A3.
const DEFAULT_SIZE_PT = 7.7;
// Roughly how wide a character runs, as a share of its size, for wrapping.
const CHARACTER_WIDTH_EM = 0.5;
const LINE_HEIGHT = 1.35;
const NUMBER_RIGHT_MM = 12;
const NUMBER_BOTTOM_MM = 8;
const NUMBER_MM = 2.5;

/** "ratios-land_use-bars", "facts-site", "notes-wind", as app_fixtures names them. */
export function mockBlockFixture(content: BlockContent): string {
  if (content.kind === "ratios") return `ratios-${content.group}-${content.style}`;
  if (content.kind === "analysis_facts") return `facts-${content.section}`;
  if (content.kind === "analysis_notes") return `notes-${content.section}`;
  return "missing";
}

function colour(theme: Theme, value: string): string {
  if (!value.startsWith("@")) return value;
  return theme.palette[value.slice(1) as keyof Theme["palette"]] ?? theme.palette.paper;
}

export function mockBackdrop(report: Report, index: number, theme: Theme): string {
  const { width, height } = pageSize(report.pages[index]);
  const number = report.show_page_numbers
    ? `<text x="${width - NUMBER_RIGHT_MM}" y="${height - NUMBER_BOTTOM_MM}" font-size="${NUMBER_MM}" fill="${theme.palette.ink}" fill-opacity="0.55" text-anchor="end" font-family="${theme.typography.family}">${index + 1} / ${report.pages.length}</text>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}mm" height="${height}mm" viewBox="0 0 ${width} ${height}"><rect width="${width}" height="${height}" fill="${colour(theme, report.background)}"/>${number}</svg>`;
}

export function mockTextBlock(block: ReportBlock, theme: Theme): string {
  if (block.content.kind !== "text") return "";
  const style = block.content.style;
  const sizePt = style.size_pt ?? DEFAULT_SIZE_PT;
  const size = sizePt * MM_PER_PT;
  const perLine = Math.max(1, Math.floor(block.width_mm / (size * CHARACTER_WIDTH_EM)));
  const lines = block.content.text.split("\n").flatMap((line) => wrap(line, perLine));
  const align = style.align ?? "left";
  const x = { left: 0, centre: block.width_mm / 2, right: block.width_mm }[align];
  const anchor = { left: "start", centre: "middle", right: "end" }[align];
  const family = style.family ?? theme.typography.family;
  const fill = colour(theme, style.colour ?? "@ink");
  const texts = lines.map((line, at) => `<text x="${x}" y="${size * 0.72 + at * size * LINE_HEIGHT}">${escape(line)}</text>`).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${block.width_mm}mm" height="${block.height_mm}mm" viewBox="0 0 ${block.width_mm} ${block.height_mm}" overflow="visible"><g data-text="report-text" data-x="${x}" data-y="0" data-align="${align}" data-family="${family}" data-size-pt="${sizePt}" data-bold="${style.bold ?? false}" data-italic="${style.italic ?? false}" data-colour="${fill}" data-opacity="${style.opacity ?? 1}" font-family="'${family}'" font-size="${size}" font-weight="${style.bold ? 600 : 400}" font-style="${style.italic ? "italic" : "normal"}" fill="${fill}" text-anchor="${anchor}">${texts}</g></svg>`;
}

function wrap(line: string, perLine: number): string[] {
  const lines: string[] = [];
  let current = "";
  for (const word of line.split(" ")) {
    if (current && current.length + word.length + 1 > perLine) {
      lines.push(current);
      current = word;
    } else {
      current = current ? `${current} ${word}` : word;
    }
  }
  return [...lines, current];
}

function escape(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
