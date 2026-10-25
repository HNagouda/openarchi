// The report editor's right column: the report's palette and pages, the
// current page's paper and blocks, and the selected block's box and options.

import type { Catalog, Diagram, Paper, Ratios, Report, ReportBlock, RatiosStyle } from "../lib/types";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { NumberBox } from "../ui/NumberField";
import { Section } from "../ui/Section";
import { Segmented } from "../ui/Segmented";
import { Toggle } from "../ui/Toggle";
import { deletePage, findBlock, MIN_BLOCK_MM, pageSize, setPaper, tidy, updateBlock } from "./edits";
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

// A block's box is typed to a tenth of a millimetre, the same as `tidy` keeps it.
const MM_STEP = 0.1;

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
              className="text-input"
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
            <Button icon="text" variant="secondary" tip="Under the last block" onClick={() => props.onAddText("caption")}>
              Add caption
            </Button>
          </div>
          <div className="field-stack">
            <span className="field-label">Paper</span>
            <Segmented label="Paper" value={page.paper} options={PAPERS} onChange={(paper) => onChange(setPaper(report, pageIndex, paper, page.landscape))} />
          </div>
          <div className="field-stack">
            <span className="field-label">Orientation</span>
            <Segmented
              label="Orientation"
              value={page.landscape}
              options={[
                { value: false, label: "Portrait" },
                { value: true, label: "Landscape" },
              ]}
              onChange={(landscape) => onChange(setPaper(report, pageIndex, page.paper, landscape))}
            />
          </div>
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
    <NumberBox
      prefix={label}
      label={label}
      tip={`${label} in millimetres`}
      value={block[key]}
      min={min}
      max={max}
      step={MM_STEP}
      onChange={(value) => set({ [key]: value })}
    />
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
        <div className="field-stack">
          <span className="field-label">Chart</span>
          <Segmented label="Chart" value={content.style} options={RATIOS_STYLES} onChange={setStyle} />
        </div>
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
      <label className="field-stack">
        <span className="field-label">Diagram</span>
        <select className="text-input" value={diagramId} onChange={(event) => onChange(event.target.value, null)}>
          {diagrams.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name}
            </option>
          ))}
        </select>
      </label>
      {diagram && diagram.variants.length > 0 && (
        <div className="field-stack">
          <span className="field-label">Variant</span>
          <Segmented
            label="Variant"
            value={variant ?? diagram.variants[0].id}
            options={diagram.variants.map((entry) => ({ value: entry.id, label: entry.name }))}
            onChange={(chosen) => onChange(diagramId, chosen)}
          />
        </div>
      )}
    </>
  );
}
