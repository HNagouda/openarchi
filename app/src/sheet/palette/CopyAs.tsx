// Copies the swatches out as text, for another tool or a colleague. The JSON
// is the shape the importer reads back.

import { errorText } from "../../lib/ipc";
import { paletteText, type ExportFormat, type PaletteSwatch } from "../../lib/palette";
import { Button } from "../../ui/Button";
import { useToast } from "../../ui/Toasts";

const EXPORTS: { format: ExportFormat; label: string }[] = [
  { format: "json", label: "JSON" },
  { format: "csv", label: "CSV" },
  { format: "hex", label: "Hex list" },
];

export function CopyAs({ swatches }: { swatches: PaletteSwatch[] }) {
  const toast = useToast();

  const copy = async (format: ExportFormat, label: string) => {
    try {
      await navigator.clipboard.writeText(paletteText(swatches, format));
      toast(`Copied the palette as ${label}`);
    } catch (reason) {
      toast(`Couldn't copy the palette. ${errorText(reason)}`, { isError: true });
    }
  };

  return (
    <div className="palette-copy">
      <span className="field-label">Copy as</span>
      {EXPORTS.map(({ format, label }) => (
        <Button key={format} variant="ghost" onClick={() => copy(format, label)}>
          {label}
        </Button>
      ))}
    </div>
  );
}
