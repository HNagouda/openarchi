// The window: which screen shows, the dialogs over it, and where commands go.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CommandContext, commandForKey, type CommandHandler, type CommandId } from "./lib/commands";
import { getCatalog, onMenu, errorText } from "./lib/ipc";
import { hasModifier, useKeyDown } from "./lib/keys";
import { isMac } from "./lib/platform";
import { readPreference } from "./lib/storage";
import { useAppearance } from "./lib/appearance";
import type { Catalog, ProjectView } from "./lib/types";
import { ToastProvider } from "./ui/Toasts";
import { Home } from "./home/Home";
import { SiteStep } from "./site/SiteStep";
import { SheetStep } from "./sheet/SheetStep";
import { Tour, TOUR_EDITION } from "./tour/Tour";
import { Settings, type SettingsTab } from "./settings/Settings";

type Screen =
  | { name: "home" }
  | { name: "site"; project: ProjectView | null }
  | { name: "sheet"; project: ProjectView };

export type Navigate = {
  home: () => void;
  newSite: () => void;
  editSite: (project: ProjectView) => void;
  sheet: (project: ProjectView) => void;
};

export function App() {
  const [screen, setScreen] = useState<Screen>({ name: "home" });
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [isTourOpen, setTourOpen] = useState(() => readPreference("tourEdition", 0) < TOUR_EDITION);
  const [settingsTab, setSettingsTab] = useState<SettingsTab | null>(null);
  const appearance = useAppearance();
  const screenHandler = useRef<CommandHandler | null>(null);

  const loadCatalog = useCallback(() => {
    getCatalog()
      .then((loaded) => {
        setCatalog(loaded);
        setCatalogError(null);
      })
      .catch((error) => setCatalogError(errorText(error)));
  }, []);

  useEffect(loadCatalog, [loadCatalog]);

  const navigate = useMemo<Navigate>(
    () => ({
      home: () => setScreen({ name: "home" }),
      newSite: () => setScreen({ name: "site", project: null }),
      editSite: (project) => setScreen({ name: "site", project }),
      sheet: (project) => setScreen({ name: "sheet", project }),
    }),
    [],
  );

  const runCommand = (id: CommandId) => {
    const isDialogOpen = isTourOpen || settingsTab !== null;
    if (!isDialogOpen && screenHandler.current?.(id)) return;
    if (id === "undo" || id === "redo") {
      // Nothing to undo on this screen; let a text field undo its typing.
      document.execCommand(id);
      return;
    }
    if (id === "settings") setSettingsTab("general");
    if (id === "assistant") setSettingsTab("assistant");
    if (id === "shortcuts") setSettingsTab("shortcuts");
    if (id === "tour") {
      setSettingsTab(null);
      setTourOpen(true);
    }
    if (isDialogOpen) return;
    if (id === "new") navigate.newSite();
    if (id === "home") navigate.home();
  };

  const latestRun = useRef(runCommand);
  latestRun.current = runCommand;

  useEffect(() => {
    const unlisten = onMenu((id) => latestRun.current(id as CommandId));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  // macOS sends these through the menu bar; elsewhere the window catches Ctrl.
  useKeyDown((event) => {
    if (isMac || !hasModifier(event)) return;
    const id = commandForKey(event.key, event.shiftKey);
    if (!id) return;
    event.preventDefault();
    runCommand(id);
  });

  return (
    <CommandContext.Provider value={screenHandler}>
      <ToastProvider>
        <div className="app">
          {screen.name === "home" && (
            <Home navigate={navigate} catalogError={catalogError} onRetryCatalog={loadCatalog} />
          )}
          {screen.name === "site" && <SiteStep project={screen.project} navigate={navigate} />}
          {screen.name === "sheet" && (
            <SheetStep
              key={screen.project.id}
              project={screen.project}
              catalog={catalog}
              catalogError={catalogError}
              onRetryCatalog={loadCatalog}
              navigate={navigate}
            />
          )}
        </div>
        {isTourOpen && <Tour onClose={() => setTourOpen(false)} />}
        {settingsTab !== null && (
          <Settings
            tab={settingsTab}
            onTab={setSettingsTab}
            onClose={() => setSettingsTab(null)}
            appearance={appearance.appearance}
            onAppearance={appearance.choose}
            onTour={() => {
              setSettingsTab(null);
              setTourOpen(true);
            }}
          />
        )}
      </ToastProvider>
    </CommandContext.Provider>
  );
}
