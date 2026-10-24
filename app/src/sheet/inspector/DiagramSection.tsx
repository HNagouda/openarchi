import type { Diagram } from "../../lib/types";
import { Section } from "../../ui/Section";

type Props = {
  diagrams: Diagram[];
  selectedId: string;
  thumbnails: Record<string, string>;
  onSelect: (id: string) => void;
};

/** Diagrams as small pictures, so the choice is visual. Keys 1–9 pick them too. */
export function DiagramSection({ diagrams, selectedId, thumbnails, onSelect }: Props) {
  const selected = diagrams.find((diagram) => diagram.id === selectedId);
  return (
    <Section id="diagram" title="Diagram" aside={selected ? <span>{selected.name}</span> : undefined}>
      <div className="diagram-grid">
        {diagrams.map((diagram, index) => (
          <button
            type="button"
            key={diagram.id}
            className={diagram.id === selectedId ? "diagram-tile is-selected" : "diagram-tile"}
            onClick={() => onSelect(diagram.id)}
            data-tip={index < 9 ? diagram.name : undefined}
            data-tip-key={index < 9 ? String(index + 1) : undefined}
            data-tip-side="above"
          >
            <span className="diagram-picture">
              {thumbnails[diagram.id] ? <img src={thumbnails[diagram.id]} alt="" draggable={false} /> : <span className="diagram-blank" />}
            </span>
            <span className="diagram-name">{diagram.name}</span>
          </button>
        ))}
      </div>
      {selected && <p className="inspector-note">{selected.description}</p>}
    </Section>
  );
}
