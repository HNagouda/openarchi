// Hands the app the model in effect and the site edits after each change, in
// that order, and draws the sheet again once it holds both. They live in the
// sheet's undo history, so an undo comes through here like any other change.

import { useEffect, useRef } from "react";
import { errorText, setModel, setSiteEdits } from "../../lib/ipc";
import type { ModelChoice, SiteEdits } from "../../lib/types";
import type { useToast } from "../../ui/Toasts";

type Options = {
  model: ModelChoice | null;
  edits: SiteEdits;
  /** The app holds the new model and edits: draw the sheet again. */
  onApplied: () => void;
  toast: ReturnType<typeof useToast>;
};

export function useAppliedSite({ model, edits, onApplied, toast }: Options) {
  const appliedModel = useRef(JSON.stringify(model));
  const appliedEdits = useRef(JSON.stringify(edits));

  useEffect(() => {
    const wantedModel = JSON.stringify(model);
    const wantedEdits = JSON.stringify(edits);
    const isModelNew = wantedModel !== appliedModel.current;
    const isEditsNew = wantedEdits !== appliedEdits.current;
    if (!isModelNew && !isEditsNew) return;
    appliedModel.current = wantedModel;
    appliedEdits.current = wantedEdits;

    const apply = async () => {
      if (isModelNew) await setModel(model);
      if (isEditsNew) await setSiteEdits(edits);
    };
    apply()
      .then(onApplied)
      .catch((error) => toast(`Couldn't change the site. ${errorText(error)}`, { isError: true }));
  }, [model, edits, onApplied, toast]);
}
