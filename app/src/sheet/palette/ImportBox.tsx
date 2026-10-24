// Paste what a palette generator exports, or drop its file here. The text
// goes to the core's parser as plain text; nothing in it is shown or run.

import { useState, type DragEvent } from "react";
import { errorText } from "../../lib/ipc";
import { importPalette, type PaletteSwatch } from "../../lib/palette";
import { Button } from "../../ui/Button";

// The kinds of file palette generators export.
const FILE_TYPES = ".svg,.json,.xml,.txt,.csv,.css";

type Props = { onImport: (swatches: PaletteSwatch[]) => void };

export function ImportBox({ onImport }: Props) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isOver, setOver] = useState(false);

  const read = async (content: string) => {
    try {
      const swatches = await importPalette(content);
      setError(null);
      setText("");
      onImport(swatches);
    } catch (reason) {
      setError(errorText(reason));
    }
  };

  const readFile = async (file: File | undefined) => {
    if (!file) return;
    read(await file.text());
  };

  const onDrop = (event: DragEvent) => {
    event.preventDefault();
    setOver(false);
    const dropped = event.dataTransfer.files[0];
    if (dropped) readFile(dropped);
    else read(event.dataTransfer.getData("text/plain"));
  };

  return (
    <div
      className={isOver ? "import-box is-over" : "import-box"}
      onDragOver={(event) => {
        event.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={onDrop}
    >
      <textarea
        className="text-input import-text"
        value={text}
        rows={3}
        spellCheck={false}
        placeholder="Paste hex codes or a palette generator's link, or drop its SVG, JSON or XML file here"
        aria-label="Palette to import"
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) read(text);
          // Undo in the field undoes typing, not the palette.
          event.stopPropagation();
        }}
      />
      <div className="import-actions">
        <label className="button button-ghost import-file">
          <input type="file" accept={FILE_TYPES} onChange={(event) => readFile(event.target.files?.[0])} />
          <span className="button-label">Choose a file…</span>
        </label>
        <Button variant="secondary" disabled={text.trim() === ""} onClick={() => read(text)}>
          Use these colours
        </Button>
      </div>
      {error && (
        <p className="import-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
