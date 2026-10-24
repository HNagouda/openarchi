import { useEffect, useState } from "react";
import { formatModified } from "../lib/format";
import { projectThumbnail } from "../lib/ipc";
import type { ProjectSummary } from "../lib/types";
import { EditableText } from "../ui/EditableText";
import { Icon } from "../ui/Icon";

// Cards are about 280 px wide; twice that keeps the picture sharp on Retina.
const THUMBNAIL_WIDTH_PX = 560;

type Props = {
  project: ProjectSummary;
  isSelected: boolean;
  isOpening: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
};

type Thumbnail = { state: "loading" } | { state: "ready"; url: string } | { state: "none" } | { state: "failed" };

export function ProjectCard({ project, isSelected, isOpening, onSelect, onOpen, onRename, onDelete }: Props) {
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
      <div className="card-meta">
        <EditableText className="card-name" value={project.name} onCommit={onRename} />
        <span className="card-date tabular">{formatModified(project.modified)}</span>
      </div>
    </div>
  );
}
