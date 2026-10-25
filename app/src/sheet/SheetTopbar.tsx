// The sheet workspace's top bar: back to the projects, the project's name
// and the diagram shown, the save state, undo, and the ways out to text,
// reports, the site and export.

import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import type { useAutosave } from "./useAutosave";

// What the save state reads as, for each autosave status.
const SAVE_TEXT = { saved: "Saved", pending: "Edited", saving: "Saving…", failed: "Not saved" };

type Props = {
  projectName: string;
  diagramName: string | null;
  saveStatus: ReturnType<typeof useAutosave>["status"];
  canUndo: boolean;
  canRedo: boolean;
  isReady: boolean;
  hasCatalog: boolean;
  isDiagramsOpen: boolean;
  isInspectorOpen: boolean;
  /** Under the report editor the sheet is out of reach of Tab and the screen reader. */
  isInert: boolean;
  onRename: (name: string) => void;
  onLeave: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onAddText: () => void;
  onReports: () => void;
  onEditSite: () => void;
  onExport: () => void;
  onToggleDiagrams: () => void;
  onToggleInspector: () => void;
};

export function SheetTopbar(props: Props) {
  return (
    <header className="topbar" data-tauri-drag-region inert={props.isInert}>
      <Button icon="back" tip="All projects" shortcut="mod+shift+h" onClick={props.onLeave} />
      <Button icon="sidebarLeft" tip="Diagrams" shortcut="d" isActive={props.isDiagramsOpen} onClick={props.onToggleDiagrams} />
      <div className="topbar-title" data-tauri-drag-region>
        <EditableText className="project-name" value={props.projectName} onCommit={props.onRename} />
        {props.diagramName !== null && <span className="topbar-crumb">{props.diagramName}</span>}
      </div>
      <div className="topbar-spacer" data-tauri-drag-region />
      <span className={props.saveStatus === "failed" ? "save-status is-failed" : "save-status"}>{SAVE_TEXT[props.saveStatus]}</span>
      <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!props.canUndo} onClick={props.onUndo} />
      <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!props.canRedo} onClick={props.onRedo} />
      <span className="topbar-divider" />
      <Button icon="text" tip="Add text" shortcut="t" disabled={!props.isReady} onClick={props.onAddText} />
      <Button icon="report" tip="Lay out a report of this site" disabled={!props.hasCatalog} onClick={props.onReports}>
        Reports
      </Button>
      <Button icon="map" tip="Draw the site again" onClick={props.onEditSite}>
        Edit site
      </Button>
      <Button icon="export" variant="secondary" tip="Export" shortcut="mod+e" disabled={!props.isReady} onClick={props.onExport}>
        Export
      </Button>
      <Button icon="sidebar" tip="Inspector" shortcut="i" isActive={props.isInspectorOpen} onClick={props.onToggleInspector} />
    </header>
  );
}
