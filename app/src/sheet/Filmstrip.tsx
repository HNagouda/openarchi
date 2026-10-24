import { useEffect, useRef } from "react";
import type { Diagram } from "../lib/types";

type Props = {
  diagrams: Diagram[];
  selectedId: string;
  thumbnails: Record<string, string>;
  onSelect: (id: string) => void;
};

/** Every diagram for this site in the current theme, so the whole series is in view. */
export function Filmstrip({ diagrams, selectedId, thumbnails, onSelect }: Props) {
  const strip = useRef<HTMLDivElement>(null);

  // Keep the selected frame in view when keys 1–9 jump to one off-screen.
  useEffect(() => {
    strip.current?.querySelector(".frame.is-selected")?.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  }, [selectedId]);

  return (
    <div className="filmstrip" ref={strip}>
      {diagrams.map((diagram, index) => (
        <button
          type="button"
          key={diagram.id}
          className={diagram.id === selectedId ? "frame is-selected" : "frame"}
          onClick={() => onSelect(diagram.id)}
          data-tip={diagram.description}
          data-tip-side="above"
        >
          <span className="frame-picture">
            {thumbnails[diagram.id] ? <img src={thumbnails[diagram.id]} alt="" draggable={false} /> : <span className="frame-blank" />}
          </span>
          <span className="frame-caption">
            {index < 9 && <span className="frame-key tabular">{index + 1}</span>}
            <span className="frame-name">{diagram.name}</span>
          </span>
        </button>
      ))}
    </div>
  );
}
