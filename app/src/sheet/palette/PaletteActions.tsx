// The buttons under the palette picker: make a new palette, duplicate the
// current one, and edit or delete one of the user's own. A palette is renamed
// in the editor, so its name shows only once in the inspector. Choosing
// the saved palette goes through the look's undo history like any palette
// change; a deleted palette sits in Recently Deleted and the toast brings it back.

import { useState } from "react";
import { errorText } from "../../lib/ipc";
import { deletePalette, emptyDraft, paletteDraft, restorePalette, type PaletteDraft } from "../../lib/palette";
import type { Look, Theme } from "../../lib/types";
import { Button } from "../../ui/Button";
import { useToast } from "../../ui/Toasts";
import { PaletteEditor } from "./PaletteEditor";

// Where the sheet goes when its palette is deleted; the first built-in.
const FALLBACK_THEME = "paper";
const NEW_NAME = "New palette";

type Editing = { title: string; draft: PaletteDraft; id: string | null };

type Props = {
  theme: Theme | undefined;
  isOwn: boolean;
  look: Look;
  onSelect: (id: string) => void;
  /** Reads the palettes again after one was saved or deleted. */
  onCatalogChanged: () => Promise<unknown> | void;
};

export function PaletteActions({ theme, isOwn, look, onSelect, onCatalogChanged }: Props) {
  const [editing, setEditing] = useState<Editing | null>(null);
  const toast = useToast();

  const fail = (what: string) => (reason: unknown) => toast(`Couldn't ${what}. ${errorText(reason)}`, { isError: true });

  const openNew = () => setEditing({ title: "New palette", draft: emptyDraft(NEW_NAME), id: null });

  const openCopy = () => {
    if (!theme) return;
    paletteDraft(theme.id, `${theme.name} copy`)
      .then((draft) => setEditing({ title: `Duplicate ${theme.name}`, draft, id: null }))
      .catch(fail("duplicate the palette"));
  };

  const openEdit = () => {
    if (!theme) return;
    paletteDraft(theme.id, theme.name)
      .then((draft) => setEditing({ title: `Edit ${theme.name}`, draft, id: theme.id }))
      .catch(fail("open the palette"));
  };

  const remove = async () => {
    if (!theme) return;
    try {
      onSelect(FALLBACK_THEME);
      const kept = await deletePalette(theme.id);
      await onCatalogChanged();
      toast(`Deleted ${theme.name}`, {
        action: {
          label: "Undo",
          run: async () => {
            const restored = await restorePalette(kept);
            await onCatalogChanged();
            onSelect(restored.id);
          },
        },
      });
    } catch (reason) {
      fail("delete the palette")(reason);
    }
  };

  const saved = async (savedTheme: Theme) => {
    setEditing(null);
    await onCatalogChanged();
    onSelect(savedTheme.id);
    toast(`Saved ${savedTheme.name}`);
  };

  return (
    <>
      <div className="palette-actions">
        <Button icon="plus" variant="ghost" tip="Make a palette, or import one from a palette generator" onClick={openNew}>
          New
        </Button>
        <Button variant="ghost" tip="Start a palette of your own from this one" disabled={!theme} onClick={openCopy}>
          Duplicate
        </Button>
        {isOwn && (
          <>
            <Button variant="ghost" tip="Change its name, swatches and roles" onClick={openEdit}>
              Edit
            </Button>
            <Button icon="trash" variant="ghost" tip="Move it to Recently Deleted" onClick={remove} />
          </>
        )}
      </div>
      {editing && (
        <PaletteEditor
          title={editing.title}
          initial={editing.draft}
          editingId={editing.id}
          look={look}
          onClose={() => setEditing(null)}
          onSaved={saved}
        />
      )}
    </>
  );
}
