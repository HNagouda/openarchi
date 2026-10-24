import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import type { Look, Theme } from "../../lib/types";
import { Keys } from "../../ui/Button";
import { Icon } from "../../ui/Icon";
import { Section } from "../../ui/Section";
import { PaletteActions } from "../palette/PaletteActions";
import { paletteStrip } from "./lookEdits";

// Built-in themes carry this author; anything else came from the user's themes folder.
const BUILTIN_AUTHOR = "openarchi";

type Props = {
  themes: Theme[];
  selectedId: string;
  onSelect: (id: string) => void;
  /** For the palette editor's live preview of the sheet. */
  look: Look;
  onCatalogChanged: () => Promise<unknown> | void;
};

/** A palette picker that works like a font menu: the current palette closed, every palette open. [ and ] step through them. */
export function ThemeSection({ themes, selectedId, onSelect, look, onCatalogChanged }: Props) {
  const [isOpen, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const selected = themes.find((theme) => theme.id === selectedId);
  const builtIn = themes.filter((theme) => theme.author === BUILTIN_AUTHOR);
  const own = themes.filter((theme) => theme.author !== BUILTIN_AUTHOR);

  useEffect(() => {
    if (!isOpen) return;
    root.current?.querySelector<HTMLElement>(".theme-option.is-selected")?.focus();
    const closeOutside = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", closeOutside);
    return () => document.removeEventListener("mousedown", closeOutside);
  }, [isOpen]);

  const close = () => {
    setOpen(false);
    trigger.current?.focus();
  };

  const choose = (id: string) => {
    onSelect(id);
    close();
  };

  // Arrow keys walk the list the way a native menu does; Escape closes it without choosing.
  const onListKey = (event: KeyboardEvent) => {
    const options = [...(root.current?.querySelectorAll<HTMLElement>(".theme-option") ?? [])];
    const index = options.indexOf(document.activeElement as HTMLElement);
    if (event.key === "ArrowDown") options[Math.min(options.length - 1, index + 1)]?.focus();
    else if (event.key === "ArrowUp") options[Math.max(0, index - 1)]?.focus();
    else if (event.key === "Escape") close();
    else return;
    event.preventDefault();
    event.stopPropagation();
  };

  return (
    <Section
      id="theme"
      title="Palette"
      aside={
        <span className="section-keys" data-tip="Previous and next palette" data-tip-side="left">
          <Keys shortcut="[" />
          <Keys shortcut="]" />
        </span>
      }
    >
      <div className="theme-picker" ref={root}>
        <button
          type="button"
          ref={trigger}
          className={isOpen ? "theme-trigger is-open" : "theme-trigger"}
          aria-haspopup="listbox"
          aria-expanded={isOpen}
          onClick={() => setOpen(!isOpen)}
          onKeyDown={(event) => {
            if (event.key !== "ArrowDown") return;
            event.preventDefault();
            setOpen(true);
          }}
        >
          {selected && <Strip theme={selected} />}
          <span className="theme-trigger-name">{selected?.name ?? "Choose a palette"}</span>
          <Icon name="chevronUpDown" size={14} />
        </button>
        {selected && <p className="inspector-note">{selected.description}</p>}
        <PaletteActions
          theme={selected}
          isOwn={selected !== undefined && selected.author !== BUILTIN_AUTHOR}
          look={look}
          onSelect={onSelect}
          onCatalogChanged={onCatalogChanged}
        />

        {isOpen && (
          <div className="theme-menu" role="listbox" aria-label="Palettes" onKeyDown={onListKey}>
            {builtIn.map((theme) => (
              <ThemeOption key={theme.id} theme={theme} isSelected={theme.id === selectedId} onChoose={choose} />
            ))}
            {own.length > 0 && <div className="theme-menu-heading">Your palettes</div>}
            {own.map((theme) => (
              <ThemeOption key={theme.id} theme={theme} isSelected={theme.id === selectedId} onChoose={choose} />
            ))}
          </div>
        )}
      </div>
    </Section>
  );
}

function ThemeOption({ theme, isSelected, onChoose }: { theme: Theme; isSelected: boolean; onChoose: (id: string) => void }) {
  return (
    <button
      type="button"
      role="option"
      aria-selected={isSelected}
      className={isSelected ? "theme-option is-selected" : "theme-option"}
      onClick={() => onChoose(theme.id)}
    >
      <Strip theme={theme} />
      <span className="theme-option-name">{theme.name}</span>
      <span className="theme-option-check">{isSelected && <Icon name="check" size={14} />}</span>
    </button>
  );
}

function Strip({ theme }: { theme: Theme }) {
  return (
    <span className="theme-strip" aria-hidden="true">
      {paletteStrip(theme).map((colour, index) => (
        <span key={index} style={{ background: colour }} />
      ))}
    </span>
  );
}
