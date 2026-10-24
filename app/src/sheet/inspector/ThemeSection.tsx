import type { Theme } from "../../lib/types";
import { Keys } from "../../ui/Button";
import { Section } from "../../ui/Section";
import { paletteStrip } from "./lookEdits";

/** Theme cards with their palette as a strip of five colours. [ and ] step through them. */
export function ThemeSection({ themes, selectedId, onSelect }: { themes: Theme[]; selectedId: string; onSelect: (id: string) => void }) {
  return (
    <Section id="theme" title="Theme" aside={
        <span className="section-keys" data-tip="Previous and next theme" data-tip-side="left">
          <Keys shortcut="[" />
          <Keys shortcut="]" />
        </span>
      }>
      <div className="theme-list">
        {themes.map((theme) => (
          <button
            type="button"
            key={theme.id}
            className={theme.id === selectedId ? "theme-card is-selected" : "theme-card"}
            onClick={() => onSelect(theme.id)}
            data-tip={theme.description}
            data-tip-side="above"
          >
            <span className="theme-strip" aria-hidden="true">
              {paletteStrip(theme).map((colour, index) => (
                <span key={index} style={{ background: colour }} />
              ))}
            </span>
            <span className="theme-name">{theme.name}</span>
          </button>
        ))}
      </div>
    </Section>
  );
}
