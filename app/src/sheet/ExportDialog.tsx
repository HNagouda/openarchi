// The export sheet: format, resolution, one diagram or the whole series, or
// laser cutting files for a site model.
// Export settings are preferences, kept per machine and outside undo.

import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { errorText, exportFileName, exportLaser, exportSeries, exportSheet, revealPath } from "../lib/ipc";
import { fileManagerName } from "../lib/platform";
import { readPreference, writePreference } from "../lib/storage";
import type { DxfLayers, ExportFormat, Highlight, Look, Pin } from "../lib/types";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { NumberField } from "../ui/NumberField";
import { Segmented } from "../ui/Segmented";
import { clamp } from "../lib/format";
import { DEFAULT_DPI, MAX_DPI, MIN_DPI } from "../lib/units";
import { useToast } from "../ui/Toasts";
import { LaserFields, useLaserSettings } from "./LaserFields";


// Laser cutting is not a drawing of the sheet but files for a model, so it sits beside the formats.
type Choice = ExportFormat | "laser";

const FORMATS: { value: Choice; label: string; name: string; note: string }[] = [
  { value: "pdf", label: "PDF", name: "PDF document", note: "Vector, for print and InDesign." },
  { value: "svg", label: "SVG", name: "SVG image", note: "Vector, for Illustrator and the web." },
  { value: "png", label: "PNG", name: "PNG image", note: "Pixels, for slides and social posts." },
  { value: "dxf", label: "DXF", name: "DXF drawing", note: "Real layers in RD New coordinates, for CAD." },
  { value: "laser", label: "Laser", name: "Laser cutting", note: "Cut and score files for a site model of this sheet's area." },
];

// A section sheet writes its section, not the plan, so the DXF note says what its coordinates are.
const SECTION_DXF_NOTE = "The section in metres along A–A′ and metres NAP, for CAD.";

// A plan DXF holds what the sheet shows by default, so the file matches the sheet the user is looking at.
const DXF_LAYERS: { value: DxfLayers; label: string; note: string }[] = [
  { value: "shown", label: "As on the sheet", note: "The layers this sheet shows, each in its colour on the sheet." },
  { value: "all", label: "Every layer", note: "Every fetched layer and the whole neighbourhood, whatever the sheet shows." },
];

type Scope = "one" | "series";
type Settings = { format: Choice; dpi: number; scope: Scope; dxfLayers: DxfLayers };

const DEFAULT_SETTINGS: Settings = { format: "pdf", dpi: DEFAULT_DPI, scope: "one", dxfLayers: "shown" };

// `variantId` is the variant drawn now, so the suggested file name says which.
// `sheetCount` counts every variant, since the series writes one file for each.
type Props = { look: Look; pins: Pin[]; highlights: Highlight[]; diagramName: string; variantId: string | null; sheetCount: number; onClose: () => void };

export function ExportDialog({ look, pins, highlights, diagramName, variantId, sheetCount, onClose }: Props) {
  // Settings saved before an option existed lack it, so the defaults fill the gap.
  // A resolution saved when the dialog allowed more is brought within today's limit.
  const [settings, setSettings] = useState<Settings>(() => {
    const saved = { ...DEFAULT_SETTINGS, ...readPreference<Partial<Settings>>("export", DEFAULT_SETTINGS) };
    return { ...saved, dpi: clamp(saved.dpi, MIN_DPI, MAX_DPI) };
  });
  const [laser, updateLaser] = useLaserSettings(look);
  const [isExporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();
  const format = FORMATS.find((entry) => entry.value === settings.format) ?? FORMATS[0];
  const isLaser = settings.format === "laser";
  // A section sheet's DXF is its section, which has no layers to choose from.
  const isPlanDxf = settings.format === "dxf" && look.view.mode !== "section";
  const dxfLayers = DXF_LAYERS.find((entry) => entry.value === settings.dxfLayers) ?? DXF_LAYERS[0];

  const update = (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    writePreference("export", next);
  };

  const exportOne = async (drawing: ExportFormat) => {
    const suggested = await exportFileName(look.diagram_id, variantId, drawing);
    const path = await save({ defaultPath: suggested, filters: [{ name: format.name, extensions: [drawing] }] });
    if (!path) return false;
    const written = await exportSheet(look, pins, highlights, drawing, settings.dpi, settings.dxfLayers, path);
    const fileName = written.split(/[\\/]/).pop() ?? written;
    toast(`Exported ${fileName}`, { action: { label: `Show in ${fileManagerName}`, run: () => revealPath(written) } });
    return true;
  };

  const exportAll = async (drawing: ExportFormat) => {
    const folder = await open({ directory: true, title: "Choose a folder for the series" });
    if (typeof folder !== "string") return false;
    const written = await exportSeries(look, pins, highlights, drawing, settings.dpi, settings.dxfLayers, folder);
    toast(`Exported ${written.length} diagrams`, {
      action: written[0] ? { label: `Show in ${fileManagerName}`, run: () => revealPath(written[0]) } : undefined,
    });
    return true;
  };

  const exportModel = async () => {
    const folder = await open({ directory: true, title: "Choose a folder for the laser files" });
    if (typeof folder !== "string") return false;
    const written = await exportLaser(look, laser, folder);
    const count = written.files.length;
    toast(count === 1 ? "Wrote 1 laser sheet" : `Wrote ${count} laser sheets`, {
      action: written.files[0] ? { label: `Show in ${fileManagerName}`, run: () => revealPath(written.files[0]) } : undefined,
    });
    return true;
  };

  const run = async () => {
    setExporting(true);
    setError(null);
    try {
      const choice = settings.format;
      let isDone: boolean;
      if (choice === "laser") isDone = await exportModel();
      else if (settings.scope === "one") isDone = await exportOne(choice);
      else isDone = await exportAll(choice);
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
      width={isLaser ? 440 : 380}
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={run} disabled={isExporting}>
            {isExporting ? "Exporting…" : settings.scope === "one" && !isLaser ? "Export…" : "Choose folder…"}
          </Button>
        </>
      }
    >
      <div className="export-fields" onKeyDown={(event) => event.key === "Enter" && !isExporting && run()}>
        <div className="field-stack">
          <span className="field-label">Format</span>
          <Segmented label="Format" value={settings.format} options={FORMATS} onChange={(value) => update({ format: value })} />
          <span className="field-note">{settings.format === "dxf" && look.view.mode === "section" ? SECTION_DXF_NOTE : format.note}</span>
        </div>
        {isLaser && <LaserFields look={look} settings={laser} onChange={updateLaser} />}
        {settings.format === "png" && (
          <div className="field-stack">
            <span className="field-label">Resolution</span>
            <NumberField label="Resolution" value={settings.dpi} min={MIN_DPI} max={MAX_DPI} step={1} unit="dpi" onChange={(dpi) => update({ dpi })} />
          </div>
        )}
        {isPlanDxf && (
          <div className="field-stack">
            <span className="field-label">Layers</span>
            <Segmented label="Layers" value={settings.dxfLayers} options={DXF_LAYERS} onChange={(value) => update({ dxfLayers: value })} />
            <span className="field-note">{dxfLayers.note}</span>
          </div>
        )}
        {!isLaser && (
          <div className="field-stack">
            <span className="field-label">What to export</span>
            <Segmented
              label="What to export"
              value={settings.scope}
              options={[
                { value: "one", label: "This diagram" },
                { value: "series", label: `The whole series (${sheetCount})` },
              ]}
              onChange={(scope) => update({ scope })}
            />
            <span className="field-note">
              {settings.scope === "one"
                ? `${diagramName}, as it looks now.`
                : "One file per diagram and per variant, all in this palette and sheet, into a folder you choose."}
            </span>
          </div>
        )}
        {error && <p className="export-error">{error}</p>}
      </div>
    </Dialog>
  );
}
