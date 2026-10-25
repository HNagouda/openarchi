// Every change the report editor makes, as a function from one report to the
// next. Nothing here mutates its input, so each result is one undo step.

import { clamp } from "../lib/format";
import type { BlockContent, Paper, Report, ReportBlock, ReportPage } from "../lib/types";

// The same smallest box the core accepts (MIN_BLOCK_MM in report.rs).
export const MIN_BLOCK_MM = 5;
// Where a duplicate lands, from the block it was made from.
const DUPLICATE_OFFSET_MM = 6;

/** Paper sizes in millimetres, portrait, as in crates/core/src/sheet.rs. */
export const PAPER_MM: Record<Paper, [number, number]> = {
  a4: [210, 297],
  a3: [297, 420],
  a2: [420, 594],
  a1: [594, 841],
  a0: [841, 1189],
  square: [300, 300],
};

/** Width and height in millimetres, turned for landscape. */
export function pageSize(page: ReportPage): { width: number; height: number } {
  const [width, height] = PAPER_MM[page.paper];
  if (page.landscape && page.paper !== "square") return { width: height, height: width };
  return { width, height };
}

/** The first "prefix-N" no id in `taken` uses yet. */
function nextId(prefix: string, taken: string[]): string {
  let number = taken.length + 1;
  while (taken.includes(`${prefix}-${number}`)) number += 1;
  return `${prefix}-${number}`;
}

function blockIds(report: Report): string[] {
  return report.pages.flatMap((page) => page.blocks.map((block) => block.id));
}

export function newReport(taken: Report[], name: string): Report {
  return {
    id: nextId("report", taken.map((report) => report.id)),
    name,
    theme_id: null,
    background: "@paper",
    show_page_numbers: true,
    pages: [{ id: "page-1", paper: "a4", landscape: false, blocks: [] }],
  };
}

function withPage(report: Report, index: number, change: (page: ReportPage) => ReportPage): Report {
  return { ...report, pages: report.pages.map((page, at) => (at === index ? change(page) : page)) };
}

/** A new page after `index`, the same paper as that one. */
export function addPage(report: Report, index: number): Report {
  const model = report.pages[index] ?? report.pages[report.pages.length - 1];
  const page: ReportPage = {
    id: nextId("page", report.pages.map((entry) => entry.id)),
    paper: model?.paper ?? "a4",
    landscape: model?.landscape ?? false,
    blocks: [],
  };
  const pages = [...report.pages];
  pages.splice(index + 1, 0, page);
  return { ...report, pages };
}

/** A report always keeps one page. */
export function deletePage(report: Report, index: number): Report {
  if (report.pages.length <= 1) return report;
  return { ...report, pages: report.pages.filter((_, at) => at !== index) };
}

export function movePage(report: Report, from: number, to: number): Report {
  const pages = [...report.pages];
  const [page] = pages.splice(from, 1);
  pages.splice(to, 0, page);
  return { ...report, pages };
}

export function setPaper(report: Report, index: number, paper: Paper, landscape: boolean): Report {
  return withPage(report, index, (page) => ({ ...page, paper, landscape }));
}

/** Adds a block centred on (`x`, `y`) in millimetres, kept on the page. Returns the report and the new id. */
export function addBlock(
  report: Report,
  index: number,
  content: BlockContent,
  size: { width: number; height: number },
  at: { x: number; y: number },
): [Report, string] {
  const id = nextId("block", blockIds(report));
  const rect = placedRect(pageSize(report.pages[index]), size, at);
  const block: ReportBlock = { id, x_mm: rect.x, y_mm: rect.y, width_mm: rect.width, height_mm: rect.height, content };
  return [withPage(report, index, (entry) => ({ ...entry, blocks: [...entry.blocks, block] })), id];
}

/** The box a block of `size` gets centred on `at`, no bigger than the page and kept on it. */
export function placedRect(page: { width: number; height: number }, size: { width: number; height: number }, at: { x: number; y: number }) {
  const width = Math.min(size.width, page.width);
  const height = Math.min(size.height, page.height);
  return {
    x: tidy(clamp(at.x - width / 2, 0, page.width - width)),
    y: tidy(clamp(at.y - height / 2, 0, page.height - height)),
    width: tidy(width),
    height: tidy(height),
  };
}

export function updateBlock(report: Report, id: string, patch: Partial<ReportBlock>): Report {
  return {
    ...report,
    pages: report.pages.map((page) => ({
      ...page,
      blocks: page.blocks.map((block) => (block.id === id ? { ...block, ...patch } : block)),
    })),
  };
}

export function deleteBlock(report: Report, id: string): Report {
  return { ...report, pages: report.pages.map((page) => ({ ...page, blocks: page.blocks.filter((block) => block.id !== id) })) };
}

/** A copy just below and right of the block, on top of everything. Returns the report and the copy's id. */
export function duplicateBlock(report: Report, id: string): [Report, string | null] {
  const found = findBlock(report, id);
  if (!found) return [report, null];
  const copyId = nextId("block", blockIds(report));
  const page = pageSize(report.pages[found.pageIndex]);
  const copy: ReportBlock = {
    ...found.block,
    id: copyId,
    x_mm: clamp(found.block.x_mm + DUPLICATE_OFFSET_MM, 0, page.width - found.block.width_mm),
    y_mm: clamp(found.block.y_mm + DUPLICATE_OFFSET_MM, 0, page.height - found.block.height_mm),
  };
  const next = withPage(report, found.pageIndex, (entry) => ({ ...entry, blocks: [...entry.blocks, copy] }));
  return [next, copyId];
}

/** Moves a block `steps` places up (positive) or down the drawing order of its page. */
export function restack(report: Report, id: string, steps: number): Report {
  const found = findBlock(report, id);
  if (!found) return report;
  const blocks = [...report.pages[found.pageIndex].blocks];
  const from = blocks.findIndex((block) => block.id === id);
  const to = clamp(from + steps, 0, blocks.length - 1);
  return reorderBlocks(report, found.pageIndex, from, to);
}

export function reorderBlocks(report: Report, pageIndex: number, from: number, to: number): Report {
  return withPage(report, pageIndex, (page) => {
    const blocks = [...page.blocks];
    const [block] = blocks.splice(from, 1);
    blocks.splice(to, 0, block);
    return { ...page, blocks };
  });
}

export function findBlock(report: Report, id: string): { block: ReportBlock; pageIndex: number } | null {
  for (const [pageIndex, page] of report.pages.entries()) {
    const block = page.blocks.find((entry) => entry.id === id);
    if (block) return { block, pageIndex };
  }
  return null;
}

/** Rounded to a tenth of a millimetre, so stored positions stay readable. */
export function tidy(value: number): number {
  return Math.round(value * 10) / 10;
}
