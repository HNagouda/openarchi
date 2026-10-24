// What the report library offers: every block the project can draw, grouped
// as the panel lists them, each with the size it starts at on a page.

import type { AnalysisSection, BlockContent, Diagram, Ratios, RatioGroup, RatiosStyle, Sheet, TextStyle } from "../lib/types";
import { PAPER_MM } from "./edits";
import type { Size } from "./geometry";

export type LibraryGroup = "Ratios" | "Analysis" | "Sheets" | "Text";

export type LibraryItem = {
  key: string;
  group: LibraryGroup;
  label: string;
  /** A second, quieter line: the section a figure belongs to, a diagram's group. */
  detail: string;
  content: BlockContent;
  size: Size;
  /** An SVG document to show as the item's picture, when the data has one. */
  svg?: string;
};

// Starting widths in millimetres: wide enough to read on A4 without zooming.
const CHART_WIDTH_MM = 120;
const FIGURE_WIDTH_MM = 90;
const FACTS_WIDTH_MM = 170;
const NOTES_SIZE: Size = { width: 90, height: 70 };
const SHEET_WIDTH_MM = 170;

// Row pitches from crates/core/src/report/ratio_blocks.rs and analysis_blocks.rs,
// so a block starts about as tall as its content.
const RATIOS_HEADER_MM = 7.1;
const BAR_ROW_MM = 7.6;
const TABLE_ROW_MM = 5.4;
const PIE_ROW_MM = 5;
const PIE_DIAMETER_MM = 43;
const FACTS_HEADER_MM = 9.9;
const FACT_ROW_MM = 10;
const FACT_COLUMNS = 4;

// Every way a ratios group can be drawn, in one place.
export const RATIOS_STYLES: { value: RatiosStyle; label: string }[] = [
  { value: "bars", label: "Bars" },
  { value: "pie", label: "Pie" },
  { value: "table", label: "Table" },
];

// The text blocks the library starts from.
const TEXT_PRESETS: { key: string; label: string; text: string; style: TextStyle; size: Size }[] = [
  { key: "heading", label: "Heading", text: "Heading", style: { size_pt: 24, bold: true }, size: { width: 170, height: 14 } },
  { key: "body", label: "Body text", text: "Write about the site here.", style: { size_pt: 10 }, size: { width: 120, height: 30 } },
  { key: "caption", label: "Caption", text: "Caption", style: { size_pt: 7, italic: true }, size: { width: 80, height: 8 } },
];

export function ratiosItems(ratios: Ratios): LibraryItem[] {
  return ratios.groups.map((group) => ({
    key: `ratios-${group.id}`,
    group: "Ratios",
    label: group.title,
    detail: `${group.rows.length} rows`,
    content: { kind: "ratios", group: group.id, style: "bars" },
    size: { width: CHART_WIDTH_MM, height: ratiosHeight(group, "bars") },
  }));
}

/** About how tall a group comes out in `style` at the starting width. */
export function ratiosHeight(group: RatioGroup | undefined, style: RatiosStyle): number {
  const rows = group?.rows.length ?? 6;
  if (style === "bars") return RATIOS_HEADER_MM + rows * BAR_ROW_MM;
  if (style === "table") return RATIOS_HEADER_MM + rows * TABLE_ROW_MM;
  return RATIOS_HEADER_MM + Math.max(PIE_DIAMETER_MM, rows * PIE_ROW_MM);
}

export function analysisItems(sections: AnalysisSection[]): LibraryItem[] {
  return sections.flatMap((section) => {
    const figures: LibraryItem[] = section.figures.map((figure) => ({
      key: `figure-${section.id}-${figure.id}`,
      group: "Analysis",
      label: figureLabel(section, figure.id),
      detail: section.title,
      content: { kind: "analysis_figure", section: section.id, figure: figure.id },
      size: { width: FIGURE_WIDTH_MM, height: (FIGURE_WIDTH_MM * figure.height_mm) / figure.width_mm },
      svg: figure.svg,
    }));
    const rows = Math.ceil(section.facts.length / FACT_COLUMNS);
    const facts: LibraryItem = {
      key: `facts-${section.id}`,
      group: "Analysis",
      label: "Facts",
      detail: section.title,
      content: { kind: "analysis_facts", section: section.id },
      size: { width: FACTS_WIDTH_MM, height: FACTS_HEADER_MM + rows * FACT_ROW_MM },
    };
    const notes: LibraryItem = {
      key: `notes-${section.id}`,
      group: "Analysis",
      label: "Design notes",
      detail: section.title,
      content: { kind: "analysis_notes", section: section.id },
      size: NOTES_SIZE,
    };
    return section.facts.length > 0 ? [...figures, facts, notes] : [...figures, notes];
  });
}

/** "Wind" for a section's only figure, "Sun: radiation-rose" spelled out for the second. */
function figureLabel(section: AnalysisSection, figureId: string): string {
  if (section.figures.length === 1) return `${section.title} chart`;
  const words = figureId.replace(/-/g, " ");
  return words[0].toUpperCase() + words.slice(1);
}

export function sheetItems(diagrams: Diagram[], sheet: Sheet): LibraryItem[] {
  const [width, height] = PAPER_MM[sheet.paper];
  // Width over height of the sheet as it lies.
  const ratio = sheet.landscape && sheet.paper !== "square" ? height / width : width / height;
  return diagrams.map((diagram) => ({
    key: `sheet-${diagram.id}`,
    group: "Sheets",
    label: diagram.name,
    detail: diagram.group,
    content: { kind: "sheet", diagram_id: diagram.id, variant: null },
    size: { width: SHEET_WIDTH_MM, height: SHEET_WIDTH_MM / ratio },
  }));
}

export function textItems(): LibraryItem[] {
  return TEXT_PRESETS.map((preset) => ({
    key: `text-${preset.key}`,
    group: "Text",
    label: preset.label,
    detail: `${preset.style.size_pt} pt`,
    content: { kind: "text", text: preset.text, style: preset.style },
    size: preset.size,
  }));
}
