// The palette editor: the open sheet drawn live in the draft on the left; the
// name, import, swatches, a colour wheel and the roles on the right. Undo and
// redo inside it step through the draft, not the sheet.

import { useContext, useEffect, useState } from "react";
import { CommandContext, type CommandHandler } from "../../lib/commands";
import { errorText } from "../../lib/ipc";
import { paletteText, savePalette, type ExportFormat, type PaletteDraft, type Role } from "../../lib/palette";
import type { Look, Theme } from "../../lib/types";
import { Button } from "../../ui/Button";
import { ColorPicker } from "../../ui/ColorPicker";
import { Dialog } from "../../ui/Dialog";
import { useToast } from "../../ui/Toasts";
import { addSwatch, moveSwatch, pinRole, recolourSwatch, removeSwatch } from "./draftEdits";
import { ImportBox } from "./ImportBox";
import { RoleList } from "./RoleList";
import { SwatchStrip } from "./SwatchStrip";
import { useDraftHistory } from "./useDraftHistory";
import { usePalettePreview } from "./usePalettePreview";
import "./palette.css";

const DIALOG_WIDTH_PX = 1180;
const EXPORTS: { format: ExportFormat; label: string }[] = [
  { format: "json", label: "JSON" },
  { format: "csv", label: "CSV" },
  { format: "hex", label: "Hex list" },
];

type Props = {
  title: string;
  initial: PaletteDraft;
  /** The palette being edited; null saves a new one. */
  editingId: string | null;
  look: Look;
  onClose: () => void;
  onSaved: (theme: Theme) => void;
};

export function PaletteEditor({ title, initial, editingId, look, onClose, onSaved }: Props) {
  const history = useDraftHistory(initial);
  const { draft } = history;
  const [selected, setSelected] = useState(0);
  const [isSaving, setSaving] = useState(false);
  const preview = usePalettePreview(draft, look);
  const toast = useToast();
  useDraftCommands(history.undo, history.redo);

  const swatch = draft.swatches[Math.min(selected, draft.swatches.length - 1)];
  const rolesOf = (index: number) =>
    (preview.check?.roles ?? []).filter((colour) => colour.swatch === index).map((colour) => colour.role);

  const save = async () => {
    setSaving(true);
    try {
      onSaved(await savePalette(draft, editingId));
    } catch (reason) {
      toast(`Couldn't save the palette. ${errorText(reason)}`, { isError: true });
      setSaving(false);
    }
  };

  const copy = async (format: ExportFormat, label: string) => {
    try {
      await navigator.clipboard.writeText(paletteText(draft.swatches, format));
      toast(`Copied the palette as ${label}`);
    } catch (reason) {
      toast(`Couldn't copy the palette. ${errorText(reason)}`, { isError: true });
    }
  };

  const footer = (
    <div className="palette-footer">
      <span className="palette-copy">
        <span className="field-label">Copy as</span>
        {EXPORTS.map(({ format, label }) => (
          <Button key={format} variant="ghost" onClick={() => copy(format, label)}>
            {label}
          </Button>
        ))}
      </span>
      <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!history.canUndo} onClick={history.undo} />
      <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!history.canRedo} onClick={history.redo} />
      <span className="palette-footer-gap" />
      <Button variant="secondary" onClick={onClose}>
        Cancel
      </Button>
      <Button variant="primary" disabled={isSaving || draft.name.trim() === ""} onClick={save}>
        {editingId ? "Save" : "Save palette"}
      </Button>
    </div>
  );

  return (
    <Dialog title={title} onClose={onClose} width={DIALOG_WIDTH_PX} footer={footer}>
      <div className="palette-editor">
        <div className="palette-preview">
          {/* The core's own SVG of the open sheet; no pasted text goes into it. */}
          {preview.svg && <div className="palette-preview-sheet" dangerouslySetInnerHTML={{ __html: preview.svg }} />}
          {!preview.svg && !preview.error && <div className="palette-preview-waiting">Drawing the sheet…</div>}
          {preview.error && <p className="import-error">{preview.error}</p>}
        </div>

        <div className="palette-controls">
          <label className="field-stack">
            <span className="field-label">Name</span>
            <input
              className="text-input"
              value={draft.name}
              placeholder="Name this palette"
              onChange={(event) => history.change({ ...draft, name: event.target.value }, "name")}
              onKeyDown={(event) => event.stopPropagation()}
            />
          </label>

          <div className="field-stack">
            <span className="field-label">Import from a palette generator</span>
            <ImportBox
              onImport={(swatches) => {
                history.change({ ...draft, swatches, roles: {}, based_on: null });
                setSelected(0);
              }}
            />
          </div>

          <div className="field-stack">
            <span className="field-label">Swatches</span>
            <SwatchStrip
              swatches={draft.swatches}
              selected={selected}
              onSelect={setSelected}
              onMove={(from, to) => {
                history.change(moveSwatch(draft, from, to));
                setSelected(Math.min(Math.max(to, 0), draft.swatches.length - 1));
              }}
              onRemove={(index) => {
                history.change(removeSwatch(draft, index));
                setSelected(Math.max(0, index - 1));
              }}
              onAdd={() => {
                history.change(addSwatch(draft, swatch?.hex ?? "#888888"));
                setSelected(draft.swatches.length);
              }}
              rolesOf={rolesOf}
            />
          </div>

          <div className="palette-pick-and-roles">
            {swatch && (
              <div className="palette-picker" onPointerUp={history.seal} onBlur={history.seal}>
                <ColorPicker
                  hasWheel
                  value={swatch.hex}
                  onChange={(hex) => history.change(recolourSwatch(draft, selected, hex), `swatch-${selected}`)}
                />
              </div>
            )}
            <div className="field-stack palette-roles">
              <span className="field-label">Roles</span>
              {preview.check && (
                <RoleList
                  roles={preview.check.roles}
                  swatches={draft.swatches}
                  onPin={(role: Role, index) => history.change(pinRole(draft, role, index))}
                />
              )}
            </div>
          </div>

          {preview.check && preview.check.warnings.length > 0 && (
            <ul className="palette-warnings" aria-label="Contrast">
              {preview.check.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          )}
          {preview.check && preview.check.warnings.length === 0 && (
            <p className="inspector-note">Text and buildings read clearly on this paper (WCAG 2.1 contrast).</p>
          )}
        </div>
      </div>
    </Dialog>
  );
}

/** While the editor is open, the menu's Undo and Redo step through the draft. */
function useDraftCommands(undo: () => void, redo: () => void) {
  const slot = useContext(CommandContext);

  useEffect(() => {
    const previous = slot.current;
    const handler: CommandHandler = (id) => {
      if (id === "undo") undo();
      else if (id === "redo") redo();
      // Everything else waits until the editor closes.
      return true;
    };
    slot.current = handler;
    return () => {
      if (slot.current === handler) slot.current = previous;
    };
  }, [slot, undo, redo]);
}
