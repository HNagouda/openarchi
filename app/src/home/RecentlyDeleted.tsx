import { useState } from "react";
import { formatModified } from "../lib/format";
import { readPreference, writePreference } from "../lib/storage";
import type { ProjectSummary } from "../lib/types";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";

/** Deleted projects wait here, folded away like a Finder folder, until restored. */
export function RecentlyDeleted({ projects, onRestore }: { projects: ProjectSummary[]; onRestore: (project: ProjectSummary) => void }) {
  const [isOpen, setOpen] = useState(() => readPreference("trashOpen", false));

  const toggle = () => {
    setOpen(!isOpen);
    writePreference("trashOpen", !isOpen);
  };

  return (
    <section className={isOpen ? "trash is-open" : "trash"} onMouseDown={(event) => event.stopPropagation()}>
      <button type="button" className="trash-toggle" onClick={toggle} aria-expanded={isOpen}>
        <span className="trash-chevron">
          <Icon name="chevronRight" size={12} />
        </span>
        Recently Deleted
        <span className="trash-count tabular">{projects.length}</span>
      </button>
      {isOpen && (
        <ul className="trash-list">
          {projects.map((project) => (
            <li key={project.id} className="trash-row">
              <span className="trash-name">{project.name}</span>
              <span className="trash-date tabular">{formatModified(project.modified)}</span>
              <Button icon="restore" onClick={() => onRestore(project)}>
                Restore
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
