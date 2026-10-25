import { memo, useEffect, useRef } from "react";
import type { Diagram } from "../lib/types";
import { Segmented } from "../ui/Segmented";
import { chosenVariant, groupDiagrams } from "../lib/diagrams";

type Props = {
  diagrams: Diagram[];
  selectedId: string;
  variantId: string | null | undefined;
  thumbnails: Record<string, string>;
  onSelect: (diagramId: string) => void;
  onVariant: (variantId: string) => void;
};

/** The diagram picker on the left: every diagram of this site in the current palette, under its group.
 * Memoized: it holds every thumbnail, and most edits on the sheet leave all of its props alone. */
export const DiagramColumn = memo(function DiagramColumn({ diagrams, selectedId, variantId, thumbnails, onSelect, onVariant }: Props) {
  const list = useRef<HTMLDivElement>(null);
  const groups = groupDiagrams(diagrams);

  // Keep the selected entry in view when an arrow key steps to one further down.
  useEffect(() => {
    list.current?.querySelector(".diagram-entry.is-selected")?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [selectedId]);

  return (
    <nav className="diagram-column" aria-label="Diagrams">
      <header className="diagram-column-header">
        <span className="diagram-column-title">Diagrams</span>
        <span className="diagram-column-count tabular">{diagrams.length}</span>
      </header>
      <div className="diagram-list" ref={list}>
        {groups.map((group) => (
          <section key={group.name} className="diagram-group" aria-label={group.name}>
            <h3 className="diagram-group-name">{group.name}</h3>
            {group.diagrams.map((diagram) => (
              <DiagramEntry
                key={diagram.id}
                diagram={diagram}
                isSelected={diagram.id === selectedId}
                variantId={variantId}
                thumbnail={thumbnails[diagram.id]}
                onSelect={onSelect}
                onVariant={onVariant}
              />
            ))}
          </section>
        ))}
      </div>
    </nav>
  );
});

type EntryProps = {
  diagram: Diagram;
  isSelected: boolean;
  variantId: string | null | undefined;
  thumbnail: string | undefined;
  onSelect: (diagramId: string) => void;
  onVariant: (variantId: string) => void;
};

/** One diagram: its picture and name, and under the selected one the switch between its variants. */
function DiagramEntry({ diagram, isSelected, variantId, thumbnail, onSelect, onVariant }: EntryProps) {
  const hasVariants = diagram.variants.length > 0;
  const chosen = chosenVariant(diagram, isSelected ? variantId : null);

  return (
    <div className={isSelected ? "diagram-entry is-selected" : "diagram-entry"}>
      {/* A native title, because the list scrolls and would clip the app's own tooltips. */}
      <button type="button" className="diagram-pick" onClick={() => onSelect(diagram.id)} title={diagram.description} aria-current={isSelected}>
        <span className="diagram-thumb">
          {thumbnail ? <img src={thumbnail} alt="" draggable={false} /> : <span className="diagram-thumb-blank" />}
        </span>
        <span className="diagram-label">
          <span className="diagram-name">{diagram.name}</span>
          {hasVariants && !isSelected && <span className="diagram-variant-names">{diagram.variants.map((variant) => variant.name).join(" · ")}</span>}
        </span>
      </button>
      {isSelected && chosen && (
        <div className="diagram-variants">
          <Segmented
            label={`${diagram.name}, measure`}
            value={chosen.id}
            options={diagram.variants.map((variant) => ({ value: variant.id, label: variant.name }))}
            onChange={onVariant}
          />
        </div>
      )}
    </div>
  );
}
