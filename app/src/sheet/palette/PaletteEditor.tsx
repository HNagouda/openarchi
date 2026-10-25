// The palette editor: the open sheet drawn live in the draft on the left, and
// on the right the short path most palettes take: paste or pick colours, check
// they read, name the palette, save. Which colour paints which role, and
// copying the palette out as text, wait behind one disclosure. Undo and redo
// inside it step through the draft, not the sheet.

import { useContext, useEffect, useState } from "react";
import { CommandContext, type CommandHandler } from "../../lib/commands";
import { useHistory } from "../../lib/history";
import { errorText } from "../../lib/ipc";
import { savePalette, type PaletteDraft, type Role } from "../../lib/palette";
import type { Look, Theme } from "../../lib/types";
import { Button } from "../../ui/Button";
import { ColorPicker } from "../../ui/ColorPicker";
import { Dialog } from "../../ui/Dialog";
import { useToast } from "../../ui/Toasts";
import { CopyAs } from "./CopyAs";
import { addSwatch, moveSwatch, pinRole, recolourSwatch, removeSwatch } from "./draftEdits";
import { ImportBox } from "./ImportBox";
import { Readability } from "./Readability";
import { RoleList } from "./RoleList";
import { SwatchStrip } from "./SwatchStrip";
import { usePalettePreview } from "./usePalettePreview";
import "./palette.css";

// As wide as the window allows, up to where a wider sheet stops helping.
const DIALOG_WIDTH = "min(1720px, calc(100vw - 48px))";
// The colour a new swatch starts as when none is chosen.
const NEW_SWATCH_HEX = "#888888";

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
  const history = useHistory<PaletteDraft>(initial);
  const draft = history.present;
  // The swatch whose colour picker is open; none at first, so the editor opens calm.
  const [selected, setSelected] = useState<number | null>(null);
  const [isSaving, setSaving] = useState(false);
  const preview = usePalettePreview(draft, look);
  const toast = useToast();
  useDraftCommands(history.undo, history.redo);

  const swatch = selected === null ? null : draft.swatches[Math.min(selected, draft.swatches.length - 1)];
  const rolesOf = (index: number) =>
    (preview.check?.roles ?? []).filter((colour) => colour.swatch === index).map((colour) => colour.role);
  const isReadable = preview.check?.is_readable ?? false;
  const canSave = !isSaving && isReadable && draft.name.trim() !== "";

  const save = async () => {
    setSaving(true);
    try {
      onSaved(await savePalette(draft, editingId));
    } catch (reason) {
      toast(`Couldn't save the palette. ${errorText(reason)}`, { isError: true });
      setSaving(false);
    }
  };

  const footer = (
    <div className="palette-footer">
      <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!history.canUndo} onClick={history.undo} />
      <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!history.canRedo} onClick={history.redo} />
      <span className="palette-footer-gap" />
      {preview.check && !isReadable && <span className="palette-save-blocked">Fix the text contrast to save</span>}
      <Button variant="secondary" onClick={onClose}>
        Cancel
      </Button>
      <Button variant="primary" disabled={!canSave} onClick={save}>
        {editingId ? "Save" : "Save palette"}
      </Button>
    </div>
  );

  return (
    <Dialog title={title} onClose={onClose} width={DIALOG_WIDTH} footer={footer}>
      <div className="palette-editor">
        <div className="palette-preview">
          {/* The core's own SVG of the open sheet; no pasted text goes into it. */}
          {preview.svg && <div className="palette-preview-sheet" dangerouslySetInnerHTML={{ __html: preview.svg }} />}
          {!preview.svg && !preview.error && <div className="palette-preview-waiting">Drawing the sheet…</div>}
          {preview.error && <p className="import-error">{preview.error}</p>}
        </div>

        <div className="palette-controls">
          <section className="palette-step">
            <h3 className="palette-step-title">Colours</h3>
            <ImportBox
              onImport={(swatches) => {
                history.change({ ...draft, swatches, roles: {}, based_on: null });
                setSelected(null);
              }}
            />
            <SwatchStrip
              swatches={draft.swatches}
              selected={selected}
              onSelect={(index) => setSelected(index === selected ? null : index)}
              onMove={(from, to) => {
                history.change(moveSwatch(draft, from, to));
                setSelected(Math.min(Math.max(to, 0), draft.swatches.length - 1));
              }}
              onRemove={(index) => {
                history.change(removeSwatch(draft, index));
                setSelected(null);
              }}
              onAdd={() => {
                history.change(addSwatch(draft, swatch?.hex ?? NEW_SWATCH_HEX));
                setSelected(draft.swatches.length);
              }}
              rolesOf={rolesOf}
            />
            {swatch && selected !== null && (
              <div className="palette-picker" onPointerUp={history.seal} onBlur={history.seal}>
                <ColorPicker
                  value={swatch.hex}
                  onChange={(hex) => history.change(recolourSwatch(draft, selected, hex), `swatch-${selected}`)}
                />
              </div>
            )}
            {preview.check && <Readability warnings={preview.check.warnings} isReadable={isReadable} />}
          </section>

          <section className="palette-step">
            <label className="palette-step-title" htmlFor="palette-name">
              Name
            </label>
            <input
              id="palette-name"
              className="text-input palette-name"
              value={draft.name}
              placeholder="Name this palette"
              onChange={(event) => history.change({ ...draft, name: event.target.value }, "name")}
              onKeyDown={(event) => event.stopPropagation()}
            />
          </section>

          <details className="palette-advanced">
            <summary>Fine-tune which colour paints what</summary>
            {preview.check && (
              <RoleList
                roles={preview.check.roles}
                swatches={draft.swatches}
                onPin={(role: Role, index) => history.change(pinRole(draft, role, index))}
              />
            )}
            <CopyAs swatches={draft.swatches} />
          </details>
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
