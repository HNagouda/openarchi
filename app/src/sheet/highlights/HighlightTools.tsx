// The highlight tool in the tab bar: a button per shape and an eye that hides
// the highlights on this sheet. A click arms the tool to draw that shape on
// the sheet; Return or Space places one of a house's size in the middle of
// the view instead, so the whole tool works from the keyboard.

import type { HighlightKind, Look } from "../../lib/types";
import { Button } from "../../ui/Button";
import { HIGHLIGHT_KINDS } from "./highlightEdits";
import { HIGHLIGHT_KEY } from "./useHighlightTool";
import "./highlights.css";

type Props = {
  look: Look;
  isReady: boolean;
  /** False on an explode or section view, where highlights are not edited. */
  isAvailable: boolean;
  drawingKind: HighlightKind | null;
  onDraw: (kind: HighlightKind) => void;
  onStop: () => void;
  onPlace: (kind: HighlightKind) => void;
  onLook: (next: Look) => void;
};

export function HighlightTools({ look, isReady, isAvailable, drawingKind, onDraw, onStop, onPlace, onLook }: Props) {
  const areHidden = look.hide_highlights ?? false;
  const isOff = !isReady || !isAvailable || areHidden;
  const offTip = !isReady ? "Highlights need a fetched site" : areHidden ? "Highlights are hidden on this sheet" : "Highlights are drawn on a plan or an axonometric view";

  return (
    <div className="highlight-tools" role="toolbar" aria-label="Highlights">
      {HIGHLIGHT_KINDS.map(({ kind, label, drawTip }) => (
        <Button
          key={kind}
          icon={kind}
          tip={isOff ? offTip : `Highlight a ${label.toLowerCase()}: ${drawTip}`}
          tipSide="above"
          shortcut={kind === "rectangle" ? HIGHLIGHT_KEY : undefined}
          isActive={drawingKind === kind}
          disabled={isOff}
          onClick={() => (drawingKind === kind ? onStop() : onDraw(kind))}
          onKeyDown={(event) => {
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            onPlace(kind);
          }}
        />
      ))}
      <Button
        icon={areHidden ? "eyeOff" : "eye"}
        tip={areHidden ? "Show highlights on this sheet" : "Hide highlights on this sheet"}
        tipSide="above"
        disabled={!isReady}
        onClick={() => onLook({ ...look, hide_highlights: !areHidden })}
      />
    </div>
  );
}
