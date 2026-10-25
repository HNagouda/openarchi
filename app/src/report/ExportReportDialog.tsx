// Exporting a report: one PDF of every page, or an SVG or PNG per page.

import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { errorText, exportReport, reportFileName, revealPath } from "../lib/ipc";
import { fileManagerName } from "../lib/platform";
import { readPreference, writePreference } from "../lib/storage";
import type { Report, ReportFormat } from "../lib/types";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { NumberField } from "../ui/NumberField";
import { Segmented } from "../ui/Segmented";
import { DEFAULT_DPI, MAX_DPI, MIN_DPI } from "../lib/units";
import { useToast } from "../ui/Toasts";


const FORMATS: { value: ReportFormat; label: string; name: string; note: string }[] = [
  { value: "pdf", label: "PDF", name: "PDF document", note: "Every page in one vector file, ready to print or send." },
  { value: "svg", label: "SVG", name: "SVG image", note: "One vector file per page, each block a named layer." },
  { value: "png", label: "PNG", name: "PNG image", note: "One picture per page, for slides." },
];

type Settings = { format: ReportFormat; dpi: number };

export function ExportReportDialog({ report, onClose }: { report: Report; onClose: () => void }) {
  const [settings, setSettings] = useState<Settings>(() => readPreference<Settings>("reportExport", { format: "pdf", dpi: DEFAULT_DPI }));
  const [isExporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();
  const format = FORMATS.find((entry) => entry.value === settings.format) ?? FORMATS[0];
  const pageCount = report.pages.length;

  const update = (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    writePreference("reportExport", next);
  };

  const run = async () => {
    setExporting(true);
    setError(null);
    try {
      const suggested = await reportFileName(report.name, settings.format);
      const path = await save({ defaultPath: suggested, filters: [{ name: format.name, extensions: [settings.format] }] });
      if (!path) return;
      const written = await exportReport(report, settings.format, settings.dpi, path);
      const shown = written.length === 1 ? (written[0].split(/[\\/]/).pop() ?? written[0]) : `${written.length} pages`;
      toast(`Exported ${shown}`, { action: { label: `Show in ${fileManagerName}`, run: () => revealPath(written[0]) } });
      onClose();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setExporting(false);
    }
  };

  return (
    <Dialog
      title="Export report"
      placement="sheet"
      width={380}
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={run} disabled={isExporting}>
            {isExporting ? "Exporting…" : "Export…"}
          </Button>
        </>
      }
    >
      <div className="export-fields" onKeyDown={(event) => event.key === "Enter" && !isExporting && run()}>
        <div className="field-stack">
          <span className="field-label">Format</span>
          <Segmented label="Format" value={settings.format} options={FORMATS} onChange={(value) => update({ format: value })} />
          <span className="field-note">
            {format.note} {pageCount === 1 ? "One page." : `${pageCount} pages.`}
          </span>
        </div>
        {settings.format === "png" && (
          <div className="field-stack">
            <span className="field-label">Resolution</span>
            <NumberField label="Resolution" value={settings.dpi} min={MIN_DPI} max={MAX_DPI} step={1} unit="dpi" onChange={(dpi) => update({ dpi })} />
          </div>
        )}
        {error && <p className="export-error">{error}</p>}
      </div>
    </Dialog>
  );
}
