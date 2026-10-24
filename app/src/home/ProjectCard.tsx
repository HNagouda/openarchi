import { useEffect, useState } from "react";
import { formatModified, formatScale } from "../lib/format";
import { projectThumbnail } from "../lib/ipc";
import type { Catalog, Look, ProjectSummary } from "../lib/types";
import { EditableText } from "../ui/EditableText";
import { Icon } from "../ui/Icon";

// Cards are about 280 px wide; twice that keeps the picture sharp on Retina.
const THUMBNAIL_WIDTH_PX = 560;

type Props = {
  project: ProjectSummary;
  diagrams: Catalog["diagrams"];
  isSelected: boolean;
  isOpening: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
};

type Thumbnail = { state: "loading" } | { state: "ready"; url: string } | { state: "none" } | { state: "failed" };

export function ProjectCard({ project, diagrams, isSelected, isOpening, onSelect, onOpen, onRename, onDelete }: Props) {
  const [thumbnail, setThumbnail] = useState<Thumbnail>({ state: "loading" });

  useEffect(() => {
    if (!project.has_scene) {
      setThumbnail({ state: "none" });
      return;
    }
    let isCurrent = true;
    projectThumbnail(project.id, project.modified, THUMBNAIL_WIDTH_PX)
      .then((url) => {
        if (isCurrent) setThumbnail(url ? { state: "ready", url } : { state: "none" });
      })
      .catch(() => {
        if (isCurrent) setThumbnail({ state: "failed" });
      });
    return () => {
      isCurrent = false;
    };
  }, [project.id, project.modified, project.has_scene]);

  return (
    <div
      className={["card", isSelected ? "is-selected" : "", isOpening ? "is-opening" : ""].join(" ")}
      onMouseDown={(event) => {
        event.stopPropagation();
        onSelect();
        // Read the click count here rather than waiting for a dblclick event,
        // so selection shows on the first press without delay.
        if (event.detail === 2) onOpen();
      }}
    >
      <div className="card-picture">
        {thumbnail.state === "ready" && <img src={thumbnail.url} alt="" draggable={false} />}
        {thumbnail.state === "loading" && <div className="card-shimmer" />}
        {thumbnail.state === "none" && (
          <div className="card-placeholder">
            <Icon name="map" />
            <span>Site not fetched yet</span>
          </div>
        )}
        {thumbnail.state === "failed" && (
          <div className="card-placeholder">
            <span>No preview</span>
          </div>
        )}
        <button
          type="button"
          className="card-delete"
          data-tip="Move to Recently Deleted"
          data-tip-key="⌫"
          aria-label="Move to Recently Deleted"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={onDelete}
        >
          <Icon name="trash" size={14} />
        </button>
      </div>
      {/* A title block, as on the sheet itself: the name, then what was printed and when. */}
      <div className="card-meta">
        <EditableText className="card-name" value={project.name} onCommit={onRename} />
        <span className="card-specs">
          {project.look && <span>{printLine(project.look, diagrams)}</span>}
          <span className="card-date">{formatModified(project.modified)}</span>
        </span>
      </div>
    </div>
  );
}

/** "Building age · 1:2500 · A3". The diagram name is left out until the catalog has loaded. */
function printLine(look: Look, diagrams: Catalog["diagrams"]): string {
  const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
  const paper = look.sheet.paper === "square" ? "Square" : look.sheet.paper.toUpperCase();
  const parts = [diagram?.name, formatScale(look.sheet.scale), paper];
  return parts.filter(Boolean).join(" · ");
}
