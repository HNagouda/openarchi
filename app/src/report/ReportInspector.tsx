// The report editor's right column: the report's palette and pages, the
// current page's paper and blocks, and the selected block's box and options.

import type { Catalog, Diagram, Paper, Ratios, Report, ReportBlock, RatiosStyle } from "../lib/types";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { Section } from "../ui/Section";
import { Segmented } from "../ui/Segmented";
import { Toggle } from "../ui/Toggle";
import { clampTo, deletePage, findBlock, MIN_BLOCK_MM, pageSize, setPaper, tidy, updateBlock } from "./edits";
import { LayerList } from "./LayerList";
import { RATIOS_STYLES, ratiosHeight } from "./library";
import type { TextKind } from "./textBlocks";

// The papers a report page offers, in one place.
const PAPERS: { value: Paper; label: string }[] = [
  { value: "a4", label: "A4" },
  { value: "a3", label: "A3" },
  { value: "a2", label: "A2" },
  { value: "a1", label: "A1" },
];

// Page backgrounds as palette roles, so they follow the palette.
const BACKGROUNDS: { value: string; label: string; tip: string }[] = [
  { value: "@paper", label: "Paper", tip: "The palette's paper colour" },
  { value: "@ground", label: "Ground", tip: "The palette's ground colour" },
  { value: "#ffffff", label: "White", tip: "Plain white, whatever the palette" },
];

type Props = {
  report: Report;
  catalog: Catalog;
  ratios: Ratios | null;
  pageIndex: number;
  selectedId: string | null;
  onChange: (report: Report, mergeKey?: string) => void;
  onSeal: () => void;
  onSelect: (id: string | null) => void;
  /** Puts a new text on the current page. */
  onAddText: (kind: TextKind) => void;
  onDuplicate: () => void;
  onRestack: (steps: number) => void;
  onDelete: () => void;
};

export function ReportInspector(props: Props) {
  const { report, catalog, ratios, pageIndex, selectedId, onChange } = props;
  const page = report.pages[pageIndex];
  const found = selectedId ? findBlock(report, selectedId) : null;

  return (
    <aside className="report-inspector inspector" aria-label="Report inspector">
      <Section id="report.report" title="Report">
        <div className="sheet-fields">
          <div className="title-fields">
            <EditableText className="sheet-title" value={report.name} onCommit={(name) => onChange({ ...report, name })} />
          </div>
          <label className="field-stack">
            <span className="field-label">Palette</span>
            <select
              className="report-select"
              value={report.theme_id ?? ""}
              onChange={(event) => onChange({ ...report, theme_id: event.target.value === "" ? null : event.target.value })}
            >
              <option value="">Same as the sheet</option>
              {catalog.themes.map((theme) => (
                <option key={theme.id} value={theme.id}>
                  {theme.name}
                </option>
              ))}
            </select>
          </label>
          <div className="field-stack">
            <span className="field-label">Background</span>
            <Segmented label="Background" value={report.background} options={BACKGROUNDS} onChange={(background) => onChange({ ...report, background })} />
          </div>
          <div className="toggles">
            <Toggle label="Page numbers" isOn={report.show_page_numbers} onChange={(show_page_numbers) => onChange({ ...report, show_page_numbers })} />
          </div>
        </div>
      </Section>

      <Section
        id="report.page"
        title={`Page ${pageIndex + 1} of ${report.pages.length}`}
        aside={<Button icon="trash" tip="Delete this page" disabled={report.pages.length <= 1} onClick={() => onChange(deletePage(report, pageIndex))} />}
      >
        <div className="sheet-fields">
          <div className="report-add-text">
            <Button icon="text" variant="secondary" tip="Under the last block" onClick={() => props.onAddText("heading")}>
              Add heading
            </Button>
            <Button icon="text" variant="secondary" tip="Under the last block" shortcut="t" onClick={() => props.onAddText("body")}>
              Add text
            </Button>
          </div>
          <Segmented label="Paper" value={page.paper} options={PAPERS} onChange={(paper) => onChange(setPaper(report, pageIndex, paper, page.landscape))} />
          <Segmented
            label="Orientation"
            value={page.landscape}
            options={[
              { value: false, label: "Portrait" },
              { value: true, label: "Landscape" },
            ]}
            onChange={(landscape) => onChange(setPaper(report, pageIndex, page.paper, landscape))}
          />
          <LayerList report={report} pageIndex={pageIndex} selectedId={selectedId} onChange={onChange} onSelect={props.onSelect} />
        </div>
      </Section>

      {found && (
        <Section id="report.block" title="Block">
          <BlockFields {...props} block={found.block} diagrams={catalog.diagrams} ratios={ratios} />
        </Section>
      )}
    </aside>
  );
}

type BlockProps = Props & { block: ReportBlock; diagrams: Diagram[] };

/** The block's box in millimetres, what it shows, and arranging it. */
function BlockFields({ report, block, diagrams, ratios, pageIndex, onChange, onSeal, onDuplicate, onRestack, onDelete }: BlockProps) {
  const page = pageSize(report.pages[pageIndex]);
  const content = block.content;
  const set = (patch: Partial<ReportBlock>) => onChange(updateBlock(report, block.id, patch));
  const field = (label: string, key: "x_mm" | "y_mm" | "width_mm" | "height_mm", min: number, max: number) => (
    <MillimetreField label={label} value={block[key]} min={min} max={max} onCommit={(value) => set({ [key]: value })} />
  );

  const setStyle = (style: RatiosStyle) => {
    if (content.kind !== "ratios") return;
    const group = ratios?.groups.find((entry) => entry.id === content.group);
    set({ content: { ...content, style }, height_mm: tidy(ratiosHeight(group, style)) });
  };

  return (
    <div className="sheet-fields" onBlur={onSeal}>
      <div className="report-box-fields">
        {field("X", "x_mm", 0, page.width - block.width_mm)}
        {field("Y", "y_mm", 0, page.height - block.height_mm)}
        {field("W", "width_mm", MIN_BLOCK_MM, page.width)}
        {field("H", "height_mm", MIN_BLOCK_MM, page.height)}
      </div>
      {content.kind === "ratios" && (
        <Segmented label="Chart" value={content.style} options={RATIOS_STYLES} onChange={setStyle} />
      )}
      {content.kind === "sheet" && (
        <SheetChoice diagrams={diagrams} diagramId={content.diagram_id} variant={content.variant ?? null} onChange={(diagram_id, variant) => set({ content: { ...content, diagram_id, variant } })} />
      )}
      <div className="report-arrange">
        <Button icon="chevronUp" tip="Bring forward" shortcut="mod+]" onClick={() => onRestack(1)} />
        <Button icon="chevronDown" tip="Send backward" shortcut="mod+[" onClick={() => onRestack(-1)} />
        <Button icon="plus" tip="Duplicate" shortcut="mod+d" onClick={onDuplicate} />
        <Button icon="trash" tip="Delete" shortcut="delete" onClick={onDelete} />
      </div>
    </div>
  );
}

function SheetChoice({ diagrams, diagramId, variant, onChange }: { diagrams: Diagram[]; diagramId: string; variant: string | null; onChange: (diagramId: string, variant: string | null) => void }) {
  const diagram = diagrams.find((entry) => entry.id === diagramId);
  return (
    <>
      <select className="report-select" aria-label="Diagram" value={diagramId} onChange={(event) => onChange(event.target.value, null)}>
        {diagrams.map((entry) => (
          <option key={entry.id} value={entry.id}>
            {entry.name}
          </option>
        ))}
      </select>
      {diagram && diagram.variants.length > 0 && (
        <Segmented
          label="Variant"
          value={variant ?? diagram.variants[0].id}
          options={diagram.variants.map((entry) => ({ value: entry.id, label: entry.name }))}
          onChange={(chosen) => onChange(diagramId, chosen)}
        />
      )}
    </>
  );
}

/** A typed millimetre value, applied on Return or when the field loses focus. */
function MillimetreField({ label, value, min, max, onCommit }: { label: string; value: number; min: number; max: number; onCommit: (value: number) => void }) {
  const commit = (typed: string) => {
    const number = Number(typed.replace(",", "."));
    if (typed.trim() === "" || Number.isNaN(number)) return;
    const clamped = tidy(clampTo(number, min, max));
    if (clamped !== value) onCommit(clamped);
  };

  return (
    <label className="number-box" data-tip={`${label} in millimetres`}>
      <span className="number-unit">{label}</span>
      <input
        key={value}
        className="number-input tabular"
        inputMode="decimal"
        aria-label={label}
        defaultValue={String(value)}
        onBlur={(event) => commit(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit(event.currentTarget.value);
        }}
      />
    </label>
  );
}
