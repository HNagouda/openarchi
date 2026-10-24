// The project gallery: every project as a card, plus Recently Deleted.

import { useCallback, useEffect, useState } from "react";
import type { Navigate } from "../App";
import { useCommandHandler } from "../lib/commands";
import {
  deleteProject,
  errorText,
  listProjects,
  listTrash,
  onLibraryChanged,
  openProject,
  renameProject,
  restoreProject,
} from "../lib/ipc";
import { usePlainKeys } from "../lib/keys";
import type { Catalog, ProjectSummary } from "../lib/types";
import { Button, Keys } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { Problem } from "../ui/Problem";
import { useToast } from "../ui/Toasts";
import { ProjectCard } from "./ProjectCard";
import { RecentlyDeleted } from "./RecentlyDeleted";

// Several file events arrive per save; one refresh after they settle is enough.
const REFRESH_DELAY_MS = 300;

type Props = {
  navigate: Navigate;
  catalog: Catalog | null;
  catalogError: string | null;
  onRetryCatalog: () => void;
};

export function Home({ navigate, catalog, catalogError, onRetryCatalog }: Props) {
  const [projects, setProjects] = useState<ProjectSummary[] | null>(null);
  const [trash, setTrash] = useState<ProjectSummary[]>([]);
  const [libraryError, setLibraryError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [openingId, setOpeningId] = useState<string | null>(null);
  const toast = useToast();

  const refresh = useCallback(() => {
    listProjects()
      .then((listed) => {
        setProjects(listed);
        setLibraryError(null);
      })
      .catch((error) => setLibraryError(errorText(error)));
    listTrash()
      .then(setTrash)
      .catch(() => setTrash([]));
  }, []);

  useEffect(refresh, [refresh]);

  useEffect(() => {
    let timer = 0;
    const unlisten = onLibraryChanged(() => {
      window.clearTimeout(timer);
      timer = window.setTimeout(refresh, REFRESH_DELAY_MS);
    });
    return () => {
      window.clearTimeout(timer);
      unlisten.then((stop) => stop());
    };
  }, [refresh]);

  const open = async (id: string) => {
    setOpeningId(id);
    try {
      const project = await openProject(id);
      if (project.has_scene) navigate.sheet(project);
      else navigate.editSite(project);
    } catch (error) {
      toast(`Couldn't open the project. ${errorText(error)}`, { isError: true });
    } finally {
      setOpeningId(null);
    }
  };

  const remove = async (project: ProjectSummary) => {
    try {
      await deleteProject(project.id);
      refresh();
      toast(`Moved “${project.name}” to Recently Deleted`, {
        action: { label: "Undo", run: () => restoreProject(project.id).then(refresh) },
      });
    } catch (error) {
      toast(`Couldn't delete the project. ${errorText(error)}`, { isError: true });
    }
  };

  const restore = async (project: ProjectSummary) => {
    try {
      await restoreProject(project.id);
      refresh();
      setSelectedId(project.id);
    } catch (error) {
      toast(`Couldn't restore the project. ${errorText(error)}`, { isError: true });
    }
  };

  const rename = async (id: string, name: string) => {
    setProjects((current) => current?.map((project) => (project.id === id ? { ...project, name } : project)) ?? null);
    try {
      await renameProject(id, name);
    } catch (error) {
      toast(`Couldn't rename the project. ${errorText(error)}`, { isError: true });
    }
    refresh();
  };

  const list = projects ?? [];
  const selectedIndex = list.findIndex((project) => project.id === selectedId);

  useCommandHandler((id) => {
    if (id === "open" && selectedId) {
      open(selectedId);
      return true;
    }
    return false;
  });

  usePlainKeys((key, event) => {
    if (key === "Enter" && selectedId) open(selectedId);
    if ((key === "Backspace" || key === "Delete") && selectedIndex >= 0 && !event.repeat) {
      remove(list[selectedIndex]);
      setSelectedId(list[selectedIndex + 1]?.id ?? list[selectedIndex - 1]?.id ?? null);
    }
    if (key === "ArrowRight" || key === "ArrowDown") {
      setSelectedId(list[Math.min(list.length - 1, selectedIndex + 1)]?.id ?? null);
      event.preventDefault();
    }
    if (key === "ArrowLeft" || key === "ArrowUp") {
      setSelectedId(list[Math.max(0, selectedIndex - 1)]?.id ?? null);
      event.preventDefault();
    }
    if (key === "Escape") setSelectedId(null);
  });

  return (
    <div className="home">
      <header className="topbar" data-tauri-drag-region>
        <Wordmark />
        <div className="topbar-spacer" data-tauri-drag-region />
        <Button icon="plus" tip="New project" shortcut="mod+n" variant="primary" onClick={navigate.newSite}>
          New project
        </Button>
      </header>

      <main className="home-scroll" onMouseDown={() => setSelectedId(null)}>
        <div className="home-content">
          {libraryError && (
            <Problem title="Your library couldn't be opened." detail={libraryError}>
              <Button variant="secondary" onClick={refresh}>
                Try again
              </Button>
            </Problem>
          )}
          {catalogError && !libraryError && (
            <div className="home-notice">
              <Problem title="Diagrams and palettes aren't available yet." detail={catalogError}>
                <Button variant="secondary" onClick={onRetryCatalog}>
                  Try again
                </Button>
              </Problem>
            </div>
          )}

          {projects !== null && (
            <>
              <div className="home-heading">
                <h1>Projects</h1>
                {list.length > 0 && (
                  <span className="home-count">
                    {list.length} {list.length === 1 ? "site" : "sites"}
                  </span>
                )}
              </div>
              {list.length === 0 ? (
                <EmptyGallery onNew={navigate.newSite} />
              ) : (
                <div className="gallery">
                  <NewSiteCard onNew={navigate.newSite} />
                  {list.map((project) => (
                    <ProjectCard
                      key={project.id}
                      project={project}
                      diagrams={catalog?.diagrams ?? []}
                      isSelected={project.id === selectedId}
                      isOpening={project.id === openingId}
                      onSelect={() => setSelectedId(project.id)}
                      onOpen={() => open(project.id)}
                      onRename={(name) => rename(project.id, name)}
                      onDelete={() => remove(project)}
                    />
                  ))}
                </div>
              )}
            </>
          )}

          {trash.length > 0 && <RecentlyDeleted projects={trash} onRestore={restore} />}
        </div>
      </main>
    </div>
  );
}

function Wordmark() {
  return (
    <div className="wordmark" data-tauri-drag-region>
      <span className="wordmark-mark" aria-hidden="true" />
      openarchi
    </div>
  );
}

/** The first tile: a blank sheet waiting for a site. */
function NewSiteCard({ onNew }: { onNew: () => void }) {
  return (
    <button type="button" className="card card-new" onClick={onNew} onMouseDown={(event) => event.stopPropagation()}>
      <span className="card-picture card-blank">
        <Icon name="plus" />
        <span>New site</span>
        <Keys shortcut="mod+n" />
      </span>
      <span className="card-meta">
        <span className="card-name">Blank sheet</span>
        <span className="card-specs">Search a place and draw the area</span>
      </span>
    </button>
  );
}

function EmptyGallery({ onNew }: { onNew: () => void }) {
  return (
    <div className="empty-gallery">
      <div className="empty-art" aria-hidden="true">
        <svg viewBox="0 0 120 120" width="120" height="120">
          <circle cx="60" cy="60" r="44" className="empty-ring" />
          <path className="empty-block" d="M28 44 52 40 54 54 30 58Z M58 32 74 30 78 56 62 58Z M36 66 56 62 60 84 40 88Z M66 66 92 62 94 76 68 80Z" />
          <path className="empty-site" d="M54 58 68 56 70 70 56 72Z" />
        </svg>
      </div>
      <h2>Draw your first site</h2>
      <p>Pick a place in the Netherlands, draw the area, and openarchi turns it into a series of site-analysis diagrams.</p>
      <Button variant="primary" onClick={onNew} tip="New project" shortcut="mod+n">
        New project
      </Button>
    </div>
  );
}
