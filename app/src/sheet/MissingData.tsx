// A note over the sheet when it asks for data the site was fetched without,
// so an old project's empty diagram says why instead of looking broken. The
// core writes the notes onto the SVG's paper; this only reads them back.

import { Button } from "../ui/Button";
import "./missingData.css";

const NOTES = /data-missing="([^"]*)"/;

/** "a\nb" from the paper's data-missing attribute -> ["a", "b"]. */
function notesOf(svg: string) {
  const raw = NOTES.exec(svg)?.[1] ?? "";
  const text = raw.replaceAll("&amp;", "&").replaceAll("&lt;", "<").replaceAll("&gt;", ">").replaceAll("&quot;", '"');
  return text.split("\n").filter(Boolean);
}

export function MissingData({ svg, onFetchAgain }: { svg: string; onFetchAgain: () => void }) {
  const notes = notesOf(svg);
  if (notes.length === 0) return null;

  return (
    <div className="missing-data" role="status">
      <div>
        {notes.map((note) => (
          <p key={note}>{note}</p>
        ))}
      </div>
      <Button variant="secondary" onClick={onFetchAgain}>
        Fetch again
      </Button>
    </div>
  );
}
