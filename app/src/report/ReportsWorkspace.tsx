// The project's reports, over the sheet workspace: pick, add, rename or
// delete a report and edit it, with undo for all of it and saving as you go.
//
// This is the temporary way in, from the sheet's Reports button. The editor
// itself is `ReportEditor`, which only needs a report and somewhere to send
// changes, so it can move into a sheet tab without this frame.

import { useCallback, useRef, useState } from "react";
import { useCommandHandler } from "../lib/commands";
import { useHistory } from "../lib/history";
import { saveReports } from "../lib/ipc";
import type { Catalog, ProjectView, Report } from "../lib/types";
import { useAutosave } from "../sheet/useAutosave";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { Problem } from "../ui/Problem";
import { useToast } from "../ui/Toasts";
import { newReport } from "./edits";
import { ExportReportDialog } from "./ExportReportDialog";
import { ReportEditor, type ZoomControls } from "./ReportEditor";

type Props = {
  project: ProjectView;
  catalog: Catalog;
  onClose: () => void;
  /** The reports as last saved, so the sheet can hand them back when this opens again. */
  onSaved: (reports: Report[]) => void;
};

export function ReportsWorkspace({ project, catalog, onClose, onSaved }: Props) {
  const history = useHistory<Report[]>(project.reports);
  const reports = history.present;
  const [openId, setOpenId] = useState<string | null>(reports[0]?.id ?? null);
  const [isExportOpen, setExportOpen] = useState(false);
  const zoom = useRef<ZoomControls>(null);
  const toast = useToast();

  const write = useCallback(
    async (next: Report[]) => {
      await saveReports(next);
      onSaved(next);
    },
    [onSaved],
  );
  const reportError = useCallback((message: string) => toast(`Couldn't save the report. ${message}`, { isError: true }), [toast]);
  const saving = useAutosave(reports, write, reportError);
  const report = reports.find((entry) => entry.id === openId) ?? reports[0] ?? null;

  const changeReport = (next: Report, mergeKey?: string) => {
    history.change(reports.map((entry) => (entry.id === next.id ? next : entry)), mergeKey);
  };

  const add = () => {
    const created = newReport(reports, reports.length === 0 ? project.name : `${project.name} ${reports.length + 1}`);
    history.change([...reports, created]);
    setOpenId(created.id);
  };

  const remove = () => {
    if (!report) return;
    history.change(reports.filter((entry) => entry.id !== report.id));
    setOpenId(null);
    toast(`Deleted “${report.name}”`, { action: { label: "Undo", run: history.undo } });
  };

  const close = async () => {
    await saving.flush();
    onClose();
  };

  useCommandHandler((id) => {
    if (id === "undo") history.undo();
    if (id === "redo") history.redo();
    if (id === "export" && report) setExportOpen(true);
    if (id === "zoom-in") zoom.current?.zoomIn();
    if (id === "zoom-out") zoom.current?.zoomOut();
    if (id === "zoom-fit") zoom.current?.fit();
    if (id === "home") close();
    return ["undo", "redo", "export", "zoom-in", "zoom-out", "zoom-fit", "home"].includes(id);
  });

  const saveText = { saved: "Saved", pending: "Edited", saving: "Saving…", failed: "Not saved" }[saving.status];

  return (
    <div className="reports-workspace">
      <header className="topbar" data-tauri-drag-region>
        <Button icon="back" tip="Back to the sheet" onClick={close} />
        <div className="topbar-title" data-tauri-drag-region>
          <span className="project-name">{project.name}</span>
          {report && <EditableText className="topbar-crumb report-name" value={report.name} onCommit={(name) => changeReport({ ...report, name })} />}
        </div>
        {reports.length > 1 && (
          <select className="report-select report-switch" aria-label="Report" value={report?.id ?? ""} onChange={(event) => setOpenId(event.target.value)}>
            {reports.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.name}
              </option>
            ))}
          </select>
        )}
        <Button icon="plus" tip="New report" onClick={add} />
        {report && <Button icon="trash" tip="Delete this report" onClick={remove} />}
        <div className="topbar-spacer" data-tauri-drag-region />
        <span className={saving.status === "failed" ? "save-status is-failed" : "save-status"}>{saveText}</span>
        <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!history.canUndo} onClick={history.undo} />
        <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!history.canRedo} onClick={history.redo} />
        <span className="topbar-divider" />
        <Button icon="export" variant="secondary" tip="Export the report" shortcut="mod+e" disabled={!report || !project.has_scene} onClick={() => setExportOpen(true)}>
          Export
        </Button>
      </header>

      {report && project.has_scene ? (
        <ReportEditor key={report.id} project={project} catalog={catalog} report={report} onChange={changeReport} onSeal={history.seal} controls={zoom} />
      ) : (
        <div className="canvas-message reports-empty">
          {project.has_scene ? (
            <Problem title="No reports yet." detail="A report is pages you lay out yourself, from the ratios, the analysis, the sheets and your own text.">
              <Button variant="primary" icon="plus" onClick={add}>
                New report
              </Button>
            </Problem>
          ) : (
            <Problem title="This site hasn't been fetched yet." detail="Reports are drawn from the site's data, so fetch it first." />
          )}
        </div>
      )}

      {isExportOpen && report && <ExportReportDialog report={report} onClose={() => setExportOpen(false)} />}
    </div>
  );
}
