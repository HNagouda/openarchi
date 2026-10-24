// Keeps the open sheet in step with the project on disk: an assistant's edits
// show up live, and the last change is written before the window closes.

import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { errorText, onLibraryChanged, reloadProject } from "../lib/ipc";
import type { ProjectView } from "../lib/types";
import { useToast } from "../ui/Toasts";
import type { Workbook } from "./tabs/workbook";
import { workbookOf } from "./tabs/useWorkbook";

// File events come in bursts; read the project once they settle.
const RELOAD_DELAY_MS = 400;

type Saving = {
  hasUnsaved: boolean;
  flush: () => Promise<void>;
  markSaved: (saved: Workbook) => void;
};

type Undo = {
  book: Workbook;
  replace: (book: Workbook) => void;
};

type Options = {
  projectId: string;
  autosave: Saving;
  history: Undo;
  /** The project as the assistant left it, once it is the one on screen. */
  onReloaded: (fresh: ProjectView) => void;
};

export function useProjectSync({ projectId, autosave, history, onReloaded }: Options) {
  const toast = useToast();
  const latest = useRef({ autosave, history, projectId, onReloaded });
  latest.current = { autosave, history, projectId, onReloaded };

  // An assistant edited this project through the MCP server: show its version,
  // unless the user is mid-edit, in which case their next save wins and the
  // assistant's version stays in the backups.
  useEffect(() => {
    let timer = 0;
    const reload = async () => {
      const { autosave: saving, history: undo } = latest.current;
      if (saving.hasUnsaved) {
        toast("An assistant changed this project while you were editing. Your version was kept.");
        return;
      }
      try {
        const fresh = await reloadProject();
        const before = undo.book;
        const freshBook = workbookOf(fresh);
        saving.markSaved(freshBook);
        undo.replace(freshBook);
        latest.current.onReloaded(fresh);
        toast("Updated by assistant", {
          action: { label: "Undo", run: () => latest.current.history.replace(before) },
        });
      } catch (error) {
        toast(`Couldn't load the assistant's changes. ${errorText(error)}`, { isError: true });
      }
    };
    const unlisten = onLibraryChanged((id) => {
      if (id !== latest.current.projectId) return;
      window.clearTimeout(timer);
      timer = window.setTimeout(reload, RELOAD_DELAY_MS);
    });

    return () => {
      window.clearTimeout(timer);
      unlisten.then((stop) => stop());
    };
  }, [toast]);

  // Write the last change before the window closes.
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async () => {
      await latest.current.autosave.flush();
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);
}
