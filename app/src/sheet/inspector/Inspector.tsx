import type { Catalog, Look, ProjectView } from "../../lib/types";
import { DiagramSection } from "./DiagramSection";
import { LayersSection } from "./LayersSection";
import { SheetSection } from "./SheetSection";
import { ThemeSection } from "./ThemeSection";

type Props = {
  catalog: Catalog;
  project: ProjectView;
  look: Look;
  thumbnails: Record<string, string>;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

/** The right-hand panel. Every control edits the look, and every edit can be undone. */
export function Inspector({ catalog, project, look, thumbnails, onLook, onSeal }: Props) {
  const diagram = catalog.diagrams.find((entry) => entry.id === look.diagram_id);
  const theme = catalog.themes.find((entry) => entry.id === look.theme_id);

  return (
    <aside className="inspector">
      <DiagramSection
        diagrams={catalog.diagrams}
        selectedId={look.diagram_id}
        thumbnails={thumbnails}
        onSelect={(diagram_id) => onLook({ ...look, diagram_id })}
      />
      <ThemeSection themes={catalog.themes} selectedId={look.theme_id} onSelect={(theme_id) => onLook({ ...look, theme_id })} />
      {diagram && theme && (
        <LayersSection
          diagram={diagram}
          theme={theme}
          look={look}
          labels={catalog.layers}
          layersPresent={project.layers_present}
          onLook={onLook}
          onSeal={onSeal}
        />
      )}
      <SheetSection look={look} onLook={onLook} onSeal={onSeal} />
    </aside>
  );
}
