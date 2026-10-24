// The export sheet: format, resolution, one diagram or the whole series.
// Export settings are preferences, kept per machine and outside undo.

import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { errorText, exportFileName, exportSeries, exportSheet, revealPath } from "../lib/ipc";
import { fileManagerName } from "../lib/platform";
import { readPreference, writePreference } from "../lib/storage";
import type { ExportFormat, Look } from "../lib/types";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { NumberField } from "../ui/NumberField";
import { Segmented } from "../ui/Segmented";
import { useToast } from "../ui/Toasts";

const MIN_DPI = 72;
const MAX_DPI = 1200;
const DEFAULT_DPI = 300;

const FORMATS: { value: ExportFormat; label: string; name: string; note: string }[] = [
  { value: "pdf", label: "PDF", name: "PDF document", note: "Vector, for print and InDesign." },
  { value: "svg", label: "SVG", name: "SVG image", note: "Vector, for Illustrator and the web." },
  { value: "png", label: "PNG", name: "PNG image", note: "Pixels, for slides and social posts." },
  { value: "dxf", label: "DXF", name: "DXF drawing", note: "Real layers in RD New coordinates, for CAD." },
];

type Scope = "one" | "series";
type Settings = { format: ExportFormat; dpi: number; scope: Scope };

type Props = { look: Look; diagramName: string; diagramCount: number; onClose: () => void };

export function ExportDialog({ look, diagramName, diagramCount, onClose }: Props) {
  const [settings, setSettings] = useState<Settings>(() =>
    readPreference<Settings>("export", { format: "pdf", dpi: DEFAULT_DPI, scope: "one" }),
  );
  const [isExporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();
  const format = FORMATS.find((entry) => entry.value === settings.format) ?? FORMATS[0];

  const update = (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    writePreference("export", next);
  };

  const exportOne = async () => {
    const suggested = await exportFileName(look.diagram_id, settings.format);
    const path = await save({ defaultPath: suggested, filters: [{ name: format.name, extensions: [settings.format] }] });
    if (!path) return false;
    const written = await exportSheet(look, settings.format, settings.dpi, path);
    const fileName = written.split(/[\\/]/).pop() ?? written;
    toast(`Exported ${fileName}`, { action: { label: `Show in ${fileManagerName}`, run: () => revealPath(written) } });
    return true;
  };

  const exportAll = async () => {
    const folder = await open({ directory: true, title: "Choose a folder for the series" });
    if (typeof folder !== "string") return false;
    const written = await exportSeries(look, settings.format, settings.dpi, folder);
    toast(`Exported ${written.length} diagrams`, {
      action: written[0] ? { label: `Show in ${fileManagerName}`, run: () => revealPath(written[0]) } : undefined,
    });
    return true;
  };

  const run = async () => {
    setExporting(true);
    setError(null);
    try {
      const isDone = settings.scope === "one" ? await exportOne() : await exportAll();
      if (isDone) onClose();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setExporting(false);
    }
  };

  return (
    <Dialog
      title="Export"
      placement="sheet"
      width={380}
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={run} disabled={isExporting}>
            {isExporting ? "Exporting…" : settings.scope === "one" ? "Export…" : "Choose folder…"}
          </Button>
        </>
      }
    >
      <div className="export-fields" onKeyDown={(event) => event.key === "Enter" && !isExporting && run()}>
        <div className="field-stack">
          <span className="field-label">Format</span>
          <Segmented label="Format" value={settings.format} options={FORMATS} onChange={(value) => update({ format: value })} />
          <span className="field-note">{format.note}</span>
        </div>
        {settings.format === "png" && (
          <div className="field-stack">
            <span className="field-label">Resolution</span>
            <NumberField label="Resolution" value={settings.dpi} min={MIN_DPI} max={MAX_DPI} step={1} unit="dpi" onChange={(dpi) => update({ dpi })} />
          </div>
        )}
        <div className="field-stack">
          <span className="field-label">What to export</span>
          <Segmented
            label="What to export"
            value={settings.scope}
            options={[
              { value: "one", label: "This diagram" },
              { value: "series", label: `The whole series (${diagramCount})` },
            ]}
            onChange={(scope) => update({ scope })}
          />
          <span className="field-note">
            {settings.scope === "one"
              ? `${diagramName}, as it looks now.`
              : "One file per diagram, all in this theme and sheet, into a folder you choose."}
          </span>
        </div>
        {error && <p className="export-error">{error}</p>}
      </div>
    </Dialog>
  );
}
